UI PLANNING — Proyecto Rust (ratatui + chrono)
===============================================

Objetivo
--------
Generar un plan y scaffold para una TUI en Rust usando ratatui (rendering) y chrono (fechas). El diseño sigue las decisiones y heurísticas de TUI.md y deja la conexión con LLM sólo como un trait con implementación dummy (por implementar). Se define una taxonomía para el estado de la aplicación —no todo lineal— y reglas de separación de código (widgets en archivos propios si superan 100 líneas).

Resumen ejecutivo
-----------------
- Stack: Rust, ratatui, crossterm (event handling), tokio (async workers), chrono (fechas), serde/serde_json (JSON-LD previews), anyhow/thiserror (errores). La TUI no asume llamadas HTTP a servicios remotos.
- Estructura modular: cada widget + componente importante en su propio módulo/archivo. Si un archivo supera 100 líneas, dividirlo por responsabilidad.
- LLM: trait LlmClient con método async que devuelve sugerencias; DummyLlm implementa el trait y devuelve valores de ejemplo. La interfaz es agnóstica respecto a cómo se generan las sugerencias y no presupone tráfico de red: puede implementarse con modelos locales, procesos offline o adaptadores futuros.
- Validación: JSON Schema en tiempo real para previews y antes de aceptar sugerencias/guardar registros.

Árbol de archivos sugerido
-------------------------
bitacora-tui/
├─ Cargo.toml
├─ README.md
├─ src/
│  ├─ main.rs
│  ├─ app.rs                 # App state, taxonomy, commands, persistence glue
│  ├─ events.rs              # Events, input mapping, timers
│  ├─ ui/
│  │  ├─ mod.rs
│  │  ├─ layout.rs           # layout composition helpers
│  │  ├─ widgets/
│  │  │  ├─ mod.rs
│  │  │  ├─ form_widget.rs   # formulario para crear/editar registros
│  │  │  ├─ preview_widget.rs# JSON-LD preview + validation messages
│  │  │  ├─ search_widget.rs # buscador/link selector
│  │  │  ├─ help_widget.rs   # ayuda contextual
│  │  │  ├─ footer.rs        # barra inferior (estado y shortcuts)
│  │  │  └─ modal.rs         # modales genéricos (confirm, input)
│  ├─ llm/
│  │  ├─ mod.rs
│  │  └─ dummy.rs            # DummyLlm impl
│  ├─ index.rs               # index management (knowledge/index.json)
│  ├─ persistence.rs         # write/read JSON-LD, atomic writes
│  ├─ validation.rs          # JSON Schema wrapper (jsonschema crate)
│  └─ util.rs                # utilitarios (formatting, time helpers)
├─ tests/
│  ├─ integration.rs
│  └─ fixtures/
│     ├─ decision-example.jsonld
│     └─ meeting-example.jsonld
└─ docs/
   └─ ui_flow.md

Notas de arquitectura
---------------------
- main.rs: inicializa logger, configura terminal (crossterm), arranca App y loop de render/eventos con Tokio runtime.
- App (app.rs): contiene el estado principal (taxonomía), canal para workers (tokio mpsc), y funciones públicas para manipular el estado (apply_suggestion, save_record, open_modal, etc.).
- Widgets: componentes UI independientes. Cada widget expone:
  - struct WidgetState { .. }
  - fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect)
  - input handlers (on_key(...)) opcionales
- Background workers: indexación, persistencia y (si aplica) llamadas a adaptadores de sugerencia (LLM local/dummy) se ejecutan en tasks separadas; comunican resultados a App mediante canales (mpsc). La TUI no realiza llamadas web implícitas; cualquier ejecución externa (knowledge-cli o procesos LLM) requiere confirmación del usuario y queda registrada en logs.

Cargo.toml (esqueleto)
----------------------
```toml
[package]
name = "bitacora-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
crossterm = "0.26"
ratatui = { git = "https://github.com/tui-rs-revival/ratatui", branch = "main" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
jsonschema = "0.16" # opcional
log = "0.4"
env_logger = "0.9"

[dev-dependencies]
insta = "1.24"
```

App state: Taxonomía (no lineal)
--------------------------------
El estado no será una gran struct plana — se modela como un grafo/taxonomía de subcomponentes. Ejemplo:

- AppState (raíz)
  - UiState
    - active_view: ViewId (Editor, Search, Help, Preview)
    - focused_widget: WidgetId
    - modals: Vec<ModalState>
    - notifications: Vec<Notification>
  - DomainState
    - current_domain: String
    - index: IndexState (referencias ligeras)
  - EditorState
    - draft: Option<RecordDraft>
    - dirty: bool
    - validation_errors: Vec<ValidationError>
    - suggestions: Vec<Suggestion> (from LLM or heuristics)
  - SearchState
    - query: String
    - results: Vec<SearchHit>
    - selected: Option<usize>
  - PersistState
    - pending_writes: Vec<WriteJob>
    - last_save: Option<SaveResult>
  - LlmState
    - requests: HashMap<RequestId, LlmRequestMeta>
    - client_status: LlmClientStatus (Ready, RateLimited, Offline)
  - MetricsState
    - user_actions_count: HashMap<ActionType, usize>
    - avg_time_per_record: Duration

Comentarios:
- Separar UiState / DomainState / PersistState permite testing unitario por capa.
- Suggestion objects contienen provenance metadata but only saved to record on accept.

Tipos clave (sugeridos)
-----------------------
- enum ViewId { Editor, Search, Help, Preview }
- enum WidgetId { Form, Preview, Search, Footer, Help }
- struct RecordDraft { id: Option<String>, type: RecordType, fields: Map<String, Value>, created_at: DateTime<Utc>, updated_at: Option<DateTime<Utc>> }
- enum RecordType { Decision, Fact, Assumption, Meeting, Action, Person }
- struct Suggestion { id: Uuid, field: String, value: serde_json::Value, source: SuggestionSource, score: f32, prompt_hash: String }
- enum SuggestionSource { Llm { model: String }, Heuristic }

Event model y loop
-------------------
- events.rs define Event enum:
  - Input(KeyEvent)
  - Tick
  - LlmResponse(RequestId, Result<Vec<Suggestion>, Error>)
  - SaveResult(WriteJobId, Result<SaveResult, Error>)
  - SearchResults(RequestId, Vec<SearchHit>)

- main.rs establece:
  - un tokio::sync::mpsc::channel para enviar eventos desde workers a App
  - un thread para leer teclado y enviar Input events
  - un tokio task para ticks (para spinner/clock)

Widgets y responsabilidades
---------------------------
Considerar estos widgets; poner cada uno en src/ui/widgets/*.rs. Si un archivo supera 100 líneas, dividir (p. ej. form_widget_fields.rs, form_widget_handlers.rs).

- FormWidget (formulario)
  - Campos dinámicos según RecordType
  - Validación en tiempo real delegando a validation.rs
  - Soporta Ctrl-Sugg por campo para solicitar Suggestion del LLM

- PreviewWidget
  - Muestra JSON-LD generado desde RecordDraft
  - Resalta errores de schema
  - Muestra sugerencias y diff con controles para aceptar/editar

- SearchWidget
  - Buscar en index (index.rs)
  - Mostrar snippets y scores; selección con Enter => crea Link

- HelpWidget
  - Mostrar heurísticas de Nielsen aplicadas, atajos y ejemplos (extraído de TUI.md)

- FooterWidget
  - Estado global: ready/dirty/LLM working
  - Atajos visibles y notificaciones breves

- Modal
  - Confirm modal, input modal genérico, error modal

Diseño de módulos (ejemplos)
---------------------------
src/ui/widgets/form_widget.rs
- pub struct FormWidget { pub state: FormState }
- impl FormWidget { pub fn new(...) -> Self; pub fn on_key(&mut self, key: KeyEvent, app: &mut AppState); pub fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect) }

src/llm/mod.rs
- pub trait LlmClient: Send + Sync {
    async fn suggest_titles(&self, domain: &str, text: &str, rtype: &str) -> Result<Vec<Suggestion>, LlmError>;
    async fn extract_actions(&self, minutes: &str) -> Result<Vec<Suggestion>, LlmError>;
  }

- pub struct DummyLlm; impl LlmClient for DummyLlm { /* devuelve sugerencias estáticas */ }

Ejemplo de trait + impl dummy (snippet)
---------------------------------------
```rust
// src/llm/mod.rs
use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Suggestion { pub id: Uuid, pub field: String, pub value: serde_json::Value, pub score: f32 }

#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Sugiere títulos para un texto dado. La interfaz NO presupone llamadas HTTP ni
    /// servicios remotos; está pensada para adaptadores locales o procesos offline.
    async fn suggest_titles(&self, domain: &str, text: &str, rtype: &str) -> Result<Vec<Suggestion>>;

    /// Extrae acciones desde minutes/texto.
    async fn extract_actions(&self, minutes: &str) -> Result<Vec<Suggestion>>;
}

pub struct DummyLlm;

#[async_trait]
impl LlmClient for DummyLlm {
    async fn suggest_titles(&self, _domain: &str, text: &str, _rtype: &str) -> Result<Vec<Suggestion>> {
        Ok(vec![Suggestion { id: Uuid::new_v4(), field: "title".into(), value: json!( [format!("Resumen: {}", &text.chars().take(60).collect::<String>())] ), score: 0.8 }])
    }
    async fn extract_actions(&self, minutes: &str) -> Result<Vec<Suggestion>> {
        let sample = json!([{"title": "Revisar PoC Postgres","assigned_to": null,"due_date": null}]);
        Ok(vec![Suggestion { id: Uuid::new_v4(), field: "actions".into(), value: sample, score: 0.9 }])
    }
}
```

Persistencia e índice
---------------------
- persistence.rs: la TUI delega la persistencia a la herramienta knowledge-cli mediante invocaciones del sistema (bash). Implementar un wrapper que construya el comando que se va a ejecutar y lo muestre al usuario para confirmación. Uso propuesto:
  - Construir archivo temporal JSON-LD (tempfile)
  - Mostrar modal: "Se ejecutará: `knowledge-cli write --domain <domain> --file <tempfile>` ¿Confirmar?"
  - Si el usuario confirma, ejecutar con tokio::process::Command (async) y capturar stdout/stderr; registrar resultado en SaveResult.
  - Registrar provenance: comando exacto, stdout/stderr, timestamp, usuario que lo ejecutó.
- index.rs: leer knowledge/index.json para poblar IndexState en memoria. Si el usuario solicita reindex (acción explícita), ejecutar `knowledge-cli index --rebuild` después de confirmación.
- Asegurar que knowledge-cli/knowledge/ esté ignorado por git y que la TUI respete esa ruta por defecto. No ejecutar comandos externos automáticamente sin confirmación explícita.

Validación (validation.rs)
--------------------------
- Wrapper mínimo sobre jsonschema crate para validar previews antes de guardar. Devuelve Vec<ValidationError> con path y mensaje para mostrar en PreviewWidget.

Pruebas y QA
------------
- Unit tests por módulo (app state transitions, validation). Usa insta para snapshots de JSON-LD generados.
- Integration tests: tests/integration.rs simula crear Decision y Meeting; utiliza DummyLlm para respuestas estables.

CI / GitHub Actions
-------------------
- workflow: cargo build + cargo test + clippy. Opcional: job para publicar docs/ a GitHub Pages (si repo lo desea).

Guías de estilo y límites por archivo
------------------------------------
- Regla práctica: Si un módulo excede 100 líneas, revísalo y extráele subcomponentes:
  - ui/widgets/form_widget.rs -> form_fields.rs, form_handlers.rs
  - app.rs -> app_state.rs + app_commands.rs
- Mantén funciones pequeñas y bien documentadas; exporta sólo lo necesario en mod.rs para mantener API interna clara.

Conexión LLM: integración futura
-------------------------------
- La carpeta src/llm contiene el trait y Dummy impl. Cuando decidas proveedor (OpenAI, local LLM, Anthropic), implementa un cliente que cumpla LlmClient. El AppState debe depender de Box<dyn LlmClient> inyectado en inicio para facilitar testing.

Ejemplo de main.rs (esqueleto)
------------------------------
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logger
    env_logger::init();

    // construir App con DummyLlm
    let llm: Arc<dyn LlmClient> = Arc::new(DummyLlm{});
    let mut app = App::new(llm);

    // terminal setup, run UI loop
    ui::run(&mut app).await?;

    Ok(())
}
```

Siguientes pasos sugeridos (prioridad inmediata)
-----------------------------------------------
1. Crear cargo project: cargo new bitacora-tui --bin
2. Añadir dependencias a Cargo.toml según esqueleto.
3. Implementar AppState (app.rs) + eventos (events.rs) + minimal UI loop en main.rs con render vacío.
4. Añadir llm trait + DummyLlm y tests que usen DummyLlm.
5. Implementar widgets base: FooterWidget, PreviewWidget, FormWidget (esqueleto). Hacer render de layout sin lógica para validar arquitectura.
6. Integrar persistence y index skeleton.
7. Escribir tests unitarios y snapshots para JSON-LD preview.

Tareas opcionales/avanzadas
--------------------------
- Integrar jsonschema y validación en preview.
- Implementar caching por prompt_hash + context_hash para llamadas LLM.
- Añadir métricas y trazabilidad (persistir provenance cuando el usuario acepta sugerencias).
- Implementar plugin system para adaptadores de import/export.

¿Deseas que cree el scaffold inicial del proyecto (cargo new + archivos base: main.rs, app.rs, llm/mod.rs con DummyLlm y algunos widgets esqueleto) y lo agregue al repo? Si sí, confirmo y lo genero en /mnt/disk/trabajo/cencosud/bitacora/bitacora-tui/.
