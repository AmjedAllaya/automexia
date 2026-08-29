# GitHub Actions startup troubleshooting

Use this when many unrelated jobs fail before the first real build step (typically in a few seconds).

## 1. Check Actions is enabled

Open **Settings → Actions → General** and confirm GitHub Actions is enabled for the repository.

For this Free/private edition, the most portable repository setting is **Allow all actions and reusable workflows**. The workflows themselves pin every external action to an immutable 40-character commit SHA and CI audits those pins. If your account exposes **Require actions to be pinned to a full-length commit SHA**, enable it as an additional server-side control.

Use **Read repository contents permission** as the default workflow permission and keep **Allow GitHub Actions to create and approve pull requests** disabled unless another workflow genuinely requires it.

## 2. Check private-repository Actions usage

Open your GitHub billing/usage page and verify that hosted Actions usage is available. If the private-repository allowance is exhausted or Actions has been placed in a billing-disabled state, hosted jobs can fail before checkout/build steps run.

This repository intentionally keeps normal PR CI on Linux only and reserves Windows/macOS/ARM hosted work for real stable releases to reduce that risk.

## 3. Confirm this is a clean `.github` install

The only workflow files in this edition must be:

```text
ci.yml
f5-openssh-assurance.yml
nightly.yml
release.yml
s1-assurance.yml
s2-assurance.yml
```

There must be no stale `codeql.yml`, `workflow-security.yml`, or `release-drafter.yml`.

PowerShell check:

```powershell
Get-ChildItem .github\workflows | Select-Object -ExpandProperty Name | Sort-Object
```

## 4. Open the actual first failed step

If a job fails before `Check out repository`, the cause is GitHub runner/account/action policy rather than Rust compilation. If checkout succeeds and a later step fails, use that step's exact log; the workflow is intentionally fail-closed.
