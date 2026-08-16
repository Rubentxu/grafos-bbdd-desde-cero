# CONTRATO DE CAPÍTULO — Vol.II Cap. 27: Qué significa una transacción (ACID)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap27_transacciones.rs` (~1.460
> líneas, 28 tests en `tests_transacciones` + 1 doctest, verificados
> ALL_GREEN en MIGRATION-PATTERN §32). Decisiones reales: ese mismo §32
> (incluye la migración: tests recalibrados al descubrir que `stage` valida
> EAGER y el commit no ve errores de validación, doctest que no compilaba
> por importar mal el trait, `source()` con `'static` y un `unused_mut`).
> Este capítulo ABRE la Parte VI (Fiabilidad), línea 44 de
> `manuscrito/vol2/tabla-de-contenidos.md`. Ganchos: cap. 28 (WAL),
> cap. 29 (recuperación) y cap. 30 (MVCC/2PL).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el trait `GraphStore` y el modelo del brief
  «múltiples lectores, un único escritor» (cap. 8); `&mut dyn GraphStore` y
  por qué dos escrituras simultáneas NO COMPILAN; el `append_only` log del
  cap. 10 (que el comentario del módulo llama «germen» del WAL del cap. 28);
  las invariantes estructurales del store (`StoreError`: DuplicateNode/Edge,
  UnknownNode/Edge, InvalidEdgeEndpoints); `Pager::sync` (cap. 12) y
  `BufferPool::flush` (cap. 13) como camino a disco YA EXISTENTE pero sin
  protocolo write-ahead; el motor Volcano y `Expand` (cap. 20); Dijkstra
  con pesos estrictos (cap. 22); `&mut dyn` como cerrojo Rust.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «cada `put_*` ya ES una transacción (autocommit)» — cierto pero parcial:
  la 5ª de 10 ops que falla deja las 4 anteriores aplicadas y el grafo a
  medias; (2) «ACID es una propiedad que una BD tiene o no tiene» — no:
  cada letra es INDEPENDIENTE y tiene nivel (Ninguna/Parcial/Completa);
  (3) «commit en RAM es durable» — un corte de luz borra lo confirmado, y
  la D exige disco (test `panic_a_mitad_de_apply_deja_el_store_a_medias`);
  (4) «aislamiento requiere un motor de locks sofisticado» — con un solo
  hilo, el `&mut` ES el cerrojo, gratis; (5) «rollback es deshacer» —
  ANTES de aplicar es gratis (borrar el buffer); DESPUÉS exigiría un log
  (cap. 28); (6) «atomicidad es un problema de concurrencia» — no: el
  problema aparece con UN solo escritor si el apply falla a mitad.
- **NO debe saber todavía**: WAL real con `lsn`, group commit, log
  truncation y ARIES (cap. 28); recuperación al arrancar (cap. 29); MVCC,
  2PL, niveles de aislamiento, OCC, grafos de espera y deadlocks (cap. 30);
  checkpoints; log sequence numbers como numeración global. Se nombran
  como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: el vocabulario ACID tipado (`GarantiaAcid`,
  `NivelGarantia`, `EntradaAcid`, `InformeAcid`, `informe_acid()`); las
  anomalías del aislamiento como vocabulario (`Anomalia::LecturaSucia`,
  `Anomalia::ActualizacionPerdida`, con `por_que_no_pasa_hoy()`); la
  transacción como objeto (`Transaccion::begin` → staging → commit/
  rollback); la operación como DATO (`Operacion` con cuatro variantes
  — el mismo shape que `RecordKind` del cap. 10); `TransaccionError`
  con dos variantes honestas (`OperacionInvalida` y `ApplyFallido`);
  `ResumenCommit` y `ResumenRollback`; `autocommit()` como función
  ejecutable; la simulación del replay (`Simulacion` y `validar_buffer`).
- `practice`: `&mut dyn GraphStore` y `StoreError` (cap. 8); el append-
  only log del cap. 10 (la `Operacion` del cap. 27 es su heredera
  directa); la cascada de `delete_node` que arrastra aristas (cap. 8);
  pager y buffer pool como camino a disco YA EXISTENTE (caps. 12-13);
  el borrow checker como motor de aislamiento (caps. anteriores);
  validación eager de pesos sucios del cap. 22 — análoga a la
  validación eager del buffer del cap. 27.
- `consolidate`: «derivar, no llevar en cabeza»; el store como puerto al
  que se le pueden envolver instrumentos (decorador — el `ContandoStore`
  del cap. 26 es primo del `StoreQueFalla` de este); el modelo «la
  documentación no puede prometer más de lo que el código cumple»
  (auditable por tests).
- `out_of_scope` (solo nombrar): WAL real, group commit, ARIES
  (cap. 28); undo/redo y recovery al arranque (cap. 29); MVCC,
  snapshots, 2PL, niveles de aislamiento, deadlocks (cap. 30);
  checkpoints; log sequence numbers.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) enuncia cada letra del acrónimo con la definición
  PARA LiraDB (no la de un manual genérico) y el nivel honesto
  (Parcial/Ninguna) que `informe_acid()` ejecuta y los tests verifican;
  (2) describe el ciclo de vida como objeto `begin → stage* → commit|
  rollback`, explicando por qué `commit` y `rollback` consumen `self` y
  por qué anidar dos tx sobre el mismo store NO COMPILA; (3) distingue
  rollback BARATO (descartar buffer, antes de aplicar) de rollback
  IMPOSIBLE (después de aplicar sin log — gancho al cap. 28); (4) nombra
  las dos anomalías clásicas (lectura sucia, lost update) y explica por
  qué ninguna puede ocurrir HOY con un solo hilo y `&mut` exclusivo;
  (5) explica el staging como replay sobre `Simulacion` y por qué el
  orden del buffer importa.
- **Skills**: (1) ejecutar `Transaccion::begin(&mut store)` con un lote
  de `put_node`/`put_edge`/`delete_*`, leer `ResumenCommit` y verificar
  el resultado en el store; (2) predecir el error exacto
  (`OperacionInvalida{indice, causa}`) ante un buffer inválido,
  incluidos los efectos de la cascada de `delete_node` y el orden
  relativo arista→nodos; (3) envolver cualquier `GraphStore` en un
  decorador de prueba (como `StoreQueFalla`) para demostrar un apply
  fallido y un pánico a mitad.
- **Wisdom**: (1) decide cuándo el staging basta (datos limpios, store
  benévolo) y cuándo NO (sistema con riesgo de crash — entonces WAL);
  (2) decide cuándo una promesa ACID se puede dar honesta y cuándo
  hay que degradarla con un `NivelGarantia::Parcial` — la honestidad
  es parte de la interfaz.

## 4. Modelo mental

- **El bloc de notas del contable con un bolígrafo que sólo escribe
  cuando firma al final**. Cada transacción es un bloc: las
  operaciones se anotan en el bloc (`stage`), NO en el libro mayor (el
  store). Cuando el contable firma al final (`commit`), pasa a limpio
  el bloc ENTERO — y si algo del bloc estaba mal (un asiento descuadrado,
  una fecha imposible), no firma y tira el bloc (`OperacionInvalida` —
  el bloc se queda con el prefijo válido). Mientras el bloc está abierto,
  NADIE puede tocar el libro mayor: la puerta del despacho tiene un
  único cerrojo (`&mut`). Si el contable muere con el bloc abierto, su
  caída ES el descarte — la transacción muere sin escribir nada. El
  problema aparece cuando ESTÁ pasando a limpio y se le cae el bolígrafo
  a mitad (`ApplyFallido`): los tres primeros asientos YA están en el
  libro y no hay forma de saber qué faltaba. Eso es lo que el log del
  cap. 28 resuelve — el bloc con copia de cada trazo, ANTES de pasar a
  limpio.
- **Diagramas ASCII**: (a) el ciclo de vida `begin → stage* → commit|
  rollback` con el `&mut` como barra crítica; (b) el `InformeAcid` de
  las cuatro letras en formato «A — nivel — estado — cap que la
  cierra»; (c) el replay del buffer sobre la `Simulacion`: cada
  operación contra (store real � ops anteriores); (d) los dos
  escenarios de fallo (apply con error a mitad, pánico entre
  escrituras) como sendos cortes de la línea de tiempo del apply.
- **Momento ¡ajá!**: «"commit" no es un botón, es una PROMESA — y
  mientras el código no pueda distinguir un commit confirmado de un
  apply cortado, la D no existe. Hasta entonces, la A es "o todas o
  ninguna frente a validación", no "o todas o ninguna frente al
  universo". El WAL del cap. 28 cierra la distancia entre la promesa
  y el universo».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap27_transacciones.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Vocabulario ACID como TIPO (`GarantiaAcid`, `NivelGarantia`, `InformeAcid`), no prosa | Una promesa de BD que no se ejecuta en tests es marketing. El informe ACID es un artefacto EJECUTABLE: tests como `informe_acid_es_honesto_sobre_el_estado_actual` e `informe_acid_tiene_las_cuatro_letras_en_orden` lo verifican; mentir rompe CI | Texto en el capítulo y un booleano `acid: bool`: la prosa puede prometer de más, el booleano oculta el nivel real (Parcial vs Completa) | Documentación que dice «tenemos ACID» y tests que delaten lo contrario — el peor modo de fallo de una BBDD | `GarantiaAcid` (líns. 56-66), `NivelGarantia` (122-141), `InformeAcid`/`informe_acid` (234-274); tests 851-887; MIGRATION §32 lección 1 |
| 2 | `NivelGarantia = Ninguna \| Parcial \| Completa` (tres niveles, no bool) | NINGUNA letra está completa en el cap. 27 (la tesis del capítulo). Un bool esconde «cuánto falta»; tres niveles permiten ser HONESTO sobre el progreso (D=Ninguna hoy, será Completa tras el cap. 28; C es Parcial «y trivial» porque no hay esquema) | `bool` acid: una sola letra, un solo valor; el lector cree que «ACID=true» significa «lo cumple todo» | Confianza del consumidor en una promesa genérica | `NivelGarantia` (122-130); tests 879-886 (ninguna letra Completa) |
| 3 | `Operacion` como dato (`PutNode/PutEdge/DeleteNode/DeleteEdge`) en un `Vec` privado | Las operaciones se acumulan como VALORES hasta el commit: la transacción puede validarlas TODAS juntas (replay sobre `Simulacion`) antes de tocar el store; rollback = descartar el `Vec`. La misma shape es la del `RecordKind` del cap. 10 y del `CuerpoWal::Operacion` del cap. 28 — el WAL serializa exactamente esto | Aplicar directamente cada `put_*` y llevar un log de operaciones para «deshacer»: dos representaciones del mismo dato, dos fuentes de verdad, dos sitios donde la cascada de `delete_node` puede divergir | Buffer que se valida poco a poco y store a medias tras un error tardío | `Operacion` (339-348); tests 1078-1107; MIGRATION §32 decisión 2 |
| 4 | `stage()` valida EAGER; `commit()` re-valida el buffer ENTERO | `stage` rechaza la op inválida al MOMENTO y la expulsa: la tx sigue viva con el prefijo válido. `commit` re-valida por inducción — es redundante con `stage` (nada externo puede mutar el store mientras lo tenemos prestado), pero es BARATA (O(n)) y robusta a refactors que rompan la inducción | Validar sólo en `commit`: el caller ve errores de operaciones lejanas mezclados con las válidas (UX mala) | Sólo validar en `stage`: un refactor que rompa la inducción (e.g. un nuevo `GraphStore` con un `&mut self` reentrante) pasa silencioso | `stage` (551-562); `commit` (608-667); tests 1037-1107; MIGRATION §32 bug 1 (dos tests que esperaban error en commit se reescribieron a stage-time) |
| 5 | Validación = REPLAY del buffer sobre una `Simulacion` (sets de nodos/aristas creados/borrados) que respeta el orden | Cada op del buffer se valida contra (store real ∪ ops anteriores): una arista cuyos extremos se crean en el MISMO buffer es válida SI los nodos van antes. `delete_node` arrastra aristas del store Y del buffer. Coste O(n·(n+E)) — naive y DOCUMENTADO en favor de la claridad | Validación incremental sin simulación: cada op contra el store real, sin ver las anteriores — rechaza una arista a un nodo del propio buffer (el caso del test `edge_a_nodo_creado_en_la_misma_tx_es_valido`) | Buffer que parece válido y al aplicarlo descubre que sus aristas referenciaban nodos que nunca existieron — exactamente lo que el staging debe evitar | `validar_buffer` (750-825); `Simulacion` (713-735); tests 1078-1107 y 1173-1188 |
| 6 | `commit` y `rollback` consumen `self` (ciclo de vida en los tipos) | Usar una transacción cerrada O anidar dos transacciones sobre el mismo store es ERROR DE COMPILACIÓN (no de runtime). El borrow checker verifica esto gratis, sin runtime, sin overhead | Devolver un `Option<Transaccion>` o un estado interno `cerrada: bool`: mismas semánticas, pero el error se descubre en runtime y el código defensivo prolifera | Estado «muerta» usado en runtime; transacción anidada aceptada y luego rota silenciosamente | `commit(self)` y `rollback(self)` (608-680); tests 972-984 (drop implícito = rollback seguro) |
| 7 | `Drop` de una tx activa = rollback implícito SEGURO (por construcción) | Como nada se aplica fuera de `commit`, abandonar la tx (salir del scope sin commit/rollback) ES descartar el buffer — el store no se tocó. No hace falta un `Drop` custom; `Vec::drop` basta | Forzar `commit` O `rollback` explícitos: ergonomía horrible para casos como `let _ = begin(&mut store); // algo que entra en pánico» | Tx que se olvida y deja estado inconsistente (en este capítulo no aplica, pero es la garantía que el lector exporta a su propio código) | Tests 972-984 (`drop_implicito_es_rollback_seguro`); comentario del módulo líneas 508-510 |
| 8 | Modelo «un único escritor» = préstamo exclusivo `&mut dyn GraphStore` (motor de aislamiento gratis) | El brief pide «múltiples lectores, un único escritor». Sin hilos, el `&mut` codifica ESO sin una línea de locking. MIENTRAS vive la tx, NI otro escritor NI ningún lector puede tocar el store: el compilador rechaza el anidamiento y rechaza leer durante la tx | Implementar un `Mutex<GraphStore>` y métodos `lock()`: correcto, pero introduce runtime overhead y el mismo `&mut` ya basta en single-thread; un día, con hilos, la decisión se REVISA (cap. 30) | Aislamiento inexistente (multi-lector sin writer lock) o aislamiento con overhead innecesario en single-thread | `begin(store: &'a mut dyn GraphStore)` (538-543); tests 1193-1215 (`transacciones_secuenciales_el_prestamo_se_libera`) |
| 9 | Anomalías como VOCABULARIO tipado (`Anomalia`), no como errores | Hoy NO pueden ocurrir (`por_que_no_pasa_hoy()` lo dice). Pero el cap. 30 las combatirá con MVCC/2PL — y para eso hay que nombrarlas ANTES, como vocabulario. Misma lógica que el «enemigo primero, defensa después» del cap. 22 con `NegativeCycle` | Borrar el enum hasta el cap. 30: el lector llega al MVCC sin saber qué está combatiendo | Capítulo de MVCC que enumera anomalías por primera vez — pedagógicamente más débil | `Anomalia` (282-289); `por_que_no_pasa_hoy` (315-321); test 916-932 |
| 10 | `ApplyFallido` (variante DISTINTA de `OperacionInvalida`) lleva la cuenta de operaciones aplicadas | El apply puede fallar a mitad por dos razones muy distintas: validación (que NO debería pasar si `stage` funcionó) o divergencia store/simulación (que NO se puede prevenir). La distinción permite al caller saber si el store quedó a medias y cuánto | Un único `Error` con un booleano `a_medias: bool`: la información «cuántas se aplicaron» se pierde | Caller que no sabe si tiene que investigar el store o no | `TransaccionError::ApplyFallido{indice, aplicadas, causa}` (384-391); tests 1332-1392 |
| 11 | `ApplyFallido` y el `StoreQueFalla` (decorador que falla en la N-ésima escritura) son TESTS, no promesas | Probar lo que NO funciona es contenido. Los dos tests `apply_fallido_deja_el_store_a_medias_gancho_al_cap_28` y `panic_a_mitad_de_apply_deja_el_store_a_medias` AFIRMAN el estado a medias con números (`node_count() == 2`, `node_count() == 1`); quedan como regresión inversa para el cap. 28 (cuando llegue el WAL, se invierten) | Un test «feliz» que verifica el commit normal — no enseña los límites; o un párrafo explicando el límite, sin test que lo DEMUESTRE | Capítulo «conceptual» que sólo tiene tests del happy path y límites en prosa | `StoreQueFalla` (1252-1330); tests 1332-1392; MIGRATION §32 lección 2 |
| 12 | `autocommit` como FUNCIÓN ejecutable (begin + stage + commit) | Hace visible que el modo por defecto de los caps. 7-26 (cada `put_*` su propia tx) es un caso particular del nuevo mecanismo. Test de equivalencia contra la operación directa | Dejar `autocommit` implícito y por convención: el lector tiene que CREER que es lo mismo | Asimetría silenciosa entre `put_*` y una transacción de una sola op — el lector no ve que son la misma cosa | `autocommit` (690-697); tests 1220-1245 |
| 13 | El rollback barato vs el rollback imposible (la frontera del capítulo) | Descartar el buffer (rollback antes de aplicar) es trivial. Deshacer DESPUÉS de aplicar exige un log — la línea exacta que el cap. 28 cruza. El comentario de `rollback` y el de `ApplyFallido` lo dicen: «cap. 28: WAL» | Hacer el rollback también deshaga escrituras: imposible sin log, sería un WAL parcial y deshonesto | Capítulo que promete rollback completo y no puede cumplirlo | Doc de `rollback` (669-680); doc de `ApplyFallido` (380-385); MIGRATION §32 decisión 6 |
| 14 | La D es `Ninguna` con honestidad (no se maquilla con el pager/buffer pool que YA EXISTEN) | `Pager::sync` (cap. 12) y `BufferPool::flush` (cap. 13) ya escriben a disco. Lo que falta es el PROTOCOLO write-ahead: forzar el orden «log antes que página» y saber QUÉ se escribió si el proceso muere. La D es la pieza más honesta del informe — y la que más cuesta de subir de nivel | Declarar D=Completa porque tenemos `flush`: confundir PIEZAS con PROTOCOLO. Una BD sin WAL no es durable por mucho `sync` que tenga | «Tenemos durabilidad» y un `kill -9` lo desmiente | `informe_acid()` entrada D (264-271); tests 876-887 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: lo que ya tenemos y funciona — cada `put_node`/
  `put_edge`/`delete_*` del `GraphStore` es su PROPIA transacción
  (autocommit). Test `autocommit_equivalente_a_la_operacion_directa`
  lo ejecuta: `store.put_node(n)` y `autocommit(store, PutNode(n))`
  dejan el MISMO grafo.
- **Qué la rompe**: (a) un lote de 10 operaciones en el que la 5ª falla
  deja las 4 anteriores aplicadas — el grafo a medias; (b) la C nunca
  puede ser «transacciones concurrentes no se ven a medias» porque no
  hay transacciones concurrentes, pero sí puede ser «el grafo cumple
  sus invariantes tras cada operación» (eso lo da el store hoy, no
  «la BD»); (c) la D no existe aunque `Pager::sync` exista (falta el
  protocolo write-ahead); (d) dos operaciones relacionadas (arista
  entre dos nodos NUEVOS) no se pueden agrupar sin que un fallo
  entremedias las deje huérfanas.
- **Evolución visible**: `Transaccion::begin(&mut store)` toma el
  préstamo exclusivo, el staging acumula `Operacion` en un `Vec`,
  cada `stage` valida EAGER y rechaza con `OperacionInvalida{indice,
  causa}` sin expulsar el prefijo válido, el `commit` re-valida el
  buffer ENTERO y aplica operación a operación con `ResumenCommit`. Si
  el store falla a mitad, `ApplyFallido{indice, aplicadas}` —
  honesto sobre lo aplicado. `autocommit` muestra que el modo previo
  es un caso particular.

## 7. Prueba de fuego

- **TEST-TESIS A** `informe_acid_es_honesto_sobre_el_estado_actual`:
  A=C=I=`Parcial`, D=`Ninguna`; ninguna `Completa`; las cadenas «APPLY»,
  «estructurales», «concurrencia», «RAM» aparecen — la prosa coincide con
  el código.
- **TEST-TESIS B** `error_en_la_operacion_3_de_5_no_aplica_nada`: buffer
  sembrado a mano con la op 3 inválida (edge 0→7 sin nodo 7); `commit()`
  devuelve `OperacionInvalida{indice:2, causa: InvalidEdgeEndpoints}` y
  el store QUEDA INTACTO. La atomicidad naive funciona.
- **TEST-TESIS C** `apply_fallido_deja_el_store_a_medias_gancho_al_cap_28`:
  `StoreQueFalla` que falla en la 3ª escritura; `commit()` devuelve
  `ApplyFallido{indice:2, aplicadas:2, causa: UnknownNode(usize::MAX)}` y
  el store tiene 2 nodos. La atomicidad naive NO cubre el fallo del apply.
- **TEST-TESIS D** `panic_a_mitad_de_apply_deja_el_store_a_medias`:
  `StoreQueFalla` con `con_panic=true`; `catch_unwind` atrapa el pánico;
  `node_count()==1`. La primera escritura llegó, las dos siguientes no.
  Sin WAL no hay forma de saber si era tx confirmada o cortada.
- **Otros tests citados**: `informe_acid_tiene_las_cuatro_letras_en_orden`,
  `garantia_acid_letra_nombre_definicion`, `informe_acid_display_muestra_niveles_y_caps`,
  `anomalias_de_aislamiento_definidas`, `commit_aplica_todo_el_buffer`,
  `rollback_no_aplica_nada`, `drop_implicito_es_rollback_seguro`,
  `commit_vacio_es_noop_valido`, `stage_rechaza_duplicado_dentro_del_buffer_y_la_tx_sigue_viva`,
  `stage_rechaza_edge_a_nodo_inexistente`, `edge_a_nodo_creado_en_la_misma_tx_es_valido`,
  `el_orden_importa_edge_antes_de_sus_nodos_es_invalido`,
  `delete_de_nodo_creado_en_la_misma_tx`, `delete_node_inexistente_rechazado`,
  `delete_edge_tras_cascada_de_delete_node_rechazado`,
  `recrear_nodo_tras_borrarlo_en_la_misma_tx`,
  `edge_arrastrada_por_cascada_de_nodo_del_buffer`,
  `transacciones_secuenciales_el_prestamo_se_libera`,
  `autocommit_equivalente_a_la_operacion_directa`,
  `autocommit_operacion_invalida_no_toca_el_store`, `errores_display_y_std_error`,
  `resumenes_display`, `operacion_display`, `operaciones_vista_del_buffer`.
- **Síntoma si el lector se salta el capítulo**: sus lotes «de 10 nodos»
  quedan a medias si uno falla, no distingue apply-fallido de validación,
  cree que `commit` es durable cuando es RAM, y llega al cap. 28 sin
  vocabulario para pedirle al WAL lo que necesita.

## 8. Trampas y errores comunes

1. **«ACID es un todo»**: las cuatro letras son independientes y cada
   una tiene su nivel; el `InformeAcid` es la pieza honesta. Síntoma:
   «tenemos transacciones porque hicimos commit». Cura: el test del
   informe.
2. **Confundir validación con apply**: `OperacionInvalida` se rechaza
   al `stage` (la tx sigue viva con el prefijo válido); `ApplyFallido`
   se descubre en `commit` (el store quedó a medias). Son errores
   DISTINTOS y con consecuencias DISTINTAS. Síntoma: tratar todo como
   «la tx falló» sin investigar el store.
3. **Esperar rollback completo**: el rollback de este capítulo es
   BARATO (descartar buffer) por construcción — ANTES de aplicar. El
   rollback real (DESPUÉS de aplicar) exige un log; ése es el cap.
   28. Síntoma: diseñar un sistema que asume «rollback siempre
   funciona» sobre apply parcial.
- **Precisión de lenguaje (glosario)**: *transacción* (objeto con
  ciclo de vida, no «conjunto de operaciones») vs *operación*
  (`Operacion` como dato, unidad del buffer); *staging* (acumular en
  privado) vs *apply* (escribir al store); *commit* (firma al final
  del bloc) vs *rollback* (tirar el bloc); *buffer* (estado privado
  de la tx) vs *store* (libro mayor compartido); *validación* (lo que
  el staging hace) vs *apply* (lo que el commit hace); *autocommit*
  (tx de una sola op) vs *tx explícita* (una o más ops); *informe
  ACID* (el artefacto tipado) vs *ACID* (el acrónimo); *anomalía*
  (patrón de fallo) vs *garantía* (lo que la BD promete); *nivel*
  (Ninguna/Parcial/Completa) vs *letra* (A/C/I/D).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: dado el buffer
  `PutNode(0,"P"), PutEdge(0,0,1,"KNOWS"), PutNode(2,"P")` sobre un store
  vacío, predecir SIN ejecutar (a) si `commit` tiene éxito, (b) si no,
  qué variante de `TransaccionError` y con qué `indice` y `causa`, (c)
  el estado del store después. Pistas: (1) ¿existe el nodo 1 cuando se
  valida la arista?; (2) ¿el orden del buffer es libre?; (3) ¿la op 3ª
  se valida? Verificación: `el_orden_importa_edge_antes_de_sus_nodos_es_invalido`
  y `edge_a_nodo_creado_en_la_misma_tx_es_valido` (mover la arista al
  final). Criterio: predicción exacta + verificación corriendo los dos
  tests del workspace.
- **analizar (intermedio — spacing con caps. 8 y 10)**: (a) por qué el
  comentario del módulo llama a la `Operacion` del cap. 27 «la semilla
  del `RecordKind` del cap. 10» y cómo la misma shape permite que el
  cap. 28 serialice el buffer al WAL sin reinterpretar; (b) por qué
  `delete_node` arrastra aristas del store Y del buffer (test
  `edge_arrastrada_por_cascada_de_nodo_del_buffer`) y qué invariante del
  cap. 8 lo hace obligatorio; (c) por qué el rollback del cap. 27 es
  BARATO (descartar `Vec<Operacion>`) y el rollback REAL sería IMPOSIBLE
  sin un log — el `append_only::Log` del cap. 10 es la pieza que falta.
  Pistas: (1) ¿qué se almacena en `RecordKind` del cap. 10 con la misma
  forma que `Operacion`?; (2) ¿qué dice `StoreError` sobre la cascada?;
  (3) ¿qué hace `Log::append` y por qué es el «antes» del commit?
  Verificación: `delete_edge_tras_cascada_de_delete_node_rechazado` y
  la sección §10 de MIGRATION-PATTERN.md. Criterio: razonar la conexión
  entre tres capítulos sin mirar el código.
- **crear (experto — retrieval puro)**: parte 1, de memoria, el
  `InformeAcid` COMPLETO del cap. 27 (cuatro letras, sus niveles, sus
  justificaciones, los caps que cierran cada brecha); parte 2, escribir
  el test `nodo_recreado_despues_de_cascada_no_revive_sus_aristas` (grafo
  0→1 edge 0, 1→2 edge 1; tx: delete_node(1), put_node(1, "Renacido");
  commit verde; verificar `node_count()==2`, `edge_count()==0` y que las
  aristas muertas NO vuelven). Pistas: (1) ¿el `delete_node` arrastra
  las aristas adyacentes en la validación?; (2) ¿qué pasa con las
  aristas que tenían al nodo como extremo en la simulación?; (3) ¿qué
  ve la `Simulacion` cuando reaparece el id? Verificación:
  `recrear_nodo_tras_borrarlo_en_la_misma_tx` (base) + extensión propia.
  Criterio: informe exacto + test verde + razón de por qué NO revive las
  aristas.

## 10. Preguntas abiertas (gancho al cap. 28 — WAL)

1. Si el `commit` muta el store pero el proceso muere ANTES de
   `Pager::sync`, ¿cómo sabe el sistema al rearrancar qué operaciones
   estaban confirmadas? La pieza que falta es el log — un fichero al
   que se escribe ANTES de tocar el store y que sobrevive al crash
   porque `fsync` ya forma parte del protocolo. Eso es el WAL
   (cap. 28): `lsn`, `tx_id`, `CuerpoWal::Begin/Operacion(Operacion)/
   Commit/Rollback`, group commit, log truncation. La `Operacion` del
   cap. 27 es exactamente lo que se serializa.
2. Cuando el proceso muere a mitad del apply (el test
   `panic_a_mitad_de_apply_deja_el_store_a_medias`), ¿quién decide
   al rearrancar si los nodos escritos pertenecen a una tx confirmada
   (hay que CONSERVARLOS) o a una que se cortó (hay que DESHACERLOS)?
   Eso es recuperación (cap. 29): replay del WAL con undo y redo
   estilo ARIES.
3. Con un solo hilo, el borrow checker es el cerrojo. ¿Qué cambia
   cuando llegue la concurrencia (cap. 30)? ¿Qué anomalías —
   `LecturaSucia`, `ActualizacionPerdida` y otras que aún no hemos
   nombrado: lectura no repetible, fantasma, escritura sesgada —
   aparecerán, y qué las combatirá: MVCC, 2PL, niveles de
   aislamiento, OCC?
- **Términos nuevos de glosario**: transacción, autocommit, begin,
  staging, buffer, commit, rollback, apply, validación eager,
  atomicidad naive, operación (PutNode/PutEdge/DeleteNode/DeleteEdge),
  anomalía (lectura sucia, lost update), nivel de garantía (Ninguna/
  Parcial/Completa), informe ACID, préstamo exclusivo como cerrojo,
  rollback implícito, store a medias, decorator sobre `&dyn GraphStore`
  (StoreQueFalla como primo del ContandoStore del cap. 26).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto reconstruye el `InformeAcid` COMPLETO
  (cuatro letras, niveles, justificaciones, caps que las cierran) sin pistas
  en el enunciado; el esencial predice el resultado de un buffer inválido SIN
  ejecutar (variante del error + índice + causa) y lo verifica con los tests.
- **Spacing**: cap. 8 (`StoreError`/`delete_node` con cascada; `&mut dyn`
  como cerrojo), cap. 10 (la `Operacion` del 27 hereda del `RecordKind`;
  `Log::append` es el «antes» que el WAL del 28 codificará), caps. 12-13
  (`Pager::sync`/`BufferPool::flush` como camino a disco ya existente — la
  D sólo necesita el protocolo), cap. 20 (`Expand` y el motor que asumía
  autocommit), cap. 22 (validación eager: misma política ruidosa que el
  cap. 22 con pesos sucios), cap. 26 (el `ContandoStore` del 26 es primo
  del `StoreQueFalla` del 27 — decorador sobre `&dyn GraphStore`,
  instrumento de medida o de fallo).
- **Interleaving**: el intermedio mezcla el `Operacion` del 27 con el
  `RecordKind` del 10 y el `Pager::sync` del 12 («qué pieza hace falta para
  cerrar la D?»); el experto mezcla el `InformeAcid` del 27 con la
  validación eager del 22 y la cascada del 8.
- **Dificultad asimétrica**: una idea nueva por sección (vocabulario ACID →
  ciclo de vida de la tx → staging/replay → apply → `ApplyFallido` honesto
  → `InformeAcid` ejecutable → gancho al WAL); los ejercicios exigen
  predicción y construcción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb --lib cap27`
  (28 tests + 1 doctest citados por nombre; los del lector compilan contra
  el mismo módulo).
- **Citas**: Gray & Reuter, «Transaction Processing: Concepts and Techniques»
  (Morgan Kaufmann, 1993) — referencia canónica ACID/recovery/aislamiento;
  Haerder & Reuter, «Principles of Transaction-Oriented Database Recovery»,
  ACM Computing Surveys 15(4), 1983, pp. 287-317 (DOI 10.1145/289.291) —
  formalización de ARIES que el cap. 28 hereda; Mohan et al., «ARIES»,
  ACM TODS 17(1), 1992, pp. 94-162 (DOI 10.1145/128765); Berenson et al.,
  «A Critique of ANSI SQL Isolation Levels», SIGMOD Record 24(2), 1995,
  pp. 1-10 (DOI 10.1145/223784.223785); SQLite docs «Atomic Commit In
  SQLite»; PostgreSQL docs cap. 13 «Concurrency Control»; los comentarios
  del módulo como prosa verificable.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada
  y fuente (14 en la tabla §5).
- [x] Escenario de fallo visible: dos tests DEMUESTRAN el store a medias
  (`apply_fallido_deja_el_store_a_medias`, `panic_a_mitad_de_apply_deja_el_store_a_medias`).
- [x] Código ejecutable en workspace (28 tests + 1 doctest, ALL_GREEN en
  MIGRATION §32) citado por nombre y línea, no duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: seis; ACID-no-es-un-todo,
  autocommit-parcial, apply-vs-validación, RAM-no-es-durable, rollback-barato-
  vs-imposible, aislamiento-gratis).
- [x] Ejercicios con solución verificable (tests del workspace + un test de
  extensión propio del lector).
- [x] ≥1 ejercicio de retrieval (InformeAcid desde memoria, buffer con arista
  antes de nodos) y ≥1 de spacing (caps. 8/10/12-13/20/22/26 tocados).
- [x] Responde la pregunta crítica del CORPUS («ACID: trade-offs») y abre
  la Parte VI con vocabulario tipado y primera maquinaria.
- [x] Anécdota verificada con fuente (Gray & Reuter 1993, Haerder & Reuter
  1983, Mohan 1992, Berenson 1995).
- [x] Gancho explícito al cap. 28 (WAL): `Operacion` como shape del
  `CuerpoWal::Operacion`, `Pager::sync` como pieza existente, `ApplyFallido`
  como motivación.
- [x] Borrow checker del cap. 8 re-ejercitado como motor de aislamiento
  (`transacciones_secuenciales_el_prestamo_se_libera`).
