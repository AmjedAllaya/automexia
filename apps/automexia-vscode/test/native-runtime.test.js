const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');
const { createNativeRuntime, runtimeTarget } = require('../native-runtime');

function fixture({ platform = 'linux', arch = 'x64', remoteName, runtimeSettings = {}, env = {}, expectsBundle = false } = {}) {
  const windows = platform === 'win32';
  const hostPath = windows ? path.win32 : path.posix;
  const extensionPath = windows ? 'C:\\example\\Installed Extension' : '/example/extension';
  const relative = [extensionPath, 'runtime', `${platform}-${arch}`];
  const root = hostPath.join(...relative);
  const bundleFolders = platform === 'darwin' ? ['Automexia Terminal.app', 'Contents', 'MacOS'] : [];
  const desktop = hostPath.join(root, 'desktop', ...bundleFolders);
  const suffix = windows ? '.exe' : '';
  const host = hostPath.join(root, 'companion', `automexia-vscode-host${suffix}`);
  const cli = hostPath.join(desktop, `amx${suffix}`);
  const app = hostPath.join(desktop, `automexia${suffix}`);
  const files = new Set([host, cli, app]);
  const inaccessible = new Set();
  const aliases = new Map();
  const directories = new Set([extensionPath, root]);
  const filesystem = {
    async realpath(value) {
      const target = aliases.get(value) ?? value;
      if (directories.has(target) || files.has(target)) return target;
      throw new Error('file missing');
    },
    async stat(value) { return { isFile: () => files.has(value) }; },
    async access(value) {
      if (windows) assert.fail('Windows does not use POSIX execute permissions');
      if (inaccessible.has(value)) throw new Error('file is not executable');
    },
  };
  const vscode = {
    env: { remoteName },
    workspace: { getConfiguration: section => ({ get: name => runtimeSettings[`${section}.${name}`] }) },
  };
  const runtime = createNativeRuntime(vscode, extensionPath, { platform, arch, filesystem, env, expectsBundle });
  return { runtime, aliases, files, inaccessible, directories, root, host, cli, app, desktop };
}

test('selects exactly the supported native platform and architecture, including remote WSL', () => {
  for (const platform of ['win32', 'darwin', 'linux']) {
    for (const arch of ['x64', 'arm64']) {
      assert.equal(runtimeTarget(platform, arch), `${platform}-${arch}`);
    }
  }
  assert.equal(runtimeTarget('linux', 'arm64', 'wsl'), 'linux-arm64');
  assert.equal(runtimeTarget('win32', 'x64', 'wsl'), undefined);
  assert.equal(runtimeTarget('linux', 'x64', 'ssh-remote'), undefined);
  assert.equal(runtimeTarget('linux', 'arm'), undefined);
});

test('discovers the complete package and preserves Windows PATH naming and prior entries', async () => {
  const env = { Path: 'C:\\example\\Path Bin', AUTOMEXIA_FIXTURE: 'unchanged' };
  const bundle = fixture({ platform: 'win32', env });
  assert.equal(await bundle.runtime.companion(), bundle.host);
  assert.equal(await bundle.runtime.desktop(), bundle.app);
  assert.deepEqual(await bundle.runtime.bundledDesktop(), { directory: bundle.desktop, automexia: bundle.app });
  assert.deepEqual(await bundle.runtime.dockedEnvironment(), { ...env, Path: `${bundle.desktop};${env.Path}` });
  assert.equal(env.Path, 'C:\\example\\Path Bin');
});

test('discovers the macOS app layout and injects only the packaged desktop directory', async () => {
  const bundle = fixture({ platform: 'darwin', arch: 'arm64', env: { PATH: '/example/bin' } });
  assert.equal(await bundle.runtime.companion(), bundle.host);
  assert.match(await bundle.runtime.desktop(), /desktop\/Automexia Terminal\.app\/Contents\/MacOS\/automexia$/);
  assert.deepEqual(await bundle.runtime.dockedEnvironment(), { PATH: `${bundle.desktop}:/example/bin` });
});

test('keeps developer overrides effective and uses PATH for a source install', async () => {
  const settings = {
    'automexia.docked.hostExecutable': '/example/alternate-host',
    'automexia.terminal.binaryDirectory': '/example/external',
  };
  const bundle = fixture({ runtimeSettings: settings, env: { PATH: 'relative:/example/path' } });
  bundle.files.add('/example/alternate-host');
  bundle.files.add('/example/external/automexia');
  bundle.files.add('/example/path/automexia-vscode-host');
  bundle.files.add('/example/path/automexia');
  assert.equal(await bundle.runtime.companion(), '/example/alternate-host');
  assert.equal(await bundle.runtime.desktop(), '/example/external/automexia');
  bundle.files.delete('/example/alternate-host');
  assert.equal(await bundle.runtime.companion(), undefined);
  delete settings['automexia.docked.hostExecutable'];
  delete settings['automexia.terminal.binaryDirectory'];
  bundle.directories.delete(bundle.root);
  assert.equal(await bundle.runtime.companion(), '/example/path/automexia-vscode-host');
  assert.equal(await bundle.runtime.desktop(), '/example/path/automexia');
  assert.equal(await bundle.runtime.dockedEnvironment(), undefined);
});

test('ignores incomplete, unexecutable, or escaping bundled runtimes', async () => {
  const bundle = fixture({ remoteName: 'wsl' });
  bundle.files.delete(bundle.cli);
  assert.equal(await bundle.runtime.desktop(), undefined);
  assert.equal(await bundle.runtime.dockedEnvironment(), undefined);
  bundle.inaccessible.add(bundle.host);
  assert.equal(await bundle.runtime.companion(), undefined);
  bundle.files.add('/example/extension/runtime/linux-arm64/host');
  bundle.aliases.set(bundle.host, '/example/extension/runtime/linux-arm64/host');
  assert.equal(await bundle.runtime.companion(), undefined);
  bundle.aliases.delete(bundle.host);
  bundle.inaccessible.clear();
  bundle.directories.add('/example/elsewhere');
  bundle.aliases.set(bundle.root, '/example/elsewhere');
  assert.equal(await bundle.runtime.companion(), undefined);
});

test('an incomplete platform package never silently launches an ambient executable', async () => {
  const settings = {};
  const bundle = fixture({ runtimeSettings: settings, env: { PATH: '/example/path' }, expectsBundle: true });
  bundle.directories.delete(bundle.root);
  bundle.files.add('/example/path/automexia-vscode-host');
  bundle.files.add('/example/path/automexia');
  assert.equal(await bundle.runtime.companion(), undefined);
  assert.equal(await bundle.runtime.desktop(), undefined);

  settings['automexia.terminal.binaryDirectory'] = '/example/path';
  assert.equal(await bundle.runtime.companion(), '/example/path/automexia-vscode-host');
  assert.equal(await bundle.runtime.desktop(), '/example/path/automexia');
});
