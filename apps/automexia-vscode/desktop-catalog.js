const { execFile } = require('node:child_process');
const path = require('node:path');
const { promisify } = require('node:util');

const runFile = promisify(execFile);
const maxOutputBytes = 8 * 1024 * 1024;
const maxTextBytes = 4096;
const queryTimeoutMs = 5000;
const unsafeText = /[\x00-\x1f\x7f-\x9f\u061c\u200e\u200f\u2028-\u202e\u2066-\u2069]/u;
const actionId = /^[a-z][a-z0-9.-]{2,63}$/;
const workspaceId = /^[a-z0-9][a-z0-9._-]{0,127}$/;
const actionScopes = new Map([
  ['session', 'Session'], ['capsule', 'Capsule'], ['trusted-workspace', 'Trusted workspace'],
  ['shell-user', 'Shell user'], ['global-user', 'Global user'], ['builtin-disabled', 'Built-in'],
]);
const actionRisks = new Map([
  ['read-only', 'Read only'], ['mutating', 'Mutating'], ['destructive', 'Destructive'], ['privileged', 'Privileged'],
]);
const actionModes = new Map([
  ['insert', 'Insert'], ['copy', 'Copy'], ['exact-launch', 'Exact launch'], ['run-workflow', 'Run workflow'],
]);
const shells = new Map([
  ['powershell', 'PowerShell'], ['bash', 'Bash'], ['zsh', 'Zsh'], ['fish', 'Fish'], ['cmd', 'Command Prompt'],
]);
const workspaceRisks = new Map([
  ['local', 'Local'], ['development', 'Development'], ['test', 'Test'], ['staging', 'Staging'], ['production', 'Production'],
]);
const missingExecutable = Symbol('missing executable');

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function text(value, allowEmpty = false) {
  if (typeof value !== 'string' || Buffer.byteLength(value) > maxTextBytes || unsafeText.test(value)
    || (!allowEmpty && !value.trim())) {
    throw new Error('Invalid catalog text');
  }
  // Quick Pick parses codicon syntax even in plain text fields.
  return value.replaceAll('$(', '$ (');
}

function nonnegativeInteger(value, max = Number.MAX_VALUE) {
  return Number.isInteger(value) && value >= 0 && value <= max;
}

function actionItem(action) {
  if (!isRecord(action) || typeof action.id !== 'string' || !actionId.test(action.id)
    || !actionScopes.has(action.scope) || !actionRisks.has(action.risk) || !actionModes.has(action.execution)
    || typeof action.enabled !== 'boolean' || !Array.isArray(action.shells) || action.shells.length > shells.size
    || !action.shells.every(shell => shells.has(shell))) {
    throw new Error('Invalid Quick Action summary');
  }
  const summary = [action.enabled ? 'Enabled' : 'Disabled', actionScopes.get(action.scope),
    actionRisks.get(action.risk), actionModes.get(action.execution), action.shells.map(shell => shells.get(shell)).join(', ')];
  const description = text(action.description, true);
  const metadata = summary.filter(Boolean).join(' · ');
  return { label: text(action.display_name), description: action.id, detail: description ? `${metadata} · ${description}` : metadata };
}

function workspaceItem(workspace) {
  if (!isRecord(workspace) || typeof workspace.id !== 'string' || !workspaceId.test(workspace.id)
    || !isRecord(workspace.environment) || !workspaceRisks.has(workspace.environment.risk)
    || !nonnegativeInteger(workspace.window_count, 16) || workspace.window_count === 0
    || !nonnegativeInteger(workspace.connection_count, 128)) {
    throw new Error('Invalid workspace summary');
  }
  const windows = `${workspace.window_count} ${workspace.window_count === 1 ? 'window' : 'windows'}`;
  const connections = `${workspace.connection_count} ${workspace.connection_count === 1 ? 'connection' : 'connections'}`;
  return {
    label: text(workspace.display_name), description: workspace.id,
    detail: `${workspaceRisks.get(workspace.environment.risk)} · ${windows} · ${connections}`,
  };
}

const catalogs = {
  actions: { command: 'automexia.browseQuickActions', title: 'Automexia Quick Actions', maxRows: 1024, item: actionItem },
  workspaces: { command: 'automexia.browseWorkspaces', title: 'Automexia Saved Workspaces', maxRows: 256, item: workspaceItem },
};

function catalogItems(subject, payload) {
  if (!Object.hasOwn(catalogs, subject) || !isRecord(payload)) {
    throw new Error('Invalid catalog response');
  }
  const { maxRows, item } = catalogs[subject];
  const rows = payload[subject];
  const validHeader = subject === 'actions' ? nonnegativeInteger(payload.revision)
    : payload.schema_version === 1 && nonnegativeInteger(payload.library_revision) && payload.execution_enabled === false;
  if (!validHeader || !Array.isArray(rows) || rows.length > maxRows) {
    throw new Error('Invalid catalog response');
  }
  const items = rows.map(item);
  if (new Set(items.map(entry => entry.description)).size !== items.length) {
    throw new Error('Duplicate catalog identifiers');
  }
  return items;
}

async function readCatalog(executable, subject, signal, execute = runFile) {
  if (!Object.hasOwn(catalogs, subject)) {
    throw new Error('Invalid catalog request');
  }
  if (typeof executable !== 'string' || !path.isAbsolute(executable)) {
    throw missingExecutable;
  }
  const { stdout } = await execute(executable, [subject, 'list', '--json'], {
    cwd: path.dirname(executable), encoding: 'utf8', windowsHide: true, shell: false,
    maxBuffer: maxOutputBytes, timeout: queryTimeoutMs, signal,
  });
  return catalogItems(subject, JSON.parse(stdout));
}

function registerDesktopCatalog(vscode, context, findDesktopExecutable) {
  let active;
  let disposed = false;
  context.subscriptions.push({ dispose() { disposed = true; active?.abort(); } });

  for (const [subject, { command, title }] of Object.entries(catalogs)) {
    context.subscriptions.push(vscode.commands.registerCommand(command, async () => {
      if (active || disposed || !vscode.workspace.isTrusted) {
        return;
      }
      const controller = new AbortController();
      active = controller;
      try {
        const items = await vscode.window.withProgress({
          location: vscode.ProgressLocation.Notification, title: `Loading ${title}`, cancellable: true,
        }, async (_progress, token) => {
          const subscription = token.onCancellationRequested(() => controller.abort());
          try {
            if (token.isCancellationRequested) {
              controller.abort();
              return undefined;
            }
            const executable = await findDesktopExecutable();
            if (controller.signal.aborted || !vscode.workspace.isTrusted) {
              return undefined;
            }
            return await readCatalog(executable, subject, controller.signal);
          } finally {
            subscription.dispose();
          }
        });
        if (!items || controller.signal.aborted || !vscode.workspace.isTrusted) {
          return;
        }
        if (items.length === 0) {
          void vscode.window.showInformationMessage(`${title} has no saved entries.`);
          return;
        }
        await vscode.window.showQuickPick(items, {
          title, placeHolder: 'Search saved entries', matchOnDescription: true, matchOnDetail: true,
        });
      } catch (error) {
        if (!controller.signal.aborted && vscode.workspace.isTrusted) {
          void vscode.window.showErrorMessage(error === missingExecutable
            ? 'Install the platform VSIX for this extension host, or set automexia.terminal.binaryDirectory to the folder containing the Automexia executable.'
            : `Could not load ${title}. Check that the configured Automexia executable supports the catalog command.`);
        }
      } finally {
        active = undefined;
      }
    }));
  }
}

module.exports = { registerDesktopCatalog, catalogItems, readCatalog };
