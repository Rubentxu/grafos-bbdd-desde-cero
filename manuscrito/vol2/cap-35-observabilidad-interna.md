# Capítulo 35 — Observabilidad interna

> *«El capítulo anterior te enseñó a MEDIR. Este hace visible el VIAJE: un recibo de contadores que dice cuánto costó la respuesta y un árbol de spans que dice dónde se fue el tiempo. Porque un número sin itinerario deja la pregunta a medias: ¿qué PARTE fue lenta?»*

## 35.0 La anécdota de la esquina

A mediados de los dos mil, en Google, una petición de búsqueda atravesaba rutinariamente cientos o miles de máquinas. Cada servicio tenía sus contadores, y aun así nadie sabía responder con confianza la pregunta más simple: ¿por qué ESTA petición tardó tanto, y en qué parte del viaje se fue el tiempo? En abril de 2010, Benjamin Sigelman, Luiz André Barroso, Mike Burrows y sus colegas publicaron la respuesta que industrializó el campo: «Dapper, a Large-Scale Distributed Systems Tracing Infrastructure» (Google Technical Report dapper-2010-1). La idea cabe en una frase: modelar la ejecución como un **árbol de trazas** (*trace*) donde cada **span** lleva nombre, identificador propio, identificador del PADRE y duración. El árbol entero responde «quién llamó a quién y cuánto tardó cada tramo» — sin importar cuántas máquinas crucen de camino.

Tres decisiones de diseño explican que Dapper llegara a producción y no se quedara en papel. Primero, transparencia: la instrumentación se escondió en las librerías comunes — hilos, control de flujo, RPC — para que ninguna aplicación tuviera que tocarse. Segundo, coste: el sobrecoste quedó por debajo del 1 % para las peticiones muestreadas y despreciable para las demás, porque emitir telemetría que nadie va a leer debe ser casi gratis. Tercero, ubiquidad: solo un sistema barato puede estar SIEMPRE encendido, y lo que no está siempre encendido nunca captura el incidente que importa. La moraleja es la tesis de este capítulo: el árbol no se INVENTA al depurar — ya estaba latente en la ejecución; Dapper le puso nombre, padre y duración. Eso funciona para miles de máquinas y también para una sola consulta de LiraDB: `query → parse → plan → execute → operador → página`.

## 35.1 Objetivo

El capítulo anterior cerró su diálogo nocturno con una promesa: ya sabes MEDIR; te falta hacer la medición VISIBLE. Al terminar tendrás:

1. **El recibo**: `crates/vol2-liradb/src/cap35_observabilidad.rs` (1.045 líneas, std puro — el crate sigue *dependency-free* como veintiocho capítulos antes). El registro **`Contadores`** con campos fijos nombrados exactamente como la métrica (`queries_total`, `nodes_scanned`, `relationships_expanded`, `index_hits`, `page_reads`, `page_writes`, `wal_bytes_written`, `transactions_committed`, `transactions_aborted`), mutabilidad interior con `Cell<u64>` (el patrón de `ContandoStore` del cap. 26), foto inmutable vía `snapshot()` y un `Display` que imita el formato de texto de Prometheus. Más los dos **decoradores** (*decorator*): `MedidorOperador` sobre `PhysicalOperator` y `MedidorPaginas<P>` sobre `Pager`.
2. **El itinerario**: `crates/vol2-liradb-cli/src/observabilidad.rs` (842 líneas). El mini-subscriber **`SuscriptorArbol`** (~100 líneas sobre el trait `tracing::Subscriber`: sus siete métodos requeridos más `try_close` sobrescrito), los envoltorios apilables `OperadorTrazado`/`PagerTrazado`, el render `arbol_indentado` y `pipeline_perfilada` — el corazón del hito.
3. **El flag**: `--profile` junto a `--plan`/`--stats` (+35/−4 líneas en `src/lib.rs`) — ADITIVO, off por defecto, goldens intocables. Y `tracing = "0.1"`: la ÚNICA dependency nueva del capítulo, SOLO en la CLI.
4. **Las pruebas**: 11 tests en la lib + 5 unitarios de la CLI + `tests/observabilidad_cli.rs` (363 líneas, 5 de integración) = **21 nuevos**, 822 + 21 = **843 ALL_GREEN** con los dorados byte-exactos intactos.

Y una tesis que lo vertebra: **métricas y trazas son dos vistas complementarias del mismo viaje**. Los contadores (*counters*) dicen CUÁNTO trabajo costó la respuesta; las trazas dicen DÓNDE y en qué ORDEN se gastó. Ningún sistema de observabilidad real separa ambas — LiraDB tampoco.

## 35.2 Problema

En realidad, ya tienes 843−21 tests verdes, benchmarks con percentiles y `ExecMetrics` por operador. Y aun así, ante «esta consulta tarda 40 ms… ¿y ahora qué?», callas. Las cifras existen pero están ENCERRADAS: `Executor::metrics()` solo sale por el desenrollado manual del pipeline, `BufferPool::metrics()` exige tener el pool a mano, y nada muestra el VIAJE. Antes de construir la salida, desactivemos las cuatro ideas equivocadas que suelen venir con ella:

1. **«Observar es repartir `println!` por el código.»** No: el texto plano mezcla datos y presentación, no distingue módulos ni niveles, contamina stderr y — peor en este repo — contaminaría los goldens byte-exactos del cap. 33.
2. **«Métricas contra trazas: elige una.»** Falsa disyuntiva: responden preguntas DISTINTAS y complementarias. Sin contadores no sabes cuánto costó; sin trazas no sabes dónde. Un motor con una sola de las dos está medio ciego.
3. **«Instrumentar exige tocar el motor.»** No: store, pager y operador son TRAITS públicos (caps. 8/12/20). Un decorador que envuelva el trait mide sin cambiar una línea del motor — `ContandoStore` ya lo demostró en el cap. 26.
4. **«Un span es un log bonito.»** No: un span tiene nombre, PADRE y duración. Muchos spans forman un ÁRBOL causal. Sin jerarquía no existe respuesta a «¿qué parte de mi consulta fue lenta?».

Y debajo de todo, la pregunta crítica del CORPUS: **¿cómo se ve la jerarquía `query → plan → operator → page fetch`?** Respuesta corta: esa jerarquía YA existe latente en el pipeline que la CLI desenrolla desde el cap. 31. Este capítulo la NOMBRA con spans anidados, la CAPTURA con un subscriber propio y la IMPRIME con `liradb query --profile '<LiraQL>'`.

## 35.3 Modelo mental

Piensa en un viaje con dos documentos: el ITINERARIO (por dónde pasaste y cuánto tardó cada tramo) y el RECIBO de peajes (cuánto costó el total, desglosado). La TRAZA es el primero; los CONTADORES son el segundo. La jerarquía del itinerario para LiraDB:

```text
span «query» ──────────────────────────────── raíz (1 por consulta)     CONTADORES (el recibo)
 ├─ span «parse»       LiraQL → AST           queries_total           = 1 por pasada
 ├─ span «plan»        AST → LogicalPlan      nodes_scanned           = Σ NodeScan    ← derivado
 ├─ (span «optimise»)  reglas del cap. 21     relationships_expanded  = Σ Expand      ← derivado
 └─ span «execute»     Volcano (cap. 20)      index_hits              = Σ IndexSeek   ← derivado
     └─ span «Project»                        page_reads/page_writes  ← pager medido (caps. 12/13)
         └─ span «Filter»                     wal_bytes_written       = Δ Wal::as_bytes().len()
             └─ span «Expand»                 transactions_{committed,aborted} ← capa conductora
                 └─ span «NodeScan»
                     └─ span «storage_read»   ← SOLO si hay pager detrás del puerto
```

Cada métrica del recibo tiene UNA fuente y un instrumento — la tabla que ordena todo el capítulo:

| Métrica | Fuente única | Instrumento que la captura |
|---|---|---|
| `nodes_scanned` / `relationships_expanded` / `index_hits` | Σ de `ExecMetrics.per_operator` POR nombre canónico (cap. 20) | derivación pura (`metricas_consulta`) |
| `page_reads` / `page_writes` | `read()`/`write()` OK del pager envuelto | `MedidorPaginas` ≡ `pool.metrics()` (composición, cap. 13) |
| `wal_bytes_written` | delta de `Wal::as_bytes().len()` (cap. 28) | la capa conductora |
| `transactions_committed` / `aborted` | resultado de `commit`/`rollback` consumidos (cap. 27) | la capa conductora |
| `queries_total` | una pasada del pipeline | la capa conductora |
| duración por fase | `Instant` antes/después (herencia del cap. 34) | tabla «Fases» del `--profile` |
| jerarquía causal | parenting contextual del propio pipeline | `SuscriptorArbol` |

Y debajo de la tabla, la REGLA DE ORO heredada del cap. 26: todo punto de medida es un DECORADOR sobre un puerto existente (`GraphStore`, `Pager`, `PhysicalOperator`). El motor no sabe que lo observan; apilar decoradores (medidor dentro, span fuera) compone vistas sin acoplarse.

```text
Lo que SÍ se ve hoy:  query → plan → operador en la CLI (--profile); nivel página COMPLETO
                      a nivel componente (índice sobre pool); contadores de consulta, pool,
                      WAL (delta) y transacciones
Lo que AÚN NO:        page fetch EN la ruta de consulta (espera DiskStore tras el puerto);
                      exportación a backends externos (Prometheus/OpenTelemetry: §35.10)
```

El momento ¡ajá!: instrumentar no es tocar el motor, es ENVOLVER sus puertos — y la jerarquía de spans no se inventa, SE REVELA: ya estaba en el pipeline. El ¡ajá! honesto: mientras tus consultas vivan en RAM, el cuarto nivel no aparece en la ruta — decirlo ES parte del tema.

## 35.4 Primera solución

La versión ingenua es la que todo el mundo escribe — incluido nuestro yo de ayer:

```rust
let query = match parse(src) {
    Ok(q) => q,
    Err(e) => { eprintln!("[lira] ERROR parseando: {e}"); return Err(e); }
};
eprintln!("[lira] ok, bajando plan…");
// …treinta líneas más abajo, otra vez:
eprintln!("[lira] scan terminado, {} filas", n);
```

Compila, corre y «te informa». Es la versión de todo el mundo, y por eso merece un análisis serio antes de enterrarla.

## 35.5 Sus límites

1. **Mezcla datos y presentación.** Cada mensaje decide a mano qué imprimir y con qué formato: mañana querrás agregados, JSON o un dashboard, y tendrás que reescribir treinta cadenas repartidas por el código.
2. **No hay jerarquía.** «scan terminado» no dice QUIÉN lo llamó ni DESDE dónde. Los mensajes responden «pasó esto», jamás «¿qué parte del total fue?» — que era la pregunta.
3. **No se filtran ni se apagan.** Sin niveles ni filtros, el ruido escala peor que el código: en producción, cada `eprintln!` es escritura sincronizada a stderr que pagas aunque nadie escuche.
4. **Contamina el contrato con la shell.** La regla del cap. 31 — datos a stdout, diagnósticos a stderr — se rompe con el primer `println!` descuidado. Y el cap. 33 doró `demo.txt` y `explain.txt` byte-exactos: un solo log nuevo en el camino rompe `golden_demo_coincide` a las tres de la madrugada.
5. **No agrega.** Diez consultas, diez chorros de texto. ¿Cuántos nodos escanearon EN TOTAL? Lee tú los diez logs y súmalos a mano. Un contador es un acumulador; un `println!` es una confesión efímera.

## 35.6 Solución evolucionada

### El recibo: Contadores a mano

Un registro (**registry**) con campos FIJOS nombrados igual que la métrica, mutabilidad interior `Cell<u64>` y `snapshot()` que devuelve `SnapshotContadores` — un struct plano copiable: la foto se congela aunque el registro siga vivo. Por qué campos fijos y no `HashMap<&'static str, u64>`: el typo en una clave de mapa COMPILA y miente en runtime; aquí el compilador es el guardián, el orden del `Display` es el de declaración (determinismo) y no hay hashing en el camino caliente. Por qué `Cell` y no atómicos: el motor es monohilo — lo garantiza el borrow checker desde el cap. 27 — y fingir concurrencia sería deshonestidad. El crate industrial `metrics` queda DOCUMENTADO, no integrado: es una facade con recorder GLOBAL (`set_global_recorder`) — estado oculto e invisible frente a nuestro struct local pasable por `&`; pedagógicamente, quieres ver dónde vive el estado.

El `Display` imita el formato de texto de Prometheus (`exposition format`): una línea `# TYPE x counter` y otra `x valor` por métrica, en orden fijo. Se IMITA, no se depende: aprender el formato escribiéndolo a mano enseña qué es — pares nombre/valor legibles sin más — sin pagar exporter alguno. El test `contadores_display_formato_texto_y_snapshot_exacto` clava las dieciocho líneas esperadas byte a byte, y `contadores_campos_fijos_sin_mapa_sin_typos` demuestra que añadir una métrica obliga a tocar tres puntos visibles en el diff (struct, incrementos, Display) — nunca una clave perdida en un mapa.

### Derivar, no duplicar: UNA sola verdad

`nodes_scanned`, `relationships_expanded` e `index_hits` NO tienen contadores propios dentro de los operadores: se DERIVAN sumando `ExecMetrics.per_operator` (cap. 20) por nombre canónico — `metricas_consulta(&ExecMetrics) -> (u64, u64, u64)` y `derivar_contadores` que vuelca al registro. Función pura de ~10 líneas con una consecuencia enorme: los números del `--profile`, del `--stats` (cap. 31) y del `explain` (cap. 21) coinciden POR CONSTRUCCIÓN, porque todos salen del mismo `collect_metrics()`. Duplicar contadores paralelos es el bug clásico de telemetría: dos fuentes que divergen en silencio — y este capítulo lo sufrió en carne propia (§35.9). El test `metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico` compara la derivación contra la suma hecha a mano: si el cap. 20 renombrara `NodeScan`, el test gritaría en vez de bajar a cero callando.

### Los voltímetros generalizados: MedidorOperador y MedidorPaginas

El precedente exacto es `ContandoStore` (cap. 26): inner + `Cell<u64>`, solo lectura compartida. La generalización envuelve los otros dos puertos. `MedidorOperador` implementa el MISMO trait `PhysicalOperator`, delega open/next/close/name/rows_produced/collect_metrics sin tocar nada, y cuenta llamadas a `next()`, filas vistas y tiempo en contadores LOCALES — las filas van además al registro compartido según el nombre canónico del operador envuelto. `MedidorPaginas<P>` hace lo propio con `Pager`: reads/writes/syncs OK y bytes movidos (operaciones × tamaño de página — contrato del trait del cap. 12: el buffer SIEMPRE mide exactamente una página). Composición probada: un pool sobre el medidor cuenta la misma realidad que `BufferPool::metrics()` por otro camino (`medidor_paginas_compone_con_buffer_pool`).

### El itinerario: spans de fase y de operador

Aquí entra `tracing` — la ÚNICA dependency nueva, SOLO en la CLI; `vol2-liradb` no paga nada. Dos piezas. Primero, spans de FASE: `pipeline_perfilada` rodea el desenrollado con `info_span!("query")` como raíz y `parse`/`plan`/`execute` como hijos directos — los macros se COMPILAN siempre, y emitir sin subscriber instalado cuesta un chequeo barato de interés del **callsite** (una carga atómica y una bifurcación): separar «instrumentar» de «activar» es la decisión de producción, porque cualquier subscriber futuro verá los spans sin recompilar. Segundo, spans de OPERADOR: `OperadorTrazado` implementa `PhysicalOperator`, delega en el operador real y crea su span en `open()` con el NOMBRE CANÓNICO (`NodeScan`, `IndexSeek`, `Expand`…) — la traza habla el idioma del `explain` del cap. 21. El padre contextual lo pone quien esté activo en ese momento: el árbol físico emerge SOLO del parenting contextual, sin pasar padres a mano. `PagerTrazado` emite `storage_read` por cada `read()` del trait `Pager` — el cuarto nivel, listo para cuando haya disco tras el puerto.

Fíjate en el APILADO: medidor dentro, span fuera. `OperadorTrazado` envuelve un `MedidorOperador` que envuelve el operador real — cada pieza independiente y testeable sola (`operador_trazado_apila_span_sobre_medidor_y_delega_todo`).

### Capturar el árbol: SuscriptorArbol

Un subscriber (*subscriber*) es quien RECIBE los spans; sin él, todo se pierde silenciosamente. `SuscriptorArbol` implementa los siete métodos requeridos del trait `tracing::Subscriber` y graba `(id, NodoSpan{nombre, padre, inicio, duración})` — tipado a propósito: los tests afirman sobre NODOS y PADRES, jamás parseando texto indentado. Tres detalles que valen un capítulo:

- **Padres contextuales**: si `Attributes::parent()` trae id explícito, gana; si viene vacío (lo normal), el padre es el TOPE de la pila de spans actuales, mantenida con `enter`/`exit`. Sin esa pila, todos colgarían de la raíz y el árbol sería mentira.
- **`try_close` SOBRESCRITO**: los métodos por defecto NO notifican el cierre — sin override, ninguna duración existiría jamás. Sobrescribirlo es la diferencia entre tener duraciones y creerlas.
- **`Mutex` + `AtomicU64` en vez de `RefCell`**: `dispatcher::with_default` — el mecanismo documentado para subscriber de ámbito, usado por TODOS los tests de captura — exige `Send + Sync` en el subscriber (la firma de `Dispatch::new`). En monohilo la semántica es idéntica; el lock sin contención cuesta nanosegundos.

### El hito aditivo: --profile

`--profile` instala el subscriber, cronometra fases con `Instant` (herencia directa del harness del cap. 34) e imprime resultado + fases + árbol + recibo. Aditivo por diseño: off por defecto, la salida sin él es EXACTAMENTE la de siempre, y `perfil_aditivo_goldens_demo_explain_intactos` re-verifica los dorados byte-exactos por si alguien ensuciara la ruta. La salida del `--profile` NO es golden — los tiempos varían entre máquinas — así que lo pactado son nombres, nesting y CONTADORES exactos (§35.8).

## 35.7 Código completo ejecutable

Todo vive en tres piezas que puedes leer de corrido: `src/cap35_observabilidad.rs` en la lib y `src/observabilidad.rs` en la CLI, más el flag en `lib.rs`. Las firmas que lo sostienen:

```rust
// crates/vol2-liradb/src/cap35_observabilidad.rs (std puro)
pub struct Contadores { /* 9 campos Cell<u64>, nombres fijos */ }
impl Contadores {
    pub fn new() -> Self;
    pub fn snapshot(&self) -> SnapshotContadores;      // foto copiable
    pub fn sumar_wal_bytes(&self, delta: usize);       // Δ Wal::as_bytes().len()
    pub fn contar_commit(&self);                       // los llama la conductora
    pub fn contar_rollback(&self);
}
pub fn metricas_consulta(m: &ExecMetrics) -> (u64, u64, u64); // Σ por nombre canónico
pub fn derivar_contadores(m: &ExecMetrics, c: &Contadores);

pub struct MedidorOperador<'a> { /* inner + &Contadores + Cell locales */ }
impl PhysicalOperator for MedidorOperador<'_> { /* delega TODO, cuenta por nombre */ }

pub struct MedidorPaginas<'a, P> { /* inner: P + &Contadores + Cell locales */ }
impl<P: Pager> Pager for MedidorPaginas<'_, P> { /* delega TODO, cuenta bytes */ }
```

```rust
// crates/vol2-liradb-cli/src/observabilidad.rs
pub struct SuscriptorArbol { /* AtomicU64 + Mutex<Vec<(u64, NodoSpan)>> + pila */ }
impl Subscriber for SuscriptorArbol {
    /* enabled · new_span · record · record_follows_from · event · enter · exit
       + try_close SOBRESCRITO (los defaults no notifican el cierre) */
}
pub fn arbol_indentado(sub: &SuscriptorArbol) -> String;

pub struct OperadorTrazado<'a> { inner: Box<dyn PhysicalOperator + 'a>, span: Option<tracing::Span> }
pub struct PagerTrazado<P> { inner: P }            // span storage_read POR read()

pub fn pipeline_perfilada(src: &str, store: &dyn GraphStore,
                          out: &mut dyn Write, plan: bool, stats: bool)
    -> Result<(), ExecError>;                      // EL hito --profile
```

El corazón del subscriber, el método que convierte «pasó» en «tardó»:

```rust
fn try_close(&self, id: Id) -> bool {
    let raw = id.into_u64();
    let mut nodos = self.nodos.lock().expect("sin envenenamiento");
    if let Some((_, nodo)) = nodos.iter_mut().find(|(i, _)| *i == raw)
        && nodo.duracion.is_none()
    {
        nodo.duracion = Some(nodo.inicio.elapsed());
    }
    true
}
```

Y el cableado completo en `Cargo.toml`:

```toml
# crates/vol2-liradb-cli/Cargo.toml — LA única dependency nueva del capítulo
tracing = "0.1"   # spans; SOLO en la CLI: vol2-liradb sigue dependency-free
```

Fíjate en lo que NO hay: ni una línea cambiada en los módulos `cap*` de caps. 7-34; cero dependencias nuevas en la lib (los medidores son std); ningún estado global (los tests capturan con `with_default`, thread-local de ámbito); y ningún golden nuevo con tiempos dentro.

## 35.8 Prueba de fuego

Primero el bucle rápido, en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap35
$ cargo test -p liradb-cli --test observabilidad_cli
```

Dieciséis tests en verde: en la lib, `contadores_display_formato_texto_y_snapshot_exacto`, `contadores_campos_fijos_sin_mapa_sin_typos`, `metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico`, `derivacion_cubre_expand_e_indexseek_del_pipeline_real`, `medidor_operador_cuenta_llamadas_filas_y_tiempo`, `medidor_operador_en_arbol_coincide_con_exec_metrics`, `medidor_operador_alimenta_relationships_expanded_desde_texto`, `medidor_paginas_cuenta_reads_writes_y_bytes_movidos`, `medidor_paginas_compone_con_buffer_pool`, `wal_bytes_escritos_delta_tras_commit_waltransaccion` y `transacciones_committed_aborted_contadas_en_conductora`; en la CLI, `suscriptor_arbol_jerarquia_query_plan_optimise_execute`, `perfil_cli_arbol_indentado_spans_operador`, `perfil_contadores_exactos_y_fases_cronometradas`, `perfil_aditivo_goldens_demo_explain_intactos` y `jerarquia_cuatro_niveles_componente_indice_sobre_pool`. Ejecútalo tú: el tiempo exacto no importa para nada.

Ahora el hito. Hardware declarado como en el cap. 34 (Xeon E5-2682 v4 @ 2,50 GHz, Linux, rustc 1.96.0; binario en perfil dev de `cargo run`) — las duraciones VARÍAN entre máquinas y ejecuciones; lo pactado son nombres, anidamiento y contadores:

```text
$ cargo run -q -p liradb-cli -- query --profile 'MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name, r.since'
Resultado:
f.name | r.since
"Bo"   | 2020
Perfil (cap. 35):
Fases:
  parse         40.7 µs
  plan          22.3 µs
  execute       72.3 µs
Árbol de spans:
query                           141.7 µs
├─ parse                         39.4 µs
├─ plan                          21.1 µs
└─ execute                       71.1 µs
   └─ Project                    53.7 µs
      └─ Filter                  51.5 µs
         └─ Expand               50.0 µs
            └─ NodeScan          47.1 µs
Contadores:
# TYPE queries_total counter
queries_total 1
# TYPE nodes_scanned counter
nodes_scanned 4
# TYPE relationships_expanded counter
relationships_expanded 4
... (index_hits/page_reads/page_writes/wal_bytes_written/transactions_committed/transactions_aborted = 0)
Métricas: (usa --stats para el detalle)
```

Tres lecturas que valen el capítulo. **Primera: el recibo y el árbol cuentan LA MISMA historia por caminos distintos.** El recibo deriva `nodes_scanned=4` de `ExecMetrics.per_operator`; el árbol muestra el `NodeScan` colgando bajo `execute` — y ambos dicen lo mismo: para encontrar a Ana, hoy se recorrieron las cuatro Person y se expandieron las cuatro aristas KNOWS. Corre la MISMA consulta por `explain` (cap. 21) y verás lo que el optimizador HARÍA: `IndexSeek` directo. El perfil enseña lo que SU pipeline compiló — y ahí, el scan SALTA A LA VISTA: cuatro nodos escaneados para devolver una fila es trabajo que un índice evitaría. **Segunda: las duraciones están ANIDADAS, no disjuntas.** El `Filter` tarda 51,5 µs filtrando… cuatro filas. Claro: es Volcano pull-based (cap. 20) — su tiempo CONTIENE al del `Expand` (50,0) que contiene al del `NodeScan` (47,1). El span más ancho no es necesariamente el culpable; hay que leer quién contiene a quién. **Tercera: los ceros honestos.** `page_reads=0`, `wal_bytes_written=0`, transacciones a cero — la consulta no toca disco ni WAL porque vive en RAM, y el recibo lo IMPRIME en lugar de esconderlo: la frontera del capítulo, tal cual.

¿Y el cuarto nivel? A nivel COMPONENTE, completo: `jerarquia_cuatro_niveles_componente_indice_sobre_pool` monta un `HashIndex` REAL (cap. 15) sobre `BufferPool<PagerTrazado<MedidorPaginas<FilePager>>>` (caps. 12+13) en fichero temporal, y traza `query → execute → index_seek → storage_read`. Dos trucos documentados en el propio test: `HashIndex::create` exige capacidad ≥ 3+num_buckets (su flush final toca todas las páginas primarias — con menos frames, `UnknownPage`); y UN solo bucket con 2.000 claves contra un pool de 4 frames fabrica una cadena de desborde larga que fuerza fetches REALES. El cierre es la coherencia triple que da título al capítulo: delta de `page_reads` del medidor == nº de spans `storage_read` == delta de lecturas del pool. Medidor y traza cuentan LA MISMA realidad.

## 35.9 Qué hemos sacrificado

1. **Sin exportación a backend.** Todo sale por stdout. OTLP de OpenTelemetry, scrape de Prometheus y dashboards de Grafana se NOMBRAN en §35.10 y no se implementan: exportar añadiría dependencias y red a un capítulo cuya enseñanza es la instrumentación interna.
2. **Sin sampling.** Dapper muestrea para sostener <1 % en escala Google; aquí se captura TODO, porque el dataset es pequeño y el determinismo pedagógico manda. Migrar a sampling es política del subscriber, no cambio estructural.
3. **Registro monohilo.** `Cell<u64>`, no atómicos: honestidad sobre concurrencia futura. (El subscriber sí usa `AtomicU64`/`Mutex` — porque `Dispatch` exige `Send + Sync`, no porque haya hilos.)
4. **Page fetch ausente de la ruta de consulta.** Sin DiskStore tras el puerto (frontera heredada de caps. 33/34), el cuarto nivel se demuestra en componente. Fingirlo habría sido la falsedad exacta que este libro prohíbe.
5. **El hallazgo estrella: dos fuentes que se suman.** La primera versión del recibo contaba DOS veces. Los `MedidorOperador` del pipeline alimentaban el registro EN VIVO fila a fila, y al terminar `derivar_contadores` sumaba las MISMAS filas desde `ExecMetrics`: `nodes_scanned=8` con cuatro nodos reales. Ni motor roto ni datos falsos — dos fuentes correctas SUMÁNDOSE. La solución fue quirúrgica y de principio: UNA sola verdad — el recibo del `--profile` se DERIVA siempre de `ExecMetrics`, `compilar_perfilado` deliberadamente NO apila medidores (está escrito en su doc comment), y los medidores componen donde no hay derivación. El test `medidor_operador_en_arbol_coincide_con_exec_metrics` vigila que cuando ambos caminos existan, den el mismo número. Moraleja para toda carrera: duplicar una medida no es redundancia — es divergencia futura garantizada.
6. **Correlaciones pendientes.** Log↔trace-id nombrada no hecha; transacciones contadas en tests-patrón de conductora sin cablear el REPL; y el span `optimise` existe y está testado aunque el pipeline del hito aún no corre fase de optimización — el vocabulario completo ya está, la fase llega cuando el pipeline la tenga.

## 35.10 Cómo lo hace una BBDD real

Nada de lo que hiciste es exótico: es la versión artesanal de industria madura. **PostgreSQL** es el ejemplo canónico de «perfilado de consultas nativo»: `pg_stat_statements` acumula tiempos y filas por consulta en producción (nuestros `ExecMetrics` con décadas de rodaje) y `EXPLAIN (ANALYZE, BUFFERS)` ejecuta el plan real devolviendo tiempo por operador Y contadores de páginas — hits y reads — que es EXACTAMENTE nuestra pareja recibo+itinerario en un solo comando. **MySQL** ofrece `performance_schema` y el slow query log; los catálogos del sistema (`pg_stat_*`, `information_schema`) son el registro de contadores hecho tablas. En grafos, Neo4j publica métricas vía endpoint Prometheus y query logging configurable. El stack industrial completo une lo nuestro: **Prometheus** hace SCRAPE de endpoints que exponen el formato de texto que nuestro `Display` imita a propósito (un servidor real parsearía nuestro recibo tal cual), **Grafana** dibuja los dashboards que el diálogo del cap. 34 echaba de menos, y **OpenTelemetry** unifica traces, metrics y logs en un protocolo (OTLP) con la misma anatomía de Dapper: spans con padre, propagación de contexto, exportación desacoplada. La diferencia entre LiraDB y ellos no es conceptual: es escala, sampling y red — las tres cosas que un proceso único y local no necesita todavía.

Brendan Gregg ya te lo dijo en el cap. 34: los bloques anchos SON el perfil; aquí aprendiste a hacer que el motor mismo dibuje sus bloques.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: añade un contador nuevo al registro (por ejemplo `filters_applied`) siguiendo la regla de los tres puntos: campo en `SnapshotContadores`, incremento, línea en el `Display` — y extiende `contadores_display_formato_texto_y_snapshot_exacto` con sus DOS líneas nuevas. Verifica que el typo imposible sigue imposible: intenta escribir mal el nombre en UNO solo de los tres sitios.
- *Intermedio* (20+35): envuelve un `FilterOp` con `OperadorTrazado` bajo un span `execute` creado con `capturar`, y afirma el nesting EXACTO (`execute` → `Filter`) y la duración presente, como hace `operador_trazado_apila_span_sobre_medidor_y_delega_todo`. Predice ANTES cuántos spans tendrá el árbol.
- *Experto*: extiende `SuscriptorArbol` para registrar eventos (`event`) o para acumular duraciones por nombre de span y exponer el p99 con `percentiles` del cap. 34 — o deriva una métrica nueva desde `ExecMetrics` (Σ `Filter`, por ejemplo) con su test de consistencia contra la suma manual, al estilo de `metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico`.

## 35.11 Lo que te llevas

- **Dos vistas, un viaje**: el recibo (contadores) dice CUÁNTO; el itinerario (árbol de spans) dice DÓNDE y EN QUÉ ORDEN. Juntas convierten «va lento» en «esto exacto va lento».
- **Instrumentar es envolver puertos, no tocar el motor**: `MedidorOperador`, `MedidorPaginas`, `OperadorTrazado`, `PagerTrazado` — cuatro decoradores sobre traits que ya existían, precedidos por `ContandoStore` (cap. 26).
- **UNA sola verdad**: el recibo se deriva de `ExecMetrics` por nombre canónico; duplicar fuentes es el bug 8-vs-4 que este capítulo cazó en su propia casa.
- **Separar instrumentar de activar**: los spans se emiten SIEMPRE (coste ≈ 0 sin subscriber — callsite check); `--profile` solo instala el receptor e imprime. Los goldens intactos son la prueba empírica.
- **`try_close` sobrescrito o cero duraciones**: los defaults del trait delegan el cierre sin avisar.
- **Parenting contextual**: el árbol físico emerge de la pila enter/exit del subscriber — nadie pasa padres a mano.
- **Campos fijos > mapa**: el compilador como guardián de nombres; `Display` determinista imitando el exposition format de Prometheus.
- **Fiabilidad honesta**: WAL por delta (`Wal::as_bytes().len()`), transacciones contadas en la conductora, page fetch ausente de la ruta y DECLARADO.
- **Duraciones anidadas**: en Volcano pull-based, el tiempo del padre CONTIENE al del hijo — lee quién contiene a quién antes de señalar culpables.

## 35.12 Ojo, cuidado con…

- **«`--profile` sustituye al `explain`.»** No: `explain` (cap. 21) estima ANTES/DESPUÉS del optimizador; `--profile` mide lo que SU pipeline compiló — y ese pipeline hoy no corre la fase `optimise` (documentado). Por eso la consulta del hito muestra `NodeScan` donde `explain` enseña `IndexSeek`: son dos vistas distintas de planes distintos.
- **Comparar anchuras entre niveles como si fueran disjuntas.** Son contenciones pull-based: `Filter` 51,5 µs incluye a `Expand` y `NodeScan`. El tiempo PROPIO de cada nivel es la diferencia con su hijo más lento.
- **Implementar TU subscriber y fiarte de los defaults.** Sin `try_close`, tu árbol tendrá nombres y padres perfectos… y ninguna duración. Es el fallo más silencioso del capítulo.
- **Usar `RefCell` en el subscriber.** `Dispatch::new` exige `Send + Sync`; `RefCell` no es `Sync`. `Mutex` en monohilo tiene semántica idéntica y nanosegundos de coste.
- **Sumar dos fuentes «por si acaso».** El bug 8-vs-4: dos contadores correctos que se suman dan un recibo falso. Una métrica, una fuente.
- **Condicionar la emisión de spans a un flag booleano.** Dos rutas de código y la promesa de coste-cero-desactivado convertida en mentira estructural. Emitir SIEMPRE; activar con subscriber.
- **Dorar la salida del perfil.** Los tiempos varían entre máquinas: golden con duraciones = CI rojo por razones atmosféricas. Estructura sí, contadores sí, tiempos jamás.

*Precisión de lenguaje*: *traza*/*trace* (el árbol completo) vs *span* (un tramo con nombre, padre y duración) vs *log* (evento puntual sin estructura causal); *contador*/*counter* (acumulable monótono) vs *duración* (distribución — por eso no es contador); *registro*/*registry* (dónde viven los contadores) vs *subscriber* (quién recibe los spans); *exposición*/*metrics exposition* (formato textual servido) vs *exportación* (enviar a backend); *decorador*/*decorator* (envoltorio transparente del mismo trait) vs *instrumentación* (colocar los puntos de medida); *callsite* (el punto estático del macro, con su chequeo de interés) vs *emisión* (el aviso efectivo al subscriber).

## 35.13 Pin de batalla

> *«Instrumentar no es tocar el motor: es envolver sus puertos. Y la jerarquía de spans no se inventa — se revela: ya estaba latente en el pipeline. El recibo dice CUÁNTO; el itinerario dice DÓNDE.»*

## 35.14 Si solo lees 30 segundos

Observabilidad interna = recibo + itinerario. `Contadores` (std, campos fijos, `Cell<u64>`, `snapshot()`, `Display` estilo Prometheus) en la lib dependency-free; spans de fase/operador/página en la CLI con `tracing = "0.1"` (única dependency nueva). `SuscriptorArbol`: los 7 métodos requeridos del trait `Subscriber` + `try_close` SOBRESCRITO (los defaults no avisan del cierre) + pila para padres contextuales; `Send+Sync` ⇒ `Mutex`, no `RefCell`. Cuatro decoradores apilables sobre traits existentes: `MedidorOperador`, `MedidorPaginas` (medidor), `OperadorTrazado`, `PagerTrazado` (span fuera). Métricas derivadas de `ExecMetrics.per_operator` por nombre canónico — UNA sola verdad (el bug doble conteo 8-vs-4 mandó la lección). Hito: `liradb query --profile '...'` imprime fases (`Instant`, cap. 34) + árbol indentado + recibo; ADITIVO, goldens intactos; salida NO-golden (tiempos libres, estructura y contadores exactos). Jerarquía revelada: query → parse/plan/(optimise)/execute → Project→Filter→Expand→NodeScan; el 4º nivel (`storage_read`) demostrado a nivel componente sobre pool. Frontera honesta: sin DiskStore, page fetch no aparece en la ruta de consulta.

## 35.15 Una historia pequeña

Abril de 1970. El Apolo 13 viajaba a 330.000 kilómetros de la Tierra cuando, a las 55 horas y 55 minutos de vuelo, una explosión sacudió el módulo de servicio. Nadie podía mirar dentro de la nave: no había ventana posible hacia el depósito de oxígeno número 2. Lo único que Houston tenía era la TELEMETRÍA — corrientes, presiones y estados llegando como flujos continuos de números. Y fue la telemetría la que habló primero: la propia NASA documenta que Mission Control registró una caída de señal de 1,8 segundos antes del informe de la tripulación, seguida del desplome de presión en el tanque 2 y de las caídas de corriente en las pilas de combustible 1 y 3. Al principio parecía imposible — demasiados fallos simultáneos para ser ciertos — y hubo que confiar en los datos contra la incredulidad: varios canales independientes fallando juntos no era un sensor roto, era el sistema contando su propia historia. La junta de revisión (Edgar Cortright, «Report of Apollo 13 Review Board», NASA TM X-65270, junio de 1970) reconstruyó después la secuencia exacta combinando esas grabaciones con análisis del hardware: termostatos dañados durante una prueba en tierra, aislamiento Teflon fragilizado, chispa al arrancar los ventiladores. Hay un detalle para guardar: el sensor de temperatura del calentador solo media hasta 85 °F — y el tubo alcanzó cerca de 1.000 °F. La telemetría decía «todo bien» porque el instrumento era ciego por arriba: lo que no mides, no puede avisarte. La tripulación volvió viva gracias a procedimientos diseñados EN TIERRA leyendo esos mismos flujos. Moraleja del capítulo: cuando no puedes abrir el sistema — una nave a 330.000 kilómetros, un motor en producción — la única ventana es la observabilidad bien construida; y una ventana con el rango equivocado es una pared pintada de cristal.

## Ejercicios resueltos

**1. Lee el árbol del hito.** En la salida de §35.8, `Filter` tarda 51,5 µs. Filtrar cuatro filas no cuesta eso. ¿Qué mide realmente ese número? El tiempo TOTAL del subtree: en Volcano pull-based (cap. 20), cada `next()` del `Filter` arrastra un `next()` del `Expand`, que arrastra un `next()` del `NodeScan` — por eso `Filter` 51,5 ⊃ `Expand` 50,0 ⊃ `NodeScan` 47,1. El trabajo PROPIO del filtro es la diferencia con su hijo, unos microsegundos. Lectura correcta: busca el primer nivel donde el tiempo «se apila» respecto del hijo, no el span más ancho a secas. Verificación: la misma estructura anidada que `MedidorOperador::tiempo_total_ns` documenta en la lib («incluye el coste de TODOS los hijos: el pull anida»).

**2. Diagnostica el recibo mentiroso.** Una versión antigua imprimía `nodes_scanned 8` para una consulta sobre el grafo demo… que tiene cuatro Person. ¿Qué pasó, cómo se detecta hoy y por qué la solución no fue «restar dos»? Pasaron dos cosas correctas a la vez: los medidores del árbol contaban filas EN VIVO hacia el registro y, al cerrar, `derivar_contadores` sumaba LAS MISMAS filas desde `ExecMetrics` — 4+4=8. Detectarlo hoy es gratis: `medidor_operador_en_arbol_coincide_con_exec_metrics` exige que, si ambos caminos existen, coincidan; y `metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico` fija la cifra verdadera (4). La solución no fue aritmética sino arquitectural: UNA sola fuente — el recibo se deriva, los medidores no alimentan el pipeline del hito (`compilar_perfilado` lo declara en su doc comment). Restar dos habría «arreglado» el síntoma dejando el diseño podrido.

**3. Retrieval: la jerarquía, de memoria.** Cierra el libro y dibuja los cuatro niveles de la jerarquía del CORPUS, con el instrumento que captura cada uno. Respuesta: `query` (raíz, span) → `parse`/`plan`/`(optimise)`/`execute` (spans de fase) → operadores (`Project`/`Filter`/`Expand`/`NodeScan`/`IndexSeek`: spans con nombre canónico vía `OperadorTrazado`) → `storage_read` (span por lectura de página vía `PagerTrazado`, hoy solo a nivel componente). ¿Qué captura cada instrumento? Spans: jerarquía y duraciones (`SuscriptorArbol`); contadores: trabajo agregado (`Contadores` por derivación/composición/delta); `Instant`: duración por fase (tabla Fases). Si escribiste «logs» en cualquier casilla, revisa §35.2: un log no tiene padre ni duración.

## Ejercicios propuestos

**Esencial (recordar — retrieval practice).** Sin mirar el capítulo: (a) reproduce la salida de `liradb query --profile` sección a sección (¿cuáles son las cinco partes y en qué orden?); (b) explica qué responde un SPAN y qué responde un CONTADOR ante «esta consulta va lenta»; (c) nombra los tres métodos del subscriber que hacen posible el árbol (uno asigna ids y padres, dos mantienen la pila, uno fija duraciones). *Verificación*: `cargo test -p liradb-cli --test observabilidad_cli` y §35.8. *Criterio*: las cinco secciones en orden SIN releer el capítulo.

**Intermedio (interleaving 20+35: predecir + derivar).** Deriva una métrica nueva `filters_passed` (filas producidas por Σ `Filter`) siguiendo el patrón de `metricas_consulta`: función pura sobre `per_operator`, test de consistencia contra la suma manual. ANTES de correr Q2 (`MATCH (p:Person) WHERE p.age < 40 RETURN p.name`), escribe tu predicción: ¿cuántas filas pasan el filtro y qué valor espera el recibo? Luego verifica con `derivacion_cubre_expand_e_indexseek_del_pipeline_real` como modelo. *Criterio*: predicción escrita antes del primer comando; el demo tiene 4 Person y 3 pasan `age < 40`.

**Intermedio (interleaving 12+35: el cuarto nivel con B+Tree).** Reproduce `jerarquia_cuatro_niveles_componente_indice_sobre_pool` sustituyendo `HashIndex` por `BPlusTree` (cap. 15) sobre el mismo pool de 4 frames. ANTES de correr: anota cuántos `storage_read` esperas para 200 búsquedas y POR QUÉ (cadena de desborde lineal del hash vs altura logarítmica del B+Tree). Después compara el delta de `page_reads` del medidor con el nº de spans capturados. *Verificación*: la coherencia triple del test original (medidor == spans == pool). *Criterio*: la predicción explica la diferencia estructural entre las dos estructuras, no solo la cifra.

**Experto (crear).** Dos caminos, elige uno: (a) añade a `SuscriptorArbol` registro de EVENTOS (`event`) y un método `eventos_de(span_id)`; escribe el test que captura un evento dentro de un span y afirma su pertenencia. (b) Acumula duraciones por NOMBRE de span en el subscriber y expón `p99(nombre)` usando `percentiles` del cap. 34; valida contra 100 spans generados en un bucle. En ambos: cero cambios en caps. anteriores, captura con `capturar`/`with_default`, sin stderr ni globales. *Verificación*: `cargo test -p liradb-cli --lib observabilidad`. *Criterio*: el test nuevo falla si borras el override de `try_close` — demuestra que dependes del método correcto.

## Para profundizar

- **Benjamin H. Sigelman, Luiz André Barroso, Mike Burrows, Pat Stephenson, Manoj Plakal, Donald Beaver, Saul Jaspan y Chandan Shanbhag, «Dapper, a Large-Scale Distributed Systems Tracing Infrastructure» (Google Technical Report dapper-2010-1, abril de 2010)** — la fuente primaria de la anécdota y del modelo mental: árboles de spans, padre explícito, <1 % de overhead y despliegue ubicuo.
- **Docs oficiales de `tracing` (docs.rs/tracing y github.com/tokio-rs/tracing)** y **de `tracing-core`** — el trait `Subscriber` con sus métodos requeridos, `dispatch::with_default`, y el coste de emitir sin subscriber: el manual exacto de la mitad itineraria.
- **Docs de `metrics-rs` (docs.rs/metrics)** — la facade con recorder global (`set_global_recorder`): el contraste industrial con nuestro registro local pasable por `&`.
- **Prometheus, «Exposition Formats» (prometheus.io/docs/instrumenting/exposition_formats)** — el text format que el `Display` de `Contadores` imita: `# TYPE`, pares nombre/valor, orden.
- **OpenTelemetry (opentelemetry.io/docs)** — spans con padre, propagación de contexto, OTLP: Dapper convertido en estándar CNCF; la frontera que este capítulo NOMBRÓ sin implementar.
- **PostgreSQL: `pg_stat_statements` y `EXPLAIN (ANALYZE, BUFFERS)` (docs oficiales)** — el recibo-y-itinerario nativo de producción, con contadores de páginas incluidos.
- **MySQL: `performance_schema` y el slow query log (docs oficiales)** — el registro de contadores y el filtro de cola del motor de Oracle.
- **Gamma, Helm, Johnson y Vlissides, «Design Patterns» (GoF, 1994), patrón Decorator** — la teoría detrás de los cuatro envoltorios; el precedente interno es `ContandoStore` (cap. 26).
- **Brendan Gregg, «Systems Performance», 2ª ed. (2020)** — retomado UNA línea del cap. 34: los bloques anchos son el perfil; aquí el motor los dibuja solo.
- **NASA, «Report of Apollo 13 Review Board» (TM X-65270, junio de 1970)** y el «Apollo 13 Mission Report» (MSC-02680, septiembre de 1970) — las fuentes primarias de la historia pequeña: telemetría grabada, reconstrucción de la secuencia y el sensor ciego por arriba de 85 °F.

## Mini-diálogo: en guardia nocturna

> — Suena el pager: «la base de datos va lenta». Tres de la madrugada. Abro el dashboard y…
>
> — Respira. Esta noche tienes algo que anoche no tenías. ¿Qué?
>
> — El… ¿recibo? Ejecuto la consulta con `--profile` y miro los contadores.
>
> — ¿Y qué dice?
>
> — `nodes_scanned 500_000`… para una consulta que devuelve diez filas. Pero esto ya lo sabía ayer: que escanea mucho. Lo que NO sé es DÓNDE se va el tiempo.
>
> — Por eso no has acabado. Baja al árbol.
>
> — `query` → `execute` → `NodeScan`… ¡y el `Filter` está ARRIBA del scan, no debajo! El índice por email existe y el plan no lo está usando.
>
> — Ayer habrías mirado la media, habrías rezado y habrías reiniciado algo. Hoy el recibo dijo CUÁNTO (medio millón de nodos), el itinerario dijo DÓNDE (el `NodeScan` bajo `execute`, no el filtro) — y nadie adivinó nada: se LEYÓ.
>
> — ¿Y si mañana el problema no fuera un scan sino, no sé, un expand con fanout enorme?
>
> — El mismo gesto: el recibo diría `relationships_expanded` disparado y el árbol señalaría al `Expand`. Dos preguntas, dos vistas, un solo comando. Ahora duerme — y mañana, cuando lo arregles, guarda el `--profile` de esta noche: es tu baseline del cap. 34 con itinerario dentro.

---

*(Próximo capítulo: 36 — Arquitectura final. Treinta y cinco capítulos después, el motor tiene mapa: puertos hexagonales, torre de pruebas, benchmarks y ahora ojos propios. Los puertos medidos cierran el círculo — hora de contemplar el edificio completo.)*
