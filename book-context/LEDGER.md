# LEDGER — Estado de la obra unificada

> Estado del workflow BOOK-WORKFLOW.md para el proyecto `grafos-bbdd-desde-cero`.
> Mantener ligero: estado + `blocked_on` + `cycle` por capítulo.

```yaml
book:
  title: "Grafos en Computación: de Cero a Experto + Construye tu Propia BBDD de Grafos"
  author: "Rubentxu"
  edition: "Edición unificada 2026"
  language: "es"
  license: "CC BY-NC-SA 4.0"
  stack: "Rust 2024 + petgraph + crates seleccionadas (LiraDB)"

volumes:
  - id: vol-I
    title: "Grafos en Computación: de Cero a Experto — Algoritmos, estructuras y aplicaciones (Rust)"
    file: vol1-grafos-de-cero-a-experto-rust.md
    state: DONE                              # 2ª edición ya publicada
    chapters_total: 32
    chapters_done: 32
    appendices: [A, B, C, D]
    style: "Grokking 2.0"
    cross_references_to_vol_II: pending      # Fase 1
  - id: vol-II
    title: "Construye LiraDB — De los algoritmos fundamentales a un motor persistente de consultas en Rust"
    file: vol2-construye-liradb.md
    state: SKELETON                          # sólo front-matter + Prólogo + Apéndice 0 + Epílogo
    chapters_total: 40
    chapters_done: 0
    appendices: ["0", A, B, C, D, E]         # Apéndice 0 = Manual de estilo unificado
    style: "Híbrido (10 pasos LiraDB + baterías narrativas Vol.I)"
    current_macro_phase: A-bis              # reorganización física en curso
    notes: "refuerzos quirúrgicos aplicados al guion 2026-08-14 (ADR-005): caps 15/16/21/30/38/40 + Apéndice E; numeración congelada"
  - id: vol-III
    title: "Grafos en la era de la IA: modelar, razonar y recuperar"
    file: vol3-grafos-era-ia.md
    state: SKELETON                          # outline aprobado (ADR-005); 0 caps redactados
    chapters_total: 13
    chapters_done: 0
    appendices: [A, B, C, D, E]
    style: "Híbrido (plantilla Apéndice 0) + metodología teaching (CONVENTIONS §2)"
    current_macro_phase: A                   # currículo + outline hechos → chapter-planner
    progressive_example: "KB-Lira (dataset hilo conductor, generador en workspace pendiente)"

current_macro_phase: A-bis                  # re-alineamiento + reorganización
next_macro_phase: R                         # deep research para Vol.II/III
last_updated: 2026-08-14
```

## Capítulos Vol.II — máquina de estados

```yaml
vol-II-chapters:
  # PLANNED → DRAFTING → IN_REVIEW → PASS? → DONE
  #                                                    ↘ BLOCKED → DRAFTING (con hallazgos)
  - id: vol-II-prólogo
    state: DRAFTING
    blocked_on: null
    cycle: 0
  - id: vol-II-cap-01  # Qué es realmente un grafo (repaso + puente con Vol.I)
    state: PLANNED
    blocked_on: null
    cycle: 0
  # caps 02-06 planificados, saltamos al 07 (migración al workspace)
  - id: vol-II-cap-07  # Modelo de datos (Property Graph + Value)
    state: DONE        # migrado en sesión bootstrap (2026-07-30)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-08  # trait GraphStore + MemoryStore
    state: DONE        # migrado junto con cap 7 en bootstrap
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-09  # encoding binario
    state: DONE        # migrado junto con cap 7 en bootstrap
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-10  # append-only log + CRC32
    state: DONE        # migrado junto con cap 7 en bootstrap
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-11  # páginas, slotted pages, metapágina
    state: DONE        # migrado en sesión 2026-07-31
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-12  # trait Pager + FilePager
    state: DONE        # migrado en sesión 2026-07-31
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-13  # buffer pool (Clock, LRU, métricas)
    state: DONE        # migrado en sesión 2026-07-31
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-14  # CSR / adyacencias
    state: DONE        # migrado en sesión 2026-07-31
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-15  # Índices para encontrar datos (hash + B+ tree)
    state: DONE        # migrado en sesión 2026-07-31
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-16  # Compactación y mantenimiento (inspect|check|compact)
    state: DONE        # migrado en sesión 2026-08-03 — cierra la Parte III
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-17  # Diseñar un lenguaje pequeño (MATCH-WHERE-RETURN mini)
    state: DONE        # migrado en sesión 2026-08-03 — abre la Parte IV
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-18  # Construir el lexer y el parser
    state: DONE        # migrado en sesión 2026-08-03 — lexer + parser descendente manual
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-19  # Del AST al plan lógico
    state: DONE        # migrado en sesión 2026-08-14 — binder + operadores lógicos + tipos
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-20  # El motor de ejecución (modelo Volcano)
    state: DONE        # migrado en sesión 2026-08-14 — hito: ejecutar consultas de extremo a extremo
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
  - id: vol-II-cap-21  # Un optimizador pequeño pero real (liradb explain; estadísticas)
    state: DONE        # migrado en sesión 2026-08-15 — CIERRA la Parte IV (5/5)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap21_optimizador
  - id: vol-II-cap-22  # Caminos mínimos ponderados (Dijkstra, Bellman-Ford)
    state: DONE        # migrado en sesión 2026-08-15 — ABRE la Parte V (Sesión 18)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap22_caminos_minimos
  - id: vol-II-cap-23  # A*, heurísticas y búsquedas dirigidas
    state: DONE        # migrado en sesión 2026-08-15 — Parte V (Sesión 19)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap23_a_estrella
  - id: vol-II-cap-24  # Centralidad y PageRank
    state: DONE        # migrado en sesión 2026-08-15 — Parte V (Sesión 20)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap24_centralidad
  - id: vol-II-cap-25  # Comunidades y agrupaciones (Louvain simplificado)
    state: DONE        # migrado en sesión 2026-08-15 — Parte V (Sesión 21)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap25_comunidades
  - id: vol-II-cap-26  # Ejecutar algoritmos sin agotar la memoria (proyección, streaming, frontiers)
    state: DONE        # migrado en sesión 2026-08-16 — CIERRA la Parte V (5/5)
    blocked_on: null
    cycle: 0
    workspace_crate: vol2-liradb
    module: cap26_proyeccion
  # ... caps 27-40 por planificar
  - id: vol-II-apendice-0   # Manual de estilo unificado
    state: PLANNED
    blocked_on: null
    cycle: 0
  - id: vol-II-epilogo
    state: PLANNED
    blocked_on: null
    cycle: 0
```

## Decisiones arquitectónicas (ADRs, mini-resumen)

| ID | Decisión | Estado |
|---|---|---|
| ADR-001 | Atribución a Ladybug/Kùzu (clean-room conceptual, MIT/CC-BY 4.0) | pendiente — reescribir como relato histórico (Kùzu→Apple oct. 2025→archivado→forks) antes del cap. 37 |
| ADR-002 | `rust-toolchain.toml` + `Cargo.lock` pinneado por capítulo | pendiente |
| ADR-003 | Numeración Vol.II reinicia en 1 (autoportante) | aprobada 2026-07-30 |
| ADR-004 | Plantilla híbrida fija para TODO capítulo del Vol.II | aprobada 2026-07-30 |
| ADR-005 | Vol.III «Grafos en la era de la IA» + refuerzos quirúrgicos Vol.II + pipeline pedagógico (skills + teaching) | aprobada 2026-08-14 — ver `adr/005-vol3-y-refuerzos.md` |
| ADR-006 | Monorepo único público grafos-bbdd-desde-cero (supersede «repo separado» del plan original) | aprobada 2026-08-14 — ver `adr/006-monorepo-publico.md`; push a github.com/Rubentxu/grafos-bbdd-desde-cero |
| ADR-007 | Estructura por volúmenes/capítulos: manuscrito/volN/ fichero-por-capítulo + ensamblado build_book.sh + vol2-liradb módulo-por-capítulo | aprobada 2026-08-14 — ver `adr/007-estructura-por-capitulos.md`; reensamblado byte-idéntico verificado |

## Sesiones

- 2026-07-30 — Sesión 1: exploración, plan de fusión aprobado, Fase 0 ejecutada.
- 2026-07-31 — Sesión 2: caps Vol.II 11 (páginas) y 12 (Pager/FilePager) migrados a `vol2-liradb`.
- 2026-07-31 — Sesión 3: cap Vol.II 13 (buffer pool con política Clock/LRU, métricas) migrado a `vol2-liradb`. 24 tests propios, ALL_GREEN.
- 2026-07-31 — Sesión 4: cap Vol.II 14 (CSR persistente sobre BufferPool, forward+backward, errores tipados, invariantes, disk roundtrip) migrado a `vol2-liradb`. 28 tests propios, ALL_GREEN. Caps 7, 11, 12, 13, 14 migrados en total (5 caps).
- 2026-07-31 — Sesión 5: cap Vol.II 15 (HashIndex + BPlusTree sobre BufferPool, FNV-1a, overflow chain, range_scan, errores tipados) migrado a `vol2-liradb`. 27 tests propios, ALL_GREEN.
- 2026-08-03 — Sesión 6: cap Vol.II 16 (Compactación y mantenimiento: inspect|check|compact) migrado a `vol2-liradb`. StorageStats, CheckReport con IntegrityIssue/IssueKind, repack_page in-place, compact masivo, MaintenanceError tipado. 25 tests propios, ALL_GREEN. Cierra la Parte III (motor de almacenamiento): caps 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 migrados (10 caps).
- 2026-08-03 — Sesión 7: cap Vol.II 17 (Diseñar un lenguaje pequeño — LiraQL) migrado a `vol2-liradb`. Span, TokenKind/Token, Expression (Literal reutiliza Value del cap 7), CompareOp, patrones (NodePattern/RelationshipPattern/PathPattern), cláusulas (MatchClause/WhereClause/ReturnClause), AstNode (hito del brief), Query con validate() semántico, QueryError tipado con posición, Display canónico para todo el AST (round-trip), hex_bytes propio. 41 tests propios, ALL_GREEN. Abre la Parte IV (consultar el grafo): caps 7-17 migrados (11 caps). Lexer + parser descendente manual serán el cap 18.
- 2026-08-03 — Sesión 8: cap Vol.II 18 (Construir el lexer y el parser) migrado a `vol2-liradb`. Lexer manual (Lexer sobre &[u8] con cursor, scan_token maximal-munch, skip_whitespace, scan_identifier/number/string con escapes), parser descendente recursivo (Parser sobre Vec<Token>, una función por regla EBNF, precedence climbing por funciones parse_or→parse_and→parse_not→parse_comparison→parse_primary). Reutiliza todos los tipos del cap 17. Cambios al AST del cap 17: TokenKind::Dash (extremos -[...] y ]-) y Expression::Variable (hito RETURN p). Errores tipados LexError (UnexpectedChar/UnterminatedString/InvalidEscape/IntegerOverflow/MalformedNumber) y ParseError (Lex propagado/UnexpectedToken/UnexpectedEof/MissingMatch/MissingReturn/MalformedRelationship/TrailingTokens) con Display estilo rustc/miette e impl std::error::Error con source(). API parse()/parse_query(). 73 tests propios, ALL_GREEN (366 tests workspace total). Parte IV caps 17-18 cerrados.
- 2026-08-14 — Sesión 9: estudio estratégico (`PROPUESTA-EVOLUCION.md`) + **ADR-005 aprobado**: se crea el Vol.III «Grafos en la era de la IA» (13 caps/3 partes, hilo conductor KB-Lira) con outline completo (`OUTLINE-VOL3.yml`, `CURRICULUM-VOL3.yml`, esqueleto `vol3-grafos-era-ia.md`); refuerzos quirúrgicos al guion del Vol.II sin renumerar (caps 15/16/21/30/38/40 + Apéndice E paisaje 2026 post-Kùzu); pipeline pedagógico integrado en CONVENTIONS §2 (curriculum-designer → book-outline-architect → chapter-planner → chapter-writer + exercise-designer → pedagogical-reviewer, más metodología teaching de opencode). CORPUS extendido con 9 temas y 14 capítulos del Vol.II. Contexto de mercado documentado: Kùzu archivada tras compra de Apple (oct. 2025), GraphRAG en producción, GQL ISO 39075.
- 2026-08-14 — Sesión 10: cap Vol.II 19 (Del AST al plan lógico) migrado a `vol2-liradb`. Pipeline parse() → lower() → LogicalPlan. LogicalType (inferencia de tipos básica, conservadora por schemaless), Bindings (tabla de variables ligadas Node/Edge en orden de declaración), ScalarExpr (Expression resuelto: sin Span, Var con BindingKind incrustado, HasLabel construido por el planner), operadores NodeScan/Expand/Filter/Project/CartesianProduct con Display indentado (base de liradb explain cap 21) y bound_variables(), PlanError tipado (UnknownVariable/DuplicateVariable/VariableRebind/SharedPatternVariables/TypeMismatch/EmptyMatch/EmptyReturn) con span. Bug del brief corregido: su plan de ejemplo omitía imponer f:Person — ahora baja como predicado HasLabel al Filter. 40 tests propios, ALL_GREEN (406 tests workspace). Caps 7-19 migrados (13 caps); Parte IV queda con Volcano (cap 20) y optimizador (cap 21) pendientes.
- 2026-08-14 — Sesión 11: cap Vol.II 20 (El motor de ejecución — modelo Volcano) migrado a `vol2-liradb`. HITO DEL BRIEF CUMPLIDO: ejecutar consultas completas desde texto con run(src, store) = parse → lower → compile → Volcano → ResultSet (y Query::execute(&store)). Trait PhysicalOperator (open/next/close del brief + name/rows_produced/collect_metrics para métricas), Row (Vec<(String, Cell)>: la materialización de los Bindings del cap 19), Cell (Scalar(Value) | Node | Edge — RETURN p/r devuelve el elemento entero; igualdad de nodos por identidad de id → self-loops con WHERE a = b), eval_scalar con semántica SQL/Cypher (NULL domina comparaciones, Int/Float promocionan, tipos distintos no iguales pero sin orden, HasLabel sobre arista → NULL) y AND/OR/NOT trivalentes con cortocircuito real y testeado. 8 operadores: NodeScanOp, IndexSeekOp (ids externos: la selección de índice es del cap 21), ExpandOp (OUTGOING/INCOMING/UNDIRECTED, self-loop undirected una vez, filtra rel_type, liga rel_variable), FilterOp, ProjectOp, CartesianProductOp (materializa el lado derecho: Volcano no rebobina), LimitOp y DistinctOp (listos aunque LiraQL aún no exponga las keywords). Executor (ciclo open→next*→close con close siempre, métricas ExecMetrics por operador = semilla del explain cap 21, ResultSet con Display en tabla = semilla CLI cap 31), compile() 1:1 por ahora (el cap 21 reescribirá ahí). ExecError tipado (Parse/Plan envueltos con From+source, TypeMismatch runtime = concreción del Any schemaless). Motor contra &dyn GraphStore del cap 8 (hexagonal; MemoryStore en tests). 37 tests propios, ALL_GREEN (443 tests workspace). Caps 7-20 migrados (14 caps); Parte IV 4/5 — queda el optimizador (cap 21). Desbloqueado: hito de CLI mínima (ejecutar consultas e imprimir la tabla) y explain con cardinalidades reales.
- 2026-08-14 — Sesión 12: **HITO CLI MÍNIMA cumplido** (ADR-005, medida de utilidad "CLI mínima anticipada tras el cap. 20"): LiraDB demostrable desde shell. Crate nueva `crates/vol2-liradb-cli` en el workspace liradb (package `liradb-cli`, binario `liradb` vía [[bin]]) con tres subcomandos: `liradb demo` (4 consultas representativas sobre el grafo demo imprimiendo consulta + plan lógico + tabla ResultSet + métricas por operador), `liradb query "<LiraQL>"` (tabla; errores parse/plan/runtime a stderr con exit 1) y `liradb help`/sin args (ayuda con 2 ejemplos). Parseo de argumentos MANUAL con std::env::args (sin clap — regla "primero a mano"; clap llega en el cap. 31). Exit codes 0/1/2. Testabilidad: lógica en lib.rs con run(args, out, err) -> i32 sobre dyn Write y main fino; 11 tests end-to-end sin spawn + 1 doctest. Para no duplicar datos, `vol2-liradb` expone ahora `demo_graph() -> MemoryStore` público (el fixture Ana/Bo/Carla/Dani + Madrid/Lisboa de los tests del cap. 20 promovido a API única; el fixture de tests delega en él). Bugs de tests corregidos: age<40 devuelve 3 filas (Dani 36), ancho de columna fijo por "Carla" (7 chars). ALL_GREEN (455 tests workspace). NOTA DE ALCANCE: el cap. 31 QUEDA como expansión (REPL interactivo, import/export CSV/GraphML, clap con subcomandos ricos, configuración) — nada de eso se ha adelantado. Docs: code-map.yml (entrada `vol2-hito-cli-minima`, distinta del cap. 31) y MIGRATION-PATTERN §25.- 2026-08-14 — Sesión 13: **ADR-006 — monorepo único público**. Commit previo de caps 12-20 + CLI (`6df4ee1`); historial del workspace (12 commits, autores y fechas preservados) reencauzado bajo `liradb-workspace/` (plumbing: filter-branch no disponible); manuscritos importados encima (`c63fb26`) con README del monorepo, LICENSE CC BY-NC-SA y .gitignore raíz. Publicado en https://github.com/Rubentxu/grafos-bbdd-desde-cero (main, 13 commits). verify.sh ALL_GREEN verificado desde la nueva ubicación + smoke test `liradb query`. A partir de ahora: commits sólo con ALL_GREEN y push a origin/main.
- 2026-08-14 — Sesión 13: **ADR-006 — monorepo único público**. El autor decidió unificar obra+código en un solo repo trazable y público: commit previo de caps 12-20 + CLI (`6df4ee1`); historial del workspace (12 commits, autores y fechas preservados) reencauzado bajo `liradb-workspace/` con plumbing git (read-tree --prefix + commit-tree; filter-branch no disponible); manuscritos importados encima (`c63fb26`) con README del monorepo, LICENSE CC BY-NC-SA y .gitignore raíz. Publicado en https://github.com/Rubentxu/grafos-bbdd-desde-cero (público). verify.sh ALL_GREEN verificado desde la nueva ubicación + smoke test `liradb query`. Disciplina: commits sólo con ALL_GREEN y push a origin/main.
- 2026-08-14 — Sesión 14: **ADR-007 — reestructuración por volúmenes y capítulos** (decisión del autor, previa a la redacción de prosa). Manuscritos: `manuscrito/volN/` fichero-por-capítulo (vol1=46, vol2=6, vol3=5) + SUMARIO.txt + `scripts/build_book.sh` que ensambla los volN-completo.md de la raíz (**reensamblado byte a byte idéntico, verificado con cmp**; `# Cargo.toml` dentro de bloques NO es límite — split_manuscrito.py documenta la heurística). Código: `vol2-liradb/src/lib.rs` (~15.500 líneas) dividido en 14 módulos cap07_modelo…cap20_volcano; lib.rs queda en 105 líneas (docs + mod + pub use); API pública sin cambios, 455 tests antes y después (ALL_GREEN), reconstrucción verbatim byte-idéntica verificada; 5 ítems internos pasaron a pub(crate). code-map.yml actualizado con el mapa real de módulos. Regla editorial: los ensamblados de la raíz nunca se editan a mano; fuentes + build + commit conjunto (build_book.sh --check para detectar desincronía).
- 2026-08-15 — Sesión 15: triple entrega. (1) **Cap. 21 DONE — Parte IV CERRADA (5/5)**: optimizador por reglas (pushdown, combinación de Filters, reordenación por selectividad) + catálogo de estadísticas del store + estimación de cardinalidad por heurísticas + `liradb explain` (plan antes/después con estimaciones vs filas reales) + equivalencia pre/post optimización testeada. Módulo cap21_optimizador.rs (~1.900 líneas, 30 tests; primer capítulo nacido como módulo propio ADR-007); colateral: IndexSeek en cap 19 y su compilación en cap 20. 485 tests workspace ALL_GREEN. Nota: el agente quedó interrumpido por usage-limit dejando el trabajo sin formatear/verificar en el árbol; el orquestador lo completó (fmt, imports, docs). (2) **Estándar de profundidad** (respuesta al feedback del autor «los capítulos son muy cortos, no explican los porqués»): PLANTILLA-CONTRATO-CAPITULO.md — auto-interrogatorio obligatorio por capítulo que integra skill `teach` real (~/.agents/skills/teach/: fluency vs storage strength, retrieval practice/spacing/interleaving, dificultad asimétrica conocimiento/destreza, feedback inmediato, citas de alta confianza) + grill-with-docs (los porqués de cada decisión con fuente) + exercise-designer; CONVENTIONS §2 actualizado. (3) **Capítulo piloto de prosa profunda**: cap-11 (páginas/slotted pages) redactado completo (342 líneas: anécdota Jim Gray, modelo mental libro-contabilidad, porqués de 4096/length-prefix/little-endian/magic redundante, solución ingenua→evolucionada, sacrificios, cómo lo hace PostgreSQL/SQLite/InnoDB, ejercicios con retrieval practice, mini-diálogo) — primer fichero de manuscrito/vol2/; typo corregido en doc-comment de MetaPage.
- 2026-08-15 — Sesión 16: **Parte III completa en prosa profunda (caps. 11-16)**. Redactados los 6 capítulos de la Parte III al estándar de PLANTILLA-CONTRATO-CAPITULO.md (teach + grill-with-docs + exercise-designer), cada uno con contrato formal en book-context/contratos/vol2/: cap-11 (piloto, 342 líneas: Jim Gray, porqués de 4096/length-prefix/little-endian), cap-12 (378: anécdota D. Richard Hipp/SQLite, 10 porqués incl. free list LIFO no persistida como deuda documentada), cap-13 (380: anécdota ARC/patente IBM→clock sweep de PostgreSQL 8.1, 14 porqués incl. el bug real de la aguja Clock), cap-14 (310: Yale Sparse Matrix Package 1977→Kùzu CIDR 2023, CSR vs Vec<Vec> con números de localidad), cap-15 (340: Bayer-McCreight 1970 y el misterio de la B, FNV-1a con vectores oficiales, + apéndice multinivel ADR-005), cap-16 (330: autovacuum PostgreSQL 8.1, repack in-place cuantificado en ~260 punteros, sección LSM-trees ADR-005). Todos: plantilla híbrida completa verificada por script, ejercicios con retrieval practice/spacing/interleaving, anécdotas verificadas con fuentes, prosa anclada al código real (tests citados por nombre). Vol.II ensamblado: 1.998 líneas (era 298).
- 2026-08-15 — Sesión 17: **Parte IV completa en prosa profunda (caps. 17-21) — las Partes III y IV del Vol.II quedan cerradas en código Y prosa.** Redactados en paralelo (5 agentes) al estándar de PLANTILLA-CONTRATO-CAPITULO.md con contratos en book-context/contratos/vol2/: cap-17 (379 líneas: SEQUEL/Chamberlin-Boyce 1974 + Cypher ASCII-art y GQL ISO 39075; design-before-code como tema transversal; discrepancia honesta: TokenKind tiene 34 variantes, no las 32 del brief), cap-18 (342: libro del dragón 1986 + Wirth; los TRES bugs reales como caso de estudio — el bug de <> que se paga LEJOS de la causa), cap-19 (344: Selinger/System R 1979; el bug del brief (f:Person omitido) como caso estrella — un binder descuidado devuelve filas de más en silencio), cap-20 (337: Graefe/Volcano 1994→Cascades; el hito celebrado con liradb query ejecutándose de verdad), cap-21 (317: Selinger otra vez — sus defaults 1/10 y 1/3 son literalmente nuestras constantes de selectividad; salida real del explain reproducida: est. 1 vs 3 reales como contenido, no bug; repaso-árco de la Parte con diagrama de la cadena completa). Todos: plantilla híbrida verificada por script sin secciones faltantes, prosa anclada al código (tests citados por nombre), ejercicios retrieval/spacing/interleaving, anécdotas verificadas con fuentes. Vol.II ensamblado: 2.378 → 3.997 líneas.
- 2026-08-15 — Sesión 18: cap Vol.II 22 (Caminos mínimos ponderados — Dijkstra + Bellman-Ford) migrado a `vol2-liradb`. **ABRE LA PARTE V (algoritmos sobre el grafo persistente)**: el ángulo del Vol.II no es el algoritmo (Vol.I caps 4/9) sino ejecutarlo SOBRE el store con pesos leídos de PROPIEDADES de arista (la consulta del brief: SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance). Módulo cap22_caminos_minimos.rs (~1.290 líneas, 30 tests + 3 doctests, sin crates): WeightSource (Property configurable / Constant con Default 1.0=saltos) + edge_weight con semántica estricta tipada (ausente o NULL=MissingWeight, tipo no numérico=InvalidWeight con type_name, NaN/±∞=NonFiniteWeight, Int→Float con pérdida >2^53 documentada); dijkstra/dijkstra_path sobre &dyn GraphStore (DECISIÓN: no CSR — los pesos viven en Edge.props y el CSR del cap 14 sólo persiste topología; la proyección con pesos es cap 26; el CSR queda como oráculo de consistencia en un test) con BinaryHeap<Reverse<(Cost, NodeId)>> (Cost=newtype f64 con Ord total), borrado perezoso settled, predecesores PathStep (reconstrucción sin re-tocar el store), FINALIZACIÓN ANTICIPATA al destino y sanidad eager O(E) que rechaza negativos ANTES de correr (una BD prefiere fallar ruidosamente a contestar casi-bien); bellman_ford/bellman_ford_path (lista de relajación materializada 1 vez, V-1 pasadas con parada temprana, pasada de verificación → ciclo negativo ALCANZABLE=NegativeCycle señalando la arista que aún relaja, el inalcanzable no contamina, sin early-exit por destino); misma interfaz ambos: ShortestPaths (dist+pred+PathStats con relax_attempts/updates/popped/rounds) y Path (steps con edge/from/to/weight, nodes(), hops(), Display estilo Cypher); PathError (7 variantes) con Display+Error incl. CostOverflow para no confundir infinito real con el centinela de inalcanzable. ALL_GREEN (486→519 tests workspace). Colateral: code-map.yml traía deuda de la interrupción de la Sesión 15 — el bloque integrador no listaba cap21_optimizador y stats/next_action apuntaban al cap 21 ya hecho; corregido junto a la entrada vol2-cap-22, stats (29 caps con código) y next_action (cap 23 A*). Docs: MIGRATION-PATTERN §27, Cargo.toml descripción caps 7-22.
- 2026-08-15 — Sesión 19: cap Vol.II 23 (A*, heurísticas y búsquedas dirigidas) migrado a `vol2-liradb`. Parte V, continuación del cap 22: A* SOBRE el grafo persistente con heurísticas definidas por el USUARIO de la API (primera vez que un algoritmo de la Parte V necesita datos del NODO — coordenadas —, no sólo de la arista). Módulo cap23_a_estrella.rs (~1.000 líneas, 14 tests + 4 doctests, sin crates; hypot de std): trait Heuristic { estimate(&self, store, node) } (trait y no closure: heurísticas con estado ligadas a destino — coords, landmarks —, contrato validado en un sitio, &dyn sin genéricos) con ZeroHeuristic (h≡0 ⇒ A* ES Dijkstra: mismo orden de pops, testeado) y EuclideanHeuristic (recta por props x/y de nodo con la semántica estricta del cap 22: MissingCoordinate/InvalidCoordinate/Int-promoción; destino eager, resto on-demand); a_star reutiliza WeightSource/edge_weight/Path/PathStats/PathError del cap 22 — la sanidad eager de pesos EXTRAÍDA a validate_edge_weights pub(crate) compartida (refactor puro de dijkstra_impl) — con heap por f=g+h (Reverse<(Cost,Cost,NodeId)>, desempate g+id), RE-APERTURA de nodos (óptimo incluso con heurísticas admisibles-inconsistentes; re-expansión medida: 5 expansiones para 4 nodos) y parada al primer pop vivo del destino; validación HONESTA por coste: pesos eager O(E), h finita y ≥0 por estimación cacheada (NaN haría panic en Cost::cmp), admisibilidad documentada y su riesgo DEMOSTRADO (sobre-estimación y unidades km/min ⇒ subóptimo silencioso, testeado), consistencia diagnosticable con check_consistency (local O(E), InconsistentHeuristic tipado) pero NO exigida. PathStats extendido con `expanded` (pops vivos; dijkstra también lo incrementa) → comparativa medible: Dijkstra 13 vs A* 10 en el grafo-trampa y 7 vs 3 en el HITO del brief (red de 7 ciudades españolas con coords km y carreteras ≥ recta: Madrid→Barcelona por Zaragoza 440). PathError de 7 a 12 variantes. ALL_GREEN (519→537 tests workspace). Docs: code-map.yml (entrada vol2-cap-23 + cap-22 backfill en LEDGER + bloque integrador + stats 30 caps + next_action cap 24), MIGRATION-PATTERN §28, Cargo.toml caps 7-23.

- 2026-08-15 — Sesión 20: cap Vol.II 24 (Centralidad y PageRank) migrado a `vol2-liradb`. Parte V, capítulo 3. Las CINCO familias del guion (brief §cap 24: grado, closeness, betweenness, eigenvector, PageRank — "para explicar familias de algoritmos") sobre `&dyn GraphStore` vía una PROYECCIÓN materializada una vez (nodos ordenados → determinismo, índice denso que compacta los huecos de delete_node, vecindarios por GraphDirection Out/In/Both con Both=conjunto y self-loop una vez). Módulo cap24_centralidad.rs (~1.800 líneas, 28 tests + 6 doctests, sin crates): degree O(V+E); closeness por BFS con corrección WASSERMAN-FAUST para componentes desconectadas (ponderado = deuda cap 26); betweenness con BRANDS 2001 (O(V·E), σ/predecesores/dependencias, normalización dirigida que sobre simetrizado reproduce el libro); eigenvector por potencia sobre adyacencia CRUDA con L2/paso — sus DOS fallos demostrados en tests (hojas a 0, oscilación periódica converged=false); PageRank = eigenvector REPARADO: damping ∈ (0,1) ABIERTO validado explícitamente (Range::contains no excluye el 0: inicio inclusivo — bug cazado en tests), dangling redistribuido uniformemente (Brin-Page; masa=1 en cada iteración, invariante testeado), convergencia por L1 (la MASA que se mueve) con `history` de deltas por iteración (razón geométrica ≈ d·λ₂ testeada; contraste: grafo que arranca en el estacionario → history=[0.0]); PPR BIEN SEPARADO para GraphRAG cap 51: enum Teleport {Uniform, Personalized} + núcleo único iteracion_de_potencia compartido (costura limpia, cero duplicación). Soluciones a mano en tests (ciclos compartidos 0.4865/0.2568; cadena+dangling 0.1844/0.3412/0.4744; PPR 0.15/(1-0.85²) con masa EXACTAMENTE 0 fuera del mundo; damping extremos d→0≈uniforme, d→1 más iteraciones; masa por componente ∝ tamaño; multigrafo roba-masa). Bug corregido durante la implementación: la proyección In mezclaba in-edges y transponía (doble error) y Both duplicaba pares en stores simetrizados; y el cálculo a mano del test de demo_graph ignoraba la cuota colgante del self-loop de Dani (real 0.386 TOP, no 1/6 — reescrito como lección de trampa de acumulación). Errores CentralidadError (6 variantes) + CentralidadStats (bfs_runs/edges_scanned/iterations = el "coste computacional" del guion, medible). ALL_GREEN (537→571 tests workspace). Docs: code-map.yml (entrada vol2-cap-24 + bloque integrador + stats 31 caps + next_action cap 25 Louvain), MIGRATION-PATTERN §29, Cargo.toml caps 7-24, lib.rs cabecera.

- 2026-08-15 — Sesión 21: cap Vol.II 25 (Comunidades y agrupaciones — Louvain simplificado) migrado a `vol2-liradb`. Parte V, capítulo 4. El cap 24 rankeó NODOS; éste PARTICIONA el grafo. Las CUATRO familias del guion (brief §cap 25: componentes, label propagation, modularidad, Louvain simplificado) sobre `&dyn GraphStore`. Módulo cap25_comunidades.rs (~2.200 líneas, 22 tests + 4 doctests, sin crates): componentes_conexas (el caso límite, BFS sobre vista simétrica, numeradas por menor miembro); label_propagation (heurística SIN métrica que motiva Louvain: votos ponderados, barrido asíncrono por id, empates → conservar la propia si empata con la máxima y si no la MENOR — determinismo total; su límite GOTEA por puentes con pesos uniformes, testeado vs Louvain sobre el MISMO grafo); modularidad = Q_γ de Newman-Girvan como función VERIFICABLE sobre una partición dada (γ de Reichardt-Bornholdt, trivial Q=0 exacto, ausentes → singletons, ids u64 densificados, UnknownNode); louvain = Blondel 2008 simplificado: fase local greedy con ΔQ exacto por diferencia de términos de comunidad (sólo ΔQ>0 estricto) + AGREGACIÓN en supernodos (2m conservado ⇒ Q de cada nivel igual en contraído y original, invariante testeada) hasta convergencia. DECISIÓN: GrafoPonderado propio (simétrico, dirigidas SUMADAS al par, paralelas ACUMULADAS, self-loops con convención A_ii=2s contando doble) y NO la Proyeccion del cap 24 (no ponderada y Both DEDUPLICA; además Louvain reconstruye el grafo por nivel); pesos con semántica estricta del cap 22 (WeightSource/edge_weight, From<PathError>), negativos rechazados eager. Determinismo TOTAL (nodos por id, candidatos por id de comunidad, total_cmp, renumeración por menor miembro; dos ejecuciones idénticas incluso con orden de inserción invertido — testeado). Cota de terminación demostrable: cada nivel con movimientos reduce estrictamente los nodos (el primer movimiento vacía una comunidad singleton) ⇒ niveles ≤ V; max_pasadas por nivel como seguro anti-ruido-f64. JERARQUÍA para GraphRAG cap 51: NivelLouvain con asignación de nodos ORIGINALES (composición), Q por nivel, anidamiento garantizado (testado en la dirección correcta: fina ⇒ gruesa), Q monótono entre niveles, particion_en(nivel) como dendrograma a demanda. Tests estrella: LÍMITE DE RESOLUCIÓN Fortunato-Barthélemy demostrado (anillo de 12 tríos: γ=1 → 6 pares Q=17/24 con DOS niveles 12→6; γ=2 → los 12 tríos Q=7/12; valores Q_γ verificados analíticamente con modularidad()); dos K4+puente separadas (Q=11/26); ground truth sintético recuperado EXACTO (3 anillos con cuerdas + 2 eslabones, Q ≥ Q_truth); pesos que RESTRUCTURAN (puente w100 rompe los tríos en {0,2},{1,4},{3,5} con Q=100/2809 — Q invariante a escala: no es "más peso = más fusión"); demo_graph con DOS óptimos Q=5/18 separados por un dq exactamente 0 (Dani solo por su self-loop). Hallazgos corregidos: LPA determinista gotea por puentes (política keep-own + pesos), el camino 0-1-2 se funde entero en LPA (cascada, documentado), y max_pasadas=1 NO rebaja calidad (la agregación repara el nivel truncado — testeado). Errores ComunidadesError (5 variantes, Display+Error) + ComunidadesStats (edges_scanned/pasadas/movimientos/niveles) + Particion como interfaz común (grupo/grupos/tamanos). ALL_GREEN (571→597 tests workspace). Docs: code-map.yml (entrada vol2-cap-25 + bloque integrador + stats 32 caps + next_action cap 26 proyección con pesos), MIGRATION-PATTERN §30, Cargo.toml caps 7-25, lib.rs cabecera.

- 2026-08-16 — Sesión 22: **Cap. 26 DONE — Parte V CERRADA (5/5)**. ProyeccionPonderada pública con pesos (deuda de caps 22/24 saldada: CSR heredero del cap 14, sanidad del cap 22 pagada una vez, filtros etiqueta/tipo, dijkstra_proyeccion + closeness_ponderado sobre ella) + streaming por fronteras SIN materializar (FronterasBfs iterator, Presupuesto triple con MotivoParada, bfs_streaming) + BitSet denso vs HashSet disperso + ContandoStore (voltímetro que verifica externamente las lecturas). Test-tesis: BFS profundidad 2 sobre cadena de 500 lee 2/499 aristas; test-economía: K Dijkstras vía proyección = E lecturas totales vs Σ(E−i) por store. 27 tests + doctests, ALL_GREEN (629 workspace). NOTA: agente interrumpido por usage-limit dejó el módulo completo sin cablear ni verificar; orquestador completó (lib.rs, 2 errores compilación, 4 lints, 4 expectativas recalibradas trazando el código a mano — tabla completa en MIGRATION-PATTERN §31). Con esto, 20 de 40 caps del Vol.II tienen código verificado (7-26) y 11 tienen prosa profunda (11-21).
