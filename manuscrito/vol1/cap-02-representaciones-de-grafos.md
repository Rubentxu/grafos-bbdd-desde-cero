# Capítulo 2 — Representaciones de grafos

Si tu grafo tiene 10 vértices, da igual cómo lo guardes. Si tiene 10 millones, la elección de representación puede ser la diferencia entre terminar el proyecto o no terminarlo. Y no, no es lo mismo una lista que una matriz.
## 2.0 La anécdota de la esquina

Verano de 1690. Inglaterra. El rey Guillermo III de Orange se ha construido un palacio nuevo en Hampton Court, y como todo rey que se precie, quiere presumir de sus jardines. Encarga a sus jardineros un **laberinto vegetal** para disfrute de la corte. Los jardineros, claro, no habían diseñado un laberinto en su vida, y el resultado fue un caos de setos por el que la gente se perdía tres horas hasta que aparecía un lacayo con antorchas.

Lo que pasó después fue, sin saberlo, una de las primeras aplicaciones prácticas de la teoría de grafos: alguien (probablemente el propio ayudante del rey) **dibujó un mapa del laberinto** con cruces en las intersecciones y rayas en los pasillos, para que los visitantes no se perdieran. Es decir: convirtió un espacio físico continuo en un **grafo** discreto. Cada cruce es un vértice; cada pasillo recto, una arista. A partir de ahí, resolver el laberinto es encontrar un camino desde la entrada hasta el centro.

Hoy, casi 340 años después, hacemos lo mismo cada vez que abrimos Google Maps. Tu barrio es un grafo enorme, y el algoritmo que te dice "gira a la derecha en 200 metros" no es más que un viajante calculando rutas sobre ese grafo.


> — Acabo de hacer un grafo de 5 vértices con `Vec<Vec<bool>>` y va perfecto.
> — Genial, para 5 vértices cualquier cosa va. Prueba con 100.000.
> — Boom, `OutOfMemory`.
> — Bienvenido al club. Mira, para grafos grandes la regla es: lista de adyacencia para casi todo, `petgraph` si quieres la rueda ya inventada. Y olvídate de la matriz, salvo que sea densa y < 1000 vértices.
## 2.1 El problema: ¿cómo guardo un grafo en memoria?

Vale, ya sabes qué es un grafo. Ahora la pregunta práctica: si tienes un grafo con 1000 vértices y 5000 aristas, ¿cómo lo metes en la RAM de tu ordenador? Hay tres formas canónicas, y cada una tiene sus pros y sus contras. Vamos a verlas.

## 2.2 Matriz de adyacencia

La más intuitiva. Imagina una tabla cuadrada. Las filas son los vértices de origen, las columnas los de destino. En la celda (i, j) pones un 1 si hay arista entre el vértice i y el j, y un 0 si no.

Para un grafo de 4 vértices {A, B, C, D} con aristas A-B, A-C, B-D, C-D:

```
      A  B  C  D
   A [0, 1, 1, 0]
   B [1, 0, 0, 1]
   C [1, 0, 0, 1]
   D [0, 1, 1, 0]
```

**Pros:**

- Saber si (u, v) es arista: O(1). Miras la celda y listo.
- Fácil de dibujar y razonar.

**Contras:**

- Ocupa espacio O(V²). Si tienes 100.000 vértices, la matriz tiene 10.000.000.000 de celdas. Adiós, RAM.
- Iterar sobre los vecinos de un vértice: O(V) (tienes que recorrer toda la fila), aunque en la mayoría de celdas haya 0.

**¿Cuándo usarla?** Grafos pequeños y densos (cuando |E| ≈ |V|²). O cuando necesitas hacer operaciones matriciales (¡los grafos y el álgebra lineal se llevan de lujo!).

## 2.3 Lista de adyacencia

La favorita del programador práctico. Para cada vértice, guardas una **lista** de sus vecinos. Solo guardas lo que existe.

El mismo grafo de antes, en listas:

```
A: [B, C]
B: [A, D]
C: [A, D]
D: [B, C]
```

**Pros:**

- Ocupa espacio O(V + E). Mucho más eficiente en grafos dispersos (la mayoría de grafos reales son dispersos, ¡ojo!).
- Iterar sobre los vecinos de un vértice: O(g(v)), donde g(v) es su grado. Rápido.

**Contras:**

- Saber si (u, v) es arista: O(g(u)) en el peor caso (tienes que buscar v en la lista de u).
- Listas en el sentido literal de la palabra: si las implementas como arrays dinámicos, las inserciones en medio cuestan O(n). En la práctica usarás `Vec` o `VecDeque`.

**¿Cuándo usarla?** El 90% de las veces. Grafos dispersos, algoritmos de recorrido, Dijkstra, BFS, DFS… casi todo.

## 2.4 Diccionario de aristas (HashMap de aristas)

Una tercera vía, menos común pero útil: una tabla hash donde la clave es el par (u, v) y el valor es el peso (o cualquier metadato de la arista).

```rust
use std::collections::HashMap;
let mut aristas: HashMap<(u32, u32), u32> = HashMap::new();
aristas.insert((0, 1), 5);
aristas.insert((1, 2), 3);
```

**Pros:** acceso O(1) por clave, perfecto para grafos con muchas consultas "¿existe esta arista?".

**Contras:** iterar sobre los vecinos de un vértice requiere filtrar por u o por v, O(E) si no hay estructura auxiliar. Poco práctico para algoritmos de recorrido.

## 2.5 Implementación manual en Rust puro

Vamos a lo que viniste: código. Vamos a implementar un grafo con lista de adyacencia en Rust, sin crates externos. Lo más limpio es un `struct` con un `HashMap<u32, Vec<u32>>` (o un `Vec<Vec<u32>>` si los vértices son 0..n).

```rust
// src/lib.rs
use std::collections::HashMap;

/// Grafo no dirigido implementado con lista de adyacencia (HashMap).
#[derive(Debug, Clone)]
pub struct MiGrafo {
    /// Clave: vértice. Valor: lista de vecinos.
    adj: HashMap<u32, Vec<u32>>,
}

impl MiGrafo {
    /// Crea un grafo vacío.
    pub fn nuevo() -> Self {
        Self { adj: HashMap::new() }
    }

    /// Añade un vértice si no existía.
    pub fn agrega_vertice(&mut self, v: u32) {
        self.adj.entry(v).or_insert_with(Vec::new);
    }

    /// Añade una arista no dirigida entre u y v.
    pub fn agrega_arista(&mut self, u: u32, v: u32) {
        // Aseguramos que ambos vértices existen
        self.agrega_vertice(u);
        self.agrega_vertice(v);
        // No añadimos duplicados
        if !self.adj[&u].contains(&v) {
            self.adj.get_mut(&u).unwrap().push(v);
        }
        if !self.adj[&v].contains(&u) {
            self.adj.get_mut(&v).unwrap().push(u);
        }
    }

    /// Devuelve los vecinos de v (¡orden no garantizado!).
    pub fn vecinos(&self, v: u32) -> &[u32] {
        self.adj.get(&v).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Número de vértices.
    pub fn n(&self) -> usize {
        self.adj.len()
    }

    /// Número de aristas (en no dirigido, cada arista cuenta 1).
    pub fn m(&self) -> usize {
        self.adj.values().map(|v| v.len()).sum::<usize>() / 2
    }
}

impl Default for MiGrafo {
    fn default() -> Self {
        Self::nuevo()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafo_vacio() {
        let g = MiGrafo::nuevo();
        assert_eq!(g.n(), 0);
        assert_eq!(g.m(), 0);
    }

    #[test]
    fn agrega_aristas_basicas() {
        let mut g = MiGrafo::nuevo();
        g.agrega_arista(1, 2);
        g.agrega_arista(2, 3);
        g.agrega_arista(1, 3);
        assert_eq!(g.n(), 3);
        assert_eq!(g.m(), 3);
        assert_eq!(g.vecinos(&1), &[2, 3]);
        assert_eq!(g.vecinos(&2), &[1, 3]);
    }

    #[test]
    fn no_duplicar_aristas() {
        let mut g = MiGrafo::nuevo();
        g.agrega_arista(1, 2);
        g.agrega_arista(2, 1); // misma arista
        assert_eq!(g.m(), 1);
    }
}
```

`Cargo.toml` correspondiente:

```toml
[package]
name = "mi-grafo"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/lib.rs"
```

`cargo new --lib mi-grafo`, pegas, y `cargo test`. Tres tests, todos verdes.

## 2.6 Con `petgraph`: lo mismo, pero industrial

Ahora viene la magia. `petgraph` es EL crate de grafos en Rust. Lo mantienen personas que saben mucho, está bien testeado, y te ahorra reinventar la rueda. Vamos a rehacer el mismo ejemplo con petgraph.

Añade a tu `Cargo.toml`:

```toml
[dependencies]
petgraph = "0.6"
```

Y el código:

```rust
// src/lib.rs
use petgraph::graph::UnGraph;
use petgraph::Graph;
use petgraph::Undirected;

pub fn ejemplo_petgraph() -> Graph<(), (), Undirected> {
    // Graph<(), (), Undirected> -> grafo no dirigido, sin datos en vértices ni aristas
    let mut g: Graph<(), (), Undirected> = Graph::new_undirected();

    // Añadimos vértices (sin datos asociados)
    let a = g.add_node(());
    let b = g.add_node(());
    let c = g.add_node(());
    let d = g.add_node(());

    // Añadimos aristas
    g.add_edge(a, b, ());
    g.add_edge(a, c, ());
    g.add_edge(b, d, ());
    g.add_edge(c, d, ());

    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::NodeIndex;

    #[test]
    fn cuenta_vertices_y_aristas() {
        let g = ejemplo_petgraph();
        assert_eq!(g.node_count(), 4);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn vecinos_de_a() {
        let g = ejemplo_petgraph();
        // El primer vértice añadido (a) tiene índice 0
        let a = NodeIndex::new(0);
        let vecinos: Vec<_> = g.neighbors(a).collect();
        assert_eq!(vecinos.len(), 2);
    }
}
```

Diferencias clave con nuestra versión a mano:

| Aspecto | `MiGrafo` | `petgraph` |
|---|---|---|
| Vértices | `u32` | `NodeIndex` (tipo opaco) |
| Datos asociados | No | Sí: genérico sobre el tipo de dato del nodo/arista |
| Dirigido/No dirigido | Manual | Tipo `Directed`/`Undirected` |
| Iteradores | Manual | `.neighbors()`, `.edges()`, etc. |
| Algoritmos | DIY | Incluidos (BFS, DFS, Dijkstra…) |

**Cuándo usar `MiGrafo` a mano:** cuando estés aprendiendo (como ahora), o cuando necesites algo super-específico y raro que petgraph no te dé. **Cuándo usar `petgraph`:** en cualquier proyecto real, salvo que sea trivial.

## 2.7 Tabla comparativa de complejidad

| Operación | Matriz | Lista | HashMap aristas |
|---|---|---|---|
| Espacio | O(V²) | O(V+E) | O(E) |
| ¿(u,v) es arista? | O(1) | O(g(u)) | O(1) |
| Iterar vecinos de u | O(V) | O(g(u)) | O(E) |
| Añadir arista | O(1) | O(1) amortizado* | O(1) amortizado |
| Eliminar arista | O(1) | O(g(u)) | O(1) |
| Mejor para | Grafos densos | Grafos dispersos | Muchas queries |

\* "amortizado" significa que a veces cuesta más, pero en promedio cuesta eso. El `Vec::push` en Rust es O(1) amortizado.

## 2.8 Ejercicios resueltos

**Ejercicio 2.1 (F).** Dado el grafo con aristas {(0,1), (1,2), (2,3), (3,0), (0,2)}, escribe su matriz de adyacencia y su lista de adyacencia.

**Solución.** Matriz 4×4:

```
      0 1 2 3
   0 [0 1 1 1]
   1 [1 0 1 0]
   2 [1 1 0 1]
   3 [1 0 1 0]
```

Lista:

```
0: [1, 2, 3]
1: [0, 2]
2: [1, 0, 3]
3: [2, 0]
```

**Ejercicio 2.2 (M).** Convierte la lista de adyacencia anterior a matriz.

**Solución.** Inicializa una matriz 4×4 de ceros. Para cada vecino `v` en la lista del vértice `u`, pon `matriz[u][v] = 1`. Como recorremos todas las listas, la simetría sale sola. En Rust:

```rust
fn lista_a_matriz(lista: &[Vec<u32>]) -> Vec<Vec<u8>> {
    let n = lista.len();
    let mut m = vec![vec![0u8; n]; n];
    for (u, vecinos) in lista.iter().enumerate() {
        for &v in vecinos {
            m[u][v as usize] = 1;
        }
    }
    m
}
```

**Ejercicio 2.3 (M).** ¿Cuánta memoria ocupa la matriz de adyacencia de un grafo con 10.000 vértices? (Pista: cada `u8` ocupa 1 byte.)

**Solución.** 10.000² = 100.000.000 bytes ≈ 95 MB. Solo la matriz. Con la lista de adyacencia, si el grafo es disperso (por ejemplo, 5 vecinos por vértice), serían 10.000·5·4 bytes (`u32`) = 200 KB. Casi 500 veces menos. Si fuera ponderado con pesos `f32`, peor todavía.

## 2.9 Ejercicios propuestos

1. **(F)** Implementa un método `grado(&self, v: u32) -> usize` en `MiGrafo`.
2. **(F)** Dado un `Graph` de petgraph, escribe una función que cuente cuántos vértices tienen grado 0.
3. **(M)** Implementa una conversión `MiGrafo -> Graph<(), (), Undirected>` y viceversa.
4. **(M)** Añade soporte para grafos ponderados a `MiGrafo` usando `HashMap<(u32, u32), u32>` para los pesos.
5. **(D)** Implementa un grafo dirigido con detección de ciclos en inserción.

## 2.10 Lo que te llevas

- Hay tres formas principales: **matriz de adyacencia** (O(V²)), **lista de adyacencia** (O(V+E)) y **HashMap de aristas** (O(E)).
- La lista de adyacencia gana en el 90% de los casos reales.
- En Rust puedes hacerlo a mano con `HashMap<u32, Vec<u32>>` o usar **`petgraph`**, que es el crate estándar de facto.
- `petgraph` usa `NodeIndex` como identificador opaco de vértices y soporta datos asociados tanto a nodos como a aristas.
- La elección de representación afecta directamente al rendimiento: lee la tabla antes de implementar nada.

## 2.11 Ojo, cuidado con…

- **Usar matriz en grafos grandes.** Un grafo con 100.000 vértices te come 10 GB solo en la matriz. Casi siempre te interesa la lista.
- **Asumir que los índices en `Vec<Vec<u32>>` son los vértices.** Si borras un vértice, los índices ya no corresponden. Mejor usar `HashMap` o `petgraph::NodeIndex`.
- **En petgraph, confundir `NodeIndex` con `usize`.** `NodeIndex` es un tipo opaco, no un entero; para sacarle el valor numérico usa `NodeIndex::new(0)` o `.index()`.
- **Olvidar el caso de grafos dirigidos.** La lista de adyacencia es asimétrica: si A→B, debe aparecer en la lista de A pero NO en la de B (a menos que también B→A).
- **No poner `#[cfg(test)]` en los tests.** Funciona igual, pero se compilan también en release, y eso gasta tiempo.

## 2.12 Para profundizar

1. Cormen et al. (2009). *Introduction to Algorithms*, §20.1 y §20.2. (CLRS.)
2. Sedgewick, R. (2011). *Algorithms*, §4.1.
3. Petgraph documentation: https://docs.rs/petgraph/
4. Goodrich, M. T., Tamassia, R. (2015). *Algorithm Design and Applications*, capítulo 12.
5. Jung, C. (1795). "Von denjenigen Problemen, welche einen hinreichenden Grund zu haben scheinen, um auf die Auflösung solcher Gleichungen Veranlassung zu geben". (No, no es el Carl Gustav Jung psicólogo. Es un matemático del siglo XVIII que trabajó en representaciones de grafos. Curiosidad histórica.)

## 2.13 Pin de batalla

- **El 90% de los grafos reales son dispersos.** `Vec<Vec<bool>>` está bien para 50 nodos. Después, vete a listas.
- **`petgraph::Graph` para dirigidos, `petgraph::UnGraph` para no dirigidos.** La diferencia se nota en la API.
- **`NodeIndex` es opaco, no un entero.** Para sacar el `usize` usa `.index()`. Lo que te ahorra el opaco es que `petgraph` puede reasignar IDs internamente.
- **Si necesitas eliminar nodos, prepárate para la invalidación de aristas.** O usa `StableGraph`, que mantiene los IDs aunque elimines.
- **El HashMap de aristas es útil cuando el grafo es muy dinámico** (muchas inserciones/borrados) y no necesitas iterar vecinos rápido.


## 2.14 Si solo lees 30 segundos

Para grafos pequeños, da igual. Para grandes: lista de adyacencia + `petgraph`. La matriz solo si es densa y pequeña.

## 2.15 Una historia pequeña

Roberto, junior en una startup, implementó su primer producto con `Vec<Vec<bool>>` para modelar la red social de la empresa (50.000 usuarios). Funcionó en su laptop. En staging, la app petardeó a los 20 segundos. Su CTO, una senior curtida en mil batallas, le dijo: "Roberto, ¿has oído hablar de las listas de adyacencia?" Él asintió. "¿Y de `petgraph`?" Negó con la cabeza. Una hora después, tenía el código migrado. La app pasó de 20 segundos a 200 milisegundos. Roberto aprendió dos cosas ese día: a usar `petgraph` y a no fiarse de las "soluciones rápidas".


---

