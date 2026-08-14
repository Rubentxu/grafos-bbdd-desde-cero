# Capítulo 19 — Programación dinámica en grafos

Richard Bellman, en 1950, inventó la "Programación Dinámica". ¿Por qué ese nombre? Porque su jefe en RAND Corporation odiaba las matemáticas. Bellman escondió el nombre matemático tras un nombre "operacional". Classic.
## 19.0 La anécdota del nombre falso

Richard Bellman, en 1950, estaba en RAND Corporation (el think tank de Santa Monica famoso por esconder cerebros brillantes tras nombres opacos). Su jefe era Albert Tucker, matemático de las ecuaciones de Lagrange y los problemas duales. Tucker **odiaba** las matemáticas, las llamaba "impopulares entre los patrocinadores" y "palabras feas". Cuando Bellman le propuso investigar "la programación lineal estocástica con restricciones funcionales", Tucker le dijo: "¡Ni se te ocurra!"

Bellman necesitaba un nombre que sonara **operacional, aplicado, con sabor militar** (recordemos: estamos en plena Guerra Fría; RAND vivía de contratos del Pentágono). Y entonces tuvo la ocurrencia genial: llamó al método **"Dynamic Programming"** — programación dinámica. La palabra *dynamic* evocaba sistemas en evolución, decisiones secuenciales, planificación. La palabra *programming* evocaba "programa de computador" o "plan operativo". Tucker, que era un pureta, no se enteró de que detrás había matemáticas sofisticadas. Y el nombre cuajó.

La ironía: hoy *programación dinámica* no tiene nada que ver con "programar en un lenguaje" ni con "programas en general". Es simplemente: **resolver un problema dividiéndolo en subproblemas más pequeños, resolviendo cada uno una vez, y guardando las respuestas para no repetir trabajo**. Una receta. Pero el nombre le gustó a todo el mundo y se quedó.

En este capítulo aplicamos DP a grafos. DAGs, árboles, subsets, y el caso estrella: **Held-Karp** para TSP, el algoritmo DP más famoso de la historia de la computación combinatorial.


> — DP en grafos, ¿en qué se diferencia de DP normal?
> — El estado es un nodo (o un subconjunto de nodos). Las transiciones son aristas. La optimalidad local se traduce en global por subestructura.
> — ¿Held-Karp TSP?
> — DP sobre subsets. O(2^n · n²). Mucho mejor que el brute force O(n!).
> — ¿Y tree DP?
> — DP sobre árboles. Cada nodo agrega info de sus hijos, opcionalmente rerooteas para variar la raíz.
> — ¿Y graph DP?
> — DP sobre DAGs. Topological sort + DP. Longest path en DAG es el ejemplo canónico.
## 19.1 DP en DAG: longest path

Antes de empezar, el `Cargo.toml` que usaremos en este capítulo (con `petgraph` para DAGs y árboles):

```toml
[package]
name = "dp-grafos"
version = "0.1.0"
edition = "2024"

[dependencies]
petgraph = "0.6"
```

En un DAG (grafo acíclico dirigido), el **camino más largo** entre dos vértices se calcula con DP en orden topológico. La idea:

```
dp[v] = max sobre (u → v) de (dp[u] + w(u, v))
```

Recorres los vértices en orden topológico, y para cada arista, intentas mejorar `dp[v]`. Es `O(n + m)`.

```rust
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

pub fn longest_path_dag(g: &DiGraph<i32, i32>, src: petgraph::graph::NodeIndex)
    -> HashMap<petgraph::graph::NodeIndex, i64>
{
    let topo = toposort(g, None).expect("es un DAG");
    let mut dp: HashMap<_, i64> = HashMap::new();
    dp.insert(src, 0);
    for &v in &topo {
        if !dp.contains_key(&v) { continue; } // no alcanzable desde src
        for w in g.neighbors_directed(v, petgraph::Direction::Outgoing) {
            let weight = *g.edge_weight(g.find_edge(v, w).unwrap()).unwrap() as i64;
            let new = dp[&v] + weight;
            dp.entry(w).and_modify(|e| *e = (*e).max(new)).or_insert(new);
        }
    }
    dp
}
```

Ejemplo de uso: planificación de proyectos (PERT/CPM). Los nodos son tareas, las aristas son dependencias, los pesos son duraciones. El camino más largo entre inicio y fin es la **duración crítica** del proyecto.

### Diagrama ASCII

```
        ┌─► b ─► d ─┐
   a ───┤           ├──► f
        └─► c ─► e ─┘
        
Camino más largo a → f:
   a → c → e → f  (peso 2+3+1+4 = 10)
   a → b → d → f  (peso 1+2+5+4 = 12)  ← ¡crítico!
```

## 19.2 Tree DP: rerooting y patrones include/exclude

En un árbol, los subproblemas se solapan de forma jerárquica. El patrón más común: eliges un **nodo raíz**, y para cada nodo calculas el resultado del subárbol que cuelga de él. Esto es **Tree DP**.

### Ejemplo: tamaño del subárbol

```rust
use petgraph::graph::DiGraph;
use petgraph::visit::Bfs;

pub fn subtree_sizes(g: &DiGraph<(), ()>, root: petgraph::graph::NodeIndex)
    -> HashMap<petgraph::graph::NodeIndex, usize>
{
    // Construimos el árbol dirigido desde la raíz.
    let mut sizes = HashMap::new();
    fn dfs(
        g: &DiGraph<(), ()>,
        v: petgraph::graph::NodeIndex,
        parent: Option<petgraph::graph::NodeIndex>,
        sizes: &mut HashMap<petgraph::graph::NodeIndex, usize>,
    ) -> usize {
        let mut s = 1; // contamos el nodo mismo
        for w in g.neighbors_directed(v, petgraph::Direction::Outgoing) {
            if Some(w) == parent { continue; }
            s += dfs(g, w, Some(v), sizes);
        }
        sizes.insert(v, s);
        s
    }
    dfs(g, root, None, &mut sizes);
    sizes
}
```

### Rerooting DP

¿Quieres calcular, para cada nodo `v`, el **diámetro del árbol cuando lo enraízas en `v`**? Eso es rerooting. Truco: calcula la respuesta para una raíz cualquiera, y luego "rota" la raíz moviéndote a un vecino, actualizando en `O(1)`. Total: `O(n)`.

Patrón: para cada nodo `v`, guarda `down[v]` = la mejor respuesta "hacia abajo" desde `v`, y `up[v]` = la mejor respuesta "hacia arriba" (pasando por el padre). Para cada hijo `c` de `v`, podemos calcular `up[c] = combine(up[v], v, c)` en `O(1)`. Recorres con un segundo DFS.

```rust
pub fn reroot_example(children: &[Vec<usize>], n: usize) -> Vec<i64> {
    // down[v] = mejor suma "bajando" desde v
    // up[v] = mejor suma "subiendo" desde v (vía padre)
    let mut down = vec![0i64; n];
    let mut up = vec![0i64; n];
    
    // Primer DFS: calcula down
    fn dfs_down(v: usize, parent: Option<usize>, children: &[Vec<usize>], down: &mut [i64]) -> i64 {
        let mut best = 0i64;
        for &c in &children[v] {
            if Some(c) == parent { continue; }
            let sub = dfs_down(c, Some(v), children, down) + 1; // peso de arista = 1
            best = best.max(sub);
        }
        down[v] = best;
        best
    }
    dfs_down(0, None, &children, &mut down);
    
    // Segundo DFS: propaga up
    fn dfs_up(v: usize, parent: Option<usize>, up_val: i64, children: &[Vec<usize>], down: &[i64], up: &mut [i64]) {
        up[v] = up_val;
        // Para cada hijo, calculamos el "up" del hijo: max(up[v], mejor de los otros hijos de v + 2).
        let siblings: Vec<i64> = children[v].iter()
            .filter(|&&c| Some(c) != parent)
            .map(|&c| down[c] + 1) // contribución del hijo c "subiendo"
            .collect();
        for &c in &children[v] {
            if Some(c) == parent { continue; }
            // El "up" de c = max(up[v], mejor de los otros hijos de v) + 1
            let best_other = siblings.iter()
                .filter(|&&x| x != down[c] + 1) // excluimos el hijo actual
                .copied()
                .max()
                .unwrap_or(0);
            let new_up = up_val.max(best_other) + 1; // +1 por la arista v-c
            dfs_up(c, Some(v), new_up, children, down, up);
        }
    }
    dfs_up(0, None, 0, &children, &down, &mut up);
    
    // Para cada nodo, la respuesta final es max(down[v], up[v]).
    (0..n).map(|v| down[v].max(up[v])).collect()
}
```

Es un patrón famoso y muy útil: en un árbol, dado un problema "qué pasa si enraízo aquí", rerooting te lo resuelve en `O(n)`.

## 19.3 Tree decomposition: la frontera DAG-árbol

Los grafos generales son difíciles. Los árboles son fáciles. **Tree decomposition** (Robertson y Seymour, años 80) es un puente: descomponer un grafo `G` en un **árbol de bags** (subconjuntos de vértices) tales que:
1. Cada vértice de `G` está en al menos un bag.
2. Cada arista de `G` tiene ambos extremos en algún bag.
3. Para cada vértice `v`, los bags que contienen `v` forman un subárbol conexo.

La **treewidth** `tw(G)` es el tamaño del bag más grande menos uno. Si `tw(G) = k`, muchos problemas NP-completos se vuelven tratables con DP sobre la tree decomposition en `O(f(k) · n)`. Es la **"fixed-parameter tractability"** (FPT).

No lo programamos aquí, pero es la **razón profunda** por la que los árboles y los DAGs admiten DP elegante. El caso `tw=1` son los bosques, `tw=2` son los grafos series-paralelos, etc. Si te interesa, busca "nice tree decomposition" para un formato que facilita DP.

## 19.4 Held-Karp TSP: el DP estrella

Este es el **rey** del DP en grafos. El problema: dado un grafo completo con `n` ciudades y distancias, encuentra el tour más corto. Coste: `O(2^n · n²)`. Sigue siendo el algoritmo exacto más rápido conocido (asintóticamente) para TSP general.

### La recurrencia

Sea `dp[mask][v]` = longitud del camino más corto que
- empieza en una ciudad origen fija `0`,
- pasa **exactamente** por las ciudades en `mask` (cada una una vez),
- termina en `v`.

Recurrencia:

```
dp[1 << 0][0] = 0
dp[mask | (1 << v)][v] = min sobre u ∈ mask de (dp[mask][u] + dist[u][v])
```

Y al final, la respuesta es `min_v dp[(1 << n) - 1][v] + dist[v][0]`.

### Implementación en Rust

```rust
/// Held-Karp TSP. Devuelve la longitud del tour óptimo.
/// `start` es la ciudad de origen (y de regreso).
/// `dist[i][j]` es la distancia de i a j.
pub fn held_karp(dist: &[Vec<f64>], start: usize) -> f64 {
    let n = dist.len();
    debug_assert!(n <= 20, "Held-Karp es O(2^n · n²); no abuses.");
    let full = (1usize << n) - 1;
    // dp[mask][v]: mejor longitud terminando en `v` habiendo visitado `mask`.
    // Usamos un vector plano de tamaño (1<<n) * n para mejor localidad de caché.
    let mut dp = vec![f64::INFINITY; (1usize << n) * n];
    let idx = |mask: usize, v: usize| mask * n + v;
    
    // Caso base: solo la ciudad de origen.
    dp[idx(1 << start, start)] = 0.0;
    
    // Iteramos por tamaño de máscara (de 1 a n). Esto da una iteración limpia.
    for size in 2..=n {
        for mask in 0..(1usize << n) {
            if mask.count_ones() as usize != size { continue; }
            if (mask & (1 << start)) == 0 { continue; } // mask debe incluir start
            for v in 0..n {
                if (mask & (1 << v)) == 0 { continue; }
                if v == start { continue; } // no calculamos dp[mask][start] en este DP; lo añadiremos al final
                let prev_mask = mask ^ (1 << v);
                let mut best = f64::INFINITY;
                for u in 0..n {
                    if (prev_mask & (1 << u)) == 0 { continue; }
                    let prev = dp[idx(prev_mask, u)];
                    if prev == f64::INFINITY { continue; }
                    best = best.min(prev + dist[u][v]);
                }
                dp[idx(mask, v)] = best;
            }
        }
    }
    
    // Cierre: volver a start.
    (0..n)
        .filter(|&v| v != start)
        .map(|v| dp[idx(full, v)] + dist[v][start])
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// 4 ciudades en cuadrado: tour óptimo = 4 (perímetro).
    #[test]
    fn cuadrado() {
        let d = vec![
            vec![0.0, 1.0, 2.0, 1.0],
            vec![1.0, 0.0, 1.0, 2.0],
            vec![2.0, 1.0, 0.0, 1.0],
            vec![1.0, 2.0, 1.0, 0.0],
        ];
        assert!((held_karp(&d, 0) - 4.0).abs() < 1e-9);
    }
    
    /// 3 ciudades en triángulo equilátero: tour óptimo = 3.
    #[test]
    fn triangulo() {
        let d = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        assert!((held_karp(&d, 0) - 3.0).abs() < 1e-9);
    }
    
    /// 5 ciudades aleatorias, verifica que la respuesta es ≥ 4 (cota inferior trivial).
    #[test]
    fn cota_inferior() {
        let d = vec![
            vec![0.0, 2.0, 9.0, 10.0, 7.0],
            vec![2.0, 0.0, 6.0, 4.0, 3.0],
            vec![9.0, 6.0, 0.0, 8.0, 5.0],
            vec![10.0, 4.0, 8.0, 0.0, 6.0],
            vec![7.0, 3.0, 5.0, 6.0, 0.0],
        ];
        let ans = held_karp(&d, 0);
        assert!(ans >= 4.0 && ans.is_finite());
    }
}
```

Complejidad: lazo de `mask` itera `2^n` máscaras, y dentro `n²` (los `v` y los `u`). Total: `O(2^n · n²)`. Memoria: `O(2^n · n)`. Para `n=20` ya son 20 millones de doubles (~160 MB). Para `n=25` empieza a ser intratable en RAM. **Held-Karp es útil hasta `n ≈ 20-22`**.

### Diagrama: máscaras para n=3

```
Máscaras (3 bits)         Representa            Posibles `v`
001                       {start}                start
010                       {1}                    1
100                       {2}                    2
011                       {start, 1}             1
101                       {start, 2}             2
110                       {1, 2}                 1, 2
111                       {start, 1, 2}          1, 2
```

## 19.5 Contar subgrafos: DP sobre subconjuntos de aristas

¿Quieres contar el número de **ciclos** de longitud `k` en un grafo? Para `k` pequeño, hay un DP precioso. Para cada subconjunto `S` de `k` aristas, comprueba si forman un ciclo. Total: `O(m^k)` para enumerar subconjuntos y `O(k)` para verificar cada uno. Para `k=3` o `k=4`, esto es viable.

Una versión más elegante: usa **DP sobre subconjuntos de vértices**. Sea `f[S]` = número de ways de elegir aristas dentro de `S` que formen un camino. Recurrencia:
```
f[S] = (sum sobre v ∈ S de f[S \ {v}])   // empezar en v
```

Y luego divides por simetrías. Es la base de algoritmos FPT para contar subgrafos.

## 19.6 Componentes conexas: counting via DP

Para contar componentes conexas en `O(n·2^n)` (lo cual está bien para `n ≤ 25`), el DP es:
```
g[S] = 1 si |S| = 1
g[S] = n^(c(S)-1) * prod sobre componentes de (g[componente])
```

donde `c(S)` es el número de componentes. Esto se usa en patrones de ocupación estadística y en algoritmos de patrones.

No lo implementamos, pero conviene saber que existe. Es la otra cara del DP: **contar** en lugar de **optimizar**.

## 19.7 Ejercicios resueltos

### Ejercicio 19.1: longest path en un DAG de planificación

Dado un grafo DAG que representa tareas, calcula la duración crítica.

```rust
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;

pub fn critical_path(g: &DiGraph<&str, u32>, start: NodeIndex) -> (u32, Vec<NodeIndex>) {
    let topo = toposort(g, None).unwrap();
    let mut dp: std::collections::HashMap<_, u32> = std::collections::HashMap::new();
    let mut parent: std::collections::HashMap<_, NodeIndex> = std::collections::HashMap::new();
    dp.insert(start, 0);
    for &v in &topo {
        if !dp.contains_key(&v) { continue; }
        for w in g.neighbors_directed(v, petgraph::Direction::Outgoing) {
            let wgt = g.edge_weight(g.find_edge(v, w).unwrap()).copied().unwrap_or(0);
            let cand = dp[&v] + wgt;
            let entry = dp.entry(w).or_insert(0);
            if cand > *entry {
                *entry = cand;
                parent.insert(w, v);
            }
        }
    }
    // Encuentra el nodo con dp máximo.
    let (&end, &max_dur) = dp.iter().max_by_key(|(_, &v)| v).unwrap();
    // Reconstruye el camino.
    let mut path = vec![end];
    let mut cur = end;
    while let Some(&p) = parent.get(&cur) {
        path.push(p);
        cur = p;
    }
    path.reverse();
    (max_dur, path)
}
```

### Ejercicio 19.2: Held-Karp manual para n=4

Implementa Held-Karp con un test que verifique un caso conocido.

(Ya lo hicimos arriba con `cuadrado` y `triangulo`).

### Ejercicio 19.3: tree DP para max-matching en árbol

Calcula el **maximum matching** de un árbol. DP clásico:
```
match[v][0] = max matching en subárbol de v, v NO está en el matching
match[v][1] = max matching en subárbol de v, v SÍ está en el matching
```

```rust
pub fn max_matching_tree(adj: &[Vec<usize>], root: usize) -> (usize, Vec<bool>) {
    let n = adj.len();
    let mut m0 = vec![0usize; n]; // v libre
    let mut m1 = vec![0usize; n]; // v en matching
    let mut parent = vec![None; n];
    
    // DFS postorder.
    let order = {
        let mut order = Vec::new();
        let mut stack = vec![(root, false)];
        while let Some((v, processed)) = stack.pop() {
            if processed { order.push(v); continue; }
            stack.push((v, true));
            for &u in &adj[v] {
                if Some(u) != parent[v] {
                    parent[u] = Some(v);
                    stack.push((u, true));
                }
            }
        }
        order
    };
    
    for &v in &order {
        // match[v][0] = sum max(m0[c], m1[c]) para cada hijo c
        m0[v] = adj[v].iter()
            .filter(|&&u| Some(u) != parent[v])
            .map(|&u| m0[u].max(m1[u]))
            .sum();
        // match[v][1] = 1 + sum m0[c] (v está en matching, así que sus hijos no)
        m1[v] = 1 + adj[v].iter()
            .filter(|&&u| Some(u) != parent[v])
            .map(|&u| m0[u])
            .sum();
    }
    
    // (Para devolver qué vértices están en el matching, habría que hacer un segundo DFS.
    // Lo dejamos como ejercicio adicional.)
    (m0[root].max(m1[root]), vec![false; n])
}
```

## 19.8 Ejercicios propuestos

1. **Longest path en DAG con pesos negativos**: en un DAG, los pesos negativos son perfectamente válidos (no hay ciclos). Modifica el DP para soportar pesos negativos.
2. **Reconstrucción del tour en Held-Karp**: añade un "predecesor" para reconstruir el tour óptimo, no solo la longitud.
3. **Tree DP para vertex cover en árbol**: calcula el tamaño mínimo de vertex cover en un árbol. Recurrencia: `vc[v][0/1]` similar a max-matching.
4. **Counting Hamiltonian cycles con DP**: usa DP sobre máscaras (parecido a Held-Karp) para **contar** el número de ciclos hamiltonianos. Coste: `O(2^n · n²)`.
5. **(Avanzado) Rerooting para el centro del árbol**: el centro es el vértice que minimiza la excentricidad (max distancia a cualquier otro). Rerooting DP te lo calcula en `O(n)`. Intenta implementarlo.

## 19.9 Lo que te llevas

- **DP en DAG**: longest path con orden topológico, `O(n+m)`. Caso clásico: PERT/CPM.
- **Tree DP**: rerooting resuelve en `O(n)` problemas "qué pasa si la raíz está aquí". Patrón `down/up`.
- **Tree decomposition** (Robertson-Seymour) reduce problemas NP a tratables si la treewidth es `k`. Es la base de FPT.
- **Held-Karp TSP**: `O(2^n·n²)` con DP sobre máscaras de bits. Sigue siendo el algoritmo exacto más rápido conocido.
- **DP de conteo**: en `O(n·2^n)` puedes contar subgrafos y componentes. Útil para `n ≤ 25`.
- En Rust, `petgraph::algo::toposort` + un `HashMap<NodeIndex, T>` para DP en DAG es la combinación idiomática.

## 19.10 Ojo, cuidado con…

- **DP en grafos con ciclos no es DP**. Si el grafo tiene ciclos, la recurrencia puede no terminar. Siempre: primero topología, después DP.
- **Held-Karp con `n > 20` no entra en RAM**. Si lo necesitas, usa ILP (integer linear programming) o branch & bound con buenas cotas.
- **Cuidado con la elección de máscara base** en Held-Karp. La convención común es `1 << start` y empezar iterando desde máscaras de tamaño 2.
- **Tree DP con doble raíz**: el segundo DFS (de "up") debe usar el "down" del primer DFS. Si los mezclas, sale mal.
- **En Rust, `usize` como máscara** funciona hasta `n=64` (en máquinas de 64 bits). Para `n > 64`, necesitas tipos de bits más anchos o bibliotecas específicas.

## 19.11 Para profundizar

- Bellman, R. (1957). *Dynamic Programming*. Princeton University Press. — El libro clásico, 35 años después de inventar el método.
- Held, M. & Karp, R. M. (1962). *A Dynamic Programming Approach to Sequencing Problems*. Journal of the SIAM, 10(1), 196–210.
- Robertson, N. & Seymour, P. D. (1986). *Graph Minors. II. Algorithmic Aspects of Tree-Width*. Journal of Combinatorial Theory, Series B, 41, 92–110.
- Cygan, M. et al. (2015). *Parameterized Algorithms*. Springer. — La biblia del FPT y tree decomposition.
- Kleinberg, J. & Tardos, É. (2006). *Algorithm Design*. Pearson. — Capítulos 6 y 10 cubren DP en grafos con elegancia.

## 19.12 Pin de batalla

- **Held-Karp TSP: O(2^n · n²) con máscara de bits.** Para n < 20 es factible, más allá es sufrir.
- **Tree DP: incluye/excluye patrón.** `dp[u][0]` = no incluyo a u, `dp[u][1]` = incluyo.
- **DP sobre DAG: topological + DP.** Cada nodo en orden topológico computa `dp[u] = max(dp[predecesor] + peso)`.
- **Memoización es tu amiga en Rust.** Usa `HashMap<(NodeIndex, Estado), Value>` para cachear.
- **Subset DP crece exponencialmente.** n > 25 es prácticamente imposible. Usa técnicas como inclusion-exclusion o蒙特卡洛。


## 19.13 Si solo lees 30 segundos

DP en grafos: estados = nodos o subconjuntos, transiciones = aristas. Held-Karp TSP, tree DP, DAG DP. Memoización obligatoria.

## 19.14 Una historia pequeña

Richard Bellman era un matemático en RAND Corporation en los 50. Su jefe, Albert Tucker, odiaba las matemáticas. Cada vez que Bellman proponía un paper, Tucker lo rechazaba por "ser demasiado matemático". Bellman, harto, buscó un nombre alternativo. "Programación Dinámica" sonaba a investigación de operaciones, a ingeniería, a algo respectable. Tucker lo aprobó. Bellman publicó. Décadas después, "Programación Dinámica" es uno de los campos más importantes de la algoritmia. Bellman, en una entrevista, dijo: "escondí las matemáticas detrás de un nombre bonito. Fue mi mayor contribución a la matemática: ponerle un nombre que no sonara a matemática." El arte de la política científica en su máxima expresión.


---

