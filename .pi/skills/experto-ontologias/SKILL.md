---
name: experto-ontologias
description: Skill que actúa como un experto en ontologías (modelado conceptual, OWL/RDF, SKOS, alineamiento, consultas SPARQL y evaluación). Provee heurísticas, plantillas, buenas prácticas y comandos recomendados para trabajar con ontologías en proyectos de datos y conocimiento.
disable-model-invocation: false

description_agent_friendly: |
  Skill de experto en ontologías. Diseñada para asistir en todo el ciclo de vida de una ontología:
  - levantamiento y modelado conceptual (TBox/ABox)
  - serialización y normas (OWL2, RDF, Turtle, SKOS)
  - mapeo y alineamiento entre esquemas/ontologías
  - validación, pruebas y métricas de calidad (consistencia, cobertura, redundancia)
  - consultas y extracción (SPARQL)
  - integración con herramientas (Protégé, ROBOT, rdflib, Owlready2)

  Invoca esta skill cuando necesites: diseño de ontologías, revisión técnica, generación de axiomas, resolución de ambigüedades en modelos conceptuales, o transformar datos a grafos RDF.
---

# Skill: Experto en Ontologías

Propósito
--------
Esta skill documenta heurísticas, patrones y comandos recomendados para trabajar con ontologías en proyectos de información y conocimiento. Está pensada para ser usada por agentes y personas que necesitan:
- modelar dominios complejos con OWL/RDF
- garantizar interoperabilidad entre vocabularios
- extraer y razonar sobre datos enlazados (Linked Data)

Cuándo invocar
---------------
- Al definir clases, propiedades y axiomas de dominio (TBox).
- Al mapear esquemas o fusionar vocabularios.
- Al evaluar la consistencia y competitividad de una ontología.
- Al construir pipelines de ingestión RDF o consultas SPARQL.

Capacidades y responsabilidades
--------------------------------
- Sugerir diseño conceptual (clases, propiedades, relaciones, patrones de herencia).
- Recomendar serializaciones adecuadas (TTL, RDF/XML, JSON-LD) y namespaces.
- Proponer axiomas OWL (restricciones, cardinalidades, equivalencias) cuando estén justificados.
- Detectar y proponer resoluciones para ambigüedades de modelado (clases vs. instancias, reificación, n-aries).
- Generar y optimizar consultas SPARQL para tareas comunes (búsqueda, agregación, construct).
- Sugerir pipelines de validación con ROBOT, SHACL o razonadores (Hermit, Pellet).
- Proveer plantillas para documentación y ejemplos de uso.

Heurísticas y buenas prácticas
-----------------------------
1. Separar TBox y ABox conceptualmente: modela primero las clases y relaciones (TBox), luego los datos (ABox).
2. Preferir ontologías modulares y reusar vocabularios existentes (schema.org, FOAF, Dublin Core, SKOS) antes de crear términos nuevos.
3. Nombres y URIs: usar URIs estables, legibles y con buenas prácticas de versionado.
4. Evitar sobrecargar el modelo con axiomas innecesarios; priorizar axiomas que producen inferencias útiles para las aplicaciones previstas.
5. Documentar decisiones de modelado: para cada clase/property anotar intención, ejemplos, y alternativas consideradas.
6. Validación continua: integrar checks automáticos (ROBOT, SHACL) en CI para evitar regresiones semánticas.

Herramientas recomendadas
-------------------------
- Protégé: edición y visualización de ontologías OWL.
- ROBOT: transformación, pruebas y automatización (CLI).
- RDFLib / Owlready2: manipulación programática en Python.
- Apache Jena / Fuseki: triple store y endpoints SPARQL.
- Razonadores OWL: HermiT, Pellet para consistencia y clasificación.

Operaciones comunes (plantillas de prompts / acciones)
------------------------------------------------------
- Generar esqueleto de ontología a partir de un glosario o CSV de entidades.
  "Genera un esqueleto OWL/Turtle con las clases y propiedades extraídas de este CSV: ...".

- Alinear dos ontologías (mapas equivalencia/broader/narrower):
  "Propón un mapeo entre ontología A y ontología B. Identifica correspondencias exactas y aproximadas y su confianza. Devuelve resultados en formato Simple Alignment o CSV."

- Producir axiomas OWL para una restricción de dominio:
  "Para la clase Producto, añade axiomas que indiquen que tiene exactamente una marca (hasBrand) y al menos una categoría (hasCategory). Escribe en Turtle."

- Escribir consultas SPARQL:
  "Dame una consulta SPARQL que liste productos con su categoría y conteo de ventas, ordenado por ventas desc."

- Validación con SHACL/ROBOT:
  "Genera shapes SHACL que verifiquen cardinalidades y tipos para las propiedades principales. Devuelve ejemplos de comandos ROBOT para validar un archivo TTL."

Integración con Mulch / registros de decisiones
------------------------------------------------
- Registrar decisiones de modelado de forma atómica en Mulch: cada decisión importante debe ser un registro ("ml record ontologia --type decision --title ... --description ... --files 'ontologias/**' ").
- Al crear una nueva clase o cambiar un axioma crítico, agregar un registro que explique el porqué, alternativas y archivos impactados.

Heurísticas para atomizar notas (alineado con skill log-diario-separation)
---------------------------------------------------------------------------
- Cada registro debe responder: ¿qué se decidió? ¿por qué? ¿qué opciones se descartaron? ¿qué archivos cambian?
- Preferir títulos cortos y descripciones que permitan reproducir la decisión.

Extras: fragmentos prácticos
---------------------------
- Ejemplo Turtle mínimo de clase y propiedad:

  @prefix ex: <http://example.org/ns#> .
  @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
  @prefix owl: <http://www.w3.org/2002/07/owl#> .

  ex:Producto a owl:Class ;
    rdfs:label "Producto"@es ;
    rdfs:comment "Entidad que representa un producto vendible"@es .

  ex:hasCategory a owl:ObjectProperty ;
    rdfs:domain ex:Producto ;
    rdfs:range ex:Categoria ;
    rdfs:label "tiene categoría"@es .

- Comando ROBOT para validar un TTL (ejemplo):
  robot validate --input ontologia.ttl

- Ejemplo de SPARQL SELECT simple:

  SELECT ?producto ?categoria WHERE {
    ?producto a ex:Producto ;
             ex:hasCategory ?categoria .
  }

Referencias y lecturas recomendadas
----------------------------------
- W3C OWL 2 Web Ontology Language Primer
- W3C RDF 1.1 Concepts and Abstract Syntax
- SHACL primer y ejemplos
- ROBOT documentation (https://robot.obolibrary.org/)
- Protégé (https://protege.stanford.edu/)

Contactos / cuándo escalar
--------------------------
- Esta skill funciona como primer nivel experto. Para decisiones de arquitectura semántica corporativa (vocabularios compartidos entre unidades de negocio) recomienda escalado a arquitectos de datos/semánticos y registrar la decisión en Mulch.

# Fin
