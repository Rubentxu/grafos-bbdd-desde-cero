# Capítulo 7 — Union-Find y componentes conexas

Robert Tarjan inventó el algoritmo de SCC trabajando solo los martes y los miércoles por la tarde. Porque le daba igual la productividad industrial. Es uno de los algoritmos más elegantes que existen, y probablemente el más difícil de recordar de la historia.
## 7.0 La anécdota de Tarjan y los “martes productivos”

**Robert Endre Tarjan** es, posiblemente, el científico de computación más infravalorado del siglo XX en proporción a su impacto. Ganó el **Premio Turing en 1986** (el “Nobel de la computación”) por inventar, entre otras cosas, el algoritmo de **Strongly Connected Components** en una sola pasada de DFS —una proeza teórica que durante años la comunidad creyó imposible sin dos DFS, como en Kosaraju.

Lo fascinante de Tarjan no es solo lo que inventó, sino *cómo lo hacía*. Él mismo contó en entrevistas que su rutina era peculiar: **solo programaba los martes por la tarde** y los miércoles por la mañana. El resto de la semana la pasaba leyendo, pensando, dando paseos y hablando con colegas. Decía que la productividad industrial (8 horas diarias picando código) era una “ilusión social” y que las mejores ideas le venían resolviendo el problema de fondo en su cabeza el resto del tiempo. La algoritmos de SCC, low-link, union-find casi-lineal y muchos otros salieron de esa extraña cadencia.

Esta filosofía —**invertir el tiempo en pensar el problema, no en teclear la solución**— es exactamente lo que necesitarás para entender este capítulo. Union-Find parece trivial al principio, pero esconder tras él una de las cotas amortizadas más bellas de toda la algoritmia: la **inversa de la función de Ackermann**, $\alpha(n)$, que para cualquier $n$ que exista en el universo observable vale menos de 5. Casi constante.

Definamos, por fin, qué hace.


> — Tarjan o Kosaraju para SCC, ¿cuál?
> — Tarjan. 1 pasada de DFS vs 2 de Kosaraju. Más eficiente en práctica.
> — ¿Y low-link values?
> — Es lo que hace Tarjan. Cada nodo guarda el menor `discovery time` alcanzable por back-edges. Si el low de un hijo es >= discovery del padre, hay un bridge.
> — ¿Y los articulation points?
> — Mismo algoritmo, mirando padres. Si un hijo tiene low >= disc del padre y no es la raíz, es articulation.
> — Madre mía, qué difícil.
> — Sí. Pero `petgraph::algo::tarjan_scc` te lo da en una línea. Aprende la teoría, usa la librería.
## 7.1 Conjuntos disjuntos: la estructura

La estructura **Disjoint Set Union** (**DSU**), también llamada **Union-Find**, mantiene una partición de un universo de $n$ elementos en conjuntos disjuntos y soporta dos operaciones en tiempo casi-constante:

- `find(x)`: devuelve un *representante* canónico del conjunto que contiene $x$.
- `union(x, y)`: fusiona los conjuntos que contienen $x$ e $y$.

Aplicaciones directas: componentes conexas en grafos no-dirigidos, detección de ciclos, MST (Capítulo 5), Kruskal, percolación, segmentación de imágenes, *accounts merge*, *friend circles*.

**Palabras clave**:
- **DSU** (*Disjoint Set Union*): nombre formal de Union-Find.
- **Path compression**: aplana el árbol durante `find` colgando cada nodo de la raíz.
- **Union by rank/size**: cuelga el árbol pequeño del grande, manteniendo altura $O(\log n)$.
- **Ackermann inversa** $\alpha(n)$: cota amortizada de las operaciones combinando las dos optimizaciones.
- **SCC** (*Strongly Connected Component*): maximal subconjunto de vértices donde cada uno alcanza a todos los demás.
- **Kosaraju-Sharir**: algoritmo de 2 DFS para SCC.
- **Tarjan SCC**: algoritmo de 1 DFS basado en *low-link values*.
- **Low-link**: para cada vértice, el menor `disc[u]` alcanzable desde su subárbol DFS.
- **Bridge**: arista cuya eliminación desconecta el grafo.
- **Articulation point**: vértice análogo.

## 7.2 Las dos optimizaciones clásicas

**Path compression**: durante `find`, hacemos que cada nodo visitado apunte directamente a la raíz. El camino se aplana y las futuras búsquedas son $O(1)$.

**Union by rank/size**: al fusionar, colgamos la raíz del árbol *más bajo* (menor rank) bajo la del *más alto*. La altura se mantiene en $O(\log n)$.

Con ambas optimizaciones, la complejidad amortizada de $m$ operaciones sobre $n$ elementos es $O(m \cdot \alpha(n))$. En cualquier $n$ realista del universo, $\alpha(n) < 5$. En la práctica, es **constante**.

**Variante `union by size`**: en lugar de rank, comparamos el tamaño del subárbol y colgamos el pequeño del grande; facilita cálculos de tamaños de conjunto.

## 7.3 Componentes conexas en grafo no-dirigido

**Opción A — BFS/DFS** (Sección 1): un recorrido marca toda una componente; iterando sobre vértices no visitados se obtienen todas en $O(V + E)$.

**Opción B — Union-Find**: recorremos las aristas y hacemos `union(u, v)` por cada una. Al final, el número de conjuntos restantes es el número de componentes. Coste $O(E \cdot \alpha(V))$, ideal en streaming o cuando solo necesitamos el *conteo*, no los miembros.

## 7.4 Strongly Connected Components (SCC)

En grafos *dirigidos* hablamos de **SCC**: subconjuntos de vértices donde cada uno alcanza a todos los demás. El **grafo de componentes** (SCC-graph) es siempre un **DAG** — una de esas joyitas teóricas que se demuestra con dos líneas.

### Kosaraju-Sharir (2 DFS)

1. DFS desde todos los vértices: guarda el *tiempo de salida* (o pila de post-orden).
2. Construye el grafo transpuesto $G^T$ (aristas invertidas).
3. DFS en $G^T$ en orden *decreciente* de tiempo de salida; cada recorrido encuentra una SCC.

**Complejidad**: $O(V + E)$. **Memoria**: $O(V)$ para la pila.

### Tarjan (1 DFS, low-link)

Mantiene para cada vértice $u$ un `disc[u]` (tiempo de descubrimiento) y un `low[u]` (mínimo `disc` alcanzable por aristas del subárbol DFS). Apila los vértices en una pila auxiliar; cuando `low[u] == disc[u]`, $u$ es la raíz de una SCC y se desapila todo hasta $u$.

**Complejidad**: $O(V + E)$ con una sola pasada. Es el algoritmo favorito de Tarjan, y el más elegante.

## 7.5 Bridges y articulation points

- Una **puente** (*bridge*) es una arista cuya eliminación aumenta el número de componentes conexas.
- Un **punto de articulación** (*articulation point*, *cut vertex*) es un vértice análogo.

Ambos se detectan con el mismo esquema low-link de Tarjan:
- Una arista $(u, v)$ es puente si `low[v] > disc[u]`.
- Un vértice $u$ es punto de articulación si tiene un hijo $v$ con `low[v] >= disc[u]` (o, si $u$ es raíz, tiene más de un hijo en el DFS-tree).

## 7.6 Aplicaciones del mundo real

- **Análisis de redes web**: una SCC es un conjunto de páginas que se enlazan mutuamente, útil para detección de *link farms*.
- **Reacciones químicas**: SCC en grafo de dependencia de especies.
- **Resiliencia de redes**: bridges son cuellos de botella; puntos de articulación son routers cuya caída fragmentaría la red.
- **2-SAT**: las SCC del grafo de implicaciones determinan satisfacibilidad.
- **Detección de comunidades**: en redes sociales, el SCC de “followers mutuos” es una comunidad fuerte.
- **Compiladores**: detección de ciclos en grafos de llamadas o flujos de datos.
- **Procesamiento de imágenes**: componentes conexas definen regiones y *flood fill*.

## 7.7 Implementación en Rust 2024

Empecemos por la DSU “a mano”:

```rust
// src/dsu.rs
//! Union-Find con path compression + union by size.

/// DSU con dos optimizaciones: path compression + union by size.
#[derive(Debug)]
pub struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
    components: usize,
}

impl Dsu {
    /// Crea un DSU con `n` elementos, cada uno en su propio conjunto.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
            components: n,
        }
    }

    /// Encuentra la raíz del conjunto de `x` con path compression.
    pub fn find(&mut self, x: usize) -> usize {
        // Primer pase: encontrar la raíz.
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Segundo pase: comprimir el camino.
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    /// Fusiona los conjuntos de `a` y `b`. Devuelve `true` si estaban separados.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        // Cuelga el árbol pequeño del grande.
        let (big, small) = if self.size[ra] < self.size[rb] {
            (rb, ra)
        } else {
            (ra, rb)
        };
        self.parent[small] = big;
        self.size[big] += self.size[small];
        self.components -= 1;
        true
    }

    /// ¿Están `a` y `b` en el mismo conjunto?
    pub fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Número de conjuntos disjuntos actuales.
    pub fn components(&self) -> usize {
        self.components
    }

    /// Tamaño del conjunto que contiene `x`.
    pub fn size_of(&mut self, x: usize) -> usize {
        let r = self.find(x);
        self.size[r]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basico() {
        let mut dsu = Dsu::new(5);
        assert_eq!(dsu.components(), 5);
        dsu.union(0, 1);
        dsu.union(2, 3);
        dsu.union(3, 4);
        assert!(dsu.connected(0, 4));
        assert_eq!(dsu.components(), 1);
        assert_eq!(dsu.size_of(0), 4);
    }

    #[test]
    fn union_rechaza_repetidos() {
        let mut dsu = Dsu::new(3);
        assert!(dsu.union(0, 1));
        assert!(!dsu.union(0, 1));
        assert_eq!(dsu.components(), 2);
    }
}
```

Ahora **Kosaraju** y **Tarjan** para SCC:

```rust
// src/scc.rs
//! Strongly Connected Components: Kosaraju y Tarjan.

/// Kosaraju: dos pasadas de DFS (una sobre el grafo, otra sobre el transpuesto).
pub fn kosaraju(n: usize, adj: &[Vec<usize>], radj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    // Fase 1: orden de salida en `adj`.
    let mut visited = vec![false; n];
    let mut order: Vec<usize> = Vec::with_capacity(n);

    fn dfs1(u: usize, adj: &[Vec<usize>], visited: &mut [bool], order: &mut Vec<usize>) {
        visited[u] = true;
        for &v in &adj[u] {
            if !visited[v] {
                dfs1(v, adj, visited, order);
            }
        }
        order.push(u);
    }

    for u in 0..n {
        if !visited[u] {
            dfs1(u, adj, &mut visited, &mut order);
        }
    }

    // Fase 2: DFS sobre el grafo transpuesto en orden de salida decreciente.
    let mut comp_of = vec![-1i32; n];
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    fn dfs2(u: usize, c: i32, radj: &[Vec<usize>], comp_of: &mut [i32], sccs: &mut Vec<Vec<usize>>) {
        comp_of[u] = c;
        sccs[c as usize].push(u);
        for &v in &radj[u] {
            if comp_of[v] == -1 {
                dfs2(v, c, radj, comp_of, sccs);
            }
        }
    }

    for &u in order.iter().rev() {
        if comp_of[u] == -1 {
            sccs.push(Vec::new());
            let c = (sccs.len() - 1) as i32;
            dfs2(u, c, radj, &mut comp_of, &mut sccs);
        }
    }
    sccs
}

/// Tarjan: una sola pasada de DFS con low-link values.
pub fn tarjan_scc(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut disc: Vec<i32> = vec![-1; n];
    let mut low: Vec<usize> = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    let mut time = 0usize;

    fn strongconnect(
        u: usize,
        adj: &[Vec<usize>],
        disc: &mut [i32],
        low: &mut [usize],
        on_stack: &mut [bool],
        stack: &mut Vec<usize>,
        sccs: &mut Vec<Vec<usize>>,
        time: &mut usize,
    ) {
        disc[u] = *time as i32;
        low[u] = *time;
        *time += 1;
        stack.push(u);
        on_stack[u] = true;

        for &v in &adj[u] {
            if disc[v] == -1 {
                strongconnect(v, adj, disc, low, on_stack, stack, sccs, time);
                low[u] = low[u].min(low[v]);
            } else if on_stack[v] {
                low[u] = low[u].min(disc[v] as usize);
            }
        }

        if low[u] == disc[u] as usize {
            let mut comp: Vec<usize> = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                comp.push(w);
                if w == u {
                    break;
                }
            }
            sccs.push(comp);
        }
    }

    for u in 0..n {
        if disc[u] == -1 {
            strongconnect(u, adj, &mut disc, &mut low, &mut on_stack, &mut stack, &mut sccs, &mut time);
        }
    }
    sccs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grafo_3_scc() -> (usize, Vec<Vec<usize>>, Vec<Vec<usize>>) {
        // 3 SCC: {0,1,2}, {3}, {4}
        let n = 5;
        let adj = vec![
            vec![1],         // 0
            vec![2],         // 1
            vec![0],         // 2
            vec![4],         // 3
            vec![],          // 4
        ];
        let radj = vec![
            vec![2],         // 0
            vec![0],         // 1
            vec![1],         // 2
            vec![],          // 3
            vec![3],         // 4
        ];
        (n, adj, radj)
    }

    #[test]
    fn kosaraju_encuentra_3_sccs() {
        let (n, adj, radj) = grafo_3_scc();
        let sccs = kosaraju(n, &adj, &radj);
        assert_eq!(sccs.len(), 3);
    }

    #[test]
    fn tarjan_encuentra_3_sccs() {
        let (n, adj, _) = grafo_3_scc();
        let sccs = tarjan_scc(n, &adj);
        assert_eq!(sccs.len(), 3);
    }
}
```

Bridges y puntos de articulación (el low-link de Tarjan en su salsa):

```rust
// src/bridges.rs
//! Bridges y articulation points con el esquema low-link de Tarjan.

/// Devuelve (puentes, puntos de articulación).
pub fn bridges_and_articulations(
    n: usize,
    edges: &[(usize, usize)],
) -> (Vec<(usize, usize)>, Vec<usize>) {
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for &(u, v) in edges {
        adj[u].push(v);
        adj[v].push(u);
    }

    let mut disc: Vec<i32> = vec![-1; n];
    let mut low: Vec<usize> = vec![0; n];
    let mut bridges: Vec<(usize, usize)> = Vec::new();
    let mut is_artic = vec![false; n];
    let mut time = 0usize;

    fn dfs(
        u: usize,
        parent: i32,
        adj: &[Vec<usize>],
        disc: &mut [i32],
        low: &mut [usize],
        bridges: &mut Vec<(usize, usize)>,
        is_artic: &mut [bool],
        time: &mut usize,
    ) {
        disc[u] = *time as i32;
        low[u] = *time;
        *time += 1;
        let mut children = 0usize;

        for &v in &adj[u] {
            if disc[v] == -1 {
                children += 1;
                dfs(v, u as i32, adj, disc, low, bridges, is_artic, time);
                low[u] = low[u].min(low[v]);

                if low[v] > disc[u] as usize {
                    bridges.push((u, v));
                }
                if parent != -1 && low[v] >= disc[u] as usize {
                    is_artic[u] = true;
                }
            } else if v as i32 != parent {
                low[u] = low[u].min(disc[v] as usize);
            }
        }

        if parent == -1 && children > 1 {
            is_artic[u] = true;
        }
    }

    for u in 0..n {
        if disc[u] == -1 {
            dfs(u, -1, &adj, &mut disc, &mut low, &mut bridges, &mut is_artic, &mut time);
        }
    }

    let articulos: Vec<usize> = (0..n).filter(|&u| is_artic[u]).collect();
    (bridges, articulos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_con_puente() {
        // Triángulo 0-1-2-0, arista puente 2-3, y vértice colgante 4.
        let edges = vec![(0, 1), (1, 2), (2, 0), (2, 3), (3, 4)];
        let (bridges, arts) = bridges_and_articulations(5, &edges);
        assert!(bridges.contains(&(2, 3)));
        assert!(bridges.contains(&(3, 4)));
        assert!(arts.contains(&2));
        assert!(arts.contains(&3));
    }
}
```

## 7.8 Componentes, SCC y bridges con `petgraph`

`petgraph` lo trae casi todo hecho. La versión 0.6 expone:

```toml
# Cargo.toml
[dependencies]
petgraph = "0.6"
```

```rust
// src/petgraph_algoritmos.rs
//! Componentes, SCC y bridges con `petgraph`.

use petgraph::algo::{connected_components, kosaraju_scc, tarjan_scc};
use petgraph::graph::{DiGraph, UnGraph};

/// Número de componentes conexas en un grafo no-dirigido.
pub fn n_componentes_no_dirigido(n: usize, aristas: &[(usize, usize)]) -> usize {
    let mut g = UnGraph::<(), ()>::new_undirected();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    connected_components(&g)
}

/// SCC con Kosaraju (interfaz oficial de petgraph).
pub fn scc_kosaraju_pg(n: usize, aristas: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut g = DiGraph::<(), ()>::new();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    kosaraju_scc(&g)
        .into_iter()
        .map(|v| v.into_iter().map(|idx| idx.index()).collect())
        .collect()
}

/// SCC con Tarjan (interfaz oficial de petgraph).
pub fn scc_tarjan_pg(n: usize, aristas: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut g = DiGraph::<(), ()>::new();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    tarjan_scc(&g)
        .into_iter()
        .map(|v| v.into_iter().map(|idx| idx.index()).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn componentes_no_dirigido() {
        let aristas = vec![(0, 1), (1, 2), (3, 4)];
        assert_eq!(n_componentes_no_dirigido(5, &aristas), 2);
    }

    #[test]
    fn scc_petgraph() {
        let aristas = vec![(0, 1), (1, 2), (2, 0), (3, 4)];
        let sccs_k = scc_kosaraju_pg(5, &aristas);
        let sccs_t = scc_tarjan_pg(5, &aristas);
        assert_eq!(sccs_k.len(), 3);
        assert_eq!(sccs_t.len(), 3);
    }
}
```

Para **bridges**, en `petgraph` 0.6 hay `petgraph::algo::bridges`:

```rust
use petgraph::algo::bridges;
use petgraph::graph::UnGraph;

pub fn puentes_petgraph(n: usize, aristas: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut g = UnGraph::<(), ()>::new_undirected();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    bridges(&g)
        .into_iter()
        .map(|(a, b)| (a.index(), b.index()))
        .collect()
}
```

> **Cuándo usar `petgraph` vs a mano**: si la estructura de tu problema es naturalmente un grafo, usa `petgraph`: el código es más legible y las APIs están testeadas por la comunidad. Si solo necesitas union-find aislado (por ejemplo, en *streaming* donde no construyes un grafo en memoria), tu DSU hecha a mano es imbatible.

## 7.9 Ejercicios resueltos

### Ejercicio 1 — Número de provincias (LeetCode 547)

Dada una matriz `isConnected`, devuelve el número de provincias.

**Solución**: DSU; recorremos la diagonal superior, uniendo $i$ y $j$ cuando `isConnected[i][j] == 1`. Al final, `dsu.components()` es la respuesta. Coste $O(n^2 \alpha(n))$.

```rust
// src/ej_leetcode_547.rs
//! LeetCode 547 — Number of Provinces.

use crate::dsu::Dsu;

pub fn find_circle_num(is_connected: &[Vec<i32>]) -> i32 {
    let n = is_connected.len();
    let mut dsu = Dsu::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            if is_connected[i][j] == 1 {
                dsu.union(i, j);
            }
        }
    }
    dsu.components() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caso_ejemplo() {
        let m = vec![
            vec![1, 1, 0],
            vec![1, 1, 0],
            vec![0, 0, 1],
        ];
        assert_eq!(find_circle_num(&m), 2);
    }
}
```

### Ejercicio 2 — Redundant Connection (LeetCode 684)

Un árbol de $n$ nodos recibe una arista extra, formando exactamente un ciclo. Devuelve la arista que puede eliminarse para recuperar el árbol.

**Solución**: insertamos aristas con DSU; la primera que cierre un ciclo (es decir, `union` devuelva `false`) es la respuesta.

```rust
// src/ej_leetcode_684.rs
//! LeetCode 684 — Redundant Connection.

use crate::dsu::Dsu;

pub fn find_redundant_connection(edges: &[(usize, usize)]) -> (usize, usize) {
    let n = edges.len();
    let mut dsu = Dsu::new(n + 1);
    for &(u, v) in edges {
        if !dsu.union(u, v) {
            return (u, v);
        }
    }
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caso_ejemplo() {
        let edges = vec![(1, 2), (1, 3), (2, 3)];
        assert_eq!(find_redundant_connection(&edges), (2, 3));
    }
}
```

### Ejercicio 3 — Bridges en un grafo de red

Una red de 6 routers tiene enlaces: `0-1, 1-2, 2-0, 2-3, 3-4, 4-5, 5-3`. ¿Qué enlaces son puentes?

**Solución**: con el código de §7.7, los puentes son `(3,4)` y `(4,5)`. Los puntos de articulación son `2` y `3` (eliminar cualquiera desconecta la red). El nodo `2` desconecta el triángulo `0-1-2` del resto; el nodo `3` desconecta el triángulo `3-4-5` de todo.

## 7.10 Ejercicios propuestos

1. **(F) LeetCode 1971 — Find if Path Exists in Graph**. Dados $n$ vértices y un array de aristas, determina si existe camino entre `source` y `destination`. Aplica DSU.
2. **(M) Accounts Merge**. Fusiona cuentas que comparten emails; modela cada email como nodo y cada cuenta como un *set* inicial. Usa DSU y devuelve listas de cuentas fusionadas.
3. **(M) LeetCode 1319 — Number of Operations to Make Network Connected**. Dadas $n$ máquinas y cables, calcula cuántas reconexiones se necesitan para que la red esté totalmente conexa. Pista: cables sobrantes = $E - (n - \text{components})$.
4. **(D) LeetCode 1192 — Critical Connections in a Network**. Generalización del Ejercicio 3 con $n$ hasta $10^5$. Aplica el algoritmo de Tarjan descrito en §7.7.
5. **(D) 2-SAT**. Implementa un solver 2-SAT usando SCC: añade cláusulas $x \lor y$ como dos implicaciones en un grafo, una variable y su negación en SCCs distintas $\Rightarrow$ satisfacible.

## 7.11 Lo que te llevas

- La **DSU** mantiene una partición de $n$ elementos con `find` y `union` casi-constantes ($\alpha(n)$ amortizado).
- **Path compression** aplana árboles; **union by rank/size** mantiene altura $O(\log n)$. Combinadas, dan la cota $\alpha(n)$ que es “constante práctica”.
- En grafos no-dirigidos, las **componentes conexas** se detectan con BFS/DFS o con DSU (en streaming, DSU gana).
- En grafos dirigidos, las **SCC** son el análogo: cada SCC es un maximal subconjunto de vértices mutuamente alcanzables. **Kosaraju** (2 DFS) y **Tarjan** (1 DFS, low-link) son los algoritmos canónicos.
- Los **bridges** y **puntos de articulación** se detectan con low-link: una arista es puente si `low[v] > disc[u]`; un vértice es punto de articulación si tiene un hijo con `low[v] >= disc[u]`.
- `petgraph` expone `connected_components`, `kosaraju_scc`, `tarjan_scc` y `bridges` listos para producción; pero entender las versiones hechas a mano te hace mejor algoritmista.

## 7.12 Ojo, cuidado con…

- **Índices 1-indexados vs 0-indexados**. LeetCode adora los grafos con vértices `1..n`. Adapta la DSU: o bien creas `Dsu::new(n + 1)` y trabajas con 1-based, o ajustas a 0-based al recibir el input. Mezclar ambos es la fuente #1 de *off-by-one* en estos problemas.
- **`union` mutable**. Tanto `find` como `union` toman `&mut self`. Si los metes en un bucle con `for (u, v) in &edges` y luego llamas `dsu.union(*u, *v)`, el borrow checker se quejará. La forma idiomática es `for &(u, v) in edges { dsu.union(u, v); }` (copiando por valor).
- **Recursión profunda en Tarjan**. El DFS recursivo de Tarjan puede reventar la pila en grafos con caminos largos. Para producción, considera una versión iterativa o aumenta el stack size.
- **Confundir SCC con componentes conexas**. SCC es para grafos *dirigidos*: en un dígrafo `a → b`, $a$ alcanza $b$ pero $b$ no alcanza $a$, así que $\{a, b\}$ **no** es una SCC. Las componentes conexas no-dirigidas son otro concepto.
- **Bridges solo en no-dirigidos**. La definición de bridge presupone que la arista puede eliminarse sin dirección. En dígrafo se usa otra noción, *strong bridge*, que es más compleja.
- **Olvidar el caso `n == 0`**. DSU vacío funciona, pero si haces `find(0)` revienta. Comprueba tamaño antes de invocar.

## 7.13 Para profundizar

- **Tarjan, R. E. (1972).** “Depth-first search and linear graph algorithms”. *SIAM J. Comput.*, 1(2). — el paper original de SCC y bridges.
- **Tarjan, R. E. (1975).** “Efficiency of a good but not linear set union algorithm”. *J. ACM*, 22(2). — el análisis de $\alpha(n)$.
- **Kosaraju, S. R. (1978).** “Strong-connectivity algorithm” (unpublished lecture notes).
- **Cormen et al.,** *Introduction to Algorithms* (3ª ed.), Cap. 21 (*Data Structures for Disjoint Sets*) y Cap. 22 (*Elementary Graph Algorithms*).
- Vídeo: WilliamFiset — *Disjoint Set Union* (<https://www.youtube.com/watch?v=8j0MG7jkCxA>).

## 7.14 Pin de batalla

- **Union-Find con path compression + union by rank = casi O(1).** Las dos optimizaciones son obligatorias.
- **Componentes conexas en no-dirigido: BFS/DFS o Union-Find.** Union-Find gana si recibes las aristas en streaming.
- **SCC en dirigido: Tarjan (1 DFS) o Kosaraju (2 DFS).** Tarjan más eficiente, Kosaraju más fácil de entender.
- **Bridges y articulation points se calculan en el mismo DFS de Tarjan.** Aprovéchalo.
- **Si tu grafo cambia dinámicamente, usa `link-cut trees` o `Euler tour trees`.** DSU no soporta borrar aristas.


## 7.15 Si solo lees 30 segundos

Union-Find para conjuntos disjuntos. SCC con Tarjan (low-link) o Kosaraju (2 DFS). Bridges y articulation points en el mismo DFS. `petgraph` lo trae.

## 7.16 Una historia pequeña

Lucía, ingeniera de redes, tenía un problema recurrente: un router se caía y la mitad de la oficina se quedaba sin internet. Su jefe le dijo: "encuentra el router crítico para que tengamos redundancia." Lucía implementó Tarjan, encontró el articulation point, lo duplicó. La red nunca más se cayó. Dos años después, un cortocircuito dejó sin luz toda la planta. Cuando volvió la luz, los routers arrancaron uno a uno, y el duplicado asumió su carga. La oficina siguió trabajando. Lucía recibió un email del CTO: "gracias por hacer tu trabajo antes de que fuera urgente." La mejor clase de héroe es el que evita que el drama ocurra.


---

