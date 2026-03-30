use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
fn main() {
    let mut dir = std::env::current_dir().expect("cwd");
    // find .mulch
    loop {
        if dir.join(".mulch").exists() {
            break;
        }
        if !dir.pop() {
            eprintln!("could not find .mulch");
            std::process::exit(2);
        }
    }
    let repo = dir;
    eprintln!("cwd {:?}", repo);
    let mut proc = Command::new("pi")
        .current_dir(&repo)
        .arg("--mode")
        .arg("rpc")
        .arg("--no-session")
        .arg("--model")
        .arg("github-copilot/gpt-4.1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn pi");
    eprintln!("spawned pid {}", proc.id());
    let stdout = proc.stdout.take().expect("stdout");
    let stderr = proc.stderr.take().expect("stderr");
    let mut stdin = proc.stdin.take().expect("stdin");

    // stdout reader thread
    let out_handle = std::thread::spawn(move || {
        let rdr = BufReader::new(stdout);
        for line in rdr.lines() {
            match line {
                Ok(l) => println!("OUT: {}", l),
                Err(e) => {
                    eprintln!("OUT read err: {}", e);
                    break;
                }
            }
        }
        eprintln!("OUT EOF");
    });
    let err_handle = std::thread::spawn(move || {
        let rdr = BufReader::new(stderr);
        for line in rdr.lines() {
            match line {
                Ok(l) => eprintln!("ERR: {}", l),
                Err(e) => {
                    eprintln!("ERR read err: {}", e);
                    break;
                }
            }
        }
        eprintln!("ERR EOF");
    });

    std::thread::sleep(Duration::from_secs(3));
    let req = serde_json::json!({"type":"prompt","message":"ping"});
    let s = serde_json::to_string(&req).unwrap() + "\n";
    stdin.write_all(s.as_bytes()).expect("write stdin");
    stdin.flush().ok();
    eprintln!("sent prompt");

    // wait up to 20s
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if let Ok(Some(status)) = proc.try_wait() {
            eprintln!("child exited: {:?}", status);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // ensure we wait/kill the child to avoid zombies in all code paths
    match proc.try_wait() {
        Ok(Some(_)) => { /* already exited */ }
        Ok(None) => {
            let _ = proc.kill();
            let _ = proc.wait();
        }
        Err(e) => {
            eprintln!("error trying to wait/kill child: {}", e);
        }
    }

    let _ = out_handle.join();
    let _ = err_handle.join();
}
