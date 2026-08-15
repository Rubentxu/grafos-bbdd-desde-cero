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