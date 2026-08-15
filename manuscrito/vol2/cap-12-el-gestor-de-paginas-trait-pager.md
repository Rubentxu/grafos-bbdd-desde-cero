# Capítulo 12 — El gestor de páginas (trait `Pager`)

> *«Todo el mundo quiere escribir páginas. Nadie quiere llevar la cuenta de cuáles están libres.»*

## 12.0 La anécdota de la esquina

Año 2000. D. Richard Hipp trabaja como contratista para Bath Iron Works, el astillero de Maine que construye destructores de la clase Arleigh Burke. Su equipo escribe software para el USS Oscar Austin (DDG-79), y los datos de tuberías y válvulas del barco viven en una base de datos Informix instalada en un servidor. El problema: cuando el servidor se caía —y en un barco las cosas se caen—, la aplicación solo sabía mostrar «no puedo conectar», y el equipo de Hipp cargaba con la culpa de un fallo que no controlaba. En un destructor, donde las decisiones de control de daños no esperan a nadie, no hay administrador de sistemas que reinicie el servidor de bases de datos en mitad del humo.

Hipp se hizo la pregunta que lo cambió todo: si su aplicación apenas leía datos y los metía en RAM, ¿para qué necesitaba un servidor? Lo que quería era una base de datos sin dependencias que pudieran fallar: incrustada en el proceso, sin instalación, sin administración. Escribió SQLite. Y si hoy abres el diagrama de arquitectura de SQLite, entre la capa B-tree y la interfaz con el sistema operativo hay un módulo con nombre propio: el Pager (`pager.c`). Es la pieza que convierte «un fichero cualquiera» en «una colección numerada de páginas». Es exactamente lo que vamos a construir para LiraDB.

## 12.1 Objetivo

En el capítulo 11 construiste la página: 4.096 bytes con cabecera, registros con prefijo de longitud y una metapágina en la página 0. Pero una página quieta no sirve de nada. Falta el personaje que las reparte: quién decide en qué byte del fichero vive cada página, cuáles están ocupadas, cuáles están libres y cuándo lo escrito está de verdad en disco.

Ese personaje es el Pager. Vas a construir tres piezas:

1. `PageId` y `PagerError` — el vocabulario: cómo se nombra una página y cómo se queja el gestor.
2. El trait `Pager` — el contrato de ocho operaciones: `allocate`, `read`, `write`, `sync`, `free`, `num_pages`, `is_allocated`, `page_size`.
3. `FilePager` — la primera implementación real, sobre `std::fs::File`.

## 12.2 Problema

Tienes `SlottedPage::encode()` que devuelve 4.096 bytes. La pregunta suena tan tonta como la del capítulo anterior: **¿quién los pone en el fichero?**

La respuesta ingenua: cada módulo, por su cuenta. El CSR calcula «mi página siguiente es `file_len / 4096`», hace `seek` y escribe. Tres semanas después, el índice hash hace el mismo cálculo. Y un buen día ambos calculan lo mismo: `file_len / 4096 == 7` para los dos. El CSR escribe su página en la 7, el índice escribe la suya encima, y nadie se entera hasta que el grafo devuelve datos del índice equivocado. Eso es el **doble uso de una página**: el fallo clásico de no tener un dueño del inventario.

Inventemos más desastres, porque hay para elegir:

- **Offsets a mano por todo el código**: `seek(id * 4096)` repetido en diez sitios. El día que `PAGE_SIZE` cambie, o que alguien multiplique `id * PAGE_SIZE` en un `u32` ya grande (2²⁰ páginas × 2¹² = 2³²: desbordamiento exacto), la corrupción está servida.
- **Páginas a medias tras un crash**: el proceso muere a mitad de extender el fichero y quedan 4.096·n + 2.000 bytes. ¿Quién lo detecta? Si nadie, la última «página» se lee a medias: un `read` normal devuelve los bytes que hay y punto.
- **Borrado sin registro**: liberas la página 5 porque un nodo se borró. ¿Quién apunta que la 5 está libre? Si nadie, el fichero solo crece; si cada uno se apunta sus libres en su cuaderno, vuelves al doble uso.

La raíz es la misma lección del cap. 11, un nivel más arriba: **mezclar decisiones que deben estar separadas**. Aquí: *qué hay dentro de una página* (cap. 11) frente a *qué páginas existen y quién las posee* (este capítulo). La base de datos real pone un cuello de botella deliberado entre ambas: todas las capas superiores piden páginas a un único gestor. Nosotros lo llamaremos `Pager`.

## 12.3 Modelo mental

Piensa en un **bibliotecario de un almacén de cajas**:

- El almacén es una nave con **cajas idénticas, numeradas 0, 1, 2…** apiladas en fila. La caja N está a `N × tamaño_de_caja` metros de la puerta: llegar a ella es aritmética, no búsqueda.
- Tú **nunca entras al almacén**. Pides la caja 12 al bibliotecario (`read`), la recibes, la modificas en tu mesa y se la devuelves (`write`).
- El bibliotecario lleva **una carpeta con las cajas vacías** (la free list). Cuando pides caja nueva (`allocate`), te da una de la carpeta si hay; si no, amplía la nave.
- Cuando devuelves una caja porque ya no la necesitas (`free`), él la anota en la carpeta. La caja sigue ocupando sitio en la nave: tirarla a la basura dejaría huecos imposibles de renumerar.
- Al cierre del día, el bibliotecario **recorre la nave comprobando que cada caja devuelta está realmente en su estante** (`sync`). Que tú la hayas dejado en el pasillo no basta.

En capas de código:

```
   SlottedPage / MetaPage / (cap. 13) BufferPool   ← callers: saben QUÉ hay en una página
   ───────────────────────────────────────────────
   trait Pager                    ← el PORT: el contrato, la carpeta, la aritmética
   ───────────────────────────────────────────────
   FilePager      (futuro: MmapPager, MemoryPager) ← ADAPTERs: saben CÓMO se llega al medio
   ───────────────────────────────────────────────
   un fichero = [pag 0][pag 1][pag 2][pag 3][pag 4]...
                 meta     CSR    LIBRE   índice
                                  ↑
                        free_list: [2]
```

El momento ¡ajá! de este capítulo: **el `PageId` ES el offset físico**. `offset_of(id) = id * PAGE_SIZE`, ni una indirección más. Por eso pedir y devolver cajas es O(1)… y por eso, cuando llegue la compactación (cap. 16), no podremos mover páginas de sitio sin reescribir todos los punteros del CSR y de los índices. Esa doble cara —barato ahora, rígido después— es la decisión estructural de todo el motor.

## 12.4 Primera solución

Lo más simple que funciona: funciones sueltas sobre un `File`.

```rust
// Solución ingenua: leer una página "a mano".
fn read_page(file: &mut File, id: u32, buf: &mut [u8]) -> std::io::Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(id as u64 * 4096))?;
    file.read_exact(buf)?;
    Ok(())
}
```

Siete líneas. Correctas, incluso. Escribe su hermana `write_page`, réplícalas en cada módulo, y durante un tiempo todo va bien. Los tests pasan.

## 12.5 Sus límites

Hasta que el sistema crece y la función inocente enseña los dientes:

1. **No hay inventario**: nada impide el doble uso de la página 7, ni responde «¿la 5 está libre?». Cada `allocate` es un `file.metadata()?.len() / 4096` — y dos módulos que lo hagan a la vez obtienen el mismo número.
2. **Cada caller re-implementa (u olvida) la validación**: ¿el buffer tiene 4.096 bytes? ¿el `id` existe? Con un buffer de 10 bytes, un `read` normal lee 10, devuelve «éxito» y el resto del buffer queda con basura de la operación anterior.
3. **`io::Error` lo explica todo y no dice nada**: «permission denied» (el disco arde), «página fuera de rango» (tu bug) y «página libre» (otro bug) llegan como el mismo tipo indistinguible. El caller no puede reaccionar: solo loguear.
4. **Estás soldado al disco**: el cap. 13 necesita probar la evicción de la caché cientos de veces por segundo; con `std::fs::File` real, cada test paga el sistema de ficheros. Y el día que quieras `mmap`, tendrás que reescribir todos los callers.

## 12.6 Solución evolucionada

La solución tiene dos gestos: **un contrato (trait) y un dueño del inventario**. Primero el contrato, que es el corazón del capítulo:

```rust
/// Trait del gestor de páginas (port en arquitectura hexagonal).
pub trait Pager {
    /// Asigna una nueva página (nunca reutiliza un ID en la free list).
    fn allocate(&mut self) -> Result<PageId, PagerError>;
    /// Lee la página `id` en el buffer `page` (debe tener `PAGE_SIZE` bytes).
    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError>;
    /// Escribe el buffer `page` (debe tener `PAGE_SIZE` bytes) en la página `id`.
    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError>;
    /// Sincroniza el estado del pager con disco (`fsync`/`fdatasync`).
    fn sync(&mut self) -> Result<(), PagerError>;
    /// Número total de páginas que el pager puede direccionar (incluyendo
    /// páginas en la free list, que existen en disco pero no están asignadas).
    fn num_pages(&self) -> u32;
    /// Libera una página: la marca como libre y la añade a la free list.
    /// La página sigue ocupando espacio en disco hasta un futuro `vacuum`.
    fn free(&mut self, id: PageId) -> Result<(), PagerError>;
    /// ¿Está la página `id` asignada (no en la free list)?
    fn is_allocated(&self, id: PageId) -> bool;
    /// Tamaño de página (en bytes) usado por este pager.
    fn page_size(&self) -> usize { PAGE_SIZE }
}
```

**¿Por qué un trait y no funciones sueltas?** Porque un trait es un *port* de la arquitectura hexagonal que ya usamos en el cap. 8 con `GraphStore`: las capas superiores dependen del contrato, no del medio. Los beneficios son concretos y con fecha: (a) el cap. 13 definirá un `MemoryPager` en sus tests para probar la evicción de la caché **sin tocar disco**; (b) un apéndice comparativo añadirá `MmapPager` sin cambiar ni una línea de los callers; (c) cada implementación puede decidir su propio `page_size()` (por eso es método con valor por defecto, no constante pegada al trait). La alternativa — funciones sobre `&mut File` — habría soldado el buffer pool al sistema de ficheros para siempre. SQLite hace lo mismo con su capa VFS: el pager habla con una interfaz de OS abstracta, y debajo hay una implementación por plataforma (`os_unix.c`, `os_win.c`).

Segunda decisión: la **free list vive en un `Vec<PageId>` y se consume en LIFO** (Last In, First Out). ¿Por qué LIFO y no FIFO? Tres razones: (1) `Vec::pop()` es O(1) con cero estructuras extra — una cola FIFO exigiría un `VecDeque` para lo mismo; (2) la página recién liberada es la más «caliente»: sigue probablemente en la caché del sistema operativo, así que reutilizarla primero es gratis, mientras que FIFO te manda siempre a la página más fría y olvidada; (3) el orden resultante es determinista y lo dicta una sola línea, no una política repartida por el código.

Y la decisión más incómoda del capítulo, escrita en los comentarios del propio módulo: **la free list NO se persiste**. Vive en memoria; si cierras y reabres el fichero, se pierde. ¿No es eso un bug? No: es una **deuda pedagógica documentada**, y el porqué merece su propio apartado. Lo vemos dentro de un momento, leyendo el código real.

## 12.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap12_pager.rs` (22 tests). Vamos a leerlo por partes; todos los fragmentos salen de ese módulo, con alguna línea larga compactada para caber en página.

### El vocabulario: `PageId` y `PagerError`

```rust
/// Identificador de página (índice 0-based; la página 0 es la metapágina).
pub type PageId = u32;
```

¿Por qué `u32` y no `usize`? Porque `PageId` es un tipo de **formato persistente**: el `page_id` de la cabecera del cap. 11 es un u32 little-endian en disco, y los tipos que viajan a disco deben ser de anchura fija (`usize` cambia con la plataforma — 32 bits ahí, 64 aquí). El límite es honesto y está escrito: 2³² páginas × 4 KB = **16 TiB** de fichero máximo. Cuando se agote, el error tiene nombre (`NoFreePageId`), no un overflow silencioso.

```rust
pub enum PagerError {
    Io(std::io::Error),                       // el entorno falló (disco, permisos)
    OutOfRange { requested: PageId, num_pages: u32 }, // pediste una página que no existe
    FreePage(PageId),                         // la página está en la free list
    BadBufferSize { expected: usize, got: usize },    // buffer ≠ PAGE_SIZE
    NoFreePageId,                             // 4 GiB de páginas agotados
}
```

**¿Por qué un error tipado y no `io::Error` directo?** Porque los callers necesitan distinguir *culpa propia* de *culpa del entorno*. `OutOfRange`, `FreePage` y `BadBufferSize` son bugs del caller: un `id` que no salió de `allocate`, un use-after-free, un buffer mal dimensionado. `Io` es el mundo exterior. Con `io::Error` a secas, el buffer pool del cap. 13 tendría que parsear strings para reaccionar distinto a cada caso. Además el enum implementa `Display`, `Error::source()` (que solo la variante `Io` devuelve — el resto no envuelve nada) y `From<io::Error>`, para que el operador `?` convierta los errores de E/S automáticamente. Es el mismo patrón que SQLite expone en `pager.h`: códigos de error propios, no errno del SO.

### `create`: por qué la página 0 se reserva desde el minuto uno

```rust
// Reservar la página 0 escribiéndola vacía. Esto fija el tamaño del
// fichero en PAGE_SIZE y deja la metapágina lista para `MetaPage`.
let zeros = vec![0u8; PAGE_SIZE];
file.write_all(&zeros)?;
file.sync_all()?;
Ok(Self { file, path, num_pages: 1, free_list: Vec::new() })
```

`create()` no deja un fichero vacío: deja un fichero con **exactamente una página, toda a ceros, y `num_pages == 1`**. ¿Por qué? Porque en el cap. 11 establecimos que la página 0 es *siempre* la metapágina — el punto de arranque en frío del fichero entero. Reservarla aquí fija dos invariantes de golpe: (1) ningún `allocate()` futuro devolverá jamás la página 0, así que ninguna estructura de datos pisará el catálogo del fichero; (2) `read(0, …)` funciona inmediatamente tras crear, sin casos especiales de «fichero vacío». La alternativa — crear el fichero con `num_pages == 0` y asignar la 0 a quien la pida primero — habría sembrado `if fichero_vacío` por todo el código… o peor, habría entregado la metapágina al CSR. Fíjate además en dos detalles con dientes: `create` abre con `.truncate(true)` (crear significa crear: destruye un fichero previo sin preguntar — está en su doc comment, y es la semántica que una CLI `liradb init` espera), y hace `sync_all()` antes de devolverse: la promesa «el fichero existe y tiene una página» no vale nada si el corte de luz la borra.

### `open`: por qué valida que el tamaño sea múltiplo de `PAGE_SIZE`

```rust
let len = file.metadata()?.len();
if len % PAGE_SIZE as u64 != 0 {
    return Err(PagerError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("file size {len} not a multiple of PAGE_SIZE={PAGE_SIZE}"),
    )));
}
```

Un fichero con 4.096·n + k bytes (0 < k < 4096) es un fichero **truncado**: un crash a mitad de `extend_by`, un `cp` interrumpido, un disco lleno. Si lo aceptáramos en silencio, `num_pages` mentiría y la corrupción se descubriría muy lejos de su causa, en un `decode` del cap. 11 que falla con «magic inválido» en la última página. Rechazar en el `open` con `InvalidData` y un mensaje que dice exactamente qué está mal es la diferencia entre diez segundos de diagnóstico y una tarde de autopsia.

### `allocate` y la carpeta de cajas libres

```rust
fn allocate(&mut self) -> Result<PageId, PagerError> {
    // Reutilizar free list primero (LIFO).
    if let Some(id) = self.free_list.pop() {
        return Ok(id);
    }
    // Si no hay IDs libres, extender el fichero.
    let id = self.num_pages;
    self.extend_by(1)?;
    Ok(id)
}
```

Dos caminos, en este orden: primero reutilizar (el fichero no crece), luego extender. Y extender (`extend_by`) trae su propio guardián contra el final de los tiempos: el contador de páginas solo crece con `checked_add(extra).ok_or(PagerError::NoFreePageId)?`. Sin ese chequeo, al agotarse el u32 el contador daría la vuelta y `allocate` empezaría a repartir IDs ya usados: el doble uso de página, ahora en versión apocalipsis. Con él, error tipado.

### `read`/`write`: tres checks baratos antes de tocar el fichero

```rust
if page.len() != PAGE_SIZE {
    return Err(PagerError::BadBufferSize { expected: PAGE_SIZE, got: page.len() });
}
if id >= self.num_pages {
    return Err(PagerError::OutOfRange { requested: id, num_pages: self.num_pages });
}
if self.free_list.contains(&id) {
    return Err(PagerError::FreePage(id));
}
self.file.seek(SeekFrom::Start(Self::offset_of(id)))?;
self.file.read_exact(page)?;
```

Orden deliberado: los checks más baratos y más probables primero, y **todos antes de la primera llamada al sistema**. Un bug del caller debe producir siempre el mismo error, sin importar el estado del disco. Y el remate es `read_exact`, no `read`: un `read` puede devolver menos bytes de los pedidos (un *short read*) y quedarse tan ancho; una página leída a medias es peor que ninguna, porque parece válida.

### `free` y `sync`: el inventario y la verdad

```rust
fn free(&mut self, id: PageId) -> Result<(), PagerError> {
    ...
    if self.free_list.contains(&id) {
        return Err(PagerError::FreePage(id));
    }
    self.free_list.push(id);
    Ok(())
}
```

Liberar es **anotar, no borrar**: ni se rellena la página de ceros (una escritura de 4 KB que nadie necesita: quien la reutilice la sobrescribe entera) ni se encoge el fichero (el `PageId` es un offset físico; truncar invalidaría los punteros del CSR del cap. 14). El segundo `free` de la misma página es error, no idempotencia silenciosa: liberar dos veces es el síntoma de dos dueños, y queremos oírlo.

```rust
fn sync(&mut self) -> Result<(), PagerError> {
    self.file.sync_all()?;
    Ok(())
}
```

**¿Por qué existe `sync` si el SO «ya escribió»?** Porque no lo escribió. Cuando `write_all` vuelve con éxito, los bytes están en la *page cache* del kernel: páginas sucias en RAM del sistema operativo, no en el disco. Si el proceso muere, el SO aún tiene los datos y los terminará escribiendo; pero si se apaga la máquina, se evaporan. `sync_all()` es `fsync(2)`: obliga al kernel a volcar esas páginas sucias al medio. ¿Y por qué no llamarlo dentro de cada `write`? Porque `fsync` cuesta órdenes de magnitud más que `write` (hablamos de milisegundos de espera a disco frente a microsegundos de copia a memoria): pagarlo por cada página de 4 KB colapsaría el rendimiento. El patrón correcto es N escrituras + 1 `sync` en el punto de compromiso — y ese «punto de compromiso» será, en los caps. 27-28, la transacción.

### La deuda documentada: la free list no sobrevive al reopen

El módulo cierra con la decisión más honesta del capítulo, escrita como test:

```rust
// Decisión pedagógica: la free list es en memoria. Un reopen la
// pierde (lo cual se corregirá en cap 14 cuando se persista en la
// metapágina). Aquí documentamos el comportamiento actual.
```

¿Por qué no persistirla ya, si la `MetaPage` del cap. 11 incluso tiene un campo `free_pages` esperándola? Porque el orden de las escrituras importa más que la prisa. Piensa qué exige persistir: reescribir la metapágina en cada `free()`, y garantiza que **la free list nueva y el dato que liberó la página llegan a disco de forma consistente**. Sin WAL (cap. 28) no tenemos esa garantía: si la free list llega antes que el dato y hay un crash en medio, la página aparece como libre y asignada a su dueño anterior a la vez — el siguiente `allocate` la entrega a otro dueño, y eso es **corrupción**. Si no llega nunca, la página queda huérfana: espacio perdido hasta una compactación (cap. 16). Entre las dos formas de fallar, elegimos a propósito la que **desperdicia espacio en vez de la que corrompe datos**. El leak se recupera; el doble dueño, no siempre. Esa asimetría — preferir leak a corrupción cuando no puedes garantizar atomicidad — es una regla de diseño que reaparecerá en todo el Vol. II.

## 12.8 Prueba de fuego

Los tests del módulo no verifican «que compila»: verifican las promesas del contrato, una por una. Tres ejemplos con sus aserciones textuales (los comentarios de conexión son míos):

```rust
// LIFO: la free list devuelve la última página liberada, y el fichero no crece.
pager.free(p2).unwrap();
assert!(!pager.is_allocated(p2));
let p4 = pager.allocate().unwrap();
assert_eq!(p4, p2, "LIFO: free list debe reutilizar p2");
assert_eq!(pager.num_pages(), 4); // no creció el fichero
```

```rust
// Persistencia real: create → allocate×4 → write → sync → cerrar → reabrir → releer.
{
    let mut p = FilePager::create(&path).unwrap();
    for id_alloc in 0..4 {
        let id = p.allocate().unwrap();
        let buf = pattern_page(id);
        p.write(id, &buf).unwrap();
        assert_eq!(id, id_alloc + 1);
    }
    p.sync().unwrap();
}
let mut p2 = FilePager::open(&path).unwrap();
assert_eq!(p2.num_pages(), 5);
```

```rust
// La deuda, en forma de test: tras reopen la free list está vacía...
let p2 = FilePager::open(&path).unwrap();
assert!(p2.free_list().is_empty(), "free list es en memoria");
// ...y la página liberada aparece como "asignada" (leak, no corrupción).
assert!(p2.is_allocated(1));
```

Añade la batería de errores —`read_en_pagina_libre_falla`, `read_buffer_mal_tamano_falla`, `open_archivo_con_tamanho_invalido_falla`, `free_sobre_id_no_asignado_o_fuera_de_rango`— y `escribir_metapagina_y_reabrir`, que integra con el cap. 11 escribiendo una `MetaPage` real (`num_pages: 42, free_pages: 5, root_page: 7`) en la página 0, reabre y la decodifica intacta.

¿Qué pasaría si te saltaras este capítulo? Los síntomas son concretos: offsets `id * 4096` calculados a mano por todo el código, dos estructuras escribiendo en la misma página liberada, y un fichero de 8.191 bytes que «funciona» hasta que alguien lo reabre en un `open` honesto. Corrupción silenciosa con aspecto de grafo válido: la bestia de la historia del cap. 11, ahora con patas.

## 12.9 Qué hemos sacrificado

1. **La free list no persiste**: tras un reopen, toda página liberada queda huérfana (parece asignada) hasta una compactación. Decisión documentada en test; su casa natural (la metapágina) ya tiene campo reservado desde el cap. 11.
2. **`free_list.contains()` es O(n)**: cada `read`, `write` y `free` paga un escaneo lineal de la lista. Para miles de páginas da igual; para millones, un bitmap lo haría O(1). Preferimos el `Vec` legible.
3. **El fichero crece página a página**: sin pre-alocación (`fallocate`), cada `allocate` al final del fichero es una escritura de 4 KB de ceros; los motores serios reservan espacio de antemano.
4. **Un solo hilo**: `&mut self` en todo el trait excluye concurrencia por diseño. El wrapper concurrente llega con MVCC (caps. 28-30).
5. **`sync` es un botón global**: `sync_all()` vuelca todo el fichero; no distinguimos `fdatasync` ni sincronizamos por página. Suficiente ahora, tosco para un WAL real.

## 12.10 Cómo lo hace una BBDD real

- **SQLite** es el mapa exacto de este capítulo. En su arquitectura (`sqlite.org/arch.html`), el Pager está justo debajo del B-tree y encima de la capa VFS/OS: el B-tree «pide páginas concretas al pager y le avisa cuando quiere modificarlas, hacer commit o rollback». Su pager (`pager.c`, más `wal.c` y `pcache.c`) añade lo que nosotros dejamos para los caps. 13 y 28: caché, journal y commit atómico. ¿Y la free list? **Persistida**: en la cabecera de 100 bytes de la página 1, el offset 32 guarda el número de la primera *freelist trunk page* y el 36 el total de páginas libres. Las trunk pages forman una lista enlazada cuyas hojas apuntan a las *leaf pages*… que «no contienen información alguna» — SQLite ni las lee ni las escribe al liberar, por la misma razón que nuestro `free` no borra: sobrescribir una página libre es I/O regalada. Incluso hay historia en esa estructura: por compatibilidad con un bug anterior a la versión 3.6.0, las trunk pages modernas dejan las últimas seis entradas sin usar. Los formatos en disco son para siempre.
- **PostgreSQL** separa las mismas responsabilidades con otros nombres: el *storage manager* (`smgr`) es el port, `md.c` («magnetic disk») es el adapter — nuestro `FilePager` con pedigrí — y encima vive el buffer manager de 8 KB por página. El inventario de espacio libre no es una lista: es el **FSM** (free space map), un bitmap por página indexado en árbol.
- **InnoDB (MySQL)** gestiona tablespaces con páginas de 16 KB y listas de extensión libres dentro de su capa FSP — la misma idea (quien posee el inventario de páginas lo posee todo), a escala industrial.

En todos, la lección es idéntica a la de tu `FilePager`: **una única pieza posee la aritmética de páginas y el inventario de libres; las capas de arriba piden y devuelven cajas numeradas**.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `num_pages()` devuelve también las páginas que están en la free list? ¿Qué se rompería en el test `free_y_reutilizacion` si no fuera así?
- *Intermedio*: `read` comprueba buffer, rango y free list en ese orden. Encuentra un escenario donde invertir dos checks cambie el error que ve el caller, y argumenta si importa.
- *Experto*: SQLite persiste su freelist en la cabecera (offsets 32/36). Diseña el protocolo de escritura para persistir nuestra free list en `MetaPage` sin que un crash a mitad entregue una página a dos dueños. ¿Qué garantía te falta del cap. 28?

## 12.11 Lo que te llevas

- El **`PageId` es el offset** (`id × PAGE_SIZE`): acceso O(1) y cero indirección, a cambio de páginas inamovibles.
- Un **port (`trait Pager`) + adapter (`FilePager`)**: el contrato desacopla las capas superiores del medio; los tests del cap. 13 irán a RAM y el mmap del apéndice no tocará a nadie.
- La **free list LIFO en memoria**: reutiliza lo más caliente con `pop()` O(1); su no-persistencia es una deuda documentada que elige *leak* sobre *corrupción*.
- **`write` no es durable**: los bytes viven en la page cache del SO hasta que `sync` (fsync) los baja al disco. N writes + 1 sync.
- **Validar en la frontera**: `open` rechaza ficheros truncados (tamaño no múltiplo), `read/write` validan buffer, rango y propiedad antes de la primera llamada al sistema, y `read_exact` prohíbe medias páginas.
- **Errores tipados** (`PagerError`): el caller distingue su bug (`OutOfRange`, `FreePage`, `BadBufferSize`) del fallo del entorno (`Io`) sin parsear strings.

## 12.12 Ojo, cuidado con…

- **«Ya está guardado»**: sin `sync`, «guardado» significa «en la RAM del kernel». El proceso puede morir sin drama; el corte de luz, no.
- **Confundir el contenido del `Vec` con el orden de salida**: `free_list` guarda en orden de inserción `[1, 2, 3]`; el LIFO lo dicta `pop()` (sale `3` primero). Caímos en eso con el primer test — ver la historia de abajo.
- **`num_pages` cuenta páginas del fichero, no páginas asignadas**: incluye las de la free list, que existen en disco aunque no tengan dueño. Para «¿tiene dueño?» está `is_allocated`.
- **`create()` es destructivo**: trunca un fichero previo sin preguntar (`.truncate(true)`). Es lo que quieres en `liradb init` y una sorpresa en cualquier otro sitio.
- **Liberar no es borrar**: la página sigue en disco con sus datos viejos hasta que alguien la reasigne y sobrescriba. Si guardas secretos, sabes qué ejercicio te espera (el intermedio de abajo, con ceros).

## 12.13 Pin de batalla

> *«Un `PageId` es un puntero desnudo a disco. Repártelo dos veces y corrompes; piérdelo y desperdicias. El pager existe para que ninguna de las dos cosas pase por accidente.»*

## 12.14 Si solo lees 30 segundos

El fichero es un array de páginas: la `i` vive en los bytes `[i·4096, (i+1)·4096)`. El trait `Pager` es el único dueño de esa aritmética y del inventario: `allocate` da páginas (primero reutiliza la free list LIFO, luego extiende), `read`/`write` las mueven con buffers exactos de 4.096 bytes, `free` las devuelve a la lista sin borrar nada, y `sync` baja a disco lo que el SO aún retiene en RAM. `FilePager` lo implementa sobre `std::fs::File`, y los errores tipados dicen si la culpa es tuya o del disco. Todo lo demás —caché, transacciones— se construye encima de este contrato.

## 12.15 Una historia pequeña

El primer test de reutilización múltiple que escribimos para `FilePager` liberaba las páginas 1, 2 y 3 en ese orden y esperaba que `allocate()` las devolviera en el mismo orden: 1, 2, 3. Falló. La free list era `[1, 2, 3]` — el `Vec` había guardado exactamente lo que le dimos — pero `pop()` sacaba por el otro extremo: 3, 2, 1. Nuestra primera reacción fue «bug del pager»; la correcta llegó cinco minutos después: el pager hacía exactamente lo que debía (LIFO), y el que mentía era el test, escrito pensando en una cola. Lo arreglamos y dejamos el comentario en el test para el siguiente lector: *«free_list almacena en orden de inserción; el orden LIFO lo decide el `pop`»*. Moraleja que nos ha servido desde entonces: cuando un test falla, antes de tocar el código pregunta **quién de los dos tiene el modelo equivocado**. A veces el sistema está bien y es tu expectativa la que no sabía leer un `Vec`.

## Ejercicios resueltos

**1. Liberas las páginas 1, 2 y 3 (en ese orden) y luego llamas `allocate()` tres veces. ¿Qué devuelve cada llamada y por qué?**

Devuelve 3, luego 2, luego 1. El `Vec` de la free list contiene `[1, 2, 3]` en orden de inserción, pero `allocate` consume con `pop()`, que extrae por el final. Es LIFO: última página liberada, primera reutilizada — que además es la más «caliente» en la caché del SO. Verifícalo con `cargo test free_y_reutilizacion_multiple`.

**2. Un fichero mide 8.191 bytes y lo abres con `FilePager::open`. ¿Qué pasa y qué harías con esos 8.191 bytes si fueras el encargado de repararlo?**

`open` devuelve `PagerError::Io` con `ErrorKind::InvalidData` y el mensaje `file size 8191 not a multiple of PAGE_SIZE=4096` (el test `open_archivo_con_tamanho_invalido_falla` lo congela). 8.191 = 4.096·2 − 1: dos páginas completas menos un byte, señal de truncamiento. Como encargado, no inventaría la página 2: copiaría el fichero, verificaría las dos páginas completas con el `check` del cap. 16 y reconstruiría lo perdido desde el origen de datos. Redondear «hacia arriba» rellenando ceros fabricaría una página con magic `0x00` que el `decode` del cap. 11 rechazaría igual — solo que ya habrías mutado la evidencia.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el capítulo 11: escribe de memoria los tres campos de información que guarda la `MetaPage` (además de su cabecera) y explica por qué `FilePager::create()` reserva la página 0 antes de que exista ningún dato. Después verifica con un test en la línea de `escribir_metapagina_y_reabrir`: construye la `MetaPage` con tus campos, escríbela, haz `sync`, reabre y decodifica. Si no recuerdas los campos, eso es señal de relectura, no de pista.

**Intermedio (spacing con el cap. 11).** Nuestro `free` deja los datos viejos en la página liberada. Escribe `free_and_zero(&mut self, id)` que, además de apuntar la página en la free list, la rellene de ceros y la escriba. Mide con qué factor cae el rendimiento de un bucle de 1.000 allocate/free (compara con `cargo test -- --nocapture` y un `Instant::now`). Después conecta con lo aprendido: ¿por qué SQLite ni siquiera lee sus freelist leaf pages? ¿Qué gana y qué paga tu versión?

**Experto.** Sustituye la `free_list: Vec<PageId>` por un bitmap (`Vec<u64>` donde el bit i dice si la página i está libre). Mantén la semántica LIFO documentada (o argumenta, en un comentario del código, qué semántica eliges y por qué) y consigue que los 22 tests del módulo pasen sin modificar sus aserciones. Analiza: ¿qué gana `contains` en coste? ¿Qué pasa con `num_pages` cuando el bitmap crece? ¿Cómo afecta a la persistencia futura en la `MetaPage` (pista: un bitmap de 2³² páginas no cabe en una página de 4 KB)?

## Para profundizar

- **Alex Petrov, «Database Internals» (O'Reilly), caps. 2-3** — diseño de páginas, gestión de espacio libre en ficheros y el papel del pager en motores reales.
- **CMU 15-445 (Intro to Database Systems)** — lecciones de buffer pool y disk I/O: el cap. 13 de este libro es su proyecto BusTub en miniatura, y este capítulo es su `DiskManager`.
- **Código y documentación de SQLite**: `sqlite.org/arch.html` (posición del pager), `sqlite.org/fileformat2.html` (la freelist en la cabecera: offsets 32/36, trunk/leaf pages), y `pager.c`/`pager.h` en el código fuente — uno de los ficheros mejor comentados del software libre.
- **Martin Kleppmann, «Designing Data-Intensive Applications», cap. 3** — por qué `write` no es durable: page cache del SO, fsync y los límites del hardware.
- **La historia de SQLite en voz de su autor**: episodio «The Untold Story of SQLite» del pódcast CoRecursive (entrevista a D. Richard Hipp) y su charla en CMU Databaseology (2015).

## Mini-diálogo: en guardia nocturna

> — A ver si lo entiendo: ¿el pager es un `seek` con diploma? `id * 4096` lo hacía yo con una función de siete líneas.

> — Y con esas siete líneas, ¿quién te devolvía la página 7 cuando el índice hash la dejó libre? ¿Quién impedía que el CSR la cogiera a la vez?

> — …nadie. Por eso existía aquel bug que nadie encontraba.

> — Exacto. El valor del pager no está en la aritmética, sino en que la aritmética tiene **un solo dueño**. El `seek` era gratis; el doble uso de una página valía una semana de debug. Y de regalo, el trait: el cap. 13 probará la caché contra un pager en RAM, mil veces por segundo, sin disco. Prueba a hacer eso con tus once líneas.

> — Vale. Pero lo de no persistir la free list me sigue pareciendo trampa.

> — Es la parte más honesta del capítulo: es una deuda **escrita en un test**. Podemos permitirnos perder una página; no podemos permitirnos dársela a dos dueños. Cuando lleguemos al cap. 28 sabremos garantizar el orden de escritura… y entonces la persistiremos sin miedo.

---

*(Próximo capítulo: 13 — El buffer pool. El pager ya sabe leer y escribir páginas… y ahora descubriremos lo caro que es hacerlo por consulta: cachearlas en memoria exige frames, pin counts, bits de suciedad y una política de evicción. Y por fin sabrás para qué servía el port.)*
