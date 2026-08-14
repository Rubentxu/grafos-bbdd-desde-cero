# Prólogo

Este libro es para ti.

Para ti que oyes "grafos" y piensas en "eso que estudié en primero de carrera y que olvidé tres meses después". Para ti que llevas años programando y sabes que un BFS existe, pero nunca te has sentado a implementarlo desde cero. Para ti que estudias Machine Learning y todo el mundo te dice "primero aprende grafos" pero nadie te explica cómo. Para ti que vienes de Python y te han dicho que Rust es el futuro, y quieres un proyecto serio para aprender.

Pero también para ti que **no eres programador** y te pica la curiosidad: un psicólogo que estudia redes sociales, una bióloga que analiza proteínas, una diseñadora de juegos que quiere entender pathfinding, una periodista que investiga cómo funciona PageRank, un estudiante de secundaria que ha visto un grafo en un vídeo de YouTube y quiere más. Este libro es para todos. Si no te lo crees, mira los diálogos de ascensor y las "historias pequeñas" al final de cada capítulo — están escritos para que un cuñado los entienda.

Y para ti que llevas media vida entre grafos, conoces a Dijkstra por el nombre de pila, y aún así quieres volver a las raíces con buen material.

## Qué hay de nuevo en esta edición extendida

En la primera edición cubrimos los **20 capítulos clásicos** de teoría de grafos, de Euler a las GNN. En esta edición ampliada añadimos:

- **12 capítulos nuevos** (21 a 32) cubriendo cómo los grafos son la columna vertebral de **toda la informática moderna**: bases de datos, compiladores, sistemas operativos, redes, sistemas distribuidos, seguridad, bioinformática, NLP, robótica, verificación, recomendadores y quantum computing.
- **5 proyectos finales extra** correspondientes a las nuevas áreas.
- **Un glosario extendido** con ~80 términos nuevos del mundo real.
- **Estilo Grokking 2.0** aplicado a los capítulos nuevos: hooks, regla de tres, mini-diálogos, ilustraciones ASCII, "pin de batalla", "si solo lees 30 segundos" e "historias pequeñas" con personajes ficticios. Si te enganchó "Grokking Algorithms" de Aditya Bhargava, te sentirás como en casa.

## Qué asume este libro

No mucho. Asumimos que sabes programar en algún lenguaje (idealmente Rust, al menos lo básico para leer un programa; si no, te enseñamos lo necesario sobre la marcha). No asumimos que recuerdes nada de matemáticas de grafos. La construiremos desde cero.

Si eres completamente nuevo en Rust, dedicamos los primeros capítulos a Rust idiomático: structs, enums, traits, módulos y `#[test]`. Verás que Rust es exigente pero honesto: te avisa de los errores en tiempo de compilación en lugar de dejarte descubrirlos en producción a las 3 de la mañana. Es perfecto para algoritmos.

Si ni siquiera sabes programar, los capítulos nuevos (sobre todo 21-32) están escritos con analogías tan visuales que se pueden leer saltándose el código. Adelante.

## Cómo leer este libro

**Ruta lineal** (principiante curioso): lee del Capítulo 1 al 32 en orden, haciendo todos los ejercicios. Tardarás entre 130 y 180 horas, dependiendo de tu ritmo. Pero te prometemos que cada hora vale la pena.

**Ruta focal** (programador con experiencia): salta a la sección que te interese, pero asegúrate de leer primero los capítulos 1, 3 y 4 (representaciones, BFS/DFS, primeros shortest paths) porque son el vocabulario que todo lo demás asume.

**Ruta avanzada** (experto que repasa): lee solo las anécdotas históricas y las secciones "Lo que te llevas" de cada capítulo. Te servirá como compendio rápido y como recordatorio cultural. Luego, ve directo a los capítulos 9-12 (flujos) y 18-20 (NP-completitud, DP, GNN) para refrescar lo que más uses. Y por supuesto, salta a la nueva Parte VI (caps 21-32) para ver cómo los grafos viven en tu stack de trabajo diario.

**Ruta "para tu madre"** (no-programador curioso): lee los hooks, las anécdotas, los "pin de batalla", los "si solo lees 30 segundos" y las "historias pequeñas". Con eso tendrás el 80% de la diversión sin tocar una línea de código. Si te pica, vuelve a las secciones de teoría (subsección 1) sin mirar el código.

## Convenciones del libro

- **Tú**: nos tuteamos. Esto no es un paper; es una conversación.
- **Términos en negrita**: palabras que se introducen por primera vez. Aparecen en el glosario final.
- **Bloques de código**: Rust idiomático, Rust **2024**. Los snippets son ejecutables; cuando un crate aparece, mostramos el `Cargo.toml` necesario.
- **Diagramas ASCII**: cuando un grafo se entiende mejor dibujado, lo dibujamos. No es arte, es claridad.
- **"Ojo, cuidado con..."**: trampas comunes que pilla todo el mundo al principio. Léelas; te ahorrarás horas de debug.
- **"Lo que te llevas"**: el resumen de 3-5 ideas que el capítulo quiere que recuerdes dentro de un año.
- **"Pin de batalla"**: tips prácticos aprendidos con sangre. Cosas que solo sabes después de pegarte con el código.
- **"Si solo lees 30 segundos"**: la versión TL;DR del capítulo. Explica el concepto a tu madre en media frase.
- **"Una historia pequeña"**: un personaje ficticio aplicando lo aprendido. Para que el concepto se quede.
- **"Para profundizar"**: 3-5 referencias (papers seminales, libros, vídeos). Si un tema te atrapa, ahí tienes por dónde tirar del hilo.
- **Diálogos de ascensor**: conversaciones entre personajes inventados. Porque explicar a otro es la mejor manera de entender.

## Sobre las anécdotas históricas

Cada capítulo abre con una. A veces son del inventor, a veces del problema original, a veces del contexto que hizo falta. La idea es que cuando oigas "algoritmo de Dijkstra" no pienses solo en una fórmula: pienses en Edsger Dijkstra comprando café con su prometida en Amsterdam un domingo de 1956, cuando se le ocurrió la idea de los 20 minutos. Las matemáticas son más fáciles de recordar cuando hay alguien detrás.

Cuando la historia del personaje o del algoritmo es desconocida, lo decimos: "hay varias versiones de esta anécdota, las fuentes no se ponen de acuerdo". La honestidad histórica es parte de la diversión.

## Sobre los crates

Rust tiene un ecosistema excelente para grafos. Este libro usa:

- **`petgraph`**: el crate estándar de facto. Lo verás en casi todos los capítulos.
- **`criterion`**: para benchmarks. Lo introducimos con Dijkstra vs Floyd-Warshall.
- **`ratatui`**: para interfaces TUI donde visualizamos un grafo en la terminal.
- **`image`**: para procesar imágenes (visualizamos Aho-Corasick sobre un PNG).
- **`nalgebra`**: para matrices y autovalores.
- **`ndarray`**: para la mini-GCN del capítulo final y la factorización de matrices en recomendadores.
- **`rand`**: para algoritmos randomizados (Karger, random walks, MCTS).
- **`good-lp`**: para el algoritmo húngaro con pesos.
- **`tokio`**: para async (simulaciones de redes y sistemas distribuidos).
- **`bio`**: para alineamiento de secuencias.
- **`syn` y `quote`**: para parsear el AST de Rust.
- **`qrust` / `qoqo`**: para simular circuitos cuánticos.

Si no conoces alguno, te explicamos su `Cargo.toml` la primera vez que aparezca. Si prefieres Python, este libro tiene una versión paralela con todo el código en Python; los dos comparten estructura y anécdotas.

## ¿Qué te llevarás?

Después de leer este libro:

- Sabrás modelar **cualquier** problema de grafos y elegir la representación correcta.
- Podrás implementar **a mano** los algoritmos clásicos (BFS, DFS, Dijkstra, Bellman-Ford, Kruskal, Prim, Ford-Fulkerson, Dinic, A*, MCTS).
- Conocerás los **crates** clave y cuándo usar cada uno.
- Habrás visto cómo los grafos llegan a **Machine Learning** (PageRank, GCN, DeepWalk).
- **Y lo más importante**: entenderás que los grafos están en TODAS PARTES en informática. Cada vez que abras un IDE, una base de datos, una red, un compilador, un sistema distribuido, un ataque de seguridad, una proteína, una frase en NLP, un robot, un test, un recomendador o un computador cuántico, hay un grafo debajo. Después de leer este libro, mirarás el software con otros ojos.

Empezamos. Bienvenido a la mejor esquina de las matemáticas discretas.

---

