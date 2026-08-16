# CONTRATO DE CAPÍTULO — Vol.II Cap. 09: Del objeto al byte (encoding, endianness, versionado)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap09_encoding.rs` (182 líneas,
> 3 tests en `tests_encoding`, verificables con `cargo test -p vol2-liradb
> --lib cap09` → PASS). Depende del `Value` de `cap07_modelo.rs`
> (`liradb-workspace/crates/vol2-liradb/src/cap07_modelo.rs`). Este capítulo
> ES la carta de estar por casa de la Parte III: sin él, slotted pages
> (cap. 11), el Pager (cap. 12), el CSR persistente (cap. 14), el WAL
> (cap. 28) y la recuperación (cap. 29) no pueden escribir UN byte. Ganchos:
> cap. 10 (append-only, donde los `Vec<u8>` de aquí se convierten en un log)
> y cap. 11 (que retoma el length-prefix desde el ángulo de la página).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el enum `Value` del cap. 7 (`Null | Bool |
  Int(i64) | Float(f64) | String(String) | Bytes(Vec<u8>)`) y cómo se usa
  como dato flexible en las propiedades de los `Node`/`Edge`; la diferencia
  entre tipos concretos y el tipo de datos `Value`; el trait `GraphStore`
  del cap. 8 como frontera de diseño "antes de persistir"; Rust básico:
  `match`, vectores `Vec<u8>`, slices `&[u8]`, `Result`/`Option`,
  `to_string()`, tests con `#[cfg(test)]`.
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**:
  (1) «un string en memoria ya es bytes, solo hay que escribir esos bytes» —
  NO: un `String` Rust es UTF-8 pero sin límite de longitud escrito junto al
  dato; al leerlo, ¿dónde acaba? Sin length-prefix, no hay dónde; (2) «la
  endianness es un detalle de teóricos, en mi máquina funciona» — funciona
  HASTA que un fichero escrito en tu x86 se abre en un ARM big-endian y se
  corrompe en silencio; (3) «el tag del enum se puede deducir» — si no lo
  escribes, el decodificador no sabe montar `Float` o `String`; (4) «mi
  formato es para siempre» — el día que añadas una variante a `Value`, los
  ficheros viejos o bien revientan o, peor, se leen «casi-bien».
- **NO debe saber todavía**: persistencias reales (páginas, buffer pool,
  WAL, MVCC — caps. 11-30), checksums/CRC (cap. 10), serialización de
  grafos completos, sync/durabilidad real. Se nombran como «luego lo verás»
  y se corta. El encoding de este capítulo es el MÍNIMO: un `Value` suelto y
  sus primitivas, en memoria y de vuelta.

## 2. Conceptos (del grafo curricular)

- `present`: encoding binario reversible (encode/decode); endianness
  (little- vs big-endian; `to_le_bytes`/`from_le_bytes` explícitas);
  length-prefix para longitudes variables vs delimitadores; tag (`u8`) que
  discrimina la variante de un enum; format versioning con `FORMAT_VERSION`
  + magic byte (`encode_header`/`decode_header`); determinismo (un valor →
  los mismos bytes, reclamado por los CRC del cap. 10); formato en disco
  INDEPENDIENTE de la máquina (nada de `to_ne_bytes`).
- `practice`: `Value` del cap. 7 (spacing central: este capítulo LO codifica);
  `match` exhaustivo sobre enum; slices y `copy_from_slice`; `Result` con
  errores `String` descriptivos.
- `consolidate`: «derivar, no llevar en cabeza» (la longitud se escribe
  ANTES del dato, el resto se deriva); política de fallo ruidoso (decode
  devuelve `Err("truncated")`, nunca datos a medias como válidos).
- `out_of_scope` (solo nombrar): páginas/slots (cap. 11), CRC/checksum
  (cap. 10), append-only (cap. 10), serde u otros crates (el libro es
  «primero a mano, luego con crates» — aquí sin serde), WAL (cap. 28).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica POR QUÉ little-endian como default (x86 y la
  casi totalidad de las máquinas, y la reversibilidad byte a byte: el primer
  byte es siempre el menos significativo) y por qué las redes prefieren
  big-endian; (2) enuncia la regla del length-prefix frente al end-marker
  (por qué un marcador de fin envenena con escapes y el prefijo no); (3)
  justifica `FORMAT_VERSION` y el magic: un entero de versión protege los
  ficheros viejos de un cambio de esquema; (4) distingue los problemas
  propios de `f64` (IEEE 754: NaN, precisión, representación binaria) de los
  de los enteros; (5) dice por qué el determinismo (valor → mismos bytes) es
  un requisito para que un CRC de capítulos posteriores tenga sentido.
- **Skills**: (1) codificar y decodificar a mano u32/i64/f64/string y el
  enum `Value` completo con tag + length-prefix; (2) escribir un test de
  roundtrip (encode → decode → igualdad, y `rest.is_empty()`); (3) usar
  `to_le_bytes`/`from_le_bytes` explícitas y evitar `to_ne_bytes`.
- **Wisdom**: (1) decide NO usar `to_ne_bytes`: cualquier vez que dependas
  de la máquina, tu formato de disco no es portable (modo de fallo: corrupción
  en otra arquitectura); (2) decide cuándo NO perseguir compatibilidad hacia
  atrás a costa de un formato opaco: el versionado es la válvula de escape,
  no la bolsa de complejidad.

## 4. Modelo mental

- **El archivador que VUELVE de una carta**. Escribir en disco = dictarle al
  archivador una carta que otro archivador (quizá en otra máquina, quizá
  dentro de 20 años) debe PODER releer. Toda carta tiene: (a) un *sobre con
  la firma y el número de formato* (magic + `FORMAT_VERSION`): si el sobre no
  es legible, no abres — no adivinas; (b) un *orden de lectura acordado*,
  byte a byte (endianness): si ambos lados no acuerdan qué byte es el primero,
  la carta es galimatías; (c) para cada campo de longitud variable, *cuántos
  caracteres siguen* escritos ANTES del texto (length-prefix) — porque un
  texto libre puede contener cualquier carácter, incluso el que serviría de
  "fin de línea"; (d) para cada dato de tipo flexible, *una etiqueta* al
  principio (tag) que dice qué clase de dato viene.
- **Diagramas ASCII**: (a) flujo de un `Value::String("hola")` a bytes
  `[4][h][o][l][a]`; (b) flujo del `Value::Float(3.14)` a `[3][8 bytes LE]`
  y su vuelta; (c) la cabeza del fichero `[magic 4][version 4]` =
  `encode_header()`/`decode_header()`; (d) los seis tags del enum `Value`
  (0..5) lado a lado con su payload.
- **Momento ¡ajá!**: «`encode` no está "convirtiendo a binario": está
  REDACTANDO una carta con reglas que un desconocido debe poder releer
  byte a byte, sin darme la mano. Si el byte 1 y el byte 2 no significan lo
  mismo para el que escribe y el que lee, el dato no sobrevivió el viaje.»

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap09_encoding.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | `encode_u32_le` = `value.to_le_bytes()` y `decode_u32_le` = `u32::from_le_bytes` | Byte a byte reversible y portable: en LE, el primer byte escrito es el menos significativo; cualquier máquina puede reconstruir el mismo `u32` a partir de los mismos 4 bytes | `to_ne_bytes` (representación nativa): idéntico en la máquina de desarrollo, CORRUPTO en otra endianness sin ningún error | Fichero que se lee distinto (o revienta) en ARM big-endian — corrupción silenciosa | Cap. 11 ya lo clava: «nunca `to_ne_bytes`»; Intel/x86 little-endian (hecho histórico de hardware) |
| 2 | Cuatro primitivas explícitas (u32/i64/f64) en vez de un genérico de bytes crudos | La lectura de un número SUELTO necesita saber de cuántos bytes consta y su orden; primitivas explícitas enseñan la mecánica y dejan el patrón visible para cualquiera que añada otra | Un único «blob de N bytes» sin semántica: el decodificador no sabría cuántos ni en qué orden; repetir a mano bytes = fuente de bugs | Longitudes mal leídas; offsets descuadrados | El código; el cap. 7 `Int(i64)` vs `Float(f64)` (64 bits ambos, distinta semántica) |
| 3 | length-prefix para strings (`encode_string` escribe `u32 len` + bytes UTF-8) | Byte prohibido → escape → corrupción silenciosa; un prefijo de longitud permite CUALQUIER byte en el payload y el decodificador sabe EXACTAMENTE cuántos leer sin escanear | Delimitador/end-marker (`\0`): si los datos contienen `\0`, hay que escaparlo; un escape mal hecho es corrupción | Strings que se leen recortados o «con basura pinchada» | Cap. 11 §11.6 (length-prefix vs end-marker), protocolos de red con length-prefix |
| 4 | Tag `u8` (0..5) que precede a cada `Value`, y `u8::from(bool)` para `Bool` | El decodificador necesita saber qué variante del enum es antes de montarla; `match tag` es directo y el tag 0 no ocupa payload | Deducir la variante de la longitud (ambiguo, `Int` y `Float` pesan igual 8) o de un `SizeOf enum` en memoria (dependiente del compilador) | El decodificador monta la variante equivocada en silencio | `encode_value`/`decode_value` (líns. 49-127) |
| 5 | `encode_header` = `[magic 0x4C444231][FORMAT_VERSION u32=1]`, y `decode_header` REVISA el magic | Sin firma, no sabes si el fichero es tuyo ni en qué versión está; la firma + versión permiten «si no es la v1, dime antes de intentar» | Abrir el fichero y «rezar» leyendo bytes con el layout actual | Un fichero de otra versión o de otra app se interpreta como LiraDB y corrompe datos | `encode_header`/`decode_header` (líns. 129-142); SQLite file format spec (magic string + version); PostgreSQL pg_type |
| 6 | decode devuelve `Err(String)` en TODOS los casos de truncamiento (`too short`, `payload truncated`, `tag` desconocido) | Modo de fallo ruidoso: NUNCA entregar datos a medias como si fueran válidos; el llamador decide cómo reaccionar | Panic, o devolver datos incompletos «best effort» | Decodes que entregan Value con payload cortado y lo propagan como bueno | `decode_string`/`decode_value` (líns. 33-47, 78-127); política del cap. 8/11 |
| 7 | Resto devuelto `(&[u8], &[u8])` en cada decode | Permite encadenar valores en un flujo (el `Vec<u8>` concatenado de una página, cap. 11): cada decode consume de `bytes[..4+len]` y deja el resto | Decode que consuma el buffer entero obligando a encabezar cada valor con su longitud total | Imposible embeber varios `Value` en un solo buffer sin cuentas manuales | Firmas de `decode_string`/`decode_value` |
| 8 | `convert Int(i64)->u32` para lengths con `as u32` y comprobar `bytes.len() < 4+len` | Longitudes no negativas, y cada length se comprueba contra el buffer ANTES de indexar, para no leer fuera de rango | Confiar en el U32 y `slicar` sin comprobar (panic por index out of bounds) | Panic o lectura fuera de los límites del slice | Checks `if bytes.len() < ...` en todo el decode |

## 6. Primera solución vs solución evolucionada

- **Ingenua (a mano)**: guardar `Value::String(s)` como `s.as_bytes()` a
  secas, y el `Value::Int` con `to_ne_bytes()`. En la máquina de desarrollo
  los tests pasan. Incluso un `format!("{:?}", v)`/`to_string()` del Value
  (legible pero ambiguo: ¿dónde termina un campo y empieza otro al releer?)
  puede parecer "que se guarda".
- **Qué la rompe exactamente**: (a) un string contiene `\0` o `"` → se lee
  recortado (delimitador) o con el tamaño equivocado; (b) un fichero escrito
  en el PC se abre en una máquina big-endian → `to_ne_bytes` corrompe los
  `u32/i64/f64`; (c) una variante añadida al enum `Value` en el futuro →
  `decode` no reconoce el tag o monta la variante equivocada.
- **Evolución visible**: `encode_*_le` explícitas, length-prefix `u32` para
  strings/y bytes, tag `u8` por variante, y la pareja `encode_header`/
  `decode_header` que firma el fichero con magic + `FORMAT_VERSION`. Un mismo
  `Value` produce SIEMPRE los mismos bytes (determinismo), listo para el CRC
  del cap. 10.

## 7. Prueba de fuego

- **TEST-TESIS** `value_roundtrip`: para `Null, Bool, Int(42), Int(-1234567890),
  Float(PI), String("hola, mundo!"), Bytes([1,2,3,4,5])`, `encode_value` →
  `decode_value` devuelve un `Value` IDÉNTICO (`assert_eq!(dec, v)`) y el
  resto del buffer queda vacío (`rest.is_empty()`).
- **TEST-TESIS** `string_roundtrip`: `"abcdefghij"` regresa intacto.
- **TEST-TESIS** `header_roundtrip`: `encode_header` → `decode_header`
  devuelve `FORMAT_VERSION` (el magic se comprueba, no se ignora).
- **Síntoma si el lector se salta el capítulo**: en el cap. 11 intentará
  meter `Value` dentro de una página y no sabrá dónde termina cada uno (¿dónde
  está la longitud?), en el cap. 10 el CRC fallará porque los bytes no son
  deterministas, y en el cap. 28 el WAL no podrá reconstruir un `Value` tras
  un crash. El síntoma temprano: strings que se leen recortadas o números
  que cambian de valor al releer un fichero.

## 8. Trampas y errores comunes

1. **Usar `to_ne_bytes` "porque funciona en mi máquina"**: el modo de fallo
   llega tarde y en otra arquitectura. Detectarlo: el código no menciona LE
   ni BE en ningún punto; la regla es «siempre hay una decisión explícita de
   endianness».
2. **Confundir bytes (los que codificas) con la representación en memoria**:
   un `f64` de 8 bytes no es «8 números»: es 8 bytes en orden LE de un IEEE 754.
   Mételo en un volcado hex para verlo.
3. **Olvidar el length-prefix y usar un delimitador**: el `\0` de C es la
   trampa clásica. Si el payload puede contener el delimitador (un string
   UTF-8 puede), tienes escape o corrupción.
4. **`as u32` demasiado a la ligera (cast)**: convertir un `usize` aunque sea
   desbordando, o indexar el slice sin comprobar `bytes.len() < 4+len`.
5. **Glosario / precisión**: *endianness* (orden en que se escriben los
   bytes de un entero: LE = menos significativo primero) vs *byte order mark
   (BOM)* vs *network byte order* (big-endian); *length-prefix* vs
   *delimiter/end-marker*; *tag* (el discriminante de variante) vs *payload*
   (los bytes del dato); *encode* (valor → bytes) vs *serialize* (que en el
   libro se usa como sinónimo coloquial); *reversible* (decode sin pérdida)
   vs *roundtrip* (el test que lo verifica); *determinismo* (mismo valor →
   mismos bytes) vs *estabilidad*.

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: dado el enum `Value` del cap. 7, escribe
  A MANO (papel o en un comentario) los bytes exactos que produce
  `encode_value` para `Value::Int(-1)`, `Value::Bool(true)` y
  `Value::String("ab")` — sin ejecutar. Pistas: (1) ¿qué tag lleva cada
  variante?; (2) ¿el `-1` de i64 en LE empieza por qué byte? (pista: menos
  significativo); (3) ¿el string lleva su longitud en cuántos bytes y delante
  de qué? Criterio: secuencia HEX correcta para los tres → verificar con
  `cargo test -p vol2-liradb cap09` (escribiendo los valores en el test).
- **analizar (intermedio — retrieval + spacing caps. 7 y 11)**: en un
  `decode_value` que recibe el buffer `[2, 0xFF, 0xFF, ...]`, ¿cómo sabes que
  el tag es `Int` y que hay EXACTAMENTE 8 bytes de payload? Explica por qué
  el length-prefix es aquí fijo (8) y en el string es variable (u32), y por
  qué el prefijo evita el problema del `\0` del cap. 11 §11.6. Pistas: (1)
  mira `decode_value` para el tag 2; (2) `Int` y `Float` pesan igual pero el
  tag las distingue; (3) el delimitador falla si el dato lo contiene. Criterio:
  razonamiento completo sobre tag + longitud fija/variable.
- **crear (experto — format versioning)**: añade una variante `Value::U64(u64)`
  con tag 6, y BUMPA `FORMAT_VERSION` a 2. Implementa encode/decode y un test
  que demuestre: (a) un `Value::U64(7)` se codifica y decodifica; (b)
  `decode_header` de un fichero v1 sigue devolviendo 1 (los ficheros viejos
  se leen); (c) un fichero escrito con versión 2 no se confunde con uno v1
  (el magic y el versionado lo protegen). Pistas: (1) copia el patrón del tag
  2; (2) `decode_value` con tag 6; (3) explica en un comentario por qué bump
  de versión protege los viejos. Criterio: los tests compilan contra
  `cap09_encoding.rs` y el razonamiento de versionado está escrito.

## 10. Preguntas abiertas (gancho al cap. 10 — append-only)

1. Ahora un `Value` se convierte en una secuencia de bytes reversible… ¿y si
   ese flujo se va ESCRIBIENDO una detrás de otra en un fichero, sin borrar
   nunca? (Ese es el cap. 10, append-only: el `Vec<u8>` de aquí se vuelve un
   log de bytes en disco.)
2. Un mismo `Value` produce siempre los mismos bytes (determinismo)… ¿cómo
   convierto esa propiedad en una garantía de que el fichero no se corrompió
   en el camino? (CRC — cap. 10.)
3. Si el versionado del formato protege los ficheros viejos… ¿cómo se detecta
   y corrige una CRECIENTE lista de versiones sin que el lector explote?
   (Nace el versionado maduro que el cap. 28, WAL, exige.)
- **Términos nuevos de glosario**: encoding/encode/decode, endianness
  (little/big), byte order, length-prefix, payload, tag (discriminante),
  format versioning, magic number/string, roundtrip, determinismo,
  to_le_bytes / to_ne_bytes.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial escribe los bytes DE MEMORIA de tres
  `Value`s (nada del enunciado entrega los tags ni el orden LE); el
  intermedio reconstruye el razonamiento tag+longitud sin mirar el código
  completo.
- **Spacing**: cap. 7 (el `Value` que aquí se codifica — ejercicio esencial
  lo repasa), cap. 8 (el `GraphStore` que «persiste», ahora sí tiene bytes),
  cap. 11 (length-prefix retomado); el ejercicio intermedio conecta
  explícitamente §11.6 con este capítulo.
- **Interleaving**: el intermedio mezcla endianness (9) con length-prefix
  (11) y el modelo `Value` (7); el experto mezcla versionado (9) con la
  promesa de WAL (28, nombrado).
- **Dificultad asimétrica**: una idea nueva por sección (endianness →
  primitivas → length-prefix → tag → versionado → determinismo); los
  ejercicios exigen escribir bytes de memoria y razonar sobre tags.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap09`
  verifica los roundtrips; los ejercicios del lector compilan contra el mismo
  `cap09_encoding.rs`.
- **Citas**: Intel/x86 little-endian y la historia de la endianness;
  prof. de redes big-endian (network byte order, RFC 1700); IEEE 754 (estándar
  de floats, 1985, revisado 2008/2019); SQLite file format spec (magic string
  "SQLite format 3\0" + version bytes); PostgreSQL TOAST y la política de
  `pg_type` (cómo PostgreSQL discrimina tipos y versiona); el paper clásico
  sobre diseño de formatos de fichero de los 90 (reglas del layout de
  estructuras de datos en disco).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (8 en la tabla §5).
- [x] Escenario de fallo visible: `to_ne_bytes` que corrompe en otra endianness; delimitador que se rompe con `\0`; tag ignorado que monta la variante equivocada.
- [x] Código ejecutable en workspace (`cap09_encoding.rs`, tests ALL GREEN citados por nombre y línea), referenciado sin duplicarlo.
- [x] Misconcepciones corregidas explícitamente (§1: cuatro; «string ya es bytes», «funciona en mi máquina», «el tag se deduce», «mi formato es para siempre»).
- [x] Ejercicios con solución verificable (tests del workspace + bytes a mano).
- [x] ≥1 ejercicio de retrieval (escribir bytes de memoria) y ≥1 de spacing (caps. 7 y 11 tocados en el intermedio).
- [x] Responde las preguntas críticas del CORPUS id `vol-II-cap-09` («¿Por qué little-endian como default en x86 pero big-endian en redes?», «Format versioning: cómo añadir campos sin romper bases existentes»).
- [x] Anécdota verificada con fuente (endianness/formatos de fichero) y enlaces históricos reales (Intel x86, network byte order, IEEE 754, SQLite format spec, PostgreSQL).
- [x] Ejercicio de retrieval + spacing al cap. 7 (Value como dato).
- [x] Mini-diálogo final; gancho al cap. 10 (append-only).
