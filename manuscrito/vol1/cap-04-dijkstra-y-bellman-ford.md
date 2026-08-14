# Capítulo 4 — Dijkstra y Bellman-Ford

Edsger Dijkstra estaba comprando café con su prometida en Amsterdam un domingo de 1956. En 20 minutos se le ocurrió el algoritmo que lleva su nombre. Sigues vivo hoy gracias a él cada vez que Google Maps te dice "gira a la derecha en 200 metros".
## 4.0 La anécdota de la esquina

Ámsterdam, 1956. Edsger W. Dijkstra era un joven informático holandés que trabajaba en el Centro Matemático de Ámsterdam. Una tarde, salió a dar un paseo con su prometida (según cuenta la historia) y, mientras caminaba hacia una cafetería, se le ocurrió el algoritmo que le iba a hacer famoso. Veinte minutos de paseo, veinte minutos de inspiración. Cuando llegó a la cafetería, ya tenía el algoritmo en la cabeza. Lo escribió en una servilleta. Bueno, eso dice la leyenda; la realidad es que lo publicó al año siguiente en "A Note on Two Problems in Connexion with Graphs" (1956), un paper de una página y media que cambió la informática para siempre.

Lo más fascinante es que Dijkstra no estaba pensando en mapas cuando lo concibió. Estaba pensando en el problema de **encontrar el camino más corto en un grafo con pesos no negativos**, que era un dolor de cabeza para los ingenieros de telecomunicación. ¿Cómo enruto una llamada telefónica de la forma más barata posible, sabiendo que cada central tiene un coste de conmutación distinto?

La solución que se le ocurrió es elegantísima: en vez de probar todos los caminos (que serían factoriales), vas expandiendo una "frontera" desde el origen, siempre eligiendo el vértice más cercano todavía no procesado. Es la misma idea de BFS, pero con una **cola de prioridad** que te dice cuál es el siguiente más cercano. Y aquí, en pleno siglo XXI, sigue siendo el algoritmo que tu móvil, Google Maps, y los protocolos de routing de Internet usan cada día.


> — Dijkstra y Bellman-Ford, ¿cuál uso?
> — Si todos los pesos son positivos (que es el 95% de los casos reales), Dijkstra. Si tienes pesos negativos, Bellman-Ford.
> — ¿Y por qué Bellman-Ford es más lento?
> — Porque relaja TODAS las aristas V-1 veces. Dijkstra solo toca cada vértice una vez gracias al heap.
> — Y entonces, ¿para qué existe Bellman-Ford?
> — Para detectar ciclos negativos. Si los hay, el camino más corto no existe, es -infinito. Dijkstra no te avisa, Bellman-Ford sí.
## 4.1 El problema del camino más corto

Tienes un grafo ponderado, dirigido o no, y quieres la distancia mínima (suma de pesos) entre un vértice origen y todos los demás. Asumimos pesos no negativos (para Dijkstra). Si hay pesos negativos, la cosa se complica y necesitamos Bellman-Ford.

Aplicaciones: navegación GPS, routing de paquetes en Internet, planificación de vuelos, juegos de estrategia, robótica…

## 4.2 Dijkstra: intuición y código

**Intuición.** Imagina que estás en un cruce y tienes un mapa de calles con sus tiempos. Quieres llegar a TODOS los demás cruces lo más rápido posible. ¿Qué haces? Tomas el cruce más cercano (en tiempo) que aún no has "resuelto", lo resuelves, y actualizas el tiempo estimado a sus vecinos. Repite hasta que no quede nadie por resolver.

**Formalmente:**

```
Dijkstra(grafo, origen):
  dist[v] = ∞ para todo v
  dist[origen] = 0
  cola_prioridad = MinHeap([(0, origen)])
  while cola_prioridad no vacía:
    (d, u) = cola_prioridad.pop_min()
    if d > dist[u]: continue   // ya hay un camino mejor
    for (v, peso) in aristas(u):
      nueva = d + peso
      if nueva < dist[v]:
        dist[v] = nueva
        cola_prioridad.push((nueva, v))
  return dist
```

**Complejidad:** O((V + E) log V) con un `BinaryHeap` (que es un min-heap en Rust; explico el truco ahora).

## 4.3 El truco de Rust: `BinaryHeap` es max-heap, no min-heap

El `BinaryHeap` de `std::collections` es un **max-heap** (el más grande arriba). Para hacer Dijkstra necesitamos un min-heap (el más pequeño arriba). El truco estándar en Rust es invertir las prioridades: envuelve el peso en un struct y dale un `Ord` invertido, o más fácil, usa `Reverse`:

```rust
use std::cmp::Reverse;
use std::collections::BinaryHeap;

// Encolar así:
let mut q: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
q.push(Reverse((0, origen)));
// Desencolar el mínimo:
while let Some(Reverse((d, u))) = q.pop() {
    // d es el más pequeño
}
```

¡`Reverse` ya implementa `Ord` invertido! Es idiomático, limpio, y rápido. (Otra opción es usar el crate `priority-queue`, que tiene un min-heap nativo, pero con `Reverse` no necesitas dependencias extra.)

## 4.4 Dijkstra en Rust puro

```rust
// src/lib.rs
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub type AristasPonderadas = Vec<Vec<(u32, u32)>>; // (vecino, peso)

/// Devuelve un vector `dist` donde dist[v] = distancia mínima desde `origen` a v.
/// Si v es inalcanzable, dist[v] = u32::MAX.
pub fn dijkstra(adj: &AristasPonderadas, origen: usize) -> Vec<u32> {
    let n = adj.len();
    let mut dist: Vec<u32> = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();

    dist[origen] = 0;
    heap.push(Reverse((0, origen)));

    while let Some(Reverse((d, u))) = heap.pop() {
        // Si ya hay una distancia mejor registrada, saltamos.
        if d > dist[u] {
            continue;
        }
        for &(v, peso) in &adj[u] {
            let v = v as usize;
            // Importante: controlar overflow antes de sumar
            let nueva = match d.checked_add(peso) {
                Some(x) => x,
                None => continue,
            };
            if nueva < dist[v] {
                dist[v] = nueva;
                heap.push(Reverse((nueva, v)));
            }
        }
    }
    dist
}
```

Y los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Grafo:
    ///     0 --1-- 1
    ///     |       |
    ///     4       2
    ///     |       |
    ///     2 --3-- 3
    fn grafo_ejemplo() -> AristasPonderadas {
        vec![
            vec![(1, 1), (2, 4)],
            vec![(0, 1), (3, 2)],
            vec![(0, 4), (3, 3)],
            vec![(1, 2), (2, 3)],
        ]
    }

    #[test]
    fn distancias_desde_0() {
        let g = grafo_ejemplo();
        let dist = dijkstra(&g, 0);
        // dist[0] = 0
        // dist[1] = 1 (0 -> 1)
        // dist[2] = 4 (0 -> 2 directo) mejor que 0 -> 1 -> 3 -> 2 = 1+2+3 = 6
        // dist[3] = 3 (0 -> 1 -> 3) mejor que 0 -> 2 -> 3 = 4+3 = 7
        assert_eq!(dist, vec![0, 1, 4, 3]);
    }

    #[test]
    fn destino_inalcanzable_es_max() {
        let g = vec![
            vec![(1, 5)],   // 0 -> 1
            vec![(0, 5)],   // 1 -> 0
            // vértice 2 aislado
        ];
        let dist = dijkstra(&g, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 5);
        assert_eq!(dist[2], u32::MAX);
    }
}
```

## 4.5 Bellman-Ford: para cuando hay pesos negativos

Dijkstra falla si hay aristas con peso negativo. ¿Por qué? Porque asume que una vez que "resuelves" un vértice (lo sacas del heap), su distancia no va a mejorar. Con pesos negativos, eso no se sostiene: puede aparecer un camino posterior más barato.

**Solución: Bellman-Ford.** Repite el bucle de relajación V−1 veces, propagando mejoras. Si en una iteración extra (la V-ésima) todavía se relaja algo, hay un ciclo negativo alcanzable desde el origen.

```rust
/// `aristas`: lista de tuplas (u, v, peso) para un grafo DIRIGIDO.
/// `n`: número de vértices. `origen`: vértice de partida.
pub fn bellman_ford(aristas: &[(u32, u32, i64)], n: usize, origen: usize)
    -> Result<Vec<i64>, &'static str>
{
    let mut dist: Vec<i64> = vec![i64::MAX; n];
    dist[origen] = 0;

    for _ in 0..n - 1 {
        let mut cambio = false;
        for &(u, v, w) in aristas {
            if dist[u as usize] != i64::MAX
                && dist[u as usize] + w < dist[v as usize]
            {
                dist[v as usize] = dist[u as usize] + w;
                cambio = true;
            }
        }
        if !cambio { break; } // optimización: si nada cambió, terminamos
    }

    // Detección de ciclo negativo
    for &(u, v, w) in aristas {
        if dist[u as usize] != i64::MAX
            && dist[u as usize] + w < dist[v as usize]
        {
            return Err("¡Hay un ciclo negativo alcanzable desde el origen!");
        }
    }
    Ok(dist)
}
```

Usamos `i64` (signed) y no `u32` porque los pesos negativos existen. **Complejidad:** O(V·E). Más lento que Dijkstra, pero detecta ciclos negativos y permite pesos negativos.

## 4.6 Con `petgraph`: una línea

Petgraph ya tiene ambas implementaciones. Usarlas es trivial:

```rust
use petgraph::algo::{bellman_ford, dijkstra};
use petgraph::graph::{DiGraph, NodeIndex};

pub fn dijkstra_petgraph(g: &DiGraph<(), u32>, origen: NodeIndex) -> Vec<Option<u32>> {
    // Map<NodeIndex, u32> con la distancia desde `origen`.
    let res = dijkstra(g, origen, None, |e| *e.weight());
    // Convertimos a vector indexado por NodeIndex.index()
    let mut dist: Vec<Option<u32>> = vec![None; g.node_count()];
    for (n, d) in res {
        dist[n.index()] = Some(d);
    }
    dist
}

pub fn bellman_ford_petgraph(
    g: &DiGraph<(), i64>,
    origen: NodeIndex,
) -> Result<Vec<Option<i64>>, petgraph::algo::NegativeCycle> {
    // bellman_ford devuelve Potential -> distances por nodo, o NegativeCycle.
    let res = bellman_ford(g, origen, |e| *e.weight())?;
    let mut dist: Vec<Option<i64>> = vec![None; g.node_count()];
    for (n, d) in res {
        dist[n.index()] = Some(d);
    }
    Ok(dist)
}
```

La función `dijkstra` de petgraph devuelve un `HashMap<NodeIndex, u32>` con las distancias a TODOS los vértices alcanzables. ¡Más fácil imposible!

## 4.7 Tabla: ¿cuándo uso cada uno?

| Criterio | Dijkstra | Bellman-Ford |
|---|---|---|
| Pesos no negativos | ✅ Ideal | ✅ Funciona |
| Pesos negativos | ❌ Falla | ✅ Funciona |
| Detecta ciclo negativo | ❌ | ✅ |
| Complejidad | O((V+E) log V) con heap | O(V·E) |
| Más rápido en grafos grandes | ✅ | ❌ |
| Implementación en petgraph | `petgraph::algo::dijkstra` | `petgraph::algo::bellman_ford` |

**Regla de oro:** usa Dijkstra por defecto. Si sabes que hay pesos negativos, o necesitas detectar ciclos negativos, usa Bellman-Ford. (Si necesitas lo segundo, plantéate usar SPFA, una variante optimizada de Bellman-Ford, pero eso ya es tema avanzado.)

## 4.8 Ejercicios resueltos

**Ejercicio 4.1 (F).** Calcula las distancias mínimas desde A en el siguiente grafo:

```
    A --1-- B
    |       |
    4       2
    |       |
    C --1-- D
```

Aristas: A-B (1), A-C (4), B-D (2), C-D (1).

**Solución.** dist(A) = 0. dist(B) = 1 (directo). dist(C) = 4 (directo) o 1+2+1 = 4 (A-B-D-C), igual. dist(D) = 1+2 = 3 (A-B-D) o 4+1 = 5 (A-C-D). Mínimo: 3.

**Ejercicio 4.2 (M).** Implementa una función que, además de las distancias, devuelva el camino concreto (no solo la distancia).

**Pista:** añade un array `padre[]` y actualízalo cuando actualices `dist[v]`. Para reconstruir, ve saltando de `padre[destino]` a `padre[padre[destino]]` hasta llegar al origen.

```rust
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub fn dijkstra_con_camino(adj: &AristasPonderadas, origen: usize)
    -> (Vec<u32>, Vec<Option<usize>>)
{
    let n = adj.len();
    let mut dist: Vec<u32> = vec![u32::MAX; n];
    let mut padre: Vec<Option<usize>> = vec![None; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    dist[origen] = 0;
    heap.push(Reverse((0, origen)));
    while let Some(Reverse((d, u))) = heap.pop() {
        if d > dist[u] { continue; }
        for &(v, w) in &adj[u] {
            let v = v as usize;
            let nueva = match d.checked_add(w) { Some(x) => x, None => continue };
            if nueva < dist[v] {
                dist[v] = nueva;
                padre[v] = Some(u);
                heap.push(Reverse((nueva, v)));
            }
        }
    }
    (dist, padre)
}

pub fn reconstruye_camino(padre: &[Option<usize>], destino: usize) -> Vec<usize> {
    let mut camino = Vec::new();
    let mut actual = Some(destino);
    while let Some(v) = actual {
        camino.push(v);
        actual = padre[v];
    }
    camino.reverse();
    camino
}

#[test]
fn test_camino() {
    let g = vec![
        vec![(1, 1), (2, 4)],
        vec![(0, 1), (3, 2)],
        vec![(0, 4), (3, 3)],
        vec![(1, 2), (2, 3)],
    ];
    let (dist, padre) = dijkstra_con_camino(&g, 0);
    assert_eq!(dist[3], 3);
    let camino = reconstruye_camino(&padre, 3);
    assert_eq!(camino, vec![0, 1, 3]);
}
```

**Ejercicio 4.3 (M).** Detecta si un grafo tiene un ciclo negativo alcanzable desde un origen.

**Pista:** ejecuta Bellman-Ford. Si al hacer la iteración extra se relaja alguna arista, hay ciclo negativo.

```rust
pub fn tiene_ciclo_negativo(aristas: &[(u32, u32, i64)], n: usize) -> bool {
    // Truco: inicializamos todas las distancias a 0 para detectar
    // cualquier ciclo alcanzable desde CUALQUIER vértice, no solo desde
    // un origen concreto.
    let mut dist = vec![0i64; n];
    for _ in 0..n {
        for &(u, v, w) in aristas {
            if dist[u as usize] + w < dist[v as usize] {
                dist[v as usize] = dist[u as usize] + w;
            }
        }
    }
    // Una pasada más: si algo todavía se relaja, hay ciclo negativo.
    for &(u, v, w) in aristas {
        if dist[u as usize] + w < dist[v as usize] {
            return true;
        }
    }
    false
}
```

## 4.9 Ejercicios propuestos

1. **(F)** Calcula el camino más corto entre cada par de vértices de un grafo de 4 nodos. (Pista: Floyd-Warshall, que verás en otro capítulo.)
2. **(F)** Modifica Dijkstra para que devuelva la **suma de longitudes de los K caminos más cortos** desde el origen.
3. **(M)** Dado un grafo con pesos representando tiempos de viaje, calcula el camino más rápido desde tu casa al trabajo, asumiendo que hay un atasco conocido de 8:30 a 9:00 que afecta a ciertas aristas. ¿Cómo modelarías el atasco? (Pista: aristas con peso dependiente del tiempo.)
4. **(M)** Implementa el algoritmo de **A*** (A-estrella), que es Dijkstra con una heurística. Útil para mapas: la heurística suele ser la distancia en línea recta hasta el destino.
5. **(D)** Implementa el algoritmo de **Johnson** para caminos más cortos entre todos los pares con pesos negativos, combinando Bellman-Ford + Dijkstra. (Spoiler: Johnson's es polinomial y maneja pesos negativos, ganándole a Floyd-Warshall en grafos dispersos.)

## 4.10 Lo que te llevas

- **Dijkstra** resuelve el camino más corto desde un origen en grafos con pesos no negativos en O((V+E) log V) usando un **heap**.
- En Rust, el `BinaryHeap` es max-heap; usa `Reverse(...)` para convertirlo en min-heap. Es idiomático y rápido.
- **Bellman-Ford** es O(V·E) pero soporta **pesos negativos** y detecta **ciclos negativos**.
- Petgraph te da ambos algoritmos listos: `petgraph::algo::dijkstra` y `petgraph::algo::bellman_ford`.
- Regla práctica: Dijkstra por defecto; Bellman-Ford cuando haya pesos negativos o necesites detectar ciclos negativos.
- A* (con heurística) es la versión "con GPS" de Dijkstra; lo verás más adelante.

## 4.11 Ojo, cuidado con…

- **Usar Dijkstra con pesos negativos.** NO funciona. El algoritmo asume que una vez procesado un vértice, su distancia es final; los pesos negativos rompen esa asunción.
- **Olvidar el `Reverse` en `BinaryHeap`.** Si no lo usas, tu "min-heap" será en realidad un max-heap, y tu Dijkstra "funcionará" pero dará resultados incorrectos en silencio. (Este es de los bugs más bonitos de debuggear.)
- **Overflow al sumar pesos.** En grafos con pesos grandes, `d + peso` puede desbordar. Usa `checked_add` o `saturating_add` o, mejor, usa `u64` o `i64` desde el principio.
- **Marcar visitados con un `bool` en Dijkstra.** No funciona bien. El truco es: cuando sacas un nodo del heap, si su `d > dist[u]`, ignóralo. Ese check es el alma del algoritmo.
- **Asumir que "no hay camino" significa "cero".** En Rust, si inicializas con `u32::MAX` o con `None`, te ahorras el bug clásico de "origen = 0, destino = 0, parece que hay camino de distancia 0".
- **En petgraph, no convertir el `HashMap` de salida a un `Vec`.** La salida de `dijkstra` es `HashMap<NodeIndex, _>`, no un array indexado por vértice. Si quieres acceso por índice, convierte tú.

## 4.12 Para profundizar

1. Dijkstra, E. W. (1956). "A note on two problems in connexion with graphs". *Numerische Mathematik*.
2. Bellman, R. (1958). "On a routing problem". *Quarterly of Applied Mathematics*.
3. Ford, L. R. (1956). *Network Flow Theory*. RAND Corporation.
4. Cormen et al. (2009). *Introduction to Algorithms*, capítulos 24 (Dijkstra) y 24.1 (Bellman-Ford).
5. Hart, P. E., Nilsson, N. J., Raphael, B. (1968). "A Formal Basis for the Heuristic Determination of Minimum Cost Paths". *IEEE Transactions on Systems Science and Cybernetics*. (El paper de A*.)
6. Fredman, M. L., Tarjan, R. E. (1987). "Fibonacci heaps and their uses in improved network optimization algorithms". *Journal of the ACM*. (La mejora teórica que llevó Dijkstra a O(E + V log V) con heaps de Fibonacci.)

## 4.13 Pin de batalla

- **Dijkstra con `BinaryHeap` de Rust es prácticamente óptimo.** `petgraph` ya lo trae, pero entiende la mecánica.
- **Si los pesos son pequeños (enteros 0-100), usa Dial's implementation o 0-1 BFS.** Más rápido que el heap genérico.
- **Bellman-Ford te dice si hay ciclo negativo en el grafo.** Si te importa ese caso, no hay alternativa: Dijkstra miente.
- **A* gana a Dijkstra si tienes una heurística admisible.** Para mapas, distancia Manhattan o euclídea.
- **En grafos enormes, considera contraction hierarchies o ALT.** Dijkstra "puro" no escala a millones de nodos sin preprocessing.


## 4.14 Si solo lees 30 segundos

Dijkstra para pesos no negativos, Bellman-Ford si los hay. A* si tienes heurística. Bellman-Ford detecta ciclos negativos, Dijkstra no.

## 4.15 Una historia pequeña

Marc, desarrollador en una empresa de logística, llevaba meses sufriendo: los camiones de la empresa no optimizaban bien las rutas. Un día leyó sobre Dijkstra y pensó: "esto es lo que necesito." Reescribió el motor de rutas en una semana. Los camiones empezaron a ahorrar un 23% de combustible. El CEO le preguntó: "¿y por qué no lo hiciste antes?" Marc: "porque no sabía que existía." El CEO: "y los 6 meses de gasolina que hemos quemado de más, ¿quién me los paga?" Marc buscó trabajo en otra empresa. La moraleja: conoce los algoritmos antes de que te los pregunten.


---
