const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { access, mkdtemp, rmdir } = require('node:fs/promises');
const { tmpdir } = require('node:os');
const path = require('node:path');
const test = require('node:test');
const { setTimeout: delay } = require('node:timers/promises');

const maxMessageBytes = 1024 * 1024;
const pollInterval = 50;
const expectedCode = 37;
const marker = 'automexia-docked-probe';

async function* messages(stream) {
  let buffer = '';
  for await (const chunk of stream.setEncoding('utf8')) {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf('\n')) !== -1) {
      const line = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      assert.ok(Buffer.byteLength(line) < maxMessageBytes, 'oversized companion response');
      let value;
      try {
        value = JSON.parse(line);
      } catch {
        assert.fail('invalid companion response');
      }
      yield value;
    }
    assert.ok(Buffer.byteLength(buffer) < maxMessageBytes, 'oversized companion response');
  }
  assert.equal(buffer.length, 0, 'incomplete companion response');
}

function visibleText(frame) {
  return frame.rows.map(row => row.map(run => run[0]).join('')).join('\n');
}

async function probePowerShell(t, args) {
  assert.equal(process.platform, 'win32', 'run this probe in a disposable Windows guest');
  const host = process.env.AUTOMEXIA_VSCODE_TEST_HOST;
  assert.ok(path.isAbsolute(host ?? ''), 'set AUTOMEXIA_VSCODE_TEST_HOST to the built companion executable');
  const shell = path.join(process.env.SystemRoot ?? '', 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe');
  assert.ok(path.isAbsolute(shell), 'Windows PowerShell is required');
  await Promise.all([access(host), access(shell)]);

  const cwd = await mkdtemp(path.join(tmpdir(), 'automexia-docked-probe-'));
  const child = spawn(host, ['--shell', shell, '--cwd', cwd, '--', ...args], {
    cwd, shell: false, windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'], signal: t.signal,
  });
  const closed = new Promise(resolve => child.once('close', resolve));
  child.on('error', () => {});
  child.stdin.on('error', () => {});
  t.after(async () => {
    child.stdin.end();
    child.kill();
    const code = await closed;
    assert.ok(code === null || Number.isInteger(code), 'companion did not exit');
    await rmdir(cwd);
  });

  const responses = messages(child.stdout);
  async function receive(stage) {
    const response = await responses.next();
    if (response.done) assert.fail(`companion stream closed during ${stage}; helper status ${await closed}`);
    return response.value;
  }

  const send = message => child.stdin.write(JSON.stringify(message) + '\n');
  async function pollUntil(stage, matches) {
    const expires = Date.now() + 6000;
    do {
      send({ type: 'poll' });
      const reply = await receive(stage);
      if (matches(reply)) return reply;
      if (reply.type === 'exit') assert.fail(`PowerShell exited during ${stage}; shell status ${reply.status ?? 'unavailable'}`);
      assert.ok(reply.type === 'idle' || reply.type === 'frame', 'unexpected companion response');
      await delay(pollInterval, undefined, { signal: t.signal });
    } while (Date.now() < expires);
    assert.fail(`no ${stage} after six seconds`);
  }

  const ready = await receive('startup');
  assert.equal(ready.type, 'ready');
  assert.equal(ready.version, 1);
  send({ type: 'resize', cols: 117, rows: 14 });
  let sawPrompt = false;
  let sawContext = false;
  await pollUntil('PowerShell prompt and Windows context', reply => {
    sawPrompt ||= reply.type === 'frame' && /^PS [^\n]*>/m.test(visibleText(reply));
    sawContext ||= reply.context?.tags.some(tag => tag.role === 'windows' && tag.value === 'Windows') === true;
    return sawPrompt && sawContext;
  });

  send({ type: 'text', text: "Write-Output ('automexia-' + 'docked-probe')" });
  send({ type: 'key', key: 'Enter' });
  await pollUntil('typed command result', reply => reply.type === 'frame' && visibleText(reply).includes(marker));

  send({ type: 'text', text: `exit ${expectedCode}` });
  send({ type: 'key', key: 'Enter' });
  const exit = await pollUntil('shell exit status', reply => reply.type === 'exit');
  assert.equal(exit.status, expectedCode);
  assert.equal(await closed, 0, 'companion should close cleanly after reporting the shell status');
}

test('starts Windows PowerShell, runs typed input, and reports status', { timeout: 25_000 }, t => probePowerShell(t, []));
test('runs the same Windows PowerShell probe with startup profiles bypassed', { timeout: 25_000 }, t => probePowerShell(t, ['-NoProfile']));
