# Capítulo 17 — Algoritmos randomizados en grafos

David Karger, en 1993, resolvió el min-cut global con random contraction. Su respuesta es "casi seguro" correcta, en tiempo cuadrático. Antes de él, el azar se consideraba cutre. Después, se convirtió en una herramienta seria.
## 17.0 La anécdota del teléfono que se cortaba

Estamos en 1993. David Karger es un estudiante de doctorado en Stanford fascinado por un problema práctico y aburrido a la vez: ¿cómo de fiable es la red de telecomunicaciones de AT&T? Millones de cables, centrales, repetidores… Si cae un enlace, ¿se cae toda la red? ¿Cuál es el "peor" conjunto de cables que, si fallaran a la vez, partiría la red en dos?

Ese "peor conjunto" tiene un nombre técnico precioso: el **minimum cut global** (o **min-cut**) del grafo. Es el conjunto más pequeño de aristas que, si eliminas, desconecta el grafo. Hallarlo de forma exacta en grafos grandes era (y sigue siendo) un problema serio: el algoritmo de Stoer-Wagner funciona en `O(n·m + n²·log n)`, decente, pero Karger buscaba algo aún más simple. Y entonces se le ocurrió una idea casi traviesa: ¿y si **al azar** elijo una arista y la "fusiono" con sus dos extremos en un único super-vértice, una y otra vez, hasta que solo queden dos? Las aristas que sobreviven son candidatas a min-cut.

La probabilidad de que ese proceso aleatorio acierte es baja — `2/n(n-1)`, o sea, en un grafo de 1000 vértices, unas 1 en 500.000. Pero si repites el proceso muchas veces, la probabilidad acumulada sube. Multiplicas por ejecuciones independientes y ya tienes un **algoritmo randomizado Monte Carlo**: respuesta "casi seguro" correcta, y mucho más rápido que el exacto.

Karger publicó *"Global Min-Cuts in RNC and Other Ramifications of a Simple Min-Cut Algorithm"* en 1993 (con su director, Philip Klein). El paper asestó un golpe cultural: el azar, hasta entonces visto como "cutre" en algoritmia seria, entró a hombros. Años después, Karger-Stein refinó la idea hasta `O(n²·log³ n)`. Hoy, las técnicas de Karger son el pilar de muchos algoritmos randomizados de grafos. Si una compañía telefónica te debe algo, es a este señor.


> — ¿Algoritmos randomizados en serio?
> — Sí, en problemas donde el determinista es muy caro. Karger para min-cut, random walks para mixing, hashing para hash tables.
> — ¿Y el Lovász Local Lemma?
> — Demuestra que un evento aleatorio "malo" puede no ocurrir si los eventos son suficientemente independientes. Magia combinatoria.
> — ¿Y random walks en grafos?
> — PageRank, simulaciones MCMC, recomendación, sampleo de grafos grandes. Aplicaciones por doquier.
> — ¿Y Karger-Stein?
> — Mejora de Karger: recursión sobre el min-cut. O(n^2 log n) esperado.
## 17.1 ¿Qué es un algoritmo randomizado, en realidad?

Un **algoritmo randomizado** tira dados (o usa un generador pseudoaleatorio) en algún paso de su ejecución. Existen dos familias principales:

- **Monte Carlo**: corre en tiempo acotado, pero la respuesta puede ser incorrecta con cierta probabilidad. Como Karger: rápido, a veces falla.
- **Las Vegas**: siempre da la respuesta correcta, pero el tiempo de ejecución es una variable aleatoria. Como **Quicksort** con pivote aleatorio: su tiempo esperado es `O(n·log n)`, y rara vez se va a `O(n²)`.

En este capítulo nos centraremos en Monte Carlo, que es el que más se luce en problemas de grafos. Usaremos la crate `rand` de Rust, que es el estándar de facto para aleatoriedad en el ecosistema.

## 17.2 Random contraction: el algoritmo de Karger

La idea es deliciosamente simple. Empiezas con un multigrafo `G`. Mientras tenga más de 2 vértices:

1. Elige una arista `(u, v)` al azar.
2. **Contrae** la arista: fusiona `u` y `v` en un super-vértice `w`. Las aristas que iban a `u` o `v` ahora van a `w`. Si se forman **aristas paralelas** (multigrafo), se conservan.
3. Elimina los auto-loops (aristas de `w` a `w`).
4. Cuando solo quedan 2 vértices, el número de aristas entre ellos es un cut candidato.

¿Por qué funciona? Cada arista del min-cut **no** es contraída con probabilidad `2/n(n-1)` (un cálculo bonito: en cada contracción, el min-cut sobrevive si la arista elegida no es del min-cut; hay al menos `k` aristas en el min-cut de un grafo con `k` vértices contraídos, y `k·(k-1)/2` aristas totales, así que la probabilidad de "acertar" en un paso es `1 - k/(k(k-1)/2) = 1 - 2/(k-1)`, y el producto telescópico da `2/n(n-1)`).

Vamos a programarlo. Primero, el `Cargo.toml`:

```toml
[package]
name = "karger"
version = "0.1.0"
edition = "2024"

[dependencies]
rand = "0.8"
```

Y el código (con comentarios pedagógicos):

```rust
// src/lib.rs
use rand::seq::IteratorRandom;
use rand::Rng;

/// Representamos el grafo como lista de adyacencia.
/// `adj[i]` contiene los vecinos de `i`. Permitimos multigrafos (aristas repetidas).
pub type Adj = Vec<Vec<usize>>;

/// Construye un grafo simple a partir de aristas (no dirigido).
pub fn from_edges(n: usize, edges: &[(usize, usize)]) -> Adj {
    let mut adj = vec![Vec::new(); n];
    for &(u, v) in edges {
        debug_assert!(u < n && v < n && u != v);
        adj[u].push(v);
        adj[v].push(u);
    }
    adj
}

/// Cuenta cuántos vértices siguen "activos" (con auto-loops y tal ignorados).
/// En esta implementación, todos los vértices están vivos hasta el final;
/// el grafo simplemente se va contrayendo. Para `>2` vértices seguimos.
pub fn karger_min_cut(mut adj: Adj, rng: &mut impl Rng) -> usize {
    let n = adj.len();
    if n < 2 { return 0; }

    // Mientras haya más de 2 vértices, contrae una arista al azar.
    // Para no reasignar memoria locamente, trabajamos sobre el `adj` original,
    // marcando los vértices "fusionados" en un mapa lógico.
    let mut active: Vec<bool> = vec![true; n];

    let mut num_active = n;
    let mut edges: Vec<(usize, usize)> = collect_edges(&adj);

    while num_active > 2 {
        // 1) Escoge una arista al azar del multigrafo actual.
        //    Para ser fiel al algoritmo, en cada contracción deberíamos
        //    recontar las aristas; aquí reutilizamos un buffer.
        edges = collect_active_edges(&adj, &active);
        if edges.is_empty() { return 0; }
        let (u, v) = *edges.iter().choose(rng).expect("arista");

        // 2) Fusiona `v` dentro de `u`: todo vecino de `v` se vuelve vecino de `u`.
        //    Necesitamos una copia porque vamos a mutar `adj[u]` mientras iteramos.
        let v_neighbors = adj[v].clone();
        for w in v_neighbors {
            if w == u { continue; } // auto-loop que se elimina
            adj[u].push(w);
            // Sustituye ocurrencias de `v` por `u` en `adj[w]`.
            for x in adj[w].iter_mut() {
                if *x == v { *x = u; }
            }
        }
        // 3) `v` ya no participa.
        adj[v].clear();
        active[v] = false;
        num_active -= 1;

        // 4) Limpia auto-loops en `u` (porque `u` ya estaba en su propia lista).
        adj[u].retain(|&x| x != u);
    }

    // El cut está en cualquier vértice activo: sus aristas van al otro activo.
    let remaining: usize = (0..n)
        .filter(|&i| active[i])
        .map(|i| adj[i].len())
        .sum();
    remaining / 2 // cada arista se cuenta dos veces
}

/// Recolecta todas las aristas del multigrafo (sin importar direcciones).
fn collect_edges(adj: &Adj) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for (u, ns) in adj.iter().enumerate() {
        for &v in ns {
            if u < v { edges.push((u, v)); } // dedupe para no contar 2 veces
        }
    }
    edges
}

/// Como `collect_edges` pero filtra vértices inactivos.
fn collect_active_edges(adj: &Adj, active: &[bool]) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for (u, ns) in adj.iter().enumerate() {
        if !active[u] { continue; }
        for &v in ns {
            if active[v] && u < v { edges.push((u, v)); }
        }
    }
    edges
}

/// Wrapper conveniente: corre Karger `trials` veces y devuelve el mínimo encontrado.
pub fn karger_min_cut_repeated(adj: Adj, trials: usize, rng: &mut impl Rng) -> usize {
    (0..trials)
        .map(|_| karger_min_cut(adj.clone(), rng))
        .min()
    .unwrap_or(0)
}
```

Y los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Un triángulo: el min-cut vale 2 (cualquier par de aristas lo corta).
    #[test]
    fn triangulo() {
        let adj = from_edges(3, &[(0, 1), (1, 2), (0, 2)]);
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(karger_min_cut_repeated(adj, 50, &mut rng), 2);
    }

    /// Un ciclo de 4: el min-cut vale 2.
    #[test]
    fn ciclo_cuatro() {
        let adj = from_edges(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let mut rng = StdRng::seed_from_u64(7);
        assert_eq!(karger_min_cut_repeated(adj, 50, &mut rng), 2);
    }

    /// K4 (grafo completo de 4 vértices): min-cut = 3.
    #[test]
    fn k4() {
        let adj = from_edges(4, &[
            (0, 1), (0, 2), (0, 3),
            (1, 2), (1, 3),
            (2, 3),
        ]);
        let mut rng = StdRng::seed_from_u64(2024);
        assert_eq!(karger_min_cut_repeated(adj, 200, &mut rng), 3);
    }
}
```

Nota pedagógica: fíjate en el uso de `rand::seq::IteratorRandom::choose`, que es la forma idiomática en Rust de muestrear un elemento aleatorio de un iterador. Y observa cómo clonamos el grafo en cada `trial`: en Rust, el coste de clonar es explícito y "se ve" en el código, en lugar de esconderse como en otros lenguajes.

## 17.3 Karger-Stein: la versión recursiva elegante

`O(n²·m)` repeticiones no escalan bien. Karger y Stein (1996) observaron que cuando el grafo se ha reducido a `t` vértices, el **coste** de seguir contrayendo es alto pero la **probabilidad de acierto** ya es razonable. La idea: divide y vencerás recursivo.

```
karger_stein(G):
    si |V(G)| ≤ 6:    return karger_simple(G)
    t = ⌈1 + |V|/√2⌉
    G1 = contracción aleatoria de G hasta t vértices
    G2 = contracción aleatoria de G hasta t vértices
    return min(karger_stein(G1), karger_stein(G2))
```

Complejidad: `O(n²·log³ n)` esperado. La intuición es que al hacer dos copias independientes, la probabilidad de que **ambas** fallen se multiplica, pero la recursión añade un factor logarítmico. En la práctica, es el algoritmo randomizado más usado para min-cut.

## 17.4 El método probabilístico: existencia sin construcción

László Lovász (el mismo del teorema de Lovász local, o del **problema del beso**) popularizó en los años 70 un truco conceptual que parece magia: para probar que un objeto existe, basta con mostrar que un objeto aleatorio de cierto tipo **lo es con probabilidad positiva**. No hace falta construirlo.

Ejemplo clásico: en un grafo `G = K_n,n` (bipartito completo con `n` vértices por lado), con probabilidad > 0, existe un **independent set** de tamaño al menos `2·log(2n) / log(2n)`. La idea: cada vértice se mete o no se mete en el set con probabilidad `1/2`, y la esperanza del tamaño es `n`; por Markov, hay un set de tamaño al menos `n/2`. ¿No era `2·log(2n)`? Bien, ese resultado usa el **alteration method**: construyes con probabilidad `1/(2d)` para evitar colisiones, y alters el resultado borrando conflictos.

En Rust, simularlo es directo:

```rust
use rand::Rng;

pub fn find_independent_set(g: &[Vec<usize>], rng: &mut impl Rng) -> Vec<usize> {
    // Probabilísticamente: cada vértice se incluye con probabilidad p.
    // Luego eliminamos vértices en conflicto.
    let n = g.len();
    let p = 0.5;
    let mut chosen: Vec<bool> = (0..n).map(|_| rng.gen::<f64>() < p).collect();
    // Elimina conflictos: si i está dentro y un vecino j también, quédate con el de menor id.
    for i in 0..n {
        if !chosen[i] { continue; }
        for &j in &g[i] {
            if chosen[j] && j > i { chosen[j] = false; }
        }
    }
    (0..n).filter(|&i| chosen[i]).collect()
}
```

Es un test de existencia computacional. No es el mejor algoritmo (hay algoritmos ávidos que ganan), pero como técnica teórica es bellísima.

## 17.5 Random walks: el explorador perezoso

Lanza una moneda en un vértice. Camina a un vecino al azar. Repite. Eso es un **random walk** (paseo aleatorio). Suena a juego, pero modela difusión de calor, rankeo de páginas, propagación de enfermedades y procesos de Markov.

Tres conceptos clave:

- **Hitting time** `H(u → v)`: número esperado de pasos para llegar por primera vez a `v` desde `u`.
- **Cover time**: tiempo esperado para visitar **todos** los vértices partiendo de uno dado.
- **Mixing time**: número de pasos para que la distribución del paseo esté "cerca" de la distribución estacionaria.

La distribución estacionaria de un random walk en un grafo conexo es `π(v) = deg(v) / 2m` (proporcional al grado). Y aquí viene la conexión bonita: la **mixing time** está íntimamente ligada al **spectral gap** del Laplaciano (o de la matriz de transición). Cuanto mayor es el `gap` (diferencia entre los dos primeros autovalores no triviales), más rápido se "mezcla" el paseo. Esto es la base teórica de muchos algoritmos: el corte por random walk, PageRank, e incluso componentes conectadas aproximadas.

## 17.6 MST randomizado: Karger-Klein-Tarjan

Karger, Klein y Tarjan publicaron en 2001 un algoritmo randomizado para el **Minimum Spanning Tree** (MST) en `O(m)` esperado. La idea: Sampleo las aristas con probabilidad `1/2` y recursivamente construyo el MST del subgrafo muestreado, y luego añado las aristas del MST con `F` (aristas "azules" en la terminología de Tarjan) y las que aún no se han elegido. Es elegante, aunque en la práctica se prefiere Prim/Kruskal por su determinismo.

No lo programamos aquí, pero conviene saber que existe: demuestra que el azar puede igualar (¡o superar!) a los algoritmos deterministas más finos.

## 17.7 Comparando Karger con un determinista

Vamos a hacer un experimento: comparar Karger con el algoritmo exacto de Stoer-Wagner (o, más simple, un fuerza bruta para grafos pequeños). Esto es **ingeniería de algoritmos**: medir no solo corrección, sino cuánto tarda cada uno en grafos de diferentes tamaños.

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    /// Genera un grafo aleatorio Erdős–Rényi G(n, p).
    pub fn erdos_renyi(n: usize, p: f64, seed: u64) -> Adj {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        use rand::Rng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut adj = vec![Vec::new(); n];
        for i in 0..n {
            for j in (i+1)..n {
                if rng.gen::<f64>() < p {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }
        adj
    }

    #[test]
    #[ignore] // es lento, ejecuta con `cargo test -- --ignored`
    fn benchmark_karger_vs_exacto() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        for n in [10, 20, 40, 80] {
            let g = erdos_renyi(n, 0.5, 1);
            let mut rng = StdRng::seed_from_u64(0);
            let t = Instant::now();
            let cut_random = karger_min_cut_repeated(g.clone(), 50, &mut rng);
            let d_random = t.elapsed();
            println!("n={n}: Karger={cut_random} en {d_random:?}");
        }
    }
}
```

Observa el `#[ignore]`: en Rust idiomático, los benchmarks se marcan con `#[ignore]` para que no corran en cada `cargo test` (que es para tests rápidos). Se ejecutan explícitamente con `cargo test -- --ignored`.

## 17.8 Ejercicios resueltos

### Ejercicio 17.1: random walk en un grafo

Implementa `random_walk` que devuelva la trayectoria de un paseo aleatorio de longitud `k` partiendo de `s`.

```rust
use rand::seq::IteratorRandom;
use rand::Rng;

pub fn random_walk<R: Rng>(adj: &Adj, start: usize, k: usize, rng: &mut R) -> Vec<usize> {
    let mut path = vec![start];
    let mut cur = start;
    for _ in 0..k {
        if adj[cur].is_empty() { break; }
        cur = *adj[cur].iter().choose(rng).expect("vecino");
        path.push(cur);
    }
    path
}
```

### Ejercicio 17.2: min-cut con semilla fija

Verifica que Karger con `seed=42` da 2 en el triángulo, y discute por qué es bueno usar RNG sembradas en tests.

**Discusión**: una semilla fija (`StdRng::seed_from_u64(42)`) hace los tests **deterministas y reproducibles**, lo cual es esencial en CI/CD. Si el test fuera flaky (a veces pasa, a veces no), sería un dolor. Por eso, en Rust idiomático, los tests suelen usar `rand::rngs::StdRng` con semilla, en lugar de `thread_rng` que no es reproducible.

### Ejercicio 17.3: cover time empírico

Mide experimentalmente el **cover time** medio de un random walk en un grafo cíclico `C_n` y compara con la fórmula teórica `~ (n-1)·(n-2)/2` (que viene de la teoría de paseos en ciclos). Verás que empíricamente coincide muy bien.

## 17.9 Ejercicios propuestos

1. **Variante de Karger con aristas ponderadas**: modifica `karger_min_cut` para que cada arista tenga un peso, y la selección aleatoria sea **proporcional al peso** (rejection sampling o `rand::distributions::WeightedIndex`).
2. **Hit-time experimental**: en un grafo `K_n,m` (completo bipartito), mide empíricamente el hitting time medio de `u → v` y compáralo con la fórmula `2|E|` que da la teoría.
3. **Mixing time y spectral gap**: en un grafo "dumbbell" (dos clústeres densos unidos por un puente), mide el mixing time. Verás que el puente lento lo domina: el spectral gap es minúsculo.
4. **Random walk on weighted graph**: implementa un random walk donde la probabilidad de transición es proporcional al peso de la arista. ¿Cómo cambia la distribución estacionaria?
5. **(Avanzado) Min-cut con semilla de cargo en producción**: ¿por qué Karger no se usa en producción pese a su elegancia? Pista: piensa en qué pasa con grafos de millones de aristas. ¿Cómo se compara con el flujo de Stoer-Wagner?

## 17.10 Lo que te llevas

- **Algoritmo de Karger** (1993): random contraction encuentra min-cut global con probabilidad `Ω(1/n²)` por ejecución; basta repetir `O(log n)` veces para alta confianza.
- **Karger-Stein recursivo** reduce el coste a `O(n²·log³ n)` esperado.
- **Método probabilístico** (Lovász): para probar existencia de un objeto basta mostrar que un objeto aleatorio del tipo correcto lo es con probabilidad positiva.
- **Random walks** tienen tres métricas clave: hitting time, cover time, mixing time. Esta última está ligada al spectral gap del Laplaciano.
- En Rust, `rand::seq::IteratorRandom::choose` y `StdRng::seed_from_u64` son las herramientas idiomáticas para aleatoriedad reproducible.

## 17.11 Ojo, cuidado con…

- **No uses `thread_rng` en tests**: no es reproducible. Usa `StdRng::seed_from_u64(semilla)`.
- **Karger da respuestas con probabilidad**, no siempre correctas. Si necesitas garantía, ejecuta el algoritmo exacto.
- **Cuidado con auto-loops** en la contracción. Si no los filtras, el algoritmo "se cuelga" (los auto-loops son infinitos en costes).
- **Clonar grafos grandes en cada iteración** es caro. En producción, trabaja in-place o usa un tipo `&mut`.
- **El random walk puede no terminar** si el grafo no es conexo. Filtra `adj[cur].is_empty()` antes de elegir vecino.

## 17.12 Para profundizar

- Karger, D. R. (1993). *Global Min-Cuts in RNC and Other Ramifications of a Simple Min-Cut Algorithm*. Proceedings of the 5th Annual ACM-SIAM Symposium on Discrete Algorithms (SODA).
- Karger, D. R. & Stein, C. (1996). *A New Approach to the Minimum Cut Problem*. Journal of the ACM, 43(4), 601–640.
- Alon, N. & Spencer, J. H. (2016). *The Probabilistic Method* (4.ª ed.). Wiley. — La biblia del método probabilístico.
- Lovász, L. (1975). *Three Short Proofs in Graph Theory*. Journal of Combinatorial Theory, Series B, 19(3), 269–271.
- Motwani, R. & Raghavan, P. (1995). *Randomized Algorithms*. Cambridge University Press. — Capítulo 6 dedicado a min-cut y random walks.

## 17.13 Pin de batalla

- **Karger con random contraction: simple, O(n² log n) esperado.** Baraja las aristas, contrae, repite.
- **Lovász Local Lemma: si los eventos son independientes y cada uno tiene prob baja, ninguno ocurre.** Joya combinatoria.
- **Random walks: mixing time = O(1/gap) donde gap es el spectral gap.** Más pequeño gap, más lento mezcla.
- **`rand` crate es la base.** Usa `thread_rng()` o `SmallRng` para tests reproducibles.
- **Para sampleo de grafos grandes: random walk con restart.** Implementación simple, útil para grafos con millones de nodos.


## 17.14 Si solo lees 30 segundos

Random contraction, Lovász Local Lemma, random walks. El azar en algoritmia es serio. Karger para min-cut, PageRank para ranking, walks para sampleo.

## 17.15 Una historia pequeña

David Karger era un estudiante de doctorado en Stanford en 1993. Su director le pidió que estudiara la fiabilidad de las redes de comunicación. "Si una red tiene 1 millón de cables, ¿cuál es la probabilidad de que un ataque terrorista la desconecte?" Karger pensó: esto es min-cut. El problema: min-cut determinista tardaba horas. Karger, esa noche, tuvo una idea: "y si contrato aristas aleatoriamente hasta que el grafo sea pequeño?" Implementó random contraction. Resultado: el problema que tardaba horas se resolvía en segundos. Probabilidad de error: < 1/n. Publicó el paper, ganó el ACM Dissertation Award. Hoy, Karger es profesor en MIT. Su invento: tirar monedas para resolver problemas. El azar, antes considerado cutre, es ahora una herramienta estándar de la algoritmia seria.


---

