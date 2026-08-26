use automexia_ecosystem::{
    decode_strict_json, safe_relative_path, Capability, EcosystemManifest, GrantBinding,
    GrantError, InvocationBinding, Limits, SelectedInput,
};
use proptest::prelude::*;

fn invocation(
    exact_scope: &str,
    current_generation: u64,
    current_time: u64,
    revoked: bool,
) -> InvocationBinding<'_> {
    InvocationBinding {
        publisher_id: "example.publisher",
        extension_id: "example.extension",
        version: "1.0.0",
        package_sha256:
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        capability: Capability::SelectedInputReadOnce,
        exact_scope,
        profile_id: "dev",
        now_unix: current_time,
        generation: current_generation,
        revoked,
    }
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_manifest_bytes_never_escape_strict_bounds(source in proptest::collection::vec(any::<u8>(), 0..131_072)) {
        let result = decode_strict_json::<EcosystemManifest>(&source, Limits::MANIFEST_BYTES);
        if source.len() > Limits::MANIFEST_BYTES {
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn path_controls_and_platform_escapes_are_always_rejected(
        candidate in prop_oneof![
            "[a-z0-9]{0,24}".prop_map(|prefix| format!("{prefix}/../escape")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("/{suffix}")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("-{suffix}")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("root\\{suffix}")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("C:/{suffix}")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("safe\u{202e}{suffix}")),
            "[a-z0-9]{0,24}".prop_map(|suffix| format!("safe\0{suffix}")),
        ],
    ) {
        prop_assert!(!safe_relative_path(&candidate));
    }

    #[test]
    fn grants_are_exact_scope_generation_expiry_and_revocation_capabilities(
        pane in 1u16..4096,
        generation in 1u64..u64::MAX / 2,
        now in 1u64..u64::MAX / 2,
    ) {
        let scope = format!("profile/dev/pane/{pane}");
        let grant = GrantBinding {
            publisher_id: "example.publisher".into(),
            extension_id: "example.extension".into(),
            version: "1.0.0".into(),
            package_sha256: "a".repeat(64),
            capability: Capability::SelectedInputReadOnce,
            exact_scope: scope.clone(),
            profile_id: "dev".into(),
            expires_at_unix: now.saturating_add(2),
            generation,
        };

        prop_assert_eq!(grant.authorize(&invocation(&scope, generation, now, false)), Ok(()));
        prop_assert_eq!(
            grant.authorize(&invocation("profile/dev/pane/other", generation, now, false)),
            Err(GrantError::BindingMismatch)
        );
        prop_assert_eq!(
            grant.authorize(&invocation(&scope, generation.saturating_add(1), now, false)),
            Err(GrantError::StaleGeneration)
        );
        prop_assert_eq!(
            grant.authorize(&invocation(&scope, generation, now.saturating_add(2), false)),
            Err(GrantError::Expired)
        );
        prop_assert_eq!(
            grant.authorize(&invocation(&scope, generation, now, true)),
            Err(GrantError::Revoked)
        );
    }

    #[test]
    fn selected_model_input_is_never_accepted_over_its_byte_ceiling(extra in 1usize..4096) {
        let selection = "x".repeat(Limits::SELECTED_MODEL_INPUT_BYTES.saturating_add(extra));
        prop_assert!(SelectedInput::new(selection).is_err());
    }
}
