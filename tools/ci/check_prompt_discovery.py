"""Source-boundary guards supplement (never replace) native prompt regressions."""
from pathlib import Path
import re
from check_command_productivity import rust_code_without_comments_and_literals

ROOT = Path(__file__).resolve().parents[2]
OWNERS = {
    "helper": "apps/automexia-terminal/src/automexia/prompt_discovery.rs",
    "main": "apps/automexia-terminal/src/main.rs",
    "runtime": "apps/automexia-terminal/src/automexia/runtime.rs",
    "parser": "automexia-devops/src/kubernetes.rs",
    "context": "automexia-devops/src/context.rs",
    "renderer": "apps/automexia-terminal/src/renderer/devops_status.rs",
}


def validate(sources: dict[str, str]) -> None:
    sources = {name: rust_code_without_comments_and_literals(source) for name, source in sources.items()}
    helper = sources["helper"].split("#[cfg(test)]", 1)[0]
    required = ["std::env::current_exe()", "Command::new(executable)", "cancellation.is_cancelled()",
                "start.elapsed() >= deadline", "stop_and_reap(&mut child)", "child.try_wait()",
                "MAX_RESPONSE", "PeekNamedPipe", "Stdio::null()"]
    required.extend([
        "const MAX_REQUEST: usize = 8192;", "const MAX_RESPONSE: usize = 4096;",
        "const DEADLINE: Duration = Duration::from_millis(1500);",
        "const POLL: Duration = Duration::from_millis(10);",
    ])
    if any(token not in helper for token in required):
        raise ValueError("prompt helper lifecycle boundary changed")
    if re.findall(r"Command::new\s*\(\s*([^)]*?)\s*\)", helper) != ["executable"] or any(token in helper for token in ['println!', 'eprintln!', 'tracing::']):
        raise ValueError("prompt helper gained unreviewed launch or diagnostic authority")
    main = sources["main"]
    if main.index("prompt_discovery::dispatch_helper()") > main.index("panic::attach_handler()"):
        raise ValueError("prompt helper initializes application state")
    if "super::prompt_discovery::refresh(" not in sources["runtime"] or "session.environment.hash(&mut hasher)" not in sources["runtime"]:
        raise ValueError("prompt discovery lost its live caller or session identity")
    runtime = sources["runtime"].split("#[cfg(test)]", 1)[0]
    if "session.title.hash(&mut hasher)" in runtime or "same_devops_context(&capsule.session, session)" not in runtime:
        raise ValueError("mutable titles can restart passive discovery")
    if runtime.index("publish_devops_progress(&request, &snapshot)") > runtime.index("super::prompt_discovery::refresh("):
        raise ValueError("guest reads block initial context publication")
    progress = runtime.split("fn put_devops_progress(", 1)[1].split("fn publish_devops_failure", 1)[0]
    if any(token not in progress for token in ["!self.accepts_refresh(request)", "self.devops_snapshot(session_id).is_some()", "Freshness::Refreshing"]):
        raise ValueError("intermediate context lost stale-result or anti-flicker guards")
    if "fn accepts_refresh(" not in runtime:
        raise ValueError("context refresh lost its shared acceptance owner")
    acceptance = runtime.split("fn accepts_refresh(", 1)[1].split("fn register_refresh(", 1)[0]
    required_acceptance = ["self.context_status_enabled()", "&& self.context_revision == request.context_revision", "&& !request.cancellation.is_cancelled()", "&& self.accepts(", "request.session.session_id", "request.operation_id", "request.capsule_revision"]
    if any(token not in acceptance for token in required_acceptance):
        raise ValueError("context refresh lost feature, revision, cancellation or session ownership")
    if progress.index("!self.accepts_refresh(request)") >= progress.index("self.put_devops_snapshot("):
        raise ValueError("context progress publication must follow request acceptance")
    renderer = sources["renderer"].split("#[cfg(test)]\nmod tests", 1)[0]
    if "self.request_in_flight = self.refresh_pending" not in renderer or "runtime::same_devops_context(previous, session)" not in renderer:
        raise ValueError("renderer discards progress or restarts title-only discovery")
    context = sources["context"].split("#[cfg(all(test", 1)[0]
    if "let kubernetes = if view.wsl.is_some()" not in context or "kubernetes_context(host_home" in context:
        raise ValueError("guest context can select the host cluster")
    for name in ["parser", "context"]:
        production = sources[name].split("#[cfg(test)]", 1)[0].split("#[cfg(all(test", 1)[0]
        if any(token in production for token in ["std::process", "Command::new", "std::net", ".spawn("]):
            raise ValueError("passive extension gained launch or network authority")
    parser = sources["parser"]
    for token in ["pub const MAX_BYTES: usize = 1024 * 1024;", "pub const MAX_FILES: usize = 16;",
                  "const MAX_CONTEXTS: usize = 256;", "max_depth: 32,", "max_documents: 1,",
                  "max_events: 20_000,", "max_merge_keys: 0,", "MergeKeyPolicy::Error"]:
        if token not in parser:
            raise ValueError("passive parser resource boundary changed")


def sources() -> dict[str, str]:
    return {name: (ROOT / relative).read_text(encoding="utf-8") for name, relative in OWNERS.items()}


if __name__ == "__main__":
    validate(sources())
    print("PASS: prompt discovery source boundaries")
