// clipboard.rs was retired originally from https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty/src/clipboard.rs
// which is licensed under Apache 2.0 license.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardType {
    Clipboard,
    Selection,
}

// The `Clipboard` helper below backs onto the system clipboard via copypasta,
// which pulls in windowing/AppKit dependencies. It is only used by frontends;
// headless embedders (parse + snapshot, OSC 52 as events) need just the
// `ClipboardType` enum above, so the rest is gated behind the `clipboard`
// feature.
#[cfg(feature = "clipboard")]
use raw_window_handle::RawDisplayHandle;
#[cfg(feature = "clipboard")]
use tracing::warn;

#[cfg(all(feature = "wayland", not(any(target_os = "macos", windows))))]
use copypasta::wayland_clipboard;
#[cfg(all(feature = "x11", not(any(target_os = "macos", windows))))]
use copypasta::x11_clipboard::{Primary as X11SelectionClipboard, X11ClipboardContext};
#[cfg(all(
    feature = "clipboard",
    any(feature = "x11", target_os = "macos", windows)
))]
use copypasta::ClipboardContext;
#[cfg(feature = "clipboard")]
use copypasta::ClipboardProvider;

#[cfg(feature = "clipboard")]
pub struct Clipboard {
    clipboard: Option<Box<dyn ClipboardProvider>>,
    selection: Option<Box<dyn ClipboardProvider>>,
}

/// A clipboard operation failed without exposing provider diagnostics or contents.
#[cfg(feature = "clipboard")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardError {
    Unavailable,
    Failed,
}

#[cfg(feature = "clipboard")]
impl std::fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "Clipboard is unavailable",
            Self::Failed => "Clipboard operation failed",
        })
    }
}

#[cfg(feature = "clipboard")]
impl std::error::Error for ClipboardError {}

#[cfg(feature = "clipboard")]
impl Clipboard {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(display: RawDisplayHandle) -> Self {
        match display {
            #[cfg(all(feature = "wayland", not(any(target_os = "macos", windows))))]
            RawDisplayHandle::Wayland(display) => {
                let (selection, clipboard) =
                    wayland_clipboard::create_clipboards_from_external(
                        display.display.as_ptr(),
                    );
                Self {
                    clipboard: Some(Box::new(clipboard)),
                    selection: Some(Box::new(selection)),
                }
            }
            _ => Self::default(),
        }
    }

    /// Used for tests and to handle missing clipboard provider when built without the `x11`
    /// feature.
    pub fn new_nop() -> Self {
        Self {
            clipboard: None,
            selection: None,
        }
    }
}

#[cfg(feature = "clipboard")]
impl Default for Clipboard {
    fn default() -> Self {
        #[cfg(any(target_os = "macos", windows))]
        return Self {
            clipboard: Some(Box::new(ClipboardContext::new().unwrap())),
            selection: None,
        };

        #[cfg(all(feature = "x11", not(any(target_os = "macos", windows))))]
        return Self {
            clipboard: Some(Box::new(ClipboardContext::new().unwrap())),
            selection: Some(Box::new(
                X11ClipboardContext::<X11SelectionClipboard>::new().unwrap(),
            )),
        };

        #[cfg(not(any(feature = "x11", target_os = "macos", windows)))]
        return Self::new_nop();
    }
}

#[cfg(feature = "clipboard")]
impl Clipboard {
    pub fn set(&mut self, ty: ClipboardType, text: impl Into<String>) {
        if self.try_set(ty, text) == Err(ClipboardError::Failed) {
            warn!("Unable to store text in clipboard");
        }
    }

    /// Callers that remove source text after copying must wait for success.
    pub fn try_set(
        &mut self,
        ty: ClipboardType,
        text: impl Into<String>,
    ) -> Result<(), ClipboardError> {
        let clipboard = match ty {
            ClipboardType::Selection => self.selection.as_mut(),
            ClipboardType::Clipboard => self.clipboard.as_mut(),
        }
        .ok_or(ClipboardError::Unavailable)?;
        clipboard
            .set_contents(text.into())
            .map_err(|_| ClipboardError::Failed)
    }

    pub fn get(&mut self, ty: ClipboardType) -> String {
        // Preserve the legacy primary-selection fallback for existing callers.
        let target = match ty {
            ClipboardType::Selection if self.selection.is_none() => {
                ClipboardType::Clipboard
            }
            target => target,
        };
        match self.try_get(target) {
            Err(ClipboardError::Failed) => {
                warn!("Unable to load text from clipboard");
                String::new()
            }
            Err(ClipboardError::Unavailable) => String::new(),
            Ok(text) => text,
        }
    }

    /// Read the requested clipboard without conflating failure with empty text.
    pub fn try_get(&mut self, ty: ClipboardType) -> Result<String, ClipboardError> {
        let clipboard = match ty {
            ClipboardType::Selection => self.selection.as_mut(),
            ClipboardType::Clipboard => self.clipboard.as_mut(),
        }
        .ok_or(ClipboardError::Unavailable)?;
        clipboard.get_contents().map_err(|_| ClipboardError::Failed)
    }
}

#[cfg(all(test, feature = "clipboard"))]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestProvider {
        stored: Arc<Mutex<String>>,
        fail_write: bool,
        fail_read: bool,
    }

    impl ClipboardProvider for TestProvider {
        fn get_contents(
            &mut self,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_read {
                return Err(std::io::Error::other("private provider diagnostic").into());
            }
            Ok(self.stored.lock().unwrap().clone())
        }

        fn set_contents(
            &mut self,
            text: String,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_write {
                return Err(std::io::Error::other("private provider diagnostic").into());
            }
            *self.stored.lock().unwrap() = text;
            Ok(())
        }
    }

    #[test]
    fn fallible_write_preserves_provider_failure_without_exposing_details() {
        let stored = Arc::new(Mutex::new(String::from("previous clipboard")));
        let mut clipboard = Clipboard {
            clipboard: Some(Box::new(TestProvider {
                stored: stored.clone(),
                fail_write: true,
                fail_read: false,
            })),
            selection: None,
        };
        let error = clipboard
            .try_set(ClipboardType::Clipboard, "new draft")
            .unwrap_err();
        assert_eq!(error, ClipboardError::Failed);
        assert_eq!(*stored.lock().unwrap(), "previous clipboard");
        assert!(!error.to_string().contains("private provider diagnostic"));
        clipboard.set(ClipboardType::Clipboard, "legacy caller");
        assert_eq!(*stored.lock().unwrap(), "previous clipboard");
    }

    #[test]
    fn fallible_and_legacy_writes_share_the_existing_provider() {
        let stored = Arc::new(Mutex::new(String::new()));
        let selected = Arc::new(Mutex::new(String::new()));
        let mut clipboard = Clipboard {
            clipboard: Some(Box::new(TestProvider {
                stored: stored.clone(),
                fail_write: false,
                fail_read: false,
            })),
            selection: Some(Box::new(TestProvider {
                stored: selected.clone(),
                fail_write: false,
                fail_read: false,
            })),
        };
        assert_eq!(clipboard.try_set(ClipboardType::Clipboard, "界é"), Ok(()));
        assert_eq!(*stored.lock().unwrap(), "界é");
        assert_eq!(
            clipboard.try_set(ClipboardType::Selection, "selected"),
            Ok(())
        );
        assert_eq!(*selected.lock().unwrap(), "selected");
        clipboard.set(ClipboardType::Clipboard, "legacy");
        assert_eq!(clipboard.get(ClipboardType::Clipboard), "legacy");
    }

    #[test]
    fn absent_clipboards_never_report_a_successful_write_or_redirect_to_another() {
        let mut absent = Clipboard::new_nop();
        for target in [ClipboardType::Clipboard, ClipboardType::Selection] {
            assert_eq!(
                absent.try_set(target, "draft"),
                Err(ClipboardError::Unavailable)
            );
            absent.set(target, "legacy");
            assert_eq!(absent.get(target), "");
            assert_eq!(absent.try_get(target), Err(ClipboardError::Unavailable));
        }
        let stored = Arc::new(Mutex::new(String::from("unchanged")));
        let mut clipboard = Clipboard {
            clipboard: Some(Box::new(TestProvider {
                stored: stored.clone(),
                fail_write: false,
                fail_read: false,
            })),
            selection: None,
        };
        assert_eq!(
            clipboard.try_set(ClipboardType::Selection, "not primary"),
            Err(ClipboardError::Unavailable)
        );
        assert_eq!(*stored.lock().unwrap(), "unchanged");
        assert_eq!(
            clipboard.try_get(ClipboardType::Selection),
            Err(ClipboardError::Unavailable)
        );
        assert_eq!(clipboard.get(ClipboardType::Selection), "unchanged");
    }

    #[test]
    fn fallible_read_distinguishes_failure_empty_content_and_success() {
        let stored = Arc::new(Mutex::new(String::from("unchanged")));
        let mut failing = Clipboard {
            clipboard: Some(Box::new(TestProvider {
                stored: stored.clone(),
                fail_write: false,
                fail_read: true,
            })),
            selection: None,
        };
        let error = failing.try_get(ClipboardType::Clipboard).unwrap_err();
        assert_eq!(error, ClipboardError::Failed);
        assert!(!error.to_string().contains("private provider diagnostic"));
        assert_eq!(failing.get(ClipboardType::Clipboard), "");
        assert_eq!(*stored.lock().unwrap(), "unchanged");
        let mut successful = Clipboard {
            clipboard: Some(Box::new(TestProvider {
                stored: stored.clone(),
                fail_write: false,
                fail_read: false,
            })),
            selection: Some(Box::new(TestProvider {
                stored: Arc::new(Mutex::new(String::from("primary"))),
                fail_write: false,
                fail_read: false,
            })),
        };
        assert_eq!(
            successful.try_get(ClipboardType::Clipboard),
            Ok(String::from("unchanged"))
        );
        assert_eq!(
            successful.try_get(ClipboardType::Selection),
            Ok(String::from("primary"))
        );
        stored.lock().unwrap().clear();
        assert_eq!(
            successful.try_get(ClipboardType::Clipboard),
            Ok(String::new())
        );
    }
}
