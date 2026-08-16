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

# Capítulo 11 — Páginas, bloques y slotted pages

> *«El disco no lee bytes. El disco lee bloques. Todo lo demás es una mentira piadosa que nos contamos para dormir tranquilos.»*

## 11.0 La anécdota de la esquina

En 1979, un programador de IBM llamado Jim Gray estaba tratando de explicar por qué cierta base de datos era absurdamente lenta a pesar de tener "solo" un millón de registros. El problema no estaba en el CPU —el CPU hacía lo suyo en microsegundos—, sino en algo que nadie quería admitir: **cada vez que el programa pedía un registro, el disco se movía**. No un poco. Se movía *físicamente*: un brazo mecánico se desplazaba hasta la pista correcta, esperaba a que el plato girara hasta el sector correcto, y solo entonces leía.

Ese viaje mecánico tarda unos 5-10 milisegundos. Suena a poco, hasta que haces la cuenta: si lees los registros **de uno en uno, en posiciones dispersas del disco**, un millón de registros te cuesta varios días. Si los lees **agrupados en bloques contiguos**, el mismo millón de registros cabe en horas o minutos, porque el brazo se mueve una vez y arrastra miles de registros de golpe.

De ahí nace la idea más importante del almacenamiento de bases de datos: **la página**. No es una idea glamurosa. No tiene nombre de algoritmo. Pero sin ella, nada de lo que construiremos en LiraDB funcionaría. Este capítulo es el cimiento de todo el motor de almacenamiento.

## 11.1 Objetivo

Al terminar este capítulo sabrás **por qué una base de datos no escribe registros sueltos en un fichero**, y habrás implementado la estructura que lo resuelve: la **slotted page** — una página de tamaño fijo que puede guardar registros de longitud variable sin malgastar espacio ni corromperse.

En concreto, vas a construir tres piezas:

1. `PageHeader` — los 10 bytes de cabecera que identifican cada página.
2. `SlottedPage` — la página con registros de tamaño variable.
3. `MetaPage` — la página 0, que guarda el catálogo del fichero.

## 11.2 Problema

Imagina que ya tienes el encoding del capítulo 9. Sabes convertir un `Node` en un `Vec<u8>`. La pregunta suena tonta: **¿dónde guardas esos bytes en disco?**

La respuesta ingenua es: *"los escribo uno detrás de otro, en orden"*. Y funciona... hasta que deja de funcionar. Veamos por qué, con números:

- El disco (o el SSD, o el sistema operativo) **no lee bytes individuales**. Lee **bloques**: normalmente 512 bytes, 4096 bytes, o múltiplos. Si pides un solo byte, te llega el bloque entero, lo quieras o no.
- Si guardas registros de longitud variable uno tras otro (`[A][BB][CCC][DDDD]...`), para encontrar el registro número 7.482 tienes que **leer desde el principio y sumar longitudes**. Esa operación es O(n) y, peor, obliga a leer del disco fragmentos dispersos.
- Si borras un registro del medio, queda un hueco. ¿Lo rellenas? ¿Con qué? ¿Y si el hueco mide 3 bytes y el nuevo registro 100?

La raíz del problema: **mezclamos dos decisiones que deberían estar separadas** — *dónde* están los datos en disco (que debe ser fijo y predecible) y *cómo se organizan dentro de cada unidad* (que puede ser flexible).

La base de datos real separa ambas cosas. La unidad fija se llama **página**; la organización interna flexible se llama **slotted page**. Vamos a verlas.

## 11.3 Modelo mental

Piensa en un **libro de contabilidad con páginas numeradas**:

- El libro tiene páginas **todas del mismo tamaño** (digamos 4.096 caracteres). No importa lo que pongas dentro: cada página ocupa exactamente lo mismo.
- Para llegar a la página 500, no lees las 499 anteriores: **vas directamente a la posición `500 × 4096`**. Eso es aritmética, no búsqueda.
- Dentro de cada página, el contable anota los asientos **apretados pero con un índice al principio** que dice "el asiento 1 empieza en la línea 12, el 2 en la 40, el 3 en la 55...".

Esa es exactamente nuestra estructura:

```
Página (4.096 bytes, tamaño FIJOO):
┌──────────────────────────────────────────────────────┐
│ PageHeader (10 bytes): magic | tipo | id | n_rec | libre │
├──────────────────────────────────────────────────────┤
│ [4 bytes: len=6][A B C D E F]                        │  ← registro 1
│ [4 bytes: len=3][G H I]                              │  ← registro 2
│ [4 bytes: len=5][J K L M N]                          │  ← registro 3
│ ...                                                  │
│                    (espacio libre)                   │
└──────────────────────────────────────────────────────┘
```

Dos ideas clave se esconden aquí:

1. **Tamaño fijo de página ⇒ acceso directo.** La página N está en el byte `N × 4096`. Para leerla, haces `seek(N * 4096)` y lees 4096 bytes. No dependes de lo que haya dentro de las páginas anteriores.
2. **Longitud variable de registro ⇒ sin desperdicio.** Cada registro guarda su propia longitud como prefijo, así que no hay que rellenar todos los registros hasta un tamaño máximo ficticio.

La tercera idea, que veremos cuando hablemos del `Pager` (capítulo 12), es que **la página es la unidad de caché**: cuando el buffer pool (capítulo 13) decide qué guardar en memoria, guarda páginas enteras, no registros.

### ¿Por qué 4.096 bytes?

`PAGE_SIZE = 4096` no es casualidad. 4.096 es:

- **Múltiplo del tamaño de sector** (512) y del tamaño de página del sistema operativo (también 4 KB en la inmensa mayoría de sistemas). Si tu página no está alineada con la del sistema operativo, cada lectura puede convertirse en *dos* lecturas físicas.
- **Suficientemente grande** para que un `seek` + lectura traiga un buen montón de datos útiles de una vez (amortiza el coste del movimiento del disco).
- **Suficientemente pequeña** para que no malgastes memoria cacheando una página gigante cuando solo querías un registro.

No es un número sagrado: PostgreSQL usa 8 KB por defecto, SQLite 4 KB, y muchos sistemas permiten configurarlo entre 4 KB y 64 KB. El punto no es el número exacto, sino **que sea fijo y conocido por todo el sistema**. En LiraDB elegimos 4.096 y lo declaramos como constante única (`PAGE_SIZE`): si mañana quieres 8 KB, cambias una línea y nada más.

## 11.4 Primera solución

Empecemos por lo más simple que puede funcionar: **una página que guarda registros de longitud fija**.

```rust
// Solución ingenua: registros de longitud FIJA.
// Cada registro ocupa 100 bytes, haya o no datos.
struct FixedPage {
    records: Vec<[u8; 100]>,
}
```

Para leer el registro `i`, calculas `offset = i * 100` y lees 100 bytes. Fácil. Rápido. Predecible.

Los tests pasan. Los registros cortos ("Ana", 3 bytes) se guardan en un cajón de 100 bytes. Funciona, y durante un rato nadie se queja.

## 11.5 Sus límites

Hasta que alguien guarda un `Value` con una lista de 500 elementos, o un `string` de 10.000 caracteres. Y entonces la solución de longitud fija se enfrenta a un muro:

1. **¿Qué tamaño máximo eliges?** Si pones 100, los registros de 10.000 no caben. Si pones 10.000, cada "Ana" de 3 bytes te cuesta 10.000 bytes en disco — un desperdicio del 99,97 %.
2. **El tamaño de tus datos no es constante.** Un `Node` puede tener cero propiedades o cincuenta. Una arista de un grafo de redes sociales no mide lo mismo que la descripción de un paper.
3. **No hay forma de ganar:** cualquier longitud fija es, o demasiado pequeña (no cabe todo), o demasiado grande (desperdicias disco).

La conclusión duele: **los datos de un grafo son de longitud variable por naturaleza**, y una página de longitud fija no sirve. Necesitamos una página que:

- guarde registros de tamaño arbitrario,
- permita leer el registro `i` sin escanear toda la página,
- y detecte si algo se corrompió.

Esa es la **slotted page**.

## 11.6 Solución evolucionada

La idea de la slotted page tiene décadas (aparece en sistemas de los años 70-80 y sigue siendo la base de PostgreSQL, SQLite, InnoDB y casi cualquier motor serio). Es esta:

**Cada registro va precedido de un prefijo de 4 bytes con su longitud.** Para leer los registros, caminas por la página siguiendo las longitudes:

```
leer registro i:
    pos = 10                      # saltamos la cabecera
    repetir i veces:
        len = leer_u32(pos)        # longitud del siguiente registro
        pos += 4 + len             # saltamos prefijo + datos
    # aquí pos apunta al registro i
    return bytes[pos..pos+len]
```

Esto nos da:

- **Longitud variable**: cada registro mide exactamente lo que necesita + 4 bytes de prefijo.
- **Lectura O(i)**: para llegar al registro `i` saltas exactamente `i` registros, leyendo solo sus prefijos (barato). No escaneas *el contenido* de los anteriores.
- **Sin tabla de offsets separada**: el "índice" y los datos conviven en el mismo flujo. (Una alternativa — tabla de offsets al final de la página — existe, pero para nuestro primer motor el prefijo es más simple y basta.)

La cabecera (`PageHeader`) ocupa 10 bytes al principio y guarda lo imprescindible:

| Byte(s) | Campo | Para qué |
|---|---|---|
| 0 | `magic` | Byte de tipo (0xDA datos, 0xFE meta). Sirve de firma: si no es el esperado, la página está corrupta o no es lo que crees. |
| 1 | `page_type` | Redundante con `magic`: si ambos no coinciden, hay corrupción (autochequeo barato). |
| 2-5 | `page_id` (u32) | Qué página es. Te permite detectar "me han dado la página equivocada". |
| 6-7 | `num_records` (u16) | Cuántos registros hay. Para saber cuándo parar de leer. |
| 8-9 | `free_space` (u16) | Bytes libres. Para saber si un registro cabe sin tener que probar. |

Fíjate en un detalle sutil: `magic` y `page_type` guardan **el mismo valor** (0xDA o 0xFE). Es redundancia deliberada: si un solo bit se corrompe en cualquiera de los dos bytes, `decode` detecta el desajuste y devuelve error en lugar de entregarte datos basura. Es un checksum de un byte, gratuito.

Y aquí está la pieza clave — **por qué el prefijo de longitud y no un marcador de fin**:

Una alternativa popular es terminar cada registro con un byte especial (un "end-marker", como el `\0` de C). El problema: ¿qué pasa si tus datos *contienen* ese byte especial? Tienes que escaparlo, y eso envenena todo el pipeline: cada vez que guardas o lees, hay que escapar/desescapar, y un escape mal hecho es una corrupción silenciosa.

Con el **prefijo de longitud**, no hay byte prohibido: el registro puede contener *cualquier* secuencia de bytes, incluido el valor 0x00, porque sabemos su longitud exacta de antemano. Es la misma razón por la que los protocolos de red serios usan length-prefix en vez de delimitadores.

## 11.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap11_slotted_pages.rs`. Vamos a leerlo por partes, porque cada línea tiene un porqué.

### La cabecera

```rust
pub const PAGE_SIZE: usize = 4096;

pub struct PageHeader {
    pub page_id: u32,
    pub page_type: PageType,
    pub num_records: u16,
    pub free_space: u16,
}

pub enum PageType {
    Data = 0xDA,
    Meta = 0xFE,
}
```

`PageType` es un enum con `#[repr(u8)]` y valores explícitos 0xDA/0xFE. Elegimos esos valores por dos razones: son fáciles de reconocer en un volcado hexadecimal, y son suficientemente "raros" como para que sea improbable encontrarlos por casualidad en datos aleatorios. (Elegir valores con todos los bits puestos y no puestos alternados — como 0xDA y 0xFE — es una heurística de defensa: un byte de datos aleatorios rara vez cae exactamente en ellos.)

Fíjate en el orden de los campos de codificación: es **little-endian**, igual que el resto del capítulo 9. Lo declaramos explícitamente con `to_le_bytes()` / `from_le_bytes()`. Nunca uses la representación nativa de memoria (`to_ne_bytes`): el día que abras tu fichero en una máquina big-endian, se corrompería todo en silencio. El formato en disco debe ser **independiente de la máquina**.

### La página con slots

```rust
pub struct SlottedPage {
    pub header: PageHeader,
    pub(crate) records: Vec<Vec<u8>>,
}
```

En memoria, `records` es un simple `Vec<Vec<u8>>`. En disco, se codifica como el flujo con prefijos de longitud que describimos. La operación de insertar es la que más nos interesa:

```rust
pub fn insert(&mut self, record: &[u8]) -> Option<usize> {
    if record.len() > self.free_space() {
        return None;              // no cabe: la página está llena
    }
    let offset = PageHeader::SIZE + self.records.iter().map(|r| r.len()).sum::<usize>();
    self.records.push(record.to_vec());
    self.header.num_records += 1;
    self.header.free_space = self.free_space() as u16;
    Some(offset)
}
```

Tres cosas importantes:

1. **Devolvemos `Option`**: `None` significa "no cabe". No lanzamos error ni lo metemos a la fuerza. El `Pager` del capítulo 12 decidirá qué hacer con ese `None` (normalmente: buscar otra página o asignar una nueva).
2. **El offset se calcula sumando longitudes**: es la posición donde *empezaría* el registro si lo insertáramos. Por eso es O(n) en número de registros — aceptable para una página de 4 KB, que jamás tendrá millones de registros.
3. **`free_space` se recalcula**: lo derivamos, no lo mantenemos "a ojo". Así la cabecera siempre dice la verdad sobre cuánto queda.

### La metapágina

La página 0 es especial: es la **metapágina**. No guarda datos de usuario, guarda el **catálogo del fichero**: cuántas páginas hay, cuántas están libres, y dónde está la raíz de los índices.

```rust
pub struct MetaPage {
    pub header: PageHeader,
    pub num_pages: u32,
    pub free_pages: u32,
    pub root_page: u32,
}
```

¿Por qué separar la metapágina de las páginas de datos? Porque **necesitas saber cómo leer el fichero antes de poder leer el fichero**. Es el problema del huevo y la gallina: para encontrar el catálogo de índices necesitas la metapágina; para encontrar la metapágina... bueno, esa es fácil, **siempre está en la página 0**. Es una convención que resuelve el arranque en frío: cuando LiraDB abre un fichero, lo primero que lee es la página 0, y desde ahí descubre todo lo demás.

## 11.8 Prueba de fuego

La prueba de fuego no es "los tests pasan" — es **"el roundtrip sobrevive a la realidad"**. Codificar una página y decodificarla debe devolver exactamente lo mismo, incluso con registros de distinto tamaño:

```rust
let mut p = SlottedPage::new(7, PageType::Data);
p.insert(b"hello").unwrap();
p.insert(b"world!").unwrap();
p.insert(b"LiraDB").unwrap();   // tres registros de distinta longitud

let enc = p.encode();
let dec = SlottedPage::decode(&enc).unwrap();
assert_eq!(p, dec);             // idénticos, byte a byte
```

Y los casos de fallo son igual de importantes que el camino feliz:

```rust
// Un registro gigante no cabe, y no lo metemos a la fuerza:
let huge = vec![0u8; PAGE_SIZE - PageHeader::SIZE];
assert!(p.insert(&huge).is_some());     // justo cabe
assert!(p.insert(b"extra").is_none());  // ya no cabe: devuelve None

// Una página corrupta (magic no coincide) se DETECTA:
let mut bytes = [0u8; 10];
bytes[0] = 0xAA;   // magic corrupto
bytes[1] = 0xDA;   // page_type correcto
assert!(PageHeader::decode(&bytes).is_err());
```

Estos tres tests (`slotted_page_con_records`, `slotted_page_record_no_cabe`, `page_header_magic_mismatch`) encapsulan las tres promesas de la slotted page: **roundtrip fiel, no desbordar, detectar corrupción**. Si este capítulo se te olvidara, tu base de datos escribiría registros de longitud variable en un fichero plano y — tarde o temprano — leería basura de una posición mal calculada sin que nadie se enterara. Ese es el síntoma: **corrupción silenciosa al borrar o al insertar registros de distinto tamaño**.

## 11.9 Qué hemos sacrificado

Toda estructura tiene un precio. La slotted page no es gratis:

1. **4 bytes de sobrecarga por registro** (el prefijo de longitud). Para registros diminutos, eso puede ser proporcionalmente mucho. Existe una alternativa — la tabla de slots al final de la página — que guarda los offsets en un array compacto, pero la dejamos fuera por simplicidad; es material de "cómo lo hace una BBDD real".
2. **Sin reutilización de huecos**: si borras un registro del medio, ese espacio queda muerto hasta que compactes la página (eso es, justamente, el capítulo 16, `liradb compact`). Por ahora, `SlottedPage` no tiene borrado individual — los registros solo crecen.
3. **Fragmentación interna**: los últimos bytes de una página casi nunca se usan por completo (un registro de 100 bytes no cabe en 90 libres, y esos 90 quedan ahí). Es el precio de la simplicidad; se recupera con compactación.
4. **Sin checksum real**: el magic de 1 byte detecta desajustes groseros, pero no una corrupción de bits *dentro* de un registro. Un CRC real (como el del capítulo 10) en la cabecera sería el siguiente paso; los sistemas serios lo hacen.

## 11.10 Cómo lo hace una BBDD real

La slotted page es, con variantes, **lo que usan casi todas**:

- **PostgreSQL** usa páginas de 8 KB con una cabecera y un **array de punteros al final de la página** (los "item pointers" crecen desde el final hacia atrás, los datos desde el principio hacia delante, y se encuentran en el medio). Eso le permite **borrar y reutilizar espacio** sin compactar: un puntero puede quedar "muerto" y reaprovecharse.
- **SQLite** usa páginas de 4 KB (por defecto) y guarda los registros como **celdas** con un formato de longitud variable muy similar al nuestro, con una cabecera de "cell pointers" que apunta al contenido.
- **InnoDB (MySQL)** usa páginas de 16 KB con una estructura de slots al final, pensada para el árbol B+.

En todos, el patrón es el mismo que has construido: **cabecera + registros de longitud variable + metadatos que permiten navegar sin escanear**. Lo que cambia es el detalle (tabla de slots vs prefijo) y la cantidad de metadatos extra (checksums CRC, `lsn` de transacciones, punteros al árbol).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en nuestra página, ¿por qué `insert` es O(n) en registros? ¿Qué estructura de datos haría O(1) el cálculo del offset?
- *Intermedio*: implementa el borrado de un registro de una `SlottedPage` compactando los siguientes hacia la izquierda. ¿Qué pasa con los offsets que devolviste antes de borrar?
- *Experto*: rediseña `SlottedPage` para usar una **tabla de slots al final** (offsets creciendo desde el final, datos desde el principio). Compara la sobrecarga de espacio con el prefijo de longitud para registros de 5, 50 y 500 bytes.

## 11.11 Lo que te llevas

- La **página** es la unidad de almacenamiento: tamaño fijo, acceso directo por aritmética (`N × PAGE_SIZE`).
- La **slotted page** resuelve el problema de los registros de longitud variable: prefijo de longitud + cabecera con metadatos.
- **Length-prefix > end-marker**: el prefijo permite cualquier byte en los datos y evita la corrupción silenciosa por escapes.
- La **metapágina** (página 0) resuelve el arranque en frío: siempre sabes dónde empezar a leer.
- **Little-endian explícito** en disco: el formato es independiente de la máquina.
- La **redundancia barata** (magic duplicado) es tu primera línea de defensa contra la corrupción.

## 11.12 Ojo, cuidado con…

- **Confundir página con registro**: la página es la unidad de disco; el registro es lo que vive dentro. Casi todos los errores de diseño vienen de mezclarlas.
- **`free_space` desincronizado**: si no lo derivas del contenido real, acabarás mintiendo sobre cuánto queda y aceptando registros que no caben. Derívalo, no lo "lleves en la cabeza".
- **Endianness**: usar `to_ne_bytes` en vez de `to_le_bytes` funciona en tu portátil y corrompe todo en cualquier otra arquitectura. El formato de disco debe ser explícito.
- **`PAGE_SIZE` repartido por el código**: si el 4096 aparece "a pelo" en diez sitios, cambiarlo será una pesadilla. Una sola constante, referenciada en todas partes.

## 11.13 Pin de batalla

> *«Si tu formato en disco depende de la máquina que lo escribió, no tienes una base de datos. Tienes un accidente esperando a ocurrir.»*

## 11.14 Si solo lees 30 segundos

La base de datos no escribe registros sueltos: escribe **páginas de tamaño fijo** (4 KB en LiraDB). Dentro de cada página, los registros de longitud variable van con un **prefijo de longitud** y una **cabecera** que identifica la página y su espacio libre. La **página 0** es la metapágina: el catálogo que te dice cómo leer el resto. Esa es toda la magia del almacenamiento.

## 11.15 Una historia pequeña

Cuando arrancamos LiraDB por primera vez, antes de este capítulo, "guardar en disco" significaba `fs::write("graph.bin", bytes)` y rezar. Funcionó hasta que Ana intentó guardar un grafo con una descripción de 8 KB en un nodo. El fichero se corrompió en silencio: los registros siguientes se leían desde el byte equivocado, y el grafo entero se convirtió en basura con aspecto de grafo válido. Tardamos una tarde en darnos cuenta de que el fallo no estaba en el algoritmo, sino en que **nunca habíamos decidido dónde termina un registro y empieza el siguiente**. La slotted page fue la respuesta: ahora, hasta un byte de corrupción se detecta, porque la longitud de cada registro está escrita delante de él, en tinta indeleble.

## Ejercicios resueltos

**1. ¿Cuántos registros de exactamente 100 bytes caben en una página de 4.096 bytes?**

Cada registro ocupa 4 (prefijo) + 100 (datos) = 104 bytes. La cabecera ocupa 10. Espacio útil = 4.096 − 10 = 4.086 bytes. Número de registros = 4.086 ÷ 104 = 39,28… → caben **39 registros** (39 × 104 = 4.056 ≤ 4.086; el registro 40 necesitaría 4.160 > 4.086 y no cabe). Sobran 4.086 − 4.056 = 30 bytes, que quedan como fragmentación interna. Puedes verificar esto ejecutando un test que inserte registros de 100 bytes y cuente cuántos devuelven `Some`.

**2. ¿Por qué `decode` puede confiar en `num_records` para saber cuántos registros leer?**

Porque `num_records` se escribe en la cabecera en el momento de codificar, junto con el resto de la página. Si la página no está corrupta, `num_records` es exactamente el número de registros codificados. Si está corrupta y el número es demasiado grande, `decode` se sale de los 4.096 bytes y devuelve `Err("record truncated")` — nunca devuelve datos incompletos como si fueran válidos. Es un ejemplo de **validación defensiva**: el formato lleva su propio límite y lo comprueba.

## Ejercicios propuestos

**Esencial.** Modifica `SlottedPage::insert` para que, además del offset, devuelva cuántos bytes de `free_space` quedan tras insertar. Escribe un test que inserte varios registros y compruebe que `free_space` decrece exactamente en `4 + len` cada vez.

**Intermedio.** Implementa `SlottedPage::delete(index)` que borre el registro `i` y compacte los siguientes (moviendo sus bytes hacia la izquierda). ¿Qué ocurre con `free_space`? ¿Por qué los offsets devueltos por `insert` anteriores al borrado dejan de ser válidos?

**Experto.** Diseña una variante con **tabla de slots al final de la página**: los datos crecen desde el principio, la tabla de offsets crece desde el final, y `free_space` es el hueco central. Implementa `insert` y `decode`, y escribe un test de roundtrip. Compara el espacio total ocupado por 10 registros de 10 bytes en tu diseño vs. el diseño de length-prefix.

## Para profundizar

- **"Database Internals" (Alex Petrov)** — capítulos 2 y 3: layouts de página, slotted pages, B-tree pages. Es el mapa completo del territorio que acabamos de pisar.
- **"Designing Data-Intensive Applications" (Martin Kleppmann)** — capítulo 3: por qué el disco es lento y cómo las páginas y los B-tree lo mitigan.
- **CMU 15-445 (Intro to Database Systems)** — lecciones de storage, con el proyecto BusTub donde implementas un buffer pool sobre páginas exactamente como estas.
- **Código fuente de SQLite** (`btreeInt.h`, `pager.c`): la implementación real de slotted pages y del pager, comentada con un detalle exquisito.

## Mini-diálogo: en guardia nocturna

> — O sea, que "página" es solo un array de 4.096 bytes. ¿Y por eso un capítulo entero?
>
> — Porque ese array de 4.096 bytes es la frontera entre "esto funciona en mi portátil" y "esto funciona en un disco de verdad". Todo lo que construyas encima —el buffer pool, el CSR, los índices, el WAL— asume que puede leer y escribir páginas enteras sin sorpresas.
>
> — Pero entonces, ¿no es un poco... aburrido?
>
> — Lo aburrido es lo que no falla. Las estructuras glamurosas fallan estrepitosamente si el cimiento miente. La slotted page no miente: te dice cuánto hay, dónde empieza cada cosa, y si alguien la pisó. Con eso, ya puedes construir de noche sin miedo.

---

*(Próximo capítulo: 12 — El gestor de páginas. Aquí la página existía como idea; ahora veremos quién la escribe, la lee y decide cuándo está libre — el trait `Pager`.)*
# Capítulo 12 — El gestor de páginas (trait `Pager`)

> *«Todo el mundo quiere escribir páginas. Nadie quiere llevar la cuenta de cuáles están libres.»*

## 12.0 La anécdota de la esquina

Año 2000. D. Richard Hipp trabaja como contratista para Bath Iron Works, el astillero de Maine que construye destructores de la clase Arleigh Burke. Su equipo escribe software para el USS Oscar Austin (DDG-79), y los datos de tuberías y válvulas del barco viven en una base de datos Informix instalada en un servidor. El problema: cuando el servidor se caía —y en un barco las cosas se caen—, la aplicación solo sabía mostrar «no puedo conectar», y el equipo de Hipp cargaba con la culpa de un fallo que no controlaba. En un destructor, donde las decisiones de control de daños no esperan a nadie, no hay administrador de sistemas que reinicie el servidor de bases de datos en mitad del humo.

Hipp se hizo la pregunta que lo cambió todo: si su aplicación apenas leía datos y los metía en RAM, ¿para qué necesitaba un servidor? Lo que quería era una base de datos sin dependencias que pudieran fallar: incrustada en el proceso, sin instalación, sin administración. Escribió SQLite. Y si hoy abres el diagrama de arquitectura de SQLite, entre la capa B-tree y la interfaz con el sistema operativo hay un módulo con nombre propio: el Pager (`pager.c`). Es la pieza que convierte «un fichero cualquiera» en «una colección numerada de páginas». Es exactamente lo que vamos a construir para LiraDB.

## 12.1 Objetivo

En el capítulo 11 construiste la página: 4.096 bytes con cabecera, registros con prefijo de longitud y una metapágina en la página 0. Pero una página quieta no sirve de nada. Falta el personaje que las reparte: quién decide en qué byte del fichero vive cada página, cuáles están ocupadas, cuáles están libres y cuándo lo escrito está de verdad en disco.

Ese personaje es el Pager. Vas a construir tres piezas:

1. `PageId` y `PagerError` — el vocabulario: cómo se nombra una página y cómo se queja el gestor.
2. El trait `Pager` — el contrato de ocho operaciones: `allocate`, `read`, `write`, `sync`, `free`, `num_pages`, `is_allocated`, `page_size`.
3. `FilePager` — la primera implementación real, sobre `std::fs::File`.

## 12.2 Problema

Tienes `SlottedPage::encode()` que devuelve 4.096 bytes. La pregunta suena tan tonta como la del capítulo anterior: **¿quién los pone en el fichero?**

La respuesta ingenua: cada módulo, por su cuenta. El CSR calcula «mi página siguiente es `file_len / 4096`», hace `seek` y escribe. Tres semanas después, el índice hash hace el mismo cálculo. Y un buen día ambos calculan lo mismo: `file_len / 4096 == 7` para los dos. El CSR escribe su página en la 7, el índice escribe la suya encima, y nadie se entera hasta que el grafo devuelve datos del índice equivocado. Eso es el **doble uso de una página**: el fallo clásico de no tener un dueño del inventario.

Inventemos más desastres, porque hay para elegir:

- **Offsets a mano por todo el código**: `seek(id * 4096)` repetido en diez sitios. El día que `PAGE_SIZE` cambie, o que alguien multiplique `id * PAGE_SIZE` en un `u32` ya grande (2²⁰ páginas × 2¹² = 2³²: desbordamiento exacto), la corrupción está servida.
- **Páginas a medias tras un crash**: el proceso muere a mitad de extender el fichero y quedan 4.096·n + 2.000 bytes. ¿Quién lo detecta? Si nadie, la última «página» se lee a medias: un `read` normal devuelve los bytes que hay y punto.
- **Borrado sin registro**: liberas la página 5 porque un nodo se borró. ¿Quién apunta que la 5 está libre? Si nadie, el fichero solo crece; si cada uno se apunta sus libres en su cuaderno, vuelves al doble uso.

La raíz es la misma lección del cap. 11, un nivel más arriba: **mezclar decisiones que deben estar separadas**. Aquí: *qué hay dentro de una página* (cap. 11) frente a *qué páginas existen y quién las posee* (este capítulo). La base de datos real pone un cuello de botella deliberado entre ambas: todas las capas superiores piden páginas a un único gestor. Nosotros lo llamaremos `Pager`.

## 12.3 Modelo mental

Piensa en un **bibliotecario de un almacén de cajas**:

- El almacén es una nave con **cajas idénticas, numeradas 0, 1, 2…** apiladas en fila. La caja N está a `N × tamaño_de_caja` metros de la puerta: llegar a ella es aritmética, no búsqueda.
- Tú **nunca entras al almacén**. Pides la caja 12 al bibliotecario (`read`), la recibes, la modificas en tu mesa y se la devuelves (`write`).
- El bibliotecario lleva **una carpeta con las cajas vacías** (la free list). Cuando pides caja nueva (`allocate`), te da una de la carpeta si hay; si no, amplía la nave.
- Cuando devuelves una caja porque ya no la necesitas (`free`), él la anota en la carpeta. La caja sigue ocupando sitio en la nave: tirarla a la basura dejaría huecos imposibles de renumerar.
- Al cierre del día, el bibliotecario **recorre la nave comprobando que cada caja devuelta está realmente en su estante** (`sync`). Que tú la hayas dejado en el pasillo no basta.

En capas de código:

```
   SlottedPage / MetaPage / (cap. 13) BufferPool   ← callers: saben QUÉ hay en una página
   ───────────────────────────────────────────────
   trait Pager                    ← el PORT: el contrato, la carpeta, la aritmética
   ───────────────────────────────────────────────
   FilePager      (futuro: MmapPager, MemoryPager) ← ADAPTERs: saben CÓMO se llega al medio
   ───────────────────────────────────────────────
   un fichero = [pag 0][pag 1][pag 2][pag 3][pag 4]...
                 meta     CSR    LIBRE   índice
                                  ↑
                        free_list: [2]
```

El momento ¡ajá! de este capítulo: **el `PageId` ES el offset físico**. `offset_of(id) = id * PAGE_SIZE`, ni una indirección más. Por eso pedir y devolver cajas es O(1)… y por eso, cuando llegue la compactación (cap. 16), no podremos mover páginas de sitio sin reescribir todos los punteros del CSR y de los índices. Esa doble cara —barato ahora, rígido después— es la decisión estructural de todo el motor.

## 12.4 Primera solución

Lo más simple que funciona: funciones sueltas sobre un `File`.

```rust
// Solución ingenua: leer una página "a mano".
fn read_page(file: &mut File, id: u32, buf: &mut [u8]) -> std::io::Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(id as u64 * 4096))?;
    file.read_exact(buf)?;
    Ok(())
}
```

Siete líneas. Correctas, incluso. Escribe su hermana `write_page`, réplícalas en cada módulo, y durante un tiempo todo va bien. Los tests pasan.

## 12.5 Sus límites

Hasta que el sistema crece y la función inocente enseña los dientes:

1. **No hay inventario**: nada impide el doble uso de la página 7, ni responde «¿la 5 está libre?». Cada `allocate` es un `file.metadata()?.len() / 4096` — y dos módulos que lo hagan a la vez obtienen el mismo número.
2. **Cada caller re-implementa (u olvida) la validación**: ¿el buffer tiene 4.096 bytes? ¿el `id` existe? Con un buffer de 10 bytes, un `read` normal lee 10, devuelve «éxito» y el resto del buffer queda con basura de la operación anterior.
3. **`io::Error` lo explica todo y no dice nada**: «permission denied» (el disco arde), «página fuera de rango» (tu bug) y «página libre» (otro bug) llegan como el mismo tipo indistinguible. El caller no puede reaccionar: solo loguear.
4. **Estás soldado al disco**: el cap. 13 necesita probar la evicción de la caché cientos de veces por segundo; con `std::fs::File` real, cada test paga el sistema de ficheros. Y el día que quieras `mmap`, tendrás que reescribir todos los callers.

## 12.6 Solución evolucionada

La solución tiene dos gestos: **un contrato (trait) y un dueño del inventario**. Primero el contrato, que es el corazón del capítulo:

```rust
/// Trait del gestor de páginas (port en arquitectura hexagonal).
pub trait Pager {
    /// Asigna una nueva página (nunca reutiliza un ID en la free list).
    fn allocate(&mut self) -> Result<PageId, PagerError>;
    /// Lee la página `id` en el buffer `page` (debe tener `PAGE_SIZE` bytes).
    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError>;
    /// Escribe el buffer `page` (debe tener `PAGE_SIZE` bytes) en la página `id`.
    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError>;
    /// Sincroniza el estado del pager con disco (`fsync`/`fdatasync`).
    fn sync(&mut self) -> Result<(), PagerError>;
    /// Número total de páginas que el pager puede direccionar (incluyendo
    /// páginas en la free list, que existen en disco pero no están asignadas).
    fn num_pages(&self) -> u32;
    /// Libera una página: la marca como libre y la añade a la free list.
    /// La página sigue ocupando espacio en disco hasta un futuro `vacuum`.
    fn free(&mut self, id: PageId) -> Result<(), PagerError>;
    /// ¿Está la página `id` asignada (no en la free list)?
    fn is_allocated(&self, id: PageId) -> bool;
    /// Tamaño de página (en bytes) usado por este pager.
    fn page_size(&self) -> usize { PAGE_SIZE }
}
```

**¿Por qué un trait y no funciones sueltas?** Porque un trait es un *port* de la arquitectura hexagonal que ya usamos en el cap. 8 con `GraphStore`: las capas superiores dependen del contrato, no del medio. Los beneficios son concretos y con fecha: (a) el cap. 13 definirá un `MemoryPager` en sus tests para probar la evicción de la caché **sin tocar disco**; (b) un apéndice comparativo añadirá `MmapPager` sin cambiar ni una línea de los callers; (c) cada implementación puede decidir su propio `page_size()` (por eso es método con valor por defecto, no constante pegada al trait). La alternativa — funciones sobre `&mut File` — habría soldado el buffer pool al sistema de ficheros para siempre. SQLite hace lo mismo con su capa VFS: el pager habla con una interfaz de OS abstracta, y debajo hay una implementación por plataforma (`os_unix.c`, `os_win.c`).

Segunda decisión: la **free list vive en un `Vec<PageId>` y se consume en LIFO** (Last In, First Out). ¿Por qué LIFO y no FIFO? Tres razones: (1) `Vec::pop()` es O(1) con cero estructuras extra — una cola FIFO exigiría un `VecDeque` para lo mismo; (2) la página recién liberada es la más «caliente»: sigue probablemente en la caché del sistema operativo, así que reutilizarla primero es gratis, mientras que FIFO te manda siempre a la página más fría y olvidada; (3) el orden resultante es determinista y lo dicta una sola línea, no una política repartida por el código.

Y la decisión más incómoda del capítulo, escrita en los comentarios del propio módulo: **la free list NO se persiste**. Vive en memoria; si cierras y reabres el fichero, se pierde. ¿No es eso un bug? No: es una **deuda pedagógica documentada**, y el porqué merece su propio apartado. Lo vemos dentro de un momento, leyendo el código real.

## 12.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap12_pager.rs` (22 tests). Vamos a leerlo por partes; todos los fragmentos salen de ese módulo, con alguna línea larga compactada para caber en página.

### El vocabulario: `PageId` y `PagerError`

```rust
/// Identificador de página (índice 0-based; la página 0 es la metapágina).
pub type PageId = u32;
```

¿Por qué `u32` y no `usize`? Porque `PageId` es un tipo de **formato persistente**: el `page_id` de la cabecera del cap. 11 es un u32 little-endian en disco, y los tipos que viajan a disco deben ser de anchura fija (`usize` cambia con la plataforma — 32 bits ahí, 64 aquí). El límite es honesto y está escrito: 2³² páginas × 4 KB = **16 TiB** de fichero máximo. Cuando se agote, el error tiene nombre (`NoFreePageId`), no un overflow silencioso.

```rust
pub enum PagerError {
    Io(std::io::Error),                       // el entorno falló (disco, permisos)
    OutOfRange { requested: PageId, num_pages: u32 }, // pediste una página que no existe
    FreePage(PageId),                         // la página está en la free list
    BadBufferSize { expected: usize, got: usize },    // buffer ≠ PAGE_SIZE
    NoFreePageId,                             // 4 GiB de páginas agotados
}
```

**¿Por qué un error tipado y no `io::Error` directo?** Porque los callers necesitan distinguir *culpa propia* de *culpa del entorno*. `OutOfRange`, `FreePage` y `BadBufferSize` son bugs del caller: un `id` que no salió de `allocate`, un use-after-free, un buffer mal dimensionado. `Io` es el mundo exterior. Con `io::Error` a secas, el buffer pool del cap. 13 tendría que parsear strings para reaccionar distinto a cada caso. Además el enum implementa `Display`, `Error::source()` (que solo la variante `Io` devuelve — el resto no envuelve nada) y `From<io::Error>`, para que el operador `?` convierta los errores de E/S automáticamente. Es el mismo patrón que SQLite expone en `pager.h`: códigos de error propios, no errno del SO.

### `create`: por qué la página 0 se reserva desde el minuto uno

```rust
// Reservar la página 0 escribiéndola vacía. Esto fija el tamaño del
// fichero en PAGE_SIZE y deja la metapágina lista para `MetaPage`.
let zeros = vec![0u8; PAGE_SIZE];
file.write_all(&zeros)?;
file.sync_all()?;
Ok(Self { file, path, num_pages: 1, free_list: Vec::new() })
```

`create()` no deja un fichero vacío: deja un fichero con **exactamente una página, toda a ceros, y `num_pages == 1`**. ¿Por qué? Porque en el cap. 11 establecimos que la página 0 es *siempre* la metapágina — el punto de arranque en frío del fichero entero. Reservarla aquí fija dos invariantes de golpe: (1) ningún `allocate()` futuro devolverá jamás la página 0, así que ninguna estructura de datos pisará el catálogo del fichero; (2) `read(0, …)` funciona inmediatamente tras crear, sin casos especiales de «fichero vacío». La alternativa — crear el fichero con `num_pages == 0` y asignar la 0 a quien la pida primero — habría sembrado `if fichero_vacío` por todo el código… o peor, habría entregado la metapágina al CSR. Fíjate además en dos detalles con dientes: `create` abre con `.truncate(true)` (crear significa crear: destruye un fichero previo sin preguntar — está en su doc comment, y es la semántica que una CLI `liradb init` espera), y hace `sync_all()` antes de devolverse: la promesa «el fichero existe y tiene una página» no vale nada si el corte de luz la borra.

### `open`: por qué valida que el tamaño sea múltiplo de `PAGE_SIZE`

```rust
let len = file.metadata()?.len();
if len % PAGE_SIZE as u64 != 0 {
    return Err(PagerError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("file size {len} not a multiple of PAGE_SIZE={PAGE_SIZE}"),
    )));
}
```

Un fichero con 4.096·n + k bytes (0 < k < 4096) es un fichero **truncado**: un crash a mitad de `extend_by`, un `cp` interrumpido, un disco lleno. Si lo aceptáramos en silencio, `num_pages` mentiría y la corrupción se descubriría muy lejos de su causa, en un `decode` del cap. 11 que falla con «magic inválido» en la última página. Rechazar en el `open` con `InvalidData` y un mensaje que dice exactamente qué está mal es la diferencia entre diez segundos de diagnóstico y una tarde de autopsia.

### `allocate` y la carpeta de cajas libres

```rust
fn allocate(&mut self) -> Result<PageId, PagerError> {
    // Reutilizar free list primero (LIFO).
    if let Some(id) = self.free_list.pop() {
        return Ok(id);
    }
    // Si no hay IDs libres, extender el fichero.
    let id = self.num_pages;
    self.extend_by(1)?;
    Ok(id)
}
```

Dos caminos, en este orden: primero reutilizar (el fichero no crece), luego extender. Y extender (`extend_by`) trae su propio guardián contra el final de los tiempos: el contador de páginas solo crece con `checked_add(extra).ok_or(PagerError::NoFreePageId)?`. Sin ese chequeo, al agotarse el u32 el contador daría la vuelta y `allocate` empezaría a repartir IDs ya usados: el doble uso de página, ahora en versión apocalipsis. Con él, error tipado.

### `read`/`write`: tres checks baratos antes de tocar el fichero

```rust
if page.len() != PAGE_SIZE {
    return Err(PagerError::BadBufferSize { expected: PAGE_SIZE, got: page.len() });
}
if id >= self.num_pages {
    return Err(PagerError::OutOfRange { requested: id, num_pages: self.num_pages });
}
if self.free_list.contains(&id) {
    return Err(PagerError::FreePage(id));
}
self.file.seek(SeekFrom::Start(Self::offset_of(id)))?;
self.file.read_exact(page)?;
```

Orden deliberado: los checks más baratos y más probables primero, y **todos antes de la primera llamada al sistema**. Un bug del caller debe producir siempre el mismo error, sin importar el estado del disco. Y el remate es `read_exact`, no `read`: un `read` puede devolver menos bytes de los pedidos (un *short read*) y quedarse tan ancho; una página leída a medias es peor que ninguna, porque parece válida.

### `free` y `sync`: el inventario y la verdad

```rust
fn free(&mut self, id: PageId) -> Result<(), PagerError> {
    ...
    if self.free_list.contains(&id) {
        return Err(PagerError::FreePage(id));
    }
    self.free_list.push(id);
    Ok(())
}
```

Liberar es **anotar, no borrar**: ni se rellena la página de ceros (una escritura de 4 KB que nadie necesita: quien la reutilice la sobrescribe entera) ni se encoge el fichero (el `PageId` es un offset físico; truncar invalidaría los punteros del CSR del cap. 14). El segundo `free` de la misma página es error, no idempotencia silenciosa: liberar dos veces es el síntoma de dos dueños, y queremos oírlo.

```rust
fn sync(&mut self) -> Result<(), PagerError> {
    self.file.sync_all()?;
    Ok(())
}
```

**¿Por qué existe `sync` si el SO «ya escribió»?** Porque no lo escribió. Cuando `write_all` vuelve con éxito, los bytes están en la *page cache* del kernel: páginas sucias en RAM del sistema operativo, no en el disco. Si el proceso muere, el SO aún tiene los datos y los terminará escribiendo; pero si se apaga la máquina, se evaporan. `sync_all()` es `fsync(2)`: obliga al kernel a volcar esas páginas sucias al medio. ¿Y por qué no llamarlo dentro de cada `write`? Porque `fsync` cuesta órdenes de magnitud más que `write` (hablamos de milisegundos de espera a disco frente a microsegundos de copia a memoria): pagarlo por cada página de 4 KB colapsaría el rendimiento. El patrón correcto es N escrituras + 1 `sync` en el punto de compromiso — y ese «punto de compromiso» será, en los caps. 27-28, la transacción.

### La deuda documentada: la free list no sobrevive al reopen

El módulo cierra con la decisión más honesta del capítulo, escrita como test:

```rust
// Decisión pedagógica: la free list es en memoria. Un reopen la
// pierde (lo cual se corregirá en cap 14 cuando se persista en la
// metapágina). Aquí documentamos el comportamiento actual.
```

¿Por qué no persistirla ya, si la `MetaPage` del cap. 11 incluso tiene un campo `free_pages` esperándola? Porque el orden de las escrituras importa más que la prisa. Piensa qué exige persistir: reescribir la metapágina en cada `free()`, y garantiza que **la free list nueva y el dato que liberó la página llegan a disco de forma consistente**. Sin WAL (cap. 28) no tenemos esa garantía: si la free list llega antes que el dato y hay un crash en medio, la página aparece como libre y asignada a su dueño anterior a la vez — el siguiente `allocate` la entrega a otro dueño, y eso es **corrupción**. Si no llega nunca, la página queda huérfana: espacio perdido hasta una compactación (cap. 16). Entre las dos formas de fallar, elegimos a propósito la que **desperdicia espacio en vez de la que corrompe datos**. El leak se recupera; el doble dueño, no siempre. Esa asimetría — preferir leak a corrupción cuando no puedes garantizar atomicidad — es una regla de diseño que reaparecerá en todo el Vol. II.

## 12.8 Prueba de fuego

Los tests del módulo no verifican «que compila»: verifican las promesas del contrato, una por una. Tres ejemplos con sus aserciones textuales (los comentarios de conexión son míos):

```rust
// LIFO: la free list devuelve la última página liberada, y el fichero no crece.
pager.free(p2).unwrap();
assert!(!pager.is_allocated(p2));
let p4 = pager.allocate().unwrap();
assert_eq!(p4, p2, "LIFO: free list debe reutilizar p2");
assert_eq!(pager.num_pages(), 4); // no creció el fichero
```

```rust
// Persistencia real: create → allocate×4 → write → sync → cerrar → reabrir → releer.
{
    let mut p = FilePager::create(&path).unwrap();
    for id_alloc in 0..4 {
        let id = p.allocate().unwrap();
        let buf = pattern_page(id);
        p.write(id, &buf).unwrap();
        assert_eq!(id, id_alloc + 1);
    }
    p.sync().unwrap();
}
let mut p2 = FilePager::open(&path).unwrap();
assert_eq!(p2.num_pages(), 5);
```

```rust
// La deuda, en forma de test: tras reopen la free list está vacía...
let p2 = FilePager::open(&path).unwrap();
assert!(p2.free_list().is_empty(), "free list es en memoria");
// ...y la página liberada aparece como "asignada" (leak, no corrupción).
assert!(p2.is_allocated(1));
```

Añade la batería de errores —`read_en_pagina_libre_falla`, `read_buffer_mal_tamano_falla`, `open_archivo_con_tamanho_invalido_falla`, `free_sobre_id_no_asignado_o_fuera_de_rango`— y `escribir_metapagina_y_reabrir`, que integra con el cap. 11 escribiendo una `MetaPage` real (`num_pages: 42, free_pages: 5, root_page: 7`) en la página 0, reabre y la decodifica intacta.

¿Qué pasaría si te saltaras este capítulo? Los síntomas son concretos: offsets `id * 4096` calculados a mano por todo el código, dos estructuras escribiendo en la misma página liberada, y un fichero de 8.191 bytes que «funciona» hasta que alguien lo reabre en un `open` honesto. Corrupción silenciosa con aspecto de grafo válido: la bestia de la historia del cap. 11, ahora con patas.

## 12.9 Qué hemos sacrificado

1. **La free list no persiste**: tras un reopen, toda página liberada queda huérfana (parece asignada) hasta una compactación. Decisión documentada en test; su casa natural (la metapágina) ya tiene campo reservado desde el cap. 11.
2. **`free_list.contains()` es O(n)**: cada `read`, `write` y `free` paga un escaneo lineal de la lista. Para miles de páginas da igual; para millones, un bitmap lo haría O(1). Preferimos el `Vec` legible.
3. **El fichero crece página a página**: sin pre-alocación (`fallocate`), cada `allocate` al final del fichero es una escritura de 4 KB de ceros; los motores serios reservan espacio de antemano.
4. **Un solo hilo**: `&mut self` en todo el trait excluye concurrencia por diseño. El wrapper concurrente llega con MVCC (caps. 28-30).
5. **`sync` es un botón global**: `sync_all()` vuelca todo el fichero; no distinguimos `fdatasync` ni sincronizamos por página. Suficiente ahora, tosco para un WAL real.

## 12.10 Cómo lo hace una BBDD real

- **SQLite** es el mapa exacto de este capítulo. En su arquitectura (`sqlite.org/arch.html`), el Pager está justo debajo del B-tree y encima de la capa VFS/OS: el B-tree «pide páginas concretas al pager y le avisa cuando quiere modificarlas, hacer commit o rollback». Su pager (`pager.c`, más `wal.c` y `pcache.c`) añade lo que nosotros dejamos para los caps. 13 y 28: caché, journal y commit atómico. ¿Y la free list? **Persistida**: en la cabecera de 100 bytes de la página 1, el offset 32 guarda el número de la primera *freelist trunk page* y el 36 el total de páginas libres. Las trunk pages forman una lista enlazada cuyas hojas apuntan a las *leaf pages*… que «no contienen información alguna» — SQLite ni las lee ni las escribe al liberar, por la misma razón que nuestro `free` no borra: sobrescribir una página libre es I/O regalada. Incluso hay historia en esa estructura: por compatibilidad con un bug anterior a la versión 3.6.0, las trunk pages modernas dejan las últimas seis entradas sin usar. Los formatos en disco son para siempre.
- **PostgreSQL** separa las mismas responsabilidades con otros nombres: el *storage manager* (`smgr`) es el port, `md.c` («magnetic disk») es el adapter — nuestro `FilePager` con pedigrí — y encima vive el buffer manager de 8 KB por página. El inventario de espacio libre no es una lista: es el **FSM** (free space map), un bitmap por página indexado en árbol.
- **InnoDB (MySQL)** gestiona tablespaces con páginas de 16 KB y listas de extensión libres dentro de su capa FSP — la misma idea (quien posee el inventario de páginas lo posee todo), a escala industrial.

En todos, la lección es idéntica a la de tu `FilePager`: **una única pieza posee la aritmética de páginas y el inventario de libres; las capas de arriba piden y devuelven cajas numeradas**.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `num_pages()` devuelve también las páginas que están en la free list? ¿Qué se rompería en el test `free_y_reutilizacion` si no fuera así?
- *Intermedio*: `read` comprueba buffer, rango y free list en ese orden. Encuentra un escenario donde invertir dos checks cambie el error que ve el caller, y argumenta si importa.
- *Experto*: SQLite persiste su freelist en la cabecera (offsets 32/36). Diseña el protocolo de escritura para persistir nuestra free list en `MetaPage` sin que un crash a mitad entregue una página a dos dueños. ¿Qué garantía te falta del cap. 28?

## 12.11 Lo que te llevas

- El **`PageId` es el offset** (`id × PAGE_SIZE`): acceso O(1) y cero indirección, a cambio de páginas inamovibles.
- Un **port (`trait Pager`) + adapter (`FilePager`)**: el contrato desacopla las capas superiores del medio; los tests del cap. 13 irán a RAM y el mmap del apéndice no tocará a nadie.
- La **free list LIFO en memoria**: reutiliza lo más caliente con `pop()` O(1); su no-persistencia es una deuda documentada que elige *leak* sobre *corrupción*.
- **`write` no es durable**: los bytes viven en la page cache del SO hasta que `sync` (fsync) los baja al disco. N writes + 1 sync.
- **Validar en la frontera**: `open` rechaza ficheros truncados (tamaño no múltiplo), `read/write` validan buffer, rango y propiedad antes de la primera llamada al sistema, y `read_exact` prohíbe medias páginas.
- **Errores tipados** (`PagerError`): el caller distingue su bug (`OutOfRange`, `FreePage`, `BadBufferSize`) del fallo del entorno (`Io`) sin parsear strings.

## 12.12 Ojo, cuidado con…

- **«Ya está guardado»**: sin `sync`, «guardado» significa «en la RAM del kernel». El proceso puede morir sin drama; el corte de luz, no.
- **Confundir el contenido del `Vec` con el orden de salida**: `free_list` guarda en orden de inserción `[1, 2, 3]`; el LIFO lo dicta `pop()` (sale `3` primero). Caímos en eso con el primer test — ver la historia de abajo.
- **`num_pages` cuenta páginas del fichero, no páginas asignadas**: incluye las de la free list, que existen en disco aunque no tengan dueño. Para «¿tiene dueño?» está `is_allocated`.
- **`create()` es destructivo**: trunca un fichero previo sin preguntar (`.truncate(true)`). Es lo que quieres en `liradb init` y una sorpresa en cualquier otro sitio.
- **Liberar no es borrar**: la página sigue en disco con sus datos viejos hasta que alguien la reasigne y sobrescriba. Si guardas secretos, sabes qué ejercicio te espera (el intermedio de abajo, con ceros).

## 12.13 Pin de batalla

> *«Un `PageId` es un puntero desnudo a disco. Repártelo dos veces y corrompes; piérdelo y desperdicias. El pager existe para que ninguna de las dos cosas pase por accidente.»*

## 12.14 Si solo lees 30 segundos

El fichero es un array de páginas: la `i` vive en los bytes `[i·4096, (i+1)·4096)`. El trait `Pager` es el único dueño de esa aritmética y del inventario: `allocate` da páginas (primero reutiliza la free list LIFO, luego extiende), `read`/`write` las mueven con buffers exactos de 4.096 bytes, `free` las devuelve a la lista sin borrar nada, y `sync` baja a disco lo que el SO aún retiene en RAM. `FilePager` lo implementa sobre `std::fs::File`, y los errores tipados dicen si la culpa es tuya o del disco. Todo lo demás —caché, transacciones— se construye encima de este contrato.

## 12.15 Una historia pequeña

El primer test de reutilización múltiple que escribimos para `FilePager` liberaba las páginas 1, 2 y 3 en ese orden y esperaba que `allocate()` las devolviera en el mismo orden: 1, 2, 3. Falló. La free list era `[1, 2, 3]` — el `Vec` había guardado exactamente lo que le dimos — pero `pop()` sacaba por el otro extremo: 3, 2, 1. Nuestra primera reacción fue «bug del pager»; la correcta llegó cinco minutos después: el pager hacía exactamente lo que debía (LIFO), y el que mentía era el test, escrito pensando en una cola. Lo arreglamos y dejamos el comentario en el test para el siguiente lector: *«free_list almacena en orden de inserción; el orden LIFO lo decide el `pop`»*. Moraleja que nos ha servido desde entonces: cuando un test falla, antes de tocar el código pregunta **quién de los dos tiene el modelo equivocado**. A veces el sistema está bien y es tu expectativa la que no sabía leer un `Vec`.

## Ejercicios resueltos

**1. Liberas las páginas 1, 2 y 3 (en ese orden) y luego llamas `allocate()` tres veces. ¿Qué devuelve cada llamada y por qué?**

Devuelve 3, luego 2, luego 1. El `Vec` de la free list contiene `[1, 2, 3]` en orden de inserción, pero `allocate` consume con `pop()`, que extrae por el final. Es LIFO: última página liberada, primera reutilizada — que además es la más «caliente» en la caché del SO. Verifícalo con `cargo test free_y_reutilizacion_multiple`.

**2. Un fichero mide 8.191 bytes y lo abres con `FilePager::open`. ¿Qué pasa y qué harías con esos 8.191 bytes si fueras el encargado de repararlo?**

`open` devuelve `PagerError::Io` con `ErrorKind::InvalidData` y el mensaje `file size 8191 not a multiple of PAGE_SIZE=4096` (el test `open_archivo_con_tamanho_invalido_falla` lo congela). 8.191 = 4.096·2 − 1: dos páginas completas menos un byte, señal de truncamiento. Como encargado, no inventaría la página 2: copiaría el fichero, verificaría las dos páginas completas con el `check` del cap. 16 y reconstruiría lo perdido desde el origen de datos. Redondear «hacia arriba» rellenando ceros fabricaría una página con magic `0x00` que el `decode` del cap. 11 rechazaría igual — solo que ya habrías mutado la evidencia.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el capítulo 11: escribe de memoria los tres campos de información que guarda la `MetaPage` (además de su cabecera) y explica por qué `FilePager::create()` reserva la página 0 antes de que exista ningún dato. Después verifica con un test en la línea de `escribir_metapagina_y_reabrir`: construye la `MetaPage` con tus campos, escríbela, haz `sync`, reabre y decodifica. Si no recuerdas los campos, eso es señal de relectura, no de pista.

**Intermedio (spacing con el cap. 11).** Nuestro `free` deja los datos viejos en la página liberada. Escribe `free_and_zero(&mut self, id)` que, además de apuntar la página en la free list, la rellene de ceros y la escriba. Mide con qué factor cae el rendimiento de un bucle de 1.000 allocate/free (compara con `cargo test -- --nocapture` y un `Instant::now`). Después conecta con lo aprendido: ¿por qué SQLite ni siquiera lee sus freelist leaf pages? ¿Qué gana y qué paga tu versión?

**Experto.** Sustituye la `free_list: Vec<PageId>` por un bitmap (`Vec<u64>` donde el bit i dice si la página i está libre). Mantén la semántica LIFO documentada (o argumenta, en un comentario del código, qué semántica eliges y por qué) y consigue que los 22 tests del módulo pasen sin modificar sus aserciones. Analiza: ¿qué gana `contains` en coste? ¿Qué pasa con `num_pages` cuando el bitmap crece? ¿Cómo afecta a la persistencia futura en la `MetaPage` (pista: un bitmap de 2³² páginas no cabe en una página de 4 KB)?

## Para profundizar

- **Alex Petrov, «Database Internals» (O'Reilly), caps. 2-3** — diseño de páginas, gestión de espacio libre en ficheros y el papel del pager en motores reales.
- **CMU 15-445 (Intro to Database Systems)** — lecciones de buffer pool y disk I/O: el cap. 13 de este libro es su proyecto BusTub en miniatura, y este capítulo es su `DiskManager`.
- **Código y documentación de SQLite**: `sqlite.org/arch.html` (posición del pager), `sqlite.org/fileformat2.html` (la freelist en la cabecera: offsets 32/36, trunk/leaf pages), y `pager.c`/`pager.h` en el código fuente — uno de los ficheros mejor comentados del software libre.
- **Martin Kleppmann, «Designing Data-Intensive Applications», cap. 3** — por qué `write` no es durable: page cache del SO, fsync y los límites del hardware.
- **La historia de SQLite en voz de su autor**: episodio «The Untold Story of SQLite» del pódcast CoRecursive (entrevista a D. Richard Hipp) y su charla en CMU Databaseology (2015).

## Mini-diálogo: en guardia nocturna

> — A ver si lo entiendo: ¿el pager es un `seek` con diploma? `id * 4096` lo hacía yo con una función de siete líneas.

> — Y con esas siete líneas, ¿quién te devolvía la página 7 cuando el índice hash la dejó libre? ¿Quién impedía que el CSR la cogiera a la vez?

> — …nadie. Por eso existía aquel bug que nadie encontraba.

> — Exacto. El valor del pager no está en la aritmética, sino en que la aritmética tiene **un solo dueño**. El `seek` era gratis; el doble uso de una página valía una semana de debug. Y de regalo, el trait: el cap. 13 probará la caché contra un pager en RAM, mil veces por segundo, sin disco. Prueba a hacer eso con tus once líneas.

> — Vale. Pero lo de no persistir la free list me sigue pareciendo trampa.

> — Es la parte más honesta del capítulo: es una deuda **escrita en un test**. Podemos permitirnos perder una página; no podemos permitirnos dársela a dos dueños. Cuando lleguemos al cap. 28 sabremos garantizar el orden de escritura… y entonces la persistiremos sin miedo.

---

*(Próximo capítulo: 13 — El buffer pool. El pager ya sabe leer y escribir páginas… y ahora descubriremos lo caro que es hacerlo por consulta: cachearlas en memoria exige frames, pin counts, bits de suciedad y una política de evicción. Y por fin sabrás para qué servía el port.)*
# Capítulo 13 — El buffer pool (LRU, Clock, métricas)

> *«Todo el mundo quiere la página en RAM. Nadie quiere decidir cuál sale cuando la RAM se llena.»*

## 13.0 La anécdota de la esquina

En enero de 2005, PostgreSQL 8.0 salió a la calle con el algoritmo de reemplazo más elegante que había estrenado una base de datos: **ARC** (Adaptive Replacement Cache), publicado dos años antes por Nimrod Megiddo y Dharmendra Modha en el laboratorio Almaden de IBM. ARC es una pieza de relojería: mantiene dos listas y, según cómo fallen las búsquedas recientes, recalibra solo el equilibrio entre «favorecer lo reciente» y «favorecer lo frecuente». Sin parámetros que tunear. Un trabajo hermoso.

Había un problema: IBM lo había patentado (patente US 6.996.676). En abril de 2005, Tom Lane reescribió el buffer manager de la versión 8.0.2 para sustituir ARC por una variante de 2Q libre de la patente. Y en diciembre de ese mismo año, la 8.1 hizo algo más radical: rediseñó el buffer manager entero para eliminar el cerrojo monolítico que lo estrangulaba (`BufMgrLock`) y adoptó un **clock sweep** — una aguja que gira en círculo bajando contadores de uso — que no es sino el descendiente directo del algoritmo de reloj que Fernando Corbató describió en 1969 para la memoria virtual.

La patente caducó el 22 de febrero de 2024. A día de hoy, PostgreSQL sigue usando el clock sweep y nadie ha corrido a devolver ARC a su sitio. La lección que este capítulo encarna: en un buffer manager, **un algoritmo O(1) sin cerraduras vence al algoritmo brillante que cuesta mantener**. Vamos a construir ese reloj con nuestras manos — y a descubrir que es más difícil de lo que parece, porque nosotros mismos implementamos mal la aguja la primera vez. Sigue leyendo.

## 13.1 Objetivo

El `FilePager` del capítulo 12 funciona, y cada una de sus lecturas es un `seek` + `read_exact` al disco. Este capítulo intercala una capa de memoria entre las estructuras de datos y el pager. Vas a construir tres piezas:

1. `BufferPool<P: Pager>` — un array fijo de **frames** (mesas de 4.096 bytes), una **page table** que mapea `PageId → FrameId`, y el protocolo **pin/unpin** que protege las páginas en uso.
2. La política de reemplazo — **Clock** (la aguja con segunda oportunidad) como defecto, **LRU** como comparación (`PolicyKind`).
3. `Metrics` — contadores y el **hit ratio**: el número con el que se entrevista a una base de datos en producción.

Y de propina, la promesa que dejamos en el mini-diálogo del capítulo 12: un `MemoryPager` en los tests que probará la caché contra RAM pura, miles de veces por segundo, sin tocar disco. El port por fin cobra una factura.

## 13.2 Problema

Los números primero. Acceder a RAM cuesta del orden de **100 nanosegundos**; un SSD NVMe, unos **100 microsegundos**; un seek en disco mecánico, **5-10 milisegundos** (lo vimos en el cap. 11). Entre RAM y disco mecánico hay cuatro a cinco órdenes de magnitud: por cada lectura de disco caben cien mil accesos a memoria.

Ahora mira tus futuras estructuras con esos ojos. La raíz de un B+ tree se consulta en **cada** búsqueda. La página de offsets del CSR (cap. 14) se consulta una vez por nodo del grafo. Un BFS que visite 100.000 nodos con el pager desnudo paga 100.000 lecturas — muchas de ellas de páginas que ya habías leído hace un milisegundo. Sin buffer pool, la consulta más tonta cuesta milisegundos por página tocada, y una base de datos que sólo sirve datos diminutos.

La pregunta de diseño: **¿qué cachear?** Y la respuesta del capítulo 11 vuelve a aparecer: páginas enteras, no registros. Cuatro razones:

1. **El disco ya te da la página entera.** No existe «leer un registro»: leer es traer el bloque de 4 KB. Cachear solo el registro significaría leer la página completa, extraer el registro y tirar el resto — incluido su vecino, que estadísticamente será consultado pronto (localidad espacial).
2. **El pool queda contenido-agnóstico.** Un frame es `[u8; PAGE_SIZE]`: bytes opacos. El mismo pool servirá al CSR del cap. 14 y a los índices del cap. 15 sin parsear nada. Una caché de registros, en cambio, tendría que entender el formato de cada página.
3. **Amortización por localidad.** Los vecinos de un nodo comparten página; una página cacheada paga su lectura una vez y sirve decenas de consultas.
4. **La alternativa «cachear resultados» tiene su cuento**. MySQL tuvo una query cache y la eliminó en la versión 8.0: cada escritura invalidaba entradas y el libro mayor de invalidación se comió el rendimiento. Cachear la unidad de disco es más aburrido y más robusto.

## 13.3 Modelo mental

Piensa en un **taller con mesas limitadas** junto al almacén de cajas del capítulo 12.

- El taller tiene **N mesas idénticas** (frames). Sobre cada mesa hay, como mucho, **una caja abierta** (una página cargada del almacén). Un tablón anuncia qué caja hay en cada mesa (la **page table**).
- Cada mesa tiene tres chapaletas:
  - **«EN USO»** — el `pin_count`. Si alguien está trabajando en la mesa, el inspector no puede tocarla, aunque sea la más vieja del taller.
  - **«SUCIA»** — el `dirty` bit. La copia del almacén ya no coincide con la de la mesa: antes de reutilizar esa mesa hay que devolver la caja.
  - **«usada recientemente»** — el `ref_bit`, la chapaleta de recencia que da nombre a la segunda oportunidad.
- Un **inspector con aguja** recorre las mesas en círculo, como las manecillas de un reloj. Cuando hace falta hueco: mesa con «EN USO» → la salta; mesa con la chapaleta de recencia puesta → **la quita y sigue** (segunda oportunidad: estuvo activa hace poco, que viva una vuelta más); mesa limpia y sin chapaleta → **esa mesa se libera** (la víctima; si estaba sucia, antes se devuelve la caja al almacén).

```
        caller (cap. 14: CSR, cap. 15: índices)
              │ get_page / unpin
              ▼
      ┌─────────────────────────┐
      │ BufferPool<P: Pager>    │   frames (N × 4096 B) + page table
      │   ⌐clock_hand→┐         │   PolicyKind::Clock | Lru
      │   [f0]→[f1]→[f2]…↺      │   Metrics (hits/misses/…)
      └───────────┬─────────────┘
                  │ read / write / sync        ← el port del cap. 12
          ┌───────┴────────┐
          │ FilePager      │ MemoryPager (tests)
          └───────┬────────┘
                disco
```

Y aquí está el momento ¡ajá! del capítulo: la chapaleta de recencia solo sabe contestar **sí o no** («¿esta mesa se usó desde la última pasada de la aguja?»). Quien convierte ese sí/no en un **orden** — quién es más vieja — es la **posición de la aguja** respecto a cada mesa. El reloj es una estampa de tiempo barata: *distancia hasta la aguja ≈ antigüedad*. Cuando lleguemos al código veremos que esa frase, tal cual, es la que nos faltó escribir la primera vez.

Es el mismo truco que los sistemas operativos usan para la memoria virtual desde 1969: el algoritmo de Corbató. Las bases de datos lo copiaron porque el problema es idéntico — un medio lento, una memoria pequeña y rápida, y una pregunta incómoda: ¿a quién echo?

## 13.4 Primera solución

Lo más simple que funciona: cachearlo todo, para siempre.

```rust
// Solución ingenua: un mapa sin límite ni modales.
struct Cache {
    pages: HashMap<PageId, Vec<u8>>,
}
```

Cada página leída se guarda; cada petición mira el mapa primero. Cero política de reemplazo, cero contadores. Los tests pasan con ficheros de cinco páginas. Durante un rato, nadie se queja.

## 13.5 Sus límites

Tres muros, en orden de aparición:

1. **La memoria es finita aunque el disco no.** Un fichero de 2 GiB en una máquina con 512 MiB libres mata el proceso (OOM). Necesitamos **evicción**: decidir quién sale. Y esa decisión no puede ser «el primero que llegó» (FIFO) ni «cualquiera» (aleatorio): expulsarías la página raíz del índice que se consulta en cada búsqueda.
2. **El caller muta el `Vec<u8>` cacheado y nadie se entera.** Sin un `dirty` bit, la expulsión tira la página sin escribirla y las modificaciones se evaporan en silencio. La alternativa obvia — escribir en disco en cada modificación (write-through) — convierte cada cambio de 8 bytes en una escritura de 4 KB y, si somos honestos con la durabilidad, en un `fsync` de milisegundos. Ninguna de las dos vale.
3. **La expulsión descontrolada pisa al que trabaja.** Si el pool reutiliza una mesa mientras alguien la ocupa, sobrescribe los 4 KB bajo sus pies: el caller lee bytes de la página nueva creyendo que son la vieja, y el `mark_dirty` posterior puede escribir una página bajo el id de otra. Corrupción silenciosa con aspecto de grafo válido: la bestia de siempre, ahora en RAM.

La solución real responde a los tres muros con tres mecanismos: **frames fijos + política de reemplazo**, **dirty bit con write-back**, **pin count**. Y un cuarto ingrediente sin el cual los tres anteriores serían ciegos: **métricas**.

## 13.6 Solución evolucionada

El diseño canónico, el mismo de System R a PostgreSQL: una tabla de frames más una book-keeping mínima por frame.

```rust
pub struct BufferPool<P: Pager> {
    pager: P,
    frames: Vec<Frame>,               // mesas: capacidad fija
    page_table: Vec<Option<FrameId>>, // PageId → FrameId (None = no cargada)
    policy: PolicyKind,               // Clock (defecto) | Lru
    clock_hand: usize,                // la aguja
    lru_counter: u64,                 // reloj lógico para LRU
    metrics: Metrics,                 // contadores monotónicos
}
```

Cada `Frame` guarda la página (`data: [u8; PAGE_SIZE]` — exactamente lo que `SlottedPage::encode` produce en el cap. 11), su `page_id: Option<PageId>` (`None` = mesa vacía), el `pin_count: u32`, el `dirty: bool`, el `ref_bit: bool` y el `lru_counter: u64`.

El protocolo de uso es un contrato de tres pasos, y todo el capítulo 14 lo usará:

```
let buf = pool.get_page(id)?;    // 1. HIT o MISS; el frame queda PINEADO
buf[..4].copy_from_slice(&x);     // 2. lee/escribe los 4096 bytes
pool.unpin(id, true)?;           // 3. despinea; true = "la modifiqué"
```

**¿Por qué pin/unpin explícitos y no un guard RAII?** Porque el lifetime debe ser *visible*. Un `PageGuard<'_>` que despinea en su `Drop` sería elegante, pero prestaría `&mut pool` durante toda la vida de la página: no podrías tener DOS páginas pineadas a la vez (y el CSR del cap. 14 itera dos columnas en paralelo) sin `Arc<Mutex<Frame>>` ni `unsafe`, y cada firma del motor arrastraría lifetimes. El coste del pin explícito es que puedes olvidarte: el pin leak. Pero ese olvido tiene detector — `PoolFullOfPinned` — y el error llega en el test que lo provoca, no en producción. Además, así aprendes el mecanismo que PostgreSQL usa de verdad: un contador de referencia por buffer que el clock sweep respeta.

**¿Por qué `PoolFullOfPinned` es un error y no una espera?** Porque aquí no hay nadie que pueda despinear mientras esperas: el pool vive tras `&mut self`, un solo hilo. «Esperar» sería un deadlock contigo mismo. El error tipado es lo honesto: o te falta un `unpin` (el error es su detector), o pediste más páginas pineadas de las que caben y tienes que soltar algo antes. Cuando llegue la concurrencia (cap. 28), esperar volverá a tener sentido.

**¿Por qué dirty + write-back y no write-through?** Por aritmética. Write-through honesto = escritura + `fsync` por mutación: milisegundos por cambiar 8 bytes. Write-back difiere la escritura a la expulsión o al `flush`, y de regalo **fusiona mil modificaciones en una escritura** (write coalescing). El precio: la ventana de caída — un crash antes del flush pierde lo escrito desde el último flush. Esa ventana es exactamente lo que el WAL del cap. 28 vendrá a cerrar.

## 13.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap13_buffer_pool.rs` (24 tests). Lo leemos por partes.

### `get_page`, paso a paso

El corazón del pool distingue hit de miss en la primera línea:

```rust
if let Some(fid) = self.find_frame(id) {          // 1. HIT: ya está en memoria
    self.metrics.buffer_hits += 1;
    self.touch_frame(fid);                        // marca de uso para la política
    self.frames[fid].pin_count += 1;              // pin automático
    return Ok(&mut self.frames[fid].data);
}
if !self.pager.is_allocated(id) {                 // 2. MISS: ¿existe siquiera?
    return Err(BufferPoolError::UnknownPage(id));
}
```

Fíjate en el orden: `is_allocated` **antes** de buscar frame — validar en la frontera, como el `read` del cap. 12 validaba antes de tocar el fichero. Luego viene la parte con drama: buscar hueco. Primero un frame vacío (`find_free_frame`: mesa sin caja, víctima gratis, sin flush); si no hay, la política elige (`pick_victim`); si tampoco puede, `PoolFullOfPinned`. Y la víctima sucia se escribe **antes** de sobrescribirla:

```rust
if victim_dirty {
    self.pager.write(vp, &victim_data)?;   // los cambios del usuario NO se tiran
    self.metrics.page_writes += 1;
}
```

Tras cargar (paso 4: `pager.read` + `page_reads += 1`), el paso 5 resetea el frame: `pin_count = 1`, `dirty = false`, y — detalle importante — `ref_bit = true`: la página recién llegada cuenta como usada, una vuelta de gracia para que un scan de N+1 páginas sobre N frames no expulse cada página recién traída antes de usarla.

### El bug de la aguja: por qué avanza en CADA acceso

Aquí está la decisión más fina del módulo, la que implementamos mal la primera vez. La versión «de libro» del Clock mueve la aguja solo al buscar víctima. Nosotros la movemos también en cada `touch_frame` y en cada miss-load:

```rust
fn touch_frame(&mut self, fid: FrameId) {
    match self.policy {
        PolicyKind::Clock => {
            self.frames[fid].ref_bit = true;
            self.clock_hand = (self.clock_hand + 1) % self.frames.len();
        }
        PolicyKind::Lru => {
            self.lru_counter += 1;
            self.frames[fid].lru_counter = self.lru_counter;
        }
    }
}
```

¿Por qué? Recuerda el modelo mental: el bit dice sí/no; la aguja da el orden. En un workload de hits en ráfagas (el normal: la mayoría de accesos son hits, las expulsiones son raras), si la aguja solo se mueve al buscar víctima, entre búsqueda y búsqueda **todos** los frames acumulan `ref_bit = true`, y el barrido arranca desde una posición congelada en el pasado — la del orden de *carga*, no del de *uso*. Dos páginas accedidas hace un segundo y hace una hora son indistinguibles: ambos bits están a true. El barrido limpia todo lo que encuentra y expulsa a quien esté delante de la aguja: evicción efectivamente aleatoria, LRU perdido. Avanzando la aguja en cada acceso, la aguja pasa por delante del frame recién usado y el círculo empieza a reflejar el orden de uso: la víctima se busca empezando por lo más viejo. El comentario del módulo lo dice sin anestesia: sin avance en acceso, «dos frames accedidos en tiempos distintos parecerían idénticos». Lo vivimos: los tests básicos pasaban igual (contaban hits y misses, no *qué* salía), y fue el test de segunda oportunidad el que cazó la víctima incorrecta.

El barrido completo, con su cota:

```rust
for _ in 0..(2 * n) {
    let cand = self.clock_hand % n;
    if self.frames[cand].pin_count == 0 {
        if self.frames[cand].ref_bit {
            self.frames[cand].ref_bit = false;      // second chance
            self.clock_hand = (self.clock_hand + 1) % n;
            continue;
        }
        self.clock_hand = (self.clock_hand + 1) % n;
        return Some(cand);                          // víctima
    }
    self.clock_hand = (self.clock_hand + 1) % n;    // pineada: saltar
}
None  // todos pineados (o todos con segunda oportunidad perpetua) → error
```

Dos vueltas como máximo: la primera gasta segundos chances, la segunda encuentra mesa. Coste O(1) amortizado, cero listas, cero cerrojos — la razón por la que PostgreSQL cambió su LRU por esto.

### `unpin`, `mark_dirty` y la disciplina del borrow

```rust
pub fn unpin(&mut self, id: PageId, dirty: bool) -> Result<(), BufferPoolError> {
    let fid = self.find_frame(id).ok_or(BufferPoolError::UnknownPage(id))?;
    if self.frames[fid].pin_count == 0 {
        return Err(BufferPoolError::BadPinCount { page_id: id, current: 0 });
    }
    self.frames[fid].pin_count -= 1;
    if dirty { self.frames[fid].dirty = true; }
    Ok(())
}
```

El doble `unpin` no se perdona (`BadPinCount`): es el síntoma de dos dueños o de un pin fantasma. Y el flag `dirty` se puede poner aquí o con `mark_dirty(id)` — dos caminos al mismo estado, para el patrón real de uso que ya viste en los tests del módulo:

```rust
{   // el bloque existe para SOLTAR el préstamo de get_page antes de
    // volver a llamar al pool: nos lo enseñó el compilador a base de errores.
    let buf = pool.get_page(id).unwrap();
    buf[..4].copy_from_slice(&42u32.to_le_bytes());
}
pool.mark_dirty(id).unwrap();
pool.unpin(id, true).unwrap();
```

Ese bloque `{ }` es la respuesta práctica a «¿y por qué no RAII?»: mientras vives el `&mut` de `get_page`, el pool entero está prestado. La disciplina es tuya — y el compilador te la cobra.

### `flush`, `flush_page`, `discard`

`flush` escribe todas las sucias y devuelve cuántas; fíjate en el detalle de Rust: recolecta primero los `(frame_id, page_id)` sucios para no pelearse con el borrow al escribir y limpiar el flag a la vez. Y la última línea es la mitad de la promesa:

```rust
self.pager.sync()?;   // fsync: sin esto, "flush" sería un nombre mentiroso
```

Recordatorio del cap. 12: `pager.write` solo deposita bytes en la page cache del kernel; el corte de luz se los lleva. La promesa de `flush` es «a salvo», así que N writes + 1 sync. `flush_page(id)` es la versión selectiva (devuelve `false` si no estaba sucia). `discard(id)` invalida un frame sin escribirlo — útil cuando alguien reescribe la página por fuera (lo necesitará el `replace()` del CSR en el cap. 14) — y **rechaza** las páginas sucias con un truco honesto que el propio comentario confiesa: como el enum no tenía variante «Dirty», lo señala con `BadPinCount { current: u32::MAX }`. Una deuda de API documentada en el propio código: obligar a `flush_page` explícito antes de descartar es la política conservadora — preferir molestar a corromper.

### `Metrics`: qué mide y qué no

Cinco contadores monotónicos (`buffer_hits`, `buffer_misses`, `page_reads`, `page_writes`, `evictions`) y un ratio:

```rust
pub fn hit_ratio(&self) -> f64 {
    let total = self.buffer_hits + self.buffer_misses;
    if total == 0 { 0.0 } else { self.buffer_hits as f64 / total as f64 }
}
```

Fíjate en dos sutilezas. Primera: `page_reads` y `buffer_misses` cuentan cosas distintas aunque aquí siempre coincidan (cada miss lee una página exacta una vez) — divergirían el día que haya prefetch, y medir cosas distintas por separado es lo que permite diagnosticar. Segunda: **el ratio no lo dice todo**. No mide el frío (la primera lectura de cada página es miss por definición: un fichero leído una vez da ratio 0), no distingue misses aleatorios de secuenciales (un scan que toca cada página una vez da 0.0 con lecturas contiguas, relativamente baratas), no mide escrituras (`page_writes` va en otro contador) y no te dice si el pool está bien dimensionado para el mañana: un hot set diminuto infla el ratio mientras el resto del fichero nunca entra. En producción lo mismo: el hit ratio de `pg_stat_database` (`blks_hit`/`blks_read`) es la primera pregunta, no el veredicto.

### `MemoryPager`: el port paga su primera factura

Los tests no usan `FilePager` salvo para los dos end-to-end. Usan esto:

```rust
struct MemoryPager {
    pages: Vec<Option<[u8; PAGE_SIZE]>>,
    free_list: Vec<PageId>,
}
```

Un `Vec` de páginas con la misma semántica del trait (`read` copia, `write` valida buffer y propiedad, `sync` es `Ok(())` porque no hay disco que engañar). Con él, `small_pool()` monta un pool de 3 frames sobre 5 páginas, y la batería de evicción, dirty-flush y `PoolFullOfPinned` corre en microsegundos. Es la promesa del cap. 12 hecha código: el trait `Pager` existía *para que esto fuera posible*.

## 13.8 Prueba de fuego

Los tests del módulo congelan las promesas una a una (los nombres son del fichero real):

- **Hit/miss contados**: `bp_basic_get_unpin` — dos gets de la misma página → `page_reads = 1`, `buffer_misses = 1`, `buffer_hits = 1`, `page_writes = 0`.
- **El dirty llega a disco en la expulsión**: `bp_dirty_page_is_flushed_on_eviction` — pool de 1 frame, modifica p1 (`buf[0] = 0xAA`), marca sucia, pide p2 (la expulsa), y luego lee p1 *del pager directamente*: el `0xAA` está ahí. Sin el flush de víctima, ese byte sería 0.
- **El pin manda**: `bp_pool_full_of_pinned` — dos páginas pineadas en un pool de 2; el tercer `get_page` es `Err(PoolFullOfPinned)`. Y `bp_unpin_all_resets_pins` muestra la salida de emergencia.
- **La aguja protege a lo caliente**: `bp_clock_second_chance_protects_hot_page` — capacidad 2; tras cargar A y B, tocar A, y cargar C, los misses totales son exactamente 3: A sobrevivió. Es el test que cazó el bug de la aguja.
- **LRU de verdad**: `bp_lru_policy_evicts_least_recent` — mismo escenario con `PolicyKind::Lru`: sale la menos reciente, por diseño y no por aproximación.
- **End-to-end**: `bp_persistence_via_filepager` (create → pool → escribir 3 páginas → flush → reabrir con `FilePager` y verificar) y `bp_reload_via_pool` (reabrir también el pool: `occupied() == 0` y el primer get es miss — la caché no sobrevive al proceso, por contrato).

¿Qué pasaría si te saltaras este capítulo? El síntoma es medible: cien accesos a la misma página, cien `page_reads`. Y el funcional: el cap. 14 construiría un CSR que paga milisegundos por página tocada.

## 13.9 Qué hemos sacrificado

1. **LRU con escaneo O(n) por evicción**: el contador monotónico es transparente pero lineal; producción usaría lista doblemente enlazada.
2. **Un solo hilo**: `&mut self` en todo; sin átomos ni esperas. El wrapper concurrente llega en el cap. 28.
3. **Dos deudas menores confesadas en el código**: el `discard` sucio señalado con el sentinel `u32::MAX`, y la `page_table` como `Vec` (8 bytes por posible `PageId`: perfecto para ids densos como los nuestros, derrochador en espacios dispersos).
4. **Sin background writer**: las escrituras se agrupan en `flush` y en expulsiones — el pico de latencia de un flush grande llegaría de golpe.
5. **Métricas de vida, no de ventana**: los contadores son monotónicos desde la creación; para tendencia haría falta sampling periódico.

## 13.10 Cómo lo hace una BBDD real

- **PostgreSQL** es este capítulo con veinte años de producción encima. Su buffer manager cachéa páginas de 8 KB en `shared_buffers`; la política es el **clock sweep** que adoptó en la 8.1 (la historia del comienzo): como nuestro Clock, pero con el bit de 1 ampliado a un **usage count de 0 a 5** — usar la página lo sube, la aguja al pasar lo baja, y la víctima es el primer buffer con contador a 0 y sin pins (pins de verdad: un contador de referencia por buffer). El hit ratio se vigila en `pg_stat_database` (`blks_hit` frente a `blks_read`); la extensión `pg_buffercache` te deja mirar dentro del pool.
- **InnoDB (MySQL)** cachéa páginas de 16 KB con un LRU **con punto medio**: las páginas nuevas entran al 37 % «viejo» de la lista (`innodb_old_blocks_pct`), y solo ascienden a la zona joven si vuelven a usarse tras `innodb_old_blocks_time` (1 s por defecto). Es una vacuna contra el full scan que arrastraría la caché entera — el mismo problema que nuestro `ref_bit` al cargar ataca de forma más tosca.
- **SQLite** mantiene una caché de páginas por conexión (2.000 KiB por defecto) en su módulo pcache, encima del pager del cap. 12.
- **Kùzu**, la base de datos de grafos que tomamos como referencia, usa una variante **GClock** (contador de uso en vez de bit), como anota el comentario del módulo.

En todos, el patrón es el que acabas de construir: frames + tabla + pins + dirty + una aguja. Lo que cambia es cuántos bits le dan a la recencia.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: PostgreSQL tenía LRU «mejor» en teoría y lo cambió por una aproximación. Resume el argumento en una frase y di qué le falta a Clock a cambio.
- *Intermedio*: reproduce en LiraDB el ataque del que se vacuna InnoDB: un working set caliente de 10 páginas en un pool de 64, y un scan de 100 páginas que lo arrasa. Mide el hit ratio antes y después con `metrics()`. ¿Qué política lo sufre más?
- *Experto*: implementa `PolicyKind::GClock` al estilo PostgreSQL: contador de uso saturado (máx 5) en vez de bit; la aguja decrementa, la víctima es el contador a 0. Haz pasar los 24 tests sin tocar aserciones y añade uno que demuestre que una página tocada k veces sobrevive k barridos.

## 13.11 Lo que te llevas

- El buffer pool intercala **memoria entre las estructuras y el pager**: la diferencia entre ~100 ns y milisegundos por acceso.
- Se cachean **páginas enteras, no registros**: el disco entrega bloques, la localidad amortiza, y el pool queda contenido-agnóstico.
- El protocolo **pin → uso → unpin(dirty)**: el pin blinda contra la expulsión bajo tus pies; el leak tiene detector (`PoolFullOfPinned`), el doble unpin tiene error (`BadPinCount`).
- **Write-back con dirty bit**: mil modificaciones, una escritura; el precio es la ventana de caída que el WAL (cap. 28) cerrará. Toda víctima sucia se escribe antes de reutilizarse.
- **Clock = LRU aproximado a O(1)**: un bit, una aguja, segunda oportunidad. Y la aguja avanza **en cada acceso**, no solo al buscar víctima — o la recencia se evapora.
- **flush termina en `sync`**: sin fsync, «a salvo» es un nombre mentiroso.
- **hit_ratio** mide la fracción de accesos servida de memoria — y nada más: ni frío, ni secuencialidad, ni escrituras, ni dimensionamiento.

## 13.12 Ojo, cuidado con…

- **El `unpin` olvidado**: pin leak → `PoolFullOfPinned` en el peor momento. Un `get_page` con `Ok`, un `unpin` — siembre.
- **Mutar sin `dirty`**: ni `unpin(id, true)` ni `mark_dirty` → la expulsión tira tus cambios sin error. El fallo más silencioso del capítulo.
- **Confundir hit con `page_read`**: uno es servir de RAM; otro, llamar al pager. Hoy coinciden; no escribas código que lo asuma para siempre.
- **Tratar Clock como LRU exacto**: promete segunda oportunidad, no la víctima LRU. El test original del módulo lo asumía y mentía.
- **Confundir evicción con flush**: se expulsa sin escribir (víctima limpia) y se escribe sin expulsar (`flush`). Son ejes distintos.

## 13.13 Pin de batalla

> *«Una caché no hace tus datos más rápidos: hace que tus discos parezcan innecesarios. Cuida a quién expulsas, porque la lentitud también se cachea.»*

## 13.14 Si solo lees 30 segundos

Entre las estructuras y el pager vive el `BufferPool<P: Pager>`: N frames de 4.096 bytes, una page table, y el protocolo pin → uso → unpin(dirty). Cada `get_page` es hit (pin + touch) o miss (leer del pager, quizá expulsar a una víctima — sucia se escribe primero — con Clock: la aguja salta pineados, da segunda oportunidad a los recientes y elige al primero sin bit). `flush` escribe las sucias y sincroniza; `metrics().hit_ratio()` te dice qué fracción de accesos no fueron al disco — solo eso, pero eso ya lo es casi todo.

## 13.15 Una historia pequeña

La primera versión del Clock de LiraDB movía la aguja solo dentro de `pick_victim` — la versión de libro, juramos. Los tests de hits, misses y expulsiones pasaban todos: los contadores no dicen *quién* salió. Y entonces escribimos el test de la segunda oportunidad: cargar A y B en dos frames, tocar A, cargar C, y comprobar que A sobrevivía. Falló: C había entrado expulsando a A, la página que acabábamos de usar. Nos sentamos a trazar la aguja a mano y el problema era tan simple como incómodo: como la aguja no se movía con los accesos, apuntaba a donde la dejó la última *carga*, y el barrido se gastaba el bit de A de primero. Dos páginas tocadas a un segundo y a una hora de distancia eran, para el algoritmo, la misma página. El arreglo fueron dos líneas (avanzar en `touch_frame` y en el miss-load); la lección quedó escrita en el comentario del módulo: en Clock, la recencia no la guarda el bit — la guarda la aguja. Desde entonces, cuando algo «aproximadamente correcto» falla, buscamos qué parte del estado llevábamos congelada.

## Ejercicios resueltos

**1. Pool de capacidad 2, política Clock, páginas A, B y C en el pager. Secuencia: get(A) + unpin, get(B) + unpin, get(A) + unpin (hit), get(C). ¿Qué página se expulsa y por qué?**

Sigamos la aguja (empieza en 0): get(A) es miss → frame 0, `ref_bit = true`, y la aguja avanza a 1. get(B) → frame 1, bit a true, aguja a 0. get(A) es **hit** → touch: bit a true (ya lo estaba) y la aguja avanza a 1 — A queda *detrás* de la aguja, protegida hasta el final del próximo barrido. get(C): sin frame libre, el barrido arranca en la aguja (frame 1 = B): bit a true → se lo quita, avanza; frame 0 (A): bit a true → se lo quita, avanza; vuelta al frame 1: bit a false → **víctima = B**. C entra en su frame y los misses totales son 3 (A, B, C): A sobrevivió. Sin el avance en el hit (el bug), la aguja habría quedado en 0 tras cargar B: el barrido habría gastado el bit de A primero y la víctima habría sido **A**, la página caliente. Verifícalo con `bp_clock_second_chance_protects_hot_page`.

**2. Un pool acumula `hits = 990, misses = 10`. Otro, `hits = 0, misses = 100` tras escanear un fichero por un pool de 10 frames. ¿Cuál está «peor»?**

El ratio dice 0.99 y 0.0, pero el 0.0 puede ser saludable: son lecturas *secuenciales* de un fichero que no cabe (y no cabe por diseño: 100 páginas, 10 frames — ningún algoritmo de reemplazo evita un solo miss por página en un scan puro). El 0.99, en cambio, esconde la pregunta importante: ¿y las escrituras? Un `page_writes` alto con `evictions` altas diría que las víctimas salen sucias y el disco está en el camino crítico. El ratio mide *una* cosa — accesos servidos de memoria — y se lee junto a `page_reads`, `page_writes` y la forma del workload, nunca solo. (El test `metrics_hit_ratio` fija la aritmética: 3/4 = 0.75, y 0 accesos = 0.0, no NaN.)

## Ejercicios propuestos

**Esencial (retrieval, cap. 11).** Sin mirar los capítulos 11 y 12: escribe de memoria el layout de la cabecera de página — los campos, su ancho en bytes y para qué sirve cada uno — y explica por qué `Frame.data` es exactamente `[u8; 4096]`. Después convierte el recuerdo en test: con un `MemoryPager`, guarda una `SlottedPage` codificada pasando por el pool (`get_page` + `mark_dirty` + `unpin` + `flush`) y recupérala decodificando la cabecera *desde el buffer que te devuelve un segundo `get_page`* — que debe ser hit, y las métricas deben demostrarlo. Si no recuerdas los campos, eso es señal de relectura, no de pista.

**Intermedio (spacing + interleaving).** Implementa `fn read_record<P: Pager>(pool: &mut BufferPool<P>, page: PageId, index: usize) -> Result<Vec<u8>, BufferPoolError>` que cargue la página por el pool y extraiga el registro `index` caminando los prefijos de longitud del cap. 11 — sin decodificar la página entera ni tocar el pager directamente. Escribe un test que inserte tres registros de longitudes distintas, los lea de uno en uno y verifique con `metrics()` que la segunda lectura de la misma página no produjo `page_read` nuevo. ¿Dónde acaba viviendo el offset del registro: en el pool, en el caller o en ninguna parte?

**Experto.** El `discard` actual rechaza páginas sucias con el sentinel `BadPinCount { current: u32::MAX }`. Añade la variante honesta `BufferPoolError::PageDirty(PageId)`, migra el sentinel, actualiza `bp_discard_dirty_rechazado`… y discute en un comentario del código las dos semánticas posibles de descartar una sucia: flush implícito (cómodo, esconde coste) o rechazo explícito (verboso, obliga a decidir). ¿Cuál elegiría el principio «leak mejor que corrupción» del cap. 12… y cuál elegiría un DBA a las 3 de la madrugada?

## Para profundizar

- **The Internals of PostgreSQL, cap. 8** (interdb.jp) — el buffer manager de PostgreSQL con diagramas: clock sweep con usage count, pins, y el ciclo de vida de un buffer.
- **PostgreSQL wiki, «Multiple Buffer Pools»** y el **LWN.net de la 8.0.2** (lwn.net/Articles/131554) — la historia ARC → 2Q → clock sweep contada por quienes la vivieron; la patente US 6.996.676 caducó en 2024.
- **Wikipedia, «Page replacement algorithm»** — el origen: Corbató describió el algoritmo de reloj en 1969; Carr y Scott lo elevaron a WSCLOCK (CACM, 1981).
- **Megiddo y Modha, «ARC: A Self-Tuning, Low Overhead Replacement Cache»** (FAST '03) — el algoritmo que perdió contra el reloj.
- **Manual de MySQL, «The InnoDB Buffer Pool»** — el LRU con punto medio: `innodb_old_blocks_pct` y `innodb_old_blocks_time`.
- **Alex Petrov, «Database Internals» (O'Reilly), cap. 2, y CMU 15-445** — caching y buffer management en motores reales; el proyecto BusTub de CMU es este capítulo a escala de curso.

## Mini-diálogo: en guardia nocturna

> — Cumpliste lo prometido: el pager falso. La caché entera probándose contra RAM, sin un solo tempfile. Tardé en creerlo.

> — El mérito es del trait, no mío. El cap. 12 dejó el enchufe; hoy solo hemos enchufado otra cosa. Fíjate qué más: ni el pool ni los 24 tests saben si abajo hay un `FilePager` o un `MemoryPager`.

> — Pero reconoce el bug de la aguja. Hiciste el Clock «de libro» y el libro te salió rana.

> — Y fue el mejor día del capítulo. Los tests de contadores pasaban todos; el de segunda oportunidad fue el único que preguntó *quién* salió, no *cuántos*. Moraleja: un test que solo cuenta aciertos no audita una política.

> — Lo que aún no trago es eso de acordarme del `unpin` a mano. Un guard lo haría imposible.

> — Imposible de olvidar, sí. E imposible de tener dos páginas a la vez, e imposible de leer sin tres lifetimes por firma. Rust ya nos presta el pool entero en cada `get_page` — ¿viste el bloque `{ }` de los tests? El pin explícito te enseña lo que PostgreSQL hace por dentro; el guard te lo escondería.

> — Vale. Y el hit ratio, ¿ese número es la nota del capítulo?

> — Es la primera pregunta del interrogatorio, no el veredicto. Un 0.99 con escrituras desbocadas es un enfermo con buena cara. Lo aprendimos leyendo a InnoDB: él ni siquiera se fía del LRU…

> — …y por eso mete las páginas nuevas por la mitad de la lista. O sea, que ya ni los que pueden pagar LRU estricto lo quieren.

> — Exacto. Ahora duerme: mañana llega el primer cliente de verdad. El CSR va a pedirte cuatro columnas página a página… y a reescribirlas enteras cuando el grafo cambie. Pregunta de guardia: cuando `replace()` pise una página que sigue cacheada en un frame, ¿quién limpia el frame?

> — …¿`discard`? Para eso nació, ¿no?

> — Para eso nació. Buenas noches.

---

*(Próximo capítulo: 14 — Cómo almacenar adyacencias. El CSR persistente será el primer cliente real del buffer pool: cuatro columnas de arrays repartidas en páginas, leídas con `get_page`/`unpin`, y reescritas por completo con `replace()` — el momento en que `discard` justificará su existencia.)*
# Capítulo 14 — Cómo almacenar adyacencias (CSR, segmentos)

> *«La lista de adyacencia de un nodo no es una lista: es un intervalo de un array más grande, con alguien que recuerda dónde empieza.»*

## 14.0 La anécdota de la esquina

A mediados de los años 70, en el departamento de informática de la Universidad de Yale, un grupo de matemáticos aplicados —Stanley Eisenstat, Martin Gursky, Martin Schultz y Andrew Sherman— tenía un problema de los que mueven la historia: resolver sistemas de ecuaciones lineales con millones de variables que salían del método de elementos finitos, de yacimientos de petróleo, de estructuras de puentes. La matriz de esos sistemas es gigantesca y casi toda ceros: en una malla fina, cada variable se relaciona con un puñado de vecinas. Guardar los ceros era absurdo, pero ¿cómo guardar solo lo que importa y seguir pudiendo multiplicar la matriz por un vector sin perderse?

Su respuesta, publicada en 1977 como **Yale Sparse Matrix Package** (informes YALEU/DCS/RR-112 y RR-114), no fue un algoritmo: fue una *forma de mirar los datos*. Tres arrays: los valores distintos de cero, apuntados fila a fila; un array de índices de columna; y un array de **inicios acumulados de fila**. Cada fila es un intervalo de un único array plano. El formato llevaba años usándose en computación científica (hay descripciones completas desde 1967) y Yale lo estandarizó; durante décadas se lo conoció como «el formato Yale». Hoy lo llamamos **CSR, Compressed Sparse Row**.

¿Y qué tiene que ver una matriz con tu base de datos de grafos? Todo: la matriz de adyacencia de un grafo ES una matriz dispersa — cada arista es un elemento distinto de cero. Cuando en 2023 el equipo de la Universidad de Waterloo presentó Kùzu (paper «Kùzu Graph Database Management System», CIDR 2023), el corazón de su almacenamiento era exactamente esto: listas de adyacencia en estilo CSR, columna a columna, como índice de join fundamental del motor. Un formato nacido para resolver puentes y yacimientos en FORTRAN es hoy la columna vertebral de las bases de datos de grafos analíticas. En este capítulo lo construyes, y lo persistes sobre todo lo que ya tienes: slotted pages, pager y buffer pool.

## 14.1 Objetivo

Hasta ahora LiraDB sabía guardar *cosas* — nodos con sus propiedades, en slotted pages, a través de un pager, con un buffer pool encima. Pero una base de datos de grafos no vive de guardar nodos: vive de responder **«¿quiénes son los vecinos de X?»** en microsegundos, millones de veces. Esa pregunta necesita su propia estructura, dedicada, pensada para el patrón de acceso de los recorridos.

Vas a construir dos piezas:

1. `Csr` — la representación en memoria: cuatro arrays (`forward_offsets`, `forward_targets`, `backward_offsets`, `backward_targets`) construidos desde una lista de aristas, con sus invariantes verificadas.
2. `PersistentCsr<P: Pager>` — la persistencia: el CSR guardado como *chunks* en slotted pages a través del buffer pool, con un `CsrHeader` de 24 bytes como catálogo en la página 1.

Es la primera estructura de LiraDB que usa **todo** el motor. El capítulo 7 dio el modelo, el 9 el encoding, el 11 la página, el 12 el pager, el 13 el pool. Este capítulo los encadena.

## 14.2 Problema

La pregunta suena inocente: **¿dónde guardas los vecinos?** Ya la conoces del Vol. I, cap. 2: lista de adyacencia o matriz de adyacencia. Y ya usaste listas en `MemoryStore` (cap. 8). Pero ahora hay requisitos nuevos, de base de datos: la estructura debe sobrevivir a un cierre del proceso, entrar y salir por páginas de 4.096 bytes, pasar por el buffer pool, y aguantar el patrón de acceso de un BFS sin fundir la memoria. Repasemos las candidatas con números de verdad, para un grafo de 1 millón de nodos y 4 millones de aristas:

- **Matriz de adyacencia**: 1M × 1M = 10¹² celdas. A 1 bit por celda, 125 GB. A un `u32` por celda, 4 TB. Y tu grafo tiene 4M aristas: densidad 4·10⁻⁶. La matriz solo compite cuando el grafo es denso (E ≈ V²); un grafo real es un desierto con un lago. Descartada por O(V²).
- **`Vec<Vec<NodeId>>`**: memoria razonable a primera vista (los 4M targets son 16 MB de datos), pero cada `Vec` interior es un puntero a una alocación propia del heap. Lo desmenuzamos en la §14.5, porque el problema no es el espacio: es *dónde* vive.
- **Una página por nodo**: 1M páginas de 4 KB = 4 GB para 16 MB de datos útiles. La página es unidad de E/S, no de grafo.

Falta además un requisito que en el Vol. I no pesaba: **la dirección inversa**. «¿Quién sigue a Ana?» (vecinos salientes) y «¿a quién sigue Ana?» no; «¿quién SIGUE a Ana?» (entrantes) son consultas distintas, y ambas deben ser baratas. Y todo esto debe persistir sin corromperse y recargar verificándose a sí mismo.

## 14.3 Modelo mental

Piensa en el **índice temático de un libro**. El libro tiene un texto corrido (las páginas, en orden) y, al principio, un índice con una entrada por término: «Arquitectura: 23-41. Árboles: 42-44. Bases de datos: 45-90». El índice no contiene el contenido: dice *dónde mirar*, y el rango es contiguo. Un término sin entradas no desaparece del índice: figura como «42-42», un rango vacío. Y el índice tiene exactamente una entrada por término, en orden fijo, más una marca final.

CSR es eso, en dos arrays:

```
aristas: 0→1, 0→2, 1→2, 2→0          (el "texto corrido")

forward_offsets = [0, 2, 3, 4]        el índice: num_nodes + 1 entradas
forward_targets = [1, 2, | 2, | 0]    cada fila, aplanada
                   └nodo 0┘ └nodo 1┘

vecinos(u) = targets[offsets[u] .. offsets[u+1]]     ← una resta, no una búsqueda
grado(u)   = offsets[u+1] - offsets[u]               ← ni siquiera tocas targets
```

El nodo 1 tiene `offsets[1]=3, offsets[2]=4`: un vecino, el `[2]`. El nodo aislado tendría `offsets[u] == offsets[u+1]`: rango vacío, «42-42». El último offset (`4`) es el número total de aristas: el índice termina donde termina el libro, y esa es una de nuestras invariantes.

¿Y la dirección inversa? Un índice de «temas → páginas» no sirve para preguntar «¿en qué temas aparece la página 77?». Para eso necesitas un segundo índice invertido. Por eso LiraDB mantiene **dos CSR espejo**: forward (aristas tal cual) y backward (aristas invertidas). El momento ¡ajá! del capítulo: la «lista» de adyacencia de un nodo **no existe como objeto**. No hay puntero, no hay alocación, no hay cabecera de `Vec`: hay dos números que delimitan un intervalo de un array único. Localizarla es O(1); recorrerla es escanear memoria contigua; y el hardware de memoria (prefetcher, líneas de caché) está construido exactamente para eso.

## 14.4 Primera solución

La solución que ya escribiste en el capítulo 8, y que es el punto de partida honesto:

```rust
// La solución ingenua: listas dinámicas por nodo (MemoryStore, cap. 8).
struct Adjacency {
    out: Vec<Vec<NodeId>>,   // out[u] = vecinos salientes de u
    ins: Vec<Vec<NodeId>>,   // ins[v] = vecinos entrantes de v
}
```

Correcta, simple, y con las dos direcciones bien resueltas. Para pruebas con cientos de nodos, imbatible en claridad. Los tests pasan. Durante un tiempo, nadie se queja.

## 14.5 Sus límites

Hasta que el grafo crece y el perfilador dice algo incómodo: el tiempo ya no se lo lleva el CPU «pensando», sino la memoria **esperando**. Hagamos la autopsia con el grafo de 1M nodos / 4M aristas (grado medio 4), en una máquina típica de 64 bytes por línea de caché y ~100 ns de latencia a RAM:

| | `Vec<Vec<NodeId>>` | CSR (una dirección) |
|---|---|---|
| Alocaciones | 1M + 1 | 2 (offsets + targets) |
| Memoria | ≈24 MB de cabeceras `Vec` (ptr+cap+len, 24 B c/u) + 1M trozos de heap (mínimo ~32 B cada uno con el overhead del allocador) ≈ **56-64 MB** | (1M+1)·8 + 4M·4 = **24 MB** |
| Recorrer TODA la adyacencia | 1M saltos a direcciones dispersas: ≥1M fallos de caché a RAM ≈ **≥0,1 s solo esperando** (más fallos de TLB) | escanear 16 MB contiguos: a ~10 GB/s, ≈ **1,6 ms**, con el prefetcher trabajando |
| Vecinos de un nodo | 1-2 fallos de caché (cabecera + datos) | casi siempre 1 línea de caché |

Dos órdenes de magnitud, y no por el espacio: por la **localidad**. Un `Vec<Vec>` es un millón de islotes en el heap; un CSR es una autopista. El prefetcher del CPU ve venir los `targets` secuenciales y los carga antes de que los pidas; ante un `Vec<Vec>`, cada `out[u]` es una sorpresa de dirección. Y hay un segundo límite, más silencioso: esa estructura no sabe persistirse. ¿Una página por nodo? 4 GB. ¿Serializar cada `Vec`? Un formato nuevo, solo para esto, que además arrastraría la dispersión al disco.

## 14.6 Solución evolucionada

La solución tiene tres gestos, y cada uno tiene su porqué con alternativa descartada.

**Gesto 1: aplanar a offsets + targets.** Sustituye N alocaciones por dos arrays densos. Es el CSR de Yale, 1977: `offsets: Vec<u64>` de longitud `num_nodes+1` (contadores de 64 bits, holgados para crecer), `targets: Vec<NodeId>` de longitud `edge_count` — el propio `NodeId` (`u32`) del cap. 7, 4 bytes por vecino. El constructor `from_edges` usa `Vec<Vec>` *temporalmente* — la solución ingenua no se tira: se convierte en una fase de construcción — y luego aplana. El coste dinámico se paga una vez, en el rebuild, no en cada consulta.

**Gesto 2: dos CSR espejo, forward y backward.** Podríamos guardar solo forward y responder «¿quién apunta a Ana?» escaneando las 4M aristas: O(E) por consulta, inaceptable. Reconstruir el backward on-demand: O(E) otra vez, cada vez. Kùzu guarda ambas columnas por cada tabla de relaciones, y nosotros heredamos la decisión. El precio es honesto y contable: ~8 bytes por arista en targets (4 en cada dirección) más otros 8 por nodo en offsets. Cambia `incoming(u)` de O(E) a O(1) + grado entrante. En una base de datos analítica, ese trueque se paga solo la primera tarde.

**Gesto 3: inmutabilidad por diseño — `replace`, no `update`.** Aquí está la decisión incómoda. Insertar una arista `(3, 7)` «in situ» significa escribirla en medio de `targets`, desplazando todo lo que viene detrás (O(E) entries), y ajustar todos los offsets posteriores. CSR no muta: se reconstruye. Por eso la API de persistencia es `replace(&nuevo_csr)` — rebuild completo — y no `add_edge`. ¿Cuándo es eso correcto? Cuando el CSR es una **proyección**: una estructura derivada, read-mostly, reconstruida por lotes — exactamente lo que necesitarán los algoritmos de la Parte V, que cargan el grafo una vez y consultan millones de veces. ¿Cuándo no? Cuando el grafo muta a ritmo de transacción; ahí hacen falta estructuras híbridas (regiones densas + zonas de desbordamiento, como Neo4j y Kùzu) que dejamos citadas en la §14.10. Elegir sabiendo cuál de los dos mundos habitas es el wisdom de este capítulo.

Y un cuarto ingrediente transversal: **las invariantes se verifican en cada puerta** — al construir, al persistir, al recargar. Ya viste por qué en los caps. 11-12: un formato que no se autocomprueba entrega corrupción con aspecto de dato válido.

## 14.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap14_csr.rs` (28 tests en `tests_csr`). Vamos por partes; todos los fragmentos salen de ese módulo.

### El CSR en memoria

```rust
pub struct Csr {
    pub num_nodes: u32,
    pub forward_offsets: Vec<u64>,   // len == num_nodes + 1
    pub forward_targets: Vec<NodeId>,// len == edge_count
    pub backward_offsets: Vec<u64>,
    pub backward_targets: Vec<NodeId>,
}
```

La construcción es un two-phase clásico: primero acumular (los `Vec<Vec>` de la §14.4, usados como andamiaje), luego aplanar:

```rust
let mut forward_offsets = Vec::with_capacity(num_nodes as usize + 1);
let mut forward_targets = Vec::with_capacity(edge_count as usize);
forward_offsets.push(0);
for list in &adj_out {
    forward_targets.extend_from_slice(list);
    forward_offsets.push(forward_targets.len() as u64);  // inicio acumulado
}
```

Fíjate: el offset de cada nodo se deriva de `targets.len()` tras volcar su lista — nunca se calcula a mano, así que no puede desincronizarse (la regla «derivar, no llevar en la cabeza» del cap. 11). El orden de inserción de las aristas se preserva dentro de cada nodo: determinismo que los tests explotan para verificar orden exacto. Y la consulta:

```rust
pub fn neighbors_out(&self, u: NodeId) -> &[NodeId] {
    if (u as u32) >= self.num_nodes || self.forward_offsets.len() < 2 {
        return &[];   // fuera de rango = nodo vacío, coherente con MemoryStore (cap. 8)
    }
    let start = self.forward_offsets[u] as usize;
    let end = self.forward_offsets[u + 1] as usize;
    if start > end || end > self.forward_targets.len() { return &[]; }  // guard
    &self.forward_targets[start..end]
}
```

Una resta y un slice: cero copias, cero punteros intermedios. El guard final es *defensivo* (un CSR verificado jamás lo activa); su trabajo es que la vía de consulta nunca haga `panic` aunque los arrays estén tocados — detectar la corrupción es trabajo de `verify()`, no del lector. `degree_out(u)` es simplemente la longitud del slice devuelto.

### `verify()`: seis invariantes y tres puertas

```rust
// 1-2. offsets.len() == num_nodes + 1            (forward y backward)
// 3.   offsets[i] <= offsets[i+1]                 (monotonía no decreciente)
// 4.   offsets[num_nodes] == targets.len()        (el índice termina donde el libro)
// 5.   todo target < num_nodes                    (IDs válidos)
// 6.   forward_targets.len() == backward_targets.len()   (misma cuenta de aristas)
```

`verify()` se llama en `from_edges()` (nada inválido nace), al inicio de `PersistentCsr::replace()` (nada inválido **llega a disco**) y al final de `load()` (nada inválido **sale de disco**). ¿Paranoico? Míralo al revés: cuesta O(V+E), lo que ya cuesta construir. Y el día que un bit gira en una página de targets, un CSR sin verificación te devuelve *vecinos equivocados que parecen válidos* — no un crash. Un error visible se depura en minutos; una mentira estructural, en semanas.

### Self-loops y multigrafo: admitidos, no tolerados

`from_edges` acepta `(0,0)` y acepta `(0,1)` dos veces. Un modelo que los rechazara te obligaría a *mentir sobre tus datos*: los grafos reales (follows, citas, interacciones proteicas) tienen autociclos y aristas paralelas, y Kùzu los soporta. CSR los absorbe sin un solo `if`: un self-loop `0→0` es un target más en el segmento del nodo 0 (y otro en el backward del 0); un duplicado es una entrada repetida en `targets`. Fíjate en la invariante 6: compara *longitudes* (cuenta de aristas), no conjuntos de IDs — precisamente porque el multigrafo hace que los conjuntos puedan diferir.

### Persistencia: chunks sobre slotted pages

Aquí se encadena todo el motor. El fichero tras un `replace()` de un grafo pequeño:

```
página 0         página 1            página 2         página 3       página 4         página 5
[metapágina]    [CsrHeader 24 B]    [chunk offsets]  [chunk tgts]   [chunk offsets]  [chunk tgts]
 cat. 11          catálogo CSR        forward (u64)   forward (u32)  backward (u64)   backward (u32)
                  num_nodes, edges,   ← start_page apuntado por el CsrHeader →
                  4 × *_page
```

La página 0 es la metapágina del cap. 11, intocable. La **página 1** guarda el `CsrHeader` (24 bytes little-endian: `num_nodes`, `edge_count` y las cuatro páginas donde arranca cada columna). ¿Por qué una página propia y no la metapágina? Porque la metapágina es el catálogo *genérico* del fichero — reutilizable por cualquier módulo — y el header CSR es específico de esta estructura. Es la misma separación cap. 12: cada quien su inventario.

Cada array viaja como **chunks**: porciones autocontenidas guardadas como record único dentro de una `SlottedPage`. El layout del chunk:

```
[kind: u8] [chunk_index: u32] [count: u32] [valores... little-endian]
 └── ChunkHeader: 9 bytes ──┘  u64 si es offsets, u32 si es targets
```

¿Por qué reutilizar `SlottedPage` en vez de un formato propio de página? Porque así el CSR hereda gratis todo lo construido: validación de cabecera y magic (cap. 11), paginación y free list (cap. 12), pin/unpin/evicción/métricas (cap. 13). El `kind` del chunk permite saber cómo interpretar los bytes al releer; el `chunk_index` viaja ya en disco **anticipando la segmentación** (hoy siempre es el chunk 0 de su cadena; el campo espera su momento).

### `PersistentCsr`: el ciclo create / replace / load / open

```rust
pub struct PersistentCsr<P: Pager> {
    pool: BufferPool<P>,
    header_page: PageId,   // siempre 1, primera data page
}
```

El ciclo de vida simétrico: `create` inicializa la página 1 con un header vacío; `replace(&csr)` reconstruye el mundo (verifica → codifica los 4 arrays como chunks → asigna una página por chunk → escribe cada chunk → **actualiza el header al final** → flushea todo); `load()` relee y verifica; `open` reabre un fichero existente comprobando que la página 1 está asignada. Fíjate en el orden de `replace`: **los datos antes que el metadato**. Si un rayo cae entre los chunks y el header, el header viejo sigue apuntando a páginas viejas completas: el grafo anterior sobrevive y las páginas nuevas quedan huérfanas. Leak, no corrupción — la misma asimetría que elegimos para la free list en el cap. 12, ahora a escala de estructura.

Dos convenciones sutiles del formato, ambas con porqué:

- **`start_page == 0` ⇒ array vacío.** Distingue «vacío intencional» de «corrupto» sin gastar páginas. Solo es segura porque la página 0 es *siempre* la metapágina y jamás sale de `allocate()` (cap. 12). Un formato vive de sus convenciones, y las convenciones viven de sus invariantes.
- **`num_nodes == 0` cortocircuita a `Csr::empty()`.** Hasta el grafo vacío tiene estructura: sus `offsets` son `[0]` — una entrada, rango vacío, «42-42». No corregirlo fue un bug real (la §14.15 lo cuenta).

### Los límites de esta versión: 500 y 1000

`OFFSETS_CHUNK_MAX = 500` y `TARGETS_CHUNK_MAX = 1000` están calibrados para que un chunk (9 bytes de cabecera + payload) quepa como record único en una página de 4.096 bytes: 9 + 500·8 + 4 (prefijo de longitud) + 10 (PageHeader) = 4.023 ≤ 4.096. Los máximos *reales* serían 509 y 1018 — los límites son deliberadamente conservadores, con margen para extensiones. Consecuencia honesta: esta versión persiste grafos de hasta ~500 nodos y ~1.000 aristas por columna; si los superas, `load()` falla alto (`Inconsistent`/`TooLarge` al verificar longitudes), nunca en silencio. La evolución anunciada — y el motivo de que el capítulo se llame «CSR, *segmentos*» — es encadenar chunks: un `next_page` en la cabecera del chunk, siguiendo la cadena hasta la página 0 como marcador de fin. El `chunk_index` ya viaja en disco esperándola. La segmentación no cambia la idea; solo alarga el array.

## 14.8 Prueba de fuego

La prueba no es «compila»: es que el grafo sobrevive al disco y **se conoce a sí mismo**. La cadena completa, de test real:

```rust
// End-to-end: pager en disco, persistir, cerrar, reabrir, leer. (Test real.)
let csr_in = Csr::from_edges([(0,1),(0,2),(1,2),(2,0),(2,1),(3,0)]).unwrap();
{
    let pager = FilePager::create(&path).unwrap();
    let mut p = PersistentCsr::create(BufferPool::new(pager, 16)).unwrap();
    p.replace(&csr_in).unwrap();
}                                    // drop: se cierra todo
let mut p2 = PersistentCsr::open(BufferPool::new(FilePager::open(&path).unwrap(), 16)).unwrap();
assert_eq!(p2.load().unwrap(), csr_in);   // idéntico, arista a arista
```

Y el escenario de fallo visible prometido: **el CSR con offsets que mienten**. Construye a mano un `Csr` con `forward_offsets = vec![3, 1, 2]` (monotonía rota) y `verify()` lo rechaza con `Inconsistent("forward_offsets monotonic")` — test `csr_verify_rejects_bad_offsets`. ¿Y si nadie verificara? Tres futuribles: offsets decrecientes → el guard de `neighbors_out` devuelve vacío (un nodo que «no tiene vecinos» — mentira piadosa que esconde la corrupción); último offset ≠ longitud de targets → lees basura contigua con aspecto de vecinos; offset desplazado pero *dentro de rango* → **respuestas incorrectamente correctas**: el BFS te devuelve el grafo equivocado sin un solo error. Por eso `verify()` está en las tres puertas y no solo en el constructor: la puerta que no verifica es la puerta por la que entra la corrupción.

La batería completa (`persistent_csr_replace_overwrites`, `persistent_csr_replace_rejects_invalid`, `persistent_csr_open_without_header_fails`, `csr_from_edges_duplicates`, `csr_verify_offsets_consistent_with_edge_count` con 50 aristas pseudoaleatorias sobre 20 nodos verificando que Σgrado_salida = Σgrado_entrada = edge_count…) congela cada promesa. ¿Síntoma si te saltas el capítulo? Consultas inversas O(E) «que funcionan» en tests pequeños, BFS que no escala porque el tiempo se lo llevan los fallos de caché, y adyacencias persistidas que nadie verifica al reabrir.

## 14.9 Qué hemos sacrificado

1. **Mutación incremental**: ni `add_edge` ni `remove_edge`. Un update in situ es O(E); un replace completo también, pero honesto y por lotes. Para mutación frecuente: híbridos (§14.10).
2. **El doble de espacio en adyacencia**: forward + backward ≈ 8 B/arista + 16 B/nodo. Compramos consultas bidireccionales O(1); no cabe otra cosa en esa factura.
3. **Páginas huérfanas en cada `replace`**: las páginas del CSR anterior no se liberan (el pager del cap. 12 ya sabe `free`, pero encadenarlo aquí exige atomicidad que llega con el WAL del cap. 28). Leak hasta la compactación del cap. 16.
4. **Límite single-chunk**: ~500 nodos / ~1.000 aristas por columna en esta versión. La segmentación es la evolución con nombre propio.
5. **`load` materializa el grafo entero**: no hay lectura perezosa por segmento; el CSR completo vive en RAM. Para grafos que no caben: proyección y streaming, Parte V (cap. 26).

## 14.10 Cómo lo hace una BBDD real

- **Kùzu** (paper CIDR 2023, Universidad de Waterloo) es el mapa de este capítulo a escala industrial: sistema **columnar** donde cada tabla de relaciones se guarda dos veces (una por dirección) como **listas de adyacencia estilo CSR**, y esas listas son sus *join indices* fundamentales — el motor escanea listas de adyacencia, las ordena y las interseca para unir nodos. El trabajo previo del mismo grupo («Columnar Storage and List-based Processing for GDBMSs», PVLDB 2021) formaliza la idea: los vecinos contiguos conviven en columnas comprimidas, y las listas se encadenan en segmentos cuando desbordan una región. Nuestro «chunk + segmentación futura» es la versión pedagógica de exactamente ese diseño.
- **Neo4j** elige el otro extremo del espectro: **registros de tamaño fijo** (históricamente ~15 bytes por nodo, ~34 por relación en los formatos clásicos) con las relaciones de cada nodo encadenadas como **lista doblemente enlazada** a base de punteros. Es «index-free adjacency»: mutación barata y punteros directos, a cambio de *pointer chasing* donde Kùzu escanea arrays. Densos nodos exigen registros de grupo especiales. CSR vs registros enlazados es la gran bifurcación del almacenamiento de grafos: escaneo rápido frente a mutación rápida.
- **Graphflow** (SIGMOD 2017, también de Waterloo) demostró el argumento de rendimiento en memoria pura: adyacencias CSR con listas **ordenadas por ID de vecino**, para que intersecar dos listas (el corazón del matching de subgrafos, WCOJ en el cap. 39) sea un merge lineal en vez de un baile de hash.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: para 1M nodos y 4M aristas, calcula los bytes exactos de `forward_offsets` + `forward_targets` y el total con backward. ¿A partir de cuántas aristas por nodo de media dejaría CSR de ahorrar memoria frente a la matriz de bits?
- *Intermedio*: nuestro `from_edges` preserva el orden de inserción por nodo. Modifícalo para que ordene cada segmento por target y escribe el test que lo congela. ¿Qué consultas de intersección se vuelven lineales (pista: Graphflow)? ¿Qué invariante NUEVA tendría sentido añadir a `verify()`?
- *Experto*: implementa la segmentación encadenada: `next_page: PageId` en `ChunkHeader`, cadena terminada en 0, lecturas acumulando longitudes. Que un grafo de 600 nodos persista y recargue idéntico (hoy falla). ¿Qué pasa con `chunk_index`?

## 14.11 Lo que te llevas

- **CSR**: adyacencia como dos arrays (`offsets` u64 de `num_nodes+1`, `targets` de `edge_count`); el vecindario de `u` es el intervalo `[offsets[u], offsets[u+1])` — O(1) para localizar, contiguo para recorrer. Formato de las matrices dispersas de los 70, corazón de Kùzu hoy.
- **Localidad es la decisión**: 2 alocaciones y 24 MB frente a 1M+1 alocaciones y ~60 MB dispersos; dos órdenes de magnitud en recorridos, por *dónde* vive la memoria, no por cuánta hay.
- **Forward y backward espejo**: ~8 B/arista extra que cambian `incoming(u)` de O(E) a O(1)+grado.
- **Inmutable por diseño**: `replace` (rebuild por lotes) para proyecciones read-mostly; la mutación frecuente exige híbridos — saber en qué mundo habitas es el trade-off.
- **Chunks sobre slotted pages**: reutiliza caps. 11-13 al completo; header de 9 bytes (`kind`, `chunk_index`, `count`), little-endian, un record por página, límites 500/1000 y segmentación como evolución con el campo ya reservado en disco.
- **`verify()` en las tres puertas** (nacer, persistir, recargar): seis invariantes baratas contra la corrupción silenciosa más peligrosa — la que responde mal sin fallar.
- **Convenciones con dientes**: `start_page == 0` ⇒ vacío (segura porque la página 0 es sagrada), datos antes que metadato en `replace` (leak, no corrupción).

## 14.12 Ojo, cuidado con…

- **Confundir chunk, record y página**: la página es la unidad de disco (cap. 11); dentro vive un record (con su prefijo de longitud); el chunk es el *contenido* de ese record. Tres niveles, un solo fichero.
- **«Verify es opcional»**: el guard de `neighbors_out` solo tapa offsets imposibles; los offsets *plausibles pero falsos* generan respuestas equivocadas en silencio. La única defensa es la puerta verificada.
- **Esperar updates baratos**: añadir una arista in situ es O(E). Si tu carga de trabajo es OLTP de grafos, CSR puro no es tu estructura — y forzarlo te convertirá en la persona que escribe «arreglé el BFS» cada viernes.
- **El grafo vacío no es la ausencia de grafo**: sus offsets son `[0]`. Tratar el vacío como «sin estructura» fue literalmente nuestro bug (§14.15).
- **`edge_count()` devuelve el mínimo** de los totales forward/backward: robustez ante corrupción que `verify()` rechazaría de todas formas. No lo leas como «el dato exacto» en un fichero tocado.

## 14.13 Pin de batalla

> *«Una matriz dispersa no se comprime para ahorrar bytes: se comprime para que las cosas que importan queden juntas. Lo mismo vale para un grafo.»*

## 14.14 Si solo lees 30 segundos

La adyacencia vive en dos arrays: `offsets` (una entrada por nodo más una final, inicios acumulados) y `targets` (los vecinos de todos los nodos, aplanados). Los vecinos de `u` son `targets[offsets[u]..offsets[u+1]]`: localizar es una resta, recorrer es memoria contigua — por eso CSR, nacido para matrices dispersas en 1977, es el estándar de los motores de grafos analíticos. LiraDB guarda dos CSR espejo (forward/backward), no muta: reconstruye (`replace`), y lo persiste como chunks en slotted pages sobre el buffer pool, con un header-catálogo en la página 1, seis invariantes verificadas en cada puerta, y la segmentación en cadena como evolución reservada.

## 14.15 Una historia pequeña

El test `persistent_csr_create_load_empty_roundtrip` falló el primer día, y su fallo nos enseñó más que cualquier acierto. Creábamos un `PersistentCsr`, hacíamos `load()` sin haber guardado nada, y comparábamos con `Csr::empty()`. El load devolvía unos offsets de longitud **cero**; `Csr::empty()` los tiene de longitud **uno**: `[0]`. ¿Quién tenía razón? `empty()`: hasta el grafo sin nodos tiene un índice con una entrada — la marca final que dice «aquí acaba un libro de cero páginas». El load, en cambio, había leído «no hay páginas asignadas» y traducido «no hay array». Casi: un array vacío y un array de un cero son estados distintos, y confundirlos hacía que hasta el roundtrip del vacío fallara. El arreglo fue una línea — cortocircuitar `load()` a `Csr::empty()` cuando `num_nodes == 0` — pero la moraleja se quedó: **el vacío también es un estado con estructura**, y los formatos que no lo definen explícitamente dejan que cada lector lo invente.

## Ejercicios resueltos

**1. Dadas las aristas `(0,1), (0,2), (1,2)`, escribe a mano los cuatro arrays del CSR y los grados de cada nodo.**

Forward: `adj_out = [[1,2],[2],[]]` → `forward_offsets = [0, 2, 3, 3]`, `forward_targets = [1, 2, 2]`. Backward: `adj_in = [[], [0], [0,1]]` → `backward_offsets = [0, 0, 1, 3]`, `backward_targets = [0, 0, 1]`. Grados: `degree_out = [2,1,0]`, `degree_in = [0,1,2]` — en este DAG las distribuciones quedan invertidas, pero la invariante es la suma. Comprueba que `offsets[3] = 3 = targets.len()` (la invariante 4) y que Σgrados = 3 = edge_count en ambas direcciones. Verifícalo con `cargo test persistent_csr_replace_keeps_invariants`, que congela exactamente estos arrays.

**2. ¿Por qué `OFFSETS_CHUNK_MAX` es 500 y no 509, y qué pasa exactamente si tu grafo tiene 600 nodos?**

Espacio útil de una página para un record único: 4.096 − 10 (PageHeader) − 4 (prefijo de longitud) = 4.082 bytes. El chunk mide 9 + n·8, así que n ≤ 509,125 → caben 509 offsets. Elegimos 500 por margen conservador (comentario del propio código). Con 600 nodos, `forward_offsets` tiene 601 entradas: 500 caben en un chunk, pero esta versión no encadena — `load()` solo lee la primera página, obtiene 500 offsets para 600 nodos, y `verify()` falla con `Inconsistent("forward_offsets length")`. Fallo alto, nunca silencioso. La salida es la segmentación del reto experto.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar los capítulos 11 y 12: explica por qué la convención `start_page == 0` ⇒ «array vacío» es segura, qué propiedad del pager la garantiza, y qué debería devolver `load()` si encontrara un header con `num_nodes = 0` pero `edge_count = 3`. Luego verifica tu respuesta leyendo `read_array_u64` y el chequeo del caso vacío en `load()`. Si no recuerdas quién posee la página 0, eso es señal de relectura, no de pista.

**Intermedio (spacing con el cap. 13).** Antes de ejecutar nada, predice cuántas `page_reads` y cuántos `buffer_misses` produce `load()` de un CSR recién persistido con un pool de capacidad 8. Escribe un test que lo compruebe con `p.pool().metrics()` (el patrón de `persistent_csr_pool_metrics_after_reload`). ¿Contaría igual una segunda `load()` inmediata? Razona con el pin/unpin del cap. 13, y explica por qué el header también genera una lectura.

**Experto.** Implementa la segmentación: añade `next_page: PageId` a `ChunkHeader`, haz que `replace()` escriba la cadena (último chunk con `next_page = 0`) y que `read_array_u64` la siga acumulando valores. Tu meta medible: que un CSR de 600 nodos (`(0..600).map(|i| (i, (i+1) % 600))`, un anillo) sobreviva al roundtrip en disco idéntico — hoy falla. Piensa antes: ¿qué longitud máxima pones ahora por chunk, cómo validas que la cadena no tiene ciclos, y qué invariante de `verify()` te protege si la cadena se corta a medias?

## Para profundizar

- **Eisenstat, Gursky, Schultz, Sherman — «Yale Sparse Matrix Package» (Yale DCS, informes RR-112 y RR-114, 1977)**: el origen documentado del formato «Yale»/CSR; los arrays de inicios acumulados de fila son literalmente nuestros `offsets`.
- **Wikipedia, «Sparse matrix»**: historia del formato (en uso desde mediados de los 60; primera descripción completa formal en 1967) y comparativa COO/CSC/CSR.
- **Feng, Jin, Chen, Liu, Salihoğlu — «Kùzu Graph Database Management System» (CIDR 2023, paper p48)**: columnar + listas de adyacencia CSR en ambas direcciones como join indices; el destino industrial de esta idea.
- **Gupta, Mhedhbi, Salihoglu — «Columnar Storage and List-based Processing for Graph Database Management Systems» (PVLDB 14(11), 2021)**: la fundamentación técnica de las listas encadenadas por columnas que inspiran nuestra segmentación.
- **Kankanamge, Sahu, Mhedhbi, Salihoglu, Roy, Özsu — «Graphflow: An Active Graph Database» (SIGMOD 2017)**: CSR en memoria con listas ordenadas por vecino e intersecciones lineales.
- **Petrov, «Database Internals», caps. 2-3**, y **Kleppmann, «DDIA», cap. 3**: la jerarquía de memoria y por qué la localidad decide — los números de la §14.5, con rigor.

## Mini-diálogo: en guardia nocturna

> — A ver: ¿he construido tres capítulos de motor para acabar serializando cuatro `Vec`? Podría haber hecho `bincode` y santas pascuas.

> — Podrías. Y el primer `load()` te devolvería cuatro arrays sin saber si mienten. ¿Quién te comprueba que el último offset cierra con la longitud de targets? ¿Quién distingue «array vacío» de «página corrupta»? ¿Quién pasa por el buffer pool, con sus métricas y su evicción?

> — Vale, el pool y el verify. Pero lo de `replace` me sigue doliendo: ¿reconstruir el grafo entero por cada cambio?

> — Eso no es un defecto del código, es la naturaleza del formato: CSR es inmutable como un PDF. Nadie «edita» un PDF párrafo a párrafo; lo regeneras. Para consultas analíticas —cargar una vez, preguntar un millón de veces— es exactamente la forma correcta de ser perezoso.

> — ¿Y cuando el grafo muta a cada rato?

> — Entonces no vives en el mundo CSR, y forzarlo sería diseñar con las manos atadas. Neo4j encadena registros mutables; Kùzu combina columnas densas con desbordamientos. Saber qué estructura habita tu carga de trabajo… eso no te lo da un capítulo, te lo da haber construido uno de los dos mundos y haber sentido dónde le dolía.

---

*(Próximo capítulo: 15 — Índices para encontrar datos. El CSR te dice en O(1) quiénes son los vecinos de un nodo… cuando ya sabes su ID. Pero ¿cómo encuentras el nodo cuyo `name` es «Ana» sin recorrerlos todos? Hash y B+ tree, también ellos sobre páginas y buffer pool.)*
# Capítulo 15 — Índices para encontrar datos (hash + B+ tree)

> *«El hash convierte "¿dónde está?" en aritmética. El orden convierte "¿qué viene después?" en lectura secuencial. Ninguna estructura te regala las dos cosas.»*

## 15.0 La anécdota de la esquina

Agosto de 1970, Seattle. Rudolf Bayer y Edward M. McCreight trabajan en los Boeing Scientific Research Labs y tienen un problema de los que definen una época: los índices ordenados en disco se les caían con los datos creciendo. Los árboles binarios de búsqueda, pensados para RAM, se degeneraban en listas; y cada inserción en un fichero ordenado empujaba media banda magnética de un lado a otro. Su respuesta fue un artículo —«Organization and Maintenance of Large Ordered Indices», presentado en el ACM SIGFIDET de 1970 y publicado en *Acta Informatica* en 1972— que describía una estructura donde cada nodo era **del tamaño de una página de disco**, no de un registro: el B-tree. Retrieval, inserción y borrado en tiempo proporcional a log_k(I), siendo I el tamaño del índice y k la capacidad de la página. Cincuenta y seis años después, esa estructura sigue siendo el índice por defecto de PostgreSQL, MySQL, Oracle y SQLite.

Y aquí viene la parte que gusta a todo el mundo: **nadie sabe a ciencia cierta qué significa la B**. Bayer y McCreight nunca lo explicaron en el paper. Douglas Comer, en su encuesta de 1979 «The Ubiquitous B-Tree» —el artículo que popularizó la estructura fuera del círculo académico—, dejó escrito que los autores jamás aclararon la letra, y barajó candidatos: *balanced* (balanceado), *broad*, *bushy*… o simplemente **Boeing**, su patrón. Hay quien apunta a *Bayer*. McCreight zanjó la pregunta años después con una broma que es mejor definición que la mayoría de libros de texto: cuanto más piensas en lo que significa la B, mejor entiendes los B-trees. En este capítulo vas a construir la mitad de esa idea —la mitad ordenada— con las manos, y la otra mitad (la que parte nodos) queda esbozada en el apéndice del capítulo.

La otra protagonista de hoy es más joven y más humilde: la función hash **FNV**. Nació en 1991 de unos comentarios de revisión al comité IEEE POSIX P1003.2 que enviaron Glenn Fowler y Phong Vo; Landon Curt Noll propuso la mejora que la hizo estable, y el nombre Fowler/Noll/Vo lo bautizó un correo electrónico. No es criptográfica ni pretende serlo: son diez líneas que dispersan claves sorprendentemente bien. Diez líneas que vamos a escribir con las manos, porque en un índice persistente el hash no es un detalle de implementación: **es parte del formato en disco**.

## 15.1 Objetivo

En el capítulo 14 persististe la topología del grafo (CSR): dado un nodo, recorrer sus vecinos es seguir offsets. Pero «¿qué nodo tiene `name = "Ada"`?» o «¿qué aristas pesan entre 5 y 9?» siguen siendo un escaneo completo. Este capítulo construye las dos estructuras que acaban con ese escaneo:

1. **`HashIndex`** — igualdad en O(1): FNV-1a propio, buckets que son páginas, desbordamiento encadenado con `next_page`, catálogo con magic «HID1».
2. **`BPlusTree`** — orden y rangos: una hoja ordenada (raíz = hoja) con búsqueda binaria y `range_scan`, catálogo con magic «BPLU», y la limitación single-level documentada para que el apéndice la derribe.

Ambos sobre `BufferPool<Pager>`: los índices son los primeros **clientes de toda la pila** que llevas construyendo desde el capítulo 9 (encode), el 11 (SlottedPage), el 12 (Pager) y el 13 (BufferPool).

## 15.2 Problema

La pregunta crítica de este capítulo, la que el CORPUS del libro clava con chinchetas: **¿cuándo hash index y cuándo B+ tree?** Para responderla hay que mirar las dos consultas que un grafo real recibe todo el día:

```text
Equality query:   ¿qué nodo tiene id_propiedad = 482?      → UNA clave exacta
Range scan:       ¿qué aristas tienen weight ENTRE 5 y 9?  → un INTERVALO en orden
```

Son gemelas y desdichadas: **ninguna estructura de datos responde las dos a la vez con coste mínimo**, y no por falta de ingenio, sino por una razón estructural:

- Para responder igualdad en O(1), el hash **destruye el orden a propósito**. La dispersión es su trabajo: la clave 500 y la 501 caen en buckets sin ninguna relación, porque si los vecinos cayeran cerca, los clusters matarían la distribución. Consecuencia: «dame las claves entre 500 y 600» obliga a escanear **todos** los buckets.
- Para responder rangos, necesitas los datos **ordenados**, y el orden no admite aritmética directa: no puedes calcular «dónde está García» con una multiplicación; tienes que **comparar** para navegar. Por eso el mejor coste de igualdad en una estructura ordenada es O(log n) — búsqueda binaria — y no O(1).

Esa tensión es la decisión de arquitectura del capítulo. Pero antes de decidir, demos números al dolor que queremos curar. Sin índice, una consulta por propiedad lee y decodifica todo: con 100.000 nodos (~40 bytes cada uno, ~100 nodos por página), son **~1.000 páginas de 4 KB (~4 MB) leídas y 100.000 registros decodificados y comparados por consulta**. Con un buffer pool de 64 frames, casi todas las lecturas van a disco: a ~7 ms por seek en un disco mecánico (la aritmética del capítulo 11), eso son ~7 segundos **por consulta**; en un SSD, decenas de milisegundos que se multiplican por cada usuario y cada query. Con índice, la parte de búsqueda cuesta **1 lectura de página** (el bucket o la raíz; alguna más si hay overflow), más 1 para el dato real. De 1.000 lecturas a 2-4: esa es la magnitud del capítulo.

## 15.3 Modelo mental

Dos edificios, dos filosofías.

**El `HashIndex` es un parking por matrícula hasheada.** El parking tiene 16 secciones (nuestros `DEFAULT_BUCKETS`). Al entrar, un cartel dice: «calcula `hash(matrícula) % 16` y aparca en esa sección». No buscas plaza: la sección **se calcula**. Si tu sección se llena, te manda a la sección ampliada de al lado con un puntero (el `next_page`). Ahora pregúntale al parking por «todas las matrículas entre 1234AAA y 1234ZZZ»: imposible. Los coches con matrículas vecinas están repartidos al azar por las 16 secciones. El parking responde una sola pregunta, y la responde volando.

**El `BPlusTree` es una agenda ordenada por apellido.** 203 nombres por página, alfabéticamente. Para encontrar «García» abres por la mitad, miras dónde caes («López» → quedó a la izquierda), descartas la mitad restante y repites: 8 aperturas como mucho para 203 nombres (2⁸ = 256 > 203). Y para listar a todos entre «Fernández» y «López» no abres por la mitad: pones el dedo en el primero y **deslizas**. La agenda responde las dos preguntas… pero ninguna con aritmética pura.

Así viven en disco:

```text
Fichero de un HashIndex (B = num_buckets):
┌────┬────┬──────────┬─────────┬─────────┬─────┬─────────┐
│ p0 │ p1 │ p2       │ p3      │ p4      │ ... │ p(2+B)  │
│meta│rsv │catálogo  │ bucket0 │ bucket1 │     │bucketB-1│──→ overflow
└────┴────┴──────────┴─────────┴─────────┴─────┴─────────┘   (colgado por next_page)

Página de bucket (4.096 B):                Raíz-hoja del B+Tree (4.096 B):
[PageHeader 10 B]                         [PageHeader 10 B]
[len=4 ][next_page: u32]   ← record 0     [len=16][magic "BPLU" | key_count]  ← record 0
[len=16][key u64 | value]  ← record 1     [len=16][key=5  | value]           ← record 1..N
[len=16][key u64 | value]  ← record 2     [len=16][key=10 | value]             ORDENADOS
[ ...hasta ~203 entradas ]                [ ...hasta 203 entradas... ]
```

El momento ¡ajá! es doble. Primero: el hash **no es un detalle, es formato en disco** — «clave 42 → página 10» debe ser verdad ayer, hoy y con otro proceso abierto, o el índice pierde datos en silencio. Segundo: la raíz del B+ es exactamente **la hoja de cualquier B+ tree real** — todo lo que aprendas aquí (orden, búsqueda binaria, recorrido) se transplantará sin cambios al árbol multinivel del apéndice.

## 15.4 Primera solución

La solución que ya tienes (y la que usa medio software del mundo sin decirlo): **escanear**. `WHERE p.name = "Ada"` lee las páginas de nodos una a una, decodifica cada registro (cap. 9) y compara. Funciona. Es correcta. Y el cap. 14 la usa para construir el CSR sin quejarse.

La mejora tentadora que escribe todo el mundo: cargar un `HashMap<u64, u64>` en RAM al arrancar, mapeando `id_propiedad → id_nodo`. Cinco líneas, velocidad absurda, cero páginas. Los tests pasan. Durante una semana, todo el mundo está contento.

## 15.5 Sus límites

Hasta que miras al `HashMap` con la honestidad de quien acaba de construir la pila de almacenamiento completa (caps. 9-13):

1. **Muere con el proceso.** Nada de eso está en disco. Cada arranque paga el escaneo completo otra vez: has optimizado la consulta y desoptimizado el arranque.
2. **No es persistible tal cual, y el intento corrompe.** `std::collections::hash_map::RandomState` siembra cada instancia con una semilla aleatoria (defensa contra HashDoS, y buena idea para lo suyo). Si hoy la clave 42 hashea al bucket 7 y ayer a la 3, no puedes haber escrito las entradas del bucket 7 en una página fija: **el índice en disco no puede depender del hasher del proceso**. Y aunque fijaras la semilla, `std` no promete estabilidad del output entre versiones de Rust: tu fichero dejaría de leerse con el primer `rustup`.
3. **No hay orden.** Ni rangos, ni mínimos, ni iterarlo en orden de clave sin copiar y ordenar.
4. **La memoria es el límite, no el disco.** Un índice que solo vive en RAM es un índice que se reconstruye a la velocidad de tu peor día.

El escaneo, por su parte, ya vimos su factura: ~1.000 lecturas de página y 100.000 comparaciones **por consulta**, creciendo linealmente con el grafo.

## 15.6 Solución evolucionada

La solución son dos estructuras, y cada decisión de diseño tiene alternativa descartada. Las importantes:

**FNV-1a escrito a mano, no `std`, no CRC.** Diez líneas: un offset basis, un primo, xor y multiplicación envolvente. Lo exigimos **puro**: constantes publicadas, cero semillas, cero estado del proceso. Así `clave → bucket` es una función matemática: mismas claves → mismo bucket, para siempre, en cualquier proceso y cualquier versión de Rust. ¿Y por qué no CRC, que también es determinista? Porque el CRC está diseñado para **detectar bits** (distancia de Hamming), no para dispersar: es lineal sobre GF(2) y claves consecutivas producen hashes aritméticamente relacionadas — justo el clustering que mata a una tabla con 16 buckets. La prueba del contrato son los vectores oficiales: `fnv1a_64(b"")` = offset basis, `"a"` = `0xAF63DC4C8601EC8C`, `"foobar"` = `0x85944171F73967E8`.

**Buckets + overflow encadenado, no open addressing.** Esta es la decisión donde la RAM y el disco disagree. En memoria, el probing (sondar la siguiente posición libre) es barato: una caché line. En disco, cada sonda es una **página entera leída en un offset aleatorio**: seek tras seek, la bestia del capítulo 11. El encadenado convierte la colisión en «sigue este puntero y lee la página siguiente»: una lectura, y dentro de ella caben ~203 entradas, así que la cadena es corta de nacimiento. Además, el probing degrada cerca de lleno y crecer exige rehashearlo todo; la cadena crece **una página cada 203 colisiones**, sin reconstruir nada.

**El B+ tree de un solo nivel, deliberadamente.** Nuestra raíz ES la hoja: pares ordenados en una página, búsqueda binaria para `get`, filtro ordenado para `range_scan`. ¿Por qué no splits ya? Porque splits + promoción de separadores + rebalanceo son ~300 líneas de complejidad que enterrarían la idea nueva del capítulo — el orden — bajo mecánica de mantenimiento. Lo que pierdes es medible y honesto: un tope de 203 entradas y un insert O(n) (desplazar registros en el `Vec` + reescribir la página entera). ¿Cuánto duele de verdad? Poco, en un dataset pedagógico: desplazar ~2 KB en RAM son microsegundos y la reescritura es una página de 4 KB. El dolor real no es la velocidad: es el **tope**. Cuando el dataset lo supere, el apéndice del capítulo (y el refuerzo ADR-005) tienen el paso a multinivel esperándote.

**Catálogos con magic: «HID1» y «BPLU».** La página 2 de cada fichero de índice guarda su catálogo (`num_buckets`, `key_count` para el hash; `key_count` para el B+) precedido de un magic de 4 bytes que deletrea algo legible en un hexdump: `0x4849_4431` («HID1») y `0x4250_4C55` («BPLU»). Misma filosofía que el `0xDA`/`0xFE` del cap. 11: si esos bytes no encajan, `open` devuelve `IndexError::Inconsistent("bad magic")` en vez de interpretar basura como índice. Coste: 4 bytes. Beneficio: corrupción detectada en la frontera.

**Un fichero por índice.** Ambos reclaman la página 2 (catálogo el hash, raíz el B+), así que en este capítulo **no coexisten en el mismo `BufferPool`**: cada índice vive en su fichero, y el test `hash_and_bplus_coexist` documenta la colisión con un comentario. La solución real —un catálogo global de índices en la página 1, que ya reservamos «para uso futuro»— es deuda apuntada al Apéndice D; desplazar la raíz del B+ a `3+B` habría acoplado ambos formatos para siempre.

**Errores tipados (`IndexError`).** `Io` (el entorno falló), `Inconsistent` (invariante rota tras reopen), `PageNotAllocated` (el catálogo apunta a una página fantasma), `InvalidParam` (raíz B+ llena, `num_buckets == 0`). El caller distingue «necesitas rebuild» de «disco roto» sin parsear strings.

Y con todo eso sobre la mesa, la respuesta operativa a la pregunta del principio, en forma de tabla de decisión:

| Tu consulta pregunta… | Índice | Coste | Qué sacrificas |
|---|---|---|---|
| «¿Dónde está la clave K?» (igualdad, masiva) | `HashIndex` | O(1) + cadena corta | Todo rango, orden, min/max, iteración |
| «¿Qué hay entre A y B?» / «el siguiente» / «ordena por» | `BPlusTree` | O(log n) + lectura secuencial del rango | La O(1) de la igualdad pura |
| Ambas, sobre la misma propiedad | Los dos (ficheros separados) | Lo mejor de cada uno | Espacio: indexas dos veces |
| Topología: «los vecinos de u» | Ninguno: el CSR del cap. 14 | O(grado) | Ya está resuelto — no lo re-indices |

## 15.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap15_indices.rs` (~650 líneas + 27 tests). Lo leemos por partes; todos los fragmentos salen de ese módulo.

### El hash determinista

```rust
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01B3);
    }
    h
}
```

Eso es todo: offset basis, xor del byte, multiplicación envolvente por el primo. Fíjate en `wrapping_mul`: en Rust el overflow de enteros en modo debug **asusta** (panic), y el corazón de cualquier hash es justo desbordarse sin culpa. El bucket se elige en `bucket_index`: `fnv1a_64(&key.to_le_bytes()) % num_buckets` — la clave se hashea **en su representación de disco** (8 bytes little-endian, cap. 9). Cambiar esa representación a big-endian reubicaría cada clave del índice: otra vez, el formato en disco manda.

### El catálogo, la entrada y la cabecera de bucket

`HashIndexHeader` (16 B LE: magic, `num_buckets`, `key_count`, reserved) se persiste como **único record** de la página 2. `HashEntry` son 16 B fijos (`key: u64`, `value: u64`) — anchura fija significa aritmética de capacidad exacta. Y la pieza que da personalidad al diseño: `BucketHeader` (4 B: `next_page: u32`) viaja como **primer record** de cada página de bucket. Si `next_page == 0`, la cadena termina. Es el patrón «primer record = header lógico»: no hay offsets mágicos dentro de la página; el orden de records y su tamaño discriminan lo que son.

### `create` y `open`: el layout

`create(pool, num_buckets)` rechaza `num_buckets == 0` con `InvalidParam`, extiende el pager hasta `3 + num_buckets` páginas, escribe el catálogo en la 2 y un `BucketHeader{next_page: 0}` en cada una de las páginas `3..3+B-1`, y flushea. Con los 16 buckets por defecto, tu fichero ya ocupa 19 páginas: **76 KB** aunque el índice esté vacío — el precio de comprar parking por secciones. `open(pool)` hace el camino inverso con la paranoia de siempre: ¿existe la página 2? (`PageNotAllocated` si no), ¿el payload mide 16? ¿el magic es «HID1»? ¿`num_buckets > 0`? ¿Existen todas las páginas de bucket? Cada respuesta negativa es un error distinto y tipado.

### `insert`: recorrer la cadena, reemplazar o colgar

Antes de la lógica, observa el **cómo** se lee cada página — es la disciplina pin/unpin del cap. 13 hecha carne:

```rust
let buf = self.pool.get_page(page_id)?;      // frame pineado
let bytes: [u8; PAGE_SIZE] = *buf;           // copia: suelta el borrow…
self.pool.unpin(page_id, false)?;            // …y despina YA, sin dirty
let sp = SlottedPage::decode(&bytes)...      // decodifica fuera del pool
```

Copia los bytes, despina inmediatamente, y trabaja sobre tu copia. Ningún índice retiene un frame mientras piensa: `PoolFullOfPinned` es la consecuencia de vivir del pool sin respetarlo, y el cap. 13 te lo enseñó a la manera difícil.

La lógica (l. 457-599) es un paseo por la cadena: en cada página, decodificar la `SlottedPage`, saltar el record 0 (`BucketHeader`), buscar la clave entre los `HashEntry`. Si aparece: reemplazo in-place de ese record y a casa. Si no aparece y hay `next_page`: siguiente página. Si se acaba la cadena: la entrada se añade a la última página visitada si cabe (el chequeo es aritmética del cap. 11: `used + need <= PAGE_SIZE`, con `need = 4 + 16`), y si no cabe, `allocate()` una página nueva, escribir en ella el `BucketHeader` + la entrada, y **recablear** el `next_page` de la última página:

```rust
// No cabe: aloca nueva página y cuélgala del header de la última.
let new_page = self.pool.pager_mut().allocate()?;
// (nueva página: BucketHeader{next_page: 0} + la entrada; write_record_page)
let mut bh_old = BucketHeader::decode(&records_mut[0])?;
bh_old.next_page = new_page;                 // recablear la cadena
records_mut[0] = bh_old.encode().to_vec();   // …y reescribir la página llena
```

El catálogo (`key_count`) se actualiza y flushea al final de cada inserción — tosco (una escritura de catálogo por insert) pero seguro; el cap. 28 (WAL) traerá la forma adulta de agrupar.

### El `BPlusTree`: orden en una página

El struct mantiene las entradas **cacheadas en memoria** (`entries: Vec<TreeEntry>`) y las reescribe enteras en cada `persist`. Las dos operaciones que lo definen:

```rust
pub fn get(&self, key: u64) -> Option<u64> {
    match self.entries.binary_search_by_key(&key, |e| e.key) {
        Ok(idx) => Some(self.entries[idx].value),
        Err(_) => None,
    }
}
```

Búsqueda binaria del estándar sobre el invariante «ordenado por clave»: O(log n), unas 8 comparaciones para 203 entradas. Y el insert (l. 874-896) reutiliza la misma búsqueda: `Ok(idx)` → reemplazo in-place (y devuelve `false`: no añadió); `Err(idx)` → el punto de inserción ya está calculado, se comprueba la capacidad y se hace `entries.insert(idx, …)` — que desplaza a la derecha todo lo posterior. Ahí está el O(n) del single-level, visible y sin disimulo. La capacidad se **deriva**, no se estima a ojo:

```rust
let usable = PAGE_SIZE - PageHeader::SIZE - 4 - BPlusHeader::SIZE;
usable / (4 + TreeEntry::SIZE)      // (4.096 - 10 - 4 - 16) / 20 = 203
```

Y `range_scan(lo, hi)` es un filtro inclusivo en ambos extremos sobre las entradas ordenadas — con `lo > hi` devuelve vacío.

### `create` con bucle, `persist` con header de record

Dos detalles del código que son lecciones en sí mismos. Primero, `BPlusTree::create` asegura la página raíz así:

```rust
while !pool.pager().is_allocated(root_page) {
    pool.pager_mut().allocate()?;
}
```

¿Por qué un `while` y no un `allocate()` solitario? Porque la primera versión hacía exactamente eso y fallaba: un pager recién creado tiene solo la página 0 (la metapágina, cap. 12), así que un único `allocate()` entrega la página **1** — la reservada — y la 2 sigue sin existir. `allocate` es secuencial: reparte 1, luego 2. El `while` hasta `is_allocated` es un contrato («esta página existe») en vez de una suposición («habrá bastado una»). Segundo, `persist` escribe el `BPlusHeader` como **primer record** y cada `TreeEntry` como record independiente — lo que permite a `open` distinguirlos por tamaño y validar que `key_count` coincide con el número real de records. Que ese diseño es load-bearing lo aprendimos por la vía dura: la historia de la sección 15.15.

## 15.8 Prueba de fuego

La batería de `tests_index` (27 tests) verifica las promesas una a una. Las que debes poder citar de memoria:

```rust
// El contrato del hash: vectores oficiales FNV (cualquier desviación = bug).
assert_eq!(fnv1a_64(b""), 0xCBF2_9CE4_8422_2325);
assert_eq!(fnv1a_64(b"a"), 0xAF63_DC4C_8601_EC8C);
```

```rust
// Roundtrip REAL en disco: create → insert → flush → cerrar → reabrir → get.
let mut h2 = HashIndex::open(pool2).unwrap();
assert_eq!(h2.bucket_count(), 8);
for &k in &keys { assert_eq!(h2.get(k).unwrap(), Some(k * 3)); }
assert_eq!(h2.get(999_999).unwrap(), None);      // las que no están, no están
```

```rust
// El rango: inclusivo en ambos extremos, vacío si lo > hi, completo si abarca.
let r = t.range_scan(4, 12);
let got: Vec<u64> = r.iter().map(|e| e.key).collect();
assert_eq!(got, vec![5, 7, 9, 11]);
```

```rust
// Corrupción detectada: el magic del B+ vive en el offset 14 de la página.
buf[14..18].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
// ... y open() responde Err(IndexError::Inconsistent("bplus root: bad magic"))
```

Una curiosidad honesta sobre los nombres: `hash_insert_many_triggers_overflow_chain` inserta 200 claves en 2 buckets — 100 por bucket — y **no llega a desbordar** (caben 203 por página): lo que el test realmente certifica es que la cadena, corta, responde todas las claves. El punto exacto donde salta la primera página de overflow lo encontrarás tú, con lápiz, en el ejercicio esencial. Completa la batería con `hash_insert_replaces_existing` (upsert sin inflar `key_count`), `hash_open_without_catalog_fails` y `bplus_open_without_root_fails` (ambos `PageNotAllocated(2)`), y `hash_and_bplus_coexist` (la no-coexistencia, documentada).

¿Y si te saltas este capítulo? Los síntomas llegan con factura: cada filtro por propiedad escanea el grafo entero (latencia que crece linealmente con tus datos), y si improvisas un índice con un hasher sembrado, te espera algo peor — un `get` que devuelve `None` para claves que están en disco, con pinta de bug de la aplicación y olor a corrupción silenciosa.

## 15.9 Qué hemos sacrificado

1. **Índices estáticos**: «actualizar» es rebuild (`create` + `insert*` + `flush`). Los índices dinámicos en línea son el cap. 28; y el rebuild deja páginas huérfanas que el cap. 16 aprenderá a ver y recoger.
2. **Sin rehash**: si las cadenas crecen, no hay linear hashing ni split de buckets — la respuesta honesta hoy es rebuild con más buckets. ¿Cuándo es urgente? Cuando el factor de carga (claves/buckets) supera ~1: las cadenas empiezan a costar 2-3 páginas por `get`.
3. **B+ single-level**: tope de 203 entradas, insert O(n), sin deletes. El apéndice del capítulo es el mapa de la salida.
4. **Catálogo re-escrito en cada insert** del hash (y raíz entera en cada insert del B+): durabilidad por bravery, no por diseño; el WAL del cap. 28 lo convertirá en lotes.
5. **Un fichero por índice**: la página 2 tiene dos dueños posibles y solo uno vive en cada fichero. El catálogo global (página 1) queda como deuda declarada.
6. **Claves y valores de 8 bytes fijos**: sin claves de longitud variable ni compresión (vecinos que comparten prefijos, como hacen los B+ reales — cap. 38 para compresión).

## 15.10 Cómo lo hace una BBDD real + retos

- **PostgreSQL** es la lección completa de «cuándo cada uno». Su índice por defecto es un **B-tree** (variante B-link de Lehman-Yao, con páginas de 8 KB): responde igualdad *y* rangos *y* orden, y desde la v13 comprime claves duplicadas (deduplicación con *posting lists*), exprimiendo que las hojas están ordenadas. ¿Tiene hash indexes? Sí — pero durante años **no se logueaban en WAL**: no sobrevivían a un crash ni se replicaban, y la documentación los trataba como ciudadanos de segunda. Desde la v10 sí son crash-safe… y siguen sin ser el default, porque un índice que solo hace igualdad compite contra uno que hace igualdad + rangos en el mismo espacio. Ese es el veredicto del mercado que este capítulo ha construido a mano.
- **InnoDB (MySQL)** apuesta todo al B+ con páginas de 16 KB: la tabla entera ES un índice clustered por clave primaria (las filas viven en las hojas), y los índices secundarios guardan la PK como valor — dos índices, dos B+ trees, un solo orden físico. Fan-outs de cientos de punteros por nodo: árboles de 3-4 niveles para millones de filas.
- **Neo4j** indexa propiedades con **B+ trees nativos** (los *schema indexes* de etiqueta/propiedad), recorre etiquetas con *label scan stores*, y delega el texto completo a Lucene. La igualdad exacta sobre propiedades — nuestra `equality query` — cae en esos B+ o en los *native token lookup*; el hash puro es menos protagonista en grafos porque las consultas de propiedades quieren también rangos (`age > 30`) y orden.
- **Kùzu/Ladybug**, nuestra referencia de la Parte III: los hash *joins* materializan tablas hash exactamente con la estructura de buckets + overflow que acabas de escribir — la diferencia es que la suya vive en memoria de trabajo y la tuya sobrevive al proceso.
- **SQLite**: un B+ tree por tabla y por índice, páginas de 4 KB, interiores y hojas — la versión completa de lo que el apéndice esboza.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: con `num_buckets = 16`, ¿cuántas claves (bien distribuidas) caben antes del primer overflow? ¿Y cuál es la huella mínima en disco del índice vacío? Justifica ambas con aritmética de páginas, no con estimaciones.
- *Intermedio*: el catálogo del hash se re-escribe y flushea en cada `insert`. Diseña un `insert_batch(keys: &[(u64, u64)])` que agrupe: ¿qué garantía pierdes si el proceso muere a mitad del lote? ¿Leak o corrupción? (Recuerda la regla del cap. 12: cuando no puedas garantizar atomicidad, prefiere leak.)
- *Experto*: el puente al apéndice — da el primer paso a multinivel: cuando la raíz-hoja se llene, reparte sus entradas en dos hojas, promueve el separador y convierte la página 2 en nodo interno. Pista de diseño: la raíz se queda en la página 2; solo cambia su contenido.

## 15.11 Lo que te llevas

- **Igualdad quiere dispersión; rango quiere orden. Ninguna estructura da ambas gratis** — el hash destruye el orden a propósito, y el orden prohíbe la aritmética directa. Elegir índice es elegir qué pregunta quieres que sea barata.
- **El hasher de un índice persistente es formato en disco**: FNV-1a puro (constantes fijas, vectores verificables), semillas aleatorias o hashers versionados = pérdida silenciosa tras reopen.
- **En disco, encadena; no sonde**: la unidad de I/O es la página — cada sonda de open addressing es un seek; la cadena de overflow es una lectura con ~203 entradas dentro.
- **La raíz-hoja ordenada es la mitad de un B+ tree**: búsqueda binaria O(log n) y range scan salen del mismo invariante. La otra mitad (splits) tiene su mapa en el apéndice.
- **«HID1» y «BPLU»**: 4 bytes que convierten «leer basura con confianza» en `IndexError::Inconsistent`.
- **Los índices son clientes de toda la pila**: caps. 9 (LE de la clave), 11 (records y prefijos), 12 (allocate/is_allocated), 13 (pin/unpin/flush). Ninguno abre el fichero directamente.

## 15.12 Ojo, cuidado con…

- **El magic en el hexdump sale invertido**: guardamos little-endian, así que «HID1» aparece como `31 44 49 48` («1DIH») y «BPLU» como `55 4C 50 42`. La primera vez confunde a todo el mundo.
- **`HashIndex::insert` no devuelve el valor anterior**: el doc-comment lo promete, pero el código devuelve el valor **nuevo** como marca de «estaba» (y un comentario lo confiesa: «no se usa realmente»). El B+ sí tiene el contrato limpio (`true` = añadió, `false` = reemplazó). Arreglar el del hash es un buen primer commit.
- **Cambiar la representación de la clave rompe el índice**: el hash opera sobre `key.to_le_bytes()`. Un cambio a big-endian reubica todas las claves — es la lección del cap. 9 reaparecida dentro del hasher.
- **Más buckets no es gratis**: cada bucket es una página de 4 KB aunque nunca reciba una clave. 16 buckets = 76 KB mínimos; 1.024 buckets = 4 MB. El `num_buckets` es una apuesta sobre el tamaño futuro del dataset.
- **Confundir factor de carga con capacidad de página**: la capacidad (203 entradas/página) es aritmética fija; el factor de carga (claves/buckets) es la decisión que decide cuánto miden las cadenas.

## 15.13 Pin de batalla

> *«Un índice no hace tus datos más rápidos: decide qué datos NO necesitas mirar. El hash compra preguntas exactas vendiendo el orden; el B+ compra el orden vendiendo la aritmética. Antes de indexar, decide qué factura quieres pagar.»*

## 15.14 Si solo lees 30 segundos

Sin índice, cada consulta por propiedad escanea el grafo (~1.000 páginas y 100.000 comparaciones para 100k nodos). El **`HashIndex`** la convierte en aritmética: FNV-1a determinista elige un bucket (una página, con hasta 203 entradas de 16 B), y el desbordamiento se encadena con `next_page` — en disco se encadena porque cada sonda alternativa sería un seek. El **`BPlusTree`** mantiene las entradas **ordenadas** en una hoja-raíz: búsqueda binaria para la igualdad y `range_scan` para los intervalos, con un tope honesto de 203 entradas hasta el multinivel del apéndice. Ambos persisten un catálogo con magic («HID1»/«BPLU») en la página 2, validan al `open` y devuelven `IndexError` tipados. Igualdad → hash; orden y rangos → B+. Ese es el capítulo.

## 15.15 Una historia pequeña

La primera versión de `BPlusTree::persist` hacía lo más natural del mundo: concatenar header y entradas en un único blob y escribirlo como un solo record de la `SlottedPage`. Compilaba, los tests en memoria pasaban — el árbol guardaba sus entradas y las devolvía. El drama llegó con el test de persistencia: cerrar el fichero, reabrir, y el árbol aparecía **vacío**. `key_count` decía cero, `range_scan` no devolvía nada… pero el fichero en disco seguía creciendo con cada insert, así que los datos *estaban* en alguna parte. La autopsia fue humillante en su simplicidad: `open` leía el primer record, veía 16 bytes, los interpretaba como el `BPlusHeader`, e ignoraba educationadamente todo lo demás — las entradas quedaban en disco como un apéndice ilegible. La lección se quedó grabada como patrón del módulo: **el formato debe poder distinguir sus piezas por sí mismo** — header como primer record, cada entrada como record independiente, tamaños que discriminan — porque un formato que solo entiende su escritor no es un formato: es una memoria privada con suerte. Y de propina, una regla que ya sabías del cap. 12: los tests en memoria no validan persistencia; solo el disco reabierto dice la verdad.

## Apéndice del capítulo — El paso a multinivel (a dónde va el B+ tree)

La limitación single-level está declarada en el propio módulo: la raíz se llena a las 203 entradas y `insert` responde `IndexError::InvalidParam("bplus root full (single-level cap; rebuild required)")`. Este apéndice no la implementa: dibuja el camino, para que el reto experto y el refuerzo del ADR-005 tengan mapa. El truco en una frase: **la raíz no se muda; cambia de oficio**.

1. **Split de hoja**: cuando la hoja se llena, se aloca una segunda, se reparten las 203 entradas ordenadas por la mediana (≈102 + 101), y la clave que queda en la frontera se **promociona** como separador.
2. **La raíz se convierte en nodo interno**: la página 2 deja de guardar entradas y pasa a guardar pares *(separador, página_hoja)*: «claves < 507 → hoja A; claves ≥ 507 → hoja B». Fíjate en lo que NO cambia: la dirección de la raíz. Por eso elegimos raíz fija — el multinivel crece **hacia abajo** sin reescribir punteros externos.
3. **Las hojas se enlazan** con `next_page` — exactamente el patrón que el `BucketHeader` del hash ya usa — para que el `range_scan` multi-hoja sea deslizar el dedo, no volver a subir al nodo padre por cada hoja.
4. **Cascada**: si un nodo interno se llena, se parte igual y promociona otro separador; la altura solo crece cuando TODO el nivel está lleno — por eso un B+ de páginas de 4 KB con fan-out ~200 alcanza millones de claves en 3 niveles: 200³ = 8 millones.
5. **Lo que la ordenación regala**: claves vecinas comparten prefijos (y a menudo son duplicados) — ahí viven la deduplicación de PostgreSQL 13 y la compresión de prefijos de los B+ comerciales.

Coste estimado del salto, según el propio módulo: ~300 líneas. Complejidad nueva: splits, promoción, rebalanceo… y después, concurrencia sobre nodos que se parten (el *latch crabbing* de Lehman-Yao que usa PostgreSQL). Todo eso ya es otro capítulo — este solo quería que supieras exactamente qué hay detrás de la puerta.

## Ejercicios resueltos

**1. ¿En qué inserción exacta aparece la primera página de overflow de un `HashIndex` con un solo bucket, y cuántas entradas caben en la raíz de un `BPlusTree`?**

Aritmética de páginas, cap. 11 en la mano. Bucket: `PageHeader` 10 + record 0 (prefijo 4 + `BucketHeader` 4) = 18 B de overhead; cada entrada cuela 4 + 16 = 20 B. La página admite n entradas mientras 18 + 20n ≤ 4.096 → n ≤ 203,9 → **203 entradas**; la 204.ª encuentra `used + need = 18 + 4.060 + 20 > 4.096` y dispara `allocate()`: el fichero pasa de 4 páginas (0,1,2,bucket) a 5. B+: 10 + (4+16) del header + 20 por entrada → (4.096 − 30)/20 = 203,3 → también **203**. Los dos índices comparten tope por casualidad de overheads similares — pero no por la misma razón: el bucket gasta 8 B en su cabecera de cadena; la raíz B+, 20 en su catálogo con magic. Verificación: un test con `num_buckets = 1` observando `pool().pager().num_pages()`.

**2. En `bplus_disk_bad_magic_fails` se corrompen los bytes 14..18 de la página 2. ¿Por qué ahí y no en 0..4? ¿Qué testearía corromper 0..4?**

Porque el layout de la página es `[PageHeader: 10 B][prefijo de longitud: 4 B][BPlusHeader: 16 B]` — el magic del B+ empieza en el byte 10 + 4 = **14**. Corromper 0..4 tocaría el magic del `PageHeader` del cap. 11: ruta de error distinta (`SlottedPage::decode` fallaría antes de llegar al B+), y el test demostraría otra detección, no la del catálogo B+. Es la lección del «detectar el magic exige saber dónde vive» — la versión cap. 15 de la documentación de offsets del cap. 11. El comentario del test, que deja el cálculo escrito, previene la próxima hora perdida.

## Ejercicios propuestos

**Esencial (retrieval + spacing).** Sin mirar los caps. 11 ni 15: predice, con lápiz, en qué inserción exacta salta la primera página de overflow de un `HashIndex` creado con `num_buckets = 1`. Después verifica con un test que inserte claves y observe `pool().pager().num_pages()` — el fichero crece de 4 a 5 páginas en un punto muy concreto. Si tu predicción no sale de la aritmética de records (cuánto cuela cada uno dentro de una `SlottedPage`), eso es señal de relectura, no de pista.

**Intermedio (interleaving: hash + orden).** Inserta las claves 0..1.000 con valores `k*7` en un `HashIndex` y en un `BPlusTree` (pagers separados — ya sabes por qué). Escribe un test que: (a) ejecute `range_scan(137, 731)` sobre el B+ y verifique con `get` del hash cada clave devuelta y su valor; (b) compruebe que 136 y 732 existen en ambos índices pero no aparecen en el rango; (c) responda por escrito: ¿por qué es imposible escribir este test usando un solo índice? ¿Qué tendría que hacer el hash para responder el rango, y cuánto le costaría?

**Experto (el puente al multinivel).** Implementa el primer split según el apéndice: cuando `insert` detecta la raíz llena (203 entradas), reparte por la mediana en dos hojas nuevas, promueve el separador y reescribe la página 2 como nodo interno *(separador, página)*. Restricciones: la raíz no se mueve de la página 2; los magics siguen validándose al `open`; `range_scan` sigue devolviendo rango completo (ahora navegando hojas); todos los tests existentes pasan, y añades uno de roundtrip en disco con más de 203 claves. Bonus: usa el campo `reserved: u64` del `BPlusHeader` como puntero a la primera hoja.

## Para profundizar

- **Bayer & McCreight, «Organization and Maintenance of Large Ordered Indices»** (*Acta Informatica* 1(3), 173-189, 1972; presentado en ACM SIGFIDET 1970) — el paper original, corto y sorprendentemente legible, de los Boeing Scientific Research Labs.
- **Douglas Comer, «The Ubiquitous B-Tree»** (*ACM Computing Surveys* 11(2), 1979) — la encuesta que popularizó la estructura y dejó escrito el misterio de la B.
- **El sitio oficial de FNV** (isthe.com/chongo/tech/comp/fnv) y el **draft IETF draft-eastlake-fnv** — historia (1991, POSIX P1003.2), constantes y vectores de test que usa `fnv1a_known_values`.
- **PostgreSQL**: documentación de índices (§11.2: la historia de los hash sin WAL pre-v10 y por qué B-tree es el default) y `src/backend/access/nbtree/README` (la variante Lehman-Yao y la deduplicación de la v13).
- **Alex Petrov, «Database Internals» (O'Reilly), caps. 2-4** — B-tree basics, formatos de fichero y la implementación completa de un B-tree con page splits: el apéndice de este capítulo en versión larga.
- **CMU 15-445** — las lecciones de tree indexes: el B+ tree multinivel con splits, explicado con la misma filosofía de páginas que hemos seguido aquí.

## Mini-diálogo: en guardia nocturna

> — A ver: tengo un hash O(1). ¿Para qué quiero un B+ tree que es O(log n)? O(1) gana a O(log n) en cualquier pizarra.

> — En la pizarra sí. Ahora pregúntale a tu hash por «todas las aristas con peso entre 5 y 9». Se rasca la cabeza y te escanea los dieciséis buckets, uno a uno.

> — …porque el bucket del 5 no tiene nada que ver con el del 6.

> — Exacto: la dispersión no es un defecto del hash, es su producto. La clave 500 y la 501 caen en mundos distintos porque si cayeran cerca, los clusters las lian. Elegir índice no es elegir el más rápido: es decidir qué pregunta quieres que sea barata. Igualdad exacta y masiva → hash. Rangos, orden, «dame el siguiente» → B+.

> — Y lo del FNV escrito a mano, ¿no era orgullo de más? `HashMap` viene con el idioma.

> — `HashMap` viene con una semilla aleatoria por instancia. En RAM es una virtud: nadie te clava un HashDoS. En disco es una sentencia: la clave 42 vivía en el bucket 7 ayer, hoy el hasher la manda a la 3, y tu `get` devuelve `None` para un dato que está ahí, en el fichero, perfectamente sano. Pérdida silenciosa con traje de bug de negocio. Por eso el hasher de un índice persistente es formato en disco: constantes publicadas, cero semillas, verificado contra vectores oficiales.

> — Y la B del B-tree, ¿al final qué era?

> — Boeing, balanced, bushy, Bayer… McCreight nunca lo dijo. Y cuanto más piensas en lo que significa, mejor entiendes los B-trees. Con eso te quedas esta noche.

---

*(Próximo capítulo: 16 — Compactación y mantenimiento. Los índices son estáticos y «actualizarse» es rebuild: ahora hay páginas huérfanas por el fichero, `free_space` que miente y basura que acumular. Llega `liradb inspect|check|compact` — y de regalo, los LSM-trees mirando desde la otra acera.)*
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
# Capítulo 17 — Diseñar un lenguaje pequeño (MATCH-WHERE-RETURN mini)

> *«Un lenguaje de consulta no se programa: se diseña. Programarlo es el capítulo 18.»*

## 17.0 La anécdota de la esquina

En mayo de 1974, en un taller de ACM SIGMOD en Ann Arbor (Michigan), Donald Chamberlin y Raymond Boyce presentaron un paper con un título optimista: *«SEQUEL: A Structured English Query Language»*. Trabajaban en IBM sobre el modelo relacional que Edgar F. Codd había publicado cuatro años antes, y su apuesta era radical para la época: que el usuario escribiera **frases casi inglesas** diciendo *qué* datos quería — `GET name OF employees WHERE dept = "toy"` — y que una máquina decidiera *cómo* buscarlos. SEQUEL se rebautizó después como SQL porque, según cuenta la historia oral del propio Chamberlin, la palabra estaba registrada como marca por una empresa aeronáutica británica llamada Hawker Siddeley. El nombre cambió; la idea no: **declarar en vez de programar**. SQL terminó siendo estandarizado por ANSI en 1986 e ISO en 1987, y es el lenguaje de datos más exitoso de la historia.

Treinta y siete años después, en 2011, un ingeniero de Neo4j llamado Andrés Taylor se enfrentó al mismo problema… con grafos. Copiar SQL era posible, pero JOIN tras JOIN esconde la forma del grafo tras columnas. Su respuesta, Cypher, apostó por algo visual: que **el patrón se dibuje a sí mismo** con paréntesis y flechas —

```text
(aniversario:Persona)-[:CONOCE_A]->(invitado:Persona)
```

— es literalmente ASCII-art del subgrafo que buscas. El paper de referencia lo llama así: *ASCII-art graph pattern matching* (Francis et al., SIGMOD 2018). En 2016 Neo4j liberó el proyecto openCypher para que otros motores adoptaran la sintaxis, y en abril de 2024 esa línea desembocó en **GQL (ISO/IEC 39075)**, el primer lenguaje de consulta de bases de datos estandarizado por ISO desde SQL en 1987. Casi cuatro décadas para que naciera el segundo.

Este capítulo abre la Parte IV: LiraDB ya sabe **guardar** un grafo (caps. 11-16); ahora aprenderá a que se le **pregunte**. Y lo hará igual que SEQUEL y Cypher: primero el diseño del lenguaje —vocabulario, gramática, estructura, errores—, después el código. Hoy, cero lexer, cero parser, cero ejecución. Solo el contrato.

## 17.1 Objetivo

Al terminar este capítulo habrás **diseñado LiraQL**, el mini-Cypher de LiraDB: un lenguaje de consulta declarativo reducido a tres cláusulas. En concreto, fijarás cuatro piezas que viven en `liradb-workspace/crates/vol2-liradb/src/cap17_liraql_ast.rs`:

1. La **gramática EBNF** — qué secuencias de texto son consultas válidas.
2. El **vocabulario de tokens** (`TokenKind`) — qué átomos existen.
3. El **AST** (`Expression`, `PathPattern`, `AstNode`, `Query`) — qué estructura significativa tiene una consulta.
4. Los **errores con posición** (`Span`, `QueryError`) y la **forma canónica** (`Display`) — qué mensajes ve el usuario y cómo se normaliza su consulta.

Lo que NO harás: tokenizar, parsear, planificar ni ejecutar. Eso son los caps. 18, 19 y 20. Este capítulo compila 970 líneas Rust sin dependencias externas y sin ejecutar una sola consulta — y eso es exactamente lo que debe hacer un capítulo de diseño.

## 17.2 Problema

La Parte III cerró con un motor completo: páginas, buffer pool, CSR, índices, mantenimiento. Ana puede guardar un grafo de personas y relaciones en disco. Ahora quiere preguntar: *«¿a quién conoce Ana?»*. ¿Cómo se lo dice a LiraDB?

Opción A: una **API de funciones**. Opción B: un **lenguaje**. La tentación es A, porque ya sabes Rust. Pero fíjate en quién decide *cómo* buscar la respuesta: con una API, el usuario escribe el **cómo** («recorre la adyacencia de Ana, filtra por tipo CONOCE_A, proyecta el nombre») y cada consulta es un programa; con un lenguaje, el usuario declara el **qué** — `MATCH (a:Persona {nombre: "Ana"})-[:CONOCE_A]->(f) RETURN f.nombre` — y el *cómo* puede decidirse después, y decidirse **mejor**: ese es el trabajo del optimizador del cap. 21.

Esta es la razón profunda por la que los lenguajes declarativos ganaron a los imperativos para consultar datos, y no es una moda: Codd la articuló en 1970. Si el usuario fija la estrategia de acceso, ningún sistema puede mejorarla más tarde; si solo declara el resultado deseado, el motor puede reordenar, usar índices (cap. 15), empujar filtros… sin cambiar una coma del significado. Un lenguaje es, además, **texto**: se teclea en la CLI (cap. 31), se copia en un issue, se guarda en un fichero, se loguea. Ninguna API encadenada hace eso.

## 17.3 Modelo mental

Diseñar un lenguaje es diseñar **el menú de un restaurante**:

- La **carta** son los *tokens*: los platos que existen (`MATCH`, `->`, `<>`, `"Ana"`, 42). Si no está en la carta, no hay forma de pedirlo.
- La **comanda** es la *gramática*: cómo se combinan los platos. En LiraQL la comanda tiene forma fija: primero MATCH, luego WHERE (opcional), al final RETURN. No se sirve el postre antes del entrante.
- El **camarero** es `validate()`: cuando pides algo que no está —una variable que nadie declaró—, no dice «error» a secas; dice *qué* no está y *dónde* lo estás señalando (`kind` + `span`).
- La **cocina** son los caps. 18-21: lexer, parser, plan, ejecución. Hoy no existe; solo diseñamos el restaurante sobre papel. Y la **comanda reescrita en limpio** es el `Display` canónico: la misma orden, en forma normalizada, lista para archivarse o compararse con otra.

Y el pipeline completo que estamos empezando a construir:

```text
             cap.18 (lexer)      cap.18 (parser)     cap.17 (validate)    caps.19-21
  "MATCH …" ─────────────────► tokens ────────────► AST ──────────────► plan ──► filas
   texto                     Token{kind,span}    Query + Span  ▲ TODO este capítulo vive aquí
```

## 17.4 Primera solución

Empecemos por lo que un novato (yo incluido) escribiría: una API builder encadenada, type-safe, cero parsing que implementar.

```rust
// Solución ingenua: la consulta como cadena de llamadas Rust.
let consulta = Consulta::nueva()
    .nodo("p", "Persona").flecha_saliente("CONOCE_A").nodo("f", "Persona")
    .donde(eq(prop("p", "nombre"), texto("Ana")))
    .devolver(prop("f", "nombre"));
```

Compila. El compilador te protege del orden de las cláusulas. No hay gramática, ni tokens, ni mensajes de error que diseñar. Durante una tarde, parece ganado.

## 17.5 Sus límites

Hasta que te sientas delante de la CLI del cap. 31 y descubras el muro:

1. **No es texto.** No se puede teclear en un terminal, ni pegar en un issue, ni guardar en un fichero de consultas, ni escribir en un log. Una consulta que solo existe como código Rust exige un compilador para existir.
2. **Congela el *cómo*.** La cadena de llamadas dicta el orden de recorrido: primero el nodo, luego la flecha, luego el filtro. El optimizador del cap. 21 no tiene nada que reordenar — cada consulta *es* su plan. Hemos regalado la promesa declarativa antes de empezar.
3. **Cada constructo nuevo es un método nuevo.** Añadir `LIMIT` o `ORDER BY` cambia la API pública y recompila a todos los usuarios; en un lenguaje, es una palabra más de la gramática.
4. **Exige ser programador Rust** para preguntar «¿a quién conoce Ana?».

La conclusión no es que la API sea mala: es que ocupa el otro lado del mostrador. Los motores reales tienen ambas (Neo4j tiene Cypher *y* drivers); pero el producto es el lenguaje. Necesitamos **texto → estructura**, y para eso hay que diseñar el contrato primero.

## 17.6 Solución evolucionada: LiraQL

LiraQL es deliberadamente un mini-Cypher: solo consulta, tres cláusulas, sin `CREATE`/`MERGE`/`DELETE` (eso es DML del cap. 31), sin `WITH`, sin `OPTIONAL MATCH`, sin recursión. La consulta emblema:

```text
MATCH (p:Persona)-[:CONOCE_A]->(f:Persona)
WHERE p.nombre = "Ana"
RETURN f.nombre, p.edad AS edad
```

### 17.6.1 La gramática primero

Antes de escribir un tipo Rust, escribimos la gramática. Está en el propio fichero, como documentación ejecutable en papel:

```text
query         ::= match_clause where_clause? return_clause ;
match_clause  ::= 'MATCH' path_pattern (',' path_pattern)* ;
path_pattern  ::= node_pattern ( rel_pattern node_pattern )* ;
node_pattern  ::= '(' [variable] [':' label] ['{' prop_map '}'] ')' ;
rel_pattern   ::= '-[' [variable] [':' rel_type] ']-' ( '>' | '<' )?
               |  '<-[' [variable] [':' rel_type] ']-' ;
prop_map      ::= ident ':' expression (',' ident ':' expression)* ;
where_clause  ::= 'WHERE' expression ;
return_clause ::= 'RETURN' return_item (',' return_item)* ;
return_item   ::= expression (['AS'] alias)? ;
expression    ::= or_expr ;
or_expr       ::= and_expr ('OR' and_expr)* ;
and_expr      ::= not_expr ('AND' not_expr)* ;
not_expr      ::= 'NOT' not_expr | comparison ;
comparison    ::= primary ( comp_op primary )? ;
comp_op       ::= '=' | '<>' | '<' | '<=' | '>' | '>=' ;
primary       ::= literal | property_access | '(' expression ')' ;
literal       ::= INTEGER | FLOAT | STRING | 'TRUE' | 'FALSE' | 'NULL' ;
```

¿Por qué la gramática ANTES del parser? Porque es el **contrato del que el parser se deriva**: en el cap. 18, cada regla se convertirá en una función (`parse_match_clause`, `parse_path_pattern`, `parse_node_pattern`…). Una regla, una función; sin tabla de precedencia, sin magia. Es el mismo principio del trait `Pager` (cap. 12): el contrato antes que el adapter.

Y fíjate en la parte más fina de la gramática, la escalera de `expression`:

```text
or_expr → and_expr → not_expr → comparison → primary
```

¿Por qué cinco niveles en vez de una regla plana `expression ::= expression OR expression | expression AND expression | …`? Porque esa regla plana es **ambigua**: `a OR b AND c` tendría dos árboles posibles y dos significados distintos. El escenario de fallo es real: una gramática ambigua devuelve árboles distintos según el día, y nadie puede razonar sobre su lenguaje. Los niveles de precedencia eliminan la ambigüedad *estructuralmente*: `OR` es más flojo que `AND`, que es más flojo que `NOT`, que es más flojo que la comparación. La gramática no dice «resuelve así»: *no puede* resolverse de otra forma. (Nótese también que `comparison` no encadena: `a < b < c` no es LiraQL. Cada recorte es deliberado.)

Para las relaciones, una advertencia de honestidad: la regla `rel_pattern` del comentario es una abreviatura descuidada (ese `( '>' | '<' )?` tras `']-'` sugeriría un `]-<` que no existe). La lectura operativa —la que el parser del cap. 18 implementa y su comentario documenta como «forma canónica»— son tres: `-[:TIPO]->` (saliente), `<-[:TIPO]-` (entrante) y `-[:TIPO]-` (sin dirección), más el `--` desnudo; el AST las captura como `RelDirection::Outgoing | Incoming | Undirected`. Conviene saber que las gramáticas en comentarios también tienen bugs: por eso el contrato se valida con tests.

### 17.6.2 El vocabulario: `TokenKind`

La carta del restaurante: 34 variantes en cuatro grupos — 10 palabras clave (`MATCH`, `WHERE`, `RETURN`, `AS`, `AND`, `OR`, `NOT`, `TRUE`, `FALSE`, `NULL`), 4 categorías léxicas (`Ident`, `Integer`, `Float`, `String`), 13 signos (`(` `)` `[` `]` `{` `}` `,` `:` `.` `->` `<-` `--` `-`) y 6 comparadores (`=` `<>` `<` `<=` `>` `>=`), más `Eof`.

```rust
pub enum TokenKind {
    Match, Where, Return, As, And, Or, Not, True, False, Null,   // keywords
    Ident(String), Integer(i64), Float(f64), String(String),     // léxicos
    LParen, RParen, LBracket, RBracket, LBrace, RBrace, Comma, Colon, Dot,
    ArrowRight, ArrowLeft, DashDash, Dash,                       // flechas y guiones
    Eq, NotEq, Lt, Lte, Gt, Gte,                                 // comparadores
    Eof,
}
```

Cada token viajará con su posición: `Token { kind, span }`.

¿Por qué definir los tokens aquí y no en el cap. 18 con el lexer? Porque el AST (este capítulo) necesita referenciar categorías de tokens sin depender del escáner: `Token` es parte del contrato, no de la implementación. Y aquí una confesión honesta, sacada de la historia real del workspace: el diseño original tenía 33 variantes y le faltaba el guión simple `-`. Nadie lo notó… hasta que el parser del cap. 18 intentó reconocer `-[` y `]-` y no pudo. El fix fue retroactivo y quirúrgico: una variante `Dash`, el lexer la produce, el parser la consume. La lección no es «el diseño fue malo»; es que **el diseño es lo bastante pequeño y está lo bastante aislado para que sus fallos sean locales y baratos**. Si el vocabulario hubiera nacido mezclado con el lexer, ese hueco habría estado esparcido por mil líneas de mecánica.

Un detalle de Rust que muerde aquí: `TokenKind` deriva `PartialEq` pero **no** `Eq`, y no es un descuido. Contiene `Float(f64)`, y `f64` no implementa `Eq` porque `NaN != NaN`: los lexemas flotantes no tienen igualdad total. El compilador lo impide — de hecho, durante el desarrollo real este capítulo derivó `Eq`, no compiló, y la lista de la §21 de `MIGRATION-PATTERN.md` lo registra como bug corregido. Deja que el compilador te diga qué puedes prometer.

### 17.6.3 El AST: la estructura significativa

El texto plano pierde información útil y contiene ruido (¿importan los espacios? ¿los paréntesis redundantes?). El AST es la estructura que **queda** cuando tiras el ruido y añades lo que importa:

```rust
pub enum AstNode {
    Match(MatchClause),
    Where(WhereClause),
    Return(ReturnClause),
}

pub struct Query {
    pub match_clause: MatchClause,          // MATCH (p:Persona)-[:CONOCE_A]->(f)
    pub where_clause: Option<WhereClause>,  // WHERE … — opcional
    pub return_clause: ReturnClause,        // RETURN … — siempre presente
    pub span: Span,
}

/// El patrón de camino: start + cadena de eslabones (rel, nodo).
pub struct PathPattern {
    pub start: NodePattern,                              // (p:Persona)
    pub chain: Vec<(RelationshipPattern, NodePattern)>,  // [(-[:CONOCE_A]->, (f))]
    pub span: Span,
}
```

El hito del brief pedía ese `AstNode` (con `Where(Expression)`; la versión final envuelve la expresión en un `WhereClause` con span propio — el contrato maduró un paso, como maduran los contratos). ¿Por qué un enum además del struct `Query`? Permite construir **sub-árboles en tests** y da al planner del cap. 19 su unidad de trabajo: operar cláusula a cláusula.

Y cada pieza es opcional por dentro — `(p:Persona {nombre: "Ana"})`, `()`, `(:Persona)`, `(p)` son todas válidas — porque `variable`, `label` y `properties` son `Option`/`Vec`. Las propiedades entre llaves son *inline predicates*: la gramática las admite en el patrón, y el cap. 19 las convertirá en filtros como cualquier condición de WHERE.

Las expresiones de WHERE y RETURN son un árbol recursivo con siete variantes (`Literal`, `Variable`, `PropertyAccess`, `Compare`, `And`, `Or`, `Not`). Fíjate en cuál NO está: no hay aritmética. `p.edad + 1` no es LiraQL. Cada ausencia es una decisión.

### 17.6.4 `Span` en TODO el AST

Aquí está la decisión que más calidad de usuario compra por menos código:

```rust
pub struct Span {
    pub start: u32,
    pub end: u32,
}
```

Un rango semiabierto `[start, end)` en bytes UTF-8 desde el inicio de la consulta. Cada nodo del AST lleva el suyo. ¿Para qué? Compara los dos mensajes:

```text
Error: unexpected token              ← ¿cuál? ¿dónde? el usuario rastrea a ojo
Error: variable 'x' usada pero no declarada en MATCH (en 47..53)
                                        ↑ apunta AL carácter exacto
```

Es la diferencia entre un lenguaje usable y uno frustrante, y es la convención de `rustc`, `miette` y `codespan-reporting`: el `kind` dice *qué* pasó, el `span` dice *dónde*. ¿Por qué en TODO el AST y no solo en tokens? Porque el lexer (cap. 18) produce spans gratis — ya sabe dónde está — y propagarlos con `Span::merge` cuesta poco; en cambio, retroalimentarlos después, cuando el árbol ya no recuerda de dónde vino, es imposible. La información de posición es de las cosas que solo se pueden conservar en el momento.

`Span` trae su aritmética mínima: `at(offset)` para spans vacíos (tokens sintéticos), `new` que normaliza el orden, `merge` que devuelve la unión, `is_empty`, `len`. Veinte líneas que sostienen todos los mensajes del lenguaje.

### 17.6.5 `Literal` envuelve el `Value` del cap. 7

Momento de spacing: cierra los ojos y recuerda el cap. 7. ¿Cuáles eran las seis variantes de `Value`? (`Null`, `Bool`, `Int`, `Float`, `String`, `Bytes` — compruébalo después, no ahora.) La decisión de este capítulo:

```rust
pub enum Expression {
    Literal { value: Value, span: Span },   // ← Value del cap. 7, no un enum nuevo
    ...
}
```

`Expression::Literal` **envuelve** el `Value` del capítulo 7 en vez de duplicar un enum `Literal` con `Int/Float/String/…`. ¿Por qué? Porque el modelo de datos ya existe: lo que el lenguaje compara es exactamente lo que el grafo guarda. Duplicar tipos crearía dos universos con conversiones en cada frontera (AST → executor, cap. 20), y los bugs de conversión silenciosa entre Int/Float/String son de los más difíciles de cazar. Un solo modelo, una sola verdad.

¿Y qué pasa con `Bytes`? No tiene literal: `TokenKind` no tiene token para bytes y la gramática no lo contempla (¿cómo escribirías bytes crudos en un lenguaje de texto?). Aun así, `Display` sabe imprimirlo — `0x` + hexadecimal vía un `hex_bytes` propio de 12 líneas, sin crates — porque una consulta canonificada puede venir de datos, no solo de texto. Un detalle cosmético con una lección dentro: el diseño distingue *lo que el usuario puede escribir* de *lo que el sistema puede contener*.

### 17.6.6 `validate()` devuelve `Vec<QueryError>`, no `Result`

Hasta ahora, todos los errores del Vol.II eran `Result`: el pager, el buffer pool, los índices. Este capítulo rompe el patrón, y es deliberado:

```rust
pub struct QueryError {
    pub kind: QueryErrorKind,   // QUÉ pasó
    pub span: Span,             // DÓNDE
}

impl Query {
    pub fn validate(&self) -> Vec<QueryError> { ... }
}
```

Un humano escribe una consulta con **tres** variables mal escritas. Con `Result`, corrige una, vuelve a validar, corrige la siguiente, vuelve a validar… ciclos de fix-recompila. Con `Vec`, ve los tres errores de golpe. Para una operación de máquina (leer una página, insertar en un índice), abortar en el primer fallo es correcto — no hay humano iterando. Para un lenguaje, reportar todo es la UX. Mismo Rust, capas distintas, decisiones distintas. (El parser del cap. 18 sí elegirá `Result`, y hablaremos de por qué.)

Las seis reglas que `validate()` comprueba, cada una con su error tipado:

| Regla | `QueryErrorKind` |
|---|---|
| MATCH con al menos un patrón | `EmptyMatch` |
| Ningún nodo `()` vacío (sin variable, label ni props) | `EmptyNodePattern` |
| Ninguna variable duplicada (nodos, aristas, y nodo↔arista) | `DuplicateVariable` |
| Toda variable de WHERE/RETURN declarada en MATCH | `UnknownVariable` |
| RETURN con al menos un item | `EmptyReturn` |
| Ningún alias vacío | `EmptyAlias` |

Nota el alcance: las variables de **arista** también ligan (`-[r:CONOCE_A]->` declara `r` usable en WHERE), y un nombre no puede ser a la vez nodo y arista — si `p` fuese ambos, el executor del cap. 20 tendría dos tipos para un nombre. Y nota también lo que `validate()` **no** comprueba: `1 < "x"` le parece válido. Los tipos de las expresiones son cosa del binder del cap. 19; aquí solo se valida el *alcance*, porque es lo único que se puede saber sin mirar los datos.

### 17.6.7 El `Display` canónico

La última pieza del contrato: el AST sabe reescribirse como texto **canónico**:

```text
MATCH (p:Persona)-[:CONOCE_A]->(f:Persona) WHERE (p.nombre = "Ana") RETURN f.nombre, p.edad AS edad
```

¿Observas los paréntesis alrededor de `p.nombre = "Ana"`? No estaban en el original. El `Display` no reproduce el texto fuente — normaliza: paréntesis explícitos por nodo de expresión, comas con un espacio, cláusulas en orden fijo, comillas dobles. ¿Para qué sirve pagar esto?

1. **Tests**: el cap. 18 comparará «lo que esperaba parsear» con «lo que parseó» vía su forma canónica (comparar `Debug` de árboles con spans es frágil; la forma canónica es estable).
2. **`liradb explain`** (cap. 21): mostrar la consulta normalizada junto al plan es la semilla del explain.
3. **Round-trip**: `display(parse(display(parse(x)))) == display(parse(x))` — la forma canónica es idempotente. Es la misma promesa del encode/decode de la slotted page (cap. 11): que la representación dual sea fiel, que ida y vuelta no pierdan nada.

## 17.7 Prueba de fuego

La prueba de fuego de un capítulo de diseño es: **¿se puede probar el diseño sin la implementación?** Los 41 tests del módulo `tests_query` responden que sí — construyen los AST **a mano**, con spans fingidos, porque no hay parser:

```rust
// De tests_query (cap18_lexer_parser.rs:2007): AST sin parser.
// Los spans son sintéticos — posiciones fingidas, no de un fuente real.
fn minimal_query() -> Query {
    let node = person_node("p", "Person", s(7, 18));   // (p:Person)
    let path = PathPattern { start: node, chain: Vec::new(), span: s(6, 19) };
    Query {
        match_clause: MatchClause { patterns: vec![path], span: s(0, 19) },
        where_clause: None,
        return_clause: ReturnClause { items: vec![ReturnItem {
            expr: Expression::prop("p", "name", s(27, 33)), alias: None, span: s(27, 33) }],
            span: s(20, 33) },
        span: s(0, 33),
    }
}
```

Batería de tests que cubren las cuatro piezas del contrato: spans (`span_new_normaliza_orden`, `span_merge_cubre_a_ambos`), vocabulario (`token_kind_cubre_todos_los_grupos`), AST (`path_pattern_edge_variables_incluye_rel_var`, `expression_and_or_not_recolecta_recursivo`), validación (`validate_variable_duplicada_entre_nodo_y_arista`, `validate_variable_desconocida_en_where`, `validate_acepta_variable_de_arista_en_where`, `validate_node_pattern_vacio_devuelve_empty_node_pattern`), errores (`query_error_display_incluye_span`, `query_error_implementa_std_error`) y Display (`display_query_completa_round_trip_canonico`, `display_relationship_pattern_direcciones`, `display_value_bytes_canonico`).

¿Y si te saltas este capítulo? El síntoma aparece en el 18: sin contrato, rediseñarás tokens y AST *mientras* escribes el parser, y cada descubrimiento se propagará como refactor en vez de como una línea nueva en la gramática.

## 17.8 Qué hemos sacrificado

Un lenguaje pequeño es una lista larga de «no»:

1. **No hay DML**: nada de crear, borrar ni modificar (cap. 31).
2. **No hay `WITH`, `OPTIONAL MATCH`, ni recursión** (`*1..3` de Cypher): exigen pipeline de partes y semántica de opcionalidad que aún no tenemos.
3. **`()` desnudo se rechaza** (`EmptyNodePattern`) aunque Cypher lo permita: un patrón que no liga ni filtra nada es un hoyo pedagógico. (El binder del cap. 19 lo aceptará con variables internas — divergencia documentada.)
4. **Sin aritmética, sin funciones, sin `IN`, sin `LIMIT`, sin `ORDER BY`** — `LIMIT` será tu ejercicio experto.
5. **Comparaciones no encadenables** (`a < b < c` fuera), y el parser abortará en el primer error sintáctico (cap. 18) aunque `validate()` reporte todos los semánticos: recovery multi-error es un proyecto entero.

Cada recorte tiene el mismo formato: *no lo necesitamos para aprender lo que sigue, y sin él el diseño cabe en la cabeza*.

## 17.9 Cómo lo hace una BBDD real

- **openCypher** (2016): Neo4j liberó la especificación de Cypher para que RedisGraph, SAP HANA, Memgraph y otros la implementaran. Es la deuda directa de LiraQL: nuestras tres cláusulas y nuestros paréntesis-flechas vienen de ahí. La especificación formal (tcs + BNF) hace por openCypher lo que nuestra EBNF hace por nosotros: contrato antes que implementación.
- **GQL, ISO/IEC 39075:2024** (publicado el 17 de abril de 2024): el primer lenguaje de consulta de bases de datos estandarizado por ISO desde SQL (1987). Desarrollado por el mismo comité que mantiene SQL, con Cypher como INPUT principal — la sintaxis de dibujar patrones que nació en 2011 acabó en un estándar internacional.
- **SPARQL** (W3C, 2008/2013): el veterano de los lenguajes de grafos, pero para RDF — triples sujeto-predicado-objeto, no property graph. Su `?x :conoce ?y` con variables prefijadas es la otra gran tradición: básica en web semántica, ajena a nuestro modelo. La comparación enseña que el lenguaje sigue al modelo de datos: no puedes diseñar el primero sin decidir el segundo.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: escribe la EBNF de `match` con DOS patrones separados por coma y explica qué operador lógico implementan (pista: producto).
- *Intermedio*: en Cypher real, `WHERE` acepta `exists()`, `size()`, matching de prefijos (`startsWith`). ¿Por qué añadir funciones obliga a tocar gramática, AST, validación y Display A LA VEZ? Enumera los cuatro puntos de cambio para una función como `startsWith`.
- *Experto*: lee la gramática EBNF de openCypher (la tcs) y localiza una regla que nuestra gramática no tenga (p.ej. `variableLength` o `shortestPath`); escribe su EBNF al estilo del capítulo y explica qué nodo AST nuevo exigiría y por qué nuestro `PathPattern` actual no puede contenerlo.

## 17.10 Lo que te llevas

- **Declarativo**: el usuario dice QUÉ; el CÓMO es del optimizador (cap. 21). Es el argumento de Codd (1970) y la razón de ser de SEQUEL/SQL.
- **La gramática es el contrato**: EBNF primero, parser derivado — una función por regla (cap. 18). La precedencia por niveles elimina la ambigüedad estructuralmente, no por decreto.
- **`Span` en todo el AST**: errores que apuntan al carácter exacto, estilo rustc. Barato si se conserva desde el principio; imposible de recuperar después.
- **Un solo modelo de datos**: `Literal` envuelve el `Value` del cap. 7.
- **`Vec<QueryError>` para humanos, `Result` para máquinas**: reportar todo vs abortar — la UX decide.
- **`Display` canónico**: tests, semilla de explain, round-trip idempotente.

## 17.11 Ojo, cuidado con…

- **Confundir diseño con implementación**: este capítulo no tokeniza ni parsea nada. Si te pica escribir el lexer, es el cap. 18 llamándote.
- **Confundir error sintáctico con semántico**: el primero es del parser (cap. 18, aborta); el segundo de `validate()` (reporta todos).
- **Derivar `Eq` en enums con `f64`**: `TokenKind` no puede — `Float(f64)` → NaN ≠ NaN. Bug real, registrado.
- **Esperar que `Display` reproduzca el original**: es forma canónica; añade paréntesis, normaliza espacios. El round-trip es idempotencia, no igualdad de texto.

## 17.12 Pin de batalla

> *«Un error que no dice dónde está no es un error: es un acertijo. Y los usuarios de los acertijos se cambian de base de datos.»*

## 17.13 Si solo lees 30 segundos

LiraQL es un mini-Cypher declarativo: `MATCH (patrón)-[:FLECHA]->(patrón) WHERE expr RETURN expr`. Su diseño son cuatro piezas fijadas HOY, antes del código: la **gramática EBNF** (el contrato del que el parser del cap. 18 se deriva, una función por regla), el **vocabulario de tokens** (34 variantes), el **AST con `Span` en cada nodo** (errores que apuntan al carácter exacto) y la **validación semántica + Display canónico** (todos los errores de golpe; forma normalizada para tests y explain). Nada de esto ejecuta nada — y por eso todo lo demás podrá construirse encima sin sorpresas.

## 17.14 Una historia pequeña

La primera versión del módulo compiló a la primera… menos un `#[derive(Eq)]` en `TokenKind` que el compilador rechazó por el `Float(f64)` de dentro. Cinco minutos. Después llegó el cap. 18, y con él la cuenta de resultados del diseño: faltaba el token `-` (el guión simple de `-[` y `]-`) y faltaba la variante `Expression::Variable` para que `RETURN p` —retornar el nodo entero, no una propiedad— existiera. Dos huecos en un contrato de 970 líneas, dos fixes de diez líneas, y ni un solo tipo rediseñado a mitad de implementación. Cuando Ana vio el mensaje `variable 'x' usada pero no declarada en MATCH (en 47..53)` señaló la pantalla con el dedo, clavada en el 47, y dijo: «ah, ese era». Ese gesto — el dedo en el carácter exacto — es todo lo que este capítulo quería comprar.

## Ejercicios resueltos

**1. ¿Por qué `validate()` devuelve `Vec<QueryError>` si el `insert` de la slotted page (cap. 11) devuelve `Option` y el pager (cap. 12) `Result`?**

Porque el consumidor del error es distinto. El pager y la página fallan para una **máquina** (otra capa del motor) que va a abortar o reintentar: un solo error basta y sobra, y `Result`/`Option` fuerzan tratarlo. `validate()` falla para un **humano** que está escribiendo una consulta con el dedo en el teclado: si tiene tres variables mal, quiere ver las tres, no tres ciclos de corrige-y-vuelve-a-probar. Es la misma razón por la que `rustc` reporta varios errores por compilación. La regla que queda: *cuántos errores devuelves depende de quién los lee*.

**2. ¿Qué imprime exactamente el `Display` canónico de la consulta emblema del capítulo?**

`MATCH (p:Persona)-[:CONOCE_A]->(f:Persona) WHERE (p.nombre = "Ana") RETURN f.nombre, p.edad AS edad`. Tres canonicalizaciones visibles: la comparación de WHERE gana paréntesis (`Expression::Compare` imprime `({left} {op} {right})`), las comillas dobles para strings, y el alias con ` AS ` aunque el fuente usara solo espacio (`p.edad edad` es gramática válida; la forma canónica siempre escribe `AS`). Verificable con `display_query_con_where_y_alias` y `display_query_completa_round_trip_canonico`.

## Ejercicios propuestos

**Esencial (retrieval, cap. 7).** Sin mirar nada —ni el cap. 7 ni este capítulo—, lista de memoria las seis variantes de `Value`. Luego responde: ¿cuáles tienen literal en LiraQL y cuál no tiene NI token que la represente? ¿Cómo puede aun así aparecer esa variante en el `Display`? Verifica ejecutando `display_value_bytes_canonico` y `hex_bytes_formatea_correctamente`.

**Intermedio (predicción).** Construye a mano (helpers al estilo de `tests_query`) el AST de `MATCH (p:Persona)-[r:CONOCE_A]->(p:Persona), () RETURN x.nombre` y predice, ANTES de ejecutar: cuántos errores devuelve `validate()`, en qué orden, con qué `QueryErrorKind` y con qué span cada uno. ¿Declara `r` algo que interfiera? Verifica con `validate_variable_duplicada_entre_nodo_y_arista` y `validate_node_pattern_vacio_devuelve_empty_node_pattern` como patrón.

**Experto (diseño puro).** Diseña `LIMIT n` para LiraQL sin escribir una línea de lexer ni parser: (a) regla EBNF — ¿extiende `return_clause` o es cláusula hermana?, y argumenta por qué tu elección no introduce ambigüedad; (b) campo nuevo en el AST con su `Span` y su tipo (¿por qué `i64` y no `Value`?); (c) regla de `validate()`: qué pasa con `LIMIT 0` y `LIMIT -1`, con qué `QueryErrorKind` nuevo y qué span; (d) extensión del `Display` canónico; (e) dos tests con AST construido a mano. El cap. 18 hará el resto: una función nueva por regla nueva.

## Para profundizar

- **Chamberlin & Boyce, «SEQUEL: A Structured English Query Language» (SIGMOD 1974)** — el paper fundacional; el origen de todo lo declarativo en datos.
- **Codd, «A Relational Model of Data for Large Shared Data Banks» (CACM, 1970)** — el argumento de la navegación automática vs la programada.
- **Francis et al., «Cypher: An Evolving Query Language for Property Graphs» (SIGMOD 2018)** — la referencia de Cypher/openCypher, con la semántica formal de los patrones.
- **openCypher (opencypher.org)** — la especificación con gramática BNF completa: compara su tamaño con la nuestra y mide lo que significa «lenguaje pequeño».
- **GQL, ISO/IEC 39075:2024 (gqlstandards.org)** — el estándar de 2024; lee al menos su índice para ver el territorio completo de un lenguaje de grafos industrial.
- **Nystrom, «Crafting Interpreters»** y **Wirth, «Compiler Construction»** — los mejores acompañamientos para los caps. 17-18; de Wirth viene la idea «una función por regla» que el cap. 18 ejecutará.
- **rustc Dev Guide (capítulo de diagnósticos) y `codespan-reporting`** — cómo se diseñan errores con span en compilers reales.

## Mini-diálogo: la cena de diseño

> — Entonces no hemos construido nada. Nombres, reglas en un comentario, structs vacíos de comportamiento. ¿Esto es un capítulo o una reunión?
>
> — Es la reunión que te ahorra la obra. Cada regla EBNF que escribiste hoy es una función del parser que mañana escribirás sin decidir nada; cada `Span` que exigiste es un error que apuntará al carácter exacto; cada `QueryErrorKind` es un mensaje que Ana leerá a las tantas de la noche.
>
> — Pero el lexer podría haber salido antes, con el diseño «sobre la marcha».
>
> — Y entonces `Dash` no habría faltado en un contrato de 34 líneas: habría faltado repartido por mil líneas de escáner. El diseño no elimina los errores — el nuestro tuvo dos. Los concentra en un sitio donde son baratos de encontrar. Eso es diseñar: no adivinar el futuro, sino decidir dónde van a vivir los fallos. Y el camarero ya existe: `validate()` solo dice dos cosas, pero las dice bien — qué plato no está en la carta, y en qué línea del menú lo señalas. La cocina abre en el capítulo 18.

---

*(Próximo capítulo: 18 — Construir el lexer y el parser. El contrato ya está firmado: cada regla de la gramática se convierte en una función, cada token de la carta en un byte reconocido por maximal-munch, y el texto de Ana se convierte, por fin, en un `Query`.)*
# Capítulo 18 — Construir el lexer y el parser

> *«El lexer ve bytes. El parser ve tokens. El que mezcla las dos cosas paga sus errores en el sitio equivocado.»*

## 18.0 La anécdota de la esquina

En 1986, Alfred Aho, Ravi Sethi y Jeffrey Ullman publicaron *Compilers: Principles, Techniques, and Tools*. La portada muestra un caballero luchando contra un dragón rojo, y ese detalle le dio el nombre con el que todo el mundo conoce al libro: **el libro del dragón**. La imagen no era decorativa: el dragón representa la complejidad del diseño de compiladores, y el caballero la combate blandiendo una lanza etiquetada en la propia portada como *«LALR parser generator»*. (Existen tres dragones: el verde de 1977 —*Principles of Compiler Design*, de Aho y Ullman—, este rojo de 1986 y el púrpura de la segunda edición de 2006, ya con Monica Lam.)

Lo que el libro enseñó a generaciones fue algo más humilde que LALR: **separar el escaneo del parsing**. Antes de pensar en árboles de derivación, un compilador tiene que resolver un problema estúpidamente difícil de hacer bien: cortar un flujo continuo de caracteres en piezas con sentido —tokens— y recordar dónde empieza y dónde acaba cada una. Los tokens liberan al parser de ocuparse de espacios, saltos de línea y del contenido de los strings. Esa separación es la columna vertebral de este capítulo.

Hay una ironía deliciosa: la lanza del caballero es un *generador* de parsers LALR, y nosotros no vamos a usar ninguna. Niklaus Wirth —el padre de Pascal, que compiló sus lenguajes con descendente recursivo en los años 70 y defendió esa técnica durante toda su vida en *Compiler Construction*— demostró que para un lenguaje pequeño y bien diseñado, la mejor herramienta no es una tabla generada: es **una función por regla de la gramática**. Hasta `rustc`, uno de los compiladores más serios del mundo, usa un parser descendente escrito a mano. Hoy construiremos la nuestra.

## 18.1 Objetivo

En el cap. 17 diseñamos LiraQL sobre el papel: tokens, gramática EBNF, AST, errores con posición. Pero las `Query` se construían a mano en los tests. Este capítulo baja un escalón y construye **el código que convierte texto en ese AST**:

1. `Lexer` — el escáner: un cursor sobre bytes que produce `Vec<Token>` con spans exactos.
2. `Parser` — el descendente recursivo: una función por regla de la EBNF, que produce `Query`.
3. `LexError` y `ParseError` — las dos culpas, tipadas y con posición.

El hito que abre la Parte IV por dentro: `parse("MATCH (p:Person) RETURN p")` debe devolver una consulta válida.

## 18.2 Problema

Tienes el fuente `"MATCH (p:Person) RETURN p"` — 25 caracteres en un `&str`— y un `Query` con `Expression::Variable` en el RETURN. Entre ambos hay un abismo: el string no tiene estructura, el AST es todo estructura.

El problema se parte en dos, y esa partición es el 80 % de las decisiones del capítulo:

- **¿Cómo corto el texto en piezas?** El texto no trae espacios fiables: `(p:Person)-[:KNOWS]->(f)` es una sola palabra para `split_whitespace`, y `<>` son dos caracteres que significan UN token.
- **¿Cómo compruebo que las piezas forman una frase válida?** Y cuando no lo son, ¿quién lo dice y señalando a qué byte?

## 18.3 Modelo mental

Piensa en una **oficina de registro con dos funcionarios**:

```
 "MATCH (p:Person) RETURN p"          EBNF del cap. 17
        │                                   │
        ▼                                   │
┌─────────────────┐                         │
│  LEXER (oficial)│  corta el rollo de bytes en palabras
│                 │  y sella cada una: [nace, muere]
└─────────────────┘                         │
        │  Vec<Token>                       │
        ▼                                   ▼
┌─────────────────┐    ┌──────────────────────────┐
│ PARSER (gramático)   │  "MATCH ( )" luego "p",
│                 │    │  luego RETURN... ¿encaja? │
└─────────────────┘    └──────────────────────────┘
        │
        ▼
      Query
```

El **lexer** es el oficial que recibe el rollo continuo de papel y lo corta en palabras sueltas, estampando en cada una su certificado de origen: el `Span` con el byte donde nace y el byte donde muere. No opina sobre si la frase tiene sentido; su trabajo es cortar bien y certificar.

El **parser** es el gramático: recibe la bandeja de palabras numeradas y, con la EBNF del cap. 17 bajo el brazo, comprueba que forman frase. Nunca toca un byte crudo.

Y aquí está la lección oscura del modelo: **si el oficial corta mal, el gramático culpa a un inocente**. Un certificado falso no rompe la oficina de registro: rompe la inspección, en otro mostrador. Guarda esa idea para el §18.8.

## 18.4 Primera solución

Lo más simple que parece funcionar: métodos de `str` y buen ojo.

```rust
// Solución ingenua: trocear por espacios y mirar prefijos.
for palabra in src.split_whitespace() {
    if palabra.starts_with('(') { /* empieza nodo... */ }
    if palabra.contains("->")   { /* flecha... */ }
}
```

Con `"MATCH (p) RETURN p.name` — espacios perfectos, sin adornos — hasta avanza. Los tests del happy path pasan. Y durante un rato nadie se queja.

## 18.5 Sus límites

Hasta que llegan consultas reales:

1. **`(p:Person)-[:KNOWS]->(f:Person)`** no tiene espacios entre piezas: `split_whitespace` lo devuelve entero. Necesitarías `starts_with` en cascada… re-inventando el lexer, pero mal.
2. **`<>`, `<=`, `<-`, `->`, `--`** comparten prefijos: `contains("<")` no distingue menor-que de distinto-de.
3. **`"Ana García"`** — un string con un espacio — se parte en dos basuras.
4. **Cero posiciones.** Cuando algo falla, lo único que puedes decir es «consulta inválida». Ni byte, ni línea. Compáralo con rustc señalando el carácter exacto.
5. **UTF-8.** `p.name = "cañón"` descuadra cualquier aritmética pensada en caracteres: `ñ` ocupa 2 bytes.

## 18.6 Solución evolucionada

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap18_lexer_parser.rs`. Leámoslo por partes, porque cada decisión tiene un porqué.

### El lexer: un cursor y una regla de oro

```rust
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: u32,
}
```

Dos campos. El fuente como **bytes** (`&[u8]`, no `chars()`): el `Span` del cap. 17 mide bytes, y el cursor debe avanzar en la misma unidad que el span certifica — si no, los offsets mienten. Es la misma disciplina del cap. 9 con el little-endian: **declara la unidad, no la dejes implícita**. Y `pos: u32` porque ningún fuente didáctico supera 4 GiB.

Sobre ese cursor, el bucle principal es el escaneo canónico: saltar espacios, mirar el primer byte, y según ese byte consumir el resto del token:

```rust
pub fn lex(mut self) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    while !self.is_at_end() {
        self.skip_whitespace();
        if self.is_at_end() { break; }
        tokens.push(self.scan_token()?);
    }
    tokens.push(Token::new(TokenKind::Eof, Span::at(self.pos)));
    Ok(tokens)
}
```

Fíjate en el último `push`: **siempre hay un token `Eof` al final**. Gracias a eso, `peek()` en el parser devuelve `&Token`, jamás `Option<Token>`, y «me quedé sin tokens» (`UnexpectedEof`) es distinguible de «encontré lo que no tocaba» (`UnexpectedToken`).

El corazón es `scan_token`, que despacha por el primer byte. Y aquí vive la regla de oro — **maximal-munch**: gana el token más largo posible. Antes de decidir que un byte es un token, prueba si con su vecino forma uno más largo:

```rust
b'-' => {
    if self.match_byte(b'>')      { TokenKind::ArrowRight }
    else if self.match_byte(b'-') { TokenKind::DashDash }
    else                           { TokenKind::Dash }
}
b'<' => {
    if self.match_byte(b'-')      { TokenKind::ArrowLeft }
    else if self.match_byte(b'=') { TokenKind::Lte }
    else if self.match_byte(b'>') { TokenKind::NotEq }
    else                           { TokenKind::Lt }
}
```

¿Por qué maximal-munch y no partir siempre en tokens de un carácter y que el parser pegue? Porque `-` y `>` sueltos en `(p)-[:X]->(f)` obligarían a CADA regla del parser a mirar pares de tokens, duplicando la lógica. Y si el lexer no prueba las combinaciones de dos bytes, `a <> b` llega como `Lt Gt` — exactamente el bug del §18.8. El matching más largo elimina la ambigüedad de raíz. Cada token se sella al salir: `Span::new(start, self.pos)` — nació en `start`, murió donde el cursor quedó. El span se **deriva** del propio escaneo, no se calcula después: deriva, no lleves en la cabeza.

### Palabras clave, números y strings

Los identificadores (`scan_identifier`) consumen `[A-Za-z_][A-Za-z0-9_]*` y se clasifican con un `match` exacto sobre el texto: `MATCH` es `TokenKind::Match`, pero `match` es `Ident("match")` — las palabras clave son **case-sensitive**, por convención de Cypher. Sin estado, sin normalización: el texto tal cual contra la lista.

`scan_number` acumula dígitos con `checked_mul`/`checked_add`: si el literal desborda `i64`, **sigue consumiendo dígitos** (para que el span del error cubra todo el literal) y luego devuelve `IntegerOverflow`. Detalle fino: `12.` sin dígitos tras el punto no es un float roto — es `Integer(12)` seguido de `Dot` (el `peek_next` exige un dígito tras el punto para formar flotante).

`scan_string` es el territorio ciego, y su inmunidad es una decisión de diseño: **dentro de las comillas, el lexer no conoce la sintaxis**. Consume bytes crudos hasta la comilla de cierre, entendiendo solo dos cosas: `\` inicia un escape (`\n \t \r \\ \" \0`; cualquier otra cosa es `InvalidEscape` con el byte culpable en el error) y `"` cierra. ¿Por qué esta ceguera es obligatoria? Porque `WHERE p.name = "Ana-García \"la grande\""` contiene `-`, `{` y comillas escapadas que NO son sintaxis. Si el interior se escaneara con las reglas generales, cualquier descripción con un guión rompería el WHERE. Y un string sin cerrar antes del final del fuente es `UnterminatedString` con el span desde la comilla inicial hasta el EOF.

### El parser: el código ES la gramática

El `Parser` es igual de austero que el `Lexer`: `tokens: Vec<Token>` y `current: usize`. Con cuatro helpers de cursor (`peek`, `check`, `match_kind`, `advance`/`expect`) construimos la correspondencia que organiza todo el módulo — **una función por regla EBNF del cap. 17**:

| Regla EBNF (cap. 17) | Función gemela |
|---|---|
| `query ::= match_clause where_clause? return_clause` | `Parser::parse` |
| `match_clause ::= 'MATCH' path_pattern (',' path_pattern)*` | `parse_match_clause` |
| `path_pattern ::= node_pattern (rel_pattern node_pattern)*` | `parse_path_pattern` |
| `node_pattern ::= '(' [variable] [':' label] ['{' prop_map '}'] ')'` | `parse_node_pattern` |
| `rel_pattern` (tres direcciones) | `parse_relationship_pattern` |
| `where_clause ::= 'WHERE' expression` | `parse_where_clause` |
| `return_clause ::= 'RETURN' return_item (',' return_item)*` | `parse_return_clause` |
| `return_item ::= expression (['AS'] alias)?` | `parse_return_item` |

Se llama **descendente recursivo predictivo**: desciende por la gramática llamando a funciones que se llaman entre sí, y es *predictivo* porque decide qué alternativa tomar mirando UN token de preanálisis (`peek`). ¿Cómo sabe `parse_path_pattern` que el camino sigue? `starts_relation`: si el token actual es `Dash`, `ArrowLeft` o `DashDash`, hay otra relación encadenada.

¿Por qué esta técnica y no las alternativas? Una **tabla LL(1)/LALR** (la lanza del dragón del libro) es compacta y detecta ambigüedades de la gramática al generarla, pero el resultado es una tabla que se depura con autopsia: cuando falla, no hay función donde poner un punto de ruptura. Un **parser Pratt** es magnífico cuando hay decenas de niveles de precedencia con expresiones densas, pero esconde la gramática en una tabla de potencias de enlace. Aquí, con diez reglas, la alternativa ganadora es la legibilidad: cuando `parse` falla, la pila de llamadas ES la derivación gramatical que estaba intentando. Wirth llevaba razón medio siglo: para un lenguaje pequeño, esto es lo que se enseña y lo que se mantiene.

### La precedencia es la pila de llamadas

Las expresiones tienen operadores con distinta fuerza: `AND` ata más que `OR`, `NOT` más que `AND`, la comparación más que todo. Nuestra solución no es una tabla: es **una cadena de funciones**, donde cada nivel consume SU operador y delega hacia abajo el más fuerte:

```rust
fn parse_or(&mut self) -> Result<Expression, ParseError> {
    let mut left = self.parse_and()?;
    while self.match_kind(&TokenKind::Or).is_some() {
        let right = self.parse_and()?;
        let span = Span::new(left.span().start, right.span().end);
        left = Expression::Or { left: Box::new(left), right: Box::new(right), span };
    }
    Ok(left)
}
```

```
parse_expression ─► parse_or ─► parse_and ─► parse_not ─► parse_comparison ─► parse_primary
   (la más floja, OR)                                              (la más fuerte: literal, p.prop, ( ))
```

El truco: `parse_or` se llama a través de `parse_and`, que se llama a través de `parse_not`… Así, cuando `parse_or` busca sus `OR`, todo lo que cuelgue debajo ya se ha agrupado con precedencia mayor. La prueba:

```text
WHERE p.x = 1 OR p.y = 2 AND p.z = 3
       └─ Compare ─┘    └──── And(Compare, Compare) ────┘
              └────────── Or(Compare, And) ──────────────┘
```

`a OR b AND c` sale como `a OR (b AND c)` — exactamente lo que verifica `parse_precedencia_or_es_menor_que_and`. Y los paréntesis del fuente (`parse_primary` regla `LParen`) rompen el orden cuando el usuario lo pide. Si quisieras cambiar la precedencia de LiraQL, moverías UNA función de sitio en la cadena: el orden de las funciones ES la precedencia, a la vista. Una tabla de precedencia habría desacoplado la especificación del código; con cuatro niveles, ese desacoplamieno solo añade indirección.

### Dos fases, dos culpas: `LexError` y `ParseError`

Los errores heredan la disciplina de los caps. 12-16 — tipados, con `Display` legible — y añaden la posición:

- **`LexError`** (5 variantes): `UnexpectedChar { byte }`, `UnterminatedString`, `InvalidEscape { byte }`, `IntegerOverflow`, `MalformedNumber`. Cada una describe qué rompió el ESCANEO.
- **`ParseError`** (la envoltura `Lex(LexError)` + 7 variantes sintácticas): `UnexpectedToken { expected, found }`, `UnexpectedEof`, `MissingMatch`, `MissingReturn`, `PathMustStartWithNode`, `MalformedRelationship`, `TrailingTokens { found }`. Cada una describe qué rompió la ESTRUCTURA.

¿Por qué dos errores y no uno? Porque son **dos fases con dos culpas**: cuando `parse` falla, el mensaje debe decir si el usuario escribió un carácter imposible (léxico) o una frase mal ordenada (sintáctico). Y se unen con el patrón idiomático de Rust:

```rust
impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        let span = e.span;
        ParseError::new(ParseErrorKind::Lex(e), span)
    }
}
```

Gracias a `From`, el `lex(src)?` dentro de `Parser::new` propaga el error léxico sin una línea extra, y `source()` conserva la cadena causal: el `ParseError` sabe que su causa raíz es un `LexError`. El `Display` de ambos remata con el sufijo de localización — ` (en 0..1)`, o `(en offset 7)` si el span es vacío — al estilo de rustc y miette.

Y la recuperación es minimalista a propósito: **primer error, mensaje claro, abort**. ¿No sería mejor reportarlos todos, como hace `validate()` en el cap. 17? No aquí: detectar el segundo error exige sincronizar (avanzar hasta un `)` o una `,` «punto de reinscripción»), y aun así los errores derivados contaminan. Un `MissingReturn` bien localizado enseña más que una cascada de tres mensajes por un solo olvido. La recuperación completa queda declarada como ejercicio y deuda.

## 18.7 Prueba de fuego

El hito del brief, tal cual, con su test real (`parse_hito_del_brief`):

```rust
let q = parse("MATCH (p:Person) RETURN p").unwrap();
assert!(q.is_valid());
assert!(matches!(q.return_clause.items[0].expr, Expression::Variable { .. }));
```

Ese `Expression::Variable` tiene historia (§18.8). La batería completa — 73 tests en `tests_lexer_parser` — cubre lo que este capítulo promete, y los nombres son un mapa del territorio: `lex_comparadores` y `lex_flechas_y_guiones` (maximal-munch), `lex_span_de_token_cubre_exactamente_su_texto` (certificados: en `"MATCH (p)"`, `MATCH` es `0..5`, `(` es `6..7`, `p` es `7..8`, `)` es `8..9` — el espacio no cuenta), `lex_whitespace_no_cuenta_en_spans`, `lex_span_es_aware_a_utf8_en_bytes` (`"cañón"`: 7 caracteres, span `0..9` en bytes), `lex_string_con_escapes`, `lex_entero_desborda_i64_es_error`, `parse_where_todos_los_comparadores` (los seis operadores), las tres de precedencia, `round_trip_consulta_minima` (parsear el `Display` del AST reproduce el AST) y `parser_from_tokens_funciona` (el parser acepta tokens sintéticos sin pasar por el lexer — la prueba de que las dos fases están de verdad separadas).

Y el camino de error es parte de la prueba de fuego. La consulta ajena:

```rust
let err = parse("SELECT * FROM nada").unwrap_err();
// MissingMatch, span 0..6:
// "toda consulta LiraQL debe empezar con MATCH (en 0..6)"
```

No «consulta inválida»: la cláusula culpable, con su byte. Y la cadena rota: imagina un lexer que pierde un byte de `<>` — el token siguiente llega mal certificado y el error explota en el parser, señalando a un testigo. ¿Exageración? Lección 18.8: nos pasó.

## 18.8 Tres bugs reales, tres lecciones (caso de estudio)

El workspace dejó constancia de tres bugs durante la construcción de este capítulo. Los estudiamos porque enseñan más que el happy path.

**Bug 1: el lexer no reconocía `<>` como `NotEq`.** La rama `b'<'` probaba `match_byte(b'-')` y `match_byte(b'=')`… pero faltaba `match_byte(b'>')`. El síntoma fue diabólico: `WHERE p.age <> 30` llegaba al parser como `... Lt Gt Integer(30)`. El `parse_comparison` consumía el `Lt` contento, llamaba a `parse_primary`… y encontraba un `Gt` huérfano: *«se esperaba uno de [literal, variable.propiedad, '('], se encontró '>'»*. Señalando a un `>` que el usuario **nunca escribió como token separado**. Fix: una línea (`else if self.match_byte(b'>') { TokenKind::NotEq }`). **Lección: los errores de lexer se pagan en el parser, con intereses de distancia.** Cuando un parser culpa a un token que nadie escribió, sospecha del oficial de registro, no del gramático.

**Bug 2: `TokenKind::Dash` no existía.** El cap. 17 definió `ArrowRight` (`->`), `ArrowLeft` (`<-`) y `DashDash` (`--`)… y olvidó el guión simple. Consecuencia: los extremos `-[ ... ]-` y `]-` de las relaciones entrantes y sin dirección **no tenían token**: `MATCH (p)<-[:KNOWS]-(f)` era imposible de parsear. Fix retroactivo al vocabulario: añadir `Dash`, que el lexer produce y `parse_relationship_pattern` consume (su cierre `]-` para Incoming, o la apertura `-[` que decide Outgoing vs Undirected). **Lección: el vocabulario de tokens se diseña DESDE la gramática** — recorre las producciones y marca cada símbolo terminal; el que no aparezca en ninguna regla sobra, y el que una regla necesite y no exista, la regla muere.

**Bug 3: `Expression::Variable` no existía.** La EBNF del cap. 17 decía `primary ::= literal | property_access | '(' expression ')'` — sin variable sola. Pero el hito `RETURN p` exige referenciar el nodo completo, y un nodo **no es una propiedad**. Fix: variante nueva `Expression::Variable { name, span }` (con sus actualizaciones de `span()`, `references_var()`, `variables()` y `Display`), y en `parse_primary` la bifurcación: viene `Dot` → `PropertyAccess` (`p.name`); no viene → `Expression::var(variable, ...)` (`p`). La tentación descartada — hacer `PropertyAccess` con propiedad opcional — habría envenenado el executor del cap. 20 con un «¿`.None`?». **Lección: cada constructor sintáctico merece su variante de AST**, aunque parezcan el mismo concepto.

Los tres bugs comparten moraleja: **el diseño de la fase 1 (tokens) y la fase 3 (AST) se audita con la gramática de por medio**. Ninguno era de los difíciles: eran huecos entre capítulos.

## 18.9 Qué hemos sacrificado

1. **Recuperación multi-error**: abortamos en el primero. El coste de sincronizar puntos de reinscripción no paga en un lenguaje didáctico.
2. **Notación científica** (`1e10`) y **separadores** (`1_000`) en números: recortes declarados en `scan_number`.
3. **Operadores unarios**: `-3` se lexea como `Dash Integer(3)` y el parser lo rechaza; LiraQL no los tiene en su gramática.
4. **Comentarios** (`// ...`): trivial de añadir en `skip_whitespace`, y buen ejercicio.
5. **Palabras clave en minúsculas**: `match` es un identificador válido; la clasificación exacta mantiene el lexer sin estado.
6. **Rendimiento**: clonamos tokens al consumirlos (`advance` devuelve `Token`, no `&Token`) y el `Vec<Token>` es completo antes de parsear. Para consultas de decenas de tokens, irrelevante; un lexer de producción opera en streaming.

## 18.10 Cómo lo hace una BBDD real

En el ecosistema Rust hay tres caminos industriales, y conocerlos es responder la pregunta «¿cuándo dejar de hacerlo a mano?»:

- **`logos`** — el lexer derivado: declaras `#[derive(Logos)]` en tu `TokenKind` con atributos de regex y la macro genera el escáner (se autodenomina «el lexer más rápido del oeste»). Elimina el boilerplate de `scan_token`… y también su enseñanza: por eso la regla del Vol.II es **primero a mano, luego con crate** — la versión `logos` de LiraQL llegará al apéndice comparativo, cuando ya sepas qué está delegando.
- **`pest`** — gramática declarativa PEG en un fichero `.pest`, con posiciones y mensajes de serie. Es **scannerless**: gramática y escaneo en una sola especificación. Es exactamente la alternativa que descartamos en la decisión nº 1: elegante, compacta… y el escaneo deja de ser visible. Su recuperación multi-error, sin embargo, es superior a la nuestra.
- **`LALRPOP`** — el pariente moderno de la lanza del dragón: generador LR(1)/LALR que compila la gramática a tablas Rust. Impresionante para gramáticas grandes y estables; deprimente de depurar cuando la tabla rechaza algo que creías válido.

¿Y las bases de datos de grafos? **Neo4j** generó durante años el parser de Cypher con JavaCC — una gramática `.jj` compilada a parser. **Kùzu** (ahora Ladybug) escribió el suyo a mano: su parser de Cypher es un descendente recursivo en `src/parser/` — la misma técnica que acabas de construir, sosteniendo un lenguaje real. Y la nueva **GQL** (estándar ISO/IEC 39075:2024) se implementa sobre la misma maquinaria de siempre: lexer, parser descendente, AST. El estándar cambia el idioma; la oficina de registro, no. Hasta **rustc** — el argumento de autoridad definitivo — usa un parser descendente recursivo escrito a mano, y sus errores con span exacto son el modelo de nuestra factura de errores.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: añade comentarios de línea `// ...` a LiraQL (pista: es una línea en `skip_whitespace`). ¿Qué test demuestra que `MATCH (p) // amigo\n RETURN p` parsea igual que sin comentario?
- *Intermedio*: reescribe SOLO el lexer con `logos` (misma `TokenKind`, mismos spans) y deja el parser intacto. ¿Cuántas líneas te ahorras? ¿Qué test de la batería actual te verifica que no rompiste nada?
- *Experto*: LiraQL ganará `+ - * /` aritméticos en el cap. 22. Implementa esa expresión con un **parser Pratt** real (tabla de potencias de enlace) y compara: ¿qué gana, qué pierde, frente a alargar la cadena de funciones? Escribe ambos y discute en el PR.

## 18.11 Lo que te llevas

- **Dos fases, dos culpas**: el lexer corta bytes y certifica spans; el parser juzga tokens contra la EBNF. Nunca al revés.
- **Maximal-munch** es la regla de oro del escaneo: `->` antes que `-`, `<>` antes que `<`. Olvidar una combinación rompe el parser lejos del error.
- **El cursor mide en bytes** porque el span certifica bytes: la unidad explícita es la lección del cap. 9 aplicada al texto.
- **El código ES la gramática**: una función por regla EBNF; la precedencia es el orden de la cadena `parse_or → parse_and → parse_not → parse_comparison → parse_primary`.
- **Errores tipados con `From` + `source()` y span en el `Display`**: el usuario ve el byte culpable, no «consulta inválida».
- **El hito**: `parse("MATCH (p:Person) RETURN p")` funciona — texto dentro, AST fuera. La Parte IV tiene motor de entrada.

## 18.12 Ojo, cuidado con…

- **Consumir en un `peek`**: `peek`, `peek_at`, `peek_next` JAMÁS avanzan `pos`. Si miras avanzando, todos los spans posteriores mienten por un byte — la cadena rota en miniatura.
- **Medir spans en caracteres**: `cañón` son 7 caracteres y 9 bytes. Los offsets del certificado son bytes UTF-8, siempre.
- **Igualdad vs discriminante**: `match_kind(&TokenKind::Ident(String::new()))` casa CUALQUIER identificador porque `check` compara `std::mem::discriminant`. Es intencional (para «dame el identificador que sea»), pero sorprende.
- **Esperar `-3` como literal**: no hay unarios; es `Dash Integer(3)` y el parser lo rechaza con `UnexpectedToken`. Recorte declarado, no bug.
- **Culpar al parser**: cuando el error señala un token que nadie escribió, el bug casi siempre está un piso abajo, en el lexer.

## 18.13 Pin de batalla

> *«Un byte tragado por el lexer no rompe el lexer: rompe el parser, y en otro sitio.»*

## 18.14 Si solo lees 30 segundos

El lexer es un bucle `while` con un cursor sobre bytes que corta el fuente en tokens, cada uno con su `Span` de nacimiento y muerte — y gana siempre la combinación más larga (`<>` antes que `<`). El parser es descendente recursivo: una función por regla de la EBNF del cap. 17, con la precedencia codificada como cadena de llamadas de la más floja (`parse_or`) a la más fuerte (`parse_primary`). Los errores son tipados por fase (`LexError` → `ParseError` vía `From`), con span en el mensaje. Y el hito ya corre: `parse("MATCH (p:Person) RETURN p")`.

## 18.15 Una historia pequeña

La tarde del bug de `<>`, el test `parse_where_todos_los_comparadores` falló solo en su segundo caso. El mensaje decía *«se encontró '>'»* y señalaba un byte del medio del `WHERE`. Media hora dale que dale al parser: `parse_comparison` parecía correcto, `parse_primary` impecable… hasta que alguien imprimió los tokens de `p.age <> 30` y apareció la secuencia maldita: `Lt`, `Gt`, separados como desconocidos. El oficial de registro había cortado la palabra en dos, y el gramático llevaba toda la tarde declarando culpable a la mitad de al lado. El fix fue una línea; la lección, permanente: cuando el parser acusa a un fantasma, pidele al lexer el manifiesto de tokens y compáralo con lo que escribiste.

## Ejercicios resueltos

**1. Tokeniza a mano `MATCH (p:Person)-[:KNOWS]->(f)` con spans.**

Cuenta bytes: `MATCH` nace en 0 y muere en 5; el espacio (5) no pertenece a nadie; `(` es `6..7`; `p` `7..8`; `:` `8..9`; `Person` `9..15`; `)` `15..16`. Ahora maximal-munch: `-` en 16 prueba con `>` (17) y juntos forman `ArrowRight` `16..18`. `[` `18..19`, `:` `19..20`, `KNOWS` `20..25`, `]` `25..26`, de nuevo `ArrowRight` `26..28`, `(` `28..29`, `f` `29..30`, `)` `30..31`, `Eof` vacío en 31. Quince tokens + Eof. Verifícalo mentalmente contra `lex_span_de_token_cubre_exactamente_su_texto` y sorpréndete con lo que NO hay: ningún token de whitespace.

**2. ¿Qué AST produce `WHERE p.x = 1 OR p.y = 2 AND p.z = 3`?**

`parse_or` arranca pidiendo `parse_and`; ese `parse_and` agrupa primero `p.y = 2 AND p.z = 3` (porque dentro busca sus `AND` llamando a niveles más fuertes); solo entonces `parse_or` encuentra su `OR` y cuelga el `And` como hijo derecho. Resultado: `Or( Compare(p.x,1), And( Compare(p.y,2), Compare(p.z,3) ) )` — es decir, `a OR (b AND c)`. Es exactamente lo que asserts `parse_precedencia_or_es_menor_que_and`.

## Ejercicios propuestos

**Esencial (recordar).** Cierra el libro y el workspace. Escribe de memoria las producciones EBNF de `return_item`, `comparison` y `primary` del cap. 17, y al lado el nombre del método del parser que las implementa. Ábrelo después y contrasta con el comentario EBNF de `cap17_liraql_ast.rs`. Criterio: las tres producciones exactas y sus funciones gemelas correctas.

**Intermedio (analizar).** Predice EN PAPEL la variante exacta de error y el byte inicial de su span para: (a) `MATCH (p:Person RETURN p.name`; (b) `MATCH (p) RETURN`; (c) `MATCH (p) WHERE p.name = "Ana RETURN p`. Verifícalo con tres tests de humo. Pistas graduadas: (1) ¿qué `expect` revienta primero en (a) y qué token encuentra en su lugar?; (2) en (b), ¿qué token mira `parse_return_item` cuando busca una expresión?; (3) en (c), ¿dónde acaba el span de un string que nunca cierra?

**Experto (crear).** Implementa la notación científica en `scan_number`: `1e10`, `2.5e3` y `1.5e-3`. Decisiones que te pide el ejercicio: ¿es un solo token (maximal-munch lo es) o `1e` + `10`? ¿qué variante de `LexError` produce `1e` sin exponente? ¿el signo del exponente exige tocar la gramática del cap. 17 o solo el lexer? Añade tests estilo `lex_flotante` y haz que el span del literal cubra TODO el lexema. Criterio: cero panics, error tipado en `1e`, y `parse` acepta `WHERE p.weight > 1.5e-3`.

## Para profundizar

- **Aho, Sethi, Ullman — *Compilers: Principles, Techniques, and Tools* (1986)**: el dragón rojo. Capítulos 2-4: escaneo, autómatas, parsing. La separación de fases de este capítulo es suya.
- **Niklaus Wirth — *Compiler Construction* (Addison-Wesley, 1996; rev. 2005)**: un compilador completo de Oberon-0 en descendente recursivo, página a página. La defensa clásica de la técnica que hemos usado.
- **Robert Nystrom — *Crafting Interpreters*** (craftinginterpreters.com): los capítulos «Scanning» y «Compiling Expressions» son la versión divertida y en Java/C de exactamente este capítulo, incluida la cadena de precedencia.
- **rustc-dev-guide** (rustc-dev-guide.rust-lang.org): la sección del parser — descendente recursivo escrito a mano, con la discusión de por qué no una tabla.
- **Documentación de `logos`, `pest` y `LALRPOP`**: los tres caminos industriales del ecosistema Rust, para cuando toque el apéndice comparativo.
- **ISO/IEC 39075:2024 (GQL)** y la gramática de Cypher de openCypher: cómo se especifica formalmente un lenguaje de grafos real — nuestra EBNF del cap. 17 es su descendiente enana.

## Mini-diálogo: la sala de máquinas

> — Entonces el lexer es un `while` con un puntero, y el parser son funciones que se llaman. ¿Eso es todo? ¿Dónde está la parte difícil?

> — En las fronteras. Que el lexer sea ciego dentro de los strings. Que gane siempre el token más largo. Que el span se derive del propio escaneo. Cada una de esas fronteras mal marcada se convierte en un bug que estalla dos puertas más allá de donde se originó — pregúntale al `<>` aquel.

> — Pero LALR generaba todo esto de una tabla…

> — Y por eso la portada del libro del dragón muestra un caballero peleando. Las tablas son potentes y opacas. Aquí, cuando algo falla, abres el depurador y la pila de llamadas te dice qué regla gramatical estaba intentando cumplirse. Para un lenguaje de diez reglas, verlo todo es la característica, no la limitación. Ya tendrás dragones que matar con generadores — cuando sepas qué hacen por dentro.

---

*(Próximo capítulo: 19 — Del AST al plan lógico. Aquí el texto ya es `Query`; ahora veremos cómo el planner la baja a un árbol de operadores — `NodeScan`, `Expand`, `Filter`, `Project` — y quién decide el orden en que se filtra y se expande.)*
# Capítulo 19 — Del AST al plan lógico

> *«El AST dice lo que pediste. El plan dice cómo se calcula. Confundirlos es la receta para ejecutar consultas equivocadas con mucho entusiasmo.»*

## 19.0 La anécdota de la esquina

En 1979, en el laboratorio de IBM en San José (California), el equipo de System R —el prototipo del que nacerían SQL/DS y DB2— publicó en SIGMOD un paper de una docena de páginas: *Access Path Selection in a Relational Database Management System*, firmado por Patricia Selinger, Morton Astrahan, Donald Chamberlin, Raymond Lorie y Thomas Price. El problema que abordaban es exactamente el nuestro, escalado: SQL permite decir **qué** quieres, y el motor tiene que decidir **cómo** conseguirlo. ¿Recorro la tabla entera o uso el índice? ¿Uno primero y filtro, o filtro primero? Su respuesta tuvo dos partes, y la segunda cambió la industria para siempre.

La primera parte era casi burocrática: convertir la consulta en una **representación interna** sobre la que se pueda razonar — bloques de consulta, expresiones de álgebra relacional. La segunda fue el primer **optimizador basado en coste**: fórmulas que combinan CPU y E/S, programación dinámica para ordenar joins, «interesting orders» y estimación de selectividad. Medio siglo después, PostgreSQL y DB2 siguen corriendo sobre el esqueleto de aquel paper, y Selinger —que acabó siendo IBM Fellow en 1994 y ACM Fellow— sigue siendo LA referencia cuando alguien pregunta «¿quién inventó el optimizador?».

Una precisión honesta antes de seguir: el paper no usa las palabras «plan lógico» ni «plan físico» — ese vocabulario maduró con Volcano y Cascadas en los años 90 y se popularizó con Catalyst en los 2010s. Habla de *query blocks* y *access paths*. Pero la idea de fondo es de 1979: **entre tu sintaxis y la ejecución hace falta una representación intermedia que un optimizador pueda reescribir sin tocar lo que escribiste**.

Este capítulo construye la primera mitad de esa idea para LiraDB: el `LogicalPlan`, el árbol de operadores que declara qué calcular. La otra mitad —elegir entre planes equivalentes, como Selinger— es el capítulo 21. Hoy sólo preparamos el terreno sobre el que se razonará.

## 19.1 Objetivo

Los capítulos 17 y 18 completaron la cadena `texto → tokens → AST`. Este capítulo da el paso siguiente: convertir ese AST en un **plan lógico** — un árbol de operadores que declara *qué* hay que calcular, sin decidir aún *cómo* ejecutarlo (eso es el motor Volcano del capítulo 20) ni *cómo óptimo* (eso es el optimizador del capítulo 21).

Vas a construir cuatro piezas, todas en `liradb-workspace/crates/vol2-liradb/src/cap19_plan_logico.rs`:

1. `Bindings` — la tabla de variables ligadas (`p → NODE`, `r → EDGE`), que responde la pregunta crítica: ¿cómo se representan las variables de un patrón?
2. `ScalarExpr` — la versión *resuelta* de `Expression`: sin spans, sin nombres sin ligar, con el tipo de binding incrustado.
3. `LogicalPlan` — el árbol: `NodeScan`, `Expand`, `Filter`, `Project`, `CartesianProduct` (y `IndexSeek`, que hoy se declara pero no se construye).
4. `lower()` — el binder que baja cláusula a cláusula un `Query` a su plan, con errores tipados y localizados.

## 19.2 Problema

Tienes el AST de la consulta estrella del libro:

```
MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name
```

El `parse()` del capítulo 18 te dio un `Query` con `Span` en cada nodo. La pregunta suena tonta: ¿por qué no ejecutar ese AST directamente — un intérprete que recorra las cláusulas y consulte el store?

Porque el AST es **sintaxis pura**. Contiene lo que pediste, en el orden en que lo escribiste, con los paréntesis que pusiste. No contiene: qué variables están ligadas en cada punto, qué comparaciones son imposibles, dónde empezaría un índice si lo hubiera, ni cómo se ordena el trabajo. Ejecutarlo directo significa resolver todas esas preguntas *sobre la marcha, en la hot path*, una y otra vez. Y significa que **nadie puede reordenar nada**: el orden de ejecución queda clavado al orden sintáctico — que es exactamente la libertad que Selinger necesitaba para poder optimizar.

El pipeline completo quedaba así, y hoy rellenamos la tercera caja:

```
  "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name"
       │   parse() (cap 18)  │    lower() (ESTE cap)    │  executor (cap 20)
       └──► Query (AST) ─────┴──►  LogicalPlan ─────────┴──► filas de resultado
                                 (cap 21 lo REESCRIBE, jamás toca el AST)
```

Y una advertencia que da título a una sección entera más abajo: un plan puede verse perfecto y estar equivocado. Nosotros mismos lo vivimos — el brief del libro traía un plan de ejemplo para esta consulta que omitía imponer `f:Person`. Se veía bien. Habría devuelto conocidos de cualquier etiqueta. De eso va la mitad de este capítulo.

## 19.3 Modelo mental

Piensa en un restaurante con estaciones.

- El **AST es el pedido manuscrito del cliente**: «lo de siempre, sin cebolla». Es lo que pediste, con tu letra, tus tachones y tu ambigüedad — y con el papel a la vista: cada nodo lleva su `Span`, se puede señalar.
- El **plan lógico es la comanda interna**: la orden de trabajo que el jefe de sala escribe con estaciones numeradas. *Estación 1: sacar todos los platos `Person`. Estación 2: por cada uno, seguir los enlaces `KNOWS` salientes. Estación 3: quedarse con los que cumplan las condiciones. Estación 4: emplatar `f.name`.* Dice qué produce cada estación y en qué orden alimenta a la siguiente — y **nada** sobre quién lo cocina ni con qué sartén.
- El **plan físico** (capítulos 20-21) es quién cocina y con qué utensilio: ¿consultamos la alacena ordenada (el índice del capítulo 15) o vaciamos todos los armarios uno a uno (el scan)?
- El **binder** —el corazón de este capítulo— es el jefe de sala que traduce tu papel a números de estación: cuando pides algo que no está en la carta, no improvisa; te dice «no tenemos `x`» **señalando la línea del folio** (`PlanError { kind, span }`).

Para la consulta estrella, la comanda que produce `lower()` es este árbol — apréndelo, porque es el ejemplo canónico del resto del libro:

```
Project(f.name)
  Filter(f:Person AND p.name = "Ana")
    Expand(p, KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
```

Léelo de abajo arriba, como se lee toda comanda: la estación de abajo produce bindings que suben. `NodeScan` liga `p` a cada nodo `Person`; `Expand` recibe cada `p` y liga `f` (y la arista, si tuviera nombre) por cada `KNOWS` saliente; `Filter` descarta las combinaciones que no cumplen; `Project` emplata la columna de salida. El momento ¡ajá! llega al fijarte en una asimetría: `p:Person` alimenta el `NodeScan`, pero `f:Person` vive dentro del `Filter` como predicado. No es capricho — y entender por qué es entender el bug del brief. Antes de eso, veamos qué haría un novato.

## 19.4 Primera solución

La versión que escribiría cualquiera: un intérprete del AST, todo en una función.

```rust
// Solución ingenua: ejecutar el AST directamente.
fn ejecutar_ast(q: &Query, store: &dyn GraphStore) -> Vec<Vec<Value>> {
    let mut filas = Vec::new();
    for path in &q.match_clause.patterns {
        // recorrer el patrón recursivamente contra el store...
        for nodo in store.iter_nodes() { /* ¿y si el label está en el 2º nodo? */
            for arista in store.edges_of(nodo.id()) { /* ¿dirección? ¿tipo? */
                // ¿esta variable ya estaba ligada? ¿existe siquiera?
            }
        }
    }
    // después: WHERE... ¿con qué tabla de símbolos? ¿y si falta una?
    // después: RETURN... ¿en qué orden salen las columnas?
    filas
}
```

Funciona. Durante un rato. Los tests simples pasan: `(p:Person)` escanea y devuelve. Pero fíjate en los comentarios — cada uno es una pregunta que el intérprete responde *tarde, con el grafo ya a medio recorrer*, y cada una tiene una respuesta correcta que no es «resolverla ahora».

## 19.5 Sus límites

La solución ingenua se rompe de tres formas distintas, y conviene separarlas:

1. **Errores descubiertos tarde.** Ejecuta `MATCH (p:Person) WHERE x.name = "A" RETURN p` con el intérprete: el error (`x` no está ligada por ningún patrón) aparece cuando el WHERE se evalúa, con el scan ya recorrido. El AST tiene el `Span` de `x.name` — pero el intérprete no tiene un lugar donde usarlo *antes* de tocar el store. La detección correcta es estática: antes de ejecutar nada, contra la tabla de variables que el MATCH liga.
2. **Índices invisibles.** El capítulo 15 construyó un `HashIndex` que responde `name = "Ana"` en O(1). Un intérprete de AST no tiene dónde «ver» eso: no existe árbol que reescribir, así que la única implementación posible es el escaneo completo. Cada optimización futura exigiría tocar sintaxis — la libertad que Selinger pagó su paper por conseguir.
3. **Resultados equivocados sin error.** La más traicionera: si el intérprete olvida imponer la etiqueta del nodo destino (¡exactamente el resbalón del brief!), la consulta devuelve conocidos de cualquier etiqueta. No crashea. No protesta. **Devuelve filas de más con cara de estar bien.** Ningún test que sólo compruebe «no revienta» lo detectaría.

Lo que necesitamos es una fase separada que *liga* variables, *resuelve* expresiones, *verifica* tipos y *estructura* el trabajo — antes de ejecutar nada. Eso es un binder, y su salida es el plan.

## 19.6 Solución evolucionada

El código completo vive en `cap19_plan_logico.rs` (1.985 líneas, 40 tests). Vamos por piezas, y cada pieza con su porqué.

### Bindings: la tabla de variables ligadas

La pregunta crítica del capítulo —¿cómo se representan las variables de un patrón?— tiene aquí su respuesta: una tabla nombre → clase de elemento:

```rust
pub struct Bindings {
    entries: Vec<(String, BindingKind)>,   // en orden de declaración
}
pub enum BindingKind { Node, Edge }
```

`declare()` rechaza duplicados; `get()` consulta. Y la decisión de diseño que más preguntas recibe: **un `Vec` ordenado, no un `HashMap`**. El motivo es determinismo: el mismo AST debe producir exactamente el mismo orden de ligadura en cada ejecución, porque ese orden se filtra a todo el sistema — el `Display` del plan, las columnas de `bound_variables()`, y el orden en que el executor del capítulo 20 materializará las filas. Un `HashMap` no garantiza orden de iteración: tendrías tests intermitentes y un `explain` no reproducible entre ejecuciones. El coste es O(n) por consulta — aceptable cuando n es «variables de un MATCH».

Mientras el binder baja `(p:Person)-[:KNOWS]->(f:Person)`, la tabla crece en silencio: `{}` → `{p:NODE}` → `{p:NODE, f:NODE}`. El `Display` la pinta tal cual (`{p:NODE, r:EDGE}`), y ese texto es el que verás en mensajes y tests.

### Variables internas: los anónimos también ligan

Aquí está la primera misconception seria del capítulo: «`( )` anónimo no liga nada». **Falso.** Un nodo anónimo liga exactamente igual que uno nombrado — el executor necesita *nombrarlo todo* para saber qué hay en cada fila. La diferencia es que el binder le pone un nombre interno:

```rust
fn fresh_internal_var(&mut self, prefix: &str) -> String {
    loop {
        self.next_internal += 1;
        let candidate = format!("_{prefix}{}", self.next_internal);
        if !self.bindings.contains(&candidate) {
            return candidate;          // _n1, _e2, ... saltando ocupados
        }
    }
}
```

`_n1` para nodos, `_e2` para aristas. El bucle que salta nombres ocupados evita una colisión sutil: un usuario puede escribir `MATCH (_n1:Person)` — y su variable tiene tantos derechos como las internas. La alternativa descartada era `Option<String>` en cada operador: duplicaría los `match` por todo el motor y envenenaría `Expand { to }` para ahorrar un string. (Nota de honestidad: el `validate()` del capítulo 17 rechazaba el `()` desnudo por «inútil»; el binder es deliberadamente más permisivo y lo liga con interna — la divergencia está documentada en test: `lower_dos_patrones_con_anonimos_no_colisionan`.)

### ScalarExpr: la expresión resuelta

El WHERE y el RETURN del AST traen `Expression` — sintaxis con spans. El plan necesita su versión *resuelta*, `ScalarExpr`, y las diferencias son toda una filosofía:

- **Sin `Span`.** El span vive y muere en el frente sintáctico: sirve para señalarte el fuente, y el plan ya no está en el fuente — está camino de ejecución. Cuando `lower()` detecta un error, usa el span *del AST* para el mensaje (fíjate en `build_scalar`: construye el error con `*span` del nodo sintáctico) y el plan resultante no arrastra posiciones. El plan es para siempre; el fuente, sólo al diagnosticar.
- **`Var { name, kind }` con el tipo de binding incrustado.** En el AST, `p` es un nombre; en el plan, `p` ya sabe que es un NODE. El executor del capítulo 20 jamás re-resolverá nombres: no habrá tabla de símbolos en la hot path. Es la versión miniatura del patrón binder de las bases de datos reales (Kùzu documenta su pipeline como Parser → Binder → Planner por esta razón).
- **`HasLabel { variable, label }` — una variante que no existe en la sintaxis.** En LiraQL no puedes escribir `f:Person` como expresión suelta: la construye el planner. Guárdala: es la protagonista de la próxima sección.

Además de `and_all()` (la conjunción left-asociativa que apila predicados: `[a, b, c]` → `And(And(a, b), c)`, y `None` si la lista está vacía — sin predicados no hay `Filter`), `ScalarExpr::type_of()` hace la inferencia de tipos de la que hablaremos en dos secciones.

### El bug del brief: el caso de estudio estrella

Vamos a lo prometido. Cuando migramos este capítulo al workspace, el plan de ejemplo del brief original para la consulta estrella era:

```
Project(f.name)
  Filter(p.name = "Ana")          ← ¡falta f:Person!
    Expand(p, KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
```

Se ve bien. Es compacto. Y es **semánticamente incompleto**: el patrón pide `(f:Person)` y el plan no impone nada sobre la etiqueta de `f`. Ejecutado tal cual, el `Expand` ligaría `f` a cualquier vecino de una Persona vía `KNOWS` — conocidos con etiqueta `City`, `Paper`, lo que fuera. El WHERE sólo filtra por `p.name`, así que todas esas filas de más pasarían el filtro y llegarían a tu `RETURN f.name`. **Nadie se quejaría: la consulta devolvería más de lo pedido, con pinta de éxito.** Es el modo de fallo favorito de los binder descuidados, porque no produce errores — produce silencio con datos sucios.

¿Por qué lo omitió el brief? Por una asimetría real del diseño: `NodeScan` tiene un campo `label`, y sólo puede absorber la etiqueta de *su* nodo — el inicial del camino. `p:Person` alimenta el scan. Pero `f` no tiene scan propio: nace en el `Expand`. Y la etiqueta de `f` tiene que vivir en algún sitio. La solución del código real:

```rust
let scan_label = if label_como_predicado {
    if let Some(label) = &np.label {
        predicates.push(ScalarExpr::has_label(&variable, label));  // baja al Filter
    }
    None                                                            // el scan queda sin label
} else {
    np.label.clone()                                                // sólo el nodo inicial
};
```

El label del nodo inicial alimenta el `NodeScan` (es el único sitio donde una etiqueta NO es un predicado); el de los nodos de la cadena baja como predicado `HasLabel` y se conjunta en el `Filter` global. Por eso el plan correcto es `Filter(f:Person AND p.name = "Ana")`. Es el mismo patrón que Neo4j usa para los labels hasta que su optimizador reordena.

Y la red de seguridad quedó tejida para siempre: el test canónico compara el texto EXACTO del plan,

```rust
assert_eq!(plan.to_string(),
    "Project(f.name)\n  Filter(f:Person AND p.name = \"Ana\")\n    \
     Expand(p, KNOWS, OUTGOING, f)\n      NodeScan(Person AS p)");
```

La lección que nos quedamos grabada (MIGRATION-PATTERN §23): *un plan puede ser «correcto de pinta» y semánticamente incompleto; al traducir un plan a código ejecutable, cada restricción del patrón —label, props, dirección, tipo— debe quedar representada en algún operador o predicado.* El test de display es la forma barata de que nada se pierda.

### LogicalType: prometer poco, en un mundo sin esquema

`type_of()` infiere el tipo de cada `ScalarExpr`: `LogicalType` con nueve variantes (`Any`, `Null`, `Bool`, `Int`, `Float`, `String`, `Bytes`, `Node`, `Edge`). La decisión clave es ser **conservadores**:

- Una propiedad (`p.name`) tipa `Any`. No `String`. LiraDB es schemaless desde el capítulo 7: el store no garantiza que `name` exista ni que sea texto. Prometer un tipo que nadie garantiza genera falsos positivos — consultas legales rechazadas.
- `Any` (y `Null`) son **comodines**: compatibles con todo en igualdad. Así, `p.edad = TRUE` pasa el plan (quizá alguien guarda Bool en `edad`) y se resuelve en ejecución.
- `TypeMismatch` sólo cuando es **probable, no posible**: `WHERE 3` (un Int como condición), `p = TRUE` con `p` ligada a nodo, `TRUE < FALSE` (los Bool no son ordenables), `1 AND 2`. Concretos contra concretos incompatibles: error. Todo lo demás: adelante, y el capítulo 20 dirá la última palabra.

La frontera entre igualdad y orden es deliberada: `eq_compatible` acepta iguales entre sí, numéricos cruzados (`Int` vs `Float` promociona) y comodines; `order_compatible` sólo numéricos y strings — porque `Node < Node` o `Bool < Bool` no significan nada (¿`TRUE < FALSE`?), mientras que `a = b` entre nodos sí (identidad: ¿es el mismo nodo? — el capítulo 20 lo usará para self-loops).

### LogicalPlan: el árbol de operadores

Con las piezas anteriores, el árbol es casi una declaración:

- **`NodeScan { variable, label }`** — la hoja: liga `variable` a cada nodo con `label` (todos si `None`). Es siempre el punto de partida de un camino.
- **`Expand { input, from, rel_variable, rel_type, direction, to }`** — un tramo de relación: por cada binding de `from` que le llega, recorre las aristas de `rel_type` (todas si `None`) en `direction` y liga `to` (y la arista, si el patrón la nombra: `-[r:KNOWS]->` liga `r`). Un camino de tres nodos encadena dos `Expand`, como muñecas rusas.
- **`Filter { input, predicate }`** — se queda con los bindings que cumplen el predicado (WHERE + todo lo inline, conjuntado).
- **`Project { input, items }`** — el RETURN: una columna por proyección, con alias (`AS nombre`) o nombre derivado (`p.name` se llama `p.name`).
- **`CartesianProduct { left, right }`** — patrones disjuntos separados por coma: `MATCH (a:Person), (b:City)` produce el producto de sus matches, porque eso ES la coma en Cypher. Correcto pero ingenuo — el capítulo 21 lo reordenará. Que dos patrones *compartan* variables es otro cantar: exige un join, y eso hoy es error (`SharedPatternVariables`, con el mensaje apuntando al capítulo que lo resolverá).
- **`IndexSeek`** — el operador que *no* construye nadie hoy. Está declarado en el enum con sus `ids: Vec<NodeId>` ya resueltos, porque el plan lógico debe poder **expresar** el uso de índice para que alguien pueda elegirlo; pero elegir es optimizar, no planificar. La regla `index_seek` del capítulo 21 reescribirá `Filter(name = "Ana") + NodeScan` en `IndexSeek(Person.name = "Ana")`. Si lo construyéramos aquí, mezclaríamos capas y el binder tendría que conocer catálogos y estadísticas que no le pertenecen.

El árbol es inmutable y sin magia: los hijos van en `Box` dentro de cada variante, y lo que el `Display` dibuja es exactamente la estructura. `bound_variables()` la recoge en orden de ligadura y sin duplicados — es exactamente la API que el push-down del capítulo 21 necesita: *un predicado sólo puede bajar hasta un operador si sus variables ya están ligadas allí*.

### lower(): tres pasos, cláusula a cláusula

La función pública es `lower(&Query) -> Result<LogicalPlan, PlanError>` (más el atajo `query.lower()`):

1. **MATCH** — un fragmento de plan por patrón (`lower_path` encadena `NodeScan` + `Expand`s); antes de bajar cada patrón, se comprueba que no comparta variables con lo ya ligado (eso exigiría join → `SharedPatternVariables`); los fragmentos se combinan con `reduce` en `CartesianProduct`. Los predicados inline (labels de la cadena, props `{edad: 30}` → `p.edad = 30`) se acumulan.
2. **WHERE** — `build_scalar` resuelve la expresión contra `Bindings` (aquí muere toda variable sin ligar, con su span del AST); `type_of` exige raíz `Bool` o `Any` (`WHERE 3` → `TypeMismatch { context: "WHERE" }`); el predicado se añade a la lista y `and_all` lo conjunta todo en UN `Filter`.
3. **RETURN** — cada item se resuelve, se type-checkea (sí: `RETURN NOT 3` se caza aquí, no en ejecución) y forma una `Projection`; el plan se envuelve en `Project`, que es siempre la raíz.

Los errores son `PlanError { kind, span }` — el mismo patrón `{ kind, span }` de `QueryError` y `ParseError`, con `write_span_suffix` para el `(en start..end)` y `std::error::Error` implementado. Siete variantes, cada una con su porqué: `EmptyMatch`/`EmptyReturn` (sólo alcanzables con ASTs construidos a mano — `parse()` ya lo impide; el binder no confía en nadie), `UnknownVariable`, `DuplicateVariable` (declarar dos veces en todo el MATCH), `VariableRebind` (re-ligar *dentro* del mismo patrón — eso es un ciclo, y los ciclos los resuelve el executor del cap. 20, no el plan), `SharedPatternVariables` (el join pendiente) y `TypeMismatch`.

Fíjate en la doble barrera que forma esto con el `validate()` del capítulo 17: `validate()` es UX (reporta todos los errores de golpe, para arreglar la query en una pasada); `lower()` es la puerta de corrección — no confía en que alguien validó antes, porque también llega con ASTs programáticos. Dos capas, la misma invariante: la corrección la garantiza la que no se puede saltar.

### El Display canónico: la cara pública del plan

```text
Project(f.name)
  Filter(f:Person AND p.name = "Ana")
    Expand(p, KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
```

Dos espacios por nivel; el tramo de relación se pinta como en Cypher (`r:KNOWS`, `KNOWS`, `r`, o `ANY` si el patrón no restringe); sin etiqueta, `NodeScan(ANY AS p)`. Y dentro de los predicados, **paréntesis mínimos por precedencia** `NOT > AND > OR`: `b:Person AND (a.age > 30 OR b.age > 40)` necesita los paréntesis; `a AND b AND c` no. La sutileza (nos mordió en migración): la misma expresión se envuelve distinto según cuelgue de un `AND`, un `OR` o un `NOT` — son reglas por contexto, no un flag global.

¿Por qué tanto esmero en un pretty-printer? Porque este texto no es decoración: es la base de `liradb explain` (capítulo 21) y el oráculo de los tests de lowering. Ser canónico —idempotente, sin ruido, sin ambigüedad— es lo que permite escribir `assert_eq!(plan.to_string(), "...")` y dormir tranquilos. Fue, literalmente, la red que cazó al bug del brief.

## 19.7 Prueba de fuego

Los 40 tests del módulo ejercitan el capítulo entero. Los que prueban las promesas centrales:

- **El plan correcto, texto y estructura**: `lower_display_ejemplo_canonico_del_brief` y `lower_estructura_del_ejemplo_canonico` (el predicado es exactamente `And(HasLabel(f, Person), p.name = "Ana")`, y el `Filter` queda ENCIMA del `Expand` — sin push-down).
- **Anónimos e internas**: `lower_nodo_anonimo_genera_variable_interna` (`Expand(p, KNOWS, OUTGOING, _n1)`), `lower_dos_patrones_con_anonimos_no_colisionan`.
- **Inline y conjunción**: `lower_propiedades_inline_bajan_al_filter`, `lower_where_y_props_inline_se_conjuntan_en_un_filter`, `lower_path_de_tres_nodos_encadena_expands` (`Filter(b:Person AND c:Person)` — dos labels bajando).
- **Direcciones y relaciones**: `lower_direccion_entrante_y_sin_definir` (`INCOMING`/`UNDIRECTED`), `lower_relacion_con_variable_y_sin_tipo` (`r:KNOWS`, `ANY`).
- **La coma**: `lower_patrones_disjuntos_cartesian_product` y `lower_patrones_que_comparten_variables_exigen_join`.
- **Errores localizados**: `lower_where_variable_no_ligada` — y fíjate en el detalle: el span apunta al ACCESO ofensivo (`x.name`), no a toda la cláusula. Detectado aquí, con el fuente aún a mano, no en ejecución. Más `lower_where_no_booleano`, `lower_where_igualdad_imposible`, `lower_where_property_schemaless_pasa` (el caso que NO se rechaza), `lower_return_item_type_checkeado`.
- **El pipeline entero**: `integracion_parse_lower_plan_pipeline_completo` (parse → validate → lower → `bound_variables()` = `[p, f]`), `plan_display_es_estable_e_idempotente`, `plan_error_display_localiza_y_es_std_error`.

¿Y el segundo escenario de fallo del capítulo —el plan que escanea todo cuando existiría un índice? Está ahí, a propósito, esperándote: con un millón de `Person` y una sola `Ana`, este plan hace que `NodeScan` produzca un millón de filas para que el `Filter` deje una. El `HashIndex` del capítulo 15 sabría responder `name = "Ana"` sin mover un músculo. El operador `IndexSeek` ya existe en el enum... y `lower()` jamás lo construye. Si te hierve la sangre mirando ese `Filter` arriba del árbol: bien. Esa indignación es el programa del capítulo 21. Si te saltas este capítulo, el síntoma es el de siempre: errores de nombres descubiertos tarde (o nunca) y resultados con filas de más y sin diagnóstico.

## 19.8 Qué hemos sacrificado

1. **Sin push-down**: el `Filter` queda arriba del MATCH completo. Correcto, ingenuo, deliberado — el «antes» del capítulo 21.
2. **Sin join entre patrones**: compartir variables entre comas es error tipado, no join implícito. Mejor un no rotundo que un join a medias.
3. **Sin ciclos ni re-binding**: `(a)-[:X]->(a)` es `VariableRebind`. Los ciclos son trabajo del executor, no del árbol.
4. **Inferencia tímida**: casi todo lo que toca una propiedad pasa el plan. El coste: algunos errores de tipos saltan en ejecución (capítulo 20) en vez de aquí. El beneficio: nunca rechazamos una consulta legal.
5. **`IndexSeek` declarado pero no construido**: el binder no conoce catálogos ni estadísticas — y no debe conocerlos.

## 19.9 Cómo lo hace una BBDD real

- **PostgreSQL** divide la vida de una consulta en parse → rewrite → plan → execute, y su planificador trabaja sobre una representación interna de la consulta. `EXPLAIN (VERBOSE)` te enseña el árbol con el targetlist (las expresiones de salida) de cada nodo: es lo más parecido a «ver el plan lógico» que muestra por defecto — cada nodo de scan, join y sort con sus columnas. La descendencia del DP de Selinger vive ahí dentro.
- **Catalyst (Spark SQL)** es la encarnación moderna y más pedagógica de la separación: árbol lógico **sin resolver** → *Analyzer* que resuelve referencias contra el catálogo (nuestro binder: `Var { kind }` incrustado es exactamente «resolved») → reglas de optimización lógica (predicate pushdown, column pruning — el capítulo 21) → *physical planning* con estrategias que eligen el operador físico.
- **Kùzu**, la base de datos de grafos embebida que inspiró parte de la arquitectura de LiraDB, documenta su pipeline como Parser → **Binder** → Planner → Optimizer → Executor. Su binder resuelve expresiones contra el catálogo igual que el nuestro contra `Bindings` — la palabra que dimos a nuestra fase es la palabra del oficio.
- **Neo4j** muestra con `EXPLAIN` el plan de una consulta Cypher con operadores cuyo parentesco con los nuestros es directly visible: `NodeByLabelScan`, `Expand (All)`, `Filter`, `Projection`, `CartesianProduct`. Los labels que no puede absorber el scan bajan como predicados — el mismo arreglo que curó nuestro bug del brief.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en Catalyst, ¿qué fase corresponde a nuestro `lower()` y cuál al capítulo 21? ¿Qué nodo de nuestro plan es el «resolved» del que habla la documentación de Spark?
- *Intermedio*: PostgreSQL tipa las columnas con esquema; LiraDB tipa las propiedades a `Any`. ¿Qué errores puede detectar PostgreSQL en plan que nosotros dejamos para ejecución — y qué consultas legales podría rechazar si prometiéramos tipos como él?
- *Experto*: el optimizador de Selinger sólo consideraba planes "left-deep" en su DP. Busca por qué (pista: tamaño del espacio de búsqueda) y estima cuántos órdenes hay para 5 relaciones si permites bushy trees.

## 19.10 Lo que te llevas

- **El AST es sintaxis; el plan es álgebra**: qué pediste vs cómo se calcula. Ejecutar el AST directo clava el orden de ejecución al orden sintáctico y mata la optimización.
- **`Bindings` responde la pregunta crítica**: variables ligadas en orden de declaración, `Node`/`Edge`, deterministas para tests, explain y executor.
- **Los anónimos también ligan** (`_n1`, `_e2`): el executor necesita nombrarlo todo.
- **`ScalarExpr` es `Expression` resuelta**: sin span (el span muere en parse), con el `kind` incrustado (nadie re-resuelve nombres después), y con `HasLabel` — una variante que no existe en la sintaxis y que fabrica el planner.
- **El bug del brief**: un plan puede verse perfecto y estar incompleto; las filas de más no se quejan. Cada restricción del patrón debe aterrizar en un operador o predicado — y el test canónico del display es la red.
- **Tipado conservador**: `Any` = «no prometido» por schemaless; se rechaza lo PROBABLE, jamás lo meramente posible.
- **El plan es deliberadamente ingenuo**: `Filter` arriba y `CartesianProduct` son el «antes» que da sentido al capítulo 21. Planificar no es optimizar — la separación es la lección de 1979.

## 19.11 Ojo, cuidado con…

- **Confundir los tres errores de variables**: `DuplicateVariable` (dos veces en todo el MATCH), `VariableRebind` (dos veces en el MISMO patrón — ciclos: capítulo 20), `SharedPatternVariables` (entre patrones con coma — join: capítulo 21). Tres confusiones, tres capítulos.
- **Buscar `HasLabel` en la gramática**: no está. Es un predicado fabricado — la sintaxis no sabe de predicados, el plan sí.
- **Esperar optimización del binder**: si te tienta bajar ese `Filter` «ya que estamos», recuerda qué capítulo es este. El binder construye; el optimizador reescribe.
- **Creer que `p:Person` SIEMPRE baja al `Filter`**: sólo el label del nodo inicial alimenta el `NodeScan`; el resto baja como predicado. Confundirlo es reeditar el bug del brief al revés.

## 19.12 Pin de batalla

> *«Un plan lógico incompleto no lanza errores: devuelve filas de más con cara de éxito. Por eso el test del plan es texto exacto, no vibes.»*

## 19.13 Si solo lees 30 segundos

`lower()` convierte el AST en un árbol de operadores: `NodeScan` liga la variable inicial (y absorbe SU label), `Expand` encadena cada tramo de relación ligando el siguiente nodo, y TODO lo demás —labels de la cadena, propiedades inline, el WHERE— se conjunta en un único `Filter` arriba, deliberadamente ingenuo. `Project` corona el árbol con el RETURN. Las variables viven en `Bindings` (orden de declaración; los anónimos como `_n1`), las expresiones bajan resueltas a `ScalarExpr` (sin spans, con el tipo incrustado), y los errores —variable no ligada, tipos imposibles, re-ligaduras— se detectan aquí, con span, antes de tocar el store. El `Display` del plan es canónico: base de `liradb explain` y red que cazó al bug del brief.

## 19.14 Una historia pequeña

El bug del brief nos enseñó más que cualquier sección limpia de este capítulo. El plan de ejemplo llevaba meses impreso en el documento: cuatro líneas, alineadas, con su `Expand` y su `NodeScan` — y sin `f:Person` por ninguna parte. Había sobrevivido revisiones porque *se leía bien*. El día que lo convertimos en un test con `assert_eq!` sobre el texto exacto del plan, la ausencia saltó a la primera ejecución: el plan real tenía un predicado más que el del brief. ¿Cuál de los dos estaba mal? El que devolvía conocidos con etiqueta `City`, claro — pero nadie lo habría notado sin el test, porque las consultas de prueba nunca mezclaban etiquetas en los vecinos. Desde entonces, en LiraDB, ningún plan de ejemplo entra al libro sin su test canónico de display. La sintaxis se lee; el plan se verifica.

## Ejercicios resueltos

**1. Escribe el `Display` completo del plan de `MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN b` y sus `bound_variables()`.**

De abajo arriba: el nodo inicial `a` alimenta el scan con SU label → `NodeScan(Person AS a)`; el tramo encadena → `Expand(a, KNOWS, OUTGOING, b)`; el label de `b` (nodo de la cadena, ¡lección del brief!) baja como predicado → `Filter(b:Person)`; y el RETURN corona → `Project(b)`. Junto: `Project(b)` / `  Filter(b:Person)` / `    Expand(a, KNOWS, OUTGOING, b)` / `      NodeScan(Person AS a)` (indentación de 2 espacios por nivel). Variables: `["a", "b"]`, en orden de ligadura. Patrón verificable: `lower_path_de_tres_nodos_encadena_expands` muestra el mismo mecanismo con dos tramos.

**2. ¿Por qué `WHERE p.edad = TRUE` pasa el plan, pero `WHERE p = TRUE` no, siendo `p` un nodo?**

`p.edad` es un acceso a propiedad en un motor schemaless (capítulo 7): tipa `Any`, y `Any` es comodín — compatible con cualquier cosa en igualdad. La comparación concreta (¿es Bool? ¿existe?) se resuelve en ejecución. En cambio, `p` tipa `Node`: `Node` vs `Bool` son dos tipos concretos incompatibles, y eso el plan SÍ lo sabe → `TypeMismatch { context: "comparación de igualdad", expected: Bool, got: Node }`, con span de la comparación. La frontera exacta: rechazamos lo PROBABLE, aplazamos lo POSIBLE. Tests: `lower_where_property_schemaless_pasa` (pasa) y `lower_where_igualdad_imposible` (no pasa).

## Ejercicios propuestos

**Esencial (recordar + aplicar — retrieval puro).** Cierra el libro y, DE MEMORIA, dibuja el árbol del plan y escribe su `Display` exacto para `MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age > 30 OR b.age > 40 RETURN a, b`. Atención a los detalles que separan al que sabe del que reconoce: ¿dónde acaba el label de `b`?, ¿el `OR` necesita paréntesis dentro del `AND`... y al revés?, ¿qué pinta `Expand` entre `a` y `b`? Verifica después con el patrón del test `plan_display_es_estable_e_idempotente`. *Pistas* (sólo si te atascas): (1) sólo UN label alimenta un scan; (2) `NOT > AND > OR`, con paréntesis por contexto; (3) `Expand(from, rel, DIRECCIÓN, to)`. *Criterio*: árbol y texto exactos — el `Filter` debe decir `b:Person AND (a.age > 30 OR b.age > 40)`.

**Intermedio (analizar).** Toma la consulta estrella y los dos planes candidatos: el del brief original (sin `f:Person`) y el de `lower()`. (a) Explica con un grafo concreto —dos `Person` y un `City` vecino vía `KNOWS`— qué filas devuelve cada uno. (b) ¿Por qué ningún usuario protestaría en un grafo homogéneo? (c) Escribe el test estructural que cazaría la regresión para siempre: qué `assert_eq!`/`matches!` sobre el predicado del `Filter`. *Pistas*: (1) el `Filter` del plan del brief sólo menciona a `p`; (2) ¿qué etiqueta tiene el vecino del grafo de prueba?; (3) `lower_estructura_del_ejemplo_canonico` ya lo hace — entiende cada línea antes de copiarla. *Criterio*: identificar el predicado ausente, el modo de fallo silencioso y el test como vacuna.

**Experto (crear).** Implementa `RETURN *`: proyectar TODAS las variables ligadas por el MATCH, en orden de declaración (`Project(p, f)` para la consulta estrella). Decide y documenta: ¿proyectas las variables internas `_n1` o las excluyes — y por qué? ¿Qué le pasa al `Display` con alias? Escribe el test de display y uno con un patrón de tres nodos (¿en qué orden salen `a`, `b`, `c`?). *Pistas*: (1) ¿qué estructura conserva el orden de ligadura y qué método lo itera?; (2) mira el paso 3 de `lower()` — ¿dónde viven aún los bindings en ese punto?; (3) `Projection::output_name()` ya sabe derivar nombres de `Var`. *Criterio*: display exacto, orden de declaración, decisión explícita sobre internas, tests verdes con `cargo test -p vol2-liradb`.

## Para profundizar

- **Selinger, Astrahan, Chamberlin, Lorie y Price, «Access Path Selection in a Relational Database Management System» (SIGMOD 1979)** — el paper de la anécdota: el nacimiento de la representación intermedia y del optimizador de coste. Doce páginas legibles que siguen vivas (ACM DL, 10.1145/582095.582099).
- **Graefe & McKenna, «The Volcano Optimizer Generator» (VLDB 1993)** y **Graefe, «Volcano—An Extensible and Parallel Query Evaluation System» (IEEE TKDE 1994)** — donde el vocabulario lógico/físico y los árboles de transformación se formalizan (y de donde viene el modelo del capítulo 20).
- **Armbrust et al., «Spark SQL: Relational Data Processing in Spark» (SIGMOD 2015)** — Catalyst: análisis → optimización lógica → plan físico, explicado por sus autores. El paralelo moderno más claro de este capítulo.
- **Kùzu, documentación de arquitectura** (docs.kuzudb.com) — el pipeline Parser → Binder → Planner → Optimizer de una base de grafos embebida real.
- **Neo4j Manual, «Execution Plans»** — `NodeByLabelScan`, `Expand (All)`, `Filter`, `Projection`: nuestros operadores con nombre de producción.
- **PostgreSQL docs, `EXPLAIN (VERBOSE)`** — targetlists por nodo: el plan interno asomando.
- **Ramakrishnan & Gehrke, «Database Management Systems»**, capítulos de query processing; **CMU 15-445**, lecciones de query execution y optimization.

## Mini-diálogo: la comanda nocturna

> — A ver si lo pillo. El capítulo 18 me dio el AST, y en vez de ejecutarlo… ¿lo he vuelto a convertir en otra cosa? ¿No estábamos haciendo una base de datos y no una fábrica de árboles?

> — Es el último árbol, te lo prometo. Pero fíjate en lo que ha cambiado de manos: el AST tenía tu sintaxis; el plan tiene el trabajo repartido en estaciones. La diferencia se ve el día que quieres cambiar algo — porque el plan se puede reescribir sin tocarte a ti.

> — ¿Y el bug del brief? Un plan que se veía bien y devolvía conocidos de cualquier etiqueta.

> — Ese es el examen de verdad del capítulo. La sintaxis se lee y parece correcta; el plan se ejecuta y miente en silencio. Por eso el test compara el texto exacto: «parecía bien» no es una categoría de ingeniería.

> — Y el `Filter` ahí arriba, escaneando un millón de nodos con un índice al lado…

> — Lo ves, ¿verdad? Esa comezón es el producto real de este capítulo. El 21 viene a rascar justo ahí.

---

*(Próximo capítulo: 20 — El motor de ejecución Volcano. El plan ya declara qué calcular; ahora alguien tiene que recorrerlo operador a operador, fila a fila — y descubrir qué vale `p.edad` cuando no existe.)*
# Capítulo 20 — El motor de ejecución (modelo Volcano)

> *«Una consulta no se calcula. Se va pidiendo.»*

## 20.0 La anécdota de la esquina

En 1994, Goetz Graefe publicó en *IEEE Transactions on Knowledge and Data Engineering* un sistema de investigación llamado **Volcano** («Volcano — An Extensible and Parallel Query Evaluation System»). No fue el primer motor de consultas, ni el más rápido, ni el que más gente usó. Pero dejó una idea tan limpia que, treinta años después, casi todos los motores que existen la implementan: **cada operador de un plan es un iterador con tres métodos — `open`, `next`, `close` — y el resultado se construye pidiendo una fila cada vez**. El consumidor tira de la cadena; nadie calcula nada que no se haya pedido.

La genealogía de Graefe es la columna vertebral de los motores modernos: su sistema anterior (EXODUS) y Volcano engendraron el framework de optimización **Cascades** (1995), que es literalmente el optimizador con el que Microsoft construyó **SQL Server**. Y el modelo de ejecución de Volcano —el «iterator model» que los apuntes de CMU 15-445 describen como «el más común, usado por casi todos los DBMS orientados a fila»— es el que siguen PostgreSQL, MySQL, Neo4j y el nuestro. Cuando en este capítulo escribas `fn next(&mut self) -> Result<Option<Row>, ExecError>`, estarás escribiendo la misma firma conceptual que Graefe describió en 1994. Bienvenido al club.

## 20.1 Objetivo

Al terminar este capítulo habrás **cerrado el círculo**: texto → tokens → AST → plan lógico → **filas**. Los caps. 17-19 construyeron la mitad izquierda de esa cadena; aquí construimos la mitad que falta y ejecutamos consultas completas desde una cadena de texto.

En concreto, vas a construir:

1. `PhysicalOperator` — el trait con la tríada Volcano (`open`/`next`/`close`) más observabilidad (`name`, `rows_produced`, `collect_metrics`).
2. Ocho operadores — `NodeScanOp`, `IndexSeekOp`, `ExpandOp`, `FilterOp`, `ProjectOp`, `CartesianProductOp`, `LimitOp`, `DistinctOp`.
3. `Row` y `Cell` — la fila que circula por el pipeline: variables ligadas a nodos, aristas o escalares.
4. `eval_scalar` — evaluación de `ScalarExpr` con semántica NULL de SQL/Cypher y lógica trivalente con cortocircuito real.
5. `Executor` + `run(src, store)` — el ciclo completo y el hito del libro.

## 20.2 Problema

Tienes un `LogicalPlan` (cap. 19): un árbol bonito como

```
Project(f.name)
  Filter(p.name = "Ana")
    Expand(p, KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
```

¿Qué significa «ejecutarlo»? La pregunta esconde cuatro decisiones incómodas:

- **¿Cuándo se calculan las filas?** ¿Todas de golpe, o una a una? (Veremos que esta decisión lo cambia todo.)
- **¿Qué ES una fila aquí?** Abajo del `Project` las filas son *variables ligadas a elementos del grafo* (`p` → un nodo, `r` → una arista); arriba son *columnas de resultado*. Y `RETURN p` exige poder devolver un nodo entero, no un `Value`.
- **¿Qué significa `WHERE` cuando una propiedad no existe?** Nuestro grafo es schemaless (cap. 7): `p.nick` puede no estar. ¿Eso es false? ¿Un error?
- **¿Quién limpia si algo falla a mitad?** Si el `Filter` revienta en la fila 3 de un millón, ¿quién cierra los cursores del scan?

Este capítulo responde las cuatro con un solo modelo.

## 20.3 Modelo mental

Piensa en una **cadena de montaje** al revés de lo que sueles imaginar: aquí **cada estación pide la pieza a la anterior solo cuando la necesita**. Nadie apila el almacén entero en la primera estación esperando a que la última empiece a trabajar.

```
        (el cliente: tu terminal)
              ▲
        ┌─────┴─────┐
        │  Project  │  "dame la siguiente fila"
        └─────┬─────┘
         next ▲ │ Row
        ┌─────┴─────┐
        │  Filter   │  "pásame otra; la evalúo y decide"
        └─────┬─────┘
         next ▲ │ Row
        ┌─────┴─────┐
        │  Expand   │  "por esta fila, sus aristas, una a una"
        └─────┬─────┘
         next ▲ │ Row
        ┌─────┴─────┐
        │ NodeScan  │  "el siguiente nodo del almacén (GraphStore)"
        └───────────┘
```

La consulta se evalúa en **pull** (Graefe lo llamaba *demand-driven*): la petición baja, la fila sube. Dos consecuencias inmediatas:

1. **La primera fila sale sin esperar a la última.** El `Project` ya puede imprimir mientras el `NodeScan` sigue recorriendo el grafo.
2. **Si la raíz deja de pedir, la cadena entera se detiene.** Eso ES un `Limit`: cuando ha emitido sus `max` filas, devuelve `None`, y nadie vuelve a pedirle nada al árbol de abajo.

El momento ¡ajá! de este capítulo: **una consulta no es un cálculo, es una negociación de peticiones**. «¿Tienes otra fila?» — «Sí, toma» — «¿Tienes otra?» — «No: agotado».

## 20.4 Primera solución

El novato escribe un intérprete recursivo que materializa cada operador en un `Vec`:

```rust
// Solución ingenua: cada operador produce TODAS sus filas de golpe.
fn eval_plan(plan: &LogicalPlan, store: &dyn GraphStore) -> Vec<Row> {
    match plan {
        LogicalPlan::NodeScan { .. } => /* iterar TODOS los nodos → Vec */,
        LogicalPlan::Filter { input, predicate } => {
            let rows = eval_plan(input, store);      // materializo la entrada
            rows.into_iter()                         // …y luego filtro
                .filter(|r| evalua(predicate, r))
                .collect()
        }
        /* …etc… */
    }
}
```

Sobre nuestro grafo demo (6 nodos, 6 aristas) funciona. Los tests pasan. Y durante un rato nadie se queja.

## 20.5 Sus límites

Hasta que el grafo tiene un millón de nodos y alguien escribe «dame una persona». Con el modelo materializador:

1. **`LIMIT 1` sobre 1M de filas calcula el millón.** El `NodeScan` llena un `Vec` con 1.000.000 de filas, el `Filter` produce otro `Vec`, el `Project` otro… y al final alguien se queda con la primera y tira 999.999. La latencia de la primera fila es igual al trabajo total.
2. **Memoria O(n) por operador intermedio.** Cada nivel del árbol duplica el resultado en memoria.
3. **Un error a mitad deja basura construida.** Si el `Filter` revienta en la fila 3, los `Vec` gigantes ya existen; y nadie «cierra» nada, porque no hay nada que cerrar: el ciclo de vida no existe.
4. **`RETURN p` no cabe.** `Vec<Vec<Value>>` no puede contener un *nodo entero*; necesitamos que la celda de una fila sea un escalar, un nodo o una arista.

La raíz del problema es la misma de siempre: **mezclar dos decisiones que deben ir separadas** — *qué* produce cada operador (semántica) y *cuándo* lo produce (estrategia). El modelo Volcano las separa de raíz.

## 20.6 Solución evolucionada

### El contrato: el trait `PhysicalOperator`

```rust
pub trait PhysicalOperator {
    fn open(&mut self) -> Result<(), ExecError>;
    fn next(&mut self) -> Result<Option<Row>, ExecError>;
    fn close(&mut self) -> Result<(), ExecError>;
    fn name(&self) -> &'static str;
    fn rows_produced(&self) -> u64;
    fn collect_metrics(&self) -> Vec<(&'static str, u64)> { /*…*/ }
}
```

¿Por qué la tríada y no solo `next`? **`open`** prepara y resetea: posiciona el cursor del scan, materializa lo imprescindible (veremos el caso del cartesiano), y deja el operador listo para re-ejecutarse tras un `close` (testeado: `nodescan_ciclo_open_close_reopen`). **`close`** libera y se propaga a los hijos, y es idempotente. Y lo más importante: el `Executor` cierra **siempre**, también tras error — como un `defer`. Si el consumidor aborta en la fila 3, quien limpia es el `close` del ciclo, no la suerte.

### La moneda común: `Row` y `Cell`

```rust
pub enum Cell {
    Scalar(Value),   // reutiliza el Value del cap. 7
    Node(Node),      // RETURN p → el nodo entero
    Edge(Edge),      // RETURN r → la arista entera
}

pub struct Row { entries: Vec<(String, Cell)> }
```

`Row` es la materialización en ejecución de los `Bindings` del cap. 19: el scan **liga** (`row.bind("p", Cell::Node(nodo))`), el `Expand` **extiende** (clona la fila y liga relación y destino), el `CartesianProduct` **concatena** dos filas de patrones disjuntos (`merge`), y el `Project` produce la fila de salida re-ligando cada `Projection` a su `output_name()`. ¿Por qué un `Vec<(String, Cell)>` y no un array posicional? Porque el nombre viaja con la celda: el binder del cap. 19 ya validó las variables, y aquí solo buscamos por nombre (`row.get("p")`); además `RETURN p, p` produce dos columnas con el mismo nombre y ambas se conservan. Con un único tipo de fila, el trait queda uniforme — no hacen falta dos jerarquías (filas de bindings vs filas de salida).

### Los ocho operadores, cada uno con su porqué

- **`NodeScanOp`** — la hoja: su cursor es un iterador perezoso sobre `GraphStore::iter_nodes` (cap. 8) que se posiciona en `open()`. El orden es el del store: determinista, requisito para tests. Un detalle fino de Rust: en `open()` copiamos la referencia (`let store = self.store;`) antes de guardar el iterador, para que el préstamo del cursor viva tanto como el store y no como el `&mut self` del método.
- **`IndexSeekOp`** — liga exactamente los `NodeId` que recibe. La gracia es NO escanear; pero **la selección del índice no es cosa suya**: quien lo construye ya resolvió la búsqueda (con un índice del cap. 15). Elegir este operador en vez del scan es trabajo del optimizador (cap. 21). Si un ID no existe, el índice está desactualizado: `ExecError::UnknownNode`.
- **`ExpandOp`** — el bucle anidado clásico: por cada fila del input (bucle externo), recorre sus aristas candidatas por dirección (bucle interno) usando `out_edges`/`in_edges` como índice de adyacencia. UNDIRECTED recorre out+in y cuenta el self-loop UNA vez (Dani→Dani aparece una sola vez).
- **`FilterOp`** — deja pasar las filas cuyo predicado evalúa a TRUE. **FALSE y NULL se descartan** — y aquí está la diferencia sutil: NULL no es false, es *desconocido*. Por eso `WHERE p.missing > 30` saca 0 filas… y `WHERE NOT p.missing > 30` también. Además, un predicado no booleano (`WHERE p.age` con `age` INT) es un `TypeMismatch` **en ejecución**: el plan del cap. 19 solo pudo tiparlo como `Any` (schemaless); aquí se concreta.
- **`ProjectOp`** — la única operación que cambia de forma: evalúa cada item del RETURN sobre la fila interna y produce la fila de salida.
- **`CartesianProductOp`** — cada fila izquierda × cada fila derecha. Y aquí, la lección más honesta del capítulo: **materializa el lado derecho completo en `open()`**. Volcano es monotónico: un operador no puede «rebobinar» su input, y el producto necesita re-leer el lado derecho por cada fila de la izquierda. Ese coste (memoria + filas de más antes de cualquier filtro) es exactamente el «antes» que el optimizador del cap. 21 eliminará reordenando el punto de partida. No lo escondemos: lo numeramos con métricas.
- **`LimitOp`** — emite como máximo `max` filas y se agota. En un pipeline pull esto corta la ejecución **de verdad**: si es la raíz, nadie pide más filas al árbol de abajo.
- **`DistinctOp`** — descarta repetidas con búsqueda lineal en un `Vec` (deliberadamente simple: las celdas contienen `f64`, no hasheables; una versión real usaría una firma hasheable por fila).

`LimitOp` y `DistinctOp` son operadores de pleno derecho aunque la gramática LiraQL (caps. 17-18) aún no exponga las keywords: se componen programáticamente hasta que el lenguaje las admita.

### `eval_scalar`: SQL/Cypher, no Rust

Las reglas son las del estándar de facto (SQL ISO y openCypher), por una razón simple: es la semántica que cualquier usuario de bases de datos espera, y en schemaless la propiedad ausente **tiene que** ser NULL, no false ni un error:

- `p.name` ausente → `Value::Null`; `f:Person` sobre una arista → `Null` (las aristas no tienen labels: desconocido, no falso).
- **NULL domina las comparaciones**: `Null = x` → `Null`; `p.missing > 30` → `Null`.
- Igualdad numérica Int/Float con promoción (`1 = 1.0` → true); tipos distintos no son iguales (`1 = "1"` → false, estilo Cypher) pero **sin orden** (`1 < "a"` → Null); solo números y cadenas se ordenan — espejo de `order_compatible` del cap. 19.
- **Igualdad de nodos por IDENTIDAD de id**: `WHERE a = b` es el predicado «mismo nodo». Con igualdad de valor (comparar propiedades) dos nodos distintos con las mismas props serían «iguales»: mentir. Con identidad, `(a)-[:KNOWS]->(b) WHERE a = b` encuentra self-loops — exactamente el test `hito_self_loop_con_igualdad_de_nodos`, que devuelve a Dani.
- **AND/OR/NOT trivalentes con cortocircuito real**: `FALSE AND x` devuelve FALSE sin evaluar `x`; `TRUE OR x` devuelve TRUE sin evaluar `x`. ¿Y cómo sabemos que la rama elidida de verdad no se evalúa? Porque es **observable**: `TRUE AND p.age` (con `age` INT) da `TypeMismatch`, pero `FALSE AND p.age` devuelve FALSE sin error. La rama que habría errado, no se ejecutó. Test: `eval_cortocircuito_real`. Es la promesa de los caps. 17 y 19, cumplida y verificada.

### `compile`, `Executor` y el hito

`compile(plan, store)` traduce el `LogicalPlan` a su árbol de operadores **1:1, sin reescrituras** — deliberado: el cap. 21 insertará ahí el push-down de filtros, la conversión `NodeScan`→`IndexSeek` y la reordenación. El `Filter` alto que produce `lower()` se ejecuta tal cual, y las métricas dejan ver su ineficiencia: el mejor anuncio del optimizador.

El `Executor` impone el ciclo sagrado:

```rust
pub fn execute(&mut self) -> Result<ResultSet, ExecError> {
    self.root.open()?;
    let drained = loop {
        match self.root.next() {
            Ok(Some(row)) => rows.push(row.cells()),
            Ok(None) => break Ok(()),
            Err(e) => break Err(e),
        }
    };
    self.root.close()?;   // close SIEMPRE (incluso tras error): como un defer
    drained?;
    /* … ResultSet … */
}
```

Fíjate en el orden: el error se **guarda** (`drained`), se cierra, y solo después se propaga. Y fíjate en `Executor::new`: exige un `Project` raíz (`NotAProjection` si no), porque las columnas del `ResultSet` salen de sus `Projection::output_name()` — la invariante que `lower()` ya garantizaba.

Todo el motor va contra `&dyn GraphStore` — el puerto hexagonal del cap. 8. Hoy enchufas un `MemoryStore`; mañana, el store en disco de la Parte III **sin tocar una línea del motor**.

## 20.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap20_volcano.rs` (2.446 líneas, 37 tests en `tests_executor`). Las piezas que acabas de leer son esas; aquí solo el desenlace. La API pública tiene tres niveles:

```rust
// Nivel 1: operadores programáticos (componer a mano, como los tests de Limit).
let op = LimitOp::new(compile(&plan, &store)?, 2);

// Nivel 2: el Executor con su ciclo y sus métricas.
let mut exec = Executor::new(&plan, &store)?;
let rs = exec.execute()?;
let m = exec.metrics();   // ExecMetrics en pre-orden: raíz → hojas

// Nivel 3: EL HITO — texto a filas.
let rs = run("MATCH (p:Person)-[:KNOWS]->(f:Person) \
              WHERE p.name = \"Ana\" RETURN f.name", &store)?;
```

`run(src, store)` es `parse` (cap. 18) + `Query::execute` (lower + Executor). Y el grafo de las demos es `demo_graph()`: Ana(36), Bo(41), Carla(29), Dani(36), Madrid y Lisboa; KNOWS en triángulo + el self-loop de Dani, y LIVES_IN — el mismo fixture de los tests, promovido a API pública para que la CLI no duplique el dato.

## 20.8 Prueba de fuego — el hito

Este es el momento que llevamos nueve capítulos construyendo. Ejecuta:

```
$ cargo run -p liradb-cli -- query "MATCH (p:Person) WHERE p.age < 40 RETURN p.name, p.age"
p.name  | p.age
"Ana"   | 36
"Carla" | 29
"Dani"  | 36
```

Una cadena de texto entró; una tabla salió. Sin escribir Rust. (La CLI mínima del hito ADR-005 corre sobre `demo_graph()`; en el workspace final `Query::execute` pasa además por el optimizador del cap. 21 — llega en el próximo capítulo, y los resultados son equivalentes.)

Y con `liradb demo`, la consulta canónica del brief muestra el pipeline entero con sus métricas reales:

```
LiraQL: MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name, r.since
Plan lógico:
Project(f.name, r.since)
  Filter(f:Person AND p.name = "Ana")
    Expand(p, r:KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
Resultado:
f.name | r.since
"Bo"   | 2020
Métricas: Project: 1 filas
Filter: 1 filas
Expand: 4 filas
NodeScan: 4 filas
filas devueltas: 1
```

Detente en las métricas: **NodeScan produjo 4 filas y Expand 4 para que el Filter devolviera 1**. Nadie mintió: contamos lo que de verdad fluyó. Esa es la semilla del `explain` del cap. 21 — el eco del `hit_ratio` del cap. 13: métricas que observan mientras el sistema trabaja, no suposiciones. Los 37 tests del módulo (de `row_bind_get_merge_y_display` a `result_set_display_tabla_y_column`) cubren desde las tablas trivalentes hasta el camino de dos tramos con anónimo intermedio; si te saltaras este capítulo, tu síntoma sería evidente: tendrías planes bonitos que no producen ninguna fila, y WHEREs con propiedades ausentes devolviendo resultados falsos.

## 20.9 Qué hemos sacrificado

1. **El cartesiano materializa el lado derecho.** Precio del modelo monotónico; el cap. 21 lo evita reordenando, no lo esconde.
2. **Una fila cada llamada (tuple-at-a-time).** El modelo Volcano clásico paga overhead de llamada por fila; los motores vectorizados (cap. 38) procesan lotes. Lo mantenemos así porque la claridad didáctica del iterador puro es el objetivo aquí.
3. **`DistinctOp` es O(n²) en filas vistas** (búsqueda lineal; las celdas con `f64` no son hasheables tal cual).
4. **Sin paralelismo.** El `exchange` de Volcano'89 y los morsels de DuckDB quedan nombrados; nuestro motor es single-threaded.
5. **Sin ORDER BY, el orden de las filas no es parte del contrato** — y desde que el cap. 21 reordene planes, no podrás depender de él. Los tests comparan ordenado cuando toca.

## 20.10 Cómo lo hace una BBDD real

- **PostgreSQL** ejecuta árboles de `PlanState` con el ciclo `ExecutorStart`/`ExecutorRun`/`ExecutorEnd`: pull fila a fila, el mismo modelo con otros nombres.
- **SQL Server** desciende de Graefe en las dos mitades: ejecución iteradora (Volcano) y optimizador (Cascades, su framework de 1995).
- **MonetDB/X100** (Boncz, Zukowski, Nes; CIDR 2005) midió el costo de tuple-at-a-time y lo resolvió con **ejecución vectorizada**: cada `next` devuelve un lote (vector) de columnas. Es el puente directo al cap. 38.
- **Kùzu** combina almacenamiento columnar con operadores vectorizados y pipelines morsel-driven: el modelo Volcano «por lotes» y en paralelo.
- **DuckDB** empuja (push-based) los datos por pipelines con morsel-driven parallelism (Leis et al., CIDR 2014): cuando la raíz es un agregado, empujar evita llamadas de función equivalentes al pull.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en `liradb demo`, ¿qué operador produce filas que nadie consume en la consulta canónica? ¿Cuántas?
- *Intermedio*: implementa `OffsetOp { skip }` al estilo de `LimitOp`, con `collect_metrics`, y predice sus métricas antes de ejecutarlo.
- *Experto*: escribe `run_materializando(plan, store)` (el intérprete de §20.4) y demuestra la equivalencia con el `Executor` en 4 consultas; luego mide, con métricas, `Limit(1)` sobre un grafo de 1.000 personas: Volcano escanea 1, el materializador 1.000.

## 20.11 Lo que te llevas

- **Pull, no cálculo**: la consulta se va pidiendo; `Limit` corto-circuita el pipeline de raíz.
- **La tríada `open`/`next`/`close`**, con `close` SIEMPRE — incluso en error. Quien aborta no limpia: limpia el ciclo.
- **Un trait, ocho structs componibles**: el plan del cap. 19 se traduce 1:1 a un árbol que se enchufa.
- **`Row = Vec<(String, Cell)>`**: los `Bindings` del cap. 19 materializados; `Cell` resuelve `RETURN p` sin magia.
- **NULL SQL/Cypher con lógica trivalente y cortocircuito observable**; igualdad de nodos por identidad.
- **El cartesiano materializa porque Volcano no rebobina** — el coste que motiva el cap. 21.
- **Métricas reales por operador**: `NodeScan: 4 | Filter: 1` es evidencia, no opinión.
- **El hito**: `run(src, store)` y `liradb query` ejecutan consultas completas desde texto.

## 20.12 Ojo, cuidado con…

- **Llamar `next` antes de `open` (o tras `close`)**: el contrato dice «agotado en silencio», no «reabre». Re-ejecutar es `open` de nuevo.
- **Tratar NULL como false**: en `Filter` ambos quedan fuera, pero `NOT NULL` es NULL. «Desconocido» no es «falso».
- **Esperar rebobinar**: un operador agotado está agotado. Si necesitas releer, materializa (como el cartesiano) o re-abre.
- **Comparar nodos por valor**: la igualdad es identidad de id. Dos Anas distintas son dos nodos.

## 20.13 Pin de batalla

> *«Si tu motor calcula la respuesta antes de que nadie la haya pedido, no tienes un motor de consultas: tienes un generador de resultados que ignora cuánta respuesta se necesita.»*

## 20.14 Si solo lees 30 segundos

Cada operador es un iterador con `open`/`next`/`close`; la consulta se evalúa en pull (la raíz pide, la cadena responde) de modo que `Limit` corta de raíz y la primera fila sale sin esperar a la última. Las filas son variables ligadas a `Cell`s (escalar, nodo o arista); WHERE usa lógica trivalente (NULL se descarta, pero no es false) con cortocircuito real. El cartesiano materializa su lado derecho porque nadie rebobina. Y `run(texto, store)` ejecuta consultas completas: el hito del libro.

## 20.15 Una historia pequeña

La primera versión del executor de LiraDB no tenía `close`. Funcionaba, pasa los tests, fin. Hasta que una consulta con `WHERE p.age` (INT usado como booleano) reventó a mitad del pipeline en un script largo, y el cursor del `NodeScanOp` —un iterador que pide prestado el store— quedó vivo junto con media ejecución huérfana. El borrow checker no dijo nada: el préstamo era legítimo. Lo que faltaba no era memoria, era **protocolo**. El `close` SIEMPRE del `Executor` nació esa tarde: el error se guarda, se cierra, y solo entonces se propaga. Desde entonces, ejecutar una consulta que falla y otra que no deja el motor exactamente igual de limpio. Lo que no se cierra, se filtra.

## Ejercicios resueltos

**1. `LIMIT 1` sobre un scan de 1M de filas: ¿cuántas produce el `NodeScan` en cada modelo?**

En Volcano, **1**. El `LimitOp` pide una fila al `Project`, que pide al input… la fila llega, el `Limit` la emite, y como ya alcanzó su máximo, su siguiente `next` devuelve `None` sin pedir nada más a nadie: el pull se cortó de raíz. En el materializador, **1.000.000**: el scan llena su `Vec` entero antes de que exista ningún consumidor. Puedes ver la versión pequeña en el test `limit_corta_el_pipeline`: con `LimitOp::new(scan, 2)`, las métricas dicen `("Limit", 2), ("Project", 2), ("NodeScan", 2)` — el scan solo produjo lo pedido.

**2. Nadie tiene la propiedad `nick`. ¿Qué devuelven `WHERE p.nick = "anita"` y `WHERE NOT p.nick = "anita"`?**

Ambas devuelven **0 filas**. La primera: `p.nick` es NULL (propiedad ausente en schemaless), `NULL = "anita"` es NULL, y el `Filter` solo pasa TRUE. La segunda: `NOT NULL` sigue siendo NULL — negar un desconocido no lo convierte en conocido. Es la diferencia entre trivalente y booleano a secas, y está testeada en `hito_where_con_null_no_pasa_nada`.

## Ejercicios propuestos

**Esencial (retrieval).** Cierra el libro y el editor. De memoria: escribe el trait `PhysicalOperator` completo (las cinco firmas) y responde: si el consumidor aborta con error tras un `next`, ¿quién limpia y por qué? Luego ábrelo y corrige. Verifica tu respuesta compilando un test que drene un `NodeScanOp` con el ciclo completo y lo re-ejecute tras `close`.

**Intermedio (spacing + interleaving).** Implementa `OffsetOp { skip: usize }` (salta `skip` filas, luego emite el resto) componiéndolo a mano sobre `compile()`, como hacen los tests de `LimitOp`. Antes de ejecutarlo, **predice por escrito** sus `collect_metrics` para `MATCH (p:Person) RETURN p.name` con `skip = 2` sobre el grafo demo. ¿Por qué tu operador NO aparece en `compile()`? (Pista: ¿qué capítulo decide qué operadores físicos existen?)

**Experto (crear).** Escribe `run_materializando(plan, store) -> ResultSet`: el intérprete recursivo de §20.4, sin el trait, materializando cada operador. Demuestra con un test que devuelve lo mismo que el `Executor` (columnas y filas ordenadas) en 4 consultas del grafo demo. Luego genera un store con 1.000 `Person`, envuelve ambas rutas con un límite de 1 fila, y compara las filas escaneadas. Explica por qué en TU versión el cartesiano no necesita materializar el lado derecho.

## Para profundizar

- **Graefe, «Volcano — An Extensible and Parallel Query Evaluation System» (IEEE TKDE, 1994)** — el paper original del modelo que acabas de implementar.
- **Graefe, «Encapsulation of Parallelism in the Volcano Query Processing System» (SIGMOD 1989)** — el operador `exchange`: paralelismo detrás de la misma interfaz iteradora.
- **Graefe, «The Cascades Framework for Query Optimization» (IEEE DE Bulletin, 1995)** — la otra mitad de la herencia: el optimizador de SQL Server.
- **CMU 15-445, notas de Query Execution I** — el iterator model frente a materialization y vectorization, con la terminología que usarás en el cap. 38.
- **Boncz, Zukowski, Nes, «MonetDB/X100: Hyper-Pipelining Query Execution» (CIDR 2005)** — por qué tuple-at-a-time duele y cómo se vectoriza.
- **Raasveldt y Mierle, «DuckDB: an Embeddable Analytical Database» (SIGMOD 2020)** — pipelines push-based y morsel-driven parallelism.

## Mini-diálogo: en guardia nocturna

> — Entonces el motor entero es… ¿un montón de structs con el mismo trait de tres métodos?
>
> — Y una disciplina: quien abre, cierra. Aunque la consulta reviente a mitad.
>
> — Pero si materializar todo funciona en el grafo de seis nodos…
>
> — Todo funciona con seis nodos. El modelo se elige para el día en que son un millón y alguien pide una sola fila. Ese día, el pull te salva y la materialización te hunde. Y ojo: el cartesiano ya te mostró el precio de no poder rebobinar.
>
> — ¿Y las métricas esas de «NodeScan: 4 filas»?
>
> — La prueba de que el motor te cuenta la verdad. El próximo capítulo las usará para no escanear 4 cuando basta 1. Hoy ejecutamos; mañana, ejecutamos bien.

---

*(Próximo capítulo: 21 — Un optimizador pequeño pero real. Las métricas de este capítulo numeraron el problema (4 filas escaneadas para devolver 1); ahora construiremos quién lo arregla: `optimize` con estadísticas y reglas, visible en `liradb explain`.)*
# Capítulo 21 — Un optimizador pequeño pero real (`liradb explain`)

> *«El usuario dice QUÉ quiere. El motor decide CÓMO conseguirlo. Esa frontera tiene nombre: optimizador.»*

## 21.0 La anécdota de la esquina

En 1979, en el laboratorio de IBM de San José, Patricia Selinger y su equipo (Astrahan, Chamberlin, Lorie y Price) publicaron en SIGMOD un paper con un título tan sobrio como su contenido era revolucionario: «Access Path Selection in a Relational Database Management System». Describe la pieza que le faltaba a System R —la base de datos que estaba definiendo cómo sería SQL— para no ser un juguete lento: **el primer optimizador basado en coste**.

El paper es famoso por la enumeración dinámica de órdenes de joins. Pero dos detalles suyos te van a sonar muchísimo dentro de unas páginas. Primero: cuando no había estadísticas, System R asumía que una igualdad (`col = valor`) deja pasar **una décima parte** de las filas y un rango (`col < valor`), **un tercio**. Exactamente los `0.1` y `1/3` que usarás hoy en LiraDB. Segundo: entre sus heurísticas explícitas estaba «anidar los predicados lo más profundamente posible en el árbol de consulta» — lo que hoy llamamos **predicate pushdown**. Cuarenta y tantos años después, esa sigue siendo la regla número 1 del optimizador de cualquier motor que puedas nombrar, del PostgreSQL de tu servidor al Catalyst de Spark.

Y la herramienta para VERLO también tiene historia: `EXPLAIN` existe desde los orígenes de PostgreSQL en Berkeley, y la variante que además ejecuta la consulta para contrastar —`EXPLAIN ANALYZE`, «que muestra tiempos y recuentos de filas», según las notas de la 7.2.0, febrero de 2002— es el antepasado directo del `liradb explain` que construiremos hoy: plan ANTES, plan DESPUÉS, y las filas reales al final para comprobar cuánto mienten las estimaciones.

## 21.1 Objetivo

Al terminar este capítulo sabrás **por qué un motor que ya parsea, planifica y ejecuta (caps. 17-20) todavía deja la mitad del trabajo en la mesa**, y habrás construido la pieza que lo reclama: un **optimizador** — un programa que reescribe el plan lógico antes de ejecutarlo para que haga el mismo trabajo con menos esfuerzo.

Tres piezas, las tres en `cap21_optimizador.rs`:

1. **El catálogo** (`Catalog`) — estadísticas recolectadas del `GraphStore`: nodos por etiqueta, grados medios out/in, aristas por tipo, y un índice de igualdad.
2. **La estimación de cardinalidad** (`estimate`) — heurísticas simples y documentadas para adivinar cuántas filas producirá cada operador.
3. **Las cinco reglas** (`optimize`) — reescrituras en orden fijo que transforman el plan ingenuo del cap. 19 en el plan que ejecuta el Volcano del cap. 20.

Y el hito: `liradb explain "..."`, que enseña el antes y el después con estimaciones y filas reales.

## 21.2 Problema

Ejecuta el demo del capítulo anterior y mira la última consulta, la canónica del brief:

```text
LiraQL: MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name, r.since
Plan lógico:
Project(f.name, r.since)
  Filter(f:Person AND p.name = "Ana")
    Expand(p, r:KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
Métricas: Project: 1 filas
Filter: 1 filas
Expand: 4 filas
NodeScan: 4 filas
filas devueltas: 1
```

Lee las métricas de abajo arriba: el escaneo produce 4 filas, la expansión produce 4, y el filtro… devuelve 1. **Escaneamos 4 para devolver 1.** Con 6 nodos en el grafo demo eso es una anécdota; con 10 millones de personas es un delito. Y lo peor: el plan ni se inmuta, porque es exactamente lo que `lower()` (cap. 19) le pidió — pusimos el `Filter` encima de todo y el cap. 20 lo ejecutó fielmente. Las métricas del cap. 20 ya numeraban esta ineficiencia a propósito: son el mejor anuncio del optimizador.

La pregunta del capítulo: ¿quién decide que, en vez de escanear todas las personas y filtrar Ana al final, empieces directamente POR Ana?

## 21.3 Modelo mental

Piensa en el **planificador de rutas de un GPS**. Tú dictas el destino: «de mi casa al aeropuerto». Eso es la consulta — intocable. El GPS elige el ORDEN de las calles: hoy la M-30 está colapsada (lo sabe porque MIRA el tráfico), así que va por la alternativa. Mismo destino, misma llegada, distinto camino. Y si mañana el tráfico cambia, el camino cambia — pero tu destino no.

Traduce: la consulta del usuario es el destino; el plan lógico es la lista de calles; las estadísticas del catálogo son el tráfico; y el optimizador es el planificador. Tres consecuencias que vertebran todo el capítulo:

1. **El GPS nunca te cambia el destino** — el optimizador nunca cambia los resultados (lo probaremos con tests de equivalencia).
2. **El GPS necesita mirar el tráfico** — sin estadísticas del grafo no hay decisión posible, sólo manías.
3. **Convenir un orden de cálculo fijo** — el GPS evalúa primero autopistas, luego avenidas, luego calles: reglas en orden conocido, mismo input → mismo output.

```
        consulta (destino, del usuario)
                   │
        ┌──────────▼──────────┐
        │   OPTIMIZADOR       │  mira el catálogo (tráfico):
        │  5 reglas fijas     │  Person: 4 · out 1.50 / in 1.00
        │  R1..R5             │  KNOWS: 4 de 6 aristas
        └──────────┬──────────┘
     plan ANTES ──►│──► plan DESPUÉS   (mismo resultado, menos trabajo)
```

El momento ¡ajá!: hasta ahora, el plan lógico decía QUÉ hacer y el motor lo obedecía al pie de la letra. Hoy descubres que un mismo QUÉ admite muchos órdenes de cálculo equivalentes, y que elegir bien entre ellos exige **mirar los datos**.

## 21.4 Primera solución

La versión ingenua ya la tienes: es no tener optimizador. El plan de `lower` baja tal cual a `compile` (cap. 20, que a propósito era 1:1) y se ejecuta como llegó. Si el usuario quiere velocidad, que escriba mejor la consulta — o que el código que llama al motor construya el plan a mano y use `IndexSeek` directamente, que el operador existe desde el cap. 20.

Y una segunda versión ingenua, más tentadora: «pues que el motor reescriba la CONSULTA» — que detecte `WHERE p.name = "Ana"` y la convierta en otra consulta mejor escrita antes de planificarla.

## 21.5 Sus límites

Ambas se rompen en cuanto te alejas del juguete:

1. **El AST es del usuario.** Reescribir su consulta rompe el contrato de fidelidad texto→AST que construimos en los caps. 17-18: los errores dejan de apuntar a SUS palabras, y cualquier reescritura del texto exige re-parsear, re-ligar y re-verificar. El plan, en cambio, es nuestra representación interna: reordenar ligaduras ahí es invisible para él. Optimizamos el plan, no la consulta.
2. **La combinatoria se come la búsqueda exhaustiva.** Elegir «el mejor plan» enumerando todos es caro: con n joins hay n! órdenes (10 joins = 3.628.800). System R ya lo sabía: por eso enumeraba con programación dinámica y además recortaba a árboles left-deep. Nosotros, con reglas totales en orden fijo, pagamos cero combinatoria.
3. **El 4-para-1 escala linealmente.** Extrapolalo: si escaneas 4 filas para devolver 1 en un grafo de 40 millones de nodos con ese ratio, mueves ~160 millones de filas para responder con 40. El plan ingenuo no empeora con los datos — empeora CONTIGO, multiplicando tu muestreo.
4. **El caller con `IndexSeek` a mano no tiene datos.** ¿Merece la pena el índice? Depende de cuántos ids devuelva frente a cuántas filas escanea el `NodeScan` — información que vive en el grafo, no en quien llama.

La conclusión: necesitamos una pieza nueva que (a) reescriba planes, no consultas; (b) mire el grafo antes de decidir; (c) garantice los mismos resultados. Ese es el optimizador.

## 21.6 Solución evolucionada, parte 1: el catálogo (o mirar el tráfico)

Antes de elegir ruta hay que saber cómo está el tráfico. `Catalog::collect(&dyn GraphStore)` hace UNA pasada por el store y recolecta: nodos totales, nodos por etiqueta, aristas por tipo, grados saliente/entrante acumulados por etiqueta de los extremos, y un índice de igualdad `(etiqueta, propiedad, valor) → ids`:

```text
Catálogo (estadísticas del store): 6 nodos · 6 aristas
  Person: 4 nodos · grado medio out 1.50 / in 1.00
  City: 2 nodos · grado medio out 0.00 / in 1.00
  aristas por tipo: KNOWS 4, LIVES_IN 2
```

Eso no es una salida inventada: es literalmente lo que imprime `liradb explain` sobre el grafo demo. Y fíjate en el porqué de cada número:

- **¿Por qué grados medios POR ETIQUETA y no globales?** Porque el coste de un `Expand` desde `f` depende de cómo son las Person, no de cómo es el grafo en general. Una etiqueta hub (grado 500) y una hoja (grado 1) no pueden compartir media.
- **¿Por qué un índice de igualdad si el cap. 15 ya construyó índices?** El `HashIndex`/`BPlusTree` del cap. 15 viven en el fichero en disco; este catálogo es su primo en memoria, reconstruido por consulta. En un sistema real el catálogo persistiría y se mantendría incrementalmente (esa es exactamente la infraestructura natural del cap. 15); aquí reconstruirlo cuesta un escaneo y nos da un catálogo obviamente correcto del que razonar. Los índices del cap. 15, por fin, tienen alguien que decide cuándo usarlos.
- **¿Por qué constantes NO mágicas?** Porque sin mirar el grafo no hay decisión posible: la regla R1 compara costes reales (1.50 de grado out de Person, fracción 4/6 de KNOWS). Con constantes a ciegas, «optimizador» sería un nombre bonito para una manía.

## 21.7 Solución evolucionada, parte 2: estimar cardinalidad (la sección de estadísticas)

El catálogo dice cómo es el grafo; la **estimación** traduce eso a «cuántas filas producirá este operador». La función `estimate` es un pequeño `match` recursivo con fórmulas que caben en una servilleta:

```text
  NodeScan         → nodos con la etiqueta (todos si ANY)
  IndexSeek        → ids resueltos (exacto)
  Filter           → entrada × selectividad del predicado
  Expand           → entrada × grado medio de la dirección × fracción del tipo
  Project          → lo que produce su entrada
  CartesianProduct → izquierda × derecha
```

Y la **selectividad** de un predicado (la fracción de filas que se espera que sobrevivan) usa los defaults de 1979 cuando no hay estadística, y la estadística cuando la hay:

```rust
pub const SEL_EQ: f64 = 0.1;        // igualdad sin estadística (System R)
pub const SEL_RANGE: f64 = 1.0 / 3.0; // rango <, <=, >, >= (System R)
pub const SEL_NOT_EQ: f64 = 0.9;
pub const SEL_UNKNOWN: f64 = 0.5;
```

Con `AND` se multiplican (independencia), `OR` usa inclusión-exclusión, `NOT` complementa. Y el caso estrella: si el predicado es `v.prop = literal` y el índice de igualdad del catálogo conoce esa clave, la selectividad es EXACTA — `ids / nodos de la etiqueta`. Si el valor no ocurre (buscas a «Zoe» y no existe), la selectividad es 0.0: no filtras, aniquilas.

Apliquémoslo al plan ANTES del problema del §21.2, con el catálogo real:

```text
NodeScan(Person AS p)        → 4                      (hay 4 Persons)
Expand(p, KNOWS, OUT, f)     → 4 × 1.5 × (4/6) = 4    (grado × fracción de tipo)
Filter(f:Person ∧ f.age<40)  → 4 × 1.0 × 1/3 = 1.33   → est. 1
```

**¿Por qué heurísticas simples y no muestreo?** Porque la estimación sólo necesita **ordenar planes**, no prometer costes: para elegir entre empezar por `p` o por `f` basta saber cuál es más barato, con error del 50 % incluido. Un histograma o un muestreo serían infraestructura desproporcionada para comparar dos candidatos — y nos robarían el momento pedagógico de ver, con números, cuánto mienten las heurísticas. Porque mienten: en el demo, 3 de 4 personas tienen `age < 40` (selectividad real 0.75, no 1/3). Ya volveremos a ello: esa discrepancia es contenido, no bug.

## 21.8 Solución evolucionada, parte 3: las cinco reglas

`optimize(plan, &catalog)` aplica cinco reescrituras **en orden fijo**:

| # | Regla | Qué hace |
|---|---|---|
| R1 | `rule_selective_start` | Elige la variable más selectiva como punto inicial y reordena la cadena de `Expand` (los tramos a su izquierda se recorren con la dirección invertida). Es el «join ordering» de los grafos. |
| R2 | `rule_predicate_pushdown` | Parte los `AND` en átomos y baja cada átomo lo más profundo posible — sin cruzar variables que aún no están ligadas. |
| R3 | `rule_absorb_label` | El `HasLabel` del nodo escaneado se integra en la etiqueta del `NodeScan` (que filtra al escanear). |
| R4 | `rule_index_seek` | `Filter(v.prop = literal) + NodeScan` → `IndexSeek` con los ids del catálogo — **sólo si ahorra**: si el índice devolviera tantas filas como el escaneo, se queda el scan. |
| R5 | `rule_prune_projections` | Elimina proyecciones de identidad redundantes. |

**¿Por qué R2 —el predicate pushdown— es LA regla reina?** Números: en el demo pagamos 4 filas escaneadas para devolver 1. El `Filter` de la edad está ENCIMA del `Expand`, así que el motor expande cada candidata y luego la tira. Bájalo al escaneo y las filas que no cumplen `age < 40` ni siquiera entran al pipeline: no se expanden, no se filtran, no existen. Filtrar antes de expandir convierte trabajo proporcional al grafo en trabajo proporcional al resultado. Es la misma regla que Selinger escribió en 1979 y que hoy ejecuta tu PostgreSQL cada vez que lanza una query.

**¿Por qué respetando bindings?** Esta es la trampa. Bajar un átomo que menciona una variable que ahí abajo aún NO está ligada no optimiza: cambia la semántica (el runtime evaluaría la propiedad contra otra fila, o no la encontraría). La implementación lo respeta con la misma herramienta del cap. 19: `sink` consulta `bound_variables()` del subárbol y sólo hunde lo que menciona variables ya ligadas. Un `Filter(p.age > 30)` sobre un `Expand(f → p)` se queda donde está: `p` la liga la expansión. Un optimizador que cambia resultados no es un optimizador: es un bug con buen marketing.

**¿Por qué orden fijo y documentado?** Determinismo didáctico: mismo input → mismo output, siempre. R1 corre primero porque decide la FORMA de la cadena (por dónde empezar a ligar); R2 cuelga los predicados del plan resultante; R3 y R4 pulen el escaneo que quedó abajo; R5 barre. Con reglas iteradas hasta fijación o en orden arbitrario, los planes serían irreproducibles — imposibles de enseñar, de testear y de explicar en un `explain`. El test `optimizar_es_idempotente_y_conservador` además verifica que las reglas convergen: optimizar dos veces da lo mismo que una.

## 21.9 El hito: `liradb explain`

Todo junto, ejecutado de verdad en el workspace:

```console
$ cargo run -q -p liradb-cli -- explain \
    "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name"

liradb explain — optimizador (cap. 21)
Consulta: MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name

Catálogo (estadísticas del store): 6 nodos · 6 aristas
  Person: 4 nodos · grado medio out 1.50 / in 1.00
  City: 2 nodos · grado medio out 0.00 / in 1.00
  aristas por tipo: KNOWS 4, LIVES_IN 2

Plan ANTES (lower, cap. 19):
Project(p.name, f.name)            est. 1 filas
  Filter(f:Person AND f.age < 40)  est. 1 filas
    Expand(p, KNOWS, OUTGOING, f)  est. 4 filas
      NodeScan(Person AS p)        est. 4 filas

Plan DESPUÉS (optimize, cap. 21):
Project(p.name, f.name)          est. 1 filas
  Expand(f, KNOWS, INCOMING, p)  est. 1 filas
    Filter(f.age < 40)           est. 1 filas
      NodeScan(Person AS f)      est. 4 filas

Filas reales al ejecutar el plan optimizado: 3 (raíz estimada: 1)
```

Lee el DESPUÉS con calma, porque en cuatro líneas están las cinco reglas:

1. **R1**: la cadena ya no empieza por `p` sino por `f` — el coste estimado de empezar por f (`4 × 1/3 × 1.0 × 4/6 ≈ 0.89`) gana al de empezar por p (`4 × 1.5 × 4/6 = 4`). El tramo KNOWS se recorre en sentido INCOMING: dar la vuelta a la flecha es gratis, la semántica es la misma.
2. **R2**: el AND se partió — `f.age < 40` bajó y quedó PEGADO al escaneo de f.
3. **R3**: `f:Person` desapareció como filtro: se absorbió en `NodeScan(Person AS f)`.
4. **R4**/**R5**: aquí no aplican (no hay igualdad ni proyecciones sobrantes) — y el explain de la canónica del brief te muestra la R4 en acción: `Filter(p.name = "Ana") + NodeScan` se convierte en `IndexSeek(Person.name = "Ana")` que lee UN nodo en vez de cuatro.

Y la última línea es la lección más honesta del capítulo: **la raíz estimada era 1 y las filas reales son 3**. La heurística de rango (1/3) subestimó la selectividad real (3/4: Ana 36, Carla 29 y Dani 36 pasan; Bo 41 no). ¿Es un bug? No: la estimación cumplió SU trabajo — ordenar candidatos (f era más barato que p, y sigue siéndolo con 0.75) — y falló el que NO tenía (predecir filas). PostgreSQL vive de la misma tensión: por eso su `EXPLAIN ANALYZE` (7.2.0, 2002) muestra, como nosotros, estimadas y reales lado a lado. Cuando la discrepancia importa, se refinan las estadísticas (histogramas); cuando no, se agradece el orden correcto.

Fíjate también en qué NO cambió: las columnas y las filas. Tres antes, tres después, las mismas. Eso no es suerte: es un contrato testeado.

## 21.10 Prueba de fuego: equivalencia, o el GPS nunca te cambia el destino

La prueba de fuego de un optimizador no es «va más rápido»: es **«va más rápido Y devuelve exactamente lo mismo»**. El test `equivalencia_antes_y_despues_sobre_bateria_de_consultas` ejecuta 12 consultas (la canónica, filtros en ambos lados, dirección entrante, sin dirección, caminos de tres nodos, anónimos intermedios, cartesiano con filtros, propiedades inline y de arista, self-loops, etiqueta inexistente, OR) por los DOS caminos — plan ingenuo de `lower` y plan optimizado de `optimize` — y compara:

- **columnas**: idénticas;
- **filas**: idénticas como multiconjunto ORDENADO.

¿Por qué ordenadas? Porque sin `ORDER BY` (que LiraQL aún no tiene) el orden de las filas no es parte del contrato — exactamente como en SQL, y exactamente porque un optimizador que reordena ligaduras puede producir las filas en otro orden. Lo que se promete es el contenido, no la secuencia.

¿Y qué pasaría si ROMPIÉRAMOS la equivalencia? Imagina el pushdown mal hecho del §21.8: `f.age < 40` empujado por debajo del `Expand` que liga `f`. En el plan, el filtro se evaluaría para cada `p` contra una `f` que aún no existe. Síntoma: filas de menos (o de más) SIN error — la peor clase de bug, porque la consulta «funciona». Los tests de equivalencia son el detector: `ingenuo ≠ optimizado` en cualquier consulta es un fallo del test, no una curiosidad.

Desde este capítulo, `run()` y `Query::execute` pasan SIEMPRE por el optimizador (`lower` → `Catalog::collect` → `optimize` → `Executor`): el usuario escribe la consulta; el motor elige la ruta.

## 21.11 Repaso de la Parte IV: la cadena completa

Este capítulo cierra la Parte IV. Reconstruyamos la cadena que ya sabes construir, de izquierda a derecha:

```
 texto LiraQL ──parse──► AST ──lower──► LogicalPlan ──optimize──► LogicalPlan' ──compile──► operadores ──open/next/close──► filas
   (cap. 17:      (cap. 18:       (cap. 19: binder     (cap. 21: catálogo       (cap. 20: 1:1            (cap. 20:
   qué es          tokens +        + plan lógico;      + estimaciones +          al árbol                modelo
   LiraQL)         errores con     el QUÉ sin el       5 reglas; el             físico)                 Volcano)
                  byte y línea)    CÓMO)               CÓMO barato)
```

Cada capa dejó una garantía que las siguientes heredan: el **lenguaje** (17) fijó MATCH-WHERE-RETURN sin prometer orden; el **parser** (18) garantiza que el AST es fiel al texto o señala el byte exacto; el **lowerer** (19) garantiza variables únicas y ligadas (`bound_variables()` — la herramienta con la que R2 respeta bindings); el **Volcano** (20) garantiza el ciclo open/next/close y NUMERA lo que fluye (sus métricas destaparon el 4-para-1); y el **optimizador** (21) garantiza mismos resultados con menos trabajo. Quita cualquier eslabón y la cadena se rompe en un sitio previsible: sin parser no hay feedback de errores; sin binder no hay pushdown seguro; sin métricas no sabrías que había nada que optimizar; sin optimizador, los índices del cap. 15 siguen siendo un órgano sin función.

## 21.12 Qué hemos sacrificado

1. **Estimaciones honestas, no precisas.** 1/3 para todo rango subestima el 0.75 del demo. El precio de la simplicidad; el premio: ver la discrepancia en cada explain.
2. **Búsqueda por coste real.** Nuestras reglas reordenan CADENAS simples; los cartesianos múltiples y los grafos cíclicos con backtracking quedan como están. Un optimizador de coste enumeraría más espacio.
3. **Catálogo no persistente.** Recalcular por consulta cuesta un escaneo completo: impensable en producción, perfecto para razonar sin dudar de la frescura de los números.
4. **Orden de filas no garantizado.** Consecuencia necesaria de reordenar ligaduras; llegará `ORDER BY` y con él la obligación.
5. **Igualdad exacta en el índice del catálogo.** `p.age = 36.0` no encuentra el `36` almacenado (sin la promoción Int/Float del runtime). Las estadísticas estiman; la ejecución decide.

## 21.13 Cómo lo hace una BBDD real

- **System R / Selinger (1979)**: el origen de todo. Coste = páginas leídas + CPU, selectividades por defecto (1/10, 1/3), enumeración dinámica de joins con poda a árboles left-deep y la heurística de hundir predicados que hoy es nuestra R2.
- **PostgreSQL**: planificador de coste con estadísticas mantenidas por `ANALYZE` (histogramas, distinct values, MCV). Su `EXPLAIN` imprime el árbol con `rows` estimadas; `EXPLAIN ANALYZE` (7.2.0, 2002) además lo ejecuta y muestra `actual rows` y tiempos por nodo — el molde exacto de nuestro «est. N filas» contra «filas reales».
- **Catalyst (Spark)**: optimizador funcional por reglas + coste: parseo → plan lógico sin resolver → resolver → reescrituras (pushdown de filtros y proyecciones incluidas) → planificación física eligiendo estrategias. Nuestro pipeline ANTES/DESPUÉS es la misma película en miniatura.
- **Kùzu (grafos)**: optimizador de grafo con reordenación de joins por coste, filtro pushdown hacia los escaneos y uso del catálogo de estadísticas del grafo — punto por punto, las tres piezas de este capítulo, a escala industrial.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en la consulta del §21.9, ¿por qué el `Expand` del DESPUÉS estima 1 si el grado in de Person es 1.00 y la fracción de KNOWS 4/6?
- *Intermedio*: ¿qué pasa —plan y filas— si el WHERE fuera `f.age < 100`? ¿Cambia R1? ¿Debería?
- *Experto*: diseña `rule_pushdown_limit` para un hipotético `Limit` encima de un `Filter`: ¿cuándo es seguro bajarlo? ¿Y sobre un `Expand` (pista: top-k)?

## 21.14 Lo que te llevas

- El **optimizador** reescribe el PLAN (nuestra IR), nunca la consulta (suya): mismo destino, otra ruta.
- El **catálogo** mira el grafo: nodos por etiqueta, grados medios, tipos, índice de igualdad. Sin datos no hay decisión, hay manía.
- **Estimar** con heurísticas (System R: 0.1 / 1/3) basta para ORDENAR planes; la discrepancia con lo real (est. 1 vs 3) es contenido, no bug.
- **Predicate pushdown** (R2) es la regla reina: filtrar antes de expandir convierte 4-para-1 en 1-para-1 — de 1979 a hoy.
- **IndexSeek lo elige el optimizador** con la única información que lo hace seguro: cuántos ids devuelve frente a cuántos escanea.
- **Equivalencia testada**: un optimizador que cambia resultados es un bug; sin ORDER BY, el multiconjunto sí, el orden no.

## 21.15 Ojo, cuidado con…

- **Bajar predicados sin mirar bindings**: la trampa nº 1. `sink` usa `bound_variables()`; tú, usa `sink`.
- **Confundir selectividad con cardinalidad**: la primera es una fracción (0.25), la segunda un número de filas (4 × 0.25 = 1). El explain muestra la segunda.
- **IndexSeek siempre**: si el índice devuelve tantas filas como el escaneo, no ahorra; R4 sólo aplica si `ids.len() < scan_rows`.
- **Leer `est. 1 filas` como una promesa**: es un argumento de comparación, no una respuesta. La respuesta son las filas reales de abajo.
- **Catálogo vs índice**: el catálogo AYUDA A DECIDIR; el índice RESUELVE la búsqueda. Uno informa reglas; el otro ejecuta lecturas.

## 21.16 Pin de batalla

> *«Un optimizador que devuelve resultados distintos no está optimizando: está mintiendo más rápido.»*

## 21.17 Si solo lees 30 segundos

El plan que sale de `lower` es correcto pero ingenuo: filtra al final, escanea de más. El optimizador lo reescribe en cinco pasos fijos — elegir el punto inicial más selectivo, bajar los predicados, absorber etiquetas, convertir igualdades en `IndexSeek`, podar proyecciones — usando estadísticas del grafo (cuántos nodos por etiqueta, qué grados medios) para comparar candidatos. Los resultados NO cambian: se testea la equivalencia antes y después. `liradb explain` te enseña el ANTES, el DESPUÉS, las filas estimadas y las reales — y la diferencia entre ellas es la lección.

## 21.18 Una historia pequeña

Cuando conectamos el optimizador por primera vez, `liradb query` dejó de devolver filas en una consulta del demo. Pánico: habíamos roto el motor. Media hora después, el culpable: un pushdown demasiado entusiasta empujaba `f.age < 40` por debajo del `Expand` que liga `f` — el filtro evaluaba la edad de una variable que aún no existía. El fix fue una línea: preguntarle al subárbol sus `bound_variables()`. La moraleja se nos quedó grabada: el optimizador trabaja sobre el contrato del binder del cap. 19; quien reescribe planes sin saber qué variables están ligadas no está optimizando, está apostando.

## Ejercicios resueltos

**1. En la salida real del §21.9, ¿por qué el plan ANTES estima 4 en el `Expand` y 1 en el `Filter`?**

`NodeScan(Person AS p)` estima 4 porque el catálogo cuenta 4 Person. El `Expand` multiplica su entrada por el grado medio OUT de Person (1.50 — seis aristas salen de personas: Ana 2, Bo 2, Carla 1, Dani 1, sobre 4 nodos) y por la fracción de aristas KNOWS (4/6): 4 × 1.5 × 0.667 = 4. El `Filter` multiplica por la selectividad de `f:Person ∧ f.age < 40`: la etiqueta ya está declarada por el patrón (1.0) y el rango usa SEL_RANGE (1/3): 4 × 1/3 = 1.33, que al mostrarse se redondea a 1. Verificación: `estimacion_scan_filter_expand` calcula estas mismas cifras contra el plan real.

**2. ¿Por qué en la canónica del brief (`WHERE p.name = "Ana"`) el átomo `f:Person` NO baja hasta el escaneo, si el pushdown baja predicados?**

Porque el pushdown baja átomos, no árdenes: `f:Person` menciona a `f`, y en el plan tras R1 `f` la liga el `Expand` — no hay NINGÚN sitio por debajo donde `f` ya esté ligada. La frontera del `Expand` es exactamente eso: lo que menciona variables ligadas por debajo baja; lo que menciona `to` se queda arriba. El plan queda `Filter(f:Person)` sobre `Expand(p, KNOWS, OUTGOING, f)` sobre `IndexSeek(Person.name = "Ana")` — verificado por `pushdown_canonico_del_brief_con_index_seek`.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin ejecutar nada, predice el plan DESPUÉS de `MATCH (a:Person), (c:City) WHERE a.age > 35 AND c.name = "Madrid" RETURN a.name, c.name`: cómo se parte el AND, dónde acaba cada átomo, cuál se convierte en `IndexSeek` y por qué el otro no. Verifícate con `cargo run -q -p liradb-cli -- explain "..."` y con el test `pushdown_reparte_el_cartesiano_y_busca_indices`. *Pistas*: (1) ¿qué variables liga cada lado del cartesiano? (2) ¿qué forma exacta exige R4? (3) ¿cuántos "Madrid" hay bajo City? *Criterio*: acertar la partición del AND y el IndexSeek exacto.

**Intermedio (analizar).** La raíz del §21.9 estima 1 y devuelve 3. (a) Calcula la selectividad real de `age < 40` sobre las Person del fixture y compara con SEL_RANGE. (b) ¿Podría esa discrepancia llegar a cambiar el plan elegido por R1? Construye un WHERE donde sí (pista: filtros a ambos lados con heurísticas que mientan en direcciones opuestas). (c) ¿Qué estadística mínima —sin histogramas completos— afinaría el rango? Verifícate con `explain_la_consulta_del_reordenado` y `estimacion_scan_filter_expand`. *Criterio*: separar «ordenar planes» de «acertar filas».

**Experto (crear — cierre de Parte IV, retrieval puro).** Primera parte, de memoria (sin mirar los caps. 17-20): reconstruye la cadena `texto → parse → lower → optimize → compile → open/next/close → filas` y escribe, para cada eslabón, qué capa la añade y qué invariante garantiza. Segunda parte: extiende `Catalog` con min/max por (etiqueta, propiedad) — acumúlalos en `collect` — y úsalos en `compare_selectivity` para estimar rangos por interpolación ((max − x)/(max − min)) en lugar de 1/3. Comprueba que el explain del §21.9 pasa de `est. 1` a algo cercano a 3 en la raíz y que el PLAN elegido no cambia (R1 sigue empezando por f). *Pistas*: (1) ¿dónde del bucle de nodos tocaría acumular el par?, (2) ¿qué rama del `match op` es la de los rangos?, (3) ¿por qué el plan ganador no debe moverse? *Criterio*: estimación mejorada + mismo plan + la batería de equivalencia (`equivalencia_antes_y_despues_sobre_bateria_de_consultas`) sigue verde.

## Para profundizar

- **P. G. Selinger et al., «Access Path Selection in a Relational Database Management System» (SIGMOD 1979)** — el paper fundacional: selectividades por defecto, dinámica de joins, y el pushdown como heurística. Nuestros 0.1 y 1/3 salen de aquí.
- **PostgreSQL, «Using EXPLAIN» (docs oficiales) y release notes 7.2.0** — el formato plan + rows estimadas + actual rows que hemos imitado.
- **Alex Petrov, «Database Internals» (O'Reilly, 2019)** — capítulo de ejecución y optimización como pieza del motor completo.
- **CMU 15-445, lecciones de query optimization** — la taxonomía heurística vs coste, con Cascades como horizonte.
- **Armbrust et al., «Spark SQL: Relational Data Processing in Spark» (SIGMOD 2015)** — Catalyst: reglas + estrategias sobre árboles de planes, en producción a escala enorme.

## Mini-diálogo: en la boca del túnel

> — Entonces el optimizador es… ¿un montón de ifs que cambian mi plan?
>
> — Es un montón de ifs con TASTE. Cada uno encarna una decisión que alguien tomó mirando datos: qué lado es más barato, qué predicado baja, qué índice ahorra. Selinger los escribió en 1979 y siguen ahí.
>
> — ¿Y si se equivoca? Mi estimación decía 1 y eran 3.
>
> — Se equivocó en la cifra y acertó en la decisión: f seguía siendo el mejor punto de partida. Las estimaciones no tienen que acertar; tienen que ORDENAR bien. El día que ordenen mal, no cambias el plan: cambias las estadísticas.
>
> — ¿Y nada puede romper mis resultados?
>
> — Todo puede romper tus resultados. Por eso no confiamos en que no: lo testeamos. Doce consultas, dos caminos, mismas filas. El optimizador no pide fe; pide evidence.

---

*(Próximo capítulo: 22 — Caminos mínimos ponderados. El optimizador eligió la ruta más barata para LIGAR un patrón; ahora la pregunta cambia: ¿cuál es el camino más corto de Ana a Carla cuando las aristas pesan? Dijkstra entra en escena — y abre la Parte V.)*
# Capítulo 22 — Caminos mínimos ponderados (Dijkstra y Bellman-Ford)

> *«Un camino puede tener más saltos y costar menos. El día que tus aristas pesan, "cerca" deja de significar "pocos saltos" y empieza a significar "barato".»*

## 22.0 La anécdota de la esquina

Según contaba el propio Dijkstra, en 1956 hizo «dos cosas importantes»: terminó su carrera y asistió a la inauguración oficial del ARMAC, el computador del Mathematisch Centrum de Ámsterdam. Para la inauguración necesitaba una demostración que los no-informáticos entendieran —y su respuesta también—, así que preparó un programa que hallara la ruta más corta entre dos ciudades de Holanda sobre un mapa reducido de 64 ciudades (seis bits bastaban para identificar una). La pregunta que se hizo era de una sencillez insultante: ¿cuál es el camino más corto de Rotterdam a Groningen?

Y aquí la parte que ha contado mil veces quien firma el algoritmo: «Una mañana estaba de compras en Ámsterdam con mi joven prometida, y cansados, nos sentamos en la terraza de un café a tomar un café, y yo estaba pensando si sería capaz de hacerlo, y entonces diseñé el algoritmo del camino mínimo. Como he dicho, fue una invención de veinte minutos». Después remataba: «de hecho, se publicó en el 59, tres años tarde». Y un detalle que es puro Dijkstra: «una de las razones de que [el paper] sea tan agradable es que lo diseñé sin lápiz ni papel. Sin lápiz ni papel estás casi obligado a evitar toda complejidad evitable».

Ese algoritmo nacido en una servilleta que nunca existió se publicó como «A Note on Two Problems in Connexion with Graphs» (Numerische Mathematik 1, 1959, págs. 269-271), dos páginas y media; en los años sesenta ya aparecía en un libro alemán de investigación operativa como «Das Dijkstra'sche Verfahren», y hoy vive en cada GPS: como le gustaba decir al entrevistador que acababa de consultar una ruta, «esta mañana ha usado usted mi algoritmo». Nosotros vamos a instalarlo donde nunca ha estado tan a gusto: sobre un grafo persistente cuyo peso es una PROPIEDAD escrita por un usuario que puede, tranquilamente, haberse equivocado. (Fuente: entrevista de Philip L. Frana a E. W. Dijkstra, historia oral del Charles Babbage Institute, 2001, publicada en Communications of the ACM 53(8), agosto de 2010.)

## 22.1 Objetivo

Al terminar este capítulo sabrás **por qué "el camino con menos saltos" deja de ser la respuesta correcta en cuanto las aristas pesan**, y habrás ejecutado los dos algoritmos clásicos de caminos mínimos SOBRE el grafo persistente de LiraDB — con los pesos leídos de las propiedades de las aristas, no de una matriz preparada a mano como en el Vol.I (cap. 4).

Cuatro piezas, todas en `cap22_caminos_minimos.rs`:

1. **La fuente de pesos** (`WeightSource`) — de dónde sale el peso de una arista: una propiedad (`WEIGHT relationship.distance`, la consulta del brief) o una constante.
2. **La extracción estricta** (`edge_weight`) — semántica tipada para el dato sucio: ausente, no numérico, no finito.
3. **Dijkstra** (`dijkstra` / `dijkstra_path`) — min-heap de std, borrado perezoso, finalización anticipada.
4. **Bellman-Ford** (`bellman_ford` / `bellman_ford_path`) — pesos negativos legítimos y detección del ciclo negativo que contamina.

Este capítulo abre la Parte V: algoritmos que ya no recorren el motor (Parte IV), sino el grafo.

## 22.2 Problema

La consulta que abre esta parte existe en el brief desde el principio:

```text
SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance
```

Fíjate en lo que exige: el peso no lo pone quien consulta, lo pone QUIÉN GUARDÓ el dato. `relationship.distance` es una propiedad de arista (cap. 7): un `Value` dentro de `Edge.props`. Y un grafo de propiedades es schemaless — nadie garantiza que la propiedad exista en todas las aristas, ni que sea numérica donde exista.

Compruébalo con nuestro propio `demo_graph` (cap. 20): las tres `KNOWS` «de verdad» llevan `since` (2020, 2021, 2022), pero el self-loop de Dani (edge 3) no lleva ninguna, y las dos `LIVES_IN` tampoco. Pregunta por `SHORTEST ... WEIGHT relationship.since` y el grafo te responde: no puedo, hay tramos sin precio. El problema del capítulo no es el algoritmo — ya lo conoces del Vol.I — sino el CONTRACTO entre el algoritmo y un dato que nadie validó al escribirlo.

Y hay un segundo problema, más silencioso. Hasta hoy, «camino» en LiraDB ha querido decir saltos: el `Expand` del cap. 20 encadena relaciones y cuenta. Pero con pesos, el directo caro pierde contra el rodeo barato:

```text
        2.0        3.0
    0 ────► 1 ────► 2        coste por arriba: 5.0
    └──────────────────►      coste por abajo: 10.0 (una arista sola)
             10.0
```

El camino de dos saltos cuesta 5; el de un salto, 10. Menos aristas no es más barato. Todo lo que sigue existe para responder a esa obviedad con rigor.

## 22.3 Modelo mental

Piensa en una **red de tramos con tarifa**: nodos = estaciones; aristas = tramos; pegado a cada tramo, su precio (la propiedad). El coste de un viaje es la suma de los precios de los tramos que cruzas. Sobre esa misma red, dos formas de organizarse:

- **Dijkstra es la ventanilla que llama por tarifa.** Cada estación espera su turno con un número provisional; la ventanilla llama SIEMPRE al más barato pendiente. Cuando te llama, tu tarifa queda lacrada: es definitiva, porque todo el que sigue en la cola pagó ya igual o más para llegar hasta ahí. Eso es `settled`. Y si tú sólo esperabas a UNA estación (tu destino), cuando la llaman te levantas y te vas: **finalización anticipada**.
- **Bellman-Ford es el tablón de anuncios por rondas.** Cada tarde (cada pasada), todos los viajeros releen el tablón completo de tarifas y pegan su mejora. No hay orden de llamada, nada se lacra — pero tolera descuentos (pesos negativos) y, si en el pueblo hay una rueda de descuento infinita que alguien puede alcanzar, la última pasada la delata.

```
 DIJKSTRA (ventanilla)                    BELLMAN-FORD (tablón)
 heap: [(0,A)]                            ronda 1: releer TODAS las aristas
 llama a A (0.0)  ── lacrado              ronda 2: releer TODAS las aristas
 heap: [(1,B),(4,C)]                      ...hasta que una ronda no pega nada
 llama a B (1.0)  ── lacrado              pasada extra de verificación:
 llama a C (3.0)  ── lacrado                ¿algo aún mejora? → ciclo negativo
 (todo lo que quede pesa ≥ que lo
  ya llamado: por eso lacre = verdad)
```

El momento ¡ajá!: **el orden de llamada no es un truco de implementación, es la corrección misma**. Puedo lacrar a B porque ningún tramo futuro puede rebajar lo ya pagado — y eso es exactamente lo que un precio negativo rompe. Dijkstra rechaza negativos porque su prueba de corrección los usa; Bellman-Ford paga V-1 pasadas para recuperar el derecho a tenerlos.

## 22.4 Primera solución

La versión ingenua ya la tienes funcionando: encadenar `Expand` (cap. 20) y contar saltos. De hecho es TAN legítima que la hicimos oficial: `WeightSource::Constant(1.0)` es el `Default` — con todos los tramos a precio 1, el camino mínimo ponderado degenera en el camino con menos saltos, que es lo que el motor llevaba haciendo sin saberlo.

Y la versión ingenua «ponderada» que escribiría cualquiera: leer `edge.props["distance"]` y, si no está, usar 1.0 «para no romper la consulta».

## 22.5 Sus límites

1. **El default amable miente.** Con `1.0` donde falta el dato, un import a medias produce caminos con precios inventados — y nadie lo sabe. Un grafo schemaless debe VER su dato sucio; una base de datos que rellena el hueco no está siendo amable, está mintiendo con buena letra.
2. **El NULL no es cero ni es uno.** En el cap. 20 decidimos que la propiedad ausente SE VE como NULL; hoy decidimos la simetría: el NULL SE TRATA como ausencia. «Sin precio pegado» es un único concepto con un único error (`MissingWeight`).
3. **Un NaN suelto revienta el heap.** `BinaryHeap` exige `Ord`; `f64` no lo implementa precisamente porque el NaN no tiene orden total. Sin validación previa, el día que alguien importe un `distance: NaN`, tu consulta muere de panic a mitad de cálculo — o peor, ordena «a medias».
4. **El infinito ya está ocupado.** Usamos `f64::INFINITY` como centinela de «inalcanzable». Si un coste real desborda a infinito (1e308 + 1e308), se colaría en el disfraz de inalcanzable: un camino que existe reportado como «no hay camino».

## 22.6 Solución evolucionada, parte 1: la fuente de pesos (o el precio pegado al tramo)

Primera pieza: decir DE DÓNDE sale el peso. Dos historias, un enum:

```rust
pub enum WeightSource {
    Property(String),   // WEIGHT relationship.distance — el caso del brief
    Constant(f64),      // todas igual; Default = 1.0 = contar saltos
}
```

Y la extracción, con semántica estricta y errores tipados (`edge_weight`):

| El dato que hay en `props[name]` | Resultado |
|---|---|
| nada, o `Value::Null` | `MissingWeight { edge, prop }` |
| `Bool`, `String`, `Bytes` | `InvalidWeight { edge, prop, found }` (con `Value::type_name`) |
| `Int(i)` | `i as f64` — promoción Int→Float |
| `Float` NaN o ±∞ | `NonFiniteWeight { edge, weight }` |

Los tres errores hablan en cristiano, con arista y propiedad — esto es su `Display` REAL:

```text
edge 3 has no weight property 'since' (missing or NULL)    ← MissingWeight
edge 3: weight property 'cost' is String, not a number     ← InvalidWeight
edge 3: non-finite weight NaN                              ← NonFiniteWeight
```

**¿Por qué tan estrictos, si un default sería tan cómodo?** Porque las tres filas de esa tabla son preguntas distintas que el usuario necesita oír por separado: «no guardaste el peso» (corríjase el import), «guardaste texto donde iba un número» (corríjase el ETL), «guardaste algo que no es representable» (corríjase el origen). Un `1.0` silencioso fusiona las tres en una respuesta limpia sobre un dato roto. El test `demo_graph_con_pesos_reales_y_calidad_de_dato` lo documenta con nuestro propio grafo: con `WeightSource::property("since")`, la primera arista sin el dato —el self-loop de Dani, edge 3— es la que salta, ANTES de calcular nada.

**¿Por qué promoción Int→Float y no aritmética entera?** Porque los pesos nacen del `Value` del cap. 7, que ya mezcla `Int` y `Float`, y porque el coste de un camino es una suma que se quiere uniforme. Tiene un precio documentado y testeado: f64 representa enteros exactos sólo hasta 2^53, así que `9.007.199.254.740.993` (2^53+1) se promueve a `9.007.199.254.740.992` — el test `pesos_int_se_promocionan_a_float_con_perdida_documentada` lo clava con números, y con valores razonables la suma es exacta.

**¿Por qué `Constant` por defecto?** Lo menos sorprendente cuando nadie ha dicho qué propiedad es el peso: contar saltos, la semántica que el motor ya tenía. Y deja explícita una idea que vale el capítulo entero: «no ponderado» no es otro problema — es el caso particular en que todos los tramos valen 1. El test `la_fuente_de_pesos_cambia_la_respuesta` ejecuta el MISMO grafo con las dos fuentes y gana un camino distinto: ponderado, el rodeo (5.0); a saltos, el directo (1.0).

## 22.7 Solución evolucionada, parte 2: Dijkstra sobre el store (la ventanilla)

El corazón es literalmente el del Vol.I, cap. 4 — pero leyendo aristas del puerto del cap. 8:

```rust
let mut heap: BinaryHeap<Reverse<(Cost, NodeId)>> = BinaryHeap::new();
while let Some(Reverse((Cost(d), u))) = heap.pop() {
    if settled[u] { continue; }          // entrada obsoleta: borrado perezoso
    settled[u] = true;                   // lacre: dist[u] es definitiva
    if target == Some(u) { break; }      // finalización anticipada
    for eid in store.out_edges(u) {
        // ... relajar: new = d + w; si new < dist[v] { push }
    }
}
```

Cada línea tiene su porqué:

**¿Por qué sobre `&dyn GraphStore` y no sobre el CSR del cap. 14?** Porque los pesos viven en `Edge.props`, y EdgeId→Edge es exactamente el acceso que da el trait; el CSR persiste SÓLO topología (offsets + targets, sin ids de arista), así que no puede responder «¿cuánto pesa esta arista?». Podríamos proyectar un CSR con pesos… y esa es exactamente la proyección que el cap. 26 generalizará. Deuda explícita, no omisión. De regalo, trabajar contra el trait mantiene el algoritmo agnóstico al backend: `MemoryStore` hoy, disco mañana.

**¿Por qué `Reverse`?** El `BinaryHeap` de std es un max-heap: sin `Reverse`, llama primero al PEOR candidato. La clave `(Cost, NodeId)` ordena por coste y desempata por id — mismas entradas, mismo orden de salida, siempre.

**¿Por qué borrado perezoso y no decrease-key?** Porque std no tiene decrease-key (no hay forma de «bajar» la prioridad de un elemento ya insertado). El patrón: cuando `dist[v]` mejora, se INSERTA una entrada nueva y la vieja se descarta al salir, detectada por `settled[u]`. Coste: O(log n) por push y algunos pops muertos (que `PathStats.popped` cuenta honestamente). La alternativa —una cola indexada con decrease-key— existe en crates externas; la descartamos por el mismo criterio del Vol.I: std y nada más que std.

**¿Por qué `Cost` como newtype?** `f64` no implementa `Ord` (culpa del NaN) y el heap lo exige. `Cost` envuelve el f64 y le da orden total; su `expect` es un unreachable documentado —todo lo que entra en el heap es finito porque `edge_weight` ya lo garantizó y los desbordes se reportan como `CostOverflow` antes de tocar el centinela.

**¿Por qué finalización anticipada?** No es una micro-optimización: es el invariante codicioso hecho código. Cuando tu destino sale del heap, su distancia es definitiva (todo lo pendiente pesa igual o más). El test `dijkstra_finalizacion_anticipada_extrae_menos_nodos` lo mide: en una cadena 0→1→…→5, preguntar por 1 asienta {0, 1} y se va; la tabla completa asienta los seis.

**¿Y los negativos?** Aquí está la decisión más discutida del capítulo: `validate_edge_weights` recorre TODAS las aristas ANTES de correr y rechaza cualquier negativo con `PathError::NegativeWeight` — aunque la consulta no fuera a pisar esa zona. ¿No es exagerado? Piensa qué significa la alternativa: consultas que a veces aciertan y a veces devuelven números plausibles pero malos, según qué zona del grafo tocaron. Una base de datos prefiere FALLAR RUIDOSAMENTE a contestar casi-bien; el día que importes pesos negativos, querrás enterarte en todas tus consultas, y el propio mensaje te dice la salida: `use bellman_ford`.

### Dijkstra en marcha: la vida de una entrada del heap

Merece la pena seguir una ejecución completa. El diamante del Vol.I — `0→1 (e0, w1)`, `1→3 (e1, w2)`, `0→2 (e2, w4)`, `2→3 (e3, w3)` — con destino 3:

| Paso | Pop | Acción | Heap tras el paso |
|---|---|---|---|
| 0 | — | `dist[0]=0`, push (0, 0) | `[(0,0)]` |
| 1 | (0, 0) | asienta 0; relaja e0: `dist[1]=1` (push); relaja e2: `dist[2]=4` (push) | `[(1,1), (4,2)]` |
| 2 | (1, 1) | asienta 1; relaja e1: `dist[3]=3` (push, con su `PathStep`) | `[(3,3), (4,2)]` |
| 3 | (3, 3) | asienta 3 = **destino → break**. La (4, 2) ni se mira | (queda (4,2), ignorada) |

Tres pops, tres expandidos, cuatro intentos de relajación, tres que mejoraron. La entrada `(4, 2)` sobrevive en el heap sin importar nada: ahí está el borrado perezoso en acción — si el bucle continuara (variante tabla completa), saldría, asentaría el 2 con `dist[2]=4` definitivo, y relajaría e3 (`4+3=7 > 3`: intento fallido, no mejora). El momento en que un pop descubre `settled[u] == true` y hace `continue` es la firma del patrón: esa entrada nació de una promesa que otra mejora dejó obsoleta. Y la relajación que sí mejora es literalmente:

```rust
if new < dist[v] {
    dist[v] = new;
    pred[v] = Some(PathStep { edge: eid, from: u, to: v, weight: w });
    heap.push(Reverse((Cost(new), v)));   // la entrada vieja quedará obsoleta
}
```

Nota lo que NO hay: ningún «buscar en el heap y bajar la prioridad». Se inserta y se abandona. El coste de esa comodidad —pops muertos— queda anotado en `stats.popped`, no escondido.

## 22.8 Solución evolucionada, parte 3: Bellman-Ford (el tablón que admite descuentos)

Bellman-Ford compra con sus V-1 pasadas el derecho que Dijkstra no puede pagar: pesos negativos LEGÍTIMOS (una arista de -4 es una respuesta, no un error — test `bellman_ford_explota_un_peso_negativo_que_dijkstra_rechaza`: la ruta 3 + (−4) = −1 gana al directo 1). Míralo en rondas sobre ese mismo grafo — aristas en orden de id: `e0: 0→2 (w3)`, `e1: 2→3 (w−4)`, `e2: 0→3 (w1)`:

| Ronda | Lo que pasa al releer el tablón | dist tras la ronda |
|---|---|---|
| 1 | e0: `dist[2]=3` (mejora); e1: `3−4=−1` → `dist[3]=−1` (¡mejora!); e2: `0+1=1 < −1`? No | `[0, ∞, 3, −1]` |
| 2 | nada mejora | `[0, ∞, 3, −1]` → `changed=false`, BREAK |

`rounds == 2`, no las V−1 = 3 posibles: la parada temprana trabajó. Fíjate en la ronda 1: BF procesó `e1` ANTES que `e2` por pura casualidad de orden de lista — con otro orden, `dist[3]` habría bajado primero a 1 y a −1 en la ronda 2. El RESULTADO converge igual (−1); el camino que recorre cada ronda, no. Dijkstra no tiene esa tolerancia al azar: su heap impone un orden que ES su prueba de corrección.

Tres decisiones más de la implementación:

**La lista de relajación se materializa UNA vez.** Antes de las pasadas, `bellman_ford` recolecta `(source, target, edge_id, weight)` de todas las aristas en un `Vec`. ¿Por qué? Porque los pesos se leen de las props, y releerlas en cada ronda serían V-1 búsquedas hash por arista para el MISMO valor. Y si el store viviera en disco (caps. 11-13): V-1 lecturas de página por arista. El capítulo del buffer pool se cobra aquí su moraleja: cada lectura que puedas no repetir, no la repitas.

**Parada temprana de pasadas, sí; del destino, no.** Si una pasada no mejora nada, el tablón ha convergido: otra vuelta no cambiaría nada, `if !changed { break; }` — una cadena de 4 saltos converge en 2 rondas, no en las V-1=4 posibles (test `bellman_ford_para_temprano_cuando_nada_cambia`, `rounds == 2`). Pero NO hay early-exit «cuando el destino ya tiene distancia», y la ausencia es deliberada: con negativos, un camino más LARGO puede ganar DESPUÉS. Lacrar el destino exigiría el invariante codicioso, y BF existe precisamente porque renunció a él.

**La pasada de verificación distingue el ciclo que contamina.** Tras las V-1 pasadas, una vuelta extra: si alguna arista con `dist[u]` finito TODAVÍA relaja, hay un ciclo negativo ALCANZABLE desde el origen → `PathError::NegativeCycle` señalando ESA arista. ¿Por qué reachable y no «exista»? Aguas abajo de un ciclo alcanzable las distancias tienden a −∞: devolver media tabla válida sería mentir. Pero un ciclo en una componente que NADIE alcanza desde el origen no contamina la respuesta — el test `bellman_ford_ciclo_negativo_inalcanzable_no_contamina` tiene los dos casos en el mismo archivo: la isla rota responde con normalidad, y el mismo ciclo conectado al origen sí es error. Fíjate en la condición exacta: `dist[u] != f64::INFINITY && dist[u] + w < dist[v]` — la primera mitad ES la distinción alcanzable/inalcanzable.

Y una garantía silenciosa que agradece el lector: sin negativos, BF y Dijkstra dan LA MISMA tabla, distancia a distancia (test `bellman_ford_coincide_con_dijkstra_sin_negativos`). Dos algoritmos, un contrato de resultado.

## 22.9 Prueba de fuego

Los tests no verifican «devuelve un número»: verifican que el camino es VÁLIDO contra el store — continuidad, aristas existentes, coste = suma de pesos (`assert_camino_valido`). Tres pruebas destacadas:

**El oráculo CSR.** El cap. 14 nos dio una proyección topológica del grafo; hoy le pedimos que actúe de testigo (`proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra`): la alcanzabilidad por BFS sobre el CSR debe coincidir EXACTO con `sp.reached()` de Dijkstra. Topología y algoritmo cuentan la misma historia — y cuando el cap. 26 añada pesos al CSR, este test es el molde del oráculo definitivo.

**El grafo demo con calidad de dato.** `dijkstra_path(&demo, 0, 2, &Default::default())` → coste 2.0 por 0 -KNOWS-> 1 -KNOWS-> 2; con `property("since")` → `MissingWeight { edge: 3 }` (el self-loop de Dani); y Dani inalcanzable → `Ok(None)`, que no es error: es una respuesta. Es el doctest del módulo, ejecutado por `cargo test --doc` en cada build:

```rust
let store = demo_graph();
let sp = dijkstra(&store, 0, &Default::default()).unwrap(); // pesos = 1.0 (saltos)
assert_eq!(sp.distance(2), Some(2.0)); // 0 -KNOWS-> 1 -KNOWS-> 2
assert_eq!(sp.distance(5), Some(2.0)); // 0 -KNOWS-> 1 -LIVES_IN-> 5 (Lisboa)
assert_eq!(sp.distance(3), None);      // Dani (sólo self-loop) es inalcanzable
```

Tres líneas, tres lecciones: los saltos son `Constant(1.0)`; un camino puede cruzar tipos de arista (KNOWS + LIVES_IN) si la fuente de pesos no discrimina; y `None` es parte del contrato, no un fallo.

**La respuesta con formato.** `Path` implementa `Display` estilo Cypher — `(n0)-[e4 w=1.5]->(n1)-[e7 w=2]->(n2) cost=3.5` — porque el `PathStep` guarda arista, extremos y peso de cada salto. ¿Por qué el predecesor es el paso COMPLETO y no un `NodeId`? Para que `path_to` reconstruya sin volver a tocar el store: cada `get_edge` que te ahorras es una página que no pides al pager (cap. 12). En memoria no se nota; en disco, es la diferencia entre una reconstrucción gratis y E lecturas.

¿Y si te saltas este capítulo? Tus caminos «ponderados» siguen siendo BFS con otro nombre, los pesos ausentes se rellenan en silencio y nadie puede explicar por qué la misma consulta responde distinto tras un import sucio. Síntoma exacto: números plausibles, cero contrato.

## 22.10 Qué miden las PathStats

`PathStats` es la métrica del cap. 20 aplicada al algoritmo — el «cuánto cuesta calcular»:

- **`relax_attempts` vs `relax_updates`**: aristas consideradas vs relajaciones que MEJORARON una distancia. El ratio es un diagnóstico del grafo: si intentas 1.000 veces y mejoras 12, el grafo «no mejora» — casi todo tu trabajo fue mirar aristas que ya no podían ganar. Un camino óptimo en un grafo hostil es, sobre todo, una colección de intentos fallidos.
- **`popped`** (Dijkstra): extracciones del heap, entradas obsoletas INCLUIDAS. Es el precio visible del borrado perezoso: pops muertos que existen porque std no tiene decrease-key.
- **`rounds`** (Bellman-Ford): pasadas ejecutadas — la convergencia real frente a las V-1 teóricas.
- **`expanded`**: pops VIVOS (sin obsoletos). Aparece numerado YA, pero es del cap. 23: será la vara para medir cuánto ahorra A* frente a Dijkstra. Hoy coincide con los pops útiles; mañana, la comparación.

## 22.11 Qué hemos sacrificado

1. **Velocidad frente a un CSR con pesos**: cada relajación pasa por `out_edges` + `get_edge` del trait. El cap. 26 pagará esa deuda con la proyección.
2. **Decrease-key real**: el perezoso gasta memoria en entradas duplicadas y pops muertos. Medido en `popped`; aceptado por no depender de crates.
3. **Aritmética exacta**: la promoción Int→Float pierde más allá de 2^53. Documentada y testeada; para pesos físicos, irrelevante; para bitcoins, ya sabes dónde mirar.
4. **Dijkstra «híbrido» que validara sólo lo que pisa**: sería más rápido en grafos parcialmente sucios — y una inconsistencia entre consultas. Preferimos el fail ruidoso.
5. **Recuperar distancias parciales tras un ciclo negativo**: cuando BF detecta el ciclo, tira TODA la respuesta. Devolver «la parte buena» invitaría a leerla como completa.

## 22.12 Cómo lo hace una BBDD real

- **Neo4j (Cypher clásico)**: la función `shortestPath()` cuenta RELACIONES, no pesos — exactamente nuestro `Constant(1.0)` como ciudadano de primera clase.
- **Neo4j GDS**: `gds.shortestPath.dijkstra.stream(..., { relationshipWeightProperty: 'distance' })` — el mismo parámetro conceptual que nuestro `WeightSource::Property`, y el mismo contrato: la documentación exige pesos POSITIVOS y, si no especificas propiedad, corre «unweighted» (nuestro `Constant`).
- **Kùzu**: la sintaxis `SHORTEST k` sobre relaciones de longitud variable (`MATCH (a)-[:Follows* SHORTEST 1..10]->(b)`) se resuelve con un recursive join tipo BFS — saltos, otra vez — y la variante ponderada llegó con función de coste explícita. Detalle delicioso: un PR de 2025 añadió al weighted shortest path un chequeo de pesos negativos que ERRA en vez de ignorarlos — la industria llegando, por su cuenta, a nuestra misma decisión del §22.7.
- **pgRouting**: `pgr_dijkstra(edges_sql, start, end)` — y una política opuesta a la nuestra que vale oro como contraste: «un valor negativo en la columna cost se interpreta como que la arista NO EXISTE». Ellos BORRAN silenciosamente; nosotros gritamos `NegativeWeight`. Para negativos legítimos tienen `pgr_bellmanFord` (experimental), con Bellman (1958) y Ford (1956) en el nombre.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en el grafo del §22.2, ¿qué camino gana con `Constant(2.5)` y cuál es su coste? ¿Cambia el ganador respecto a `Constant(1.0)`?
- *Intermedio*: pgRouting trata el coste negativo como arista inexistente; nosotros devolvemos `NegativeWeight`. Construye un caso donde esa diferencia cambie la RESPUESTA (no sólo el error).
- *Experto*: ¿qué le pasaría a `Cost::cmp` si relajáramos un NaN hasta el heap a pesar de la validación? Describe el modo de fallo exacto y por qué lo llamamos «unreachable documentado».

## 22.13 Lo que te llevas

- **El peso es un dato, no un parámetro**: vive en `Edge.props`, llega por `WeightSource::Property` y su ausencia es `MissingWeight`, nunca un default.
- **Semántica estricta en tres errores tipados**: ausente/NULL, tipo no numérico, no finito — un grafo schemaless debe VER su dato sucio.
- **`settled` + heap con `Reverse`**: el lacre del invariante codicioso; el borrado perezoso es el precio de no tener decrease-key en std.
- **Early-exit al destino en Dijkstra, NUNCA en Bellman-Ford**: uno lo autoriza el invariante; el otro renunció a él para admitir negativos.
- **Ciclo negativo: sólo el ALCANZABLE contamina** — y la arista señalada es la que aún relaja.
- **`CostOverflow` existe para que el infinito real no se disfraze de inalcanzable.**
- **`PathStats` numera el trabajo**: attempts vs updates, popped con muertos, rounds de convergencia.

## 22.14 Ojo, cuidado con…

- **Confundir menos saltos con más barato**: la trampa nº 1. Cura: ejecutar `la_fuente_de_pesos_cambia_la_respuesta` y mirar cómo gana un camino distinto por fuente.
- **Peso vs coste**: el peso es de UNA arista; el coste, del camino entero. `PathStep.weight` y `Path.cost` están a un lado y otro de esa frontera.
- **Inalcanzable vs desbordado**: ambos dan «infinito» en f64, pero uno es `Ok(None)` y el otro es `CostOverflow`. El centinela sólo significa lo primero.
- **Esperar symetría de early-exits**: BF tiene parada temprana (pasadas), no finalización anticipada (destino). No son lo mismo ni sirven para lo mismo.
- **Tratar NaN como «un número raro»**: es un ladrón de orden total. Todo lo que entra en un heap debe poder compararse SIEMPRE.

## 22.15 Pin de batalla

> *«Una base de datos que rellena el peso ausente con un 1.0 no está siendo amable: está mintiendo con buena letra.»*

## 22.16 Si solo lees 30 segundos

Los caminos mínimos ponderados leen el peso de las PROPIEDADES de las aristas (`WeightSource`), y como el grafo es schemaless, la extracción es estricta: peso ausente o NULL, tipo no numérico, o NaN/±∞ son errores tipados — nunca defaults. Dijkstra corre sobre `&dyn GraphStore` con un heap binario (`Reverse` + borrado perezoso, porque std no tiene decrease-key), lacra nodos al asentarlos y corta cuando el destino sale del heap; valida TODOS los pesos ANTES de empezar y rechaza negativos aunque no los pise, porque contestar casi-bien es lo peor que puede hacer una BD. Bellman-Ford admite negativos (V-1 pasadas sobre una lista materializada una vez, con parada temprana), y su pasada de verificación convierte el ciclo negativo ALCANZABLE en un error que señala la arista — el inalcanzable no contamina. Ambos devuelven lo mismo: tabla `ShortestPaths` y `Path` con pasos, coste y estadísticas.

## 22.17 Una historia pequeña

La primera vez que ejecutamos `dijkstra` con `WeightSource::property("since")` sobre el grafo demo, el resultado fue un error: `MissingWeight { edge: 3 }`. Reunión exprés: «ponle un default de 1.0 y listo, así no rompemos las consultas de nadie». Lo estuvimos mirando un rato hasta que alguien hizo la cuenta en voz alta: con el default, el self-loop de Dani costaría 1 — un precio que nadie escribió jamás — y las `LIVES_IN` también, y la consulta habría respondido un camino «barato» por tramos sin etiquetar. Decidimos esa tarde que el error era la respuesta correcta. Meses después, al leer la documentación de pgRouting, encontramos que ellos eligieron lo contrario (el coste sospechoso borra la arista, en silencio) — y nos alegramos dos veces: de nuestra decisión, y de tener un contraejemplo industrial con el que explicarla.

## Ejercicios resueltos

**1. Sobre `demo_graph`, ¿por qué `dijkstra(&s, 0, &WeightSource::property("since"))` falla con `MissingWeight { edge: 3 }` y no con una de las `LIVES_IN`, si ambas carecen de `since`?**

Porque la validación es EAGER y recorre `iter_edges()` en orden de id: las aristas 0, 1 y 2 (las KNOWS reales) llevan `since`; la primera que no lo lleva es la 3 —el self-loop de Dani— y ahí se detiene el mundo. Las `LIVES_IN` (ids 4 y 5) también están sucias, pero el error informa de la PRIMERA ofensora. Y fallar es correcto aunque la consulta fuera 0→5 y jamás pisara el self-loop: la respuesta de una BD no debe depender de qué zona del grafo llegó a tocar la búsqueda. Verificación: `demo_graph_con_pesos_reales_y_calidad_de_dato`.

**2. En el diamante del Vol.I (0→1 peso 1, 1→3 peso 2, 0→2 peso 4, 2→3 peso 3), ¿qué guarda `pred[3]` y cómo reconstruye `path_to` sin tocar el store?**

Gana 0→1→3 (coste 3.0 contra 7.0 por arriba): la última relajación que fijó `dist[3]` fue la arista 1 con `PathStep { edge: 1, from: 1, to: 3, weight: 2.0 }` — y eso es exactamente `pred[3]`. `path_to(3)` sigue la cadena hacia atrás (3 → 1 → 0 se corta en `pred[0] = None`), da la vuelta, y devuelve `nodes() = [0, 1, 3]` con `cost = 3.0` SIN una sola lectura del store: cada paso ya lleva su arista y su peso. Los predecesores no pueden ciclar: un ciclo en la cadena exigiría distancias estrictamente decrecientes al rodearlo — un ciclo negativo, que BF ya habría rechazado. Verificación: `dijkstra_camino_clasico_del_diamante` con `assert_camino_valido`.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sobre el grafo del §22.2 (0→1 peso 2, 1→2 peso 3, 0→2 peso 10), predice SIN ejecutar: camino y coste con `Constant(1.0)`, con `property("weight")`, y con `Constant(2.5)`; y la variante EXACTA del error (con su arista) si el peso viniera de `"distance"`. Verifícate con `la_fuente_de_pesos_cambia_la_respuesta` y `peso_ausente_o_null_es_missing`. *Pistas*: (1) ¿qué lee un `Constant` de las aristas — y qué cuenta entonces?; (2) ¿la validación eager mira el camino o el grafo?; (3) ¿en qué orden entrega `iter_edges` las ofensoras? *Criterio*: los tres costes exactos y la variante del error con arista y propiedad.

**Intermedio (analizar — con los caps. 11-14 en la mano).** (a) Explica por qué `bellman_ford` materializa la lista de relajación una sola vez, y qué coste tendría releer `Edge.props` en cada pasada si el store viviera en disco (cada `get_edge` puede costar una página: ¿cuántas rondas × aristas releería una cadena de 4 saltos?). (b) ¿Qué rompería exactamente un NaN que llegara al heap a pesar de todo — qué exige `BinaryHeap` de sus elementos y qué devuelve `partial_cmp` con NaN? (c) ¿Por qué `pred` guarda el `PathStep` completo en vez de sólo el `NodeId`? Verifícate con `bellman_ford_para_temprano_cuando_nada_cambia` y `peso_nan_o_infinito_es_no_finito`. *Pistas*: (1) cuenta las rondas del test; (2) mira el `expect` de `Cost::cmp`; (3) cuenta las lecturas de `get_edge` en una reconstrucción. *Criterio*: razonar en lecturas/páginas, no sólo en O-grandes.

**Experto (crear).** Primera parte, de memoria (sin mirar el capítulo): escribe la vida de una entrada del heap de Dijkstra — cuándo nace (push), cuándo queda obsoleta, cuándo la descartan y qué contador la cuenta — y enuncia el invariante que autoriza el `break` del early-exit. Segunda parte: escribe el test `ciclo_negativo_inalcanzable_se_vuelve_error_al_conectarlo` — parte del grafo de `bellman_ford_ciclo_negativo_inalcanzable_no_contamina` y añade UNA arista desde el origen hacia el ciclo; exige `NegativeCycle` señalando una arista concreta, y que `bellman_ford_path` también falle. *Pistas*: (1) ¿qué convierte «inalcanzable» en «alcanzable» con una sola arista?; (2) ¿qué arista tiene que seguir relajando tras V-1 pasadas?; (3) ¿qué condición sobre `dist[u]` pone la pasada de verificación? *Criterio*: test verde con `cargo test -p vol2-liradb` y saber explicar por qué ESA arista y no otra.

## Para profundizar

- **E. W. Dijkstra, «A Note on Two Problems in Connexion with Graphs» (Numerische Mathematik 1, 1959, 269-271)** — las dos páginas y media originales: camino mínimo y árbol generador mínimo, tal como salieron de la terraza del café.
- **Philip L. Frana, «An Interview with Edsger W. Dijkstra» (CACM 53(8), agosto 2010; historia oral del Charles Babbage Institute, 2001)** — el propio Dijkstra contando los veinte minutos, la falta de lápiz y papel, y «evitar toda complejidad evitable».
- **Cormen, Leiserson, Rivest y Stein, «Introduction to Algorithms», 3ª ed., cap. 24** — la prueba del invariante codicioso (§24.3) y el teorema de detección de ciclos negativos alcanzables (§24.1): el rigor detrás de cada decisión de este capítulo.
- **Docs de Neo4j GDS («Dijkstra Source-Target Shortest Path») y de pgRouting (`pgr_dijkstra`, `pgr_bellmanFord`)** — los dos contractos de pesos de producción, con el contraste silencio-vs-error comentado en §22.12.
- **Blog de Kùzu (release 0.0.4, «Shortest path queries»)** — cómo sintaxis Cypher (`SHORTEST k`), BFS y recursive joins conviven en un motor embebido real.

## Mini-diálogo: en la ventanilla

> — Entonces Dijkstra es un BFS con precios.
>
> — Es un BFS que aprendió a callarse hasta que sabe. BFS contesta por niveles de saltos; Dijkstra espera y llama por tarifas, y sólo cuando te llama tu número es definitivo. Esa paciencia ES la corrección.
>
> — ¿Y por qué tanto drama con un peso negativo? Total, un descuento.
>
> — Porque el lacre entero depende de que nadie pueda rebajar lo ya pagado. Un descuento rompe esa promesa: el que ya fue llamado podría mejorar. Bellman-Ford es el que acepta vivir sin lacre — y por eso paga V-1 pasadas y vigila la rueda de descuento infinita.
>
> — ¿Y si falta el precio de un tramo?
>
> — La ventanilla se cierra. Un `1.0` inventado te daría un viaje barato por tramos que nadie tasó. Preferimos que el grafo pase vergüenza una vez a que mienta para siempre.

---

*(Próximo capítulo: 23 — A*, heurísticas y búsquedas dirigidas. Dijkstra explora en círculos crecientes alrededor del origen; el destino podría estar al otro lado del grafo, esperando a que el círculo llegue. ¿Se puede tirar del hilo sin romper la optimalidad? La respuesta exige una propiedad nueva — admisibilidad — y `PathStats.expanded`, el contador que dejamos encendido hoy, medirá cuánto ahorra.)*
# Capítulo 23 — A*, heurísticas y búsquedas dirigidas

> *«Dijkstra sabe perfectamente cuánto ha caminado. No tiene ni idea de hacia dónde va. Esa diferencia es este capítulo.»*

## 23.0 La anécdota de la esquina

En 1966, en el Stanford Research Institute de Menlo Park, un robot llamado **Shakey** («temblón», por cómo le vibraba el chasis al arrancar) intentaba hacer algo que hoy suena trivial y entonces no lo era: moverse por una habitación llena de cajas sin chocar. Shakey no llevaba cerebro dentro: una cámara de TV y un telémetro láser enviaban las imágenes por radio a un DEC PDP-10 y un PDP-15 que ocupaban una habitación entera al lado, y el ordenador le devolvía por radio las órdenes. Todo ese viaje de ida y vuelta costaba tanto que Shakey se movía a paso de tortuga meditabunda (puedes verlo en las películas del SRI que conserva el Computer History Museum; el propio Peter Hart lo cuenta en el artículo de Wired de 2013).

Para que un robot que apenas se movía no perdiera además la vida planeando rutas, tres investigadores del proyecto — **Peter Hart, Nils Nilsson y Bertram Raphael** — escribieron en 1968 el paper «A Formal Basis for the Heuristic Determination of Minimum Cost Paths» (*IEEE Transactions on Systems Science and Cybernetics*, SSC-4(2)): el algoritmo **A\***. La idea no era explorar mejor que Dijkstra, sino explorar *menos*: si el robot sabe HACIA DÓNDE queda el destino, no necesita mirar en todas direcciones. Cuenta Hart que en el laboratorio iban nombrando versiones del algoritmo con letras —A, A1, A2— y al ganador le pusieron un asterisco, la estrella, para señalar que ése era el bueno. En 1972 los mismos tres autores publicaron una corrección en el SIGART Newsletter que bautizaba dos palabras que usaremos todo el capítulo: **admisible** y **consistente**. Casi sesenta años después, A\* sigue siendo el algoritmo que lleva tu GPS, los NPC de los videojuegos y el planificador de rutas de las bases de datos de grafos.

En el Vol.I ya programaste A\* sobre un grafo en memoria (cap. 9) y lo viste en robótica (cap. 29). Este capítulo lo hace correr **sobre el grafo del store** de LiraDB, y ahí aparece una pregunta que el Vol.I no necesitaba responder: ¿de dónde sale la heurística cuando los datos viven en una base de datos? Respuesta: de las **props de los nodos** — y de un trait.

## 23.1 Objetivo

Al terminar este capítulo sabrás **por qué Dijkstra explora «en círculo» y cómo dirigir la búsqueda hacia el destino sin perder la garantía de optimalidad**, y habrás usado (no reescrito: *usado*) cuatro piezas sobre la maquinaria del capítulo 22:

1. El trait `Heuristic` — el contrato «¿cuánto queda para llegar?».
2. `ZeroHeuristic` — h≡0, que convierte A\* en Dijkstra *exactamente* (y por eso es un test).
3. `EuclideanHeuristic` — la distancia recta leyendo las props `x`/`y` de los nodos.
4. `check_consistency` — el diagnóstico O(E) que delata la arista que rompe tu heurística.

Y una métrica nueva, `PathStats::expanded`, para **medir** el ahorro en vez de creerlo.

## 23.2 Problema

Piensa en el grafo-trampa del test `euclidea_mismo_coste_con_menos_expansiones`: diez ciudades en línea sobre el eje X (tramos de 1 km) y una **trampa**: tres nodos colgados al norte, a 100, 200 y 300 km del destino, unidos por aristas baratísimas de 0.5. Quieres ir de la ciudad 0 a la ciudad 9.

Dijkstra —tu `dijkstra_path` del cap. 22, con su finalización anticipada— expande **los 13 nodos**: la cadena y la trampa entera. No es un bug: Dijkstra ordena el heap por el coste acumulado `g`, y esos 0.5 son *objetivamente* más baratos que cualquier tramo de la cadena. Para quien sólo mira `g`, la trampa es irresistible. El radar no tiene brújula.

El problema no es de eficacia sino de *información*: el algoritmo sabe dónde has estado (`g`) y no sabe hacia dónde vas. La red de ciudades del hito cuenta la misma historia en grande: para ir de Madrid a Barcelona, Dijkstra asienta **las 7 ciudades** del mapa —incluidas Sevilla y Bilbao, que están en dirección *contraria*— antes de que Barcelona salga del heap. En un grafo de millones de nodos, ese círculo que crece desde el origen es la diferencia entre milisegundos y minutos.

## 23.3 Modelo mental

Dos imágenes, un contraste:

```
      DIJKSTRA (radar)                     A* (GPS)
      ordena el heap por g                 ordena el heap por f = g + h
      ondas circulares                     frente sesgado hacia el destino

  destino ●                                destino ●
          |  ____                                 /__
          | /    \                               /    ↗ ¡voy hacia allí!
      ____|/      \                          ___/↗
     /    |        \                        /  ↗ ← h(n) dice "queda poco"
    | ____ |         \                      | ↗
    |/    \|          ● origen              ● origen
      ondas: "lo más barato primero"        g (andado) + h (que queda)
```

- **Dijkstra es un radar omnidireccional**: emite desde el origen y asienta lo más cercano en coste, sin importar la dirección. Óptimo, sí; y ciego al destino.
- **A\* es un GPS**: cada nodo lleva una brújula `h(n)` que responde «¿cuánto queda hasta el destino?», y el heap ordena por la suma `f(n) = g(n) + h(n)`. Avanzar (`g`) y acercarse (`h`) puntúan juntos. La trampa del norte tiene `h` gigantesca —300 km rectos— y el GPS ni la mira.

El momento ¡ajá! es éste: **h no cambia el grafo, no cambia el algoritmo, no cambia ni una línea del bucle de relajación. Cambia el ORDEN del heap — y el orden de pops era todo lo que Dijkstra era.** Si metes `h ≡ 0`, la clave de ordenación degenera en la de Dijkstra y A\* *es* Dijkstra, pop a pop. Todo el capítulo es una consecuencia de esa frase.

## 23.4 Primera solución

La solución ingenua ya la tienes escrita: `dijkstra_path(store, origen, destino, &peso)`. Es óptima, está testeada y termina en cuanto el destino sale del heap. En la trampa devuelve el camino correcto (coste 9.0) tras expandir 13 nodos.

El intento naïve de «dirigir» la búsqueda también lo escribiría cualquiera: si la brújula es buena, ordena el heap **sólo por h** y ve directo al destino (greedy best-first). Y funciona… hasta que no funciona: una arista cara «que acerca» le gana a un desvío barato, porque `g` no cuenta para nada en la ordenación. El greedy es la sobre-estimación llevada al extremo: confía *tanto* en la corazonada que ignora lo andado.

## 23.5 Sus límites

Las dos soluciones ingenuas fallan por extremos opuestos y complementarios:

1. **Dijkstra**: toda la información sobre lo andado (`g`), ninguna sobre lo que queda. Paga el grafo entero en el peor caso: 13 expansiones donde hacían falta 10; 7 donde hacían falta 3.
2. **Greedy por h**: toda la información sobre lo que queda, ninguna sobre lo andado. Rápido y **sin garantía de optimalidad** — el mismo defecto que tendrá cualquier heurística que sobre-estima, como veremos en la sección 23.8.

La ley del todo-o-nada: o exploras en círculo, o pierdes el óptimo. Falta la idea que mezcle ambas informaciones *sumándolas* — y que sepa qué garantía conserva esa suma. Esa idea es A\*, y en una base de datos necesita resolver antes una pregunta de diseño: **¿de dónde sale h?**

## 23.6 Solución evolucionada

### El trait `Heuristic`: la heurística la aporta el CALLER

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap23_a_estrella.rs`. El corazón del diseño es un contrato de tres líneas:

```rust
pub trait Heuristic {
    fn estimate(&self, store: &dyn GraphStore, node: NodeId) -> Result<f64, PathError>;
}
```

¿Por qué un trait y no una closure `Fn(NodeId) -> f64`, que sería menos ceremonia? Tres razones, y ninguna es estética:

1. **Las heurísticas interesantes tienen estado y están ligadas a un destino.** La euclídea recuerda las coordenadas del destino y los nombres de las props (`x`, `y`); una de landmarks (sección 23.11) llevaría tablas de distancias precalculadas. Un struct con campos lo expresa sin esfuerzo; una closure tendría que fabricar ese estado con `Rc<RefCell<…>>`.
2. **Una closure no podría leer el store.** `estimate` recibe `&dyn GraphStore` porque la euclídea necesita las props *del nodo que se está estimando* — datos que no están en la closure sino en el grafo. La alternativa sería capturar el store prestado, y entonces esa closure queda casada con ese store: no podrías pasarle otro. Es la PRIMERA VEZ que un algoritmo de la Parte V lee datos del **NODO** (todo el cap. 22 leyó aristas): por eso la firma es lo que es.
3. **El contrato se valida en un sitio.** `a_star` revisa cada estimación que usa (finita, no negativa); si cada caller se fabricara su closure, ese contrato se repartiría por el código.

Y ojo: el trait no cierra la puerta al caso ad-hoc. Los tests definen `Fija(Vec<f64>)` —una tabla de valores por nodo— en tres líneas, y con ella construyen los casos patológicos del capítulo (inconsistente, sobre-estimada, negativa). El patrón es el mismo que `WeightSource` en el cap. 22: el algoritmo fija la maquinaria; el caller aporta la política.

### `ZeroHeuristic`: h≡0, o por qué un test no es una esperanza

```rust
pub struct ZeroHeuristic;   // h(n) = 0.0 para todo n
```

La heurística nula es admisible (0 ≤ coste real, siempre), consistente (0 ≤ w + 0, siempre)… y completamente inútil para dirigir nada. Su valor es doble, y los dos son de ingeniería, no de teoría:

- **Es la línea base**: contra ella se mide cuánto ahorra una heurística real (`expanded` con h≡0 = trabajo de Dijkstra).
- **Es un test de equivalencia exacta.** Con h≡0, la clave del heap `Reverse<(Cost(f), Cost(g), NodeId)>` degenera en `(Cost(g), NodeId)` — *exactamente* la de Dijkstra. No «parecida»: la misma. Por eso el test `heuristica_cero_es_dijkstra_exactamente` exige MISMO camino, MISMO coste y **MISMO orden de pops** (`popped`, `expanded` y `relax_updates` idénticos). Si tu A\* suma mal la f, desempata distinto o relaja en otro orden, el test se pone rojo. Un test que compara sólo el coste aceptaría una implementación que «acierta» explorando distinto; un test de orden de pops no perdona nada.

Ese es el sentido de la frase del modelo mental: A\* contiene a Dijkstra como caso degenerado, y ese caso degenerado es un **freno de regresión**, no una curiosidad.

### `EuclideanHeuristic`: coordenadas como props de nodo

La heurística canónica del capítulo: la distancia en línea recta de cada nodo al destino, leyendo las coordenadas de las props del nodo:

```rust
let h = EuclideanHeuristic::new(&store, destino, "x", "y")?;
// estimate(n) = hypot(x_n − x_dest, y_n − y_dest)
```

Tres decisiones escondidas aquí:

1. **El destino se liga al construir** (`new` lee y valida SUS coordenadas eager — son 2 props, fallar antes de empezar es más barato que fallar en medio de la búsqueda). Por eso `estimate` no recibe el destino: una heurística apunta a UN destino.
2. **Semántica estricta, la misma que `edge_weight` en el cap. 22**: prop ausente o `Null` → `PathError::MissingCoordinate { node, prop }`; tipo no numérico o Float no finito → `InvalidCoordinate`; un `Int` se promociona (coordenadas `Int(3)`/`Int(4)` dan la hipotenusa 5.0, el triángulo 3-4-5 del test). Un grafo schemaless delata sus huecos *cuando se pisan*, no antes.
3. **`hypot` y no `((dx*dx)+(dy*dy)).sqrt()`**: `f64::hypot` calcula la raíz sin desbordes intermedios y garantiza resultado ≥ 0 y finito con entradas finitas — que ya validamos. h nunca llega al heap con un NaN (ver la validación de `h_of` abajo).

¿Por qué es admisible? Porque **una carretera nunca es más corta que la línea recta**: si los pesos son distancias en las mismas unidades que las coordenadas, cada arista satisface `w(u,v) ≥ dist_recta(u,v)`, y por la desigualdad triangular ninguna ruta puede ser más corta que la recta al destino. Admisible *por construcción*, no por fe. Guarda esa condición de unidades: es la bomba de la sección 23.8.

### El algoritmo: la misma maquinaria, otra clave

`a_star(store, origen, destino, weight, heuristic)` reutiliza del cap. 22 *tal cual* la sanidad eager (`validate_edge_weights`, extraída a `pub(crate)` para compartirla — refactor puro), `WeightSource`/`edge_weight`, `Path`, `PathError`, `PathStats` y la reconstrucción por predecesores. Lo nuevo son cuatro detalles:

```rust
let mut heap: BinaryHeap<Reverse<(Cost, Cost, NodeId)>> = BinaryHeap::new();
// (f, g, nodo): f = g + h prioriza; g y nodo desempatan (determinismo)

while let Some(Reverse((_, Cost(g_u), u))) = heap.pop() {
    if g_u > g[u] { continue; }        // entrada obsoleta: su g ya fue superado
    stats.expanded += 1;                // pop VIVO: esto es lo que se mide
    if u == dest { break; }            // válido con h admisible
    for eid in store.out_edges(u) { /* relajar como en cap. 22 */ }
}
```

1. **La clave es `(f, g, nodo)`, no un `f64`**: el heap exige `Ord`, y `f64` no lo implementa precisamente por los NaN. El newtype `Cost` del cap. 22 devuelve; los desempates por `g` y luego por id dan determinismo.
2. **Entradas obsoletas**: cuando un nodo mejora su `g`, su entrada vieja *sigue* en el heap (los `BinaryHeap` no borran). Al salir, su `g` queda comparado contra el vigente (`g_u > g[u]`) y se descarta sin expandir. Comparación exacta: ambos valores salen de las mismas sumas.
3. **Re-apertura, no `settled`**: aquí está la diferencia estructural con Dijkstra. Dijkstra puede marcar nodos como definitivos porque h≡0 es *trivialmente consistente* y un nodo expandido jamás mejora. Con una h inconsistente, un nodo ya expandido PUEDE mejorar su `g` después… y hay que re-expandirlo o pierdes caminos. Por eso `stats.expanded` puede superar el número de nodos: es el precio medido, no escondido, de tolerar heurísticas imperfectas.
4. **Caché de estimaciones** (`h_cache`, NaN = «sin estimar»): la heurística se consulta a lo sumo UNA vez por nodo — la euclídea si no reelería dos props por cada inserción en el heap.

Y el criterio de parada, enunciado con precisión: cuando el destino sale del heap como entrada *viva*, su `g` es óptimo **si h es admisible** — cualquier camino aún por descubrir pasa por un nodo del heap cuyo `f` es cota inferior de su coste, y todos eran ≥ `f(destino)`. Con h≡0 esto es literalmente el argumento de Dijkstra del cap. 22.

### La validación honesta: qué se revisa, qué se documenta, qué se diagnostica

Ésta es la decisión de diseño más importante del capítulo, y es una decisión sobre *costes*. El cap. 22 validó eager los pesos negativos porque era O(E) y saltársela producía respuestas mentira. Aquí la escala manda:

| Propiedad | Coste de verificarla | Decisión |
|---|---|---|
| Pesos no negativos | O(E), herencia cap. 22 | Eager, misma función |
| h finita y ≥ 0 | 1 comparación por estimación (cacheada) | Se revisa SIEMPRE (`NonFiniteHeuristic`, `NegativeHeuristic`) |
| **Admisibilidad** (h ≤ coste real) | ¡Resolver Dijkstra hacia el destino! | **No verificable** — se documenta, el riesgo se DEMUESTRA en tests |
| **Consistencia** (h(u) ≤ w(u,v)+h(v)) | O(E) + ≤2V estimaciones, LOCAL | Utilidad `check_consistency`, diagnóstico opcional |

- **h finita y ≥ 0 sí es negociable que se revise**: un NaN rompería el orden total de `Cost` y haría *panic dentro del heap* — el peor lugar del mundo para depurar. Un h negativo casi siempre es un bug del caller y rompe el criterio de parada. La función `h_of` es la aduana (y de paso, la caché):

```rust
fn h_of(heuristic: &dyn Heuristic, store: &dyn GraphStore,
        cache: &mut [f64], node: NodeId) -> Result<f64, PathError> {
    if cache[node].is_nan() {                    // NaN = "todavía no estimada"
        let v = heuristic.estimate(store, node)?;
        if !v.is_finite() { return Err(PathError::NonFiniteHeuristic { node, value: v }); }
        if v < 0.0      { return Err(PathError::NegativeHeuristic  { node, value: v }); }
        cache[node] = v;                          // ≤ 1 consulta por nodo
    }
    Ok(cache[node])
}
```

- **La admisibilidad NO se puede verificar**: saber si `h(n) ≤ coste real de n al destino` exige conocer ese coste real… que es exactamente el problema que estás resolviendo. Verificarla costaría más que la propia búsqueda. Así que el contrato se **documenta** y el riesgo se **demuestra** (tests: h sobre-estimada y unidades mezcladas ⇒ camino subóptimo *sin ningún error*).
- **La consistencia sí es local**, y por eso existe `check_consistency`: una pasada por las aristas comprobando `h(u) ≤ w(u,v) + h(v)`. Devuelve `InconsistentHeuristic { edge, h_from, bound }` señalando la PRIMERA arista culpable. Y fíjate en la asimetría deliberada: es una utilidad de diagnóstico, **no un requisito** — A\* no la exige, porque rechazar una heurística inconsistente sería rechazar respuestas correctas (con h admisible-inconsistente, la re-apertura conserva el óptimo; el test lo mide: **5 expansiones para 4 nodos**, porque A se expande dos veces —con g=4 y luego con g=3.6—, y el coste 4.6 sigue siendo el óptimo que da Dijkstra).

## 23.7 El hito: rutas sobre una red de ciudades

El test `hito_del_brief_rutas_sobre_una_red_de_ciudades` monta la red del capítulo con coordenadas estilizadas en km (Madrid en el origen, Zaragoza a (190,130), Barcelona a (380,180)…) y carreteras siempre algo más largas que la recta. La pregunta: Madrid→Barcelona.

- El directo cuesta 460. La ruta por Zaragoza, 240 + 200 = **440**: ésa es la óptima, y la encuentra cualquiera de los dos algoritmos.
- Dijkstra expande **7 nodos**: asienta todas las ciudades — Valladolid, Bilbao, Valencia, Sevilla incluidas — porque ordena por `g` y el círculo no distingue.
- A\* con la euclídea expande **3**: Madrid, Zaragoza, Barcelona. La recta desde Madrid a Barcelona (≈ 420 km) ya adelanta que Valladolid y Sevilla están *lejos* del destino: sus `f` nacen altísimos y no salen del heap. El resto del mapa ni se mira.

```
              Bilbao                    Barcelona ● ← destino (3ª y última)
               |                          ▲
           Valladolid          Zaragoza ●──┘ ← 2ª: la recta la señala
               |                          ▲
             Sevilla          Valencia    │
                                │         │
                             Madrid ●─────┘ ← 1ª
        (Dijkstra: las 7 · A*: 3 · mismo camino, mismo coste 440)
```

Ese «7 vs 3» es `PathStats::expanded`, la métrica que este capítulo añadió al struct del cap. 22 (y que `dijkstra_impl` también incrementa). ¿Qué compra? Hace **visible** el ahorro de la heurística: sin ella, «A\* es mejor» es una fe; con ella, es un número que puedes `assert_eq!`. Y además delata la re-apertura (`expanded` > nodos tocados = hay inconsistencia). En el grafo-trampa de la sección 23.2, la misma métrica dice 13 vs 10: Dijkstra cae en la trampa barata que se aleja; A\* ve que esos nodos están lejísimos del destino y no los toca.

## 23.8 El bug didáctico estrella: kilómetros contra minutos

Éste es el error que quiero que te lleves tatuado. Test `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad`: coordenadas en km, pesos en **minutos**. Autopista directa de 200 km con tráfico (200 min) contra un desvío rápido por el norte (98 + 67 = 165 min, el óptimo real).

La euclídea sigue siendo «válida en forma»: finita, ≥ 0, geometría impecable. Pero la recta de 139 km ya no acota *minutos* — sobre-estima. Y entonces, sin error, sin warning, sin pánico:

- Dijkstra (que no usa h) devuelve el óptimo: **165** por el desvío.
- A\* devuelve **200** por la directa, camino `[0, 2]` perfectamente válido — encadena, las aristas existen, el coste suma — sólo que subóptimo.

`h` hundió al desvío con «139 km que no son minutos» y premió la directa. Es el mismo síntoma que la sobre-estimación deliberada del otro test (h(1)=10 ⇒ A\* responde 3.0 donde el óptimo es 2.0): **A\* no miente sobre el camino que devuelve — miente sobre su optimalidad, y en silencio.** Por eso la admisibilidad no se «verifica y santifica»: se entiende o se sufre.

¿Y cómo se caza? Con el diagnóstico local: `check_consistency` señala la arista culpable, la del desvío final — `h(1) ≈ 139.28 > w + h(2) = 67 + 0`. La primera arista cuya desigualdad triangular se rompe en unidades. Un O(E) que contesta lo que la búsqueda jamás te dirá.

## 23.9 Prueba de fuego

Los tests de este capítulo no afirman «funciona»: afirman números. Ejecútalos (`cargo test -p vol2-liradb`) y léelos como una tabla de garantías:

| Test | Lo que demuestra |
|---|---|
| `heuristica_cero_es_dijkstra_exactamente` | h≡0 ⇒ MISMO camino, coste, pops, expanded, relax_updates que Dijkstra |
| `euclidea_mismo_coste_con_menos_expansiones` | Trampa: 13 vs 10 expansiones, mismo coste 9.0, y `check_consistency` ok |
| `hito_del_brief_rutas_sobre_una_red_de_ciudades` | Madrid→Barcelona: 7 vs 3, coste 440.0 por Zaragoza |
| `admisible_no_consistente_reexpande_y_sigue_optimo` | 5 expansiones para 4 nodos, óptimo 4.6 intacto, `check_consistency` señala B→A |
| `sobre_estimacion_devuelve_suboptimo_demostrando_el_riesgo` | 3.0 vs 2.0 **sin error**; el camino devuelto es válido |
| `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad` | 200 vs 165 silencioso; `check_consistency` delata la arista |
| `coordenadas_ausentes_invalidas_o_no_finitas` | Missing/InvalidCoordinate; Int promociona (3-4-5 ⇒ 5.0) |
| `heuristica_negativa_o_no_finita_es_rechazada` | NaN ⇒ error tipado (sin él: panic en `Cost::cmp`) |

¿Qué pasaría si te saltas este capítulo? Dos síntomas detectables: tus rutas punto-a-punto escalan con todo el grafo (el círculo del radar), y el día que copies una heurística con unidades distintas a tus pesos obtendrás rutas malas **sin ningún síntoma** — ni error, ni test rojo. Sólo `check_consistency`, que no sabrás ejecutar, te lo habría dicho.

## 23.10 Qué hemos sacrificado

1. **La tabla single-source**: A\* sólo responde punto-a-punto. El sesgo hacia el destino es toda la gracia, y las distancias intermedias que va fijando NO están garantizadas (con h inconsistente, ni las de los nodos tocados). ¿Distancias desde un origen a todos? `dijkstra` del cap. 22, sin discusión.
2. **Cero re-expansiones**: tolerar h inconsistente cuesta re-abrir (5 expansiones para 4 nodos). Una implementación con `settled` sería más rápida… y mentiría sobre qué heurísticas admite.
3. **No hay `check_admissibility`**: no es una omisión, es una imposibilidad con presupuesto. La hermana local existe; la global no puede.
4. **La euclídea exige coordenadas y unidades coherentes**: grafos sin geometría (redes sociales, grafos de dependencias) no tienen recta que medir. Para ellos quedan las heurísticas de tabla — y los landmarks de la sección siguiente.

## 23.11 Cómo lo hace una BBDD real

- **Neo4j GDS** ofrece `gds.shortestPath.astar`: la heurística es la distancia entre las coordenadas **`latitude`/`longitude` que deben vivir como propiedades del nodo** — exactamente nuestra `EuclideanHeuristic` con otros nombres de props, y con la misma condición implícita de que el coste sea una distancia comparable.
- **pgRouting** expone `pgr_aStar` con parámetros `heuristic`, `factor` y `epsilon` y columnas x1/y1/x2/y2: la euclídea multiplicada por un factor que el usuario puede *subir* — es decir, sobre-estimación deliberada y configurable, cambiando óptimo garantizado por velocidad. Y su documentación/comunidad advierten del revés real: `pgr_astar` puede salir *más lento* que `pgr_dijkstra` si la heurística es mala (¡nuestro h≡0 de nuevo, pero en producción!). La moraleja de este capítulo vive en sus issues.
- **GraphHopper** (el motor de rutas detrás de muchos servidores de mapas) usa **ALT**: *A\*, Landmarks, Triangle inequality* — se eligen unos pocos nodos «hito» (landmarks), se precalculan sus distancias a todo el grafo, y `h(n) = máx |d(L,t) − d(L,n)|` sobre los hitos acota por la desigualdad triangular. Es la heurística de nuestro ejercicio experto, con pedigree: Goldberg y Harrelson la publicaron en SODA 2005 («A\* Search Meets Graph Theory»). Cuando NO hay coordenadas — o las rectas mienten, como con ferris y túneles — los landmarks son la euclídea del pobre y del rico a la vez.
- **Kùzu** hoy resuelve `SHORTEST` con recursive joins semánticos (lo veremos en el cap. 26) sin heurística geométrica: un recordatorio de que A\* es una herramienta para preguntas *métricas* punto-a-punto, no para todo patrón.

## 23.12 Lo que te llevas

- **f = g + h**: la clave del heap mezcla lo andado y lo que queda; h sólo cambia el ORDEN de los pops.
- **h≡0 ⇒ A\* ES Dijkstra**, pop a pop — y por eso es un test de regresión, no una curiosidad.
- **Admisible** (h ≤ coste real: global, NO verificable sin resolver el problema) ≠ **consistente** (local, O(E) con `check_consistency`). Admisible basta para el óptimo; consistente evita re-abrir.
- **Re-apertura** en vez de `settled`: con h inconsistente se re-expande y el óptimo se conserva — medido en `expanded`.
- **La euclídea es admisible por construcción** sólo si pesos y coordenadas están en las mismas unidades; mezclar km y minutos produce subóptimos **en silencio** (200 vs 165).
- **`expanded`** hace visible el ahorro: 13 vs 10 en la trampa, 7 vs 3 en Madrid→Barcelona.
- La heurística es un **trait ligado al destino** que lee props de NODO del store — la primera vez que la Parte V mira datos del nodo.

## 23.13 Ojo, cuidado con…

- **Confundir admisible con consistente**: la consistencia implica admisibilidad (con h(dest)=0); al revés no. Admisibilidad = óptimo; consistencia = además, sin re-expansiones.
- **Sustituir `h` por el coste real «ya calculado» de una ejecución anterior**: si el grafo cambió (¡grafo mutable!), tu h sobre-estima y el silencio vuelve.
- **Contar `popped` como trabajo**: los pops obsoletos no expanden; lo que cuesta es `expanded`. Son métricas distintas con nombres parecidos.
- **Validar TODAS las coordenadas al empezar**: el destino se valida eager (2 props); el resto se delata al pisarlo. Validar O(V) para una ruta que toca 3 nodos es pagar el grafo entero otra vez — el pecado del radar.

## 23.14 Pin de batalla

> *«Una heurística que miente hacia arriba no acelera tu búsqueda: te vende un camino peor sin decirte nada. La admisibilidad no se verifica — se entiende.»*

## 23.15 Si solo lees 30 segundos

Dijkstra explora en círculo porque su heap sólo conoce `g`, lo andado. A\* ordena por `f = g + h`, donde `h(n)` es lo que el CALLER sabe («¿cuánto queda?») — en LiraDB, un trait `Heuristic` ligado al destino que lee props de nodo (`EuclideanHeuristic` con `x`/`y`). Con h **admisible** (nunca sobre-estima) el primer pop vivo del destino es óptimo; con h **consistente** ni siquiera re-abre nodos; con h sobre-estimadora —o con unidades mezcladas— el resultado es subóptimo **sin error**, y sólo `check_consistency` (O(E), local) te señala la arista culpable. El ahorro se mide en `PathStats::expanded`: 13→10, 7→3.

## 23.16 Una historia pequeña

Cuando Ana montó la red de ciudades para el hito, reutilizó las coordenadas de un mapa viejo — en kilómetros — pero los pesos los sacó de un servicio de tráfico que devolvía **minutos**. «A\* con euclídea, esto vuela», dijo, y voló: Madrid→Barcelona en 3 expansiones. Sólo que por la autopista directa. Dijkstra, con el mismo grafo, encontraba la ruta por el desvío rápido, 35 minutos antes. Ningún test fallaba — el camino de Ana encadenaba, sumaba, existía. Tardó una tarde en caer en la cuenta de que la heurística hablaba kilómetros y el grafo escuchaba minutos, y cinco minutos en confirmarlo: `check_consistency` señaló la arista del desvío y su `139.28 > 67` fue la confesión. Desde entonces, en LiraDB, toda heurística nueva pasa por el túnel de consistencia antes de estrenarse. No porque sea obligatorio. Porque es silencioso.

## Ejercicios resueltos

**1. ¿Por qué el test de h≡0 compara también el ORDEN de pops y no sólo el coste?**

Porque la clave del heap con h≡0, `Reverse<(Cost(g+0), Cost(g), NodeId)>`, se ordena exactamente como la de Dijkstra, `(Cost(g), NodeId)`: mismas prioridades, mismos desempates. Comparar sólo camino y coste dejaría pasar una implementación que suma mal `f` o desempata por otra cosa pero «acierta» el resultado en ese grafo. Exigir `popped`, `expanded` y `relax_updates` idénticos convierte la equivalencia en exacta: cualquier desviación del orden es un rojo. Es el test de equivalencia más fuerte que se puede pedir sin leer el heap paso a paso.

**2. Con h admisible pero inconsistente, ¿por qué re-abrir nodos conserva el óptimo?**

Porque la garantía de parada no depende de que un nodo expandido sea definitivo: depende de que, al salir el destino como entrada viva, todo camino por descubrir tenga una cota inferior `f ≥ f(destino)`. Esa cota sólo exige admisibilidad (h nunca promete de menos). La inconsistencia hace que un nodo expandido pueda mejorar su `g` después; si lo marcáramos `settled`, perderíamos esa mejora y con ella el camino óptimo que la atraviesa. Re-expandir paga el precio (5 expansiones para 4 nodos en el test) pero no toca la garantía. Marcar `settled` es una optimización válida sólo bajo consistencia — y por eso es una decisión, no un default.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el código ni el capítulo: (a) di qué degenera exactamente en la clave del heap cuando h≡0 y qué TRES contadores debe igualar A\* para que la equivalencia con Dijkstra sea exacta; (b) di cuál de las dos propiedades —admisible, consistente— puede verificar LiraDB en O(E), y qué tendría que resolver para verificar la otra. Verifica tus respuestas contra `heuristica_cero_es_dijkstra_exactamente` y la doc de `check_consistency`.

**Intermedio (analizar).** El túnel del doctest: A en (0,0), B en (3,4) — recta de 5 —, una carretera de 6 y un túnel de 4. Antes de ejecutar nada, razona: ¿es consistente la euclídea hacia B en ese grafo? ¿Es admisible? Si el camino óptimo de A a B pasa por el túnel, ¿qué puede devolver A\* — camino válido, coste correcto, o silencio? Monta el store, compruébalo con `a_star` + `dijkstra_path` + `check_consistency`, y explica la diferencia entre la propiedad global rota y la local rota.

**Experto (crear, con spacing del cap. 22).** Implementa `LandmarkHeuristic`: elige un hito L (¿Sevilla o Zaragoza? discute cuál acota mejor hacia Barcelona), precalcula con el `dijkstra` single-source del cap. 22 la tabla `d(L,·)`, y define `h(n) = |d(L,dest) − d(L,n)|` (la desigualdad triangular la hace admisible; el destino fíjalo en construcción, como la euclídea). Corre Madrid→Barcelona y compara `expanded` contra `ZeroHeuristic` (7) y la euclídea (3). Ejecuta `check_consistency` e interpreta lo que diga. Criterio de éxito: camino 440.0 idéntico, `expanded` medido, y una frase defendiendo tu elección de hito.

## Para profundizar

- **Hart, Nilsson y Raphael, «A Formal Basis for the Heuristic Determination of Minimum Cost Paths» (IEEE Trans. SSC, 1968)** — el paper de A\*, nacido del robot Shakey. Corto, legible, y las definiciones son las que usamos aquí.
- **Hart, Nilsson y Raphael, corrección en SIGART Newsletter 37 (1972)** — donde se aclara la distinción admisible/consistente que este capítulo explota.
- **Dechter y Pearl, «Generalized Best-First Search Strategies and the Optimality of A\*» (JACM, 1985)** — por qué A\* con h consistente es óptimamente eficiente entre quienes usan la misma h.
- **Goldberg y Harrelson, «Computing the Shortest Path: A\* Search Meets Graph Theory» (SODA, 2005)** — ALT: landmarks + desigualdad triangular, la evolución natural de este capítulo.
- **Docs de Neo4j GDS (A\* con latitud/longitud), de pgRouting (`pgr_aStar` y sus parámetros `heuristic`/`factor`/`epsilon`) y de GraphHopper (`landmarks.md`)** — las tres decisiones de producción sobre la misma teoría.

## Mini-diálogo: frente al mapa

> — Entonces A\* es Dijkstra más una corazonada.
>
> — Menos una mitad del mapa. La corazonada —h— sólo reordena el heap; el bucle, la relajación, el camino, todo es del capítulo 22. Por eso h≡0 ES Dijkstra, pop a pop: no es una casualidad, es el caso degenerado del diseño.
>
> — ¿Y si mi corazonada exagera?
>
> — Entonces llega antes y por el camino equivocado, y no te enteras: el camino que te devuelve existe, encadena, suma. Se llama subóptimo silencioso, y es el motivo de que exista `check_consistency`. Dijkstra no se equivoca así porque no promete nada que no haya andado. La heurística es la primera pieza de LiraDB que habla del futuro — y hablar del futuro tiene reglas.
>
> — ¿Y la regla?
>
> — Nunca prometas menos de lo que queda. En km, si el grafo mide en km.

---

*(Próximo capítulo: 24 — Centralidad y PageRank. A\* necesitaba UN destino para saber hacia dónde ir; ahora la pregunta se invierte: cuando no hay destino sino importancia repartida, ¿quién manda a quién? La Parte V deja de buscar caminos y empieza a puntuar nodos.)*
# Capítulo 24 — Centralidad y PageRank

> *«Un enlace no es un voto igualitario: es la reputación del que enlaza, repartida entre sus salidas.»*

## 24.0 La anécdota de la esquina

En 1996, Larry Page llegó al doctorado de Stanford con una idea que sonaba académica: estudiar el **grafo de enlaces** de la web. Su director, Terry Winograd, le sugirió el ángulo que lo cambiaría todo: en ciencia, un artículo vale por sus citas — y un enlace ES una cita. La web entera, vista así, es el mayor grafo de citas jamás construido. Con Sergey Brin —y con Rajeev Motwani y Winograd como coautores— el proyecto, primero llamado BackRub, se convirtió en un buscador llamado Google, y en enero de 1998 la idea quedó por escrito: «The PageRank Citation Ranking: Bringing Order to the Web», un informe técnico de Stanford presentado también en la conferencia WWW7 (Brisbane, abril de 1998).

El corazón del paper es una imagen que vale un buscador: el **surfer aleatorio**. Alguien que navega pulsando enlaces al azar para siempre. ¿En qué páginas pasará más tiempo? La respuesta a esa pregunta de paseo infinito es el PageRank. Y dos detalles que hoy siguen vigentes: el factor de amortiguación `d = 0.85` que usaremos tal cual, y el nombre — un juego de palabras con el apellido Page.

La historia tiene un tercer acta verificable: la **patente US 6,285,999**, «Method for node ranking in a linked database». Solicitada el 9 de enero de 1998 por el propio Page… y asignada a Stanford, no a Google: la universidad la licenció en exclusiva a la empresa que sus dos estudiantes de doctorado fundaron el 4 de septiembre de 1998. La patente se concedió el 4 de septiembre de 2001 — tres años después, al día. En 2018 expiró. El IEEE la conmemora como hito («PageRank and the Birth of Google, 1996–1998»).

Este capítulo construye PageRank — y las cuatro familias de centralidad que lo rodean — sobre el grafo persistente de LiraDB. Y lo hace empezando por el final: primero veremos el algoritmo que PageRank vino a arreglar, y veremos fallar.

## 24.1 Objetivo

Al terminar este capítulo sabrás responder, sobre el grafo REAL de tu store, la pregunta «¿quién es importante aquí?» — y sabrás por qué esa pregunta tiene cinco respuestas distintas, cada una con su coste. Habrás construido, en `cap24_centralidad.rs`:

1. **Las familias clásicas**: grado, closeness (con corrección de Wasserman-Faust) y betweenness (algoritmo de Brandes 2001).
2. **El «antes» honesto**: la centralidad eigenvector — implementada con sus DOS fallos como tests, porque son la mejor demostración de por qué existe el siguiente.
3. **PageRank**: damping en (0,1) ABIERTO, masa colgante redistribuida uniformemente, convergencia L1 con historial por iteración — y su variante personalizada (`Teleport::Personalized`), la costura que el Vol. III usará para GraphRAG.

Una regla del capítulo, heredada del brief: estas familias no están implementadas con optimización industrial — están para explicar cómo PIENSA cada una, y el coste se mide (`CentralidadStats`), no se declama.

## 24.2 Problema

Mira el grafo demo que arrastras desde el capítulo 20:

```
   KNOWS:      0 → 1 → 2 → 0        (Ana, Bo, Carla: triángulo)
               3 → 3                (Dani: self-loop)
   LIVES_IN:   0 → 4,  1 → 5        (Madrid, Sevilla: sumideros)
```

Pregunta: ¿quién es la persona «más importante»? La respuesta ingenua ya la tienes: contar aristas. Pero el grado es una medida LOCAL: cuenta vecinos y no pregunta quién los aporta. En la web de 1998 esto era literalmente un campo de batalla: los buscadores contaban palabras y enlaces, así que los spammers rellenaban páginas de ambas cosas. La genialidad de Page y Brin fue darse cuenta de que **un enlace lo publica OTRO**: tú puedes repetir mil palabras, pero no puedes obligar a mil webs a enlazarte. El enlace ajeno es una señal difícil de falsificar.

Falta, aún así, la segunda mitad del insight: no todos los enlaces valen igual. Ser citado por un desconocido es algo; ser citado por una fuente importante es otra cosa. Quiero una métrica donde **la importancia se propague**: un nodo es importante si nodos importantes lo apuntan. Y una condición de ingeniería: el cálculo debe correr sobre `&dyn GraphStore`, el grafo persistente que llevas desde el capítulo 8, no sobre una matriz ad-hoc.

Y aquí un problema práctico que condiciona todo el código: los algoritmos iterativos van a tocar la adyacencia muchísimas veces — decenas de iteraciones, todas las aristas cada una. Si cada acceso pasa por `out_edges` + `get_edge`, pagamos el store completo en CADA ronda.

## 24.3 Modelo mental

El **surfer aleatorio**, con tres reglas:

1. **Pulsa enlaces**: desde la página actual, elige un enlace saliente al azar. Si la página tiene 3 enlaces, cada uno recibe 1/3 de su «voto».
2. **Se aburre**: con probabilidad 1−d (el `damping` d es la probabilidad de seguir pulsando), no pulsa nada y **teletransporta** a una página elegida según el vector de teleport (uniforme: cualquier página, 1/n).
3. **Los callejones teletransportan**: si cae en una página sin enlaces salientes (nodo colgante, dangling), se teletransporta igualmente.

```
   UNA iteración de PageRank (d = 0.85):

   x[u] = (1-d)·t[u]              ← teleport: la renta básica (1/n)
        + d·D/n                   ← cuota de la masa colgante
        + d·Σ_{v→u} x[v]/grado(v) ← votos: cada vecino reparte su
                                     importancia entre sus salidas

   Σ_u x[u] = 1  en CADA iteración  (la masa no se crea ni se destruye)
```

PageRank(u) = fracción de la eternidad que el surfer pasa en u. La «importancia» es una **masa** que fluye por los enlaces: entra por los votos, sale repartida entre las salidas, y el teleport la devuelve al sistema. Este modelo de masa es el que hace legibles todas las decisiones del capítulo: el delta de convergencia es «cuánta masa se movió», el dangling es «masa a punto de fugarse», el damping es «con qué fuerza vuelve la masa al sistema».

Comprueba el modelo en miniatura: una estrella de tres hojas apuntando a un centro. Las hojas no reciben votos — sin teleport, su importancia se desangra a 0 y el centro lo acumula TODO (es el fallo 1 del §24.5, y el test lo clava: centro > 0.999999, hojas < 1e-6). Con teleport, cada hoja cobra su renta básica de 1−d repartida en 1/n y el cuadro deja de ser tan despótico. Y si una hoja tuviera un enlace saliente hacia otra hoja, su voto valdría poco (pobre votante)… pero el del centro, repartido entre SUS salidas, vale por su prestigio. Voto = prestigio del emisor dividido por sus salidas: ni más ni menos.

El momento ¡ajá!: la importancia no se declara — se FLUYE. Y toda la aritmética del capítulo es contabilidad de ese flujo.

## 24.4 Primera solución

Primero la infraestructura común. Toda la Parte V corre sobre una **proyección** del store: `Proyeccion::proyectar(store, dir)` materializa una sola vez los nodos (ordenados por id: determinismo), un índice denso NodeId→posición (los huecos de `delete_node` quedan fuera del cálculo) y los vecindarios. Es el CSR del capítulo 14 en forma de memoria volátil: misma idea (adyacencia compactada, acceso O(1) por nodo), sin persistencia. ¿Por qué materializar? Porque el bucle de PageRank toca cada arista `iteraciones` veces: re-leer el store por ronda sería pagarlo O(iteraciones) veces. ¿Y por qué es PRIVADA y no ponderada? El guion pide las familias «para explicar»; el BFS de saltos basta. La versión con pesos — con la semántica estricta `edge_weight` del capítulo 22 — es una deuda explícita hacia el capítulo 26, que la saldó con `ProyeccionPonderada`.

Con dirección configurable: `GraphDirection::Out` (salientes), `In` (la transpuesta PURA de la salida, con aristas paralelas conservadas) y `Both` (la vista «no dirigida» como CONJUNTO: vecinos distintos y self-loop una sola vez — la convención del `Expand` UNDIRECTED del capítulo 20; sin dedup, un store simetrizado a mano contaría cada par doble).

Sobre esa base, las cinco familias del capítulo — cuatro «de consulta puntual» y una global. Las preguntas, para tenerlas juntas:

- **Grado** — «¿cuántos vecinos?». Una pasada, O(V+E), normalizado por n−1 (el máximo posible: estar conectado a todos). Con dirección: en el triángulo `0→1, 0→2, 2→0`, el nodo 0 tiene grado out 1.0 (2 de 2 posibles), pero grado in 0.5. Mismo grafo, dos preguntas: el que HABLA y el que es ESCUCHADO.
- **Closeness** — «¿a qué distancia estoy de todos?». Un BFS por nodo: O(V·(V+E)). En el camino simetrizado 0-1-2-3 sale el valor de libro: extremos 0.5 y centro 0.75 — `C = 3/Σd` con Σd = 6 en los extremos y 4 en el centro. Con la corrección de **Wasserman-Faust** para componentes desconectadas: `C(u) = ((r−1)/(n−1))·((r−1)/Σd)`, donde r es cuántos nodos alcanzas. Sin ella, en un grafo de dos 2-ciclos separados cada nodo daría 1.0 (¡perfecto dentro de su burbuja!) — con ella dan 1/3: penalizados por el mundo que NO alcanzan.
- **Betweenness** — «¿qué fracción de los caminos mínimos AJENOS pasan por mí?». Es la métrica del control: el nodo que está en TODOS los caminos entre otros dos es un cuello de botella — un aeropuerto hub, un router troncal, un corredor de información. En el camino 0-1-2-3, los intermedios 1 y 2 acumulan TODOS los pares: crudo 4, normalizado 2/3. En la estrella, el centro llega al máximo absoluto, 1.0: sin él, nadie habla con nadie. Aquí está la joya algorítmica del capítulo: **Brandes 2001**, con números en el §24.7.

Y la versión ingenua de la pregunta global: la **centralidad eigenvector**. «Soy lo que mis entrantes valen», sin ningún correctivo: `x_u ← Σ_{v→u} x_v`, iteración de potencia sobre la adyacencia cruda, normalizada en L2 cada paso. ¿Por qué la normalización, si «ensucia» el autovector? Porque la masa que escapa por los colgantes no vuelve: sin renormalizar, el vector entero se desangra hacia 0 y la iteración no tendría punto fijo que alcanzar. Con L2 por paso, al menos el vector conserva norma — y sus dos fallos quedan a la vista, que es justo lo que queremos de él. Caso borde honesto: un grafo sin aristas (autovalor 0, cualquier vector es autovector) devuelve el uniforme con `converged = true` sin iterar — no todo es drama.

## 24.5 Sus límites

El eigenvector crudo se rompe de DOS maneras en grafos dirigidos reales. No lo afirmamos: lo ejecutamos.

**Fallo 1: la masa se fuga por los colgantes.** Estrella con las hojas apuntando al centro: converge — pero las hojas mueren a 0 (`ev.score(hoja) < 1e-6`) y el centro se lo lleva todo (`> 0.999999`). ¿Y si el centro tuviera a su vez un sumidero favorito? La masa que entra en un nodo sin salidas SE ESCAPA del sistema: el flujo de importancia se desangra. En la cadena 0→1→2 lo ves en pequeño: la masa fluye río abajo, llega a 2… y desaparece del mundo. Iteración a iteración, el vector entero mengua — por eso el código renormaliza en L2 cada paso: para que al menos el vector conserve la norma mientras la ESTRUCTURA interna colapsa. En la web esto no es un caso borde: casi TODA página tiene enlaces salientes a alguna parte… y muchas (PDFs, imágenes, páginas abandonadas) no. Es la regla, no la excepción.

**Fallo 2: la oscilación periódica.** El grafo trampa: una cola que desemboca en un 3-ciclo (`0→1, 1→2, 2→3, 3→1`). La masa muere en la cola y lo que queda ROTA en el ciclo: en cada iteración el vector se desplaza un terzo de ciclo y al siguiente vuelve — oscila para siempre. El test lo dice sin rodeos: `eigenvector_centrality(&s, 100, 1e-9)` devuelve `converged = false` tras agotar las cien iteraciones, con `delta > 1e-9`.

```
   grafo: 0 → 1 → 2 → 3, con 3 → 1   (cola + 3-ciclo: 1→2→3→1)

   eigenvector (sin damping)          PageRank (d = 0.85)
   ─────────────────────────────      ─────────────────────────────
   la cola muere; la masa             la cola muere; la masa rota
   ROTA en el ciclo sin decaer:       AMORTIGUADA: en cada paso un
   it k:   pico en 1                  15 % se va al teleport y
   it k+1: pico en 2                  termina asentándose:
   it k+2: pico en 3, como en k       δ_k → 0 geométricamente
   δ_k ≈ constante > tol, nunca baja  (razón ≈ d·λ₂ < 1)
   converged = false                  converged = true
```

Si tu motor de ranking se rompe en un ciclo de tres nodos, no tienes un motor de ranking. Hace falta algo que repare AMBOS fallos: que devuelva la masa fugada y que amortigüe la rotación. Ese algo no es un parche numérico — es el damping.

El test que lo clava es tan corto que cabe entero — y es, en cuatro líneas, la tesis del capítulo:

```rust
// Cola + 3-ciclo: T→A, A→B, B→C, C→A.
let s = dirigido(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]);

let ev = eigenvector_centrality(&s, 100, 1e-9).unwrap();
assert!(!ev.converged);                  // agotó las 100 y sigue oscilando
assert!(ev.delta > 1e-9);

let pr = page_rank(&s, 0.85, 200, 1e-9).unwrap();
assert!(pr.converged);                   // MISMO grafo: converge
assert!((pr.total_mass() - 1.0).abs() < 1e-6);  // y la masa sigue ahí
```

## 24.6 Solución evolucionada

PageRank es el eigenvector con exactamente DOS arreglos quirúrgicos.

**Arreglo 1: el teleport (damping).** Con probabilidad 1−d, el surfer reaparece en una página del vector t. ¿Por qué repara la oscilación? Algebra lineal honesta, con nombre: **Perron-Frobenius**. La iteración de potencia converge al autovector dominante si todo lo demás es, en módulo, menor que 1. Un ciclo puro tiene autovalores complejos de módulo 1 (las raíces de la unidad: por eso rota sin decaer). La matriz de Google `G = d·M + (1−d)·t·1ᵀ` contrae TODOS los autovalores salvo el 1 por el factor d: con teleport uniforme, el segundo autovalor es exactamente d (Haveliwala y Kamvar, 2003, lo demostraron). Con d = 0.85, la masa de error se contrae un 15 % por iteración… pero se contrae SIEMPRE: la matriz es positiva (todas las entradas > 0, gracias al teleport que toca todo el mundo), luego primitiva, luego con un ÚNICO estacionario y convergencia garantizada — sin importar ciclos, colas ni componentes. Y por eso `damping ∈ (0,1)` ABIERTO por ambos extremos: d = 0 es teleport puro (una iteración, sin estructura — no hay grafo que rankear), y d = 1 es eigenvector PURO… que ya vimos fallar, y que por eso existe como función propia con sus tests de fallo. Los bordes no son valores válidos: son otros dos algoritmos. (Detalle técnico que ya nos costó un test: para excluir el 0 no sirve `Range::contains` — el inicio del rango es inclusivo. Comparación explícita.)

**Arreglo 2: redistribución uniforme de la masa colgante.** Un surfer en un nodo sin salidas teletransporta. ¿Por qué uniforme y no «se descarta y se renormaliza al final» (la variante «no-scale»)? Porque con redistribución la masa total es 1 EN CADA ITERACIÓN — un invariante que testeamos, no una esperanza — y el delta L1 conserva su lectura de probabilidad. La variante no-scale está documentada en el código como alternativa legítima: cambia el límite, no el procedimiento. Aquí no la implementamos: dos contratos semánticos para el mismo algoritmo es una trampa de lector.

El damping también paga otra deuda silenciosa: las **componentes desconectadas**. El teleport es lo único que conecta mundos que no se tocan — hace la cadena irreducible. Con teleport uniforme, cada componente recibe masa proporcional a su tamaño: dos 2-ciclos aislados se reparten 0.5 y 0.5; un 3-ciclo frente a un 2-ciclo, 3/5 y 2/5. Sin teleport no habría respuesta global posible: cada componente tendría su propio PageRank y ninguno sabría nada del otro.

Los EXTREMOS, medidos y no filosofados: con d = 0.01 el resultado es prácticamente el teleport uniforme (todos a 1/3 en la cadena con colgante, con margen 0.01); y a d fijo, subir d retrasa la convergencia — d = 0.99 necesita MÁS iteraciones que d = 0.5 para la misma tolerancia, porque la razón de contracción se acerca a 1 (`pagerank_damping_extremos` lo testea con las dos ejecuciones).

Sobre el núcleo compartido (`iteracion_de_potencia`), dos funciones públicas con una costura:

```rust
pub enum Teleport {
    Uniform,                          // PageRank global: 1/n
    Personalized(Vec<(NodeId, f64)>), // PPR: semillas ponderadas
}

pub fn page_rank(store, damping, max_iterations, tol)
    -> Result<PageRankResult, _>
{ /* mismo núcleo, t = Teleport::Uniform */ }

pub fn personalized_page_rank(store, seeds, damping, max_iterations, tol)
    -> Result<PageRankResult, _>
{ /* MISMO núcleo, t = Teleport::Personalized(seeds) */ }
```

¿Por qué el teleport es un parámetro y no está pegado al cálculo? Porque el PageRank personalizado no es una floritura: es un operador de RECUPERACIÓN. Su lectura cambia por completo: el global pregunta «¿qué es importante en general?»; el personalizado pregunta «¿qué es relevante PARA ESTE punto de partida?». Las semillas definen «el centro del mundo»: la masa que escapa por el damping vuelve a ELLAS. Con dos 2-ciclos desconectados y semilla en el nodo 0, la componente lejana queda a masa EXACTAMENTE 0 — fuera del mundo — y en la componente sembrada la solución a mano es `a = 0.15/(1−0.85²) ≈ 0.54`. El Vol. III (capítulo 51, GraphRAG) enchufará aquí su pregunta del usuario como `Teleport::Personalized` sobre el subgrafo de documentos. Si teleport y núcleo estuvieran acoplados, tendría que duplicar PageRank; tal como está, no toca el núcleo.

## 24.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs`. Los tests corren con `cargo test -p vol2-liradb tests_centralidad` (28, todos verdes). Recorremos las decisiones.

**Brandes, con números.** Betweenness(u) = Σ σ_st(u)/σ_st: la fracción de caminos mínimos entre pares ajenos que pasan por u. La versión ingenua corre un BFS POR PAR: V² pares × O(E) por BFS = O(V²·E)… y encima eso sólo da distancias — enumerar los caminos de cada par puede explotar exponencialmente (Brandes cita el estado previo como O(V³) y peor). Brandes 2001 lo reduce a V BFS con acumulación: en el BFS se cuentan los caminos (σ) y se anotan predecesores; después se recorren los nodos EN ORDEN INVERSO acumulando dependencias `delta[v] += (σ_v/σ_w)·(1 + delta[w])` — la recursión que evita enumerar caminos. Coste: O(V·E). Números: V = 1.000.000, E = 10.000.000 → por pares ~10¹⁹ operaciones (a 10⁹ op/s: siglos); Brandes ~10¹³ (del orden de horas). La normalización es la dirigida, 1/((n−1)(n−2)) — con `Both` sobre un grafo simetrizado reproduce EXACTO el libro no dirigido: camino 0-1-2-3 → intermedios a 2/3; estrella → centro a 1.0. Las aristas paralelas son caminos distintos (σ las cuenta).

**El bucle de PageRank.** Cae entero en `iteracion_de_potencia`: (1) sumar la masa de los colgantes; (2) sembrar el vector nuevo con teleport + cuota colgante; (3) repartir votos de 1/grado por cada arista saliente; (4) delta L1, history, y parar bajo tolerancia. Multigrafo, sutileza testeada: duplicar la arista 0→2 NO duplica el voto de 2 (también duplica el denominador)… pero le ROBA la mitad de la cuota al otro vecino: 2 sube, 1 baja (`pagerank_multigrafo_el_duplicado_roba_masa`).

Y cuando el grafo es simétrico, el sistema se resuelve A MANO — el mejor test es el que puedes verificar con lápiz. A↔B y A↔C (A reparte entre B y C; ambos le devuelven todo): por simetría x_B = x_C = y, con dos ecuaciones `x = (1−d)/3 + d·2y` e `y = (1−d)/3 + d·x/2`. Con d = 0.85: `y = (0.05 + 0.425)/1.85 ≈ 0.257` y `x = 1 − 2y ≈ 0.487`. El test `pagerank_ciclos_compartidos_solucion_a_mano` ejecuta y compara — EPS 1e-6.

**Por qué L1 y no max-delta.** El delta L1 (Σ|Δscore|) es la MASA TOTAL que se movió en la iteración: «¿cuánta masa falta por asentar?» — un número con lectura de probabilidad, y comparable entre grafos de distinto tamaño (1e-6 de masa es 1e-6 de masa con 6 nodos o con 6 millones). El max-delta (el cambio del nodo que más se movió) es más estricto por nodo, pero no significa nada como masa y su umbral cambia de sentido con el tamaño del grafo. Documentado y descartado.

**Por qué el history es contenido.** `PageRankResult.history` guarda el delta de CADA iteración. Ejecuta la cadena 0→1→2 (colgante al final) y míralo — esto es la salida real, no un esquema:

```text
$ page_rank(0→1→2, d=0.85, tol=1e-10)  →  converged=true, 33 iteraciones
history: 3.778e-1  2.676e-1  1.668e-1  4.726e-2  2.800e-2  1.931e-2
         7.134e-3  2.630e-3  2.063e-3  1.022e-3  2.897e-4  2.229e-4
         ... (monótono a la baja) ...
         3.670e-9  2.146e-9  1.492e-9  5.629e-10 1.998e-10 1.584e-10 8.006e-11
```

Tres lecturas en esos números. Primera: decrece MONÓTONO desde la segunda iteración (la primera mezcla el arranque) — el test lo exige término a término. Segunda: la razón entre consecutivos no es una constante limpia — oscila (el colgante devuelve la masa a saltos) — pero se mantiene acotada por debajo de 1: la contracción geométrica de Perron-Frobenius trabajando, con la huella del colgante encima. Tercera: el contraste es todavía más elocuente. En dos 2-ciclos simetrizados el teleport uniforme YA es el estacionario: converge en UNA iteración y `history = [0.0]`. El historial cuenta ambas historias — y en ningún caso tienes que creerte nada: ejecutas y ves.

**El coste, medido.** `CentralidadStats { bfs_runs, edges_scanned, iterations }`: closeness y betweenness reportan un BFS por nodo; PageRank acumula `iteraciones × E`. El test del camino lineal comprueba `edges_scanned` EXACTO (12 de proyección + 24 de BFS). La tabla del guion queda verificable:

| Familia | Pregunta | Coste | Stats |
|---|---|---|---|
| Grado | ¿cuántos vecinos? | O(V+E) | — |
| Closeness | ¿lo lejos que estoy de todos? | O(V·(V+E)) | bfs_runs = V |
| Betweenness | ¿por cuántos caminos paso? | O(V·E) | bfs_runs = V |
| Eigenvector | ¿quién me apunta? | O(iter·E) | iterations |
| PageRank | ¿dónde pasa la eternidad el surfer? | O(iter·E) | iterations |

**Cómo leer un resultado.** Los tres tipos devuelven más que números: `score(id)` (que responde `None` para ids inexistentes o borrados), `entries()` en orden de id, `ranking()` por score descendente con desempate por id (determinismo: dos ejecuciones dan el mismo ranking, siempre), y en PageRank la transparencia del cálculo: `converged`, `delta`, `history`, `damping` y `Display` — `PageRank(d=0.85, iteraciones=33, delta=8.01e-11, convergido=sí)`. La validación de entrada falla ruidosamente ANTES de iterar: `InvalidDamping` (con los bordes explicados en el propio mensaje de error), `InvalidTolerance`, `InvalidMaxIterations`, y para el teleport personalizado, `NegativeTeleportWeight` señalando el nodo, `ZeroTeleportMass` y `UnknownNode`. Una función que va a correr 500 iteraciones no puede descubrir en la 499 que la semilla no existía.

## 24.8 Prueba de fuego

El test que ES el capítulo, `eigenvector_no_converge_en_periodico_y_pagerank_si`: el MISMO grafo cola+3-ciclo donde eigenvector agota 100 iteraciones sin converger, `page_rank(&s, 0.85, 200, 1e-9)` converge con masa 1. Los DOS fallos del §24.5, con su cura al lado.

Después, el grafo demo — y su sorpresa pedagógica. KNOWS forma el triángulo 0→1→2→0 más el self-loop de Dani (3→3); LIVES_IN lleva de 0 y 1 a las ciudades 4 y 5. ¿Quién gana? **Dani, con 0.386** — por delante de todo el triángulo. Nosotros también pensamos que era un bug. No lo era: el self-loop le devuelve CADA voto (es su único vecino saliente), y encima cobra la cuota uniforme de la masa colgante de las ciudades, que no votan. Self-loop + masa colgante = trampa de acumulación. Las salidas reales, lado a lado:

```text
PageRank (d=0.85, 92 iteraciones):   Grado OUT (una pasada):
  n3 (Dani)   = 0.386                  n0 = 0.40   ← Ana
  n0 (Ana)    = 0.151                  n1 = 0.40   ← Bo
  n1 (Bo)     = 0.122                  n2 = 0.20
  n4 (Madrid) = 0.122                  n3 = 0.20   ← Dani, "casi marginal"
  n2 (Carla)  = 0.110                  n4 = 0.00
  n5 (Sevilla)= 0.110                  n5 = 0.00
```

Por GRADO, Dani es casi el más marginal (1/5 — sólo supera a las ciudades, que no votan); por PageRank, arrasa. Dos métricas, dos historias — y ninguna de las dos «la verdad». Fíjate también en los empates estructurales: n1 con n4, n2 con n5 — la simetría rotacional del triángulo arrastra a cada persona a empatar con la ciudad que alimenta. El ranking desempata por id (determinismo), pero el empate es información del grafo, no ruido.

Los invariantes, en todos los tests: `total_mass() = 1` (desviaciones de ~1e-15: redondeo de f64); damping fuera de (0,1) — incluido NaN, que no compara bajo `PartialEq` y por eso se testea con `matches!` — rechazado ruidosamente; semillas negativas, de masa cero o inexistentes rechazadas señalando el nodo culpable. Y `converged = false` es una RESPUESTA: una base de datos prefiere decir «no convergió» que devolver números casi-buenos en silencio.

Dos cierres de análisis sobre el mismo demo. Primero: personaliza en Madrid (semilla = nodo 4) y el grafo se RE-CENTRA — `ppr_vs_global_en_demo_graph`, salida real:

```text
   global           PPR semilla=Madrid(4)
   n3 = 0.386       n3 = 0.328   ← Dani se desinfla: pierde el teleport uniforme
   n4 = 0.122       n4 = 0.254   ← Madrid se duplica: el teleport vuelve a casa
   n0 = 0.151       n0 = 0.128   ← hasta Ana BAJA: fuera de órbita, fuera de masa
```

La ciudad sube, la trampa de Dani se desinfla al dejar de cobrar teleport uniforme, y hasta el nodo 0 (que la apunta) baja — quien no está en la órbita de la semilla pierde teleport sin ganar nada equivalente. Mismo grafo, mismo algoritmo, otro mundo — esa es la potencia del vector de teleport. Segundo: borra el nodo central de una cadena (`delete_node`) y el índice denso de la proyección excluye el hueco: los ids restantes siguen puntuando, el borrado responde `None` — no existe, no puntúa (`pagerank_huecos_tras_delete_node`). La proyección deriva del store; no lleva el grafo «en la cabeza».

## 24.9 Qué hemos sacrificado

1. **La proyección no ponderada.** El closeness ponderado (Dijkstra del capítulo 22 por cada origen) queda como deuda hacia el capítulo 26: cuando exista la proyección con pesos, el BFS de este capítulo se sustituye sin tocar nada más. Deuda declarada, no olvidada.
2. **Nada de optimización industrial.** Ni PageRank por bloques, ni betweenness aproximada por muestreo, ni GPU. El guion manda: familias para explicar, coste para medir.
3. **La variante no-scale de dangling.** Documentada, no implementada: dos contratos semánticos para un algoritmo es una trampa.
4. **Arranque asistido, acoplado.** El arranque es el propio teleport (uniforme o semillas): ahorra iteraciones de mezclado y es una línea — los arranques «inteligentes» (scores del grado) ganarían poco y oscurecerían el análisis.
5. **Empates no desempatados por significado.** El ranking desempata por id ascendente (determinismo); en el demo, 1 empata con 4 y 2 con 5 por simetría estructural — el orden entre iguales es convención.

## 24.10 Cómo lo hace una BBDD real

- **Neo4j GDS** expone `gds.pageRank` con la MISMA parametrización de siempre y los mismos números de hace 27 años: `dampingFactor` 0.85, `maxIterations` 20, tolerancia 1e-7. Y el PageRank personalizado NO es otra función: es el parámetro `sourceNodes`, que acepta pares (nodo, sesgo) — traducción literal de nuestro `Personalized(Vec<(NodeId, f64)>)`. Mismo núcleo, otro mundo: exactamente la costura de nuestro `Teleport`.
- **`gds.betweenness`** implementa Brandes… y ofrece una variante por MUESTREO para grafos grandes, porque hasta O(V·E) se queda corto en producción. Nuestro `CentralidadStats` mide lo que su muestreo recorta.
- **PPR como base de recomendación** es patrón clásico de la industria: el «Who to Follow» de Twitter se construyó sobre random walks con reinicio en el grafo de follows, y la personalización de Google News (2007) rankeó noticias con PageRank sobre el grafo usuario-noticia. La receta es siempre la misma: semilla = usuario/consulta, ranking = dónde pasa el surfer su tiempo.
- **Google**, por supuesto, ya no usa «el» PageRank de 1998: cientos de señales encima. Pero la decisión de diseño — medir la importancia como proceso global sobre el grafo de enlaces, no como recuento local — es la que ordenó la web.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en la cadena 0→1→2, ¿por qué el colgante 2 NO absorbe toda la masa si recibe el voto de 1 y no emite nada?
- *Intermedio*: ¿cómo cambiaría el history de un PageRank con d = 0.5 frente a d = 0.95 en el MISMO grafo? Predícelo y mídelo.
- *Experto*: ¿qué teleport reproduce la centralidad de grado como caso degenerado de PPR, y con qué d? (Pista: piensa qué pasa cuando d→0 y las semillas son TODOS los nodos con peso 1/n… y por qué aún así no es el grado.)

## 24.11 Lo que te llevas

- «Importante» tiene cinco respuestas con cinco costes: grado O(V+E), closeness O(V·(V+E)), betweenness O(V·E) (Brandes, no por pares), eigenvector/PageRank O(iter·E). Mídelo con `CentralidadStats`.
- El eigenvector crudo falla DOS veces: masa que se fuga por colgantes y oscilación periódica. Ambos fallos son tests.
- PageRank = eigenvector + damping (teleport) + redistribución colgante. La matriz se vuelve positiva ⇒ primitiva ⇒ converge SIEMPRE, geométricamente, razón ≈ d·λ₂ — y el `history` te lo enseña.
- Masa = 1 en cada iteración es un invariante testeado; el delta L1 es «masa que se mueve» — interpretable y comparable entre tamaños.
- `Teleport{Uniform, Personalized}` separa el MUNDO del NÚCLEO: el mismo código rankea la web entera o la órbita de una consulta. El capítulo 51 del Vol. III vive en esa costura.

## 24.12 Ojo, cuidado con…

- **Confundir damping con tuning**: no es velocidad, es reparación. d→1 converge MÁS lento (la razón se acerca a 1); d→0 devuelve el teleport. 0.85 es el equilibrio del paper, no una manía.
- **Olvidar la masa colgante**: quien calcula «a mano» con solo teleport+votos obtiene masa < 1 y no lo nota. `total_mass()` existe; úsalo.
- **Both no es out+in apilados**: es unión como CONJUNTO (vecinos distintos, self-loop UNA vez); `In` es la transpuesta pura. Mezclar ambas cosas fue un bug real de esta implementación.
- **Duplicar aristas esperando duplicar votos**: el duplicado roba masa a los OTROS vecinos del mismo origen. Semántica multigrafo: sutil, testeada.
- **Eigenvector en grafos dirigidos**: si necesitas el autovector de verdad (espectral), el Vol. I cap. 16 lo trata; aquí su papel es ser el «antes» honesto del PageRank.

## 24.13 Pin de batalla

> *«Sin damping, la importancia se pierde en los callejones o se queda dando vueltas para siempre. El teleport no es un parámetro: es decidir a dónde vuelve toda masa que se extravía.»*

## 24.14 Si solo lees 30 segundos

La centralidad pregunta «¿quién es importante?». El grado cuenta vecinos; el closeness mide distancias; el betweenness mide atascos (Brandes: V BFS en vez de todos los pares); el eigenvector propaga importancia… y se rompe en grafos dirigidos: la masa fuga por los colgantes y oscila en los ciclos. PageRank lo repara con dos arreglos — teleport con probabilidad 1−d (la matriz se vuelve primitiva: converge siempre, a razón ≈ d·λ₂) y redistribución uniforme de la masa colgante (masa total 1 en cada iteración). El teleport personalizado cambia el centro del mundo: semillas = consulta, ranking = recuperación. Esa es la pieza que GraphRAG heredará.

## 24.15 Una historia pequeña

La primera vez que corrimos PageRank sobre el grafo demo, Dani ganó con 0.386. Presentamos el resultado como «seguramente un bug»: nadie enlaza a Dani salvo él mismo. Pasamos la tarde buscando el error — reparticiones mal sumadas, teleport mal normalizado, el self-loop contado dos veces — hasta que hicimos lo que había que hacer desde el principio: resolver el sistema a mano. El número era 0.386 exacto: el self-loop devuelve cada voto y las ciudades le pagan la cuota colgante. El algoritmo estaba bien; MALA era nuestra intuición sobre «quién es importante». Esa tarde aprendimos la regla que atraviesa el capítulo: cuando un ranking sorprende, primero se hace la aritmética — el surfer nunca se equivoca sobre dónde pasa su tiempo; nosotros sí sobre dónde debería.

## Ejercicios resueltos

**1. En la cadena 0→1→2 (2 es colgante), calcula a mano la PRIMERA iteración desde el arranque uniforme y comprueba la masa.**

Arranque: x = [1/3, 1/3, 1/3]. Teleport uniforme t = 1/3, d = 0.85. Masa colgante D = x[2] = 1/3. Cuota colgante: d·D/n = 0.85·(1/3)/3 ≈ 0.0944 por nodo. Base por nodo: (1−d)·t + cuota = 0.05 + 0.0944 = 0.1444. Votos: 0→1 aporta 0.85·(1/3) ≈ 0.2833 a 1; 1→2 aporta 0.2833 a 2. Resultado: y = [0.1444, 0.4278, 0.4278], suma = 1.0000 exacto. La masa se conserva DESDE la primera iteración — el invariante no espera al límite. (El límite, para curiosos: 0.184, 0.341, 0.474 — el colgante acumula, pero la redistribución le impide absorberlo todo.)

**2. ¿Por qué la corrección de Wasserman-Faust da 1/3 a TODOS los nodos de dos 2-ciclos desconectados, y Freeman puro daría 1?**

Cada nodo alcanza a 1 de los 3 posibles, con Σd = 1. Freeman puro aplicado a lo alcanzable daría (r−1)/Σd = 1/1 = 1: la forma ingenua compara «mi mundo alcanzado» consigo mismo y premia la burbuja. Wasserman-Faust multiplica por la fracción alcanzada: C = ((r−1)/(n−1))·((r−1)/Σd) = (1/3)·(1/1) = 1/3: penalizado por los 2 nodos que NO alcanza. Verificación: `closeness_componentes_desconectadas_wasserman_faust`.

## Ejercicios propuestos

**Esencial (recordar — retrieval puro).** Con el libro CERRADO: (a) los DOS fallos del eigenvector crudo, con el grafo mínimo de cada uno; (b) los DOS arreglos de PageRank y qué garantiza cada uno; (c) qué mide el delta L1 y por qué la razón entre deltas consecutivos tiende a d·λ₂. Después ejecuta `cargo test -p vol2-liradb tests_centralidad` y localiza cada afirmación en un test POR NOMBRE. *Pistas*: (1) ¿qué le pasa al surfer en una página sin enlaces? (2) ¿puede una masa rotar en un ciclo para siempre? (3) ¿qué matriz converge siempre: la positiva o la cruda? *Criterio*: cuatro tests citados correctamente sin mirar.

**Intermedio (analizar — spacing con el cap. 14).** Explica en dos frases por qué la `Proyeccion` es «el CSR del capítulo 14 en memoria» y qué le falta para SERLO (offsets+targets compactos vs `Vec<Vec<usize>>`, pesos, ids de arista). Luego PREDICE, antes de ejecutar, `bfs_runs` y `edges_scanned` de `betweenness_centrality` sobre el camino simetrizado de 6 nodos (fórmula V·E menos la deduplicación de Both) y verifícalo con un miniprograma que lea `CentralidadStats`. *Pistas*: (1) ¿cuántos BFS corre Brandes? (2) ¿cuántas entradas de vecindario pisa cada BFS en un camino? (3) ¿qué cuenta `edges_scanned` de la proyección además del BFS? *Criterio*: predicción exacta y el paralelo CSR bien dicho.

**Experto (crear — interleaving caps. 7-8-22-26 + costura Vol. III).** Construye el mini-operador de recuperación que el capítulo 51 necesitará: `ppr_por_etiqueta(store, etiqueta, damping) -> Result<PageRankResult>` que siembre `personalized_page_rank` con TODOS los nodos de esa etiqueta a peso uniforme (recorriendo `iter_nodes` del trait, cap. 8). Verifica: masa = 1; los nodos de otra etiqueta SIN camino desde la semilla quedan a ~0 (¿por qué en el demo las ciudades NO quedan a 0 exacto?); el ranking difiere del global. Extensión conceptual sin código: ¿qué habría que cambiar para que los enlaces PESen por la propiedad `since`? (Respuesta esperada: nada aquí — es la `ProyeccionPonderada` del cap. 26 leyendo pesos con la semántica estricta `edge_weight` del cap. 22; el núcleo de potencia no se toca.) *Pistas*: (1) ¿quién valida las semillas: tu función o `densificar`? (2) ¿por dónde entra el peso en `iteracion_de_potencia`? (3) ¿por qué tu función no debe duplicar el bucle? *Criterio*: test propio verde, núcleo intacto.

## Para profundizar

- **L. Page, S. Brin, R. Motwani y T. Winograd, «The PageRank Citation Ranking: Bringing Order to the Web» (Stanford Digital Library, 1998/1999; presentado en WWW7, Brisbane, 1998)** — el paper del capítulo: el surfer aleatorio, el damping 0.85, la web como grafo de citas.
- **S. Brin y L. Page, «The Anatomy of a Large-Scale Hypertextual Web Search Engine» (WWW7, 1998)** — el paper de Google como sistema, con PageRank como pieza.
- **U. Brandes, «A Faster Algorithm for Betweenness Centrality» (Journal of Mathematical Sociology 25(2), 2001)** — la reducción de todos-los-pares a V BFS con dependencias hacia atrás.
- **S. Wasserman y K. Faust, «Social Network Analysis: Methods and Applications» (Cambridge Univ. Press, 1994)** — la corrección de cercanía en componentes desconectadas.
- **A. Langville y C. Meyer, «Google's PageRank and Beyond» (Princeton Univ. Press, 2006)** y **T. Haveliwala y S. Kamvar, «The Second Eigenvalue of the Google Matrix» (Stanford, 2003)** — por qué converge y a qué velocidad: λ₂ = d.
- **Neo4j Graph Data Science — `gds.pageRank` (con `sourceNodes`) y `gds.betweenness`** — las mismas decisiones de diseño, en producción.
- **Patente US 6,285,999** («Method for node ranking in a linked database», 1998-2018) e **IEEE Milestone «PageRank and the Birth of Google, 1996–1998»** (ethw.org) — la historia, documentada.

## Mini-diálogo: en la cima del ranking

> — Entonces el damping es… ¿el parámetro que hace que el bucle acabe?
>
> — Es el parámetro que hace que el bucle PUEDA acabar: contrae todos los autovalores salvo el 1. Sin él, un miserable ciclo de tres nodos te deja oscilando para siempre — lo hemos ejecutado, no es retórica.
>
> — ¿Y el 0.85 es sagrado?
>
> — Es el del paper y el de Neo4j. Bájalo y rankeas el teleport; súbelo y tardas más en asentar. El número importa menos que el intervalo: (0,1) ABIERTO, porque los bordes son otros dos algoritmos degenerados.
>
> — Y lo personalizado… ¿eso no es hacer trampa con el resultado?
>
> — Es cambiar la pregunta. El global pregunta qué importa en general; el personalizado pregunta qué importa DESDE ti. La web entera o tu órbita: mismo surfer, distinto mapa. Recuérdalo — en el Vol. III, esa diferencia será un buscador.

---

*(Próximo capítulo: 25 — Comunidades y agrupaciones. PageRank dice quién es importante; aún no sabe decir en qué GRUPOS se organiza el grafo. Llega Louvain — y con él una sorpresa: no podrá reutilizar la proyección de este capítulo tal cual. Te avisamos por algo.)
# Capítulo 25 — Comunidades y agrupaciones (Louvain simplificado)

> *«El ojo ve doce tribus. La modularidad, con el grafo entero en la cabeza, prefiere seis. Ninguna de las dos miente: preguntan cosas distintas.»*

## 25.0 La anécdota de la esquina

En 2008, cuatro físicos e informáticos belgas — Vincent D. Blondel, Jean-Loup Guillaume, Renaud Lambiotte y Etienne Lefebvre — publicaron en una revista de física estadística (*Journal of Statistical Mechanics: Theory and Experiment*, artículo P10008) un método para «desplegar» la estructura de comunidades de redes enormes. El título del paper, «Fast unfolding of communities in large networks», no menciona ninguna universidad. Pero el método nació en la Université catholique de Louvain (Louvain-la-Neuve, Bélgica), y la comunidad científica, que necesita nombres cortos, lo bautizó con el de la institución: **el método de Louvain**. Hoy es uno de los papers más citados de la ciencia de redes — decenas de miles de citas — y hasta la página del propio Blondel en UCLouvain lo presenta ya como «the Louvain method».

La velocidad era la afirmación audaz del paper: «supera a todos los métodos conocidos de detección de comunidades en tiempo de cómputo» (es la claim literal del abstract), y lo demostraron con la red de llamadas de una operadora belga — 2,6 millones de clientes — y con un grafo web de 118 millones de nodos y más de mil millones de enlaces. Para dimensionar lo que eso significaba: el método clásico que lo precedió, el **divisivo de Girvan-Newman**, detectaba comunidades cortando repetidamente la arista de mayor *betweenness* — la que calculaste con tanto esfuerzo en el capítulo 24 — y tenía que recalcularla entera después de cada corte: los propios autores (Newman-Girvan 2004) reportan un coste de O(m²·n), es decir O(n³) en grafos dispersos. Con 2,6 millones de nodos, ni hablemos. Curiosamente, la **modularidad** — la métrica que guía todo este capítulo — nació precisamente en esa tradición: Newman y Girvan la inventaron en 2004 como criterio para decidir *cuándo parar de cortar aristas*. Louvain invirtió el papel de las dos piezas: en vez de cortar aristas y usar Q para parar, **optimiza Q directamente** con movimientos locales baratos y una jerarquía que se contrae sola. Ese giro — y no una fórmula nueva — es lo que lo hizo rápido.

## 25.1 Objetivo

Al terminar este capítulo sabrás **particionar un grafo en comunidades de forma verificable**: no «grupos que se ven bonitos», sino grupos con un número — la modularidad Q — que cualquiera puede recalcular sobre la partición y comparar con alternativas. Construirás las cuatro piezas del módulo `cap25_comunidades.rs`:

1. `componentes_conexas` — el suelo del concepto: alcanzabilidad pura.
2. `label_propagation` — la primera heurística densa, y sus límites documentados.
3. `modularidad` — la métrica Q de Newman-Girvan, con resolución γ, verificable sobre cualquier partición dada.
4. `louvain` — el greedy jerárquico: fase local con ΔQ exacto + agregación en supernodos, nivel a nivel, dejando un dendrograma.

El hito: detectar los dos grupos de una red social pequeña (dos K4 unidos por un puente) y *demostrar con números* que es la mejor partición.

## 25.2 Problema

El capítulo 24 te dio el mapa del «quién es importante». Esta es la pregunta complementaria: **¿quiénes forman grupo?** Piensa en la red social del `demo_graph`: Ana, Bea y Carlos se conocen entre sí; cada uno vive en una ciudad; Dani se conoce a sí mismo (ese `KNOWS` de Dani hacia sí mismo que ya te encontraste en el capítulo 24, dándole cuota de PageRank). ¿Cuántas comunidades hay? ¿Quién está con quién?

La pregunta es resbaladiza por dos motivos:

- **«Comunidad» no puede ser «grupo denso» a secas.** Un triángulo dentro de un grafo completísimo no es comunidad: ahí TODO es denso. Lo que hace comunidad es ser denso **respecto a lo que cabría esperar al azar**. Necesitas un modelo de azar de referencia — un *modelo nulo* — o el concepto no significa nada.
- **Cualquier partición es una respuesta.** «Todos en una» es una partición. «Cada uno solo» es otra. Sin una función que puntúe particiones, no puedes ni compararlas ni decir que tu algoritmo hizo un buen trabajo. «Encontré 7 comunidades» no es un resultado: 7 salió de ti, no del grafo.

Y hay una restricción de negocio, heredada del capítulo 24: esto corre **dentro de una base de datos**. Así que los análisis tienen que ser **reproducibles** (dos ejecuciones, mismo resultado: nada de aleatoriedad) y **ruidosos al fallar** (un peso negativo no se «tolera»: se señala con la arista culpable, igual que hacía Dijkstra en el capítulo 22).

## 25.3 Modelo mental

Piensa en el grafo como un **mapa de tribus**. Una tribu es una región del mapa donde la gente se relaciona sobre todo entre sí, y poco con los de fuera. La pregunta que mide la calidad del mapa:

> **De todo el peso de las aristas del grafo, ¿qué fracción cae dentro de las tribus… y cuánto MÁS de lo que esperaría el azar si repartiáramos esas mismas aristas al azar (conservando cuántas relaciones tiene cada nodo)?**

Ese «azar que conserva los grados» es el **modelo nulo de configuración**: la probabilidad de que el azar ponga una arista entre i y j es proporcional a k_i·k_j. La **modularidad** (Newman-Girvan 2004) es exactamente ese exceso sobre el azar, sumado por comunidad:

```text
Q_γ = Σ_c [ Σ_in(c)/2m − γ·(Σ_tot(c)/2m)² ]
        \_______/   \____________________/
        fracción de   lo que el azar
        peso INTERNO  esperaría para c
```

- `Σ_in(c)`: peso interno de la comunidad c, contando cada arista por sus dos extremos.
- `Σ_tot(c)`: suma de los grados (ponderados) de los miembros de c — incluye las aristas que salen fuera.
- `2m`: el peso total del grafo contando cada arista en ambas direcciones (la convención de siempre: Σ_tot de todos = 2m).
- `γ` (gamma): la **resolución** de Reichardt-Bornholdt 2006 — cuánto le exigimos al azar. γ=1 es el clásico.

Lecturas inmediatas: la partición trivial (todo junto) da Q = 1 − γ = 0 exacto; Q > 0 es «mejor que el azar»; Q < 0, peor. Y como es una fracción de 2m, **escalar todos los pesos por una constante no cambia Q** — lo testea el módulo.

El diagrama que ordena el capítulo es el anillo de tríos — el grafo canónico del límite de resolución de Fortunato-Barthélemy (2007), que veremos en 25.6:

```text
      trío 0        trío 1        trío 2         ...      trío 11
    0 ─── 1  ~~  3 ─── 4  ~~   6 ─── 7   ...         ~~ 33 ─── 34
     \  /          \  /          \  /                      \  /
      2 ────────────5 ────────────8 ───── ... ───────────── 35
      (K3)          (K3)          (K3)                     (K3)
        eslabón del anillo: cada trío cuelga del siguiente por UNA arista
```

Doce tríos idénticos, un eslabón cada uno. Tu ojo ve doce tribus. Veremos que Q, con γ=1, prefiere seis pares — y por qué eso no es un bug sino una lección profunda sobre lo que significa «comunidad global».

## 25.4 Primera solución

La solución más ingenua que funciona un poco: **una comunidad es una componente conexa**. El módulo la implementa con el BFS de toda la vida sobre la vista simétrica del store (pesos constantes 1, porque la alcanzabilidad no pesa):

```rust
pub fn componentes_conexas(store: &dyn GraphStore) -> Result<ComponentesResult, ComunidadesError>
```

O(V+E), números por menor miembro (el barrido ascendente numera cada componente con el nodo más pequeño que contiene — la renumeración canónica sale gratis). Es el **suelo del concepto**: toda comunidad vive dentro de una componente (nadie se agrupa con quien no alcanza), pero ninguna noción de densidad decide todavía.

Segundo paso, ya con ambición: **label propagation** (LPA, Raghavan-Albert-Kumara 2007). Cada nodo empieza con su etiqueta; en cada pasada (nodos por id ascendente, actualización asíncrona) adopta la etiqueta más votada entre sus vecinos, con votos ponderados por el peso de la arista. Casi lineal, sin función objetivo. El módulo lo hace determinista: empates → se conserva la propia etiqueta si empata con la máxima, y si no gana la menor.

## 25.5 Sus límites

**Las componentes mueren con un solo puente.** Dos K3 unidos por una arista: una componente. La red social real suele ser UNA componente gigante — la pregunta interesante (tribus dentro de la isla) queda intacta.

**LPA no optimiza nada verificable — y encima gotea.** No hay Q que comprobar. Peor: con pesos uniformes, los empates de la primera pasada pueden GOTEAR por los puentes. El test `lpa_separa_dos_trios_y_empates_deterministas` lo documenta con un experimento espejo que vale la pena despacio:

```text
dos K3 con puente 2-3:  LPA → 1 comunidad   Louvain → 2 comunidades
dos K3 con puente 1-4:  LPA → 2 comunidades  Louvain → 2 comunidades
```

El MISMO grafo espejado (solo cambia qué nodos toca el puente), el MISMO algoritmo… y resultados opuestos. Mecánica del goteo en el primer caso: al barrer por id ascendente, el trío izquierdo (nodos 0-2) se forma primero; cuando le llega el turno al nodo 3 (el del puente derecho), sus votos son {etiqueta 1: 1, etiqueta 4: 1, etiqueta 5: 1} — tres etiquetas empatadas, y la de él propio (3) tiene cero votos de vecinos. Adopta la 1. Arrastró la etiqueta izquierda a través del puente antes de que su propio trío tuviera tiempo de formarse. En el caso espejo (puente 1-4), cuando el barrido llega al nodo 4, el nodo 3 ya barrió y le dio un voto a la etiqueta 4 — la política de «conservar la propia si empata con la máxima» lo salva. El resultado de LPA depende del orden y de la numeración; es determinista aquí (dos ejecuciones, idéntico), pero frágil. La receta práctica — test incluido — es romper los empates con pesos (puente de peso 0.5 → LPA ya separa).

Ésta es exactamente la motivación de Louvain: una heurística sin métrica no se puede ni verificar ni defender. Hace falta la métrica primero, el algoritmo después.

## 25.6 Solución evolucionada

### Paso 1: Q como juez — `modularidad(partición dada)`

La decisión de diseño más importante del capítulo: la modularidad es una **función verificable sobre CUALQUIER partición dada**, no un subproducto interno del algoritmo. `modularidad(store, particion, weight, gamma)` la calcula sobre lo que le pases (nodos ausentes → singletons; ids de grupo u64 arbitrarios; γ validado: 0, negativo, NaN e ∞ rechazados). Con eso, Q es a la vez la métrica guía de Louvain y el oráculo de sus tests.

Cuenta conmigo los dos tríos con puente (pesos 1; 7 aristas → 2m = 14; grados [2,3,2,2,3,2] — el puente sube a 3 los nodos 1 y 4):

```text
partición perfecta {0,1,2} {3,4,5}:  cada trío In=6 (3 aristas × 2 extremos), K=7
    Q = 2·[6/14 − (7/14)²] = 12/14 − 1/2 = 5/14 ≈ 0.357
partición trivial (todo junto):      In=2m → Q = 1 − 1 = 0 exacto
singletons:  Q = −Σ(k_i/2m)² = −(4+9+4+4+9+4)/196 = −17/98
```

Los tres números están testeados contra la aritmética exacta (`EPS = 1e-12`). Fíjate en el porqué de usar Q y no «número de comunidades»: 2 comunidades (Q=5/14), 6 (Q=−17/98) y 1 (Q=0) son particiones de distinto tamaño — el conteo no las ordena; Q sí. Y para demostrar que Q está bien calculada, un ejercicio de honestidad numérica: en el MISMO grafo, ¿conviene fundir los dos tríos? ΔQ = 2/14 − 2·(7/14)² = −5/14 < 0: no. Ojo a esa fórmula, porque reaparece.

### Paso 2: el greedy local con ΔQ EXACTO — y la agregación

Louvain alterna dos fases por nivel:

1. **Fase local**: desde singletons, cada nodo (por id ascendente — el Louvain «de literatura» baraja; una BD no puede) evalúa moverse a cada comunidad *vecina* con el **ΔQ exacto** — solo cambian los términos de las DOS comunidades implicadas:

```text
ΔQ = q(In_c + 2·k_{i,c} + 2·s_i, K_c + k_i)      ← destino ganado
   + q(In_d − 2·k_{i,d} − 2·s_i, K_d − k_i)      ← origen perdido
   − q(In_c, K_c) − q(In_d, K_d)                 ← como estaban
donde q(in, k) = in/2m − γ·(k/2m)²,  k_{i,c} = peso de i hacia c,  s_i = self-loop de i
```

Se mueve solo si ΔQ > 0 estricto; empates → comunidad de menor id (`total_cmp`, la misma disciplina anti-f64 de los caps. 22 y 24). Pasada tras pasada hasta que nadie se mueva. ¿Por qué ΔQ exacto y no «recalcular Q entera»? **Trazabilidad aritmética**: cada movimiento se puede re-verificar a mano con cuatro términos, la complejidad por evaluación es O(grado) en vez de O(V+E), y la monotonía de Q queda garantizada por construcción (test: Q nunca baja entre niveles).

2. **Agregación**: cada comunidad se contrae en un **supernodo**; las aristas internas se vuelven self-loops del supernodo, las externas suman pesos entre supernodos. La fase (1) se repite sobre el grafo contraído. La gracia: movimientos que a escala de nodo estaban bloqueados (dos tríos que individualmente no ganan nada moviéndose) se vuelven visibles cuando cada trío YA es un supernodo. En el anillo de 12 tríos, el nivel 0 encuentra los 12 tríos (Q = 2/3) y el nivel 1 los funde en 6 pares (Q = 17/24): dos niveles, dendrograma real.

### Paso 3: el TEST ESTRELLA — el límite de resolución

¿Por qué el nivel 1 del anillo FUNDE tríos si tu ojo ve doce tribus? La cuenta, con la fórmula del paso 1 (cada trío: In=6, K=8; 2m=96):

```text
fundir dos tríos adyacentes (el eslabón, peso 1, se vuelve interno):
  ΔQ = 2/96 − γ·2·(8/96)²  =  1/48 − γ/72
γ=1:  ΔQ = +1/144  → FUNDE (Q pares 17/24  >  Q tríos 2/3)
γ=2:  ΔQ = −1/144  → NO funde (Q tríos 7/12  >  Q pares 13/24)
```

Ahí está, en dos fracciones, el **límite de resolución de Fortunato-Barthélemy (2007)**: la modularidad global pregunta al azar de TODO el grafo, y en un grafo grande el azar espera tan poquito cruce entre dos tríos que fusionarlos «ahorra» penalización — aunque el eslabón sea una única arista floja. La MISMA estructura local que en el grafo de 2m=14 NO se funde (ΔQ = −5/14), en el anillo de 2m=96 SÍ (ΔQ = +1/144): el tamaño del mundo cambia la respuesta. La escala crítica de Fortunato-Barthélemy: comunidades más chicas que ~√(2m) aristas se vuelven invisibles para γ=1. El remedio es γ: γ>1 exige comunidades más densas y chicas (el umbral de este anillo está en γ* = 3/2, donde ΔQ = 0). El panorama completo, todo él verificable con `modularidad()`:

| Partición del anillo | Q con γ=1 | Q con γ=2 | Ganadora |
|---|---|---|---|
| 12 tríos (la del ojo) | 2/3 ≈ 0.667 | **7/12 ≈ 0.583** | γ=2 |
| 6 pares fundidos | **17/24 ≈ 0.708** | 13/24 ≈ 0.542 | γ=1 |

El test `louvain_limite_de_resolucion_gamma` demuestra ambas columnas con particiones ground-truth y Q analíticos — y por eso es el test estrella: no valida el código, valida la MÉTRICA, enseñándote cuándo tu herramienta va a mentirte.

## 25.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap25_comunidades.rs`. Las piezas con su porqué.

### `GrafoPonderado`: la proyección hermana del cap 24 — pero que SUMA

```rust
struct GrafoPonderado {
    nodes: Vec<NodeId>,            // orden ascendente → determinismo
    self_loop: Vec<f64>,           // s_i SIN el ×2 (la convención se materializa en k)
    vecinos: Vec<Vec<(usize, f64)>>, // vecinos distintos, pesos ACUMULADOS, ordenados
    k: Vec<f64>,                   // k_i = 2·s_i + Σ_j w_ij  (self-loop contado doble)
    dos_m: f64,                    // 2m = Σ_i k_i
    edges_scanned: u64,
}
```

¿Por qué un grafo propio y no reusar la `Proyeccion` del cap 24? Porque son **familias distintas con convenciones distintas**. El cap 24 contaba *vecinos distintos* — su `GraphDirection::Both` hace unión como CONJUNTO (deduplica aristas paralelas), que es correcto para grado y centralidades. La modularidad es de **multigrafo**: tres mensajes entre Ana y Bea son un lazo triple, y deben SUMAR (el término 2·k_{i,c} del ΔQ lo exige). El test `louvain_multigrafo_paralelas_equivalen_a_peso_sumado` lo clava: 3 aristas paralelas de peso 1 ≡ una de peso 3, mismo resultado exacto. Además Louvain **reconstruye** el grafo en cada agregación: `contraer()` devuelve otro `GrafoPonderado`. Lo heredado del cap 24 es el patrón (ids ordenados, índice denso que compacta los huecos de `delete_node`, materializar una vez); lo heredado del cap 22 es la semántica estricta de pesos (`WeightSource` + `edge_weight`, con `From<PathError>` para envolver `MissingWeight`/`InvalidWeight`/`NonFiniteWeight`).

### El self-loop ×2 — la convención que sostiene la jerarquía

Un self-loop de peso s entra como A_ii = 2s: **cuenta doble** en k_i y en 2m. No es capricho: es la convención estándar de la modularidad (así k_i = Σ_j A_ij lo cuenta «una vez por dirección»), y sobre todo es lo que hace que la **contracción conserve Q**: una arista interna de peso w aportaba 2w a Σ_in (sus dos extremos) y w+w a Σ_tot; contraída, es un self-loop s=w que aporta A_cc = 2w a ambas cosas — idéntico. Test de invariante nivel a nivel: la Q de cada nivel, calculada en el grafo contraído, coincide con `modularidad()` sobre el original. En una red social, un self-loop es una relación consigo mismo (el `KNOWS` de Dani): refuerza su comunidad propia sin unirlo a nadie — en `demo_graph`, Dani acaba solo, con Q = 5/18 empatada entre DOS óptimos distintos (el test documenta el empate en vez de fingir unicidad).

### `louvain`: el bucle de niveles y su cota de terminación

```rust
let tope_niveles = n + 1;   // cota DEMOSTRABLE, no esperanza
loop {
    let (com, movimientos, pasadas) = g.fase_local(gamma, max_pasadas, &mut stats);
    if movimientos == 0 { break; }             // nada que agregar
    // ... grabar NivelLouvain (asignación de nodos ORIGINALES, Q, stats)
    mapeo = mapeo.iter().map(|&t| densa[t]).collect(); // composición
    g = g.contraer(&densa);                     // 2m se conserva
    if g.len() < 2 || niveles.len() >= tope_niveles { break; }
}
```

La cota: cada nivel arranca de singletons, así que su PRIMER movimiento vacía una comunidad ⇒ el nivel siguiente tiene estrictamente menos nodos ⇒ niveles con movimientos ≤ V (en la práctica O(log V): el grafo se contrae geométricamente). `max_pasadas` limita cada fase local — un seguro contra el ruido de f64 en ΔQ ≈ 0 (un ΔQ «positivo» de 1e-18 movería nodos para siempre). Y ojo al hallazgo testeado: `max_pasadas=1` NO rebaja la calidad final — lo que una pasada deja a medias en el nivel 0, la agregación lo repara en el nivel 1.

### La jerarquía lleva nodos ORIGINALES

`NivelLouvain` guarda, por nivel, la asignación de los nodos **originales** (la composición de particiones), su Q y su número de comunidades; `particion_en(nivel)` construye el dendrograma a demanda. ¿Por qué original y no supernodos? Porque esto es un producto: el cap 51 (Vol. III, GraphRAG) consumirá los niveles para generar resúmenes del grafo a varias granularidades, y necesita responder «¿quién está con quién?» en el vocabulario del usuario. El anidamiento queda garantizado por construcción (cada comunidad del nivel ℓ+1 es unión exacta de comunidades del ℓ — testeado en la dirección correcta: fina ⇒ gruesa).

## 25.8 Prueba de fuego

El hito del capítulo: **dos K4 unidos por un puente** (13 aristas, 2m = 26). Louvain DEBE separarlos, y la Q debe cuadrar a mano: cada K4 tiene In = 12 (6 aristas × 2) y K = 13 (los nodos del puente tienen grado 4):

```rust
let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
assert_eq!(r.num_comunidades(), 2);
assert_ne!(r.comunidad(0), r.comunidad(5));
assert!((r.modularidad - 11.0 / 26.0).abs() < 1e-12);   // 2·[12/26 − (13/26)²] = 11/26
```

Los tests que cierran el círculo (todos ejecutables con `cargo test -p vol2-liradb cap25`):

- **Oráculo**: Q del resultado == `modularidad()` de la misma partición, en CADA caso.
- **Determinismo**: dos ejecuciones idénticas, jerarquía incluida, incluso con el orden de inserción de las aristas INVERTIDO (`louvain_determinismo_y_orden_de_insercion`).
- **Ground truth sintético**: 3 anillos con cuerdas y 2 eslabones (30 nodos): recupera los 3 grupos EXACTO y con Q ≥ Q_truth — mientras `componentes_conexas` dice que todo es UNO (alcanabilidad ≠ densidad, dos nociones de grupo).
- **Los pesos reestructuran**: dos tríos con puente de peso 100 → NO se funde todo (Q es invariante a escala): el puente deja de ser explicable por el azar y la mejor partición ROMPE los tríos alrededor de él — {0,2},{1,4},{3,5} con Q = 100/2809, mejor que la trivial (0). «Más peso» no es «más fusión».
- **Fallos ruidosos**: peso negativo → `NegativeWeight { edge, weight }` con la arista señalada; prop ausente → el `MissingWeight` del cap 22 envuelto.
- **Coste medible, no declamado**: `ComunidadesStats` cuenta `edges_scanned` (96 en el anillo de 12 tríos: 48 pares × 2 aristas dirigidas), `pasadas`, `movimientos` y `niveles` — la sección «coste computacional» del guion, verificada por `louvain_stats_coherentes`.

Si te saltaras este capítulo, el síntoma te delataría: agruparías por componentes («¡una comunidad gigante!») o confiarías en LPA sin pesos, y cuando alguien pregunte «¿por qué esas comunidades y no otras?» no tendrías número ni criterio — y el cap 51 se quedaría sin dendrograma.

## 25.9 Qué hemos sacrificado

1. **El barajado del Louvain original**: la aleatoriedad explora mejor (escapa de algunos óptimos locales); la pagamos con la reproducibilidad, que en una BD no se negocia. Queda documentado el precio: varios óptimos con Q igual se resuelven por orden (`demo_graph`: dos óptimos con Q = 5/18).
2. **La re-localización de Leiden**: Louvain clásico puede dejar comunidades MAL CONECTADAS internamente (dos piezas unidas «por detrás» del supernodo). Leiden (Traag-Waltman-van Eck 2019) lo repara con una fase de refinamiento; aquí lo declaramos limitación (sección 25.10).
3. **La proyección con pesos compartida**: la Parte V merece una proyección ponderada única sobre el CSR del cap 14 — es exactamente el capítulo 26. Aquí el `GrafoPonderado` la materializa en memoria y la deuda queda declarada.
4. **Exactitud global**: greedy local = óptimo local garantizado-coherente, no óptimo global. El contrato del resultado es «una partición coherente con su Q», no «la mejor partición posible».

## 25.10 Cómo lo hace una BBDD real

**Neo4j Graph Data Science** expone exactamente estas piezas: `gds.louvain` (Tier 1) y `gds.leiden` (Tier 2), con `relationshipWeightProperty` (nuestro `WeightSource::Property`), `tolerance` (nuestro corte de ΔQ), `maxLevels` (nuestro tope de niveles) y — dato que valida el capítulo — `includeSelfLoops`, un flag que decide si los self-loops cuentan en el cálculo: la convención A_ii = 2s es una decisión de semántica real, la que discutimos en 25.7. Los modos `stats/mutate/stream/write` escriben el `communityId` como propiedad de nodo — la partición como dato consultable, igual que nuestra `Particion::grupo(id)`.

¿Por qué existe Leiden si Louvain es tan bueno? Por la crítica demoledora de Traag, Waltman y van Eck («From Louvain to Leiden: guaranteeing well-connected communities», *Scientific Reports* 9:5233, 2019): el greedy de Louvain evalúa si cada NODO está bien conectado a su comunidad, nunca si la COMUNIDAD está bien conectada consigo misma — y pueden salir (ellos lo fotografían) comunidades internamente desconectadas: dos trozos pegados solo a través de otras comunidades, invisibles tras la agregación. Leiden añade una fase de refinamiento que parte comunidades mal conectadas y garantiza (con su procedimiento de agregación) conectividad interna. La lección de ingeniería: un algoritmo que optimiza una métrica no garantiza propiedades que la métrica no mide.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: calcula a mano la Q de la partición {0,1,2},{3,4,5},{4}… no: de {0,1,2,4},{3},{5} sobre los dos tríos con puente. ¿Supera al −17/98 de los singletons? ¿Y al 0 de la trivial?
- *Intermedio*: ¿por qué el umbral γ* del anillo de 12 tríos es exactamente 3/2? Derívalo de ΔQ = 1/48 − γ/72 y comprueba que γ* no depende de k (número de tríos). ¿Qué SÍ depende de k?
- *Experto*: construye el anillo de k tríos con k parámetro y encuentra empíricamente el k donde γ=1 empieza a fundir; explica por qué el k crítico crece con… ¿o no crece? (Fortunato-Barthélemy: la escala invisible es ~√(2m).)

## 25.11 Lo que te llevas

- **Comunidad = densidad contra el azar** (modelo nulo de configuración), no componente (alcanabilidad) ni «grupo denso» a secas.
- **Q_γ es la métrica guía Y el oráculo**: verificable sobre cualquier partición dada; compara particiones de distinto tamaño; un «número de comunidades» no compara nada.
- **Greedy local con ΔQ exacto + agregación**: O(grado) por evaluación, trazable a mano, monótono en Q; el corte de aristas de Girvan-Newman (O(m²·n)) es lo que Louvain reemplazó.
- **Convenciones que sostienen todo**: self-loop A_ii = 2s (hace que la contracción conserve Q), simetrización SUMANDO (multigrafo), pesos estrictos del cap 22 y negativos rechazados eager.
- **Determinismo total**: orden por id, empates por `total_cmp` → menor, renumeración por menor miembro; cota ≤ V niveles + `max_pasadas` anti-ruido-f64.
- **El límite de resolución**: Q global no ve comunidades más chicas que ~√(2m); γ lo arregla (anillo de tríos: γ=1 → 6 pares con Q=17/24; γ=2 → 12 tríos con Q=7/12).
- **La jerarquía es el producto**: nodos ORIGINALES por nivel, anidamiento garantizado — el dendrograma que el cap 51 consumirá.

## 25.12 Ojo, cuidado con…

- **Comparar Q entre grafos o entre γ distintos**: Q es «exceso sobre el azar» DENTRO de un grafo y una γ. 0.42 aquí no es «mejor» que 0.36 allá.
- **Contar el self-loop simple**: rompe k_i, 2m y la conservación de Q al agregar. Se detecta porque la Q de un nivel deja de cuadrar con `modularidad()` sobre el original.
- **Deduplicar aristas paralelas** (la convención del cap 24): aquí el multigrafo ACUMULA; deduplicar mide otro grafo.
- **Confundir «partición coherente con su Q» con «partición óptima»**: el greedy para cuando nadie mejora; el mundo puede tener dos óptimos empatados (`demo_graph`, Q = 5/18 ×2) o uno mejor que el greedy nunca vio.

## 25.13 Pin de batalla

> *«Si tu detector de comunidades no puede decirte QUÉ está optimizando con un número que puedas recalcular, no es un detector: es una opinión compilada.»*

## 25.14 Si solo lees 30 segundos

Una comunidad es una región del grafo más densa de lo que el azar esperaría. La **modularidad Q_γ** pone número a una partición completa — «fracción de peso interno menos la esperada por el azar» — y por eso guía Y juzga. **Louvain** la optimiza en dos fases alternadas: mover nodos al vecino con mayor ΔQ exacto (greedy local, determinista), y contraer comunidades en supernodos (agregación) nivel a nivel, conservando Q. Sus límites declarados: óptimos locales (greedy) y el **límite de resolución** (Q global funde comunidades chicas: el anillo de 12 tríos lo demuestra; γ lo corrige). La jerarquía resultante, con nodos originales, es el insumo del GraphRAG del cap 51.

## 25.15 Una historia pequeña

Cuando añadimos el límite de resolución al módulo, el primer impulso fue «arreglarlo»: si Q prefiere seis pares donde el ojo ve doce tribus, que la libería corrija el resultado. Nos detuvimos a medio commit. El anillo no estaba roto — estaba ENSEÑANDO: la misma estructura local funde o no funde según el tamaño del grafo entero, porque el azar de referencia es global. Ocultar eso habría convertido el test estrella en una caja negra que «funciona». Lo dejamos, con γ al aire y los valores analíticos al lado: 17/24 contra 2/3 con γ=1, 7/12 contra 13/24 con γ=2, y el umbral en 3/2 derivable a mano. El día que un usuario pregunte «¿por qué mi detección une estos dos equipos que claramente son distintos?», la respuesta estará esperando en un test, no en un foro.

## Ejercicios resueltos

**1. ¿Por qué la partición trivial da exactamente 0 y no «casi 0»?**

Con todo en una comunidad: Σ_in = 2m (todas las aristas son internas) y Σ_tot = 2m. El término único es 2m/2m − γ·(2m/2m)² = 1 − γ. Con γ=1, exactamente 0 — sin redondeos, sin suerte: la estructura de la fórmula lo garantiza para CUALQUIER grafo. Es el test más barato y más duro de romper de la función `modularidad` (está en el doctest).

**2. En `demo_graph` (KNOWS: 0→1→2→0 y self-loop de Dani; LIVES_IN: 0→4, 1→5; k = [3,3,2,2,1,1], 2m = 12), ¿por qué Dani acaba solo SIEMPRE?**

El self-loop de Dani entra como A_33 = 2. Su única «relación» es consigo mismo: no tiene comunidad vecina a la que moverse (la fase local evalúa solo comunidades VECINAS). Queda de semilla singleton en cualquier nivel. Y las dos particiones restantes empatan EXACTO en Q = 5/18: {0,2,4},{1,5},{3} y {0,1,2,4,5},{3} — el ΔQ que las separa es 0, y el greedy determinista toma una. El test afianza lo que NO depende del camino (Dani solo, Q = 5/18), no la elección.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el capítulo: escribe Q_γ de memoria y calcula a mano la Q de la partición perfecta de los dos tríos con puente. Verifica contra el doctest de `modularidad` y el test `modularidad_particion_trivial_es_cero_y_perfecta_analitica`. Pistas: (1) ¿cuánto vale Σ_in si cada arista interna se cuenta por sus DOS extremos?; (2) ¿el puente entra en Σ_tot del trío?; (3) ¿por qué la trivial es 0 exacto?

**Intermedio (spacing con el cap 24).** Explica, nodo a nodo, la primera pasada de LPA sobre dos K3 con puente 2-3 (gotea a 1 comunidad) y sobre el espejo con puente 1-4 (separa en 2), y qué haría Louvain en ambos. Verifica con `lpa_separa_dos_trios_y_empates_deterministas`. Pistas: (1) ¿en qué orden se barren los nodos?; (2) ¿de dónde salen los votos de la etiqueta PROPIA de un nodo?; (3) ¿qué ΔQ tendría el movimiento que gotea?

**Experto (interleaving, gancho al cap 51).** Generaliza el anillo a k tríos con k parámetro: deriva el ΔQ de fundir dos tríos adyacentes en función de k y γ, predice para qué k γ=1 empieza a fundir, y verifícalo empíricamente. Luego, con γ=2 y `particion_en(0)`/`particion_en(1)` sobre el anillo de 12, lista las comunidades finas y gruesas — el dendrograma que el cap 51 resumirá. Pistas: (1) ¿qué términos de Q dependen de k?; (2) ¿2m crece con k… y el término de fusión también?; (3) ¿por qué el anidamiento solo se afirma en la dirección fina ⇒ gruesa?

## Para profundizar

- **Blondel, Guillaume, Lambiotte, Lefebvre, «Fast unfolding of communities in large networks», J. Stat. Mech. (2008) P10008** — el paper de Louvain; el abstract con la claim de velocidad y los 2,6 M de clientes está en [IOPscience](https://iopscience.iop.org/article/10.1088/1742-5468/2008/10/P10008) y [arXiv:0803.0476](https://arxiv.org/abs/0803.0476).
- **Fortunato, «Community Detection in Graphs», Physics Reports 486 (2010)** — el mapa completo: divisive, LPA, modularidad, límites.
- **Fortunato-Barthélemy, «Resolution limit in community detection», PNAS 104(1):36-41 (2007)** — [el paper del anillo de cliques](https://www.pnas.org/doi/10.1073/pnas.0605965104).
- **Traag, Waltman, van Eck, «From Louvain to Leiden», Sci. Rep. 9:5233 (2019)** — [la crítica de las comunidades mal conectadas y el algoritmo que las garantiza](https://www.nature.com/articles/s41598-019-41695-z).
- **Neo4j GDS: [Louvain](https://neo4j.com/docs/graph-data-science/current/algorithms/louvain/) y [Leiden](https://neo4j.com/docs/graph-data-science/current/algorithms/leiden/)** — `relationshipWeightProperty`, `includeSelfLoops`, `tolerance`, `maxLevels`: este capítulo en producción.

## Mini-diálogo: en guardia nocturna

> — O sea que Louvain es «mueve nodos si Q sube, contrae, repite». ¿Y por qué tanto bombo?
>
> — Porque cada pieza es verificable. El ΔQ es exacto y lo recalculas a mano; la Q de cada nivel coincide en el grafo contraído y en el original; dos ejecuciones dan lo mismo hasta con las aristas insertadas al revés. Puedes enseñarle el algoritmo a un auditor línea a línea.
>
> — Pero el anillo de tríos… tu métrica prefiere seis pares donde yo veo doce tribus.
>
> — Exacto: esa es la mejor parte. La herramienta no te falla en silencio — te dice CUÁNDO va a mentirte, con la aritmética a la vista. γ es el zoom, y el umbral 3/2 del anillo lo derivas tú, no lo decreta la librería.
>
> — ¿Y si necesito que las comunidades estén bien conectadas por dentro?
>
> — Entonces ya sabes qué paper leer, qué algoritmo pedirle a tu base de datos, y por qué existe. Eso es saber más que «usar Louvain».

---

*(Próximo capítulo: 26 — Ejecutar algoritmos sin agotar la memoria. Aquí `GrafoPonderado` materializó el grafo entero en RAM; veremos la proyección con pesos sobre el CSR del capítulo 14, el streaming y los frontiers — y saldremos de la deuda que este capítulo dejó declarada.)*
# Capítulo 26 — Ejecutar algoritmos sin agotar la memoria (proyección, streaming, frontiers)

> *«La memoria no se gestiona con swap. Se gestiona decidiendo qué existe y cuándo.»*

## 26.0 La anécdota de la esquina

Hacia 2008-2009, en Google se toparon con un muro que no era de algoritmos. El grafo de la Web, el de las redes sociales, los mapas de enlaces entre sitios: miles de millones de vértices que no cabían en la RAM de ninguna máquina. La herramienta generalista de la casa, MapReduce, encadenaba trabajos que re-leían el grafo ENTERO en cada iteración — y los algoritmos de grafos son, por naturaleza, iterativos. PageRank con 50 pasadas era re-leer el planeta 50 veces para que cada página mirase a sus vecinas.

La respuesta se presentó en SIGMOD 2010: Grzegorz Malewicz y seis coautores firmaron «Pregel: a System for Large-Scale Graph Processing» (pp. 135-146). La idea se resume en tres palabras que desde entonces son un lema: **think like a vertex** — piensa como un vértice. No escribas código que recorre el grafo: escribe el `compute()` de UN vértice, que en cada *superpaso* lee los mensajes que le llegaron, actualiza su estado y manda mensajes a sus vecinos. El sistema reparte los vértices entre máquinas y sincroniza los superpasos con una barrera — el modelo *bulk synchronous parallel* que Leslie Valiant había publicado en 1990 en Communications of the ACM. Cada vértice trabaja sólo con su trozo; el grafo nunca necesita estar entero en un sitio.

Y fíjate en la cronología de lo que vino DESPUÉS, porque es la trama de este capítulo. **Giraph** clonó Pregel en open source (y Facebook lo escaló a billones de aristas). **GraphChi** (Kyrola, OSDI 2012) invirtió la apuesta: nada de clúster — mil millones de aristas desde el DISCO de un solo PC, con *parallel sliding windows* que procesan el grafo por bloques sin cargarlo. **GraphX** (Xin et al., OSDI 2014) lo unificó todo dentro de Spark. Dos familias sobreviven de aquella era: los que **materializan** una copia compacta del grafo y la iteran muchas veces (Pregel, Giraph, GDS), y los que **procesan sin cargar** (GraphChi y su estirpe). Hoy construyes las dos, a mano, sobre LiraDB.

## 26.1 Objetivo

Al terminar este capítulo sabrás **por qué ejecutar un algoritmo no es lo mismo que leer el grafo**, y habrás construido las dos estrategias con las que una base de datos real responde a esa diferencia. Cuatro piezas en `cap26_proyeccion.rs`:

1. **`ProyeccionPonderada`** — el grafo (o su subgrafo filtrado) materializado en memoria compacta, con pesos resueltos UNA vez. La API pública que la Parte V llevaba debiendo desde los caps. 22 y 24.
2. **`FronterasBfs`** — un `Iterator` perezoso que produce frontera a frontera leyendo el store bajo demanda, bajo un `Presupuesto` con `MotivoParada` explícito.
3. **`ContandoStore`** — el voltímetro: un wrapper que cuenta las lecturas que llegan de verdad al store, para VERIFICAR las promesas de los dos anteriores.
4. **`BitSet`** — el conjunto de visitados a mano: un bit por nodo, la lección de denso contra disperso.

Y el hito que cierra la Parte V: un BFS de profundidad 2 sobre una cadena de 500 nodos que lee **2 de las 499 aristas**, y 5 Dijkstras que cuestan **11 lecturas en vez de 45**. Medidos, no prometidos.

## 26.2 Problema

La Parte V lleva cuatro capítulos ejecutando algoritmos SOBRE el store persistente, y cada uno leyendo arista a arista lo que necesita: `out_edges(u)` y `get_edge(e)` por cada relajación. Para UNA consulta, perfecto. Pero mira los dos extremos que ya tienes:

**El extremo iterativo.** El cap. 24 calculó closeness con un BFS por cada origen; el 25 iteró Louvain nivel a nivel. Llevado a Dijkstra sobre el store (cap. 22), cada llamada re-lee y re-valida E aristas. En una cadena de 12 nodos, cinco orígenes distintos leen 11+10+9+8+7 = **45 aristas**. ¿Para qué? Los pesos no cambiaron entre la primera y la quinta pasada. Estás pagando cinco veces la misma fotocopia. Y cada una de esas lecturas tiene precio doble desde el cap. 13: la que falla la caché del buffer pool cuesta una página de disco — comprarlas por adelantado, cuando ya sabes que vas a necesitar MUCHAS, es exactamente el trato que aquí cerramos.

**El extremo local.** «¿Quién está a 2 saltos de Ana?» Con lo que sabes, la respuesta honesta del motor es: materializa el grafo y recórrelo. Sobre una cadena de 500 nodos, eso es cargar 499 aristas para usar... 2. El 99,6% de lo leído se tira. Y lo de antes al revés: aquí la estrategia del cap. 22 (leer bajo demanda) era la BUENA, y materializar sería el disparate.

Dos síntomas opuestos, un mismo diagnóstico: **tratar todas las consultas igual**. Y una espina clavada desde el cap. 22: el CSR persistente del cap. 14 sólo guarda topología (offsets y targets) — sin ids de arista ni pesos no puede alimentar un Dijkstra. La deuda está escrita en el banner de aquel módulo.

La pregunta del capítulo: ¿qué existe en memoria en cada momento, y quién decide cuándo?

## 26.3 Modelo mental

Piensa en un **archivo provincial** con dos formas de trabajar:

- **La biblioteca que fotocopia UNA sección.** Vas a necesitar esa sección muchas veces (un análisis, una tesis). Pagas UNA fotocopia completa de la sección que te interesa — ni más (filtro: sólo Personas, sólo KNOWS) ni menos — y trabajas sobre tu copia. El archivo sigue recibiendo documentos nuevos mientras tanto, pero tu copia es una **foto**: inmutable, coherente, tuya. Materializar = `ProyeccionPonderada`.
- **El archivista que va hoja a hoja.** Sólo necesitas saber qué hay a dos carpetas de la tuya. Le pides la carpeta, la miras, pides las que cita, y le dices basta. Nunca pidió lo que no necesitabas. Le pones un tope — «máximo 10 carpetas» — y él te deja una nota final: si trajo TODO lo que había o se quedó sin permiso. Streaming = `FronterasBfs` + `Presupuesto` + `MotivoParada`.

Y en la puerta del archivo, **un contador de solicitudes**: el archivista puede presumir de trabajar poco; el contador no sabe presumir, sólo sumar. Ese es `ContandoStore`.

```
               ┌─ MATERIALIZAR (K iteraciones) ─────────────────────────┐
               │  store ──proyectar(filtro)──► copia CSR (pesos ya)     │
 store vivo ───┤                                    │                   │
 (OLTP sigue   │                        K algoritmos │ CERO lecturas     │
  mutando)     │                        sobre la foto │ del store        │
               └────────────────────────────────────────────────────────┘
               ┌─ STREAMEAR (consulta local) ───────────────────────────┐
               │  store ──frontera 0──► frontera 1──► frontera 2 ──║ corte
               │            lee SÓLO la adyacencia que expande      ║ presupuesto
               │            memoria ∝ visitados, nunca ∝ grafo      ║ MotivoParada
               └───────────────────────────────────────────────────────┘
```

El momento ¡ajá!: «¿qué hay a 2 saltos de Ana?» no necesita el grafo — necesita DOS adyacencias. Y «Dijkstra desde todos los nodos» no necesita releer el grafo V veces — necesita UNA fotocopia. El tipo de consulta decide la estrategia; el motor sólo tiene que ofrecerte las dos.

## 26.4 Primera solución

La solución ingenua ya la tienes y funciona: es la de los caps. 22-23. Cada algoritmo lee el store cuando lo necesita, arista a arista, sin copia previa. Cero infraestructura nueva. Para una consulta suelta, es incluso la opción correcta.

Y su gemela simétrica, igual de tentadora: «pues materializo siempre el grafo entero al abrir la base de datos, y todos los algoritmos trabajan sobre la copia».

## 26.5 Sus límites

Ambas se rompen en el extremo contrario:

1. **Leer-bajo-demanda multiplica por K.** Cada Dijkstra del cap. 22 valida los E pesos eager (una BD prefiere fallar ruidosamente a contestar casi-bien — política que NO vamos a ablandar) y luego relaja leyendo del store. El closeness ponderado que el cap. 24 dejó apuntado como deuda exigiría V Dijkstras: V·E lecturas. Las 45 de la cadena de 12 son 11 útiles + 34 repetidas.
2. **Materializar-siempre castiga lo local.** La cadena de 500: cargar 499 aristas para contestar con 2. Y de regalo, O(grafo) de memoria para una consulta O(2 saltos) — el título del capítulo, literal.
3. **La copia ingenua no sabe de filtros ni de huecos.** Sin más, copiarías nodos borrados (huecos de `delete_node`, cap. 16) y aristas que la consulta no puede pisar (hacia nodos fuera del subgrafo: un subgrafo no tiene aristas colgando hacia nodos que no contiene).

Conclusión: no hay UNA estrategia buena; hay DOS, y la consulta elige. Empecemos por la primera.

## 26.6 Solución evolucionada, parte 1: la proyección materializada

`ProyeccionPonderada::proyectar(store, &WeightSource, &FiltroProyeccion)` hace UNA pasada y devuelve una copia compacta e inmutable. Su layout es el CSR del cap. 14 **completado** — lo único que allí no cabía en disco, aquí sí en memoria:

```text
nodes:   [id0, id1, ...]     ids ordenados          (determinismo)
index:   id → posición densa compacta huecos        (herencia del cap. 24)
offsets: [0, g0, g0+g1, ...]  fronteras de fila u32  (EL CSR del cap. 14)
targets: posiciones destino   ← lo ÚNICO que persiste el cap. 14
pesos:   f64 por arista       ← lo que AÑADE este capítulo
aristas: EdgeId por arista    ← lo que AÑADE este capítulo
```

Tres decisiones que valen un porqué cada una:

**¿Por qué el filtro hace que las aristas de nodos excluidos NI SE LEAN?** Porque `proyectar` itera las ADYACENCIAS de los nodos admitidos — no todas las aristas. Una arista cuyo ORIGEN está fuera del filtro jamás llega a `get_edge`: su adyacencia no se consulta. El ahorro del subgrafo no es «leer y tirar», es no leer. Mira el bucle, que es todo el secreto:

```rust
for &u in &nodes {                    // SÓLO nodos admitidos: la adyacencia
    for eid in store.out_edges(u) {   // de un EXCLUIDO jamás se itera
        let edge = store.get_edge(eid)?;
        stats.edges_scanned += 1;
        if !filtro.admite_arista(edge) { stats.descartadas += 1; continue; }
        let destino = match index.get(edge.target).copied().flatten() {
            Some(d) => d,
            None => { stats.descartadas += 1; continue; } // destino fuera
        };
        let w = edge_weight(edge, weight)?;               // peso UNA vez
        fila.push((destino, eid, w));
    }
}
```

Y es medible: en el test de la red mínima (2 Personas + 1 Ciudad, aristas 0→1 KNOWS, 1→4 VIVE_EN, 4→0 VIVE_EN), proyectar sólo Personas da `edges_scanned = 2` (las adyacencias de los dos nodos vivos) y `descartadas = 1` — la 1→4 se leyó y se descartó (su destino no entra), pero la 4→0 **no aparece ni en descartadas: no existe para esta proyección**. Esa es la diferencia entre filtrar al leer y filtrar leyendo.

**¿Por qué los pesos se resuelven UNA vez, con la semántica ESTRICTA del cap. 22?** Porque la calidad del dato no se negocia por el lado analítico: prop ausente/NULL = `MissingWeight`, tipo no numérico = `InvalidWeight`, NaN/±∞ = `NonFiniteWeight` — el mismo `edge_weight`, el mismo contrato, ahora pagado O(E) una sola vez en la vida de la copia. Es el trato de la analítica: 11 lecturas que valen para las 5 consultas siguientes. Los pesos negativos NO son error de proyección (Bellman-Ford los admite): los rechaza `dijkstra_proyeccion` eager sobre TODA la copia — y aquí está la sutileza económica: `dijkstra_proyeccion` valida por llamada, pero `closeness_ponderado` valida UNA vez y llama V veces al núcleo sin validar. La sanidad se paga una vez; quien itera, no re-paga.

**¿Por qué es un snapshot y no una vista viva?** Porque inmutable es una FEATURE: los V Dijkstras del closeness recorren la MISMA foto (resultados consistentes entre sí, determinismo total — dos proyecciones del mismo store son `PartialEq` idénticas), y el store puede seguir recibiendo escrituras OLTP mientras la analítica corre. La separación OLTP/analítica del guion no es un diagrama: es un tipo que congela un instante. (¿Qué garantiza que el instante fue coherente? Nada todavía — eso es la Parte VI.)

Sobre la foto, las deudas se pagan solas: `dijkstra_proyeccion` reproduce punto por punto al del cap. 22 (mismas distancias, mismos caminos, misma arista elegida entre paralelas — test `dijkstra_proyeccion_coincide_con_dijkstra_store`), y `closeness_ponderado` hace lo que el cap. 24 prometió: Wasserman-Faust con distancias PONDERADAS. En la cadena 0→1→2→3 con pesos 1, 5, 1: el nodo 0 cae de 3/6 (saltos) a 3/14 (Σd = 1+6+7), el 1 a 4/33, y el 2 ni se entera (1/3 en ambas: su mundo no toca la arista cara). La fuente de pesos cambia la respuesta — la lección del cap. 22, ahora en centralidad.

## 26.7 Solución evolucionada, parte 2: streaming por fronteras con presupuesto

La otra mitad del título. `bfs_fronteras(store, origen, dir, presupuesto)` devuelve un `FronterasBfs` que **es un `Iterator` de verdad**:

```rust
let mut it = bfs_fronteras(&s, 0, Out, Presupuesto::profundidad(2))?;
it.next();  // Some([0])  — el origen
it.next();  // Some([1])  — leyó 1 arista
drop(it);   // la adyacencia del 1 JAMÁS se consultó
```

**¿Por qué un Iterator y no un `Vec<Vec<NodeId>>` de niveles?** Porque el Vec materializa TODO antes de que empieces: pagas el recorrido completo para quizás mirar dos niveles. El iterador es perezoso de verdad: la frontera k+1 no existe hasta que la pides — su adyacencia se lee al EXPANDIR, nunca antes. El que consume decide cuándo parar (un callback que encontró lo que buscaba suelta el iterador y ahí se acabó la factura). Test: `bfs_iterador_perezoso_una_frontera` — pedir 2 fronteras de una cadena de 6 deja el voltímetro en exactamente 1 lectura de arista.

**¿Por qué el `Presupuesto` se comprueba ANTES de cada lectura?** Porque un límite que se puede superar no es un límite. Mira dónde viven los chequeos — dentro del bucle, no por frontera:

```rust
for eid in eids {
    if let Some(max) = presupuesto.max_lecturas
        && stats.aristas_leidas >= max {
        terminado = Some(MotivoParada::PresupuestoLecturas);
        break 'nodos;                       // ANTES de leer
    }
    let edge = store.get_edge(eid)?;         // la lectura autorizada
    stats.aristas_leidas += 1;
    // ... y el de nodos, ANTES de marcar el descubrimiento:
    if let Some(max) = presupuesto.max_nodos
        && stats.nodos_visitados >= max { /* corte */ }
}
```

`Presupuesto{max_profundidad, max_nodos, max_lecturas}` se valida antes de cada `get_edge` y antes de cada descubrimiento: las promesas son EXACTAS — «máximo 2 lecturas» produce exactamente 2. Y `max_lecturas` es el presupuesto más importante de los tres en un store en disco: acotar lecturas es acotar el tiempo Y la memoria de trabajo de una sola vez. (Un presupuesto de 0 no tiene sentido — «no empezar» se consigue no llamando — y se rechaza con error tipado.)

**¿Por qué `MotivoParada` es parte de la RESPUESTA?** Porque cambia lo que el resultado significa. `Completo`: se agotó la componente — tu lista de nodos ES la respuesta. `PresupuestoNodos` o `PresupuestoLecturas`: había más y no te lo puedo dar — tu lista es un recorte, y tomar decisiones sobre él como si fuera completo es mentir con estadística. `ProfundidadMaxima`: cortaste tú, en el borde exacto que pediste. El test lo clava: un nodo aislado con presupuesto de 1 nodo acaba en `Completo` (no había nada más), no en `PresupuestoNodos` — mismo recorrido, significados opuestos.

Detalle fino que la dirección regala: en streaming, `GraphDirection::In` es GRATIS — se leen las `in_edges` bajo demanda, no hay que transponer nada (la proyección dirigida-out habría necesitado una segunda copia). Y `Both` deduplica por bitset, documentando que un store simetrizado a mano paga cada par dos veces: visible en las stats, no escondido.

`bfs_streaming` es la versión de una tirada: consume el iterador y te devuelve niveles + stats + `MotivoParada` en un `RecorridoBfs`. Mismo motor, dos ergonomías.

## 26.8 Solución evolucionada, parte 3: el voltímetro y el bitset

Todo lo anterior PROMETE lecturas («2 de 499», «45 contra 11»). ¿Quién verifica la promesa? No el propio algoritmo: **no confíes en que el código se auto-auditore** — el mismo bug que infla la optimización podría desinflar el contador. `ContandoStore` es un voltímetro: un wrapper de sólo lectura sobre `&dyn GraphStore` cuyos `get_edge`/`get_node`/`out_edges`/`in_edges` suman en contadores `Cell` y delegan:

```rust
fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
    self.lecturas_arista.set(self.lecturas_arista.get() + 1); // Cell: los
    self.inner.get_edge(id)                                    // métodos son &self
}
fn put_edge(&mut self, _: Edge) -> Result<(), StoreError> {
    panic!("ContandoStore es un instrumento de medida de sólo lectura")
}
```

Fíjate en las dos formas: `Cell` porque el trait de lectura va con `&self` (contar es una mutación invisible para el sistema de tipos), y el `panic` en las escrituras porque un instrumento de medida no es un store — si dejases escribir a través del voltímetro, dejarías de poder confiar en lo que cuenta. Los tests enchufan el BFS o la proyección al voltímetro y exigen que la stats interna y el contador externo — dos fuentes INDEPENDIENTES — coincidan. Es el patrón de medir desde fuera, y su lección trasciende el capítulo: cuando un sistema se autoinforma, pon un instrumento que él no controle.

Y el `BitSet`: el conjunto de visitados del BFS, a mano — `Vec<u64>`, un bit por id. **¿Por qué aquí BitSet y en el filtro HashSet?** Denso contra disperso. Los ids del store nacen densos (cap. 7): un bitset gasta 1 bit por id posible — 1/8 de lo que ocupa un `usize` en un HashSet, sin hashing ni rehash, con la consulta «¿ya visité este vecino?» en dos instrucciones. Crece bajo demanda (una palabra cada 64 ids) y su espacio es O(id_máximo_visitado/8). Pero el `FiltroProyeccion` guarda STRINGS arbitrarios (etiquetas, tipos) dispersos por definición: un bitset no puede indexarlos sin un diccionario previo, y entonces ya tienes un HashSet con pasos extra. Regla: universo denso de enteros → bitset; claves dispersas o arbitrarias → tabla hash. (¿Y si los ids fueran gigantes y dispersos? El bitset pagaría palabras vacías: el doc del `BitSet` lo deja escrito, y el HashSet ganaría. Aquí no pasa: los ids nacen del contador del store.)

## 26.9 Prueba de fuego: la tesis y la economía, medidas

**El test-tésis** (`bfs_streaming_no_lee_todo_el_grafo`): cadena de 500 nodos, 499 aristas. BFS profundidad 2 desde el 0:

```text
niveles          = [[0], [1], [2]]
parada           = ProfundidadMaxima
nodos_visitados  = 3        aristas_leidas = 2   ← 2 de 499 (<0,5%)
voltímetro       = 2        ← la stats interna NO mintió
```

Dos aristas leídas — no tres: con profundidad 2 sólo se EXPANDEN los nodos 0 y 1; para DESCUBRIR el 2 basta leer la arista de 1, y el 2 no se expande porque su frontera ya excede el presupuesto. El resto del grafo — 497 aristas — no existió para esta consulta. Esa es la tesis del capítulo en un `assert`.

**El test-economía** (`economia_multiorigen_una_lectura_por_adelantado`): misma cadena de 12 (E = 11). Por la proyección: materializar = 11 lecturas; 5 Dijkstras después = SIGUEN siendo 11. Directo contra el store (cap. 22): 11+10+9+8+7 = **45**. Cuatro veces más, y cada origen extra agranda la brecha. (¿Por qué 45 y no 55? La validación eager del cap. 22 usa `iter_edges` — el voltímetro cuenta `get_edge` — y cada Dijkstra del origen i expande los nodos i..11 y lee 11−i aristas. Calibrar contadores se hace trazando el código a mano, nunca «lo que suena razonable».)

Y las dos mitades del capítulo se dan la mano en el grafo demo de la Parte IV: el BFS streaming desde Ana produce niveles `[0], [1, 4], [2, 5]` (Dani inalcanzable: sólo su self-loop), y esos visitados son EXACTAMENTE los `alcanzados()` del Dijkstra sobre la proyección — misma componente, dos estrategias, una verdad (`bfs_en_demo_graph_alcance_y_contraste_con_dijkstra`).

**Las deudas, saldadas ante notario**: `dijkstra_proyeccion_coincide_con_dijkstra_store` (consistencia proyección↔algoritmo, la deuda del cap. 22) y `closeness_ponderado_paga_la_deuda_del_cap24` (el closeness ponderado, con consenso contra la versión por saltos cuando el peso es `Constant(1.0)` y CERO lecturas del store en los V Dijkstras). La Parte V cierra sin facturas pendientes.

¿Y si te saltas el capítulo? Síntoma detectable: tus analíticas tardan proporcionalmente a iteraciones (cada pasada relee), tus consultas locales tardan proporcionalmente al GRAFO, y no puedes demostrar ninguna de las dos cosas porque no tienes voltímetro.

## 26.10 Repaso de la Parte V: la cadena 22→26

Este capítulo cierra la Parte V. Reconstruyamos el árc completo — cada capítulo ejecutó el algoritmo académico del Vol.I SOBRE el store persistente, y cada uno descubrió que los datos viven en el disco:

```
 22 CAMINOS MÍNIMOS ──► 23 A* ──► 24 CENTRALIDAD ──► 25 COMUNIDADES ──► 26 SIN AGOTAR MEMORIA
    pesos de PROPS        heurística    V iteraciones         K niveles        la estrategia:
    del EDGE, semántica   del NODE      → proyección          → grafo          MATERIALIZE o
    ESTRICTA (fail loud)  (coords km)   PRIVADA sin pesos     simétrico        STREAM según consulta
    │                     │             + índice denso        propio           └─ la API pública
    └─ deuda: CSR no pesa └─ refactors  └─ deuda: ponderado  └─ decisión:     que las salda todas
       (cap. 14)             compartidos    y proyección        CONVIVIR         + el voltímetro
                           (validate_     pública               (no unificar)
                            edge_weights)
```

Cada eslabón dejó una garantía heredada: el **22** fijó el contrato de calidad de pesos (estricto, ruidoso) y anotó que el CSR del 14 no podía pesarlo; el **23** descubrió que los algoritmos también necesitan datos del NODO, y extrajo `validate_edge_weights` compartido; el **24** tropezó con la K-iteración y se construyó la primera proyección — privada, sin pesos, con el índice denso que aquí reutilizas — dejando apuntado «cuando exista la proyección con pesos, el BFS de saltos se cambia por Dijkstra»; el **25** decidió CONVIVIR (su `GrafoPonderado` simétrico codifica el contrato de Louvain; unificar a la fuerza sería re-asegurar 597 tests por cero valor). Y el **26** reúne: hereda el layout del 14, el contrato del 22, el índice denso del 24, la sabiduría de convivencia del 25 — y añade lo que ninguno tenía: la decisión explícita de qué existe en memoria, y el instrumento para demostrarla. El método de la Parte V en una frase: **el algoritmo es el fácil; lo difícil es ejecutarlo sin mentir sobre los datos ni quebrar la memoria.**

Estas dos piernas — materializar con filtro y streamear con presupuesto — son también las que sostendrán el futuro inmediato del libro: en el Vol.III, el cap. 51 montará GraphRAG sobre exactamente esto (PageRank personalizado multi-hop sobre proyecciones de una base de conocimiento que no cabe en memoria, con fronteras acotadas por presupuesto). Antes, la Parte VI contestará la pregunta que este capítulo deja abierta a propósito: ¿quién garantiza que la FOTO fue coherente?

## 26.11 Qué hemos sacrificado

1. **Paralelismo real, documentado no implementado**: `bloques_de_nodos(tam)` reparte rangos de posiciones — cada bloque un slice CSR independiente, perfectamente divisible entre hilos. Pero `&dyn GraphStore` no es `Sync` y el workspace no usa crates. La semilla queda; cómo lo paralelizan GDS y Kùzu es prosa de §26.12.
2. **Snapshot sin aislamiento transaccional**: la proyección es inmutable, pero ¿fue coherente el instante fotográfico? Nada lo garantiza aún: eso exige transacciones (Parte VI) y MVCC (cap. 30).
3. **Proyección dirigida-out fiel**: paralelas y self-loops se conservan tal cual (la proyección FOTOGRAFÍA, no interpreta). Quien necesite simetría o unión, la construye encima — como hicieron los caps. 24/25.
4. **El presupuesto cuenta `get_edge`, no `iter_edges`**: la validación eager del cap. 22 escapa al voltímetro (por eso 45, no 55). Contar el catálogo entero de accesos difiere el instrumento; documentado.
5. **Convivencia antes que pureza**: tres estructuras de proyección coexisten (la pública de aquí, la del 24, la del 25). El refactor unificador queda como deuda declarada, no oculta.

## 26.12 Cómo lo hace una BBDD real

- **Neo4j GDS**: `gds.graph.project()` materializa una proyección EN MEMORIA (native projection por configuración, o Cypher projection por consulta) con exactamente nuestros tres ejes: qué nodos, qué tipos de relación, qué propiedades. Vive en un graph catalog con estimación de memoria previa (`estimate`) y se suelta con `gds.graph.drop`. Es nuestra `ProyeccionPonderada` con catálogo y factura de RAM.
- **Kùzu** invierte el diseño: almacenamiento COLUMNAR (CIDR 2023). Las propiedades viven en columnas, de modo que una analítica que necesita 2 propiedades de 20 lee DOS columnas del disco — nada que copiar a RAM. Kùzu hace barato lo que nosotros materializamos; nosotros materializamos lo que Kùzu hace barato.
- **Pregel/Giraph**: materializar y iterar por superpasos BSP (Valiant 1990) — la proyección repartida entre máquinas, frontera de mensajes en cada barrera. Facebook escaló Giraph a billones de aristas.
- **GraphChi** (OSDI 2012): el streaming extremo — parallel sliding windows procesan el grafo POR BLOQUES desde el disco de un PC, sin cargarlo nunca entero. La estirpe de nuestro `FronterasBfs` bajo presupuesto de lecturas.
- **GraphX** (OSDI 2014): unifica grafo-paralelo y dato-paralelo en Spark, reinterpretando los superpasos como joins distribuidos — la prueba de que las dos familias eran dos vistas del mismo problema.
- **DuckDB**, fuera del mundo grafo: su buffer manager derrama a disco temporal cuando la consulta no cabe en RAM (out-of-core). Nuestro `Presupuesto` es su primo pequeño: en vez de reaccionar al desbordamiento, lo prohíbes de antemano.
- **El survey del guion**: (a) *paralelismo* — las fronteras y los bloques CSR son divisibles; GDS y Kùzu paralelizan exactamente por bloques de nodos; (b) *snapshots* — la foto inmutable es la separación OLTP/analítica encarnada; el paso siguiente es MVCC (cap. 30); (c) *OLTP vs analítica* — el punto (`get_edge` sobre el vivo) contra el recorrido (K pasadas sobre la foto): dos cargas, dos estrategias, una base de datos que ofrece ambas.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en el test de la Persona-only, ¿por qué `descartadas` es 1 si DOS aristas (1→4 y 4→0) quedan fuera de la proyección?
- *Intermedio*: ¿por qué el presupuesto de lecturas se comprueba DENTRO del bucle de aristas y no por frontera? Construye un caso donde la diferencia sea visible.
- *Experto*: tu `Presupuesto` no distingue lecturas de PÁGINA (cap. 13) de lecturas de arista. ¿Qué mediría un voltímetro de páginas sobre un BFS frontera a frontera, y por qué la localidad del CSR lo favorece?

## 26.13 Lo que te llevas

- **Dos estrategias, no una**: K iteraciones sobre todo el grafo → materializar UNA vez (E lecturas); consulta local a k saltos → streaming frontera a frontera (k adyacencias). El tipo de consulta decide.
- **La proyección es el CSR del cap. 14 completado**: mismo layout, más el peso y el id de arista que el disco no podía pagar; índice denso compactando huecos; determinismo total.
- **Filtrar aquí es no leer**: las aristas de nodos excluidos NI SE LEEN (ahorro medible en `edges_scanned`/`descartadas`).
- **La sanidad de pesos se paga UNA vez**: semántica estricta heredada del 22, validación eager en `closeness_ponderado` FUERA del bucle de orígenes — 11 lecturas que valen para 5 consultas (45 de la vía directa).
- **Un Iterator, no un Vec**: la frontera k+1 no existe hasta que la pides; soltarlo a mitad corta la factura.
- **`MotivoParada` es parte de la respuesta**: `Completo` y `PresupuestoNodos` son recorridos con el mismo aspecto y significados opuestos.
- **El voltímetro externo**: dos fuentes independientes (stats interna + `ContandoStore`) que deben coincidir; nunca confíes en el autoinforme.
- **BitSet donde es denso, HashSet donde es disperso**: un bit por id si los ids nacen contiguos; tabla hash si las claves son arbitrarias.

## 26.14 Ojo, cuidado con…

- **Interpretar sin mirar `parada`**: tres nodos visitados puede ser la componente entera o el corte del presupuesto. Primero `parada`, después conclusiones.
- **Calibrar contadores de oído**: profundidad k ⇒ expandir k nodos (2 aristas leídas, no 3). Los tests de contadores se calibran trazando el código a mano.
- **Presupuestos a posteriori**: comprobar el límite tras la lectura permite superarlo de a una — el chequeo va ANTES de cada `get_edge` y de cada descubrimiento.
- **Bitset con ids dispersos**: ids gigantes y huecos pagan palabras vacías; ahí el HashSet gana. El bitset es denso o no es.
- **Confundir la proyección con una vista viva**: es una FOTO. El store siguió mutando; tu analítica contesta sobre el instante de la foto, no sobre el ahora.

## 26.15 Pin de batalla

> *«Un resultado recortado por presupuesto no es una respuesta a medias: es una promesa a medias. Sin MotivoParada, ni siquiera sabes cuál de las dos te dieron.»*

## 26.16 Si solo lees 30 segundos

Materializar (`ProyeccionPonderada`) paga E lecturas UNA vez y luego itera K algoritmos con CERO lecturas del store — 11 contra 45 en la cadena de 12. Streamear (`FronterasBfs`) lee frontera a frontera bajo demanda: profundidad 2 en la cadena de 500 lee 2 aristas de 499. El `Presupuesto` (profundidad/nodos/lecturas) nunca se supera porque se comprueba antes de leer, y `MotivoParada` dice si tu respuesta está completa o recortada. El `ContandoStore` verifica todo desde fuera: dos fuentes independientes, ninguna se auto-audita. BitSet si los ids son densos; HashSet si las claves son dispersas.

## 26.17 Una historia pequeña

La migración de este capítulo casi se pierde. El agente que escribía el módulo quedó cortado por usage-limit con las 2.150 líneas COMPLETAS pero sin cablear en `lib.rs`, sin compilar, y con cuatro tests mal calibrados. El orquestador lo terminó a mano: dos errores de compilación (`sort_unstable` no ordena `f64` — se necesita `total_cmp`; y el `#[derive(Debug)]` no sabe imprimir un `&dyn GraphStore` — impl manual con los contadores), cuatro lints, y luego lo bueno: los cuatro tests de contadores que fallaban.

Uno decía `5·E = 55` lecturas para 5 Dijkstras; el voltímetro decía 45. ¿Quién mentía? Nadie: la validación eager del cap. 22 usa `iter_edges` — que el voltímetro no cuenta como `get_edge` — y cada origen i lee E−i aristas: 11+10+9+8+7. Otro decía que el BFS de profundidad 2 leía 3 aristas; la traza de `FronterasBfs::next` a mano demostró que se EXPANDEN 2 nodos, no 3 — descubrir el nodo 2 no exige expandirlo. Las expectativas se recalibraron una a una, cada una con su derivación al lado. La moraleja quedó escrita en la bitácora de migración: los tests de contadores no se calibran con lo que suena razonable; se calibran trazando el código. El voltímetro no opina: suma.

## Ejercicios resueltos

**1. En el filtro Persona-only de §26.6, ¿por qué `descartadas == 1` si DOS aristas quedan fuera de la proyección (1→4 y 4→0)?**

Porque `descartadas` cuenta lo que se LEYÓ y se descartó, no todo lo que queda fuera. La arista 1→4 se lee (la adyacencia del nodo 1 — admitido — se itera) y se descarta: su destino (la ciudad) no pasa el filtro de nodos. La arista 4→0 NI SE LEE: su origen (la ciudad) está excluido y `proyectar` jamás consulta su adyacencia — por eso `edges_scanned == 2` (las adyacencias de los dos nodos admitidos) y la 4→0 no aparece en ninguna stats. El ahorro del subgrafo: no pagar lecturas de lo que ni entra. Verificación: `subgrafo_filtrado_por_label_y_tipo_de_arista`.

**2. ¿Por qué 5 Dijkstras del cap. 22 sobre la cadena de 12 son 45 lecturas y no 55 (5×11)?**

Dos razones que hay que separar. Primero, no cada Dijkstra lee las 11: el origen i sólo expande los nodos i..11, luego lee 11−i aristas — la suma es Σ(E−i) = 11+10+9+8+7 = 45. Segundo, la validación eager de pesos del cap. 22 recorre `iter_edges` (un iterador del store), que el voltímetro NO cuenta como `get_edge` — por eso no suma ni una lectura más. Con la proyección: 11 lecturas de materialización y las 5 consultas no tocan el store. Verificación: `economia_multiorigen_una_lectura_por_adelantado` (y su comentario con la derivación exacta).

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin ejecutar nada, sobre la estrella 0→{1,2,3,4} (cuatro aristas 0→i) con `Presupuesto::sin_limite().con_nodos(3)`: predice `niveles`, `nodos()`, las cuatro stats (`nodos_visitados`, `aristas_leidas`, `adyacencia_consultas`, `fronteras`) y el `MotivoParada`. Luego verifica con `bfs_streaming` envuelto en un `ContandoStore`: stats interna y voltímetro deben coincidir. Repite con el origen aislado y presupuesto 1. *Pistas*: (1) ¿el límite de nodos se comprueba antes o después de descubrir?; (2) ¿cuántas aristas se leen hasta descubrir la segunda hoja?; (3) ¿qué `MotivoParada` espera a un grafo que ya no tiene nada? *Criterio*: predicción exacta de niveles+parada y coincidencia stats/voltímetro (compárate con `bfs_streaming_presupuesto_nodos_exacto`).

**Intermedio (analizar — mezcla caps. 22 y 24).** La cadena 0→1→2→3 con pesos 1, 5, 1. (a) Calcula a mano el closeness ponderado de los cuatro nodos (Wasserman-Faust con Σd ponderado). (b) Explica por qué 0 y 1 se devalúan respecto a la versión por saltos y el 2 no cambia. (c) ¿Qué error tipado esperas con pesos negativos, quién lo lanza y CUÁNTAS veces valida `closeness_ponderado`? Verifica con `closeness_ponderado_paga_la_deuda_del_cap24` (allí están los valores 3/14, 4/33, 1/3, 0 y el test de economía con voltímetro). *Pistas*: (1) ¿qué Σd y qué r para cada origen?; (2) ¿qué aristas toca el mundo alcanzable del 2?; (3) ¿dónde corre `validar_pesos_no_negativos` respecto al bucle de orígenes? *Criterio*: números exactos + la diferencia semántica salto/peso + la validación UNA vez.

**Experto (crear — cierre de Parte V, retrieval puro).** Primera parte, de memoria (sin mirar los banners de los caps. 22-25): reconstruye el árc de la Parte V — qué añadió cada capítulo (22: pesos estrictos de props; 23: heurísticas del nodo; 24: familias + proyección privada e índice denso; 25: comunidades + grafo simétrico propio) y qué garantía o deuda dejó cada uno. Segunda parte: implementa `pagerank_proyeccion(&ProyeccionPonderada)` — damping validado en (0,1), masa dangling redistribuida uniformemente, convergencia L1 (todo lo aprendiste en el 24) — con CERO lecturas del store tras materializar (voltímetro en el test) y verifica que sus scores coinciden con el `pagerank` del cap. 24 sobre el mismo grafo simetrizado. *Pistas*: (1) ¿qué convención de self-loops y paralelas debes re-declarar para que la equivalencia sea justa?; (2) ¿dónde del bucle por iteraciones se escapa la masa dangling?; (3) ¿por qué aquí no hay validación de pesos DENTRO del bucle? *Criterio*: árc completo de memoria + equivalencia con el 24 + cero lecturas medidas.

## Para profundizar

- **Malewicz et al., «Pregel: a System for Large-Scale Graph Processing» (SIGMOD 2010, pp. 135-146, DOI 10.1145/1807167.1807184)** — el paper que bautizó «think like a vertex» y los superpasos. La anécdota de la esquina, en fuente primaria.
- **L. G. Valiant, «A Bridging Model for Parallel Computation» (CACM 33(8), 1990)** — el modelo BSP en el que Pregel se apoyó veinte años después.
- **A. Kyrola, «GraphChi: Large-Scale Graph Computation on Just a PC» (OSDI 2012)** — el streaming por bloques desde disco, en un solo PC: parallel sliding windows.
- **R. S. Xin et al., «GraphX: Unifying Data-Parallel and Graph-Parallel Analytics» (OSDI 2014)** — la unificación de las dos familias en Spark.
- **Neo4j Graph Data Science — docs de graph projection (native y Cypher), memory estimation y graph catalog** — la `ProyeccionPonderada` a escala industrial, con factura de RAM.
- **G. Jin et al., «Kùzu» (CIDR 2023) y Gupta et al., «Columnar Storage and List-based Processing for Graph Queries» (VLDB 2021)** — el camino columnar: leer poco en vez de copiar.
- **DuckDB, «Memory Management in DuckDB» (blog, 2024)** — out-of-core: derramar a disco cuando no cabe, el complemento reactivo de nuestro presupuesto preventivo.
- **McCune et al., «A Survey of Vertex-Centric Frameworks» (ACM Computing Surveys, 2015)** — el mapa de toda la familia TLAV/BSP.

## Mini-diálogo: a la puerta del archivo

> — Entonces, ¿materializo o streameo?
>
> — Pregúntale a la consulta, no a la moda. ¿Vas a recorrer el grafo K veces? Fotocópialo una vez. ¿Sólo quieres saber qué hay a dos saltos? Pide dos carpetas y di basta.
>
> — ¿Y si miento y me equivoco?
>
> — Para eso está el contador de la puerta. Tus stats dicen dos lecturas; el contador dice dos; bien. El día que digan cosas distintas, ya tienes el bug acorralado entre los dos.
>
> — ¿Y el presupuesto? Me da miedo cortar la respuesta a medias.
>
> — Al contrario: cortar a medias y SABERLO es lo honesto. `MotivoParada` es la diferencia entre «esto es todo» y «esto es lo que me dejaron ver». Lo peligroso no es el recorte: es el recorte sin etiqueta.

---

*(Próximo capítulo: 27 — Qué significa una transacción. Tu proyección es la foto de un instante… ¿pero quién garantiza que ese instante fue coherente, y que dos escritores no te rompan el store mientras analizas su foto? La Parte VI — ACID, WAL, recuperación — convierte la fe en contrato. Y más adelante, en el Vol.III, el cap. 51 montará GraphRAG sobre las piernas de éste: PPR multi-hop sobre proyecciones y fronteras de una base de conocimiento que no cabe en memoria.)*
# Capítulo 27 — Qué significa una transacción (ACID)

> *«"Commit" no es un botón: es una promesa. Y mientras el código no pueda distinguir un commit confirmado de un apply cortado, la durabilidad es un título que adorna un artefacto que todavía no la ha ganado.»*

## 27.0 La anécdota de la esquina

En junio de 1981, Jim Gray — por entonces en Tandem Computers, en Cupertino, California — presentó un paper invitado en la conferencia VLDB titulado «The Transaction Concept: Virtues and Limitations». En las primeras líneas resumía en una frase lo que llevaba años cocinándose en IBM Research: *una transacción es una transformación de estado que tiene tres propiedades: atomicidad (o todo o nada), durabilidad (los efectos sobreviven a los fallos) y consistencia (una transformación correcta)*. Y para ilustrar por qué el concepto importaba, no citaba ningún teorema: citaba **vuelos de avión**, transferencias electrónicas de fondos y alquileres de coches — esos sistemas donde una reserva que se queda «a medias» es un desastre.

La génesis era más antigua. En System R, el prototipo relacional de IBM de mediados de los setenta, ya corría un Recovery Manager — documentado en 1981 en «The Recovery Manager of the System R Database Manager» (Gray y coautores) — y los *predicate locks* de Eswaran, Gray, Lorie y Traiger ya hablaban de consistencia e aislamiento en 1976. Pero faltaba la palabra. El acrónimo **ACID** no lo acuñó Gray: fue Theo Härder y Andreas Reuter quienes lo consolidaron en 1983, en «Principles of Transaction-Oriented Database Recovery» (ACM Computing Surveys 15(4), pp. 287-317), donde lo llaman el *«ACID principle»*.

Detente en el giro que encierra esta historia: la atomicidad, la consistencia y la durabilidad existían como *problema* antes que como nombre. El nombre llegó después, como pegamento, para que cuatro términos hablasen de un único paradigma. Lo que haremos aquí es lo contrario: **despegar el acrónimo** y preguntar, para LiraDB, qué significa *de verdad* cada letra.

## 27.1 Objetivo

Al terminar este capítulo sabrás **qué es realmente una transacción**, y habrás construido en Rust la primera maquinaria que la representa. Cuatro piezas en `cap27_transacciones.rs`:

1. **`GarantiaAcid` + `NivelGarantia` + `InformeAcid`** — el vocabulario ACID como *tipo* y como *informe ejecutable* que los tests verifican. No es prosa: es un artefacto auditable que dice, de cada letra, hasta dónde llega hoy y qué capítulo la cerrará.
2. **`Transaccion` — el ciclo de vida `begin → stage* → commit|rollback`** — la transacción como *objeto* que acumula operaciones en un buffer y solo las aplica al commit, validándolas todas de golpe.
3. **`Operacion`** — la operación de escritura como *dato* (`PutNode/PutEdge/DeleteNode/DeleteEdge`), la pieza que hace posible el staging.
4. **`autocommit` e `informe_acid`** — que hacen visibles, una como función y otra como reporte, las dos caras de la moneda: el modo por defecto de los capítulos 7-26 era autocommit, y ninguna letra del ACID está completa todavía.

Y la lección que lo vertebra: **la honestidad**. `informe_acid()` te dirá que la A es parcial, que la C es parcial y trivial, que la I es parcial — *por diseño* — y que la D es ninguna. Aprender a decir eso, y a ejecutarlo en tests, es más importante que fingir ACID en un banner.

## 27.2 Problema

LiraDB lleva veintiséis capítulos escribiendo grafos. Mira el modo en que ha escrito desde el capítulo 7: cada `put_node`/`put_edge`/`delete_*` del `GraphStore` (cap. 8) te devuelve su `StoreError` si algo falla... y sigues. Pero fíjate en lo que tienes sembrado sin darte cuenta. Piensa en una arista entre **dos nodos nuevos**, ambos creados en la misma ráfaga:

```rust
store.put_node(Node::new(0, "A"))?;   // ok
store.put_node(Node::new(1, "B"))?;   // ok
store.put_edge(Edge::new(0, 0, 1, "KNOWS"))?;  // ok… ¿siempre?
```

Hoy, si la primera falla, las demás ni se intentan — pero nada te *agrupó* esas tres como una unidad. Cada `put_*` es su propia transacción: lo que una base de datos real llama **autocommit**. Y el autocommit tiene una debilidad concreta: un *lote* de diez operaciones en el que la quinta falla deja las cuatro anteriores aplicadas. Tu grafo queda **a medias**.

Tres síntomas aparecen:

1. **Un lote de 10 nodos que falla en el 5 deja 4 nodos en el store.** Nadie te pidió eso. Nadie sabe que eran «un lote».
2. **Dos operaciones relacionadas (arista + sus nodos) no se pueden agrupar** sin que un fallo entremedias las deje huérfanas.
3. **No sabes ni siquiera qué *es* «lo que pediste».** No tienes vocabulario para decir «esto son cinco operaciones que deben aplicarse juntas, o ninguna».

El problema de fondo: **el store aplica lo que le dices, cuando se lo dices**. No hay una unidad intermedia que te permita decir «reúne esto primero; cuando te diga, aplícalo todo de una vez». Y sin esa unidad, no hay transacción — solo una serie de comandos sueltos.

## 27.3 Modelo mental

Piensa en el **bloc de notas del contable** que solo escribe en el libro mayor cuando firma al final.

El contable tiene dos objetos físicos. Un **libro mayor** — el único con valor legal, donde se pasan a limpio los asientos. Y un **bloc de notas** — desechable, donde *anota* los asientos mientras los revisa. Las reglas son:

- Cada asiento se **anota en el bloc** primero (`stage`). El libro mayor no se toca todavía.
- Cuando el contable **firma** (`commit`), pasa a limpio el bloc **entero**, en un solo trazo.
- Si algo del bloc estaba mal — un asiento descuadrado, una fecha imposible — **no firma** y tira el bloc (`OperacionInvalida`). La transacción muere sin escribir nada.

Y hay dos detalles que se parecen sospechosamente a lo que vamos a programar:

- **La puerta del despacho tiene un único cerrojo** (`&mut`). Mientras el bloc está abierto, *nadie* puede tocar el libro mayor — ni otro contable ni un inspector que solo viniera a mirar. No es el resultado de un protocolo sofisticado: es un candado físico en la puerta.
- Si el contable **muere con el bloc abierto**, su caída *es* el descarte: la transacción muere sin haber escrito nada.

Pero el problema de este capítulo aparece exactamente donde el modelo se rompe: cuándo el contable **está a media firma** — pasando a limpio el bloc — y se le **cae el bolígrafo** (`ApplyFallido`). Los tres primeros asientos ya están en el libro, el cuarto no, y *no hay forma de saber cuánto llegó*. Ese es el hueco que el WAL del cap. 28 cerrará: un bloc con **copia de cada trazo registrada antes de tocar el libro**.

El diagrama del ciclo de vida, con el `&mut` como barra de la puerta:

```
   &mut store (el cerrojo)
   │
   ▼ ┌────────────────────────────────────────────────┐
begin │  buffer (bloc) = Vec<Operacion>               │
──►   │  stage(op1); stage(op2); … — NUNCA toca el store │
      │   stage inválida → se expulsa, la tx sigue viva  │
      │   commit() → valida TODO →→→ apply → libro mayor │
      │      (si algo falla: A MEDIAS — cap. 28)         │
      │   rollback() → descarta buffer (gratis)          │
      └────────────────────────────────────────────────┘
```

**El momento ¡ajá!**: *«commit» no es un botón, es una **promesa** — y mientras el código no pueda distinguir un commit confirmado de un apply cortado, la D no existe. Hasta entonces, la A es «o todas o ninguna frente a la validación», no «o todas o ninguna frente al universo». El WAL del cap. 28 cerrará la distancia entre la promesa y el universo.*

## 27.4 Primera solución

La primera solución *ya la tienes y funciona*: es la de los capítulos 7-26. Cada `put_node`/`put_edge`/`delete_*` del `GraphStore` es su propia transacción. El test `autocommit_equivalente_a_la_operacion_directa` lo clava: `store.put_node(n)` a pelo y `autocommit(store, Operacion::PutNode(n))` dejan el **mismo grafo** — `node_count()` y `edge_count()` idénticos. O sea: la forma en que LiraDB ha escrito hasta hoy no es un modo distinto del nuevo mecanismo; es **una transacción de una sola operación**, con el begin y el commit implícitos.

Para escrituras sueltas, es perfecto. El autocommit no es un error: es un caso particular.

## 27.5 Sus límites

El problema no es una escritura suelta. El problema es **agrupar operaciones que dependen unas de otras**. Cuatro límites concretos:

1. **Un lote de 10 operaciones en el que la 5ª falla deja las 4 anteriores aplicadas.** El grafo queda a medias — un estado intermedio que nadie decidió.
2. **Dos operaciones relacionadas no se pueden agrupar.** Una arista cuyos extremos son dos nodos NUEVOS es válida solo si los tres entran juntos. Con autocommit, un fallo entremedias deja la arista huérfana o los nodos sin conectar — exactamente lo que el staging evitará.
3. **No hay manera de «deshacer» ni siquiera lo que aún no se aplicó.** Si te equivocas en la operación 3 de 5, ¿revientas las 2 primeras? No hay transacción que descartar; ya escribiste.
4. **No puedes saber *qué* prometes.** Sin un `InformeAcid`, «tenemos transacciones» es una afirmación sin matices — y el matiz es todo.

Y la pregunta incómoda que plantea el límite: **¿qué significa «a medias»?** Para contestarla con rigor no basta con escribir código; hay que construir el vocabulario para decir *cuánto está completo* cada promesa. Empecemos por ahí.

## 27.6 Solución evolucionada

La evolución tiene dos mitades que se complementan, tal y como manda el contrato del capítulo: **el vocabulario honesto** y **la primera maquinaria**.

**Primera mitad — el vocabulario típado.** La tesis es que el ACID no es un interruptor que la base de datos enciende. Es un conjunto de **cuatro promesas independientes**, cada una con su nivel. Eso exige un tipo de tres valores — `NivelGarantia::Ninguna | Parcial | Completa` — y no un booleano: un bool esconde *cuánto falta*. Y exige que el informe sea un **artefacto ejecutable**: `informe_acid()` devuelve estructura que los tests comparan. Si la documentación prometiera más que el código, los tests lo delatarían.

**Segunda mitad — la transacción como objeto, con staging.** `Transaccion::begin(&mut store)` toma el préstamo exclusivo del store. Las operaciones se **acumulan** como `Operacion` en un `Vec<Operacion>` privado — el buffer. Cada `stage` valida **eager** contra el store *más las operaciones anteriores* (un replay sobre una `Simulacion`): si la operación es inválida, se **expulsa** con `OperacionInvalida{indice, causa}` y la transacción **sigue viva con su prefijo válido**. El `commit` re-valida el buffer **entero** (el punto de no retorno, segunda cerradura) y, solo si todo es válido, lo aplica operación a operación. El `rollback` descarta el buffer — y como nada se aplicó, es **gratis por construcción**.

Y una pieza de honestidad crítica: si el **apply real** falla a mitad — el store dice «no» a algo que la simulación aprobó, o el proceso muere entre dos escrituras — el `commit` devuelve `ApplyFallido{indice, aplicadas, causa}`: te dice *cuántas* operaciones llegaron, pero **no puede deshacerlas**. Ese hueco es el motor del próximo capítulo.

## 27.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap27_transacciones.rs` (~1.460 líneas, 28 tests), abriendo la Parte VI. Léelo por partes — cada decisión tiene un porqué.

### El vocabulario ACID como tipo

```rust
pub enum GarantiaAcid { Atomicidad, Consistencia, Aislamiento, Durabilidad }
pub enum NivelGarantia { Ninguna, Parcial, Completa }
pub struct EntradaAcid { pub garantia: GarantiaAcid, pub nivel: NivelGarantia,
    pub como_esta_hoy: &'static str, pub capitulo_que_la_cierra: u8 }
```

Cinco decisiones que valen un porqué cada una:

- **Cada letra es una variante de `GarantiaAcid`** que sabe su letra (`'A'…'D'`), su nombre largo y su **definición PARA LiraDB** — no la de un manual genérico. El test `garantia_acid_letra_nombre_definicion` lo exige.
- **`NivelGarantia` tiene tres niveles, no un bool.** La tesis del capítulo es que **ninguna** letra está completa todavía; un bool te diría «sí tengo ACID» y ocultaría el progreso. Tres niveles permiten decir, con precisión, «parcial», «ninguna».
- **`EntradaAcid.capitulo_que_la_cierra`** apunta a qué capítulo de la Parte VI construye lo que falta. El informe no solo dice *dónde* estás: dice *hacia dónde*.
- **El informe es ejecutable**: `informe_acid()` devuelve las cuatro entradas, y el test `informe_acid_tiene_las_cuatro_letras_en_orden` exige que la concatenación de letras sea exactamente «ACID». Una promesa de base de datos que no se ejecuta en tests es marketing; aquí, mentir **rompe CI**.
- **`Display` del informe** imprime cada entrada en el formato `A — Atomicidad: parcial / naive: … (cap. 28 lo cierra)` y cierra con la línea honesta: *«(W: nada de esto es durable sin WAL — cap. 28)»*.

### El informe honesto

El test `informe_acid_es_honesto_sobre_el_estado_actual` es la **tesis ejecutada**:

```rust
// A: PARCIAL — staging «o todo o nada» frente a VALIDACIÓN; un fallo durante
//    el APPLY real deja el store a medias. La cierra el cap. 28.
// C: PARCIAL y trivial — solo invariantes ESTRUCTURALES, sin restricciones
//    declarativas; la C es un contrato COMPARTIDO con la app. Cap. 30.
// I: PARCIAL por diseño — no hay CONCURRENCIA; el préstamo &mut ES el cerrojo
//    (borrow checker). Cap. 30.
// D: NINGUNA — commit() muta RAM; hay sync/flush pero falta el protocolo WAL.
//    Cap. 28.
```

Fíjate en el juego de palabras que el tipo permite: la A **parcial** (frente a la validación sí, frente al apply no), la C **parcial y trivial** (no hay esquema, solo invariantes estructurales; la C es un contrato *compartido* entre motor y aplicación), la I **parcial por diseño** (no hay motores de locks que construir — el `&mut` lo hace gratis), y la D **ninguna** (hay piezas a disco, pero no protocolo; confundir piezas con protocolo sería la mentira más cara del capítulo). El test asegura que la prosa coincide con el código: busca los strings «apply», «estructurales», «concurrencia» y «RAM» dentro del informe.

### Las anomalías, como vocabulario

```rust
pub enum Anomalia { LecturaSucia, ActualizacionPerdida }
```

Hoy **no pueden ocurrir** — `por_que_no_pasa_hoy()` lo dice: *«no hay concurrencia: mientras vive una Transaccion, el préstamo exclusivo &mut del store impide que cualquier otro lector o escritor lo toque»*. Pero el cap. 30 las hará posibles y las combatirá con MVCC/2PL. La lógica es la del cap. 22 con `NegativeCycle`: **primero se nombra el enemigo, luego se construye la defensa**. Si dejásemos el enum para el cap. 30, el lector llegaría al MVCC sin saber qué está combatiendo.

### La operación como dato

```rust
pub enum Operacion {
    PutNode(Node),
    PutEdge(Edge),
    DeleteNode(NodeId),
    DeleteEdge(EdgeId),
}
```

Es la pieza clave del staging. Mientras las operaciones son **valores** que se acumulan en un buffer, la transacción puede validarlas **todas juntas** antes de tocar el store — y descartarlas limpio en rollback. Y hay una herencia deliberada escondida aquí: **el mismo shape que `RecordKind` del cap. 10**. El append-only log que construiste entonces anticipa el formato; la `Operacion` de este capítulo es su heredera directa. Eso no es una coincidencia: el cap. 28 serializará *exactamente esto* al WAL, y como la forma ya existe, no habrá que reinterpretar nada. El comentario del módulo lo llama «la semilla del WAL».

### La transacción como objeto — y el ciclo de vida en los tipos

```rust
pub struct Transaccion<'a> { store: &'a mut dyn GraphStore, buffer: Vec<Operacion> }
pub fn begin(store: &'a mut dyn GraphStore) -> Self { … }            // toma el cerrojo
pub fn commit(self)   -> Result<ResumenCommit, TransaccionError> { … }  // consume self
pub fn rollback(self) -> ResumenRollback { … }                       // consume self
```

La decisión más hermosa de todo el módulo está en **dos letras**: `self`. `commit` y `rollback` **consumen** la transacción. **no existe el objeto «transacción cerrada»** — usarla tras su cierre es error de compilación, no de runtime; y **anidar dos transacciones sobre el mismo store no compila** — `begin` pide `&mut dyn GraphStore`, así que el préstamo exclusivo del modelo «un único escritor» del brief lo **ejecuta el borrow checker**, gratis, sin una línea de locking (el test `transacciones_secuenciales_el_prestamo_se_libera` demuestra la única forma de encadenar: `commit`/`rollback` consumen la tx y liberan el préstamo).

Y otra propiedad que fluye del diseño sin código extra: **`Drop` de una transacción activa es rollback implícito y seguro por construcción** (el test `drop_implicito_es_rollback_seguro`). Como nada se aplica fuera del commit, abandonar el scope (un `?` temprano, un pánico, un olvido) simplemente descarta el buffer — el store no se tocó.

### El staging: valida eager, y el commit revalida por inducción

```rust
pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
    self.buffer.push(operacion);
    match validar_buffer(self.store, &self.buffer) {
        Ok(()) => Ok(()),
        Err(e) => {
            self.buffer.pop();   // el prefijo era válido: solo pudo fallar la última
            Err(e)
        }
    }
}
```

`stage` valida **eager**: si la operación añadida rompe una invariante, se **expulsa** y la transacción **sigue viva con su prefijo válido**. El test `stage_rechaza_duplicado_dentro_del_buffer_y_la_tx_sigue_viva` lo demuestra: metes un duplicado, recibes `OperacionInvalida{indice:1, causa:DuplicateNode(0)}`, y el buffer sigue con la operación válida que metiste antes — la transacción continúa usable.

`commit` hará lo contrario: **revalida el buffer ENTERO** antes de tocar el store. Es redundante con `stage` (cada stage validó su prefijo, y nada externo puede cambiar el store mientras lo tenemos prestado), pero es **barata** (O(n)) y robusta a refactors que rompan la inducción. Visualiza el `commit` como la *segunda cerradura*: aunque un refactor futuro rompiera la re-validación por etapas del `stage`, el commit sigue siendo la única puerta responsable de la decisión todo-o-nada. Es por esto que el test de la «op 3 de 5» siembra el buffer a mano (el test vive dentro del módulo) y espera el error **en commit**, no en stage: ejercita manualmente esa segunda cerradura.

### La validación como replay sobre una `Simulacion`

El corazón de la atomicidad naive es `validar_buffer`: un **replay** del buffer sobre una vista simulada del store, respetando el **orden**.

```text
Simulacion: nodos_creados | nodos_borrados | aristas_creadas (con extremos) | aristas_borradas
validar_buffer(store, buffer) → por cada op_k EN ORDEN:
  valida contra (store real ∪ efecto de op_1..op_{k-1})
  ├─ arista a nodo creado en el MISMO buffer: válida SI los nodos van ANTES
  ├─ delete_node arrastra aristas del store (out ∪ in) Y del buffer
  └─ si algo falla → OperacionInvalida{indice,causa}; no se aplica NADA
```

Tres decisiones dignas de nota:

- **El orden del buffer es parte del contrato** (como el orden de un log). El test `edge_a_nodo_creado_en_la_misma_tx_es_valido` demuestra la mitad buena — arista a dos nodos del *mismo* buffer si los nodos van antes — y `el_orden_importa_edge_antes_de_sus_nodos_es_invalido` la estricta: la arista antes que sus nodos se rechaza, porque en la vista simulada los extremos aún no «existen».
- **El `delete_node` arrastra aristas del store Y del buffer** (el test `edge_arrastrada_por_cascada_de_nodo_del_buffer` lo prueba). La cascada del cap. 8 no puede quebrarse a medio buffer: si creaste una arista en la tx y luego borras uno de sus nodos, esa arista del buffer muere también. Es la invariante del cap. 8, vista a través del tiempo.
- **Coste O(n·(n+E))** por `stage` — una elección **naive y documentada**, a favor de la claridad. El WAL del cap. 28 validará incrementalmente; aquí manda la legibilidad.

El test `error_en_la_operacion_3_de_5_no_aplica_nada` es la prueba de fuego de la A naive: buffer «a mano» con la op 3 inválida (edge 0→7 sin nodo 7), commit → `OperacionInvalida{indice:2, causa:InvalidEdgeEndpoints{0,7}}`, y el store **queda intacto** — `node_count()==0`. Atomicidad naive: funciona (frente a la validación).

### El apply y el error honesto a mitad

```rust
pub fn commit(self) -> Result<ResumenCommit, TransaccionError> {
    validar_buffer(self.store, &self.buffer)?;           // 2ª cerradura
    let mut resumen = ResumenCommit::default();
    for (indice, op) in self.buffer.iter().enumerate() {
        let ya = resumen.total_operaciones();
        match op {
            Operacion::PutNode(n) => match self.store.put_node(n.clone()) {
                Ok(()) => resumen.nodos_escritos += 1,
                Err(causa) => return Err(TransaccionError::ApplyFallido {
                    indice, aplicadas: ya, causa,          // ← honesto sobre "cuánto" }),
            },
            // … PutEdge / DeleteNode / DeleteEdge análogos …
        }
    }
    Ok(resumen)
}
```

Distinguir **dos errores que la naïveté del problema confunde** es la clave:

- `OperacionInvalida` — un problema de **validación**, que se descubre en `stage` (y se re-descubre en `commit`). La transacción **no se aplicó**, el prefijo válido sobrevive. Nada que deshacer.
- `ApplyFallido` — un problema del **apply real**: el store dijo «no» a lo que la `Simulacion` aprobó, o el proceso murió a mitad. Aquí `aplicadas` te dice **cuántas operaciones ya están escritas**. Y el store **quedó a medias** — sin log no hay vuelta atrás.

`aplicadas` existe precisamente para que sepas si tienes que investigar el store o no: esa información se perdería si guardaras un único `Error` con un booleano.

### El rollback barato vs el rollback imposible

```rust
/// ROLLBACK: descarta el buffer. El store no se ha tocado NUNCA…,
/// así que el descarte es limpio por construcción.
///
/// Ésa es la lección del staging: deshacer es trivial ANTES de aplicar.
/// Deshacer DESPUÉS de aplicar (un rollback de verdad, a mitad de
/// escrituras) exigiría un log — cap. 28.
pub fn rollback(self) -> ResumenRollback { … }
```

Esta es **la frontera exacta del capítulo**. Descartar el buffer — rollback *antes* de aplicar — es gratis por la propia estructura: nada se aplicó. Deshacer *después* de aplicar, a mitad de escrituras, **exige un log** — es la línea que el cap. 28 cruza. El test `rollback_no_aplica_nada` lo demuestra: la tx acumuló dos operaciones (un put y un delete), hace rollback, y el store queda **exactamente** como estaba.

### `autocommit` como función ejecutable

```rust
pub fn autocommit(store: &mut dyn GraphStore, operacion: Operacion)
    -> Result<ResumenCommit, TransaccionError>
{ let mut tx = Transaccion::begin(store); tx.stage(operacion)?; tx.commit() }
```

`autocommit` no es código nuevo: es el modo por defecto de los caps. 7-26 hecho **visible y ejecutable**. `autocommit_equivalente_a_la_operacion_directa` demuestra la equivalencia exacta con `store.put_node(n)`; `autocommit_operacion_invalida_no_toca_el_store` que una operación inválida ni toca el store. Ya no tienes que *creer* que `put_node(n)` es una transacción de una sola operación: lo puedes ver.

## 27.8 Prueba de fuego

No basta con que el código compile: la tesis — *«ninguna letra está completa, y los límites son reales»* — debe poder **fallar** si miente. La prueba de fuego son cuatro tests:

**TEST-TESIS A — `informe_acid_es_honesto_sobre_el_estado_actual`.** A=C=I=`Parcial`, D=`Ninguna`; ninguna `Completa`; los strings «apply», «estructurales», «concurrencia» y «RAM» aparecen — la prosa coincide con el código ejecutable.

**TEST-TESIS B — `error_en_la_operacion_3_de_5_no_aplica_nada`.** La atomicidad naive FUNCIONA frente a la validación: buffer con la op 3 inválida, commit → `OperacionInvalida{indice:2, causa:InvalidEdgeEndpoints}`, store intacto.

**TEST-TESIS C — `apply_fallido_deja_el_store_a_medias_gancho_al_cap_28`.** La atomicidad naive NO cubre el fallo del apply: `StoreQueFalla` (un decorador que falla en la 3ª escritura) hace que `commit()` devuelva `ApplyFallido{indice:2, aplicadas:2, causa:UnknownNode(usize::MAX)}` y el store tenga **2 nodos**. A medias, sin log. Es una **regresión inversa**: cuando llegue el WAL en el cap. 28, este test se *invertirá*.

**TEST-TESIS D — `panic_a_mitad_de_apply_deja_el_store_a_medias`.** El «corte de luz» simulado: `StoreQueFalla` con `con_panic=true`, `catch_unwind` atrapa el pánico, y `node_count()==1`. La primera escritura llegó; las dos siguientes no. Sin WAL **no hay forma de saber** si ese nodo pertenecía a una transacción confirmada o a una que murió a medias.

Otros tests citados, por si quieres seguir la verificación mientras lees: `informe_acid_tiene_las_cuatro_letras_en_orden`, `garantia_acid_letra_nombre_definicion`, `informe_acid_display_muestra_niveles_y_caps`, `anomalias_de_aislamiento_definidas`, `commit_aplica_todo_el_buffer`, `commit_vacio_es_noop_valido`, `stage_rechaza_edge_a_nodo_inexistente`, `delete_de_nodo_creado_en_la_misma_tx`, `delete_node_inexistente_rechazado`, `delete_edge_tras_cascada_de_delete_node_rechazado`, `recrear_nodo_tras_borrarlo_en_la_misma_tx`, `errores_display_y_std_error`, `resumenes_display`, `operacion_display`, `operaciones_vista_del_buffer`.

**Síntoma si te saltas el capítulo**: tus lotes «de 10 nodos» quedan a medias cuando uno falla, no distingues un apply-fallido de una validación, crees que `commit` es durable (y es RAM), y llegas al cap. 28 sin vocabulario para pedirle al WAL lo que necesitas.

## 27.9 Qué hemos sacrificado

1. **La D — durabilidad — es cero.** `commit()` solo muta RAM. El camino a disco existe (`Pager::sync` del cap. 12, `BufferPool::flush` del cap. 13), pero lo que falta es el **protocolo** write-ahead — y confundir piezas con protocolo sería el peor modo de fallo de una BD: un «tenemos durabilidad» que un `kill -9` desmiente.
2. **La A es naive.** «O todo o nada» vale frente a la **validación**; frente a un fallo del apply real a mitad, el store queda a medias (`ApplyFallido`). El staging no puede y *no pretende* arreglar eso.
3. **Aislamiento sin motores de locks** (porque no hace falta): la I la da el borrow checker en single-thread. Con concurrencia real (cap. 30), se revisa.
4. **Validación O(n·(n+E)) *por stage***, naive y documentada; un WAL valida incrementalmente. La C es «trivial»: solo invariantes estructurales, sin restricciones declarativas — en una BD real es un contrato *compartido* con la aplicación.

## 27.10 Cómo lo hace una BBDD real

Todo lo que aquí es «parcial», una base de datos real lo lleva el resto del camino — el de los tres capítulos que vienen:

- **El WAL (cap. 28)** es el bloc con copia de cada trazo *antes* de tocar el libro mayor. El estándar de hecho se llama **ARIES** (Mohan et al., «ARIES», ACM TODS 17(1), 1992). La `Operacion` de este capítulo es **exactamente** lo que el WAL serializa, bajo el framing del cap. 10.
- **La recuperación (cap. 29)** responde a la pregunta del test D — conservar o deshacer al arrancar. Härder & Reuter (1983, el paper que acuñó ACID) la formalizaron; ARIES la convirtió en replay con undo y redo.
- **MVCC y 2PL (cap. 30)** convierten la I «parcial por diseño» en aislamiento real bajo concurrencia. Berenson et al. («A Critique of ANSI SQL Isolation Levels», SIGMOD Record 24(2), 1995) mostraron que las definiciones ANSI eran incompletas — por eso hoy se usa el vocabulario de anomalías más rico (lectura sucia, lost update, no repetible, fantasma) que este capítulo abrió.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `apply` válida el buffer entero en `commit` si `stage` ya validó cada operación al acumularla?
- *Intermedio*: dibuja los dos cortes de la línea de tiempo del apply que los tests C y D representan; ¿en qué se diferencian el error tipado `ApplyFallido` y el pánico del «corte de luz»?
- *Experto*: ¿qué *es* exactamente «a medias»? Define el estado del store tras cada fallo, y justifica por qué ambos agujeros son irresolubles sin un log de lo ya aplicado.

## 27.11 Lo que te llevas

- **ACID no es un interruptor**: son cuatro promesas independientes, cada una con nivel (`Ninguna/Parcial/Completa`). `informe_acid()` es un artefacto **ejecutable** que los tests verifican — la documentación no puede prometer más que el código.
- **La transacción es un objeto con ciclo de vida**: `begin → stage* → commit|rollback`. El staging acumula `Operacion` en un buffer; el commit valida todo y aplica; el rollback descarta.
- **El ciclo de vida vive en los tipos**: `commit`/`rollback` consumen `self` — usar una tx cerrada o anidar dos no compila. Droppear una tx activa es rollback implícito seguro.
- **El borrow checker es el cerrojo**: el «un único escritor» del brief lo ejecuta `&mut`, gratis, sin locking.
- **El orden del buffer importa** (como el de un log): una arista a nodos del mismo buffer es válida si los nodos vienen antes; `delete_node` arrastra aristas del store y del buffer.
- **Rollback barato ≠ rollback imposible**: deshacer antes de aplicar es descartar el buffer; deshacer después exige un log. Esa frontera es el cap. 28.
- **Estado real hoy**: A parcial, C parcial y trivial, I parcial por diseño, D ninguna. Decirlo con honestidad es parte de la interfaz.

## 27.12 Ojo, cuidado con…

- **«Tengo ACID porque hice commit».** El commit no enciende nada: cada letra tiene su nivel, y `informe_acid()` es la pieza honesta. Si dices «tenemos transacciones», pregunta *cuál* letra y *hasta dónde*.
- **Confundir validación con apply.** `OperacionInvalida` se rechaza en `stage` (la tx sigue viva con su prefijo válido); `ApplyFallido` se descubre en `commit` (el store quedó a medias). Son errores DISTINTOS con consecuencias DISTINTAS — trata cada uno como lo que es, no como «la tx falló».
- **Esperar rollback completo.** El rollback de este capítulo es barato por construcción, *antes* de aplicar. El rollback real (después de aplicar) exige un log — no diseñes sistemas que asumen «rollback siempre funciona» sobre apply parcial.
- **Confundir `commit` con durable.** `commit()` muta RAM. El corte de luz borra lo confirmado y no hay WAL que lo recite. La D no se adivina: se construye (cap. 28).
- **Pensar que el aislamiento exige locks.** Con un solo hilo, el `&mut` *es* el cerrojo. No introduzcas un `Mutex` «por si acaso» — cuando llegue la concurrencia (cap. 30) se decidirá con criterio.

*Precisión de lenguaje*: *transacción* (objeto con ciclo de vida) vs *operación* (`Operacion`, la unidad del buffer); *staging* (acumular en privado) vs *apply* (escribir al store); *commit* (firma del bloc) vs *rollback* (tirar el bloc); *buffer* (estado privado de la tx) vs *store* (libro mayor compartido); *validación* vs *apply*; *autocommit* (tx de una op) vs *tx explícita*; *informe ACID* (artefacto tipado) vs *ACID* (el acrónimo); *anomalía* (patrón de fallo) vs *garantía* (lo que la BD promete); *nivel* (Ninguna/Parcial/Completa) vs *letra* (A/C/I/D).

## 27.13 Pin de batalla

> *«Un "commit" que el código no puede distinguir de un apply cortado no es una promesa — es una esperanza. La durabilidad no se declara: se protocoliza.»*

## 27.14 Si solo lees 30 segundos

ACID no es un interruptor: son cuatro promesas independientes con nivel. `informe_acid()` (ejecutable, auditado por tests) dice la verdad: **A parcial, C parcial y trivial, I parcial por diseño, D ninguna**. `Transaccion::begin(&mut store)` toma el préstamo exclusivo del store; el staging acumula `Operacion` en un buffer sin tocar el store; `commit` revalida todo y aplica; `rollback` descarta (gratis, porque nada se aplicó). `commit`/`rollback` consumen `self`, así que el ciclo de vida vive en los tipos: usar una tx cerrada o anidar dos no compila. El borrow checker es el cerrojo del «único escritor». El orden del buffer importa (arista después de sus nodos). Y lo honesto: si el apply falla a mitad, `ApplyFallido{aplicadas}` te dice cuánto llegó — pero sin log no hay vuelta atrás. Ese es el gancho del cap. 28 (WAL).

## 27.15 Una historia pequeña

Ana llevaba tres días «arreglando» un bug que no era un bug. Su analítica del cap. 26 materializaba una proyección y, de vez en cuando, el grafo quedaba con nodos huérfanos — aristas que apuntaban a ids que no existían. Descubrió que el problema no estaba en su código de análisis: estaba en *cómo escribía*. Un script rellenaba un batch de vértices con `store.put_node(...)` en bucle, y cuando el vértice 5 de 10 era un duplicado, el script seguía — el grafo se quedaba con los cuatro primeros, sin la arista que los unía, sin que nadie lo hubiera pedido. LiraDB no «borraba» nada: nunca había tenido la noción de *unidad*. Tras este capítulo, su script abría una `Transaccion`, metía los diez vértices y *la arista* en el buffer, y hacía `commit`. Si el quinto era inválido, la transacción se descartaba entera — o, mejor, `stage` se lo decía en el acto sin tocar el store. La moraleja, gritada en la bitácora: *no era un bug de datos; era una ausencia de acuerdo sobre qué es «un lote».*

## Ejercicios resueltos

**1. El fallo en la operación 3 de 5.** Buffer `PutNode(0,"P"), PutNode(1,"P"), PutEdge(0,0,1,"KNOWS"), …`. ¿Por qué el commit *no* falla aquí aunque valide el buffer entero? Porque la validación es un replay **sobre la `Simulacion`**: cuando se valida la arista, los nodos 0 y 1 ya fueron creados por las operaciones anteriores del buffer. Los extremos *existen en la vista simulada*. Es exactamente el caso `edge_a_nodo_creado_en_la_misma_tx_es_valido`. En cambio, si la arista fuera **antes** que sus nodos, los extremos aún no están en la simulación → `OperacionInvalida{indice:0, causa:InvalidEdgeEndpoints}` — `el_orden_importa_edge_antes_de_sus_nodos_es_invalido`. El orden del buffer no es cosmética: es parte del contrato (como el orden de un log).

**2. ¿Por qué el rollback del cap. 27 es gratis y el rollback «real» sería imposible sin log?** Porque nada se aplica fuera del commit. El buffer es privado y el store no se toca hasta la re-validación y el apply del `commit`. `rollback()` descarta el `Vec<Operacion>` — y como el store quedó intacto, no hay nada que deshacer (test `rollback_no_aplica_nada`). Deshacer *después* de aplicar es otro problema: si el apply ya escribió 2 de 5 operaciones y quieres revertirlas, necesitas saber *qué* se escribió y en qué orden — eso es un log. El `Log::append` del append-only cap. 10 es la pieza que falta: la única forma de «deshacer» es tener registrado lo que se hizo. Por eso `rollback()` pone la linde en «antes de aplicar»: más allá, la tarea ya es del WAL (cap. 28).

## Ejercicios propuestos

**Esencial (recordar/aplicar — predicción, no ejecución).** Sobre un store VACÍO, predice SIN ejecutar el resultado de commitear este buffer: `PutNode(0,"A"), PutEdge(0, 0, 1, "KNOWS"), PutNode(1,"B")`. Responde: (a) ¿tiene éxito el commit?; (b) si no, ¿qué variante de `TransaccionError`, con qué `indice` y qué `causa`; (c) el estado del store después. *Pistas*: (1) ¿existe el nodo 1 cuando se valida la arista (en el orden del buffer)?; (2) ¿el orden del buffer es libre?; (3) ¿la operación 3ª (que sí es válida por sí sola) llega a aplicarse si la 2ª falla? *Verificación*: `el_orden_importa_edge_antes_de_sus_nodos_es_invalido` (el caso que falla) y `edge_a_nodo_creado_en_la_misma_tx_es_valido` (cómo se corrige moviendo la arista al final). *Criterio*: predicción exacta + verificación corriendo ambos tests del workspace (`cargo test -p vol2-liradb --lib cap27`).

**Intermedio (analizar — mezcla caps. 8 y 10).** El comentario del módulo llama a la `Operacion` del cap. 27 «la heredera del `RecordKind` del cap. 10» y «la semilla del WAL del cap. 28». Razona (a) qué se almacenaba en `RecordKind` (cap. 10) con la **misma forma** que `Operacion`, y por qué esa forma compartida permite que el cap. 28 serialice el buffer al WAL **sin reinterpretar**; (b) por qué `delete_node` arrastra aristas del store **Y del buffer** (test `edge_arrastrada_por_cascada_de_nodo_del_buffer`) — ¿qué invariante del cap. 8 lo hace obligatorio, y qué le pasaría a la simulación si no lo hiciera?; (c) por qué el rollback del cap. 27 es barato (descartar `Vec<Operacion>`) pero el rollback real *después* de aplicar sería imposible sin un log — y por qué `Log::append` (cap. 10) es exactamente el «antes» que el WAL del cap. 28 codificará. *Pistas*: (1) ¿qué variantes pintaba `RecordKind` y qué hacen `stage`/`apply`?; (2) ¿qué dice `StoreError` sobre la cascada al borrar?; (3) ¿dónde encaja `Log::append` respecto al `commit`? *Verificación*: `delete_edge_tras_cascada_de_delete_node_rechazado` y la sección §32 de `MIGRATION-PATTERN.md`. *Criterio*: razonar la conexión entre tres capítulos **sin mirar el código**.

**Experto (crear — retrieval puro).** Parte 1, de memoria y sin pistas en el enunciado: reconstruye el `InformeAcid` COMPLETO del cap. 27 — las cuatro letras, su nivel (`Ninguna/Parcial`), su justificación honesta (qué garantiza hoy y qué no), y el capítulo que cierra cada brecha. Parte 2: escribe el test `nodo_recreado_despues_de_cascada_no_revive_sus_aristas` sobre el grafo 0→1 (edge 0) →2 (edge 1): tx `delete_node(1), put_node(1, "Renacido")`; `commit` verde; verifica `node_count()==2`, `edge_count()==0` y que las aristas muertas NO vuelven al recrear el nodo. *Pistas*: (1) ¿el `delete_node` arrastra las aristas adyacentes en la validación?; (2) ¿qué ve la `Simulacion` cuando vuelve a aparecer el id 1 — sus viejas aristas siguen en `aristas_borradas`?; (3) ¿cómo verifica el test que re-crear el nodo no re-crea la historia? *Verificación*: `recrear_nodo_tras_borrarlo_en_la_misma_tx` (base) + tu extensión propia. *Criterio*: informe exacto de memoria + test verde + la razón de por qué las aristas no se reviven.

## Para profundizar

- **Jim Gray, «The Transaction Concept: Virtues and Limitations» (VLDB 1981, pp. 144-154)** — el paper que abrió este capítulo: la transacción como transformación atómica, consistente y durable, con la anécdota de las reservas de vuelo y el agente de viajes. Fuente primaria de la anécdota de la esquina.
- **T. Härder & A. Reuter, «Principles of Transaction-Oriented Database Recovery» (ACM Computing Surveys 15(4), 1983, pp. 287-317; DOI 10.1145/289.291)** — el paper que **acuñó el acrónimo ACID**. Formalización de los conceptos que los caps. 28-29 heredan.
- **C. Mohan et al., «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging» (ACM TODS 17(1), 1992, pp. 94-162; DOI 10.1145/128765)** — el estándar moderno de WAL + undo/redo, el destino del cap. 28 (y su recuperación en el 29).
- **H. Berenson et al., «A Critique of ANSI SQL Isolation Levels» (SIGMOD Record 24(2), 1995, pp. 1-10; DOI 10.1145/223784.223785)** — por qué el vocabulario de anomalías (lectura sucia, lost update y compañía) que aquí abrimos es el que los niveles de aislamiento reales usan; base del cap. 30.
- **Gray & Reuter, «Transaction Processing: Concepts and Techniques» (Morgan Kaufmann, 1993)** — la referencia canónica de ACID, recovery y aislamiento de la que estos capítulos son un recuerdo en miniatura.
- **SQLite — documento oficial «Atomic Commit In SQLite»** — cómo un motor real, en producción, implementa atomicidad en una base de un solo fichero con un WAL; y **PostgreSQL docs, cap. 13 «Concurrency Control»** — aislamiento y niveles en el mundo real.
- Los **comentarios del módulo** `cap27_transacciones.rs` funcionan como prosa verificable: cada límite que anuncian tiene un test encima.

## Mini-diálogo: en guardia nocturna

> — O sea, que «commit» no me hace durable. ¿Entonces para qué me sirve?
>
> — Para lo que sí promete. El commit es la diferencia entre «metí una arista colgada que nadie pidió» y «estas cinco operaciones entran juntas o ninguna». La A *frente a la validación* ya es tuya.
>
> — Pero dices que la D es *ninguna*. Menuda venta.
>
> — Lo honesto. Decir «durable» con un corte de luz que lo borra sería el peor error posible de una base de datos — peor que admitir que falta. El informe te dice exactamente cuánto tienes y quién lo completa. El cap. 28 construye el WAL que cierra la D. Y luego el 30, el aislamiento de verdad.
>
> — ¿Y si me da miedo el borrow checker? Pensé que necesitaba un motor de locks.
>
> — Ahí está la gracia. Tu cerrojo ya está puesto: es `&mut`. Mientras viva la transacción, ni un lector ni otro escritor tocan el store, y te lo verifica el compilador en vez de un runtime. El día que llegue la concurrencia, revisamos.

---

*(Próximo capítulo: 28 — Write-ahead log. Has visto el hueco: cuando el apply se corta a mitad, `node_count()==2` y «nadie recuerda» qué faltaba. El WAL es el bloc con copia de cada trazo — se escribe ANTES de tocar el store, sobrevive al crash porque `fsync` ya forma parte del protocolo, y su registro serializa exactamente la `Operacion` del cap. 27. Cuando llegue, los dos tests que aquí AFIRMAN el store a medias se invertirán: el log sí recuerda.)*
# Capítulo 28 — Write-Ahead Log

> *«El log se escribe antes que el dato. No como orden de paso: como promesa sobre la dirección de la recuperación.»*

## 28.0 La anécdota de la esquina

En 1992, un equipo de IBM en Almaden —C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh y P. Schwarz— publicó un paper largas veces citado y pocas veces leído entero: «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging» (ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162). El paper cierra una pelea que llevaba quince años coleando en la industria: cuándo, dónde y cómo se le promete a un usuario que su `COMMIT` sobrevivirá a un corte de luz. Y lo hace con una frase que define un capítulo entero: «write-ahead logging — the log records for a change must be written to stable storage before the change itself is written to the database».

Que el log se escriba ANTES que la página de datos suena a orden de pasos. No lo es. Es una promesa sobre la dirección de la recuperación. Si la promesa es «el log antes que el dato», entonces cuando el sistema despierta tras el fallo, el camino es RE-APLICAR lo que el log dijo que se confirmó (roll-forward / redo). Si la promesa es al revés —el commit se escribe al final del apply y los datos pueden estar ya en disco mientras el log sigue en RAM por un instante—, entonces la mitad desconocida del problema («¿qué sobrevivió?») NO se resuelve releyendo: se resuelve DESHACIENDO lo que se aplicó de más. Y deshacer requiere imágenes ANTES, lo que dobla el log y la maquinaria. Esa segunda mitad se llama UNDO; el capítulo que la construye es el 29. ARIES propuso las dos mitades (Analysis-Redo-Undo) y la decisión de CÓMO se ordenan. Aquí elegimos la mitad más simple, la que enseñamos primero y la que cierra la Parte VI del Vol.II en su primer tramo: sólo REDO, y el redo existe porque el log se escribió ANTES.

La otra raíz, anterior, está en System R: Jim Gray documentó la «bitácora» en «Notes on Database Operating Systems» (IBM Research Report RJ 2188, 1978) y la convirtió en columna vertebral de la recuperación. Lo que el lector de hoy llama WAL es la versión 1992 de la bitácora de 1978 — con LSN monótono, con CRC, con el «log antes que el dato» como axioma. Y lo que hoy les voy a pedir que entiendan no es un detalle: es la decisión de la que viven los demás capítulos de la Parte VI.

## 28.1 Objetivo

Al terminar este capítulo sabrás **por qué** la transacción del cap. 27 (con su `commit` re-validando y aplicando) se rompía frente a un fallo del apply, y **cómo** un write-ahead log convierte ese «se quedó a medias y nadie recuerda qué faltaba» en «un log SÍ recuerda y el replay lo completa». Cinco piezas en `cap28_wal.rs`:

1. **El registro del WAL** (`WalRecord`) — la pareja `(lsn, tx_id)` más `CuerpoWal = Begin | Operacion(op) | Commit | Rollback`, con el framing del cap. 10 (`length-prefix + CRC32`) y la `Operacion` del cap. 27.
2. **El protocolo write-ahead** (`WalTransaccion::commit`) — el commit en dos fases: el log se escribe y sella CON UN `Commit` ANTES de tocar el store. Esa es la decisión del capítulo.
3. **El redo idempotente** (`aplicar_para_redo` + `replay_wal`) — la regla por la cual re-aplicar lo ya aplicado es un no-op, y por la que el replay no necesita saber qué sobrevivió al fallo.
4. **La política de flush** (`PoliticaFlush`) — `CadaEscritura` (la regla de oro literal) vs `SoloCommit` (un sync por transacción; semilla del group commit del cap. 30).
5. **El truncado con contrato** (`truncar_hasta_lsn`) — poda del log BAJO FIRMA del llamador; los LSNs no se reinician.

Y el hito que demuestra que el capítulo cierra el agujero del cap. 27: el `StoreQueFalla` con cuatro ops y fallo en la tercera, escenario IDÉNTICO al de aquél, termina con `node_count()==4` tras el replay.

## 28.2 Problema

El cap. 27 dejó una deuda escrita en el `Display` de su error:

> *«fallo durante el APPLY de la operación #2 (…): 2 operaciones ya estaban aplicadas y sin log no se pueden deshacer — el store quedó a medias (cap. 28: WAL)»*

Esa frase no es retórica: es el agujero que vino a cerrar este capítulo. Recordemos los dos escenarios que el `StoreQueFalla` del cap. 27 dejó como tests vivos:

1. **Error del store a mitad del apply.** El buffer validó, las dos primeras escrituras pasan, la tercera falla. El `commit` devuelve `ApplyFallido{indice: 2, aplicadas: 2}` y el store queda con `node_count()==2`. La transacción «no ocurrió» PORQUE SU DECLARACIÓN está sólo en RAM; el siguiente open del proceso es un open con dos nodos y la mitad de la verdad perdida.
2. **Pánico a mitad de apply.** El store «muere» eléc­tri­ca­mente (un `panic!("corte de luz simulado")` en la segunda escritura). `catch_unwind` rescata el hilo. El store quedó con un nodo. La memoria del proceso se evaporó; nadie sabe qué faltaba.

En ambos casos, la transacción «no se completó» pero su ESFUERZO no se puede revertir limpiamente. El cap. 27 ya había decidido que sólo se aplica «o todo o nada» AL NÍVEL DEL STAGING (la validación de `stage()` expulsa la op mala y la tx SIGUE VIVA). Pero el apply real es otra cosa: cuando `put_node` se ejecuta de verdad contra el store, ya no hay rollback posible sin UNDO — y UNDO no existe en el motor.

Y al lado de la atomicidad (A), la durabilidad (D) del cap. 27 era **Ninguna** — el `commit` vivía en RAM. Un close del proceso se llevaba la transacción confirmada, las páginas de datos, todo. La entrada D del `informe_acid()` del cap. 27 decía, textual, «en RAM, no durable (cap. 28)».

La pregunta del capítulo: ¿cuál es la pieza que CONECTA las dos cosas — la atomicidad frente al apply real y la durabilidad del commit — sin pedir UNDO?

## 28.3 Modelo mental

Piensa en una **notaría con libro de entrada**.

- Toda escritura del grafo que promete durabilidad PASA primero por la notaría. El notario abre un asiento nuevo, anota la operación y la «sella» con un número correlativo (LSN; entry `1`, `2`, `3`…). Tan sólo después de que la anotación está sellada y firmada contra el libro (el `sync`) se le da al cliente la copia que modifica el fichero físico de la notaría (apply al store).
- Si en el momento de la firma el cliente sufre un infarto (el `StoreQueFalla` del cap. 27), la notación YA está en el libro; al día siguiente, nuevo notario, **relee el libro y completa la operación** (roll-forward). Las dos primeras escrituras del nodo ya estaban; la tercera se aplica al releer. La idempotencia del redactor hace que las escrituras duplicadas no dupliquen.
- Si la firma no llegó (commit truncado por un corte de luz), el asiento existe pero el cliente no tiene sello: la operación «como si nunca hubiera ocurrido». El replay del día siguiente la descarta limpia.
- Si alguien arrancó una página del libro (un CRC roto) o arrancó un folio de en medio (un hueco de LSN), el libro entero deja de ser confiable — la notaría no se inventa los huecos. El iterador para limpio en la cola rota; el modo estricto grita el LSN del registro dañado.

```
            commit en dos fases (commit-marker-ANTES-del-apply)
  ┌──────────────────────────────┐   ┌──────────────────────────────┐
  │  RE-VALIDAR                  │   │  ALTERNATIVA RECHAZADA       │
  │  buffer con validar_buffer   │   │  marker al final del apply   │
  │  (cap. 27, pub(crate))       │   │  → apply a medias sin commit │
  ├──────────────────────────────┤   │  → rescate exige UNDO        │
  │  log_write(op) por cada op   │   │  → ARIES, cap. 29            │
  │  (sync según PolíticaFlush)  │   └──────────────────────────────┘
  ├──────────────────────────────┤
  │  log_write(Commit) + sync    │   ← EL PUNTO DE DURABILIDAD
  ├──────────────────────────────┤
  │  apply al store              │   ← puede fallar a medias
  │  (si falla, replay_wal       │
  │   rescue con roll-forward)   │
  └──────────────────────────────┘
```

Y un detalle no obvio: el **LSN** es el sello notarial. Una vez asignado, no se reutiliza — ni aunque arranque una página nueva del libro. Por eso un sello de 2007 (`lsn=42`) puede seguir viviendo en el libro aunque las primeras 40 páginas ya se hayan borrado: la identidad de una anotación es su sello, no su posición en bytes. La identidad del redo «el nodo 7 que escribí en `lsn=42`» es estable a través del truncado.

El momento ¡ajá!: «"el log antes que el dato" no es una orden de paso. Es una promesa sobre la dirección de la recuperación. Si el log se escribe ANTES, la mitad DESCONOCIDA del problema (qué sobrevivió al fallo) se resuelve con un re-leer. Si se escribe DESPUÉS, la mitad desconocida exige deshacer — y deshacer necesita imágenes ANTES, lo que dobla el log. La pregunta "¿UNDO o no?" se contesta ANTES de escribir la primera línea de código».

## 28.4 Primera solución

La solución ingenua ya la tienes y funciona: la del cap. 27. `Transaccion::commit` re-valida el buffer y aplica. Si el apply falla a medias, `ApplyFallido` y fin. La transacción era ACID en todo MENOS en D (durabilidad en RAM) y A frente al apply real (no frente a validación). El ERROR del cap. 27 es exactamente la motivación del cap. 28.

Una segunda solución ingenua es tan tentadora como la primera: «pues hago que el apply sea atómico mágico» — operaciones tan pequeñas que nunca produzcan fallo a medias. No funciona: cualquier cambio a una página de 4.096 bytes es o SÍ o NO; no hay «a medias significativas». Y aunque funcionara, no resuelve la durabilidad: si el proceso muere justo después del apply, la operación se fue a RAM y se perdió.

## 28.5 Sus límites

Tres límites llevan del cap. 27 al cap. 28:

1. **El apply real falla a medias y NO hay UNDO.** El `StoreQueFalla` del cap. 27 hace esto literalmente: falla en la N-ésima escritura y el store queda inconsistente. Sin UNDO no hay forma de sacar las dos operaciones que sí pasaron. Y UNDO exige imágenes ANTES, con la mitad de la complejidad y el doble del log.
2. **El commit vive en RAM.** El `ResumenCommit` del cap. 27 decía, textual, «en RAM, no durable (cap. 28)». Un cierre del proceso se lleva la tx confirmada. La entrada D del `informe_acid()` era `NivelGarantia::Ninguna`.
3. **El siguiente proceso que abre la BD no recuerda nada.** El store en RAM arranca de cero. La `Operacion` se construyó como dato precisamente para que algún día se pudiera serializar — pero ningún capítulo la había serializado todavía.

El patrón común: **sin log no hay vuelta atrás**. El nombre técnico del agujero es «atomicidad frente al apply real», pero el síntoma — store a medias sin memoria — es lo que duele.

## 28.6 Solución evolucionada: el log antes que el dato

Una sola decisión de diseño cambia tres cosas a la vez. La decisión es del capítulo y es la siguiente:

> **El registro `Commit` se escribe al log y se sella con un `sync` ANTES de que el apply toque el store.**

A partir de esa decisión, todo lo demás se deduce:

- El log crece con un orden FIJO: Begin, operaciones (cada una con su LSN), Commit. Cuando el apply arranca, TODO el intento ya es durable. Si el apply falla a medias, NO es un escenario perdido: el log contiene el Begin, las operaciones y el Commit. Un `replay_wal` las re-aplica (idempotente) y la transacción se completa.
- El LSN es la dirección física del log y la identidad del redo. Monótono, consecutivo, nunca reutilizado. La duración del log y la posición de los registros son ortogonales: truncar mueve bytes, no toca LSNs.
- El sólo hecho de escribir el log antes que el dato HACE que UNDO sea innecesario: el apply a medias es una operación INCOMPLETA, no una operación APLICADA. La idempotencia del redo se encarga del resto.

La otra decisión — complementaria, no rival — es la política de flush. La regla de oro es simple: tras cada `log_write`, un `sync`. Es la opción por defecto del capítulo (`CadaEscritura`). Pero si la semilla del group commit (cap. 30) ya obligó a la observación de que las páginas de datos no se llevan a disco antes del commit, entonces un `sync` POR TRANSACCIÓN es correcto: justo antes del apply ya se sabe que las páginas de datos están en RAM y no pueden quedar pisadas por un fsync previo. Esa es la política `SoloCommit`, un sync por transacción, semilla del cap. 30.

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap28_wal.rs`. Vamos a leerlo por partes, porque cada tipo y cada `commit` tiene un porqué.

### El formato del registro

```rust
pub type Lsn = u64;
pub type TxId = u64;

pub enum CuerpoWal {
    Begin,
    Operacion(Operacion),   // MISMA del cap. 27
    Commit,
    Rollback,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub tx_id: TxId,
    pub cuerpo: CuerpoWal,
}
```

Cuatro piezas básicas, tres reutilizaciones deliberadas:

- **`Operacion(Operacion)`** es la misma del cap. 27 — y la misma forma que el `RecordKind` del cap. 10 anticipaba. La semilla del WAL estaba plantada dos capítulos antes.
- Los strings y `Value` se serializan con el encoding del cap. 9 (`encode_string`, `encode_value`, `decode_*`).
- El framing hereda el del cap. 10: `length-prefix u32` para saber dónde termina cada registro, **`CRC32` con `crc32_simple`** para detectar bytes modificados sin re-parsear.
- Lo único nuevo: `u64` LE para LSN y TxId, porque el LSN exige un orden monótono y los ids del cap. 7 no cabían en `u32`.

Un detalle cargadito de consecuencias: las props de un `Node` son un `HashMap`. La iteración de un `HashMap` **no es determinista** — dos llamadas pueden devolver orden distinto. Un log debe codificar SIEMPRE los mismos bytes para el mismo valor (porque el CRC se calcula sobre los bytes, no sobre la operación lógica), así que `encode_props` ordena las claves antes de serializar. Sin esta ordenación, el mismo grafo produciría dos logs distintos y los tests de roundtrip fallarían al azar.

### El framing byte a byte

```
[record_len: u32 LE][lsn: u64 LE][tx_id: u64 LE][tag: u8][payload...][crc32: u32 LE]
```

El orden de las validaciones al decodificar importa y es la cadena del cap. 11 + cap. 10:

1. **length-prefix** → sabemos si el registro está completo o truncado.
2. **CRC32** sobre `body` (todo menos el CRC). Si no cuadra, NO parseamos el cuerpo: un byte rotado no debe «interpretarse» como un valor legal.
3. **tag** → Begin/Operacion/Commit/Rollback.
4. **payload** → `Operacion` completa, codificada con la misma `encode_operacion` que el commit también produce.

El CRC se valida ANTES que el tag porque la corrupción es local: si el cuerpo está roto, no sabemos qué versión del payload es la «correcta». Esa cadena — length → CRC → parse — es la del cap. 11, y se hereda tal cual.

### El WAL: el «disco» del capítulo

```rust
pub struct Wal {
    bytes: Vec<u8>,         // concatenación de registros
    next_lsn: Lsn,          // 1, 2, 3, … — nunca reinicia
    next_tx_id: TxId,       // 1, 2, 3, … — nunca reinicia
    syncs: u64,             // contador de "fsync"
    politica: PoliticaFlush,
}
```

En RAM (igual que el `AppendOnlyLog` del cap. 10, su germen directo): un `Vec<u8>` de registros encadenados por length-prefix. Un WAL de fichero real sería un `File` con `O_APPEND` y `sync_all` — el mismo `FilePager::sync` del cap. 12; el PROTOCOLO (qué se escribe, cuándo, y quién se lee al despertar) es lo que este capítulo construye. La `fsync` real es la del cap. 12; aquí la durabilidad es un CONTADOR que cuenta las veces que el protocolo manda llevarla a cabo. Lo que los tests verifican es que se LLAME cuando el protocolo lo exige — y eso es la pieza AUDITABLE.

Tres invariantes que el capítulo sostiene y los tests verifican:

- Los LSN son monótonos y consecutivos desde el 1, y NUNCA se reutilizan — ni después de truncar.
- Los TxId también son monótonos desde el 1.
- `sync()` cuenta las «fsync»: en RAM no hay nada que sincronizar, pero la PROMESA es verificable y los tests la cuentan.

### El commit en dos fases

```rust
pub fn commit(self) -> Result<ResumenCommitWal, WalError> {
    // 1. Re-validar el buffer entero (punto de no retorno)
    validar_buffer(self.store, &self.buffer).map_err(WalError::Validacion)?;

    // 2. FASE LOG: cada operación al log ANTES que al store
    for op in &self.buffer {
        self.wal.log_write(self.tx_id, CuerpoWal::Operacion(op.clone()));
        if self.wal.politica() == PoliticaFlush::CadaEscritura {
            self.wal.sync();
        }
    }
    // 3. EL PUNTO DE DURABILIDAD: Commit + sync
    let lsn_commit = self.wal.log_write(self.tx_id, CuerpoWal::Commit);
    self.wal.sync();

    // 4. FASE APPLY: el store puede fallar aquí; el log ya tiene TODO
    for (indice, op) in self.buffer.iter().enumerate() {
        // … (put_node / put_edge / delete_node / delete_edge) …
    }
    Ok(resumen)
}
```

Cinco líneas que esconden las decisiones del capítulo:

- **Línea 2** (`re-validar`): el mismo `validar_buffer` del cap. 27 (que pasa a `pub(crate)` en este capítulo para que el handshake sea limpio). El «punto de no retorno» del cap. 27 se hereda; el `commit` re-valida el buffer entero porque entre el `stage()` y el `commit` nada cambió — el borrow checker lo garantiza.
- **Líneas 5-9** (FASE LOG): el write-AHEAD. Cada `Operacion` se serializa al log con su LSN. En `CadaEscritura`, cada `log_write` añade un `sync`. La política correcta es la del cap. 22: ruidosa, explícita, sin atajos silenciosos.
- **Línea 11** (`Commit + sync`): el punto de durabilidad. El LSN de este registro es devuelto al llamador (`lsn_commit`), y queda escrito en `ResumenCommitWal.lsn_commit`. A partir de esta línea, la transacción existe aunque el proceso muera al siguiente tick.
- **Líneas 14-24** (FASE APPLY): el store se toca. Puede fallar. Si falla, el llamador recibe `WalError::ApplyFallido{ indice, aplicadas, causa }` — la MISMA FORMA que el `TransaccionError::ApplyFallido` del cap. 27 — pero el `Display` cambia: «… pero el log YA contiene el commit: `replay_wal` COMPLETA la transacción (arranque automático: cap. 29)».

Ésta es la decisión del capítulo: **commit-marker-ANTES-del-apply**. La alternativa — escribir el `Commit` al final del apply — dejaría el apply a medias SIN commit y rescatarlo exigiría UNDO. UNDO es la mitad que ARIES (Mohan et al. 1992) construye completa. Aquí no la necesitamos: el log ya contiene todo lo que la tx quería hacer, y `replay_wal` la termina. El nombre técnico de la estrategia es **roll-forward** o **redo-only**.

### El redo idempotente

```rust
pub(crate) fn aplicar_para_redo(
    store: &mut dyn GraphStore,
    op: &Operacion,
) -> Result<(), StoreError> {
    match op {
        Operacion::PutNode(n) => match store.get_node(n.id) {
            Some(actual) if actual == n => Ok(()),       // idéntico = no-op
            Some(_) => {                                  // divergente = el log manda
                store.delete_node(n.id);
                store.put_node(n.clone())
            }
            None => store.put_node(n.clone()),
        },
        // … igual con aristas y deletes …
    }
}
```

Dos reglas y media:

- **Put idéntico al que ya está = no-op.** Por eso un replay sobre un store al que ya se aplicó la mitad NO duplica.
- **Put divergente del que ya está = overwrite.** El log es la verdad; si el registro dice «este nodo tiene estos labels» y el store tenía «este nodo tiene estos otros labels», gana el log.
- **Delete de lo ausente = no-op silencioso.** Re-aplicar un `DeleteNode` sobre un nodo que el apply a medias ya borró no es un error.

La idempotencia es lo que hace que el replay no necesite saber qué sobrevivió al fallo. Sin ella, el replay necesitaría un análisis del estado exacto al fallar (eso es la fase Analysis de ARIES, el cap. 29), y la «orden de re-aplicar lo que el log dice» dejaría de ser segura. La regla es: si una operación del log puede ejecutarse DOS VECES seguidas sin cambiar el resultado, la operación es REDO-segura. Las cuatro `Operacion` lo son.

### El replay en dos pasadas

```rust
pub fn replay_wal(store: &mut dyn GraphStore, wal: &Wal) -> Result<InformeReplay, WalError> {
    // Pasada 1: ¿quién llegó a Commit?
    let mut confirmadas: HashSet<TxId> = HashSet::new();
    let mut iniciadas: HashSet<TxId> = HashSet::new();
    for rec in wal.iter() {
        match rec.cuerpo {
            CuerpoWal::Begin => { iniciadas.insert(rec.tx_id); }
            CuerpoWal::Commit => { confirmadas.insert(rec.tx_id); }
            _ => {}
        }
    }

    // Pasada 2: redo de lo confirmado, en orden de LSN
    let mut operaciones = 0usize;
    for rec in wal.iter() {
        if let CuerpoWal::Operacion(op) = &rec.cuerpo
            && confirmadas.contains(&rec.tx_id)
        {
            aplicar_para_redo(store, op).map_err(|causa| WalError::RedoFallido {
                lsn: rec.lsn,
                causa,
            })?;
            operaciones += 1;
        }
    }

    Ok(InformeReplay {
        transacciones_confirmadas: confirmadas.len(),
        transacciones_descartadas: iniciadas.difference(&confirmadas).count(),
        operaciones_reaplicadas: operaciones,
    })
}
```

Dos pasadas, una decisión:

- **Pasada 1** junta los `tx_id` con `Commit`. Es O(N) sobre el log (N = número de registros). Te dice, sin importar el orden, quién lived to tell the tale.
- **Pasada 2** re-aplica, en orden de LSN (= orden del log), sólo las operaciones de las txs confirmadas. También O(N). Los Begin sin Commit posterior son DESCARTADOS: «como si nunca hubieran ocurrido». El `informe.transacciones_descartadas` los cuenta.

La alternativa — una sola pasada con estado por registro — exige tres estados por entrada (pendiente / en redo / confirmado) y no resuelve la intercalación de Begin/Operación/Commit/Rollback de V transacciones que el log admite por construcción (preparado para el group commit). Dos pasadas es la complejidad mínima que respeta la intercalación.

### El truncado con contrato

```rust
pub fn truncar_hasta_lsn(&mut self, lsn: Lsn) -> usize {
    // … busca el primer registro con lsn > lsn, descarta lo anterior …
    // NUNCA: next_lsn, next_tx_id — los LSNs no se reutilizan.
}
```

El log puede ser enorme; en algún momento hay que podarlo. La poda es SEGURA sólo si los registros podados ya son visibles en el store. **Si se trunca más allá, los redos posteriores pueden quedar HUÉRFANOS** (arista con un nodo que ya no está en el log). El contrato del llamador cierra la puerta:

> *«Truncar lo no-durable PIERDE datos: el replay sólo ve lo que queda. El checkpoint que decide «hasta dónde es seguro» de forma automática es el cap. 29; la rotación por tamaño del fichero queda como deuda documentada.»*

Lo que NUNCA se reinicia: `next_lsn` / `next_tx_id`. La identidad de un redo es su LSN, no su posición en bytes. Si el llamador trunca los primeros 2000 LSNs y luego escribe una tx nueva, los nuevos LSNs empiezan en 2001, no en 1. Que dos redos tuvieran el mismo LSN sería una corrupción lógica; el motor los distinguiría por la posición en bytes, sí, pero la identidad del registro se rompería debajo.

## 28.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap28_wal.rs`. Vamos a leerlo por partes, porque cada `commit` y cada `replay` tiene un porqué.

### La transacción con WAL

```rust
pub struct WalTransaccion<'a> {
    store: &'a mut dyn GraphStore,
    wal: &'a mut Wal,
    tx_id: TxId,
    buffer: Vec<Operacion>,
}

impl<'a> WalTransaccion<'a> {
    pub fn begin(store: &'a mut dyn GraphStore, wal: &'a mut Wal) -> Self {
        let (tx_id, _lsn) = wal.begin_tx();
        WalTransaccion { store, wal, tx_id, buffer: Vec::new() }
    }

    pub fn stage(&mut self, operacion: Operacion) -> Result<(), TransaccionError> {
        self.buffer.push(operacion);
        match validar_buffer(self.store, &self.buffer) {
            Ok(()) => Ok(()),
            Err(e) => { self.buffer.pop(); Err(e) }
        }
    }

    pub fn put_node(&mut self, node: Node) -> Result<(), TransaccionError> {
        self.stage(Operacion::PutNode(node))
    }

    pub fn commit(self) -> Result<ResumenCommitWal, WalError> {
        // (cuerpo del commit en dos fases, ver §28.6)
    }

    pub fn rollback(self) -> ResumenRollbackWal {
        let lsn_rollback = self.wal.log_write(self.tx_id, CuerpoWal::Rollback);
        ResumenRollbackWal {
            operaciones_descartadas: self.buffer.len(),
            lsn_rollback,
        }
    }
}
```

Tres préstamos exclusivos simultáneos — uno al store, otro al WAL, otro a sí misma — codifican el «un único escritor» del cap. 27. Mientras la transacción vive, nadie más toca el store NI el WAL. El ciclo de vida en los tipos: `commit` y `rollback` consumen `self`; usar una tx cerrada o anidar dos sobre el mismo store no compila.

Y un detalle que la prosa debe decir: el `Drop` implícito (no hay `Drop` explícito) no escribe Rollback. ¿Por qué? Porque si la tx se cierra sin commit ni rollback, lo más probable es que sea un `unwrap()` intermedio o un pánico. **No escribir el marker** es la decisión correcta: la ausencia de Commit cancela la tx en el replay, y un marker Rollback escrito en el panic path sería mentiroso (el store «no murió» realmente, sólo el unwrap falló). El `Wal::iter` la descarta con la misma pinta: sin Commit, la tx no ocurrió.

### La `PoliticaFlush`

```rust
pub enum PoliticaFlush {
    CadaEscritura,  // sync tras cada log_write: la regla de oro, literal
    SoloCommit,     // sync SÓLO en el commit: un sync por tx; semilla del group commit
}
```

Se enseña la regla antes que la optimización. El alumno que tropieza con `SoloCommit` primero NO entiende por qué un sync por tx es suficiente, y el día que la máquina sufra un fsync lento y la mitad de los commits se alarguen misteriosamente, no sabe dónde mirar. Con `CadaEscritura` por defecto, el alumno mide 4 syncs para 3 ops + commit; con `SoloCommit`, mide 1. Y cuando el cap. 30 enchufe concurrencia, la semilla ya está plantada.

El `Wal::default` se implementa a mano, no se deriva: la política por defecto es CONTENIDO del capítulo, no azar. `#[derive(Default)]` requeriría un `PoliticaFlush::default()` que decide quién sabe cómo. El `Default` manual fija `CadaEscritura` y documenta la decisión.

## 28.8 Prueba de fuego

La prueba de fuego no es «los tests pasan» — es **«el agujero del cap. 27 está cerrado»**. El test-tesis del capítulo reproduce el escenario que el cap. 27 AFIRMABA (con `node_count()==2` y «sin log no se pueden deshacer») y lo termina al revés.

```rust
// El mismo StoreQueFalla del cap. 27, las mismas 4 ops, el mismo fallar_en: 3.
let mut store = StoreQueFalla {
    inner: MemoryStore::new(),
    escrituras: 0,
    fallar_en: 3,
    con_panic: false,
};
let mut wal = Wal::new();
let mut tx = WalTransaccion::begin(&mut store, &mut wal);
tx.put_node(Node::new(0, "A")).unwrap();
tx.put_node(Node::new(1, "B")).unwrap();
tx.put_node(Node::new(2, "C")).unwrap();  // el store fallará aquí
tx.put_node(Node::new(3, "D")).unwrap();
let err = tx.commit().unwrap_err();

// MISMA FORMA que el cap. 27…
assert_eq!(
    err,
    WalError::ApplyFallido {
        indice: 2,
        aplicadas: 2,
        causa: StoreError::UnknownNode(usize::MAX)
    }
);
assert!(err.to_string().contains("replay_wal"));

// …PERO el log SÍ contiene TODO + Commit + sync.
assert_eq!(store.node_count(), 2);                     // a medias, como el cap. 27
assert_eq!(wal.syncs(), 1 + 4);                        // commit + 4 escrituras
let ultimo = wal.iter().last().unwrap();
assert_eq!(ultimo.cuerpo, CuerpoWal::Commit);          // EL COMMIT ESTÁ

// EL RESCATE: replay sobre el MISMO store a medias → COMPLETA.
let informe = replay_wal(&mut store, &wal).unwrap();
assert_eq!(informe.operaciones_reaplicadas, 4);
assert_eq!(store.node_count(), 4);                      // LA TRANSACCIÓN COMPLETA
assert!(store.get_node(3).is_some());                  // el nodo "D" llegó
```

Y la versión con `panic!` (el `corte de luz` literal del cap. 27):

```rust
let mut store = StoreQueFalla { fallar_en: 2, con_panic: true, … };
let resultado = catch_unwind(AssertUnwindSafe(|| {
    let mut tx = WalTransaccion::begin(&mut store, &mut wal);
    tx.put_node(Node::new(0, "A")).unwrap();
    tx.put_node(Node::new(1, "B")).unwrap();  // pánico AQUÍ
    tx.put_node(Node::new(2, "C")).unwrap();
    tx.put_node(Node::new(3, "D")).unwrap();
    tx.commit()
}));
// Rescatamos el pánico, el store quedó con 1 nodo.
assert_eq!(store.node_count(), 1);
// Pero el log SÍ terminó con Commit.
assert!(matches!(wal.iter().last(), Some(r) if r.cuerpo == CuerpoWal::Commit));
// Y el replay completa.
let informe = replay_wal(&mut store, &wal).unwrap();
assert_eq!(informe.operaciones_reaplicadas, 4);
assert_eq!(store.node_count(), 4);
```

Ésta es la inversión de la regresión del cap. 27: el escenario que aquél cerraba con «y nadie recuerda qué faltaba», éste lo abre con «el log SÍ recuerda» y lo cierra con `node_count()==4`. La forma del error es la MISMA (`ApplyFallido{indice: 2, aplicadas: 2}`); el contenido del `Display` cambió. La prueba de que el capítulo sirve es que vuelve falsa la afirmación del anterior.

Y los casos de fallo son igual de importantes:

```rust
// Un CRC tocado en el ÚLTIMO byte del log:
let mut bytes = encode_wal_record(&rec1);
bytes.extend(encode_wal_record(&rec2));
bytes[bytes.len() - 1] ^= 0xFF;  // touch the CRC
let err = decodificar_wal(&bytes).unwrap_err();
assert!(matches!(err, WalError::CrcInvalido { lsn: Some(2), .. }));

// Un registro truncado por un corte de luz:
let cortado = &completo[..completo.len() - 3];
let err = decodificar_wal(cortado).unwrap_err();
assert!(matches!(err, WalError::RegistroTruncado { .. }));

// Un hueco de LSN (bytes quitados de en medio):
let err = decodificar_wal(&encode_con_hueco).unwrap_err();
assert_eq!(err, WalError::LsnInvalido { leido: 5, esperado: 2 });

// Un truncado agresivo (rompe dependencias):
let err = replay_wal(&mut vacio, &wal_truncada).unwrap_err();
assert!(matches!(err, WalError::RedoFallido { lsn: 6, .. }));
```

Estos tests — `crc_invalido_detectado`, `registro_truncado_detectado`, `lsn_invalido_en_cadena_detectado`, `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias` — encapsulan las cuatro promesas del WAL: **detectar corrupción, detectar truncado, detectar huecos, fallar ruidosamente cuando el contrato del truncado se rompe**. Si este capítulo se te olvidara, las dos entradas (A y D) del `informe_acid()` volverían a `NivelGarantia::Ninguna` y el `StoreQueFalla` del cap. 27 volvería a dejar el store a medias «sin vuelta atrás».

## 28.9 Qué hemos sacrificado

Toda estructura tiene un precio. El WAL no es gratis:

1. **Trabajo extra en cada commit.** El log se escribe entero ANTES del apply; las escrituras adicionales son O(N) registros por transacción. Sin WAL, el commit era un loop sobre el buffer. El precio es la AUDITABILIDAD: lo que se gana es que ningún commit confirmado se pierde.
2. **El log crece sin parar.** Sin truncado, el log crece hasta llenar el disco. El truncado con contrato del llamador es una solución PARCIAL: el cap. 29 construirá el checkpoint que decide «hasta dónde» automáticamente.
3. **Sin UNDO, una tx con apply a medias se rescata con replay.** Pero si la política fuera *steal* (aplicar antes de confirmar), el apply a medias de una tx abortada exigiría UNDO — antes-images o CLR. La pregunta «¿UNDO o no?» es de DISEÑO, no de implementación: la mitad «no-UNDO» es lo que hace al WAL del cap. 28 tan simple.
4. **El `ApplyFallido` mantiene la forma `ApplyFallido{ indice, aplicadas, causa }` del cap. 27.** Quien migró del cap. 27 al cap. 28 ve un cambio de `Display` y un cambio de signatura del wrapper (`WalError::ApplyFallido` en vez de `TransaccionError::ApplyFallido`). El compilador lo señala, pero la ambigüedad conceptual — «¿es lo mismo?» — es justo la que el capítulo quiere que el alumno resuelva.
5. **Group commit REAL (varias tx concurrentes compartiendo un fsync) NO está implementado.** La semilla — `SoloCommit` con un sync por tx — sí. La concurrencia es el cap. 30. Deuda documentada, código plantado.

## 28.10 Cómo lo hace una BBDD real

El WAL del capítulo es mínimo. Las bases de datos reales añaden piezas que aquí NO están (y se nombran como «luego lo verás», honestamente):

- **Persistencia real.** Un WAL en RAM es un `Vec<u8>`; un WAL real es un `File` abierto con `O_APPEND` y `sync_all` por medio. El `FilePager::sync` del cap. 12 es la pieza que enchufa el disco debajo del log. Lo que aquí se cuenta como un contador, allí es la `fsync(2)` del sistema operativo.
- **Checkpoint.** Quinielamos: ¿hasta qué LSN es seguro truncar? El cap. 29 construye el algoritmo — normalmente, el LSN hasta el cual TODAS las páginas de datos han sido llevadas a disco (con `dirty page table` y todo eso). ARIES clásico: la dirty page table y el `recLSN` por página.
- **Steal / no-steal.** Aquí NO se roba: sólo se aplica después del commit. Por eso UNDO no hace falta. Una BD real con steal (deja páginas modificadas en disco antes del commit) necesita antes-images o CLR — la otra mitad de ARIES.
- **Group commit.** Varias tx concurrentes comparten un fsync: la semilla es `SoloCommit` sin concurrencia; el cierre es el cap. 30.
- **Write-ahead con reordenación.** Una BD grande puede diferir la escritura de la página de datos al disco LUEGO del commit, siempre que la página esté en el log antes; la política es la misma, la cola es más larga.

En todas, el patrón es idéntico al que has construido: **registro con framing + política de flush explícita + commit-marker-ANTES-del-apply + redo idempotente + truncado con checkpoint**. Lo que cambia es la cantidad de piezas de cada tipo y la finura del contrato.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: predice los LSNs y el número de syncs de una transacción con 2 puts y 1 delete, en `CadaEscritura`, y compruébalo con `commit_aplica_todo_y_es_durable`.
- *Intermedio*: cambia la `Tag` numérica de un `Operacion::PutNode` (1 → 5) en el encoding y demuestra que el roundtrip falla con `tag de cuerpo desconocido`. ¿Por qué la serialización trata los tags como una decisión ABI?
- *Experto*: añade una nueva variante `Operacion::PutLabel(NodeId, String)` que asigne una segunda etiqueta a un nodo existente. Codifícala, decodifícala, propágala por el `apply` del `commit` y por `aplicar_para_redo`, y rehaz el ciclo de tests.

## 28.11 Lo que te llevas

- La **regla del write-ahead** es una promesa sobre la dirección de la recuperación: si el log se escribe ANTES, el camino es re-aplicar (redo). Si se escribe DESPUÉS, el camino es deshacer (undo). La mitad que cuesta menos es la primera.
- El **LSN** es la dirección física del log y la identidad del redo: monótono, consecutivo, asignado por el `Wal`, **nunca reutilizado** (ni tras truncar).
- El **commit en dos fases** del capítulo: re-validar → log_write de cada op (sync según política) → Commit + sync → apply. El `Commit` se sella ANTES de que el apply toque el store.
- El **redo idempotente** es lo que permite re-aplicar sin saber qué sobrevivió: put idéntico = no-op, put divergente = overwrite, delete de lo ausente = no-op.
- La **parada limpia** del iterador ante corrupción/truncado/LSN-no-consecutivo + el `decodificar_wal` que grita el LSN aparente del daño.
- El **truncado con contrato** del llamador: los LSNs no se reinician; truncar lo no-durable PIERDE datos.
- `CadaEscritura` es la regla de oro; `SoloCommit` es la semilla del group commit. La diferencia en fsync es 4 vs 1, con el mismo resultado al hacer replay.
- El **`ApplyFallido` con la misma forma del cap. 27** y su `Display` cambiado: «replay_wal COMPLETA la transacción (arranque automático: cap. 29)».
- **Tocar el `ApplyFallido` del cap. 27 para que rescate la transacción**, es la inversión de la regresión del cap. 27 — la prueba de que el capítulo sirve.

## 28.12 Ojo, cuidado con…

- **Confundir commit con persistencia.** El commit del cap. 27 era en RAM; la durabilidad es del **registro `Commit` en el log + `sync`**, y por eso `ResumenCommitWal` lleva `lsn_commit`. Síntoma: alumno lee `ResumenCommit` del cap. 27 y asume que la tx está durable en disco.
- **Confundir `CadaEscritura` y `SoloCommit` con fsync de verdad.** `Wal::sync` es un CONTADOR en RAM; la `sync_all` REAL es la del `FilePager::sync` del cap. 12. Mide la política, no el disco.
- **Truncar lo no-durable, en serio.** `truncar_hasta_lsn(lsn)` es PODEROSO. Si pasas el LSN de una tx intermedia (apply a medias) y luego llamas a `replay_wal` sobre un store VACÍO, los nodos se pierden y las aristas quedan huérfanas (`RedoFallido { lsn: 6, ... }`). El test `replay_falla_ruidosamente_si_el_truncado_rompio_dependencias` es la DEMOSTRACIÓN; el contrato del llamador en el doc es la PREVENCIÓN.
- **No confundir `LSN` con `posición en bytes`.** El primero es la dirección física del log, monótono, nunca reutilizado. El segundo cambia con el truncado. La identidad de un redo es la primera.
- **Olvidar la ordenación de las props.** El `encode_props` ordena las claves antes de serializar para que el CRC dé lo mismo. Si reordenas a mano o cambias el sort por un HashMap.iter, el roundtrip falla — porque el CRC del mismo nodo cambia.
- **Creer que la «semilla del group commit» es group commit.** No: es el `SoloCommit` con UN sync por tx. El group commit concurrente es OTRA cosa (varias tx compartiendo un fsync) y exige concurrencia real — cap. 30.

## 28.13 Pin de batalla

> *«"El log se escribe antes que el dato" no es una orden de paso. Es una promesa sobre la dirección de la recuperación. Si el log se escribe ANTES, la mitad desconocida del problema se resuelve releyendo. Si se escribe DESPUÉS, se resuelve deshaciendo — y deshacer dobla el log. La pregunta "¿UNDO o no?" se contesta ANTES de escribir la primera línea de código.»*

## 28.14 Si solo lees 30 segundos

La transacción del cap. 27 vivía en RAM; un cierre del proceso se llevaba la confirmación. El **Write-Ahead Log** lo arregla con tres decisiones: (1) el log se escribe y sella con un `Commit + sync` ANTES de que el apply toque el store; (2) cada registro lleva un **LSN** monótono y un CRC32; (3) el **redo idempotente** re-aplica lo confirmado y descarta lo no confirmado. Si el apply falla a medias, el log YA contiene el `Commit` y un `replay_wal` completa la transacción. `CadaEscritura` es la regla de oro (un fsync por cada log_write); `SoloCommit` es la semilla del group commit (un fsync por transacción). El truncado es con CONTRATO del llamador y los LSNs no se reinician.

## 28.15 Una historia pequeña

Cuando implementamos el cap. 27 por primera vez, los tests del `StoreQueFalla` se veían finos: el `commit` devolvía `ApplyFallido{indice: 2, aplicadas: 2}`, el `Display` del error adjetivaba «sin log no se pueden deshacer», y la transacción «no se completó». Los tests verdes. El módulo compilaba. Pero el `informe_acid()` del cap. 27 dejaba la **D** en `NivelGarantia::Ninguna` con un comentario al lado: «en RAM, no durable (cap. 28)». Y la **A** en `Parcial` con la coletilla: «frente a validación; NO frente a un fallo del apply real».

Eso no era un módulo roto. Era un módulo HONESTO. Y la honestidad es lo que el capítulo 28 viene a cambiar. Hoy, el `Display` de `WalError::ApplyFallido` lleva, en su última línea, la frase que invierte la regresión: «replay_wal COMPLETA la transacción (arranque automático: cap. 29)». Y el `informe_acid_post_wal()` cierra la **D** de `Ninguna` a `Parcial` con un LOG en mayúsculas. La tx que ayer se perdía en un cierre de proceso, hoy es recuperable con un replay. Es la promesa del write-ahead, hecha cumplir.

## Ejercicios resueltos

**1. ¿Por qué el `Begin` no se sincroniza y el `Commit` sí?**

Porque la ausencia de `Commit` ya cancela la transacción en el replay: si la tx se cierra sin Commit, no «ocurrió» del lado de la durabilidad. El `Begin` necesita quedar en el log para que la pasada 1 del `replay_wal` pueda contar la tx como «iniciada pero no confirmada» y descartarla, pero la ausencia de `Commit` ya implica eso. En cambio, el `Commit` es el MOMENTO de la durabilidad: si ese registro se pierde (corte de luz a mitad de su escritura), la tx NO se considera confirmada. Un `sync` antes del `Commit` no impide que el `Commit` mismo se pierda; lo que lo protege es que, o se escribe entero (y entonces el sync lo sella), o se escribe a medias y un crc/truncado lo detecta como cola rota. La asimetría es la razón por la que se enseñan dos políticas de `sync` distintas: el `Begin` no lo necesita; el `Commit` SÍ.

**2. ¿Por qué el `replay_wal` descartaBegin` Commit`s y replica operaciones, en ese orden y no otro?**

Porque la pasada 1 (`HashSet` de `tx_id` con `Commit`) resuelve la pregunta lógica («¿quién confirmó?») sin importar el orden temporal: el `Commit` es lo que decide, no el orden de las ops. La pasada 2 (en orden de LSN) ejecuta la respuesta en el orden FISICO del log, lo que garantiza dos cosas: (a) si dos txs tocaron el mismo nodo (algo que no pasa hoy sin concurrencia, pero el log ya lo admite), el replay las aplica en el orden en que se firmaron; (b) si una tx posterior depende de una anterior (una arista al nodo que la primera crea), el LSN menor garantiza que la dependencia se aplica antes. La regla «orden de LSN = orden de aplicación» es el reflejo físico del orden lógico «orden de Commit» dentro de cada tx.

**3. ¿Por qué los LSNs no se reinician tras truncar?**

Porque la identidad de un redo es su LSN, no su posición en bytes. Si el `Wal` llevara la cuenta de los LSNs restantes (digamos, que tras truncar se reinicia en 1), dos transacciones en distintas épocas del log podrían tener LSNs coincidentes; un redactor que viera los dos no sabría distinguirlos, y cualquier cosa que indexe por LSN (la `dirty page table` que ARIES usará, nuestro propio `RedoFallido { lsn, … }`) confundiría el presente con el pasado. La monotonía del LSN hace al log AUTO-REFERENCIABLE: «el nodo 7 que escribí en `lsn=42`» es una oración estable a través del truncado y del paso del tiempo.

## Ejercicios propuestos

**Esencial.** Predice, ANTES de ejecutar, el contenido y los LSNs de un log de UNA transacción `WalTransaccion` que crea 3 nodos y 2 aristas y confirma, con `CadaEscritura`. ¿Cuántos registros tiene? ¿Cuál es `lsn_commit`? ¿Cuántos `syncs` se cuentan? ¿Qué tipo de cuerpo tiene cada uno? Compruébalo con `commit_aplica_todo_y_es_durable` y `politica_por_defecto_y_syncs_por_escritura`. Criterio: predicción exacta de número de registros + `lsn_commit` + número de syncs.

**Intermedio.** Sobre un log con 12 registros (3 Begin, 6 Operacion, 3 Commit), TODOS con su CRC y LSN válidos, pero donde la transacción 2 NO tiene Commit (la operación 6 de 6 fue su última), calcula a MANO el `InformeReplay` sobre un store vacío (confirmadas, descartadas, reaplicadas) y el `node_count`/`edge_count` del store renacido. Compruébalo con `transacciones_intercaladas_solo_la_confirmada_sobrevive`. Criterio: informe + cuentas exactas y respuesta a «¿se tocó el store con la abortada?».

**Experto.** Toma un log del capítulo, corrompe UN byte en la mitad del cuerpo de un registro y PREDECIR qué devuelve `decodificar_wal` (tipo de error, `lsn` reportado), qué hace `WalIterator`, y qué ve `replay_wal`. Compara con `crc_invalido_detectado` y `corrupcion_al_inicio_el_replay_para_en_el_prefijo_integro`. Criterio: tipo de error + `lsn` correcto + semántica recuperación/estricto + afirmación de que el replay sobre store pre-poblado no duplica.

## Para profundizar

- **Mohan, Haderle, Lindsay, Pirahesh y Schwarz, «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging»**, ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162 (DOI 10.1145/128765.128770). El paper donde la decisión commit-marker-antes-del-apply se formaliza como la mitad REDO de ARIES, y la otra mitad (UNDO + Analysis) cierra la recuperación completa. El capítulo 29 es la otra mitad.
- **Jim Gray, «Notes on Database Operating Systems»**, IBM Research Report RJ 2188, 1978. La huella de la bitácora como columna vertebral de la recuperación en System R. La idea del LSN como ordinal nace aquí.
- **Bernstein, Hadzilacos y Goodman, «Concurrency Control and Recovery in Database Systems»**, cap. 11 («Logging and Recovery»), Addison-Wesley 1987. Los algoritmos de recovery usando LSN y la tabla de páginas sucias: la lectura más densa del tema.
- **Lampson y Sturgis, «Crash Recovery in a Distributed Data Storage System»**, Xerox PARC 1976. El precursor cuidadoso: la versión anterior a la invención del LSN, con un protocolo que también funciona pero es más caro.
- **«Database Internals» (Alex Petrov)**, capítulo 7. Layout del WAL en bases reales (PostgreSQL, MySQL InnoDB) y los detalles de la persistencia que aquí hemos modelado con un contador.
- **CMU 15-445 (Intro to Database Systems)**, Lecture 17 — «Write-Ahead Logging», con el proyecto de BusTub que implementa un WAL REAL (con file system, log sequence, y todo lo que el cap. 29 promete).
- **Código fuente de SQLite** (`pager.c`, `wal.c`): la implementación de un WAL real en producción, con sus group commit, su checkpoint y su truncate.

## Mini-diálogo: en guardia nocturna

> — O sea, que el WAL es como un cuaderno de notas antes de hacer un cambio.
>
> — Un poco más serio: es un cuaderno de notas con un sello notarial. Cada anotación lleva un número correlativo, un checksum, y la garantía de que el cuaderno se firma EN SECO antes de tocar el original. Si la firma llega al final pero el original se quedó a medias, relees el cuaderno y completas. Si la firma NUNCA llegó, el original nunca se tocó.
>
> — ¿Y por qué COMMIT antes y no después?
>
> — Porque entonces el cuaderno contiene la historia ANTES de que la historia se ejecute. Releer es «haz otra vez lo que el cuaderno dice». Si el COMMIT fuera al final, el cuaderno contendría la historia DESPUÉS de que la historia se ejecutó — y releer no terminaría lo que se quedó a medias, tendría que DESHACERLO. La mitad de la maquinaria de undo es justo ésa: que el cuaderno tiene imágenes ANTES y DESPUÉS, y el sistema elige cuál mira.
>
> — Entonces lo de undo es para más tarde.
>
> — Sí. El cap. 29. Cuando ya tengamos el cuaderno en disco y el motor abra la base de datos, ARIES completo: Analysis (averigua qué quedó a medias), Redo (lo que el cuaderno dice que se confirmó), Undo (lo que el cuaderno dice que se abortó). La Parte VI del libro es eso.

---

*(Próximo capítulo: 29 — Recuperación después de un fallo (ARIES simplificado). Aquí el `replay_wal` se invocaba a mano. Ahora se enchufa al abrir la base de datos persistente: el log se escanea, el store se redibuja, y la pregunta "¿UNDO?" se responde con la misma claridad con la que este capítulo respondió la pregunta previa.)*
# Capítulo 29 — Recuperación después de un fallo (ARIES simplificado)

> *«Un log no vale lo que sus bytes: vale lo que puede reconstruirse con ellos tras un corte de luz.»*

## 29.0 La anécdota de la esquina

Hacia 1988, en el IBM Almaden Research Center, un grupo de cuatro ingenieros —Chandra Mohan, Don Haderle, Bruce Lindsay, Hamid Pirahesh y Peter Schwarz— publicó un paper interno de cuarenta páginas que llevaba un título para entonces modesto: «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollback Using Write-Ahead Logging». ARIES son las siglas de Algorithm for Recovery and Isolation Exploiting Semantics. Lo modesto era el título; lo que proponían era la receta que, treinta y cuatro años después, siguen usando Postgres, InnoDB, Oracle, SQL Server y una larga lista de sistemas que, cambiaron implementaciones, ajustaron políticas y pulieron heurísticas, pero no tocaron el esqueleto.

¿Por qué ese esqueleto triunfó? Porque antes de ARIES las bases de datos recuperaban su estado tras un crash de cuatro o cinco maneras distintas, cada una con su propio catálogo de bugs: unas no deshacían un apply a medias, otras descubrían el undo necesario cuando ya era tarde, las había que asumían no-steal por construcción y pagaban el precio en buffer pool bloqueado. Mohan y compañía miraron el problema de frente y se preguntaron —en una sola frase— qué tres cosas tiene que hacer un motor al despertar: **(1) reconstruir lo que sabe del mundo**, **(2) llevar el store EXACTAMENTE al estado del corte**, y **(3) retroceder lo que las transacciones rotas no debieron haber dejado**. Las llamaron, en orden, **Analysis**, **Redo**, **Undo**. Y convinieron en llamarlo ARIES, como al pastor germánico que guía a la base de datos de vuelta a casa tras el corte de luz.

Este capítulo es la versión sencilla de ese esqueleto, sobre `MemoryStore` y el WAL del cap. 28. No es un ARIES para producción: faltan los registros de compensación (CLR) y el log de before-image — los huecos que un motor de verdad cierra con una capa más. Pero es un ARIES HONESTO: las tres fases están, el orden está, y la pieza más interesante del capítulo —qué hacer cuando falta la imagen anterior de un elemento borrado— se documenta como una CUENTA en el informe de recuperación, no como un bug silencioso. La honestidad manda cuando los logs son uno de los pocos lugares donde mentir tiene consecuencias duraderas.

## 29.1 Objetivo

Al terminar este capítulo sabrás **cómo se reconstruye un motor transaccional tras un corte de luz**, y habrás construido las tres fases del algoritmo que se convirtió en estándar: Analysis, Redo, Undo. Cuatro piezas en `cap29_recuperacion.rs`:

1. **`analizar(wal)`** — la fase 1: recorre el log hacia delante y reconstruye la tabla de transacciones, los contadores `next_lsn`/`next_tx_id` y la *dirty element table* (qué elementos tocó una operación y en qué LSN).
2. **`redo` + `deshacer`** — las fases 2 y 3: re-aplicar todo en orden de LSN (incluidas las perdedoras robadas) y deshacer después, en orden inverso, sólo las perdedoras.
3. **`guardar_wal` + `cargar_wal` + `reabrir`** — el FICHERO del WAL: el `sync` del cap. 28 era un contador; aquí el fichero es el almacenamiento estable real.
4. **`Checkpoint` + `truncar_seguro` + `rotar_si_excede`** — el truncado ahora automatizado y la rotación por tamaño (la deuda que el cap. 28 dejaba firmada a mano).

Y el hito del capítulo: una perdedora con **steal** —es decir, que ya escribió al store antes de morir— se RECUPERA de un crash y deja el store como si nunca hubiera existido. Medido, no prometido.

## 29.2 Problema

El cap. 28 dejó la regla «commit-marker-antes-del-apply» hecha protocolo, y construyó lo más difícil: **`replay_wal` re-aplica las operaciones de las confirmadas en orden de LSN, idempotente**, y un apply a medias de una CONFIRMADA se completa por roll-forward. Funciona, y bajo la política no-steal del cap. 28 es incluso elegante: una perdedora no tocó el store (su staging vive en `WalTransaccion` y sólo aplica tras el Commit), y el undo es trivialmente vacío.

Pero elegante NO es lo que necesita una base de datos de verdad. Mira los tres problemas concretos que el cap. 28 dejó con la palabra *deuda* escrita al lado:

1. **El «disco» del WAL es un contador.** `Wal::sync()` del cap. 28 incrementa un entero. Los tests verifican que el contador sube; nadie verifica que los bytes lleguen a un sitio del que se pueda RECUPERAR. Cuando el proceso muere, el `Wal` se evapora: el próximo `Wal::new()` parte de cero.
2. **El truncado es a mano.** `truncar_hasta_lsn` firma el contrato «truncar lo no-durable pierde datos» — el llamador decide cuándo sabe que algo es durable. Lo que el operador humano decida se queda en el OPERADOR, no en el motor. Necesitamos un protocolo donde esa decisión sea de la base de datos, no del DBA a las tres de la mañana.
3. **No hay UNDO.** El staging vive en `WalTransaccion` y sólo aplica tras el Commit. Una transacción no confirmada NO dejó huella en el store, así que no hay nada que deshacer. Esto es la política **no-steal** — un buffer pool que sólo evacúa páginas limpias. Funciona, pero BLOQUEA: hasta que la transacción confirma o aborta, sus páginas sucias ocupan memoria, y bajo carga se traduce en latencia que sube.

Y hay un cuarto problema que el cap. 28 no escribió como deuda, pero que sale a la palestra en cuanto el tercero se arregla: **si un buffer pool REAL evacua páginas sucias para hacer hueco (steal)**, las perdedoras DEJARON escritura en el store. Esa escritura no fue autorizada por ningún Commit. Si la recuperación del cap. 28 se ejecuta, las perdedoras robadas SOBREVIVEN al reinicio. Eso es la pregunta que ARIES responde: ¿cómo levanto la base de datos después de que un motor real la dejara a medias — con partes confirmadas, partes perdedoras, y partes robadas?

## 29.3 Modelo mental

Piensa en un **museo que se reconstruye después de un incendio a partir del vídeo de las cámaras de seguridad**. Las cámaras no parpadean, escriben cada movimiento (el WAL); al reconstruir:

- **ANÁLISIS** = mirar el vídeo HACIA DELANTE y reconstruir el mapa: ¿quién estaba activo? ¿qué pieza tocó qué? ¿quién confirmó y quién se cayó a medias? Es la fase LECTORA: del vídeo deduces la tabla de transacciones, los contadores `next_lsn`/`next_tx_id`, y la *dirty element table* (qué elemento fue tocado por primera vez en qué LSN).
- **REDO** = REPETIR cada movimiento de las cámaras, en orden, hasta dejar la colección EXACTAMENTE como estaba en el instante del corte — incluido lo que una pieza rota empezó a mover sin terminar. Es la fase RE-ESCRITORA: re-aplica TODOS los registros — ganadoras y perdedoras — con la idempotencia del cap. 28. Al terminar, el store está en el estado del fallo.
- **UNDO** = borrar las huellas de las piezas que se cayeron a medias y no debieron seguir, en orden inverso al que aparecieron en el vídeo. Es la fase DESHACEDORA: de atrás hacia delante, compensa cada operación perdedora con su inversa lógica (un `PutNode` robado se borra; un `DeleteNode` robado se restaura desde su imagen anterior). Las huellas desaparecen; el museo queda como si las perdedoras nunca hubieran pasado por ahí.

```
                     ┌──────────────────────────────────────────┐
                     │ log en disco (reabierto = leer + escanear)│
                     └───────────────────┬──────────────────────┘
                                         │
                                         ▼
                            ANÁLISIS (hacia delante)
                            tabla de transacciones
                            contadores
                            dirty element table
                                         │
                                         ▼
                            REDO (hacia delante, TODO)
                            aplicar_para_redo idempotente
                                         │
                                         ▼
                            UNDO (hacia atrás, sólo perdedoras)
                            compensación lógica + before-image
                                         │
                                         ▼
                  store consistente = ganadoras confirmadas,
                                       sin huellas de perdedoras
```

**El truco del steal.** En un buffer pool real, las páginas sucias de una tx NO confirmada se evacuan al disco para hacer hueco — el steal. La fase de REDO las DEJA en el store (las re-aplica); la fase de UNDO las BORRA. Si sólo rehiciéramos las ganadoras (como hacía `replay_wal` del cap. 28), el undo no tendría de dónde partir: «¿qué hago si una página robada no está?». Redo + undo, en ese orden, es la única forma de tener una base coherente sobre la que retroceder.

**La frontera del before-image.** El vídeo de las cámaras enfocó el RESULTADO (la vitrina NUEVA), no el plano de la vitrina VIEJA. Si una perdedora borró un nodo y se llevó el contenido, no tenemos manera de saber qué ponía en la vitrina antes — y `operaciones_sin_before_image` lo CUENTA, no lo inventa. ARIES completo graba también el ANTES (before-images y CLR); aquí lo decimos y contamos: el motor HONESTO informa lo que no pudo hacer, no aborta silenciosamente y tampoco maquilla el resultado.

El momento ¡ajá!: «El redo no es "rehacer lo bueno": es "rehacer TODO, hasta dejar la colección EXACTA como estaba en el corte, incluidas las obras medio movidas". El undo es la pieza que quita las huellas de las perdedoras. Y un log que sólo guardó el resultado no puede restaurar lo que se borró — esa frontera, contada, es lo que separa un motor honesto de uno que miente.»

## 29.4 Primera solución

La solución ingenua es la del cap. 28, ya funcional: `replay_wal` re-aplica las operaciones de las confirmadas, en orden de LSN, con idempotencia. El operador humano lo ejecuta tras un crash; si tuvo cuidado, reconstruye lo bueno. La perdedora, sin commit marker, NO tocó el store (la política no-steal del cap. 27), y el undo es trivialmente vacío.

```
Fase (cap. 28): replay_wal(wal, store)
  → re-aplica Operaciones de tx con Commit
  → idempotente (un apply repetido es un no-op)
  → no toca perdedoras (no tocaron nada)
  → requiere Wal manualmente reanimado tras un crash
  → requiere truncado a mano si el log crece
```

Es una solución CORRECTA, SIMPLE y HONESTA bajo no-steal. Los tests la verifican (`replay_es_idempotente`, `replay_re_construye_lo_confirmado`). El propio cap. 28 lo escribe: «la alternativa (marker al final) exigiría UNDO para rescatar el apply a medias — eso es ARIES».

## 29.5 Sus límites

Tres límites que te empujan al cap. 29:

1. **Steal rompe no-steal.** El momento en que un buffer pool REAL evacua páginas sucias de una tx no confirmada para hacer hueco —ese es el steal—, las escrituras robadas están en el store. `replay_wal` del cap. 28 las IGNORA porque la tx no era ganadora; el store post-replay arranca con datos no autorizados. Síntoma: tras un crash con carga alta (muchas tx concurrentes, presión de memoria), las perdedoras sobreviven al reinicio.
2. **El `sync` del cap. 28 era un contador.** Cuando el proceso muere, `Wal::new()` parte de cero. Los `next_lsn`/`next_tx_id` se reinician; un nuevo Begin naciente con id 1 entra en el reinicio del log y se confunde con un id histórico. Modo de fallo: reorganización silenciosa de la historia, undo descompensando a la tx equivocada.
3. **Truncar a mano rompe Durabilidad silenciosamente.** `truncar_hasta_lsn` exige saber qué era durable. Sin un protocolo codificado (un Checkpoint que registre «hasta aquí todo es durable»), el operador trunca de más y la próxima recuperación falta de piezas.

La pregunta del capítulo: ¿qué pasa cuando un motor real, con buffer pool real y steal activo, se cae — y el operador no está para ejecutar `replay_wal` a mano?

## 29.6 Solución evolucionada: ARIES en tres fases

`recuperar(store, wal, antes)` ejecuta el esqueleto completo: análisis → redo → undo. Tres fases, en ese orden estricto, sobre el mismo log, sin coordinación entre ellas más que el flujo:

```rust
pub fn recuperar(store, wal, antes) -> Result<InformeRecuperacion, RecoveryError> {
    let analisis = analizar(wal);           // fase 1
    let operaciones_redo = redo(store, &analisis)?;   // fase 2
    let undo = deshacer(store, &analisis, antes);    // fase 3
    Ok(InformeRecuperacion { ... })
}
```

**Fase 1 — ANÁLISIS.** `analizar(&wal)` recorre el log hacia delante con `Wal::iter` (que PARA LIMPIO ante cola corrupta — el mismo contrato que el cap. 28 dejó para la iteración). Para cada registro, deduce:

- **Tabla de transacciones** (`EstadoTx::{Activa, Confirmada, Abortada}`): un `Begin` (o cualquier `Operacion`) la inserta Activa; un `Commit` la promueve a Confirmada (GANADORA); un `Rollback` la promueve a Abortada (PERDEDORA).
- **Contadores**: `next_lsn = max(lsn) + 1` y `next_tx_id = max(tx_id) + 1` se reconstruyen del contenido del log. Es la diferencia clave frente al cap. 28: los contadores no se «mantienen», se DEDUCEN.
- **Dirty element table** (`ElementoId → primer LSN que lo tocó`): es el análogo a la *dirty page table* de ARIES (Mohan et al. 1992, §3.2), pero a nivel de ELEMENTO del grafo — no de página de disco. `PutNode(n)`, `PutEdge(e)`, `DeleteNode(id)`, `DeleteEdge(id)` registran el elemento al que tocan; `sucias.entry(elem).or_insert(rec.lsn)` deja el PRIMER LSN — rehacer a partir de él con checkpoint es lo que ahorraría trabajo; aquí, sin checkpoint, el redo es conservador y todo lo recorre (`Analisis::primer_lsn_sucio` queda como información, no como punto de arranque real).

Al terminar la fase 1 tienes un mapa completo del estado del mundo justo antes del corte — sin haber tocado el store una sola vez. `primer_lsn_sucio` es el ancla del redo futuro cuando exista checkpoint; mientras tanto, el redo recorre todo.

**Fase 2 — REDO.** `redo(&mut store, &analisis)` re-aplica TODAS las operaciones en orden de LSN, ganadoras y perdedoras, usando `aplicar_para_redo` del cap. 28 (idempotente, ya usado por `replay_wal`). La diferencia CLAVE frente al replay del cap. 28:

```text
cap. 28 replay_wal:   re-aplica SOLO ganadoras  (política no-steal: 
                       las perdedoras no tocaron nada)
cap. 29 redo:         re-aplica TODO, ganadoras y perdedoras
                       (política steal: las perdedoras PUEDEN haber 
                       dejado huella — el undo necesita una base 
                       coherente sobre la que retroceder)
```

¿No es re-aplicar las perdedoras una LOCURA? Por la idempotencia de `aplicar_para_redo`, re-aplicar lo ya aplicado es un no-op; y si la perdedora sólo había escrito a través de steal (no-aplicada aún al store), el redo la INTRODUCE y el undo la BORRA después. Las dos fases son SECUENCIALES sobre el mismo registro de escrituras: redo deja el store en el ESTADO DEL FALLO; undo retrocede desde ahí hasta el conjunto de ganadoras. Sin redo-exhaustivo, el undo no tiene una base coherente.

**Fase 3 — UNDO.** `deshacer(&mut store, &analisis, &antes)` recorre los registros en orden INVERSO de LSN y deshace a las perdedoras:

```rust
for rec in analisis.registros.iter().rev() {       // HACIA ATRÁS
    if !perdedoras.contains(&rec.tx_id) { continue; }
    if let CuerpoWal::Operacion(op) = &rec.cuerpo {
        match op {
            PutNode(n)   => if store.get_node(n.id).is_some() {
                                store.delete_node(n.id);
                            }
                            informe.operaciones_deshechas += 1,
            PutEdge(e)   => /* análogo */,
            DeleteNode(id) => if store.get_node(*id).is_some() {
                                  // ya restaurado — idempotente
                              } else {
                                  match antes.get(&ElementoId::Nodo(*id)) {
                                      Some(Element::Node(n)) => 
                                          store.put_node(n.clone()),
                                      _ => informe.operaciones_sin_before_image += 1,
                                  }
                              },
            DeleteEdge(id) => /* análogo */,
        }
    }
}
```

La **compensación es lógica e idempotente**. ¿Por qué lógica y no física? Porque un log de solo after-image (el del cap. 28) no guarda las imágenes anteriores — sólo guarda cómo quedó cada elemento. Restablecer el estado anterior exige la imagen previa, que puede no estar disponible; lo que sí está es la INVERSA: un `PutNode` robado se deshace con un `delete_node` idempotente. El «antes» del `PutNode` es trivial: «no existía». El «antes» del `DeleteNode` NO — deshacer un borrado exige saber qué había, y eso sólo lo sabe un snapshot pre-robo (o un log con before-images, ARIES completo). Por eso el match: si `antes.get(...)` es `None`, `operaciones_sin_before_image += 1` — INFORMACIÓN, no error, no invención. La pieza del count es el capítulo honesto.

El **orden inverso** es la otra decisión: deshacer primero la arista y luego sus nodos evita que un `delete_node` arrastre aristas ajenas durante la cascada; deshacer «de atrás hacia delante» es lo que ARIES prescribe para que las compensaciones no interfieran entre sí.

El **informe** (`InformeRecuperacion`) agrega todo: cuántas ganadoras, cuántas perdedoras, cuántas operaciones se re-aplicaron, cuántas se deshicieron, cuántas quedaron sin imagen anterior (`operaciones_sin_before_image`), y los contadores reconstruidos (`next_lsn`, `next_tx_id`). Es la respuesta a la pregunta «¿qué se hizo al despertar?».

## 29.7 Solución evolucionada: el fichero y el checkpoint

ARIES sin fichero y sin checkpoint es un esqueleto, no un motor. El cap. 28 dejó dos deudas; el cap. 29 las cierra:

**El fichero del WAL.** `guardar_wal(&wal, path)` y `cargar_wal(path)` convierten los bytes del WAL en persistencia real. `std::fs::write` cierra el fichero y el sistema lo vuelca. `cargar_wal` reconstruye el `Wal` ESCANEANDO los bytes — `Wal::reconstruir(&bytes)` devuelve un Wal con `next_lsn = max(lsn) + 1` y `next_tx_id = max(tx_id) + 1`. Esto es la diferencia clave frente a `Wal::new()`: los contadores no se inventan, se DEDUCEN del prefijo íntegro.

```rust
pub fn guardar_wal(wal, path)    { std::fs::write(path, wal.as_bytes())? }
pub fn cargar_wal(path)         {
    let bytes = std::fs::read(path)?;
    Ok(Wal::reconstruir(&bytes))
}
pub fn reabrir(store, path, antes) {
    let bytes = std::fs::read(path).map_err(RecoveryError::Io)?;
    let wal    = Wal::reconstruir(&bytes);
    recuperar(store, &wal, antes)
}
```

El patrón de `fsync` riguroso ya existe (`FilePager::sync`, cap. 12; `BufferPool` ya sabe `flush`→`sync`); aquí «durabilidad real» = `guardar_wal` con la maquinaria de fichero del sistema. La separación `RecoveryError::Io` vs `RecoveryError::Redo {lsn, causa}` lleva la pregunta correcta (`¿fue el disco o fue el log?`) hasta el operador.

**El checkpoint que persiste los contadores.** `Checkpoint { hasta_lsn, next_lsn, next_tx_id }` no es una foto del store: es un REGISTRO del log (en ARIES real, un `WalRecord::Checkpoint`; aquí, una estructura aparte con el mismo papel). La parte crítica:

```text
hasta_lsn   = último LSN cuyo efecto es durable (todo ≤ a él se puede truncar)
next_lsn    = contador a reanudar tras el reinicio (congelado)
next_tx_id  = contador de TxId a reanudar (congelado)
```

¿POR QUÉ es crítico persistir los contadores? Porque truncar el log a vacío SIN guardarlos hace que `Wal::reconstruir` los ponga a 1 y REUTILICE identificadores. Un log recuperable no puede permitir eso: dos transacciones con el mismo id hacen que el análisis confunda cuál era cuál y el undo descompense a la que NO debía. El `Checkpoint::tomar(&wal)` los captura; el `truncar_seguro(wal, cp)` los usa para AUTOMATIZAR el truncado que el cap. 28 dejaba firmado a mano.

**La rotación por tamaño.** `rotar_si_excede(wal, umbral)` cierra la deuda «rotación por tamaño» del cap. 28 sin un scheduler: si `wal.as_bytes().len() > umbral`, tomar checkpoint y truncar. Política «una línea», decisión del llamador (en producción: cientos de MB, disparado por timer). La frontera `<=`/`>` del test lo verifica: por debajo del umbral no rota; justo al alcanzarlo (ya no `≤`) rota y trunca.

## 29.8 La frontera del before-image

`operaciones_sin_before_image` merece su propia sección porque es la pieza más honesta del capítulo. La pregunta que contesta: «¿qué pasa si deshacer requiere una imagen anterior que el log no tiene?»

```rust
DeleteNode(id) => {
    if store.get_node(*id).is_some() {
        // Ya restaurado por una pasada anterior — undo idempotente.
        informe.operaciones_deshechas += 1;
    } else {
        match antes.get(&ElementoId::Nodo(*id)) {
            Some(Element::Node(n)) => {
                let _ = store.put_node(n.clone());
                informe.operaciones_deshechas += 1;
            }
            _ => informe.operaciones_sin_before_image += 1,  // ← el hueco
        }
    }
}
```

Tres salidas por rama:

1. **El nodo YA está en el store** (porque una pasada anterior lo restauró, o porque no fue robado y un Confirmado legítimo lo dejó): se cuenta como deshecho y se sigue — la idempotencia es lo que permite que la recuperación sea re-ejecutable.
2. **El nodo NO está y `antes` lo tiene** (el llamador capturó `capturar_antes(&store)` ANTES del robo): se restaura con `put_node(n.clone())`. La imagen anterior es lo que te salva — pero exige que el LLAMADOR haya planeado la captura antes de que la perdedora tocara el store. Si capturas DESPUÉS, capturas la imagen de después, que no es la que quieres.
3. **El nodo NO está y `antes` NO lo tiene**: `operaciones_sin_before_image += 1`. INFORMACIÓN. No `panic!`, no `unreachable!`, no `unwrap_or_default()`. La recuperación REPORTA el hueco y devuelve el control. Es la decisión de diseño: el motor HONESTO deja que la decisión (¿continuar? ¿abortar?) sea del operador, no del motor.

**¿Por qué no abortar el reinicio por defecto?** Porque un borrado robado PERDEDOR sin before-image NO NECESARIAMENTE corrompe las ganadoras: la fase de undo puede haber restaurado todo lo demás correctamente, y este nodo en particular haber sido robado y borrado por una perdedora que abortó sin que nadie tomara la imagen previa. La aplicación podría haberlo recreado por su cuenta, o podría ser información de diagnóstico suficiente. La decisión es del operador.

**¿Por qué no inventar el dato?** Porque sería peor que abandonarlo. La base de datos "recuperaría" un nodo con `labels == ["DESCONOCIDO"]` y la siguiente consulta confiaría en él. La honestidad manda.

ARIES completo cierra este hueco con registros CLR (Compensation Log Records) que graban la imagen anterior al hacer el propio undo — escritura adicional al log, que se re-aplica con idempotencia completa. Es la pieza que AÑADE el undo a un log de after-image para hacerlo robusto. Aquí la dejamos como hueco documentado: el lector que quiera ir a ARIES real lee a Mohan, Don Haderle, Bruce Lindsay, Hamid Pirahesh y Peter Schwarz («ARIES» 1992).

## 29.9 Código completo ejecutable

Cuatro piezas en `liradb-workspace/crates/vol2-liradb/src/cap29_recuperacion.rs`, ~1.290 líneas, sin crates externas. Tres toques al cap. 28: `aplicar_para_redo` pasa a `pub(crate)` (reutilizado por el redo), `Wal::reconstruir(bytes)` y `Wal::next_tx_id()` se hacen públicas para que `cargar_wal` pueda deducir los contadores. Cero duplicación.

```rust
pub fn analizar(wal: &Wal) -> Analisis {
    let mut transacciones = HashMap::new();
    let mut sucias        = HashMap::new();
    let mut next_lsn = 1;
    let mut next_tx_id = 1;
    for rec in wal.iter() {                         // parada limpia
        next_lsn    = next_lsn.max(rec.lsn + 1);
        next_tx_id  = next_tx_id.max(rec.tx_id + 1);
        match &rec.cuerpo {
            CuerpoWal::Begin => { /* Activa */ }
            CuerpoWal::Operacion(op) => {
                for elem in ElementoId::de_operacion(op) {
                    sucias.entry(elem).or_insert(rec.lsn); // primer LSN
                }
            }
            CuerpoWal::Commit  => { /* Confirmada */ }
            CuerpoWal::Rollback => { /* Abortada */ }
        }
    }
    // ... ordena por primer LSN, deduplica ...
    Analisis { transacciones, sucias, next_lsn, next_tx_id, ... }
}
```

El REDO y el UNDO ya los cubrimos en la sección anterior; lo nuevo es la pieza administrativa:

```rust
pub struct Checkpoint {
    pub hasta_lsn:  Lsn,
    pub next_lsn:   Lsn,
    pub next_tx_id: TxId,
}

pub fn truncar_seguro(wal: &mut Wal, cp: &Checkpoint) -> usize {
    wal.truncar_hasta_lsn(cp.hasta_lsn)   // automatiza lo del cap. 28
}

pub fn rotar_si_excede(wal: &mut Wal, umbral_bytes: usize) -> Option<Checkpoint> {
    if wal.as_bytes().len() <= umbral_bytes { return None; }
    let cp = Checkpoint::tomar(wal);
    truncar_seguro(wal, &cp);
    Some(cp)
}
```

Y la re-valoración ACID que cierra el capítulo honestamente:

```rust
pub fn informe_acid_post_recovery() -> Vec<EntradaAcid> {
    vec![
        EntradaAcid { garantia: Atomicidad, nivel: Parcial,
          como_esta_hoy: "el arranque automático (análisis + redo + undo) 
                          repara un apply a medias y deshace lo no confirmado, 
                          incluso robado por steal; queda el before-image para 
                          deshacer borrados robados (ARIES completo lo cierra 
                          con CLR)",
          capitulo_que_la_cierra: 30 },                         // ← 29→30
        EntradaAcid { garantia: Durabilidad, nivel: Parcial,
          como_esta_hoy: "el WAL persiste a fichero y se reabre con recuperación: 
                          lo confirmado sobrevive al reinicio vía replay; el store 
                          de datos no tiene checkpoint independiente",
          capitulo_que_la_cierra: 37 },                         // ← 29→37
        // C e I: sin cambios (cap. 30).
    ]
}
```

Las dos flechas son la parte importante: A pasa del cierre 29 al 30 (queda el before-image), D pasa del 29 al 37 (queda el checkpoint del store de datos). El test que verifica las transiciones (`informe_post_recovery_actualiza_a_y_d`) compara contra `informe_acid_post_wal()` — el sistema de tipos lleva la trazabilidad.

## 29.10 Prueba de fuego

Tres tests explican el capítulo mejor que tres secciones más:

**TEST-TESIS** `undo_elimina_las_escrituras_robadas_de_una_perdedora`: una tx sin commit con `PutNode(0)`, `PutNode(1)`, `PutEdge(0, 0, 1)` logueadas. El store YA contiene esas escrituras (steal simulado aplicándolas a mano: `store.put_node(0)`, `store.put_node(1)`, `store.put_edge(...)`). `recuperar(store, wal, AntesImagenes::new())` devuelve `transacciones_perdedoras = 1, operaciones_undo = 3, operaciones_sin_before_image = 0` y DEJA EL STORE VACÍO. Como si la perdedora nunca hubiera existido. La pieza que el cap. 28 no sabía construir, vuelta y vuelta.

**TEST-FRONTERA** `borrado_robado_sin_before_image_se_reporta_no_se_calla`: un `DeleteNode(0)` robado SIN snapshot previo. El undo no encuentra `antes.get(0)` y CUENTA 1 en `sin_before_image`; el store queda con 0 nodos Y el informe dice `sin_before_image=1`. La honestidad del capítulo en un `assert`.

**TEST-PERSISTENCIA** `reabrir_recupera_lo_confirmado_tras_corte_de_luz`: una tx confirmada, `guardar_wal`, cierre del bloque (el «proceso muere»), `reabrir(&mut renacido, &path, &AntesImagenes::new())` devuelve `ganadoras = 1` y reconstruye 2 nodos + 1 arista con `labels == ["Person"]`. El flujo REAL de arranque verificado end-to-end.

Las **deudas del cap. 28**, una a una, verificadas:

- **El fichero** — `guardar_y_cargar_wal_roundtrip` (1161-1179): bytes idénticos antes y después de pasar por disco.
- **El checkpoint que persiste los contadores** — `checkpoint_y_truncar_seguro_no_reutiliza_lsns` (1112-1138): tras truncar, una nueva tx nace con `next_tx_id = 2` (no con 1 — los contadores persisten, no se reutilizan).
- **La rotación por tamaño** — `rotar_si_excede_trunca_solo_cuando_hace_falta` (1141-1156): por debajo del umbral no rota; justo al alcanzarlo (ya no `≤`) rota y trunca.
- **La contaminación detectada** — `redo_falla_ruidosamente_si_el_truncado_rompio_dependencias` (1231-1266): un truncado a mano que rompe el contrato (`truncar_hasta_lsn(3)` sobre un log que necesita el lsn 4) hace que la fase de redo falle con `RecoveryError::Redo { lsn: 5, ...InvalidEdgeEndpoints }` — el grito diagnóstico del cap. 28, heredado y tipado.

El síntoma si te saltas el capítulo es el de siempre: tras un crash con steal activo, escrituras de perdedoras sobreviven al reinicio; reabrir exige intervención manual; el log crece hasta OOM; truncarlo a ojo rompe Durabilidad silenciosamente; y no tienes un único punto de entrada que diga «le di el path, recuperé todo lo confirmado, aquí está el informe de qué no se pudo».

## 29.11 Repaso de la Parte VI: la cadena 27→28→29

La Parte VI tiene tres capítulos y un esqueleto común: cada uno ejecuta una pieza de ACID sobre el resto y deja una garantía heredada.

```
 27 ACID — TRANSACCIONES ──► 28 WAL ──────────► 29 RECUPERACIÓN (ARIES)
   Atomicidad               el cambio se         el arranque automático:
   Durabilidad RUDIMENTARIA  escribe en el WAL    • análisis reconstruye el mapa
   (un solo escritor,        antes que en la       • redo deja el store como
   staging → apply)          página de datos        en el instante del fallo
                             replay_wal a mano     • undo deshace las perdedoras
                             truncar a mano
   │                          │                    │
   └─ deuda: apply a medias   └─ deuda: steal      └─ deuda: before-image
      podría dejar store         rompe no-steal,     (sin imagen anterior
      inconsistente              perdedoras pueden    deshacer borrado robado
                                 dejar huella —       ⇒ ARIES completo: CLR)
                                 hace falta UNDO
```

Cada eslabón dejó una garantía heredada: el **27** fijó `Operacion` (la pieza que viaja por las tres fases sin duplicarse) y el staging+apply-after-commit; el **28** descubrió que `sync()` no es un contador sino un protocolo, y construyó `replay_wal`/`truncar_hasta_lsn` con contrato firmado; el **29** reúne: hereda `aplicar_para_redo` idempotente, `Wal::iter` con parada limpia, `Operacion` del 27 — y añade lo que ninguno tenía: la decisión explícita de CÓMO se reconstruye un motor tras un crash (las tres fases de ARIES), la pieza administrativa que automatiza el truncado (Checkpoint con contadores), y la honestidad de DECIR lo que no se pudo hacer (`operaciones_sin_before_image`). El método de la Parte VI en una frase: **la transacción es la promesa; el WAL es el cuaderno donde se anota; la recuperación es quien lee el cuaderno al volver del corte de luz y decide quién sigue y quién se queda en el suelo**.

Estas dos piernas — el `recuperar(store, wal, antes)` para un único motor y el `reabrir(store, path, antes)` para un motor con fichero— son también las que sostendrán el futuro inmediato del libro: el **cap. 30** añadirá concurrencia real (MVCC: varios lectores leyendo MIENTRAS un escritor escribe, sin lecturas sucias); el **cap. 36/37** añadirá el checkpoint del STORE DE DATOS (la pieza que falta para la Durabilidad completa).

## 29.12 Qué hemos sacrificado

1. **Sólo un escritor (`&mut dyn GraphStore`)**: la recuperación exige el préstamo exclusivo del store, igual que los caps. 27-28. Un cliente intentando leer MIENTRAS el recovery reescribe el store debe esperar; la concurrencia real es cap. 30. Documentado, no implementado.
2. **Sin CLR ni log de before-image (ARIES completo)**: la frontera del `DeleteNode`/`DeleteEdge` robado sin snapshot sigue ahí — `operaciones_sin_before_image` la cuenta. ARIES completo añade registros CLR y log de before-images para cerrar el hueco; aquí lo dejamos documentado como la pieza que el siguiente nivel añade. La razón: añadirla exige mover el undo a una fase con escri-tura adicional al log, que es materia del cap. 36 cuando se introduce el ciclo undo→redo→undo→page flush.
3. **Fuzzy checkpoint NO implementado (ARIES completo)**: el checkpoint aquí es EXACTO (congela el estado actual). Un motor real usa fuzzy checkpoint (la página-sucia-tabla periódica) para no pagar el «todo se para mientras se hace checkpoint» del fuzzy→exact. Lo nombramos y lo dejamos al lector que mire Mohan 1999 («Repeating History Beyond ARIES»).
4. **El log es append-only sin CRC de bloque**: el frame por registro del cap. 28 ya lleva el CRC32 (`crc32_simple`), pero si UN bloque de disco se corrompe a MITAD de registro, la iteración con parada limpia del cap. 28 CORTA antes; la tx confirmada con el byte corrupto se considera perdedora. Esto está bien para un WAL append-only, no lo está para un sistema que quiera recovery con bit-error-resistance — fuera de alcance.
5. **Group commit y `fsync` agrupado**: la política `SoloCommit` del cap. 28 ya está aquí (un fsync por tx), pero el GROUP COMMIT REAL (varias tx compartiendo un fsync) exige concurrencia — cap. 30.
6. **Recuperación distribuida (dos PCs, Paxos)**: nombrada y cortada. La pregunta «qué pasa si el recovery mismo se cae» la responden los algoritmos de consenso — fuera del alcance de este libro.

## 29.13 Cómo lo hace una BBDD real

- **PostgreSQL**: ARIES con las tres fases exactas (`XLOG` records con LSN, `xact` table, dirty page table en `pg_stat_get_db_*`, checkpoint en `pg_control` con `nextXid`). La pieza que el cap. 29 deja como `operaciones_sin_before_image` la cierra con `xl_invalid_page`/`FULL_PAGE_WRITES`: cada escritura al store lleva un backup completo de la página, lo que ARIES llama *before-image*. Modo completo, coste en disco: cada página modificada se escribe DOS veces (al log, a disco).
- **InnoDB (MySQL)**: ARIES sobre redo log (las escrituras previas a la página: `LOG_BLOCK_HDR_NO`, `LOG_CHECKPOINT` con `LOG_DYNHDR_CPN_NO` para el último checkpoint) y undo log (segmentos por tx; `TRX_UNDO_PAGE` con el estado anterior). La frontera del cap. 29 la cruzan con **undo log de verdad**: cada `DeleteNode` lleva su `update undo log` con la fila completa. Lo que aquí es «el log no la tiene, el informe la cuenta», allí es «el log SÍ la tiene, y deshacer restaura» — la pieza ARIES completa del cap. 29.
- **Oracle**: ARIES con `REDO LOG`/`UNDO TABLESPACE` y «flashback» (queries «AS OF» sobre undo log). Las tres fases están; lo que cambia es la granularidad (undo por segmento, no por registro).
- **SQL Server**: ARIES bajo el nombre «ARIES-style recovery» con `LSN` por byte, `recovery interval`, `fuzzy checkpoint` y la `version store` para versiones de fila (cercano a MVCC pero distinto).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: sobre el log `Begin(1) Op(2) DeleteNode(0) Commit(3)`, ¿cuántos `operaciones_deshechas` cuenta el undo? ¿Y si lo ejecutas dos veces seguidas (idempotencia)? Pista: la «Op» toca el elemento 0 con `DeleteNode`, una CONFIRMADA — no es perdedora, no se deshace.
- *Intermedio*: tu base tiene el log `{Tx=1: PutNode(0), PutNode(1), PutEdge(0,0,1)}` y el store YA tiene esas escrituras (steal). La tx 1 está SIN Commit (proceso muere). ¿Qué predice el undo si capturas `antes` DESPUÉS del robo? Pista: la imagen que capturas es la de después, no la de antes — `operaciones_sin_before_image` se dispara.
- *Experto*: implementa `recuperar_sin_truncar(wal)` (análisis + redo + undo sin habilitar el truncado del log). ¿Qué tiene que devolver para que un test verifique que `wal.record_count()` post-recovery es el MISMO que pre-crash? Pista: el log no se toca en ninguna de las tres fases; el truncado es opt-in.

## 29.14 Lo que te llevas

- **ARIES en tres fases**: análisis reconstruye el mapa, redo deja el store como en el fallo, undo deshace las perdedoras en orden inverso. El esqueleto de Mohan et al. 1992, treinta y cuatro años después.
- **El REDO no es «rehacer lo bueno»**: es «rehacer TODO hasta el estado EXACTO del fallo, incluidas las escrituras robadas de perdedoras» — la base sobre la que el undo puede operar.
- **La compensación es lógica e idempotente**: un `PutNode` robado se deshace con un `delete_node` idempotente; un `DeleteNode` robado exige la imagen anterior que un log de solo after-image no tiene.
- **`operaciones_sin_before_image` es información, no error**: el motor HONESTO reporta lo que no pudo hacer; no aborta por defecto y tampoco maquilla el resultado.
- **El checkpoint persiste los CONTADORES, no sólo el LSN**: truncar a vacío sin guardar `next_tx_id` los reutiliza — la diferencia entre recovery y corruptor silencioso.
- **El fichero del WAL** convierte el `sync` (contador) en `guardar_wal` (bytes en disco) — y `cargar_wal` reconstruye los contadores del prefijo ESCANEANDO, no manteniéndolos.
- **`recuperar` y `reabrir`**: uno opera sobre un `Wal` ya cargado, el otro lee del fichero y reconstruye. El flujo REAL de arranque en una sola llamada.
- **`informe_acid_post_recovery` continúa la honestidad**: A pasa de cierre 29 a 30 (queda before-image), D de 29 a 37 (queda checkpoint del store). El sistema de tipos lleva la trazabilidad.

## 29.15 Ojo, cuidado con…

- **Confundir `replay_wal` del cap. 28 con `recuperar` del cap. 29**. El replay sólo rehace ganadoras (política no-steal); el recuperar ejecuta las TRES fases. Si una base de datos con steal activo «se recupera» con `replay_wal`, las perdedoras sobreviven al reinicio — inconsistencia lógica.
- **Pensar que el undo es simétrico al redo**. No: `PutNode`/`PutEdge` se deshacen con un borrado idempotente (el «antes» es trivial); `DeleteNode`/`DeleteEdge` exigen una imagen anterior que el log no guarda. La frontera es asimétrica, no universal.
- **«Checkpoint = foto del store»**. Falso aquí. Es un REGISTRO del log («todo lo anterior a este LSN es durable Y los contadores son estos») y su segunda parte — los contadores — es la sorpresa administrativa que truncar a vacío sin ella paga caro.
- **Tomar el snapshot del store POST-ROBO**. `capturar_antes` sobre un store que ya tiene el borrado captura la imagen de DESPUÉS, no la de ANTES. En un sistema real el snapshot es pre-tx (o viene de un log de before-images).
- **`operaciones_sin_before_image` como bug**. Es información. El motor decidió reportar y seguir; la decisión de abortar es del operador.
- **Confundir `Rollback` (cap. 27) con `deshacer` (cap. 29)**. El primero escribe el marker y vacía el staging de UNA tx en vivo; el segundo recorre el log al revés y compensa a las perdedoras tras un crash. Distintas.

## 29.16 Pin de batalla

> *«Un log que no cuenta lo que no supo deshacer no es recuperable — es un cuaderno elegante que miente cuando lo lees a las tres de la mañana.»*

## 29.17 Si solo lees 30 segundos

ARIES tiene tres fases: **análisis** reconstruye del log la tabla de transacciones, los contadores y la dirty element table; **redo** re-aplica TODO en orden de LSN (ganadoras Y perdedoras) con `aplicar_para_redo` idempotente — el store queda como en el instante del fallo; **undo** recorre al revés y deshace las perdedoras, `PutNode`→`delete_node` idempotente, `DeleteNode`→restaurar imagen anterior (si la hay; si no, CONTAR, no inventar). El **fichero del WAL** persiste los bytes y `cargar_wal` reconstruye los contadores ESCANEANDO; **`Checkpoint`** congela durable Y contadores — sin `next_tx_id` persistido, truncar a vacío REUTILIZA identificadores. `recuperar` opera sobre un `Wal`; `reabrir` lee del fichero. **ACID post**: A 29→30 (queda before-image), D 29→37 (queda checkpoint del store).

## 29.18 Una historia pequeña

La migración de este capítulo casi se pierde en el camino, como cuenta MIGRATION-PATTERN §34. Los tests aparecían en verde, pero dos fallaban en `next_lsn` y `operaciones_redo`. ¿Quién mentía, el test o el código? Resultó que el test ASUMÍA que `WalTransaccion::rollback` logueaba las operaciones (luego undo las compensaría); pero rollback sólo escribe el marker `Rollback` — el staging del cap. 27 nunca llegó al log. La distinción «staging vs log» del cap. 27, que parecía un detalle, salvó la calibración: un rollback deja `Begin+Rollback` SIN operaciones; el redo de una tx rollbackeada es vacío. La moraleja quedó escrita en la bitácora: **los tests que comparan con la realidad se calibran recorriendo el código, no con lo que suena razonable** — el mismo `assert_eq!` del cap. 26, ahora en WAL/Recovery.

Y un detalle de calidad que se cazó al integrar: el `RecoveryError::Redo { lsn, causa }` movía `causa` al hacer `match err { Redo { causa, .. } }` y luego `err.to_string()` fallaba por uso de valor movido (E0382). El fix fue comprobar el `Display` ANTES del match por valor — la clase de bug que el compilador te cuenta si lo escuchas.

## Ejercicios resueltos

**1. ¿Por qué el REDO re-aplica también las perdedoras, si luego el UNDO las va a borrar?**

Porque el undo necesita una base coherente sobre la que operar. Si el redo sólo aplicara ganadoras, el store post-redo tendría MENOS datos de los que tenía en el corte (las robadas faltarían), y el undo no encontraría dónde borrar — quedaría incoherente. Re-aplicar lo ya aplicado es un no-op por la idempotencia de `aplicar_para_redo`; re-aplicar lo robado y luego DESHACERLO es lo que cierra el ciclo. Es la diferencia entre un replay optimista (cap. 28, no-steal: re-aplicar los confirmados es suficiente) y un replay GENERAL (cap. 29, steal: re-aplicar todo y retroceder las perdedoras).

**2. ¿Por qué `truncar_seguro` exige un `Checkpoint` con `next_lsn` Y `next_tx_id`, no sólo el `hasta_lsn`?**

Porque tras truncar el log a vacío, `Wal::reconstruir` lo reanimará ESCANEANDO los bytes. Si el log está vacío, los contadores se ponen a 1 por defecto — y la siguiente transacción nacería con id 1, REUTILIZANDO un identificador histórico. Un log recuperable no puede permitir eso: dos tx con el mismo id hacen que el análisis confunda cuál era cuál y el undo descompense a la que NO debía. `Checkpoint::tomar` congela ambos contadores con su valor real; `truncar_seguro` los respeta (al reconstruir de un log vacío se usará un valor por defecto DOCUMENTADO, y `cargar_wal` testea ese caso). Es un detalle que parece administrativo y es la diferencia entre recuperar y corromper.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin mirar el código, enuncia las tres fases de ARIES y para qué sirve cada una. Sobre el log `[Begin(1), Op(2 PutNode(0)), Commit(3), Begin(4), Op(5 PutNode(1))]` (la segunda tx abandonada), predice A MANO `analisis.ganadoras`, `analisis.perdedoras`, `analisis.next_lsn`, `analisis.next_tx_id`, `analisis.sucias`. Verificación con `analizar(&wal)` — debe coincidir. *Pistas*: (1) `next_lsn = max(lsn) + 1` no `max(lsn)`; (2) la tx abandonada entra Activa en el primer registro; (3) la dirty table guarda el PRIMER LSN de cada elemento. *Criterio*: predicción idéntica a `analisis_reconstruye_tabla_de_transacciones_y_contadores` (873-905).

**Intermedio (analizar — spacing caps. 27 y 28).** Sobre la cadena de 12 con sólo inserciones en una sola tx (12 nodos + 11 aristas en orden): predice cuántas operaciones aplicaría la fase de redo y si el undo sería vacío. Da el caso de UNA operación de borrado confirmada (paso 11, arista 10→11) y predice la diferencia. Explica por qué el replay del cap. 28 difiere del redo de ARIES en qué (la pregunta central). Verificación con `red_recorre_todas_las_operaciones_con_contador` (1081-1107): el voltímetro debe cuadrar con la predicción. *Pistas*: (1) `aplicar_para_redo` idempotente cuenta cada operación UNA vez, no varias; (2) el undo sólo deshace perdedoras — una CONFIRMADA no entra; (3) la tx abandonada (sin Commit) deja todas sus operaciones en `operaciones_deshechas`. *Criterio*: distingue store post-crash con steal (las perdedoras están ahí, undo las borra) de store post-crash sin steal (no están, undo vacío).

**Experto (crear — bridge retrieval al cap. 28).** Reconstruye desde la memoria el flujo completo «store se cae → DBA lo reabre → todo lo confirmado vuelve, lo perdedor no», citando en orden las llamadas (`reabrir` → interno `cargar_wal` → `Wal::reconstruir` → `recuperar` → interno `analizar` + `redo` + `deshacer`), diciendo para cada una de qué pieza del cap. 28 viene el material (`Wal::iter` con parada limpia, `aplicar_para_redo`, `truncar_hasta_lsn`, `informe_acid_post_wal`) y qué pieza NUEVA añade el cap. 29. Implementa `recuperar_sin_truncar(wal)` que ejecute sólo análisis + redo + undo sin habilitar el truncado, y un test que la use para verificar que `wal.record_count()` post-recovery sigue siendo el MISMO que pre-crash (la operación no toca el log — separación clara). *Pistas*: (1) qué firmas del cap. 28 reutilizas sin tocar (la mayoría) y cuál automatizas (`truncar_hasta_lsn` → `truncar_seguro(wal, cp)`); (2) `Wal::next_tx_id()` se hizo público para este capítulo — úsalo; (3) la re-valoración ACID dispara A: 29→30 y D: 29→37. *Criterio*: citación correcta de TODOS los puentes al cap. 28 + la nueva función compila y su test pasa + la re-valoración coincide con `informe_post_recovery_actualiza_a_y_d` (1287-1310).

## Para profundizar

- **C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh, P. Schwarz, «ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollback Using Write-Ahead Logging»**, ACM Transactions on Database Systems 17(1), marzo 1992, pp. 94-162 (DOI 10.1145/128765.128770) — el paper original, cuarenta páginas de algoritmo. Las tres fases, la dirty page table, los CLR, el fuzzy checkpoint, todo escrito en una prosa densa que sobrevive.
- **C. Mohan, «Repeating History Beyond ARIES»**, en «VLDB 1999 / ICDE 1999» — la evolución: LRU-K, fuzzy checkpoint, registros redo-only/undo-only, modificaciones posteriores. La misma arquitectura con piezas más finas.
- **T. Haerder, A. Reuter, «Principles of Transaction-Oriented Database Recovery»**, ACM Computing Surveys 15(4), 1983, pp. 287-317 (DOI 10.1145/322290.322291) — el análisis de las cuatro políticas (steal/no-steal × force/no-force) que el cap. 29 recoge indirectamente. Anterior a ARIES; el cuadro mental de «qué sacrifica cada política» viene de aquí.
- **R. Ramakrishnan, J. Gehrke, «Database Management Systems» (3.ª ed., McGraw-Hill 2003)**, capítulo 18 — la presentación académica estándar de recovery con WAL y ARIES simplificado; el libro de texto del que sale el capítulo conceptual.
- **A. Silberschatz, H. F. Korth, S. Sudarshan, «Database System Concepts» (7.ª ed., McGraw-Hill 2020)**, capítulo 17 — el complemento: cover recovery con buffer management detallado.
- **PostgreSQL internals: `xlog.c`, `xact.c`, `pg_control`** — la implementación ARIES de PostgreSQL leyendo el código; los nombres difieren (`XLogRecord` por `WalRecord`), el esqueleto no.
- **InnoDB internals: `log0log.cc`, `trx0undo.cc`** — la otra implementación ARIES; undo log con before-image completo, lo que aquí es hueco documentado.

## Mini-diálogo: en guardia nocturna

> — O sea, ¿recuperar es replay más algo?
>
> — No. Replay del cap. 28 SOLO rehace las confirmadas. Si un buffer pool real evacua páginas sucias de una tx no confirmada para hacer hueco — eso es el *steal* — esas escrituras SE QUEDAN en el store. Si sólo rehaces las confirmadas, las robadas sobreviven al reinicio: la base «recuerda» cosas que nunca debió.
>
> — Y recovery las borra.
>
> — Recovery las borra, sí — pero solo después de RE-APLICARLAS. Es la pieza que suena rara: el redo deja el store EXACTAMENTE como estaba en el corte (incluidos los robos), y el undo retrocede desde ahí hasta las ganadoras. Sin redo completo, el undo no tiene base. Las dos fases son SECUENCIALES sobre el mismo registro de escrituras.
>
> — ¿Y si el undo no encuentra la imagen anterior de un borrado robado?
>
> — El capítulo lo CUENTA, no lo inventa. `operaciones_sin_before_image` lo reporta, la decisión de continuar o abortar es del operador. ARIES completo cierra el hueco con CLR y before-images; aquí lo nombramos y lo dejamos al siguiente nivel. La honestidad es lo que mantiene el log confiable cuando lo lees a las tres de la mañana.

---

*(Próximo capítulo: 30 — Snapshots, concurrencia y aislamiento. Recovery es un único escritor con `&mut dyn GraphStore` — ¿qué pasa si dos transacciones quieren recuperarse a la vez, o si un cliente está leyendo mientras el recovery reescribe el store? La MVCC y el group commit cierran la Parte VI — y en el Vol.III, el cap. 51 montará GraphRAG sobre las piernas de la Parte V con la durabilidad del WAL+recovery que acabamos de construir.)*
# Capítulo 30 — Snapshots y concurrencia: MVCC limitado

> *«El lector no necesita ver lo último. Necesita ver lo que decidió ver cuando empezó a mirar.»*

## 30.0 La anécdota de la esquina

En 1978, David P. Reed presentó en el MIT una tesis doctoral titulada «Naming and Synchronization in a Decentralized Computer System». Era un trabajo sobre cómo coordinar nodos independientes en una red que no compartía reloj ni disco, y la pieza clave de su propuesta — el capítulo 4 — proponía algo que parecía obvio pero que nadie había formulado así: **en lugar de sobrescribir un dato cuando cambia, mantener las versiones anteriores y dejar que cada observador elija cuál ve**. La idea era tan simple que, durante años, los sistemas distribuidos la reinventaban una y otra vez sin saber que ya estaba escrita.

Cuarenta años después, esa idea — versiones múltiples, una por cada «momento lógico» — es el corazón de casi todas las bases de datos modernas. Se llama MVCC (Multi-Version Concurrency Control), y es lo que vamos a construir en este capítulo: la maquinaria que permite a LiraDB hacer lo que el cap. 27 no pudo — tener varios lectores leyendo MIENTRAS un escritor escribe, sin lecturas sucias, sin actualizaciones perdidas. La diferencia entre el «único escritor por el borrow checker» del cap. 27 y la «MVCC limitada» del cap. 30 no es un matiz: es el momento en que LiraDB deja de ser un sistema de un solo hilo lógico y empieza a hablar el idioma de las bases de datos reales.

Lo que NO arreglaremos aquí es igual de importante que lo que sí: en Snapshot Isolation, dos transacciones que leen y modifican elementos DISJUNTOS a partir del mismo snapshot pueden producir un resultado no serializable — la anomalía que la literatura llama write skew. Cerrar eso exige Serializable Snapshot Isolation con predicate locks (Cahill, Fekete, Liarokapis y Bernstein, «Serializable Snapshot Isolation in PostgreSQL», VLDB 2008), y eso queda para la Parte VIII. La honestidad de este capítulo es justamente ésa: promete lo que cumple y avisa de lo que no.

## 30.1 Objetivo

Al terminar este capítulo vas a entender por qué el modelo del cap. 27 — un único escritor por el borrow checker — es una **limitación benigna**, no una característica: simplifica el código y por construcción prohíbe las anomalías de aislamiento, pero deja sin resolver el caso del lector que quiere ver un estado coherente mientras otro escribe. Vas a construir la pieza que lo resuelve: una capa MVCC sobre el `MemoryStore` del cap. 8 que entrega snapshots coherentes sin bloquear al escritor, y vas a ver la frontera honesta donde la instantánea deja de bastar (write skew).

En concreto, vas a construir seis piezas:

1. `VersionNode` / `VersionEdge` — el registro de versión con su `ts_begin` y `ts_end`.
2. `MvccStore` — la capa MVCC sobre `MemoryStore` (la hexagonal del cap. 8).
3. `commit` con un solo `ts` por lote y validación propia (`validar_mvcc`).
4. `gc(hasta)` — la pieza de recuperación del espacio de versiones.
5. `NivelAislamiento` como vocabulario (lo que PROHÍBE y lo que DEJA PASAR cada nivel).
6. `GrafoEspera` para deadlocks — anzuelo para caps. futuros, no código en uso.

## 30.2 Problema

Volvamos al cap. 27, un momento. Teníamos una `Transaccion` que tomaba `&mut dyn GraphStore` durante toda su vida — begin, stage, commit, drop. El borrow checker era el cerrojo: mientras una transacción vivía, NINGÚN otro escritor y NINGÚN lector podía tocar el store. Eso significaba que las anomalías clásicas — `Anomalia::LecturaSucia` (una tx ve lo que otra NO ha confirmado), `Anomalia::ActualizacionPerdida` (dos tx escriben el mismo elemento y una pisa a la otra) — se definían como vocabulario pero NO PODÍAN ocurrir. El aislamiento era perfecto por construcción: no había concurrencia que aislar.

El problema es que ése no es el aislamiento que promete una base de datos. Una base de datos REAL debe permitir que un lector recorra el grafo MIENTRAS otro confirma cambios — y que el lector vea un estado coherente del momento en que empezó a mirar, no los cambios que están ocurriendo AHORA. El cap. 27 nos dio las palabras (`Anomalia`, `NivelAislamiento`); el cap. 28 nos dio la durabilidad; el cap. 29 nos dio la recuperación. Lo que nos falta es la pieza que une todo: la forma de tener **varios lectores concurrentes con un escritor**, sin lecturas sucias, sin actualizaciones perdidas.

La raíz del problema es la misma de siempre en sistemas concurrentes: cuando dos operaciones «leen y escriben» al mismo tiempo, hay tres opciones:

1. **Bloquear al lector mientras el escritor escribe**: garantiza consistencia, pero mata el paralelismo — el lector espera al escritor y la base de datos parece de un solo hilo.
2. **Bloquear al escritor mientras el lector lee**: mismo problema al revés — el escritor espera al lector, y dos transacciones que tardan en leer bloquean el sistema entero.
3. **Hacer que lean COSAS DISTINTAS**: el lector y el escritor no pisan la misma versión. El lector lee la versión que existía cuando empezó a mirar; el escritor crea una versión nueva que verá el siguiente lector.

La opción 3 es MVCC. Y es la única que escala.

## 30.3 Modelo mental

Vamos a usar dos analogías juntas, porque juntas lo ordenan todo:

**El fotógrafo con contador de exposición.** Imagina que cada lector es un fotógrafo con una cámara antigua. Cuando toma una foto, anota un número en el borde del negativo — su `ts`, su «número de exposición». Durante el revelado (su transacción) sólo ve lo que estaba en el visor EN ESE instante. El fotógrafo puede disparar N fotos simultáneas — cada una con su propio número — y cada revelado ve SU momento, no el de los demás. El escritor, mientras tanto, hace clic en su obturador (commit) y CAMBIA el visor; pero el revelado anterior ya está en su cubeta, terminado, y el nuevo visor no lo toca. Las fotos viejas que nadie quiere revelar se tiran a la basura — eso es `gc`.

**El editor de versiones de un documento** (tipo Git o Google Docs, pero a nuestra escala). Un `PutNode` NO pisa el `Node` anterior: RETIRA su versión actual (le pone `ts_end`) y APPENDIZA una nueva al final de la cadena. La historia del documento está disponible para quien sepa qué commit le interesa (`leer_nodo(id, ts)`). Un `DeleteNode` RETIRA la versión actual sin appendizar — la AUSENCIA es el nuevo estado: el documento ya no existe para los snapshots futuros, pero los antiguos lo siguen viendo.

Las dos analogías se necesitan mutuamente: el fotógrafo explica la CONSISTENCIA del snapshot («mi foto no cambia porque alguien más disparó»), el editor de versiones explica la IMPLEMENTACIÓN («cada elemento lleva una cadena de versiones»).

```
Nodo 0 «Ana»:
+-----------+-----------+--------+
| ts_begin=1| ts_begin=4| ts_begin=7|
| ts_end=4  | ts_end=7  | ts_end=None|
| «Ana»     | «Ana S.»  | «Ana S.»  |
+-----------+-----------+--------+
   ▲ ts=2 ve «Ana»               ▲ ts=8 ve «Ana S.»; ts=5 ve «Ana S.»
```

Cada cuadro es una `VersionNode`: tres campos — cuándo empezó a ser visible (`ts_begin`), cuándo dejó de serlo (`ts_end`), y qué contenía (`nodo`). La cadena está ordenada por `ts_begin` ASCENDENTE; la última entrada (la del final) es la versión actual — su `ts_end` es `None` hasta que otra la retire. Para encontrar la versión visible en un instante `ts`, recorremos la cadena del final al principio y devolvemos la primera versión con `ts_begin ≤ ts` y (`ts_end > ts` o `ts_end = None`).

## 30.4 Primera solución

La solución ingenua — la que escribiría un novato — es exactamente lo que parece: un `HashMap<NodeId, Node>` con un cerrojo de lectura (`RwLock`). El lector toma `read_lock()`, lee lo que hay, suelta el cerrojo. El escritor toma `write_lock()`, reescribe el nodo, suelta el cerrojo.

Funciona. Los tests pasan. Y tiene un problema que sólo se ve cuando se mide:

1. **El lector bloquea al escritor durante su recorrido.** Si un lector pide `iter_nodos()` y tarda 10 ms en consumirlos, los escritores que lleguen durante esos 10 ms ESPERAN. En una base de datos con analíticas largas, eso es mortal.
2. **El escritor bloquea a los lectores.** Si un escritor tarda 5 ms en confirmar 1.000 escrituras, los lectores que lleguen durante esos 5 ms ESPERAN. Es el mismo problema al revés.
3. **No hay garantía de coherencia DURANTE el recorrido del lector.** Si el lector empieza en `ts=1` y el escritor confirma un cambio en `ts=2` mientras el lector está en la mitad del recorrido, el lector puede ver una mezcla de los dos estados — el `iter_nodos()` del cap. 8 NO toma snapshot, recorre el HashMap EN EL MOMENTO. La anomalía `LecturaSucia` se materializa sin que nadie la pidiera.

Hay otra solución más sutil pero igual de ingenua: **bloquear a nivel de elemento**. Cada nodo y cada arista tiene su propio `Mutex`. El lector pide el lock del nodo X, lo lee, lo suelta. El escritor pide los locks de los nodos que toca, los modifica, los suelta. Es lo que llaman «cerrojos de granularidad fina».

Funciona mejor, pero abre una puerta nueva: los **deadlocks**. Si la tx A tiene el lock del nodo X y espera el del Y, y la tx B tiene el del Y y espera el del X, las dos se quedan bloqueadas para siempre. Solucionarlo exige el grafo de espera y la detección de ciclos — la pieza del cap. 30 que vamos a construir como anzuelo, no como código en uso.

Ninguna de las dos soluciones ataca el problema real: **el lector quiere ver un estado coherente, no el estado actual en cada instante**. Y para eso necesitamos algo cualitativamente distinto.

## 30.5 Sus límites

Las dos soluciones ingenuas comparten un límite conceptual: tratan la lectura como un «accidente físico» — un observador que mira en un instante y se va. Pero una base de datos NO funciona así. Una analítica que pregunta «¿cuántos nodos hay en el subgrafo X?» necesita ver un estado COHERENTE durante toda su ejecución, no los cambios que ocurren a media consulta.

Lo que necesitamos no es un cerrojo más fino ni más rápido. Necesitamos que el lector **tome una foto del grafo en el instante en que empieza a mirar**, y que esa foto no cambie mientras la trabaja. Eso es un snapshot. Y el snapshot, en MVCC, se materializa como un **número lógico** — el `Ts` — que el lector pasa a cada `leer_nodo(id, ts)`. La foto no es una copia de los datos: es un INSTANTE LÓGICO al que cada elemento responde con la versión que le correspondía.

Esta idea resuelve los tres problemas de un golpe:

1. **El lector no bloquea al escritor:** el lector clona la versión visible al `ts` y trabaja con su copia. El escritor modifica la versión actual (que el lector NO está mirando).
2. **El escritor no bloquea al lector:** la modificación crea una versión NUEVA con un `ts` mayor; el lector, con su `ts` viejo, sigue viendo la versión anterior.
3. **La coherencia del snapshot es por construcción:** la cadena es append-only (las versiones nuevas se añaden al final), la lectura es por valor (clona), y no hay un instante en que el grafo «cambie» a mitad del recorrido.

Y aquí viene la pieza clave que el cap. 27 no podía enunciar: **los lectores toman `&self` y el escritor toma `&mut self`, y AMBOS conviven sobre el mismo `MvccStore`**. El borrow checker no se queja: `&self` y `&mut self` son incompatibles para el MISMO dato, pero la MVCC los separa — los lectores leen cadenas (inmutables), el escritor modifica el `inner` y APPENDIZA a las cadenas. Es el patrón que convierte el «único escritor del cap. 27» en «N lectores concurrentes con un escritor».

## 30.6 Solución evolucionada

La solución evolucionada se reduce a tres reglas:

1. **Cada elemento lleva una CADENA de versiones (`ts_begin`, `ts_end?`, `valor`).** Las escrituras RETIRAN la versión actual (ponen su `ts_end`) y APPENDIZAN una nueva. Los deletes RETIRAN sin appendizar (la ausencia es el estado).
2. **Un snapshot es un `Ts = u64` monótono.** Una lectura en `ts` ve la versión con el MAYOR `ts_begin ≤ ts` Y `ts_end > ts` (o sin `ts_end`).
3. **El escritor es único (`&mut MvccStore`); los lectores son concurrentes (`&MvccStore`).** Sin cerrojos de lectura: la consistencia viene del versionado, no de los locks.

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap30_mvcc.rs`. Vamos a leerlo por partes, porque cada línea tiene un porqué.

### Las versiones

```rust
pub type Ts = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct VersionNode {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub nodo: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionEdge {
    pub ts_begin: Ts,
    pub ts_end: Option<Ts>,
    pub arista: Edge,
}
```

`Ts` no es un timestamp físico (`SystemTime::now()`, ni `Instant::now()`). Es un **contador lógico**: el ORDEN de las escrituras. ¿Por qué no tiempo real? Porque dos commits no pueden coincidir en el orden del programa (uno va antes que el otro en el hilo del escritor); un reloj de tiempo real mezcla «orden» con «cuándo pasó» y abre problemas de deriva entre máquinas — la Parte VIII los cierra con vector clocks o true time, fuera del alcance del cap. 30. El contador es barato, monótono y portable: dos `Ts` son comparables sin ambigüedad.

`VersionNode` lleva tres campos. `ts_begin` es obligatorio: toda versión EMPEZÓ a ser visible en algún momento. `ts_end` es opcional: si es `None`, la versión SIGUE vigente; cuando otra la retire, se le pone `Some(ts)`. El campo `nodo` es el valor — clonado de la `Node` del cap. 7.

### El store MVCC

```rust
pub struct MvccStore {
    pub inner: MemoryStore,
    pub versiones_nodos: HashMap<NodeId, Vec<VersionNode>>,
    pub versiones_aristas: HashMap<EdgeId, Vec<VersionEdge>>,
    pub reloj: Ts,
}
```

Tres campos importantes. `inner: MemoryStore` es el **espejo material** — la versión «del momento presente» que las queries que NO piden snapshot (el código del cap. 8) usan. La MVCC vive ENCIMA; el `inner` es la verdad material para quien no sabe de versiones. `versiones_nodos` y `versiones_aristas` son los mapas de cadenas, una por cada elemento que haya pasado por el sistema. `reloj: Ts` es el contador: el siguiente `ts` a asignar.

La estructura del `MvccStore` es la hexagonal del cap. 8 probada una vez más: el `inner` es un `MemoryStore` CONCRETO hoy, pero cualquier backend que cumpla `GraphStore` serviría — un `FilePager`+CSR del cap. 14, un backend distribuido del cap. 40. El versionado no cambia; el backend sí. Es exactamente la inversión de dependencias que los caps. 8 y 26 establecieron.

### El commit con un solo timestamp

```rust
pub fn commit(&mut self, ops: &[Operacion]) -> Result<ResumenCommitMvcc, MvccError> {
    self.validar_mvcc(ops)?;
    let ts = self.siguiente_ts();
    let mut resumen = ResumenCommitMvcc {
        ts_asignado: ts,
        ..ResumenCommitMvcc::default()
    };
    for op in ops {
        match op {
            Operacion::PutNode(n) => {
                let chain = self.versiones_nodos.entry(n.id).or_default();
                if let Some(last) = chain.last_mut()
                    && last.ts_end.is_none()
                {
                    last.ts_end = Some(ts);
                    resumen.versiones_retiradas += 1;
                }
                chain.push(VersionNode {
                    ts_begin: ts, ts_end: None, nodo: n.clone(),
                });
                let _ = self.inner.delete_node(n.id);
                self.inner.put_node(n.clone()).map_err(MvccError::Store)?;
                resumen.nodos_escritos += 1;
            }
            // ... PutEdge, DeleteNode, DeleteEdge análogos
        }
    }
    Ok(resumen)
}
```

Tres decisiones que merecen explicarse:

**Un solo `ts` por lote.** La asignación se hace UNA vez, antes del bucle. ¿Por qué? Asignar UN `ts` por commit da una vista atómica: para cualquier `ts`, todos los elementos que esa transacción tocó son visibles en su nueva versión O todos en la vieja. Asignar uno por operación abriría una ventana en la que un lector ve la mitad del commit — un snapshot «mezclado». Volvería la `Anomalia::LecturaSucia` dentro de un commit.

**Validación PROPIA, no la del cap. 27.** La `validar_buffer` del cap. 27 asume INSERCIÓN ESTRICTA: rechaza `PutNode` de un id existente con `StoreError::DuplicateNode`. En MVCC, SOBREESCRIBIR es LEGAL — es lo que crea una nueva versión. Por eso el cap. 30 implementa `validar_mvcc` propia, que registra `PutNode` como «sim_creados_nodos.insert(n.id)» (no como error) y verifica `PutEdge` contra el estado visible (inner + simulación del buffer). Es la pieza que la calibración del módulo descubrió: 6 tests fallaban con `Validacion(DuplicateNode)` hasta que se separaron las dos políticas (MIGRATION §35).

**`delete-then-put` en el `inner`.** El `MemoryStore` del cap. 8 es de INSERCIÓN ESTRICTA. MVCC SOBREESCRIBE legalmente, así que el `inner` debe aceptar la nueva versión: `delete_node` (que es silencioso si no existe) seguido de `put_node`. La CADENA ya hizo su trabajo de versionado; el `inner` sólo es el espejo material.

### Las lecturas por valor

```rust
pub fn leer_nodo(&self, id: NodeId, ts: Ts) -> Option<Node> {
    let chain = self.versiones_nodos.get(&id)?;
    version_visible_node(chain, ts).map(|v| v.nodo.clone())
}

fn version_visible_node(chain: &[VersionNode], ts: Ts) -> Option<&VersionNode> {
    chain.iter().rev()
        .find(|v| v.ts_begin <= ts && v.ts_end.is_none_or(|t| t > ts))
}
```

La pieza fundamental. `leer_nodo` toma `&self` (no `&mut self`), busca la cadena del elemento y devuelve la versión visible al `ts` — clonada. La búsqueda recorre la cadena del final al principio (la versión actual está al final — es donde están los cambios más recientes) y devuelve la primera versión con `ts_begin ≤ ts` y (`ts_end > ts` o `ts_end = None`).

Que sea `&self` es la pieza clave. Permite que el lector clone la versión y se vaya, sin pedir nada al escritor. El escritor, mientras tanto, tiene `&mut self` y modifica el `inner` y APPENDIZA a las cadenas. El borrow checker admite `&self` y `&mut self` SIMULTÁNEOS mientras las operaciones que cada uno hace sean disjuntas — y la MVCC las hace disjuntas por construcción: el lector no toca el `inner` ni el `último elemento` de la cadena (lee una versión histórica cualquiera); el escritor no toca las versiones retiradas (las deja en paz). La única zona que ambos necesitan — la cola de la cadena — la gestiona el escritor con su `&mut`, y el lector la evita por construcción.

### La garbage collection

```rust
pub fn gc(&mut self, hasta: Ts) -> usize {
    let mut eliminadas = 0usize;
    let mut vacias: Vec<NodeId> = Vec::new();
    for (id, chain) in &mut self.versiones_nodos {
        let antes = chain.len();
        chain.retain(|v| match v.ts_end {
            None => true,
            Some(t_end) => t_end >= hasta,
        });
        let quitadas = antes - chain.len();
        eliminadas += quitadas;
        if chain.is_empty() {
            vacias.push(*id);
        }
    }
    for id in &vacias {
        self.versiones_nodos.remove(id);
    }
    // ... análogo para aristas
    eliminadas
}
```

La invariante de `gc` es la pieza que un programador con prisa se saltaría: ningún snapshot con `ts ≥ hasta` puede ver una versión retirada con `ts_end < hasta` (los `ts` son monótonos — `siguiente_ts` siempre crece). Si la versión actual (`ts_end = None`) tiene `ts_begin < hasta`, NO se quita: snapshots futuros la necesitan. Cuando una cadena queda totalmente vacía, su entrada del mapa se elimina también — el elemento ya no existe.

La regla mnemotécnica: **`gc(hasta)` borra el PASADO visible para NADIE**. Borra las versiones que ningún snapshot — actual ni futuro — puede ver. La memoria se libera cuando ya no hay observadores que la necesiten.

### El grafo de espera

```rust
pub struct GrafoEspera {
    aristas: Vec<(TxIdLocal, TxIdLocal, Recurso)>,
}

impl GrafoEspera {
    pub fn agregar_espera(&mut self, esperador: TxIdLocal, tenedor: TxIdLocal, recurso: Recurso) {
        self.aristas.push((esperador, tenedor, recurso));
    }
    pub fn quitar_tx(&mut self, tx: TxIdLocal) {
        self.aristas.retain(|&(e, t, _)| e != tx && t != tx);
    }
    pub fn detectar_ciclo(&self) -> Option<Vec<TxIdLocal>> {
        // DFS con tres colores (blanco/gris/negro) — O(V+E)
        // ...
    }
}
```

Aunque HOY no puede haber ciclos en este grafo — `&mut self` en el commit impide que dos escritores compitan por cerrojos — la estructura existe y se DEMUESTRA. Es anzuelo: cuando llegue la Parte VIII y el `MvccStore` acepte varios escritores concurrentes, el gestor de cerrojos que los coordine enchufa `agregar_espera` cuando un escritor pide un recurso que otro tiene, `quitar_tx` cuando termina, y `detectar_ciclo` cada vez que un escritor se bloquea. La detección de ciclos es DFS con tres colores (blanco/gris/negro) en O(V+E): si al descender encontramos un nodo gris, hay ciclo y devolvemos los nodos desde el gris en adelante.

El test `grafo_espera_detecta_ciclo_de_dos` lo demuestra: T1 espera a T2 por el nodo 10, T2 espera a T1 por el nodo 11 — `detectar_ciclo` devuelve `Some([1, 2, 1])`. `quitar_tx(1)` rompe el ciclo. Aunque no esté enchufado al `MvccStore` hoy, la pieza existe y funciona.

## 30.7 Código completo ejecutable

El código del capítulo vive en `liradb-workspace/crates/vol2-liradb/src/cap30_mvcc.rs`. Son ~1.158 líneas, 21 tests en `tests_mvcc` que pasan `cargo test -p vol2-liradb cap30` con ALL_GREEN. La estructura se resume en:

- **Tipos base**: `Ts = u64`, `VersionNode`, `VersionEdge`, `NivelAislamiento::{LecturaSucia, Instantanea, Serializable}` con método `prohibe()` que enuncia qué anomalías quita cada nivel.
- **Errores**: `MvccError::{Validacion, Store}` con `From<TransaccionError>` y `source()` para componer la cadena de errores.
- **El store**: `MvccStore` con `inner: MemoryStore`, `versiones_nodos/aristas`, `reloj`; métodos `new()`, `reloj()`, `siguiente_ts()`, `leer_nodo/arista`, `iter_nodos/aristas`, `commit`, `validar_mvcc`, `gc`.
- **El resumen**: `ResumenCommitMvcc { ts_asignado, nodos_escritos, aristas_escritas, versiones_retiradas }` con `Display` que produce un reporte humano.
- **El grafo de espera**: `Recurso::{Nodo, Arista}`, `GrafoEspera` con `nuevo`, `agregar_espera`, `quitar_tx`, `detectar_ciclo`, `aristas` para inspección; `dfs` interno con tres colores.
- **La re-valoración ACID**: `informe_acid_post_mvcc()` que devuelve `Vec<EntradaAcid>` con el aislamiento AVANZADO (lectura sucia y lost update prohibidas, write skew sobrevive — closer = 40).

Veamos el test central — la demostración clave del capítulo:

```rust
#[test]
fn varios_snapshots_coexisten_sin_bloquearse() {
    let (mut mv, ts1) = store_basico();
    // Lector A lee en ts1.
    let snap_a = mv.leer_nodo(0, ts1).unwrap();
    assert_eq!(snap_a.labels, vec!["Person".to_string()]);

    // Commit que reescribe el nodo 0.
    let mut n = mv.leer_nodo(0, ts1).unwrap();
    n.labels = vec!["Cambiado".to_string()];
    let _ = mv.commit(&[Operacion::PutNode(n)]).unwrap();

    // Lector B (que tomó su foto EN ts1 antes) sigue viendo lo suyo.
    let snap_b = mv.leer_nodo(0, ts1).unwrap();
    assert_eq!(snap_b.labels, vec!["Person".to_string()]);

    // Y nadie bloqueó a nadie: las dos lecturas son por valor y la
    // escritura ocurrió entre medias sin invalidar el snapshot A.
    assert_eq!(snap_a.labels, snap_b.labels);
}
```

Tres líneas que valen un capítulo: lector A lee, commit ocurre entre medias, lector B lee — ambos ven lo mismo. Es lo que el cap. 27 no podía hacer (el borrow checker hubiera bloqueado al escritor mientras A tenía su referencia). Es la promesa del cap. 30 cumpliéndose por construcción: la cadena es append-only, la lectura es por valor, y los `ts` son monótonos.

## 30.8 Prueba de fuego

La prueba de fuego tiene cinco tests-tesis que DEMUESTRAN cada pieza del capítulo:

- **`leer_en_snapshot_anterior_devuelve_la_version_visible`** (807-826): un nodo se reescribe en un commit posterior; el snapshot anterior SIGUE viendo la versión vieja. La diferencia entre MVCC y un store que sobrescribe (el cap. 27 sin MVCC pierde la versión).
- **`varios_snapshots_coexisten_sin_bloquearse`** (928-950): lector A lee, commit ocurre, lector B (foto antes del commit) lee — ambos ven lo mismo. La promesa del capítulo: N lectores + 1 escritor sin bloqueos de lectura.
- **`niveles_prohiben_las_anomalias_esperadas`** (994-1011): `NivelAislamiento::prohibe()` dice la verdad — Instantanea prohíbe lectura sucia y lost update, DEJA PASAR write skew; Serializable prohíbe las tres (SSI con predicate locks cerraría write skew).
- **`grafo_espera_detecta_ciclo_de_dos`** (1031-1041) y **`grafo_espera_detecta_ciclo_de_tres`** (1044-1053): el `GrafoEspera` detecta ciclos T1→T2→T1 y T1→T2→T3→T1; `quitar_tx` rompe el primero. Aunque no se usa en producción HOY, la pieza está testeada.
- **`informe_post_mvcc_avanza_el_aislamiento`** (1115-1146): el `informe_acid_post_mvcc()` documenta que el aislamiento AVANZA (lectura sucia y lost update pasan a prohibidas) — pero write skew sigue pasando y el closer del aislamiento salta al cap. 40.

**Síntoma si el lector se salta este capítulo**: su `MvccStore` sobrescribirá el `Node` anterior (no habrá cadena); un commit con `ts=2` no podrá distinguir «versión reescrita en `ts=2`» de «versión inicial que sigue vigente»; la palabra «snapshot» será un eufemismo para «el estado en RAM ahora mismo». Y — lo más importante — NO entenderá por qué write skew es un problema HONESTO: creerá que MVCC «lo arregla todo» y diseñará transacciones disjuntas confiando en la garantía equivocada.

## 30.9 Qué hemos sacrificado

Toda estructura tiene un precio. MVCC no es gratis:

1. **Memoria para las cadenas**: cada reescritura de un elemento deja una versión retirada en la cadena hasta que `gc` la purgue. En un grafo muy reescrito (p.ej. un nodo que cambia sus etiquetas en cada commit), la cadena crece monótonamente. La `gc` es la pieza que lo controla, pero exige disciplina: llamarla con un `hasta` adecuado (mínimo `mv.reloj()` para vaciar todo lo no visible para snapshots futuros).
2. **Sin timestamp físico**: el `Ts` es orden del programa, no medida de tiempo. Si dos nodos de una red confirman cambios «a la vez» en tiempo real, sus `Ts` son los que el escritor local asignó — distintos y no comparables en términos temporales. La Parte VIII cierra esto con vector clocks; aquí lo admitimos como limitación.
3. **Concurrencia de escritores NO resuelta**: sigue habiendo un único escritor lógico (`&mut self`). MVCC multiplica los LECTORES, no los escritores. Un motor real con varios escritores exige un gestor de cerrojos — la pieza del `GrafoEspera` está construida como anzuelo, pero NO se enchufa al `MvccStore` HOY.
4. **Write skew ABIERTO**: en Snapshot Isolation, dos transacciones que leen y modifican elementos DISJUNTOS a partir del mismo snapshot pueden producir un resultado no serializable. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — fuera del alcance. La `informe_acid_post_mvcc()` lo dice sin ambigüedades: «write skew sigue pasando — Serializable SI con predicate locks lo cerraría», closer = 40.
5. **GC manual**: el capítulo enseña la operación `gc`; integrarla como tarea programada es integración del motor (no entra aquí). Un usuario que olvide llamar `gc` acabará con un `MvccStore` que crece sin parar.

## 30.10 Cómo lo hace una BBDD real

MVCC no es una rareza académica — es la elección por defecto de casi todas las bases de datos modernas, con variantes:

- **PostgreSQL** implementa MVCC desde 8.0 (2005): cada fila lleva `xmin` y `xmax` (los equivalentes de nuestro `ts_begin` y `ts_end`); la «instantánea» de una transacción se materializa como una lista de `xmin` visibles. El nivel por defecto es «Read Committed» (cada statement ve su propio snapshot, NO la transacción entera); «Repeatable Read» usa SI (prohibe lectura sucia y lost update, igual que nosotros); «Serializable» implementa SSI con predicate locks sobre los predicados leídos (Cahill et al. 2008 — el paper que el cap. 30 cita como «lo que cerraría write skew»).
- **CockroachDB** y **YugabyteDB** llevan MVCC al territorio distribuido: cada nodo asigna su propio `Ts` con un reloj HLC (Hybrid Logical Clock), combinación de tiempo físico y contador lógico. Es lo que llamábamos «la frontera con vector clocks» — la Parte VIII.
- **FoundationDB** implementa Serializable SI sobre un MVCC con `read_version` por lectura y conflict ranges para detectar write skew (Ports & Grittner, 2016). Su `detectar_conflicto` es el equivalente industrial del `detectar_ciclo` de nuestro `GrafoEspera`.
- **Wu et al., «An Empirical Evaluation of In-Memory Multi-Version Concurrency Control»**, VLDB 2017: una evaluación sistemática de las variantes MVCC (timestamp ordering, snapshot isolation, serializable) sobre cargas OLTP. Conclusión: MVCC gana en throughput a 2PL bajo concurrencia media-alta, y SSI añade menos overhead del que la intuición sugiere — pero la implementación importa más que la teoría.
- **David P. Reed**, «Naming and Synchronization in a Decentralized Computer System», MIT 1978: la génesis. La propuesta formal de versiones múltiples en sistemas descentralizados. Lo que el cap. 30 cita como el origen de la idea — y la conexión histórica directa con la Parte VIII.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: predice el `ResumenCommitMvcc` de tres commits consecutivos sobre `store_basico()` (un PutNode que reescribe, un DeleteNode). Explica por qué la cadena del nodo 0 tiene longitud 1, 2, 1 — no 1, 2, 0.
- *Intermedio*: implementa `leer_en_snapshot_exterior` que devuelva `None` si NO hay ninguna versión visible para el `ts` (la lógica actual devuelve `Some` aunque el `ts` sea muy anterior). ¿Cómo distinguirías «el nodo nunca existió» de «el nodo se borró»?
- *Experto*: implementa `mvcc_iter_nodos_entre(desde, hasta)` que devuelva los nodos cuyo `ts_begin` está en el rango — el primer ladrillo de un «time-travel query» estilo SQL Server `AS OF`. Conecta con el gancho del cap. 40: ¿qué cambia cuando los `Ts` vienen de MÁQUINAS DISTINTAS?

## 30.11 Lo que te llevas

- **MVCC** es la maquinaria que da MATERIALIDAD al «snapshot» del cap. 26: una foto lógica no es una copia, es un `Ts` al que cada elemento responde con la versión visible.
- **El `Ts` es orden del programa, no medida de tiempo**: dos `Ts` son comparables sin ambigüedad dentro de un escritor; entre máquinas, la Parte VIII los reconcilia.
- **Las cadenas de versiones** son append-only: la última entrada es la actual (`ts_end = None`); las escrituras RETIRAN la actual y APPENDIZAN; los deletes RETIRAN sin appendizar (la ausencia es el estado).
- **`&self` para lectores, `&mut self` para el escritor**: el borrow checker admite la convivencia porque las operaciones son disjuntas por construcción. Es el patrón que multiplica los lectores del cap. 27 sin tocar la regla «un único escritor».
- **Instantanea PROHÍBE lectura sucia y lost update, DEJA PASAR write skew**: la frontera honesta. Cerrar write skew exige Serializable SI con predicate locks — fuera del alcance del Vol.II.
- **El `GrafoEspera` existe sin uso HOY**: anzuelo para el gestor de cerrojos del cap. 40. La detección de ciclos es DFS 3 colores en O(V+E).
- **El `inner` es el espejo material**: las queries que NO piden snapshot lo usan; la MVCC vive ENCIMA y el `delete-then-put` mantiene la consistencia entre ambos mundos.

## 30.12 Ojo, cuidado con…

- **Confundir `mv.reloj()` con un `ts` válido**: `reloj()` es el SIGUIENTE timestamp a asignar — NO una snapshot válida (todavía no hay versión visible para ese `ts`). Usa `resumen.ts_asignado` del commit inicial o de un commit anterior. Síntoma: `leer_nodo(2, mv.reloj())` devuelve `None` cuando debería devolver la versión actual.
- **Asumir que `validar_buffer` del cap. 27 sirve para MVCC**: la del 27 rechaza `PutNode` de id existente; la MVCC SOBREESCRIBE. Síntoma: 6 tests fallaban con `Validacion(DuplicateNode)` durante la calibración. El fix fue `validar_mvcc` PROPIA con semántica de sobreescritura.
- **Creer que la versión «más reciente» es siempre la actual**: la versión visible para un `ts` se calcula por la condición `ts_begin ≤ ts ∧ (ts_end > ts ∨ ts_end = None)`, NO por «el último siempre». Olvidar el caso de la versión retirada devuelve `None` cuando había versión.
- **Olvidar que `gc(hasta)` puede vaciar cadenas**: si TODAS las versiones quedan retiradas y `ts_end < hasta`, la entrada del mapa se ELIMINA. Síntoma: el test `gc_elimina_cadenas_vacias_cuando_el_elemento_se_borra` falla si se asume que la cadena sobrevive.
- **Pensar que `write skew` es un bug**: NO — es la frontera HONESTA de Snapshot Isolation. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008). El cap. 30 lo DEJA ABIERTO a propósito. Síntoma: diseñar transacciones disjuntas creyendo que MVCC las serializa — produce un resultado no serializable.

## 30.13 Pin de batalla

> *«La consistencia del snapshot no viene de un cerrojo que sostiene al lector en su sitio. Viene de que cada elemento lleva la historia de quién lo vio, y de que el lector elige qué historia le interesa.»*

## 30.14 Si solo lees 30 segundos

MVCC resuelve la anomalía histórica del Vol.II — un único escritor por el borrow checker — permitiendo que N lectores lean MIENTRAS un escritor escribe, sin lecturas sucias ni actualizaciones perdidas. Cada elemento lleva una cadena de versiones `{ts_begin, ts_end?, valor}`; un snapshot es un `Ts` lógico (un `u64` monótono), y `leer_nodo(id, ts)` devuelve la versión visible ESE instante. El escritor toma `&mut self` y APPENDIZA una versión nueva; los lectores toman `&self` y clonan — la convivencia por el borrow checker es la forma del aislamiento. Instantanea PROHÍBE lectura sucia y lost update, DEJA PASAR write skew. El `GrafoEspera` para deadlocks existe como anzuelo para caps. futuros, no como código en uso HOY.

## 30.15 Una historia pequeña

Cuando llegamos al cap. 27 con el primer `commit(&mut self)` funcional, pensábamos que teníamos «transacciones». Teníamos vocabulario ACID, `Anomalia::LecturaSucia` y `Anomalia::ActualizacionPerdida` bien definidas, y la promesa — todavía vacía — de que el aislamiento mejoraría. Lo que NO teníamos era la posibilidad de demostrar la promesa: el borrow checker era el cerrojo, y bajo el cerrojo no había concurrencia que aislar.

Fue el cap. 30 el que cerró el círculo. El momento clave no fue técnico: fue cuando el test `varios_snapshots_coexisten_sin_bloquearse` pasó por primera vez. Lector A lee, commit ocurre, lector B lee — ambos ven lo mismo. Era algo que el cap. 27 NO PODÍA hacer, y que ahora era trivial: dos `&self` y un `&mut self` sobre el mismo `MvccStore`, sin más ceremonia que pasarle el `ts` correcto. La anomalía histórica del Vol.II estaba resuelta — no por añadir cerrojos, sino por QUITARLOS: la cadena es el cerrojo, no un lock manager.

Y aquí aprendimos también la honestidad de la Parte VI: el `informe_acid_post_mvcc()` dice, sin ambigüedad, que el aislamiento AVANZA pero NO SE CIERRA. Write skew sobrevive, y cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — la pieza que dejaremos para la Parte VIII, cuando la concurrencia REAL de varios procesos abra la puerta al skew y el `GrafoEspera` aquí construido encuentre su uso.

## Ejercicios resueltos

**1. ¿Por qué la validación del cap. 27 (`validar_buffer`) no sirve para MVCC?**

Porque `validar_buffer` del cap. 27 asume INSERCIÓN ESTRICTA: rechaza `PutNode` de un id existente con `StoreError::DuplicateNode`. En MVCC, SOBREESCRIBIR un nodo es LEGAL — es lo que crea una nueva versión en la cadena (la versión anterior se RETIRA con `ts_end` y la nueva se APPENDIZA). Si reutilizáramos `validar_buffer`, un commit que reescribe el nodo 0 fallaría con `Validacion(DuplicateNode)` y la MVCC no podría hacer su trabajo. Por eso el cap. 30 implementa `validar_mvcc` PROPIA, que registra `PutNode` como `sim_creados_nodos.insert(n.id)` (no como error) y verifica `PutEdge` contra el estado visible (inner + simulación del buffer). Las dos políticas — inserción estricta (cap. 27, `MemoryStore`) y sobreescritura (cap. 30, MVCC) — son DIFERENTES, y la lección de la calibración (6 tests fallaban) lo demostró empíricamente.

**2. ¿Por qué `leer_nodo(0, mv.reloj())` puede devolver `None` cuando el nodo existe?**

Porque `mv.reloj()` es el SIGUIENTE `ts` a asignar — todavía no hay ninguna versión con `ts_begin <= mv.reloj()` (la versión actual tiene `ts_begin` igual al del último commit, y `ts_end = None`, así que es visible para `ts >= ts_begin`, pero el SIGUIENTE `ts` aún no es visible para sí mismo — no hay versión que cumpla `ts_begin <= mv.reloj()` y `ts_end > mv.reloj()`). Es la diferencia entre «el próximo número que se asignará» y «un número que ya es válido como snapshot». La forma correcta es capturar `resumen.ts_asignado` del commit inicial o de un commit anterior y usar ESE como snapshot. Esta trampa aparece en §35 de MIGRATION-PATTERN como una de las lecciones de calibración.

## Ejercicios propuestos

**Esencial.** Predice, ANTES de ejecutar, cuántas versiones retira cada uno de los tres commits consecutivos y cuál es el `ts_asignado`. Parte del `store_basico()` (3 nodos, 2 aristas, `ts=1`); ejecuta un segundo commit con `[PutNode(Nodo::new(0, "Renacido"))]` y comprueba el `ResumenCommitMvcc` (nodos_escritos=1, versiones_retiradas=1, ts_asignado=2); luego un tercero con `[DeleteNode(0)]` y comprueba (nodos_escritos=0, versiones_retiradas=1, ts_asignado=3 — un delete RETIRA sin appendizar). Pistas: ¿qué hace `DeleteNode` con la cadena del elemento? ¿`reloj()` es lo mismo que `ts_asignado`? Tras el `DeleteNode`, ¿cuántas entradas tiene la cadena del nodo 0? Criterio: predicción exacta de los tres `ResumenCommitMvcc` y de la longitud de la cadena (1, 2, 1 respectivamente).

**Intermedio.** Tomando la cadena del nodo 0 tras los tres commits anteriores (`{ts_begin=1, ts_end=3, "Ana"}` y vacío tras el delete), explica por qué `leer_nodo(0, 1)` devuelve la versión inicial, por qué `leer_nodo(0, 2)` también, y por qué `leer_nodo(0, 3)` devuelve `None`; conecta con `Anomalia::ActualizacionPerdida` del cap. 27 (¿qué habría pasado SIN MVCC si dos tx leen `leer_nodo(0, 2)` y reescriben?); conecta con la `RecoveryError` del cap. 29 (¿qué ganaría la MVCC sobre el WAL si el sistema cae a mitad del commit?). Pistas: ¿cuál es la versión con `ts_begin ≤ ts ∧ ts_end > ts` para cada `ts`? ¿Qué condición del cap. 27 PROHIBIRÍA la actualización perdida? ¿`delete-then-put` es atómico ante un crash? Criterio: tres predicciones de `leer_nodo` correctas + una frase que conecte `Anomalia::ActualizacionPerdida` con la condición de visibilidad + una frase que conecte `delete-then-put` con la fragilidad ante crash (el cap. 29 lo cubre; el cap. 30 NO).

**Experto.** Implementa `gc(hasta)` sobre un `MvccStore` con esta historia: `ts=1` escribe nodo 0 («Ana»), `ts=2` reescribe («Ana S.»), `ts=3` reescribe («Ana Sofía»), `ts=4` borra; predice cuántas versiones se quitan con `gc(4)` y `gc(5)` y demuestra con `cargo test` que coincide con `gc_descarta_versiones_retiradas_antiguas`. LUEGO razona al revés: dado un `MvccStore` con N reescrituras del mismo nodo y SIN deletes, ¿cuál es el MÍNIMO `gc(hasta)` que vacía todas las versiones retiradas? Pistas: tras `ts=4` (delete), ¿cuál es el `ts_end` de la versión inicial? ¿La cadena queda con longitud 1 o 0 tras el delete? ¿`gc(hasta)` quita la versión con `ts_end = None`? Criterio: predicciones exactas + identificación de que la versión con `ts_end = None` (la «actual») NO se quita NUNCA por `gc` (los snapshots futuros la necesitan) + reconocimiento de que el `gc` mínimo para vaciar todo es `mv.reloj()` (el siguiente `ts` a asignar — más allá, ningún snapshot puede estar vivo).

## Para profundizar

- **David P. Reed**, «Naming and Synchronization in a Decentralized Computer System», MIT 1978 — la génesis de las versiones múltiples en sistemas descentralizados. El cap. 4 de la tesis es donde está la idea; el resto es la conexión con la Parte VIII.
- **Cahill, Fekete, Liarokapis, Bernstein**, «Serializable Snapshot Isolation in PostgreSQL», VLDB 2008, DOI 10.14778/1454159.1454166 — el algoritmo que cierra el write skew que el cap. 30 DEJA ABIERTO. La pieza que el Vol.II cita como anzuelo al cap. 40.
- **PostgreSQL Global Development Group**, «13.2. Transaction Isolation», PostgreSQL documentation — las definiciones operativas de Read Committed / Repeatable Read / Serializable. El vocabulario del que `NivelAislamiento` es una traducción.
- **Wu, Arulraj, Lin, Xi, Pavlo, Chen, Lee, Song, Feng, Lohman, Xu, Zhao, Chen**, «An Empirical Evaluation of In-Memory Multi-Version Concurrency Control», VLDB 2017 — la evidencia moderna de que MVCC es la elección de la mayoría de motores analíticos en RAM y de las diferencias prácticas entre variantes.
- **Ports & Grittner**, «Serializable Snapshot Isolation in FoundationDB», 2016 — la implementación industrial de SSI con `read_version` y conflict ranges; el puente directo al cap. 40.
- **Código fuente de PostgreSQL** (`heapam.c`, `tqual.c`): la implementación canónica de MVCC con `xmin`/`xmax`, comentada con un detalle exquisito.
- **Liran Einav**, «The Art of Writing Efficient Database Code» (workshop, 2019) — la conferencia que reconcilia las intuiciones de MVCC con las medidas de throughput en producción.

## Mini-diálogo: en guardia nocturna

> — O sea, que MVCC es «cada elemento lleva una cadena de versiones». ¿Y por eso un capítulo entero?
>
> — Porque esa cadena es la diferencia entre «tu motor lee y escribe» y «tu motor tiene varios lectores y un escritor sin que se pisen». El cap. 27 te daba las palabras — `Anomalia::LecturaSucia`, `Anomalia::ActualizacionPerdida` — pero no podías DEMOSTRAR que estaban prohibidas: el borrow checker era el cerrojo. El cap. 30 te da la maquinaria: ahora un lector toma `&self`, clona su versión, y el escritor toma `&mut self` y APPENDIZA. La anomalía histórica del Vol.II está resuelta.
>
> — Pero entonces, ¿no es un poco... redundante con cerrojos?
>
> — Lo contrario. La MVCC QUITA cerrojos de lectura: la cadena ES el cerrojo. Los lectores no esperan a nadie; el escritor no espera a nadie que esté leyendo. Y ése es exactamente el patrón que el cap. 27 no podía enunciar: el borrow checker admite `&self` y `&mut self` SIMULTÁNEOS mientras las operaciones sean disjuntas, y la MVCC las hace disjuntas por construcción. Con eso, ya puedes construir de noche — y de día, y entre dos procesos cuando llegue la Parte VIII.
>
> — ¿Y el write skew?
>
> — Queda abierto, a propósito. Instantanea prohíbe lectura sucia y lost update, pero no el skew. Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — fuera del Vol.II. La honestidad de este capítulo es justamente decir «avanzamos, pero no cerramos»: el closer del aislamiento salta al cap. 40. Es la misma honestidad que el cap. 27 con las anomalías y el cap. 29 con el undo completo: no mentimos sobre lo que falta.

---

*(Próximo capítulo: 31 — La CLI de LiraDB. Aquí el `MvccStore` aprendió a convivir con N lectores; ahora el REPL aprenderá a exponérselos al usuario — qué `ts` toma cada comando, cómo se gestiona una sesión, qué significa «transacción» en una shell.)*# Apéndice 0 — Manual de estilo unificado

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