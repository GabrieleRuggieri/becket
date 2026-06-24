//! JSON Schema generation and validation for `.repoctx/*.json` artifacts.

use schemars::schema_for;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::artifacts::{
    ArchitectureArtifact, DependenciesArtifact, EntrypointsArtifact, FlowsArtifact, SymbolsArtifact,
};
use crate::error::SchemaError;
use crate::workspace::CrossRepoArtifact;

/// Artifact base names (without `.json` extension).
pub const ARTIFACT_NAMES: &[&str] = &[
    "symbols",
    "dependencies",
    "flows",
    "entrypoints",
    "architecture",
    "cross_repo",
];

/// Returns the JSON Schema document for a named artifact.
pub fn root_schema_for(artifact: &str) -> Result<Value, SchemaError> {
    let schema = match artifact {
        "symbols" => serde_json::to_value(schema_for!(SymbolsArtifact))?,
        "dependencies" => serde_json::to_value(schema_for!(DependenciesArtifact))?,
        "flows" => serde_json::to_value(schema_for!(FlowsArtifact))?,
        "entrypoints" => serde_json::to_value(schema_for!(EntrypointsArtifact))?,
        "architecture" => serde_json::to_value(schema_for!(ArchitectureArtifact))?,
        "cross_repo" => serde_json::to_value(schema_for!(CrossRepoArtifact))?,
        other => {
            return Err(SchemaError::UnknownArtifact(other.to_string()));
        }
    };
    Ok(schema)
}

/// Validates a JSON document against the schema for `artifact`.
pub fn validate_artifact_json(artifact: &str, json: &str) -> Result<(), SchemaError> {
    let schema = root_schema_for(artifact)?;
    let instance: Value =
        serde_json::from_str(json).map_err(|source| SchemaError::InvalidJson {
            artifact: artifact.to_string(),
            source,
        })?;

    let validator =
        jsonschema::validator_for(&schema).map_err(|error| SchemaError::InvalidSchema {
            artifact: artifact.to_string(),
            message: error.to_string(),
        })?;

    if let Err(error) = validator.validate(&instance) {
        return Err(SchemaError::ValidationFailed {
            artifact: artifact.to_string(),
            messages: vec![error.to_string()],
        });
    }

    Ok(())
}

/// Parses and validates JSON, returning the typed artifact.
pub fn parse_artifact<T>(artifact: &str, json: &str) -> Result<T, SchemaError>
where
    T: DeserializeOwned,
{
    validate_artifact_json(artifact, json)?;
    serde_json::from_str(json).map_err(|source| SchemaError::InvalidJson {
        artifact: artifact.to_string(),
        source,
    })
}

/// Pretty-printed JSON Schema for committed files under `schemas/`.
pub fn pretty_schema_for(artifact: &str) -> Result<String, SchemaError> {
    let value = root_schema_for(artifact)?;
    serde_json::to_string_pretty(&value).map_err(SchemaError::from)
}
