#![no_main]

use automexia_devops::actions::{
    action_digest, builtin_packs, evaluate_pack_health, materialize_pack_action,
    plan_pack_update, PackOverlay, PackToolObservation, ShellKind,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let packs = builtin_packs();
    let pack = &packs[data.first().copied().unwrap_or_default() as usize % packs.len()];
    let version_output = String::from_utf8_lossy(data.get(1..).unwrap_or_default())
        .chars()
        .take(160)
        .collect::<String>();
    let completion_shells = [
        ShellKind::Powershell,
        ShellKind::Bash,
        ShellKind::Zsh,
        ShellKind::Fish,
        ShellKind::Cmd,
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(index, shell)| {
        (data.get(index + 1).copied().unwrap_or_default() & 1 == 1).then_some(shell)
    })
    .collect();
    let _ = evaluate_pack_health(
        pack,
        &PackToolObservation::Detected {
            version_output,
            completion_shells,
        },
    );

    let entry = &pack.actions
        [data.get(2).copied().unwrap_or_default() as usize % pack.actions.len()];
    if let Ok(mut custom_action) = materialize_pack_action(&pack.id, &entry.action.id) {
        custom_action.display_name.push_str(
            &String::from_utf8_lossy(data.get(3..).unwrap_or_default())
                .chars()
                .filter(|character| !character.is_control())
                .take(32)
                .collect::<String>(),
        );
        let digest = if data.get(1).copied().unwrap_or_default() & 2 == 0 {
            action_digest(&entry.action)
        } else {
            "0".repeat(64)
        };
        let _ = plan_pack_update(
            pack,
            pack,
            &[PackOverlay {
                action_id: entry.action.id.clone(),
                base_action_digest: digest,
                custom_action,
            }],
        );
    }
});
