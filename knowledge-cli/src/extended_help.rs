pub fn print_extended_help() {
    let text = r##"NAME
    knowledge-cli - gestor de conocimiento JSON-LD con consultas de grafo

SYNOPSIS
    knowledge-cli [--knowledge-dir PATH] <COMMAND> [OPTIONS]
    knowledge-cli --skill [DOMINIO]
    knowledge-cli extended-help

DESCRIPTION
    knowledge-cli crea, vincula, valida y consulta registros JSON-LD de tipo
    Decision, Fact, Assumption, Meeting, Action y Person.

    Arquitectura de datos:
    - Un registro por archivo JSON-LD
    - Index de metadatos en knowledge/index.json
    - Relaciones salientes en links[] del registro origen

GLOBAL OPTIONS
    --knowledge-dir PATH
        Directorio base de conocimiento (default: knowledge)

    --skill[=DOMINIO]
        Guia rapida operativa. Si no se pasa dominio usa "general".

COMMANDS
    create <tipo>
        Crea registros por tipo. Tipos: decision, fact, assumption, meeting,
        action, person.

    link
        Agrega una relacion tipada entre dos IDs existentes.

    validate
        Valida sintaxis JSON-LD y reglas de negocio de un archivo.

    show
        Muestra un registro completo por ID.

    list
        Lista desde index con filtros por metadatos.

    links
        Consulta aristas del grafo por origen/destino/relacion.

    neighbors
        Recorre vecinos en el grafo hasta cierta profundidad.

    search
        Busca texto libre sobre campos indexables y metadatos extendidos.

    validate-graph
        Revisa links rotos, nodos huerfanos y ciclos.

    path
        Busca camino dirigido entre dos nodos.

    extended-help
        Muestra esta documentacion completa.

CREATE REFERENCE
    Requeridos por subtipo:
    - decision:   --title --rationale
    - fact:       --observation
    - assumption: --assumption-statement
    - meeting:    --title --date
    - action:     --title
    - person:     --name

    Opcionales comunes (decision/fact/assumption/meeting/action):
    --id STRING
    --recorded-at RFC3339
    --domain STRING
    --project STRING               (alias de --domain)
    --description STRING
    --body STRING                  (alias de --description)
    --tags a,b,c
    --author STRING
    --status STRING                (segun tipo)
    --confidence FLOAT             (0.0..1.0)
    --impact STRING                (low|medium|high)
    --classification STRING        (foundational|tactical|observational)
    --related-files a,b,c
    --evidence-json JSON           (repetible)
    --outcomes-json JSON           (repetible)
    --provenance-json JSON
    --context PATH                 (default: ./context.jsonld)

    Opcionales especificos:
    - decision:   --options-considered a,b --chosen-option X --effective-from DATE
                  --consequences TEXT --impacted-components a,b
    - fact:       --related-components a,b
    - assumption: --tests-needed a,b --expire-at DATE
    - meeting:    --location X --participants a,b --minutes X --decisions-made a,b
                  --actions a,b
    - action:     --assigned-to X --due-date DATE --parent-decision ID
                  --outcome-json JSON
    - person:     --role X --contact X

ENUMS AND RESTRICTIONS
    Record status (Decision y campos status generales de record):
        proposed | accepted | rejected | in-progress | done | deprecated

    Action status:
        todo | in-progress | blocked | done | cancelled

    impact:
        low | medium | high

    classification:
        foundational | tactical | observational

    relation (link):
        relatesTo | supersedes | references | dependsOn | recordedInMeeting |
        derivedFrom | confirms | contradicts | actionFor | assignedTo

    evidence.type:
        commit | file | issue | screenshot | link | bead

    outcomes.status y action.outcome.status:
        success | failure | partial

DATE AND TIME RULES
    recorded_at, link.recorded_at, outcome.recorded_at, evidence.date:
        RFC3339 (ISO-8601 completo)

    effective_from, due_date, expire_at, meeting.date:
        YYYY-MM-DD o RFC3339

GRAPH QUERY RULES
    neighbors:
        --depth requerido en rango 1..5

    path:
        --max-depth en rango 1..20
        el camino es dirigido (from -> to)

    links:
        requiere al menos uno de --from o --to

VALIDATION AND SAFETY NOTES
    - ID debe ser unico; si ya existe se rechaza create.
    - confidence debe ser numerico y estar entre 0.0 y 1.0.
    - duration en outcomes debe ser numerico y >= 0.
    - Arrays tipados (tags, related-files, related-components, etc.) deben
      contener strings.
    - Campos JSON (evidence-json, outcomes-json, provenance-json, outcome-json)
      deben ser JSON valido y respetar su esquema.
    - link valida existencia de --from y --to en index.
    - validate-graph falla (exit code != 0) si hay referencias rotas.

COMBINATION EXAMPLES
    Crear decision con ontologia extendida:
        knowledge-cli create decision \
          --title 'Adoptar Postgres' \
          --rationale 'Necesitamos ACID' \
          --project payments \
          --classification tactical \
          --related-files 'src/db.rs,docs/adr.md' \
          --evidence-json '{"type":"issue","reference":"#123"}' \
          --outcomes-json '{"status":"partial","notes":"pendiente"}'

    Vincular y validar:
        knowledge-cli link --from <ID_A> --to <ID_B> --relation dependsOn
        knowledge-cli validate --file knowledge/payments/<archivo>.jsonld
        knowledge-cli validate-graph --report json

    Consultas combinadas:
        knowledge-cli list --domain payments --classification tactical --type Decision
        knowledge-cli search --query postgres --in Decision,Fact --limit 20
        knowledge-cli path --from <ID_A> --to <ID_B> --relation dependsOn --max-depth 8

EXIT CODES
    0   operacion exitosa
    !=0 error de validacion, parseo o integridad

RECOMMENDED CHECKLIST
    1) create atomico por registro
    2) link de trazabilidad
    3) validate por archivo
    4) validate-graph para salud global
    5) list/search/path para verificacion funcional"##;
    println!("{text}");
}
