# CONTRATO DE CAPÍTULO — Vol.II Cap. 28: Write-Ahead Log

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap28_wal.rs` (2.229 líneas,
> 26 tests en `tests_wal` + 1 doctest, verificados ALL_GREEN
> `cargo test -p vol2-liradb --lib cap28` → 26 passed). Decisiones reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §33 (incluye la
> historia de la migración: módulo único, un único toque quirúrgico al
> `cap27_transacciones.rs` — `validar_buffer` pasa a `pub(crate)` para
> reutilización sin duplicación, `Wal::Default` manual para fijar la
> política `CadaEscritura` como contenido y no azar). El capítulo vive
> en la **Parte VI** y abre la línea 351 del `CORPUS.yml`:
> *"Write-Ahead Log"* — preguntas críticas
> *"Log record formats; LSN."* Continuación directa del cap. 27
> (transacciones ACID); cubre la primera mitad del protocolo de
> durabilidad de la Parte VI y deja la mitad complementaria (recuperación
> con ARIES) al cap. 29, el group commit concurrente al cap. 30.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la **transacción con staging** del cap. 27
  (`Transaccion::commit` re-valida el buffer y luego aplica; rollback
  descarta; `Drop` = rollback implícito por el borrow checker); las
  cuatro letras ACID tipadas (`GarantiaAcid`, `NivelGarantia`,
  `informe_acid()`) y CÓMO el cap. 27 las dejó — **A** parcial, **C**
  parcial trivial, **I** parcial por el préstamo `&mut`, **D** *Ninguna*
  porque el commit vivía en RAM; la **Operacion como dato** del cap. 27
  (`PutNode` / `PutEdge` / `DeleteNode` / `DeleteEdge`) y su papel
  exactamente como el `RecordKind` del cap. 10 — semilla deliberada; el
  **encoding del cap. 9** (`encode_string`, `encode_value`, `decode_*`,
  little-endian explícito) y el **framing del cap. 10** (length-prefix
  u32 + CRC32 con `crc32_simple`); la **slotted page** del cap. 11 y
  por qué el `FilePager::sync` del cap. 12 ya hace la `fsync` real
  cuando la conectemos; el **append-only log** del cap. 10 y la
  herramienta de truncado `truncate_to`; el `StoreQueFalla` del cap. 27
  (test que falla en la N-ésima escritura y demuestra el apply a
  medias); la decisión de diseño «antes que la corrijas» del cap. 16
  sobre compactación y huecos.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1)
  «Hacer commit es persistir» — no: el commit del cap. 27 era en RAM, y
  los datos se perdían al cerrar el proceso; la durabilidad no es del
  commit, es del **registro Commit en disco + sync**; (2) «El log se
  escribe PARA DESPUÉS deshacer» — no en nuestro motor: la decisión
  del capítulo es **commit-marker-ANTES-del-apply** (roll-forward /
  redo-only), por lo que el log se escribe PARA RE-APLICAR, no para
  deshacer; (3) «Sync tras cada log_write es siempre lo correcto» —
  no: es la **regla de oro** (correcta y simple), pero `SoloCommit`
  basta porque las **páginas de datos no se llevan a disco antes del
  commit** (lo que prohíbe el write-ahead); un `fsync` por transacción
  es legítimo y es la semilla del group commit; (4) «El WAL es una
  pila cualquiera de bytes» — no: cada registro lleva LSN (monótono,
  consecutivo, **nunca reutilizado** ni tras truncar) y CRC32; un
  salto en la numeración o un CRC roto NO bytes aleatorios — es un
  error tipado (`LsnInvalido`, `CrcInvalido`); (5) «Si trunco el log,
  ya está» — no: `truncar_hasta_lsn(lsn)` lo firma el llamador; truncar
  lo no-durable PIERDE datos (testado como `RedoFallido` con arista
  huérfana); (6) «El replay re-aplica sólo lo que faltaba» — no: el
  redo es **idempotente** (put idéntico = no-op, put divergente =
  overwrite, delete de lo ausente = no-op), por eso re-replay no
  duplica y por eso la `pasada 1` de `replay_wal` cobra sentido.
- **NO debe saber todavía**: ARIES con su fase Analysis (dirty page
  table, CLR) y UNDO parcial (cap. 29), el fichero real del WAL sobre
  `File` con `O_APPEND` + `sync_all` (cap. 29 también), el checkpoint
  que decide «hasta dónde es seguro truncar» automáticamente (cap. 29),
  la rotación por tamaño del fichero (cap. 29), el group commit REAL
  con varias transacciones concurrentes compartiendo un fsync (cap. 30),
  MVCC y aislamiento transaccional de snapshots (cap. 30), `&dyn
  GraphStore` como `Sync` (cap. 30). Se nombran como «luego lo verás» y
  se corta.

## 2. Conceptos (del grafo curricular)

- `present`: **WAL** como *«disco»* del log (en RAM, contrato de
  framing del cap. 10); **WalRecord** = `(lsn, tx_id, CuerpoWal)` con
  `CuerpoWal = Begin | Operacion(op) | Commit | Rollback`; **LSN** (u64
  monótono, consecutivo, asignado por el `Wal`, nunca reutilizado —
  ni tras truncar) como la dirección física del log y la base del
  «¿hasta dónde he recuperado?» del cap. 29; **TxId** (u64, asignados
  por el `Wal`, intercalación permitida si hay concurrencia futura);
  **`PoliticaFlush`** (`CadaEscritura` por defecto: la regla de oro
  literal — sync tras cada log_write — vs `SoloCommit`: un sync por
  transacción, semilla del group commit); **commit en dos fases**
  (`WalTransaccion::commit` = re-validar → log_write de cada op (sync
  según política) → Commit + sync → apply); **REDO idempotente**
  (`aplicar_para_redo`: put idéntico no-op, put divergente overwrite,
  delete de lo ausente no-op; la tolerancia sin la cual no se puede
  re-aplicar «sin saber qué sobrevivió»); **replay_wal** (dos pasadas:
  una para juntar las txs con Commit, otra para reaplicar sus ops en
  orden de LSN; `InformeReplay { confirmadas, descartadas,
  reaplicadas }`); **parada limpia** del iterador ante CRC roto /
  registro truncado / LSN no consecutivo (semántica de recuperación:
  se confía en el prefijo íntegro); **`decodificar_wal`** modo
  estricto (grita `WalError::CrcInvalido{lsn aparente}` /
  `RegistroTruncado` / `LsnInvalido`); **`truncar_hasta_lsn`**
  con contrato del llamador (los LSNs no se reinician); **`reconstruir`**
  del WAL a partir de bytes persistidos (escaneo al reabrir, semilla del
  cap. 29); **`ApplyFallido`** rescatable por `replay_wal` (la
  existencia del tipo de error cambia DE contenido: ya no es «a medias
  sin vuelta atrás», es «a medias CON log → roll-forward completa»);
  **`informe_acid_post_wal`** que re-valora con los MISMOS tipos del
  cap. 27 (D: Ninguna → Parcial; A sigue Parcial pero pasa a cerrarse
  en el 29).
- `practice`: `Operacion` del cap. 27 (la misma, tal cual); encoding
  cap. 9 (strings/values); framing cap. 10 (length-prefix + CRC32);
  `AcumulacionValidacion` del cap. 27 (validación eager en `stage`
  mediante `validar_buffer`); `ResumenCommit` y `ResumenRollback` del
  cap. 27 (`ResumenCommitWal` y `ResumenRollbackWal` los EXTENDEN con
  `lsn_commit` / `lsn_rollback`); `Drop` = rollback implícito por el
  borrow checker.
- `consolidate`: «derivar, no llevar en cabeza» (`free_space` del cap.
  11; aquí: `InformeReplay` deriva del recorrido, `syncs` se cuenta,
  `next_lsn` y `next_tx_id` se mantienen); «un único escritor por
  préstamo» (cap. 13 en `BufferPool`, cap. 27 en `Transaccion`); la
  política ruidosa de fallar antes que contestar «casi-bien» (caps. 17
  y 22); el store como puerto al que se le puede ENVOLVER un
  instrumento (cap. 26, `ContandoStore` — aquí `StoreQueFalla`).
- `out_of_scope` (solo nombrar): ARIES completo con Analysis / CLR
  (cap. 29), checkpoint automático y rotación por tamaño (cap. 29),
  group commit concurrente (cap. 30), UNDO parcial (cap. 29), MVCC
  (cap. 30), abrir el WAL desde fichero en `O_APPEND` (cap. 29).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) enuncia la REGLA del write-ahead («el cambio se
  escribe en el WAL **antes** que en la página de datos») y la
  RAMIFICACIÓN clave (commit-marker-ANTES-del-apply → roll-forward /
  redo-only; la alternativa marker-al-final exige UNDO = ARIES, cap.
  29); (2) describe el formato del `WalRecord` con el framing del cap.
  10 (`[record_len u32][lsn u64][tx_id u64][tag u8][payload][crc32]`)
  y dice por qué la `Operacion` es la MISMA del cap. 27 (la semilla
  del `RecordKind` del cap. 10 era deliberada); (3) explica la
  semántica del LSN (monótono, consecutivo, asignado por el `Wal`,
  nunca reutilizado) y la diferencia entre orden de bytes y orden
  lógico de operaciones; (4) demuestra la diferencia entre
  `CadaEscritura` (regla de oro, 4 syncs para 3 ops + commit) y
  `SoloCommit` (1 sync; correcto porque las páginas de datos NO se
  llevan a disco antes del commit) y por qué la segunda es **semilla**
  del group commit (cap. 30); (5) distingue la parada limpia del
  iterador (modo recuperación) del grito del modo estricto
  (`decodificar_wal`) y por qué los registros van con
  `length-prefix` + CRC32 + LSN consecutivo; (6) explica el contrato
  firmado por el llamador en `truncar_hasta_lsn` y por qué los LSNs
  no se reinician (la identidad de un redo es su LSN, no su posición
  en bytes).
- **Skills**: (1) lee y reconstruye un log con `WalIterator` y
  `decodificar_wal` distinguiendo los tres errores tipados; (2)
  ejecuta una transacción con `WalTransaccion` y predice el `lsn_commit`
  y la cuenta de `syncs` ANTES de correr el test; (3) simula un
  «corte de luz» con `StoreQueFalla` y demuestra que `replay_wal`
  rescata la transacción; (4) trunca el log con `truncar_hasta_lsn`
  y verifica que un replay sobre store vacío pierde lo truncado
  (la otra cara del contrato).
- **Wisdom**: (1) decide entre `CadaEscritura` y `SoloCommit` leyendo
  el patrón de uso (qué transacciones se mezclan y cuán caro es un
  fsync en la máquina) y conoce la semilla del group commit; (2)
  decide cuándo un fallo del apply es recuperable vía replay (log
  contiene todo + Commit) y cuándo NO lo es (log truncado que rompe
  dependencias → `RedoFallido` ruidoso), y entiende por qué la regla
  es «el log manda» y por qué UNDO no entra en juego hoy.

## 4. Modelo mental

- **La notaría con libro de entrada.** Toda escritura del grafo que
  promete durabilidad PASA primero por la notaría: el notario abre un
  asiento nuevo, anota la operación y la «sella» con un número
  correlativo (LSN); tan sólo después de que la anotación está
  firmada y *sellada contra el libro* (sync) se le da al cliente la
  copia que modifica el fichero físico de la notaría (apply al
  store). Si en el momento de la firma el cliente sufre un infarto
  (el `StoreQueFalla` del cap. 27), la notación YA está en el libro;
  al día siguiente, nuevo notario, **relee el libro y completa la
  operación** (roll-forward). Si la firma no llegó (commit truncado),
  el asiento existe pero el cliente no tiene sello: la operación
  como si nunca hubiera ocurrido. Si alguien arrancó una página (un
  CRC roto) o arrancó un folio de en medio (hueco de LSN), el libro
  entero deja de ser confiable — la notaría no se inventa los huecos.
- **El LSN como sello notarial**: una vez asignado, no se
  reutiliza — ni aunque arranque una página nueva del libro. Por eso
  un sello de 2007 puede vivir en el libro aunque las primeras 100
  páginas ya se hayan borrado. La identidad de una anotación es su
  sello, no su posición en bytes.
- **Diagramas ASCII**: (a) el flujo del commit en dos fases
  (re-validar → log_write → Commit+sync → apply) lado a lado con la
  alternativa que el capítulo descarta (marker al final → UNDO); (b)
  el formato del `WalRecord` con el framing del cap. 10; (c) el
  escenario de la regresión inversa del cap. 27
  (`StoreQueFalla` + `replay_wal` → 4 nodos en vez de 2).
- **Momento ¡ajá!**: «"el log antes que el dato" no es una orden de
  paso: es una promesa sobre la dirección de la recuperación. Si el
  log se escribe ANTES, la mitad DESCONOCIDA del problema (qué
  sobrevivió al fallo) se resuelve con un re-leer; si se escribe
  DESPUÉS, la mitad desconocida exige deshacer — y deshacer necesita
  imágenes ANTES, lo que dobla el log. La pregunta "¿UNDO o no UNDO?"
  se contesta ANTES de escribir la primera línea de código».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap28_wal.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | `WalRecord = (lsn, tx_id, CuerpoWal)` reutilizando `Operacion` del cap. 27 y el framing del cap. 10 | Tres capítulos ya hicieron el trabajo: la `Operacion` del cap. 27 es la misma pieza que el `RecordKind` del cap. 10 anticipaba; el encoding del cap. 9 y el CRC32 del cap. 10 cierran el formato. Cero re-trabajo, cero duplicación | Nuevo tipo `WalOp` separado: dos paralelas que divergen (la del log con campos extra, la del buffer sin CRC) y dos encodings a mantener | El log y la transacción dejan de ser congruentes; las anomalías «en disco pero no en RAM» se vuelven indetectables | Doc de `WalRecord` (líns. 122-149); MIGRATION §33 decisión 1; `encode_operacion` (404-457) reusa el `Operacion` |
| 2 | Commit-marker-ANTES-del-apply (`WalTransaccion::commit` re-validar → log_write ops → Commit+sync → apply) | El «log antes que el dato» del brief: cuando el apply arranca, TODO el intento YA es durable. Un apply a medias (el `StoreQueFalla` del cap. 27) tiene salida: `replay_wal` lo completes. Mínimo equipamiento: un redo idempotente | Marker al final del apply: el apply a medias queda SIN commit y rescatarlo exige UNDO (before-images o CLR) — ARIES, cap. 29. UNDO exige guardar dos imágenes por cambio o re-aplicar desde un punto de no retorno | `ApplyFallido` = estado a medias sin salida; el contrato del cap. 27 («o todas o ninguna FRENTE a errores de validación, NO frente a fallos del apply real») sigue siendo una mentira | Doc de `WalTransaccion::commit` (1050-1093); `commit` (1177-1248); MIGRATION §33 decisión 2; Mohan et al. 1992 §ARIES (paper) |
| 3 | `Operacion` re-playada con `aplicar_para_redo` IDEMPOTENTE (put idéntico=no-op; put divergente=overwrite; delete de lo ausente=no-op) | El replay no sabe qué sobrevivió al fallo: si las dos escrituras del nodo ya están, la tercera no debe duplicar NADA. Sin idempotencia, el replay necesita un análisis del estado exacto al fallar (Analysis de ARIES) — eso es el cap. 29, más maquinaria | Re-aplicar a ciegas sin idempotencia: el segundo replay duplica el nodo si las dos primeras escrituras pasaron y la tercera no, o hace fallar una cascada por dependencia | Replay no reproducible (mismo log, distinto resultado); la semilla del ARIES completo no echa raíz | `aplicar_para_redo` (872-903); `replay_es_idempotente` (1599-1620); MIGRATION §33 lección 3 |
| 4 | LSN u64 monótono, consecutivo desde 1, asignado por el `Wal`, NUNCA reutilizado ni tras truncar | Es la dirección física del log y la identidad de un redo: si dos `WalRecord` tuvieran el mismo LSN, el replay no sabría distinguirlos. La monotonía sostiene la traza de progreso («¿hasta dónde he recuperado?» del cap. 29) y la propiedad de idempotencia del replay | LSN `= bytes.len()`: cambia con truncar, re-replay se confunde; LSN reciclable: dos redos idénticos posibles; LSN por llamador: el llamador se equivoca y rompe la monotonía | El «¿hasta dónde?» del cap. 29 no tiene respuesta; el truncado esconde huecos que el modo estricto delata pero el iterador consiente | `append` (764-770); `truncar_hasta_lsn` (803-826); `lsn_monotonos_asignados_por_el_wal` (1429-1446); `Lsn` doc (77-83) |
| 5 | Framing `[record_len u32][lsn u64][tx_id u64][tag u8][payload][crc32]` con `crc32_simple` del cap. 10 | Length-prefix para saber dónde termina el registro (lectura O(1) del siguiente); CRC32 para detectar bytes modificados sin re-parsear; el orden de comprobaciones es **length → CRC → parse** (un cuerpo corrupto nunca se parsea) | Tabla de offsets al final del log (alternativa del cap. 11): no ayuda con la integridad, sólo con la posición; sin CRC, un bit rotado pasa silencioso y aparece como un valor legal | Corrupción silenciosa: «el log se leyó completo», pero los datos parseados son basura con etiquetas válidas | `encode_wal_record` (461-484); `decode_wal_record` (491-548); cap. 10 framing; `crc_invalido_detectado` (1837-1869) |
| 6 | `PoliticaFlush::CadaEscritura` por defecto (regla de oro LITERAL) y `SoloCommit` como optimización consciente | Se enseña la regla antes que la optimización: el alumno que vea `SoloCommit` PRIMERO no entiende POR QUÉ se puede hacer. `SoloCommit` es correcto porque las páginas de datos no se llevan a disco antes del commit (lo que prohíbe el write-ahead); y es la semilla del group commit (cap. 30) | Política única (siempre `CadaEscritura`): rendimiento perdido; `SoloCommit` por defecto: el alumno aprende la optimización sin saber la regla | El alumno usa el fsync como sacramento y degrada su BD sin entender; o al revés, quita syncs y descubre la corrupción tras un corte de luz real | `PoliticaFlush` (579-605); `politica_por_defecto_y_syncs_por_escritura` (1466-1480); `solo_commit_dura_con_un_unico_sync_y_el_mismo_resultado` (1482-1512); MIGRATION §33 decisión 6 |
| 7 | Parada limpia del `WalIterator` ante corrupción / truncado / LSN no consecutivo + `decodificar_wal` para el modo estricto | El iterador es el modo recuperación: si el primer registro legible cumple todos los tests, se confía en él y en lo que sigue mientras mantenga la monotonía. Si la cola está rota, se PARA — no se inventa. El modo estricto es para los TESTS: gritar `CrcInvalido{lsn aparente}` / `LsnInvalido{hueco}` para que los tests afirme la detección | Iterador que devuelve `Err` por registro: lento y ruidoso para la recuperación; recuperación que ignora la cola y la lee igual: traga bytes podridos | Logs de producción que esconden corrupción: el siguiente test que crea un archivo con un byte roto aprende a leerlo como si fuera válido | `WalIterator::next` (835-859); `decodificar_wal` (555-574); `corrupcion_al_inicio_el_replay_para_en_el_prefijo_integro` (1916-1944) |
| 8 | `truncar_hasta_lsn(lsn)` con CONTRATO firmado por el llamador («sólo lo YA durable en el store»), sin reiniciar contadores | El log puede ser enorme; en algún momento hay que podarlo. La poda es SEGURA sólo si los registros podados ya son visibles en el store. Si se trunca más allá, los redos posteriores pueden quedar HUÉRFANOS (arista con nodo que no está). Los LSNs NO se reinician para que la identidad de un redo sea estable | Truncado por bytes: no respeta la frontera de registros; truncado sin contrato: `RedoFallido` silencioso; reinicio de LSN: replay duplica o pierde por la nueva numeración | Grafo renacido con aristas huérfanas o nodos fantasma; los ids de los redos colisionan con los nuevos | `truncar_hasta_lsn` (803-826); `truncar_hasta_lsn_deja_lo_posterior_y_no_reutiliza_lsns` (1975-2005); `truncado_la_deuda_documentada_replay_no_recupera_lo_truncado` (2007-2026); `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias` (2187-2227) |
| 9 | Replay en dos pasadas: (1) conjunto de `tx_id` con Commit; (2) redo de sus ops en orden de LSN, idempotente | Una sola pasada exige tres estados por registro (pendiente / en redo / confirmado) y aún así no resuelve intercalar Begin/Operación/Commit/Rollback de V transacciones en el log. Dos pasadas permiten que un Begin abortado y un Commit posterior de OTRA tx estén entrelazados: la pasada 1 decide la verdad, la pasada 2 la aplica | Una pasada con estado por registro: más maquinaria, mismo resultado; «redo siempre, undo si no llegó al commit»: ocupa el lugar de un redo idempotente con la mitad de la complejidad | Transacciones INTERCALADAS (preparadas para el group commit) hacen que el redo aplique operaciones de una tx abortada porque la pasada no sabe qué ocurrió al final | `transacciones_intercaladas_solo_la_confirmada_sobrevive` (1640-1672); `replay_wal` (948-984); MIGRATION §33 decisión 3 |
| 10 | `StoreQueFalla` reusado del cap. 27 como **TEST-TESIS** del capítulo (inversión de la regresión del cap. 27) | El cap. 27 dejó DOS tests que AFIRMABAN `node_count()==2` y «el store quedó a medias». El cap. 28 RECONSTRUYE el mismo escenario (mismo `StoreQueFalla`, mismas ops, mismo `fallar_en: 3`) y demuestra que el replay lleva al `node_count()==4`. La inversión de la regresión es la prueba de que el capítulo sirve | Tests nuevos con un store a medida: el paralelismo con el cap. 27 se pierde; el alumno no ve la CONTINUIDAD de la Parte VI | El capítulo se queda en «un test más que sigue el patrón» y nunca grita lo que cambia respecto a su predecesor | `apply_fallido_a_medias_rescatado_por_replay` (1758-1799); `corte_de_luz_a_mitad_de_apply_rescatado_por_replay` (1801-1833); MIGRATION §33 lección 1 |
| 11 | `informe_acid_post_wal()` reusa los MISMOS tipos del cap. 27 (`EntradaAcid`, `GarantiaAcid`, `NivelGarantia`) y cambia SOLO el contenido y el `capitulo_que_la_cierra` | La re-valoración honesta es la PRUEBA de que el capítulo sirve: D sube de *Ninguna* a *Parcial* (commit durable en el log), A pasa a cerrarse en el 29 (roll-forward funciona, falta el arranque automático). Test que verifica la TRANSICIÓN estructura por estructura | Informe enteramente nuevo: se pierde la comparativa con el cap. 27; el alumno no ve QUÉ cambió y por qué | Documentación sin auditoría: lo que el capítulo promete no puede ser verificable | `informe_acid_post_wal` (1286-1322); `informe_post_wal_actualiza_d_y_reasigna_caps` (2029-2063); cap. 27 `informe_acid` |
| 12 | `Wal::Default` MANUAL que fija `CadaEscritura` (no `Default` derivado); `PoliticaFlush` sin `Default` | La política por defecto es CONTENIDO del capítulo: la regla de oro es lo que se enseña y por defecto. `#[derive(Default)]` requeriría un `Policy::default()` que decide AZAR | `#[derive(Default)]` + `PoliticaFlush::default = SoloCommit`: sutilmente rápido y sutilmente incorrecto como entrada al capítulo; ningún `Default` en `Wal`: el alumno olvida la inicialización | El alumno elige otra política para «probar» y mide algo que no es la regla | `Wal::default` (635-647); `PoliticaFlush` (579-605); MIGRATION §33 bug fix del `Default` |
| 13 | `validar_buffer` del cap. 27 pasa a `pub(crate)` (toque quirúrgico, no duplicación) | El cap. 28 NECESITA la misma validación eager para que el commit no aplique operaciones inválidas (que ya pasaron `stage()` pero hay que re-validar al cruzar el «punto de no retorno»). Cero duplicación, misma semántica | Duplicar la validación en `WalTransaccion::commit`: dos copias que divergen en el primer refactor del cap. 27 | Uno de los dos `validate`s se queda atrás; la «validación al momento del commit» deja de equivaler a la «validación en stage» | `validar_buffer` en cap27 + `pub(crate)` ajustado; MIGRATION §33 nota sobre el único toque quirúrgico |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: la del cap. 27 — `commit` re-valida el buffer y aplica
  directamente. Si el apply falla a medias, error tipado (`ApplyFallido`)
  y fin. La transacción del cap. 27 era ACID en todo MENOS en D
  (durabilidad en RAM) y A frente al apply real (no frente a validación).
- **Qué la rompe**: el mismo `StoreQueFalla` que el cap. 27 introdujo
  para advertir del problema. Tres escrituras planas, la tercera
  falla: el store queda con `node_count()==2` y el display de
  `ApplyFallido` dice «operaciones ya estaban aplicadas y sin log no
  se pueden deshacer — el store quedó a medias (cap. 28: WAL)».
  Esa mención al cap. 28 no es retórica: es el gancho, y es el
  agujero que el capítulo viene a cerrar.
- **Evolución visible**: `WalTransaccion::commit` inserta una fase
  NUEVA entre la validación y el apply: el **write-ahead** (cada
  `Operacion` al log con sync según política, registro `Commit` +
  sync, el `lsn_commit` queda en `ResumenCommitWal`). El error
  `ApplyFallido{c}` del cap. 27 toma ahora la **misma FORMA** pero
  su **DISPLAY** cambia: «… pero el log YA contiene el commit:
  `replay_wal` COMPLETA la transacción (arranque automático: cap.
  29)». El test `apply_fallido_a_medias_rescatado_por_replay`
  reproduce el escenario del cap. 27 con las MISMAS 4 ops y el
  MISMO `fallar_en: 3` y termina con `node_count()==4` — la
  inversión de la regresión del cap. 27.

## 7. Prueba de fuego

- **TEST-TESIS** `apply_fallido_a_medias_rescatado_por_replay`:
  mismo `StoreQueFalla` y mismas 4 ops que el cap. 27 demostraba
  roto. Aquí el `commit` devuelve `ApplyFallido{indice: 2,
  aplicadas: 2}` (idéntica forma), el log YA contiene Begin + 4 ops
  + Commit + sync, y `replay_wal` sobre el mismo store a medias
  lleva al `node_count()==4`. **Inversión de la regresión del cap.
  27**: el escenario que aquél cerraba con «y nadie recuerda qué
  faltaba», éste lo abre con «el log SÍ recuerda» y lo cierra con
  `node_count()==4`.
- **TEST-TESIS-DOS** `corte_de_luz_a_mitad_de_apply_rescatado_por_replay`:
  el escenario del `panic!("corte de luz simulado...")` del cap.
  27. `catch_unwind` rescata el pánico, el store quedó con 1 nodo,
  el log contiene Begin + 2 ops + Commit + sync. `replay_wal`
  completa las 4 ops.
- **TEST-POLÍTICA** `solo_commit_dura_con_un_unico_sync_y_el_mismo_resultado`:
  misma transacción bajo `CadaEscritura` (4 syncs) y `SoloCommit` (1
  sync): los dos logs producen el MISMO resultado al hacer replay
  sobre stores vacíos. La optimización es transparente.
- **TEST-PROPAGACIÓN** `todos_los_value_sobreviven_wal_y_replay`:
  el `nodo_rico` (todos los tipos de `Value` del cap. 7: Int, Float,
  String, Bool, Null, Bytes) sobrevive WAL + replay — el encoding
  del cap. 9 y la operación «orden por clave» no pierden
  información.
- **Síntoma si el lector se salta el capítulo**: cualquier
  transacción confirmada es vulnerable a un cierre de proceso: las
  páginas de datos y el commit mismo son RAM. El `ApplyFallido` del
  cap. 27 sigue documentando la pesadilla: store a medias sin
  vuelta atrás. Y el cap. 29 (ARIES) no tiene nada que cerrar.

## 8. Trampas y errores comunes

1. **Confundir commit con persistencia**: «commit == durable» es la
   mentira del cap. 27; la durabilidad es del **registro Commit en
   el log + sync**, y por eso `ResumenCommitWal` lleva `lsn_commit`.
   Síntoma: el alumno lee `ResumenCommit` del cap. 27 y asume que
   `commit` es la durabilidad — sin mirar la entrada D de
   `informe_acid()`.
2. **Confundir `CadaEscritura` y `SoloCommit` con fsync de verdad**:
   `Wal::sync` es un CONTADOR en RAM; la `sync_all` REAL es la de
   `FilePager::sync` del cap. 12. El alumno que pone `SoloCommit`
   en un test y otro `CadaEscritura` en otro mide syncs distintos
   sobre la misma transacción y se pregunta cuál está mintiendo.
   Respuesta: ninguno, miden la POLÍTICA, no el disco.
3. **Truncar lo no-durable, en serio**: `truncar_hasta_lsn(lsn)`
   es un método PODEROSO. Si el alumno pasa el LSN de una tx
   intermedia (cuyo apply se completó a medias) y luego llama a
   `replay_wal` sobre un store VACÍO, los nodos se pierden y las
   aristas quedan huérfanas (`RedoFallido { lsn: 6, ... }`). El
   test `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias`
   es la DEMOSTRACIÓN; el contrato del llamador en el doc es la
   PREVENCIÓN.
- **Precisión de lenguaje (glosario)**: *WAL* (el log) vs *registro*
  (cada `WalRecord`) vs *transacción* (la unidad con Begin/Commit);
  *LSN* (dirección física del log, monotónica, nunca reutilizada) vs
  *posición en bytes* (lo que se mueve con el truncado); *flush*
  (llevar a almacenamiento estable, fsync) vs *sync* (en este WAL, el
  contador); *redo* (re-aplicar, idempotente) vs *undo* (deshacer, no
  existe aún); *commit-marker* (el registro Commit) vs *commit* (la
  operación que lo deja en el log); *durable en el log* (LSN con
  Commit + sync) vs *durable en el store* (aplicado y reflejado en
  las páginas de datos que el cap. 29 persistirá); *parada limpia*
  (el iterador termina en la cola rota) vs *modo estricto*
  (`decodificar_wal` grita); *truncado* (descarte del log) vs
  *rollback* (descarte del buffer de la tx); *group commit* (varias
  tx compartiendo un fsync — semilla, cap. 30) vs *log físico sólo*
  (cada fsync sirve a UNA tx).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: predecir, ANTES de ejecutar, el
  contenido y los LSNs de un log de UNA transacción `WalTransaccion`
  que crea 3 nodos y 2 aristas y confirma, con `CadaEscritura`. ¿Cuántos
  registros tiene? ¿Cuál es `lsn_commit`? ¿Cuántos `syncs` se
  cuentan? ¿Qué tipo de cuerpo tiene cada uno? Pistas: (1) ¿qué
  escribe `begin_tx`?; (2) ¿el `Begin` se sincroniza?; (3) en
  `CadaEscritura`, ¿quién llama a `sync`?
  Verificación: `commit_aplica_todo_y_es_durable` y
  `politica_por_defecto_y_syncs_por_escritura`. Criterio: predicción
  exacta de número de registros + `lsn_commit` + número de syncs.
- **analizar (intermedio — spacing caps. 27+10)**: sobre un log con
  12 registros (3 Begin, 6 Operacion, 3 Commit), TODOS con su CRC y
  LSN válidos, pero donde la transacción 2 NO tiene Commit (la
  operación 5 de 6 fue su última), calcular a MANO el `InformeReplay`
  sobre un store vacío (confirmadas, descartadas, reaplicadas) y el
  `node_count`/`edge_count` del store renacido. Pistas: (1) ¿qué
  registra la pasada 1?; (2) ¿en qué orden procesa la pasada 2?;
  (3) ¿qué pasa con la tx sin Commit? Verificación:
  `transacciones_intercaladas_solo_la_confirmada_sobrevive`. Criterio:
  informe + cuentas exactas y respuesta a «¿se tocó el store con la
  abortada?».
- **crear (experto — concatenación WAL+CRC, retrieval puro)**: tomar
  un log del capítulo, corromper UN byte en la mitad del cuerpo de
  un registro y PREDECIR qué devuelve `decodificar_wal` (tipo de
  error, `lsn` reportado), qué hace `WalIterator`, y qué ve
  `replay_wal`. Luego comparar con
  `crc_invalido_detectado` y `corrupcion_al_inicio_el_replay_para_en_el_prefijo_integro`.
  Pistas: (1) ¿el CRC cubre el `body` o cubre `length+body`?; (2)
  el `lsn` `aparente` se calcula best-effort — ¿qué pasa si el
  cuerpo está tan truncado que ni siquiera tiene 8 bytes?; (3) el
  iterador de recuperación y el modo estricto comparten el mismo
  `decode_wal_record` — ¿en qué se diferencian?
  Criterio: tipo de error + `lsn` correcto + semántica
  recuperación/estricto + afirmación de que el replay sobre store
  pre-poblado no duplica.

## 10. Preguntas abiertas (gancho al cap. 29 — abre la
recuperación; y al cap. 30, group commit)

1. El log `Vec<u8>` en RAM se pierde al cerrar el proceso: ¿cuál es
   el fichero que lo persiste y cómo reabre automáticamente,
   ejecutando el `replay_wal`? (Nace el cap. 29: persistencia +
   ARIES simplificado.)
2. ¿Quién decide hasta dónde es seguro truncar el log
   automáticamente? (El checkpoint con su metadata,
   `Wal::truncar_hasta_lsn` ya es la pieza — falta el algoritmo.)
3. Sin concurrencia, una transacción por `sync`: el `SoloCommit`
   es una semilla. ¿Cómo se COMPARTEN varios `fsync` entre txs
   concurrentes? (Cap. 30: group commit real.)
4. Sin UNDO, una tx con apply a medias se rescata con replay. Pero
   si se deja APLICAR antes de confirmar (política *steal*), el
   apply a medias de una tx abortada exige UNDO — y la pregunta
   «¿UNDO o no?» se contesta en el diseño, no en la
   implementación. (Cap. 29: ARIES con Analysis-Redo-Undo.)
- **Términos nuevos de glosario**: WAL, LSN, registro (log record),
  cuerpo (Begin/Operacion/Commit/Rollback), commit-marker,
  write-ahead, roll-forward, redo, idempotencia, replay,
  fsync, flush, política de flush, group commit (semilla),
  truncate / truncado, frame (length-prefix + CRC), checkpoint
  (nombre, no implementación), ARIES (nombre, no
  implementación), undo (ausente hoy), state saving (partial /
  full), steal / no-steal.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto reconstruye DE MEMORIA el flujo
  del commit en dos fases y dice en qué orden se ejecutan los
  cinco pasos (re-validar → log_write ops → Commit + sync → apply);
  el esencial predice sin ejecutar los registros y syncs de una
  secuencia; el de corrupción predice el `WalError` antes de correr
  el test.
- **Spacing**: cap. 9 (encoding Little-Endian de strings/values y
  la regla de orden por clave de las props), cap. 10 (length-prefix
  + CRC32, `crc32_simple`, `truncate_to`), cap. 11 (la página como
  unidad y la «bitácora con prefijo» que el WAL hereda), cap. 12
  (`FilePager::sync` — la `fsync` real que el cap. 29 enchufa),
  cap. 13 (precio de la lectura que falla la caché — por qué
  `SoloCommit` es legítimo), cap. 17 (la política ruidosa de fallar
  antes que contestar casi-bien), cap. 22 (semántica estricta de
  los datos sucios — heredada en la validación eager del
  `stage()`), cap. 27 (`Operacion` como dato, `Drop` = rollback,
  `ApplyFallido` la forma que el cap. 28 modifica el contenido).
- **Interleaving**: el intermedio mezcla `Operacion` (cap. 27),
  `CRC32` (cap. 10) y la decisión de ARIES (mohan-citeado); el
  experto mezcla tipos de corrupción distintos (CRC, truncado,
  hueco de LSN) y predice el modo de fallo de cada uno.
- **Dificultad asimétrica**: una idea nueva por sección (LSN →
  framing → idempotencia → commit-marker → política → truncado →
  re-valoración ACID); los ejercicios exigen predicción y
  reconstrucción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap28`
  (26 tests citados por nombre; los del lector compilan contra el
  mismo módulo).
- **Citas**: Mohan, Haderle, Lindsay, Pirahesh y Schwarz,
  «ARIES: A Transaction Recovery Method Supporting
  Fine-Granularity Locking and Partial Rollbacks Using
  Write-Ahead Logging», ACM TODS 17(1), marzo 1992, pp. 94-162
  (DOI 10.1145/128765.128770 — la decisión
  commit-marker-ANTES-del-apply como roll-forward, ARIES completo
  con Analysis-Redo-Undo como siguiente paso); Gray, «Notes on
  Database Operating Systems», IBM Research Report RJ 2188, 1978
  (la huella de la bitácora como origen del WAL en System R);
  Bernstein, Hadzilacos y Goodman,
  *Concurrency Control and Recovery in Database Systems*,
  Cap. 11 («Logging and Recovery»), Addison-Wesley 1987
  (los algoritmos Recovery usando LSN y la tabla de páginas
  sucias); Lampson y Sturgis, «Crash Recovery in a Distributed
  Data Storage System», Xerox PARC 1976 (precursor, algoritmo
  cuidadoso).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa
  descartada y fuente (13 en la tabla §5).
- [x] Escenario de fallo visible: el `ApplyFallido` del cap. 27
  «store a medias y nadie recuerda qué faltaba» y la INVERSIÓN
  con `replay_wal` → `node_count()==4`; corte de luz con `panic!`
  → mismo rescate; corrupción al inicio (parada limpia) y
  truncado que rompe dependencias (grito ruidoso).
- [x] Código ejecutable en workspace (26 tests + 1 doctest
  ALL_GREEN, verificados) citado por nombre y línea, no
  duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: seis;
  «commit == persistir», «log PARA DESHACER», «CadaEscritura
  SIEMPRE», «log bytes aleatorios», «truncar ya está»,
  «replay sólo lo que faltaba»).
- [x] Ejercicios con solución verificable (tests del workspace +
  predicciones medibles con secuencia de LSN o de fsync).
- [x] ≥1 ejercicio de retrieval (experto reconstruye el flujo de
  commit y predice tipos de corrupción) y ≥1 de spacing (caps.
  10, 22, 27 tocados por el contrato; caps. 10, 27, 12-13 por
  el capítulo).
- [x] Responde la pregunta crítica del CORPUS («Log record
  formats; LSN.») y las once piezas del brief (write-ahead,
  LSN, begin/commit/rollback records, redo log, flush, group
  commit como semilla, checksums, log truncation con
  contrato, simulación de fallo, recuperación = replay
  manual, ARIES = cap. 29).
- [x] Gancho al cap. 29 (recuperación, ARIES, reopen, checkpoint,
  rotación) y al cap. 30 (group commit concurrente, MVCC).
- [x] Anécdota verificada con fuentes: Mohan et al. 1992 (DOI
  10.1145/128765.128770) para la decisión commit-marker y para
  ARIES; Gray 1978 (IBM RJ 2188) para la bitácora de System R;
  Bernstein et al. 1987 (§11) para los algoritmos de recovery
  con LSN; Lampson y Sturgis 1976 (precursor cuidadoso).
- [x] Las cifras usadas en capítulo y ejercicios son las de los
  tests REALES ejecutados (4 syncs = 3 ops + 1 commit; replay
  sobre store a medias = 4 ops re-aplicadas; 1 sync con
  `SoloCommit`; `lsn_commit` reportado en `ResumenCommitWal`).
