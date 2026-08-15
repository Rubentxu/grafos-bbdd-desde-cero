# CONTRATO DE CAPÍTULO — Vol.II Cap. 18: Construir el lexer y el parser

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap18_lexer_parser.rs` (2.963 líneas;
> 73 tests en `tests_lexer_parser`; el módulo incluye también `tests_query` del
> cap. 17). Decisiones y bugs reales: `liradb-workspace/book-context/
> MIGRATION-PATTERN.md` §22. Línea del TOC: 31 (`manuscrito/vol2/tabla-de-contenidos.md`).
> Abre la Parte IV por dentro: el cap. 17 fijó el lenguaje; este lo ejecuta.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el contrato de tipos del cap. 17 (`TokenKind`,
  `Token { kind, span }`, `Span { start, end }` en bytes UTF-8, `Expression`,
  `Query`, `AstNode`, `validate() -> Vec<QueryError>`, `Display` canónico) y su
  gramática EBNF documentada en `cap17_liraql_ast.rs`; `Value` del cap. 7
  (Int/Float/String/Bool/Null) envuelto por `Expression::Literal`; el patrón de
  errores tipados con `Display` + `std::error::Error` + `From` (caps. 12-16); la
  disciplina del cap. 9 de declarar formatos explícitos (little-endian en disco).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «el parser
  lee texto» — falso: el parser lee TOKENS; quien lee bytes es el lexer, y son dos
  fases con dos culpas distintas; (2) «un carácter mal lexado rompe donde
  aparece» — falso: los errores de lexer se pagan en el parser, a veces lejos de
  la causa (caso real `<>`); (3) «la precedencia de operadores exige una tabla» —
  falso: una cadena de funciones la codifica y la hace legible; (4) «más errores
  reportados = mejor parser» — falso: sin recuperación, la multi-detección
  produce cascadas de errores derivados que confunden; (5) «un string se escanea
  como el resto del fuente» — falso: dentro de `"..."` el lexer debe ser CIEGO a
  la sintaxis (un `-` dentro de un string no es `Dash`).
- **NO debe saber todavía**: cómo el AST baja a plan lógico (`lower`, `Bindings`,
  `ScalarExpr` — cap. 19), el modelo Volcano (cap. 20), reglas de optimización
  (cap. 21), `logos`/`pest`/LALRPOP más allá de la sección comparativa
  (apéndices; el CORPUS pregunta los trade-offs y la sección «cómo lo hace una
  BBDD real» los responde a nivel de criterio, no de implementación), DML y CLI
  (cap. 31). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: **lexer/tokenizador** como bucle `while` + cursor de bytes
  (`Lexer { src: &[u8], pos: u32 }`); **maximal-munch** (el token más largo
  gana: `->` antes que `-`, `<>` antes que `<`); `skip_whitespace`; escaneo de
  identificadores/palabras clave (case-sensitive), números (con overflow
  `checked_mul`) y strings con escapes (`\n \t \r \\ \" \0`); **spans UTF-8 en
  bytes** (el certificado de origen de cada token); token `Eof` explícito;
  **parser descendente recursivo predictivo** (una función por regla EBNF,
  preanálisis con `peek`); **precedence climbing por funciones**
  (`parse_or → parse_and → parse_not → parse_comparison → parse_primary`);
  `LexError` (5 variantes) y `ParseError` (envoltura `Lex` + 7 sintácticas) con
  `From` y `source()`; recuperación minimalista de un solo error.
- `practice`: `Span`/`TokenKind` del cap. 17 (ahora los rellena el lexer, no los
  tests); `Value` del cap. 7 en `parse_primary`; `Display` canónico y round-trip;
  patrón «errores tipados + `From`» de los caps. 12-16.
- `consolidate`: «el código ES la gramática» (¿de la regla a la función en un
  paso); «derivar, no llevar en la cabeza» (el span sale de `start..pos`, no se
  calcula después); rigor del cap. 9: declarar unidades explícitas (aquí, que
  los offsets son bytes, no caracteres).
- `out_of_scope` (solo nombrar): plan lógico y binder (19), Volcano (20),
  optimizador (21), `logos`/`pest`/LALRPOP en detalle (apéndice comparativo),
  notación científica y operadores unarios (recortes declarados), comentarios
  `//` en LiraQL.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué lexer y parser van SEPARADOS y qué le
  pasa al parser si el escaneo y el parsing se mezclan (scannerless); (2) enuncia
  maximal-munch y predice qué tokens produce `<>`, `->`, `--`, `12.`; (3) dibuja
  la cadena de precedencia y dice qué función parsea `a OR b AND c` y con qué
  estructura de AST sale; (4) distingue error léxico de sintáctico y dice quién
  envuelve a quién (`ParseErrorKind::Lex` con `source()`); (5) explica por qué
  el lexer debe ser inmune al contenido de los strings.
- **Skills**: (1) trazar `parse("MATCH (p:Person) RETURN p")` a mano: tokens
  con spans → AST con `Expression::Variable`; (2) leer un `ParseError` y
  localizar el byte culpable en el fuente (incluido el caso adverso: error del
  parser causado por el lexer); (3) extender el lexer/parser con un token o una
  producción nueva (ejercicio experto: `1e10`).
- **Wisdom**: (1) decide cuándo un parser a mano deja de ser suficiente y qué
  delegar (`logos` para el escaneo repetitivo, `pest`/LALRPOP para gramáticas
  grandes declarativas — y por qué aquí NO, regla «primero a mano, luego con
  crate»); (2) decide cuánta recuperación de errores vale la pena (un error bien
  localizado vs una cascada).

## 4. Modelo mental

- **El oficial de registro y el gramático**: el fuente llega como un rollo
  continuo de bytes; el **lexer** es el oficial que lo corta en palabras
  (tokens) y estampa en cada una su certificado de origen —nacimiento y muerte
  en BYTES (`Span { start, end }`)—; el **parser** es el gramático que recibe
  la bandeja de palabras ya numeradas y comprueba que la frase cumple la EBNF
  del cap. 17. El oficial no opina sobre sintaxis; el gramático nunca toca un
  byte crudo. Si el oficial pierde una palabra (`Dash`) o la corta mal (`<>` en
  dos trozos), el gramático señala un culpable inocente.
- **Diagramas ASCII**: (a) la cadena `texto → Lexer → Vec<Token> → Parser →
  Query`; (b) el fuente `"MATCH (p)"` con la regla marcando cada byte y los
  spans `[0..5) [6..7) [7..8) [8..9)`; (c) la pila de llamadas
  `parse_or → parse_and → parse_not → parse_comparison → parse_primary` como
  «pirámide de precedencia» (OR el más flojo abajo del árbol, primary el más
  fuerte).
- **Momento ¡ajá!**: «la precedencia no es una tabla: es el ORDEN de las
  funciones; el código del parser ES la gramática del cap. 17». Y su reverso
  oscuro: «un byte tragado por el lexer no rompe el lexer: rompe el parser,
  en otro sitio».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap18_lexer_parser.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Lexer y parser SEPARADOS (`lex()` → `Vec<Token>`; `Parser { tokens, current }`) | Separar el nivel regular (tokens: blanks, escapes, palabras clave) del libre-de-contexto (la EBNF). Los tokens liberan al parser de ocuparse de espacios y del contenido de los strings; y el brief (§11) exige enseñar cursores y spans | Scannerless (PEG estilo `pest`): una sola gramática, pero cada regla arrastra `_ws` y el escaneo deja de ser visible — justo lo que hay que aprender | El parser tendría que mirar bytes crudos dentro de reglas gramaticales: se duplica la lógica de `->` vs `-` en cada regla | Dragon Book cap. 3 (separación de fases); MIGRATION-PATTERN §22.1 |
| 2 | Cursor sobre `&[u8]` con `pos: u32` (no `chars()`) | `Span` es un rango de BYTES (cap. 17, como rustc): el cursor debe avanzar en la misma unidad que el span mide, o los offsets mienten | `str::chars()`: decodifica y la posición devuelta es en caracteres → `"cañón"` reportaría spans falsos (test: el span del string es 0..9 en BYTES) | Mensajes de error que señalan el byte equivocado en cualquier fuente con acentos/ñ | Test `lex_span_es_aware_a_utf8_en_bytes` (ñ = 2 bytes); cap. 9: unidades explícitas, nunca implícitas |
| 3 | **Maximal-munch** en `scan_token` (probar combinaciones de 2 bytes antes que la de 1) | El token más largo posible gana: `->`, `--` antes que `-`; `<-`, `<=`, `<>` antes que `<`. Es la regla que todo lexer real aplica | Matching minimal: `-` siempre `Dash` y el parser tendría que re-pegar pares; ambigüedad en `a-b` | `<>` se parte en `Lt Gt` y el parser falla en un token inocente (bug real §8 del capítulo) | MIGRATION-PATTERN §22 lección 1; ramas `b'-'`/`b'<'` de `scan_token`; test `lex_flechas_y_guiones` |
| 4 | `TokenKind::Dash` añadido (deuda del cap. 17) | El vocabulario cap. 17 definió `ArrowRight/ArrowLeft/DashDash` pero olvidó el guión simple: los extremos `-[ ... ]-` y `]-` de relaciones entrantes/sin dirección no tenían token | Sin `Dash`: imposible parsear `MATCH (p)<-[:KNOWS]-(f)`; toda relación no saliente falla | `MalformedRelationship`/`UnexpectedToken` en TODAS las consultas con relaciones entrantes | MIGRATION-PATTERN §22.3 (gap de diseño cap. 17); tests `parse_path_con_relacion_entrante/sin_direccion` |
| 5 | `Expression::Variable { name, span }` (deuda del cap. 17) | La EBNF `primary ::= literal \| property_access \| '(' expression ')'` no contemplaba la variable sola; el hito `RETURN p` exige referenciar el nodo COMPLETO, que no es una propiedad | `PropertyAccess` sin propiedad (prop = `Option`): envenena el executor (cap. 20) con «¿qué significa `.None`?» y rompe el `Display` canónico | El hito del brief `parse("MATCH (p:Person) RETURN p")` no parsea | MIGRATION-PATTERN §22.4 y lección 6; test `parse_hito_del_brief` (matchea `Expression::Variable`) |
| 6 | Descendente recursivo **predictivo** (una función por regla EBNF, decisión con `peek`) | El código ES la gramática: `parse_match_clause` ↔ `match_clause ::= ...`. Legible, depurable con el depurador (la pila de llamadas es la derivación), extensión local | Tabla LL(1)/LALR generada: compacta y detecta ambigüedades, pero ilegible («depurar una tabla»); Pratt: excelente con decenas de precedencias, esconde la gramática en una tabla de binding powers | Con LALR/Pratt aquí: el lector no vería la correspondencia regla↔código, objetivo pedagógico del capítulo | Wirth, «Compiler Construction» (recursive descent como técnica de referencia); rustc usa parser descendente manual (rustc-dev-guide); MIGRATION-PATTERN §22.5 |
| 7 | Precedencia por CADENA de funciones (`parse_or → parse_and → parse_not → parse_comparison → parse_primary`) | El orden de las funciones ES la precedencia: cada nivel consume SU operador y delega hacia abajo el más fuerte; cambiar la precedencia = mover una línea | Tabla de precedencia + Pratt: desacopla especificación de código, útil con ≥8-10 niveles; con 4 niveles es burocracia | Tabla opaca: la jerarquía deja de verse en el código y los bugs de precedencia se esconden | Tests `parse_precedencia_or_es_menor_que_and`, `parse_precedencia_and_es_menor_que_not`, `parse_precedencia_parentesis_rompe_orden` |
| 8 | `LexError` y `ParseError` SEPARADOS, unidos por `From<LexError> for ParseError` + `source()` | Dos fases, dos culpas: el que escribe el mensaje sabe si el problema es del escaneo (`UnexpectedChar`) o de la estructura (`MissingMatch`); `?` propaga sin código extra y la cadena causal se conserva | Un solo error con `String`: imposible `match` fiable; o solo `ParseError` aplanado: se pierde quién falló | El CLI (cap. 31) no podría distinguir «tu string no cierra» de «te falta RETURN» | Tests `parse_error_propagado_del_lexer`, `parse_error_implements_std_error_con_source` |
| 9 | Recovery minimalista: primer error y abort | Un único mensaje claro CON SPAN EXACTO enseña más que una cascada; la recuperación exige sincronizar (saltar a `)` o `,`) y filtra errores derivados | Multi-error estilo `pest`: mejor UX de producción, pero duplica la complejidad del parser para un lenguaje didáctico | Cascadas: «falta RETURN», «token inesperado», «basura final» — tres mensajes para UN olvido | MIGRATION-PATTERN §22.7; doc-comment de `ParseError`; contrastar con `validate()` del cap. 17 que SÍ acumula (semántica vs sintaxis) |
| 10 | Lexer CIEGO al contenido de los strings (`scan_string` hasta comilla con escapes) | `"Ana-García \"cita\""` contiene `-`, `{`, `"` escapada y UTF-8 multi-byte: nada de eso es sintaxis. `scan_string` sólo reconoce `\` y `"` de cierre | Escanear el interior con las reglas generales: cualquier descripción con un guión rompería el WHERE | `WHERE p.name = "a-b"` produce tokens basura dentro del literal | Tests `lex_string_con_escapes`, `lex_string_sin_cerrar_es_error`, `lex_escape_invalido_es_error` |
| 11 | Token `Eof` explícito al final de `lex()` | El parser SIEMPRE tiene token actual: `peek()` devuelve `&Token`, nunca `Option`. «Falta RETURN» y «consulta incompleta» son distinguibles (`MissingReturn` vs `UnexpectedEof`) | Stream sin Eof: `peek() -> Option<Token>` y `if let` por todas partes | Doble comprobación (fin + tipo) en cada regla; errores de EOF confusos | Tests `lex_eof_se_anade_al_final`, `lex_cadena_vacia_solo_eof` |
| 12 | Palabras clave case-sensitive (`MATCH` sí, `match` → `Ident`) | Convención Cypher (mayúsculas); la clasificación es un `match` sobre el texto exacto en `scan_identifier` — cero estado extra | Case-insensitive: exigiría normalizar el texto y cargaría al lexer; además `match` dejaría de ser un identificador válido | Sorpresas silenciosas: `where` como variable usada como cláusula | Test `lex_palabras_clave_son_case_sensitive` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: parsear con métodos de `str`: `src.split_whitespace()`,
  `starts_with('(')`, `contains("->")`. Funciona para `MATCH (p) RETURN p.name`
  con espacios perfectos.
- **Qué la rompe exactamente**: (a) `(p:Person)-[:KNOWS]->(f)` no tiene espacios
  que separen `-[:KNOWS]->`; (b) `<>` vs `<` vs `<=`: sin maximal-munch, `-` y
  `>` sueltos llegan al parser; (c) strings con espacios o escapes:
  `"Ana García"` se parte en dos; (d) cero posiciones: «error en la consulta»
  sin byte culpable; (e) UTF-8: `cañón` descuadra cualquier aritmética en
  caracteres.
- **Evolución visible en el capítulo**: fase 1, `Lexer` con cursor de bytes que
  produce `Vec<Token>` con spans exactos (y `Eof`); fase 2, `Parser` con
  `current: usize` sobre esa lista, una función por regla EBNF y la cadena de
  precedencia; errores tipados en las dos fases unidos por `From`. La versión
  con `logos` (escaneo delegado) queda para el apéndice comparativo: la regla
  «primero a mano, luego con crate» del Vol. II.

## 7. Prueba de fuego

- **Hito del brief**: `parse("MATCH (p:Person) RETURN p")` → `Query` válida con
  `Expression::Variable` en el RETURN (test `parse_hito_del_brief`).
- **Batería real** (se citan, no se duplican): lexer — `lex_palabras_clave`,
  `lex_comparadores`, `lex_flechas_y_guiones`, `lex_span_de_token_cubre_exactamente_su_texto`,
  `lex_whitespace_no_cuenta_en_spans`, `lex_span_es_aware_a_utf8_en_bytes`,
  `lex_string_con_escapes`, `lex_entero_desborda_i64_es_error`,
  `lex_caracter_inesperado_es_error`; parser — `parse_path_con_relacion_saliente/entrante/sin_direccion`,
  `parse_node_pattern_con_propiedades`, `parse_where_todos_los_comparadores`,
  `parse_precedencia_*` (3), `parse_error_no_empieza_por_match`,
  `parse_error_falta_return`, `parse_error_tokens_sobra_al_final`,
  `parse_error_propagado_del_lexer`, `round_trip_consulta_minima/completa`,
  `consulta_ejemplo_cap17_brief`, `parser_from_tokens_funciona`.
- **Escenario de fallo visible**: `parse("SELECT * FROM nada")` → `MissingMatch`
  con span `0..6` (señala `SELECT`, no «consulta inválida»); y la cadena rota:
  un lexer que pierde un byte de `<>` produce el error en el parser, apuntando
  al token SIGUIENTE — el caso de estudio §18.8 del capítulo.
- **Síntoma si el lector se salta el capítulo**: el cap. 19 solo recibiría
  `Query` construidas a mano en tests (como el cap. 17), la CLI del cap. 31 no
  podría leer una consulta del usuario, y todo error de sintaxis sería un
  pánico o un «consulta inválida» sin posición.

## 8. Trampas y errores comunes

1. **Olvidar una combinación maximal-munch** (bug real de `<>`): el lexer parte
   el token y el parser culpa a un inocente. Detectar: el error apunta a un
   token que el usuario «no escribió».
2. **Consumir en un `peek`**: `peek`/`peek_at` jamás avanza `pos`; si avanzas al
   mirar, todos los spans posteriores mienten por un byte (la «cadena rota»).
3. **Medir spans en caracteres**: los offsets son BYTES UTF-8; `cañón` ocupa 7
   caracteres y 9 bytes. Confundir unidades es el endianness implícito de este
   capítulo (misma lección del cap. 9).
4. **Confundir igualdad con discriminante**: `check` usa
   `std::mem::discriminant` porque `Ident("x")` lleva datos y
   `match_kind(&TokenKind::Ident(String::new()))` casa CUALQUIER identificador.
5. **Esperar `-3` como literal**: LiraQL no tiene unarios (cap. 17); el lexer
   emite `Dash Integer(3)` y el parser lo rechaza — recorte declarado.
- **Precisión de lenguaje (glosario)**: *token* vs *lexema* (el lexema es el
  texto; el token es lexema+clase+span) vs *byte*; *lexing/scanning* vs
  *parsing*; *preanálisis/lookahead* (`peek`); *maximal-munch*; *descendente
  recursivo* vs *predictivo* (el nuestro es ambos: desciende por la gramática y
  decide con un token de lookahead) vs *Pratt* vs *tabla LL/LALR*;
  *recovering* vs *abortar*; *span vacío* (`Span::at`) vs *rango*.

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval puro)**: SIN mirar el cap. 17,
  reescribir de memoria las producciones EBNF de `return_item`, `comparison` y
  `primary`, y decir qué método del parser corresponde a cada una. Verificación:
  contrastar con el comentario EBNF de `cap17_liraql_ast.rs` (líneas 41-59).
  Pistas: (1) ¿qué cláusula es opcional en la consulta completa?, (2) ¿qué
  separa ítems de RETURN?, (3) ¿qué puede aparecer a la izquierda de un
  comparador? Criterio: las tres producciones exactas + su función gemela.
- **analizar (intermedio — interleaving lexer/parser/spans)**: predecir EN
  PAPEL el error exacto (variante + span) de: `MATCH (p:Person RETURN p.name`
  (falta `)`), `MATCH (p) RETURN` (falta ítem), `MATCH (p) WHERE p.name = "Ana
  RETURN p` (string sin cerrar). Luego verificar con un test de humo. Pistas:
  (1) ¿qué `expect` falla primero y con qué span?, (2) ¿qué token encuentra al
  buscar el ítem?, (3) ¿dónde termina el span de un string sin cerrar?
  Criterio: variante Y byte inicial del span correctos en los tres casos.
- **crear (experto — spacing con cap. 9/cap. 7)**: implementar la notación
  científica `1e10` y `1.5e-3` en `scan_number` (el doc-comment la declara
  recorte pedagógico) decidiendo: ¿es parte del MISMO token (maximal-munch) o
  tokens separados? ¿Qué variante de `LexError` cubre `1e` sin exponente?
  Añadir tests tipo `lex_flotante`. Criterio: el span del literal cubre todo el
  lexema, y `1e` produce `MalformedNumber`, no un panic.

## 10. Preguntas abiertas (gancho al capítulo 19)

1. Ya hay `Query` desde texto… ¿qué hace el motor con ella? ¿Quién decide que
   `Filter` va encima del `NodeScan` y no debajo? (cap. 19: el plan lógico.)
2. `WHERE p.age > 18 AND p.vip = TRUE` es un árbol `Expression`: ¿cómo se
   convierte en predicados que el executor evalúe fila a fila? (`ScalarExpr`,
   binder.)
3. El `Display` canónico ya sabe re-serializar el AST: ¿podría `liradb explain`
   (cap. 21) imprimir el PLAN igual de bonito? (Sí: mismo patrón.)
- **Términos nuevos de glosario**: lexer, token, lexema, maximal-munch, span,
  lookahead/preanálisis, descendente recursivo, predictivo, precedence
  climbing, Pratt parser, tabla LL/LALR, scannerless, PEG, recuperación de
  errores, token de fin (Eof).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio esencial obliga a RECONSTRUIR la EBNF
  del cap. 17 desde la memoria (nada del enunciado la regala); el intermedio
  exige recordar variantes y semántica de spans sin pistas textuales.
- **Spacing**: esencial → gramática EBNF del cap. 17; intermedio → `Span`
  semiabierto y `Display` con sufijo `(en a..b)` del cap. 17; experto → rigor
  de unidades del cap. 9 (bytes, formatos explícitos) + `Value::Float` del
  cap. 7; la sección «dos fases, dos culpas» re-ejercita el patrón de errores
  tipados de los caps. 12-16.
- **Interleaving**: el intermedio mezcla errores del LEXER (UnterminatedString)
  con errores del PARSER (UnexpectedToken, MissingReturn) y aritmética de
  spans; el experto cruza escaneo numérico con diseño de errores.
- **Dificultad asimétrica**: cada sección introduce UNA idea (cursor /
  maximal-munch / una-función-por-regla / precedencia-cadena / dos fases de
  error); los ejercicios exigen recuperación, predicción y diseño.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb
  tests_lexer_parser` (73 tests, citados por nombre).
- **Citas**: Aho/Sethi/Ullman, «Compilers: Principles, Techniques, and Tools»
  (Addison-Wesley, 1986 — la separación de fases y la portada del dragón);
  Wirth, «Compiler Construction» (descendente recursivo); rustc-dev-guide
  (parser descendente manual de rustc); Nystrom, «Crafting Interpreters»
  (scanning/compiling expressions); docs de `logos`, `pest`, `LALRPOP`;
  ISO/IEC 39075:2024 (GQL); repos de Neo4j (JavaCC) y Kùzu (`src/parser/`,
  descendente manual).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: `SELECT` → `MissingMatch` con span 0..6; y la cadena rota del lexer que pierde un byte de `<>` (§7, §18.8 del capítulo).
- [x] Código ejecutable en workspace (73 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (parser lee tokens no texto; error de lexer ≠ error de parser; precedencia sin tabla; multi-error no siempre mejor; string = territorio ciego).
- [x] Ejercicios con solución verificable (`cargo test` + contraste con cap17).
- [x] ≥1 ejercicio de retrieval (EBNF del cap. 17 desde memoria) y ≥1 de spacing (caps. 7, 9, 12-16).
- [x] Responde la pregunta crítica del CORPUS (`vol-II-cap-18`): «`logos` vs `pest` vs parser manual: trade-offs» — sección «cómo lo hace una BBDD real» + porqués 1 y 6.
- [x] Anécdota verificada: libro del dragón rojo (1986; portada caballero + dragón, lanza «LALR parser generator»; metáfora de vencer la complejidad de los compiladores; verde 1977, púrpura 2006) — Wikipedia + Jargon File.
