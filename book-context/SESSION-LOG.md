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
## 2026-08-25 — Sesión 34

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer (agentes secuenciales).

**Trabajo realizado**: Cap. 33 «Pruebas de una base de datos» DONE (código Y prosa), Parte VII cap 3.
- Contrato por planner-agente (`book-context/contratos/vol2/cap-33.md`, 277 líneas): torre de riesgos (cada piso ataca un riesgo que el inferior no ve); hallazgo al planear: `WalIterator` traga cola corrupta en silencio (cap28_wal.rs:856).
- Código por generator-agente: `cap33_pruebas.rs` (~1.424 líneas; invariantes sobre el puerto hexagonal, batería de contrato con StoreAlternativo didáctica, proptest 1.11.0 con 5 propiedades, crash suite sobre bytes reales + `cargar_wal_estricta`, compat magic+versión) y primer `tests/` del workspace (`golden_cli.rs` + dorados, `ACTUALIZAR_GOLDEN=1`). Cero toques caps 7-32.
- Prosa por writer-agente (313 líneas, 20 secciones): anécdota Gray TR 85.7, salidas reales ejecutadas (WAL 548 B/13 registros: indulgente Ok(12)/txs 2de3 vs estricta RegistroTruncado).
- Docs: code-map, MIGRATION-PATTERN §38, SUMARIO, LEDGER (incl. registro retroactivo Sesión 33-bis = prosa cap-32, commit 0e27c4e).

**Estado al cierre**: ALL_GREEN 809 tests. Vol.II 33/40 caps con prosa; Parte VII 3/6 (quedan 34 benchmarks, 35 observabilidad, 36 arquitectura final). Commits 940403c / fa846c2 / f20fab7 en main (pushed). build_book.sh --check: ALL_ASSEMBLED.

**Próxima sesión**: cap-34 «Benchmarks y perfilado» (dataset de referencia 100k/500k, warm/cold cache, percentiles, flamegraphs; criterion entra aquí tras la regla «primero a mano»; LiraDB contra sí misma, no contra Neo4j). Antes del cap-38: resolver ADR-001.

## 2026-08-25 — Sesión 35

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer.

**Trabajo realizado**: Cap. 34 «Benchmarks y perfilado» DONE (código Y prosa), Parte VII cap 4.
- Contrato por planner-agente (`book-context/contratos/vol2/cap-34.md`, 288 líneas): torre de instrumentos; realidad verificada: sin store-en-disco end-to-end ⇒ cold-cache honesto a nivel componente; criterion entra aquí (gancho del cap-33).
- Código por generator-agente: `cap34_benchmarks.rs` (~1.036 líneas; Xorshift64Star a mano, dataset_referencia() determinista 100k/500k/10 etiquetas/20 claves, percentiles std) + primer `benches/` del workspace (bench_micro + bench_consultas, criterion 0.7) + [profile.bench] debug=true. 809 → 822 ALL_GREEN.
- cargo bench ejecutado con cifras reales (Xeon E5-2682 v4): point-lookup 2,19 µs; hub vs grado bajo ×794; CSR vs puerto ×16; pool frío/caliente ×142; trampa ×3,5. HALLAZGO ESTRELLA: Catalog::collect O(valores_distintos²) — ~224 s vs 281 ms del grafo completo; deuda documentada, no parcheada.
- Prosa por writer-agente (320 líneas): Weicker/Dhrystone→SPEC, tablas reales, Pentium FDIV, mini-diálogo p99, gancho cap-35.

**Estado al cierre**: ALL_GREEN 822 tests. Vol.II 34/40 caps con prosa; Parte VII 4/6 (quedan 35 observabilidad, 36 arquitectura final). Commits a7945d5 / 5cce872 pushed. build --check ALL_ASSEMBLED.

**Próxima sesión**: cap-35 «Observabilidad interna» (metrics+tracing, span hierarchy query→plan→operator→page fetch, hito `liradb query --profile`; ExecMetrics cap 20 + Metrics pool cap 13 ya existen — aquí se INSTRUMENTA). Después: cap-36 arquitectura final cierra Parte VII. Antes del cap-38: resolver ADR-001. Candidata de deuda técnica: reparar Catalog::collect cuadrático (MIGRATION §39).

## 2026-08-25 — Sesión 36

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer (×2 capítulos en secuencia).

**Trabajo realizado**: Caps. 35 Y 36 DONE — **PARTE VII CERRADA (31-36)**.
- **Cap-35 Observabilidad interna**: contrato (299 líneas), código (cap35_observabilidad.rs ~1.045 líneas std-only + observabilidad.rs CLI ~842 con SuscriptorArbol propio sobre tracing-core + hito `liradb query --profile` aditivo; tracing 0.1.44 SOLO en la CLI), prosa (356 líneas, Dapper TR 2010, Apolo 13 historia pequeña). Hallazgos: bug dos-fuentes-que-se-suman (v1 contaba nodes_scanned doble), try_close default no notifica, coste ≈0 spans sin subscriber demostrado con goldens intactos. 822→843 tests.
- **Cap-36 Arquitectura final**: contrato (294 líneas, decisión: síntesis con UN smoke `tests/arquitectura.rs`), código (5 tests/509 líneas: la pila completa en una respiración — nadie había probado el edificio entero), prosa (372 líneas, Wren/epitafio San Pablo). Hallazgos de componer el edificio: HashIndex capacity ≥ 3+num_buckets (acoplamiento oculto entre adaptadores), Float(2.0) no sobrevive roundtrip JSONL, reloj MVCC pre-incremento ts=1, recuperación solo renace lo confirmado por WAL. 843→848 tests.

**Estado al cierre**: ALL_GREEN **848 tests**. Vol.II **36/40 caps con prosa** (~12.718 líneas ensambladas); Partes I-VII cerradas. Commits df7ae95 / 1febf51 pushed. build --check ALL_ASSEMBLED.

**Próxima sesión**: Parte VIII (caps 37-40). ANTES del cap-38 resolver ADR-001 con el autor (reescribir atribución Ladybug/Kùzu como relato histórico post-Apple oct. 2025). Deudas documentadas candidatas a reparar algún día: Catalog::collect cuadrático (§39), HashIndex capacity (§41), Float-entero en JSONL (§41).

## 2026-08-25 — Sesión 37

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer (prosa completada tras interrupción, verificada estructuralmente por el orquestador).

**Trabajo realizado**: ADR-001 RESUELTA + Cap. 37 DONE — ABRE la Parte VIII.
- ADR-001: investigación web con fuentes independientes; 2 errores factuales corregidos (LadybugDB = fork comunitario; paper CIDR 2023); línea temporal fuenteada en adr/001; prólogo/colofón/ToC/caps 15+18 corregidos. Cap-38 desbloqueado.
- Cap-37 «Qué necesitaría una BBDD de producción»: contrato (292 líneas) + cap37_produccion.rs (623 líneas std puro: informe_produccion() hermano de informe_acid(), 11 dimensiones 0·6·5, test-pinzón bidireccional) + prosa verificada (20 secciones, ransomware MongoDB ene-2017 con cifras fijadas, B-17 gust lock como historia pequeña).

**Estado al cierre**: ALL_GREEN **853 tests**. Vol.II **37/40 caps con prosa** (~13.005 líneas). Parte VIII abierta. build --check ALL_ASSEMBLED.

**Próxima sesión**: cap-38 «Almacenamiento columnar y ejecución vectorizada» (row vs column, dictionary encoding, bit-packing, SIMD, batch, factorización; citar Kùzu CIDR 2023 con relato histórico correcto; semilla CSR×16 del cap 34). Después: cap-39 WCOJ y cap-40 distribución/Raft cierran el Vol.II.

## 2026-08-25 — Sesión 38

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer.

**Trabajo realizado**: Cap. 38 «Almacenamiento columnar y ejecución vectorizada» DONE (código Y prosa), Parte VIII cap 2.
- Contrato por planner-agente (`book-context/contratos/vol2/cap-38.md`, 299 líneas): columnar como CAPA DE LECTURA sobre MemoryStore; SIMD honesto = auto-vectorización medida sin nightly; factorización mínima viable; ADR-001 aplicada (Kùzu CIDR 2023).
- Código por generator-agente: cap38_columnar.rs (~1.458 líneas) + bench_columnar.rs (tercer [[bench]]). Hallazgo estrella medido y verificado con asm: filtro row 34,574 ms vs columna-lotes 548 µs = ×63 LAYOUT-no-SIMD. Diccionario bidireccional (idioma pierde ×0,50 con cardinalidad 6: regla len-vs-código). Packing ×6,40. Factorización 64,5% ahorro de celdas.
- Prosa por writer-agente (396 líneas): MonetDB/X100 CWI como N.0, Grace Hopper nanosegundo como historia pequeña, tablas con cifras reales hardware declarado.

**Estado al cierre (checkpoint de fin de jornada)**: ALL_GREEN **864 tests**. Vol.II **38/40 caps con prosa** (~13.401 líneas ensambladas); Partes I-VII cerradas; Parte VIII 2/4. Todo pushed hasta 240e295. build --check ALL_ASSEMBLED.

**Próxima sesión (mañana)**:
1. **Cap-39 «Joins, patrones y consultas cíclicas»** — expand como join, WCOJ, LeapFrog Triejoin simplificado, triángulos, consultas recursivas (penúltimo capítulo; el motor factorizado del cap-38 queda como semilla).
2. **Cap-40 «Distribuir una BBDD de grafos»** (Raft, sharding hash vs comunidad, cut edges) — CIERRA el Vol.II.
3. Tras cap-40: Apéndice 0 (verificar si al estándar o placeholder), epílogo, cierre del volumen.
4. Deudas documentadas candidatas (MIGRATION §39/§41/§43): Catalog::collect cuadrático, HashIndex capacity ≥ 3+num_buckets, Float-entero JSONL.

## 2026-08-26 — Sesión 39

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer.

**Trabajo realizado**: Cap. 39 «Joins, patrones y consultas cíclicas» DONE (código Y prosa), Parte VIII cap 3.
- Contrato por planner-agente (`book-context/contratos/vol2/cap-39.md`, 323 líneas): COBRA las tres deudas heredadas (cap-14 «WCOJ sobre adyacencias ordenadas», cap-20 «joins reales WCOJ», cap-38 «motor factorizado completo = cap 39»). Citas verificadas hoy: AGM FOCS 2008 (rev. SICOMP 42(4) 2013); NPRR PODS 2012 pp. 37-48; survey «Skew Strikes Back» SIGMOD Record 42(4) **2014**; LeapFrog ICDT 2014 + ICDT Test-of-Time 2024; factorización **ICDT 2012** / TODS 40(1) mar-2015. ⚠️ HALLAZGO: el contrato de cap-38 citaba «Olteanu-Závodný, ICDT 2015» (mezcla venue/año) — detectado, NO corregido allí por regla de fichero único; deuda menor pendiente.
- Código por generator-agente: `cap39_joins.rs` (1.470 líneas std puro, 14 tests con nombres EXACTOS del §2: expand-como-join formalizado, intermedios K₈ 392→56, join binario vs fuerza bruta, BuscadorSalto salto exponencial+binary_search, frontera_comun, TriangulosWcoj orden estático a→b→c, cota_agm ⌊m^1,5⌋, cierre_transitivo con BitSet/Presupuesto del cap-26, ResultadoFactorizadoTriangulos, informe reproducible) + CUARTO `[[bench]] bench_joins.rs` (178 líneas) + wiring aditivo (lib.rs +2, Cargo.toml +8). Cero toques caps 7-38.
- Bench real (Xeon E5-2682 v4 @2.50GHz): regular (800n/11.906a) 2,36 ms vs 1,15 ms (**~2×**); hub-skew (rueda 513n) 2,14 ms vs **72 µs** (**×29**, intermedios 266.752 para 512 triángulos = ratio ×521); enumeración factorizada vs plana 570 ns vs 382 ns (~1,5×). Honestidad declarada en prosa: delta regular modesto confirma la misconcepción «WCOJ no gana siempre»; pasos WCOJ hub 12.832 ≪ AGM 92.681.
- Prosa por writer-agente (388 líneas, 20 secciones verificadas): anécdota LogicBlox/Veldhuizen «implemented before its optimality guarantees were discovered» (ICDT 2014 + Test-of-Time 2024); historia pequeña = el propio Test-of-Time desde el ángulo producción→prueba→premio; K₈ contado a mano; panel dual join-oriented vs variable-oriented; ganchos cap-38 («columnas = CÓMO lees») y cap-40 («¿y cuando el grafo no cabe en una?»).
- SUMARIO.txt actualizado (cap-39 tras cap-38); rebuild vol2 13.401 → **13.789 líneas** (+388 exactas).

**Estado al cierre**: ALL_GREEN **878 tests**. Vol.II **39/40 caps con prosa** (~13.789 líneas ensambladas); Parte VIII 3/4 (queda cap-40). Commits e248c0c / 0a4f170 en main (pushed). build_book.sh --check ALL_ASSEMBLED.

**Próxima sesión**: cap-40 «Distribuir una BBDD de grafos» (particionado por nodos, edge/vertex cuts, replicación de fronteras, consultas entre particiones, consistencia, Raft, rebalanceo, hotspots) — **CIERRA EL VOL.II**. Después: Apéndice 0 (decidir si al estándar o placeholder), epílogo/colofón, preflight D1 (book-builder). Deuda menor: corregir cita Olteanu-Závodný en contrato cap-38 (ICDT 2015 → ICDT 2012/TODS 40(1) 2015). Deudas mayores documentadas (MIGRATION §39/§41/§43): Catalog::collect cuadrático, HashIndex capacity, Float-entero JSONL.

## 2026-08-26 — Sesión 40

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer.

**Trabajo realizado**: Cap. 40 «Distribuir una base de datos de grafos» DONE (código Y prosa) — **CIERRA EL VOL.II: 40/40 caps con prosa al estándar**.
- Contrato por planner-agente (`book-context/contratos/vol2/cap-40.md`, 342 líneas): cobra CUATRO deudas (cap-36 pregunta abierta multi-máquina, cap-37 informe_produccion, cap-39 gancho saliente, cap-30 `GrafoEspera` enchufada por fin). Citas verificadas: Raft USENIX ATC '14 pp.305-319; Pregel SIGMOD '10; PowerGraph OSDI '12 pp.17-30; Petrov O'Reilly 2019; Kùzu CIDR 2023/ADR-001. DECISIÓN #11: SIN bench nuevo — la moneda aquí son enteros exactos, no µs.
- Código por generator-agente: `cap40_distribucion.rs` (1.835 líneas std puro, 14 tests): 3 estrategias de particionado (hash FNV-1a mód k reutilizado del cap-15, comunidad vía louvain() del cap-25, codicioso), MetricasCorte (definiciones PowerGraph), replicar_hub vertex-cut, bfs_entre_particiones con contador de mensajes, carga+rebalanceo, EnjambreRaft DETERMINISTA (tics lógicos, timeouts escalonados fijos 10/15/20, bus FIFO, sin RNG/sleeps/hilos), deadlock entre particiones con GrafoEspera global. Wiring aditivo lib.rs +2; Cargo.toml intacto.
- Cifras reales medidas: comunidad 630 cortes vs hash 1062 (**−40,7%**) PERO desbalancea (tam máx 115 / mín 23); hash balancea 50×8 exacto con hotspot 14,5% en la partición del hub; codicioso literal corta el 100% (lección greedy sin lookahead, reportada tal cual); vertex-cut estrella 4→0 pagando 3 réplicas; Raft elección estable en tic 10, logs idénticos, mayoría caída congela compromiso, rezagado <200 tics.
- Prosa por writer-agente (378 líneas, 20 secciones): anécdota Pregel→PowerGraph («invertir el cuchillo»); historia pequeña Raft «In Search of an UNDERSTANDABLE Consensus Algorithm»; tabla trade-off real pegada; gancho de cierre de VOLUMEN hacia el epílogo (hexágono del cap-36 respondido).
- SUMARIO.txt actualizado; rebuild vol2 13.789 → **14.167 líneas** (+378 exactas).

**Estado al cierre**: ALL_GREEN **892 tests**. Vol.II **40/40 caps DONE** (~14.167 líneas ensambladas); las 8 Partes cerradas en código Y prosa. Commits f1bf9b5 / 56dd171 pushed.

**Próxima sesión (cierre del volumen antes de D1)**:
1. **Apéndice 0 «Manual de estilo unificado»**: decidir si está al estándar o es placeholder (LEDGER lo marca PLANNED).
2. Epílogo + colofón: revisar que cierren el viaje (pueden ser borradores breves); corregir deuda menor cita Olteanu-Závodný cap-38 (ICDT 2015 → ICDT 2012/TODS 40(1) 2015) en contrato Y prosa si la heredó.
3. **Preflight D1**: todos los caps DONE ✓, verify-report ALL_GREEN ✓ → `book-builder` (HTML/PDF/EPUB) + citation-manager.
4. Deudas mayores documentadas (MIGRATION §39/§41/§43): Catalog::collect cuadrático, HashIndex capacity ≥ 3+num_buckets, Float-entero JSONL — candidatas a reparar antes o después del render (decisión del autor).
