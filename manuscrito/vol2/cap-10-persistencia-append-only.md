# Capítulo 10 — Persistencia append-only

> *«No borres nunca una línea de la bitácora: cuando el mar se pone feo, lo único que puedes hacer es añadir al final.»*

## 10.0 La anécdota de la esquina

En 1978 un investigador de IBM llamado Jim Gray publicó una tesis que parecía aburridísima: trataba de **logs**, de cómo los sistemas transaccionales tenían que dejar constancia escrita de lo que hacían antes de hacerlo. La idea no nació de una oficina con UNIX, sino de algo más viejo: las **bitácoras de los barcos** y las **tarjetas perforadas de los mainframes**, donde la disciplina era la misma que la de un contable — *nunca tachar lo escrito; añade una línea nueva*. Cuando un día se quemó un disco en un sistema de reservas de vuelos (un fallo que, a mitad de una escritura, dejaba alterado el fichero principal), la única forma de saber qué existía realmente era leer el **log**: la secuencia de cambios bien formada que se había ido apéndizando por delante.

Gray formalizó eso que hoy llamamos **write-ahead log** (WAL, 1981): *escribe el cambio en el log ANTES de tocar los datos*. Y hay un detalle más sutil que el propio Gray remarcó: el log no te salva *a pesar de* las escrituras a medias, sino que es inmune a ellas. Si un registro del log queda a medias (cortó la luz), las filas anteriores siguen **íntegras** — porque nunca pisaste ninguna. LiraDB no inventa nada aquí: este capítulo es la semilla más pequeñita de esa idea, y la vas a plantar tú, en Rust, antes de que existan las páginas, el buffer pool o las transacciones.

## 10.1 Objetivo

Al terminar este capítulo sabrás **por qué una base de datos no reescribe los datos en sitio cuando algo cambia**, y habrás implementado la pieza de bajo nivel que lo resuelve: un **log append-only** donde cada cambio es un **registro** — con un *framing* que dice dónde termina cada uno y un **CRC32** que detecta si algo se rompió.

En concreto, vas a construir cinco piezas en `liradb-workspace/crates/vol2-liradb/src/cap10_append_only.rs`:

1. `RecordKind` — el tipo de cambio (PutNode, PutEdge, DeleteNode, DeleteEdge, Commit).
2. `LogRecord` — el cambio en memoria (kind + id + payload).
3. `encode_log_record` / `decode_log_record` — el formato en bytes, **length-prefix + CRC32**.
4. `crc32_simple` — el checksum estándar, a mano.
5. `AppendOnlyLog` + `LogIterator` — el log y su iterador que **para limpio** ante la corrupción.

## 10.2 Problema

Imagina que ya sabes convertir un `Node` en bytes (capítulo 9). Ahora viene la pregunta incómoda: **cuando Ana y Luis añaden una arista, ¿dónde y cómo guardas ese cambio en el fichero?**

La primera idea que se le ocurre a todo el mundo es **sobrescribir en sitio**: abro el fichero, voy al byte donde vive esa arista, escribo el byte nuevo encima y cierro. Rápido. Barato. Y funciona… hasta que ocurre un **corte de luz a mitad de la escritura**.

Y aquí está el problema que el capítulo quiere que sientas antes de resolverlo. Resulta que "escribir un byte" **no es una operación atómica a nivel físico**: el disco escribe en unidades de **sector** (512 bytes o más), y si la electricidad se va justo en el instante en que el brazo está cambiando ese sector, lo que queda en la mitad puede ser un estado intermedio — una mezcla del dato viejo y del nuevo, a mitad de bit. Peor aún: como "pisaste" la versión buena, **ya no tienes de dónde saber cuál era**. La arista podría estar medio escrita, o escrita dos veces con una mitad de cada una. Y el programa no se entera.

Encima, guardar datos de longitud variable (un nodo con una descripción larga, otro con solo un nombre) tiene el problema que ya viste con las páginas del capítulo 11: si el nuevo contenido es más largo que el hueco, tienes que **desplazar todos los bytes que vienen detrás** — O(n) por cambio y muchas más oportunidades de escritura a medias.

La conclusión duele: **sobrescribir en sitio convierte un crash en corrupción silenciosa**. Necesitamos una estrategia en la que un corte de luz no pueda embarrar lo ya guardado.

## 10.3 Modelo mental

Piensa en el **cuaderno de bitácora del barco** (y en su descendiente moderno: el log de escritura anticipada de Gray). El contramaestre nunca borra una línea ni reescribe una fecha encima: **añade cada observación al final**. Cuando llega la tempestad y todo se va al carajo, la guardia anterior queda **íntegra y en orden**, porque nadie la pisó. Lo único "raro" que puede quedar es **la última línea a medio escribir** — y precisamente por eso es *visiblemente* rara: le falta el final, o su sello de verificación no cuadra.

```
ESCRITURA EN SITIO (el barco reescribiendo la línea 3 del censo):
  ┌──────┬──────┬──────┬──────┐
  │reg 1 │reg 2 │reg 3 │      │      ← el crash pisa reg 3 a mitad: CORRUPCIÓN
  └──────┴──────┴──╳───┴──────┘        ya no sabes cuál era el reg 3 bueno

LOG APPEND-ONLY (el barco apéndizando):
  reg1 │ reg2 │ reg3 │ ...  ← el crash deja una cola →  reg4 a medias
  └────┴─────┴─────┴────┘                                                   
        íntegro y en orden (nadie pisó nada)
                                  [reg4 a medias] se detecta (CRC) y se
                                  descarta: vuelven a valer reg1..reg3
```

Este es el momento ¡ajá! del capítulo: **la durabilidad no se gana haciendo las escrituras más fuertes, sino haciéndolas imposibles de hacer mal.** Si nunca pisas lo ya escrito, un corte de luz solo puede *cortarte el final*, y el final se detecta solo.

## 10.4 Primera solución

Empecemos por lo que escribiría un novato. Iba a abrir el fichero y modificar en sitio el byte donde vive el nodo:

```rust
// Solución ingenua (NO es esto lo que construimos): sobrescribir en sitio.
fn actualizar_en_sitio(fichero: &mut File, nodo: &Node) -> io::Result<()> {
    let pos = buscar_posicion_del_nodo(nodo.id);
    fichero.seek(SeekFrom::Start(pos))?;
    let bytes = encode_node(nodo);           // cap. 9
    fichero.write(&bytes)?;                  // ESCRIBE ENCIMA del viejo
    Ok(())
}
```

Parece perfecto: busca, y escribe encima. Los tests pasan. Nadie se queja. Durante un rato.

## 10.5 Sus límites

Hasta que alguien tira de la corriente en el momento justo. Los límites de **sobrescribir en sitio** son tres y los tres son serios:

1. **Una escritura a medias pisa lo bueno.** La física del sector hace que "cambiar un byte" pueda quedar a mitad de camino. Como pisaste la versión buena, **no hay forma de reconstruir el estado anterior**. Es la corrupción silenciosa e irreparable.
2. **Los cambios de longitud variable desplazan todo.** Si el nodo que llega es más largo que el que había, los bytes posteriores hay que moverlos. Cada movimiento es más escritura en sitio, más posibilidades de mitad.
3. **No dejas rastro de orden.** "¿Qué cambió primero? ¿Qué es lo último válido?" Sin un historial, no hay *redo*, no hay *undo*, no hay forma de "arrancar de nuevo" desde algo que se sabe bueno.

La conclusión: **necesitamos una escritura que nunca pise lo que ya está**. Esa es exactamente la propiedad del log append-only.

## 10.6 Solución evolucionada

La idea tiene décadas — es el corazón de la recuperación transaccional que Bernstein, Hadzilacos y Goodman formalizaron en 1987 (los *redo/undo logs*) y que Gray ligó al write-ahead log desde 1981. Es esta:

**Cuando algo cambia, no busques dónde vivía y pisa: encadena al final del fichero un registro completo del cambio.** Cada registro se **enmarca** (le decimos de antemano cuántos bytes mide) y se **sella** (le colgamos un checksum). Así:

1. Para añadir: `append(record)` — escribes los bytes al final. El fichero crece, no se pisa nada.
2. Para leer: caminas **siguiendo los length-prefix**, sin escanear contenido.
3. Para detectar un crash: verificas el **CRC32** de cada registro; si no cuadra, está a medias.

El layout exacto de un registro, el mismo que vas a ver reflejado en el código, es:

```
[record_len: u32 LE]   ← cuántos bytes cubre TODO lo que sigue (framing)
[kind: u8]             ← el tipo: 1 PutNode, 2 PutEdge, 3 DeleteNode, 4 DeleteEdge, 5 Commit
[id: u32 LE]           ← el id del nodo/arista (u32 estable, cap. 3 Vol. I)
[payload_len: u32 LE]  ← cuántos bytes de payload
[payload bytes...]     ← el contenido (aquí, meramente conceptual)
[crc32: u32 LE]        ← el «cubre-roñoso»: verifica kind||id||payload_len||payload
```

Fíjate en dos decisiones finas del framing:

- **`record_len` NO cuenta sobre el payload:** cuenta sobre el "inner" (todo lo que sigue al propio prefijo de 4 bytes). Así el iterador sabe exactamente cuánto saltar (`pos += 4 + inner_len`).
- **El CRC cubre `kind || id || payload_len || payload`, NO el `record_len`.** El prefijo de longitud es la frontera: si un byte del prefijo se corrompe, la lectura ya falla por `truncated` (o el CRC del registro siguiente lo atrapa). Cubrir el prefijo también valdría, pero es cómputo añadido para casi nada.

Y el formato es **little-endian** — igual que el capítulo 9. El log entero se edifica sobre `encode_u32_le`/`decode_u32_le`: una sola convención endian para todo el motor.

## 10.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap10_append_only.rs`. Lo leemos por partes, porque cada línea tiene un porqué.

### El tipo de registro: `RecordKind` y `LogRecord`

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordKind {
    PutNode = 1,    // insert/update de un nodo
    PutEdge = 2,    // insert/update de una arista
    DeleteNode = 3,
    DeleteEdge = 4,
    Commit = 5,     // commit point (checkpoint para recovery)
}
```

Cuatro tags de escritura más un `Commit`. Este enum es **la semilla deliberada** de lo que el capítulo 27 llamará `Operacion`. Fíjate en el nombre del tipo y en el orden de los tags: no es casual que PutNode/PutEdge/DeleteNode/DeleteEdge mapeen 1-1 a las variantes de la `Operacion` del capítulo 27. Se planta ahora el shape para que, cuando el capítulo 28 serialice el *buffer* de operaciones de una transacción al **WAL**, lo haga **sin reinterpretar** — el comentario del propio `cap28_wal.rs` lo declara explícito: *«la semilla era deliberada»* (línea 33) y *«los tags 1-4 replican el orden del `RecordKind` del cap. 10»* (línea 402).

El registro en memoria:

```rust
pub struct LogRecord {
    pub kind: RecordKind,
    pub id: u32,
    pub payload: Vec<u8>,
}
```

`payload` aquí es un `Vec<u8>` conceptual, no un `Value` ni un `Node`: en capítulo 10 todavía no hay transacciones. El capítulo 27 llenará ese payload con la `Operacion` completa.

### El formato: `encode_log_record` y `decode_log_record`

```rust
pub fn encode_log_record(rec: &LogRecord) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(rec.kind as u8);
    body.extend_from_slice(&encode_u32_le(rec.id));
    body.extend_from_slice(&encode_u32_le(rec.payload.len() as u32));
    body.extend_from_slice(&rec.payload);

    let crc = crc32_simple(&body);
    let mut inner = body;
    inner.extend_from_slice(&encode_u32_le(crc));

    let len_prefix = encode_u32_le(inner.len() as u32);
    let mut out = Vec::with_capacity(4 + inner.len());
    out.extend_from_slice(&len_prefix);
    out.extend(inner);
    out
}
```

El `cuerpo` (kind + id + payload_len + payload) se convierte en el "inner" añadiéndole el CRC; y el inner se abre con el length-prefix. La decodificación es la parte crítica y la más defensiva:

```rust
pub fn decode_log_record(bytes: &[u8]) -> Result<(LogRecord, &[u8]), String> {
    if bytes.len() < 17 { return Err(format!("record: need at least 17 bytes, have {}.", bytes.len())); }
    let inner_len = decode_u32_le(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + inner_len { return Err(format!("record: truncated ...")); }
    let inner = &bytes[4..4 + inner_len];
    // ... verificar CRC sobre `body` (inner sin los 4 de CRC) ...
    // ... parsear kind / id / payload_len / payload con comprobaciones en cada paso ...
    Ok((LogRecord { .. }, &bytes[4 + inner_len..]))
}
```

Tres comprobaciones defensivas obligatorias: el mínimo de 17 bytes, `bytes.len() < 4 + inner_len` (nada de `slice` fuera de rango -> `panic`), y `body.len() < 9 + payload_len`. Siempre devolvemos `Result`, nunca datos a medias "como si fueran buenos". Esto es la **lectura estricta**: cuando pides un registro concreto y está corrupto, lo sabes.

### El CRC: `crc32_simple`

```rust
pub fn crc32_simple(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    crc ^ 0xFFFF_FFFF
}
```

CRC32 del **polinomio IEEE 802.3** (`0xEDB8_8320`), el estándar: lo usan Ethernet, zlib y la mayoría de formatos de red/fs. La versión aquí es didáctica — O(n) por byte, sin tabla de 256 entradas, sin dependencias. El banner lo confiesa: *«Para producción usaríamos `crc32fast`»*. Y los tests sujetan el algoritmo a los **valores canónicos del estándar**: `crc32("")` debe dar `0`, y `crc32("a")` debe dar `0xE8B7BE43`. Si tu implementación diera otra cosa, no es que "funcione de otra forma": es que no es CRC32 interoperable. (En un CPU de verdad habría una instrucción CRC32-C; la mayoría de las implementaciones de producción la usan.)

### El log y su iterador de recuperación

```rust
pub struct AppendOnlyLog { bytes: Vec<u8>, count: usize }

pub fn append(&mut self, rec: &LogRecord) -> usize {
    let encoded = encode_log_record(rec);
    let offset = self.bytes.len();
    self.bytes.extend_from_slice(&encoded);   // APPEND: no se pisa nada
    self.count += 1;
    offset
}
```

En RAM, el "disco" es un `Vec<u8>` y los registros se encadenan por length-prefix. El comentario del struct lo deja claro: *«En producción, el 'disco' sería un `File` con `O_APPEND`»*. El `O_APPEND` del sistema operativo es exactamente esta garantía: escribes al final, que el SO te lo promete.

Y ahora la pieza que es **literalmente la semilla del WAL del capítulo 29**:

```rust
fn next(&mut self) -> Option<Self::Item> {
    if self.pos >= self.bytes.len() { return None; }
    match decode_log_record(&self.bytes[self.pos..]) {
        Ok((rec, rest)) => { self.pos = self.bytes.len() - rest.len(); Some(rec) }
        Err(_) => None,          // ← PARA LIMPIO: descarta la cola corrupta
    }
}
```

Contraste deliberado de dos filosofías de lectura:

- **`decode_log_record` es ESTRICTA**: ante corrupción devuelve `Err` (truncated, crc mismatch, unknown kind). Cuando NECESITAS un registro concreto, la corrupción es un evento que debe saberse.
- **`LogIterator` es la ITERACIÓN DE RECUPERACIÓN**: ante el primer error devuelve `None` y **para**. Entrega el prefijo íntegro y descarta la cola ilegible. En un corte de luz no quieres que la recuperación *falle* porque el último registro esté a medias: quieres que rescate todo lo válido y pare ahí.

Eso es el comportamiento exacto del WAL del capítulo 29: *leer hasta el prefijo íntegro y descartar la cola*. El `cap28_wal.rs` lo explica con las mismas palabras (líneas 157-160: «`CrcInvalido` ... se LEE HASTA AQUÍ y se descarta la cola; el iterador para limpio, `decodificar_wal` lo grita»).

Para simular el crash en RAM sin fichero, hay `truncate_to`:

```rust
pub fn truncate_to(&mut self, len: usize) { self.bytes.truncate(len); }
```

Es el análogo de un corte de luz a mitad de escritura: cortas el final del `Vec`. Con eso se testea la promesa central del capítulo.

## 10.8 Prueba de fuego

La prueba de fuego no es "los tests pasan" — es **"el prefijo sobrevive al crash"**. Veamos los seis tests de `mod tests_log`:

```rust
// Valores canónicos del estándar CRC32:
assert_eq!(crc32_simple(b""), 0);
assert_eq!(crc32_simple(b"a"), 0xE8B7_BE43);

// Roundtrip: codificar → decodificar devuelve el mismísimo registro:
let rec = LogRecord { kind: RecordKind::PutNode, id: 42, payload: vec![1,2,3,4] };
let encoded = encode_log_record(&rec);
let (decoded, rest) = decode_log_record(&encoded).unwrap();
assert_eq!(decoded, rec);
assert!(rest.is_empty());

// Un único byte corrupto (el CRC) se DETECTA:
let mut encoded = encode_log_record(&rec);
encoded[encoded.len()-1] ^= 0xFF;
assert!(decode_log_record(&encoded).is_err());

// CRASH: truncar el log a la mitad y comprobar que el prefijo se lee:
let mid = log.len() / 2;
log.truncate_to(mid);
let records: Vec<LogRecord> = log.iter().collect();
assert!(!records.is_empty());      // al menos un registro íntegro antes del corte
for r in &records {
    assert_eq!(r.kind, RecordKind::PutNode);   // y todos son válidos
}
```

Los tres casos de fallo encapsulan las tres promesas del capítulo: **roundtrip fiel, corrupción detectada, prefijo íntegro rescado**. Si este capítulo se te olvidara, tu LiraDB escribiría los cambios pisando el fichero y — tras un corte de luz — leería un estado a medio cambiar sin saberlo. Ese es el síntoma: **corrupción silenciosa e irreparable tras un crash**.

## 10.9 Qué hemos sacrificado

Toda estrategia tiene un precio. El append-only no es gratis:

1. **Desgaste de disco**: el fichero **solo crece**. Cada cambio añade bytes y nadie borra los antiguos. Si no compactas, un millón de `PutNode` sobre el mismo nodo ocupan un millón de registros. La respuesta (compactar, checkpoint, truncar el prefijo ya durable) es material del capítulo 16 y del capítulo 29. Este cap. la deja **documentada como deuda a saldar**, no la resuelve.
2. **Releer es caro**: para encontrar el estado "actual" de un dato, un log puro obliga a recorrerlo desde el principio y quedarse con la última versión O(n). Por eso el log NO es el formato final de datos: es la pieza *de durabilidad*. Las páginas del capítulo 11 y el pager del capítulo 12 dan el acceso directo; el log da la crash-safety. Son dos formatos complementarios.
3. **Sobrecarga por registro**: length-prefix + kind + id + payload_len + CRC ≈ 17 bytes de envoltura por registro. Para cambios diminutos es proporcionalmente mucho. Es el precio de poder leer y verificar cada uno de forma independiente.
4. **Sin trueque de espacio**: el CRC detecta, pero no corrige (para corregir haría falta redundancia de *datos*, tipo RAID — no lo necesitas aquí).

La elección de fondo es Wisdom del capítulo: un log plano, sin índices, **vale más que cualquier layout sofisticado** que aún no puede garantizar consistencia ante un crash. La simplicidad y la crash-safety pesan más — y el rendimiento del acceso directo lo aportarán las páginas del capítulo 11.

## 10.10 Cómo lo hace una BBDD real

El log por escritura anticipada es el estándar de hecho en casi todas:

- **Jim Gray** (1978-1981) formalizó el **write-ahead log** y la idea de que la durabilidad se apoya en escribir el cambio al log *antes* de tocar los datos. Su trabajo con el "long lifetime" de los datos y las formas de fallo de los sistemas es la base de todo.
- **Bernstein, Hadzilacos y Goodman** (1987), en *Concurrency Control and Recovery in Database Systems*, describen los **redo/undo logs**: un registro del cambio se puede re-aplicar (redo, tras un crash a mitad de commit) o deshacer (undo, si la transacción abortó). El capítulo 29 (ARIES) los combina.
- **SQLite** es el ejemplo más cercano y didáctico: tiene **dos modos**. El *rollback journal* guarda la imagen antigua ANTES de cambiar la página para poder deshacer. El **WAL mode** guarda los cambios en un fichero separado append-only y *aplica* al fichero principal en un *checkpoint* (`wal_checkpoint`) — exactamente la misma separación de conceptos que aquí: un log que crece y un "lavado" posterior que integra.
- El **CRC32** (IEEE 802.3) es el mismo checksum que usa Ethernet y zlib; en producción LiraDB usaría `crc32fast` (tablas + SIMD), no la versión de juguete — pero el formato en disco sería **interoperable** porque el polinomio es el estándar.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `record_len` cubre el "inner" (todo excepto el propio prefijo) y no solo el payload? ¿Qué pasaría si el iterador avanzara con `pos += 4 + payload_len + crc` olvidando el kind y el id?
- *Intermedio*: extiende el formato para que el payload lleve un `Value` real (cap. 9): añade un campo `tipo_payload` para distinguir nodo/arista. ¿Cómo cambia el layout y qué pisa ahora el CRC?
- *Experto*: implementa `log_a_estado(log)` que recorra el `AppendOnlyLog` y devuelva el "estado final" aplicando los `PutNode`/`DeleteNode`/`PutEdge`/`DeleteEdge` en orden, ignorando la cola corrupta. Es el ancestro del *redo* del capítulo 28.

## 10.11 Lo que te llevas

- La **página del barco**: apéndiza cada cambio al final; no reescribes nunca.
- El **layout**: `[record_len][kind][id][payload_len][payload][crc32]`, little-endian del cap. 9.
- El log es **inmune a las escrituras a medias**: nunca pisa, y el CRC detecta la cola cortada.
- **Length-prefix > end-marker** y **CRC32 > checksum de 1 byte**.
- El **germen del WAL**: same framing y tags del cap. 10 se reutilizan en el cap. 28.

## 10.12 Ojo, cuidado con…

- **Tratar el log como un fichero de datos editable**: no muevas ni borres bytes del medio "para compactar". La compactación es un lavado aparte (caps. 16 y 29), no una edición in situ.
- **Calcular el CRC sobre el prefijo**: el CRC cubre solo `kind||id||payload_len||payload`. Mantén el contrato del módulo, o el verificado no cuadrará.
- **`to_ne_bytes` en vez de `encode_u32_le`**: funciona en x86 y corrompe todo en big-endian. El log es formato de disco: explícito, LE.
- **No comprobar `bytes.len() < mínimo`**: un fichero truncado a 5 bytes paniquea en `bytes[4..]`. La validación defensiva no es opcional.
- **Usar `decode_log_record` (estricto) donde debes usar `LogIterator` (recuperación)**: con un solo byte corrupto abortarías la recuperación entera.

## 10.13 Pin de batalla

> *«Un log append-only no hace las escrituras más fuertes: las hace imposibles de hacer mal. El resto —detección, recuperación, durabilidad— es consecuencia.»*

## 10.14 Si solo lees 30 segundos

Una base de datos no reescribe los datos en sitio: para cada cambio **apéndiza un registro nuevo al final** de un log append-only. Cada registro va **enmarcado** (un `record_len` de 4 bytes dice cuánto mide) y **sellado** (un `CRC32` sobre su contenido). Como nunca pisa lo ya escrito, un corte de luz solo deja un **prefijo íntegro** más una cola corrupta que el CRC detecta: el iterador lee el prefijo y **para limpio**. Ese framing y esos tags son la semilla exacta del **WAL** del capítulo 28.

## 10.15 Una historia pequeña

El día que arrancamos LiraDB con "guardar en sitio", Ana añadió 200 aristas y se fue a comer. Al volver, se había ido la luz a mitad del *segundo* y el fichero tenía un aspecto perfectamente válido — con aristas a medio escribir que parecían reales. No había ningún error: el programa leía, sonreía y devolvía silencio-tratado-de-datos. Tardamos la tarde en darnos cuenta de que el fallo no estaba en el algoritmo, sino en que **habíamos pisado lo bueno** y ya no existía forma de saber cuál era. El append-only fue la respuesta: ahora, cuando la luz se va, el fichero se ve *exactamente* correcto hasta el último registro íntegro, y el resto grita. No porque seamos más listos: porque ya no pisamos nada.

## Ejercicios resueltos

**1. ¿Cuántos bytes mide un `PutNode` con `id=7` y `payload=[1,2,3]`?**

Layout: `record_len` (4) + kind (1) + id (4) + payload_len (4) + payload (3) + CRC (4) = **20 bytes**. El "inner" (lo que sigue al `record_len`) mide 1+4+4+3+4 = 16; por tanto `record_len` = 16, y el total es 4 + 16 = 20. Tras escribir `encode_u32_le(7)` (los 4 bytes `07 00 00 00` en LE) y el `payload_len` `03 00 00 00`, el CRC se calcula sobre esos 1+4+4+3 = 12 bytes de cuerpo. Puedes comprobarlo con `encode_log_record(...).len()`.

**2. ¿Por qué `LogIterator::next` devuelve `None` ante un error, y no `Err`?**

Porque un iterador en Rust `Option`/`Iterator` no propaga errores sin complicar el trait; y porque la semántica `None` es exactamente la correcta para la recuperación: "no hay más registros *válidos*". Ante una cola corrupta (un crash), quieres entregar los registros íntegros y **parar** — no hacer fallar la recuperación entera. Si necesitas saber *que* hubo corrupción, usas `decode_log_record` directo (estricto). Son dos herramientas con dos contratos.

## Ejercicios propuestos

**Esencial.** Codifica a mano, sin mirar el módulo, un `LogRecord` `PutNode` con `id=7`, `payload=[1,2,3]`: escribe la secuencia exacta de bytes (kind, id LE, payload_len LE, payload, y dónde iría el CRC) y verifica con `encode_log_record` que tu layout mide 20 bytes. *(Retrieval: re-construir de memoria el layout del cap. 10. Spacing: re-usa `encode_u32_le` del cap. 9.)*

**Intermedio (mezcla caps. 9 y 10).** Como payload, usa un `Value::Float(PI)` codificado con `encode_value` del cap. 9 (8 bytes de `f64` LE). Explica qué pisa el CRC (los 17 bytes de kind+id+payload_len+payload, ahora con 8 bytes de float) y qué pasa si corrompes el byte de la parte exponencial: en `decode_log_record` (Err crc mismatch) vs en `LogIterator` (para y no entrega la cola). Verifica con `log_record_corrupto_falla` y `log_recovery_desde_offset`.

**Experto.** Implementa `crcer_entero(log: &AppendOnlyLog) -> u32` (un CRC32 de toda la secuencia `log.as_bytes()`) y `truncar_sin_corromper(log, target_len)`: trunca SOLO a un límite de registro completo (un offset que sea exactamente `4 + inner_len` acumulado), nunca a mitad de un record. Escribe un test que pruebe que tras `truncar_sin_corromper` el iterador devuelve N registros completos y que un `truncate_to` a mitad de record deja `iter()` cortando limpio.

## Para profundizar

- **Jim Gray, "The Transaction Concept: Virtues and Limitations", 1981** — el WAL y por qué los logs son la pieza de durabilidad de las transacciones.
- **Bernstein, Hadzilacos & Goodman, "Concurrency Control and Recovery in Database Systems", 1987** — redo/undo logs y la teoría de la recuperación, formal.
- **Documentación oficial de SQLite, "Write-Ahead Logging"** — `wal_checkpoint`, WAL mode vs rollback journal: el mismo concepto visto en un motor de verdad, con el código a un clic.
- **"Designing Data-Intensive Applications" (Martin Kleppmann), cap. 3** — por qué los logs y el append-only son ubicuos (desde mensajería hasta SSTables, el bisado con el cap. 15 de LiraDB).
- **`crc32fast` (crate de Rust)** — la implementación de producción del CRC32 IEEE 802.3 con tablas y SIMD; mira su fuente para ver por qué la versión de juguete del cap. es solo didáctica.

## Mini-diálogo: en guardia nocturna

> — Espera. ¿Todo este capítulo es "escribir al final de un fichero"? ¿Tan simple?
>
> — Es tan barato de entender como caro de descubrir. Gray tardó años en darse cuenta de que la durabilidad no es hacer las escrituras *imponentes*, es hacerlas *irreversibles en la dirección correcta*. Nunca piso lo escrito.
>
> — ¿Y el CRC? ¿Es ese de verdad "el que no se calla"?
>
> — Detecta el byte que giró sin pedir permiso. Una cola a medias ya no es "datos que parecen válidos": es ruido que el iterador descarta sin drama. Con esto, un corte de luz ya no asusta: nos corta el final, y el final sabemos leerlo.
>
> — ¿Y esto qué pinta en una base de datos de grafos?
>
> — Es la semilla más chiquitita del WAL. Dentro de unos capítulos, cuando hablemos de transacciones y de recuperación, cada registro que escribimos aquí se volverá un `record` del *write-ahead log* con un LSN y un id de transacción. Y el formato ya estará plantado. La semilla se siembra antes de lo que parece.

---

*(Próximo capítulo: 11 — Páginas, bloques y slotted pages. Aquí persistimos cambio a cambio; ahora veremos en QUÉ UNIDAD se lee y escribe el fichero para no volvernos O(n) — la página de tamaño fijo que abre la Parte III, el motor de almacenamiento.)*
