# Prólogo — Vamos a construir una base de datos

> *Borrador. Se completará en la Fase B del workflow.*

Este libro es para ti que has leído el **Volumen I** —o que tienes claros los fundamentos de grafos— y quieres llegar al fondo: **construir tú mismo un motor de base de datos de grafos desde cero, en Rust, y entender cada decisión que hay debajo de Cypher, de MATCH, de un índice de adyacencia y de un log de transacciones.**

Aquí no usamos magia negra. Aquí no "instalamos Neo4j y consultamos". Aquí **implementamos**:

- una representación en memoria (`petgraph` y a mano);
- un fichero en disco con páginas, buffer pool y CSR;
- un lenguaje de consultas mínimo (MATCH-WHERE-RETURN);
- un lexer, un parser, un planificador y un motor de ejecución;
- un write-ahead log y recuperación tras crash;
- tests, benchmarks, observabilidad y una CLI.

El proyecto integrador se llama **LiraDB** (Lite, embedded, didáctico). A lo largo de los 40 capítulos, LiraDB crece desde un `Vec<Vec<usize>>` hasta un motor con persistencia, transacciones y un mini-optimizer.

## Qué asume este volumen

Asumimos que sabes lo básico de grafos (BFS, DFS, Dijkstra) —toda esa base la cubre el Vol.I— y que sabes programar en Rust al nivel de leer un programa y escribir uno sencillo. Si vienes del Vol.I, lo tienes. Si vienes de fuera, lee primero al menos los capítulos 1-4 del Vol.I (representaciones, BFS/DFS, primeros shortest paths) — son el vocabulario que este Vol.II asume.

No asumimos conocimientos previos de:

- internals de bases de datos (páginas, buffer pool, WAL);
- lenguajes de consulta de grafos (Cypher, GQL, SPARQL);
- optimización de consultas;
- sistemas distribuidos.

Todo se construye desde cero.

## Cómo leer este volumen

**Ruta lineal**: lee del capítulo 1 al 40 en orden, haciendo los ejercicios. Tiempo estimado: 100-150 horas. El primer borrador apunta a ~500-650 páginas.

**Ruta focal (tras leer Vol.I)**: si ya sabes grafos y vienes por la parte de motor, salta a la Parte II (cap. 6) y lee en orden. Si vienes por la parte de consultas, ve a la Parte IV (cap. 17).

**Ruta "arquitecto"**: lee solo los capítulos de las Partes III, VI y VIII (almacenamiento, fiabilidad, distribución). Es la versión de 130 páginas que cualquier ingeniero de plataforma debería poder leer en un fin de semana.

## Convenciones del libro

Este Volumen sigue una **plantilla pedagógica híbrida** definida en el **Apéndice 0 — Manual de estilo unificado**. En resumen:

1. Abre con una anécdota histórica (estilo Vol.I).
2. Continúa con el cuerpo técnico de 10 pasos: objetivo, problema, modelo mental, primera solución, sus límites, solución evolucionada, código completo ejecutable, prueba de fuego, qué hemos sacrificado, cómo lo hace una BBDD real + retos.
3. Cierra con las "baterías narrativas" del Vol.I: Lo que te llevas, Ojo cuidado con…, Pin de batalla, Si solo lees 30 segundos, Una historia pequeña, Ejercicios resueltos y propuestos (esencial / intermedio / experto), Para profundizar, Mini-diálogo.

Lee el Apéndice 0 antes de empezar — es breve (~15 pp) y te ahorrará sorpresas.

## Sobre los crates

La política de crates está en `book-context/CONVENTIONS.md` §3. Resumen: usamos `petgraph`, `slotmap`, `serde`, `thiserror`, `clap`, `tracing`, `proptest`, `criterion`, `logos`, `pest` (comparativo), `zerocopy`, `memmap2`, `crc32fast`, `lru`, y opcionalmente `redb`. La regla es **"primero a mano, luego con crate"**: cada componente se construye sin dependencias, luego con la herramienta madura, luego se comparan y se decide.

## Sobre Ladybug / Kùzu

Este libro aprende de la arquitectura de Kùzu (renombrado a Ladybug tras la adquisición por Apple en 2025) como referencia de GDBMS moderno, pero **no copia su código**. La reimplementación es *clean-room conceptual*: leemos los papers, especialmente el Kùzu VLDB 2023, y los artículos de Semih Salihoğlu, y luego escribimos nuestro propio código desde cero. La atribución completa está en el Colofón.

## ¿Qué te llevarás?

Después de leer este libro:

- Habrás implementado **a mano** los componentes de un GDBMS moderno.
- Sabrás por qué cada decisión (slotted pages, CSR, WAL, MVCC, Volcano, factorización) existe y qué trade-off resuelve.
- Podrás leer el código de Neo4j, Kùzu/Ladybug, Cozo o Oxigraph sin que te suene a magia.
- Tendrás un proyecto real —LiraDB— en tu GitHub que demuestra todo lo anterior.
- Y lo más importante: entenderás que las bases de datos no son cajas negras; son software, escrito por personas, con decisiones, compromisos e historia.

Empezamos. Bienvenido al motor.

---

*(El Prólogo se completará cuando se hayan redactado los caps. 1-5. Mientras tanto, este párrafo actúa de placeholder.)*

---

