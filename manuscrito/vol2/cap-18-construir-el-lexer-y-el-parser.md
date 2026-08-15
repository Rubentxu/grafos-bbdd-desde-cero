# Capítulo 18 — Construir el lexer y el parser

> *«El lexer ve bytes. El parser ve tokens. El que mezcla las dos cosas paga sus errores en el sitio equivocado.»*

## 18.0 La anécdota de la esquina

En 1986, Alfred Aho, Ravi Sethi y Jeffrey Ullman publicaron *Compilers: Principles, Techniques, and Tools*. La portada muestra un caballero luchando contra un dragón rojo, y ese detalle le dio el nombre con el que todo el mundo conoce al libro: **el libro del dragón**. La imagen no era decorativa: el dragón representa la complejidad del diseño de compiladores, y el caballero la combate blandiendo una lanza etiquetada en la propia portada como *«LALR parser generator»*. (Existen tres dragones: el verde de 1977 —*Principles of Compiler Design*, de Aho y Ullman—, este rojo de 1986 y el púrpura de la segunda edición de 2006, ya con Monica Lam.)

Lo que el libro enseñó a generaciones fue algo más humilde que LALR: **separar el escaneo del parsing**. Antes de pensar en árboles de derivación, un compilador tiene que resolver un problema estúpidamente difícil de hacer bien: cortar un flujo continuo de caracteres en piezas con sentido —tokens— y recordar dónde empieza y dónde acaba cada una. Los tokens liberan al parser de ocuparse de espacios, saltos de línea y del contenido de los strings. Esa separación es la columna vertebral de este capítulo.

Hay una ironía deliciosa: la lanza del caballero es un *generador* de parsers LALR, y nosotros no vamos a usar ninguna. Niklaus Wirth —el padre de Pascal, que compiló sus lenguajes con descendente recursivo en los años 70 y defendió esa técnica durante toda su vida en *Compiler Construction*— demostró que para un lenguaje pequeño y bien diseñado, la mejor herramienta no es una tabla generada: es **una función por regla de la gramática**. Hasta `rustc`, uno de los compiladores más serios del mundo, usa un parser descendente escrito a mano. Hoy construiremos la nuestra.

## 18.1 Objetivo

En el cap. 17 diseñamos LiraQL sobre el papel: tokens, gramática EBNF, AST, errores con posición. Pero las `Query` se construían a mano en los tests. Este capítulo baja un escalón y construye **el código que convierte texto en ese AST**:

1. `Lexer` — el escáner: un cursor sobre bytes que produce `Vec<Token>` con spans exactos.
2. `Parser` — el descendente recursivo: una función por regla de la EBNF, que produce `Query`.
3. `LexError` y `ParseError` — las dos culpas, tipadas y con posición.

El hito que abre la Parte IV por dentro: `parse("MATCH (p:Person) RETURN p")` debe devolver una consulta válida.

## 18.2 Problema

Tienes el fuente `"MATCH (p:Person) RETURN p"` — 25 caracteres en un `&str`— y un `Query` con `Expression::Variable` en el RETURN. Entre ambos hay un abismo: el string no tiene estructura, el AST es todo estructura.

El problema se parte en dos, y esa partición es el 80 % de las decisiones del capítulo:

- **¿Cómo corto el texto en piezas?** El texto no trae espacios fiables: `(p:Person)-[:KNOWS]->(f)` es una sola palabra para `split_whitespace`, y `<>` son dos caracteres que significan UN token.
- **¿Cómo compruebo que las piezas forman una frase válida?** Y cuando no lo son, ¿quién lo dice y señalando a qué byte?

## 18.3 Modelo mental

Piensa en una **oficina de registro con dos funcionarios**:

```
 "MATCH (p:Person) RETURN p"          EBNF del cap. 17
        │                                   │
        ▼                                   │
┌─────────────────┐                         │
│  LEXER (oficial)│  corta el rollo de bytes en palabras
│                 │  y sella cada una: [nace, muere]
└─────────────────┘                         │
        │  Vec<Token>                       │
        ▼                                   ▼
┌─────────────────┐    ┌──────────────────────────┐
│ PARSER (gramático)   │  "MATCH ( )" luego "p",
│                 │    │  luego RETURN... ¿encaja? │
└─────────────────┘    └──────────────────────────┘
        │
        ▼
      Query
```

El **lexer** es el oficial que recibe el rollo continuo de papel y lo corta en palabras sueltas, estampando en cada una su certificado de origen: el `Span` con el byte donde nace y el byte donde muere. No opina sobre si la frase tiene sentido; su trabajo es cortar bien y certificar.

El **parser** es el gramático: recibe la bandeja de palabras numeradas y, con la EBNF del cap. 17 bajo el brazo, comprueba que forman frase. Nunca toca un byte crudo.

Y aquí está la lección oscura del modelo: **si el oficial corta mal, el gramático culpa a un inocente**. Un certificado falso no rompe la oficina de registro: rompe la inspección, en otro mostrador. Guarda esa idea para el §18.8.

## 18.4 Primera solución

Lo más simple que parece funcionar: métodos de `str` y buen ojo.

```rust
// Solución ingenua: trocear por espacios y mirar prefijos.
for palabra in src.split_whitespace() {
    if palabra.starts_with('(') { /* empieza nodo... */ }
    if palabra.contains("->")   { /* flecha... */ }
}
```

Con `"MATCH (p) RETURN p.name` — espacios perfectos, sin adornos — hasta avanza. Los tests del happy path pasan. Y durante un rato nadie se queja.

## 18.5 Sus límites

Hasta que llegan consultas reales:

1. **`(p:Person)-[:KNOWS]->(f:Person)`** no tiene espacios entre piezas: `split_whitespace` lo devuelve entero. Necesitarías `starts_with` en cascada… re-inventando el lexer, pero mal.
2. **`<>`, `<=`, `<-`, `->`, `--`** comparten prefijos: `contains("<")` no distingue menor-que de distinto-de.
3. **`"Ana García"`** — un string con un espacio — se parte en dos basuras.
4. **Cero posiciones.** Cuando algo falla, lo único que puedes decir es «consulta inválida». Ni byte, ni línea. Compáralo con rustc señalando el carácter exacto.
5. **UTF-8.** `p.name = "cañón"` descuadra cualquier aritmética pensada en caracteres: `ñ` ocupa 2 bytes.

## 18.6 Solución evolucionada

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap18_lexer_parser.rs`. Leámoslo por partes, porque cada decisión tiene un porqué.

### El lexer: un cursor y una regla de oro

```rust
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: u32,
}
```

Dos campos. El fuente como **bytes** (`&[u8]`, no `chars()`): el `Span` del cap. 17 mide bytes, y el cursor debe avanzar en la misma unidad que el span certifica — si no, los offsets mienten. Es la misma disciplina del cap. 9 con el little-endian: **declara la unidad, no la dejes implícita**. Y `pos: u32` porque ningún fuente didáctico supera 4 GiB.

Sobre ese cursor, el bucle principal es el escaneo canónico: saltar espacios, mirar el primer byte, y según ese byte consumir el resto del token:

```rust
pub fn lex(mut self) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    while !self.is_at_end() {
        self.skip_whitespace();
        if self.is_at_end() { break; }
        tokens.push(self.scan_token()?);
    }
    tokens.push(Token::new(TokenKind::Eof, Span::at(self.pos)));
    Ok(tokens)
}
```

Fíjate en el último `push`: **siempre hay un token `Eof` al final**. Gracias a eso, `peek()` en el parser devuelve `&Token`, jamás `Option<Token>`, y «me quedé sin tokens» (`UnexpectedEof`) es distinguible de «encontré lo que no tocaba» (`UnexpectedToken`).

El corazón es `scan_token`, que despacha por el primer byte. Y aquí vive la regla de oro — **maximal-munch**: gana el token más largo posible. Antes de decidir que un byte es un token, prueba si con su vecino forma uno más largo:

```rust
b'-' => {
    if self.match_byte(b'>')      { TokenKind::ArrowRight }
    else if self.match_byte(b'-') { TokenKind::DashDash }
    else                           { TokenKind::Dash }
}
b'<' => {
    if self.match_byte(b'-')      { TokenKind::ArrowLeft }
    else if self.match_byte(b'=') { TokenKind::Lte }
    else if self.match_byte(b'>') { TokenKind::NotEq }
    else                           { TokenKind::Lt }
}
```

¿Por qué maximal-munch y no partir siempre en tokens de un carácter y que el parser pegue? Porque `-` y `>` sueltos en `(p)-[:X]->(f)` obligarían a CADA regla del parser a mirar pares de tokens, duplicando la lógica. Y si el lexer no prueba las combinaciones de dos bytes, `a <> b` llega como `Lt Gt` — exactamente el bug del §18.8. El matching más largo elimina la ambigüedad de raíz. Cada token se sella al salir: `Span::new(start, self.pos)` — nació en `start`, murió donde el cursor quedó. El span se **deriva** del propio escaneo, no se calcula después: deriva, no lleves en la cabeza.

### Palabras clave, números y strings

Los identificadores (`scan_identifier`) consumen `[A-Za-z_][A-Za-z0-9_]*` y se clasifican con un `match` exacto sobre el texto: `MATCH` es `TokenKind::Match`, pero `match` es `Ident("match")` — las palabras clave son **case-sensitive**, por convención de Cypher. Sin estado, sin normalización: el texto tal cual contra la lista.

`scan_number` acumula dígitos con `checked_mul`/`checked_add`: si el literal desborda `i64`, **sigue consumiendo dígitos** (para que el span del error cubra todo el literal) y luego devuelve `IntegerOverflow`. Detalle fino: `12.` sin dígitos tras el punto no es un float roto — es `Integer(12)` seguido de `Dot` (el `peek_next` exige un dígito tras el punto para formar flotante).

`scan_string` es el territorio ciego, y su inmunidad es una decisión de diseño: **dentro de las comillas, el lexer no conoce la sintaxis**. Consume bytes crudos hasta la comilla de cierre, entendiendo solo dos cosas: `\` inicia un escape (`\n \t \r \\ \" \0`; cualquier otra cosa es `InvalidEscape` con el byte culpable en el error) y `"` cierra. ¿Por qué esta ceguera es obligatoria? Porque `WHERE p.name = "Ana-García \"la grande\""` contiene `-`, `{` y comillas escapadas que NO son sintaxis. Si el interior se escaneara con las reglas generales, cualquier descripción con un guión rompería el WHERE. Y un string sin cerrar antes del final del fuente es `UnterminatedString` con el span desde la comilla inicial hasta el EOF.

### El parser: el código ES la gramática

El `Parser` es igual de austero que el `Lexer`: `tokens: Vec<Token>` y `current: usize`. Con cuatro helpers de cursor (`peek`, `check`, `match_kind`, `advance`/`expect`) construimos la correspondencia que organiza todo el módulo — **una función por regla EBNF del cap. 17**:

| Regla EBNF (cap. 17) | Función gemela |
|---|---|
| `query ::= match_clause where_clause? return_clause` | `Parser::parse` |
| `match_clause ::= 'MATCH' path_pattern (',' path_pattern)*` | `parse_match_clause` |
| `path_pattern ::= node_pattern (rel_pattern node_pattern)*` | `parse_path_pattern` |
| `node_pattern ::= '(' [variable] [':' label] ['{' prop_map '}'] ')'` | `parse_node_pattern` |
| `rel_pattern` (tres direcciones) | `parse_relationship_pattern` |
| `where_clause ::= 'WHERE' expression` | `parse_where_clause` |
| `return_clause ::= 'RETURN' return_item (',' return_item)*` | `parse_return_clause` |
| `return_item ::= expression (['AS'] alias)?` | `parse_return_item` |

Se llama **descendente recursivo predictivo**: desciende por la gramática llamando a funciones que se llaman entre sí, y es *predictivo* porque decide qué alternativa tomar mirando UN token de preanálisis (`peek`). ¿Cómo sabe `parse_path_pattern` que el camino sigue? `starts_relation`: si el token actual es `Dash`, `ArrowLeft` o `DashDash`, hay otra relación encadenada.

¿Por qué esta técnica y no las alternativas? Una **tabla LL(1)/LALR** (la lanza del dragón del libro) es compacta y detecta ambigüedades de la gramática al generarla, pero el resultado es una tabla que se depura con autopsia: cuando falla, no hay función donde poner un punto de ruptura. Un **parser Pratt** es magnífico cuando hay decenas de niveles de precedencia con expresiones densas, pero esconde la gramática en una tabla de potencias de enlace. Aquí, con diez reglas, la alternativa ganadora es la legibilidad: cuando `parse` falla, la pila de llamadas ES la derivación gramatical que estaba intentando. Wirth llevaba razón medio siglo: para un lenguaje pequeño, esto es lo que se enseña y lo que se mantiene.

### La precedencia es la pila de llamadas

Las expresiones tienen operadores con distinta fuerza: `AND` ata más que `OR`, `NOT` más que `AND`, la comparación más que todo. Nuestra solución no es una tabla: es **una cadena de funciones**, donde cada nivel consume SU operador y delega hacia abajo el más fuerte:

```rust
fn parse_or(&mut self) -> Result<Expression, ParseError> {
    let mut left = self.parse_and()?;
    while self.match_kind(&TokenKind::Or).is_some() {
        let right = self.parse_and()?;
        let span = Span::new(left.span().start, right.span().end);
        left = Expression::Or { left: Box::new(left), right: Box::new(right), span };
    }
    Ok(left)
}
```

```
parse_expression ─► parse_or ─► parse_and ─► parse_not ─► parse_comparison ─► parse_primary
   (la más floja, OR)                                              (la más fuerte: literal, p.prop, ( ))
```

El truco: `parse_or` se llama a través de `parse_and`, que se llama a través de `parse_not`… Así, cuando `parse_or` busca sus `OR`, todo lo que cuelgue debajo ya se ha agrupado con precedencia mayor. La prueba:

```text
WHERE p.x = 1 OR p.y = 2 AND p.z = 3
       └─ Compare ─┘    └──── And(Compare, Compare) ────┘
              └────────── Or(Compare, And) ──────────────┘
```

`a OR b AND c` sale como `a OR (b AND c)` — exactamente lo que verifica `parse_precedencia_or_es_menor_que_and`. Y los paréntesis del fuente (`parse_primary` regla `LParen`) rompen el orden cuando el usuario lo pide. Si quisieras cambiar la precedencia de LiraQL, moverías UNA función de sitio en la cadena: el orden de las funciones ES la precedencia, a la vista. Una tabla de precedencia habría desacoplado la especificación del código; con cuatro niveles, ese desacoplamieno solo añade indirección.

### Dos fases, dos culpas: `LexError` y `ParseError`

Los errores heredan la disciplina de los caps. 12-16 — tipados, con `Display` legible — y añaden la posición:

- **`LexError`** (5 variantes): `UnexpectedChar { byte }`, `UnterminatedString`, `InvalidEscape { byte }`, `IntegerOverflow`, `MalformedNumber`. Cada una describe qué rompió el ESCANEO.
- **`ParseError`** (la envoltura `Lex(LexError)` + 7 variantes sintácticas): `UnexpectedToken { expected, found }`, `UnexpectedEof`, `MissingMatch`, `MissingReturn`, `PathMustStartWithNode`, `MalformedRelationship`, `TrailingTokens { found }`. Cada una describe qué rompió la ESTRUCTURA.

¿Por qué dos errores y no uno? Porque son **dos fases con dos culpas**: cuando `parse` falla, el mensaje debe decir si el usuario escribió un carácter imposible (léxico) o una frase mal ordenada (sintáctico). Y se unen con el patrón idiomático de Rust:

```rust
impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        let span = e.span;
        ParseError::new(ParseErrorKind::Lex(e), span)
    }
}
```

Gracias a `From`, el `lex(src)?` dentro de `Parser::new` propaga el error léxico sin una línea extra, y `source()` conserva la cadena causal: el `ParseError` sabe que su causa raíz es un `LexError`. El `Display` de ambos remata con el sufijo de localización — ` (en 0..1)`, o `(en offset 7)` si el span es vacío — al estilo de rustc y miette.

Y la recuperación es minimalista a propósito: **primer error, mensaje claro, abort**. ¿No sería mejor reportarlos todos, como hace `validate()` en el cap. 17? No aquí: detectar el segundo error exige sincronizar (avanzar hasta un `)` o una `,` «punto de reinscripción»), y aun así los errores derivados contaminan. Un `MissingReturn` bien localizado enseña más que una cascada de tres mensajes por un solo olvido. La recuperación completa queda declarada como ejercicio y deuda.

## 18.7 Prueba de fuego

El hito del brief, tal cual, con su test real (`parse_hito_del_brief`):

```rust
let q = parse("MATCH (p:Person) RETURN p").unwrap();
assert!(q.is_valid());
assert!(matches!(q.return_clause.items[0].expr, Expression::Variable { .. }));
```

Ese `Expression::Variable` tiene historia (§18.8). La batería completa — 73 tests en `tests_lexer_parser` — cubre lo que este capítulo promete, y los nombres son un mapa del territorio: `lex_comparadores` y `lex_flechas_y_guiones` (maximal-munch), `lex_span_de_token_cubre_exactamente_su_texto` (certificados: en `"MATCH (p)"`, `MATCH` es `0..5`, `(` es `6..7`, `p` es `7..8`, `)` es `8..9` — el espacio no cuenta), `lex_whitespace_no_cuenta_en_spans`, `lex_span_es_aware_a_utf8_en_bytes` (`"cañón"`: 7 caracteres, span `0..9` en bytes), `lex_string_con_escapes`, `lex_entero_desborda_i64_es_error`, `parse_where_todos_los_comparadores` (los seis operadores), las tres de precedencia, `round_trip_consulta_minima` (parsear el `Display` del AST reproduce el AST) y `parser_from_tokens_funciona` (el parser acepta tokens sintéticos sin pasar por el lexer — la prueba de que las dos fases están de verdad separadas).

Y el camino de error es parte de la prueba de fuego. La consulta ajena:

```rust
let err = parse("SELECT * FROM nada").unwrap_err();
// MissingMatch, span 0..6:
// "toda consulta LiraQL debe empezar con MATCH (en 0..6)"
```

No «consulta inválida»: la cláusula culpable, con su byte. Y la cadena rota: imagina un lexer que pierde un byte de `<>` — el token siguiente llega mal certificado y el error explota en el parser, señalando a un testigo. ¿Exageración? Lección 18.8: nos pasó.

## 18.8 Tres bugs reales, tres lecciones (caso de estudio)

El workspace dejó constancia de tres bugs durante la construcción de este capítulo. Los estudiamos porque enseñan más que el happy path.

**Bug 1: el lexer no reconocía `<>` como `NotEq`.** La rama `b'<'` probaba `match_byte(b'-')` y `match_byte(b'=')`… pero faltaba `match_byte(b'>')`. El síntoma fue diabólico: `WHERE p.age <> 30` llegaba al parser como `... Lt Gt Integer(30)`. El `parse_comparison` consumía el `Lt` contento, llamaba a `parse_primary`… y encontraba un `Gt` huérfano: *«se esperaba uno de [literal, variable.propiedad, '('], se encontró '>'»*. Señalando a un `>` que el usuario **nunca escribió como token separado**. Fix: una línea (`else if self.match_byte(b'>') { TokenKind::NotEq }`). **Lección: los errores de lexer se pagan en el parser, con intereses de distancia.** Cuando un parser culpa a un token que nadie escribió, sospecha del oficial de registro, no del gramático.

**Bug 2: `TokenKind::Dash` no existía.** El cap. 17 definió `ArrowRight` (`->`), `ArrowLeft` (`<-`) y `DashDash` (`--`)… y olvidó el guión simple. Consecuencia: los extremos `-[ ... ]-` y `]-` de las relaciones entrantes y sin dirección **no tenían token**: `MATCH (p)<-[:KNOWS]-(f)` era imposible de parsear. Fix retroactivo al vocabulario: añadir `Dash`, que el lexer produce y `parse_relationship_pattern` consume (su cierre `]-` para Incoming, o la apertura `-[` que decide Outgoing vs Undirected). **Lección: el vocabulario de tokens se diseña DESDE la gramática** — recorre las producciones y marca cada símbolo terminal; el que no aparezca en ninguna regla sobra, y el que una regla necesite y no exista, la regla muere.

**Bug 3: `Expression::Variable` no existía.** La EBNF del cap. 17 decía `primary ::= literal | property_access | '(' expression ')'` — sin variable sola. Pero el hito `RETURN p` exige referenciar el nodo completo, y un nodo **no es una propiedad**. Fix: variante nueva `Expression::Variable { name, span }` (con sus actualizaciones de `span()`, `references_var()`, `variables()` y `Display`), y en `parse_primary` la bifurcación: viene `Dot` → `PropertyAccess` (`p.name`); no viene → `Expression::var(variable, ...)` (`p`). La tentación descartada — hacer `PropertyAccess` con propiedad opcional — habría envenenado el executor del cap. 20 con un «¿`.None`?». **Lección: cada constructor sintáctico merece su variante de AST**, aunque parezcan el mismo concepto.

Los tres bugs comparten moraleja: **el diseño de la fase 1 (tokens) y la fase 3 (AST) se audita con la gramática de por medio**. Ninguno era de los difíciles: eran huecos entre capítulos.

## 18.9 Qué hemos sacrificado

1. **Recuperación multi-error**: abortamos en el primero. El coste de sincronizar puntos de reinscripción no paga en un lenguaje didáctico.
2. **Notación científica** (`1e10`) y **separadores** (`1_000`) en números: recortes declarados en `scan_number`.
3. **Operadores unarios**: `-3` se lexea como `Dash Integer(3)` y el parser lo rechaza; LiraQL no los tiene en su gramática.
4. **Comentarios** (`// ...`): trivial de añadir en `skip_whitespace`, y buen ejercicio.
5. **Palabras clave en minúsculas**: `match` es un identificador válido; la clasificación exacta mantiene el lexer sin estado.
6. **Rendimiento**: clonamos tokens al consumirlos (`advance` devuelve `Token`, no `&Token`) y el `Vec<Token>` es completo antes de parsear. Para consultas de decenas de tokens, irrelevante; un lexer de producción opera en streaming.

## 18.10 Cómo lo hace una BBDD real

En el ecosistema Rust hay tres caminos industriales, y conocerlos es responder la pregunta «¿cuándo dejar de hacerlo a mano?»:

- **`logos`** — el lexer derivado: declaras `#[derive(Logos)]` en tu `TokenKind` con atributos de regex y la macro genera el escáner (se autodenomina «el lexer más rápido del oeste»). Elimina el boilerplate de `scan_token`… y también su enseñanza: por eso la regla del Vol.II es **primero a mano, luego con crate** — la versión `logos` de LiraQL llegará al apéndice comparativo, cuando ya sepas qué está delegando.
- **`pest`** — gramática declarativa PEG en un fichero `.pest`, con posiciones y mensajes de serie. Es **scannerless**: gramática y escaneo en una sola especificación. Es exactamente la alternativa que descartamos en la decisión nº 1: elegante, compacta… y el escaneo deja de ser visible. Su recuperación multi-error, sin embargo, es superior a la nuestra.
- **`LALRPOP`** — el pariente moderno de la lanza del dragón: generador LR(1)/LALR que compila la gramática a tablas Rust. Impresionante para gramáticas grandes y estables; deprimente de depurar cuando la tabla rechaza algo que creías válido.

¿Y las bases de datos de grafos? **Neo4j** generó durante años el parser de Cypher con JavaCC — una gramática `.jj` compilada a parser. **Kùzu** (ahora Ladybug) escribió el suyo a mano: su parser de Cypher es un descendente recursivo en `src/parser/` — la misma técnica que acabas de construir, sosteniendo un lenguaje real. Y la nueva **GQL** (estándar ISO/IEC 39075:2024) se implementa sobre la misma maquinaria de siempre: lexer, parser descendente, AST. El estándar cambia el idioma; la oficina de registro, no. Hasta **rustc** — el argumento de autoridad definitivo — usa un parser descendente recursivo escrito a mano, y sus errores con span exacto son el modelo de nuestra factura de errores.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: añade comentarios de línea `// ...` a LiraQL (pista: es una línea en `skip_whitespace`). ¿Qué test demuestra que `MATCH (p) // amigo\n RETURN p` parsea igual que sin comentario?
- *Intermedio*: reescribe SOLO el lexer con `logos` (misma `TokenKind`, mismos spans) y deja el parser intacto. ¿Cuántas líneas te ahorras? ¿Qué test de la batería actual te verifica que no rompiste nada?
- *Experto*: LiraQL ganará `+ - * /` aritméticos en el cap. 22. Implementa esa expresión con un **parser Pratt** real (tabla de potencias de enlace) y compara: ¿qué gana, qué pierde, frente a alargar la cadena de funciones? Escribe ambos y discute en el PR.

## 18.11 Lo que te llevas

- **Dos fases, dos culpas**: el lexer corta bytes y certifica spans; el parser juzga tokens contra la EBNF. Nunca al revés.
- **Maximal-munch** es la regla de oro del escaneo: `->` antes que `-`, `<>` antes que `<`. Olvidar una combinación rompe el parser lejos del error.
- **El cursor mide en bytes** porque el span certifica bytes: la unidad explícita es la lección del cap. 9 aplicada al texto.
- **El código ES la gramática**: una función por regla EBNF; la precedencia es el orden de la cadena `parse_or → parse_and → parse_not → parse_comparison → parse_primary`.
- **Errores tipados con `From` + `source()` y span en el `Display`**: el usuario ve el byte culpable, no «consulta inválida».
- **El hito**: `parse("MATCH (p:Person) RETURN p")` funciona — texto dentro, AST fuera. La Parte IV tiene motor de entrada.

## 18.12 Ojo, cuidado con…

- **Consumir en un `peek`**: `peek`, `peek_at`, `peek_next` JAMÁS avanzan `pos`. Si miras avanzando, todos los spans posteriores mienten por un byte — la cadena rota en miniatura.
- **Medir spans en caracteres**: `cañón` son 7 caracteres y 9 bytes. Los offsets del certificado son bytes UTF-8, siempre.
- **Igualdad vs discriminante**: `match_kind(&TokenKind::Ident(String::new()))` casa CUALQUIER identificador porque `check` compara `std::mem::discriminant`. Es intencional (para «dame el identificador que sea»), pero sorprende.
- **Esperar `-3` como literal**: no hay unarios; es `Dash Integer(3)` y el parser lo rechaza con `UnexpectedToken`. Recorte declarado, no bug.
- **Culpar al parser**: cuando el error señala un token que nadie escribió, el bug casi siempre está un piso abajo, en el lexer.

## 18.13 Pin de batalla

> *«Un byte tragado por el lexer no rompe el lexer: rompe el parser, y en otro sitio.»*

## 18.14 Si solo lees 30 segundos

El lexer es un bucle `while` con un cursor sobre bytes que corta el fuente en tokens, cada uno con su `Span` de nacimiento y muerte — y gana siempre la combinación más larga (`<>` antes que `<`). El parser es descendente recursivo: una función por regla de la EBNF del cap. 17, con la precedencia codificada como cadena de llamadas de la más floja (`parse_or`) a la más fuerte (`parse_primary`). Los errores son tipados por fase (`LexError` → `ParseError` vía `From`), con span en el mensaje. Y el hito ya corre: `parse("MATCH (p:Person) RETURN p")`.

## 18.15 Una historia pequeña

La tarde del bug de `<>`, el test `parse_where_todos_los_comparadores` falló solo en su segundo caso. El mensaje decía *«se encontró '>'»* y señalaba un byte del medio del `WHERE`. Media hora dale que dale al parser: `parse_comparison` parecía correcto, `parse_primary` impecable… hasta que alguien imprimió los tokens de `p.age <> 30` y apareció la secuencia maldita: `Lt`, `Gt`, separados como desconocidos. El oficial de registro había cortado la palabra en dos, y el gramático llevaba toda la tarde declarando culpable a la mitad de al lado. El fix fue una línea; la lección, permanente: cuando el parser acusa a un fantasma, pidele al lexer el manifiesto de tokens y compáralo con lo que escribiste.

## Ejercicios resueltos

**1. Tokeniza a mano `MATCH (p:Person)-[:KNOWS]->(f)` con spans.**

Cuenta bytes: `MATCH` nace en 0 y muere en 5; el espacio (5) no pertenece a nadie; `(` es `6..7`; `p` `7..8`; `:` `8..9`; `Person` `9..15`; `)` `15..16`. Ahora maximal-munch: `-` en 16 prueba con `>` (17) y juntos forman `ArrowRight` `16..18`. `[` `18..19`, `:` `19..20`, `KNOWS` `20..25`, `]` `25..26`, de nuevo `ArrowRight` `26..28`, `(` `28..29`, `f` `29..30`, `)` `30..31`, `Eof` vacío en 31. Quince tokens + Eof. Verifícalo mentalmente contra `lex_span_de_token_cubre_exactamente_su_texto` y sorpréndete con lo que NO hay: ningún token de whitespace.

**2. ¿Qué AST produce `WHERE p.x = 1 OR p.y = 2 AND p.z = 3`?**

`parse_or` arranca pidiendo `parse_and`; ese `parse_and` agrupa primero `p.y = 2 AND p.z = 3` (porque dentro busca sus `AND` llamando a niveles más fuertes); solo entonces `parse_or` encuentra su `OR` y cuelga el `And` como hijo derecho. Resultado: `Or( Compare(p.x,1), And( Compare(p.y,2), Compare(p.z,3) ) )` — es decir, `a OR (b AND c)`. Es exactamente lo que asserts `parse_precedencia_or_es_menor_que_and`.

## Ejercicios propuestos

**Esencial (recordar).** Cierra el libro y el workspace. Escribe de memoria las producciones EBNF de `return_item`, `comparison` y `primary` del cap. 17, y al lado el nombre del método del parser que las implementa. Ábrelo después y contrasta con el comentario EBNF de `cap17_liraql_ast.rs`. Criterio: las tres producciones exactas y sus funciones gemelas correctas.

**Intermedio (analizar).** Predice EN PAPEL la variante exacta de error y el byte inicial de su span para: (a) `MATCH (p:Person RETURN p.name`; (b) `MATCH (p) RETURN`; (c) `MATCH (p) WHERE p.name = "Ana RETURN p`. Verifícalo con tres tests de humo. Pistas graduadas: (1) ¿qué `expect` revienta primero en (a) y qué token encuentra en su lugar?; (2) en (b), ¿qué token mira `parse_return_item` cuando busca una expresión?; (3) en (c), ¿dónde acaba el span de un string que nunca cierra?

**Experto (crear).** Implementa la notación científica en `scan_number`: `1e10`, `2.5e3` y `1.5e-3`. Decisiones que te pide el ejercicio: ¿es un solo token (maximal-munch lo es) o `1e` + `10`? ¿qué variante de `LexError` produce `1e` sin exponente? ¿el signo del exponente exige tocar la gramática del cap. 17 o solo el lexer? Añade tests estilo `lex_flotante` y haz que el span del literal cubra TODO el lexema. Criterio: cero panics, error tipado en `1e`, y `parse` acepta `WHERE p.weight > 1.5e-3`.

## Para profundizar

- **Aho, Sethi, Ullman — *Compilers: Principles, Techniques, and Tools* (1986)**: el dragón rojo. Capítulos 2-4: escaneo, autómatas, parsing. La separación de fases de este capítulo es suya.
- **Niklaus Wirth — *Compiler Construction* (Addison-Wesley, 1996; rev. 2005)**: un compilador completo de Oberon-0 en descendente recursivo, página a página. La defensa clásica de la técnica que hemos usado.
- **Robert Nystrom — *Crafting Interpreters*** (craftinginterpreters.com): los capítulos «Scanning» y «Compiling Expressions» son la versión divertida y en Java/C de exactamente este capítulo, incluida la cadena de precedencia.
- **rustc-dev-guide** (rustc-dev-guide.rust-lang.org): la sección del parser — descendente recursivo escrito a mano, con la discusión de por qué no una tabla.
- **Documentación de `logos`, `pest` y `LALRPOP`**: los tres caminos industriales del ecosistema Rust, para cuando toque el apéndice comparativo.
- **ISO/IEC 39075:2024 (GQL)** y la gramática de Cypher de openCypher: cómo se especifica formalmente un lenguaje de grafos real — nuestra EBNF del cap. 17 es su descendiente enana.

## Mini-diálogo: la sala de máquinas

> — Entonces el lexer es un `while` con un puntero, y el parser son funciones que se llaman. ¿Eso es todo? ¿Dónde está la parte difícil?

> — En las fronteras. Que el lexer sea ciego dentro de los strings. Que gane siempre el token más largo. Que el span se derive del propio escaneo. Cada una de esas fronteras mal marcada se convierte en un bug que estalla dos puertas más allá de donde se originó — pregúntale al `<>` aquel.

> — Pero LALR generaba todo esto de una tabla…

> — Y por eso la portada del libro del dragón muestra un caballero peleando. Las tablas son potentes y opacas. Aquí, cuando algo falla, abres el depurador y la pila de llamadas te dice qué regla gramatical estaba intentando cumplirse. Para un lenguaje de diez reglas, verlo todo es la característica, no la limitación. Ya tendrás dragones que matar con generadores — cuando sepas qué hacen por dentro.

---

*(Próximo capítulo: 19 — Del AST al plan lógico. Aquí el texto ya es `Query`; ahora veremos cómo el planner la baja a un árbol de operadores — `NodeScan`, `Expand`, `Filter`, `Project` — y quién decide el orden en que se filtra y se expande.)*
