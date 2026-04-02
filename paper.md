Título
======

Log Diario — Integración de extensiones con Mulch

Resumen
-------
Este documento describe, a alto nivel, los objetivos y el comportamiento esperado de la extensión "log_diario" para el agente pi. La extensión tiene como propósito transformar notas y entradas libres en registros estructurados para Mulch, facilitando el registro atómico de hallazgos, decisiones y fallos técnicos. Se presenta la filosofía de diseño, el flujo de interacción con el usuario y con Mulch, y los beneficios esperados, sin entrar en detalles de implementación.

1. Objetivo
-----------
La extensión pretende convertir entradas de texto (notas de reuniones, reportes de incidentes, ideas y acciones) en registros Mulch listos para almacenar. El enfoque es mantener registros atómicos, rastreables y vinculados a archivos o ámbitos del repositorio mediante globs (patrones sencillos de rutas que identifican áreas del código).

2. Principios de diseño
-----------------------
- Automatización asistida: generar propuestas automáticas a partir de contenido libre, pero siempre con confirmación humana final.
- Registros atómicos: cada hallazgo o decisión se transforma en un registro independiente para Mulch.
- Trazabilidad: asociar registros con metadatos (dominio, tipo, fecha, participantes) y, cuando sea relevante, con rutas o patrones que indiquen la zona del repositorio.
- No intrusivo: la extensión propone y facilita la ejecución del comando ml record, pero no impone cambios sin la aprobación del usuario.

3. Flujo de interacción (esquema general)
-----------------------------------------
- Entrada: el usuario pega o envía una nota al agente o invoca la extensión sobre una selección.
- Extracción: la extensión detecta posibles propuestas (líneas, oraciones o bloques que correspondan a hallazgos/decisiones/acciones).
- Normalización: cada propuesta se normaliza en un objeto con campos sugeridos: dominio, título, descripción, fecha, participantes, archivos, tipo.
- Presentación: la extensión muestra al usuario un resumen o vista previa de las propuestas y genera los comandos ml correspondientes.
- Confirmación: el usuario puede confirmar todas, confirmar parcialmente, editar una propuesta, pedir un preview o cancelar.
- Persistencia: tras la confirmación, la extensión ejecuta (o sugiere ejecutar) los comandos ml record para guardar en Mulch, y opcionalmente valida y sincroniza el índice Mulch.

4. Interacción con Mulch (visión general)
-----------------------------------------
- Generación de comandos: la extensión construye comandos ml record con el dominio y metadatos sugeridos.
- Asociación a archivos: las propuestas pueden incluir rutas o patrones (globs) que enlacen el registro a archivos relevantes en el repositorio, mejorando la relevancia en búsquedas.
- Registro atómico: cada comando ml crea una entrada independiente en la base Mulch, siguiendo la recomendación de registros atomizados.
- Validación y sincronización: opcionalmente, tras crear registros, se puede invocar ml validate y ml sync para mantener la consistencia e índices actualizados.

5. Beneficios esperados
----------------------
- Reducción de fricción al registrar conocimiento operativo y técnico.
- Mayor consistencia en la estructura de registros (campos normalizados como dominio, tipo, fecha).
- Facilita la búsqueda y priming para agentes al tener registros atómicos y vinculados a archivos o áreas del código.
- Permite trazabilidad de decisiones y fallos ligados al contexto del código o documentación.

6. Riesgos y limitaciones
-------------------------
- Calidad del NLP: la extracción automática puede proponer registros irrelevantes o mal categorizados; por eso la confirmación humana es necesaria.
- Dependencia de herramientas externas: la ejecución de comandos ml requiere que la utilidad ml esté disponible en el entorno.
- Privacidad y alcance: hay que evitar subir información sensible sin revisión humana.
- Reglas de mapeo: la correspondencia entre lenguaje libre y dominios/tipos puede requerir ajustes de configuración o reglas específicas del equipo.

7. Extensibilidad y líneas futuras
----------------------------------
- Mejora de heurísticas y modelos para extracción y clasificación.
- Integración más estrecha con interfaces de edición (preview enriquecido, edición inline de propuestas).
- Automatizaciones por política (por ejemplo: propuestas de alta prioridad que se marquen para revisión especial).
- Métricas y trazabilidad del flujo (qué propuestas se aceptan vs. rechazan) para mejorar la calidad del mapeo.
- Skill para separación y registro de heurísticas: se desarrollará una skill que permita separar notas en propuestas; si se descubren nuevas heurísticas, se agregará una referencia a esa heurística en Mulch para documentarlas y facilitar su reutilización.

8. Conclusión
-------------
La extensión log_diario busca ser un puente entre la captura informal de conocimiento (notas, conversaciones) y la base de conocimiento estructurada Mulch. Mediante un flujo asistido por el agente, se propone convertir texto libre en registros Mulch atómicos y trazables, reduciendo fricción al registrar decisiones y fallos, sin reemplazar la revisión humana.


Fecha: 2026-03-28
Autor: Equipo de desarrollo — Extensión log_diario
