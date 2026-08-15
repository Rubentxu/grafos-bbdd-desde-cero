# CONTRATO DE CAPÍTULO — Vol.II Cap. 14: Cómo almacenar adyacencias (CSR, segmentos)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap14_csr.rs` (28 tests en `tests_csr`).
> Decisiones y bugs reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md` §18.
> Fuentes externas verificadas: Yale Sparse Matrix Package (Eisenstat, Gursky, Schultz,
> Sherman; YALEU/DCS/RR-112 y RR-114, 1977); Wikipedia «Sparse matrix» (CSR en uso desde
> mediados de los 60, primera descripción completa formal en 1967); Kùzu GSM paper
> (Feng, Jin, Chen, Liu, Salihoğlu — CIDR 2023, paper p48); Gupta/Mhedhbi/Salihoglu,
> «Columnar Storage and List-based Processing for GDBMSs» (PVLDB 14(11), 2021);
> Kankanamge et al., «Graphflow: An Active Graph Database» (SIGMOD 2017).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: `Value`/`Node`/`Edge` y `NodeId` (cap. 7); el trait
  `GraphStore` y su `MemoryStore` con listas de adyacencia out/in (cap. 8); little-endian
  explícito (cap. 9); `SlottedPage`/`PAGE_SIZE`/metapágina en página 0 (cap. 11);
  `Pager`/`PageId`/`allocate` y que la página 0 jamás se asigna a datos (cap. 12);
  `BufferPool` con `get_page`/`unpin`/`mark_dirty`/`flush_page`/`metrics` (cap. 13); BFS
  y por qué el patrón de acceso a vecinos lo es todo (Vol. I, cap. 4); lista de adyacencia
  vs matriz de adyacencia como conceptos (Vol. I, cap. 2).
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**:
  (1) «una lista de adyacencia es una lista» — en CSR es un *intervalo* de un array único,
  ni siquiera tiene puntero propio; (2) «la matriz de adyacencia es la representación
  compacta» — es compacta en bits y a la vez inviable (O(V²): 1M nodos = 125 GB a 1 bit);
  (3) «guardar forward y backward es redundancia prescindible» — no: convierte
  `incoming(u)` de O(E) a O(1)+grado; (4) «un grafo vacío no tiene estructura» — falso:
  `offsets` siempre tiene `num_nodes+1` entradas (mínimo `[0]`; bug real del §18: el
  `load` devolvía `vec![]` en vez de `vec![0]`); (5) «self-loops y duplicados son errores
  de datos que el modelo debe rechazar» — son datos legítimos (los admite Kùzu) y CSR los
  absorbe sin caso especial.
- **NO debe saber todavía**: índices hash/B+ (cap. 15), compactación/vacuum de páginas
  huérfanas (cap. 16), atomicidad/WAL para replace crash-safe (cap. 28), compresión de
  columnas y ejecución vectorizada (cap. 38), proyección de grafos para algoritmos
  (cap. 26), CSR+Delta para updates (Apéndice E/paisaje). Se nombran como «luego lo
  verás» y se corta ahí.

## 2. Conceptos (del grafo curricular)

- `present`: CSR (offsets `u64` + targets `NodeId`, `num_nodes+1` y `edge_count`);
  doble índice forward/backward; inmutabilidad y rebuild (`from_edges`/`replace`);
  invariantes (6) verificadas en las tres puertas (build/replace/load); chunk como
  record único en `SlottedPage` (`ChunkHeader` de 9 bytes: kind/index/count);
  `CsrHeader` (24 bytes) en página 1; convención `start_page == 0` ⇒ array vacío;
  límites single-chunk (500 offsets / 1000 targets) y segmentación encadenada futura;
  `CsrError` tipado (`Io`, `InvalidEdge`, `Inconsistent`, `TooLarge`).
- `practice`: `SlottedPage::new/insert/decode` con un solo record (cap. 11);
  `pager_mut().allocate()` y `is_allocated` (cap. 12); `get_page`/`unpin`/`mark_dirty`/
  `flush_page`/`flush` y `metrics()` (cap. 13); encode/decode little-endian (cap. 9).
- `consolidate`: leak-preferido-sobre-corrupción (replace escribe chunks antes que el
  header); validar en la frontera; tipos de anchura fija en formato; «derivar, no llevar
  en la cabeza».
- `out_of_scope` (solo nombrar): índices (cap. 15), vacuum de páginas huérfanas
  (cap. 16), WAL y atomicidad real de replace (cap. 28), compresión/bit-packing de
  columnas (cap. 38), WCOJ sobre adyacencias ordenadas (cap. 39).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué `vecinos(u) = targets[offsets[u]..offsets[u+1])` es
  O(1) para localizar y secuencial para recorrer, y por qué eso (una sola alocación
  contigua) aplasta a `Vec<Vec>` en localidad de caché; (2) calcula el coste en memoria de
  `Vec<Vec>` vs CSR vs matriz para 1M nodos / 4M aristas (≈56-64 MB dispersos en 1M+1
  alocaciones vs 24 MB en 2 alocaciones vs 125 GB a 1 bit); (3) enuncia las 6 invariantes
  de `verify()` y por qué se comprueban en las TRES puertas; (4) dibuja el layout del
  fichero (0 metapágina, 1 `CsrHeader`, 2.. chunks) y explica la convención
  `start_page == 0`; (5) justifica forward+backward (~8 B/arista extra) y replace
  completo (CSR inmutable por diseño) con sus modos de uso correctos (proyección
  analítica) e incorrectos (grafo mutable OLTP).
- **Skills**: (1) construir un `Csr` con `from_edges`, consultarlo con
  `neighbors_out/in`, `degree_out/in` y `edge_count`, y persistirlo/recargarlo con
  `PersistentCsr::create/replace/load/open`; (2) trazar a mano los cuatro arrays de un
  grafo pequeño (incluidos self-loops y duplicados) y validarlos contra los tests;
  (3) diagnosticar un `CsrError::Inconsistent` diciendo qué invariante se rompió y en
  qué puerta se detectó.
- **Wisdom**: (1) decide cuándo CSR-rebuild es la estructura correcta (proyección
  read-mostly, cargas por lotes, algoritmos de la Parte V) y cuándo no (mutación
  frecuente: hace falta CSR+delta o listas con huecos); (2) reconoce el patrón
  «escribir datos antes que metadatos» (chunks antes que header) como la misma regla
  leak-vs-corrupción de los caps. 12-13, ahora a nivel de estructura.

## 4. Modelo mental

- **El índice temático de un libro**: el libro completo (targets) es el texto corrido; el
  índice (offsets) tiene una entrada por término en orden fijo y dice «Arquitectura:
  págs. 23-41» — no contiene el contenido, dice *dónde mirar*, y el rango es contiguo.
  Para consultar en ambas direcciones necesitas DOS índices: temas→páginas (forward) y
  páginas→temas (backward). Un término sin entradas es «23-23»: rango vacío, no ausencia
  de estructura.
- **Diagramas ASCII**: (a) los cuatro arrays de `[(0,1),(0,2),(1,2)]` con los intervalos
  señalados; (b) layout del fichero tras `replace()` (0 meta, 1 header, 2..=5 chunks);
  (c) layout del chunk (9 bytes de cabecera + payload LE).
- **Momento ¡ajá!**: la «lista» de adyacencia de `u` ni siquiera existe como objeto: es
  un intervalo `[offsets[u], offsets[u+1])` de un array único. Localizarla es una resta;
  recorrerla es escanear memoria contigua. Eso es lo que una base de datos de grafos
  compra con CSR — y lo que Kùzu (CIDR 2023) usa como índice de join fundamental.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap14_csr.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | `offsets: Vec<u64>` + `targets: Vec<NodeId>` | Una sola alocación contigua por dirección: localizar O(1), recorrer secuencial, prefetch y SIMD posibles; 24 MB para 1M nodos/4M aristas | `Vec<Vec<u64>>`: 1M+1 alocaciones dispersas, ≈56-64 MB y ≥1M cache misses de RAM (~100 ns c/u) al recorrer todo | BFS/page-rank se vuelven cuellos de botella de memoria, no de CPU | Origen: Yale Sparse Matrix Package (RR-112/114, 1977); Wikipedia «Sparse matrix»; Vol. I cap. 2 |
| 2 | Forward Y backward simultáneos | `neighbors_in(u)` sin escanear E aristas: O(1)+grado entrante | Sólo forward + escaneo: O(E) por consulta entrante; reconstruir backward on-demand: O(E) cada vez | Toda consulta inversa (¿quién me apunta?) degrada a escaneo global | `neighbors_in` (código); decisión heredada de Kùzu (brief §7; CIDR 2023); MIGRATION §18.2 |
| 3 | Y no matriz de adyacencia | O(V+E) vs O(V²): 1M nodos a 1 bit = 125 GB; a u32 = 4 TB; grafo real E≈4M (densidad 4·10⁻⁶) → 24 MB | Matriz: sólo competitiva si E ≈ V² (denso); además no soporta multigrafo (celda única) | Memoria agotada mucho antes de tener un grafo «grande» | Vol. I cap. 2; MIGRATION §18-lección 2 |
| 4 | `replace` completo, sin updates incrementales | CSR es inmutable por diseño: insertar en medio de targets desplaza O(E) entradas; rebuild O(V+E) por lotes es correcto para proyecciones read-mostly (Parte V) | Updates in situ: O(E) por arista o gaps intermedios (complejidad de Neo4j/Kùzu: regiones densas + overflow) | Un `insert` «barato» degrada a O(E) y rompe la localidad que justifica CSR | `replace()` (doc comment); MIGRATION §18.5; cap. 26 (proyección) |
| 5 | Chunks como record único en `SlottedPage` | Reutiliza TODO el motor (caps. 11-13): sin formato nuevo, con pin/unpin/flush y métricas; el chunk es auto-descriptivo (kind+index+count) | Formato de página propio: duplicaría validación, pager y pool; bindeo a mano de arrays crudos sin cabecera | El CSR sería una isla sin caché ni durabilidad compartidas | `ChunkKind`/`ChunkHeader`/`write_chunks`; MIGRATION §18.3 |
| 6 | `verify()` en las TRES puertas (build/replace/load) | «Nada corrupto llega a disco»: 6 invariantes (longitudes, monotonicidad, totales, targets en rango, forward==backward) cuestan O(V+E) frente a un grafo inconsistente que responde MAL en silencio | Verificar sólo al construir: un bit girado en disco entregaría vecinos equivocados con aspecto de válidos | Offsets que mienten dentro de rango = respuestas incorrectas silenciosas (no crash) | `verify()`; tests `csr_verify_rejects_*`; MIGRATION §18.7, §18-lección 4 |
| 7 | Self-loops y duplicados admitidos | Son datos legítimos (Kùzu los soporta); CSR los absorbe sin caso especial: un self-loop es un target más en el segmento propio; un duplicado, una entrada repetida | Rechazarlos: obliga al usuario a mentir sobre sus datos (limpiar antes de cargar) y complica `from_edges` con política de deduplicación oculta | «Error de datos» en grafos reales (follows, interacciones) que sí los contienen | Tests `csr_from_edges_with_self_loops`, `csr_from_edges_duplicates`; MIGRATION §18.8 |
| 8 | `CsrHeader` en página 1, no en la metapágina | La metapágina (cap. 11) es el catálogo genérico del fichero; el header CSR es específico del módulo. Página 1 = primera data page, convenio fijo | Reusar metapágina: acopla el catálogo del fichero a un módulo concreto y la satura | Cada módulo nuevo pisaría el catálogo global | `CsrHeader` (doc comment); MIGRATION §18.4 |
| 9 | Convención `start_page == 0` ⇒ array vacío | Distingue «vacío intencional» de «corrupto» sin gastar páginas; funciona porque la página 0 es SIEMPRE la metapágina (cap. 12) y jamás sale de `allocate` | Página dedicada al vacío: desperdicio y caso especial en todos los lectores | Confundir vacío con corrupción (o al revés) en cada load | `read_array_u64` (comentario); `load` cortocircuita `num_nodes==0`; MIGRATION §18-lección 1 |
| 10 | Límites 500 offsets / 1000 targets por página single-chunk | Calibrados al espacio útil de una página (9+n·w+4+10 ≤ 4096; máximos reales 509/1018, redondeo conservador). Más allá: `load` falla alto (`Inconsistent`/`TooLarge`), nunca en silencio; la segmentación encadenada (el `chunk_index` ya viaja en el header) es la evolución anunciada | Encadenar ya: nextPage por chunk, lecturas multi-página — complejidad que este capítulo no necesita | Falso infinito: grafos medianos (>500 nodos) que aparentan persistir bien | `OFFSETS_CHUNK_MAX`/`TARGETS_CHUNK_MAX` (comentarios); MIGRATION §18.3, §18-lección 2 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: `Vec<Vec<NodeId>>` (la de `MemoryStore`, cap. 8) y, para persistir, volcar
  cada Vec con su longitud — o peor, una página por nodo.
- **Qué la rompe**: (a) localidad: 1M nodos = 1M+1 alocaciones dispersas, ≥1M misses de
  RAM al recorrer (números en §5.1); (b) páginas por nodo: 1M páginas de 4 KB = 4 GB para
  16 MB de datos; (c) sin dirección inversa: `incoming(u)` escanea todo; (d) sin
  invariantes: nadie detecta offsets corruptos tras un reopen.
- **Evolución visible**: `from_edges` usa `Vec<Vec>` TEMPORALMENTE para contar grados y
  luego aplana a los 4 arrays (el coste dinámico se paga una vez en el rebuild);
  `PersistentCsr` monta esos arrays como chunks sobre el pool. El lector ve que la
  solución ingenua no se tirá: se convierte en una *fase de construcción* del CSR.

## 7. Prueba de fuego

- **Tests reales** (se citan, no se duplican): `csr_from_edges_no_self_loops`,
  `csr_from_edges_with_self_loops`, `csr_from_edges_duplicates`,
  `csr_verify_rejects_bad_offsets`, `csr_verify_rejects_total_mismatch`,
  `csr_verify_rejects_out_of_range_target`, `persistent_csr_replace_then_load`,
  `persistent_csr_replace_keeps_invariants` (los 4 arrays trazados a mano),
  `persistent_csr_replace_rejects_invalid`, `persistent_csr_disk_roundtrip_via_filepager`
  (end-to-end: create→replace→cerrar→open→load),
  `persistent_csr_disk_roundtrip_two_replaces`, `persistent_csr_open_without_header_fails`,
  `persistent_csr_pool_metrics_after_reload` (page_reads/misses ≥ 1),
  `csr_verify_offsets_consistent_with_edge_count` (LCG: Σdeg_out = Σdeg_in = edge_count).
- **Síntoma si el lector se salta el capítulo**: consultas inversas O(E) «que funcionan»
  en tests pequeños; BFS que no escala (tiempo dominado por misses de memoria, no por
  aristas); adyacencia persistida sin invariantes que devuelve vecinos equivocados tras
  un reopen — corrupción con aspecto de grafo válido.

## 8. Trampas y errores comunes

1. **Confundir chunk con página o con record**: el chunk es el CONTENIDO del record
   único que vive dentro de la `SlottedPage` de una página. Tres niveles: página >>
   record >> chunk.
2. **Creer que `verify()` es paranoia**: sin ella, offsets corruptos PERO en rango
   devuelven vecinos incorrectos en silencio; el guard defensivo de `neighbors_out`
   (`start > end` → vacío) sólo tapa los casos groseros.
3. **Esperar updates incrementales**: añadir una arista a un CSR «in situ» es O(E);
   quien necesita mutación frecuente necesita otra estructura (CSR+delta, listas con
   huecos), no más ingenio.
- **Precisión de lenguaje (glosario)**: *offsets* (array de inicios acumulados, u64) vs
  *targets* (array plano de vecinos); *chunk* (porción de un array en una página) vs
  *segmento* (array completo encadenado en múltiples chunks — evolución futura);
  *forward/backward* (dirección de la arista que indexan); *replace* (rebuild completo)
  vs *update* (mutación in situ, no soportada); *grado* (longitud del intervalo).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial, retrieval de caps. 11-12)**: sin mirar atrás, explicar
  por qué la convención `start_page == 0` ⇒ «array vacío» es segura, qué tiene que
  cumplir el pager para que lo sea, y qué devolvería `load()` si el header apuntara a la
  página 0 con `edge_count = 3`. Verificación: `persistent_csr_create_load_empty_roundtrip`
  + razonamiento del error `Inconsistent("empty csr with edge_count > 0")`. Pistas:
  (1) ¿quién posee la página 0?, (2) ¿puede `allocate` devolverla?, (3) ¿vacío intencional
  vs corrupto? Criterio: distingue convenio de formato de bug de datos.
- **analizar (intermedio, spacing con cap. 13)**: predecir cuántas `page_reads` y
  `buffer_misses` produce `load()` de un CSR con pool capacidad 8 y por qué el header
  cuenta como lectura. Verificación: `persistent_csr_pool_metrics_after_reload` y una
  variante propia con `pool().metrics()`. Pistas: (1) ¿cuántas páginas toca `load()`?,
  (2) ¿qué hace la primera `get_page` si el pool está vacío?, (3) ¿cuándo NO habría miss?
- **crear (experto, la evolución anunciada)**: implementar la segmentación encadenada —
  extender `ChunkHeader` con `next_page` y hacer que `read_array_u64` siga la cadena;
  que un grafo de 600 nodos persista y recargue idéntico. Pistas: (1) el `chunk_index`
  ya viaja en disco, (2) ¿qué valor de `next_page` significa «fin de cadena» (recuerda
  la convención del 0)?, (3) ¿qué invariante adicional exige la cadena (longitudes
  acumuladas)? Criterio: el límite 500 deja de ser techo; `verify()` sigue pasando.

## 10. Preguntas abiertas (gancho al capítulo 15)

1. Con CSR, encontrar los vecinos de un nodo con ID conocido es O(1). Pero ¿cómo
   encuentras el nodo cuyo `name = "Ana"` sin escanear todos los nodos?
2. ¿Pueden los índices vivir también como páginas sobre el pool, o necesitan su propio
   formato? ¿Qué comparten con el `CsrHeader` (catálogo en página fija)?
3. `replace` deja huérfanas las páginas del CSR anterior: ¿quién las reclama y cuándo?
   (Respuesta parcial: cap. 16, compactación.)
- **Términos nuevos de glosario**: CSR, offsets, targets, forward/backward, chunk,
  segmento, rebuild/replace, grado entrante/saliente, proyección de grafo, multigrafo,
  self-loop, single-chunk, localidad espacial.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio esencial obliga a recordar (sin re-leer) por qué
  la página 0 nunca contiene datos y quién lo garantiza — el enunciado no nombra
  `create()` ni la metapágina; el lector debe reconstruir la cadena cap. 12 → cap. 14.
- **Spacing**: el intermedio re-ejercita `metrics()`/misses del cap. 13 a través del CSR;
  el esencial re-ejercita metapágina/página 0 de los caps. 11-12; el capítulo referencía
  `MemoryStore` (cap. 8) y little-endian (cap. 9) al justificar `from_edges` y los chunks.
- **Interleaving**: el experto mezcla formato en disco (9 bytes de `ChunkHeader`),
  convención del 0 (cap. 14), paging del cap. 12 e invariantes del CSR; el intermedio
  cruza estructura de datos con métricas de caché.
- **Dificultad asimétrica**: una idea nueva por sección (intervalo → dos direcciones →
  inmutabilidad → disco en chunks); los ejercicios exigen predicción y traza a mano.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` ejecuta los 28 tests de
  `tests_csr`; cada ejercicio nombra el test que lo verifica.
- **Citas**: Yale RR-112/114 (1977) y Wikipedia «Sparse matrix» (historia de CSR); Kùzu
  GSM (CIDR 2023 p48) y Gupta/Mhedhbi/Salihoglu (PVLDB 14(11) 2021) para CSR columnar
  con doble dirección; Graphflow (SIGMOD 2017) para CSR en memoria con listas ordenadas;
  Neo4j (registros de tamaño fijo + cadenas de relaciones) como contraejemplo;
  Petrov «Database Internals» y Kleppmann «DDIA» cap. 3 (jerarquía de memoria).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (10 en la tabla §5).
- [x] Escenario de fallo visible: BFS `Vec<Vec>` vs CSR con números de caché; CSR con offsets que mienten (respuestas erróneas silenciosas vs `verify`).
- [x] Código ejecutable en workspace (28 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (las 5 del §1, incluida «vacío sin estructura» con el bug real del §18).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (página 0 / metapágina desde memoria) y ≥1 de spacing (métricas del pool, cap. 13).
- [x] Responde las preguntas críticas: CSR vs Vec<Vec>, forward+backward, no matriz, replace vs update, chunks sobre caps. 11-13, verify en tres puertas, self-loops/multigrafo, start_page==0, límites y segmentación.
