use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_knowledge-cli"))
}

fn run_cli(knowledge: &PathBuf, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(bin());
    cmd.arg("--knowledge-dir")
        .arg(knowledge.to_str().expect("utf8 path"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run cli command")
}

fn read_index(knowledge: &PathBuf) -> Vec<Value> {
    let index_path = knowledge.join("index.json");
    let index_raw = fs::read_to_string(&index_path).expect("read index");
    serde_json::from_str(&index_raw).expect("parse index")
}

fn index_id_by_title(index: &[Value], title: &str) -> String {
    index
        .iter()
        .find(|entry| entry.get("title").and_then(Value::as_str) == Some(title))
        .and_then(|entry| entry.get("id").and_then(Value::as_str))
        .expect("id by title")
        .to_string()
}

fn index_path_by_id(index: &[Value], id: &str) -> String {
    index
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
        .and_then(|entry| entry.get("path").and_then(Value::as_str))
        .expect("path by id")
        .to_string()
}

fn validate_file(knowledge: &PathBuf, file: &PathBuf) {
    let validate = run_cli(
        knowledge,
        &["validate", "--file", file.to_str().expect("utf8 path")],
    );
    assert!(
        validate.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn create_decision_and_validate_file() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let output = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "create",
            "decision",
            "--title",
            "Adoptar Postgres",
            "--rationale",
            "Necesitamos ACID",
            "--domain",
            "payments",
            "--status",
            "accepted",
        ])
        .output()
        .expect("run create decision");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));

    let index_path = knowledge.join("index.json");
    assert!(index_path.exists(), "index.json debe existir");

    let index_raw = fs::read_to_string(&index_path).expect("read index");
    let index: Vec<Value> = serde_json::from_str(&index_raw).expect("parse index");
    assert_eq!(index.len(), 1);

    let record_path = knowledge.join(
        index[0]
            .get("path")
            .and_then(Value::as_str)
            .expect("path in index"),
    );
    assert!(record_path.exists(), "record file should exist");

    let validate = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "validate",
            "--file",
            record_path.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("run validate");

    assert!(validate.status.success(), "stderr={}", String::from_utf8_lossy(&validate.stderr));
}

#[test]
fn create_two_records_and_link_them() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create_decision = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "create",
            "decision",
            "--title",
            "Adoptar Postgres",
            "--rationale",
            "Necesitamos ACID",
            "--domain",
            "payments",
        ])
        .output()
        .expect("run create decision");
    assert!(create_decision.status.success(), "stderr={}", String::from_utf8_lossy(&create_decision.stderr));

    let create_fact = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "create",
            "fact",
            "--observation",
            "La cola X se saturó",
            "--domain",
            "payments",
            "--confidence",
            "0.95",
        ])
        .output()
        .expect("run create fact");
    assert!(create_fact.status.success(), "stderr={}", String::from_utf8_lossy(&create_fact.stderr));

    let index_path = knowledge.join("index.json");
    let index_raw = fs::read_to_string(&index_path).expect("read index");
    let index: Vec<Value> = serde_json::from_str(&index_raw).expect("parse index");
    assert_eq!(index.len(), 2);

    let decision_id = index
        .iter()
        .find(|entry| entry.get("type").and_then(Value::as_str) == Some("Decision"))
        .and_then(|entry| entry.get("id"))
        .and_then(Value::as_str)
        .expect("decision id")
        .to_string();

    let fact_id = index
        .iter()
        .find(|entry| entry.get("type").and_then(Value::as_str) == Some("Fact"))
        .and_then(|entry| entry.get("id"))
        .and_then(Value::as_str)
        .expect("fact id")
        .to_string();

    let link = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "link",
            "--from",
            &decision_id,
            "--to",
            &fact_id,
            "--relation",
            "relatesTo",
        ])
        .output()
        .expect("run link");
    assert!(link.status.success(), "stderr={}", String::from_utf8_lossy(&link.stderr));

    let decision_path = knowledge.join(
        index
            .iter()
            .find(|entry| entry.get("id").and_then(Value::as_str) == Some(decision_id.as_str()))
            .and_then(|entry| entry.get("path").and_then(Value::as_str))
            .expect("decision path"),
    );

    let decision_raw = fs::read_to_string(&decision_path).expect("read decision");
    let decision_json: Value = serde_json::from_str(&decision_raw).expect("parse decision");
    let links = decision_json
        .get("links")
        .and_then(Value::as_array)
        .expect("links array");

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].get("relation").and_then(Value::as_str), Some("relatesTo"));
    assert_eq!(links[0].get("target").and_then(Value::as_str), Some(fact_id.as_str()));
}

#[test]
fn create_and_validate_all_record_types() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let decision = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Adopt Postgres",
            "--rationale",
            "Need ACID",
            "--domain",
            "payments",
            "--status",
            "accepted",
        ],
    );
    assert!(
        decision.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&decision.stderr)
    );

    let fact = run_cli(
        &knowledge,
        &[
            "create",
            "fact",
            "--observation",
            "Queue X saturated",
            "--domain",
            "payments",
            "--confidence",
            "0.95",
        ],
    );
    assert!(
        fact.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&fact.stderr)
    );

    let assumption = run_cli(
        &knowledge,
        &[
            "create",
            "assumption",
            "--assumption-statement",
            "Traffic will grow steadily",
            "--domain",
            "payments",
            "--confidence",
            "0.7",
            "--expire-at",
            "2027-03-31",
        ],
    );
    assert!(
        assumption.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&assumption.stderr)
    );

    let meeting = run_cli(
        &knowledge,
        &[
            "create",
            "meeting",
            "--title",
            "Payments weekly",
            "--date",
            "2026-03-31",
            "--domain",
            "payments",
        ],
    );
    assert!(
        meeting.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&meeting.stderr)
    );

    let action = run_cli(
        &knowledge,
        &[
            "create",
            "action",
            "--title",
            "Tune queue worker",
            "--domain",
            "payments",
            "--status",
            "todo",
            "--due-date",
            "2026-04-15",
        ],
    );
    assert!(
        action.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&action.stderr)
    );

    let person = run_cli(
        &knowledge,
        &["create", "person", "--name", "Alice Example", "--role", "Engineer"],
    );
    assert!(
        person.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&person.stderr)
    );

    let index = read_index(&knowledge);
    assert_eq!(index.len(), 6);

    let expected_types = [
        "Decision",
        "Fact",
        "Assumption",
        "Meeting",
        "Action",
        "Person",
    ];

    for expected in expected_types {
        let count = index
            .iter()
            .filter(|entry| entry.get("type").and_then(Value::as_str) == Some(expected))
            .count();
        assert_eq!(count, 1, "expected exactly one {} record", expected);
    }

    for entry in &index {
        let record_path = knowledge.join(
            entry
                .get("path")
                .and_then(Value::as_str)
                .expect("path in index"),
        );
        assert!(record_path.exists(), "record file should exist");
        validate_file(&knowledge, &record_path);
    }
}

#[test]
fn skill_general_without_subcommand_works() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let output = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "--skill",
        ])
        .output()
        .expect("run skill general");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GUIA GENERAL"), "stdout={stdout}");
    assert!(stdout.contains("Contraejemplos"), "stdout={stdout}");
    assert!(stdout.contains("SKILL CREATE (TODAS LAS VARIANTES)"), "stdout={stdout}");
    assert!(stdout.contains("decision"), "stdout={stdout}");
    assert!(stdout.contains("person"), "stdout={stdout}");
}

#[test]
fn skill_specific_without_subcommand_works() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let output = Command::new(bin())
        .args([
            "--knowledge-dir",
            knowledge.to_str().expect("utf8 path"),
            "--skill",
            "payments",
        ])
        .output()
        .expect("run skill specific");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GUIA SKILL: payments"), "stdout={stdout}");
    assert!(stdout.contains("--domain payments"), "stdout={stdout}");
    assert!(stdout.contains("SKILL CREATE (TODAS LAS VARIANTES)"), "stdout={stdout}");
}

#[test]
fn help_includes_examples_and_counterexamples() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("run help");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("EJEMPLOS"), "stdout={stdout}");
    assert!(stdout.contains("CONTRAEJEMPLOS COMUNES"), "stdout={stdout}");
    assert!(stdout.contains("SKILL CREATE (TODAS LAS VARIANTES)"), "stdout={stdout}");
    assert!(stdout.contains("create person"), "stdout={stdout}");
}

#[test]
fn extended_help_includes_sections_enums_and_restrictions() {
    let output = Command::new(bin())
        .arg("extended-help")
        .output()
        .expect("run extended-help");

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("NAME"), "stdout={stdout}");
    assert!(stdout.contains("SYNOPSIS"), "stdout={stdout}");
    assert!(stdout.contains("ENUMS AND RESTRICTIONS"), "stdout={stdout}");
    assert!(stdout.contains("DATE AND TIME RULES"), "stdout={stdout}");
    assert!(stdout.contains("foundational | tactical | observational"), "stdout={stdout}");
    assert!(stdout.contains("todo | in-progress | blocked | done | cancelled"), "stdout={stdout}");
    assert!(stdout.contains("--depth requerido en rango 1..5"), "stdout={stdout}");
    assert!(stdout.contains("--max-depth en rango 1..20"), "stdout={stdout}");
}

#[test]
fn show_returns_record_json() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Show",
            "--rationale",
            "Validar comando show",
            "--domain",
            "payments",
        ],
    );
    assert!(create.status.success(), "stderr={}", String::from_utf8_lossy(&create.stderr));

    let index = read_index(&knowledge);
    let id = index_id_by_title(&index, "Decision Show");

    let show = run_cli(&knowledge, &["show", "--id", &id]);
    assert!(show.status.success(), "stderr={}", String::from_utf8_lossy(&show.stderr));

    let value: Value = serde_json::from_slice(&show.stdout).expect("show json");
    assert_eq!(value.get("id").and_then(Value::as_str), Some(id.as_str()));
    assert_eq!(value.get("type").and_then(Value::as_str), Some("Decision"));
}

#[test]
fn list_filters_by_domain_and_type() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let c1 = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Payments",
            "--rationale",
            "R1",
            "--domain",
            "payments",
        ],
    );
    assert!(c1.status.success(), "stderr={}", String::from_utf8_lossy(&c1.stderr));

    let c2 = run_cli(
        &knowledge,
        &[
            "create",
            "fact",
            "--observation",
            "Observacion Ops",
            "--domain",
            "ops",
        ],
    );
    assert!(c2.status.success(), "stderr={}", String::from_utf8_lossy(&c2.stderr));

    let list = run_cli(
        &knowledge,
        &["list", "--domain", "payments", "--type", "Decision"],
    );
    assert!(list.status.success(), "stderr={}", String::from_utf8_lossy(&list.stderr));

    let rows: Vec<Value> = serde_json::from_slice(&list.stdout).expect("list json");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("type").and_then(Value::as_str), Some("Decision"));
    assert_eq!(rows[0].get("domain").and_then(Value::as_str), Some("payments"));
}

#[test]
fn links_query_returns_outgoing_edges() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let a = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision A",
            "--rationale",
            "A",
            "--domain",
            "payments",
        ],
    );
    assert!(a.status.success(), "stderr={}", String::from_utf8_lossy(&a.stderr));
    let b = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision B",
            "--rationale",
            "B",
            "--domain",
            "payments",
        ],
    );
    assert!(b.status.success(), "stderr={}", String::from_utf8_lossy(&b.stderr));

    let index = read_index(&knowledge);
    let id_a = index_id_by_title(&index, "Decision A");
    let id_b = index_id_by_title(&index, "Decision B");

    let link = run_cli(
        &knowledge,
        &["link", "--from", &id_a, "--to", &id_b, "--relation", "relatesTo"],
    );
    assert!(link.status.success(), "stderr={}", String::from_utf8_lossy(&link.stderr));

    let links = run_cli(&knowledge, &["links", "--from", &id_a]);
    assert!(links.status.success(), "stderr={}", String::from_utf8_lossy(&links.stderr));

    let edges: Vec<Value> = serde_json::from_slice(&links.stdout).expect("links json");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].get("from").and_then(Value::as_str), Some(id_a.as_str()));
    assert_eq!(edges[0].get("to").and_then(Value::as_str), Some(id_b.as_str()));
    assert_eq!(edges[0].get("relation").and_then(Value::as_str), Some("relatesTo"));
}

#[test]
fn neighbors_depth_finds_two_hops() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    for title in ["N-A", "N-B", "N-C"] {
        let out = run_cli(
            &knowledge,
            &[
                "create",
                "decision",
                "--title",
                title,
                "--rationale",
                "R",
                "--domain",
                "payments",
            ],
        );
        assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    }

    let index = read_index(&knowledge);
    let a = index_id_by_title(&index, "N-A");
    let b = index_id_by_title(&index, "N-B");
    let c = index_id_by_title(&index, "N-C");

    let l1 = run_cli(&knowledge, &["link", "--from", &a, "--to", &b, "--relation", "relatesTo"]);
    assert!(l1.status.success(), "stderr={}", String::from_utf8_lossy(&l1.stderr));
    let l2 = run_cli(&knowledge, &["link", "--from", &b, "--to", &c, "--relation", "relatesTo"]);
    assert!(l2.status.success(), "stderr={}", String::from_utf8_lossy(&l2.stderr));

    let out = run_cli(&knowledge, &["neighbors", "--id", &a, "--depth", "2"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let payload: Value = serde_json::from_slice(&out.stdout).expect("neighbors json");
    let neighbors = payload
        .get("neighbors")
        .and_then(Value::as_array)
        .expect("neighbors array");
    let ids: Vec<&str> = neighbors
        .iter()
        .filter_map(|n| n.get("id").and_then(Value::as_str))
        .collect();
    assert!(ids.contains(&b.as_str()));
    assert!(ids.contains(&c.as_str()));
}

#[test]
fn search_finds_record_by_text() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let fact = run_cli(
        &knowledge,
        &[
            "create",
            "fact",
            "--observation",
            "Rabbitmq se saturo en horario punta",
            "--domain",
            "payments",
        ],
    );
    assert!(fact.status.success(), "stderr={}", String::from_utf8_lossy(&fact.stderr));

    let out = run_cli(&knowledge, &["search", "--query", "rabbitmq", "--in", "Fact"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let rows: Vec<Value> = serde_json::from_slice(&out.stdout).expect("search json");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("type").and_then(Value::as_str), Some("Fact"));
}

#[test]
fn validate_graph_fails_with_broken_reference() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Broken",
            "--rationale",
            "R",
            "--domain",
            "payments",
        ],
    );
    assert!(create.status.success(), "stderr={}", String::from_utf8_lossy(&create.stderr));

    let index = read_index(&knowledge);
    let id = index_id_by_title(&index, "Decision Broken");
    let rel_path = index_path_by_id(&index, &id);
    let full_path = knowledge.join(rel_path);

    let raw = fs::read_to_string(&full_path).expect("read record");
    let mut value: Value = serde_json::from_str(&raw).expect("record json");
    value["links"] = serde_json::json!([
        {
            "relation": "relatesTo",
            "target": "urn:log:payments:does-not-exist",
            "recorded_at": "2026-03-31T00:00:00Z"
        }
    ]);
    fs::write(&full_path, serde_json::to_vec_pretty(&value).expect("serialize")).expect("write record");

    let out = run_cli(&knowledge, &["validate-graph", "--report", "json"]);
    assert!(!out.status.success(), "stdout={}", String::from_utf8_lossy(&out.stdout));

    let report: Value = serde_json::from_slice(&out.stdout).expect("report json");
    let broken = report
        .get("broken_references")
        .and_then(Value::as_array)
        .expect("broken refs array");
    assert_eq!(broken.len(), 1);
}

#[test]
fn path_finds_shortest_route() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    for title in ["P-A", "P-B", "P-C"] {
        let out = run_cli(
            &knowledge,
            &[
                "create",
                "decision",
                "--title",
                title,
                "--rationale",
                "R",
                "--domain",
                "payments",
            ],
        );
        assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    }

    let index = read_index(&knowledge);
    let a = index_id_by_title(&index, "P-A");
    let b = index_id_by_title(&index, "P-B");
    let c = index_id_by_title(&index, "P-C");

    let l1 = run_cli(&knowledge, &["link", "--from", &a, "--to", &b, "--relation", "dependsOn"]);
    assert!(l1.status.success(), "stderr={}", String::from_utf8_lossy(&l1.stderr));
    let l2 = run_cli(&knowledge, &["link", "--from", &b, "--to", &c, "--relation", "dependsOn"]);
    assert!(l2.status.success(), "stderr={}", String::from_utf8_lossy(&l2.stderr));

    let out = run_cli(
        &knowledge,
        &["path", "--from", &a, "--to", &c, "--max-depth", "5"],
    );
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let payload: Value = serde_json::from_slice(&out.stdout).expect("path json");
    assert_eq!(payload.get("found").and_then(Value::as_bool), Some(true));
    assert_eq!(payload.get("hops").and_then(Value::as_u64), Some(2));
    let path = payload.get("path").and_then(Value::as_array).expect("path array");
    assert_eq!(path.len(), 3);
}

#[test]
fn create_decision_supports_advanced_ontology_fields() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Ontology",
            "--rationale",
            "Necesitamos trazabilidad",
            "--project",
            "payments",
            "--body",
            "Descripcion desde alias body",
            "--classification",
            "tactical",
            "--related-files",
            "src/main.rs,docs/adr.md",
            "--evidence-json",
            "{\"type\":\"issue\",\"reference\":\"#42\"}",
            "--outcomes-json",
            "{\"status\":\"success\",\"notes\":\"ok\"}",
            "--provenance-json",
            "{\"source_tool\":\"knowledge-cli\",\"recorded_via\":\"manual\"}",
        ],
    );
    assert!(create.status.success(), "stderr={}", String::from_utf8_lossy(&create.stderr));

    let index = read_index(&knowledge);
    let id = index_id_by_title(&index, "Decision Ontology");
    let rel_path = index_path_by_id(&index, &id);
    let full_path = knowledge.join(rel_path);
    validate_file(&knowledge, &full_path);

    let raw = fs::read_to_string(&full_path).expect("read record");
    let record: Value = serde_json::from_str(&raw).expect("record json");

    assert_eq!(record.get("domain").and_then(Value::as_str), Some("payments"));
    assert_eq!(
        record.get("description").and_then(Value::as_str),
        Some("Descripcion desde alias body")
    );
    assert_eq!(record.get("classification").and_then(Value::as_str), Some("tactical"));

    let related_files = record
        .get("related_files")
        .and_then(Value::as_array)
        .expect("related_files array");
    assert_eq!(related_files.len(), 2);

    let evidence = record
        .get("evidence")
        .and_then(Value::as_array)
        .expect("evidence array");
    assert_eq!(evidence[0].get("type").and_then(Value::as_str), Some("issue"));
    assert_eq!(evidence[0].get("reference").and_then(Value::as_str), Some("#42"));

    let outcomes = record
        .get("outcomes")
        .and_then(Value::as_array)
        .expect("outcomes array");
    assert_eq!(outcomes[0].get("status").and_then(Value::as_str), Some("success"));

    let provenance = record
        .get("provenance")
        .and_then(Value::as_object)
        .expect("provenance object");
    assert_eq!(
        provenance.get("source_tool").and_then(Value::as_str),
        Some("knowledge-cli")
    );
}

#[test]
fn create_action_supports_single_outcome_json() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "action",
            "--title",
            "Action With Outcome",
            "--domain",
            "ops",
            "--outcome-json",
            "{\"status\":\"partial\",\"notes\":\"pendiente\"}",
        ],
    );
    assert!(create.status.success(), "stderr={}", String::from_utf8_lossy(&create.stderr));

    let index = read_index(&knowledge);
    let id = index_id_by_title(&index, "Action With Outcome");
    let rel_path = index_path_by_id(&index, &id);
    let full_path = knowledge.join(rel_path);
    validate_file(&knowledge, &full_path);

    let raw = fs::read_to_string(&full_path).expect("read action");
    let record: Value = serde_json::from_str(&raw).expect("action json");
    let outcome = record
        .get("outcome")
        .and_then(Value::as_object)
        .expect("outcome object");
    assert_eq!(outcome.get("status").and_then(Value::as_str), Some("partial"));
}

#[test]
fn create_with_invalid_classification_fails() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Invalid",
            "--rationale",
            "R",
            "--classification",
            "invalid-class",
        ],
    );

    assert!(
        !create.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&create.stdout),
        String::from_utf8_lossy(&create.stderr)
    );

    let stderr = String::from_utf8_lossy(&create.stderr);
    assert!(stderr.contains("classification invalido"), "stderr={stderr}");
}

#[test]
fn create_with_invalid_evidence_fails() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let create = run_cli(
        &knowledge,
        &[
            "create",
            "fact",
            "--observation",
            "Fact invalid evidence",
            "--evidence-json",
            "{}",
        ],
    );

    assert!(
        !create.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&create.stdout),
        String::from_utf8_lossy(&create.stderr)
    );

    let stderr = String::from_utf8_lossy(&create.stderr);
    assert!(stderr.contains("--evidence-json invalido"), "stderr={stderr}");
}

#[test]
fn list_can_filter_by_classification() {
    let tmp = tempdir().expect("tmp dir");
    let knowledge = tmp.path().join("knowledge");

    let c1 = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Tactical",
            "--rationale",
            "R1",
            "--classification",
            "tactical",
        ],
    );
    assert!(c1.status.success(), "stderr={}", String::from_utf8_lossy(&c1.stderr));

    let c2 = run_cli(
        &knowledge,
        &[
            "create",
            "decision",
            "--title",
            "Decision Foundational",
            "--rationale",
            "R2",
            "--classification",
            "foundational",
        ],
    );
    assert!(c2.status.success(), "stderr={}", String::from_utf8_lossy(&c2.stderr));

    let list = run_cli(&knowledge, &["list", "--classification", "tactical"]);
    assert!(list.status.success(), "stderr={}", String::from_utf8_lossy(&list.stderr));

    let rows: Vec<Value> = serde_json::from_slice(&list.stdout).expect("list json");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("classification").and_then(Value::as_str), Some("tactical"));
}
