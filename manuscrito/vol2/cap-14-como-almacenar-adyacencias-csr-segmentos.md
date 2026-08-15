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
