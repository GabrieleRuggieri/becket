//! Multi-repo workspace manifest loading and cross-repo linking.

mod linker;

pub use linker::{load_repo_index, CrossRepoLinker, RepoIndex};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use becket_schema::workspace::{CrossRepoArtifact, WorkspaceManifest};
use serde::Serialize;
use tracing::info;

use crate::build::{BuildOptions, BuildPipeline, BuildReport};
use crate::error::CoreError;
use crate::parse::{
    scan_service_contracts, FileParseResult, ParsedGrpcClient, ParsedGrpcServer, ParsedHttpClient,
    ParsedImport, ParsedQueueEndpoint, TreeSitterParser,
};
use crate::walker::FileWalker;

/// Default workspace manifest file name at the workspace root.
pub const WORKSPACE_MANIFEST_FILE: &str = "becket.workspace.toml";

/// Resolved paths for workspace-level `.becket/` output.
#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    /// Workspace root directory.
    pub root: PathBuf,
    /// Workspace-level `.becket/` directory.
    pub output_dir: PathBuf,
    /// Path to `becket.workspace.toml`.
    pub manifest_path: PathBuf,
}

impl WorkspacePaths {
    /// Creates path helpers for a workspace root.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            manifest_path: root.join(WORKSPACE_MANIFEST_FILE),
            output_dir: root.join(".becket"),
            root,
        }
    }

    /// Returns the path for a workspace-level JSON artifact.
    pub fn artifact(&self, name: &str) -> PathBuf {
        self.output_dir.join(format!("{name}.json"))
    }
}

/// Summary counters emitted after a successful workspace build.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceBuildReport {
    /// Workspace name from manifest.
    pub workspace: String,
    /// Per-repo build reports keyed by repo name.
    pub repos: Vec<RepoBuildSummary>,
    /// Cross-repo edges discovered.
    pub cross_repo_edges: usize,
    /// Path to workspace `.becket/` output.
    pub output_dir: String,
}

/// Build summary for one workspace member.
#[derive(Debug, Clone, Serialize)]
pub struct RepoBuildSummary {
    /// Repo name.
    pub name: String,
    /// Repo path relative to workspace root.
    pub path: String,
    /// Underlying single-repo build report.
    pub report: BuildReport,
}

/// Multi-repo workspace build pipeline.
pub struct WorkspacePipeline {
    paths: WorkspacePaths,
    options: BuildOptions,
}

impl WorkspacePipeline {
    /// Creates a workspace pipeline rooted at `root`.
    pub fn new(root: impl AsRef<Path>, options: BuildOptions) -> Self {
        Self {
            paths: WorkspacePaths::new(root),
            options,
        }
    }

    /// Runs build for every member repo and links cross-repo edges.
    pub fn run(&self) -> Result<WorkspaceBuildReport, CoreError> {
        let manifest = load_workspace_manifest(&self.paths.manifest_path)?;
        fs::create_dir_all(&self.paths.output_dir)?;

        let mut repo_reports = Vec::new();
        let mut repo_indexes = Vec::new();

        for member in &manifest.repos {
            let repo_root = self.paths.root.join(&member.path);
            if !repo_root.is_dir() {
                return Err(CoreError::InvalidRepository(format!(
                    "workspace repo '{}' path not found: {}",
                    member.name,
                    repo_root.display()
                )));
            }

            info!(repo = %member.name, path = %member.path, "building workspace member");
            let report = BuildPipeline::new(&repo_root, self.options.clone()).run()?;
            let (http_clients, grpc_clients, grpc_servers, queue_endpoints, imports) =
                collect_cross_repo_signals(&repo_root)?;
            let index = load_repo_index(
                &member.name,
                &repo_root,
                http_clients,
                grpc_clients,
                grpc_servers,
                queue_endpoints,
                imports,
            )?;
            repo_indexes.push(index);
            repo_reports.push(RepoBuildSummary {
                name: member.name.clone(),
                path: member.path.clone(),
                report,
            });
        }

        let cross_repo = CrossRepoLinker::link(&manifest, &repo_indexes);
        write_cross_repo_artifact(&self.paths, &cross_repo)?;

        Ok(WorkspaceBuildReport {
            workspace: manifest.name.clone(),
            repos: repo_reports,
            cross_repo_edges: cross_repo.edges.len(),
            output_dir: self.paths.output_dir.display().to_string(),
        })
    }
}

/// Loads `becket.workspace.toml` from disk.
pub fn load_workspace_manifest(path: &Path) -> Result<WorkspaceManifest, CoreError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        CoreError::InvalidRepository(format!("read {}: {error}", path.display()))
    })?;
    toml::from_str(&raw)
        .map_err(|error| CoreError::InvalidRepository(format!("parse workspace manifest: {error}")))
}

/// Discovers a workspace manifest by walking up from `start`.
pub fn discover_workspace_manifest(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;
    loop {
        let candidate = current.join(WORKSPACE_MANIFEST_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn write_cross_repo_artifact(
    paths: &WorkspacePaths,
    artifact: &CrossRepoArtifact,
) -> Result<(), CoreError> {
    let json = serde_json::to_string_pretty(artifact)
        .map_err(|error| CoreError::InvalidRepository(error.to_string()))?;
    fs::write(paths.artifact("cross_repo"), json)?;
    Ok(())
}

/// Signals collected from source for cross-repo linking.
pub type CrossRepoSignals = (
    Vec<ParsedHttpClient>,
    Vec<ParsedGrpcClient>,
    Vec<ParsedGrpcServer>,
    Vec<ParsedQueueEndpoint>,
    Vec<ParsedImport>,
);

fn collect_cross_repo_signals(repo_root: &Path) -> Result<CrossRepoSignals, CoreError> {
    let walker = FileWalker::new(repo_root);
    let discovered = walker.discover()?;
    let mut parse_cache: HashMap<String, FileParseResult> = HashMap::new();

    for file in &discovered {
        let parsed =
            TreeSitterParser::parse_file(&file.relative_path, file.language, &file.absolute_path)?;
        parse_cache.insert(file.relative_path.clone(), parsed);
    }

    let mut http_clients = Vec::new();
    let mut grpc_clients = Vec::new();
    let mut grpc_servers = Vec::new();
    let mut queue_endpoints = Vec::new();
    let mut imports = Vec::new();

    for file in &discovered {
        let Some(parsed) = parse_cache.get(&file.relative_path) else {
            continue;
        };
        http_clients.extend(parsed.http_clients.iter().cloned());
        imports.extend(parsed.imports.iter().cloned());

        let source = std::fs::read_to_string(&file.absolute_path)?;
        let scan = scan_service_contracts(&file.relative_path, &source, &parsed.symbols);
        grpc_clients.extend(scan.grpc_clients);
        grpc_servers.extend(scan.grpc_servers);
        queue_endpoints.extend(scan.queue_endpoints);
    }

    Ok((
        http_clients,
        grpc_clients,
        grpc_servers,
        queue_endpoints,
        imports,
    ))
}

/// Returns true when `becket.workspace.toml` exists at the given root.
pub fn is_workspace_root(root: &Path) -> bool {
    root.join(WORKSPACE_MANIFEST_FILE).is_file()
}

/// Loads an existing cross-repo artifact from a workspace `.becket/` directory.
pub fn load_cross_repo_artifact(root: &Path) -> Result<CrossRepoArtifact, CoreError> {
    let paths = WorkspacePaths::new(root);
    let json = fs::read_to_string(paths.artifact("cross_repo")).map_err(|error| {
        CoreError::InvalidRepository(format!(
            "cross_repo artifact missing at {}: {error}",
            paths.artifact("cross_repo").display()
        ))
    })?;
    serde_json::from_str(&json).map_err(|error| CoreError::InvalidRepository(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let manifest: WorkspaceManifest = toml::from_str(
            r#"
schema_version = "1.0.0"
name = "demo"

[[repos]]
name = "api"
path = "services/api"
"#,
        )
        .unwrap();
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.repos.len(), 1);
    }
}
