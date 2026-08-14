# Capítulo 21 — Grafos en Bases de Datos

¿Alguna vez has mirado un `SELECT` con cinco `JOIN` anidados y has sentido que el código te devolvía la mirada? No estás solo. Hay tres tipos de desarrolladores: los que escriben `JOIN`s y los entienden, los que los escriben y rezan, y los que se pasaron a las bases de datos de grafos y ya no quieren volver. Bienvenido al club.

En este capítulo vamos a hacer algo modesto pero potente: demostrar que un `JOIN` no es más que un producto cartesiano con filtro (spoiler: eso ya es un grafo de facto), y luego aprenderemos un lenguaje de queries de grafos — Cypher — que hace lo mismo pero sin el dolor lumbar. Y todo con código Rust real.

## 21.0 La anécdota del matemático que se hartó de los archivos jerárquicos

Estamos a finales de los 60. Edgar F. Codd, un matemático británico trabajando en IBM San José, estaba harto. Los sistemas de gestión de datos de la época eran un caos: archivos planos, índices propietarios, bases jerárquicas (como IMS de IBM) o en red (Codasyl). Programar una consulta decente era como hacer malabares con cuatro antorchas y los ojos cerrados.

Codd publicó *«A Relational Model of Data for Large Shared Data Banks»* en 1970, un paper de 11 páginas que cambió la informática para siempre. Su idea: los datos viven en **tablas** (que él llamó *relations*), y las consultas se expresan con álgebra relacional. El resto es historia: SQL nació en los 70, Oracle y DB2 en los 80, MySQL y PostgreSQL en los 90. Las tablas dominaron el mundo durante 40 años.

Pero las tablas tienen un punto débil: modelar **relaciones** entre entidades. Una amistad entre dos personas, una compra de un cliente, una proteína que interactúa con otra… Si metes todo en tablas, acabas con 12 `JOIN`s. Los grafos, en cambio, son nativos para esto. Por eso, en la década de los 2010, varios proyectos recuperaron la idea de **bases de datos de grafos** (graph databases): Neo4j (con Cypher), ArangoDB (multi-modelo), JanusGraph (distribuida, ex-Titan). Codd, irónicamente, ya lo había avisado: las relaciones son ciudadanos de primera, no accesorios. Neo4j simplemente se lo tomó en serio.

## 21.1 SQL JOIN, desnudo: producto cartesiano + filtro

Hay un mito urbano: que un `JOIN` es "mágico". No. Un `JOIN` es, literalmente, esto:

```
1. Tomar TODAS las combinaciones de filas (producto cartesiano).
2. Aplicar un predicado (ON ... = ...).
3. Filtrar con WHERE.
4. Proyectar con SELECT.
```

Vamos a verlo con un ejemplo. Imagina dos tablas mínimas:

```
personas                   amistades
┌────┬───────┐            ┌──────────┬──────────┐
│ id │ nom   │            │ pers_a   │ pers_b   │
├────┼───────┤            ├──────────┼──────────┤
│ 1  │ Ana   │            │ 1        │ 2        │
│ 2  │ Beto  │            │ 1        │ 3        │
│ 3  │ Clara │            │ 2        │ 3        │
└────┴───────┘            └──────────┴──────────┘
```

El query `SELECT * FROM personas JOIN amistades ON personas.id = amistades.pers_a` da, paso a paso:

```
Paso 1: producto cartesiano
Ana,Ana  |  Ana,Beto  |  Ana,Clara
Beto,Ana |  Beto,Beto |  Beto,Clara
Clara,Ana|  Clara,Beto|  Clara,Clara

Paso 2: filtro id = pers_a
Ana,Ana    (id=1, pers_a=1) ✓
Ana,Beto   (id=1, pers_a=2) ✗
Ana,Clara  (id=1, pers_a=3) ✗
Beto,Ana   (id=2, pers_a=1) ✗
... (solo quedan 3 filas)
```

Observa: ese "filtro" no es más que **emparejar dos conjuntos por una clave**, que es exactamente lo que hace un **matching de aristas** en un grafo bipartito. Si lo dibujas, el `JOIN` es un grafo bipartito con `WHERE` como etiquetado.

```
       personas                amistades
      ┌───┐                   ┌────────┐
      │ 1 │ ───────────────► │ (1,2)  │
      │ 2 │ ───┐             │ (1,3)  │
      │ 3 │ ───┼───────────► │ (2,3)  │
      └───┘     │             └────────┘
                └─► (1,2), (1,3), (2,3)
```

Y cuando encadenas tres `JOIN`s, el grafo se vuelve más denso: una `traversal` (recorrido) por un grafo donde cada tabla es un "tipo de nodo" y cada clave foránea es una arista. Cuando dibujas las cinco tablas con las líneas, te das cuenta: **ya estabas pensando en grafos sin saberlo**. Solo te faltaba el lenguaje.

## 21.2 Modelo relacional vs modelo de grafos: ¿cuándo gana cada uno?

Las dos cosas. Una frase que se tatúan los que llevan años modelando: **el martillo no le tiene miedo al destornillador**.

| Escenario | Campeón | Por qué |
|---|---|---|
| Datos tabulares, agregaciones, BI clásico | **Relacional** | SQL está hiper-optimizado (CBO, índices, paralelismo). |
| Muchas relaciones N:M, profundidad variable | **Grafo** | El `JOIN` cascada explota; el recorrido nativo no. |
| Transacciones ACID estrictas | **Relacional** (todavía) | Ecosistema maduro. |
| Datos cambiantes, esquema flexible | **Grafo / documental** | Menos migraciones dolorosas. |
| Knowledge graphs, redes sociales | **Grafo** | Es su terreno natural. |
| Inventario, contabilidad, banca | **Relacional** | Necesitas joins duros y constraints. |

La regla de oro que yo uso: si mi query tiene más de 4 `JOIN`s consecutivos, me detengo y me pregunto si no debería ser un grafo. Y la inversa también: si solo tengo dos tablas y dos `JOIN`s, no merece la pena montar Neo4j, abro SQLite.

## 21.3 Las tres bases de datos de grafos que importan

- **Neo4j**: la más popular, madura, con Cypher como lenguaje. Modelo de **property graph**: nodos y aristas con propiedades (clave-valor). Muy querida por startups y equipos de datos.
- **ArangoDB**: multi-modelo (documento + grafo + clave-valor). Si ya tienes un sistema políglota y no quieres otra pieza, esta es tu amiga. Usa AQL (parecido a SQL).
- **JanusGraph**: la "Linux de los grafos". Distribuida, open-source, pensada para grafos enormes. Usa Apache TinkerPop por debajo (lenguaje Gremlin). Es para cuando Neo4j se te queda corto y necesitas escalabilidad horizontal.

Hay otras (TigerGraph, Memgraph, Amazon Neptune), pero si entiendes las tres primeras, las demás son variaciones sobre el mismo tema.

## 21.4 Cypher: el SQL de los grafos

Neo4j inventó Cypher, y la idea es brillante: **dibujar el patrón que quieres encontrar**. La sintaxis usa paréntesis para nodos y corchetes para aristas, y unas flechas ASCII (--> o <--) que parecen un diagrama.

Ejemplo: amigos de amigos de Ana.

```cypher
MATCH (ana:Persona {nombre: 'Ana'})-[:AMIGO_DE]->()-[:AMIGO_DE]->(fof)
RETURN DISTINCT fof.nombre
```

Eso es todo. Léelo en voz alta: "Busca un patrón donde Ana sea AMIGO_DE alguien, y ese alguien sea AMIGO_DE otra persona (fof). Devuélveme los nombres distintos." Si en vez de dos saltos quieres tres, añades otro `()-[:AMIGO_DE]->()`. Si quieres un camino de cualquier longitud: `-[*1..5]->`.

```
     ┌─── AMIGO_DE ───►┐
     │                  ▼
   Ana ◄── AMIGO_DE ─── Beto ◄── AMIGO_DE ─── Clara
     │                                               
     └─── AMIGO_DE ───► Diego ── AMIGO_DE ──► Eva
```

Encontrar "amigos de amigos de Ana" en SQL serían 4 `JOIN`s (o una subquery recursiva en PostgreSQL). En Cypher, una línea.

## 21.5 Un mini-query engine en Rust con petgraph

No vamos a montar un Neo4j casero (¡ojalá!), pero sí un mini-motor que entienda tres queries tipo Cypher. Esto demuestra el patrón fundamental: **un grafo + un matcher de patrones**. Usaremos `petgraph`, que ya conoces.

```toml
[dependencies]
petgraph = "0.6"
```

```rust
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Undirected;
use std::collections::HashMap;

/// Property graph casero: cada nodo y arista tiene un dict de propiedades.
type Props = HashMap<String, String>;

pub fn build_demo() -> (Graph<Props, Props, Undirected>, HashMap<&'static str, NodeIndex>) {
    let mut g: Graph<Props, Props, Undirected> = Graph::new_undirected();
    let ana = g.add_node([("nombre", "Ana"), ("edad", "30")].iter().cloned().collect());
    let beto = g.add_node([("nombre", "Beto"), ("edad", "28")].iter().cloned().collect());
    let clara = g.add_node([("nombre", "Clara"), ("edad", "32")].iter().cloned().collect());
    let diego = g.add_node([("nombre", "Diego"), ("edad", "35")].iter().cloned().collect());

    g.add_edge(ana, beto, [("tipo", "AMIGO_DE"), ("desde", "2018")].iter().cloned().collect());
    g.add_edge(ana, clara, [("tipo", "AMIGO_DE"), ("desde", "2020")].iter().cloned().collect());
    g.add_edge(beto, clara, [("tipo", "AMIGO_DE"), ("desde", "2015")].iter().cloned().collect());
    g.add_edge(beto, diego, [("tipo", "AMIGO_DE"), ("desde", "2019")].iter().cloned().collect());

    let mut idx = HashMap::new();
    idx.insert("Ana", ana);
    idx.insert("Beto", beto);
    idx.insert("Clara", clara);
    idx.insert("Diego", diego);
    (g, idx)
}

/// Query 1: MATCH (a)-[:AMIGO_DE]->(b) WHERE a.nombre = X RETURN b.nombre
pub fn amigos_de(g: &Graph<Props, Props, Undirected>, nombre: &str) -> Vec<String> {
    let mut out = Vec::new();
    for edge in g.edge_references() {
        let (a, b) = (edge.source(), edge.target());
        if edge.weight().get("tipo").map(|s| s.as_str()) == Some("AMIGO_DE") {
            for n in [a, b] {
                if g[n].get("nombre").map(|s| s.as_str()) == Some(nombre) {
                    let otro = if n == a { b } else { a };
                    if let Some(otro_nom) = g[otro].get("nombre") {
                        out.push(otro_nom.clone());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Query 2: amigos de amigos (longitud 2)
pub fn amigos_de_amigos(g: &Graph<Props, Props, Undirected>, nombre: &str) -> Vec<String> {
    let mut out = Vec::new();
    let directos = amigos_de(g, nombre);
    for d in &directos {
        for amigo in amigos_de(g, d) {
            if amigo != nombre && !directos.contains(&amigo) {
                out.push(amigo);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Query 3: shortest path entre dos nodos (BFS)
pub fn camino_mas_corto(
    g: &Graph<Props, Props, Undirected>,
    desde: NodeIndex,
    hasta: NodeIndex,
) -> Option<usize> {
    use std::collections::VecDeque;
    let mut q = VecDeque::new();
    let mut dist = HashMap::new();
    q.push_back(desde);
    dist.insert(desde, 0);
    while let Some(v) = q.pop_front() {
        if v == hasta { return dist.get(&v).copied(); }
        for w in g.neighbors(v) {
            if !dist.contains_key(&w) {
                dist.insert(w, dist[&v] + 1);
                q.push_back(w);
            }
        }
    }
    None
}

fn main() {
    let (g, idx) = build_demo();
    println!("Amigos de Ana: {:?}", amigos_de(&g, "Ana"));
    println!("Amigos de amigos de Ana: {:?}", amigos_de_amigos(&g, "Ana"));
    println!("Camino Ana → Diego: {} saltos", 
        camino_mas_corto(&g, idx["Ana"], idx["Diego"]).unwrap());
}
```

Salida esperada:
```
Amigos de Ana: ["Beto", "Clara"]
Amigos de amigos de Ana: ["Diego"]
Camino Ana → Diego: 2 saltos
```

¿Ves? Tres queries, todas con un patrón Cypher-mental, y todas implementadas como `match` sobre aristas + BFS. Esa es la idea: el lenguaje cambia, el grafo es el mismo.

## 21.6 Diálogo de pasillo

> — Oye, Carla, ¿por qué mi `MATCH` en Neo4j tarda tres segundos y el `JOIN` de SQL tarda trescientos milisegundos?
> — Porque tu `MATCH` recorre 4 millones de relaciones y no tienes índice en `Persona.id`. Es como preguntar "¿cuántos amigos de amigos tiene Ana?" y no saber ni siquiera dónde vive Ana.
> — Vale, ¿puedo meterle un índice?
> — Sí, con `CREATE INDEX ON :Persona(nombre)`. Pero más profundo: aprende a leer el `EXPLAIN`. Neo4j te dice si está haciendo `NodeByLabelScan` (malo) o `NodeIndexSeek` (bueno).
> — Como cuando en SQL miras el plan de ejecución.
> — Exacto. Los grafos no te libran de pensar, solo te cambian **en qué** piensas.

## 21.7 Aplicaciones del mundo real

- **Redes sociales**: Twitter/X, LinkedIn, Facebook. "Personas que quizás conozcas" = amigos de amigos + ponderación.
- **Recomendaciones**: Amazon, Netflix. "Compraste X, otros que compraron X también compraron Y" = un grafo bipartito producto-cliente.
- **Knowledge graphs**: el Knowledge Graph de Google, Wikidata, ConceptNet. Entidades y relaciones, consultados por buscadores y chatbots.
- **Detección de fraude**: redes de transacciones sospechosas. Encuentras ciclos sospechosos (anillo de tarjetas) que un `JOIN` en cascada tardaría horas en revelar.
- **Gestión de identidades y permisos**: en una empresa, modelar quién-puede-acceder-a-qué como un grafo de roles y recursos.

## 21.8 Ejercicios resueltos

**Ejercicio 21.1.** Dado el grafo del §21.5, escribe una query Cypher que devuelva los nombres de los amigos de Ana, ordenados por edad descendente. Pista: en Cypher sería `ORDER BY b.edad DESC`.

*Solución (mental, en Cypher):*
```cypher
MATCH (ana:Persona {nombre: 'Ana'})-[:AMIGO_DE]->(b)
RETURN b.nombre, b.edad
ORDER BY b.edad DESC
```
Equivalente en Rust: extender `amigos_de` para que devuelva tuplas `(nombre, edad)`, y ordenar con `.sort_by_key(|x| -x.1)`.

**Ejercicio 21.2.** Implementa una query "amigos en común" (intersección de la lista de amigos de X y de Y).

*Solución:*
```rust
pub fn amigos_en_comun(g: &Graph<Props, Props, Undirected>, x: &str, y: &str) -> Vec<String> {
    let a: std::collections::HashSet<_> = amigos_de(g, x).into_iter().collect();
    let b: std::collections::HashSet<_> = amigos_de(g, y).into_iter().collect();
    a.intersection(&b).cloned().collect()
}
```

**Ejercicio 21.3.** Explica por qué un `JOIN` con cuatro tablas (A, B, C, D) podría recorrer 10⁸ filas en una base relacional, mientras que un grafo haría lo mismo en O(aristas · log n) con índices.

*Solución:* El optimizador relacional (CBO) elige un orden de JOINs basándose en estadísticas. Si las cardinalidades son altas, el plan se vuelve subóptimo y explota. En un grafo, el recorrido (traversal) usa índices de adyacencia, así que saltar de un nodo a sus vecinos es O(1) por arista, no O(n) por tabla. La diferencia es brutal cuando los datos son sparse pero muy conectados (lo típico en redes sociales).

## 21.9 Ejercicios propuestos

1. Implementa una variante de `amigos_de_amigos` que acepte un parámetro de profundidad `k` (1, 2, 3, …).
2. Dado un grafo dirigido de personas y "sigue_a" (Twitter), escribe una query que liste los seguidos de Ana que **no** la siguen de vuelta (asimetría).
3. Modela una librería como grafo: nodos `Libro` y `Autor`, aristas `ESCRIBIO`. Escribe una Cypher query para "todos los libros co-escritos por al menos dos autores".
4. Crea una función que detecte **triángulos** (cliques de 3) en el grafo del §21.5. Útil para detectar comunidades pequeñas.
5. Implementa PageRank simple sobre el grafo. Pista: 10 iteraciones de `(PR(v) = (1-d)/n + d · Σ PR(u)/outdeg(u))` para cada u que apunta a v.

## 21.10 Pin de batalla

- Si tu query Cypher tarda más de 100ms, **primero mira el `EXPLAIN`, luego mira el modelo**. Un mal modelo de grafos es peor que una tabla SQL.
- No metas TODO en un grafo. Las propiedades grandes (logs, blobs, JSONs enormes) son de bases documentales o relacionales. El grafo es para **conexiones**.
- Cuidado con los **super-nodos**: un nodo con 100.000 aristas hará sufrir cualquier recorrido. A veces hay que "romperlo" (nodo a un lado, aristas a otro) o usar sub-grafos.
- **Las transacciones en Neo4j son ACID**, pero las queries de lectura en grafos distribuidos (JanusGraph, Neptune) son eventualmente consistentes. No asumas lo que no es.
- **No uses grafos cuando la cardinalidad importa y la profundidad no**: un carrito de la compra, una factura, un inventario tabular. El martillo relacional es excelente para eso.

## 21.11 Lo que te llevas

Un `JOIN` no es magia, es un producto cartesiano filtrado, que ya es un matching de aristas en un grafo bipartito. Las bases de datos de grafos (Neo4j, ArangoDB, JanusGraph) externalizan ese matching con un lenguaje (Cypher, AQL, Gremlin) que **dibuja el patrón que quieres**. Para datos muy relacionados (redes sociales, knowledge graphs, fraude), el grafo gana por goleada. Para datos tabulares y transacciones ACID pesadas, el relacional sigue siendo el rey.

## 21.12 Ojo, cuidado con…

- **Modelar todo como grafo "porque mola"**. He visto proyectos de bases de datos de grafos con datos que claramente eran tabulares. El resultado: queries lentas y un modelo imposible de mantener.
- **Cypher y SQL no son excluyentes**. Muchas arquitecturas modernas usan los dos: Postgres para datos transaccionales, Neo4j para recomendaciones. Se llaman **polyglot persistence**.
- **Las "bases de datos de grafos" no son una panacea de performance**. Sin índices, sin modelo, sin pensar, son lentas. Como todo.

## 21.13 Para profundizar

- *"Graph Databases"* de Ian Robinson, Jim Webber y Emil Eifrem (los creadores de Neo4j). El libro introductorio por excelencia.
- Documentación oficial de Neo4j: https://neo4j.com/docs/cypher-manual/current/
- El libro *"Designing Data-Intensive Applications"* de Martin Kleppmann, capítulo sobre modelos de datos (especialmente la comparación relacional vs documental vs grafo).
- *Apache TinkerPop*: documentación de Gremlin, otro lenguaje de queries de grafos.
- *"Seven Databases in Seven Weeks"* de Eric Redmond y Jim Wilson: tiene un capítulo brillante sobre Neo4j.

## 21.14 Si solo lees 30 segundos

Un `JOIN` SQL es un producto cartesiano con filtro, que es exactamente un matching de aristas en un grafo. Las bases de datos de grafos como Neo4j te dan un lenguaje (Cypher) que dibuja los patrones en vez de escribir joins. Úsalo cuando los datos son muy relacionales y los `JOIN`s empiezan a doler. No lo uses para todo.

## 21.15 Una historia pequeña

Marta, junior en un equipo de 4 personas, heredó un módulo de recomendaciones que hacía 6 `JOIN`s en cascada sobre una tabla de "usuarios", "vistos", "comprados", "categorías", "similares" y "productos". El query tardaba 14 segundos en producción. Ella sabía poco de grafos, pero leyó un párrafo sobre Neo4j, montó un proof-of-concept en una tarde, y descubrió que un `MATCH (u)-[:VIO]->(:Producto)-[:EN_CAT]->(c) <-[:EN_CAT]-(:Producto)<-[:COMPRO]-(u2)` hacía lo mismo en 80 milisegundos. Cuando lo presentó al equipo, el senior le dijo: "el martillo no le tiene miedo al destornillador". Ella respondió: "no, pero cuando el tornillo está oxidado, mejor usar un destornillador."

---

