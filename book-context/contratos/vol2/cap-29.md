# CONTRATO DE CAPÍTULO — Vol.II Cap. 29: Recuperación después de un fallo (ARIES simplificado)

> Rellenado a partir de `book-context/PLANTILLA-CONTRATO-CAPITULO.md`. Código
> ancla: `liradb-workspace/crates/vol2-liradb/src/cap29_recuperacion.rs`
> (~1.290 líneas, 20 tests en `tests_recuperacion` + 2 doctests, verificados
> ALL_GREEN — `cargo test -p vol2-liradb --lib cap29` → 20 passed). Decisiones:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §34. Prerrequisito del
> propio capítulo: `cap28_wal.rs` (la regla "el cambio se escribe en el WAL
> antes que en la página de datos" hecha protocolo, con `aplicar_para_redo`
> idempotente que aquí se reutiliza). Pregunta crítica del CORPUS (líneas
> 357-362): **"ARIES (Analysis-Redo-Undo) simplificado."** Este capítulo
> cubre la línea 46 del ToC (`manuscrito/vol2/tabla-de-contenidos.md`),
> "Recuperación después de un fallo", Parte VI. Gancho: cap. 30 (MVCC y
> aislamiento) cierra la Parte VI — cap. 37 (persistencia end-to-end) cierra
> Durabilidad. Paper fuente: C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh,
> P. Schwarz, «ARIES: A Transaction Recovery Method Supporting Fine-Granularity
> Locking and Partial Rollback Using Write-Ahead Logging», ACM TODS 17(1),
> 1992 (las tres fases y la dirty page table).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la regla "commit-marker-ANTES-del-apply"
  del cap. 28 (el log se escribe antes que la página; el Commit va antes
  del apply, así un apply a medias de una CONFIRMADA se completa por
  replay-roll-forward); el `WalRecord { lsn, tx_id, cuerpo }` con su
  `CuerpoWal::{Begin, Operacion, Commit, Rollback}` y la iteración `Wal::iter`
  con parada limpia ante cola corrupta; `aplicar_para_redo` idempotente (un
  apply repetido es un no-op, no un duplicado); `truncar_hasta_lsn` firma
  contrato: truncar lo no-durable pierde datos; ACID `Parcial` con cierres
  anotados (`cap27_transacciones::{EntradaAcid, GarantiaAcid, NivelGarantia,
  informe_acid_post_wal}`); el `&dyn GraphStore` como puerto (cap. 8); el
  `Operacion` que ya cruza caps. 27-28-29 sin duplicarse.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) **"recovery es replay del cap. 28"** — no exactamente: replay solo
  re-aplica ganadoras (la política no-steal del 27/28 elude el undo
  por construcción); aquí el REDO re-aplica TODAS (incluidas perdedoras que
  robaron escrituras) y existe UNDO que deshace a las perdedoras en orden
  inverso; (2) **"redo y undo son opuestos y contradictorios"** — no:
  REDO deja el store en el estado EXACTO del instante del fallo (incluido
  el robo), y UNDO retrocede desde ahí hasta un punto consistente con el
  Commit del log — son fases SECUENCIALES sobre el mismo registro de
  escritura; (3) **"checkpoint es una foto del store"** — aquí checkpoint
  es un REGISTRO del log ("todo lo anterior a LSN K es durable en el
  store") Y persiste los CONTADORES (la segunda parte es la sorpresa
  administrativa: truncar a vacío sin `next_tx_id` los reutiliza); (4)
  **"si deshaces los cambios robados se acabó"** — la frontera del
  before-image: un `DeleteNode` robado no se puede DESHACER con un log
  de solo after-image (no guarda el "antes"); `InformeUndo::
  operaciones_sin_before_image` lo CUENTA, no lo inventa; (5) **"ARIES
  es un algoritmo de IBM antiguo, hoy hay algo mejor"** — ARIES es el
  estándar de facto desde 1992: lo que cambió es su IMPLEMENTACIÓN
  (LRU-K, fuzzy checkpoint, registros de compensation con
  redo-only/undo-only), no su ESQUELETO (Analysis-Redo-Undo); (6) **"si
  no hay commit, basta con no aplicar"** — eso es NO-STEAL (cap. 27),
  válido pero CARO (buffer pool bloqueado); ARIES es la solución general
  que un buffer pool con steal (evacuar páginas sucias de tx no
  confirmadas para hacer hueco) necesita.
- **NO debe saber todavía**: el algoritmo LRU-K del checkpoint difuso, los
  registros de compensación (CLR) y el log de before-image (ARIES completo
  — sólo se nombra como el hueco que aquí se documenta); concurrencia real
  con varios escritores en paralelo (cap. 30); group commit con varias
  transacciones compartiendo un fsync (cap. 30); checkpoint del STORE DE
  DATOS independiente del log (cap. 36/37 — persistencia end-to-end);
  recovery distribuido (dos PCs, Paxos) — nombrado y cortado.

## 2. Conceptos (del grafo curricular)

- `present`: algoritmo **ARIES simplificado en tres fases** (Analysis
  → Redo → Undo, Mohan et al. TODS 1992); la fase de análisis reconstruye
  del log la **tabla de transacciones** (`EstadoTx::{Activa,
  Confirmada, Abortada}` → ganadoras/perdedoras), los contadores
  `next_lsn`/`next_tx_id`, y la **dirty element table** (`ElementoId ::
  {Nodo(id), Arista(id)} → primer LSN que lo tocó`, análoga a la dirty
  page table de ARIES a nivel de elemento); la política **steal** del
  buffer pool (evacuar páginas sucias de tx no confirmadas — la razón
  histórica por la que se necesita UNDO); la **compensación lógica e
  idempotente** en orden inverso de LSN (`deshacer`: PutNode → delete
  idempotente; DeleteNode → restaurar `AntesImagenes`; sin before-image
  → CONTAR, no inventar); el **log como fichero** (`guardar_wal` /
  `cargar_wal` — el `sync` del cap. 28 era un contador, aquí el fichero
  ES el almacenamiento estable real); la **reconstrucción por scan**
  (`cargar_wal` + `Wal::reconstruir` reabre el `Wal` escaneando bytes:
  reabrir no es "mantener contadores", es "leer y deducir"); el
  **checkpoint** (`Checkpoint { hasta_lsn, next_lsn, next_tx_id }` —
  congela durable Y los contadores, es la única forma de truncar a
  vacío sin reutilizar identificadores); el **truncado seguro
  automatizado** (`truncar_seguro(wal, cp)`: la operación que el cap. 28
  dejaba firmada a mano); la **rotación por tamaño**
  (`rotar_si_excede(wal, umbral)` = checkpoint disparado por bytes); el
  **informe de recuperación** (`InformeRecuperacion`: ganadoras,
  perdedoras, redo, undo, sin_before_image — el "qué se hizo al
  despertar"); la **re-valoración ACID** post-cap-29
  (`informe_acid_post_recovery`: A 29→30 [queda el before-image], D
  29→37 [queda el checkpoint del store]).
- `practice`: ACID (caps. 27-28); `WalRecord` y `Wal::iter` con parada
  limpia (cap. 28); `aplicar_para_redo` idempotente (cap. 28);
  `truncar_hasta_lsn` con contrato (cap. 28); `MemoryStore`,
  `GraphStore`, `&dyn GraphStore` (cap. 8); `Operacion` (cap. 27);
  `CapturaAntes` polimórfica (cap. 27 — aquí renombrado
  `AntesImagenes`); `EntradaAcid`, `NivelGarantia::Parcial`,
  `informe_acid_post_wal` (cap. 28).
- `consolidate`: idempotencia como propiedad cardinal de un log
  recuperable (el replay se ejecuta MUCHAS veces en la vida del
  sistema); "derivar, no llevar en cabeza" (`next_lsn`, `next_tx_id` se
  reconstruyen del log, no se persisten por separado); honestidad como
  valor (`operaciones_sin_before_image` es UN CONTADOR, no un error);
  el log append-only como memoria estable del sistema.
- `out_of_scope` (sólo nombrar): CLR y before-images (ARIES completo,
  la pieza que aquí se documenta como hueco); fuzzy checkpoint
  (página-sucia-tabla periódica); LRU-K; group commit real (cap. 30);
  recovery distribuido; checkpoint del store de datos (cap. 36/37);
  `lock_manager` y 2PL; paralelismo del recovery (`&dyn GraphStore` no
  es `Sync` — un único escritor por préstamo exclusivo `&mut`,
  idéntico a caps. 27-28).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge** (afirmaciones comprobables): (1) Enuncia las TRES fases
  de ARIES y para qué sirve cada una (Análisis = reconstruir el mapa;
  Redo = ir al estado del fallo; Undo = retroceder las perdedoras);
  (2) Explica por qué el redo de ARIES re-aplica también las
  perdedoras (porque el buffer pool pudo robar escrituras, y el undo
  necesita una base sobre la que operar) frente a la política
  no-steal del cap. 28 que eludía el undo por construcción; (3)
  Justifica el orden inverso de LSN en el undo (deshacer primero la
  arista y luego sus nodos evita que `delete_node` arrastre aristas
  ajenas, y deshacer "de atrás hacia delante" es lo que ARIES
  prescribe para que las compensaciones no interfieran); (4) Calcula
  la diferencia entre `truncar_seguro` CON y SIN `Checkpoint` (con
  checkpoint, los contadores se congelan y la rotación es segura; sin
  checkpoint, `Wal::reconstruir` sobre un log vacío los pone a 1 y
  REUTILIZA identificadores — corruptor silencioso); (5) Identifica
  la frontera del before-image: `PutNode`/`PutEdge` se deshacen con un
  `delete_node`/`delete_edge` idempotente (la "imagen anterior" es
  trivial: "no existía"), pero `DeleteNode`/`DeleteEdge` NO (deshacer
  un borrado exige saber qué había antes, y un log de solo
  after-image no lo sabe); (6) Distingue `recuperar` (en RAM,
  re-ejecuta el log sobre un store) de `reabrir` (desde fichero:
  leer + reconstruir + recuperar — el flujo de arranque real).
- **Skills**: (1) Tomar un `Checkpoint` de un `Wal` con sus
  operaciones confirmadas, llamar a `truncar_seguro`, y verificar que
  los LSN no se reutilizan abriendo una segunda transacción;
  (2) Escribir un test estilo `undo_elimina_las_escrituras_robadas`
  sobre un grafo propio y verificar las TRES propiedades
  (perdedoras eliminadas, sin before-image = 0, store idéntico a "la
  perdedora nunca existió"); (3) Construir un flujo abrir-cerrar-
  reabrir con `guardar_wal` + `cargar_wal` + `recuperar`, y verificar
  que lo confirmado vuelve y lo no confirmado no.
- **Wisdom** (decisiones y trade-offs): (1) Decide cuándo reportar un
  hueco (sin before-image) como dato del informe (aquí: siempre — la
  honestidad manda) vs cuándo sería correcto abortar el reinicio
  (sólo si la política de la aplicación lo exige); (2) Decide cuándo
  tomar un checkpoint (cada commit: simple y seguro; por tamaño: como
  aquí — la rotación por bytes cierra la deuda del cap. 28; por
  tiempo: lógica de producción real, fuera del alcance).

## 4. Modelo mental

- **El museo que se reconstruye después de un incendio a partir del
  vídeo de las cámaras**. Las cámaras grabaron cada movimiento (el
  WAL); la reconstrucción tiene que mirar la grabación hacia
  DELANTE para reconstruir quién estaba activo, qué tocó qué y qué
  confirmó (ANÁLISIS); luego REPETIR cada movimiento en orden, hasta
  dejar la colección EXACTAMENTE como estaba en el instante del corte
  — incluido lo que una pieza rota empezó a mover sin terminar (REDO);
  y por último BORRAR las huellas de las piezas que se cayeron a
  medias y no debieron seguir, en orden inverso al que aparecieron
  (UNDO). El inventario final tiene sólo lo confirmado; las huellas
  de las perdedoras, evaporadas. El **checkpoint** es una nota en
  el diario: "todo lo anterior a las 10:00 ya estaba en su sitio y
  la cámara puede borrar esa franja sin perder nada". La **rotación
  por tamaño** es la misma nota, pero puesta por un guardia que
  mira cuándo se llena el disco. Y la **frontera del
  before-image** es la cámara que sólo enfocó el resultado (la
  vitrina NUEVA) pero no tenía el plano de la vitrina VIEJA: si una
  perdedora la rompió, no sabes cómo era antes — y el museo lo
  reporta, no lo inventa.
- **Diagramas ASCII**:
  (a) las tres fases en flujo (log → Análisis → Redo → Undo → store
  consistente);
  (b) el truco del steal: página sucia evacuada mientras la tx no
  confirmó — el undo la borra del store (analogía del buffer pool);
  (c) el checkpoint como contrato entre lo durable y el prefijo
  truncable (LSNs a la izquierda del checkpoint: ya en el store; a
  la derecha: aún en el log; truncar lo de la izquierda pierde
  NADA, y los contadores se persisten para no reutilizar).
- **Momento ¡ajá!**: «El redo NO es "rehacer lo bueno": es "rehacer
  TODO, hasta dejar la colección EXACTA como estaba en el corte,
  incluidas las obras medio movidas". El undo es la pieza que
  quita las huellas de las obras que se cayeron a medias — y un log
  de solo after-image no puede deshacer un borrado robado, porque
  no grabó la imagen anterior. ARIES completo lo cierra con CLR y
  before-images; aquí el capítulo lo dice y lo cuenta.»

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap29_recuperacion.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | ARIES completo (Analysis-Redo-Undo), no redo del cap. 28 | El cap. 28 era no-steal puro: la perdedora NO había tocado el store, undo trivialmente vacío. Aquí el STORE puede tener escrituras de perdedoras robadas (steal) — hace falta UNDO. Y hace falta REPLAY COMPLETO del estado del fallo, NO solo de las ganadoras, para tener una base consistente sobre la que retroceder | Sólo `replay_wal` del cap. 28: el store no llega al estado del fallo (las perdedoras robadas faltan), el undo no tiene de dónde partir; o replay del cap. 28 + UNDO a mano: dos pasadas sin coordinación, riesgo de doble apply | Datos de perdedoras sobreviven al reinicio (inconsistencia lógica: el Commit no autorizó sus escrituras) | Mohan et al. 1992, TODS 17(1) §3 (las tres fases); banner cap29 18-54; `undo_elimina_las_escrituras_robadas_de_una_perdedora` (999-1035) |
| 2 | Redo re-aplica TODAS (incluidas perdedoras), no solo ganadoras | Si el redo solo rehace ganadoras, el store post-redo tiene MENOS datos que en el instante del fallo (faltan los robos de las perdedoras); el undo no puede DESHACER "lo que no está" — quedaría incoherente. Redo + idempotencia del `aplicar_para_redo` (cap. 28) hacen este "rehacer de más" un no-op sobre lo ya aplicado | Solo ganadoras en el redo: las perdedoras robadas siguen vivas, undo "no encuentra qué borrar", y un análisis honesto de qué había en el instante del fallo es imposible | Las escrituras robadas de una perdedora sobreviven al reinicio | `redo` (392-404); `redo_recorre_todas_las_operaciones_con_contador` (1081-1107) verifica 3 reaplicadas con el voltímetro |
| 3 | Undo en orden INVERSO de LSN (no por tx, sino por registro) | Deshacer por registro al revés preserva las dependencias: una arista creada por tx1 después de sus nodos se desharía ANTES que los nodos que la arista necesita para existir en la cascada. Por-tx sería más legible pero perdería la garantía de que las compensaciones no interfieren entre sí | Undo por-transacción (cada tx se deshace agrupada): más legible, pero una arista que referencia nodos aún no restaurados se queda "flotando" durante el undo | Borrados y puts se interfieren; inconsistencias intermedias no idempotentes | `deshacer` (447-505): iter `rev()` sobre `analisis.registros`; el comentario 438-446 lo documenta |
| 4 | Compensación LÓGICA e IDEMPOTENTE (PutNode → delete_node idempotente) | La recuperación puede ejecutarse VARIAS veces en la vida del sistema (un reinicio, otro reinicio…). Cada pasada debe ser un no-op sobre lo ya compensado. `delete_node` devuelve `bool` = "estaba"; el undo lo cuenta igual (`operaciones_deshechas += 1`) — el "ya estaba borrado" no es un fallo | Compensación física (escribir la imagen anterior a pelo): exige mantener un buffer de before-images por nodo y arista, y rompe el contrato "sólo after-image en el log". Aquí la compensación lógica es la ÚNICA forma de cerrar undo sin before-images | Undo no idempotente: la segunda pasada borra lo que la primera ya había "intentado" dejar — fail not loud, fail NOT safe | `deshacer` 462-471 (PutNode/PutEdge): `if store.get_node(n.id).is_some() { store.delete_node(n.id); }`; `recuperar_es_idempotente` (979-995) |
| 5 | `InformeUndo::operaciones_sin_before_image` SE CUENTA, no se oculta, no se aborta | La frontera honesta del after-image: deshacer un borrado robado exige la imagen anterior, que el log no lleva (es el motivo exacto por el que ARIES completo añade CLR/before-image). Un capítulo honesto REPORTA el hueco y deja que la decisión (¿abortar? ¿continuar?) sea del llamador, no del motor | `panic!` en el undo si falta la imagen anterior (fail loud): pero es información que el motor conoce en el momento del reinicio — perderla es perder el diagnóstico. O `unwrap_or(default)` silencioso: la base de datos "recupera" pero con un hueco sin documentar — `Redo falló` gritaba el cap. 28; este error es IGUAL de ruidoso, sólo cambia el verbo | Log silenciosamente incompleto: una base de datos "recuperada" con un nodo que debería estar y no está (y nadie lo sabe); o `panic!` que aborta el reinicio por una tx perdedora — caro y evitable | `deshacer` 483 (rama Some sin nodo + None en `antes`): `informe.operaciones_sin_before_image += 1`; `borrado_robado_sin_before_image_se_reporta_no_se_calla` (1062-1078); doc-info `InformeUndo` 408-415 |
| 6 | Checkpoint = ESTRUCTURA aparte (`{ hasta_lsn, next_lsn, next_tx_id }`), no registro del log | En ARIES el checkpoint es un REGISTRO del log (para que el análisis lo encuentre al despertar); aquí se modela aparte — su PAPEL es idéntico, su serialización dentro del log queda como simplificación documentada. La parte CRÍTICA es persistir `next_lsn` Y `next_tx_id`: truncar el log a vacío sin ellos hace que `Wal::reconstruir` arranque con 1 y REUTILICE identificadores — corruptor silencioso de la siguiente tx | Checkpoint que sólo guarda `hasta_lsn`: tras truncar el log a vacío, `Wal::reconstruir` pone `next_tx_id = 1` y la siguiente tx nace con id 1 — IDENTIFICADOR REUTILIZADO. Un log recuperable no puede permitir eso nunca | Reutilización de LSN/TxId tras truncate: dos transacciones con el mismo id, análisis confunde cuál era cuál, undo descompensa a la que NO debía | `Checkpoint` (682-718), doc 676-681 y 685-693; `truncar_seguro` (722-724) usa cp como contrato |
| 7 | `truncar_seguro(wal, cp)` AUTOMATIZA el truncado (era a mano en el cap. 28) | El cap. 28 firmó el contrato "truncar lo no-durable pierde datos" — la consecuencia operacional era que el llamador tenía que SABER qué era durable (un algoritmo). Aquí se codifica la PROMESA: si entregas un Checkpoint consistente, truncar hasta `hasta_lsn` ES SEGURO; si no, no toma el checkpoint | Dejar `truncar_hasta_lsn` directo: invariante "el llamador sabe lo que hace" se rompe en silencio (MIGRATION §34 lo subraya como clave); el `cp` lo convierte en API cerrada y verificable por tests | Truncado que rompe Durabilidad: el log pierde operaciones necesarias para el próximo recovery — el siguiente reinicio deshará transacciones que NO debió | `truncar_seguro` 722-724; `checkpoint_y_truncar_seguro_no_reutiliza_lsns` (1112-1138) |
| 8 | `rotar_si_excede(wal, umbral)` = checkpoint por bytes | Cierra la deuda "rotación por tamaño" del cap. 28 sin un scheduler: cuando el log supera N bytes, se toma un checkpoint y se trunca. Política "una línea" — quién decide cuándo rotar queda como decisión del llamador (en producción: cientos de MB, disparado por un timer) | Scheduler de tiempo: añade un reloj al sistema (tema de cap. 30, no aquí); un log sin rotación: crece hasta OOM — el "antes de": el log del cap. 28 ya podía truncarse, pero la AUTOMATIZACIÓN faltaba | Crecimiento ilimitado del log en producción; tests: `rotar_si_excede_trunca_solo_cuando_hace_falta` (1141-1156) verifica la frontera `≤` vs `>` | banner 64-66 (deuda rotación); `rotar_si_excede` (733-740) |
| 9 | `guardar_wal`/`cargar_wal` persisten los BYTES del log (no el `Wal` reanimado en RAM) | El `sync` del cap. 28 era un CONTADOR (la promesa que los tests verificaban); aquí el FICHERO es el almacenamiento estable real. `std::fs::write` cierra el fichero y el sistema lo vuelca — y el patrón de `fsync` riguroso ya existe (`FilePager::sync`, cap. 12). `cargar_wal` reconstruye el `Wal` ESCANEANDO los bytes (los contadores vuelven a deducirse del log) | "Persisto el `Wal`": imposible sin serialización específica; reabrir debe ser ESCANEO — esa es la propiedad que hace al log append-only RECUPERABLE (no se necesita saber dónde está cada cosa para encontrar todas) | `Wal::next_tx_id` y `Wal::lsn_siguiente` quedan a 0 tras reabrir → reutilización masiva de ids → undo descompensa a la transacción equivocada | `guardar_wal` (615-617); `cargar_wal` (624-627); `cargar_wal_reconstruye_contadores_sin_reutilizar` (1209-1226); MIGRATION §34 decisiones 4 |
| 10 | `AntesImagenes = HashMap<ElementoId, Element>` pasado por el llamador (NO generado internamente) | Capturar la imagen anterior al UNDO es responsabilidad del LLAMADOR: en un sistema real vendría de un snapshot pre-tx o de un log de before-images; aquí lo construye el usuario desde su clon pre-crash. El cap. 29 no IMAGINA capturas — las pide, y deja escrito el matiz (tomar el snapshot ANTES de que la perdedora toque el store, no después) | `capturar_antes` sobre el store post-robo: captura el estado de DESPUÉS del robo (la imagen que queremos es la de ANTES). El matiz está en `cap29_recuperacion.rs` líneas 167-170 y en el test del borrado robado con before-image (1043-1059), que construye el snapshot manualmente | Undo usa la imagen equivocada y "restaura" lo que ya estaba borrado — corrupción silenciosa | `AntesImagenes` (159, 167-170); `capturar_antes` (170-179); `undo_restaura_un_borrado_robado_con_before_image` (1039-1059) |
| 11 | `reabrir(store, path, antes)` = leer + reconstruir + recuperar | El flujo de arranque de una base de datos REAL, sin granos: tres pasos en uno. El llamador solo ve "le dí el path y el store vacío, recibí el store con las ganadoras". Internamente: IO (`std::fs::read`) → `Wal::reconstruir(bytes)` → `recuperar(store, &wal, antes)`. Dos fuentes de fallo claramente diferenciadas (`RecoveryError::Io` vs `RecoveryError::Redo`) | Que el llamador encadene los tres: duplicación obvia; un DOCTEST tutorial (558-586 y 638-660) muestra además que el flujo es el que un DBA haría a mano ante un corte de luz | Tres puntos de fallo sin tipar; un error de fs se confunde con un error de store | `reabrir` (661-669); doctests 557-586 y 636-660 |
| 12 | `RecoveryError::{Io, Redo}` con `source()` correctamente enlazado (`impl std::error::Error`) | Mismo patrón que el cap. 28 para errores tipados: distinción clara entre el "fichero no encontrado" (reabrir `path`) y el "log truncado rompiendo contrato" (redo de una arista huérfana). El segundo es el eco de `RecoveryError::Redo { lsn, causa }` — el lsn es la pista diagnóstica ("¿se truncó el log rompiendo el contrato de durabilidad?") | Error único `String`: pierde la pista del lsn y la distinción IO/lógica — el cap. 28 ya enseñó que la pregunta correcta "¿fue el disco o fue el log?" necesita tipos | Usuario no distingue "fichero no encontrado" de "log corrupto" → intenta arreglar el primero cuando el problema es el segundo | `RecoveryError` (184-214); `errores_display_y_std_error` (1269-1284); `redo_falla_ruidosamente_si_el_truncado_rompio_dependencias` (1231-1266) |
| 13 | `informe_acid_post_recovery()` RE-VALORA y DOCUMENTA las transiciones | Una sola fuente de verdad para "qué cubre este capítulo y qué queda". A pasa de cierre 29 a 30 (queda el before-image para borrar lo robado), D pasa de 29 a 37 (queda el checkpoint del store de datos). El test verifica las transiciones contra `informe_acid_post_wal()` — el SISTEMA de tipos lleva la trazabilidad | "Después del cap. 29 las garantías ACID están Cerradas" sin re-valoración: cualquier día, un cambio al log rompe una garantía y nadie lo dice | ACID reportado como Cerrado cuando sigue Parcial → el lector deja de exigir lo que falta | `informe_acid_post_recovery` (761-797); `informe_post_recovery_actualiza_a_y_d` (1287-1310) |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: la del cap. 28, ya funcional — `replay_wal` re-aplica las
  operaciones de las confirmadas; el llamador ejecuta replay tras un
  crash y reconstruye lo bueno. La perdedora, sin commit marker, NO
  tocó el store (el staging del cap. 27 sólo aplica al confirmar). Es
  una solución CORRECTA bajo no-steal y SIMPLE — un único escritor
  serializado, un log append-only, un replay idempotente.
- **Qué la rompe**: (a) el momento en que un buffer pool REAL evacúa
  páginas sucias de una tx no confirmada para hacer hueco (steal) —
  las escrituras robadas están en el store y replay del cap. 28 las
  IGNORA porque la tx no era ganadora; el store arranca con basura
  no autorizad; (b) el `sync` del cap. 28 era un contador (la
  promesa se verificaba por un entero, no por un fichero en disco);
  tras cerrar el proceso, el `Wal` se evapora; (c) truncar el log es
  decisión del operador humano (un `truncar_hasta_lsn` a mano con
  riesgo de romper Durabilidad); (d) los contadores `next_lsn`/
  `next_tx_id` no se reconstruyen — abrir = nuevo `Wal::new()`, lo
  que reinicia los ids.
- **Evolución visible (cap. 29 → cap. 28)**: `recuperar(store, wal,
  antes)` ejecuta las TRES fases de ARIES sobre el log (frente a
  `replay_wal` que sólo rehace ganadoras); `reabrir(store, path,
  antes)` encadena leer→reconstruir→recuperar (frente a un
  `replay_wal` huérfano sobre un `Wal` recién nacido); `Checkpoint`
  congela el truncado seguro (frente a `truncar_hasta_lsn` directo);
  `rotar_si_excede` cierra la rotación por tamaño (frente a la deuda
  documentada del cap. 28); `InformeRecuperacion` CUENTA `operaciones_
  sin_before_image` (frente al silencio del cap. 28 sobre lo que
  truncó). El test-tesis (`undo_elimina_las_escrituras_robadas`)
  DEMUESTRA con un store que ya contenía el robo que el undo lo deja
  limpio — la pieza que el cap. 28 no podía construir.

## 7. Prueba de fuego

- **TEST-TESIS** `undo_elimina_las_escrituras_robadas_de_una_perdedora`
  (999-1035): un WAL con una tx SIN Commit que escribió 2 nodos + 1
  arista; el store pre-crash YA contiene esas escrituras (steal
  simulado aplicándolas a mano); `recuperar(store, wal,
  AntesImagenes::new())` devuelve `transacciones_perdedoras = 1,
  operaciones_undo = 3, operaciones_sin_before_image = 0` y DEJA
  EL STORE VACÍO. La pieza que el cap. 28 no sabía construir
  — vuelta y vuelta.
- **TEST-FRONTERA** `borrado_robado_sin_before_image_se_reporta_no_
  se_calla` (1062-1078): un `DeleteNode(0)` robado SIN snapshot
  previo; el undo no encuentra `antes.get(0)` y CUENTA 1 en
  `sin_before_image` — el store queda con 0 nodos Y el informe grita
  `sin_before_image=1`. La honestidad del capítulo en un `assert`.
- **TEST-PERSISTENCIA** `reabrir_recupera_lo_confirmado_tras_corte_de_
  luz` (1182-1206): una transacción confirmada, `guardar_wal`, PROCESO
  MUERE (cierre del bloque), `reabrir(&mut renacido, &path,
  &AntesImagenes::new())` devuelve `ganadoras = 1` y reconstruye 2
  nodos + 1 arista; `nodo 0` con `labels == ["Person"]`. El flujo
  REAL de arranque verificado end-to-end.
- **Deudas saldadas (frente al cap. 28)**: el fichero (`guardar_wal` /
  `cargar_wal`), el checkpoint automático y la rotación por tamaño
  (`Checkpoint`, `truncar_seguro`, `rotar_si_excede`), el undo general
  (`deshacer` con compensación lógica + before-image cuando hay),
  el informe de recuperación (`InformeRecuperacion`), el reaprovisiona-
  miento de contadores tras reabrir (`cargar_wal_reconstruye_
  contadores_sin_reutilizar`).
- **Síntoma si el lector se salta el capítulo**: tras un corte de luz,
  la base de datos "recuerda" sólo lo que no había robado (si steal
  está activo, datos de perdedoras sobreviven al reinicio); el reinicio
  exige intervención manual (`replay_wal` a mano); el log crece sin
  rotación hasta OOM; truncarlo rompe Durabilidad silenciosamente; y
  no hay un único punto de entrada que diga "le dí el path y recuperé
  todo lo confirmado".

## 8. Trampas y errores comunes

1. **Confundir `replay_wal` del cap. 28 con `recuperar` del cap. 29**.
   Síntoma: tras un crash con steal activo, las escrituras de
   perdedoras sobreviven. Distinción: replay REHACE lo confirmado;
   recuperar EJECUTA las tres fases sobre un log potencialmente
   incompleto.
2. **Pensar que el undo "borra todo lo que la perdedora escribió"**:
   eso es sólo verdadero para `PutNode`/`PutEdge` (compensación
   lógica = `delete_node`/`delete_edge` idempotente). Para
   `DeleteNode`/`DeleteEdge`, deshacer = restaurar la imagen
   ANTERIOR — y sin before-image, ese "antes" NO EXISTE en el log de
   solo after-image. Si el lector cree que undo es simétrico al redo,
   diseñará mal el siguiente sistema.
3. **"Checkpoint = una foto del store"** — falso aquí. Checkpoint es
   un REGISTRO del log (contrato: "todo lo anterior a este LSN es
   durable") Y conserva `next_lsn`/`next_tx_id` — la segunda parte es
   la sorpresa administrativa sin la cual truncar a vacío REUTILIZA
   identificadores.
4. **Creer que `operaciones_sin_before_image` es un error** — es
   información. El motor lo reporta para que la decisión
   (continuar/abortar) sea del operador; no aborta por defecto, no
   inventa el dato.
5. **Tomar el snapshot desde el store POST-ROBO**: capturar_antes
   sobre el store que ya tiene el borrado captura el estado de
   DESPUÉS, no el de ANTES — la imagen se construye manualmente desde
   un clon pre-crash (test 1043-1059). El matiz importa: en un sistema
   real, el snapshot debe tomarse ANTES de que la perdedora toque el
   store, o venir de un log de before-images.

- **Precisión de lenguaje (glosario)**:
  *recuperación* (el flujo completo: análisis + redo + undo) vs
  *replay* (re-aplicar las operaciones — el redo del cap. 28);
  *redo* (re-aplicar TODO, ganadoras y perdedoras) vs *replay del 28*
  (sólo ganadoras, política no-steal); *undo* (deshacer perdedoras en
  orden inverso) vs *rollback* (la operación del cap. 27 que escribe
  el marker Rollback y VACÍA el staging — distintas);
  *ganadora* (confirmada: se mantiene) vs *perdedora* (sin commit o
  abortada: se deshace); *steal* (evacuar páginas sucias de tx no
  confirmadas) vs *no-steal* (no aplicar nada hasta el Commit — la
  política del cap. 28); *forwarding* / *roll-forward* (redo del
  cap. 28) vs *roll-back* (undo del cap. 29); *checkpoint lógico*
  (registro del log con hasta_lsn + contadores) vs *checkpoint del
  store* (foto del store, cap. 36/37); *before-image* (estado
  anterior de un elemento, necesario para deshacer borrados) vs
  *after-image* (el log del cap. 28: el estado resultante); *idempotente*
  (ejecutable N veces con el mismo efecto que una vez) vs *conmutativo*
  (orden-independiente); *parada limpia* (la del `Wal::iter` ante
  cola corrupta: se confía en el prefijo íntegro) vs *abortar
  ruidoso* (cuando un dato falta, `panic!` o `RecoveryError::Redo`,
  NO callar).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: sin mirar el código, enuncia las
  tres fases de ARIES y para qué sirve cada una; da un ejemplo
  mínimo de un log con 1 confirmada y 1 abandonada y predice el
  contenido de las TRES tablas del análisis (transacciones,
  contadores, dirty element table) — luego compáralo con el output
  de `analizar(&wal)` en una transacción preparada a mano. Pistas:
  (1) ¿qué marca el estado de una tx en el log?; (2) ¿qué
  contador se reconstruye del MAYOR LSN visto o del MAYOR + 1?;
  (3) ¿una `PutNode` o una `PutEdge` registra el elemento en la
  dirty table, o sólo la primera vez que lo toca? *Criterio*:
  coincide exactamente con `analisis_tabla_sucias_registra_
  primer_lsn_de_cada_elemento` (908-930).
- **analizar (intermedio — spacing caps. 27 y 28)**: sobre la cadena
  de 12 (sólo inserciones: 12 nodos + 11 aristas en una tx), predice
  A MANO cuántas operaciones aplicaría la fase de redo y si el undo
  sería vacío — luego ejecútalo y compara. Da UNA operación de borrado
  confirmada (paso 11) y predice la diferencia. Pistas: (1) ¿el
  replay del cap. 28 difiere del redo de ARIES en qué? (la pregunta
  central del capítulo); (2) ¿qué traería un undo a una confirmada
  con borrado?; (3) ¿y si esa misma cadena la hiciera una tx NO
  confirmada? *Criterio*: distingue store post-crash con steal
  (las perdedoras están ahí) de store post-crash sin steal (no
  están), y razona sobre la diferencia con el cap. 28.
- **crear (experto — bridge retrieval al cap. 28)**: reconstruye
  desde la memoria el flujo completo "store se cae → DBA lo reabre
  → todo lo confirmado vuelve, lo perdedor no", citando en orden las
  llamadas (`reabrir` → interno `cargar_wal` → `Wal::reconstruir`
  → `recuperar` → interno `analizar` + `redo` + `deshacer`),
  diciendo para cada una de qué pieza del cap. 28 viene el material
  (`Wal::iter` con parada limpia, `aplicar_para_redo`, `truncar_
  hasta_lsn`, `informe_acid_post_wal`) y qué pieza NUEVA añade el
  cap. 29. Implementa una variante `recuperar_sin_truncar(wal)` que
  ejecute sólo análisis + redo + undo sin habilitar el truncado, y
  un test que la use para verificar que `wal.record_count()` post-
  recovery sigue siendo el MISMO que pre-crash (la operación no
  toca el log — separación clara). Pistas: (1) ¿qué firmas no usas
  del cap. 28 al construir el recovery? (`truncar_hasta_lsn`
  quedaba CONTRATO; aquí se automatiza con `truncar_seguro`);
  (2) ¿qué `impl` ya tienes en `Wal` para reconstruir? (`iter`,
  `reconstruir`, `next_tx_id`); (3) ¿la re-valoración ACID qué
  cambio de cierres dispara respecto al cap. 28? (A: 29→30 por
  before-image; D: 29→37 por checkpoint del store). *Criterio*:
  citación correcta de TODOS los puentes al cap. 28 + la nueva
  función compila y su test pasa + la re-valoración ACID
  coincide con `informe_post_recovery_actualiza_a_y_d` (1287-1310).

## 10. Preguntas abiertas (gancho al cap. 30)

1. La recuperación aquí es un único escritor (`&mut dyn GraphStore`,
   mismo préstamo exclusivo que caps. 27-28) — ¿qué pasa si DOS
   transacciones quieren RECUPERARSE a la vez, o si un cliente
   está leyendo MIENTRAS el recovery reescribe el store? (cap. 30
   — concurrencia, MVCC, group commit.)
2. El checkpoint del log es ya robusto; ¿quién se encarga del
   checkpoint del STORE DE DATOS para que la siguiente recuperación
   pueda empezar DESDE un store pre-recuperado en vez de desde
   cero? (cap. 36/37 — persistencia end-to-end; D sigue Parcial
   hasta entonces.)
3. Antes de un crash, ¿pueden DOS lectores estar leyendo una
   MISMA proyección del cap. 26 a la vez? El `recuperar` exige
   `&mut` — incompatible con cualquier lector activo. (cap. 30 —
   snapshots MVCC: la foto sin escritor.)

- **Términos nuevos de glosario** (los registra `book-memory-keeper`):
  *recuperación* (analysis-redo-undo), *ARIES*, *fase analysis/redo/undo*,
  *dirty page table* (aqui dirty element table), *ganadora/perdedora*,
  *steal* (evacuar páginas sucias), *compensación lógica*,
  *before-image / after-image*, *CLR* (Compensation Log Record,
  ARIES completo — nombrado y cortado), *checkpoint*, *truncado
  seguro*, *rotación por tamaño*, *idempotencia de recuperación*,
  *parada limpia del log*, *fuzzy checkpoint* (nombrado y cortado),
  *group commit* (nombrado y cortado), *recuperación distribuida*
  (nombrado y cortada).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio experto reconstruye desde la
  memoria el flujo abrir-reabrir cerrando contra el cap. 28 — citar
  en orden las llamadas y de dónde viene cada material NO está
  revelado en el enunciado (la pregunta exige traer el esquema a la
  memoria). El esencial predice el contenido de las TRES tablas del
  análisis A MANO.
- **Spacing**: cap. 28 (WAL — fuente de las tres piezas que el
  cap. 29 reutiliza: `aplicar_para_redo` idempotente, `Wal::iter`
  con parada limpia, `truncar_hasta_lsn` con contrato); cap. 27
  (Operacion — fuente de `PutNode`/`PutEdge`/`DeleteNode`/
  `DeleteEdge`; el `staging` del cap. 27 es por qué el cap. 28 pudo
  no tener UNDO); cap. 12 (`FilePager::sync` — el fsync riguroso
  del que aquí se documenta el patrón); cap. 13 (BufferPool — la
  razón histórica por la que existe steal, y por la que ARIES es
  general); cap. 8 (`GraphStore` — el puerto que el recovery pide en
  préstamo exclusivo).
- **Interleaving**: el intermedio mezcla el replay del cap. 28 con
  la operación confirmada / no confirmada + el steal del cap. 13;
  el experto mezcla WAL (cap. 28), Operacion (27) y la re-valoración
  ACID (caps. 27-28-29).
- **Regla de dificultad asimétrica**: la explicación introduce UNA
  idea nueva por sección (analysis → redo → undo → before-image →
  checkpoint → fichero → rotación → ACID post — ocho secciones, ocho
  ideas, dentro de la memoria de trabajo); los ejercicios exigen
  predicción, análisis manual e implementación.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap29`
  ejecuta los 20 tests en `tests_recuperacion` y los 2 doctests; el
  lector puede verificar cada ejercicio al instante.
- **Citas**: C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh, P.
  Schwarz, «ARIES: A Transaction Recovery Method Supporting
  Fine-Granularity Locking and Partial Rollback Using Write-Ahead
  Logging», ACM TODS 17(1), marzo 1992, pp. 94-162 (DOI
  10.1145/128765.128770) — fuente de las tres fases y la dirty page
  table; C. Mohan, «Repeating History Beyond ARIES» (ICDE 1999) —
  la evolución (LRU-K, fuzzy checkpoint, CLR); R. Ramakrishnan, J.
  Gehrke, «Database Management Systems» (3.ª ed., McGraw-Hill 2003),
  capítulo 18 — la presentación académica estándar de recovery;
  T. Haerder, A. Reuter, «Principles of Transaction-Oriented Database
  Recovery», ACM Computing Surveys 15(4), 1983, pp. 287-317 (DOI
  10.1145/322290.322291) — el análisis de las políticas steal/
  no-steal y force/no-force que el capítulo recoge indirectamente.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (13 en la tabla §5; todas remiten a líneas del módulo o a MIGRATION-PATTERN §34).
- [x] Escenario de fallo visible: steal activo + perdedora que escribió + recuperación; truncado rompiendo contrato (`redo_falla_ruidosamente_si_el_truncado_rompio_dependencias` 1231-1266).
- [x] Código ejecutable en workspace (20 tests `tests_recuperacion` + 2 doctests, ALL_GREEN) citado por nombre y línea, no duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: seis — replay ≠ recovery, redo ≠ undo, checkpoint ≠ foto del store, before-image ≠ simetría, `operaciones_sin_before_image` ≠ error, no-steal elude el undo POR DISEÑO).
- [x] Ejercicios con solución verificable (los 20 tests del módulo + los 2 doctests).
- [x] ≥1 ejercicio de retrieval (experto reconstruye el flujo desde memoria; esencial predice A MANO las tres tablas del análisis) y ≥1 de spacing (intermedio mezcla caps. 27 + 28 + 13; experto usa caps. 8 + 12 + 27 + 28).
- [x] Responde la pregunta crítica del CORPUS («ARIES (Analysis-Redo-Undo) simplificado»): las tres fases tienen sección propia y la analogía del steal motiva cada decisión.
- [x] Repaso-árc de la Parte VI (27→28→29) con diagrama explícito al estilo del cap. 26 con la Parte V.
- [x] El paper ARIES (Mohan et al. 1992, TODS 17(1)) citado en §11 con paper y DOI; las alternativas históricas (Haerder & Reuter 1983) y la evolución posterior (Mohan 1999) también citadas.
- [x] Ganchos al cap. 30 (MVCC, concurrencia, group commit), cap. 36/37 (persistencia end-to-end, checkpoint del store) y al Vol.III.
- [x] Las cifras usadas en capítulo y ejercicios son las de los tests REALES ejecutados (20 passed + 2 doctests).
