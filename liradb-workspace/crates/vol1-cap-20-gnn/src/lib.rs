//! Vol.I — Capítulo 20: Grafos en Machine Learning.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §20.3.
//!
//! - [`GcnLayer`] — Una capa GCN: H' = σ(Ã · H · W + b).
//! - [`normalized_adjacency`] — Construye Ã = D̃^(-1/2) · (A + I) · D̃^(-1/2).
//! - [`Gnn`] — Red GCN completa (pila de capas).
//!
//! Requiere `ndarray 0.15`, `ndarray-rand 0.14`, `rand 0.9`.

use ndarray::{Array1, Array2, Axis};

#[derive(Clone, Copy)]
pub enum Activation {
    Relu,
    None,
}

/// Una capa de GCN: transforma H ∈ R^{n×d_in} a H' ∈ R^{n×d_out}.
pub struct GcnLayer {
    pub weights: Array2<f64>, // W ∈ R^{d_in × d_out}
    pub bias: Array1<f64>,    // b ∈ R^{d_out}
    pub activation: Activation,
}

impl GcnLayer {
    /// Construye una capa con pesos inicializados aleatoriamente (He-like).
    pub fn new(d_in: usize, d_out: usize, seed: u64, activation: Activation) -> Self {
        use ndarray_rand::RandomExt;
        use ndarray_rand::rand_distr::Uniform;
        use rand::SeedableRng;

        // Inicialización: distribución uniforme en [-1/√d_in, 1/√d_in].
        let scale = 1.0 / (d_in as f64).sqrt();
        let dist = Uniform::new(-scale, scale);
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let weights = Array2::random_using((d_in, d_out), dist, &mut rng);
        let bias = Array1::zeros(d_out);
        Self {
            weights,
            bias,
            activation,
        }
    }

    /// Forward pass: H_new = σ(Ã · H · W + b).
    pub fn forward(&self, a_hat: &Array2<f64>, h: &Array2<f64>) -> Array2<f64> {
        // Paso 1: Ã · H  (n × d_in)
        let ah = a_hat.dot(h);
        // Paso 2: (Ã·H) · W  (n × d_out)
        let mut z = ah.dot(&self.weights);
        // Paso 3: añadir bias (broadcasting)
        for mut row in z.axis_iter_mut(Axis(0)) {
            row += &self.bias;
        }
        // Paso 4: activación
        match self.activation {
            Activation::Relu => z.mapv(|x| x.max(0.0)),
            Activation::None => z,
        }
    }
}

/// Construye la matriz de adyacencia normalizada con auto-loops:
///   Ã = D̃^(-1/2) · (A + I) · D̃^(-1/2)
/// donde D̃ es la matriz de grados de (A + I).
///
/// `edges` es la lista de aristas (no dirigido).
pub fn normalized_adjacency(n: usize, edges: &[(usize, usize)]) -> Array2<f64> {
    let mut a = Array2::<f64>::zeros((n, n));
    let mut deg = vec![0.0f64; n];
    for &(u, v) in edges {
        a[[u, v]] = 1.0;
        a[[v, u]] = 1.0;
        deg[u] += 1.0;
        deg[v] += 1.0;
    }
    // Auto-loops: A + I.
    for i in 0..n {
        a[[i, i]] += 1.0;
        deg[i] += 1.0;
    }
    // D̃^(-1/2)
    let d_inv_sqrt: Array1<f64> = deg
        .iter()
        .map(|&d| if d > 0.0 { 1.0 / d.sqrt() } else { 0.0 })
        .collect();
    // Ã = D^(-1/2) · A · D^(-1/2)
    let mut a_hat = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        for j in 0..n {
            a_hat[[i, j]] = d_inv_sqrt[i] * a[[i, j]] * d_inv_sqrt[j];
        }
    }
    a_hat
}

/// Una GNN apilada: pila de capas GCN.
pub struct Gnn {
    pub layers: Vec<GcnLayer>,
}

impl Gnn {
    /// `layer_dims = [(d_in_0, d_out_0), (d_in_1, d_out_1), ...]`.
    /// Cada capa menos la última lleva ReLU; la última no.
    pub fn new(layer_dims: &[(usize, usize)], seed: u64) -> Self {
        let mut layers = Vec::new();
        for (i, &(d_in, d_out)) in layer_dims.iter().enumerate() {
            let act = if i + 1 < layer_dims.len() {
                Activation::Relu
            } else {
                Activation::None
            };
            layers.push(GcnLayer::new(d_in, d_out, seed + i as u64, act));
        }
        Self { layers }
    }

    pub fn forward(&self, a_hat: &Array2<f64>, h0: &Array2<f64>) -> Array2<f64> {
        let mut h = h0.clone();
        for layer in &self.layers {
            h = layer.forward(a_hat, &h);
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capa_produce_tamano_correcto() {
        // 4 nodos, 3 features → 2 features.
        let layer = GcnLayer::new(3, 2, 42, Activation::Relu);
        let a_hat = Array2::<f64>::eye(4);
        let h = Array2::<f64>::ones((4, 3));
        let out = layer.forward(&a_hat, &h);
        assert_eq!(out.shape(), &[4, 2]);
    }

    #[test]
    fn gnn_completa_forward() {
        // Camino 0-1-2-3, 4 nodos. Capa 3 -> 4 -> 2.
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let a_hat = normalized_adjacency(4, &edges);
        let gnn = Gnn::new(&[(3, 4), (4, 2)], 0);
        let h0 = Array2::<f64>::ones((4, 3));
        let out = gnn.forward(&a_hat, &h0);
        assert_eq!(out.shape(), &[4, 2]);
        // Cada nodo debe tener un embedding (no NaN).
        for &v in out.iter() {
            assert!(v.is_finite(), "embeddings contienen NaN/inf");
        }
    }

    #[test]
    fn normalized_adjacency_kn() {
        // K3 (3 nodos, todos conectados).
        let edges = vec![(0, 1), (1, 2), (0, 2)];
        let a_hat = normalized_adjacency(3, &edges);
        // K3 regular tiene todos los auto-loops normalizados a 1/3.
        // Ã[i][i] = 1/deg_tilde(i) = 1/3 (deg_tilde = 3: 2 vecinos + 1 self-loop).
        // Tras multiplicar por D^(-1/2) en ambos lados: (1/sqrt(3)) * 1 * (1/sqrt(3)) = 1/3.
        for i in 0..3 {
            assert!((a_hat[[i, i]] - 1.0 / 3.0).abs() < 1e-9);
        }
    }
}
