# Grafos en Computación: de Cero a Experto — Obra unificada

Obra técnica en **3 volúmenes**, escrita en español, sobre teoría de grafos, algoritmos en Rust, construcción desde cero de una base de datos de grafos (proyecto **LiraDB**) y grafos como capa de conocimiento en la era de la IA (**KB-Lira**).

Este repositorio es un **monorepo trazable**: manuscritos + código verificado conviven, y todo commit del motor lleva detrás una cadena `fmt → check → test → clippy` en verde (**455 tests, ALL_GREEN**).

## Volúmenes

| Volumen | Título | Foco | Estado | Archivo |
|---|---|---|---|---|
| **I** | Grafos en Computación: de Cero a Experto | Algoritmos, estructuras, aplicaciones (Rust) | Publicado (2ª ed., julio 2026) | [`vol1-grafos-de-cero-a-experto-rust.md`](./vol1-grafos-de-cero-a-experto-rust.md) |
| **II** | Construye LiraDB | Motor de BBDD de grafos desde cero (proyecto LiraDB) | En construcción — código 14/40 caps verificado, prosa en arranque | [`vol2-construye-liradb.md`](./vol2-construye-liradb.md) |
| **III** | Grafos en la era de la IA: modelar, razonar y recuperar | Knowledge bases, GraphRAG y memoria de agentes (KB-Lira) | Esqueleto — outline aprobado (ADR-005) | [`vol3-grafos-era-ia.md`](./vol3-grafos-era-ia.md) |

## Estructura del repositorio

```
.
├── vol1-grafos-de-cero-a-experto-rust.md   # Vol.I — 32 caps + 4 apéndices (2ª ed.)
├── vol2-construye-liradb.md                # Vol.II — 40 caps/8 Partes (guion + prólogo)
├── vol3-grafos-era-ia.md                   # Vol.III — 13 caps/3 Partes (esqueleto)
├── book-context/                           # Memoria de la obra (fuente de verdad editorial)
│   ├── LEDGER.md                           #   Estado del workflow por capítulo/volumen
│   ├── SESSION-LOG.md                      #   Bitácora de sesiones
│   ├── CONVENTIONS.md                      #   Convenciones editoriales + pipeline pedagógico
│   ├── CORPUS.yml                          #   Temas + preguntas críticas por capítulo
│   ├── OUTLINE-VOL3.yml / CURRICULUM-VOL3.yml  # Outline y grafo curricular del Vol.III
│   ├── PROPUESTA-EVOLUCION.md              #   Estudio estratégico que originó el Vol.III
│   └── adr/                                #   Decisiones arquitectónicas (ADR-005: Vol.III)
└── liradb-workspace/                       # TODO el código ejecutable de la obra (Rust 2024)
    ├── Cargo.toml                          #   Workspace: 21 crates (19 Vol.I + vol2-liradb + liradb-cli)
    ├── crates/vol2-liradb/                 #   Motor LiraDB: caps 7-20 (storage, LiraQL, plan, Volcano)
    ├── crates/vol2-liradb-cli/             #   CLI mínima (hito ADR-005): `liradb demo|query`
    ├── scripts/verify.sh                   #   Puerta de calidad: fmt → check → test → clippy
    └── book-context/                       #   code-map prosa↔código + MIGRATION-PATTERN (§§1-25)
```

## Cómo está organizada la obra

- **Vol.I (32 caps, 6 Partes)**: teoría de grafos, algoritmos clásicos (BFS, DFS, Dijkstra, MST, flujo, coloración, planaridad, strings, espectral, randomización, NP, DP, GNN) y 12 capítulos sobre cómo los grafos viven en la informática moderna (BBDD, compiladores, SO, redes, distribuidos, seguridad, bio, NLP, robótica, verificación, recomendadores, quantum).

- **Vol.II (40 caps, 8 Partes)**: construye desde cero una BBDD de grafos embedded en Rust —**LiraDB**—: modelo Property Graph, persistencia, slotted pages, Pager, buffer pool (Clock/LRU), CSR persistente, índices hash/B+tree, compactación, lenguaje de consulta LiraQL (lexer, parser, plan lógico, ejecutor Volcano, optimizador), algoritmos, ACID/WAL/MVCC, CLI, tests, benchmarks y comparación con BBDD de producción.

- **Vol.III (13 caps, 3 Partes)**: modelado de datos de grafos (antipatrones, temporalidad, esquemas, ingesta/entity resolution), knowledge bases semánticas (RDF/OWL/SHACL, Cypher/GQL/SPARQL/Gremlin) y grafos × IA (embeddings, HNSW, GraphRAG, extracción con LLM, memoria de agentes). Hilo conductor: **KB-Lira**.

## Validación del código (validation-first)

El código del libro **se verifica antes de publicarse**. Desde la raíz del workspace:

```bash
cd liradb-workspace
./scripts/verify.sh        # fmt → check → test → clippy → ALL_GREEN (455 tests)
cargo run -p liradb-cli -- demo   # el motor, ejecutable desde shell
```

Regla de disciplina: **sólo se commitea con ALL_GREEN**. El historial de migraciones y bugs corregidos por capítulo vive en `liradb-workspace/book-context/MIGRATION-PATTERN.md`.

## Workflow editorial

El proyecto sigue el workflow `BOOK-WORKFLOW.md` (macro-fases: A fundamentos, R investigación, B construcción, C validación, D publicación) con skills de libro y metodología teaching (ver `book-context/CONVENTIONS.md` §2). El estado por capítulo vive en `book-context/LEDGER.md`.

## Licencia

CC BY-NC-SA 4.0 (ver [LICENSE](./LICENSE)). Atribuciones completas en el Colofón de cada volumen (especialmente a Kùzu/Ladybug por los papers seminales sobre GDBMS modernos).

## Autor

Rubentxu — 2026.
