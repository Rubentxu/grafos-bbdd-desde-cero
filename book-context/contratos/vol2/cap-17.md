# CONTRATO DE CAPÍTULO — Vol.II Cap. 17: Diseñar un lenguaje pequeño (MATCH-WHERE-RETURN mini)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap17_liraql_ast.rs` (970 líneas, CERO
> dependencias externas — diseño puro). Tests: 41 en el módulo `tests_query`
> (`cap18_lexer_parser.rs:2007`): construyen los AST **a mano**, sin parser, porque
> en este capítulo no existe — eso es exactamente el tema del capítulo. Decisiones y
> bugs reales: `MIGRATION-PATTERN.md` §21 (cap 17) y §22 (los DOS gaps que el cap. 18
> detectó en este diseño: `TokenKind::Dash` y `Expression::Variable` — el diseño
> sometido a la prueba de la implementación). ABRE la Parte IV «Consultar el grafo»
> (línea 30 de `manuscrito/vol2/tabla-de-contenidos.md`).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el modelo de datos `Value` (Null/Bool/Int/Float/
  String/Bytes, cap. 7) y el property graph con nodos/aristas/propiedades; el patrón
  «contrato antes que implementación» ya visto dos veces (trait `GraphStore` cap. 8,
  trait `Pager` cap. 12); errores tipados con `Display` + `std::error::Error` (caps.
  12-16); la disciplina «primero a mano, luego crates» (todo el Vol.II).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «diseñar un
  lenguaje ES escribir el parser» — falso: primero se fija vocabulario (tokens),
  gramática (EBNF), estructura (AST) y diagnósticos (errores con posición); el parser
  es mecánica derivada (cap. 18); (2) «para consultar basta una API de funciones
  encadenadas» — funciona, pero mata al optimizador (fija el CÓMO, no declara el
  QUÉ), no es texto (no CLI/logs/issues del cap. 31) y exige compilar Rust;
  (3) «un error de consulta es un error» — confunde error **sintáctico** (cap. 18,
  se aborta en el primero) con error **semántico** (cap. 17, se reportan TODOS);
  (4) «el AST es una traducción 1:1 del texto» — falso: canonicaliza (`Display`
  normaliza paréntesis/whitespace) y añade información que el texto no tiene
  (`Span`, `RelDirection`).
- **NO debe saber todavía**: cómo se escribe un lexer o un parser descendente (cap.
  18), el plan lógico/binder (cap. 19), el executor Volcano (cap. 20), el optimizador
  y `liradb explain` (cap. 21), DML (CREATE/MERGE/DELETE, cap. 31), WITH/OPTIONAL
  MATCH/recursión (fuera de LiraQL). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: lenguaje **declarativo** (QUÉ vs CÓMO) y su pacto con el optimizador;
  frontend de un lenguaje como pipeline texto → tokens → AST → validación; gramática
  **EBNF** como contrato escrito ANTES del código; reglas y precedencia por niveles
  (`or_expr < and_expr < not_expr < comparison < primary`); `TokenKind` (34
  variantes: 10 keywords, 4 léxicos, 13 puntuación/flechas, 6 comparadores, `Eof`);
  `Token { kind, span }`; AST (`Expression` con `Literal`/`Variable`/
  `PropertyAccess`/`Compare`/`And`/`Or`/`Not`, `NodePattern`, `RelationshipPattern`,
  `RelDirection`, `PathPattern`, `MatchClause`/`WhereClause`/`ReturnClause`/
  `ReturnItem`, `AstNode`, `Query`); **Span** semiabierto `[start, end)` en bytes
  UTF-8 en TODO el AST; validación semántica de alcance de variables
  (`validate() → Vec<QueryError>` con `kind + span`); `Display` canónico con
  round-trip; patrón ASCII-art de Cypher vs JOINs de SQL.
- `practice`: el enum `Value` del cap. 7 (envuelto por `Expression::Literal`);
  errores tipados con `Display`/`Error` (caps. 12-16); «contrato antes que adapter»
  (cap. 12); round-trip como prueba de fidelidad (encode/decode del cap. 11 →
  display/parse del 18).
- `consolidate`: «definir el contrato primero»; «derivar, no duplicar» (reusar
  tipos); «los mensajes de error son producto».
- `out_of_scope` (solo nombrar): lexer/parser (18), binder/plan lógico (19),
  Volcano (20), optimizador/explain (21), LIMIT/ORDER BY (ejercicio experto),
  DML (31), tipado estático de expresiones (`LogicalType`, cap. 19).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué un lenguaje declarativo y no una API,
  nombrando qué gana el optimizador del cap. 21 (reordenar sin cambiar significado);
  (2) lee la EBNF de LiraQL y predice qué consultas acepta y cuáles no; (3) enuncia
  las 6 reglas de `validate()` y las mapea a sus `QueryErrorKind`; (4) dice por qué
  `Span` va en TODO el AST y qué pierde el usuario sin él; (5) explica por qué
  `Literal` envuelve `Value` del cap. 7 y qué variante NO tiene literal (`Bytes`).
- **Skills**: (1) construir a mano (sin parser) el AST de una consulta
  MATCH-WHERE-RETURN usando los constructores ergonómicos (`Expression::lit/prop/var`,
  `NodePattern`, `RelationshipPattern`) y verificarlo con `validate()` y `Display`;
  (2) predecir la lista exacta de `QueryError` (kinds + spans + orden) de un AST
  defectuoso.
- **Wisdom**: (1) decide cuándo `Vec<Error>` (validación humana, reportar todo)
  frente a `Result` (operación de máquina, abortar) — y por qué el parser del cap.
  18 elige `Result`; (2) decide qué recortar de un lenguaje pequeño (por qué LiraQL
  rechaza `()` vacío aunque Cypher lo permita) y qué precio paga cada recorte.

## 4. Modelo mental

- **Diseñar un lenguaje = diseñar el menú de un restaurante**: la **carta** = los
  tokens (los platos que EXISTEN: MATCH, `->`, `<>`, literales…); la **comanda** =
  la gramática (cómo se combinan: MATCH primero, WHERE opcional, RETURN al final —
  no se sirve el postre antes del entrante); el **camarero** = `validate()` (cuando
  pides algo que no está, no dice «no» a secas: dice QUÉ no está y DÓNDE lo estás
  señalando — `kind + span`); la **cocina** = caps. 18-21 (aún no existe: hoy solo
  diseñamos el restaurante sobre papel); la **comanda reescrita en limpio** = el
  `Display` canónico (misma orden, forma normalizada, lista para archivarse).
- **Diagramas ASCII**: (a) pipeline del frontend con sus capítulos (texto → [18:
  lexer] tokens → [18: parser] AST → [17: validate] errores|AST válido → [19-21]);
  (b) árbol AST de la consulta emblema `(p:Person)-[:KNOWS]->(f:Person)` con los
  spans dibujados encima del texto fuente; (c) jerarquía de precedencia como
  escalera de funciones (una per regla — la firma del cap. 18).
- **Momento ¡ajá!**: «la gramática es el contrato y el parser es un empleado más
  que lo firma» — el cap. 18 derivará UNA FUNCIÓN POR REGLA sin rediseñar ni un
  tipo, y cuando el diseño falló (faltaba `Dash`), el fallo se localizó en el
  CONTRATO, no esparcido por la implementación.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap17_liraql_ast.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Lenguaje **declarativo** (MATCH-WHERE-RETURN) y no API de funciones | El usuario declara QUÉ quiere, no CÓMO recorrerlo: deja al optimizador (cap. 21) reordenar (push-down, índices) sin cambiar el significado. Es el argumento de Codd 1970: navegación automática vs navegación programada | Builder API encadenada: compila y es type-safe, pero congela el orden de ejecución en la cadena de llamadas y no es texto | El optimizador del cap. 21 no tendría nada que reordenar: cada consulta ES su plan | Codd, «A Relational Model of Data…», CACM 1970; MIGRATION-PATTERN §21-1 |
| 2 | Sintaxis de **paréntesis y flechas** de Cypher, no JOINs de SQL | El patrón SE DIBUJA: `(a)-[:KNOWS]->(b)` es el subgrafo en ASCII-art; el equivalente SQL (self-JOINs + where de claves foráneas) esconde la forma del grafo tras columnas | Sintaxis tipo SQL (`FROM nodes JOIN edges ON…`): correcta pero el usuario traduce mentalmente grafo→tablas en cada consulta | El usuario escribe joins de tablas en vez de patrones: la consulta deja de parecerse a la pregunta | Francis et al., «Cypher: An Evolving Query Language for Property Graphs», SIGMOD 2018 («ASCII-art graph pattern matching»); Wikipedia: Cypher, invención de Andrés Taylor en Neo4j, 2011 |
| 3 | Diseñar **tokens/gramática/AST ANTES** del lexer/parser | Diseño como contrato: fija el vocabulario y la forma para que el cap. 18 sea mecánica pura (una función por regla). Mismo principio que el trait `Pager` (cap. 12) antes que `FilePager` | Escribir el lexer primero y «ir descubriendo» el AST: los tipos se rediseñan a mitad de implementación | El cap. 18 rediseñaría tipos en plena mecánica; los tests del diseño serían imposibles antes del código | Hito del brief `pub enum AstNode { Match, Where, Return }` (doc-comment del código, líneas 25-32 y 547-569); MIGRATION-PATTERN §21-1 y lección §22-2 |
| 4 | **`Span` en TODO el AST** (rango semiabierto `[start, end)`, bytes UTF-8) | Los errores apuntan al carácter exacto, estilo rustc/miette/codespan-reporting: `kind` dice QUÉ, `span` dice DÓNDE. Si el span se rellena gratis en el lexer (cap. 18) y se propaga con `merge`, el coste es mínimo | Errores sin posición: «unexpected token» — el usuario rastrea la consulta a ojo | Diagnósticos inútiles en consultas de 10 líneas: UX de lenguaje rota | Código: `Span { start, end }` + `at/new/merge`; `QueryError { kind, span }`; tests `query_error_display_incluye_span`, `query_error_display_span_vacio_muestra_offset` |
| 5 | `Expression::Literal` **envuelve el `Value` del cap. 7** | Un solo modelo de datos: lo que el lenguaje compara/escribe es lo que el grafo guarda. El executor (cap. 20) opera sobre los mismos tipos | Enum `Literal` duplicado (Int/Float/String/…): dos sistemas de tipos que divergen y una conversión en cada frontera AST→datos | Bugs de conversión silenciosa Int/Float/String entre lenguaje y almacenamiento | MIGRATION-PATTERN §21-4 y lección §21-5; `use crate::cap07_modelo::Value` (línea 1 del fichero) |
| 6 | `validate() → Vec<QueryError>` y no `Result` | UX de lenguaje: el usuario quiere ver TODOS los errores de su consulta de una vez, no iterar ciclos corrige-recompila por cada fallo | `Result<Query>`: aborta en el primer error (el patrón de los caps. 12-16, correcto para operaciones de máquina) | Una consulta con 3 fallos exige 3 rondas de validación | MIGRATION-PATTERN §21-6 y lección §21-3; tests `validate_variable_duplicada_entre_nodo_y_arista`, `validate_variable_desconocida_en_where` |
| 7 | **`Display` canónico** con round-trip para TODO el AST | (a) tests del cap. 18: comparar AST esperado vs parseado; (b) semilla de `liradb explain` (cap. 21: mostrar la consulta normalizada); (c) idempotencia `display(parse(display(x))) == display(x)` | No tener Display: cada consumidor escribe su propio pretty-printer (derivan) | Tests frágiles comparando Debug; explain sin forma normalizada | Comentario del código (líneas 779-787); test `display_query_completa_round_trip_canonico`; lección §22-5 |
| 8 | **Gramática EBNF escrita primero** (doc-comment del fichero) | Es la especificación ejecutable en papel: cada regla → una función del parser (cap. 18); los niveles de precedencia (`or_expr → and_expr → not_expr → comparison → primary`) eliminan la ambigüedad ESTRUCTURALMENTE | Gramática plana `expression ::= expression OR expression | …`: ambigua (`a OR b AND c` tiene dos árboles) | Parser con ambigüedad: mismo texto, distintos significados según el día | EBNF líneas 38-60 del código; MIGRATION-PATTERN §22-5 (una función por regla); Wirth, «Compiler Construction» |
| 9 | `TokenKind` deriva `PartialEq` pero **NO `Eq`** | Contiene `Float(f64)` y `f64` no implementa `Eq` (NaN ≠ NaN): los lexemas flotantes no tienen igualdad total. Es correcto, no un descuido | `#[derive(Eq)]`: error de compilación real (bug documentado) | — (el compilador lo impide: la anécdota del capítulo) | MIGRATION-PATTERN §21 bug 1 y lección §21-1 |
| 10 | `AstNode` como **enum** (no solo el struct `Query`) | Coincide con el hito del brief Y permite construir sub-árboles en tests; el planner del cap. 19 opera cláusula a cláusula | Solo `Query { match, where, return }`: el hito del brief no se refleja y el planner pierde su unidad de trabajo | Tests que solo pueden construir consultas completas | Doc-comment líneas 547-569; test `ast_node_variants_build_and_match`; MIGRATION-PATTERN §21-8 |
| 11 | LiraQL = mini-Cypher **solo consulta**, RETURN siempre presente | Recorte pedagógico deliberado: MATCH obligatorio + WHERE opcional + RETURN obligatorio. Sin CREATE/MERGE/DELETE (DML cap. 31), WITH, OPTIONAL MATCH ni recursión | Clonar Cypher entero: el diseño del capítulo se ahoga en casos borde antes de que exista una sola línea de executor | Capítulo de especificación de 60 páginas sin motor que la ejecute | Doc-comment líneas 13-24; MIGRATION-PATTERN §21-2 |
| 12 | `EmptyNodePattern`: rechazar `()` puro | Cypher lo permite (matchea cualquier nodo), pero en un lenguaje didáctico un patrón que no liga nada ni filtra nada es un hoyo pedagógico; el error enseña el propósito del patrón | Aceptarlo: un error silencioso de diseño que el alumno repite sin saber por qué existe | Consultas que recorren el grafo entero sin querer | `QueryErrorKind::EmptyNodePattern`; test `validate_node_pattern_vacio_devuelve_empty_node_pattern`; divergencia con el binder del cap. 19 documentada en §23-7 |
| 13 | Alcance de variables **incluye las de arista** (`-[r:KNOWS]->` liga `r`) y prohíbe duplicados nodo↔arista | Una consulta es un solo ámbito: si `p` fuese nodo y arista a la vez, el executor (cap. 20) tendría dos tipos para un nombre | Ámbitos separados silenciosos: `p` nodo y `p` arista coexisten | Ligadura ambigua en WHERE/RETURN: el tipo de `p` depende de la cláusula que lo mire | Tests `validate_variable_duplicada_entre_nodo_y_arista`, `validate_acepta_variable_de_arista_en_where` |
| 14 | `hex_bytes()` propio (sin crates) para `Value::Bytes` en `Display` | Política de dependencias del Vol.II: 14 líneas propias vs una crate para 6 dígitos hexadecimales | Dependencia `hex`: viola «primero a mano» para un detalle cosmético | — (solo coste de mantenimiento de una función trivial) | Código líneas 841-852; test `hex_bytes_formatea_correctamente` |

## 6. Primera solución vs solución evolucionada

- **Ingenua (la que escribiría un novato)**: API builder encadenada en Rust:
  `Query::match_().node("p", "Person").arrow_out("KNOWS").node("f", "Person")
  .where_(eq(prop("p", "name"), str("Ana"))).return_(prop("f", "name"))`.
  Compila, el tipo protege el orden, cero parsing que escribir.
- **Qué la rompe**: (a) no es TEXTO — no se puede teclear en la CLI del cap. 31,
  ni pegar en un issue, ni guardar en un fichero de consultas, ni loguear; (b) fija
  el CÓMO: la cadena dicta el orden de recorrido y el optimizador del cap. 21 no
  puede reordenar nada; (c) exige compilar Rust para preguntar; (d) cada constructo
  nuevo (LIMIT, ORDER BY) es un método más y un breaking change de API, no una
  palabra del lenguaje.
- **Evolución visible en el capítulo**: el texto `MATCH (p:Person)-[:KNOWS]->(f:Person)
  WHERE p.name = "Ana" RETURN f.name` se convierte en un CONTRATO en cuatro piezas:
  EBNF (qué secuencias son válidas) → `TokenKind` (qué átomos existen) → AST (qué
  estructura significativa tienen, con `Span`) → `validate()` + `Display` (qué
  errores y qué forma canónica). Nada de esto ejecuta nada: todo es diseño.

## 7. Prueba de fuego

- **Tests reales** (módulo `tests_query`, 41, en `cap18_lexer_parser.rs:2007`,
  construyendo AST A MANO sin parser — la prueba de que el diseño es testeable
  antes de existir la implementación): `span_new_normaliza_orden`,
  `span_merge_cubre_a_ambos`, `token_kind_cubre_todos_los_grupos`,
  `expression_and_or_not_recolecta_recursivo`, `path_pattern_edge_variables_incluye_rel_var`,
  `validate_consulta_minima_es_valida`, `validate_consulta_completa_es_valida`,
  `validate_match_vacio_devuelve_empty_match`,
  `validate_node_pattern_vacio_devuelve_empty_node_pattern`,
  `validate_variable_duplicada_en_nodos`,
  `validate_variable_duplicada_entre_nodo_y_arista`,
  `validate_variable_desconocida_en_where`,
  `validate_variable_desconocida_en_return`, `validate_return_vacio`,
  `validate_alias_vacio_en_return`, `validate_acepta_variable_de_arista_en_where`,
  `query_error_display_incluye_span`, `query_error_display_todas_variantes`,
  `query_error_implementa_std_error`, `display_expression_compare_and_or_not`,
  `display_relationship_pattern_direcciones`, `display_query_completa_round_trip_canonico`,
  `display_query_con_where_y_alias`, `ast_node_variants_build_and_match`,
  `display_value_bytes_canonico`, `hex_bytes_formatea_correctamente`.
- **Síntoma si el lector se salta el capítulo**: al llegar al cap. 18 no tendrá un
  contrato que implementar: rediseñará tokens/AST mientras escribe el parser (lo que
  la historia real del workspace evitó: los dos gaps —`Dash`, `Variable`— se
  detectaron como fallos puntuales del CONTRATO, corregibles en diez líneas).

## 8. Trampas y errores comunes

1. **Confundir diseño con implementación**: esperar código de lexer/parser aquí.
   Este capítulo compila 970 líneas SIN dependencias y SIN ejecutar una consulta.
2. **Confundir error sintáctico con semántico**: el parser (cap. 18) aborta en el
   primero (`Result`); `validate()` (cap. 17) reporta todos (`Vec`). Capas
   distintas, UX distinta.
3. **Derivar `Eq` en enums con `f64`**: no compila (`Float(f64)` → NaN ≠ NaN). Bug
   real del desarrollo, documentado en §21.
4. **Creer que `Display` debe reproducir el texto original**: es forma CANÓNICA —
   añade paréntesis a las comparaciones (`(p.name = "Ana")`), normaliza comas y
   espacios; el round-trip es idempotencia de la forma canónica, no igualdad de
   texto.
- **Precisión de lenguaje (glosario)**: *token* vs *lexema* (categoría vs texto);
  *gramática* vs *sintaxis*; *AST* vs *plan* (forma vs estrategia, cap. 19);
  *error sintáctico* vs *semántico*; *span* (bytes `[start, end)`) vs *línea:columna*;
  *variable ligada/declarada* vs *usada*; *forma canónica*; *declarativo* vs
  *imperativo*; *inline predicate* (`{name: "Ana"}`) vs predicado de WHERE.

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval del cap. 7, sin pistas en el enunciado)**:
  sin mirar nada, listar de memoria las seis variantes de `Value` del cap. 7; luego
  decir cuáles tienen literal en LiraQL y cuál NO lo tiene ni lo tendrá como token,
  y cómo aun así aparece en `Display`. Verificación: `display_value_bytes_canonico`
  + `hex_bytes_formatea_correctamente`. Pistas (≤3, graduadas): (1) ¿qué tipos
  sabes escribir sin comillas ni notación especial?, (2) mira `TokenKind`: ¿hay
  token para bytes?, (3) `display_value` tiene un brazo para esa variante. Criterio:
  las 6 variantes + identificar `Bytes` + justificar (no hay literal de bytes; su
  Display es `0x…` hex).
- **analizar (intermedio)**: predecir la lista EXACTA (kinds, spans, orden) de
  `validate()` sobre un AST construido a mano para
  `MATCH (p:Person)-[r:KNOWS]->(p:Person), () RETURN x.name` (3 errores:
  DuplicateVariable `p`, EmptyNodePattern, UnknownVariable `x`). Verificación:
  `validate_variable_duplicada_entre_nodo_y_arista` como patrón +
  `validate_node_pattern_vacio_devuelve_empty_node_pattern`. Pistas: (1) ¿qué
  recorre primero validate: nodos o aristas?, (2) ¿declara `r` algo que compita con
  `p`?, (3) ¿qué span lleva UnknownVariable: el de la declaración o el del uso?
  Criterio: 3 kinds correctos + orden de informe + span del uso en RETURN.
- **crear (experto — diseño puro, sin parser)**: diseñar `LIMIT n`: (a) regla EBNF
  que extienda `return_clause` (¿o va tras RETURN…?), (b) campo nuevo en el AST con
  su `Span`, (c) regla de `validate()` (¿LIMIT negativo o cero? ¿dónde vive el
  error?), (d) extensión de `Display` canónico, (e) dos tests con AST a mano al
  patrón de `tests_query`. Prohibido escribir lexer/parser (eso es cap. 18: una
  función nueva por regla nueva). Verificación: compilar + `cargo test -p
  vol2-liradb` con los tests añadidos en un módulo propio. Pistas: (1) ¿LIMIT
  modifica RETURN o es cláusula hermana?, (2) ¿qué tipo tiene `n` (mira los
  literales disponibles)?, (3) ¿debería `Query` seguir siendo igual de clonable?
  Criterio: EBNF sin ambigüedad + error tipado con span + Display idempotente.

## 10. Preguntas abiertas (gancho al capítulo 18)

1. Ya sabemos QUÉ es una consulta válida; ¿cómo se transforma un `String` crudo en
   `Vec<Token>`? ¿Quién decide que `->` es UN token y no `-` seguido de `>`?
   (maximal-munch.)
2. La EBNF tiene una regla por concepto: ¿basta una función por regla para tener
   parser? (sí: descendente recursivo predictivo.)
3. `validate()` asume AST sintácticamente bien formado: ¿qué errores detecta el
   parser que aquí no pueden existir, y por qué elige `Result` en vez de `Vec`?
- **Términos nuevos de glosario**: LiraQL, token, lexema, keyword, EBNF, regla
  gramatical, precedencia, AST, span, forma canónica, round-trip, validación
  semántica, alcance (scope) de variables, inline predicate, patrón de camino,
  dirección de relación (outgoing/incoming/undirected), alias, declarativo.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio esencial obliga a reconstruir las variantes
  de `Value` DESDE LA MEMORIA (cap. 7) antes de razonar sobre sus literales — el
  enunciado no las nombra; el intermedio exige recordar la estructura de `validate()`
  (orden de comprobaciones) sin ver el código.
- **Spacing**: esencial → `Value` del cap. 7 (spacing directo, pedido por el
  diseño); intermedio → patrón de errores tipados de los caps. 12-16 (`Display` +
  `std::error::Error`); en prosa se re-ejercita el «contrato primero» del trait
  `Pager` (cap. 12) y el round-trip encode/decode del cap. 11 como espejo del
  display/parse del 18.
- **Interleaving**: el experto mezcla gramática (EBNF), tipos (AST + Span),
  semántica (validate) y serialización (Display) — las cuatro piezas del capítulo
  en un solo diseño; el intermedio cruza alcance de variables con aritmética de
  spans.
- **Dificultad asimétrica**: una idea nueva por sección (declarar / dibujar /
  tokenizar / estructurar / situar / validar / canonicalizar); los ejercicios
  exigen recuperación y predicción, no reconocimiento.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` (41 tests citados
  por nombre; los nuevos del ejercicio experto compilan en el mismo workspace).
- **Citas**: Codd (CACM 1970); Chamberlin & Boyce (SIGMOD 1974, «SEQUEL»); Francis
  et al. (SIGMOD 2018, Cypher/openCypher); openCypher (opencypher.org, 2016); GQL
  ISO/IEC 39075:2024 (publicado 17-abril-2024, primer lenguaje ISO nuevo desde SQL
  1987 — gqlstandards.org); SPARQL 1.1 (W3C Rec. 2013); Wirth «Compiler
  Construction»; Nystrom «Crafting Interpreters» (cap. de ASTs); rustc Dev Guide /
  codespan-reporting / miette (spans y diagnósticos).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (14 en la tabla §5).
- [x] Escenario de fallo visible: «unexpected token» sin posición vs error con `Span`; gramática ambigua `a OR b AND c` sin niveles de precedencia.
- [x] Código ejecutable en workspace (41 tests de `tests_query`) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (diseñar ≠ parsear; API ≠ lenguaje; sintáctico ≠ semántico; Display ≠ reproducción).
- [x] Ejercicios con solución verificable (`cargo test`).
- [x] ≥1 ejercicio de retrieval (variantes de `Value` desde memoria) y ≥1 de spacing (cap. 7 directo; caps. 11-12 en prosa).
- [x] Responde las preguntas críticas: por qué declarativo, por qué ASCII-art y no JOINs, por qué diseño antes que parser, por qué Span en todo, por qué Value reutilizado, por qué Vec y no Result, por qué Display canónico, por qué EBNF primero.
- [x] Sección «cómo lo hace una BBDD real» incluida: openCypher, GQL ISO 39075, SPARQL, con retos esencial/intermedio/experto.
- [x] Anécdota verificada: SEQUEL (Chamberlin & Boyce, IBM, SIGMOD 1974; renombrado a SQL — la marca de Hawker Siddeley, «según cuenta») y Cypher (Andrés Taylor, Neo4j, 2011; openCypher 2016; GQL ISO abril 2024). Fuentes: 1995 SQL Reunion (IBM), SIGMOD 2018, Wikipedia, gqlstandards.org.
