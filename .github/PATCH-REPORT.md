# Free/private production rebuild

* aligned the workflow with restrictive repository-level Actions policies by installing pinned `zizmor@1.21.0` through `taiki-e/install-action` rather than invoking `zizmorcore/zizmor-action`;
This tree is a **clean replacement**, not an overlay. Major fixes relative to the previous archive include:

* removed stale private CodeQL, Release Drafter and separate workflow-security workflows;
* reduced normal PR CI to Linux-only security/quality gates to conserve Free minutes;
* made deep nightly/fuzz/Miri/sanitizer assurance manual-only;
* retained merged internal `release/X.Y.Z` as the only stable-release authorization event;
* added robust latest-human-review counting and optional distinct-merger enforcement;
* added after-merge mandatory release quality/security tests;
* explicitly selects the compiler from `rust-toolchain.toml` and verifies actual rustc/Cargo identity in native release/package jobs;
* moved GNU/Linux production builds/packages to Ubuntu 22.04 / GLIBC 2.35 baseline;
* replaced project/enterprise release-manifest coupling with a self-contained allowlist/manifest generator;
* removed an unconditional dependency on optional hardware evidence;
* fixed Windows MSI staging filename whitespace;
* fixed generated final-verification shell/PowerShell blocks;
* fixed Linux `find` grouping syntax in final verification;
* fixed universal macOS DMG creation to stage an actual `.app` inside a DMG root;
* kept a single isolated final `contents: write` publication job;
* creates the annotated version tag only after final gates succeed.

* added an exact seven-workflow inventory contract so stale workflows fail policy validation;
* added a reviewed external-Action repository allowlist in addition to full-SHA pin enforcement;
* fixed publication-preparation `needs` wiring so the reproducibility result is a direct dependency;
* added an isolated minisign checksum-signing job and published `SHA256SUMS.minisig` for Linux/common artifact integrity;
* moved macOS universal-app execution out of the Apple-secret-bearing signing step;
* added startup troubleshooting for Actions permission/quota failures that occur before checkout.
* split Windows release processing into source preparation, runtime signing, unsigned packaging, final MSI signing, and native final verification so signing credentials never coexist with a repository checkout;
* split macOS universal-app assembly from Developer-ID/notarization so Apple credentials never coexist with a source checkout or application execution;
* added a pre-build signing-readiness gate that checks credential presence without checking out repository code;
* normalized the release source to the merged-event `GITHUB_SHA`, then verifies it is still current `main`, avoiding dependence on PR-head SHA semantics across merge methods;
* changed release governance defaults to fail closed with one independent human approval and a distinct merger unless a solo maintainer explicitly opts out.
* fixed the final artifact gate so a merged non-`release/X.Y.Z` PR produces skipped/neutral release jobs instead of one failing red gate;
* added an explicit human-readable workflow `run-name` based on PR number and source branch;
* replaced matrix expressions in job display names with stable readable labels so skipped jobs never show literal `${{ matrix.* }}` text.
