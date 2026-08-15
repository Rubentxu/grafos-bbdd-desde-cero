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
