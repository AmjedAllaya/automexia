# Fix bounded worker retirement and registration

- Run accepted-work registration callbacks outside worker locks while preserving registration before handler execution.
- Keep a retiring generation owned until its native thread and outstanding registration callbacks finish; reject replacement and stale work while cleanup is pending.
- Request shutdown without waiting for queue capacity and provide bounded acknowledgement waits and an explicit cleanup-failed status.
- Reuse one cleanup service per worker owner, limit live cleanup owners across the process, and retain ownership through blocking handler or native thread-local cleanup.
- Dispose recoverable panic payloads inside the worker's native join boundary and keep failed cleanup distinct from successful retirement.
