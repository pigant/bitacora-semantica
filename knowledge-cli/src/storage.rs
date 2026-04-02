use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::models::{GraphEdge, IndexEntry};

pub fn sanitize_id_for_filename(id: &str) -> String {
    id.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

pub fn save_record(knowledge_dir: &Path, domain: &str, id: &str, value: &Value) -> Result<PathBuf> {
    let domain_dir = knowledge_dir.join(domain);
    fs::create_dir_all(&domain_dir)
        .with_context(|| format!("no se pudo crear directorio: {}", domain_dir.display()))?;

    let filename = format!("{}.jsonld", sanitize_id_for_filename(id));
    let target = domain_dir.join(filename);

    if target.exists() {
        return Err(anyhow!("ya existe un registro con ese id: {}", target.display()));
    }

    write_json_atomic(&target, value)?;
    Ok(target)
}

pub fn write_record_at(path: &Path, value: &Value) -> Result<()> {
    write_json_atomic(path, value)
}

pub fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("ruta invalida, sin directorio padre: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("no se pudo crear directorio: {}", parent.display()))?;

    let tmp = path.with_extension("jsonld.tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp, bytes).with_context(|| format!("no se pudo escribir tmp: {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("no se pudo renombrar {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

pub fn load_index(knowledge_dir: &Path) -> Result<Vec<IndexEntry>> {
    let index_path = knowledge_dir.join("index.json");
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read(&index_path)
        .with_context(|| format!("no se pudo leer index: {}", index_path.display()))?;
    let entries: Vec<IndexEntry> = serde_json::from_slice(&data)
        .with_context(|| format!("index invalido: {}", index_path.display()))?;
    Ok(entries)
}

pub fn save_index(knowledge_dir: &Path, entries: &[IndexEntry]) -> Result<()> {
    let index_path = knowledge_dir.join("index.json");
    let value = serde_json::to_value(entries)?;
    write_json_atomic(&index_path, &value)
}

pub fn upsert_index_entry(entries: &mut Vec<IndexEntry>, entry: IndexEntry) {
    if let Some(pos) = entries.iter().position(|e| e.id == entry.id) {
        entries[pos] = entry;
    } else {
        entries.push(entry);
    }
}

pub fn resolve_record_path(knowledge_dir: &Path, id: &str) -> Result<PathBuf> {
    let entries = load_index(knowledge_dir)?;
    let found = entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| anyhow!("id no encontrado en index: {id}"))?;

    let candidate = knowledge_dir.join(&found.path);
    if !candidate.exists() {
        return Err(anyhow!(
            "registro en index pero archivo no existe: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}

pub fn load_record_value(path: &Path) -> Result<Value> {
    let data = fs::read(path).with_context(|| format!("no se pudo leer {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("json invalido en {}", path.display()))
}

pub fn load_record_by_id(knowledge_dir: &Path, id: &str) -> Result<(IndexEntry, Value)> {
    let entries = load_index(knowledge_dir)?;
    let entry = entries
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| anyhow!("id no encontrado en index: {id}"))?;

    let path = knowledge_dir.join(&entry.path);
    if !path.exists() {
        return Err(anyhow!(
            "registro en index pero archivo no existe: {}",
            path.display()
        ));
    }

    let value = load_record_value(&path)?;
    Ok((entry, value))
}

pub fn load_all_records(knowledge_dir: &Path) -> Result<Vec<(IndexEntry, Value)>> {
    let entries = load_index(knowledge_dir)?;
    let mut out = Vec::with_capacity(entries.len());

    for entry in entries {
        let path = knowledge_dir.join(&entry.path);
        let value = load_record_value(&path)?;
        out.push((entry, value));
    }

    Ok(out)
}

pub fn collect_graph_edges(records: &[(IndexEntry, Value)]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    for (entry, record) in records {
        let Some(links) = record.get("links").and_then(|v| v.as_array()) else {
            continue;
        };

        for link in links {
            let Some(relation) = link.get("relation").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(target) = link.get("target").and_then(|v| v.as_str()) else {
                continue;
            };

            edges.push(GraphEdge {
                from: entry.id.clone(),
                to: target.to_string(),
                relation: relation.to_string(),
            });
        }
    }

    edges
}
