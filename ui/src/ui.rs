use crate::state::{AppState, FIELD_COUNT};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw(f: &mut ratatui::terminal::Frame<'_>, area: ratatui::layout::Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(area);

    // Top bar: status
    let top = Paragraph::new(Text::from(format!(
        "Log Diario — Nuevo Registro   {}",
        if app.editing { "Edición" } else { "Listo" }
    )))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(top, chunks[0]);

    // Main area: left form / right preview
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(chunks[1]);

    // Left form: fields stacked
    // build each field as a Paragraph so we can apply borders/colors per input
    let mut fields: Vec<Paragraph> = Vec::new();

    for i in 0..FIELD_COUNT {
        let (label, content) = match i {
            0 => (
                "Title:",
                if app.editing && app.current_field == 0 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.title.clone()
                },
            ),
            1 => (
                "Domain:",
                if app.editing && app.current_field == 1 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.domain.clone()
                },
            ),
            2 => (
                "Date:",
                if app.editing && app.current_field == 2 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.date.clone()
                },
            ),
            3 => (
                "Participants:",
                if app.editing && app.current_field == 3 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.participants.clone()
                },
            ),
            4 => (
                "Files:",
                if app.editing && app.current_field == 4 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.files.clone()
                },
            ),
            5 => (
                "Tags:",
                if app.editing && app.current_field == 5 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.tags.clone()
                },
            ),
            6 => (
                "Description:",
                if app.editing && app.current_field == 6 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.description.clone()
                },
            ),
            7 => (
                "Rationale:",
                if app.editing && app.current_field == 7 {
                    format!("{text}▮", text = app.editing_text)
                } else {
                    app.form.rationale.clone()
                },
            ),
            _ => ("", String::new()),
        };
        let mut p =
            Paragraph::new(Text::from(format!("{label} {content}", label = label, content = content))).wrap(Wrap { trim: true });
        // highlight if focused
        if app.editing && app.current_field == i {
            p = p
                .block(Block::default().borders(Borders::ALL).title(label))
                .style(Style::default().bg(Color::Blue).fg(Color::White));
        } else {
            p = p.block(Block::default().borders(Borders::ALL).title(label));
        }
        fields.push(p);
    }
    // Layout fields vertically inside left pane
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(3); FIELD_COUNT].as_slice())
        .split(main[0]);
    for (i, p) in fields.into_iter().enumerate() {
        let idx = if i < left_chunks.len() {
            i
        } else {
            left_chunks.len() - 1
        };
        f.render_widget(p, left_chunks[idx]);
    }

    // Right: preview & help
    // show suggestions or preview
    let mut preview = format!("Preview:\n\nTitle: {title}\nDomain: {domain}\nDate: {date}\nParticipants: {participants}\nFiles: {files}\nTags: {tags}\n\nDescription:\n{description}\n\nRationale:\n{rationale}\n\nActions:\n- Ctrl+S Save draft\n- Ctrl+Enter Submit (generate ml record)\n- Esc Quit\n\nRelational fields:\n- Participants: comma-separated list of user IDs or names.\n- Files: glob patterns (e.g., src/**, tests/*.rs).\n- Domain: logical grouping, e.g., infra/database or operacion/tienda.\n", title = app.form.title, domain = app.form.domain, date = app.form.date, participants = app.form.participants, files = app.form.files, tags = app.form.tags, description = app.form.description, rationale = app.form.rationale)

    if app.suggestion_loading {
        preview.push_str("\nSearching suggestions...\n");
    } else if !app.suggestions.is_empty() {
        preview.push_str("\nSuggestions:\n");
        for (i, s) in app.suggestions.iter().enumerate().take(6) {
            let title = s
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(no title)");
            let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let snippet = s.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            preview.push_str(&format!("{i}: {title} — {snippet}\n", i = i + 1, title = title, snippet = snippet));
            if !id.is_empty() {
                preview.push_str(&format!("   id: {id}\n", id = id));
            }
        }
    } else if let Some(e) = &app.suggestion_error {
        preview.push_str(&format!("\nSuggestion error: {err}\n", err = e));
    }

    let right = Paragraph::new(Text::from(preview))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Preview / Ayuda"),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(right, main[1]);

    // Bottom bar
    let bottom = Paragraph::new(Text::from("Keys: Tab Next field • Shift+Tab Prev field • Enter commit field • Ctrl+S Save • Ctrl+Enter Submit • Esc Quit")).block(Block::default().borders(Borders::TOP));
    f.render_widget(bottom, chunks[2]);
}
