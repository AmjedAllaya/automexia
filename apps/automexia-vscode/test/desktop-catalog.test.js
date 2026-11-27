const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');
const { catalogItems, readCatalog, registerDesktopCatalog } = require('../desktop-catalog');

const action = {
  id: 'example.action', display_name: 'Check status', description: 'See saved details',
  scope: 'global-user', shells: ['powershell', 'bash'], risk: 'read-only', execution: 'insert', enabled: true,
};
const workspace = {
  id: 'example_workspace', display_name: 'Project work', environment: { risk: 'staging' },
  window_count: 1, connection_count: 2,
};
const actionResponse = row => ({ revision: 7, actions: [row] });
const workspaceResponse = row => ({ schema_version: 1, library_revision: 5, workspaces: [row], execution_enabled: false });

test('projects saved Quick Action metadata without forwarding action templates', () => {
  const payload = actionResponse({ ...action, display_name: '$(warning) Check status', template: { text: 'unused' } });
  assert.deepEqual(catalogItems('actions', payload), [{
    label: '$ (warning) Check status', description: 'example.action',
    detail: 'Enabled · Global user · Read only · Insert · PowerShell, Bash · See saved details',
  }]);
  assert.deepEqual(catalogItems('actions', { revision: 0, actions: [] }), []);
});

test('rejects hostile or malformed catalog fields before displaying them', () => {
  const rows = [
    { ...action, id: '--help' }, { ...action, description: 'unsafe\u202e text' },
    { ...action, display_name: 'a'.repeat(4097) }, { ...action, shells: [42] }, { ...action, enabled: 1 },
  ];
  for (const row of rows) {
    assert.throws(() => catalogItems('actions', actionResponse(row)));
  }
  assert.throws(() => catalogItems('actions', { revision: 1, actions: [action, action] }));
  assert.throws(() => catalogItems('actions', { revision: 1, actions: Array(1025).fill(action) }));
  assert.throws(() => catalogItems('actions', { revision: -1, actions: [] }));
});

test('projects workspace risk and counts without connection or credential fields', () => {
  const payload = workspaceResponse({
    ...workspace, environment: { ...workspace.environment, label: 'private' }, credential: 'unused',
  });
  assert.deepEqual(catalogItems('workspaces', payload), [{
    label: 'Project work', description: 'example_workspace', detail: 'Staging · 1 window · 2 connections',
  }]);
  assert.throws(() => catalogItems('workspaces', { ...payload, execution_enabled: true }));
  assert.throws(() => catalogItems('workspaces', { ...payload, schema_version: 2 }));
  assert.throws(() => catalogItems('workspaces', workspaceResponse({ ...workspace, window_count: 0 })));
});

test('queries the approved absolute CLI with exact arguments and resource limits', async () => {
  const executable = path.resolve('Automexia fixture', 'automexia');
  const { signal } = new AbortController();
  const calls = [];
  async function execute(file, argv, options) {
    calls.push({ file, argv, options });
    return { stdout: JSON.stringify(actionResponse(action)) };
  }
  const [item] = await readCatalog(executable, 'actions', signal, execute);
  assert.equal(item.label, action.display_name);
  assert.equal(calls.length, 1);
  const [{ file, argv, options }] = calls;
  assert.equal(file, executable);
  assert.deepEqual(argv, ['actions', 'list', '--json']);
  assert.equal(options.cwd, path.dirname(executable));
  assert.equal(options.shell, false);
  assert.equal(options.windowsHide, true);
  assert.equal(options.timeout, 5000);
  assert.equal(options.maxBuffer, 8 * 1024 * 1024);
  assert.equal(options.signal, signal);
  await assert.rejects(readCatalog(executable, '__proto__', signal, execute));
  await assert.rejects(readCatalog('automexia', 'actions', signal, execute));
  assert.equal(calls.length, 1);
});

test('gates catalog loading on trust, prevents overlap, and discards a cancelled lookup', async () => {
  const commands = new Map();
  const notifications = [];
  let cancelProgress;
  let finishLookup;
  let lookups = 0;
  const disposable = { dispose() {} };
  const subscriptions = [];
  const vscode = {
    ProgressLocation: { Notification: 15 }, workspace: { isTrusted: false },
    commands: { registerCommand(id, handler) { commands.set(id, handler); return disposable; } },
    window: {
      withProgress(_options, operation) {
        return operation({}, { onCancellationRequested(callback) { cancelProgress = callback; return disposable; } });
      },
      showQuickPick() { notifications.push('picker'); },
      showErrorMessage() { notifications.push('error'); },
      showInformationMessage() { notifications.push('info'); },
    },
  };
  registerDesktopCatalog(vscode, { subscriptions }, () => {
    lookups += 1;
    return new Promise(resolve => { finishLookup = resolve; });
  });
  await commands.get('automexia.browseQuickActions')();
  assert.equal(lookups, 0);

  vscode.workspace.isTrusted = true;
  const opening = commands.get('automexia.browseQuickActions')();
  await commands.get('automexia.browseWorkspaces')();
  assert.equal(lookups, 1);
  cancelProgress();
  finishLookup(undefined);
  await opening;
  assert.deepEqual(notifications, []);
});
