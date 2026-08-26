#![no_main]

use automexia_keybindings::{
    compile_with_options, parse_binding_lines, BindingOrigin, CompileOptions, ModeFlags,
    SurfaceBindingState, MAX_FIXTURE_BYTES,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FIXTURE_BYTES {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(bindings) = parse_binding_lines(text.lines(), BindingOrigin::User) else {
        return;
    };
    let report = compile_with_options(&bindings, CompileOptions { strict: false });
    let Some(registry) = report.registry else {
        return;
    };

    // Exercise the direct and sequence trie for every accepted chord while
    // keeping every simulated surface independent and retained input bounded.
    for binding in &bindings {
        let mut state = SurfaceBindingState::default();
        for trigger in &binding.sequence {
            let encoded = trigger.to_string();
            let _ =
                state.resolve(&registry, trigger, encoded.as_bytes(), ModeFlags::empty());
        }
        let _ = state.cancel(automexia_keybindings::CancellationReason::SurfaceClosed);
    }
});
