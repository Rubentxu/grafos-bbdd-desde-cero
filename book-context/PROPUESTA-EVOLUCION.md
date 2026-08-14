# Propuesta de evolución de la obra — Estudio estratégico

> Fecha: 2026-08-14 · Estado: **ACEPTADA (Propuesta A)** — ver ADR-005 (`adr/005-vol3-y-refuerzos.md`) y artefactos: `vol3-grafos-era-ia.md`, `OUTLINE-VOL3.yml`, `CURRICULUM-VOL3.yml`.
> Ámbito: Vol.I (publicado), Vol.II (en curso, código-first), posible Vol.III.
> Origen: petición del autor — «ver cómo hacer útil el libro, meter más capítulos útiles de internals de BBDD, el alcance de los grafos para knowledge bases en IA, y refuerzos en modelado de datos (entidades, propiedades, buenas prácticas, workflows de extracción, abstracciones)».

---

## 1. Estado actual (checkpoint 2026-08-14)

### Vol.I — «Grafos en Computación: de Cero a Experto»
- **DONE** — 2ª edición, 32 capítulos, 14.936 líneas.
- Cobertura: fundamentos → algoritmos clásicos (MST, caminos, flujo, coloración, planaridad, strings, espectral) → frontera (randomizados, NP-completitud, PD, ML/GNN) → 12 dominios de aplicación (BBDD, compiladores, SO, redes, distribuidos, seguridad, bio, NLP, robótica, testing, recomendación, quantum).
- Migrado íntegro al workspace (19 crates + `vol2-liradb`).

### Vol.II — «Construye LiraDB»
- **Manuscrito**: esqueleto (297 líneas — ToC + borradores de Prólogo y Apéndice 0). **0 capítulos de prosa redactados.**
- **Código validado (validation-first)**: 12 de 40 caps en `vol2-liradb` (7-18), 366 tests, ALL_GREEN.

| Parte | Caps | Código | Prosa |
|---|---|---|---|
| I — Pensar en grafos | 1-5 | pendiente | pendiente |
| II — De estructura a BBDD | 6-10 | 7-10 DONE | pendiente |
| III — Motor de almacenamiento | 11-16 | **DONE completa** | pendiente |
| IV — Consultar el grafo | 17-21 | 17-18 DONE | pendiente |
| V — Algoritmos persistentes | 22-26 | pendiente | pendiente |
| VI — Fiabilidad | 27-30 | pendiente | pendiente |
| VII — Producto técnico | 31-36 | pendiente | pendiente |
| VIII — Sistemas avanzados | 37-40 | pendiente | pendiente |

**Observación clave**: la estrategia código-first ha validado el 30% del Vol.II, pero la prosa es el recurso escaso. El motor (Parte III) está cerrado y verificable; sus 6 capítulos pueden redactarse ya.

---

## 2. Contexto de mercado y tecnológico (2025-2026) — por qué ahora

Hechos que cambian el valor comercial y editorial del proyecto:

1. **Kùzu fue adquirida por Apple (oct. 2025) y el proyecto open-source se archivó.** La comunidad se fragmentó en forks (p. ej. «Ladybug», «bighorn») y DuckDB añade capacidades de grafos. El hueco de «BBDD de grafos embebida» que Kùzu ocupaba **está vacío** — un libro que enseña a construir una es más relevante que nunca. Además, la historia de LiraDB (inspirada en Kùzu/Ladybug) pasa de «nota de atribución» a **relato de actualidad**: ADR-001 debe actualizarse.
2. **GraphRAG pasó de experimental a producción.** El patrón ganador 2025-26 es **híbrido vector + grafo**: recuperación semántica (embeddings) + recorrido estructurado (multi-hop). Microsoft Research reporta ~86% de exactitud de GraphRAG vs ~32% de RAG vectorial puro en sus benchmarks internos. Memgraph, FalkorDB, SurrealDB y Redis ya envían «HybridRAG» de primera clase.
3. **La extracción con LLM (texto → entidades/relaciones → grafo) es el driver nº 1 de adopción** de BBDD de grafos (Neo4j LLM Knowledge Graph Builder, LangChain/LlamaIndex KG constructors).
4. **GQL (ISO/IEC 39075)** — primer lenguaje de BD ISO nuevo desde SQL — respaldado conjuntamente por Neo4j y AWS (sept. 2025); Google Spanner Graph se declara conforme. El paisaje de lenguajes (Cypher → GQL, SPARQL, Gremlin) merece capítulo propio.
5. **Memoria de agentes**: los agentes de IA necesitan KGs temporales para multi-hop y frescura de datos (Zep/Graphiti como referencia de producción). Es la aplicación estrella del momento.
6. **Mercado**: ~2.900 M$ (2025) → ~3.600 M$ (2026), proyección 20.000-25.000 M$ hacia 2034 (CAGR ~24%). Neo4j >200 M$ de ingresos, ~44% de cuota.

**Conclusión**: los tres ejes que el autor quiere reforzar (internals, modelado, grafos×IA) coinciden exactamente con los tres vectores de demanda del mercado. No es un capricho: es donde estará el lector en 2026-27.

---

## 3. Diagnóstico de lagunas del guion actual

### 3.1 Internals de BBDD (presente pero incompleto)
Ya cubierto: slotted pages, pager, buffer pool, CSR, hash/B+ tree, compactación, WAL, recovery, MVCC limitado, columnar/vectorizado, WCOJ, distribución.

Faltan temas que un lector de internals espera:
- **Concurrencia clásica**: 2PL, niveles de aislamiento y sus anomalías (dirty/non-repeatable/phantom read, write skew), control optimista (OCC), y **detección de deadlocks con grafo de espera** (¡un grafo dentro de la BBDD — perfecta simbiosis con el Vol.I!).
- **B+ tree multinivel**: el cap. 15 fue deliberadamente single-level; splits, balanceo y cosecha de claves quedaron prometidos.
- **LSM-trees**: el otro gran paradigma de almacenamiento (write-optimized), comparativa B-tree vs LSM, compacción leveled/tiered — conecta con cap. 16.
- **Compresión** (diccionario, RLE, bit-packing, delta) — refuerza cap. 38.
- **Estadísticas y estimación de cardinalidad** — refuerza cap. 21 (optimizador).

### 3.2 Modelado de datos de grafos (ausente — el guion está centrado en el motor, no en el uso)
Nada del guion actual enseña **cómo diseñar un grafo bueno**:
- Node vs edge vs property: criterios de decisión, reificación, hiperaristas.
- Antipatrones: supernodos, direccionalidad, multigrafos, propiedades que deberían ser nodos.
- Temporalidad: valid-time, bitemporal, versionado de entidades.
- Esquema abierto vs estricto, constraints, unicidad, índices sobre propiedades (GQL DDL).
- **Workflows de extracción e ingesta**: CSV/JSONL → grafo, NER/LLM → triples, **entity resolution** (deduplicación como grafo de similitud — de nuevo, un grafo resolviendo un problema de grafos).

### 3.3 Grafos × IA / knowledge bases (ausente)
El Vol.I cap. 20 toca GNN/ML, pero nada cubre el uso actual de grafos **como base de conocimiento**:
- Property Graph vs RDF (quads, IRIs, OWL/SHACL), mappings.
- Embeddings de grafos en la práctica (node2vec/TransE → similitud estructural).
- **HNSW y ANN: el índice vectorial más usado ES un grafo** (navigable small-world). Capítulo puente perfecto: internals + IA en uno.
- GraphRAG híbrido (multi-hop, Personalized PageRank, resúmenes de comunidades — conecta con Louvain cap. 25).
- Extracción LLM → grafo (grounding, alucinaciones, human-in-the-loop).
- KGs como memoria de agentes (frescura, temporalidad).

### 3.4 Utilidad práctica (transversal)
- Sin **dataset hilo conductor**: cada capítulo vive en su propio ejemplo. Un caso real compartido (modelado → ingesta → consultas → algoritmos → GraphRAG) multiplicaría la retención.
- Sin prosa: el mayor riesgo del proyecto no es técnico, es editorial.
- La CLI llega tarde (cap. 31); un hito de CLI mínimo tras el cap. 20 haría la Parte V-VIII demostrable desde shell.

---

## 4. Propuesta A (RECOMENDADA): Vol.III + refuerzos quirúrgicos

### 4.1 Vol.III — «Grafos en la era de la IA: modelar, razonar y recuperar» (~13 caps, 3 partes)

Mantiene intactos Vol.I y Vol.II (cero renumeración, cero impacto en workspace/LEDGER), y crea un producto con gancho comercial propio y distinto (el lector de IA no necesita leer los 40 caps del motor).

**Parte I — Modelar datos de grafos (caps. 41-45)**
| # | Capítulo | Sinopsis |
|---|---|---|
| 41 | Modelado de entidades, propiedades y relaciones | Node/edge/property: criterios; abstracción vs consultabilidad |
| 42 | Antipatrones y trampas del diseño | Supernodos, reificación, dirección, multigrafos; refactorings de modelo |
| 43 | El tiempo en el grafo | Versionado, valid-time, bitemporal; conexión con WAL (Vol.II cap. 28) |
| 44 | Esquema, constraints e índices | LPG abierta vs estricta; unicidad; tipos; GQL DDL |
| 45 | Workflows de ingesta y entity resolution | CSV/JSONL/NER → grafo; deduplicación como grafo de similitud |

**Parte II — Knowledge bases semánticas (caps. 46-48)**
| # | Capítulo | Sinopsis |
|---|---|---|
| 46 | Property Graph vs RDF | Quads, IRIs, tipado literal; mappings bidireccionales |
| 47 | Ontologías y razonamiento ligero | OWL subsets, SHACL, subClassOf; cuándo merece la pena |
| 48 | El paisaje de lenguajes | Cypher, **GQL (ISO 39075)**, SPARQL, Gremlin; por qué LiraQL se parece a Cypher |

**Parte III — Grafos × IA (caps. 49-53)**
| # | Capítulo | Sinopsis |
|---|---|---|
| 49 | Embeddings de grafos en la práctica | node2vec/TransE (puente con Vol.I cap. 20); similitud estructural |
| 50 | Índices vectoriales: HNSW | ANN; navegabilidad small-world — **el índice también es un grafo** |
| 51 | GraphRAG: recuperación híbrida | vector + grafo, multi-hop, PPR, resúmenes de comunidades (puente Louvain cap. 25) |
| 52 | Extracción con LLM: de texto a grafo | NER+RE, grounding, alucinaciones, human-in-the-loop |
| 53 | Grafos como memoria de agentes | KG temporal, frescura, estilo Zep/Graphiti |

**Impacto en el workspace (validation-first se mantiene)**: `vol2-liradb` (o nueva crate `vol3-liradb-ai`) ganaría: tipo vector en `Value`, índice HNSW (reutiliza patrones del cap. 15), un subcomando `liradb extract` con pipeline LLM stubeable (proveedor intercambiable, sin acoplar a ninguna API), y consultas de similitud en LiraQL.

### 4.2 Refuerzos quirúrgicos en Vol.II (sin renumerar)

| Cap. existente | Refuerzo |
|---|---|
| 15 (índices) | Apéndice del capítulo: **B+ tree multinivel** (splits, balanceo) — cierra la promesa «single-level pedagógico» |
| 16 (compactación) | Sección comparativa **LSM vs slotted pages** (leveled/tiered compaction) |
| 21 (optimizador) | Sección **estadísticas y cardinalidad** (histogramas) |
| 30 (MVCC limitado) | Ampliar a **concurrencia completa**: 2PL, niveles de aislamiento y anomalías, OCC, **detección de deadlocks con grafo de espera** (grafo de waits-for + ciclo = Vol.I cap. 3) |
| 38 (columnar) | Sección de **compresión** (diccionario, RLE, bit-packing, delta) |
| 40 (distribución) | Nota sobre sistemas híbridos vector+grafo distribuidos |
| Apéndice E | Actualizar a paisaje 2026: **Kùzu archivada** (compra de Apple, forks Ladybug/bighorn), GQL ISO, Neo4j+vector, FalkorDB/Memgraph, DuckDB-graph |
| ADR-001 | Reescribir: de nota de atribución a relato histórico (Kùzu 2023-2025 → Apple → forks; LiraDB como lectura clean-room) |

---

## 5. Propuesta B (alternativa): expandir Vol.II a ~48 caps

Añadir Partes IX (modelado) y X (grafos×IA) dentro del Vol.II.
- **Pros**: obra única; el lector que termina el motor sigue directo al uso.
- **Contras**: 48 caps alarga un volumen que ya es grande; retrasa su cierre; mezcla dos audiencias (el constructor del motor vs el usuario/modelador); el «gancho IA» queda enterrado en la Parte X.
- Veredicto: **solo si se prioriza tener una única obra** por encima de todo lo demás.

---

## 6. Cómo hacer útil el libro (medidas transversales)

1. **Dataset hilo conductor** — elegir un caso realista (propuesta: grafo de papers+citas+autores tipo arXiv, o knowledge base corporativa personas/proyectos/documentos) y usarlo de cap. 41 a 53; versionado en `datasets/` del workspace con generador determinista. Para el Vol.II, un mini-dataset común desde el cap. 32 (import/export).
2. **CLI mínima anticipada** — tras el cap. 20, hito `liradb load + query "MATCH ..."` ejecutable: las Partes V-VIII se prueban desde shell y el lector tiene «su producto» a mitad de libro.
3. **Prosa en paralelo** — la Parte III está cerrada en código: redactar sus 6 capítulos ya (la memoria del diseño está fresca en MIGRATION-PATTERN.md §§16-20). Alternar a partir de ahora: 1 cap. de código nuevo + 1 cap. de prosa del bloque cerrado anterior.
4. **«Cómo lo hace una BBDD real» actualizado a 2026** en cada cierre de capítulo (batería ya prevista) — con el paisaje post-Kùzu.
5. **Ejercicios con solución verificada** — las soluciones son tests del workspace (el lector puede ejecutar `cargo test` y verlas pasar).
6. **Glosario bilingüe ES/EN** ampliado en Apéndice B (términos GQL/RDF/HNSW/GraphRAG).
7. **Posicionamiento editorial**: «la BBDD de grafos embebida que aprendes construyendo» — en el hueco que Kùzu dejó. El workspace público es el argumento de venta: 366 tests verificados.

---

## 7. Próximos pasos sugeridos (si se aprueba la Propuesta A)

1. **Decidir**: Propuesta A (Vol.III) vs B (expandir Vol.II). — solo el autor
2. Actualizar ToC del Vol.II con los refuerzos quirúrgicos + ADR-001 reescrito + Apéndice E.
3. Continuar código Vol.II: cap. 19 (plan lógico) → 20 (Volcano) → 21 (optimizador + estadísticas).
4. Hito CLI mínima (tras cap. 20).
5. Redactar prosa Parte III (caps. 11-16) — bloque cerrado.
6. Crear esqueleto del Vol.III (ToC + brief por capítulo + entradas en LEDGER/CORPUS).
7. Elegir dataset hilo conductor y construir `datasets/` + generador.

---

## 8. Fuentes del contexto de mercado (2025-26)

- [Future AGI — Vector DB vs Knowledge Graph in 2026](https://futureagi.com/blog/vector-databases-knowledge-graphs-rag-2025/)
- [arXiv — Hybrid GraphRAG benchmark](https://arxiv.org/html/2507.03608v1)
- [K-AI — Knowledge Graph vs Vector RAG](https://k-ai.ai/en/news/knowledge-graph-vs-vector-rag-neural-semantic-graph/)
- [Atlan — Vector DB vs Knowledge Graph for agent memory](https://atlan.com/know/vector-database-vs-knowledge-graph-agent-memory/)
- [Redis — Knowledge graph RAG for AI agents](https://redis.io/blog/knowledge-graph-rag-structured-retrieval-ai-agents/)
- [Fortune Business Insights — Graph DB market](https://www.fortunebusinessinsights.com/graph-database-market-105916)
- [BigDATAwire — Neo4j surpasses $200M revenue](https://www.hpcwire.com/bigdatawire/this-just-in/neo4j-surpasses-200m-in-revenue/)
- [ArcadeDB — From KuzuDB migration guide (Kùzu archivada)](https://arcadedb.com/blog/from-kuzudb-to-arcadedb-migration-guide/)
- [gdotv — Kùzu forks, DuckDB graph (oct. 2025)](https://gdotv.com/blog/weekly-edge-kuzu-forks-duckdb-graph-cypher-24-october-2025/)
- [ArcadeDB — Neo4j open-source alternatives in 2026](https://arcadedb.com/blog/neo4j-alternatives-in-2026-a-fair-look-at-the-open-source-options/)
- [Blocks & Files — Neo4j+AWS back GQL standard](https://www.blocksandfiles.com/ai-ml/2025/09/22/neo4j-backs-new-graph-query-standard-for-ai-era/)
- [NebulaGraph — GQL vs Cypher (ISO 39075)](https://nebula-graph.io/posts/gql-vs.-cypher-what-the-new-iso-standard-brings-to-the-table)
- [Neo4j — LLM Knowledge Graph Builder](https://neo4j.com/blog/developer/llm-knowledge-graph-builder-release/)
