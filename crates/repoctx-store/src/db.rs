//! SQLite schema, migrations, and query helpers for the index store.

use std::path::Path;

use repoctx_schema::artifacts::{
    ArchitectureArtifact, DependenciesArtifact, EntrypointsArtifact, FlowRecord, FlowsArtifact,
    SymbolRecord, SymbolsArtifact,
};
use repoctx_schema::edge::{BoundaryKind, EdgeType};
use repoctx_schema::symbol::{EntrypointKind, SymbolKind, Visibility};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::StoreError;

/// User-refined domain: `(flow_id, display_name, members)`.
pub type DomainOverride = (String, String, Vec<(String, String)>);

/// Embedded SQLite index for symbols, edges, flows, and file hashes.
pub struct IndexStore {
    conn: Connection,
}

impl IndexStore {
    /// Opens or creates the index database at `path` and applies migrations.
    ///
    /// # Arguments
    ///
    /// * `path` - Filesystem path to `index.db`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] if SQLite initialization fails.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Applies idempotent schema migrations.
    fn migrate(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS files (
                id          TEXT PRIMARY KEY,
                path        TEXT NOT NULL UNIQUE,
                language    TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                mtime_secs  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id          TEXT PRIMARY KEY,
                kind        TEXT NOT NULL,
                name        TEXT NOT NULL,
                fqn         TEXT NOT NULL,
                file_id     TEXT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                start_line  INTEGER NOT NULL,
                end_line    INTEGER NOT NULL,
                visibility  TEXT NOT NULL,
                module_id   TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_fqn ON symbols(fqn);

            CREATE TABLE IF NOT EXISTS edges (
                id              TEXT PRIMARY KEY,
                src_symbol_id   TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
                dst_symbol_id   TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
                edge_type       TEXT NOT NULL,
                boundary        TEXT,
                confidence      REAL NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_edges_src ON edges(src_symbol_id);
            CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst_symbol_id);

            CREATE TABLE IF NOT EXISTS modules (
                id      TEXT PRIMARY KEY,
                name    TEXT NOT NULL,
                kind    TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS flows (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS flow_steps (
                id              TEXT PRIMARY KEY,
                flow_id         TEXT NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
                step_order      INTEGER NOT NULL,
                symbol_id       TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
                external_system TEXT
            );

            CREATE TABLE IF NOT EXISTS entrypoints (
                id          TEXT PRIMARY KEY,
                symbol_id   TEXT NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
                kind        TEXT NOT NULL,
                label       TEXT
            );

            CREATE TABLE IF NOT EXISTS domains (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                source          TEXT NOT NULL DEFAULT 'deterministic',
                user_confirmed  INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS domain_members (
                domain_id     TEXT NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
                member_kind   TEXT NOT NULL,
                member_value  TEXT NOT NULL,
                PRIMARY KEY (domain_id, member_kind, member_value)
            );

            CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    /// Clears indexed code data while preserving user domain refinements.
    pub fn clear_all(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "
            DELETE FROM flow_steps;
            DELETE FROM flows;
            DELETE FROM entrypoints;
            DELETE FROM edges;
            DELETE FROM symbols;
            DELETE FROM modules;
            DELETE FROM files;
            ",
        )?;
        Ok(())
    }

    /// Clears user domain overrides (for tests or full reset).
    pub fn clear_domains(&self) -> Result<(), StoreError> {
        self.conn
            .execute_batch("DELETE FROM domain_members; DELETE FROM domains;")?;
        Ok(())
    }

    /// Upserts a tracked source file record.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn upsert_file(
        &self,
        id: &str,
        path: &str,
        language: &str,
        content_hash: &str,
        mtime_secs: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO files (id, path, language, content_hash, mtime_secs)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET
                language = excluded.language,
                content_hash = excluded.content_hash,
                mtime_secs = excluded.mtime_secs",
            params![id, path, language, content_hash, mtime_secs],
        )?;
        Ok(())
    }

    /// Returns the stored content hash for a file path, if present.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn file_hash(&self, path: &str) -> Result<Option<String>, StoreError> {
        let hash = self
            .conn
            .query_row(
                "SELECT content_hash FROM files WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?;
        Ok(hash)
    }

    /// Returns the stable file id for a repository-relative path.
    pub fn file_id(&self, path: &str) -> Result<Option<String>, StoreError> {
        let id = self
            .conn
            .query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?;
        Ok(id)
    }

    /// Removes all symbols (and cascaded edges) for a file path before re-indexing.
    pub fn delete_symbols_for_path(&self, path: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM symbols WHERE file_id = (SELECT id FROM files WHERE path = ?1)",
            params![path],
        )?;
        Ok(())
    }

    /// Deletes all dependency edges (rebuilt after each parse pass).
    pub fn clear_edges(&self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM edges", [])?;
        Ok(())
    }

    /// Deletes all entrypoints (rebuilt after each parse pass).
    pub fn clear_entrypoints(&self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM entrypoints", [])?;
        Ok(())
    }

    /// Deletes all flows and steps (rebuilt after each parse pass).
    pub fn clear_flows(&self) -> Result<(), StoreError> {
        self.conn
            .execute_batch("DELETE FROM flow_steps; DELETE FROM flows;")?;
        Ok(())
    }

    /// Inserts a flow and its ordered steps.
    pub fn insert_flow(&self, flow: &FlowRecord) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO flows (id, name, description) VALUES (?1, ?2, ?3)",
            params![flow.id, flow.name, flow.description],
        )?;
        for step in &flow.steps {
            self.conn.execute(
                "INSERT OR REPLACE INTO flow_steps (id, flow_id, step_order, symbol_id, external_system)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("{}:{}", flow.id, step.order),
                    flow.id,
                    step.order,
                    step.symbol_id,
                    step.external_system,
                ],
            )?;
        }
        Ok(())
    }

    /// Returns all call edges as source/target symbol id pairs.
    pub fn load_call_edges(&self) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT src_symbol_id, dst_symbol_id FROM edges WHERE edge_type = 'calls'")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    /// Inserts a dependency edge between two symbols.
    pub fn insert_edge(
        &self,
        edge: &repoctx_schema::artifacts::DependencyEdgeRecord,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO edges
             (id, src_symbol_id, dst_symbol_id, edge_type, boundary, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                edge.id,
                edge.src_symbol_id,
                edge.dst_symbol_id,
                edge_type_to_str(edge.edge_type),
                edge.boundary.map(boundary_to_str),
                edge.confidence,
            ],
        )?;
        Ok(())
    }

    /// Inserts a detected entrypoint.
    pub fn insert_entrypoint(
        &self,
        record: &repoctx_schema::artifacts::EntrypointRecord,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO entrypoints (id, symbol_id, kind, label)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                record.id,
                record.symbol_id,
                entrypoint_kind_to_str(record.kind),
                record.label,
            ],
        )?;
        Ok(())
    }

    /// Inserts a symbol row.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn insert_symbol(&self, symbol: &SymbolRecord, file_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO symbols
             (id, kind, name, fqn, file_id, start_line, end_line, visibility, module_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                symbol.id,
                symbol_kind_to_str(symbol.kind),
                symbol.name,
                symbol.fqn,
                file_id,
                symbol.start_line,
                symbol.end_line,
                visibility_to_str(symbol.visibility),
                symbol.module_id,
            ],
        )?;
        Ok(())
    }

    /// Loads all symbols joined with their file paths.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn load_symbols(&self) -> Result<Vec<SymbolRecord>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.kind, s.name, s.fqn, f.path, s.start_line, s.end_line,
                    s.visibility, s.module_id
             FROM symbols s
             JOIN files f ON f.id = s.file_id
             ORDER BY f.path, s.start_line",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SymbolRecord {
                id: row.get(0)?,
                kind: str_to_symbol_kind(row.get::<_, String>(1)?.as_str()),
                name: row.get(2)?,
                fqn: row.get(3)?,
                file_path: row.get(4)?,
                start_line: row.get::<_, i64>(5)? as u32,
                end_line: row.get::<_, i64>(6)? as u32,
                visibility: str_to_visibility(row.get::<_, String>(7)?.as_str()),
                module_id: row.get(8)?,
            })
        })?;
        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(row?);
        }
        Ok(symbols)
    }

    /// Finds symbols whose name or FQN matches the query (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn find_symbols_by_name(&self, query: &str) -> Result<Vec<SymbolRecord>, StoreError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.kind, s.name, s.fqn, f.path, s.start_line, s.end_line,
                    s.visibility, s.module_id
             FROM symbols s
             JOIN files f ON f.id = s.file_id
             WHERE lower(s.name) = lower(?1) OR lower(s.fqn) LIKE lower(?2)
             ORDER BY CASE WHEN lower(s.name) = lower(?1) THEN 0 ELSE 1 END, s.name",
        )?;
        let rows = stmt.query_map(params![query, pattern], |row| {
            Ok(SymbolRecord {
                id: row.get(0)?,
                kind: str_to_symbol_kind(row.get::<_, String>(1)?.as_str()),
                name: row.get(2)?,
                fqn: row.get(3)?,
                file_path: row.get(4)?,
                start_line: row.get::<_, i64>(5)? as u32,
                end_line: row.get::<_, i64>(6)? as u32,
                visibility: str_to_visibility(row.get::<_, String>(7)?.as_str()),
                module_id: row.get(8)?,
            })
        })?;
        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(row?);
        }
        Ok(symbols)
    }

    /// Returns downstream symbol ids reachable within `depth` hops.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn downstream_symbols(
        &self,
        root_symbol_id: &str,
        depth: u32,
    ) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE reach(id, depth) AS (
                SELECT dst_symbol_id, 1 FROM edges WHERE src_symbol_id = ?1
                UNION ALL
                SELECT e.dst_symbol_id, r.depth + 1
                FROM edges e
                JOIN reach r ON e.src_symbol_id = r.id
                WHERE r.depth < ?2
             )
             SELECT DISTINCT id FROM reach",
        )?;
        let rows = stmt.query_map(params![root_symbol_id, depth], |row| row.get(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    /// Finds a flow by exact id or case-insensitive name.
    pub fn find_flow_id(&self, key: &str) -> Result<Option<String>, StoreError> {
        let by_id: Option<String> = self
            .conn
            .query_row("SELECT id FROM flows WHERE id = ?1", params![key], |row| {
                row.get(0)
            })
            .optional()?;
        if by_id.is_some() {
            return Ok(by_id);
        }
        self.conn
            .query_row(
                "SELECT id FROM flows WHERE lower(name) = lower(?1) LIMIT 1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Returns true if another flow already uses `name`.
    pub fn flow_name_taken(&self, name: &str, except_id: Option<&str>) -> Result<bool, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM flows WHERE lower(name) = lower(?1) AND (?2 IS NULL OR id != ?2)",
            params![name, except_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Updates the display name of a flow.
    pub fn update_flow_name(&self, flow_id: &str, name: &str) -> Result<(), StoreError> {
        let updated = self.conn.execute(
            "UPDATE flows SET name = ?1 WHERE id = ?2",
            params![name, flow_id],
        )?;
        if updated == 0 {
            return Err(StoreError::NotFound(format!("flow '{flow_id}'")));
        }
        Ok(())
    }

    /// Inserts or updates a domain record.
    pub fn upsert_domain(
        &self,
        id: &str,
        name: &str,
        source: &str,
        user_confirmed: bool,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO domains (id, name, source, user_confirmed)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                source = excluded.source,
                user_confirmed = excluded.user_confirmed",
            params![id, name, source, i64::from(user_confirmed)],
        )?;
        Ok(())
    }

    /// Syncs deterministic domain rows from auto-discovered flows without overwriting user edits.
    pub fn sync_domains_from_flows(&self, flows: &[FlowRecord]) -> Result<(), StoreError> {
        for flow in flows {
            let confirmed: i64 = self
                .conn
                .query_row(
                    "SELECT user_confirmed FROM domains WHERE id = ?1",
                    params![flow.id],
                    |row| row.get(0),
                )
                .optional()?
                .unwrap_or(0);
            if confirmed == 0 {
                self.upsert_domain(&flow.id, &flow.name, "deterministic", false)?;
            }
        }
        Ok(())
    }

    /// Loads user-refined domains and their members for build-time overrides.
    pub fn load_user_domain_overrides(&self) -> Result<Vec<DomainOverride>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM domains WHERE user_confirmed = 1")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))?;
        let mut overrides = Vec::new();
        for row in rows {
            let (id, name) = row?;
            let members = self.list_domain_members(&id)?;
            overrides.push((id, name, members));
        }
        Ok(overrides)
    }

    /// Adds a path or symbol member to a domain.
    pub fn add_domain_member(
        &self,
        domain_id: &str,
        member_kind: &str,
        member_value: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO domain_members (domain_id, member_kind, member_value)
             VALUES (?1, ?2, ?3)",
            params![domain_id, member_kind, member_value],
        )?;
        Ok(())
    }

    /// Lists members attached to a domain.
    pub fn list_domain_members(
        &self,
        domain_id: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT member_kind, member_value FROM domain_members WHERE domain_id = ?1")?;
        let rows = stmt
            .query_map(params![domain_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Replaces flow steps for an existing flow.
    pub fn replace_flow(&self, flow: &FlowRecord) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO flows (id, name, description) VALUES (?1, ?2, ?3)",
            params![flow.id, flow.name, flow.description],
        )?;
        self.conn.execute(
            "DELETE FROM flow_steps WHERE flow_id = ?1",
            params![flow.id],
        )?;
        for step in &flow.steps {
            self.conn.execute(
                "INSERT INTO flow_steps (id, flow_id, step_order, symbol_id, external_system)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("{}:{}", flow.id, step.order),
                    flow.id,
                    step.order,
                    step.symbol_id,
                    step.external_system,
                ],
            )?;
        }
        Ok(())
    }

    /// Writes all JSON artifacts from the current index state.
    pub fn write_artifacts(&self, paths: &crate::paths::RepoCtxPaths) -> Result<(), StoreError> {
        let writer = crate::artifacts::ArtifactWriter::new(paths.clone());
        let (symbols, dependencies, flows, entrypoints, architecture) = self.export_artifacts()?;
        writer.write_artifact("symbols", &symbols)?;
        writer.write_artifact("dependencies", &dependencies)?;
        writer.write_artifact("flows", &flows)?;
        writer.write_artifact("entrypoints", &entrypoints)?;
        writer.write_artifact("architecture", &architecture)?;
        Ok(())
    }

    /// Finds a flow by exact or partial name match.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn find_flow_by_name(&self, name: &str) -> Result<Option<FlowRecord>, StoreError> {
        let flow_row: Option<(String, String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT id, name, description FROM flows
                 WHERE lower(name) = lower(?1)
                 ORDER BY length(name) ASC LIMIT 1",
                params![name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((flow_id, flow_name, description)) = flow_row else {
            return Ok(None);
        };

        let mut stmt = self.conn.prepare(
            "SELECT step_order, symbol_id, external_system
             FROM flow_steps WHERE flow_id = ?1 ORDER BY step_order",
        )?;
        let steps = stmt
            .query_map(params![flow_id], |row| {
                Ok(repoctx_schema::artifacts::FlowStepRecord {
                    order: row.get::<_, i64>(0)? as u32,
                    symbol_id: row.get(1)?,
                    external_system: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(FlowRecord {
            id: flow_id,
            name: flow_name,
            description,
            steps,
        }))
    }

    /// Builds in-memory artifact snapshots from the current index state.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Database`] on SQLite failure.
    pub fn export_artifacts(
        &self,
    ) -> Result<
        (
            SymbolsArtifact,
            DependenciesArtifact,
            FlowsArtifact,
            EntrypointsArtifact,
            ArchitectureArtifact,
        ),
        StoreError,
    > {
        let symbols = SymbolsArtifact {
            symbols: self.load_symbols()?,
            ..SymbolsArtifact::default()
        };

        let mut edge_stmt = self.conn.prepare(
            "SELECT id, src_symbol_id, dst_symbol_id, edge_type, boundary, confidence
             FROM edges",
        )?;
        let edges = edge_stmt
            .query_map([], |row| {
                Ok(repoctx_schema::artifacts::DependencyEdgeRecord {
                    id: row.get(0)?,
                    src_symbol_id: row.get(1)?,
                    dst_symbol_id: row.get(2)?,
                    edge_type: str_to_edge_type(row.get::<_, String>(3)?.as_str()),
                    boundary: row
                        .get::<_, Option<String>>(4)?
                        .map(|b| str_to_boundary(b.as_str())),
                    confidence: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let dependencies = DependenciesArtifact {
            edges,
            ..DependenciesArtifact::default()
        };

        let mut flow_stmt = self
            .conn
            .prepare("SELECT id, name, description FROM flows")?;
        let flow_ids: Vec<(String, String, Option<String>)> = flow_stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut flows = Vec::new();
        for (flow_id, flow_name, description) in flow_ids {
            let mut step_stmt = self.conn.prepare(
                "SELECT step_order, symbol_id, external_system
                 FROM flow_steps WHERE flow_id = ?1 ORDER BY step_order",
            )?;
            let steps = step_stmt
                .query_map(params![flow_id], |row| {
                    Ok(repoctx_schema::artifacts::FlowStepRecord {
                        order: row.get::<_, i64>(0)? as u32,
                        symbol_id: row.get(1)?,
                        external_system: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            flows.push(FlowRecord {
                id: flow_id,
                name: flow_name,
                description,
                steps,
            });
        }

        let flows = FlowsArtifact {
            flows,
            ..FlowsArtifact::default()
        };

        let mut ep_stmt = self
            .conn
            .prepare("SELECT id, symbol_id, kind, label FROM entrypoints")?;
        let entrypoints = ep_stmt
            .query_map([], |row| {
                Ok(repoctx_schema::artifacts::EntrypointRecord {
                    id: row.get(0)?,
                    symbol_id: row.get(1)?,
                    kind: str_to_entrypoint_kind(row.get::<_, String>(2)?.as_str()),
                    label: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let entrypoints = EntrypointsArtifact {
            entrypoints,
            ..EntrypointsArtifact::default()
        };

        let architecture = ArchitectureArtifact::default();

        Ok((symbols, dependencies, flows, entrypoints, architecture))
    }
}

fn symbol_kind_to_str(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Var => "var",
        SymbolKind::Type => "type",
        SymbolKind::Module => "module",
    }
}

fn str_to_symbol_kind(value: &str) -> SymbolKind {
    match value {
        "class" => SymbolKind::Class,
        "method" => SymbolKind::Method,
        "var" => SymbolKind::Var,
        "type" => SymbolKind::Type,
        "module" => SymbolKind::Module,
        _ => SymbolKind::Function,
    }
}

fn visibility_to_str(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "public",
        Visibility::Internal => "internal",
        Visibility::Private => "private",
    }
}

fn str_to_visibility(value: &str) -> Visibility {
    match value {
        "public" => Visibility::Public,
        "internal" => Visibility::Internal,
        _ => Visibility::Private,
    }
}

fn str_to_edge_type(value: &str) -> EdgeType {
    match value {
        "imports" => EdgeType::Imports,
        "extends" => EdgeType::Extends,
        "implements" => EdgeType::Implements,
        "references" => EdgeType::References,
        "reads" => EdgeType::Reads,
        "writes" => EdgeType::Writes,
        "http" => EdgeType::Http,
        "grpc" => EdgeType::Grpc,
        "queue" => EdgeType::Queue,
        _ => EdgeType::Calls,
    }
}

fn str_to_boundary(value: &str) -> BoundaryKind {
    match value {
        "queue" => BoundaryKind::Queue,
        "shared_lib" => BoundaryKind::SharedLib,
        _ => BoundaryKind::Network,
    }
}

fn str_to_entrypoint_kind(value: &str) -> EntrypointKind {
    match value {
        "cli" => EntrypointKind::Cli,
        "http" => EntrypointKind::Http,
        "cron" => EntrypointKind::Cron,
        "event" => EntrypointKind::Event,
        _ => EntrypointKind::Main,
    }
}

fn entrypoint_kind_to_str(kind: EntrypointKind) -> &'static str {
    match kind {
        EntrypointKind::Cli => "cli",
        EntrypointKind::Http => "http",
        EntrypointKind::Cron => "cron",
        EntrypointKind::Event => "event",
        EntrypointKind::Main => "main",
    }
}

fn edge_type_to_str(edge_type: EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Calls => "calls",
        EdgeType::Imports => "imports",
        EdgeType::Extends => "extends",
        EdgeType::Implements => "implements",
        EdgeType::References => "references",
        EdgeType::Reads => "reads",
        EdgeType::Writes => "writes",
        EdgeType::Http => "http",
        EdgeType::Grpc => "grpc",
        EdgeType::Queue => "queue",
    }
}

fn boundary_to_str(boundary: BoundaryKind) -> &'static str {
    match boundary {
        BoundaryKind::Network => "network",
        BoundaryKind::Queue => "queue",
        BoundaryKind::SharedLib => "shared_lib",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repoctx_schema::artifacts::SymbolRecord;
    use repoctx_schema::symbol::{SymbolKind, Visibility};
    use tempfile::tempdir;

    #[test]
    fn open_creates_schema() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("index.db");
        let store = IndexStore::open(&db_path).unwrap();
        store.clear_all().unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn upsert_and_load_symbol_roundtrip() {
        let dir = tempdir().unwrap();
        let store = IndexStore::open(dir.path().join("index.db")).unwrap();
        store.clear_all().unwrap();
        store
            .upsert_file("f1", "src/main.rs", "rust", "abc", 1)
            .unwrap();
        let symbol = SymbolRecord {
            id: "sym1".into(),
            kind: SymbolKind::Function,
            name: "main".into(),
            fqn: "main".into(),
            file_path: "src/main.rs".into(),
            start_line: 1,
            end_line: 3,
            visibility: Visibility::Public,
            module_id: None,
        };
        store.insert_symbol(&symbol, "f1").unwrap();
        let loaded = store.load_symbols().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "main");
    }
}
