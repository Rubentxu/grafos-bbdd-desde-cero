# Capítulo 30 — Snapshots y concurrencia: MVCC limitado

> *«El lector no necesita ver lo último. Necesita ver lo que decidió ver cuando empezó a mirar.»*

## 30.0 La anécdota de la esquina

En 1978, David P. Reed presentó en el MIT una tesis doctoral titulada «Naming and Synchronization in a Decentralized Computer System». Era un trabajo sobre cómo coordinar nodos independientes en una red que no compartía reloj ni disco, y la pieza clave de su propuesta — el capítulo 4 — proponía algo que parecía obvio pero que nadie había formulado así: **en lugar de sobrescribir un dato cuando cambia, mantener las versiones anteriores y dejar que cada observador elija cuál ve**. La idea era tan simple que, durante años, los sistemas distribuidos la reinventaban una y otra vez sin saber que ya estaba escrita.

Cuarenta años después, esa idea — versiones múltiples, una por cada «momento lógico» — es el corazón de casi todas las bases de datos modernas. Se llama MVCC (Multi-Version Concurrency Control), y es lo que vamos a construir en este capítulo: la maquinaria que permite a LiraDB hacer lo que el cap. 27 no pudo — tener varios lectores leyendo MIENTRAS un escritor escribe, sin lecturas sucias, sin actualizaciones perdidas. La diferencia entre el «único escritor por el borrow checker» del cap. 27 y la «MVCC limitada» del cap. 30 no es un matiz: es el momento en que LiraDB deja de ser un sistema de un solo hilo lógico y empieza a hablar el idioma de las bases de datos reales.

Lo que NO arreglaremos aquí es igual de importante que lo que sí: en Snapshot Isolation, dos transacciones que leen y modifican elementos DISJUNTOS a partir del mismo snapshot pueden producir un resultado no serializable — la anomalía que la literatura llama write skew. Cerrar eso exige Serializable Snapshot Isolation con predicate locks (Cahill, Fekete, Liarokapis y Bernstein, «Serializable Snapshot Isolation in PostgreSQL», VLDB 2008), y eso queda para la Parte VIII. La honestidad de este capítulo es justamente ésa: promete lo que cumple y avisa de lo que no.

## 30.1 Objetivo

Al terminar este capítulo vas a entender por qué el modelo del cap. 27 — un único escritor por el borrow checker — es una **limitación benigna**, no una característica: simplifica el código y por construcción prohíbe las anomalías de aislamiento, pero deja sin resolver el caso del lector que quiere ver un estado coherente mientras otro escribe. Vas a construir la pieza que lo resuelve: una capa MVCC sobre el `MemoryStore` del cap. 8 que entrega snapshots coherentes sin bloquear al escritor, y vas a ver la frontera honesta donde la instantánea deja de bastar (write skew).

En concreto, vas a construir seis piezas:

1. `VersionNode` / `VersionEdge` — el registro de versión con su `ts_begin` y `ts_end`.
2. `MvccStore` — la capa MVCC sobre `MemoryStore` (la hexagonal del cap. 8).
3. `commit` con un solo `ts` por lote y validación propia (`validar_mvcc`).
4. `gc(hasta)` — la pieza de recuperación del espacio de versiones.
5. `NivelAislamiento` como vocabulario (lo que PROHÍBE y lo que DEJA PASAR cada nivel).
6. `GrafoEspera` para deadlocks — anzuelo para caps. futuros, no código en uso.

## 30.2 Problema

Volvamos al cap. 27, un momento. Teníamos una `Transaccion` que tomaba `&mut dyn GraphStore` durante toda su vida — begin, stage, commit, drop. El borrow checker era el cerrojo: mientras una transacción vivía, NINGÚN otro escritor y NINGÚN lector podía tocar el store. Eso significaba que las anomalías clásicas — `Anomalia::LecturaSucia` (una tx ve lo que otra NO ha confirmado), `Anomalia::ActualizacionPerdida` (dos tx escriben el mismo elemento y una pisa a la otra) — se definían como vocabulario pero NO PODÍAN ocurrir. El aislamiento era perfecto por construcción: no había concurrencia que aislar.

El problema es que ése no es el aislamiento que promete una base de datos. Una base de datos REAL debe permitir que un lector recorra el grafo MIENTRAS otro confirma cambios — y que el lector vea un estado coherente del momento en que empezó a mirar, no los cambios que están ocurriendo AHORA. El cap. 27 nos dio las palabras (`Anomalia`, `NivelAislamiento`); el cap. 28 nos dio la durabilidad; el cap. 29 nos dio la recuperación. Lo que nos falta es la pieza que une todo: la forma de tener **varios lectores concurrentes con un escritor**, sin lecturas sucias, sin actualizaciones perdidas.

La raíz del problema es la misma de siempre en sistemas concurrentes: cuando dos operaciones «leen y escriben» al mismo tiempo, hay tres opciones:

1. **Bloquear al lector mientras el escritor escribe**: garantiza consistencia, pero mata el paralelismo — el lector espera al escritor y la base de datos parece de un solo hilo.
2. **Bloquear al escritor mientras el lector lee**: mismo problema al revés — el escritor espera al lector, y dos transacciones que tardan en leer bloquean el sistema entero.
3. **Hacer que lean COSAS DISTINTAS**: el lector y el escritor no pisan la misma versión. El lector lee la versión que existía cuando empezó a mirar; el escritor crea una versión nueva que verá el siguiente lector.

La opción 3 es MVCC. Y es la única que escala.

## 30.3 Modelo mental

Vamos a usar dos analogías juntas, porque juntas lo ordenan todo:

**El fotógrafo con contador de exposición.** Imagina que cada lector es un fotógrafo con una cámara antigua. Cuando toma una foto, anota un número en el borde del negativo — su `ts`, su «número de exposición». Durante el revelado (su transacción) sólo ve lo que estaba en el visor EN ESE instante. El fotógrafo puede disparar N fotos simultáneas — cada una con su propio número — y cada revelado ve SU momento, no el de los demás. El escritor, mientras tanto, hace clic en su obturador (commit) y CAMBIA el visor; pero el revelado anterior ya está en su cubeta, terminado, y el nuevo visor no lo toca. Las fotos viejas que nadie quiere revelar se tiran a la basura — eso es `gc`.

**El editor de versiones de un documento** (tipo Git o Google Docs, pero a nuestra escala). Un `PutNode` NO pisa el `Node` anterior: RETIRA su versión actual (le pone `ts_end`) y APPENDIZA una nueva al final de la cadena. La historia del documento está disponible para quien sepa qué commit le interesa (`leer_nodo(id, ts)`). Un `DeleteNode` RETIRA la versión actual sin appendizar — la AUSENCIA es el nuevo estado: el documento ya no existe para los snapshots futuros, pero los antiguos lo siguen viendo.

Las dos analogías se necesitan mutuamente: el fotógrafo explica la CONSISTENCIA del snapshot («mi foto no cambia porque alguien más disparó»), el editor de versiones explica la IMPLEMENTACIÓN («cada elemento lleva una cadena de versiones»).

```
Nodo 0 «Ana»:
+-----------+-----------+--------+
| ts_begin=1| ts_begin=4| ts_begin=7|
| ts_end=4  | ts_end=7  | ts_end=None|
| «Ana»     | «Ana S.»  | «Ana S.»  |
+-----------+-----------+--------+
   ▲ ts=2 ve «Ana»               ▲ ts=8 ve «Ana S.»; ts=5 ve «Ana S.»
```

Cada cuadro es una `VersionNode`: tres campos — cuándo empezó a ser visible (`ts_begin`), cuándo dejó de serlo (`ts_end`), y qué contenía (`nodo`). La cadena está ordenada por `ts_begin` ASCENDENTE; la última entrada (la del final) es la versión actual — su `ts_end` es `None` hasta que otra la retire. Para encontrar la versión visible en un instante `ts`, recorremos la cadena del final al principio y devolvemos la primera versión con `ts_begin ≤ ts` y (`ts_end > ts` o `ts_end = None`).

## 30.4 Primera solución

La solución ingenua — la que escribiría un novato — es exactamente lo que parece: un `HashMap<NodeId, Node>` con un cerrojo de lectura (`RwLock`). El lector toma `read_lock()`, lee lo que hay, suelta el cerrojo. El escritor toma `write_lock()`, reescribe el nodo, suelta el cerrojo.

Funciona. Los tests pasan. Y tiene un problema que sólo se ve cuando se mide:

1. **El lector bloquea al escritor durante su recorrido.** Si un lector pide `iter_nodos()` y tarda 10 ms en consumirlos, los escritores que lleguen durante esos 10 ms ESPERAN. En una base de datos con analíticas largas, eso es mortal.
2. **El escritor bloquea a los lectores.** Si un escritor tarda 5 ms en confirmar 1.000 escrituras, los lectores que lleguen durante esos 5 ms ESPERAN. Es el mismo problema al revés.
3. **No hay garantía de coherencia DURANTE el recorrido del lector.** Si el lector empieza en `ts=1` y el escritor confirma un cambio en `ts=2` mientras el lector está en la mitad del recorrido, el lector puede ver una mezcla de los dos estados — el `iter_nodos()` del cap. 8 NO toma snapshot, recorre el HashMap EN EL MOMENTO. La anomalía `LecturaSucia` se materializa sin que nadie la pidiera.

Hay otra solución más sutil pero igual de ingenua: **bloquear a nivel de elemento**. Cada nodo y cada arista tiene su propio `Mutex`. El lector pide el lock del nodo X, lo lee, lo suelta. El escritor pide los locks de los nodos que toca, los modifica, los suelta. Es lo que llaman «cerrojos de granularidad fina».

Funciona mejor, pero abre una puerta nueva: los **deadlocks**. Si la tx A tiene el lock del nodo X y espera el del Y, y la tx B tiene el del Y y espera el del X, las dos se quedan bloqueadas para siempre. Solucionarlo exige el grafo de espera y la detección de ciclos — la pieza del cap. 30 que vamos a construir como anzuelo, no como código en uso.

Ninguna de las dos soluciones ataca el problema real: **el lector quiere ver un estado coherente, no el estado actual en cada instante**. Y para eso necesitamos algo cualitativamente distinto.

## 30.5 Sus límites

Las dos soluciones ingenuas comparten un límite conceptual: tratan la lectura como un «accidente físico» — un observador que mira en un instante y se va. Pero una base de datos NO funciona así. Una analítica que pregunta «¿cuántos nodos hay en el subgrafo X?» necesita ver un estado COHERENTE durante toda su ejecución, no los cambios que ocurren a media consulta.

Lo que necesitamos no es un cerrojo más fino ni más rápido. Necesitamos que el lector **tome una foto del grafo en el instante en que empieza a mirar**, y que esa foto no cambie mientras la trabaja. Eso es un snapshot. Y el snapshot, en MVCC, se materializa como un **número lógico** — el `Ts` — que el lector pasa a cada `leer_nodo(id, ts)`. La foto no es una copia de los datos: es un INSTANTE LÓGICO al que cada elemento responde con la versión que le correspondía.

Esta idea resuelve los tres problemas de un golpe:

1. **El lector no bloquea al escritor:** el lector clona la versión visible al `ts` y trabaja con su copia. El escritor modifica la versión actual (que el lector NO está mirando).
2. **El escritor no bloquea al lector:** la modificación crea una versión NUEVA con un `ts` mayor; el lector, con su `ts` viejo, sigue viendo la versión anterior.
3. **La coherencia del snapshot es por construcción:** la cadena es append-only (las versiones nuevas se añaden al final), la lectura es por valor (clona), y no hay un instante en que el grafo «cambie» a mitad del recorrido.

Y aquí viene la pieza clave que el cap. 27 no podía enunciar: **los lectores toman `&self` y el escritor toma `&mut self`, y AMBOS conviven sobre el mismo `MvccStore`**. El borrow checker no se queja: `&self` y `&mut self` son incompatibles para el MISMO dato, pero la MVCC los separa — los lectores leen cadenas (inmutables), el escritor modifica el `inner` y APPENDIZA a las cadenas. Es el patrón que convierte el «único escritor del cap. 27» en «N lectores concurrentes con un escritor».

## 30.6 Solución evolucionada

La solución evolucionada se reduce a tres reglas:

1. **Cada elemento lleva una CADENA de versiones (`ts_begin`, `ts_end?`, `valor`).** Las escrituras RETIRAN la versión actual (ponen su `ts_end`) y APPENDIZAN una nueva. Los deletes RETIRAN sin appendizar (la ausencia es el estado).
2. **Un snapshot es un `Ts = u64` monótono.** Una lectura en `ts` ve la versión con el MAYOR `ts_begin ≤ ts` Y `ts_end > ts` (o sin `ts_end`).
3. **El escritor es único (`&mut MvccStore`); los lectores son concurrentes (`&MvccStore`).** Sin cerrojos de lectura: la consistencia viene del versionado, no de los locks.

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap30_mvcc.rs`. Vamos a leerlo por partes, porque cada línea tiene un porqué.

### Las versiones

```rust
pub type Ts = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct VersionNode {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub nodo: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionEdge {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub arista: Edge,
}
```

`Ts` no es un timestamp físico (`SystemTime::now()`, ni `Instant::now()`). Es un **contador lógico**: el ORDEN de las escrituras. ¿Por qué no tiempo real? Porque dos commits no pueden coincidir en el orden del programa (uno va antes que el otro en el hilo del escritor); un reloj de tiempo real mezcla «orden» con «cuándo pasó» y abre problemas de deriva entre máquinas — la Parte VIII los cierra con vector clocks o true time, fuera del alcance del cap. 30. El contador es barato, monótono y portable: dos `Ts` son comparables sin ambigüedad.

`VersionNode` lleva tres campos. `ts_begin` es obligatorio: toda versión EMPEZÓ a ser visible en algún momento. `ts_end` es opcional: si es `None`, la versión SIGUE vigente; cuando otra la retire, se le pone `Some(ts)`. El campo `nodo` es el valor — clonado de la `Node` del cap. 7.

### El store MVCC

```rust
pub struct MvccStore {
    pub inner: MemoryStore,
    pub versiones_nodos: HashMap<NodeId, Vec<VersionNode>>,
    pub versiones_aristas: HashMap<EdgeId, Vec<VersionEdge>>,
    pub reloj: Ts,
}
```

Tres campos importantes. `inner: MemoryStore` es el **espejo material** — la versión «del momento presente» que las queries que NO piden snapshot (el código del cap. 8) usan. La MVCC vive ENCIMA; el `inner` es la verdad material para quien no sabe de versiones. `versiones_nodos` y `versiones_aristas` son los mapas de cadenas, una por cada elemento que haya pasado por el sistema. `reloj: Ts` es el contador: el siguiente `ts` a asignar.

La estructura del `MvccStore` es la hexagonal del cap. 8 probada una vez más: el `inner` es un `MemoryStore` CONCRETO hoy, pero cualquier backend que cumpla `GraphStore` serviría — un `FilePager`+CSR del cap. 14, un backend distribuido del cap. 40. El versionado no cambia; el backend sí. Es exactamente la inversión de dependencias que los caps. 8 y 26 establecieron.

### El commit con un solo timestamp

```rust
pub fn commit(&mut self, ops: &[Operacion]) -> Result<ResumenCommitMvcc, MvccError> {
    self.validar_mvcc(ops)?;
    let ts = self.siguiente_ts();
    let mut resumen = ResumenCommitMvcc {
        ts_asignado: ts,
        ..ResumenCommitMvcc::default()
    };
    for op in ops {
        match op {
            Operacion::PutNode(n) => {
                let chain = self.versiones_nodos.entry(n.id).or_default();
                if let Some(last) = chain.last_mut()
                    && last.ts_end.is_none()
                {
                    last.ts_end = Some(ts);
                    resumen.versiones_retiradas += 1;
                }
                chain.push(VersionNode {
                    ts_begin: ts, ts_end: None, nodo: n.clone(),
                });
                let _ = self.inner.delete_node(n.id);
                self.inner.put_node(n.clone()).map_err(MvccError::Store)?;
                resumen.nodos_escritos += 1;
            }
            // ... PutEdge, DeleteNode, DeleteEdge análogos
        }
    }
    Ok(resumen)
}
```

Tres decisiones que merecen explicarse:

**Un solo `ts` por lote.** La asignación se hace UNA vez, antes del bucle. ¿Por qué? Asignar UN `ts` por commit da una vista atómica: para cualquier `ts`, todos los elementos que esa transacción tocó son visibles en su nueva versión O todos en la vieja. Asignar uno por operación abriría una ventana en la que un lector ve la mitad del commit — un snapshot «mezclado». Volvería la `Anomalia::LecturaSucia` dentro de un commit.

**Validación PROPIA, no la del cap. 27.** La `validar_buffer` del cap. 27 asume INSERCIÓN ESTRICTA: rechaza `PutNode` de un id existente con `StoreError::DuplicateNode`. En MVCC, SOBREESCRIBIR es LEGAL — es lo que crea una nueva versión. Por eso el cap. 30 implementa `validar_mvcc` propia, que registra `PutNode` como «sim_creados_nodos.insert(n.id)» (no como error) y verifica `PutEdge` contra el estado visible (inner + simulación del buffer). Es la pieza que la calibración del módulo descubrió: 6 tests fallaban con `Validacion(DuplicateNode)` hasta que se separaron las dos políticas (MIGRATION §35).

**`delete-then-put` en el `inner`.** El `MemoryStore` del cap. 8 es de INSERCIÓN ESTRICTA. MVCC SOBREESCRIBE legalmente, así que el `inner` debe aceptar la nueva versión: `delete_node` (que es silencioso si no existe) seguido de `put_node`. La CADENA ya hizo su trabajo de versionado; el `inner` sólo es el espejo material.

### Las lecturas por valor

```rust
pub fn leer_nodo(&self, id: NodeId, ts: Ts) -> Option<Node> {
    let chain = self.versiones_nodos.get(&id)?;
    version_visible_node(chain, ts).map(|v| v.nodo.clone())
}

fn version_visible_node(chain: &[VersionNode], ts: Ts) -> Option<&VersionNode> {
    chain.iter().rev()
        .find(|v| v.ts_begin <= ts && v.ts_end.is_none_or(|t| t > ts))
}
```

La pieza fundamental. `leer_nodo` toma `&self` (no `&mut self`), busca la cadena del elemento y devuelve la versión visible al `ts` — clonada. La búsqueda recorre la cadena del final al principio (la versión actual está al final — es donde están los cambios más recientes) y devuelve la primera versión con `ts_begin ≤ ts` y (`ts_end > ts` o `ts_end = None`).

Que sea `&self` es la pieza clave. Permite que el lector clone la versión y se vaya, sin pedir nada al escritor. El escritor, mientras tanto, tiene `&mut self` y modifica el `inner` y APPENDIZA a las cadenas. El borrow checker admite `&self` y `&mut self` SIMULTÁNEOS mientras las operaciones que cada uno hace sean disjuntas — y la MVCC las hace disjuntas por construcción: el lector no toca el `inner` ni el `último elemento` de la cadena (lee una versión histórica cualquiera); el escritor no toca las versiones retiradas (las deja en paz). La única zona que ambos necesitan — la cola de la cadena — la gestiona el escritor con su `&mut`, y el lector la evita por construcción.

### La garbage collection

```rust
pub fn gc(&mut self, hasta: Ts) -> usize {
    let mut eliminadas = 0usize;
    let mut vacias: Vec<NodeId> = Vec::new();
    for (id, chain) in &mut self.versiones_nodos {
        let antes = chain.len();
        chain.retain(|v| match v.ts_end {
            None => true,
            Some(t_end) => t_end >= hasta,
        });
        let quitadas = antes - chain.len();
        eliminadas += quitadas;
        if chain.is_empty() {
            vacias.push(*id);
        }
    }
    for id in &vacias {
        self.versiones_nodos.remove(id);
    }
    // ... análogo para aristas
    eliminadas
}
```

La invariante de `gc` es la pieza que un programador con prisa se saltaría: ningún snapshot con `ts ≥ hasta` puede ver una versión retirada con `ts_end < hasta` (los `ts` son monótonos — `siguiente_ts` siempre crece). Si la versión actual (`ts_end = None`) tiene `ts_begin < hasta`, NO se quita: snapshots futuros la necesitan. Cuando una cadena queda totalmente vacía, su entrada del mapa se elimina también — el elemento ya no existe.

La regla mnemotécnica: **`gc(hasta)` borra el PASADO visible para NADIE**. Borra las versiones que ningún snapshot — actual ni futuro — puede ver. La memoria se libera cuando ya no hay observadores que la necesiten.

### El grafo de espera

```rust
pub struct GrafoEspera {
    aristas: Vec<(TxIdLocal, TxIdLocal, Recurso)>,
}

impl GrafoEspera {
    pub fn agregar_espera(&mut self, esperador: TxIdLocal, tenedor: TxIdLocal, recurso: Recurso) {
        self.aristas.push((esperador, tenedor, recurso));
    }
    pub fn quitar_tx(&mut self, tx: TxIdLocal) {
        self.aristas.retain(|&(e, t, _)| e != tx && t != tx);
    }
    pub fn detectar_ciclo(&self) -> Option<Vec<TxIdLocal>> {
        // DFS con tres colores (blanco/gris/negro) — O(V+E)
        // ...
    }
}
```

Aunque HOY no puede haber ciclos en este grafo — `&mut self` en el commit impide que dos escritores compitan por cerrojos — la estructura existe y se DEMUESTRA. Es anzuelo: cuando llegue la Parte VIII y el `MvccStore` acepte varios escritores concurrentes, el gestor de cerrojos que los coordine enchufa `agregar_espera` cuando un escritor pide un recurso que otro tiene, `quitar_tx` cuando termina, y `detectar_ciclo` cada vez que un escritor se bloquea. La detección de ciclos es DFS con tres colores (blanco/gris/negro) en O(V+E): si al descender encontramos un nodo gris, hay ciclo y devolvemos los nodos desde el gris en adelante.

El test `grafo_espera_detecta_ciclo_de_dos` lo demuestra: T1 espera a T2 por el nodo 10, T2 espera a T1 por el nodo 11 — `detectar_ciclo` devuelve `Some([1, 2, 1])`. `quitar_tx(1)` rompe el ciclo. Aunque no esté enchufado al `MvccStore` hoy, la pieza existe y funciona.

## 30.7 Código completo ejecutable

El código del capítulo vive en `liradb-workspace/crates/vol2-liradb/src/cap30_mvcc.rs`. Son ~1.158 líneas, 21 tests en `tests_mvcc` que pasan `cargo test -p vol2-liradb cap30` con ALL_GREEN. La estructura se resume en:

- **Tipos base**: `Ts = u64`, `VersionNode`, `VersionEdge`, `NivelAislamiento::{LecturaSucia, Instantanea, Serializable}` con método `prohibe()` que enuncia qué anomalías quita cada nivel.
- **Errores**: `MvccError::{Validacion, Store}` con `From<TransaccionError>` y `source()` para componer la cadena de errores.
- **El store**: `MvccStore` con `inner: MemoryStore`, `versiones_nodos/aristas`, `reloj`; métodos `new()`, `reloj()`, `siguiente_ts()`, `leer_nodo/arista`, `iter_nodos/aristas`, `commit`, `validar_mvcc`, `gc`.
- **El resumen**: `ResumenCommitMvcc { ts_asignado, nodos_escritos, aristas_escritas, versiones_retiradas }` con `Display` que produce un reporte humano.
- **El grafo de espera**: `Recurso::{Nodo, Arista}`, `GrafoEspera` con `nuevo`, `agregar_espera`, `quitar_tx`, `detectar_ciclo`, `aristas` para inspección; `dfs` interno con tres colores.
- **La re-valoración ACID**: `informe_acid_post_mvcc()` que devuelve `Vec<EntradaAcid>` con el aislamiento AVANZADO (lectura sucia y lost update prohibidas, write skew sobrevive — closer = 40).

Veamos el test central — la demostración clave del capítulo:

```rust
#[test]
fn varios_snapshots_coexisten_sin_bloquearse() {
    let (mut mv, ts1) = store_basico();
    // Lector A lee en ts1.
    let snap_a = mv.leer_nodo(0, ts1).unwrap();
    assert_eq!(snap_a.labels, vec!["Person".to_string()]);

    // Commit que reescribe el nodo 0.
    let mut n = mv.leer_nodo(0, ts1).unwrap();
    n.labels = vec!["Cambiado".to_string()];
    let _ = mv.commit(&[Operacion::PutNode(n)]).unwrap();

    // Lector B (que tomó su foto EN ts1 antes) sigue viendo lo suyo.
    let snap_b = mv.leer_nodo(0, ts1).unwrap();
    assert_eq!(snap_b.labels, vec!["Person".to_string()]);

    // Y nadie bloqueó a nadie: las dos lecturas son por valor y la
    // escritura ocurrió entre medias sin invalidar el snapshot A.
    assert_eq!(snap_a.labels, snap_b.labels);
}
```

Tres líneas que valen un capítulo: lector A lee, commit ocurre entre medias, lector B lee — ambos ven lo mismo. Es lo que el cap. 27 no podía hacer (el borrow checker hubiera bloqueado al escritor mientras A tenía su referencia). Es la promesa del cap. 30 cumpliéndose por construcción: la cadena es append-only, la lectura es por valor, y los `ts` son monótonos.

## 30.8 Prueba de fuego

La prueba de fuego tiene cinco tests-tesis que DEMUESTRAN cada pieza del capítulo:

- **`leer_en_snapshot_anterior_devuelve_la_version_visible`** (807-826): un nodo se reescribe en un commit posterior; el snapshot anterior SIGUE viendo la versión vieja. La diferencia entre MVCC y un store que sobrescribe (el cap. 27 sin MVCC pierde la versión).
- **`varios_snapshots_coexisten_sin_bloquearse`** (928-950): lector A lee, commit ocurre, lector B (foto antes del commit) lee — ambos ven lo mismo. La promesa del capítulo: N lectores + 1 escritor sin bloqueos de lectura.
- **`niveles_prohiben_las_anomalias_esperadas`** (994-1011): `NivelAislamiento::prohibe()` dice la verdad — Instantanea prohíbe lectura sucia y lost update, DEJA PASAR write skew; Serializable prohíbe las tres (SSI con predicate locks cerraría write skew).
- **`grafo_espera_detecta_ciclo_de_dos`** (1031-1041) y **`grafo_espera_detecta_ciclo_de_tres`** (1044-1053): el `GrafoEspera` detecta ciclos T1→T2→T1 y T1→T2→T3→T1; `quitar_tx` rompe el primero. Aunque no se usa en producción HOY, la pieza está testeada.
- **`informe_post_mvcc_avanza_el_aislamiento`** (1115-1146): el `informe_acid_post_mvcc()` documenta que el aislamiento AVANZA (lectura sucia y lost update pasan a prohibidas) — pero write skew sigue pasando y el closer del aislamiento salta al cap. 40.

**Síntoma si el lector se salta este capítulo**: su `MvccStore` sobrescribirá el `Node` anterior (no habrá cadena); un commit con `ts=2` no podrá distinguir «versión reescrita en `ts=2`» de «versión inicial que sigue vigente»; la palabra «snapshot» será un eufemismo para «el estado en RAM ahora mismo». Y — lo más importante — NO entenderá por qué write skew es un problema HONESTO: creerá que MVCC «lo arregla todo» y diseñará transacciones disjuntas confiando en la garantía equivocada.

## 30.9 Qué hemos sacrificado

Toda estructura tiene un precio. MVCC no es gratis:

1. **Memoria para las cadenas**: cada reescritura de un elemento deja una versión retirada en la cadena hasta que `gc` la purgue. En un grafo muy reescrito (p.ej. un nodo que cambia sus etiquetas en cada commit), la cadena crece monótonamente. La `gc` es la pieza que lo controla, pero exige disciplina: llamarla con un `hasta` adecuado (mínimo `mv.reloj()` para vaciar todo lo no visible para snapshots futuros).
2. **Sin timestamp físico**: el `Ts` es orden del programa, no medida de tiempo. Si dos nodos de una red confirman cambios «a la vez» en tiempo real, sus `Ts` son los que el escritor local asignó — distintos y no comparables en términos temporales. La Parte VIII cierra esto con vector clocks; aquí lo admitimos como limitación.
3. **Concurrencia de escritores NO resuelta**: sigue habiendo un único escritor lógico (`&mut self`). MVCC multiplica los LECTORES, no los escritores. Un motor real con varios escritores exige un gestor de cerrojos — la pieza del `GrafoEspera` está construida como anzuelo, pero NO se enchufa al `MvccStore` HOY.
4. **Write skew ABIERTO**: en Snapshot Isolation, dos transacciones que leen y modifican elementos DISJUNTOS a partir del mismo snapshot pueden producir un resultado no serializable. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — fuera del alcance. La `informe_acid_post_mvcc()` lo dice sin ambigüedades: «write skew sigue pasando — Serializable SI con predicate locks lo cerraría», closer = 40.
5. **GC manual**: el capítulo enseña la operación `gc`; integrarla como tarea programada es integración del motor (no entra aquí). Un usuario que olvide llamar `gc` acabará con un `MvccStore` que crece sin parar.

## 30.10 Cómo lo hace una BBDD real

MVCC no es una rareza académica — es la elección por defecto de casi todas las bases de datos modernas, con variantes:

- **PostgreSQL** implementa MVCC desde 8.0 (2005): cada fila lleva `xmin` y `xmax` (los equivalentes de nuestro `ts_begin` y `ts_end`); la «instantánea» de una transacción se materializa como una lista de `xmin` visibles. El nivel por defecto es «Read Committed» (cada statement ve su propio snapshot, NO la transacción entera); «Repeatable Read» usa SI (prohibe lectura sucia y lost update, igual que nosotros); «Serializable» implementa SSI con predicate locks sobre los predicados leídos (Cahill et al. 2008 — el paper que el cap. 30 cita como «lo que cerraría write skew»).
- **CockroachDB** y **YugabyteDB** llevan MVCC al territorio distribuido: cada nodo asigna su propio `Ts` con un reloj HLC (Hybrid Logical Clock), combinación de tiempo físico y contador lógico. Es lo que llamábamos «la frontera con vector clocks» — la Parte VIII.
- **FoundationDB** implementa Serializable SI sobre un MVCC con `read_version` por lectura y conflict ranges para detectar write skew (Ports & Grittner, 2016). Su `detectar_conflicto` es el equivalente industrial del `detectar_ciclo` de nuestro `GrafoEspera`.
- **Wu et al., «An Empirical Evaluation of In-Memory Multi-Version Concurrency Control»**, VLDB 2017: una evaluación sistemática de las variantes MVCC (timestamp ordering, snapshot isolation, serializable) sobre cargas OLTP. Conclusión: MVCC gana en throughput a 2PL bajo concurrencia media-alta, y SSI añade menos overhead del que la intuición sugiere — pero la implementación importa más que la teoría.
- **David P. Reed**, «Naming and Synchronization in a Decentralized Computer System», MIT 1978: la génesis. La propuesta formal de versiones múltiples en sistemas descentralizados. Lo que el cap. 30 cita como el origen de la idea — y la conexión histórica directa con la Parte VIII.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: predice el `ResumenCommitMvcc` de tres commits consecutivos sobre `store_basico()` (un PutNode que reescribe, un DeleteNode). Explica por qué la cadena del nodo 0 tiene longitud 1, 2, 1 — no 1, 2, 0.
- *Intermedio*: implementa `leer_en_snapshot_exterior` que devuelva `None` si NO hay ninguna versión visible para el `ts` (la lógica actual devuelve `Some` aunque el `ts` sea muy anterior). ¿Cómo distinguirías «el nodo nunca existió» de «el nodo se borró»?
- *Experto*: implementa `mvcc_iter_nodos_entre(desde, hasta)` que devuelva los nodos cuyo `ts_begin` está en el rango — el primer ladrillo de un «time-travel query» estilo SQL Server `AS OF`. Conecta con el gancho del cap. 40: ¿qué cambia cuando los `Ts` vienen de MÁQUINAS DISTINTAS?

## 30.11 Lo que te llevas

- **MVCC** es la maquinaria que da MATERIALIDAD al «snapshot» del cap. 26: una foto lógica no es una copia, es un `Ts` al que cada elemento responde con la versión visible.
- **El `Ts` es orden del programa, no medida de tiempo**: dos `Ts` son comparables sin ambigüedad dentro de un escritor; entre máquinas, la Parte VIII los reconcilia.
- **Las cadenas de versiones** son append-only: la última entrada es la actual (`ts_end = None`); las escrituras RETIRAN la actual y APPENDIZAN; los deletes RETIRAN sin appendizar (la ausencia es el estado).
- **`&self` para lectores, `&mut self` para el escritor**: el borrow checker admite la convivencia porque las operaciones son disjuntas por construcción. Es el patrón que multiplica los lectores del cap. 27 sin tocar la regla «un único escritor».
- **Instantanea PROHÍBE lectura sucia y lost update, DEJA PASAR write skew**: la frontera honesta. Cerrar write skew exige Serializable SI con predicate locks — fuera del alcance del Vol.II.
- **El `GrafoEspera` existe sin uso HOY**: anzuelo para el gestor de cerrojos del cap. 40. La detección de ciclos es DFS 3 colores en O(V+E).
- **El `inner` es el espejo material**: las queries que NO piden snapshot lo usan; la MVCC vive ENCIMA y el `delete-then-put` mantiene la consistencia entre ambos mundos.

## 30.12 Ojo, cuidado con…

- **Confundir `mv.reloj()` con un `ts` válido**: `reloj()` es el SIGUIENTE timestamp a asignar — NO una snapshot válida (todavía no hay versión visible para ese `ts`). Usa `resumen.ts_asignado` del commit inicial o de un commit anterior. Síntoma: `leer_nodo(2, mv.reloj())` devuelve `None` cuando debería devolver la versión actual.
- **Asumir que `validar_buffer` del cap. 27 sirve para MVCC**: la del 27 rechaza `PutNode` de id existente; la MVCC SOBREESCRIBE. Síntoma: 6 tests fallaban con `Validacion(DuplicateNode)` durante la calibración. El fix fue `validar_mvcc` PROPIA con semántica de sobreescritura.
- **Creer que la versión «más reciente» es siempre la actual**: la versión visible para un `ts` se calcula por la condición `ts_begin ≤ ts ∧ (ts_end > ts ∨ ts_end = None)`, NO por «el último siempre». Olvidar el caso de la versión retirada devuelve `None` cuando había versión.
- **Olvidar que `gc(hasta)` puede vaciar cadenas**: si TODAS las versiones quedan retiradas y `ts_end < hasta`, la entrada del mapa se ELIMINA. Síntoma: el test `gc_elimina_cadenas_vacias_cuando_el_elemento_se_borra` falla si se asume que la cadena sobrevive.
- **Pensar que `write skew` es un bug**: NO — es la frontera HONESTA de Snapshot Isolation. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008). El cap. 30 lo DEJA ABIERTO a propósito. Síntoma: diseñar transacciones disjuntas creyendo que MVCC las serializa — produce un resultado no serializable.

## 30.13 Pin de batalla

> *«La consistencia del snapshot no viene de un cerrojo que sostiene al lector en su sitio. Viene de que cada elemento lleva la historia de quién lo vio, y de que el lector elige qué historia le interesa.»*

## 30.14 Si solo lees 30 segundos

MVCC resuelve la anomalía histórica del Vol.II — un único escritor por el borrow checker — permitiendo que N lectores lean MIENTRAS un escritor escribe, sin lecturas sucias ni actualizaciones perdidas. Cada elemento lleva una cadena de versiones `{ts_begin, ts_end?, valor}`; un snapshot es un `Ts` lógico (un `u64` monótono), y `leer_nodo(id, ts)` devuelve la versión visible ESE instante. El escritor toma `&mut self` y APPENDIZA una versión nueva; los lectores toman `&self` y clonan — la convivencia por el borrow checker es la forma del aislamiento. Instantanea PROHÍBE lectura sucia y lost update, DEJA PASAR write skew. El `GrafoEspera` para deadlocks existe como anzuelo para caps. futuros, no como código en uso HOY.

## 30.15 Una historia pequeña

Cuando llegamos al cap. 27 con el primer `commit(&mut self)` funcional, pensábamos que teníamos «transacciones». Teníamos vocabulario ACID, `Anomalia::LecturaSucia` y `Anomalia::ActualizacionPerdida` bien definidas, y la promesa — todavía vacía — de que el aislamiento mejoraría. Lo que NO teníamos era la posibilidad de demostrar la promesa: el borrow checker era el cerrojo, y bajo el cerrojo no había concurrencia que aislar.

Fue el cap. 30 el que cerró el círculo. El momento clave no fue técnico: fue cuando el test `varios_snapshots_coexisten_sin_bloquearse` pasó por primera vez. Lector A lee, commit ocurre, lector B lee — ambos ven lo mismo. Era algo que el cap. 27 NO PODÍA hacer, y que ahora era trivial: dos `&self` y un `&mut self` sobre el mismo `MvccStore`, sin más ceremonia que pasarle el `ts` correcto. La anomalía histórica del Vol.II estaba resuelta — no por añadir cerrojos, sino por QUITARLOS: la cadena es el cerrojo, no un lock manager.

Y aquí aprendimos también la honestidad de la Parte VI: el `informe_acid_post_mvcc()` dice, sin ambigüedad, que el aislamiento AVANZA pero NO SE CIERRA. Write skew sobrevive, y cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — la pieza que dejaremos para la Parte VIII, cuando la concurrencia REAL de varios procesos abra la puerta al skew y el `GrafoEspera` aquí construido encuentre su uso.

## Ejercicios resueltos

**1. ¿Por qué la validación del cap. 27 (`validar_buffer`) no sirve para MVCC?**

Porque `validar_buffer` del cap. 27 asume INSERCIÓN ESTRICTA: rechaza `PutNode` de un id existente con `StoreError::DuplicateNode`. En MVCC, SOBREESCRIBIR un nodo es LEGAL — es lo que crea una nueva versión en la cadena (la versión anterior se RETIRA con `ts_end` y la nueva se APPENDIZA). Si reutilizáramos `validar_buffer`, un commit que reescribe el nodo 0 fallaría con `Validacion(DuplicateNode)` y la MVCC no podría hacer su trabajo. Por eso el cap. 30 implementa `validar_mvcc` PROPIA, que registra `PutNode` como `sim_creados_nodos.insert(n.id)` (no como error) y verifica `PutEdge` contra el estado visible (inner + simulación del buffer). Las dos políticas — inserción estricta (cap. 27, `MemoryStore`) y sobreescritura (cap. 30, MVCC) — son DIFERENTES, y la lección de la calibración (6 tests fallaban) lo demostró empíricamente.

**2. ¿Por qué `leer_nodo(0, mv.reloj())` puede devolver `None` cuando el nodo existe?**

Porque `mv.reloj()` es el SIGUIENTE `ts` a asignar — todavía no hay ninguna versión con `ts_begin <= mv.reloj()` (la versión actual tiene `ts_begin` igual al del último commit, y `ts_end = None`, así que es visible para `ts >= ts_begin`, pero el SIGUIENTE `ts` aún no es visible para sí mismo — no hay versión que cumpla `ts_begin <= mv.reloj()` y `ts_end > mv.reloj()`). Es la diferencia entre «el próximo número que se asignará» y «un número que ya es válido como snapshot». La forma correcta es capturar `resumen.ts_asignado` del commit inicial o de un commit anterior y usar ESE como snapshot. Esta trampa aparece en §35 de MIGRATION-PATTERN como una de las lecciones de calibración.

## Ejercicios propuestos

**Esencial.** Predice, ANTES de ejecutar, cuántas versiones retira cada uno de los tres commits consecutivos y cuál es el `ts_asignado`. Parte del `store_basico()` (3 nodos, 2 aristas, `ts=1`); ejecuta un segundo commit con `[PutNode(Nodo::new(0, "Renacido"))]` y comprueba el `ResumenCommitMvcc` (nodos_escritos=1, versiones_retiradas=1, ts_asignado=2); luego un tercero con `[DeleteNode(0)]` y comprueba (nodos_escritos=0, versiones_retiradas=1, ts_asignado=3 — un delete RETIRA sin appendizar). Pistas: ¿qué hace `DeleteNode` con la cadena del elemento? ¿`reloj()` es lo mismo que `ts_asignado`? Tras el `DeleteNode`, ¿cuántas entradas tiene la cadena del nodo 0? Criterio: predicción exacta de los tres `ResumenCommitMvcc` y de la longitud de la cadena (1, 2, 1 respectivamente).

**Intermedio.** Tomando la cadena del nodo 0 tras los tres commits anteriores (`{ts_begin=1, ts_end=3, "Ana"}` y vacío tras el delete), explica por qué `leer_nodo(0, 1)` devuelve la versión inicial, por qué `leer_nodo(0, 2)` también, y por qué `leer_nodo(0, 3)` devuelve `None`; conecta con `Anomalia::ActualizacionPerdida` del cap. 27 (¿qué habría pasado SIN MVCC si dos tx leen `leer_nodo(0, 2)` y reescriben?); conecta con la `RecoveryError` del cap. 29 (¿qué ganaría la MVCC sobre el WAL si el sistema cae a mitad del commit?). Pistas: ¿cuál es la versión con `ts_begin ≤ ts ∧ ts_end > ts` para cada `ts`? ¿Qué condición del cap. 27 PROHIBIRÍA la actualización perdida? ¿`delete-then-put` es atómico ante un crash? Criterio: tres predicciones de `leer_nodo` correctas + una frase que conecte `Anomalia::ActualizacionPerdida` con la condición de visibilidad + una frase que conecte `delete-then-put` con la fragilidad ante crash (el cap. 29 lo cubre; el cap. 30 NO).

**Experto.** Implementa `gc(hasta)` sobre un `MvccStore` con esta historia: `ts=1` escribe nodo 0 («Ana»), `ts=2` reescribe («Ana S.»), `ts=3` reescribe («Ana Sofía»), `ts=4` borra; predice cuántas versiones se quitan con `gc(4)` y `gc(5)` y demuestra con `cargo test` que coincide con `gc_descarta_versiones_retiradas_antiguas`. LUEGO razona al revés: dado un `MvccStore` con N reescrituras del mismo nodo y SIN deletes, ¿cuál es el MÍNIMO `gc(hasta)` que vacía todas las versiones retiradas? Pistas: tras `ts=4` (delete), ¿cuál es el `ts_end` de la versión inicial? ¿La cadena queda con longitud 1 o 0 tras el delete? ¿`gc(hasta)` quita la versión con `ts_end = None`? Criterio: predicciones exactas + identificación de que la versión con `ts_end = None` (la «actual») NO se quita NUNCA por `gc` (los snapshots futuros la necesitan) + reconocimiento de que el `gc` mínimo para vaciar todo es `mv.reloj()` (el siguiente `ts` a asignar — más allá, ningún snapshot puede estar vivo).

## Para profundizar

- **David P. Reed**, «Naming and Synchronization in a Decentralized Computer System», MIT 1978 — la génesis de las versiones múltiples en sistemas descentralizados. El cap. 4 de la tesis es donde está la idea; el resto es la conexión con la Parte VIII.
- **Cahill, Fekete, Liarokapis, Bernstein**, «Serializable Snapshot Isolation in PostgreSQL», VLDB 2008, DOI 10.14778/1454159.1454166 — el algoritmo que cierra el write skew que el cap. 30 DEJA ABIERTO. La pieza que el Vol.II cita como anzuelo al cap. 40.
- **PostgreSQL Global Development Group**, «13.2. Transaction Isolation», PostgreSQL documentation — las definiciones operativas de Read Committed / Repeatable Read / Serializable. El vocabulario del que `NivelAislamiento` es una traducción.
- **Wu, Arulraj, Lin, Xi, Pavlo, Chen, Lee, Song, Feng, Lohman, Xu, Zhao, Chen**, «An Empirical Evaluation of In-Memory Multi-Version Concurrency Control», VLDB 2017 — la evidencia moderna de que MVCC es la elección de la mayoría de motores analíticos en RAM y de las diferencias prácticas entre variantes.
- **Ports & Grittner**, «Serializable Snapshot Isolation in FoundationDB», 2016 — la implementación industrial de SSI con `read_version` y conflict ranges; el puente directo al cap. 40.
- **Código fuente de PostgreSQL** (`heapam.c`, `tqual.c`): la implementación canónica de MVCC con `xmin`/`xmax`, comentada con un detalle exquisito.
- **Liran Einav**, «The Art of Writing Efficient Database Code» (workshop, 2019) — la conferencia que reconcilia las intuiciones de MVCC con las medidas de throughput en producción.

## Mini-diálogo: en guardia nocturna

> — O sea, que MVCC es «cada elemento lleva una cadena de versiones». ¿Y por eso un capítulo entero?
>
> — Porque esa cadena es la diferencia entre «tu motor lee y escribe» y «tu motor tiene varios lectores y un escritor sin que se pisen». El cap. 27 te daba las palabras — `Anomalia::LecturaSucia`, `Anomalia::ActualizacionPerdida` — pero no podías DEMOSTRAR que estaban prohibidas: el borrow checker era el cerrojo. El cap. 30 te da la maquinaria: ahora un lector toma `&self`, clona su versión, y el escritor toma `&mut self` y APPENDIZA. La anomalía histórica del Vol.II está resuelta.
>
> — Pero entonces, ¿no es un poco... redundante con cerrojos?
>
> — Lo contrario. La MVCC QUITA cerrojos de lectura: la cadena ES el cerrojo. Los lectores no esperan a nadie; el escritor no espera a nadie que esté leyendo. Y ése es exactamente el patrón que el cap. 27 no podía enunciar: el borrow checker admite `&self` y `&mut self` SIMULTÁNEOS mientras las operaciones sean disjuntas, y la MVCC las hace disjuntas por construcción. Con eso, ya puedes construir de noche — y de día, y entre dos procesos cuando llegue la Parte VIII.
>
> — ¿Y el write skew?
>
> — Queda abierto, a propósito. Instantanea prohíbe lectura sucia y lost update, pero no el skew. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — fuera del Vol.II. La honestidad de este capítulo es justamente decir «avanzamos, pero no cerramos»: el closer del aislamiento salta al cap. 40. Es la misma honestidad que el cap. 27 con las anomalías y el cap. 29 con el undo completo: no mentimos sobre lo que falta.

---

*(Próximo capítulo: 31 — La CLI de LiraDB. Aquí el `MvccStore` aprendió a convivir con N lectores; ahora el REPL aprenderá a exponérselos al usuario — qué `ts` toma cada comando, cómo se gestiona una sesión, qué significa «transacción» en una shell.)*