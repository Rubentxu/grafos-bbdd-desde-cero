# CONTRATO DE CAPÍTULO — Vol.II Cap. 10: Persistencia append-only

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap10_append_only.rs` (295 líneas.
> LibRust: módulo `cap10_append_only` cableado en `lib.rs` líns. 261/286 con
> `pub use cap10_append_only::*`). Tests en `mod tests_log` (6 tests:
> `crc32_known_value_empty`, `crc32_known_value_a`, `log_record_roundtrip`,
> `log_record_corrupto_falla`, `append_only_log_basico`, `log_recovery_desde_offset`),
> verificados con `cargo test -p vol2-liradb --lib cap10_append_only`.
> Este capítulo CIERRA la Parte II («Del objeto al byte»): repaso 6→9 + el
> log, y es el GERMEN DIRECTO del WAL del cap. 28 (el propio cap28_wal.rs
> lo declara en su banner: «la semilla era deliberada», y usa `crc32_simple`
> del cap. 10 en su línea 9). Línea 20 del ToC
> (`manuscrito/vol2/tabla-de-contenidos.md`, «10. Persistencia append-only»).
> Ganchos: cap. 27 (Parte VI, la `Operacion` que replica el shape del
> `RecordKind`) y cap. 28 (WAL, el heredero directo).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: codificar/decodificar valores del grafo a
  bytes con **little-endian explícito** (`encode_*_le` / `decode_*_le`,
  strings con length-prefix u32, el `Value` con tag + payload; cap. 9); que
  el formato en disco debe ser **independiente de la máquina** (cap. 9: nunca
  `to_ne_bytes`, siempre `to_le_bytes`); que LiraDB persiste vía el trait
  `GraphStore` como puerto (cap. 8); por qué la **estabilidad de IDs**
  (cap. 3, Vol. I) importa para sobrevivir a un crash; el modelo de datos
  Property Graph + `Value` (cap. 7); la idea de bloque/sector y de page como
  unidad de disco **aún no presentada** (cap. 11).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «para
  guardar un cambio en un fichero hay que abrirlo, modificar el byte que
  toca y volver a escribir» — en sitio; no: un log **nunca pisa** lo ya
  escrito, apéndiza un registro de cada cambio; (2) «un corte de luz es un
  problema de datos *en el medio* del fichero» — en un log append-only el
  corte deja un **prefijo íntegro** y un residuo al final que el CRC detecta
  (por eso es crash-safe justo donde el sobrescribir en sitio falla);
  (3) «se detecta la corrupción comparando los datos con algo» — no: se
  detecta con un **checksum matemático sobre los propios bytes** (CRC32),
  sin copia de referencia; (4) «una lectura corrupta y un formato roto son
  lo mismo» — no: la lectura ESTRICTA grita y aborta; la ITERACIÓN de
  recuperación **para limpia en el prefijo íntegro** — son dos conductas
  deliberadas, semilla del comportamiento del WAL caps. 28-29; (5) «un log
  guarda los datos de forma redundante, así que desperdicia» — sí, paga ese
  coste a cambio de STMLICIDAD y crash-safety; la compactación (que es el
  cap. 16 + cap. 29 truncar/checkpoint) es lo que lo vuelve sostenible.
- **NO debe saber todavía**: el gestor de páginas (`Pager`, cap. 12), el
  buffer pool (cap. 13), la slotted page (cap. 11), transacciones ACID y el
  buffer de operaciones (cap. 27), el **WAL** con LSN/tx_id/flush (cap. 28),
  la recuperación tras fallo y ARIES con UNDO/REDO/checkpoint (cap. 29), la
  concurrencia MVCC (cap. 30). Se nombran como «luego lo verás» y se corta;
  este cap. NO construye todavía un sistema transaccional: solo la pieza de
  bajo nivel (el **formato del registro**: framing + CRC) que el WAL del
  cap. 28 reutilizará **tal cual**.

## 2. Conceptos (del grafo curricular)

- `present`: **log append-only** (la PRIMERA escritura, antes que las
  páginas del cap. 11); `RecordKind` (enum `#[repr(u8)]` 1..=5: PutNode /
  PutEdge / DeleteNode / DeleteEdge / Commit — payloads conceptuales sin
  `Operacion` todavía); `LogRecord` (kind + id + payload); **framing
  length-prefix u32** (`record_len` cubre todo lo que sigue hasta el
  siguiente `record_len`); el **layout por registro**:
  `[record_len:u32 LE][kind:u8][id:u32 LE][payload_len:u32 LE][payload][crc32:u32 LE]`;
  **CRC32** (`crc32_simple`, polinomio IEEE 802.3 `0xEDB8_8320`,
  didáctico sin tabla; en producción `crc32fast`); el CRC cubre
  `kind || id || payload_len || payload` (NO el length-prefix); `AppendOnlyLog`
  (en RAM `Vec<u8>` con `append`, `truncate_to` para tests de recovery; en
  producción un `File` con `O_APPEND`); `LogIterator` que **para limpio ante
  corrupción** (devuelve los records íntegros y luego `None`, en vez de
  gritar); la semilla deliberada del shape de `Operacion` (cap. 27).
- `practice`: `encode_u32_le`/`decode_u32_le` (cap. 9) — el log entero se
  construye sobre el encoding LE del cap. 9; strong little-endian; el
  length-prefix: **prefijo de longitud > marcador de fin** (cap. 11
  profundiza; aquí ya se aplica con `record_len`).
- `consolidate`: «el formato en disco es independiente de la máquina»
  (cap. 9); «derivar, no llevar en cabeza» (el `record_len` deriva de los
  bytes reales, nunca es un campo "a ojo"); validación defensiva (comprobar
  `bytes.len() < mínimo` antes de indexar, nunca tocar fuera de rango);
  política de fallo ruidoso (decode -> `Result`, error tipado por `String`).
- `out_of_scope` (solo nombrar): slotted pages / páginas (cap. 11, la
  siguiente Parte III), transacciones y ACID (cap. 27), WAL con LSN + flush
  + redo (`cap28_wal.rs`), recuperación/ARIES/checkpoint (cap. 29), MVCC
  (cap. 30).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué **apéndizar** en vez de sobrescribir en
  sitio hace al fichero inmune a las escrituras a medias (nunca pisas lo ya
  escrito: un corte de luz deja un **prefijo íntegro** + un residuo final);
  (2) dibuja el layout exacto de un `LogRecord` en bytes (length-prefix +
  kind + id + payload_len + payload + CRC32) y dice qué cubre el CRC y qué
  NO; (3) explica por qué la lectura **estricta** (decode grita ante
  corrupción) y la **iteración de recuperación** (para en el prefijo íntegro)
  son dos conductas deliberadas con distinto contrato; (4) enuncia por qué
  `RecordKind` (1..=5, payloads conceptuales) es la **semilla deliberada**
  del shape de `Operacion` del cap. 27 y por eso el cap. 28 serializará ese
  buffer **sin reinterpretar**; (5) dice qué es el CRC32 (checksum de 32
  bits sobre los bytes, polinomio IEEE 802.3) y su papel de "el que no se
  calla".
- **Skills**: (1) codificar y decodificar un `LogRecord` a mano
  (`encode_log_record` / `decode_log_record`) comprobando a mano el layout
  byte a byte; (2) elegir/implementar una función CRC32 («el que roñoso
  detecta el bit que giró») y probar contra valores conocidos
  (`crc32("")==0`, `crc32("a")==0xE8B7BE43`); (3) usar `AppendOnlyLog` +
  `LogIterator` y simular un crash con `truncate_to`, comprobando que los
  records del prefijo íntegro se leen y la cola corrupta se descarta.
- **Wisdom**: (1) decide entre **escritura en sitio** (rápida, barata en
  disco, pero vulnerable a desgaste y a escritura a medias) y **append-only**
  (crash-safe y simple, pero paga el desgaste de disco que hay que
  compactar) según qué garantía importa en cada capa; (2) decide cuándo la
  sencillez (log plano, sin índice) vale más que el rendimiento de un
  layout sofisticado que aún no puede garantizar consistencia (el log es la
  pieza de bajo nivel; las páginas vienen en cap. 11).

## 4. Modelo mental

- **El cuaderno de bitácora del barco: nunca se borra lo escrito, se
  apéndiza cada trazo nuevo**. Cada guardia, el contramaestre añade una
  línea al final; si llega una tempestad (crash) en mitad, lo que quedó
  escrito hasta esa hora es **íntegro y en orden** — nadie borró ni pisó una
  fila anterior. El problema en el barco NO es perder la página (un
  sobrescribir en sitio), es que un golpe de mar "a medias" podría dejar una
  fila escrita *por encima* de la buena sin que se note. Con el bitácora
  append-only, una fila a medias queda **visiblemente a medias** (el CRC no
  cuadra) y las anteriores siguen siendo las verdaderas.
- **Diagramas ASCII** (uno central + dos micro): (a) escritura en sitio
  (pisas el byte) vs append-only (una flecha que crece hacia la derecha,
  nadie pisa); (b) layout del `LogRecord` mostrando qué cubre el CRC
  (`kind||id||payload_len||payload`) y qué no (el propio `record_len`);
  (c) el corte de luz: prefijo íntegro + cola corrupta que `LogIterator`
  detiene limpiamente.
- **Momento ¡ajá!**: «si no piso NUNCA lo que ya está escrito, un corte de
  luz no puede embarrarme los datos: solo me corta el final, que ya sé
  detectar. La durabilidad no se gana haciendo las escrituras más fuertes,
  sino haciéndolas **imposibles de hacer mal**».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap10_append_only.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | APENDIZAR (append-only), nunca sobrescribir en sitio | Nunca pisas lo ya escrito: un corte de luz deja un PREFIJO ÍNTEGRO + residuo al final que se detecta. Con sobrescribir, una escritura a medias pisa bytes buenos y no hay forma de saber qué era el estado anterior | Reescribir el fichero entero: O(n) por cambio y una escritura a medias lo deja todo roto; sobrescribir en sitio en un byte lógicamente atómico: físico no atómico (tornio/gap) | Corrupción silenciosa e irreparable ante un crash a mitad de escritura | `log_recovery_desde_offset` (cap10): truncar a la mitad y leer el prefijo válido; doc de `AppendOnlyLog` (líns. 116-118: «el disco sería un File con O_APPEND») |
| 2 | Framing con length-prefix u32 (`record_len`) que cubre TODO lo que sigue | El iterador sabe DÓNDE termina cada record y puede avanzar `pos += 4 + inner_len` sin escanear contenido; no hay byte prohibido en payload (a diferencia de end-marker) | Marcador de fin (end-marker/`\0`): hay que escapar cualquier dato que lo contenga → corrupción silenciosa por escapes; tabla de offsets separada: más piezas que sincronizar | No saber dónde termina y empieza el siguiente → imposible iterar sin corromper | Doc `LogRecord` (líns. 21-28) y `decode_log_record` (líns. 57-98); `log_record_roundtrip` |
| 3 | CRC32 sobre `kind || id || payload_len || payload` (NO sobre el `record_len`) | El CRC cubre el CONTENIDO del record; el `record_len` es la "frontera" que, de estar corrupto, ya hace fallar la lectura (truncated) o lo atrapa el CRC siguiente. Cubrir el prefijo también valdría, pero duplica cómputo para casi nada | Un digest barato tipo checksum XOR (1 byte): no detecta varios bytes girados ni reordenaciones; sin checksum: corrupción de bits dentro del payload pasa DESAPERCIBIDA | Corrupción silenciosa dentro de un payload («leemos basura con aspecto de dato válido») | `crc32_simple` (líns. 100-114); `log_record_corrupto_falla` (corromper el último byte → `is_err()`); tests `crc32_known_value_*` |
| 4 | CRC32 con polinomio IEEE 802.3 (`0xEDB8_8320`), implementado a mano y didáctico | El polinomio ESTÁNDAR de la industria (la mayoría de CPUs tienen instrucción CRC32-C, y `crc32fast` lo usa): interop a futuro con lógica de red/fs reales; la versión a mano es O(n) por byte pero sin dependencias — para aprender | `crc32fast` de crate ahora: rompe la política de dependencias del workspace y oculta el algoritmo; un hash criptográfico (SHA) aquí es desmesurado y lento | Que el "checksum" no sea el estándar → reimplementar el CRC del WAL del cap. 28 a mano sin referencia de interoperabilidad | Banner del módulo (líns. 100-101: «Para producción usaríamos crc32fast»); test `crc32_known_value_a` (0xE8B7BE43, valor canónico del estándar) |
| 5 | `RecordKind` replica el shape de `Operacion` (cap. 27) con payloads conceptuales | Es la SEMILLA DELIBERADA: los 4 tags de escritura (PutNode/PutEdge/DeleteNode/DeleteEdge) + Commit mapean 1-1 a las variantes de `Operacion`. El cap. 28 serializa `Operacion` al WAL «sin reinterpretar» porque la forma YA EXISTÍA | Un esquema de registro genérico sin semántica (solo bytes opacos): el cap. 28 tendría que REINVENTAR el contratto; tipos desconectados → re-costuras | No anticipar el WAL = el cap. 28 reinterpreta el formato y pierde la garantía de compatibilidad | cap28_wal.rs línea 33: «la semilla era deliberada»; línea 402: «Los tags 1-4 replican el orden del RecordKind del cap. 10»; cap27 ms §cuerpo semilla |
| 6 | `LogIterator` PARA LIMPIO ante corrupción (devuelve `None` al primer error) | La RECUPERACIÓN no debe fallar ante una cola ilegible: los records íntegros hasta el corte se entregan, después `None`. Es la conducta EXACTA del WAL del cap. 29 («leer hasta el prefijo íntegro y descartar la cola») | Que el iterador propague el error al primer signo de corrupción: un solo byte dañado abortaría TODA la recuperación aunque el 99 % esté íntegro; que cominterre como la lectura estricta: violencia de contratto | Una cola corrupta paraliza la recuperación entera en vez de rescatar el prefijo válido | `LogIterator::next` (líns. 180-195: `Err(_) => None`); `log_recovery_desde_offset` |
| 7 | La lectura ESTRICTA (`decode_log_record`) grita (`Result::Err`) en corrupción | Cuando NECESITAS un registro concreto (p.ej. inspeccionar), un CRC que no cuadra es un EVENTO que se debe saber, no silenciar | Devolver datos a medias o `Option` silencioso: validación pasiva que convierte la corrupción en "dato válido con garbar" | Código que decide sobre datos corruptos creyéndolos buenos | `decode_log_record` (líns. 57-98: tres `Err` tipados: truncated / crc mismatch / unknown kind); contraste con `LogIterator::next` |
| 8 | `record_len` minimo 17 y comprobado ANTES de indexar (`bytes.len() < 17`) | Nunca tocar fuera de rango: si el prefijo reclama más de lo que hay o hay menos del mínimo, cortes a planeras. Sin estas comprobaciones, `bytes[4..4+inner_len]` paniquea (slice fuera de rango) en vez de devolver un `Err` | Indexar confiando en que bytes.len() siempre es suficiente: `panic`/UB por out-of-bounds en un fichero truncado | Un `panic` en lugar de un error tipado en plena recuperación | `decode_log_record` (líns. 59-72, 93-95); validación defensiva del cap. 11 |
| 9 | `truncate_to(offset)` como la SIMULACIÓN de crash en RAM | El «disco» del cap. 10 es un `Vec<u8>`: truncar a un offset es el análogo de un corte de luz a mitad de escritura; permite probar la recuperación SIN un fichero de verdad | No ofrecer API de truncado → no se puede testear el crash; escribir un fichero real → necesita `File`/fs que el cap. no usa todavía | No poder demostrar la crash-safety (la promesa central) en un test | `AppendOnlyLog::truncate_to` (líns. 169-172); `log_recovery_desde_offset` |

## 6. Primera solución vs solución evolucionada

- **Ingenua** (lo que escribiría el novato): para "guardar un cambio", abrir
  el fichero, hacer `seek` a la posición del dato y `write` el byte nuevo
  **en sitio**. Rápido de pensar, y el lector no ve el problema hasta que
  simboliza un crash.
- **Qué la rompe exactamente**: (a) un corte de luz a mitad de la escritura
  deja el byte **a medio cambiar** (la escritura física no es atómica: el
  disco escribe en unidades de sector, no bytes lógicos), y como "pisaste"
  la versión buena, ya no hay forma de saber cuál era; (b) si el cambio no
  cabe en el hueco (longitud variable), hay que **desplazar TODO el caudal**
  de bytes posterior = O(n) y más oportunidades de escritura a medias;
  (c) el fichero crece pero el layout no deja rastro de orden ni de
  integridad: no puedes "rehacer" nada desde el principio.
- **Evolución visible** (el código del capítulo): en vez de pisar, `append`
  añade un registro al final con **framing** (`record_len` length-prefix) y
  **CRC32**; `LogIterator` camina por los length-prefix y entrega los
  records íntegros; `truncate_to` + test `log_recovery_desde_offset`
  demuestran que tras un corte el prefijo se lee y la cola se descarta. La
  diferencia VISIBLE: el lector ya no busca "el byte que cambiar" — busca
  "dónde cae la próxima línea de la bitácora".

## 7. Prueba de fuego

- **TEST-TESIS** `log_recovery_desde_offset`: un log con 10 `PutNode` se
  truncata a la mitad (`truncate_to(len/2)`); `iter()` devuelve **al menos un
  registro completo antes del corte** y todos los IDs leídos son válidos —
  el crash dejó un prefijo íntegro, y el iterador lo rescata.
- **TEST-INTEGRIDAD** `log_record_corrupto_falla`: corromper el **último
  byte** (el CRC) de un registro hace `decode_log_record(...).is_err()` —
  el «cubre-roñoso» CRC detecta el bit que giró.
- **TEST-ROUNDTRIP** `log_record_roundtrip`: `encode_log_record` →
  `decode_log_record` devuelve exactamente el `LogRecord` original con
  `rest.is_empty()`.
- **TEST-SANOS** `crc32_known_value_empty`/`crc32_known_value_a`
  (0 y 0xE8B7BE43) y `append_only_log_basico` (3 records, orden PutNode →
  PutEdge → Commit).
- **Síntoma si el lector se salta este capítulo**: no entiende por qué el
  WAL del cap. 28 declara que su framing (`length-prefix u32 + CRC32`) y sus
  tags 1-4 "vienen del cap. 10"; su log deja de ser crash-safe (escribe en
  sitio y no distingue el prefijo íntegro de la cola corrupta) y el cap. 29
  de recuperación le resultaría incomprensible sin la diferencia lectura
  estricta vs iteración de recuperación.

## 8. Trampas y errores comunes

1. **Tratar el log como si fuera datos "en sitio"** (mover/borrar bytes del
   medio para "compactar" a mano): rompe la invariante central. La
   compactación es UN LAVADO ESCANO (cap. 16 / cap. 29 truncar), no una
   edición in situ del log.
2. **Creer que `record_len = 4 + cuerpo` y por tanto contarlo dos veces**:
   el `record_len` es la longitud del `inner` (cosa que sigue al propio
   prefix); si la confundes con `4 + payload`, los `decode`/iter avanzan mal.
   Síntoma: registros que "salta" o error `truncated` con bytes de sobra.
3. **Calcular el CRC sobre el prefijo cuando en realidad cubre solo
   `kind||id||payload_len||payload`**: al verificar no ha de cuadrar si un
   length-prefix de un byte se corrompe *antes* de ser "visto" por el CRC
   (en realidad lo atrapa el salto fuera de línea siguiente). Mantén el
   contrato exacto del módulo.
4. **Usar `to_ne_bytes`** en vez de `encode_u32_le` para el framing: copias
   el formato del cap. 9 roto host-endian. En un portátil x86 funciona; en
   una máquina big-endian, el log entero se lee al revés.
5. **No comprobar `bytes.len() < mínimo` antes de indexar**: un fichero
   truncado a 5 bytes paniquea en `bytes[4..]` en vez de devolver un `Err`.
   La validación defensiva (≥17 bytes y `4 + inner_len`) es obligatoria.
6. **Confundir los DOS contratos de lectura**: `decode_log_record` (estricto,
   grita) vs `LogIterator` (recuperación, para limpio). Usar el estricto en
   la recuperación aborta todo con un solo byte dañado.
- **Precisión de lenguaje (glosario)**: *log/bitácora* (secuencia append-only
  de registros) vs *fichero de datos* (el layout de datos en sitio, cap. 11+);
  *apéndizar* (añadir al final) vs *sobrescribir en sitio* (pisar); *record /
  registro de log* vs *tupla/registro de datos* del cap. 11; *length-prefix /
  framing* (cuántos bytes) vs *end-marker* (marca de fin); *CRC32 / checksum*
  (digest que detecta cambios) vs *hash criptográfico* (bloquea falsificación,
  desmesurado aquí); *lectura estricta* (grita) vs *iteración de recuperación*
  (para en el prefijo íntegro); *semilla deliberada* (un shape plantado
  adelantado para anticipar una necesidad futura — el WAL del cap. 28);
  *desgaste de disco* (el coste del append-only que paga la compactación).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: codificar A MANO, sobre papel, un
  `LogRecord` `PutNode` con `id=7` y `payload=[1,2,3]` byte a byte (kind=1,
  id u32 LE, payload_len=3 u32 LE, los 3 bytes, CRC32) — re-usa el `id` LE
  del cap. 9 — y luego confirmar con `encode_log_record` que tu layout es el
  del módulo (cuántos bytes totales esperas: 4 + kind + 4 + 4 + 3 + 4 = 20).
  Pistas: (1) ¿qué escribe `encode_u32_le(3)` para payload_len?; (2) ¿hace
  el CRC sobre 16 bytes (kind+id+len+payload) o sobre más?; (3) ¿cuánto
  vale el `record_len` que abre el registro? Criterio: el layout a mano
  coincide con `encode_log_record` y `decode_log_record` (roundtrip) devuelve
  el mismo record.
- **analizar (intermedio — spacing caps. 9 + 10)**: intercalar el CRC del
  cap. 10 con un `Value::Float` del cap. 9: tomas `PI` (un `f64` LE de 8
  bytes) como payload; explica en qué bytes exactos pisa el CRC y por qué el
  `record_len` no se mete en el cómputo; luego corrompes un bit del payload
  del float y razonas qué pasa en `decode_log_record` (Err crc mismatch) vs
  qué pasa en `LogIterator` (para y no entrega esa cola). Verificación:
  tests `log_record_corrupto_falla` y `log_recovery_desde_offset`. Pistas:
  (1) ¿qué separa el payload del CRC en el layout?; (2) si corrompes el
  byte de la parte exponencial del float, ¿qué te diría el CRC?; (3) ¿cuándo
  usarías ESTRICTO y cuándo el iterador? Criterio: ubicar el CRC sobre el
  byte 16 y razonar correctamente las dos conductas.
- **crear (experto — cierre de Parte II, retrieval puro + semilla)**: sin
  mirar el código, re-construir el layout de bytes de un `LogRecord` (los 6
  campos y qué cubre el CRC) y explicar, ENLACE forzado, por qué los tags
  1-4 del `RecordKind` MApean 1-1 con las variantes de `Operacion` del
  cap. 27 y qué gano ese mapa para el cap. 28 (serializar sin reinterpretar);
  después implementar `crcer_entero(log: &AppendOnlyLog, salt: u32) -> u32`
  (CRC de toda la secuencia de bytes del log) y una política
  `truncar_sin_corromper(log, target_len)` que solo trunque a un límite de
  registro válido (nunca a mitad de un record) — con tests que prueben que
  tras llamarla `iter()` falla al primer error para en el límite correcto.
  Pistas: (1) ¿cómo saber qué offsets son límites de registro válidos
  (length-prefix que no saltan fuera)?; (2) ¿qué aporta el mapa tag→variante
  si `Operacion` aún no existe?; (3) ¿dónde del reloj de la Parte II
  convendría un checksum de todo el fichero (cap. 9: el `magic`)? Criterio:
  una función de truncado que jamás deja un `record_len` muerto + el mapa
  tag↔variante razonado a mano + tests ALL_GREEN.

## 10. Preguntas abiertas (gancho al cap. 11 — abre la Parte III)

1. "Apéndizar cada cambio" da un log crash-safe en el piso bajo… ¿pero en
   QUÉ UNIDAD se lee el fichero para no volvernos O(n)? (Nace la página de
   tamaño fijo y la slotted page: unidad de disco + registros variables
   dentro — cap. 11.)
2. Un log append-only solo dice QUÉ cambió, no dónde vive el dato en el
   fichero para encontrarlo rápido: ¿cómo se indexa? (Pager cap. 12, buffer
   pool cap. 13, CSR cap. 14, índices cap. 15.)
3. Este log anticipa el WAL: pero un WAL necesita saber QUÉ transacción
   escribió qué, y en QUÉ orden, y cuándo "hacer durable". ¿Qué le falta al
   `RecordKind` de aquí — con su `Commit` — para ser un WAL de verdad?
   (Parte VI caps. 27-29: la `Operacion` que replica este shape, el LSN/tx_id
   y el redo del cap. 28, la recuperación del 29.)
- **Términos nuevos de glosario**: log append-only / bitácora, apéndizar,
  registro de log (record), framing / length-prefix, CRC32 / polinomio IEEE
  802.3, checksum vs hash criptográfico, lectura estricta, iteración de
  recuperación / truncado, prefijo íntegro, semilla deliberada, desgaste de
  disco, O_APPEND.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial re-construye el layout en bytes a mano
  sin mirar el módulo; el experto re-construye el log completo y el mapa
  tag↔variante de `Operacion` (cap. 27, un capítulo NO escrito todavía —
  retrieval de la intención, no del código).
- **Spacing**: cap. 9 (encoding LE: el log entero usa `encode_u32_le` /
  `decode_u32_le`, y el intermedio re-pone un `Value::Float` f64 LE); cap. 3
  Vol. I (estabilidad de IDs tras crash — motivo por el que el `id` del nodo
  es u32 estable en el registro); se re-ejercita «el formato en disco es
  independiente de la máquina».
- **Interleaving**: el intermedio mezcla el CRC del cap. 10 con un float f64
  LE del cap. 9 (dos capítulos de la misma Parte II); el experto mezcla el
  layout del cap. 10 con el mapa a `Operacion` del cap. 27 y el `magic`/endian
  del cap. 9.
- **Dificultad asimétrica**: una idea nueva por sección (apéndizar →
  framing → CRC → código → las DOS lecturas → desgaste); para el conocimiento
  dificultad baja; para la destreza, los ejercicios exigen reconstrucción y
  razonamiento de contrato sin pistas que lo regalen.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb --lib
  cap10_append_only` (6 tests citados por nombre).
- **Citas**: Gray 1978 (thesis/«Notes on Data Base Operating Systems» 1978;
  «The Transaction Concept: Virtues and Limitations» mostró el papel de los
  logs en transacciones y formas de fallo) y Gray 1981 (WAL: escribir el
  cambio al log ANTES que a los datos); SQLite «Write-Ahead Logging»
  (documentación oficial: WAL mode vs rollback journal, `wal_checkpoint`);
  Bernstein, Hadzilacos & Goodman, «Concurrency Control and Recovery in
  Database Systems», 1987 (redo/undo logs y la regla de que una escritura
  física exitosa o nula estructura del log antes que los datos); CRC32
  (polinomio IEEE 802.3 estándar; valor canónico `crc32("")==0`,
  `crc32("a")==0xE8B7BE43`); `crc32fast` (la implementación de producción
  que usa tablas / SIMD).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (9 en la tabla §5; Gray 1978/1981, Bernstein et al. 1987, SQLite WAL, CRC32 IEC 802.3).
- [x] Escenario de fallo visible: corte de luz (escritura en sitio pisa el byte bueno) y el `log_recovery_desde_offset` que lo demuestra en el append-only.
- [x] Código ejecutable en workspace (6 tests ALL_GREEN en `mod tests_log`) citado por nombre y línea, no duplicado.
- [x] Misconcepción corregida explícitamente (§1: seis; «apéndizar ≠ pisar», «el corte deja prefijo íntegro», «el CRC detecta sin copia de referencia», «lectura estricta ≠ iteración de recuperación», «el log no es redundancia gratuita»).
- [x] Ejercicios con solución verificable (tests del workspace + layout a mano comprobable contra `encode_log_record`).
- [x] ≥1 ejercicio de retrieval (layout a mano / mapa tag↔variante sin mirar código) y ≥1 de spacing (cap. 9 encoding LE re-usado; cap. 3 Vol. I IDs estables).
- [x] Responde la pregunta crítica del CORPUS (línea ~193-198: la idea central de no modificar datos en sitio, apéndizar) y el brief: inmune a escrituras a medias, fundamento del WAL del cap. 28, recuperación.
- [x] Repaso-cierre de la Parte II (6→9 + log) con diagrama; ganchos al cap. 11 (Parte III, páginas), cap. 27 (Operacion heredera) y cap. 28 (WAL — el germen directo).
- [x] Anécdotas verificables con fuente: Jim Gray / bitácoras de mainframes (los logs de recuperación de SYSTEM R y los «reditos» de los sistemas de los 70), WAL (Gray), SQLite (rollback journal vs WAL mode), Bernstein et al. 1987 (redo/undo).
- [x] Las cifras del layout (20 bytes para un PutNode id=1 payload=[1,2,3]) son comprobables contra el código real; `crc32("")`/`crc32("a")` son los valores canónicos del estándar.
