# PROPUESTA: Ontología para "Bitácora Semántica"

## Resumen

Objetivo: diseñar una ontología práctica para representar una "Bitácora Semántica" de trabajo técnico (decisiones, hechos, suposiciones, reuniones, acciones), enlazar registros entre sí y facilitar búsqueda, auditoría y trazabilidad.

Nota sobre el término "Bitácora Semántica"
------------------------------------------
Se reemplaza la expresión "Log Diario" por "Bitácora Semántica" para enfatizar dos ideas clave: (1) "bitácora" comunica claramente que se trata de registros de trabajo o entradas cronológicas del proyecto, y (2) "semántica" indica que los registros contienen metadatos estructurados y enlaces (ontología) que permiten consultas y automatización. En el diseño recomendamos usar URNs con prefijo `urn:bitacora:` (por ejemplo `urn:bitacora:payments:dec-2026-0001`) como identificadores estables; si existen registros previos con `urn:log:...`, documentar un mapeo o migración.

## Principios de diseño

- Modelo en forma de grafo dirigido con nodos tipados (Decision, Fact, Meeting, Action, Person, etc.) y relaciones tipadas (relatesTo, supersedes, dependsOn).
- Formato primario: JSON-LD por registro (un fichero = un registro). Alternativa: JSONL por dominio si se prefiere simplicidad.
- Identificadores estables: URN local del estilo urn:bitacora:<dominio>:<id> o UUIDs reproducibles. (Se recomienda `urn:bitacora:` para nuevos registros.)
- Diseñar para independencia de cualquier herramienta específica; priorizar interoperabilidad, claridad y validación con esquema.

## Ontología: clases principales (aclaradas)

Convenciones rápidas
- Notación: Tipo (string), Cardinalidad: 1..1 (obligatorio), 0..1 (opcional), 0..* (lista).
- id: URI/URN o UUID (recomendado: mantener único y estable).

1) Record (superclase)
- Propósito: normalizar metadatos compartidos por todo tipo de registro.
- Requeridos:
  - id: string (1..1)
  - type: string (1..1) — uno de los subtipos (Decision, Fact, Meeting, Action, ...)
  - recorded_at: string (ISO-8601) (1..1)
- Opcionales / recomendados:
  - title / name: string (0..1)
  - description / body: string (0..1)
  - domain / project: string (0..1)
  - tags: string[] (0..*)
  - classification: enum (foundational|tactical|observational) (0..1)
  - author: Person.id (0..1)
  - participants: Person.id[] (0..*)
  - evidence: Evidence[] (0..*)
  - outcomes: Outcome[] (0..*)
  - links: Link[] (0..*)
  - status: enum (proposed|accepted|rejected|in-progress|done|deprecated) (0..1)
  - confidence: number (0..1) (0..1)
  - impact: enum (low|medium|high) (0..1)
  - related_files: string[] (0..*)
  - provenance: object { source_tool?, recorded_via?, recorded_by? } (0..1)
- Notas: usar Record para indexación y metadatos; id y recorded_at deben ser estables.

Mini-ejemplo (genérico):
{
  "@context":"./context.jsonld",
  "id":"urn:bitacora:proj:123",
  "type":"Record",
  "recorded_at":"2026-03-31T10:00:00Z",
  "title":"Registro genérico"
}

2) Decision (subclase de Record)
- Propósito: capturar decisiones arquitectónicas/operativas con su razonamiento.
- Requeridos:
  - id, type="Decision", recorded_at (heredados)
  - title: string (1..1)
  - rationale: string (1..1)
- Opcionales:
  - options_considered: string[] (0..*)
  - chosen_option: string (0..1)
  - effective_from: date (0..1)
  - consequences: string (0..1)
  - impacted_components: string[] (0..*)
  - status, confidence, impact, evidence, links, outcomes (0..*)
- Relaciones comunes: recordedInMeeting -> Meeting, supersedes -> Decision, actionFor -> Action
- Índices sugeridos: title, domain, status, recorded_at, tags

Ejemplo mínimo:
{
  "@context":"./context.jsonld",
  "id":"urn:bitacora:proj:dec-001",
  "type":"Decision",
  "title":"Adoptar Postgres",
  "rationale":"Necesitamos ACID para conciliación",
  "recorded_at":"2026-03-30T15:00:00Z"
}

3) Fact
- Propósito: observación verificable o métrica registrada.
- Requeridos:
  - id, type="Fact", recorded_at
  - observation: string (1..1)
- Opcionales:
  - evidence: Evidence[] (0..*)
  - confidence: number (0..1)
  - related_components, tags, links
- Relaciones: confirms / contradicts -> Decision | Assumption | Fact

Ejemplo:
{
  "id":"urn:bitacora:proj:fact-007",
  "type":"Fact",
  "observation":"La cola X se saturó el 2026-03-28",
  "recorded_at":"2026-03-28T18:00:00Z",
  "confidence":0.95
}

4) Assumption
- Propósito: hipótesis o premisa que debe validarse.
- Requeridos:
  - id, type="Assumption", recorded_at
  - assumption_statement: string (1..1)
- Opcionales:
  - confidence: number (0..1)
  - tests_needed: string[] (0..*)
  - expire_at: date (0..1)
  - evidence, links
- Relaciones: dependsOn / influences -> Decision | Action; verifiedBy -> Fact | Outcome

Ejemplo:
{
  "id":"urn:bitacora:proj:assump-01",
  "type":"Assumption",
  "assumption_statement":"El tráfico mensual no excederá 500k reqs",
  "recorded_at":"2026-03-01T09:00:00Z"
}

5) Meeting
- Propósito: registrar reuniones (asistentes, minutos), decisiones y acciones surgidas.
- Requeridos:
  - id, type="Meeting", recorded_at
  - title: string (1..1)
  - date: string (start or start/end) (1..1)
- Opcionales:
  - location: string (0..1)
  - participants: Person.id[] (0..*)
  - minutes: string (0..1)
  - decisions_made: Record.id[] (0..*)
  - actions: Action.id[] or embedded Action objects (0..*)
  - tags, links, evidence
- Relaciones: decisions_made -> Decision, actions -> Action

Ejemplo:
{
  "id":"urn:bitacora:proj:meet-2026-03-28",
  "type":"Meeting",
  "title":"Arquitectura pagos",
  "date":"2026-03-28T10:00:00Z",
  "participants":["mailto:lead@org"]
}

6) Action (Task)
- Propósito: tarea concreta derivada de una reunión o decisión para seguimiento.
- Requeridos:
  - id, type="Action", recorded_at
  - title: string (1..1)
- Opcionales:
  - assigned_to: Person.id (0..1)
  - due_date: date (0..1)
  - status: enum (todo|in-progress|blocked|done|cancelled) (0..1)
  - outcome: Outcome (0..1)
  - parent_decision: Decision.id (0..1)
  - tags, links
- Relaciones: actionFor / derivedFrom -> Decision | Meeting; assignedTo -> Person

Ejemplo:
{
  "id":"urn:bitacora:proj:act-001",
  "type":"Action",
  "title":"PoC Postgres",
  "assigned_to":"mailto:dev1@org",
  "due_date":"2026-04-07"
}

7) Person
- Propósito: actor humano o agente (autor, participante, responsable).
- Requeridos:
  - id: string (1..1) — email o urn
  - name: string (1..1)
- Opcionales:
  - role: string (0..1)
  - contact: string (phone/other) (0..1)
- Uso: referenciar en author, participants, assigned_to

Ejemplo:
{
  "id":"mailto:dev1@org",
  "type":"Person",
  "name":"Dev Uno",
  "role":"Backend"
}

8) Evidence
- Propósito: enlace o prueba que sustenta un Fact/Decision/etc.
- Estructura (objeto embebido):
  - type: enum (commit|file|issue|screenshot|link|bead) (1..1)
  - reference: string (sha, path, url, id) (1..1)
  - excerpt: string (0..1)
  - date: ISO-8601 (0..1)
  - recorded_by: Person.id (0..1)

Ejemplo:
{"type":"commit","reference":"a1b2c3d","date":"2026-03-29T12:00:00Z"}

9) Outcome
- Propósito: resultado asociado a una Decision o Action (para validar efectividad).
- Campos:
  - status: enum (success|failure|partial) (1..1)
  - notes: string (0..1)
  - duration: number (seconds/minutes) (0..1)
  - recorded_at: ISO-8601 (0..1)
  - agent: string (who recorded) (0..1)

Ejemplo:
{"status":"success","notes":"PoC demostró mejora 20%","recorded_at":"2026-04-10T10:00:00Z"}

10) Link (relación tipada)
- Propósito: representar relaciones entre registros con metadata (rationale, fuerza).
- Campos:
  - relation: enum (relatesTo|supersedes|references|dependsOn|recordedInMeeting|derivedFrom|confirms|contradicts) (1..1)
  - target: Record.id (1..1)
  - rationale: string (0..1)
  - strength: number 0..1 (0..1)
  - recorded_at: date (0..1)

Ejemplo:
{"relation":"supersedes","target":"urn:bitacora:proj:dec-000-old","rationale":"Actualiza política X","strength":0.8}

Relaciones y cardinalidad ejemplos (diagrama pequeño unicode)
- Decision con acciones y reunión:
Decision D
├─ recordedInMeeting -> Meeting M
└─ creates -> Action A1, Action A2
- Meeting con participantes:
Meeting M
├─ participants -> Person P1, P2
├─ decisions_made -> Decision D
└─ actions -> Action A1

Índices recomendados (para index.json / búsqueda rápida)
- id (único), type, title/name, domain, tags, recorded_at, status, assigned_to (para Actions), participants (para Meetings), author, confidence, impact.

Validación y constraints
- Validar con JSON Schema: declarar required para campos por tipo (ej. Decision requiere title+rationale).
- En RDF/OWL: Decision ⊑ Record, etc. (si se opta por OWL más adelante).
- Mantener inmutabilidad parcial: id y created_at no deben cambiar; updates generan updated_at y opcionalmente version.

Buenas prácticas de modelado
- Preferir referencias por id (URI) en vez de anidar objetos completos cuando el objeto existe por separado.
- Embeber solo cuando el objeto no es reutilizable.
- Usar Link para relaciones que requieran justificación o metadata.
- Registrar provenance (quién/qué herramienta escribió el registro) para audit trail.

## Relaciones recomendadas (tipadas)

- relatesTo: vínculo semántico general
- supersedes: reemplaza un registro anterior
- references: referencia documental (archivo/url)
- dependsOn: dependencia de otra decisión/registro
- recordedInMeeting: relaciona decisión/hecho con reunión
- actionFor/assignedTo: conecta acciones con responsables

## Diagrama conceptual (sencillo)

  +-----------------+       relatesTo        +------------------+
  |    Decision A   | <--------------------> |     Fact X       |
  +-----------------+                       +------------------+
        |  \                                / |  
        |   recordedInMeeting             /  |   
        v                                v   
  +--------------+     supersedes     +------------+
  |   Meeting M  | -----------------> | Decision B |
  +--------------+                    +------------+

Grafo con acciones y personas:

  [Meeting M]
       |
  recorded -> [Decision A] <- impacts -- [Component DB]
       |
    creates
       v
    [Action 1] --assignedTo--> (person:dev1@org)

## Formato recomendado: JSON-LD (contexto mínimo)

Usar JSON-LD permite mantener JSON legible y habilitar transformación a RDF si se necesita.

context.jsonld (ejemplo mínimo)
```
{
  "@context": {
    "id": "@id",
    "type": "@type",
    "Record": "https://example.org/ontology/Record",
    "Decision": "https://example.org/ontology/Decision",
    "title": "https://schema.org/name",
    "description": "https://schema.org/description",
    "recorded_at": "https://schema.org/dateCreated",
    "author": {"@id": "https://schema.org/author", "@type": "@id"},
    "tags": "https://schema.org/keywords",
    "relatesTo": {"@id": "https://example.org/ontology/relatesTo", "@type": "@id"},
    "supersedes": {"@id": "https://example.org/ontology/supersedes", "@type": "@id"},
    "evidence": "https://example.org/ontology/evidence",
    "confidence": "https://example.org/ontology/confidence",
    "status": "https://schema.org/ActionStatus"
  }
}
```

## Ejemplos (JSON-LD)

Decision
```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:payments:mx-6f2a1b",
  "type": "Decision",
  "title": "Usar PostgreSQL para pagos",
  "description": "Elegimos Postgres por estabilidad y soporte de transacciones distribuidas.",
  "rationale": "Necesitamos strong consistency para conciliaciones nocturnas; las operaciones toleran latencia adicional.",
  "options_considered": ["SQLite","Postgres","CockroachDB"],
  "chosen_option": "Postgres",
  "domain": "payments",
  "tags": ["database","arch-decision"],
  "recorded_at": "2026-03-30T15:12:00Z",
  "author": "mailto:yo@empresa.com",
  "relatesTo": ["urn:bitacora:payments:mx-a1b2c3"],
  "supersedes": ["urn:bitacora:payments:mx-old123"],
  "evidence": [{"type":"commit","reference":"a1b2c3d","date":"2026-03-29"}],
  "status": "accepted",
  "confidence": 0.9,
  "impact": "high"
}
```

Meeting con acciones
```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:payments:mx-meet-2026-03-28",
  "type": "Meeting",
  "title": "Reunión Arquitectura Pagos",
  "date": "2026-03-28T10:00:00Z",
  "participants": ["mailto:yo@empresa.com","mailto:dev1@empresa.com"],
  "minutes": "Se discutió la migración a Postgres; acordamos crear PoC y timeline.",
  "decisions_made": ["urn:bitacora:payments:mx-6f2a1b"],
  "actions": [
    {"id":"urn:bitacora:payments:mx-act-001","title":"PoC Postgres","assigned_to":"mailto:dev1@empresa.com","due_date":"2026-04-07","status":"in-progress"}
  ]
}
```

## Ejemplos adicionales

Fact (detallado)
```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:fact-042",
  "type": "Fact",
  "observation": "Pico de CPU en servicio checkout a 92% durante el despliegue del 2026-03-25",
  "recorded_at": "2026-03-25T14:37:00Z",
  "evidence": [
    {"type":"link","reference":"https://monitoring.example.com/incident/1234","excerpt":"CPU spike graph"},
    {"type":"file","reference":"/var/logs/checkout/2026-03-25.log","excerpt":"stack trace near 14:35"}
  ],
  "confidence": 0.98,
  "tags": ["monitoring","production","incident"]
}
```

### Assumption -> Test Action -> Fact -> Outcome (flujo)

```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:assump-02",
  "type": "Assumption",
  "assumption_statement": "Las consultas más costosas se ejecutan menos de 10 veces por minuto",
  "recorded_at": "2026-03-20T09:10:00Z",
  "tests_needed": ["Crear test de carga para endpoint /search"],
  "confidence": 0.6
}

{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:act-test-001",
  "type": "Action",
  "title": "Crear PoC test de carga /search",
  "assigned_to": "mailto:perf@org",
  "due_date": "2026-03-27",
  "status": "done",
  "parent_decision": null
}

{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:fact-043",
  "type": "Fact",
  "observation": "El endpoint /search alcanzó 120 consultas/min durante el test de carga",
  "recorded_at": "2026-03-27T11:05:00Z",
  "evidence": [{"type":"file","reference":"/artifacts/perf/test-2026-03-27.json","excerpt":"requests per minute"}],
  "confidence": 0.9
}

{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:outcome-001",
  "type": "Record",
  "recorded_at": "2026-03-28T09:00:00Z",
  "title": "Resultado validación suposición /search",
  "outcomes": [{"status":"failure","notes":"La suposición es falsa: tráfico mayor al estimado","recorded_at":"2026-03-27T12:00:00Z"}],
  "links": [{"relation":"confirms","target":"urn:bitacora:proj:fact-043","rationale":"Test demuestra comportamiento real"}, {"relation":"contradicts","target":"urn:bitacora:proj:assump-02","rationale":"Asunción invalidada por test"}]
}
```

### Action con Outcome (seguimiento)

```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:act-002",
  "type": "Action",
  "title": "Optimizar consulta /search",
  "assigned_to": "mailto:dev-search@org",
  "due_date": "2026-04-20",
  "status": "in-progress",
  "tags": ["perf","backend"],
  "links": [{"relation":"derivedFrom","target":"urn:bitacora:proj:fact-043","rationale":"Root cause from perf test"}]
}

{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:outcome-002",
  "type": "Record",
  "recorded_at": "2026-05-01T10:00:00Z",
  "title": "Optimización /search implementada",
  "outcomes": [{"status":"success","notes":"Reducción 40% en p95","recorded_at":"2026-05-01T09:50:00Z","agent":"mailto:dev-search@org"}],
  "links": [{"relation":"confirms","target":"urn:bitacora:proj:act-002","rationale":"Resultado del trabajo"}]
}
```

Ejemplo de uso de Link con metadata
```
{
  "@context": "./context.jsonld",
  "id": "urn:bitacora:proj:dec-umbrella-01",
  "type": "Decision",
  "title": "Política de cache para resultados de búsqueda",
  "rationale": "Reducir carga en DB y mejorar latencia",
  "recorded_at": "2026-03-29T08:00:00Z",
  "links": [
    {"relation":"dependsOn","target":"urn:bitacora:proj:act-002","rationale":"Depende de optimización de consultas","strength":0.9},
    {"relation":"references","target":"https://docs.internal/cache-policy","rationale":"Documento de referencia"}
  ]
}
```

Mini-grafo (ascii) que muestra flujo Assumption -> Test -> Fact -> Action -> Outcome

Assumption A (urn:bitacora:proj:assump-02)
  └─ triggers -> Action act-test-001
            └─ produces -> Fact fact-043
                      └─ spawns -> Action act-002
                                └─ outcome -> outcome-002



## Import / Export y migración (opcional)

Si necesitas importar registros desde sistemas existentes o exportar el grafo para otras herramientas, recomendamos una capa de import/export desacoplada que haga las siguientes tareas (genéricas, sin dependencia de herramientas concretas):

1. Origen: definir adaptadores para cada fuente (CSV, JSONL, API, export de herramientas internas).
2. Normalización: convertir timestamps a ISO-8601, normalizar nombres de campos y tipos, y validar con el JSON Schema de la ontología.
3. Identificadores: asignar IDs estables (URN o UUID) y no reescribirlos al actualizar un registro.
4. Almacenamiento: escribir cada registro como un fichero independiente en knowledge/<domain>/<id>.jsonld (o un JSONL por dominio si se prefiere).
5. Índice: mantener un índice sencillo (knowledge/index.json) con metadatos para búsquedas rápidas.
6. Export: proporcionar exportadores (JSON-LD, TTL, CSV) para consumir el grafo desde otras herramientas.

Pseudocódigo (idea genérica)
```
for each sourceRecord in source:
  normalized = normalize(sourceRecord)
  if validate(normalized):
    id = normalized.id || generateId(normalized)
    writeFile(`knowledge/${normalized.domain}/${id}.jsonld`, normalized)
    updateIndex(normalized)
```

Notas:
- Mantener el importador como proceso idempotente y con modo dry-run para evitar sobrescrituras accidentales.
- Validar todos los registros con JSON Schema antes de indexarlos.
- Registrar un log de operaciones de import/export para auditoría.
## Índice y búsqueda

- Mantener un índice plano: knowledge/index.json con objetos {id,type,title,domain,tags,recorded_at} actualizado por el script de ingestión.
- Búsqueda simple: ripgrep/jq para queries rápidas; para consultas complejas, exportar a triplestore y usar SPARQL.

## Ejemplo de uso (comandos)

- Encontrar decisiones abiertas: rg -n '"type": "Decision"' knowledge | xargs -n1 jq .
- Validar esquema: usar Ajv con JSON Schema generado a partir de la ontología.

## Validación: JSON Schema (recomendación)

- Generar JSON Schema que represente la unión de subtipos (Decision, Meeting, Action, etc.) para validar en ingestión.
- Opcional: usar SHACL si finalmente se usa RDF/graph DB.

## Buenas prácticas

- Archivo por registro facilita PRs, blame y revisiones.
- IDs estables (no regenerar al reescribir).
- Mantener set limitado de relaciones tipadas.
- Validar con JSON Schema en cada `record`/`ingest`.
- Registrar provenance (registered_by, recorded_via) para auditoría.
- Registrar reuniones con actions como registros independientes (fácil seguimiento).

## Extensiones útiles (futuro)

- Riesgos y mitigaciones (Risk nodes) enlazados a Decisions.
- Cost/Benefit estimado en Decisions.
- Milestones / Releases vinculando decisiones y acciones.
- Mecanismo de confirmación: outcomes vinculados para trackear si una decisión fue efectiva.

## Diagrama de flujo de almacenamiento e import/export (unicode)

  [source data / exporter]
           |
           v
  [importer] ---> knowledge/<domain>/<id>.jsonld
           |
           v
       knowledge/index.json     (opcional: triple-store export) 

## Ejemplo ASCII de vínculo entre objetos

  urn:bitacora:payments:mx-meet-2026-03-28
  ├─ decisions_made -> urn:bitacora:payments:mx-6f2a1b
  └─ actions -> urn:bitacora:payments:mx-act-001

  urn:bitacora:payments:mx-6f2a1b
  ├─ supersedes -> urn:bitacora:payments:mx-old123
  └─ relatesTo -> urn:bitacora:infra:mx-db-pattern

## Siguientes pasos sugeridos

- Confirma la ontología base (¿algo que quieras cambiar, añadir o renombrar?).
- Opciones de entrega si confirmas:
  1) Generar context.jsonld + JSON Schema (archivo) para validación e interoperabilidad.
  2) Escribir importadores/exportadores genéricos (Node.js) para fuentes comunes (CSV, JSONL, API) y un modo dry-run para validar sin escribir.
  3) Tests básicos de validación y ejemplos reales extraídos de tus registros actuales (pruebas en modo dry-run).

Si quieres que genere estos artefactos (context.jsonld, schema, scripts y tests), dime y los creo en el repositorio y ejecuto pruebas en modo dry-run sobre tus datos de origen.

---

Documento generado por petición. Si prefieres otro formato (OWL/TTL, RDF/XML, o mantener JSONL por dominio), lo adapto.
