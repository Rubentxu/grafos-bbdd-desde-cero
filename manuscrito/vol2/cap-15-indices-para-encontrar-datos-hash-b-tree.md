# Capítulo 15 — Índices para encontrar datos (hash + B+ tree)

> *«El hash convierte "¿dónde está?" en aritmética. El orden convierte "¿qué viene después?" en lectura secuencial. Ninguna estructura te regala las dos cosas.»*

## 15.0 La anécdota de la esquina

Agosto de 1970, Seattle. Rudolf Bayer y Edward M. McCreight trabajan en los Boeing Scientific Research Labs y tienen un problema de los que definen una época: los índices ordenados en disco se les caían con los datos creciendo. Los árboles binarios de búsqueda, pensados para RAM, se degeneraban en listas; y cada inserción en un fichero ordenado empujaba media banda magnética de un lado a otro. Su respuesta fue un artículo —«Organization and Maintenance of Large Ordered Indices», presentado en el ACM SIGFIDET de 1970 y publicado en *Acta Informatica* en 1972— que describía una estructura donde cada nodo era **del tamaño de una página de disco**, no de un registro: el B-tree. Retrieval, inserción y borrado en tiempo proporcional a log_k(I), siendo I el tamaño del índice y k la capacidad de la página. Cincuenta y seis años después, esa estructura sigue siendo el índice por defecto de PostgreSQL, MySQL, Oracle y SQLite.

Y aquí viene la parte que gusta a todo el mundo: **nadie sabe a ciencia cierta qué significa la B**. Bayer y McCreight nunca lo explicaron en el paper. Douglas Comer, en su encuesta de 1979 «The Ubiquitous B-Tree» —el artículo que popularizó la estructura fuera del círculo académico—, dejó escrito que los autores jamás aclararon la letra, y barajó candidatos: *balanced* (balanceado), *broad*, *bushy*… o simplemente **Boeing**, su patrón. Hay quien apunta a *Bayer*. McCreight zanjó la pregunta años después con una broma que es mejor definición que la mayoría de libros de texto: cuanto más piensas en lo que significa la B, mejor entiendes los B-trees. En este capítulo vas a construir la mitad de esa idea —la mitad ordenada— con las manos, y la otra mitad (la que parte nodos) queda esbozada en el apéndice del capítulo.

La otra protagonista de hoy es más joven y más humilde: la función hash **FNV**. Nació en 1991 de unos comentarios de revisión al comité IEEE POSIX P1003.2 que enviaron Glenn Fowler y Phong Vo; Landon Curt Noll propuso la mejora que la hizo estable, y el nombre Fowler/Noll/Vo lo bautizó un correo electrónico. No es criptográfica ni pretende serlo: son diez líneas que dispersan claves sorprendentemente bien. Diez líneas que vamos a escribir con las manos, porque en un índice persistente el hash no es un detalle de implementación: **es parte del formato en disco**.

## 15.1 Objetivo

En el capítulo 14 persististe la topología del grafo (CSR): dado un nodo, recorrer sus vecinos es seguir offsets. Pero «¿qué nodo tiene `name = "Ada"`?» o «¿qué aristas pesan entre 5 y 9?» siguen siendo un escaneo completo. Este capítulo construye las dos estructuras que acaban con ese escaneo:

1. **`HashIndex`** — igualdad en O(1): FNV-1a propio, buckets que son páginas, desbordamiento encadenado con `next_page`, catálogo con magic «HID1».
2. **`BPlusTree`** — orden y rangos: una hoja ordenada (raíz = hoja) con búsqueda binaria y `range_scan`, catálogo con magic «BPLU», y la limitación single-level documentada para que el apéndice la derribe.

Ambos sobre `BufferPool<Pager>`: los índices son los primeros **clientes de toda la pila** que llevas construyendo desde el capítulo 9 (encode), el 11 (SlottedPage), el 12 (Pager) y el 13 (BufferPool).

## 15.2 Problema

La pregunta crítica de este capítulo, la que el CORPUS del libro clava con chinchetas: **¿cuándo hash index y cuándo B+ tree?** Para responderla hay que mirar las dos consultas que un grafo real recibe todo el día:

```text
Equality query:   ¿qué nodo tiene id_propiedad = 482?      → UNA clave exacta
Range scan:       ¿qué aristas tienen weight ENTRE 5 y 9?  → un INTERVALO en orden
```

Son gemelas y desdichadas: **ninguna estructura de datos responde las dos a la vez con coste mínimo**, y no por falta de ingenio, sino por una razón estructural:

- Para responder igualdad en O(1), el hash **destruye el orden a propósito**. La dispersión es su trabajo: la clave 500 y la 501 caen en buckets sin ninguna relación, porque si los vecinos cayeran cerca, los clusters matarían la distribución. Consecuencia: «dame las claves entre 500 y 600» obliga a escanear **todos** los buckets.
- Para responder rangos, necesitas los datos **ordenados**, y el orden no admite aritmética directa: no puedes calcular «dónde está García» con una multiplicación; tienes que **comparar** para navegar. Por eso el mejor coste de igualdad en una estructura ordenada es O(log n) — búsqueda binaria — y no O(1).

Esa tensión es la decisión de arquitectura del capítulo. Pero antes de decidir, demos números al dolor que queremos curar. Sin índice, una consulta por propiedad lee y decodifica todo: con 100.000 nodos (~40 bytes cada uno, ~100 nodos por página), son **~1.000 páginas de 4 KB (~4 MB) leídas y 100.000 registros decodificados y comparados por consulta**. Con un buffer pool de 64 frames, casi todas las lecturas van a disco: a ~7 ms por seek en un disco mecánico (la aritmética del capítulo 11), eso son ~7 segundos **por consulta**; en un SSD, decenas de milisegundos que se multiplican por cada usuario y cada query. Con índice, la parte de búsqueda cuesta **1 lectura de página** (el bucket o la raíz; alguna más si hay overflow), más 1 para el dato real. De 1.000 lecturas a 2-4: esa es la magnitud del capítulo.

## 15.3 Modelo mental

Dos edificios, dos filosofías.

**El `HashIndex` es un parking por matrícula hasheada.** El parking tiene 16 secciones (nuestros `DEFAULT_BUCKETS`). Al entrar, un cartel dice: «calcula `hash(matrícula) % 16` y aparca en esa sección». No buscas plaza: la sección **se calcula**. Si tu sección se llena, te manda a la sección ampliada de al lado con un puntero (el `next_page`). Ahora pregúntale al parking por «todas las matrículas entre 1234AAA y 1234ZZZ»: imposible. Los coches con matrículas vecinas están repartidos al azar por las 16 secciones. El parking responde una sola pregunta, y la responde volando.

**El `BPlusTree` es una agenda ordenada por apellido.** 203 nombres por página, alfabéticamente. Para encontrar «García» abres por la mitad, miras dónde caes («López» → quedó a la izquierda), descartas la mitad restante y repites: 8 aperturas como mucho para 203 nombres (2⁸ = 256 > 203). Y para listar a todos entre «Fernández» y «López» no abres por la mitad: pones el dedo en el primero y **deslizas**. La agenda responde las dos preguntas… pero ninguna con aritmética pura.

Así viven en disco:

```text
Fichero de un HashIndex (B = num_buckets):
┌────┬────┬──────────┬─────────┬─────────┬─────┬─────────┐
│ p0 │ p1 │ p2       │ p3      │ p4      │ ... │ p(2+B)  │
│meta│rsv │catálogo  │ bucket0 │ bucket1 │     │bucketB-1│──→ overflow
└────┴────┴──────────┴─────────┴─────────┴─────┴─────────┘   (colgado por next_page)

Página de bucket (4.096 B):                Raíz-hoja del B+Tree (4.096 B):
[PageHeader 10 B]                         [PageHeader 10 B]
[len=4 ][next_page: u32]   ← record 0     [len=16][magic "BPLU" | key_count]  ← record 0
[len=16][key u64 | value]  ← record 1     [len=16][key=5  | value]           ← record 1..N
[len=16][key u64 | value]  ← record 2     [len=16][key=10 | value]             ORDENADOS
[ ...hasta ~203 entradas ]                [ ...hasta 203 entradas... ]
```

El momento ¡ajá! es doble. Primero: el hash **no es un detalle, es formato en disco** — «clave 42 → página 10» debe ser verdad ayer, hoy y con otro proceso abierto, o el índice pierde datos en silencio. Segundo: la raíz del B+ es exactamente **la hoja de cualquier B+ tree real** — todo lo que aprendas aquí (orden, búsqueda binaria, recorrido) se transplantará sin cambios al árbol multinivel del apéndice.

## 15.4 Primera solución

La solución que ya tienes (y la que usa medio software del mundo sin decirlo): **escanear**. `WHERE p.name = "Ada"` lee las páginas de nodos una a una, decodifica cada registro (cap. 9) y compara. Funciona. Es correcta. Y el cap. 14 la usa para construir el CSR sin quejarse.

La mejora tentadora que escribe todo el mundo: cargar un `HashMap<u64, u64>` en RAM al arrancar, mapeando `id_propiedad → id_nodo`. Cinco líneas, velocidad absurda, cero páginas. Los tests pasan. Durante una semana, todo el mundo está contento.

## 15.5 Sus límites

Hasta que miras al `HashMap` con la honestidad de quien acaba de construir la pila de almacenamiento completa (caps. 9-13):

1. **Muere con el proceso.** Nada de eso está en disco. Cada arranque paga el escaneo completo otra vez: has optimizado la consulta y desoptimizado el arranque.
2. **No es persistible tal cual, y el intento corrompe.** `std::collections::hash_map::RandomState` siembra cada instancia con una semilla aleatoria (defensa contra HashDoS, y buena idea para lo suyo). Si hoy la clave 42 hashea al bucket 7 y ayer a la 3, no puedes haber escrito las entradas del bucket 7 en una página fija: **el índice en disco no puede depender del hasher del proceso**. Y aunque fijaras la semilla, `std` no promete estabilidad del output entre versiones de Rust: tu fichero dejaría de leerse con el primer `rustup`.
3. **No hay orden.** Ni rangos, ni mínimos, ni iterarlo en orden de clave sin copiar y ordenar.
4. **La memoria es el límite, no el disco.** Un índice que solo vive en RAM es un índice que se reconstruye a la velocidad de tu peor día.

El escaneo, por su parte, ya vimos su factura: ~1.000 lecturas de página y 100.000 comparaciones **por consulta**, creciendo linealmente con el grafo.

## 15.6 Solución evolucionada

La solución son dos estructuras, y cada decisión de diseño tiene alternativa descartada. Las importantes:

**FNV-1a escrito a mano, no `std`, no CRC.** Diez líneas: un offset basis, un primo, xor y multiplicación envolvente. Lo exigimos **puro**: constantes publicadas, cero semillas, cero estado del proceso. Así `clave → bucket` es una función matemática: mismas claves → mismo bucket, para siempre, en cualquier proceso y cualquier versión de Rust. ¿Y por qué no CRC, que también es determinista? Porque el CRC está diseñado para **detectar bits** (distancia de Hamming), no para dispersar: es lineal sobre GF(2) y claves consecutivas producen hashes aritméticamente relacionadas — justo el clustering que mata a una tabla con 16 buckets. La prueba del contrato son los vectores oficiales: `fnv1a_64(b"")` = offset basis, `"a"` = `0xAF63DC4C8601EC8C`, `"foobar"` = `0x85944171F73967E8`.

**Buckets + overflow encadenado, no open addressing.** Esta es la decisión donde la RAM y el disco disagree. En memoria, el probing (sondar la siguiente posición libre) es barato: una caché line. En disco, cada sonda es una **página entera leída en un offset aleatorio**: seek tras seek, la bestia del capítulo 11. El encadenado convierte la colisión en «sigue este puntero y lee la página siguiente»: una lectura, y dentro de ella caben ~203 entradas, así que la cadena es corta de nacimiento. Además, el probing degrada cerca de lleno y crecer exige rehashearlo todo; la cadena crece **una página cada 203 colisiones**, sin reconstruir nada.

**El B+ tree de un solo nivel, deliberadamente.** Nuestra raíz ES la hoja: pares ordenados en una página, búsqueda binaria para `get`, filtro ordenado para `range_scan`. ¿Por qué no splits ya? Porque splits + promoción de separadores + rebalanceo son ~300 líneas de complejidad que enterrarían la idea nueva del capítulo — el orden — bajo mecánica de mantenimiento. Lo que pierdes es medible y honesto: un tope de 203 entradas y un insert O(n) (desplazar registros en el `Vec` + reescribir la página entera). ¿Cuánto duele de verdad? Poco, en un dataset pedagógico: desplazar ~2 KB en RAM son microsegundos y la reescritura es una página de 4 KB. El dolor real no es la velocidad: es el **tope**. Cuando el dataset lo supere, el apéndice del capítulo (y el refuerzo ADR-005) tienen el paso a multinivel esperándote.

**Catálogos con magic: «HID1» y «BPLU».** La página 2 de cada fichero de índice guarda su catálogo (`num_buckets`, `key_count` para el hash; `key_count` para el B+) precedido de un magic de 4 bytes que deletrea algo legible en un hexdump: `0x4849_4431` («HID1») y `0x4250_4C55` («BPLU»). Misma filosofía que el `0xDA`/`0xFE` del cap. 11: si esos bytes no encajan, `open` devuelve `IndexError::Inconsistent("bad magic")` en vez de interpretar basura como índice. Coste: 4 bytes. Beneficio: corrupción detectada en la frontera.

**Un fichero por índice.** Ambos reclaman la página 2 (catálogo el hash, raíz el B+), así que en este capítulo **no coexisten en el mismo `BufferPool`**: cada índice vive en su fichero, y el test `hash_and_bplus_coexist` documenta la colisión con un comentario. La solución real —un catálogo global de índices en la página 1, que ya reservamos «para uso futuro»— es deuda apuntada al Apéndice D; desplazar la raíz del B+ a `3+B` habría acoplado ambos formatos para siempre.

**Errores tipados (`IndexError`).** `Io` (el entorno falló), `Inconsistent` (invariante rota tras reopen), `PageNotAllocated` (el catálogo apunta a una página fantasma), `InvalidParam` (raíz B+ llena, `num_buckets == 0`). El caller distingue «necesitas rebuild» de «disco roto» sin parsear strings.

Y con todo eso sobre la mesa, la respuesta operativa a la pregunta del principio, en forma de tabla de decisión:

| Tu consulta pregunta… | Índice | Coste | Qué sacrificas |
|---|---|---|---|
| «¿Dónde está la clave K?» (igualdad, masiva) | `HashIndex` | O(1) + cadena corta | Todo rango, orden, min/max, iteración |
| «¿Qué hay entre A y B?» / «el siguiente» / «ordena por» | `BPlusTree` | O(log n) + lectura secuencial del rango | La O(1) de la igualdad pura |
| Ambas, sobre la misma propiedad | Los dos (ficheros separados) | Lo mejor de cada uno | Espacio: indexas dos veces |
| Topología: «los vecinos de u» | Ninguno: el CSR del cap. 14 | O(grado) | Ya está resuelto — no lo re-indices |

## 15.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap15_indices.rs` (~650 líneas + 27 tests). Lo leemos por partes; todos los fragmentos salen de ese módulo.

### El hash determinista

```rust
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01B3);
    }
    h
}
```

Eso es todo: offset basis, xor del byte, multiplicación envolvente por el primo. Fíjate en `wrapping_mul`: en Rust el overflow de enteros en modo debug **asusta** (panic), y el corazón de cualquier hash es justo desbordarse sin culpa. El bucket se elige en `bucket_index`: `fnv1a_64(&key.to_le_bytes()) % num_buckets` — la clave se hashea **en su representación de disco** (8 bytes little-endian, cap. 9). Cambiar esa representación a big-endian reubicaría cada clave del índice: otra vez, el formato en disco manda.

### El catálogo, la entrada y la cabecera de bucket

`HashIndexHeader` (16 B LE: magic, `num_buckets`, `key_count`, reserved) se persiste como **único record** de la página 2. `HashEntry` son 16 B fijos (`key: u64`, `value: u64`) — anchura fija significa aritmética de capacidad exacta. Y la pieza que da personalidad al diseño: `BucketHeader` (4 B: `next_page: u32`) viaja como **primer record** de cada página de bucket. Si `next_page == 0`, la cadena termina. Es el patrón «primer record = header lógico»: no hay offsets mágicos dentro de la página; el orden de records y su tamaño discriminan lo que son.

### `create` y `open`: el layout

`create(pool, num_buckets)` rechaza `num_buckets == 0` con `InvalidParam`, extiende el pager hasta `3 + num_buckets` páginas, escribe el catálogo en la 2 y un `BucketHeader{next_page: 0}` en cada una de las páginas `3..3+B-1`, y flushea. Con los 16 buckets por defecto, tu fichero ya ocupa 19 páginas: **76 KB** aunque el índice esté vacío — el precio de comprar parking por secciones. `open(pool)` hace el camino inverso con la paranoia de siempre: ¿existe la página 2? (`PageNotAllocated` si no), ¿el payload mide 16? ¿el magic es «HID1»? ¿`num_buckets > 0`? ¿Existen todas las páginas de bucket? Cada respuesta negativa es un error distinto y tipado.

### `insert`: recorrer la cadena, reemplazar o colgar

Antes de la lógica, observa el **cómo** se lee cada página — es la disciplina pin/unpin del cap. 13 hecha carne:

```rust
let buf = self.pool.get_page(page_id)?;      // frame pineado
let bytes: [u8; PAGE_SIZE] = *buf;           // copia: suelta el borrow…
self.pool.unpin(page_id, false)?;            // …y despina YA, sin dirty
let sp = SlottedPage::decode(&bytes)...      // decodifica fuera del pool
```

Copia los bytes, despina inmediatamente, y trabaja sobre tu copia. Ningún índice retiene un frame mientras piensa: `PoolFullOfPinned` es la consecuencia de vivir del pool sin respetarlo, y el cap. 13 te lo enseñó a la manera difícil.

La lógica (l. 457-599) es un paseo por la cadena: en cada página, decodificar la `SlottedPage`, saltar el record 0 (`BucketHeader`), buscar la clave entre los `HashEntry`. Si aparece: reemplazo in-place de ese record y a casa. Si no aparece y hay `next_page`: siguiente página. Si se acaba la cadena: la entrada se añade a la última página visitada si cabe (el chequeo es aritmética del cap. 11: `used + need <= PAGE_SIZE`, con `need = 4 + 16`), y si no cabe, `allocate()` una página nueva, escribir en ella el `BucketHeader` + la entrada, y **recablear** el `next_page` de la última página:

```rust
// No cabe: aloca nueva página y cuélgala del header de la última.
let new_page = self.pool.pager_mut().allocate()?;
// (nueva página: BucketHeader{next_page: 0} + la entrada; write_record_page)
let mut bh_old = BucketHeader::decode(&records_mut[0])?;
bh_old.next_page = new_page;                 // recablear la cadena
records_mut[0] = bh_old.encode().to_vec();   // …y reescribir la página llena
```

El catálogo (`key_count`) se actualiza y flushea al final de cada inserción — tosco (una escritura de catálogo por insert) pero seguro; el cap. 28 (WAL) traerá la forma adulta de agrupar.

### El `BPlusTree`: orden en una página

El struct mantiene las entradas **cacheadas en memoria** (`entries: Vec<TreeEntry>`) y las reescribe enteras en cada `persist`. Las dos operaciones que lo definen:

```rust
pub fn get(&self, key: u64) -> Option<u64> {
    match self.entries.binary_search_by_key(&key, |e| e.key) {
        Ok(idx) => Some(self.entries[idx].value),
        Err(_) => None,
    }
}
```

Búsqueda binaria del estándar sobre el invariante «ordenado por clave»: O(log n), unas 8 comparaciones para 203 entradas. Y el insert (l. 874-896) reutiliza la misma búsqueda: `Ok(idx)` → reemplazo in-place (y devuelve `false`: no añadió); `Err(idx)` → el punto de inserción ya está calculado, se comprueba la capacidad y se hace `entries.insert(idx, …)` — que desplaza a la derecha todo lo posterior. Ahí está el O(n) del single-level, visible y sin disimulo. La capacidad se **deriva**, no se estima a ojo:

```rust
let usable = PAGE_SIZE - PageHeader::SIZE - 4 - BPlusHeader::SIZE;
usable / (4 + TreeEntry::SIZE)      // (4.096 - 10 - 4 - 16) / 20 = 203
```

Y `range_scan(lo, hi)` es un filtro inclusivo en ambos extremos sobre las entradas ordenadas — con `lo > hi` devuelve vacío.

### `create` con bucle, `persist` con header de record

Dos detalles del código que son lecciones en sí mismos. Primero, `BPlusTree::create` asegura la página raíz así:

```rust
while !pool.pager().is_allocated(root_page) {
    pool.pager_mut().allocate()?;
}
```

¿Por qué un `while` y no un `allocate()` solitario? Porque la primera versión hacía exactamente eso y fallaba: un pager recién creado tiene solo la página 0 (la metapágina, cap. 12), así que un único `allocate()` entrega la página **1** — la reservada — y la 2 sigue sin existir. `allocate` es secuencial: reparte 1, luego 2. El `while` hasta `is_allocated` es un contrato («esta página existe») en vez de una suposición («habrá bastado una»). Segundo, `persist` escribe el `BPlusHeader` como **primer record** y cada `TreeEntry` como record independiente — lo que permite a `open` distinguirlos por tamaño y validar que `key_count` coincide con el número real de records. Que ese diseño es load-bearing lo aprendimos por la vía dura: la historia de la sección 15.15.

## 15.8 Prueba de fuego

La batería de `tests_index` (27 tests) verifica las promesas una a una. Las que debes poder citar de memoria:

```rust
// El contrato del hash: vectores oficiales FNV (cualquier desviación = bug).
assert_eq!(fnv1a_64(b""), 0xCBF2_9CE4_8422_2325);
assert_eq!(fnv1a_64(b"a"), 0xAF63_DC4C_8601_EC8C);
```

```rust
// Roundtrip REAL en disco: create → insert → flush → cerrar → reabrir → get.
let mut h2 = HashIndex::open(pool2).unwrap();
assert_eq!(h2.bucket_count(), 8);
for &k in &keys { assert_eq!(h2.get(k).unwrap(), Some(k * 3)); }
assert_eq!(h2.get(999_999).unwrap(), None);      // las que no están, no están
```

```rust
// El rango: inclusivo en ambos extremos, vacío si lo > hi, completo si abarca.
let r = t.range_scan(4, 12);
let got: Vec<u64> = r.iter().map(|e| e.key).collect();
assert_eq!(got, vec![5, 7, 9, 11]);
```

```rust
// Corrupción detectada: el magic del B+ vive en el offset 14 de la página.
buf[14..18].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
// ... y open() responde Err(IndexError::Inconsistent("bplus root: bad magic"))
```

Una curiosidad honesta sobre los nombres: `hash_insert_many_triggers_overflow_chain` inserta 200 claves en 2 buckets — 100 por bucket — y **no llega a desbordar** (caben 203 por página): lo que el test realmente certifica es que la cadena, corta, responde todas las claves. El punto exacto donde salta la primera página de overflow lo encontrarás tú, con lápiz, en el ejercicio esencial. Completa la batería con `hash_insert_replaces_existing` (upsert sin inflar `key_count`), `hash_open_without_catalog_fails` y `bplus_open_without_root_fails` (ambos `PageNotAllocated(2)`), y `hash_and_bplus_coexist` (la no-coexistencia, documentada).

¿Y si te saltas este capítulo? Los síntomas llegan con factura: cada filtro por propiedad escanea el grafo entero (latencia que crece linealmente con tus datos), y si improvisas un índice con un hasher sembrado, te espera algo peor — un `get` que devuelve `None` para claves que están en disco, con pinta de bug de la aplicación y olor a corrupción silenciosa.

## 15.9 Qué hemos sacrificado

1. **Índices estáticos**: «actualizar» es rebuild (`create` + `insert*` + `flush`). Los índices dinámicos en línea son el cap. 28; y el rebuild deja páginas huérfanas que el cap. 16 aprenderá a ver y recoger.
2. **Sin rehash**: si las cadenas crecen, no hay linear hashing ni split de buckets — la respuesta honesta hoy es rebuild con más buckets. ¿Cuándo es urgente? Cuando el factor de carga (claves/buckets) supera ~1: las cadenas empiezan a costar 2-3 páginas por `get`.
3. **B+ single-level**: tope de 203 entradas, insert O(n), sin deletes. El apéndice del capítulo es el mapa de la salida.
4. **Catálogo re-escrito en cada insert** del hash (y raíz entera en cada insert del B+): durabilidad por bravery, no por diseño; el WAL del cap. 28 lo convertirá en lotes.
5. **Un fichero por índice**: la página 2 tiene dos dueños posibles y solo uno vive en cada fichero. El catálogo global (página 1) queda como deuda declarada.
6. **Claves y valores de 8 bytes fijos**: sin claves de longitud variable ni compresión (vecinos que comparten prefijos, como hacen los B+ reales — cap. 38 para compresión).

## 15.10 Cómo lo hace una BBDD real + retos

- **PostgreSQL** es la lección completa de «cuándo cada uno». Su índice por defecto es un **B-tree** (variante B-link de Lehman-Yao, con páginas de 8 KB): responde igualdad *y* rangos *y* orden, y desde la v13 comprime claves duplicadas (deduplicación con *posting lists*), exprimiendo que las hojas están ordenadas. ¿Tiene hash indexes? Sí — pero durante años **no se logueaban en WAL**: no sobrevivían a un crash ni se replicaban, y la documentación los trataba como ciudadanos de segunda. Desde la v10 sí son crash-safe… y siguen sin ser el default, porque un índice que solo hace igualdad compite contra uno que hace igualdad + rangos en el mismo espacio. Ese es el veredicto del mercado que este capítulo ha construido a mano.
- **InnoDB (MySQL)** apuesta todo al B+ con páginas de 16 KB: la tabla entera ES un índice clustered por clave primaria (las filas viven en las hojas), y los índices secundarios guardan la PK como valor — dos índices, dos B+ trees, un solo orden físico. Fan-outs de cientos de punteros por nodo: árboles de 3-4 niveles para millones de filas.
- **Neo4j** indexa propiedades con **B+ trees nativos** (los *schema indexes* de etiqueta/propiedad), recorre etiquetas con *label scan stores*, y delega el texto completo a Lucene. La igualdad exacta sobre propiedades — nuestra `equality query` — cae en esos B+ o en los *native token lookup*; el hash puro es menos protagonista en grafos porque las consultas de propiedades quieren también rangos (`age > 30`) y orden.
- **Kùzu/Ladybug**, nuestra referencia de la Parte III: los hash *joins* materializan tablas hash exactamente con la estructura de buckets + overflow que acabas de escribir — la diferencia es que la suya vive en memoria de trabajo y la tuya sobrevive al proceso.
- **SQLite**: un B+ tree por tabla y por índice, páginas de 4 KB, interiores y hojas — la versión completa de lo que el apéndice esboza.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: con `num_buckets = 16`, ¿cuántas claves (bien distribuidas) caben antes del primer overflow? ¿Y cuál es la huella mínima en disco del índice vacío? Justifica ambas con aritmética de páginas, no con estimaciones.
- *Intermedio*: el catálogo del hash se re-escribe y flushea en cada `insert`. Diseña un `insert_batch(keys: &[(u64, u64)])` que agrupe: ¿qué garantía pierdes si el proceso muere a mitad del lote? ¿Leak o corrupción? (Recuerda la regla del cap. 12: cuando no puedas garantizar atomicidad, prefiere leak.)
- *Experto*: el puente al apéndice — da el primer paso a multinivel: cuando la raíz-hoja se llene, reparte sus entradas en dos hojas, promueve el separador y convierte la página 2 en nodo interno. Pista de diseño: la raíz se queda en la página 2; solo cambia su contenido.

## 15.11 Lo que te llevas

- **Igualdad quiere dispersión; rango quiere orden. Ninguna estructura da ambas gratis** — el hash destruye el orden a propósito, y el orden prohíbe la aritmética directa. Elegir índice es elegir qué pregunta quieres que sea barata.
- **El hasher de un índice persistente es formato en disco**: FNV-1a puro (constantes fijas, vectores verificables), semillas aleatorias o hashers versionados = pérdida silenciosa tras reopen.
- **En disco, encadena; no sonde**: la unidad de I/O es la página — cada sonda de open addressing es un seek; la cadena de overflow es una lectura con ~203 entradas dentro.
- **La raíz-hoja ordenada es la mitad de un B+ tree**: búsqueda binaria O(log n) y range scan salen del mismo invariante. La otra mitad (splits) tiene su mapa en el apéndice.
- **«HID1» y «BPLU»**: 4 bytes que convierten «leer basura con confianza» en `IndexError::Inconsistent`.
- **Los índices son clientes de toda la pila**: caps. 9 (LE de la clave), 11 (records y prefijos), 12 (allocate/is_allocated), 13 (pin/unpin/flush). Ninguno abre el fichero directamente.

## 15.12 Ojo, cuidado con…

- **El magic en el hexdump sale invertido**: guardamos little-endian, así que «HID1» aparece como `31 44 49 48` («1DIH») y «BPLU» como `55 4C 50 42`. La primera vez confunde a todo el mundo.
- **`HashIndex::insert` no devuelve el valor anterior**: el doc-comment lo promete, pero el código devuelve el valor **nuevo** como marca de «estaba» (y un comentario lo confiesa: «no se usa realmente»). El B+ sí tiene el contrato limpio (`true` = añadió, `false` = reemplazó). Arreglar el del hash es un buen primer commit.
- **Cambiar la representación de la clave rompe el índice**: el hash opera sobre `key.to_le_bytes()`. Un cambio a big-endian reubica todas las claves — es la lección del cap. 9 reaparecida dentro del hasher.
- **Más buckets no es gratis**: cada bucket es una página de 4 KB aunque nunca reciba una clave. 16 buckets = 76 KB mínimos; 1.024 buckets = 4 MB. El `num_buckets` es una apuesta sobre el tamaño futuro del dataset.
- **Confundir factor de carga con capacidad de página**: la capacidad (203 entradas/página) es aritmética fija; el factor de carga (claves/buckets) es la decisión que decide cuánto miden las cadenas.

## 15.13 Pin de batalla

> *«Un índice no hace tus datos más rápidos: decide qué datos NO necesitas mirar. El hash compra preguntas exactas vendiendo el orden; el B+ compra el orden vendiendo la aritmética. Antes de indexar, decide qué factura quieres pagar.»*

## 15.14 Si solo lees 30 segundos

Sin índice, cada consulta por propiedad escanea el grafo (~1.000 páginas y 100.000 comparaciones para 100k nodos). El **`HashIndex`** la convierte en aritmética: FNV-1a determinista elige un bucket (una página, con hasta 203 entradas de 16 B), y el desbordamiento se encadena con `next_page` — en disco se encadena porque cada sonda alternativa sería un seek. El **`BPlusTree`** mantiene las entradas **ordenadas** en una hoja-raíz: búsqueda binaria para la igualdad y `range_scan` para los intervalos, con un tope honesto de 203 entradas hasta el multinivel del apéndice. Ambos persisten un catálogo con magic («HID1»/«BPLU») en la página 2, validan al `open` y devuelven `IndexError` tipados. Igualdad → hash; orden y rangos → B+. Ese es el capítulo.

## 15.15 Una historia pequeña

La primera versión de `BPlusTree::persist` hacía lo más natural del mundo: concatenar header y entradas en un único blob y escribirlo como un solo record de la `SlottedPage`. Compilaba, los tests en memoria pasaban — el árbol guardaba sus entradas y las devolvía. El drama llegó con el test de persistencia: cerrar el fichero, reabrir, y el árbol aparecía **vacío**. `key_count` decía cero, `range_scan` no devolvía nada… pero el fichero en disco seguía creciendo con cada insert, así que los datos *estaban* en alguna parte. La autopsia fue humillante en su simplicidad: `open` leía el primer record, veía 16 bytes, los interpretaba como el `BPlusHeader`, e ignoraba educationadamente todo lo demás — las entradas quedaban en disco como un apéndice ilegible. La lección se quedó grabada como patrón del módulo: **el formato debe poder distinguir sus piezas por sí mismo** — header como primer record, cada entrada como record independiente, tamaños que discriminan — porque un formato que solo entiende su escritor no es un formato: es una memoria privada con suerte. Y de propina, una regla que ya sabías del cap. 12: los tests en memoria no validan persistencia; solo el disco reabierto dice la verdad.

## Apéndice del capítulo — El paso a multinivel (a dónde va el B+ tree)

La limitación single-level está declarada en el propio módulo: la raíz se llena a las 203 entradas y `insert` responde `IndexError::InvalidParam("bplus root full (single-level cap; rebuild required)")`. Este apéndice no la implementa: dibuja el camino, para que el reto experto y el refuerzo del ADR-005 tengan mapa. El truco en una frase: **la raíz no se muda; cambia de oficio**.

1. **Split de hoja**: cuando la hoja se llena, se aloca una segunda, se reparten las 203 entradas ordenadas por la mediana (≈102 + 101), y la clave que queda en la frontera se **promociona** como separador.
2. **La raíz se convierte en nodo interno**: la página 2 deja de guardar entradas y pasa a guardar pares *(separador, página_hoja)*: «claves < 507 → hoja A; claves ≥ 507 → hoja B». Fíjate en lo que NO cambia: la dirección de la raíz. Por eso elegimos raíz fija — el multinivel crece **hacia abajo** sin reescribir punteros externos.
3. **Las hojas se enlazan** con `next_page` — exactamente el patrón que el `BucketHeader` del hash ya usa — para que el `range_scan` multi-hoja sea deslizar el dedo, no volver a subir al nodo padre por cada hoja.
4. **Cascada**: si un nodo interno se llena, se parte igual y promociona otro separador; la altura solo crece cuando TODO el nivel está lleno — por eso un B+ de páginas de 4 KB con fan-out ~200 alcanza millones de claves en 3 niveles: 200³ = 8 millones.
5. **Lo que la ordenación regala**: claves vecinas comparten prefijos (y a menudo son duplicados) — ahí viven la deduplicación de PostgreSQL 13 y la compresión de prefijos de los B+ comerciales.

Coste estimado del salto, según el propio módulo: ~300 líneas. Complejidad nueva: splits, promoción, rebalanceo… y después, concurrencia sobre nodos que se parten (el *latch crabbing* de Lehman-Yao que usa PostgreSQL). Todo eso ya es otro capítulo — este solo quería que supieras exactamente qué hay detrás de la puerta.

## Ejercicios resueltos

**1. ¿En qué inserción exacta aparece la primera página de overflow de un `HashIndex` con un solo bucket, y cuántas entradas caben en la raíz de un `BPlusTree`?**

Aritmética de páginas, cap. 11 en la mano. Bucket: `PageHeader` 10 + record 0 (prefijo 4 + `BucketHeader` 4) = 18 B de overhead; cada entrada cuela 4 + 16 = 20 B. La página admite n entradas mientras 18 + 20n ≤ 4.096 → n ≤ 203,9 → **203 entradas**; la 204.ª encuentra `used + need = 18 + 4.060 + 20 > 4.096` y dispara `allocate()`: el fichero pasa de 4 páginas (0,1,2,bucket) a 5. B+: 10 + (4+16) del header + 20 por entrada → (4.096 − 30)/20 = 203,3 → también **203**. Los dos índices comparten tope por casualidad de overheads similares — pero no por la misma razón: el bucket gasta 8 B en su cabecera de cadena; la raíz B+, 20 en su catálogo con magic. Verificación: un test con `num_buckets = 1` observando `pool().pager().num_pages()`.

**2. En `bplus_disk_bad_magic_fails` se corrompen los bytes 14..18 de la página 2. ¿Por qué ahí y no en 0..4? ¿Qué testearía corromper 0..4?**

Porque el layout de la página es `[PageHeader: 10 B][prefijo de longitud: 4 B][BPlusHeader: 16 B]` — el magic del B+ empieza en el byte 10 + 4 = **14**. Corromper 0..4 tocaría el magic del `PageHeader` del cap. 11: ruta de error distinta (`SlottedPage::decode` fallaría antes de llegar al B+), y el test demostraría otra detección, no la del catálogo B+. Es la lección del «detectar el magic exige saber dónde vive» — la versión cap. 15 de la documentación de offsets del cap. 11. El comentario del test, que deja el cálculo escrito, previene la próxima hora perdida.

## Ejercicios propuestos

**Esencial (retrieval + spacing).** Sin mirar los caps. 11 ni 15: predice, con lápiz, en qué inserción exacta salta la primera página de overflow de un `HashIndex` creado con `num_buckets = 1`. Después verifica con un test que inserte claves y observe `pool().pager().num_pages()` — el fichero crece de 4 a 5 páginas en un punto muy concreto. Si tu predicción no sale de la aritmética de records (cuánto cuela cada uno dentro de una `SlottedPage`), eso es señal de relectura, no de pista.

**Intermedio (interleaving: hash + orden).** Inserta las claves 0..1.000 con valores `k*7` en un `HashIndex` y en un `BPlusTree` (pagers separados — ya sabes por qué). Escribe un test que: (a) ejecute `range_scan(137, 731)` sobre el B+ y verifique con `get` del hash cada clave devuelta y su valor; (b) compruebe que 136 y 732 existen en ambos índices pero no aparecen en el rango; (c) responda por escrito: ¿por qué es imposible escribir este test usando un solo índice? ¿Qué tendría que hacer el hash para responder el rango, y cuánto le costaría?

**Experto (el puente al multinivel).** Implementa el primer split según el apéndice: cuando `insert` detecta la raíz llena (203 entradas), reparte por la mediana en dos hojas nuevas, promueve el separador y reescribe la página 2 como nodo interno *(separador, página)*. Restricciones: la raíz no se mueve de la página 2; los magics siguen validándose al `open`; `range_scan` sigue devolviendo rango completo (ahora navegando hojas); todos los tests existentes pasan, y añades uno de roundtrip en disco con más de 203 claves. Bonus: usa el campo `reserved: u64` del `BPlusHeader` como puntero a la primera hoja.

## Para profundizar

- **Bayer & McCreight, «Organization and Maintenance of Large Ordered Indices»** (*Acta Informatica* 1(3), 173-189, 1972; presentado en ACM SIGFIDET 1970) — el paper original, corto y sorprendentemente legible, de los Boeing Scientific Research Labs.
- **Douglas Comer, «The Ubiquitous B-Tree»** (*ACM Computing Surveys* 11(2), 1979) — la encuesta que popularizó la estructura y dejó escrito el misterio de la B.
- **El sitio oficial de FNV** (isthe.com/chongo/tech/comp/fnv) y el **draft IETF draft-eastlake-fnv** — historia (1991, POSIX P1003.2), constantes y vectores de test que usa `fnv1a_known_values`.
- **PostgreSQL**: documentación de índices (§11.2: la historia de los hash sin WAL pre-v10 y por qué B-tree es el default) y `src/backend/access/nbtree/README` (la variante Lehman-Yao y la deduplicación de la v13).
- **Alex Petrov, «Database Internals» (O'Reilly), caps. 2-4** — B-tree basics, formatos de fichero y la implementación completa de un B-tree con page splits: el apéndice de este capítulo en versión larga.
- **CMU 15-445** — las lecciones de tree indexes: el B+ tree multinivel con splits, explicado con la misma filosofía de páginas que hemos seguido aquí.

## Mini-diálogo: en guardia nocturna

> — A ver: tengo un hash O(1). ¿Para qué quiero un B+ tree que es O(log n)? O(1) gana a O(log n) en cualquier pizarra.

> — En la pizarra sí. Ahora pregúntale a tu hash por «todas las aristas con peso entre 5 y 9». Se rasca la cabeza y te escanea los dieciséis buckets, uno a uno.

> — …porque el bucket del 5 no tiene nada que ver con el del 6.

> — Exacto: la dispersión no es un defecto del hash, es su producto. La clave 500 y la 501 caen en mundos distintos porque si cayeran cerca, los clusters las lian. Elegir índice no es elegir el más rápido: es decidir qué pregunta quieres que sea barata. Igualdad exacta y masiva → hash. Rangos, orden, «dame el siguiente» → B+.

> — Y lo del FNV escrito a mano, ¿no era orgullo de más? `HashMap` viene con el idioma.

> — `HashMap` viene con una semilla aleatoria por instancia. En RAM es una virtud: nadie te clava un HashDoS. En disco es una sentencia: la clave 42 vivía en el bucket 7 ayer, hoy el hasher la manda a la 3, y tu `get` devuelve `None` para un dato que está ahí, en el fichero, perfectamente sano. Pérdida silenciosa con traje de bug de negocio. Por eso el hasher de un índice persistente es formato en disco: constantes publicadas, cero semillas, verificado contra vectores oficiales.

> — Y la B del B-tree, ¿al final qué era?

> — Boeing, balanced, bushy, Bayer… McCreight nunca lo dijo. Y cuanto más piensas en lo que significa, mejor entiendes los B-trees. Con eso te quedas esta noche.

---

*(Próximo capítulo: 16 — Compactación y mantenimiento. Los índices son estáticos y «actualizarse» es rebuild: ahora hay páginas huérfanas por el fichero, `free_space` que miente y basura que acumular. Llega `liradb inspect|check|compact` — y de regalo, los LSM-trees mirando desde la otra acera.)*
