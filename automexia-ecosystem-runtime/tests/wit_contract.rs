use std::path::PathBuf;

use automexia_ecosystem::{ALLOWED_IMPORTS, WIT_WORLD};
use wit_parser::{Resolve, WorldKey};

#[test]
fn checked_in_wit_parses_and_matches_the_exact_reviewed_world_and_imports() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../wit/automexia-ecosystem-1.0.0");
    let mut resolve = Resolve::default();
    let (package, _) = resolve.push_dir(path).expect("checked-in WIT must parse");
    let world = resolve
        .select_world(&[package], Some("extension"))
        .expect("the reviewed suggestion world must exist");
    let world = &resolve.worlds[world];

    assert_eq!(WIT_WORLD, "automexia:ecosystem/suggestion@1");
    assert_eq!(world.name, "extension");
    let imports = world
        .imports
        .keys()
        .map(|key| match key {
            WorldKey::Name(name) => format!("automexia:ecosystem/{name}@1"),
            WorldKey::Interface(id) => {
                let interface = &resolve.interfaces[*id];
                let name = interface
                    .name
                    .as_deref()
                    .expect("reviewed interfaces must retain names");
                let package = interface
                    .package
                    .map(|id| &resolve.packages[id].name)
                    .expect("reviewed interfaces must retain package ownership");
                package
                    .interface_id(name)
                    .strip_suffix(".0.0")
                    .unwrap_or_else(|| {
                        panic!("reviewed interface must use an exact 1.0.0 ABI")
                    })
                    .to_owned()
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(imports, ALLOWED_IMPORTS);
    assert_eq!(world.exports.len(), 1);
    assert!(
        matches!(world.exports.keys().next(), Some(WorldKey::Name(name)) if name == "run")
    );
}
