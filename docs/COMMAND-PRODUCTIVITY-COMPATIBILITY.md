# Shell productivity compatibility

This page records compatibility expectations for the public free-terminal shell
productivity behavior.

| Area | Authority | Automexia behavior | Fallback |
|---|---|---|---|
| Command editing | Native shell editor | Forwards input and preserves shell modes | Ordinary shell editor |
| Completion | Native shell | Session-local bounded integration only | Native completion |
| History | Native shell | Does not silently import history files | Native history |
| Quoting and parsing | Native shell | Inserts or copies reviewed text without Enter | Manual typing |
| Aliases | Native shell configuration | Optional reviewed Automexia-owned generated file | User aliases |
| Search | Terminal session | Bounded pane/workspace-visible results | Shell search |
| Clipboard | Operating system and focused route | Explicit copy/paste with no implicit Enter | Native clipboard |

Supported behavior must be verified separately for installed PowerShell, CMD,
Bash, Zsh, and Fish versions. A source fixture does not prove every native shell,
keyboard layout, IME, or operating system.

Unreleased integrations are intentionally outside this public compatibility
matrix.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Official Provider Completion Baseline

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Shell And Editor Ownership

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Version And Fallback Policy

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

## CP0 acceptance

The capability-free baseline names the supported shell families—PowerShell,
Bash, Zsh, Fish, and CMD—and the reviewed provider/tool vocabulary: Git,
Docker, Kubernetes, OpenShift, Helm, Terraform, OpenTofu, AWS, Azure, GCP, and
OpenSSH. Naming a provider documents compatibility input; it does not grant
provider authentication, network, process, or credential authority.

## Read-only discovery contract

Caller-owned discovery input is bounded to a 500 ms deadline, 256 KiB of output,
and a 4,096-entry ceiling. Missing, timed-out, malformed, oversized, unsupported,
or unauthenticated observations produce explicit unavailable/unknown state and
never activate an action, mutate native configuration, or weaken the native
shell/editor fallback.

## CP5 source-status clarification

CP5.0 fully done means the bounded research and source decision are complete;
it does not mean an optional preview or stable product is enabled.
CP5.1-CP5.4 are fully implemented at their source/local boundaries, while
native transport, shell, privacy, visual, accessibility, performance, resource,
package, disable, uninstall, rollback, and long-duration evidence remains gated.
