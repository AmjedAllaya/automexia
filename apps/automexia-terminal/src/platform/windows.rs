use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerSetRequest,
};
use windows_sys::Win32::System::SystemServices::POWER_REQUEST_CONTEXT_VERSION;
use windows_sys::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};

const FULLSCREEN_REASON: &str = "Automexia Terminal is displaying a fullscreen session.";
static ACTIVE_FULLSCREEN_DISPLAY_REQUESTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "native-gui-test-hooks")]
#[inline]
pub(crate) fn active_fullscreen_display_requests() -> usize {
    ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst)
}

fn record_display_request_release() {
    let result = ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.fetch_update(
        Ordering::SeqCst,
        Ordering::SeqCst,
        |active| active.checked_sub(1),
    );
    debug_assert!(result.is_ok(), "fullscreen display request count underflow");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestTransition {
    Activate,
    Release,
}

#[inline]
fn request_transition(active: bool, required: bool) -> Option<RequestTransition> {
    match (active, required) {
        (false, true) => Some(RequestTransition::Activate),
        (true, false) => Some(RequestTransition::Release),
        _ => None,
    }
}

/// Owns one Windows display-power request for one Automexia window.
///
/// Fullscreen terminals are often used as passive dashboards, where PTY output
/// does not count as local user input. Keeping this request route-scoped makes
/// Windows skip idle display dimming only for the exact lifetime of a
/// fullscreen window. It does not prevent system sleep and it does not modify
/// the user's power plan.
#[derive(Debug, Default)]
pub struct FullscreenDisplayRequest {
    handle: Option<HANDLE>,
    active: bool,
}

impl FullscreenDisplayRequest {
    pub fn new(required: bool) -> Self {
        let mut request = Self::default();
        request.set_required(required);
        request
    }

    pub fn set_required(&mut self, required: bool) {
        let Some(transition) = request_transition(self.active, required) else {
            return;
        };

        match transition {
            RequestTransition::Activate => {
                let Some(handle) = self.handle.or_else(create_request) else {
                    return;
                };
                self.handle = Some(handle);
                if unsafe { PowerSetRequest(handle, PowerRequestDisplayRequired) } == 0 {
                    tracing::warn!(
                        error = %io::Error::last_os_error(),
                        "could not prevent Windows from dimming a fullscreen Automexia display"
                    );
                    return;
                }
                self.active = true;
                ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.fetch_add(1, Ordering::SeqCst);
            }
            RequestTransition::Release => {
                let Some(handle) = self.handle else {
                    self.active = false;
                    return;
                };
                if unsafe { PowerClearRequest(handle, PowerRequestDisplayRequired) } == 0
                {
                    tracing::warn!(
                        error = %io::Error::last_os_error(),
                        "could not release the fullscreen Automexia display request"
                    );
                    return;
                }
                self.active = false;
                record_display_request_release();
            }
        }
    }
}

impl Drop for FullscreenDisplayRequest {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if self.active {
                if unsafe { PowerClearRequest(handle, PowerRequestDisplayRequired) } == 0
                {
                    tracing::warn!(
                        error = %io::Error::last_os_error(),
                        "could not clear the fullscreen Automexia display request during teardown"
                    );
                }
                record_display_request_release();
            }
            if unsafe { CloseHandle(handle) } == 0 {
                tracing::warn!(
                    error = %io::Error::last_os_error(),
                    "could not close the fullscreen Automexia display request"
                );
            }
        }
        self.active = false;
    }
}

fn create_request() -> Option<HANDLE> {
    let mut reason = FULLSCREEN_REASON
        .encode_utf16()
        .chain([0])
        .collect::<Vec<_>>();
    let context = REASON_CONTEXT {
        Version: POWER_REQUEST_CONTEXT_VERSION,
        Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
        Reason: REASON_CONTEXT_0 {
            SimpleReasonString: reason.as_mut_ptr(),
        },
    };
    let handle = unsafe { PowerCreateRequest(&context) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        tracing::warn!(
            error = %io::Error::last_os_error(),
            "could not create a fullscreen Automexia display request"
        );
        None
    } else {
        Some(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        request_transition, FullscreenDisplayRequest, RequestTransition,
        ACTIVE_FULLSCREEN_DISPLAY_REQUESTS,
    };
    use std::sync::atomic::Ordering;

    #[test]
    fn inactive_request_activates_only_for_fullscreen() {
        assert_eq!(request_transition(false, false), None);
        assert_eq!(
            request_transition(false, true),
            Some(RequestTransition::Activate)
        );
    }

    #[test]
    fn active_request_releases_only_after_fullscreen() {
        assert_eq!(request_transition(true, true), None);
        assert_eq!(
            request_transition(true, false),
            Some(RequestTransition::Release)
        );
    }

    #[test]
    fn independent_windows_release_only_their_own_display_request() {
        let baseline = ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst);
        let mut first = FullscreenDisplayRequest::new(true);
        assert!(
            first.active,
            "Windows rejected the first DisplayRequired request"
        );
        assert_eq!(
            ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst),
            baseline + 1
        );

        let second = FullscreenDisplayRequest::new(true);
        assert!(
            second.active,
            "Windows rejected the second DisplayRequired request"
        );
        assert_eq!(
            ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst),
            baseline + 2
        );

        first.set_required(false);
        assert!(!first.active);
        assert_eq!(
            ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst),
            baseline + 1
        );

        drop(second);
        assert_eq!(
            ACTIVE_FULLSCREEN_DISPLAY_REQUESTS.load(Ordering::SeqCst),
            baseline
        );
    }
}
