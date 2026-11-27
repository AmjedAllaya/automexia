'use strict';

const bridge = acquireVsCodeApi();
const encoder = new TextEncoder();
const limits = { sessions: 8, splits: 4, columns: 240, rows: 120, textBytes: 4096, matches: 1000 };
const contextRoles = new Map([
  ['windows', 'OS'], ['ubuntu-wsl', 'OS'], ['shell', 'Shell'], ['user', 'User'], ['git', 'Git'],
]);
const barVisuals = new Set(['capsule', 'flat', 'chevron', 'hexagon', 'card', 'underline']);
const barArrangements = new Set(['flow', 'split', 'two-line', 'adaptive', 'dashboard', 'compact']);
const directorySources = new Set(['reported', 'launch']);
const unsafeContextText = /[\x00-\x1f\x7f-\x9f\u061c\u200e\u200f\u2028-\u202e\u2066-\u206f]/u;
const tabs = [];
const panes = new Map();
const pending = new Set();
let nextId = 1;
let activeTab;
let activePane;
let scheduled = false;
let cellWidth = 0;
let cellHeight = 0;

const tabsElement = document.getElementById('tabs');
const workspaces = document.getElementById('workspaces');
const welcome = document.getElementById('welcome');
const notice = document.getElementById('notice');
const newTabButton = document.getElementById('new-tab');
const splitButton = document.getElementById('split');
const findButton = document.getElementById('find');
const cellSample = document.getElementById('cell-sample');
const actions = document.getElementById('actions');
const actionsLabel = document.getElementById('actions-label');
const actionQuery = document.getElementById('action-query');
const actionQueryLabel = document.getElementById('action-query-label');
const actionList = document.getElementById('action-list');

const savedAppearance = bridge.getState();
const appearance = { theme: savedAppearance?.theme === 'arctic' ? 'arctic' : 'default' };
if (validPresetId(savedAppearance?.informationBarPreset)) {
  appearance.informationBarPreset = savedAppearance.informationBarPreset;
}
document.documentElement.dataset.theme = appearance.theme;
let informationBarPresets = new Map();

function post(type, fields = {}) {
  bridge.postMessage({ type, ...fields });
}

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(text, label, className, handler) {
  const node = element('button', className, text);
  node.type = 'button';
  node.title = label;
  node.setAttribute('aria-label', label);
  node.addEventListener('click', handler);
  return node;
}

function showNotice(message = '') {
  notice.textContent = message;
  notice.hidden = message.length === 0;
}

function contextText(value, maxBytes) {
  if (typeof value === 'string' && value.length <= maxBytes && value.trim()
    && !unsafeContextText.test(value) && encoder.encode(value).byteLength <= maxBytes) {
    return value;
  }
  return '';
}

function readContext(data) {
  if (!directorySources.has(data.cwdSource) || !Array.isArray(data.tags) || data.tags.length > contextRoles.size) {
    return undefined;
  }
  const tags = [];
  for (const item of data.tags) {
    if (item && contextRoles.has(item.role)) {
      const label = contextText(item.label, 512);
      const value = contextText(item.value, 384);
      if (label && value) tags.push({ role: item.role, label, value });
    }
  }
  return { cwd: contextText(data.cwd, 4096), cwdSource: data.cwdSource, tags };
}

function validPresetId(id) {
  return typeof id === 'string' && /^[a-z][a-z0-9-]{0,47}$/.test(id);
}

function readInformationBarPresets(data) {
  if (!Array.isArray(data) || data.length === 0 || data.length > 12) return undefined;
  const presets = new Map();
  for (const preset of data) {
    if (!preset || !validPresetId(preset.id) || presets.has(preset.id)
      || !barVisuals.has(preset.visual) || !barArrangements.has(preset.arrangement)
      || !Number.isInteger(preset.spacingPercent) || preset.spacingPercent < 0 || preset.spacingPercent > 300
      || !Array.isArray(preset.slots) || preset.slots.length === 0 || preset.slots.length > contextRoles.size) {
      return undefined;
    }
    const label = contextText(preset.label, 80);
    if (!label) return undefined;
    const slots = [];
    const seen = new Set();
    for (const slot of preset.slots) {
      if (!slot || !contextRoles.has(slot.role) || seen.has(slot.role)
        || (slot.lane !== 'leading' && slot.lane !== 'trailing') || typeof slot.iconOnly !== 'boolean') {
        return undefined;
      }
      seen.add(slot.role);
      slots.push({ role: slot.role, lane: slot.lane, iconOnly: slot.iconOnly });
    }
    presets.set(preset.id, {
      id: preset.id, label, visual: preset.visual, arrangement: preset.arrangement,
      spacingPercent: preset.spacingPercent, slots,
    });
  }
  return presets;
}

function currentInformationBarPreset() {
  return informationBarPresets.get(appearance.informationBarPreset)
    ?? informationBarPresets.get('rounded-capsules')
    ?? informationBarPresets.values().next().value;
}

function arrangedTags(items, preset) {
  if (!preset) return items;
  const tagsByRole = new Map(items.map(item => [item.role, item]));
  const included = [];
  for (const slot of preset.slots) {
    const tag = tagsByRole.get(slot.role);
    if (tag) included.push({ ...tag, lane: slot.lane, iconOnly: slot.iconOnly });
  }
  if (preset.arrangement === 'split') {
    return included.filter(item => item.lane === 'leading')
      .concat(included.filter(item => item.lane === 'trailing'));
  }
  return included;
}

function showContext(pane) {
  const context = pane.status === 'running' ? pane.context : undefined;
  const { bar, tags, directory, directoryLabel, directoryValue } = pane.information;
  const preset = currentInformationBarPreset();
  const items = context ? arrangedTags(context.tags, preset)
    .filter(item => item.role !== 'shell' || item.value !== pane.title.textContent) : [];
  bar.hidden = !context || (!context.cwd && items.length === 0);
  if (bar.hidden) {
    tags.replaceChildren();
    directoryValue.textContent = '';
    directory.removeAttribute('title');
    return;
  }

  bar.dataset.visual = preset?.visual ?? 'legacy';
  bar.dataset.arrangement = preset?.arrangement ?? 'flow';
  tags.style.setProperty('--context-tag-gap', `${4 * (preset?.spacingPercent ?? 100) / 100}px`);
  const firstLine = preset?.arrangement === 'two-line' ? directory : tags;
  if (bar.firstElementChild !== firstLine) bar.prepend(firstLine);

  tags.hidden = items.length === 0;
  tags.replaceChildren(...items.map(item => {
    const tag = element('li', 'context-tag');
    tag.dataset.role = item.role;
    tag.dataset.lane = item.lane ?? 'leading';
    const description = item.label.includes(item.value) ? item.label : `${item.label}: ${item.value}`;
    tag.title = description;
    tag.setAttribute('aria-label', description);
    if (item.iconOnly) {
      tag.classList.add('context-tag-icon-only');
      const icon = element('span', 'context-tag-icon', item.role === 'windows' ? '⊞' : '◈');
      icon.setAttribute('aria-hidden', 'true');
      tag.append(icon);
      return tag;
    }
    const kind = element('span', 'context-tag-kind',
      item.role === 'ubuntu-wsl' && item.label.startsWith('WSL distribution') ? 'WSL' : contextRoles.get(item.role));
    kind.setAttribute('aria-hidden', 'true');
    tag.append(kind, element('bdi', 'context-tag-value', item.value));
    return tag;
  }));

  directory.hidden = !context.cwd;
  directoryLabel.textContent = context.cwdSource === 'launch' ? 'Started in' : 'Directory';
  directoryValue.textContent = context.cwd;
  directory.title = context.cwd ? `${directoryLabel.textContent}: ${context.cwd}` : '';
}

function createInformationBar() {
  const bar = element('div', 'pane-context');
  bar.setAttribute('role', 'group');
  bar.setAttribute('aria-label', 'Session information');
  bar.hidden = true;
  const tags = element('ul', 'context-tags');
  tags.setAttribute('aria-label', 'Context tags');
  tags.tabIndex = 0;
  const directory = element('div', 'context-directory');
  const directoryLabel = element('span', 'context-directory-label');
  const directoryValue = element('bdi', 'context-directory-value');
  directory.append(directoryLabel, directoryValue);
  bar.append(tags, directory);
  return { bar, tags, directory, directoryLabel, directoryValue };
}

function syncSelection() {
  for (const tab of tabs) {
    const isActive = tab === activeTab;
    tab.node.classList.toggle('selected', isActive);
    tab.select.tabIndex = isActive ? 0 : -1;
    tab.select.setAttribute('aria-selected', String(isActive));
    tab.panel.hidden = !isActive;
    tab.panel.classList.toggle('multiple', tab.panes.length > 1);
    for (const pane of tab.panes) {
      pane.node.classList.toggle('selected', pane === activePane);
    }
  }
  welcome.hidden = tabs.length > 0;
  newTabButton.disabled = panes.size >= limits.sessions;
  splitButton.disabled = !activeTab || activeTab.panes.length >= limits.splits || newTabButton.disabled;
  findButton.disabled = !activePane?.frame;
}

function activatePane(pane, focusInput = false) {
  activePane = pane;
  activeTab = pane?.tab;
  syncSelection();
  if (activeTab) {
    for (const visiblePane of activeTab.panes) {
      visiblePane.needsMeasure = true;
      schedule(visiblePane);
    }
  }
  if (focusInput) pane?.input.focus({ preventScroll: true });
}

function selectTab(tab, focusInput = false) {
  activatePane(tab.activePane ?? tab.panes[0], focusInput);
}

function createTab(id) {
  const tab = { node: element('div', 'tab'), panel: element('section', 'tab-panel'), panes: [] };
  const name = `Terminal ${id}`;
  tab.node.setAttribute('role', 'presentation');
  tab.select = button(name, name, 'tab-name', () => selectTab(tab, true));
  tab.select.id = `tab-${id}`;
  tab.select.setAttribute('role', 'tab');
  tab.select.setAttribute('aria-controls', `workspace-${id}`);
  tab.close = button('×', `Close ${name}`, 'close-tab', () => closeTab(tab));
  tab.panel.id = `workspace-${id}`;
  tab.panel.setAttribute('role', 'tabpanel');
  tab.panel.setAttribute('aria-labelledby', tab.select.id);
  tab.node.append(tab.select, tab.close);
  tabsElement.append(tab.node);
  workspaces.append(tab.panel);
  tabs.push(tab);
  return tab;
}

function createPane(tab, id) {
  const pane = {
    tab, id, status: 'starting', rows: [], rowNodes: [], searchOpen: false,
    matchIndex: 0, matches: [], searchQuery: '', cols: 0, lines: 0, needsMeasure: true,
  };
  pane.node = element('section', 'pane');
  pane.node.setAttribute('aria-label', `Shell ${id}`);
  pane.node.dataset.state = pane.status;
  const chrome = element('header', 'pane-chrome');
  const header = element('div', 'pane-header');
  pane.title = element('span', 'title', `Shell ${id}`);
  const close = button('×', `Close shell ${id}`, 'quiet-button', () => closePane(pane));
  header.append(pane.title, close);

  const wrap = element('div', 'terminal-wrap');
  pane.grid = element('div', 'terminal-grid');
  pane.grid.setAttribute('role', 'group');
  pane.grid.setAttribute('aria-label', `Terminal output for shell ${id}`);
  pane.output = element('div', 'terminal-output');
  pane.message = element('p', 'terminal-message', 'Starting shell…');
  pane.cursor = element('span', 'cursor');
  pane.cursor.hidden = true;
  pane.cursor.setAttribute('aria-hidden', 'true');
  pane.input = element('textarea', 'terminal-input');
  pane.input.setAttribute('aria-label', `Shell ${id} input`);
  pane.input.setAttribute('aria-keyshortcuts', 'F6');
  pane.input.autocomplete = 'off';
  pane.input.spellcheck = false;
  pane.grid.append(pane.output, pane.message, pane.cursor, pane.input);
  wrap.append(pane.grid);

  const footer = element('footer', 'pane-footer');
  pane.stateLabel = element('span', 'pane-state', 'Starting');
  pane.stateLabel.setAttribute('role', 'status');
  pane.geometry = element('span', 'geometry');
  footer.append(pane.stateLabel, pane.geometry);

  pane.information = createInformationBar();
  pane.information.tags.addEventListener('focus', () => focusPane(pane));
  chrome.append(header, pane.information.bar);
  pane.search = createSearch(pane);
  pane.node.append(chrome, pane.search.node, wrap, footer);
  tab.panel.append(pane.node);
  tab.panes.push(pane);
  panes.set(id, pane);
  resizeObserver.observe(pane.grid);
  setupInput(pane);
  return pane;
}

function openSession(split = false) {
  if (panes.size >= limits.sessions || (split && (!activeTab || activeTab.panes.length >= limits.splits))) return;
  const id = nextId++;
  const tab = split ? activeTab : createTab(id);
  const pane = createPane(tab, id);
  tab.activePane = pane;
  showNotice();
  post('open', { id });
  activatePane(pane, true);
}

function releasePane(pane) {
  post('close', { id: pane.id });
  resizeObserver.unobserve(pane.grid);
  pending.delete(pane);
  panes.delete(pane.id);
  pane.node.remove();
}

function closeTab(tab) {
  const index = tabs.indexOf(tab);
  for (const pane of tab.panes) releasePane(pane);
  tab.panel.remove();
  tab.node.remove();
  tabs.splice(index, 1);
  if (activeTab === tab) {
    const adjacent = tabs[Math.min(index, tabs.length - 1)];
    if (adjacent) selectTab(adjacent, true);
    else activatePane(undefined);
  } else syncSelection();
  if (!activeTab) document.getElementById('first-tab').focus();
}

function closePane(pane) {
  const { tab } = pane;
  if (tab.panes.length === 1) {
    closeTab(tab);
    return;
  }
  const index = tab.panes.indexOf(pane);
  releasePane(pane);
  tab.panes.splice(index, 1);
  if (tab.activePane === pane) tab.activePane = tab.panes[Math.min(index, tab.panes.length - 1)];
  if (tab === activeTab) selectTab(tab, true);
  else syncSelection();
}

function focusPane(pane) {
  pane.tab.activePane = pane;
  if (pane !== activePane) activatePane(pane);
}

function selectionFor(pane) {
  const selection = window.getSelection();
  if (selection?.anchorNode && selection.focusNode &&
      pane.output.contains(selection.anchorNode) && pane.output.contains(selection.focusNode)) {
    return selection.toString();
  }
  return '';
}

function sendText(pane, type, text) {
  if (!text || pane.status !== 'running') return;
  if (text.length > limits.textBytes || /[\u0000-\u001f\u007f-\u009f]/u.test(text) || encoder.encode(text).length > limits.textBytes) {
    showNotice('Enter or paste up to 4096 bytes of single-line text.');
    return;
  }
  post(type, { id: pane.id, text });
}

function setupInput(pane) {
  let composing = false;
  let commit = '';
  const specialKeys = /^(?:Arrow(?:Up|Down|Left|Right)|Home|End|Page(?:Up|Down)|Escape|Enter|Tab|Backspace|Delete|Insert|F(?:[1-9]|1[0-2]))$/;
  pane.input.addEventListener('focus', () => focusPane(pane));
  pane.grid.addEventListener('pointerdown', () => focusPane(pane));
  pane.grid.addEventListener('pointerup', (event) => {
    if (event.button === 0 && !selectionFor(pane)) pane.input.focus({ preventScroll: true });
  });

  pane.input.addEventListener('keydown', (event) => {
    if (event.isComposing || composing) return;
    const { key, ctrlKey: ctrl, altKey: alt, shiftKey: shift, metaKey: meta } = event;
    const isModifier = ctrl || alt || meta;
    const shortcut = key.toLowerCase();
    if (key === 'F6' && !isModifier && !shift) {
      event.preventDefault();
      tabOrToolbar().focus();
      return;
    }
    if (pane.status !== 'running') return;
    if (shift && !isModifier && (key === 'PageUp' || key === 'PageDown')) {
      event.preventDefault();
      post('scroll', { id: pane.id, lines: (key === 'PageUp' ? 1 : -1) * Math.max(pane.lines, 1) });
      return;
    }
    if (((ctrl || meta) && (shortcut === 'v' || (shortcut === 'c' && selectionFor(pane)))) || (shift && key === 'Insert')) return;
    if (specialKeys.test(key) || (isModifier && key.length === 1 && !event.getModifierState('AltGraph'))) {
      event.preventDefault();
      post('key', { id: pane.id, key, ctrl, alt, shift, meta });
    }
  });

  pane.input.addEventListener('compositionstart', () => { composing = true; commit = ''; });
  pane.input.addEventListener('compositionend', (event) => {
    composing = false;
    commit = event.data;
    pane.input.value = '';
    sendText(pane, 'text', commit);
  });
  pane.input.addEventListener('input', (event) => {
    if (composing || event.isComposing) return;
    const text = pane.input.value;
    pane.input.value = '';
    if (commit && event.inputType === 'insertCompositionText' && event.data === commit) {
      commit = '';
      return;
    }
    commit = '';
    sendText(pane, 'text', text);
  });

  pane.input.addEventListener('paste', (event) => {
    event.preventDefault();
    sendText(pane, 'paste', event.clipboardData?.getData('text/plain'));
  });

  pane.grid.addEventListener('wheel', (event) => {
    if (pane.status !== 'running' || event.ctrlKey || event.metaKey || !event.deltaY) return;
    event.preventDefault();
    const lines = event.deltaMode === WheelEvent.DOM_DELTA_PIXEL
      ? Math.ceil(Math.abs(event.deltaY) / Math.max(1, cellHeight))
      : Math.ceil(Math.abs(event.deltaY));
    post('scroll', { id: pane.id, lines: -Math.sign(event.deltaY) * Math.min(lines, 120) });
  }, { passive: false });
}

function tabOrToolbar() {
  return activeTab?.select ?? newTabButton;
}

function schedule(pane) {
  pending.add(pane);
  if (scheduled) return;
  scheduled = true;
  requestAnimationFrame(() => {
    scheduled = false;
    for (const changed of pending) {
      if (changed.tab === activeTab) {
        if (changed.needsMeasure) measure(changed);
        paint(changed);
      }
    }
    pending.clear();
  });
}

const resizeObserver = new ResizeObserver((entries) => {
  for (const entry of entries) {
    if (entry.target === cellSample) {
      cellWidth = entry.contentRect.width;
      cellHeight = entry.contentRect.height;
      for (const pane of activeTab?.panes ?? []) {
        pane.needsMeasure = true;
        schedule(pane);
      }
      continue;
    }
    const pane = [...panes.values()].find(({ grid }) => grid === entry.target);
    if (pane) {
      pane.needsMeasure = true;
      schedule(pane);
    }
  }
});
resizeObserver.observe(cellSample);

function measure(pane) {
  if (pane.status !== 'running' || cellWidth <= 0 || cellHeight <= 0) return;
  pane.needsMeasure = false;
  const cols = Math.min(limits.columns, Math.floor(pane.grid.clientWidth / cellWidth));
  const rows = Math.min(limits.rows, Math.floor(pane.grid.clientHeight / cellHeight));
  if (cols >= 2 && rows >= 1 && (cols !== pane.cols || rows !== pane.lines)) {
    pane.cols = cols;
    pane.lines = rows;
    post('resize', { id: pane.id, cols, rows });
  }
}

function validFrame(frame) {
  if (!Number.isInteger(frame.cols) || frame.cols < 1 || frame.cols > limits.columns ||
      !Array.isArray(frame.rows) || frame.rows.length > limits.rows) return false;
  return frame.rows.every((row) => {
    if (!Array.isArray(row) || row.length > frame.cols) return false;
    let width = 0;
    return row.every((run) => {
      if (!Array.isArray(run) || typeof run[0] !== 'string' || run[0].length > limits.textBytes ||
          !Number.isInteger(run[3]) || !Number.isInteger(run[4]) || run[4] < 1) return false;
      width += run[4];
      return width <= frame.cols;
    });
  });
}

function equalRow(left, right) {
  return left?.length === right.length && right.every((run, index) =>
    run[0] === left[index][0] && run[1] === left[index][1] && run[2] === left[index][2] &&
    run[3] === left[index][3] && run[4] === left[index][4]);
}

function paint(pane) {
  if (!pane.frame) return;
  const { rows, cols, cursor } = pane.frame;
  const selectionActive = Boolean(selectionFor(pane));
  if (pane.paintFrame !== pane.frame || pane.searchChanged) {
    const searching = pane.searchOpen && Boolean(pane.searchQuery);
    const repaintSearch = pane.searchChanged || (searching &&
      (pane.rows.length !== rows.length || rows.some((row, line) => !equalRow(pane.rows[line], row))));
    if (repaintSearch) buildMatches(pane, rows);
    // Preserve a live browser selection until the user copies or clicks away.
    if (!selectionActive || pane.searchChanged) {
      for (let line = 0; line < rows.length; line++) {
        if (!pane.rowNodes[line]) {
          pane.rowNodes[line] = element('div', 'terminal-row');
          pane.output.append(pane.rowNodes[line]);
        }
        if (repaintSearch || !equalRow(pane.rows[line], rows[line])) {
          paintRow(pane.rowNodes[line], rows[line], searching ? pane.matches[line] : []);
        }
      }
      while (pane.rowNodes.length > rows.length) pane.rowNodes.pop().remove();
      pane.rows = rows;
      pane.paintFrame = pane.frame;
      pane.searchChanged = false;
    }
  }
  const geometry = `${cols} × ${rows.length}`;
  if (pane.geometry.textContent !== geometry) pane.geometry.textContent = geometry;
  const cursorValid = !selectionActive && pane.status === 'running' && cursor?.visible &&
    Number.isInteger(cursor.col) && Number.isInteger(cursor.row) && cursor.col >= 0 &&
    cursor.col < cols && cursor.row >= 0 && cursor.row < rows.length && cellWidth > 0 && cellHeight > 0;
  pane.cursor.hidden = !cursorValid;
  if (cursorValid) {
    const position = `translate(${cursor.col * cellWidth}px, ${cursor.row * cellHeight}px)`;
    if (pane.cursor.style.transform !== position || pane.cursor.style.width !== `${cellWidth}px` ||
        pane.cursor.style.height !== `${cellHeight}px`) {
      pane.cursor.style.width = `${cellWidth}px`;
      pane.cursor.style.height = `${cellHeight}px`;
      pane.cursor.style.transform = position;
      pane.input.style.transform = position;
    }
  }
}

function paintRow(node, runs, matches = []) {
  const content = document.createDocumentFragment();
  let textOffset = 0;
  let currentMatch = 0;
  const hex = /^#[a-f0-9]{6}$/i;
  for (const [text, foreground, background, flags, columns] of runs) {
    const run = element('span', 'terminal-run');
    const value = text || ' '.repeat(columns);
    const runEnd = textOffset + value.length;
    run.style.width = `${columns}ch`;
    if (typeof foreground === 'string' && hex.test(foreground)) run.style.color = foreground;
    if (typeof background === 'string' && hex.test(background)) run.style.backgroundColor = background;
    if (flags & 1) run.classList.add('bold');
    if (flags & 2) run.classList.add('italic');
    if (flags & 4) run.classList.add('underline');

    let previous = 0;
    while (currentMatch < matches.length && matches[currentMatch].start < runEnd) {
      const match = matches[currentMatch];
      const start = Math.max(0, match.start - textOffset);
      const end = Math.min(value.length, match.end - textOffset);
      if (start > previous) run.append(value.slice(previous, start));
      if (end > start) run.append(element('mark', match.focused ? 'focused-match' : '', value.slice(start, end)));
      previous = end;
      if (match.end > runEnd) break;
      currentMatch++;
    }
    run.append(value.slice(previous));
    content.append(run);
    textOffset = runEnd;
  }
  node.replaceChildren(content);
}

function buildMatches(pane, rows) {
  let count = 0;
  const query = pane.searchQuery;
  const matches = rows.map((row) => {
    const rowMatches = [];
    if (query && count < limits.matches) {
      const text = row.map(([value, , , , width]) => value || ' '.repeat(width)).join('');
      let start = text.indexOf(query);
      while (start >= 0 && count < limits.matches) {
        const end = start + query.length;
        rowMatches.push({ start, end, index: count++ });
        start = text.indexOf(query, end);
      }
    }
    return rowMatches;
  });
  pane.matchIndex = count ? ((pane.matchIndex % count) + count) % count : 0;
  for (const row of matches) {
    for (const match of row) match.focused = match.index === pane.matchIndex;
  }
  pane.matches = matches;
  pane.search.label.textContent = !query ? '' : count ? `${pane.matchIndex + 1} / ${count}` : 'No matches';
}

function updateSearch(pane) {
  pane.searchChanged = true;
  schedule(pane);
}

function showSearch(pane) {
  if (!pane?.frame) return;
  pane.searchOpen = true;
  pane.search.node.hidden = false;
  pane.search.field.focus();
  pane.search.field.select();
  updateSearch(pane);
}

function closeSearch(pane) {
  pane.searchOpen = false;
  pane.search.node.hidden = true;
  pane.input.focus({ preventScroll: true });
  updateSearch(pane);
}

function createSearch(pane) {
  const node = element('div', 'search');
  node.hidden = true;
  node.setAttribute('role', 'search');
  const field = element('input');
  field.type = 'search';
  field.maxLength = 160;
  field.autocomplete = 'off';
  field.placeholder = 'Find in visible output';
  field.setAttribute('aria-label', 'Find in visible output');
  const label = element('span', 'matches');
  label.setAttribute('aria-live', 'polite');
  function move(delta) {
    pane.matchIndex += delta;
    updateSearch(pane);
  }
  field.addEventListener('input', () => {
    pane.searchQuery = field.value;
    pane.matchIndex = 0;
    updateSearch(pane);
  });
  field.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeSearch(pane);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      move(event.shiftKey ? -1 : 1);
    }
  });
  node.append(field, label, button('↑', 'Previous match', 'quiet-button', () => move(-1)),
    button('↓', 'Next match', 'quiet-button', () => move(1)),
    button('×', 'Close search', 'quiet-button', () => closeSearch(pane)));
  return { node, field, label };
}

function useTheme(theme) {
  document.documentElement.dataset.theme = theme;
  appearance.theme = theme;
  bridge.setState(appearance);
  activePane?.input.focus({ preventScroll: true });
}

function useInformationBarPreset(id) {
  appearance.informationBarPreset = id;
  bridge.setState(appearance);
  for (const pane of panes.values()) showContext(pane);
  activePane?.input.focus({ preventScroll: true });
}

const commands = [
  { label: 'New terminal tab', enabled: () => !newTabButton.disabled, run: () => openSession() },
  { label: 'Split terminal pane', enabled: () => !splitButton.disabled, run: () => openSession(true) },
  { label: 'Find in visible output', enabled: () => !findButton.disabled, run: () => showSearch(activePane) },
  { label: 'Close active pane', enabled: () => !!activePane, run: () => closePane(activePane) },
  { label: 'Browse Quick Actions', enabled: () => true, run: () => post('browse-quick-actions') },
  { label: 'Browse Saved Workspaces', enabled: () => true, run: () => post('browse-workspaces') },
  { label: 'Open Desktop App', enabled: () => true, run: () => post('open-desktop') },
  {
    label: 'Choose information bar preset', enabled: () => informationBarPresets.size > 0,
    keepOpen: true, run: () => preparePalette(true),
  },
  { label: 'Use Liquid Hacker theme', enabled: () => true, run: () => useTheme('default') },
  { label: 'Use Arctic Day theme', enabled: () => true, run: () => useTheme('arctic') },
];

let commandMatches = [];
let selectedCommand = 0;
let returnFocus;
let choosingBarPreset = false;

function chooseCommand(command) {
  if (command.keepOpen) {
    command.run();
    return;
  }
  returnFocus = undefined;
  actions.close();
  command.run();
  if (document.activeElement === document.body) tabOrToolbar().focus();
}

function renderCommands() {
  const query = actionQuery.value.trim().toLowerCase();
  const currentPreset = currentInformationBarPreset();
  const choices = choosingBarPreset
    ? Array.from(informationBarPresets.values(), preset => ({
      label: preset.label, current: currentPreset === preset, run: () => useInformationBarPreset(preset.id),
    }))
    : commands.filter(command => command.enabled());
  commandMatches = choices.filter(({ label }) => label.toLowerCase().includes(query));
  selectedCommand = Math.min(selectedCommand, Math.max(0, commandMatches.length - 1));
  actionList.replaceChildren(...commandMatches.map((command, index) => {
    const label = command.current ? `${command.label}, current preset` : command.label;
    const item = button(command.label, label, '', () => chooseCommand(command));
    item.id = `action-${index}`;
    item.tabIndex = -1;
    item.setAttribute('role', 'option');
    item.setAttribute('aria-selected', String(index === selectedCommand));
    return item;
  }));
  if (!commandMatches.length) {
    actionQuery.removeAttribute('aria-activedescendant');
    actionList.append(element('p', '', choosingBarPreset ? 'No matching presets' : 'No matching commands'));
  } else {
    actionQuery.setAttribute('aria-activedescendant', `action-${selectedCommand}`);
    if (actions.open) actionList.children[selectedCommand].scrollIntoView({ block: 'nearest' });
  }
}

function preparePalette(presets) {
  choosingBarPreset = presets;
  actionsLabel.textContent = presets ? 'Information bar presets' : 'Automexia commands';
  actionQueryLabel.textContent = presets ? 'Find a preset' : 'Find a command';
  actionQuery.placeholder = actionQueryLabel.textContent;
  actionList.setAttribute('aria-label', presets ? 'Information bar presets' : 'Commands');
  actionQuery.value = '';
  const currentPreset = currentInformationBarPreset();
  selectedCommand = presets ? Math.max(0, [...informationBarPresets.values()].indexOf(currentPreset)) : 0;
  renderCommands();
  if (actions.open) actionQuery.focus();
}

function openPalette() {
  returnFocus = document.activeElement;
  preparePalette(false);
  actions.showModal();
  actionQuery.focus();
}

actionQuery.addEventListener('input', () => { selectedCommand = 0; renderCommands(); });
actionQuery.addEventListener('keydown', (event) => {
  if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && commandMatches.length) {
    event.preventDefault();
    const delta = event.key === 'ArrowDown' ? 1 : -1;
    selectedCommand = (selectedCommand + delta + commandMatches.length) % commandMatches.length;
    renderCommands();
  } else if (event.key === 'Enter' && commandMatches[selectedCommand]) {
    event.preventDefault();
    chooseCommand(commandMatches[selectedCommand]);
  }
});
actions.addEventListener('close', () => {
  if (returnFocus?.isConnected && !returnFocus.disabled) returnFocus.focus();
  returnFocus = undefined;
});

tabsElement.addEventListener('keydown', (event) => {
  const index = tabs.findIndex((tab) => tab.select === event.target);
  if (index < 0) return;
  const key = event.key;
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(key)) return;
  event.preventDefault();
  let nextIndex = key === 'Home' ? 0 : tabs.length - 1;
  if (key === 'ArrowLeft' || key === 'ArrowRight') {
    nextIndex = (index + (key === 'ArrowLeft' ? -1 : 1) + tabs.length) % tabs.length;
  }
  const tab = tabs[nextIndex];
  selectTab(tab);
  tab.select.focus();
});

document.addEventListener('selectionchange', () => {
  if (activePane && !selectionFor(activePane)) schedule(activePane);
});
document.addEventListener('copy', (event) => {
  if (!activePane) return;
  const text = selectionFor(activePane);
  if (!text) return;
  event.preventDefault();
  if (text.length > limits.textBytes || encoder.encode(text).length > limits.textBytes) {
    showNotice('Select up to 4096 bytes to copy.');
    return;
  }
  post('copy', { id: activePane.id, text });
});
newTabButton.addEventListener('click', () => openSession());
document.getElementById('first-tab').addEventListener('click', () => openSession());
splitButton.addEventListener('click', () => openSession(true));
findButton.addEventListener('click', () => showSearch(activePane));
document.getElementById('palette').addEventListener('click', openPalette);

const sessionLabels = new Map([['starting', 'Starting'], ['running', 'Live'], ['exited', 'Exited'], ['error', 'Error']]);

window.addEventListener('message', ({ data }) => {
  if (!data || typeof data !== 'object') return;
  if (data.type === 'informationBarPresets') {
    const presets = readInformationBarPresets(data.presets);
    if (presets) {
      informationBarPresets = presets;
      for (const pane of panes.values()) showContext(pane);
      if (actions.open) renderCommands();
    }
    return;
  }
  if (data.type === 'status' && typeof data.message === 'string') {
    showNotice(data.message.slice(0, 512));
    return;
  }
  const pane = panes.get(data.id);
  if (!pane) return;
  if (data.type === 'context' && (pane.status === 'starting' || pane.status === 'running')) {
    const context = readContext(data);
    if (context) {
      pane.context = context;
      showContext(pane);
    }
  } else if (data.type === 'frame' && validFrame(data)) {
    pane.frame = data;
    if (pane.status === 'running') pane.message.hidden = true;
    if (pane === activePane) findButton.disabled = false;
    schedule(pane);
  } else if (data.type === 'session' && sessionLabels.has(data.status)) {
    pane.status = data.status;
    pane.node.dataset.state = pane.status;
    pane.stateLabel.textContent = sessionLabels.get(pane.status);
    if (typeof data.title === 'string' && data.title) {
      const title = data.title.slice(0, 160);
      pane.title.textContent = title;
      pane.title.title = title;
    }
    if (typeof data.message === 'string' && data.message) pane.message.textContent = data.message.slice(0, 512);
    else if (pane.status === 'exited') pane.message.textContent = 'Shell exited.';
    pane.message.hidden = pane.status === 'running' && Boolean(pane.frame);
    if (pane.status === 'running') schedule(pane);
    if (pane.status === 'exited' || pane.status === 'error') {
      pane.cursor.hidden = true;
      pane.context = undefined;
    }
    if (pane.context || !pane.information.bar.hidden) showContext(pane);
  }
});

post('ready');
