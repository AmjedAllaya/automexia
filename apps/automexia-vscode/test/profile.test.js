const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const test = require('node:test');
const manifest = require('../package.json');
const { activateExtension, createExecutables, temporaryDirectory } = require('./profile-harness');

async function createFixture(t) {
  const { directory, cleanup } = await temporaryDirectory();
  t.after(cleanup);
  return directory;
}

function mockWindowsDirectory(directory, resolved = directory) {
  const inspected = [];
  const filesystem = {
    async realpath(configured) {
      assert.equal(path.win32.normalize(configured), path.win32.normalize(directory));
      return resolved;
    },
    async stat(executable) {
      inspected.push(executable);
      return { isFile: () => true };
    },
    async access() {
      assert.fail('POSIX executable permission check was used on Windows');
    },
  };
  return { filesystem, inspected };
}

test('contributes the existing workspace profile ID with a machine-scoped binary path', () => {
  const profiles = manifest.contributes.terminal.profiles;
  assert.deepEqual(profiles.map(({ id, title }) => ({ id, title })), [{ id: 'automexia.wsl', title: 'Automexia CLI' }]);
  assert.equal(manifest.displayName, 'Automexia Terminal');
  assert.match(manifest.version, /^\d+\.\d+\.\d+$/);
  assert.ok(manifest.activationEvents.includes('onTerminalProfile:automexia.wsl'));
  assert.deepEqual(manifest.extensionKind, ['workspace']);
  assert.equal(manifest.capabilities.untrustedWorkspaces.supported, false);
  const setting = manifest.contributes.configuration.properties['automexia.terminal.binaryDirectory'];
  assert.equal(setting.scope, 'machine');
  assert.equal(setting.default, '');
});

test('delegates the default shell and environment on every supported host', async () => {
  const filesystemReads = [];
  const filesystem = new Proxy({}, {
    get(_target, name) {
      filesystemReads.push(name);
      assert.fail('Filesystem was accessed for an unconfigured profile');
    },
  });

  for (const host of [{ platform: 'win32' }, { platform: 'darwin' }, { platform: 'linux' }, { platform: 'linux', remoteName: 'wsl' }]) {
    for (const binaryDirectory of [undefined, '']) {
      const extension = activateExtension({
        ...host,
        binaryDirectory,
        filesystem,
        defaultProfile: 'Personal',
        profiles: { Personal: { path: 'personal-shell', args: ['-l'], env: { SAMPLE: 'personal' } } },
        configuredEnvironment: { PATH: '/custom/bin:${env:PATH}' },
      });
      assert.deepEqual([...extension.registrations.keys()], ['automexia.wsl']);
      assert.equal(extension.context.subscriptions[0], extension.registration);

      const profile = await extension.open();
      assert.deepEqual(profile.options, { name: 'Automexia CLI', location: 1 });
      assert.deepEqual(extension.configurationReads, ['automexia.terminal.binaryDirectory']);
      assert.deepEqual(extension.errors, []);
    }
  }
  assert.deepEqual(filesystemReads, []);
});

test('adds the packaged Linux CLI in WSL without selecting a shell', async () => {
  const extensionDirectory = '/extensions/automexia';
  const root = `${extensionDirectory}/runtime/linux-arm64`;
  const directory = `${root}/desktop`;
  const executables = [`${directory}/amx`, `${directory}/automexia`];
  const filesystem = {
    async realpath(value) {
      assert.ok([extensionDirectory, root, ...executables].includes(value));
      return value;
    },
    async stat(value) { return { isFile: () => executables.includes(value) }; },
    async access(value) { assert.ok(executables.includes(value)); },
  };
  const extension = activateExtension({ remoteName: 'wsl', arch: 'arm64', extensionDirectory, filesystem, expectsBundle: true });
  const profile = await extension.open();
  assert.deepEqual(profile.options, { name: 'Automexia CLI', location: 1, env: { PATH: `${directory}:\${env:PATH}` } });
  assert.deepEqual(extension.errors, []);
});

test('reports a missing packaged CLI instead of silently delegating to PATH', async () => {
  const extension = activateExtension({
    extensionDirectory: '/extensions/automexia', expectsBundle: true,
    filesystem: { async realpath() { throw new Error('package is incomplete'); } },
  });
  assert.equal(await extension.open(), undefined);
  assert.deepEqual(extension.errors, ['The bundled Automexia runtime is unavailable. Install the Automexia VSIX for this extension host.']);
});

test('adds a configured home-relative directory to a profile-local PATH', async (t) => {
  const home = await createFixture(t);
  const directory = path.join(home, 'CLI tools');
  await createExecutables(directory);
  const extension = activateExtension({
    home,
    binaryDirectory: '~/CLI tools',
    remoteName: 'wsl',
    configuredEnvironment: { PATH: '/custom/bin:${env:PATH}' },
  });

  const profile = await extension.open();
  assert.equal(profile.options.name, 'Automexia CLI');
  assert.equal(profile.options.location, 1);
  assert.equal(profile.options.shellPath, undefined);
  assert.equal(profile.options.shellArgs, undefined);
  assert.equal(profile.options.cwd, undefined);
  assert.deepEqual(profile.options.env, { PATH: `${directory}:/custom/bin:\${env:PATH}` });
  assert.deepEqual(extension.errors, []);
});

test('accepts an absolute directory with spaces and Unicode and retains inherited PATH', async (t) => {
  const home = await createFixture(t);
  const directory = path.join(home, 'CLI 工具');
  await createExecutables(directory);
  const extension = activateExtension({ binaryDirectory: directory });

  const profile = await extension.open();
  assert.equal(profile.options.env.PATH, `${directory}:\${env:PATH}`);
});

test('validates a symlink target while adding the PATH-safe alias', async (t) => {
  const directory = await createFixture(t);
  const target = path.join(directory, 'installation:bin');
  const alias = path.join(directory, 'tools');
  await createExecutables(target);
  await fs.symlink(target, alias, 'dir');
  const extension = activateExtension({ binaryDirectory: alias });

  const profile = await extension.open();
  assert.equal(profile.options.env.PATH, `${alias}:\${env:PATH}`);
  assert.deepEqual(extension.errors, []);
});

test('respects an explicitly cleared terminal PATH', async (t) => {
  const home = await createFixture(t);
  const directory = path.join(home, 'bin');
  await createExecutables(directory);
  const extension = activateExtension({ home, binaryDirectory: directory, configuredEnvironment: { PATH: null } });

  const profile = await extension.open();
  assert.equal(profile.options.env.PATH, directory);
});

test('prepends a CLI directory to the named macOS default profile environment', async (t) => {
  const directory = await createFixture(t);
  await createExecutables(directory);
  const extension = activateExtension({
    platform: 'darwin',
    binaryDirectory: directory,
    configuredEnvironment: { PATH: '/terminal/bin' },
    defaultProfile: 'Custom zsh',
    profiles: {
      'Custom zsh': { path: '/bin/zsh', args: ['-l'], env: { PATH: '/profile/bin', SAMPLE: 'profile' } },
    },
  });

  const profile = await extension.open();
  assert.deepEqual(profile.options, {
    name: 'Automexia CLI',
    location: 1,
    env: { PATH: `${directory}:/profile/bin`, SAMPLE: 'profile' },
  });
});

test('uses the CLI directory alone when the default profile clears PATH', async (t) => {
  const directory = await createFixture(t);
  await createExecutables(directory);
  const extension = activateExtension({
    binaryDirectory: directory,
    configuredEnvironment: { PATH: '/terminal/bin' },
    defaultProfile: 'Custom shell',
    profiles: { 'Custom shell': { path: '/bin/bash', env: { PATH: null, SAMPLE: 'profile' } } },
  });

  const profile = await extension.open();
  assert.deepEqual(profile.options.env, { PATH: directory, SAMPLE: 'profile' });
});

test('ignores malformed PATH and environment values while keeping valid profile entries', async (t) => {
  const directory = await createFixture(t);
  await createExecutables(directory);
  const extension = activateExtension({
    binaryDirectory: directory,
    configuredEnvironment: { PATH: ['/bad/bin'] },
    defaultProfile: 'Custom shell',
    profiles: {
      'Custom shell': {
        path: '/bin/bash',
        env: { PATH: 42, SAMPLE: 'profile', REMOVE: null, MALFORMED: true },
      },
    },
  });

  const profile = await extension.open();
  assert.deepEqual(profile.options.env, { PATH: `${directory}:\${env:PATH}`, SAMPLE: 'profile', REMOVE: null });
});

test('uses Windows .exe names and retains the case-insensitive terminal PATH', async () => {
  const directory = 'D:/CLI 工具';
  const resolved = 'D:\\CLI 工具';
  const { filesystem, inspected } = mockWindowsDirectory(directory, resolved);
  const extension = activateExtension({
    platform: 'win32',
    binaryDirectory: directory,
    configuredEnvironment: { Path: 'C:\\Terminal Bin' },
    filesystem,
  });

  const profile = await extension.open();
  assert.deepEqual(profile.options, {
    name: 'Automexia CLI',
    location: 1,
    env: { PATH: `${directory};C:\\Terminal Bin` },
  });
  assert.deepEqual(inspected.sort(), [`${resolved}\\amx.exe`, `${resolved}\\automexia.exe`]);
});

test('preserves an absolute UNC directory and a Windows default profile Path', async () => {
  const directory = '\\\\server\\share\\CLI tools';
  const { filesystem } = mockWindowsDirectory(directory);
  const extension = activateExtension({
    platform: 'win32',
    binaryDirectory: directory,
    configuredEnvironment: { PATH: 'C:\\Terminal Bin' },
    defaultProfile: 'Custom PowerShell',
    profiles: {
      'Custom PowerShell': {
        source: 'PowerShell',
        env: { PATH: 'C:\\Earlier Bin', Path: 'C:\\Profile Bin', SAMPLE: 'profile' },
      },
    },
    filesystem,
  });

  const profile = await extension.open();
  assert.deepEqual(profile.options.env, { PATH: `${directory};C:\\Profile Bin`, SAMPLE: 'profile' });
});

test('accepts both Windows home-relative forms', async () => {
  const home = 'C:\\Users\\sample';
  const directory = `${home}\\CLI tools`;
  const { filesystem } = mockWindowsDirectory(directory);
  for (const binaryDirectory of ['~/CLI tools', '~\\CLI tools']) {
    const extension = activateExtension({ platform: 'win32', home, binaryDirectory, filesystem });
    const profile = await extension.open();
    assert.equal(path.win32.normalize(profile.options.env.PATH.split(';')[0]), directory);
    assert.equal(profile.options.env.PATH.split(';')[1], '${env:PATH}');
  }
});

test('rejects drive-relative, current-drive-rooted, and PATH-ambiguous Windows paths', async () => {
  const inspected = [];
  const filesystem = {
    async realpath(directory) {
      inspected.push(directory);
      assert.fail('Invalid Windows directory reached the filesystem');
    },
  };
  for (const directory of ['C:relative', '\\current-drive', '/current-drive', 'C:\\left;right', 'C:\\${env:TEMP}']) {
    const extension = activateExtension({ platform: 'win32', binaryDirectory: directory, filesystem });
    assert.equal(await extension.open(), undefined);
    assert.equal(extension.errors.length, 1);
  }
  assert.deepEqual(inspected, []);
});

test('declines unsupported platforms and remote hosts other than Linux WSL', async () => {
  const ssh = activateExtension({ remoteName: 'ssh-remote' });
  const invalidWsl = activateExtension({ platform: 'win32', remoteName: 'wsl' });
  const unsupportedPlatform = activateExtension({ platform: 'freebsd' });

  for (const extension of [ssh, invalidWsl, unsupportedPlatform]) {
    assert.equal(await extension.open(), undefined);
    assert.equal(extension.errors.length, 1);
  }
});

test('declines an untrusted workspace', async () => {
  const extension = activateExtension({ trusted: false });
  assert.equal(await extension.open(), undefined);
});

test('returns silently when opening the profile is canceled', async () => {
  const extension = activateExtension();
  assert.equal(await extension.open({ isCancellationRequested: true }), undefined);
  assert.deepEqual(extension.errors, []);
});

test('discards the validated profile when cancellation arrives during file checks', async (t) => {
  const home = await createFixture(t);
  const binaryDirectory = path.join(home, 'bin');
  await createExecutables(binaryDirectory);
  const token = { isCancellationRequested: false };
  const extension = activateExtension({
    home,
    binaryDirectory,
    filesystem: {
      ...fs,
      async realpath(directory) {
        const resolved = await fs.realpath(directory);
        token.isCancellationRequested = true;
        return resolved;
      },
    },
  });

  assert.equal(await extension.open(token), undefined);
  assert.equal(token.isCancellationRequested, true);
  assert.deepEqual(extension.errors, []);
});

test('rejects relative directories and directories that cannot be included in PATH', async (t) => {
  const home = await createFixture(t);
  const ambiguousDirectory = path.join(home, 'invalid:path');
  await createExecutables(ambiguousDirectory);

  for (const binaryDirectory of ['relative/bin', ambiguousDirectory]) {
    const extension = activateExtension({ home, binaryDirectory });
    assert.equal(await extension.open(), undefined);
    assert.equal(extension.errors.length, 1);
  }
});

test('declines missing, non-executable, and non-regular CLI files', async (t) => {
  const home = await createFixture(t);
  const binaryDirectory = path.join(home, 'bin');
  await createExecutables(binaryDirectory);
  const amxPath = path.join(binaryDirectory, 'amx');

  await fs.chmod(amxPath, 0o600);
  const nonExecutable = activateExtension({ home, binaryDirectory });
  assert.equal(await nonExecutable.open(), undefined);
  assert.equal(nonExecutable.errors.length, 1);

  await fs.unlink(amxPath);
  const missing = activateExtension({ home, binaryDirectory });
  assert.equal(await missing.open(), undefined);
  assert.equal(missing.errors.length, 1);

  await fs.mkdir(amxPath);
  const nonRegular = activateExtension({ home, binaryDirectory });
  assert.equal(await nonRegular.open(), undefined);
  assert.equal(nonRegular.errors.length, 1);
});
