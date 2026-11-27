const assert = require('node:assert/strict');
const test = require('node:test');
const { DockedView, inputForHost, validFrame } = require('../docked-view');
const manifest = require('../package.json');
const { activateExtension } = require('./profile-harness');

test('registers the docked panel and open command without launching a shell', async () => {
  const extension = activateExtension();
  const view = manifest.contributes.views['automexia-panel'][0];
  assert.equal(view.type, 'webview');
  assert.equal(typeof extension.views.get(view.id).provider.resolveWebviewView, 'function');
  assert.equal(await extension.commands.get('automexia.openDockedView')(), view.id + '.focus');
  assert.equal(typeof extension.commands.get('automexia.browseQuickActions'), 'function');
  assert.equal(typeof extension.commands.get('automexia.browseWorkspaces'), 'function');
  assert.equal(typeof extension.commands.get('automexia.openDesktop'), 'function');
});

test('forwards only typed, bounded terminal input to the native host', () => {
  assert.deepEqual(inputForHost({ type: 'text', text: 'é', shell: '/bin/sh' }), { type: 'text', text: 'é' });
  for (const text of ['first\nsecond', '\x1b[31m', '\u0085', 'é'.repeat(2049)]) {
    assert.equal(inputForHost({ type: 'paste', text }), undefined);
  }
  assert.equal(inputForHost({ type: 'resize', cols: 241, rows: 24 }), undefined);
  assert.equal(inputForHost({ type: 'scroll', lines: 0.5 }), undefined);
  assert.deepEqual(inputForHost({ type: 'resize', cols: 80, rows: 24 }), { type: 'resize', cols: 80, rows: 24 });
  const key = { type: 'key', key: 'Enter', ctrl: false, alt: false, shift: false, meta: false };
  assert.deepEqual(inputForHost(key), key);
  assert.equal(inputForHost({ ...key, ctrl: 1 }), undefined);
});

test('checks styled output geometry and colors before publishing a frame', () => {
  const frame = {
    cols: 4,
    rows: [[['表', '#ff0000', null, 1, 2], ['a ', null, null, 0, 2]]],
    cursor: { row: 0, col: 2, visible: true },
  };
  assert.equal(validFrame(frame), true);
  assert.equal(validFrame({ ...frame, cols: 3 }), false);
  assert.equal(validFrame({ ...frame, cursor: { ...frame.cursor, col: 4 } }), false);
  assert.equal(validFrame({ ...frame, rows: [[['a', 'url(https://example.test)', null, 0, 1]]] }), false);
  assert.equal(validFrame({ ...frame, rows: [[['\n', null, null, 0, 1]]] }), false);
  assert.equal(validFrame(null), false);
});

function dockedView(postMessage, vscode = {}) {
  const disposable = { dispose() {} };
  return new DockedView(vscode, {
    visible: false,
    webview: {
      postMessage,
      onDidReceiveMessage() { return disposable; },
    },
    onDidChangeVisibility() { return disposable; },
    onDidDispose() { return disposable; },
  });
}

function receiveExit(message) {
  const posted = [];
  const view = dockedView(data => { posted.push(data); return Promise.resolve(true); });
  const session = { id: 7, title: 'PowerShell', ready: true, pending: false, buffer: '' };
  view.receiveHost(session, JSON.stringify({ type: 'exit', ...message }) + '\n');
  assert.equal(session.closed, true);
  return posted;
}

test('relays a companion preset catalog and accepts legacy handshakes', () => {
  const posted = [];
  const view = dockedView(data => { posted.push(data); return Promise.resolve(true); });
  const presets = [{
    id: 'compact-icon-mode', label: 'Compact icon mode', visual: 'flat',
    arrangement: 'compact', spacingPercent: 100,
    slots: [{ role: 'windows', lane: 'leading', iconOnly: true }],
  }];
  const start = (id, message) => view.receiveHost({ id, title: 'Shell', buffer: '' },
    JSON.stringify({ type: 'ready', version: 1, context: true, ...message }) + '\n');
  start(1, {});
  assert.deepEqual(posted.map(message => message.type), ['session']);
  start(2, { presets });
  assert.deepEqual(posted[2], { type: 'informationBarPresets', presets });
});

test('retries unchanged context after webview delivery fails', async () => {
  const posted = [];
  const view = dockedView(data => { posted.push(data); return Promise.resolve(posted.length > 1); });
  const session = { id: 7, supportsContext: true };
  const data = { cwd: '/example', cwdSource: 'launch', remote: false, tags: [] };
  view.publishContext(session, data);
  await new Promise(setImmediate);
  assert.equal(session.context, undefined);
  view.publishContext(session, data);
  await new Promise(setImmediate);
  assert.equal(posted.length, 2);
  assert.deepEqual(posted[0], posted[1]);
  view.publishContext(session, data);
  assert.equal(posted.length, 2);
});

test('shows the shell exit status and final output even without a pending poll', () => {
  const frame = {
    cols: 4,
    rows: [[['done', null, null, 0, 4]]],
    cursor: { row: 0, col: 0, visible: false },
  };
  assert.deepEqual(receiveExit({ status: -1073741819, frame }), [
    { type: 'frame', id: 7, ...frame },
    {
      type: 'session', id: 7, status: 'exited', title: 'PowerShell',
      message: 'Shell exited with status -1073741819 (0xC0000005).',
    },
  ]);
  assert.equal(receiveExit({ status: 0 })[0].message, 'Shell exited with status 0.');
});

test('still reports shell exit with unavailable status or malformed final output', () => {
  for (const status of [undefined, null, '7', 2147483648]) {
    const posted = receiveExit({ status });
    assert.equal(posted.length, 1);
    assert.equal(posted[0].message, 'Shell exited. Exit status unavailable.');
  }
  const posted = receiveExit({ status: 17, frame: { cols: 0 } });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].message, 'Shell exited with status 17.');
});
