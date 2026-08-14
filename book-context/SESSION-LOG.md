# SESSION-LOG — Bitácora de sesiones

> Log cronológico de sesiones del orquestador del libro. Mantener conciso.

## 2026-07-30 — Sesión 1

**Asistentes**: book-orchestrator + 3 Explore agents (parallel).

**Trabajo realizado**:
- Lectura estructural de `grafos-de-cero-a-experto-rust-v3.md` (14936 líneas, 32 caps + 4 apéndices, estilo Grokking 2.0).
- Lectura estructural de `Libros-y-tutoriales-de-grafos-a-crear.md` (4545 líneas, export ChatGPT con brief de LiraDB: 2 TOC divergentes — 40 caps/8 Partes y 54 caps/6 Partes; deep-dive de 18 conceptos sobre Ladybug/Kùzu).
- Diagnóstico: los dos documentos NO se referencian entre sí; son independientes. El brief NO menciona el v3 por nombre.
- 4 preguntas de clarificación al usuario resueltas:
  1. Tipo de fusión → 2 volúmenes (v3 = Vol.I, LiraDB = Vol.II).
  2. Guion Vol.II → 40 caps / 8 Partes.
  3. Nombre proyecto → LiraDB (definitivo).
  4. Estilo → Manual de estilo unificado en Apéndice 0.
- Plan de fusión presentado en modo plan y aprobado por el usuario.
- **Fase 0 ejecutada**:
  - Backup `vol1-v3-backup-20260730.md` (767 KB, idéntico al original vía `diff`).
  - Renombrado `grafos-de-cero-a-experto-rust-v3.md` → `vol1-grafos-de-cero-a-experto-rust.md`.
  - Creado `book-context/` y movido `Libros-y-tutoriales-de-grafos-a-crear.md` → `book-context/brief-liradb-original.md`.
  - Creado `book-context/LEDGER.md`, `SESSION-LOG.md`, `CONVENTIONS.md`, `CORPUS.yml`.

**Estado al cierre**: Fase 0 completada. Pendiente Fase 1 (cross-references Vol.I) y Fase 2 (Apéndice 0).

**Próxima sesión**: arrancar Fase 1 (cross-references) o Fase 2 (Apéndice 0) según prioridad del usuario.

## 2026-08-14 — Sesión 9

**Asistentes**: book-orchestrator + skills (book-outline-architect, book-memory-keeper) + curriculum-designer/exercise-designer (integradas como pipeline).

**Trabajo realizado**:
- Estudio estratégico de la obra (`PROPUESTA-EVOLUCION.md`) con investigación de mercado: Kùzu archivada tras compra de Apple (oct. 2025), GraphRAG en producción, GQL ISO 39075, mercado ~2.900M$→20-25B$ 2034.
- **ADR-005 aprobado (Propuesta A)**: Vol.III «Grafos en la era de la IA» (13 caps/3 partes) + refuerzos quirúrgicos Vol.II + medidas de utilidad (dataset KB-Lira, CLI anticipada, prosa en paralelo).
- Creados: `vol3-grafos-era-ia.md` (esqueleto con Prólogo borrador), `book-context/OUTLINE-VOL3.yml` (13 caps con secciones/conceptos/objetivos/dependencias validadas), `book-context/CURRICULUM-VOL3.yml` (grafo de conceptos, orden topológica, out-of-scope), `book-context/adr/005-vol3-y-refuerzos.md`.
- Refuerzos aplicados al ToC del Vol.II (sin renumerar): cap. 15 (+B+ tree multinivel), cap. 16 (+LSM), cap. 21 (+estadísticas), cap. 30 (ampliado: 2PL, aislamiento, OCC, deadlocks con grafo de espera), cap. 38 (+compresión), cap. 40 (+híbridos), Apéndice E (paisaje 2026 post-Kùzu).
- Pipeline pedagógico integrado en `CONVENTIONS.md` §2: skills zcode (curriculum-designer → book-outline-architect → chapter-planner → chapter-writer + exercise-designer → pedagogical-reviewer) + metodología **teaching de opencode** (`~/.config/opencode/teaching/`): contrato de dominio knowledge/skills/wisdom, modelo mental, reflexión, preguntas abiertas, design-before-code.
- CORPUS.yml: +9 temas Vol.III, +14 capítulos (41-53 + epílogo), stats 55 caps.
- LEDGER.md: vol-III registrado, ADR-001 pendiente de reescritura como relato histórico, sesión 9.

**Estado al cierre**: obra a 3 volúmenes. Vol.II: código 12/40 caps (ALL_GREEN, 366 tests), prosa 0/40. Vol.III: outline DRAFT → pendiente aprobación fina del autor antes de chapter-planner.

**Próxima sesión** (opciones): (a) chapter-planner + chapter-writer para vol-III-cap-41 tras aprobar outline; (b) prose drafting Parte III Vol.II (caps 11-16, bloque cerrado en código); (c) continuar código Vol.II cap. 19 (plan lógico).

## 2026-08-14 — Sesiones 10-13 (resumen consolidado)

**Sesión 10** — cap. 19 (plan lógico): `lower()` → `LogicalPlan` (NodeScan/Expand/Filter/Project/CartesianProduct), `Bindings`, `ScalarExpr` resuelto, `LogicalType` conservador, `PlanError` con span; bug del brief corregido (labels bajan como `HasLabel` al Filter). 40 tests.

**Sesión 11** — cap. 20 (Volcano): `PhysicalOperator{open,next,close}`, `Row`/`Cell`, `eval_scalar` (NULL SQL/Cypher, promoción Int/Float, trivalentes con cortocircuito), 8 operadores, `Executor`+`ExecMetrics`, `run(src,store)` end-to-end. 37 tests. HITO del brief: consultas ejecutadas desde texto.

**Sesión 12** — hito CLI mínima (ADR-005): crate `liradb-cli` (binario `liradb`), `demo|query|help`, sin clap, `demo_graph()` API pública. 12 tests. Workspace: 455 tests ALL_GREEN.

**Sesión 13** — **ADR-006: monorepo único público**. El autor decidió unificar todo: commit de caps 12-20+CLI, reescritura del historial del workspace bajo `liradb-workspace/` (12 commits preservados), importación de manuscritos, README/LICENSE/.gitignore raíz, y publicación en **https://github.com/Rubentxu/grafos-bbdd-desde-cero** (público, main, ALL_GREEN verificado). La decisión original de «repo separado» queda superseded.

**Sesión 14** — **ADR-007: estructura por volúmenes y capítulos** (petición del autor, ejecutada ANTES de redactar prosa). Manuscritos: `manuscrito/vol1|2|3/` con fichero-por-capítulo (46/6/5 ficheros) + `SUMARIO.txt` + `scripts/build_book.sh` (ensambla → ficheros completos de la raíz; **reensamblado byte a byte idéntico verificado con cmp**; `--check` detecta desincronía; `split_manuscrito.py` documenta que `# Cargo.toml` dentro de bloques no es límite). Código: `vol2-liradb/src/lib.rs` (~15.500 líneas) dividido en 14 módulos `cap07_modelo.rs`…`cap20_volcano.rs` vía agente (lib.rs: 105 líneas de mod+pub use; 455 tests antes/después ALL_GREEN; reconstrucción verbatim byte-idéntica; 5 ítems a `pub(crate)`; API pública intacta). code-map.yml con mapa real de módulos. Regla editorial: los ensamblados nunca se editan a mano.

**Estado al cierre**: monorepo público estructurado por capítulos. Vol.II: código 14/40 caps (7-20) + CLI mínima; prosa 0/40. Vol.III: outline aprobado. Parte IV 4/5 (falta cap. 21 optimizador).

**Próxima sesión**: (a) cap. 21 (optimizador + explain + estadísticas) sobre la nueva estructura de módulos — añadirá `cap21_optimizador.rs` y fichero `manuscrito/vol2/cap-21-*.md`; (b) prosa Parte III Vol.II (ficheros `manuscrito/vol2/cap-1[1-6]-*.md` ya creados por el split); (c) chapter-planner vol-III-cap-41.