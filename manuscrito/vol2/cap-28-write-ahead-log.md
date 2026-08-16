# Capítulo 28 — Write-Ahead Log

> *«El log se escribe antes que el dato. No como orden de paso: como promesa sobre la dirección de la recuperación.»*

## 28.0 La anécdota de la esquina

En 1992, un equipo de IBM en Almaden —C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh y P. Schwarz— publicó un paper largas veces citado y pocas veces leído entero: «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging» (ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162). El paper cierra una pelea que llevaba quince años coleando en la industria: cuándo, dónde y cómo se le promete a un usuario que su `COMMIT` sobrevivirá a un corte de luz. Y lo hace con una frase que define un capítulo entero: «write-ahead logging — the log records for a change must be written to stable storage before the change itself is written to the database».

Que el log se escriba ANTES que la página de datos suena a orden de pasos. No lo es. Es una promesa sobre la dirección de la recuperación. Si la promesa es «el log antes que el dato», entonces cuando el sistema despierta tras el fallo, el camino es RE-APLICAR lo que el log dijo que se confirmó (roll-forward / redo). Si la promesa es al revés —el commit se escribe al final del apply y los datos pueden estar ya en disco mientras el log sigue en RAM por un instante—, entonces la mitad desconocida del problema («¿qué sobrevivió?») NO se resuelve releyendo: se resuelve DESHACIENDO lo que se aplicó de más. Y deshacer requiere imágenes ANTES, lo que dobla el log y la maquinaria. Esa segunda mitad se llama UNDO; el capítulo que la construye es el 29. ARIES propuso las dos mitades (Analysis-Redo-Undo) y la decisión de CÓMO se ordenan. Aquí elegimos la mitad más simple, la que enseñamos primero y la que cierra la Parte VI del Vol.II en su primer tramo: sólo REDO, y el redo existe porque el log se escribió ANTES.

La otra raíz, anterior, está en System R: Jim Gray documentó la «bitácora» en «Notes on Database Operating Systems» (IBM Research Report RJ 2188, 1978) y la convirtió en columna vertebral de la recuperación. Lo que el lector de hoy llama WAL es la versión 1992 de la bitácora de 1978 — con LSN monótono, con CRC, con el «log antes que el dato» como axioma. Y lo que hoy les voy a pedir que entiendan no es un detalle: es la decisión de la que viven los demás capítulos de la Parte VI.

## 28.1 Objetivo

Al terminar este capítulo sabrás **por qué** la transacción del cap. 27 (con su `commit` re-validando y aplicando) se rompía frente a un fallo del apply, y **cómo** un write-ahead log convierte ese «se quedó a medias y nadie recuerda qué faltaba» en «un log SÍ recuerda y el replay lo completa». Cinco piezas en `cap28_wal.rs`:

1. **El registro del WAL** (`WalRecord`) — la pareja `(lsn, tx_id)` más `CuerpoWal = Begin | Operacion(op) | Commit | Rollback`, con el framing del cap. 10 (`length-prefix + CRC32`) y la `Operacion` del cap. 27.
2. **El protocolo write-ahead** (`WalTransaccion::commit`) — el commit en dos fases: el log se escribe y sella CON UN `Commit` ANTES de tocar el store. Esa es la decisión del capítulo.
3. **El redo idempotente** (`aplicar_para_redo` + `replay_wal`) — la regla por la cual re-aplicar lo ya aplicado es un no-op, y por la que el replay no necesita saber qué sobrevivió al fallo.
4. **La política de flush** (`PoliticaFlush`) — `CadaEscritura` (la regla de oro literal) vs `SoloCommit` (un sync por transacción; semilla del group commit del cap. 30).
5. **El truncado con contrato** (`truncar_hasta_lsn`) — poda del log BAJO FIRMA del llamador; los LSNs no se reinician.

Y el hito que demuestra que el capítulo cierra el agujero del cap. 27: el `StoreQueFalla` con cuatro ops y fallo en la tercera, escenario IDÉNTICO al de aquél, termina con `node_count()==4` tras el replay.

## 28.2 Problema

El cap. 27 dejó una deuda escrita en el `Display` de su error:

> *«fallo durante el APPLY de la operación #2 (…): 2 operaciones ya estaban aplicadas y sin log no se pueden deshacer — el store quedó a medias (cap. 28: WAL)»*

Esa frase no es retórica: es el agujero que vino a cerrar este capítulo. Recordemos los dos escenarios que el `StoreQueFalla` del cap. 27 dejó como tests vivos:

1. **Error del store a mitad del apply.** El buffer validó, las dos primeras escrituras pasan, la tercera falla. El `commit` devuelve `ApplyFallido{indice: 2, aplicadas: 2}` y el store queda con `node_count()==2`. La transacción «no ocurrió» PORQUE SU DECLARACIÓN está sólo en RAM; el siguiente open del proceso es un open con dos nodos y la mitad de la verdad perdida.
2. **Pánico a mitad de apply.** El store «muere» eléc­tri­ca­mente (un `panic!("corte de luz simulado")` en la segunda escritura). `catch_unwind` rescata el hilo. El store quedó con un nodo. La memoria del proceso se evaporó; nadie sabe qué faltaba.

En ambos casos, la transacción «no se completó» pero su ESFUERZO no se puede revertir limpiamente. El cap. 27 ya había decidido que sólo se aplica «o todo o nada» AL NÍVEL DEL STAGING (la validación de `stage()` expulsa la op mala y la tx SIGUE VIVA). Pero el apply real es otra cosa: cuando `put_node` se ejecuta de verdad contra el store, ya no hay rollback posible sin UNDO — y UNDO no existe en el motor.

Y al lado de la atomicidad (A), la durabilidad (D) del cap. 27 era **Ninguna** — el `commit` vivía en RAM. Un close del proceso se llevaba la transacción confirmada, las páginas de datos, todo. La entrada D del `informe_acid()` del cap. 27 decía, textual, «en RAM, no durable (cap. 28)».

La pregunta del capítulo: ¿cuál es la pieza que CONECTA las dos cosas — la atomicidad frente al apply real y la durabilidad del commit — sin pedir UNDO?

## 28.3 Modelo mental

Piensa en una **notaría con libro de entrada**.

- Toda escritura del grafo que promete durabilidad PASA primero por la notaría. El notario abre un asiento nuevo, anota la operación y la «sella» con un número correlativo (LSN; entry `1`, `2`, `3`…). Tan sólo después de que la anotación está sellada y firmada contra el libro (el `sync`) se le da al cliente la copia que modifica el fichero físico de la notaría (apply al store).
- Si en el momento de la firma el cliente sufre un infarto (el `StoreQueFalla` del cap. 27), la notación YA está en el libro; al día siguiente, nuevo notario, **relee el libro y completa la operación** (roll-forward). Las dos primeras escrituras del nodo ya estaban; la tercera se aplica al releer. La idempotencia del redactor hace que las escrituras duplicadas no dupliquen.
- Si la firma no llegó (commit truncado por un corte de luz), el asiento existe pero el cliente no tiene sello: la operación «como si nunca hubiera ocurrido». El replay del día siguiente la descarta limpia.
- Si alguien arrancó una página del libro (un CRC roto) o arrancó un folio de en medio (un hueco de LSN), el libro entero deja de ser confiable — la notaría no se inventa los huecos. El iterador para limpio en la cola rota; el modo estricto grita el LSN del registro dañado.

```
            commit en dos fases (commit-marker-ANTES-del-apply)
  ┌──────────────────────────────┐   ┌──────────────────────────────┐
  │  RE-VALIDAR                  │   │  ALTERNATIVA RECHAZADA       │
  │  buffer con validar_buffer   │   │  marker al final del apply   │
  │  (cap. 27, pub(crate))       │   │  → apply a medias sin commit │
  ├──────────────────────────────┤   │  → rescate exige UNDO        │
  │  log_write(op) por cada op   │   │  → ARIES, cap. 29            │
  │  (sync según PolíticaFlush)  │   └──────────────────────────────┘
  ├──────────────────────────────┤
  │  log_write(Commit) + sync    │   ← EL PUNTO DE DURABILIDAD
  ├──────────────────────────────┤
  │  apply al store              │   ← puede fallar a medias
  │  (si falla, replay_wal       │
  │   rescue con roll-forward)   │
  └──────────────────────────────┘
```

Y un detalle no obvio: el **LSN** es el sello notarial. Una vez asignado, no se reutiliza — ni aunque arranque una página nueva del libro. Por eso un sello de 2007 (`lsn=42`) puede seguir viviendo en el libro aunque las primeras 40 páginas ya se hayan borrado: la identidad de una anotación es su sello, no su posición en bytes. La identidad del redo «el nodo 7 que escribí en `lsn=42`» es estable a través del truncado.

El momento ¡ajá!: «"el log antes que el dato" no es una orden de paso. Es una promesa sobre la dirección de la recuperación. Si el log se escribe ANTES, la mitad DESCONOCIDA del problema (qué sobrevivió al fallo) se resuelve con un re-leer. Si se escribe DESPUÉS, la mitad desconocida exige deshacer — y deshacer necesita imágenes ANTES, lo que dobla el log. La pregunta "¿UNDO o no?" se contesta ANTES de escribir la primera línea de código».

## 28.4 Primera solución

La solución ingenua ya la tienes y funciona: la del cap. 27. `Transaccion::commit` re-valida el buffer y aplica. Si el apply falla a medias, `ApplyFallido` y fin. La transacción era ACID en todo MENOS en D (durabilidad en RAM) y A frente al apply real (no frente a validación). El ERROR del cap. 27 es exactamente la motivación del cap. 28.

Una segunda solución ingenua es tan tentadora como la primera: «pues hago que el apply sea atómico mágico» — operaciones tan pequeñas que nunca produzcan fallo a medias. No funciona: cualquier cambio a una página de 4.096 bytes es o SÍ o NO; no hay «a medias significativas». Y aunque funcionara, no resuelve la durabilidad: si el proceso muere justo después del apply, la operación se fue a RAM y se perdió.

## 28.5 Sus límites

Tres límites llevan del cap. 27 al cap. 28:

1. **El apply real falla a medias y NO hay UNDO.** El `StoreQueFalla` del cap. 27 hace esto literalmente: falla en la N-ésima escritura y el store queda inconsistente. Sin UNDO no hay forma de sacar las dos operaciones que sí pasaron. Y UNDO exige imágenes ANTES, con la mitad de la complejidad y el doble del log.
2. **El commit vive en RAM.** El `ResumenCommit` del cap. 27 decía, textual, «en RAM, no durable (cap. 28)». Un cierre del proceso se lleva la tx confirmada. La entrada D del `informe_acid()` era `NivelGarantia::Ninguna`.
3. **El siguiente proceso que abre la BD no recuerda nada.** El store en RAM arranca de cero. La `Operacion` se construyó como dato precisamente para que algún día se pudiera serializar — pero ningún capítulo la había serializado todavía.

El patrón común: **sin log no hay vuelta atrás**. El nombre técnico del agujero es «atomicidad frente al apply real», pero el síntoma — store a medias sin memoria — es lo que duele.

## 28.6 Solución evolucionada: el log antes que el dato

Una sola decisión de diseño cambia tres cosas a la vez. La decisión es del capítulo y es la siguiente:

> **El registro `Commit` se escribe al log y se sella con un `sync` ANTES de que el apply toque el store.**

A partir de esa decisión, todo lo demás se deduce:

- El log crece con un orden FIJO: Begin, operaciones (cada una con su LSN), Commit. Cuando el apply arranca, TODO el intento ya es durable. Si el apply falla a medias, NO es un escenario perdido: el log contiene el Begin, las operaciones y el Commit. Un `replay_wal` las re-aplica (idempotente) y la transacción se completa.
- El LSN es la dirección física del log y la identidad del redo. Monótono, consecutivo, nunca reutilizado. La duración del log y la posición de los registros son ortogonales: truncar mueve bytes, no toca LSNs.
- El sólo hecho de escribir el log antes que el dato HACE que UNDO sea innecesario: el apply a medias es una operación INCOMPLETA, no una operación APLICADA. La idempotencia del redo se encarga del resto.

La otra decisión — complementaria, no rival — es la política de flush. La regla de oro es simple: tras cada `log_write`, un `sync`. Es la opción por defecto del capítulo (`CadaEscritura`). Pero si la semilla del group commit (cap. 30) ya obligó a la observación de que las páginas de datos no se llevan a disco antes del commit, entonces un `sync` POR TRANSACCIÓN es correcto: justo antes del apply ya se sabe que las páginas de datos están en RAM y no pueden quedar pisadas por un fsync previo. Esa es la política `SoloCommit`, un sync por transacción, semilla del cap. 30.

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap28_wal.rs`. Vamos a leerlo por partes, porque cada tipo y cada `commit` tiene un porqué.

### El formato del registro

```rust
pub type Lsn = u64;
pub type TxId = u64;

pub enum CuerpoWal {
    Begin,
    Operacion(Operacion),   // MISMA del cap. 27
    Commit,
    Rollback,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub tx_id: TxId,
    pub cuerpo: CuerpoWal,
}
```

Cuatro piezas básicas, tres reutilizaciones deliberadas:

- **`Operacion(Operacion)`** es la misma del cap. 27 — y la misma forma que el `RecordKind` del cap. 10 anticipaba. La semilla del WAL estaba plantada dos capítulos antes.
- Los strings y `Value` se serializan con el encoding del cap. 9 (`encode_string`, `encode_value`, `decode_*`).
- El framing hereda el del cap. 10: `length-prefix u32` para saber dónde termina cada registro, **`CRC32` con `crc32_simple`** para detectar bytes modificados sin re-parsear.
- Lo único nuevo: `u64` LE para LSN y TxId, porque el LSN exige un orden monótono y los ids del cap. 7 no cabían en `u32`.

Un detalle cargadito de consecuencias: las props de un `Node` son un `HashMap`. La iteración de un `HashMap` **no es determinista** — dos llamadas pueden devolver orden distinto. Un log debe codificar SIEMPRE los mismos bytes para el mismo valor (porque el CRC se calcula sobre los bytes, no sobre la operación lógica), así que `encode_props` ordena las claves antes de serializar. Sin esta ordenación, el mismo grafo produciría dos logs distintos y los tests de roundtrip fallarían al azar.

### El framing byte a byte

```
[record_len: u32 LE][lsn: u64 LE][tx_id: u64 LE][tag: u8][payload...][crc32: u32 LE]
```

El orden de las validaciones al decodificar importa y es la cadena del cap. 11 + cap. 10:

1. **length-prefix** → sabemos si el registro está completo o truncado.
2. **CRC32** sobre `body` (todo menos el CRC). Si no cuadra, NO parseamos el cuerpo: un byte rotado no debe «interpretarse» como un valor legal.
3. **tag** → Begin/Operacion/Commit/Rollback.
4. **payload** → `Operacion` completa, codificada con la misma `encode_operacion` que el commit también produce.

El CRC se valida ANTES que el tag porque la corrupción es local: si el cuerpo está roto, no sabemos qué versión del payload es la «correcta». Esa cadena — length → CRC → parse — es la del cap. 11, y se hereda tal cual.

### El WAL: el «disco» del capítulo

```rust
pub struct Wal {
    bytes: Vec<u8>,         // concatenación de registros
    next_lsn: Lsn,          // 1, 2, 3, … — nunca reinicia
    next_tx_id: TxId,       // 1, 2, 3, … — nunca reinicia
    syncs: u64,             // contador de "fsync"
    politica: PoliticaFlush,
}
```

En RAM (igual que el `AppendOnlyLog` del cap. 10, su germen directo): un `Vec<u8>` de registros encadenados por length-prefix. Un WAL de fichero real sería un `File` con `O_APPEND` y `sync_all` — el mismo `FilePager::sync` del cap. 12; el PROTOCOLO (qué se escribe, cuándo, y quién se lee al despertar) es lo que este capítulo construye. La `fsync` real es la del cap. 12; aquí la durabilidad es un CONTADOR que cuenta las veces que el protocolo manda llevarla a cabo. Lo que los tests verifican es que se LLAME cuando el protocolo lo exige — y eso es la pieza AUDITABLE.

Tres invariantes que el capítulo sostiene y los tests verifican:

- Los LSN son monótonos y consecutivos desde el 1, y NUNCA se reutilizan — ni después de truncar.
- Los TxId también son monótonos desde el 1.
- `sync()` cuenta las «fsync»: en RAM no hay nada que sincronizar, pero la PROMESA es verificable y los tests la cuentan.

### El commit en dos fases

```rust
pub fn commit(self) -> Result<ResumenCommitWal, WalError> {
    // 1. Re-validar el buffer entero (punto de no retorno)
    validar_buffer(self.store, &self.buffer).map_err(WalError::Validacion)?;

    // 2. FASE LOG: cada operación al log ANTES que al store
    for op in &self.buffer {
        self.wal.log_write(self.tx_id, CuerpoWal::Operacion(op.clone()));
        if self.wal.politica() == PoliticaFlush::CadaEscritura {
            self.wal.sync();
        }
    }
    // 3. EL PUNTO DE DURABILIDAD: Commit + sync
    let lsn_commit = self.wal.log_write(self.tx_id, CuerpoWal::Commit);
    self.wal.sync();

    // 4. FASE APPLY: el store puede fallar aquí; el log ya tiene TODO
    for (indice, op) in self.buffer.iter().enumerate() {
        // … (put_node / put_edge / delete_node / delete_edge) …
    }
    Ok(resumen)
}
```

Cinco líneas que esconden las decisiones del capítulo:

- **Línea 2** (`re-validar`): el mismo `validar_buffer` del cap. 27 (que pasa a `pub(crate)` en este capítulo para que el handshake sea limpio). El «punto de no retorno» del cap. 27 se hereda; el `commit` re-valida el buffer entero porque entre el `stage()` y el `commit` nada cambió — el borrow checker lo garantiza.
- **Líneas 5-9** (FASE LOG): el write-AHEAD. Cada `Operacion` se serializa al log con su LSN. En `CadaEscritura`, cada `log_write` añade un `sync`. La política correcta es la del cap. 22: ruidosa, explícita, sin atajos silenciosos.
- **Línea 11** (`Commit + sync`): el punto de durabilidad. El LSN de este registro es devuelto al llamador (`lsn_commit`), y queda escrito en `ResumenCommitWal.lsn_commit`. A partir de esta línea, la transacción existe aunque el proceso muera al siguiente tick.
- **Líneas 14-24** (FASE APPLY): el store se toca. Puede fallar. Si falla, el llamador recibe `WalError::ApplyFallido{ indice, aplicadas, causa }` — la MISMA FORMA que el `TransaccionError::ApplyFallido` del cap. 27 — pero el `Display` cambia: «… pero el log YA contiene el commit: `replay_wal` COMPLETA la transacción (arranque automático: cap. 29)».

Ésta es la decisión del capítulo: **commit-marker-ANTES-del-apply**. La alternativa — escribir el `Commit` al final del apply — dejaría el apply a medias SIN commit y rescatarlo exigiría UNDO. UNDO es la mitad que ARIES (Mohan et al. 1992) construye completa. Aquí no la necesitamos: el log ya contiene todo lo que la tx quería hacer, y `replay_wal` la termina. El nombre técnico de la estrategia es **roll-forward** o **redo-only**.

### El redo idempotente

```rust
pub(crate) fn aplicar_para_redo(
    store: &mut dyn GraphStore,
    op: &Operacion,
) -> Result<(), StoreError> {
    match op {
        Operacion::PutNode(n) => match store.get_node(n.id) {
            Some(actual) if actual == n => Ok(()),       // idéntico = no-op
            Some(_) => {                                  // divergente = el log manda
                store.delete_node(n.id);
                store.put_node(n.clone())
            }
            None => store.put_node(n.clone()),
        },
        // … igual con aristas y deletes …
    }
}
```

Dos reglas y media:

- **Put idéntico al que ya está = no-op.** Por eso un replay sobre un store al que ya se aplicó la mitad NO duplica.
- **Put divergente del que ya está = overwrite.** El log es la verdad; si el registro dice «este nodo tiene estos labels» y el store tenía «este nodo tiene estos otros labels», gana el log.
- **Delete de lo ausente = no-op silencioso.** Re-aplicar un `DeleteNode` sobre un nodo que el apply a medias ya borró no es un error.

La idempotencia es lo que hace que el replay no necesite saber qué sobrevivió al fallo. Sin ella, el replay necesitaría un análisis del estado exacto al fallar (eso es la fase Analysis de ARIES, el cap. 29), y la «orden de re-aplicar lo que el log dice» dejaría de ser segura. La regla es: si una operación del log puede ejecutarse DOS VECES seguidas sin cambiar el resultado, la operación es REDO-segura. Las cuatro `Operacion` lo son.

### El replay en dos pasadas

```rust
pub fn replay_wal(store: &mut dyn GraphStore, wal: &Wal) -> Result<InformeReplay, WalError> {
    // Pasada 1: ¿quién llegó a Commit?
    let mut confirmadas: HashSet<TxId> = HashSet::new();
    let mut iniciadas: HashSet<TxId> = HashSet::new();
    for rec in wal.iter() {
        match rec.cuerpo {
            CuerpoWal::Begin => { iniciadas.insert(rec.tx_id); }
            CuerpoWal::Commit => { confirmadas.insert(rec.tx_id); }
            _ => {}
        }
    }

    // Pasada 2: redo de lo confirmado, en orden de LSN
    let mut operaciones = 0usize;
    for rec in wal.iter() {
        if let CuerpoWal::Operacion(op) = &rec.cuerpo
            && confirmadas.contains(&rec.tx_id)
        {
            aplicar_para_redo(store, op).map_err(|causa| WalError::RedoFallido {
                lsn: rec.lsn,
                causa,
            })?;
            operaciones += 1;
        }
    }

    Ok(InformeReplay {
        transacciones_confirmadas: confirmadas.len(),
        transacciones_descartadas: iniciadas.difference(&confirmadas).count(),
        operaciones_reaplicadas: operaciones,
    })
}
```

Dos pasadas, una decisión:

- **Pasada 1** junta los `tx_id` con `Commit`. Es O(N) sobre el log (N = número de registros). Te dice, sin importar el orden, quién lived to tell the tale.
- **Pasada 2** re-aplica, en orden de LSN (= orden del log), sólo las operaciones de las txs confirmadas. También O(N). Los Begin sin Commit posterior son DESCARTADOS: «como si nunca hubieran ocurrido». El `informe.transacciones_descartadas` los cuenta.

La alternativa — una sola pasada con estado por registro — exige tres estados por entrada (pendiente / en redo / confirmado) y no resuelve la intercalación de Begin/Operación/Commit/Rollback de V transacciones que el log admite por construcción (preparado para el group commit). Dos pasadas es la complejidad mínima que respeta la intercalación.

### El truncado con contrato

```rust
pub fn truncar_hasta_lsn(&mut self, lsn: Lsn) -> usize {
    // … busca el primer registro con lsn > lsn, descarta lo anterior …
    // NUNCA: next_lsn, next_tx_id — los LSNs no se reutilizan.
}
```

El log puede ser enorme; en algún momento hay que podarlo. La poda es SEGURA sólo si los registros podados ya son visibles en el store. **Si se trunca más allá, los redos posteriores pueden quedar HUÉRFANOS** (arista con un nodo que ya no está en el log). El contrato del llamador cierra la puerta:

> *«Truncar lo no-durable PIERDE datos: el replay sólo ve lo que queda. El checkpoint que decide «hasta dónde es seguro» de forma automática es el cap. 29; la rotación por tamaño del fichero queda como deuda documentada.»*

Lo que NUNCA se reinicia: `next_lsn` / `next_tx_id`. La identidad de un redo es su LSN, no su posición en bytes. Si el llamador trunca los primeros 2000 LSNs y luego escribe una tx nueva, los nuevos LSNs empiezan en 2001, no en 1. Que dos redos tuvieran el mismo LSN sería una corrupción lógica; el motor los distinguiría por la posición en bytes, sí, pero la identidad del registro se rompería debajo.

## 28.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap28_wal.rs`. Vamos a leerlo por partes, porque cada `commit` y cada `replay` tiene un porqué.

### La transacción con WAL

```rust
pub struct WalTransaccion<'a> {
    store: &'a mut dyn GraphStore,
    wal: &'a mut Wal,
    tx_id: TxId,
    buffer: Vec<Operacion>,
}

impl<'a> WalTransaccion<'a> {
    pub fn begin(store: &'a mut dyn GraphStore, wal: &'a mut Wal) -> Self {
        let (tx_id, _lsn) = wal.begin_tx();
        WalTransaccion { store, wal, tx_id, buffer: Vec::new() }
    }

    pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
        self.buffer.push(operacion);
        match validar_buffer(self.store, &self.buffer) {
            Ok(()) => Ok(()),
            Err(e) => { self.buffer.pop(); Err(e) }
        }
    }

    pub fn put_node(&mut self, node: Node) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutNode(node))
    }

    pub fn commit(self) -> Result<ResumenCommitWal, WalError> {
        // (cuerpo del commit en dos fases, ver §28.6)
    }

    pub fn rollback(self) -> ResumenRollbackWal {
        let lsn_rollback = self.wal.log_write(self.tx_id, CuerpoWal::Rollback);
        ResumenRollbackWal {
            operaciones_descartadas: self.buffer.len(),
            lsn_rollback,
        }
    }
}
```

Tres préstamos exclusivos simultáneos — uno al store, otro al WAL, otro a sí misma — codifican el «un único escritor» del cap. 27. Mientras la transacción vive, nadie más toca el store NI el WAL. El ciclo de vida en los tipos: `commit` y `rollback` consumen `self`; usar una tx cerrada o anidar dos sobre el mismo store no compila.

Y un detalle que la prosa debe decir: el `Drop` implícito (no hay `Drop` explícito) no escribe Rollback. ¿Por qué? Porque si la tx se cierra sin commit ni rollback, lo más probable es que sea un `unwrap()` intermedio o un pánico. **No escribir el marker** es la decisión correcta: la ausencia de Commit cancela la tx en el replay, y un marker Rollback escrito en el panic path sería mentiroso (el store «no murió» realmente, sólo el unwrap falló). El `Wal::iter` la descarta con la misma pinta: sin Commit, la tx no ocurrió.

### La `PoliticaFlush`

```rust
pub enum PoliticaFlush {
    CadaEscritura,  // sync tras cada log_write: la regla de oro, literal
    SoloCommit,     // sync SÓLO en el commit: un sync por tx; semilla del group commit
}
```

Se enseña la regla antes que la optimización. El alumno que tropieza con `SoloCommit` primero NO entiende por qué un sync por tx es suficiente, y el día que la máquina sufra un fsync lento y la mitad de los commits se alarguen misteriosamente, no sabe dónde mirar. Con `CadaEscritura` por defecto, el alumno mide 4 syncs para 3 ops + commit; con `SoloCommit`, mide 1. Y cuando el cap. 30 enchufe concurrencia, la semilla ya está plantada.

El `Wal::default` se implementa a mano, no se deriva: la política por defecto es CONTENIDO del capítulo, no azar. `#[derive(Default)]` requeriría un `PoliticaFlush::default()` que decide quién sabe cómo. El `Default` manual fija `CadaEscritura` y documenta la decisión.

## 28.8 Prueba de fuego

La prueba de fuego no es «los tests pasan» — es **«el agujero del cap. 27 está cerrado»**. El test-tesis del capítulo reproduce el escenario que el cap. 27 AFIRMABA (con `node_count()==2` y «sin log no se pueden deshacer») y lo termina al revés.

```rust
// El mismo StoreQueFalla del cap. 27, las mismas 4 ops, el mismo fallar_en: 3.
let mut store = StoreQueFalla {
    inner: MemoryStore::new(),
    escrituras: 0,
    fallar_en: 3,
    con_panic: false,
};
let mut wal = Wal::new();
let mut tx = WalTransaccion::begin(&mut store, &mut wal);
tx.put_node(Node::new(0, "A")).unwrap();
tx.put_node(Node::new(1, "B")).unwrap();
tx.put_node(Node::new(2, "C")).unwrap();  // el store fallará aquí
tx.put_node(Node::new(3, "D")).unwrap();
let err = tx.commit().unwrap_err();

// MISMA FORMA que el cap. 27…
assert_eq!(
    err,
    WalError::ApplyFallido {
        indice: 2,
        aplicadas: 2,
        causa: StoreError::UnknownNode(usize::MAX)
    }
);
assert!(err.to_string().contains("replay_wal"));

// …PERO el log SÍ contiene TODO + Commit + sync.
assert_eq!(store.node_count(), 2);                     // a medias, como el cap. 27
assert_eq!(wal.syncs(), 1 + 4);                        // commit + 4 escrituras
let ultimo = wal.iter().last().unwrap();
assert_eq!(ultimo.cuerpo, CuerpoWal::Commit);          // EL COMMIT ESTÁ

// EL RESCATE: replay sobre el MISMO store a medias → COMPLETA.
let informe = replay_wal(&mut store, &wal).unwrap();
assert_eq!(informe.operaciones_reaplicadas, 4);
assert_eq!(store.node_count(), 4);                      // LA TRANSACCIÓN COMPLETA
assert!(store.get_node(3).is_some());                  // el nodo "D" llegó
```

Y la versión con `panic!` (el `corte de luz` literal del cap. 27):

```rust
let mut store = StoreQueFalla { fallar_en: 2, con_panic: true, … };
let resultado = catch_unwind(AssertUnwindSafe(|| {
    let mut tx = WalTransaccion::begin(&mut store, &mut wal);
    tx.put_node(Node::new(0, "A")).unwrap();
    tx.put_node(Node::new(1, "B")).unwrap();  // pánico AQUÍ
    tx.put_node(Node::new(2, "C")).unwrap();
    tx.put_node(Node::new(3, "D")).unwrap();
    tx.commit()
}));
// Rescatamos el pánico, el store quedó con 1 nodo.
assert_eq!(store.node_count(), 1);
// Pero el log SÍ terminó con Commit.
assert!(matches!(wal.iter().last(), Some(r) if r.cuerpo == CuerpoWal::Commit));
// Y el replay completa.
let informe = replay_wal(&mut store, &wal).unwrap();
assert_eq!(informe.operaciones_reaplicadas, 4);
assert_eq!(store.node_count(), 4);
```

Ésta es la inversión de la regresión del cap. 27: el escenario que aquél cerraba con «y nadie recuerda qué faltaba», éste lo abre con «el log SÍ recuerda» y lo cierra con `node_count()==4`. La forma del error es la MISMA (`ApplyFallido{indice: 2, aplicadas: 2}`); el contenido del `Display` cambió. La prueba de que el capítulo sirve es que vuelve falsa la afirmación del anterior.

Y los casos de fallo son igual de importantes:

```rust
// Un CRC tocado en el ÚLTIMO byte del log:
let mut bytes = encode_wal_record(&rec1);
bytes.extend(encode_wal_record(&rec2));
bytes[bytes.len() - 1] ^= 0xFF;  // touch the CRC
let err = decodificar_wal(&bytes).unwrap_err();
assert!(matches!(err, WalError::CrcInvalido { lsn: Some(2), .. }));

// Un registro truncado por un corte de luz:
let cortado = &completo[..completo.len() - 3];
let err = decodificar_wal(cortado).unwrap_err();
assert!(matches!(err, WalError::RegistroTruncado { .. }));

// Un hueco de LSN (bytes quitados de en medio):
let err = decodificar_wal(&encode_con_hueco).unwrap_err();
assert_eq!(err, WalError::LsnInvalido { leido: 5, esperado: 2 });

// Un truncado agresivo (rompe dependencias):
let err = replay_wal(&mut vacio, &wal_truncada).unwrap_err();
assert!(matches!(err, WalError::RedoFallido { lsn: 6, .. }));
```

Estos tests — `crc_invalido_detectado`, `registro_truncado_detectado`, `lsn_invalido_en_cadena_detectado`, `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias` — encapsulan las cuatro promesas del WAL: **detectar corrupción, detectar truncado, detectar huecos, fallar ruidosamente cuando el contrato del truncado se rompe**. Si este capítulo se te olvidara, las dos entradas (A y D) del `informe_acid()` volverían a `NivelGarantia::Ninguna` y el `StoreQueFalla` del cap. 27 volvería a dejar el store a medias «sin vuelta atrás».

## 28.9 Qué hemos sacrificado

Toda estructura tiene un precio. El WAL no es gratis:

1. **Trabajo extra en cada commit.** El log se escribe entero ANTES del apply; las escrituras adicionales son O(N) registros por transacción. Sin WAL, el commit era un loop sobre el buffer. El precio es la AUDITABILIDAD: lo que se gana es que ningún commit confirmado se pierde.
2. **El log crece sin parar.** Sin truncado, el log crece hasta llenar el disco. El truncado con contrato del llamador es una solución PARCIAL: el cap. 29 construirá el checkpoint que decide «hasta dónde» automáticamente.
3. **Sin UNDO, una tx con apply a medias se rescata con replay.** Pero si la política fuera *steal* (aplicar antes de confirmar), el apply a medias de una tx abortada exigiría UNDO — antes-images o CLR. La pregunta «¿UNDO o no?» es de DISEÑO, no de implementación: la mitad «no-UNDO» es lo que hace al WAL del cap. 28 tan simple.
4. **El `ApplyFallido` mantiene la forma `ApplyFallido{ indice, aplicadas, causa }` del cap. 27.** Quien migró del cap. 27 al cap. 28 ve un cambio de `Display` y un cambio de signatura del wrapper (`WalError::ApplyFallido` en vez de `TransaccionError::ApplyFallido`). El compilador lo señala, pero la ambigüedad conceptual — «¿es lo mismo?» — es justo la que el capítulo quiere que el alumno resuelva.
5. **Group commit REAL (varias tx concurrentes compartiendo un fsync) NO está implementado.** La semilla — `SoloCommit` con un sync por tx — sí. La concurrencia es el cap. 30. Deuda documentada, código plantado.

## 28.10 Cómo lo hace una BBDD real

El WAL del capítulo es mínimo. Las bases de datos reales añaden piezas que aquí NO están (y se nombran como «luego lo verás», honestamente):

- **Persistencia real.** Un WAL en RAM es un `Vec<u8>`; un WAL real es un `File` abierto con `O_APPEND` y `sync_all` por medio. El `FilePager::sync` del cap. 12 es la pieza que enchufa el disco debajo del log. Lo que aquí se cuenta como un contador, allí es la `fsync(2)` del sistema operativo.
- **Checkpoint.** Quinielamos: ¿hasta qué LSN es seguro truncar? El cap. 29 construye el algoritmo — normalmente, el LSN hasta el cual TODAS las páginas de datos han sido llevadas a disco (con `dirty page table` y todo eso). ARIES clásico: la dirty page table y el `recLSN` por página.
- **Steal / no-steal.** Aquí NO se roba: sólo se aplica después del commit. Por eso UNDO no hace falta. Una BD real con steal (deja páginas modificadas en disco antes del commit) necesita antes-images o CLR — la otra mitad de ARIES.
- **Group commit.** Varias tx concurrentes comparten un fsync: la semilla es `SoloCommit` sin concurrencia; el cierre es el cap. 30.
- **Write-ahead con reordenación.** Una BD grande puede diferir la escritura de la página de datos al disco LUEGO del commit, siempre que la página esté en el log antes; la política es la misma, la cola es más larga.

En todas, el patrón es idéntico al que has construido: **registro con framing + política de flush explícita + commit-marker-ANTES-del-apply + redo idempotente + truncado con checkpoint**. Lo que cambia es la cantidad de piezas de cada tipo y la finura del contrato.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: predice los LSNs y el número de syncs de una transacción con 2 puts y 1 delete, en `CadaEscritura`, y compruébalo con `commit_aplica_todo_y_es_durable`.
- *Intermedio*: cambia la `Tag` numérica de un `Operacion::PutNode` (1 → 5) en el encoding y demuestra que el roundtrip falla con `tag de cuerpo desconocido`. ¿Por qué la serialización trata los tags como una decisión ABI?
- *Experto*: añade una nueva variante `Operacion::PutLabel(NodeId, String)` que asigne una segunda etiqueta a un nodo existente. Codifícala, decodifícala, propágala por el `apply` del `commit` y por `aplicar_para_redo`, y rehaz el ciclo de tests.

## 28.11 Lo que te llevas

- La **regla del write-ahead** es una promesa sobre la dirección de la recuperación: si el log se escribe ANTES, el camino es re-aplicar (redo). Si se escribe DESPUÉS, el camino es deshacer (undo). La mitad que cuesta menos es la primera.
- El **LSN** es la dirección física del log y la identidad del redo: monótono, consecutivo, asignado por el `Wal`, **nunca reutilizado** (ni tras truncar).
- El **commit en dos fases** del capítulo: re-validar → log_write de cada op (sync según política) → Commit + sync → apply. El `Commit` se sella ANTES de que el apply toque el store.
- El **redo idempotente** es lo que permite re-aplicar sin saber qué sobrevivió: put idéntico = no-op, put divergente = overwrite, delete de lo ausente = no-op.
- La **parada limpia** del iterador ante corrupción/truncado/LSN-no-consecutivo + el `decodificar_wal` que grita el LSN aparente del daño.
- El **truncado con contrato** del llamador: los LSNs no se reinician; truncar lo no-durable PIERDE datos.
- `CadaEscritura` es la regla de oro; `SoloCommit` es la semilla del group commit. La diferencia en fsync es 4 vs 1, con el mismo resultado al hacer replay.
- El **`ApplyFallido` con la misma forma del cap. 27** y su `Display` cambiado: «replay_wal COMPLETA la transacción (arranque automático: cap. 29)».
- **Tocar el `ApplyFallido` del cap. 27 para que rescate la transacción**, es la inversión de la regresión del cap. 27 — la prueba de que el capítulo sirve.

## 28.12 Ojo, cuidado con…

- **Confundir commit con persistencia.** El commit del cap. 27 era en RAM; la durabilidad es del **registro `Commit` en el log + `sync`**, y por eso `ResumenCommitWal` lleva `lsn_commit`. Síntoma: alumno lee `ResumenCommit` del cap. 27 y asume que la tx está durable en disco.
- **Confundir `CadaEscritura` y `SoloCommit` con fsync de verdad.** `Wal::sync` es un CONTADOR en RAM; la `sync_all` REAL es la del `FilePager::sync` del cap. 12. Mide la política, no el disco.
- **Truncar lo no-durable, en serio.** `truncar_hasta_lsn(lsn)` es PODEROSO. Si pasas el LSN de una tx intermedia (apply a medias) y luego llamas a `replay_wal` sobre un store VACÍO, los nodos se pierden y las aristas quedan huérfanas (`RedoFallido { lsn: 6, ... }`). El test `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias` es la DEMOSTRACIÓN; el contrato del llamador en el doc es la PREVENCIÓN.
- **No confundir `LSN` con `posición en bytes`.** El primero es la dirección física del log, monótono, nunca reutilizado. El segundo cambia con el truncado. La identidad de un redo es la primera.
- **Olvidar la ordenación de las props.** El `encode_props` ordena las claves antes de serializar para que el CRC dé lo mismo. Si reordenas a mano o cambias el sort por un HashMap.iter, el roundtrip falla — porque el CRC del mismo nodo cambia.
- **Creer que la «semilla del group commit» es group commit.** No: es el `SoloCommit` con UN sync por tx. El group commit concurrente es OTRA cosa (varias tx compartiendo un fsync) y exige concurrencia real — cap. 30.

## 28.13 Pin de batalla

> *«"El log se escribe antes que el dato" no es una orden de paso. Es una promesa sobre la dirección de la recuperación. Si el log se escribe ANTES, la mitad desconocida del problema se resuelve releyendo. Si se escribe DESPUÉS, se resuelve deshaciendo — y deshacer dobla el log. La pregunta "¿UNDO o no?" se contesta ANTES de escribir la primera línea de código.»*

## 28.14 Si solo lees 30 segundos

La transacción del cap. 27 vivía en RAM; un cierre del proceso se llevaba la confirmación. El **Write-Ahead Log** lo arregla con tres decisiones: (1) el log se escribe y sella con un `Commit + sync` ANTES de que el apply toque el store; (2) cada registro lleva un **LSN** monótono y un CRC32; (3) el **redo idempotente** re-aplica lo confirmado y descarta lo no confirmado. Si el apply falla a medias, el log YA contiene el `Commit` y un `replay_wal` completa la transacción. `CadaEscritura` es la regla de oro (un fsync por cada log_write); `SoloCommit` es la semilla del group commit (un fsync por transacción). El truncado es con CONTRATO del llamador y los LSNs no se reinician.

## 28.15 Una historia pequeña

Cuando implementamos el cap. 27 por primera vez, los tests del `StoreQueFalla` se veían finos: el `commit` devolvía `ApplyFallido{indice: 2, aplicadas: 2}`, el `Display` del error adjetivaba «sin log no se pueden deshacer», y la transacción «no se completó». Los tests verdes. El módulo compilaba. Pero el `informe_acid()` del cap. 27 dejaba la **D** en `NivelGarantia::Ninguna` con un comentario al lado: «en RAM, no durable (cap. 28)». Y la **A** en `Parcial` con la coletilla: «frente a validación; NO frente a un fallo del apply real».

Eso no era un módulo roto. Era un módulo HONESTO. Y la honestidad es lo que el capítulo 28 viene a cambiar. Hoy, el `Display` de `WalError::ApplyFallido` lleva, en su última línea, la frase que invierte la regresión: «replay_wal COMPLETA la transacción (arranque automático: cap. 29)». Y el `informe_acid_post_wal()` cierra la **D** de `Ninguna` a `Parcial` con un LOG en mayúsculas. La tx que ayer se perdía en un cierre de proceso, hoy es recuperable con un replay. Es la promesa del write-ahead, hecha cumplir.

## Ejercicios resueltos

**1. ¿Por qué el `Begin` no se sincroniza y el `Commit` sí?**

Porque la ausencia de `Commit` ya cancela la transacción en el replay: si la tx se cierra sin Commit, no «ocurrió» del lado de la durabilidad. El `Begin` necesita quedar en el log para que la pasada 1 del `replay_wal` pueda contar la tx como «iniciada pero no confirmada» y descartarla, pero la ausencia de `Commit` ya implica eso. En cambio, el `Commit` es el MOMENTO de la durabilidad: si ese registro se pierde (corte de luz a mitad de su escritura), la tx NO se considera confirmada. Un `sync` antes del `Commit` no impide que el `Commit` mismo se pierda; lo que lo protege es que, o se escribe entero (y entonces el sync lo sella), o se escribe a medias y un crc/truncado lo detecta como cola rota. La asimetría es la razón por la que se enseñan dos políticas de `sync` distintas: el `Begin` no lo necesita; el `Commit` SÍ.

**2. ¿Por qué el `replay_wal` descartaBegin` Commit`s y replica operaciones, en ese orden y no otro?**

Porque la pasada 1 (`HashSet` de `tx_id` con `Commit`) resuelve la pregunta lógica («¿quién confirmó?») sin importar el orden temporal: el `Commit` es lo que decide, no el orden de las ops. La pasada 2 (en orden de LSN) ejecuta la respuesta en el orden FISICO del log, lo que garantiza dos cosas: (a) si dos txs tocaron el mismo nodo (algo que no pasa hoy sin concurrencia, pero el log ya lo admite), el replay las aplica en el orden en que se firmaron; (b) si una tx posterior depende de una anterior (una arista al nodo que la primera crea), el LSN menor garantiza que la dependencia se aplica antes. La regla «orden de LSN = orden de aplicación» es el reflejo físico del orden lógico «orden de Commit» dentro de cada tx.

**3. ¿Por qué los LSNs no se reinician tras truncar?**

Porque la identidad de un redo es su LSN, no su posición en bytes. Si el `Wal` llevara la cuenta de los LSNs restantes (digamos, que tras truncar se reinicia en 1), dos transacciones en distintas épocas del log podrían tener LSNs coincidentes; un redactor que viera los dos no sabría distinguirlos, y cualquier cosa que indexe por LSN (la `dirty page table` que ARIES usará, nuestro propio `RedoFallido { lsn, … }`) confundiría el presente con el pasado. La monotonía del LSN hace al log AUTO-REFERENCIABLE: «el nodo 7 que escribí en `lsn=42`» es una oración estable a través del truncado y del paso del tiempo.

## Ejercicios propuestos

**Esencial.** Predice, ANTES de ejecutar, el contenido y los LSNs de un log de UNA transacción `WalTransaccion` que crea 3 nodos y 2 aristas y confirma, con `CadaEscritura`. ¿Cuántos registros tiene? ¿Cuál es `lsn_commit`? ¿Cuántos `syncs` se cuentan? ¿Qué tipo de cuerpo tiene cada uno? Compruébalo con `commit_aplica_todo_y_es_durable` y `politica_por_defecto_y_syncs_por_escritura`. Criterio: predicción exacta de número de registros + `lsn_commit` + número de syncs.

**Intermedio.** Sobre un log con 12 registros (3 Begin, 6 Operacion, 3 Commit), TODOS con su CRC y LSN válidos, pero donde la transacción 2 NO tiene Commit (la operación 6 de 6 fue su última), calcula a MANO el `InformeReplay` sobre un store vacío (confirmadas, descartadas, reaplicadas) y el `node_count`/`edge_count` del store renacido. Compruébalo con `transacciones_intercaladas_solo_la_confirmada_sobrevive`. Criterio: informe + cuentas exactas y respuesta a «¿se tocó el store con la abortada?».

**Experto.** Toma un log del capítulo, corrompe UN byte en la mitad del cuerpo de un registro y PREDECIR qué devuelve `decodificar_wal` (tipo de error, `lsn` reportado), qué hace `WalIterator`, y qué ve `replay_wal`. Compara con `crc_invalido_detectado` y `corrupcion_al_inicio_el_replay_para_en_el_prefijo_integro`. Criterio: tipo de error + `lsn` correcto + semántica recuperación/estricto + afirmación de que el replay sobre store pre-poblado no duplica.

## Para profundizar

- **Mohan, Haderle, Lindsay, Pirahesh y Schwarz, «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging»**, ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162 (DOI 10.1145/128765.128770). El paper donde la decisión commit-marker-antes-del-apply se formaliza como la mitad REDO de ARIES, y la otra mitad (UNDO + Analysis) cierra la recuperación completa. El capítulo 29 es la otra mitad.
- **Jim Gray, «Notes on Database Operating Systems»**, IBM Research Report RJ 2188, 1978. La huella de la bitácora como columna vertebral de la recuperación en System R. La idea del LSN como ordinal nace aquí.
- **Bernstein, Hadzilacos y Goodman, «Concurrency Control and Recovery in Database Systems»**, cap. 11 («Logging and Recovery»), Addison-Wesley 1987. Los algoritmos de recovery usando LSN y la tabla de páginas sucias: la lectura más densa del tema.
- **Lampson y Sturgis, «Crash Recovery in a Distributed Data Storage System»**, Xerox PARC 1976. El precursor cuidadoso: la versión anterior a la invención del LSN, con un protocolo que también funciona pero es más caro.
- **«Database Internals» (Alex Petrov)**, capítulo 7. Layout del WAL en bases reales (PostgreSQL, MySQL InnoDB) y los detalles de la persistencia que aquí hemos modelado con un contador.
- **CMU 15-445 (Intro to Database Systems)**, Lecture 17 — «Write-Ahead Logging», con el proyecto de BusTub que implementa un WAL REAL (con file system, log sequence, y todo lo que el cap. 29 promete).
- **Código fuente de SQLite** (`pager.c`, `wal.c`): la implementación de un WAL real en producción, con sus group commit, su checkpoint y su truncate.

## Mini-diálogo: en guardia nocturna

> — O sea, que el WAL es como un cuaderno de notas antes de hacer un cambio.
>
> — Un poco más serio: es un cuaderno de notas con un sello notarial. Cada anotación lleva un número correlativo, un checksum, y la garantía de que el cuaderno se firma EN SECO antes de tocar el original. Si la firma llega al final pero el original se quedó a medias, relees el cuaderno y completas. Si la firma NUNCA llegó, el original nunca se tocó.
>
> — ¿Y por qué COMMIT antes y no después?
>
> — Porque entonces el cuaderno contiene la historia ANTES de que la historia se ejecute. Releer es «haz otra vez lo que el cuaderno dice». Si el COMMIT fuera al final, el cuaderno contendría la historia DESPUÉS de que la historia se ejecutó — y releer no terminaría lo que se quedó a medias, tendría que DESHACERLO. La mitad de la maquinaria de undo es justo ésa: que el cuaderno tiene imágenes ANTES y DESPUÉS, y el sistema elige cuál mira.
>
> — Entonces lo de undo es para más tarde.
>
> — Sí. El cap. 29. Cuando ya tengamos el cuaderno en disco y el motor abra la base de datos, ARIES completo: Analysis (averigua qué quedó a medias), Redo (lo que el cuaderno dice que se confirmó), Undo (lo que el cuaderno dice que se abortó). La Parte VI del libro es eso.

---

*(Próximo capítulo: 29 — Recuperación después de un fallo (ARIES simplificado). Aquí el `replay_wal` se invocaba a mano. Ahora se enchufa al abrir la base de datos persistente: el log se escanea, el store se redibuja, y la pregunta "¿UNDO?" se responde con la misma claridad con la que este capítulo respondió la pregunta previa.)*
