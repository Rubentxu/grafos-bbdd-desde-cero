# CONTRATO DE CAPÍTULO — Vol.II Cap. 12: El gestor de páginas (trait `Pager`)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap12_pager.rs` (22 tests, `tests_pager`;
> MIGRATION-PATTERN §16 los contabiliza como 18 — cifra desactualizada).
> Decisiones y bugs reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md` §16.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: qué es una página y por qué tiene tamaño fijo (cap. 11);
  `PAGE_SIZE = 4096`; el formato `PageHeader`/`SlottedPage`; que la página 0 es la
  metapágina; little-endian explícito; escribir y leer bytes con `std::fs` (Vol. I, cap. 10);
  qué es un trait de Rust y un `enum` de error con `Display`/`Error` (caps. 8-9).
- **Cree saber pero es vago/erróneo (misconcepción central)**: «`write()` escribe en disco».
  Falso: escribe en la *page cache* del kernel; sin `fsync` los datos pueden evaporarse en
  un corte de luz. Segunda misconception: «borrar = quitar bytes del fichero»; el pager no
  encoge el fichero, solo apunta la página en una free list. Tercera: «un `Vec` guarda en
  orden de salida» — guarda en orden de inserción; el LIFO lo decide el `pop()` (bug real
  documentado en §16 de MIGRATION-PATTERN).
- **NO debe saber todavía**: caché de páginas ni política de evicción (cap. 13: buffer
  pool), cómo se persiste la free list (deuda documentada; casa natural = metapágina,
  la introduce el cap. 14), WAL/fsync por transacción (cap. 28), mmap (apéndice
  comparativo futuro). Se nombran como «luego lo verás» y se corta ahí.

## 2. Conceptos (del grafo curricular)

- `present`: `Pager` como *port*, `FilePager` como *adapter* (arquitectura hexagonal
  aplicada a disco); `PageId` (= offset físico `id × PAGE_SIZE`); *free list* LIFO;
  `sync`/`fsync` y la diferencia página sucia del SO vs disco; errores tipados
  (`PagerError`) con `From<io::Error>`; validación de invariantes al `open`
  (múltiplo de `PAGE_SIZE` = detección de *torn write*/truncamiento).
- `practice`: página/metapágina (cap. 11: el pager reserva la página 0 para ella);
  `std::fs::File` + `seek`/`read_exact`/`write_all`; traits como contrato (cap. 8
  `GraphStore`).
- `consolidate`: little-endian, formato autodescriptivo, «derivar, no llevar en la cabeza».
- `out_of_scope` (solo nombrar): buffer pool y pin/unpin (cap. 13), persistencia de la
  free list (caps. 14+), compactación/vacuum (cap. 16), mmap (`MmapPager`, apéndice),
  concurrencia sobre el fichero (cap. 28+), LSM-trees (cap. 16, sección comparativa).

## 3. Objetivos de dominio

- **Knowledge**: (1) explica por qué un fichero se trata como array de páginas
  `página i = bytes [i·4096, (i+1)·4096)` y qué error detecta `open` cuando eso se rompe;
  (2) enuncia las 8 operaciones del contrato `Pager` y qué invariante protege cada una;
  (3) distingue página sucia del SO vs disco y dice cuándo `sync` es obligatorio;
  (4) explica por qué la free list vive en memoria hoy y qué pasa tras un reopen;
  (5) justifica LIFO vs FIFO para la free list con el coste de cada alternativa.
- **Skills**: (1) usar `FilePager` (create/open/allocate/write/read/free/sync) y leer sus
  errores tipados sin parsear strings; (2) escribir un test de persistencia real
  (create → write → sync → drop → open → read) como `persistencia_reabrir_tras_sync`.
- **Wisdom**: (1) decide cuándo preferir *leak* (perder espacio) sobre *corrupción*
  (doble propietario de página) al ordenar escrituras de metadatos; (2) reconoce cuándo
  una abstracción (trait) se paga sola: tests sin disco hoy, mmap mañana.

## 4. Modelo mental

- **El bibliotecario del almacén de cajas**: un hangar de cajas idénticas numeradas
  0,1,2,…; el bibliotecario (`Pager`) entrega la caja N (`read`), la recoge (`write`),
  tiene una carpeta con las cajas vacías (free list), amplía el hangar cuando no quedan
  cajas (`allocate` → `extend_by`) y al cierre comprueba que cada caja está realmente en
  su estante (`sync`). Nadie entra al hangar sin pasar por él.
- **Diagrama ASCII**: apilado port/adapter (capas de arriba: `SlottedPage`/`MetaPage`;
  port `Pager`; adapter `FilePager`; abajo: fichero/OS) + mapa del fichero como páginas
  con la free list apuntando a la página 2.
- **Momento ¡ajá!**: el `PageId` ES el offset (`id × PAGE_SIZE`), sin indirección. Por eso
  el pager puede dar y recoger cajas en O(1)… y por eso compactar (cap. 16) no puede
  mover páginas sin reescribir todos los punteros.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap12_pager.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Trait `Pager` (port) + `FilePager` (adapter) | El contrato desacopla a las capas superiores del `std::fs::File`; el cap. 13 necesita un pager falso en tests (`MemoryPager`) y un apéndice traerá `MmapPager` | Funciones sueltas sobre `&mut File`: soldaría el buffer pool al disco real → tests de evicción lentísimos y mmap imposible sin reescribir callers | Cada nuevo motor de E/S obliga a tocar todos los callers | MIGRATION-PATTERN §16.1-16.2, §17.1; sqlite.org/arch.html (el Pager de SQLite aísla el B-tree del VFS) |
| 2 | Free list LIFO (`Vec` + `pop`) | `pop()` es O(1) y sin estructuras extra; la página recién liberada es la más «caliente» (más probable que siga en caché del SO); orden determinista documentado en tests | FIFO (`VecDeque`): O(1) también, pero reutiliza la página MÁS fría y no aporta nada pedagógico; pila ordenada: O(n log n) sin beneficio | Cualquier política «creativa» ad hoc hace los tests no deterministas y esconde bugs de reutilización | `free_y_reutilizacion_multiple` (test); MIGRATION-PATTERN §16-lección 1 |
| 3 | Free list en memoria, NO persistida | Persistirla ahora exigiría reescribir la metapágina en cada `free()` y decidir el orden de durabilidad con datos que aún no tenemos (WAL es cap. 28). Elegimos el fallo benigno: tras reopen la página liberada parece «asignada» (leak de espacio, jamás corrupción) | Persistir ya: si la free list llega a disco ANTES que el dato que liberó la página, un crash entre ambas escrituras entrega la página a dos dueños → corrupción | Leak creciente de páginas hasta compactación (cap. 16); visible en `inspect` | Test `free_list_no_persiste_tras_reopen` (comentario: decisión pedagógica); MIGRATION-PATTERN §16.3, §16-lección 3; §18-lección 1 |
| 4 | `create()` reserva la página 0 | Fija `num_pages == 1` como invariante del arranque y deja el sitio de la metapágina (cap. 11) listo para `MetaPage::encode`; el convenio del cap. 14 «`start_page == 0` ⇒ array vacío» DEPENDE de que la página 0 nunca se asigne a datos | Fichero vacío (`num_pages == 0`): cada `read/write` necesita el caso especial «fichero vacío» y la metapágina podría ser reclamada por un `allocate` | La primera página de datos pisaría el catálogo del fichero entero | `create()` (comentario in-code); tests `metapagina_inicial_vacia`, `escribir_metapagina_y_reabrir`; MIGRATION-PATTERN §16.4, §18-lección 1 |
| 5 | `open()` valida múltiplo de `PAGE_SIZE` | Un tamaño no múltiplo = truncamiento parcial (torn write, crash a mitad de `extend_by`, `cp` interrumpido). Fallar en el `open` con mensaje claro (`"not a multiple of PAGE_SIZE"`) evita el debug post-mortem | Ignorar los bytes sobrantes: `num_pages` mentiría y la última media página se leería como basura válida | Corrupción silenciosa en la página final del fichero | `open_archivo_con_tamanho_invalido_falla`; MIGRATION-PATTERN §16.6, §16-lección 4 |
| 6 | `sync()` existe aunque el SO «ya escribió» | `write_all` termina en la page cache del kernel (página sucia del SO, no disco). `sync_all()` (fsync) fuerza el volcado. Separado de `write` porque fsync cuesta órdenes de magnitud más: se agrupa N writes + 1 sync | fsync en cada `write`: throughput colapsado (µs → ms por página) | El proceso «funciona», el corte de luz llega y lo escrito «con éxito» desaparece | `sync()`; `persistencia_reabrir_tras_sync`; DDIA cap. 3 (durabilidad y page cache del SO); CMU 15-445 |
| 7 | `PagerError` tipado, no `io::Error` directo | El caller debe distinguir bug propio (`OutOfRange`, `FreePage`, `BadBufferSize`) de fallo del entorno (`Io`); con `io::Error` habría que parsear strings | `io::Error` a secas: mensajes frágiles, imposible `match` fiable | El buffer pool del cap. 13 no podría reaccionar distinto a cada fallo | MIGRATION-PATTERN §16.5; test `pager_error_display_y_source`; `From<io::Error>` para `?` |
| 8 | `read` valida tamaño→rango→free y usa `read_exact` | Los checks baratos primero y ANTES de tocar el fichero; `read` puede devolver menos bytes (short read), y media página es peor que ninguna | Confiar en el caller: un buffer de 10 bytes recibe 10 bytes y el resto queda basura vieja | Páginas «leídas» a medias, indistinguibles de corruptas | `read_buffer_mal_tamano_falla`, `read_pagina_inexistente_falla`, `read_en_pagina_libre_falla` |
| 9 | `free()` ni borra contenido ni encoge el fichero | El `PageId` es offset físico: mover/truncar invalida punteros del CSR (cap. 14) e índices (cap. 15); borrar a ceros cuesta una escritura de 4 KB extra sin ganar nada (quien reuse la página la sobrescribe entera) | Zero-fill o truncate: I/O extra + invalidación de punteros | Punteros internos apuntando a páginas movidas = corrupción masiva | `free()` (comentario: «sigue ocupando espacio hasta un futuro vacuum»); SQLite: sus freelist leaf pages «contain no information at all» (fileformat2.html) |
| 10 | `PageId = u32` (no `usize`) | Es un tipo de FORMATO persistente: `PageHeader.page_id` (cap. 11) es u32 little-endian; `usize` cambia con la plataforma. Límite honesto: 2^32 páginas × 4 KB = 16 TiB, custodiado por `NoFreePageId` (`checked_add` en `extend_by`) | `usize`: compila en tu portátil y corrompe el formato en otra arquitectura | Overflow silencioso de IDs | `extend_by` con `checked_add`; variante `NoFreePageId` («4 GiB agotados») |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: funciones sueltas `fn read_page(file: &mut File, id: u32, buf: &mut [u8])`
  replicadas en cada módulo, con `file.seek(id * 4096)` desparramado y sin registro de
  páginas libres. Compila y pasa el happy path.
- **Qué la rompe**: (a) dos estructuras calculan su sitio y ambas usan la página 7
  (doble uso de página libre); (b) un crash a mitad de `extend` deja el fichero con
  4096·n + k bytes y nadie lo detecta; (c) cada caller re-implementa (u olvida) la
  validación de buffer/rango; (d) todo el sistema queda soldado a `std::fs::File`.
- **Evolución visible en el capítulo**: el trait `Pager` concentra la aritmética
  (`offset_of`), la propiedad del inventario (`free_list`, `num_pages`), la validación
  (3 checks antes de I/O) y la durabilidad (`sync`); `FilePager` queda como pieza
  intercambiable detrás del contrato.

## 7. Prueba de fuego

- **Tests reales del módulo** (se citan, no se duplican): `create_y_open_roundtrip`,
  `allocate_extiende_fichero`, `read_write_roundtrip`, `free_y_reutilizacion` (LIFO),
  `free_y_reutilizacion_multiple`, `read_en_pagina_libre_falla`,
  `open_archivo_con_tamanho_invalido_falla`, `persistencia_reabrir_tras_sync`,
  `escribir_metapagina_y_reabrir` (integración con cap. 11),
  `free_list_no_persiste_tras_reopen` (la deuda, documentada en forma de test).
- **Síntoma si el lector se salta el capítulo**: offsets calculados a mano esparcidos por
  el código, ficheros con tamaño no múltiplo de 4096 «que funcionan» hasta el primer
  corte de luz, y páginas liberadas reutilizadas por dos dueños a la vez
  (corrupción silenciosa con aspecto de grafo válido).

## 8. Trampas y errores comunes

1. **Creer que `write` es durable**: sin `sync`, «guardado» significa «en la RAM del SO».
2. **Esperar FIFO de la free list**: el `Vec` guarda en orden de inserción; el LIFO lo
   dicta `pop()` (bug real del test `free_y_reutilizacion_multiple`, MIGRATION §16).
3. **Usar `read` en vez de `read_exact`**: media página leída parece éxito.
- **Precisión de lenguaje (glosario)**: *página* (unidad de disco) vs *frame* (unidad de
  memoria del pool, cap. 13); *free page* (existe en disco, sin dueño) vs *página
  borrada* (no existe tal cosa aquí); *página sucia* (modificada en caché, aún sin
  escribir) vs *página libre*; `num_pages` (páginas del fichero, INCLUYE libres).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar**: predecir (sin ejecutar) qué devuelve `free_list()` y los tres
  siguientes `allocate()` tras liberar 1, 2, 3 en ese orden. Verificación: test
  `free_y_reutilizacion_multiple`. Pistas: (1) ¿dónde añade `push`?, (2) ¿de dónde saca
  `pop`?, (3) ¿quién decide el orden de salida? Criterio: razona LIFO sin confundirlo
  con el orden de inserción del `Vec`.
- **analizar**: el fichero `bad.liradb` tiene 4095 bytes. ¿Qué variante de error, con
  qué mensaje, y por qué `InvalidData` y no truncar silenciosamente? Verificación:
  `open_archivo_con_tamanho_invalido_falla`. Pistas: (1) ¿quién dejó esos 4095 bytes?,
  (2) ¿qué diría `num_pages`?, (3) ¿leak o corrupción?
- **crear**: escribir un test que integre cap. 11 y cap. 12: construir una `MetaPage`
  con valores conocidos, escribirla en la página 0, `sync`, reabrir y decodificar.
  Verificación: patrón de `escribir_metapagina_y_reabrir`. Pistas: (1) ¿qué página es
  siempre la metapágina?, (2) ¿qué falta entre write y drop?, (3) ¿qué método del
  cap. 11 decodifica? Criterio: el test debe fallar si se quita el `sync`… o
  documentar por qué NO falla (el drop cierra y el SO ya tiene los bytes).

## 10. Preguntas abiertas (gancho al capítulo 13)

1. Si cada `get_page` del futuro fuera al disco, ¿de qué serviría la page cache del SO?
   ¿Y si dos partes del motor quieren la MISMA página a la vez?
2. ¿Quién decide qué página sale de memoria cuando esta se llena (pin, dirty, evicción)?
3. ¿Cómo probamos evicción y dirty-flush SIN tocar el disco en cada test?
   (Respuesta: `BufferPool<P: Pager>` + `MemoryPager` — cap. 13.)
- **Términos nuevos de glosario**: port/adapter, free list, LIFO, fsync/sync_all,
  página sucia, short read, torn write, leak vs corrupción, adapter `MmapPager`.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: ejercicio propuesto «esencial» pide recordar DESDE LA MEMORIA
  qué guarda la `MetaPage` del cap. 11 y por qué `create()` reserva la página 0 — el
  enunciado no regala los campos.
- **Spacing**: el ejercicio «intermedio» y el «resuelto» re-ejercitan la metapágina y el
  roundtrip little-endian del cap. 11 a través del pager; «lo que te llevas» referencía
  la regla `id × PAGE_SIZE` nacida en el cap. 11.
- **Interleaving**: el ejercicio «experto» mezcla free list LIFO con estructura de
  datos (bitmap vs Vec) y con el coste real O(n) de `contains`; el «analizar» cruza
  truncamiento de fichero con la decisión leak-vs-corrupción.
- **Dificultad asimétrica**: cada sección introduce UNA idea nueva (contrato, inventario
  libre, durabilidad); los ejercicios exigen esfuerzo de recuperación y predicción.
- **Bucle de feedback inmediato**: todo ejercicio se verifica con `cargo test -p
  vol2-liradb` en el workspace (tests reales citados por nombre).
- **Citas**: SQLite (sqlite.org/arch.html, fileformat2.html; CoRecursive #066 y CMU
  Databaseology 2015 para la anécdota de Hipp), Petrov «Database Internals» cap. 2-3,
  Kleppmann «DDIA» cap. 3, CMU 15-445 (buffer pool), PostgreSQL (smgr/md.c, FSM).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (10 en la tabla §5).
- [x] Escenario de fallo visible sin pager: offsets a mano, media página tras crash, doble uso de página libre (§6 del capítulo).
- [x] Código ejecutable en workspace (22 tests) citado por nombre, no duplicado.
- [x] Misconcepción corregida explícitamente («write es durable» + FIFO vs LIFO + Vec).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (MetaPage desde memoria) y ≥1 de spacing (roundtrip cap. 11 vía pager).
- [x] Responde las preguntas críticas del guion: trait vs funciones, LIFO vs FIFO, free list no persistida, página 0, validación de `open`, por qué `sync`, errores tipados.
