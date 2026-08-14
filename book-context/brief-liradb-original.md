> From: https://chatgpt.com/c/6a6b0d6a-02b8-83ed-b707-c31181de0745

# you asked

message time: 2026-07-30 10:38:32

hay algun libro tecnico o videotutoriales que expliquen de manera didactica como crear una base de datos de grafos y sus algoritmos?

---

# chatgpt response

Voy a separar recursos para **aprender algoritmos de grafos** de los que realmente enseñan a **construir el motor de una base de datos de grafos**: almacenamiento, índices, consultas y ejecución.

Sí, aunque hay una distinción importante:

1. **Aprender teoría y algoritmos de grafos**: BFS, DFS, Dijkstra, PageRank, componentes, comunidades…
2. **Construir una base de datos de grafos**: persistencia, páginas de disco, índices, transacciones, almacenamiento de adyacencias, lenguaje de consultas, optimizador y motor de ejecución.

No he encontrado un único libro realmente didáctico que construya de principio a fin una **Neo4j en miniatura**. La mejor opción es combinar varios recursos.

## La ruta que más te recomiendo

### 1. Algoritmos de grafos desde los fundamentos

**Algorithms, 4th Edition — Robert Sedgewick y Kevin Wayne**

Es probablemente el mejor punto de partida didáctico. El capítulo de grafos incluye:

- Representación mediante listas de adyacencia.
- BFS y DFS.
- Componentes conexos.
- Ordenación topológica.
- Componentes fuertemente conexos.
- Prim y Kruskal.
- Dijkstra, Bellman-Ford y Floyd-Warshall.
- Código completo en Java y ejercicios.

La web oficial permite consultar explicaciones, diapositivas y código, aunque el libro completo es comercial. ([Algs4](https://algs4.cs.princeton.edu/40graphs/))

**Valoración:** excelente para implementar después los algoritmos en Rust.

---

### 2. Vídeos: MIT 6.006 Introduction to Algorithms

El curso gratuito del MIT tiene clases completas sobre:

- Representación de grafos.
- BFS y DFS.
- Caminos mínimos ponderados.
- Bellman-Ford.
- Dijkstra.
- Análisis de complejidad.

La edición de 2020 incluye vídeos, apuntes y problemas. ([MIT OpenCourseWare](https://ocw.mit.edu/courses/6-006-introduction-to-algorithms-spring-2020/resources/lecture-videos/?utm_source=chatgpt.com))

Para aprender de manera visual y después implementarlo, probablemente sea el mejor videotutorial académico disponible.

---

### 3. Algoritmos aplicados a bases de datos de grafos

**Graph Algorithms — Mark Needham y Amy Hodler**

Está disponible también como **Algoritmos gráficos** en español. Explica búsqueda de caminos, centralidad, detección de comunidades, similitud y predicción de enlaces, con ejemplos prácticos en Neo4j y Apache Spark. ([O'Reilly Media](https://www.oreilly.com/library/view/graph-algorithms/9781492047674/preface01.html?utm_source=chatgpt.com))

No enseña a construir el motor de almacenamiento, pero sí ayuda a entender qué debe soportar una base de datos de grafos y cómo se ejecutan sus algoritmos.

---

## Para aprender a construir el motor de base de datos

### 4. Build Your Own Database From Scratch in Go

Aunque utiliza Go y construye una base de datos relacional pequeña, es el recurso más cercano a un tutorial paso a paso sobre cómo crear una base de datos:

- Ficheros y páginas.
- Árbol B+.
- Recuperación ante fallos.
- Almacenamiento clave-valor.
- Gestión del espacio libre.
- Índices secundarios.
- Transacciones.
- Control de concurrencia.
- Parser y lenguaje de consultas.

La primera parte puede leerse gratuitamente en la web. ([build-your-own.org](https://build-your-own.org/database/))

La idea sería sustituir la capa relacional por:

```text
Motor de páginas y persistencia
        ↓
Almacén clave-valor
        ↓
Tablas de nodos y relaciones
        ↓
Índices de adyacencia
        ↓
Operadores de recorrido
        ↓
Lenguaje de patrones
```

Es el libro que elegiría para aprender a implementar la parte de **base de datos real**, aunque luego trasladaras el código de Go a Rust.

---

### 5. CMU 15-445/645 — Database Systems

Es un curso universitario orientado a implementar internals de bases de datos. Estudia:

- Organización de almacenamiento.
- Heap files y páginas.
- Buffer pool.
- Árboles B+ y tablas hash.
- Ejecución de consultas.
- Optimización.
- ACID y concurrencia.
- WAL, checkpoints y recuperación.
- Arquitecturas paralelas y distribuidas.

El curso utiliza el proyecto educativo **BusTub**, donde se implementan progresivamente componentes de un DBMS. ([Carnegie Mellon Database Group](https://db.cs.cmu.edu/courses/))

Aunque sea relacional, el 70–80 % de la infraestructura es reutilizable para una base de datos de grafos:

```text
Buffer pool
WAL
MVCC
catálogo
páginas
serialización
transacciones
planificador
operadores
estadísticas
```

Después cambiarías principalmente el modelo lógico, los índices de adyacencia y los operadores de consulta.

---

### 6. Database Internals — Alex Petrov

Es menos tutorial y más libro de arquitectura. Explica cómo funcionan los motores modernos:

- Árboles B y B+.
- LSM trees.
- Formatos de fichero.
- Páginas.
- Buffering.
- WAL.
- Recuperación.
- Replicación.
- Consenso.
- Sistemas distribuidos.

El propio libro está orientado a comprender motores de almacenamiento y arquitectura interna de bases de datos. ([O'Reilly Media](https://www.oreilly.com/library/view/database-internals/9781492040330/?utm_source=chatgpt.com))

Lo usaría como libro de consulta mientras implementas, no como primer recurso.

---

## La parte específicamente gráfica: Kùzu

Para entender cómo una base de datos de grafos moderna representa y consulta grafos, el mejor material técnico que he encontrado es **Kùzu**.

Su arquitectura incluye:

- Property Graph.
- Cypher.
- Almacenamiento columnar en disco.
- Listas de adyacencia basadas en CSR comprimido.
- Índices de joins.
- Ejecución vectorizada.
- Ejecución factorizada.
- Joins binarios y worst-case optimal.
- Paralelismo multinúcleo.
- Transacciones ACID serializables. ([kuzudb.github.io](https://kuzudb.github.io/docs))

El artículo técnico de Kùzu explica específicamente los problemas que diferencian a un GDBMS de una base relacional: joins muchos-a-muchos, joins cíclicos, recorridos recursivos y ejecución factorizada. ([VLDB](https://vldb.org/cidrdb/2023/kuzu-graph-database-management-system.html))

El profesor Semih Salihoğlu mantiene publicaciones, charlas y referencias sobre Kùzu y GraphflowDB, un GDBMS construido desde cero. ([Escuela Cheriton](https://cs.uwaterloo.ca/~ssalihog/))

Eso sí: Kùzu fue adquirido por Apple y el repositorio público quedó sin actividad posterior a octubre de 2025, por lo que lo consideraría principalmente una **fuente de estudio arquitectónico**, no necesariamente la base de un proyecto nuevo. ([Universidad de Waterloo](https://uwaterloo.ca/computer-science/news/waterloo-based-graph-database-start-up-kuzu-acquired-apple?utm_source=chatgpt.com))

## Para practicarlo en Rust

Dado que trabajas con Rust, estudiaría estos proyectos en este orden:

### `petgraph`

No es una base de datos, sino una biblioteca de estructuras y algoritmos. Tiene listas de adyacencia, CSR, BFS, DFS, Dijkstra, componentes, árboles de expansión e interfaces genéricas para implementar algoritmos nuevos. ([Docs.rs](https://docs.rs/petgraph/?utm_source=chatgpt.com))

Es ideal para aprender cómo separar:

```rust
trait GraphStorage {}
trait IntoNeighbors {}
trait NodeIndexable {}
trait EdgeRef {}
trait GraphAlgorithm {}
```

Pero para tu propio motor convendría implementar inicialmente los algoritmos sin apoyarte en `petgraph`, y usarlo después para comparar resultados.

### `IndraDB`

Es una base de datos de grafos escrita en Rust, con biblioteca embebible y servidor gRPC. Su código es bastante más manejable que el de SurrealDB, por lo que puede servir para estudiar una implementación completa pero relativamente contenida. ([GitHub](https://github.com/indradb/indradb?utm_source=chatgpt.com))

### `Oxigraph`

Es una base de datos RDF/SPARQL escrita en Rust y construida sobre RocksDB. Resulta especialmente interesante para estudiar:

- Codificación de triples.
- Índices SPO, POS, OSP y sus permutaciones.
- Parser SPARQL.
- Evaluación de patrones.
- Persistencia sobre un KV store. ([GitHub](https://github.com/oxigraph/oxigraph?utm_source=chatgpt.com))

### `CozoDB`

También está escrito en Rust y utiliza Datalog como lenguaje de consulta, con especial atención a datos y algoritmos de grafos. Es interesante para aprender evaluación recursiva y cómo integrar algoritmos dentro del lenguaje de consultas. ([GitHub](https://github.com/cozodb/cozo?utm_source=chatgpt.com))

## Videotutoriales concretos

Mi selección sería:

1. **MIT 6.006** para algoritmos.
2. **CMU 15-445** para construir un DBMS.
3. **Charlas de Semih Salihoğlu sobre GraphflowDB/Kùzu** para arquitectura específica de grafos. Su página oficial enlaza directamente varias charlas. ([Escuela Cheriton](https://cs.uwaterloo.ca/~ssalihog/))
4. **Neo4j GraphAcademy** para comprobar cómo se utilizan los algoritmos desde una base real. Su curso de Graph Data Science es gratuito, dura alrededor de tres horas y cubre proyecciones, configuración y ejecución de algoritmos. ([graphacademy.neo4j.com](https://graphacademy.neo4j.com/courses/gds-fundamentals))

GraphAcademy enseña a **usar** una base de grafos, no a implementarla.

## Itinerario práctico que seguiría

```text
1. Implementar Graph<Node, Edge> en memoria
2. BFS, DFS y componentes conexos
3. Dijkstra, A* y Bellman-Ford
4. Representación CSR
5. Identificadores estables para nodos y relaciones
6. Persistencia append-only
7. Páginas y buffer pool
8. Índices por ID, etiqueta y propiedad
9. Índices de adyacencia outgoing/incoming
10. WAL y recuperación
11. Transacciones MVCC
12. Lenguaje mínimo MATCH-WHERE-RETURN
13. Parser → AST → plan lógico
14. Operadores Expand, Filter y Project
15. Optimizador basado en estadísticas
16. PageRank, Louvain y centralidades
```

Para un proyecto didáctico en Rust, mi combinación principal sería:

> **Sedgewick + MIT 6.006 + Build Your Own Database + CMU 15-445 + artículos de Kùzu.**

Con esos recursos sí tendrías material suficiente para construir una base de datos de grafos embebida, pequeña pero real, en lugar de limitarte a una estructura `Graph` en memoria.

---

# you asked

message time: 2026-07-30 10:49:14

ok y si quisiera crear un libro tecnico crea un bbdd de grafo desde cero, con ejemplo de un miniproyecto lite, para poner todos los conocimientos mas destacables en valor de todo lo que me has representado, dime cual seria el guion del libro, estilo, para novatos de cero a experto etc

---

# chatgpt response

Sí. La mejor forma de plantearlo sería como un libro **orientado a proyecto**, donde el lector construye gradualmente una base de datos de grafos embebida y funcional en Rust.

El libro no debería intentar crear un competidor de Neo4j. El objetivo sería construir una base pequeña pero real que permita comprender las decisiones internas de un sistema de grafos:

- Almacenamiento de nodos y relaciones.
- Recorridos y algoritmos.
- Persistencia.
- Índices.
- Lenguaje de consultas.
- Ejecución de consultas.
- Transacciones básicas.
- Recuperación ante fallos.
- Medición y optimización.

# Propuesta de título

## Título principal

**Construye una base de datos de grafos desde cero**

### Subtítulo

**De los algoritmos fundamentales a un motor persistente de consultas en Rust**

El miniproyecto podría llamarse:

```text
LiraDB
```

Nombre corto, fácil de recordar y relacionado indirectamente con conexiones y cuerdas.

Otros nombres posibles:

- MiniGraphDB
- Graphite
- NodusDB
- Arista
- GrafoLite

Yo utilizaría **LiraDB** para el proyecto y reservaría “lite” para describir su alcance:

> LiraDB es una base de datos de grafos embebida, educativa y ligera, escrita en Rust.

---

# Filosofía del libro

El libro debería seguir esta progresión:

```text
Entender → Implementar → Probar → Medir → Mejorar
```

Cada concepto se introduce solo cuando el proyecto lo necesita.

Por ejemplo, no se explicaría un buffer pool durante cien páginas de teoría. Primero se mostraría el problema:

```text
Cada consulta vuelve a leer las mismas páginas del disco.
```

Después se mediría, se explicaría el concepto y se construiría una solución.

La estructura pedagógica de cada capítulo sería:

1. Problema que queremos resolver.
2. Modelo mental sencillo.
3. Fundamento teórico.
4. Diseño de la solución.
5. Implementación en Rust.
6. Pruebas.
7. Benchmark.
8. Decisiones y alternativas.
9. Mejoras propuestas.
10. Estado final de LiraDB.

Esto convierte el libro en algo más cercano a una experiencia de ingeniería que a un manual académico.

# Público objetivo

El lector puede empezar sabiendo programar de forma básica, pero sin conocimientos previos sobre bases de datos o teoría de grafos.

Se asumiría:

- Variables, funciones y estructuras de datos básicas.
- Uso elemental de Git.
- Conocimientos básicos de terminal.
- Capacidad para leer código.

No se asumiría:

- Teoría de grafos.
- Algoritmos avanzados.
- Rust avanzado.
- Internals de bases de datos.
- Diseño de lenguajes.
- Optimización de consultas.
- Sistemas distribuidos.

El libro debería incluir pequeñas introducciones de Rust cuando se necesiten:

- `struct` y `enum`.
- Propiedad y préstamos.
- Traits.
- Iteradores.
- Gestión de errores.
- Serialización.
- Concurrencia.
- Organización por crates.

No debería convertirse, sin embargo, en un libro general de Rust.

# Resultado final del proyecto

Al terminar el libro, LiraDB permitiría ejecutar algo parecido a:

```text
CREATE NODE Person {
    name: "Ana",
    age: 34
}

CREATE NODE Person {
    name: "Luis",
    age: 31
}

CREATE EDGE KNOWS
FROM node:1
TO node:2
{
    since: 2022
}
```

Y consultar:

```text
MATCH (person:Person)-[:KNOWS]->(friend:Person)
WHERE person.name = "Ana"
RETURN friend.name
```

También dispondría de una API Rust:

```rust
let ana = db.create_node(
    "Person",
    properties! {
        "name" => "Ana",
        "age" => 34,
    },
)?;

let friends = db
    .match_node("Person")
    .property("name", "Ana")
    .outgoing("KNOWS")
    .execute()?;
```

Y algoritmos:

```rust
let path = db.shortest_path(ana, destination)?;
let components = db.connected_components()?;
let ranking = db.page_rank(PageRankOptions::default())?;
```

## Funcionalidades finales

LiraDB Lite incluiría:

- Nodos y relaciones dirigidas.
- Etiquetas y tipos de relación.
- Propiedades primitivas.
- IDs estables.
- Persistencia en disco.
- Representación mediante listas de adyacencia.
- Índices por ID, etiqueta y propiedad.
- BFS y DFS.
- Caminos mínimos.
- Componentes conexos.
- Ordenación topológica.
- PageRank básico.
- Parser de un lenguaje de consultas pequeño.
- Plan lógico y plan físico.
- Operadores de ejecución.
- Transacciones de un único escritor.
- Write-ahead log.
- Recuperación básica.
- CLI interactiva.
- Herramientas de inspección.
- Benchmarks y trazas.

Quedarían fuera de la versión lite:

- Clústeres distribuidos.
- Replicación.
- Consenso.
- Sharding.
- Compatibilidad completa con Cypher.
- Optimización cost-based sofisticada.
- Ejecución distribuida.
- Alta disponibilidad.

Esos temas aparecerían al final como evolución posible.

# Guion completo del libro

## Prólogo — Vamos a construir una base de datos

El prólogo presenta el reto:

> ¿Qué ocurre realmente cuando preguntamos quiénes son los amigos de los amigos de una persona?

Se construye una primera representación extremadamente simple:

```rust
struct Node {
    id: u64,
}

struct Edge {
    from: u64,
    to: u64,
}
```

Al final del prólogo el lector ya puede crear tres nodos y recorrer sus conexiones.

La intención es eliminar desde el principio la sensación de que construir una base de datos es inaccesible.

---

# Parte I — Pensar en grafos

## Capítulo 1. Qué es realmente un grafo

Contenido:

- Nodos, aristas y propiedades.
- Grafos dirigidos y no dirigidos.
- Grafos ponderados.
- Caminos y ciclos.
- Grado de un nodo.
- Subgrafos.
- Grafos dispersos y densos.
- Property Graph frente a RDF.

Ejemplo conductor:

```text
Personas → relaciones → empresas → proyectos
```

Hito:

> Representar manualmente un pequeño grafo en Rust.

---

## Capítulo 2. Cómo representar un grafo en memoria

Comparación entre:

- Matriz de adyacencia.
- Lista de adyacencia.
- Lista de aristas.
- CSR.
- Estructuras híbridas.

Se analizan:

- Memoria.
- Coste de inserción.
- Coste de recorrer vecinos.
- Eliminación de nodos.
- Localidad de caché.

Hito:

```rust
pub struct MemoryGraph {
    nodes: Vec<Node>,
    outgoing: Vec<Vec<EdgeId>>,
    incoming: Vec<Vec<EdgeId>>,
}
```

---

## Capítulo 3. Identidad, referencias y datos estables

Problemas:

- ¿Qué sucede cuando eliminamos un nodo de un `Vec`?
- ¿Podemos utilizar posiciones como identificadores?
- ¿Cómo detectamos referencias obsoletas?

Contenido:

- IDs lógicos y posiciones físicas.
- Generational indexes.
- Free lists.
- Tombstones.
- Integridad referencial.

Hito:

```rust
pub struct NodeId {
    slot: u32,
    generation: u32,
}
```

---

## Capítulo 4. El primer recorrido: búsqueda en anchura

Contenido:

- Cola.
- Conjunto de visitados.
- Complejidad `O(V + E)`.
- Caminos mínimos sin peso.
- Reconstrucción del camino.

Hito:

```rust
graph.shortest_unweighted_path(source, target)
```

Se visualiza paso a paso el contenido de la cola.

---

## Capítulo 5. Profundidad, ciclos y componentes

Contenido:

- DFS iterativo y recursivo.
- Detección de ciclos.
- Componentes conexos.
- Ordenación topológica.
- Componentes fuertemente conexos.

Hito:

```rust
graph.detect_cycles()
graph.connected_components()
graph.topological_sort()
```

Al finalizar esta parte ya existe una biblioteca de grafos en memoria.

---

# Parte II — De estructura de datos a base de datos

## Capítulo 6. Qué convierte un grafo en una base de datos

Se explican las diferencias entre:

```text
Estructura en memoria
Biblioteca de grafos
Motor de almacenamiento
Base de datos
Sistema gestor de bases de datos
```

Se introducen:

- Persistencia.
- Consultas.
- Integridad.
- Transacciones.
- Concurrencia.
- Recuperación.
- Observabilidad.

Hito:

> Definir formalmente el alcance de LiraDB Lite.

---

## Capítulo 7. El modelo de datos de LiraDB

Diseño del modelo Property Graph:

```rust
pub struct Node {
    id: NodeId,
    labels: Vec<LabelId>,
    properties: PropertyMap,
}

pub struct Relationship {
    id: RelationshipId,
    source: NodeId,
    target: NodeId,
    relationship_type: RelationshipTypeId,
    properties: PropertyMap,
}
```

Se introducen tipos de propiedad:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}
```

También:

- Catálogo de etiquetas.
- Tipos de relación.
- Claves de propiedades.
- Validación de valores.
- Evolución del esquema.

---

## Capítulo 8. Diseñar una API antes de persistir

Se construye primero una API limpia:

```rust
pub trait GraphStore {
    fn create_node(&mut self, node: NewNode) -> Result<NodeId>;
    fn get_node(&self, id: NodeId) -> Result<Option<Node>>;
    fn delete_node(&mut self, id: NodeId) -> Result<()>;

    fn create_relationship(
        &mut self,
        relationship: NewRelationship,
    ) -> Result<RelationshipId>;

    fn outgoing(
        &self,
        node: NodeId,
        relationship_type: Option<RelationshipTypeId>,
    ) -> Result<Vec<Relationship>>;
}
```

Se explican:

- Puertos y adaptadores.
- Separación entre modelo lógico y almacenamiento.
- Errores de dominio.
- Invariantes.
- Tests de contrato.

Hito:

> Backend completamente en memoria detrás de una interfaz estable.

---

## Capítulo 9. Del objeto al byte

Contenido:

- Representación binaria.
- Endianness.
- Longitudes variables.
- Strings.
- Checksums.
- Versiones de formato.
- Compatibilidad hacia atrás.

Ejemplo de registro:

```text
| version | record_type | id | payload_length | payload | checksum |
```

Hito:

```rust
encode_node(&node) -> Vec<u8>
decode_node(bytes) -> Result<Node>
```

---

## Capítulo 10. Persistencia append-only

Se empieza con la solución más sencilla:

```text
Solo añadimos registros al final del fichero.
```

Contenido:

- Log estructurado.
- Registros de creación, actualización y borrado.
- Reconstrucción al arrancar.
- Tombstones.
- Compactación.

Hito:

> Cerrar el proceso, volver a abrirlo y recuperar el grafo.

Este capítulo resulta muy gratificante porque LiraDB se convierte por primera vez en una base persistente.

---

# Parte III — Construir el motor de almacenamiento

## Capítulo 11. Páginas, bloques y organización del fichero

Contenido:

- Por qué no conviene leer registros arbitrarios.
- Tamaño de página.
- Cabecera de página.
- Slotted pages.
- Fragmentación.
- Páginas libres.
- Metapágina.

Diseño:

```text
database.lira
├── metadata pages
├── node pages
├── relationship pages
├── property pages
└── index pages
```

Hito:

```rust
PageId
Page
PageHeader
SlotId
```

---

## Capítulo 12. El gestor de páginas

Responsabilidades:

- Crear páginas.
- Leer páginas.
- Escribir páginas.
- Asignar espacio.
- Liberar páginas.
- Mantener una free list.

Hito:

```rust
pub trait Pager {
    fn allocate(&mut self) -> Result<PageId>;
    fn read(&mut self, page: PageId) -> Result<Page>;
    fn write(&mut self, page: PageId, data: &Page) -> Result<()>;
}
```

---

## Capítulo 13. El buffer pool

El problema aparece mediante benchmarks:

```text
Consultar repetidamente los mismos nodos genera demasiadas lecturas.
```

Contenido:

- Caché de páginas.
- Frames.
- Pinning.
- Dirty pages.
- Eviction.
- LRU, Clock y variantes.
- Flush.

Hito:

> Buffer pool configurable con algoritmo Clock.

Se añaden métricas:

```text
page_reads
page_writes
buffer_hits
buffer_misses
evictions
```

---

## Capítulo 14. Cómo almacenar adyacencias

Este es uno de los capítulos centrales del libro.

Comparación:

- Relaciones enlazadas.
- Listas de adyacencia.
- CSR.
- Adyacencias segmentadas.
- Separación outgoing/incoming.
- Agrupación por tipo de relación.

Modelo inicial:

```text
NodeRecord
 ├── first_outgoing_relationship
 └── first_incoming_relationship
```

Evolución hacia:

```text
AdjacencySegment
 ├── node_id
 ├── relationship_type
 ├── direction
 └── relationship_ids[]
```

Hito:

> Recorrer vecinos sin escanear todas las relaciones.

---

## Capítulo 15. Índices para encontrar datos

Contenido:

- Índice primario por ID.
- Índice por etiqueta.
- Índice por propiedad.
- Hash index.
- B+ tree.
- Índices compuestos.
- Selectividad.

Implementación propuesta:

1. Hash index persistente sencillo para IDs.
2. B+ tree didáctico para propiedades ordenables.

Hito:

```text
Person.name → NodeId[]
```

Consulta:

```text
Buscar personas cuyo nombre sea "Ana"
```

sin recorrer todos los nodos.

---

## Capítulo 16. Compactación y mantenimiento

Contenido:

- Espacio muerto.
- Tombstones.
- Reescritura.
- Compactación online y offline.
- Vacuum.
- Estadísticas de almacenamiento.
- Verificación de integridad.

Hito CLI:

```bash
liradb inspect
liradb check
liradb compact
```

---

# Parte IV — Consultar el grafo

## Capítulo 17. Diseñar un lenguaje pequeño

Se evita implementar Cypher completo.

Lenguaje inicial:

```text
MATCH (p:Person)
WHERE p.name = "Ana"
RETURN p
```

Después:

```text
MATCH (p:Person)-[:KNOWS]->(friend:Person)
RETURN friend.name
```

Contenido:

- Tokens.
- Gramática.
- AST.
- Errores comprensibles.
- Posición en el código fuente.

Hito:

```rust
pub enum AstNode {
    Match(MatchClause),
    Where(Expression),
    Return(ReturnClause),
}
```

---

## Capítulo 18. Construir el lexer y el parser

Se implementa:

```text
Texto → tokens → AST
```

Se comparan:

- Parser manual.
- Parser combinators.
- Generadores de parser.

Para fines didácticos, conviene crear un parser manual pequeño.

Hito:

```rust
parse(
    "MATCH (p:Person) RETURN p"
)
```

---

## Capítulo 19. Del AST al plan lógico

Ejemplo:

```text
MATCH (p:Person)-[:KNOWS]->(f:Person)
WHERE p.name = "Ana"
RETURN f.name
```

Plan:

```text
Project(f.name)
  Filter(p.name = "Ana")
    Expand(p, KNOWS, f)
      NodeScan(Person AS p)
```

Contenido:

- Operadores.
- Variables.
- Expresiones.
- Resolución de nombres.
- Validación semántica.
- Inferencia de tipos básica.

---

## Capítulo 20. El motor de ejecución

Se implementan operadores:

- `NodeScan`
- `IndexSeek`
- `Expand`
- `Filter`
- `Project`
- `Limit`
- `Distinct`

Interfaz posible:

```rust
pub trait PhysicalOperator {
    fn open(&mut self) -> Result<()>;
    fn next(&mut self) -> Result<Option<Row>>;
    fn close(&mut self) -> Result<()>;
}
```

Se explican:

- Modelo iterator o Volcano.
- Materialización.
- Pipelines.
- Pull frente a push.
- Procesamiento por lotes.

Hito:

> Ejecutar consultas completas desde texto.

---

## Capítulo 21. Un optimizador pequeño pero real

Reglas:

- Empujar filtros hacia abajo.
- Utilizar índices.
- Eliminar proyecciones innecesarias.
- Elegir el punto inicial más selectivo.
- Reordenar expansiones sencillas.

Ejemplo:

```text
Antes:

Expand
  Filter(name = "Ana")
    NodeScan(Person)

Después:

Expand
  IndexSeek(Person.name = "Ana")
```

Contenido:

- Optimización basada en reglas.
- Estadísticas.
- Cardinalidad.
- Selectividad.
- Coste estimado.

Hito:

```bash
liradb explain 'MATCH ...'
```

Salida:

```text
IndexSeek Person.name = "Ana"    estimated: 1
Expand KNOWS                     estimated: 12
Project friend.name              estimated: 12
```

---

# Parte V — Algoritmos sobre el grafo persistente

## Capítulo 22. Caminos mínimos ponderados

Contenido:

- Dijkstra.
- Bellman-Ford.
- Pesos negativos.
- Cola de prioridad.
- Finalización anticipada.
- Lectura de pesos desde propiedades.

Consulta:

```text
SHORTEST PATH
FROM node:1
TO node:42
WEIGHT relationship.distance
```

---

## Capítulo 23. A*, heurísticas y búsquedas dirigidas

Contenido:

- Diferencia entre Dijkstra y A*.
- Heurísticas admisibles.
- Uso de coordenadas.
- Ejemplo de rutas.
- Cuándo A* no ayuda.

Hito:

> Calcular rutas sobre una red de ciudades.

---

## Capítulo 24. Centralidad y PageRank

Contenido:

- Grado.
- Closeness.
- Betweenness.
- Eigenvector centrality.
- PageRank.
- Iteraciones.
- Convergencia.
- Coste computacional.

No se implementarían todas con optimización industrial. Se utilizarían para explicar familias de algoritmos.

Hito:

```bash
liradb algorithm page-rank \
  --label Page \
  --relationship LINKS
```

---

## Capítulo 25. Comunidades y agrupaciones

Contenido:

- Componentes.
- Label propagation.
- Modularidad.
- Louvain, explicado e implementado en versión simplificada.
- Casos de uso.
- Limitaciones.

Hito:

> Detectar grupos en una red social pequeña.

---

## Capítulo 26. Ejecutar algoritmos sin agotar la memoria

Contenido:

- Vista proyectada del grafo.
- Streaming.
- Procesamiento por bloques.
- Frontiers.
- Bitsets.
- Paralelismo.
- Snapshots.
- Diferencia entre OLTP y analítica.

Este capítulo conecta algoritmos académicos con restricciones reales de una base de datos.

---

# Parte VI — Fiabilidad

## Capítulo 27. Qué significa una transacción

Contenido:

- Atomicidad.
- Consistencia.
- Aislamiento.
- Durabilidad.
- Autocommit.
- Transacciones explícitas.
- Lecturas sucias.
- Lost updates.

Modelo inicial:

```text
Múltiples lectores
Un único escritor
```

Es una restricción razonable para una implementación educativa.

---

## Capítulo 28. Write-ahead log

Regla central:

```text
El cambio se escribe en el WAL antes que en la página de datos.
```

Contenido:

- Log sequence numbers.
- Begin, commit y rollback.
- Registros redo.
- Flush.
- Group commit.
- Checksums.
- Log truncation.

Hito:

> Simular un fallo durante una escritura y recuperar la base.

---

## Capítulo 29. Recuperación después de un fallo

Contenido:

- Arranque limpio y arranque tras fallo.
- Redo.
- Undo simplificado.
- Checkpoints.
- Páginas parcialmente escritas.
- Corrupción.
- Herramientas de reparación.

Pruebas:

```text
1. Iniciar transacción.
2. Crear nodos.
3. Matar el proceso.
4. Reiniciar.
5. Verificar invariantes.
```

---

## Capítulo 30. Snapshots y concurrencia

Contenido:

- Locks.
- Read-write locks.
- Snapshot isolation.
- MVCC.
- Versiones de registros.
- Garbage collection.
- Conflictos de escritura.

En la versión lite se implementaría una aproximación limitada:

```text
Snapshot por transacción
Un escritor
Lectores concurrentes
```

---

# Parte VII — Convertir el proyecto en un producto técnico

## Capítulo 31. La CLI de LiraDB

Comandos:

```bash
liradb init social.db
liradb shell social.db
liradb import social.db people.csv
liradb query social.db 'MATCH ...'
liradb explain social.db 'MATCH ...'
liradb inspect social.db
liradb check social.db
liradb compact social.db
liradb benchmark social.db
```

REPL:

```text
lira> MATCH (p:Person) RETURN p.name LIMIT 10;
```

---

## Capítulo 32. Importación y exportación

Contenido:

- CSV.
- JSON Lines.
- Importación por lotes.
- Validación.
- Reintentos.
- IDs externos.
- Duplicados.
- Exportación a GraphML o formato similar.

Hito:

> Importar un conjunto de datos realista de miles de nodos.

---

## Capítulo 33. Pruebas de una base de datos

Tipos:

- Unitarias.
- Contratos.
- Integración.
- Property-based testing.
- Fuzzing.
- Golden tests.
- Crash testing.
- Pruebas de compatibilidad del formato.

Invariantes:

```text
Toda relación referencia nodos existentes.
Cada relación saliente tiene su entrada correspondiente.
Ningún slot apunta fuera de una página.
Los índices contienen solo IDs válidos.
```

Uso de herramientas del ecosistema Rust:

- `proptest`.
- `cargo-fuzz`.
- `criterion`.
- `insta`, cuando tenga sentido.

---

## Capítulo 34. Benchmarks y perfilado

Contenido:

- Microbenchmarks.
- Benchmarks de consulta.
- Warm cache y cold cache.
- Throughput.
- Latencia.
- Percentiles.
- Flamegraphs.
- Contadores internos.
- Comparaciones honestas.

Dataset generado:

```text
100.000 personas
500.000 relaciones
10 etiquetas
20 propiedades
```

Se evita la comparación sensacionalista con Neo4j. El objetivo es medir la evolución de LiraDB contra sí misma.

---

## Capítulo 35. Observabilidad interna

Métricas:

```text
queries_total
query_duration
nodes_scanned
relationships_expanded
index_hits
page_reads
page_writes
buffer_hit_ratio
wal_bytes_written
transactions_committed
transactions_aborted
```

Trazas:

```text
parse
plan
optimise
execute
storage_read
index_seek
expand
```

Hito:

```bash
liradb query --profile 'MATCH ...'
```

---

## Capítulo 36. Arquitectura final de LiraDB

Diagrama de componentes:

```text
                    ┌─────────────┐
                    │ CLI / API   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ Parser      │
                    └──────┬──────┘
                           │ AST
                    ┌──────▼──────┐
                    │ Planner     │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ Optimizer   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ Executor    │
                    └──────┬──────┘
                           │
             ┌─────────────▼─────────────┐
             │ Graph Storage Engine      │
             ├───────────┬───────────────┤
             │ Indexes   │ Adjacencies   │
             ├───────────┴───────────────┤
             │ Buffer Pool               │
             ├───────────────────────────┤
             │ Pager / WAL / Recovery    │
             └───────────────────────────┘
```

Se revisan:

- Límites de los módulos.
- Dependencias.
- Decisiones arquitectónicas.
- Deuda técnica.
- Qué haríamos diferente.

---

# Parte VIII — De LiraDB Lite a un sistema avanzado

## Capítulo 37. Qué necesitaría una base de datos de producción

Contenido:

- Compatibilidad de formatos.
- Seguridad.
- Autenticación.
- Autorización.
- Cifrado.
- Backups.
- Migraciones.
- Control de recursos.
- Protección ante consultas costosas.
- Telemetría.
- Herramientas operativas.

---

## Capítulo 38. Almacenamiento columnar y ejecución vectorizada

Contenido avanzado:

- Row store frente a column store.
- Columnas de propiedades.
- Compresión.
- Dictionary encoding.
- Bit packing.
- Vectores.
- SIMD.
- Batch execution.
- Factorización de resultados.

Aquí se conectaría el proyecto con ideas utilizadas por motores como Kùzu.

---

## Capítulo 39. Joins, patrones y consultas cíclicas

Contenido:

- Expand como join.
- Joins binarios.
- Muchos-a-muchos.
- Triángulos.
- Worst-case optimal joins.
- Consultas recursivas.
- Explosión de resultados intermedios.
- Factorized execution.

Este sería uno de los capítulos que lleva al lector de nivel intermedio a experto.

---

## Capítulo 40. Distribuir una base de datos de grafos

No se implementaría por completo, pero sí se diseñaría.

Contenido:

- Particionado por nodos.
- Edge cuts y vertex cuts.
- Replicación de fronteras.
- Consultas entre particiones.
- Consistencia.
- Raft.
- Rebalanceo.
- Hotspots.
- Por qué distribuir grafos es difícil.

---

## Epílogo. Ya sabes construir una base de datos

El epílogo resume el recorrido:

```text
Vec<Vec<Edge>>
      ↓
Grafo en memoria
      ↓
Almacenamiento persistente
      ↓
Índices
      ↓
Lenguaje de consultas
      ↓
Planificador
      ↓
Transacciones
      ↓
Recuperación
      ↓
Motor de grafos completo
```

El lector no habrá construido un Neo4j, pero entenderá las piezas que hacen posible uno.

# Organización del código fuente

Conviene estructurar el repositorio como un workspace de Rust:

```text
liradb/
├── Cargo.toml
├── crates/
│   ├── lira-types/
│   ├── lira-graph/
│   ├── lira-algorithms/
│   ├── lira-storage/
│   ├── lira-buffer/
│   ├── lira-index/
│   ├── lira-transaction/
│   ├── lira-query-parser/
│   ├── lira-query-plan/
│   ├── lira-query-executor/
│   ├── lira-engine/
│   ├── lira-cli/
│   └── lira-testkit/
├── examples/
├── datasets/
├── benchmarks/
├── docs/
└── justfile
```

Pero no conviene comenzar el libro con doce crates. Inicialmente se utilizaría:

```text
liradb/
└── src/
    ├── graph.rs
    ├── algorithms.rs
    └── main.rs
```

Los crates se extraerían según aparezcan límites reales.

Esto enseña una lección importante:

> La arquitectura no se dibuja completamente antes del proyecto; evoluciona a partir de responsabilidades comprobadas.

# Proyecto conductor

El libro necesita un dominio que crezca con todas las funcionalidades.

Propongo una **plataforma de conocimiento técnico**:

```text
Person
Repository
Service
Technology
Document
Incident
Team
```

Relaciones:

```text
Person ──MEMBER_OF──> Team
Team ──OWNS──> Service
Service ──DEPENDS_ON──> Service
Repository ──IMPLEMENTS──> Service
Service ──USES──> Technology
Incident ──AFFECTED──> Service
Document ──DESCRIBES──> Service
```

Este dominio es mejor que la típica red social porque permite trabajar con:

- Caminos de dependencias.
- Impacto de incidentes.
- Ownership.
- Documentación.
- Centralidad.
- Servicios críticos.
- Comunidades tecnológicas.
- Análisis de arquitectura.
- Detección de dependencias circulares.

Ejemplos:

```text
¿Qué servicios dependen directa o indirectamente de PostgreSQL?
```

```text
¿Qué equipo es propietario de un servicio afectado?
```

```text
¿Cuál es el camino de dependencias entre Checkout y Payments?
```

```text
¿Qué servicio tiene mayor centralidad?
```

```text
¿Existen dependencias circulares?
```

Además, conecta muy bien con tu experiencia en DevOps y plataformas internas.

# Progresión por versiones

Cada parte del libro produciría una versión ejecutable.

| Versión | Resultado |
|---|---|
| `v0.1` | Grafo básico en memoria |
| `v0.2` | BFS, DFS y componentes |
| `v0.3` | Property Graph |
| `v0.4` | Persistencia append-only |
| `v0.5` | Páginas y buffer pool |
| `v0.6` | Índices y adyacencias persistentes |
| `v0.7` | Lenguaje de consultas |
| `v0.8` | Planificador y ejecutor |
| `v0.9` | WAL y transacciones |
| `v1.0` | LiraDB Lite completa |
| `v1.1` | Algoritmos analíticos |
| `v1.2` | Ejecución vectorizada experimental |

Cada etiqueta del repositorio debería corresponder con el final de una parte del libro.

# Estilo narrativo

## Tono

- Cercano, pero no infantil.
- Técnico, pero sin asumir conocimientos ocultos.
- Pragmático.
- Transparente sobre las simplificaciones.
- Orientado a decisiones.
- Con sentido de progresión.

Evitaría frases como:

> Como todo el mundo sabe, un B+ tree…

Y utilizaría:

> Necesitamos encontrar un nodo sin recorrer todo el fichero. Antes de elegir una estructura, midamos qué cuesta hacerlo de la forma más sencilla.

## Principio: enseñar desde el problema

En lugar de comenzar con:

> Un write-ahead log es una técnica…

Comenzaría con:

> Nuestra base puede quedar corrupta si el proceso se detiene después de modificar una página, pero antes de escribir las restantes. Vamos a provocar el fallo deliberadamente.

Después:

1. Se reproduce el problema.
2. Se observa el daño.
3. Se introduce el WAL.
4. Se implementa.
5. Se vuelve a ejecutar el fallo.
6. Se verifica la recuperación.

## Visualización progresiva

Cada algoritmo debería mostrarse en tres niveles:

### Nivel conceptual

```text
Ana → Luis → Marta
     ↘ Pedro
```

### Nivel de estructuras

```text
queue   = [Luis, Pedro]
visited = {Ana, Luis, Pedro}
parent  = {Luis: Ana, Pedro: Ana}
```

### Nivel de almacenamiento

```text
Page 12
Slot 3 → Node Luis
Adjacency segment → [Relationship 8, Relationship 14]
```

Esto une teoría, código y funcionamiento físico.

# Recursos pedagógicos recurrentes

Cada capítulo debería incluir:

## El objetivo

```text
Al terminar este capítulo podrás recuperar los vecinos de un nodo persistido sin escanear todas las relaciones.
```

## El problema

Un escenario reproducible que justifique la nueva funcionalidad.

## Modelo mental

Una explicación visual y sin implementación.

## Primera solución

La versión más sencilla que funciona.

## Sus límites

Mediciones o casos que la rompen.

## Solución evolucionada

Diseño más robusto.

## Código completo

Solo el código central en el libro. El repositorio contiene el resto.

## Prueba de fuego

Un test que demuestra que la funcionalidad resuelve el problema.

## Qué hemos sacrificado

Ejemplo:

```text
Nuestra implementación permite un único escritor. Esto simplifica la consistencia, pero reduce el throughput de escritura.
```

## Cómo lo hace una base real

Breve contraste con Neo4j, Kùzu, CozoDB, Oxigraph u otros motores, sin intentar copiar su implementación.

## Retos

Tres niveles:

- Esencial.
- Intermedio.
- Experto.

# Tipos de ejercicios

## Ejercicio esencial

> Añade soporte para relaciones no dirigidas.

## Ejercicio intermedio

> Implementa eliminación perezosa de nodos mediante tombstones.

## Ejercicio experto

> Sustituye la lista de adyacencia por segmentos ordenados por tipo de relación y compara el rendimiento.

## Investigación

> Estudia por qué CSR es excelente para analítica, pero problemático para actualizaciones frecuentes.

## Diseño

> Escribe un ADR comparando hash index y B+ tree para las propiedades.

## Depuración

> Se proporciona una base dañada y el lector debe encontrar el incumplimiento de una invariante.

# Material complementario

El libro debería acompañarse de:

```text
Repositorio Git
├── Código por capítulos
├── Tags por versión
├── Soluciones a ejercicios seleccionados
├── Datasets
├── Benchmarks
├── Diagramas
├── ADR
├── Tests de fallo
└── Playground
```

También serían útiles:

- Vídeos cortos para capítulos especialmente visuales.
- Animaciones de BFS, DFS, Dijkstra y PageRank.
- Visor de páginas de almacenamiento.
- Visualizador del plan de ejecución.
- Sandbox web opcional mediante WebAssembly.
- Cuadernos de experimentación.
- Hoja de referencia del lenguaje de consulta.

# Cuántas páginas tendría

Una edición completa podría quedar así:

| Parte | Páginas aproximadas |
|---|---:|
| Fundamentos de grafos | 100 |
| Modelo y persistencia | 130 |
| Motor de almacenamiento | 180 |
| Lenguaje y consultas | 170 |
| Algoritmos avanzados | 120 |
| Transacciones y recuperación | 140 |
| Operación, pruebas y rendimiento | 110 |
| Temas expertos | 100 |
| Total | 950–1.050 |

Eso sería demasiado para una primera edición.

## Primera edición más viable

Yo publicaría primero un libro de entre **500 y 650 páginas**, con este alcance:

1. Grafos en memoria.
2. Algoritmos fundamentales.
3. Property Graph.
4. Persistencia append-only.
5. Páginas y buffer pool.
6. Índices.
7. Adyacencias persistentes.
8. Lenguaje pequeño.
9. Ejecución.
10. WAL y recuperación.
11. CLI.
12. Benchmarks.
13. PageRank y caminos mínimos.

Los temas distribuidos y los algoritmos de joins avanzados podrían quedar para un segundo volumen.

# División recomendada en dos volúmenes

## Volumen I — Construye LiraDB

**De cero a motor persistente**

- Estructuras de grafos.
- Algoritmos básicos.
- Modelo Property Graph.
- Persistencia.
- Páginas.
- Buffer pool.
- Índices.
- Adyacencias.
- CLI.
- Pruebas.

Resultado:

> Una base embebida capaz de crear, recuperar y recorrer grafos persistentes.

## Volumen II — Consultas, transacciones y analítica

**De motor persistente a sistema de consultas**

- Lenguaje.
- Parser.
- Planificador.
- Ejecución.
- Optimización.
- WAL.
- Recuperación.
- MVCC.
- PageRank.
- Comunidades.
- Vectorización.
- Diseño distribuido.

Resultado:

> Un pequeño DBMS de grafos con lenguaje y motor de consultas.

# Mi recomendación final de estructura

Para no perder al lector ni crear un proyecto inabarcable, utilizaría esta secuencia central:

```text
1. Grafo en memoria
2. Recorridos
3. Property Graph
4. API de almacenamiento
5. Serialización
6. Persistencia append-only
7. Páginas
8. Buffer pool
9. Adyacencias persistentes
10. Índices
11. Lenguaje de consultas
12. Plan lógico
13. Motor de ejecución
14. Optimizador sencillo
15. Transacciones
16. WAL y recuperación
17. Algoritmos analíticos
18. Pruebas, benchmarks y observabilidad
```

El mayor valor diferencial del libro no sería simplemente implementar BFS o un B+ tree. Sería mostrar **cómo todas estas piezas se conectan para formar un sistema coherente**, incluyendo las decisiones equivocadas, las mediciones y las sucesivas evoluciones de LiraDB.

La promesa editorial podría resumirse así:

> Al terminar este libro no solo sabrás utilizar una base de datos de grafos. Sabrás cómo está construida, por qué toma determinadas decisiones y cómo crear tu propio motor desde los primeros nodos en memoria hasta una consulta persistente, transaccional y optimizada.

---

# you asked

message time: 2026-07-30 10:54:24

la idea es de usar codigo sin dependencias de librerias y crates especializados en primera instancia para enseñar bases, pero una vez mostradas usar y enseñar crates mas optimos para hacer lo mismo bien explicados, elegimos crates solidos, con mucho recorrido y apoyo.

---

# chatgpt response

Voy a convertir el libro en una progresión de dos capas: primero implementación “a mano” con `std`, después sustitución razonada por crates maduros, midiendo qué se gana y qué se pierde.

Exactamente. Esa decisión mejora mucho el libro: **primero comprender construyendo; después aprender ingeniería real sustituyendo partes por crates maduros**.

La idea no sería decir «las librerías son malas», sino enseñar tres niveles:

```text
1. Implementación didáctica con std
2. Implementación profesional con crates
3. Comparación: corrección, rendimiento y mantenibilidad
```

Así el lector entiende qué hace el crate, qué problema resuelve y qué abstracciones está comprando.

# Principio editorial: implementar, sustituir y medir

Cada bloque importante del libro tendría dos versiones.

## Versión A: implementación de referencia

Solo utilizaría:

```rust
std::collections
std::fs
std::io
std::sync
std::time
```

Su objetivo sería:

- Mostrar el algoritmo.
- Exponer las estructuras internas.
- Hacer visibles los costes.
- Provocar los errores habituales.
- Mantener el código suficientemente pequeño para estudiarlo.

No tendría que ser la versión más rápida ni completa, pero sí:

- Correcta.
- Legible.
- Probada.
- Determinista.
- Fácil de depurar.

## Versión B: implementación con el ecosistema Rust

Una vez comprendida la pieza, se sustituiría por un crate seleccionado.

El lector aprendería:

- Qué ofrece el crate.
- Cómo está modelada su API.
- Qué decisiones internas toma.
- Qué garantías proporciona.
- Qué limitaciones tiene.
- Cómo integrarlo sin acoplar todo el sistema.
- Cuándo no merece la pena utilizarlo.

## Versión C: comparación

Cada migración terminaría con una comparación como esta:

| Aspecto | Implementación manual | Crate |
|---|---|---|
| Claridad conceptual | Alta | Media |
| Líneas de código | 250 | 30 |
| Rendimiento | Básico | Optimizado |
| Casos límite | Parciales | Amplios |
| Mantenimiento | Nuestro | Compartido |
| Control interno | Completo | Limitado |
| Adecuación educativa | Excelente | Complementaria |
| Adecuación productiva | Limitada | Mayor |

El objetivo no sería que el crate gane siempre. En algunos componentes, como el formato persistente o el buffer pool, podría ser mejor conservar la implementación propia.

# Plantilla definitiva de los capítulos

Cada capítulo técnico importante seguiría esta estructura:

1. **El problema real.**
2. **Modelo mental y teoría mínima.**
3. **Diseño de la solución sencilla.**
4. **Implementación utilizando únicamente `std`.**
5. **Pruebas de corrección.**
6. **Medición y descubrimiento de límites.**
7. **Presentación del crate.**
8. **Lectura guiada de su API y conceptos internos.**
9. **Sustitución mediante un adaptador.**
10. **Pruebas diferenciales entre ambas implementaciones.**
11. **Benchmark comparativo.**
12. **Decisión para LiraDB y ADR.**
13. **Ejercicios de extensión.**

Esta fórmula sería una de las señas de identidad del libro.

# Selección inicial de crates

Los crates deberían agruparse en tres categorías.

## Nivel 1: crates fundamentales del ecosistema

Son los que utilizaría sin demasiadas reservas cuando llegue su momento:

- `petgraph`
- `serde`
- `thiserror`
- `clap`
- `tracing`
- `proptest`
- `criterion`

## Nivel 2: crates especializados

Se introducirían después de haber implementado manualmente el concepto:

- `slotmap`
- `logos`
- `pest`
- `zerocopy`
- `memmap2`
- `crc32fast`
- `lru`

## Nivel 3: motores completos utilizados como alternativa

No formarían parte del motor principal porque esconderían demasiado:

- `redb`
- Eventualmente un backend basado en RocksDB, solo como comparación avanzada.

`redb` es especialmente interesante para construir una variante de LiraDB sobre un almacén clave-valor transaccional: ofrece almacenamiento embebido ACID en Rust y utiliza árboles B+ copy-on-write. Pero se introduciría después de construir páginas, índices y transacciones, porque utilizarlo antes eliminaría gran parte del aprendizaje. ([Docs.rs](https://docs.rs/redb?utm_source=chatgpt.com))

# Mapa completo: implementación manual y crate

## 1. Identificadores estables

### Primero, a mano

El lector construiría:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    slot: u32,
    generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}
```

Así entendería:

- Por qué un índice de `Vec` no siempre es un ID seguro.
- Qué ocurre al borrar y reutilizar posiciones.
- Qué es una referencia obsoleta.
- Cómo funcionan las generaciones.
- Qué papel tiene una free list.

### Después, `slotmap`

Se sustituiría por:

```rust
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct NodeId;
    pub struct RelationshipId;
}

pub struct Graph {
    nodes: SlotMap<NodeId, Node>,
    relationships: SlotMap<RelationshipId, Relationship>,
}
```

`slotmap` proporciona contenedores con claves persistentes y únicas, precisamente el problema que habremos implementado manualmente. También dispone de variantes como `DenseSlotMap`, lo que permite comparar estabilidad de identificadores, densidad y localidad. ([Docs.rs](https://docs.rs/slotmap/?utm_source=chatgpt.com))

### Decisión para LiraDB

- `NodeId` persistido: identificador lógico propio.
- IDs temporales del grafo en memoria: `slotmap`.
- Nunca persistir directamente una clave interna de `slotmap` sin definir antes un contrato de formato.

---

## 2. Representación del grafo

### Primero, a mano

Se implementarían sucesivamente:

```rust
Vec<Vec<NodeId>>
```

Después:

```rust
pub struct AdjacencyGraph {
    nodes: Vec<Node>,
    outgoing: Vec<Vec<RelationshipId>>,
    incoming: Vec<Vec<RelationshipId>>,
    relationships: Vec<Relationship>,
}
```

Y finalmente una representación CSR:

```rust
pub struct CsrGraph {
    offsets: Vec<usize>,
    targets: Vec<NodeId>,
}
```

El lector mediría:

- Inserciones.
- Eliminaciones.
- Recorrido de vecinos.
- Consumo de memoria.
- Localidad.
- Grafos estáticos frente a dinámicos.

### Después, `petgraph`

`petgraph` es la elección principal porque proporciona múltiples representaciones y algoritmos sobre traits comunes:

- `Graph`: lista de adyacencia.
- `StableGraph`: mantiene estables los índices no relacionados cuando se eliminan elementos.
- `GraphMap`: grafo respaldado por mapas.
- `MatrixGraph`: matriz.
- `Csr`: representación compressed sparse row. ([Docs.rs](https://docs.rs/petgraph/latest/petgraph/?utm_source=chatgpt.com))

Esto encaja perfectamente con el enfoque del libro: no se enseña solamente a usar `petgraph`, sino a comprender por qué contiene varios tipos de grafo.

### Capítulo comparativo

```text
Nuestro AdjacencyGraph frente a petgraph::Graph
Nuestra GenerationalGraph frente a petgraph::StableGraph
Nuestro CsrGraph frente a petgraph::csr::Csr
```

`petgraph::Csr` ocupa espacio `O(|V| + |E|)` y está orientado a iterar rápidamente las aristas salientes, aunque restringe algunas operaciones y no admite aristas paralelas. Ese tipo de diferencia debe ser estudiado, no ocultado. ([Docs.rs](https://docs.rs/petgraph/latest/petgraph/csr/struct.Csr.html?utm_source=chatgpt.com))

---

## 3. BFS y DFS

### Primero, a mano

```rust
pub fn breadth_first_search<G>(
    graph: &G,
    start: G::NodeId,
) -> Vec<G::NodeId>
where
    G: NeighborProvider,
{
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut result = Vec::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(node) = queue.pop_front() {
        result.push(node);

        for neighbor in graph.neighbors(node) {
            if visited.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }

    result
}
```

Se construirían:

- Cola.
- Pila.
- Registro de visitados.
- Reconstrucción de caminos.
- Recorridos iterativos y recursivos.
- Trait mínimo requerido por un algoritmo.

### Después, traits de visita de `petgraph`

El lector aprendería que un algoritmo no debería depender necesariamente de una estructura concreta.

`petgraph` separa recorridos, estructuras y capacidades mediante su módulo `visit`; por ejemplo, su DFS mantiene explícitamente una pila y un mapa de nodos descubiertos. ([Docs.rs](https://docs.rs/petgraph/latest/petgraph/visit/struct.Dfs.html?utm_source=chatgpt.com))

Este sería un punto excelente para enseñar diseño genérico en Rust:

```rust
IntoNeighbors
IntoNodeIdentifiers
Visitable
NodeIndexable
```

No se presentarían los traits como magia. Primero habríamos construido nuestra propia interfaz pequeña.

---

## 4. Algoritmos de grafos

### Implementaciones manuales

El libro implementaría íntegramente:

- BFS.
- DFS.
- Dijkstra.
- A*.
- Bellman-Ford.
- Ordenación topológica.
- Componentes conexos.
- Kosaraju o Tarjan.
- Kruskal.
- PageRank.
- Label propagation simplificado.

### Posteriormente, `petgraph::algo`

`petgraph` ofrece algoritmos como Dijkstra, A*, Bellman-Ford, Floyd-Warshall, componentes fuertemente conexos, árboles de expansión y PageRank, entre otros. ([Docs.rs](https://docs.rs/petgraph/latest/petgraph/algo/page_rank/fn.page_rank.html?utm_source=chatgpt.com))

El libro debería comparar:

```rust
manual::dijkstra(&graph, source)
```

con:

```rust
petgraph::algo::dijkstra(
    &graph,
    source,
    Some(destination),
    |edge| edge.weight().cost,
);
```

La comparación no se limitaría al código. También cubriría:

- Representación del resultado.
- Trait bounds.
- Generalización sobre el tipo de grafo.
- Costes de asignación.
- Manejo de pesos.
- Finalización anticipada.
- Errores y precondiciones.

### Pruebas diferenciales

Cada algoritmo tendría una prueba común:

```rust
#[test]
fn manual_and_petgraph_dijkstra_agree() {
    let case = fixtures::weighted_graph();

    let manual = manual::dijkstra(&case.manual, case.start);
    let library = petgraph_adapter::dijkstra(&case.petgraph, case.start);

    assert_eq!(manual.distances(), library.distances());
}
```

Esto enseña una técnica muy valiosa: usar una implementación reconocida como **oráculo parcial de corrección**.

---

## 5. Serialización

Aquí conviene ser especialmente cuidadosos.

### Primero, formato manual

El lector implementaría:

```rust
fn encode_u32(value: u32, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn decode_u32(input: &[u8], offset: &mut usize) -> Result<u32, DecodeError> {
    let end = offset.checked_add(4).ok_or(DecodeError::Overflow)?;
    let bytes = input
        .get(*offset..end)
        .ok_or(DecodeError::UnexpectedEnd)?;

    *offset = end;

    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length validated"),
    ))
}
```

Se enseñarían:

- Endianness.
- Longitudes.
- Offsets.
- Alineamiento.
- Padding.
- Versionado.
- Compatibilidad.
- Corrupción.
- Límites de memoria.
- Validación de datos no confiables.

### Después, `serde`

Serde separa los tipos serializables de los formatos concretos mediante los traits `Serialize` y `Deserialize`. Es ideal para:

- Configuración.
- JSON de importación y exportación.
- Ficheros de diagnóstico.
- Protocolos externos.
- Fixtures de pruebas.
- Snapshots del catálogo. ([serde.rs](https://serde.rs/?utm_source=chatgpt.com))

Pero **no utilizaría Serde como sustituto automático del formato principal de páginas**.

Una base de datos necesita controlar explícitamente:

- El formato físico.
- La estabilidad entre versiones.
- La recuperación parcial.
- Las migraciones.
- La validación.
- La compatibilidad hacia delante y atrás.

Serde se enseñaría como una capa de interoperabilidad, no como excusa para ignorar el formato de almacenamiento.

### No elegiría `bincode` para el formato canónico

Aunque ha sido popular, el propio registro de crates indica desde diciembre de 2025 que `bincode` no está mantenido y que no se publicarán más versiones. Por tanto, no lo utilizaría como dependencia estratégica de un libro que pretende seleccionar crates con continuidad. ([crates.io](https://crates.io/crates/bincode?utm_source=chatgpt.com))

Puede mencionarse históricamente y utilizarse en un ejercicio de evaluación de dependencias, pero no sería parte oficial de LiraDB.

---

## 6. Representación binaria segura

### Primero, a mano

El lector escribiría las cabeceras campo por campo:

```rust
pub struct PageHeader {
    page_type: u8,
    version: u8,
    flags: u16,
    page_id: u64,
    checksum: u32,
}
```

Y aprendería por qué esto es peligroso:

```rust
// No asumir que la memoria de la struct
// es directamente el formato de disco.
```

Se explicarían:

- Padding.
- Alineamiento.
- `repr(C)`.
- `repr(packed)`.
- Endianness.
- Validez de bit patterns.
- Por qué `transmute` no es un serializador.

### Después, `zerocopy`

`zerocopy` proporciona traits derivados y tipos conscientes del orden de bytes para convertir de forma controlada entre representaciones tipadas y secuencias de bytes. ([Docs.rs](https://docs.rs/zerocopy/latest/zerocopy/?utm_source=chatgpt.com))

Se usaría para cabeceras pequeñas y muy controladas:

```rust
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
struct DiskPageHeader {
    magic: [u8; 4],
    version: u16,
    flags: u16,
    page_id: u64,
}
```

Pero el libro insistiría en que:

> Zero-copy no elimina la necesidad de diseñar un formato. Solo permite acceder a un formato correctamente definido con menos copias y más comprobaciones.

---

## 7. Checksums

### Primero, CRC32 didáctico

Se podría implementar un CRC sencillo para comprender:

- Polinomios.
- Tablas.
- Estado incremental.
- Detección de corrupción.
- Diferencia entre checksum y hash criptográfico.

### Después, `crc32fast`

`crc32fast` proporciona una implementación CRC32 optimizada, con selección dinámica de implementaciones aceleradas en arquitecturas compatibles. ([Docs.rs](https://docs.rs/crc32fast/latest/crc32fast/?utm_source=chatgpt.com))

La migración sería muy clara:

```rust
let checksum = manual_crc32(page);
```

frente a:

```rust
let checksum = crc32fast::hash(page);
```

Este sería uno de los mejores ejemplos de algo que merece la pena comprender, pero que rara vez conviene mantener manualmente.

---

## 8. Gestor de páginas e I/O

### Primero, `std::fs::File`

Se utilizarían:

```rust
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
```

Implementando:

```rust
pub trait Pager {
    fn allocate(&mut self) -> Result<PageId, PagerError>;
    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError>;
    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError>;
    fn sync(&mut self) -> Result<(), PagerError>;
}
```

El lector entendería:

- Offsets.
- Lecturas parciales.
- `seek`.
- `flush`.
- `sync_data`.
- `sync_all`.
- Tamaño de página.
- Extensión de fichero.
- Orden de persistencia.

### Después, `memmap2`

`memmap2` ofrece una API multiplataforma para mapear ficheros como regiones de memoria mediante `Mmap` y `MmapMut`. ([Docs.rs](https://docs.rs/crate/memmap2/latest?utm_source=chatgpt.com))

Se implementaría un segundo pager:

```rust
pub struct MmapPager {
    mapping: memmap2::MmapMut,
    page_size: usize,
}
```

Y se analizarían sus riesgos:

- Redimensionado.
- Invalidación del mapping.
- Acceso fuera del fichero.
- Flush.
- Interacción con el sistema operativo.
- Falta de control directo sobre la caché del kernel.

La documentación de `memmap2` advierte, por ejemplo, de condiciones potencialmente indefinidas si se accede más allá del final del fichero mapeado. Esto debe tratarse explícitamente en el libro. ([Docs.rs](https://docs.rs/memmap2/latest/memmap2/struct.Mmap.html?utm_source=chatgpt.com))

### Decisión para LiraDB

Mantendría dos implementaciones:

```text
FilePager     ← predeterminada y pedagógica
MmapPager     ← experimental y comparativa
```

No sustituiría automáticamente una por otra.

---

## 9. Buffer pool y políticas de expulsión

### Primero, a mano

Se implementarían:

- Frame.
- Page table.
- Pin count.
- Dirty flag.
- LRU sencillo.
- Clock.

```rust
pub trait EvictionPolicy {
    fn accessed(&mut self, frame: FrameId);
    fn pin(&mut self, frame: FrameId);
    fn unpin(&mut self, frame: FrameId);
    fn victim(&mut self) -> Option<FrameId>;
}
```

La política Clock probablemente debería continuar siendo propia, porque forma parte del corazón pedagógico y arquitectónico del motor.

### Después, `lru`

El crate `lru` ofrece una caché LRU con capacidad limitada y API directa para inserción, acceso y expulsión. ([Docs.rs](https://docs.rs/lru/latest/lru/?search=lru&utm_source=chatgpt.com))

Se utilizaría para comparar la política manual, no necesariamente para reemplazar el buffer pool:

```text
ManualLruPolicy
ClockPolicy
CrateLruPolicy
```

Esto permite demostrar que una caché genérica no es exactamente igual que un buffer pool:

- No entiende páginas sucias.
- No entiende pinning.
- No sabe hacer flush.
- No conoce dependencias con el WAL.
- No debería expulsar cualquier entrada indiscriminadamente.

Es un caso muy instructivo donde **un crate resuelve una parte, pero no toda la responsabilidad**.

---

## 10. Errores

### Primero, manualmente

```rust
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    CorruptPage {
        page_id: PageId,
        expected: u32,
        actual: u32,
    },
    InvalidPageType(u8),
}
```

Se implementarían:

```rust
impl Display for StorageError { /* ... */ }
impl Error for StorageError { /* ... */ }
impl From<std::io::Error> for StorageError { /* ... */ }
```

### Después, `thiserror`

`thiserror` genera implementaciones de `std::error::Error` manteniendo tipos de error explícitos y diseñados por la aplicación. ([Docs.rs](https://docs.rs/crate/thiserror/latest?utm_source=chatgpt.com))

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("error de entrada/salida")]
    Io(#[from] std::io::Error),

    #[error(
        "checksum incorrecto en la página {page_id:?}: \
         esperado {expected:#010x}, obtenido {actual:#010x}"
    )]
    CorruptPage {
        page_id: PageId,
        expected: u32,
        actual: u32,
    },
}
```

Es un buen ejemplo de dependencia que elimina código repetitivo sin esconder el modelo de dominio.

---

## 11. Lexer y parser

### Primero, completamente manual

Se construiría:

```text
Texto
  ↓
Caracteres
  ↓
Tokens
  ↓
Parser descendente
  ↓
AST
```

El lexer manual enseñaría:

- Cursores.
- Spans.
- Identificadores.
- Literales.
- Palabras reservadas.
- Errores léxicos.

El parser enseñaría:

- Gramática.
- Precedencia.
- Asociatividad.
- Recursión.
- AST.
- Recuperación de errores.

### Después, `logos`

`logos` genera lexers a partir de enums anotados y está orientado a combinar facilidad de definición con velocidad. ([Docs.rs](https://docs.rs/logos/latest/logos/?utm_source=chatgpt.com))

```rust
#[derive(Logos, Debug, PartialEq)]
enum TokenKind {
    #[token("MATCH")]
    Match,

    #[token("WHERE")]
    Where,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,
}
```

### Después, `pest`

En una segunda evolución, se mostraría cómo definir la gramática mediante PEG con `pest`, que está enfocado en accesibilidad, corrección y mantenibilidad de parsers. ([pest.rs](https://pest.rs/?utm_source=chatgpt.com))

Mi propuesta sería:

```text
LiraDB principal:
    Logos + parser descendente manual

Apéndice comparativo:
    Parser completo con Pest
```

¿Por qué?

Porque `logos` elimina la parte repetitiva del lexer, pero permite que el lector siga viendo claramente el AST, la precedencia y la planificación. `pest` sería útil para comparar un enfoque declarativo, pero no debería ocultar demasiado pronto el funcionamiento del parser.

---

## 12. CLI

### Primero, `std::env::args`

Se implementaría manualmente:

```rust
let mut args = std::env::args().skip(1);

match args.next().as_deref() {
    Some("init") => { /* ... */ }
    Some("query") => { /* ... */ }
    Some(command) => {
        return Err(CliError::UnknownCommand(command.to_owned()));
    }
    None => print_help(),
}
```

Esto permite explicar:

- Argumentos.
- Subcomandos.
- Validación.
- Códigos de salida.
- Ayuda.
- Separación CLI/aplicación.

### Después, `clap`

`clap` proporciona un parser de línea de comandos completo, derivable y con soporte para subcomandos, validación y generación de ayuda. Su documentación señala además su pertenencia al ámbito del Rust CLI Working Group y una política explícita de compatibilidad y cambios mayores. ([Docs.rs](https://docs.rs/clap?utm_source=chatgpt.com))

```rust
#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Init {
        path: PathBuf,
    },
    Query {
        path: PathBuf,
        expression: String,
    },
    Inspect {
        path: PathBuf,
    },
}
```

Aquí la conclusión será sencilla: una vez entendidos los fundamentos, no existe demasiado valor en mantener un parser CLI propio.

---

## 13. Observabilidad

### Primero, contadores y eventos propios

```rust
pub struct Metrics {
    pub page_reads: AtomicU64,
    pub page_writes: AtomicU64,
    pub buffer_hits: AtomicU64,
    pub buffer_misses: AtomicU64,
}
```

Y un evento sencillo:

```rust
pub enum EngineEvent {
    QueryStarted,
    PageRead(PageId),
    IndexSeek(IndexId),
    QueryFinished(Duration),
}
```

### Después, `tracing`

`tracing` es un framework para instrumentación estructurada basada en eventos y spans, especialmente adecuado para representar operaciones anidadas. ([Docs.rs](https://docs.rs/tracing?utm_source=chatgpt.com))

```rust
#[tracing::instrument(skip(self), fields(page_id = %page_id))]
fn read_page(&mut self, page_id: PageId) -> Result<Page, StorageError> {
    tracing::trace!("reading page");
    self.pager.read(page_id)
}
```

Se enseñaría la correspondencia:

```text
Query
├── Parse
├── Plan
├── Optimise
└── Execute
    ├── IndexSeek
    ├── Expand
    │   ├── PageRead
    │   └── PageRead
    └── Project
```

El lector entendería `tracing` porque antes habrá diseñado sus propios eventos.

---

## 14. Property-based testing

### Primero, tests convencionales

```rust
#[test]
fn deleting_a_node_removes_its_relationships() {
    // Arrange
    // Act
    // Assert
}
```

Después se definirían invariantes:

```text
Toda relación referencia dos nodos existentes.
Toda arista saliente tiene una representación entrante.
La decodificación de un valor codificado devuelve el valor original.
Una página nunca contiene slots solapados.
```

### Después, `proptest`

`proptest` genera entradas y reduce automáticamente los casos que provocan fallos, por lo que resulta muy apropiado para estructuras, parsers y formatos binarios. ([Docs.rs](https://docs.rs/crate/proptest/latest/source/Cargo.toml.orig?utm_source=chatgpt.com))

```rust
proptest! {
    #[test]
    fn encoded_values_round_trip(value in any::<i64>()) {
        let encoded = codec::encode_integer(value);
        let decoded = codec::decode_integer(&encoded)?;

        prop_assert_eq!(decoded, value);
    }
}
```

Posteriormente se introducirían pruebas con modelos de estado:

```text
Modelo sencillo en memoria
          frente a
Motor persistente real
```

Ese sería uno de los capítulos más importantes para acercar al lector a un nivel experto.

---

## 15. Benchmarks

### Primero, `Instant`

```rust
let started = Instant::now();

for _ in 0..iterations {
    black_box(graph.neighbors(node));
}

println!("elapsed: {:?}", started.elapsed());
```

Esto permite explicar:

- Warm-up.
- Ruido.
- Optimizaciones del compilador.
- Caché caliente y fría.
- Repeticiones.
- Por qué un único tiempo no es suficiente.

### Después, `criterion`

Criterion ofrece microbenchmarks basados en análisis estadístico para detectar mejoras y regresiones de rendimiento. ([Docs.rs](https://docs.rs/criterion/latest/criterion/?utm_source=chatgpt.com))

```rust
fn adjacency_lookup(c: &mut Criterion) {
    c.bench_function("adjacency/manual", |b| {
        b.iter(|| manual_graph.neighbors(black_box(node)));
    });

    c.bench_function("adjacency/petgraph", |b| {
        b.iter(|| petgraph_graph.neighbors(black_box(node)).count());
    });
}
```

Cada sustitución de crate debería incluir su benchmark correspondiente.

# Arquitectura para soportar las dos implementaciones

No conviene llenar el código de `#[cfg]` desde el comienzo. Primero habría un proyecto sencillo y, cuando aparezca la necesidad, se extraerían contratos.

## Contratos principales

```rust
pub trait GraphBackend {
    type NodeId: Copy + Eq + Hash;
    type RelationshipId: Copy + Eq + Hash;

    fn create_node(&mut self, node: Node) -> Result<Self::NodeId, GraphError>;

    fn create_relationship(
        &mut self,
        source: Self::NodeId,
        target: Self::NodeId,
        relationship: Relationship,
    ) -> Result<Self::RelationshipId, GraphError>;

    fn outgoing(
        &self,
        node: Self::NodeId,
    ) -> impl Iterator<Item = Self::RelationshipId> + '_;
}
```

```rust
pub trait PageStorage {
    fn allocate(&mut self) -> Result<PageId, StorageError>;
    fn read(&mut self, page: PageId) -> Result<Page, StorageError>;
    fn write(&mut self, page: PageId, value: &Page) -> Result<(), StorageError>;
}
```

```rust
pub trait QueryFrontend {
    fn parse(&self, query: &str) -> Result<Ast, QueryError>;
}
```

## Implementaciones

```text
GraphBackend
├── ManualGraph
└── PetgraphBackend

PageStorage
├── FilePager
├── MmapPager
└── RedbPageStore       opcional

QueryFrontend
├── ManualParser
├── LogosParser
└── PestParser          comparativo

EvictionPolicy
├── ManualLru
├── Clock
└── CrateLru
```

# Tests de contrato

Todas las implementaciones deberían superar las mismas pruebas:

```rust
pub fn graph_backend_contract<B>(mut backend: B)
where
    B: GraphBackend,
{
    let ana = backend.create_node(Node::person("Ana")).unwrap();
    let luis = backend.create_node(Node::person("Luis")).unwrap();

    backend
        .create_relationship(
            ana,
            luis,
            Relationship::new("KNOWS"),
        )
        .unwrap();

    let outgoing = backend.outgoing(ana).collect::<Vec<_>>();

    assert_eq!(outgoing.len(), 1);
}
```

Y se ejecutaría contra ambas:

```rust
#[test]
fn manual_backend_satisfies_contract() {
    graph_backend_contract(ManualGraph::new());
}

#[test]
fn petgraph_backend_satisfies_contract() {
    graph_backend_contract(PetgraphBackend::new());
}
```

Esto pone en valor:

- Arquitectura hexagonal.
- Sustitución de implementaciones.
- Ports and adapters.
- Tests reutilizables.
- Diferencia entre API y mecanismo interno.
- Ausencia de acoplamiento accidental.

# Cómo cambiaría el guion del libro

En lugar de una parte separada titulada «librerías», los crates aparecerían justo después de cada fundamento.

## Parte I — Grafos construidos a mano

1. Grafo mediante lista de aristas.
2. Matriz de adyacencia.
3. Lista de adyacencia.
4. Identificadores generacionales.
5. BFS.
6. DFS.
7. Dijkstra.
8. Componentes y ciclos.
9. CSR.

## Parte II — Del código didáctico al ecosistema Rust

10. Diseñar traits para abstraer representaciones.
11. `slotmap`: identidad estable.
12. `petgraph::Graph`.
13. `petgraph::StableGraph`.
14. `petgraph::Csr`.
15. Algoritmos de `petgraph`.
16. Pruebas diferenciales.
17. Benchmarks comparativos.
18. Decidir qué implementación conservar.

## Parte III — Persistencia construida a mano

19. Bytes, endianness y offsets.
20. Codec manual.
21. Log append-only.
22. Páginas.
23. Slotted pages.
24. Pager basado en `File`.
25. Buffer pool.
26. LRU.
27. Clock.
28. Checksums.
29. Formato versionado.

## Parte IV — Herramientas especializadas de almacenamiento

30. `zerocopy`.
31. `crc32fast`.
32. `memmap2`.
33. `lru`.
34. Serde para importación y herramientas.
35. Comparar pager tradicional y mmap.
36. Diseñar formatos estables.
37. Adapter opcional sobre `redb`.

## Parte V — Consultas a mano y con herramientas

38. Lexer manual.
39. Parser descendente.
40. AST.
41. Planner.
42. Motor Volcano.
43. Lexer con `logos`.
44. Gramática alternativa con `pest`.
45. CLI manual.
46. CLI con `clap`.

## Parte VI — Calidad y operación

47. Errores manuales y `thiserror`.
48. Métricas propias y `tracing`.
49. Tests convencionales.
50. Property-based testing con `proptest`.
51. Medición manual.
52. Benchmarks con Criterion.
53. Fuzzing y crash testing.
54. Auditoría de dependencias.

# Crates seleccionados para la primera edición

| Responsabilidad | Implementación inicial | Evolución recomendada |
|---|---|---|
| Grafo dinámico | Listas de adyacencia propias | `petgraph::Graph` |
| Índices estables | Arena generacional propia | `slotmap` o `petgraph::StableGraph` |
| Grafo analítico | CSR propio | `petgraph::Csr` |
| Algoritmos | Implementación propia | `petgraph::algo` |
| Serialización externa | Codec/JSON sencillo | `serde` |
| Formato interno | Codec binario propio | Mantener propio |
| Vistas binarias | Lectura de campos manual | `zerocopy` |
| Checksums | CRC didáctico | `crc32fast` |
| I/O | `File`, `Read`, `Write`, `Seek` | `memmap2` como alternativa |
| Buffer pool | LRU y Clock propios | `lru` como comparación |
| Errores | `Display`, `Error`, `From` | `thiserror` |
| Lexer | Manual | `logos` |
| Parser | Descendente manual | `pest` como variante |
| CLI | `std::env::args` | `clap` |
| Observabilidad | Eventos propios | `tracing` |
| Property testing | Casos manuales | `proptest` |
| Benchmarks | `Instant` | `criterion` |
| Backend ACID alternativo | Motor propio | `redb` opcional |

# Lo que no sustituiría por crates

Para conservar el valor formativo y la identidad del proyecto, mantendría propios:

- Formato de página.
- Slotted pages.
- Pager principal.
- Buffer pool principal.
- Política Clock.
- Índices de adyacencia persistentes.
- Catálogo.
- WAL.
- Recuperación.
- Transacciones.
- Plan lógico.
- Plan físico.
- Motor de ejecución.
- Optimizador.
- Formato persistente versionado.

Estos componentes son precisamente los que convierten el proyecto en una **base de datos construida desde cero**.

Utilizar un crate para todo eso produciría una biblioteca de grafos sobre otra base de datos, no un libro sobre cómo crear un DBMS.

# Lo que sí sustituiría casi siempre

Una vez aprendido el fundamento:

- Parsing de argumentos → `clap`.
- Derivación de errores → `thiserror`.
- Instrumentación → `tracing`.
- Checksums → `crc32fast`.
- Property testing → `proptest`.
- Benchmarks → Criterion.
- Lexer → `logos`.
- Serialización de interfaces externas → Serde.

Son responsabilidades en las que reinventar indefinidamente aporta poco valor al motor.

# Política para seleccionar dependencias

El libro debería documentar formalmente los criterios mediante un ADR:

```text
ADR-0004: Política de adopción de crates
```

Cada crate se evaluaría por:

1. Actividad y mantenimiento actuales.
2. Historial y estabilidad de la API.
3. Calidad de documentación.
4. Claridad de su alcance.
5. Compatibilidad y MSRV.
6. Número y naturaleza de dependencias transitivas.
7. Uso de `unsafe`.
8. Soporte multiplataforma.
9. Política de versiones.
10. Facilidad para ser sustituido.
11. Licencia.
12. Ausencia de dependencia directa del dominio.

Y se clasificaría:

```text
Adoptar
Adoptar detrás de un adaptador
Solo para desarrollo
Solo como ejemplo
Rechazar
```

# Versionado reproducible del libro

No pondría versiones exactas de crates repetidas continuamente en el texto, porque quedarían desactualizadas.

Cada capítulo tendría:

```text
Tag Git: chapter-12
Cargo.lock: incluido
rust-toolchain.toml: incluido
cargo metadata: reproducible
```

El texto diría:

```toml
petgraph = "0.8"
```

Pero la etiqueta del capítulo contendría el `Cargo.lock` exacto utilizado.

Además:

```text
main
├── edición mantenida
book-v1
├── versiones originales de publicación
chapter-01
chapter-02
...
chapter-54
```

Así el lector puede:

- Reproducir la edición original.
- Utilizar una rama actualizada.
- Comparar migraciones.
- Entender cambios de API.

# Promesa pedagógica revisada

La promesa del libro podría quedar así:

> Construiremos cada pieza fundamental de una base de datos de grafos utilizando Rust y su biblioteca estándar. Cuando entendamos su funcionamiento, la compararemos con crates maduros del ecosistema, sustituiremos aquello que no merece ser mantenido a mano y conservaremos bajo nuestro control las partes que definen el motor de almacenamiento, consultas y transacciones.

Ese enfoque consigue algo difícil: que el lector termine dominando tanto los **fundamentos internos** como la forma profesional de construir software real aprovechando correctamente el ecosistema Rust.

---

# you asked

message time: 2026-07-30 14:13:58

podemos extraer concepto y algoritmos de ladybug

---

# chatgpt response

Sí. Voy a identificar exactamente qué proyecto “Ladybug” tienes en mente y separar qué ideas podemos reutilizar legal y pedagógicamente de lo que conviene reimplementar desde cero.

Sí. **Ladybug es probablemente la mejor referencia moderna para llevar el libro desde una base de datos de grafos educativa hasta una arquitectura analítica avanzada**.

Pero no conviene copiar su arquitectura completa desde el principio. La usaría como una **fuente de conceptos, algoritmos y decisiones de diseño** que iremos introduciendo después de construir una versión sencilla.

Ladybug es la continuación del proyecto anteriormente conocido como Kùzu. Es una base de grafos embebida, orientada a consultas analíticas, con almacenamiento columnar en disco, adyacencias basadas en CSR, procesamiento vectorizado y factorizado, paralelismo multinúcleo y transacciones ACID serializables. Su repositorio se distribuye bajo licencia MIT. ([GitHub](https://github.com/LadybugDB/ladybug))

# Qué podemos extraer de Ladybug

Podemos estudiar Ladybug en cuatro niveles:

```text
1. Modelo conceptual
2. Estructuras de almacenamiento
3. Procesamiento de consultas
4. Algoritmos avanzados de ejecución
```

No todo debe entrar en la primera versión de LiraDB. Algunas ideas son fundamentales y otras deberían reservarse para la parte experta.

---

# 1. Arquitectura completa de procesamiento

Ladybug utiliza una cadena muy clara:

```text
Consulta Cypher
      ↓
Parser
      ↓
AST
      ↓
Binder
      ↓
BoundStatement
      ↓
Planner
      ↓
LogicalPlan
      ↓
Optimizer
      ↓
LogicalPlan optimizado
      ↓
Physical Planner / Mapper
      ↓
PhysicalPlan
      ↓
Processor
      ↓
Resultados
```

Su documentación diferencia claramente:

- **Parser**: comprende la sintaxis.
- **Binder**: resuelve tablas, propiedades, variables y tipos.
- **Planner**: genera un plan lógico correcto.
- **Optimizer**: transforma el plan sin cambiar su significado.
- **Processor**: selecciona algoritmos físicos concretos.
- **Expression evaluator**: evalúa filtros, proyecciones y expresiones.
- **Catalog**: mantiene el esquema.
- **Storage**: conserva los datos persistentes. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/?utm_source=chatgpt.com))

Esta división debería convertirse en la arquitectura final del proyecto del libro:

```text
lira-parser
lira-binder
lira-logical-plan
lira-optimizer
lira-physical-plan
lira-executor
lira-storage
lira-catalog
```

## Adaptación pedagógica

Primero construiremos una versión simplificada:

```text
Parser → AST → Executor
```

Después detectaremos sus problemas y evolucionaremos hacia:

```text
Parser → Binder → LogicalPlan → PhysicalPlan → Executor
```

Esto permite que el lector entienda por qué existe cada capa, en vez de presentarle una arquitectura enorme desde la primera página.

---

# 2. Separar tipos lógicos y físicos

Ladybug distingue entre:

- **Tipo lógico**: cómo entiende el usuario el dato.
- **Tipo físico**: cómo se representa realmente en memoria o disco.

Por ejemplo:

```text
DATE    → tipo lógico
INT32   → representación física
```

Dos tipos lógicos diferentes pueden compartir una representación física. El binder, el planner y el catálogo trabajan principalmente con tipos lógicos; el almacenamiento y la ejecución utilizan tipos físicos. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/datatype?utm_source=chatgpt.com))

Esto es muy útil para el libro.

## Primera implementación

```rust
enum Value {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
}
```

## Evolución inspirada en Ladybug

```rust
enum LogicalType {
    Boolean,
    Int32,
    Int64,
    Date,
    Timestamp,
    String,
    Node,
    Relationship,
}

enum PhysicalType {
    Bool,
    Int32,
    Int64,
    Float64,
    FixedBytes,
    VarBytes,
}
```

Después introduciríamos:

```rust
trait PhysicalEncoding {
    fn physical_type(&self) -> PhysicalType;
}
```

Esto permite enseñar:

- Semántica frente a representación.
- Casting.
- Comparadores físicos.
- Validación de expresiones.
- Optimización vectorizada.
- Evolución del formato.

---

# 3. Modelo de grafo estructurado

Ladybug no utiliza un Property Graph completamente libre. Emplea un **Structured Property Graph** basado en tablas de nodos y relaciones con esquema conocido. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

Ejemplo:

```text
NODE TABLE Person
─────────────────
id       INT64
name     STRING
age      INT32

REL TABLE Knows
─────────────────
from     Person
to       Person
since    DATE
```

Esto permite tratar nodos y relaciones como datos gráficos, pero también aplicar técnicas de bases de datos columnares.

## Evolución para LiraDB

### Modelo inicial

```rust
struct Node {
    labels: Vec<String>,
    properties: HashMap<String, Value>,
}
```

### Modelo estructurado

```rust
struct NodeTableSchema {
    table_id: TableId,
    name: String,
    primary_key: PropertyId,
    properties: Vec<PropertyDefinition>,
}

struct NodeRecord {
    table_id: TableId,
    offset: NodeOffset,
}
```

El lector aprende así la tensión entre:

```text
Flexibilidad          Rendimiento
HashMap dinámico  ↔   columnas tipadas
```

Podemos conservar ambos:

- Propiedades estructuradas para las columnas frecuentes.
- Una columna dinámica opcional para propiedades no declaradas.

---

# 4. Almacenamiento columnar

Ladybug almacena las propiedades de nodos en columnas, en lugar de guardar cada nodo como un registro completo. Las relaciones y sus propiedades también se organizan mediante estructuras columnares y listas de adyacencia. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## Primera versión del libro: almacenamiento por filas

```text
Node 1: [id, name, age, country]
Node 2: [id, name, age, country]
Node 3: [id, name, age, country]
```

Una consulta que solo necesita `name` debe leer todos los campos.

## Evolución columnar

```text
id column:      [1, 2, 3]
name column:    [Ana, Luis, Marta]
age column:     [34, 31, 42]
country column: [ES, ES, PT]
```

Consulta:

```text
MATCH (p:Person)
RETURN p.name
```

Solo necesita leer:

```text
name column
```

## Capítulo propuesto

**De filas a columnas: rediseñando el almacenamiento de LiraDB**

Contenido:

1. Implementar un row store.
2. Medir una proyección de una sola propiedad.
3. Construir un column store.
4. Comparar lecturas y memoria.
5. Añadir null masks.
6. Añadir columnas de longitud variable.
7. Seleccionar columnas durante la planificación.

---

# 5. `ValueVector` y ejecución vectorizada

Ladybug utiliza `ValueVector`, una estructura columnar en memoria que contiene fragmentos de valores del mismo tipo. Su capacidad predeterminada documentada es de 2048 elementos, elegida buscando que los datos procesados tengan buena relación con la caché de CPU. Cada vector contiene datos, máscara de nulos y almacenamiento auxiliar para strings o tipos anidados. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/vector/?utm_source=chatgpt.com))

## Versión manual inicial

```rust
for row in rows {
    if row.age > 18 {
        result.push(row.name);
    }
}
```

## Versión vectorizada

```rust
struct ValueVector<T> {
    values: Vec<T>,
    validity: BitMask,
    selection: SelectionVector,
}
```

```rust
fn greater_than_i32(
    input: &ValueVector<i32>,
    constant: i32,
    result: &mut SelectionVector,
);
```

El motor procesa:

```text
1024 o 2048 valores por bloque
```

en lugar de crear un objeto por fila.

## Conceptos extraíbles

- Procesamiento por lotes.
- Null masks.
- Selection vectors.
- Datos de tamaño fijo.
- Datos de longitud variable.
- Buffers auxiliares.
- Reutilización de memoria.
- Localidad de caché.
- Menos llamadas virtuales por registro.
- Posible vectorización SIMD.

Esta debería ser una evolución avanzada del motor Volcano inicial.

---

# 6. Adyacencias mediante CSR

Ladybug utiliza estructuras similares a **Compressed Sparse Row** para almacenar listas de adyacencia y utilizarlas también como índices de unión. Las relaciones se indexan tanto en dirección saliente como entrante. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## CSR básico

Para este grafo:

```text
0 → 1
0 → 2
1 → 2
2 → 0
```

podemos guardar:

```text
offsets = [0, 2, 3, 4, 4]
targets = [1, 2, 2, 0]
```

Vecinos de `0`:

```text
targets[offsets[0]..offsets[1]]
targets[0..2]
```

Resultado:

```text
[1, 2]
```

## Lección del libro

Primero:

```rust
Vec<Vec<Edge>>
```

Después:

```rust
struct Csr {
    offsets: Vec<u64>,
    targets: Vec<NodeId>,
}
```

Finalmente:

```rust
struct DirectedAdjacencyIndex {
    offsets: Column<u64>,
    targets: Column<NodeId>,
    relationship_ids: Column<RelationshipId>,
    properties: Vec<PropertyColumn>,
}
```

Compararíamos:

| Operación | Lista dinámica | CSR |
|---|---:|---:|
| Recorrer vecinos | Buena | Excelente |
| Inserción individual | Buena | Costosa |
| Borrado | Sencillo | Complejo |
| Localidad de memoria | Media | Alta |
| Analítica masiva | Aceptable | Muy buena |
| Actualizaciones constantes | Buena | Menos adecuada |

---

# 7. Índice doble de relaciones

Una decisión importante heredada de Kùzu es almacenar las relaciones para poder recorrerlas eficientemente en ambas direcciones:

```text
A ──KNOWS──> B
```

Debe encontrarse rápidamente tanto desde:

```text
outgoing(A)
```

como desde:

```text
incoming(B)
```

Conceptualmente:

```text
Forward adjacency index
A → [B]

Backward adjacency index
B → [A]
```

Esto aumenta el almacenamiento y el coste de las actualizaciones, pero evita reconstruir o escanear todas las relaciones durante recorridos inversos. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## Ejercicio excelente

Implementar tres versiones:

```text
1. Solo outgoing
2. Outgoing + búsqueda global para incoming
3. Outgoing + incoming indexado
```

Después medir:

```text
MATCH (a)<-[:DEPENDS_ON]-(b)
```

---

# 8. Node groups y particionado horizontal

La documentación interna de Ladybug describe los `NodeGroup` como particiones horizontales similares al concepto de `RowGroup`. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/?utm_source=chatgpt.com))

Aunque guardemos columnas, no conviene tener una única columna gigantesca indivisible:

```text
Person.name
├── NodeGroup 0: nodos 0–65.535
├── NodeGroup 1: nodos 65.536–131.071
└── NodeGroup 2: ...
```

Esto permite:

- Actualizar por segmentos.
- Cargar parcialmente datos.
- Aplicar MVCC por grupos.
- Paralelizar escaneos.
- Saltar bloques mediante estadísticas.
- Facilitar compactación.

## Evolución propuesta

```rust
struct NodeGroup {
    start_offset: NodeOffset,
    row_count: usize,
    columns: Vec<ColumnChunk>,
    statistics: Vec<ColumnStatistics>,
}
```

Este concepto conecta directamente con Parquet, DuckDB y otros motores analíticos.

---

# 9. Buffer manager con GClock

El diseño documentado de Kùzu utiliza páginas de tamaño fijo y una política de expulsión GClock para el buffer manager. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

En el libro podemos progresar:

```text
Sin caché
   ↓
LRU manual
   ↓
Clock
   ↓
Generalized Clock
```

## Clock sencillo

Cada frame mantiene un bit de referencia:

```rust
struct Frame {
    page_id: PageId,
    referenced: bool,
    pin_count: usize,
    dirty: bool,
}
```

## GClock

En lugar de un booleano, utiliza un contador:

```rust
struct Frame {
    usage_count: u8,
}
```

Al buscar víctima:

```text
usage_count > 0 → decrementar
usage_count = 0 → candidato a expulsión
```

Esto aproxima la frecuencia de acceso sin el coste de mantener una lista LRU exacta.

---

# 10. Factorización de resultados

Este es uno de los conceptos más valiosos y diferenciadores.

Supongamos:

```text
Ana conoce a Luis, Marta y Pedro.
Cada uno participa en cinco proyectos.
```

Una representación plana genera:

```text
Ana, Luis, Proyecto1
Ana, Luis, Proyecto2
Ana, Luis, Proyecto3
...
Ana, Marta, Proyecto1
...
```

Se repiten muchas veces `Ana` y cada persona.

Una representación factorizada puede guardar:

```text
Ana
 ├── Luis
 │    └── [Proyecto1, Proyecto2, Proyecto3, ...]
 ├── Marta
 │    └── [Proyecto1, Proyecto4, ...]
 └── Pedro
      └── [...]
```

Kùzu/Ladybug utiliza vectores factorizados para evitar repeticiones en resultados intermedios de joins muchos-a-muchos. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## Implementación didáctica

Primero:

```rust
struct FlatTuple {
    person: NodeId,
    friend: NodeId,
    project: NodeId,
}
```

Después:

```rust
struct FactorizedTuple {
    person: NodeId,
    friends: Vec<FactorizedFriend>,
}

struct FactorizedFriend {
    friend: NodeId,
    projects: Vec<NodeId>,
}
```

Y finalmente una representación genérica:

```rust
struct FactorizedTable {
    groups: Vec<DataChunk>,
    dependencies: FactorizationTree,
}
```

Este tema debería aparecer en el nivel avanzado, después de que el lector comprenda joins, operadores y ejecución por lotes.

---

# 11. Sideways Information Passing

Ladybug hereda de Kùzu el uso de **Sideways Information Passing**, normalmente abreviado SIP.

La idea es utilizar información obtenida en una parte del plan para reducir anticipadamente lo que otra parte debe escanear.

Ejemplo:

```text
MATCH (p:Person)-[:OWNS]->(s:Service)
WHERE p.name = "Ana"
RETURN s
```

Primero encontramos:

```text
Person Ana → NodeId 42
```

Podemos pasar ese conjunto de IDs al escaneo de relaciones para evitar leer relaciones que nunca podrán coincidir.

```text
Sin SIP:
escanear todas las relaciones OWNS

Con SIP:
escanear solo las listas asociadas a NodeId 42
```

El artículo técnico explica que esta información puede emplearse como filtro de semijoin para limitar las propiedades y aristas que se leen posteriormente. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## Implementación progresiva

1. Ejecutar ambos lados completamente.
2. Hash join convencional.
3. Crear un conjunto de IDs candidatos.
4. Pasarlo al operador de scan.
5. Añadir filtros bitmap.
6. Aplicarlo a índices de adyacencia.

---

# 12. ASP-Join

Kùzu introdujo el operador denominado **Accumulate–Semijoin–Probe Join**, o ASP-Join. Es una pieza central de su procesador y sirve también como base de sus joins multiway de peor caso óptimo. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

Se divide conceptualmente en tres fases:

```text
1. Accumulate
2. Semijoin
3. Probe
```

## 1. Accumulate

Se acumulan las tuplas factorizadas del lado de prueba.

## 2. Semijoin

Se extraen las claves que realmente pueden participar y se filtra el lado de construcción.

## 3. Probe

Se construye una tabla hash más pequeña y se ejecuta el join.

Esquema:

```text
Probe input
    ↓
Accumulate keys
    ↓
Semijoin filter
    ↓
Filtered build scan
    ↓
Hash table
    ↓
Probe
```

No intentaría implementar ASP-Join pronto. Antes enseñaríamos:

```text
Nested-loop join
      ↓
Index nested-loop join
      ↓
Hash join
      ↓
Semijoin reduction
      ↓
ASP-Join simplificado
```

Así el lector entiende qué problema resuelve.

---

# 13. Worst-case optimal joins

Las consultas cíclicas son especialmente importantes en grafos:

```text
MATCH (a)-[:KNOWS]->(b),
      (b)-[:KNOWS]->(c),
      (c)-[:KNOWS]->(a)
RETURN a, b, c
```

Buscamos triángulos.

Una secuencia tradicional de joins binarios puede producir una cantidad enorme de resultados intermedios:

```text
A ⋈ B
   ↓ resultado grande
(resultado) ⋈ C
```

Los **worst-case optimal joins** procesan varias relaciones de forma coordinada para evitar ciertos resultados intermedios explosivos.

Ladybug/Kùzu combina joins binarios y algoritmos multiway de peor caso óptimo, especialmente para patrones cíclicos. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

## Itinerario pedagógico

```text
Capítulo 1: detectar triángulos con tres bucles
Capítulo 2: detectar triángulos con intersección de vecinos
Capítulo 3: joins binarios
Capítulo 4: coste de resultados intermedios
Capítulo 5: Leapfrog Triejoin o algoritmo similar
Capítulo 6: selección de estrategia según patrón
```

No copiaríamos directamente el operador de Ladybug. Construiríamos una versión educativa basada en la literatura académica.

---

# 14. Optimización mediante programación dinámica

El artículo de Kùzu describe un optimizador de joins basado en programación dinámica, además de reglas como el pushdown de filtros. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

Para un patrón:

```text
A ──R1── B ──R2── C ──R3── D
```

existen varios órdenes:

```text
((A ⋈ B) ⋈ C) ⋈ D
(A ⋈ B) ⋈ (C ⋈ D)
A ⋈ (B ⋈ (C ⋈ D))
```

El optimizador estima:

```text
cardinalidad
selectividad
coste de scan
coste de expansión
coste de join
```

## Progresión del libro

1. Orden escrito por el usuario.
2. Reglas heurísticas.
3. Comenzar por la etiqueta más pequeña.
4. Comenzar por el predicado más selectivo.
5. Programación dinámica para patrones pequeños.
6. Límite de búsqueda para patrones grandes.
7. Comparación entre coste estimado y real.

---

# 15. Pipelines y operadores bloqueantes

Ladybug descompone el plan físico en pipelines lineales.

Un pipeline comienza en una fuente:

```text
Scan
```

y termina normalmente en un operador que necesita consumir toda su entrada:

```text
HashJoinBuild
Aggregate
OrderBy
```

Dentro del pipeline, los datos fluyen sin materializarse completamente. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/execution/?utm_source=chatgpt.com))

Ejemplo:

```text
Pipeline 1
NodeScan → Filter → HashJoinBuild

Pipeline 2
RelScan → HashJoinProbe → Project → ResultSink
```

Esto permite enseñar:

- Operadores streaming.
- Operadores bloqueantes.
- Materialización.
- Dependencias entre pipelines.
- Backpressure.
- Reutilización de buffers.

---

# 16. Paralelismo dirigido por morsels

Ladybug utiliza paralelismo **morsel-driven**: una fuente divide su entrada en fragmentos pequeños que diferentes workers pueden procesar. ([Ladybug](https://docs.ladybugdb.com/developer-guide/database-internal/execution/?utm_source=chatgpt.com))

Ejemplo:

```text
NodeGroup 0 ─┐
NodeGroup 1 ─┼──> worker pool
NodeGroup 2 ─┤
NodeGroup 3 ─┘
```

Cada worker ejecuta una copia local del pipeline:

```text
Scan → Filter → Expand → AggregateLocal
```

Después se combinan resultados:

```text
AggregateGlobal
```

Para Rust:

```rust
struct Morsel {
    start: usize,
    len: usize,
}

trait ParallelSource {
    fn next_morsel(&self) -> Option<Morsel>;
}
```

Primero puede implementarse con `std::thread` y canales, y más adelante compararse con crates maduros como Rayon.

---

# 17. WAL y modelo de un escritor

La arquitectura original de Kùzu utilizaba:

```text
Un escritor
Múltiples lectores concurrentes
WAL
Transacciones serializables
```

El WAL aporta atomicidad y durabilidad, mientras que limitar el sistema a un escritor simplifica considerablemente la corrección. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

Esto coincide muy bien con el alcance de LiraDB Lite.

En lugar de intentar MVCC multiwriter desde el principio:

```text
v0.8
un único escritor + lectores

v1.0
snapshots

v2.0
MVCC experimental
```

---

# 18. Algoritmos de grafos

Ladybug ofrece una extensión para ejecutar algoritmos sobre un **grafo proyectado**. Su documentación muestra PageRank como ejemplo y permite escribir el resultado nuevamente en las tablas de nodos. ([Ladybug](https://docs.ladybugdb.com/get-started/graph-algorithms/))

El concepto de proyección es importante:

```text
Base completa
├── Person
├── Team
├── Service
├── Repository
└── Technology

Proyección para PageRank
├── Service
└── DEPENDS_ON
```

En vez de ejecutar el algoritmo directamente sobre todo el almacenamiento transaccional, creamos una representación especializada:

```rust
struct ProjectedGraph {
    node_mapping: Vec<NodeId>,
    offsets: Vec<usize>,
    targets: Vec<ProjectedNodeId>,
}
```

Esto permite:

- Seleccionar etiquetas y relaciones.
- Convertir IDs dispersos en IDs densos.
- Construir CSR.
- Reducir memoria.
- Ejecutar algoritmos rápidamente.
- Descartar la proyección después.
- Escribir solo los resultados deseados.

## Algoritmos que encajan

```text
BFS
DFS
Caminos mínimos
Weakly connected components
Strongly connected components
PageRank
Betweenness
Closeness
Label propagation
Louvain
Triangle counting
K-core
Topological sort
Cycle detection
```

Conviene aclarar que algoritmos como PageRank no son una innovación propia de Ladybug. Lo valioso que extraemos es **cómo integrar algoritmos en una base de datos**:

```text
Consulta → proyección → algoritmo → tabla de resultados → persistencia opcional
```

---

# Cómo lo incorporaría al libro

Crearía una línea narrativa llamada:

## «El camino hacia una arquitectura como Ladybug»

Cada parte terminaría con tres apartados.

### 1. Nuestra implementación

```text
Qué hemos construido desde cero
```

### 2. El límite encontrado

```text
Qué ocurre al aumentar datos, conexiones o complejidad
```

### 3. Cómo lo resuelve Ladybug

```text
Qué concepto avanzado utiliza y por qué
```

No copiaríamos Ladybug en miniatura. Evolucionaríamos LiraDB mediante problemas reales.

---

# Matriz de adopción

| Concepto Ladybug | Nivel | Implementación en el libro |
|---|---|---|
| Structured Property Graph | Básico | Sí |
| Tablas de nodos y relaciones | Básico | Sí |
| Tipos lógicos y físicos | Intermedio | Sí |
| Relaciones forward/backward | Básico | Sí |
| CSR | Intermedio | Sí |
| Column store | Intermedio | Sí |
| Node groups | Intermedio | Sí |
| Hash index de claves | Básico | Sí |
| ART | Avanzado | Comparación |
| Buffer manager | Intermedio | Sí |
| GClock | Intermedio | Sí |
| WAL | Intermedio | Sí |
| Un escritor/múltiples lectores | Intermedio | Sí |
| Binder | Intermedio | Sí |
| Plan lógico y físico | Intermedio | Sí |
| Ejecución vectorizada | Avanzado | Sí |
| Factorized vectors | Experto | Versión educativa |
| SIP | Experto | Sí |
| ASP-Join | Experto | Simplificado |
| Worst-case optimal joins | Experto | Capítulo avanzado |
| Optimización por programación dinámica | Experto | Patrones pequeños |
| Pipelines | Avanzado | Sí |
| Morsel-driven parallelism | Experto | Versión reducida |
| PageRank y comunidades | Avanzado | Sí |
| Grafo proyectado | Avanzado | Sí |

---

# Lo que no deberíamos copiar directamente

No recomendaría trasladar línea por línea el motor C++ a Rust.

Eso produciría:

- Una traducción difícil de explicar.
- Abstracciones diseñadas para otro lenguaje.
- Mucho código accidental.
- Dependencia conceptual excesiva.
- Riesgo de conservar decisiones que no encajan con Rust.
- Un libro sobre Ladybug, no sobre fundamentos de bases de datos.

Usaría este proceso:

```text
Código o artículo de Ladybug
          ↓
Identificar el problema
          ↓
Extraer la idea y sus invariantes
          ↓
Diseñar una versión mínima en Rust
          ↓
Implementarla sin mirar el código
          ↓
Probarla
          ↓
Comparar comportamiento y arquitectura
```

Esto se parece a una reimplementación **clean-room conceptual**.

---

# Licencia y atribución

El repositorio de Ladybug utiliza licencia MIT, por lo que permite uso, modificación y redistribución, manteniendo el aviso de copyright y la licencia cuando se reutiliza código. ([GitHub](https://github.com/LadybugDB/ladybug))

El artículo de Kùzu presentado en CIDR 2023 está publicado bajo CC BY 4.0, por lo que sus diagramas o contenido pueden adaptarse con la atribución correspondiente. ([cidrdb.org](https://www.cidrdb.org/cidr2023/papers/p48-jin.pdf))

Para el libro aplicaría esta política:

```text
Concepto o algoritmo:
explicación propia + cita académica

Código inspirado en la idea:
implementación original en Rust + referencia

Código adaptado directamente:
indicación explícita + aviso MIT

Figura adaptada:
“Adaptada de…” + referencia CC BY 4.0
```

La solución más limpia sería que casi todo el código del libro fuera original y que Ladybug/Kùzu apareciera como referencia arquitectónica y académica.

# Nueva promesa técnica del libro

El resultado podría describirse así:

> Construiremos una base de datos de grafos desde cero y, una vez comprendidos sus fundamentos, la evolucionaremos siguiendo ideas contrastadas por motores de investigación y producción como Ladybug: almacenamiento columnar, adyacencias CSR, ejecución vectorizada y factorizada, optimización de patrones y procesamiento paralelo.

Ladybug no debería ser solo un ejemplo mencionado al final. Puede convertirse en el **modelo avanzado contra el que evoluciona LiraDB**, dejando muy clara la separación entre una implementación educativa y un motor analítico de altas prestaciones.

---
Powered by [AI Exporter](https://saveai.net)