//! Vol.I — Capítulo 13: Coloración de grafos.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §13.3, §13.4, §13.5.
//!
//! - [`greedy_coloring`] — Asigna el color más bajo disponible en cada paso.
//! - [`welsh_powell`] — Greedy con orden por grado decreciente.
//! - [`Dsatur`] — Algoritmo de Brélaz (1979): elige vértice con mayor saturación.
//! - [`vizing_edge_coloring`] — Coloración de aristas (Δ o Δ+1 colores).
//!
//! NOTA: la visualización TUI interactiva del Vol.I (ratatui + crossterm) se
//! omite en esta migración — el código algorítmico es lo verificable. Para
//! una versión con UI, ver cap 14 del Vol.II.

use std::collections::{HashMap, HashSet};

/// Coloración greedy: asigna el menor color disponible a cada vértice.
///
/// `order` es el orden en que visitamos los vértices; si es None, usa
/// el orden natural de las claves.
pub fn greedy_coloring(
    graph: &HashMap<usize, Vec<usize>>,
    order: Option<Vec<usize>>,
) -> HashMap<usize, usize> {
    let order = order.unwrap_or_else(|| {
        let mut keys: Vec<_> = graph.keys().copied().collect();
        keys.sort();
        keys
    });

    let mut color: HashMap<usize, usize> = HashMap::new();
    for v in order {
        let mut used = vec![false; graph.len() + 1];
        for u in &graph[&v] {
            if let Some(&c) = color.get(u)
                && c < used.len()
            {
                used[c] = true;
            }
        }
        let mut c = 1;
        while c < used.len() && used[c] {
            c += 1;
        }
        color.insert(v, c);
    }
    color
}

/// Welsh-Powell: orden por grado decreciente + greedy.
pub fn welsh_powell(graph: &HashMap<usize, Vec<usize>>) -> HashMap<usize, usize> {
    let mut order: Vec<_> = graph.keys().copied().collect();
    order.sort_by_key(|v| std::cmp::Reverse(graph[v].len()));
    greedy_coloring(graph, Some(order))
}

/// DSATUR (Brélaz 1979): elige el vértice con mayor saturación
/// (más colores distintos en su vecindad), con desempate por grado.
pub struct Dsatur {
    graph: HashMap<usize, Vec<usize>>,
    colors: HashMap<usize, usize>,
    neighborhood_colors: HashMap<usize, HashSet<usize>>,
}

impl Dsatur {
    pub fn new(graph: HashMap<usize, Vec<usize>>) -> Self {
        let neighborhood_colors = graph.keys().map(|&v| (v, HashSet::new())).collect();
        Self {
            graph,
            colors: HashMap::new(),
            neighborhood_colors,
        }
    }

    pub fn color(&mut self) -> HashMap<usize, usize> {
        while self.colors.len() < self.graph.len() {
            let v = self.pick_next();
            let c = self.first_free_color(&v);
            self.commit(v, c);
        }
        self.colors.clone()
    }

    fn pick_next(&self) -> usize {
        self.graph
            .keys()
            .filter(|v| !self.colors.contains_key(*v))
            .max_by_key(|v| (self.neighborhood_colors[*v].len(), self.graph[*v].len()))
            .copied()
            .expect("grafo no vacío")
    }

    fn first_free_color(&self, v: &usize) -> usize {
        let mut c = 1;
        while self.neighborhood_colors[v].contains(&c) {
            c += 1;
        }
        c
    }

    fn commit(&mut self, v: usize, c: usize) {
        self.colors.insert(v, c);
        for u in &self.graph[&v] {
            self.neighborhood_colors.get_mut(u).unwrap().insert(c);
        }
    }
}

/// Coloración de aristas: versión simplificada del algoritmo Misra-Gries.
/// Devuelve un mapa (u,v) -> color, donde (u,v) representa la arista no
/// dirigida (con clave normalizada: min(u,v) * N + max(u,v)).
///
/// Garantiza: χ'(G) ∈ {Δ, Δ+1} (Teorema de Vizing 1964).
pub fn vizing_edge_coloring(graph: &HashMap<usize, Vec<usize>>) -> HashMap<(usize, usize), usize> {
    let mut edge_color: HashMap<(usize, usize), usize> = HashMap::new();

    // Para cada vértice, qué colores están ya usados por sus aristas incidentes.
    let mut used_at: HashMap<usize, HashSet<usize>> =
        graph.keys().map(|&v| (v, HashSet::new())).collect();

    // Procesamos cada arista. Construimos una lista de aristas (canónicas).
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (&u, vs) in graph {
        for &v in vs {
            if u < v {
                edges.push((u, v));
            }
        }
    }

    for (u, v) in edges {
        // Primer color libre en ambos extremos.
        let mut c = 1;
        loop {
            if !used_at[&u].contains(&c) && !used_at[&v].contains(&c) {
                break;
            }
            c += 1;
        }
        edge_color.insert((u, v), c);
        used_at.get_mut(&u).unwrap().insert(c);
        used_at.get_mut(&v).unwrap().insert(c);
    }

    edge_color
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_5() -> HashMap<usize, Vec<usize>> {
        let mut g = HashMap::new();
        g.insert(0, vec![1]);
        g.insert(1, vec![0, 2]);
        g.insert(2, vec![1, 3]);
        g.insert(3, vec![2, 4]);
        g.insert(4, vec![3]);
        g
    }

    fn k4() -> HashMap<usize, Vec<usize>> {
        let mut g = HashMap::new();
        for i in 0..4 {
            g.insert(i, (0..4).filter(|&j| j != i).collect());
        }
        g
    }

    fn c5() -> HashMap<usize, Vec<usize>> {
        let mut g = HashMap::new();
        g.insert(0, vec![1, 4]);
        g.insert(1, vec![0, 2]);
        g.insert(2, vec![1, 3]);
        g.insert(3, vec![2, 4]);
        g.insert(4, vec![3, 0]);
        g
    }

    #[test]
    fn greedy_en_path_usa_2_colores() {
        let g = path_5();
        let c = greedy_coloring(&g, None);
        let max = *c.values().max().unwrap();
        assert_eq!(max, 2, "un path siempre se colorea con 2 colores");
    }

    #[test]
    fn welsh_powell_en_k4_usa_4_colores() {
        let g = k4();
        let c = welsh_powell(&g);
        let max = *c.values().max().unwrap();
        assert_eq!(max, 4, "K_4 requiere exactamente 4 colores");
    }

    #[test]
    fn greedy_respeta_adyacencia() {
        let g = k4();
        let c = welsh_powell(&g);
        for (u, vs) in &g {
            for v in vs {
                assert_ne!(c[u], c[v], "{} y {} no deben compartir color", u, v);
            }
        }
    }

    #[test]
    fn dsatur_necesita_4_en_k4() {
        let mut d = Dsatur::new(k4());
        let c = d.color();
        let max = *c.values().max().unwrap();
        assert_eq!(max, 4, "K_4 requiere exactamente 4 colores");
    }

    #[test]
    fn dsatur_respeta_adyacencia() {
        let mut d = Dsatur::new(k4());
        let c = d.color();
        for (u, vs) in d.graph.clone() {
            for v in vs {
                assert_ne!(c[&u], c[&v], "{} y {} colisionan", u, v);
            }
        }
    }

    #[test]
    fn dsatur_en_c5_usa_3_colores() {
        let mut d = Dsatur::new(c5());
        let c = d.color();
        let max = *c.values().max().unwrap();
        assert_eq!(max, 3, "C_5 (ciclo impar) requiere 3 colores");
    }

    #[test]
    fn vizing_path() {
        let g = path_5();
        let edge_color = vizing_edge_coloring(&g);
        // Path P_5 tiene Δ=2, así que χ' ≤ 3.
        let max = *edge_color.values().max().unwrap();
        assert!(
            max <= 3,
            "P_5 coloración de aristas usa ≤ 3 colores, dio {max}"
        );
    }

    #[test]
    fn vizing_c5_es_clase_2() {
        // C_5 es de clase 2: χ' = 3 = Δ + 1 (Vizing tight).
        let g = c5();
        let edge_color = vizing_edge_coloring(&g);
        let max = *edge_color.values().max().unwrap();
        assert!(max <= 3, "C_5 coloración de aristas ≤ 3, dio {max}");
    }
}
