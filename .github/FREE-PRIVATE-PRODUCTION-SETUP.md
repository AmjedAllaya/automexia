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

## 7. Ordinary PR CI

Every PR to `main` runs only three Linux-hosted gates:

```text
Repository and workflow policy
Rust quality and tests
Dependency security
```

A branch whose name starts with `release/` additionally runs `Release candidate gate`, including the coverage regression check. This keeps routine GitHub Free usage reasonable while preserving a stronger release ceremony.

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

## 9. Deep assurance

`nightly.yml` is manual-only in this edition. Daily scheduled fuzz/Miri/sanitizer runs can consume a private Free repository's allowance rapidly. Run **Deep assurance (manual)** before major releases when desired. The specialized S1/S2/F5 workflows are also manual/self-hosted controls.

## 10. Trust boundary

GitHub Free private repositories cannot technically prevent a repository administrator from changing workflows or bypassing team conventions. Repository secrets are therefore appropriate only when every person with write access is trusted with release authority. For stronger separation of duties, move signing to an external controlled signer or use a plan/platform with server-enforced protected environments/rulesets.

## 11. If many jobs fail immediately

Read `ACTIONS-STARTUP-TROUBLESHOOTING.md`. Failures before checkout/build normally indicate Actions permissions, account usage/billing state, or stale workflow files rather than independent Rust failures.
