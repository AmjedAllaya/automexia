Fixed

- Fixed a race that could report an error after successfully requesting terminal
  shutdown. Managed sessions acknowledge their existing cancellation signal
  without sending a redundant message to a closing worker.
