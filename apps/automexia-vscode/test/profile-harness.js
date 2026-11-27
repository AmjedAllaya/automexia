const fs = require('node:fs/promises');
const { readFileSync } = require('node:fs');
const { createRequire } = require('node:module');
const os = require('node:os');
const path = require('node:path');
const { compileFunction } = require('node:vm');
const manifest = require('../package.json');

const extensionPath = path.resolve(__dirname, '..', manifest.main);
const extensionRequire = createRequire(extensionPath);
const loadExtension = compileFunction(readFileSync(extensionPath, 'utf8'), ['require', 'module', 'process'], {
  filename: extensionPath,
});

function activateExtension({
  home,
  binaryDirectory,
  configuredEnvironment,
  defaultProfile,
  profiles,
  platform = 'linux',
  arch = 'x64',
  extensionDirectory,
  expectsBundle = false,
  remoteName,
  trusted = true,
  filesystem = fs,
} = {}) {
  const errors = [];
  const registrations = new Map();
  const views = new Map();
  const commands = new Map();
  const configurationReads = [];
  const context = { subscriptions: [], extensionPath: extensionDirectory };
  const registration = { dispose() {} };
  const profilePlatform = { win32: 'windows', darwin: 'osx', linux: 'linux' }[platform];
  const settings = new Map([
    ['automexia.terminal.binaryDirectory', binaryDirectory],
    [`terminal.integrated.env.${profilePlatform}`, configuredEnvironment],
    [`terminal.integrated.defaultProfile.${profilePlatform}`, defaultProfile],
    [`terminal.integrated.profiles.${profilePlatform}`, profiles],
  ]);
  const vscode = {
    env: { remoteName },
    workspace: {
      isTrusted: trusted,
      getConfiguration(section) {
        return {
          get(key, fallback) {
            const name = `${section}.${key}`;
            configurationReads.push(name);
            const setting = settings.get(name);
            return setting === undefined ? fallback : setting;
          },
        };
      },
    },
    window: {
      registerWebviewViewProvider(id, provider, options) {
        views.set(id, { provider, options });
        return { dispose() {} };
      },
      registerTerminalProfileProvider(id, provider) {
        registrations.set(id, provider);
        return registration;
      },
      showErrorMessage(message) {
        errors.push(message);
        return Promise.resolve(undefined);
      },
    },
    commands: {
      registerCommand(id, command) {
        commands.set(id, command);
        return { dispose() {} };
      },
      executeCommand(id) {
        return Promise.resolve(id);
      },
    },
    TerminalLocation: { Panel: 1 },
    TerminalProfile: class {
      constructor(options) {
        this.options = options;
      }
    },
  };
  function mockRequire(name) {
    if (name === 'vscode') {
      return vscode;
    }
    if (name === 'node:os') {
      return { ...os, homedir: () => home };
    }
    if (name === 'node:fs/promises') {
      return filesystem;
    }
    if (name === './package.json' && expectsBundle) {
      return { ...manifest, files: [...manifest.files, 'runtime'] };
    }
    return extensionRequire(name);
  }
  const extension = { exports: {} };
  loadExtension(mockRequire, extension, { platform, arch });
  extension.exports.activate(context);

  return {
    errors,
    registrations,
    views,
    commands,
    configurationReads,
    context,
    registration,
    open(token = { isCancellationRequested: false }) {
      const provider = registrations.get(manifest.contributes.terminal.profiles[0].id);
      return provider.provideTerminalProfile(token);
    },
  };
}

async function temporaryDirectory() {
  const root = await fs.realpath(os.tmpdir());
  const directory = await fs.mkdtemp(path.join(root, 'automexia-profile-'));
  return {
    directory,
    cleanup: () => fs.rm(directory, { recursive: true, force: true }),
  };
}

async function createExecutables(directory) {
  await fs.mkdir(directory, { recursive: true });
  for (const name of ['automexia', 'amx']) {
    await fs.writeFile(path.join(directory, name), '', { mode: 0o700 });
  }
}

module.exports = { activateExtension, createExecutables, temporaryDirectory };
