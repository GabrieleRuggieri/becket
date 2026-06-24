//! Tree-sitter based parsing and per-file extraction results.

mod http_clients;
mod http_routes;
mod registry;
mod tree_sitter;

pub use http_clients::ParsedHttpClient;
pub use http_routes::ParsedHttpRoute;
pub use registry::{GrammarRegistry, GrammarSpec};
pub use tree_sitter::{
    FileParseResult, ParsedCall, ParsedEntrypoint, ParsedImport, ParsedInheritance,
    TreeSitterParser,
};
