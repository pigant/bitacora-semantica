---
name: mulch-skill
description: "Registra y consulta bitácora diaria por proyecto (dominio)"
---

# Skill: Mulch (bitácora diaria de proyectos)

## Objetivo
Esta skill está orientada a **guardar registros diarios de trabajo**.

Regla principal de modelado:
- Cada **dominio = un proyecto**.
- Usar `general` solo para trabajo transversal (sin proyecto único).

---

## Modelo de registro recomendado

Un registro debe capturar una unidad de trabajo diaria con valor de seguimiento:
- Qué se hizo.
- Por qué se hizo.
- Evidencia/contexto (personas, archivos, links internos, impacto).

### Plantilla base
```bash
ml record <dominio_proyecto> --type <type> --title "<titulo breve>" \
  --rationale "<motivo o contexto de negocio/técnico>" \
  --description "<detalle de lo ejecutado y resultado>" \
  --files "<glob opcional>"
```

> Si no hay archivos, **omitir `--files`**.

---

## Política de dominios (importante)

1. Antes de registrar, verificar dominios existentes:
```bash
ml status
```

2. Selección de dominio:
- Si el trabajo pertenece claramente a un proyecto: usar ese dominio.
- Si involucra varios proyectos o tareas transversales: usar `general`.

3. No crear dominios ambiguos tipo `misc`, `otros`, `tmp`.

---

## Tipos recomendados para bitácora diaria

- `guide`: procedimiento ejecutado (pasos operativos del día).
- `decision`: decisión tomada (requiere justificación clara).
- `failure`: incidente/error + resolución aplicada.
- `reference`: dato operativo de referencia para el proyecto.
- `pattern`: forma repetible de operar o resolver tareas.

---

## Criterio de atomicidad

- Un registro = una unidad de trabajo entendible por sí sola.
- Si en el día hubo 3 cambios distintos en un proyecto, crear 3 registros.
- No mezclar en un mismo registro: incidente + decisión + guía no relacionada.

---

## Flujo recomendado diario

1. Cargar contexto del repo/proyecto:
```bash
ml prime
```

2. Revisar estado y dominios:
```bash
ml status
```

3. Registrar cada hallazgo diario de forma atómica:
```bash
ml record <dominio> --type <type> --title "..." --rationale "..." --description "..."
```

4. Validar base:
```bash
ml validate
```

---

## Consulta y seguimiento

- Ver registros del proyecto:
```bash
ml query <dominio>
```

- Buscar por texto en toda la base:
```bash
ml search "<consulta>"
```

- Registrar resultado posterior de una decisión/acción:
```bash
ml outcome <dominio> <id> --status <success|failure|partial> --notes "<resultado observado>"
```

---

## Buenas prácticas específicas de bitácora

### Sí registrar
- Instalaciones, despliegues, migraciones, cambios de configuración.
- Incidentes del día y cómo se resolvieron.
- Decisiones operativas con impacto.
- Coordinación relevante entre personas/equipos.

### No registrar
- Cambios triviales sin impacto.
- Notas personales sin contexto de proyecto.
- Entradas duplicadas del mismo hecho.

---

## Convenciones de títulos (sugeridas)

Formato sugerido:
- `YYYY-MM-DD: acción principal en <proyecto/sitio>`

Ejemplos:
- `2026-03-31: instalación de pinpad en cenco florida`
- `2026-03-31: ajuste de configuración kiosco por timeout`
- `2026-03-31: resolución de fallo de enrolamiento`

---

## Ejemplos

### Registro diario por proyecto
```bash
ml record kiosco --type guide --title "2026-03-31: instalación de pinpad en cenco florida" \
  --rationale "Habilitar kioscos nuevos para operación comercial" \
  --description "Se instalaron equipos recibidos el día anterior y se validó operación básica en sala" \
  --files "operaciones/kiosco/**"
```

### Registro transversal
```bash
ml record general --type decision --title "2026-03-31: priorización de soporte en horario punta" \
  --rationale "Reducir impacto en atención de tiendas con mayor volumen" \
  --description "Se definió ventana de atención y criterio de escalamiento para incidencias compartidas"
```

---

## Regla operativa final

Si hay duda de dominio:
1) intenta mapear al proyecto real,
2) si no es posible, usa `general`,
3) documenta en `description` por qué fue transversal.
