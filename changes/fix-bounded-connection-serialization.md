# Bounded connection serialization

Connection Library and managed receipts share an application-private writer that caps
capacity growth and reports reservation failures without exposing content.
Document limits, pretty-JSON bytes, validation order and recovery are preserved.
Tests reproduce the prior large-string/final-quote over-reservation and cover
write boundaries, UTF-8, rejected-write integrity and compatibility.

SSH metadata retains its small extension-local writer and now applies the same
fallible, capped reservation discipline without an application dependency.
