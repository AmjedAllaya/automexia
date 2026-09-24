# Register the Windows event-loop wake before creating its target

Initialize the existing user-event message identifier before creating the Windows
event target. A registration failure now returns the established startup error
without allocating a target window, and later worker completion wakes use the
cached identifier. Other internal message identities retain their existing policy.
