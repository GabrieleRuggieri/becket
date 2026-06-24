//! JSON Schema contract tests for committed `schemas/` files.

use std::fs;
use std::path::PathBuf;

use repoctx_schema::{pretty_schema_for, validate_artifact_json, ARTIFACT_NAMES};

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

#[test]
fn committed_schemas_match_generated() {
    for name in ARTIFACT_NAMES {
        let path = schemas_dir().join(format!("{name}.schema.json"));
        let committed = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let generated = pretty_schema_for(name).expect("generate schema");
        assert_eq!(
            committed.trim(),
            generated.trim(),
            "schema drift for {name}: run `cargo test -p repoctx-schema write_schemas -- --ignored --nocapture`"
        );
    }
}

#[test]
#[ignore = "run manually to refresh schemas/ after schema changes"]
fn write_schemas() {
    fs::create_dir_all(schemas_dir()).expect("create schemas dir");
    for name in ARTIFACT_NAMES {
        let path = schemas_dir().join(format!("{name}.schema.json"));
        let content = pretty_schema_for(name).expect("generate schema");
        fs::write(&path, format!("{content}\n")).expect("write schema");
        println!("wrote {}", path.display());
    }
}

#[test]
fn minimal_artifacts_validate() {
    let samples = [
        ("symbols", r#"{"schemaVersion":"1.0.0","symbols":[]}"#),
        ("dependencies", r#"{"schemaVersion":"1.0.0","edges":[]}"#),
        ("flows", r#"{"schemaVersion":"1.0.0","flows":[]}"#),
        (
            "entrypoints",
            r#"{"schemaVersion":"1.0.0","entrypoints":[]}"#,
        ),
        (
            "architecture",
            r#"{"schemaVersion":"1.0.0","modules":[],"edges":[]}"#,
        ),
        (
            "cross_repo",
            r#"{"schemaVersion":"1.0.0","workspace":"demo","edges":[]}"#,
        ),
    ];

    for (name, json) in samples {
        validate_artifact_json(name, json).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn rejects_invalid_symbols_artifact() {
    let err = validate_artifact_json("symbols", r#"{"symbols":[]}"#).unwrap_err();
    assert!(err.to_string().contains("validation failed"));
}
