# Capítulo 5 — Árbol de Expansión Mínima (MST)

Hay tres algoritmos para conectar pueblos con cable mínimo. Los tres los inventó gente distinta en distintos países, sin hablarse. La matemática converge cuando el problema es real.
## 5.0 La anécdota de la electrificación y el “cuaderno Amsterdam”

Antes de que el MST tuviera nombre, hubo dos problemas prácticos esperando la misma solución. En **1926**, el checo **Otakar Borůvka** trabajaba como ingeniero eléctrico en Moravia (parte de la actual Chequia) y le pidieron la manera más barata de tender cableado para electrificar la región. Literalmente, la tarea era: dadas varias ciudades, conecta todos los pueblos con cable de alta tensión minimizando el cobre total. Borůvka publicó un algoritmo en checo, en una revista de ingeniería local, y durante treinta años nadie fuera de su país se enteró de que existía.

Trece años después, en **1956**, el estadounidense **Joseph Kruskal** envió a la revista *Proceedings of the AMS* un artículo breve, casi un diario personal, donde redescubría la idea desde cero y proponía su versión *greedy* (la que hoy todos llamamos “Kruskal”). Lo escribió desde el *Mathematical Center* de **Ámsterdam**, donde estaba de visitante. Lo que el propio Kruskal reconoció más tarde es que su artículo contenía un error en el ejemplo principal y que el revisor, distraído o generoso, lo publicó igual. La moraleja: a veces los papers más citados de la historia son los que salieron con erratas.

Mientras tanto, **Robert Prim** (1957) publicó independientemente su propia variante, descubierta también por **Vojtěch Jarník** en 1930 —un patrón habitual en algoritmos: alguien lo inventa, el resto lo “redescubre” y se le acaba poniendo el nombre de quien lo difundió en inglés.

Definamos, por fin, de qué hablamos.


> — Kruskal o Prim, ¿cuál es mejor?
> — Para grafos dispersos, Kruskal. Para grafos densos, Prim con heap. En la práctica, `petgraph::algo::min_spanning_tree` usa Prim y va bien para todo.
> — ¿Y Union-Find?
> — Kruskal no funciona sin Union-Find. Implementa `find` con path compression y `union` con rank. Las dos optimizaciones son obligatorias, no opcionales.
> — ¿Y si tengo aristas con peso 0?
> — Funciona igual. MST con peso 0 es "gratis", como debería ser.
## 5.1 ¿Qué es un MST?

Dado un grafo **no-dirigido**, **ponderado** y **conexo** $G = (V, E, w)$ con $w: E \to \mathbb{R}$, un **árbol de expansión mínima** (*Minimum Spanning Tree*, **MST**) es un subconjunto $T \subseteq E$ que cumple:

1. **Acíclico**: $T$ no contiene ciclos.
2. **Expansor**: $T$ conecta todos los $|V|$ vértices.
3. **Óptimo**: $w(T) = \sum_{e \in T} w(e)$ es mínimo entre todos los árboles de expansión.

Como $|T| = |V| - 1$ y $T$ es acíclico y conexo, $T$ es un árbol por definición. La analogía de Borůvka sigue siendo la más clara: si tienes pueblos que electrificar, el MST es el **cableado mínimo** que mantiene a todos enchufados, sin que sobre cobre dando vueltas.

**Palabras clave** que vamos a usar en este capítulo:
- **Árbol de expansión**: subconjunto de aristas que conecta todos los nodos sin ciclos.
- **Corte** (*cut*): partición $(S, V \setminus S)$ del conjunto de vértices en dos lados.
- **Arista de corte mínima**: la arista más barata que cruza un corte.
- **Greedy**: estrategia que toma la mejor decisión local en cada paso esperando que sea globalmente buena.
- **Union-Find** (DSU): estructura que mantiene conjuntos disjuntos con `union` y `find` casi-constantes.
- **Path compression**: optimización que aplana árboles de punteros durante `find`.
- **Union by rank**: heurística de equilibrado al fusionar dos árboles.

## 5.2 La propiedad de corte

Un **corte** $(S, V \setminus S)$ parte el grafo en dos; las aristas que tienen un extremo en cada lado “cruzan” el corte.

> **Teorema (Propiedad de corte)**: para cualquier corte del grafo, la arista de menor peso que lo cruza pertenece a *algún* MST.

La intuición es deliciosa. Imagina que ya tienes un MST construido. Si una arista barata que cruza un corte no está incluida, **siempre** puedes intercambiarla por la arista más cara que sí esté en tu árbol cruzando ese mismo corte, y obtienes un árbol de igual o menor peso. Por tanto, esa arista barata era “segura” desde el principio.

Esta propiedad es el corazón de los dos algoritmos que vamos a ver: ambos eligen repetidamente la arista de menor peso que cruza *algún* corte válido.

```
       S          V \ S
    [a]---1---[b]
     |  \         |
     3   2        4
     |     \      |
    [c]---5---[d]
```

En este dibujo, el corte $S = \{a, c\}$ deja tres aristas que lo cruzan: $a\!-\!b$ (peso 1), $a\!-\!d$ (peso 2) y $c\!-\!d$ (peso 5). Por la propiedad de corte, la arista $a\!-\!b$ (peso 1) está en *algún* MST. La intuición: si no estuviera, podríamos meterla y quitar la más pesada del camino entre $a$ y $b$ por el árbol — el coste no empeora.

## 5.3 Kruskal: aristas en orden y Union-Find

**Idea**: ordena las aristas por peso ascendente y ve añadiendo cada una *si no forma un ciclo*.

Para detectar el ciclo necesitamos una estructura auxiliar: **Union-Find** (también llamada **DSU**, *Disjoint Set Union*). Mantiene una partición de elementos en conjuntos y responde en tiempo casi-constante:

- `find(x)`: ¿cuál es el representante del conjunto de $x$?
- `union(x, y)`: fusiona los conjuntos de $x$ e $y$.

El truco: si al intentar añadir $(u, v)$ resulta que `find(u) == find(v)`, entonces $u$ y $v$ ya están conectados, y la arista cerraría un ciclo. La descartamos.

**Complejidad**: $O(E \log E)$ por el orden, dominando sobre las operaciones de DSU (que con *path compression* + *union by rank* son $O(\alpha(V))$ amortizadas, donde $\alpha$ es la inversa de la función de Ackermann: para cualquier $n$ realista, vale menos de 5).

## 5.4 Prim: crecer desde un nodo con un heap

Prim también es *greedy*, pero en vez de mirar aristas globales, **crece el árbol a partir de un vértice raíz**. En cada paso, añade la arista más barata que conecta el árbol actual con un vértice nuevo.

**Variante “lazy”** (la más fácil de implementar):
1. Elige un vértice $s$, márcalo visitado, mete en un *min-heap* todas las aristas que salen de $s$.
2. Repite: saca la arista de menor peso $(u, v)$. Si $v$ ya está visitado, ignórala. Si no, márcalo visitado, añade la arista al árbol y mete en el heap las aristas $(v, x)$ con $x$ aún no visitado.
3. Detente cuando el árbol tenga $|V| - 1$ aristas.

**Complejidad**: $O(E \log V)$ con un *binary heap*. La variante “eager” con un *Fibonacci heap* baja a $O(E + V \log V)$, pero en la práctica el binary heap gana por constantes.

## 5.5 Maximum Spanning Tree

Si lo que quieres es **maximizar** el peso total (por ejemplo, maximizar el ancho de banda agregado de una red), basta con **multiplicar los pesos por $-1$** y aplicar el MST normal. La estructura del árbol es la misma, los pesos solo cambian de signo. Mismo coste, misma elegancia.

## 5.6 Aplicaciones del mundo real

- **Redes eléctricas y de fibra**: el caso Borůvka original.
- **Clustering aglomerativo**: cortar las $k-1$ aristas más pesadas del MST da $k$ clusters maximizando la separación.
- **Aproximaciones a NP-duros**: TSP métrico admite una 2-aproximación basada en MST; Steiner tree, factor 2.
- **Bioinformática**: redes de genes con pesos por correlación.
- **Diseño de redes de agua y tuberías**.
- **Análisis de imágenes**: segmentación de píxeles con pesos por diferencia de intensidad.

## 5.7 Implementación en Rust 2024

Empecemos por el Union-Find, que es nuestro “caballo de batalla”. Lo escribimos *a mano* la primera vez, luego veremos cómo nos ahorra trabajo `petgraph`.

```rust
// src/union_find.rs
//! Union-Find (DSU) con path compression + union by rank.

/// Estructura para mantener conjuntos disjuntos.
#[derive(Debug)]
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
    n_components: usize,
}

impl UnionFind {
    /// Crea un DSU con `n` elementos, cada uno en su propio conjunto.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
            n_components: n,
        }
    }

    /// Devuelve el representante (raíz) del conjunto de `x`.
    /// Aplica *path compression* en dos pasos para aplanar el camino.
    pub fn find(&mut self, x: usize) -> usize {
        // Primer pase: encuentra la raíz.
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Segundo pase: cuelga cada nodo visitado directamente de la raíz.
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    /// Fusiona los conjuntos de `a` y `b`. Devuelve `true` si estaban separados.
    /// Aplica *union by rank* para mantener la altura acotada.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        // El árbol más bajo se cuelga del más alto.
        let (big, small) = if self.rank[ra] < self.rank[rb] {
            (rb, ra)
        } else {
            (ra, rb)
        };
        self.parent[small] = big;
        if self.rank[big] == self.rank[small] {
            self.rank[big] += 1;
        }
        self.n_components -= 1;
        true
    }

    /// ¿Están `a` y `b` en el mismo conjunto?
    pub fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Número de conjuntos disjuntos actuales.
    pub fn components(&self) -> usize {
        self.n_components
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_basico() {
        let mut dsu = UnionFind::new(5);
        assert!(dsu.union(0, 1));
        assert!(dsu.union(2, 3));
        assert!(dsu.union(3, 4));
        assert!(dsu.union(0, 4));
        assert!(dsu.connected(0, 4));
        assert_eq!(dsu.components(), 1);
    }

    #[test]
    fn union_rechaza_repetidos() {
        let mut dsu = UnionFind::new(3);
        assert!(dsu.union(0, 1));
        assert!(!dsu.union(0, 1)); // ya estaban juntos
        assert_eq!(dsu.components(), 2);
    }
}
```

Y ahora Kruskal sobre la estructura:

```rust
// src/kruskal.rs
//! Algoritmo de Kruskal para MST, usando Union-Find.

use crate::union_find::UnionFind;

/// Arista no-dirigida con peso. Implementa `Ord` por peso para ordenar/heap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub u: usize,
    pub v: usize,
    pub w: f64,
}

impl Eq for Edge {}

impl PartialOrd for Edge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Edge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // `f64` no implementa `Ord` por la presencia de NaN; en MST los pesos
        // son finitos, así que la comparación por `total_cmp` es segura.
        self.w.total_cmp(&other.w)
    }
}

/// MST de Kruskal. Devuelve (aristas, peso total).
/// Si el grafo no es conexo, devuelve un *Minimum Spanning Forest* (MSF).
pub fn mst_kruskal(n: usize, mut edges: Vec<Edge>) -> (Vec<Edge>, f64) {
    edges.sort(); // por peso, gracias al `Ord` que definimos
    let mut dsu = UnionFind::new(n);
    let mut mst: Vec<Edge> = Vec::with_capacity(n.saturating_sub(1));
    let mut total = 0.0;

    for e in edges {
        if dsu.union(e.u, e.v) {
            total += e.w;
            mst.push(e);
            if mst.len() == n.saturating_sub(1) {
                break;
            }
        }
    }
    (mst, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafo_ejemplo() -> Vec<Edge> {
        vec![
            Edge { u: 0, v: 1, w: 1.0 },
            Edge { u: 0, v: 2, w: 4.0 },
            Edge { u: 0, v: 3, w: 3.0 },
            Edge { u: 1, v: 3, w: 2.0 },
            Edge { u: 2, v: 3, w: 5.0 },
        ]
    }

    #[test]
    fn mst_peso_7() {
        let (mst, total) = mst_kruskal(4, grafo_ejemplo());
        assert_eq!(mst.len(), 3);
        assert!((total - 7.0).abs() < 1e-9);
    }

    #[test]
    fn mst_completo_triangulo() {
        // Triángulo equilátero: MST = 2 aristas más baratas.
        let edges = vec![
            Edge { u: 0, v: 1, w: 1.0 },
            Edge { u: 1, v: 2, w: 1.0 },
            Edge { u: 0, v: 2, w: 1.0 },
        ];
        let (mst, total) = mst_kruskal(3, edges);
        assert_eq!(mst.len(), 2);
        assert!((total - 2.0).abs() < 1e-9);
    }
}
```

Prim, esta vez con `BinaryHeap` de la librería estándar:

```rust
// src/prim.rs
//! Prim "lazy" con BinaryHeap estándar de Rust.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::kruskal::Edge;

/// MST de Prim. Recibe lista de adyacencia `adj[u] = [(v, w), ...]`.
pub fn mst_prim(n: usize, adj: &[Vec<(usize, f64)>]) -> (Vec<Edge>, f64) {
    debug_assert!(!adj.is_empty());
    let mut visited = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(OrderedFloat, usize, usize)>> = BinaryHeap::new();
    let mut mst: Vec<Edge> = Vec::with_capacity(n.saturating_sub(1));
    let mut total = 0.0;

    // Empezamos por el vértice 0 (arbitrario).
    visited[0] = true;
    for &(v, w) in &adj[0] {
        heap.push(Reverse((OrderedFloat(w), 0, v)));
    }

    while let Some(Reverse((OrderedFloat(w), u, v))) = heap.pop() {
        if visited[v] {
            continue; // arista obsoleta
        }
        visited[v] = true;
        mst.push(Edge { u, v, w });
        total += w;
        if mst.len() == n.saturating_sub(1) {
            break;
        }
        for &(x, wx) in &adj[v] {
            if !visited[x] {
                heap.push(Reverse((OrderedFloat(wx), v, x)));
            }
        }
    }
    (mst, total)
}

/// Wrapper de `f64` para usarlo en `BinaryHeap` (que requiere `Ord`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prim_coincide_con_kruskal() {
        let adj = vec![
            vec![(1, 1.0), (2, 4.0), (3, 3.0)], // 0
            vec![(0, 1.0), (3, 2.0)],          // 1
            vec![(0, 4.0), (3, 5.0)],          // 2
            vec![(0, 3.0), (1, 2.0), (2, 5.0)],// 3
        ];
        let (mst, total) = mst_prim(4, &adj);
        assert_eq!(mst.len(), 3);
        assert!((total - 7.0).abs() < 1e-9);
    }
}
```

> **Nota sobre el `OrderedFloat`**: `BinaryHeap` requiere que sus elementos implementen `Ord`. Como `f64` solo implementa `PartialOrd` (por culpa del infame `NaN`), envolvemos el peso en una *newtype* y usamos `total_cmp`, que ordena de manera total sin tropezar con `NaN`. Es un patrón muy común en Rust numérico.

## 5.8 MST con `petgraph`

La crate [`petgraph`](https://crates.io/crates/petgraph) es la navaja suiza de grafos en Rust. Para MST expone `min_spanning_tree`, que devuelve un iterador de aristas:

```toml
# Cargo.toml
[dependencies]
petgraph = "0.6"
```

```rust
// src/mst_petgraph.rs
//! MST usando `petgraph` (comparación con versión manual).

use petgraph::algo::min_spanning_tree;
use petgraph::graph::UnGraph;

pub fn mst_petgraph_limpio(n: usize, aristas: &[(usize, usize, f64)]) -> f64 {
    let mut g: UnGraph<(), f64> = UnGraph::new_undirected();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v, w) in aristas {
        g.add_edge(u.into(), v.into(), w);
    }
    min_spanning_tree(&g)
        .map(|e| *e.weight())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petgraph_coincide_con_kruskal() {
        let aristas = vec![
            (0, 1, 1.0), (0, 2, 4.0), (0, 3, 3.0),
            (1, 3, 2.0), (2, 3, 5.0),
        ];
        let total = mst_petgraph_limpio(4, &aristas);
        assert!((total - 7.0).abs() < 1e-9);
    }
}
```

> **Pista**: `min_spanning_tree` de `petgraph` implementa **Prim eager con binary heap**. Si quieres Kruskal explícito, lo más fácil es pasar por `min_spanning_tree_prim` o construirlo a mano con la DSU, como hicimos en §5.7.

### Comparación: ¿cuándo usar qué?

| Situación | ¿Qué uso? |
|---|---|
| `petgraph` ya en el proyecto, grafo “típico” | `petgraph::algo::min_spanning_tree` |
| Necesito el MST en streaming o no quiero un grafo entero | DSU + sort manual (Kruskal) |
| Necesito Maximum Spanning Tree | Negar pesos o usar el truco de `Reverse` |
| Fines didácticos / entrevista | Implementar Kruskal a mano con DSU |
| Grafo disperso de millones de aristas | Kruskal + DSU casi siempre es más eficiente en RAM |

## 5.9 Maximum Spanning Tree en Rust

El truco clásico: multiplicar por $-1$ los pesos antes de pasar al MST estándar.

```rust
// src/max_mst.rs
//! Maximum Spanning Tree: negar pesos y aplicar MST clásico.

use crate::kruskal::{mst_kruskal, Edge};

pub fn max_st(n: usize, mut edges: Vec<Edge>) -> (Vec<Edge>, f64) {
    for e in &mut edges {
        e.w = -e.w;
    }
    let (mst, neg_total) = mst_kruskal(n, edges);
    let total = -neg_total;
    (mst, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_st_triangulo() {
        // Pesos 1, 2, 3 → max-ST = 2 aristas más caras = 2 + 3 = 5.
        let edges = vec![
            Edge { u: 0, v: 1, w: 1.0 },
            Edge { u: 1, v: 2, w: 2.0 },
            Edge { u: 0, v: 2, w: 3.0 },
        ];
        let (_, total) = max_st(3, edges);
        assert!((total - 5.0).abs() < 1e-9);
    }
}
```

## 5.10 Ejercicios resueltos

### Ejercicio 1 — Cableado de Moravia (mini)

Cinco ciudades deben conectarse. Costes de las posibles líneas (miles de €):

```
        Mad  Bcn  Val  Sev  Bil
Mad  –    6    4    8    7
Bcn  6   –     3    9    5
Val  4   3    –     6    4
Sev  8   9    6    –     2
Bil  7   5    4    2    –
```

Aplica Kruskal a mano. ¿Cuál es la red mínima?

**Solución**: ordenamos las aristas por peso: (Sev–Bil, 2), (Bcn–Val, 3), (Mad–Val, 4), (Val–Bil, 4), (Bcn–Bil, 5), (Mad–Bcn, 6), (Mad–Bil, 7), (Mad–Sev, 8), (Bcn–Sev, 9), (Val–Sev, 6). Las cuatro primeras no crean ciclo: 2 + 3 + 4 + 4 = **13 mil €**. (¡Y vemos que Mad–Val, la diagonal corta, sí entra!)

### Ejercicio 2 — ¿Por qué Prim no se atasca?

Explica por qué Prim no puede quedarse atascado si el grafo es conexo.

**Solución**: mientras $|T| < |V| - 1$, existe al menos una arista de $T$ hacia $V \setminus T$ (porque $G$ es conexo), así que el heap siempre contiene candidatos. Por la propiedad de corte, la más barata es segura.

### Ejercicio 3 — LeetCode 1584: *Min Cost to Connect All Points*

Dados $n$ puntos en el plano, conecta todos con coste igual a la **distancia Manhattan** $|x_1 - x_2| + |y_1 - y_2|$. Devuelve el coste mínimo.

**Planteamiento**: el grafo implícito es **completo** ($n(n-1)/2$ aristas). Para $n$ moderado ($n \le 10^3$) basta con Prim:

```rust
// src/ej_leetcode_1584.rs
//! LeetCode 1584 — Min Cost to Connect All Points (Manhattan).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub fn min_cost_connect_points(points: &[Vec<i32>]) -> i32 {
    let n = points.len();
    if n == 0 {
        return 0;
    }
    let mut visited = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = BinaryHeap::new();
    let mut total = 0i64;
    let mut visitados = 0usize;

    // Empezamos por el punto 0.
    visited[0] = true;
    for j in 1..n {
        let d = (points[0][0] - points[j][0]).abs()
              + (points[0][1] - points[j][1]).abs();
        heap.push(Reverse((d, j)));
    }
    visitados += 1;

    while let Some(Reverse((d, v))) = heap.pop() {
        if visited[v] {
            continue;
        }
        visited[v] = true;
        total += d as i64;
        visitados += 1;
        if visitados == n {
            break;
        }
        for u in 0..n {
            if !visited[u] {
                let dd = (points[v][0] - points[u][0]).abs()
                       + (points[v][1] - points[u][1]).abs();
                heap.push(Reverse((dd, u)));
            }
        }
    }
    total as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caso_ejemplo() {
        let pts = vec![vec![0, 0], vec![2, 2], vec![3, 10], vec![5, 2], vec![7, 0]];
        assert_eq!(min_cost_connect_points(&pts), 20);
    }

    #[test]
    fn un_solo_punto() {
        assert_eq!(min_cost_connect_points(&[vec![1, 1]]), 0);
    }
}
```

Complejidad: $O(n^2 \log n)$ (las aristas son $O(n^2)$).

## 5.11 Ejercicios propuestos

1. **(F) MST único**. Prueba que si todos los pesos de las aristas son distintos, el MST es único. Pista: usa la propiedad de corte.
2. **(M) Second-Best MST**. Dado un MST, encuentra el árbol de expansión de peso mínimo **distinto** del MST. Pista: para cada arista no-MST, busca la arista de peso máximo en el camino entre sus extremos dentro del MST.
3. **(M) Reverse Delete**. Implementa el algoritmo inverso a Kruskal: parte de todas las aristas y elimina la más pesada que no desconecte el grafo. Demuestra que también produce un MST.
4. **(D) Red de agua con 8 pueblos**. Modela con coordenadas y coste proporcional a la distancia euclídea. Compara el resultado de `petgraph::algo::min_spanning_tree` con tu Kruskal manual y verifica que coincidan (en peso, no necesariamente en aristas, si hay empates).

## 5.12 Lo que te llevas

- Un **MST** es el subconjunto de aristas más barato que conecta todos los vértices sin ciclos.
- La **propiedad de corte** es el pegamento teórico: cualquier arista de peso mínimo cruzando un corte puede añadirse con seguridad.
- **Kruskal** ordena aristas y decide con Union-Find; **Prim** crece desde un nodo con un heap. Ambos son $O(E \log V)$ en la práctica.
- **Union-Find** con *path compression* + *union by rank* ofrece operaciones casi-constantes ($\alpha(V)$ amortizado).
- En Rust, `petgraph::algo::min_spanning_tree` te lo da hecho (Prim eager); para Kruskal, escribe la DSU tú mismo (es un *rite of passage*).
- **Maximum Spanning Tree** = negar pesos y aplicar el MST normal.

## 5.13 Ojo, cuidado con…

- **Grafos no conexos**. Si el grafo no es conexo, el MST no existe. Lo que sí existe es un *Minimum Spanning Forest* (un MST por componente). Kruskal y Prim lo manejan devolviendo menos de $|V| - 1$ aristas — comprueba `mst.len()` antes de cantar victoria.
- **Pesos `NaN`**. `f64::NaN` rompe la comparación: usa `total_cmp` o `OrderedFloat` como hicimos. Si los pesos vienen de divisiones, ¡cuidado con el `0.0 / 0.0`!
- **Overflow con enteros**. En grafos grandes, los pesos en `i32` pueden desbordarse. Si vas a sumar, usa `i64` o `f64`.
- **Aristas paralelas**. En grafos con aristas duplicadas, el MST no incluye a la peor de cada par. Kruskal las descarta automáticamente; Prim con un heap las “evalúa” varias veces. Ambos llegan al mismo resultado, pero Kruskal es más predecible.
- **Empezar Prim “con nodo que no existe”**. Si recibes un grafo vacío, `visited[0]` revienta. Comprueba `n == 0` antes.
- **“Confundir maximal con máximo”**. Un *maximal matching* (cap. 8) o un *maximal independent set* no son necesariamente máximos. Aquí no aplica directamente, pero recuerda la distinción: en MST hablamos siempre de máximo (global), no maximal (local).

## 5.14 Para profundizar

- **Kruskal, J. B. (1956).** “On the shortest spanning subtree of a graph and the traveling salesman problem”. *Proceedings of the AMS*, 7(1).
- **Borůvka, O. (1926).** “O jistém problému minimálním” (en checo). Disponible en traducción al inglés en *Prague Studies in Mathematical Linguistics* (2012).
- **Cormen, Leiserson, Rivest, Stein.** *Introduction to Algorithms* (3ª ed.), Cap. 23 — la referencia canónica.
- **Kleinberg & Tardos.** *Algorithm Design*, Cap. 4 — explicaciones intuitivas.
- **Sedgewick & Wayne.** *Algorithms* (4ª ed.), Sec. 4.3 — implementaciones en Java que traduces a Rust fácilmente.
- Vídeo: Reducible — *Minimum Spanning Trees* (<https://www.youtube.com/watch?v=Ia1nOzC0vyY>).

## 5.15 Pin de batalla

- **`petgraph::algo::min_spanning_tree` ya está implementado.** No lo reescribas a no ser que sea para aprender.
- **Para grafos muy grandes con cambios dinámicos, mira `link-cut trees`.** MST dinámico es otro juego.
- **Maximum Spanning Tree es MST con pesos negados.** Truco viejo pero útil.
- **Si tu grafo es bipartito, el MST es único y se calcula en O(E).** Característica topológica bonita.
- **En redes de computadores, MST = topología mínima para que todo se comunique.** Útil para diseñar LANs de oficinas.


## 5.16 Si solo lees 30 segundos

MST = árbol de peso mínimo que conecta todos los nodos. Kruskal con Union-Find, o Prim con heap. `petgraph` ya lo trae.

## 5.17 Una historia pequeña

Otakar Borůvka era un ingeniero eléctrico checo en 1926. Le pidieron la manera más barata de electrificar Moravia. Publicó su algoritmo en checo, en una revista local. Treinta años después, un estadounidense llamado Joseph Kruskal publicó casi el mismo algoritmo en inglés. Se hizo famoso. Borůvka nunca supo que su invento era referencia obligatoria en universidades de todo el mundo. Cuando le preguntaron en una entrevista, ya anciano, cómo se sentía, dijo: "no me importa el crédito, me importa que la gente siga electrificando pueblos." Heroico.


---

