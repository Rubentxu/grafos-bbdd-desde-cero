# Capítulo 27 — Qué significa una transacción (ACID)

> *«"Commit" no es un botón: es una promesa. Y mientras el código no pueda distinguir un commit confirmado de un apply cortado, la durabilidad es un título que adorna un artefacto que todavía no la ha ganado.»*

## 27.0 La anécdota de la esquina

En junio de 1981, Jim Gray — por entonces en Tandem Computers, en Cupertino, California — presentó un paper invitado en la conferencia VLDB titulado «The Transaction Concept: Virtues and Limitations». En las primeras líneas resumía en una frase lo que llevaba años cocinándose en IBM Research: *una transacción es una transformación de estado que tiene tres propiedades: atomicidad (o todo o nada), durabilidad (los efectos sobreviven a los fallos) y consistencia (una transformación correcta)*. Y para ilustrar por qué el concepto importaba, no citaba ningún teorema: citaba **vuelos de avión**, transferencias electrónicas de fondos y alquileres de coches — esos sistemas donde una reserva que se queda «a medias» es un desastre.

La génesis era más antigua. En System R, el prototipo relacional de IBM de mediados de los setenta, ya corría un Recovery Manager — documentado en 1981 en «The Recovery Manager of the System R Database Manager» (Gray y coautores) — y los *predicate locks* de Eswaran, Gray, Lorie y Traiger ya hablaban de consistencia e aislamiento en 1976. Pero faltaba la palabra. El acrónimo **ACID** no lo acuñó Gray: fue Theo Härder y Andreas Reuter quienes lo consolidaron en 1983, en «Principles of Transaction-Oriented Database Recovery» (ACM Computing Surveys 15(4), pp. 287-317), donde lo llaman el *«ACID principle»*.

Detente en el giro que encierra esta historia: la atomicidad, la consistencia y la durabilidad existían como *problema* antes que como nombre. El nombre llegó después, como pegamento, para que cuatro términos hablasen de un único paradigma. Lo que haremos aquí es lo contrario: **despegar el acrónimo** y preguntar, para LiraDB, qué significa *de verdad* cada letra.

## 27.1 Objetivo

Al terminar este capítulo sabrás **qué es realmente una transacción**, y habrás construido en Rust la primera maquinaria que la representa. Cuatro piezas en `cap27_transacciones.rs`:

1. **`GarantiaAcid` + `NivelGarantia` + `InformeAcid`** — el vocabulario ACID como *tipo* y como *informe ejecutable* que los tests verifican. No es prosa: es un artefacto auditable que dice, de cada letra, hasta dónde llega hoy y qué capítulo la cerrará.
2. **`Transaccion` — el ciclo de vida `begin → stage* → commit|rollback`** — la transacción como *objeto* que acumula operaciones en un buffer y solo las aplica al commit, validándolas todas de golpe.
3. **`Operacion`** — la operación de escritura como *dato* (`PutNode/PutEdge/DeleteNode/DeleteEdge`), la pieza que hace posible el staging.
4. **`autocommit` e `informe_acid`** — que hacen visibles, una como función y otra como reporte, las dos caras de la moneda: el modo por defecto de los capítulos 7-26 era autocommit, y ninguna letra del ACID está completa todavía.

Y la lección que lo vertebra: **la honestidad**. `informe_acid()` te dirá que la A es parcial, que la C es parcial y trivial, que la I es parcial — *por diseño* — y que la D es ninguna. Aprender a decir eso, y a ejecutarlo en tests, es más importante que fingir ACID en un banner.

## 27.2 Problema

LiraDB lleva veintiséis capítulos escribiendo grafos. Mira el modo en que ha escrito desde el capítulo 7: cada `put_node`/`put_edge`/`delete_*` del `GraphStore` (cap. 8) te devuelve su `StoreError` si algo falla... y sigues. Pero fíjate en lo que tienes sembrado sin darte cuenta. Piensa en una arista entre **dos nodos nuevos**, ambos creados en la misma ráfaga:

```rust
store.put_node(Node::new(0, "A"))?;   // ok
store.put_node(Node::new(1, "B"))?;   // ok
store.put_edge(Edge::new(0, 0, 1, "KNOWS"))?;  // ok… ¿siempre?
```

Hoy, si la primera falla, las demás ni se intentan — pero nada te *agrupó* esas tres como una unidad. Cada `put_*` es su propia transacción: lo que una base de datos real llama **autocommit**. Y el autocommit tiene una debilidad concreta: un *lote* de diez operaciones en el que la quinta falla deja las cuatro anteriores aplicadas. Tu grafo queda **a medias**.

Tres síntomas aparecen:

1. **Un lote de 10 nodos que falla en el 5 deja 4 nodos en el store.** Nadie te pidió eso. Nadie sabe que eran «un lote».
2. **Dos operaciones relacionadas (arista + sus nodos) no se pueden agrupar** sin que un fallo entremedias las deje huérfanas.
3. **No sabes ni siquiera qué *es* «lo que pediste».** No tienes vocabulario para decir «esto son cinco operaciones que deben aplicarse juntas, o ninguna».

El problema de fondo: **el store aplica lo que le dices, cuando se lo dices**. No hay una unidad intermedia que te permita decir «reúne esto primero; cuando te diga, aplícalo todo de una vez». Y sin esa unidad, no hay transacción — solo una serie de comandos sueltos.

## 27.3 Modelo mental

Piensa en el **bloc de notas del contable** que solo escribe en el libro mayor cuando firma al final.

El contable tiene dos objetos físicos. Un **libro mayor** — el único con valor legal, donde se pasan a limpio los asientos. Y un **bloc de notas** — desechable, donde *anota* los asientos mientras los revisa. Las reglas son:

- Cada asiento se **anota en el bloc** primero (`stage`). El libro mayor no se toca todavía.
- Cuando el contable **firma** (`commit`), pasa a limpio el bloc **entero**, en un solo trazo.
- Si algo del bloc estaba mal — un asiento descuadrado, una fecha imposible — **no firma** y tira el bloc (`OperacionInvalida`). La transacción muere sin escribir nada.

Y hay dos detalles que se parecen sospechosamente a lo que vamos a programar:

- **La puerta del despacho tiene un único cerrojo** (`&mut`). Mientras el bloc está abierto, *nadie* puede tocar el libro mayor — ni otro contable ni un inspector que solo viniera a mirar. No es el resultado de un protocolo sofisticado: es un candado físico en la puerta.
- Si el contable **muere con el bloc abierto**, su caída *es* el descarte: la transacción muere sin haber escrito nada.

Pero el problema de este capítulo aparece exactamente donde el modelo se rompe: cuándo el contable **está a media firma** — pasando a limpio el bloc — y se le **cae el bolígrafo** (`ApplyFallido`). Los tres primeros asientos ya están en el libro, el cuarto no, y *no hay forma de saber cuánto llegó*. Ese es el hueco que el WAL del cap. 28 cerrará: un bloc con **copia de cada trazo registrada antes de tocar el libro**.

El diagrama del ciclo de vida, con el `&mut` como barra de la puerta:

```
   &mut store (el cerrojo)
   │
   ▼ ┌────────────────────────────────────────────────┐
begin │  buffer (bloc) = Vec<Operacion>               │
──►   │  stage(op1); stage(op2); … — NUNCA toca el store │
      │   stage inválida → se expulsa, la tx sigue viva  │
      │   commit() → valida TODO →→→ apply → libro mayor │
      │      (si algo falla: A MEDIAS — cap. 28)         │
      │   rollback() → descarta buffer (gratis)          │
      └────────────────────────────────────────────────┘
```

**El momento ¡ajá!**: *«commit» no es un botón, es una **promesa** — y mientras el código no pueda distinguir un commit confirmado de un apply cortado, la D no existe. Hasta entonces, la A es «o todas o ninguna frente a la validación», no «o todas o ninguna frente al universo». El WAL del cap. 28 cerrará la distancia entre la promesa y el universo.*

## 27.4 Primera solución

La primera solución *ya la tienes y funciona*: es la de los capítulos 7-26. Cada `put_node`/`put_edge`/`delete_*` del `GraphStore` es su propia transacción. El test `autocommit_equivalente_a_la_operacion_directa` lo clava: `store.put_node(n)` a pelo y `autocommit(store, Operacion::PutNode(n))` dejan el **mismo grafo** — `node_count()` y `edge_count()` idénticos. O sea: la forma en que LiraDB ha escrito hasta hoy no es un modo distinto del nuevo mecanismo; es **una transacción de una sola operación**, con el begin y el commit implícitos.

Para escrituras sueltas, es perfecto. El autocommit no es un error: es un caso particular.

## 27.5 Sus límites

El problema no es una escritura suelta. El problema es **agrupar operaciones que dependen unas de otras**. Cuatro límites concretos:

1. **Un lote de 10 operaciones en el que la 5ª falla deja las 4 anteriores aplicadas.** El grafo queda a medias — un estado intermedio que nadie decidió.
2. **Dos operaciones relacionadas no se pueden agrupar.** Una arista cuyos extremos son dos nodos NUEVOS es válida solo si los tres entran juntos. Con autocommit, un fallo entremedias deja la arista huérfana o los nodos sin conectar — exactamente lo que el staging evitará.
3. **No hay manera de «deshacer» ni siquiera lo que aún no se aplicó.** Si te equivocas en la operación 3 de 5, ¿revientas las 2 primeras? No hay transacción que descartar; ya escribiste.
4. **No puedes saber *qué* prometes.** Sin un `InformeAcid`, «tenemos transacciones» es una afirmación sin matices — y el matiz es todo.

Y la pregunta incómoda que plantea el límite: **¿qué significa «a medias»?** Para contestarla con rigor no basta con escribir código; hay que construir el vocabulario para decir *cuánto está completo* cada promesa. Empecemos por ahí.

## 27.6 Solución evolucionada

La evolución tiene dos mitades que se complementan, tal y como manda el contrato del capítulo: **el vocabulario honesto** y **la primera maquinaria**.

**Primera mitad — el vocabulario típado.** La tesis es que el ACID no es un interruptor que la base de datos enciende. Es un conjunto de **cuatro promesas independientes**, cada una con su nivel. Eso exige un tipo de tres valores — `NivelGarantia::Ninguna | Parcial | Completa` — y no un booleano: un bool esconde *cuánto falta*. Y exige que el informe sea un **artefacto ejecutable**: `informe_acid()` devuelve estructura que los tests comparan. Si la documentación prometiera más que el código, los tests lo delatarían.

**Segunda mitad — la transacción como objeto, con staging.** `Transaccion::begin(&mut store)` toma el préstamo exclusivo del store. Las operaciones se **acumulan** como `Operacion` en un `Vec<Operacion>` privado — el buffer. Cada `stage` valida **eager** contra el store *más las operaciones anteriores* (un replay sobre una `Simulacion`): si la operación es inválida, se **expulsa** con `OperacionInvalida{indice, causa}` y la transacción **sigue viva con su prefijo válido**. El `commit` re-valida el buffer **entero** (el punto de no retorno, segunda cerradura) y, solo si todo es válido, lo aplica operación a operación. El `rollback` descarta el buffer — y como nada se aplicó, es **gratis por construcción**.

Y una pieza de honestidad crítica: si el **apply real** falla a mitad — el store dice «no» a algo que la simulación aprobó, o el proceso muere entre dos escrituras — el `commit` devuelve `ApplyFallido{indice, aplicadas, causa}`: te dice *cuántas* operaciones llegaron, pero **no puede deshacerlas**. Ese hueco es el motor del próximo capítulo.

## 27.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap27_transacciones.rs` (~1.460 líneas, 28 tests), abriendo la Parte VI. Léelo por partes — cada decisión tiene un porqué.

### El vocabulario ACID como tipo

```rust
pub enum GarantiaAcid { Atomicidad, Consistencia, Aislamiento, Durabilidad }
pub enum NivelGarantia { Ninguna, Parcial, Completa }
pub struct EntradaAcid { pub garantia: GarantiaAcid, pub nivel: NivelGarantia,
    pub como_esta_hoy: &'static str, pub capitulo_que_la_cierra: u8 }
```

Cinco decisiones que valen un porqué cada una:

- **Cada letra es una variante de `GarantiaAcid`** que sabe su letra (`'A'…'D'`), su nombre largo y su **definición PARA LiraDB** — no la de un manual genérico. El test `garantia_acid_letra_nombre_definicion` lo exige.
- **`NivelGarantia` tiene tres niveles, no un bool.** La tesis del capítulo es que **ninguna** letra está completa todavía; un bool te diría «sí tengo ACID» y ocultaría el progreso. Tres niveles permiten decir, con precisión, «parcial», «ninguna».
- **`EntradaAcid.capitulo_que_la_cierra`** apunta a qué capítulo de la Parte VI construye lo que falta. El informe no solo dice *dónde* estás: dice *hacia dónde*.
- **El informe es ejecutable**: `informe_acid()` devuelve las cuatro entradas, y el test `informe_acid_tiene_las_cuatro_letras_en_orden` exige que la concatenación de letras sea exactamente «ACID». Una promesa de base de datos que no se ejecuta en tests es marketing; aquí, mentir **rompe CI**.
- **`Display` del informe** imprime cada entrada en el formato `A — Atomicidad: parcial / naive: … (cap. 28 lo cierra)` y cierra con la línea honesta: *«(W: nada de esto es durable sin WAL — cap. 28)»*.

### El informe honesto

El test `informe_acid_es_honesto_sobre_el_estado_actual` es la **tesis ejecutada**:

```rust
// A: PARCIAL — staging «o todo o nada» frente a VALIDACIÓN; un fallo durante
//    el APPLY real deja el store a medias. La cierra el cap. 28.
// C: PARCIAL y trivial — solo invariantes ESTRUCTURALES, sin restricciones
//    declarativas; la C es un contrato COMPARTIDO con la app. Cap. 30.
// I: PARCIAL por diseño — no hay CONCURRENCIA; el préstamo &mut ES el cerrojo
//    (borrow checker). Cap. 30.
// D: NINGUNA — commit() muta RAM; hay sync/flush pero falta el protocolo WAL.
//    Cap. 28.
```

Fíjate en el juego de palabras que el tipo permite: la A **parcial** (frente a la validación sí, frente al apply no), la C **parcial y trivial** (no hay esquema, solo invariantes estructurales; la C es un contrato *compartido* entre motor y aplicación), la I **parcial por diseño** (no hay motores de locks que construir — el `&mut` lo hace gratis), y la D **ninguna** (hay piezas a disco, pero no protocolo; confundir piezas con protocolo sería la mentira más cara del capítulo). El test asegura que la prosa coincide con el código: busca los strings «apply», «estructurales», «concurrencia» y «RAM» dentro del informe.

### Las anomalías, como vocabulario

```rust
pub enum Anomalia { LecturaSucia, ActualizacionPerdida }
```

Hoy **no pueden ocurrir** — `por_que_no_pasa_hoy()` lo dice: *«no hay concurrencia: mientras vive una Transaccion, el préstamo exclusivo &mut del store impide que cualquier otro lector o escritor lo toque»*. Pero el cap. 30 las hará posibles y las combatirá con MVCC/2PL. La lógica es la del cap. 22 con `NegativeCycle`: **primero se nombra el enemigo, luego se construye la defensa**. Si dejásemos el enum para el cap. 30, el lector llegaría al MVCC sin saber qué está combatiendo.

### La operación como dato

```rust
pub enum Operacion {
    PutNode(Node),
    PutEdge(Edge),
    DeleteNode(NodeId),
    DeleteEdge(EdgeId),
}
```

Es la pieza clave del staging. Mientras las operaciones son **valores** que se acumulan en un buffer, la transacción puede validarlas **todas juntas** antes de tocar el store — y descartarlas limpio en rollback. Y hay una herencia deliberada escondida aquí: **el mismo shape que `RecordKind` del cap. 10**. El append-only log que construiste entonces anticipa el formato; la `Operacion` de este capítulo es su heredera directa. Eso no es una coincidencia: el cap. 28 serializará *exactamente esto* al WAL, y como la forma ya existe, no habrá que reinterpretar nada. El comentario del módulo lo llama «la semilla del WAL».

### La transacción como objeto — y el ciclo de vida en los tipos

```rust
pub struct Transaccion<'a> { store: &'a mut dyn GraphStore, buffer: Vec<Operacion> }
pub fn begin(store: &'a mut dyn GraphStore) -> Self { … }            // toma el cerrojo
pub fn commit(self)   -> Result<ResumenCommit, TransaccionError> { … }  // consume self
pub fn rollback(self) -> ResumenRollback { … }                       // consume self
```

La decisión más hermosa de todo el módulo está en **dos letras**: `self`. `commit` y `rollback` **consumen** la transacción. **no existe el objeto «transacción cerrada»** — usarla tras su cierre es error de compilación, no de runtime; y **anidar dos transacciones sobre el mismo store no compila** — `begin` pide `&mut dyn GraphStore`, así que el préstamo exclusivo del modelo «un único escritor» del brief lo **ejecuta el borrow checker**, gratis, sin una línea de locking (el test `transacciones_secuenciales_el_prestamo_se_libera` demuestra la única forma de encadenar: `commit`/`rollback` consumen la tx y liberan el préstamo).

Y otra propiedad que fluye del diseño sin código extra: **`Drop` de una transacción activa es rollback implícito y seguro por construcción** (el test `drop_implicito_es_rollback_seguro`). Como nada se aplica fuera del commit, abandonar el scope (un `?` temprano, un pánico, un olvido) simplemente descarta el buffer — el store no se tocó.

### El staging: valida eager, y el commit revalida por inducción

```rust
pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
    self.buffer.push(operacion);
    match validar_buffer(self.store, &self.buffer) {
        Ok(()) => Ok(()),
        Err(e) => {
            self.buffer.pop();   // el prefijo era válido: solo pudo fallar la última
            Err(e)
        }
    }
}
```

`stage` valida **eager**: si la operación añadida rompe una invariante, se **expulsa** y la transacción **sigue viva con su prefijo válido**. El test `stage_rechaza_duplicado_dentro_del_buffer_y_la_tx_sigue_viva` lo demuestra: metes un duplicado, recibes `OperacionInvalida{indice:1, causa:DuplicateNode(0)}`, y el buffer sigue con la operación válida que metiste antes — la transacción continúa usable.

`commit` hará lo contrario: **revalida el buffer ENTERO** antes de tocar el store. Es redundante con `stage` (cada stage validó su prefijo, y nada externo puede cambiar el store mientras lo tenemos prestado), pero es **barata** (O(n)) y robusta a refactors que rompan la inducción. Visualiza el `commit` como la *segunda cerradura*: aunque un refactor futuro rompiera la re-validación por etapas del `stage`, el commit sigue siendo la única puerta responsable de la decisión todo-o-nada. Es por esto que el test de la «op 3 de 5» siembra el buffer a mano (el test vive dentro del módulo) y espera el error **en commit**, no en stage: ejercita manualmente esa segunda cerradura.

### La validación como replay sobre una `Simulacion`

El corazón de la atomicidad naive es `validar_buffer`: un **replay** del buffer sobre una vista simulada del store, respetando el **orden**.

```text
Simulacion: nodos_creados | nodos_borrados | aristas_creadas (con extremos) | aristas_borradas
validar_buffer(store, buffer) → por cada op_k EN ORDEN:
  valida contra (store real ∪ efecto de op_1..op_{k-1})
  ├─ arista a nodo creado en el MISMO buffer: válida SI los nodos van ANTES
  ├─ delete_node arrastra aristas del store (out ∪ in) Y del buffer
  └─ si algo falla → OperacionInvalida{indice,causa}; no se aplica NADA
```

Tres decisiones dignas de nota:

- **El orden del buffer es parte del contrato** (como el orden de un log). El test `edge_a_nodo_creado_en_la_misma_tx_es_valido` demuestra la mitad buena — arista a dos nodos del *mismo* buffer si los nodos van antes — y `el_orden_importa_edge_antes_de_sus_nodos_es_invalido` la estricta: la arista antes que sus nodos se rechaza, porque en la vista simulada los extremos aún no «existen».
- **El `delete_node` arrastra aristas del store Y del buffer** (el test `edge_arrastrada_por_cascada_de_nodo_del_buffer` lo prueba). La cascada del cap. 8 no puede quebrarse a medio buffer: si creaste una arista en la tx y luego borras uno de sus nodos, esa arista del buffer muere también. Es la invariante del cap. 8, vista a través del tiempo.
- **Coste O(n·(n+E))** por `stage` — una elección **naive y documentada**, a favor de la claridad. El WAL del cap. 28 validará incrementalmente; aquí manda la legibilidad.

El test `error_en_la_operacion_3_de_5_no_aplica_nada` es la prueba de fuego de la A naive: buffer «a mano» con la op 3 inválida (edge 0→7 sin nodo 7), commit → `OperacionInvalida{indice:2, causa:InvalidEdgeEndpoints{0,7}}`, y el store **queda intacto** — `node_count()==0`. Atomicidad naive: funciona (frente a la validación).

### El apply y el error honesto a mitad

```rust
pub fn commit(self) -> Result<ResumenCommit, TransaccionError> {
    validar_buffer(self.store, &self.buffer)?;           // 2ª cerradura
    let mut resumen = ResumenCommit::default();
    for (indice, op) in self.buffer.iter().enumerate() {
        let ya = resumen.total_operaciones();
        match op {
            Operacion::PutNode(n) => match self.store.put_node(n.clone()) {
                Ok(()) => resumen.nodos_escritos += 1,
                Err(causa) => return Err(TransaccionError::ApplyFallido {
                    indice, aplicadas: ya, causa,          // ← honesto sobre "cuánto" }),
            },
            // … PutEdge / DeleteNode / DeleteEdge análogos …
        }
    }
    Ok(resumen)
}
```

Distinguir **dos errores que la naïveté del problema confunde** es la clave:

- `OperacionInvalida` — un problema de **validación**, que se descubre en `stage` (y se re-descubre en `commit`). La transacción **no se aplicó**, el prefijo válido sobrevive. Nada que deshacer.
- `ApplyFallido` — un problema del **apply real**: el store dijo «no» a lo que la `Simulacion` aprobó, o el proceso murió a mitad. Aquí `aplicadas` te dice **cuántas operaciones ya están escritas**. Y el store **quedó a medias** — sin log no hay vuelta atrás.

`aplicadas` existe precisamente para que sepas si tienes que investigar el store o no: esa información se perdería si guardaras un único `Error` con un booleano.

### El rollback barato vs el rollback imposible

```rust
/// ROLLBACK: descarta el buffer. El store no se ha tocado NUNCA…,
/// así que el descarte es limpio por construcción.
///
/// Ésa es la lección del staging: deshacer es trivial ANTES de aplicar.
/// Deshacer DESPUÉS de aplicar (un rollback de verdad, a mitad de
/// escrituras) exigiría un log — cap. 28.
pub fn rollback(self) -> ResumenRollback { … }
```

Esta es **la frontera exacta del capítulo**. Descartar el buffer — rollback *antes* de aplicar — es gratis por la propia estructura: nada se aplicó. Deshacer *después* de aplicar, a mitad de escrituras, **exige un log** — es la línea que el cap. 28 cruza. El test `rollback_no_aplica_nada` lo demuestra: la tx acumuló dos operaciones (un put y un delete), hace rollback, y el store queda **exactamente** como estaba.

### `autocommit` como función ejecutable

```rust
pub fn autocommit(store: &mut dyn GraphStore, operacion: Operacion)
    -> Result<ResumenCommit, TransaccionError>
{ let mut tx = Transaccion::begin(store); tx.stage(operacion)?; tx.commit() }
```

`autocommit` no es código nuevo: es el modo por defecto de los caps. 7-26 hecho **visible y ejecutable**. `autocommit_equivalente_a_la_operacion_directa` demuestra la equivalencia exacta con `store.put_node(n)`; `autocommit_operacion_invalida_no_toca_el_store` que una operación inválida ni toca el store. Ya no tienes que *creer* que `put_node(n)` es una transacción de una sola operación: lo puedes ver.

## 27.8 Prueba de fuego

No basta con que el código compile: la tesis — *«ninguna letra está completa, y los límites son reales»* — debe poder **fallar** si miente. La prueba de fuego son cuatro tests:

**TEST-TESIS A — `informe_acid_es_honesto_sobre_el_estado_actual`.** A=C=I=`Parcial`, D=`Ninguna`; ninguna `Completa`; los strings «apply», «estructurales», «concurrencia» y «RAM» aparecen — la prosa coincide con el código ejecutable.

**TEST-TESIS B — `error_en_la_operacion_3_de_5_no_aplica_nada`.** La atomicidad naive FUNCIONA frente a la validación: buffer con la op 3 inválida, commit → `OperacionInvalida{indice:2, causa:InvalidEdgeEndpoints}`, store intacto.

**TEST-TESIS C — `apply_fallido_deja_el_store_a_medias_gancho_al_cap_28`.** La atomicidad naive NO cubre el fallo del apply: `StoreQueFalla` (un decorador que falla en la 3ª escritura) hace que `commit()` devuelva `ApplyFallido{indice:2, aplicadas:2, causa:UnknownNode(usize::MAX)}` y el store tenga **2 nodos**. A medias, sin log. Es una **regresión inversa**: cuando llegue el WAL en el cap. 28, este test se *invertirá*.

**TEST-TESIS D — `panic_a_mitad_de_apply_deja_el_store_a_medias`.** El «corte de luz» simulado: `StoreQueFalla` con `con_panic=true`, `catch_unwind` atrapa el pánico, y `node_count()==1`. La primera escritura llegó; las dos siguientes no. Sin WAL **no hay forma de saber** si ese nodo pertenecía a una transacción confirmada o a una que murió a medias.

Otros tests citados, por si quieres seguir la verificación mientras lees: `informe_acid_tiene_las_cuatro_letras_en_orden`, `garantia_acid_letra_nombre_definicion`, `informe_acid_display_muestra_niveles_y_caps`, `anomalias_de_aislamiento_definidas`, `commit_aplica_todo_el_buffer`, `commit_vacio_es_noop_valido`, `stage_rechaza_edge_a_nodo_inexistente`, `delete_de_nodo_creado_en_la_misma_tx`, `delete_node_inexistente_rechazado`, `delete_edge_tras_cascada_de_delete_node_rechazado`, `recrear_nodo_tras_borrarlo_en_la_misma_tx`, `errores_display_y_std_error`, `resumenes_display`, `operacion_display`, `operaciones_vista_del_buffer`.

**Síntoma si te saltas el capítulo**: tus lotes «de 10 nodos» quedan a medias cuando uno falla, no distingues un apply-fallido de una validación, crees que `commit` es durable (y es RAM), y llegas al cap. 28 sin vocabulario para pedirle al WAL lo que necesitas.

## 27.9 Qué hemos sacrificado

1. **La D — durabilidad — es cero.** `commit()` solo muta RAM. El camino a disco existe (`Pager::sync` del cap. 12, `BufferPool::flush` del cap. 13), pero lo que falta es el **protocolo** write-ahead — y confundir piezas con protocolo sería el peor modo de fallo de una BD: un «tenemos durabilidad» que un `kill -9` desmiente.
2. **La A es naive.** «O todo o nada» vale frente a la **validación**; frente a un fallo del apply real a mitad, el store queda a medias (`ApplyFallido`). El staging no puede y *no pretende* arreglar eso.
3. **Aislamiento sin motores de locks** (porque no hace falta): la I la da el borrow checker en single-thread. Con concurrencia real (cap. 30), se revisa.
4. **Validación O(n·(n+E)) *por stage***, naive y documentada; un WAL valida incrementalmente. La C es «trivial»: solo invariantes estructurales, sin restricciones declarativas — en una BD real es un contrato *compartido* con la aplicación.

## 27.10 Cómo lo hace una BBDD real

Todo lo que aquí es «parcial», una base de datos real lo lleva el resto del camino — el de los tres capítulos que vienen:

- **El WAL (cap. 28)** es el bloc con copia de cada trazo *antes* de tocar el libro mayor. El estándar de hecho se llama **ARIES** (Mohan et al., «ARIES», ACM TODS 17(1), 1992). La `Operacion` de este capítulo es **exactamente** lo que el WAL serializa, bajo el framing del cap. 10.
- **La recuperación (cap. 29)** responde a la pregunta del test D — conservar o deshacer al arrancar. Härder & Reuter (1983, el paper que acuñó ACID) la formalizaron; ARIES la convirtió en replay con undo y redo.
- **MVCC y 2PL (cap. 30)** convierten la I «parcial por diseño» en aislamiento real bajo concurrencia. Berenson et al. («A Critique of ANSI SQL Isolation Levels», SIGMOD Record 24(2), 1995) mostraron que las definiciones ANSI eran incompletas — por eso hoy se usa el vocabulario de anomalías más rico (lectura sucia, lost update, no repetible, fantasma) que este capítulo abrió.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `apply` válida el buffer entero en `commit` si `stage` ya validó cada operación al acumularla?
- *Intermedio*: dibuja los dos cortes de la línea de tiempo del apply que los tests C y D representan; ¿en qué se diferencian el error tipado `ApplyFallido` y el pánico del «corte de luz»?
- *Experto*: ¿qué *es* exactamente «a medias»? Define el estado del store tras cada fallo, y justifica por qué ambos agujeros son irresolubles sin un log de lo ya aplicado.

## 27.11 Lo que te llevas

- **ACID no es un interruptor**: son cuatro promesas independientes, cada una con nivel (`Ninguna/Parcial/Completa`). `informe_acid()` es un artefacto **ejecutable** que los tests verifican — la documentación no puede prometer más que el código.
- **La transacción es un objeto con ciclo de vida**: `begin → stage* → commit|rollback`. El staging acumula `Operacion` en un buffer; el commit valida todo y aplica; el rollback descarta.
- **El ciclo de vida vive en los tipos**: `commit`/`rollback` consumen `self` — usar una tx cerrada o anidar dos no compila. Droppear una tx activa es rollback implícito seguro.
- **El borrow checker es el cerrojo**: el «un único escritor» del brief lo ejecuta `&mut`, gratis, sin locking.
- **El orden del buffer importa** (como el de un log): una arista a nodos del mismo buffer es válida si los nodos vienen antes; `delete_node` arrastra aristas del store y del buffer.
- **Rollback barato ≠ rollback imposible**: deshacer antes de aplicar es descartar el buffer; deshacer después exige un log. Esa frontera es el cap. 28.
- **Estado real hoy**: A parcial, C parcial y trivial, I parcial por diseño, D ninguna. Decirlo con honestidad es parte de la interfaz.

## 27.12 Ojo, cuidado con…

- **«Tengo ACID porque hice commit».** El commit no enciende nada: cada letra tiene su nivel, y `informe_acid()` es la pieza honesta. Si dices «tenemos transacciones», pregunta *cuál* letra y *hasta dónde*.
- **Confundir validación con apply.** `OperacionInvalida` se rechaza en `stage` (la tx sigue viva con su prefijo válido); `ApplyFallido` se descubre en `commit` (el store quedó a medias). Son errores DISTINTOS con consecuencias DISTINTAS — trata cada uno como lo que es, no como «la tx falló».
- **Esperar rollback completo.** El rollback de este capítulo es barato por construcción, *antes* de aplicar. El rollback real (después de aplicar) exige un log — no diseñes sistemas que asumen «rollback siempre funciona» sobre apply parcial.
- **Confundir `commit` con durable.** `commit()` muta RAM. El corte de luz borra lo confirmado y no hay WAL que lo recite. La D no se adivina: se construye (cap. 28).
- **Pensar que el aislamiento exige locks.** Con un solo hilo, el `&mut` *es* el cerrojo. No introduzcas un `Mutex` «por si acaso» — cuando llegue la concurrencia (cap. 30) se decidirá con criterio.

*Precisión de lenguaje*: *transacción* (objeto con ciclo de vida) vs *operación* (`Operacion`, la unidad del buffer); *staging* (acumular en privado) vs *apply* (escribir al store); *commit* (firma del bloc) vs *rollback* (tirar el bloc); *buffer* (estado privado de la tx) vs *store* (libro mayor compartido); *validación* vs *apply*; *autocommit* (tx de una op) vs *tx explícita*; *informe ACID* (artefacto tipado) vs *ACID* (el acrónimo); *anomalía* (patrón de fallo) vs *garantía* (lo que la BD promete); *nivel* (Ninguna/Parcial/Completa) vs *letra* (A/C/I/D).

## 27.13 Pin de batalla

> *«Un "commit" que el código no puede distinguir de un apply cortado no es una promesa — es una esperanza. La durabilidad no se declara: se protocoliza.»*

## 27.14 Si solo lees 30 segundos

ACID no es un interruptor: son cuatro promesas independientes con nivel. `informe_acid()` (ejecutable, auditado por tests) dice la verdad: **A parcial, C parcial y trivial, I parcial por diseño, D ninguna**. `Transaccion::begin(&mut store)` toma el préstamo exclusivo del store; el staging acumula `Operacion` en un buffer sin tocar el store; `commit` revalida todo y aplica; `rollback` descarta (gratis, porque nada se aplicó). `commit`/`rollback` consumen `self`, así que el ciclo de vida vive en los tipos: usar una tx cerrada o anidar dos no compila. El borrow checker es el cerrojo del «único escritor». El orden del buffer importa (arista después de sus nodos). Y lo honesto: si el apply falla a mitad, `ApplyFallido{aplicadas}` te dice cuánto llegó — pero sin log no hay vuelta atrás. Ese es el gancho del cap. 28 (WAL).

## 27.15 Una historia pequeña

Ana llevaba tres días «arreglando» un bug que no era un bug. Su analítica del cap. 26 materializaba una proyección y, de vez en cuando, el grafo quedaba con nodos huérfanos — aristas que apuntaban a ids que no existían. Descubrió que el problema no estaba en su código de análisis: estaba en *cómo escribía*. Un script rellenaba un batch de vértices con `store.put_node(...)` en bucle, y cuando el vértice 5 de 10 era un duplicado, el script seguía — el grafo se quedaba con los cuatro primeros, sin la arista que los unía, sin que nadie lo hubiera pedido. LiraDB no «borraba» nada: nunca había tenido la noción de *unidad*. Tras este capítulo, su script abría una `Transaccion`, metía los diez vértices y *la arista* en el buffer, y hacía `commit`. Si el quinto era inválido, la transacción se descartaba entera — o, mejor, `stage` se lo decía en el acto sin tocar el store. La moraleja, gritada en la bitácora: *no era un bug de datos; era una ausencia de acuerdo sobre qué es «un lote».*

## Ejercicios resueltos

**1. El fallo en la operación 3 de 5.** Buffer `PutNode(0,"P"), PutNode(1,"P"), PutEdge(0,0,1,"KNOWS"), …`. ¿Por qué el commit *no* falla aquí aunque valide el buffer entero? Porque la validación es un replay **sobre la `Simulacion`**: cuando se valida la arista, los nodos 0 y 1 ya fueron creados por las operaciones anteriores del buffer. Los extremos *existen en la vista simulada*. Es exactamente el caso `edge_a_nodo_creado_en_la_misma_tx_es_valido`. En cambio, si la arista fuera **antes** que sus nodos, los extremos aún no están en la simulación → `OperacionInvalida{indice:0, causa:InvalidEdgeEndpoints}` — `el_orden_importa_edge_antes_de_sus_nodos_es_invalido`. El orden del buffer no es cosmética: es parte del contrato (como el orden de un log).

**2. ¿Por qué el rollback del cap. 27 es gratis y el rollback «real» sería imposible sin log?** Porque nada se aplica fuera del commit. El buffer es privado y el store no se toca hasta la re-validación y el apply del `commit`. `rollback()` descarta el `Vec<Operacion>` — y como el store quedó intacto, no hay nada que deshacer (test `rollback_no_aplica_nada`). Deshacer *después* de aplicar es otro problema: si el apply ya escribió 2 de 5 operaciones y quieres revertirlas, necesitas saber *qué* se escribió y en qué orden — eso es un log. El `Log::append` del append-only cap. 10 es la pieza que falta: la única forma de «deshacer» es tener registrado lo que se hizo. Por eso `rollback()` pone la linde en «antes de aplicar»: más allá, la tarea ya es del WAL (cap. 28).

## Ejercicios propuestos

**Esencial (recordar/aplicar — predicción, no ejecución).** Sobre un store VACÍO, predice SIN ejecutar el resultado de commitear este buffer: `PutNode(0,"A"), PutEdge(0, 0, 1, "KNOWS"), PutNode(1,"B")`. Responde: (a) ¿tiene éxito el commit?; (b) si no, ¿qué variante de `TransaccionError`, con qué `indice` y qué `causa`; (c) el estado del store después. *Pistas*: (1) ¿existe el nodo 1 cuando se valida la arista (en el orden del buffer)?; (2) ¿el orden del buffer es libre?; (3) ¿la operación 3ª (que sí es válida por sí sola) llega a aplicarse si la 2ª falla? *Verificación*: `el_orden_importa_edge_antes_de_sus_nodos_es_invalido` (el caso que falla) y `edge_a_nodo_creado_en_la_misma_tx_es_valido` (cómo se corrige moviendo la arista al final). *Criterio*: predicción exacta + verificación corriendo ambos tests del workspace (`cargo test -p vol2-liradb --lib cap27`).

**Intermedio (analizar — mezcla caps. 8 y 10).** El comentario del módulo llama a la `Operacion` del cap. 27 «la heredera del `RecordKind` del cap. 10» y «la semilla del WAL del cap. 28». Razona (a) qué se almacenaba en `RecordKind` (cap. 10) con la **misma forma** que `Operacion`, y por qué esa forma compartida permite que el cap. 28 serialice el buffer al WAL **sin reinterpretar**; (b) por qué `delete_node` arrastra aristas del store **Y del buffer** (test `edge_arrastrada_por_cascada_de_nodo_del_buffer`) — ¿qué invariante del cap. 8 lo hace obligatorio, y qué le pasaría a la simulación si no lo hiciera?; (c) por qué el rollback del cap. 27 es barato (descartar `Vec<Operacion>`) pero el rollback real *después* de aplicar sería imposible sin un log — y por qué `Log::append` (cap. 10) es exactamente el «antes» que el WAL del cap. 28 codificará. *Pistas*: (1) ¿qué variantes pintaba `RecordKind` y qué hacen `stage`/`apply`?; (2) ¿qué dice `StoreError` sobre la cascada al borrar?; (3) ¿dónde encaja `Log::append` respecto al `commit`? *Verificación*: `delete_edge_tras_cascada_de_delete_node_rechazado` y la sección §32 de `MIGRATION-PATTERN.md`. *Criterio*: razonar la conexión entre tres capítulos **sin mirar el código**.

**Experto (crear — retrieval puro).** Parte 1, de memoria y sin pistas en el enunciado: reconstruye el `InformeAcid` COMPLETO del cap. 27 — las cuatro letras, su nivel (`Ninguna/Parcial`), su justificación honesta (qué garantiza hoy y qué no), y el capítulo que cierra cada brecha. Parte 2: escribe el test `nodo_recreado_despues_de_cascada_no_revive_sus_aristas` sobre el grafo 0→1 (edge 0) →2 (edge 1): tx `delete_node(1), put_node(1, "Renacido")`; `commit` verde; verifica `node_count()==2`, `edge_count()==0` y que las aristas muertas NO vuelven al recrear el nodo. *Pistas*: (1) ¿el `delete_node` arrastra las aristas adyacentes en la validación?; (2) ¿qué ve la `Simulacion` cuando vuelve a aparecer el id 1 — sus viejas aristas siguen en `aristas_borradas`?; (3) ¿cómo verifica el test que re-crear el nodo no re-crea la historia? *Verificación*: `recrear_nodo_tras_borrarlo_en_la_misma_tx` (base) + tu extensión propia. *Criterio*: informe exacto de memoria + test verde + la razón de por qué las aristas no se reviven.

## Para profundizar

- **Jim Gray, «The Transaction Concept: Virtues and Limitations» (VLDB 1981, pp. 144-154)** — el paper que abrió este capítulo: la transacción como transformación atómica, consistente y durable, con la anécdota de las reservas de vuelo y el agente de viajes. Fuente primaria de la anécdota de la esquina.
- **T. Härder & A. Reuter, «Principles of Transaction-Oriented Database Recovery» (ACM Computing Surveys 15(4), 1983, pp. 287-317; DOI 10.1145/289.291)** — el paper que **acuñó el acrónimo ACID**. Formalización de los conceptos que los caps. 28-29 heredan.
- **C. Mohan et al., «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging» (ACM TODS 17(1), 1992, pp. 94-162; DOI 10.1145/128765)** — el estándar moderno de WAL + undo/redo, el destino del cap. 28 (y su recuperación en el 29).
- **H. Berenson et al., «A Critique of ANSI SQL Isolation Levels» (SIGMOD Record 24(2), 1995, pp. 1-10; DOI 10.1145/223784.223785)** — por qué el vocabulario de anomalías (lectura sucia, lost update y compañía) que aquí abrimos es el que los niveles de aislamiento reales usan; base del cap. 30.
- **Gray & Reuter, «Transaction Processing: Concepts and Techniques» (Morgan Kaufmann, 1993)** — la referencia canónica de ACID, recovery y aislamiento de la que estos capítulos son un recuerdo en miniatura.
- **SQLite — documento oficial «Atomic Commit In SQLite»** — cómo un motor real, en producción, implementa atomicidad en una base de un solo fichero con un WAL; y **PostgreSQL docs, cap. 13 «Concurrency Control»** — aislamiento y niveles en el mundo real.
- Los **comentarios del módulo** `cap27_transacciones.rs` funcionan como prosa verificable: cada límite que anuncian tiene un test encima.

## Mini-diálogo: en guardia nocturna

> — O sea, que «commit» no me hace durable. ¿Entonces para qué me sirve?
>
> — Para lo que sí promete. El commit es la diferencia entre «metí una arista colgada que nadie pidió» y «estas cinco operaciones entran juntas o ninguna». La A *frente a la validación* ya es tuya.
>
> — Pero dices que la D es *ninguna*. Menuda venta.
>
> — Lo honesto. Decir «durable» con un corte de luz que lo borra sería el peor error posible de una base de datos — peor que admitir que falta. El informe te dice exactamente cuánto tienes y quién lo completa. El cap. 28 construye el WAL que cierra la D. Y luego el 30, el aislamiento de verdad.
>
> — ¿Y si me da miedo el borrow checker? Pensé que necesitaba un motor de locks.
>
> — Ahí está la gracia. Tu cerrojo ya está puesto: es `&mut`. Mientras viva la transacción, ni un lector ni otro escritor tocan el store, y te lo verifica el compilador en vez de un runtime. El día que llegue la concurrencia, revisamos.

---

*(Próximo capítulo: 28 — Write-ahead log. Has visto el hueco: cuando el apply se corta a mitad, `node_count()==2` y «nadie recuerda» qué faltaba. El WAL es el bloc con copia de cada trazo — se escribe ANTES de tocar el store, sobrevive al crash porque `fsync` ya forma parte del protocolo, y su registro serializa exactamente la `Operacion` del cap. 27. Cuando llegue, los dos tests que aquí AFIRMAN el store a medias se invertirán: el log sí recuerda.)*
