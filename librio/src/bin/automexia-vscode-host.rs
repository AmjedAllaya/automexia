//! One terminal surface for the Automexia VS Code view.

#[path = "vscode/context.rs"]
mod context;

use context::{information_bar_presets, Context, Preset, SessionContext};
use librio::{
    AnsiColor, ColorRgb, Engine, Key, KeyAction, KeyEvent, Modifiers, NamedColor,
    RenderState, StyleFlags, Surface, SurfaceDelegate, SurfaceDesc,
};
use rio_vt::config::colors::{term::List, Colors};
use rio_vt::crosswords::square::Wide;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use std::time::Instant;

const PROTOCOL_VERSION: u8 = 1;
const MAX_COLUMNS: u16 = 240;
const MAX_ROWS: u16 = 120;
const MAX_REQUEST_BYTES: usize = 32 * 1024;
const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_SHELL_ARGS: usize = 32;
const MAX_ENVIRONMENT_ENTRIES: usize = 512;
const MAX_ENVIRONMENT_BYTES: usize = 256 * 1024;
const MAX_COMBINING_MARKS_PER_CELL: usize = 6;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Poll {},
    Resize {
        cols: u16,
        rows: u16,
    },
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
    Key {
        key: String,
        #[serde(default)]
        ctrl: bool,
        #[serde(default)]
        alt: bool,
        #[serde(default)]
        shift: bool,
        #[serde(default)]
        meta: bool,
    },
    Scroll {
        lines: i32,
    },
    Close {},
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Ready {
        version: u8,
        context: bool,
        presets: Vec<Preset>,
    },
    Idle {
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<Context>,
    },
    Frame(Frame),
    Exit {
        status: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        frame: Option<Frame>,
    },
}

#[derive(Serialize)]
struct Frame {
    cols: usize,
    rows: Vec<Vec<Run>>,
    cursor: Cursor,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Context>,
}

#[derive(Serialize)]
struct Cursor {
    row: usize,
    col: usize,
    visible: bool,
}

type Run = (String, Option<String>, Option<String>, u8, usize);

#[derive(Default)]
struct ViewState {
    dirty: AtomicBool,
    exited: AtomicBool,
    child_status: OnceLock<Option<i32>>,
}

impl ViewState {
    fn exit_code(&self) -> Option<i32> {
        let raw_status = self.child_status.get().copied().flatten()?;
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let status = std::process::ExitStatus::from_raw(raw_status);
            status
                .code()
                .or_else(|| status.signal().map(|signal| 128 + signal))
        }
        #[cfg(not(unix))]
        {
            Some(raw_status)
        }
    }
}

impl SurfaceDelegate for ViewState {
    fn wakeup(&self, _surface: usize) {
        self.dirty.store(true, Ordering::Release);
    }

    fn child_exited(&self, _surface: usize, raw_status: Option<i32>) {
        let _ = self.child_status.set(raw_status);
    }

    fn close_surface(&self, _surface: usize) {
        self.exited.store(true, Ordering::Release);
    }
}

fn input_error(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn launch_description(args: &[String]) -> io::Result<SurfaceDesc> {
    let [shell_flag, shell, cwd_flag, cwd, delimiter, argv @ ..] = args else {
        return Err(input_error("expected shell and working directory options"));
    };
    if shell_flag != "--shell" || cwd_flag != "--cwd" || delimiter != "--" {
        return Err(input_error("unexpected launch options"));
    }
    let shell_path = Path::new(shell);
    let cwd_path = Path::new(cwd);
    if shell.len() > MAX_TEXT_BYTES || !shell_path.is_absolute() || !shell_path.is_file()
    {
        return Err(input_error(
            "expected an absolute path to a shell executable",
        ));
    }
    if cwd.len() > MAX_TEXT_BYTES || !cwd_path.is_absolute() || !cwd_path.is_dir() {
        return Err(input_error(
            "expected an absolute path to a working directory",
        ));
    }
    if argv.len() > MAX_SHELL_ARGS || argv.iter().any(|arg| arg.len() > MAX_TEXT_BYTES) {
        return Err(input_error("shell argument limit exceeded"));
    }
    Ok(SurfaceDesc {
        shell: Some(shell.clone()),
        args: argv.to_vec(),
        working_dir: Some(cwd.clone()),
        ..SurfaceDesc::default()
    })
}

fn shell_environment() -> io::Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    let mut total = 0;
    for (name, value) in std::env::vars_os() {
        let (Ok(name), Ok(value)) = (name.into_string(), value.into_string()) else {
            continue;
        };
        // The explicit launch API rejects names such as Windows' drive entries.
        if name.is_empty() || name.contains('=') {
            continue;
        }
        total += name.len() + value.len();
        if entries.len() == MAX_ENVIRONMENT_ENTRIES || total > MAX_ENVIRONMENT_BYTES {
            return Err(input_error("shell environment limit exceeded"));
        }
        entries.push((name, value));
    }
    Ok(entries)
}

fn read_request(
    reader: &mut impl BufRead,
    buffer: &mut Vec<u8>,
) -> io::Result<Option<Request>> {
    buffer.clear();
    let count = reader
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_until(b'\n', buffer)?;
    if count == 0 {
        return Ok(None);
    }
    if count > MAX_REQUEST_BYTES || !buffer.ends_with(b"\n") {
        return Err(input_error("invalid or oversized request framing"));
    }
    serde_json::from_slice(buffer)
        .map(Some)
        .map_err(|_| input_error("invalid protocol request"))
}

fn write_response(writer: &mut impl Write, mut response: Response) -> io::Result<()> {
    let mut message = serde_json::to_vec(&response).map_err(io::Error::other)?;
    // Optional context and final output must not prevent a core response.
    if message.len() >= MAX_FRAME_BYTES {
        let reduced = match &mut response {
            Response::Frame(frame) => frame.context.take().is_some(),
            Response::Exit { frame, .. } => frame.take().is_some(),
            _ => false,
        };
        if reduced {
            message = serde_json::to_vec(&response).map_err(io::Error::other)?;
        }
    }
    if message.len() >= MAX_FRAME_BYTES {
        return Err(input_error("frame size limit exceeded"));
    }
    writer.write_all(&message)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn allowed_text(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_TEXT_BYTES
        && !text.chars().any(char::is_control)
}

fn terminal_key(value: &str) -> Option<Key> {
    let key = match value {
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Escape" => Key::Escape,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        _ => {
            if let Ok(character) = value.parse::<char>() {
                return (!character.is_control())
                    .then_some(Key::Char(character.to_ascii_lowercase()));
            }
            let function = value.strip_prefix('F')?.parse::<u8>().ok()?;
            if !(1..=12).contains(&function) {
                return None;
            }
            Key::F(function)
        }
    };
    Some(key)
}

fn apply_input(surface: &Surface, request: Request) -> bool {
    match request {
        Request::Resize { cols, rows }
            if (1..=MAX_COLUMNS).contains(&cols) && (1..=MAX_ROWS).contains(&rows) =>
        {
            surface.resize(cols, rows, cols * 9, rows * 18);
        }
        Request::Text { text } if allowed_text(&text) => surface.text(&text),
        Request::Paste { text } if allowed_text(&text) => surface.paste(&text),
        Request::Key {
            key,
            ctrl,
            alt,
            shift,
            meta,
        } => {
            let Some(terminal_key) = terminal_key(&key) else {
                return false;
            };
            let mut mods = Modifiers::empty();
            mods.set(Modifiers::CTRL, ctrl);
            mods.set(Modifiers::ALT, alt);
            mods.set(Modifiers::SHIFT, shift);
            mods.set(Modifiers::SUPER, meta);
            let text = matches!(terminal_key, Key::Char(_)).then_some(key);
            return surface.key(&KeyEvent {
                action: KeyAction::Press,
                key: Some(terminal_key),
                mods,
                text,
                ..KeyEvent::default()
            });
        }
        Request::Scroll { lines } if (-120..=120).contains(&lines) => {
            surface.scroll(lines)
        }
        _ => return false,
    }
    true
}

fn color(color: AnsiColor, default: NamedColor, palette: &List) -> Option<String> {
    let rgb = match color {
        AnsiColor::Named(name) if name == default => return None,
        AnsiColor::Named(name) => ColorRgb::from_color_arr(palette[name]),
        AnsiColor::Indexed(index) => {
            ColorRgb::from_color_arr(palette[usize::from(index)])
        }
        AnsiColor::Spec(rgb) => rgb,
    };
    Some(format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b))
}

fn frame(state: &mut RenderState, palette: &List, context: Option<Context>) -> Frame {
    state.update();
    let cols = state.columns();
    let rows = (0..state.lines())
        .map(|row| {
            let mut runs: Vec<Run> = Vec::new();
            let mut last_style = None;
            for col in 0..cols {
                let Some(square) = state.square(row, col) else {
                    continue;
                };
                if square.wide() == Wide::Spacer {
                    continue;
                }
                let width =
                    if square.wide() == Wide::Wide { 2 } else { 1 }.min(cols - col);
                let style = state.style_of(square);
                let blank = style.flags.contains(StyleFlags::HIDDEN)
                    || square.is_bg_only()
                    || square.c().is_control();
                if last_style != Some(style) {
                    let (fg, bg) = if style.flags.contains(StyleFlags::INVERSE) {
                        (style.bg, style.fg)
                    } else {
                        (style.fg, style.bg)
                    };
                    let flags = u8::from(style.flags.contains(StyleFlags::BOLD))
                        | (u8::from(style.flags.contains(StyleFlags::ITALIC)) << 1)
                        | (u8::from(style.flags.intersects(StyleFlags::ALL_UNDERLINES))
                            << 2);
                    runs.push((
                        String::new(),
                        color(fg, NamedColor::Foreground, palette),
                        color(bg, NamedColor::Background, palette),
                        flags,
                        0,
                    ));
                    last_style = Some(style);
                }
                if let Some((text, _, _, _, cells)) = runs.last_mut() {
                    if blank {
                        text.extend(std::iter::repeat_n(' ', width));
                    } else {
                        text.push(square.c());
                        text.extend(
                            state
                                .combining_chars(square)
                                .iter()
                                .copied()
                                .take(MAX_COMBINING_MARKS_PER_CELL)
                                .filter(|character| !character.is_control()),
                        );
                    }
                    *cells += width;
                }
            }
            runs
        })
        .collect();
    let (row, col) = state.cursor();
    Frame {
        cols,
        rows,
        cursor: Cursor {
            row: row.min(state.lines().saturating_sub(1)),
            col: col.min(cols.saturating_sub(1)),
            visible: state.cursor_visible()
                && state.display_offset() == 0
                && row < state.lines()
                && col < cols,
        },
        context,
    }
}

fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .map_err(|_| input_error("launch options must be Unicode"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let description = launch_description(&args)?;
    let mut session_context = SessionContext::new(&description)?;
    let view = Arc::new(ViewState::default());
    let engine = Engine::new(view.clone());
    let surface = engine.create_owned_surface(&description, shell_environment()?)?;
    let mut state = RenderState::new(&surface);
    let palette = List::from(&Colors::default());
    let mut reader = io::stdin().lock();
    let mut writer = io::stdout().lock();
    let mut request_buffer = Vec::new();
    view.dirty.store(true, Ordering::Release);
    write_response(
        &mut writer,
        Response::Ready {
            version: PROTOCOL_VERSION,
            context: true,
            presets: information_bar_presets(),
        },
    )?;
    let (status, final_frame) = loop {
        let Some(request) = read_request(&mut reader, &mut request_buffer)? else {
            break (None, None);
        };
        if matches!(request, Request::Close {}) {
            break (None, None);
        }
        if view.exited.load(Ordering::Acquire) {
            // The worker drains final output before announcing the child exit.
            break (view.exit_code(), Some(frame(&mut state, &palette, None)));
        }
        if matches!(request, Request::Poll {}) {
            let context = session_context
                .poll_due(Instant::now())
                .then(|| session_context.snapshot(surface.docked_metadata()));
            let response = if view.dirty.swap(false, Ordering::AcqRel) {
                Response::Frame(frame(&mut state, &palette, context))
            } else {
                Response::Idle { context }
            };
            write_response(&mut writer, response)?;
        } else if apply_input(&surface, request) {
            view.dirty.store(true, Ordering::Release);
        }
    };
    // Report the status before native cleanup can outlast the editor's poll.
    write_response(
        &mut writer,
        Response::Exit {
            status,
            frame: final_frame,
        },
    )?;
    drop(state);
    drop(surface);
    Ok(())
}

fn main() -> std::process::ExitCode {
    if run().is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!("Automexia could not start or continue the terminal session.");
        std::process::ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_adds_context_without_changing_the_version_or_empty_idle() {
        let ready = serde_json::to_value(Response::Ready {
            version: PROTOCOL_VERSION,
            context: true,
            presets: information_bar_presets(),
        })
        .unwrap();
        assert_eq!(ready["type"], "ready");
        assert_eq!(ready["version"], 1);
        assert_eq!(ready["context"], true);
        assert_eq!(ready["presets"].as_array().unwrap().len(), 12);
        assert_eq!(
            serde_json::to_value(Response::Idle { context: None }).unwrap(),
            serde_json::json!({"type": "idle"})
        );
    }

    #[test]
    fn a_bounded_grid_survives_when_optional_context_would_overflow() {
        let session = SessionContext::new(&SurfaceDesc {
            working_dir: Some("x".repeat(MAX_TEXT_BYTES)),
            ..SurfaceDesc::default()
        })
        .unwrap();
        let mut wire = Vec::new();
        write_response(
            &mut wire,
            Response::Frame(Frame {
                cols: 2,
                rows: vec![vec![("x".repeat(MAX_FRAME_BYTES - 1024), None, None, 0, 2)]],
                cursor: Cursor {
                    row: 0,
                    col: 0,
                    visible: true,
                },
                context: Some(session.snapshot(librio::DockedMetadata::default())),
            }),
        )
        .unwrap();
        assert!(wire.len() < MAX_FRAME_BYTES);
        let message: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(message["type"], "frame");
        assert!(message.get("context").is_none());
    }

    #[test]
    fn child_exit_keeps_success_failure_and_unknown_distinct() {
        let raw_failure = if cfg!(unix) { 37 << 8 } else { 37 };
        for (raw_status, expected_status) in [
            (None, None),
            (Some(0), Some(0)),
            (Some(raw_failure), Some(37)),
        ] {
            let view = ViewState::default();
            view.child_exited(7, raw_status);
            view.close_surface(7);
            assert!(view.exited.load(Ordering::Acquire));
            assert_eq!(view.exit_code(), expected_status);
        }
    }

    #[cfg(unix)]
    #[test]
    fn signal_exit_uses_shell_status_convention() {
        let view = ViewState::default();
        view.child_exited(7, Some(15));
        assert_eq!(view.exit_code(), Some(143));
    }

    #[test]
    fn child_exit_can_carry_final_output_within_the_wire_limit() {
        for (text, frame_present) in [
            (String::from("end"), true),
            ("x".repeat(MAX_FRAME_BYTES), false),
        ] {
            let mut wire = Vec::new();
            write_response(
                &mut wire,
                Response::Exit {
                    status: Some(37),
                    frame: Some(Frame {
                        cols: 3,
                        rows: vec![vec![(text, None, None, 0, 3)]],
                        cursor: Cursor {
                            row: 0,
                            col: 0,
                            visible: false,
                        },
                        context: None,
                    }),
                },
            )
            .unwrap();
            let message: serde_json::Value = serde_json::from_slice(&wire).unwrap();
            assert_eq!(message["type"], "exit");
            assert_eq!(message["status"], 37);
            if frame_present {
                assert_eq!(message["frame"]["rows"][0][0][0], "end");
                assert!(message["frame"].get("type").is_none());
            } else {
                assert_eq!(message, serde_json::json!({"type": "exit", "status": 37}));
            }
        }

        assert_eq!(
            serde_json::to_value(Response::Exit {
                status: None,
                frame: None
            })
            .unwrap(),
            serde_json::json!({"type": "exit", "status": null}),
        );
    }

    #[test]
    fn protocol_accepts_a_poll_and_preserves_following_input() {
        let mut input: &[u8] =
            b"{\"type\":\"poll\"}\n{\"type\":\"text\",\"text\":\"world\"}\n";
        let mut buffer = Vec::new();
        assert!(matches!(
            read_request(&mut input, &mut buffer),
            Ok(Some(Request::Poll {}))
        ));
        assert!(
            matches!(read_request(&mut input, &mut buffer), Ok(Some(Request::Text { text })) if text == "world")
        );
        assert!(matches!(read_request(&mut input, &mut buffer), Ok(None)));
    }

    #[test]
    fn protocol_limits_or_rejects_invalid_frames() {
        for value in [
            b"{\"type\":\"unknown\"}\n".as_slice(),
            b"{\"type\":\"poll\"}",
            b"{\"type\":\"poll\",\"shell\":\"other\"}\n",
        ] {
            let mut input = value;
            assert!(read_request(&mut input, &mut Vec::new()).is_err());
        }
        let oversized = vec![b' '; MAX_REQUEST_BYTES + 4];
        let mut input = oversized.as_slice();
        let mut buffer = Vec::new();
        assert!(read_request(&mut input, &mut buffer).is_err());
        assert_eq!(buffer.len(), MAX_REQUEST_BYTES + 1);
    }

    #[test]
    fn input_text_rejects_terminal_controls_and_implicit_newlines() {
        assert!(allowed_text("echo Français"));
        assert!(allowed_text(&"a".repeat(MAX_TEXT_BYTES)));
        for value in [
            "",
            "first\nsecond",
            "first\rsecond",
            "text\x1b",
            "x\u{0085}",
        ] {
            assert!(!allowed_text(value));
        }
        assert!(!allowed_text(&"a".repeat(MAX_TEXT_BYTES + 1)));
    }

    #[test]
    fn keys_use_the_terminal_encoder_vocabulary() {
        assert_eq!(terminal_key("ArrowUp"), Some(Key::Up));
        assert_eq!(terminal_key("F12"), Some(Key::F(12)));
        assert_eq!(terminal_key("A"), Some(Key::Char('a')));
        assert_eq!(terminal_key("F13"), None);
        assert_eq!(terminal_key("\x1b"), None);
    }
}
