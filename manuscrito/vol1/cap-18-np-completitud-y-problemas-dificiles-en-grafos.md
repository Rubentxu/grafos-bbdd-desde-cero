# Capítulo 18 — NP-completitud y problemas difíciles en grafos

Stephen Cook demostró en 1971 que SAT es NP-completo en un paper de 6 páginas. La conferencia no se lo creyó. Tardó 5 años en publicar. Leonid Levin lo demostró de forma independiente en la URSS casi al mismo tiempo. Hoy, "P vs NP" sigue abierto, con 1 millón de dólares esperando.
## 18.0 La anécdota del teorema que nadie creyó

Abril de 1971. Conferencia de la ACM en Shaker Heights, Ohio. Un joven assistant professor llamado **Stephen Cook** presenta un teorema portentoso: todo problema cuya respuesta puede **verificarse** en tiempo polinomial puede **reducirse** a otro problema particular, el de la satisfacibilidad booleana (SAT). Es decir, SAT es "el más difícil" de los problemas verificables. Si SAT se resolviera en tiempo polinomial, todos los problemas verificables lo harían.

El teorema era profundo. La audiencia, escéptica. Cook mismo admitiría después que el resultado era demasiado abstracto para su época. Pasaron **cinco años** hasta que el paper apareció publicado, en 1976, en *Transactions of the American Mathematical Society*. Para entonces, **Leonid Levin**, un joven matemático soviético de 22 años, había demostrado el mismo resultado de forma independiente en 1972 desde la otra mitad de la Cortina de Hierro, con un paper que tardó aún más en salir (publicado en ruso en 1973). El teorema se llama hoy **Cook-Levin**.

Y aquí viene lo gordo: el **problema P vs NP** — ¿es realmente más difícil *verificar* una solución que *encontrarla*? — sigue abierto en 2024. Clay Mathematics Institute ofrece un millón de dólares a quien lo resuelva. Lo más inquietante: en **grafos** nos toca más de cerca que en casi cualquier otra área. Independent Set, Hamiltonian Cycle, Graph Coloring… son todos NP-completos. Si te dedicas a grafos, la sombra de P vs NP te acompañará siempre.


> — Espera, ¿cómo que NP-completo no significa "imposible"?
> — No. NP-completo significa "no sabemos resolverlo en tiempo polinómico, pero si alguien lo hace, todos los problemas de NP caen."
> — ¿Y P = NP?
> — Problema abierto. Si P = NP, RSA se rompe. Si P ≠ NP, ciertas cosas son inherentemente difíciles.
> — ¿Cómo lidio con problemas NP-completos en práctica?
> — Aproximaciones, heurísticas, casos especiales, parameterized complexity. Nunca el algoritmo exacto (salvo para instancias pequeñas).
> — ¿Cuál es el más famoso?
> — TSP: viajar por N ciudades minimizando distancia. NP-hard, pero hay 2-aprox para MST.
## 18.1 Las clases P y NP, explicadas con monedas de céntimo

Vamos a definir las clases con un ejemplo cotidiano.

Estás en un parque, ves un conjunto de monedas de céntimo tiradas en el suelo. Te pregunto: **¿hay un subconjunto de monedas que sume exactamente 137 céntimos?**

- **Clase P**: problemas para los que existe un algoritmo **polinomial** que **encuentra** la respuesta. Por ejemplo, "¿hay un camino de A a B?" se resuelve con BFS en `O(n+m)`. Eso es P.
- **Clase NP**: problemas para los que, si alguien te **da** una respuesta candidata, puedes **verificarla** en tiempo polinomial. El problema de las monedas es NP: si tu amigo te dice "toma estas 7 monedas suman 137", tú puedes sumarlas en `O(7)` y verificar. Pero **encontrar** esas 7 monedas puede costar exponencialmente (probar todos los subconjuntos es `O(2^n)`).

La pregunta del millón: **¿P = NP?** Si alguien te da la respuesta, ¿puedes encontrarla tan rápido como verificarla? La mayoría cree que no, pero nadie lo ha demostrado.

### Definiciones formales (sin dolor)

- **P**: problemas decidibles por una máquina de Turing determinista en tiempo `O(n^k)` para alguna constante `k`.
- **NP**: problemas decidibles por una máquina de Turing **no determinista** en tiempo `O(n^k)`. Equivalentemente: problemas cuyas soluciones se verifican en tiempo polinomial.
- **NP-hard**: problemas que son "al menos tan difíciles" como cualquier problema en NP. Formalmente, un problema `H` es NP-hard si para todo problema `L ∈ NP` existe una **reducción polinomial** `L ≤_p H`.
- **NP-completo**: problemas que están en NP **y** son NP-hard. Son "los más difíciles" de NP.

Un detalle que confunde: **NP no significa "no polinomial"**. Significa "no determinista polinomial". Es desafortunado, pero así es.

## 18.2 Reducciones polinomiales: el arte de transformar problemas

Una **reducción polinomial** de un problema `A` a un problema `B` es una transformación `f` tal que:
- `f` se computa en tiempo polinomial.
- Una entrada `x` es instancia "sí" de `A` si y solo si `f(x)` es instancia "sí" de `B`.

Es decir, **si supieras resolver `B`, podrías resolver `A`**. Las reducciones son la moneda de cambio de la NP-completitud: para probar que un problema es NP-completo, basta con reducirle un problema ya conocido como NP-completo.

### Reducciones canónicas en grafos

- **Hamiltonian Cycle ≤_p TSP**: dado un grafo `G` con `n` vértices, construye una instancia TSP con `n` ciudades. La distancia entre `i` y `j` es `1` si `(i,j) ∈ E(G)`, y `2` en caso contrario. Entonces `G` tiene un ciclo hamiltoniano si y solo si el TSP tiene un tour de longitud `n`.
- **Independent Set ≤_p Clique**: ¡trivial! Un conjunto es independiente en `G` si y solo si es un **clique** en el **complemento** de `G`. Esto es importantísimo:Independent Set y Clique son "el mismo problema con el grafo dado la vuelta".
- **3-SAT ≤_p 3-Color**: la reducción clásica de Garey, Johnson y Stockmeyer (1974). De cada cláusula `C = (ℓ₁ ∨ ℓ₂ ∨ ℓ₃)` se construye un pequeño **gadget** que fuerza a usar 3 colores de forma que codifique la satisfacibilidad.
- **Vertex Cover ≤_p Independent Set (por complemento)**: en cualquier grafo, un conjunto `S` es vertex cover si y solo si `V \ S` es independent set. Esta es la más bonita y la más fácil de recordar: **VC(G) + IS(G) = n**, donde `|VC| = n - |IS|`.

## 18.3 Los 6 problemas canónicos NP-completos en grafos

Memoriza esta lista. Son los "Big Six" de NP-completitud en grafos:

| # | Problema | Entrada | Pregunta |
|---|----------|---------|----------|
| 1 | **Independent Set** (IS) | Grafo `G`, entero `k` | ¿Hay `k` vértices sin aristas entre sí? |
| 2 | **Clique** | Grafo `G`, entero `k` | ¿Hay `k` vértices con todas las aristas entre sí? |
| 3 | **Vertex Cover** (VC) | Grafo `G`, entero `k` | ¿Hay `k` vértices que toquen todas las aristas? |
| 4 | **Hamiltonian Cycle** | Grafo `G` | ¿Hay un ciclo que visite cada vértice exactamente una vez? |
| 5 | **Travelling Salesman** (TSP) | Grafo con pesos, entero `k` | ¿Hay un tour de longitud ≤ `k`? |
| 6 | **Graph Coloring** | Grafo `G`, entero `k` | ¿Puedo colorear vértices con `k` colores sin adyacentes iguales? |
| 7 (bonus) | **Subgraph Isomorphism** | Grafos `G`, `H` | ¿Es `H` subgrafo de `G`? |

Todos estos problemas son NP-completos. **Subgraph Isomorphism** es particularmente traicionero: incluso el caso particular de **clique** es NP-completo. Si lo reduces, te sale NP-difícil.

## 18.4 El teorema de Cook-Levin en cinco frases

Cook (1971) demostró que **SAT** (dada una fórmula booleana, ¿hay una asignación que la satisfaga?) es NP-completo. Levin (1972, URSS) lo demostró independientemente. El truco: dada una máquina de Turing no determinista `M` y una entrada `w`, simulas las `n^k` configuraciones de `M` con una fórmula booleana enorme que es satisfacible si y solo si `M` acepta `w`. La fórmula es **enorme** (exponencial en `n`), pero construible en tiempo polinomial.

A partir de Cook-Levin, una cascada de reducciones mostró que cientos de problemas son NP-completos. Karp (1972) publicó una lista de 21 problemas NP-completos, varios de grafos. Desde entonces, miles más.

## 18.5 El lado amable: aproximaciones y heurísticas

"Si no puedo resolverlo exacto, ¿puedo resolverlo **casi** exacto?" La **aproximación** es un campo enorme. Veamos tres ejemplos estrella.

### 18.5.1 Vertex Cover 2-aproximado

Algoritmo: toma cualquier matching maximal `M` del grafo. Devuelve los `2|M|` extremos de las aristas del matching. Esto es un vertex cover (las aristas de un matching maximal requieren sus dos extremos). Y es 2-aprox: como `M` es maximal, ningún vértice queda "descubierto", así que cubrimos todo; y `|OPT| ≥ |M|`, así que `2|M| ≤ 2|OPT|`.

```rust
pub fn vc_2approx(matching: &[(usize, usize)]) -> Vec<usize> {
    matching.iter().flat_map(|&(u, v)| [u, v]).collect()
}
```

### 18.5.2 TSP métrico: MST-TSP con factor 2

Algoritmo (para TSP métrico, donde las distancias cumplen la desigualdad triangular):
1. Calcula un MST `T` del grafo.
2. Haz un **DFS** del árbol, listando los vértices en orden de visita.
3. Devuelve ese orden como tour. Salta vértices repetidos (la desigualdad triangular te dice que "saltar" no empeora el tour).

Coste: 2-aproximado. Si usas **Christofides** (1976) con un matching mínimo en los vértices de grado impar, obtienes un 1.5-aproximado, que sigue siendo el récord.

### 18.5.3 Independent Set: intratabilidad de aproximación

Para IS, hay una mala noticia: a menos que `P = NP`, **no existe** un algoritmo de aproximación de factor `n^(1-ε)` para ningún `ε > 0` (Håstad 1999). Es decir, no puedes hacer nada mejor que la fuerza bruta `O(2^n)` salvo trucos combinatorios. La intuición: un set independiente es muy frágil; cualquier vértice que metas puede romper muchas relaciones.

## 18.6 Branch & Bound: cuando queremos ser exactos

Para grafos pequeños (digamos, `n ≤ 50`), a veces **podemos** resolver problemas NP-completos de forma exacta con **Branch & Bound** (ramificación y poda):

1. **Branch**: en cada paso, ramifica el problema: por ejemplo, "el vértice 5 está en el independent set" o "el vértice 5 no está". Crea dos subproblemas.
2. **Bound**: calcula una **cota superior** (o inferior, según maximices o minimices) usando relajación lineal, heurística, o un greedy.
3. **Poda**: si la cota del subproblema es peor que la mejor solución encontrada, **descarta** esa rama entera.

```rust
pub struct BnBNode {
    pub chosen: Vec<usize>,
    pub excluded: Vec<usize>,
    pub upper_bound: usize,
}

pub fn branch_and_bound_is(adj: &[Vec<usize>], n: usize, k: usize) -> Option<Vec<usize>> {
    // Búsqueda DFS con poda por cota trivial.
    // upper_bound = n - excluded.size() (cota ingenua, pero suficiente para toy cases).
    fn dfs(adj: &[Vec<usize>], chosen: &[usize], excluded: &[usize], k: usize) -> Option<Vec<usize>> {
        if chosen.len() == k { return Some(chosen.to_vec()); }
        if chosen.len() + (adj.len() - excluded.len()) < k { return None; } // poda
        // elige el primer vértice no decidido
        let next = (0..adj.len()).find(|&v| !chosen.contains(&v) && !excluded.contains(&v))?;
        // rama 1: incluir
        if let Some(sol) = dfs(adj, &[chosen, &[next]].concat(), excluded, k) {
            return Some(sol);
        }
        // rama 2: excluir (y propagar a sus vecinos)
        let mut new_excluded = excluded.to_vec();
        new_excluded.push(next);
        for &nb in &adj[next] {
            if !new_excluded.contains(&nb) { new_excluded.push(nb); }
        }
        dfs(adj, chosen, &new_excluded, k)
    }
    dfs(adj, &[], &[], k)
}
```

Este código es **didáctico**; en producción usarías cotas más finas (LP relaxation, clique cover inferior, etc.).

## 18.7 Held-Karp TSP O(2^n · n²): el DP que ya no cabe

TSP admite un algoritmo exacto de **programación dinámica** con coste `O(2^n · n²)`, gracias a Held y Karp (1962) y Bellman (1962, de forma independiente). La idea: para cada subconjunto `S` de vértices y cada vértice final `v ∈ S`, calculamos el camino más corto que empieza en un origen fijo, pasa por todos los vértices de `S`, y termina en `v`.

Lo programamos en detalle en el Capítulo 19 (DP en grafos), donde tiene más sentido. Aquí solo dejamos la complejidad y la promesa: **Held-Karp** es exponencial en `n`, pero es **el algoritmo más rápido conocido** para TSP exacto. Cualquier mejora por debajo de `O(2^n · poly(n))` sería revolucionaria (e implicaría P = NP, se sospecha).

## 18.8 Ejercicios resueltos

### Ejercicio 18.1: reconocer una reducción

Considera: ¿es Vertex Cover reducible a Independent Set? ¿Cómo?

**Solución**: sí, mediante la identidad `VC(G) = V \ IS(G)`. Dado un grafo `G` y un `k`, la pregunta "¿hay VC de tamaño `k`?" equivale a "¿hay IS de tamaño `n - k`?". Si tuvieras un oráculo para IS, resolverías VC en `O(1)`.

### Ejercicio 18.2: clique a IS

Dado un grafo `G`, construye `G'` (el complemento). Muestra que `S` es IS en `G` si y solo si `S` es clique en `G'`.

**Solución**: `S` es IS en `G` si para todo par `u, v ∈ S`, no hay arista `(u,v)` en `G`. Equivalentemente, para todo par, la arista **no** está en `G`, luego **sí** está en `G'` (el complemento tiene todas las aristas que `G` no tiene). Esto es exactamente la definición de clique en `G'`.

### Ejercicio 18.3: 2-aprox de VC por matching maximal

Implementa el algoritmo 2-aprox. ¿Por qué es 2-aprox?

```rust
pub fn vc_2approx_by_matching(adj: &[Vec<usize>]) -> Vec<usize> {
    let mut covered = vec![false; adj.len()];
    let mut cover = Vec::new();
    for u in 0..adj.len() {
        if covered[u] { continue; }
        for &v in &adj[u] {
            if !covered[v] {
                cover.push(u);
                cover.push(v);
                covered[u] = true;
                covered[v] = true;
                break;
            }
        }
    }
    cover.sort_unstable();
    cover.dedup();
    cover
}
```

**Prueba de 2-aprox**: el matching `M` que construimos es maximal, así que `|M|` es al menos `|OPT|/2` (cada vértice del óptimo cubre a lo sumo una arista del matching). El cover tiene `2|M| ≤ 2|OPT|`.

## 18.9 Ejercicios propuestos

1. **3-SAT a 3-COLOR**: implementa la reducción de Garey-Johnson para una fórmula `C = (x ∨ y ∨ ¬z)`. Dibuja el gadget.
2. **Verificador NP**: implementa un verificador polinomial para Hamiltonian Cycle. La entrada incluye un grafo y una secuencia de vértices; el verificador devuelve sí/no en `O(n)`.
3. **TSP con 4 ciudades**: implementa fuerza bruta para TSP con `n=4` y compara con Held-Karp. Verifica que dan el mismo resultado.
4. **MST-TSP en Rust**: implementa el algoritmo 2-aprox del TSP métrico. Prueba con un grafo cuadrado de 4 ciudades.
5. **(Avanzado) Branch & Bound mejorado**: añade una **cota inferior** al BnB de IS usando el **clique cover number**: cada clique del cover aporta a lo sumo un vértice al IS. ¿Cuánto mejora el tiempo?

## 18.10 Lo que te llevas

- **P, NP, NP-hard, NP-completo** son clases de complejidad. P es resolver, NP es verificar. NP-completo es "lo más difícil de NP".
- **Cook-Levin (1971/1972)**: SAT es NP-completo. De ahí, por reducción, miles de problemas más.
- **6 problemas canónicos** en grafos: IS, Clique, VC, Ham Cycle, TSP, Graph Coloring. Son todos NP-completos.
- **Aproximaciones**: 2-aprox para VC y TSP-métrico (MST-TSP), 1.5-aprox con Christofides. IS no admite buen aprox.
- **Branch & Bound** y **Held-Karp `O(2^n·n²)`** son los caballeros de batalla para problemas pequeños.
- En Rust, los algoritmos de aproximación son especialmente limpios: `matching`, `clique cover`, `LP relax` se prestan a composición con iteradores y folds.

## 18.11 Ojo, cuidado con…

- **NP no es "no polinomial"**. Es "no determinista polinomial". Memorízalo antes de discutir con alguien.
- **"Resuelvo cualquier problema NP"** es una promesa enorme. Si te la crees, revisa: el solver que usas probablemente hace heurísticas, no magia.
- **Cuidado con las reducciones circulares**: la cadena clásica es `3-SAT ≤_p IS ≤_p VC ≤_p...`. Si te encuentras en un loop, probablemente te has equivocado.
- **TSP sin la propiedad métrica** (sin desigualdad triangular) es **mucho** más difícil de aproximar. En ese caso, no hay constante.
- **"P = NP" o "P ≠ NP"**: nadie lo sabe. No hagas como el que dice "yo creo que P = NP porque…" sin pruebas.

## 18.12 Para profundizar

- Cook, S. A. (1971). *The Complexity of Theorem-Proving Procedures*. Proceedings of the 3rd Annual ACM Symposium on Theory of Computing (STOC).
- Karp, R. M. (1972). *Reducibility among Combinatorial Problems*. Complexity of Computer Computations, Plenum Press, 85–103.
- Garey, M. R. & Johnson, D. S. (1979). *Computers and Intractability: A Guide to the Theory of NP-Completeness*. W. H. Freeman. — La biblia.
- Håstad, J. (1999). *Clique is Hard to Approximate within n^(1-ε)*. Acta Mathematica, 182, 105–142.
- Christofides, N. (1976). *Worst-Case Analysis of a New Heuristic for the Travelling Salesman Problem*. Technical Report 388, Carnegie Mellon University.

## 18.13 Pin de batalla

- **Cook-Levin (1971) y Karp (1972) son los padres de NP-completitud.** Sus 21 problemas son el canon.
- **Aproximaciones son tu mejor amigo en producción.** 2-approx para VC y MST-TSP. IN-approx para IS.
- **Branch & bound para instancias pequeñas.** Held-Karp TSP O(2^n · n²) para n < 20.
- **Si reduces A a B y B es NP, A es NP-hard (o NP-completo si A está en NP).** Reducciones bien construidas son el truco.
- **No todo lo "lento" es NP-completo.** A veces O(n^5) es solo O(n^5), no NP-hard.


## 18.14 Si solo lees 30 segundos

NP-completo = problemas que si se resuelven en P, todos los NP caen. P vs NP sigue abierto. En práctica: aproximaciones + heurísticas + casos especiales.

## 18.15 Una historia pequeña

Stephen Cook era un matemático canadiense trabajando en Berkeley en 1971. Demostró que SAT es NP-completo. Presentó su resultado en una conferencia. La audiencia no se lo creyó. El paper tardó 5 años en publicarse en una revista. Mientras tanto, Leonid Levin, en la URSS, demostró lo mismo de forma independiente. Nadie en occidente lo supo hasta la Guerra Fría. Cook y Levin se conocieron en los 80. Se llevan bien. Ambos tienen razón, ambos merecen crédito. Y el problema P vs NP sigue abierto, con 1 millón de dólares del Clay Mathematics Prize esperando. Si alguien lo resuelve, las criptomonedas, la criptografía, la logística, y básicamente la informática tal como la conocemos, cambiarán para siempre. ¿Te animas?


---

