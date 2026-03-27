# AGENTS

Este repositorio usa Mulch (.mulch/) como la base de conocimiento para registrar decisiones, fallos, patrones y referencias técnicas.

Propósito
--------
Registrar y consultar conocimiento relevante generado por el equipo o por agentes automatizados. Mulch permite mantener registros atómicos vinculados a archivos del código, facilitando búsqueda, priming y priming para agentes.

Uso recomendado
--------------
- Antes de crear registros, ejecutar `ml prime` para cargar contexto cuando sea necesario.
- Registrar cada hallazgo de forma atómica con `ml record <domain> --type <type> ... --files "<glob>"`.
  - Preferir registrar un solo hallazgo por comando.
  - Incluir globs con `--files` para asociar registros a archivos y mejorar las sugerencias de dominio.

Roles y responsabilidades
-------------------------
- Personas: todo miembro del equipo debe crear registros al introducir cambios significativos, decisiones o resoluciones de fallos.
- Agentes/CI: los agentes pueden ejecutar `ml learn` y preparar sugerencias; siempre crear registros atómicos tras revisión humana si procede.

Buenas prácticas
---------------
- Registros atómicos: un registro = una idea/decisión/fallo.
- Usar lenguaje claro, títulos concisos y descripciones que expliquen el porqué y la resolución.
- Siempre prefiere utilizar dominios conocidos los dominios los vas a encontrar con `ml status`

Ejemplo de comando
------------------
ml record operaciones --type decision --title "Título corto" --rationale "Razonamiento breve" --description "Descripción con contexto y resolución" --files "src/servicio/**"

Dónde se almacena
-----------------
Los registros se guardan en la carpeta `.mulch/` del repositorio.

Contacto
--------
Para dudas sobre flujo Mulch consultar .pi/skills/mulch-skill/SKILL.md.
