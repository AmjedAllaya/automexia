use automexia_command_productivity::suggestions::helper::HelperRequest;
use automexia_command_productivity::suggestions::{
    QuoteContext, ReplacementSpan, RouteIdentity, ShellKind, SuggestionCapability,
};
use automexia_terminal::automexia::suggestions::HelperSessionBinding;

pub fn binding() -> HelperSessionBinding {
    HelperSessionBinding {
        route: RouteIdentity {
            application_generation: 1,
            window_id: 2,
            tab_id: 3,
            pane_id: 4,
            session_id: 5,
            shell: ShellKind::Bash,
            editor_version: "5.2".into(),
            endpoint_instance: 6,
        },
        capability: SuggestionCapability::from_bytes([8; 32]),
        prompt_generation: 7,
        source_revision: 8,
    }
}

pub fn helper_request(generation: u64, candidate: &str) -> HelperRequest {
    HelperRequest {
        buffer: "git ch".into(),
        cursor_byte: 6,
        adapter_generation: generation,
        replacement_span: ReplacementSpan { start: 4, end: 6 },
        selection: None,
        quote_context: QuoteContext::ShellSpecific,
        native_candidates: vec![candidate.into()],
    }
}
