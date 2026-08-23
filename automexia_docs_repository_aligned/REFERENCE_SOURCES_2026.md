
# Reference Sources — 2026 Research Snapshot

**Access date for this pack:** 2026-08-23

Source categories:

- **Pinned evidence:** exact release/tag/advisory used for a specific decision.
- **Mutable upstream:** "latest", download, support-policy, or main-branch URLs; useful for research but not immutable release evidence.
- **License/model evidence:** source-code license and model-weight redistribution must be recorded separately before shipping any model.

Do not treat a mutable "latest" URL as the release receipt for a shipped artifact.

---

## Reference Sources for the 2026 Re-Audit

These are release-time verification references, not immutable architecture dependencies.

- Rust release announcements: https://blog.rust-lang.org/releases/latest/
- Rust August 2026 supply-chain incident: https://blog.rust-lang.org/2026/08/20/supply-chain-attack-on-arrayref/
- Tokio README/LTS policy: https://github.com/tokio-rs/tokio/blob/master/README.md
- Wasmtime LTS RFC: https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-lts.md
- Wasmtime security advisories: https://github.com/bytecodealliance/wasmtime/security/advisories
- WASI 0.3 announcement: https://bytecodealliance.org/articles/WASI-0.3
- Component Model Rust toolchain guidance: https://component-model.bytecodealliance.org/language-support/creating-runnable-components/rust.html
- AccessKit releases: https://github.com/AccessKit/accesskit/releases
- SQLite WAL notes: https://www.sqlite.org/wal.html
- SQLite releases: https://sqlite.org/changes.html
- OpenSSH release/security notes: https://www.openbsd.org/openssh/releasenotes.html
- OpenSSH manual: https://man.openbsd.org/ssh
- Azure CLI authentication: https://learn.microsoft.com/en-us/cli/azure/authenticate-azure-cli-interactively
- Google Cloud CLI authentication: https://docs.cloud.google.com/sdk/docs/authenticate
- Kubernetes 1.35 credential plugin policy: https://kubernetes.io/blog/2026/01/09/kubernetes-v1-35-kuberc-credential-plugin-allowlist/
- Teleport tsh documentation: https://goteleport.com/docs/connect-your-client/teleport-clients/tsh/
- OpenBao release notes: https://openbao.org/community/release-notes/2-5-0/
- OpenBao token helper: https://openbao.org/docs/commands/token-helper/
- KeePassXC SSH agent documentation: https://keepassxc.org/docs/
- Bitwarden SSH agent documentation: https://bitwarden.com/help/about-ssh/
- 1Password SSH agent/bookmarks: https://developer.1password.com/docs/ssh/bookmarks/
- FFmpeg downloads/releases: https://ffmpeg.org/download.html


### Ghostty compatibility reference

Current compatibility fixture work is pinned to Ghostty 1.3.1, released 2026-03-13.

Official references:
- https://ghostty.org/docs/install/release-notes/1-3-1
- https://ghostty.org/docs/install/release-notes
- https://ghostty.org/download
- https://github.com/ghostty-org/ghostty/tree/v1.3.1
- https://github.com/ghostty-org/ghostty/blob/v1.3.1/src/input/Binding.zig

Architecture note: these sources are used for maintainer/reference work. They do not make Ghostty a required Automexia runtime dependency.


### Future video/media license evidence — exact links

Accessed 2026-08-23. These are research inputs, not current Automexia runtime dependencies.

- Silero VAD license: https://github.com/snakers4/silero-vad/blob/master/LICENSE
- YuNet model-specific license: https://github.com/opencv/opencv_zoo/blob/main/models/face_detection_yunet/LICENSE
- RNNoise repository/license: https://github.com/xiph/rnnoise
- DeepFilterNet MIT license: https://github.com/Rikorose/DeepFilterNet/blob/main/LICENSE-MIT
- DeepFilterNet pretrained-weight license question (2026-07-15): https://github.com/Rikorose/DeepFilterNet/issues/697
- whisper.cpp license: https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE
- whisper.cpp FFmpeg-example licensing issue: https://github.com/ggml-org/whisper.cpp/issues/3838
- resvg README/license: https://github.com/linebender/resvg/blob/main/README.md
- resvg changelog/license transition: https://github.com/linebender/resvg/blob/main/CHANGELOG.md
- FFmpeg legal/licensing guidance: https://www.ffmpeg.org/legal.html
