# Capítulo 6 — Topological sort y DAGs

La Marina de EE.UU. necesitaba planificar 300.000 eventos encadenados para lanzar un misil Polaris. Los sistemas de gestión de proyectos de los años 50 no daban para tanto. El topological sort salvó el programa.
## 6.0 La anécdota de Polaris y los 300 000 eventos

En **1958**, la Marina de los Estados Unidos lanzó el **Proyecto Polaris**: construir el primer misil balístico lanzado desde un submarino. Era una pesadilla logística. El programa involucraba a más de **300 000 eventos** encadenados (diseñar una pieza, probar un motor, esperar un informe, contratar a un proveedor, pintar el fuselaje…) y dependía de la cooperación entre cientos de contratistas. La pregunta era: ¿en qué orden atacamos todo esto para terminar lo antes posible?

La Marina contrató a la empresa **Booz Allen Hamilton** y a la división de investigación de **Lockheed**. El equipo necesitaba, literalmente, *dibujar el orden en que debían ejecutarse las tareas* sin meter la pata con dependencias circulares. Aparecieron en escena dos técnicas gemelas: **PERT** (*Program Evaluation and Review Technique*) y **CPM** (*Critical Path Method*). Ambas reducían el problema a un **grafo dirigido acíclico (DAG)**: cada tarea era un nodo, cada dependencia una arista. Calcular el *longest path* en ese DAG daba la duración mínima del proyecto y, de paso, qué tareas eran críticas (atrasarte en una de ellas retrasaba todo el misil).

El orden topológico era el primer paso: decidir **en qué orden ejecutar las tareas sin violar dependencias**. Y aquí viene el *spoiler*: el algoritmo de Kahn, publicado en 1962, nació directamente de la experiencia Polaris. La Marina tenía dos supercomputadoras en ese momento, la teoría de grafos ya tenía décadas, pero la aplicación industrial lo cambió todo. Sin DAGs, Polaris se habría retrasado años. Con ellos, el misil estuvo listo en 1959 y se lanzó al agua en 1960. Un orden topológico bien puesto salvó el programa.


> — Tengo un grafo con 50 tareas. ¿En qué orden las ejecuto?
> — Si no hay ciclos, `petgraph::algo::toposort` te las devuelve en orden topológico.
> — ¿Y si hay ciclos?
> — Te avisa con un error. Los ciclos en DAGs son imposibles por definición; si los hay, tu "DAG" no es DAG.
> — Vale, ¿y para qué sirve esto en la vida real?
> — Para TODO. Compilación, scheduling, dependencias de paquetes, orden de desayuno (café depende de hervir agua, hervir agua depende de encender fuego...).
## 6.1 DAG y orden topológico

Un **DAG** (*Directed Acyclic Graph*) es un grafo dirigido **sin ciclos**. Es la estructura canónica para modelar **dependencias**: tareas, módulos, cursos, archivos, instrucciones.

Un **orden topológico** de un DAG es una permutación de los vértices tal que para toda arista dirigida $(u, v)$, $u$ aparece **antes** que $v$. Equivalentemente, es una *extensión lineal* del orden parcial definido por alcanzabilidad.

> **Teoremas básicos**:
> - Un grafo dirigido admite un orden topológico $\iff$ es un DAG.
> - El orden no es único salvo que el DAG sea un camino: vértices “incomparables” admiten varios órdenes.

**Palabras clave** que vamos a usar:
- **DAG**: grafo dirigido acíclico. La estructura que modela todo lo que tiene dependencias.
- **In-degree** (*grado entrante*): número de aristas que llegan a un vértice.
- **Back-edge**: arista que apunta a un ancestro en el árbol DFS, señal inequívoca de ciclo.
- **Post-orden**: orden en que un DFS “termina” cada vértice; invertirlo da un orden topológico.
- **Longest path en DAG**: DP sobre el orden topológico; en grafos generales es NP-difícil.
- **Ruta crítica** (*critical path*): el longest path; determina la duración mínima de un proyecto.

## 6.2 Algoritmo de Kahn (BFS por in-degree)

Kahn (1962) hace algo simple: emite vértices cuyo **in-degree** es 0 (no les llega nada, no dependen de nadie), y al emitirlos, los “borra” lógicamente (decrementa el in-degree de sus vecinos). Si al final emitimos todos los vértices, era un DAG; si no, hay un ciclo atrapado.

```
1. Calcular in-degree de cada vértice.
2. Encolar todos los vértices con in-degree = 0.
3. Mientras la cola no esté vacía:
     v = desencolar
     emitir v
     para cada (v, w) en E:
         in-degree[w] -= 1
         si in-degree[w] == 0: encolar w
4. Si emitimos |V| vértices, el grafo era DAG; si no, hay ciclo.
```

**Complejidad**: $O(V + E)$ con almacenamiento explícito del in-degree. Lineal. Bonito.

## 6.3 Algoritmo DFS-based (reverse postorder)

```
1. Para cada vértice no visitado, ejecutar un DFS.
2. Cuando “terminamos” un vértice (post-visita), apilarlo.
3. Al final, desapilar para obtener el orden topológico.
```

**Por qué funciona**: cuando DFS termina de visitar $v$, todos sus descendientes ya están apilados *debajo*. Al invertir el orden, los descendientes quedan antes que sus ancestros, satisfaciendo la propiedad topológica.

**Complejidad**: $O(V + E)$.

**Kahn vs DFS**:

| Criterio | Kahn | DFS |
|---|---|---|
| Detección de ciclo | Natural (quedan vértices sin emitir) | Natural (back-edge en recursion stack) |
| Enumerar **todos** los órdenes | Más fácil | Más engorroso |
| Memoria | Cola + array in-degree | Stack de recursión |
| Iterativo sin recursión | Nativo | Necesita pila manual |

## 6.4 Detección de ciclos

Para grafos dirigidos basta con un DFS que mantenga el conjunto de “visitados-en-el-stack” (color gris). Si en una exploración encontramos una arista $(u, v)$ con $v$ aún gris, hay un **back-edge** y, por tanto, un ciclo.

## 6.5 Longest path en DAG (DP sobre el orden)

En grafos generales, el camino más largo es NP-difícil. En DAGs se resuelve elegantemente con **programación dinámica** sobre el orden topológico:

```
dist[v] = max(w(u, v) + dist[u]) sobre aristas entrantes (u, v)
```

- Inicializa `dist[fuente] = 0`, el resto a $-\infty$.
- Procesa los vértices en orden topológico.
- Cada arista puede mejorar `dist[v]`.

Aplicaciones: ruta crítica en PERT, planning con duraciones, cadenas de compilación, *longest chain of dependencies*.

## 6.6 Aplicaciones del mundo real

- **Compilación**: `make`, `cargo build`, Bazel, todos resuelven dependencias con topological sort.
- **Course schedule**: cada curso depende de sus prerrequisitos.
- **PERT/CPM**: el caso Polaris del principio.
- **Planificación de proyectos**: cualquier *task scheduler* (Asana, Notion, Jira por dentro) hace topological sort.
- **Resolución de fórmulas en hojas de cálculo**: Excel y Google Sheets detectan dependencias circulares precisamente con un DFS que busca back-edges.
- **Pipelines de datos** (Apache Airflow, Spark, Prefect): cada *task* es un nodo dirigido.
- **Decodificación de diccionarios alienígenas**: a partir de un diccionario ordenado, inferir el orden del alfabeto (ver Ejercicios).

## 6.7 Implementación en Rust 2024

Empecemos por Kahn, con `VecDeque`:

```rust
// src/toposort.rs
//! Topological sort: Kahn y DFS, con detección de ciclo.

use std::collections::VecDeque;

/// Resultado de un topological sort.
#[derive(Debug, PartialEq, Eq)]
pub enum Topsort {
    /// Orden topológico válido.
    Order(Vec<usize>),
    /// El grafo tiene un ciclo; contiene los vértices atrapados.
    Cycle(Vec<usize>),
}

/// Kahn: BFS por in-degree. Devuelve `Order` si es DAG, `Cycle` si no.
pub fn kahn(n: usize, adj: &[Vec<usize>]) -> Topsort {
    // Calculamos el in-degree de cada vértice.
    let mut in_deg = vec![0usize; n];
    for u in 0..n {
        for &v in &adj[u] {
            in_deg[v] += 1;
        }
    }

    // Encolamos vértices sin dependencias.
    let mut queue: VecDeque<usize> = (0..n).filter(|&u| in_deg[u] == 0).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    if order.len() == n {
        Topsort::Order(order)
    } else {
        Topsort::Cycle((0..n).filter(|&u| in_deg[u] > 0).collect())
    }
}

/// DFS-based: reverse postorder, iterativo para no reventar la pila en grafos grandes.
pub fn dfs_topsort(n: usize, adj: &[Vec<usize>]) -> Topsort {
    // Colores: 0 = blanco (no visto), 1 = gris (en stack), 2 = negro (terminado).
    let mut color = vec![0u8; n];
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (vértice, idx del vecino a explorar)
    let mut order = Vec::with_capacity(n);

    for start in 0..n {
        if color[start] != 0 {
            continue;
        }
        color[start] = 1;
        stack.push((start, 0));

        while let Some((u, i)) = stack.last().copied() {
            if i < adj[u].len() {
                let v = adj[u][i];
                stack.last_mut().unwrap().1 += 1;
                match color[v] {
                    0 => {
                        color[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => {
                        // Back-edge: ciclo.
                        return Topsort::Cycle(
                            (0..n).filter(|&x| color[x] == 1).collect(),
                        );
                    }
                    _ => {} // negro, ya procesado
                }
            } else {
                color[u] = 2;
                order.push(u);
                stack.pop();
            }
        }
    }

    order.reverse();
    Topsort::Order(order)
}

/// Detección rápida de ciclo: ¿el grafo es DAG?
pub fn es_dag(n: usize, adj: &[Vec<usize>]) -> bool {
    matches!(kahn(n, adj), Topsort::Order(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dag_ejemplo() -> (usize, Vec<Vec<usize>>) {
        // 6 módulos con dependencias A->C, A->D, B->D, B->E, C->F, D->F
        let n = 6;
        let mut adj = vec![vec![]; n];
        // A=0, B=1, C=2, D=3, E=4, F=5
        adj[0].extend([2, 3]); // A -> C, A -> D
        adj[1].extend([3, 4]); // B -> D, B -> E
        adj[2].push(5);        // C -> F
        adj[3].push(5);        // D -> F
        (n, adj)
    }

    #[test]
    fn kahn_orden_valido() {
        let (n, adj) = dag_ejemplo();
        if let Topsort::Order(o) = kahn(n, &adj) {
            assert_eq!(o.len(), 6);
            // A y B (los de in-degree 0) deben ir antes que F.
            let pos_f = o.iter().position(|&x| x == 5).unwrap();
            assert!(o.iter().take(pos_f).any(|&x| x == 0));
            assert!(o.iter().take(pos_f).any(|&x| x == 1));
        } else {
            panic!("debería ser un DAG");
        }
    }

    #[test]
    fn dfs_orden_valido() {
        let (n, adj) = dag_ejemplo();
        if let Topsort::Order(o) = dfs_topsort(n, &adj) {
            assert_eq!(o.len(), 6);
        } else {
            panic!("debería ser un DAG");
        }
    }

    #[test]
    fn detecta_ciclo() {
        // a -> b -> c -> a
        let adj = vec![vec![1], vec![2], vec![0]];
        assert!(matches!(kahn(3, &adj), Topsort::Cycle(_)));
        assert!(matches!(dfs_topsort(3, &adj), Topsort::Cycle(_)));
        assert!(!es_dag(3, &adj));
    }
}
```

> **Por qué DFS iterativo**: en Rust, un DFS recursivo profundo puede reventar la pila del sistema en grafos con caminos largos (pocas decenas de miles de vértices ya son peligrosos). Por eso implementamos el DFS con una pila explícita `Vec<(usize, usize)>`. Bonus: al iterativo le añadimos la detección de ciclo *casi* gratis.

## 6.8 Longest path en DAG

```rust
// src/longest_path.rs
//! Longest path en un DAG: DP sobre el orden topológico.

use crate::toposort::{kahn, Topsort};

/// Devuelve la longitud del longest path desde cualquier fuente.
/// Las aristas son (origen, destino, peso).
pub fn longest_path_dag(
    n: usize,
    adj_w: &[Vec<(usize, f64)>],
) -> Option<Vec<f64>> {
    let adj: Vec<Vec<usize>> = adj_w
        .iter()
        .map(|row| row.iter().map(|&(v, _)| v).collect())
        .collect();

    let order = match kahn(n, &adj) {
        Topsort::Order(o) => o,
        Topsort::Cycle(_) => return None,
    };

    let mut dist = vec![f64::NEG_INFINITY; n];
    // Cada vértice sin entrada puede ser fuente con distancia 0.
    for &u in &order {
        if dist[u] == f64::NEG_INFINITY {
            dist[u] = 0.0;
        }
        for &(v, w) in &adj_w[u] {
            if dist[u] + w > dist[v] {
                dist[v] = dist[u] + w;
            }
        }
    }
    Some(dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_path_simple() {
        // A -> B (3), A -> C (2), B -> D (4), C -> D (1)
        // Longest: A->B->D = 7
        let adj_w = vec![
            vec![(1, 3.0), (2, 2.0)], // A
            vec![(3, 4.0)],            // B
            vec![(3, 1.0)],            // C
            vec![],                    // D
        ];
        let dist = longest_path_dag(4, &adj_w).unwrap();
        assert!((dist[3] - 7.0).abs() < 1e-9);
    }

    #[test]
    fn longest_path_con_ciclo() {
        // Ciclo A -> B -> A: no es DAG.
        let adj_w = vec![vec![(1, 1.0)], vec![(0, 1.0)]];
        assert!(longest_path_dag(2, &adj_w).is_none());
    }
}
```

## 6.9 Topological sort con `petgraph`

```toml
# Cargo.toml
[dependencies]
petgraph = "0.6"
```

```rust
// src/toposort_petgraph.rs
//! Topological sort usando `petgraph`.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::DiGraph;

/// Construye un DiGraph y devuelve su orden topológico o un error legible.
pub fn toposort_petgraph(n: usize, aristas: &[(usize, usize)]) -> Result<Vec<usize>, String> {
    let mut g = DiGraph::<(), ()>::new();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }

    match toposort(&g) {
        Ok(iter) => Ok(iter.map(|idx| idx.index()).collect()),
        Err(cycle) => Err(format!(
            "el grafo tiene un ciclo que pasa por el nodo {:?}",
            cycle.node_id()
        )),
    }
}

/// ¿Es el grafo un DAG?
pub fn es_dag_petgraph(n: usize, aristas: &[(usize, usize)]) -> bool {
    let mut g = DiGraph::<(), ()>::new();
    for _ in 0..n {
        g.add_node(());
    }
    for &(u, v) in aristas {
        g.add_edge(u.into(), v.into(), ());
    }
    !is_cyclic_directed(&g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn orden_valido() {
        // A=0, B=1, C=2, D=3, E=4, F=5; aristas como en dag_ejemplo.
        let aristas = vec![(0, 2), (0, 3), (1, 3), (1, 4), (2, 5), (3, 5)];
        let order = toposort_petgraph(6, &aristas).unwrap();
        let set: HashSet<_> = order.iter().copied().collect();
        assert_eq!(set.len(), 6);
        // Verifica la propiedad: para cada arista (u, v), u aparece antes.
        for &(u, v) in &aristas {
            let pos_u = order.iter().position(|&x| x == u).unwrap();
            let pos_v = order.iter().position(|&x| x == v).unwrap();
            assert!(pos_u < pos_v);
        }
    }

    #[test]
    fn detecta_ciclo_petgraph() {
        // a -> b -> a
        let mut g = DiGraph::<(), ()>::new();
        let a = g.add_node(());
        let b = g.add_node(());
        g.add_edge(a, b, ());
        g.add_edge(b, a, ());
        assert!(is_cyclic_directed(&g));
    }
}
```

> **Pista práctica**: cuando uses `toposort` de `petgraph`, el resultado es un `Topological` (un iterador con garantía de orden topológico). Si lo que quieres es sólo saber si hay ciclo, `is_cyclic_directed` es $O(V + E)$ y muy legible. La documentación de `petgraph` es *excelente* — léela antes de reimplementar.

## 6.10 Ejercicios resueltos

### Ejercicio 1 — Orden de compilación

Tienes 6 módulos: `A, B, C, D, E, F` con dependencias `A → C`, `A → D`, `B → D`, `B → E`, `C → F`, `D → F`. Da un orden topológico válido.

**Solución**: Kahn comienza con $\{A, B\}$ (in-degree 0), emite `A, B`, luego procesa `C, D, E` y por último `F`. Un orden válido: `A, B, C, D, E, F`. (Recuerda que el orden no es único: `B, A, C, D, E, F` también vale.)

### Ejercicio 2 — Detección de ciclo en expresiones

Una expresión como `a = b + 1; b = c + 1; c = a + 1` produce un grafo `a → b → c → a`. ¿Es un DAG?

**Solución**: no, tiene un ciclo. Un topological sort devolvería `Cycle([...])` y un compilador lanzaría un error de *inicialización circular*. Esto es exactamente lo que hacen `rustc` y `clang` con variables `let` que se referencian entre sí.

### Ejercicio 3 — PERT simple (4 tareas)

Cuatro tareas con duraciones $A = 3$, $B = 2$, $C = 4$, $D = 2$. Dependencias: $A → C$, $B → C$, $C → D$. Duración crítica y ruta crítica.

**Solución**: longest path desde un nodo fuente: $A + C + D = 9$ o $B + C + D = 8$. La ruta crítica es $A → C → D$ con **9 unidades**. La moraleja PERT: si puedes paralelizar dos tareas, hazlo, pero vigila la cadena que no se puede paralelizar — esa es la ruta crítica.

## 6.11 Ejercicios propuestos

1. **(F) LeetCode 207 — Course Schedule**. Dados `numCourses` y un array de prerrequisitos `[a, b]` (tomar `a` antes que `b`), determina si es posible terminar todos los cursos. Aplica Kahn.
2. **(M) LeetCode 210 — Course Schedule II**. Devuelve *cualquier* orden topológico válido. Si hay varios, basta con uno.
3. **(M) Alien Dictionary**. Dadas palabras de un idioma alienígena ordenadas lexicográficamente, deduce el orden del alfabeto. Pista: comparar pares de palabras adyacentes, extraer la primera diferencia como arista, ejecutar topological sort. Si hay un ciclo, el diccionario es inconsistente.
4. **(M) Longest path con reconstrucción**. Modifica la implementación de §6.8 para devolver, además de la distancia, la **lista de vértices** del camino crítico (guarda el padre).
5. **(D) Detección de deadlock en sistemas distribuidos**. Modela procesos como nodos y “$P_i$ espera un recurso de $P_j$” como aristas $i → j$. Un deadlock es un ciclo. Implementa un detector que use Kahn y emita los procesos a “matar” para romper el ciclo.

## 6.12 Lo que te llevas

- Un **DAG** modela cualquier sistema con dependencias que no se muerden la cola.
- El **orden topológico** existe si y solo si el grafo es un DAG; en caso contrario, hay un ciclo.
- **Kahn** (BFS por in-degree) y **DFS** (reverse postorder) son las dos formas canónicas, ambas $O(V + E)$.
- La **detección de ciclos** es gratis con cualquiera de los dos algoritmos: si Kahn emite menos de $|V|$ vértices, hay ciclo; si el DFS encuentra un back-edge, hay ciclo.
- El **longest path en DAG** se resuelve con DP sobre el orden topológico, y es la base de PERT/CPM.
- En Rust, `petgraph::algo::toposort` y `is_cyclic_directed` te ahorran reinventar la rueda; pero saber hacerlo a mano te salva en entrevistas y en sistemas sin librerías.

## 6.13 Ojo, cuidado con…

- **Recursión profunda en DFS**. Rust no optimiza tail-calls y la pila del sistema es limitada. En grafos con caminos largos, usa el DFS iterativo de §6.7 o un iterador explícito para evitar stack overflows.
- **Grafos cíclicos disfrazados**. Un grafo no-dirigido siempre se puede “convertir” en dirigido para toposort, pero ahí sí o sí hay ciclos (cada arista $u\!-\!v$ genera dos aristas $u → v$ y $v → u$ que forman ciclo). Topological sort **solo aplica a grafos dirigidos**.
- **Múltiples órdenes válidos**. No asumas que el orden que devuelve Kahn o DFS es “el correcto” — solo es *uno* válido. Si tu problema requiere un orden concreto (por ejemplo, lexicográfico), mete los in-degree-0 en una cola de prioridad.
- **Recrear el grafo con el orden topológico**. Si el grafo no es DAG, no existe tal orden. Comprueba `result.is_ok()` antes de iterar.
- **Acumular pesos en `f64`**. Si sumas muchos pesos pequeños, `f64` puede perder precisión. Para grafos grandes o pesos `i64`, considera usar enteros y saturar.

## 6.14 Para profundizar

- **Kahn, A. B. (1962).** “Topological sorting of large networks”. *Communications of the ACM*, 5(11).
- **Tarjan, R. E. (1976).** “Reachability in digraphs”. *SIAM J. Comput.*, 5(2).
- **Cormen et al.,** *Introduction to Algorithms* (3ª ed.), Cap. 22.4 — *Topological sort*.
- **Kleinberg & Tardos,***Algorithm Design*, Cap. 3.6.
- Vídeo: WilliamFiset — *Topological Sort* (<https://www.youtube.com/watch?v=ddTC4Z17l54>).

## 6.15 Pin de batalla

- **Kahn (BFS de in-degree) vs DFS-based postorder.** Los dos correctos; Kahn itera, DFS recursiona. Usa Kahn para grafos grandes.
- **`petgraph::algo::toposort` te da un `Result`.** Unwrap con cuidado; en grafos con ciclos revienta.
- **Longest path en DAG = -shortest path con pesos negados.** Truco clásico. Aplica Dijkstra con `-w` y ya está.
- **PERT/CPM** se modelan como DAGs. El critical path es el longest path.
- **Si tu grafo es "casi DAG" pero tiene un ciclo, mira el feedback loop.** A veces no quieres eliminarlo, sino entenderlo.


## 6.16 Si solo lees 30 segundos

DAG = grafo sin ciclos. Topological sort = orden lineal respetando dependencias. Kahn con in-degree, o DFS postorder. `petgraph` ya lo trae.

## 6.17 Una historia pequeña

Pablo era project manager en una consultora. Manejaba 30 proyectos a la vez, cada uno con 20-50 tareas. Un día se le cayó el sistema de planificación que usaban. En 4 horas, modeló todo en Python con un grafo: tareas como vértices, dependencias como aristas. Aplicó topological sort. Lo que antes le tomaba 2 días reorganizar, ahora le tomaba 1 hora. Su jefe le preguntó qué había usado. "Un grafo," dijo Pablo. "Los mismos que estudió en la carrera, pero entonces no sabía para qué servían." Su jefe le dobló el sueldo. Los algoritmos bien aplicados pagan.


---

