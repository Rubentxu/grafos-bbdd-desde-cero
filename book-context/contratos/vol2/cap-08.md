# CONTRATO DE CAPÍTULO — Vol.II Cap. 8: Diseñar una API antes de persistir (trait `GraphStore`)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap08_graph_store.rs` (251 líneas,
> 4 tests en `tests_store`: `memory_store_basico`, `rechaza_duplicado`,
> `delete_node_elimina_aristas` — verificados ALL_GREEN
> `cargo test -p vol2-liradb --lib cap08` → 4 passed). Este capítulo ABRE la
> Parte II del Vol.II (caps. 8-10): se diseña el CONTRATO de persistencia
> ANTES de escribir el primer byte en disco. Código modelo (prerrequisito):
> `liradb-workspace/crates/vol2-liradb/src/cap07_modelo.rs` (cap. 7). Ganchos:
> cap. 9 (encoding → del objeto al byte), cap. 10 (persistencia append-only),
> cap. 27 (el borrow checker como germen del "único escritor").

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda al llegar aquí**: el modelo de datos del cap. 7
  (`cap07_modelo.rs`): `Node { id, labels, props }`, `Edge { id, source,
  target, label, props }`, `Value` tipado (Null/Bool/Int/Float/String/Bytes),
  con `NodeId = usize` y `EdgeId = usize` de base (aún no generacionales:
  eso llega más adelante). Rust básico: `struct`, `enum`, `impl`, `Vec`,
  `HashMap`, `Option`, `match`, y cómo se escribe/y lee un `&mut self` frente
  a `&self`. Qué es un grafo y los principales algoritmos (Vol. I, caps. 1-9
  introducidos en el cap. 7 del Vol.II).
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**:
  (1) «la forma de guardar un grafo es LA FUNCIÓN que lo guarda» — no: la
  función para un `MemoryStore` es distinta de la de un backend en disco, y
  si el cliente depende de ellas no puedes cambiar de backend sin reescribir
  TODA la aplicación; (2) «las operaciones del grafo son un TODO, no un
  CONTRATO» — no: hay un "qué" (las operaciones, idénticas para cualquier
  almacén) y un "cómo" (la implementación, que cambia); (3) «Result es
  siempre mejor que Option/bool» — no: la API TIPAtiza el error SOLO donde
  hay error que distinguir (put → Duplicate/Unknown/InvalidEdgeEndpoints)
  y usa `bool`/`Option` donde el "no" es un hecho normal (delete → false,
  get → None); (4) «borrar un nodo es borrar un nodo» — no: `delete_node`
  DETONA una cascada que borra sus aristas adyacentes, y quién lo hace (el
  store, no el cliente) es LA decisión que el trait encapsula; (5) «un
  `Vec<T>` de hace tres capítulos vale» — no: aquí usamos `Vec<Option<T>>`
  porque las aristas NO se re-numeran al borrar (el `usize` es una clave
  estable, no una posición en un array compacto).
- **NO debe saber todavía**: encoding en bytes (cap. 9), WAL/append-only
  (cap. 10), páginas y slotted pages (cap. 11), CSR e índices (caps. 14-15),
  IDs generacionales (cap. 3, nombrado como "luego lo verás"), transacciones
  y el "único escritor" (cap. 27, donde el borrow checker del trait se
  convierte en transacción). NO se anticipan; se corta nombrando el capítulo
  futuro como referencia.

## 2. Conceptos (del grafo curricular)

- `present`: **trait** de Rust (el `GraphStore` como PUERTO); interfaz o
  **contrato**; desacoplamiento **qué vs cómo**; **arquitectura hexagonal /
  ports and adapters** (Cockburn); **`StoreError`** tipado
  (Duplicate/Unknown/InvalidEdgeEndpoints); la firma como semántica — por qué
  `put_node -> Result<()>`, `get_node -> Option<&Node>`, `delete -> bool`;
  `Vec<Option<T>>` como "slots" con tombstones (no re-numerar los `usize`);
  listas de adyacencia frágiles (`adj_out`/`adj_in`) y lo que hay que
  mantener en sincronía al borrar; `Box<dyn Iterator + '_>` para iterar;
  `&mut self` (escrituras) vs `&self` (lecturas) y su invariante de exclusión.
- `practice`: el modelo de datos del cap. 7 dentro de la API (Node/Edge y
  sus campos); crear una `struct` con `Vec<Option<..>>` y `impl`; el
  patrón "derivar, no llevar en cabeza" (node_count = filtrar `is_some()`,
  no una columna de contadores).
- `consolidate`: `Option` y `match`; `usize` como ID base; `HashMap` y el
  modelo Property Graph del cap. 7.
- `out_of_scope` (solo nombrar, sin explicar): encoding/endianness (cap. 9),
  persistencia en disco (cap. 10), slots/páginas (cap. 11), CSR (cap. 14),
  índices (cap. 15), generational IDs (cap. 3), transacciones/único
  escritor (cap. 27).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge** (qué SABE al terminar):
  (1) enuncia la tesis del capítulo: caps. 9-10 implementarán ESTE trait y
  el resto de la Parte II usará el puerto sin tocar a los clientes — la
  persistencia es un ADAPTER de un puerto ya diseñado;
  (2) justifica por qué `delete_node -> bool` y `delete_edge -> bool`
  (borrar una clave inexistente es un "no" NORMAL, no un error) frente a
  `put* -> Result<(), StoreError>` (insertar un duplicado o una arista
  huérfana SÍ es un fallo tipado que el cliente debe poder distinguir);
  (3) explica que `delete_node` DETONA la cascada de eliminación de sus
  aristas adyacentes, y que delegar esto al STORE (no al cliente) es la
  razón de ser de la API: cualquier backend la cumple igual;
  (4) describe por qué los `usize` de nodo/arista NO se re-numeran al borrar
  (los slots quedan `None`: `Vec<Option<T>>`), y la diferencia
  `get_node -> Option<&Node>` (ausencia normal) vs `put_node -> Result`
  (duplicado es un error);
  (5) identifica el borrow checker impregnado en el trait: `&mut self` en
  toda escritura ⇒ un solo `&mut` vivo a la vez ⇒ germen del "único
  escritor" del cap. 27.
- **Skills** (qué HACE — 2-3 tareas con el código):
  (1) implementa un backend de memoria (`MemoryStore`) que cumple el trait
  `GraphStore` completo y lo verifica con `cargo test cap08`;
  (2) usa el trait como mero cliente: poblar un grafo, consultar
  `out_edges`/`in_edges`, recorrerlo con `iter_nodes`/`iter_edges`, y borrar
  (observando la cascada) SIN saber qué backend lo hace;
  (3) escribe un código genérico contra el puerto que compila igual para
  `MemoryStore` y para `&dyn GraphStore`, preparando el intercambio de
  backend del cap. 10.
- **Wisdom** (qué DECIDE — 1-2 "cuándo NO"):
  (1) decide cuándo un resultado es `bool`/`Option` (hecho normal) y cuándo
  `Result` con error tipado (caso que el llamador DEBE distinguir), en vez
  de usar siempre `Result` "por si acaso";
  (2) decide escribir el CONTRATO antes de la implementación en disco: el
  coste de re-diseñar la API tras tener un motor con offset de páginas es
  órdenes de magnitud mayor que cambiar una firma ahora.

## 4. Modelo mental

- **La figura ordenadora**: la cafetería/puerto hexagonal. El trait
  `GraphStore` es el MOSTRADOR (interfaz): pides "pon un nodo", "dame las
  aristas salientes", "borra este nodo (y sus aristas)". Detrás del mostrador
  hay tres cocinas que sirven exactamente el mismo menú: Memoria (Cap. 8),
  Disco append-only (Cap. 10), Páginas (Cap. 11). El cliente NUNCA sabe qué
  cocina está abierta. Es ports-and-adapters (Cockburn): el "port" es el
  mostrador; cada cocina es un "adapter".
- **Diagramas ASCII**:
  (a) el puerto hexagonal: `App` → [GraphStore] → {Memory | Disk | Network};
  (b) `Vec<Option<Node>>` con slots `Some(..)` y `None` (tombstones), y
  `delete_node` borrando las aristas de su slot; (c) la mesa de firmas
  `Result` vs `bool` vs `Option` con el porqué semántico de cada una.
- **Momento ¡ajá!**: «la API por la que pides datos es la MISMA tanto si
  los datos viven en un `Vec` de RAM como si viven en 40.000 páginas en
  disco. Lo que cambia es lo que hay detrás del mostrador — y por eso puedo
  escribir los capítulos 9-10 sin tocar a nadie que use el grafo».

## 5. Los porqués (grill — la pregunta más importante de cada decisión)

| # | Decisión (`cap08_graph_store.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Definir un TRAIT (`GraphStore`) antes que cualquier backend en disco | Cocinar el contrato AHORA: los caps. 9-10 implementarán este puerto; la aplicación que usa el grafo queda agnóstica al backend (hexagonal). Reescribir la API después de tener un motor con offset de páginas costaría tocar TODO cliente | Escribir primero `MemoryStore` y "extraer" el trait después: el trait saldría moldear a la memoria y no valdría para disco; o escribir el backend de disco y deconstruir: cambio de API enorme tarde | El cap. 10 dejaría de poder cambiar de backend; cada backend (mem/disk/páginas) inventaría su propia semántica de borrado | Doc del trait (líns. 5-9); Cockburn, "Hexagonal Architecture" (2005) / "Ports and Adapters" |
| 2 | `put_node` / `put_edge` → `Result<(), StoreError>` con Duplicate/Unknown/InvalidEdgeEndpoints TIPADOS | Insertar un dup o una arista huérfana es un fallo que el llamador DEBE distinguir (¿reintento? ¿hablo con el esquema? ¿corrijo endpoints?). El error es un `enum` con datos: el cliente puede hacer `match` | `bool` en put: el llamador no sabe QUÉ falló; `String` de error: rompe el `match` y escribe la mensajería a mano | El llamador no puede responder al fallo; "debuguear" errores como strings opacos | `StoreError` (líns. 49-56, 58-70); test `rechaza_duplicado` (232-239) |
| 3 | `delete_node` / `delete_edge` → `bool` (¡no Result!) | Borrar una clave que no existe es un "NO" NORMAL (idempotencia), no un fallo tipado. `bool` dice "existía → lo borré" vs "no existía"; y `get_/delete` cuentan con el "fallo normal" como primer ciudadano de la API | `Result<(), StoreError>` en delete: sobre-tipar un "no" que no es error, forzando `?` y `match` donde no hay nada que distinguir | El cliente inventa conventos de retorno; código ruidoso por doquier | Firmas 36-39; la MESA de firmas (Result vs bool vs Option) en §4/§8 |
| 4 | `delete_node` DETONA la cascada: borra sus `out_edges` + `in_edges` ANTES de `None` el slot | La integridad del grafo (no quedar aristas a nodos borrados) es CONTRATO del store, no harina del cliente. Cualquier backend debe garantizar "si borro el nodo, sus aristas desaparecen" — por eso el cap. 10 (disco) implementará la misma promesa | Que el cliente recuerde `out_edges(u)` y borre cada arista a mano antes de `delete_node`: cada aplicación lo haría distinto (y se lo olvidaría a menudo), re-introduciendo el bug que la API mata | Aristas colgantes: "nodo X arista Y", get_node(X)=None. Corrupción silenciosa de la topología | `delete_node` (162-185), `delete_edge` (187-204); test `delete_node_elimina_aristas` (242-250) |
| 5 | `Vec<Option<Node>>` / `Vec<Option<Edge>>` con tombstones, y NO re-numerar | `NodeId`/`EdgeId` son CLAVES estables de 32-bit+ contra las que otros apuntan (source/target). Si al borrar re-numeraras (compactar), cada arista apuntaría a otro nodo sin avisar. El `None` es un tombstone: reciclar la clave sería legal pero NO la hacen los esquemas reales sin generational ID (cap. 3) | `Vec<Node>` compactado (borrar = shift): ids mentirosos, aristas rotas; re-numerar con un mapa id→índice: dos niveles de indirección, sin necesidad para ids densos en memoria | get_node con id viejo devuelve un NODO DISTINTO (colisión silenciosa) — el peor bug de una BD | `nodes: Vec<Option<Node>>` (78), `ensure_node_capacity` (88-94), `get_node` (138-140) |
| 6 | `get_node -> Option<&Node>` (referencia), no `Option<Node>` | Leer NO copia: el store PINTA la ref y apunta al slot si `is_some()`. Cero clonar en el camino caliente de lectura; el cliente puede solo‑leer | `get_node -> Option<Node>` por valor: clona el `HashMap` de props en cada get (caro); `&Node` con `get(id).and_then(Option::as_ref)` (139-140) es O(1) y sin copy | Cada get_ clona propiedades: lectura lenta sin motivo | firma 18; `get_node` (138-140) |
| 7 | Listas de adyacencia SEPARADAS `adj_out`+`adj_in` y su borrado en `retain` | out_edges/in_edges en O(grado) resuelven el recorrido dirigido que usan los algoritmos del Vol. I (BFS/Dijkstra); mantener AMBAS y sincronizarlas en `delete_edge` (retain) es el coste real de la topología. `out_edges` devuelve `Vec<EdgeId>` (copia de ids, barata) | Tabla de adyacencia única (sin in): in_edges sería O(E); derivar in de out cada vez: O(E) por consulta | Recorridos entrantes (PageRank etc.) se enquistan; borrar sin tocar las listas deja huérfanas `adj_out`/`adj_in` | `adj_out`/`adj_in` (79-80), `out_edges` (146-148), `delete_edge` (187-204: retain en ambos) |
| 8 | `&mut self` en TODAS las escrituras y `&self` en TODAS las lecturas (borrow checker del trait) | El trait codifica la invariante de exclusión MUDA: solo un `&mut` vivo a la vez ⇒ sin dos escritores simultáneos ⇒ la semilla del "único escritor" / transacciones del cap. 27. El propio compilador se lo garantiza | Quitar el `&mut` y usar `&self` con mutación interior (`RefCell`): rompe la garantía de exclusión y se convierte en un problema de runtime; añadir hilos/`Sync` ahora: anti-requisito del tomo | Dos hilos mutando el grafo a la vez → corrupción o panics de borrow; imposible razonar sobre "estado consistente" | Firmas 12-45 (mut en writes, sin mut en reads); cap. 27 relaciona esto con el escritor único |
| 9 | Iterar con `Box<dyn Iterator<Item = &T> + '_>` (no `Vec<T>` de las listas) | `iter_nodes`/`iter_edges` devuelven VISTAS perezosas sobre los slots ocupados (`filter_map(as_ref)`), sin clonar ni materializar todo; el `+ '_` ata la vida al borrow del store (ninguna mutación mientras se recorre) | Devolver `Vec<Node>` clonado: materializa y clona TODOS los nodos en cada iteración; trait objects estilo callback: menos "RSity" y peor de componer | Iterar un grafo grande duplica memoria/CPU; el cliente no puede explotar la pereza (p. ej. parar pronto) | `iter_nodes` (206-208), `iter_edges` (210-212) |

## 6. Primera solución vs solución evolucionada

- **Ingenua (lo que escribiría un novato)**: funciones de memoria sueltas —
  `fn put_node(store: &mut MemoryStore, ...)`, `fn delete_node(store, id)`
  que borra el `Vec[i] = None` y "ya está". Sin trait, sin errores tipados,
  sin adyacencias, delegando en el cliente el recordatorio de borrar aristas:
  "borro el nodo y aparte recuerdo borrar sus aristas".
- **Qué la rompe exactamente**: (a) el borrado no sincroniza las aristas —
  queda un `edge` cuyo `source`/`target` apunta a un nodo ya `None`
  (arista colgante, get_node devuelve None, la topología miente); (b)
  cada llamada a mano lo hace con un criterio distinto (¿borro out? ¿in?);
  (c) al cambiar del `Vec` a un backend en disco, hay que reescribir TODAS
  las llamadas; (d) no saber si borrar falló por "no existía" o por "no
  puedo".
- **Evolución visible**: las mismas operaciones viven detrás de un TRAIT con
  firma semántica (`put..Result` / `get..Option` / `delete..bool`),
  `StoreError` tipado, `delete_node` que SÍ detona la cascada solitaria, y
  sin re-numeración (`Vec<Option<T>>`). Que el cliente llame a `delete_node`
  y vea que las aristas adyacentes desaparecen sin que él tuviera que
  pedirlo — el momento donde el contrato gana a la improvisación.

## 7. Prueba de fuego

- **TEST-TESIS** `delete_node_elimina_aristas` (242-250): meter 2 nodos + 1
  arista, `delete_node(0)` → `edge_count()==0` sin que el cliente toque
  nada. La cascada es CONTRATO, no cortesía.
- **TEST-SEMÁNTICA** `rechaza_duplicado` (232-239): `put_node(id A)` + 
  `put_node(id A)` → `Err(StoreError::DuplicateNode(0))`. El error es
  TIPADO y `match`-eable.
- **TEST-BÁSICO** `memory_store_basico` (220-229): counts + out_edges +
  in_edges coherentes tras put.
- **Síntoma si el lector se saltara el capítulo**: en el cap. 10 no tendría
  dónde conectar el backend de disco; cada uno de sus programas volvería a
  decidir a mano cómo borrar un nodo y con qué retorno; y (peor) el trait
  que los caps. 11+ esperan (páginas/CSR) no existiría como contrato
  estable con `&mut self` de único escritor.

## 8. Trampas y errores comunes

1. **Borrar sin cascada**: hacer `delete_node` que solo `None`s el slot y
   olvidar las aristas adyacentes → aristas colgantes que apuntan a nodos
   inexistentes. Detección: `delete_node` seguido de `edge_count()` no
   baja, o `get_edge` devuelve un `Edge` con `source` que es `None`.
2. **Usar `Result` para TODO** (hasta delete/get): sobre-tipar hechos
   normales. Detección: firmas que devuelven `Result<Option<_>>`,
   `Result<bool>`, o `?` en cada llamada a delete. La regla: ¿es un "no"
   normal o un fallo que debe distinguir? delete→bool, get→Option,
   put→Result.
3. **Re-numerar o compactar los IDs al borrar** (usar `Vec<T>` y shift):
   rompe los `source`/`target` de las aristas → colisión silenciosa.
   Detección: `get_node(i)` tras borrar devuelve un nodo DISTINTO. La regla:
   los ids son claves estables; borrar es dejar `None` (tombstone).
4. **No mantener en sincronía `adj_out`/`adj_in`** en `delete_edge`
   (solo tocar `edges`): los recorridos `out_edges`/`in_edges` siguen
   contando la arista borrada. Detección: `out_edges` devuelve un id que
   `get_edge` ya no encuentra.
- **Precisión de lenguaje (glosario)**: _trait / puerto / interfaz_ (el
  contrato); _adapter / backend_ (la implementación concreta); _tombstone_
  (slot vacío `None` que "entierra" un id); _cascada_ (borrado en cadena de
  las aristas de un nodo); _caja / trait object_ (`Box<dyn Iterator>`);
  _borrow_ (el préstamo del compilador; `&mut` exclusivo vs `&` compartido);
  _CLAVE estable_ vs _índice compacto_ (usize‑ID que no se re-numera).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)** — sobre la estrella `0→{1,2,3}`
  (4 nodos, 3 aristas `Edge::new(0,0,1)`, `(1,0,2)`, `(2,0,3)`), predecir
  ANTES de ejecutar qué devuelve `out_edges(0)`, `in_edges(1)`, `node_count`
  y `edge_count`; luego `delete_node(0)`: ¿cuánto vale `edge_count()` tras
  borrar y por qué, SI el store detona cascada? Verificar con
  `memory_store_basico` + `delete_node_elimina_aristas` (test añadido).
  Pistas: (1) ¿quién llama a los borrados de aristas, tú o `delete_node`?;
  (2) ¿`out_edges` o `in_edges` devuelven Vec<EdgeId>?; (3) ¿qué cuenta
  `edge_count`, slots `Some` o índices ocupados? Criterio: predicción exacta
  de los 4 números antes y después, y explicación de la cascada. (Retrieval
  del contrato del capítulo.)
- **analizar (intermedio — SPACING al cap. 7)**: desde el modelo del cap. 7
  (`Node::with_prop`, `Value`), construir 2 nodos `Person` con props y una
  arista `KNOWS`, insertarlos vía el STORE (no a mano), y razonar sobre la
  decisión del cap. 7: ¿por qué las props existen en el `Node` y no en el
  trait?; ¿qué pasa con la arista `KNOWS` si `delete_node(0)` y aún está
  en `iter_edges`? Verificación: `memory_store_basico` con props de cap. 7.
  Pistas: (1) ¿el trait inserta `Node`/`Edge` o sus props por separado?;
  (2) tras borrar el nodo 0, ¿`iter_edges` sigue viendo la arista?; (3) ¿tu
  programita depende de un `MemoryStore` o del `dyn GraphStore`? Criterio:
  distinguir qué vive en el modelo (caps.7) y qué en el puerto (cap.8), y
  detectar la arista colgante antes de la cascada.
- **crear (experto — cierre, retrieval puro)**: implementar desde cero un
  segundo backend mínimo `HashMapStore` que cumpla `GraphStore` (sin
  adyacencias: que `out_edges`/`in_edges` escaneen los `Edge` para
  demostrar que el CONTRATO no dicta la estructura interna) y un test que
  pase la MISMA batería (poblar, recorrer, cascada, duplicado) contra ambos
  `MemoryStore` y `HashMapStore` — prueba de que el código cliente es
  agnóstico. Pistas: (1) `delete_node` debe barrer TODAS las aristas para
  la cascada (no hay `adj_*`); (2) `get_node(&self, id)` sobre un
  `HashMap<NodeId,Node>` — ¿`Option<Node>` o `Option<&Node>`?; (3) si el test
  funciona para el HashMap, demuestras que tu lógica no sabe qué cocina
  hay detrás. Criterio: genérico (contra `&mut dyn GraphStore`) + cascada +
  Duplicate correctos en AMBOS backends.

## 10. Preguntas abiertas (gancho al cap. 9 — y al cap. 10/27)

1. El trait dice QUÉ opera con `Node`/`Edge` de la RAM… pero en disco solo
   hay BYTES. ¿Cómo convierto un `Node`/`Edge` en un `Vec<u8>` sin
   ambigüedad, y qué pasa con el orden de los bytes de un `usize` o un
   `f64` en una máquina big-endian? (Cap. 9: encoding, endianness,
   version intro de `StoreError`/formato.)
2. `MemoryStore` guarda en un `Vec<Option<T>>`. ¿Y si el grafo es 10.000
   veces más grande que la RAM y tengo que ir al disco? ¿Qué aspecto tiene
   un `GraphStore` cuyo "detrás" son páginas de 4 KB? (Caps. 10-11: el mismo
   puerto, otro adapter.)
3. El borrow checker garantiza un solo `&mut` a la vez. En una BD con dos
   procesos, ¿cómo se convierte esa exclusión en "transacción"? (Cap. 27:
   el germen del único escritor.)
- **Términos nuevos de glosario** (los registra `book-memory-keeper`): trait,
  puerto, adapter/backend, arquitectura hexagonal (ports & adapters),
  contrato, que vs cómo, `StoreError`, tombstone (`Vec<Option>`), cascada,
  clave estable, `Result` vs `Option` vs `bool` como semántica.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial predice números ANTES de ejecutar y
  explica la cascada de memoria — recordar el contrato (no reconocerlo); el
  experto reimplementa de memoria un segundo backend y exige que el test
  siga pasando (recuperación estructural del trait completo).
- **Spacing**: el intermedio re-ejercita el modelo de datos del cap. 7
  (`Node::with_prop`, `Value`, `Edge::new`) dentro del puerto nuevo —
  "¿qué vive en el modelo y qué en la API?" es la pregunta de spacing.
- **Interleaving**: el experto mezcla el diseño del trait (cap. 8) con la
  elección de estructura interna (Map vs Vec) y con el modelo de datos
  (cap. 7); el esencial mezcla lectura/escritura con el matiz de firmas.
- **Dificultad asimétrica**: para el CONOCIMIENTO, una sola idea nueva por
  sección (qué es un trait → por qué un contrato → firmas semánticas →
  `Vec<Option>` → cascada → borrow checker). Para la DESTREZA, esfuerzo de
  recuperación en los ejercicios.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap08`
  (4 tests citados por nombre: `memory_store_basico`, `rechaza_duplicado`,
  `delete_node_elimina_aristas`; el tercer ejercicio añade el suyo contra
  dos backends). Feedback Tight, no "confía en mí".
- **Citas**: Cockburn, "Hexagonal Architecture" (2005) / ports-and-adapters
  (la interfaz que desacopla qué de cómo); SQLite, "The VFS layer"
  / `sqlite3_vfs` (la capa de E/S detrás del mismo motor SQL: un puerto que
  permite swap de E/S nativa/OS); Código fuente SQLite `vdbbe.c` + docs
  sobre preparación de statements (cómo las Mismas consultas llevan intents
  por un puerto estable);

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (9 en la tabla §5).
- [x] Escenario de fallo visible: aristas colgantes al borrar sin cascada (`delete_node` que no toca `edges`), `Result` sobre-usado, ids re-numerados, `adj_*` desincronizados — todos con síntoma detectable.
- [x] Código ejecutable en workspace (4 tests ALL_GREEN citados por nombre) que la prosa referencia sin duplicar.
- [x] Misconcepciones corregidas explícitamente (§1: cinco; «la API es el TODO», «Result para todo», «borrar un nodo es borrar un nodo», «Vec<T> basta», «la forma de guardar ES la función»).
- [x] Ejercicios con solución verificable (tests del workspace + predicciones medibles).
- [x] ≥1 ejercicio de retrieval (esencial predice y explica la cascada de memoria; experto reimplementa otro backend) y ≥1 de spacing (intermedio toca el modelo de datos del cap. 7).
- [x] Responde la pregunta crítica del CORPUS para su id («¿Qué métodos debe tener el trait para soportar todos los caps siguientes?») — la §2 y §5 lo responden explícitamente.
- [x] Estructura del cap. 11 (N.0 anécdota → objetivo → problema → modelo mental → primera solución → límites → evolucionada → código → trampas/historia → BBDD real → retos → mini-diálogo → gancho) respetada en el capítulo.
- [x] Citas de alta confianza: Cockburn (hexagonal), SQLite VFS y VDBE. Las cifras son de los tests reales (4 passed).
