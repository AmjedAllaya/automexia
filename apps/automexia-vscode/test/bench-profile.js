const assert = require('node:assert/strict');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const { activateExtension, createExecutables, temporaryDirectory } = require('./profile-harness');
const { validFrame } = require('../docked-view');
const { validatedContext, gitTag } = require('../docked-context');

async function measure(label, extension, expectedEnvironment) {
  const warmupCount = 100;
  const sampleCount = 1000;
  const durations = [];

  for (let trial = 0; trial < warmupCount + sampleCount; trial += 1) {
    const start = performance.now();
    const profile = await extension.open();
    const duration = performance.now() - start;
    assert.deepEqual(profile.options.env, expectedEnvironment);
    if (trial >= warmupCount) {
      durations.push(duration);
    }
  }

  durations.sort((first, second) => first - second);
  const percentile = (value) => durations[Math.ceil(sampleCount * value) - 1].toFixed(3);
  console.log(`${label}: ${sampleCount} samples, p50 ${percentile(0.50)} ms, p95 ${percentile(0.95)} ms`);
}

async function main() {
  await measure('Inherited PATH', activateExtension(), undefined);

  const { directory, cleanup } = await temporaryDirectory();
  try {
    const binaryDirectory = path.join(directory, 'bin');
    await createExecutables(binaryDirectory);
    const extension = activateExtension({ home: directory, binaryDirectory });
    await measure('Configured CLI directory', extension, { PATH: `${binaryDirectory}:\${env:PATH}` });
    const bundledDirectory = path.join(directory, 'runtime', 'linux-x64', 'desktop');
    await createExecutables(bundledDirectory);
    const packaged = activateExtension({ extensionDirectory: directory, expectsBundle: true });
    await measure('Packaged CLI directory', packaged, { PATH: `${bundledDirectory}:\${env:PATH}` });
  } finally {
    await cleanup();
  }

  const row = Array.from({ length: 80 }, () => ['abc', '#68d5ff', null, 1, 3]);
  const frame = { cols: 240, rows: Array.from({ length: 120 }, () => row), cursor: { row: 23, col: 0, visible: true } };
  for (let index = 0; index < 100; index += 1) {
    assert.equal(validFrame(frame), true);
  }
  const start = performance.now();
  for (let index = 0; index < 1000; index += 1) {
    assert.equal(validFrame(frame), true);
  }
  console.log(`Validate 1,000 styled 240 × 120 frames: ${(performance.now() - start).toFixed(3)} ms`);

  const cwd = '/work/project127/src';
  const info = { cwd, cwdSource: 'reported', remote: false, tags: [{ role: 'shell', label: 'Shell Bash', value: 'Bash' }] };
  const repositories = Array.from({ length: 128 }, (_, index) => ({
    rootUri: { fsPath: '/work/project' + index }, state: { HEAD: { name: 'main' } },
  }));
  const contextStart = performance.now();
  for (let index = 0; index < 1000; index += 1) {
    assert.equal(gitTag(validatedContext(info), repositories, path.posix).value, 'main');
  }
  console.log(`Validate and match 1,000 context snapshots: ${(performance.now() - contextStart).toFixed(3)} ms`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
