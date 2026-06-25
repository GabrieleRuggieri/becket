//! Tree-sitter based parsing and per-file extraction results.

mod http_clients;
mod http_routes;
mod registry;
mod service_contracts;
mod tree_sitter;

pub use http_clients::ParsedHttpClient;
pub use http_routes::ParsedHttpRoute;
pub use registry::{GrammarRegistry, GrammarSpec};
pub use service_contracts::{
    scan_service_contracts, ParsedGrpcClient, ParsedGrpcServer, ParsedQueueEndpoint, QueueRole,
    ServiceContractScan,
};
pub use tree_sitter::{
    FileParseResult, ParsedCall, ParsedEntrypoint, ParsedImport, ParsedInheritance,
    TreeSitterParser,
};
