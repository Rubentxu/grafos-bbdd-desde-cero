# CONTRATO DE CAPÍTULO — Vol.II Cap. 35: Observabilidad interna

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla: todo lo que el
> capítulo hace VISIBLE ya existe y está verde — el Volcano del cap. 20 (`PhysicalOperator`
> con `open`/`next`/`close`/`name()`/`rows_produced()`/`collect_metrics()`, nombres
> canónicos `&'static str`: `NodeScan`, `IndexSeek`, `Expand`, …; `ExecMetrics {
> per_operator: Vec<(&'static str, u64)>, rows_returned }` expuesto SOLO por
> `Executor::metrics()` — ni `run()` ni `Query::execute()` lo revelan: verificado
> 2026-08-25), el trait `Pager` con `FilePager` real sobre fichero (cap. 12),
> `BufferPool::metrics() -> Metrics { page_reads, page_writes, buffer_hits, buffer_misses,
> evictions }` + `hit_ratio()` (cap. 13), `Wal` con `as_bytes() -> &[u8]` (= bytes
> escritos TOTALES, derivable por delta), `syncs()` y `record_count()` — SIN contador de
> bytes ni de transacciones; y `Transaccion::commit(self)`/`rollback(self)` consumen self
> sin contar nada (caps. 28/27, verificado). Precedente EXACTO del patrón que este
> capítulo generaliza: `ContandoStore<'a>` (inner + `Cell<u64>`, sólo lectura), el
> «voltímetro» decorador del cap. 26. La CLI ya desenrolla el pipeline en
> `pipeline_con_detalle(src, store, out, plan, stats)` (lib.rs:424: parse → lower →
> `Executor::new` → execute → `Métricas: {}`) tras los flags clap `--plan/--stats`;
> goldens BYTE-EXACTOS en `tests/golden/{demo,explain}.txt` (`ACTUALIZAR_GOLDEN=1` para
> regenerar); el REPL ya conduce `autocommit`/`Transaccion` en `sesion.rs` — la capa
> conductora donde se cablean los contadores de transacciones SIN tocar caps previos.
> Estado verificado 2026-08-25: **822 tests** ALL_GREEN; toolchain 1.96.0; `vol2-liradb`
> **dependency-free** en runtime (dev-deps: `tempfile`, `proptest`, `criterion`).
> Código NUEVO previsto:
> `cap35_observabilidad.rs` en `vol2-liradb` (std puro: registro `Contadores` con campos
> fijos `Cell<u64>` + `snapshot()` + `Display` imitando el text format de Prometheus;
> `MedidorOperador` envolviendo cualquier `dyn PhysicalOperator`; `MedidorPaginas<P:
> Pager>`; derivación de `nodes_scanned`/`relationships_expanded`/`index_hits` desde
> `ExecMetrics.per_operator`) y, en la CLI, `src/observabilidad.rs`
> (`OperadorTrazado`/`PagerTrazado` apilando span sobre medidor; mini-subscriber propio
> `SuscriptorArbol` ~100 líneas sobre el trait `tracing::Subscriber` — 7 métodos
> requeridos verificados en `tracing-core/src/subscriber.rs`), flag ADITIVO `--profile`
> y `tracing = "0.1"` como ÚNICA dependency nueva, SOLO en la CLI. Hallazgo honesto
> heredado de caps. 33/34: NO existe `GraphStore` respaldado por disco — las consultas
> corren sobre `MemoryStore`, así que el nivel page fetch se demuestra a nivel COMPONENTE
> (índice sobre `BufferPool<FilePager>`). Decisiones irán a `MIGRATION-PATTERN.md` §40.
> Pregunta crítica del CORPUS
> (`vol-II-cap-35`): «Span hierarchy: query → plan → operator → page fetch». Cap. 5 de la
> Parte VII; recoge el gancho del cap. 34 («ahora que puedes MEDIR, hazlo visible» / «de
> MEDIR a VER»). Gancho saliente: cap. 36 arquitectura final — el mapa completo del motor;
> la observabilidad cierra el círculo hexagonal: puertos medidos.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el puerto hexagonal `GraphStore` (cap. 8) y su
  «voltímetro» `ContandoStore` (cap. 26); pager/`FilePager` (cap. 12) y las métricas del
  `BufferPool` con `hit_ratio()` (cap. 13); el pipeline parse → lower → optimize →
  Volcano (caps. 17-21) con `ExecMetrics` por operador; WAL/transacciones (caps. 27-28)
  sabiendo qué expone `Wal` y qué NO (contadores de tx: ninguno); la CLI con
  `--plan/--stats` y goldens byte-exactos (cap. 31); torre de pruebas (cap. 33) y
  benchmarks/percentiles/baselines (cap. 34).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «observar = repartir `println!` por el código» — no: texto plano mezcla datos y
  presentación, no distingue módulos ni niveles, contamina stderr y goldens, y no se
  puede apagar ni agregar.
  (2) «métricas contra trazas: elige una» — no: preguntas DISTINTAS y complementarias;
  los contadores (counters) dicen CUÁNTO trabajo costó la respuesta; las trazas (traces)
  dicen DÓNDE y EN QUÉ ORDEN se gastó. Sin una de las dos, el motor está medio ciego.
  (3) «instrumentar exige tocar el motor» — no: store, pager y operador son TRAITS
  públicos (caps. 8/12/20); un decorador (decorator pattern) que envuelva el trait mide
  sin cambiar una línea del motor — `ContandoStore` ya lo hizo en el cap. 26.
  (4) «un span es un log bonito» — no: un span (span) tiene nombre, PADRE y duración;
  muchos spans forman un ÁRBOL causal (trace tree). Sin jerarquía no hay respuesta a
  «¿qué parte de mi consulta fue lenta?».
- **Pregunta crítica que el capítulo tiene que responder**: «Span hierarchy: query →
  plan → operator → page fetch.» Respuesta: la jerarquía YA existe LATENTE en el pipeline
  desenrollado de la CLI — este capítulo la NOMBRA con spans anidados, la CAPTURA con un
  subscriber propio que reconstruye el árbol, y la IMPRIME con el hito
  `liradb query --profile '<LiraQL>'` junto a los contadores. El nivel página, hoy fuera
  de la ruta de consulta (MemoryStore), se demuestra en componente — frontera declarada,
  nunca fingida.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `cap35_observabilidad.rs` (vol2-liradb, std puro): registro `Contadores` con campos fijos nombrados EXACTO igual que las métricas del brief (`queries_total`, `nodes_scanned`, `relationships_expanded`, `index_hits`, `wal_bytes_written`, `transactions_committed`, `transactions_aborted`, …), mutabilidad interior `Cell<u64>` (patrón `ContandoStore`), `snapshot() -> Snapshot` copiable, `Display` imitando el text format de Prometheus (`# TYPE x counter` / `name value`) | `cargo test -p vol2-liradb --lib cap35`: tesis `contadores_display_formato_texto_y_snapshot_exacto`, `contadores_campos_fijos_sin_mapa_sin_typos` |
| Derivación de métricas EXISTENTES sin duplicar: `metricas_consulta(&ExecMetrics) -> (nodes_scanned, relationships_expanded, index_hits)` sumando `per_operator` por nombre canónico (`NodeScan`/`Expand`/`IndexSeek`); `page_reads`/`page_writes`/`buffer_hit_ratio` llegan del pool (cap. 13) por composición, no copia | `metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico` |
| `MedidorOperador<'a>`: envuelve `Box<dyn PhysicalOperator + 'a>` y acumula llamadas a `next()`, filas y tiempo en `&Contadores` — delega open/next/close/name/rows_produced/collect_metrics | `medidor_operador_cuenta_llamadas_filas_y_tiempo` (+ composición con el pipeline real Q1/Q3 del dataset mini) |
| `MedidorPaginas<P: Pager>`: envuelve cualquier pager y cuenta reads/writes/syncs y bytes movidos (read/write × PAGE_SIZE) en `&Contadores` | `medidor_paginas_cuenta_reads_writes_y_bytes_movidos` (FilePager sobre `tempfile`, dev-dep existente) |
| Contadores de fiabilidad HONESTOS: `wal_bytes_written` = delta de `Wal::as_bytes().len()` antes/después; `transactions_committed`/`aborted` contados en la capa conductora (REPL `:begin/:commit/:rollback` en la CLI; alrededor de `WalTransaccion` en tests) — caps. 27/28 NO exponen contadores internos (verificado) y NO se tocan | `wal_bytes_escritos_delta_tras_commit_waltransaccion`, `transacciones_committed_aborted_contadas_en_conductora` |
| `liradb-cli/src/observabilidad.rs`: `OperadorTrazado`/`PagerTrazado` (envoltorios que APILAN span sobre medidor — decoradores componibles), spans de fase `query → parse/lower/optimise/execute` emitidos SIEMPRE en el pipeline (coste ≈ 0 sin subscriber) y `SuscriptorArbol` (mini-subscriber propio: los 7 métodos requeridos del trait `Subscriber` + `try_close` sobrescrito + pila current-span para padres contextuales; graba árbol tipado en `RefCell<Vec<NodoSpan>>`) | `cargo test -p liradb-cli --test observabilidad_cli`: `suscriptor_arbol_jerarquia_query_plan_optimise_execute` (captura vía `with_default`, sin stderr ni globales) |
| HITO del brief: `liradb query --profile '<LiraQL>'` — flag clap ADITIVO junto a `--plan/--stats`; imprime (1) tabla de fases cronometradas con `Instant` (herencia del harness del cap. 34), (2) árbol de spans indentado, (3) `Contadores` + métricas del executor | `perfil_cli_arbol_indentado_spans_operador`, `perfil_contadores_exactos_y_fases_cronometradas`; salida REAL pegada en prosa |
| Goldens INTACTOS: `--profile` es aditivo (off por defecto); su salida NO es golden byte-exacto porque los tiempos varían — se asertan estructura (nombres/nesting exactos) y contadores exactos, nunca duraciones | `cargo test -p liradb-cli --test golden_cli` sigue en verde sin regenerar; `perfil_aditivo_goldens_demo_explain_intactos` |
| Jerarquía de CUATRO niveles demostrada a nivel COMPONENTE: seek real sobre `HashIndex`/`BPlusTree` sobre `BufferPool<FilePager>` (tempfile) envuelto con medidor+span — query → execute → index_seek → storage_read/page fetch | `jerarquia_cuatro_niveles_componente_indice_sobre_pool` (test de integración CLI; frontera DOCUMENTADA: la ruta de consultas aún no atraviesa páginas — mismo hallazgo que caps. 33/34) |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (822 + ~11 tests nuevos); `cargo fmt`/clippy limpios con `--all-targets` |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Span hierarchy: query → plan → operator → page fetch.» — el capítulo la
convierte en **respuesta en nueve pasos**:

1. **El modelo mental primero** (Dapper, 2010): una traza es un ÁRBOL de spans; cada
   span tiene nombre, padre y duración; el árbol responde «¿quién llamó a quién y cuánto
   tardó cada uno?». Google lo resolvió para miles de máquinas; LiraDB lo aplica a UNA
   consulta con cuatro pisos.
2. **La jerarquía ya existe latente**: `pipeline_con_detalle` de la CLI (cap. 31) ya
   ejecuta en orden query → parse → lower(plan) → optimise → execute. Nombrar esos pasos
   como spans anidados NO cambia el flujo: cambia cómo se VE.
3. **Spans de fase**: el span raíz `query` rodea todo; `parse`, `plan`, `optimise`,
   `execute` son hijos directos — los siete nombres del brief quedan cubiertos: fase aquí,
   `index_seek`/`expand` como operadores (paso 4), `storage_read` como página (paso 5).
4. **Spans de operador**: `OperadorTrazado` implementa el MISMO trait `PhysicalOperator`
   (cap. 20), delega en el operador real y crea un span hijo de `execute` en
   open/next/close. Los nombres canónicos ya existen (`IndexSeek`, `Expand`): la traza
   habla el idioma del `explain` del cap. 21.
5. **El nivel página — con frontera honesta**: `PagerTrazado`/`MedidorPaginas` emiten
   `storage_read` por `read()` del trait `Pager` (cap. 12). Como las consultas corren hoy
   sobre `MemoryStore`, ese nivel se demuestra en COMPONENTE (índice sobre
   `BufferPool<FilePager>`): con un futuro DiskStore la jerarquía completa aparecerá en
   la ruta de consulta sin tocar la instrumentación.
6. **Captura determinista**: el `SuscriptorArbol` graba new_span/enter/exit y resuelve
   padres contextuales (si `parent()` es `None`, el padre es el span actual); los tests
   capturan con `with_default` (dispatcher thread-local) — sin globales ni stderr.
7. **Contadores junto al árbol**: `Contadores` suma `queries_total`, deriva
   `nodes_scanned`/`relationships_expanded`/`index_hits` de `ExecMetrics.per_operator`
   (UNA sola verdad), toma pool cuando hay pager, mide WAL por delta y transacciones en
   la conductora.
8. **El hito**: `liradb query --profile 'MATCH (p:Persona)-[:CONOCE]->(q) RETURN q'`
   imprime fases cronometradas + árbol indentado + recibo de contadores. Aditivo: los
   goldens de `demo`/`explain` no cambian.
9. **Lectura conjunta**: el recibo dice que la consulta escaneó 100k nodos; el itinerario
   dice que fue el `NodeScan` bajo `execute` — y no el `Filter`. Juntas convierten
   «va lento» en «esto exacto va lento».

---

## 4. La arquitectura: una consulta es un VIAJE con itinerario y un RECIBO de peajes

Modelo mental único: **dos vistas complementarias del mismo viaje**. La TRAZA es el
itinerario (árbol de spans: por dónde pasó y cuánto tardó cada tramo); los CONTADORES
son el recibo (cuánto trabajo total costó). Ningún sistema de observabilidad real separa
ambas; LiraDB tampoco.

```
span «query» ──────────────────────────────  raíz (1 por consulta)          CONTADORES (el recibo)
 ├─ span «parse»      LiraQL → AST          queries_total            = 1
 ├─ span «plan»       AST → LogicalPlan     nodes_scanned            = Σ NodeScan
 ├─ span «optimise»   reglas (cap. 21)      relationships_expanded   = Σ Expand
 └─ span «execute»    Volcano (cap. 20)     index_hits               = Σ IndexSeek
     ├─ span «index_seek»   IndexSeekOp     query_duration.{fase}    = Instant por fase (cap. 34)
     │   └─ span «storage_read»  read(página)   ← SOLO si hay pager   page_reads/writes/hit_ratio (pool, cap. 13)
     └─ span «expand» ×N filas             wal_bytes_written        = Δ Wal::as_bytes().len()
                                           transactions_{committed,aborted} = capa conductora
```

Y debajo, la REGLA de oro heredada del cap. 26: todo punto de medida es un DECORADOR
sobre un puerto existente (`GraphStore`, `Pager`, `PhysicalOperator`). El motor no sabe
que lo observan; apilar decoradores (medidor dentro, span fuera) compone vistas sin
acoplarse.

Momento ¡ajá! perseguido: «instrumentar no es tocar el motor: es ENVOLVER sus puertos —
y la jerarquía de spans no se INVENTA, se REVELA: ya estaba en el pipeline». Y el
¡ajá! honesto: los 4 niveles no pueden verse en la ruta de consultas mientras el store
viva en RAM — decirlo ES parte del tema.

```text
Lo que SÍ se ve hoy:  query→plan→operador en la CLI (--profile); nivel página COMPLETO a
                      nivel componente (índice sobre pool); contadores de consulta, pool,
                      WAL (delta) y transacciones
Lo que AÚN NO:        page fetch EN la ruta de consulta (espera DiskStore tras el puerto);
                      exportación a backend externo (Prometheus/OpenTelemetry: N.10)
```

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Registro `Contadores` A MANO (std): campos fijos nombrados igual que la métrica, `Cell<u64>` (interior mutable monohilo), `snapshot()`, `Display` estilo text format de Prometheus | Campos fijos = typo imposible en compilación y Display determinista; `Cell` replica el patrón probado de `ContandoStore` (cap. 26); el formato de exposición se aprende IMITÁNDOLO, no dependiendo de él | `HashMap<&'static str, u64>`: claves que derivan, typos en runtime, orden no determinista | CONVENTIONS §4 (regla «primero a mano»); Prometheus, «Exposition Formats» (prometheus.io/docs/instrumenting/exposition_formats); código cap. 26 |
| 2 | El crate `metrics` (metrics-rs) se DOCUMENTA como equivalente industrial y NO entra | Es una facade con recorder GLOBAL (`set_global_recorder`): estado oculto y acoplamiento invisible frente a nuestro struct local pasable por `&`; peso de dependencia para algo de ~60 líneas; pedagógicamente, el lector debe VER dónde vive el estado | Instalar `metrics` + exporter Prometheus «porque es lo que se usa»: facade global prematura en un motor monohilo | docs.rs/metrics (patrón recorder/facade); CONVENTIONS §4; decisión espejo del cap. 33 (cargo-fuzz documentado, no integrado al pipeline) |
| 3 | `tracing = "0.1"` como dependency NORMAL pero SOLO de `liradb-cli`; `vol2-liradb` permanece dependency-free | Los spans se emiten en el pipeline de PRODUCCIÓN de la CLI (hito `--profile`), no en tests: eso es uso runtime ⇒ dependency del crate que lo usa. Los MEDIDORES (std) sí viven en la lib; el crate principal no paga ninguna dependencia | `tracing` en `vol2-liradb`: rompe la regla dependency-free ganada en 28 capítulos; o como dev-dep: entonces el hito CLI no podría emitir spans reales | CONVENTIONS §4 (Observabilidad: tracing OK); código real: `[dependencies]` vacío en vol2-liradb/Cargo.toml (verificado 2026-08-25) |
| 4 | Spans emitidos SIEMPRE en la CLI (no solo con `--profile`); `--profile` instala el subscriber e imprime | Con tracing, emitir sin subscriber instalado cuesta ≈ nada (chequeo de `enabled`); separar «instrumentar» de «activar» es LA idea de producción: cualquier subscriber futuro (stderr, OTLP) verá los spans sin recompilar | Emitir spans condicionados a un flag booleano: dos rutas de código, y la promesa de coste-cero-desactivado se vuelve mentira estructural | docs.rs/tracing («getting started»: overhead when no subscriber is set); Dapper §4 (low overhead como requisito de despliegue ubicuo) |
| 5 | Decoradores GENERALIZAN `ContandoStore`: `MedidorOperador` envuelve `dyn PhysicalOperator`, `MedidorPaginas<P>` envuelve `P: Pager` — cero cambios en caps. 12/20 | Los traits ya son la costura perfecta; el patrón está PRECEDIDO en el propio repo (cap. 26); medir y trazar SON dos decoradores APILABLES (medidor dentro, span fuera): composición visible, cada pieza testeable sola | Modificar `PhysicalOperator`/`Pager` para que cuenten ellos mismos: rompe caps. previos y acopla observación a negocio | Código cap26_proyeccion.rs (`ContandoStore` con `Cell`, sólo lectura, verificado); GoF Decorator vía precedente interno del repo |
| 6 | Métricas derivadas de `ExecMetrics.per_operator` POR NOMBRE canónico (`Σ NodeScan`, `Σ Expand`, `Σ IndexSeek`) en vez de contadores nuevos dentro de los operadores | UNA sola verdad: los números del `--profile`, del `--stats` (cap. 31) y del `explain` (cap. 21) coinciden por construcción; derivar es una función pura de ~10 líneas | Duplicar contadores paralelos: dos fuentes que pueden divergir (el bug clásico de telemetría) | Código cap20 (`name() -> &'static str`, `collect_metrics()` en pre-orden, verificado) |
| 7 | Mini-subscriber PROPIO (`SuscriptorArbol`, ~100 líneas) implementando `tracing::Subscriber` (7 métodos requeridos + `try_close` + pila current-span para padres contextuales), árbol tipado en `RefCell<Vec<NodoSpan>>`; tests vía `with_default` | Verificación TIPO-SEGURA de la jerarquía (assert sobre nodos/padres, no sobre texto); sin estado global ni stderr; `with_default` es el mecanismo documentado para subscriber de ámbito | `tracing-subscriber` fmt con writer a `Vec<u8>` PARSEANDO la salida indentada: árbol de dependencias mayor (sharded-slab, thread_local, smallvec…) + parse frágil con timestamps incrustados; o `tracing-mock` (dev-tool de los autores, pesado para el libro) | docs.rs/tracing-core `Subscriber` (required methods: enabled/new_span/record/record_follows_from/event/enter/exit — verificado); dispatcher::with_default; parenting contextual en `Attributes::parent` |
| 8 | `wal_bytes_written` = DELTA de `Wal::as_bytes().len()`; `transactions_committed/aborted` contados en la CAPA CONDUCTORA (REPL/tests) | Verificado: `Wal` no expone contador de bytes (pero sí el total via `as_bytes`) ni de transacciones; `Transaccion/WalTransaccion::commit(self)` CONSUMEN self — envolverlos transparentemente exigiría tocar caps. 27/28. La conductora (sesion.rs) ya evolucionó en caps. 32-34: es SU trabajo contar | Contadores internos en cap27/28: modifica historia publicada; wrapper de `Transaccion`: imposible sin `Drop` hacks que confunden ownership | Código cap28 (`as_bytes/syncs/record_count`, verificado), cap27 (`commit(self)`/`rollback(self)`, verificado); honestidad como en caps. 33/34 |
| 9 | `query_duration` NO es un contador del registro: tiempos por fase con `Instant` reportados junto al árbol (herencia directa del harness del cap. 34); percentiles ya viven allí | Duración es distribución, no acumulable sin perder significado; duplicar histogramas en cap. 35 rompería «una idea nueva por sección» y el cap. 34 acaba de construir el harness | Histograma propio en `Contadores`: scope creep; el lector ya tiene percentiles a mano en cap34_benchmarks.rs | Código cap34 (`percentiles`, `ejecutar_texto/plan`); G. Tene, «How NOT to Measure Latency» (2015) — ya citado en cap. 34 |
| 10 | Frontera DECLARADA: `--profile` muestra query→plan→operador (3 niveles) en la ruta de consulta; el 4º nivel (page fetch) se demuestra en componente (índice sobre `BufferPool<FilePager>`) | No existe `GraphStore` en disco (solo `MemoryStore` de producción — verificado 2026-08-25); fingir page fetches en la ruta de consulta sería la falsedad exacta que caps. 33/34 prohibieron | Demo `--profile` contra un grafo «respaldado por pool»: requeriría escribir el DiskStore que no existe — scope masivo disfrazado de ejemplo | Código real caps. 8/12/13 (verificado); contratos cap-33 §5 y cap-34 §5.7 (frontera frío/caliente idéntica) |
| 11 | Salida de `--profile` SIN golden byte-exacto: tests ESTRUCTURALES (nombres/nesting de spans exactos + contadores exactos; duraciones sólo presencia/formato) | Los tiempos varían entre máquinas y runs: un golden con duraciones rompería CI determinista; los goldens EXISTENTES (demo/explain) deben quedar intactos — `--profile` off por defecto lo garantiza | Golden completo con máscara de tiempos: post-proceso frágil que ensaya el fallo que pretende evitar | tests/golden_cli.rs (comparación byte-exacta + `ACTUALIZAR_GOLDEN=1`, verificado); política de determinismo del workspace |
| 12 | Delimitación N.10: OpenTelemetry (OTLP), servidor Prometheus (scrape/PromQL), Grafana y propagación distribuida de trace-id NOMBRADOS, jamás implementados | LiraDB es un proceso único y local: exportar a backends añadiría dependencias y servidores sin enseñar nada nuevo sobre EL MOTOR; el capítulo entrega visibilidad LOCAL verificable (CLI + tests) | Implementar un exporter mínimo «para que se vea moderno»: dependencias de runtime + red en un capítulo cuyo valor es la instrumentación interna | opentelemetry.io (docs); Prometheus docs (exposition/scrape); Dapper §5 (sampling/colección centralizada — contexto distribuido que aquí NO aplica) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: Dapper (Sigelman et al., Google
   Technical Report dapper-2010-1, abril 2010): ¿cómo depuras una petición que cruza
   miles de máquinas? Modelarla como ÁRBOL de spans con padre explícito e
   instrumentación mínima — bajo coste, transparencia, despliegue ubicuo. Pregunta:
   ¿puedes VER qué hace LiraDB por dentro cuando ejecuta TU consulta?
2. **N.1-N.2 Objetivo/Problema**: contadores encerrados (`Executor::metrics()` sólo vía
   desenrollado; `BufferPool::metrics()` con el pool a mano) y tiempos del cap. 34 — pero
   nada muestra el VIAJE. Síntoma: «40 ms de consulta… ¿y ahora qué?».
3. **N.3 Modelo mental**: el viaje y el recibo (§4) + jerarquía dibujada + tabla
   métrica↔fuente↔vista (qué existe, qué se deriva, qué es nuevo).
4. **N.4 Primera solución**: `println!`/`eprintln!` esparcidos (log manual ingenuo) — la
   versión de todo el mundo: datos y presentación mezclados, sin padre/hijo ni niveles,
   contaminando goldens y stderr.
5. **N.5 Sus límites**: no agregan, no filtran, no se apagan; responden «pasó esto» pero
   nunca «¿qué parte?»; el ruido escala peor que el código.
6. **N.6 Solución evolucionada**: `Contadores` std + medidores decorador (lib) → spans de
   fase y operador en la CLI → `SuscriptorArbol` captura → `--profile` compone itinerario
   + recibo. Cada pieza con su test antes de apilar la siguiente.
7. **N.7 Código completo ejecutable**: `cap35_observabilidad.rs` y
   `liradb-cli/src/observabilidad.rs` por `include::` (nunca duplicados) + diff de
   Cargo.toml (una línea: `tracing = "0.1"` en la CLI).
8. **N.8 Prueba de fuego**: `liradb query --profile '...'` con salida REAL pegada; test
   estructural del árbol en verde; escenario de componente con los 4 niveles sobre pool;
   un `NodeScan` sin índice SALTA a la vista comparando dos `--profile`.
9. **N.9 Qué hemos sacrificado**: sin exportación a backend (stdout únicamente), sin
   sampling (Dapper muestrea; aquí todo se captura), spans síncronos monohilo (`Cell`, no
   `AtomicU64`: honestidad sobre concurrencia futura), page fetch ausente de la ruta de
   consulta (frontera), correlación log↔traza nombrada no hecha.
10. **N.10 Cómo lo hace una BBDD real + retos**: PostgreSQL (`EXPLAIN (ANALYZE, BUFFERS)`,
    `pg_stat_statements`), MySQL performance_schema, Neo4j (query log + endpoint
    Prometheus), stacks OpenTelemetry+Grafana; retos esencial (span para `FilterOp`
    reutilizando `OperadorTrazado`), intermedio (diagnosticar con `--profile` una consulta
    que escanea cuando debería hacer seek), experto (snapshots por intervalo con deltas
    estilo rate de Prometheus + consistencia contra `ExecMetrics`).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla / 30 segundos /
    Una historia pequeña / Mini-diálogo de guardia nocturna (3 a.m.: el recibo dice
    `nodes_scanned=500_000`; el itinerario señala el `Expand` del hub). Retrieval
    practice: dibujar DE MEMORIA la jerarquía de 4 niveles y qué pregunta responde un span
    vs un contador. Interleaving: 13+35 (hit ratio junto al árbol), 21+35 (explain vs
    traza), 26+35 (generalizar el voltímetro), 28+35 (delta del WAL), 34+35 (percentiles
    junto a spans). Glosario nuevo: observability, traza (trace), span, contador
    (counter), registro (registry), subscriber, exposición de métricas (metrics
    exposition), decorador (decorator), capa conductora, telemetría.
12. **Gancho de cierre (preguntas abiertas)**: todo puerto del motor ya se puede MEDIR y
    VER — ¿cómo es el MAPA COMPLETO de lo construido? Cap. 36: arquitectura final de
    LiraDB, el círculo hexagonal cerrado con puertos medidos.

---

## 7. Estilo y tono (consistencia con caps. 27-34)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (observability, trace, span, counter, registry, subscriber,
  exposition format, decorator); salidas REALES de `--profile` pegadas con hardware/SO/
  toolchain declarados, nunca reconstruidas de memoria.
- **Diagramas**: el árbol de spans con su recibo lateral (§4), el bloque «lo que sí /
  lo que aún no» y la tabla métrica↔fuente↔vista.
- **Spacing** (conceptos viejos que se EJERCITAN): puerto hexagonal y `ContandoStore`
  (caps. 8/26), `FilePager` (cap. 12), métricas del BufferPool (cap. 13), `ExecMetrics`
  y nombres canónicos (cap. 20), ruta `IndexSeek` (cap. 21), WAL/transacciones
  (caps. 27-28), CLI y goldens (cap. 31), harness de tiempos (cap. 34).
- **Interleaving**: reto esencial 20+35 (span nuevo reutilizando el envoltorio);
  intermedio 21+35 (scan-vs-seek con la traza); experto 13+28+35 (deltas por intervalo);
  retrieval obliga a reconstruir la jerarquía SIN mirar el capítulo.
- **Dificultad asimétrica**: una idea nueva por sección (modelo viaje/recibo → registro →
  medidores → spans de fase → operador → página/frontera → subscriber → hito); ejercicios
  que exigen PREDECIR (¿qué span será el más ancho aquí?) y recordar sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap35` y
  `cargo test -p liradb-cli --test observabilidad_cli` (milisegundos); ALL_GREEN como
  puerta con los goldens intocables como prueba de aditividad. Nunca «confía en mí».
- **Anécdota (única, verificada)**: Dapper — Sigelman, Barroso, Burrows, Stephenson,
  Plakal, Beaver, Jaspan y Shanbhag, «Dapper, a Large-Scale Distributed Systems Tracing
  Infrastructure», Google Technical Report dapper-2010-1 (abril 2010): árboles de trazas,
  spans con padre, anotaciones; <1% de overhead y despliegue ubicuo como claves del éxito.
  DESCARTADA la alarma 1202 del Apollo AGC (1969) — es monitorización/alertas más que
  trazas; reservada para un capítulo futuro. Apoyo: docs.rs/tracing y tracing-core;
  docs.rs/metrics (facade); Prometheus exposition formats; PostgreSQL `EXPLAIN`.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5: paper Dapper, docs tracing/tracing-core/metrics, Prometheus
  exposition formats, código real del workspace verificado).
- [x] Escenario de fallo visible: log-ingenuo que contamina goldens/stderr (N.4-N.5),
  consulta que escanea y se delata en el árbol (N.8), frontera page-fetch declarada no
  fingida (§5.10), `--profile` NO-golden por ser temporal (§5.11).
- [x] Código ejecutable citado por nombre (IMPLEMENTADO 2026-08-25:
  `cap35_observabilidad.rs` (1.045 líneas), `liradb-cli/src/observabilidad.rs`
  (842), flag `--profile` (+35/−4 en lib.rs de la CLI),
  `tests/observabilidad_cli.rs` (363; 5 tests integración con los nombres de
  §2) — **843 tests** = 822 + 21, ALL_GREEN, goldens intactos, tracing
  0.1.44 solo en la CLI. Divergencias documentadas por el implementador:
  SuscriptorArbol con Mutex+AtomicU64 (no RefCell: Dispatch exige Send+Sync);
  recibo SOLO derivado de ExecMetrics — la v1 contaba DOS veces
  (nodes_scanned 8 vs 4: el bug «dos fuentes que se suman», material N.9);
  sin fase optimise en el hito (pipeline_con_detalle nunca la tuvo; el span
  optimise queda verificado en el test de jerarquía); transacciones contadas
  en tests-patrón de conductora sin cablear el REPL. Salida REAL del hito
  capturada (árbol query→parse/plan/execute→Project→Filter→Expand→NodeScan +
  recibo Prometheus-format).
- [x] Misconcepciones corregidas explícitamente (§1: cuatro).
- [x] Ejercicios con solución verificable diseñados (retos N.10; nombres de tests
  previstos en §2/N.10).
- [x] ≥1 ejercicio de retrieval (jerarquía de 4 niveles de memoria) y spacing
  planificado (caps. 8/12/13/20/21/26/27/28/31/34 tocados; §7).
- [x] Responde la pregunta crítica del CORPUS (jerarquía revelada, capturada e impresa;
  nivel página con frontera honesta) y recoge el gancho del cap. 34 («de MEDIR a VER»,
  §3 y §6.12).
- [x] Anécdota única verificada con fuente primaria (Google Technical Report
  dapper-2010-1; autores en research.google.com/pubs/pub36356) — Apollo 1202 descartada.
- [x] Alcance de código nuevo acotado y honesto (un módulo std en la lib sin deps +
  un módulo y un flag en la CLI + UNA dependency nueva solo en la CLI; cero cambios en
  los módulos cap* de caps. 7-34; goldens intactos por diseño; §5.3/§5.8/§5.11).
- [x] Gancho saliente al cap. 36 fijado (arquitectura final: mapa completo, puertos
  medidos, círculo hexagonal cerrado; §6.12).
