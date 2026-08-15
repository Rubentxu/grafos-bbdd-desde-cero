# Capítulo 16 — Compactación y mantenimiento: `liradb inspect | check | compact`

> *«Una base de datos no se ensucia de golpe. Se ensucia un poco en cada escritura. Por eso el mantenimiento no es un accesorio: es la consecuencia lógica de todo lo que decidiste en los capítulos anteriores.»*

## 16.0 La anécdota de la esquina

El 8 de noviembre de 2005 salió PostgreSQL 8.1 con un cambio que hoy suena obvio y entonces era una confesión: **autovacuum pasó de ser un módulo aparte (`contrib`) a estar integrado en el servidor**. Las release notes oficiales lo dicen sin drama: integrarlo permitía que arrancara y parara sincronizado con el servidor y se configurara con los ajustes estándar. Es decir: durante una década, la recolección de basura de PostgreSQL había sido… opcional. Y la gente se la saltaba.

¿Garbage? ¿No habíamos quedado en que las bases de datos borran? No exactamente. PostgreSQL usa MVCC (lo veremos a fondo en el capítulo 30): un `UPDATE` **no sobrescribe** la fila vieja —inserta la versión nueva y marca la antigua como muerta en su sitio—, y un `DELETE` también se limita a marcar. Las tuplas muertas se quedan en su página, ocupando bytes, hasta que alguien pasa el aspirador: `VACUUM`. Sin él, las tablas se hinchan, los índices engordan y —peor— el contador de transacciones da la vuelta y la base deja de aceptar escrituras. Los administradores serios ponían un `cron`. Los demás descubrían el problema a las tres de la mañana.

La lección para nosotros es doble. Primera: **el mantenimiento no es un extra que se añade al final; es la factura de decisiones de diseño anteriores** (en su caso, MVCC; en el nuestro, el `free_space` cacheado del capítulo 11 y las páginas liberadas del capítulo 12). Segunda: PostgreSQL tuvo que convertir el aspirador en un proceso automático **porque los humanos no lo ejecutaban**. LiraDB es embebida y didáctica: aquí el mantenimiento será manual, explícito y de tres herramientas — pero con los papeles tan bien separados que sepas exactamente qué hace cada una y qué NO debe hacer.

## 16.1 Objetivo

Este capítulo cierra la Parte III. Hemos construido páginas (11), un pager (12), un buffer pool (13), adyacencias CSR (14) e índices (15). Toca hacer lo que hace todo DBDS adulto: **mirar su propio fichero con ojos críticos**. Vas a construir tres operaciones:

1. `inspect()` — estadísticas de almacenamiento (`StorageStats`): cuántas páginas hay, cuántas sirven, cuántos records guardan, y dos ratios de negocio (`fragmentation_ratio`, `utilization`).
2. `check()` — verificación de integridad read-only (`CheckReport`): las cuatro invariantes estructurales de cada página, con tipos de problema tipados (`IssueKind`).
3. `compact()` — repack masivo in-place (`CompactReport`): reescribir cada `SlottedPage` sin huecos internos, con `free_space` veraz y basura a cero. Sin mover páginas. Jamás.

Y una sección de criterio: por qué el mundo exterior está lleno de LSM-trees (RocksDB, LevelDB, Cassandra) y por qué LiraDB, aun sabiéndolo, sigue con slotted pages + repack.

## 16.2 Problema

Tras semanas de uso, el fichero `graph.liradb` de LiraDB presenta los tres síndromes clásicos de cualquier motor:

1. **Espacio muerto.** Cada `replace()` del CSR (capítulo 14) y cada rebuild de índice (capítulo 15) asigna páginas nuevas. Las antiguas acaban en la free list del pager… que vive en memoria y se pierde al reabrir (capítulo 12): tras un reopen son páginas huérfanas que nadie liberará. El fichero solo crece.
2. **Fragmentación.** Dentro de cada página quedan bytes libres dispersos y bytes basura tras el último record. Ningún record nuevo los aprovecha sin una reescritura.
3. **Inconsistencia.** Un crash a mitad de un update puede dejar un `PageHeader` con el `free_space` de antes mientras los records son los de después. O peor: un magic machacado por un fallo de disco.

**El escenario de fallo concreto** —fíjate, porque es el que motiva todo el capítulo—: un grafo que se actualiza cada noche (rebuild del CSR) durante seis meses. El fichero pasa de 80 MiB a 3 GiB. `ls -lh` asusta. Y cuando alguien abre la página 42 descubre que su cabecera dice `free_space = 18` cuando dentro caben 3.900 bytes libres de verdad: **el fichero crece y crece con un `free_space` mentiroso**. ¿Cuánto de esos 3 GiB son datos? ¿Está el fichero sano? ¿Se puede arreglar sin romperlo? Ningún comando actual responde. Los tres de este capítulo, sí.

## 16.3 Modelo mental

Vuelve al almacén del capítulo 12: el hangar de cajas numeradas. Han pasado seis meses de mudanzas y ahora mismo:

- Cada **estante tiene un número pintado en la pared** — y ese número es una **dirección pública**: el CSR (capítulo 14) tiene cuatro punteros a estantes concretos, el hash index (capítulo 15) guarda el estante donde empieza cada bucket, el B+tree apunta a los estantes de sus hijos. Publicada quiere decir publicada: **no puedes cambiar una caja de estante sin avisar a todos los que te apuntan**.
- Dentro de cada **caja**, en cambio, nadie mira: los records pueden tener huecos entre ellos, orden distinto, basura al fondo. El interior es **privado**.

Con eso, las tres herramientas se definen solas:

- `inspect` es el **inventario**: abre cada caja y cuenta lo que HAY (no lo que dicen las etiquetas, que pueden estar desactualizadas).
- `check` es el **auditor con linterna**: verifica que cada caja está en su estante, que su etiqueta encaja, que los bultos están enteros. Y **no toca nada**: señala y anota.
- `compact` es la **cuadrilla de orden**: entra en cada caja, apila los bultos consecutivos, limpia la basura del fondo y reescribe la etiqueta con la cifra exacta. Jamás —jamás— cambia una caja de estante.

El momento ¡ajá! del capítulo es exactamente esa frontera: **la dirección del estante es pública; el interior de la caja es privado**. Todo lo que `compact` puede hacer vive dentro de la página; todo lo que no puede hacer (mover, truncar, vacunar) choca contra punteros ajenos. Y esa frontera no la inventamos hoy: la decidió el capítulo 12 cuando proclamó `PageId = offset físico`.

## 16.4 Primera solución

La versión que escribiría cualquiera: mirar el tamaño del fichero y sumar los metadatos.

```rust
// Solución ingenua: creer las etiquetas.
fn salud_ingenua(pool: &mut BufferPool<impl Pager>) -> u64 {
    let mut libres = 0;
    for id in 0..pool.pager().num_pages() {
        let mut buf = [0u8; PAGE_SIZE];
        pool.pager_mut().read(id, &mut buf).unwrap();
        let header = PageHeader::decode(&buf[..PageHeader::SIZE]).unwrap();
        libres += header.free_space as u64;   // ← fe
    }
    libres
}
```

Un número. Barato de calcular. Y profundamente inútil, por tres razones distintas que conviene separar:

1. **Cree en etiquetas que pueden mentir.** El `free_space` del header es un cache de `PAGE_SIZE − header − Σ records`. Si un crash lo desincroniza, estamos midiendo la mentira, no la realidad — que es justo lo que queríamos detectar.
2. **No distingue nada.** 3 GiB con 2 GiB «libres» puede ser un grafo sano con hueco normal, un fichero lleno de páginas huérfanas, o un disco muriéndose con magics corruptos. El número total no diferencia síndromes.
3. **No es accionable.** ¿Y ahora qué? ¿Reescribo el fichero? ¿Lo copio? ¿Rezo? Sin tipificar los problemas, no hay decisión posible.

## 16.5 Sus límites

La solución ingenua se rompe de forma verificable. Falsifica el `free_space` de una página (machaca los bytes 8..10 de su cabecera, little-endian, como hicimos en el capítulo 11) y la función ingenua reportará una cifra desviada sin que nada la contradiga. Machaca el magic (bytes 0-1) y el `PageHeader::decode` del bucle hará `unwrap()` y **el inventario entero reventará en la página corrupta** — el peor comportamiento posible: la herramienta de diagnóstico es la primera en morir ante el daño que debía reportar. Es el síndrome del médico que se desmaya al ver sangre.

Lo que necesitamos son tres herramientas con contratos distintos y explícitos: medir (tolerante), verificar (exhaustivo), reparar (conservador). Y todas con una regla común: **derivar la verdad del contenido, no de las etiquetas**.

## 16.6 Solución evolucionada

El código completo vive en `liradb-workspace/crates/vol2-liradb/src/cap16_mantenimiento.rs` (1.144 líneas, 25 tests). Vamos por piezas.

### inspect: el inventario que no cree en etiquetas

`StorageStats` cuenta nueve cosas, y todas se calculan **decodificando el contenido real** de cada página asignada:

```rust
pub struct StorageStats {              // campos reales del módulo
    pub total_pages: u32,      // tamaño del fichero en páginas
    pub allocated_pages: u32,  // no en la free list
    pub free_pages: u32,       // en la free list (reutilizables)
    pub data_pages: u32, pub meta_pages: u32,
    pub bytes_on_disk: u64,    // total_pages × PAGE_SIZE
    pub bytes_used: u64,       // header + records (¡medido!)
    pub bytes_free: u64,
    pub total_records: u64,
}
```

El bucle central de `inspect` es un barrido completo con una decisión en cada esquina:

- **Páginas no asignadas**: se cuentan en `free_pages` y **no se leen** (contienen ceros por diseño, capítulo 12; leerlas sería reportar `BadMagic` falsos).
- **Página 0**: se decodifica como `MetaPage`; su consumo es fijo (`PageHeader::SIZE + MetaPage::INFO_SIZE` = 10 + 12 = 22 bytes).
- **Data pages**: `SlottedPage::decode` y a contar: `used = PageHeader::SIZE + Σ record.len()`. Ojo al detalle fino (nos mordió en los tests): esta medida **excluye los length-prefixes de 4 bytes**, porque es exactamente la aritmética de `SlottedPage::free_space()` del capítulo 11. Derivar con la misma regla que la primitiva, o los números no casan.
- **Página corrupta**: se cuenta como data sin records, `bytes_free += PAGE_SIZE`… y **se sigue**. `inspect` no aborta jamás: un inventario que explota en la página 401 de 10.000 no sirve para inventariar.

Y sobre esos totales, dos métodos derivados —métodos, no campos almacenados, porque ya sabes nuestra regla: derivar, no llevar en la cabeza:

```rust
pub fn fragmentation_ratio(&self) -> f64 { // bytes_free / bytes_on_disk
pub fn utilization(&self) -> f64 {         // bytes_used / bytes_on_disk
```

**¿Qué mide `fragmentation_ratio` y qué decisión de negocio te dice?** Es la proporción del fichero que NO son datos. Piensa en un despliegue embebido — un dispositivo edge con 8 GiB de flash. Fichero de 3 GiB con `fragmentation_ratio = 0.7`: hay ~2.1 GiB de flash ocupada que no son datos. La decisión que dispara la métrica no es «corre compact» (compact, ya lo veremos, no encoge el fichero): es **«este fichero necesita una reconstrucción»** — export/import (capítulo 32) hoy, el vacuum pendiente en el futuro. Y en el otro extremo, un ratio bajo en un grafo de solo-lectura te dice que no toques nada: el mantenimiento también consiste en saber cuándo no actuar. `utilization` es su complemento y la métrica que querrás graficar en el capítulo 35 (observabilidad).

### check: el auditor que no toca nada

`check` verifica las cuatro invariantes que los capítulos 11-12 prometieron, y las tipifica:

| Invariante | `IssueKind` si falla | Qué suele significar |
|---|---|---|
| Magic válido (`0xDA`/`0xFE`, `bytes[0]==bytes[1]`) | `BadMagic { expected, got }` | Fallo de disco, página pisada |
| `header.page_id == offset físico` | `PageIdMismatch { header_says, actual }` | Página movida o escrita en el sitio equivocado |
| Records decodifican sin truncado | `RecordTruncated` | `num_records` fuera de rango, escritura rasgada |
| `free_space` declarado == real | `FreeSpaceMismatch { declared, actual }` | Crash a mitad de un update |

(Más `Undecodable(reason)` como cajón genérico para lo que no encaja en nada.) El resultado es un `CheckReport { pages_checked, issues }` con `ok()` e `issue_count()`. Cero escrituras: `check` es **read-only por contrato**.

**¿Por qué read-only y no un «check y arregla»?** Porque un check que repara esconde corrupción real. Las cuatro invariantes no son iguales: un `FreeSpaceMismatch` es un metadato desactualizado — benigno, `repack` lo reconcilia sin perder nada. Pero un `BadMagic` significa que los bytes de esa página NO son lo que el formato prometió: puede ser un fallo de disco en curso. Si `check` «arreglara» ese magic reescribiéndolo, estarías **destruyendo la evidencia y bendiciendo datos posiblemente corruptos**. Es la diferencia entre el médico que diagnostica y el que receta sin mirar. La separación check-diagnostica / compact-repara existe para que la decisión sobre lo estructural sea humana: restaurar backup, extraer a mano los records legibles, o aceptar la pérdida. Por eso el contrato dice literalmente: *«Es read-only: no modifica nada. Para reparar free_space desactualizado usar repack_page o compact»*.

**¿Y por qué exhaustivo?** Porque su trabajo es dar la lista COMPLETA. Fíjate en el detalle del bucle: cuando el header de la página 1 falla, `check` no hace `return` — hace `push` del issue y `continue` a la página 2. Un auditor que se va a casa al primer problema detectado es un auditor inútil.

### repack_page: la unidad atómica

`repack_page(pool, id)` reescribe UNA data page in-place. El algoritmo es honestamente pequeño:

```rust
let mut buf = [0u8; PAGE_SIZE];
pool.pager_mut().read(page_id, &mut buf)?;
let sp = SlottedPage::decode(&buf)?;            // 1. leer y decodificar
let mut repacked = SlottedPage::new(page_id, PageType::Data);
for rec in sp.records() {
    repacked.insert(rec)?;                      // 2. re-insertar consecutivos
}
pool.pager_mut().write(page_id, &repacked.encode())?;  // 3. mismo PageId
```

```
ANTES (página 7):                       DESPUÉS (página 7, mismo estante):
┌────────────────────────────┐          ┌────────────────────────────┐
│ header: free_space = 18 ✗  │          │ header: free_space = 3.900 ✓│
│ [rec1][hueco][rec2][basura]│   ──►    │ [rec1][rec2][ceros…]      │
└────────────────────────────┘          └────────────────────────────┘
```

Tres NO deliberados: **no mueve la página** (mismo `PageId`), **no elimina records** (`num_records` se conserva) y **no toca la metapágina** (id 0 → `MaintenanceError::BadPageType { expected: 0xDA, got: 0xFE }` — su layout es fijo y no tiene nada que repackear). El `RepackResult` devuelve `free_before` (lo que decía la etiqueta), `free_after` (la realidad recalculada) y `bytes_reclaimed = |free_before − free_after|`.

Honestidad brutal sobre esa métrica: **`bytes_reclaimed` mide la corrección del metadato y la limpieza de basura, no espacio utilizable nuevo**. El repack no mueve records entre páginas ni fusiona nada — los mismos bytes de datos siguen en la misma página. Quien espere milagros aquí no ha entendido la frontera público/privado del modelo mental. Lo que compra el repack: cabeceras veraces (`check` queda limpio), bytes basura a cero (fichero más comprimible y diff-eable), idempotencia (repack de una página sana → `modified = false`, `bytes_reclaimed = 0`, como exige el test `repack_page_idempotente_on_clean_page`).

### compact: repack masivo con seguimiento

`compact` es el bucle que encadena todo: `inspect` antes → `repack_page` en cada página asignada (1..N) → `sync()` → `inspect` después. El `CompactReport` lleva `stats_before`/`stats_after` para que verifiques el efecto con datos, no con fe. Y una regla fina de tolerancia: si `repack_page` falla con `DecodeFailed` (página corrupta), la página se **salta** (`pages_skipped`) y se deja intacta; cualquier otro error —E/S, asignación— **escala y aborta**. Distinguir «este dato está roto» (se salta, `check` ya lo reportó) de «el entorno está roto» (se aborta) es exactamente la diferencia entre tolerante y temerario.

### Por qué leer con `pager_mut` y no por el buffer pool

Todas las lecturas del capítulo usan `pool.pager_mut().read(id, &mut buf)` con un buffer reutilizable, en vez de `pool.get_page(id)`. La razón conecta directo con el capítulo 13: el buffer pool existe para la **hot path**, donde la locality manda y las páginas se reutilizan. Un barrido de mantenimiento es lo contrario: **cada página se lee exactamente una vez**. Pasar por el pool llenaría los frames de páginas que jamás volverán a pedirse y expulsaría las calientes — tras un `inspect`, tu grafo quedaría «enfriado» y el primer `MATCH` post-mantenimiento pagaría el precio de recargarlo todo. Es el anti-patrón clásico de los full scans sobre caches LRU (en 15-445 lo llaman scan-resistance). Regla general que te llevas: **administración ≠ hot path**. La misma decisión aplicará a backups, estadísticas y recuperación (capítulo 29).

Los errores, como en los capítulos 14-15, son un enum tipado (`MaintenanceError`: `Io`, `BadPageType`, `DecodeFailed`, `PageNotAllocated`) con `From<BufferPoolError>` y `From<PagerError>` para que `?` fluya — paralelo exacto de `IndexError`.

## 16.7 Prueba de fuego

El módulo trae 25 tests. Los que prueban las promesas del capítulo:

- **inspect mide realidad**: `inspect_counts_records_and_pages` (3 páginas × 2 records de 8 B; `bytes_used` calculado SIN prefixes — el bug que tuvimos), `inspect_counts_free_pages` (una página liberada baja `data_pages` y sube `free_pages`), `inspect_is_tolerant_to_corrupt_data_page` (página de 0xFF: cuenta y sigue).
- **check detecta cada invariante**: `check_detects_bad_magic` (bytes 0-1 ← 0x11), `check_detects_page_id_mismatch` (bytes 2..6 ← 99), `check_detects_free_space_mismatch` (bytes 8..10 ← 1234), `check_skips_free_pages` (una página de ceros NO genera falsos positivos), `check_clean_passer_ok`.
- **repack corrige y es idempotente**: `repack_page_corrije_free_space_corrupto` (falsifica 1111 → repack → `check` limpio), `repack_page_rechaza_meta_page` / `_no_asignada` / `_corrupta`.
- **compact masivo**: `compact_corrige_free_space_y_mejora_stats` (2 páginas falsificadas → 2 issues → compact → check OK), `compact_salta_paginas_corruptas`, `compact_sin_data_pages_no_op`.
- **Persistencia real**: `inspect_y_check_sobre_filepager_tras_reopen` y `repack_persiste_free_space_corregido_a_disco` — con `FilePager` de verdad: corrupción en disco, reopen, check la ve, repack + `sync`, reopen, check limpio.

¿Qué pasaría si te saltaras este capítulo? El síntoma es el del §16.2: ficheros que solo crecen, `free_space` desincronizado sin nadie que lo note, y corrupción de disco descubierta por un `panic` en producción en lugar de por un informe con `page_id` y variante.

## 16.8 El motor completo, de un vistazo

Cerramos la Parte III encadenando las piezas — cubre la columna derecha antes de mirar:

```
┌────────────────────────────────────────────────────────────────┐
│  (15) HashIndex / B+Tree     ¿dónde está el nodo con clave k?  │
│        buckets en páginas 3..  │ raíz en página 2              │
├────────────────────────────────────────────────────────────────┤
│  (14) CSR                    ¿quiénes son los vecinos de v?    │
│        4 arrays en chunks, CsrHeader con 4 PageIds             │
├────────────────────────────────────────────────────────────────┤
│  (13) BufferPool             ¿qué páginas viven en RAM?        │
│        frames + pin/unpin + LRU  ── hot path ──                │
├────────────────────────────────────────────────────────────────┤
│  (12) Pager                  ¿dónde está cada página?          │
│        PageId = offset físico  │ free list  │ sync             │
├────────────────────────────────────────────────────────────────┤
│  (11) SlottedPage/MetaPage   ¿qué hay dentro de la página?     │
│        header 10 B + records length-prefixed                  │
├────────────────────────────────────────────────────────────────┤
│  (16) inspect│check│compact  ¿está sano? ¿cuánto sobra?        │
│        mira TODO el stack desde fuera de la hot path           │
└────────────────────────────────────────────────────────────────┘
```

Retrieval (sin mirar arriba): ¿qué estructura guarda la dirección de la página donde viven los targets forward del CSR? ¿Y la del bucket 5 del hash? ¿Quién valida que una página está en el estante que dice su header? Las respuestas — `CsrHeader.forward_targets_page`, `bucket_starts[5]`, y nuestro `check` de hoy — son la columna vertebral del último porqué del capítulo, al que llegamos ahora mismo.

## 16.9 ¿Por qué no movimos páginas? El coste exacto del vacuum

La pregunta del millón: si el fichero tiene páginas huérfanas y huecos, ¿por qué no reorganizarlo de verdad — mover la página 7 al hueco de la 2? Cuantifiquémoslo. Mover la 7 a la 2 significa que **todas las páginas por encima de la 2 bajan una posición**. Cada `PageId` del fichero pasa a mentir. ¿Quiénes guardan `PageId`s?

- La **MetaPage** (`root_page`): 1 puntero.
- El **`CsrHeader`** (cap. 14): `forward_offsets_page`, `forward_targets_page`, `backward_offsets_page`, `backward_targets_page`: 4.
- El **HashIndex** (cap. 15): `bucket_starts[i]` — uno por bucket (B), más un `next_page` por cada página de desbordamiento encadenada.
- El **B+Tree** (cap. 15): cada nodo interno guarda los `PageId` de sus hijos (~fanout F por nodo; con 3 nodos internos y fanout 64 son ~190).

Con B = 64 buckets y ese árbol modesto ya rondamos **260 punteros a reescribir** por cada página que baja un hueco — y un vacuum real reordena miles. Pero el número ni siquiera es lo importante: lo importante es que la reescritura debe ser **atómica y transaccional** (crash a mitad = fichero roto), y eso exige un snapshot consistente y un WAL que ordenen la danza completa. Esa maquinaria no existe todavía: llega con los capítulos 28-30. Por eso MIGRATION-PATTERN §20 lo deja escrito: *«declarar la limitación en el propio cap es más honesto que implementar un vacuum a medias que corrompería punteros»*. El vacuum completo queda como **deuda declarada**, y vivirá en el capítulo 29 (recuperación, donde el WAL permitirá reescribir punteros con seguridad) y el 36 (arquitectura final). Mientras tanto: `inspect` te dice cuánto te cuesta la deuda; export/import es la vía manual.

Y una curiosidad que confirma el diagnóstico: el síntoma de «una página movida sin reescribir punteros» existe y lo tenemos tipificado — es exactamente `IssueKind::PageIdMismatch`: el header dice `page_says: 99` pero la página vive en el offset 1.

## 16.10 LSM-trees comparados (refuerzo ADR-005)

Seamos honestos: si buscas «database storage engine» en 2026, la mitad del mundo te hablará de LSM-trees. Mereces saber qué son y por qué NO los elegimos — no por ignorancia, sino por criterio.

**Qué es.** Un Log-Structured Merge-tree (O'Neil, 1996) invierte la apuesta: en vez de páginas in-place, las escrituras caen en una **memtable** en memoria (un mapa ordenado); cuando llena, se vuelca a disco como **SSTable** —un fichero ordenado e INMUTABLE— mediante una escritura 100 % secuencial; y como todo es inmutable, la única forma de fusionar, compactar o borrar es escribir SSTables nuevas: la **compaction**, que corre en background. En LevelDB y RocksDB se habla de *minor compaction* (memtable → SSTable de nivel 0) y *major compaction* (fusionar SSTables entre niveles). Hay dos sabores: **leveled** (cada nivel ~10× mayor, rangos disjuntos dentro del nivel; menos lectura de fondo, más escritura) y **tiered/universal** (se agrupan SSTables del mismo nivel y se fusionan al siguiente; menos escritura total, más ficheros que mirar al leer).

**Por qué encanta.** Es **write-optimized** en estado puro: un SSD escribe secuencialmente a velocidad de susto y las SSTables no se tocan nunca más. Ningún update in-place, ninguna página a medias, ningún repack: el desorden se paga en background, no en la escritura.

**El precio.** Las lecturas se degradan: un `get` puede tener que mirar memtable + varias SSTables solapadas del nivel 0 + un rancho por nivel (**read amplification** — de ahí los bloom filters obligatorios). Cada byte se reescribe varias veces al subir niveles (**write amplification**: en leveled, ~10× por nivel). La compaction come CPU e IO de forma permanente y su tuning es legendariamente difícil; los *write stalls* cuando no da abasto son el susto clásico de producción. Y los deletes son **tombstones** que siguen ocupando espacio hasta que una compaction los fusiona — sí, el mismo «espacio muerto» de siempre, disfrazado.

**Por qué LiraDB no.** Para un grafo embebido didáctico, el patrón dominante es: escrituras por lotes (carga + rebuild), y lecturas puntuales por `NodeId` más recorridos de adyacencia (CSR, capítulo 14) — es decir, exactamente lo que las páginas direccionables + índices hacen bien. Un LSM añadiría un compactor concurrente con scheduling y snapshots: complejidad de la que aún no tenemos ni los primitivos (caps. 28-30), para un beneficio que nuestras slotted pages + repack cubren en el 90 % de los casos con el 10 % de la máquina. La decisión queda registrada como ADR-005: **slotted pages + repack para Vol. II; LSM como alternativa estudiada y descartada por ahora**.

**Honestidad final.** Hay sistemas reales —buenos— que SÍ eligen LSM para grafos y documentos: JanusGraph corre sobre Cassandra y HBase (ambos LSM); Apache HugeGraph ofrece backend RocksDB; Dgraph construyó su KV propio (Badger) con diseño LSM-like; y en el mundo documento/KV, RocksDB está debajo de TiKV, MyRocks, Couchbase y estuvo bajo CockroachDB y MongoRocks. Cuando tu carga es escritura masiva distribuida, LSM gana. Cuando eres un fichero embebido con punteros físicos y vocación pedagógica, gana la caja en su estante.

## 16.11 Cómo lo hace una BBDD real

- **PostgreSQL**: `VACUUM` normal marca el espacio de las tuplas muertas como reutilizable DENTRO de la tabla (no devuelve nada al SO, igual que nuestro repack no encoje el fichero); `VACUUM FULL` reescribe la tabla entera con bloqueo exclusivo (desde 9.0, como una reescritura estilo CLUSTER). Y desde 8.1 (2005), autovacuum lo ejecuta solo según umbrales de tuplas muertas — la anécdota del §16.0.
- **SQLite**: `VACUUM` reconstruye la base entera en una temporal y la copia de vuelta — offline y radical. Más interesante aún: `PRAGMA auto_vacuum` mantiene **páginas de punteros (ptrmap)** que registran quién apunta a cada página, justo la capa de indirección que LiraDB NO tiene y que por eso puede truncar el fichero al liberar. Nos lo ganaron… pagando la indirección en cada acceso.
- **RocksDB/LevelDB**: compaction continua en background, minor y major (doc/impl.html de LevelDB los nombra así), online con el sistema sirviendo — al precio de CPU/IO permanente y write stalls.
- **Online vs offline**: PostgreSQL VACUUM es online (locks suaves); VACUUM FULL y SQLite VACUUM son efectivamente offline; RocksDB es online-con-costo. LiraDB elige offline explícito: conexión única, compact como operación de ventana — coherente con ser embebida y con no tener aún concurrencia (capítulo 30).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `VACUUM` de PostgreSQL no devuelve espacio al sistema operativo, igual que nuestro `compact` no encoje el fichero?
- *Intermedio*: SQLite con `auto_vacuum` puede mover páginas porque mantiene un ptrmap. ¿Qué le pasa al coste de cada acceso a página? ¿Qué estructura de LiraDB habría que tocar para imitarlo?
- *Experto*: RocksDB leveled con niveles de 10×: si escribes 1 GiB de datos nuevos, ¿cuántas veces se reescribe cada byte hasta reposar en el último nivel? ¿Cómo cambia esa cifra con compaction tiered?

## 16.12 Qué hemos sacrificado

1. **El fichero no encoge** — la limitación estrella, herencia directa de `PageId = offset físico` (cap. 12). Deuda declarada, casa en caps. 29/36.
2. **Sin fusión entre páginas**: repack no mueve records de una página medio vacía a otra. Los datos siguen donde estaban; solo el interior de cada página queda ordenado.
3. **Sin checksum fuerte**: `check` valida estructura (magic, ids, truncados), no contenido — un bit volteado dentro de un record válido sigue siendo indetectable sin CRC (capítulo 10).
4. **Offline**: no hay compactación con el grafo sirviendo. En un sistema embebido monousuario es aceptable; en uno concurrente habría que esperar al capítulo 30.

## 16.13 Lo que te llevas

- **Tres herramientas, tres contratos**: `inspect` mide (tolerante), `check` diagnostica (read-only, exhaustivo), `compact` repara (conservador, in-place).
- **El header puede mentir; el contenido no**: deriva del contenido (`free_space()`, `inspect`), reconcilia con `repack`.
- **Repack in-place, no vacuum**: la dirección del estante es pública (CSR, buckets, B+tree apuntan a ella); el interior de la caja es privado.
- **Mantenimiento offline = leer sin pool**: administración ≠ hot path; no contamines la caché (capítulo 13).
- **`fragmentation_ratio` es una métrica de negocio**: no dice «corre compact», dice «esta flash está comprando aire».
- **LSM es la alternativa seria** (write-optimized, inmutable, compaction) con precios reales (read amplification, CPU de fondo) — ADR-005 la deja fuera con criterio, no con miedo.

## 16.14 Ojo, cuidado con…

- **Esperar que `compact` libere disco**: no. Nunca. Repack in-place; el fichero mide lo mismo antes y después (`bytes_on_disk` idéntico en `stats_before`/`stats_after`).
- **Confundir `free_space` declarado y real**: el del header es un cache; el real se deriva. `check` compara, `repack` reconcilia, `inspect` solo cree en el segundo.
- **Contar length-prefixes al medir**: `bytes_used` sigue la aritmética de `SlottedPage::free_space()` (sin prefijos). Dos tests nuestros nacieron mal por esto.
- **Olvidar la MetaPage con `FilePager`**: `create` deja la página 0 a ceros; sin escribirla, `check` reporta `BadMagic` en la página 0 (nos pasó — helper `filepool_with_meta`).

## 16.15 Pin de batalla

> *«Un diagnóstico que también repara no es un diagnóstico: es una coartada. Primero la linterna, luego la llave.»*

## 16.16 Si solo lees 30 segundos

`inspect` cuenta lo que HAY en cada página (no lo que dicen las cabeceras) y lo resume en `fragmentation_ratio()`. `check` verifica las 4 invariantes del formato —magic, page_id, records, free_space— y reporta cada fallo tipado (`IssueKind`) sin tocar nada. `compact` repackea cada página IN-PLACE: records consecutivos, `free_space` veraz, basura a cero, mismo `PageId` — porque los `PageId` son offsets físicos y mover páginas rompería el CSR y los índices. El fichero no encoje: el vacuum es deuda documentada. Y los LSM-trees son la alternativa write-optimized que pagaría ese milagro con lecturas más lentas y una compaction comiéndose la CPU.

## 16.17 Una historia pequeña

Ana dejó su tesis corriendo toda la noche: cada hora, un script recalculaba el grafo de citas y rebuildaba el CSR. A los tres meses el fichero de 200 MiB iniciales marcaba 4 GiB y el disco del laboratorio estaba a puntito de llenarse. «Fuga de memoria», dictaminó alguien. No: eran las páginas huérfanas de cada rebuild, esperando en un fichero que solo sabe crecer. El primer `inspect` lo mostró en una línea: `fragmentation_ratio = 0.93`. No había nada que reparar — `check` estaba limpio, `compact` fue idempotente — pero la cifra convertió un misterio en una decisión: reconstruir el fichero por export/import y agendar la conversación sobre el vacuum pendiente. Ese día aprendimos que una base de datos necesita tres verbs más de los que enseña el tutorial: mirar, escuchar y ordenar.

**Preguntas que dejamos abiertas** (las responde la Parte IV): ahora que el motor guarda, indexa y se mantiene solo — ¿cómo se le pregunta sin compilar Rust contra él? ¿Qué aspecto tiene un lenguaje de consulta diminuto, y quién decide qué índice usar?

## Ejercicios resueltos

**1. Un fichero tiene 12 páginas de datos (más la metapágina), cada una con 3 records de 40 bytes, cabeceras de 10 bytes y sin basura. ¿Qué reportan `bytes_used`, `bytes_on_disk`, `fragmentation_ratio` y `utilization`?**

Por página de datos: `used = 10 + 3×40 = 130` bytes (recuerda: sin length-prefixes). Metapágina: `10 + 12 = 22`. `bytes_used = 22 + 12×130 = 1.582`. `bytes_on_disk = 13 × 4.096 = 53.248`. `bytes_free = 53.248 − 1.582 = 51.666`. `fragmentation_ratio = 51.666 / 53.248 ≈ 0.970`; `utilization ≈ 0.030`. ¿Conclusión de negocio? Ojo: un ratio altísimo con pocas páginas escritas NO es alarma — es un fichero recién nacido con páginas casi vacías. La métrica mide densidad; la decisión exige compararla en el tiempo (tendencia) o contra el nº de páginas. Verifica la aritmética con `inspect_counts_records_and_pages` como patrón.

**2. La página 5 tiene `FreeSpaceMismatch { declared: 9.000, actual: 3.970 }`. ¿Qué hizo exactamente `repack_page`? ¿Y si en vez de eso el header dijera `page_id = 990`?**

En el primer caso: decode OK, re-insert de los records en una `SlottedPage` nueva, y escritura al MISMO `PageId` con `free_space = 3.970`. El `RepackResult` devuelve `free_before: 9.000`, `free_after: 3.970`, `bytes_reclaimed: 5.030`, `modified: true` (patrón del test `repack_page_corrije_free_space_corregido_a_disco`). En el segundo caso, `page_id = 990` con la página en offset 5 es `PageIdMismatch`: repack escribiría una página CONSISTENTE pero seguiría clavada en el offset físico 5 con su header corrigiendo a 5 — el issue desaparece del reporte… porque la página nunca se movió: el header mentía y el repack lo alineó con el offset real. La lección: `PageIdMismatch` casi siempre significa «header corrupto», no «página movida» — pero si ALGUIEN moviera páginas de verdad (un vacuum casero), este issue sería el primero en chivarlo.

## Ejercicios propuestos

**Esencial (recordar + aplicar).** Sin ejecutar nada: escribe en papel el `CheckReport` exacto —variantes y campos— tras (a) machacar los bytes 8..10 de la página 1 con `1.234`, y (b) machacar los bytes 0 y 1 de la página 2 con `0x11`. ¿Cuenta `pages_checked` esas páginas? ¿Y si además liberaras la página 3? Después verifica con `check_detects_free_space_mismatch` y `check_detects_bad_magic`. *Pistas*: (1) ¿en qué bytes del `PageHeader` vive cada campo?; (2) ¿qué magic espera `check` fuera de la página 0?; (3) ¿qué hace el bucle con `is_allocated == false`? *Criterio*: acertar variante Y campos (`declared/actual`, `expected/got`) Y el `pages_checked` final.

**Intermedio (analizar).** Implementa `doctor(pool) -> (CheckReport, u32)`: llama a `check`, y por cada issue `FreeSpaceMismatch` ejecuta `repack_page`; devuelve el informe ORIGINAL y cuántas páginas tocó. ¿Por qué las `BadMagic` quedan fuera por diseño? ¿Qué devuelve `compact` para esas mismas páginas en `pages_skipped`? *Pistas*: (1) ¿qué variante de `MaintenanceError` devuelve `repack_page` ante una página no decodificable?; (2) ¿quién debe decidir sobre un posible fallo de disco?; (3) mira el `match` de `compact`: ¿qué errores escala y cuáles salta? *Criterio*: el doctor jamás escribe en una página que no decodifica, y el test de regresión es el patrón de `compact_corrige_free_space_y_mejora_stats`.

**Experto (crear — cierre de la Parte III, retrieval puro).** «La mudanza completa»: cierra el manuscrito y, SIN mirar los capítulos 11-15, lista desde memoria TODAS las estructuras de LiraDB que guardan un `PageId`. Después: en un fichero con un hash de 64 buckets, 3 páginas de desbordamiento encadenadas y un B+tree de 3 nodos internos (fanout ~64), estima cuántos punteros hay que reescribir para mover la página 7 al hueco de la 2. Diseña en papel el orden seguro: ¿qué se reescribe primero, qué garantiza atomicidad, cuándo se trunca? *Pistas*: (1) empieza por la página 0; (2) ¿quién apunta a los buckets y quién a las hojas del árbol?; (3) ¿qué sección de este capítulo cuantificó exactamente esto? *Criterio*: ≥5 familias de punteros (metapágina, CsrHeader ×4, bucket_starts, next_page de overflow, hijos de nodos internos), la cifra ~260 del §16.9 bien razonada, y un orden del tipo snapshot → hoja→raíz → swap atómico → truncate. Contraste final: ¿qué `IssueKind` delataría una mudanza hecha a medias?

## Para profundizar

- **PostgreSQL** — «Routine Vacuuming» (docs, cap. de mantenimiento routine) y las release notes de la 8.1.0 (postgresql.org/docs/release/8.1.0): la anécdota del autovacuum, contada por sus autores.
- **LevelDB** — `doc/impl.html` (impl.md en el repo): define literalmente *minor* y *major compaction*; el documento LSM más claro que existe.
- **RocksDB** — wiki: «Basic Compaction» y «Universal Compaction» para leveled vs tiered con números de amplificación.
- **Alex Petrov, «Database Internals»** (O'Reilly): la Parte II entera es LSM (memtable, SSTable, compaction) y la Parte I compara con B-trees in-place — las dos mitades de este capítulo, con el detalle de producción.
- **Martin Kleppmann, «DDIA»**, cap. 3: la comparación B-tree vs LSM con los trade-offs de write/read amplification.
- **O'Neil et al., 1996**, «The Log-Structured Merge-Tree» (SIGMOD): el paper original del que todo lo anterior es implementación.
- **SQLite** — `fileformat2.html` (freelist y el ptrmap de `auto_vacuum`) y `lang_vacuum.html`: cómo se gana el derecho a truncar.

## Mini-diálogo: la sala de máquinas

> — A ver si lo pillo. ¿Construímos un motor entero en seis capítulos y el resumen es que el fichero se ensucia y no se puede limpiar del todo?

> — Se puede limpiar DEL TODO, no HOY. La diferencia la pone una palabra que ya conoces: punteros. El día que tengamos WAL y snapshots, reescribirlos todos será rutina. Hoy sería corrupción con buena intención.

> — Y el vecino RocksDB ni se inmuta: escribe secuencial, compacta en background, nadie sufre.

> — Nadie sufre… hasta que miras su CPU en producción y ves la compaction comiéndose un núcleo para siempre, o el primer pico de escritura que la desborda. El LSM no elimina el desorden: lo cambia de dueño. Nosotros lo dejamos a mano, en una herramienta que entiendes de punta a punta.

> — ¿Y si mañana escribo más de lo que leo?

> — Entonces relee el §16.10, revisa el ADR-005 y dime qué cambiarías. Ese ejercicio —no el código— es lo que cierra de verdad la Parte III.

---

*(Próximo capítulo: 17 — Diseñar un lenguaje pequeño. El motor ya guarda, indexa y se mantiene; ahora toca la pregunta que el usuario lleva seis capítulos esperando hacer: MATCH-WHERE-RETURN, el nacimiento de LiraQL.)*
