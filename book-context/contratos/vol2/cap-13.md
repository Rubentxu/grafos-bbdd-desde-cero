# CONTRATO DE CAPÍTULO — Vol.II Cap. 13: El buffer pool (LRU, Clock, métricas)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap13_buffer_pool.rs` (24 tests, `tests_buffer_pool`;
> MIGRATION-PATTERN §17). Decisiones y bugs reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md` §17
> (aguja de Clock que no avanzaba en accesos → víctima incorrecta; trait `EvictionPolicy` muerto → enum
> `PolicyKind`; borrow de `get_page` retenido al llamar `unpin`).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el trait `Pager` y sus 8 operaciones, `FilePager`, `PagerError`
  tipado y que `write` no es durable sin `sync` (cap. 12); `PAGE_SIZE = 4096`, `SlottedPage`
  con length-prefix, metapágina (cap. 11); traits/genéricos con bounds (cap. 8 `GraphStore`);
  patrones de latencia disco vs RAM (cap. 11: seek mecánico 5-10 ms).
- **Cree saber pero es vago/erróneo (misconceptions centrales)**: (1) «Clock ES LRU» — falso:
  es una aproximación; un test real del módulo asumió determinismo LRU en Clock y hubo que
  reescribirlo (MIGRATION §17, bug 6); (2) «un cacheo acelera las escrituras» — el pool difiere
  escrituras (write-back), no las abarata; sin `dirty` los cambios se pierden; (3) «unpin es
  opcional si no modifiqué» — falso: pin leak → `PoolFullOfPinned`; (4) «hit ratio alto = caché
  bien dimensionada» — el ratio no mide frío, ni secuencialidad, ni escrituras; (5) «flush
  escribe en disco» — write va a la page cache del SO; lo baja el `sync` (recuperación del
  cap. 12).
- **NO debe saber todavía**: concurrencia sobre el pool (`AtomicU64`, condition variables,
  wrapper del cap. 28), WAL/recuperación para cerrar la ventana de caída del write-back
  (caps. 28-29), `MmapPager` (apéndice), ring buffers de PostgreSQL para scans grandes,
  GClock/CLOCK-Pro (reto experto), MVCC. Se nombran como «luego lo verás» y se corta ahí.

## 2. Conceptos (del grafo curricular)

- `present`: buffer pool; frame (`FrameId`) vs página; page table (`PageId → FrameId`);
  pin/unpin y `pin_count`; dirty bit y write-back vs write-through; política de reemplazo:
  Clock/second chance (aguja + `ref_bit`), LRU con contador monotónico; víctima (limpia vs
  sucia, flush-previo); `PoolFullOfPinned`; `discard` (invalidación); métricas (`hits`, `misses`,
  `page_reads`, `page_writes`, `evictions`, `hit_ratio`); `MemoryPager` como test double del port.
- `practice`: trait `Pager` (cap. 12 — ahora con su segundo cliente: el pool es genérico
  `<P: Pager>`); `[u8; PAGE_SIZE]` y `SlottedPage::encode/decode` (cap. 11 — un frame ALOJA una
  página del cap. 11); errores tipados con `From` y `source()`; `sync`/fsync (N writes + 1 sync).
- `consolidate`: validar en la frontera antes de la E/S; errores que distinguen culpa propia
  del entorno; «leak mejor que corrupción» (discard conservador); derivar, no llevar en la cabeza.
- `out_of_scope` (solo nombrar): concurrencia/pines bloqueantes (cap. 28), WAL (cap. 28),
  recovery (cap. 29), mmap (apéndice), múltiples pools (mención PostgreSQL), GClock (reto).

## 3. Objetivos de dominio

- **Knowledge**: (1) calcula el coste de operar SIN pool (cada acceso = seek+read; ~100 ns RAM
  vs ~5-10 ms disco mecánico) y por qué la raíz del B+ del cap. 15 hace que un pool pague
  solo; (2) enuncia el protocolo `get_page` → uso → `unpin(dirty)` y qué rompe cada desviación
  (falta de unpin → `PoolFullOfPinned`; unpin doble → `BadPinCount`; mutar sin dirty → cambios
  perdidos en la expulsión); (3) ejecuta Clock sobre papel: aguja, `ref_bit`, second chance,
  salto de pineados, cota de 2·n pasos — y explica por qué la aguja debe avanzar TAMBIÉN en
  cada acceso (el bug real: sin ello, dos frames accedidos en tiempos distintos son
  indistinguibles y la evicción degenera); (4) opone write-back+dirty a write-through con
  números (fsync por mutación vs coalescing de 1.000 modificaciones en 1 escritura) y nombra
  el precio (ventana de caída → WAL cap. 28); (5) dice qué mide `hit_ratio` y qué NO (frío,
  scans secuenciales, escrituras, dimensionamiento).
- **Skills**: (1) usa `get_page`/`unpin`/`mark_dirty`/`flush`/`flush_page`/`discard` y lee
  `metrics()` sin parsear nada; (2) traza una secuencia de accesos (capacidad 2, A-B-A-C) y
  predice la víctima — verificable contra `bp_clock_second_chance_protects_hot_page`; (3)
  escribe un test del pool contra `MemoryPager` (sin disco) en la línea de `small_pool()`.
- **Wisdom**: (1) decide cuándo una aproximación O(1) sin cerraduras (Clock) vence a la
  política «mejor» (LRU estricto, ARC): el coste de mantener el orden exacto — la historia
  real de PostgreSQL 8.1; (2) reconoce el trade-off que compra toda caché: lecturas baratas
  pagadas con durabilidad diferida, y dónde poner el corte (`flush` vs crash window).

## 4. Modelo mental

- **El taller con mesas limitadas**: N mesas idénticas (frames) sobre las que se abren cajas
  (páginas) traídas del almacén del cap. 12 (pager). Cada mesa tiene tres chapaletas: «EN USO»
  (pin_count > 0 — el inspector no puede tocarla), «SUCIA» (dirty — hay que devolver la caja al
  almacén antes de reutilizar la mesa) y «usada recientemente» (`ref_bit`). Un inspector recorre
  las mesas EN CÍRCULO con una aguja: mesa pineada → la salta; chapaleta de recencia puesta →
  la quita y sigue (second chance); mesa limpia y sin chapaleta → esa mesa se libera (víctima).
  La aguja avanza CADA VEZ que alguien usa una mesa, no solo cuando el inspector busca hueco:
  así el orden del círculo refleja el orden de uso.
- **Diagrama ASCII**: (a) apilado de capas — callers (caps. 14-15) → `BufferPool<P>` → trait
  `Pager` → disco/RAM; (b) pool de frames en círculo con la aguja, pins y dirty flags + page
  table mapeando `PageId → FrameId`.
- **Momento ¡ajá!**: el bit de referencia solo sabe «sí/no desde la última pasada»; quien
  convierte ese bit en un ORDEN (quién es más viejo) es la POSICIÓN de la aguja respecto al
  frame. El reloj es una estampa de tiempo barata: distancia-a-la-aguja ≈ antigüedad.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap13_buffer_pool.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Cachear PÁGINAS enteras (`[u8; PAGE_SIZE]`), no registros ni resultados | El disco entrega bloques, no bytes (cap. 11): cachear el registro obligaría a leer la página entera y tirarla, desperdiciando la localidad espacial del vecino; el pool queda contenido-agnóstico (sirve a CSR cap. 14 e índices cap. 15 sin parsear nada) | Record/result cache: acopla la caché al formato y muere con cada escritura (invalidación); MySQL tuvo query cache y la ELIMINÓ en 8.0 por cuello de botella | Dos registros de la misma página = dos lecturas completas; cada nueva estructura exige su propia caché | Cap. 11 (el disco lee bloques); MySQL 8.0 release notes (query cache removed); CMU 15-445 |
| 2 | `BufferPool<P: Pager>` genérico | El pool es un adapter sobre el port del cap. 12: los tests de evicción corren contra `MemoryPager` (RAM, sin tempfiles, miles por segundo) y la persistencia contra `FilePager` sin cambiar una línea del pool | Pool soldado a `FilePager`: cada test de evicción paga sistema de ficheros; mmap futuro imposible sin reescribir | La batería de 24 tests tardaría segundos y nadie la ejecutaría | MIGRATION §17.1 y §17-lección 5; el puente anunciado en el mini-diálogo del cap. 12 |
| 3 | Pin/unpin EXPLÍCITO, no guard RAII | El lifetime del préstamo es visible en el protocolo: `get_page` pinea, `unpin` despinea — igual que PostgreSQL (pin count por buffer). Un guard con `Drop` prestaría `&mut pool` durante toda la vida de la página: imposible pedir DOS páginas a la vez (joins/iteraciones del cap. 14) sin `Arc<Mutex>` o `unsafe` | `PageGuard<'_>`: ergonómico, pero lifetimes en cada firma (ruido pedagógico) y API de una página a la vez | El alumno nunca VE el invariante pin_count que todo motor real mantiene | MIGRATION §17.2; doc-comment de `get_page`; test real del borrow conflictivo (§17, bug 3) |
| 4 | `pin_count` blinda la víctima; `PoolFullOfPinned` es ERROR, no espera | Expulsar una página en uso = sobrescribir los 4 KB bajo los pies del caller → corrupción silenciosa. Y «esperar» es imposible en single-thread (`&mut self`): sería un deadlock consigo mismo; el error tipado es el DETECTOR del pin leak (falta un `unpin`) | Espera bloqueante: sin otro hilo que pueda despinear, cuelga para siempre; panic: no deja reaccionar | Caller escribe bytes de la página nueva creyendo que son la vieja; `mark_dirty` de la página equivocada | `pick_victim_clock` salta pineados; test `bp_pool_full_of_pinned`; cap. 28 para la versión con espera |
| 5 | `dirty` + write-back, no write-through | Escribir en cada mutación (con durabilidad honesta, fsync) cuesta ms por cambio de 8 bytes; el dirty bit difiere la escritura a la expulsión o al `flush`, y FUSIONA 1.000 modificaciones en 1 escritura (coalescing) | Write-through: latencia de escritura colapsada (µs → ms por mutación) | UPDATE de una propiedad paga seek+fsync por nodo | `bp_dirty_page_is_flushed_on_eviction`; cap. 12 (fsync cuesta órdenes de magnitud); PostgreSQL/InnoDB usan write-back con background writers |
| 6 | La víctima sucia se flushea ANTES de sobrescribir el frame | Reutilizar un frame sucio sin escribir = perder los cambios del usuario sin que nadie se entere; el flush previo (write + `page_writes += 1`) es la mitad de la promesa del write-back | Descartar y recargar: corrupción silenciosa con aspecto de grafo válido (la bestia del cap. 11) | Cambios «guardados» evaporados en la expulsión siguiente | Pasos 3 del `get_page`; test `bp_dirty_page_is_flushed_on_eviction` (verifica el 0xAA en el pager) |
| 7 | La aguja avanza en CADA acceso (touch y miss-load), no solo al buscar víctima | El `ref_bit` solo dice «usada desde la última pasada»; el ORDEN lo da la posición de la aguja. Sin avance en accesos, entre ráfagas de hits todos los bits se acumulan a true y la aguja arranca el barrido desde una posición congelada (orden de CARGA, no de USO): dos frames accedidos hace 1 s y hace 1 h son indistinguibles → evicción ~aleatoria. Avanzando en cada touch, distancia-a-la-aguja ≈ antigüedad | Aguja solo en `pick_victim` (Clock «de libro», pensado para fallos frecuentes y ref bit por hardware): degenera en nuestro workload de hits en ráfagas — EL BUG REAL del módulo | La página caliente expulsada; el test de second chance falla (misses 4 ≠ 3) | MIGRATION §17.4 y §17-lección 1 (bug 4 de la tabla); comentarios de `PolicyKind::Clock` y `touch_frame`; test `bp_clock_second_chance_protects_hot_page` |
| 8 | `ref_bit = true` también al cargar (paso 5 de `get_page`) | La página recién llegada cuenta como usada: una pasada de gracia para que un scan N+1 páginas / N frames no expulse cada página recién traída antes de usarla | Cargar con bit a false: la página nueva es víctima inmediata en el siguiente miss | Thrashing: cada carga expulsa a la anterior | Paso 5 del `get_page` (`// recién cargada, márcala como usada`); patrón second-chance estándar |
| 9 | Clock por defecto; LRU con contador monotónico como comparación (`PolicyKind`) | Clock: O(1) por acceso y O(1) de estado (1 bit + 1 puntero), sin listas que mantener — la razón por la que PostgreSQL lo adoptó (eliminar el BufMgrLock monolítico). LRU estricto necesita el orden EXACTO de uso: lista doblemente enlazada + punteros por touch | Solo LRU estricto: cada touch re-ordena una lista (coste + contención en concurrencia — la historia real de pre-8.1); solo Clock: el alumno no VE la recencia explícita | Elegir mal la política = pagar contención (PG) o pagarse O(n) por evicción (nuestro LRU de escaneo) | MIGRATION §17.5; PostgreSQL wiki (clock-sweep desde 8.1); tests `bp_clock_second_chance_protects_hot_page` y `bp_lru_policy_evicts_least_recent` |
| 10 | `PolicyKind` enum, no trait `EvictionPolicy` con `dyn` | Para DOS variantes cerradas, el enum guarda el estado como campos planos (`clock_hand`, `lru_counter`) y el dispatch es un `match` legible; el trait inicial quedó como dead code (las dos políticas comparten firma pero no estado) y se eliminó | `dyn EvictionPolicy`: vtable + object-safety + estado repartido en boxes, para 2 variantes conocidas; los traits son para ports ABIERTOS (`Pager`), los enums para variación interna CERRADA | Complejidad sin usuario: dead code que los lints cazan | MIGRATION §17 (bug 5 de la tabla: «Trait EvictionPolicy quedó como dead code → eliminar»); contraste con `Pager` del cap. 12 |
| 11 | `page_table: Vec<Option<FrameId>>` con `ensure_page_table` | Índice directo por `PageId` → lookup O(1) sin hash; los ids de `FilePager` son densos (secuenciales + free list), así que el Vec no desperdicia | `HashMap<PageId, FrameId>`: hashing por acceso y orden de iteración no determinista (tests frágiles) | Lookup lento y tests no deterministas | `find_frame` (`.get(id).and_then`) y `ensure_page_table`; densidad de ids heredada del cap. 12 |
| 12 | `flush()` y `flush_page()` terminan en `pager.sync()` | La promesa de flush es «a salvo»: `pager.write` solo llega a la page cache del kernel (lección del cap. 12); sin fsync, un crash post-flush pierde lo que el usuario creía seguro. N writes + 1 sync | Parar en write: «flush» sería un nombre mentiroso — mitad de la durabilidad prometida | Falsa sensación de durabilidad; pérdida post-crash | Última línea de `flush`/`flush_page`; cap. 12 (`sync_all`); `bp_persistence_via_filepager` |
| 13 | `hit_ratio()` = hits/(hits+misses), 0.0 sin accesos; contadores separados (`page_reads` ≠ `buffer_misses`) | Mide la fracción de `get_page` servidos desde memoria; `page_reads` cuenta llamadas reales al pager (divergirían con prefetch): medir cosas distintas por separado es lo que permite diagnosticar | Un solo contador «eficiencia»: escondería la diferencia entre fallo de caché y lectura física | Decisiones de tuning tomadas sobre un número que no dice lo que parece | `Metrics::hit_ratio`; test `metrics_hit_ratio` (3/4 = 0.75, 0.0 vacío); `bp_basic_get_unpin` (reads == misses == 1) |
| 14 | `discard` rechaza frames sucios (sentinel `current = u32::MAX`) | Invalidar una página con cambios no escritos = perderlos; la política conservadora obliga a `flush_page` explícito antes de descartar (leak de un paso, jamás corrupción — la asimetría del cap. 12) | Flush implícito dentro de discard: esconde la pérdida de latencia y decide por el caller; variante `Dirty` del enum: la API no la tenía y el sentinel es la deuda documentada | Cambios evaporados en un `discard` «que parecía inofensivo» | Código y comentario de `discard` («decisión pedagógica… optamos por la conservadora»); tests `bp_discard_dirty_rechazado`, `bp_discard_cleans_dirty_ok` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: `HashMap<PageId, Vec<u8>>` ilimitado — cachear todo lo leído, para siempre. Sin
  evicción, sin pins, sin dirty. Pasa el happy path con ficheros pequeños.
- **Qué la rompe**: (a) un fichero de 2 GiB en una máquina con 512 MiB libres → OOM (la memoria
  es finita aunque el disco no); (b) el caller que muta el `Vec<u8>` cacheado no deja rastro:
  nada sabe que hay que reescribir la página (cambios perdidos); (c) si alguien «expulsa» al
  azar mientras otro usa la página, se sobreescriben bytes bajo sus pies; (d) sin métricas,
  ninguna decisión de tuning tiene número que la respalde.
- **Evolución visible en el capítulo**: array fijo de `Frame` (contenido + `pin_count` + `dirty`
  + `ref_bit`/`lru_counter`), `page_table` O(1), política de reemplazo con contrato
  (`pick_victim` respeta pins y devuelve `None` si no hay víctima → `PoolFullOfPinned`),
  write-back con flush de víctima sucia, y `Metrics` desde el primer día.

## 7. Prueba de fuego

- **Tests reales del módulo** (se citan, no se duplican): `bp_basic_get_unpin` (miss+hit,
  `page_reads == buffer_misses`), `bp_modify_mark_dirty_flush`, `bp_eviction_when_pool_full`
  (2 frames, 4 páginas → 2 expulsiones), `bp_dirty_page_is_flushed_on_eviction` (el 0xAA
  sobrevive en el pager), `bp_pool_full_of_pinned`, `bp_flush_no_dirty_is_noop`,
  `bp_flush_page_only_dirty`, `bp_clock_second_chance_protects_hot_page`,
  `bp_lru_policy_evicts_least_recent`, `bp_unpin_all_resets_pins`, `bp_discard_dirty_rechazado`,
  `bp_double_unpin_error`, y los end-to-end `bp_persistence_via_filepager` /
  `bp_reload_via_pool` (create → pool → flush → reopen, y el primer get tras reopen es miss).
- **Síntoma si el lector se salta el capítulo**: medible — un bucle de 100 accesos a la MISMA
  página produce 100 `page_reads` (con pool: 1); funcional — el CSR del cap. 14 y los índices
  del 15 quedan inutilizables (ms por página tocada); silencioso — mutaciones sin dirty se
  pierden en la primera expulsión sin error alguno.

## 8. Trampas y errores comunes

1. **Olvidar el `unpin`** (pin leak): el pool se llena de pineadas y todo `get_page` nuevo
   devuelve `PoolFullOfPinned`. Detector: el propio error; disciplina: un `unpin` por cada
   `get_page` con `Ok`.
2. **Mutar sin `dirty`**: ni `unpin(id, true)` ni `mark_dirty(id)` → la expulsión tira la
   página sin escribirla. Silencioso total.
3. **Tratar Clock como LRU exacto**: Clock solo promete second chance, NO la víctima LRU
   exacta — el test original del módulo lo asumía y hubo que reescribirlo tocando la página
   a proteger inmediatamente antes de la carga (MIGRATION §17, bug 6).
- **Precisión de lenguaje (glosario)**: *frame* (hueco de memoria del pool) vs *página*
  (contenido venido del disco); *hit* vs *page_read* (uno es servir de RAM, otro llamar al
  pager); *evicción* vs *flush* (sacar de la pool vs escribir a disco; se puede flushear sin
  expulsar y viceversa); *write-back* vs *write-through*; *pinear* vs *bloquear* (pin es
  recuento de uso, no un lock de exclusión); *víctima limpia* vs *sucia* (la sucia cuesta
  escritura).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar**: trazar A-B-A-C con capacidad 2 y Clock, prediciendo víctima y métricas.
  Verificación: razonamiento idéntico al test `bp_clock_second_chance_protects_hot_page`
  (misses = 3). Pistas (≤3, graduadas): (1) ¿dónde queda la aguja tras cada touch?, (2) ¿qué
  pasa con los ref_bit en la primera vuelta del barrido?, (3) ¿quién se examina ÚLTIMO?
  Criterio: distinguir orden de círculo de orden de carga.
- **analizar**: un workload tiene `hits = 990, misses = 10` y otro `hits = 0, misses = 100`
  (scan secuencial de un fichero por un pool de 10). ¿Cuál está «peor»? ¿Qué NO dice el ratio
  en cada caso? Verificación: `metrics_hit_ratio` + `bp_eviction_when_pool_full`. Pistas:
  (1) ¿el miss del scan es aleatorio o secuencial?, (2) ¿cuánto mide el pool frente al
  working set?, (3) ¿qué contador faltó mirar (`page_writes`)?
- **crear** (retrieval, cap. 11): escribir de memoria el layout de `PageHeader` (10 bytes,
  campos y anchos) y por qué el frame es `[u8; 4096]` exacto; luego montar un test con
  `MemoryPager` que guarde una `SlottedPage` codificada vía `get_page`+`mark_dirty`+`flush` y
  la recupere decodificando el header desde el buffer del pool. Pistas: (1) ¿qué produce
  exactamente `SlottedPage::encode`?, (2) ¿quién valida el tamaño?, (3) ¿dónde vive el magic?
  Criterio: el test debe fallar si se decodifica desde el offset equivocado.

## 10. Preguntas abiertas (gancho al capítulo 14)

1. El CSR persistente leerá sus cuatro columnas página a página: ¿cuántos frames pinéa un
   recorrido completo y en qué orden conviene tomarlos para que la aguja no expulse lo que
   vas a necesitar en el paso siguiente?
2. `PersistentCsr::replace()` reescribe páginas por completo: ¿qué queda en los frames que
   cacheaban las versiones antiguas (stale) y qué método de ESTE capítulo lo resuelve?
   (Respuesta: `discard` — nació para esto.)
3. En producción, ¿cómo decides si tu pool es pequeño? El `hit_ratio` acumulado de vida no
   basta (el frío lo contamina): ¿qué ventana harías? (Semilla para el cap. 35,
   observabilidad.)
- **Términos nuevos de glosario**: frame, page table, pin/unpin, pin_count, dirty bit,
  write-back/write-through, coalescing, víctima, second chance, aguja (clock hand), hit/miss,
  hit ratio, thrashing, working set, test double, stale frame.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio «crear» pide reconstruir DESDE LA MEMORIA el
  `PageHeader` del cap. 11 (campos y anchos) — el cap. 13 solo dice «el frame aloja una página
  de 4.096 bytes», sin regalar el layout; y el «analizar» obliga a recordar (cap. 12) por qué
  `flush` acaba en `sync` para razonar sobre durabilidad.
- **Spacing**: el ejercicio «crear» re-ejercita `SlottedPage::encode/decode` y el length-prefix
  del cap. 11 a través del pool; `bp_persistence_via_filepager` re-ejercita create/sync/reopen
  del cap. 12; la narrativa del capítulo reutiliza explícitamente «N writes + 1 sync» y
  «validar en la frontera».
- **Interleaving**: el «analizar» cruza métricas del cap. 13 con coste de E/S secuencial vs
  aleatoria (cap. 11); el experto propuesto mezcla política de reemplazo con el clock sweep
  con usage_count de PostgreSQL (producción real); el trace de Clock obliga a razonar pins,
  bits y aguja a la vez.
- **Dificultad asimétrica**: cada sección introduce UNA idea nueva (frames, protocolo pin,
  dirty, aguja, métricas); los ejercicios exigen predicción y reconstrucción desde memoria.
- **Bucle de feedback inmediato**: todo se verifica con `cargo test -p vol2-liradb
  cap13` (24 tests citados por nombre).
- **Citas**: PostgreSQL wiki (clock-sweep desde 8.1; patente caducada 2024-02-22), LWN 131554
  (8.0.2, reescritura por patente), Wikipedia «Page replacement algorithm» (Corbató 1969),
  Megiddo & Modha ARC (FAST'03) + patente US 6.996.676, The Internals of PostgreSQL cap. 8
  (usage count, pins), MySQL manual (midpoint LRU de InnoDB, `innodb_old_blocks_time`),
  MySQL 8.0 release notes (query cache eliminada), MIGRATION-PATTERN §17, Petrov «Database
  Internals» cap. 2, CMU 15-445 (proyecto buffer pool).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (14 en la tabla §5).
- [x] Escenario de fallo visible sin pool (ms por acceso, 100 reads por 100 accesos) y el bug
      clásico de expulsar una página pineada (§6-§7 del capítulo).
- [x] Código ejecutable en workspace (24 tests) citado por nombre, no duplicado.
- [x] Misconcepción corregida explícitamente («Clock = LRU», «unpin opcional», «hit ratio lo
      dice todo», «flush escribe en disco»).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (PageHeader desde memoria, cap. 11) y ≥1 de spacing
      (length-prefix del cap. 11 vía pool; sync del cap. 12 vía flush).
- [x] Responde las preguntas críticas: páginas vs registros, pin explícito vs RAII, dirty vs
      write-through, aguja en cada acceso (el bug real), Clock vs LRU, PoolFullOfPinned como
      error, flush→sync, qué mide y qué no hit_ratio, enum vs trait dyn.
