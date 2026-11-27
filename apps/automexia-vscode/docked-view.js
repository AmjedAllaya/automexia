const { spawn } = require('node:child_process');
const { randomBytes } = require('node:crypto');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { validatedContext, validatedPresetCatalog, gitTag, knownGitRepositories } = require('./docked-context');
const { createNativeRuntime, terminalPlatform } = require('./native-runtime');

const viewId = 'automexia.dockedTerminal';
const maxSessions = 8;
const maxTextBytes = 4096;
const maxQueuedBytes = 64 * 1024;
const maxFrameBytes = 1024 * 1024;
const controlCharacter = /[\x00-\x1f\x7f-\x9f]/;
const color = /^#[0-9a-f]{6}$/i;
const panelCommands = new Map([
  ['browse-quick-actions', 'automexia.browseQuickActions'],
  ['browse-workspaces', 'automexia.browseWorkspaces'],
  ['open-desktop', 'automexia.openDesktop'],
]);

function within(value, min, max) {
  return Number.isSafeInteger(value) && value >= min && value <= max;
}

function inputForHost(message) {
  switch (message.type) {
    case 'text':
    case 'paste':
      if (typeof message.text === 'string' && message.text.length > 0
        && Buffer.byteLength(message.text) <= maxTextBytes && !controlCharacter.test(message.text)) {
        return { type: message.type, text: message.text };
      }
      break;
    case 'key':
      if (typeof message.key === 'string' && message.key.length > 0 && message.key.length <= 24
        && !controlCharacter.test(message.key)) {
        const { key, ctrl, alt, shift, meta } = message;
        if ([ctrl, alt, shift, meta].every(value => typeof value === 'boolean')) {
          return { type: 'key', key, ctrl, alt, shift, meta };
        }
      }
      break;
    case 'resize':
      if (within(message.cols, 2, 240) && within(message.rows, 1, 120)) {
        return { type: 'resize', cols: message.cols, rows: message.rows };
      }
      break;
    case 'scroll':
      if (within(message.lines, -120, 120)) {
        return { type: 'scroll', lines: message.lines };
      }
      break;
  }
  return undefined;
}

function validFrame(frame) {
  if (!frame || typeof frame !== 'object') {
    return false;
  }
  const { cols, rows, cursor } = frame;
  if (!within(cols, 2, 240) || !Array.isArray(rows) || !within(rows.length, 1, 120)
    || !cursor || !within(cursor.row, 0, rows.length - 1) || !within(cursor.col, 0, cols - 1)
    || typeof cursor.visible !== 'boolean') {
    return false;
  }
  return rows.every(row => {
    if (!Array.isArray(row) || row.length > cols) {
      return false;
    }
    let columns = 0;
    return row.every(run => {
      if (!Array.isArray(run) || run.length !== 5) {
        return false;
      }
      const [text, foreground, background, flags, width] = run;
      if (typeof text !== 'string' || controlCharacter.test(text) || !within(flags, 0, 7)
        || !within(width, 1, cols)
        || ![foreground, background].every(value => value === null || (typeof value === 'string' && color.test(value)))) {
        return false;
      }
      columns += width;
      return columns <= cols;
    });
  });
}

function shellExitMessage(status) {
  if (!within(status, -2147483648, 2147483647)) {
    return 'Shell exited. Exit status unavailable.';
  }
  const hexadecimal = status < 0 ? ` (0x${(status >>> 0).toString(16).toUpperCase()})` : '';
  return `Shell exited with status ${status}${hexadecimal}.`;
}

function shellArguments(vscode) {
  const platform = terminalPlatform(process.platform, vscode.env.remoteName);
  const settings = vscode.workspace.getConfiguration('terminal.integrated');
  const name = settings.get('defaultProfile.' + platform);
  const profiles = settings.get('profiles.' + platform) ?? {};
  const args = typeof name === 'string' && Object.hasOwn(profiles, name) ? profiles[name]?.args : undefined;
  if (args === undefined) {
    return [];
  }
  if (!Array.isArray(args) || args.length > 32 || !args.every(arg => typeof arg === 'string'
    && Buffer.byteLength(arg) <= 4096 && !arg.includes('\0') && !arg.includes('${'))) {
    return undefined;
  }
  return args;
}

class DockedView {
  constructor(vscode, view, runtime = createNativeRuntime(vscode)) {
    this.vscode = vscode;
    this.view = view;
    this.runtime = runtime;
    this.sessions = new Map();
    this.disposables = [
      view.webview.onDidReceiveMessage(message => this.receive(message)),
      view.onDidChangeVisibility(() => {
        this.updatePolling();
        if (view.visible) this.publishPresets();
      }),
      view.onDidDispose(() => this.dispose()),
    ];
    this.updatePolling();
  }

  async send(message) {
    if (this.disposed) return false;
    try {
      return await this.view.webview.postMessage(message);
    } catch {
      return false;
    }
  }

  report(session, status, message) {
    return this.send({ type: 'session', id: session.id, status, title: session.title, message });
  }

  sendFrame(session, { cols, rows, cursor }) {
    return this.send({ type: 'frame', id: session.id, cols, rows, cursor });
  }

  receive(message) {
    if (this.disposed || !this.vscode.workspace.isTrusted || !message || typeof message !== 'object') {
      return;
    }
    if (message.type === 'ready') {
      this.presetsDelivered = false;
      this.publishPresets();
      return;
    }
    const command = panelCommands.get(message.type);
    if (command) {
      void this.runPanelCommand(command);
      return;
    }
    if (!within(message.id, 1, Number.MAX_SAFE_INTEGER)) {
      return;
    }
    const session = this.sessions.get(message.id);
    if (message.type === 'open') {
      if (!session) {
        void this.open(message.id);
      }
      return;
    }
    if (!session || session.closed) {
      return;
    }
    if (message.type === 'close') {
      this.close(session);
      return;
    }
    if (message.type === 'copy' && session.ready && typeof message.text === 'string'
      && Buffer.byteLength(message.text) <= 64 * 1024) {
      void Promise.resolve(this.vscode.env.clipboard.writeText(message.text)).catch(() => {
        void this.send({ type: 'status', message: 'Could not copy the selection.' });
      });
      return;
    }
    const input = inputForHost(message);
    if (input?.type === 'resize') {
      session.size = input;
    }
    if (input && session.ready) {
      this.write(session, input);
    }
  }

  async runPanelCommand(command) {
    try {
      await this.vscode.commands.executeCommand(command);
    } catch {
      await this.send({ type: 'status', message: 'The Automexia command could not be opened.' });
    }
  }

  async open(id) {
    if (!this.runtime.supported()) {
      void this.report({ id }, 'error', 'Open desktop VS Code locally or connect to a WSL distribution.');
      return;
    }
    if (this.sessions.size >= maxSessions) {
      void this.report({ id }, 'error', 'The session limit is reached. Close a session or wait for an existing one to finish closing.');
      return;
    }
    const session = { id, title: 'Shell', buffer: '', pending: false, ready: false, closed: false };
    this.sessions.set(id, session);
    void this.report(session, 'starting');
    session.startTimer = setTimeout(() => this.fail(session, 'The session host did not start in time.'), 15_000);
    try {
      const [host, shell, env] = await Promise.all([
        this.runtime.companion(), this.runtime.executable(this.vscode.env.shell), this.runtime.dockedEnvironment(),
      ]);
      if (session.closed || this.disposed) {
        return;
      }
      if (!host) {
        this.fail(session, 'Install the Automexia VSIX for this extension host, or set automexia.docked.hostExecutable to a native companion.');
        return;
      }
      const args = shellArguments(this.vscode);
      if (!shell || !args) {
        this.fail(session, 'Select a shell with an executable path and literal startup arguments.');
        return;
      }
      const folder = this.vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      const cwd = folder && path.isAbsolute(folder) ? folder : os.homedir();
      session.title = path.basename(shell);
      const child = spawn(host, ['--shell', shell, '--cwd', cwd, '--', ...args], {
        cwd,
        env,
        shell: false,
        windowsHide: true,
        stdio: ['pipe', 'pipe', 'ignore'],
      });
      session.child = child;
      child.stdin.on('error', () => this.fail(session, 'The session input closed unexpectedly.'));
      child.stdout.setEncoding('utf8');
      child.stdout.on('data', data => this.receiveHost(session, data));
      child.stdout.on('error', () => this.fail(session, 'The session output could not be read.'));
      child.on('error', () => this.fail(session, 'The session host could not start.'));
      child.on('close', () => {
        if (!session.closed) {
          void this.report(session, 'error', session.ready
            ? 'The session host stopped without reporting the shell exit status.'
            : 'The session host stopped unexpectedly.');
        }
        session.closed = true;
        clearTimeout(session.startTimer);
        clearTimeout(session.stopTimer);
        this.sessions.delete(id);
        this.updatePolling();
      });
    } catch {
      this.fail(session, 'The session host could not start.');
    } finally {
      if (session.closed && !session.child) {
        this.sessions.delete(id);
      }
    }
  }

  receiveHost(session, data) {
    if (session.closed) {
      return;
    }
    session.buffer += data;
    let boundary;
    while ((boundary = session.buffer.indexOf('\n')) !== -1 && !session.closed) {
      const line = session.buffer.slice(0, boundary);
      session.buffer = session.buffer.slice(boundary + 1);
      if (Buffer.byteLength(line) >= maxFrameBytes) {
        this.fail(session, 'The session host exceeded its output limit.');
        return;
      }
      try {
        const message = JSON.parse(line);
        if (message.type === 'ready' && message.version === 1 && !session.ready) {
          session.ready = true;
          session.supportsContext = message.context === true;
          clearTimeout(session.startTimer);
          void this.report(session, 'running');
          this.presets ??= validatedPresetCatalog(message.presets);
          this.publishPresets();
          if (!session.supportsContext) {
            void this.send({ type: 'status', message: 'Build the current Automexia session host to show information tags.' });
          }
          if (session.size) {
            this.write(session, session.size);
          }
          this.updatePolling();
        } else if (session.ready && session.pending && message.type === 'idle') {
          session.pending = false;
          this.publishContext(session, message.context);
        } else if (session.ready && session.pending && message.type === 'frame' && validFrame(message)) {
          session.pending = false;
          this.publishContext(session, message.context);
          session.delivering = true;
          void this.sendFrame(session, message).finally(() => { session.delivering = false; });
        } else if (message.type === 'exit') {
          if (validFrame(message.frame)) {
            void this.sendFrame(session, message.frame);
          }
          void this.report(session, 'exited', shellExitMessage(message.status));
          this.close(session);
        } else {
          this.fail(session, 'The session host returned an unsupported message.');
        }
      } catch {
        this.fail(session, 'The session host sent invalid output.');
      }
    }
    if (!session.closed && Buffer.byteLength(session.buffer) >= maxFrameBytes) {
      this.fail(session, 'The session host exceeded its output limit.');
    }
  }

  publishPresets() {
    if (!this.presets || this.presetsDelivered || this.presetsPending) return;
    this.presetsPending = true;
    void this.send({ type: 'informationBarPresets', presets: this.presets }).then(delivered => {
      this.presetsPending = false;
      this.presetsDelivered = delivered;
    });
  }

  publishContext(session, data) {
    if (!session.supportsContext || data === undefined) {
      return;
    }
    this.publishPresets();
    const context = validatedContext(data) ?? { cwd: null, cwdSource: 'launch', remote: false, tags: [] };
    const { cwd, cwdSource, tags } = context;
    const git = gitTag(context, knownGitRepositories(this.vscode));
    if (git) {
      tags.push(git);
    }
    const next = JSON.stringify({ cwd, cwdSource, tags });
    if (next !== session.context && !session.contextPending) {
      session.contextPending = true;
      void this.send({ type: 'context', id: session.id, cwd, cwdSource, tags }).then(delivered => {
        session.contextPending = false;
        if (delivered && !session.closed) session.context = next;
      });
    }
  }

  write(session, message) {
    if (session.closed) {
      return false;
    }
    const line = JSON.stringify(message) + '\n';
    if (session.child.stdin.writableLength + Buffer.byteLength(line) > maxQueuedBytes) {
      this.fail(session, 'The session input queue exceeded its limit.');
      return false;
    }
    session.child.stdin.write(line);
    return true;
  }

  updatePolling() {
    clearInterval(this.polling);
    if (!this.disposed && this.view.visible && [...this.sessions.values()].some(session => session.ready && !session.closed)) {
      this.polling = setInterval(() => {
        for (const session of this.sessions.values()) {
          if (!session.ready || session.closed) {
            continue;
          }
          if (session.pending && Date.now() - session.requestTime > 5_000) {
            this.fail(session, 'The session host stopped responding.');
          } else if (!session.pending && !session.delivering && this.write(session, { type: 'poll' })) {
            session.pending = true;
            session.requestTime = Date.now();
          }
        }
      }, 50);
    }
  }

  fail(session, message) {
    if (!session.closed) {
      void this.report(session, 'error', message);
      this.close(session);
    }
  }

  close(session) {
    if (session.closed) {
      return;
    }
    session.closed = true;
    clearTimeout(session.startTimer);
    this.updatePolling();
    if (session.child) {
      // EOF lets the native host shut down and reap its owned PTY.
      session.child.stdin.end();
      session.stopTimer = setTimeout(() => session.child.kill(), 12_000);
    }
  }

  dispose() {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    clearInterval(this.polling);
    for (const session of this.sessions.values()) {
      this.close(session);
    }
    for (const disposable of this.disposables) {
      disposable.dispose();
    }
  }
}

function registerDockedView(vscode, context, runtime) {
  let activeView;
  const provider = {
    async resolveWebviewView(view, _context, token) {
      if (token.isCancellationRequested || !vscode.workspace.isTrusted) {
        return;
      }
      const template = await fs.readFile(context.asAbsolutePath('media/panel.html'), 'utf8');
      if (token.isCancellationRequested || !vscode.workspace.isTrusted) {
        return;
      }
      const uri = vscode.Uri.joinPath(context.extensionUri, 'media');
      const replacements = {
        cspSource: view.webview.cspSource,
        nonce: randomBytes(24).toString('base64url'),
        styleUri: view.webview.asWebviewUri(vscode.Uri.joinPath(uri, 'panel.css')).toString(),
        scriptUri: view.webview.asWebviewUri(vscode.Uri.joinPath(uri, 'panel.js')).toString(),
      };
      activeView?.dispose();
      activeView = new DockedView(vscode, view, runtime);
      view.webview.options = { enableScripts: true, localResourceRoots: [uri] };
      view.webview.html = template.replace(/\{\{(cspSource|nonce|styleUri|scriptUri)\}\}/g, (_placeholder, name) => replacements[name]);
    },
  };
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(viewId, provider, {
      webviewOptions: { retainContextWhenHidden: true },
    }),
    vscode.commands.registerCommand('automexia.openDockedView', () => vscode.commands.executeCommand(viewId + '.focus')),
    { dispose: () => activeView?.dispose() },
  );
}

module.exports = { registerDockedView, DockedView, inputForHost, validFrame };
