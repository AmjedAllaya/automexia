# Restrict and own the optional ConPTY library

- Load the optional ConPTY library only beside the application, with dependency searches limited to that directory and the Windows system directory.
- Keep each acquired module reference alive through pseudoconsole close, then release it; failed export resolution and failed creation also release their references.
- Preserve the Windows inbox fallback and existing launch validation. Native isolated fixtures cover discovery, dependencies, partial exports, repeated lifetimes, and invalid images.
