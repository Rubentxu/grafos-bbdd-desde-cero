# CONTRATO DE CAPÍTULO — Vol.II Cap. 7: El modelo de datos de LiraDB (Property Graph + Value)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap07_modelo.rs` (284 líneas, 6 tests en
> el módulo `tests` verdes con `cargo test -p vol2-liradb --lib cap07_modelo`:
> `value_type_names`, `node_with_props`, `edge_basic`,
> `graph_add_and_neighbors`, `graph_add_rechaza_duplicado`,
> `element_enum_id`). Este capítulo ABRE el cuerpo conceptual de la Parte II
> (después del cap. 6 de transición) y define `Value`, `NodeId`/`EdgeId`,
> `Node`, `Edge`, `Element` y `PropertyGraph`: el modelo de datos que el resto
> del Vol.II persiste, indexa, consulta y recorre. NO tiene MIGRATION-PATTERN
> propio (es de los primeros, del bootstrap). Ganchos: cap. 8 (`GraphStore`
> como puerto) y cap. 9 (encoding/versionado). Preguntas críticas del CORPUS
> (id `vol-II-cap-07`): «¿Cómo tipar `Value` para soportar string/int/bool/
> list/map?» y «Diferencia entre LPG y RDF». Código real en workspace del
> volumen: `vol2-construye-liradb.md` y `liradb-workspace/`.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: qué es un grafo (nodos + aristas, dirigido o
  no) (Vol.I caps. 1-2; Vol.II cap. 1); las tres representaciones — edge list,
  adjacency list, CSR — y cuándo elegir cada una (Vol.I cap. 2 / Vol.II
  cap. 2); que la identidad de un elemento NO es su posición ni su contenido
  y la cuestión de `slotmap`/IDs generacionales (Vol.II cap. 3); BFS sobre
  CSR vs sobre HashMap de listas (Vol.II cap. 4); que una "biblioteca de
  grafos" no es una "base de datos de grafos" — la diferencia es la
  persistencia y el catálogo (Vol.II cap. 6); la sintaxis base de Rust
  (structs, enums, `match`, `HashMap`, `Option`, `Vec`) y el `cargo test`
  del workspace (`vol2-liradb`).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «el
  modelo de datos es trivial: cada nodo es un `HashMap<String, String>`» —
  no: "todo son strings" rompe la comparación, el orden numérico, los rangos
  y el ahorro de espacio; hace falta una unión tipada; (2) «un id ES la
  posición del nodo en el array» — no: el id es un nombre que existe AUNQUE el
  nodo no tenga propiedades ni exista aún el hueco (por eso `add_node`
  distingue "ocupado" de "vacío" con `usize::MAX`); (3) «la etiqueta y las
  propiedades son lo mismo» — no: los labels clasifican (jerarquía de tipos),
  las props describen (datos); lo primero es estructural y se consulta en
  bucle, lo segundo es opaco; (4) «un grafo de BBDD guarda las aristas en
  una matriz de adyacencia con bits» — no: esa representación sirve para
  *analizar* (Vol.I) pero no para *conceder propiedades* a nodos/aristas ni
  para *recuperar* el elemento por su identidad.
- **NO debe saber todavía**: la API de persistencia (`trait GraphStore`, cap.
  8); el encoding a bytes, endianness y versionado de `Value` (cap. 9); las
  páginas/slotted pages (cap. 11); el CSR persistente (cap. 14); RDF/RDF-star
  en detalle (se NOMBRA como contraste LPG-vs-RDF y se corta; se desarrolla en
  Vol.III). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: `Value` (enum tipado: Null/Bool/Int/Float/String/Bytes) con
  `type_name()`/`is_null()`; `NodeId`/`EdgeId` = `usize` (con nota explícita:
  serán IDs generacionales `slotmap` en el cap. 3); `Node` (id + `Vec<String>`
  labels + `HashMap<String,Value>`, `new`/`with_prop`/`has_label`); `Edge`
  (id + source/target + label de relación + props); `Element` (sum type
  Node|Edge, `id()`); `PropertyGraph` (Vec<Node> + Vec<Edge> + `adj_out` +
  `adj_in`, `add_node`/`add_edge`/`neighbors_out`/`neighbors_in`/`num_nodes`/
  `num_edges`).
- `practice`: adjacency list (Vol.I cap. 2 / Vol.II cap. 2 → aquí como
  `adj_out`/`adj_in` de `EdgeId`); identidad separada de los datos (Vol.II
  cap. 3); recorrido por listas de adyacencia (BFS, Vol.II cap. 4); tipos y
  enums en Rust.
- `consolidate`: la definición de grafo (existe topología y, ahora,
  propiedades); el principio «derivar, no llevar en cabeza» (nótese cómo
  `num_nodes` se DERIVA contando los que no son `usize::MAX`).
- `out_of_scope` (solo nombrar): RDF/RDF-star y SPARQL (Vol.III); encoding y
  versionado de `Value` (cap. 9); almacenamiento en disco (cap. 11); el puerto
  de acceso a datos (cap. 8).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué un grafo de BBDD necesita un modelo de
  datos (tipos de valor, identidades, etiquetas) y por qué una matriz de
  adyacencia con bits no basta para *persistir y conceder propiedades*; (2)
  enuncia qué es `Value` — una unión tipada (enum Rust), no «todo strings» —
  y por qué las seis variantes; (3) explica por qué la identidad existe
  AUNQUE el nodo no tenga propiedades y por qué `usize` hoy (y sus límites que
  resuelve `slotmap` en el cap. 3); (4) distingue label (tipo estructural,
  consultable) de propiedad (dato opaco); (5) compara LPG con RDF (diferencia
  de granularidad: aquí las aristas son entidades de primera clase, no
  triples con sujeto/objeto-anónimo).
- **Skills**: (1) construir un `PropertyGraph` completo (nodos con labels y
  props tipadas, aristas dirigidas con props) y recorrer adyacencias salientes
  y entrantes; (2) detectar la reutilización de IDs (duplicado → `add_node`
  devuelve `false`) y razonar sobre el placeholder invisible que mantiene la
  aritmética de índices.
- **Wisdom**: (1) decide que la extensibilidad de `Value` no es gratis: añadir
  una variante es un bump de versionado del formato (cap. 9), no un cambio
  local; (2) prefiere una representación que conserve propiedades sobre una
  que solo encode la topología, incluso si la matriz de adyacencia es "más
  simple".

## 4. Modelo mental

- **La ficha del paciente donde el número de expediente vive separado del
  contenido de la ficha**. El número de expediente (id) existe aunque la
  ficha esté vacía; el tipo de datos de cada campo está predefinido (no "todo
  escrito a tinta borrosa"); la etiqueta "Paciente"/"Hospital" es la carpeta
  (clasificación), las notas son las propiedades (contenido). El archivo
  guarda las carpetas (nodos) en estanterías (arrays) y, en cada carpeta, una
  lista "estas fichas están enlazadas con esta" (aristas). La clave: el número
  de expediente NO es "la 5ª ficha de la estantería" — es un nombre estable.
- **Diagramas ASCII**: (a) `(Ada, id=0) -[KNOWS, since=2020]-> (Bo, id=1)`
  como LPG con etiquetas y props; (b) el enum `Value` como cajón con seis
  compartimentos; (c) `adj_out`/`adj_in` como el índice "envíos/recepciones"
  de un burofax; (d) matriz de adyacencia con bits vs LPG (topología pura vs
  topología + datos).
- **Momento ¡ajá!**: «El id existe aunque no haya datos, y las etiquetas
  clasifican mientras las propiedades describen. Elegir UN único tipo "string"
  para todo es elegir no poder ordenar, comparar ni ahorrar. Por eso `Value`
  es un enum: el tipo es una decisión del modelo, no un accidente».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap07_modelo.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | `Value` como enum tipado (Null/Bool/Int/Float/String/Bytes), no `HashMap<String,String>` | "Todo son strings" rompe orden numérico, rangos, comparación y ahorro de espacio; una unión tipada conserva el tipo Y permite bajar a bytes compactos (diferencia con los nulos de Codd: aquí `Null` es un valor EXPLÍCITO del lenguaje, no ausencia de columna) | `enum` con `Nan`/`Inf` descartados de `Float`? — no: IEEE 754 los permite; guardar tipos como strings paralelas (`tag` aparte) — coste: desincronización tag/dato | Ordenar `1`,`10`,`2` lexicográficamente; un `Int` que no suma; espacio de texto | doc del enum (líns. 13-31); tests `value_type_names` (226-231); Codd 1970 (nulos/dominios) |
| 2 | `NodeId`/`EdgeId` = `usize` con nota explícita de ids generacionales en cap. 3 | Pedagógico: la aritmética de índices es transparente al novato; se declara en un `type alias` UNA vez, de modo que migrar a `slotmap` toca un solo punto (diferencia con el cap. 3 que YA justifica `slotmap`) | `slotmap`/generacional ya aquí — coste: opacidad pedagógica y romper escrutinio; UUID `String` — coste: 32 bytes + hashing por id | Índices reciclados (ABIBA problem) el día que se borre | comentario líns. 3-6; Vol.II cap. 3; CORPUS cap-03 «Generational IDs: por qué slotmap» |
| 3 | `add_node` rechaza duplicados y estira arrays con placeholder `usize::MAX` | La identidad es un nombre independiente del índice: un hueco `id` no ocupado debe distinguirse de uno vacío; `usize::MAX` marca "veneno"; `num_nodes` se DERIVA contando los no-venenos | Insertar siempre en posición `id` sin comprobación — coste: sobreescritura silenciosa de otro nodo | Dos nodos con el mismo id y datos mezclados | `add_node` (153-173); `num_nodes` (210-212); test `graph_add_rechaza_duplicado` (270-275) |
| 4 | `Vec<String>` para labels de nodo (múltiples) y `String` para la arista (una clase de relación) | Un nodo pertenece a varias categorías simultáneas (`Person`, `Author`); una arista tiene UN tipo de relación (`KNOWS`) que es el verbo de la frase | Label único por nodo — coste: categorías cruzadas imposibles; label de arista como lista — coste: semántica confusa | No poder filtrar "Person & Author"; `Element` no sabría de qué arista habla | `Node.labels` (58-59), `Edge.label` (90); `has_label` (79-81); test `node_with_props` (234-242) |
| 5 | `Element` como sum type (Node\|Edge) con `id()` | Un recorrido genérico que no sabe qué devuelve puede preguntar `id()` sin `match`; es la semilla del `match` masivo que cap. 17+ necesitará al proyectar resultados | Una sola struct `Element` con discriminante a mano — coste: campos inválidos por combinación; dos funciones extra | Cosificar el recorrido; código duplicado por nodo/arista | `Element` (113-126); test `element_enum_id` (278-283) |
| 6 | `adj_out`/`adj_in` como listas de `EdgeId` (adyacencia), no matriz de bits | La matriz de adyacencia con bits sirve para *analizar* (Vol.I caps. 4-5) pero aquí los `EdgeId` apuntan a `Edge` (props, label); el grafo es DIRIGIDO y etiquetado: el índice separa directorio de "hacia dónde" | Matriz `bool[ N ][ N ]` — coste: sin propiedades, sin labels, O(N²) memoria; CSR — se introduce persistente en cap. 14 (esta es la versión en memoria) | No poder obtener el `Edge` detrás de la adyacencia; no poder pesarla | `PropertyGraph` (135-143); tests `graph_add_and_neighbors` (254-267) |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: el novato representaría cada elemento como un `HashMap<String,
  String>` suelto y las aristas como pares `(String, String)`; o —peor— como
  una matriz de adyacencia de bits como en el Vol.I. Funciona para leer
  topología y nada más.
- **Qué la rompe**: guardar `age = 36` como `"36"` impide `num > 30` numérico;
  borrar un nodo del medio reutiliza su índice para otro (identidad
  reciclada); no hay forma de ponerle propiedades a una arista (la matriz solo
  dice "sí/no hay enlace"); los nodos no pueden tener dos etiquetas.
- **Evolución visible**: el código del capítulo introduce `Value` (tipos
  fuertes), `NodeId`/`EdgeId` (identidad estable, hoy `usize`), `Node`/`Edge`
  con props y labels, `Element` (unión), y `PropertyGraph` con `adj_out`/
  `adj_in` que SÍ permiten recuperar el `Edge` detrás de cada adyacencia.

## 7. Prueba de fuego

- **TEST-TESIS** `graph_add_and_neighbors`: un micro-grafo `Ada→Bo` y `Bo→Ada`
  con la arista `KNOWS`; comprobar `num_nodes()==2`, `num_edges()==2` y que
  `neighbors_out`/`neighbors_in` de ambos nodos ven 1 arista cada uno. Es el
  circuito completo: tipar `Value`, construir `Node`/`Edge`, poblar el grafo
  y leer adyacencia desde ambos lados.
- **TEST-IDENTIDAD** `node_with_props` + `graph_add_rechaza_duplicado`: un nodo
  sin propiedades conserva su `id` (la identidad no depende de los datos) y
  re-insertar el mismo id es rechazado (`false`), demostrando que el id es un
  nombre, no un hueco a sobreescribir.
- **Síntoma si el lector se salta el capítulo**: sus nodos son
  `HashMap<String, String>` que no ordenan números, sus aristas no tienen
  propiedades, y el cap. 9 (encoding) no tendría un `Value` tipado que codificar
  — el lector acabaría inventando su propio "todo-strings" y arrastrándolo toda
  la obra.

## 8. Trampas y errores comunes

1. **Confundir id con índice**: `id` es un nombre estable; `pos` es un hueco.
   El síntoma de mezclarlas es el test de duplicados o el placeholder
   `usize::MAX` leyéndose como si fuera un nodo real en `num_nodes`.
2. **Creer que `Value` es solo "un string disfrazado"**: en cuanto ordenes o
   compares, la falta de tipo te muerde. Añadir tipado al revés (después de
   miles de líneas) es un refactor masivo — mejor decidirlo AHORA.
3. **Mezclar label con propiedad**: poner la categoría en `props["type"]`
   obliga a escanear todas las props; el label vive en `labels` por una razón.
4. **Añadir variantes a `Value` sin planificar el versionado**: es un cambio
   del formato; el cap. 9 explica el bump de versión. No es "otra línea más".
- **Precisión de lenguaje (glosario)**: *label* (etiqueta/clasificación
  estructural) vs *propiedad* (`props`, dato opaco); *id* (identidad estable)
  vs *índice* (posición); *LPG* (label+property graph, aristas de primera
  clase) vs *RDF* (triple sujeto-predicado-objeto, sin labels en ese sentido);
  *Null* como variante EXPLÍCITA del lenguaje de valores (no es "falta de
  columna").

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval)**: SIN mirar el código, escribe de
  memoria los seis constructores del enum `Value` con la función
  `type_name()` que devuelve el nombre de cada uno; luego compáralo ejecutando
  el test `value_type_names`. Pistas: (1) ¿cuál es la variante para "sin
  valor"?; (2) ¿qué tipo de float?; (3) ¿qué longitud tiene cada constructor?
  Criterio: `<value>.type_name()` devuelve el `&'static str` exacto y el test
  pasa.
- **analizar (intermedio — spacing Vol.II caps. 2/4)**: sobre el grafo de la
  prueba de fuego (`Ada→Bo`, `Bo→Ada`), dibuja la lista de adyacencia y resume
  cómo `adj_out`/`adj_in` son la "dos caras" de una arista dirigida, y asígnales
  a cada cara el respaldo que ya existe en el cap. 2 (edge list vs adjacency
  list). Verificación: `graph_add_and_neighbors`. Pistas: (1) ¿qué guarda
  `adj_out[0]`?; (2) ¿y `adj_in[1]`?; (3) ¿por qué ambos necesitan saber el
  `EdgeId`, no solo el destino?
- **crear (experto — interleaving cap. 8)**: define `impl PropertyGraph { fn
  neighbors_out_by_label(&self, u: NodeId, label: &str) -> Vec<NodeId> }` que
  devuelva los destinos de las aristas salientes de `u` cuyo label coincida
  con el argumento, usando `neighbors_out`, el `Edge` y un bucle `match`.
  Pistas: (1) ¿qué te da `neighbors_out(u)` — ids o el `Edge`? (2) ¿dónde
  filtras por label? (3) ¿qué haces con el `target`? Criterio: test propio que
  filtre `KNOWS` sobre un grafo con aristas de dos labels.

## 10. Preguntas abiertas (gancho al cap. 8)

1. Ya tenemos el modelo de datos… ¿quién se lo da al resto del sistema? El
   capítulo NO dio forma a la VISTA de acceso: qué métodos debe exponer un
   motor para que cualquiera (y los capítulos siguientes) pueda crear, mirar
   y recorrer grafos SIN conocer la representación interna. (Nace el trait
   `GraphStore`, cap. 8.)
2. Un `Value` en memoria es un enum… ¿qué aspecto tiene cuando sale a disco?
   ¿Cómo sabrás, al volver a leer bytes, cuál de las seis variantes era, y
   en qué orden, y cómo añadir una séptima sin romper lo guardado? (Encoding,
   endianness y versionado: cap. 9.)
3. RDF vs LPG es un contraste que este capítulo solo nombra… ¿y si quieres
   interoperar con el mundo semántico? (Vol.III, modelado RDF/OWL/SHACL y
   Cypher/GQL/SPARQL/Gremlin.)
- **Términos nuevos de glosario**: property graph / LPG, Value (unión tipada),
  Null, label vs propiedad, id estable, adjacency in/out, Element, NodeId/
  EdgeId, RDF (nombrado), triple (nombrado).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial pide re-escribir el enum `Value` y
  `type_name()` DE MEMORIA y verificar contra el test — recordar, no reconocer.
- **Spacing**: los ejercicios intermedio y experto re-ejercitan la adjacency
  list del Vol.I cap. 2 / Vol.II cap. 2 y el BFS por listas del cap. 4
  (recorrido) ; el apartado de identidad toca el cap. 3 (slotmap) con la nota
  de `usize` hoy.
- **Interleaving**: el experto mezcla el modelo de datos (7) con la forma de
  la futura API de consulta (cap. 8); el intermedio mezcla representaciones
  (cap. 2) con recorrido (cap. 4).
- **Dificultad asimétrica**: una idea nueva por sección (qué es un modelo de
  datos → identidad → Value → Node/Edge → Element → PropertyGraph); los
  ejercicios exigen recordar el enum y razonar sobre adyacencia.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb --lib
  cap07_modelo` (6 tests citados por nombre); el experto añade su propio test
  contra el mismo módulo.
- **Citas**: Codd, «A Relational Model of Data for Large Shared Data Banks»,
  CACM 13(6), 1970 (dominios tipados y Null como trasfondo); Robinson,
  Webber & Eifrem, «Graph Databases» 2ª ed., O'Reilly/Neo4j, 2015 (el modelo
  property graph: nodos etiquetados + aristas de primera clase); ISO/IEC
  39075:2024 (GQL, el estándar para property graphs y aristas con
  propiedades) y Neo4j Cypher (Francis et al., SIGMOD 2018) como referencia
  del mundo real; ácua de refs Vol.II caps. 1-6 y Vol.I caps. 1-2/4-5.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (6 en la tabla §5).
- [x] Escenario de fallo visible: ordenar `"1","10","2"` como strings; identidad reciclada tras borrado; aristas sin propiedades (matriz de bits).
- [x] Código ejecutable en workspace (6 tests ALL_GREEN, verificados) citado por nombre y línea, no duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: cuatro; «no todo es strings», «id ≠ índice», «label ≠ propiedad», «la matriz de bits no basura para una BBDD»).
- [x] Ejercicios con solución verificable (tests del workspace; el experto añade test propio).
- [x] ≥1 ejercicio de retrieval (re-escribir `Value` de memoria) y ≥1 de spacing (caps. 2/4 re-ejercitadas; nota slotmap al cap. 3).
- [x] Responde las preguntas críticas del CORPUS (tipar `Value` string/int/bool/list/map; diferencia LPG vs RDF) y encaja el cap. 7 que ABRE el modelo de la Parte II.
- [x] Anécdota verificada con fuente: Codd conel ACM SIGFIDET 1969/1970 y la evolución Neo4j/Cypher → GQL ISO 39075 (aprobado 2024) documentada; el model relacional como contraste.
- [x] Los 6 tests citados coinciden con los del módulo real (284 líneas, `cap07_modelo.rs`).
