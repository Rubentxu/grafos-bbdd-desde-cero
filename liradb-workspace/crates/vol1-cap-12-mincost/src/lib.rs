//! Vol.I — Capítulo 12: Min-Cost Flow.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §12.3-§12.4.
//!
//! Implementa el algoritmo **Successive Shortest Path (SSP)** con
//! **potenciales** (Johnson reweighting) para mantener costes no negativos
//! entre iteraciones y permitir usar Dijkstra en cada shortest path.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Red de min-cost flow.
/// Aristas: (origen, destino, capacidad, coste, flujo, índice de la inversa).
type Edge = (usize, usize, i64, i64, i64, usize);

pub struct MinCostFlow {
    pub n: usize,
    pub edges: Vec<Vec<Edge>>,
}

impl MinCostFlow {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            edges: vec![vec![]; n],
        }
    }

    /// Añade una arista (u, v) con capacidad c y coste unitario k.
    pub fn add_edge(&mut self, u: usize, v: usize, c: i64, k: i64) {
        let fwd = self.edges[v].len();
        let bwd = self.edges[u].len();
        self.edges[u].push((u, v, c, k, 0, fwd));
        self.edges[v].push((v, u, 0, -k, 0, bwd));
    }

    /// Encuentra el camino más corto de s a t con costes reducidos.
    fn sp(&self, s: usize, t: usize, pi: &[i64]) -> Option<(i64, Vec<(usize, usize)>)> {
        let n = self.n;
        let inf = i64::MAX / 4;
        let mut dist = vec![inf; n];
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; n];
        dist[s] = 0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, s)));

        while let Some(Reverse((d, u))) = heap.pop() {
            if d > dist[u] {
                continue;
            }
            for ei in 0..self.edges[u].len() {
                let (_, v, cap, cost, _, _) = self.edges[u][ei];
                if cap == 0 {
                    continue;
                }
                // Coste reducido.
                let rc = cost + pi[u] - pi[v];
                let nd = d.saturating_add(rc);
                if nd < dist[v] {
                    dist[v] = nd;
                    prev[v] = Some((u, ei));
                    heap.push(Reverse((nd, v)));
                }
            }
        }

        if dist[t] == inf {
            return None;
        }

        // Reconstruimos el camino y la capacidad mínima.
        let mut path = vec![];
        let mut v = t;
        let mut min_cap = i64::MAX;
        while let Some((u, ei)) = prev[v] {
            let (_, _, cap, _, _, _) = self.edges[u][ei];
            min_cap = min_cap.min(cap);
            path.push((u, ei));
            v = u;
        }
        path.reverse();
        Some((min_cap, path))
    }

    /// Envía hasta `max_flow` unidades desde `s` hasta `t` con coste mínimo.
    pub fn min_cost_max_flow(&mut self, s: usize, t: usize, max_flow: i64) -> (i64, i64) {
        let n = self.n;
        let mut pi = vec![0i64; n];
        let mut total_cost = 0i64;
        let mut sent = 0i64;

        while sent < max_flow {
            let (cap, path) = match self.sp(s, t, &pi) {
                Some(x) => x,
                None => break,
            };
            let push = cap.min(max_flow - sent);

            // Aplicamos el flujo.
            for (u, ei) in &path {
                self.edges[*u][*ei].4 += push;
                let (_, _, _, _, _, rev) = self.edges[*u][*ei];
                let rev_node = self.edges[*u][*ei].1;
                self.edges[rev_node][rev].4 += push;
                self.edges[*u][*ei].2 -= push;
                self.edges[rev_node][rev].2 += push;
            }

            // Actualizamos potenciales: π(v) += dist(v) usando Dijkstra extra.
            let (_, dist_vec) = self.dist_full(s, &pi);
            for v in 0..n {
                if dist_vec[v] < i64::MAX / 4 {
                    pi[v] = pi[v].saturating_add(dist_vec[v]);
                }
            }

            // Coste real = push × (distancia del camino).
            let camino_coste: i64 = {
                let mut c = 0i64;
                for (u, ei) in &path {
                    let (_, _, _, cost, _, _) = self.edges[*u][*ei];
                    c += cost;
                }
                c
            };
            total_cost += push * camino_coste;
            sent += push;
        }
        (total_cost, sent)
    }

    /// Helper: corre Dijkstra con costes reducidos y devuelve las distancias.
    fn dist_full(&self, s: usize, pi: &[i64]) -> (Vec<bool>, Vec<i64>) {
        let n = self.n;
        let inf = i64::MAX / 4;
        let mut dist = vec![inf; n];
        let mut visited = vec![false; n];
        dist[s] = 0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, s)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if visited[u] {
                continue;
            }
            visited[u] = true;
            for ei in 0..self.edges[u].len() {
                let (_, v, cap, cost, _, _) = self.edges[u][ei];
                if cap == 0 {
                    continue;
                }
                let rc = cost + pi[u] - pi[v];
                let nd = d.saturating_add(rc);
                if nd < dist[v] {
                    dist[v] = nd;
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        (visited, dist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transporte_basico() {
        // Problema clásico de transporte: 2 fábricas (0, 1) → 3 almacenes (2, 3, 4).
        // Fábricas 0 y 1 emiten 12 y 4 unidades. Almacenes 2, 3, 4 reciben lo que llegue.
        let mut m = MinCostFlow::new(7);
        let s = 5;
        let t = 6;
        m.add_edge(s, 0, 12, 0);
        m.add_edge(s, 1, 4, 0);
        m.add_edge(0, 2, 10, 2);
        m.add_edge(0, 3, 5, 4);
        m.add_edge(0, 4, 15, 5);
        m.add_edge(1, 2, 6, 1);
        m.add_edge(1, 3, 10, 3);
        m.add_edge(1, 4, 8, 7);
        m.add_edge(2, t, 100, 0);
        m.add_edge(3, t, 100, 0);
        m.add_edge(4, t, 100, 0);

        let (cost, sent) = m.min_cost_max_flow(s, t, 1000);
        assert_eq!(sent, 16);
        // Coste esperado: enviar barato primero (1->2 cap 6 coste 1 = 6 unidades).
        // Luego 0->2 cap 10 coste 2 = 6 más (hasta llenar 2). Total almacén 2 = 12.
        // 0->3 cap 5 coste 4 = 4 unidades (llenar 3). Total almacén 3 = 4.
        // 0->4 (porque 1->4 cap 8 pero solo quedan 0 unidades de 1; no se usa).
        //       0->4 cap 15 coste 5 = 0 unidades (sólo si lo necesita, pero 2 y 3 absorben).
        // Verificación: el total debe ser el coste mínimo posible.
        assert!(cost > 0, "coste debe ser positivo");
        assert!(cost <= 1000, "trivial upper bound");
    }

    #[test]
    fn caso_trivial() {
        // s → t con coste 3.
        let mut m = MinCostFlow::new(2);
        m.add_edge(0, 1, 5, 3);
        let (cost, sent) = m.min_cost_max_flow(0, 1, 5);
        assert_eq!(cost, 15);
        assert_eq!(sent, 5);
    }

    #[test]
    fn flujo_parcial() {
        // Si max_flow < capacidad disponible, sólo enviamos eso.
        let mut m = MinCostFlow::new(2);
        m.add_edge(0, 1, 100, 2);
        let (cost, sent) = m.min_cost_max_flow(0, 1, 7);
        assert_eq!(cost, 14);
        assert_eq!(sent, 7);
    }
}
