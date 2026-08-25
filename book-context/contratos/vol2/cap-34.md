# CONTRATO DE CAPÍTULO — Vol.II Cap. 34: Benchmarks y perfilado

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla: todo lo que el
> capítulo MIDE ya existe y está verde — el Volcano del cap. 20 (`PhysicalOperator`
> con `rows_produced()`/`collect_metrics()`, `ExecMetrics { per_operator:
> Vec<(&'static str, u64)>, rows_returned }` expuesto SOLO por `Executor::metrics()` —
> ni `run()` ni `Query::execute()` lo revelan: hay que desenrollar el pipeline como
> hace `pipeline_con_detalle` de la CLI, cap. 31, flags `--plan/--stats`),
> `encode_value`/`decode_value` (cap. 9), `FilePager` real sobre fichero (cap. 12),
> `BufferPool` Clock/LRU con `Metrics { page_reads, page_writes, buffer_hits,
> buffer_misses, evictions }` + `hit_ratio()` vía `metrics()` (cap. 13), `Csr` +
> `PersistentCsr` (cap. 14), `HashIndex::get(&mut self, key)` y
> `BPlusTree::{get, range_scan}` sobre `BufferPool<P>` (cap. 15),
> `dijkstra`/`dijkstra_path` (cap. 22), `page_rank(store, damping, max_iterations,
> tol)` (cap. 24), `ProyeccionPonderada::proyectar` + `dijkstra_proyeccion` +
> `ContandoStore` como «voltímetro» (cap. 26). Código NUEVO previsto: módulo
> `cap34_benchmarks.rs` en `vol2-liradb` (PRNG `Xorshift64Star` ~15 líneas +
> `dataset_referencia(seed) -> DatasetReferencia` + harness de percentiles), el PRIMER
> directorio `benches/` del workspace (`crates/vol2-liradb/benches/{bench_micro.rs,
> bench_consultas.rs}`, dos `[[bench]]` con `harness = false`), `criterion = "0.7"`
> como ÚNICA dev-dependency nueva (hoy solo `tempfile` y `proptest`, SIN `[[bench]]`)
> y `[profile.bench] debug = true` en la raíz del workspace. Estado verificado
> 2026-08-25: **809 tests** en verde; toolchain pinneada 1.96.0. Implicación
> verificada de `verify.sh`: check/clippy con `--all-targets` COMPILAN los benches y
> `cargo test` los CONSTRUYE pero NO los EJECUTA por defecto (Cargo Book: bench =
> «build only») — ALL_GREEN sigue rápida. Hallazgo clave: NO existe ningún
> `GraphStore` respaldado por disco end-to-end — los únicos `impl GraphStore` son
> `MemoryStore` (producción) y stores de prueba; `PersistentCsr`/`HashIndex`/
> `BPlusTree` viven sobre `BufferPool<FilePager>` PERO no exponen el puerto, y el
> executor corre sobre `&dyn GraphStore`. Decisiones irán a `MIGRATION-PATTERN.md`
> §39. Pregunta crítica del CORPUS (`vol-II-cap-34`): «Dataset de referencia (100k
> personas / 500k relaciones)». Cap. 4 de la Parte VII. Recoge el gancho del cap. 33
> (§5.12: «criterion deliberadamente ausente — primero correcto, luego rápido»).
> Gancho saliente: cap. 35 observabilidad («ahora que puedes MEDIR, hazlo VISIBLE
> en producción»).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el puerto hexagonal `GraphStore` (cap. 8) con
  `MemoryStore` como única implementación de producción; encoding (cap. 9);
  pager/páginas/BufferPool/CSR/índices (caps. 11-15) con sus métricas internas;
  el pipeline completo parse → lower → optimize → Volcano (caps. 17-21) con
  `ExecMetrics` por operador; algoritmos (caps. 22-26); WAL/ARIES/MVCC (27-30);
  CLI e import/export (31-32); la TORRE DE PRUEBAS del cap. 33 con **809 tests**.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «mi código es rápido porque me lo parece / cronometré con `Instant` en debug»
  — no: el compilador optimiza (y ELIMINA trabajo no usado — `black_box`), debug y
  release no miden lo mismo, y una medida sin repeticiones es una anécdota.
  (2) «la media resume el rendimiento» — no: la COLA manda (p99/p99.9: una consulta
  lenta cada cien arruina la experiencia); medir solo media oculta la distribución.
  Y «un número suelto basta» tampoco: throughput y latencia son preguntas distintas,
  y sin hardware/metodología declarados el número no es reproducible.
  (3) «warm/cold cache = reiniciar el proceso» — no: la caché del SO persiste tras
  reiniciar; además en LiraDB HOY no hay store-en-disco tras el puerto (verificado),
  así que el frío/caliente de consultas completas AÚN NO EXISTE — fingirlo está
  prohibido.
  (4) «comparar con Neo4j en un capítulo demuestra quién es mejor» — comparaciones
  cross-sistema exigen metodología completa (mismo hardware, tuning comparable,
  datasets representativos) que no cabe aquí; objetivo honesto del brief: medir la
  EVOLUCIÓN de LiraDB CONTRA SÍ MISMA (baselines).
- **Pregunta crítica que el capítulo tiene que responder**: «¿cuál es el DATASET DE
  REFERENCIA (100k personas / 500k relaciones) y cómo se convierte en instrumento de
  medición reproducible?» Respuesta: `dataset_referencia(seed)` — generador
  DETERMINISTA (PRNG xorshift64* a mano, seed fija pública), distribución heavy-tail
  ligera (mayoría grados bajos + hubs), 10 etiquetas y esquema de 20 propiedades con
  nodos dispersos (sparse), presupuesto <10 s, reutilizado por benches y tests, con
  contrato testeado (cuentas exactas + determinismo). Sin dataset determinista no hay
  baseline comparable.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `cap34_benchmarks.rs`: `Xorshift64Star` (PRNG a mano, ~15 líneas, seed≠0) + constantes `NODOS_DATASET=100_000`, `ARISTAS_DATASET=500_000`, `ETIQUETAS_DATASET=10`, `PROPS_ESQUEMA=20`, `SEMILLA_REFERENCIA` | `cargo test -p vol2-liradb --lib cap34`: tesis `prng_determinista_misma_semilla_misma_secuencia`, `prng_rechaza_semilla_cero` |
| `dataset_referencia(seed) -> DatasetReferencia { pub store: MemoryStore, pub hubs: Vec<NodeId>, pub emails_muestra: Vec<String> }` (metadatos para consultas deterministas) | `dataset_cuenta_exacta_100k_nodos_500k_aristas`, `dataset_determinista_misma_semilla_grafo_identico`, `dataset_hubs_concentran_grado_maximo`, `dataset_esquema_10_etiquetas_20_claves_email_unico` |
| PRIMER directorio `benches/` del workspace: `benches/bench_micro.rs` y `benches/bench_consultas.rs` (`[[bench]]`, `name = "bench_micro"` / `"bench_consultas"`, `harness = false`) — convención `bench_*` | Compilación automática en `./scripts/verify.sh` (check+clippy `--all-targets`; `cargo test` construye pero NO ejecuta); ejecución MANUAL: `cargo bench -p vol2-liradb --bench bench_micro` |
| Microbenchmarks de componentes: encode/decode Value (cap. 9), seek `HashIndex`/`BPlusTree` (cap. 15), iteración CSR vs `out_edges` directo (caps. 14/8), `ProyeccionPonderada::proyectar`, `dijkstra`, `page_rank` sobre el dataset (caps. 22/24/26) | `cargo bench --bench bench_micro` produce grupos estables; cada grupo declara `Throughput::Elements(...)` (valores/aristas procesados) |
| Benchmarks de consulta end-to-end: 5 consultas representativas sobre el dataset — Q1 point lookup por igualdad (ruta IndexSeek, cap. 21), Q2 expansión 1-hop desde un hub, Q3 scan+filtro sin índice, Q4 proyección amplia con `LIMIT`, Q5 camino mínimo vía API `dijkstra_path` (LiraQL aún no tiene shortest-path: delimitado) | `cargo bench --bench bench_consultas`; tesis de tests: `consultas_reportan_exec_metrics` (para Q1..Q5, `ExecMetrics.per_operator` no vacío y `rows_returned > 0`) |
| Contadores internos VISIBLES junto a tiempos: pipeline desenrollado (`parse`→`lower`→`optimize`→`Executor`) capturando `executor.metrics()` — SIN tracing (eso es cap. 35: aquí solo se REPORTAN) | mismo test `consultas_reportan_exec_metrics`; la prosa pega tabla tiempo + filas/operador REAL |
| Warm/cold cache HONESTO a nivel COMPONENTE: dos pasadas `get_page` sobre `BufferPool<FilePager>` (tempfile) — primera (frío: misses) vs segunda (caliente: hits) | test `buffer_pool_frio_caliente_hit_ratio_sube` (usa `Metrics::hit_ratio` del cap. 13; `tempfile` ya es dev-dep) + bench correspondiente; FRONTERA DOCUMENTADA: cold-cache de CONSULTAS espera a un DiskStore tras el puerto (no existe hoy — verificado) |
| Harness de PERCENTILES propio (std): `percentiles(muestras: &[u64]) -> Percentiles { p50, p90, p99, p999 }` sobre muestras crudas | `percentiles_monotonos_y_completos` (orden y cobertura; jamás valores de pared — CI determinista) |
| Comparaciones honestas contra sí misma: baselines de criterion (`--save-baseline` / `--baseline`) | comandos documentados en prosa; PROHIBIDA la comparación sensacionalista con Neo4j (brief, línea 1461) |
| Flamegraphs DOCUMENTADOS, FUERA de `verify.sh` (perf/Linux + `[profile.bench] debug = true` en raíz) | prosa N.10 + `MIGRATION-PATTERN.md` §39 con instrucciones reales y ejemplo de lectura de `flamegraph.svg`; NINGÚN paso perf en el pipeline |
| Sección de TRAMPAS con código que las comete a propósito (setup medido dentro del loop, falta de `black_box`) y su diagnóstico | prosa N.12 + ejercicio de análisis (predecir qué cifra infla cada trampa antes de correrlo) |
| ALL_GREEN workspace + tablas reales | `./scripts/verify.sh` → `ALL_GREEN` (809 + ~7 tests nuevos); prosa pega salidas REALES de `cargo bench` con hardware declarado y metodología para repetirlas |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Dataset de referencia (100k personas / 500k relaciones).» — el capítulo
la convierte en **respuesta en nueve pasos** (los nueve puntos del brief):

1. **Microbenchmarks** → `bench_micro.rs`: operaciones de COMPONENTE aisladas con
   `Throughput::Elements` — encode/decode `Value` (cap. 9), seek de índices (cap. 15),
   iteración CSR vs adyacencia directa (caps. 14/8), proyección y algoritmos
   (caps. 26/22/24). Localizan QUÉ componente mueve la aguja.
2. **Benchmarks de consulta** → `bench_consultas.rs`: el pipeline COMPLETO
   `parse→lower→optimize→Volcano` sobre el dataset (Q1-Q5): ms por consulta real.
3. **Warm/cold cache** → HONESTO: a nivel componente (BufferPool frío vs caliente
   sobre `FilePager` real, hit-ratio del cap. 13). La frontera se DELIMITA: no existe
   DiskStore detrás del puerto (verificado), luego el cold-cache de consultas llega
   cuando exista. PROHIBIDO fingir.
4. **Throughput** → `criterion::Throughput::Elements(n)` por grupo: aristas iteradas,
   nodos proyectados, filas devueltas — el DENOMINADOR declarado convierte segundos
   en elementos/s comparables.
5. **Latencia** → tiempo por operación (criterion) + harness propio de percentiles
   para la historia de la COLA que la media esconde.
6. **Percentiles** → p50/p90/p99/p999 calculados A MANO sobre muestras crudas:
   criterion expone media/mediana/desv., no percentiles arbitrarios; el lenguaje de
   los SLO reales es la cola.
7. **Flamegraphs** → documentados (cargo-flamegraph + perf + `debug = true`),
   con ejemplo de lectura: los bloques anchos SON el perfil; nunca cableados a CI.
8. **Contadores internos** → `ExecMetrics` (filas reales por operador, cap. 20)
   reportadas JUNTO a los tiempos: el número sin el TRABAJO que lo causó es mitad de
   la verdad. Semilla del cap. 35 — aquí solo se REPORTAN, no se instrumenta.
9. **Comparaciones honestas** → baselines de criterion contra sí misma
   (`--save-baseline cap34-inicial`), trampas enseñadas con ejemplos que MIENTEN a
   propósito, y la línea roja del brief: nada de Neo4j sensacionalista.

El dataset de referencia es el SUELO común de los nueve: mismo grafo determinista ⇒
diferencias entre runs = ruido de medición, no entrada distinta.

---

## 4. La arquitectura: una torre de instrumentos, cada uno para una pregunta

Modelo mental único: **cada instrumento responde UNA pregunta distinta**, y usar el
nivel equivocado produce respuestas equivocadas. Es la torre de riesgos del cap. 33
girada 90°: allí cada piso atacaba un riesgo; aquí cada piso EXPLICA un aspecto del
rendimiento:

```
        ¿QUÉ PREGUNTA RESPONDE CADA INSTRUMENTO?
 ▲  flamegraph     ¿dónde se va el CPU POR DENTRO?          (perfilado, offline)
 │  contadores     ¿cuánto TRABAJO hizo cada operador?      (ExecMetrics, cap. 20)
 │  bench consulta ¿cuánto tarda una consulta END-TO-END?   (criterion + pipeline)
 │  microbench     ¿cuánto tarda UNA operación de pieza?    (encode, seek, CSR…)
 └──────────────────────────────────────────────────────────────────────────────▶
   localiza QUÉ componente falló                        explica POR QUÉ falla
```

Y debajo de la torre, el CALIBRADO del instrumento (lo que separa un experimento de
una opinión): dataset DETERMINISTA (misma entrada siempre), `black_box` (que el
compilador no borre el trabajo), warm-up, repeticiones con estadística (criterion) y
DECLARACIÓN de hardware. Un número sin calibrado es INCOMPARABLE.

Momento ¡ajá! perseguido: «una sola cifra puede mentir; throughput + latencia +
contadores + flamegraph JUNTOS cuentan la verdad — y todo número sin metodología es
marketing». Hallazgo honesto del capítulo: LiraDB aún no puede medir cold-cache de
consultas porque su store de producción vive en RAM — y decirlo ES parte del tema.

```text
Lo que SÍ se mide hoy:   piezas sobre BufferPool/FilePager (frío/caliente, hit ratio)
                         consultas completas sobre MemoryStore (dataset 100k/500k)
Lo que AÚN NO:           cold-cache de consultas (espera DiskStore tras el puerto)
```

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Los benchmarks viven en el PRIMER `benches/` del workspace (`crates/vol2-liradb/benches/`, `[[bench]]` `harness = false`), no en binarios sueltos | Integración nativa de Cargo: `verify.sh` los COMPILA (check/clippy `--all-targets`) y `cargo test` los construye sin EJECUTARLOS — código verde sin ralentizar la puerta | Binario propio cronometrado a mano: sin estadística, sin warm-up, sin baselines; reimplementar criterion mal | Cargo Book (tabla de selección de targets: bench = build-only en `cargo test`); criterion Book; stack-profile.yml |
| 2 | «Primero a mano» aplicado donde criterion no llega: PRNG `Xorshift64Star` (~15 líneas) y harness de percentiles escritos con std | El crate principal sigue dependency-free; rand/uuid meterían deps de RUNTIME para generar datos de TEST | `rand`/`uuid` como deps: peso y superficie de supply-chain para algo trivial | CONVENTIONS §4 (regla «primero a mano»); Marsaglia, xorshift (JSS 2003); misma regla que caps. 32/33 (parsers a mano, goldens a mano) |
| 3 | Dataset DETERMINISTA con seed fija pública (`SEMILLA_REFERENCIA`) | Reproducibilidad = requisito de benchmark: si la entrada cambia entre runs, comparar baselines es comparar peras con manzanas; además tests de cuenta exacta exigen determinismo | Dataset aleatorio por ejecución: números incomparables y tests no repetibles | Mytkowicz et al., «Producing Wrong Data…», ASPLOS 2009 (control de variables); criterion Book (baselines) |
| 4 | Distribución HEAVY-TAIL ligera: mayoría de nodos con grado bajo + ~100 hubs que concentran una parte de las 500k aristas | Los grafos sociales reales son libre-escala; ejercita a la vez point lookups triviales y expansiones de hub caras — lo honesto para un MOTOR DE GRAFOS | Uniforme: nadie despliega grafos uniformes y OCULTARÍA el coste de los hubs (la mentira cómoda) | Barabási & Albert, «Emergence of Scaling in Random Networks», Science 286 (1999) |
| 5 | «20 propiedades» = 20 claves en el ESQUEMA con nodos SPARSE (subconjunto por nodo); `email` único alimenta la ruta IndexSeek del cap. 21 | Property graphs reales son sparse (props opcionales); 20 props × 100k nodos inflaría memoria ×N sin realismo; el email único da a Q1 su ruta de índice | 20 props OBLIGATORIAS por nodo: memoria desproporcionada y patrón irreal | Modelo property graph del cap. 7 (props opcionales por elemento); optimizador cap. 21 (`equality_lookup`) |
| 6 | Dos ficheros bench según la taxonomía del brief: `bench_micro.rs` (piezas) y `bench_consultas.rs` (pipeline), convención `bench_*` | El brief divide exactamente ahí; separar familias evita un monolito y permite correrlas por separado | Un único bench gigante: ciclos larguísimos y mezcla de niveles de abstracción | Brief cap. 34 (líneas 1442-1443); criterion Book (grupos) |
| 7 | Warm/cold a nivel COMPONENTE (BufferPool+FilePager, hit-ratio) y frontera DOCUMENTADA para consultas | Verificado: NO hay `GraphStore` en disco (solo `MemoryStore` + stores de test); simularlo exigiría escribir el DiskStore que no existe | Fingir cold-cache de consultas con trucos de proceso: FALSA medición — prohibición absoluta del capítulo | Código real caps. 8/12/13/14 (verificado 2026-08-25); honestidad hexagonal cap. 33 (§5.4) |
| 8 | Percentiles p50/p90/p99/p999 A MANO sobre muestras crudas; criterion solo aporta media/mediana/desv. | El lenguaje de los SLO es la cola; la media esconde la distribución (una lentitud de cada cien no se ve en la media) | Quedarse con las estadísticas de criterion: no hay p99 y la cola desaparecería del libro | Criterion Book (salidas estadísticas); G. Tene, «How NOT to Measure Latency» (2015) |
| 9 | Throughput SIEMPRE con `Throughput::Elements(...)` declarado por grupo | Sin denominador explícito, «segundos» no dicen nada; con él, criterion imprime elem/s comparable entre runs y baselines | Tiempo crudo sin unidades de trabajo: cifras no comparables entre tamaños de dataset | Criterion Book, sección Throughput |
| 10 | Flamegraphs DOCUMENTADOS fuera de `verify.sh`; `[profile.bench] debug = true` en la raíz | cargo-flamegraph exige perf/Linux/símbolos — herramienta externa como cargo-fuzz en cap. 33; `debug=true` da símbolos legibles al coste de binarios mayores | Cablear flamegraphs al pipeline: rompe reproducibilidad multiplataforma del ALL_GREEN | cargo-flamegraph README; Gregg, «The Flame Graph», ACM Queue 14(2) 2016; política toolchain ADR-002 |
| 11 | Comparación ÚNICAMENTE contra sí misma: `cargo bench -- --save-baseline cap34-inicial` y `--baseline` después; Neo4j PROHIBIDO | El objetivo del brief es la evolución de LiraDB; comparar motores exige metodología (hardware, tuning, licencias) fuera de alcance y alimenta titulares huecos | Tabla LiraDB vs Neo4j «de regalo»: sensacionalismo sin control experimental | Brief línea 1461; SPEC CPU Run Rules (prohibición de optimización específica del benchmark); Hennessy & Patterson, CA:AQA §1.8 |
| 12 | Trampas ENSEÑADAS con código que miente a propósito (setup dentro del loop, sin `black_box`) y su diagnóstico posterior | Se aprende a DESCONFÍAR viendo el fallo, no leyendo advertencias; `black_box` es LA defensa contra el compilador que elimina trabajo muerto | Solo listar trampas en prosa: conocimiento sin evidencia ejecutable | Criterion Book (`black_box`, `iter_batched`); Mytkowicz et al. 2009 (el ENTORNO silencioso invierte resultados) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: Reinhold Weicker crea Dhrystone
   (1984) y acaba DOCUMENTANDO cómo su propio benchmark fue manipulado: compiladores
   que reconocían el código, fabricantes afinando casos concretos; su respuesta fue
   la versión 2 con reglas (1988) y la industria maduró hacia SPEC. Pregunta
   enmarcada: ¿cómo medir HONESTAMENTE a LiraDB?
2. **N.1-N.2 Objetivo/Problema**: «tienes 809 tests verdes y ni idea de cuánto tarda
   un expand de 1-hop sobre 500k aristas». Qué NO te dice la suite de tests.
3. **N.3 Modelo mental**: la torre de instrumentos (§4) + tabla
   pregunta↔instrumento↔artefacto de LiraDB; el calibrado (determinismo, black_box,
   warm-up, repeticiones, hardware declarado).
4. **N.4 Primera solución**: cronómetro a mano con `Instant::now()` alrededor de un
   bucle (la versión ingenua de todo el mundo) y sus tres pecados: sin repeticiones,
   sin control del compilador, sin percentiles.
5. **N.5 Sus límites**: ruido, outliers, trabajo eliminado, setup contabilizado,
   números irrepetibles, media mentirosa.
6. **N.6 Solución evolucionada**: PRNG a mano + `dataset_referencia` (presentación
   del dataset: distribución, presupuesto, metadatos de consulta) + criterion con
   `[[bench]]` + percentiles propios + frío/caliente de componente + contadores
   junto a tiempos + baselines.
7. **N.7 Código completo ejecutable**: `cap34_benchmarks.rs` y los dos `benches/`
   referenciados por `include::` (nunca duplicados) + el `[profile.bench]`.
8. **N.8 Prueba de fuego**: `cargo bench --bench bench_micro --bench bench_consultas`
   con salidas REALES pegadas; guardar baseline, tocar UNA cosa (p.ej. política del
   BufferPool) y VER la detección con `--baseline`; el flamegraph que señala el
   hotspot esperado (expand/iteración).
9. **N.9 Qué hemos sacrificado**: sin cold-cache de consultas (sin DiskStore),
   flamegraphs fuera de CI, sin comparativas cross-motor, sin profiling de memoria
   (heaptrack fuera), percentiles limitados al harness propio.
10. **N.10 Cómo lo hace una BBDD real + retos**: PostgreSQL (`pg_stat_statements`,
    `EXPLAIN ANALYZE`), SQLite (`sqlite3_status`), benchmark protocol de
    Hennessy-Patterson, flamegraphs en producción (Gregg/Netflix); retos esencial
    (benchmark nuevo para `range_scan` siguiendo `bench_*`), intermedio (diagnosticar
    el bench envenenado: predecir ANTES qué cifra infla), experto (p99.9 de Q3 con
    el harness propio + informe contra `--baseline`).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla /
    30 segundos / Una historia pequeña / Mini-diálogo de guardia nocturna (el p99 de
    las consultas se dispara a las 3 a.m. y la media no se entera). Retrieval
    practice: reproducir DE MEMORIA las cuatro preguntas de la torre. Interleaving:
    cada ejercicio toca ≥2 capítulos (15+34, 13+34, 20+21+34). Glosario nuevo:
    microbenchmark, benchmark de consulta, throughput, latencia, percentil,
    cola (tail), warm-up, baseline, flamegraph, black box, hit ratio, heavy-tail.
12. **Gancho de cierre (preguntas abiertas)**: ahora que puedes MEDIR, ¿cómo haces
    VISIBLE la medición EN PRODUCCIÓN? Cap. 35: `queries_total`, spans query→plan→
    operador→page fetch, y `liradb query --profile`.

---

## 7. Estilo y tono (consistencia con caps. 27-33)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez; salidas REALES de `cargo bench` pegadas (hardware, SO y
  toolchain declarados), nunca reconstruidas de memoria; metodología reproducible.
- **Diagramas**: la torre de instrumentos (§4) y el bloque «lo que sí / lo que aún
  no» de la honestidad warm/cold; 1 tabla pregunta↔instrumento↔artefacto.
- **Spacing** (conceptos viejos que se EJERCITAN): puerto hexagonal (cap. 8),
  encoding (cap. 9), FilePager (cap. 12), métricas del BufferPool (cap. 13), CSR
  (cap. 14), índices (cap. 15), ExecMetrics/Volcano (cap. 20), IndexSeek del
  optimizador (cap. 21), Dijkstra/PageRank/proyección (caps. 22/24/26), CLI
  `--plan/--stats` (cap. 31) y la torre de pruebas como espejo estructural (cap. 33).
- **Interleaving**: el reto esencial mezcla 15+34; el intermedio mezcla 9+20+34
  (detectar el trabajo eliminado y el setup contabilizado); el experto mezcla 22+34
  (percentiles de un camino mínimo); el frío/caliente mezcla 12+13+34.
- **Dificultad asimétrica**: una idea nueva por sección (calibrado → dataset →
  micro → consulta → cola → frío/caliente → perfil → honestidad); los ejercicios
  exigen PREDECIR (¿qué cifra infla esta trampa?) y recordar sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap34` (milisegundos) y
  `./scripts/verify.sh` ALL_GREEN como puerta; `cargo bench` como acto EXPLÍCITO del
  lector — los benches compilan gratis en la puerta pero se ejecutan cuando TÚ
  decides. Nunca «confía en mí».
- **Anécdota (única, verificada)**: Weicker y Dhrystone (1984/1988/1990) — el
  benchmark secuestrado por sus usuarios y la respuesta del autor: reglas.
  Apoyo: Mytkowicz et al. (ASPLOS 2009); Marsaglia (xorshift, JSS 2003); criterion
  Book (bheisler.github.io); Barabási & Albert (Science 1999); Tene («How NOT to
  Measure Latency», 2015); Gregg («The Flame Graph», ACM Queue 2016; Systems
  Performance, 2ª ed. 2020); SPEC CPU Run Rules; Hennessy & Patterson (CA:AQA,
  6ª ed.); Cargo Book (selección de targets); PostgreSQL `pg_stat_statements`.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5).
- [x] Escenario de fallo visible, no solo happy path: bench envenenado que MIENTE
  (setup medido, sin black_box), media que esconde el p99, frío/caliente infingible
  (frontera documentada), baseline que detecta una regresión real.
- [x] Código ejecutable en workspace citado por nombre (IMPLEMENTADO 2026-08-25:
  `cap34_benchmarks.rs` (1.036 líneas, PRNG Xorshift64Star + dataset_referencia
  100k/500k + percentiles a mano), `benches/{bench_micro,bench_consultas}.rs`,
  `[profile.bench] debug=true` en raíz; **822 tests** = 809 + 13 nuevos,
  verify.sh ALL_GREEN; `cargo bench` EJECUTADO con números reales — Xeon
  E5-2682 v4 @2.50GHz. Divergencias documentadas por el implementador:
  criterion resuelve **0.7** (no la 0.5 prevista); Q1/Q2 miden ejecución sobre
  el plan semi-ligado SIN paso optimize porque se descubrió que
  `Catalog::collect` es O(valores_distintos²) — ~224 s con los 100k emails
  únicos frente a 281 ms para construir TODO el grafo (hallazgo estrella,
  material de N.8/N.9 y deuda del optimizador); Q4 sin LIMIT (la gramática
  LiraQL no lo tiene aún). Cifras clave: hub vs grado bajo ×794; CSR vs puerto
  ×16; buffer pool frío/caliente ×142 (hit_ratio 0.000→1.000); trampa mide
  ×3,5 de más.
- [x] Misconcepciones corregidas explícitamente (§1: cuatro, de «cronómetro a mano
  basta» a «comparar con Neo4j demuestra algo»).
- [x] Ejercicios con solución verificable diseñados (retos N.10 con nombres previstos).
- [x] ≥1 ejercicio de retrieval (torre de instrumentos de memoria, sin mirar) y
  spacing planificado (caps. 8/9/12/13/14/15/20/21/22/24/26/31/33 tocados; §7).
- [x] Responde la pregunta crítica del CORPUS (dataset determinista 100k/500k/10
  etiquetas/20 props) y recoge el gancho del cap. 33 («primero correcto, luego
  rápido» — criterion entra AHORA, §5.1).
- [x] Anécdota única verificada con fuentes primarias (Weicker 1988/1990; SPEC run
  rules) — candidata descartada (Gregg/flamegraphs) reservada para N.10.
- [x] Alcance de código nuevo acotado y honesto (un módulo + un directorio benches/
  + una dev-dependency + un perfil; cero cambios en caps. 7-33; §5.1/5.7/5.10).
- [x] Gancho saliente al cap. 35 fijado (observabilidad: de MEDIR a VER, §6.12).
