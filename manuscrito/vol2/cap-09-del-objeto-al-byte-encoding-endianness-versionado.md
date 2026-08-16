# Capítulo 9 — Del objeto al byte (encoding, endianness, versionado)

> *«Un fichero no guarda nodos ni aristas. Guarda bytes. Si no decides tú cómo se traducen los unos en los otros, lo decidirá el azar.»*

## 9.0 La anécdota de la esquina

Todo el debate del byte venía de una novela de 1726. En «Los viajes de Gulliver», Jonathan Swift describe la guerra eterna entre los Liliputienses que rompían el huevo por el lado *grande* (*Big-Endians*) y los que lo rompían por el lado *pequeño* (*Little-Endians*). Doscientos cincuenta años después, cuando los primeros estándares de red tuvieron que fijar el orden de los bytes entre computadoras que hablaban idiomas distintos, los ingenieros recuperaron los nombres de Swift para algo que NO era una metáfora.

En los años 60 y 70, cada fabricante escribía los números en disco como le parecía: DEC y las máquinas de Intel-heredado en *little-endian* (el byte menos significativo primero), IBM y Motorola en *big-endian*. La diferencia no era académica. Un fichero binario escrito en un IBM y abierto en un DEC se leía **distinto, sin ningún error**: el `u16` `0x0A0B` viajaba como los bytes `0A 0B` en un lado y `0B 0A` en el otro, y el número aparecía falseado. Por eso los protocolos de Internet acabaron fijando un *network byte order* (big-endian) como idioma común entre máquinas: cuando dos sistemas no comparten convención, alguien tiene que traducir a una única que todos conozcan.

Y con los formatos de fichero pasó algo parecido, medio siglo después. En los años 90, el RFC 1925 de Ross Callon titulado *«The Twelve Networking Truths»* —y con él la cultura que popularizó **Peter Deutsch** en sus buenas prácticas de estado persistente— resumió la lección que nos importa hoy: un formato de fichero **es una promesa con una versión estampada**. No basta con que tú lo leas hoy: debe sobrevivir a tu máquina, a tu equipo y a tu esquema de datos. La primera red de defensa para lograrlo es ponerle una firma y un número de edición arriba del todo.

Ese es el tema de este capítulo. Quizá no haya vuelos espaciales ni batallas de huevos; pero cuando acabes, entenderás que **un `Value` solo existe en disco si decides, byte a byte, cómo se traduce —y pones la versión para que esa traducción no se rompa mañana**.

## 9.1 Objetivo

Al terminar este capítulo podrás llevar las estructuras del **cap. 7** (`Value`, y por extensión `Node` y `Edge`) a una secuencia de bytes **reversible**: codificar y decodificar a mano, sin ninguna crate, los seis tipos de `Value` de forma que un mismo valor produzca siempre los mismos bytes.

En concreto, construirás cuatro piezas (todas en `cap09_encoding.rs`):

1. Primitivas explícitas: `encode_u32_le`, `encode_i64_le`, `encode_f64_le` (y sus `decode_*`).
2. `encode_string`/`decode_string` — strings con length-prefix.
3. `encode_value`/`decode_value` — el enum `<Data>` del cap. 7 completo.
4. `encode_header`/`decode_header` — el magic + `FORMAT_VERSION` que firma un fichero.

## 9.2 Problema

Ya tienes un `Value::String("Ana")` en memoria. Pregunta trampa: **¿qué es exactamente lo que guardas en disco?**

- Un `String` de Rust ES UTF-8, es decir, ya son bytes. «Perfecto», dices, «escribo los 3 bytes `A`, `n`, `a`». Pero cuando lo leas: **¿dónde acaba?** Si el siguiente `Value` empieza justo después, no tienes manera de saber cuántos bytes eran suyos.
- Un `i64` en memoria son 8 bytes. Pero su **orden** depende de la máquina: en tu x86 es little-endian; en un ARM o un MIPS de otra generación podría ser big-endian. Escribir «los 8 bytes del número» sin decir en qué orden es escribir **sin contrato**.
- Un `f64` (el `Float` de IEEE 754) son 8 bytes, pero interpretados según el estándar de coma flotante — y ahí no todo es recuperable igual de fácil (NaN, precisión).
- Y un `enum` `Value` puede ser cualquiera de seis variantes. Sin una **etiqueta** al principio, el que lee no sabe si lo que sigue es un `Int`, un `Float` o un `String`.

El problema central, en una frase: **mezclamos el objeto (con sus tipos) con el flujo de bytes (sin tipos)**. La máquina en memoria conoce el tipo de cada variable; el fichero no. Alguien tiene que traducir, y esa traducción tiene que ser **decidida y documentada**, no dejada al azar del compilador.

## 9.3 Modelo mental

Piensa en **una carta que debe cruzar el mundo y ser leída dentro de veinte años, por un archivador que no te conoce**.

- **El sobre** lleva la firma y el número de edición: `[magic 0x4C444231][FORMAT_VERSION=1]`. El archivador, antes de abrir, comprueba que el sobre es TUYO y que edición es. Si no reconoce la firma, no intenta leer: lo dice. Eso es `encode_header`/`decode_header`.
- **El orden de lectura** está pactado: `little-endian`, el primer byte es el **menos significativo**. Ambos lados (el que escribe y el que relee, quizá una máquina distinta) siguen el mismo orden. Sin ese pacto, la carta es galimatías: los dos bytes `0A 0B` escritos en un IBM (big-endian) y abiertos en un DEC (little-endian) se leen como `0B0A` — el número que pretendías, falseado en silencio.
- **Los campos de longitud variable** llevan su longitud escrita DELANTE: `[4][h][o][l][a]`. El archivador sabe que el texto ocupa exactamente 4 bytes, aunque el texto contenga un carácter raro (incluso un carácter «de fin de línea», del que en una carta normal esperarías que terminara — en nuestro formato no existe el fin de línea reservado).
- **Los datos polimórficos** llevan una etiqueta al principio: `[3][8 bytes del float]`. El archivador mira el 3 y sabe «esto es un Float, léeme 8 bytes».

Los diagramas de los dos casos que más te importan:

```
Value::Int(-1)  →  tag=2  [FF FF FF FF FF FF FF FF]   (i64, little-endian: -1 = todos los bits a 1)
Value::String("hola")  →  tag=4  [04 00 00 00] [h][o][l][a]   (u32 len + 4 bytes UTF-8)
Value::Float(PI)  →  tag=3  [los 8 bytes LE del IEEE 754 de PI]

Fichero:
[4C 44 42 31] [01 00 00 00]  ...   ← magic "LDB1"  +  FORMAT_VERSION = 1
```

El momento «¡ajá!»: **`encode` no está «convirtiendo a binario»: está REDACTANDO una carta con reglas que un desconocido debe poder releer byte a byte, sin darte la mano.** Si el byte 1 no significa lo mismo para el que escribe y el que lee, el dato no sobrevivió el viaje — aunque ambos creyeran haberlo guardado.

## 9.4 Primera solución

La versión ingenua, la que escribe cualquiera al principio:

```rust
// Solución ingenua: guardar el Value "a secas".
fn guardar(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }
fn guardar_num(n: i64) -> [u8; 8] { n.to_ne_bytes() }   // ← ojo con esto
```

En tu portátil los tests pasan. `"Ana"` se guarda como `[41 6E 61]`, y `guardar_num(-1)` escribe sus 8 bytes «a lo nativo». Incluso podrías caer en la tentación de serializar con `format!("{:?}", v)` o `v.to_string()`: legible, cómodo… y ambiguo. Al releer `Value::Float(3.14).to_string()` — `"3.14"` — ¿de dónde sacas los 8 bytes binarios del IEEE 754? Un `f64` y su representación decimal legible NO son lo mismo.

## 9.5 Sus límites

La solución ingenua se rompe —en silencio— en cuanto dejas de mirar:

1. **El string no delimita su final.** `guardar("hola")` devuelve `[68 6F 6C 61]`. Sin longitud escrita, `decode` no sabe si el `Value` termina en `a` o si hay tres bytes más escondidos después. Y si introduces un delimitador (`\0`, el «fin de línea» de C), el problema vuelve por la puerta de atrás: ¿y si tu texto contiene un `\0`? (Veremos en §9.6 por qué el length-prefix gana siempre.)
2. **`to_ne_bytes` es una bomba de relojería.** Funciona en el x86 que lo escribió. El fichero viaja; se abre en una máquina big-endian; y ahora `Int(-1234567890)` se lee como un número gigante y absurdo, sin ningún error. Corrupción total y silenciosa. Este es exactamente el modo de fallo que ataca el cap. 11 cuando dice «nunca `to_ne_bytes`».
3. **El tag del enum no se deduce.** Un `Vec<u8>` de `Value::Int(42)` y uno de `Value::Float(42.0)` pesan lo mismo (8 bytes) pero significan cosas distintas. Si no escribes la etiqueta, `decode` adivina — y a veces acierta. Eso es lo peor: «casi-bien» es indetectable en un debug rápido.
4. **El fichero es para siempre… hasta que ya no.** Añades una variante nueva a `Value` (verás esta idea en el cap. 7) y `decode` se encuentra un byte que antes no existía. A menos que haya un `FORMAT_VERSION`, no tienes la menor idea de si el fichero que estás leyendo se escribió con la versión nueva o la vieja.

## 9.6 Solución evolucionada

La solución de verdad tiene **cinco reglas** — las mismas cinco desde los 90, y las que usan SQLite, PostgreSQL y cualquier motor serio:

1. **Endianness explícita** (little-endian, LE). `encode_u32_le` = `value.to_le_bytes()`; el primer byte es el menos significativo. Del otro lado, `u32::from_le_bytes` recompone el número. Por qué LE: porque x86 (la máquina de casi todo el mundo, y la de LiraDB) es little-endian desde los 8086, y porque es **reversible byte a byte**: el primer byte del stream es, siempre, el byte de menor peso, así que «recorrer bytes en orden» es «recorrer dígitos de menor a mayor», sin saltos. (Las redes históricamente prefirieron big-endian — el *network byte order* — por herencia de los primeros prototipos; nosotros somos un fichero de disco, la endianness de nuestra arquitectura es LE.)

2. **Length-prefix, no delimitador.** `encode_string` escribe `u32 longitud` seguido de los bytes UTF-8. `decode_string` lee los 4 primeros bytes (la longitud) y luego exactamente `len` bytes de payload. No hay byte prohibido: un string puede contener `\0`, `"`, `0x00`, lo que sea, porque sabemos su longitud EXACTA de antemano. Un delimitador obligaría a escapar esos bytes, y cada escape es una oportunidad de corrupción silenciosa. (Es la misma batalla del cap. 11 §11.6, pero aquí desde el ángulo del *encoding de un valor*: el `\0` no existe como término de línea en nuestro flujo.)

3. **Tag por variante.** Antes de cada `Value` va un `u8` (0..5) que dice qué variante es. El decodificador lee el tag, y con él sabe CUÁNTOS bytes leer y cómo interpretarlos.

```rust
match tag {
    0 => (Value::Null, resto),
    1 => ... Bool (un byte, 0 o 1) ...
    2 => ... Int (8 bytes LE) ...
    3 => ... Float (8 bytes LE IEEE 754) ...
    4 => ... String (u32 len + payload) ...
    5 => ... Bytes (u32 len + payload) ...
}
```

4. **Format versioning.** `encode_header` escribe `[magic 0x4C444231]` («LDB1» en ASCII) + `[FORMAT_VERSION]`. `decode_header` **comprueba** el magic antes de devolver nada: si no lo reconoce, `Err("magic mismatch")`. El día que el formato cambie (p.ej. añadas una variante), subes `FORMAT_VERSION` y el lector sabe con qué reglas leer. Un fichero viejo no se rompe: se detecta.

5. **Fallos ruidosos y resto.** Cada `decode_*` devuelve `Result`, y **nunca** entrega datos a medias como válidos: `"too short"`, `"payload truncated"`, `"tag {n}"` son errores reales. Y cada decode devuelve el resto del buffer (`&[u8]`): así los `Value`s pueden encadenarse uno tras otro en un mismo flujo — exactamente lo que necesitará una página (cap. 11).

### El `Float` y el IEEE 754

El `Float(f64)` no es especial para nosotros (lo codificamos con `encode_f64_le`, 8 bytes LE igual que el `Int`), pero merece su propio aviso: esos 8 bytes se interpretan bajo el **estándar IEEE 754** (1 bit de signo, 11 de exponente, 52 de mantisa). Tres consecuencias que verás en el capítulo:

- **La precisión es finita.** `0.1 + 0.2` no es `0.3`; es `0.30000000000000004`. Codificar y recodificar no lo arregla: es intrínseco al formato binario del estándar. Por eso comparar floats con `==` exacto en el roundtrip es una trampa (salvo por los bytes, que sí son deterministas).
- **Los NaN no son iguales a sí mismos.** El IEEE 754 define que `NaN != NaN`. Si guardas un `NaN`, el roundtrip «funciona» (los bytes vuelven), pero `assert_eq!(dec, v)` fallará si comparas los `Value` directamente y uno contiene NaN. Nuestro test aparta el caso a propósito: comparamos el `Float(PI)` exacto, no un NaN.
- **Reversibilidad byte a byte sí existe**, en tanto que los 8 bytes LE de un float dado son siempre los mismos. Eso es lo que el determinismo pide, y es la base de los CRC del cap. 10.

## 9.7 Código completo ejecutable

El código está en `liradb-workspace/crates/vol2-liradb/src/cap09_encoding.rs`. Lo leemos por partes, porque cada línea tiene un porqué.

### Primitivas

```rust
pub const FORMAT_VERSION: u32 = 1;

pub fn encode_u32_le(value: u32) -> [u8; 4] { value.to_le_bytes() }
pub fn decode_u32_le(bytes: &[u8; 4]) -> u32 { u32::from_le_bytes(*bytes) }

pub fn encode_i64_le(value: i64) -> [u8; 8] { value.to_le_bytes() }
pub fn decode_i64_le(bytes: &[u8; 8]) -> i64 { i64::from_le_bytes(*bytes) }

pub fn encode_f64_le(value: f64) -> [u8; 8] { value.to_le_bytes() }
pub fn decode_f64_le(bytes: &[u8; 8]) -> f64 { f64::from_le_bytes(*bytes) }
```

Tres primitivas idénticas en forma, distinta semántica. `to_le_bytes()` y `from_le_bytes()` son las únicas vías: **nunca** `to_ne_bytes()`. El tipo de retorno (`[u8; 4]` vs `[u8; 8]`) fija de cuántos bytes consta cada número, y el par encode/decode es exactamente reverso.

### Strings con length-prefix

```rust
pub fn encode_string(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + s.len());
    out.extend_from_slice(&encode_u32_le(s.len() as u32));
    out.extend_from_slice(s.as_bytes());
    out
}

pub fn decode_string(bytes: &[u8]) -> Result<(String, &[u8]), String> {
    if bytes.len() < 4 {
        return Err("string: too short".into());          // sin ni siquiera la longitud
    }
    let mut lb = [0u8; 4];
    lb.copy_from_slice(&bytes[..4]);
    let len = decode_u32_le(&lb) as usize;
    if bytes.len() < 4 + len {
        return Err("string: payload truncated".into());   // la longitud promete más de lo que hay
    }
    let s = std::str::from_utf8(&bytes[4..4 + len])
        .map_err(|e| e.to_string())?
        .to_string();
    Ok((s, &bytes[4 + len..]))                             // devolvemos el resto del buffer
}
```

Fíjate en las **dos comprobaciones defensivas** antes de tocar el slice: `bytes.len() < 4` (aunque no haya siquiera campo de longitud) y `bytes.len() < 4 + len` (la longitud promete más bytes de los presentes — `payload truncated`). Nunca indexamos `&bytes[4..4+len]` sin saber que existe. Y el `from_utf8` es otra capa: si el payload no es UTF-8 válido, error, no string basura.

### El enum `Value` (cap. 7) completo

```rust
pub fn encode_value(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    match v {
        Value::Null   => out.push(0),
        Value::Bool(b) => { out.push(1); out.push(u8::from(*b)); }
        Value::Int(i) => { out.push(2); out.extend_from_slice(&encode_i64_le(*i)); }
        Value::Float(f) => { out.push(3); out.extend_from_slice(&encode_f64_le(*f)); }
        Value::String(s) => { out.push(4); out.extend_from_slice(&encode_string(s)); }
        Value::Bytes(b) => { out.push(5); out.extend_from_slice(&encode_u32_le(b.len() as u32)); out.extend_from_slice(b); }
    }
    out
}
```

La belleza está en la simetría. El tag `u8` va primero; `Null` no lleva más nada; `Bool` un byte; `Int`/`Float` ocho bytes LE; `String`/`Bytes` un `u32 len` + payload. El `decode_value` es el espejo exacto.

### La cabeza del fichero

```rust
pub fn encode_header() -> [u8; 8] {
    let mut out = [0u8; 8];
    out[..4].copy_from_slice(&encode_u32_le(0x4C_44_42_31));  // magic "LDB1"
    out[4..].copy_from_slice(&encode_u32_le(FORMAT_VERSION));
    out
}

pub fn decode_header(bytes: &[u8; 8]) -> Result<u32, String> {
    let magic = decode_u32_le(bytes[..4].try_into().unwrap());
    if magic != 0x4C_44_42_31 {
        return Err(format!("magic mismatch: got {magic:#x}"));
    }
    Ok(decode_u32_le(bytes[4..].try_into().unwrap()))
}
```

El magic `0x4C444231` es «LDB1» en ASCII: unos bytes con todos los bits alternados a propósito, improbables de aparecer por casualidad (recuerda el «magic raro» del cap. 11). `decode_header` **exige** el magic; si falla, error inmediato. Ese `if magic != ...` es tu red contra «abrí un fichero de otra versión y lo leí como si fuera mío».

## 9.8 Prueba de fuego

La prueba de fuego es el **roundtrip**: codificar y decodificar debe devolver exactamente lo mismo, y el buffer debe quedar vacío (`rest.is_empty()`), una prueba de que el límite del `Value` se respetó a la perfección:

```rust
for v in [
    Value::Null, Value::Bool(true), Value::Bool(false),
    Value::Int(42), Value::Int(-1234567890),
    Value::Float(std::f64::consts::PI),
    Value::String("hola, mundo!".into()),
    Value::Bytes(vec![1, 2, 3, 4, 5]),
] {
    let enc = encode_value(&v);
    let (dec, rest) = decode_value(&enc).unwrap();
    assert_eq!(dec, v);        // idénticos
    assert!(rest.is_empty());  // el Value ocupó TODO el buffer, ni uno de sobra ni falta
}
```

Y los casos de fallo son tan importantes como el camino feliz: `string: too short`, `payload truncated`, `float: need 8 bytes`, `value: tag {other}`. **Si se te olvidara este capítulo, el síntoma lo notarías en el siguiente**: al meter un `Value` en una página (cap. 11) no sabrías dónde termina cada uno, y los strings se leerían recortados o con basura pinchada, o los números cambiarían de valor al releer el fichero.

## 9.9 Qué hemos sacrificado

Toda solución paga un precio. Nuestro encoding no es gratis; esto es lo que cedes a cambio de la reversibilidad:

1. **4-5 bytes de sobrecarga por `Value`** (el tag + el length-prefix). Para valores diminutos el overhead es proporcionalmente alto. Es el precio de tener un formato autodescriptivo.
2. **Sin referencias ni aliasing**: codificar duplica los datos en memoria. No «apuntamos» a un node ya guardado: lo reinterpretamos a bytes cada vez.
3. **Sin compresión**: un string se guarda como está. La compresión está fuera de alcance (será parte de la optimización, nunca del capítulo 9).
4. **Texto UTF-8, no otro encoding**: asumimos `String` UTF-8 de Rust. Otros encodings son una decisión futura, no una obligación.
5. **Un `u32` de longitud sacrifica algo de compatibilidad** para strings mayores de 4 GB: teóricamente un `usize` de 64 bits cabría más, pero 4 GB por valor es más que suficiente para el modelo de datos y mantiene el prefijo en 4 bytes.

## 9.10 Cómo lo hace una BBDD real

Los patrones que acabas de construir no son invención nuestra; son la base de todo el almacenamiento de datos:

- **SQLite** abre sus ficheros con un **magic string** —exactamente *"SQLite format 3\0"* al comienzo— seguido de campos de versión. Es la misma idea que `decode_header`: una firma que `SQLite` comprueba antes de interpretar cualquier byte. Su [file format](https://www.sqlite.org/fileformat.html) documenta byte a byte (encabezados, lengths varint con prefijo de longitud, etc.) lo que aquí hacemos a mano.
- **PostgreSQL** usa length-prefixes y encabeza todo con una cabecera de formato; su modelo de **TOAST** y el catálogo `pg_type` son la demostración del mismo principio a escala de tipos: un mecanismo que dice «este tipo es tal, esta versión vale esto» antes de interpretar. El formato de fila y el encoding son explícitos y versionados.
- **IEEE 754** gobierna el `Float` como nosotros lo codificamos; es el estándar que todas las CPUs implementan. Su historia (1985, revisado en 2008 y 2019) explica por qué un `f64` son 8 bytes y qué pasa con NaN y la precisión.
- **El paper de Deutsch y las buenas prácticas de los 90** (formato de fichero estable pero versionado) siguen siendo, tres décadas después, el contrato de todo motor: *«design for change; put a version on it»*.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿cuántos bytes ocupa `Value::Bytes(vec![0u8; 100])` en total (tag + length + payload)? Escribe la cuenta a mano.
- *Intermedio*: `encode_string` usa `u32::from(s.len() as u32)`. ¿Qué pasa si el string mide más de 4 GB? ¿Es un peligro real en el modelo de datos? ¿Qué alternativa tendrías?
- *Experto*: diseña un length-prefix **varint** (como el de SQLite) donde `0-127` quepan en UN byte y valores grandes usen varios. Compara el espacio total para strings de 3, 300 y 30.000 bytes.

## 9.11 Lo que te llevas

- **La endianness es un contrato, no un detalle**: little-endian explícito (`to_le_bytes`), jamás `to_ne_bytes`, para que el formato sea portable e independiente de la máquina.
- **Length-prefix > delimitador**: escribir la longitud ANTES del dato permite cualquier byte en el payload y evita la corrupción silenciosa por escapes.
- **Un tag por variante**: el `Value` del cap. 7 se codifica con `[tag][payload]` y el decodificador sabe exactamente cuántos bytes leer.
- **Magic + `FORMAT_VERSION`**: se firma el fichero; si no es nuestro o es otra versión, se DETECTA antes de interpretar.
- **Determinismo**: un mismo `Value` produce siempre los mismos bytes — el requisito previo de cualquier checksum (cap. 10).
- **Fallos ruidosos y resto explícito**: decode devuelve `Result` y el resto del buffer, para encadenar valores en un solo flujo.

## 9.12 Ojo, cuidado con…

- **`to_ne_bytes`**: la tentación de «lo nativo». Funciona en tu máquina y corrompe en todas las demás. Regla: *si el código no decide LE o BE en un punto visible, es sospechoso.*
- **Usar un delimitador para strings** (el `\0` o un `0xFF`): el dato puede contenerlo; entonces hay que escaparlo, y un escape mal hecho es corrupción.
- **Confundir bytes con números**: `decode_f64_le` devuelve un `f64` cuya precisión es finita (IEEE 754). Comparar floats con `==` exacto es una trampa; los NaN ni siquiera son iguales a sí mismos.
- **Indexar el slice sin comprobar la longitud**: el `payload truncated` existe; nunca hagas `&bytes[4..4+len]` sin haber verificado que el buffer lo contiene.
- **Creer que el fichero es para siempre**: un formato sin versión es un fichero que, el día que cambies el esquema, se leerá «casi-bien» (peor que mal). `FORMAT_VERSION` es tu válvula.

## 9.13 Pin de batalla

> *«Un fichero que se lee distinto según la máquina que lo abrió no es una base de datos: es un accidente que aún no ha ocurrido.»*

## 9.14 Si solo lees 30 segundos

Para que un `Value` sobreviva en disco necesitas decidir CUATRO cosas: **el orden de los bytes** (little-endian, `to_le_bytes`), **dónde termina cada dato de longitud variable** (length-prefix: la longitud DELANTE), **qué variante es** (un tag `u8` al principio), y **qué versión de formato es** (magic + `FORMAT_VERSION`). Con eso, `encode_value` y `decode_value` son reversos exactos: un mismo valor produce siempre los mismos bytes. Esa frase —«siempre los mismos bytes»— es lo que permite que mañana un checksum (cap. 10) pueda decir «esto no se ha corrompido».

## 9.15 Una historia pequeña

Corría el primer intento de LiraDB de guardar un grafo en un fichero. «El encoding es obvio», dijimos, y escribimos cada propiedad con `format!("{:?}", v)` — legible, simple. El grafo se guardaba y releía… hasta que Ana añadió a un nodo una bio de 2 KB con un salto de línea y dos caracteres raros. Al releer, el nodo siguiente aparecía al revés y un `Float(3.14)` se transformó en los bytes de su patrón de bits, leídos como si fueran un `Int` gigante y absurdo. No había error: solo datos ligeramente incorrectos, exactamente el peor tipo de fallo. Reconstruir ese nodo nos llevó una tarde. El culpable no era la bio: era que **no habíamos decidido dónde termina cada valor y en qué orden se escriben sus bytes**. El encoding binario del capítulo 9 fue la respuesta. Hoy, Ana guarda una bio de 2 KB y el roundtrip queda intacto, byte a byte.

---

## Ejercicios resueltos

**1. ¿Cuántos bytes ocupa en total `Value::Int(42)` codificado por `encode_value`?**

`encode_value` empieza con `out.push(2)` (un byte, el tag). Luego `out.extend_from_slice(&encode_i64_le(42))`, que aporta 8 bytes. Total: **9 bytes**. Una sola prueba mental con el test `value_roundtrip`: al decodificar, `decode_value` gasta 1 byte leyendo el tag, 8 bytes del payload, y `rest.is_empty()` confirma que no sobraba nada.

**2. ¿Por qué `decode_value` puede confiar en el tag para saber cuántos bytes leer?**

Porque `encode_value` escribió el tag primero y, según el tag, el payload tiene un número de bytes CONOCIDO: 0 (Null), 1 (Bool), 8 (Int/Float), o `u32 len + len` (String/Bytes). El decodificador no adivina nada: el formato lo *declara*. Y si el buffer no contiene los bytes prometidos, `decode_*` devuelve `Err("payload truncated")` en lugar de entregar un `Value` incompleto como si fuera válido. Esa **validación defensiva** —el formato lleva su propio contrato y lo comprueba— es la misma filosofía del cap. 11.

## Ejercicios propuestos

**Esencial.** En papel (o en un comentario), escribe a mano la secuencia HEX exacta que produce `encode_value` para los tres `Value`s: `Int(-1)`, `Bool(true)` y `String("ab")`. No ejecutes hasta haberlo escrito. Verifica luego contra `cargo test -p vol2-liradb cap09`.

**Intermedio.** Explica, para `decode_value` recibiendo `[2, 0xFF, 0xFF, ...]`: (a) cómo sabes que es el tag 2 (Int); (b) cuántos bytes de payload lees y por qué ESOS y no otros; (c) por qué el length-prefix del string es variable (u32) mientras el del Int/float es fijo (8). Pista: mira `decode_value` para los tags 2 y 4. Este ejercicio conecta con el cap. 11 §11.6: mismas reglas de length-prefix, distinto ángulo.

**Experto.** Añade la variante `Value::U64(u64)` con tag 6 a `cap07_modelo::Value`, implementa su encode/decode en `encode_value`/`decode_value`, y sube `FORMAT_VERSION` a 2. Escribe un test que: (a) `Value::U64(7)` hace roundtrip; (b) `decode_header` de un fichero v1 sigue devolviendo `1` (los ficheros viejos se siguen leyendo); (c) un fichero v2 no se confunde con uno v1. Explica por qué ese bump de versión protege a las bases existentes.

## Para profundizar

- **"Database Internals" (Alex Petrov)** — capítulos sobre encoding de formatos de página y registros: la misma mecánica de length-prefix y endianness llevada a producción.
- **The SQLite File Format** — la [documentación oficial](https://www.sqlite.org/fileformat.html) detalla byte a byte un motor real: magic string, versión, varints. Compara con tu `decode_header`.
- **IEEE 754-2019** — el estándar de coma flotante; por qué un `f64` son 8 bytes y qué significan NaN/infinitos/precisión.
- **El paper de Peter Deutsch, «PERSISTENT STATE (Internet Best Practices)»** — las «cosas que ya no hay que hacer» en diseño de formatos de fichero, incluida la endianness y el versionado.
- **PostgreSQL: TOAST y `pg_type`** — cómo un motor maduro versiona y discrimina sus tipos de dato ante de interpretarlos en disco.

## Mini-diálogo: en guardia nocturna

> — En serio, ¿un capítulo entero para escribir cuatro bytes?
>
> — Cuatro bytes QUE NADA NI NADIE PUEDE LEER MAL. El `String`, el `Int` y el `Float` de tu `Value` no llevan un rótulo en la memoria que diga «yo soy un float». En disco, la máquina que relea no sabe qué eras: lo ha de deducir de los bytes. Cada regla que escribimos —el orden, la longitud, el tag, la versión— es una promesa: *cuando releas, vas a leer esto igual que yo lo escribí*.
>
> — ¿Y el `FORMAT_VERSION`? ¿No es paranoia?
>
> — No. Es el seguro. Es el único palito que nos impide, el día que añadamos `U64` al `Value`, que todos los ficheros viejos se corrompan en silencio. Mejor que un fichero se niegue a abrir que se abra «casi-bien».

---

*(Próximo capítulo: 10 — Persistencia append-only. Ahora un `Value` es bytes reversibles; veremos cómo esos flujos se van escribiendo uno tras otro en un fichero que jamás reescribe el pasado, y cómo tu determinismo permite que un CRC lo vigile.)*
