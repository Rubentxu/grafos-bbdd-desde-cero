# Capítulo 11 — Páginas, bloques y slotted pages

> *«El disco no lee bytes. El disco lee bloques. Todo lo demás es una mentira piadosa que nos contamos para dormir tranquilos.»*

## 11.0 La anécdota de la esquina

En 1979, un programador de IBM llamado Jim Gray estaba tratando de explicar por qué cierta base de datos era absurdamente lenta a pesar de tener "solo" un millón de registros. El problema no estaba en el CPU —el CPU hacía lo suyo en microsegundos—, sino en algo que nadie quería admitir: **cada vez que el programa pedía un registro, el disco se movía**. No un poco. Se movía *físicamente*: un brazo mecánico se desplazaba hasta la pista correcta, esperaba a que el plato girara hasta el sector correcto, y solo entonces leía.

Ese viaje mecánico tarda unos 5-10 milisegundos. Suena a poco, hasta que haces la cuenta: si lees los registros **de uno en uno, en posiciones dispersas del disco**, un millón de registros te cuesta varios días. Si los lees **agrupados en bloques contiguos**, el mismo millón de registros cabe en horas o minutos, porque el brazo se mueve una vez y arrastra miles de registros de golpe.

De ahí nace la idea más importante del almacenamiento de bases de datos: **la página**. No es una idea glamurosa. No tiene nombre de algoritmo. Pero sin ella, nada de lo que construiremos en LiraDB funcionaría. Este capítulo es el cimiento de todo el motor de almacenamiento.

## 11.1 Objetivo

Al terminar este capítulo sabrás **por qué una base de datos no escribe registros sueltos en un fichero**, y habrás implementado la estructura que lo resuelve: la **slotted page** — una página de tamaño fijo que puede guardar registros de longitud variable sin malgastar espacio ni corromperse.

En concreto, vas a construir tres piezas:

1. `PageHeader` — los 10 bytes de cabecera que identifican cada página.
2. `SlottedPage` — la página con registros de tamaño variable.
3. `MetaPage` — la página 0, que guarda el catálogo del fichero.

## 11.2 Problema

Imagina que ya tienes el encoding del capítulo 9. Sabes convertir un `Node` en un `Vec<u8>`. La pregunta suena tonta: **¿dónde guardas esos bytes en disco?**

La respuesta ingenua es: *"los escribo uno detrás de otro, en orden"*. Y funciona... hasta que deja de funcionar. Veamos por qué, con números:

- El disco (o el SSD, o el sistema operativo) **no lee bytes individuales**. Lee **bloques**: normalmente 512 bytes, 4096 bytes, o múltiplos. Si pides un solo byte, te llega el bloque entero, lo quieras o no.
- Si guardas registros de longitud variable uno tras otro (`[A][BB][CCC][DDDD]...`), para encontrar el registro número 7.482 tienes que **leer desde el principio y sumar longitudes**. Esa operación es O(n) y, peor, obliga a leer del disco fragmentos dispersos.
- Si borras un registro del medio, queda un hueco. ¿Lo rellenas? ¿Con qué? ¿Y si el hueco mide 3 bytes y el nuevo registro 100?

La raíz del problema: **mezclamos dos decisiones que deberían estar separadas** — *dónde* están los datos en disco (que debe ser fijo y predecible) y *cómo se organizan dentro de cada unidad* (que puede ser flexible).

La base de datos real separa ambas cosas. La unidad fija se llama **página**; la organización interna flexible se llama **slotted page**. Vamos a verlas.

## 11.3 Modelo mental

Piensa en un **libro de contabilidad con páginas numeradas**:

- El libro tiene páginas **todas del mismo tamaño** (digamos 4.096 caracteres). No importa lo que pongas dentro: cada página ocupa exactamente lo mismo.
- Para llegar a la página 500, no lees las 499 anteriores: **vas directamente a la posición `500 × 4096`**. Eso es aritmética, no búsqueda.
- Dentro de cada página, el contable anota los asientos **apretados pero con un índice al principio** que dice "el asiento 1 empieza en la línea 12, el 2 en la 40, el 3 en la 55...".

Esa es exactamente nuestra estructura:

```
Página (4.096 bytes, tamaño FIJOO):
┌──────────────────────────────────────────────────────┐
│ PageHeader (10 bytes): magic | tipo | id | n_rec | libre │
├──────────────────────────────────────────────────────┤
│ [4 bytes: len=6][A B C D E F]                        │  ← registro 1
│ [4 bytes: len=3][G H I]                              │  ← registro 2
│ [4 bytes: len=5][J K L M N]                          │  ← registro 3
│ ...                                                  │
│                    (espacio libre)                   │
└──────────────────────────────────────────────────────┘
```

Dos ideas clave se esconden aquí:

1. **Tamaño fijo de página ⇒ acceso directo.** La página N está en el byte `N × 4096`. Para leerla, haces `seek(N * 4096)` y lees 4096 bytes. No dependes de lo que haya dentro de las páginas anteriores.
2. **Longitud variable de registro ⇒ sin desperdicio.** Cada registro guarda su propia longitud como prefijo, así que no hay que rellenar todos los registros hasta un tamaño máximo ficticio.

La tercera idea, que veremos cuando hablemos del `Pager` (capítulo 12), es que **la página es la unidad de caché**: cuando el buffer pool (capítulo 13) decide qué guardar en memoria, guarda páginas enteras, no registros.

### ¿Por qué 4.096 bytes?

`PAGE_SIZE = 4096` no es casualidad. 4.096 es:

- **Múltiplo del tamaño de sector** (512) y del tamaño de página del sistema operativo (también 4 KB en la inmensa mayoría de sistemas). Si tu página no está alineada con la del sistema operativo, cada lectura puede convertirse en *dos* lecturas físicas.
- **Suficientemente grande** para que un `seek` + lectura traiga un buen montón de datos útiles de una vez (amortiza el coste del movimiento del disco).
- **Suficientemente pequeña** para que no malgastes memoria cacheando una página gigante cuando solo querías un registro.

No es un número sagrado: PostgreSQL usa 8 KB por defecto, SQLite 4 KB, y muchos sistemas permiten configurarlo entre 4 KB y 64 KB. El punto no es el número exacto, sino **que sea fijo y conocido por todo el sistema**. En LiraDB elegimos 4.096 y lo declaramos como constante única (`PAGE_SIZE`): si mañana quieres 8 KB, cambias una línea y nada más.

## 11.4 Primera solución

Empecemos por lo más simple que puede funcionar: **una página que guarda registros de longitud fija**.

```rust
// Solución ingenua: registros de longitud FIJA.
// Cada registro ocupa 100 bytes, haya o no datos.
struct FixedPage {
    records: Vec<[u8; 100]>,
}
```

Para leer el registro `i`, calculas `offset = i * 100` y lees 100 bytes. Fácil. Rápido. Predecible.

Los tests pasan. Los registros cortos ("Ana", 3 bytes) se guardan en un cajón de 100 bytes. Funciona, y durante un rato nadie se queja.

## 11.5 Sus límites

Hasta que alguien guarda un `Value` con una lista de 500 elementos, o un `string` de 10.000 caracteres. Y entonces la solución de longitud fija se enfrenta a un muro:

1. **¿Qué tamaño máximo eliges?** Si pones 100, los registros de 10.000 no caben. Si pones 10.000, cada "Ana" de 3 bytes te cuesta 10.000 bytes en disco — un desperdicio del 99,97 %.
2. **El tamaño de tus datos no es constante.** Un `Node` puede tener cero propiedades o cincuenta. Una arista de un grafo de redes sociales no mide lo mismo que la descripción de un paper.
3. **No hay forma de ganar:** cualquier longitud fija es, o demasiado pequeña (no cabe todo), o demasiado grande (desperdicias disco).

La conclusión duele: **los datos de un grafo son de longitud variable por naturaleza**, y una página de longitud fija no sirve. Necesitamos una página que:

- guarde registros de tamaño arbitrario,
- permita leer el registro `i` sin escanear toda la página,
- y detecte si algo se corrompió.

Esa es la **slotted page**.

## 11.6 Solución evolucionada

La idea de la slotted page tiene décadas (aparece en sistemas de los años 70-80 y sigue siendo la base de PostgreSQL, SQLite, InnoDB y casi cualquier motor serio). Es esta:

**Cada registro va precedido de un prefijo de 4 bytes con su longitud.** Para leer los registros, caminas por la página siguiendo las longitudes:

```
leer registro i:
    pos = 10                      # saltamos la cabecera
    repetir i veces:
        len = leer_u32(pos)        # longitud del siguiente registro
        pos += 4 + len             # saltamos prefijo + datos
    # aquí pos apunta al registro i
    return bytes[pos..pos+len]
```

Esto nos da:

- **Longitud variable**: cada registro mide exactamente lo que necesita + 4 bytes de prefijo.
- **Lectura O(i)**: para llegar al registro `i` saltas exactamente `i` registros, leyendo solo sus prefijos (barato). No escaneas *el contenido* de los anteriores.
- **Sin tabla de offsets separada**: el "índice" y los datos conviven en el mismo flujo. (Una alternativa — tabla de offsets al final de la página — existe, pero para nuestro primer motor el prefijo es más simple y basta.)

La cabecera (`PageHeader`) ocupa 10 bytes al principio y guarda lo imprescindible:

| Byte(s) | Campo | Para qué |
|---|---|---|
| 0 | `magic` | Byte de tipo (0xDA datos, 0xFE meta). Sirve de firma: si no es el esperado, la página está corrupta o no es lo que crees. |
| 1 | `page_type` | Redundante con `magic`: si ambos no coinciden, hay corrupción (autochequeo barato). |
| 2-5 | `page_id` (u32) | Qué página es. Te permite detectar "me han dado la página equivocada". |
| 6-7 | `num_records` (u16) | Cuántos registros hay. Para saber cuándo parar de leer. |
| 8-9 | `free_space` (u16) | Bytes libres. Para saber si un registro cabe sin tener que probar. |

Fíjate en un detalle sutil: `magic` y `page_type` guardan **el mismo valor** (0xDA o 0xFE). Es redundancia deliberada: si un solo bit se corrompe en cualquiera de los dos bytes, `decode` detecta el desajuste y devuelve error en lugar de entregarte datos basura. Es un checksum de un byte, gratuito.

Y aquí está la pieza clave — **por qué el prefijo de longitud y no un marcador de fin**:

Una alternativa popular es terminar cada registro con un byte especial (un "end-marker", como el `\0` de C). El problema: ¿qué pasa si tus datos *contienen* ese byte especial? Tienes que escaparlo, y eso envenena todo el pipeline: cada vez que guardas o lees, hay que escapar/desescapar, y un escape mal hecho es una corrupción silenciosa.

Con el **prefijo de longitud**, no hay byte prohibido: el registro puede contener *cualquier* secuencia de bytes, incluido el valor 0x00, porque sabemos su longitud exacta de antemano. Es la misma razón por la que los protocolos de red serios usan length-prefix en vez de delimitadores.

## 11.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap11_slotted_pages.rs`. Vamos a leerlo por partes, porque cada línea tiene un porqué.

### La cabecera

```rust
pub const PAGE_SIZE: usize = 4096;

pub struct PageHeader {
    pub page_id: u32,
    pub page_type: PageType,
    pub num_records: u16,
    pub free_space: u16,
}

pub enum PageType {
    Data = 0xDA,
    Meta = 0xFE,
}
```

`PageType` es un enum con `#[repr(u8)]` y valores explícitos 0xDA/0xFE. Elegimos esos valores por dos razones: son fáciles de reconocer en un volcado hexadecimal, y son suficientemente "raros" como para que sea improbable encontrarlos por casualidad en datos aleatorios. (Elegir valores con todos los bits puestos y no puestos alternados — como 0xDA y 0xFE — es una heurística de defensa: un byte de datos aleatorios rara vez cae exactamente en ellos.)

Fíjate en el orden de los campos de codificación: es **little-endian**, igual que el resto del capítulo 9. Lo declaramos explícitamente con `to_le_bytes()` / `from_le_bytes()`. Nunca uses la representación nativa de memoria (`to_ne_bytes`): el día que abras tu fichero en una máquina big-endian, se corrompería todo en silencio. El formato en disco debe ser **independiente de la máquina**.

### La página con slots

```rust
pub struct SlottedPage {
    pub header: PageHeader,
    pub(crate) records: Vec<Vec<u8>>,
}
```

En memoria, `records` es un simple `Vec<Vec<u8>>`. En disco, se codifica como el flujo con prefijos de longitud que describimos. La operación de insertar es la que más nos interesa:

```rust
pub fn insert(&mut self, record: &[u8]) -> Option<usize> {
    if record.len() > self.free_space() {
        return None;              // no cabe: la página está llena
    }
    let offset = PageHeader::SIZE + self.records.iter().map(|r| r.len()).sum::<usize>();
    self.records.push(record.to_vec());
    self.header.num_records += 1;
    self.header.free_space = self.free_space() as u16;
    Some(offset)
}
```

Tres cosas importantes:

1. **Devolvemos `Option`**: `None` significa "no cabe". No lanzamos error ni lo metemos a la fuerza. El `Pager` del capítulo 12 decidirá qué hacer con ese `None` (normalmente: buscar otra página o asignar una nueva).
2. **El offset se calcula sumando longitudes**: es la posición donde *empezaría* el registro si lo insertáramos. Por eso es O(n) en número de registros — aceptable para una página de 4 KB, que jamás tendrá millones de registros.
3. **`free_space` se recalcula**: lo derivamos, no lo mantenemos "a ojo". Así la cabecera siempre dice la verdad sobre cuánto queda.

### La metapágina

La página 0 es especial: es la **metapágina**. No guarda datos de usuario, guarda el **catálogo del fichero**: cuántas páginas hay, cuántas están libres, y dónde está la raíz de los índices.

```rust
pub struct MetaPage {
    pub header: PageHeader,
    pub num_pages: u32,
    pub free_pages: u32,
    pub root_page: u32,
}
```

¿Por qué separar la metapágina de las páginas de datos? Porque **necesitas saber cómo leer el fichero antes de poder leer el fichero**. Es el problema del huevo y la gallina: para encontrar el catálogo de índices necesitas la metapágina; para encontrar la metapágina... bueno, esa es fácil, **siempre está en la página 0**. Es una convención que resuelve el arranque en frío: cuando LiraDB abre un fichero, lo primero que lee es la página 0, y desde ahí descubre todo lo demás.

## 11.8 Prueba de fuego

La prueba de fuego no es "los tests pasan" — es **"el roundtrip sobrevive a la realidad"**. Codificar una página y decodificarla debe devolver exactamente lo mismo, incluso con registros de distinto tamaño:

```rust
let mut p = SlottedPage::new(7, PageType::Data);
p.insert(b"hello").unwrap();
p.insert(b"world!").unwrap();
p.insert(b"LiraDB").unwrap();   // tres registros de distinta longitud

let enc = p.encode();
let dec = SlottedPage::decode(&enc).unwrap();
assert_eq!(p, dec);             // idénticos, byte a byte
```

Y los casos de fallo son igual de importantes que el camino feliz:

```rust
// Un registro gigante no cabe, y no lo metemos a la fuerza:
let huge = vec![0u8; PAGE_SIZE - PageHeader::SIZE];
assert!(p.insert(&huge).is_some());     // justo cabe
assert!(p.insert(b"extra").is_none());  // ya no cabe: devuelve None

// Una página corrupta (magic no coincide) se DETECTA:
let mut bytes = [0u8; 10];
bytes[0] = 0xAA;   // magic corrupto
bytes[1] = 0xDA;   // page_type correcto
assert!(PageHeader::decode(&bytes).is_err());
```

Estos tres tests (`slotted_page_con_records`, `slotted_page_record_no_cabe`, `page_header_magic_mismatch`) encapsulan las tres promesas de la slotted page: **roundtrip fiel, no desbordar, detectar corrupción**. Si este capítulo se te olvidara, tu base de datos escribiría registros de longitud variable en un fichero plano y — tarde o temprano — leería basura de una posición mal calculada sin que nadie se enterara. Ese es el síntoma: **corrupción silenciosa al borrar o al insertar registros de distinto tamaño**.

## 11.9 Qué hemos sacrificado

Toda estructura tiene un precio. La slotted page no es gratis:

1. **4 bytes de sobrecarga por registro** (el prefijo de longitud). Para registros diminutos, eso puede ser proporcionalmente mucho. Existe una alternativa — la tabla de slots al final de la página — que guarda los offsets en un array compacto, pero la dejamos fuera por simplicidad; es material de "cómo lo hace una BBDD real".
2. **Sin reutilización de huecos**: si borras un registro del medio, ese espacio queda muerto hasta que compactes la página (eso es, justamente, el capítulo 16, `liradb compact`). Por ahora, `SlottedPage` no tiene borrado individual — los registros solo crecen.
3. **Fragmentación interna**: los últimos bytes de una página casi nunca se usan por completo (un registro de 100 bytes no cabe en 90 libres, y esos 90 quedan ahí). Es el precio de la simplicidad; se recupera con compactación.
4. **Sin checksum real**: el magic de 1 byte detecta desajustes groseros, pero no una corrupción de bits *dentro* de un registro. Un CRC real (como el del capítulo 10) en la cabecera sería el siguiente paso; los sistemas serios lo hacen.

## 11.10 Cómo lo hace una BBDD real

La slotted page es, con variantes, **lo que usan casi todas**:

- **PostgreSQL** usa páginas de 8 KB con una cabecera y un **array de punteros al final de la página** (los "item pointers" crecen desde el final hacia atrás, los datos desde el principio hacia delante, y se encuentran en el medio). Eso le permite **borrar y reutilizar espacio** sin compactar: un puntero puede quedar "muerto" y reaprovecharse.
- **SQLite** usa páginas de 4 KB (por defecto) y guarda los registros como **celdas** con un formato de longitud variable muy similar al nuestro, con una cabecera de "cell pointers" que apunta al contenido.
- **InnoDB (MySQL)** usa páginas de 16 KB con una estructura de slots al final, pensada para el árbol B+.

En todos, el patrón es el mismo que has construido: **cabecera + registros de longitud variable + metadatos que permiten navegar sin escanear**. Lo que cambia es el detalle (tabla de slots vs prefijo) y la cantidad de metadatos extra (checksums CRC, `lsn` de transacciones, punteros al árbol).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en nuestra página, ¿por qué `insert` es O(n) en registros? ¿Qué estructura de datos haría O(1) el cálculo del offset?
- *Intermedio*: implementa el borrado de un registro de una `SlottedPage` compactando los siguientes hacia la izquierda. ¿Qué pasa con los offsets que devolviste antes de borrar?
- *Experto*: rediseña `SlottedPage` para usar una **tabla de slots al final** (offsets creciendo desde el final, datos desde el principio). Compara la sobrecarga de espacio con el prefijo de longitud para registros de 5, 50 y 500 bytes.

## 11.11 Lo que te llevas

- La **página** es la unidad de almacenamiento: tamaño fijo, acceso directo por aritmética (`N × PAGE_SIZE`).
- La **slotted page** resuelve el problema de los registros de longitud variable: prefijo de longitud + cabecera con metadatos.
- **Length-prefix > end-marker**: el prefijo permite cualquier byte en los datos y evita la corrupción silenciosa por escapes.
- La **metapágina** (página 0) resuelve el arranque en frío: siempre sabes dónde empezar a leer.
- **Little-endian explícito** en disco: el formato es independiente de la máquina.
- La **redundancia barata** (magic duplicado) es tu primera línea de defensa contra la corrupción.

## 11.12 Ojo, cuidado con…

- **Confundir página con registro**: la página es la unidad de disco; el registro es lo que vive dentro. Casi todos los errores de diseño vienen de mezclarlas.
- **`free_space` desincronizado**: si no lo derivas del contenido real, acabarás mintiendo sobre cuánto queda y aceptando registros que no caben. Derívalo, no lo "lleves en la cabeza".
- **Endianness**: usar `to_ne_bytes` en vez de `to_le_bytes` funciona en tu portátil y corrompe todo en cualquier otra arquitectura. El formato de disco debe ser explícito.
- **`PAGE_SIZE` repartido por el código**: si el 4096 aparece "a pelo" en diez sitios, cambiarlo será una pesadilla. Una sola constante, referenciada en todas partes.

## 11.13 Pin de batalla

> *«Si tu formato en disco depende de la máquina que lo escribió, no tienes una base de datos. Tienes un accidente esperando a ocurrir.»*

## 11.14 Si solo lees 30 segundos

La base de datos no escribe registros sueltos: escribe **páginas de tamaño fijo** (4 KB en LiraDB). Dentro de cada página, los registros de longitud variable van con un **prefijo de longitud** y una **cabecera** que identifica la página y su espacio libre. La **página 0** es la metapágina: el catálogo que te dice cómo leer el resto. Esa es toda la magia del almacenamiento.

## 11.15 Una historia pequeña

Cuando arrancamos LiraDB por primera vez, antes de este capítulo, "guardar en disco" significaba `fs::write("graph.bin", bytes)` y rezar. Funcionó hasta que Ana intentó guardar un grafo con una descripción de 8 KB en un nodo. El fichero se corrompió en silencio: los registros siguientes se leían desde el byte equivocado, y el grafo entero se convirtió en basura con aspecto de grafo válido. Tardamos una tarde en darnos cuenta de que el fallo no estaba en el algoritmo, sino en que **nunca habíamos decidido dónde termina un registro y empieza el siguiente**. La slotted page fue la respuesta: ahora, hasta un byte de corrupción se detecta, porque la longitud de cada registro está escrita delante de él, en tinta indeleble.

## Ejercicios resueltos

**1. ¿Cuántos registros de exactamente 100 bytes caben en una página de 4.096 bytes?**

Cada registro ocupa 4 (prefijo) + 100 (datos) = 104 bytes. La cabecera ocupa 10. Espacio útil = 4.096 − 10 = 4.086 bytes. Número de registros = 4.086 ÷ 104 = 39,28… → caben **39 registros** (39 × 104 = 4.056 ≤ 4.086; el registro 40 necesitaría 4.160 > 4.086 y no cabe). Sobran 4.086 − 4.056 = 30 bytes, que quedan como fragmentación interna. Puedes verificar esto ejecutando un test que inserte registros de 100 bytes y cuente cuántos devuelven `Some`.

**2. ¿Por qué `decode` puede confiar en `num_records` para saber cuántos registros leer?**

Porque `num_records` se escribe en la cabecera en el momento de codificar, junto con el resto de la página. Si la página no está corrupta, `num_records` es exactamente el número de registros codificados. Si está corrupta y el número es demasiado grande, `decode` se sale de los 4.096 bytes y devuelve `Err("record truncated")` — nunca devuelve datos incompletos como si fueran válidos. Es un ejemplo de **validación defensiva**: el formato lleva su propio límite y lo comprueba.

## Ejercicios propuestos

**Esencial.** Modifica `SlottedPage::insert` para que, además del offset, devuelva cuántos bytes de `free_space` quedan tras insertar. Escribe un test que inserte varios registros y compruebe que `free_space` decrece exactamente en `4 + len` cada vez.

**Intermedio.** Implementa `SlottedPage::delete(index)` que borre el registro `i` y compacte los siguientes (moviendo sus bytes hacia la izquierda). ¿Qué ocurre con `free_space`? ¿Por qué los offsets devueltos por `insert` anteriores al borrado dejan de ser válidos?

**Experto.** Diseña una variante con **tabla de slots al final de la página**: los datos crecen desde el principio, la tabla de offsets crece desde el final, y `free_space` es el hueco central. Implementa `insert` y `decode`, y escribe un test de roundtrip. Compara el espacio total ocupado por 10 registros de 10 bytes en tu diseño vs. el diseño de length-prefix.

## Para profundizar

- **"Database Internals" (Alex Petrov)** — capítulos 2 y 3: layouts de página, slotted pages, B-tree pages. Es el mapa completo del territorio que acabamos de pisar.
- **"Designing Data-Intensive Applications" (Martin Kleppmann)** — capítulo 3: por qué el disco es lento y cómo las páginas y los B-tree lo mitigan.
- **CMU 15-445 (Intro to Database Systems)** — lecciones de storage, con el proyecto BusTub donde implementas un buffer pool sobre páginas exactamente como estas.
- **Código fuente de SQLite** (`btreeInt.h`, `pager.c`): la implementación real de slotted pages y del pager, comentada con un detalle exquisito.

## Mini-diálogo: en guardia nocturna

> — O sea, que "página" es solo un array de 4.096 bytes. ¿Y por eso un capítulo entero?
>
> — Porque ese array de 4.096 bytes es la frontera entre "esto funciona en mi portátil" y "esto funciona en un disco de verdad". Todo lo que construyas encima —el buffer pool, el CSR, los índices, el WAL— asume que puede leer y escribir páginas enteras sin sorpresas.
>
> — Pero entonces, ¿no es un poco... aburrido?
>
> — Lo aburrido es lo que no falla. Las estructuras glamurosas fallan estrepitosamente si el cimiento miente. La slotted page no miente: te dice cuánto hay, dónde empieza cada cosa, y si alguien la pisó. Con eso, ya puedes construir de noche sin miedo.

---

*(Próximo capítulo: 12 — El gestor de páginas. Aquí la página existía como idea; ahora veremos quién la escribe, la lee y decide cuándo está libre — el trait `Pager`.)*
