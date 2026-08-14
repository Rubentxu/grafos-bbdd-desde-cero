# Capítulo 10 — Max-Flow: cómo vender todo el crudo que puedas

En 1956, dos investigadores de la RAND Corporation inventaron Ford-Fulkerson para calcular el flujo de crudo soviético hipotético que llegaría a Europa durante la Guerra Fría. Sí, en serio. Era un paper de la Fuerza Aérea de EE.UU. El algoritmo que hoy planifica evacuaciones de hospitales nació de la paranoia nuclear.
## 10.0 La anécdota del crudo soviético y la Fuerza Aérea

Estamos en 1956. La Guerra Fría está en su apogeo. Dos matemáticos de la RAND Corporation, una think tank financiada por la Fuerza Aérea de los Estados Unidos, reciben un encargo con aroma a bunker: modelar cómo los soviéticos podrían bombear crudo desde los Urales hasta Europa del Este. La pregunta operativa era: **¿cuál es la capacidad máxima de la red de oleoductos?**

Los dos matemáticos son Lester R. Ford Jr. y Delbert R. Fulkerson. Publican su paper en 1956 — *"Maximal flow through a network"*, Canadian Journal of Mathematics — y de paso inventan el algoritmo de Ford-Fulkerson, que es la base de prácticamente todo lo que vamos a ver en este capítulo. El algoritmo para una pregunta de logística militar de la Guerra Fría. Y de ahí saltó a logística, después a redes de telecomunicación, después a emparejamientos de mercados, después a *matchings* de ofertas de trabajo, después a segmentación de imágenes médicas.

Y sí, Fulkerson le puso a su algoritmo el nombre de su propio apellido. Eso en matemáticas se considera de mala educación. Pero como luego todo el mundo le llamó "Ford-Fulkerson" de todos modos, queda claro que Ford tenía mejor gusto para los nombres.

En este capítulo vamos a:
1. Definir formalmente una red de flujo.
2. Implementar Ford-Fulkerson.
3. Subir de nivel con Edmonds-Karp.
4. Llegar a Dinic, el algoritmo que de verdad se usa en producción.
5. Mencionar Push-Relabel.
6. Ver cómo petgraph se posiciona (spoiler: no trae max-flow).


> — Ford-Fulkerson o Edmonds-Karp o Dinic, ¿cuál?
> — Ford-Fulkerson es la familia. Edmonds-Karp es Ford-Fulkerson con BFS. Dinic es lo que se usa en serio.
> — ¿Por qué?
> — Dinic es O(V²·E). Para grafos grandes, es el más rápido en práctica. Edmonds-Karp es O(V·E²), más fácil de implementar.
> — ¿Cuándo uso Ford-Fulkerson puro?
> — Solo para enseñar. No lo uses en producción.
> — Vale. ¿Y push-relabel?
> — Más rápido en teoría (O(V³)), más difícil de implementar. Solo si tienes implementaciones de referencia.
## 10.1 Redes de flujo: definiciones

Una **red de flujo** es un grafo dirigido `G = (V, E)` con:

- Una **fuente** `s` y un **sumidero** `t`.
- Cada arista `(u, v)` tiene una **capacidad** `c(u, v) ≥ 0`.
- Un **flujo** `f(u, v)` en cada arista, con:
  - `0 ≤ f(u, v) ≤ c(u, v)` (no se excede la capacidad).
  - **Conservación de flujo**: para todo nodo `u ≠ s, t`, la cantidad que entra = la cantidad que sale. Es decir, `Σ f(v, u) = Σ f(u, w)`.
- El **valor del flujo** es la cantidad total que sale de `s` (o llega a `t`).

El **problema de max-flow**: maximizar el valor del flujo de `s` a `t`.

```
Ejemplo visual:

   s                  t
   | 5                |
   v                  v
   1 --3--> 2 --4--> 3
   |                   ^
   +-----2-------------+

Capacidades: s->1: 5, 1->2: 3, 2->3: 4, 1->3: 2.
Flujo máximo: 5+2 = 7, usando s->1 (3) -> 2 -> 3 y s->1 (2) -> 3.
```

## 10.2 Ford-Fulkerson: la idea de los caminos aumentantes

La intuición es brillante y muy visual:

1. Empieza con `f = 0` en todas las aristas.
2. Encuentra un **camino aumentante** de `s` a `t` en el **grafo residual**.
3. El **grafo residual** tiene, para cada arista `(u, v)` con capacidad `c` y flujo `f`, dos aristas:
   - Una arista de avance `(u, v)` con capacidad residual `c - f`.
   - Una arista de retroceso `(v, u)` con capacidad residual `f`.
4. Aumenta el flujo a lo largo de ese camino por la **mínima capacidad residual** del camino.
5. Repite hasta que no haya más caminos aumentantes.

El grafo residual es la clave: modela cuánto flujo *se puede aún* enviar por cada arista (capacidad restante) y cuánto se puede *devolver* (porque podemos "deshacer" un envío si encontramos un mejor camino).

```rust
use std::collections::HashMap;
use std::collections::VecDeque;

type Capacity = i64;
type EdgeId = usize;

/// Red de flujo con aristas dirigidas.
pub struct FlowNetwork {
    /// Lista de adyacencia: para cada nodo, aristas salientes.
    pub adj: Vec<Vec<EdgeId>>,
    /// Aristas: (origen, destino, capacidad, flujo).
    pub edges: Vec<(usize, usize, Capacity, Capacity)>,
}

impl FlowNetwork {
    pub fn new(n: usize) -> Self {
        Self { adj: vec![vec![]; n], edges: vec![] }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, c: Capacity) {
        let id = self.edges.len();
        self.edges.push((u, v, c, 0));
        self.adj[u].push(id);
    }

    /// Capacidad residual de una arista.
    /// Devuelve c - f si es de avance, f si es de retroceso.
    pub fn residual(&self, edge_id: EdgeId, direction: bool) -> Capacity {
        let (u, v, c, f) = self.edges[edge_id];
        if direction { c - f } else { f }
    }
}

/// Ford-Fulkerson con DFS para encontrar caminos aumentantes.
/// OJO: este algoritmo puede no terminar con capacidades irracionales.
/// Con capacidades enteras termina y, en el peor caso, O(E · max_flow).
pub fn ford_fulkerson(net: &mut FlowNetwork, s: usize, t: usize) -> Capacity {
    let n = net.adj.len();
    let mut visited = vec![false; n];
    let mut total = 0;

    loop {
        visited.iter_mut().for_each(|v| *v = false);
        let pushed = dfs_augment(net, s, t, i64::MAX, &mut visited);
        if pushed == 0 { break; }
        total += pushed;
    }
    total
}

fn dfs_augment(
    net: &mut FlowNetwork,
    u: usize,
    t: usize,
    flow: Capacity,
    visited: &mut [bool],
) -> Capacity {
    if u == t { return flow; }
    visited[u] = true;
    for &eid in &net.adj[u].clone() {
        let (src, dst, _, _) = net.edges[eid];
        let residual = net.residual(eid, true);
        if !visited[dst] && residual > 0 {
            let pushed = dfs_augment(net, dst, t, flow.min(residual), visited);
            if pushed > 0 {
                // Actualizamos el flujo en la arista y creamos su gemela inversa si no existe.
                net.edges[eid].3 += pushed;
                // Aquí deberíamos tener una arista inversa. Por simplicidad lo
                // modelamos con un grafo residual separado, como hace Dinic.
                return pushed;
            }
        }
    }
    0
}
```

Esta implementación está simplificada. La versión canónica usa dos aristas por cada arista original (la directa y la "antigua", también llamada back edge) para modelar el residual. Cuando aumentas flujo por la directa, aumentas la capacidad de la back edge; cuando reduces flujo, la reduces. Esa es la implementación "limpia" y la verás en Dinic.

## 10.3 Edmonds-Karp: la misma idea, pero con BFS

Ford-Fulkerson con DFS puede tardar O(E · max_flow), que en el peor caso con capacidades grandes es horrible. **Edmonds-Karp** (1972) es la observación feliz de que si en lugar de DFS usamos BFS para encontrar el camino aumentante, el algoritmo termina en **O(V·E²)**.

La diferencia es conceptual: el camino más corto (en número de aristas) garantiza un progreso uniforme. No es que BFS sea mágico, es que la cota de iteraciones se vuelve polinómica.

```rust
use std::collections::VecDeque;

/// Edmonds-Karp: Ford-Fulkerson con BFS.
/// O(V · E²).
pub fn edmonds_karp(net: &mut FlowNetwork, s: usize, t: usize) -> Capacity {
    let n = net.adj.len();
    let mut total = 0;

    loop {
        // BFS en el grafo residual.
        let mut prev_edge: Vec<Option<EdgeId>> = vec![None; n];
        let mut prev_node: Vec<Option<usize>> = vec![None; n];
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(s);

        while let Some(u) = queue.pop_front() {
            if u == t { break; }
            for &eid in &net.adj[u] {
                let (_, v, _, f) = net.edges[eid];
                let residual = net.edges[eid].2 - f;
                if residual > 0 && prev_node[v].is_none() && v != s {
                    prev_node[v] = Some(u);
                    prev_edge[v] = Some(eid);
                    queue.push_back(v);
                }
            }
        }

        if prev_node[t].is_none() { break; }

        // Calculamos la capacidad mínima del camino.
        let mut pushed = i64::MAX;
        let mut v = t;
        while let (Some(pn), Some(pe)) = (prev_node[v], prev_edge[v]) {
            let (_, _, c, f) = net.edges[pe];
            pushed = pushed.min(c - f);
            v = pn;
        }

        // Aplicamos.
        let mut v = t;
        while let (Some(pn), Some(pe)) = (prev_node[v], prev_edge[v]) {
            net.edges[pe].3 += pushed;
            v = pn;
        }
        total += pushed;
    }
    total
}
```

Fíjate: la estructura es exactamente la misma que Ford-Fulkerson, solo cambia la búsqueda del camino. Eso es lo bonito de la familia Ford-Fulkerson: es una *plantilla* con distintas estrategias de búsqueda.

## 10.4 Dinic: el algoritmo que se usa en serio

Dinic (pronunciado "Dínik", como un cosaco, porque su inventor, Yefim Dinitz, es de origen soviético yiddish) introduce dos ideas brillantes:

1. **BFS por niveles**: en lugar de buscar un camino aumentante cualquiera, construimos un **grafo de niveles** donde el nivel de un nodo es su distancia en número de aristas desde `s`. Solo consideramos aristas que van de nivel `k` a nivel `k+1`. Esto es el **grafo de capas** o **level graph**.
2. **Blocking flow**: en cada fase (cada BFS), enviamos un **flujo de bloqueo**, es decir, saturamos al menos una arista de cada camino aumentante del nivel actual.

La complejidad es O(V²·E), que en la práctica es excelente. Es el algoritmo que verás en competiciones de programación y en librerías de producción.

```rust
/// Dinic: max-flow en O(V²·E).
/// Estructura explícita de aristas hacia adelante y hacia atrás.
pub struct Dinic {
    n: usize,
    /// Aristas: destino, capacidad, índice de la arista inversa.
    edges: Vec<Vec<(usize, i64, usize)>>,
}

impl Dinic {
    pub fn new(n: usize) -> Self {
        Self { n, edges: vec![vec![]; n] }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, c: i64) {
        let fwd = self.edges[v].len();
        let bwd = self.edges[u].len();
        self.edges[u].push((v, c, fwd));
        self.edges[v].push((u, 0, bwd));
    }

    pub fn max_flow(&mut self, s: usize, t: usize) -> i64 {
        let mut flow = 0;
        loop {
            // 1) BFS para construir el grafo de niveles.
            let mut level = vec![-1i32; self.n];
            level[s] = 0;
            let mut queue: VecDeque<usize> = VecDeque::new();
            queue.push_back(s);
            while let Some(u) = queue.pop_front() {
                for &(v, cap, _) in &self.edges[u] {
                    if cap > 0 && level[v] < 0 {
                        level[v] = level[u] + 1;
                        queue.push_back(v);
                    }
                }
            }
            if level[t] < 0 { break; }

            // 2) DFS enviando blocking flow. Usamos punteros por nodo.
            let mut it = vec![0usize; self.n];
            flow += self.dfs(s, t, i64::MAX, &level, &mut it);
        }
        flow
    }

    fn dfs(&mut self, u: usize, t: usize, f: i64, level: &[i32], it: &mut [usize]) -> i64 {
        if u == t { return f; }
        for i in it[u]..self.edges[u].len() {
            let (v, cap, rev) = self.edges[u][i];
            if cap > 0 && level[v] == level[u] + 1 {
                let pushed = self.dfs(v, t, f.min(cap), level, it);
                if pushed > 0 {
                    self.edges[u][i].1 -= pushed;
                    self.edges[v][rev].1 += pushed;
                    return pushed;
                }
            }
            it[u] += 1;
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinic_ejemplo_clasico() {
        // s=0, t=5
        // 0->1 cap 16, 0->2 cap 13
        // 1->2 cap 10, 2->1 cap 4
        // 1->3 cap 12, 3->2 cap 9
        // 2->4 cap 14, 4->3 cap 7
        // 3->5 cap 20, 4->5 cap 4
        let mut d = Dinic::new(6);
        d.add_edge(0, 1, 16); d.add_edge(0, 2, 13);
        d.add_edge(1, 2, 10); d.add_edge(2, 1, 4);
        d.add_edge(1, 3, 12); d.add_edge(3, 2, 9);
        d.add_edge(2, 4, 14); d.add_edge(4, 3, 7);
        d.add_edge(3, 5, 20); d.add_edge(4, 5, 4);
        assert_eq!(d.max_flow(0, 5), 23);
    }
}
```

Detalles importantes de la implementación:

- Cada `add_edge` añade **dos** aristas: la directa y la inversa (con capacidad inicial 0). La inversa se llena con flujo a medida que "devolvemos" camino.
- El campo `rev` apunta a la posición de la arista inversa en la lista de adyacencia del otro nodo. Es un puntero relativo, así que aunque borremos o añadamos cosas a la lista, la referencia inversa sigue siendo válida.
- El `it[u]` es el puntero de iteración: en cada DFS, solo probamos aristas que aún no hemos intentado desde `u`. Esto se llama **optimización de current-arc** y es lo que lleva a Dinic a su complejidad teórica.

## 10.5 Push-Relabel (Goldberg): un mundo diferente

Push-relabel cambia el paradigma. En vez de buscar caminos y mantener conservación de flujo, **permite** que los nodos intermedios tengan exceso (más entrada que salida) y luego lo "empujan" hacia abajo en altura.

- Mantenemos una etiqueta de altura `h(u)` en cada nodo.
- Inicializamos `h(s) = V`, `h(t) = 0`, saturamos las aristas salientes de `s`.
- Mientras haya un nodo con exceso, hacemos **push** (empujar flujo a un vecino más bajo) o **relabel** (subir la altura del nodo).

Complejidad: O(V³) con la versión naive, O(V²·√E) con la versión más avanzada (HLPP, Highest-Label Pre-First-Push). En la práctica, HLPP es el algoritmo más rápido para grafos densos grandes.

No lo implementamos aquí, pero conviene saber que existe. Si alguna vez te encuentras con un grafo de flujo de 100.000 nodos y 500.000 aristas, HLPP te va a sacar del apuro donde Dinic se ahoga.

## 10.6 ¿Y `petgraph`? La verdad incómoda

Si buscas en la documentación de `petgraph`,你会发现 que **no tiene un módulo de max-flow**. ¿Por qué?

Petgraph se diseñó para ser una librería de **algoritmos de grafos generales**, no una librería de optimización combinatoria. Max-flow, matching, programación lineal: son problemas que viven en otra familia (la de optimización), y petgraph no quiso mezclarlos.

Pero hay solución: el crate **`petgraph-algo`** o, mejor aún, **`max-flow`** de terceros. Y, por supuesto, puedes usar `petgraph` para construir el grafo y luego correr tu propio Dinic encima.

```rust
// Ejemplo conceptual: petgraph para el grafo, Dinic para el flujo.
use petgraph::graph::DiGraph;

fn flujo_con_petgraph(n: usize, edges: &[(usize, usize, i64)], s: usize, t: usize) -> i64 {
    let g = DiGraph::<(), i64>::from_edges(edges.iter().map(|&(u, v, w)| (u, v, w)));
    let _ = g; // Para que el compilador no proteste.
    let mut d = Dinic::new(n);
    for &(u, v, w) in edges { d.add_edge(u, v, w); }
    d.max_flow(s, t)
}
```

En la práctica, yo suelo hacer **una de estas dos cosas**:

1. Usar `petgraph` solo para el grafo base (Dijkstra, BFS, DFS) y mantener una representación paralela específica para flujo.
2. Usar un crate dedicado de max-flow si el problema es grande y necesito HLPP o algoritmos específicos.

## 10.7 Aplicaciones: bipartite matching, cortes, y más

Max-flow es uno de esos algoritmos con un número obsceno de reducciones. Algunos clásicos:

- **Bipartite matching**: ¿cuántos emparejamientos máximos entre dos conjuntos? Modela origen → lado izquierdo → lado derecho → sumidero.
- **Vertex cover** (en bipartitos): se reduce a matching por König.
- **Edge disjoint paths**: ¿cuántos caminos s-t disjuntos en aristas caben?
- **Network reliability**, **image segmentation**, **project scheduling**...

En el próximo capítulo cubrimos la dualidad con min-cut, que es la otra cara de la moneda.

## 10.8 Ejercicios resueltos

### Ejercicio 1: Bipartite matching con max-flow

Tienes `n` desarrolladores y `m` proyectos. Cada desarrollador tiene una lista de proyectos en los que puede trabajar. Un desarrollador solo puede hacer un proyecto y un proyecto solo puede tener un desarrollador. ¿Cuál es el número máximo de emparejamientos?

**Solución:**

```rust
pub fn max_matching<'a>(
    devs: usize,
    projs: usize,
    puede: impl Fn(usize, usize) -> bool,
) -> i64 {
    let s = devs + projs;
    let t = s + 1;
    let mut d = Dinic::new(devs + projs + 2);
    for i in 0..devs { d.add_edge(s, i, 1); }
    for j in 0..projs { d.add_edge(devs + j, t, 1); }
    for i in 0..devs {
        for j in 0..projs {
            if puede(i, j) { d.add_edge(i, devs + j, 1); }
        }
    }
    d.max_flow(s, t)
}

#[test]
fn matching_basico() {
    // 3 devs, 3 proyectos
    // dev 0: projs 0, 1
    // dev 1: projs 1
    // dev 2: projs 2, 0
    let p = |d: usize, p: usize| -> bool {
        matches!((d, p), (0,0) | (0,1) | (1,1) | (2,2) | (2,0))
    };
    assert_eq!(max_matching(3, 3, p), 3);
}
```

### Ejercicio 2: Escape del laberinto (escape problem)

Una rejilla `n×n` con paredes. ¿Cuántos "soldados" pueden salir del laberinto si solo pueden moverse en 4 direcciones y cada celda admite un único paso?

**Solución:** modela cada celda como dos nodos (`in` y `out`) con capacidad 1 entre ellos. Conecta las celdas adyacentes. Saca el flujo del origen al sumidero virtual "fuera de la rejilla". Estándar de competencias.

### Ejercicio 3: Asignación de proyectos con capacidades

Variante del matching: cada proyecto admite hasta `c` desarrolladores. **Solución:** cambiar la capacidad de la arista `proyecto → t` a `c` en lugar de 1.

## 10.9 Ejercicios propuestos

1. **(Fácil)** Modifica `Dinic` para que devuelva también las **aristas saturadas** (donde `f = c`).
2. **(Fácil)** Implementa Ford-Fulkerson con **back edges explícitas** (no con la simplificación del residual) y compara empíricamente con Dinic.
3. **(Medio)** Dado un grafo bipartito, encuentra el **min vertex cover**. Pista: König, BFS en residual tras max-flow.
4. **(Medio)** Implementa un solver de **proyect scheduling**: tareas con duraciones y dependencias; ¿cuántos "tracks" paralelos necesitas para acabarlas?
5. **(Difícil)** Implementa HLPP (Highest-Label Pre-First-Push). Compáralo con Dinic con `criterion` en grafos de 1.000, 5.000 y 10.000 nodos.

## 10.10 Lo que te llevas

- **Ford-Fulkerson** es la idea conceptual: caminos aumentantes en el grafo residual.
- **Edmonds-Karp** es Ford-Fulkerson con BFS; cota O(V·E²).
- **Dinic** es el algoritmo de producción: BFS de niveles + DFS con blocking flow. O(V²·E), fácil de implementar, difícil de superar.
- **Push-Relabel / HLPP** es lo que necesitas para grafos enormes y densos.
- **Las reducciones** son el superpoder: matching, cortes, scheduling, escape problems... todo acaba siendo un max-flow.

## 10.11 Ojo, cuidado con…

- **Capacidades irracionales** en Ford-Fulkerson: el algoritmo puede no terminar. Usa Edmonds-Karp o Dinic siempre que puedas.
- **El "grafo residual"** es lo que más confunde. Recuerda: dos aristas por cada arista original (avance + retroceso).
- **Back edges** en Dinic: si las inicializas mal, todo se rompe. Verifica con un test pequeño antes de fiarte.
- **No confundas flujo con capacidad**. La capacidad es el "tope" del tubo; el flujo es cuánta agua pasa por él.
- **Punto de saturación**: si una arista tiene `f = c`, no puede llevar más flujo. Asegúrate de que tu DFS/BFS la ignora correctamente.

## 10.12 Para profundizar

1. **Ahuja, Magnanti, Orlin — *Network Flows***. EL libro. Capítulos 6-8 cubren Ford-Fulkerson, Edmonds-Karp y Dinic con demostraciones exquisitas.
2. **CP-Algorithms: Dinic** (<https://cp-algorithms.com/graph/dinic.html>). Implementación de referencia para programación competitiva.
3. **"Max-flow algorithms compared"** (benchmark interactivo): busca visualizaciones, las hay magníficas.
4. **El código fuente de `rust-graphflow`** y crates similares en crates.io para ver implementaciones industriales.
5. **El paper original de Ford y Fulkerson (1956)**: `https://www.jstor.org/stable/10095226`. Está en JSTOR; es de los papers más legibles de la historia.

## 10.13 Pin de batalla

- **Dinic es el rey en la práctica.** Implementa BFS levels + DFS blocking flows. Más rápido que Edmonds-Karp.
- **Si tu grafo es bipartito, max-flow = matching máximo.** Reducción clásica.
- **El grafo residual es la clave del algoritmo.** Siempre piensa en residual, no en el grafo original.
- **`petgraph` no tiene max-flow.** Implementa Dinic a mano o usa un crate externo.
- **Capacidades enteras pequeñas → Edmonds-Karp es suficiente.** Capacidades grandes o reales → Dinic o push-relabel.


## 10.14 Si solo lees 30 segundos

Max-flow encuentra cuánto se puede enviar de source a sink. Ford-Fulkerson (concepto), Edmonds-Karp (BFS), Dinic (niveles + blocking flows). El más usado es Dinic.

## 10.15 Una historia pequeña

Daisy era bombera en una ciudad mediana. Cuando había un incendio en un edificio grande, evacuar a todos los vecinos sin que se agolparan en las salidas era un caos. Un día, su hermano ingeniero le mostró el algoritmo de max-flow. Daisy modeló el edificio como una red: cada pasillo con su capacidad (personas por minuto), cada habitación como un nodo, las escaleras como aristas. Aplicó Dinic. Resultado: el plan de evacuación que tardaba 3 horas en planificarse, ahora lo tenía en 5 minutos. Y era mejor que los planes manuales. La jefa de bomberos le dijo: "¿y esto cómo lo aprendiste?" Daisy: "mi hermano, una servilleta de bar y un domingo." Le dieron un ascenso. La teoría de grafos salva vidas literalmente.


---

