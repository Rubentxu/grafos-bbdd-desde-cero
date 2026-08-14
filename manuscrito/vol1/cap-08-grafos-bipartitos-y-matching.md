# Capítulo 8 — Grafos bipartitos y Matching

El algoritmo se llama "húngaro" por un capricho geográfico de 1955. La paternidad real es soviético-alemana-estadounidense. A veces los algoritmos tienen más nacionalidades que un formulario de hacienda.
## 8.0 La anécdota del algoritmo que era de tres (o cuatro)

Cuenta la historia que en los años 50 tres equipos, en paralelo y sin hablarse entre sí, dieron con variantes del mismo algoritmo de **asignación de coste mínimo** en un grafo bipartito: el **denominado “algoritmo húngaro”**.

En **1953** aparecieron dos artículos: uno del estadounidense **James Munkres** y otro del holandés **Geert König** (que ya tenía resultados previos desde 1916 sobre grafos bipartitos, conocidos como “teorema de König”). Pero la verdadera paternidad se la lleva, irónicamente, el matemático soviético **Lavrentiy Kantorovich**, que en 1939 ya había descrito una técnica equivalente para problemas de transporte en la planificación de la producción industrial de la URSS. Y si vamos más atrás, el matemático alemán **Carl Gustav Jacobi** en 1840 ya había publicado una idea muy parecida usando matrices.

El algoritmo, en cualquiera de sus versiones, resuelve el mismo problema: dadas $n$ tareas y $n$ trabajadores con un coste $c_{ij}$ por asignar el trabajador $i$ a la tarea $j$, encuentra la asignación de coste mínimo. Es $O(n^3)$ y se hace manipulando matrices con “operaciones de topping” (sumar y restar filas/columnas), **sin un solo ordenador**, porque en los años 30 no había. La única herramienta era papel, lápiz, y la idea feliz de que el optimal solution vive en una submatriz cuadrada de ceros minimales.

La injusticia histórica: el algoritmo se llama **húngaro** por un detalle nimio: en 1955 Harold Kuhn lo presentó en un congreso en Budapest, llamó al método “el algoritmo húngaro” por el parecido con los trabajos de König y Dénes Kőnig, y el nombre se quedó. Kuhn reconoció más tarde la prioridad soviética. La moraleja: el nombre no siempre es quien inventó el algoritmo.


> — ¿Cuál es la diferencia entre matching maximal y máximo?
> — Maximal: no puedes añadir más aristas sin violar la propiedad. Máximo: tiene el mayor número posible. Un maximal puede NO ser máximo.
> — ¿Y Kuhn?
> — DFS augmenting path. O(n*m) en el peor caso, pero en práctica O(n+m).
> — ¿Y Hopcroft-Karp?
> — BFS + DFS para augmenting paths en bloque. O(E·√V). Mucho más rápido en grafos grandes.
> — ¿Y Hungarian con pesos?
> — O(n³). Para matching ponderado. No es lo mismo que el bipartito sin pesos.
## 8.1 Grafos bipartitos

Un grafo $G = (V, E)$ es **bipartito** si $V$ puede particionarse en dos conjuntos $L$ y $R$ tales que toda arista conecta un vértice de $L$ con uno de $R$. Esta clase modela relaciones “naturalmente” binarias: usuarios-tareas, estudiantes-escuelas, películas-actores, servidores-clientes, palabras-significados.

**Caracterizaciones equivalentes**:

- $G$ es bipartito $\iff$ es **2-coloreable** (sus vértices admiten una coloración con 2 colores sin que vértices adyacentes compartan color).
- $G$ es bipartito $\iff$ **no contiene ciclos impares** (teorema de König).
- En grafos dirigidos, una versión adaptada exige un *cover* por dos *order ideals*.

**Palabras clave** que vamos a usar en este capítulo:
- **Bipartito**: grafo que admite partición en dos conjuntos sin aristas internas.
- **2-coloración**: colorear vértices con dos colores de modo que los adyacentes tengan color distinto.
- **Matching**: subconjunto de aristas que no comparte vértices.
- **Maximal vs máximo**: maximal = no se puede extender localmente; máximo = óptimo global.
- **Augmenting path**: camino alternante (en/no-en matching) entre dos vértices libres.
- **Teorema de Berge**: matching máximo $\iff$ no hay augmenting path.
- **Kuhn (DFS augmenting)**: $O(V \cdot E)$ en el peor caso; $O(E)$ en la práctica con optimizaciones.
- **Hopcroft-Karp**: $O(E \sqrt{V})$ con BFS+DFS por capas.
- **Hungarian algorithm**: $O(n^3)$ para asignación con pesos en grafo bipartito completo.
- **Equality subgraph**: subgrafo de aristas con peso = `u[i] + v[j]` (potenciales).

## 8.2 Detección por BFS/DFS 2-coloración

Recorremos el grafo; al primer vértice le asignamos color 0, a sus vecinos color 1, etc. Si al propagar encontramos una arista entre dos vértices del mismo color, el grafo no es bipartito. Esto es lineal: $O(V + E)$ con BFS o DFS.

```rust
// src/bipartito.rs
//! Detección de bipartito y 2-coloración.

use std::collections::VecDeque;

/// `Some(color)` si el grafo es bipartito (color[i] ∈ {0, 1});
/// `None` si no lo es.
pub fn es_bipartito(n: usize, adj: &[Vec<usize>]) -> Option<Vec<i32>> {
    let mut color = vec![-1i32; n];
    for start in 0..n {
        if color[start] != -1 {
            continue;
        }
        color[start] = 0;
        let mut q = VecDeque::new();
        q.push_back(start);
        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                if color[v] == -1 {
                    color[v] = 1 - color[u];
                    q.push_back(v);
                } else if color[v] == color[u] {
                    // Arista con mismo color: no bipartito.
                    return None;
                }
            }
        }
    }
    Some(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuadrado_si_es_bipartito() {
        // 0-1-2-3-0: ciclo par.
        let adj = vec![vec![1, 3], vec![0, 2], vec![1, 3], vec![0, 2]];
        let color = es_bipartito(4, &adj).unwrap();
        assert_eq!(color[0], color[2]);
        assert_ne!(color[0], color[1]);
    }

    #[test]
    fn triangulo_no_es_bipartito() {
        // 0-1-2-0: ciclo impar.
        let adj = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        assert!(es_bipartito(3, &adj).is_none());
    }
}
```

`color` representa la partición $\{i : \text{color}[i] = 0\}$ y $\{i : \text{color}[i] = 1\}$.

## 8.3 Definiciones de matching

Sea $G$ bipartito con partición $(L, R)$. Un **matching** $M$ es un subconjunto de aristas sin vértices compartidos:

- **Maximal**: no se puede añadir otra arista sin violar la condición.
- **Máximo** (cardinalidad máxima): $|M|$ es el mayor posible. “Máximo” ≠ “maximal” (todo máximo es maximal, no al revés).
- **Perfecto**: cubre *todos* los vértices; existe solo si $|L| = |R|$ y el grafo admite un *perfect matching* (teorema de Hall).

Un **augmenting path** es un camino que alterna aristas no-en-M y en-M, comienza y termina en vértices *libres* (no cubiertos por $M$). El **teorema de Berge (1957)** afirma que un matching es máximo $\iff$ no existe augmenting path. Esta es la base de los algoritmos de Kuhn y Hopcroft-Karp.

## 8.4 Algoritmo de Kuhn (DFS augmenting path)

**Idea**: para cada vértice de $L$, intenta encontrar un augmenting path mediante un DFS. Si lo encuentra, incrementa el matching. Repetir hasta que ningún vértice libre de $L$ encuentre augmenting path.

```rust
// src/kuhn.rs
//! Maximum bipartite matching por Kuhn (DFS augmenting path).

/// Maximum matching bipartito.
/// `adj[u] = [v1, v2, ...]` con u en L (0..n_left) y v en R (0..n_right).
/// Devuelve un vector `match_right[v] = u` (o -1 si v está libre).
pub fn kuhn(n_left: usize, n_right: usize, adj: &[Vec<usize>]) -> Vec<i32> {
    let mut match_right = vec![-1i32; n_right];

    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        match_right: &mut [i32],
        visited: &mut [bool],
    ) -> bool {
        for &v in &adj[u] {
            if visited[v] {
                continue;
            }
            visited[v] = true;
            // Si v está libre, o si el match actual de v puede reasignarse.
            if match_right[v] == -1 || dfs(match_right[v] as usize, adj, match_right, visited) {
                match_right[v] = u as i32;
                return true;
            }
        }
        false
    }

    for u in 0..n_left {
        let mut visited = vec![false; n_right];
        dfs(u, adj, &mut match_right, &mut visited);
    }
    match_right
}

/// Devuelve la lista de pares (u, v) del matching.
pub fn kuhn_pairs(n_left: usize, n_right: usize, adj: &[Vec<usize>]) -> Vec<(usize, usize)> {
    let m = kuhn(n_left, n_right, adj);
    m.into_iter()
        .enumerate()
        .filter(|(_, u)| *u != -1)
        .map(|(v, u)| (u as usize, v))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_perfecto() {
        // L = {0,1,2,3}, R = {0,1,2,3}
        // Esperado: 4 pares.
        let adj = vec![
            vec![0, 1],
            vec![1, 2, 3],
            vec![0, 2],
            vec![3],
        ];
        let pairs = kuhn_pairs(4, 4, &adj);
        assert_eq!(pairs.len(), 4);
    }

    #[test]
    fn matching_imperfecto() {
        // L = {0,1,2}, R = {0,1,2,3}
        // Esperado: 3 pares.
        let adj = vec![
            vec![0, 1],
            vec![0],
            vec![1, 2],
        ];
        let pairs = kuhn_pairs(3, 4, &adj);
        assert_eq!(pairs.len(), 3);
    }
}
```

**Complejidad**: $O(V \cdot E)$ en el peor caso. Una optimización típica de programación competitiva es reusar el `visited` entre llamadas para reducir el coste.

## 8.5 Hopcroft–Karp O(E·√V)

En lugar de buscar un único augmenting path por vértice, Hopcroft-Karp busca un **set máximo de augmenting paths disjuntos en vértices** en cada fase, mediante BFS + DFS:

1. **BFS** desde todos los vértices libres de $L$: construye capas; un vértice de $R$ está en una capa si su arista está en $M$ o no.
2. **DFS** restringido a las capas: encuentra todos los augmenting paths más cortos posibles.
3. Repite mientras el BFS encuentre un augmenting path.

**Complejidad**: $O(E \sqrt{V})$, notablemente mejor que Kuhn en grafos densos.

```rust
// src/hopcroft_karp.rs
//! Hopcroft-Karp: BFS + DFS por capas.

use std::collections::VecDeque;

const INF: i32 = i32::MAX;

/// Maximum matching bipartito con Hopcroft-Karp.
/// Devuelve `match_left[u]` y `match_right[v]` (índices del opuesto o -1).
pub fn hopcroft_karp(
    n_left: usize,
    n_right: usize,
    adj: &[Vec<usize>],
) -> (Vec<i32>, Vec<i32>) {
    let mut match_left = vec![-1i32; n_left];
    let mut match_right = vec![-1i32; n_right];
    let mut dist: Vec<i32> = vec![0; n_left];

    // BFS: calcula las distancias y detecta si quedan augmenting paths.
    fn bfs(
        n_left: usize,
        adj: &[Vec<usize>],
        match_left: &[i32],
        match_right: &[i32],
        dist: &mut [i32],
    ) -> bool {
        let mut q = VecDeque::new();
        for u in 0..n_left {
            if match_left[u] == -1 {
                dist[u] = 0;
                q.push_back(u);
            } else {
                dist[u] = INF;
            }
        }
        let mut found = false;
        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                let mu = match_right[v];
                if mu != -1 && dist[mu as usize] == INF {
                    dist[mu as usize] = dist[u] + 1;
                    q.push_back(mu as usize);
                }
                if mu == -1 {
                    found = true;
                }
            }
        }
        found
    }

    // DFS: busca augmenting paths respetando las capas calculadas por BFS.
    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        match_left: &mut [i32],
        match_right: &mut [i32],
        dist: &[i32],
    ) -> bool {
        for &v in &adj[u] {
            let mu = match_right[v];
            let next = dist[u] + 1;
            if mu != -1 && dist[mu as usize] != next {
                continue;
            }
            if mu == -1
                || (dist[mu as usize] == next
                    && dfs(mu as usize, adj, match_left, match_right, dist))
            {
                match_left[u] = v as i32;
                match_right[v] = u as i32;
                return true;
            }
        }
        dist[u] = INF;
        false
    }

    while bfs(n_left, adj, &match_left, &match_right, &mut dist) {
        for u in 0..n_left {
            if match_left[u] == -1 {
                dfs(u, adj, &mut match_left, &mut match_right, &dist);
            }
        }
    }

    (match_left, match_right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_perfecto_hk() {
        let adj = vec![
            vec![0, 1],
            vec![1, 2, 3],
            vec![0, 2],
            vec![3],
        ];
        let (ml, _) = hopcroft_karp(4, 4, &adj);
        let size = ml.iter().filter(|&&x| x != -1).count();
        assert_eq!(size, 4);
    }

    #[test]
    fn matching_imperfecto_hk() {
        let adj = vec![
            vec![0, 1],
            vec![0],
            vec![1, 2],
        ];
        let (ml, _) = hopcroft_karp(3, 4, &adj);
        let size = ml.iter().filter(|&&x| x != -1).count();
        assert_eq!(size, 3);
    }
}
```

## 8.6 Hungarian algorithm (outline)

El **algoritmo húngaro** (Kuhn-Munkres) resuelve **asignación de coste mínimo** (o máximo) en grafos bipartitos *completos* con pesos: matching perfecto que minimiza la suma de pesos. Coste $O(V^3)$ o $O(V^2 E)$ según la implementación.

**Bosquejo**:

1. Restar a cada fila su mínimo, luego a cada columna su mínimo: las matrices quedan con al menos un 0 por fila y columna.
2. Cubrir todos los ceros con un mínimo número de líneas horizontales y verticales.
3. Si el número de líneas es $n$, hay asignación óptima. Si no, ajustar la matriz y volver a 2.

En la práctica, en Rust usaríamos la crate [`pathfinding`](https://crates.io/crates/pathfinding) o [`lapjv`](https://crates.io/crates/lapjv) para Hungarian real. La implementación manual son ~150 líneas y se va del alcance de este libro.

```toml
# Cargo.toml
[dependencies]
lapjv = "0.2" # Hungarian en Rust, envoltura sobre la biblioteca C LAPJV.
```

> **Nota**: la crate `lapjv` resuelve **asignación cuadrada** (matching perfecto $n \times n$) en $O(n^3)$. Si tu problema es de matching bipartito general (no cuadrado), usa Hopcroft-Karp + Hungarian por bloques.

## 8.7 Aplicaciones del mundo real

- **Asignación de tareas**: $n$ trabajadores a $n$ trabajos, minimizar coste total (Hungarian) o maximizar tareas completadas (HK).
- **Emparejamiento de vuelos**: tripulaciones a vuelos, maximizing conexiones o minimizando tiempos muertos.
- **Recomendación bipartita**: usuarios-productos, encontrar matchings que maximicen afinidad.
- **Movimiento en tableros**: máximo de no-atacantes en un tablero de ajedrez (problema de las *n reinas* relajado) → matching bipartito en grafo de casillas.
- **Procesamiento de currículums**: asignación de candidatos a ofertas.
- **Matching médico**: residentes a hospitales (NRMP en EE. UU., usa Gale-Shapley estable, que es *otro* matching).

## 8.8 Matching bipartito con `petgraph`

`petgraph` expone `petgraph::algo::greedy_matching` (un matching maximal por heurística) y, a partir de 0.6, herramientas para que combines con tu algoritmo. Para máxima cardinalidad, lo más limpio es construir el grafo bipartito explícito y aplicar tu Hopcroft-Karp/Kuhn.

```toml
# Cargo.toml
[dependencies]
petgraph = "0.6"
```

```rust
// src/petgraph_matching.rs
//! Matching bipartito manual con `petgraph` (grafo bipartito explícito).

use petgraph::graph::UnGraph;

/// Matching greedy (maximal, no necesariamente máximo) sobre un UnGraph bipartito.
/// Devuelve los índices de las aristas elegidas.
pub fn greedy_matching_petgraph(n: usize, aristas: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut g = UnGraph::<(), ()>::new_undirected();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    petgraph::algo::greedy_matching(&g)
        .map(|(a, b)| (a.index(), b.index()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_no_optimo() {
        // Grafo bipartito: L = {0,1}, R = {2,3}, con aristas (0,2),(0,3),(1,2),(1,3).
        // El matching máximo es 2; el greedy puede dar 1 si tiene mala suerte,
        // pero en la práctica suele dar 2.
        let aristas = vec![(0, 2), (0, 3), (1, 2), (1, 3)];
        let m = greedy_matching_petgraph(4, &aristas);
        assert!(m.len() >= 1);
    }
}
```

> **Comparación**: `petgraph::algo::greedy_matching` devuelve un matching **maximal** (no necesariamente máximo). Es $O(V + E)$ pero el resultado puede no ser óptimo. Para matching **máximo**, usa tu Hopcroft-Karp o Kuhn de §8.4-§8.5.

## 8.9 Ejercicios resueltos

### Ejercicio 1 — Maximum bipartite matching manual

Grafo bipartito $L = \{a, b, c\}$, $R = \{1, 2, 3\}$ con aristas $a\!-\!1, a\!-\!2, b\!-\!1, c\!-\!2, c\!-\!3$. ¿Cuál es el matching máximo?

**Solución**: con Kuhn, matching = $\{(a,2),(b,1),(c,3)\}$, tamaño 3 (perfecto). Se obtiene buscando augmenting paths: el primer DFS empareja $a-1$, el segundo reasigna $a-1 \to b-1$ y empareja $a-2$, el tercero empareja $c-3$. Cualquier ruta de augmenting path tiene la misma forma: empieza en un libre de $L$ y termina en un libre de $R$.

### Ejercicio 2 — Tablero y “bishop problem”

En un tablero $3 \times 4$, ¿cuántos alfiles no atacantes caben? Modela filas y columnas como bipartición; cada celda es una arista.

**Solución**: el matching máximo es $\min(3, 4) = 3$ alfiles. Se puede demostrar por el teorema de Hall: cualquier subconjunto de $k$ filas tiene $4k$ celdas disponibles, siempre $\ge k$ para $k \le 3$. Así que admite matching perfecto en el lado menor.

### Ejercicio 3 — Bipartito o no

Grafo: $0-1-2-3-0$ (cuadrado) y $4-5-6-4$ (triángulo). ¿Es bipartito?

**Solución**: el cuadrado es bipartito (alternar colores 0,1,0,1). El triángulo no lo es (ciclo impar). Como son disjuntos, el grafo completo **no** es bipartito (la 2-coloración falla en el triángulo). `es_bipartito` devuelve `None`.

## 8.10 Ejercicios propuestos

1. **(F) LeetCode 785 — Is Graph Bipartite?**. Aplica la 2-coloración descrita en §8.2.
2. **(M) LeetCode 886 — Possible Bipartition**. Modela dislikes como aristas en un grafo no-dirigido y comprueba si es bipartito.
3. **(M) LeetCode 1349 — Maximum Students Taking Exam**. Asignar estudiantes a asientos de modo que ninguno “robe” a otro. Modela como matching bipartito (similar a $n$-reinas).
4. **(D) Asignación de hospitales**. Implementa Hungarian con la crate `lapjv` y compáralo con Hopcroft-Karp cuando los pesos se reemplazan por $w_{ij} = M - c_{ij}$ (para $M$ suficientemente grande).
5. **(D) Gale-Shapley (matching estable)**. Implementa el algoritmo de aceptación diferida para matching estable: cada “médico” propone a su hospital favorito que aún no lo ha rechazado; cada “hospital” acepta temporalmente al mejor y rechaza al resto. Termina en $O(n^2)$ con un matching estable.

## 8.11 Lo que te llevas

- Un grafo es **bipartito** $\iff$ es 2-coloreable $\iff$ no tiene ciclos impares. La detección es $O(V + E)$.
- Un **matching** es un subconjunto de aristas sin vértices compartidos. El **teorema de Berge** lo reduce a buscar **augmenting paths**.
- **Kuhn** es el algoritmo sencillo: $O(V \cdot E)$ en el peor caso, pero muy rápido en la práctica.
- **Hopcroft-Karp** mejora a $O(E \sqrt{V})$ con BFS+DFS por capas, ideal para grafos grandes.
- El **algoritmo húngaro** resuelve asignación con pesos en $O(n^3)$ y es un tour de force de manipulación matricial.
- En Rust: `petgraph::algo::greedy_matching` te da un maximal rápido; para máximo de verdad, escribe Hopcroft-Karp o usa crates como `lapjv`.

## 8.12 Ojo, cuidado con…

- **Kuhn lento sin reinicio del `visited`**. Si reusas el mismo `visited` entre llamadas, el algoritmo falla en encontrar augmenting paths y devuelve matching subóptimo. Crea un `visited` nuevo por vértice, o usa la versión optimizada con DFS más complejo.
- **No verificar bipartito antes de matching**. Si el grafo no es bipartito, los algoritmos de matching bipartito no tienen sentido. Llama a `es_bipartito` antes de Kuhn/HK o asegúrate por construcción.
- **Empate en Hungarian**. Si hay múltiples asignaciones óptimas, Hungarian devuelve una cualquiera. No asumas que es “la” que querías; añade tie-breaking si lo necesitas.
- **Grafos grandes con `Vec<Vec<usize>>`**. La representación “lista de listas” se vuelve lenta cuando $|L|$ y $|R|$ están en millones. Para escala industrial, usa CSR (compressed sparse row) o crates especializadas.
- **“Bipartito” en dígrafo**. La definición bipartita se extiende a dígrafos, pero las aristas deben ir en una dirección. Si pasas un dígrafo a `es_bipartito` con aristas en ambas direcciones, te dirá que es bipartito trivialmente.
- **Confundir `match_left` y `match_right`**. En Hopcroft-Karp, `match_left[u]` guarda el *índice de la derecha* con que $u$ está emparejado. No la posición del array. Confundirlos te da matches fantasma.

## 8.13 Para profundizar

- **König, D. (1931).** “Über Graphen und ihre Anwendung auf Determinantentheorie und Mengenlehre”. *Math. Ann.*, 104.
- **Berge, C. (1957).** “Two theorems in graph theory”. *PNAS*, 43(9).
- **Hopcroft, J. & Karp, R. (1973).** “An $n^{5/2}$ algorithm for maximum matchings in bipartite graphs”. *SIAM J. Comput.*, 2(4).
- **Munkres, J. (1957).** “Algorithms for the assignment and transportation problems”. *J. SIAM*, 5(1).
- **Kuhn, H. W. (1955).** “The Hungarian method for the assignment problem”. *Naval Research Logistics Quarterly*, 2(1-2).
- Vídeo: Tushar Roy — *Hopcroft-Karp* (<https://www.youtube.com/watch?v=lM5eIpEwgxA>).

## 8.14 Pin de batalla

- **`petgraph` no tiene matching bipartito built-in.** Usa el crate `matching` aparte, o implementa Kuhn (30 líneas).
- **Baraja los vecinos de cada vértice en Kuhn** antes de llamar al DFS. Mejora el caso esperado significativamente.
- **Si necesitas Hungarian, usa `good-lp` o escribe un LP solver.** No lo implementes a mano salvo para aprender.
- **Verifica bipartitud antes de aplicar matching bipartito.** Si el grafo no es bipartito, el matching no tiene sentido.
- **Kuhn con `visited` global compartido entre llamadas falla.** Cada llamada a `try_kuhn` necesita su propio `visited`.


## 8.15 Si solo lees 30 segundos

Matching bipartito = asignar lado A a lado B sin repetir. Kuhn para grafos pequeños, Hopcroft-Karp para grandes, Hungarian con pesos.

## 8.16 Una historia pequeña

Javier, recruiter en una startup, recibía 200 CVs al día. Asignarlos a 8 vacantes era un infierno. Un día leyó sobre matching bipartito. Modeló CVs × vacantes como grafo bipartito, asignó pesos por afinidad (skills match), aplicó Hungarian. De 8 contrataciones al mes, pasó a 12, todas con mejor fit. El director de RRHH le preguntó: "¿cómo lo haces?" Javier: "matemáticas que aprendí en la carrera y olvidé en dos años." El director: "y los 6 meses que hemos contratado mal, ¿quién nos los devuelve?" Javier buscó trabajo en otra empresa. La moraleja: a veces un algoritmo vale más que 10 años de experiencia en Excel.


---

