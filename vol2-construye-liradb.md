---
title: "Construye una base de datos de grafos desde cero — De los algoritmos fundamentales a un motor persistente de consultas en Rust"
subtitle: "Volumen II de Grafos en Computación: de Cero a Experto"
author: "Rubentxu"
date: "2026-07-30"
lang: es
volumen: II
obra: "Grafos en Computación: de Cero a Experto"
proyecto_integrador: LiraDB
edicion: "Edición unificada 2026 — primer borrador"
licencia: "CC BY-NC-SA 4.0"
---

# Construye una base de datos de grafos desde cero

**De los algoritmos fundamentales a un motor persistente de consultas en Rust**

*Volumen II de la obra "Grafos en Computación: de Cero a Experto" — Proyecto integrador LiraDB.*

---

> «Si puedes implementarlo en Rust, lo entiendes. Si lo entiendes, puedes mejorarlo.»
> — Manifiesto LiraDB

---

**Edición**: Primer borrador, julio de 2026
**Volumen**: II (de II)
**Idioma**: Español (con terminología técnica estándar en inglés)
**Stack**: Rust **2024** edition + `petgraph` + crates seleccionadas (ver Apéndice 0)
**Proyecto integrador**: **LiraDB** (a.k.a. LiraDB Lite)
**Licencia**: CC BY-NC-SA 4.0

---

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

# Tabla de contenidos

> *Borrador — se generará automáticamente al cierre de la Fase B.*

**Prólogo — Vamos a construir una base de datos**

**Parte I — Pensar en grafos**
1. Qué es realmente un grafo
2. Cómo representar un grafo en memoria
3. Identidad, referencias y datos estables
4. El primer recorrido: búsqueda en anchura (BFS)
5. Profundidad, ciclos y componentes (DFS, componentes conexos, ordenación topológica, SCC)

**Parte II — De estructura de datos a base de datos**
6. Qué convierte un grafo en una base de datos
7. El modelo de datos de LiraDB (Property Graph + Value)
8. Diseñar una API antes de persistir (trait `GraphStore`)
9. Del objeto al byte (encoding, endianness, versionado)
10. Persistencia append-only

**Parte III — Construir el motor de almacenamiento**
11. Páginas, bloques y organización del fichero (slotted pages, metapágina)
12. El gestor de páginas (trait `Pager`)
13. El buffer pool (LRU, Clock, métricas)
14. Cómo almacenar adyacencias (CSR, segmentos)
15. Índices para encontrar datos (hash + B+ tree; apéndice del capítulo: B+ tree multinivel y splits)
16. Compactación y mantenimiento (`liradb inspect|check|compact`; sección: LSM-trees comparados)

**Parte IV — Consultar el grafo**
17. Diseñar un lenguaje pequeño (MATCH-WHERE-RETURN mini)
18. Construir el lexer y el parser
19. Del AST al plan lógico
20. El motor de ejecución (modelo Volcano)
21. Un optimizador pequeño pero real (`liradb explain`; sección: estadísticas y estimación de cardinalidad)

**Parte V — Algoritmos sobre el grafo persistente**
22. Caminos mínimos ponderados (Dijkstra, Bellman-Ford)
23. A*, heurísticas y búsquedas dirigidas
24. Centralidad y PageRank
25. Comunidades y agrupaciones (Louvain simplificado)
26. Ejecutar algoritmos sin agotar la memoria (proyección, streaming, frontiers)

**Parte VI — Fiabilidad**
27. Qué significa una transacción (ACID)
28. Write-ahead log
29. Recuperación después de un fallo
30. Snapshots, concurrencia y aislamiento (MVCC, 2PL, niveles de aislamiento y sus anomalías, OCC, deadlocks con grafo de espera)

**Parte VII — Convertir el proyecto en un producto técnico**
31. La CLI de LiraDB
32. Importación y exportación (CSV, JSONL, GraphML)
33. Pruebas de una base de datos
34. Benchmarks y perfilado
35. Observabilidad interna
36. Arquitectura final de LiraDB

**Parte VIII — De LiraDB Lite a un sistema avanzado**
37. Qué necesitaría una base de datos de producción
38. Almacenamiento columnar y ejecución vectorizada (sección: compresión — diccionario, RLE, bit-packing, delta)
39. Joins, patrones y consultas cíclicas (WCOJ)
40. Distribuir una base de datos de grafos (nota: híbridos vector+grafo distribuidos)

**Epílogo — Ya sabes construir una base de datos**

**Apéndice 0 — Manual de estilo unificado**
**Apéndice A — Proyectos finales integradores de LiraDB**
**Apéndice B — Glosario específico de BBDD de grafos**
**Apéndice C — Bibliografía y referencias (DBMS + Ladybug/Kùzu papers)**
**Apéndice D — ADRs (dependency policy, página, WAL, format versioning)**
**Apéndice E — Mapa de "cómo lo resuelve una BBDD real" (Neo4j / Kùzu→Ladybug y forks post-adquisición / Cozo / Oxigraph; paisaje 2026 con GQL ISO y Neo4j vector)**

---

*(El cuerpo de los 40 capítulos se redactará en las Fases B-C del workflow. Este archivo es un esqueleto navegable.)*

---

# Apéndice 0 — Manual de estilo unificado

> *Borrador inicial — se completará en la Fase 2.*

## 0.1. Por qué un manual de estilo común

Esta obra se publica en **dos volúmenes** con voces distintas:

- El **Volumen I** ("Grafos en Computación: de Cero a Experto") tiene una voz **narrativa y divulgativa**, basada en el estilo que Aditya Bhargava popularizó como "Grokking Algorithms": hooks, anécdotas históricas, regla de tres, humor inesperado, ASCII art, "Pin de batalla", "Si solo lees 30 segundos", "Una historia pequeña" y Diálogos de ascensor.

- El **Volumen II** ("Construye LiraDB") tiene una voz **ingenieril y metódica**, basada en la plantilla pedagógica de 10 pasos del brief original de LiraDB: objetivo → problema → modelo mental → primera solución → sus límites → solución evolucionada → código completo ejecutable → prueba de fuego → qué hemos sacrificado → cómo lo hace una BBDD real + retos.

Ambas voces son válidas y complementarias. El Vol.I te enseña *qué es* un grafo y *qué algoritmos existen*. El Vol.II te enseña *cómo construir* un sistema que los persiste y los consulta. La fusión en una sola obra exige un manual que documente ambas plantillas y diga **cuándo y cómo se aplican**.

## 0.2. Las dos plantillas lado a lado

| # | Plantilla Vol.I (Grokking 2.0) | Plantilla Vol.II (híbrida) |
|---|---|---|
| 1 | `# Capítulo N — Título evocador` | `# Capítulo N — Título evocador` |
| 2 | `## N.0 La anécdota de la esquina` | `## N.0 La anécdota de la esquina` |
| 3 | `## N.1 ...` (cuerpo técnico libre, 4-12 secciones) | `## N.1 Objetivo` … `## N.10 Cómo lo hace una BBDD real + retos` (10 pasos fijos) |
| 4 | `## Ejercicios resueltos` | `## Ejercicios resueltos` (con niveles) |
| 5 | `## Ejercicios propuestos` | `## Ejercicios propuestos` (con niveles) |
| 6 | `## Lo que te llevas` | `## N.11 Lo que te llevas` |
| 7 | `## Ojo, cuidado con…` | `## N.12 Ojo, cuidado con…` |
| 8 | `## Para profundizar` | `## Para profundizar` |
| 9 | `## Pin de batalla` | `## N.13 Pin de batalla` |
| 10 | `## Si solo lees 30 segundos` | `## N.14 Si solo lees 30 segundos` |
| 11 | `## Una historia pequeña` | `## N.15 Una historia pequeña` |
| 12 | (sólo en Parte VI) `## Diálogo de ascensor / Mini-diálogo` | `## Mini-diálogo: en guardia nocturna` |

**Regla**: en el Vol.II, el orden es **fijo** y la sección técnica va numerada `N.1`–`N.10` con los títulos del brief LiraDB. No se eligen baterías sueltas.

## 0.3. Tabla "qué batería aplica en qué volumen"

| Batería | Vol.I | Vol.II |
|---|:-:|:-:|
| Anécdota de apertura | ✅ siempre | ✅ siempre (N.0) |
| 10 pasos LiraDB | ❌ no aplica | ✅ siempre (N.1–N.10) |
| Lo que te llevas | ✅ siempre | ✅ siempre (N.11) |
| Ojo, cuidado con… | ✅ siempre | ✅ siempre (N.12) |
| Pin de batalla | ✅ siempre | ✅ siempre (N.13) |
| Si solo lees 30 segundos | ✅ siempre | ✅ siempre (N.14) |
| Una historia pequeña | ✅ siempre | ✅ siempre (N.15) |
| Ejercicios resueltos | ✅ siempre | ✅ siempre |
| Ejercicios propuestos | ✅ siempre | ✅ siempre (esencial/intermedio/experto) |
| Para profundizar | ✅ siempre | ✅ siempre |
| Diálogo de ascensor | ⚠️ sólo Parte VI Vol.I | ✅ siempre (mini-diálogo) |

## 0.4. Reglas de transición entre volúmenes

- Cualquier referencia a un concepto del Vol.I desde el Vol.II debe incluir la notación `(Vol. I, cap. N)`.
- El **cap. 32 del Vol.I** (Quantum Computing) cierra el Vol.I invitando al lector a continuar con el Vol.II.
- El **cap. 1 del Vol.II** ("Qué es realmente un grafo") abre citando explícitamente los caps. 1-2 del Vol.I como prerequisito.
- Los caps. 21-32 del Vol.I (Grafos en la Informática Moderna) funcionan como "semilleros" del Vol.II: cada uno termina con una nota al pie apuntando al capítulo del Vol.II que implementa lo que ese cap. introdujo.

## 0.5. Política de versiones Rust y `Cargo.lock`

- Cada capítulo del Vol.II incluye su propio `Cargo.toml` con versiones **pineadas** (sin `^` ni `~`).
- Cada workspace de capítulo incluye `rust-toolchain.toml` con la versión exacta de Rust stable usada para escribirlo.
- El `Cargo.lock` se commitea al repositorio, no se regenera por CI.
- Si una versión de crate queda obsoleta durante la escritura, se documenta en `book-context/CHANGELOG.md` y se abre una incidencia; **no se reescriben caps ya publicados**.

## 0.6. Convención de cross-references

- `(Vol. I, cap. N)` — referencia al Volumen I.
- `(Vol. II, cap. N)` — referencia al Volumen II.
- `(cap. N)` sin prefijo — referencia dentro del mismo Volumen.
- `(LiraDB §N.M)` — referencia a una sección del workspace `liradb-workspace/`.

## 0.7. Glosario de términos estructurales

| Término | Significado |
|---|---|
| **Capítulo** | Unidad principal (~200-700 líneas). Numerado dentro de cada Vol. |
| **Parte** | Agrupación de 5-8 capítulos. Numerada en romanos. |
| **Batería** | Sección recurrente fija. |
| **Reto esencial/intermedio/experto** | Niveles de ejercicios en Vol.II. |
| **Claim** | Afirmación técnica con `claim_id` y `confidence_score`. |
| **Evidence card** | Recorte verificable de fuente, extraído por `source-researcher`. |
| **Code card** | Snippet de código Rust con `Cargo.toml` asociado. |
| **ADR** | Architecture Decision Record (Apéndice D Vol.II). |

*(El Manual de estilo se completará con ejemplos canónicos cuando se hayan publicado los primeros caps. del Vol.II.)*

---

# Epílogo — Ya sabes construir una base de datos

> *Borrador.*

*(Este epílogo se redactará al cierre de la Fase B, cuando todos los caps. estén en estado `DONE`. Incluirá: qué hemos construido, qué queda por hacer, cómo contribuir al proyecto LiraDB, y una carta al lector.)*

---

# Colofón

**Agradecimientos** — *pendiente*.

**Sobre esta edición** — *pendiente*.

**Versión Python** — El Vol.II tendrá una versión paralela en Python (LiraDB-py) en un repositorio hermano, compartiendo estructura y decisiones arquitectónicas.

**Licencia** — CC BY-NC-SA 4.0.

**Atribuciones** — A Semih Salihoğlu y al equipo de Kùzu/Ladybug por los papers seminales sobre GDBMS modernos. La arquitectura conceptual de los caps. 37-40 del Vol.II se inspira en el Kùzu VLDB 2023 paper y en las publicaciones del grupo de Salihoğloo en la Universidad de Waterloo. La reimplementación es clean-room: ningún código de Kùzu/Ladybug ha sido copiado. Texto y código de este libro están bajo CC BY-NC-SA 4.0; los papers referenciados mantienen sus licencias originales.

**Contacto** — *pendiente*.

---

*Fin del esqueleto del Volumen II. El cuerpo se redactará en las Fases B-C del workflow BOOK-WORKFLOW.*