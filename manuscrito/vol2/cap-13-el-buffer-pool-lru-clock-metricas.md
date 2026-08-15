# Capítulo 13 — El buffer pool (LRU, Clock, métricas)

> *«Todo el mundo quiere la página en RAM. Nadie quiere decidir cuál sale cuando la RAM se llena.»*

## 13.0 La anécdota de la esquina

En enero de 2005, PostgreSQL 8.0 salió a la calle con el algoritmo de reemplazo más elegante que había estrenado una base de datos: **ARC** (Adaptive Replacement Cache), publicado dos años antes por Nimrod Megiddo y Dharmendra Modha en el laboratorio Almaden de IBM. ARC es una pieza de relojería: mantiene dos listas y, según cómo fallen las búsquedas recientes, recalibra solo el equilibrio entre «favorecer lo reciente» y «favorecer lo frecuente». Sin parámetros que tunear. Un trabajo hermoso.

Había un problema: IBM lo había patentado (patente US 6.996.676). En abril de 2005, Tom Lane reescribió el buffer manager de la versión 8.0.2 para sustituir ARC por una variante de 2Q libre de la patente. Y en diciembre de ese mismo año, la 8.1 hizo algo más radical: rediseñó el buffer manager entero para eliminar el cerrojo monolítico que lo estrangulaba (`BufMgrLock`) y adoptó un **clock sweep** — una aguja que gira en círculo bajando contadores de uso — que no es sino el descendiente directo del algoritmo de reloj que Fernando Corbató describió en 1969 para la memoria virtual.

La patente caducó el 22 de febrero de 2024. A día de hoy, PostgreSQL sigue usando el clock sweep y nadie ha corrido a devolver ARC a su sitio. La lección que este capítulo encarna: en un buffer manager, **un algoritmo O(1) sin cerraduras vence al algoritmo brillante que cuesta mantener**. Vamos a construir ese reloj con nuestras manos — y a descubrir que es más difícil de lo que parece, porque nosotros mismos implementamos mal la aguja la primera vez. Sigue leyendo.

## 13.1 Objetivo

El `FilePager` del capítulo 12 funciona, y cada una de sus lecturas es un `seek` + `read_exact` al disco. Este capítulo intercala una capa de memoria entre las estructuras de datos y el pager. Vas a construir tres piezas:

1. `BufferPool<P: Pager>` — un array fijo de **frames** (mesas de 4.096 bytes), una **page table** que mapea `PageId → FrameId`, y el protocolo **pin/unpin** que protege las páginas en uso.
2. La política de reemplazo — **Clock** (la aguja con segunda oportunidad) como defecto, **LRU** como comparación (`PolicyKind`).
3. `Metrics` — contadores y el **hit ratio**: el número con el que se entrevista a una base de datos en producción.

Y de propina, la promesa que dejamos en el mini-diálogo del capítulo 12: un `MemoryPager` en los tests que probará la caché contra RAM pura, miles de veces por segundo, sin tocar disco. El port por fin cobra una factura.

## 13.2 Problema

Los números primero. Acceder a RAM cuesta del orden de **100 nanosegundos**; un SSD NVMe, unos **100 microsegundos**; un seek en disco mecánico, **5-10 milisegundos** (lo vimos en el cap. 11). Entre RAM y disco mecánico hay cuatro a cinco órdenes de magnitud: por cada lectura de disco caben cien mil accesos a memoria.

Ahora mira tus futuras estructuras con esos ojos. La raíz de un B+ tree se consulta en **cada** búsqueda. La página de offsets del CSR (cap. 14) se consulta una vez por nodo del grafo. Un BFS que visite 100.000 nodos con el pager desnudo paga 100.000 lecturas — muchas de ellas de páginas que ya habías leído hace un milisegundo. Sin buffer pool, la consulta más tonta cuesta milisegundos por página tocada, y una base de datos que sólo sirve datos diminutos.

La pregunta de diseño: **¿qué cachear?** Y la respuesta del capítulo 11 vuelve a aparecer: páginas enteras, no registros. Cuatro razones:

1. **El disco ya te da la página entera.** No existe «leer un registro»: leer es traer el bloque de 4 KB. Cachear solo el registro significaría leer la página completa, extraer el registro y tirar el resto — incluido su vecino, que estadísticamente será consultado pronto (localidad espacial).
2. **El pool queda contenido-agnóstico.** Un frame es `[u8; PAGE_SIZE]`: bytes opacos. El mismo pool servirá al CSR del cap. 14 y a los índices del cap. 15 sin parsear nada. Una caché de registros, en cambio, tendría que entender el formato de cada página.
3. **Amortización por localidad.** Los vecinos de un nodo comparten página; una página cacheada paga su lectura una vez y sirve decenas de consultas.
4. **La alternativa «cachear resultados» tiene su cuento**. MySQL tuvo una query cache y la eliminó en la versión 8.0: cada escritura invalidaba entradas y el libro mayor de invalidación se comió el rendimiento. Cachear la unidad de disco es más aburrido y más robusto.

## 13.3 Modelo mental

Piensa en un **taller con mesas limitadas** junto al almacén de cajas del capítulo 12.

- El taller tiene **N mesas idénticas** (frames). Sobre cada mesa hay, como mucho, **una caja abierta** (una página cargada del almacén). Un tablón anuncia qué caja hay en cada mesa (la **page table**).
- Cada mesa tiene tres chapaletas:
  - **«EN USO»** — el `pin_count`. Si alguien está trabajando en la mesa, el inspector no puede tocarla, aunque sea la más vieja del taller.
  - **«SUCIA»** — el `dirty` bit. La copia del almacén ya no coincide con la de la mesa: antes de reutilizar esa mesa hay que devolver la caja.
  - **«usada recientemente»** — el `ref_bit`, la chapaleta de recencia que da nombre a la segunda oportunidad.
- Un **inspector con aguja** recorre las mesas en círculo, como las manecillas de un reloj. Cuando hace falta hueco: mesa con «EN USO» → la salta; mesa con la chapaleta de recencia puesta → **la quita y sigue** (segunda oportunidad: estuvo activa hace poco, que viva una vuelta más); mesa limpia y sin chapaleta → **esa mesa se libera** (la víctima; si estaba sucia, antes se devuelve la caja al almacén).

```
        caller (cap. 14: CSR, cap. 15: índices)
              │ get_page / unpin
              ▼
      ┌─────────────────────────┐
      │ BufferPool<P: Pager>    │   frames (N × 4096 B) + page table
      │   ⌐clock_hand→┐         │   PolicyKind::Clock | Lru
      │   [f0]→[f1]→[f2]…↺      │   Metrics (hits/misses/…)
      └───────────┬─────────────┘
                  │ read / write / sync        ← el port del cap. 12
          ┌───────┴────────┐
          │ FilePager      │ MemoryPager (tests)
          └───────┬────────┘
                disco
```

Y aquí está el momento ¡ajá! del capítulo: la chapaleta de recencia solo sabe contestar **sí o no** («¿esta mesa se usó desde la última pasada de la aguja?»). Quien convierte ese sí/no en un **orden** — quién es más vieja — es la **posición de la aguja** respecto a cada mesa. El reloj es una estampa de tiempo barata: *distancia hasta la aguja ≈ antigüedad*. Cuando lleguemos al código veremos que esa frase, tal cual, es la que nos faltó escribir la primera vez.

Es el mismo truco que los sistemas operativos usan para la memoria virtual desde 1969: el algoritmo de Corbató. Las bases de datos lo copiaron porque el problema es idéntico — un medio lento, una memoria pequeña y rápida, y una pregunta incómoda: ¿a quién echo?

## 13.4 Primera solución

Lo más simple que funciona: cachearlo todo, para siempre.

```rust
// Solución ingenua: un mapa sin límite ni modales.
struct Cache {
    pages: HashMap<PageId, Vec<u8>>,
}
```

Cada página leída se guarda; cada petición mira el mapa primero. Cero política de reemplazo, cero contadores. Los tests pasan con ficheros de cinco páginas. Durante un rato, nadie se queja.

## 13.5 Sus límites

Tres muros, en orden de aparición:

1. **La memoria es finita aunque el disco no.** Un fichero de 2 GiB en una máquina con 512 MiB libres mata el proceso (OOM). Necesitamos **evicción**: decidir quién sale. Y esa decisión no puede ser «el primero que llegó» (FIFO) ni «cualquiera» (aleatorio): expulsarías la página raíz del índice que se consulta en cada búsqueda.
2. **El caller muta el `Vec<u8>` cacheado y nadie se entera.** Sin un `dirty` bit, la expulsión tira la página sin escribirla y las modificaciones se evaporan en silencio. La alternativa obvia — escribir en disco en cada modificación (write-through) — convierte cada cambio de 8 bytes en una escritura de 4 KB y, si somos honestos con la durabilidad, en un `fsync` de milisegundos. Ninguna de las dos vale.
3. **La expulsión descontrolada pisa al que trabaja.** Si el pool reutiliza una mesa mientras alguien la ocupa, sobrescribe los 4 KB bajo sus pies: el caller lee bytes de la página nueva creyendo que son la vieja, y el `mark_dirty` posterior puede escribir una página bajo el id de otra. Corrupción silenciosa con aspecto de grafo válido: la bestia de siempre, ahora en RAM.

La solución real responde a los tres muros con tres mecanismos: **frames fijos + política de reemplazo**, **dirty bit con write-back**, **pin count**. Y un cuarto ingrediente sin el cual los tres anteriores serían ciegos: **métricas**.

## 13.6 Solución evolucionada

El diseño canónico, el mismo de System R a PostgreSQL: una tabla de frames más una book-keeping mínima por frame.

```rust
pub struct BufferPool<P: Pager> {
    pager: P,
    frames: Vec<Frame>,               // mesas: capacidad fija
    page_table: Vec<Option<FrameId>>, // PageId → FrameId (None = no cargada)
    policy: PolicyKind,               // Clock (defecto) | Lru
    clock_hand: usize,                // la aguja
    lru_counter: u64,                 // reloj lógico para LRU
    metrics: Metrics,                 // contadores monotónicos
}
```

Cada `Frame` guarda la página (`data: [u8; PAGE_SIZE]` — exactamente lo que `SlottedPage::encode` produce en el cap. 11), su `page_id: Option<PageId>` (`None` = mesa vacía), el `pin_count: u32`, el `dirty: bool`, el `ref_bit: bool` y el `lru_counter: u64`.

El protocolo de uso es un contrato de tres pasos, y todo el capítulo 14 lo usará:

```
let buf = pool.get_page(id)?;    // 1. HIT o MISS; el frame queda PINEADO
buf[..4].copy_from_slice(&x);     // 2. lee/escribe los 4096 bytes
pool.unpin(id, true)?;           // 3. despinea; true = "la modifiqué"
```

**¿Por qué pin/unpin explícitos y no un guard RAII?** Porque el lifetime debe ser *visible*. Un `PageGuard<'_>` que despinea en su `Drop` sería elegante, pero prestaría `&mut pool` durante toda la vida de la página: no podrías tener DOS páginas pineadas a la vez (y el CSR del cap. 14 itera dos columnas en paralelo) sin `Arc<Mutex<Frame>>` ni `unsafe`, y cada firma del motor arrastraría lifetimes. El coste del pin explícito es que puedes olvidarte: el pin leak. Pero ese olvido tiene detector — `PoolFullOfPinned` — y el error llega en el test que lo provoca, no en producción. Además, así aprendes el mecanismo que PostgreSQL usa de verdad: un contador de referencia por buffer que el clock sweep respeta.

**¿Por qué `PoolFullOfPinned` es un error y no una espera?** Porque aquí no hay nadie que pueda despinear mientras esperas: el pool vive tras `&mut self`, un solo hilo. «Esperar» sería un deadlock contigo mismo. El error tipado es lo honesto: o te falta un `unpin` (el error es su detector), o pediste más páginas pineadas de las que caben y tienes que soltar algo antes. Cuando llegue la concurrencia (cap. 28), esperar volverá a tener sentido.

**¿Por qué dirty + write-back y no write-through?** Por aritmética. Write-through honesto = escritura + `fsync` por mutación: milisegundos por cambiar 8 bytes. Write-back difiere la escritura a la expulsión o al `flush`, y de regalo **fusiona mil modificaciones en una escritura** (write coalescing). El precio: la ventana de caída — un crash antes del flush pierde lo escrito desde el último flush. Esa ventana es exactamente lo que el WAL del cap. 28 vendrá a cerrar.

## 13.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap13_buffer_pool.rs` (24 tests). Lo leemos por partes.

### `get_page`, paso a paso

El corazón del pool distingue hit de miss en la primera línea:

```rust
if let Some(fid) = self.find_frame(id) {          // 1. HIT: ya está en memoria
    self.metrics.buffer_hits += 1;
    self.touch_frame(fid);                        // marca de uso para la política
    self.frames[fid].pin_count += 1;              // pin automático
    return Ok(&mut self.frames[fid].data);
}
if !self.pager.is_allocated(id) {                 // 2. MISS: ¿existe siquiera?
    return Err(BufferPoolError::UnknownPage(id));
}
```

Fíjate en el orden: `is_allocated` **antes** de buscar frame — validar en la frontera, como el `read` del cap. 12 validaba antes de tocar el fichero. Luego viene la parte con drama: buscar hueco. Primero un frame vacío (`find_free_frame`: mesa sin caja, víctima gratis, sin flush); si no hay, la política elige (`pick_victim`); si tampoco puede, `PoolFullOfPinned`. Y la víctima sucia se escribe **antes** de sobrescribirla:

```rust
if victim_dirty {
    self.pager.write(vp, &victim_data)?;   // los cambios del usuario NO se tiran
    self.metrics.page_writes += 1;
}
```

Tras cargar (paso 4: `pager.read` + `page_reads += 1`), el paso 5 resetea el frame: `pin_count = 1`, `dirty = false`, y — detalle importante — `ref_bit = true`: la página recién llegada cuenta como usada, una vuelta de gracia para que un scan de N+1 páginas sobre N frames no expulse cada página recién traída antes de usarla.

### El bug de la aguja: por qué avanza en CADA acceso

Aquí está la decisión más fina del módulo, la que implementamos mal la primera vez. La versión «de libro» del Clock mueve la aguja solo al buscar víctima. Nosotros la movemos también en cada `touch_frame` y en cada miss-load:

```rust
fn touch_frame(&mut self, fid: FrameId) {
    match self.policy {
        PolicyKind::Clock => {
            self.frames[fid].ref_bit = true;
            self.clock_hand = (self.clock_hand + 1) % self.frames.len();
        }
        PolicyKind::Lru => {
            self.lru_counter += 1;
            self.frames[fid].lru_counter = self.lru_counter;
        }
    }
}
```

¿Por qué? Recuerda el modelo mental: el bit dice sí/no; la aguja da el orden. En un workload de hits en ráfagas (el normal: la mayoría de accesos son hits, las expulsiones son raras), si la aguja solo se mueve al buscar víctima, entre búsqueda y búsqueda **todos** los frames acumulan `ref_bit = true`, y el barrido arranca desde una posición congelada en el pasado — la del orden de *carga*, no del de *uso*. Dos páginas accedidas hace un segundo y hace una hora son indistinguibles: ambos bits están a true. El barrido limpia todo lo que encuentra y expulsa a quien esté delante de la aguja: evicción efectivamente aleatoria, LRU perdido. Avanzando la aguja en cada acceso, la aguja pasa por delante del frame recién usado y el círculo empieza a reflejar el orden de uso: la víctima se busca empezando por lo más viejo. El comentario del módulo lo dice sin anestesia: sin avance en acceso, «dos frames accedidos en tiempos distintos parecerían idénticos». Lo vivimos: los tests básicos pasaban igual (contaban hits y misses, no *qué* salía), y fue el test de segunda oportunidad el que cazó la víctima incorrecta.

El barrido completo, con su cota:

```rust
for _ in 0..(2 * n) {
    let cand = self.clock_hand % n;
    if self.frames[cand].pin_count == 0 {
        if self.frames[cand].ref_bit {
            self.frames[cand].ref_bit = false;      // second chance
            self.clock_hand = (self.clock_hand + 1) % n;
            continue;
        }
        self.clock_hand = (self.clock_hand + 1) % n;
        return Some(cand);                          // víctima
    }
    self.clock_hand = (self.clock_hand + 1) % n;    // pineada: saltar
}
None  // todos pineados (o todos con segunda oportunidad perpetua) → error
```

Dos vueltas como máximo: la primera gasta segundos chances, la segunda encuentra mesa. Coste O(1) amortizado, cero listas, cero cerrojos — la razón por la que PostgreSQL cambió su LRU por esto.

### `unpin`, `mark_dirty` y la disciplina del borrow

```rust
pub fn unpin(&mut self, id: PageId, dirty: bool) -> Result<(), BufferPoolError> {
    let fid = self.find_frame(id).ok_or(BufferPoolError::UnknownPage(id))?;
    if self.frames[fid].pin_count == 0 {
        return Err(BufferPoolError::BadPinCount { page_id: id, current: 0 });
    }
    self.frames[fid].pin_count -= 1;
    if dirty { self.frames[fid].dirty = true; }
    Ok(())
}
```

El doble `unpin` no se perdona (`BadPinCount`): es el síntoma de dos dueños o de un pin fantasma. Y el flag `dirty` se puede poner aquí o con `mark_dirty(id)` — dos caminos al mismo estado, para el patrón real de uso que ya viste en los tests del módulo:

```rust
{   // el bloque existe para SOLTAR el préstamo de get_page antes de
    // volver a llamar al pool: nos lo enseñó el compilador a base de errores.
    let buf = pool.get_page(id).unwrap();
    buf[..4].copy_from_slice(&42u32.to_le_bytes());
}
pool.mark_dirty(id).unwrap();
pool.unpin(id, true).unwrap();
```

Ese bloque `{ }` es la respuesta práctica a «¿y por qué no RAII?»: mientras vives el `&mut` de `get_page`, el pool entero está prestado. La disciplina es tuya — y el compilador te la cobra.

### `flush`, `flush_page`, `discard`

`flush` escribe todas las sucias y devuelve cuántas; fíjate en el detalle de Rust: recolecta primero los `(frame_id, page_id)` sucios para no pelearse con el borrow al escribir y limpiar el flag a la vez. Y la última línea es la mitad de la promesa:

```rust
self.pager.sync()?;   // fsync: sin esto, "flush" sería un nombre mentiroso
```

Recordatorio del cap. 12: `pager.write` solo deposita bytes en la page cache del kernel; el corte de luz se los lleva. La promesa de `flush` es «a salvo», así que N writes + 1 sync. `flush_page(id)` es la versión selectiva (devuelve `false` si no estaba sucia). `discard(id)` invalida un frame sin escribirlo — útil cuando alguien reescribe la página por fuera (lo necesitará el `replace()` del CSR en el cap. 14) — y **rechaza** las páginas sucias con un truco honesto que el propio comentario confiesa: como el enum no tenía variante «Dirty», lo señala con `BadPinCount { current: u32::MAX }`. Una deuda de API documentada en el propio código: obligar a `flush_page` explícito antes de descartar es la política conservadora — preferir molestar a corromper.

### `Metrics`: qué mide y qué no

Cinco contadores monotónicos (`buffer_hits`, `buffer_misses`, `page_reads`, `page_writes`, `evictions`) y un ratio:

```rust
pub fn hit_ratio(&self) -> f64 {
    let total = self.buffer_hits + self.buffer_misses;
    if total == 0 { 0.0 } else { self.buffer_hits as f64 / total as f64 }
}
```

Fíjate en dos sutilezas. Primera: `page_reads` y `buffer_misses` cuentan cosas distintas aunque aquí siempre coincidan (cada miss lee una página exacta una vez) — divergirían el día que haya prefetch, y medir cosas distintas por separado es lo que permite diagnosticar. Segunda: **el ratio no lo dice todo**. No mide el frío (la primera lectura de cada página es miss por definición: un fichero leído una vez da ratio 0), no distingue misses aleatorios de secuenciales (un scan que toca cada página una vez da 0.0 con lecturas contiguas, relativamente baratas), no mide escrituras (`page_writes` va en otro contador) y no te dice si el pool está bien dimensionado para el mañana: un hot set diminuto infla el ratio mientras el resto del fichero nunca entra. En producción lo mismo: el hit ratio de `pg_stat_database` (`blks_hit`/`blks_read`) es la primera pregunta, no el veredicto.

### `MemoryPager`: el port paga su primera factura

Los tests no usan `FilePager` salvo para los dos end-to-end. Usan esto:

```rust
struct MemoryPager {
    pages: Vec<Option<[u8; PAGE_SIZE]>>,
    free_list: Vec<PageId>,
}
```

Un `Vec` de páginas con la misma semántica del trait (`read` copia, `write` valida buffer y propiedad, `sync` es `Ok(())` porque no hay disco que engañar). Con él, `small_pool()` monta un pool de 3 frames sobre 5 páginas, y la batería de evicción, dirty-flush y `PoolFullOfPinned` corre en microsegundos. Es la promesa del cap. 12 hecha código: el trait `Pager` existía *para que esto fuera posible*.

## 13.8 Prueba de fuego

Los tests del módulo congelan las promesas una a una (los nombres son del fichero real):

- **Hit/miss contados**: `bp_basic_get_unpin` — dos gets de la misma página → `page_reads = 1`, `buffer_misses = 1`, `buffer_hits = 1`, `page_writes = 0`.
- **El dirty llega a disco en la expulsión**: `bp_dirty_page_is_flushed_on_eviction` — pool de 1 frame, modifica p1 (`buf[0] = 0xAA`), marca sucia, pide p2 (la expulsa), y luego lee p1 *del pager directamente*: el `0xAA` está ahí. Sin el flush de víctima, ese byte sería 0.
- **El pin manda**: `bp_pool_full_of_pinned` — dos páginas pineadas en un pool de 2; el tercer `get_page` es `Err(PoolFullOfPinned)`. Y `bp_unpin_all_resets_pins` muestra la salida de emergencia.
- **La aguja protege a lo caliente**: `bp_clock_second_chance_protects_hot_page` — capacidad 2; tras cargar A y B, tocar A, y cargar C, los misses totales son exactamente 3: A sobrevivió. Es el test que cazó el bug de la aguja.
- **LRU de verdad**: `bp_lru_policy_evicts_least_recent` — mismo escenario con `PolicyKind::Lru`: sale la menos reciente, por diseño y no por aproximación.
- **End-to-end**: `bp_persistence_via_filepager` (create → pool → escribir 3 páginas → flush → reabrir con `FilePager` y verificar) y `bp_reload_via_pool` (reabrir también el pool: `occupied() == 0` y el primer get es miss — la caché no sobrevive al proceso, por contrato).

¿Qué pasaría si te saltaras este capítulo? El síntoma es medible: cien accesos a la misma página, cien `page_reads`. Y el funcional: el cap. 14 construiría un CSR que paga milisegundos por página tocada.

## 13.9 Qué hemos sacrificado

1. **LRU con escaneo O(n) por evicción**: el contador monotónico es transparente pero lineal; producción usaría lista doblemente enlazada.
2. **Un solo hilo**: `&mut self` en todo; sin átomos ni esperas. El wrapper concurrente llega en el cap. 28.
3. **Dos deudas menores confesadas en el código**: el `discard` sucio señalado con el sentinel `u32::MAX`, y la `page_table` como `Vec` (8 bytes por posible `PageId`: perfecto para ids densos como los nuestros, derrochador en espacios dispersos).
4. **Sin background writer**: las escrituras se agrupan en `flush` y en expulsiones — el pico de latencia de un flush grande llegaría de golpe.
5. **Métricas de vida, no de ventana**: los contadores son monotónicos desde la creación; para tendencia haría falta sampling periódico.

## 13.10 Cómo lo hace una BBDD real

- **PostgreSQL** es este capítulo con veinte años de producción encima. Su buffer manager cachéa páginas de 8 KB en `shared_buffers`; la política es el **clock sweep** que adoptó en la 8.1 (la historia del comienzo): como nuestro Clock, pero con el bit de 1 ampliado a un **usage count de 0 a 5** — usar la página lo sube, la aguja al pasar lo baja, y la víctima es el primer buffer con contador a 0 y sin pins (pins de verdad: un contador de referencia por buffer). El hit ratio se vigila en `pg_stat_database` (`blks_hit` frente a `blks_read`); la extensión `pg_buffercache` te deja mirar dentro del pool.
- **InnoDB (MySQL)** cachéa páginas de 16 KB con un LRU **con punto medio**: las páginas nuevas entran al 37 % «viejo» de la lista (`innodb_old_blocks_pct`), y solo ascienden a la zona joven si vuelven a usarse tras `innodb_old_blocks_time` (1 s por defecto). Es una vacuna contra el full scan que arrastraría la caché entera — el mismo problema que nuestro `ref_bit` al cargar ataca de forma más tosca.
- **SQLite** mantiene una caché de páginas por conexión (2.000 KiB por defecto) en su módulo pcache, encima del pager del cap. 12.
- **Kùzu**, la base de datos de grafos que tomamos como referencia, usa una variante **GClock** (contador de uso en vez de bit), como anota el comentario del módulo.

En todos, el patrón es el que acabas de construir: frames + tabla + pins + dirty + una aguja. Lo que cambia es cuántos bits le dan a la recencia.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: PostgreSQL tenía LRU «mejor» en teoría y lo cambió por una aproximación. Resume el argumento en una frase y di qué le falta a Clock a cambio.
- *Intermedio*: reproduce en LiraDB el ataque del que se vacuna InnoDB: un working set caliente de 10 páginas en un pool de 64, y un scan de 100 páginas que lo arrasa. Mide el hit ratio antes y después con `metrics()`. ¿Qué política lo sufre más?
- *Experto*: implementa `PolicyKind::GClock` al estilo PostgreSQL: contador de uso saturado (máx 5) en vez de bit; la aguja decrementa, la víctima es el contador a 0. Haz pasar los 24 tests sin tocar aserciones y añade uno que demuestre que una página tocada k veces sobrevive k barridos.

## 13.11 Lo que te llevas

- El buffer pool intercala **memoria entre las estructuras y el pager**: la diferencia entre ~100 ns y milisegundos por acceso.
- Se cachean **páginas enteras, no registros**: el disco entrega bloques, la localidad amortiza, y el pool queda contenido-agnóstico.
- El protocolo **pin → uso → unpin(dirty)**: el pin blinda contra la expulsión bajo tus pies; el leak tiene detector (`PoolFullOfPinned`), el doble unpin tiene error (`BadPinCount`).
- **Write-back con dirty bit**: mil modificaciones, una escritura; el precio es la ventana de caída que el WAL (cap. 28) cerrará. Toda víctima sucia se escribe antes de reutilizarse.
- **Clock = LRU aproximado a O(1)**: un bit, una aguja, segunda oportunidad. Y la aguja avanza **en cada acceso**, no solo al buscar víctima — o la recencia se evapora.
- **flush termina en `sync`**: sin fsync, «a salvo» es un nombre mentiroso.
- **hit_ratio** mide la fracción de accesos servida de memoria — y nada más: ni frío, ni secuencialidad, ni escrituras, ni dimensionamiento.

## 13.12 Ojo, cuidado con…

- **El `unpin` olvidado**: pin leak → `PoolFullOfPinned` en el peor momento. Un `get_page` con `Ok`, un `unpin` — siembre.
- **Mutar sin `dirty`**: ni `unpin(id, true)` ni `mark_dirty` → la expulsión tira tus cambios sin error. El fallo más silencioso del capítulo.
- **Confundir hit con `page_read`**: uno es servir de RAM; otro, llamar al pager. Hoy coinciden; no escribas código que lo asuma para siempre.
- **Tratar Clock como LRU exacto**: promete segunda oportunidad, no la víctima LRU. El test original del módulo lo asumía y mentía.
- **Confundir evicción con flush**: se expulsa sin escribir (víctima limpia) y se escribe sin expulsar (`flush`). Son ejes distintos.

## 13.13 Pin de batalla

> *«Una caché no hace tus datos más rápidos: hace que tus discos parezcan innecesarios. Cuida a quién expulsas, porque la lentitud también se cachea.»*

## 13.14 Si solo lees 30 segundos

Entre las estructuras y el pager vive el `BufferPool<P: Pager>`: N frames de 4.096 bytes, una page table, y el protocolo pin → uso → unpin(dirty). Cada `get_page` es hit (pin + touch) o miss (leer del pager, quizá expulsar a una víctima — sucia se escribe primero — con Clock: la aguja salta pineados, da segunda oportunidad a los recientes y elige al primero sin bit). `flush` escribe las sucias y sincroniza; `metrics().hit_ratio()` te dice qué fracción de accesos no fueron al disco — solo eso, pero eso ya lo es casi todo.

## 13.15 Una historia pequeña

La primera versión del Clock de LiraDB movía la aguja solo dentro de `pick_victim` — la versión de libro, juramos. Los tests de hits, misses y expulsiones pasaban todos: los contadores no dicen *quién* salió. Y entonces escribimos el test de la segunda oportunidad: cargar A y B en dos frames, tocar A, cargar C, y comprobar que A sobrevivía. Falló: C había entrado expulsando a A, la página que acabábamos de usar. Nos sentamos a trazar la aguja a mano y el problema era tan simple como incómodo: como la aguja no se movía con los accesos, apuntaba a donde la dejó la última *carga*, y el barrido se gastaba el bit de A de primero. Dos páginas tocadas a un segundo y a una hora de distancia eran, para el algoritmo, la misma página. El arreglo fueron dos líneas (avanzar en `touch_frame` y en el miss-load); la lección quedó escrita en el comentario del módulo: en Clock, la recencia no la guarda el bit — la guarda la aguja. Desde entonces, cuando algo «aproximadamente correcto» falla, buscamos qué parte del estado llevábamos congelada.

## Ejercicios resueltos

**1. Pool de capacidad 2, política Clock, páginas A, B y C en el pager. Secuencia: get(A) + unpin, get(B) + unpin, get(A) + unpin (hit), get(C). ¿Qué página se expulsa y por qué?**

Sigamos la aguja (empieza en 0): get(A) es miss → frame 0, `ref_bit = true`, y la aguja avanza a 1. get(B) → frame 1, bit a true, aguja a 0. get(A) es **hit** → touch: bit a true (ya lo estaba) y la aguja avanza a 1 — A queda *detrás* de la aguja, protegida hasta el final del próximo barrido. get(C): sin frame libre, el barrido arranca en la aguja (frame 1 = B): bit a true → se lo quita, avanza; frame 0 (A): bit a true → se lo quita, avanza; vuelta al frame 1: bit a false → **víctima = B**. C entra en su frame y los misses totales son 3 (A, B, C): A sobrevivió. Sin el avance en el hit (el bug), la aguja habría quedado en 0 tras cargar B: el barrido habría gastado el bit de A primero y la víctima habría sido **A**, la página caliente. Verifícalo con `bp_clock_second_chance_protects_hot_page`.

**2. Un pool acumula `hits = 990, misses = 10`. Otro, `hits = 0, misses = 100` tras escanear un fichero por un pool de 10 frames. ¿Cuál está «peor»?**

El ratio dice 0.99 y 0.0, pero el 0.0 puede ser saludable: son lecturas *secuenciales* de un fichero que no cabe (y no cabe por diseño: 100 páginas, 10 frames — ningún algoritmo de reemplazo evita un solo miss por página en un scan puro). El 0.99, en cambio, esconde la pregunta importante: ¿y las escrituras? Un `page_writes` alto con `evictions` altas diría que las víctimas salen sucias y el disco está en el camino crítico. El ratio mide *una* cosa — accesos servidos de memoria — y se lee junto a `page_reads`, `page_writes` y la forma del workload, nunca solo. (El test `metrics_hit_ratio` fija la aritmética: 3/4 = 0.75, y 0 accesos = 0.0, no NaN.)

## Ejercicios propuestos

**Esencial (retrieval, cap. 11).** Sin mirar los capítulos 11 y 12: escribe de memoria el layout de la cabecera de página — los campos, su ancho en bytes y para qué sirve cada uno — y explica por qué `Frame.data` es exactamente `[u8; 4096]`. Después convierte el recuerdo en test: con un `MemoryPager`, guarda una `SlottedPage` codificada pasando por el pool (`get_page` + `mark_dirty` + `unpin` + `flush`) y recupérala decodificando la cabecera *desde el buffer que te devuelve un segundo `get_page`* — que debe ser hit, y las métricas deben demostrarlo. Si no recuerdas los campos, eso es señal de relectura, no de pista.

**Intermedio (spacing + interleaving).** Implementa `fn read_record<P: Pager>(pool: &mut BufferPool<P>, page: PageId, index: usize) -> Result<Vec<u8>, BufferPoolError>` que cargue la página por el pool y extraiga el registro `index` caminando los prefijos de longitud del cap. 11 — sin decodificar la página entera ni tocar el pager directamente. Escribe un test que inserte tres registros de longitudes distintas, los lea de uno en uno y verifique con `metrics()` que la segunda lectura de la misma página no produjo `page_read` nuevo. ¿Dónde acaba viviendo el offset del registro: en el pool, en el caller o en ninguna parte?

**Experto.** El `discard` actual rechaza páginas sucias con el sentinel `BadPinCount { current: u32::MAX }`. Añade la variante honesta `BufferPoolError::PageDirty(PageId)`, migra el sentinel, actualiza `bp_discard_dirty_rechazado`… y discute en un comentario del código las dos semánticas posibles de descartar una sucia: flush implícito (cómodo, esconde coste) o rechazo explícito (verboso, obliga a decidir). ¿Cuál elegiría el principio «leak mejor que corrupción» del cap. 12… y cuál elegiría un DBA a las 3 de la madrugada?

## Para profundizar

- **The Internals of PostgreSQL, cap. 8** (interdb.jp) — el buffer manager de PostgreSQL con diagramas: clock sweep con usage count, pins, y el ciclo de vida de un buffer.
- **PostgreSQL wiki, «Multiple Buffer Pools»** y el **LWN.net de la 8.0.2** (lwn.net/Articles/131554) — la historia ARC → 2Q → clock sweep contada por quienes la vivieron; la patente US 6.996.676 caducó en 2024.
- **Wikipedia, «Page replacement algorithm»** — el origen: Corbató describió el algoritmo de reloj en 1969; Carr y Scott lo elevaron a WSCLOCK (CACM, 1981).
- **Megiddo y Modha, «ARC: A Self-Tuning, Low Overhead Replacement Cache»** (FAST '03) — el algoritmo que perdió contra el reloj.
- **Manual de MySQL, «The InnoDB Buffer Pool»** — el LRU con punto medio: `innodb_old_blocks_pct` y `innodb_old_blocks_time`.
- **Alex Petrov, «Database Internals» (O'Reilly), cap. 2, y CMU 15-445** — caching y buffer management en motores reales; el proyecto BusTub de CMU es este capítulo a escala de curso.

## Mini-diálogo: en guardia nocturna

> — Cumpliste lo prometido: el pager falso. La caché entera probándose contra RAM, sin un solo tempfile. Tardé en creerlo.

> — El mérito es del trait, no mío. El cap. 12 dejó el enchufe; hoy solo hemos enchufado otra cosa. Fíjate qué más: ni el pool ni los 24 tests saben si abajo hay un `FilePager` o un `MemoryPager`.

> — Pero reconoce el bug de la aguja. Hiciste el Clock «de libro» y el libro te salió rana.

> — Y fue el mejor día del capítulo. Los tests de contadores pasaban todos; el de segunda oportunidad fue el único que preguntó *quién* salió, no *cuántos*. Moraleja: un test que solo cuenta aciertos no audita una política.

> — Lo que aún no trago es eso de acordarme del `unpin` a mano. Un guard lo haría imposible.

> — Imposible de olvidar, sí. E imposible de tener dos páginas a la vez, e imposible de leer sin tres lifetimes por firma. Rust ya nos presta el pool entero en cada `get_page` — ¿viste el bloque `{ }` de los tests? El pin explícito te enseña lo que PostgreSQL hace por dentro; el guard te lo escondería.

> — Vale. Y el hit ratio, ¿ese número es la nota del capítulo?

> — Es la primera pregunta del interrogatorio, no el veredicto. Un 0.99 con escrituras desbocadas es un enfermo con buena cara. Lo aprendimos leyendo a InnoDB: él ni siquiera se fía del LRU…

> — …y por eso mete las páginas nuevas por la mitad de la lista. O sea, que ya ni los que pueden pagar LRU estricto lo quieren.

> — Exacto. Ahora duerme: mañana llega el primer cliente de verdad. El CSR va a pedirte cuatro columnas página a página… y a reescribirlas enteras cuando el grafo cambie. Pregunta de guardia: cuando `replace()` pise una página que sigue cacheada en un frame, ¿quién limpia el frame?

> — …¿`discard`? Para eso nació, ¿no?

> — Para eso nació. Buenas noches.

---

*(Próximo capítulo: 14 — Cómo almacenar adyacencias. El CSR persistente será el primer cliente real del buffer pool: cuatro columnas de arrays repartidas en páginas, leídas con `get_page`/`unpin`, y reescritas por completo con `replace()` — el momento en que `discard` justificará su existencia.)*
