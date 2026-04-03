# TUI.md — Interfaz de Terminal (TUI) para la "Bitácora Semántica"

Propósito
--------
Proponer el diseño y las recomendaciones funcionales para una TUI enfocada en capturar y gestionar registros según la ontología definida en ONTOLOGY.md (Decision, Fact, Assumption, Meeting, Action, Person, Evidence, Outcome, Link). La TUI debe facilitar la captura rápida, vinculación explícita entre registros, validación local y exportación en JSON-LD.

Audiencia
---------
Líderes técnicos, operadores y desarrolladores que quieren registrar el día a día, decisiones y evidencias desde terminal con un flujo minimalista, consistente y verificable.

Resumen de diseño guiado por heurísticas
----------------------------------------
El diseño de la TUI se guía explícitamente por las 10 heurísticas de usabilidad de Jakob Nielsen. Cada componente y flujo del documento incorpora estas heurísticas — esto asegura que las decisiones UX no sean sólo buenas prácticas, sino que estén justificadas y aplicadas en todos los niveles: formularios, validación, autocompletes con LLMs, persistencia, accesibilidad y errores.

Principios UX (resumidos)
-------------------------
- Formularios orientados al dato: un registro por acción; campos claros con validación inmediata.
- Minimizar modos: navegar / editar / accionar; evitar submodos confusos.
- Previsualización y control: mostrar JSON-LD antes de guardar y pedir confirmación para cambios críticos.
- Operaciones pesadas (LLM, indexación) en background con feedback visible.

Relación con heurísticas (resumen rápido)
- Visibilidad del estado: barra inferior y panel preview muestran estado (guardado, validando, LLM trabajando).
- Prevención de errores: validación en tiempo real con JSON Schema y controles forzados (enums).
- Reconocimiento en lugar de recuerdo: autocompletar personas/tags desde index y sugerencias LLM groundeadas.

Estructura general de la TUI
---------------------------
Diseño de pantalla (horizontal, dos paneles):
- Panel izquierdo (formulario): lista de campos para el tipo seleccionado y control de navegación.
- Panel derecho (preview): JSON-LD generado, mensajes de validación, spinner de tareas en background y sugerencias LLM.
- Barra inferior: ayuda de teclas (shortcuts), estado (offline/ready), mensajes breves y botón de ayuda (F1).

ASCII básico

+---------------------------------------------------------------+
| FORM (left)                      | PREVIEW / VALIDATION (right)|
| - Tipo: [Decision]               | { JSON-LD }                 |
| - Title:                         | [SUGGESTIONS: title, tags]  |
| - Rationale:                     |                             |
| - Tags:                          |                             |
| - Evidence:                      |                             |
| [Save] [Link] [Search] [Export]  | [Spinner / Result messages] |
+---------------------------------------------------------------+

Explicación integrada por heurística
------------------------------------
(1) Visibilidad del estado del sistema
- Implementación: barra inferior con bandera de estado (Ready, Dirty, Validating, Saving, Error), y un spinner/contador para llamadas LLM o indexación.
- Ejemplo: al solicitar "Sugerir títulos" aparece "LLM: sugiriendo (30%)" y luego la lista de títulos en preview.

(2) Concordancia entre sistema y mundo real
- Usar terminología del dominio (Decision, Meeting, Action) y placeholders en campos para mostrar ejemplos reales.
- Ejemplo: placeholder de "rationale": "Por qué esta decisión reduce el riesgo de inconsistencias…".

(3) Control y libertad del usuario
- Deshacer sencillo (Ctrl-Z para cambios en formulario) y opción de cancelar procesos en background (p. ej. detener una sugerencia LLM en curso).
- Guardados explícitos: el usuario confirma antes de escribir a disco.

(4) Consistencia y estándares
- Atajos y etiquetas consistentes; nombres de campos coincidentes con ONTOLOGY.md y context.jsonld.
- Ejemplo: todos los campos de fecha exigen ISO-8601 y se validan uniformemente.

(5) Prevención de errores
- Validación en tiempo real con JSON Schema; enums renderizados como selects.
- Si un campo requerido falta, el botón Guardar queda deshabilitado y aparece una ayuda con la heurística aplicada: "Evita este error: rellene title (requerido)".

(6) Reconocimiento en lugar de recuerdo
- Autocomplete para Person.id, tags y búsqueda de registros (Ctrl-F). Los items vienen con snippets para facilitar la elección.
- LLM grounding: al sugerir links, LLM recibe solo 3–5 snippets relevantes para reducir alucinaciones.

(7) Flexibilidad y eficiencia de uso
- Modo avanzado "power user" para pegar JSON-LD crudo o editar en un modal con tu editor preferido.
- Atajos configurables y macros para flujos frecuentes (crear meeting + crear actions).

(8) Diseño estético y minimalista
- Mostrar solo campos relevantes por tipo; esconder campos avanzados tras "Mostrar opciones avanzadas".
- Preview limpio: JSON-LD formateado, errores y warnings resaltados.

(9) Ayuda para reconocer, diagnosticar y recuperarse de errores
- Mensajes de error claros y acciones recomendadas; logs accesibles (F1 -> Ver logs).
- En fallos LLM, indicar fallback aplicado y permitir reintentar.

(10) Ayuda y documentación
- Ayuda contextual (F1) con ejemplos tomados de ONTOLOGY.md; atajo ? muestra ayuda rápida para el campo activo.

Flujos principales (integrando heurísticas y LLM)
------------------------------------------------
1) Crear registro
- El formulario guía por campos obligatorios (heurística 5). En cualquier campo, el usuario puede pulsar Ctrl‑Sugg para pedir sugerencias LLM (heurísticas 6 y 1: sugerencia visible en preview y con estado).
- Al aceptar una sugerencia, la TUI registra provenance.suggested_by (model, prompt_hash). El guardado requiere confirmación (heurística 3).

2) Enlazar registros
- Buscador (Ctrl-F) muestra snippets y scores; al pedir "Sugerir links" la LLM devuelve candidatos grounded (solo IDs existentes o marcados como candidatos) — el usuario siempre confirma (heurística 5 y 6).

3) Reuniones -> generar acciones
- Extraer acciones desde minutes con LLM (extract_actions). Las acciones aparecen como sugerencias con assigned_to pre‑autocompletado desde People index; el usuario decide promoverlas a registros independientes (heurística 7).

Validación y esquema (integrado)
--------------------------------
- JSON Schema por tipo valida en tiempo real; errores se muestran en preview con una explicación "por qué" y pasos de resolución (heurística 9).
- Reglas CSP: fechas ISO-8601, confidence entre 0.0 y 1.0.

Autogeneración de ID y versionado
---------------------------------
- ID por defecto: urn:bitacora:<domain>:<uuid-v4>. El usuario puede reemplazar en modo avanzado; por diseño ID es inmutable tras primer guardado; ediciones generan updated_at y version increment.
- Esta política aplica la heurística 3 (control y libertad) y 5 (prevención de errores en identificación).

Persistencia (segura y visible)
--------------------------------
- Guardado atómico: write->temp->rename; index actualizado tras éxito. En la UI se muestra el resultado y link al archivo (heurística 1).
- Si ocurre fallo de I/O, la TUI ofrece revert/recuperación y un enlace al log con la traza (heurística 9).

Import/Export
-------------
- Import con mapping y dry-run: antes de persistir, la TUI valida y muestra los cambios (heurística 3 y 5).
- Export ofrece filtros y formatos (JSON-LD, TTL, CSV) y muestra impacto esperado (nº de registros) antes de ejecutar.

Concurrencia, background workers y LLMs
---------------------------------------
- LLM y indexación se ejecutan en workers asíncronos; la UI muestra progreso y permite cancelar.
- Resultado LLM llega a preview como sugerencia (no overwrite). Registrar metadata (model, prompt, context hash) si el usuario acepta.
- Política de seguridad: no enviar contenido sensible; truncar logs; mostrar al usuario qué snippets fueron enviados (transparencia, heurística 10).

Oportunidades de inferencia con LLM (integradas)
------------------------------------------------
- Title: sugerir 1–3 títulos; mostrar confianza y opción de aceptar (heurísticas 6, 7).
- Tags: sugerir tags normalizados y mapear al vocabulario interno; permitir edición rápida.
- Actions extraction: parsear minutes y sugerir acciones estructuradas; promover a registros.
- Links: sugerir candidates grounded con score; marcar como "candidate" si no existe id.
- Generate JSON-LD completo: producir un JSON-LD provisional validado y mostrar diff.

Prompts y validación (práctico)
-------------------------------
- Todos los prompts usan "RETURN JSON ONLY" y se validan con JSON Schema. En caso de no conformidad, aplicar fallback y notificar.
- Ejemplo de prompt integrado: extraer acciones desde meeting -> LLM -> validar -> mostrar acciones en preview con botones "Promote", "Edit", "Reject".

Provenance y auditoría
----------------------
- Cada registro guarda provenance: recorded_by, recorded_via (TUI), suggestion metadata (si aplica) y timestamp.
- Las sugerencias LLM aceptadas almacenan model, prompt_hash, input_snippets (cortos) y accepted_by — útil para debugging y cumplimiento.

Atajos de teclado (alineados con heurísticas)
--------------------------------------------
- Ctrl-N: nuevo registro (eficiencia)
- Ctrl-T: cambiar Tipo
- Ctrl-S: guardar (acción explícita)
- Ctrl-P: previsualizar (visibilidad)
- Ctrl-L: añadir Link (buscar)
- Ctrl-Sugg: solicitar sugerencia LLM para campo activo
- Ctrl-I: importar (dry-run primero)
- Ctrl-E: exportar
- Ctrl-F: buscar registros
- ?: ayuda contextual (F1)

Búsqueda y enlace de registros (UX aplicado)
-------------------------------------------
- Buscador devuelve snippet+score para reconocer en lugar de recordar (heurística 6).
- Al elegir un registro para link, la TUI muestra por qué fue recomendado (rationale corto) y permite aceptar con un solo Enter.

Edición y versionado (transparente)
-----------------------------------
- Editar abre modal estructurado o raw editor; al guardar, mostrar diff y pedir confirmación (heurística 3, 9).
- Historial opcional en knowledge/_history/ con timestamp y autor.

Accesibilidad y diseño
----------------------
- Contraste alto, teclas accesibles y navegación por teclado.
- Mensajes claros con alternativas (por ejemplo, en vez de solo "Error 403" mostrar "No se detectaron permisos para escribir en path X; ver logs (F1)").

Pruebas y QA (usabilidad + técnicas)
------------------------------------
- Tests unitarios: JSON Schema, validación de I/O y atomic writes.
- E2E: crear registros, extraer acciones, aceptar sugerencias LLM, importar/exportar.
- Sesiones de usabilidad (heurística-driven): medir tiempo por registro, tasa de aceptación de sugerencias y errores detectados.

Ejemplos integrados (UI -> JSON-LD + heurística aplicable)
--------------------------------------------------------
1) Decision rápida (heur. 5,6,1)
- Usuario: abre Decision, escribe rationale corto y pulsa Ctrl-Sugg para títulos.
- Preview muestra 3 títulos sugeridos con confidence; aceptar uno y guardar. Guardado escribe knowledge/payments/urn-...jsonld y marca provenance.

Ejemplo (preview JSON-LD):
{
  "@context":"./context.jsonld",
  "id":"urn:bitacora:payments:dec-2026-0001",
  "type":"Decision",
  "title":"Adoptar Postgres",
  "rationale":"Necesitamos ACID para conciliación",
  "domain":"payments",
  "tags":["database","arch-decision"],
  "recorded_at":"2026-03-30T15:00:00Z",
  "author":"mailto:lead@org",
  "status":"accepted",
  "confidence":0.9
}

2) Meeting -> extraer Actions (heur. 6,7,1)
- Usuario pega minutes y pulsa "Extraer acciones". LLM devuelve 3 objects. Usuario promueve 2 a registros Action, asigna responsables con autocompletion desde People index y guarda.

Próximos pasos de implementación
--------------------------------
- Crear JSON Schema por tipo e integrarlo en la TUI (prioridad alta).
- Implementar microservicio/backend para orquestar: grounding (embeddings + index) -> LLM calls -> schema validation -> cache.
- Prototipo TUI (Python + textual o Rust + ratatui) con: formularios, preview, index básico y llamadas simuladas a LLM (mode dry-run).

Notas finales
------------
Esta TUI es una herramienta construida para la captura rápida y correcta de conocimiento técnico. Al integrar las heurísticas de Nielsen con capacidades LLM groundeadas, validación estricta y políticas de seguridad, se logra un equilibrio entre velocidad, precisión y trazabilidad.

Si quieres, puedo:
- Generar context.jsonld y JSON Schema basados en ONTOLOGY.md (siguiente paso lógico),
- Escribir prompts finales y los JSON Schemas para validar respuestas LLM (titles, actions),
- Esbozar el microservicio que orquesta grounding + LLM + validación y entregar pseudocódigo.

¿Qué prefieres que haga ahora?