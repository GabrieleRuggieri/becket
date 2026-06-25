//! Read/write `.becket/wiki/*.md` pages.

use std::fs;
use std::path::{Path, PathBuf};

use becket_schema::wiki::{WikiPage, WikiPageMeta};
use becket_store::BecketPaths;

use crate::error::CoreError;

const FRONTMATTER_DELIM: &str = "---";

/// Filesystem access to grounded wiki pages.
pub struct WikiStore {
    wiki_dir: PathBuf,
}

impl WikiStore {
    /// Opens the wiki store under `.becket/wiki/`.
    pub fn new(paths: &BecketPaths) -> Self {
        Self {
            wiki_dir: paths.wiki_dir(),
        }
    }

    /// Ensures the wiki directory exists.
    pub fn ensure_dir(&self) -> Result<(), CoreError> {
        fs::create_dir_all(&self.wiki_dir)?;
        Ok(())
    }

    /// Writes a page to `{id}.md` (id without path separators).
    pub fn write_page(&self, meta: &WikiPageMeta, body: &str) -> Result<PathBuf, CoreError> {
        self.ensure_dir()?;
        let filename = page_filename(&meta.id);
        let path = self.wiki_dir.join(filename);
        let frontmatter = toml::to_string(meta).map_err(|e| CoreError::Wiki(e.to_string()))?;
        let content = format!("{FRONTMATTER_DELIM}\n{frontmatter}{FRONTMATTER_DELIM}\n\n{body}");
        fs::write(&path, content)?;
        Ok(path)
    }

    /// Loads a page by id or filename stem.
    pub fn load_page(&self, key: &str) -> Result<Option<WikiPage>, CoreError> {
        let path = self.resolve_path(key)?;
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path)?;
        Ok(Some(parse_page(&raw)?))
    }

    /// Lists all page ids (excludes `index`).
    pub fn list_page_ids(&self) -> Result<Vec<String>, CoreError> {
        if !self.wiki_dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.wiki_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == "index" {
                continue;
            }
            if let Ok(Some(page)) = self.load_page(stem) {
                ids.push(page.meta.id);
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Writes `index.md` router (same content as overview page).
    pub fn write_index(&self, meta: &WikiPageMeta, body: &str) -> Result<(), CoreError> {
        self.ensure_dir()?;
        let frontmatter = toml::to_string(meta).map_err(|e| CoreError::Wiki(e.to_string()))?;
        let content = format!("{FRONTMATTER_DELIM}\n{frontmatter}{FRONTMATTER_DELIM}\n\n{body}");
        fs::write(self.wiki_dir.join("index.md"), content)?;
        Ok(())
    }

    /// Wiki directory path.
    pub fn wiki_dir(&self) -> &Path {
        &self.wiki_dir
    }

    /// Loads `index.md` router page.
    pub fn load_index(&self) -> Result<Option<WikiPage>, CoreError> {
        let path = self.wiki_dir.join("index.md");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(load_page_from_path(&path)?))
    }

    fn resolve_path(&self, key: &str) -> Result<PathBuf, CoreError> {
        let stem = key.strip_suffix(".md").unwrap_or(key);
        let stem = stem.strip_prefix("wiki_").unwrap_or(stem);
        Ok(self.wiki_dir.join(format!("{stem}.md")))
    }
}

fn page_filename(id: &str) -> String {
    let stem = id.strip_prefix("wiki_").unwrap_or(id);
    format!("{stem}.md")
}

fn parse_page(raw: &str) -> Result<WikiPage, CoreError> {
    let (frontmatter, body) = split_frontmatter(raw)?;
    let meta: WikiPageMeta =
        toml::from_str(frontmatter).map_err(|e| CoreError::Wiki(e.to_string()))?;
    Ok(WikiPage {
        meta,
        body: body.trim().to_string(),
        stale: false,
    })
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str), CoreError> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with(FRONTMATTER_DELIM) {
        return Err(CoreError::Wiki("missing frontmatter delimiter".into()));
    }
    let rest = &trimmed[FRONTMATTER_DELIM.len()..];
    let end = rest
        .find(FRONTMATTER_DELIM)
        .ok_or_else(|| CoreError::Wiki("unclosed frontmatter".into()))?;
    let frontmatter = rest[..end].trim();
    let body = &rest[end + FRONTMATTER_DELIM.len()..];
    Ok((frontmatter, body))
}

/// Parses a page from disk for external callers (query crate).
pub fn load_page_from_path(path: &Path) -> Result<WikiPage, CoreError> {
    let raw = fs::read_to_string(path)?;
    parse_page(&raw)
}

/// Finds the best wiki page whose frontmatter anchors `symbol_id`.
pub fn find_page_for_symbol(
    store: &WikiStore,
    symbol_id: &str,
) -> Result<Option<WikiPage>, CoreError> {
    use crate::wiki::util::page_kind_priority;

    let mut best: Option<(u8, WikiPage)> = None;
    for id in store.list_page_ids()? {
        let page = store
            .load_page(&id)?
            .ok_or_else(|| CoreError::Wiki(format!("missing page {id}")))?;
        if !page.meta.symbol_ids.iter().any(|s| s == symbol_id) {
            continue;
        }
        let priority = page_kind_priority(page.meta.kind);
        if best.as_ref().map_or(true, |(p, _)| priority < *p) {
            best = Some((priority, page));
        }
    }
    Ok(best.map(|(_, p)| p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use becket_schema::wiki::{WikiPageKind, WikiPageSource};

    #[test]
    fn roundtrip_frontmatter() {
        let meta = WikiPageMeta {
            id: "wiki_test".into(),
            kind: WikiPageKind::Service,
            symbol_ids: vec!["sym_a".into()],
            source: WikiPageSource::Deterministic,
            graph_fingerprint: "abc".into(),
            see_also: vec![],
            title: "Test".into(),
        };
        let body = "## Structure\n\nHello";
        let raw = format!(
            "{FRONTMATTER_DELIM}\n{}\n{FRONTMATTER_DELIM}\n\n{body}",
            toml::to_string(&meta).unwrap()
        );
        let page = parse_page(&raw).unwrap();
        assert_eq!(page.meta.id, "wiki_test");
        assert!(page.body.contains("Structure"));
    }
}
