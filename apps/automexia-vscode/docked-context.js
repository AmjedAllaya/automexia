const path = require('node:path');

const roles = new Set(['windows', 'ubuntu-wsl', 'shell', 'user']);
const presetRoles = new Set([...roles, 'git']);
const presetVisuals = new Set(['capsule', 'flat', 'chevron', 'hexagon', 'card', 'underline']);
const presetArrangements = new Set(['flow', 'split', 'two-line', 'adaptive', 'dashboard', 'compact']);
const presetLanes = new Set(['leading', 'trailing']);
const directorySources = new Set(['launch', 'reported']);
const unsafeDisplayCharacter = /[\x00-\x1f\x7f-\x9f\u061c\u200e\u200f\u2028-\u202e\u2066-\u206f]/u;
const maxKnownRepositories = 128;

function displayText(value, maxBytes) {
  return typeof value === 'string' && value.trim().length > 0
    && Buffer.byteLength(value) <= maxBytes && !unsafeDisplayCharacter.test(value);
}

function validatedContext(data) {
  if (!data || !directorySources.has(data.cwdSource) || typeof data.remote !== 'boolean'
    || (data.cwd !== null && !displayText(data.cwd, 4096)) || !Array.isArray(data.tags)
    || data.tags.length > roles.size) {
    return undefined;
  }
  const seen = new Set();
  const tags = [];
  for (const tag of data.tags) {
    if (!tag || !roles.has(tag.role) || seen.has(tag.role)
      || !displayText(tag.label, 512) || !displayText(tag.value, 384)) {
      return undefined;
    }
    seen.add(tag.role);
    tags.push({ role: tag.role, label: tag.label, value: tag.value });
  }
  return { cwd: data.cwd, cwdSource: data.cwdSource, remote: data.remote, tags };
}

function validatedPresetCatalog(data) {
  if (!Array.isArray(data) || data.length === 0 || data.length > 12) {
    return undefined;
  }
  const ids = new Set();
  const presets = [];
  for (const preset of data) {
    if (!preset || typeof preset.id !== 'string' || !/^[a-z][a-z0-9-]{0,47}$/.test(preset.id)
      || ids.has(preset.id) || !displayText(preset.label, 80) || !presetVisuals.has(preset.visual)
      || !presetArrangements.has(preset.arrangement) || !Number.isInteger(preset.spacingPercent)
      || preset.spacingPercent < 0 || preset.spacingPercent > 300 || !Array.isArray(preset.slots)
      || preset.slots.length === 0 || preset.slots.length > presetRoles.size) {
      return undefined;
    }
    const seen = new Set();
    const slots = [];
    for (const slot of preset.slots) {
      if (!slot || !presetRoles.has(slot.role) || seen.has(slot.role)
        || !presetLanes.has(slot.lane) || typeof slot.iconOnly !== 'boolean') {
        return undefined;
      }
      seen.add(slot.role);
      slots.push({ role: slot.role, lane: slot.lane, iconOnly: slot.iconOnly });
    }
    ids.add(preset.id);
    const { id, label, visual, arrangement, spacingPercent } = preset;
    presets.push({ id, label, visual, arrangement, spacingPercent, slots });
  }
  return presets;
}

function gitTag(context, repositories, hostPath = path) {
  const { cwd, cwdSource, remote } = context;
  if (!cwd || remote || !hostPath.isAbsolute(cwd) || !Array.isArray(repositories)) {
    return undefined;
  }

  // Select only among repositories already known to VS Code. A pathname from
  // terminal output cannot open a repository or trigger a filesystem read.
  let matchingRepository;
  let specificity = -1;
  for (const repo of repositories.slice(0, maxKnownRepositories)) {
    const root = repo?.rootUri?.fsPath;
    if (!displayText(root, 4096) || !hostPath.isAbsolute(root)) {
      continue;
    }
    const relative = hostPath.relative(root, cwd);
    const rootLength = hostPath.resolve(root).length;
    if (!hostPath.isAbsolute(relative) && relative !== '..' && !relative.startsWith('..' + hostPath.sep)
      && rootLength > specificity) {
      matchingRepository = repo;
      specificity = rootLength;
    }
  }

  const head = matchingRepository?.state?.HEAD;
  const prefix = cwdSource === 'launch' ? 'Starting directory Git' : 'Git';
  if (displayText(head?.name, 384)) {
    return { role: 'git', label: `${prefix} branch ${head.name}`, value: head.name };
  }
  if (typeof head?.commit === 'string' && /^[a-f0-9]{7,64}$/i.test(head.commit)) {
    const commit = head.commit.slice(0, 8);
    return { role: 'git', label: `${prefix} detached HEAD ${commit}`, value: `detached@${commit}` };
  }
  return undefined;
}

function knownGitRepositories(vscode) {
  try {
    const extension = vscode.extensions.getExtension('vscode.git');
    if (!extension?.isActive || !extension.exports?.enabled) {
      return [];
    }
    return extension.exports.getAPI(1).repositories;
  } catch {
    return [];
  }
}

module.exports = { validatedContext, validatedPresetCatalog, gitTag, knownGitRepositories };
