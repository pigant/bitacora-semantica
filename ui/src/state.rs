use std::time::Instant;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum Step {
    Identification = 0,
    DateContext = 1,
    Participants = 2,
    Files = 3,
    Content = 4,
    Preview = 5,
}

#[derive(Clone)]
pub struct Form {
    pub title: String,
    pub domain: String,
    pub date: String,
    pub participants: String,
    pub files: String,
    pub description: String,
    pub tags: String,
    pub rationale: String,
}

impl Form {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            domain: String::new(),
            date: String::new(),
            participants: String::new(),
            files: String::new(),
            description: String::new(),
            tags: String::new(),
            rationale: String::new(),
        }
    }
}

pub fn step_count() -> usize {
    6
}

#[allow(dead_code)]
pub fn step_title(s: Step) -> &'static str {
    match s {
        Step::Identification => "Identificación",
        Step::DateContext => "Fecha & Contexto",
        Step::Participants => "Participantes",
        Step::Files => "Archivos",
        Step::Content => "Contenido",
        Step::Preview => "Preview",
    }
}

#[allow(dead_code)]
pub fn status_text(editing: bool) -> &'static str {
    if editing {
        "● En edición"
    } else {
        "✓ Listo"
    }
}

pub const FIELD_COUNT: usize = 8;

pub struct AppState {
    #[allow(dead_code)]
    pub current: Step,
    #[allow(dead_code)]
    pub completed: Vec<bool>,
    pub form: Form,
    pub editing: bool,
    pub current_field: usize,
    pub edit_pos: usize,
    pub editing_text: String,
    pub toast: Option<(String, Instant)>,
    pub spinner_idx: usize,
    pub show_confirm: bool,
    pub executing: bool,
    pub exec_result: Option<String>,
    // suggestion state
    pub suggestion_loading: bool,
    pub suggestion_req_id: u64,
    pub suggestions: Vec<serde_json::Value>,
    pub suggestion_error: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current: Step::Identification,
            completed: vec![false; step_count()],
            form: Form::new(),
            editing: false,
            current_field: 0,
            edit_pos: 0,
            editing_text: String::new(),
            toast: None,
            spinner_idx: 0,
            show_confirm: false,
            executing: false,
            exec_result: None,
            suggestion_loading: false,
            suggestion_req_id: 0,
            suggestions: Vec::new(),
            suggestion_error: None,
        }
    }
}
