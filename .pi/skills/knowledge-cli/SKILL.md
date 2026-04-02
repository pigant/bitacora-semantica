---
name: knowledge-cli
description: "Skill para operar knowledge-cli: crear, vincular, consultar y validar registros JSON-LD con buenas practicas de indice y grafo"
---

# Skill: knowledge-cli

## Proposito
Esta skill define como usar `knowledge-cli` de forma consistente para:
- crear registros atomicos por dominio,
- vincular registros con relaciones validas,
- consultar por id, filtros, texto y grafo,
- validar integridad de archivos y del grafo.

## Cuando invocar
Invocar esta skill cuando necesites:
- registrar decisiones, hechos, supuestos, reuniones, acciones o personas,
- explorar relaciones entre registros,
- buscar informacion en la base JSON-LD,
- detectar links rotos o rutas entre nodos.

## Flujo recomendado
1. Crear registros atomicos con `create <tipo>`.
2. Relacionar registros con `link`.
3. Validar registros con `validate`.
4. Consultar con `show`, `list`, `links`, `neighbors`, `search`, `path`.
5. Validar integridad global con `validate-graph`.

## Reglas operativas
- Un registro = un archivo JSON-LD.
- El indice vive en `knowledge/index.json`.
- Los links se guardan en el registro origen (`from`).
- Usar ids estables tipo `urn:log:<domain>:<uuid>`.
- Preferir `--domain` explicito para facilitar filtrado.

## Create: variantes y requeridos

### Opcionales de ontologia (aplican a create decision/fact/assumption/meeting/action)
- `--classification foundational|tactical|observational`
- `--related-files a,b,c`
- `--evidence-json '{"type":"issue","reference":"#123"}'` (repetible)
- `--outcomes-json '{"status":"success","notes":"ok"}'` (repetible)
- `--provenance-json '{"source_tool":"knowledge-cli","recorded_via":"manual","recorded_by":"mailto:dev@org"}'`
- Alias de compatibilidad:
  - `--project` equivale a `--domain`
  - `--body` equivale a `--description`

### create decision
Requeridos:
- `--title`
- `--rationale`

Ejemplo:
```bash
knowledge-cli create decision \
  --title "Adoptar Postgres" \
  --rationale "Necesitamos ACID" \
  --domain payments
```

### create fact
Requerido:
- `--observation`

Ejemplo:
```bash
knowledge-cli create fact \
  --observation "Cola X saturada en horario punta" \
  --domain payments \
  --confidence 0.95 \
  --related-components queue,worker \
  --evidence-json '{"type":"issue","reference":"#42"}'
```

### create assumption
Requerido:
- `--assumption-statement`

Ejemplo:
```bash
knowledge-cli create assumption \
  --assumption-statement "El trafico crecera gradualmente" \
  --domain payments \
  --confidence 0.7 \
  --expire-at 2027-03-31
```

### create meeting
Requeridos:
- `--title`
- `--date`

Ejemplo:
```bash
knowledge-cli create meeting \
  --title "Payments weekly" \
  --date 2026-03-31 \
  --domain payments
```

### create action
Requerido:
- `--title`

Ejemplo:
```bash
knowledge-cli create action \
  --title "Afinar worker de cola" \
  --domain payments \
  --status todo \
  --due-date 2026-04-15 \
  --outcome-json '{"status":"partial","notes":"requiere seguimiento"}'
```

### create person
Requerido:
- `--name`

Ejemplo:
```bash
knowledge-cli create person \
  --name "Alice Example" \
  --role Engineer
```

## Vinculos

### link
Requeridos:
- `--from`
- `--to`
- `--relation`

Ejemplo:
```bash
knowledge-cli link \
  --from urn:log:payments:AAA \
  --to urn:log:payments:BBB \
  --relation relatesTo
```

## Consultas

### show
```bash
knowledge-cli show --id <ID>
```

### list
```bash
knowledge-cli list --domain payments --type Decision
knowledge-cli list --tags db,critical --limit 50 --offset 0
```

### links
```bash
knowledge-cli links --from <ID>
knowledge-cli links --to <ID> --relation dependsOn
```

### neighbors
```bash
knowledge-cli neighbors --id <ID> --depth 2
```

### search
```bash
knowledge-cli search --query postgres --in Decision,Fact --limit 20
```

### path
```bash
knowledge-cli path --from <ID> --to <ID> --max-depth 10
```

## Validacion

### validate (archivo)
```bash
knowledge-cli validate --file knowledge/payments/registro.jsonld
```

### validate-graph (base completa)
```bash
knowledge-cli validate-graph --report text
knowledge-cli validate-graph --report json
```

## Reglas y restricciones

### Enums y restricciones
- Record status (Decision y campos generales de record): `proposed | accepted | rejected | in-progress | done | deprecated`
- Action status: `todo | in-progress | blocked | done | cancelled`
- Impact: `low | medium | high`
- Classification: `foundational | tactical | observational`
- Relation (link): `relatesTo | supersedes | references | dependsOn | recordedInMeeting | derivedFrom | confirms | contradicts | actionFor | assignedTo`
- Evidence types: `commit | file | issue | screenshot | link | bead`
- Outcomes / action.outcome.status: `success | failure | partial`

### Reglas de fecha y hora
- `recorded_at`, `link.recorded_at`, `outcome.recorded_at`, `evidence.date`: RFC3339 (ISO-8601 completo).
- `effective_from`, `due_date`, `expire_at`, `meeting.date`: `YYYY-MM-DD` o RFC3339.

### Reglas de consulta de grafo
- `neighbors`: `--depth` en rango `1..5`.
- `path`: `--max-depth` en rango `1..20` (camino dirigido `from -> to`).
- `links`: requiere al menos `--from` o `--to`.

### Notas de validación y seguridad
- `id` debe ser único; `create` falla si ya existe.
- `confidence` debe ser numérico y estar entre `0.0` y `1.0`.
- `duration` en `outcomes` debe ser numérico y >= `0`.
- Arrays tipados (`tags`, `related-files`, `related-components`, etc.) deben contener strings.
- Flags que aceptan JSON (`--evidence-json`, `--outcomes-json`, `--provenance-json`, `--outcome-json`) deben ser JSON válidos y respetar su esquema.
- `link` valida existencia de `--from` y `--to` en el índice.
- `validate-graph` retorna exit code != 0 si hay referencias rotas.

### Combinación de ejemplo
```bash
knowledge-cli create decision \
  --title 'Adoptar Postgres' \
  --rationale 'Necesitamos ACID' \
  --domain payments \
  --classification tactical \
  --related-files 'src/db.rs,docs/adr.md' \
  --evidence-json '{"type":"issue","reference":"#123"}' \
  --outcomes-json '{"status":"partial","notes":"pendiente"}'
```

### Códigos de salida
- `0` operación exitosa
- `!=0` error de validación, parseo o integridad

### Checklist operativo recomendado
1. Crear registros atómicos por dominio.
2. Vincular para trazar relaciones (usar `link`).
3. Ejecutar `validate` por archivo nuevo/modificado.
4. Ejecutar `validate-graph` periódicamente para salud del grafo.
5. Verificar con `list/search/path` para inspección funcional.

## Skill mode
- Guia general:
```bash
knowledge-cli --skill
```
- Guia por dominio:
```bash
knowledge-cli --skill payments
```

## Contraejemplos comunes
- `create decision` sin `--title` o sin `--rationale`.
- `create action --status invalid` (usar estados permitidos).
- `create meeting --date 31-03-2026` (usar `YYYY-MM-DD` o RFC3339).
- `link --relation causes` si la relacion no esta en el set permitido.

## Checklist de cierre tecnico
Cuando se hagan cambios al codigo de `knowledge-cli`, cerrar con:
```bash
cargo check -p knowledge-cli
cargo clippy -p knowledge-cli -- -D warnings
cargo test -p knowledge-cli
```

## Regla final
Si hay duda entre tipos o relaciones, preferir:
1. registrar primero un `fact` claro,
2. luego una `decision` con rationale,
3. y finalmente conectar con `link` para trazabilidad.
