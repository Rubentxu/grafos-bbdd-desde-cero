# Apéndice B — Glosario (extendido)

(El glosario original se mantiene. Aquí añadimos los términos nuevos de la Parte VI.)

**A**

- **Aho-Corasick**: algoritmo para buscar múltiples patrones en un texto en $O(n + m + k)$ usando un trie con fail links.
- **Algoritmo de Kuhn**: método DFS con caminos aumentantes para encontrar un matching bipartito máximo.
- **Algoritmo del banquero**: algoritmo de Edsger Dijkstra (1965) para evitar deadlocks en sistemas con recursos.
- **Algoritmo húngaro**: algoritmo $O(n^3)$ para matching bipartito con pesos.
- **Anécdota de esquina**: el formato que usamos al inicio de cada capítulo. Breve, histórica, humana.
- **Ataque (grafo de)**: modelado de cómo un atacante compromete un sistema a través de exploits encadenados.
- **Attack surface**: superficie de ataque; conjunto de puntos por donde un sistema puede ser comprometido.
- **AST (Abstract Syntax Tree)**: árbol (grafo) que representa la estructura sintáctica de un programa. El primer paso de un compilador.
- **Asymptotic Bound**: cota asintótica del coste de un algoritmo (O grande, Ω, Θ).

**B**

- **BGP (Border Gateway Protocol)**: el protocolo de enrutamiento que decide cómo viajan los paquetes entre sistemas autónomos. Política + shortest path.
- **Bipartito**: grafo cuyos vértices se dividen en dos conjuntos con aristas solo entre conjuntos. Equivale a no tener ciclos de longitud impar.
- **Bisimulation**: relación entre dos grafos de estados que indica que son "equivalentes en comportamiento".
- **Boyer-Myrvold**: algoritmo lineal para testar planaridad de un grafo.
- **Bridge (puente)**: arista cuya eliminación desconecta el grafo.

**C**

- **CFG (Control Flow Graph)**: grafo de bloques básicos y aristas que muestran todos los caminos de ejecución posibles de un programa.
- **Cold start (recommender)**: el problema de recomendar a un usuario nuevo del que no tienes datos.
- **Collaborative filtering**: técnica de recomendación que usa los ratings de usuarios similares para predecir.
- **Consenso (en distribuidos)**: protocolo para que N nodos acuerden un valor a pesar de fallos. Paxos, Raft.
- **Cypher**: lenguaje de queries de Neo4j y otros graph DBs. El "SQL de los grafos".
- **Cortesía del compilador**: errores de compilación que son técnicamente correctos pero moralmente inexplicables.

**D**

- **DAG (Directed Acyclic Graph)**: grafo dirigido sin ciclos. Acepta un orden topológico.
- **Dataflow analysis**: análisis estático que rastrea cómo fluyen los datos en un programa. Liveness, reaching definitions, etc.
- **Deadlock**: situación donde 2+ procesos se bloquean mutuamente esperando recursos. Detectable con ciclos en el RAG.
- **DFG (Data Flow Graph)**: grafo que muestra cómo los datos se mueven entre operaciones.
- **DHT (Distributed Hash Table)**: tabla hash distribuida donde cada nodo es responsable de un rango de claves. Chord, Kademlia.
- **Dijkstra (algoritmo)**: $O((V+E) \log V)$ para shortest paths con pesos no negativos.
- **Dijkstra (algoritmo del banquero)**: el mismo Dijkstra. Sí, el mismo.
- **Distributed consensus**: ver consenso.
- **DSATUR**: heurística para coloración de grafos que elige el siguiente vértice por saturación de colores.
- **DSU / Union-Find**: estructura de datos para mantener conjuntos disjuntos con unión y búsqueda casi-constantes.

**E**

- **Epidemic protocol**: ver gossip protocol.
- **Espacio de estados (state space)**: el conjunto de todas las configuraciones posibles de un sistema. A menudo explota.
- **Estado seguro (banquero)**: estado del sistema en el que existe una secuencia de grants que permite a todos los procesos terminar.

**F**

- **FAA (Finite Automaton)**: ver FSM.
- **Feynman (Richard)**: físico que en 1981 dijo "la naturaleza no es clásica, maldita sea, y si quieres simularla, mejor hazlo cuántico". Fundador del quantum computing.
- **Flujo (en red)**: asignación de cantidades a aristas que respeta capacidades y conservación de flujo en vértices.
- **Floyd-Warshall**: $O(V^3)$ para shortest paths entre todos los pares.
- **FSM (Finite State Machine)**: grafo de estados, transiciones, acciones. La base de parsers, protocolos, sistemas reactivos.
- **Fuerza bruta (en matching)**: probar todas las combinaciones posibles.

**G**

- **GNN (Graph Neural Network)**: red neuronal que opera sobre grafos. Kipf-Welling 2017.
- **Grafo**: par $G = (V, E)$ con $V$ vértices y $E$ aristas.
- **Grafo completo ($K_n$)**: grafo con $n$ vértices donde todos están conectados con todos.
- **Grafo conexo**: grafo donde todo par de vértices está conectado por algún camino.
- **Grafo dirigido (dígrafo)**: grafo con aristas con dirección.
- **Grafo plano (planar)**: grafo que admite un dibujo en el plano sin cruces de aristas.
- **Gossip protocol**: protocolo de propagación de información en redes grandes donde cada nodo "habla" con un par aleatorio. Como rumores en una oficina.
- **Grado (de un vértice)**: número de aristas incidentes.
- **Graph database**: base de datos optimizada para almacenar y consultar grafos. Neo4j, ArangoDB, JanusGraph.
- **Greedy**: estrategia que toma la mejor decisión local en cada paso.
- **Grover (algoritmo de)**: búsqueda cuántica no estructurada en $\sqrt{N}$ pasos.

**H**

- **Hamilton (camino/ciclo)**: visita cada vértice exactamente una vez.
- **Hitting time (en random walk)**: tiempo esperado para visitar un vértice concreto.
- **Hook (de capítulo)**: las 3-5 primeras líneas de un capítulo que enganchan al lector. Pregunta provocadora, escenario, afirmación.
- **Hopcroft-Karp**: $O(E\sqrt{V})$ para matching bipartito máximo.

**I**

- **In-degree**: número de aristas que entran a un vértice.
- **Inferencia de tipos (type inference)**: el compilador deduce los tipos de tus variables. Haskell es el rey. En Rust, mitad inferencia mitad anotación.
- **Inyección SQL**: ataque donde el usuario introduce SQL malicioso en un input. Se previene con prepared statements.
- **Interference graph**: grafo que indica qué variables no pueden compartir registro. Base del register allocation.
- **IOC (Indicator of Compromise)**: señal de que un sistema ha sido comprometido. IP, hash, dominio malicioso.

**J**

- **Job-shop scheduling**: problema de scheduling donde trabajos compiten por máquinas. NP-hard.

**K**

- **Karger (algoritmo de)**: random contraction para min-cut global.
- **Knowledge graph**: grafo semántico que codifica entidades y relaciones. Google Knowledge Graph, Wikidata.
- **Kruskal (algoritmo de)**: MST por aristas ordenadas con Union-Find.
- **König (teorema de)**: en grafo bipartito, matching máximo = vertex cover mínimo.

**L**

- **Laplaciana (matriz)**: $L = D - A$ donde $D$ es la matriz de grados y $A$ la de adyacencia.
- **Liveness analysis**: análisis estático que determina qué variables están "vivas" (se usará su valor en el futuro) en cada punto del programa.
- **LLM (Large Language Model)**: modelos de lenguaje grandes. GPT, Claude, Llama. No son grafos pero usan grafos por dentro (atención = grafo de tokens).
- **Louvain (algoritmo de)**: método para detectar comunidades en redes grandes. O(n log n).
- **Low-link value**: en DFS, valor `low[v] = min(discovery[w])` para ancestros y back-edges.
- **LTL (Linear Temporal Logic)**: lógica para especificar propiedades sobre secuencias de estados. Usada en model checking.

**M**

- **MapReduce**: modelo de programación distribuida. Las funciones `map` y `reduce` forman un DAG.
- **Matrix factorization**: descomposición de una matriz como producto de dos. Base de muchos recomendadores.
- **Max-flow (problema de)**: maximizar el flujo desde un origen a un sumidero en una red.
- **Max-flow min-cut (teorema)**: en una red, el valor del flujo máximo = capacidad del corte mínimo.
- **MCTS (Monte Carlo Tree Search)**: búsqueda en árbol usando simulaciones aleatorias. AlphaGo.
- **Message passing (en GNN)**: esquema general de las GNN modernas: cada nodo recibe mensajes de vecinos, los agrega, y actualiza su estado.
- **Min-cut**: corte de capacidad mínima.
- **Min-cost flow**: flujo de coste total mínimo.
- **Minimax**: algoritmo para juegos de 2 jugadores donde uno maximiza y el otro minimiza. Con alpha-beta pruning.
- **Model checking**: verificación formal que explora el grafo de estados y comprueba propiedades lógicas.
- **Motion planning**: en robótica, encontrar un camino libre de colisiones del estado A al B.

**N**

- **Neo4j**: el graph database más popular. Usa Cypher.
- **Network topology**: cómo se organizan los nodos de una red. Estrella, anillo, malla, etc.
- **NLP (Natural Language Processing)**: procesamiento de lenguaje natural.
- **NP-completo**: clase de problemas que están en NP y cualquier problema en NP se reduce a ellos.
- **NP-hard**: al menos tan difícil como cualquier problema en NP.
- **Needleman-Wunsch**: algoritmo de alineamiento global de secuencias. DP sobre una matriz.

**O**

- **OSI (modelo)**: modelo de 7 capas que describe cómo se comunican los computadores en red.
- **Orden topológico**: orden lineal de vértices de un DAG tal que cada arista va de un vértice anterior a uno posterior.
- **OSPF (Open Shortest Path First)**: protocolo de routing link-state que usa Dijkstra sobre el grafo de la red.
- **Out-degree**: número de aristas que salen de un vértice.

**P**

- **PageRank**: medida de importancia de un nodo basada en la estructura de enlaces; equivalente a un random walk con reset.
- **Paxos**: protocolo de consenso para sistemas distribuidos. Famósamente difícil de entender.
- **Path compression**: optimización de Union-Find que aplana árboles durante `find`.
- **PDG (Program Dependence Graph)**: grafo que muestra dependencias de datos y control en un programa.
- **Permutation flowshop**: problema de scheduling donde N trabajos pasan por M máquinas en el mismo orden. NP-hard.
- **Phylogenetics**: reconstruir árboles evolutivos a partir de datos moleculares.
- **Pin de batalla**: las secciones de tips prácticos al final de cada capítulo. Aprendizajes con sangre.
- **Planar (grafo)**: grafo que admite un embedding planar.
- **PageRank personalizado (Personalized PageRank)**: random walk con restart desde un nodo específico. Usado en recomendadores.
- **Prim (algoritmo de)**: MST por vértices, usando un heap.
- **Probabilistic method**: técnica que prueba existencia mostrando que un evento aleatorio tiene probabilidad positiva.
- **Protein-Protein Interaction (PPI)**: red de proteínas y sus interacciones físicas. Modelada como grafo.

**Q**

- **Qubit**: unidad básica de información cuántica. Puede estar en superposición de |0⟩ y |1⟩.
- **Quantum circuit**: secuencia de puertas cuánticas aplicadas a qubits. Se modela como un grafo.
- **Quantum walk**: análogo cuántico de un random walk en grafos.

**R**

- **Raft**: protocolo de consenso más amigable que Paxos. Usado en Kubernetes, etcd, Consul.
- **Random walk**: secuencia de vértices donde cada paso es a un vecino aleatorio.
- **RAG (Resource Allocation Graph)**: grafo de procesos, recursos, asignaciones y peticiones. Detección de deadlock = encontrar ciclo.
- **Reachable (en grafos)**: vértice $v$ es alcanzable desde $u$ si existe un camino de $u$ a $v$.
- **RecSys (Recommendation System)**: sistema que recomienda items a usuarios.
- **Reweight**: técnica de preproceso que transforma pesos para que Dijkstra funcione tras Bellman-Ford.
- **Residual (grafo)**: en flujo, grafo que indica cuánto flujo adicional puede ir por cada arista.
- **Rust 2024**: edición del lenguaje Rust lanzada en 2024, con varias mejoras idiomáticas (entre otras: `if let` chaining, nuevas APIs en la std, mejoras en el sistema de traits). Es la edición usada en todo este libro a partir de la presente revisión.

**S**

- **SAT**: problema de satisfacibilidad booleana. El primer problema demostrado NP-completo.
- **SCC**: ver componente fuertemente conexa.
- **Shortest path (camino más corto)**: camino entre dos vértices de peso total mínimo.
- **Si solo lees 30 segundos**: el TL;DR del capítulo. Explica a tu madre en media frase.
- **Spectral (teoría)**: estudio de grafos mediante los autovalores de sus matrices de adyacencia o Laplaciana.
- **SSA (Static Single Assignment)**: forma intermedia donde cada variable se asigna exactamente una vez. Base de LLVM.
- **State space explosion**: el problema de que un sistema con N componentes tiene $2^N$ estados. La maldición de la verificación.
- **STRIDE**: modelo de amenazas de Microsoft. Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege.
- **Suffix array**: array de sufijos ordenados lexicográficamente.
- **Suffix tree**: trie de todos los sufijos de una cadena; construcción $O(n)$ por Ukkonen.
- **Supply chain attack**: ataque donde comprometes un proveedor para llegar a todos tus clientes. Log4Shell (2021) es el ejemplo canónico.

**T**

- **Tarjan (algoritmo de)**: SCC, bridges, articulation points en $O(V+E)$.
- **Threat modeling**: modelar las amenazas de un sistema. A menudo como grafos de ataque.
- **Topological sort**: ver orden topológico.
- **Transition system**: ver FSM.
- **Tries**: estructura de datos en árbol para prefijos.
- **Two-coloring**: coloración con 2 colores (equivalente a bipartito en no-dirigidos).
- **Type inference**: ver inferencia de tipos.

**U**

- **Ukkonen (algoritmo de)**: construcción de suffix tree en $O(n)$.
- **Union-Find**: ver DSU.

**V**

- **Vértice**: nodo de un grafo.
- **Vertex cover**: subconjunto de vértices tal que toda arista tiene al menos un extremo en él.
- **Vizing (teorema de)**: número cromático de aristas está entre $\Delta$ y $\Delta + 1$.

**W**

- **Weighted (grafo)**: ver ponderado.
- **Word embedding**: representación vectorial densa de una palabra. Word2Vec, GloVe. A veces se aprende con random walks en grafos.

**Z**

- **Zachary's Karate Club**: dataset clásico de redes sociales (1977) usado en análisis de comunidades.

---

