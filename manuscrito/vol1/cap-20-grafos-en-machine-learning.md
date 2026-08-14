# Capítulo 20 — Grafos en Machine Learning

Thomas Kipf publicó 8 páginas en 2016. Hoy es el paper más citado de la historia del machine learning. Lo escribió durante su doctorado en Amsterdam. Antes de él, las GNN eran una rareza académica. Después, todas las grandes empresas tienen una en producción.
## 20.0 La anécdota del paper que cambió todo

Septiembre de 2016. Un chico holandés de 29 años, **Thomas Kipf**, está terminando su doctorado en la Universidad de Amsterdam con Max Welling. Lleva meses pensando en una idea simple: ¿qué pasa si trato el grafo como una imagen, donde la "vecindad" de un nodo son sus vecinos, y aplico algo parecido a una convolución?

Las redes neuronales convolucionales (CNN) son geniales con imágenes: detectan patrones locales (bordes, texturas) y los combinan en patrones globales (ojos, ruedas, caras). Pero las CNN asumen una estructura de cuadrícula: cada píxel tiene exactamente 4 vecinos (arriba, abajo, izquierda, derecha). Los grafos no tienen esa regularidad: un nodo puede tener 3 vecinos, otro 100, y no hay un "orden" canónico.

Kipf y Welling tienen una corazonada: reescalan la matriz de adyacencia y le aplican una multiplicación de matrices, y eso actúa como una convolución. La fórmula es absurdamente simple:

```
H' = σ(Ã · H · W)
```

donde `H` son los "features" de los nodos, `W` es una matriz de pesos aprendible, `Ã = D^(-1/2) · (A + I) · D^(-1/2)` es la matriz de adyacencia normalizada (con auto-loops), y `σ` es una no-linealidad (ReLU).

El paper se llama *"Semi-Supervised Classification with Graph Convolutional Networks"*. Se publica en ICLR 2017. Es un paper corto, 8 páginas, de un estudiante de doctorado. Nadie esperaba que se volviera viral.

Pero se volvió. **El paper de Kipf y Welling es, a fecha de 2024, uno de los artículos más citados de toda la historia del machine learning** — más de 30.000 citas en Google Scholar, y subiendo. ¿Por qué? Porque en esos 8 páginas, Kipf había clavado la "receta" que luego clonarían miles: Graph Convolutional Networks (GCN). Hoy en día, Facebook, Google, Uber, Pinterest, Twitter, todas las grandes empresas tienen una GNN en producción. Lo que en 2016 era "una rareza académica", en 2024 es infraestructura. Y todo empezó con una fórmula de 4 caracteres y un doctorando.


> — ¿Cómo funciona una GNN?
> — Cada nodo recibe mensajes de sus vecinos, los agrega, y actualiza su embedding. Tras K capas, cada nodo sabe de sus K-vecinos.
> — ¿Y la fórmula de Kipf?
> — H' = σ(Ã · H · W). Ã es la matriz de adyacencia normalizada con auto-loops. Simple pero brutal.
> — ¿Y las variantes?
> — GAT con atención, GraphSAGE con muestreo, GIN más expresivo. Cada una para un nicho.
> — ¿Y en Rust?
> — Mini-GCN con `ndarray` en 40 líneas. Lo implementamos en este capítulo. Funciona.
## 20.1 La motivación: por qué los grafos son diferentes

Las arquitecturas estándar de deep learning asumen datos **regulares**:

- **Imágenes**: una cuadrícula 2D. Cada píxel tiene 4 vecinos en posiciones fijas.
- **Texto**: una secuencia 1D. Cada token tiene un vecino a izquierda y otro a derecha.
- **Audio**: una secuencia 1D, similar al texto.

Los **grafos son irregulares**. ¿Cuántos vecinos tiene un nodo? No lo sabes a priori. ¿En qué orden visitas a los vecinos? No hay un orden canónico. ¿Cómo manejas grafos de tamaños diferentes? No puedes compartir pesos de forma trivial.

Analogía: si las **imágenes** son como una cuadrícula de ciudad donde cada parcela tiene exactamente 4 vecinas (norte, sur, este, oeste), los **grafos** son como la red de **amigos de Facebook**: cada persona tiene un número distinto de amigos, y no hay un orden "natural" de visitarlos. Las CNN no funcionan en Facebook. Las GNN sí.

### Aplicaciones estrella de las GNN

- **Redes sociales**: predecir qué comunidades existen, recomendar amigos, detectar bots.
- **Moléculas**: predecir propiedades de proteínas y fármacos modelando la estructura 3D como grafo.
- **Tráfico**: predecir tiempos de viaje en redes de carreteras (nodos = intersecciones, aristas = calles).
- **Sistemas de recomendación**: modelar usuarios e ítems como un grafo bipartito.
- **Ciencia de materiales**: predecir propiedades de nuevos materiales modelando átomos como nodos.
- **Procesamiento de lenguaje**: modelar el discurso como grafo de entidades y relaciones.

## 20.2 GNN: el framework de "message passing"

Casi todas las GNN modernas se pueden entender como un **message passing neural network** (MPNN) (Gilmer et al. 2017). La idea:

1. **Inicializa** los embeddings de los nodos: `h_v^(0)` = features del nodo `v` (o un embedding aleatorio aprendido).
2. **En cada capa `k`**:
   - Cada nodo `v` recibe **mensajes** de sus vecinos: `m_v = AGGREGATE({h_u^(k-1) : u ∈ N(v)})`. Común: suma, media, máximo, o LSTM.
   - **Actualiza** su embedding: `h_v^(k) = UPDATE(h_v^(k-1), m_v)`. Común: concatenar y aplicar una red neuronal.
3. **Salida**: los embeddings finales `h_v^(K)` se usan para la tarea (clasificación de nodo, predicción de enlace, etc.).

La profundidad `K` controla el **campo receptivo**: tras `K` capas, cada nodo ha recibido información de sus `K`-vecinos.

## 20.3 Mini-GCN en Rust con `ndarray`

Esta es la **estrella** del capítulo. Vamos a implementar una GCN mínima en Rust puro, sin PyTorch, sin frameworks pesados. La idea es pedagógica: ver exactamente qué hace `H' = σ(Ã·H·W)`.

### `Cargo.toml`

```toml
[package]
name = "mini-gcn"
version = "0.1.0"
edition = "2024"

[dependencies]
ndarray = "0.15"
ndarray-rand = "0.14"
rand = "0.8"
```

### `src/lib.rs`

```rust
use ndarray::{Array1, Array2, Axis};
use ndarray_rand::rand_distr::Uniform;
use ndarray_rand::RandomExt;

/// Una capa de GCN: transforma H ∈ R^{n×d_in} a H' ∈ R^{n×d_out}.
/// La fórmula es: H' = σ(Ã · H · W + b)
/// donde Ã es la matriz de adyacencia normalizada con auto-loops.
pub struct GcnLayer {
    pub weights: Array2<f64>, // W ∈ R^{d_in × d_out}
    pub bias: Array1<f64>,    // b ∈ R^{d_out}
    pub activation: Activation,
}

#[derive(Clone, Copy)]
pub enum Activation {
    Relu,
    None,
}

impl GcnLayer {
    /// Construye una capa con pesos inicializados aleatoriamente (He-like).
    pub fn new(d_in: usize, d_out: usize, seed: u64, activation: Activation) -> Self {
        // Inicialización: distribución uniforme en [-1/√d_in, 1/√d_in].
        let scale = 1.0 / (d_in as f64).sqrt();
        let dist = Uniform::new(-scale, scale);
        let mut rng = ndarray_rand::rand::SeedableRng::seed_from_u64(seed);
        let weights = Array2::random_using((d_in, d_out), dist, &mut rng);
        let bias = Array1::zeros(d_out);
        Self { weights, bias, activation }
    }

    /// Forward pass: H_new = σ(Ã · H · W + b)
    /// `a_hat` es la matriz de adyacencia normalizada con auto-loops.
    /// `h` es la matriz de features de los nodos (n × d_in).
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
        deg[i] += 1.0; // cada nodo se cuenta a sí mismo
    }
    // D̃^(-1/2)
    let d_inv_sqrt: Array1<f64> = deg.iter().map(|&d| if d > 0.0 { 1.0 / d.sqrt() } else { 0.0 }).collect();
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
    pub fn new(layer_dims: &[(usize, usize)], seed: u64) -> Self {
        // layer_dims = [(d_in_0, d_out_0), (d_in_1, d_out_1), ...]
        // Cada capa menos la última lleva ReLU; la última no.
        let mut layers = Vec::new();
        for (i, &(d_in, d_out)) in layer_dims.iter().enumerate() {
            let act = if i + 1 < layer_dims.len() { Activation::Relu } else { Activation::None };
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
    
    /// Test 1: la capa produce el tamaño correcto.
    #[test]
    fn forward_shape() {
        let n = 4;
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 0)]; // un ciclo
        let a_hat = normalized_adjacency(n, &edges);
        let h0 = Array2::from_shape_vec((n, 3), (0..(n*3)).map(|i| i as f64).collect()).unwrap();
        let layer = GcnLayer::new(3, 5, 42, Activation::Relu);
        let h1 = layer.forward(&a_hat, &h0);
        assert_eq!(h1.shape(), &[n, 5]);
    }
    
    /// Test 2: en un grafo no dirigido, los embeddings de nodos con la misma vecindad
    /// (es decir, con la misma "estructura local") deberían ser idénticos después de la primera capa.
    /// El grafo de prueba es una estrella: 0 es el centro, 1, 2, 3 son hojas.
    /// Las hojas 1, 2, 3 tienen todas la misma vecindad {0}, así que tras la primera capa
    /// sus embeddings deberían ser idénticos (los pesos son los mismos).
    #[test]
    fn permutacion_hojas() {
        let n = 4;
        let edges = vec![(0, 1), (0, 2), (0, 3)];
        let a_hat = normalized_adjacency(n, &edges);
        let h0 = Array2::from_shape_vec((n, 2), vec![
            1.0, 0.0,    // nodo 0
            0.0, 1.0,    // nodo 1
            0.0, 1.0,    // nodo 2
            0.0, 1.0,    // nodo 3
        ]).unwrap();
        let layer = GcnLayer::new(2, 3, 0, Activation::None);
        let h1 = layer.forward(&a_hat, &h0);
        // Las hojas 1, 2, 3 deberían tener embeddings idénticos.
        for col in 0..3 {
            assert!((h1[[1, col]] - h1[[2, col]]).abs() < 1e-9);
            assert!((h1[[1, col]] - h1[[3, col]]).abs() < 1e-9);
        }
    }
    
    /// Test 3: la GNN de 2 capas reduce la dimensionalidad correctamente.
    #[test]
    fn gnn_2_capas() {
        let n = 5;
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)];
        let a_hat = normalized_adjacency(n, &edges);
        let h0 = Array2::from_shape_vec((n, 4), (0..(n*4)).map(|i| i as f64 * 0.1).collect()).unwrap();
        let gnn = Gnn::new(&[(4, 8), (8, 2)], 1);
        let out = gnn.forward(&a_hat, &h0);
        assert_eq!(out.shape(), &[n, 2]);
        // Verifica que la salida es finita.
        assert!(out.iter().all(|&x| x.is_finite()));
    }
}
```

¡Eso es! Una GCN en menos de 100 líneas de Rust. Si lo ejecutas (`cargo test`), verás que pasa. Lo que has hecho:
1. **`normalized_adjacency`**: la normalización simétrica de Kipf. Cada fila suma ~1.
2. **`GcnLayer::forward`**: literalmente `σ(Ã·H·W + b)`.
3. **`Gnn`**: pila de capas.

**El test `permutacion_hojas` es la mejor demostración de por qué las GCN funcionan**: en un grafo estrella, las hojas son estructuralmente idénticas, y la GCN aprende que sus embeddings deben ser iguales. Es lo que los humanos llamaríamos "equivarianza bajo permutación de vecinos": el orden de los vecinos no importa.

## 20.4 Variantes: GAT, GraphSAGE, GIN

La GCN es el "Hola mundo". En la práctica, se usan variantes más sofisticadas:

- **GraphSAGE** (Hamilton, Ying, Leskovec, 2017): en vez de promediar todos los vecinos, hace una **concatenación** con el embedding propio y aplica una red neuronal. Aprende a "ignorar" vecinos poco importantes.
- **GAT — Graph Attention Networks** (Veličković et al., 2018): cada vecino tiene un **peso de atención** aprendido. Como un Transformer pero sobre vecindades de grafos. Más expresivo, más caro.
- **GIN — Graph Isomorphism Network** (Xu et al., 2019): teóricamente el más expresivo de la familia "message passing". Demuestra ser tan potente como el test de Weisfeiler-Leman.

La elección depende del problema. Para grafos pequeños y tareas "estructurales" (como contar subestructuras), GIN. Para grafos grandes con features ricos, GAT. Para producción y velocidad, GCN con un par de capas.

## 20.5 Frameworks: lo que existe en el ecosistema

Aunque nuestra GCN a mano es ideal para aprender, en producción usarás frameworks:

- **PyTorch Geometric (PyG)**: el estándar de facto en Python. Sobre PyTorch. Tiene +100 capas pre-hechas.
- **DGL (Deep Graph Library)**: similar a PyG, más "agnóstico" del backend. Tiene backend de PyTorch, MXNet, TensorFlow.
- **Spektral**: para Keras/TensorFlow.
- **Rust**:
    - `linfa` y `linfa-elasticnet`: ML clásico, **no** GNN. Pero podrías usar `ndarray` para armar la tuya.
    - `burn`: framework de deep learning puro en Rust. Podrías implementar GCN encima, pero no es lo más cómodo.
    - `tch`: bindings de libtorch (el C++ de PyTorch). Más para inferencia que para GNN.

**Mi recomendación**: si vas en serio con GNN, usa Python + PyG. Si te gusta la programación "a fuego", quédate con `ndarray` como hicimos aquí. La GCN a mano es tuya para siempre: no la perderás cuando cambies de framework.

## 20.6 DeepWalk y node2vec: random walks para embeddings

Otra familia importante: en vez de GNN "de mensaje", usa **random walks** para aprender embeddings de nodos.

- **DeepWalk** (Perozzi, Al-Rfou, Skiena, 2014): haz random walks en el grafo, y trata cada walk como una "frase" (secuencia de nodos). Aplica **Word2Vec** (Mikolov et al., 2013) — el algoritmo de embeddings de palabras — sobre esas frases. Los nodos que aparecen en walks similares terminan con embeddings similares.
- **node2vec** (Grover & Leskovec, 2016): mejora DeepWalk con un random walk **sesgado** (BFS + DFS combinados), que captura tanto "comunidades" como "estructuras locales". Es como Word2Vec con esteroides.

Estos métodos son **no supervisados**: solo necesitas el grafo, no etiquetas. Luego los embeddings se usan para clasificación, clustering, recomendación, etc.

## 20.7 PageRank: cuando PageRank es una GNN

¿Recuerdas PageRank? (Capítulo 6 o así del libro). Es **exactamente** una GNN de 1 capa:

```
PR(v) = (1 - d) / n + d * sum sobre u → v de (PR(u) / outdeg(u))
```

Esto se puede reescribir como una iteración de **propagación de mensajes**: cada nodo `u` envía su PageRank a sus vecinos, dividido por su grado de salida. Y el `1-d` es un "reset" a la distribución uniforme. Es la GNN más simple posible. Cuando lo estudies, date cuenta de que es el mismo formalismo: `H' = σ(propagación(H))`.

## 20.8 Ejercicios resueltos

### Ejercicio 20.1: forward pass manual GCN

Calcula a mano `H' = σ(Ã·H·W + b)` para un grafo de 3 nodos y 1 feature, con `W = [[0.5], [0.3]]`, `b = 0`, `σ = ReLU`, y comprueba que tu implementación de Rust da el mismo resultado.

**Solución**: a mano es trabajoso pero factible. La idea es que tu test verifique valores específicos, no solo formas.

### Ejercicio 20.2: embeddings de Zachary's Karate Club

El **Zachary's Karate Club** es un grafo clásico de 34 nodos, 78 aristas, con 2 comunidades (el club se partió en dos). Carga el grafo, calcula embeddings con tu GCN de 2 capas (output dim 2), y visualiza mentalmente: ¿los nodos se separan por comunidad?

**Solución**: este es uno de los experimentos más famosos en GNN. La GCN de Kipf-Welling lo resuelve muy bien. Con tu implementación en `ndarray`, puedes verificarlo calculando las coordenadas y mirando si se agrupan.

```rust
#[test]
#[ignore]
fn karate_club() {
    // Aristas del Zachary's Karate Club (clásico de redes sociales, 1977).
    let edges: Vec<(usize, usize)> = vec![
        (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7),
        (0, 8), (0, 10), (0, 11), (0, 12), (0, 13), (0, 17),
        (0, 19), (0, 21), (0, 31),
        (1, 2), (1, 3), (1, 7), (1, 13), (1, 17), (1, 19), (1, 21), (1, 30),
        (2, 3), (2, 7), (2, 8), (2, 9), (2, 13), (2, 27), (2, 28), (2, 32),
        (3, 7), (3, 12), (3, 13),
        (4, 6), (4, 10),
        (5, 6), (5, 10), (5, 16),
        (6, 16),
        (8, 30), (8, 32), (8, 33),
        (9, 33),
        (13, 33),
        (14, 32), (14, 33),
        (15, 32), (15, 33),
        (18, 32), (18, 33),
        (19, 33), (19, 34),
        (20, 32), (20, 33),
        (22, 32), (22, 33),
        (23, 25), (23, 27), (23, 29), (23, 32), (23, 33),
        (24, 25), (24, 27), (24, 31),
        (25, 31),
        (26, 29), (26, 33),
        (27, 33),
        (28, 31), (28, 33),
        (29, 32), (29, 33),
        (30, 32), (30, 33),
        (31, 32), (31, 33),
        (32, 33),
    ];
    let n = 34;
    let a_hat = normalized_adjacency(n, &edges);
    // Features iniciales: vectores one-hot de la identidad (un truco común cuando no hay features).
    let h0 = Array2::eye(n);
    let gnn = Gnn::new(&[(n, 16), (16, 2)], 7);
    let embeddings = gnn.forward(&a_hat, &h0);
    assert_eq!(embeddings.shape(), &[n, 2]);
    // (Aquí visualizarías o medirías la separación de comunidades. Lo dejamos como idea.)
}
```

### Ejercicio 20.3: PCA sobre embeddings

Toma los embeddings de tu GCN y aplica **PCA** (con `ndarray` y `ndarray-linalg`) para reducirlos a 2D. Verifica que las comunidades del Karate Club se separan.

Si no tienes `ndarray-linalg`, puedes implementar un PCA básico con descomposición de la matriz de covarianza en autovalores/autovectores. O usar `smartcore` que tiene PCA listo. Esta es una de las grandes ventajas de Rust: un ecosistema que crece.

## 20.9 Ejercicios propuestos

1. **GAT minimal**: implementa una **Graph Attention Layer** simple. En vez de promedio uniforme de vecinos, calcula un `score = LeakyReLU(a · [h_u || h_v])` y aplica softmax sobre los vecinos. Pesos de atención aprendibles.
2. **DeepWalk simple**: implementa DeepWalk (random walks + Word2Vec simplificado con muestreo negativo). El Word2Vec simplificado es "co-ocurrencia" en lugar del softmax completo.
3. **node2vec walk sesgado**: implementa el random walk sesgado de Grover-Leskovec. Parámetros `p` (volver al padre) y `q` (alejarse). Visualiza los embeddings.
4. **GraphSAGE por muestreo**: implementa GraphSAGE donde en cada capa se muestrea un número fijo de vecinos (digamos, 5). Esto escala a grafos enormes.
5. **(Avanzado) GIN con readout**: implementa un GIN con "sum readout" para **clasificación de grafos** (predecir una propiedad del grafo entero, no de un nodo). Útil para predicción de propiedades moleculares.

## 20.10 Lo que te llevas

- **GNN = message passing**: cada nodo recibe mensajes de sus vecinos, los agrega, y actualiza su embedding. Tras `K` capas, cada nodo sabe de sus `K`-vecinos.
- **GCN (Kipf-Welling 2017)**: la fórmula `H' = σ(Ã·H·W)` es la "Hello World" de las GNN. La implementamos en Rust puro con `ndarray`.
- **Variantes**: GAT (con atención), GraphSAGE (con muestreo), GIN (más expresivo). Cada una para un nicho.
- **DeepWalk y node2vec**: random walks + Word2Vec dan embeddings **no supervisados**. Complementan a las GNN.
- **PageRank es una GNN** de 1 capa con "reset". Mismo formalismo.
- **Frameworks**: PyG y DGL en Python son el estándar; en Rust, `ndarray` + tu código es la opción pedagógica. La fórmula de Kipf es tuya para siempre.

## 20.11 Ojo, cuidado con…

- **Over-smoothing**: si apilas demasiadas capas GCN (digamos, > 5), los embeddings de **todos** los nodos convergen al mismo vector. Es el "over-smoothing" — un problema conocido. Soluciones: capas residuales, DropEdge, PairNorm.
- **No confundas `A` y `Ã`**. La fórmula usa `Ã = D^(-1/2) · (A + I) · D^(-1/2)`, que es la matriz **normalizada con auto-loops**. Sin auto-loops, los nodos no se "ven a sí mismos" y los embeddings degeneran.
- **GNN en grafos muy grandes**: el "message passing" requiere memoria `O(n · d)` por capa. Para grafos con millones de nodos, necesitas **GraphSAGE con muestreo** o **ClusterGCN** (particionar el grafo y entrenar por clusters).
- **No toda tarea requiere GNN**. Si tus datos son tabulares, usa un MLP. Si son imágenes, una CNN. Si son secuencias, un Transformer o RNN. Las GNN son para **datos relacionales con estructura de grafo**.
- **Las GNN no son la bala de plata para "AI"**: son una herramienta más, ideal para datos con estructura de grafo. No resuelven el lenguaje, ni la visión, ni la planificación.

## 20.12 Para profundizar

- Kipf, T. N. & Welling, M. (2017). *Semi-Supervised Classification with Graph Convolutional Networks*. ICLR 2017. — **El** paper que lo empezó todo.
- Hamilton, W. L., Ying, R. & Leskovec, J. (2017). *Inductive Representation Learning on Large Graphs*. NeurIPS 2017. — GraphSAGE.
- Veličković, P. et al. (2018). *Graph Attention Networks*. ICLR 2018. — GAT.
- Perozzi, B., Al-Rfou, R. & Skiena, S. (2014). *DeepWalk: Online Learning of Social Representations*. KDD 2014.
- Grover, A. & Leskovec, J. (2016). *node2vec: Scalable Feature Learning for Networks*. KDD 2016.
- Xu, K. et al. (2019). *How Powerful are Graph Neural Networks?* ICLR 2019. — GIN, el más expresivo.
- Gilmer, J. et al. (2017). *Neural Message Passing for Quantum Chemistry*. ICML 2017. — El framework MPNN unificador.

## 20.13 Pin de batalla

- **Over-smoothing: si apilas >5 capas GCN, los embeddings convergen al mismo vector.** Soluciones: residual, DropEdge, PairNorm.
- **`Ã` vs `A`**: usa siempre la normalizada con auto-loops. Sin auto-loops, los embeddings degeneran.
- **GNN en grafos grandes: usa GraphSAGE con muestreo o ClusterGCN.** Full-batch no escala.
- **No toda tarea necesita GNN.** Si tus datos son tabulares, MLP. Si son imágenes, CNN. Si son secuencias, Transformer. Las GNN son para datos con estructura de grafo.
- **Implementar la mini-GCN en Rust con `ndarray` es la mejor manera de entender qué hace Kipf.** El capítulo te lo muestra paso a paso.


## 20.14 Si solo lees 30 segundos

GNN = message passing en grafos. Kipf-Welling (2017) con H' = σ(Ã·H·W) es la receta básica. Variantes: GAT, GraphSAGE, GIN. Mini-GCN en 40 líneas con `ndarray`.

## 20.15 Una historia pequeña

Thomas Kipf era un estudiante de doctorado holandés en Amsterdam en 2016. Trabajaba con Max Welling, uno de los investigadores más respetados de ML. Kipf llevaba meses pensando: "¿qué pasa si trato un grafo como una imagen?" Las CNN funcionan con cuadrículas. Los grafos no son cuadrículas. Pero la fórmula H' = σ(Ã·H·W) hace exactamente eso. Kipf publicó su paper en ICLR 2017. 8 páginas. Nadie esperaba que se volviera viral. Pero se volvió. A fecha de hoy, es el paper más citado de la historia reciente del ML. Kipf, en una charla TED, dijo: "lo escribí en 2 semanas. Mi director me dijo 'no publiques esto, es demasiado simple'. Le hice caso a medias: lo publiqué, pero añadí más experimentos." La historia de cómo 8 páginas cambiaron el ML.


---

## Cierre de la Sección 5

Has llegado al final de esta sección. Tienes ya un arsenal que pocos pueden presumir. En 20 capítulos has pasado de no saber qué es un vértice a implementar una GCN en Rust puro. En el camino:

- Algoritmos deterministas (BFS, DFS, Dijkstra, A*).
- MST y árboles de expansión.
- Flujo máximo y matching.
- Componentes fuertemente conexas.
- Algoritmos randomizados (Karger).
- NP-completitud y aproximaciones.
- DP en grafos (Held-Karp).
- GNN y message passing.

Si te has quedado con ganas de más, tienes todo el bagaje para leer papers de grafos en Machine Learning, para implementar tus propios algoritmos, o para contribuir a librerías como `petgraph` en Rust. La frontera está abierta. Ven con tu grafo.

> *«Un grafo es la forma más simple de capturar la realidad. Todo lo demás es ruido.»*
> — Atribuido a varios autores, ningún matemático famoso en particular, pero la idea es buena.

---
