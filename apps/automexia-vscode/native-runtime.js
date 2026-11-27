const { constants } = require('node:fs');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

const terminalPlatforms = new Map([['win32', 'windows'], ['darwin', 'osx'], ['linux', 'linux']]);
const packagedArchitectures = new Set(['x64', 'arm64']);
const controlCharacters = /[\x00-\x1f\x7f-\x9f]/;

function terminalPlatform(platform, remoteName) {
  if (remoteName && (remoteName !== 'wsl' || platform !== 'linux')) return undefined;
  return terminalPlatforms.get(platform);
}

function runtimeTarget(platform, arch, remoteName) {
  return terminalPlatform(platform, remoteName) && packagedArchitectures.has(arch) ? `${platform}-${arch}` : undefined;
}

function canUseInPath(directory, hostPath) {
  const fullyQualified = hostPath.isAbsolute(directory)
    && (hostPath !== path.win32 || (hostPath.parse(directory).root.length > 1 && !/^[\\/]{2}[?.][\\/]/.test(directory)));
  return fullyQualified && Buffer.byteLength(directory) < 4096 && !/[\x00-\x1f\x7f]/.test(directory)
    && !directory.includes(hostPath.delimiter) && !directory.includes('${');
}

function containsPath(hostPath, directory, candidate) {
  const relative = hostPath.relative(directory, candidate);
  return relative !== '' && relative !== '..' && !relative.startsWith(`..${hostPath.sep}`)
    && !hostPath.isAbsolute(relative);
}

function createNativeRuntime(vscode, extensionPath, {
  platform = process.platform, arch = process.arch, filesystem = fs, homedir = os.homedir,
  env = process.env, expectsBundle = false,
} = {}) {
  const windows = platform === 'win32';
  const hostPath = windows ? path.win32 : path.posix;
  const executableName = basename => (windows ? `${basename}.exe` : basename);

  function supported() {
    return Boolean(terminalPlatform(platform, vscode.env?.remoteName));
  }

  function expandHome(value) {
    if (typeof value !== 'string') return '';
    return value.startsWith('~/') || (windows && value.startsWith('~\\'))
      ? homedir() + value.slice(1) : value;
  }

  async function executable(candidate, within) {
    if (typeof candidate !== 'string' || !hostPath.isAbsolute(candidate) || controlCharacters.test(candidate)
      || (windows && hostPath.parse(candidate).root.length <= 1) || Buffer.byteLength(candidate) >= 4096) {
      return undefined;
    }
    try {
      const resolved = await filesystem.realpath(candidate);
      if ((within && !containsPath(hostPath, within, resolved)) || !(await filesystem.stat(resolved)).isFile()) {
        return undefined;
      }
      if (windows) return hostPath.extname(resolved).toLowerCase() === '.exe' ? resolved : undefined;
      await filesystem.access(resolved, constants.X_OK);
      return resolved;
    } catch {
      return undefined;
    }
  }

  async function bundledRoot() {
    const target = runtimeTarget(platform, arch, vscode.env?.remoteName);
    if (!target || typeof extensionPath !== 'string' || !hostPath.isAbsolute(extensionPath)) return undefined;
    try {
      const root = hostPath.join(await filesystem.realpath(extensionPath), 'runtime', target);
      const resolved = await filesystem.realpath(root);
      // An alias to another target, or out of the package, is never a bundle.
      if (hostPath.relative(root, resolved) === '') return resolved;
    } catch {
      // Developer checkouts and source-only packages may omit the bundle.
    }
    return undefined;
  }

  async function bundledDesktop() {
    const root = await bundledRoot();
    if (!root) return undefined;
    const folder = platform === 'darwin' ? ['Automexia Terminal.app', 'Contents', 'MacOS'] : [];
    const directory = hostPath.join(root, 'desktop', ...folder);
    const [automexia, amx] = await Promise.all([
      executable(hostPath.join(directory, executableName('automexia')), root),
      executable(hostPath.join(directory, executableName('amx')), root),
    ]);
    return automexia && amx ? { directory, automexia } : undefined;
  }

  function configuredDirectory() {
    return expandHome(vscode.workspace.getConfiguration('automexia.terminal').get('binaryDirectory'));
  }

  function environmentPathKeys() {
    // Node uses the first case-insensitive key in sorted order on Windows.
    return windows ? Object.keys(env).filter(key => key.toUpperCase() === 'PATH').sort() : ['PATH'];
  }

  async function findOnPath(name) {
    if (expectsBundle) return undefined;
    const [key = 'PATH'] = environmentPathKeys();
    for (const directory of (env[key] ?? '').split(hostPath.delimiter, 64)) {
      if (hostPath.isAbsolute(directory)) {
        const candidate = await executable(hostPath.join(directory, name));
        if (candidate) return candidate;
      }
    }
    return undefined;
  }

  async function companion() {
    if (!supported()) return undefined;
    const configuration = vscode.workspace.getConfiguration('automexia.docked').get('hostExecutable');
    if (configuration) return executable(expandHome(configuration));
    const name = executableName('automexia-vscode-host');
    const root = await bundledRoot();
    const bundled = root && await executable(hostPath.join(root, 'companion', name), root);
    if (bundled) return bundled;
    const directory = configuredDirectory();
    const configured = hostPath.isAbsolute(directory) && await executable(hostPath.join(directory, name));
    return configured || findOnPath(name);
  }

  async function desktop() {
    if (!supported()) return undefined;
    const name = executableName('automexia');
    const directory = configuredDirectory();
    const configured = hostPath.isAbsolute(directory) && await executable(hostPath.join(directory, name));
    if (configured) return configured;
    const bundled = await bundledDesktop();
    return bundled?.automexia || findOnPath(name);
  }

  async function dockedEnvironment() {
    const bundle = await bundledDesktop();
    if (!bundle || !canUseInPath(bundle.directory, hostPath)) return undefined;
    const environment = { ...env };
    const [key = 'PATH', ...duplicates] = environmentPathKeys();
    for (const duplicate of duplicates) delete environment[duplicate];
    environment[key] = [bundle.directory, env[key]].filter(Boolean).join(hostPath.delimiter);
    return environment;
  }

  return { supported, executable, companion, desktop, bundledDesktop, dockedEnvironment };
}

module.exports = { canUseInPath, createNativeRuntime, runtimeTarget, terminalPlatform };
