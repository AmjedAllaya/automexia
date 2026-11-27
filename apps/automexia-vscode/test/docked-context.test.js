const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');
const { validatedContext, validatedPresetCatalog, gitTag, knownGitRepositories } = require('../docked-context');

const context = Object.freeze({ cwd: '/work/project', cwdSource: 'reported', remote: false, tags: [] });

test('accepts a bounded native preset catalog and preserves role order', () => {
  const preset = {
    id: 'git-first-developer-bar', label: 'Git-first developer bar', visual: 'capsule',
    arrangement: 'flow', spacingPercent: 100,
    slots: [
      { role: 'git', lane: 'leading', iconOnly: false },
      { role: 'windows', lane: 'trailing', iconOnly: true },
    ],
  };
  assert.deepEqual(validatedPresetCatalog([{ ...preset, command: 'ignored' }]), [preset]);
  assert.equal(validatedPresetCatalog([preset, preset]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, slots: [preset.slots[0], preset.slots[0]] }]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, slots: [] }]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, visual: 'url(https://example.test)' }]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, spacingPercent: 301 }]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, label: 'a\nb' }]), undefined);
  assert.equal(validatedPresetCatalog([{ ...preset, label: 'é'.repeat(41) }]), undefined);
});

function repository(root, head) {
  return { rootUri: { fsPath: root }, state: { HEAD: head } };
}

test('projects only bounded display fields and supported tag roles', () => {
  const shell = { role: 'shell', label: 'Shell zsh', value: 'zsh' };
  const input = { ...context, tags: [{ ...shell, command: 'invalid' }], pathToOpen: '/private' };
  assert.deepEqual(validatedContext(input), { ...context, tags: [shell] });
  assert.equal(validatedContext({ ...context, tags: [shell, shell] }), undefined);
  assert.equal(validatedContext({ ...context, tags: [{ ...shell, role: 'unknown' }] }), undefined);
  assert.equal(validatedContext({ ...context, tags: [{ ...shell, role: 'git' }] }), undefined);
  assert.equal(validatedContext({ ...context, tags: [{ ...shell, value: 'x\u202ey' }] }), undefined);
  assert.equal(validatedContext({ ...context, tags: [{ ...shell, label: 'x\u2029y' }] }), undefined);
  assert.equal(validatedContext({ ...context, cwd: '/unsafe\npath' }), undefined);
  assert.equal(validatedContext({ ...context, cwd: 'é'.repeat(2049) }), undefined);
});

test('uses the most specific known repository and follows its current HEAD', () => {
  const nested = repository('/work/project', { name: 'main' });
  const repos = [repository('/work', { name: 'parent' }), nested];
  assert.deepEqual(gitTag({ ...context, cwd: '/work/project/src' }, repos, path.posix), {
    role: 'git', label: 'Git branch main', value: 'main',
  });
  nested.state.HEAD = { name: 'feature/context' };
  assert.equal(gitTag(context, repos, path.posix).value, 'feature/context');
  assert.equal(gitTag({ ...context, cwd: '/work/project-other' }, [nested], path.posix), undefined);
  assert.equal(gitTag({ ...context, cwd: '/work/project/../elsewhere' }, [nested], path.posix), undefined);
});

test('matches Windows paths and distinguishes launch context and detached HEAD', () => {
  const known = repository('C:\\Work\\Project', { commit: 'abcdef012345' });
  const windowsContext = { ...context, cwd: 'c:/work/project/src', cwdSource: 'launch' };
  assert.deepEqual(gitTag(windowsContext, [known], path.win32), {
    role: 'git', label: 'Starting directory Git detached HEAD abcdef01', value: 'detached@abcdef01',
  });
  assert.equal(gitTag({ ...windowsContext, cwd: 'D:\\Work\\Project' }, [known], path.win32), undefined);
});

test('hides local Git data for a remote shell and never activates Git on its own', () => {
  assert.equal(gitTag({ ...context, remote: true }, [repository(context.cwd, { name: 'main' })], path.posix), undefined);
  let reads = 0;
  const extension = { isActive: false, exports: { enabled: true, getAPI() { reads += 1; return { repositories: [] }; } } };
  const vscode = { extensions: { getExtension: id => id === 'vscode.git' ? extension : undefined } };
  assert.deepEqual(knownGitRepositories(vscode), []);
  assert.equal(reads, 0);
  extension.isActive = true;
  knownGitRepositories(vscode);
  assert.equal(reads, 1);
});
