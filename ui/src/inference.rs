use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Represents the result of an inference call.
pub enum InferenceResult {
    Ok(String),
    Err(String),
}

/// Call `pi --mode rpc` to ask the agent to review mulch and suggest a rationale based on the form.
/// This function is synchronous and returns either the rationale text or an error message.
pub fn infer_rationale_from_form(form: &crate::state::Form) -> InferenceResult {
    // Build a prompt summarizing the form and asking the assistant to search mulch for related notes
    let prompt = format!(
        "Revisa la base de conocimiento Mulch en el repo (usa el contexto del repositorio) y busca notas relacionadas con el siguiente registro. Devuelve solo un texto conciso para el campo 'Rationale'.\n\nForm:\nTitle: {title}\nDomain: {domain}\nDate: {date}\nParticipants: {participants}\nFiles: {files}\nTags: {tags}\nDescription:\n{description}\n\nRespuesta esperada: un único string con la rationale.\n",
        title = form.title,
        domain = form.domain,
        date = form.date,
        participants = form.participants,
        files = form.files,
        tags = form.tags,
        description = form.description
    );

    // Spawn pi
    let mut cmd = Command::new("pi");
    cmd.arg("--mode").arg("rpc").arg("--no-session");
    // ensure we don't inherit stdio
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return InferenceResult::Err(format!("failed to spawn pi: {}", e)),
    };

    // Write the prompt as JSONL
    if let Some(mut stdin) = child.stdin.take() {
        let obj = serde_json::json!({ "type": "prompt", "message": prompt });
        let s = obj.to_string() + "\n";
        let _ = stdin.write_all(s.as_bytes());
        // close stdin to signal end
    }

    // Read stdout lines until we find an assistant message containing the rationale
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(20);

    while start.elapsed() < timeout {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                // try parse JSON
                if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
                    if let Some(t) = v.get("type").and_then(|x| x.as_str()) {
                        if t == "message_update" || t == "message_end" {
                            if let Some(arr) = v
                                .get("assistantMessageEvent")
                                .and_then(|ame| ame.get("partial"))
                                .and_then(|p| p.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                let mut collected = String::new();
                                for block in arr {
                                    if block.get("type").and_then(|x| x.as_str()) == Some("text") {
                                        if let Some(txt) = block.get("text").and_then(|x| x.as_str()) {
                                            collected.push_str(txt);
                                        }
                                    }
                                }
                                // heuristics: if collected is non-empty, return it
                                if !collected.trim().is_empty() {
                                    let _ = child.kill();
                                    let _ = child.wait();
                                    return InferenceResult::Ok(collected.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => return InferenceResult::Err(format!("read error: {}", e)),
        }
    }

    // timeout
    let _ = child.kill();
    let _ = child.wait();
    InferenceResult::Err("timeout waiting for rationale".into())
}
