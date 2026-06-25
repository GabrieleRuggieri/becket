//! Cross-repo edge resolution across workspace members.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use becket_schema::artifacts::{EntrypointsArtifact, SymbolsArtifact};
use becket_schema::edge::{BoundaryKind, EdgeType};
use becket_schema::workspace::{
    CrossRepoArtifact, CrossRepoEdgeRecord, GrpcContract, HttpContract, QueueContract,
    SharedLibContract, WorkspaceManifest,
};

use crate::ids::stable_cross_repo_edge_id;
use crate::parse::{
    ParsedGrpcClient, ParsedGrpcServer, ParsedHttpClient, ParsedImport, ParsedQueueEndpoint,
    QueueRole,
};

/// Indexed data for one workspace member repository.
#[derive(Debug, Clone)]
pub struct RepoIndex {
    /// Workspace repo name.
    pub name: String,
    /// Absolute path to repository root.
    pub root: std::path::PathBuf,
    /// Indexed symbols.
    pub symbols: Vec<becket_schema::artifacts::SymbolRecord>,
    /// HTTP entrypoints (server routes).
    pub http_entrypoints: Vec<HttpRouteEntry>,
    /// Outbound HTTP client calls.
    pub http_clients: Vec<ParsedHttpClient>,
    /// gRPC client call sites.
    pub grpc_clients: Vec<ParsedGrpcClient>,
    /// gRPC server registrations.
    pub grpc_servers: Vec<ParsedGrpcServer>,
    /// Queue producers and consumers.
    pub queue_endpoints: Vec<ParsedQueueEndpoint>,
    /// Import edges (for shared-lib matching).
    pub imports: Vec<ParsedImport>,
}

/// A normalized HTTP server route entrypoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HttpRouteEntry {
    /// HTTP method.
    pub method: String,
    /// Route path.
    pub path: String,
    /// Handler symbol id.
    pub symbol_id: String,
}

/// Resolves cross-repo edges from workspace members and optional contracts.
pub struct CrossRepoLinker;

impl CrossRepoLinker {
    /// Links workspace repos into cross-repo edges.
    pub fn link(manifest: &WorkspaceManifest, repos: &[RepoIndex]) -> CrossRepoArtifact {
        let mut edges = Vec::new();
        let mut seen = HashSet::new();

        Self::link_http_auto(&mut edges, &mut seen, repos);
        Self::link_http_contracts(&mut edges, &mut seen, manifest, repos);
        Self::link_grpc_auto(&mut edges, &mut seen, repos);
        Self::link_grpc_contracts(&mut edges, &mut seen, manifest, repos);
        Self::link_queue_auto(&mut edges, &mut seen, repos);
        Self::link_queue_contracts(&mut edges, &mut seen, manifest, repos);
        Self::link_shared_lib_auto(&mut edges, &mut seen, manifest, repos);
        Self::link_shared_lib_contracts(
            &mut edges,
            &mut seen,
            &manifest.contracts.shared_lib,
            repos,
        );

        edges.sort_by(|a, b| {
            (&a.src_repo, &a.src_symbol_id, &a.dst_repo, &a.dst_symbol_id).cmp(&(
                &b.src_repo,
                &b.src_symbol_id,
                &b.dst_repo,
                &b.dst_symbol_id,
            ))
        });

        CrossRepoArtifact {
            schema_version: manifest.schema_version.clone(),
            workspace: manifest.name.clone(),
            edges,
        }
    }

    fn link_http_auto(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        repos: &[RepoIndex],
    ) {
        let routes_by_repo: HashMap<&str, Vec<&HttpRouteEntry>> = repos
            .iter()
            .map(|repo| (repo.name.as_str(), repo.http_entrypoints.iter().collect()))
            .collect();

        for client_repo in repos {
            for client in &client_repo.http_clients {
                for server_repo in repos {
                    if server_repo.name == client_repo.name {
                        continue;
                    }
                    let Some(routes) = routes_by_repo.get(server_repo.name.as_str()) else {
                        continue;
                    };
                    for route in routes {
                        if !route_matches(client, route) {
                            continue;
                        }
                        let edge = CrossRepoEdgeRecord {
                            id: stable_cross_repo_edge_id(
                                &client_repo.name,
                                &client.caller_symbol_id,
                                &server_repo.name,
                                &route.symbol_id,
                                "http",
                            ),
                            src_repo: client_repo.name.clone(),
                            src_symbol_id: client.caller_symbol_id.clone(),
                            dst_repo: server_repo.name.clone(),
                            dst_symbol_id: route.symbol_id.clone(),
                            edge_type: EdgeType::Http,
                            boundary: BoundaryKind::Network,
                            confidence: 0.85,
                            label: Some(format!("{} {}", client.method, client.path)),
                        };
                        if seen.insert(edge.id.clone()) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }
    }

    fn link_http_contracts(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        manifest: &WorkspaceManifest,
        repos: &[RepoIndex],
    ) {
        for contract in &manifest.contracts.http {
            let Some(edge) = Self::edge_from_http_contract(contract, repos) else {
                continue;
            };
            if seen.insert(edge.id.clone()) {
                edges.push(edge);
            }
        }
    }

    fn edge_from_http_contract(
        contract: &HttpContract,
        repos: &[RepoIndex],
    ) -> Option<CrossRepoEdgeRecord> {
        let client_repo = repos.iter().find(|r| r.name == contract.client_repo)?;
        let server_repo = repos.iter().find(|r| r.name == contract.server_repo)?;
        let label = format!(
            "{} {}",
            contract.method.to_uppercase(),
            normalize_route_path(&contract.path)
        );
        let route = server_repo
            .http_entrypoints
            .iter()
            .find(|route| route.label_matches(&label))?;
        let client: ParsedHttpClient = client_repo
            .http_clients
            .iter()
            .find(|client| client.label_matches(&label))
            .cloned()
            .or_else(|| {
                client_repo.symbols.first().map(|symbol| ParsedHttpClient {
                    file_path: symbol.file_path.clone(),
                    caller_symbol_id: symbol.id.clone(),
                    method: contract.method.to_uppercase(),
                    path: normalize_route_path(&contract.path),
                })
            })?;

        Some(CrossRepoEdgeRecord {
            id: stable_cross_repo_edge_id(
                &client_repo.name,
                &client.caller_symbol_id,
                &server_repo.name,
                &route.symbol_id,
                "http",
            ),
            src_repo: client_repo.name.clone(),
            src_symbol_id: client.caller_symbol_id.clone(),
            dst_repo: server_repo.name.clone(),
            dst_symbol_id: route.symbol_id.clone(),
            edge_type: EdgeType::Http,
            boundary: BoundaryKind::Network,
            confidence: 1.0,
            label: Some(label),
        })
    }

    fn link_grpc_auto(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        repos: &[RepoIndex],
    ) {
        for client_repo in repos {
            for client in &client_repo.grpc_clients {
                for server_repo in repos {
                    if server_repo.name == client_repo.name {
                        continue;
                    }
                    for server in &server_repo.grpc_servers {
                        if client.service_name != server.service_name {
                            continue;
                        }
                        let edge = CrossRepoEdgeRecord {
                            id: stable_cross_repo_edge_id(
                                &client_repo.name,
                                &client.caller_symbol_id,
                                &server_repo.name,
                                &server.symbol_id,
                                "grpc",
                            ),
                            src_repo: client_repo.name.clone(),
                            src_symbol_id: client.caller_symbol_id.clone(),
                            dst_repo: server_repo.name.clone(),
                            dst_symbol_id: server.symbol_id.clone(),
                            edge_type: EdgeType::Grpc,
                            boundary: BoundaryKind::Network,
                            confidence: 0.85,
                            label: Some(client.service_name.clone()),
                        };
                        if seen.insert(edge.id.clone()) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }
    }

    fn link_grpc_contracts(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        manifest: &WorkspaceManifest,
        repos: &[RepoIndex],
    ) {
        for contract in &manifest.contracts.grpc {
            let Some(edge) = Self::edge_from_grpc_contract(contract, repos) else {
                continue;
            };
            if seen.insert(edge.id.clone()) {
                edges.push(edge);
            }
        }
    }

    fn edge_from_grpc_contract(
        contract: &GrpcContract,
        repos: &[RepoIndex],
    ) -> Option<CrossRepoEdgeRecord> {
        let client_repo = repos.iter().find(|r| r.name == contract.client_repo)?;
        let server_repo = repos.iter().find(|r| r.name == contract.server_repo)?;
        let service = normalize_service_name(&contract.service);
        let server = server_repo
            .grpc_servers
            .iter()
            .find(|server| server.service_name == service)?;
        let client = client_repo
            .grpc_clients
            .iter()
            .find(|client| client.service_name == service)
            .cloned()
            .or_else(|| {
                client_repo.symbols.first().map(|symbol| ParsedGrpcClient {
                    file_path: symbol.file_path.clone(),
                    caller_symbol_id: symbol.id.clone(),
                    service_name: service.clone(),
                })
            })?;

        Some(CrossRepoEdgeRecord {
            id: stable_cross_repo_edge_id(
                &client_repo.name,
                &client.caller_symbol_id,
                &server_repo.name,
                &server.symbol_id,
                "grpc",
            ),
            src_repo: client_repo.name.clone(),
            src_symbol_id: client.caller_symbol_id.clone(),
            dst_repo: server_repo.name.clone(),
            dst_symbol_id: server.symbol_id.clone(),
            edge_type: EdgeType::Grpc,
            boundary: BoundaryKind::Network,
            confidence: 1.0,
            label: Some(service),
        })
    }

    fn link_queue_auto(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        repos: &[RepoIndex],
    ) {
        for producer_repo in repos {
            for producer in producer_repo
                .queue_endpoints
                .iter()
                .filter(|endpoint| endpoint.role == QueueRole::Producer)
            {
                for consumer_repo in repos {
                    if consumer_repo.name == producer_repo.name {
                        continue;
                    }
                    for consumer in consumer_repo
                        .queue_endpoints
                        .iter()
                        .filter(|endpoint| endpoint.role == QueueRole::Consumer)
                    {
                        if producer.topic != consumer.topic {
                            continue;
                        }
                        let edge = CrossRepoEdgeRecord {
                            id: stable_cross_repo_edge_id(
                                &producer_repo.name,
                                &producer.symbol_id,
                                &consumer_repo.name,
                                &consumer.symbol_id,
                                "queue",
                            ),
                            src_repo: producer_repo.name.clone(),
                            src_symbol_id: producer.symbol_id.clone(),
                            dst_repo: consumer_repo.name.clone(),
                            dst_symbol_id: consumer.symbol_id.clone(),
                            edge_type: EdgeType::Queue,
                            boundary: BoundaryKind::Queue,
                            confidence: 0.85,
                            label: Some(producer.topic.clone()),
                        };
                        if seen.insert(edge.id.clone()) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }
    }

    fn link_queue_contracts(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        manifest: &WorkspaceManifest,
        repos: &[RepoIndex],
    ) {
        for contract in &manifest.contracts.queue {
            let Some(edge) = Self::edge_from_queue_contract(contract, repos) else {
                continue;
            };
            if seen.insert(edge.id.clone()) {
                edges.push(edge);
            }
        }
    }

    fn edge_from_queue_contract(
        contract: &QueueContract,
        repos: &[RepoIndex],
    ) -> Option<CrossRepoEdgeRecord> {
        let producer_repo = repos.iter().find(|r| r.name == contract.producer_repo)?;
        let consumer_repo = repos.iter().find(|r| r.name == contract.consumer_repo)?;
        let producer = producer_repo
            .queue_endpoints
            .iter()
            .find(|endpoint| {
                endpoint.role == QueueRole::Producer && endpoint.topic == contract.topic
            })
            .cloned()
            .or_else(|| {
                producer_repo
                    .symbols
                    .first()
                    .map(|symbol| ParsedQueueEndpoint {
                        file_path: symbol.file_path.clone(),
                        symbol_id: symbol.id.clone(),
                        topic: contract.topic.clone(),
                        role: QueueRole::Producer,
                    })
            })?;
        let consumer = consumer_repo
            .queue_endpoints
            .iter()
            .find(|endpoint| {
                endpoint.role == QueueRole::Consumer && endpoint.topic == contract.topic
            })
            .cloned()
            .or_else(|| {
                consumer_repo
                    .symbols
                    .first()
                    .map(|symbol| ParsedQueueEndpoint {
                        file_path: symbol.file_path.clone(),
                        symbol_id: symbol.id.clone(),
                        topic: contract.topic.clone(),
                        role: QueueRole::Consumer,
                    })
            })?;

        Some(CrossRepoEdgeRecord {
            id: stable_cross_repo_edge_id(
                &producer_repo.name,
                &producer.symbol_id,
                &consumer_repo.name,
                &consumer.symbol_id,
                "queue",
            ),
            src_repo: producer_repo.name.clone(),
            src_symbol_id: producer.symbol_id.clone(),
            dst_repo: consumer_repo.name.clone(),
            dst_symbol_id: consumer.symbol_id.clone(),
            edge_type: EdgeType::Queue,
            boundary: BoundaryKind::Queue,
            confidence: 1.0,
            label: Some(contract.topic.clone()),
        })
    }

    fn link_shared_lib_auto(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        manifest: &WorkspaceManifest,
        repos: &[RepoIndex],
    ) {
        let packages: HashSet<&str> = manifest
            .contracts
            .shared_lib
            .iter()
            .map(|contract| contract.package.as_str())
            .collect();

        for package in packages {
            let participants: Vec<&RepoIndex> = repos
                .iter()
                .filter(|repo| {
                    repo.imports
                        .iter()
                        .any(|import| import.imported_name == package)
                        || repo.symbols.iter().any(|symbol| symbol.name == package)
                })
                .collect();

            for client_repo in &participants {
                if !client_repo
                    .imports
                    .iter()
                    .any(|import| import.imported_name == package)
                {
                    continue;
                }
                for server_repo in &participants {
                    if client_repo.name == server_repo.name {
                        continue;
                    }
                    let Some(target) = server_repo
                        .symbols
                        .iter()
                        .find(|symbol| symbol.name == package)
                    else {
                        continue;
                    };
                    let Some(client_symbol) = client_repo
                        .symbols
                        .iter()
                        .find(|symbol| {
                            client_repo.imports.iter().any(|import| {
                                import.file_path == symbol.file_path
                                    && import.imported_name == package
                            })
                        })
                        .or_else(|| client_repo.symbols.first())
                    else {
                        continue;
                    };

                    let edge = CrossRepoEdgeRecord {
                        id: stable_cross_repo_edge_id(
                            &client_repo.name,
                            &client_symbol.id,
                            &server_repo.name,
                            &target.id,
                            "shared_lib",
                        ),
                        src_repo: client_repo.name.clone(),
                        src_symbol_id: client_symbol.id.clone(),
                        dst_repo: server_repo.name.clone(),
                        dst_symbol_id: target.id.clone(),
                        edge_type: EdgeType::Imports,
                        boundary: BoundaryKind::SharedLib,
                        confidence: 0.9,
                        label: Some(package.to_string()),
                    };
                    if seen.insert(edge.id.clone()) {
                        edges.push(edge);
                    }
                }
            }
        }
    }

    fn link_shared_lib_contracts(
        edges: &mut Vec<CrossRepoEdgeRecord>,
        seen: &mut HashSet<String>,
        contracts: &[SharedLibContract],
        repos: &[RepoIndex],
    ) {
        for contract in contracts {
            let participants: Vec<&RepoIndex> = repos
                .iter()
                .filter(|repo| contract.repos.iter().any(|name| name == &repo.name))
                .collect();

            for client_repo in &participants {
                for server_repo in &participants {
                    if client_repo.name == server_repo.name {
                        continue;
                    }
                    let Some(target) = server_repo
                        .symbols
                        .iter()
                        .find(|symbol| symbol.name == contract.package)
                    else {
                        continue;
                    };
                    let Some(client_symbol) = client_repo.symbols.first() else {
                        continue;
                    };
                    let edge = CrossRepoEdgeRecord {
                        id: stable_cross_repo_edge_id(
                            &client_repo.name,
                            &client_symbol.id,
                            &server_repo.name,
                            &target.id,
                            "shared_lib",
                        ),
                        src_repo: client_repo.name.clone(),
                        src_symbol_id: client_symbol.id.clone(),
                        dst_repo: server_repo.name.clone(),
                        dst_symbol_id: target.id.clone(),
                        edge_type: EdgeType::Imports,
                        boundary: BoundaryKind::SharedLib,
                        confidence: 1.0,
                        label: Some(contract.package.clone()),
                    };
                    if seen.insert(edge.id.clone()) {
                        edges.push(edge);
                    }
                }
            }
        }
    }
}

impl HttpRouteEntry {
    fn label_matches(&self, label: &str) -> bool {
        format!("{} {}", self.method, self.path).eq_ignore_ascii_case(label)
    }
}

impl ParsedHttpClient {
    fn label_matches(&self, label: &str) -> bool {
        format!("{} {}", self.method, self.path).eq_ignore_ascii_case(label)
    }
}

fn route_matches(client: &ParsedHttpClient, route: &HttpRouteEntry) -> bool {
    client.method.eq_ignore_ascii_case(&route.method)
        && normalize_route_path(&client.path) == normalize_route_path(&route.path)
}

fn normalize_route_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn normalize_service_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_servicer = trimmed.strip_suffix("Servicer").unwrap_or(trimmed);
    let without_impl = without_servicer
        .strip_suffix("Impl")
        .unwrap_or(without_servicer);
    if without_impl.ends_with("Service") {
        without_impl.to_string()
    } else {
        format!("{without_impl}Service")
    }
}

/// Loads repo index data from `.becket/` artifacts and optional HTTP client scan cache.
pub fn load_repo_index(
    name: &str,
    root: &Path,
    http_clients: Vec<ParsedHttpClient>,
    grpc_clients: Vec<ParsedGrpcClient>,
    grpc_servers: Vec<ParsedGrpcServer>,
    queue_endpoints: Vec<ParsedQueueEndpoint>,
    imports: Vec<ParsedImport>,
) -> Result<RepoIndex, crate::error::CoreError> {
    let paths = becket_store::BecketPaths::new(root);
    let symbols_json = std::fs::read_to_string(paths.artifact("symbols"))?;
    let entrypoints_json = std::fs::read_to_string(paths.artifact("entrypoints"))?;

    let symbols_artifact: SymbolsArtifact = serde_json::from_str(&symbols_json)
        .map_err(|error| crate::error::CoreError::InvalidRepository(error.to_string()))?;
    let entrypoints_artifact: EntrypointsArtifact = serde_json::from_str(&entrypoints_json)
        .map_err(|error| crate::error::CoreError::InvalidRepository(error.to_string()))?;

    let http_entrypoints = entrypoints_to_routes(&symbols_artifact.symbols, &entrypoints_artifact);

    Ok(RepoIndex {
        name: name.to_string(),
        root: root.to_path_buf(),
        symbols: symbols_artifact.symbols,
        http_entrypoints,
        http_clients,
        grpc_clients,
        grpc_servers,
        queue_endpoints,
        imports,
    })
}

fn entrypoints_to_routes(
    symbols: &[becket_schema::artifacts::SymbolRecord],
    entrypoints: &EntrypointsArtifact,
) -> Vec<HttpRouteEntry> {
    let mut routes = Vec::new();
    for entry in &entrypoints.entrypoints {
        if entry.kind != becket_schema::symbol::EntrypointKind::Http {
            continue;
        }
        let Some(label) = entry.label.as_deref() else {
            continue;
        };
        let Some((method, path)) = parse_route_label(label) else {
            continue;
        };
        if symbols.iter().any(|symbol| symbol.id == entry.symbol_id) {
            routes.push(HttpRouteEntry {
                method,
                path,
                symbol_id: entry.symbol_id.clone(),
            });
        }
    }
    routes
}

fn parse_route_label(label: &str) -> Option<(String, String)> {
    let mut parts = label.split_whitespace();
    let method = parts.next()?.to_uppercase();
    let path = parts.next()?;
    Some((method, normalize_route_path(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_matching_is_case_insensitive() {
        let client = ParsedHttpClient {
            file_path: "src/client.ts".into(),
            caller_symbol_id: "sym_client".into(),
            method: "get".into(),
            path: "/users".into(),
        };
        let route = HttpRouteEntry {
            method: "GET".into(),
            path: "/users".into(),
            symbol_id: "sym_server".into(),
        };
        assert!(route_matches(&client, &route));
    }
}
