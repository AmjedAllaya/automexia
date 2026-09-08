//! The private renderer harness must never become an application dependency.

use serde_json::Value;

const PACKAGE: &str = "automexia-renderer-benchmarks";
const DEPENDENCIES: &[&str] = &[
    "criterion",
    "parking_lot",
    "rio-backend",
    "smallvec",
    "unicode-segmentation",
];

pub(super) fn verify(packages: &[Value]) -> Result<(), String> {
    let matching: Vec<_> = packages
        .iter()
        .filter(|package| package["name"].as_str() == Some(PACKAGE))
        .collect();
    let [package] = matching.as_slice() else {
        return Err("exactly one private renderer benchmark package is required".into());
    };
    if package["publish"]
        .as_array()
        .is_none_or(|value| !value.is_empty())
    {
        return Err("renderer benchmarks must remain unpublished".into());
    }
    let targets = package["targets"]
        .as_array()
        .ok_or("renderer benchmark targets are missing")?;
    if targets.len() != 1
        || targets[0]["name"].as_str() != Some("text_fit")
        || targets[0]["kind"] != serde_json::json!(["bench"])
        || targets[0]["crate_types"] != serde_json::json!(["bin"])
        || targets[0]["required-features"]
            .as_array()
            .is_some_and(|features| !features.is_empty())
    {
        return Err(
            "renderer package must contain only the unconditional text_fit benchmark"
                .into(),
        );
    }
    let dependencies = package["dependencies"]
        .as_array()
        .ok_or("renderer benchmark dependencies are missing")?;
    if package["features"] != serde_json::json!({"wgpu": ["rio-backend/wgpu"]})
        || !dependencies.iter().any(|dependency| {
            dependency["name"].as_str() == Some("rio-backend")
                && dependency["target"].as_str()
                    == Some("cfg(any(windows, target_arch = \"wasm32\"))")
                && dependency["features"] == serde_json::json!(["wgpu"])
        })
    {
        return Err(
            "renderer benchmark backend selection must retain Windows/wasm WGPU parity"
                .into(),
        );
    }
    if dependencies.len() != DEPENDENCIES.len() + 1
        || dependencies.iter().any(|dependency| {
            dependency["kind"].as_str() != Some("dev")
                || !dependency["name"]
                    .as_str()
                    .is_some_and(|name| DEPENDENCIES.contains(&name))
        })
        || DEPENDENCIES.iter().any(|required| {
            !dependencies.iter().any(|dependency| {
                dependency["name"].as_str() == Some(required)
                    && dependency["target"].is_null()
            })
        })
    {
        return Err(
            "renderer benchmarks require only their reviewed development dependencies"
                .into(),
        );
    }
    for other in packages {
        let dependencies = other["dependencies"]
            .as_array()
            .ok_or("package dependency metadata is missing")?;
        // Cargo's name is the real package identity even for renamed edges.
        if dependencies
            .iter()
            .any(|dependency| dependency["name"].as_str() == Some(PACKAGE))
        {
            return Err("no package may depend on the renderer benchmark package".into());
        }
        if other["name"].as_str() == Some("automexia-terminal")
            && other["targets"].as_array().is_some_and(|targets| {
                targets
                    .iter()
                    .any(|target| target["name"].as_str() == Some("text_fit"))
            })
        {
            return Err(
                "application must not duplicate the isolated text_fit benchmark".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify;
    use serde_json::{json, Value};

    fn fixture() -> Vec<Value> {
        vec![
            json!({
                "name": "automexia-renderer-benchmarks", "publish": [],
                "features": {"wgpu": ["rio-backend/wgpu"]},
                "targets": [{"name": "text_fit", "kind": ["bench"], "crate_types": ["bin"]}],
                "dependencies": [
                    {"name": "criterion", "kind": "dev"},
                    {"name": "parking_lot", "kind": "dev"},
                    {"name": "rio-backend", "kind": "dev"},
                    {"name": "smallvec", "kind": "dev"},
                    {"name": "unicode-segmentation", "kind": "dev"},
                    {"name": "rio-backend", "kind": "dev",
                     "target": "cfg(any(windows, target_arch = \"wasm32\"))",
                     "features": ["wgpu"]}
                ]
            }),
            json!({"name": "automexia-terminal", "dependencies": [], "targets": []}),
        ]
    }

    #[test]
    fn accepts_only_the_development_compilation_boundary() {
        assert!(verify(&fixture()).is_ok());
    }

    #[test]
    fn rejects_missing_duplicate_or_publishable_package() {
        assert!(verify(&fixture()[1..]).is_err());
        let mut duplicate = fixture();
        duplicate.push(duplicate[0].clone());
        assert!(verify(&duplicate).is_err());
        for publish in [Value::Null, json!(["crates-io"]), json!(false)] {
            let mut packages = fixture();
            packages[0]["publish"] = publish;
            assert!(verify(&packages).is_err());
        }
    }

    #[test]
    fn rejects_extra_replaced_or_conditional_targets() {
        for kind in ["bin", "lib", "custom-build", "example", "test"] {
            let mut packages = fixture();
            let target =
                json!({"name": "text_fit", "kind": [kind], "crate_types": ["bin"]});
            packages[0]["targets"] = json!([target.clone()]);
            assert!(verify(&packages).is_err());
            packages[0]["targets"] = fixture()[0]["targets"].clone();
            packages[0]["targets"].as_array_mut().unwrap().push(target);
            assert!(verify(&packages).is_err());
        }
        for (field, value) in [
            ("name", json!("replacement")),
            ("crate_types", json!(["cdylib"])),
            ("required-features", json!(["hidden"])),
        ] {
            let mut packages = fixture();
            packages[0]["targets"][0][field] = value;
            assert!(verify(&packages).is_err());
        }
    }

    #[test]
    fn rejects_dependency_authority_drift_and_missing_edges() {
        for index in 0..6 {
            for kind in [Value::Null, json!("build")] {
                let mut packages = fixture();
                packages[0]["dependencies"][index]["kind"] = kind;
                assert!(verify(&packages).is_err());
            }
            let mut packages = fixture();
            packages[0]["dependencies"]
                .as_array_mut()
                .unwrap()
                .remove(index);
            assert!(verify(&packages).is_err());
        }
        let mut packages = fixture();
        packages[0]["dependencies"][0]["name"] = json!("unreviewed");
        assert!(verify(&packages).is_err());
    }

    #[test]
    fn rejects_inverse_edges_even_when_renamed_and_duplicate_app_harness() {
        for kind in [Value::Null, json!("dev"), json!("build")] {
            let mut packages = fixture();
            packages[1]["dependencies"] = json!([{
                "name": "automexia-renderer-benchmarks", "rename": "innocent_alias", "kind": kind
            }]);
            assert!(verify(&packages).is_err());
        }
        let mut packages = fixture();
        packages[1]["targets"] = fixture()[0]["targets"].clone();
        assert!(verify(&packages).is_err());
    }

    #[test]
    fn actual_workspace_has_the_isolated_boundary() {
        let metadata = crate::metadata().unwrap();
        verify(metadata["packages"].as_array().unwrap()).unwrap();
    }

    #[test]
    fn rejects_backend_drift_and_missing_platform_scope() {
        for (field, value) in [
            ("target", json!("cfg(windows)")),
            ("target", Value::Null),
            ("features", json!([])),
            ("features", json!(["native-gui-test-hooks"])),
        ] {
            let mut packages = fixture();
            packages[0]["dependencies"][5][field] = value;
            assert!(verify(&packages).is_err());
        }
        for features in [
            json!({}),
            json!({"wgpu": []}),
            json!({"wgpu": ["rio-backend/wgpu"], "hidden": []}),
        ] {
            let mut packages = fixture();
            packages[0]["features"] = features;
            assert!(verify(&packages).is_err());
        }
    }
}
