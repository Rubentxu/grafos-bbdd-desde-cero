# Capítulo 29 — Recuperación después de un fallo (ARIES simplificado)

> *«Un log no vale lo que sus bytes: vale lo que puede reconstruirse con ellos tras un corte de luz.»*

## 29.0 La anécdota de la esquina

Hacia 1988, en el IBM Almaden Research Center, un grupo de cuatro ingenieros —Chandra Mohan, Don Haderle, Bruce Lindsay, Hamid Pirahesh y Peter Schwarz— publicó un paper interno de cuarenta páginas que llevaba un título para entonces modesto: «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollback Using Write-Ahead Logging». ARIES son las siglas de Algorithm for Recovery and Isolation Exploiting Semantics. Lo modesto era el título; lo que proponían era la receta que, treinta y cuatro años después, siguen usando Postgres, InnoDB, Oracle, SQL Server y una larga lista de sistemas que, cambiaron implementaciones, ajustaron políticas y pulieron heurísticas, pero no tocaron el esqueleto.

¿Por qué ese esqueleto triunfó? Porque antes de ARIES las bases de datos recuperaban su estado tras un crash de cuatro o cinco maneras distintas, cada una con su propio catálogo de bugs: unas no deshacían un apply a medias, otras descubrían el undo necesario cuando ya era tarde, las había que asumían no-steal por construcción y pagaban el precio en buffer pool bloqueado. Mohan y compañía miraron el problema de frente y se preguntaron —en una sola frase— qué tres cosas tiene que hacer un motor al despertar: **(1) reconstruir lo que sabe del mundo**, **(2) llevar el store EXACTAMENTE al estado del corte**, y **(3) retroceder lo que las transacciones rotas no debieron haber dejado**. Las llamaron, en orden, **Analysis**, **Redo**, **Undo**. Y convinieron en llamarlo ARIES, como al pastor germánico que guía a la base de datos de vuelta a casa tras el corte de luz.

Este capítulo es la versión sencilla de ese esqueleto, sobre `MemoryStore` y el WAL del cap. 28. No es un ARIES para producción: faltan los registros de compensación (CLR) y el log de before-image — los huecos que un motor de verdad cierra con una capa más. Pero es un ARIES HONESTO: las tres fases están, el orden está, y la pieza más interesante del capítulo —qué hacer cuando falta la imagen anterior de un elemento borrado— se documenta como una CUENTA en el informe de recuperación, no como un bug silencioso. La honestidad manda cuando los logs son uno de los pocos lugares donde mentir tiene consecuencias duraderas.

## 29.1 Objetivo

Al terminar este capítulo sabrás **cómo se reconstruye un motor transaccional tras un corte de luz**, y habrás construido las tres fases del algoritmo que se convirtió en estándar: Analysis, Redo, Undo. Cuatro piezas en `cap29_recuperacion.rs`:

1. **`analizar(wal)`** — la fase 1: recorre el log hacia delante y reconstruye la tabla de transacciones, los contadores `next_lsn`/`next_tx_id` y la *dirty element table* (qué elementos tocó una operación y en qué LSN).
2. **`redo` + `deshacer`** — las fases 2 y 3: re-aplicar todo en orden de LSN (incluidas las perdedoras robadas) y deshacer después, en orden inverso, sólo las perdedoras.
3. **`guardar_wal` + `cargar_wal` + `reabrir`** — el FICHERO del WAL: el `sync` del cap. 28 era un contador; aquí el fichero es el almacenamiento estable real.
4. **`Checkpoint` + `truncar_seguro` + `rotar_si_excede`** — el truncado ahora automatizado y la rotación por tamaño (la deuda que el cap. 28 dejaba firmada a mano).

Y el hito del capítulo: una perdedora con **steal** —es decir, que ya escribió al store antes de morir— se RECUPERA de un crash y deja el store como si nunca hubiera existido. Medido, no prometido.

## 29.2 Problema

El cap. 28 dejó la regla «commit-marker-antes-del-apply» hecha protocolo, y construyó lo más difícil: **`replay_wal` re-aplica las operaciones de las confirmadas en orden de LSN, idempotente**, y un apply a medias de una CONFIRMADA se completa por roll-forward. Funciona, y bajo la política no-steal del cap. 28 es incluso elegante: una perdedora no tocó el store (su staging vive en `WalTransaccion` y sólo aplica tras el Commit), y el undo es trivialmente vacío.

Pero elegante NO es lo que necesita una base de datos de verdad. Mira los tres problemas concretos que el cap. 28 dejó con la palabra *deuda* escrita al lado:

1. **El «disco» del WAL es un contador.** `Wal::sync()` del cap. 28 incrementa un entero. Los tests verifican que el contador sube; nadie verifica que los bytes lleguen a un sitio del que se pueda RECUPERAR. Cuando el proceso muere, el `Wal` se evapora: el próximo `Wal::new()` parte de cero.
2. **El truncado es a mano.** `truncar_hasta_lsn` firma el contrato «truncar lo no-durable pierde datos» — el llamador decide cuándo sabe que algo es durable. Lo que el operador humano decida se queda en el OPERADOR, no en el motor. Necesitamos un protocolo donde esa decisión sea de la base de datos, no del DBA a las tres de la mañana.
3. **No hay UNDO.** El staging vive en `WalTransaccion` y sólo aplica tras el Commit. Una transacción no confirmada NO dejó huella en el store, así que no hay nada que deshacer. Esto es la política **no-steal** — un buffer pool que sólo evacúa páginas limpias. Funciona, pero BLOQUEA: hasta que la transacción confirma o aborta, sus páginas sucias ocupan memoria, y bajo carga se traduce en latencia que sube.

Y hay un cuarto problema que el cap. 28 no escribió como deuda, pero que sale a la palestra en cuanto el tercero se arregla: **si un buffer pool REAL evacua páginas sucias para hacer hueco (steal)**, las perdedoras DEJARON escritura en el store. Esa escritura no fue autorizada por ningún Commit. Si la recuperación del cap. 28 se ejecuta, las perdedoras robadas SOBREVIVEN al reinicio. Eso es la pregunta que ARIES responde: ¿cómo levanto la base de datos después de que un motor real la dejara a medias — con partes confirmadas, partes perdedoras, y partes robadas?

## 29.3 Modelo mental

Piensa en un **museo que se reconstruye después de un incendio a partir del vídeo de las cámaras de seguridad**. Las cámaras no parpadean, escriben cada movimiento (el WAL); al reconstruir:

- **ANÁLISIS** = mirar el vídeo HACIA DELANTE y reconstruir el mapa: ¿quién estaba activo? ¿qué pieza tocó qué? ¿quién confirmó y quién se cayó a medias? Es la fase LECTORA: del vídeo deduces la tabla de transacciones, los contadores `next_lsn`/`next_tx_id`, y la *dirty element table* (qué elemento fue tocado por primera vez en qué LSN).
- **REDO** = REPETIR cada movimiento de las cámaras, en orden, hasta dejar la colección EXACTAMENTE como estaba en el instante del corte — incluido lo que una pieza rota empezó a mover sin terminar. Es la fase RE-ESCRITORA: re-aplica TODOS los registros — ganadoras y perdedoras — con la idempotencia del cap. 28. Al terminar, el store está en el estado del fallo.
- **UNDO** = borrar las huellas de las piezas que se cayeron a medias y no debieron seguir, en orden inverso al que aparecieron en el vídeo. Es la fase DESHACEDORA: de atrás hacia delante, compensa cada operación perdedora con su inversa lógica (un `PutNode` robado se borra; un `DeleteNode` robado se restaura desde su imagen anterior). Las huellas desaparecen; el museo queda como si las perdedoras nunca hubieran pasado por ahí.

```
                     ┌──────────────────────────────────────────┐
                     │ log en disco (reabierto = leer + escanear)│
                     └───────────────────┬──────────────────────┘
                                         │
                                         ▼
                            ANÁLISIS (hacia delante)
                            tabla de transacciones
                            contadores
                            dirty element table
                                         │
                                         ▼
                            REDO (hacia delante, TODO)
                            aplicar_para_redo idempotente
                                         │
                                         ▼
                            UNDO (hacia atrás, sólo perdedoras)
                            compensación lógica + before-image
                                         │
                                         ▼
                  store consistente = ganadoras confirmadas,
                                       sin huellas de perdedoras
```

**El truco del steal.** En un buffer pool real, las páginas sucias de una tx NO confirmada se evacuan al disco para hacer hueco — el steal. La fase de REDO las DEJA en el store (las re-aplica); la fase de UNDO las BORRA. Si sólo rehiciéramos las ganadoras (como hacía `replay_wal` del cap. 28), el undo no tendría de dónde partir: «¿qué hago si una página robada no está?». Redo + undo, en ese orden, es la única forma de tener una base coherente sobre la que retroceder.

**La frontera del before-image.** El vídeo de las cámaras enfocó el RESULTADO (la vitrina NUEVA), no el plano de la vitrina VIEJA. Si una perdedora borró un nodo y se llevó el contenido, no tenemos manera de saber qué ponía en la vitrina antes — y `operaciones_sin_before_image` lo CUENTA, no lo inventa. ARIES completo graba también el ANTES (before-images y CLR); aquí lo decimos y contamos: el motor HONESTO informa lo que no pudo hacer, no aborta silenciosamente y tampoco maquilla el resultado.

El momento ¡ajá!: «El redo no es "rehacer lo bueno": es "rehacer TODO, hasta dejar la colección EXACTA como estaba en el corte, incluidas las obras medio movidas". El undo es la pieza que quita las huellas de las perdedoras. Y un log que sólo guardó el resultado no puede restaurar lo que se borró — esa frontera, contada, es lo que separa un motor honesto de uno que miente.»

## 29.4 Primera solución

La solución ingenua es la del cap. 28, ya funcional: `replay_wal` re-aplica las operaciones de las confirmadas, en orden de LSN, con idempotencia. El operador humano lo ejecuta tras un crash; si tuvo cuidado, reconstruye lo bueno. La perdedora, sin commit marker, NO tocó el store (la política no-steal del cap. 27), y el undo es trivialmente vacío.

```
Fase (cap. 28): replay_wal(wal, store)
  → re-aplica Operaciones de tx con Commit
  → idempotente (un apply repetido es un no-op)
  → no toca perdedoras (no tocaron nada)
  → requiere Wal manualmente reanimado tras un crash
  → requiere truncado a mano si el log crece
```

Es una solución CORRECTA, SIMPLE y HONESTA bajo no-steal. Los tests la verifican (`replay_es_idempotente`, `replay_re_construye_lo_confirmado`). El propio cap. 28 lo escribe: «la alternativa (marker al final) exigiría UNDO para rescatar el apply a medias — eso es ARIES».

## 29.5 Sus límites

Tres límites que te empujan al cap. 29:

1. **Steal rompe no-steal.** El momento en que un buffer pool REAL evacua páginas sucias de una tx no confirmada para hacer hueco —ese es el steal—, las escrituras robadas están en el store. `replay_wal` del cap. 28 las IGNORA porque la tx no era ganadora; el store post-replay arranca con datos no autorizados. Síntoma: tras un crash con carga alta (muchas tx concurrentes, presión de memoria), las perdedoras sobreviven al reinicio.
2. **El `sync` del cap. 28 era un contador.** Cuando el proceso muere, `Wal::new()` parte de cero. Los `next_lsn`/`next_tx_id` se reinician; un nuevo Begin naciente con id 1 entra en el reinicio del log y se confunde con un id histórico. Modo de fallo: reorganización silenciosa de la historia, undo descompensando a la tx equivocada.
3. **Truncar a mano rompe Durabilidad silenciosamente.** `truncar_hasta_lsn` exige saber qué era durable. Sin un protocolo codificado (un Checkpoint que registre «hasta aquí todo es durable»), el operador trunca de más y la próxima recuperación falta de piezas.

La pregunta del capítulo: ¿qué pasa cuando un motor real, con buffer pool real y steal activo, se cae — y el operador no está para ejecutar `replay_wal` a mano?

## 29.6 Solución evolucionada: ARIES en tres fases

`recuperar(store, wal, antes)` ejecuta el esqueleto completo: análisis → redo → undo. Tres fases, en ese orden estricto, sobre el mismo log, sin coordinación entre ellas más que el flujo:

```rust
pub fn recuperar(store, wal, antes) -> Result<InformeRecuperacion, RecoveryError> {
    let analisis = analizar(wal);           // fase 1
    let operaciones_redo = redo(store, &analisis)?;   // fase 2
    let undo = deshacer(store, &analisis, antes);    // fase 3
    Ok(InformeRecuperacion { ... })
}
```

**Fase 1 — ANÁLISIS.** `analizar(&wal)` recorre el log hacia delante con `Wal::iter` (que PARA LIMPIO ante cola corrupta — el mismo contrato que el cap. 28 dejó para la iteración). Para cada registro, deduce:

- **Tabla de transacciones** (`EstadoTx::{Activa, Confirmada, Abortada}`): un `Begin` (o cualquier `Operacion`) la inserta Activa; un `Commit` la promueve a Confirmada (GANADORA); un `Rollback` la promueve a Abortada (PERDEDORA).
- **Contadores**: `next_lsn = max(lsn) + 1` y `next_tx_id = max(tx_id) + 1` se reconstruyen del contenido del log. Es la diferencia clave frente al cap. 28: los contadores no se «mantienen», se DEDUCEN.
- **Dirty element table** (`ElementoId → primer LSN que lo tocó`): es el análogo a la *dirty page table* de ARIES (Mohan et al. 1992, §3.2), pero a nivel de ELEMENTO del grafo — no de página de disco. `PutNode(n)`, `PutEdge(e)`, `DeleteNode(id)`, `DeleteEdge(id)` registran el elemento al que tocan; `sucias.entry(elem).or_insert(rec.lsn)` deja el PRIMER LSN — rehacer a partir de él con checkpoint es lo que ahorraría trabajo; aquí, sin checkpoint, el redo es conservador y todo lo recorre (`Analisis::primer_lsn_sucio` queda como información, no como punto de arranque real).

Al terminar la fase 1 tienes un mapa completo del estado del mundo justo antes del corte — sin haber tocado el store una sola vez. `primer_lsn_sucio` es el ancla del redo futuro cuando exista checkpoint; mientras tanto, el redo recorre todo.

**Fase 2 — REDO.** `redo(&mut store, &analisis)` re-aplica TODAS las operaciones en orden de LSN, ganadoras y perdedoras, usando `aplicar_para_redo` del cap. 28 (idempotente, ya usado por `replay_wal`). La diferencia CLAVE frente al replay del cap. 28:

```text
cap. 28 replay_wal:   re-aplica SOLO ganadoras  (política no-steal: 
                       las perdedoras no tocaron nada)
cap. 29 redo:         re-aplica TODO, ganadoras y perdedoras
                       (política steal: las perdedoras PUEDEN haber 
                       dejado huella — el undo necesita una base 
                       coherente sobre la que retroceder)
```

¿No es re-aplicar las perdedoras una LOCURA? Por la idempotencia de `aplicar_para_redo`, re-aplicar lo ya aplicado es un no-op; y si la perdedora sólo había escrito a través de steal (no-aplicada aún al store), el redo la INTRODUCE y el undo la BORRA después. Las dos fases son SECUENCIALES sobre el mismo registro de escrituras: redo deja el store en el ESTADO DEL FALLO; undo retrocede desde ahí hasta el conjunto de ganadoras. Sin redo-exhaustivo, el undo no tiene una base coherente.

**Fase 3 — UNDO.** `deshacer(&mut store, &analisis, &antes)` recorre los registros en orden INVERSO de LSN y deshace a las perdedoras:

```rust
for rec in analisis.registros.iter().rev() {       // HACIA ATRÁS
    if !perdedoras.contains(&rec.tx_id) { continue; }
    if let CuerpoWal::Operacion(op) = &rec.cuerpo {
        match op {
            PutNode(n)   => if store.get_node(n.id).is_some() {
                                store.delete_node(n.id);
                            }
                            informe.operaciones_deshechas += 1,
            PutEdge(e)   => /* análogo */,
            DeleteNode(id) => if store.get_node(*id).is_some() {
                                  // ya restaurado — idempotente
                              } else {
                                  match antes.get(&ElementoId::Nodo(*id)) {
                                      Some(Element::Node(n)) => 
                                          store.put_node(n.clone()),
                                      _ => informe.operaciones_sin_before_image += 1,
                                  }
                              },
            DeleteEdge(id) => /* análogo */,
        }
    }
}
```

La **compensación es lógica e idempotente**. ¿Por qué lógica y no física? Porque un log de solo after-image (el del cap. 28) no guarda las imágenes anteriores — sólo guarda cómo quedó cada elemento. Restablecer el estado anterior exige la imagen previa, que puede no estar disponible; lo que sí está es la INVERSA: un `PutNode` robado se deshace con un `delete_node` idempotente. El «antes» del `PutNode` es trivial: «no existía». El «antes» del `DeleteNode` NO — deshacer un borrado exige saber qué había, y eso sólo lo sabe un snapshot pre-robo (o un log con before-images, ARIES completo). Por eso el match: si `antes.get(...)` es `None`, `operaciones_sin_before_image += 1` — INFORMACIÓN, no error, no invención. La pieza del count es el capítulo honesto.

El **orden inverso** es la otra decisión: deshacer primero la arista y luego sus nodos evita que un `delete_node` arrastre aristas ajenas durante la cascada; deshacer «de atrás hacia delante» es lo que ARIES prescribe para que las compensaciones no interfieran entre sí.

El **informe** (`InformeRecuperacion`) agrega todo: cuántas ganadoras, cuántas perdedoras, cuántas operaciones se re-aplicaron, cuántas se deshicieron, cuántas quedaron sin imagen anterior (`operaciones_sin_before_image`), y los contadores reconstruidos (`next_lsn`, `next_tx_id`). Es la respuesta a la pregunta «¿qué se hizo al despertar?».

## 29.7 Solución evolucionada: el fichero y el checkpoint

ARIES sin fichero y sin checkpoint es un esqueleto, no un motor. El cap. 28 dejó dos deudas; el cap. 29 las cierra:

**El fichero del WAL.** `guardar_wal(&wal, path)` y `cargar_wal(path)` convierten los bytes del WAL en persistencia real. `std::fs::write` cierra el fichero y el sistema lo vuelca. `cargar_wal` reconstruye el `Wal` ESCANEANDO los bytes — `Wal::reconstruir(&bytes)` devuelve un Wal con `next_lsn = max(lsn) + 1` y `next_tx_id = max(tx_id) + 1`. Esto es la diferencia clave frente a `Wal::new()`: los contadores no se inventan, se DEDUCEN del prefijo íntegro.

```rust
pub fn guardar_wal(wal, path)    { std::fs::write(path, wal.as_bytes())? }
pub fn cargar_wal(path)         {
    let bytes = std::fs::read(path)?;
    Ok(Wal::reconstruir(&bytes))
}
pub fn reabrir(store, path, antes) {
    let bytes = std::fs::read(path).map_err(RecoveryError::Io)?;
    let wal    = Wal::reconstruir(&bytes);
    recuperar(store, &wal, antes)
}
```

El patrón de `fsync` riguroso ya existe (`FilePager::sync`, cap. 12; `BufferPool` ya sabe `flush`→`sync`); aquí «durabilidad real» = `guardar_wal` con la maquinaria de fichero del sistema. La separación `RecoveryError::Io` vs `RecoveryError::Redo {lsn, causa}` lleva la pregunta correcta (`¿fue el disco o fue el log?`) hasta el operador.

**El checkpoint que persiste los contadores.** `Checkpoint { hasta_lsn, next_lsn, next_tx_id }` no es una foto del store: es un REGISTRO del log (en ARIES real, un `WalRecord::Checkpoint`; aquí, una estructura aparte con el mismo papel). La parte crítica:

```text
hasta_lsn   = último LSN cuyo efecto es durable (todo ≤ a él se puede truncar)
next_lsn    = contador a reanudar tras el reinicio (congelado)
next_tx_id  = contador de TxId a reanudar (congelado)
```

¿POR QUÉ es crítico persistir los contadores? Porque truncar el log a vacío SIN guardarlos hace que `Wal::reconstruir` los ponga a 1 y REUTILICE identificadores. Un log recuperable no puede permitir eso: dos transacciones con el mismo id hacen que el análisis confunda cuál era cuál y el undo descompense a la que NO debía. El `Checkpoint::tomar(&wal)` los captura; el `truncar_seguro(wal, cp)` los usa para AUTOMATIZAR el truncado que el cap. 28 dejaba firmado a mano.

**La rotación por tamaño.** `rotar_si_excede(wal, umbral)` cierra la deuda «rotación por tamaño» del cap. 28 sin un scheduler: si `wal.as_bytes().len() > umbral`, tomar checkpoint y truncar. Política «una línea», decisión del llamador (en producción: cientos de MB, disparado por timer). La frontera `<=`/`>` del test lo verifica: por debajo del umbral no rota; justo al alcanzarlo (ya no `≤`) rota y trunca.

## 29.8 La frontera del before-image

`operaciones_sin_before_image` merece su propia sección porque es la pieza más honesta del capítulo. La pregunta que contesta: «¿qué pasa si deshacer requiere una imagen anterior que el log no tiene?»

```rust
DeleteNode(id) => {
    if store.get_node(*id).is_some() {
        // Ya restaurado por una pasada anterior — undo idempotente.
        informe.operaciones_deshechas += 1;
    } else {
        match antes.get(&ElementoId::Nodo(*id)) {
            Some(Element::Node(n)) => {
                let _ = store.put_node(n.clone());
                informe.operaciones_deshechas += 1;
            }
            _ => informe.operaciones_sin_before_image += 1,  // ← el hueco
        }
    }
}
```

Tres salidas por rama:

1. **El nodo YA está en el store** (porque una pasada anterior lo restauró, o porque no fue robado y un Confirmado legítimo lo dejó): se cuenta como deshecho y se sigue — la idempotencia es lo que permite que la recuperación sea re-ejecutable.
2. **El nodo NO está y `antes` lo tiene** (el llamador capturó `capturar_antes(&store)` ANTES del robo): se restaura con `put_node(n.clone())`. La imagen anterior es lo que te salva — pero exige que el LLAMADOR haya planeado la captura antes de que la perdedora tocara el store. Si capturas DESPUÉS, capturas la imagen de después, que no es la que quieres.
3. **El nodo NO está y `antes` NO lo tiene**: `operaciones_sin_before_image += 1`. INFORMACIÓN. No `panic!`, no `unreachable!`, no `unwrap_or_default()`. La recuperación REPORTA el hueco y devuelve el control. Es la decisión de diseño: el motor HONESTO deja que la decisión (¿continuar? ¿abortar?) sea del operador, no del motor.

**¿Por qué no abortar el reinicio por defecto?** Porque un borrado robado PERDEDOR sin before-image NO NECESARIAMENTE corrompe las ganadoras: la fase de undo puede haber restaurado todo lo demás correctamente, y este nodo en particular haber sido robado y borrado por una perdedora que abortó sin que nadie tomara la imagen previa. La aplicación podría haberlo recreado por su cuenta, o podría ser información de diagnóstico suficiente. La decisión es del operador.

**¿Por qué no inventar el dato?** Porque sería peor que abandonarlo. La base de datos "recuperaría" un nodo con `labels == ["DESCONOCIDO"]` y la siguiente consulta confiaría en él. La honestidad manda.

ARIES completo cierra este hueco con registros CLR (Compensation Log Records) que graban la imagen anterior al hacer el propio undo — escritura adicional al log, que se re-aplica con idempotencia completa. Es la pieza que AÑADE el undo a un log de after-image para hacerlo robusto. Aquí la dejamos como hueco documentado: el lector que quiera ir a ARIES real lee a Mohan, Don Haderle, Bruce Lindsay, Hamid Pirahesh y Peter Schwarz («ARIES» 1992).

## 29.9 Código completo ejecutable

Cuatro piezas en `liradb-workspace/crates/vol2-liradb/src/cap29_recuperacion.rs`, ~1.290 líneas, sin crates externas. Tres toques al cap. 28: `aplicar_para_redo` pasa a `pub(crate)` (reutilizado por el redo), `Wal::reconstruir(bytes)` y `Wal::next_tx_id()` se hacen públicas para que `cargar_wal` pueda deducir los contadores. Cero duplicación.

```rust
pub fn analizar(wal: &Wal) -> Analisis {
    let mut transacciones = HashMap::new();
    let mut sucias        = HashMap::new();
    let mut next_lsn = 1;
    let mut next_tx_id = 1;
    for rec in wal.iter() {                         // parada limpia
        next_lsn    = next_lsn.max(rec.lsn + 1);
        next_tx_id  = next_tx_id.max(rec.tx_id + 1);
        match &rec.cuerpo {
            CuerpoWal::Begin => { /* Activa */ }
            CuerpoWal::Operacion(op) => {
                for elem in ElementoId::de_operacion(op) {
                    sucias.entry(elem).or_insert(rec.lsn); // primer LSN
                }
            }
            CuerpoWal::Commit  => { /* Confirmada */ }
            CuerpoWal::Rollback => { /* Abortada */ }
        }
    }
    // ... ordena por primer LSN, deduplica ...
    Analisis { transacciones, sucias, next_lsn, next_tx_id, ... }
}
```

El REDO y el UNDO ya los cubrimos en la sección anterior; lo nuevo es la pieza administrativa:

```rust
pub struct Checkpoint {
    pub hasta_lsn:  Lsn,
    pub next_lsn:   Lsn,
    pub next_tx_id: TxId,
}

pub fn truncar_seguro(wal: &mut Wal, cp: &Checkpoint) -> usize {
    wal.truncar_hasta_lsn(cp.hasta_lsn)   // automatiza lo del cap. 28
}

pub fn rotar_si_excede(wal: &mut Wal, umbral_bytes: usize) -> Option<Checkpoint> {
    if wal.as_bytes().len() <= umbral_bytes { return None; }
    let cp = Checkpoint::tomar(wal);
    truncar_seguro(wal, &cp);
    Some(cp)
}
```

Y la re-valoración ACID que cierra el capítulo honestamente:

```rust
pub fn informe_acid_post_recovery() -> Vec<EntradaAcid> {
    vec![
        EntradaAcid { garantia: Atomicidad, nivel: Parcial,
          como_esta_hoy: "el arranque automático (análisis + redo + undo) 
                          repara un apply a medias y deshace lo no confirmado, 
                          incluso robado por steal; queda el before-image para 
                          deshacer borrados robados (ARIES completo lo cierra 
                          con CLR)",
          capitulo_que_la_cierra: 30 },                         // ← 29→30
        EntradaAcid { garantia: Durabilidad, nivel: Parcial,
          como_esta_hoy: "el WAL persiste a fichero y se reabre con recuperación: 
                          lo confirmado sobrevive al reinicio vía replay; el store 
                          de datos no tiene checkpoint independiente",
          capitulo_que_la_cierra: 37 },                         // ← 29→37
        // C e I: sin cambios (cap. 30).
    ]
}
```

Las dos flechas son la parte importante: A pasa del cierre 29 al 30 (queda el before-image), D pasa del 29 al 37 (queda el checkpoint del store de datos). El test que verifica las transiciones (`informe_post_recovery_actualiza_a_y_d`) compara contra `informe_acid_post_wal()` — el sistema de tipos lleva la trazabilidad.

## 29.10 Prueba de fuego

Tres tests explican el capítulo mejor que tres secciones más:

**TEST-TESIS** `undo_elimina_las_escrituras_robadas_de_una_perdedora`: una tx sin commit con `PutNode(0)`, `PutNode(1)`, `PutEdge(0, 0, 1)` logueadas. El store YA contiene esas escrituras (steal simulado aplicándolas a mano: `store.put_node(0)`, `store.put_node(1)`, `store.put_edge(...)`). `recuperar(store, wal, AntesImagenes::new())` devuelve `transacciones_perdedoras = 1, operaciones_undo = 3, operaciones_sin_before_image = 0` y DEJA EL STORE VACÍO. Como si la perdedora nunca hubiera existido. La pieza que el cap. 28 no sabía construir, vuelta y vuelta.

**TEST-FRONTERA** `borrado_robado_sin_before_image_se_reporta_no_se_calla`: un `DeleteNode(0)` robado SIN snapshot previo. El undo no encuentra `antes.get(0)` y CUENTA 1 en `sin_before_image`; el store queda con 0 nodos Y el informe dice `sin_before_image=1`. La honestidad del capítulo en un `assert`.

**TEST-PERSISTENCIA** `reabrir_recupera_lo_confirmado_tras_corte_de_luz`: una tx confirmada, `guardar_wal`, cierre del bloque (el «proceso muere»), `reabrir(&mut renacido, &path, &AntesImagenes::new())` devuelve `ganadoras = 1` y reconstruye 2 nodos + 1 arista con `labels == ["Person"]`. El flujo REAL de arranque verificado end-to-end.

Las **deudas del cap. 28**, una a una, verificadas:

- **El fichero** — `guardar_y_cargar_wal_roundtrip` (1161-1179): bytes idénticos antes y después de pasar por disco.
- **El checkpoint que persiste los contadores** — `checkpoint_y_truncar_seguro_no_reutiliza_lsns` (1112-1138): tras truncar, una nueva tx nace con `next_tx_id = 2` (no con 1 — los contadores persisten, no se reutilizan).
- **La rotación por tamaño** — `rotar_si_excede_trunca_solo_cuando_hace_falta` (1141-1156): por debajo del umbral no rota; justo al alcanzarlo (ya no `≤`) rota y trunca.
- **La contaminación detectada** — `redo_falla_ruidosamente_si_el_truncado_rompio_dependencias` (1231-1266): un truncado a mano que rompe el contrato (`truncar_hasta_lsn(3)` sobre un log que necesita el lsn 4) hace que la fase de redo falle con `RecoveryError::Redo { lsn: 5, ...InvalidEdgeEndpoints }` — el grito diagnóstico del cap. 28, heredado y tipado.

El síntoma si te saltas el capítulo es el de siempre: tras un crash con steal activo, escrituras de perdedoras sobreviven al reinicio; reabrir exige intervención manual; el log crece hasta OOM; truncarlo a ojo rompe Durabilidad silenciosamente; y no tienes un único punto de entrada que diga «le di el path, recuperé todo lo confirmado, aquí está el informe de qué no se pudo».

## 29.11 Repaso de la Parte VI: la cadena 27→28→29

La Parte VI tiene tres capítulos y un esqueleto común: cada uno ejecuta una pieza de ACID sobre el resto y deja una garantía heredada.

```
 27 ACID — TRANSACCIONES ──► 28 WAL ──────────► 29 RECUPERACIÓN (ARIES)
   Atomicidad               el cambio se         el arranque automático:
   Durabilidad RUDIMENTARIA  escribe en el WAL    • análisis reconstruye el mapa
   (un solo escritor,        antes que en la       • redo deja el store como
   staging → apply)          página de datos        en el instante del fallo
                             replay_wal a mano     • undo deshace las perdedoras
                             truncar a mano
   │                          │                    │
   └─ deuda: apply a medias   └─ deuda: steal      └─ deuda: before-image
      podría dejar store         rompe no-steal,     (sin imagen anterior
      inconsistente              perdedoras pueden    deshacer borrado robado
                                 dejar huella —       ⇒ ARIES completo: CLR)
                                 hace falta UNDO
```

Cada eslabón dejó una garantía heredada: el **27** fijó `Operacion` (la pieza que viaja por las tres fases sin duplicarse) y el staging+apply-after-commit; el **28** descubrió que `sync()` no es un contador sino un protocolo, y construyó `replay_wal`/`truncar_hasta_lsn` con contrato firmado; el **29** reúne: hereda `aplicar_para_redo` idempotente, `Wal::iter` con parada limpia, `Operacion` del 27 — y añade lo que ninguno tenía: la decisión explícita de CÓMO se reconstruye un motor tras un crash (las tres fases de ARIES), la pieza administrativa que automatiza el truncado (Checkpoint con contadores), y la honestidad de DECIR lo que no se pudo hacer (`operaciones_sin_before_image`). El método de la Parte VI en una frase: **la transacción es la promesa; el WAL es el cuaderno donde se anota; la recuperación es quien lee el cuaderno al volver del corte de luz y decide quién sigue y quién se queda en el suelo**.

Estas dos piernas — el `recuperar(store, wal, antes)` para un único motor y el `reabrir(store, path, antes)` para un motor con fichero— son también las que sostendrán el futuro inmediato del libro: el **cap. 30** añadirá concurrencia real (MVCC: varios lectores leyendo MIENTRAS un escritor escribe, sin lecturas sucias); el **cap. 36/37** añadirá el checkpoint del STORE DE DATOS (la pieza que falta para la Durabilidad completa).

## 29.12 Qué hemos sacrificado

1. **Sólo un escritor (`&mut dyn GraphStore`)**: la recuperación exige el préstamo exclusivo del store, igual que los caps. 27-28. Un cliente intentando leer MIENTRAS el recovery reescribe el store debe esperar; la concurrencia real es cap. 30. Documentado, no implementado.
2. **Sin CLR ni log de before-image (ARIES completo)**: la frontera del `DeleteNode`/`DeleteEdge` robado sin snapshot sigue ahí — `operaciones_sin_before_image` la cuenta. ARIES completo añade registros CLR y log de before-images para cerrar el hueco; aquí lo dejamos documentado como la pieza que el siguiente nivel añade. La razón: añadirla exige mover el undo a una fase con escri-tura adicional al log, que es materia del cap. 36 cuando se introduce el ciclo undo→redo→undo→page flush.
3. **Fuzzy checkpoint NO implementado (ARIES completo)**: el checkpoint aquí es EXACTO (congela el estado actual). Un motor real usa fuzzy checkpoint (la página-sucia-tabla periódica) para no pagar el «todo se para mientras se hace checkpoint» del fuzzy→exact. Lo nombramos y lo dejamos al lector que mire Mohan 1999 («Repeating History Beyond ARIES»).
4. **El log es append-only sin CRC de bloque**: el frame por registro del cap. 28 ya lleva el CRC32 (`crc32_simple`), pero si UN bloque de disco se corrompe a MITAD de registro, la iteración con parada limpia del cap. 28 CORTA antes; la tx confirmada con el byte corrupto se considera perdedora. Esto está bien para un WAL append-only, no lo está para un sistema que quiera recovery con bit-error-resistance — fuera de alcance.
5. **Group commit y `fsync` agrupado**: la política `SoloCommit` del cap. 28 ya está aquí (un fsync por tx), pero el GROUP COMMIT REAL (varias tx compartiendo un fsync) exige concurrencia — cap. 30.
6. **Recuperación distribuida (dos PCs, Paxos)**: nombrada y cortada. La pregunta «qué pasa si el recovery mismo se cae» la responden los algoritmos de consenso — fuera del alcance de este libro.

## 29.13 Cómo lo hace una BBDD real

- **PostgreSQL**: ARIES con las tres fases exactas (`XLOG` records con LSN, `xact` table, dirty page table en `pg_stat_get_db_*`, checkpoint en `pg_control` con `nextXid`). La pieza que el cap. 29 deja como `operaciones_sin_before_image` la cierra con `xl_invalid_page`/`FULL_PAGE_WRITES`: cada escritura al store lleva un backup completo de la página, lo que ARIES llama *before-image*. Modo completo, coste en disco: cada página modificada se escribe DOS veces (al log, a disco).
- **InnoDB (MySQL)**: ARIES sobre redo log (las escrituras previas a la página: `LOG_BLOCK_HDR_NO`, `LOG_CHECKPOINT` con `LOG_DYNHDR_CPN_NO` para el último checkpoint) y undo log (segmentos por tx; `TRX_UNDO_PAGE` con el estado anterior). La frontera del cap. 29 la cruzan con **undo log de verdad**: cada `DeleteNode` lleva su `update undo log` con la fila completa. Lo que aquí es «el log no la tiene, el informe la cuenta», allí es «el log SÍ la tiene, y deshacer restaura» — la pieza ARIES completa del cap. 29.
- **Oracle**: ARIES con `REDO LOG`/`UNDO TABLESPACE` y «flashback» (queries «AS OF» sobre undo log). Las tres fases están; lo que cambia es la granularidad (undo por segmento, no por registro).
- **SQL Server**: ARIES bajo el nombre «ARIES-style recovery» con `LSN` por byte, `recovery interval`, `fuzzy checkpoint` y la `version store` para versiones de fila (cercano a MVCC pero distinto).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: sobre el log `Begin(1) Op(2) DeleteNode(0) Commit(3)`, ¿cuántos `operaciones_deshechas` cuenta el undo? ¿Y si lo ejecutas dos veces seguidas (idempotencia)? Pista: la «Op» toca el elemento 0 con `DeleteNode`, una CONFIRMADA — no es perdedora, no se deshace.
- *Intermedio*: tu base tiene el log `{Tx=1: PutNode(0), PutNode(1), PutEdge(0,0,1)}` y el store YA tiene esas escrituras (steal). La tx 1 está SIN Commit (proceso muere). ¿Qué predice el undo si capturas `antes` DESPUÉS del robo? Pista: la imagen que capturas es la de después, no la de antes — `operaciones_sin_before_image` se dispara.
- *Experto*: implementa `recuperar_sin_truncar(wal)` (análisis + redo + undo sin habilitar el truncado del log). ¿Qué tiene que devolver para que un test verifique que `wal.record_count()` post-recovery es el MISMO que pre-crash? Pista: el log no se toca en ninguna de las tres fases; el truncado es opt-in.

## 29.14 Lo que te llevas

- **ARIES en tres fases**: análisis reconstruye el mapa, redo deja el store como en el fallo, undo deshace las perdedoras en orden inverso. El esqueleto de Mohan et al. 1992, treinta y cuatro años después.
- **El REDO no es «rehacer lo bueno»**: es «rehacer TODO hasta el estado EXACTO del fallo, incluidas las escrituras robadas de perdedoras» — la base sobre la que el undo puede operar.
- **La compensación es lógica e idempotente**: un `PutNode` robado se deshace con un `delete_node` idempotente; un `DeleteNode` robado exige la imagen anterior que un log de solo after-image no tiene.
- **`operaciones_sin_before_image` es información, no error**: el motor HONESTO reporta lo que no pudo hacer; no aborta por defecto y tampoco maquilla el resultado.
- **El checkpoint persiste los CONTADORES, no sólo el LSN**: truncar a vacío sin guardar `next_tx_id` los reutiliza — la diferencia entre recovery y corruptor silencioso.
- **El fichero del WAL** convierte el `sync` (contador) en `guardar_wal` (bytes en disco) — y `cargar_wal` reconstruye los contadores del prefijo ESCANEANDO, no manteniéndolos.
- **`recuperar` y `reabrir`**: uno opera sobre un `Wal` ya cargado, el otro lee del fichero y reconstruye. El flujo REAL de arranque en una sola llamada.
- **`informe_acid_post_recovery` continúa la honestidad**: A pasa de cierre 29 a 30 (queda before-image), D de 29 a 37 (queda checkpoint del store). El sistema de tipos lleva la trazabilidad.

## 29.15 Ojo, cuidado con…

- **Confundir `replay_wal` del cap. 28 con `recuperar` del cap. 29**. El replay sólo rehace ganadoras (política no-steal); el recuperar ejecuta las TRES fases. Si una base de datos con steal activo «se recupera» con `replay_wal`, las perdedoras sobreviven al reinicio — inconsistencia lógica.
- **Pensar que el undo es simétrico al redo**. No: `PutNode`/`PutEdge` se deshacen con un borrado idempotente (el «antes» es trivial); `DeleteNode`/`DeleteEdge` exigen una imagen anterior que el log no guarda. La frontera es asimétrica, no universal.
- **«Checkpoint = foto del store»**. Falso aquí. Es un REGISTRO del log («todo lo anterior a este LSN es durable Y los contadores son estos») y su segunda parte — los contadores — es la sorpresa administrativa que truncar a vacío sin ella paga caro.
- **Tomar el snapshot del store POST-ROBO**. `capturar_antes` sobre un store que ya tiene el borrado captura la imagen de DESPUÉS, no la de ANTES. En un sistema real el snapshot es pre-tx (o viene de un log de before-images).
- **`operaciones_sin_before_image` como bug**. Es información. El motor decidió reportar y seguir; la decisión de abortar es del operador.
- **Confundir `Rollback` (cap. 27) con `deshacer` (cap. 29)**. El primero escribe el marker y vacía el staging de UNA tx en vivo; el segundo recorre el log al revés y compensa a las perdedoras tras un crash. Distintas.

## 29.16 Pin de batalla

> *«Un log que no cuenta lo que no supo deshacer no es recuperable — es un cuaderno elegante que miente cuando lo lees a las tres de la mañana.»*

## 29.17 Si solo lees 30 segundos

ARIES tiene tres fases: **análisis** reconstruye del log la tabla de transacciones, los contadores y la dirty element table; **redo** re-aplica TODO en orden de LSN (ganadoras Y perdedoras) con `aplicar_para_redo` idempotente — el store queda como en el instante del fallo; **undo** recorre al revés y deshace las perdedoras, `PutNode`→`delete_node` idempotente, `DeleteNode`→restaurar imagen anterior (si la hay; si no, CONTAR, no inventar). El **fichero del WAL** persiste los bytes y `cargar_wal` reconstruye los contadores ESCANEANDO; **`Checkpoint`** congela durable Y contadores — sin `next_tx_id` persistido, truncar a vacío REUTILIZA identificadores. `recuperar` opera sobre un `Wal`; `reabrir` lee del fichero. **ACID post**: A 29→30 (queda before-image), D 29→37 (queda checkpoint del store).

## 29.18 Una historia pequeña

La migración de este capítulo casi se pierde en el camino, como cuenta MIGRATION-PATTERN §34. Los tests aparecían en verde, pero dos fallaban en `next_lsn` y `operaciones_redo`. ¿Quién mentía, el test o el código? Resultó que el test ASUMÍA que `WalTransaccion::rollback` logueaba las operaciones (luego undo las compensaría); pero rollback sólo escribe el marker `Rollback` — el staging del cap. 27 nunca llegó al log. La distinción «staging vs log» del cap. 27, que parecía un detalle, salvó la calibración: un rollback deja `Begin+Rollback` SIN operaciones; el redo de una tx rollbackeada es vacío. La moraleja quedó escrita en la bitácora: **los tests que comparan con la realidad se calibran recorriendo el código, no con lo que suena razonable** — el mismo `assert_eq!` del cap. 26, ahora en WAL/Recovery.

Y un detalle de calidad que se cazó al integrar: el `RecoveryError::Redo { lsn, causa }` movía `causa` al hacer `match err { Redo { causa, .. } }` y luego `err.to_string()` fallaba por uso de valor movido (E0382). El fix fue comprobar el `Display` ANTES del match por valor — la clase de bug que el compilador te cuenta si lo escuchas.

## Ejercicios resueltos

**1. ¿Por qué el REDO re-aplica también las perdedoras, si luego el UNDO las va a borrar?**

Porque el undo necesita una base coherente sobre la que operar. Si el redo sólo aplicara ganadoras, el store post-redo tendría MENOS datos de los que tenía en el corte (las robadas faltarían), y el undo no encontraría dónde borrar — quedaría incoherente. Re-aplicar lo ya aplicado es un no-op por la idempotencia de `aplicar_para_redo`; re-aplicar lo robado y luego DESHACERLO es lo que cierra el ciclo. Es la diferencia entre un replay optimista (cap. 28, no-steal: re-aplicar los confirmados es suficiente) y un replay GENERAL (cap. 29, steal: re-aplicar todo y retroceder las perdedoras).

**2. ¿Por qué `truncar_seguro` exige un `Checkpoint` con `next_lsn` Y `next_tx_id`, no sólo el `hasta_lsn`?**

Porque tras truncar el log a vacío, `Wal::reconstruir` lo reanimará ESCANEANDO los bytes. Si el log está vacío, los contadores se ponen a 1 por defecto — y la siguiente transacción nacería con id 1, REUTILIZANDO un identificador histórico. Un log recuperable no puede permitir eso: dos tx con el mismo id hacen que el análisis confunda cuál era cuál y el undo descompense a la que NO debía. `Checkpoint::tomar` congela ambos contadores con su valor real; `truncar_seguro` los respeta (al reconstruir de un log vacío se usará un valor por defecto DOCUMENTADO, y `cargar_wal` testea ese caso). Es un detalle que parece administrativo y es la diferencia entre recuperar y corromper.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin mirar el código, enuncia las tres fases de ARIES y para qué sirve cada una. Sobre el log `[Begin(1), Op(2 PutNode(0)), Commit(3), Begin(4), Op(5 PutNode(1))]` (la segunda tx abandonada), predice A MANO `analisis.ganadoras`, `analisis.perdedoras`, `analisis.next_lsn`, `analisis.next_tx_id`, `analisis.sucias`. Verificación con `analizar(&wal)` — debe coincidir. *Pistas*: (1) `next_lsn = max(lsn) + 1` no `max(lsn)`; (2) la tx abandonada entra Activa en el primer registro; (3) la dirty table guarda el PRIMER LSN de cada elemento. *Criterio*: predicción idéntica a `analisis_reconstruye_tabla_de_transacciones_y_contadores` (873-905).

**Intermedio (analizar — spacing caps. 27 y 28).** Sobre la cadena de 12 con sólo inserciones en una sola tx (12 nodos + 11 aristas en orden): predice cuántas operaciones aplicaría la fase de redo y si el undo sería vacío. Da el caso de UNA operación de borrado confirmada (paso 11, arista 10→11) y predice la diferencia. Explica por qué el replay del cap. 28 difiere del redo de ARIES en qué (la pregunta central). Verificación con `red_recorre_todas_las_operaciones_con_contador` (1081-1107): el voltímetro debe cuadrar con la predicción. *Pistas*: (1) `aplicar_para_redo` idempotente cuenta cada operación UNA vez, no varias; (2) el undo sólo deshace perdedoras — una CONFIRMADA no entra; (3) la tx abandonada (sin Commit) deja todas sus operaciones en `operaciones_deshechas`. *Criterio*: distingue store post-crash con steal (las perdedoras están ahí, undo las borra) de store post-crash sin steal (no están, undo vacío).

**Experto (crear — bridge retrieval al cap. 28).** Reconstruye desde la memoria el flujo completo «store se cae → DBA lo reabre → todo lo confirmado vuelve, lo perdedor no», citando en orden las llamadas (`reabrir` → interno `cargar_wal` → `Wal::reconstruir` → `recuperar` → interno `analizar` + `redo` + `deshacer`), diciendo para cada una de qué pieza del cap. 28 viene el material (`Wal::iter` con parada limpia, `aplicar_para_redo`, `truncar_hasta_lsn`, `informe_acid_post_wal`) y qué pieza NUEVA añade el cap. 29. Implementa `recuperar_sin_truncar(wal)` que ejecute sólo análisis + redo + undo sin habilitar el truncado, y un test que la use para verificar que `wal.record_count()` post-recovery sigue siendo el MISMO que pre-crash (la operación no toca el log — separación clara). *Pistas*: (1) qué firmas del cap. 28 reutilizas sin tocar (la mayoría) y cuál automatizas (`truncar_hasta_lsn` → `truncar_seguro(wal, cp)`); (2) `Wal::next_tx_id()` se hizo público para este capítulo — úsalo; (3) la re-valoración ACID dispara A: 29→30 y D: 29→37. *Criterio*: citación correcta de TODOS los puentes al cap. 28 + la nueva función compila y su test pasa + la re-valoración coincide con `informe_post_recovery_actualiza_a_y_d` (1287-1310).

## Para profundizar

- **C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh, P. Schwarz, «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollback Using Write-Ahead Logging»**, ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162 (DOI 10.1145/128765.128770) — el paper original, cuarenta páginas de algoritmo. Las tres fases, la dirty page table, los CLR, el fuzzy checkpoint, todo escrito en una prosa densa que sobrevive.
- **C. Mohan, «Repeating History Beyond ARIES»**, en «VLDB 1999 / ICDE 1999» — la evolución: LRU-K, fuzzy checkpoint, registros redo-only/undo-only, modificaciones posteriores. La misma arquitectura con piezas más finas.
- **T. Haerder, A. Reuter, «Principles of Transaction-Oriented Database Recovery»**, ACM Computing Surveys 15(4), 1983, pp. 287-317 (DOI 10.1145/322290.322291) — el análisis de las cuatro políticas (steal/no-steal × force/no-force) que el cap. 29 recoge indirectamente. Anterior a ARIES; el cuadro mental de «qué sacrifica cada política» viene de aquí.
- **R. Ramakrishnan, J. Gehrke, «Database Management Systems» (3.ª ed., McGraw-Hill 2003)**, capítulo 18 — la presentación académica estándar de recovery con WAL y ARIES simplificado; el libro de texto del que sale el capítulo conceptual.
- **A. Silberschatz, H. F. Korth, S. Sudarshan, «Database System Concepts» (7.ª ed., McGraw-Hill 2020)**, capítulo 17 — el complemento: cover recovery con buffer management detallado.
- **PostgreSQL internals: `xlog.c`, `xact.c`, `pg_control`** — la implementación ARIES de PostgreSQL leyendo el código; los nombres difieren (`XLogRecord` por `WalRecord`), el esqueleto no.
- **InnoDB internals: `log0log.cc`, `trx0undo.cc`** — la otra implementación ARIES; undo log con before-image completo, lo que aquí es hueco documentado.

## Mini-diálogo: en guardia nocturna

> — O sea, ¿recuperar es replay más algo?
>
> — No. Replay del cap. 28 SOLO rehace las confirmadas. Si un buffer pool real evacua páginas sucias de una tx no confirmada para hacer hueco — eso es el *steal* — esas escrituras SE QUEDAN en el store. Si sólo rehaces las confirmadas, las robadas sobreviven al reinicio: la base «recuerda» cosas que nunca debió.
>
> — Y recovery las borra.
>
> — Recovery las borra, sí — pero solo después de RE-APLICARLAS. Es la pieza que suena rara: el redo deja el store EXACTAMENTE como estaba en el corte (incluidos los robos), y el undo retrocede desde ahí hasta las ganadoras. Sin redo completo, el undo no tiene base. Las dos fases son SECUENCIALES sobre el mismo registro de escrituras.
>
> — ¿Y si el undo no encuentra la imagen anterior de un borrado robado?
>
> — El capítulo lo CUENTA, no lo inventa. `operaciones_sin_before_image` lo reporta, la decisión de continuar o abortar es del operador. ARIES completo cierra el hueco con CLR y before-images; aquí lo nombramos y lo dejamos al siguiente nivel. La honestidad es lo que mantiene el log confiable cuando lo lees a las tres de la mañana.

---

*(Próximo capítulo: 30 — Snapshots, concurrencia y aislamiento. Recovery es un único escritor con `&mut dyn GraphStore` — ¿qué pasa si dos transacciones quieren recuperarse a la vez, o si un cliente está leyendo mientras el recovery reescribe el store? La MVCC y el group commit cierran la Parte VI — y en el Vol.III, el cap. 51 montará GraphRAG sobre las piernas de la Parte V con la durabilidad del WAL+recovery que acabamos de construir.)*
