use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Call `pi --mode rpc` to ask the agent to review mulch and suggest a rationale based on the form.
/// This function is synchronous and returns either the parsed JSON Value from the assistant or an error string.
pub fn infer_rationale_from_form(form: &crate::state::Form) -> Result<serde_json::Value, String> {
    // Build a prompt summarizing the form and asking the assistant to search mulch for related notes
    let prompt = format!(
        "Busca notas relacionadas con el registro a otorgar despues de las instrucciones a continuación.\n\nDevuelve solo un texto conciso para el campo 'Rationale'.\n\nForm:\nTitle: {title}\nDomain: {domain}\nDate: {date}\nParticipants: {participants}\nFiles: {files}\nTags: {tags}\nDescription:\n{description}\n\nRespuesta esperada: un único string con la rationale.\n",
        title = form.title,
        domain = form.domain,
        date = form.date,
        participants = form.participants,
        files = form.files,
        tags = form.tags,
        description = form.description
    );

    // Detectar si existe .mulch en el repo (no abortamos si falta; lo incluimos en el prompt)
    let mut dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut mulch_found = false;
    while dir.parent().is_some() {
        if dir.join(".mulch").exists() {
            mulch_found = true;
            break;
        }
        dir = dir.parent().unwrap().to_path_buf();
    }

    // Recolectar coincidencias locales básicas (ruta + snippet) para dar contexto cuando no haya mulch
    let mut local_matches: Vec<serde_json::Value> = Vec::new();
    // keywords: domain + words from title + words from description (simple split)
    let mut keywords: Vec<String> = Vec::new();
    if !form.domain.trim().is_empty() {
        keywords.push(form.domain.to_lowercase());
    }
    for part in form
        .title
        .split_whitespace()
        .chain(form.description.split_whitespace())
    {
        let p = part
            .trim()
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if p.len() >= 3 && !keywords.contains(&p) {
            keywords.push(p);
        }
        if keywords.len() >= 8 {
            break;
        }
    }

    // walk repository tree (depth-first), limited to 10 matches
    let mut matches_found = 0usize;
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut stack = vec![cwd.clone()];
    while let Some(path) = stack.pop() {
        if matches_found >= 10 {
            break;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            // skip ignored dirs
            if let Some(n) = path.file_name().and_then(|s| s.to_str()) {
                if n == "target" || n == ".git" || n == ".mulch" || n == "node_modules" {
                    continue;
                }
            }
            if let Ok(rd) = std::fs::read_dir(&path) {
                for e in rd.filter_map(Result::ok) {
                    stack.push(e.path());
                }
            }
        } else if meta.is_file() {
            // try read as text
            if let Ok(s) = std::fs::read_to_string(&path) {
                for (lineno, line) in s.lines().enumerate() {
                    let low = line.to_lowercase();
                    for kw in &keywords {
                        if low.contains(kw) {
                            // create snippet truncated
                            let snippet = if line.len() > 200 {
                                format!("{}...", &line[..200])
                            } else {
                                line.to_string()
                            };
                            local_matches.push(serde_json::json!({
                                "path": path.to_string_lossy().to_string(),
                                "lineno": lineno + 1,
                                "snippet": snippet,
                            }));
                            matches_found += 1;
                            break;
                        }
                    }
                    if matches_found >= 10 {
                        break;
                    }
                }
            }
        }
    }

    // write local matches to /tmp for debugging
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/inference_local_matches.log")
        .map(|mut f| {
            use std::io::Write;
            let _ = writeln!(f, "mulch_found: {mulch_found}");
            let _ = writeln!(f, "keywords: {keywords:?}");
            let _ = writeln!(
                f,
                "local_matches: {}",
                serde_json::to_string_pretty(&local_matches).unwrap_or_default()
            );
        });

    // Spawn pi
    let mut cmd = Command::new("pi");
    cmd.current_dir(&dir)
        .arg("--mode")
        .arg("rpc")
        .arg("--no-session");
    // ensure we don't inherit stdio
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Err(format!("failed to spawn pi: {e}")),
    };

    // Run `ml prime` in the repo to capture Mulch context, include its output (truncated) in the prompt
    let mut prime_output = String::new();
    match Command::new("ml").arg("prime").current_dir(&dir).output() {
        Ok(out) => {
            prime_output = String::from_utf8_lossy(&out.stdout).to_string();
            if prime_output.trim().is_empty() {
                // capture stderr if no stdout
                prime_output = String::from_utf8_lossy(&out.stderr).to_string();
            }
        }
        Err(e) => {
            prime_output = format!("(failed to run ml prime: {})", e);
        }
    }
    // remove the session-close protocol section if present
    if let Some(idx) = prime_output.find("# 🚨 SESSION CLOSE PROTOCOL 🚨") {
        prime_output.truncate(idx);
    }

    // build full prompt including prime output
    let mut prompt_full = prompt.clone();
    prompt_full.push_str(&format!("\nMulchPrimeOutput:\n{}\n", prime_output));
    prompt_full.push_str("Nota: `ml prime` ya fue ejecutado y su salida se incluye arriba. NO ejecutes `ml prime` de nuevo.\n");
    prompt_full.push_str(&format!(
        "\nMetadata:\n- mulch_initialized: {}\n- local_matches_count: {}\n",
        mulch_found,
        local_matches.len()
    ));
    if !local_matches.is_empty() {
        prompt_full.push_str("LocalMatches:\n");
        for m in &local_matches {
            let path = m.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let lineno = m.get("lineno").and_then(|n| n.as_u64()).unwrap_or(0);
            let snippet = m.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
            prompt_full.push_str(&format!("- {path}:{lineno}: {snippet}\n"));
        }
    }
    prompt_full.push_str("\nReglas: Si no encuentras registros en Mulch, realiza una búsqueda directa en el árbol del repo. Devuelve SOLO un JSON válido con las claves: text, ids, sources, confidence, rationale_notes, actions, metadata, error. metadata debe incluir 'mulch_initialized' y 'local_matches_count'.\n");

    // take child stdin and keep it so we can send follow-ups if needed
    let mut child_stdin = child.stdin.take();

    // send initial prompt_full to the agent
    if let Some(stdin) = child_stdin.as_mut() {
        let obj = serde_json::json!({ "type": "prompt", "message": prompt_full });
        let s = obj.to_string() + "\n";
        let _ = stdin.write_all(s.as_bytes());
        let _ = stdin.flush();
        // keep stdin open for follow-ups
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

    // upon seeing agent_end, try to parse result; if not usable, send a follow-up prompt in the same session
    if saw_agent_end {
        if !collected.trim().is_empty() {
            // try parse collected as JSON
            match serde_json::from_str::<Value>(collected.trim()) {
                Ok(j) => {
                    // if JSON contains sources or ids, accept it
                    let has_sources = j.get("sources").and_then(|s| s.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
                    let has_ids = j.get("ids").and_then(|i| i.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false);
                    if has_sources || has_ids {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(j);
                    }
                    // otherwise we'll attempt a follow-up
                }
                Err(_) => {
                    // parse failed; we'll attempt a follow-up
                }
            }

            // build follow-up prompt asking to convert previous text into the exact JSON schema
            let followup = format!(
                "Por favor, convierte la respuesta anterior exactamente a un JSON válido con las claves: text, ids, sources, confidence, rationale_notes, actions, metadata, error. Si no encontraste ids, ids debe ser \"\" y confidence 0. Incluye metadata.mulch_initialized={} y metadata.local_matches_count={}. Si no hay sources, incluye suggested_queries en metadata. Devuelve SOLO JSON y nada más.",
                mulch_found, local_matches.len()
            );

            if let Some(stdin) = child_stdin.as_mut() {
                let obj = serde_json::json!({ "type": "prompt", "message": followup });
                let s = obj.to_string() + "\n";
                let _ = stdin.write_all(s.as_bytes());
                let _ = stdin.flush();
                // wait up to extra_timeout for a second agent_end
                let extra_start = std::time::Instant::now();
                let extra_timeout = Duration::from_secs(20);
                let mut second_collected = String::new();
                while extra_start.elapsed() < extra_timeout {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
                                if v.get("type").and_then(|x| x.as_str()) == Some("agent_end") {
                                    // log
                                    let _ = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open("/tmp/inference_followup.log")
                                        .and_then(|mut f| {
                                            use std::io::Write;
                                            writeln!(f, "FOLLOWUP_AGENT_END_RAW: {}", serde_json::to_string(&v).unwrap_or_default())
                                        });
                                    if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                                        for msg in msgs {
                                            if msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                                                if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                                                    for block in content {
                                                        if block.get("type").and_then(|x| x.as_str()) == Some("text") {
                                                            if let Some(txt) = block.get("text").and_then(|x| x.as_str()) {
                                                                second_collected.push_str(txt);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }

                // after follow-up wait, try parse second_collected
                if !second_collected.trim().is_empty() {
                    if let Ok(j2) = serde_json::from_str::<Value>(second_collected.trim()) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(j2);
                    } else {
                        // write to raw log and fallback
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/inference_agent_end_raw_text.log")
                            .and_then(|mut f| {
                                use std::io::Write;
                                let _ = writeln!(f, "FOLLOWUP_PARSE_ERR");
                                writeln!(f, "SECOND_COLLECTED: {}", second_collected)
                            });
                        let fallback = serde_json::json!({
                            "text": second_collected.trim(),
                            "ids": "",
                            "sources": [],
                            "confidence": 0.0,
                            "rationale_notes": "",
                            "actions": [],
                            "metadata": { "note": "fallback_from_followup" },
                            "error": null
                        });
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(fallback);
                    }
                }
            }

            // if we couldn't send followup (stdin closed) or no useful reply, fallback to wrapping original collected
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/inference_agent_end_raw_text.log")
                .and_then(|mut f| {
                    use std::io::Write;
                    let _ = writeln!(f, "PARSE_ERR and no followup reply");
                    writeln!(f, "COLLECTED_TEXT: {}", collected)
                });
            let fallback = serde_json::json!({
                "text": collected.trim(),
                "ids": "",
                "sources": [],
                "confidence": 0.0,
                "rationale_notes": "",
                "actions": [],
                "metadata": { "note": "fallback_from_plain_text_no_followup" },
                "error": null
            });
            let _ = child.kill();
            let _ = child.wait();
            return Ok(fallback);
        } else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("agent finished but no text collected".into());
        }
    }

    // timeout path: try to terminate child
    let _ = child.kill();
    let _ = child.wait();
    Err("timeout waiting for agent_end".into())
}
