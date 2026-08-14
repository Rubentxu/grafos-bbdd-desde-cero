use crate::cap07_modelo::Value;
use crate::cap17_liraql_ast::{
    CompareOp, Expression, MatchClause, NodePattern, PathPattern, Query, RelDirection,
    RelationshipPattern, ReturnClause, ReturnItem, Span, Token, TokenKind, WhereClause,
};

// ─────────────────── Cap 18: Lexer + parser descendente manual ───────────────────
//
// El cap.17 *fijó* el lenguaje LiraQL —tokens, gramática EBNF, AST, errores con
// posición— pero no construyó nada: las `Query` se armaban a mano en los tests.
// Este capítulo baja un escalón: convierte **texto** en ese AST. La cadena es
//
//   ```text
//   "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name"
//        │      lexer (cap.18)           │  parser descendente (cap.18)   │
//        └──► Vec<Token>  ─────────────►  Query  (ya validada en cap.17)
//   ```
//
// Hitos del brief (§cap 18):
//   - Parser manual pequeño (sin `nom`, sin `logos`, sin `pest`).
//   - `parse("MATCH (p:Person) RETURN p")` funciona.
//   - El lexer enseña cursores, spans, identificadores, literales, palabras
//     reservadas y errores léxicos.
//   - El parser enseña gramática, precedencia, asociatividad, recursión, AST y
//     recuperación de errores.
//
// Decisión de arquitectura (brief §11 "Lexer y parser", propuesta `logos +
// parser descendente manual`): aquí implementamos el lexer **completamente a
// mano** porque es la pieza que mejor se enseña con un bucle `while` y un
// cursor de bytes. La versión con `logos` (elimina el boilerplate del escáner
// pero deja visible el parser) y la versión con `pest` (gramática declarativa
// PEG) llegarán en caps/apéndices comparativos. La regla "primero a mano,
// luego con crate" del Vol.II exige entender el escaneo antes de delegarlo.
//
// La capa del parser es un **descendente recursivo predictivo** clásico (la
// técnica que Wirth describe en "Compiler Construction" y que Cypher/SQL
// parsean en la práctica): una función por regla de la gramática EBNF del
// cap.17, con un token de preanálisis (`peek`) que decide qué alternativa
// tomar. La precedencia de operadores (OR < AND < NOT < comparación) se
// resuelve encadenando funciones, sin tabla de precedencia.
//
// Errores: todo fallo es [`ParseError`] { kind, span }, donde `span` apunta al
// carácter exacto del fuente (estilo rustc/miette). El lexer produce
// [`LexError`] que se eleva a `ParseError` vía `From`. No hay panic, no hay
// `unwrap()`: cualquier entrada produce o bien un AST o bien una lista de
// errores legible. La recuperación es mínima e intencionada: reportar el
// primer error sintáctico con su posición y abortar (basta para un lenguaje
// didáctico; recovery completo estilo `pest` se deja como ejercicio).

// ─── LexError: fallos del escáner ───

/// Sub-tipo de error léxico (producido por el lexer del cap.18).
///
/// Cada variante describe *qué* carácter rompió el escaneo. El [`Span`]
/// acompañante en [`LexError`] localiza el problema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    /// Carácter que no inicia ningún token conocido (e.g. `@`, `#`, `!`).
    /// Lleva el byte ofensivo para el mensaje.
    UnexpectedChar { byte: u8 },
    /// Un literal string no se cerró antes de llegar a EOF.
    UnterminatedString,
    /// Secuencia de escape inválida dentro de un string (`"\q"`). Lleva el
    /// carácter encontrado tras la barra para el mensaje.
    InvalidEscape { byte: u8 },
    /// Un literal numérico se desborda `i64`.
    IntegerOverflow,
    /// Un literal numérico tiene parte fraccionaria pero ésta está vacía o
    /// contiene no-dígitos (e.g. `12.` sin dígitos tras el punto).
    MalformedNumber,
}

/// Error léxico con posición.
///
/// El lexer del cap.18 colecciona `LexError` en [`Lexer::lex`]; el parser los
/// propaga como [`ParseError`] a través de `impl From`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    pub fn new(kind: LexErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            LexErrorKind::UnexpectedChar { byte } => {
                write!(f, "carácter inesperado '{}'", escape_byte(*byte))
            }
            LexErrorKind::UnterminatedString => {
                f.write_str("string sin cerrar (EOF dentro de un literal)")
            }
            LexErrorKind::InvalidEscape { byte } => {
                write!(f, "secuencia de escape inválida '\\{}'", escape_byte(*byte))
            }
            LexErrorKind::IntegerOverflow => f.write_str("entero fuera de rango i64"),
            LexErrorKind::MalformedNumber => {
                f.write_str("número mal formado (se esperaban dígitos)")
            }
        }?;
        write_span_suffix(f, self.span)
    }
}

impl std::error::Error for LexError {}

/// Sufijo de localización común para mensajes de error léxicos/sintácticos.
pub(crate) fn write_span_suffix(f: &mut std::fmt::Formatter<'_>, span: Span) -> std::fmt::Result {
    if span.is_empty() {
        write!(f, " (en offset {})", span.start)
    } else {
        write!(f, " (en {}..{})", span.start, span.end)
    }
}

/// Muestra un byte de forma legible (ASCII imprimible o su código).
fn escape_byte(byte: u8) -> String {
    match byte {
        b'\n' => "\\n".into(),
        b'\r' => "\\r".into(),
        b'\t' => "\\t".into(),
        // 0x20..=0x7e es el rango ASCII imprimible; el espacio (0x20) cae aquí
        // también, por lo que no hace falta un caso aparte para b' '.
        0x20..=0x7e => (byte as char).to_string(),
        _ => format!("\\x{byte:02x}"),
    }
}

// ─── Lexer (tokenizer manual) ───

/// Escáner léxico de LiraQL.
///
/// Lee el fuente byte a byte (UTF-8: los caracteres multi-byte dentro de un
/// string se tratan como contenido; fuera de un string sólo se acepta ASCII)
/// y produce `Vec<Token>`. El cursor `pos` es un offset de **byte** desde el
/// inicio; cada token lleva el `Span` exacto que ocupó, para que los mensajes
/// de error del parser puedan señalar al fuente.
///
/// El bucle principal ([`Lexer::lex`]) es el ejemplo canónico de "scanning":
/// saltar espacios, mirar el primer carácter, y según éste consumir el resto
/// del token. Sin estado entre tokens (salvo el cursor), sin backtracking: el
/// matching es maximal-munch (el token más largo posible), lo que hace que
/// `->` se reconozca antes que `-`.
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: u32,
}

impl<'a> Lexer<'a> {
    /// Crea un lexer sobre el texto fuente (se trabaja con bytes).
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    /// Offset de byte actual.
    pub fn pos(&self) -> u32 {
        self.pos
    }

    /// ¿Quedan caracteres por leer?
    fn is_at_end(&self) -> bool {
        self.pos as usize >= self.src.len()
    }

    /// Byte en el offset dado (sin avanzar). `None` si fuera de rango.
    fn peek_at(&self, offset: u32) -> Option<u8> {
        self.src.get(self.pos as usize + offset as usize).copied()
    }

    /// Byte actual (sin avanzar).
    fn peek(&self) -> Option<u8> {
        self.peek_at(0)
    }

    /// Byte siguiente al actual.
    fn peek_next(&self) -> Option<u8> {
        self.peek_at(1)
    }

    /// Consume y devuelve el byte actual, avanzando el cursor.
    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    /// Consume el byte actual sólo si coincide con `expected`.
    fn match_byte(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Ejecuta el escaneo completo y devuelve la lista de tokens (con un
    /// `TokenKind::Eof` final) o el primer error léxico encontrado.
    pub fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }
            let tok = self.scan_token()?;
            tokens.push(tok);
        }
        let eof = Token::new(TokenKind::Eof, Span::at(self.pos));
        tokens.push(eof);
        Ok(tokens)
    }

    /// Salta espacios en blanco y saltos de línea (no producen token).
    ///
    /// Cypher/SQL ignoran espacios entre tokens; el lexer simplemente los
    /// descarta. El `Span` de cada token apunta sólo a su contenido útil,
    /// no al whitespace precedente (coherente con rustc/codespan).
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Lee un único token asumiendo que el cursor está en su inicio.
    ///
    /// Despacha por el primer byte; cada rama consume lo que necesite y
    /// construye el `Token` con el span `[start, pos)`.
    fn scan_token(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        let b = self.advance().ok_or_else(|| {
            // No debería ocurrir (is_at_end se comprueba antes), pero por
            // defensividad: un EOF inesperado es un error léxico.
            LexError::new(LexErrorKind::UnterminatedString, Span::at(start))
        })?;

        let kind = match b {
            // ── Puntuación simple (un solo carácter) ──
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            b'.' => TokenKind::Dot,

            // ── Flechas y guiones (maximal-munch: dos caracteres primero) ──
            b'-' => {
                if self.match_byte(b'>') {
                    TokenKind::ArrowRight
                } else if self.match_byte(b'-') {
                    TokenKind::DashDash
                } else {
                    TokenKind::Dash
                }
            }
            b'<' => {
                if self.match_byte(b'-') {
                    TokenKind::ArrowLeft
                } else if self.match_byte(b'=') {
                    TokenKind::Lte
                } else if self.match_byte(b'>') {
                    TokenKind::NotEq
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                if self.match_byte(b'=') {
                    TokenKind::Gte
                } else {
                    TokenKind::Gt
                }
            }
            b'=' => TokenKind::Eq,

            // ── Strings: " ... " con escapes \n \t \r \\ \" \0 ──
            b'"' => return self.scan_string(start),

            // ── Números: enteros y flotantes ──
            b'0'..=b'9' => return self.scan_number(start, b),

            // ── Identificadores y palabras clave ──
            // Letra o `_` inicia un identificador; el cuerpo admite dígitos.
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.scan_identifier(start, b),

            // ── Cualquier otra cosa es un error léxico ──
            other => {
                return Err(LexError::new(
                    LexErrorKind::UnexpectedChar { byte: other },
                    Span::new(start, self.pos),
                ));
            }
        };

        Ok(Token::new(kind, Span::new(start, self.pos)))
    }

    /// Lee un identificador `[A-Za-z_][A-Za-z0-9_]*` y lo clasifica:
    /// palabra clave si coincide (case-sensitive, mayúsculas por convención
    /// Cypher) o `Ident(texto)` si no.
    ///
    /// El primer byte ya se consumió en `scan_token`; aquí consumimos el
    /// resto del cuerpo. El texto se reconstruye del slice original, por lo
    /// que `first` no hace falta explícitamente (queda en la firma por
    /// simetría con `scan_number`, que sí lo usa).
    fn scan_identifier(&mut self, start: u32, _first: u8) -> TokenKind {
        while let Some(b) = self.peek() {
            if matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.src[start as usize..self.pos as usize]).unwrap_or("");
        // Clasificación case-sensitive: MATCH/AND/OR/etc. son mayúsculas.
        match text {
            "MATCH" => TokenKind::Match,
            "WHERE" => TokenKind::Where,
            "RETURN" => TokenKind::Return,
            "AS" => TokenKind::As,
            "AND" => TokenKind::And,
            "OR" => TokenKind::Or,
            "NOT" => TokenKind::Not,
            "TRUE" => TokenKind::True,
            "FALSE" => TokenKind::False,
            "NULL" => TokenKind::Null,
            _ => TokenKind::Ident(text.to_string()),
        }
    }

    /// Lee un literal numérico: `[0-9]+` (→ Integer) o `[0-9]+.[0-9]+` (→ Float).
    ///
    /// `first` es el primer dígito ya consumido. No se aceptan signos (`-3` se
    /// trata como `Dash Integer(3)` a nivel léxico; el parser podría plegarlo,
    /// pero LiraQL no tiene operadores unarios en su gramática cap.17). Tampoco
    /// notación científica (`1e10`) ni `_` separador —recortes pedagógicos.
    fn scan_number(&mut self, start: u32, first: u8) -> Result<Token, LexError> {
        let mut int_part = (first - b'0') as i64;
        let mut overflow = false;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
                let d = (b - b'0') as i64;
                int_part = match int_part.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => v,
                    None => {
                        overflow = true;
                        // Seguimos consumiendo dígitos para que el span cubra
                        // todo el literal, aunque vayamos a devolver error.
                        0
                    }
                };
            } else {
                break;
            }
        }

        // ¿Parte fraccionaria?
        if self.peek() == Some(b'.') && self.peek_next().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1; // consume '.'
            let mut frac = 0_f64;
            let mut frac_digits = 0_u32;
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() {
                    self.pos += 1;
                    frac = frac * 10.0 + (b - b'0') as f64;
                    frac_digits += 1;
                } else {
                    break;
                }
            }
            if frac_digits == 0 {
                // `12.` sin dígitos tras el punto (peek_next ya lo impidió,
                // pero por defensividad).
                return Err(LexError::new(
                    LexErrorKind::MalformedNumber,
                    Span::new(start, self.pos),
                ));
            }
            let value = int_part as f64 + frac / 10_f64.powi(frac_digits as i32);
            let kind = TokenKind::Float(value);
            return Ok(Token::new(kind, Span::new(start, self.pos)));
        }

        if overflow {
            return Err(LexError::new(
                LexErrorKind::IntegerOverflow,
                Span::new(start, self.pos),
            ));
        }
        Ok(Token::new(
            TokenKind::Integer(int_part),
            Span::new(start, self.pos),
        ))
    }

    /// Lee un literal string entre comillas dobles, procesando escapes.
    ///
    /// Escapes soportados: `\n` `\t` `\r` `\\` `\"` `\0`. Cualquier otra
    /// `\x` es `InvalidEscape`. Un string sin cerrar antes de EOF es
    /// `UnterminatedString`. El `Span` del token cubre desde la comilla
    /// inicial hasta la final (ambas incluidas); el `String(texto)` del
    /// `TokenKind` guarda sólo el contenido, ya sin comillas ni escapes.
    fn scan_string(&mut self, start: u32) -> Result<Token, LexError> {
        let mut text = String::new();
        loop {
            let b = match self.advance() {
                Some(b) => b,
                None => {
                    return Err(LexError::new(
                        LexErrorKind::UnterminatedString,
                        Span::new(start, self.pos),
                    ));
                }
            };
            match b {
                b'"' => break, // cierre
                b'\\' => {
                    let esc = match self.advance() {
                        Some(e) => e,
                        None => {
                            return Err(LexError::new(
                                LexErrorKind::UnterminatedString,
                                Span::new(start, self.pos),
                            ));
                        }
                    };
                    let ch = match esc {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'\\' => '\\',
                        b'"' => '"',
                        b'0' => '\0',
                        other => {
                            return Err(LexError::new(
                                LexErrorKind::InvalidEscape { byte: other },
                                Span::new(self.pos - 2, self.pos),
                            ));
                        }
                    };
                    text.push(ch);
                }
                // Contenido crudo (incluye UTF-8 multi-byte: lo añadimos byte
                // a byte; String valida UTF-8 al concatenar runas completas).
                _ => text.push(b as char),
            }
        }
        Ok(Token::new(
            TokenKind::String(text),
            Span::new(start, self.pos),
        ))
    }
}

/// Escanea `src` a una lista de tokens (incluye `Eof` final).
///
/// Función de conveniencia: equivalente a `Lexer::new(src).lex()`.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).lex()
}

// ─── ParseError: fallos sintácticos ───

/// Sub-tipo de error sintáctico (producido por el parser del cap.18).
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Error léxico subyacente (propagado del lexer).
    Lex(LexError),
    /// Se esperaba uno de los `expected` pero se encontró `found`.
    UnexpectedToken {
        expected: Vec<&'static str>,
        found: TokenKind,
    },
    /// Se llegó a EOF antes de completar la consulta.
    UnexpectedEof,
    /// La consulta no empieza por `MATCH`.
    MissingMatch,
    /// Falta la cláusula `RETURN` (obligatoria en LiraQL cap.17).
    MissingReturn,
    /// Un patrón de camino debe empezar por un nodo `(...)`.
    PathMustStartWithNode,
    /// Una relación mal formada (se esperaba `-[`, `<-[`, `]->` o `]-`).
    MalformedRelationship,
    /// Quedan tokens tras el `RETURN` final (basura al final de la consulta).
    TrailingTokens { found: TokenKind },
}

/// Error sintáctico con posición.
///
/// El parser es monádico en `Result<_, ParseError>`: el primer fallo aborta.
/// La recuperación multi-error (estilo `pest`) se deja como ejercicio; para un
/// lenguaje didáctico, un único mensaje claro y bien localizado es más útil
/// que una cascada de errores derivados.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Constructor ergonómico para "token inesperado".
    pub fn unexpected(found: &Token, expected: &[&'static str]) -> Self {
        Self::new(
            ParseErrorKind::UnexpectedToken {
                expected: expected.to_vec(),
                found: found.kind.clone(),
            },
            found.span,
        )
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ParseErrorKind::Lex(e) => std::fmt::Display::fmt(e, f),
            ParseErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "se esperaba uno de [")?;
                for (i, e) in expected.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{e}")?;
                }
                let desc = describe_kind(found);
                write!(f, "], se encontró {desc}")
            }
            ParseErrorKind::UnexpectedEof => {
                f.write_str("final de fichero inesperado (consulta incompleta)")
            }
            ParseErrorKind::MissingMatch => {
                f.write_str("toda consulta LiraQL debe empezar con MATCH")
            }
            ParseErrorKind::MissingReturn => f.write_str("falta la cláusula RETURN (obligatoria)"),
            ParseErrorKind::PathMustStartWithNode => {
                f.write_str("un patrón de MATCH debe empezar por un nodo '( ... )'")
            }
            ParseErrorKind::MalformedRelationship => {
                f.write_str("relación mal formada (se esperaba -[ ... ]- o <-[ ... ]-)")
            }
            ParseErrorKind::TrailingTokens { found } => {
                write!(
                    f,
                    "tokens de sobra tras RETURN: encontrado {}",
                    describe_kind(found)
                )
            }
        }?;
        write_span_suffix(f, self.span)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ParseErrorKind::Lex(e) => Some(e),
            _ => None,
        }
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        let span = e.span;
        ParseError::new(ParseErrorKind::Lex(e), span)
    }
}

/// Descripción legible de un `TokenKind` para mensajes de error.
fn describe_kind(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Match => "MATCH".into(),
        TokenKind::Where => "WHERE".into(),
        TokenKind::Return => "RETURN".into(),
        TokenKind::As => "AS".into(),
        TokenKind::And => "AND".into(),
        TokenKind::Or => "OR".into(),
        TokenKind::Not => "NOT".into(),
        TokenKind::True => "TRUE".into(),
        TokenKind::False => "FALSE".into(),
        TokenKind::Null => "NULL".into(),
        TokenKind::Ident(s) => format!("identificador '{s}'"),
        TokenKind::Integer(i) => format!("entero {i}"),
        TokenKind::Float(x) => format!("flotante {x}"),
        TokenKind::String(s) => format!("string \"{s}\""),
        TokenKind::LParen => "'('".into(),
        TokenKind::RParen => "')'".into(),
        TokenKind::LBracket => "'['".into(),
        TokenKind::RBracket => "']'".into(),
        TokenKind::LBrace => "'{'".into(),
        TokenKind::RBrace => "'}'".into(),
        TokenKind::Comma => "','".into(),
        TokenKind::Colon => "':'".into(),
        TokenKind::Dot => "'.'".into(),
        TokenKind::ArrowRight => "'->'".into(),
        TokenKind::ArrowLeft => "'<-'".into(),
        TokenKind::DashDash => "'--'".into(),
        TokenKind::Dash => "'-'".into(),
        TokenKind::Eq => "'='".into(),
        TokenKind::NotEq => "'<>'".into(),
        TokenKind::Lt => "'<'".into(),
        TokenKind::Lte => "'<='".into(),
        TokenKind::Gt => "'>'".into(),
        TokenKind::Gte => "'>='".into(),
        TokenKind::Eof => "fin de fichero".into(),
    }
}

// ─── Parser descendente recursivo ───

/// Parser predictivo de LiraQL.
///
/// Una instancia por consulta. El flujo es:
///   1. `Parser::new(src)` → lex + almacena tokens.
///   2. `Parser::parse()` → `Query` (ya estructuralmente válida).
///
/// El método [`Parser::parse`] corresponde a la regla EBNF `query`:
///
///   ```text
///   query ::= match_clause where_clause? return_clause ;
///   ```
///
/// Cada regla de la gramática del cap.17 es un método `parse_<regla>`. La
/// precedencia de operadores se resuelve con la cadena
/// `parse_or → parse_and → parse_not → parse_comparison → parse_primary`,
/// donde cada nivel consume operadores de menor precedencia y delega al
/// siguiente para los más fuertes (técnica clásica de "precedence climbing
/// por funciones").
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Construye el parser lexando `src`. No parsea todavía.
    pub fn new(src: &str) -> Result<Self, ParseError> {
        let tokens = lex(src)?;
        Ok(Self { tokens, current: 0 })
    }

    /// Construye un parser sobre un stream de tokens ya lexado (útil para
    /// tests que inyectan tokens sintéticos).
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    // ── Helpers de cursor ──

    /// Token de preanálisis (el que toca consumir).
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// ¿Estamos en EOF?
    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    /// Comprueba que el token actual es de una categoría dada.
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    /// Si el token actual coincide, lo consume y devuelve su clon; si no, None.
    fn match_kind(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    /// Consume y devuelve el token actual.
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.current].clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        tok
    }

    /// Consume el token actual si coincide con `kind`; si no, error.
    fn expect(&mut self, kind: &TokenKind, label: &'static str) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(ParseError::unexpected(self.peek(), &[label]))
        }
    }

    // ── Punto de entrada: query ::= match_clause where_clause? return_clause ──

    /// Parsea una consulta completa.
    ///
    /// Hito del brief: `parse("MATCH (p:Person) RETURN p")`.
    /// El `Span` de la `Query` resultante cubre desde el `MATCH` hasta el
    /// último token del `RETURN`.
    pub fn parse(mut self) -> Result<Query, ParseError> {
        // MATCH (obligatorio, primero).
        if !self.check(&TokenKind::Match) {
            return Err(ParseError::new(
                ParseErrorKind::MissingMatch,
                self.peek().span,
            ));
        }
        let match_clause = self.parse_match_clause()?;
        // WHERE opcional.
        let where_clause = if self.check(&TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };
        // RETURN obligatorio.
        if !self.check(&TokenKind::Return) {
            return Err(ParseError::new(
                ParseErrorKind::MissingReturn,
                self.peek().span,
            ));
        }
        let return_clause = self.parse_return_clause()?;

        // No debe quedar nada salvo EOF.
        if !self.is_at_end() {
            return Err(ParseError::new(
                ParseErrorKind::TrailingTokens {
                    found: self.peek().kind.clone(),
                },
                self.peek().span,
            ));
        }

        let span = Span::new(match_clause.span.start, return_clause.span.end);
        Ok(Query {
            match_clause,
            where_clause,
            return_clause,
            span,
        })
    }

    // ── match_clause ::= 'MATCH' path_pattern (',' path_pattern)* ──

    fn parse_match_clause(&mut self) -> Result<MatchClause, ParseError> {
        let m = self.expect(&TokenKind::Match, "MATCH")?;
        let first = self.parse_path_pattern()?;
        let mut patterns = vec![first];
        while self.match_kind(&TokenKind::Comma).is_some() {
            patterns.push(self.parse_path_pattern()?);
        }
        // Span del MATCH cubre desde la keyword hasta el final del último patrón.
        let end = patterns.last().map(|p| p.span.end).unwrap_or(m.span.end);
        Ok(MatchClause {
            patterns,
            span: Span::new(m.span.start, end),
        })
    }

    // ── path_pattern ::= node_pattern ( rel_pattern node_pattern )* ──

    fn parse_path_pattern(&mut self) -> Result<PathPattern, ParseError> {
        let start = self.parse_node_pattern()?;
        let span_start = start.span.start;
        let mut chain = Vec::new();
        let mut span_end = start.span.end;
        // Mientras el siguiente token inicie una relación (-[ , <-[ , -- ), encadenar.
        while self.starts_relation() {
            let rel = self.parse_relationship_pattern()?;
            let node = self.parse_node_pattern()?;
            span_end = node.span.end;
            chain.push((rel, node));
        }
        Ok(PathPattern {
            start,
            chain,
            span: Span::new(span_start, span_end),
        })
    }

    /// ¿El token actual inicia una relación?
    /// `-` (→ `-[`), `<-` (→ `<-[`), o `--` (relación sin dirección).
    fn starts_relation(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Dash | TokenKind::ArrowLeft | TokenKind::DashDash
        )
    }

    // ── node_pattern ::= '(' [variable] [':' label] ['{' prop_map '}'] ')' ──

    fn parse_node_pattern(&mut self) -> Result<NodePattern, ParseError> {
        let lparen = self.expect(&TokenKind::LParen, "'('")?;
        let mut variable: Option<String> = None;
        let mut label: Option<String> = None;
        let mut properties: Vec<(String, Expression)> = Vec::new();

        // variable opcional
        if let Some(tok) = self.match_kind(&TokenKind::Ident(String::new())) {
            variable = Some(extract_ident(&tok)?);
        }
        // :Label opcional
        if self.match_kind(&TokenKind::Colon).is_some() {
            let tok = self.expect(&TokenKind::Ident(String::new()), "nombre de etiqueta")?;
            label = Some(extract_ident(&tok)?);
        }
        // { props } opcional
        if self.match_kind(&TokenKind::LBrace).is_some() {
            if !self.check(&TokenKind::RBrace) {
                loop {
                    let key_tok =
                        self.expect(&TokenKind::Ident(String::new()), "nombre de propiedad")?;
                    let key = extract_ident(&key_tok)?;
                    self.expect(&TokenKind::Colon, "':'")?;
                    let value = self.parse_expression()?;
                    properties.push((key, value));
                    if self.match_kind(&TokenKind::Comma).is_none() {
                        break;
                    }
                }
            }
            self.expect(&TokenKind::RBrace, "'}'")?;
        }
        let rparen = self.expect(&TokenKind::RParen, "')'")?;
        Ok(NodePattern {
            variable,
            label,
            properties,
            span: Span::new(lparen.span.start, rparen.span.end),
        })
    }

    // ── rel_pattern ──
    //
    //   saliente:  '-[' [var] [':' type] ']' '-'? '>'
    //   entrante:  '<-[' [var] [':' type] ']' '-'
    //   sin dir.:  '-[' [var] [':' type] ']' '-'?   (sin '>' final)
    //
    // Para alinear con el AST del cap.17 (RelationshipPattern { direction })
    // y con su Display, usamos esta forma canónica:
    //   OUTGOING :  -[ ... ]->
    //   INCOMING :  <-[ ... ]-
    //   UNDIRECTED: -[ ... ]-
    // El lexer produce `Dash`/`ArrowLeft` para los extremos; el parser los
    // valida secuencialmente y construye el span total.

    fn parse_relationship_pattern(&mut self) -> Result<RelationshipPattern, ParseError> {
        let start_tok = self.advance(); // Dash | ArrowLeft | DashDash
        let start = start_tok.span.start;

        let direction = match start_tok.kind {
            // `-[ ... ]` → falta decidir dirección tras el cierre.
            TokenKind::Dash => None,
            // `<-[ ... ]` → entrante (el `<-` ya se consumió entero).
            TokenKind::ArrowLeft => Some(RelDirection::Incoming),
            // `--` → undirected sin corchetes (relación anónima sin tipo).
            TokenKind::DashDash => {
                // Sin corchetes: relación anónima sin dirección.
                return Ok(RelationshipPattern {
                    variable: None,
                    rel_type: None,
                    direction: RelDirection::Undirected,
                    span: Span::new(start, start_tok.span.end),
                });
            }
            other => {
                // El token no inicia una relación válida: reportarlo claro.
                return Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken {
                        expected: vec!["'-'", "'<-'", "'--'"],
                        found: other,
                    },
                    start_tok.span,
                ));
            }
        };

        // Esperamos `[`.
        if !self.check(&TokenKind::LBracket) {
            return Err(ParseError::new(
                ParseErrorKind::MalformedRelationship,
                self.peek().span,
            ));
        }
        self.advance(); // consume '['

        let mut variable: Option<String> = None;
        let mut rel_type: Option<String> = None;
        if let Some(tok) = self.match_kind(&TokenKind::Ident(String::new())) {
            variable = Some(extract_ident(&tok)?);
        }
        if self.match_kind(&TokenKind::Colon).is_some() {
            let tok = self.expect(&TokenKind::Ident(String::new()), "tipo de relación")?;
            rel_type = Some(extract_ident(&tok)?);
        }
        self.expect(&TokenKind::RBracket, "']'")?;

        // Tramo final: decide la dirección cuando empezó con `-`.
        let direction = match direction {
            // Empezó con `<-[` → debe cerrar con `-` (sin `>`).
            Some(RelDirection::Incoming) => {
                self.expect(&TokenKind::Dash, "'-'")?;
                RelDirection::Incoming
            }
            // Empezó con `-`: el cierre es `->` (OUTGOING) o `--` (UNDIRECTED).
            None => {
                if self.match_kind(&TokenKind::ArrowRight).is_some() {
                    RelDirection::Outgoing
                } else if self.match_kind(&TokenKind::Dash).is_some() {
                    RelDirection::Undirected
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::MalformedRelationship,
                        self.peek().span,
                    ));
                }
            }
            // Los demás no se alcanzan: direction viene None o Some(Incoming).
            Some(other) => other,
        };

        Ok(RelationshipPattern {
            variable,
            rel_type,
            direction,
            span: Span::new(start, self.peek().span.start),
        })
    }

    // ── where_clause ::= 'WHERE' expression ──

    fn parse_where_clause(&mut self) -> Result<WhereClause, ParseError> {
        let w = self.expect(&TokenKind::Where, "WHERE")?;
        let expr = self.parse_expression()?;
        let span = Span::new(w.span.start, expr.span().end);
        Ok(WhereClause { expr, span })
    }

    // ── return_clause ::= 'RETURN' return_item (',' return_item)* ──

    fn parse_return_clause(&mut self) -> Result<ReturnClause, ParseError> {
        let r = self.expect(&TokenKind::Return, "RETURN")?;
        let first = self.parse_return_item()?;
        let mut items = vec![first];
        while self.match_kind(&TokenKind::Comma).is_some() {
            items.push(self.parse_return_item()?);
        }
        let end = items.last().map(|i| i.span.end).unwrap_or(r.span.end);
        Ok(ReturnClause {
            items,
            span: Span::new(r.span.start, end),
        })
    }

    // ── return_item ::= expression ( 'AS' ident | ident )? ──
    //
    // Alias opcional: `f.name AS edad` o `f.name edad` (Cypher admite ambas).
    // Para distinguir "alias tras expresión" de la siguiente cláusula
    // comprobamos que el identificador no sea una palabra clave.

    fn parse_return_item(&mut self) -> Result<ReturnItem, ParseError> {
        let expr = self.parse_expression()?;
        let expr_end = expr.span();
        // `AS alias` explícito.
        if self.match_kind(&TokenKind::As).is_some() {
            let alias_tok = self.expect(&TokenKind::Ident(String::new()), "alias")?;
            let alias = extract_ident(&alias_tok)?;
            return Ok(ReturnItem {
                expr,
                alias: Some(alias),
                span: Span::new(expr_end.start, alias_tok.span.end),
            });
        }
        // `expr alias` implícito (identificador suelto que no sea keyword).
        if matches!(self.peek().kind, TokenKind::Ident(_))
            && !self.is_clause_keyword()
            && !expr_references_var_named(&expr, &self.peek_alias_if_any())
        {
            let alias_tok = self.advance();
            let alias = extract_ident(&alias_tok)?;
            return Ok(ReturnItem {
                expr,
                alias: Some(alias),
                span: Span::new(expr_end.start, alias_tok.span.end),
            });
        }
        Ok(ReturnItem {
            expr,
            alias: None,
            span: expr_end,
        })
    }

    /// ¿El token actual es una palabra clave de cláusula? (MATCH/WHERE/RETURN)
    /// Se usa para evitar confundir `RETURN a MATCH` con alias implícito.
    fn is_clause_keyword(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Match | TokenKind::Where | TokenKind::Return
        )
    }

    /// Nombre del identificador actual si lo es (para chequeo de alias).
    fn peek_alias_if_any(&self) -> String {
        match &self.peek().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => String::new(),
        }
    }

    // ── Expresiones: precedence climbing por funciones ──
    //
    // or_expr  ::= and_expr ('OR' and_expr)*
    // and_expr ::= not_expr ('AND' not_expr)*
    // not_expr ::= 'NOT' not_expr | comparison
    // comparison ::= primary ( comp_op primary )?
    // primary  ::= literal | property_access | '(' expression ')'

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;
        while self.match_kind(&TokenKind::Or).is_some() {
            let right = self.parse_and()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expression::Or {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_not()?;
        while self.match_kind(&TokenKind::And).is_some() {
            let right = self.parse_not()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expression::And {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expression, ParseError> {
        if let Some(not_tok) = self.match_kind(&TokenKind::Not) {
            let expr = self.parse_not()?;
            let span = Span::new(not_tok.span.start, expr.span().end);
            return Ok(Expression::Not {
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_primary()?;
        // Un único comparador (no encadenable: `a < b < c` no es válido).
        let op = match self.peek().kind {
            TokenKind::Eq => Some(CompareOp::Eq),
            TokenKind::NotEq => Some(CompareOp::NotEq),
            TokenKind::Lt => Some(CompareOp::Lt),
            TokenKind::Lte => Some(CompareOp::Lte),
            TokenKind::Gt => Some(CompareOp::Gt),
            TokenKind::Gte => Some(CompareOp::Gte),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.parse_primary()?;
            let span = Span::new(left.span().start, right.span().end);
            return Ok(Expression::Compare {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            });
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Integer(i) => {
                self.advance();
                Ok(Expression::lit(Value::Int(*i), tok.span))
            }
            TokenKind::Float(x) => {
                self.advance();
                Ok(Expression::lit(Value::Float(*x), tok.span))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expression::lit(Value::String(s.clone()), tok.span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::lit(Value::Bool(true), tok.span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::lit(Value::Bool(false), tok.span))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expression::lit(Value::Null, tok.span))
            }
            // variable | property_access ::= variable ('.' property)?
            // El hito del brief `RETURN p` requiere aceptar la variable sola.
            TokenKind::Ident(_) => {
                let var_tok = self.advance();
                let variable = extract_ident(&var_tok)?;
                if self.match_kind(&TokenKind::Dot).is_some() {
                    let prop_tok =
                        self.expect(&TokenKind::Ident(String::new()), "nombre de propiedad")?;
                    let property = extract_ident(&prop_tok)?;
                    let span = Span::new(var_tok.span.start, prop_tok.span.end);
                    Ok(Expression::prop(variable, property, span))
                } else {
                    // Variable sola: referencia a la variable ligada (todo el
                    // nodo/arista). Cap.18: hito `RETURN p`.
                    Ok(Expression::var(variable, var_tok.span))
                }
            }
            // '(' expression ')'
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(&TokenKind::RParen, "')'")?;
                Ok(inner)
            }
            _ => Err(ParseError::unexpected(
                self.peek(),
                &["literal", "variable.propiedad", "'('"],
            )),
        }
    }
}

/// Extrae el `String` de un `TokenKind::Ident`, o error si no lo es.
fn extract_ident(tok: &Token) -> Result<String, ParseError> {
    match &tok.kind {
        TokenKind::Ident(s) => Ok(s.clone()),
        other => Err(ParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec!["identificador"],
                found: other.clone(),
            },
            tok.span,
        )),
    }
}

/// ¿La expresión referencia una variable con ese nombre? (Evita alias
/// implícito ambiguo: `RETURN p p` sería `p` renombrado a `p`.)
fn expr_references_var_named(expr: &Expression, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    expr.references_var(name)
}

// ─── API pública: parse() y parse_query() ───

/// Parsea una consulta LiraQL completa desde texto.
///
/// Hito del brief (§cap 18): `parse("MATCH (p:Person) RETURN p")`.
///
/// Devuelve la `Query` estructuralmente correcta (su `validate()` semántico
/// sigue disponible para chequear variables/alcance). Cualquier error léxico o
/// sintáctico se devuelve como `ParseError` con su `Span`.
pub fn parse(src: &str) -> Result<Query, ParseError> {
    Parser::new(src)?.parse()
}

/// Alias de [`parse`] (nombre más explícito para quien prefiera verbos largos).
pub fn parse_query(src: &str) -> Result<Query, ParseError> {
    parse(src)
}

#[cfg(test)]
mod tests_lexer_parser {
    use super::*;
    use crate::cap17_liraql_ast::QueryErrorKind;

    fn s(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    /// Vec<Token> sin el Eof final (para comparar sólo lo producido).
    fn kinds(src: &str) -> Vec<TokenKind> {
        let toks = lex(src).expect("lex ok");
        toks.into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Eof))
            .map(|t| t.kind)
            .collect()
    }

    // ════════════════════════════════════════════════════════════════
    //  LEXER — tokens básicos
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lex_palabras_clave() {
        let ks = kinds("MATCH WHERE RETURN AS AND OR NOT TRUE FALSE NULL");
        assert_eq!(
            ks,
            vec![
                TokenKind::Match,
                TokenKind::Where,
                TokenKind::Return,
                TokenKind::As,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Not,
                TokenKind::True,
                TokenKind::False,
                TokenKind::Null,
            ]
        );
    }

    #[test]
    fn lex_palabras_clave_son_case_sensitive() {
        // minúsculas no son keywords → Ident.
        let ks = kinds("match where");
        assert_eq!(
            ks,
            vec![
                TokenKind::Ident("match".into()),
                TokenKind::Ident("where".into()),
            ]
        );
    }

    #[test]
    fn lex_identificadores() {
        let ks = kinds("p Person f1 _var Name_1");
        assert_eq!(
            ks,
            vec![
                TokenKind::Ident("p".into()),
                TokenKind::Ident("Person".into()),
                TokenKind::Ident("f1".into()),
                TokenKind::Ident("_var".into()),
                TokenKind::Ident("Name_1".into()),
            ]
        );
    }

    #[test]
    fn lex_puntuacion_simple() {
        let ks = kinds("(){}[],:.");
        assert_eq!(
            ks,
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Dot,
            ]
        );
    }

    #[test]
    fn lex_flechas_y_guiones() {
        // - -> -- <- < <= > >=
        let ks = kinds("- -> -- <- < <= > >=");
        assert_eq!(
            ks,
            vec![
                TokenKind::Dash,
                TokenKind::ArrowRight,
                TokenKind::DashDash,
                TokenKind::ArrowLeft,
                TokenKind::Lt,
                TokenKind::Lte,
                TokenKind::Gt,
                TokenKind::Gte,
            ]
        );
    }

    #[test]
    fn lex_comparadores() {
        let ks = kinds("= <> < <= > >=");
        assert_eq!(
            ks,
            vec![
                TokenKind::Eq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Lte,
                TokenKind::Gt,
                TokenKind::Gte,
            ]
        );
    }

    #[test]
    fn lex_eof_se_anade_al_final() {
        let toks = lex("MATCH").unwrap();
        assert_eq!(toks.len(), 2);
        assert!(matches!(toks[0].kind, TokenKind::Match));
        assert!(matches!(toks.last().unwrap().kind, TokenKind::Eof));
    }

    #[test]
    fn lex_cadena_vacia_solo_eof() {
        let toks = lex("").unwrap();
        assert_eq!(toks.len(), 1);
        assert!(matches!(toks[0].kind, TokenKind::Eof));
        assert_eq!(toks[0].span, Span::at(0));
    }

    // ── Spans correctos ──

    #[test]
    fn lex_span_de_token_cubre_exactamente_su_texto() {
        let toks = lex("MATCH (p)").unwrap();
        // MATCH = 0..5, ' ' = 5..6 (saltado), ( = 6..7, p = 7..8, ) = 8..9
        assert_eq!(toks[0].span, s(0, 5)); // MATCH
        assert_eq!(toks[1].span, s(6, 7)); // (
        assert_eq!(toks[2].span, s(7, 8)); // p
        assert_eq!(toks[3].span, s(8, 9)); // )
    }

    #[test]
    fn lex_whitespace_no_cuenta_en_spans() {
        let toks = lex("  MATCH   (p)  ").unwrap();
        // El primer token real (MATCH) empieza en offset 2.
        assert_eq!(toks[0].span, s(2, 7));
    }

    #[test]
    fn lex_span_es_aware_a_utf8_en_bytes() {
        // "ñ" = 2 bytes en UTF-8. Fuera de string se rechaza como UnexpectedChar,
        // pero dentro de un string se cuenta como contenido.
        let toks = lex("\"cañón\"").unwrap();
        assert!(matches!(toks[0].kind, TokenKind::String(_)));
        // 7 caracteres = 9 bytes (ñ y ó suman 2 bytes cada una).
        assert_eq!(toks[0].span, s(0, 9));
    }

    // ════════════════════════════════════════════════════════════════
    //  LEXER — strings
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lex_string_simple() {
        let ks = kinds("\"Ana\"");
        assert_eq!(ks, vec![TokenKind::String("Ana".into())]);
    }

    #[test]
    fn lex_string_vacio() {
        assert_eq!(kinds("\"\""), vec![TokenKind::String(String::new())]);
    }

    #[test]
    fn lex_string_con_escapes() {
        // \n \t \r \\ \" \0
        let ks = kinds(r#""a\nb\tc\\d\"e\0f""#);
        assert_eq!(ks, vec![TokenKind::String("a\nb\tc\\d\"e\0f".into())]);
    }

    #[test]
    fn lex_string_sin_cerrar_es_error() {
        let err = lex("\"sin cerrar").unwrap_err();
        assert!(matches!(err.kind, LexErrorKind::UnterminatedString));
        assert_eq!(err.span, s(0, 11));
    }

    #[test]
    fn lex_escape_invalido_es_error() {
        let err = lex(r#""\q""#).unwrap_err();
        match err.kind {
            LexErrorKind::InvalidEscape { byte } => assert_eq!(byte, b'q'),
            other => panic!("esperaba InvalidEscape, tuve {other:?}"),
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  LEXER — números
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lex_entero_simple() {
        assert_eq!(kinds("42"), vec![TokenKind::Integer(42)]);
    }

    #[test]
    fn lex_entero_cero() {
        assert_eq!(kinds("0"), vec![TokenKind::Integer(0)]);
    }

    #[test]
    fn lex_entero_grande() {
        assert_eq!(kinds("1234567890"), vec![TokenKind::Integer(1234567890)]);
    }

    #[test]
    fn lex_flotante() {
        // 2.5 (no es una constante matemática famosa → no dispara approx_constant).
        assert_eq!(kinds("2.5"), vec![TokenKind::Float(2.5)]);
    }

    #[test]
    fn lex_flotante_con_ceros() {
        // 1.50 → 1.5
        let ks = kinds("1.50");
        assert_eq!(ks, vec![TokenKind::Float(1.5)]);
    }

    #[test]
    fn lex_flotante_cero_coma_algo() {
        assert_eq!(kinds("0.5"), vec![TokenKind::Float(0.5)]);
    }

    #[test]
    fn lex_entero_punto_sin_digitos_es_solo_entero() {
        // `12.` sin dígitos tras el punto → Integer(12) + Dot (no Float).
        // (peek_next exige dígito tras el punto para formar Float.)
        let ks = kinds("12.");
        assert_eq!(ks, vec![TokenKind::Integer(12), TokenKind::Dot]);
    }

    #[test]
    fn lex_entero_desborda_i64_es_error() {
        // i64::MAX = 9_223_372_036_854_775_807; sumarle un dígito más > overflow.
        let err = lex("99999999999999999999").unwrap_err();
        assert!(matches!(err.kind, LexErrorKind::IntegerOverflow));
    }

    // ════════════════════════════════════════════════════════════════
    //  LEXER — errores
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn lex_caracter_inesperado_es_error() {
        let err = lex("@").unwrap_err();
        match err.kind {
            LexErrorKind::UnexpectedChar { byte } => assert_eq!(byte, b'@'),
            other => panic!("esperaba UnexpectedChar, tuve {other:?}"),
        }
        assert_eq!(err.span, s(0, 1));
    }

    #[test]
    fn lex_caracter_inesperado_hex() {
        let err = lex("\x01").unwrap_err();
        match err.kind {
            LexErrorKind::UnexpectedChar { byte } => assert_eq!(byte, 0x01),
            other => panic!("esperaba UnexpectedChar, tuve {other:?}"),
        }
    }

    #[test]
    fn lex_error_display_incluye_span() {
        let err = lex("@").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("carácter inesperado"));
        assert!(msg.contains("0..1"));
    }

    #[test]
    fn lex_error_display_string_sin_cerrar() {
        let err = lex("\"abc").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("string sin cerrar"));
    }

    #[test]
    fn lex_error_implements_std_error() {
        let err: LexError = lex("@").unwrap_err();
        let _e: &dyn std::error::Error = &err;
    }

    // ════════════════════════════════════════════════════════════════
    //  PARSER — hito del brief
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_hito_del_brief() {
        // Hito §cap 18: parse("MATCH (p:Person) RETURN p")
        // `RETURN p` referencia la variable ligada `p` (todo el nodo). El
        // cap.18 añade Expression::Variable precisamente para este hito.
        let q = parse("MATCH (p:Person) RETURN p").unwrap();
        assert!(q.is_valid());
        assert_eq!(q.match_clause.patterns.len(), 1);
        let path = &q.match_clause.patterns[0];
        assert_eq!(path.start.variable.as_deref(), Some("p"));
        assert_eq!(path.start.label.as_deref(), Some("Person"));
        assert!(path.chain.is_empty());
        assert_eq!(q.return_clause.items.len(), 1);
        assert!(matches!(
            q.return_clause.items[0].expr,
            Expression::Variable { .. }
        ));
        assert!(q.where_clause.is_none());
    }

    // ════════════════════════════════════════════════════════════════
    //  PARSER — MATCH (patrones de nodo y relación)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_node_pattern_solo_variable() {
        let q = parse("MATCH (p) RETURN p.name").unwrap();
        assert_eq!(
            q.match_clause.patterns[0].start.variable.as_deref(),
            Some("p")
        );
        assert!(q.match_clause.patterns[0].start.label.is_none());
    }

    #[test]
    fn parse_node_pattern_solo_label() {
        let q = parse("MATCH (:Person) RETURN p.name").unwrap();
        assert!(q.match_clause.patterns[0].start.variable.is_none());
        assert_eq!(
            q.match_clause.patterns[0].start.label.as_deref(),
            Some("Person")
        );
    }

    #[test]
    fn parse_node_pattern_anonimo_solo_aceptado_sintacticamente() {
        // () se parsea (estructura válida), aunque validate() lo marque como
        // EmptyNodePattern (regla semántica del cap.17).
        let q = parse("MATCH () RETURN p.name").unwrap();
        let errs = q.validate();
        assert!(
            errs.iter()
                .any(|e| matches!(e.kind, QueryErrorKind::EmptyNodePattern))
        );
    }

    #[test]
    fn parse_node_pattern_con_propiedades() {
        let q = parse(r#"MATCH (p:Person {name: "Ana", age: 30}) RETURN p.name"#).unwrap();
        let node = &q.match_clause.patterns[0].start;
        assert_eq!(node.properties.len(), 2);
        assert_eq!(node.properties[0].0, "name");
        assert_eq!(node.properties[1].0, "age");
    }

    #[test]
    fn parse_path_con_relacion_saliente() {
        let q = parse("MATCH (p:Person)-[:KNOWS]->(f:Person) RETURN f.name").unwrap();
        let path = &q.match_clause.patterns[0];
        assert_eq!(path.chain.len(), 1);
        let (rel, node) = &path.chain[0];
        assert_eq!(rel.direction, RelDirection::Outgoing);
        assert_eq!(rel.rel_type.as_deref(), Some("KNOWS"));
        assert_eq!(node.variable.as_deref(), Some("f"));
    }

    #[test]
    fn parse_path_con_relacion_entrante() {
        let q = parse("MATCH (p:Person)<-[:KNOWS]-(f:Person) RETURN f.name").unwrap();
        let (rel, _node) = &q.match_clause.patterns[0].chain[0];
        assert_eq!(rel.direction, RelDirection::Incoming);
    }

    #[test]
    fn parse_path_con_relacion_sin_direccion() {
        // -[:T]- (sin >)
        let q = parse("MATCH (p:Person)-[:KNOWS]-(f:Person) RETURN f.name").unwrap();
        let (rel, _node) = &q.match_clause.patterns[0].chain[0];
        assert_eq!(rel.direction, RelDirection::Undirected);
    }

    #[test]
    fn parse_path_con_relacion_con_variable() {
        let q = parse("MATCH (p)-[r:KNOWS]->(f) RETURN p.name").unwrap();
        let (rel, _) = &q.match_clause.patterns[0].chain[0];
        assert_eq!(rel.variable.as_deref(), Some("r"));
    }

    #[test]
    fn parse_path_con_relacion_anonima_sin_tipo() {
        // -[]-> (corchetes vacíos)
        let q = parse("MATCH (p)-[]->(f) RETURN p.name").unwrap();
        let (rel, _) = &q.match_clause.patterns[0].chain[0];
        assert!(rel.variable.is_none());
        assert!(rel.rel_type.is_none());
        assert_eq!(rel.direction, RelDirection::Outgoing);
    }

    #[test]
    fn parse_path_largo_tres_nodos() {
        let q = parse("MATCH (a)-[:X]->(b)-[:Y]->(c) RETURN a.name").unwrap();
        let path = &q.match_clause.patterns[0];
        assert_eq!(path.chain.len(), 2);
    }

    #[test]
    fn parse_multiples_patrones_separados_por_coma() {
        let q = parse("MATCH (a:Person), (b:City) RETURN a.name").unwrap();
        assert_eq!(q.match_clause.patterns.len(), 2);
    }

    // ════════════════════════════════════════════════════════════════
    //  PARSER — WHERE (expresiones y precedencia)
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_where_comparacion_simple() {
        let q = parse(r#"MATCH (p:Person) WHERE p.name = "Ana" RETURN p.name"#).unwrap();
        let where_c = q.where_clause.expect("where presente");
        match where_c.expr {
            Expression::Compare { op, .. } => assert_eq!(op, CompareOp::Eq),
            other => panic!("esperaba Compare, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_where_todos_los_comparadores() {
        for (src, expected) in [
            ("p.age = 30", CompareOp::Eq),
            ("p.age <> 30", CompareOp::NotEq),
            ("p.age < 30", CompareOp::Lt),
            ("p.age <= 30", CompareOp::Lte),
            ("p.age > 30", CompareOp::Gt),
            ("p.age >= 30", CompareOp::Gte),
        ] {
            let q = parse(&format!("MATCH (p:Person) WHERE {src} RETURN p.name")).unwrap();
            match q.where_clause.unwrap().expr {
                Expression::Compare { op, .. } => assert_eq!(op, expected, "para {src}"),
                other => panic!("para {src}: esperaba Compare, tuve {other:?}"),
            }
        }
    }

    #[test]
    fn parse_where_and() {
        let q = parse("MATCH (p:Person) WHERE p.age > 18 AND p.age < 65 RETURN p.name").unwrap();
        match q.where_clause.unwrap().expr {
            Expression::And { .. } => {}
            other => panic!("esperaba And, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_where_or() {
        let q = parse(r#"MATCH (p:Person) WHERE p.name = "Ana" OR p.name = "Beto" RETURN p.name"#)
            .unwrap();
        match q.where_clause.unwrap().expr {
            Expression::Or { .. } => {}
            other => panic!("esperaba Or, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_where_not() {
        let q = parse("MATCH (p:Person) WHERE NOT p.age > 18 RETURN p.name").unwrap();
        match q.where_clause.unwrap().expr {
            Expression::Not { .. } => {}
            other => panic!("esperaba Not, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_precedencia_or_es_menor_que_and() {
        // a OR b AND c  →  a OR (b AND c)
        let q = parse("MATCH (p) WHERE p.x = 1 OR p.y = 2 AND p.z = 3 RETURN p.name").unwrap();
        match q.where_clause.unwrap().expr {
            Expression::Or { left, right, .. } => {
                assert!(matches!(*left, Expression::Compare { .. }));
                assert!(matches!(*right, Expression::And { .. }));
            }
            other => panic!("esperaba Or en raíz, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_precedencia_and_es_menor_que_not() {
        // a AND NOT b  →  a AND (NOT b)
        let q = parse("MATCH (p) WHERE p.x = 1 AND NOT p.y = 2 RETURN p.name").unwrap();
        match q.where_clause.unwrap().expr {
            Expression::And { left, right, .. } => {
                assert!(matches!(*left, Expression::Compare { .. }));
                assert!(matches!(*right, Expression::Not { .. }));
            }
            other => panic!("esperaba And en raíz, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_precedencia_parentesis_rompe_orden() {
        // (a OR b) AND c  →  AND(Or(a,b), c)
        let q = parse("MATCH (p) WHERE (p.x = 1 OR p.y = 2) AND p.z = 3 RETURN p.name").unwrap();
        match q.where_clause.unwrap().expr {
            Expression::And { left, right, .. } => {
                assert!(matches!(*left, Expression::Or { .. }));
                assert!(matches!(*right, Expression::Compare { .. }));
            }
            other => panic!("esperaba And en raíz, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_where_sin_clausula_es_none() {
        let q = parse("MATCH (p:Person) RETURN p.name").unwrap();
        assert!(q.where_clause.is_none());
    }

    #[test]
    fn parse_literal_true_false_null() {
        for (src, val) in [
            ("TRUE", Value::Bool(true)),
            ("FALSE", Value::Bool(false)),
            ("NULL", Value::Null),
        ] {
            let q = parse(&format!("MATCH (p) WHERE p.x = {src} RETURN p.name")).unwrap();
            match q.where_clause.unwrap().expr {
                Expression::Compare { right, .. } => match *right {
                    Expression::Literal { value, .. } => assert_eq!(value, val),
                    other => panic!("para {src}: esperaba Literal, tuve {other:?}"),
                },
                other => panic!("para {src}: esperaba Compare, tuve {other:?}"),
            }
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  PARSER — RETURN
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_return_varios_items() {
        let q = parse("MATCH (p:Person) RETURN p.name, p.age, p.name").unwrap();
        assert_eq!(q.return_clause.items.len(), 3);
    }

    #[test]
    fn parse_return_con_alias_as() {
        let q = parse("MATCH (p:Person) RETURN p.age AS edad").unwrap();
        assert_eq!(q.return_clause.items[0].alias.as_deref(), Some("edad"));
    }

    #[test]
    fn parse_return_con_alias_implicito() {
        // RETURN p.age edad (sin AS)
        let q = parse("MATCH (p:Person) RETURN p.age edad").unwrap();
        assert_eq!(q.return_clause.items[0].alias.as_deref(), Some("edad"));
    }

    #[test]
    fn parse_return_sin_alias() {
        let q = parse("MATCH (p:Person) RETURN p.name").unwrap();
        assert!(q.return_clause.items[0].alias.is_none());
    }

    // ════════════════════════════════════════════════════════════════
    //  PARSER — errores de sintaxis
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_error_no_empieza_por_match() {
        let err = parse("(p:Person) RETURN p.name").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::MissingMatch));
    }

    #[test]
    fn parse_error_falta_return() {
        let err = parse("MATCH (p:Person)").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::MissingReturn));
    }

    #[test]
    fn parse_error_falta_match_completamente_vacio() {
        let err = parse("").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::MissingMatch));
    }

    #[test]
    fn parse_error_token_inesperado_en_node() {
        // Falta ')'.
        let err = parse("MATCH (p:Person RETURN p.name").unwrap_err();
        match err.kind {
            ParseErrorKind::UnexpectedToken { expected, .. } => {
                assert!(expected.iter().any(|e| e.contains("')'")));
            }
            other => panic!("esperaba UnexpectedToken, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_error_tokens_sobra_al_final() {
        // Tras RETURN p.name, un token que no puede extender la consulta
        // (una palabra clave suelta como una segunda cláusula RETURN).
        let err = parse("MATCH (p:Person) RETURN p.name RETURN").unwrap_err();
        match err.kind {
            ParseErrorKind::TrailingTokens { found, .. } => {
                assert!(matches!(found, TokenKind::Return));
            }
            other => panic!("esperaba TrailingTokens, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_error_propagado_del_lexer() {
        // '@' es un error léxico; el parser lo recibe como ParseErrorKind::Lex.
        let err = parse("MATCH (p@) RETURN p.name").unwrap_err();
        match err.kind {
            ParseErrorKind::Lex(inner) => {
                assert!(matches!(inner.kind, LexErrorKind::UnexpectedChar { .. }));
            }
            other => panic!("esperaba Lex, tuve {other:?}"),
        }
    }

    #[test]
    fn parse_error_display_incluye_span_y_mensaje() {
        let err = parse("MATCH (p:Person").unwrap_err();
        let msg = format!("{err}");
        // Mensaje útil + localización.
        assert!(!msg.is_empty());
        assert!(msg.contains("offset") || msg.contains(".."));
    }

    #[test]
    fn parse_error_implements_std_error_con_source() {
        let err = parse("MATCH (p@)").unwrap_err();
        let e: &dyn std::error::Error = &err;
        assert!(e.source().is_some(), "ParseError::Lex debe exponer source");
    }

    // ════════════════════════════════════════════════════════════════
    //  ROUND-TRIP — parse(display(ast)) == ast
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn round_trip_consulta_minima() {
        let src = "MATCH (p:Person) RETURN p.name";
        let q1 = parse(src).unwrap();
        let rendered = format!("{q1}");
        let q2 = parse(&rendered).unwrap();
        assert_eq!(q1, q2, "round-trip no idempotente: {rendered}");
    }

    #[test]
    fn round_trip_consulta_completa() {
        let src = r#"MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE (p.name = "Ana") RETURN f.name AS amigo"#;
        let q1 = parse(src).unwrap();
        let rendered = format!("{q1}");
        let q2 = parse(&rendered).unwrap();
        assert_eq!(q1, q2, "round-trip no idempotente: {rendered}");
    }

    #[test]
    fn round_trip_consulta_con_and_or() {
        // El fuente puede llevar paréntesis redundantes que el Display
        // canonicaliza (los paréntesis de agrupación no se preservan: la
        // estructura del AST ya codifica la precedencia). Verificamos por
        // tanto la **idempotencia de la forma canónica**: parsear el Display
        // dos veces produce el mismo texto (la forma normalizada es estable).
        let src =
            r#"MATCH (p:Person) WHERE ((p.age > 18 AND p.age < 65) OR p.vip = TRUE) RETURN p.name"#;
        let canonical1 = format!("{}", parse(src).unwrap());
        let canonical2 = format!("{}", parse(&canonical1).unwrap());
        assert_eq!(
            canonical1, canonical2,
            "la forma canónica no es idempotente"
        );
        // Y la estructura (sin spans) coincide entre ambas.
        let q1 = parse(&canonical1).unwrap();
        let q2 = parse(&canonical2).unwrap();
        assert_eq!(q1.match_clause, q2.match_clause);
        assert_eq!(q1.return_clause, q2.return_clause);
    }

    #[test]
    fn round_trip_mantiene_direccion_de_relacion() {
        for (src, dir) in [
            ("MATCH (a)-[:X]->(b) RETURN a.n", RelDirection::Outgoing),
            ("MATCH (a)<-[:X]-(b) RETURN a.n", RelDirection::Incoming),
            ("MATCH (a)-[:X]-(b) RETURN a.n", RelDirection::Undirected),
        ] {
            let q = parse(src).unwrap();
            let (rel, _) = &q.match_clause.patterns[0].chain[0];
            assert_eq!(rel.direction, dir, "para {src}");
            // Y el round-trip reproduce la misma dirección.
            let rendered = format!("{q}");
            let q2 = parse(&rendered).unwrap();
            let (rel2, _) = &q2.match_clause.patterns[0].chain[0];
            assert_eq!(rel2.direction, dir, "round-trip cambió dirección");
        }
    }

    // ════════════════════════════════════════════════════════════════
    //  CONSULTAS COMPLETAS DEL BRIEF
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn consulta_ejemplo_cap17_brief() {
        // Ejemplo canónico del cap.17 (encabezado de la sección).
        let src = r#"MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name, p.age AS edad"#;
        let q = parse(src).unwrap();
        assert!(q.is_valid(), "debe ser semánticamente válida");
        assert_eq!(q.return_clause.items.len(), 2);
        assert_eq!(q.return_clause.items[1].alias.as_deref(), Some("edad"));
    }

    #[test]
    fn consulta_ejemplo_cap19_brief() {
        // Ejemplo del cap.19 (AST→plan): mismo patrón, WHERE con =
        let src = r#"MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name"#;
        let q = parse(src).unwrap();
        assert!(q.is_valid());
    }

    #[test]
    fn consulta_con_propiedades_inline_y_where() {
        let src = r#"MATCH (p:Person {active: TRUE}) WHERE p.age >= 18 RETURN p.name AS nombre"#;
        let q = parse(src).unwrap();
        assert!(q.is_valid());
        assert_eq!(q.match_clause.patterns[0].start.properties.len(), 1);
    }

    // ════════════════════════════════════════════════════════════════
    //  API pública
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn parse_query_alias_de_parse() {
        let q1 = parse("MATCH (p:Person) RETURN p.name").unwrap();
        let q2 = parse_query("MATCH (p:Person) RETURN p.name").unwrap();
        assert_eq!(q1, q2);
    }

    #[test]
    fn parser_from_tokens_funciona() {
        // Inyección directa de tokens (sin pasar por el lexer).
        let tokens = vec![
            Token::new(TokenKind::Match, s(0, 5)),
            Token::new(TokenKind::LParen, s(6, 7)),
            Token::new(TokenKind::Ident("p".into()), s(7, 8)),
            Token::new(TokenKind::Colon, s(8, 9)),
            Token::new(TokenKind::Ident("Person".into()), s(9, 15)),
            Token::new(TokenKind::RParen, s(15, 16)),
            Token::new(TokenKind::Return, s(17, 23)),
            Token::new(TokenKind::Ident("p".into()), s(24, 25)),
            Token::new(TokenKind::Dot, s(25, 26)),
            Token::new(TokenKind::Ident("name".into()), s(26, 30)),
            Token::new(TokenKind::Eof, s(30, 30)),
        ];
        let p = Parser::from_tokens(tokens);
        let q = p.parse().unwrap();
        assert_eq!(
            q.match_clause.patterns[0].start.variable.as_deref(),
            Some("p")
        );
    }

    #[test]
    fn describe_kind_cubre_todas_las_variantes() {
        // Smoke: describir cada variante no cae en panic.
        for k in [
            TokenKind::Match,
            TokenKind::Ident("x".into()),
            TokenKind::Integer(1),
            TokenKind::Float(1.0),
            TokenKind::String("s".into()),
            TokenKind::Dash,
            TokenKind::Eof,
        ] {
            let _ = describe_kind(&k);
        }
    }
}

#[cfg(test)]
mod tests_query {
    use super::*;
    use crate::cap17_liraql_ast::{AstNode, QueryError, QueryErrorKind, hex_bytes};

    // ─── Helpers para construir ASTs de test sin parser ───

    fn s(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    /// `(p:Person)` con variable y label.
    fn person_node(var: &str, label: &str, span: Span) -> NodePattern {
        NodePattern {
            variable: Some(var.to_string()),
            label: Some(label.to_string()),
            properties: Vec::new(),
            span,
        }
    }

    /// `-[:KNOWS]->` saliente anónima con tipo.
    fn knows_rel(span: Span) -> RelationshipPattern {
        RelationshipPattern {
            variable: None,
            rel_type: Some("KNOWS".to_string()),
            direction: RelDirection::Outgoing,
            span,
        }
    }

    /// `MATCH (p:Person) RETURN p` — consulta mínima válida.
    fn minimal_query() -> Query {
        let node = person_node("p", "Person", s(7, 18));
        let path = PathPattern {
            start: node,
            chain: Vec::new(),
            span: s(6, 19),
        };
        Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 19),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "name", s(27, 33)),
                    alias: None,
                    span: s(27, 33),
                }],
                span: s(20, 33),
            },
            span: s(0, 33),
        }
    }

    // ─── Span ───

    #[test]
    fn span_new_normaliza_orden() {
        let sp = Span::new(10, 4);
        assert_eq!(sp.start, 4);
        assert_eq!(sp.end, 10);
        assert_eq!(sp.len(), 6);
        assert!(!sp.is_empty());
    }

    #[test]
    fn span_at_es_vacio() {
        let sp = Span::at(42);
        assert_eq!(sp.start, 42);
        assert_eq!(sp.end, 42);
        assert!(sp.is_empty());
        assert_eq!(sp.len(), 0);
    }

    #[test]
    fn span_merge_cubre_a_ambos() {
        let a = Span::new(2, 5);
        let b = Span::new(10, 20);
        let m = a.merge(b);
        assert_eq!(m.start, 2);
        assert_eq!(m.end, 20);

        // Disjuntos contiguos.
        let c = Span::new(5, 8);
        assert_eq!(a.merge(c), Span::new(2, 8));
    }

    #[test]
    fn span_default_es_cero() {
        let sp = Span::default();
        assert_eq!(sp.start, 0);
        assert_eq!(sp.end, 0);
        assert!(sp.is_empty());
    }

    // ─── TokenKind / Token ───

    #[test]
    fn token_kind_eq_keywords() {
        assert_eq!(TokenKind::Match, TokenKind::Match);
        assert_ne!(TokenKind::Match, TokenKind::Where);
    }

    #[test]
    fn token_construye_con_span() {
        let t = Token::new(TokenKind::Match, s(0, 5));
        assert_eq!(t.kind, TokenKind::Match);
        assert_eq!(t.span, s(0, 5));
    }

    #[test]
    fn token_kind_cubre_todos_los_grupos() {
        // Smoke test: que todas las variantes se construyen y matchean.
        let kinds = vec![
            TokenKind::Match,
            TokenKind::Where,
            TokenKind::Return,
            TokenKind::As,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Null,
            TokenKind::Ident("p".into()),
            TokenKind::Integer(42),
            TokenKind::Float(2.5),
            TokenKind::String("hi".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::Comma,
            TokenKind::Colon,
            TokenKind::Dot,
            TokenKind::ArrowRight,
            TokenKind::ArrowLeft,
            TokenKind::DashDash,
            TokenKind::Dash,
            TokenKind::Eq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::Lte,
            TokenKind::Gt,
            TokenKind::Gte,
            TokenKind::Eof,
        ];
        // Cada uno debe ser igual a sí mismo y distinto del siguiente.
        for i in 0..kinds.len() {
            assert_eq!(kinds[i], kinds[i].clone());
            if i + 1 < kinds.len() {
                assert_ne!(kinds[i], kinds[i + 1]);
            }
        }
    }

    // ─── Expression: constructores y traversal ───

    #[test]
    fn expression_lit_y_prop() {
        let lit = Expression::lit(Value::Int(42), s(0, 2));
        assert_eq!(lit.span(), s(0, 2));
        assert!(!lit.references_var("x"));

        let prop = Expression::prop("p", "name", s(0, 6));
        assert_eq!(prop.span(), s(0, 6));
        assert!(prop.references_var("p"));
        assert!(!prop.references_var("q"));
    }

    #[test]
    fn expression_compare_recolecta_variables() {
        // p.name = "Ana"
        let left = Expression::prop("p", "name", s(0, 6));
        let right = Expression::lit(Value::String("Ana".into()), s(9, 14));
        let cmp = Expression::Compare {
            op: CompareOp::Eq,
            left: Box::new(left),
            right: Box::new(right),
            span: s(0, 14),
        };
        let mut vars = Vec::new();
        cmp.variables(&mut vars);
        assert_eq!(vars, vec!["p".to_string()]);
        assert!(cmp.references_var("p"));
    }

    #[test]
    fn expression_and_or_not_recolecta_recursivo() {
        // (p.name = "Ana" OR q.age > 30) AND NOT r.active
        let p_name = Expression::prop("p", "name", s(0, 6));
        let ana = Expression::lit(Value::String("Ana".into()), s(0, 5));
        let cmp1 = Expression::Compare {
            op: CompareOp::Eq,
            left: Box::new(p_name),
            right: Box::new(ana),
            span: s(0, 10),
        };
        let q_age = Expression::prop("q", "age", s(0, 5));
        let thirty = Expression::lit(Value::Int(30), s(0, 2));
        let cmp2 = Expression::Compare {
            op: CompareOp::Gt,
            left: Box::new(q_age),
            right: Box::new(thirty),
            span: s(0, 10),
        };
        let or_expr = Expression::Or {
            left: Box::new(cmp1),
            right: Box::new(cmp2),
            span: s(0, 20),
        };
        let r_active = Expression::prop("r", "active", s(0, 8));
        let not_expr = Expression::Not {
            expr: Box::new(r_active),
            span: s(0, 8),
        };
        let and_expr = Expression::And {
            left: Box::new(or_expr),
            right: Box::new(not_expr),
            span: s(0, 30),
        };
        let mut vars = Vec::new();
        and_expr.variables(&mut vars);
        vars.sort();
        assert_eq!(vars, vec!["p", "q", "r"]);
    }

    // ─── CompareOp ───

    #[test]
    fn compare_op_as_str_canonico() {
        assert_eq!(CompareOp::Eq.as_str(), "=");
        assert_eq!(CompareOp::NotEq.as_str(), "<>");
        assert_eq!(CompareOp::Lt.as_str(), "<");
        assert_eq!(CompareOp::Lte.as_str(), "<=");
        assert_eq!(CompareOp::Gt.as_str(), ">");
        assert_eq!(CompareOp::Gte.as_str(), ">=");
    }

    // ─── Patrones: variables declaradas ───

    #[test]
    fn path_pattern_node_variables() {
        let path = PathPattern {
            start: person_node("p", "Person", s(0, 11)),
            chain: vec![
                (knows_rel(s(11, 21)), person_node("f", "Person", s(21, 32))),
                (
                    knows_rel(s(32, 42)),
                    NodePattern {
                        variable: None,
                        label: Some("Person".into()),
                        properties: Vec::new(),
                        span: s(42, 52),
                    },
                ),
            ],
            span: s(0, 52),
        };
        // Sólo los nodos con variable: p, f.
        let node_vars = path.node_variables();
        assert_eq!(node_vars, vec!["p", "f"]);
        // El nodo anónimo final no aporta variable.
    }

    #[test]
    fn path_pattern_edge_variables_incluye_rel_var() {
        let rel_var = RelationshipPattern {
            variable: Some("r".into()),
            rel_type: Some("KNOWS".into()),
            direction: RelDirection::Outgoing,
            span: s(11, 21),
        };
        let path = PathPattern {
            start: person_node("p", "Person", s(0, 11)),
            chain: vec![(rel_var, person_node("f", "Person", s(21, 32)))],
            span: s(0, 32),
        };
        assert_eq!(path.edge_variables(), vec!["r".to_string()]);
    }

    #[test]
    fn relationship_pattern_outgoing_anonymous() {
        let r = RelationshipPattern::outgoing_anonymous(s(0, 5));
        assert!(r.variable.is_none());
        assert!(r.rel_type.is_none());
        assert_eq!(r.direction, RelDirection::Outgoing);
        assert!(r.declared_variable().is_none());
    }

    #[test]
    fn node_pattern_anonymous() {
        let n = NodePattern::anonymous(s(0, 2));
        assert!(n.variable.is_none());
        assert!(n.label.is_none());
        assert!(n.properties.is_empty());
        assert!(n.declared_variable().is_none());
    }

    // ─── MatchClause: alcance de variables ───

    #[test]
    fn match_clause_bound_variables_sin_duplicados() {
        let m = MatchClause {
            patterns: vec![
                PathPattern {
                    start: person_node("p", "Person", s(0, 11)),
                    chain: vec![(knows_rel(s(0, 5)), person_node("f", "Person", s(0, 11)))],
                    span: s(0, 30),
                },
                PathPattern {
                    start: person_node("p", "Person", s(0, 11)), // duplicada, pero bound_* deduplica
                    chain: Vec::new(),
                    span: s(0, 11),
                },
            ],
            span: s(0, 40),
        };
        let nodes = m.bound_node_variables();
        // p aparece en dos patrones pero bound_* lo lista una vez.
        assert_eq!(nodes, vec!["p", "f"]);
        assert!(m.bound_edge_variables().is_empty());
    }

    // ─── Validación semántica: casos válidos ───

    #[test]
    fn validate_consulta_minima_es_valida() {
        let q = minimal_query();
        let errs = q.validate();
        assert!(errs.is_empty(), "errores inesperados: {errs:?}");
        assert!(q.is_valid());
    }

    #[test]
    fn validate_consulta_completa_es_valida() {
        // MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name AS amigo
        let path = PathPattern {
            start: person_node("p", "Person", s(7, 18)),
            chain: vec![(knows_rel(s(18, 28)), person_node("f", "Person", s(28, 39)))],
            span: s(6, 40),
        };
        let where_c = WhereClause {
            expr: Expression::Compare {
                op: CompareOp::Eq,
                left: Box::new(Expression::prop("p", "name", s(47, 53))),
                right: Box::new(Expression::lit(Value::String("Ana".into()), s(56, 61))),
                span: s(47, 61),
            },
            span: s(41, 61),
        };
        let ret = ReturnClause {
            items: vec![ReturnItem {
                expr: Expression::prop("f", "name", s(69, 75)),
                alias: Some("amigo".into()),
                span: s(69, 84),
            }],
            span: s(62, 84),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 40),
            },
            where_clause: Some(where_c),
            return_clause: ret,
            span: s(0, 84),
        };
        assert!(q.is_valid(), "errores: {:?}", q.validate());
        assert_eq!(q.bound_node_variables(), vec!["p", "f"]);
    }

    // ─── Validación semántica: casos de error ───

    #[test]
    fn validate_match_vacio_devuelve_empty_match() {
        let q = Query {
            match_clause: MatchClause {
                patterns: Vec::new(),
                span: s(0, 5),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "x", s(0, 3)),
                    alias: None,
                    span: s(0, 3),
                }],
                span: s(0, 3),
            },
            span: s(0, 8),
        };
        let errs = q.validate();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].kind, QueryErrorKind::EmptyMatch);
        // Al estar vacío el MATCH, no intenta validar variables de RETURN.
    }

    #[test]
    fn validate_node_pattern_vacio_devuelve_empty_node_pattern() {
        // MATCH () RETURN p  — el primer nodo es () puro.
        let path = PathPattern {
            start: NodePattern::anonymous(s(6, 8)),
            chain: Vec::new(),
            span: s(6, 8),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 8),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "x", s(0, 3)),
                    alias: None,
                    span: s(0, 3),
                }],
                span: s(0, 3),
            },
            span: s(0, 11),
        };
        let errs = q.validate();
        // empty_node_pattern + unknown_variable(p) en RETURN.
        assert!(
            errs.iter()
                .any(|e| e.kind == QueryErrorKind::EmptyNodePattern)
        );
        assert!(errs.iter().any(|e| matches!(
            e.kind,
            QueryErrorKind::UnknownVariable { ref variable } if variable == "p"
        )));
    }

    #[test]
    fn validate_variable_duplicada_en_nodos() {
        // MATCH (p:Person), (p:Person) RETURN p  — 'p' dos veces.
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![
                    PathPattern {
                        start: person_node("p", "Person", s(6, 18)),
                        chain: Vec::new(),
                        span: s(6, 18),
                    },
                    PathPattern {
                        start: person_node("p", "Person", s(20, 32)),
                        chain: Vec::new(),
                        span: s(20, 32),
                    },
                ],
                span: s(0, 32),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "x", s(40, 42)),
                    alias: None,
                    span: s(40, 42),
                }],
                span: s(33, 42),
            },
            span: s(0, 42),
        };
        let errs = q.validate();
        assert!(
            errs.iter()
                .any(|e| matches!(e.kind, QueryErrorKind::DuplicateVariable { ref variable } if variable == "p"))
        );
    }

    #[test]
    fn validate_variable_duplicada_entre_nodo_y_arista() {
        // MATCH (p:Person)-[p:KNOWS]->(f:Person) RETURN f  — 'p' nodo y arista.
        let path = PathPattern {
            start: person_node("p", "Person", s(6, 18)),
            chain: vec![(
                RelationshipPattern {
                    variable: Some("p".into()),
                    rel_type: Some("KNOWS".into()),
                    direction: RelDirection::Outgoing,
                    span: s(18, 29),
                },
                person_node("f", "Person", s(29, 40)),
            )],
            span: s(6, 40),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 40),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("f", "x", s(48, 50)),
                    alias: None,
                    span: s(48, 50),
                }],
                span: s(41, 50),
            },
            span: s(0, 50),
        };
        let errs = q.validate();
        assert!(
            errs.iter()
                .any(|e| matches!(e.kind, QueryErrorKind::DuplicateVariable { ref variable } if variable == "p"))
        );
    }

    #[test]
    fn validate_variable_desconocida_en_where() {
        // MATCH (p:Person) WHERE z.name = "Ana" RETURN p  — z no declarada.
        let path = PathPattern {
            start: person_node("p", "Person", s(6, 18)),
            chain: Vec::new(),
            span: s(6, 18),
        };
        let where_c = WhereClause {
            expr: Expression::Compare {
                op: CompareOp::Eq,
                left: Box::new(Expression::prop("z", "name", s(26, 32))),
                right: Box::new(Expression::lit(Value::String("Ana".into()), s(0, 5))),
                span: s(26, 32),
            },
            span: s(19, 40),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 18),
            },
            where_clause: Some(where_c),
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "x", s(48, 50)),
                    alias: None,
                    span: s(48, 50),
                }],
                span: s(41, 50),
            },
            span: s(0, 50),
        };
        let errs = q.validate();
        assert!(errs.iter().any(|e| matches!(
            e.kind,
            QueryErrorKind::UnknownVariable { ref variable } if variable == "z"
        )));
    }

    #[test]
    fn validate_variable_desconocida_en_return() {
        // MATCH (p:Person) RETURN z.name  — z no declarada.
        let path = PathPattern {
            start: person_node("p", "Person", s(6, 18)),
            chain: Vec::new(),
            span: s(6, 18),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 18),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("z", "name", s(26, 32)),
                    alias: None,
                    span: s(26, 32),
                }],
                span: s(19, 32),
            },
            span: s(0, 32),
        };
        let errs = q.validate();
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0].kind,
            QueryErrorKind::UnknownVariable { ref variable } if variable == "z"
        ));
    }

    #[test]
    fn validate_return_vacio() {
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![PathPattern {
                    start: person_node("p", "Person", s(6, 18)),
                    chain: Vec::new(),
                    span: s(6, 18),
                }],
                span: s(0, 18),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: Vec::new(),
                span: s(19, 25),
            },
            span: s(0, 25),
        };
        let errs = q.validate();
        assert!(errs.iter().any(|e| e.kind == QueryErrorKind::EmptyReturn));
    }

    #[test]
    fn validate_alias_vacio_en_return() {
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![PathPattern {
                    start: person_node("p", "Person", s(6, 18)),
                    chain: Vec::new(),
                    span: s(6, 18),
                }],
                span: s(0, 18),
            },
            where_clause: None,
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("p", "name", s(26, 32)),
                    alias: Some("   ".into()), // solo espacios en blanco
                    span: s(26, 40),
                }],
                span: s(19, 40),
            },
            span: s(0, 40),
        };
        let errs = q.validate();
        assert!(errs.iter().any(|e| e.kind == QueryErrorKind::EmptyAlias));
    }

    #[test]
    fn validate_acepta_variable_de_arista_en_where() {
        // MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE r.weight > 0.5 RETURN f
        let path = PathPattern {
            start: person_node("p", "Person", s(6, 18)),
            chain: vec![(
                RelationshipPattern {
                    variable: Some("r".into()),
                    rel_type: Some("KNOWS".into()),
                    direction: RelDirection::Outgoing,
                    span: s(18, 29),
                },
                person_node("f", "Person", s(29, 41)),
            )],
            span: s(6, 41),
        };
        let where_c = WhereClause {
            expr: Expression::Compare {
                op: CompareOp::Gt,
                left: Box::new(Expression::prop("r", "weight", s(49, 57))),
                right: Box::new(Expression::lit(Value::Float(0.5), s(60, 63))),
                span: s(49, 63),
            },
            span: s(42, 63),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 41),
            },
            where_clause: Some(where_c),
            return_clause: ReturnClause {
                items: vec![ReturnItem {
                    expr: Expression::prop("f", "x", s(71, 73)),
                    alias: None,
                    span: s(71, 73),
                }],
                span: s(64, 73),
            },
            span: s(0, 73),
        };
        assert!(q.is_valid(), "errores: {:?}", q.validate());
        assert!(q.bound_edge_variables().iter().any(|v| v == "r"));
    }

    // ─── QueryError: Display ───

    #[test]
    fn query_error_display_incluye_span() {
        let e = QueryError::new(
            QueryErrorKind::UnknownVariable {
                variable: "z".into(),
            },
            s(26, 32),
        );
        let msg = format!("{e}");
        assert!(msg.contains("'z'"));
        assert!(msg.contains("26..32"));
    }

    #[test]
    fn query_error_display_span_vacio_muestra_offset() {
        let e = QueryError::new(QueryErrorKind::EmptyMatch, Span::at(7));
        let msg = format!("{e}");
        assert!(msg.contains("MATCH vacío"));
        assert!(msg.contains("offset 7"));
        assert!(!msg.contains(".."));
    }

    #[test]
    fn query_error_display_todas_variantes() {
        let cases = [
            (QueryErrorKind::EmptyMatch, "MATCH vacío"),
            (QueryErrorKind::EmptyNodePattern, "'()'"),
            (
                QueryErrorKind::DuplicateVariable {
                    variable: "p".into(),
                },
                "'p'",
            ),
            (
                QueryErrorKind::UnknownVariable {
                    variable: "q".into(),
                },
                "'q'",
            ),
            (QueryErrorKind::EmptyReturn, "RETURN vacío"),
            (QueryErrorKind::EmptyAlias, "alias vacío"),
        ];
        for (kind, needle) in cases {
            let e = QueryError::new(kind, Span::at(0));
            assert!(format!("{e}").contains(needle), "falta '{needle}' en {e}");
        }
    }

    #[test]
    fn query_error_implementa_std_error() {
        let e = QueryError::new(QueryErrorKind::EmptyReturn, Span::at(0));
        // Si compila, implementa std::error::Error.
        let _: &dyn std::error::Error = &e;
    }

    // ─── Display del AST (pretty-printer canónico) ───

    #[test]
    fn display_expression_literal_y_prop() {
        let lit = Expression::lit(Value::Int(42), s(0, 2));
        assert_eq!(format!("{lit}"), "42");

        let lit_s = Expression::lit(Value::String("Ana".into()), s(0, 5));
        assert_eq!(format!("{lit_s}"), "\"Ana\"");

        let lit_b = Expression::lit(Value::Bool(true), s(0, 4));
        assert_eq!(format!("{lit_b}"), "TRUE");

        let lit_n = Expression::lit(Value::Null, s(0, 4));
        assert_eq!(format!("{lit_n}"), "NULL");

        let lit_f = Expression::lit(Value::Float(2.5), s(0, 3));
        assert_eq!(format!("{lit_f}"), "2.5");

        let prop = Expression::prop("p", "name", s(0, 6));
        assert_eq!(format!("{prop}"), "p.name");
    }

    #[test]
    fn display_expression_compare_and_or_not() {
        let cmp = Expression::Compare {
            op: CompareOp::Eq,
            left: Box::new(Expression::prop("p", "name", s(0, 6))),
            right: Box::new(Expression::lit(Value::String("Ana".into()), s(0, 5))),
            span: s(0, 14),
        };
        assert_eq!(format!("{cmp}"), "(p.name = \"Ana\")");

        let and = Expression::And {
            left: Box::new(cmp.clone()),
            right: Box::new(Expression::prop("p", "age", s(0, 5))),
            span: s(0, 20),
        };
        assert_eq!(format!("{and}"), "((p.name = \"Ana\") AND p.age)");

        let or = Expression::Or {
            left: Box::new(cmp.clone()),
            right: Box::new(cmp.clone()),
            span: s(0, 20),
        };
        assert_eq!(
            format!("{or}"),
            "((p.name = \"Ana\") OR (p.name = \"Ana\"))"
        );

        let not = Expression::Not {
            expr: Box::new(cmp),
            span: s(0, 10),
        };
        assert_eq!(format!("{not}"), "(NOT (p.name = \"Ana\"))");
    }

    #[test]
    fn display_node_pattern_partes_opcionales() {
        let anon = NodePattern::anonymous(s(0, 2));
        assert_eq!(format!("{anon}"), "()");

        let var_only = NodePattern {
            variable: Some("p".into()),
            label: None,
            properties: Vec::new(),
            span: s(0, 3),
        };
        assert_eq!(format!("{var_only}"), "(p)");

        let label_only = NodePattern {
            variable: None,
            label: Some("Person".into()),
            properties: Vec::new(),
            span: s(0, 9),
        };
        assert_eq!(format!("{label_only}"), "(:Person)");

        let full = NodePattern {
            variable: Some("p".into()),
            label: Some("Person".into()),
            properties: vec![(
                "name".to_string(),
                Expression::lit(Value::String("Ana".into()), s(0, 5)),
            )],
            span: s(0, 20),
        };
        assert_eq!(format!("{full}"), "(p:Person {name: \"Ana\"})");
    }

    #[test]
    fn display_relationship_pattern_direcciones() {
        let out = RelationshipPattern {
            variable: Some("r".into()),
            rel_type: Some("KNOWS".into()),
            direction: RelDirection::Outgoing,
            span: s(0, 10),
        };
        assert_eq!(format!("{out}"), "-[r:KNOWS]->");

        let inc = RelationshipPattern {
            variable: None,
            rel_type: Some("KNOWS".into()),
            direction: RelDirection::Incoming,
            span: s(0, 10),
        };
        assert_eq!(format!("{inc}"), "<-[:KNOWS]-");

        let und = RelationshipPattern {
            variable: None,
            rel_type: None,
            direction: RelDirection::Undirected,
            span: s(0, 4),
        };
        assert_eq!(format!("{und}"), "-[]-");
    }

    #[test]
    fn display_path_pattern_encadena() {
        let path = PathPattern {
            start: person_node("p", "Person", s(0, 11)),
            chain: vec![(knows_rel(s(0, 10)), person_node("f", "Person", s(0, 11)))],
            span: s(0, 30),
        };
        assert_eq!(format!("{path}"), "(p:Person)-[:KNOWS]->(f:Person)");
    }

    #[test]
    fn display_query_completa_round_trip_canonico() {
        let q = minimal_query();
        let text = format!("{q}");
        assert_eq!(text, "MATCH (p:Person) RETURN p.name");
    }

    #[test]
    fn display_query_con_where_y_alias() {
        let path = PathPattern {
            start: person_node("p", "Person", s(0, 11)),
            chain: vec![(knows_rel(s(0, 10)), person_node("f", "Person", s(0, 11)))],
            span: s(0, 30),
        };
        let where_c = WhereClause {
            expr: Expression::Compare {
                op: CompareOp::Eq,
                left: Box::new(Expression::prop("p", "name", s(0, 6))),
                right: Box::new(Expression::lit(Value::String("Ana".into()), s(0, 5))),
                span: s(0, 14),
            },
            span: s(0, 14),
        };
        let ret = ReturnClause {
            items: vec![ReturnItem {
                expr: Expression::prop("f", "name", s(0, 6)),
                alias: Some("amigo".into()),
                span: s(0, 14),
            }],
            span: s(0, 14),
        };
        let q = Query {
            match_clause: MatchClause {
                patterns: vec![path],
                span: s(0, 30),
            },
            where_clause: Some(where_c),
            return_clause: ret,
            span: s(0, 30),
        };
        assert_eq!(
            format!("{q}"),
            "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE (p.name = \"Ana\") RETURN f.name AS amigo"
        );
    }

    // ─── AstNode (enum del hito del brief) ───

    #[test]
    fn ast_node_variants_build_and_match() {
        let m = AstNode::Match(MatchClause {
            patterns: vec![PathPattern {
                start: person_node("p", "Person", s(0, 11)),
                chain: Vec::new(),
                span: s(0, 11),
            }],
            span: s(0, 11),
        });
        let w = AstNode::Where(WhereClause {
            expr: Expression::lit(Value::Bool(true), s(0, 4)),
            span: s(0, 4),
        });
        let r = AstNode::Return(ReturnClause {
            items: vec![ReturnItem {
                expr: Expression::prop("p", "x", s(0, 3)),
                alias: None,
                span: s(0, 3),
            }],
            span: s(0, 3),
        });
        // Las tres variantes del hito del brief existen y se construyen.
        assert!(matches!(m, AstNode::Match(_)));
        assert!(matches!(w, AstNode::Where(_)));
        assert!(matches!(r, AstNode::Return(_)));
    }

    // ─── hex_bytes (helper de Display de Value::Bytes) ───

    #[test]
    fn hex_bytes_formatea_correctamente() {
        assert_eq!(hex_bytes(&[]), "");
        assert_eq!(hex_bytes(&[0x00]), "00");
        assert_eq!(hex_bytes(&[0xff]), "ff");
        assert_eq!(hex_bytes(&[0xDE, 0xAD]), "dead");
        assert_eq!(hex_bytes(&[0x48, 0x49]), "4849");
    }

    #[test]
    fn display_value_bytes_canonico() {
        let e = Expression::lit(Value::Bytes(vec![0xCA, 0xFE]), s(0, 6));
        assert_eq!(format!("{e}"), "0xcafe");
    }
}
