use rio_backend::config::hints::Hint;
use rio_backend::crosswords::pos::{Column, Line, Pos};
use rio_backend::event::EventListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
#[path = "hints_tests.rs"]
mod keyboard_tests;

pub(crate) mod preview;
mod scan;
pub(crate) const MAX_HINT_MATCHES: usize = 256;
pub(crate) const MAX_HINT_BYTES: usize = 4096;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum KeyIntent {
    Consume,
    Close,
    Next(bool),
    Pan(bool),
    Activate,
    Copy,
    Backspace,
    Label(char),
}

pub(crate) fn key_intent(
    key: &rio_window::keyboard::Key,
    mods: rio_window::keyboard::ModifiersState,
    pressed: bool,
    repeat: bool,
) -> KeyIntent {
    use rio_window::keyboard::{Key, ModifiersState, NamedKey};
    if !pressed || repeat {
        return KeyIntent::Consume;
    }
    match key.as_ref() {
        Key::Named(NamedKey::Escape) => KeyIntent::Close,
        Key::Named(NamedKey::Tab) => KeyIntent::Next(!mods.shift_key()),
        Key::Named(NamedKey::ArrowDown) => KeyIntent::Next(true),
        Key::Named(NamedKey::ArrowUp) => KeyIntent::Next(false),
        Key::Named(NamedKey::ArrowRight) => KeyIntent::Pan(true),
        Key::Named(NamedKey::ArrowLeft) => KeyIntent::Pan(false),
        Key::Named(NamedKey::Enter) if mods.is_empty() => KeyIntent::Activate,
        Key::Character(c)
            if c.eq_ignore_ascii_case("c")
                && ((mods.control_key() && mods.shift_key()) || mods.super_key()) =>
        {
            KeyIntent::Copy
        }
        Key::Character(c) if c.eq_ignore_ascii_case("c") && mods.control_key() => {
            KeyIntent::Close
        }
        Key::Named(NamedKey::Backspace) => KeyIntent::Backspace,
        Key::Character(text)
            if !mods.intersects(
                ModifiersState::CONTROL | ModifiersState::ALT | ModifiersState::SUPER,
            ) =>
        {
            text.chars()
                .next()
                .map_or(KeyIntent::Consume, KeyIntent::Label)
        }
        _ => KeyIntent::Consume,
    }
}

/// State for hint selection mode
pub struct HintState {
    /// Currently active hint configuration
    active_hint: Option<Rc<Hint>>,

    /// Visible matches for the current hint
    matches: Vec<HintMatch>,

    /// Labels for each match (as Vec<char>)
    labels: Vec<Vec<char>>,

    /// Keys pressed so far for hint selection
    keys: Vec<char>,

    /// Alphabet for generating labels
    alphabet: String,
    focused: usize,
    snapshot: Option<scan::Snapshot>,
    pub(crate) preview_offset: usize,
}

/// A match found by a hint
#[derive(Clone)]
pub struct HintMatch {
    /// The text that was matched
    pub text: String,

    /// Start position of the match
    pub start: Pos,

    /// End position of the match
    pub end: Pos,

    /// The hint configuration that created this match
    pub hint: Rc<Hint>,
}

impl std::fmt::Debug for HintMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HintMatch")
            .field("bytes", &self.text.len())
            .field("start", &self.start)
            .field("end", &self.end)
            .finish_non_exhaustive()
    }
}

impl HintMatch {
    /// Identity used by the press/release hint latch. The action is allowed
    /// only when the same visible match remains under the pointer; comparing
    /// text and both endpoints prevents a repaint from activating a different
    /// link that happens to reuse one screen position.
    pub fn same_visible_match(&self, other: &Self) -> bool {
        self.text == other.text && self.start == other.start && self.end == other.end
    }
}

impl HintState {
    pub fn new(alphabet: String) -> Self {
        Self {
            active_hint: None,
            matches: Vec::new(),
            labels: Vec::new(),
            keys: Vec::new(),
            alphabet,
            focused: 0,
            snapshot: None,
            preview_offset: 0,
        }
    }

    /// Check if hint mode is active
    pub fn is_active(&self) -> bool {
        self.active_hint.is_some()
    }

    /// Start hint mode with the given hint configuration
    pub fn start(&mut self, hint: Rc<Hint>) {
        self.preview_offset = 0;
        self.active_hint = Some(hint);
        self.keys.clear();
        self.matches.clear();
        self.labels.clear();
        self.focused = 0;
        self.snapshot = None;
    }

    /// Stop hint mode
    pub fn stop(&mut self) {
        self.active_hint = None;
        self.matches.clear();
        self.labels.clear();
        self.keys.clear();
        self.snapshot = None;
    }

    /// Update visible matches for the current hint
    pub fn update_matches<T: EventListener>(
        &mut self,
        term: &rio_backend::crosswords::Crosswords<T>,
    ) {
        self.matches.clear();

        let hint = match &self.active_hint {
            Some(hint) => hint.clone(),
            None => {
                return;
            }
        };

        self.snapshot = scan::Snapshot::capture(term);
        if self.snapshot.is_some() {
            self.matches = scan::collect(term, hint);
        }
        self.focused = 0;
        self.keys.clear();
        self.preview_offset = 0;
        self.generate_labels();
    }

    /// Handle keyboard input during hint selection
    pub fn keyboard_input<T: EventListener>(
        &mut self,
        _term: &rio_backend::crosswords::Crosswords<T>,
        c: char,
    ) -> Option<HintMatch> {
        match c {
            // Use backspace to remove the last character pressed
            '\x08' | '\x1f' => {
                self.keys.pop();
                self.preview_offset = 0;
                if let Some((index, _)) = self.visible_labels().first() {
                    self.focused = *index;
                }
                return None;
            }
            // Cancel hint highlighting on ESC/Ctrl+c
            '\x1b' | '\x03' => {
                self.stop();
                return None;
            }
            _ => (),
        }

        self.active_hint.as_ref()?;

        // Get visible labels (labels filtered by keys pressed so far)
        let visible_labels = self.visible_labels();

        // Find the last label starting with the input character
        let mut matching_labels = visible_labels.iter().rev();
        let (index, remaining_label) = matching_labels
            .find(|(_, remaining)| !remaining.is_empty() && remaining[0] == c)?;
        self.preview_offset = 0;

        // Check if this completes the label (only one character remaining)
        if remaining_label.len() == 1 {
            self.focused = *index;
            self.keys.clear();
            self.preview_offset = 0;
            None
        } else {
            // Store character to preserve the selection
            self.keys.push(c);
            self.focused = *index;
            None
        }
    }

    /// Get current matches
    pub fn matches(&self) -> &[HintMatch] {
        &self.matches
    }

    pub(crate) fn label_cells(&self) -> Vec<(Pos, char, bool)> {
        let mut cells = Vec::new();
        let columns = self.snapshot.as_ref().map_or(0, scan::Snapshot::columns);
        let mut occupied = std::collections::BTreeSet::new();
        for (index, label) in self.visible_labels() {
            let Some(target) = self.matches.get(index) else {
                continue;
            };
            // A clipped or overlapping token can identify the wrong link. Keep
            // labels whole; Tab and the preview retain access in tiny/dense views.
            if label.len() > columns {
                continue;
            }
            let first = target.start.col.0.min(columns - label.len());
            if (first..first + label.len())
                .any(|col| occupied.contains(&(target.start.row.0, col)))
            {
                continue;
            }
            for (offset, character) in label.into_iter().enumerate() {
                let col = first + offset;
                occupied.insert((target.start.row.0, col));
                cells.push((
                    Pos::new(target.start.row, Column(col)),
                    character,
                    offset == 0,
                ));
            }
        }
        cells
    }

    pub(crate) fn focused_label(&self) -> String {
        self.labels
            .get(self.focused)
            .map_or_else(String::new, |label| label.iter().collect())
    }

    pub(crate) fn focused(&self) -> Option<&HintMatch> {
        self.matches.get(self.focused)
    }

    pub(crate) fn focused_index(&self) -> usize {
        self.focused
    }

    pub(crate) fn cycle(&mut self, forward: bool) {
        self.keys.clear();
        self.preview_offset = 0;
        let count = self.matches.len();
        if count != 0 {
            self.focused = (self.focused + if forward { 1 } else { count - 1 }) % count;
        }
    }

    pub(crate) fn pan(&mut self, forward: bool) {
        let count = self.focused().map_or(0, |m| m.text.graphemes(true).count());
        self.preview_offset = if forward {
            self.preview_offset
                .saturating_add(16)
                .min(count.saturating_sub(1))
        } else {
            self.preview_offset.saturating_sub(16)
        };
    }

    pub(crate) fn activate(&mut self) -> Option<HintMatch> {
        let selected = self.focused()?.clone();
        if !selected.hint.persist {
            self.stop();
        } else {
            self.keys.clear();
        }
        Some(selected)
    }

    pub(crate) fn is_current<T: EventListener>(
        &self,
        term: &rio_backend::crosswords::Crosswords<T>,
    ) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.matches(term))
    }

    pub(crate) fn focus_in_lower_half(&self) -> bool {
        self.focused()
            .zip(self.snapshot.as_ref())
            .is_some_and(|(selected, snapshot)| snapshot.lower_half(selected.start.row))
    }

    /// Get keys pressed so far
    #[allow(dead_code)]
    pub fn keys_pressed(&self) -> &[char] {
        &self.keys
    }

    /// Get visible labels (filtered by current input)
    pub fn visible_labels(&self) -> Vec<(usize, Vec<char>)> {
        let keys_len = self.keys.len();
        self.labels
            .iter()
            .enumerate()
            .filter_map(|(i, label)| {
                if label.len() >= keys_len && label[..keys_len] == self.keys[..] {
                    let remaining: Vec<char> = label[keys_len..].to_vec();
                    Some((i, remaining))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update the alphabet used for hint labels
    #[allow(dead_code)]
    pub fn update_alphabet(&mut self, alphabet: &str) {
        if self.alphabet != alphabet {
            self.alphabet = alphabet.to_string();
            self.keys.clear();
        }
    }

    fn generate_labels(&mut self) {
        self.labels.clear();
        let mut alphabet: Vec<char> = self
            .alphabet
            .chars()
            .filter(|c| c.is_ascii_graphic())
            .take(64)
            .collect();
        let mut seen = std::collections::BTreeSet::new();
        alphabet.retain(|c| seen.insert(*c));
        if alphabet.len() < 2 {
            alphabet = "jfkdlsahgurieowpq".chars().collect();
        }
        let mut digits = 1;
        let mut capacity = alphabet.len();
        while capacity < self.matches.len() {
            digits += 1;
            capacity *= alphabet.len();
        }
        for mut index in 0..self.matches.len() {
            let mut label = vec![alphabet[0]; digits];
            for c in label.iter_mut().rev() {
                *c = alphabet[index % alphabet.len()];
                index /= alphabet.len();
            }
            self.labels.push(label);
        }
    }
}

/// Validate the default opener boundary without I/O or shell interpretation.
pub(crate) fn safe_open_target(text: &str) -> bool {
    if !safe_hint_text(text) || text.starts_with('-') {
        return false;
    }
    match url::Url::parse(text) {
        Ok(url) => {
            if !url.username().is_empty() || url.password().is_some() {
                return false;
            }
            match url.scheme() {
                "http" | "https" | "ftp" | "git" | "gemini" | "gopher" => {
                    url.host_str().is_some()
                }
                "mailto" | "tel" | "magnet" | "ipfs" | "ipns" | "news" | "ssh" => {
                    !url.path().is_empty() || url.host_str().is_some()
                }
                "file" => url.host_str().is_none_or(|host| host == "localhost"),
                // Drive-qualified paths remain paths, not arbitrary URI handlers.
                _ => {
                    cfg!(windows)
                        && text.as_bytes().get(1) == Some(&b':')
                        && text
                            .as_bytes()
                            .get(2)
                            .is_some_and(|c| *c == b'\\' || *c == b'/')
                }
            }
        }
        Err(_) => !text.contains(':') && !text.starts_with("\\\\"),
    }
}

fn safe_hint_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_HINT_BYTES && !text.chars().any(|c|
        c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}

/// URI scheme prefixes that should never be resolved as file paths.
/// Matches the scheme branch of `DEFAULT_URL_REGEX`.
const URI_SCHEMES: &[&str] = &[
    "ipfs:",
    "ipns:",
    "magnet:",
    "mailto:",
    "gemini://",
    "gopher://",
    "https://",
    "http://",
    "news:",
    "file:",
    "git://",
    "ssh:",
    "ssh://",
    "ftp://",
    "tel:",
];

/// If `text` looks like a local filesystem path, resolve it against `cwd` and
/// return the absolute path when it exists on disk. Returns `None` for
/// URL-scheme strings, paths that don't exist, or anything we can't resolve
/// (e.g. relative path with no known `cwd`). On `None`, the caller should
/// fall back to the raw text and let the OS opener handle it.
///
/// Modelled on ghostty's `resolvePathForOpening` (`src/Surface.zig:2045`).
/// core only joins relative paths against the OSC 7 cwd; tilde
/// expansion lives in the macOS apprt's Swift `openURL`
/// (`.App.swift:715`, via `NSString.standardizingPath`), so `~/x`
/// works on macOS but isn't expanded on Linux/BSD where `xdg-open` gets the
/// literal `~`. The app doesn't have a per-platform opener layer, so we do the
/// expansion here to get consistent cross-platform behaviour:
///
/// 1. `~/x` and `~` expand via `dirs::home_dir()`.
/// 2. `$VAR/x` expands via `std::env::var` (ghostty doesn't do this on any
///    platform).
/// 3. Strings starting with a known URI scheme are rejected up front so the
///    OS opener routes them as URLs (saves one filesystem syscall vs
///    ghostty's "join cwd + stat → fail" path).
/// 4. Absolute paths are existence-checked too. short-circuits
///    absolute paths to `None` (caller passes raw); user-visible behaviour
///    is the same since the raw and resolved strings match.
pub fn resolve_path_for_opening(text: &str, cwd: Option<&Path>) -> Option<PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Scheme URLs are not paths — let the OS opener route them.
    if URI_SCHEMES.iter().any(|s| text.starts_with(s)) {
        return None;
    }

    // Expand a recognized path prefix. Anything falling through is treated as
    // a bare relative path (e.g. `src/main.rs`).
    let expanded: PathBuf = if let Some(rest) = text.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else if text == "~" {
        dirs::home_dir()?
    } else if let Some(rest) = text.strip_prefix('$') {
        let (var_name, tail) = rest.split_once('/').unwrap_or((rest, ""));
        if var_name.is_empty() {
            return None;
        }
        let value = std::env::var(var_name).ok()?;
        let base = PathBuf::from(value);
        if tail.is_empty() {
            base
        } else {
            base.join(tail)
        }
    } else {
        PathBuf::from(text)
    };

    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        cwd?.join(expanded)
    };

    if absolute.exists() {
        Some(absolute)
    } else {
        None
    }
}

/// Apply post-processing to hyperlink URIs (same as in screen/mod.rs)
pub(crate) fn post_process_hyperlink_uri(uri: &str) -> String {
    let chars: Vec<char> = uri.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    let mut end_idx = chars.len() - 1;
    let mut open_parents = 0;
    let mut open_brackets = 0;

    // First pass: handle uneven brackets/parentheses
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => open_parents += 1,
            '[' => open_brackets += 1,
            ')' => {
                if open_parents == 0 {
                    // Unmatched closing parenthesis, truncate here
                    end_idx = i.saturating_sub(1);
                    break;
                } else {
                    open_parents -= 1;
                }
            }
            ']' => {
                if open_brackets == 0 {
                    // Unmatched closing bracket, truncate here
                    end_idx = i.saturating_sub(1);
                    break;
                } else {
                    open_brackets -= 1;
                }
            }
            _ => (),
        }
    }

    // Second pass: remove trailing delimiters
    while end_idx > 0 {
        match chars[end_idx] {
            '.' | ',' | ':' | ';' | '?' | '!' | '(' | '[' | '\'' => {
                end_idx = end_idx.saturating_sub(1);
            }
            _ => break,
        }
    }

    chars.into_iter().take(end_idx + 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint_match(text: &str, start_col: usize, end_col: usize) -> HintMatch {
        HintMatch {
            text: text.to_string(),
            start: Pos::new(Line(0), Column(start_col)),
            end: Pos::new(Line(0), Column(end_col)),
            hint: Rc::new(Hint {
                regex: None,
                hyperlinks: true,
                post_processing: false,
                persist: false,
                action: HintAction::Action {
                    action: HintInternalAction::Open,
                },
                mouse: Default::default(),
                binding: None,
            }),
        }
    }

    #[test]
    fn click_latch_requires_the_exact_visible_hint() {
        let latched = hint_match("https://automexia.dev", 4, 25);
        assert!(latched.same_visible_match(&hint_match("https://automexia.dev", 4, 25)));
        assert!(!latched.same_visible_match(&hint_match("https://example.com", 4, 25)));
        assert!(!latched.same_visible_match(&hint_match("https://automexia.dev", 5, 25)));
        assert!(!latched.same_visible_match(&hint_match("https://automexia.dev", 4, 26)));
    }
    use rio_backend::config::hints::{HintAction, HintInternalAction};

    #[test]
    fn test_hint_state_lifecycle() {
        let mut state = HintState::new("abc".to_string());
        assert!(!state.is_active());

        let hint = Rc::new(Hint {
            regex: Some("test".to_string()),
            hyperlinks: false,
            post_processing: true,
            persist: false,
            action: HintAction::Action {
                action: HintInternalAction::Copy,
            },
            mouse: Default::default(),
            binding: None,
        });

        state.start(hint);
        assert!(state.is_active());

        state.stop();
        assert!(!state.is_active());
    }

    #[test]
    fn test_visible_labels() {
        let mut state = HintState::new("abc".to_string());
        state.labels = vec![vec!['a'], vec!['b'], vec!['a', 'b'], vec!['a', 'c']];

        // No input - all labels visible
        let visible = state.visible_labels();
        assert_eq!(visible.len(), 4);

        // Input "a" - should show labels that start with "a"
        state.keys = vec!['a'];
        let visible = state.visible_labels();
        assert_eq!(visible.len(), 3); // "a", "ab", "ac"
        assert_eq!(visible[0].1, Vec::<char>::new()); // "a" with "a" removed = []
        assert_eq!(visible[1].1, vec!['b']); // "ab" with "a" removed = ['b']
        assert_eq!(visible[2].1, vec!['c']); // "ac" with "a" removed = ['c']
    }

    #[test]
    fn test_keyboard_input_logic() {
        let mut state = HintState::new("jfkdls".to_string());

        // Simulate having some labels
        state.labels = vec![
            vec!['j'], // index 0
            vec!['f'], // index 1
            vec!['k'], // index 2
            vec!['d'], // index 3
            vec!['l'], // index 4
            vec!['s'], // index 5
        ];

        // Simulate having matches (we'll use dummy matches)
        state.matches = vec![
            HintMatch {
                text: "match0".to_string(),
                start: rio_backend::crosswords::pos::Pos::new(
                    rio_backend::crosswords::pos::Line(0),
                    rio_backend::crosswords::pos::Column(0),
                ),
                end: rio_backend::crosswords::pos::Pos::new(
                    rio_backend::crosswords::pos::Line(0),
                    rio_backend::crosswords::pos::Column(5),
                ),
                hint: Rc::new(Hint {
                    regex: Some("test".to_string()),
                    hyperlinks: false,
                    post_processing: true,
                    persist: false,
                    action: HintAction::Action {
                        action: HintInternalAction::Copy,
                    },
                    mouse: Default::default(),
                    binding: None,
                }),
            },
            HintMatch {
                text: "match1".to_string(),
                start: rio_backend::crosswords::pos::Pos::new(
                    rio_backend::crosswords::pos::Line(0),
                    rio_backend::crosswords::pos::Column(10),
                ),
                end: rio_backend::crosswords::pos::Pos::new(
                    rio_backend::crosswords::pos::Line(0),
                    rio_backend::crosswords::pos::Column(15),
                ),
                hint: Rc::new(Hint {
                    regex: Some("test".to_string()),
                    hyperlinks: false,
                    post_processing: true,
                    persist: false,
                    action: HintAction::Action {
                        action: HintInternalAction::Copy,
                    },
                    mouse: Default::default(),
                    binding: None,
                }),
            },
        ];

        let hint = Rc::new(Hint {
            regex: Some("test".to_string()),
            hyperlinks: false,
            post_processing: true,
            persist: false,
            action: HintAction::Action {
                action: HintInternalAction::Copy,
            },
            mouse: Default::default(),
            binding: None,
        });

        state.active_hint = Some(hint);

        // Test keyboard input logic without needing a terminal
        // Test that 'j' should match the first label
        let mut test_keys = state.keys.clone();
        test_keys.push('j');

        let mut matching_indices = Vec::new();
        for (i, label) in state.labels.iter().enumerate() {
            if label.len() >= test_keys.len() && label[..test_keys.len()] == test_keys[..]
            {
                matching_indices.push(i);
            }
        }

        assert!(
            !matching_indices.is_empty(),
            "Should find matching labels for 'j'"
        );
        assert_eq!(matching_indices, vec![0], "Should match index 0 for 'j'");

        // Test that the label should be completed (single character)
        let index = *matching_indices.last().unwrap();
        let label = &state.labels[index];
        assert_eq!(
            label.len(),
            test_keys.len(),
            "Label should be completed with single character"
        );
    }

    #[test]
    fn test_resolve_path_skips_scheme_urls() {
        assert!(resolve_path_for_opening("https://example.com", None).is_none());
        assert!(resolve_path_for_opening("mailto:a@b.c", None).is_none());
        assert!(resolve_path_for_opening("file:///tmp", None).is_none());
        assert!(resolve_path_for_opening("ssh://host/path", None).is_none());
    }

    #[test]
    fn test_resolve_path_returns_none_when_nonexistent() {
        let cwd = std::env::temp_dir();
        assert!(resolve_path_for_opening(
            "rio-definitely-does-not-exist-xyz",
            Some(&cwd)
        )
        .is_none());
        assert!(resolve_path_for_opening(
            "./rio-definitely-does-not-exist-xyz",
            Some(&cwd)
        )
        .is_none());
    }

    #[test]
    fn test_resolve_path_absolute_existing_file() {
        let tmp = std::env::temp_dir();
        let file = tmp.join("rio-test-resolve-abs.txt");
        std::fs::write(&file, "hi").unwrap();

        let resolved = resolve_path_for_opening(&file.to_string_lossy(), None).unwrap();
        // PathBuf::exists() follows symlinks; on macOS /tmp is a symlink to
        // /private/tmp, so compare existence rather than exact paths.
        assert!(resolved.exists());

        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn test_resolve_path_relative_joined_with_cwd() {
        let tmp = std::env::temp_dir();
        let subdir = tmp.join("rio-test-resolve-dir");
        std::fs::create_dir_all(&subdir).unwrap();
        let file = subdir.join("child.txt");
        std::fs::write(&file, "hi").unwrap();

        let resolved = resolve_path_for_opening("child.txt", Some(&subdir)).unwrap();
        assert!(resolved.exists());
        assert!(resolved.ends_with("child.txt"));

        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(&subdir);
    }

    #[test]
    fn test_resolve_path_dot_relative_joined_with_cwd() {
        let tmp = std::env::temp_dir();
        let subdir = tmp.join("rio-test-resolve-dot-dir");
        std::fs::create_dir_all(&subdir).unwrap();
        let file = subdir.join("dot-child.txt");
        std::fs::write(&file, "hi").unwrap();

        let resolved =
            resolve_path_for_opening("./dot-child.txt", Some(&subdir)).unwrap();
        assert!(resolved.exists());

        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(&subdir);
    }

    #[test]
    fn test_resolve_path_requires_cwd_for_relative() {
        // With no cwd and a relative path, we can't resolve; return None.
        assert!(resolve_path_for_opening("foo/bar.txt", None).is_none());
    }

    #[test]
    fn test_resolve_path_expands_env_var() {
        let tmp = std::env::temp_dir();
        // Safety: setting an env var inside a process-local test. This is
        // unsafe in Rust 2024; rio-backend uses an earlier edition so it's
        // permitted here. If rio moves to 2024 this test needs adjustment.
        unsafe {
            std::env::set_var("RIO_TEST_PATH_VAR", tmp.to_string_lossy().to_string());
        }

        let file = tmp.join("rio-test-env-var.txt");
        std::fs::write(&file, "hi").unwrap();

        let resolved =
            resolve_path_for_opening("$RIO_TEST_PATH_VAR/rio-test-env-var.txt", None)
                .unwrap();
        assert!(resolved.exists());

        let _ = std::fs::remove_file(&file);
        unsafe {
            std::env::remove_var("RIO_TEST_PATH_VAR");
        }
    }
}
