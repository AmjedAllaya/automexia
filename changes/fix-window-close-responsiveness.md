# Dismiss closing windows before cleanup waits

Confirmed close and Quit now dismiss their native windows before route/service
cleanup. Quit no longer waits on a shared service before reaching the application
shutdown owner. Native process cleanup and its safety budgets remain intact.

Windows PTY shutdown drains output even after a full bounded buffer loses its
consumer. Live reads preserve final buffered output before EOF. Native pipe and
ConPTY regressions, close-order mutation tests and a separate desktop-dismissal
ceiling cover these paths. Unexecuted desktop/platform evidence remains external.
