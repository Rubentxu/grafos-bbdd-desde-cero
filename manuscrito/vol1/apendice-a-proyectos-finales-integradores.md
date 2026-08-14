# Apéndice A — Proyectos finales integradores

Has terminado las 6 partes del libro. Ahora, a construir. Los siguientes diez proyectos ponen en juego los temas del libro. Empieza por el que más te apetezca; cada uno es autocontenido. Se estiman entre 4 y 30 horas de trabajo, según tu nivel.

---

## Proyectos de las Partes I-V (ya existentes)

(Estos 5 proyectos ya los conoces de la primera edición. Aquí los dejamos como referencia rápida.)

1. **Resolvedor de laberintos desde una imagen** (Parte I, Fundamentos): 4-8 horas.
2. **Diseñador de rutas de tren** (Parte II, Algoritmos centrales): 6-12 horas.
3. **Planificador de evacuaciones** (Parte III, Flujos): 8-15 horas.
4. **Visualizador interactivo de coloración y planaridad** (Parte IV, Avanzados): 12-20 horas.
5. **Detector de comunidades con GNN** (Parte V, ML): 15-25 horas.

---

## Proyectos NUEVOS de la Parte VI (Informática Moderna)

### Proyecto 6 — Mini-graph-DB tipo Neo4j (Parte VI: Bases de Datos)

**Temas**: Grafos como modelo de datos, Cypher-like queries, índices, persistencia.

**Descripción**: Implementa una mini base de datos de grafos en Rust, con un lenguaje de queries tipo Cypher simplificado. La idea: cargar nodos y aristas desde un fichero CSV, ejecutar 3 queries reales (búsqueda de paths, vecinos, shortest path), y exportar resultados.

**Plan paso a paso**:
1. Define tipos `Nodo { id, label, propiedades: HashMap<String, Value> }` y `Arista { from, to, label, peso }`.
2. Implementa un parser mini-Cypher que acepte: `MATCH (n:Usuario)-[:AMIGO_DE]->(m) RETURN n, m`.
3. Construye el grafo con `petgraph`.
4. Ejecuta las queries usando BFS/DFS/shortest path.
5. Persiste a disco en formato binario simple.
6. Haz tests con un dataset de 1000 usuarios y 5000 amistades.

**Extensiones**:
- Soporta `WHERE` para filtros sobre propiedades.
- Soporta agregaciones (`count`, `sum`).
- Índices secundarios sobre propiedades.
- Visualización web con `actix-web` o `axum`.

**Crates sugeridos**: `petgraph`, `serde`, `csv`.

---

### Proyecto 7 — Compilador de expresiones a LLVM IR (Parte VI: Compiladores)

**Temas**: AST, parsing, generación de IR, optimización simple, coloración de grafos para register allocation.

**Descripción**: Escribe un compilador de expresiones aritméticas con variables (por ejemplo: `let x = (a + b) * c; y = x - 1;`) que produzca LLVM IR válido. Bonus: implementa un register allocator que use coloración de grafos del Cap 13.

**Plan paso a paso**:
1. Define el AST: `Expr = Num(f64) | Var(String) | BinOp(Box<Expr>, Op, Box<Expr>)`.
2. Implementa un lexer con `logos` o a mano.
3. Implementa un parser recursivo descendente.
4. Genera LLVM IR textual (un subconjunto: `alloca`, `load`, `store`, `fadd`, `fmul`, `fsub`).
5. Implementa un analizador de liveness: en cada punto, qué variables están vivas.
6. Construye el interference graph (variables que no pueden estar en el mismo registro).
7. Aplica coloración de grafos para asignar registros.
8. Compila con `clang` y verifica que el binario corre.

**Extensiones**:
- Añade if/else, while.
- Añade funciones con stack frames.
- Optimizaciones: constant folding, dead code elimination.
- Genera WebAssembly en vez de LLVM IR.

**Crates sugeridos**: `logos`, `lalrpop` o parser manual, `inkwell` (bindings LLVM).

---

### Proyecto 8 — Simulador de deadlock en un sistema operativo (Parte VI: SO)

**Temas**: Resource Allocation Graph, detección de ciclos, algoritmo del banquero.

**Descripción**: Modela un sistema con N procesos y M tipos de recursos. Permite asignación y petición dinámica. Detecta deadlocks visualizando el RAG.

**Plan paso a paso**:
1. Define tipos: `Proceso`, `Recurso { id, instancias }`, `Asignacion`, `Peticion`.
2. Mantén el RAG como un `DiGraph` de `petgraph`.
3. Implementa `peticion(proceso, recurso)`: añade arista `Proceso -> Recurso`.
4. Implementa `asignar(proceso, recurso)`: si hay instancias libres, asigna y mueve arista a `Recurso -> Proceso`.
5. Detecta deadlock: si hay un ciclo en el RAG, hay deadlock.
6. Algoritmo del banquero: dado un nuevo estado, calcula si es seguro.
7. Visualiza el RAG en ASCII con caracteres Unicode (▶, ◀, ●).

**Extensiones**:
- Implementa 4 estrategias de prevención (romper una condición de Coffman).
- Soporta preemption.
- Múltiples instancias por tipo de recurso.
- Visualización con `ratatui` interactiva.

**Crates sugeridos**: `petgraph`, opcionalmente `ratatui`.

---

### Proyecto 9 — Simulador de red con OSPF (Parte VI: Redes)

**Temas**: Topología de red, link-state, Dijkstra, simulaciones de fallos.

**Descripción**: Construye un simulador de red donde los routers ejecutan OSPF (que ya conoces del Cap 24). Cuando un enlace se cae, los routers recalculan sus rutas.

**Plan paso a paso**:
1. Define `Router { id, lsdb: HashMap<RouterId, Vec<Enlace>> }` (link-state database).
2. Construye una topología aleatoria de 20 routers y 50 enlaces con `rand`.
3. Cada router ejecuta Dijkstra sobre su LSDB para construir la tabla de enrutamiento.
4. Simula el envío de un paquete de A a B, mostrando la ruta.
5. Simula la caída de un enlace, los routers inundan LSAs (Link State Advertisements) y recalculan.
6. Mide el tiempo de convergencia.
7. Visualiza la topología con `ratatui` o exporta a `graphviz` (formato `.dot`).

**Extensiones**:
- Simula ataques (router comprometido que inyecta LSA falsas).
- BGP simplificado para comunicación entre ASes.
- Métricas: jitter, packet loss.
- Topología jerárquica (áreas OSPF).

**Crates sugeridos**: `petgraph`, `rand`, opcionalmente `ratatui`.

---

### Proyecto 10 — Mini-recomendador estilo Netflix (Parte VI: Recomendadores)

**Temas**: Collaborative filtering, matrix factorization, evaluación, A/B testing.

**Descripción**: Implementa un sistema de recomendación de películas estilo Netflix Prize. Usa un dataset público (MovieLens 25M, ~25 millones de ratings), entrena un modelo de matrix factorization, y evalúa con MAP@K.

**Plan paso a paso**:
1. Descarga MovieLens 25M y parsea con `csv`.
2. Construye la matriz R (usuarios × items) sparse con `ndarray`.
3. Implementa matrix factorization: R ≈ P · Q^T, donde P y Q son embeddings latentes.
4. Entrena con SGD minimizando MSE + regularización L2.
5. Para cada usuario, predice ratings y rankea películas no vistas.
6. Evalúa con MAP@10 en un set de test.
7. Implementa un servidor HTTP con `axum` que devuelva recomendaciones.

**Extensiones**:
- Compara con baselines: popularidad global, item-item CF.
- Implementa el algoritmo de Funk SVD (sin bias).
- Soporta contenido (género, año) además de collaborative.
- Implementa bandit exploration (epsilon-greedy, UCB).
- A/B testing: simula dos algoritmos y mide click-through rate.

**Crates sugeridos**: `ndarray`, `csv`, `axum`, `rand`.

---

