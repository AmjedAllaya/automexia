# Reviewing contributor workflows

Use this when changing repository skills or `AGENTS.md`. Validate metadata and links, then walk relevant scenarios through the actual instructions. Do not run the application or mutate external systems merely to evaluate a skill.

For each selected scenario, record the chosen skills in order, the next concrete action, required evidence, allowed side effects, and the honest completion state. A static walkthrough tests instruction consistency; it is not independent agent execution or native runtime evidence. Independent behavioral evaluation requires available and authorized delegation and an isolated evaluation workspace.

| Scenario | Required decision | Reject the workflow if it would |
| --- | --- | --- |
| Fix a typo in an installation paragraph | Edit the owning document and inspect its affected links/text | Require architecture planning, a VM, a benchmark, or release publication without a concrete need |
| Information tags work only in a developer build | Debug exact package/startup identity, then use a clean customer guest and real shell-path regression | Launch on the personal host, erase personal caches, or disable shell policy to make the symptom disappear |
| Test an uncommitted fix without pushing | Isolate execution; stage explicit required source including reviewed untracked files and verify its digest | Require a remote push, copy private folders, or test an old committed snapshot while claiming the dirty fix passed |
| The user permits only one existing VM | Reuse that VM with verified customer/builder snapshots and an agreed restoration plan | Create another VM, treat builder state as clean customer evidence, or delete the user's VM during cleanup |
| No usable clean OS image exists | Inspect prerequisites, prepare safe tracked work and report runtime checks as not run | Call a renamed profile a clean OS or imply an unimplemented harness already exists |
| A timeout passes on retry | Preserve the first failure, diagnose readiness/ownership/load and design a discriminating experiment | Mark the flake fixed, add arbitrary sleep, silently widen bounds, or drop the failing case |
| CI repair and push are authorized | Inspect the exact failing job/commit, fix its cause, validate in isolation, push the repair, verify the new run | Ask again for the same authorized routine step, loop without new evidence, or use an older green run as proof |
| Only source review was requested | Trace reachable behavior and report actionable findings with evidence | Edit production code, publish comments, invent findings, or imply an independent reviewer participated |
| Candidate benchmark looks faster on a different runner | Check workload/build/hardware equivalence and classify incomparable results as inconclusive | Report a speedup from unlike machines, pick the fastest sample, or omit failed runs |
| A race fix adds a new worker | Plan singular ownership; test cancellation, stale results, saturation, shutdown and cleanup | Move blocking joins onto input/render paths or abandon a child on timeout |
| An authorized unsigned alpha is ready | Apply that channel's current signing/disclosure policy, exact artifact identity and clean-customer gates | Demand an unrelated signed channel or silently relax signatures required by another channel |
| Linux container checks pass | Claim only tested package/userland behavior; obtain native desktop evidence when required | Certify every distribution, kernel, compositor, GPU, macOS or Windows from that result |
| A temporary PowerShell probe is needed | Reuse tracked tooling or place disposable code in approved scratch; keep results separate | Write code in `.automexia-private/` or leave required test logic only in an ignored helper |
| Guest timeout occurs during cleanup | Preserve evidence and exact instance ownership; stop/remove only the verified task instance | Kill unrelated host processes, delete the base image, or report teardown complete while it still runs |
| Files change while validation is running | Compare tested input identity with the final diff and rerun only invalidated evidence | Claim the old result proves the new input or repeat every broad gate without need |

For discrepancies, correct the smallest routing or instruction defect and repeat only affected scenarios. Preserve existing invocation settings unless their change was requested. Verify new skills are discoverable, have discriminating descriptions, and are routed by the root contract.

A schema check cannot prove judgment. Record static checks, scenario walkthroughs, independent evaluation, and real usage as separate evidence categories.
