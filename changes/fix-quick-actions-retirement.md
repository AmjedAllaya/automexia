# Fix Quick Actions worker retirement

- Retire the Quick Actions search worker through the existing bounded worker owner so final runtime destruction does not join blocked work on the calling thread.
- Keep queued callback destruction and native thread-local cleanup within the owned retirement boundary, with bounded explicit acknowledgement and no restart after shutdown.
- Cancel result and workspace authorization publication atomically with route removal, provider replacement and shutdown, while preserving sibling routes and publishing before completion wakes.
