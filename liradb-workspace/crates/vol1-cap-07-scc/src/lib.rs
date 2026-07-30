//! Vol.I — Capítulo 7: Union-Find (DSU) + Strongly Connected Components.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §7.7.
//!
//! Tres piezas:
//! - [`Dsu`] — Union-Find con path compression + union by size.
//! - [`kosaraju`] — SCC con dos pasadas de DFS (grafo + transpuesto).
//! - [`tarjan_scc`] — SCC con low-link values en una sola pasada.

/// DSU con dos optimizaciones: path compression + union by size.
#[derive(Debug)]
pub struct Dsu {
    parent: Vec<usize>,
    size: Vec<usize>,
    components: usize,
}

impl Dsu {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
            components: n,
        }
    }

    /// Encuentra la raíz del conjunto de `x` con path compression.
    pub fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
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

    fn dfs2(
        u: usize,
        c: i32,
        radj: &[Vec<usize>],
        comp_of: &mut [i32],
        sccs: &mut Vec<Vec<usize>>,
    ) {
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

    #[allow(clippy::too_many_arguments)]
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
            // raíz de una SCC: vaciar la pila hasta `u`.
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                scc.push(w);
                if w == u {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for u in 0..n {
        if disc[u] == -1 {
            strongconnect(
                u,
                adj,
                &mut disc,
                &mut low,
                &mut on_stack,
                &mut stack,
                &mut sccs,
                &mut time,
            );
        }
    }
    sccs
}

/// Calcula el grafo transpuesto (invierte cada arista).
pub fn transpose(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut radj = vec![vec![]; n];
    for (u, adj_u) in adj.iter().enumerate() {
        for &v in adj_u {
            radj[v].push(u);
        }
    }
    radj
}

#[cfg(test)]
mod tests_dsu {
    use super::*;

    #[test]
    fn basico() {
        let mut dsu = Dsu::new(5);
        assert_eq!(dsu.components(), 5);
        dsu.union(0, 1);
        dsu.union(2, 3);
        dsu.union(3, 4);
        // Bug del libro: faltaba union(0, 4) para conectar las dos componentes.
        dsu.union(0, 4);
        assert!(dsu.connected(0, 4));
        assert_eq!(dsu.components(), 1);
        // Otro bug del libro: con 5 elementos conectados debe haber 5 en
        // la componente, no 4. El glosario decía 4.
        assert_eq!(dsu.size_of(0), 5);
    }

    #[test]
    fn union_rechaza_repetidos() {
        let mut dsu = Dsu::new(3);
        assert!(dsu.union(0, 1));
        assert!(!dsu.union(0, 1));
        assert_eq!(dsu.components(), 2);
    }
}

#[cfg(test)]
mod tests_scc {
    use super::*;

    fn grafo_ejemplo() -> (usize, Vec<Vec<usize>>) {
        // Grafo clásico de SCC: 0->1->2->0 (SCC {0,1,2}), 2->3, 3->4->3 (SCC {3,4}).
        let n = 5;
        let adj = vec![vec![1], vec![2], vec![0, 3], vec![4], vec![3]];
        (n, adj)
    }

    #[test]
    fn kosaraju_encuentra_dos_scc() {
        let (n, adj) = grafo_ejemplo();
        let radj = transpose(n, &adj);
        let sccs = kosaraju(n, &adj, &radj);
        assert_eq!(sccs.len(), 2);
        // Verificar que cada vértice está en exactamente una SCC.
        let mut total = 0;
        for scc in &sccs {
            total += scc.len();
        }
        assert_eq!(total, n);
    }

    #[test]
    fn tarjan_encuentra_dos_scc() {
        let (n, adj) = grafo_ejemplo();
        let sccs = tarjan_scc(n, &adj);
        assert_eq!(sccs.len(), 2);
    }

    #[test]
    fn kosaraju_y_tarjan_coinciden_en_numero() {
        let (n, adj) = grafo_ejemplo();
        let radj = transpose(n, &adj);
        let sccs_k = kosaraju(n, &adj, &radj);
        let sccs_t = tarjan_scc(n, &adj);
        assert_eq!(sccs_k.len(), sccs_t.len());
    }
}
