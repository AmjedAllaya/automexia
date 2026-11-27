const { spawn } = require('node:child_process');
const { constants } = require('node:fs');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const vscode = require('vscode');
const manifest = require('./package.json');
const { registerDockedView } = require('./docked-view');
const { registerDesktopCatalog } = require('./desktop-catalog');
const { canUseInPath, createNativeRuntime, terminalPlatform } = require('./native-runtime');

const { id, title } = manifest.contributes.terminal.profiles[0];
const binaryDirectorySetting = 'automexia.terminal.binaryDirectory';
const defaultBinaryDirectory = manifest.contributes.configuration.properties[binaryDirectorySetting].default;
const expectsBundle = manifest.files.includes('runtime');
async function findBinaryDirectory(configuredDirectory, hostPath) {
  const windows = hostPath === path.win32;
  const expandHome = configuredDirectory.startsWith('~/') || (windows && configuredDirectory.startsWith('~\\'));
  const directory = expandHome
    ? `${os.homedir()}${configuredDirectory.slice(1)}`
    : configuredDirectory;
  if (!canUseInPath(directory, hostPath)) {
    throw new Error('invalid binary directory');
  }

  const resolvedDirectory = await fs.realpath(directory);
  await Promise.all(['amx', 'automexia'].map(async (name) => {
    const executable = hostPath.join(resolvedDirectory, windows ? `${name}.exe` : name);
    if (!(await fs.stat(executable)).isFile()) {
      throw new Error('invalid CLI executable');
    }
    if (!windows) {
      await fs.access(executable, constants.X_OK);
    }
  }));

  // Keep the user's spelling so a link can point to a directory containing the PATH separator.
  return directory;
}

function terminalEnvironment(directory, terminalPlatform, hostPath) {
  const config = vscode.workspace.getConfiguration('terminal.integrated');
  const defaultProfile = config.get(`defaultProfile.${terminalPlatform}`);
  const profiles = config.get(`profiles.${terminalPlatform}`) ?? {};
  const profileEnvironment = typeof defaultProfile === 'string' && Object.hasOwn(profiles, defaultProfile)
    ? profiles[defaultProfile]?.env
    : undefined;
  const isPathKey = (key) => (hostPath === path.win32 ? key.toUpperCase() : key) === 'PATH';
  const environmentEntries = (env) => Object.entries(env ?? {}).filter(([, value]) => typeof value === 'string' || value === null);
  const profileEntries = environmentEntries(profileEnvironment);
  const platformEntries = environmentEntries(config.get(`env.${terminalPlatform}`));
  // VS Code applies Windows environment names case-insensitively, in entry order.
  const configuredPath = profileEntries.findLast(([key]) => isPathKey(key))
    ?? platformEntries.findLast(([key]) => isPathKey(key));
  const inheritedPath = configuredPath ? configuredPath[1] : '${env:PATH}';
  // Some VS Code versions only apply the default profile's env when launch env is absent.
  const env = Object.fromEntries(profileEntries.filter(([key]) => !isPathKey(key)));
  env.PATH = inheritedPath ? `${directory}${hostPath.delimiter}${inheritedPath}` : directory;
  return env;
}

async function openDesktop(findDesktopExecutable) {
  if (!vscode.workspace.isTrusted) return;
  const executable = await findDesktopExecutable();
  if (!vscode.workspace.isTrusted) return;
  if (!executable) {
    void vscode.window.showErrorMessage('Install the Automexia VSIX for this extension host or set automexia.terminal.binaryDirectory to the folder containing the Automexia desktop executable.');
    return;
  }

  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const cwd = folder && path.isAbsolute(folder) ? folder : os.homedir();
  const fail = () => void vscode.window.showErrorMessage('Could not start the Automexia desktop app.');
  try {
    const child = spawn(executable, ['--working-dir', cwd], { cwd, shell: false, detached: true, stdio: 'ignore' });
    child.once('error', fail);
    child.once('exit', code => { if (code !== null && code !== 0) fail(); });
    child.unref();
  } catch {
    fail();
  }
}

function activate(context) {
  const runtime = createNativeRuntime(vscode, context.extensionPath, {
    platform: process.platform, arch: process.arch, filesystem: fs, homedir: os.homedir, expectsBundle,
  });
  const provider = vscode.window.registerTerminalProfileProvider(id, {
    async provideTerminalProfile(token) {
      if (token.isCancellationRequested) {
        return undefined;
      }
      const platform = terminalPlatform(process.platform, vscode.env.remoteName);
      if (!platform) {
        void vscode.window.showErrorMessage('Open a local VS Code window on Windows, macOS, or Linux, or connect VS Code to WSL to use the Automexia terminal profile.');
        return undefined;
      }
      if (!vscode.workspace.isTrusted) {
        return undefined;
      }

      const configuredDirectory = vscode.workspace.getConfiguration('automexia.terminal')
        .get('binaryDirectory', defaultBinaryDirectory);
      const options = {
        name: title,
        location: vscode.TerminalLocation.Panel,
      };
      let binaryDirectory;
      const hostPath = process.platform === 'win32' ? path.win32 : path.posix;
      if (configuredDirectory === '') {
        binaryDirectory = (await runtime.bundledDesktop())?.directory;
        if (!binaryDirectory && expectsBundle) {
          if (!token.isCancellationRequested) {
            void vscode.window.showErrorMessage('The bundled Automexia runtime is unavailable. Install the Automexia VSIX for this extension host.');
          }
          return undefined;
        }
        if (binaryDirectory && !canUseInPath(binaryDirectory, hostPath)) {
          if (!token.isCancellationRequested) {
            void vscode.window.showErrorMessage('The installed Automexia directory cannot be added to the terminal PATH.');
          }
          return undefined;
        }
      } else {
        try {
          binaryDirectory = await findBinaryDirectory(configuredDirectory, hostPath);
        } catch {
          if (!token.isCancellationRequested) {
            void vscode.window.showErrorMessage('Set automexia.terminal.binaryDirectory in User Settings or WSL Remote Settings to the directory containing the amx and automexia executables, or clear it to use the bundled runtime or PATH.');
          }
          return undefined;
        }
      }
      if (token.isCancellationRequested) {
        return undefined;
      }

      if (binaryDirectory) options.env = terminalEnvironment(binaryDirectory, platform, hostPath);
      return new vscode.TerminalProfile(options);
    },
  });
  context.subscriptions.push(provider);
  registerDockedView(vscode, context, runtime);
  const findDesktopExecutable = () => runtime.desktop();
  registerDesktopCatalog(vscode, context, findDesktopExecutable);
  context.subscriptions.push(vscode.commands.registerCommand('automexia.openDesktop', () => openDesktop(findDesktopExecutable)));
}

module.exports = { activate };
