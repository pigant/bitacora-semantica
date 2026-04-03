mod cli;
mod models;
mod storage;
mod validation;
mod extended_help;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Parser;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::cli::{
    Cli, Commands, CreateActionArgs, CreateAssumptionArgs, CreateCommands, CreateDecisionArgs,
    CreateFactArgs, CreateMeetingArgs, CreatePersonArgs, ListArgs, NeighborsArgs, PathArgs,
    QueryLinksArgs, SearchArgs, ShowArgs, ValidateGraphArgs,
};
use crate::models::{
    ActionRecord, AssumptionRecord, BrokenReference, DecisionRecord, FactRecord,
    GraphValidationReport, Evidence, IndexEntry, Link, MeetingRecord, Outcome, PathResult,
    PersonRecord, Provenance,
};
use crate::storage::{
    collect_graph_edges, load_all_records, load_index, load_record_by_id, resolve_record_path,
    save_index, save_record, upsert_index_entry, write_record_at,
};
use crate::validation::{extract_search_text, validate_iso_datetime, validate_record_json, validate_relation};

fn print_skill_guide(skill: &str) {
    let normalized = skill.trim().to_lowercase();
    let create_variants = "SKILL CREATE (TODAS LAS VARIANTES):\n  decision   -> requeridos: --title --rationale\n  fact       -> requerido : --observation\n  assumption -> requerido : --assumption-statement\n  meeting    -> requeridos: --title --date\n  action     -> requerido : --title\n  person     -> requerido : --name";
    let advanced_fields = "CAMPOS OPCIONALES DE ONTOLOGIA:\n  --classification foundational|tactical|observational\n  --related-files a,b,c\n  --evidence-json '{\"type\":\"issue\",\"reference\":\"#123\"}' (repetible)\n  --outcomes-json '{\"status\":\"success\"}' (repetible)\n  --provenance-json '{\"source_tool\":\"cli\"}'\n  aliases: --project == --domain, --body == --description";

    if normalized == "general" {
        println!(
            "GUIA GENERAL\n\nComandos principales:\n  - create <tipo>\n  - link\n  - validate\n  - show/list/links/neighbors/search/validate-graph/path\n\n{}\n\nContraejemplos:\n  - create decision sin --title o sin --rationale\n  - create action --status invalid\n\n{}",
            create_variants, advanced_fields
        );
        return;
    }

    println!(
        "GUIA SKILL: {}\n\nUsa este dominio para centrar tus registros:\n  --domain {}\n\n{}\n\n{}",
        skill, skill, create_variants, advanced_fields
    );
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(skill) = cli.skill.as_deref() {
        print_skill_guide(skill);
        return Ok(());
    }

    match cli.command {
        Some(Commands::Create { kind }) => handle_create(cli.knowledge_dir, *kind),
        Some(Commands::Link(args)) => handle_link(cli.knowledge_dir, args),
        Some(Commands::Validate(args)) => handle_validate(args.file),
        Some(Commands::Show(args)) => handle_show(cli.knowledge_dir, args),
        Some(Commands::List(args)) => handle_list(cli.knowledge_dir, args),
        Some(Commands::Links(args)) => handle_links_query(cli.knowledge_dir, args),
        Some(Commands::Neighbors(args)) => handle_neighbors(cli.knowledge_dir, args),
        Some(Commands::Search(args)) => handle_search(cli.knowledge_dir, args),
        Some(Commands::ValidateGraph(args)) => handle_validate_graph(cli.knowledge_dir, args),
        Some(Commands::Path(args)) => handle_path(cli.knowledge_dir, args),
        Some(Commands::ExtendedHelp) => {
            extended_help::print_extended_help();
            Ok(())
        }
        None => Err(anyhow!(
            "debes indicar un subcomando o usar --skill. Corre --help para ver consultas"
        )),
    }

}

fn handle_create(knowledge_dir: std::path::PathBuf, kind: CreateCommands) -> Result<()> {
    let (value, domain) = match kind {
        CreateCommands::Decision(args) => create_decision(args)?,
        CreateCommands::Fact(args) => create_fact(args)?,
        CreateCommands::Assumption(args) => create_assumption(args)?,
        CreateCommands::Meeting(args) => create_meeting(args)?,
        CreateCommands::Action(args) => create_action(args)?,
        CreateCommands::Person(args) => create_person(args)?,
    };

    validate_record_json(&value)?;

    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("registro sin id"))?;

    let target = save_record(&knowledge_dir, &domain, id, &value)?;
    let mut index = load_index(&knowledge_dir)?;

    let entry = build_index_entry(&value, &knowledge_dir, &target)?;
    upsert_index_entry(&mut index, entry);
    save_index(&knowledge_dir, &index)?;

    println!("ok id={id} path={}", target.display());
    Ok(())
}

fn handle_link(knowledge_dir: std::path::PathBuf, args: cli::LinkArgs) -> Result<()> {
    validate_relation(&args.relation)?;
    if let Some(strength) = args.strength {
        if !(0.0..=1.0).contains(&strength) {
            return Err(anyhow!("strength debe estar entre 0.0 y 1.0"));
        }
    }

    let from_path = resolve_record_path(&knowledge_dir, &args.from)?;
    let _ = resolve_record_path(&knowledge_dir, &args.to)?;

    let from_data = fs::read(&from_path)
        .with_context(|| format!("no se pudo leer {}", from_path.display()))?;
    let mut from_value: Value = serde_json::from_slice(&from_data)
        .with_context(|| format!("json invalido en {}", from_path.display()))?;

    let link = Link {
        relation: args.relation.clone(),
        target: args.to.clone(),
        rationale: args.rationale.clone(),
        strength: args.strength,
        recorded_at: Some(Utc::now().to_rfc3339()),
    };

    {
        let links = from_value
            .as_object_mut()
            .ok_or_else(|| anyhow!("registro origen no es un objeto JSON"))?
            .entry("links")
            .or_insert_with(|| Value::Array(Vec::new()));

        let links_array = links
            .as_array_mut()
            .ok_or_else(|| anyhow!("campo links no es un arreglo"))?;
        links_array.push(serde_json::to_value(link)?);
    }

    validate_record_json(&from_value)?;
    write_record_at(&from_path, &from_value)?;

    println!(
        "ok linked from={} to={} path={}",
        args.from,
        args.to,
        from_path.display()
    );
    Ok(())
}

fn handle_validate(file: std::path::PathBuf) -> Result<()> {
    let data = fs::read(&file).with_context(|| format!("no se pudo leer {}", file.display()))?;
    let value: Value = serde_json::from_slice(&data)
        .with_context(|| format!("json invalido en {}", file.display()))?;
    validate_record_json(&value)?;
    println!("ok valid file={}", file.display());
    Ok(())
}

fn handle_show(knowledge_dir: std::path::PathBuf, args: ShowArgs) -> Result<()> {
    let (_, value) = load_record_by_id(&knowledge_dir, &args.id)?;
    print_json(&value)
}

fn handle_list(knowledge_dir: std::path::PathBuf, args: ListArgs) -> Result<()> {
    let mut entries = load_index(&knowledge_dir)?;

    let record_type_filter = args.record_type.as_ref().map(|s| s.to_ascii_lowercase());
    let domain_filter = args.domain.as_ref().map(|s| s.to_ascii_lowercase());
    let classification_filter = args.classification.as_ref().map(|s| s.to_ascii_lowercase());
    let status_filter = args.status.as_ref().map(|s| s.to_ascii_lowercase());
    let author_filter = args.author.as_ref().map(|s| s.to_ascii_lowercase());
    let tag_filters: Vec<String> = args.tags.iter().map(|t| t.to_ascii_lowercase()).collect();

    let from_date = args
        .from_date
        .as_deref()
        .map(|v| parse_datetime_bound(v, false))
        .transpose()?;
    let to_date = args
        .to_date
        .as_deref()
        .map(|v| parse_datetime_bound(v, true))
        .transpose()?;

    entries.retain(|entry| {
        if let Some(ref wanted) = record_type_filter
            && !entry.record_type.eq_ignore_ascii_case(wanted)
        {
            return false;
        }

        if let Some(ref wanted) = domain_filter {
            let Some(domain) = entry.domain.as_deref() else {
                return false;
            };
            if !domain.eq_ignore_ascii_case(wanted) {
                return false;
            }
        }

        if let Some(ref wanted) = status_filter {
            let Some(status) = entry.status.as_deref() else {
                return false;
            };
            if !status.eq_ignore_ascii_case(wanted) {
                return false;
            }
        }

        if let Some(ref wanted) = classification_filter {
            let Some(classification) = entry.classification.as_deref() else {
                return false;
            };
            if !classification.eq_ignore_ascii_case(wanted) {
                return false;
            }
        }

        if let Some(ref wanted) = author_filter {
            let Some(author) = entry.author.as_deref() else {
                return false;
            };
            if !author.to_ascii_lowercase().contains(wanted) {
                return false;
            }
        }

        if !tag_filters.is_empty()
            && !tag_filters.iter().all(|required| {
                entry
                    .tags
                    .iter()
                    .any(|tag| tag.eq_ignore_ascii_case(required))
            })
        {
            return false;
        }

        if from_date.is_some() || to_date.is_some() {
            let Some(recorded_at) = entry.recorded_at.as_deref() else {
                return false;
            };
            let Some(record_dt) = parse_recorded_at_utc(recorded_at) else {
                return false;
            };

            if let Some(from_dt) = from_date.as_ref() && record_dt < *from_dt {
                return false;
            }
            if let Some(to_dt) = to_date.as_ref() && record_dt > *to_dt {
                return false;
            }
        }

        true
    });

    let page = entries
        .into_iter()
        .skip(args.offset)
        .take(args.limit)
        .collect::<Vec<_>>();
    print_json(&page)
}

fn handle_links_query(knowledge_dir: std::path::PathBuf, args: QueryLinksArgs) -> Result<()> {
    if args.from.is_none() && args.to.is_none() {
        return Err(anyhow!("debes indicar --from o --to para consultar links"));
    }

    let records = load_all_records(&knowledge_dir)?;
    let mut edges = collect_graph_edges(&records);

    if let Some(from) = args.from.as_deref() {
        edges.retain(|e| e.from == from);
    }
    if let Some(to) = args.to.as_deref() {
        edges.retain(|e| e.to == to);
    }
    if let Some(relation) = args.relation.as_deref() {
        edges.retain(|e| e.relation.eq_ignore_ascii_case(relation));
    }

    print_json(&edges)
}

fn handle_neighbors(knowledge_dir: std::path::PathBuf, args: NeighborsArgs) -> Result<()> {
    if args.depth == 0 || args.depth > 5 {
        return Err(anyhow!("--depth debe estar entre 1 y 5"));
    }

    let _ = load_record_by_id(&knowledge_dir, &args.id)?;
    let records = load_all_records(&knowledge_dir)?;
    let mut edges = collect_graph_edges(&records);

    if let Some(relation) = args.relation.as_deref() {
        edges.retain(|e| e.relation.eq_ignore_ascii_case(relation));
    }

    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        adjacency
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    #[derive(Serialize)]
    struct NeighborNode {
        id: String,
        distance: usize,
    }

    #[derive(Serialize)]
    struct NeighborsOutput {
        center: String,
        depth: usize,
        neighbors: Vec<NeighborNode>,
    }

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut neighbors = Vec::new();

    visited.insert(args.id.clone());
    queue.push_back((args.id.clone(), 0));

    while let Some((current, distance)) = queue.pop_front() {
        if distance >= args.depth {
            continue;
        }

        if let Some(next_nodes) = adjacency.get(&current) {
            for next in next_nodes {
                if visited.insert(next.clone()) {
                    let next_distance = distance + 1;
                    neighbors.push(NeighborNode {
                        id: next.clone(),
                        distance: next_distance,
                    });
                    queue.push_back((next.clone(), next_distance));
                }
            }
        }
    }

    neighbors.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.id.cmp(&b.id)));

    let out = NeighborsOutput {
        center: args.id,
        depth: args.depth,
        neighbors,
    };

    print_json(&out)
}

fn handle_search(knowledge_dir: std::path::PathBuf, args: SearchArgs) -> Result<()> {
    let query = args.query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Err(anyhow!("--query no puede estar vacio"));
    }

    let records = load_all_records(&knowledge_dir)?;
    let type_filters: Vec<String> = args.in_types.iter().map(|t| t.to_ascii_lowercase()).collect();

    let mut hits = Vec::new();
    for (entry, record) in records {
        if !type_filters.is_empty()
            && !type_filters
                .iter()
                .any(|t| entry.record_type.eq_ignore_ascii_case(t))
        {
            continue;
        }

        let haystack = extract_search_text(&record);
        if haystack.contains(&query) {
            hits.push(entry);
        }
    }

    if hits.len() > args.limit {
        hits.truncate(args.limit);
    }

    print_json(&hits)
}

fn handle_validate_graph(knowledge_dir: std::path::PathBuf, args: ValidateGraphArgs) -> Result<()> {
    let records = load_all_records(&knowledge_dir)?;
    let edges = collect_graph_edges(&records);

    let ids: HashSet<String> = records.iter().map(|(entry, _)| entry.id.clone()).collect();

    let mut broken_references = Vec::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();

    for id in &ids {
        in_degree.insert(id.clone(), 0);
        out_degree.insert(id.clone(), 0);
    }

    for edge in &edges {
        *out_degree.entry(edge.from.clone()).or_insert(0) += 1;
        if ids.contains(&edge.to) {
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
        } else {
            broken_references.push(BrokenReference {
                from: edge.from.clone(),
                to: edge.to.clone(),
                relation: edge.relation.clone(),
            });
        }
    }

    let mut orphan_nodes = Vec::new();
    for id in &ids {
        let in_count = in_degree.get(id).copied().unwrap_or_default();
        let out_count = out_degree.get(id).copied().unwrap_or_default();
        if in_count == 0 && out_count == 0 {
            orphan_nodes.push(id.clone());
        }
    }
    orphan_nodes.sort();

    let has_cycle = graph_has_cycle(&ids, &edges);

    let report = GraphValidationReport {
        total_nodes: ids.len(),
        total_edges: edges.len(),
        broken_references,
        orphan_nodes,
        has_cycle,
    };

    if args.report.eq_ignore_ascii_case("json") {
        print_json(&report)?;
    } else {
        println!(
            "nodes={} edges={} broken_refs={} orphans={} has_cycle={}",
            report.total_nodes,
            report.total_edges,
            report.broken_references.len(),
            report.orphan_nodes.len(),
            report.has_cycle
        );
        for broken in &report.broken_references {
            println!(
                "broken from={} to={} relation={}",
                broken.from, broken.to, broken.relation
            );
        }
    }

    if report.broken_references.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "se encontraron {} referencias rotas en el grafo",
            report.broken_references.len()
        ))
    }
}

fn handle_path(knowledge_dir: std::path::PathBuf, args: PathArgs) -> Result<()> {
    if args.max_depth == 0 || args.max_depth > 20 {
        return Err(anyhow!("--max-depth debe estar entre 1 y 20"));
    }

    let records = load_all_records(&knowledge_dir)?;
    let mut edges = collect_graph_edges(&records);
    let ids: HashSet<String> = records.iter().map(|(entry, _)| entry.id.clone()).collect();

    if !ids.contains(&args.from) {
        return Err(anyhow!("id --from no encontrado: {}", args.from));
    }
    if !ids.contains(&args.to) {
        return Err(anyhow!("id --to no encontrado: {}", args.to));
    }

    if let Some(relation) = args.relation.as_deref() {
        edges.retain(|e| e.relation.eq_ignore_ascii_case(relation));
    }

    if args.from == args.to {
        let out = PathResult {
            found: true,
            from: args.from.clone(),
            to: args.to.clone(),
            path: vec![args.from],
            hops: 0,
        };
        return print_json(&out);
    }

    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();

    visited.insert(args.from.clone());
    queue.push_back((args.from.clone(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if current == args.to {
            break;
        }
        if depth >= args.max_depth {
            continue;
        }

        if let Some(next_nodes) = adjacency.get(&current) {
            for next in next_nodes {
                if visited.insert(next.clone()) {
                    parent.insert(next.clone(), current.clone());
                    queue.push_back((next.clone(), depth + 1));
                }
            }
        }
    }

    if !visited.contains(&args.to) {
        let out = PathResult {
            found: false,
            from: args.from,
            to: args.to,
            path: Vec::new(),
            hops: 0,
        };
        return print_json(&out);
    }

    let mut path = vec![args.to.clone()];
    let mut cursor = args.to.clone();

    while cursor != args.from {
        let prev = parent
            .get(&cursor)
            .ok_or_else(|| anyhow!("no se pudo reconstruir el camino"))?;
        cursor = prev.clone();
        path.push(cursor.clone());
    }

    path.reverse();
    let hops = path.len().saturating_sub(1);

    let out = PathResult {
        found: true,
        from: args.from,
        to: args.to,
        path,
        hops,
    };
    print_json(&out)
}

fn parse_recorded_at_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_datetime_bound(value: &str, end_of_day: bool) -> Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.with_timezone(&Utc));
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| anyhow!("fecha invalida: {value}. Usa YYYY-MM-DD o RFC3339"))?;
    let naive = if end_of_day {
        date.and_hms_opt(23, 59, 59)
            .ok_or_else(|| anyhow!("fecha invalida: {value}"))?
    } else {
        date.and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow!("fecha invalida: {value}"))?
    };

    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn graph_has_cycle(ids: &HashSet<String>, edges: &[crate::models::GraphEdge]) -> bool {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in edges {
        if ids.contains(&edge.to) {
            adjacency
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }
    }

    fn dfs(
        node: &str,
        adjacency: &HashMap<String, Vec<String>>,
        state: &mut HashMap<String, u8>,
    ) -> bool {
        state.insert(node.to_string(), 1);

        if let Some(next_nodes) = adjacency.get(node) {
            for next in next_nodes {
                match state.get(next).copied() {
                    Some(1) => return true,
                    Some(2) => continue,
                    _ => {
                        if dfs(next, adjacency, state) {
                            return true;
                        }
                    }
                }
            }
        }

        state.insert(node.to_string(), 2);
        false
    }

    let mut state: HashMap<String, u8> = HashMap::new();
    for id in ids {
        if !state.contains_key(id) && dfs(id, &adjacency, &mut state) {
            return true;
        }
    }
    false
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Debug, Default)]
struct CommonOntologyFields {
    classification: Option<String>,
    related_files: Vec<String>,
    evidence: Vec<Evidence>,
    outcomes: Vec<Outcome>,
    provenance: Option<Provenance>,
}

fn parse_json_argument<T: DeserializeOwned>(raw: &str, flag: &str) -> Result<T> {
    serde_json::from_str(raw).with_context(|| format!("{flag} invalido, se esperaba JSON: {raw}"))
}

fn parse_common_ontology_fields(common: &cli::CommonCreateArgs) -> Result<CommonOntologyFields> {
    let evidence = common
        .evidence_json
        .iter()
        .map(|raw| parse_json_argument::<Evidence>(raw, "--evidence-json"))
        .collect::<Result<Vec<_>>>()?;

    let outcomes = common
        .outcomes_json
        .iter()
        .map(|raw| parse_json_argument::<Outcome>(raw, "--outcomes-json"))
        .collect::<Result<Vec<_>>>()?;

    let provenance = common
        .provenance_json
        .as_deref()
        .map(|raw| parse_json_argument::<Provenance>(raw, "--provenance-json"))
        .transpose()?;

    Ok(CommonOntologyFields {
        classification: common.classification.clone(),
        related_files: common.related_files.clone(),
        evidence,
        outcomes,
        provenance,
    })
}

fn create_decision(args: CreateDecisionArgs) -> Result<(Value, String)> {
    let common_ontology = parse_common_ontology_fields(&args.common)?;
    let domain = args
        .common
        .domain
        .clone()
        .unwrap_or_else(|| "general".to_string());
    let id = args.common.id.unwrap_or_else(|| make_urn(&domain));
    let recorded_at = normalize_recorded_at(args.common.recorded_at.as_deref())?;

    let record = DecisionRecord {
        context: args.common.context,
        id,
        record_type: "Decision".to_string(),
        recorded_at,
        title: args.title,
        rationale: args.rationale,
        domain: Some(domain.clone()),
        description: args.common.description,
        tags: args.common.tags,
        author: args.common.author,
        status: args.common.status,
        confidence: args.common.confidence,
        impact: args.common.impact,
        classification: common_ontology.classification,
        related_files: common_ontology.related_files,
        evidence: common_ontology.evidence,
        outcomes: common_ontology.outcomes,
        provenance: common_ontology.provenance,
        options_considered: args.options_considered,
        chosen_option: args.chosen_option,
        effective_from: args.effective_from,
        consequences: args.consequences,
        impacted_components: args.impacted_components,
        links: Vec::new(),
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn create_fact(args: CreateFactArgs) -> Result<(Value, String)> {
    let common_ontology = parse_common_ontology_fields(&args.common)?;
    let domain = args
        .common
        .domain
        .clone()
        .unwrap_or_else(|| "general".to_string());
    let id = args.common.id.unwrap_or_else(|| make_urn(&domain));
    let recorded_at = normalize_recorded_at(args.common.recorded_at.as_deref())?;

    let record = FactRecord {
        context: args.common.context,
        id,
        record_type: "Fact".to_string(),
        recorded_at,
        observation: args.observation,
        domain: Some(domain.clone()),
        description: args.common.description,
        tags: args.common.tags,
        author: args.common.author,
        confidence: args.common.confidence,
        classification: common_ontology.classification,
        related_files: common_ontology.related_files,
        related_components: args.related_components,
        evidence: common_ontology.evidence,
        outcomes: common_ontology.outcomes,
        provenance: common_ontology.provenance,
        links: Vec::new(),
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn create_assumption(args: CreateAssumptionArgs) -> Result<(Value, String)> {
    let common_ontology = parse_common_ontology_fields(&args.common)?;
    let domain = args
        .common
        .domain
        .clone()
        .unwrap_or_else(|| "general".to_string());
    let id = args.common.id.unwrap_or_else(|| make_urn(&domain));
    let recorded_at = normalize_recorded_at(args.common.recorded_at.as_deref())?;

    let record = AssumptionRecord {
        context: args.common.context,
        id,
        record_type: "Assumption".to_string(),
        recorded_at,
        assumption_statement: args.assumption_statement,
        domain: Some(domain.clone()),
        description: args.common.description,
        tags: args.common.tags,
        author: args.common.author,
        confidence: args.common.confidence,
        classification: common_ontology.classification,
        related_files: common_ontology.related_files,
        evidence: common_ontology.evidence,
        outcomes: common_ontology.outcomes,
        provenance: common_ontology.provenance,
        tests_needed: args.tests_needed,
        expire_at: args.expire_at,
        links: Vec::new(),
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn create_meeting(args: CreateMeetingArgs) -> Result<(Value, String)> {
    let common_ontology = parse_common_ontology_fields(&args.common)?;
    let domain = args
        .common
        .domain
        .clone()
        .unwrap_or_else(|| "general".to_string());
    let id = args.common.id.unwrap_or_else(|| make_urn(&domain));
    let recorded_at = normalize_recorded_at(args.common.recorded_at.as_deref())?;

    let record = MeetingRecord {
        context: args.common.context,
        id,
        record_type: "Meeting".to_string(),
        recorded_at,
        title: args.title,
        date: args.date,
        domain: Some(domain.clone()),
        description: args.common.description,
        tags: args.common.tags,
        author: args.common.author,
        classification: common_ontology.classification,
        related_files: common_ontology.related_files,
        evidence: common_ontology.evidence,
        outcomes: common_ontology.outcomes,
        provenance: common_ontology.provenance,
        location: args.location,
        participants: args.participants,
        minutes: args.minutes,
        decisions_made: args.decisions_made,
        actions: args.actions,
        links: Vec::new(),
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn create_action(args: CreateActionArgs) -> Result<(Value, String)> {
    let common_ontology = parse_common_ontology_fields(&args.common)?;
    let domain = args
        .common
        .domain
        .clone()
        .unwrap_or_else(|| "general".to_string());
    let id = args.common.id.unwrap_or_else(|| make_urn(&domain));
    let recorded_at = normalize_recorded_at(args.common.recorded_at.as_deref())?;
    let outcome = args
        .outcome_json
        .as_deref()
        .map(|raw| parse_json_argument::<Outcome>(raw, "--outcome-json"))
        .transpose()?;

    let record = ActionRecord {
        context: args.common.context,
        id,
        record_type: "Action".to_string(),
        recorded_at,
        title: args.title,
        domain: Some(domain.clone()),
        description: args.common.description,
        tags: args.common.tags,
        author: args.common.author,
        classification: common_ontology.classification,
        related_files: common_ontology.related_files,
        evidence: common_ontology.evidence,
        outcomes: common_ontology.outcomes,
        provenance: common_ontology.provenance,
        status: args.common.status,
        assigned_to: args.assigned_to,
        due_date: args.due_date,
        parent_decision: args.parent_decision,
        outcome,
        links: Vec::new(),
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn create_person(args: CreatePersonArgs) -> Result<(Value, String)> {
    let domain = "people".to_string();
    let id = args.id.unwrap_or_else(|| make_urn(&domain));

    let record = PersonRecord {
        context: args.context,
        id,
        record_type: "Person".to_string(),
        name: args.name,
        role: args.role,
        contact: args.contact,
    };

    Ok((serde_json::to_value(record)?, domain))
}

fn build_index_entry(value: &Value, knowledge_dir: &std::path::Path, target: &std::path::Path) -> Result<IndexEntry> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("registro sin id"))?
        .to_string();
    let record_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("registro sin type"))?
        .to_string();
    let path = target
        .strip_prefix(knowledge_dir)
        .unwrap_or(target)
        .to_string_lossy()
        .to_string();

    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    Ok(IndexEntry {
        id,
        record_type,
        title: value
            .get("title")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        domain: value
            .get("domain")
            .or_else(|| value.get("project"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        classification: value
            .get("classification")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        tags,
        recorded_at: value
            .get("recorded_at")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        status: value
            .get("status")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        author: value
            .get("author")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        path,
    })
}

fn normalize_recorded_at(recorded_at: Option<&str>) -> Result<String> {
    if let Some(value) = recorded_at {
        validate_iso_datetime(value, "recorded_at")?;
        Ok(value.to_string())
    } else {
        Ok(Utc::now().to_rfc3339())
    }
}

fn make_urn(domain: &str) -> String {
    format!("urn:bitacora:{}:{}", domain, Uuid::new_v4())
}
