# Capítulo 8 — Diseñar una API antes de persistir (trait `GraphStore`)

> *«No escribas el código de guardar primero. Escribe primero la cara con la que vas a pedirle las cosas. La cara es el contrato; el código de detrás puede cambiar mil veces.»*

## 8.0 La anécdota de la esquina

A finales de los 90, Alistair Cockburn se preguntaba por qué los sistemas empresariales seguían siendo tan difíciles de mantener cuando cada pieza, por separado, parecía razonable. Y dio con algo que hoy parece obvio pero entonces era una herejía: **no son los componentes los que se rompen, son las uniones entre componentes**. El código que MUY dentro sabía "cómo se guarda un dato" estaba enganchado hasta el cuello con el código que "pide el dato", y bastaba cambiar el fondo de la mesa para que saltara toda la mesa.

Su respuesta fue la **arquitectura hexagonal** (la llamó *ports and adapters*, puertos y adaptadores): separemos el contrato —"esto es lo que el negocio necesita pedir"— de los muertos —"esto es de dónde sale". El negocio habla con un **puerto**, una interfaz limpia. Y detrás del puerto puedes enchufar cualquier **adaptador**: memoria, disco, red, un mock para tests. Cuando cambias el adaptador, el negocio no se entera.

Este capítulo es ese momento, en miniatura, dentro de LiraDB. Vamos a diseñar el **puerto** de nuestro grafo — la API `GraphStore` — ANTES de escribir un solo byte en disco. Y será la decisión que haga que los capítulos 9 y 10 (el encoding y la persistencia) sean simples *adaptadores* de algo que ya existe. Porque la historia de la persistencia no empieza cuando aprendes a escribir bytes: empieza cuando decides **qué cara le vas a poner al mundo**.

## 8.1 Objetivo

Al terminar este capítulo habrás diseñado **el contrato de toda la persistencia de LiraDB**: el trait `GraphStore`, la interfaz que dice *qué* se puede hacer con un grafo sin decir ni una palabra sobre *cómo* se guarda.

En concreto:

1. Definirás el trait `GraphStore` con sus once operaciones (put/get/adjacency/count/delete/iter) y su error tipado `StoreError`.
2. Implementarás en memoria un primer adaptador (`MemoryStore`) que cumple el contrato: `Vec<Option<...>>` + listas de adyacencia.
3. Entenderás por qué las firmas son semántica (quién devuelve `Result`, quién `bool`, quién `Option`), por qué `delete_node` **detona una cascada**, y por qué los ids nunca se re-numeran.

La tesis que sostiene al capítulo: **los capítulos 9 y 10 construirán la persistencia como un adaptador de ESTE mismo puerto, sin tocar a ningún cliente del grafo.**

## 8.2 Problema

Del capítulo 7 ya tienes el modelo: `Node { id, labels, props }`, `Edge { id, source, target, label, props }`, con `NodeId = usize` y `EdgeId = usize`. Tienes grafos en la cabeza, algoritmos de recorrido en la manga. Y llega el momento: *"guardemos esto en disco"*.

Pero espera. Antes de "guardar", contesta una pregunta tonta pero tramposa: **¿qué operaciones necesita cualquier aplicación sobre un grafo, sea cual sea el cajón donde viva?** Dirás las obvias: poner un nodo, poner una arista, recuperar un nodo, recuperar una arista, saber sus aristas salientes y entrantes, contarlos, borrarlos, recorrerlos.

Y ahora la trampa, en un escenario real. Imagina que escribes esto "a pelo", en memoria, y dejas que el cliente gestione el detalle:

```rust
// PELIGRO: para borrar un nodo, el cliente debe recordar borrar sus aristas.
fn borrar_nodo(slots: &mut Vec<Option<Node>>, edges: &mut Vec<Option<Edge>>, id: usize) {
    for e in edges {
        if let Some(ed) = e {
            if ed.source == id || ed.target == id {
                *e = None;            // el cliente borra las aristas A MANO
            }
        }
    }
    slots[id] = None;
}
```

¿Ves el problema? No el código en sí — que borra "bien". El problema es que **esta función vive FUERA del concepto de grafo**, y cada programador la escribiría distinta y se le olvidaría la mitad. Dentro de dos capítulos, cuando "guardar" sea ir al disco, nadie va a reescribir solo esta función: reescribirá *todas las llamadas a su alrededor*, y cada una con su criterio sobre qué significa borrar un nodo.

La raíz del problema: **mezclamos el *qué* (las operaciones que un grafo debe soportar) con el *cómo* (memoria, disco, páginas).** Y esa mezcla es la que convierte "cambiar de backend" en "reescribir la aplicación".

## 8.3 Modelo mental

Piensa en una **cafetería con mostrador de pedidos**:

- El **cliente** se planta en el **mostrador** y pide con una cara fija: "un café con leche", "otro igual", "cuántos llevas". Nunca entra a la cocina.
- El **mostrador es el puerto** (`GraphStore`): la lista fija de pedidos que se pueden hacer.
- Detrás hay **varias cocinas — los adaptadores**: la cocina de la RAM (`MemoryStore`), mañana la cocina de disco, pasado la de páginas. Todas sirven *el mismo menú*; solo cambia lo que hay detrás.

```
    ┌──────────────────────────────┐
    │        Tu aplicación         │   (cliente)
    └──────────────┬───────────────┘
                   │  pide con la cara del trait
    ┌──────────────▼───────────────┐
    │         GraphStore            │   (el MOSTRADOR / puerto)
    │   put_node put_edge get_*     │
    │   out_edges in_edges count    │
    │   delete_node delete_edge     │
    │   iter_nodes iter_edges       │
    └──────────────┬───────────────┘
                   │
      ┌────────────┼────────────┐
      ▼            ▼            ▼
  MemoryStore   Disk (cap 10)  Pages (cap 11)  (ADAPTADORES)
```

Dos reglas talladas en el mostrador:

1. **El cliente nunca sabe qué cocina hay detrás.** Solo conoce la cara del mostrador.
2. **Cada cocina cumple el mismo contrato al pie de la letra.** Si el mostrador dice "borra este nodo y sus aristas", TODAS las cocinas lo hacen igual, aunque por dentro hagan cosas completamente distintas.

Y una aclaración de lenguaje para no marearnos: llamamos **puerto** (o *trait*, o *interfaz*) al mostrador — el contrato; y **adaptador** (o *backend*) a cada cocina. Decir "el grafo", sin más, es hablar del puerto. Decir "el `MemoryStore`" es hablar de una cocina concreta.

El momento ¡ajá!: **la API por la que pides los datos es la MISMA tanto si los datos viven en un `Vec` de RAM como si viven en 40.000 páginas de disco.** Lo que cambia es lo que hay detrás del mostrador. Y por eso puedes escribir los capítulos 9 y 10 sin tocar a nadie que use el grafo.

## 8.4 Primera solución

Empecemos por lo más simple que puede funcionar, sin trait, sin cascada, sin "diseño": un montón de funciones sueltas sobre arrays planos.

```rust
// Solución ingenua: el "grafo" es dos Vec y unas funciones a mano.
type NodeId = usize;

struct Stores {
    nodes: Vec<Option<Node>>,
    edges: Vec<Option<Edge>>,
}

// Insertar es fácil (pero ya empezamos a decidir tonterías nosotros):
fn put_node(st: &mut Stores, n: Node) {
    st.nodes[n.id] = Some(n);         // ¿y si ya estaba? ¿lo sobreescribimos callado?
}
```

Y para borrar un nodo, el cliente tiene que acordarse de barrer las aristas (como vimos en 8.2), y luego decidir qué devuelve si no existía... nada, no devuelve nada, ¿cómo sabes si borró algo?

Los tests felices pasan. Poner un nodo: funciona. Leerlo: funciona. Borrarlo con una función que a ti te parece razonable: funciona *en tu cabeza*.

## 8.5 Sus límites

Hasta que llega el caso real, y la solución ingenua se enfrenta a un muro de cuatro aristas:

1. **El borrado es una promesa, no un recuerdo.** Tu `borrar_nodo` barre las aristas *si el que la escribió se acuerda*. En una app real, el borrado lo pide el frontend, lo ejecuta el servicio, lo reaprovecha otra función... cada sitio decide a su manera. Aparecen **aristas colgantes**: un `Edge` cuyo `source` apunta a un nodo que ya no existe. Y nadie lo avisa.
2. **No sabes qué falló.** Pones un nodo y ya estaba. ¿Me avisas? ¿Sobreescribes callado? ¿Y si pido una arista entre dos nodos que no existen? Con estas funciones, el cliente no tiene forma de distinguir "ya está" de "lo puse yo primero" de "me estás pidiendo algo imposible".
3. **No hay cara estable.** Cambias "guardar en `Vec`" por "guardar en disco", y de golpe tienes que tocar *todas* las llamadas, con librerías de E/S, errores de io::Error en la cara. La aplicación entera se contagia de cómo se implementa el almacenamiento.
4. **Sin dirección de recorrido.** ¿Las aristas que ENTRAN en un nodo? Con dos `Vec` a pelo, cada algoritmo las calcula escaneando todo. Los algoritmos del Vol. I (BFS, Dijkstra) necesitan `out_edges` barato; pronto querrás también `in_edges`.

La conclusión duele: **un grafo no es "un `Vec` con funciones sueltas". Es un CONTRATO de operaciones, implementable de muchos modos.** La primera solución no es solo corta; es corta *en la dimensión equivocada*.

## 8.6 Solución evolucionada

La solución es, literalmente, el patrón de Cockburn: extraer el mostrador. En Rust, el mostrador se escribe con la palabra clave **`trait`**.

Un trait es simplemente una **lista de firmas** (qué acepta cada operación y qué devuelve), sin una línea de implementación. Cualquier tipo que quiera ser "un mostrador" declara `impl GraphStore for ElTipo` y llena las firmas. Eso es exactamente un **puerto hexagonal**: la interfaz desacoplada del cómo.

```rust
/// API principal de un grafo de propiedades (en memoria o en disco).
/// El diseño hexagonal (ports & adapters): cualquier backend (memoria,
/// disco, red) implementa este trait. La aplicación que usa el grafo
/// permanece agnóstica al backend.
pub trait GraphStore {
    fn put_node(&mut self, node: Node) -> Result<(), StoreError>;
    fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError>;
    fn get_node(&self, id: NodeId) -> Option<&Node>;
    fn get_edge(&self, id: EdgeId) -> Option<&Edge>;
    fn out_edges(&self, u: NodeId) -> Vec<EdgeId>;
    fn in_edges(&self, u: NodeId) -> Vec<EdgeId>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
    fn delete_node(&mut self, id: NodeId) -> bool;
    fn delete_edge(&mut self, id: EdgeId) -> bool;
    fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_>;
    fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_>;
}
```

Hay tres ideas que no se ven a primera vista, y que son las que venden el contrato.

**Idea 1 — Las firmas son semántica.** Fíjate en cómo cada operación *devuelve*: `put_node -> Result<(), StoreError>`, `get_node -> Option<&Node>`, `delete_node -> bool`. No es capricho, es la máquina escribiéndonos la regla de cuándo usar cada cosa:

- `put_*` devuelve **`Result`** porque insertar puede ser un *fallo tipado* que el llamador debe poder distinguir: `DuplicateNode` (ese id ya está), `UnknownNode` (esa arista no apunta a nadie), `InvalidEdgeEndpoints` (la arista conecta con nodos que no están en el grafo). El cliente hace `match` y responde con criterio.
- `get_*` devuelve **`Option<&Node>`** porque "no existe" aquí es un *hecho normal*, no un error: pedir un nodo que no está te da `None`, y te pega la referencia (sin copiar las props, `&`).
- `delete_*` devuelve **`bool`** porque borrar una clave inexistente es *idempotente*: `true` = "existía, lo borré", `false` = "no existía". No es un fallo; no merece un `Result`.

Tras esta mesa de firmas:

| Operación | Qué dice "no sale bien" | Por qué esa forma |
|---|---|---|
| `put_node` / `put_edge` | `Result<(), StoreError>` | El fallo es TIPADO y el llamador debe responder |
| `get_node` / `get_edge` | `Option<&T>` | "No existe" es un hecho normal; te pego la referencia |
| `delete_node` / `delete_edge` | `bool` | "Ya no estaba" es idempotente; nadie necesita un error |

**Idea 2 — `delete_node` DETONA una cascada.** El contrato dice: *"Elimina un nodo (y todas sus aristas)"*. Fíjate en el paréntesis: **no es un recordatorio para el cliente, es una obligación del mostrador.** Cualquier cocina que quiera ser un `GraphStore` está obligada a que, al borrar un nodo, desaparezcan también sus aristas adyacentes. Así el cliente pide "borra este nodo" y el grafo queda consistente sin que el cliente mueva un dedo. Esa es justo la promesa que la solución ingenua dejaba al azar de la memoria.

**Idea 3 — `&mut self` en escrituras, `&self` en lecturas.** Y aquí llega el regalo escondido. En Rust, toda escritura pide `&mut self` (exclusivo) y toda lectura `&self` (compartido). El compilador, desde el primer día, **te prohíbe tener dos `&mut` vivos a la vez**: no puedes llamar a `put_node` mientras otra referencia mutable recorre el grafo. Eso significa que el trait lleva tatuada la invariante de "**un solo escritor a la vez**".

No es un detalle administrativo: es el germen de la transacción que veremos en el capítulo 27. La base de datos real necesita asegurarse de que dos procesos no escriban a la vez sobre el mismo estado, y aquí —sin hilos, sin locks— el propio préstamo del compilador ya separó el "escribir" (exclusivo) del "leer" (compartido). Lo que en 27 será "único escritor / transacción" empieza como `&mut self`.

El contrato además devuelve vistas de iteración con `Box<dyn Iterator<Item = &T> + '_>`: no materializa y clona todos los nodos; te da un iterador **perezoso** sobre los slots ocupados, cuya vida (el `+ '_`) está atada al borrow del propio store — o sea, no puedes mutar el grafo mientras lo recorres, y eso te lo dice el compilador también.

## 8.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap08_graph_store.rs`. Ya viste el trait; ahora la pieza que mueve el menú: cómo un adaptador de memoria lo cumple.

### El error tipado

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    DuplicateNode(NodeId),
    DuplicateEdge(EdgeId),
    UnknownNode(NodeId),
    UnknownEdge(EdgeId),
    InvalidEdgeEndpoints { source: NodeId, target: NodeId },
}
```

Un `enum` con **datos por variante**. No es un string: `DuplicateNode(0)` guarda *cuál* era el id duplicado. Y al implementar `Display` y `Error`, el cliente puede hacer `match` y, aparte, imprimirlo bonito (`duplicate node id 7`). Es la diferencia entre "quien hace error"(sin información) y "quien describe el fallo" (con la información para responder).

### La estructura interna del adaptador

```rust
pub struct MemoryStore {
    pub nodes: Vec<Option<Node>>,
    pub edges: Vec<Option<Edge>>,
    pub adj_out: Vec<Vec<EdgeId>>,
    pub adj_in: Vec<Vec<EdgeId>>,
}
```

Dos decisiones que están *dentro de la cocina* y que el cliente no necesita conocer — pero que explican mucho:

1. **`Vec<Option<T>>`, no `Vec<T>`** — porque los ids NO se re-numeran. `nodes[i]` es el slot del nodo `id = i`; si borras el 2, el slot se queda `None` (un **tombstone**: la clave `2` queda *enterrada*, no reciclada). Una arista cuyo `source == 2` nunca apuntará a otro nodo por accidente. (Reciclar claves sin un número de generación es lo que un esquema serio evita; eso es el capítulo 3, con generational IDs.)
2. **Dos listas de adyacencia** (`adj_out` y `adj_in`) — porque `out_edges(u)` e `in_edges(u)` deben ser O(grado), no O(todas las aristas). El coste es que hay que mantenerlas sincronizadas al borrar (lo verás en `delete_edge`).

Y el dibujo de los slots, con tombstones:

```
nodes:  [Some<#0>] [ None ] [Some<#2>]
                └──────┬────┘
          edges: [Some<#0>]   ← la arista 0 va de 0 a 2
                 adj_out[0] = [0]     adj_in[2] = [0]
```

### Las operaciones, con su porqué

```rust
fn put_node(&mut self, node: Node) -> Result<(), StoreError> {
    let id = node.id;
    if self.node_exists(id) {
        return Err(StoreError::DuplicateNode(id));   // fallo TIPADO
    }
    self.ensure_node_capacity(id);
    self.nodes[id] = Some(node);
    Ok(())
}
```

`put_node` mira primero si ya existía: si sí, devuelve `Err(DuplicateNode(id))`. Ese es el `Result` en acción: no sobrescribe callado (la solución ingenua) ni inventa una cadena de error, sino que le dice al llamador *exactamente* qué falló y con qué id. De paso, `node_exists` comprueba `is_some()` — derivar, no llevar la cuenta en una columna (patrón que ya viste en slotted pages).

```rust
fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError> {
    // 1º ¿ya hay una arista con este id?
    if id < self.edges.len() && self.edges[id].is_some() {
        return Err(StoreError::DuplicateEdge(id));
    }
    // 2º ¿y sus extremos existen? Si no, es una arista imposible.
    if !self.node_exists(edge.source) || !self.node_exists(edge.target) {
        return Err(StoreError::InvalidEdgeEndpoints { source, target });
    }
    // 3º registra en las dos adyacencias y en edges[id].
    self.adj_out[edge.source].push(id);
    self.adj_in[edge.target].push(id);
    self.edges[id] = Some(edge);
    Ok(())
}
```

Aquí está la clave del `put_edge -> Result`: una arista que conecta con **nodos que no están** es un error que el cliente debe poder distinguir (`InvalidEdgeEndpoints`), porque tiene una manera de corregirlo (insertar antes los nodos). Si devolviera `bool`, no sabría si falló por duplicado o por huérfana.

```rust
fn delete_node(&mut self, id: NodeId) -> bool {
    if !self.node_exists(id) {
        return false;                          // no existía: "no" normal
    }
    // COLECTA las aristas que tocan a `id` (salientes + entrantes)...
    let edges_to_remove: Vec<EdgeId> = self.adj_out.get(id)... 
        .chain(self.adj_in.get(id)...).collect();
    // ... y se las pasa a delete_edge, que limpia TODO (slot + adyacencias).
    for eid in edges_to_remove {
        self.delete_edge(eid);
    }
    self.nodes[id] = None;                     // el tombstone del nodo
    self.adj_out[id].clear();
    self.adj_in[id].clear();
    true
}
```

**Ahí está la cascada.** No es un detalle de implementación: es el contrato cumpliéndose. `delete_node` no borra solo el slot del nodo; junta sus aristas salientes y entrantes y las borra *todas* vía `delete_edge`. Como el cliente pidió "borra el nodo", el grafo entero queda consistente. Cero aristas colgantes, y el cliente no tuvo que recordar nada. (`bool` aquí = "existía, lo borré" — idempotente.)

Y `delete_edge`, el que hay que cuidar por todas partes:

```rust
fn delete_edge(&mut self, id: EdgeId) -> bool {
    if let Some(Some(edge)) = self.edges.get(id) {
        // quita el slot ...
        self.edges[id] = None;
        // ... y SACA el id de las dos listas de adyacencia, o quedarán huérfanos:
        self.adj_out[edge.source].retain(|&e| e != id);
        self.adj_in[edge.target].retain(|&e| e != id);
        true
    } else {
        false
    }
}
```

`retain` recorre la lista de adyacencia y deja solo los ids que NO son el borrado. Si te olvidaras de esto, `out_edges` seguiría devolviendo una arista *ya eliminada* — una mentira silenciosa. Las lecturas:

```rust
fn get_node(&self, id: NodeId) -> Option<&Node> {
    self.nodes.get(id).and_then(|n| n.as_ref())   // Option<&Node>, sin copiar
}
fn out_edges(&self, u: NodeId) -> Vec<EdgeId> {
    self.adj_out.get(u).cloned().unwrap_or_default()
}
```

Recuperar devuelve una referencia (`&Node`) sin clonar las props; la adyacencia devuelve los ids copiados a un `Vec` (barato: son `EdgeId`). Contar deriva del contenido:

```rust
fn node_count(&self) -> usize { self.nodes.iter().filter(|n| n.is_some()).count() }
fn edge_count(&self) -> usize { self.edges.iter().filter(|e| e.is_some()).count() }
```

**Derivar, no llevar en cabeza**: el contador se calcula filtrando los `Some`, como en las slotted pages del cap. 11 se derivaba `free_space`. Nunca una columna de contadores que pueda mentir.

## 8.8 Prueba de fuego

La prueba de fuego no es "los tests pasan", es **"el contrato se cumple aunque el cliente no mueva un dedo"**. Tres tests del workspace lo demuestran:

```rust
// La cascada es CONTRATO, no cortesía del cliente:
let mut s = MemoryStore::new();
s.put_node(Node::new(0, "A")).unwrap();
s.put_node(Node::new(1, "B")).unwrap();
s.put_edge(Edge::new(0, 0, 1, "X")).unwrap();
assert!(s.delete_node(0));
assert_eq!(s.node_count(), 1);   // quedó solo el nodo 1
assert_eq!(s.edge_count(), 0);   // la arista X desapareció SOLA (cascada)
```

Ese es `delete_node_elimina_aristas`. Fíjate en lo que NO hay: el cliente no llamó a `delete_edge`. El mostrador se ocupó. Es exactamente el fallo de la solución ingenua (arista colgante) convertido en un test que ahora pasa.

Y la semántica tipada:

```rust
// El error es TIPADO y matcheable:
s.put_node(Node::new(0, "A")).unwrap();
assert_eq!(
    s.put_node(Node::new(0, "B")),
    Err(StoreError::DuplicateNode(0))   // no: false, no: "ups", sino esto
);
```

`put_node` de un id ya usado devuelve `Err(DuplicateNode(0))` — el llamador puede hacer `match` y saber que el `0` ya estaba. Y `memory_store_basico` cierra: con un nodo `0`, un `1` y la arista `KNOWS`, `node_count()==2`, `edge_count()==1`, `out_edges(0)==[0]` e `in_edges(1)==[0]`.

**Síntoma si este capítulo se te olvidara**: en el capítulo 10 no tendrías dónde conectar el backend de disco; cada programa volvería a decidir a mano cómo borrar un nodo y con qué retorno; y —lo peor— el `&mut self` de escritor único que el capítulo 27 necesita como germen no existiría como contrato estable.

## 8.9 Qué hemos sacrificado

Todo diseño paga un precio. Aquí es especialmente honesto, porque el objetivo del capítulo es justo que el precio quede *dentro de la cocina*:

1. **`&mut self` en toda escritura = sin escritura concurrente.** Para la memoria, un solo mutador a la vez (el compilador lo impone). Es una limitación a propósito: la concurencia real (hilos, transacciones, MVCC) es material de la Parte VI (cap. 27+). Aquí la exclusión es gratuita y correcta, y el germen queda.
2. **Las adyacencias duplican información**: cada arista vive en `edges[..]`, en `adj_out[source]` y en `adj_in[target]`. Eso 3× espacio de ids y la obligación de sincronizar al borrar. Lo pagamos por `out_edges`/`in_edges` baratos (los algoritmos del Vol. I lo exigen).
3. **`usize` como id = sin reciclaje seguro.** Los tombstones (`None`) dejan huecos que nunca se reutilizan. Para pedagogía es perfecto; en producción, los ids generacionales (cap. 3) y los slots reciclables con número de generación lo resolverían.
4. **Devuelve `Vec<EdgeId>` en `out_edges`** (copia el vec de adyacencia). Es barato, pero si en el futuro fueran millones de aristas por nodo querrías un iterador o un slice. El contrato lo permite evolucionar porque la firma es del puerto, no del adaptador.

## 8.10 Cómo lo hace una BBDD real

El patrón "puerto detrás del cual cuelgo distintos motores" es, literalmente, cómo funcionan las bases de datos reales:

- **SQLite** separa el **VDBE** (Virtual Database Engine, el "motor" que ejecuta las sentencias SQL) de la capa de E/S llamada **VFS** (*Virtual File System*), expuesta como `sqlite3_vfs`. El VFS es un puerto: puedes enchufar un adaptador que lea de ficheros nativos, de un dispositivo en memoria (`:memory:`), de un socket o de un sistema remoto — *sin tocar el SQL*. Cuando tú "pides" con `SELECT`, el VDBE emite ops primitivas por un puerto estable, y el VFS decide de dónde saca los bytes. A LiraDB le pasa igual: el `GraphStore` es nuestro VFS.
- **SQLite prepara las consultas** (*prepared statements*): compila el `SELECT` a un programa de bytecode del VDBE, y se ejecuta contra el mismo puerto de almacenamiento. Es la misma idea de desacoplar "qué consulta el usuario" de "dónde vive el dato", en una escala mayor.
- **MySQL** separa su **storage engine** (InnoDB, MyISAM, Memory...) de la capa de consultas: el optimizador pide "dame el registro con clave X" a través de una interfaz de handler, y cada motor la implementa distinto con ITS propios algoritmos (B-Tree, hash, Memoria, TTSI). La capa de SQL no sabe qué motor tiene detrás.
- **El motor (cockburniano)**: Alistair Cockburn, *Hexagonal Architecture* / ports-and-adapters — el puerto desacopla el "contrato del negocio" de los "adaptadores de infraestructura", para que cambiar infraestructura no toque el negocio.

**Retos para el lector — y para nuestra cocina:**

- *Esencial*: en el caso de la estrella del test, ¿por qué al borrar el nodo `0` desaparece la arista `0` sin que el cliente la borre? ¿Qué pasaría si `delete_node` no hiciera esa cascada? Ponlo en palabras de "contrato" no de "código".
- *Intermedio*: mira `delete_edge` y reta a tu compañera: "si quito el `retain` en `adj_out[source]`, ¿qué test del §8.8 falla y con qué síntoma?" Rastrea por qué `out_edges` mentiría.
- *Experto*: ¿qué le pasaría al diseño si quisiéramos leer el grafo desde dos hilos a la vez? ¿Qué parte del trait (`&self` vs `&mut self`) lo hace seguro, y qué necesitaríamos para permitir escritura concurrente? (Anota tu respuesta; el cap. 27 la contrastará.)

## 8.11 Lo que te llevas

- La **API es el contrato**, la implementación es la cocina: nombra el puerto (`GraphStore`) y deja que los backends (memoria, disco, páginas) sean adaptadores.
- **Las firmas son semántica**: `put_* -> Result` (fallo tipado), `get_* -> Option<&T>` (ausencia normal), `delete_* -> bool` (idempotente). No uses `Result` para todo.
- **`delete_node` detona una cascada**: borrar un nodo borra sus aristas adyacentes *por el mostrador*, no por el recuerdo del cliente.
- **Los ids nunca se re-numeran**: `Vec<Option<T>>` con tombstones, la clave queda estabilísimo o enterrada (`None`).
- **`&mut self` / `&self`**: el compilador ya separó escritor (exclusivo) de lector (compartido) — el germen del único escritor del cap. 27.
- **Derivar, no llevar en cabeza**: `node_count`/`edge_count` se calculan de los `Some`, no se mantienen en una columna.

## 8.12 Ojo, cuidado con…

- **Escribir primero el almacenamiento y "sacar el trait después".** El trait saldría moldeado a la memoria y no serviría para disco. El capítulo entero es la lección contraria: contrato ANTES de cocina.
- **`Result` para todo.** Si `delete_node` devolviera `Result`, cada borrado exigiría `?` y `match` para un "no" que no es error. Regla: ¿es un hecho normal (→`bool`/`Option`) o un fallo a distinguir (→`Result`)?
- **Re-numerar o compactar los ids al borrar.** `Vec<T>` con shift rompe los `source`/`target` de las aristas: colisión silenciosa. Regla: clave estable; borrar es dejar `None`.
- **Olvidar mantener `adj_out`/`adj_in` sincronizadas en `delete_edge`.** Solo tocar `edges` deja que `out_edges` siga contando aristas muertas. Detección: `out_edges` devuelve un id que `get_edge` ya no encuentra.
- **Hacer que el cliente confíe en la cocina.** Si tu código cliente usa `MemoryStore.nodes` directamente, has perdido el desacoplamiento. El contrato gana cuando el cliente solo conoce el puerto.

## 8.13 Pin de batalla

> *«La persistencia no empieza cuando escribes bytes. Empieza cuando decides la cara con la que le vas a pedir las cosas al mundo — porque esa cara no va a cambiar, aunque detrás cambie mil veces.»*

## 8.14 Si solo lees 30 segundos

El soporte de un grafo es un **contrato** (el trait `GraphStore`) y no un montón de funciones sueltas. El contrato dice *qué* se hace (once operaciones), no *cómo*: `put_node`/`put_edge` devuelven `Result` con error tipado si hay duplicado o endpoints inválidos; `get_*` devuelve `Option<&Node>`; `delete_*` devuelve `bool` y **`delete_node` detona la cascada** que borra las aristas adyacentes. Los ids no se re-numeran: se guardan en `Vec<Option<T>>` con tombstones. Las escrituras piden `&mut self` y las lecturas `&self` — el compilador ya te da el único escritor. Ese puerto es el que los capítulos 9 y 10 implementarán como adaptador de disco, sin tocar a los clientes.

## 8.15 Una historia pequeña

Cuando empezamos LiraDB, antes de este capítulo, "guardar un grafo" significaba abrir un fichero y escribir lo que se nos ocurriera en el momento. El código de borrado vivía en la función que llamaba el frontend, y cada función vecina tenía su propio criterio sobre qué significaba borrar un nodo. La primera vez que borramos "Ana", la arista `Ana–CONOCE–Bob` siguió allí, apuntando a un nodo que ya no existía, y nadie se enteró hasta que un algoritmo de caminos se quedó en bucle buscando a Ana para siempre. El problema no era un `if` olvidado: era que **nunca habíamos decidido la cara con la que le íbamos a pedir las cosas al grafo**. El trait llegó como una percha: pon todo aquí debajo, y el cliente solo pregunta por esta cara. Desde entonces, borrar un nodo significa una cosa, para todos, siempre.

## Ejercicios resueltos

**1. ¿Por qué `delete_node` devuelve `bool` en lugar de `Result<(), StoreError>`?**

Porque borrar una clave que no existe es un **hecho normal** (idempotencia), no un fallo a tipar. `true` = "existía y lo borré"; `false` = "no existía, nada que hacer". Un `Result` obligaría a `?`/`match` para distinguir casos que al llamador no le importan. En cambio `put_node` SÍ usa `Result`, porque ahí el fallo es tipado y el cliente debe saber si fue por `DuplicateNode`, `UnknownNode` o `InvalidEdgeEndpoints`. La regla: ¿es un "no" normal o un error que debo distinguir? delete→bool, get→Option, put→Result.

**2. ¿Qué es exactamente la "cascada" de `delete_node` y qué garantiza?** 

Cuando llamas a `delete_node(0)`, el método **no solo** pone `nodes[0] = None`. Reúne primero todas las aristas que tocan al nodo — las de `adj_out[0]` (salientes) y las de `adj_in[0]` (entrantes)— y se las pasa a `delete_edge`, que elimina cada una de `edges`, `adj_out` y `adj_in`. El contrato exige que **borrar un nodo borra sus aristas adyacentes**. Garantiza que no quede ninguna arista colgante (una arista cuyo `source` o `target` apunte a un nodo borrado). El cliente pide "borra este nodo" y el grafo entero queda consistente: eso lo demuestra `delete_node_elimina_aristas` (el `edge_count()` cae a 0 sin que el cliente borre la arista a mano).

## Ejercicios propuestos

**Esencial.** Sobre la estrella `0→{1,2,3}` (nodos 0,1,2,3; aristas `0→1`, `0→2`, `0→3`), escribe un test que: (1) prediga —antes de ejecutar— `out_edges(0)`, `in_edges(1)`, `node_count`, `edge_count`; (2) llame `delete_node(0)` y verifique que `edge_count()` queda en `0` y `get_node(0) == None`. Explica, en palabras de contrato (no de código), por qué la arista `0→1` desapareció sin que tú la borraras.

**Intermedio.** (Spacing al cap. 7.) Usando `Node::with_prop` y `Value` del modelo del cap. 7, construye dos nodos `Person` con props (`nombre`, `edad`) y una arista `KNOWS`, e insértalos **a través del store** (no a mano). Luego: (1) ¿dónde viven esas props, en el `Node` o en el trait, y por qué?; (2) borra el nodo origen y razona qué pasaría con la arista `KNOWS` en un `iter_edges` **si el store no detonara la cascada**; (3) ¿tu programita depende de `MemoryStore` en concreto o del `dyn GraphStore`? Modifica el código para que no mencione `MemoryStore` en la lógica del cliente.

**Experto.** Implementa desde cero un segundo adaptador `HashMapStore` (sin `adj_out`/`adj_in`: haz que `out_edges`/`in_edges` escaneen los `Edge` con `retain`) que cumpla `GraphStore`, y escribe UN test genérico —contra `&mut dyn GraphStore`— que corra la misma batería (poblar, recorrer, cascada `delete_node`, `Duplicate`) contra ambos `MemoryStore` y `HashMapStore`. Hints: (1) sin adyacencias, la cascada de `delete_node` borra barriendo todos los `Edge` y quedándose con los que no tocan el nodo; (2) la firma `get_node(&self, id) -> Option<&Node>`—¿con qué método del `HashMap` devuelve referencia mutable?; (3) si el mismo test genérico pasa para los dos backends, has demostrado que tu lógica no sabe qué cocina hay detrás.

## Para profundizar

- **Alistair Cockburn**, *Hexagonal Architecture* (2005) y su patrón de *ports and adapters* — la formulación original del puerto que desacopla el "qué" del "cómo".
- **Código fuente de SQLite**, `os.h` / `sqlite3_vfs` (Virtual File System) y los *prepared statements* del VDBE (`vdbbe.c`): cómo un motor separa el "que consulta" del "dónde vive el dato".
- **Código fuente de MySQL**: la interfaz de *handler* de storage engines (InnoDB vs MyISAM vs Memory), y cómo la capa de SQL no sabe qué motor tiene detrás.
- **"Designing Data-Intensive Applications" (Martin Kleppmann)**, cap. 3 — por qué separar "modelo lógico" (tu API) de "modelo físico" (tu almacenamiento) es la disciplina que mantiene viva una BD.

## Mini-diálogo: en guardia nocturna

> — O sea, que todo este capítulo es... declarar un machote de funciones y que nadie entre en la cocina.
>
> — Casi. Declarar el machote es la parte fácil; lo difícil es que ese machote sea *semántica*: por qué esto devuelve `bool`, esto `Option` y aquello `Result`.
>
> — ¿Y por qué tanto lío antes de escribir el primer byte?
>
> — Porque "escribir el primer byte" es la parte que casi no cambiará. Lo que cambia es lo de detrás: disco, páginas, otro motor. Si el mostrador está bien puesto, cambiarlo de cocina no te obliga a reescribir tu aplicación ni a volver a decidir qué significa borrar un nodo. El contrato es lo que dura; el código de guardar, es del día a día.
>
> — Entonces... ¿ya puedo escribir bytes?
>
> — Puedes. El mostrador está en una mesa de firmas que se sostiene sola, y mañana —en el capítulo 9— vas a descubrir que los bytes también tienen su orden y su trampa.

---

*(Próximo capítulo: 9 — Del objeto al byte. Aquí definimos el contrato con `Node` y `Edge` vivos; ahora veremos cómo se convierten en un `Vec<u8>` sin ambigüedad — encoding, endianness y versionado — para que el mismo puerto, con otro adaptador, llegue al disco.)*
