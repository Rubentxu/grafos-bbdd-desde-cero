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

