---
name: log_diario-separation
description: "Skill que define heurísticas para separar notas en propuestas atómicas y documentar nuevas heurísticas en Mulch cuando correspondan."
---

# Skill: Log Diario — Separación de notas (heurísticas)

Propósito
---------
Proveer una colección de heurísticas reutilizables para segmentar notas libres en propuestas atómicas que luego la extensión `log_diario` transforma en registros Mulch.

Comportamiento deseado (resumen)
--------------------------------
- Dado un texto libre (nota), la skill aplica heurísticas (viñetas, numeración, 'para ...:', punto y coma, límites de oraciones) para generar una lista de propuestas.
- Si durante la segmentación se detecta una heurística nueva que no estaba registrada, la extensión puede crear una referencia en Mulch (dominio: `log-diario-heuristics`) para documentarla y facilitar su evolución.

Integración con la extensión
----------------------------
- La extensión `log_diario` delega la separación a la función `separateNote(note)` exportada por la implementación de la skill.
- `separateNote` devuelve un objeto con: `proposals: string[]`, `heuristicsUsed: string[]` y `newlyRegistered: string[]`.
- El registro automático de heurísticas en Mulch se controla por la variable de entorno `LOG_DIARIO_REGISTER_HEURISTICS=1`. Si no está activada, la skill sólo actualiza una copia local (registro) y deja la creación de records Mulch para un paso manual.

Formato de las heurísticas
-------------------------
Cada heurística tiene un identificador corto (`id`), un `name` y una `description` breve. La skill mantiene un registro local en `.pi/extensions/log_diario/heuristics_registry.json` para evitar crear duplicados.

Política para registrar en Mulch
--------------------------------
- Sólo registrar heurísticas automáticamente si la variable de entorno `LOG_DIARIO_REGISTER_HEURISTICS` está en `'1'`.
- Los registros se crean bajo el dominio `log-diario-heuristics` con tipo `pattern` y una descripción que documenta el patrón detectado.
- La creación es sin archivos asociados (`--files ""`) por defecto; si se quiere asociar ejemplos o rutas, esto se debe hacer manualmente o extender la skill para incluir ejemplos.

Extensibilidad
--------------
- Para agregar o mejorar heurísticas, editar la implementación (o exponer una configuración en el repositorio) y actualizar la registry si corresponde.
- Opcional: exponer una pequeña API o comando (`/log_diario/heuristics`) para listar, aprobar y sincronizar heurísticas documentadas en Mulch.

Notas operativas
----------------
- La skill debe ser conservadora: no dividir en exceso y permitir siempre la revisión humana antes de crear registros Mulch.
- Evitar registrar patrones que expongan información sensible.


Fecha: 2026-03-28
Autor: Equipo de desarrollo — Extensión log_diario
