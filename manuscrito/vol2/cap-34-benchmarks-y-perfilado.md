# Capítulo 34 — Benchmarks y perfilado

> *«El capítulo anterior probó que LiraDB es correcta. Este pregunta la otra mitad: ¿cuánto tarda? Y aprende antes a no mentirse: un número sin metodología declarada no es una medida — es marketing.»*

## 34.0 La anécdota de la esquina

En 1984, Reinhold Weicker —entonces en Siemens— publicó **Dhrystone** (ACM SIGPLAN Notices, junio de 1984), un benchmark sintético de programación de sistemas cuyo nombre era un guiño al veterano Whetstone. Se convirtió enseguida en la moneda corriente del marketing de workstations: «nuestra máquina hace X dhrystones por segundo». Y entonces su propio autor tuvo que documentar cómo su medidor había sido secuestrado. En «Dhrystone Benchmark: Rationale for Version 2 and Measurement Rules» (SIGPLAN Notices 23(8), 1988), Weicker describe el problema sin anestesia: compiladores que RECONOCÍAN los kernels y eliminaban o plegaban partes enteras, fabricantes afinando el compilador para ese código concreto — optimizar para el medidor, no para el usuario. Su respuesta no fue un parche: fueron **reglas**. La versión 2 congelaba el código fuente, exigía declarar las opciones de compilación y prohibía tocar lo que se mide. La industria maduró hacia SPEC y sus *run rules* auditadas — y Weicker mismo contó la lección completa en «An Overview of Common Benchmarks» (IEEE Computer, diciembre de 1990).

¿Y hoy ya no pasa? Mytkowicz et al. midieron lo contrario en «Producing Wrong Data Without Doing Anything Obviously Wrong!» (ASPLOS 2009): algo tan inocente como una variable de entorno o el alignment del ejecutable puede INVERTIR qué implementación parece más rápida. El entorno mide contigo, silenciosamente. Este capítulo construye el aparato de medición de LiraDB con esa lección clavada: primero las reglas — dataset determinista, hardware declarado, trabajo protegido del compilador — y después, los números.

## 34.1 Objetivo

El capítulo anterior cerró con una promesa deliberadamente incumplida: criterion quedó FUERA de la torre de pruebas («criterion deliberadamente ausente», §33.9) porque «primero correcto, luego rápido». Ahora llega el «luego». Al terminar tendrás:

1. **El dataset de referencia**: `cap34_benchmarks.rs` (1.036 líneas en `crates/vol2-liradb/src/`) con el PRNG `Xorshift64Star` escrito a mano, `dataset_referencia(seed)` —100.000 nodos / 500.000 aristas / 10 etiquetas / 20 claves de esquema, determinista bajo `SEMILLA_REFERENCIA`— y el harness de percentiles `p50/p90/p99/p999`.
2. **El PRIMER directorio `benches/` del workspace**: `benches/bench_micro.rs` (411 líneas, componentes) y `benches/bench_consultas.rs` (138 líneas, pipeline completo), dos `[[bench]]` con `harness = false`, y `criterion = "0.7"` como única dev-dependency nueva (resuelve 0.7; el plan decía 0.5 — así es esto cuando se mide).
3. **13 tests nuevos** sobre el módulo: 809 + 13 = **822**, ALL_GREEN.
4. Una tesis que lo vertebra: **la torre de instrumentos**. Cada instrumento responde UNA pregunta distinta del rendimiento, y usar el nivel equivocado produce respuestas equivocadas — como la torre de riesgos del cap. 33 girada 90 grados.

## 34.2 Problema

En realidad, ya tienes 809 tests verdes —los trece de este capítulo vienen ahora—. Y aun así no podrías responder la pregunta más simple del mundo: ¿cuánto tarda un expand de un salto sobre 500.000 aristas? Tu suite responde «¿es CORRECTO?» con una contundencia que nadie discute, y calla absolutamente sobre «¿es RÁPIDO, cuán VARIABLE es, y desde CUÁNDO empeoró?». Peor: tu intuición sobre rendimiento está armada con cuatro trampas clásicas que este capítulo va a desactivar una a una:

1. **«Cronometré con `Instant` y me dio rápido.»** En build debug, sin repeticiones, sin proteger el resultado del compilador: eso no es una medición, es una impresión.
2. **«La media resume el rendimiento.»** No: la media esconde la distribución. Una consulta lenta de cada cien arruina la experiencia de alguien real y ni aparece en la media.
3. **«Frío vs. caliente: reinicio el proceso.»** La caché de páginas del SO persiste tras el reinicio. Y hay algo peor: en LiraDB HOY no existe ningún store-en-disco detrás del puerto `GraphStore` (verificado: solo `MemoryStore` de producción), así que el cold-cache de consultas completas AÚN NO EXISTE — fingirlo está prohibido.
4. **«Compara con Neo4j y listo.»** Comparaciones entre motores exigen metodología completa — mismo hardware, tuning comparable, datasets representativos — que no cabe aquí. El objetivo honesto es otro: medir la EVOLUCIÓN de LiraDB CONTRA SÍ MISMA.

Y debajo de todo, la pregunta crítica del CORPUS: ¿cuál es el DATASET DE REFERENCIA (las «100k personas / 500k relaciones») que convierte cada cifra en reproducible?

## 34.3 Modelo mental

Piensa en una **torre de instrumentos**. No se sube por precisión creciente sino por PREGUNTA distinta:

```text
        ¿QUÉ PREGUNTA RESPONDE CADA INSTRUMENTO?
 ▲  flamegraph     ¿dónde se va el CPU POR DENTRO?          (perfilado, offline)
 │  contadores     ¿cuánto TRABAJO hizo cada operador?      (ExecMetrics, cap. 20)
 │  bench consulta ¿cuánto tarda una consulta END-TO-END?   (criterion + pipeline)
 │  microbench     ¿cuánto tarda UNA operación de pieza?    (encode, seek, CSR…)
 └──────────────────────────────────────────────────────────────────────────────▶
   localiza QUÉ componente falló                        explica POR QUÉ falla
```

| Pregunta | Instrumento | Artefacto en LiraDB |
|---|---|---|
| ¿Cuánto tarda UNA pieza? | **microbenchmark** | `bench_micro.rs` |
| ¿Cuánto tarda end-to-end? | **benchmark de consulta** | `bench_consultas.rs` (Q1–Q5) |
| ¿Cuánto TRABAJO hizo? | contadores por operador | `ExecMetrics` (cap. 20) + test `consultas_reportan_exec_metrics` |
| ¿Fue frío o caliente? | hit ratio del buffer pool | caps. 12/13 + test propio |
| ¿Dónde vive el tiempo DENTRO? | **flamegraph** | `perf` + `[profile.bench] debug = true` (fuera de CI) |
| ¿Mejoró o empeoró desde ayer? | **baseline** | `--save-baseline` / `--baseline` de criterion |

Y debajo de la torre, el **calibrado**: lo que separa un experimento de una opinión. Dataset DETERMINISTA (misma entrada siempre), `black_box` (que el compilador no borre el trabajo), warm-up, repeticiones con estadística, y declaración de hardware. Sin calibrado, todo número es incomparable. Así que decláralo aquí arriba y úsalo para TODO lo que sigue:

> Todas las cifras de este capítulo: Intel Xeon E5-2682 v4 @ 2,50 GHz, 64 núcleos, Linux, rustc 1.96.0, perfil release.

```text
Lo que SÍ se mide hoy:   piezas sobre BufferPool/FilePager (frío/caliente, hit ratio)
                         consultas completas sobre MemoryStore (dataset 100k/500k)
Lo que AÚN NO:           cold-cache de consultas (espera DiskStore tras el puerto)
```

El momento ¡ajá!: una sola cifra puede mentir; throughput + latencia + contadores + flamegraph JUNTOS cuentan la verdad — y el primer uso serio del aparato será cazarnos A NOSOTROS MISMOS (§34.8).

## 34.4 Primera solución

La versión ingenua es la que todo el mundo escribe — incluido nuestro yo de ayer:

```rust
let inicio = Instant::now();
for _ in 0..1_000 {
    let bytes = encode_value(&v);
    decode_value(&bytes).unwrap();
}
println!("tardó {:?}", inicio.elapsed());
```

Compila, corre, imprime un número. Y ahí termina su utilidad: ese número no significa nada defendible.

## 34.5 Sus límites

1. **Una sola medida es una anécdota.** Turbo térmico, el scheduler que te roba CPU, otra pestaña compilando: sin repeticiones ni estadística, el ruido ES el resultado. Criterion corre warm-up y decenas de muestras precisamente porque el primer número nunca vale.
2. **El compilador puede eliminar el trabajo.** Si el resultado de `decode_value` no alimenta nada observable, release tiene derecho a no hacerlo NUNCA. Mides lo que queda, no lo que crees. La defensa estándar es `std::hint::black_box`: una caja negra que el optimizador no puede mirar dentro.
3. **El setup se factura si lo dejas dentro.** Construir el valor, abrir ficheros, resolver planes: si ocurre en el bucle medido, infla la cifra con trabajo ajeno a la operación estudiada.
4. **Un escalar no es una distribución.** `elapsed()` da una suma total: ni dispersión, ni cola, ni p99. Y la media de muchas corridas esconde exactamente lo que importa (§34.6).
5. **Sin hardware ni flags declarados, es irreproducible.** Otro equipo no puede comparar; tú mismo no podrás mañana. Lección de Weicker: las reglas van ANTES que los números.

## 34.6 Solución evolucionada

### El suelo común: un dataset determinista

Todo empieza por la entrada. `Xorshift64Star` (Marsaglia, Journal of Statistical Software, 2003) son quince líneas de enteros puros con std — la regla «primero a mano»: meter `rand` sería una dependencia de RUNTIME para generar datos de TEST, peso y superficie de supply-chain gratuitos. Rechaza la semilla 0 (punto fijo del generador) y fija la pública: `SEMILLA_REFERENCIA = 0x9E37_79B9_7F4A_7C15` (la fracción áurea; nada mágico, buena dispersión).

Sobre él, `dataset_referencia(seed)` construye las 100k personas y 500k relaciones en dos fases: una ventana semilla densa de 256 nodos (la ventaja temprana) y crecimiento con **adjunción preferencial** — cada extremo nuevo sale, con probabilidad 850‰, de un extremo de arista existente elegido al azar, que es proporcional al grado. Es Barabási-Albert barato (muestrear extremos evita llevar cuentas ponderadas) y produce heavy-tail LIGERO: mayoría de nodos con grado bajísimo y 128 hubs que concentran más del 25 % de los extremos (`dataset_hubs_concentran_grado_maximo`). La distribución uniforme habría sido la mentira cómoda: nadie despliega grafos uniformes, y escondería justo el coste de los hubs que un motor de grafos debe enseñar. El resto del calibrado: esquema de 10 etiquetas y 20 claves con nodos sparse (~80 % de presencia por clave — property graphs reales tienen props opcionales), y `email` ÚNICO por nodo (`usuario000123@ejemplo.es`) para alimentar la ruta IndexSeek del cap. 21. Presupuesto: ~0,3 s en release, muy lejos de los 10 s del contrato. Y los metadatos de consulta vienen resueltos de la generación (`hubs`, `nodo_grado_bajo`, `par_camino_minimo` garantizado por BFS dirigido): nada de buscar en caliente, todo reproducible byte a byte.

### Criterion entra al workspace

Los benchmarks viven en el primer `benches/` del workspace con `[[bench]]` y `harness = false` (criterion trae su propio `main`). La integración paga sola: `verify.sh` COMPILA los benches (check/clippy con `--all-targets`) y `cargo test` los CONSTRUYE pero no los EJECUTA — el Cargo Book lo llama *build only*. Tu puerta sigue verde y rápida; correr benches es un acto EXPLÍCITO tuyo. Cada grupo declara `Throughput::Elements(n)`: sin denominador explícito, «segundos» no dicen nada; con él, criterion imprime elem/s comparables entre runs y baselines.

### La cola: percentiles calculados a mano

Criterion expone media, mediana y desviación — pero no percentiles arbitrarios, y el lenguaje de los SLO reales es la cola (Gil Tene, «How NOT to Measure Latency», 2015). Por eso `percentiles(&[u64]) -> Percentiles { p50, p90, p99, p999 }` calcula nearest-rank en ENTEROS sobre muestras crudas: `k = ⌈p·n⌉`, índice `k−1`, sin floats ni ambigüedades — auditable a mano. El test `percentiles_casos_conocidos_exactos` lo clava con 1..=100: `(50, 90, 99, 100)`.

### Frío/caliente HONESTO, a nivel componente

La única frontera frío/caliente que este capítulo puede cruzar sin mentir es la de COMPONENTE: un `BufferPool<FilePager>` real sobre tempfile (caps. 12+13), primera pasada contra segunda, `hit_ratio()` como juez. El cold-cache de consultas COMPLETAS espera a un DiskStore detrás del puerto — que no existe hoy — y fingirlo con trucos de proceso sería exactamente el tipo de medición falsa que la anécdota de Weicker condena.

### Contadores junto a tiempos, y baselines contra sí misma

`ejecutar_plan` devuelve filas Y `ExecMetrics`: el número sin el TRABAJO que lo causó es mitad de la verdad. Aquí solo se REPORTAN (instrumentarlos en producción es el cap. 35). Y la comparación permitida es una sola: LiraDB contra LiraDB, con baselines de criterion.

## 34.7 Código completo ejecutable

Todo vive en tres piezas que puedes leer de corrido: `src/cap34_benchmarks.rs` (el suelo compartido), `benches/bench_micro.rs` y `benches/bench_consultas.rs`. Las firmas que lo sostienen:

```rust
pub const SEMILLA_REFERENCIA: u64 = 0x9E37_79B9_7F4A_7C15;

pub fn dataset_referencia(seed: u64) -> DatasetReferencia {
    /* store: MemoryStore · hubs · emails_muestra · ids_muestra
       nodo_grado_bajo · par_camino_minimo: (NodeId, NodeId) */
}

pub fn percentiles(muestras: &[u64]) -> Percentiles; // { p50, p90, p99, p999 }

pub fn ejecutar_plan(store: &dyn GraphStore, plan: &LogicalPlan)
    -> Result<(ResultSet, ExecMetrics), ExecError>;   // filas + contadores
```

Para consultas, los constructores de Q1/Q2 producen el plan semi-ligado exacto que la regla R4 del cap. 21 genera (`plan_q1_point_lookup`, `plan_q2_expand_desde`); Q3/Q4 corren texto completo por `ejecutar_texto`; Q5 llama a `dijkstra_path` del cap. 22 (LiraQL aún no tiene shortest-path: delimitado por contrato, no fingido). Y el cableado en `Cargo.toml`:

```toml
# crates/vol2-liradb/Cargo.toml
[[bench]]
name = "bench_micro"
harness = false      # criterion trae su propio main con estadística

[[bench]]
name = "bench_consultas"
harness = false

[dev-dependencies]
criterion = "0.7"    # única dev-dependency nueva del capítulo

# Cargo.toml de la RAÍZ del workspace
[profile.bench]
debug = true         # símbolos legibles para perf/flamegraph
```

Fíjate en lo que NO hay: cero cambios en caps. 7-33, ninguna dependencia nueva de runtime, y ni un paso de benchmarking dentro de `verify.sh`.

## 34.8 Prueba de fuego

Primero el bucle rápido, en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap34
```

Trece tests en verde: `prng_determinista_misma_semilla_misma_secuencia`, `prng_rechaza_semilla_cero`, `percentiles_casos_conocidos_exactos`, `percentiles_monotonos_y_completos`, `percentiles_rechaza_vacio`, `dataset_cuenta_exacta_100k_nodos_500k_aristas`, `dataset_determinista_misma_semilla_grafo_identico`, `dataset_hubs_concentran_grado_maximo`, `dataset_esquema_10_etiquetas_20_claves_email_unico`, `dataset_invariantes_del_puerto_se_cumplen` (¡el oráculo del cap. 33 firmando el generador!), `consultas_reportan_exec_metrics`, `optimizador_real_produce_index_seek_en_mini` y `buffer_pool_frio_caliente_hit_ratio_sube`. Ejecútalo tú: el tiempo exacto depende de tu máquina y no lo necesitas para nada.

Ahora los números. Hardware re-declarado: Xeon E5-2682 v4 @ 2,50 GHz, Linux, rustc 1.96.0, release. Consultas end-to-end sobre el dataset completo (`cargo bench -p vol2-liradb --bench bench_consultas`):

| Bench | Ruta | Filas | Mediana |
|---|---|---|---|
| `q1_point_lookup_email_index_seek` | `IndexSeekOp` + Project | 1 | **2,19 µs** (455,7 Kelem/s) |
| `q2a_expand_un_hop_desde_hub` | IndexSeek→Expand→Project | 2.078 | **8,25 ms** |
| `q2b_expand_un_hop_desde_nodo_grado_bajo` | ídem | 2 | **10,4 µs** |
| `q3_scan_filtro_sin_indice` | NodeScan→Filter→Project | 15.104 | **230,21 ms** |
| `q4_proyeccion_amplia` | NodeScan→Project (5 props) | 100.000 | **326,43 ms** |
| `q5_camino_minimo_dijkstra_path` | API `dijkstra_path` (cap. 22) | 6 saltos | **85,09 ms** |

Tres lecturas que valen el capítulo entero. Primera: **hub vs. grado bajo, ×794**. Misma consulta, misma tabla, misma estructura de plan: la diferencia entre expandir 2.078 filas y expandir 2 es casi TRES órdenes de magnitud — el coste de un expand vive en el fanout, y el heavy-tail del dataset existía para enseñarte eso. Segunda: los contadores junto al tiempo. El bench de Q3 imprime su ventana de trabajo al arrancar:

```text
Q3_scan_filtro_sin_indice: filas_devueltas=15104 | operadores [NodeScan:100000 Filter:15104 Project:15104]
```

230 ms para recorrer 100.000 nodos y filtrar 15.104 — sin índice, el scan es inevitable, y AHORA sabes cuántos elementos fluyeron por cada operador, no solo cuánto tardó. Tercera, la honestidad de siempre: Q4 no lleva `LIMIT` porque la gramática LiraQL aún no lo tiene — se mide la proyección amplia completa, término dominante del coste de cualquier LIMIT pequeño. Y Q5 tarda 85 ms en encontrar 6 saltos porque explora prácticamente todo el componente: la finalización anticipada de Dijkstra solo corta cuando el destino SALE del heap — hasta entonces, paga el grafo.

Microbenchmarks (`cargo bench -p vol2-liradb --bench bench_micro`):

| Grupo | Bench | Denominador | Mediana | Lectura |
|---|---|---|---|---|
| `09_encoding_value` | `encode_decode_roundtrip` | 256 valores | 17,86 µs | 14,33 Melem/s |
| `15_seeks_indices` | `hash_index_get` | 10.000 seeks | 4,30 ms | 2,32 Melem/s |
| `15_seeks_indices` | `bplustree_get` | 1.536 seeks | 23,63 µs | 65 Melem/s |
| `15_seeks_indices` | `bplustree_range_scan_ancho_32` | 6 rangos × 32 | 2,68 µs | cobertura completa |
| `14_csr_vs_puerto` | `csr_neighbors_out_todos` | 500.000 aristas | 1,33 ms | 377 Melem/s |
| `14_csr_vs_puerto` | `puerto_out_edges_y_get_edge_todos` | 500.000 aristas | 21,11 ms | 23,7 Melem/s |
| `algoritmos_dataset` | `26_proyeccion_ponderada_proyectar` | 500.000 aristas | 123,98 ms | una pasada, store quieto después |
| `algoritmos_dataset` | `22_dijkstra_completo_desde_hub` | 100.000 nodos | 81,43 ms | pesos constantes = contar saltos |
| `algoritmos_dataset` | `24_page_rank_hasta_10_iteraciones` | 5 M aristas·iter | 112,21 ms | 44,6 Melem/s |
| `13_buffer_pool` | `pasada_fria_todo_misses` | 256 páginas | 423,4 µs | hit_ratio 0.000, evictions 240 |
| `13_buffer_pool` | `pasada_caliente_todo_hits` | 256 páginas | 2,98 µs | hit_ratio 1.000 |
| `trampas` | `trampa_setup_en_el_loop_sin_black_box` | 256 valores (VENENOSO) | 63,11 µs | mide ×3,5 de más |

Dos contrastes con nombre propio. El CSR itera las 500k aristas **×16 más rápido** que el puerto (1,33 ms frente a 21,11 ms): `out_edges` clona un `Vec` por llamada y cada arista exige un `get_edge`. Ahí tienes LA justificación MEDIDA de las proyecciones materializadas del cap. 26: no era estética, era ×16. Y el frío/caliente de componente: 423,4 µs con hit_ratio 0.000 frente a 2,98 µs con 1.000 — **×142** por vivir en caché. Esa frontera es honesta: componente, no consultas.

### El hallazgo estrella: nuestro aparato nos cazó a nosotros

Al preparar Q1 pasó lo que este capítulo vino a buscar. El plan correcto exige resolver ids contra el catálogo del optimizador… y `Catalog::collect` (cap. 21) resultó ser **O(valores_distintos²)**: `eq_push` inserta con búsqueda LINEAL (`find` sobre el vector de entradas) por cada propiedad de cada nodo. Con los 100.000 emails únicos —que generan dos entradas cada uno, por etiqueta y por comodín— construir el catálogo tarda **~224 segundos**, mientras construir TODO el grafo de 100k nodos y 500k aristas tarda **281 milisegundos**. Ochocientas veces más caro que el dato que cataloga. La decisión fue no parchear: Q1/Q2 miden EJECUCIÓN sobre el plan semi-ligado exacto (ids resueltos UNA vez, fuera de la región medida), y `optimizador_real_produce_index_seek_en_mini` demuestra en el dataset MINI —donde el catálogo sí es barato— que el optimizador REAL produce estructuralmente ese mismo plan con los mismos resultados. El capítulo MIDE; reparar es deuda documentada del optimizador (§34.9).

Cierra el círculo con las **baselines**, la comparación que sí está permitida:

```text
$ cargo bench -p vol2-liradb --bench bench_consultas -- --save-baseline cap34-inicial
$ # …semanas después, tocas algo (la política del pool, un sort, lo que sea)…
$ cargo bench -p vol2-liradb --bench bench_consultas -- --baseline cap34-inicial
```

Criterion compara cada grupo contra su referencia e imprime el veredicto — `Performance has REGRESSED` cuando tocas donde no debías. Eso es medir la EVOLUCIÓN contra sí misma, con reglas de SPEC en miniatura.

## 34.9 Qué hemos sacrificado

1. **Cold-cache de consultas: no existe y no se simula.** Sin DiskStore tras el puerto, cualquier «frío» de Q1-Q5 sería teatro. Documentado como frontera; llega cuando llegue el store en disco.
2. **Flamegraphs fuera del pipeline.** `perf` es Linux-only y cargo-flamegraph es toolchain externa; `[profile.bench] debug = true` compra símbolos legibles al precio de binarios mayores. Ningún paso perf en `verify.sh`.
3. **Nada de cross-motor.** Ni Neo4j ni Kùzu en tablas «de regalo»: sin metodología completa, sería el sensacionalismo que Weicker documentó. Baselines internos solamente.
4. **Sin profiling de memoria.** heaptrack y compañía quedan fuera; aquí se mide TIEMPO, TRABAJO y tasas de acierto.
5. **Percentiles limitados al harness propio.** Criterion no expone muestras crudas arbitrarias; nuestro nearest-rank cubre el hueco con enteros auditablemente, a cambio de gestionar tú el muestreo.
6. **El catálogo cuadrático queda SIN arreglar.** ~224 s medidos, deuda viva del cap. 21. Este capítulo la destapó, la cifró y la dejó señalizada — medir no es reparar, pero sin medir nadie habría sabido.

## 34.10 Cómo lo hace una BBDD real

Nada de lo que hiciste es exótico: es la versión artesanal de industria madura. El **Yahoo! Cloud Serving Benchmark** (Cooper et al., CIDR 2010) fijó el patrón YCSB que usa medio mundo: workloads declarados (A-F), cliente generador reproducible, y resultados publicados CON su configuración — exactamente nuestra tesis del dataset + hardware + reglas. PostgreSQL trae `pgbench` (su variante de TPC-B), y para MySQL lo habitual es `sysbench`; dentro del motor, `pg_stat_statements` acumula tiempos y filas POR CONSULTA en producción — nuestros `ExecMetrics` con décadas de rodaje — junto a `EXPLAIN ANALYZE`, que ejecuta el plan y te devuelve tiempo real por operador. SQLite expone contadores vía `sqlite3_status`. Los motores de grafos publican sus benchmarks oficiales de Neo4j y Kùzu con hardware declarado y datasets descargables: la diferencia entre eso y un titular hueco es la metodología, nunca el número (Hennessy & Patterson dedican su capítulo 1 de «Computer Architecture: A Quantitative Approach» a esta disciplina entera: benchmarks como contrato experimental). Y cuando hay que mirar DENTRO del CPU, el flujo canónico de Linux es `perf stat` / `perf record -g` + flamegraph — Brendan Gregg, que los popularizó diagnosticando producción en Netflix («The Flame Graph», ACM Queue 14(2), 2016), resume la lectura: los bloques anchos SON el perfil.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (15+34): añade a `bench_micro.rs` un bench de `range_scan` con OTRO ancho de rango (por ejemplo 128 entradas), siguiendo la convención `bench_*`, y declara el `Throughput::Elements(...)` correcto. ANTES de correrlo, anota si esperas que suba o baje el elem/s respecto al ancho 32 — y por qué.
- *Intermedio* (9+20+34): diagnóstico del espécimen envenenado. SIN ejecutar nada, abre `trampa_setup_en_el_loop_sin_black_box`, identifica sus dos pecados, decide qué parte del tiempo es trabajo bajo estudio y predice contra qué cifra honesta debería compararse. Después córrelo y verifica tu predicción contra los 63,11 µs y los 17,86 µs de §34.8.
- *Experto* (22+34): la cola de Q5. Cronometra 100 ejecuciones de `camino_minimo_q5` con `Instant::now()`, pasa tus muestras a `percentiles()` e informa p50/p99/p999. Guarda baseline, rompe deliberadamente ALGO pequeño (un orden, una política) hasta que `--baseline` grite REGRESSED, y documenta el diff. Alternativa: `perf record -g cargo bench -- q3_scan_filtro_sin_indice` + flamegraph, y nombra los tres bloques más anchos.

## 34.11 Lo que te llevas

- **La torre de instrumentos**: microbench localiza QUÉ pieza, consulta end-to-end dice CUÁNTO, contadores cuentan el TRABAJO, flamegraph muestra el POR QUÉ interno, baseline vigila la EVOLUCIÓN. Cada piso, una pregunta.
- **Calibrado antes que cifras**: dataset determinista, `black_box`, warm-up, repeticiones, hardware declarado. Sin eso, todo número es marketing.
- **El dataset de referencia es el suelo**: `SEMILLA_REFERENCIA` pública, heavy-tail ligero con hubs reales, email único para la ruta IndexSeek. Mismo grafo siempre ⇒ las diferencias entre runs son ruido, no entrada distinta.
- **Throughput necesita denominador**: `Throughput::Elements(n)` convierte segundos en elem/s comparables.
- **La cola manda**: p50/p99/p999 con nearest-rank a mano; la media esconde al cliente que grita.
- **Frío/caliente honesto**: componente (BufferPool, ×142) sí; consultas, todavía no — y decirlo ES parte del capítulo.
- **Contadores junto a tiempos**: `[NodeScan:100000 Filter:15104 Project:15104]` cuenta la mitad de la verdad que el tiempo solo calla.
- **Baselines, no rivales**: `--save-baseline` / `--baseline` contra sí misma; cross-motor sin metodología, jamás.
- **Y el hallazgo**: el aparato de medición detectó nuestra propia deuda O(n²) — medir bien también AUDITA.

## 34.12 Ojo, cuidado con…

- **«Mi cronómetro con `Instant` basta.»** No: sin repeticiones es anécdota, sin `black_box` el compilador borra trabajo, y en debug ni siquiera mides el binario de producción.
- **«La media resume el rendimiento.»** Resume el CENTRO. Tu SLA vive en el p99 — una lentitud de cada cien no se ve en la media y sí en la factura.
- **«Reinicio el proceso y ya está frío.»** La page cache del SO persiste; y en LiraDB el frío/caliente de consultas ni siquiera existe aún (no hay store-en-disco tras el puerto). Componente o nada.
- **«Compara con Neo4j y publica.»** Comparar motores exige protocolo completo (hardware, tuning, datasets, licencias). Sin él, estás haciendo marketing, no ingeniería.
- **Confundir los pisos**: throughput (elementos/tiempo) y latencia (tiempo/operación) responden preguntas distintas; un microbenchmark no valida una consulta; un flamegraph no es tracing — instrumentar producción es el cap. 35.

## 34.13 Pin de batalla

> *«Una sola cifra puede mentir; throughput, latencia, contadores y flamegraph juntos cuentan la verdad. Y un número sin hardware ni reglas declaradas no es una medida: es marketing.»*

## 34.14 Si solo lees 30 segundos

Dataset determinista de referencia: 100k nodos / 500k aristas, semilla pública `0x9E37_79B9_7F4A_7C15`, PRNG xorshift64* a mano, 128 hubs con >25 % de los extremos, email único. Primer `benches/` del workspace con criterion 0.7 (`harness = false`); `cargo test` los construye sin ejecutarlos. Torre: microbench → consulta → contadores (`ExecMetrics`) → flamegraph (offline) → baseline. Cifras clave (Xeon E5-2682 v4, release): point lookup 2,19 µs; expand desde hub 8,25 ms frente a 10,4 µs desde grado bajo (**×794**); scan+filtro 230,21 ms con `[NodeScan:100000 Filter:15104 Project:15104]`; CSR ×16 sobre el puerto — la justificación medida de las proyecciones del cap. 26; pool frío/caliente ×142 con hit_ratio 0.000→1.000. Percentiles p50/p90/p99/p999 a mano (nearest-rank, enteros). Baselines: `--save-baseline` / `--baseline`, contra sí misma, nunca contra titulares. Hallazgo: `Catalog::collect` es O(valores_distintos²) — ~224 s con los emails únicos; deuda señalizada, no parcheada. Todo número sin metodología es marketing.

## 34.15 Una historia pequeña

Octubre de 1994. Thomas Nicely, profesor de matemáticas en Lynchburg College, llevaba meses calculando la constante de Brun — la suma de recíprocos de primos gemelos — con millones de divisiones en coma flotante. Al contrastar resultados entre máquinas, una discrepancia minúscula no se iba: el flamante Pentium dividía mal ciertos pares. Su ejemplo canónico pasó a la historia: `4195835/3145727` salía mal a partir de algunos dígitos. La causa estaba dentro del chip: cinco entradas incompletas en la tabla de radicales del divisor SRT — un fallo tan raro (uno entre miles de millones de divisiones aleatorias) que ningún uso normal lo vería jamás. Pero Nicely no era uso normal: era cómputo intensivo CONTRASTADO, y el contraste lo delató. Intel minimizó, la prensa hizo portada (The New York Times, 22 de noviembre de 1994), y el asunto acabó costando a la compañía una provisión contable de unos 475 millones de dólares en sustituciones. La moraleja es exactamente la de este capítulo: el error vivía en la COLA extrema, invisible para cualquier media de uso; y solo lo encontró quien comparaba contra una referencia — un baseline, aunque no lo llamara así. Sin valor de referencia no hay discrepancia visible; sin discrepancia visible, el bug sigue cobrando intereses durante años.

## Ejercicios resueltos

**1. Predice la trampa (9+34).** El bench `trampa_setup_en_el_loop_sin_black_box` comete dos pecados: ¿qué cifra infla cada uno? El pecado 1, setup contabilizado: el `format!` y la asignación del `String` ocurren DENTRO del bucle medido, así que parte del tiempo medido es preparación, no encode/decode. El pecado 2, sin `black_box`: el resultado final se descarta sin opacidad, invitando al compilador a eliminar trabajo muerto si el bench se simplifica. Verificación empírica: 63,11 µs frente a los 17,86 µs del grupo honesto `encode_decode_roundtrip` — mide **×3,5 de más**, y TODA la inflación es trabajo ajeno a la operación estudiada. Corre ambos grupos y compáralos.

**2. Retrieval: la torre, de memoria.** Cierra el libro y escribe los cuatro pisos de la torre de instrumentos con SU pregunta, más el calibrado mínimo. Respuesta: microbenchmark («¿cuánto tarda UNA pieza?»), bench de consulta («¿cuánto tarda end-to-end?»), contadores («¿cuánto TRABAJO hizo cada operador?»), flamegraph («¿dónde se va el CPU por dentro?» — offline). Calibrado: dataset determinista, `black_box`, warm-up, repeticiones, hardware declarado. Si escribiste «flamegraphs en CI», revisa §34.9: son deliberadamente offline.

**3. Nearest-rank a mano.** Ordenaste 10 latencias y quieres el p99. Calcula: `k = ⌈0,99·10⌉ = ⌈9,9⌉ = 10` → el p99 es la décima muestra ordenada, es decir, la PEOR. Moraleja doble: con pocas muestras, el p99 colapsa en el máximo — por eso el harness exige muestras suficientes y por eso criterion solo no basta. Verificación: `percentiles_casos_conocidos_exactos` comprueba 1..=100 → `(50, 90, 99, 100)`.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 15+34).** Añade un bench nuevo para `HashIndex::get` con OTRO número de cubetas (por ejemplo 512 en vez de 2.048, misma carga de 10.000 claves), siguiendo la convención `bench_*` y con `Throughput::Elements(10_000)`. Antes de correr: ¿esperas mejor o peor elem/s que el 2,32 Melem/s citado, y qué papel juegan los overflows? Verificación: `cargo bench -p vol2-liradb --bench bench_micro`.

**Intermedio (predecir; 12+13+34).** Modifica temporalmente `PAGINAS_POOL` a 512 manteniendo el pool frío de 16 frames. ANTES de correr, escribe tu predicción de la ventana impresa: hits, misses, evictions y hit_ratio de UNA pasada fría. Luego ejecuta el informe de contadores y compara. Criterio: las cuatro cifras por escrito antes del primer comando.

**Experto (crear; 22+34).** Dos caminos: (a) percentil-a-percentil de Q5 — 200 ejecuciones de `camino_minimo_q5` con `Instant`, muestras a `percentiles()`, informe p50/p99/p999, y luego `--save-baseline` + romper algo pequeño + detectar con `--baseline`; documenta el diff exacto que dispara el REGRESSED. (b) `perf record -g` sobre `q3_scan_filtro_sin_indice`, flamegraph.svg, y nombra los tres bloques más anchos explicando a qué operador corresponden. Criterio en ambos: cero cambios en caps. anteriores.

## Para profundizar

- **R. P. Weicker, «Dhrystone Benchmark: Rationale for Version 2 and Measurement Rules» (ACM SIGPLAN Notices 23(8), 1988)** y **«An Overview of Common Benchmarks» (IEEE Computer, diciembre de 1990)** — la fuente primaria de la anécdota: el benchmark secuestrado y la respuesta del autor: reglas. Complemento: **SPEC CPU Run Rules** (spec.org), la institucionalización de esas reglas.
- **Todd Mytkowicz et al., «Producing Wrong Data Without Doing Anything Obviously Wrong!» (ASPLOS 2009)** — el entorno silencioso (variables de entorno, alignment) invierte resultados: por qué el calibrado no termina nunca de importar.
- **George Marsaglia, «Xorshift RNGs» (Journal of Statistical Software, 2003)** — el PRNG de quince líneas del dataset.
- **Albert-László Barabási y Réka Albert, «Emergence of Scaling in Random Networks» (Science 286, 1999)** — adjunción preferencial y free-scale: el porqué del heavy-tail del dataset.
- **Criterion Book (bheisler.github.io/criterion.rs)** — grupos, `Throughput::Elements`, `black_box`, baselines: el manual de la torre.
- **Gil Tene, «How NOT to Measure Latency» (2015)** — coalescing, media engañosa y por qué el lenguaje de los SLO es la cola.
- **Brendan Gregg, «The Flame Graph» (ACM Queue 14(2), 2016; brendangregg.com)** y **«Systems Performance», 2ª ed. (2020)** — leer un flamegraph y el flujo `perf` completo.
- **John Hennessy y David Patterson, «Computer Architecture: A Quantitative Approach» (cap. 1, fallacies and pitfalls)** — el benchmark como contrato experimental, con décadas de contrajuegos.
- **Brian Cooper et al., «Benchmarking Cloud Serving Systems with YCSB» (CIDR 2010)** — el harness que industrializó workload declarado + hardware declarado.
- **Cargo Book (targets y perfiles)** — por qué `cargo test` construye los benches sin ejecutarlos, y qué compra `[profile.bench] debug = true`.
- **PostgreSQL: `pg_stat_statements`, `EXPLAIN ANALYZE`, `pgbench` (docs oficiales)** — los contadores-y-tiempos de producción que este capítulo reporta a mano.

## Mini-diálogo: en guardia nocturna

> — Son las tres de la mañana. Pager: un cliente grita en Slack que «la base de datos va lenta». Abres el dashboard: VERDE. Latencia media de consultas: 40 ms. Media, verde, tranquila.
>
> — Respira. Si la media está bien y el cliente grita, ¿qué te falta por mirar?
>
> — La… ¿distribución?
>
> — Exacto. ¿Qué dice el p99 de las consultas de scan?
>
> — Segundos. Uno de cada cien MATCH con filtro tarda segundos… ¡y la media ni se inmuta!
>
> — Porque la media promete a todos lo que el p99 niega a unos pocos. El cliente que grita ES el p99 hecho persona.
>
> — Podría haberlo visto si el dashboard enseñara percentiles…
>
> — Ahora ya sabes calcularlos con tu propio harness. Pero fíjate en lo que acaba de pasar: tenías el DATO (contadores, tiempos) y no lo tenías VISIBLE. Nadie mira un histograma que no está en pantalla.
>
> — ¿Entonces lo que me falta no es medir mejor?
>
> — No. Ya sabes MEDIR. Te falta hacer la medición VISIBLE — contadores y spans que se emiten solos, dashboards que enseñan la cola, un flag `--profile` en la CLI. Eso tiene nombre: observabilidad. Y es exactamente el próximo capítulo.

---

*(Próximo capítulo: 35 — Observabilidad. Ahora que puedes MEDIR, hazlo VISIBLE en producción: `queries_total`, spans query→plan→operador→page fetch, y `liradb query --profile`.)*
