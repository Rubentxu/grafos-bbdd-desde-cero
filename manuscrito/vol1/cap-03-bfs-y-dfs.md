# Capítulo 3 — BFS y DFS

Dos algoritmos. Uno te dice todo lo que puedes tocar desde una habitación de tu casa sin salir. El otro te dice cómo salir sin pisar la misma baldosa dos veces. Los dos se llaman igual, los dos son grafos, y los dos caben en 30 líneas de código. Bienvenido a BFS y DFS.
## 3.0 La anécdota de la esquina

Claude Shannon, el padre de la teoría de la información, era un tipo peculiar. En los ratos libres que le dejaba su trabajo en los Bell Labs, en 1949 publicó "Communication Theory of Secrecy Systems" y, casi de pasada, "A Mathematical Theory of Communication". Pero lo que nos interesa ahora es algo más concreto: a Shannon le gustaban los laberintos.

Resulta que Shannon, además de matemático brillante, era un consumado malabarista, montaba en monociclo y construía robots-juguete que resolvían laberintos solos. ¿Cómo? Con uno de los algoritmos que vamos a ver en este capítulo. El truco era simple: el robot seguía siempre la pared de la derecha. Garantía: si el laberinto está bien conectado, antes o después sales. Eso, queridos amigos, es un caso particular de **DFS** (búsqueda en profundidad): te metes por un pasillo hasta el fondo, y si no hay salida, "retrocedes" y pruebas otro.

Poco después, a finales de los 50, E. F. Moore (1959) y, de forma independiente, C. Y. Lee (1961) formalizaron el algoritmo BFS para encontrar el camino más corto en laberintos. Lee, ingeniero de Bell Labs, publicó "An Algorithm for Path Connections and Its Applications" en 1961, donde el BFS aparece por primera vez tal y como lo conocemos. El DFS tiene raíces aún más antiguas, en los trabajos de Pierre-François Lévy y Charles Pierre Trémaux del siglo XIX, que lo usaban para salir de… laberintos. Cómo no.


> — Oye, ¿BFS o DFS para un sudoku?
> — DFS. Sudoku es búsqueda en profundidad: prueba una opción, si no funciona, retrocede.
> — ¿Y para encontrar a alguien en LinkedIn?
> — BFS. Tu red de segundo grado es un nivel más ancho que profundo. Si tu amigo conoce al CEO de Google, quieres saberlo en 2 saltos, no en 8.
> — ¿Y si no sé cuál usar?
> — BFS por defecto. Casi siempre. Y si te equivocas, mides y cambias.
## 3.1 BFS: la ola expansiva

Imagina que tiras una piedra a un estanque. Las ondas se expanden en círculos concéntricos. Eso es BFS: empiezas en un vértice, y "visitas" primero todos los que están a distancia 1, luego a distancia 2, luego a distancia 3, etc.

**Pseudolenguaje:**

```
BFS(grafo, inicio):
  cola = [inicio]
  visitado = {inicio}
  while cola no vacía:
    v = cola.desencolar()
    procesar(v)
    for w in vecinos(v):
      if w no en visitado:
        visitado.add(w)
        cola.encolar(w)
```

**Propiedades clave:**

- Encuentra el camino más corto (en número de aristas) desde el inicio a cualquier otro vértice en grafos no ponderados.
- Usa una **cola** (FIFO, first-in first-out).
- Tiempo: O(V + E).

## 3.2 BFS en Rust puro

```rust
// src/lib.rs
use std::collections::{HashSet, VecDeque};

/// Realiza un BFS desde `inicio` en un grafo dado por su lista de adyacencia.
/// Devuelve el orden en que se visitan los vértices.
pub fn bfs(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    let mut visitados: HashSet<u32> = HashSet::new();
    let mut cola: VecDeque<u32> = VecDeque::new();
    let mut orden: Vec<u32> = Vec::new();

    cola.push_back(inicio as u32);
    visitados.insert(inicio as u32);

    while let Some(v) = cola.pop_front() {
        orden.push(v);
        for &w in &adj[v as usize] {
            if !visitados.contains(&w) {
                visitados.insert(w);
                cola.push_back(w);
            }
        }
    }
    orden
}
```

Y los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Grafo de ejemplo:
    ///     0 - 1 - 2
    ///     |   |
    ///     3 - 4
    fn grafo_ejemplo() -> Vec<Vec<u32>> {
        vec![
            vec![1, 3],       // 0
            vec![0, 2, 4],    // 1
            vec![1],          // 2
            vec![0, 4],       // 3
            vec![1, 3],       // 4
        ]
    }

    #[test]
    fn bfs_desde_0() {
        let g = grafo_ejemplo();
        let orden = bfs(&g, 0);
        // Nivel 0: {0}
        // Nivel 1: {1, 3}
        // Nivel 2: {2, 4}
        assert_eq!(orden, vec![0, 1, 3, 2, 4]);
    }

    #[test]
    fn bfs_visita_todos() {
        let g = grafo_ejemplo();
        let orden = bfs(&g, 0);
        assert_eq!(orden.len(), 5); // visitamos los 5 vértices
    }
}
```

Vamos a hacer un ejemplo paso a paso con el grafo de arriba. Empezamos en 0.

| Paso | Cola | Visitados | Procesado |
|---|---|---|---|
| 0 | [0] | {0} | — |
| 1 | [] | {0} | 0 |
| 2 | [1, 3] | {0,1,3} | — |
| 3 | [3] | {0,1,3} | 1 |
| 4 | [3, 2, 4] | {0,1,3,2,4} | — |
| 5 | [2, 4] | {0,1,3,2,4} | 3 |
| 6 | [4] | {0,1,3,2,4} | 2 |
| 7 | [] | {0,1,3,2,4} | 4 |

Orden final: `[0, 1, 3, 2, 4]`. Como ves, los de nivel 1 (1 y 3) van antes que los de nivel 2 (2 y 4).

## 3.3 DFS: te metes hasta el fondo

Imagina que estás en un laberinto y solo puedes avanzar, sin volver atrás. Te metes por el primer pasillo, y cuando llegas a un cruce sigues el primero que veas. Si es un callejón sin salida, "deshaces" lo andado (eso es la recursión volviendo) y pruebas el siguiente cruce. Eso es DFS.

**Pseudolenguaje:**

```
DFS(grafo, v, visitado):
  visitado.add(v)
  procesar(v)
  for w in vecinos(v):
    if w no en visitado:
      DFS(grafo, w, visitado)
```

**Propiedades:**

- No garantiza el camino más corto.
- Usa una **pila** (LIFO, last-in first-out), ya sea explícita o con la pila de llamadas recursivas.
- Tiempo: O(V + E).

**Analogía de la pizza (sin pizzas reales, lo prometo):** Imagina que BFS es comer pizza por niveles: te comes todos los trozos del borde exterior primero, luego el siguiente anillo, etc. DFS es comerte un único trozo yendo hasta el centro en línea recta, comerte todo un cuadrante, y luego volver para hacer el siguiente cuadrante. (Vale, sí, los dos llegan al centro, pero llegan en distinto orden.)

## 3.4 DFS en Rust: recursivo e iterativo

```rust
use std::collections::HashSet;

/// DFS recursivo. ¡Cuidado con grafos profundos: desborda la pila!
pub fn dfs_recursivo(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    fn visitar(adj: &[Vec<u32>], v: u32, visitados: &mut HashSet<u32>, orden: &mut Vec<u32>) {
        visitados.insert(v);
        orden.push(v);
        for &w in &adj[v as usize] {
            if !visitados.contains(&w) {
                visitar(adj, w, visitados, orden);
            }
        }
    }
    let mut visitados = HashSet::new();
    let mut orden = Vec::new();
    visitar(adj, inicio as u32, &mut visitados, &mut orden);
    orden
}

/// DFS iterativo, con pila explícita. No desborda (salvo que la pila sea enorme).
pub fn dfs_iterativo(adj: &[Vec<u32>], inicio: usize) -> Vec<u32> {
    let mut visitados: HashSet<u32> = HashSet::new();
    let mut pila: Vec<u32> = vec![inicio as u32];
    let mut orden: Vec<u32> = Vec::new();

    while let Some(v) = pila.pop() {
        if visitados.contains(&v) {
            continue;
        }
        visitados.insert(v);
        orden.push(v);
        // Metemos los vecinos en orden inverso para que el comportamiento
        // sea equivalente a la versión recursiva.
        for &w in adj[v as usize].iter().rev() {
            if !visitados.contains(&w) {
                pila.push(w);
            }
        }
    }
    orden
}
```

Y los tests:

```rust
#[cfg(test)]
mod tests_dfs {
    use super::*;

    fn grafo_ejemplo() -> Vec<Vec<u32>> {
        vec![
            vec![1, 3],       // 0
            vec![0, 2, 4],    // 1
            vec![1],          // 2
            vec![0, 4],       // 3
            vec![1, 3],       // 4
        ]
    }

    #[test]
    fn dfs_recursivo_visita_todos() {
        let g = grafo_ejemplo();
        let orden = dfs_recursivo(&g, 0);
        assert_eq!(orden.len(), 5);
        // Una de las posibles: 0, 1, 2, 4, 3
    }

    #[test]
    fn dfs_iterativo_visita_todos() {
        let g = grafo_ejemplo();
        let orden = dfs_iterativo(&g, 0);
        assert_eq!(orden.len(), 5);
    }
}
```

**¿Por qué dos versiones?** La recursiva es elegante y corta, pero cada llamada anidada usa el call stack. En Rust, el call stack por defecto es de unos 8 MB. Si tu grafo es muy profundo (miles de vértices en cadena), puedes quedarte sin stack. La iterativa usa el heap (`Vec`), que tiene memoria de sobra. **Regla de oro:** en producción, usa la iterativa.

## 3.5 Con `petgraph`: BFS y DFS en una línea

Petgraph viene con varios "visitantes" que son iteradores sobre el grafo en distintos órdenes. Lo más cómodo es `Bfs` y `Dfs`:

```rust
use petgraph::graph::{Graph, UnGraph};
use petgraph::visit::{Bfs, Dfs};
use petgraph::graph::NodeIndex;
use petgraph::Undirected;

pub fn bfs_petgraph(g: &Graph<(), (), Undirected>, inicio: NodeIndex) -> Vec<NodeIndex> {
    let mut bfs = Bfs::new(g, inicio);
    let mut visitados = Vec::new();
    while let Some(n) = bfs.next(g) {
        visitados.push(n);
    }
    visitados
}

pub fn dfs_petgraph(g: &Graph<(), (), Undirected>, inicio: NodeIndex) -> Vec<NodeIndex> {
    let mut dfs = Dfs::new(g, inicio);
    let mut visitados = Vec::new();
    while let Some(n) = dfs.next(g) {
        visitados.push(n);
    }
    visitados
}
```

Y un test que lo prueba todo:

```rust
#[cfg(test)]
mod tests_pet {
    use super::*;

    #[test]
    fn bfs_y_dfs_con_petgraph() {
        let mut g: Graph<(), (), Undirected> = Graph::new_undirected();
        let n0 = g.add_node(());
        let n1 = g.add_node(());
        let n2 = g.add_node(());
        let n3 = g.add_node(());
        g.add_edge(n0, n1, ());
        g.add_edge(n0, n2, ());
        g.add_edge(n1, n3, ());

        let orden_bfs = bfs_petgraph(&g, n0);
        let orden_dfs = dfs_petgraph(&g, n0);

        assert_eq!(orden_bfs.len(), 4);
        assert_eq!(orden_dfs.len(), 4);
    }
}
```

## 3.6 Topological sort: ¿en qué orden estudio las asignaturas?

Imagina que en la carrera tienes que matricularte de Algoritmos II, pero necesitas haber aprobado antes Algoritmos I. Eso es una relación de **precedencia**: se modela con un **grafo dirigido acíclico** (DAG, por sus siglas en inglés: Directed Acyclic Graph). El **orden topológico** es una ordenación de los vértices tal que para toda arista u→v, u aparece antes que v. Es como decir "primero lo previo, luego lo posterior".

Petgraph te lo da hecho:

```rust
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

let mut g: DiGraph<&str, ()> = DiGraph::new();
let a = g.add_node("Algoritmos I");
let b = g.add_node("Algoritmos II");
let c = g.add_node("Compiladores");
g.add_edge(a, b, ()); // Algoritmos I -> Algoritmos II
g.add_edge(b, c, ()); // Algoritmos II -> Compiladores

let orden = toposort(&g, None).expect("¡No hay ciclos!");
// orden == [a, b, c]
```

`toposort` devuelve un error si hay un ciclo. Si no, te da un vector con los nodos en orden válido. **Aplicaciones reales:** ordenación de tareas, compilación de módulos, planificación de proyectos.

## 3.7 Componentes conexos: ¿cuántas "islas" hay?

```rust
use petgraph::algo::connected_components;
use petgraph::graph::UnGraph;

let mut g: UnGraph<(), ()> = UnGraph::new_undirected();
let a = g.add_node(());
let b = g.add_node(());
let c = g.add_node(());
let d = g.add_node(());
g.add_edge(a, b, ()); // componente 1
g.add_edge(c, d, ()); // componente 2

let n = connected_components(&g);
assert_eq!(n, 2);
```

¡Magia! Una línea y te dice cuántas "islas" tiene tu grafo. Esto es lo que usarías para el famoso problema "Number of Islands" (leer una matriz de 0s y 1s y contar cuántas islas de 1s hay).

## 3.8 Ejercicios resueltos

**Ejercicio 3.1 (F).** Aplica BFS y DFS al siguiente grafo desde el vértice 0:

```
0 - 1 - 2
|       |
3 ----- 4
```

Aristas: 0-1, 1-2, 0-3, 3-4, 2-4.

**Solución.**

- **BFS desde 0:** nivel 0: {0}; nivel 1: {1, 3}; nivel 2: {2, 4}. Orden: 0, 1, 3, 2, 4.
- **DFS desde 0** (suponiendo vecinos en orden numérico): 0 → 1 → 2 → 4 → 3. Orden: 0, 1, 2, 4, 3.

**Ejercicio 3.2 (M) — Número de islas.** Dada una matriz 2D de 0s y 1s, cuenta cuántas "islas" de 1s hay. Una isla es un grupo de 1s conectados en 4 direcciones (arriba, abajo, izquierda, derecha).

```rust
pub fn num_islas(matriz: &[Vec<u8>]) -> usize {
    if matriz.is_empty() || matriz[0].is_empty() {
        return 0;
    }
    let (n, m) = (matriz.len(), matriz[0].len());
    let mut visitado = vec![vec![false; m]; n];
    let mut count = 0;

    fn dfs(m: &[Vec<u8>], vis: &mut [Vec<bool>], i: usize, j: usize, n: usize, cols: usize) {
        if i >= n || j >= cols || vis[i][j] || m[i][j] == 0 {
            return;
        }
        vis[i][j] = true;
        if i > 0 { dfs(m, vis, i - 1, j, n, cols); }
        if i + 1 < n { dfs(m, vis, i + 1, j, n, cols); }
        if j > 0 { dfs(m, vis, i, j - 1, n, cols); }
        if j + 1 < cols { dfs(m, vis, i, j + 1, n, cols); }
    }

    for i in 0..n {
        for j in 0..m {
            if matriz[i][j] == 1 && !visitado[i][j] {
                count += 1;
                dfs(matriz, &mut visitado, i, j, n, m);
            }
        }
    }
    count
}

#[test]
fn test_islas() {
    let m = vec![
        vec![1, 1, 0, 0, 0],
        vec![1, 1, 0, 0, 0],
        vec![0, 0, 1, 0, 0],
        vec![0, 0, 0, 1, 1],
    ];
    assert_eq!(num_islas(&m), 3);
}
```

**Ejercicio 3.3 (M) — ¿Es bipartito?** Un grafo es bipartito si puedes pintar sus vértices de dos colores sin que dos adyacentes compartan color. Escribe una función que lo diga.

```rust
use std::collections::VecDeque;

pub fn es_bipartito(adj: &[Vec<u32>]) -> bool {
    let n = adj.len();
    let mut color: Vec<Option<u8>> = vec![None; n];
    for inicio in 0..n {
        if color[inicio].is_some() { continue; }
        let mut cola = VecDeque::new();
        cola.push_back(inicio);
        color[inicio] = Some(0);
        while let Some(v) = cola.pop_front() {
            for &w in &adj[v] {
                let c = color[v].unwrap();
                match color[w as usize] {
                    Some(c2) if c2 == c => return false,
                    Some(_) => {}
                    None => {
                        color[w as usize] = Some(1 - c);
                        cola.push_back(w as usize);
                    }
                }
            }
        }
    }
    true
}

#[test]
fn bipartito_clasico() {
    // Triángulo -> NO bipartito
    let g = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
    assert!(!es_bipartito(&g));
    // Cuadrado -> bipartito
    let g = vec![vec![1, 3], vec![0, 2], vec![1, 3], vec![0, 2]];
    assert!(es_bipartito(&g));
}
```

**Truco:** si en algún momento un vértice tiene el mismo color que un adyacente, NO es bipartito. BFS te lo detecta rápido.

**Ejercicio 3.4 (M) — Resolver un laberinto.** Dada una matriz donde 0 = camino y 1 = muro, ¿hay un camino desde la esquina (0,0) hasta (n−1, m−1)?

```rust
use std::collections::VecDeque;

pub fn hay_camino(maze: &[Vec<u8>]) -> bool {
    if maze.is_empty() || maze[0][0] == 1 { return false; }
    let (n, m) = (maze.len(), maze[0].len());
    let mut vis = vec![vec![false; m]; n];
    let mut cola: VecDeque<(usize, usize)> = VecDeque::new();
    cola.push_back((0, 0));
    vis[0][0] = true;
    while let Some((i, j)) = cola.pop_front() {
        if (i, j) == (n - 1, m - 1) { return true; }
        let dirs: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (di, dj) in dirs {
            let ni = (i as i32 + di) as usize;
            let nj = (j as i32 + dj) as usize;
            if ni < n && nj < m && !vis[ni][nj] && maze[ni][nj] == 0 {
                vis[ni][nj] = true;
                cola.push_back((ni, nj));
            }
        }
    }
    false
}
```

## 3.9 Ejercicios propuestos

1. **(F)** Dado un grafo y un vértice, devuelve el árbol BFS (cada vértice con su padre).
2. **(F)** Implementa BFS desde cada vértice y cuenta cuántos están a distancia par del inicio.
3. **(M)** Detecta si un grafo no dirigido tiene un ciclo usando DFS.
4. **(M)** Dado un árbol enraizado, calcula la altura con DFS.
5. **(M)** Resuelve el problema de "número de islas" en 8 direcciones (también las diagonales).
6. **(D)** Implementa un algoritmo para encontrar el **puente** (bridge) de un grafo: una arista que, al eliminarla, desconecta el grafo.
7. **(D)** Implementa el algoritmo de **Tarjan** para encontrar componentes fuertemente conexos en un grafo dirigido.

## 3.10 Lo que te llevas

- **BFS** (anchura) usa una cola, garantiza camino más corto en grafos no ponderados, O(V+E).
- **DFS** (profundidad) usa una pila, no garantiza el camino más corto pero es más sencillo para "explorar todo".
- En Rust, el `VecDeque` del stdlib es la cola, y un `Vec` con `push`/`pop` o la recursión sirven como pila.
- **Cuidado con la recursión profunda** en DFS: en grafos enormes puede desbordar el call stack. Usa la versión iterativa en producción.
- `petgraph` te da `Bfs` y `Dfs` como visitors; también te regala `toposort` y `connected_components` listos para usar.
- Aplicaciones: orden topológico (asignaturas con prerrequisitos), componentes conexos (islas), bipartito (matching), resolución de laberintos.

## 3.11 Ojo, cuidado con…

- **Asumir que BFS y DFS dan el mismo orden.** Para nada. BFS garantiza orden por niveles, DFS va "al fondo" antes de explorar otros caminos.
- **Usar recursión con grafos grandes.** En Rust, el call stack tiene un límite. Para grafos con más de unas decenas de miles de vértices en cadena, la versión iterativa es obligatoria.
- **No marcar visitados antes de encolar en BFS.** Si lo haces al desencolar, puedes encolar el mismo vértice varias veces. Márcalo al encolar y ahorrarás tiempo y memoria.
- **Confundir grafo dirigido con no dirigido en bipartito.** El algoritmo es esencialmente el mismo, pero asegúrate de iterar sobre las aristas correctas.
- **Olvidar inicializar la cola en componentes conexos.** Si el grafo tiene varios componentes, el BFS "desde un solo vértice" no los cubre todos. O bien lanzas BFS desde cada vértice no visitado, o usas `connected_components` de petgraph.

## 3.12 Para profundizar

1. Moore, E. F. (1959). "The shortest path through a maze". *Proc. International Symposium on Switching Theory*.
2. Lee, C. Y. (1961). "An Algorithm for Path Connections and Its Applications". *IRE Transactions on Electronic Computers*.
3. Tarjan, R. (1972). "Depth-first search and linear graph algorithms". *SIAM Journal on Computing*.
4. Cormen et al. (2009). *Introduction to Algorithms*, capítulo 22 (BFS) y 23 (DFS).
5. Sedgewick, R. (2011). *Algorithms*, §4.1–4.2.
6. Hopcroft, J., Tarjan, R. (1973). "Efficient algorithms for graph manipulation". *Communications of the ACM*.

## 3.13 Pin de batalla

- **BFS encuentra el camino más corto en grafos no ponderados.** Si quieres "el más corto" y los pesos son 1, es tu algoritmo.
- **DFS es recursivo por naturaleza.** Si tu grafo tiene miles de nodos en línea, te va a explotar la pila. Usa versión iterativa con stack explícito.
- **`petgraph` ya los trae**: `Bfs` y `Dfs` son iteradores. Más fácil imposible.
- **Colorea el grafo (blanco/gris/negro) para detectar ciclos en DFS.** Si ves una back edge, hay ciclo.
- **Para grafos dirigidos, `petgraph::algo::toposort` es tu amigo.** Implementa Kahn en 5 líneas. Ya te lo ha hecho alguien.


## 3.14 Si solo lees 30 segundos

BFS = anchura, encuentra el camino más corto en no ponderados. DFS = profundidad, sirve para backtracking, topological sort y detección de ciclos.

## 3.15 Una historia pequeña

Carmen, una estudiante de bachillerato, estaba haciendo un trabajo sobre el laberinto del Minotauro. Su profesora le dijo: "modela el laberinto como un grafo y aplícale BFS." Carmen no sabía qué era un BFS. Su hermano mayor, programador, le escribió 15 líneas de Python en una servilleta del bar. Carmen las pasó a Rust, las ejecutó, y en 2 segundos tenía el camino más corto del laberinto. Presentó el trabajo al día siguiente. La profesora le puso un 10. "Y ni siquiera sabía programar," dijo Carmen. Su hermano le respondió: "ya sabes, solo que no lo sabías."


---

