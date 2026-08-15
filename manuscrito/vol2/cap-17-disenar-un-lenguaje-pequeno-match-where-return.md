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
