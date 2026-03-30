use std::io::BufRead;
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn pi_rpc_ping_pong() {
    // Find nearest parent directory that contains .mulch, starting from current dir
    let mut dir = std::env::current_dir().expect("cwd");
    let mut found = None;
    loop {
        let candidate = dir.join(".mulch");
        if candidate.exists() {
            found = Some(dir.clone());
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    let repo_dir = match found {
        Some(d) => d,
        None => panic!(
            "could not find .mulch in any parent directory; please run test from inside the repo"
        ),
    };

    let pi_cmd = "pi";
    let mut proc = match Command::new(pi_cmd)
        .current_dir(&repo_dir)
        .arg("--mode")
        .arg("rpc")
        .arg("--no-session")
        .arg("--model")
        .arg("github-copilot/gpt-4.1")
        // do not enable DEBUG to avoid changing agent behavior; keep NODE_OPTIONS minimal
        .env("NODE_OPTIONS", "--trace-warnings --trace-uncaught")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(p) => p,
        Err(e) => panic!(
            "could not spawn 'pi' from PATH with cwd {:?}: {e}",
            repo_dir
        ),
    };

    // take stdout and stderr and spawn threads to read lines continuously
    let stdout = proc.stdout.take().expect("have stdout");
    let stderr = proc.stderr.take().expect("have stderr");

    // channel to collect stdout lines
    let (out_tx, out_rx) = std::sync::mpsc::channel::<String>();
    let _out_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        loop {
            let mut l = String::new();
            match reader.read_line(&mut l) {
                Ok(0) => {
                    let _ = out_tx.send("STDOUT_EOF".to_string());
                    break;
                }
                Ok(_) => {
                    if let Some(s) = l.strip_suffix('\n') {
                        let t = s.trim().to_string();
                        let _ = out_tx.send(t);
                    } else {
                        let t = l.trim().to_string();
                        let _ = out_tx.send(t);
                    }
                }
                Err(e) => {
                    let _ = out_tx.send(format!("STDOUT_ERR: {e}"));
                    break;
                }
            }
        }
    });

    // stderr thread collects lines into a Vec and returns it
    let stderr_handle = std::thread::spawn(move || {
        let mut sreader = std::io::BufReader::new(stderr);
        let mut lines = Vec::new();
        loop {
            let mut l = String::new();
            match sreader.read_line(&mut l) {
                Ok(0) => {
                    lines.push("STDERR_EOF".to_string());
                    break;
                }
                Ok(_) => {
                    let t = l.trim().to_string();
                    if !t.is_empty() {
                        lines.push(format!("STDERR: {t}"));
                    }
                }
                Err(e) => {
                    lines.push(format!("STDERR_ERR: {e}"));
                    break;
                }
            }
        }
        lines
    });

    // wait 3 seconds before sending prompt (as requested)
    std::thread::sleep(std::time::Duration::from_secs(3));

    // write diagnostic: moment of command creation, working dir, and args to the log buffer
    let mut log_buf = Vec::new();
    log_buf.push(format!("CMD_CREATED: {}", "pi"));
    log_buf.push(format!("WORKDIR: {}", repo_dir.display()));
    log_buf.push(format!("ARGS: {}", "--mode rpc --no-session"));

    // send a minimal ping prompt
    let req = serde_json::json!({"type":"prompt","message":"ping"});
    let line = serde_json::to_string(&req).unwrap() + "\n";
    if let Some(mut stdin) = proc.stdin.take() {
        let _ = stdin.write_all(line.as_bytes());
        let _ = stdin.flush();
        log_buf.push("SENT_PROMPT".to_string());
    }

    // read stdout lines from channel
    let start = std::time::Instant::now();
    let mut saw_pong = false;
    while start.elapsed() < Duration::from_secs(30) {
        match out_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(trimmed) => {
                if trimmed.is_empty() {
                    continue;
                }
                log_buf.push(format!("OUT: {trimmed}"));
                if trimmed == "STDOUT_EOF" {
                    log_buf.push("BREAK: stdout EOF".to_string());
                    break;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                    if v.get("type").and_then(|t| t.as_str()) == Some("message_update") {
                        if let Some(ame) = v.get("assistantMessageEvent") {
                            if let Some(typ) = ame.get("type").and_then(|t| t.as_str()) {
                                if typ == "text_delta" || typ == "text_end" {
                                    if let Some(delta) = ame.get("delta").and_then(|d| d.as_str()) {
                                        if delta.to_lowercase().contains("pong") {
                                            log_buf.push("FOUND pong in text_delta".to_string());
                                            saw_pong = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if v.get("type").and_then(|t| t.as_str()) == Some("agent_end") {
                        if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                            for msg in msgs {
                                if msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                                    if let Some(content) =
                                        msg.get("content").and_then(|c| c.as_array())
                                    {
                                        for block in content {
                                            if let Some(text) =
                                                block.get("text").and_then(|t| t.as_str())
                                            {
                                                if text.to_lowercase().contains("pong") {
                                                    log_buf.push(
                                                        "FOUND pong in agent_end".to_string(),
                                                    );
                                                    saw_pong = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                if saw_pong {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    if trimmed.to_lowercase().contains("pong") {
                        log_buf.push("FOUND pong in raw line".to_string());
                        saw_pong = true;
                        break;
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => { /* loop until timeout */ }
            Err(e) => {
                log_buf.push(format!("recv err: {e}"));
                break;
            }
        }
    }

    // collect stderr lines from thread
    if let Ok(stderr_lines) = stderr_handle.join() {
        for l in stderr_lines {
            log_buf.push(l);
        }
    } else {
        log_buf.push("stderr thread panicked".to_string());
    }

    // write log: dump structured IN/OUT to file for offline inspection
    let mut f = std::fs::File::create("/tmp/pi_rpc_ping_full.log").expect("create log");
    use std::io::Write as _;
    let _ = writeln!(f, "in: {}", serde_json::to_string(&req).unwrap());
    for line in &log_buf {
        let _ = writeln!(f, "out: {line}");
    }

    let _ = proc.kill();

    // reap child
    match proc.wait() {
        Ok(st) => log_buf.push(format!("waited and got status: {:?}", st)),
        Err(e) => log_buf.push(format!("wait error: {e}")),
    }

    // also print path for user
    println!("Wrote full RPC log to /tmp/pi_rpc_ping_full.log");

    assert!(
        saw_pong,
        "expected to see 'pong' in assistant messages within timeout"
    );
}
