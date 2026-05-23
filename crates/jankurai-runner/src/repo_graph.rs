//! Lightweight repository graph builder for worker context.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use syn::visit::Visit;

/// Repository graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Stable node id.
    pub id: String,
    /// Node kind.
    pub kind: String,
    /// Stable key.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Node payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_json: Option<serde_json::Value>,
}

/// Repository graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node id.
    pub from: String,
    /// Destination node id.
    pub to: String,
    /// Edge kind.
    pub kind: String,
    /// Edge payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_json: Option<serde_json::Value>,
}

/// Built repository graph.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoGraph {
    /// Nodes.
    pub nodes: Vec<GraphNode>,
    /// Edges.
    pub edges: Vec<GraphEdge>,
}

impl RepoGraph {
    /// Return tests known to cover a file key.
    pub fn tests_covering(&self, file_key: &str) -> Vec<&GraphNode> {
        let file_ids: BTreeSet<&str> = self
            .nodes
            .iter()
            .filter(|node| node.kind == "file" && node.key == file_key)
            .map(|node| node.id.as_str())
            .collect();
        let test_ids: BTreeSet<&str> = self
            .edges
            .iter()
            .filter(|edge| edge.kind == "tests" && file_ids.contains(edge.to.as_str()))
            .map(|edge| edge.from.as_str())
            .collect();
        self.nodes
            .iter()
            .filter(|node| test_ids.contains(node.id.as_str()))
            .collect()
    }

    /// Return graph summary counts by node kind.
    pub fn summary(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for node in &self.nodes {
            *counts.entry(node.kind.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Export the graph as pretty JSON.
    pub fn export_json(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}

/// Build a first-pass repo graph: files, docs, tests, Rust modules, packages, and imports.
pub fn build_repo_graph(repo_root: &Path) -> Result<RepoGraph> {
    let mut builder = GraphBuilder::default();
    let files = discover_files(repo_root)?;
    let package_id = if repo_root.join("Cargo.toml").exists() {
        Some(builder.node("package", "Cargo.toml", "Cargo package"))
    } else {
        None
    };
    for rel in &files {
        let key = rel.to_string_lossy().replace('\\', "/");
        let kind = if key.ends_with(".md") || key.starts_with("docs/") {
            "doc"
        } else if is_test_file(&key) {
            "test"
        } else {
            "file"
        };
        let file_id = builder.node(kind, &key, &key);
        if let Some(package_id) = &package_id {
            builder.edge(package_id, &file_id, "contains");
        }
        if key.ends_with(".rs") {
            let module_id = builder.node("module", &key, &key);
            builder.edge(&module_id, &file_id, "contains");
            add_import_edges(repo_root, rel, &mut builder, &module_id)?;
            add_rust_symbol_edges(repo_root, rel, &key, &mut builder, &module_id)?;
        }
    }
    add_test_edges(&files, &mut builder);
    Ok(builder.finish())
}

fn discover_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(repo_root, repo_root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if matches!(name.as_ref(), ".git" | ".zyal" | ".jankurai" | "target") {
                continue;
            }
            walk(root, &path, out)?;
        } else if path.is_file() {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn is_test_file(key: &str) -> bool {
    key.starts_with("tests/") || key.ends_with("_test.rs") || key.ends_with("_tests.rs")
}

fn add_import_edges(
    repo_root: &Path,
    rel: &Path,
    builder: &mut GraphBuilder,
    module_id: &str,
) -> Result<()> {
    let text = fs::read_to_string(repo_root.join(rel)).unwrap_or_default();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use ") {
            let key = rest
                .trim_end_matches(';')
                .split_whitespace()
                .next()
                .unwrap_or(rest);
            let import_id = builder.node("module", key, key);
            builder.edge(module_id, &import_id, "imports");
        }
        if let Some(rest) = trimmed.strip_prefix("mod ") {
            let key = rest.trim_end_matches(';').trim();
            let import_id = builder.node("module", key, key);
            builder.edge(module_id, &import_id, "imports");
        }
    }
    Ok(())
}

fn add_rust_symbol_edges(
    repo_root: &Path,
    rel: &Path,
    key: &str,
    builder: &mut GraphBuilder,
    module_id: &str,
) -> Result<()> {
    let text = fs::read_to_string(repo_root.join(rel)).unwrap_or_default();
    let Ok(file) = syn::parse_file(&text) else {
        return Ok(());
    };
    for item in file.items {
        match item {
            syn::Item::Fn(item_fn) => {
                let name = item_fn.sig.ident.to_string();
                let fn_key = format!("{key}::{name}");
                let fn_id = builder.node_with_payload(
                    "function",
                    &fn_key,
                    &name,
                    json!({"file": key, "visibility": visibility_label(&item_fn.vis)}),
                );
                builder.edge(module_id, &fn_id, "contains");
                add_call_edges(builder, &fn_id, &item_fn.block);
            }
            syn::Item::Struct(item_struct) => {
                let name = item_struct.ident.to_string();
                let struct_key = format!("{key}::{name}");
                let struct_id = builder.node_with_payload(
                    "struct",
                    &struct_key,
                    &name,
                    json!({"file": key, "visibility": visibility_label(&item_struct.vis)}),
                );
                builder.edge(module_id, &struct_id, "contains");
            }
            syn::Item::Enum(item_enum) => {
                let name = item_enum.ident.to_string();
                let enum_key = format!("{key}::{name}");
                let enum_id = builder.node_with_payload(
                    "enum",
                    &enum_key,
                    &name,
                    json!({"file": key, "visibility": visibility_label(&item_enum.vis)}),
                );
                builder.edge(module_id, &enum_id, "contains");
            }
            syn::Item::Impl(item_impl) => {
                let impl_name = impl_label(&item_impl.self_ty);
                let impl_key = format!("{key}::impl::{impl_name}");
                let impl_id =
                    builder.node_with_payload("impl", &impl_key, &impl_name, json!({"file": key}));
                builder.edge(module_id, &impl_id, "contains");
                for item in item_impl.items {
                    if let syn::ImplItem::Fn(method) = item {
                        let name = method.sig.ident.to_string();
                        let method_key = format!("{impl_key}::{name}");
                        let method_id = builder.node_with_payload(
                            "method",
                            &method_key,
                            &name,
                            json!({"file": key, "impl": impl_name}),
                        );
                        builder.edge(&impl_id, &method_id, "contains");
                        add_call_edges(builder, &method_id, &method.block);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn add_call_edges(builder: &mut GraphBuilder, caller_id: &str, block: &syn::Block) {
    let mut visitor = CallVisitor::default();
    visitor.visit_block(block);
    for call in visitor.calls {
        let callee_id = builder.node_with_payload(
            "function",
            &format!("symbol::{call}"),
            &call,
            json!({"approximate": true}),
        );
        builder.edge_with_payload(caller_id, &callee_id, "calls", json!({"approximate": true}));
    }
}

#[derive(Default)]
struct CallVisitor {
    calls: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if let Some(segment) = path.path.segments.last() {
                self.calls.insert(segment.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.calls.insert(node.method.to_string());
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn visibility_label(vis: &syn::Visibility) -> &'static str {
    match vis {
        syn::Visibility::Public(_) => "public",
        _ => "private",
    }
}

fn impl_label(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

fn add_test_edges(files: &[PathBuf], builder: &mut GraphBuilder) {
    let source_files: Vec<String> = files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|key| key.starts_with("src/") && key.ends_with(".rs"))
        .collect();
    for test in files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|key| is_test_file(key))
    {
        let stem = Path::new(&test)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        for source in &source_files {
            if source.contains(stem) || stem == "integration" {
                let test_id = builder.node("test", &test, &test);
                let source_id = builder.node("file", source, source);
                builder.edge(&test_id, &source_id, "tests");
            }
        }
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes_by_key: BTreeMap<(String, String), String>,
    nodes: Vec<GraphNode>,
    edges: BTreeMap<(String, String, String), Option<serde_json::Value>>,
}

impl GraphBuilder {
    fn node(&mut self, kind: &str, key: &str, label: &str) -> String {
        self.node_inner(kind, key, label, None)
    }

    fn node_with_payload(
        &mut self,
        kind: &str,
        key: &str,
        label: &str,
        payload_json: serde_json::Value,
    ) -> String {
        self.node_inner(kind, key, label, Some(payload_json))
    }

    fn node_inner(
        &mut self,
        kind: &str,
        key: &str,
        label: &str,
        payload_json: Option<serde_json::Value>,
    ) -> String {
        let lookup = (kind.to_string(), key.to_string());
        if let Some(id) = self.nodes_by_key.get(&lookup) {
            return id.clone();
        }
        let id = node_id(kind, key);
        self.nodes_by_key.insert(lookup, id.clone());
        self.nodes.push(GraphNode {
            id: id.clone(),
            kind: kind.to_string(),
            key: key.to_string(),
            label: label.to_string(),
            payload_json,
        });
        id
    }

    fn edge(&mut self, from: &str, to: &str, kind: &str) {
        self.edges
            .entry((from.to_string(), to.to_string(), kind.to_string()))
            .or_insert(None);
    }

    fn edge_with_payload(
        &mut self,
        from: &str,
        to: &str,
        kind: &str,
        payload_json: serde_json::Value,
    ) {
        self.edges.insert(
            (from.to_string(), to.to_string(), kind.to_string()),
            Some(payload_json),
        );
    }

    fn finish(self) -> RepoGraph {
        RepoGraph {
            nodes: self.nodes,
            edges: self
                .edges
                .into_iter()
                .map(|((from, to, kind), payload_json)| GraphEdge {
                    from,
                    to,
                    kind,
                    payload_json,
                })
                .collect(),
        }
    }
}

fn node_id(kind: &str, key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b":");
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builds_file_test_and_import_edges() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/ping.rs"),
            "use std::fmt;\nmod codec;\npub struct Ping;\npub enum Reply { Pong }\npub fn ping() { helper(); }\nfn helper() {}\nimpl Ping { pub fn run(&self) { ping(); self.private(); } fn private(&self) {} }\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(dir.path().join("tests/ping.rs"), "#[test]\nfn ping() {}\n").unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::write(dir.path().join("docs/spec.md"), "spec").unwrap();

        let graph = build_repo_graph(dir.path()).unwrap();
        let summary = graph.summary();
        assert_eq!(summary.get("test").copied(), Some(1));
        assert!(summary.get("doc").copied().unwrap_or(0) >= 1);
        assert!(!graph.tests_covering("src/ping.rs").is_empty());
        assert!(graph.edges.iter().any(|edge| edge.kind == "imports"));
        assert!(graph.nodes.iter().any(|node| node.kind == "function"));
        assert!(graph.nodes.iter().any(|node| node.kind == "struct"));
        assert!(graph.nodes.iter().any(|node| node.kind == "enum"));
        assert!(graph.nodes.iter().any(|node| node.kind == "method"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "calls"));
    }
}
