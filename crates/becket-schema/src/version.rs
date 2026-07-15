//! Current artifact schema version (SemVer, independent from CLI version).

/// Public artifact schema version embedded in every `.becket/*.json` file.
///
/// 1.1.0: additive `resolution` field on dependency edges (defaulted for
/// older documents), differentiated per-edge confidence tiers.
pub const SCHEMA_VERSION: &str = "1.1.0";
