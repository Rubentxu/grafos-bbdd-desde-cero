# CONTRATO DE CAPÍTULO — Vol.II Cap. 15: Índices para encontrar datos (hash + B+ tree)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap15_indices.rs` (~650 líneas de
> implementación + 27 tests en `tests_index`). Decisiones y bugs reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §19. Refuerzo que motiva
> el apéndice del capítulo: ADR-005 (B+ tree multinivel y splits).
> Línea de la tabla de contenidos: «Índices para encontrar datos (hash + B+ tree;
> apéndice del capítulo: B+ tree multinivel y splits)». Pregunta crítica del
> CORPUS (`vol-II-cap-15`): **«¿Cuándo hash index y cuándo B+ tree?»** — el
> capítulo entero es la respuesta.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: encode little-endian de enteros y por qué el formato
  en disco debe ser independiente de la máquina (cap. 9); `SlottedPage` con records
  de prefijo de longitud y `PageHeader` de 10 bytes (cap. 11); el trait `Pager` —
  `allocate`, `is_allocated`, `num_pages` — y que la página 0 es la metapágina
  (cap. 12); `BufferPool` con `get_page`/`unpin`/`mark_dirty`/`flush_page` y la
  disciplina de pin (cap. 13); el CSR del cap. 14 y por qué sus offsets responden
  preguntas **topológicas** (vecinos) y no **por propiedad** (valor de una clave).
- **Cree saber pero es vago/erróneo (misconceptions centrales)**:
  1. «Un índice es un HashMap persistido». Falso tres veces: el hash solo responde
     igualdad (destruye el orden), `std::collections::hash_map::RandomState` siembra
     un hasher distinto por instancia/proceso (el bucket de ayer no es el de hoy:
     pérdida silenciosa tras reopen), y un mapa en RAM no sobrevive al proceso.
  2. «El B+ tree es siempre más rápido que el hash». No: para igualdad exacta el
     hash es O(1) frente a O(log n); el B+ se justifica por el **range scan** y el
     orden, no por la velocidad punta.
  3. «Más buckets, siempre mejor». Cada bucket es una página de 4 KB aunque quede
     vacío: 16 buckets = 76 KB mínimos de huella.
- **NO debe saber todavía**: splits multinivel y árboles de altura > 1 (apéndice del
  propio capítulo, según ADR-005), índices dinámicos concurrentes y MVCC (cap. 28),
  cómo el optimizador elige un índice (`IndexSeek`, cap. 21), linear/extendible
  hashing (sección «BBDD real»), compresión de claves (cap. 38). Se nombran como
  «luego lo verás» y se corta ahí.

## 2. Conceptos (del grafo curricular)

- `present`: índice como mapa **clave → valor** auxiliar; *equality query* vs
  *range scan* (la dicotomía que ordena el capítulo); hash **determinista** FNV-1a
  64-bit (offset basis + prime, constantes publicadas); bucket como página;
  **overflow encadenado** vía `next_page` (separate chaining en disco); catálogo
  con magic («HID1»/«BPLU»); `HashEntry` 16 B; patrón «primer record = header
  lógico» (`BucketHeader` 4 B, `BPlusHeader` 16 B); B+ tree **single-level**
  (raíz = hoja) con pares ordenados y búsqueda binaria; capacidad de página
  (203 entradas por 4.096 B); `IndexError` tipado.
- `practice`: records de `SlottedPage` y aritmética de espacio con prefijos de
  4 B (cap. 11); `to_le_bytes` para claves `u64` (cap. 9); `allocate`/
  `is_allocated` y el convenio de páginas reservadas (cap. 12); ciclo
  `get_page` → modificar → `mark_dirty` → `unpin` → `flush_page` (cap. 13).
- `consolidate`: magic numbers como detección barata de corrupción; validar
  invariantes al `open`; errores tipados con `From` para `?`; «el formato en disco
  debe ser reproducible: mismas claves → mismo bucket para siempre».
- `out_of_scope` (solo nombrar): splits y multinivel (apéndice del cap.), rehash
  dinámico, WAL para índices (cap. 28), `IndexSeek` y estadísticas (cap. 21),
  LSM-trees (cap. 16), índices vectoriales HNSW (Vol. III).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) enuncia la dicotomía «igualdad quiere dispersión, rango quiere
  orden» y decide cuándo hash y cuándo B+ (pregunta crítica del CORPUS); (2) explica
  por qué el hasher de un índice persistente debe ser una función pura con
  constantes fijas (FNV-1a) y por qué `RandomState` o un CRC romperían el índice al
  reabrirlo; (3) calcula cuántas `HashEntry`/`TreeEntry` caben en una página
  (203) y en qué inserción salta la primera página de overflow (la 204 con un
  bucket); (4) dibuja el layout en disco de ambos índices (página 2 = catálogo/raíz;
  3..3+B-1 = buckets) y explica por qué ambos no pueden coexistir en el mismo
  fichero; (5) dice qué detecta cada magic y en qué offset vive el del B+
  (byte 14 de la página).
- **Skills**: (1) crear, poblar, persistir y reabrir un `HashIndex` y un `BPlusTree`
  sobre `FilePager` + `BufferPool` con `cargo test`; (2) predecir el comportamiento
  de `insert` (reemplazo in-place vs página nueva) y de `get`/`range_scan` leyendo
  el código del módulo; (3) diagnosticar un `IndexError` tipado sin parsear strings.
- **Wisdom**: (1) decide entre hash y B+ mirando la **consulta**, no la moda:
  igualdad pura → hash; orden, rangos, min/max, iteración → B+; (2) reconoce cuándo
  una simplificación pedagógica (single-level) deja de ser aceptable: no cuando
  «es O(n)» en abstracto, sino cuando la capacidad de página (203) se queda corta
  para el dataset — y sabe que el apéndice del capítulo cierra ese hueco.

## 4. Modelo mental

- **El parking hasheado vs la agenda ordenada**: el `HashIndex` es un parking cuyas
  plazas se asignan por hash de la matrícula — calculas la sección con aritmética y
  vas directo, pero «todas las matrículas entre AAA y AZZ» es imposible porque los
  vecinos aparcan en zonas sin relación. El `BPlusTree` es la agenda ordenada por
  apellido que se abre por la mitad: encontrar «García» cuesta 8 aperturas para 203
  nombres, y listar de Fernández a López es deslizar el dedo — pero no hay
  aritmética directa, hay que comparar para navegar.
- **Diagramas ASCII**: (a) fichero de un HashIndex (páginas 0/1 reservadas, 2 =
  catálogo, 3..3+B-1 = buckets, overflow colgado); (b) interior de una página de
  bucket (record 0 = `BucketHeader` con `next_page`, records 1.. = entradas de
  16 B); (c) raíz-hoja del B+ (record 0 = `BPlusHeader` BPLU, records 1.. =
  pares ordenados).
- **Momento ¡ajá!**: ninguna estructura da las dos cosas gratis — el hash destruye
  el orden **a propósito** (dispersión) y el orden prohíbe la aritmética directa
  (hay que comparar). Por eso las BBDD reales conviven con ambos. Y el segundo
  ¡ajá!: «guardar en disco» obliga a que `clave → bucket` sea una función pura;
  cualquier semilla aleatoria o hasher versionado es corrupción diferida.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap15_indices.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | DOS índices (hash + B+) en el mismo capítulo | Responde la pregunta crítica del CORPUS: equality quiere O(1), range quiere orden; mostrar ambos hace visible el trade-off en vez de enunciarlo | Solo hash: sin `range_scan`; solo B+: la igualdad O(1) queda como fe | El lector elige índice por moda, no por consulta | CORPUS `vol-II-cap-15`; Petrov caps. 2-4; código l. 15-26 |
| 2 | `fnv1a_64` propio (l. 201-208) | Determinismo persistente: función pura, sin semilla, constantes publicadas → mismas claves → mismo bucket **para siempre**, en cualquier proceso | `std DefaultHasher`/`RandomState`: semilla aleatoria por instancia → al reabrir, la clave cae en otro bucket y `get` devuelve `None` (pérdida silenciosa); además std no promete estabilidad entre versiones de Rust | Índice que «funciona» hasta el primer reopen | Test `fnv1a_known_values` (vectores oficiales FNV); draft-eastlake-fnv (IETF); isthe.com/chongo/tech/comp/fnv |
| 3 | FNV-1a y no CRC | CRC está diseñado para detectar errores de bits, no para dispersar: es lineal sobre GF(2) y claves consecutivas producen hashes aritméticamente relacionados → clustering en buckets | CRC32: colisiones agrupadas, cadenas largas donde duele (en disco) | Buckets desequilibrados; `get` degenera a O(n) páginas | FNV history (Fowler/Noll/Vo, 1991, POSIX P1003.2); Petrov cap. 2 |
| 4 | Buckets + **overflow encadenado** (`next_page`) | En disco la unidad de I/O es la página: seguir un puntero = 1 lectura (~203 entradas por página); la cadena crece página a página sin reconstruir nada | *Open addressing* (probing): cada sonda es una lectura de página en offset aleatorio = seek tras seek; además degrada cerca de lleno y crecer exige rehash total | `get` que paga seeks aleatorios por sonda; crecimiento traumático | Cap. 11 (coste del seek); CLRS (chaining vs probing); código l. 284-313 |
| 5 | `HashIndexHeader` con magic «HID1» en página 2 (l. 216-254) | El catálogo fija `num_buckets`/`key_count` y el magic detecta «esta página no es lo que crees» al `open` | Sin catálogo: `num_buckets` viviría en código → imposible reabrir con otro B; sin magic: basura indistinguible de índice | Corrupción silenciosa tras reopen | Tests `hash_open_without_catalog_fails`, `bplus_disk_bad_magic_fails`; MIGRATION §19.5 |
| 6 | `HashEntry` fija de 16 B (`u64`+`u64`) | Clave y valor de anchura fija: el record es autodescriptivo, decodificable sin contexto, y la aritmética de capacidad es exacta | Longitud variable: obliga a parsear por record y complica el cálculo de «¿cabe?» | `entry size mismatch` en tiempo de ejecución | Código l. 260-282; test `hash_entry_roundtrip` |
| 7 | `BucketHeader` como **primer record** (patrón «primer record = header lógico») | Reutiliza la maquinaria de records del cap. 11 sin offsets mágicos; el tamaño y el orden discriminan header de entradas | Byte fijo dentro de la página: acopla el formato al layout interno de `SlottedPage` | Cambiar el cap. 11 rompería el índice | MIGRATION §19-lección 1; código l. 292-313 |
| 8 | `insert` busca en la cadena y **reemplaza in-place**; si no cabe, aloca página y la cuelga | Semántica upsert sin duplicar claves; el append al final es O(cadena) y el reemplazo no reescribe la página entera | Re-pack de la página (desplazar entradas para hacer hueco): más complejo, beneficio marginal | Claves duplicadas que «ganan» por orden de lectura | Código l. 457-599; MIGRATION §19-lección 3 |
| 9 | `BPlusTree` con hoja ordenada + **búsqueda binaria** | `get` = O(log n) (≈8 comparaciones para 203) Y `range_scan` gratis: ambas salen del mismo invariante de orden | Array sin orden: `get` O(n); solo hash: rango imposible sin escaneo total | Consultas de rango que escanean el índice completo | Test `bplus_range_scan`; Bayer & McCreight 1972 (Acta Informatica 1(3) 173-189) |
| 10 | **Single-level deliberado** (raíz = hoja, sin splits) | Splits + promoción de separadores + rebalanceo ≈ 300 líneas de complejidad que enturbian la idea nueva (orden); lo que se pierde es medible: tope de 203 entradas e insert O(n) (memmove ~KBs + reescritura de la página) | Multinivel ya: el lector depura splits antes de entender por qué existe el orden | Capítulo inabarcable; el apéndice quedaría sin motivo | Código l. 718-741 (limitaciones declaradas); MIGRATION §19-lección 4; ADR-005 (refuerzo) |
| 11 | Raíz **fija en página 2** (con `while` hasta `is_allocated`) | Punto de entrada estable: el multinivel del apéndice NO cambia la dirección de la raíz (solo su contenido). El `while` corrige el bug real: un `allocate()` único no basta (pager recién creado tiene solo la página 0) | Raíz móvil: cada split reescribiría punteros externos | `create` sobre pager fresco → `PageNotAllocated(2)` | Código l. 756-762; MIGRATION §19-tabla de bugs (bug 1) |
| 12 | `persist` con el header como **primer record** + entradas como records | Permite a `open` distinguir header de entradas y validar `key_count` contra el número real de records | Concatenar todo en un blob: `open` leía 16 bytes y **ignoraba las entradas** (bug real: árbol «vacío» tras reopen) | Pérdida silenciosa de todas las entradas al reabrir | MIGRATION §19-tabla de bugs (bug 2); test `bplus_persistence_via_filepager` |
| 13 | Índices en **ficheros disjuntos** (no coexisten) | Ambos reclaman la página 2; la solución honesta del capítulo es un fichero por índice, documentado en test | Desplazar la raíz del B+ a 3+B: acopla ambos formatos; catálogo global en página 1: la casa real, pero es deuda futura (Apéndice D) | El segundo índice pisa el catálogo del primero | Test `hash_and_bplus_coexist` (comentario de colisión); MIGRATION §19.7 |
| 14 | `IndexError` tipado (5 variantes) | Distinguir fallo de entorno (`Io`) de invariante rota (`Inconsistent`), página fantasma (`PageNotAllocated`) y límite del diseño (`InvalidParam`: raíz llena) | Strings: imposible `match` fiable; `io::Error`: todo suena igual | El caller no puede diferenciar «rebuild necesario» de «disco roto» | Código l. 85-134; tests `index_error_display`, `index_from_pager_error` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: escanear todo (lo que el cap. 14 ya hace para propiedades: leer cada
  página de nodos, decodificar, comparar) — o el atajo tentador: un
  `HashMap<u64, u64>` en RAM al arrancar.
- **Qué la rompe**: (a) el escaneo toca los N nodos por consulta: 100.000 nodos ≈
  ~1.000 páginas de 4 KB leídas y 100.000 comparaciones **cada vez**; (b) el
  `HashMap` muere con el proceso, no cabe garantías de persistencia, y su
  `RandomState` impide asignar buckets estables en páginas fijas; (c) ninguno de
  los dos responde rangos con orden.
- **Evolución visible en el capítulo**: el `HashIndex` baja el mapa a páginas
  (bucket = página, cadena de overflow, catálogo con magic, FNV-1a puro) y el
  `BPlusTree` añade el invariante que el hash no puede tener: entradas **ordenadas**
  en la hoja-raíz → búsqueda binaria + `range_scan`. Ambos son clientes puros de la
  pila caps. 9/11/12/13: no abren el fichero jamás.

## 7. Prueba de fuego

- **Tests reales del módulo** (citados, no duplicados): `fnv1a_known_values`,
  `hash_insert_get_basic`, `hash_insert_replaces_existing`,
  `hash_insert_many_triggers_overflow_chain` (200 claves / 2 buckets: verifica el
  recorrido de la cadena), `hash_disk_roundtrip_via_filepager` (create → insert →
  flush → reopen → get), `hash_open_without_catalog_fails`,
  `hash_create_with_zero_buckets_fails`, `bplus_insert_and_get`, `bplus_range_scan`
  (extremos inclusivos, `lo > hi` vacío, rango total), `bplus_persistence_via_filepager`,
  `bplus_disk_bad_magic_fails` (corrupción en el offset 14),
  `hash_and_bplus_coexist` (documenta la no-coexistencia).
- **Síntoma si el lector se salta el capítulo**: cada `WHERE p.name = "Ada"`
  escanea el grafo entero (latencia que crece lineal con los datos) o, peor, un
  índice «persistido» con hasher sembrado devuelve `None` para claves que están
  en disco — pérdida silenciosa con aspecto de bug de lógica de negocio.

## 8. Trampas y errores comunes

1. **Usar un hasher con semilla** (`RandomState`, `DefaultHasher` con seed) en un
   índice persistente: funciona en la sesión, pierde datos tras reopen. Se detecta:
   el test de roundtrip en disco falla al reabrir.
2. **Cambiar la representación de la clave** (BE vs LE, `usize` vs `u64`): el hash
   se calcula sobre los 8 bytes LE de la clave; cambiar la codificación reubica
   todas las claves. Lección del cap. 9 aplicada al hashing.
3. **Confundir el tope de capacidad con la velocidad**: el dolor real del
   single-level es el tope de 203 entradas, no el insert O(n) (memmoves de ~KBs en
   RAM + reescritura de una página).
- **Precisión de lenguaje (glosario)**: *bucket* (página primaria de una clase de
  hash) vs *página de overflow* (página colgada); *equality query* vs *range scan*;
  *raíz* vs *hoja* (aquí son la misma página; en multinivel no); *factor de carga*
  (claves/buckets) vs *capacidad de página* (entradas/página: 203); *índice
  estático* (build-once + rebuild) vs *dinámico* (insert/delete en línea, cap. 28);
  *split* (romper un nodo lleno) vs *rehash* (redispersar buckets).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial, retrieval + spacing)**: predecir ANTES de ejecutar
  en qué inserción exacta aparece la primera página de overflow en un `HashIndex`
  con UN bucket (hay que recordar del cap. 11 cuánto cuela cada record: prefijo de
  4 B + `PageHeader` 10 B; respuesta: la 204.ª). Verificación: test que observa
  `pool().pager().num_pages()` pasar de 4 a 5. Pistas graduadas: (1) ¿qué dos
  records hay en una página de bucket antes de la primera entrada?, (2) ¿cuánto
  ocupa cada entrada contando el prefijo?, (3) `used + need <= PAGE_SIZE` en
  `insert`. Criterio: la predicción debe salir de la aritmética, no del ensayo.
- **analizar (intermedio, interleaving hash + orden)**: con las claves 0..1000 en
  AMBOS índices (pagers separados), usar `range_scan(137, 731)` del B+ y verificar
  con `get` del hash cada clave devuelta y su valor; comprobar además que 136 y
  732 existen en ambos pero no aparecen en el rango. Explicar por qué este test es
  imposible con un solo índice. Pistas: (1) ¿qué estructura responde «¿existe?»,
  (2) ¿cuál responde «¿qué hay entre…?», (3) ¿por qué el hash no puede iterar en
  orden sin escanearlo todo? Criterio: verificación cruzada correcta + argumento
  de la dicotomía.
- **crear (experto, puente al apéndice)**: dar el primer paso a multinivel: cuando
  la raíz-hoja se llena, repartir sus 203 entradas en dos hojas nuevas, promocionar
  el separador y convertir la página 2 en nodo interno (raíz fija, hojas enlazadas
  con el patrón `next_page` del `BucketHeader`). Mantener magics y hacer pasar
  todos los tests + uno nuevo de split. Pistas: (1) la raíz sigue SIEMPRE en la
  página 2 — solo cambia su contenido, (2) el orden se preserva al repartir por
  la mediana, (3) `BPlusHeader.reserved` (u64 libre) puede guardar el puntero a la
  primera hoja. Criterio: roundtrip en disco tras el split.

## 10. Preguntas abiertas (gancho al capítulo 16)

1. Los índices son estáticos y «actualizarse» es rebuild: ¿quién recoge las páginas
   del índice viejo? ¿Dónde quedan anotadas y quién detecta las huérfanas?
   (Respuesta: free list, `inspect`/`check`/`compact` — cap. 16.)
2. Si rebuild es la única actualización, ¿qué le pasa al fichero con cada rebuild
   sobre un dataset que crece? ¿Cuándo compensa compactar?
3. El cap. 19-21 traerá un optimizador: ¿cómo decidirá entre `NodeScan` e
   `IndexSeek` si nadie le dice cuánto cuesta cada uno? (Gancho a estadísticas,
   cap. 21.)
- **Términos nuevos de glosario**: índice, clave/valor, equality query, range scan,
  hash determinista, FNV-1a, bucket, factor de carga, overflow encadenado, open
  addressing, rehash, B+ tree, hoja, separador, split, fan-out, single-level,
  multinivel, índice estático, rebuild, catálogo de índice.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio esencial obliga a reconstruir desde memoria
  el coste de un record dentro de una `SlottedPage` (cap. 11) para predecir el
  punto exacto de overflow (204) — el enunciado no regala los tamaños.
- **Spacing**: el esencial re-ejercita el cap. 11 (prefijo de longitud) y el 12
  (`num_pages` como observable); el intermedio re-ejercita el ciclo
  pin/unpin del cap. 13 al convivir dos pools; «lo que te llevas» recupera la
  regla little-endian del cap. 9 (el hash opera sobre los bytes LE de la clave).
- **Interleaving**: el intermedio mezcla deliberadamente las dos estructuras del
  capítulo (igualdad hash + orden B+) en un solo test — no hay batería de
  ejercicios clónicos.
- **Dificultad asimétrica**: cada sección introduce UNA idea nueva (dispersión,
  luego orden, luego persistencia del formato); los ejercicios exigen predicción y
  verificación, no reconocimiento.
- **Bucle de feedback inmediato**: todo ejercicio se verifica con
  `cargo test -p vol2-liradb` (tests reales citados por nombre).
- **Citas**: Bayer & McCreight 1972 (Acta Informatica 1(3) 173-189; ACM DL
  10.1145/1734663.1734671); Comer, «The Ubiquitous B-Tree», ACM Computing Surveys
  11(2), 1979 (el misterio de la B); FNV: draft-eastlake-fnv (IETF) e
  isthe.com/chongo/tech/comp/fnv (vectores de test); PostgreSQL docs §11.2 (hash
  indexes y su historia pre-v10 sin WAL) y `nbtree/README` (Lehman-Yao, dedup v13);
  MySQL manual §15.6.2.1 (InnoDB clustered, 16 KB); Neo4j operations manual
  (índices nativos B+); Petrov «Database Internals» caps. 2-4; Kleppmann DDIA
  cap. 3; CMU 15-445.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (14 filas en §5).
- [x] Escenario de fallo visible sin índices (100k nodos por consulta) y del hasher sembrado (pérdida tras reopen) (§6-7 del capítulo).
- [x] Código ejecutable en workspace (27 tests) citado por nombre y línea, no duplicado.
- [x] Misconcepciones corregidas explícitamente («índice = HashMap persistido», «B+ siempre más rápido», «más buckets mejor»).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (layout de página desde memoria → predicción del overflow) y ≥1 de spacing (caps. 11-12-13 re-ejercitados).
- [x] Responde la pregunta crítica del CORPUS: §15.2/15.10 («cuándo hash y cuándo B+»).
- [x] Anécdota verificable: Bayer & McCreight (Boeing, 1970/1972) y el misterio de la B; FNV (1991, POSIX P1003.2) — con fuentes.
- [x] El apéndice del capítulo cubre el refuerzo ADR-005 (B+ multinivel y splits) como puente, no como deuda oculta.
