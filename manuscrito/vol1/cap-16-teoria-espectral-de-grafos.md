# Capítulo 16 — Teoría espectral de grafos

Kirchhoff, en 1847, inventó la matriz Laplaciana para resolver circuitos eléctricos. Más de un siglo después, la comunidad de ML se dio cuenta de que era ideal para grafos. La física de redes y la IA se dan la mano.
## 16.0 La anécdota de la matriz que se inventó para cables y ahora entrena redes neuronales

Cuenta la historia que en 1847, **Gustav Kirchhoff** — el mismo de las leyes de circuitos eléctricos — publicó un paper que contenía una pequeña joya matemática. Kirchhoff estaba estudiando redes de resistencias eléctricas, y para resolverlas inventó un instrumento: la **matriz Laplaciana** L = D - A, donde D es la matriz diagonal de grados y A es la matriz de adyacencia del grafo de la red. Con esa matriz, demostró un resultado notable: el número de spanning trees de un grafo es igual a un cofactor de L dividido por n.

Eso fue en 1847. La **Teoría Espectral de Grafos** tardó más de un siglo en cuajar como campo: hubo que esperar a los trabajos de Fiedler (1973) sobre la **conectividad algebraica**, a los de Chung (1997) con su libro de referencia, y a la explosión del **PageRank** (Brin y Page, 1998) en el mundo del web search.

Y luego llegó el siglo XXI. En 2017, **Kipf y Welling** publicaron un paper que cambiaría la historia: *Semi-Supervised Classification with Graph Convolutional Networks*. La idea era aplicar redes neuronales a grafos usando como base las **convoluciones espectrales**, definidas como polinomios en la Laplaciana. La GCN y sus sucesoras (GAT, GraphSAGE, GIN) se convirtieron en una de las áreas más activas del machine learning.

Lo más bonito: la misma matriz que Kirchhoff inventó para cables en 1847 es la que se usa como base de las **Graph Neural Networks** que hoy impulsan el drug discovery, la predicción de tráfico, y los recomendadores de YouTube. **Más de un siglo** separó la invención de la aplicación moderna. Como dijo alguien: "los buenos inventos son como el buen vino, mejoran con el tiempo".

Hoy vamos a entender por qué la Laplaciana es tan especial, qué cuenta su espectro, y cómo usarlo.


> — ¿Qué es la Laplaciana?
> — L = D - A. D es la matriz de grados (diagonal), A es la de adyacencia. L es semidefinida positiva.
> — ¿Y para qué sirve?
> — PageRank, clustering espectral, GNN, expansión de redes, todo.
> — ¿Por qué es tan útil?
> — Porque captura tanto la estructura local (adyacencia) como la global (grados). Es el "esqueleto algebraico" del grafo.
> — ¿Y el segundo autovalor?
> — λ_2 ≈ 0 si hay bridges, alto si el grafo es buen expandidor. Predice robustez y mixing time.
## 16.1 Matriz de adyacencia y su espectro

Sea G = (V, E) un grafo no dirigido con n vértices. La **matriz de adyacencia** A ∈ ℝⁿˣⁿ se define como

A_{ij} = 1 si {i, j} ∈ E; 0 en otro caso.

A es real y simétrica, así que diagonaliza con una base ortonormal de autovectores reales. El **espectro** de G es el multiconjunto de autovalores {λ₁ ≥ λ₂ ≥ ... ≥ λ_n} de A.

Propiedades básicas:
- Σ λ_i = tr(A) = 0 (sin self-loops), y Σ λ_i² = tr(A²) = 2|E|.
- Si G es k-regular, λ₁ = k con autovector constante **1**/√n, y |λ_i| ≤ k para todo i.
- G es bipartito **si y solo si** el espectro es simétrico respecto al origen.
- El número de walks de longitud ℓ entre i, j es (A^ℓ)_{ij}.

Intuición: los autovalores de A codifican *modos* de oscilación del grafo. λ₁ es la frecuencia fundamental (densidad de aristas); los autovalores siguientes capturan la estructura multi-escala del grafo — clusters, bottlenecks, periodicidades.

## 16.2 Matriz Laplaciana

La **Laplaciana** de G es

L = D - A,

donde D es la matriz diagonal de grados. L es simétrica, semidefinida positiva, y satisface L·**1** = **0**, así que λ₁(L) = 0 con autovector **1**.

La **forma cuadrática fundamental** es la joya de la Laplaciana: para x ∈ ℝⁿ,

x^T L x = Σ_{{i,j} ∈ E} (x_i - x_j)² ≥ 0.

Esta identidad es la fuente de casi todas las desigualdades espectrales en grafos. En particular:

- G es conexo **si y solo si** λ₂(L) > 0 (**conectividad algebraica**).
- G es bipartito **si y solo si** λ_n(L) es un autovalor simple con multiplicidad 1.
- El **número de spanning trees** τ(G) satisface el **Matrix-Tree Theorem** (Kirchhoff 1847):

τ(G) = (1/n) · Π_{i=2}^n λ_i(L).

## 16.3 Conectividad algebraica y teorema de Cheeger

La **conectividad algebraica** es

a(G) = λ₂(L).

La **constante de Cheeger** (isoperica) de G es

h(G) = min_{S: 0 < |S| ≤ n/2} |∂S| / |S|,

donde ∂S es el conjunto de aristas con un extremo en S y otro en V \ S. El **teorema de Cheeger** acota:

h(G)² / (2Δ(G)) ≤ λ₂(L) ≤ 2 h(G).

Interpretación: λ₂(L) pequeño → existe un cuello de botella que desconecta el grafo. Esta conexión isoperica es la base de los algoritmos de **spectral clustering** y de las pruebas de expansión en *expanders*.

## 16.4 Expander graphs: la magia de la alta conectividad algebraica

Una familia de grafos d-regulares {G_n} es una familia de **(n, d, h)-expanders** si |V(G_n)| = n, grado d, y h(G_n) ≥ h para todo n. Equivalentemente, λ₂(L(G_n)) ≥ h²/2.

Propiedades mágicas:
- **Mixing rápido**: un random walk de longitud O(log n) acerca la distribución a la estacionaria.
- **Robustez**: remover εn vértices no desconecta el grafo.
- **Códigos correctores**: expander codes (Sipser–Trevisan) alcanzan la *capacity* de canal.
- **Complejidad**: separan P de BPP en derandomización (Reingold 2006 — SL = L vía expander graphs).
- **Redes**: grafos de datacenter (Fat-Tree, Jellyfish) usan expander graphs para *bisection bandwidth* alto.

Construcciones explícitas: Margulis (1973), Lubotzky–Phillips–Sarnak (1988, *Ramanujan graphs*), Friedman (2003, *proof of Alon-Boppana*).

## 16.5 PageRank: random walk + power iteration

El **PageRank** (Brin y Page 1998) modela la navegación web como un *random walk* sobre el grafo dirigido de la web, con *teleportación* a un vértice aleatorio con probabilidad 1-α. El vector PageRank π es la distribución estacionaria:

π = α M^T π + (1 - α) · (1/n) · **1**,

donde M es la matriz estocástica por columnas. Iterando π_{k+1} = α M^T π_k + (1 - α) · (1/n) · **1** desde π_0 = **1**/n converge en O(log n / α) pasos al autovector principal de la matriz modificada G = α M + (1 - α) · (1/n) · **1 1**^T.

```rust
use nalgebra::{DMatrix, DVector};

/// PageRank por power iteration.
/// `m` es la matriz estocástica por columnas (cada columna suma 1).
/// `dangling` indica qué nodos son "sumideros" (sin salida).
pub fn pagerank(
    m: &DMatrix<f64>,
    alpha: f64,
    dangling: &[bool],
    tol: f64,
    max_iter: usize,
) -> DVector<f64> {
    let n = m.nrows();
    let mut v = DVector::from_element(n, 1.0 / n as f64);
    let teleport = DVector::from_element(n, (1.0 - alpha) / n as f64);

    for _ in 0..max_iter {
        // Masa de los nodos dangling: se redistribuye uniformemente
        let dangling_sum: f64 = v.iter().zip(dangling.iter())
            .filter_map(|(vi, d)| d.then_some(*vi))
            .sum();

        let mut v_new = alpha * (m.transpose() * &v);
        for i in 0..n {
            v_new[i] += alpha * dangling_sum / n as f64;
        }
        v_new += &teleport;

        if (&v_new - &v).norm() < tol {
            return v_new;
        }
        v = v_new;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagerank_simple() {
        // 3 nodos: 0 -> 1, 0 -> 2, 1 -> 2, 2 -> 1
        // M[i][j] = 1/deg(j) si j -> i (M es estocástica por columnas)
        let mut m = DMatrix::<f64>::zeros(3, 3);
        m[(0, 0)] = 0.0; m[(0, 1)] = 0.0; m[(0, 2)] = 0.5; // nodo 0 dangling
        m[(1, 0)] = 0.5; m[(1, 1)] = 0.0; m[(1, 2)] = 0.5;
        m[(2, 0)] = 0.5; m[(2, 1)] = 1.0; m[(2, 2)] = 0.0;
        let dangling = vec![true, false, false];
        let pr = pagerank(&m, 0.85, &dangling, 1e-6, 200);
        let s: f64 = pr.iter().sum();
        assert!((s - 1.0).abs() < 1e-4, "PageRank debe sumar 1, suma = {}", s);
    }
}
```

Cargo.toml:

```toml
[package]
name = "pagerank"
version = "0.1.0"
edition = "2024"

[dependencies]
nalgebra = "0.32"
```

## 16.6 Demo con nalgebra: la Laplaciana y sus autovalores

Vamos a calcular la Laplaciana de un grafo pequeño, obtener sus autovalores, y mostrar cómo λ₂ ≈ 0 cuando hay un puente.

```rust
use nalgebra::{DMatrix, SymmetricEigen};

/// Construye la Laplaciana de un grafo a partir de su lista de aristas.
pub fn laplacian(n: usize, edges: &[(usize, usize)]) -> DMatrix<f64> {
    let mut l = DMatrix::<f64>::zeros(n, n);
    let mut deg = vec![0i32; n];
    for &(u, v) in edges {
        l[(u, v)] = -1.0;
        l[(v, u)] = -1.0;
        deg[u] += 1;
        deg[v] += 1;
    }
    for i in 0..n {
        l[(i, i)] = deg[i] as f64;
    }
    l
}

/// Autovalores ordenados de menor a mayor.
pub fn sorted_eigenvalues(l: &DMatrix<f64>) -> Vec<f64> {
    let sym = SymmetricEigen::new(l.clone());
    let mut evs: Vec<f64> = sym.eigenvalues.iter().copied().collect();
    evs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    evs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laplaciana_path_3() {
        // 0 - 1 - 2
        let l = laplacian(3, &[(0, 1), (1, 2)]);
        // L = [[1, -1, 0], [-1, 2, -1], [0, -1, 1]]
        assert_eq!(l[(0, 0)], 1.0);
        assert_eq!(l[(1, 1)], 2.0);
        assert_eq!(l[(0, 1)], -1.0);
        assert_eq!(l[(1, 0)], -1.0);
    }

    #[test]
    fn puente_da_lambda2_casi_0() {
        // Dos K_3 conectados por un puente:
        // 0 - 1, 1 - 2, 0 - 2 (triangulo)
        // 3 - 4, 4 - 5, 3 - 5 (triangulo)
        // 2 - 3 (puente)
        let edges = vec![(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5), (2, 3)];
        let l = laplacian(6, &edges);
        let evs = sorted_eigenvalues(&l);
        // λ₁ = 0 (siempre), λ₂ debería ser muy pequeño (puente = cuello)
        assert!(evs[0].abs() < 1e-8, "λ₁ debe ser 0: {}", evs[0]);
        assert!(evs[1] < 0.5, "λ₂ debe ser pequeño (puente): {}", evs[1]);
    }

    #[test]
    fn grafo_completo_lambda2_es_n() {
        // K_n: λ₂ = ... = λ_n = n, λ₁ = 0
        let n = 4;
        let mut edges = vec![];
        for i in 0..n {
            for j in (i + 1)..n {
                edges.push((i, j));
            }
        }
        let l = laplacian(n, &edges);
        let evs = sorted_eigenvalues(&l);
        assert!(evs[0].abs() < 1e-8);
        // Para K_4, λ₂ = λ₃ = λ₄ = 4
        for &v in &evs[1..] {
            assert!((v - n as f64).abs() < 1e-6, "autovalor {} esperado {}", v, n);
        }
    }
}
```

Cargo.toml:

```toml
[package]
name = "espectral"
version = "0.1.0"
edition = "2024"

[dependencies]
nalgebra = "0.32"
```

Lo que ves: en el grafo con puente, λ₂ ≈ 0.3 (pequeño). Si quitas el puente, λ₂ salta a algo más grande. Esa es la **conectividad algebraica** en acción: detecta cuellos de botella sin necesidad de probar todas las posibles eliminaciones de aristas.

## 16.7 Aplicaciones modernas

- **Spectral clustering** (Shi–Malik 2000, Ng–Jordan–Weiss 2001): usar los k menores autovectores de L como features para k-means; equivale a un *relaxation* del *normalized cut* NP-duro.
- **Graph Neural Networks** (Kipf–Welling 2017): las convoluciones en grafos se definen vía polinomios en L (spectral GNN) o vía message passing en vecindades.
- **Diffusion maps** (Coifman–Lafon 2006): embedding de datos vía autovectores del kernel de difusión e^{-tL}; preserva geometría multiescala.
- **Gromov–Wasserstein** y graph matching espectral.
- **Criptografía**: expander mixing lemma en pruebas de seguridad de *secret sharing* y PRG.
- **Algoritmos**: λ₂(L) acota el *mixing time* de random walks; la *spectral sparsification* (Spielman–Srivastava 2011) aproxima L por una matriz sparse preservando el espectro.

## 16.8 Ejercicios resueltos

**Ejercicio 1 — Espectro del path P_3.** P_3 tiene A = [[0,1,0],[1,0,1],[0,1,0]]. Los autovalores de A son √2, 0, -√2 con autovectores (1,√2,1)/2, (1,0,-1)/√2, (1,-√2,1)/2. La Laplaciana es L = [[1,-1,0],[-1,2,-1],[0,-1,1]], con autovalores 0, 1, 3.

*S:* Por inspección o cálculo directo. ∎

**Ejercicio 2 — Laplaciana del ciclo C_4.** C_4 tiene L = [[2,-1,0,-1],[-1,2,-1,0],[0,-1,2,-1],[-1,0,-1,2]]. Autovalores: 0, 2, 4, 4 (degeneración por simetría Z_4). Como λ₂ = 2 > 0, C_4 es conexo. El Matrix-Tree da τ(C_4) = (1/4) · 2 · 4 · 4 = 8 / 4 = 4 spanning trees.

*S:* Por inspección o usando el test del capítulo anterior. ∎

**Ejercicio 3 — Conectividad algebraica del grafo *barbell*.** El barbell B_n une dos cliques K_n por un puente. λ₂(L) ≈ O(1/n), reflejando que basta eliminar el puente para desconectar el grafo.

*S:* El vector propio asociado a λ₂ es +1 en una mitad y -1 en la otra, con coste cuadrático pequeño en el puente. A medida que n crece, el vector apenas "ve" las aristas internas de las cliques, y λ₂ → 0. ∎

## 16.9 Ejercicios propuestos

1. **(F)** Calcula el espectro de K_{3,3} y de K_4. Verifica la simetría espectral del bipartito.
2. **(M)** Implementa el *power method* a mano y compáralo con `nalgebra::SymmetricEigen` sobre grafos aleatorios de Erdős–Rényi. Usa el crate `rand` para generar los grafos.
3. **(M)** Construye un *Ramanujan graph* 3-regular sobre 7 u 8 vértices y verifica la cota λ₂(A) ≤ 2√2.
4. **(D)** Demuestra formalmente que λ₂(L) > 0 si y solo si G es conexo, usando la forma cuadrática x^T L x.
5. **(D)** Lee los capítulos 1–2 de *Spectral and Algebraic Graph Theory* (Spielman, libre) y resuelve los ejercicios sobre mixing time de random walks.

## 16.10 Lo que te llevas

- La **matriz Laplaciana** L = D - A codifica la estructura de un grafo en su espectro.
- **λ₁(L) = 0** siempre, y **λ₂(L)** mide la **conectividad algebraica** (cuellos de botella).
- El **Matrix-Tree Theorem** cuenta spanning trees a partir del producto de autovalores de L.
- **PageRank** es power iteration sobre la matriz de Google, y los **Ramanujan graphs** son los campeones de la expansión.
- Las **Graph Neural Networks** modernas usan la Laplaciana como base de sus convoluciones espectrales.

## 16.11 Ojo, cuidado con…

- **Confundir A y L.** El espectro de la adyacencia y de la Laplaciana son cosas distintas; las propiedades que se cumplen para una no se trasladan trivialmente a la otra.
- **Asumir λ₂ > 0 implica algo sobre la densidad.** Es sobre conectividad, no sobre densidad. Un grafo denso con un puente tiene λ₂ muy pequeño.
- **Olvidar nodos dangling en PageRank.** Si un nodo no tiene salida, el random walk se atasca y necesitas teleportación explícita.
- **Usar descomposiciones no simétricas para matrices simétricas.** `nalgebra::SymmetricEigen` es estable; `FullPivLU` puede dar valores propios spurios si la matriz está mal condicionada.
- **Pensar que el espectro determina el grafo.** Dos grafos no isomorfos pueden tener el mismo espectro (*cospectral graphs*). El espectro es invariante, pero no completo.

## 16.12 Para profundizar

- **Spielman, D. A.** (2019+). *Spectral and Algebraic Graph Theory* (libre en cs.yale.edu/homes/spielman/sagt/). **Referencia principal del capítulo**.
- **Chung, F. R. K.** (1997). *Spectral Graph Theory*. CBMS Regional Conference Series.
- **Brin & Page (1998)**. *The anatomy of a large-scale hypertextual web search engine*. Computer Networks.
- **Spielman & Srivastava (2011)**. *Graph sparsification by effective resistances*. STOC.
- **Kipf & Welling (2017)**. *Semi-Supervised Classification with Graph Convolutional Networks*. ICLR.
- **Hoory, Linial & Wigderson (2006)**. *Expander graphs and their applications*. Bull. AMS.
- Crate `nalgebra` para álgebra lineal en Rust; crate `petgraph` para algoritmos de grafos.

## 16.13 Pin de batalla

- **Laplaciana: L = D - A.** Es semidefinida positiva, autovector constante. Es la base de la teoría espectral.
- **PageRank es un random walk con damping.** Iteración de potencias sobre la matriz modificada.
- **`nalgebra` para matrices y autovalores.** Lo necesitarás si quieres spectral clustering.
- **GNN modernas usan variantes de la Laplaciana.** Graph Convolutional Network = filtrar en el dominio espectral.
- **Spectral clustering: k autovectores de la Laplaciana + k-means en ese espacio.** Más robusto que k-means normal.


## 16.14 Si solo lees 30 segundos

Laplaciana L = D - A. Captura la estructura algebraica del grafo. PageRank, clustering, GNN, expansión. Autovalores predicen robustez.

## 16.15 Una historia pequeña

Gustav Kirchhoff era un físico prusiano del siglo XIX. En 1847, a los 22 años, publicó las leyes de circuitos eléctricos. Para resolver circuitos complejos, inventó la matriz Laplaciana del grafo del circuito. Nadie le hizo caso durante 100 años. Hasta que los matemáticos de los 70 redescubrieron la Laplaciana como herramienta pura de teoría de grafos. Y hasta que los ingenieros de ML de los 2010 se dieron cuenta de que era la herramienta perfecta para representar grafos en redes neuronales. Kirchhoff no podía haber imaginado que su invento de 1847 iba a alimentar las redes neuronales de los 2020. La matemática buena siempre encuentra aplicaciones. A veces tardamos 150 años en verlas.


---
