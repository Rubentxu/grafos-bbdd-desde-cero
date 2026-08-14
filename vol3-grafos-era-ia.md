---
title: "Grafos en la era de la IA: modelar, razonar y recuperar — Knowledge bases, GraphRAG y memoria de agentes"
subtitle: "Volumen III de Grafos en Computación: de Cero a Experto"
author: "Rubentxu"
date: "2026-08-14"
lang: es
volumen: III
obra: "Grafos en Computación: de Cero a Experto"
proyecto_integrador: LiraDB
edicion: "Edición unificada 2026 — primer borrador"
licencia: "CC BY-NC-SA 4.0"
---

# Grafos en la era de la IA

**Knowledge bases, GraphRAG y memoria de agentes**

*Volumen III de la obra "Grafos en Computación: de Cero a Experto" — con LiraDB como banco de pruebas.*

---

> «Los vectores recuerdan lo que se parece; el grafo recuerda por qué se relaciona.»
> — Manifiesto KB-Lira

---

**Edición**: Primer borrador, agosto de 2026
**Volumen**: III (de III)
**Idioma**: Español (con terminología técnica estándar en inglés)
**Stack**: Rust **2024** edition + LiraDB + crates seleccionadas (ver Apéndice 0 del Vol.II)
**Proyecto integrador**: **KB-Lira** — base de conocimiento sobre LiraDB, hilo conductor del volumen
**Licencia**: CC BY-NC-SA 4.0

---

# Prólogo — El grafo como capa de conocimiento

> *Borrador. Se completará cuando se hayan redactado los caps. 41-45.*

En 2024 los grafos dejaron de ser una curiosidad de nicho y se convirtieron en **infraestructura de la IA**. Los agentes necesitan memoria que no caduque; los sistemas RAG descubrieron que la similitud vectorial sola no responde preguntas multi-hop; y la extracción con LLM convirtió cualquier montón de documentos en un grafo — si sabes modelarlo.

Este volumen es la respuesta del proyecto a esa realidad. No es un libro de "prompt engineering" ni de machine learning: es un libro de **modelado y uso de grafos como base de conocimiento**, con LiraDB — el motor que construiste (o estudiaste) en el Volumen II — como banco de pruebas.

## Qué asume este volumen

Asumimos que conoces el **modelo Property Graph** (Vol.II cap. 7), sabes lo que es un `MATCH` (Vol.II caps. 17-18) y entiendes PageRank y comunidades a nivel conceptual (Vol.II caps. 24-25). El cap. 20 del Vol.I (GNN) ayuda pero no es obligatorio: aquí los embeddings se usan, se inspeccionan y se construyen a mano, no se entrenan redes profundas.

No asumimos conocimientos previos de:

- modelado de datos de grafos (antipatrones, temporalidad, esquemas);
- RDF, OWL o SHACL;
- sistemas de recuperación (RAG, ANN, índices vectoriales);
- pipelines de extracción con LLM.

Todo se construye desde cero, con la misma regla de la obra: **primero a mano, luego con herramienta, siempre entendiendo el trade-off**.

## Cómo leer este volumen

**Ruta lineal**: del cap. 41 al 53 en orden. Tiempo estimado: 60-80 horas. Primer borrador ~300 páginas.

**Ruta "arquitecto de datos"**: Partes I y II (caps. 41-48). Para quien modela knowledge bases y no va a tocar embeddings.

**Ruta "IA"**: si vienes del mundo RAG/LLM y ya sabes modelar, empieza en el cap. 49. Los caps. 41-45 se pueden consultar como referencia de modelado cuando el grafo te empiece a doler.

## El hilo conductor: KB-Lira

Cada capítulo añade una capa a **KB-Lira**, una base de conocimiento realista de un equipo de investigación: papers, notas, informes, personas, organizaciones, proyectos y temas, con relaciones de autoría, citación, mención y afiliación. Se modela en el cap. 41, se temportaliza en el 43, se valida en el 47, se vectoriza en el 49, se consulta con GraphRAG en el 51, se enriquece con un LLM en el 52 y acaba siendo la memoria de un agente en el 53. El generador determinista del dataset vive en el workspace.

## ¿Qué te llevarás?

Después de leer este libro:

- Diseñarás knowledge bases que **no se pudren** al crecer (antipatrones, temporalidad, esquema).
- Sabrás traducir entre Property Graph y RDF sin perder el alma de ninguno.
- Implementarás un índice **HNSW** desde cero — y entenderás que es, literalmente, un grafo.
- Construirás un pipeline **GraphRAG** híbrido y sabrás evaluarlo.
- Extraerás tripletas con un LLM sin envenenar tu grafo (grounding, dedup, human-in-the-loop).
- Montarás la **memoria de largo plazo** de un agente sobre un KG temporal.

Empezamos. Bienvenido a la capa de conocimiento.

---

*(El Prólogo se completará cuando se hayan redactado los caps. 41-45. Mientras tanto, este párrafo actúa de placeholder.)*

---

# Tabla de contenidos

> *Borrador — outline detallado en `book-context/OUTLINE-VOL3.yml` (secciones, conceptos y dependencias por capítulo).*

**Prólogo — El grafo como capa de conocimiento**

**Parte I — Modelar datos de grafos**
41. Modelar entidades, propiedades y relaciones
42. Antipatrones: supernodos, reificación y otras trampas
43. El tiempo en el grafo: versionado y bitemporalidad
44. Esquema, constraints e índices en property graphs
45. Workflows de ingesta: de datos crudos al grafo (CSV, JSONL, entity resolution)

**Parte II — Knowledge bases semánticas**
46. Property Graph vs RDF: dos filosofías, un mismo objetivo
47. Ontologías y validación: OWL ligero y SHACL
48. El paisaje de lenguajes: Cypher, GQL (ISO 39075), SPARQL y Gremlin

**Parte III — Grafos × IA**
49. Embeddings de grafos: de estructura a vectores
50. Índices vectoriales: HNSW, o cuando el índice también es un grafo
51. GraphRAG: recuperación híbrida vector + grafo
52. Extracción con LLM: de texto suelto a grafo fiable
53. Grafos como memoria de agentes

**Epílogo — El grafo como capa de conocimiento**

**Apéndice A — El dataset KB-Lira: guía completa y generador**
**Apéndice B — Glosario IA + grafos (bilingüe ES/EN)**
**Apéndice C — Bibliografía y papers (GraphRAG, HNSW, RDF/OWL/SHACL, GQL)**
**Apéndice D — Herramientas del ecosistema 2026 (paisaje post-Kùzu)**
**Apéndice E — ADRs del Vol.III (modelo KB-Lira, extracción, evaluación)**

---

*(El cuerpo de los 13 capítulos se redactará cuando el outline sea aprobado y pase por chapter-planner. Este archivo es un esqueleto navegable.)*

---

# Epílogo — El grafo como capa de conocimiento

> *Placeholder — se redactará tras el cap. 53.*

---

# Colofón

> *Placeholder.*

---
