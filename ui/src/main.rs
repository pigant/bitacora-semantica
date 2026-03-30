mod state;
mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use state::{AppState, FIELD_COUNT};
mod inference; // inference helper
use inference::infer_rationale_from_form;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::{
    io,
    time::{Duration, Instant},
};
mod pi_rpc;
use crossterm::event::KeyEvent;

mod date_parse;

fn validate_field(idx: usize, val: &str) -> Result<(), String> {
    match idx {
        0 => {
            if val.trim().is_empty() {
                Err("Title is required".into())
            } else {
                Ok(())
            }
        }
        2 => {
            // accept YYYY-MM-DD or textual shortcuts
            if val.trim().is_empty() {
                return Ok(());
            }
            if date_parse::parse_date_text(val).is_some() {
                Ok(())
            } else {
                Err("Date must be YYYY-MM-DD or textual (hoy/ayer/antes de ayer/la semana pasada/el mes pasado/hace N dias)".into())
            }
        }
        _ => Ok(()),
    }
}

fn commit_field(app: &mut AppState) -> Result<(), String> {
    // validate first
    validate_field(app.current_field, &app.editing_text)?;
    match app.current_field {
        0 => app.form.title = app.editing_text.clone(),
        1 => app.form.domain = app.editing_text.clone(),
        2 => app.form.date = app.editing_text.clone(),
        3 => app.form.participants = app.editing_text.clone(),
        4 => app.form.files = app.editing_text.clone(),
        5 => app.form.tags = app.editing_text.clone(),
        6 => app.form.description = app.editing_text.clone(),
        7 => app.form.rationale = app.editing_text.clone(),
        _ => {}
    }
    Ok(())
}

fn get_field(app: &AppState) -> String {
    match app.current_field {
        0 => app.form.title.clone(),
        1 => app.form.domain.clone(),
        2 => app.form.date.clone(),
        3 => app.form.participants.clone(),
        4 => app.form.files.clone(),
        5 => app.form.tags.clone(),
        6 => app.form.description.clone(),
        7 => app.form.rationale.clone(),
        _ => String::new(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();

    // channel for suggestion worker
    type SuggestionResult = (u64, Result<Vec<Value>, String>);
    let (s_tx, s_rx): (Sender<SuggestionResult>, Receiver<SuggestionResult>) = channel();

    let pi_path = "/home/piagent/.nvm/versions/node/v24.14.0/bin/pi".to_string();

    // helper to spawn worker when domain+description ready
    let mut last_snapshot = (String::new(), String::new(), 0u64); // domain, description, req_id

    // spawn a thread to receive worker results and apply to app state
    // We'll apply results in main loop by checking the channel non-blocking below (Receiver is not Clone)

    loop {
        terminal.draw(|f| ui::draw(f, f.size(), &app))?;

        // tick spinner index
        app.spinner_idx = app.spinner_idx.wrapping_add(1);

        if event::poll(Duration::from_millis(100))? {
            // check suggestion worker channel
            if let Ok((req_id, res)) = s_rx.try_recv() && req_id == app.suggestion_req_id {
                match res {
                    Ok(vec) => {
                        app.suggestions = vec;
                        app.suggestion_loading = false;
                        app.toast = Some(("Sugerencias recibidas".into(), Instant::now()));
                    }
                    Err(e) => {
                        app.suggestion_error = Some(e);
                        app.suggestion_loading = false;
                        app.toast = Some(("Error al obtener sugerencias".into(), Instant::now()));
                    }
                }
            }

            if let Event::Key(KeyEvent {
                code, modifiers: _, ..
            }) = event::read()?
            {
                if app.show_confirm {
                    match code {
                        KeyCode::Char('y') => {
                            app.show_confirm = false;
                            app.executing = true;
                            app.exec_result = None;
                            app.toast = Some(("Ejecutando...".into(), Instant::now()));
                        }
                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.show_confirm = false;
                            app.toast = Some(("Ejecución cancelada".into(), Instant::now()));
                        }
                        _ => {}
                    }
                    continue;
                }

                if app.executing {
                    app.exec_result = Some("Resultado: OK (simulado)".to_string());
                    app.executing = false;
                    app.toast = Some(("Ejecución completada".into(), Instant::now()));
                    continue;
                }

                match code {
                    KeyCode::Esc => {
                        if app.editing {
                            app.editing = false;
                            app.editing_text.clear();
                            app.edit_pos = 0;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Char(c) => {
                        if app.editing {
                            // insert at edit_pos
                            if app.edit_pos <= app.editing_text.len() {
                                app.editing_text.insert(app.edit_pos, c);
                                app.edit_pos = app.edit_pos.saturating_add(1);
                            } else {
                                app.editing_text.push(c);
                                app.edit_pos = app.editing_text.len();
                            }
                        } else {
                            // start editing focused field
                            app.editing = true;
                            app.edit_pos = 0;
                            app.editing_text = get_field(&app);
                            app.editing_text.insert(0, c);
                            app.edit_pos = 1;
                        }
                    }
                    KeyCode::Backspace => {
                        if app.editing && app.edit_pos > 0 && app.edit_pos <= app.editing_text.len()
                        {
                            app.edit_pos -= 1;
                            app.editing_text.remove(app.edit_pos);
                        }
                    }
                    KeyCode::Left => {
                        if app.editing && app.edit_pos > 0 {
                            app.edit_pos -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if app.editing && app.edit_pos < app.editing_text.len() {
                            app.edit_pos += 1;
                        }
                    }
                    KeyCode::Tab => {
                        if app.editing {
                            if let Err(e) = commit_field(&mut app) {
                                app.toast = Some((e, Instant::now()));
                            } else {
                                app.current_field = (app.current_field + 1) % FIELD_COUNT;
                                app.editing_text = get_field(&app);
                                app.edit_pos = app.editing_text.len();
                            }
                        }
                    }
                    KeyCode::BackTab => {
                        if app.editing {
                            if let Err(e) = commit_field(&mut app) {
                                app.toast = Some((e, Instant::now()));
                            } else {
                                app.current_field =
                                    (app.current_field + FIELD_COUNT - 1) % FIELD_COUNT;
                                app.editing_text = get_field(&app);
                                app.edit_pos = app.editing_text.len();
                            }
                        }
                    }
                    KeyCode::Up => {
                        if app.editing {
                            app.current_field = if app.current_field == 0 {
                                FIELD_COUNT - 1
                            } else {
                                app.current_field - 1
                            };
                            app.editing_text = get_field(&app);
                            app.edit_pos = app.editing_text.len();
                        }
                    }
                    KeyCode::Down => {
                        if app.editing {
                            app.current_field = (app.current_field + 1) % FIELD_COUNT;
                            app.editing_text = get_field(&app);
                            app.edit_pos = app.editing_text.len();
                        }
                    }
                    KeyCode::Enter => {
                        if app.editing {
                            if let Err(e) = commit_field(&mut app) {
                                app.toast = Some((e, Instant::now()));
                            } else {
                                // after successful commit, advance to next field and start editing it
                                app.current_field = (app.current_field + 1) % FIELD_COUNT;
                                app.editing_text = get_field(&app);
                                app.edit_pos = app.editing_text.len();
                                app.editing = true; // keep editing in new field
                                                    // special-case: if new field is date and empty, set today's date default
                                if app.current_field == 2 && let Some(d) = date_parse::parse_date_text(&app.editing_text) {
                                    let s = d.format("%Y-%m-%d").to_string();
                                    app.editing_text = s;
                                    app.edit_pos = app.editing_text.len();
                                }

                                // If we moved focus to Rationale (field 7), trigger inference in background
                                if app.current_field == 7 {
                                    app.suggestion_loading = true;
                                    let form_snapshot = app.form.clone();
                                    let tx = s_tx.clone();
                                    // use suggestion_req_id as a temporary request id for rationale inference
                                    app.suggestion_req_id = app.suggestion_req_id.wrapping_add(1);
                                    let req_id = app.suggestion_req_id;
                                    thread::spawn(move || {
                                        let res = infer_rationale_from_form(&form_snapshot);
                                        match res {
                                            InferenceResult::Ok(r) => {
                                                let _ = tx.send((req_id, Ok(vec![serde_json::json!({"id":"inference","title":"rationale","snippet":r})])));
                                            }
                                            InferenceResult::Err(e) => {
                                                let _ = tx.send((req_id, Err(e)));
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                    KeyCode::F(5) => {
                        app.toast = Some(("Borrador guardado (simulado)".into(), Instant::now()));
                    }
                    KeyCode::F(9) => {
                        app.show_confirm = true;
                    }
                    _ => {}
                }
            }
        }
    }

    // automatic suggestion trigger: when domain+description populated and not already loading for this snapshot
    if !app.form.domain.trim().is_empty()
        && !app.form.description.trim().is_empty()
        && !(app.suggestion_loading)
    {
        // snapshot
        let dom = app.form.domain.clone();
        let desc = app.form.description.clone();
        if dom != last_snapshot.0 || desc != last_snapshot.1 {
            // new snapshot -> spawn worker
            last_snapshot.0 = dom.clone();
            last_snapshot.1 = desc.clone();
            app.suggestion_loading = true;
            app.suggestion_req_id = app.suggestion_req_id.wrapping_add(1);
            let req_id = app.suggestion_req_id;
            let tx = s_tx.clone();
            let pi = pi_path.clone();
            // build prompt using the context the user provided earlier
            let prompt = format!(
                    "Using the following Mulch context (passed externally), find up to 6 references for domain='{}' and description='{}'. Return ONLY a JSON array of objects {{id,title,snippet,origin}}. If none, return []\nContext:\n{}\n",
                    dom, desc, "KIOSCO & OPERACIONES summary omitted for brevity"
                );
            thread::spawn(move || {
                // append start log
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/log_diario_suggestions.log")
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "START req_id={}", req_id)
                    });
                let res = pi_rpc::send_prompt_and_collect(&pi, &prompt, 30);
                // write raw events for inspection (append)
                match &res {
                    Ok(evts) => {
                        if let Ok(s) = serde_json::to_string_pretty(evts) {
                            let _ = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("/tmp/log_diario_suggestions.log")
                                .and_then(|mut f| {
                                    use std::io::Write;
                                    writeln!(f, "EVENTS:\n{}\n", s)
                                });
                        }
                    }
                    Err(e) => {
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/log_diario_suggestions.log")
                            .and_then(|mut f| {
                                use std::io::Write;
                                writeln!(f, "ERROR:{}\n", e)
                            });
                    }
                }
                match res {
                    Ok(evts) => {
                        // try simple heuristic: find message_update assistant text that contains '['
                        let mut texts: Vec<String> = Vec::new();
                        for v in evts {
                            if let Some(t) = v.get("type").and_then(|t| t.as_str()) {
                                                if t == "tool_execution_end" && let Some(arr) = v.get("result").and_then(|r| r.get("content")).and_then(|c| c.as_array()) {
                                    for item in arr {
                                        if let Some(txt) = item.get("text").and_then(|x| x.as_str()) {
                                            texts.push(txt.to_string());
                                        }
                                    }
                                }
                                if (t == "message_update" || t == "message_end") && let Some(arr) = v.get("assistantMessageEvent").and_then(|ame| ame.get("partial")).and_then(|p| p.get("content")).and_then(|c| c.as_array()) {
                                    for block in arr {
                                        if block.get("type").and_then(|x| x.as_str()) == Some("text") {
                                            if let Some(txt) = block.get("text").and_then(|x| x.as_str()) {
                                                texts.push(txt.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for t in texts {
                            if let Some(idx) = t.find('[') {
                                let sub = &t[idx..];
                                if let Ok(j) = serde_json::from_str::<serde_json::Value>(sub) && j.is_array() {
                                    let arr = j.as_array().unwrap().clone();
                                    let out = arr.to_vec();
                                    let _ = tx.send((req_id, Ok(out)));
                                    return;
                                }
                            }
                        }
                        let _ = tx.send((req_id, Ok(Vec::new())));
                    }
                    Err(e) => {
                        let _ = tx.send((req_id, Err(e)));
                    }
                }
            });
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    println!(
        "Wizard cerrado. Resultado:\nTitle='{}'\nDomain='{}'\nDate='{}'\nParticipants='{}'\nFiles='{}'\nDescription='{}'",
        app.form.title,
        app.form.domain,
        app.form.date,
        app.form.participants,
        app.form.files,
        app.form.description
    );
    Ok(())
}
