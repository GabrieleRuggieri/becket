//! Tree-sitter based parsing and per-file extraction results.

mod http_routes;
mod tree_sitter;

pub use http_routes::ParsedHttpRoute;
pub use tree_sitter::{
    FileParseResult, ParsedCall, ParsedEntrypoint, ParsedImport, ParsedInheritance,
    TreeSitterParser,
};
