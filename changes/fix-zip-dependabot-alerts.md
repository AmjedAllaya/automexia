Fixed

- Update `zip` to 9.0.5 in the application and fuzz dependency graphs to resolve CVE-2026-115905. Extension bundle verification continues to use bounded, in-memory archive reads with one set of entry names.
- Update Wasmtime to 48.0.9, `url` to 2.5.9 and `rustix` to 1.1.6 for the advisories reported by CI. Replace the yanked `async-executor` 1.14.0 with 1.14.1.
- Restore the Windows session recovery fixture at the filename used by its existing test callers and assurance records.
