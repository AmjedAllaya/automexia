use automexia_keybindings::{
    compile, parse_binding_line, parse_binding_lines, BindingOrigin, KeyAtom, ModeFlags,
    Modifiers, SequenceResolution, SurfaceBindingState, Trigger,
};
use proptest::prelude::*;

fn ctrl(key: &str) -> Trigger {
    Trigger::new(KeyAtom::Logical(key.to_string()), Modifiers::CONTROL).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn hostile_binding_text_is_total_and_deterministic(input in any::<String>()) {
        let first = parse_binding_line(&input, BindingOrigin::User);
        let second = parse_binding_line(&input, BindingOrigin::User);
        prop_assert_eq!(first, second);
    }

    #[test]
    fn invalid_sequence_flushes_exact_encoded_bytes(
        prefix in prop::collection::vec(any::<u8>(), 0..128),
        suffix in prop::collection::vec(any::<u8>(), 0..128),
    ) {
        let specs = parse_binding_lines(
            ["ctrl+a>ctrl+b=quit"],
            BindingOrigin::Profile,
        )
        .unwrap();
        let registry = compile(&specs).registry.unwrap();
        let mut state = SurfaceBindingState::default();
        prop_assert_eq!(
            state.resolve(&registry, &ctrl("a"), &prefix, ModeFlags::empty()),
            SequenceResolution::Pending,
        );
        let resolution = state.resolve(&registry, &ctrl("x"), &suffix, ModeFlags::empty());
        let mut expected = prefix;
        expected.extend_from_slice(&suffix);
        prop_assert_eq!(
            resolution,
            SequenceResolution::Flush {
                bytes: expected,
                reason: automexia_keybindings::CancellationReason::InvalidContinuation,
            },
        );
    }
}
