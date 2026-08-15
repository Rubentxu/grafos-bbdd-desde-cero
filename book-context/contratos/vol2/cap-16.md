# CONTRATO DE CAPÍTULO — Vol.II Cap. 16: Compactación y mantenimiento (`liradb inspect|check|compact`)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap16_mantenimiento.rs` (1.144 líneas,
> 25 tests en `tests_maintenance`). Decisiones y bugs reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §20. Este capítulo CIERRA la
> Parte III: incluye repaso-retrieval de las piezas 11-15 y la sección comparativa
> LSM-trees (refuerzo ADR-005, línea 27 de `manuscrito/vol2/tabla-de-contenidos.md`).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: slotted pages, `PageHeader` (10 B: magic/page_type/
  page_id/num_records/free_space) y metapágina (cap. 11); trait `Pager`, `PageId` =
  offset físico (`id × PAGE_SIZE`), free list en memoria que NO persiste, `free()` no
  trunca el fichero (cap. 12); `BufferPool` con pin/unpin y `pager_mut()` (cap. 13);
  CSR con `CsrHeader` de 4 punteros `PageId` (cap. 14); `HashIndex` con catálogo en la
  página 2, `bucket_starts` y buckets encadenados por `next_page`; `BPlusTree` con raíz
  fija en página 2 (cap. 15); errores tipados con `From` (todos los caps).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «compact libera
  espacio del fichero» — falso: es repack in-place, el fichero NO encoge (el vacuum
  queda como deuda); (2) «el `free_space` de la cabecera es la verdad» — falso: es un
  metadato-cache que se desincroniza tras un crash a mitad de update; la verdad es
  derivarla del contenido (`SlottedPage::free_space()`); (3) «check debería arreglar
  lo que encuentra» — mentalidad `fsck -y`: un check que repara esconde corrupción
  real (magic corrupto = fallo de disco, no metadato viejo); (4) «el mantenimiento
  pasa por el buffer pool como todo» — falso: es offline, leer por el pool contaminaría
  la caché.
- **NO debe saber todavía**: compactación transaccional con snapshot consistente
  (cap. 30), recuperación/WAL (cap. 28-29, donde vivirá parte del vacuum), índices
  online, scheduling de autovacuum, mmap, LSM internals profundos (la sección
  comparativa nombra, no implementa). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: mantenimiento como cuarta edad del dato (inspeccionar / verificar /
  reparar); `StorageStats` con `fragmentation_ratio()` y `utilization()` como MEDIDAS
  DE NEGOCIO; `IssueKind` (BadMagic/PageIdMismatch/FreeSpaceMismatch/RecordTruncated/
  Undecodable) y `CheckReport` read-only; repack in-place (`repack_page`) vs vacuum;
  `CompactReport` con `stats_before`/`stats_after`; tolerante vs exhaustivo;
  mantenimiento offline (leer sin cachear); LSM-tree (memtable + SSTables inmutables +
  compaction leveled/tiered), write/read amplification; online vs offline.
- `practice`: decodificar `PageHeader`/`SlottedPage`/`MetaPage` (cap. 11); `is_allocated`,
  free list LIFO y leak tras reopen (cap. 12); `pager_mut()` (cap. 13); invariantes de
  punteros CSR/índices (caps. 14-15); errores tipados con `Display`/`Error`/`From`.
- `consolidate`: «derivar, no llevar en la cabeza»; little-endian en disco; leak vs
  corrupción; formato autodescriptivo.
- `out_of_scope` (solo nombrar): vacuum/truncate real (caps. 29/36), snapshot para
  compactar en caliente (cap. 30), WAL (cap. 28), export/import como alternativa
  (cap. 32), observabilidad continua (cap. 35).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué repack in-place y no mover páginas, cuantificando
  los punteros que rompería (metapágina `root_page` + 4 del `CsrHeader` + `bucket_starts`
  del hash + hijos de nodos internos del B+Tree + cadenas `next_page`); (2) distingue
  `free_space` declarado (header, puede mentir) de real (derivado) y dice quién los
  reconcilia; (3) enuncia las 4 invariantes de `check` y qué `IssueKind` cubre cada una;
  (4) dice qué mide `fragmentation_ratio()` y qué decisión de negocio dispara; (5)
  describe un LSM-tree y por qué es write-optimized, con su precio en lecturas y CPU.
- **Skills**: (1) ejecutar el ciclo operacional inspect → check → compact y leer los
  tres informes; (2) escribir tests de corrupción quirúrgica (machacar bytes 8..10 para
  `FreeSpaceMismatch`, bytes 0-1 para `BadMagic`, bytes 2..6 para `PageIdMismatch`).
- **Wisdom**: (1) decide cuándo NO reparar automáticamente (corrupción estructural =
  decisión humana tras leer `check`); (2) decide cuándo un sistema pide LSM y cuándo
  slotted pages + repack (carga de escritura vs simplicidad embebida).

## 4. Modelo mental

- **El almacén del cap. 12 tras una mudanza**: estantes numerados = direcciones
  PÚBLICAS (PageId = offset físico; el CSR, los buckets y el B+Tree las tienen
  apuntadas); el interior de cada caja = PRIVADO → se puede reorganizar (repack) sin
  avisar a nadie. `check` es el auditor con linterna que NO toca nada; `inspect` es el
  inventario que cuenta lo que hay de verdad (no lo que dicen las etiquetas);
  `compact` es la cuadrilla que reordena DENTRO de las cajas y jamás cambia una caja
  de estante.
- **Diagramas ASCII**: (a) página antes/después de repack (huecos internos vs records
  consecutivos + padding a cero); (b) mapa de «quién apunta a la página 7» (coste del
  vacuum); (c) pipeline del motor completo 11→16 (cierre de Parte III); (d) LSM-tree
  por niveles con flechas de compaction.
- **Momento ¡ajá!**: «la dirección del estante es pública; el interior de la caja es
  privado». Esa frontera es EXACTAMENTE la que separa lo que compact puede tocar de
  lo que no, y nace de la decisión `PageId = offset físico` del cap. 12.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap16_mantenimiento.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Tres operaciones separadas (inspect/check/compact) | Son tres preguntas distintas: ¿cuánto tengo? / ¿está sano? / ¿puedes ordenarlo? El operador las encadena en el ciclo diagnóstico→decisión→acción | Una sola `maintain()` que haga todo: esconde el informe tras la reparación y no deja decidir | Reparaciones no deseadas sobre datos que había que inspeccionar | MIGRATION-PATTERN §20.1; PostgreSQL separa `VACUUM`/`VACUUM ANALYZE` del diagnóstico (`pg_stat_*`) |
| 2 | `inspect` lee el CONTENIDO real de cada página | El `free_space` del header es un cache que puede desincronizarse tras un crash a mitad de update; contar etiquetas es contar mentiras potenciales | Sumar headers: O(1) por página pero hereda el error que precisamente queremos medir | `inspect` «confirma» una salud que no existe | Lección §20-2 («`free_space()` es la fuente de verdad, no el header»); tests `inspect_counts_records_and_pages`, `inspect_is_tolerant_to_corrupt_data_page` |
| 3 | `check` read-only + exhaustivo, con `IssueKind` tipado | Un check que repara esconde corrupción real: `BadMagic` = fallo de disco, no `free_space` viejo. Reportar TODO permite al operador decidir | `fsck -y` (repara al encontrar): normaliza lo patológico y borra evidencia | Corrupción silenciosamente «arreglada» = datos perdidos con apariencia de éxito | MIGRATION-PATTERN §20.5 y lección §20-4; tests `check_detects_*` (4 invariantes) |
| 4 | `repack_page` in-place, NO mueve páginas | `PageId` = offset físico (cap. 12): mover la página 7 a la 2 desplaza a TODAS las superiores e invalida todos los punteros: `MetaPage.root_page` (1) + `CsrHeader` (4) + `bucket_starts` (B) + hijos de nodos internos del B+Tree (~I×F) + cadenas `next_page` | Vacuum/defrag clásico: exige reescritura transaccional de todos los punteros con snapshot consistente (cap. 30) — hoy corrompería CSR (14) e índices (15) | Punteros apuntando a páginas equivocadas = corrupción masiva; síntoma: `PageIdMismatch` | MIGRATION-PATTERN §20.3 y lección §20-1; `CsrHeader` (cap14_csr.rs:151), `next_page` (cap15_indices.rs:293), test `check_detects_page_id_mismatch` |
| 5 | `repack_page` rehúsa la metapágina (id 0) | Su layout es fijo (header + 12 B de info): no tiene records ni huecos que reconciliar; «repackearla» no significa nada | Repack genérico de cualquier página: código muerto + riesgo de pisar el catálogo del fichero | Metapágina corrompida = arranque en frío roto | `repack_page` devuelve `BadPageType { expected: 0xDA, got: 0xFE }`; test `repack_page_rechaza_meta_page` |
| 6 | `inspect`/`compact` tolerantes, `check` exhaustivo | `inspect` cuenta (una página corrupta se cuenta como data sin records); `compact` salta lo corrupto (`pages_skipped`) SIN tocarlo. La corrección de corrupción estructural es decisión humana tras leer `check` | Abortar al primer error: un barrido de estadísticas que revienta en la página 401 de 10.000 no sirve para nada | «O todo o nada» en mantenimiento = herramientas que nadie puede usar con ficheros dañados | MIGRATION-PATTERN §20.5; tests `inspect_is_tolerant_to_corrupt_data_page`, `compact_salta_paginas_corruptas` |
| 7 | Lectura vía `pager_mut().read()` con buffer reutilizable, sin `get_page` | El mantenimiento es offline y barre el fichero entero: cada página se usa UNA vez. Pasar por el pool llenaría los frames de páginas frías y expulsaría las calientes (anti-patrón de locality, cap. 13) | `pool.get_page()` + unpin: contamina la caché, bookkeeping de pin innecesario, cero beneficio | Tras un `inspect`, el pool queda «enfriado» y la hot path siguiente sufre | MIGRATION-PATTERN §20.6 y lección §20-3; CMU 15-445 (buffer pool, scan-resistance) |
| 8 | `bytes_reclaimed = |free_before − free_after|` (honesto) | Mide la corrección del METADATO y la limpieza de bytes basura, no movimiento de datos: repack no elimina records ni cruza páginas. Nombrarlo «recuperado» sin matiz sería marketing | Inflar la métrica contando padding o prefixes: cifras bonitas que no corresponden a espacio utilizable | El operador espera que el fichero encoche y no entiende por qué no | `RepackResult` (doc-comment del campo); bugs de tests §20 (contar length-prefixes) |
| 9 | `MaintenanceError` tipado con `From<BufferPoolError>` y `From<PagerError>` | Paralelo a `IndexError` (cap. 15): distinguir fallo del entorno (`Io`) de formato no esperado (`BadPageType`, `DecodeFailed`, `PageNotAllocated`); `?` ergonómico | `io::Error` a secas: imposible `match` fiable; strings frágiles | El CLI no podría mostrar «página 7: decode failed» con precisión | Tests `maintenance_error_display_y_from_pager`, `maintenance_error_display_variantes` |
| 10 | `compact` escala TODO error salvo `DecodeFailed` | Un error de E/S o de asignación a mitad de repack masivo es estructural: ignorarlo sería mentir en el `CompactReport`. Solo la página corrupta (datum) se salta | Tragar todos los errores: «compact exitoso» que no compactó nada | Falsos informes verdes | `compact` (match con comentario in-code); test `compact_salta_paginas_corruptas` |
| 11 | Vacuum/truncate declarado deuda (caps. 29/36) | Reducir el fichero exige reescribir todos los punteros bajo un snapshot consistente — maquinaria que aún no existe. Declararlo es más honesto que un vacuum a medias que corrompería punteros | Vacuum a medias hoy: el escenario de fallo del §4 de este contrato | Corrupción masiva con «éxito» | MIGRATION-PATTERN §20.3, lección §20-1; cap. 12: `free()` no trunca |
| 12 | Sección comparativa LSM (ADR-005) | El lector que mire alrededor verá RocksDB/LevelDB/Cassandra por todas partes y pensará que elegimos mal. Contextualizar slotted pages + repack frente a LSM cierra la decisión de diseño con honestidad (quién SÍ elige LSM para grafos/documentos) | Omitirla: hueco de criterio que el lector rellenará con hype | «RocksDB lo hace mejor» sin entender el precio (read amplification, compaction comiendo CPU) | TOC línea 27; O'Neil 1996 (LSM paper); Petrov «Database Internals» Part II; DDIA cap. 3 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: `ls -lh graph.liradb` + un bucle que suma los `free_space` de las
  cabeceras, y si algo suena raro, «reescribir el fichero entero de nuevo» (o peor:
  «reparar» cada magic raro sobre la marcha).
- **Qué la rompe**: (a) el header MIENTE tras un crash (el nº 1 de lo que había que
  medir); (b) un tamaño total no dice nada accionable (¿fragmentación? ¿corrupción?
  ¿páginas libres?); (c) reescribir en caliente sin distinguir dato sano de corrupto
  destruye evidencia; (d) mover páginas para «ordenar» rompe todos los punteros.
- **Evolución visible en el capítulo**: tres funciones con contratos distintos —
  `inspect` deriva del contenido y tolera, `check` verifica invariantes y reporta
  TODO sin tocar, `compact` repackea in-place página a página (unidad atómica
  `repack_page`) y salta lo no decodificable.

## 7. Prueba de fuego

- **Tests reales del módulo** (se citan, no se duplican): `stats_ratios_basic`,
  `inspect_counts_records_and_pages`, `inspect_counts_free_pages`,
  `inspect_is_tolerant_to_corrupt_data_page`, `check_clean_passer_ok`,
  `check_detects_bad_magic`, `check_detects_page_id_mismatch`,
  `check_detects_free_space_mismatch`, `check_skips_free_pages`,
  `repack_page_idempotente_on_clean_page`, `repack_page_corrije_free_space_corrupto`,
  `repack_page_rechaza_meta_page`/`_no_asignada`/`_corrupta`,
  `compact_repackea_todas_las_data_pages`, `compact_corrige_free_space_y_mejora_stats`,
  `compact_salta_paginas_corruptas`, `inspect_y_check_sobre_filepager_tras_reopen`,
  `repack_persiste_free_space_corregido_a_disco`, `compact_sin_data_pages_no_op`.
- **Síntoma si el lector se salta el capítulo**: ficheros que solo crecen (rebuilds de
  CSR/índices que dejan páginas huérfanas), `free_space` desincronizado que nadie
  detecta, y corrupción de disco descubierta… por un panic en producción en vez de por
  un informe de `check`.

## 8. Trampas y errores comunes

1. **Esperar que `compact` encoja el fichero**: repack in-place; `free()` ya no
   truncaba (cap. 12) y `compact` tampoco. El vacuum es deuda declarada.
2. **Contar length-prefixes al medir bytes usados**: `SlottedPage::free_space()` cuenta
   `PAGE_SIZE − header − Σ record.len()` (sin prefijos); dos tests reales nacieron
   erróneos por esto (bug documentado §20).
3. **Olvidar la MetaPage al crear un `FilePager`**: `create` deja la página 0 a ceros
   y `check` reporta `BadMagic` en la página 0 (bug real; helper `filepool_with_meta`).
4. **Mezclar `u16`/`usize`/`u32` en repack**: `header.free_space` es u16,
   `free_space()` devuelve usize, `RepackResult` u32 (bug de compilación real §20).
- **Precisión de lenguaje (glosario)**: *free_space declarado* (header) vs *real*
  (derivado); *página libre* (en free list) vs *huérfana* (asignada, sin dueño útil,
  p.ej. tras reopen); *repack* (dentro de la página) vs *vacuum* (encoger fichero) vs
  *defrag* (mover páginas); *online* vs *offline*; *write/read amplification*;
  *memtable*, *SSTable*, *compaction*, *tombstone*.

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: sin ejecutar nada, predecir el `CheckReport`
  exacto tras dos cirugías: bytes 8..10 de la página 1 ← 1234, y bytes 0-1 de la
  página 2 ← 0x11 (qué `IssueKind`, con qué campos, y en qué orden). Verificación:
  `check_detects_free_space_mismatch` + `check_detects_bad_magic`. Pistas: (1) ¿dónde
  vive `free_space` en el header del cap. 11?, (2) ¿qué espera `check` como magic en
  una página ≠ 0?, (3) ¿cuenta `pages_checked` las libres? Criterio: acertar la
  variante Y los campos (declared/actual, expected/got).
- **analizar (intermedio)**: implementar `doctor()` que llame a `check` y solo
  repackee las páginas con `FreeSpaceMismatch`; argumentar por qué las `BadMagic`
  quedan fuera (y qué hace `compact` con ellas). Verificación: patrón de
  `compact_corrige_free_space_y_mejora_stats`. Pistas: (1) ¿qué variante devuelve
  `repack_page` ante corrupción?, (2) ¿quién decide sobre un fallo de disco?, (3)
  ¿qué pasaría si el doctor «regenerara» la página? Criterio: el doctor NUNCA escribe
  en una página que no decodifica.
- **crear (experto — cierre de Parte III, retrieval puro)**: «la mudanza completa»:
  SIN mirar los caps. 11-15, listar desde memoria todas las estructuras que guardan
  un `PageId` y estimar cuántos punteros hay que reescribir para mover la página 7 a
  la 2 en un fichero con B=64 buckets y un B+Tree de 3 nodos internos (fanout ~64).
  Diseñar en papel el orden de reescritura con snapshot. Verificación contraste:
  `check_detects_page_id_mismatch` (un `header_says ≠ actual` es el síntoma exacto de
  una página movida sin reescribir punteros) + lectura de `CsrHeader`/`bucket_starts`/
  `next_page`. Pistas: (1) página 0, (2) ¿quién apunta a los buckets?, (3) ¿quién
  apunta a las hojas del árbol? Criterio: ≥5 familias de punteros + orden seguro
  (snapshot → reescribir hoja→raíz → swap atómico → truncate).

## 10. Preguntas abiertas (gancho al capítulo 17 — abre la Parte IV)

1. Ya sabemos guardar, indexar y mantener; ¿cómo se PREGUNTA a un grafo sin compilar
   Rust contra el `GraphStore`? (nace LiraQL.)
2. ¿Qué forma tiene una consulta: texto → tokens → árbol? ¿Quién detecta el error
   y a qué byte apunta?
3. ¿Podrá el optimizador (cap. 21) usar lo que `inspect` cuenta (`total_records`) para
   estimar cardinalidades? (Sí: siembra de estadísticas.)
- **Términos nuevos de glosario**: fragmentation_ratio, utilización, invariante
  estructural, repack in-place, vacuum, compaction, memtable, SSTable, write/read
  amplification, tombstone, scan-resistance, mantenimiento offline.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio experto pide reconstruir DESDE LA MEMORIA el
  mapa de punteros `PageId` de los caps. 11-15 (nada en el enunciado revela las
  estructuras); el esencial obliga a recordar el layout del `PageHeader` para elegir
  qué bytes machacar.
- **Spacing**: esencial → header cap. 11 (offsets de bytes); intermedio → errores y
  tolerancia (cap. 15 `IndexError`); experto → CSR cap. 14, índices cap. 15, free list
  cap. 12; la sección «motor completo» re-ejercita el pipeline 11→16 completo.
- **Interleaving**: el experto mezcla formato binario, índices, aritmética de punteros
  y ordenación transaccional; el intermedio cruza diagnóstico con ética de reparación.
- **Dificultad asimétrica**: cada sección introduce UNA idea nueva (medir / verificar /
  reparar / comparar con LSM); los ejercicios exigen recuperación y estimación.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` (25 tests citados por
  nombre).
- **Citas**: PostgreSQL release notes 8.1.0 y «Routine Vacuuming»; sqlite.org
  (lang_vacuum, fileformat2/auto_vacuum ptrmap); LevelDB doc/impl.html (minor/major
  compaction); RocksDB wiki; O'Neil 1996 (LSM); Petrov «Database Internals» Part II;
  Kleppmann «DDIA» cap. 3; CMU 15-445.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: fichero que crece con `free_space` mentiroso + `BadMagic` que compact no toca (§6-§7 del capítulo).
- [x] Código ejecutable en workspace (25 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (compact ≠ vacuum; header ≠ verdad; check ≠ fsck -y; offline ≠ pool).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (mapa de punteros desde memoria) y ≥1 de spacing (header cap. 11, free list cap. 12, índices caps. 14-15).
- [x] Responde las preguntas críticas: repack vs vacuum (cuantificado), check read-only vs compact reparador, tolerante vs exhaustivo, pager_mut vs pool, qué mide fragmentation_ratio, dónde vive el vacuum pendiente, por qué no LSM (ADR-005).
- [x] Sección LSM-trees comparados incluida (TOC línea 27) con sistemas reales que SÍ eligen LSM.
- [x] Anécdota verificada: autovacuum integrado en PostgreSQL 8.1 (8-nov-2005), release notes oficiales.
