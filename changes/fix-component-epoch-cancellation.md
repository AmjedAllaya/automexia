Fixed a cancellation-ordering race in the existing disabled component host.
Invocation-local deadlines survive epoch ticks before worker startup, shared
engine ticks do not cancel unrelated calls, and watchdog cleanup covers worker
failure. Added real-runtime isolation and lifecycle regressions. This source
correction does not enable component execution or change release authorization.
Full local QA explicitly runs bounded all-feature host tests without overwriting
the workspace JUnit report; a host failure is a required QA failure.
