# GitHub Free private — production setup

## 1. Clean installation

Delete the existing `.github` directory before extracting this archive. Do **not** overlay it over an older Enterprise or CodeQL configuration. Old workflows are not deleted by ZIP extraction and would continue to run.

PowerShell from the repository root:

```powershell
if (Test-Path .github) { Remove-Item .github -Recurse -Force }
Expand-Archive .\automexia-github-free-private-production.zip -DestinationPath . -Force
Get-ChildItem .github\workflows
```

The workflow list should contain only:

```text
ci.yml
f5-openssh-assurance.yml
linux-early-access.yml
nightly.yml
release.yml
s1-assurance.yml
s2-assurance.yml
```

There must be no `codeql.yml`, `workflow-security.yml`, or `release-drafter.yml`.

## 2. GitHub Actions settings

In **Settings → Actions → General**:

* enable GitHub Actions;
* choose the restrictive **Allow select actions and reusable workflows** policy (wording can vary slightly by GitHub UI);
* allow actions created by GitHub plus only these third-party repository patterns:

```text
anchore/sbom-action@*
azure/artifact-signing-action@*
azure/login@*
taiki-e/install-action@*
```

* remove obsolete patterns such as `codecov/codecov-action@*`, `release-drafter/release-drafter@*`, and `zizmorcore/zizmor-action@*` if they remain from an older configuration;
* enable **Require actions to be pinned to a full-length commit SHA** when the setting is available;
* use **Read repository contents permission** as the default workflow permission;
* keep **Allow GitHub Actions to create and approve pull requests** disabled unless you explicitly need it elsewhere.

The workflow does **not** invoke `zizmorcore/zizmor-action`. It installs pinned `zizmor@1.21.0` through the already-allowlisted, full-SHA-pinned `taiki-e/install-action`, then executes the CLI offline. `taiki-e/install-action` manages `zizmor` as a supported GitHub-Release tool and verifies release checksums by default.

Repository-level GitHub Free cannot make branch review/ruleset policy administrator-proof. Keep write access limited to trusted maintainers and require PRs by team convention.

The release-only authenticated repository audit remains deliberately
fail-closed and reports an external manual-governance prerequisite on this
plan. Do not replace it with an environment, ruleset, or paid security product:
record the independent review and verify the repository settings before a
release owner proceeds.

## 3. Free-plan release governance variables

The workflow is **secure by default** even when these variables are not created: it requires at least one independent human approval and requires the person merging the release PR to be different from the PR author. For a team, explicitly configure:

```text
AUTOMEXIA_RELEASE_MIN_APPROVALS=1   # use 2 when your team can sustain two-person review
AUTOMEXIA_RELEASE_REQUIRE_DISTINCT_MERGER=1
```

A solo developer must explicitly relax the two-person ceremony in **Settings → Secrets and variables → Actions → Variables**:

```text
AUTOMEXIA_RELEASE_MIN_APPROVALS=0
AUTOMEXIA_RELEASE_REQUIRE_DISTINCT_MERGER=0
```

This is an explicit reduction in release governance, not the default.

Those two variables apply only to the multi-platform stable-release workflow.
The Linux Early Access workflow deliberately does not read them: public Linux
publication always requires a current approval of the exact pull-request head
and a merger distinct from the pull-request author. On the current private
GitHub Free source repository, server-enforced branch protection is unavailable;
until a second trusted reviewer exists, public Linux publication remains an
external prerequisite rather than silently weakening this rule.

## 4. Windows production signing

Set variable:

```text
AUTOMEXIA_WINDOWS_SIGNING_BACKEND=pfx
AUTOMEXIA_WINDOWS_PUBLISHER_SUBJECT=<exact certificate Subject>
```

For PFX signing set repository secrets:

```text
AUTOMEXIA_WINDOWS_CERTIFICATE=<base64 PFX>
AUTOMEXIA_WINDOWS_CERTIFICATE_PASSWORD=<password>
```

Or set `AUTOMEXIA_WINDOWS_SIGNING_BACKEND=azure-artifact-signing` and configure:

```text
AZURE_ARTIFACT_SIGNING_ENDPOINT
AZURE_ARTIFACT_SIGNING_ACCOUNT
AZURE_ARTIFACT_SIGNING_PROFILE
```

and secrets:

```text
AZURE_CLIENT_ID
AZURE_TENANT_ID
AZURE_SUBSCRIPTION_ID
```

The release fails **before expensive builds** when the selected signing configuration is incomplete.

## 5. macOS production signing/notarization

Configure repository secrets:

```text
APPLE_CERTIFICATE              # base64 Developer ID Application .p12
APPLE_CERTIFICATE_PASSWORD
APPLE_ID
APPLE_PASSWORD                 # app-specific password
APPLE_TEAM_ID
APPLE_SIGNING_IDENTITY
```

The stable release intentionally fails closed without Apple production signing/notarization. Apple Developer membership is external to GitHub and is not included by the GitHub Free plan.

## 6. Free cryptographic release checksum signing

Linux archives do not have an OS-native Authenticode/Developer-ID equivalent, so the release also signs `SHA256SUMS` with a dedicated **minisign** Ed25519 release key. This is free and gives every release asset a common cryptographic integrity root.

Generate this key **once on a trusted offline/local machine**, not inside GitHub Actions:

```bash
minisign -G -W -p automexia-release.pub -s automexia-release.key
```

`-W` intentionally creates an unencrypted automation key. The security boundary is the GitHub secret plus the isolated signing step; keep the original key offline as a backup and restrict repository write access.

Set repository variable:

```text
AUTOMEXIA_RELEASE_MINISIGN_PUBLIC_KEY=<the RW... base64 line from automexia-release.pub>
```

Store the entire secret-key file as a base64 GitHub Actions secret named:

```text
AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY
```

PowerShell encoding example:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes('.\\automexia-release.key')) | Set-Clipboard
```

Linux/macOS encoding example:

```bash
base64 -w0 automexia-release.key
```

The release workflow reconstructs the key only in a dedicated read-only signing job, signs `SHA256SUMS`, verifies the signature using the public key, deletes the temporary key, and publishes `SHA256SUMS.minisig`. The final publication job never receives the minisign secret.

Users can verify a downloaded release with:

```bash
minisign -V -P '<RW...public-key-line>' -m SHA256SUMS -x SHA256SUMS.minisig
sha256sum -c SHA256SUMS
```

## 6.1 Public Linux Early Access repository and GitHub App

Linux Early Access publishes to the passive public repository
`AmjedAllaya/automexia-releases`; it does not publish public assets from the
private source repository. That repository must remain public, keep Actions
disabled, protect `main` with required cryptographic commit signatures, protect
`v*` release tags, enable private vulnerability reporting, and return a
successful authenticated response from:

```text
GET /repos/AmjedAllaya/automexia-releases/immutable-releases
```

Register a GitHub App and grant only repository **Contents: read/write** and
**Administration: read-only**. Install it only on `automexia-releases`. In this
private source repository set:

```text
AUTOMEXIA_DISTRIBUTION_APP_CLIENT_ID          # Actions variable
AUTOMEXIA_DISTRIBUTION_APP_PRIVATE_KEY        # Actions secret, PEM contents
```

The workflow mints a one-hour installation token restricted to that one
repository. Contents write creates the draft/tag/assets; Administration read
only checks immutable-release configuration. The App cannot change that setting,
and the source repository's normal `GITHUB_TOKEN` remains read-only.

The repository owner enables immutable releases once with administration write.
Do not grant that write permission to the publication App. Rotate an exposed App
private key immediately, remove the old key from the App, and rerun only a new
patch version; never replace a published release.

The public archive uses active no-bypass rulesets in addition to legacy branch
protection: `Protect main` requires squash-only reviewed changes with code-owner,
last-push, stale-review, linear-history, and resolved-thread controls; `Protect
release tags` rejects deletion, update, and non-fast-forward changes to `v*`.
Issues, Projects, the wiki, and Actions remain disabled. Repository topics,
description, homepage, distribution notice, security policy, and support policy
are public metadata only.

Every publication fetches the live repository, immutable-release, default-branch
ruleset, and release-tag ruleset payloads using the pinned GitHub API. The
repository-owned validator rejects visibility, interactive-feature, merge-mode,
scope, bypass, approval, review, linear-history, or tag-rewrite drift before any
tag or draft is created.

## 7. Ordinary PR CI

Every PR to `main` runs only three Linux-hosted gates:

```text
Repository and workflow policy
Rust quality and tests
Dependency security
```

An **internal** branch whose name is exactly `release/X.Y.Z` additionally runs:

```text
Release candidate gate                 (ubuntu-24.04)
Release candidate Windows coverage     (windows-2025)
```

The Linux job validates origin, version, protected paths, and tag uniqueness.
The Windows job waits for it and ordinary quality, then compares the exact PR
commit range to the repository's `windows-x86_64-msvc` coverage baseline. The
split prevents a Linux report from being compared to a Windows baseline and
prevents fork PRs from consuming Windows minutes.

Standard Windows-hosted time consumes the private repository's included GitHub
Free minutes at the Windows multiplier. Keep paid overage disabled if the goal
is a hard zero-cost ceiling; a release PR then waits when included quota is
exhausted rather than creating a charge.

## 7.1 Release workflow behavior on ordinary PR merges

GitHub cannot filter a `pull_request: closed` trigger by **head** branch name at workflow-trigger level. Therefore GitHub may create a lightweight `Stable release` workflow run when an ordinary PR is closed or merged. For any branch that is not exactly `release/X.Y.Z`, `Authorize merged release PR` is skipped and every downstream release job, including the final artifact gate, is also skipped. The run must not publish anything and must not produce a release-gate failure.

The run title is intentionally explicit, for example:

```text
Release gate · PR #42 · ci/refactor → main
Release gate · PR #57 · release/1.2.3 → main
```

Only the second form can authorize a stable release.

## 8. Creating a release

Do not manually create a `vX.Y.Z` tag.

```powershell
git checkout main
git pull --ff-only origin main
git checkout -b release/1.2.3
# bump automexia-terminal version to 1.2.3, update Cargo.lock/changelog as needed
git add .
git commit -m "release: prepare 1.2.3"
git push -u origin release/1.2.3
```

Open an **internal** PR:

```text
release/1.2.3 -> main
```

After CI/reviews, merge it. `Stable release` then:

1. validates the merge event, source repository, stable SemVer, current human approvals, and distinct-merger policy;
2. binds the release to GitHub's merged-event `GITHUB_SHA` (the resulting base-branch commit) and checks that this exact commit is still current `main`; this works with normal merge and squash-merge release PRs without trusting the release branch HEAD SHA;
3. rejects release PRs that changed `.github/`, `tools/ci/`, `tools/xtask/`, `packaging/`, or `shell-integration/`;
4. verifies the Cargo package version and release uniqueness;
5. validates signing prerequisites before spending six-platform minutes;
6. reruns full Rust quality, tests, RustSec, cargo-deny, and shell contracts after merge;
7. builds native x64/ARM64 Windows, GNU/Linux, Intel/Apple-Silicon macOS artifacts;
8. builds GNU/Linux on Ubuntu 22.04 to enforce the GLIBC 2.35 compatibility baseline;
9. signs Windows, produces and notarizes a universal macOS DMG, and builds DEB/RPM/tar.gz packages;
10. tests the exact final Windows ARM64 package on ARM64, final DMG on Intel and Apple Silicon, and Linux packages natively;
11. performs independent Linux x64 and ARM64 reproducibility checks;
12. generates SPDX + CycloneDX SBOMs, `release-manifest.json`, and `SHA256SUMS`;
13. signs `SHA256SUMS` with the dedicated minisign release key;
14. creates annotated `v1.2.3` only after all gates pass;
15. publishes the GitHub Release.

### Linux Early Access only

Before provisioning signing keys or a GitHub App, dispatch `Linux Early Access
release` manually from the exact candidate ref and provide the Cargo workspace
version as `X.Y.Z`. This credential-free rehearsal runs the native package jobs,
assembles only the six unsigned packages plus the manifest, and retains a
seven-day private artifact containing
`REHEARSAL-NOT-A-PUBLIC-RELEASE.txt`. It cannot read release secrets, mint an App
token, sign, publish, create a tag, or emit a website activation handoff.

The quality job stays inside the standard free Linux runner envelope by using
one Cargo build job, one nextest thread, development/test profiles without debug
artifacts, a source-only Cargo registry/Git cache keyed by `Cargo.lock`, and a
clean boundary between all-target Clippy and all-feature tests. Do not add
`target` to this cache or reorder/remove the cleanup: the policy mutation suite
rejects those changes because the first real rehearsal exhausted the linker
after retaining the lint graph.

Each native x64/Arm64 package job also uses one Cargo build job and disables
release debug data. The policy mutation suite rejects either limit being
weakened. This follows a real corrected-quality rehearsal in which x64 passed
but the hosted Arm64 runner shut down during compilation with status 143; only a
new successful rehearsal counts as native package evidence.

Use a separate internal branch named exactly `release/linux/X.Y.Z`. After an
independent approval and distinct merger, `linux-early-access.yml` reruns the
Linux release-quality gate, builds x64 and Arm64 packages natively, signs the
public bundle, publishes one immutable prerelease in `automexia-releases`, and
uses GitHub's current versioned API plus `gh release verify` and
`gh release verify-asset` for all sixteen assets before retaining
`website-activation.json` for 30 days. It does not publish Windows or macOS and
does not enable the website.

Review the activation handoff in the landing-page repository, set its trusted
minisign public-key deployment variable, run its complete live-release verifier,
review the protected preview, and only then change the Linux channel to
available. See `docs/PUBLIC-RELEASE-DISTRIBUTION.md`.

## 9. Deep assurance

`nightly.yml` is manual-only in this edition. Daily scheduled fuzz/Miri/sanitizer runs can consume a private Free repository's allowance rapidly. Run **Deep assurance (manual)** before major releases when desired. The specialized S1/S2/F5 workflows are also manual/self-hosted controls.

## 9.1 Local assurance before a push

On every trusted contributor machine, install and verify the free local
assurance tools once from a clean repository checkout:

```text
cargo xtask assurance install-tools
cargo xtask assurance initialize-vet
cargo xtask assurance pre-push
```

The installer places all downloaded or built tools in the ignored repository
local `.automexia-tools/` directory. It checksum-verifies Actionlint and
Gitleaks release archives, pins Semgrep Community Edition, Cargo Audit, Cargo
Vet, and Zizmor versions, and never stores credentials in that directory. The
pre-push profile checks the repository readiness gate, workflow pin/policy and
mutation contracts, Actionlint, offline Zizmor, RustSec/Cargo Deny/Cargo Vet,
Gitleaks against introduced commits and current files, local Semgrep rules, and
real scanner canaries. `initialize-vet` generates Cargo Vet's source-controlled
baseline once and verifies an existing complete baseline without rewriting it;
partial records fail closed. Inspect its generated `supply-chain/` records
before committing them. It is a dependency-change ratchet, not a substitute for
reviewing the imported audit criteria.

Scanner canaries must never inherit `GIT_DIR`, `GIT_WORK_TREE`, index, object,
config, prefix, or replacement state from the calling hook. The Gitleaks canary
creates and commits only inside its temporary repository, then proves the
caller's HEAD, worktree status, and local repository configuration did not
change.

Use `cargo xtask assurance audit-history-secrets` for the separate all-history
campaign. It remains fail-closed while legacy generic-key findings await
human-approved remediation; the regular profile does not suppress them with a
broad allowlist. Hosted CI follows the same introduced-commit boundary using
validated GitHub event SHAs and still scans the complete checked-out working
tree.

Use `cargo xtask assurance install-hook` only on a machine that has no existing
pre-push hook. It refuses to replace an existing hook. `release-local` adds
release policy checks; `deep-source` must run from Linux or a native WSL
checkout and runs bounded Miri, sanitizer, and fuzz campaigns. These local
profiles do not claim GitHub plan controls, native macOS/Windows accessibility,
or signing/notarization evidence.

## 10. Trust boundary

GitHub Free private repositories cannot technically prevent a repository administrator from changing workflows or bypassing team conventions. Repository secrets are therefore appropriate only when every person with write access is trusted with release authority. For stronger separation of duties, move signing to an external controlled signer or use a plan/platform with server-enforced protected environments/rulesets.

## 11. If many jobs fail immediately

Read `ACTIONS-STARTUP-TROUBLESHOOTING.md`. Failures before checkout/build normally indicate Actions permissions, account usage/billing state, or stale workflow files rather than independent Rust failures.
