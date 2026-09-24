# Preserve channel notifications during first registration

- Publish the receiver readiness handle before inspecting pending notifications, using paired atomic fences so a racing sender or last-sender disconnect cannot leave the poller asleep.
- Keep the existing queue, native poller, ownership, public API and admission behavior.
- Cover the first-registration gap with deterministic native poller tests and an independent memory-order model that rejects the original ordering and missing fences.
