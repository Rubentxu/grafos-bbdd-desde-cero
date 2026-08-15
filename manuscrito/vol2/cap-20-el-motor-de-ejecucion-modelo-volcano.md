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
