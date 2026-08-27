use std::collections::{HashMap, HashSet, VecDeque};

use super::model::{
    AutomationRecipeDocumentV1, ConnectionModelError, ConnectionModelErrorCode,
    ConnectionProfileDocumentV1, CONNECTION_SCHEMA_VERSION, MAX_DOCUMENT_BYTES,
    MAX_PROFILES, MAX_RECIPES,
};
use super::strict_json::from_json_slice_without_duplicate_keys;
use super::validation::{validate_profile, validate_recipe};

fn error(
    code: ConnectionModelErrorCode,
    field: &'static str,
    detail: &'static str,
) -> ConnectionModelError {
    ConnectionModelError::new(code, field, detail)
}

pub fn parse_profile_document_json(
    bytes: &[u8],
) -> Result<ConnectionProfileDocumentV1, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "document",
            "profile document exceeds the fixed byte ceiling",
        ));
    }
    let document = from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "document",
            "profile document does not match the strict public schema",
        )
    })?;
    validate_profile_document(&document)?;
    Ok(document)
}

pub fn parse_recipe_document_json(
    bytes: &[u8],
) -> Result<AutomationRecipeDocumentV1, ConnectionModelError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "document",
            "recipe document exceeds the fixed byte ceiling",
        ));
    }
    let document = from_json_slice_without_duplicate_keys(bytes).map_err(|_| {
        error(
            ConnectionModelErrorCode::MalformedSchema,
            "document",
            "recipe document does not match the strict public schema",
        )
    })?;
    validate_recipe_document(&document)?;
    Ok(document)
}

pub fn validate_profile_document(
    document: &ConnectionProfileDocumentV1,
) -> Result<(), ConnectionModelError> {
    if document.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "schema_version",
            "unsupported profile document schema",
        ));
    }
    if document.profiles.len() > MAX_PROFILES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "profiles",
            "profile document exceeds the fixed item ceiling",
        ));
    }
    let mut indices = HashMap::with_capacity(document.profiles.len());
    for (index, profile) in document.profiles.iter().enumerate() {
        validate_profile(profile)?;
        if indices.insert(profile.id.as_str(), index).is_some() {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "profiles.id",
                "duplicate profile IDs are forbidden",
            ));
        }
    }

    let mut edges = vec![Vec::new(); document.profiles.len()];
    let mut incoming = vec![0usize; document.profiles.len()];
    for (index, profile) in document.profiles.iter().enumerate() {
        for reference in &profile.jump_profile_references {
            let Some(&dependency) = indices.get(reference.as_str()) else {
                return Err(error(
                    ConnectionModelErrorCode::MissingDependency,
                    "jump_profile_references",
                    "jump profile reference does not exist",
                ));
            };
            edges[index].push(dependency);
            incoming[dependency] = incoming[dependency].saturating_add(1);
        }
    }
    let mut queue = incoming
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut visited = 0usize;
    while let Some(index) = queue.pop_front() {
        visited += 1;
        for &dependency in &edges[index] {
            incoming[dependency] -= 1;
            if incoming[dependency] == 0 {
                queue.push_back(dependency);
            }
        }
    }
    if visited != document.profiles.len() {
        return Err(error(
            ConnectionModelErrorCode::DependencyCycle,
            "jump_profile_references",
            "jump profile graph contains a cycle",
        ));
    }
    Ok(())
}

pub fn validate_recipe_document(
    document: &AutomationRecipeDocumentV1,
) -> Result<(), ConnectionModelError> {
    if document.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err(error(
            ConnectionModelErrorCode::UnsupportedVersion,
            "schema_version",
            "unsupported recipe document schema",
        ));
    }
    if document.recipes.len() > MAX_RECIPES {
        return Err(error(
            ConnectionModelErrorCode::LimitExceeded,
            "recipes",
            "recipe document exceeds the fixed item ceiling",
        ));
    }
    let mut ids = HashSet::with_capacity(document.recipes.len());
    for recipe in &document.recipes {
        validate_recipe(recipe)?;
        if !ids.insert(recipe.id.as_str()) {
            return Err(error(
                ConnectionModelErrorCode::DuplicateId,
                "recipes.id",
                "duplicate recipe IDs are forbidden",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_documents_are_valid_but_future_versions_are_not() {
        assert!(validate_profile_document(&ConnectionProfileDocumentV1 {
            schema_version: 1,
            revision: 0,
            profiles: Vec::new(),
        })
        .is_ok());
        assert!(validate_recipe_document(&AutomationRecipeDocumentV1 {
            schema_version: 2,
            revision: 0,
            recipes: Vec::new(),
        })
        .is_err());
    }
}
