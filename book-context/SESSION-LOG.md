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

## 2026-08-26 — Sesión 41

**Asistentes**: book-orchestrator + 2 agentes en paralelo (apéndice-0 / epílogo+colofón) + orquestador (corrección de citas + preflight D1 + render).

**Trabajo realizado**: CIERRE Y RENDER DEL VOL.II — **VOLUMEN DONE**.
- Apéndice 0 al estándar (115 líneas): 3 volúmenes, plantilla híbrida real de 20 secciones, proceso de producción verificado, ejemplos canónicos reales (Jim Gray cap-11, MonetDB/X100 cap-38, ×29 cap-39, Raft cap-40).
- Epílogo redactado (140 líneas): viaje de las 8 Partes, honestidad LiraDB (qué NO hace y por qué), contribuciones/deudas como ejercicios, carta al lector; cita de cierre Feynman «What I cannot create, I do not understand». Colofón completado (13 líneas; atribución Kùzu intacta).
- Cita Olteanu-Závodný corregida en los 6 sitios: prosa cap-38 (×2), contrato cap-38 (×4) → ICDT 2012 (+ TODS 40(1), 2015); referencias a la deuda en contrato cap-39 actualizadas a «corregida».
- **PREFLIGHT D1 VERDE**: 40/40 caps DONE + verify-report ALL_GREEN (892 tests).
- **RENDER BUILT** (pandoc 3.6.4 + typst 0.15.1 instalados user-local en ~/.local/bin): HTML ×2 (~2,2 MB c/u con TOC), EPUB ×2 (490 KB vol1 / 759 KB vol2), PDF ×2 (5,6 MB vol1 / 7,4 MB vol2, Liberation Serif/Fira Code). Vol.III excluido (SKELETON). `build/manifest.json` con sha256.
- **BUG REAL DESCUBIERTO POR EL RENDER**: anchor roto en la TOC interna del Vol.I (#apéndice-b--glosario vs heading «Glosario (extendido)») — corregido en fuente manuscrito/vol1/tabla-de-contenidos.md + rebuild (commit 44351e6). Workaround documentado para headings con ²/³ (typst no acepta esos chars en labels): normalización en copia temporal solo-headings, fuente canónico intacto.

**Estado al cierre**: VOL.II **DONE Y RENDERIZADO** (14.323 líneas ensambladas). Obra: Vol.I DONE (2ª ed.), Vol.II DONE, Vol.III SKELETON. build/ gitignored (artefactos locales). Commits b293df2 / 44351e6 pushed. ALL_GREEN 892 tests.

**Próxima sesión (opciones)**:
(a) Publicar artefactos (GitHub Pages/release) — requiere decisión del autor;
(b) Arrancar Vol.III «Grafos en la era de la IA»: chapter-planner del cap-41 (outline ya aprobado);
(c) D2 mantenimiento: version-drift-detector + code-prose-coherence-checker(drift) periódicos;
(d) ~~Deudas técnicas MIGRATION §39/§41/§43~~ → **HECHAS en Sesión 42**.

## 2026-08-26 — Sesión 42

**Asistentes**: book-orchestrator + code-example-generator (reparaciones) + writer-agente (actualización prosa).

**Trabajo realizado**: las TRES deudas técnicas documentadas REPARADAS y la prosa sincronizada.
- **D1 Catalog::collect** (cap21_optimizador.rs): O(V²)→O(V) con HashMap de clave canónica (propiedad, etiqueta, ClaveValor con Float por bits, −0.0 normalizado) + Vec en orden de primera aparición. Medido post-fix: ~224 s → **~3-4 s (×68)** sobre dataset_referencia; suelo restante = hashing SipHash (~1 µs/insert, 4.860.024 pushes legítimos); escalado lineal verificado (20k→93 ms). API pública intacta. Tests cap21 +4 (+1 #[ignore] de medición).
- **D2 HashIndex::create** (cap15_indices.rs): validación eager con variante nueva `IndexError::CapacidadInsuficiente { requerida, disponible }`. **HALLAZGO CORRECTIVO: el mínimo real es `1+num_buckets`, NO `3+num_buckets`** como decía la prosa del cap-36 — las páginas 0/1 se allocan directo en el pager y NUNCA pasan por el pool durante create; verificado con test de frontera exacto (capacity==1+B funciona).
- **D3 Float JSONL** (cap32_import_export.rs): serialización `{f:?}` → Float(2.0) sale "2.0" y reimporta Float; NaN/∞ como null; compatibilidad atrás («2» sigue siendo Int); cero cambios de goldens.
- Workspace: **892 → 901 tests ALL_GREEN** (verify.sh ×2). Único toque colateral: comentario del smoke tests/arquitectura.rs (sin aserciones).
- Prosa sincronizada por writer-agente (4 ficheros): cap-34 (3 ediciones: hallazgo histórico conservado + PAGADA ×68), cap-36 (6 ediciones: mínimo corregido a 1+B, ejercicio intermedio REESCRITO sobre la cifra real, deudas vivas actualizadas), epílogo (3 ediciones: deudas pagadas narradas, vivas = write skew/LIMIT/DiskStore/cola corrupta), MIGRATION-PATTERN (notas REPARADA en §39/§41 + §40 gotcha corregido).
- Verificado que cap-15 prosa/código hablan del LAYOUT del fichero (3+B páginas) — correcto, no tocado.
- Rebuild vol2 14.323 → 14.329 líneas. Commits c7582f5 / 820256a pushed.

⚠️ NOTA: los artefactos de build/ (HTML/PDF/EPUB) quedaron generados ANTES de estas reparaciones — si se publican, regenerar primero (los .md cambiaron).

**Estado al cierre**: Vol.II DONE con deudas técnicas saldadas. Obra: Vol.I DONE, Vol.II DONE (901 tests), Vol.III SKELETON.

**Próximas opciones**: (a) re-render + publicación; (b) Vol.III cap-41 (chapter-planner); (c) D2 mantenimiento periódico.

## 2026-08-26 — Sesión 43

**Asistentes**: book-orchestrator + chapter-planner + code-example-generator + chapter-writer.

**Trabajo realizado**: Cap. 41 «Modelar entidades, propiedades y relaciones» DONE — **ABRE EL VOL.III «Grafos en la era de la IA»** (1/13 caps).
- Contrato por planner-agente (`book-context/contratos/vol3/cap-41.md`, 358 líneas — primera carpeta vol3/): escalera R1-R7 formalizada (prueba de la identidad propia), query-first con las 10 preguntas vinculantes ANTES del modelo, 13 decisiones. Citas verificadas: Robinson-Webber-Eifrem O'Reilly 2013/2ª ed. jun-2015; Angles-Gutierrez CSUR 40(1) 2008 + LNCS 10000 2018; Francis et al. SIGMOD '18 pp.1433-1445; W3C RDF 1.1 2014; GQL ISO 39075:2024; openCypher CIF marcado [VERIFICAR]. Frontera dura con cap-42 (antipatrones): solo regla + mini-hub sembrado como gancho.
- Código por generator-agente (tras reintento — el primero falló por error de infraestructura «Endpoint is unavailable»): `cap41_modelado.rs` (1.668 líneas std puro, 19 tests): kb_lira_paso1() determinista (30 nodos/64 aristas: 6 Personas, 3 Org, 3 Proyectos, 12 Documentos con labels múltiples, 6 Temas; AUTHORED con order; hub Tema con 6 ABOUT), validar_modelo_kb_lira (Violacion con ids), ModeloDocsTodopropiedades con contador de lecturas, CSV roundtrip byte-a-byte, informe reproducible. Artefactos commiteados: `datasets/kb-lira/paso-1/{nodes.csv,edges.csv}`. Wiring lib.rs +2. 901→920 tests ALL_GREEN.
- HALLAZGOS del código: (1) frontera LiraQL VERIFICADA en cap18 — `scan_string` corrompe UTF-8 multi-byte («é»→«Ã©») así que filtros con títulos acentuados nunca matchean ⇒ P2/P3/P6 usan API directa (frontera declarada en prosa con honestidad); (2) P6 naive 12 lecturas (escaneo total) vs LPG 5 (solo MENTIONS entrantes); (3) P9 responde 3 temas (el camino admite d1==d2: el paper compartido trata 2 temas); (4) `clave_orden` natural (diacríticos→ASCII) porque el sort bytewise pondría «Índices» tras «Z».
- Prosa por writer-agente (368 líneas, 20 secciones + blockquote inicial de apertura del Vol.III): anécdota Graph Databases O'Reilly 2013/2015 → Cypher SIGMOD '18 → GQL ISO 2024; historia pequeña GOOD (Gyssens-Paredaens-Van Gucht, PODS 1990, pp.417-424) como primera ola de modelos de grafos, verificada en vivo (Exa; DOI 10.1145/1322432.1322433); cifras reales pegadas (tabla P1-P10).
- SUMARIO vol3 actualizado (cap-41 tras tabla-de-contenidos); rebuild vol3 137 → **505 líneas**.

**Estado al cierre**: ALL_GREEN **920 tests**. Vol.III **1/13 caps DONE** (~505 líneas ensambladas). Obra: Vol.I DONE, Vol.II DONE (renderizado), Vol.III DRAFTING. Commits 774fe13 / 477dcfc pushed.

**Próxima sesión**: cap-42 «Antipatrones: supernodos, reificación y otras trampas» (Parte I, cap 2) — refactor del hub sembrado en cap-41 con Louvain/estadísticas ya existentes; después caps 43 (temporalidad), 44 (constraints/índices). Recordatorio: regenerar build/ antes de publicar (deudas reparadas post-render).

## 2026-08-26 — Sesión 44 (parcial, bloqueada por infraestructura)

**Trabajo realizado**:
- Contrato cap-42 ESCRITO y validado por planner-agente (407 líneas, commit 4b8d4c8 pushed): kb_lira_paso2_degrado() (59 nodos/134 aristas, lote 24 papers), detector de supernodos con umbral doble (ratio ≥5× mediana del label Y share ≥25% del tipo), 3 refactors puros con InformeRefactor (A descomponer subtemas, B reificar Resena, C conferencias/temas-año), validador paso-2 por composición, REGRESIÓN OBLIGATORIA de las 10 preguntas del cap-41, ~20 tests nuevos (~940 ALL_GREEN previsto), SIN bench. Citas verificadas: Allen (Neo4j blog 19-oct-2020), GraphAcademy gdm-40, Aerospike AGS 3.0, Boylan-Toomey (26-feb-2024), Robinson et al. 2015 cap.2 p.63, Kimball 1996 (precisión star schema); Neo4j KB join hints [VERIFICAR fecha].

**BLOQUEO**: el generator-agente (`general`) devolvió RESPUESTA VACÍA 4 veces (3 fresh + 1 resume) sin tocar el árbol. El pipeline de subagentes funciona (diagnóstico con `explore` OK) — es el provider del agente `general` el que está degradado (mismo patrón que el cap-41, que se resolvió tras compactación). Trabajo NO perdido: contrato commiteado; workspace intacto 920 tests ALL_GREEN.

**Estado al cierre**: LEDGER cap-42 = PLANNED con `blocked_on: generator-infraestructura`. Próximo paso: REINTENTAR el generator del cap-42 (contrato ya listo) cuando el provider se estabilice.

## 2026-08-26 — Sesión 45

**Asistentes**: book-orchestrator + chapter-planner + 10 generator-agentes en SECUENCIA (piezas troceadas) + chapter-writer.

**Trabajo realizado**: Cap. 42 «Antipatrones: supernodos, reificación y otras trampas» DONE — Vol.III **2/13 caps**.
- **LECCIÓN DE INFRAESTRUCTURA**: el agente `general` muere con tareas largas (respuestas vacías, 4 intentos) pero responde perfecto a tareas PEQUEÑAS (diagnóstico con tarea trivial OK). Solución adoptada: TROCear la implementación en 10 piezas siguiendo el §8 del contrato (cada una compila y testea sola, wiring temporal revertido entre piezas). El cap-41 se implementó en 1 pieza; el 42 en 10 piezas encadenadas — mismo resultado, proceso más robusto. ESTE ES AHORA EL PATRÓN DEL PROYECTO para módulos grandes.
- Contrato (sesión 44): detector umbral doble (5× mediana + 25% share), 3 refactors, regresión 10 preguntas. Citas: Allen 2020, GraphAcademy gdm-40, Aerospike AGS 3.0, Boylan-Toomey 2024, Robinson et al. 2015 (cap. **3** — corregido por el writer verificando el PDF oficial de Neo4j), Kimball 1996.
- Código: `cap42_antipatrones.rs` (2.710 líneas, 28 tests) + `datasets/kb-lira/paso-2/{nodes,edges}.csv` (60+135 líneas) + wiring. 920→948 ALL_GREEN.
- Cifras reales: triplete del detector 3,0×/0,375 (paso-1 SILENCIO) → 6,0×/46% (paso-2 ALARMA) → 3,0×/0,23 (refactor SILENCIO). ⚠️ DECISIÓN DE IMPLEMENTACIÓN: mediana EXCLUYENDO al hub (si el outlier fija la línea base, paso-2 daba 4,8× y no alarmaba — documentado en el test). Migración: 10/2 nodos, 48/28 aristas, 64 lecturas (A 25, B 1, C 38). Tesis: 1 expansión 24 → 4 expansiones 12+4+4+4 (mismo total en fixture, honestidad declarada). P5 jerárquica 24, P8 40 AUTHORED (9 personas). Hallazgo validador: el base del cap-41 trata tipos nuevos como desconocidos (36 violaciones) — wrapper filtra los 6 tipos que gobierna. P8 exigió helper con target<30 (Elena/Fabio contaminaban). remove_prop vía mutación directa del campo público (precedente cap33).
- Prosa: 358 líneas, 20 secciones + blockquote; historia pequeña Enron/email forense (Robinson cap.3, verificada vía PDF oficial + InfoQ 2014).

**Estado al cierre**: ALL_GREEN **948 tests**. Vol.III **2/13 caps DONE** (~863 líneas ensambladas). Commits ba316d8 / ed45409 pushed.

**Próxima sesión**: cap-43 «El tiempo en el grafo: versionado y bitemporalidad» (Parte I cap 3) — valid-time, WAL como historia (la ronda de reseña como frontera declarada del cap-42). PATRÓN: trocear la implementación en piezas pequeñas.

## 2026-08-26 — Sesión 46

**Asistentes**: book-orchestrator + chapter-planner + 12 generator-agentes TROCEADOS + chapter-writer.

**Trabajo realizado**: Cap. 43 «El tiempo en el grafo: versionado y bitemporalidad» DONE — Vol.III **3/13 caps**.
- Contrato (473 líneas, commit a5a4aa8): valid-time como props de arista, bitemporalidad con el caso Dani, AS OF con coste, conexión WAL. Citas verificadas: Snodgrass (Morgan Kaufmann, julio 1999), Jensen & Snodgrass (IEEE TKDE 11(1):36-44, 1999), TSQL2 (Kluwer, 1995), Holme & Saramäki (Physics Reports 519(3):97-125, 2012), Kostakos (Physica A 388(6), 2009), Neo4j 3.4 (2018), GQL ISO 39075:2024 (abril 2024).
- Código por 12 piezas troceadas (patrón de la sesión 45 consolidado — funcionó de principio a fin SIN ningún fallo de agente): `cap43_temporalidad.rs` (1.853 líneas, 23 tests) + datasets/kb-lira/paso-3/{nodes,edges,historico}.csv + wiring. 948→971 ALL_GREEN.
- Cifras reales: 68 nodos/158 aristas/10 MEMBER_OF; tabla AS OF 2026/2024/2023/2020/2019 (28/28/28/27/27 lecturas); tesis «presente = AS OF mismo barrido» (28=28); delta arista vencida 21→20 get_edge + candidatas 4→3 (la tesis 13→14 del contrato no cuadraba con el ledger real — pineado el real con comentario); gancho reseña Some(7)→Some(8) desde 2025 (CONTRARRESTA{desde_anio}); caso Dani (2019 creído vs 2021 cierto); WAL cap-28 real demuestra la frontera (replay reconstruye lo ESCRITO, no lo CORREGIDO).
- Sorpresas cazadas: nombres reales de organizaciones («Universidad de Lira» etc.); la ronda 2 solo reina desde que emite CONTRARRESTA (gotcha resuelto); P4 es la ÚNICA discrepancia real (Beto→GrafoLuna con tiempo) — es la lección, filtrada en el subgrafo paso-1 con helper nuevo `solo_afiliaciones_paso1`.
- Prosa (358 líneas, 20 secciones): anécdota Holme & Saramäki (aristas que se encienden y apagan); historia pequeña = la reseña del 3 de marzo como cierre del arco cap-42→43; grano anual declarado dos veces.
- SUMARIO vol3 + rebuild 863 → **1221 líneas**. Commits 2818156 / 3165db2.

**Estado al cierre**: ALL_GREEN **971 tests**. Vol.III **3/13 caps DONE** (~1.221 líneas ensambladas). Parte I 3/5.

**Próxima sesión**: cap-44 «Constraints e índices» (Parte I cap 4) — el validador paso-3 siembra las reglas; el AS OF sin índice es la deuda que el índice paga. Después cap-45 ingesta (transaction-time automático).

## 2026-08-26 — Sesión 47

**Asistentes**: book-orchestrator + chapter-planner + 13 generator-agentes TROCEADOS + chapter-writer.

**Trabajo realizado**: Cap. 44 «Esquema, constraints e índices en property graphs» DONE — Vol.III **4/13 caps**, Parte I 4/5.
- Contrato (444 líneas, commit 193cc43): Esquema declarativo con 6 variantes de regla (Extremos any-of, Existencia, Tipo, Unicidad, SinSolape, IntervaloValido), ORCID único, índices lógicos con lección honesta, selectividad como juez. Citas verificadas: Schwartz «Schemaless Databases Don't Exist» (SolarWinds blog, 24-feb-2015), GQL ISO/IEC 39075:2024 (abril 2024, DDL Parte 1), Neo4j 5.x (blog 3-nov-2023).
- Código por 13 piezas troceadas (patrón consolidado, 0 fallos): `cap44_esquema.rs` (2.551 líneas, 22 tests) + datasets/kb-lira/paso-4/{nodes,edges,esquema}.csv + wiring. 971→993 ALL_GREEN.
- HALLAZGOS: (1) subsunción INCOMPLETA descubierta y corregida — el esquema no cubría las reglas temporales del validador paso-3 → variante IntervaloValido añadida; subsunción demostrada con ids idénticos {16,21,52}; (2) MENTIONS polimórfico → Extremos generalizado a hasta_labels: Vec (any-of); (3) la lección del índice con números REALES: global reescrita 12→3, AS OF persona-céntrica 28=28 (el índice simple no casa con el patrón; +10 mantenimiento = 38), IndicePorLabel 28→22 (amortiza desde n=27); las cifras 168→18/28→16 del contrato se sustituyeron por ledgers reales con discrepancia documentada; (4) el catálogo DELATA un duplicado residual: temas 26 y 61 comparten «memoria de agentes» (selectividad 8/9) — historia pequeña del capítulo; (5) Value no deriva Hash → unicidad con clave textual canónica.
- Prosa (353 líneas, 20 secciones): anécdota Schwartz «schemaless mentiroso»; informe pineado byte a byte; ganchos cobrados (SinSolape = «¿quién garantiza que dos afiliaciones no se solapen?»; el índice = «¿quién construye el que abarata AS OF?»).
- SUMARIO vol3 + rebuild 1221 → **1574 líneas**. Commits 061df3a / 7fa8061.

**Estado al cierre**: ALL_GREEN **993 tests**. Vol.III **4/13 caps DONE** (~1.574 líneas ensambladas). Parte I 4/5.

**Próxima sesión**: cap-45 «Ingesta: del archivo al grafo» (Parte I cap 5 — CIERRA LA PARTE I) — pipeline CSV/JSONL con dedup, validación al importar (write-time, el esquema del cap-44 aplicado al vuelo), transaction-time automático (el Historico del cap-43 alimentado por ingesta).
