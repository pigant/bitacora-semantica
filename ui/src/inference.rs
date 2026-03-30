use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Call `pi --mode rpc` to ask the agent to review mulch and suggest a rationale based on the form.
/// This function is synchronous and returns either the parsed JSON Value from the assistant or an error string.
pub fn infer_rationale_from_form(form: &crate::state::Form) -> Result<serde_json::Value, String> {
    // Build a prompt summarizing the form and asking the assistant to search mulch for related notes
    let prompt = format!(
        "1. ejecuta `ml prime`
         2. Busca notas relacionadas con el registro a otorgar despues de las instrucciones a continuación.
         Devuelve solo un texto conciso para el campo 'Rationale'.\n\nForm:\nTitle: {title}\nDomain: {domain}\nDate: {date}\nParticipants: {participants}\nFiles: {files}\nTags: {tags}\nDescription:\n{description}\n\nRespuesta esperada: un único string con la rationale.\n",
        title = form.title,
        domain = form.domain,
        date = form.date,
        participants = form.participants,
        files = form.files,
        tags = form.tags,
        description = form.description
    );

    // Buscar desde el directorio actual hacia arriba para encontrar el path de mulch
    let mut dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut found = false;
    while dir.parent().is_some() {
        if dir.join(".mulch").exists() {
            found = true;
            break;
        }
        dir = dir.parent().unwrap().to_path_buf();
    }
    if !found {
        return Err("no se encontró mulch en el directorio actual ni en los superiores".into());
    }

    // Spawn pi
    let mut cmd = Command::new("pi");
    cmd.arg("--mode").arg("rpc").arg("--no-session");
    // ensure we don't inherit stdio
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Err(format!("failed to spawn pi: {e}")),
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

    let mut collected = String::new();
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(60);
    let mut saw_agent_end = false;

    while start.elapsed() < timeout {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                // try parse JSON
                if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
                    if let Some(t) = v.get("type").and_then(|x| x.as_str()) {
                        // Only consider agent_end; extract messages[] -> content[] -> text blocks
                        if t == "agent_end" {
                            // For debugging, write raw agent_end to /tmp
                            let _ = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("/tmp/inference_agent_end.log")
                                .and_then(|mut f| {
                                    use std::io::Write;
                                    writeln!(
                                        f,
                                        "AGENT_END_RAW: {}",
                                        serde_json::to_string(&v).unwrap_or_default()
                                    )
                                });

                            if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                                for msg in msgs {
                                    if msg.get("role").and_then(|r| r.as_str()) == Some("assistant")
                                    {
                                        if let Some(content) =
                                            msg.get("content").and_then(|c| c.as_array())
                                        {
                                            for block in content {
                                                if block.get("type").and_then(|x| x.as_str())
                                                    == Some("text")
                                                {
                                                    if let Some(txt) =
                                                        block.get("text").and_then(|x| x.as_str())
                                                    {
                                                        collected.push_str(txt);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            saw_agent_end = true;
                            break;
                        }
                    }
                }
            }
            Err(e) => return Err(format!("read error: {e}")),
        }
    }

    // upon seeing agent_end, kill the child process to free resources, then wait for it
    if saw_agent_end {
        let _ = child.kill();
        let _ = child.wait();
        if !collected.trim().is_empty() {
            // try parse collected as JSON
            match serde_json::from_str::<Value>(collected.trim()) {
                Ok(j) => return Ok(j),
                Err(e) => {
                    // write raw to /tmp for debugging
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/inference_agent_end_raw_text.log")
                        .and_then(|mut f| {
                            use std::io::Write;
                            writeln!(f, "COLLECTED_TEXT: {}", collected)
                        });
                    return Err(format!("failed to parse assistant JSON: {}", e));
                }
            }
        } else {
            return Err("agent finished but no text collected".into());
        }
    }

    // timeout path: try to terminate child
    let _ = child.kill();
    let _ = child.wait();
    Err("timeout waiting for agent_end".into())
}
