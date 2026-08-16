# Capítulo 7 — El modelo de datos de LiraDB (Property Graph + Value)

> *«Un grafo de bits te dice dónde hay un enlace. Un modelo de datos te dice qué significa ese enlace.»*

## 7.0 La anécdota de la esquina

En 1970, Edgar Codd publicó en el *Communications of the ACM* un artículo de once páginas que iba a cambiar la informática: «A Relational Model of Data for Large Shared Data Banks» (CACM, vol. 13, nº 6, pp. 377-387). La tesis era tan sencilla como radical: los datos no deberían guardarse según *cómo se accede a ellos* (archivos, punteros, jerarquías), sino según un modelo matemático —las relaciones, o tablas— que fijara de forma declarativa qué significa cada dato **y qué tipos puede tener**. Antes de Codd, un valor cualquiera era de facto un trozo de texto; después de Codd, nadie serio lo discutía: ordenar, comparar y sumar exigían saber si aquello era un número, una fecha o un booleano.

Años después, el problema reapareció con otro rostro. Las bases de datos relacionales eran soberbias con tablas, pero los datos del mundo real —amistades, caminos, dependencias— son redes: la relación "Ana conoce a Bo (desde 2020)" tiene *cualidades propias* que no caben bien en una tabla y sus claves foráneas. En 2007, Neo4j popularizó el modelo **property graph** y el lenguaje Cypher, y en 2024 la ISO publicó **ISO/IEC 39075** — el estándar *Graph Query Language (GQL)* — reconociendo que el grafo es un modelo de datos de primera clase, con su propio tipado y sus propias aristas con propiedades.

El hilo que une a Codd, Neo4j, GQL y LiraDB es exactamente este capítulo: **un grafo no es solo una colección de aristas; es un modelo de datos**, con tipos de valor, identidades y etiquetas. Vamos a definirlos.

## 7.1 Objetivo

Al terminar este capítulo sabrás responder a una pregunta que quema: **¿qué significa "tener un grafo" dentro de una base de datos?** No basta con dibujar nodos y flechas. Hace falta decidir, con la precisión de un contrato:

1. **Cuántos tipos de valor existen** y qué puede guardar cada propiedad (`Value`).
2. **Qué es la identidad** de un nodo o una arista, y por qué existe aunque no tengan datos (los `id`).
3. **Qué papel juegan las etiquetas** (los `labels`), distintas de las propiedades.
4. **Cómo se organizan nodos y aristas** en memoria para poder recorrer el grafo (`PropertyGraph`).

Vas a construir las cuatro piezas del modelo conceptual que el resto del Vol.II persiste, indexa, consulta y recorre: `Value`, `Node`, `Edge`, `Element` y `PropertyGraph`.

## 7.2 Problema

Retoma el minigrafo de la Parte I: Ana conoce a Bo. Hasta ahora lo pintábamos como una estructura de computación — una lista de adyacencia, una matriz de bits — pensada para *recorrer*. Pero Ana no es solo "el nodo 0" y la flecha no es solo "true". Ana tiene nombre, edad y país. La flecha tiene el año desde el que se conocen. Y el día que guardes esto en un fichero y lo vuelvas a leer meses después, necesitas saber **exactamente** qué es cada cosa.

El problema es que un grafo de "computación" responde bien a *estructura* y fatal a *dato*:

- **La matriz de adyacencia con bits** (la del Vol.I cap. 2) te dice "hay una arista entre 0 y 1" — y nada más. No hay sitio para "desde 2020", ni para la edad de Ana, ni para saber que la arista es de tipo `KNOWS` y no `WORKS_AT`.
- **Los nodos como `HashMap<String, String>`** (la tentación natural) guardan "36" como texto. Ordenar por edad da `1, 10, 2`. Sumar "36" es un error de tipos enmascarado. Comparar funciones es imposible sin decidir antes qué tipo es cada campo.
- **Los `id` como posición en el array** (el atajo) funcionan hasta que borras el nodo del medio y "el índice 2" pasa a significar otra cosa.

La raíz del problema: un grafo de base de datos necesita un **modelo de datos** — la capa que Codd añadió a las tablas en 1970 y que ahora hay que añadir a los grafos — que responda tres preguntas con tipos fuertes: *qué valores guardo, qué identidad tiene cada elemento y qué etiquetas lo clasifican*.

## 7.3 Modelo mental

Piensa en el **archivo de expedientes de un hospital** (de nuevo, como el del cap. 3):

- Cada paciente tiene un **número de expediente** (el `id`). Ese número existe **aunque la ficha esté vacía** o la persona aún no haya entrado. El número no es "la ficha que está en el quinto hueco": es un nombre estable que no cambia si reorganizas la estantería.
- La ficha tiene **campos con tipos predefinidos** (`edad`: número; `activo`: sí/no; `alergias`: lista de texto). Nadie escribe la edad "a mano en tinta borrosa".
- Encima hay una **etiqueta de carpeta** ("Paciente", "Hospital", "Cardiólogo") que *clasifica* — y que puedes mirar sin abrir la ficha. Las **notas internas** (propiedades) *describen* — y exigen abrir la carpeta.
- En cada carpeta, una hoja lista los **expedientes enlazados** y con qué relación: "recibe de" (la arista entrante) y "envía a" (la arista saliente).

El archivo, visto de fuera:

```
Expediente 0 ──[KNOWS, since=2020]──▶ Expediente 1
   labels: ["Person"]                    labels: ["Person"]
   props:                                props:
     name: "Ada"                          name: "Bo"
     age:  Int(36)                         city: "Oporto"
```

Dos ideas son el corazón:

1. **La identidad vive aparte de los datos.** `Ada` "es" el expediente 0 no porque tenga nombre, sino porque su `id` es 0. Por eso un nodo *sin ninguna propiedad* es perfectamente válido.
2. **El tipo de valor es una decisión del modelo.** Que `age` sea `Int(36)` y no `"36"` no es un capricho: es lo que permite comparar, ordenar y ahorrar espacio.

### El momento ¡ajá!

> *La matriz de adyacencia con bits es una hoja de cálculo de "sí/no hay enlace". El modelo de datos es el archivo: dice qué es cada campo, cómo se llama cada elemento y con qué etiqueta se clasifica. Un grafo de BBDD necesita las dos cosas — topología Y significado — conviviendo.*

## 7.4 Primera solución

La versión ingenua (la que probablemente ya estés pensando): **todo es texto, el id es la posición, y las aristas son pares de nombres**.

```rust
// Solución ingenua: "todo son strings".
struct GrafoSimple {
    nodos: Vec<HashMap<String, String>>,  // id = índice en el Vec
    ady: HashSet<(usize, usize)>,          // (origen, destino) como bits
}
```

Define a Ana, guarda sus datos y... los tests pasan. El minigrafo "funciona". Es *exactamente* el grafo de bits y de strings del Vol.I, vestido de base de datos.

## 7.5 Sus límites

La solución ingenua se rompe por los cuatro flancos a la vez:

1. **El tipo viaja donde se guarda, no donde se define.** `age: "36"` se ordena mal, no se suma y confunde a quien lo lea. El momento en que haces `if node.edad_as_texto > "30"` ya es tarde: el hoyo lo cavaste al tipar.
2. **La arista es invisible.** `HashSet<(usize,usize)>` guarda "existe enlace", pero no permite decir que la flecha es `KNOWS`, ni que nació en 2020. Olvídate de ponderar el camino de Dijkstra en el cap. 22: no hay dónde guardar el peso.
3. **La identidad muere con el borrado.** Reubicar el nodo del medio recicla su índice; compáralo con el principio del cap. 3 — "el id estable no se recicla".
4. **No hay etiquetas.** No puedes clasificar a Ada como `Person` *y* `Author` para una búsqueda, porque no existe la idea de "carpeta".

Las cuatro son el mismo fallo de fondo: **no hay modelo de datos** — solo colecciones de bits. Codd tenía razón: hay que tipar, nombrar y clasificar.

## 7.6 Solución evolucionada

La solución (la de Neo4j, la retomada por el estándar GQL ISO 39075) es el **property graph** con cuatro decisiones firmes:

1. **`Value` es una unión tipada** (un enum Rust), no un string camuflado. Cada valor *sabe qué es*.
2. **La identidad es un campo separado**: `NodeId`/`EdgeId` = `usize` por ahora (pedagógico), con la promesa explícita de migrar a IDs generacionales (`slotmap`) en el cap. 3.
3. **Labels vs props**: las etiquetas clasifican (`Vec<String>`, varias por nodo), las propiedades describen (`HashMap<String, Value>`).
4. **La arista es una entidad de primera clase**: con su propio `id`, su `source`, su `target`, su **label de relación** (`String`, el "verbo") y sus propiedades.

Y encima de todo, un **`PropertyGraph`** que guarda nodos y aristas en arrays y mantiene dos índices de adyacencia — `adj_out` y `adj_in` como listas de `EdgeId` — para poder recorrer "quién sale" y "quién entra" sin buscar en todo el grafo. Esta es la estructura de datos que *retiene propiedades* (a diferencia de la matriz de bits, que solo retiene topología).

## 7.7 Código explicado

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap07_modelo.rs` (284 líneas). Recórrelo conmigo, no lo copies: está en tu workspace. Primero, el tipado de valores.

### El corazón: `Value`

```rust
pub enum Value {
    Null,        // ausencia explícita de valor
    Bool(bool),
    Int(i64),    // entero de 64 bits
    Float(f64),  // IEEE 754
    String(String), // UTF-8
    Bytes(Vec<u8>), // opacos, binarios
}
```

Esta es la respuesta a la pregunta crítica del capítulo: *¿cómo tipar `Value` para soportar string/int/bool/list/map?*. Fíjate en lo que *no* está: **no hay `List` ni `Map`** en esta versión. LiraDB empieza por los seis primitivos — `Null` explícito, booleano, dos números, texto y bytes — y deja listas/mapas anidados para un tipo `Value::List`/`Value::Map` cuando el modelado lo pida. Decidir los *seis* no es arbitrario: cubren los que cualquier `Edge`/`Node` real necesitará (el `Null` de Codd, `HashSet`→`Bool`, `Vec`→`Bytes`, fechas→`String`/`Int`), sin la complejidad de la recursión infinita del "map genérico".

Y es **extensible por diseño**: `Value` implementa `Debug, Clone, PartialEq` pero no `Copy` — los strings y bytes son «grandes». Añadir una variante mañana es posible, pero el código lo avisa con un comentario clave: *añadir una variante es un bump de versión del formato (cap. 9)*. Volverás a esto: la extensibilidad del modelo **no es gratis** — cada variante nueva cambia cómo se codifican los datos en disco.

Sobre el `list/map` de la pregunta crítica: merecen un apunte. Un `Value::List(Vec<Value>)` o `Value::Map(HashMap<String, Value>)` es tentador e inevitable algún día, pero introduces **recursión**: un `Map` puede contener listas que contienen maps... y entonces `Value` deja de ser plano y tu `type_name`, tu comparación y tu futura codificación (cap. 9) tienen que decidir la profundidad. La posición de LiraDB es deliberada: **empezar con seis primitivos y añadir el contenedor cuando el modelado lo exija, con su bump de versión**. Apostar por lo mínimo que cubre a los nodos y aristas reales —y migrar con un plan— es mejor que construir una maraña recursiva el día uno. (Comprueba el corolario: algo como una dirección, que "parece" un map, se modela aquí como un `String`; si necesitas buscar por ciudad, lo resolverás con etiquetas o un nodo intermedio, no con `Value::Map`.)

Dos métodos lo hacen útil: `type_name(&self) -> &'static str` (qué variante soy, para depurar) y `is_null(&self)`.

### La identidad: `NodeId` y `EdgeId`

```rust
pub type NodeId = usize;
pub type EdgeId = usize;
```

Dos `type alias`. Y aquí está la nota pedagógica honrada: *en el cap. 3 (Vol.II) se sustituirán por IDs generacionales (`slotmap`)*. Hoy `usize` nos deja hacer `id = índice` y ver la aritmética con claridad; el cap. 3 — el de "Identidad, referencias y datos estables" — demostrará por qué esa aritmética se vuelve peligrosa al borrar y cómo `slotmap` la salva. Guardar los dos alias en un único punto significa que migrar tocará un lugar, no cien.

### `Node`, `Edge` y `Element`

```rust
pub struct Node {
    pub id: NodeId,
    pub labels: Vec<String>,
    pub props: HashMap<String, Value>,
}

pub struct Edge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub label: String,      // el "verbo": KNOWS, WORKS_AT...
    pub props: HashMap<String, Value>,
}
```

- **`Node.labels` es `Vec<String>`** — un nodo puede ser `Person` y `Author` a la vez. El helper `has_label` permite filtrar por categoría sin abrir ninguna propiedad.
- **`Edge.label` es un solo `String`** — es el *tipo de relación*, el verbo de la frase. Que una arista tenga un único label (a diferencia del nodo) es una decisión deliberada: "Ana CONOCE a Bo" tiene un verbo, no tres. (La clasificación compleja se resuelve con nodos intermedios, que verás en modelado.)
- Los builders `Node::new(id, label)`, `.with_prop(key, value)` y `Edge::new(id, source, target, label)` hacen al código legible: construyes una ficha entera en una línea.

### El contraste que debes tener claro: LPG vs RDF

Una pregunta crítica del capítulo es *cuál es la diferencia entre LPG y RDF*. Ambos son "grafos", y es fácil confundirlos. La diferencia práctica y duradera es la **granularidad de la arista**:

- En el **LPG** (lo que construyes hoy), la **arista es un objeto de primera clase**: tiene su propio `id`, su `label` (el verbo `KNOWS`), su `source` y `target`, y sus `props`. "Ana conoce a Bo desde 2020" es una arista con `props["since"]`. Tú acabas de construir eso.
- En el **RDF** (Linked Data / Web Semántica, Vol.III), todo es un **triple** sujeto-predicado-objeto: `Ana KNOWS Bo`. No hay "arista con propiedades" — para decir "desde 2020" necesitas un *nodo* intermedio (reificar el objeto: crear un nodo `conocimiento:anas-bo` y enlazarlo). Es un modelo más puro y distribuible, pero a costa de verbosidad.

GQL (ISO 39075) y Cypher son del mundo LPG: aristas tipadas con propiedades. RDF/SPARQL son del otro. **LiraDB es LPG** — eliges aristas de primera clase. (Este contraste se formaliza en el Vol.III; aquí solo necesitas decidir cuál eliges y por qué.)

### El sum type `Element`

Y el **sum type** que cosifica el elemento:

```rust
pub enum Element { Node(Node), Edge(Edge) }
```

`Element` es la semilla de algo que usarás en capítulos de algoritmos y consultas (cap. 17+): un recorrido genérico que puedes preguntar `id()` sin hacer un `match`. Cuando cap. 20 (modelo Volcano) itere sobre resultados, agradecerás que la unión existiera desde aquí.

### El grafo: `PropertyGraph`

```rust
pub struct PropertyGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub adj_out: Vec<Vec<EdgeId>>,  // aristas que SALEN de cada nodo
    pub adj_in: Vec<Vec<EdgeId>>,   // aristas que LLEGAN a cada nodo
}
```

- **`nodes`/`edges`** guardan los objetos con sus datos.
- **`adj_out[u]`** es la lista de ids de las aristas que salen de `u`; **`adj_in[u]`** la de las que entran. Son la "dos caras" de una arista dirigida — y por guardar *`EdgeId`* (no destinos a pelo) puedes recuperar el `Edge` entero detrás de cada adyacencia, con su label y sus props. *Esta* es la diferencia respecto a la matriz de bits: aquí la vecindad no solo dice "sí hay enlace", dice *cuál*.

El detalle que encierra la sabiduría del capítulo está en `add_node`:

```rust
let duplicado = id < self.nodes.len() && self.nodes[id].id == id;
if duplicado { return false; }
// estirar arrays, rellenando con "veneno":
self.nodes.resize(id + 1, Node::new(usize::MAX, "_placeholder"));
```

Tres lecciones:

1. **`add_node` rechaza duplicados** devolviendo `false` en vez de sobreescribir: el `id` es un nombre estable, no un hueco que se pueda pisar.
2. **El "veneno" `usize::MAX`** distingue un hueco no ocupado de un "agujero" entre ids. Re-insertar el id 3 cuando ya existe el 3 y el 5 llenará el hueco del 4 con un placeholder que «no cuenta como nodo».
3. **`num_nodes` se deriva**, no se lleva en cabeza: cuenta los que *no* son `usize::MAX`. Con Codd y con LiraDB: los invariantes se calculan de los datos, no se memorizan.

`add_edge` comprueba que `source` y `target` existen (mira si el id coincide, no solo si el índice cae fuera), y `neighbors_out`/`neighbors_in` devuelven la vecindad con un `&[]` vacío si el nodo no existe.

## 7.8 Prueba de fuego

La prueba de fuego es que el modelo respira: se tipa, se construye, se puebla y se recorre por ambos lados. El test **`graph_add_and_neighbors`** construye `Ada→Bo` y `Bo→Ada` con aristas `KNOWS`, y comprueba:

- `num_nodes() == 2`, `num_edges() == 2`;
- `neighbors_out(0).len() == 1` y `neighbors_in(0).len() == 1` (y lo mismo para `1`): cada nodo ve su saliente Y su entrante.

Apoya la identidad con dos tests más: **`graph_add_rechaza_duplicado`** (re-insertar el id 0 devuelve `false`, y `num_nodes()` sigue en 1) y **`node_with_props`** (un nodo conserva su `id` y su label aunque tengas que filtrar `has_label`). El código, explícito:

```bash
cargo test -p vol2-liradb --lib cap07_modelo
# 6 tests: value_type_names, node_with_props, edge_basic,
#          graph_add_and_neighbors, graph_add_rechaza_duplicado, element_enum_id
```

Si este capítulo se te olvidara, tus datos serían `HashMap<String,String>`, tus aristas no tendrían propiedades y el capítulo 9 no tendría qué codificar — acabarías inventando "todo-strings" y arrastrándolo toda la obra. Ese es el síntoma: **no puedes ordenar un Int, no puedes pesar un `Edge`, y cada capítulo futuro tropieza con lo mismo**.

## 7.9 Las trampas (ojo, cuidado con…)

- **Confundir `id` con índice.** El id es un nombre estable; el índice es un hueco. El síntoma de mezclarlos: leer `usize::MAX` como un nodo real, o reutilizar un índice tras un borrado. Recuerda el cap. 3.
- **`Value` no es "un string disfrazado".** El momento en que ordenas "36" como texto es tarde. El tipo se decide *aquí*, o se paga un refactor masivo después.
- **Label vs propiedad.** Clasificar va en `labels` (lo consultas en un bucle, sin abrir la ficha); describir va en `props`. Meter la categoría en `props["type"]` obliga a escanear todas las propiedades para filtrar.
- **Extender `Value` como "otra línea más".** Añadir una variante cambia el formato en disco: es un bump de versión (cap. 9). La extensibilidad tiene precio, y hay que planearla.

## 7.10 Lo que te llevas

- Un grafo de BBDD necesita un **modelo de datos**: `Value` (tipos), identidad estable (`id`), y `labels` que clasifiquen.
- **`Value` es una unión tipada** (enum Rust), no "todo strings": `Null/Bool/Int/Float/String/Bytes`, con `type_name`/`is_null`.
- **El `id` existe aunque no haya propiedades**, y `add_node` rechaza duplicados porque la identidad no se sobreescribe.
- **La arista es de primera clase**: `id + source + target + label + props`.
- **`adj_out`/`adj_in` guardan `EdgeId`, no bits**: la vecindad retiene el `Edge` entero — la diferencia con la matriz de bits del Vol.I.
- **`Element` (Node|Edge)** cose la unión que algoritmos y consultas (cap. 17+) usarán.

## 7.11 Una historia pequeña

Cuando empezamos LiraDB, antes de este capítulo, un nodo era un `HashMap<String, String>` y una arista un par de números en un `HashSet`. Funcionó un día. Al siguiente, Ana quiso ordenar a sus contactos por edad y el resultado fue `1, 10, 2` — porque "36" y "102" son textos. Después quiso saber *desde cuándo* conocía a cada uno, y no había sitio en un bitset para "desde 2020". Escribimos el modelo de datos ese mismo fin de semana. La lección no fue "usar enums", fue **"decidir el tipo antes de escribir el dato"**. Codd lo descubrió para las tablas en 1970; nosotros lo redescubrimos para los grafos en nuestra propia mesa de trabajo, 55 años después.

## Ejercicios resueltos

**1. ¿Por qué `Null` es una variante explícita de `Value` y no "un string vacío" o "una columna vacía"?**

Porque `Null` es *significado*, no *ausencia de campo*. Un `Value::Null` te dice "este valor está representado como la ausencia *explícita* de valor" — es una decisión del modelo, la misma distinción que Codd introdujo con el `Null` relacional. `String("")` es una cadena de longitud cero (un dato); `Bytes(vec![])` lo mismo; ninguno es "no hay valor". Y como `Value` define `Null` como constructor, `is_null()` lo distingue de forma exhaustiva (`match` en `Value`), sin adivinar. Compare: una "columna vacía" solo existe en una tabla; en un grafo, que *no exista la clave* en `props` y que exista con `Value::Null` son estados distintos — y el modelo decide que prefieres tener la opción.

**2. En `PropertyGraph`, ¿por qué `add_edge` comprueba `id` y no solo que `source` caiga dentro de `nodes.len()`?**

Porque un hueco rellenado con el placeholder `usize::MAX` *cae dentro* de `nodes.len()` (el array está estirado) pero **no es un nodo real**: su `id` es `usize::MAX`, la firma del "veneno". La comprobación correcta es doble: que `source < nodes.len()` **y** que `nodes[source].id == source` (el id coincide con el índice = hueco ocupado por un nodo legítimo). Si solo miraras el tamaño, permitirías crear una arista que cuelga de un hueco vacío — y el recorrido produciría vecinos fantasma. Es el mismo principio de `num_nodes`: la *verdad* se deriva comprobando, no asumiendo que el array está lleno de extremo a extremo.

## Ejercicios propuestos

**Esencial (recordar).** Sin mirar el código, escribe de memoria las seis variantes del enum `Value` y la función `type_name()`. Luego ejecuta `value_type_names` y compara. Pistas: (1) ¿cuál es la "ausencia"?; (2) los dos números son `i64` y `f64`; (3) ¿qué constructor envuelve un `Vec<u8>`? Criterio: tu `type_name` devuelve exactamente `"Null"`, `"Bool"`, `"Int"`, `"Float"`, `"String"`, `"Bytes"` para cada variante, y el test pasa.

**Intermedio (analizar — spacing Vol.I cap. 2 / Vol.II cap. 4).** Toma el grafo de la prueba de fuego (`Ada→Bo`, `Bo→Ada`). Dibuja `adj_out` y `adj_in` como dos tablas, y explica por qué guardar **`EdgeId`** (no destinos a pelo) en cada lista es lo que conecta el recorrido con el *dato* de la arista (su label y sus props). Verificación: `graph_add_and_neighbors`. Pistas: (1) ¿qué hay dentro de `adj_out[0]`?; (2) para obtener el `Edge`, ¿qué índice necesitas?; (3) ¿qué perderías si guardaras solo `[1]` en vez de `[0]`?

**Experto (crear — interleaving cap. 8).** Añade a `PropertyGraph` el método `neighbors_out_by_label(&self, u: NodeId, label: &str) -> Vec<NodeId>` que devuelva los destinos de las aristas salientes de `u` cuyo label coincida, usando `neighbors_out`, el `Edge` real y un bucle. Escribe un test con dos labels de arista (p.ej. `KNOWS` y `WORKS_AT`) y comprueba que filtra correctamente. Pistas: (1) ¿qué te da `neighbors_out(u)` — ids o el `Edge`?; (2) ¿dónde consultas `edge.label`?; (3) ¿qué haces con `edge.target` para poblar el resultado? (Es exactamente la clase de filtro que el `trait GraphStore` del cap. 8 necesitará por diseño.)

## Para profundizar

- **E. F. Codd**, *A Relational Model of Data for Large Shared Data Banks*, CACM 13(6), 1970 — el modelo de datos y los dominios tipados: la raíz de todo.
- **I. Robinson, J. Webber, E. Eifrem**, *Graph Databases*, 2ª ed., O'Reilly/Neo4j, 2015 — la definición canónica del *property graph*: nodos etiquetados + aristas de primera clase con instrucciones.
- **ISO/IEC 39075:2024**, *Graph Query Language (GQL)* — el estándar que formaliza los property graphs y las aristas con propiedades, aprobado en 2024.
- **N. Francis et al.**, *Cypher: An Evolving Query Language for Property Graphs*, SIGMOD 2018 — cómo el modelo de datos condiciona el lenguaje de consulta.
- Dentro del libro: **cap. 3** (por qué `usize` migrará a `slotmap`), **cap. 9** (el encoding de `Value` y su versionado), **cap. 8** (el puerto `GraphStore` que este modelo alimentará), Vol.I **caps. 2 y 4-5** (las representaciones y la matriz de adyacencia que contraste).

## Mini-diálogo: en guardia nocturna

> — O sea, que "modelo de datos" es... ¿decidir que el id no es la posición y que un Int no es un string?
>
> — Exacto. E "identidad separada de los datos" significa que el `id` existe aunque la ficha esté vacía. Todo lo demás —decidir que una arista tiene label y props, que puedes recorrerla por los dos lados, que los duplicados se rechazan— cuelga de esas dos preguntas: *qué tipo tiene cada valor* y *qué es lo que se nombra*.
>
> — Pero un bitset "hacia dónde enlaza" era tan simple...
>
> — Simple y mudo. La matriz de bits te dice que hay un enlace; este modelo te dice *cuál* es, *de qué tipo*, y *desde cuándo*. Un grafo de bits es una topología; un property graph es un modelo de datos que *además* tiene topología. Cuando mañana quieras pesar una arista para Dijkstra, entenderás qué habría pasado con el bitset.

---

*(Próximo capítulo: 8 — Diseñar una API antes de persistir (trait `GraphStore`). Aquí el modelo existía como estructura; ahora veremos cómo exponerlo para que el resto del motor cree, mire y recorra grafos sin conocer su representación interna.)*
