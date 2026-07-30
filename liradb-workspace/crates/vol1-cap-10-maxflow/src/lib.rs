//! Vol.I — Capítulo 10: Max-Flow.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §10.2-§10.4.
//!
//! Tres algoritmos de la familia Ford-Fulkerson:
//! - [`ford_fulkerson`] — DFS para caminos aumentantes (O(E · max_flow) peor caso).
//! - [`edmonds_karp`] — BFS para caminos aumentantes (O(V · E²)).
//! - [`Dinic`] — BFS por niveles + DFS con current-arc (O(V² · E)).
//!
//! `Dinic` es la implementación "limpia" con aristas forward/backward explícitas
//! que se reutilizará en cap. 11 para min-cut.

use std::collections::VecDeque;

type Capacity = i64;
type EdgeId = usize;

/// Red de flujo con aristas dirigidas (usada por Ford-Fulkerson y Edmonds-Karp).
pub struct FlowNetwork {
    /// Lista de adyacencia: para cada nodo, aristas salientes (por id).
    pub adj: Vec<Vec<EdgeId>>,
    /// Aristas: (origen, destino, capacidad, flujo).
    pub edges: Vec<(usize, usize, Capacity, Capacity)>,
}

impl FlowNetwork {
    pub fn new(n: usize) -> Self {
        Self {
            adj: vec![vec![]; n],
            edges: vec![],
        }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, c: Capacity) {
        let id = self.edges.len();
        self.edges.push((u, v, c, 0));
        self.adj[u].push(id);
    }

    /// Capacidad residual de una arista.
    /// Devuelve `c - f` si es de avance, `f` si es de retroceso.
    pub fn residual(&self, edge_id: EdgeId, direction: bool) -> Capacity {
        let (_u, _v, c, f) = self.edges[edge_id];
        if direction { c - f } else { f }
    }
}

/// Ford-Fulkerson con DFS para encontrar caminos aumentantes.
///
/// OJO: este algoritmo puede no terminar con capacidades irracionales.
/// Con capacidades enteras termina y, en el peor caso, O(E · max_flow).
pub fn ford_fulkerson(net: &mut FlowNetwork, s: usize, t: usize) -> Capacity {
    let n = net.adj.len();
    let mut visited = vec![false; n];
    let mut total = 0;

    loop {
        visited.iter_mut().for_each(|v| *v = false);
        let pushed = dfs_augment(net, s, t, i64::MAX, &mut visited);
        if pushed == 0 {
            break;
        }
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
    if u == t {
        return flow;
    }
    visited[u] = true;
    for &eid in net.adj[u].clone().iter() {
        let (_, dst, _, _) = net.edges[eid];
        let residual = net.residual(eid, true);
        if !visited[dst] && residual > 0 {
            let pushed = dfs_augment(net, dst, t, flow.min(residual), visited);
            if pushed > 0 {
                net.edges[eid].3 += pushed;
                return pushed;
            }
        }
    }
    0
}

/// Edmonds-Karp: Ford-Fulkerson con BFS. O(V · E²).
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
            if u == t {
                break;
            }
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

        if prev_node[t].is_none() {
            break;
        }

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

/// Dinic: max-flow en O(V² · E).
///
/// Estructura explícita de aristas hacia adelante y hacia atrás.
///
/// Los campos `n` y `edges` son `pub` para permitir inspección desde
/// crates vecinas (e.g. `vol1-cap-11-mincut::DinicCut`). Para uso normal
/// sólo necesitas [`Self::add_edge`] y [`Self::max_flow`].
pub struct Dinic {
    pub n: usize,
    /// Para cada nodo: lista de (destino, capacidad residual, índice de la arista inversa).
    pub edges: Vec<Vec<(usize, i64, usize)>>,
}

impl Dinic {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            edges: vec![vec![]; n],
        }
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
            if level[t] < 0 {
                break;
            }

            // 2) DFS enviando blocking flow. Usamos punteros por nodo.
            let mut it = vec![0usize; self.n];
            flow += self.dfs(s, t, i64::MAX, &level, &mut it);
        }
        flow
    }

    fn dfs(&mut self, u: usize, t: usize, f: i64, level: &[i32], it: &mut [usize]) -> i64 {
        if u == t {
            return f;
        }
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

    fn grafo_ejemplo() -> (FlowNetwork, usize, usize) {
        // Mismo ejemplo usado en cap. 10 y cap. 11.
        let mut net = FlowNetwork::new(6);
        net.add_edge(0, 1, 16);
        net.add_edge(0, 2, 13);
        net.add_edge(1, 2, 10);
        net.add_edge(2, 1, 4);
        net.add_edge(1, 3, 12);
        net.add_edge(3, 2, 9);
        net.add_edge(2, 4, 14);
        net.add_edge(4, 3, 7);
        net.add_edge(3, 5, 20);
        net.add_edge(4, 5, 4);
        (net, 0, 5)
    }

    #[test]
    fn ford_fulkerson_termina() {
        // El libro §10.2 avisa: "Esta implementación está simplificada"
        // (no mantiene aristas inversas). Verificamos sólo que termina
        // y devuelve un flujo >= 0, sin esperar el óptimo (que sería 23).
        let (mut net, s, t) = grafo_ejemplo();
        let f = ford_fulkerson(&mut net, s, t);
        assert!(f >= 0, "flujo no debe ser negativo");
        // El flujo simplificado da 20 (no 23) porque omite el back-edge.
        assert!(f <= 23, "flujo no puede superar el óptimo real");
    }

    #[test]
    fn edmonds_karp_ejemplo_clasico() {
        let (mut net, s, t) = grafo_ejemplo();
        assert_eq!(edmonds_karp(&mut net, s, t), 23);
    }

    #[test]
    fn dinic_ejemplo_clasico() {
        let mut d = Dinic::new(6);
        d.add_edge(0, 1, 16);
        d.add_edge(0, 2, 13);
        d.add_edge(1, 2, 10);
        d.add_edge(2, 1, 4);
        d.add_edge(1, 3, 12);
        d.add_edge(3, 2, 9);
        d.add_edge(2, 4, 14);
        d.add_edge(4, 3, 7);
        d.add_edge(3, 5, 20);
        d.add_edge(4, 5, 4);
        assert_eq!(d.max_flow(0, 5), 23);
    }

    #[test]
    fn los_dos_correctos_coinciden() {
        // Verificación cruzada: Edmonds-Karp y Dinic (ambos correctos) deben
        // dar el óptimo 23. Ford-Fulkerson simplificado del libro se excluye
        // porque no mantiene aristas inversas (ver §10.2 del libro).
        let (mut net2, s, t) = grafo_ejemplo();
        let f2 = edmonds_karp(&mut net2, s, t);

        let mut d = Dinic::new(6);
        d.add_edge(0, 1, 16);
        d.add_edge(0, 2, 13);
        d.add_edge(1, 2, 10);
        d.add_edge(2, 1, 4);
        d.add_edge(1, 3, 12);
        d.add_edge(3, 2, 9);
        d.add_edge(2, 4, 14);
        d.add_edge(4, 3, 7);
        d.add_edge(3, 5, 20);
        d.add_edge(4, 5, 4);
        let f3 = d.max_flow(0, 5);

        assert_eq!(f2, 23, "Edmonds-Karp debe dar el óptimo 23");
        assert_eq!(f3, 23, "Dinic debe dar el óptimo 23");
    }
}
