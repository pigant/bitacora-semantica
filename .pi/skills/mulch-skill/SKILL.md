---
name: mulch-skill
description: "Genera conocimiento accionable para una base de conocimiento: detecta hallazgos relevantes, prioriza registros atómicos, identifica archivos afectados y crea entradas individuales vinculadas por globs; puede usarse tanto para guardarla como para consultarla cuando falte contexto."
---

# Skill: Mulch (agente)

## Propósito

Apoyar al agente en convertir cambios de código y hallazgos en conocimiento estructurado dentro de `.mulch/expertise/`.

## Principios clave (para el agente)

- **Siempre** registra cada hallazgo o lección por separado: un cambio = un record.
- Usa `ml prime` para cargar contexto antes de trabajar.
- Usa `ml learn` para encontrar qué cambió y qué falta documentar.
- Usa `ml record` para capturar el conocimiento (con `--files` cuando sea posible).
- Mantén `.mulch/` consistente con `ml validate` y `ml sync`.

---

## Flujo de trabajo recomendado

### 1) Cargar contexto (siempre primero)

```bash
ml prime
# o para enfoque puntual:
ml prime --files "src/modules/basket/**"
```

### 2) Detectar cambios a revisar

```bash
ml learn --since <ref>
```

- Muestra qué archivos cambiaron, qué dominios sugieren y qué archivos no están asociados.
- NO muestra “lo aprendido”, solo qué vale la pena revisar.

### 3) Crear registros (uno por cada hallazgo)

Ejemplo base:
```bash
ml record <domain> --type <type> --name "<titulo>" \
  --description "<qué y por qué>" \
  --files "<glob>"
```

**Nota importante:** cada idea/hallazgo debe ser un record independiente. No combines múltiples lecciones en un solo registro.

### 4) Validar & versionar

```bash
ml validate
ml sync
```

---

## Comandos clave (y cuándo usarlos)

### `ml learn` — detectar cambios sin registrar

- Objetivo: identificar archivos cambiados que no están asociados a registros existentes.
- Útil para decidir qué registrar mediante `ml record`.

Ejemplo:
```bash
ml learn --since main
```

### `ml record` — capturar conocimiento

- Usa siempre `--files` si quieres que Mulch relacione el registro con archivos específicos.
- **Registra cada hallazgo de forma individual**.

Tipos frecuentes:
- `convention` → reglas de estilo / arquitectura.
- `pattern` → flujos repetibles / pipeline / decoradores.
- `failure` → error + resolución.
- `decision` → decisión arquitectónica (requiere `--title` y `--rationale`).
- `reference` → información de referencia (endpoints, archivos, datos clave).
- `guide` → pasos de ejecución/procedimiento.

### `ml prime` — carga contexto

- Carga toda la base de conocimiento:
  ```bash
  ml prime
  ```
- Para enfocar en archivos específicos:
  ```bash
  ml prime --files "src/modules/**"
  ```

### `ml sync` — sincronizar cambios

- Valida y guarda cambios en `.mulch/`:
  ```bash
  ml sync
  ```

---

## Comandos de apoyo útiles

- `ml validate`: valida la estructura de los registros.
- `ml status`: muestra estado de dominios y advertencias.
- `ml query <domain>`: listar registros de un dominio.
- `ml search <query>`: buscar texto en registros (BM25).
- `ml prune --dry-run`: encontrar registros obsoletos.
- `ml doctor --fix`: arreglar formato/duplicados.
- `ml diff <ref>`: ver cambios en `.mulch/` entre refs.


### Buscar un ID o confirmar contexto

Para encontrar rápidamente un registro por su identificador dentro de un dominio o confirmar que un ID aparece en la salida de `ml query`, use:

```bash
mulch query <DOMINIO> | grep <ID>
```

Esto es útil cuando el dominio ya está identificado pero necesitas verificar la presencia o el título resumido del bead (por ejemplo `mx-363c14`). Alternativamente, si ya ejecutaste `ml prime`, puedes usar el contexto primado por `ml prime` para orientar la búsqueda y evitar búsquedas globales.

---

## Asociar archivos a dominios

Para que `ml learn` sugiera un dominio, debe existir un record que mencione los archivos con `--files`.

Ejemplo:
```bash
ml record basket --type reference --name "Estructura del módulo basket" \
  --description "Archivos y pipeline de basket" \
  --files "src/modules/basket/**"
```

A partir de entonces, cambios en `src/modules/basket/**` aparecerán sugeridos bajo `basket`.

---

## Criterios para decidir qué registrar

Registra cuando el cambio implica:
- una decisión (arquitectura, dependencias, etc.).
- un patrón reutilizable o un proceso común.
- un error y su resolución (incluyendo ajustes en dependencias).
- una referencia importante (endpoint, contrato, formato de datos).

No registres:
- cambios triviales o renombres sin impacto.
- detalles que no aportan valor generalizable.
- notas personales sin valor para otros.

---

## Consejos para búsquedas efectivas

Para que los registros se encuentren con `ml search`, usa:
- palabras clave técnicas (ej. `retry`, `circuit breaker`, `timeout`).
- sinónimos (ej. `timeout` / `time out`).
- nombres de archivos, módulos o endpoints.
- frases claras y concisas.
- `ml diff main..HEAD` para ver qué se agregó/cambió en `.mulch/` entre commits.
- `ml search <query>` para encontrar outcomes, fallos o decisiones similares.

Esto evita duplicados y permite usar outcomes existentes como base para nuevas acciones.

---

### `ml upgrade [options]`
**Qué hace:** Actualiza Mulch CLI (si está instalado globalmente) o verifica versión.

- ✅ **Usar cuando:** quieres la última versión.
- ❌ **No usar cuando:** prefieres mantener la versión fija.

Ejemplo:
```bash
ml upgrade --check
ml upgrade
```

> Nota: existe `ml update` pero está deprecado; usa `ml upgrade`.

### `ml completions <shell>`
**Qué hace:** Genera script de autocompletado para bash/zsh/fish.

- ✅ **Usar cuando:** quieres mejorar la experiencia en la CLI.

Ejemplo:
```bash
ml completions bash > ~/.bashrc
```

### `ml delete <domain> <id>` (borrar registros)
**Qué hace:** elimina uno o varios registros de un dominio.

**Uso básico:**
```bash
ml delete <domain> <id>
```

**Opciones útiles:**
- `--records <ids>` — borrar varios IDs a la vez (coma-separados).
- `--all-except <ids>` — borrar *todo* excepto los IDs listados.
- `--dry-run` — ver qué se borraría sin tocar nada.

**Ejemplos:**
```bash
ml delete pipeline mx-abc123
ml delete pipeline --records mx-abc123,mx-def456
ml delete pipeline --all-except mx-abc123
ml delete pipeline --records mx-abc123 --dry-run
```

> **Nota**: no existe un comando `ml` para borrar un dominio completo. Para eliminarlo, borra el archivo `.mulch/expertise/<domain>.jsonl` y, si usas un config fijo, quita el dominio de `.mulch/mulch.config.yaml`.

### `ml outcome <domain> <id>` (registrar resultado)
**Qué hace:** anexa un resultado (success/failure/partial) a un registro existente para indicar si aplicar ese conocimiento tuvo efecto.

**Uso básico:**
```bash
ml outcome <domain> <id> --status <success|failure|partial> --notes "<comentario>"
```

**Ejemplo:**
```bash
ml outcome pipeline mx-abc123 --status success --notes "Mejoró la estabilidad del cron job"
```

## ✅ Consejos clave para no generar ruido

### ✨ Cuando sí grabar
- Decisiones de diseño (p.ej. “por qué usamos runPipelineParallel”).
- Patrones reutilizables (p.ej. “cómo se construye la pipeline de basket”).
- Fallos recurrentes y su solución.
- Referencias de archivos clave (p.ej. “dónde está la lógica del endpoint Uber”).

### 🚫 Cuando NO grabar
- Cambios pequeños/estéticos sin valor de aprendizaje.
- Comentarios personales o TODOs.
- Bugs sin resolución clara (mejor issue/beta testing).
- Grabaciones que sólo repiten documentación existente sin aportar contexto nuevo.

## 📎 Cómo vincular archivos a dominios (para que `ml learn` sugiera dominios)

`ml learn` solo sugiere un dominio si existe al menos un registro en ese dominio con `--files` que coincida con el archivo cambiado.

Ejemplo habitual:

```bash
ml record basket --type reference --name "Estructura del módulo basket" \
  --description "Archivos principales del módulo basket y dónde está la lógica de cálculo" \
  --files "src/modules/basket/**"
```

Después de esto, si cambias cualquier archivo bajo `src/modules/basket/`, `ml learn` sugerirá automáticamente el dominio `basket`.

## 🧩 Checklist rápida (ideal para PRs)

- [ ] Ejecuté `ml learn` y revisé los archivos detectados.
- [ ] Registré (o decidí no registrar) conocimientos relevantes con `ml record`.
- [ ] Confirmé que `.mulch/` no contiene errores con `ml validate`.
- [ ] Hice `ml sync` si cambié `.mulch/`.

