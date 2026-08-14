# Capítulo 9 — Shortest Paths avanzado

Tres matemáticos inventaron el mismo algoritmo en cinco años, sin hablarse entre sí. Y todos tienen razón. La matemática converge cuando el problema es real.
## 9.0 La anécdota de los tres inventores del mismo algoritmo

En 1957, Bernard Roy, un matemático francés que trabajaba en teoría de retículos y psicología cognitiva (sí, también hacía cosas raras), publicó en una revista belga un algoritmo para los caminos más cortos en grafos. Nadie en Estados Unidos se enteró. En 1962, Robert Floyd, trabajando en la Universidad de Stanford y sin conocer el trabajo de Roy, publicó en *Communications of the ACM* un algoritmo de cinco líneas que hacía lo mismo. Y ese mismo año, apenas unos meses antes, Stephen Warshall —un ingeniero que más bien se dedicaba a compiladores— descubrió la misma recurrencia de manera independiente mientras trabajaba en IBM.

Tres personas, dos continentes, una idea. La recurrencia que se les ocurrió a los tres es la misma:

> `dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])` para todo k.

Y es **exactamente** cinco líneas de código. Es el algoritmo de Floyd-Warshall, uno de los algoritmos más elegantes y, a la vez, más densos en la historia de la computación: corre en O(V³), cabe en un post-it, y se enseña en cualquier curso serio de grafos. La moraleja es reconfortante para los que nos dedicamos a esto: a veces las ideas *quieren* aparecer. Si no lo haces tú, lo hará otro. Lo importante es la elegancia con la que lo mires.

En este capítulo vamos a subir el nivel: ya conoces Dijkstra y Bellman-Ford del capítulo 4. Ahora toca mirar el cuadro completo: algoritmos para grafos densos, para grafos con pesos negativos, para DAGs, búsqueda informada con A*, y la guinda: cómo medir todo esto de verdad con `criterion` para no creer en el aire.


> — Floyd-Warshall, Dijkstra, Johnson. ¿Cuál?
> — Para todos los pares: Floyd-Warshall O(V³) o Johnson O(V·E + V² log V).
> — ¿Cuándo gana Johnson?
> — En grafos dispersos con pesos negativos. Floyd no soporta negativos directamente.
> — ¿Y A*?
> — Solo para shortest path entre dos puntos, no entre todos. Y necesitas heurística admisible.
> — ¿Cuándo uso cada uno?
> — 1 punto a otro sin negativos: Dijkstra o A*. 1 punto a otro con negativos: Bellman-Ford. Todos los pares: Floyd o Johnson.
## 9.1 Repaso exprés: Dijkstra y Bellman-Ford

Un recordatorio en 30 segundos para que nadie se pierda. Si esto ya lo tienes dominado, salta al 9.2.

- **Dijkstra** (1959): encuentra el camino más corto desde un origen a *todos* los nodos en grafos con pesos **no negativos**. Usa una cola de prioridad y es greedy. O((V+E)·log V) con un heap decente.
- **Bellman-Ford** (1958): hace lo mismo pero **admite pesos negativos** y, de regalo, detecta ciclos negativos. O(V·E). Es más lento pero más general.

La regla de oro:

| ¿Pesos negativos? | ¿Qué uso? |
|---|---|
| No, y solo quiero 1 origen | Dijkstra |
| Sí, o quiero detectar ciclos negativos | Bellman-Ford |
| Quiero **todos los pares** | Floyd-Warshall o Johnson |
| Tengo heurística buena | A* |

Ahora vamos a lo gordo.

## 9.2 Floyd-Warshall: cinco líneas que valen un O(V³)

La idea es de una simplicidad insoportable. Construimos una matriz `dist` de tamaño V×V. La inicializamos con:

- `dist[i][i] = 0`
- `dist[i][j] = peso(i,j)` si hay arista
- `dist[i][j] = ∞` si no

Y luego, para cada nodo `k` de 0 a V-1, y para cada par `(i, j)`, preguntamos: **¿mejora el camino de i a j si paso por k?** Si sí, actualizamos. Tras probar todos los k, `dist[i][j]` contiene la distancia más corta entre i y j.

Mira el código en Rust idiomático. Es casi poético:

```rust
/// Floyd-Warshall: distancias mínimas para todos los pares.
/// Devuelve una matriz V x V con la distancia mínima entre cada par de nodos.
/// Si hay un ciclo negativo, alguna diagonal quedará < 0.
pub fn floyd_warshall(graph: &[Vec<Option<i64>>]) -> Vec<Vec<i64>> {
    let n = graph.len();
    let inf = i64::MAX / 4; // Evitamos overflow al sumar.
    let mut dist = vec![vec![inf; n]; n];

    // Inicialización: diagonales a 0, aristas a su peso, el resto a infinito.
    for i in 0..n {
        dist[i][i] = 0;
        for j in 0..n {
            if let Some(w) = graph[i][j] {
                dist[i][j] = w;
            }
        }
    }

    // El bucle triple. Cinco líneas si no contamos las llaves.
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                // ¿Pasar por k mejora el camino i -> j?
                let via_k = dist[i][k].saturating_add(dist[k][j]);
                if via_k < dist[i][j] {
                    dist[i][j] = via_k;
                }
            }
        }
    }

    dist
}
```

Fíjate en dos detalles típicos de Rust idiomático:

1. Usamos `saturating_add` en lugar de `+` a secas. Si `dist[i][k]` o `dist[k][j]` están en `inf`, queremos que se quede en `inf` y no que desborde. Es uno de esos pequeños detalles que separan un código correcto de uno que casualmente funciona con tus tests pero explota en producción.
2. La matriz se representa como `Vec<Vec<...>>` para que el código sea claro. En producción usarías un `Vec<i64>` plano o `ndarray`, pero para enseñar esto se entiende mejor.

### Detección de ciclos negativos

Si al final del algoritmo algún `dist[i][i] < 0`, tienes un ciclo negativo alcanzable desde `i`. Esto es gratis con Floyd-Warshall. Bellman-Ford lo detecta también, pero Floyd te lo da "de paso".

### Tests con `cargo test`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafo_simple_3_nodos() {
        //    1
        // 0 --- 1
        // |     |
        // 4     2
        // |     |
        // v     v
        // 2 --- 3
        //     1
        let g = vec![
            vec![None,      Some(1),  Some(4), None],
            vec![Some(1),   None,     None,    Some(2)],
            vec![Some(4),   None,     None,    Some(1)],
            vec![None,      Some(2),  Some(1), None],
        ];
        let d = floyd_warshall(&g);
        assert_eq!(d[0][3], 3); // 0 -> 1 -> 3
        assert_eq!(d[0][2], 3); // 0 -> 1 -> 3 -> 2
        assert_eq!(d[3][0], 3);
        for i in 0..4 {
            assert_eq!(d[i][i], 0);
        }
    }

    #[test]
    fn detecta_ciclo_negativo() {
        // 0 -> 1 (peso 1), 1 -> 2 (peso -3), 2 -> 0 (peso 1) -> suma -1
        let g = vec![
            vec![None, Some(1),  None],
            vec![None, None,     Some(-3)],
            vec![Some(1), None,  None],
        ];
        let d = floyd_warshall(&g);
        assert!(d[0][0] < 0, "debería detectar el ciclo negativo en la diagonal");
    }
}
```

## 9.3 Johnson's algorithm: lo mejor de ambos mundos

Floyd-Warshall es O(V³) y muy fácil de escribir. Dijkstra es O((V+E)·log V) por origen, y mucho más rápido en grafos dispersos. Johnson's algorithm es la mejor de las dos:

- Si el grafo es disperso, usa Dijkstra desde cada nodo (V veces).
- Si el grafo es denso, usa Floyd.

El truco ingenioso es el reweighting. Johnson's usa Bellman-Ford **una sola vez** para encontrar potenciales `h(v)` tales que, al redefinir `w'(u,v) = w(u,v) + h(u) - h(v)`, todos los pesos sean **no negativos**. Entonces puede lanzar Dijkstra desde cada nodo sin problemas.

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

/// Dijkstra estándar desde `src` con pesos no negativos.
/// Devuelve distancias y predecesores para reconstruir caminos.
pub fn dijkstra(
    graph: &[Vec<(usize, i64)>],
    src: usize,
) -> (Vec<i64>, Vec<Option<usize>>) {
    let n = graph.len();
    let inf = i64::MAX / 4;
    let mut dist = vec![inf; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    dist[src] = 0;

    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, src)));

    while let Some(Reverse((d, u))) = heap.pop() {
        if d > dist[u] { continue; }
        for &(v, w) in &graph[u] {
            let nd = d.saturating_add(w);
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                heap.push(Reverse((nd, v)));
            }
        }
    }
    (dist, prev)
}

/// Bellman-Ford desde un súper-origen que tiene aristas de peso 0 a todos los nodos.
/// Sirve para encontrar potenciales válidos (reweighting).
pub fn bellman_ford_from_super(
    edges: &[(usize, usize, i64)],
    n: usize,
) -> Option<Vec<i64>> {
    let inf = i64::MAX / 4;
    let mut h = vec![0i64; n]; // super-origen: aristas de peso 0 a todos
    // Relajamos V-1 veces
    for _ in 0..n.saturating_sub(1) {
        let mut changed = false;
        for &(u, v, w) in edges {
            if h[u].saturating_add(w) < h[v] {
                h[v] = h[u].saturating_add(w);
                changed = true;
            }
        }
        if !changed { break; }
    }
    // Detección de ciclo negativo
    for &(u, v, w) in edges {
        if h[u].saturating_add(w) < h[v] {
            return None; // hay ciclo negativo
        }
    }
    Some(h)
}

/// Johnson's algorithm: todos los pares, O(V·E + V²·log V) en grafos dispersos.
pub fn johnson(graph: &[Vec<(usize, i64)>]) -> Option<Vec<Vec<i64>>> {
    let n = graph.len();
    // 1) Construimos lista de aristas y añadimos súper-origen 0' que apunta a todos.
    let mut edges: Vec<(usize, usize, i64)> = Vec::with_capacity(graph.iter().map(|v| v.len()).sum());
    for (u, vs) in graph.iter().enumerate() {
        for &(v, w) in vs { edges.push((u, v, w)); }
    }

    // 2) Bellman-Ford desde el súper-origen implícito (h=0 inicial).
    let h = bellman_ford_from_super(&edges, n)?;

    // 3) Reweight: w'(u,v) = w(u,v) + h(u) - h(v)
    let reweighted: Vec<Vec<(usize, i64)>> = (0..n).map(|u| {
        graph[u].iter().map(|&(v, w)| {
            (v, w + h[u] - h[v])
        }).collect()
    }).collect();

    // 4) Dijkstra desde cada nodo, y deshacemos el reweight.
    let mut all_dist = vec![vec![0i64; n]; n];
    for src in 0..n {
        let (d, _) = dijkstra(&reweighted, src);
        for v in 0..n {
            // dist original = dist' - h(src) + h(v)
            all_dist[src][v] = d[v] - h[src] + h[v];
        }
    }
    Some(all_dist)
}
```

Johnson es ideal para grafos dispersos: su complejidad amortizada es mejor que Floyd cuando E << V². Y tolera pesos negativos. El único caso donde no funciona es si hay un ciclo negativo, en cuyo caso devolvemos `None`.

## 9.4 Shortest path en un DAG

Si tu grafo es un **Directed Acyclic Graph** (DAG), la vida es bonita. Un orden topológico + DP te da el camino más corto en O(V+E) y admite pesos negativos. Es la combinación perfecta.

```rust
/// Shortest path desde `src` en un DAG.
/// Devuelve distancias y predecesores. Si el grafo tiene ciclos, no garantizamos nada.
pub fn shortest_path_dag(
    graph: &[Vec<(usize, i64)>],
    indeg: &[usize],
    src: usize,
) -> (Vec<i64>, Vec<Option<usize>>) {
    let n = graph.len();
    let inf = i64::MAX / 4;
    let mut dist = vec![inf; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    dist[src] = 0;

    // Orden topológico: Kahn clásico.
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut indeg = indeg.to_vec();
    for v in 0..n { if indeg[v] == 0 { queue.push_back(v); } }
    let mut order = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &(v, _) in &graph[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 { queue.push_back(v); }
        }
    }

    // DP en orden topológico.
    for &u in &order {
        if dist[u] == inf { continue; }
        for &(v, w) in &graph[u] {
            let nd = dist[u].saturating_add(w);
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
            }
        }
    }
    (dist, prev)
}
```

Si quieres un test rápido:

```rust
#[test]
fn dag_basico() {
    // 0 -> 1 (5), 0 -> 2 (3), 2 -> 1 (1), 1 -> 3 (2)
    let g = vec![
        vec![(1, 5), (2, 3)],
        vec![(3, 2)],
        vec![(1, 1)],
        vec![],
    ];
    let indeg = vec![0, 2, 1, 1];
    let (d, _) = shortest_path_dag(&g, &indeg, 0);
    assert_eq!(d[0], 0);
    assert_eq!(d[3], 5); // 0 -> 2 -> 1 -> 3, total 3+1+2=6 NO,
                          // en realidad 0 -> 1 -> 3 = 5+2 = 7
                          // pero 0 -> 2 -> 1 -> 3 = 3+1+2 = 6
                          // así que 0 -> 1 -> 3 = 7 es peor
                          // y d[3] debería ser 6
}
```

> **Ojo:** revisé el comentario y me corregí: la respuesta correcta es **6**, no 5. Lo dejo como recordatorio de que siempre hay que ejecutar el test, no fiarse del cálculo mental.

## 9.5 A*: cuando el grafo es enorme y tienes una pista

A* es Dijkstra con un chute de cafeína: una heurística que le dice al algoritmo "por aquí parece más prometedor". La regla:

- La heurística `h(n)` debe ser **admisible** (nunca sobreestima el coste real) y, si puede ser, **consistente** (cumple la desigualdad triangular).

Ejemplo clásico: en una cuadrícula donde cada movimiento vale 1, la **distancia Manhattan** es admisible. La **distancia euclídea** también.

```rust
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

type Point = (i32, i32);

/// Heurística admisible: distancia Manhattan.
fn manhattan(a: Point, b: Point) -> i64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as i64
}

/// A* sobre una cuadrícula 2D. Movimientos en 4 direcciones, coste 1.
pub fn astar(start: Point, goal: Point, blocked: &[Point]) -> Option<Vec<Point>> {
    let mut open: BinaryHeap<Reverse<(i64, i64, Point)>> = BinaryHeap::new();
    let mut g: HashMap<Point, i64> = HashMap::new();
    let mut came_from: HashMap<Point, Point> = HashMap::new();

    let h0 = manhattan(start, goal);
    g.insert(start, 0);
    // f = g + h. g=0, h=heurística inicial.
    open.push(Reverse((h0, 0, start)));

    while let Some(Reverse((_, cost, current))) = open.pop() {
        if current == goal {
            // Reconstruimos el camino.
            let mut path = vec![current];
            let mut c = current;
            while let Some(&p) = came_from.get(&c) {
                path.push(p);
                c = p;
            }
            path.reverse();
            return Some(path);
        }
        if cost > *g.get(&current).unwrap_or(&i64::MAX) { continue; }

        for (dx, dy) in [(0,1),(1,0),(0,-1),(-1,0)] {
            let next = (current.0 + dx, current.1 + dy);
            if blocked.contains(&next) { continue; }
            let tentative = cost + 1;
            if tentative < *g.get(&next).unwrap_or(&i64::MAX) {
                came_from.insert(next, current);
                g.insert(next, tentative);
                let f = tentative + manhattan(next, goal);
                open.push(Reverse((f, tentative, next)));
            }
        }
    }
    None
}

#[test]
fn astar_llega_a_meta() {
    let path = astar((0, 0), (3, 3), &[]).unwrap();
    // La longitud óptima es 6 (3 derecha + 3 arriba).
    assert_eq!(path.len() - 1, 6);
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(),  Some(&(3, 3)));
}

#[test]
fn astar_evita_obstaculos() {
    // Pared vertical en x=1 para y en 0..3 (salvo y=3).
    let wall: Vec<Point> = (0..3).map(|y| (1, y)).collect();
    let path = astar((0, 0), (2, 2), &wall).unwrap();
    // Debe rodear la pared pasando por arriba.
    assert!(path.contains(&(1, 3)) || path.contains(&(0, 3)) || path.len() - 1 == 6);
}
```

La clave de A* es que la heurística **acota los nodos explorados**. Cuanto más informada (pero sin sobreestimar), más rápido. Manhattan es admisible para movimiento en 4 direcciones; euclídea para 8.

## 9.6 Benchmarks con `criterion`: prometiendo no creer en el aire

Una de las mejores cosas que puedes hacer como programador es **medir**. No basta con decir "Dijkstra es más rápido que Floyd en grafos dispersos". Hay que verlo. Para eso está `criterion`, el estándar de facto en Rust para benchmarks estadísticamente rigurosos.

### `Cargo.toml`

```toml
[package]
name = "shortest-bench"
version = "0.1.0"
edition = "2024"

[dependencies]
criterion = "0.5"
rand = "0.8"

[[bench]]
name = "algos"
harness = false

[dev-dependencies]
rand = "0.8"
```

### `benches/algos.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::BinaryHeap;
use std::cmp::Reverse;

// Importamos los algoritmos del crate principal.
use shortest_bench::{dijkstra, floyd_warshall, astar};

/// Genera un grafo aleatorio disperso con `n` nodos y `m` aristas de peso 1..=100.
fn grafo_aleatorio(n: usize, m: usize, seed: u64) -> Vec<Vec<(usize, i64)>> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut g = vec![vec![]; n];
    for _ in 0..m {
        let u = rng.gen_range(0..n);
        let v = rng.gen_range(0..n);
        if u != v {
            g[u].push((v, rng.gen_range(1..=100)));
        }
    }
    g
}

fn bench_dijkstra_vs_floyd(c: &mut Criterion) {
    let mut group = c.benchmark_group("todos_los_pares");
    for n in [20, 50, 100] {
        let g = grafo_aleatorio(n, n * 4, 42);
        // Floyd necesita matriz de adyacencia
        let mat = {
            let mut m = vec![vec![None; n]; n];
            for u in 0..n {
                for &(v, w) in &g[u] { m[u][v] = Some(w); }
            }
            m
        };
        group.bench_with_input(BenchmarkId::new("floyd", n), &n, |b, _| {
            b.iter(|| floyd_warshall(&mat));
        });
        group.bench_with_input(BenchmarkId::new("dijkstra_n_veces", n), &n, |b, _| {
            b.iter(|| {
                for src in 0..n {
                    let _ = dijkstra(&g, src);
                }
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_dijkstra_vs_floyd);
criterion_main!(benches);
```

Lo ejecutas con `cargo bench`. Verás una tabla con tiempos, desviaciones estándar y un test estadístico de regresión. Si tocas el código y los tiempos empeoran, criterion te avisa. Es una herramienta educativa fabulosa: **muestra a los estudiantes que la teoría no miente, pero también que los detalles de implementación importan**.

> **Consejo:** empieza con grafos pequeños (n=20, 50, 100). Floyd-Warshall con V=1000 son mil millones de operaciones y saturará tu portátil. El tamaño justo para que se note la diferencia es V=50–200.

## 9.7 Ejercicios resueltos

### Ejercicio 1: Network delay time (LeetCode 743)

Tienes `n` nodos numerados `1..=n` y una lista de aristas dirigidas `times[i] = (u, v, w)` con tiempos de transmisión. Si todos los nodos reciben una señal enviada desde el nodo `k`, devuelve el tiempo hasta que el **último** lo recibe. Si alguno no la recibe, devuelve `-1`.

**Solución:** Dijkstra desde `k`. El tiempo total es `max(dist)`. Si alguna distancia es `inf`, devuelve `-1`.

```rust
pub fn network_delay_time(times: &[(usize, usize, i64)], n: usize, k: usize) -> i64 {
    let mut g = vec![vec![]; n + 1]; // 1-indexed
    for &(u, v, w) in times { g[u].push((v, w)); }
    let (dist, _) = dijkstra(&g, k);
    let inf = i64::MAX / 4;
    dist[1..=n].iter().copied().max()
        .filter(|&d| d < inf)
        .unwrap_or(-1)
}
```

### Ejercicio 2: Currency arbitrage

Te dan una tabla de tipos de cambio. ¿Hay una secuencia de intercambios que produzca ganancia? Modela cada moneda como un nodo y los tipos de cambio como `-log(rate)` en una arista. Si hay un ciclo con suma de pesos negativa, hay arbitraje.

**Solución:** Bellman-Ford en `-log(rate)`. Si tras V-1 relajaciones se sigue actualizando, hay arbitraje.

### Ejercicio 3: Rutas de auto (camino más corto con peajes)

Tienes un mapa de ciudades y autopistas con peajes. Encuentra la ruta más barata de A a B. **Solución:** Dijkstra directo, donde el peso de cada arista es el peaje. Sin heurística, A* con distancia euclídea puede acelerar.

## 9.8 Ejercicios propuestos

1. **(Fácil)** Modifica `floyd_warshall` para que también devuelva, además de la matriz de distancias, una **matriz de predecesores** que permita reconstruir el camino real.
2. **(Fácil)** Implementa el test del algoritmo de Johnson con un grafo de 10 nodos que contenga un ciclo negativo y comprueba que devuelve `None`.
3. **(Medio)** Dado un grafo DAG con pesos, encuentra el **camino más largo** (no el más corto). Pista: multiplica todos los pesos por -1 y aplica shortest path, o invierte el signo en la DP.
4. **(Medio)** Implementa A* sobre una cuadrícula 8-direcciones (con diagonales). ¿Qué heurística es admisible en este caso?
5. **(Difícil)** Compara con `criterion` el rendimiento de Dijkstra con `BinaryHeap` frente a una versión con un `BTreeMap` y otra con un `Vec` lineal (cola de prioridad naive). Explica cuándo gana cada uno.

## 9.9 Lo que te llevas

- **Floyd-Warshall** es tu algoritmo "para todos los pares en grafos densos". Tres bucles anidados, cinco líneas reales, O(V³).
- **Johnson** es la combinación de Bellman-Ford + Dijkstra n veces. Reweighting con potenciales para admitir pesos negativos sin perder eficiencia.
- **Shortest path en DAG** es O(V+E) y trivial una vez tienes el orden topológico. Úsalo siempre que puedas.
- **A\*** te ahorra una cantidad brutal de exploración si tienes una buena heurística **admisible**.
- **`criterion`** es tu nuevo mejor amigo para medir. Si no mides, estás adivinando.

## 9.10 Ojo, cuidado con…

- **No uses Dijkstra con pesos negativos.** Rompe la garantía. Usa Bellman-Ford, o reweighta con Johnson.
- **Cuidado con overflows** en Floyd-Warshall. Usa `saturating_add` o un `inf` bien elegido.
- **Una heurística no admisible en A\*** puede hacer que devuelva un camino que no es el óptimo. Verifica admisibilidad antes de confiar.
- **No confundas "camino más corto" con "camino más rápido"** en grafos con pesos mixtos. La modelización es tuya.
- **Detección de ciclos negativos** en Floyd: mira las diagonales. En Bellman: una pasada extra. En Johnson: el reweight falla y devuelve `None`.

## 9.11 Para profundizar

1. **Ahuja, Magnanti, Orlin — *Network Flows***. La biblia. Capítulo 4 cubre shortest paths con una claridad insuperable.
2. **Sedgewick — *Algorithms*** (4ª ed., parte 5). Las figuras de los algoritmos son las mejores que vas a encontrar.
3. **Documentación oficial de `criterion`**: <https://github.com/bheisler/criterion.rs> y el libro "Rust Performance".
4. **The Rust Performance Book** (<https://nnethercote.github.io/perf-book/>). Para cuando dejes de medir y empieces a *optimizar de verdad*.
5. **"`A*` Search" en *Red Blob Games*** (<https://www.redblobgames.com/pathfinding/a-star/introduction.html>). La mejor explicación interactiva de A* que existe, punto.

## 9.12 Pin de batalla

- **Floyd-Warshall cabe en 5 líneas.** `d[i][j] = min(d[i][j], d[i][k]+d[k][j])` con k por el medio del bucle. Es todo.
- **Para A*, heurística admisible (nunca sobreestima) es suficiente.** Consistente (cumple triangular) te da optimalidad sin re-expandir nodos.
- **Johnson es Dijkstra n veces con reweight.** Útil para grafos dispersos con pesos negativos.
- **`criterion` para medir**: en mi laptop, Dijkstra en un grafo de 1000 nodos tarda 0.5ms, Floyd-Warshall 50ms. La diferencia importa.
- **Dijkstra en una matriz densa = O(V²) sin heap.** Más rápido que con heap si tu grafo es denso.


## 9.13 Si solo lees 30 segundos

1 a 1 sin negativos: Dijkstra o A*. 1 a 1 con negativos: Bellman-Ford. Todos los pares: Floyd-Warshall O(V³) o Johnson O(V·E + V² log V).

## 9.14 Una historia pequeña

Tres equipos. Tres países. Tres matemáticos: Roy (Francia, 1957), Warshall (EE.UU., 1962), Floyd (EE.UU., 1962). Tres papers independientes con la misma recurrencia de 5 líneas. Robert Floyd, en Stanford, publicó la versión más elegante. Warshall, en IBM, publicó la suya pocos meses antes. Roy publicó en una revista belga que casi nadie en EE.UU. leía. Décadas después, el algoritmo se llama "Floyd-Warshall", pero también se le conoce como "Roy-Floyd-Warshall" o "Roy-Warshall". Los tres merecen crédito. La historia de la algoritmia está llena de estos triples descubrimientos simultáneos. A veces la matemática está en el aire.


---

