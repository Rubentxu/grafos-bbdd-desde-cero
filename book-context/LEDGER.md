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
  # ... caps 22-40 por planificar
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
