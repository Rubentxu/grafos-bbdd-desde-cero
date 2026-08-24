//! La SESIÓN de la CLI (cap. 31): el grafo vivo + el intérprete de líneas
//! COMPARTIDO por el REPL y el script mode.
//!
//! Una «sesión» es un [`MemoryStore`] que NACE de un origen (`demo` =
//! [`vol2_liradb::demo_graph`], `empty` = vacío) y que los meta-comandos
//! pueden RECARGAR en caliente. Sobre ella, [`interpretar_linea`] ejecuta
//! una línea: una consulta LiraQL (todo lo que no empiece por `:`) o un
//! meta-comando `:...`.
//!
//! La decisión de arquitectura del capítulo: REPL y script NO son dos
//! motores — son DOS FRONTALES del MISMO intérprete. El REPL imprime
//! prompts (`liradb> ` / `tx> `) y continúa tras los errores; el script no
//! imprime prompts y DETIENE en el primero (con su nº de línea, via
//! `Err(String)` sin imprimir — el que llama decide cómo reportarlo).
//!
//! Las ESCRITURAS de la sesión usan la maquinaria del cap. 27:
//! * Fuera de transacción: [`vol2_liradb::autocommit`] — cada `:node` es
//!   su propia transacción (el modo por defecto de los caps. 7-26).
//! * Con `:begin`: una [`vol2_liradb::Transaccion`] REAL con staging y
//!   validación eager — y su préstamo exclusivo se hace VISIBLE en el
//!   producto: mientras la transacción vive, las consultas se rechazan
//!   («el store está prestado»), igual que las anidadas no compilaban.
//!   `:quit` dentro de la transacción es rollback implícito (drop), la
//!   misma garantía del cap. 27.

use std::io::{BufRead, Write};

use vol2_liradb::{
    Edge, EdgeId, MemoryStore, Node, NodeId, Operacion, Transaccion, Value, autocommit, demo_graph,
    run as correr_consulta,
};

/// Prompt del REPL en modo normal.
pub const PROMPT: &str = "liradb> ";
/// Prompt del REPL mientras una transacción está abierta: el préstamo
/// exclusivo del cap. 27, hecho visible.
pub const PROMPT_TX: &str = "tx>      ";

/// La sesión: el grafo vivo que comparten todas las líneas.
pub struct Sesion {
    store: MemoryStore,
}

impl Sesion {
    /// Crea la sesión desde un origen: `"demo"` (el fixture del cap. 20,
    /// único punto de verdad) o `"empty"` (el usuario construye el suyo).
    pub fn nueva(origen: &str) -> Sesion {
        let store = if origen == "empty" {
            MemoryStore::new()
        } else {
            demo_graph()
        };
        Sesion { store }
    }

    /// El store como puerto del motor (para las consultas LiraQL).
    pub fn store(&self) -> &MemoryStore {
        &self.store
    }

    /// Nodos y aristas actuales (para `:graph`).
    pub fn conteo(&self) -> (usize, usize) {
        use vol2_liradb::GraphStore as _;
        (self.store.node_count(), self.store.edge_count())
    }
}

/// Qué debe hacer el driver (REPL o script) tras procesar la línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accion {
    /// Seguir leyendo líneas.
    Seguir,
    /// Terminar la sesión (`:quit`, o EOF del sub-bucle de transacción).
    Salir,
}

/// Interpreta UNA línea de la sesión.
///
/// Devuelve `Err(mensaje)` (SIN imprimir) para los errores que el driver
/// debe reportar a su manera: el REPL los imprime y continúa; el script
/// añade el nº de línea y DETIENE. Los errores DENTRO del sub-bucle de
/// transacción (staging inválido) se imprimen aquí mismo y el bucle
/// continúa — igual que `stage` expulsaba la operación inválida y la tx
/// seguía viva en el cap. 27.
///
/// `entrada` permite a `:begin` consumir MÁS líneas (el sub-bucle de la
/// transacción) del mismo frente de lectura del driver. `interactivo`
/// activa los prompts `tx>` de ese sub-bucle.
pub fn interpretar_linea(
    sesion: &mut Sesion,
    linea: &str,
    entrada: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactivo: bool,
) -> Result<Accion, String> {
    let linea = linea.trim();
    // Vacías y comentarios (# o --): no son errores ni hacen nada.
    if linea.is_empty() || linea.starts_with('#') || linea.starts_with("--") {
        return Ok(Accion::Seguir);
    }

    if let Some(resto) = linea.strip_prefix(':') {
        interpretar_meta(sesion, resto, entrada, out, err, interactivo)
    } else {
        // Consulta LiraQL: se tolera el `;` final de la costumbre SQL.
        let src = linea.trim_end_matches(';').trim();
        match correr_consulta(src, sesion.store()) {
            Ok(rs) => {
                emitir(out, &rs.to_string());
                Ok(Accion::Seguir)
            }
            Err(e) => Err(format!("error: {e}")),
        }
    }
}

// ─────────────────── Meta-comandos ───────────────────

/// Ayuda de los meta-comandos (lo que imprime `:help`).
pub fn ayuda_meta(out: &mut dyn Write) {
    emitir(
        out,
        "Meta-comandos de la sesión (cap. 31):
  :help                    esta ayuda
  :quit  (:q)              terminar la sesión (EOF = lo mismo)
  :demo                    recargar el grafo demo (Ana/Bo/Carla/Dani…)
  :clear                   vaciar la sesión y empezar de cero
  :graph                   cuántos nodos y aristas hay
  :node <id>:<Etiqueta> [clave=valor…]   crear un nodo (autocommit)
  :edge <id> <de> <a> <TIPO> [clave=…]   crear una arista (autocommit)
  :del-node <id> · :del-edge <id>        borrar (autocommit)
  :begin                   abrir una TRANSACCIÓN (cap. 27): lo que sigue
                           pasa por staging; el prompt cambia a tx>
  :commit · :rollback      cerrar la transacción abierta
Valores: \"texto\" · 42 · 3.14 · true · false · null. Fuera de tx cada
escritura es AUTOCOMMIT; dentro, staging validado eager como el cap. 27.
Consultas: MATCH … RETURN … (sin `:`). El `;` final se tolera.
",
    );
}

/// El corazón de `:begin`: el sub-bucle de la transacción.
///
/// La tx entra POR VALOR: su ciclo de vida vive en los tipos (cap. 27 —
/// `commit`/`rollback` consumen `self`), así que este bucle termina
/// precisamente cuando la tx se consume. Mientras vive, el store está
/// PRESTADO (cap. 27): las consultas y los meta-comandos que tocan el
/// store se rechazan con un mensaje que lo explica — el mismo «un único
/// escritor» que el borrow checker exigía al código, ahora visible para
/// el usuario. Sólo se admiten: `:node/:edge/:del-*` (staging),
/// `:commit`, `:rollback`, `:quit` (rollback implícito, como el drop del
/// cap. 27) y `:help`.
fn bucle_transaccion(
    mut tx: Transaccion<'_>,
    entrada: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactivo: bool,
) -> Result<Accion, String> {
    loop {
        if interactivo {
            emitir(out, PROMPT_TX);
        }
        let mut linea = String::new();
        match entrada.read_line(&mut linea) {
            Ok(0) => {
                // EOF con la tx abierta: drop implícito = rollback (cap. 27).
                let resumen = tx.rollback();
                emitir(
                    out,
                    &format!(
                        "EOF con transacción abierta → rollback implícito \
                         ({} operaciones descartadas)\n",
                        resumen.operaciones_descartadas
                    ),
                );
                return Ok(Accion::Salir);
            }
            Ok(_) => {}
            Err(e) => return Err(format!("error de lectura: {e}")),
        }
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') || linea.starts_with("--") {
            continue;
        }
        let Some(resto) = linea.strip_prefix(':') else {
            emitir(
                err,
                "transacción abierta: el store está prestado (cap. 27); las consultas \
                 esperan — :commit o :rollback primero\n",
            );
            continue;
        };
        let mut partes = resto.split_whitespace();
        match partes.next().unwrap_or("") {
            "help" | "h" => ayuda_meta(out),
            "commit" => {
                // `commit` CONSUME la tx acierte o falle (cap. 27): con
                // Err la transacción queda descartada y nada se aplicó.
                match tx.commit() {
                    Ok(resumen) => emitir(out, &format!("{resumen}\n")),
                    Err(e) => {
                        emitir(
                            err,
                            &format!("commit rechazado (transacción descartada): {e}\n"),
                        );
                    }
                }
                return Ok(Accion::Seguir);
            }
            "rollback" => {
                let resumen = tx.rollback();
                emitir(out, &format!("{resumen}\n"));
                return Ok(Accion::Seguir);
            }
            "quit" | "q" => {
                let resumen = tx.rollback();
                emitir(
                    out,
                    &format!(
                        ":quit con transacción abierta → rollback implícito \
                         ({} operaciones descartadas)\n",
                        resumen.operaciones_descartadas
                    ),
                );
                return Ok(Accion::Salir);
            }
            "node" => match parse_nodo(partes.collect()) {
                Ok(n) => match tx.put_node(n) {
                    Ok(()) => emitir(out, "staged\n"),
                    Err(e) => emitir(err, &format!("staging rechazado: {e}\n")),
                },
                Err(e) => emitir(err, &format!("{e}\n")),
            },
            "edge" => match parse_arista(partes.collect()) {
                Ok(a) => match tx.put_edge(a) {
                    Ok(()) => emitir(out, "staged\n"),
                    Err(e) => emitir(err, &format!("staging rechazado: {e}\n")),
                },
                Err(e) => emitir(err, &format!("{e}\n")),
            },
            "del-node" => match parse_id_simple(partes.next(), "del-node") {
                Ok(id) => match tx.delete_node(id) {
                    Ok(()) => emitir(out, "staged\n"),
                    Err(e) => emitir(err, &format!("staging rechazado: {e}\n")),
                },
                Err(e) => emitir(err, &format!("{e}\n")),
            },
            "del-edge" => match parse_id_simple(partes.next(), "del-edge") {
                Ok(id) => match tx.delete_edge(id) {
                    Ok(()) => emitir(out, "staged\n"),
                    Err(e) => emitir(err, &format!("staging rechazado: {e}\n")),
                },
                Err(e) => emitir(err, &format!("{e}\n")),
            },
            otro => emitir(
                err,
                &format!(
                    "transacción abierta: ':{otro}' no aplica aquí — sólo staging \
                     (:node/:edge/:del-*) y :commit/:rollback/:quit\n"
                ),
            ),
        }
    }
}

/// Despacha un meta-comando (lo que va tras `:`).
fn interpretar_meta(
    sesion: &mut Sesion,
    resto: &str,
    entrada: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    interactivo: bool,
) -> Result<Accion, String> {
    let mut partes = resto.split_whitespace();
    let cmd = partes.next().unwrap_or("help");
    match cmd {
        "help" | "h" => {
            ayuda_meta(out);
            Ok(Accion::Seguir)
        }
        "quit" | "q" => Ok(Accion::Salir),
        "demo" => {
            sesion.recargar_demo();
            let (n, m) = sesion.conteo();
            emitir(
                out,
                &format!("grafo demo cargado: {n} nodos, {m} aristas\n"),
            );
            Ok(Accion::Seguir)
        }
        "clear" => {
            *sesion = Sesion::nueva("empty");
            emitir(out, "sesión vaciada (:node/:edge para construir)\n");
            Ok(Accion::Seguir)
        }
        "graph" => {
            let (n, m) = sesion.conteo();
            emitir(out, &format!("grafo: {n} nodos, {m} aristas\n"));
            Ok(Accion::Seguir)
        }
        "begin" => {
            // El préstamo exclusivo dura lo que el sub-bucle: al salir,
            // la tx está consumida (commit/rollback) y el store vuelve.
            let tx = Transaccion::begin(sesion.store_mut());
            bucle_transaccion(tx, entrada, out, err, interactivo)
        }
        "commit" | "rollback" => Err(
            "no hay transacción abierta (:begin la abre; fuera de tx cada \
             escritura es autocommit)"
                .to_string(),
        ),
        "node" => match parse_nodo(partes.collect()) {
            Ok(n) => {
                let nombre = format!("nodo {}", n.id);
                escritura_autocommit(sesion, Operacion::PutNode(n), &nombre, out)
            }
            Err(e) => Err(e),
        },
        "edge" => match parse_arista(partes.collect()) {
            Ok(a) => {
                let nombre = format!("arista {} ({} -> {})", a.id, a.source, a.target);
                escritura_autocommit(sesion, Operacion::PutEdge(a), &nombre, out)
            }
            Err(e) => Err(e),
        },
        "del-node" => match parse_id_simple(partes.next(), "del-node") {
            Ok(id) => escritura_autocommit(
                sesion,
                Operacion::DeleteNode(id),
                &format!("delete nodo {id}"),
                out,
            ),
            Err(e) => Err(e),
        },
        "del-edge" => match parse_id_simple(partes.next(), "del-edge") {
            Ok(id) => escritura_autocommit(
                sesion,
                Operacion::DeleteEdge(id),
                &format!("delete arista {id}"),
                out,
            ),
            Err(e) => Err(e),
        },
        otro => Err(format!("meta-comando desconocido ':{otro}' (ver :help)")),
    }
}

impl Sesion {
    /// Recarga el grafo demo SIN perder la identidad de la sesión.
    fn recargar_demo(&mut self) {
        self.store = demo_graph();
    }

    /// `&mut` del store para las escrituras autocommit y las transacciones.
    fn store_mut(&mut self) -> &mut MemoryStore {
        &mut self.store
    }
}

/// Aplica una operación en AUTOCOMMIT (cap. 27): la forma visible de
/// «cada put_* su propia transacción», el modo por defecto del motor.
fn escritura_autocommit(
    sesion: &mut Sesion,
    op: Operacion,
    nombre: &str,
    out: &mut dyn Write,
) -> Result<Accion, String> {
    match autocommit(sesion.store_mut(), op) {
        Ok(resumen) => {
            emitir(
                out,
                &format!(
                    "ok: {nombre} (autocommit, {} operaciones)\n",
                    resumen.total_operaciones()
                ),
            );
            Ok(Accion::Seguir)
        }
        Err(e) => Err(format!("{e}")),
    }
}

// ─────────────────── Parsers de meta-comandos ───────────────────

/// `:node <id>:<Etiqueta[:Otra…]> [clave=valor…]`
///
/// El formato `id:Etiqueta` es un guiño a Cypher (`CREATE (n:Person)`).
fn parse_nodo(args: Vec<&str>) -> Result<Node, String> {
    let Some(primero) = args.first() else {
        return Err("uso: :node <id>:<Etiqueta> [clave=valor…]".into());
    };
    let Some((id_s, labels_s)) = primero.split_once(':') else {
        return Err(format!(
            "se espera <id>:<Etiqueta> (ej. 7:Person) — llegó '{primero}'"
        ));
    };
    let id: NodeId = id_s
        .parse()
        .map_err(|_| format!("id no numérico: '{id_s}'"))?;
    let labels: Vec<String> = labels_s.split(':').map(str::to_string).collect();
    let mut nodo = Node::new(id, &labels[0]);
    nodo.labels = labels;
    for kv in &args[1..] {
        let (clave, valor) = parse_prop(kv)?;
        nodo.props.insert(clave, valor);
    }
    Ok(nodo)
}

/// `:edge <id> <de> <a> <TIPO> [clave=valor…]`
fn parse_arista(args: Vec<&str>) -> Result<Edge, String> {
    if args.len() < 4 {
        return Err("uso: :edge <id> <de> <a> <TIPO> [clave=valor…]".into());
    }
    let id: EdgeId = args[0]
        .parse()
        .map_err(|_| format!("id no numérico: '{}'", args[0]))?;
    let source: NodeId = args[1]
        .parse()
        .map_err(|_| format!("nodo origen no numérico: '{}'", args[1]))?;
    let target: NodeId = args[2]
        .parse()
        .map_err(|_| format!("nodo destino no numérico: '{}'", args[2]))?;
    let mut arista = Edge::new(id, source, target, args[3]);
    for kv in &args[4..] {
        let (clave, valor) = parse_prop(kv)?;
        arista.props.insert(clave, valor);
    }
    Ok(arista)
}

/// Un id obligatorio y numérico (`:del-node <id>`).
fn parse_id_simple(crudo: Option<&str>, cmd: &str) -> Result<usize, String> {
    let Some(s) = crudo else {
        return Err(format!("uso: :{cmd} <id>"));
    };
    s.parse()
        .map_err(|_| format!("id no numérico: '{s}' (uso: :{cmd} <id>)"))
}

/// `clave=valor` con valores del cap. 7: `"texto"`, entero, float,
/// `true`/`false`, `null`.
fn parse_prop(kv: &str) -> Result<(String, Value), String> {
    let Some((clave, valor_s)) = kv.split_once('=') else {
        return Err(format!(
            "propiedad sin '=': '{kv}' (formato clave=valor, cadenas entre comillas)"
        ));
    };
    Ok((clave.to_string(), parse_valor(valor_s)?))
}

/// Un valor textual al [`Value`] del cap. 7 (el tipado del modelo, otra vez).
fn parse_valor(s: &str) -> Result<Value, String> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Ok(Value::String(s[1..s.len() - 1].to_string()));
    }
    match s {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        "null" => return Ok(Value::Null),
        _ => {}
    }
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Value::Float(f));
    }
    Err(format!(
        "valor no reconocido: '{s}' (cadenas entre comillas dobles, \
         números sin unidades, true/false/null)"
    ))
}

/// Escribe `s` en `out` ignorando fallos de E/S (misma tolerancia que el
/// hito: `liradb demo | head` no entra en pánico).
fn emitir(out: &mut dyn Write, s: &str) {
    let _ = out.write_all(s.as_bytes());
}
