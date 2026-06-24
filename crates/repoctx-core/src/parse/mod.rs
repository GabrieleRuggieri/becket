//! Tree-sitter based parsing and per-file extraction results.

mod tree_sitter;

pub use tree_sitter::{FileParseResult, ParsedCall, ParsedEntrypoint, ParsedImport, TreeSitterParser};
