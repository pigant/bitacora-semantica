use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDate};
use serde_json::Value;

const RECORD_STATUS: &[&str] = &[
    "proposed",
    "accepted",
    "rejected",
    "in-progress",
    "done",
    "deprecated",
];
const ACTION_STATUS: &[&str] = &["todo", "in-progress", "blocked", "done", "cancelled"];
const IMPACT_STATUS: &[&str] = &["low", "medium", "high"];
const CLASSIFICATION_STATUS: &[&str] = &["foundational", "tactical", "observational"];
const EVIDENCE_TYPES: &[&str] = &["commit", "file", "issue", "screenshot", "link", "bead"];
const OUTCOME_STATUS: &[&str] = &["success", "failure", "partial"];
const RELATIONS: &[&str] = &[
    "relatesTo",
    "supersedes",
    "references",
    "dependsOn",
    "recordedInMeeting",
    "derivedFrom",
    "confirms",
    "contradicts",
    "actionFor",
    "assignedTo",
];

pub fn validate_record_json(record: &Value) -> Result<()> {
    let record_type = required_str(record, "type")?;

    if let Some(recorded_at) = optional_str(record, "recorded_at") {
        validate_iso_datetime(recorded_at, "recorded_at")?;
    }
    if let Some(confidence_value) = record.get("confidence") {
        let confidence = confidence_value
            .as_f64()
            .ok_or_else(|| anyhow!("confidence debe ser numerico entre 0.0 y 1.0"))?;
        if !(0.0..=1.0).contains(&confidence) {
            return Err(anyhow!("confidence debe estar entre 0.0 y 1.0"));
        }
    }
    if let Some(impact) = optional_str(record, "impact") {
        validate_in_enum(impact, IMPACT_STATUS, "impact")?;
    }
    if let Some(classification) = optional_str(record, "classification") {
        validate_in_enum(classification, CLASSIFICATION_STATUS, "classification")?;
    }

    validate_string_array(record, "related_files")?;
    validate_evidence_array(record)?;
    validate_outcomes_array(record)?;
    validate_provenance(record)?;

    match record_type {
        "Decision" => {
            required_str(record, "title")?;
            required_str(record, "rationale")?;
            if let Some(status) = optional_str(record, "status") {
                validate_in_enum(status, RECORD_STATUS, "status")?;
            }
            if let Some(effective_from) = optional_str(record, "effective_from") {
                validate_date_or_datetime(effective_from, "effective_from")?;
            }
        }
        "Fact" => {
            required_str(record, "observation")?;
            validate_string_array(record, "related_components")?;
        }
        "Assumption" => {
            required_str(record, "assumption_statement")?;
            if let Some(expire_at) = optional_str(record, "expire_at") {
                validate_date_or_datetime(expire_at, "expire_at")?;
            }
        }
        "Meeting" => {
            required_str(record, "title")?;
            let date = required_str(record, "date")?;
            validate_date_or_datetime(date, "date")?;
        }
        "Action" => {
            required_str(record, "title")?;
            if let Some(status) = optional_str(record, "status") {
                validate_in_enum(status, ACTION_STATUS, "status")?;
            }
            if let Some(due_date) = optional_str(record, "due_date") {
                validate_date_or_datetime(due_date, "due_date")?;
            }
            if let Some(outcome) = record.get("outcome") {
                validate_outcome(outcome, "outcome")?;
            }
        }
        "Person" => {
            required_str(record, "name")?;
            required_str(record, "id")?;
        }
        other => return Err(anyhow!("type no soportado: {other}")),
    }

    Ok(())
}

pub fn validate_relation(relation: &str) -> Result<()> {
    validate_in_enum(relation, RELATIONS, "relation")
}

pub fn validate_iso_datetime(value: &str, field: &str) -> Result<()> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| anyhow!("{field} debe ser ISO-8601 (RFC3339): {value}"))
}

pub fn validate_date_or_datetime(value: &str, field: &str) -> Result<()> {
    if DateTime::parse_from_rfc3339(value).is_ok() {
        return Ok(());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| anyhow!("{field} debe ser YYYY-MM-DD o ISO-8601: {value}"))
}

pub fn extract_search_text(record: &Value) -> String {
    let mut chunks = Vec::new();

    let string_fields = [
        "id",
        "type",
        "title",
        "name",
        "domain",
        "project",
        "description",
        "body",
        "rationale",
        "observation",
        "assumption_statement",
        "minutes",
        "consequences",
        "status",
        "author",
        "classification",
        "location",
        "assigned_to",
    ];

    for key in string_fields {
        if let Some(value) = record.get(key).and_then(|v| v.as_str()) {
            chunks.push(value.to_ascii_lowercase());
        }
    }

    let list_fields = [
        "tags",
        "options_considered",
        "impacted_components",
        "tests_needed",
        "participants",
        "decisions_made",
        "actions",
        "related_files",
        "related_components",
    ];

    for key in list_fields {
        if let Some(values) = record.get(key).and_then(|v| v.as_array()) {
            for value in values {
                if let Some(s) = value.as_str() {
                    chunks.push(s.to_ascii_lowercase());
                }
            }
        }
    }

    if let Some(provenance) = record.get("provenance").and_then(Value::as_object) {
        for key in ["source_tool", "recorded_via", "recorded_by"] {
            if let Some(value) = provenance.get(key).and_then(Value::as_str) {
                chunks.push(value.to_ascii_lowercase());
            }
        }
    }

    if let Some(evidence) = record.get("evidence").and_then(Value::as_array) {
        for item in evidence {
            if let Some(obj) = item.as_object() {
                for key in ["type", "reference", "excerpt", "recorded_by"] {
                    if let Some(value) = obj.get(key).and_then(Value::as_str) {
                        chunks.push(value.to_ascii_lowercase());
                    }
                }
            }
        }
    }

    if let Some(outcomes) = record.get("outcomes").and_then(Value::as_array) {
        for item in outcomes {
            if let Some(obj) = item.as_object() {
                for key in ["status", "notes", "agent"] {
                    if let Some(value) = obj.get(key).and_then(Value::as_str) {
                        chunks.push(value.to_ascii_lowercase());
                    }
                }
            }
        }
    }

    if let Some(outcome) = record.get("outcome").and_then(Value::as_object) {
        for key in ["status", "notes", "agent"] {
            if let Some(value) = outcome.get(key).and_then(Value::as_str) {
                chunks.push(value.to_ascii_lowercase());
            }
        }
    }

    chunks.join(" ")
}

fn validate_string_array(record: &Value, field: &str) -> Result<()> {
    let Some(values) = record.get(field) else {
        return Ok(());
    };

    let array = values
        .as_array()
        .ok_or_else(|| anyhow!("{field} debe ser un arreglo de strings"))?;

    for value in array {
        if value.as_str().is_none() {
            return Err(anyhow!("{field} debe ser un arreglo de strings"));
        }
    }

    Ok(())
}

fn validate_evidence_array(record: &Value) -> Result<()> {
    let Some(values) = record.get("evidence") else {
        return Ok(());
    };

    let array = values
        .as_array()
        .ok_or_else(|| anyhow!("evidence debe ser un arreglo de objetos"))?;

    for (idx, evidence) in array.iter().enumerate() {
        let field = format!("evidence[{idx}]");
        validate_evidence(evidence, &field)?;
    }

    Ok(())
}

fn validate_evidence(evidence: &Value, field: &str) -> Result<()> {
    let obj = evidence
        .as_object()
        .ok_or_else(|| anyhow!("{field} debe ser un objeto"))?;

    let evidence_type = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{field}.type es requerido"))?;
    validate_in_enum(evidence_type, EVIDENCE_TYPES, &format!("{field}.type"))?;

    obj.get("reference")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{field}.reference es requerido"))?;

    if let Some(date) = obj.get("date").and_then(Value::as_str) {
        validate_iso_datetime(date, &format!("{field}.date"))?;
    }

    for key in ["excerpt", "recorded_by"] {
        if let Some(value) = obj.get(key) && value.as_str().is_none() {
            return Err(anyhow!("{field}.{key} debe ser string"));
        }
    }

    Ok(())
}

fn validate_outcomes_array(record: &Value) -> Result<()> {
    let Some(values) = record.get("outcomes") else {
        return Ok(());
    };

    let array = values
        .as_array()
        .ok_or_else(|| anyhow!("outcomes debe ser un arreglo de objetos"))?;

    for (idx, outcome) in array.iter().enumerate() {
        let field = format!("outcomes[{idx}]");
        validate_outcome(outcome, &field)?;
    }

    Ok(())
}

fn validate_outcome(outcome: &Value, field: &str) -> Result<()> {
    let obj = outcome
        .as_object()
        .ok_or_else(|| anyhow!("{field} debe ser un objeto"))?;

    let status = obj
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{field}.status es requerido"))?;
    validate_in_enum(status, OUTCOME_STATUS, &format!("{field}.status"))?;

    if let Some(duration) = obj.get("duration") {
        let n = duration
            .as_f64()
            .ok_or_else(|| anyhow!("{field}.duration debe ser numerico"))?;
        if n < 0.0 {
            return Err(anyhow!("{field}.duration debe ser >= 0"));
        }
    }

    if let Some(recorded_at) = obj.get("recorded_at").and_then(Value::as_str) {
        validate_iso_datetime(recorded_at, &format!("{field}.recorded_at"))?;
    }

    for key in ["notes", "agent"] {
        if let Some(value) = obj.get(key) && value.as_str().is_none() {
            return Err(anyhow!("{field}.{key} debe ser string"));
        }
    }

    Ok(())
}

fn validate_provenance(record: &Value) -> Result<()> {
    let Some(value) = record.get("provenance") else {
        return Ok(());
    };

    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("provenance debe ser un objeto"))?;

    for key in ["source_tool", "recorded_via", "recorded_by"] {
        if let Some(item) = obj.get(key) && item.as_str().is_none() {
            return Err(anyhow!("provenance.{key} debe ser string"));
        }
    }

    Ok(())
}

fn validate_in_enum(value: &str, allowed: &[&str], field: &str) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(anyhow!(
            "{field} invalido: {value}. Permitidos: {}",
            allowed.join(", ")
        ))
    }
}

fn required_str<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("campo requerido faltante o vacio: {key}"))
}

fn optional_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
}
