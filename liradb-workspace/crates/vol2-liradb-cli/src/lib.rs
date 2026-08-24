//! Vol.II — Cap. 31: **La CLI de LiraDB** (abre la Parte VII: convertir el
//! proyecto en un producto técnico). El hito tras el cap. 20 (demo/query/
//! help con parseo MANUAL) crece hasta la CLI completa:
//!
//! * **clap** (Builder API) — el pago de la regla «primero a mano, luego
//!   con crates»: subcomandos `demo | query | explain | repl | script`,
//!   flags `--graph demo|empty`, `--plan`, `--stats`, `--version`, y los
//!   mensajes de uso/validación generados. La AYUDA propia (`help`,
//!   `--help`, `-h`) sigue siendo la curada del libro (con EJEMPLOS y el
//!   GRAFO DEMO): se desactivan los help autogenerados y se cablean a
//!   [`imprimir_ayuda`]; los ERRORES de uso sí son de clap — se
//!   re-escriben al writer inyectado con [`clap::error::Error::render`],
//!   nunca a stderr real, para seguir testeando sin procesos.
//! * **REPL** (`liradb repl`) — el modo interactivo: lee líneas de un
//!   `dyn Read` (stdin en producción, un buffer en los tests), mantiene
//!   una SESIÓN (el grafo vive entre consultas) y entiende
//!   meta-comandos `:help :quit :demo :clear :graph :node :edge
//!   :del-node :del-edge :begin :commit :rollback` (ver
//!   [`sesion::ayuda_meta`]). Las escrituras usan el AUTOCOMMIT del
//!   cap. 27 fuera de transacción y una [`vol2_liradb::Transaccion`]
//!   REAL con `:begin` — con su préstamo exclusivo hecho visible:
//!   mientras vive, las consultas se rechazan y el prompt pasa a `tx>`.
//! * **Script mode** (`liradb script <fichero|->`) — el otro frente del
//!   MISMO intérprete ([`sesion::interpretar_linea`]): sin prompts,
//!   comentarios `#`/`--`, y DETIENE en el primer error con su nº de
//!   línea (exit 1). `-` lee stdin. Los scripts pueden CONSTRUIR el
//!   grafo con los mismos meta-comandos y consultarlo después.
//!
//! Testabilidad (la lección del hito, conservada): toda la lógica vive en
//! esta lib; [`run_con_entrada`] recibe los argumentos, un `dyn Read`
//! (stdin para REPL/script `-`) y dos writers, y devuelve el exit code.
//! Los tests ejercitan REPL y scripts con buffers, sin spawn ni TTY.
//!
//! Códigos de salida (los del hito + los de clap): 0 OK · 1 error de
//! consulta/ejecución (y de fichero en script) · 2 error de uso (clap).
//!
//! NOTA de alcance: `import`/`export` (CSV, JSONL, GraphML) NO van aquí
//! — son el cap. 32; esta CLI deja el intérprete listo para enchufarlos.

pub mod sesion;

use std::io::{BufRead, BufReader, Read, Write};

use clap::{Arg, ArgAction, Command};
use sesion::{Accion, PROMPT, Sesion, interpretar_linea};
use vol2_liradb::{ExecError, Executor, GraphStore, demo_graph, explain, parse};

/// Salida correcta.
pub const EXIT_OK: i32 = 0;
/// La consulta (o el fichero del script) falló: el mensaje va a stderr.
pub const EXIT_ERROR_CONSULTA: i32 = 1;
/// Uso incorrecto (argumentos de clap): mensaje autogenerado a stderr.
pub const EXIT_ERROR_USO: i32 = 2;

/// Las 4 consultas de `liradb demo`: una por capa del pipeline.
const CONSULTAS_DEMO: &[(&str, &str)] = &[
    (
        "MATCH simple (NodeScan + Project)",
        "MATCH (p:Person) RETURN p.name, p.age",
    ),
    (
        "WHERE con comparación (Filter trivalente)",
        "MATCH (p:Person) WHERE p.age < 40 RETURN p.name, p.age",
    ),
    (
        "Patrón relacional (Expand)",
        "MATCH (p:Person)-[:KNOWS]->(f:Person) RETURN p.name, f.name",
    ),
    (
        "Consulta canónica del brief + propiedad de arista",
        "MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = \"Ana\" \
         RETURN f.name, r.since",
    ),
];

// ─────────────────── El árbol de comandos (clap Builder) ───────────────────

/// Define la CLI con clap **Builder API**: explícita, sin macros.
///
/// El capítulo contrasta esta forma con `#[derive(Parser)]` (la ergonomía
/// declarativa que la mayoría usa en producción): aquí se enseña el árbol
/// de comandos a la vista — cada subcomando y flag es una línea de Rust.
/// Ayuda y versión: `--version` es de clap (salida estándar, exit 0);
/// `help`/`--help`/`-h` se atajan ANTES (ver [`run_con_entrada`]) para
/// conservar la ayuda curada del libro (con EJEMPLOS), así que se
/// desactivan los de clap.
fn comando() -> Command {
    Command::new("liradb")
        .about("La CLI de LiraDB (Vol.II, cap. 31): consultar, explorar y scriptar el motor")
        .version(env!("CARGO_PKG_VERSION"))
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .subcommand(Command::new("demo").about("4 consultas de muestra sobre el grafo demo"))
        .subcommand(
            Command::new("query")
                .about("Ejecuta una consulta LiraQL sobre el grafo demo")
                .arg(
                    Arg::new("consulta")
                        .required(true)
                        .value_name("LIRAQL")
                        .help("La consulta, entre comillas"),
                )
                .arg(
                    Arg::new("plan")
                        .long("plan")
                        .action(ArgAction::SetTrue)
                        .help("Imprime también el plan lógico"),
                )
                .arg(
                    Arg::new("stats")
                        .long("stats")
                        .action(ArgAction::SetTrue)
                        .help("Imprime también las métricas por operador"),
                ),
        )
        .subcommand(
            Command::new("explain")
                .about("Plan ANTES/DESPUÉS del optimizador, con estimaciones (cap. 21)")
                .arg(
                    Arg::new("consulta")
                        .required(true)
                        .value_name("LIRAQL")
                        .help("La consulta, entre comillas"),
                ),
        )
        .subcommand(
            Command::new("repl")
                .about("REPL interactivo con sesión (meta-comandos con :help)")
                .arg(arg_graph()),
        )
        .subcommand(
            Command::new("script")
                .about("Ejecuta un guion de líneas (consultas y meta-comandos)")
                .arg(
                    Arg::new("fichero")
                        .required(true)
                        .value_name("FICHERO")
                        .help("El guion; '-' lee stdin"),
                )
                .arg(arg_graph()),
        )
        .subcommand(Command::new("help").about("Muestra la ayuda (curada, con ejemplos)"))
}

/// El flag compartido `--graph`: de dónde NACE la sesión del REPL/script.
fn arg_graph() -> Arg {
    Arg::new("graph")
        .long("graph")
        .value_name("ORIGEN")
        .default_value("demo")
        .value_parser(["demo", "empty"])
        .help("Grafo inicial de la sesión: demo (por defecto) o empty")
}

// ─────────────────── Entrada testable ───────────────────

/// El punto de entrada del cap. 31: como el `run` del hito, pero con la
/// ENTRADA inyectada — el REPL y `script -` leen de aquí (stdin en
/// producción, un buffer en los tests). Devuelve el código de salida.
pub fn run_con_entrada(
    args: &[String],
    entrada: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // Sin argumentos → ayuda curada y exit 0 (la semántica del hito,
    // conservada a propósito: `liradb` solo debe enseñar el camino).
    if args.is_empty() {
        imprimir_ayuda(out);
        return EXIT_OK;
    }
    // La ayuda curada también vive de `help`/`--help`/`-h` exactos: el
    // árbol de clap no la genera (disable_*), así que se ataja aquí.
    if args.len() == 1 && matches!(args[0].as_str(), "help" | "--help" | "-h") {
        imprimir_ayuda(out);
        return EXIT_OK;
    }

    // clap: el árbol se evalúa sobre argv con nombre de binario delante.
    let argv: Vec<String> = std::iter::once("liradb".to_string())
        .chain(args.iter().cloned())
        .collect();
    match comando().try_get_matches_from(argv) {
        Ok(matches) => despachar(&matches, entrada, out, err),
        Err(e) => {
            // Render al writer INYECTADO (no a stderr real): testeable.
            // use_stderr() distingue la versión (a stdout, exit 0) de los
            // errores de uso (a stderr, exit 2 — que coincide con el
            // EXIT_ERROR_USO del hito).
            let mut texto = e.render().to_string();
            if !texto.ends_with('\n') {
                texto.push('\n');
            }
            if e.use_stderr() {
                emitir(err, &texto);
            } else {
                emitir(out, &texto);
            }
            e.exit_code()
        }
    }
}

/// El `run` del hito, conservado: stdin real para REPL/script `-`.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_con_entrada(args, &mut std::io::stdin(), out, err)
}

/// Reparte cada subcomando a su driver.
fn despachar(
    matches: &clap::ArgMatches,
    entrada: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match matches.subcommand() {
        Some(("demo", _)) => cmd_demo(out, err),
        Some(("query", m)) => {
            let consulta = m.get_one::<String>("consulta").expect("required");
            let plan = m.get_flag("plan");
            let stats = m.get_flag("stats");
            cmd_query(consulta, plan, stats, out, err)
        }
        Some(("explain", m)) => {
            let consulta = m.get_one::<String>("consulta").expect("required");
            cmd_explain(consulta, out, err)
        }
        Some(("repl", m)) => {
            let origen = m.get_one::<String>("graph").expect("con default");
            cmd_repl(entrada, origen, out, err)
        }
        Some(("script", m)) => {
            let fichero = m.get_one::<String>("fichero").expect("required");
            let origen = m.get_one::<String>("graph").expect("con default");
            cmd_script(fichero, origen, entrada, out, err)
        }
        _ => {
            // `help` (y cualquier resto): la ayuda curada del libro.
            imprimir_ayuda(out);
            EXIT_OK
        }
    }
}

// ─────────────────── Subcomando `query` (con flags del cap. 31) ───────────────────

/// Ejecuta una consulta LiraQL sobre el grafo demo e imprime la tabla.
///
/// Sin flags es el comportamiento del hito (sólo la tabla). `--plan`
/// añade el plan lógico y `--stats` las métricas por operador — las dos
/// vistas que `demo` ya enseñaba, ahora a la carta.
fn cmd_query(
    consulta: &str,
    plan: bool,
    stats: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if !plan && !stats {
        let store = demo_graph();
        return match vol2_liradb::run(consulta, &store) {
            Ok(rs) => {
                emitir(out, &rs.to_string());
                EXIT_OK
            }
            Err(e) => {
                emitir(err, &format!("error: {e}\n"));
                EXIT_ERROR_CONSULTA
            }
        };
    }
    // Con flags: el pipeline del demo (parse → lower → ejecutar) a la carta.
    let store = demo_graph();
    match pipeline_con_detalle(consulta, &store, out, plan, stats) {
        Ok(()) => EXIT_OK,
        Err(e) => {
            emitir(err, &format!("error: {e}\n"));
            EXIT_ERROR_CONSULTA
        }
    }
}

// ─────────────────── Subcomando `explain` ───────────────────

/// El hito del cap. 21: plan ANTES/DESPUÉS con cardinalidades estimadas.
fn cmd_explain(consulta: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let store = demo_graph();
    match explain(consulta, &store) {
        Ok(texto) => {
            emitir(out, &texto);
            emitir(out, "\n");
            EXIT_OK
        }
        Err(e) => {
            emitir(err, &format!("error: {e}\n"));
            EXIT_ERROR_CONSULTA
        }
    }
}

// ─────────────────── Subcomando `demo` ───────────────────

/// Ejecuta las 4 consultas de muestra sobre el grafo demo (el hito).
fn cmd_demo(out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let store = demo_graph();
    emitir(
        out,
        "LiraDB — demo del motor (caps. 17-21: parse → lower → optimizador → Volcano)\n",
    );
    emitir(
        out,
        &format!(
            "Grafo demo: {} nodos (Person/City) y {} aristas (KNOWS/LIVES_IN)\n",
            store.node_count(),
            store.edge_count()
        ),
    );

    for (i, (titulo, src)) in CONSULTAS_DEMO.iter().enumerate() {
        emitir(
            out,
            &format!("\n── [{}/{}] {} ──\n", i + 1, CONSULTAS_DEMO.len(), titulo),
        );
        emitir(out, &format!("LiraQL: {src}\n"));
        if let Err(e) = ejecutar_con_detalle(src, &store, out) {
            emitir(err, &format!("error: {e}\n"));
            return EXIT_ERROR_CONSULTA;
        }
    }

    emitir(
        out,
        "\nSigue explorando: liradb repl (sesión interactiva) · liradb query \"...\"\n",
    );
    EXIT_OK
}

/// Pipeline completo de UNA consulta, con plan y métricas visibles
/// (el desenrollado de `vol2_liradb::run` que el demo ya usaba).
fn ejecutar_con_detalle(
    src: &str,
    store: &dyn GraphStore,
    out: &mut dyn Write,
) -> Result<(), ExecError> {
    pipeline_con_detalle(src, store, out, true, true)
}

/// El pipeline parametrizado de `query --plan/--stats` y `demo`.
fn pipeline_con_detalle(
    src: &str,
    store: &dyn GraphStore,
    out: &mut dyn Write,
    plan: bool,
    stats: bool,
) -> Result<(), ExecError> {
    let query = parse(src)?;
    let plan_logico = query.lower()?;
    if plan {
        emitir(out, "Plan lógico:\n");
        emitir(out, &format!("{plan_logico}\n"));
    }
    let mut executor = Executor::new(&plan_logico, store)?;
    let rs = executor.execute()?;
    emitir(out, "Resultado:\n");
    emitir(out, &rs.to_string());
    if stats {
        emitir(out, &format!("Métricas: {}\n", executor.metrics()));
    }
    Ok(())
}

// ─────────────────── Subcomando `repl` ───────────────────

/// El modo interactivo: prompts `liradb> ` (y `tx>` con transacción
/// abierta), sesión persistente entre líneas y errores que NO cortan.
///
/// EOF (= Ctrl+D) equivale a `:quit`: salida limpia con exit 0.
fn cmd_repl(entrada: &mut dyn Read, origen: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut sesion = Sesion::nueva(origen);
    let (n, m) = sesion.conteo();
    emitir(
        out,
        &format!(
            "LiraDB REPL — sesión '{origen}' ({n} nodos, {m} aristas). \
             :help para ayuda, :quit para salir.\n"
        ),
    );

    let mut reader = BufReader::new(entrada);
    loop {
        emitir(out, PROMPT);
        let mut linea = String::new();
        match reader.read_line(&mut linea) {
            Ok(0) => {
                // EOF: tan limpio como :quit.
                emitir(out, "\n");
                return EXIT_OK;
            }
            Ok(_) => {}
            Err(e) => {
                emitir(err, &format!("error de lectura: {e}\n"));
                return EXIT_ERROR_CONSULTA;
            }
        }
        match interpretar_linea(&mut sesion, &linea, &mut reader, out, err, true) {
            Ok(Accion::Salir) => return EXIT_OK,
            Ok(Accion::Seguir) => {}
            Err(mensaje) => emitir(err, &format!("{mensaje}\n")),
        }
    }
}

// ─────────────────── Subcomando `script` ───────────────────

/// El modo guion: el MISMO intérprete, sin prompts, DETENIENDO en el
/// primer error (con su nº de línea) y exit 1.
///
/// `-` lee stdin (pipe-friendly). Los comentarios (`#`, `--`) y las
/// líneas vacías se saltan; los meta-comandos son legales — un guion
/// puede CONSTRUIR su grafo (`:clear`, `:node…`) y consultarlo.
fn cmd_script(
    fichero: &str,
    origen: &str,
    stdin: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // El frente de lectura: stdin o el fichero pedido.
    let contenido: Box<dyn Read> = if fichero == "-" {
        Box::new(stdin)
    } else {
        match std::fs::File::open(fichero) {
            Ok(f) => Box::new(f),
            Err(e) => {
                emitir(err, &format!("error: no se puede abrir '{fichero}': {e}\n"));
                return EXIT_ERROR_CONSULTA;
            }
        }
    };

    let mut reader = BufReader::new(contenido);
    let mut sesion = Sesion::nueva(origen);
    let mut numero = 0usize;
    loop {
        let mut linea = String::new();
        match reader.read_line(&mut linea) {
            Ok(0) => return EXIT_OK,
            Ok(_) => {}
            Err(e) => {
                emitir(err, &format!("error de lectura: {e}\n"));
                return EXIT_ERROR_CONSULTA;
            }
        }
        numero += 1;
        match interpretar_linea(&mut sesion, &linea, &mut reader, out, err, false) {
            Ok(Accion::Salir) => return EXIT_OK, // :quit corta el guion
            Ok(Accion::Seguir) => {}
            Err(mensaje) => {
                emitir(err, &format!("línea {numero}: {mensaje}\n"));
                return EXIT_ERROR_CONSULTA;
            }
        }
    }
}

// ─────────────────── Ayuda y utilidades ───────────────────

/// La ayuda curada del libro (con EJEMPLOS y el GRAFO DEMO): la que
/// muestran `liradb`, `liradb help`, `--help` y `-h`.
fn imprimir_ayuda(out: &mut dyn Write) {
    emitir(
        out,
        r#"liradb — la CLI de LiraDB (Vol.II, cap. 31: REPL, script y clap)

USO:
  liradb demo                          4 consultas de muestra sobre el grafo demo
  liradb query "<LiraQL>" [--plan] [--stats]
                                       Ejecuta una consulta sobre el grafo demo
  liradb explain "<LiraQL>"            Plan ANTES/DESPUÉS del optimizador (cap. 21)
  liradb repl [--graph demo|empty]     REPL interactivo con sesión (:help dentro)
  liradb script <FICHERO|-> [--graph demo|empty]
                                       Ejecuta un guion de líneas; '-' lee stdin
  liradb --version                     Versión
  liradb help                          Esta ayuda

EJEMPLOS:
  liradb query "MATCH (p:Person) RETURN p.name, p.age"
  printf ':clear\n:node 0:Person name="Zoe" age=44\nMATCH (p) RETURN p.name\n' | liradb script -

REPL — meta-comandos: :help :quit :demo :clear :graph
  :node <id>:<Etiqueta> [clave=valor…]      crear un nodo (autocommit)
  :edge <id> <de> <a> <TIPO> [clave=…]      crear una arista (autocommit)
  :begin → :commit | :rollback              transacción real (cap. 27):
                                           mientras vive, el prompt es tx> y
                                           las consultas esperan (store prestado)

GRAFO DEMO (vol2_liradb::demo_graph, el fixture del cap. 20):
  Person: Ana(36), Bo(41), Carla(29), Dani(36) · City: Madrid, Lisboa
  KNOWS: Ana→Bo, Bo→Carla, Carla→Ana, Dani→Dani · LIVES_IN: Ana→Madrid, Bo→Lisboa

La importación/exportación (CSV, JSONL, GraphML) llega en el cap. 32.
"#,
    );
}

/// Escribe `s` en `out` ignorando fallos de E/S: una CLI didáctica no debe
/// entrar en pánico si la salida se cierra antes de tiempo
/// (p.ej. `liradb demo | head -1`).
fn emitir(out: &mut dyn Write, s: &str) {
    let _ = out.write_all(s.as_bytes());
}

// ─────────────────── Tests: la CLI end-to-end, sin spawn ───────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Ejecuta la CLI con `args` y stdin real (no usado salvo REPL/script).
    fn cli(args: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let codigo = run(&args, &mut out, &mut err);
        (
            codigo,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    /// Como `cli` pero con la ENTRADA inyectada (REPL y `script -`).
    fn cli_con_entrada(args: &[&str], entrada: &str) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut cursor = Cursor::new(entrada.as_bytes().to_vec());
        let codigo = run_con_entrada(&args, &mut cursor, &mut out, &mut err);
        (
            codigo,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // ── El hito, conservado ──────────────────────────────────────

    #[test]
    fn sin_args_muestra_ayuda_y_sale_0() {
        let (codigo, out, err) = cli(&[]);
        assert_eq!(codigo, EXIT_OK);
        assert!(err.is_empty());
        assert!(out.contains("USO:"));
        assert!(out.contains("liradb demo"));
        assert!(out.contains("liradb query"));
    }

    #[test]
    fn help_explicito_muestra_ayuda_y_sale_0() {
        for forma in ["help", "--help", "-h"] {
            let (codigo, out, err) = cli(&[forma]);
            assert_eq!(codigo, EXIT_OK, "forma: {forma}");
            assert!(err.is_empty());
            assert!(out.contains("EJEMPLOS:"));
            // La ayuda curada: un ejemplo de query y uno de script (el
            // cap. 31 amplía el repertorio del hito).
            assert_eq!(out.matches("liradb query \"MATCH").count(), 1);
            assert!(out.contains("liradb script -"));
        }
    }

    #[test]
    fn version_imprime_la_version_y_sale_0() {
        let (codigo, out, err) = cli(&["--version"]);
        assert_eq!(codigo, EXIT_OK);
        assert!(err.is_empty());
        assert!(out.contains(env!("CARGO_PKG_VERSION")), "out: {out}");
    }

    #[test]
    fn demo_ejecuta_las_cuatro_consultas_con_plan_y_tabla() {
        let (codigo, out, err) = cli(&["demo"]);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty());
        assert_eq!(out.matches("── [").count(), 4);
        assert_eq!(out.matches("LiraQL:").count(), 4);
        assert!(out.contains("Plan lógico:"));
        assert!(out.contains("NodeScan(Person AS p)"));
        assert!(out.contains("Expand(p, KNOWS, OUTGOING, f)"));
        assert!(out.contains("Métricas:"));
        assert!(out.contains("\"Ana\""));
        assert!(out.contains("\"Carla\""));
    }

    #[test]
    fn query_match_simple_imprime_la_tabla() {
        let (codigo, out, err) = cli(&["query", "MATCH (p:Person) RETURN p.name, p.age"]);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty());
        let lineas: Vec<&str> = out.lines().map(str::trim_end).collect();
        assert_eq!(lineas[0], "p.name  | p.age");
        assert!(lineas.contains(&"\"Ana\"   | 36"));
        assert_eq!(lineas.len(), 5);
    }

    #[test]
    fn query_error_de_parse_a_stderr_y_exit_1() {
        let (codigo, out, err) = cli(&["query", "SELECCIONA TODO"]);
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        assert!(out.is_empty());
        assert!(err.starts_with("error: "), "stderr: {err}");
        assert!(err.contains("error de sintaxis"), "stderr: {err}");
    }

    #[test]
    fn query_error_runtime_de_tipos_a_stderr_y_exit_1() {
        let (codigo, out, err) = cli(&["query", "MATCH (p:Person) WHERE p.age RETURN p.name"]);
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        assert!(out.is_empty());
        assert!(err.contains("tipos incompatibles"), "stderr: {err}");
    }

    // ── Los errores de uso ahora son de clap (exit 2) ─────────────

    #[test]
    fn subcomando_desconocido_es_error_de_uso_de_clap() {
        let (codigo, out, err) = cli(&["drop", "todo"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(out.is_empty());
        // Mensaje autogenerado por clap, en nuestro writer.
        assert!(err.contains("unrecognized subcommand"), "stderr: {err}");
        assert!(err.contains("'drop'"), "stderr: {err}");
        assert!(err.contains("Usage:"), "stderr: {err}");
    }

    #[test]
    fn query_sin_consulta_es_error_de_uso_de_clap() {
        let (codigo, _, err) = cli(&["query"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(
            err.contains("required arguments were not provided"),
            "stderr: {err}"
        );
    }

    #[test]
    fn graph_invalido_es_error_de_uso_de_clap() {
        let (codigo, _, err) = cli(&["repl", "--graph", "nada"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(err.contains("invalid value"), "stderr: {err}");
        // Las alternativas legales aparecen en el error (ergonomía clap).
        assert!(err.contains("demo"), "stderr: {err}");
        assert!(err.contains("empty"), "stderr: {err}");
    }

    // ── `query` con flags (cap. 31) ───────────────────────────────

    #[test]
    fn query_con_plan_y_stats_a_la_carta() {
        let (codigo, out, err) = cli(&[
            "query",
            "MATCH (p:Person) WHERE p.age < 40 RETURN p.name",
            "--plan",
            "--stats",
        ]);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("Plan lógico:"));
        assert!(out.contains("NodeScan(Person AS p)"));
        assert!(out.contains("Resultado:"));
        assert!(out.contains("Métricas:"));
        // Y sin flags sigue siendo SÓLO la tabla (el hito, intacto).
        let (codigo, out, _) = cli(&["query", "MATCH (p:Person) RETURN p.name"]);
        assert_eq!(codigo, EXIT_OK);
        assert!(!out.contains("Plan lógico:"));
    }

    // ── REPL: la sesión ───────────────────────────────────────────

    #[test]
    fn repl_consulta_y_quit() {
        let (codigo, out, err) = cli_con_entrada(
            &["repl"],
            "MATCH (p:Person) WHERE p.age < 40 RETURN p.name\n:quit\n",
        );
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("LiraDB REPL"));
        assert!(out.contains("liradb> "));
        assert!(out.contains("p.name"));
        assert!(out.contains("\"Ana\""));
        assert!(!out.contains("\"Bo\""));
    }

    #[test]
    fn repl_eof_es_salida_limpia() {
        let (codigo, out, err) = cli_con_entrada(&["repl"], ":graph\n");
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("grafo: 6 nodos, 6 aristas"));
    }

    #[test]
    fn repl_graph_empty_arranca_vacio() {
        let (codigo, out, _) = cli_con_entrada(&["repl", "--graph", "empty"], ":graph\n:q\n");
        assert_eq!(codigo, EXIT_OK);
        assert!(out.contains("0 nodos, 0 aristas"));
    }

    #[test]
    fn repl_los_errores_no_cortan_la_sesion() {
        let (codigo, out, err) = cli_con_entrada(
            &["repl"],
            "SELECCIONA TODO\nMATCH (p:Person) RETURN p.name\n:quit\n",
        );
        assert_eq!(codigo, EXIT_OK);
        assert!(err.contains("error de sintaxis"));
        // La sesión SIGUIÓ: la segunda consulta respondió.
        assert!(out.contains("\"Ana\""));
    }

    // ── REPL: construir el grafo (autocommit del cap. 27) ────────

    #[test]
    fn repl_construye_y_consulta_su_propio_grafo() {
        // EL test-tesis del capítulo: sin ficheros ni importaciones —
        // la CLI como cliente completo del motor.
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":node 0:Person name=\"Zoe\" age=44\n",
                ":node 1:City name=\"Oviedo\"\n",
                ":edge 0 0 1 LIVES_IN since=2019\n",
                "MATCH (p:Person)-[:LIVES_IN]->(c:City) RETURN p.name, c.name\n",
                ":quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty(), "stderr: {err}");
        // Las tres escrituras fueron autocommit…
        assert_eq!(out.matches("autocommit").count(), 3);
        // …y la consulta lo ve TODO (la sesión vive entre líneas).
        assert!(out.contains("p.name"));
        assert!(out.contains("\"Zoe\""));
        assert!(out.contains("\"Oviedo\""));
    }

    #[test]
    fn repl_props_de_todos_los_tipos_y_errores_de_formato() {
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":node 0:X entero=42 real=3.5 cierto=true falso=false nada=null texto=\"hola\"\n",
                ":node 1 sin-dospuntos\n",
                ":node x:X\n",
                ":node 2:X prop_rota\n",
                ":graph\n:quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK);
        assert!(out.contains("ok: nodo 0"));
        // Los tres errores de formato, cada uno con su diagnóstico…
        assert!(err.contains("se espera <id>:<Etiqueta>"), "err: {err}");
        assert!(err.contains("id no numérico"), "err: {err}");
        assert!(err.contains("sin '='"), "err: {err}");
        // …y la sesión siguió: el nodo 2 no llegó, el 0 sí.
        assert!(out.contains("grafo: 1 nodos, 0 aristas"));
    }

    #[test]
    fn repl_demo_recarga_y_clear_vacia() {
        let (codigo, out, _) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            ":demo\n:clear\n:graph\n:quit\n",
        );
        assert_eq!(codigo, EXIT_OK);
        assert!(out.contains("grafo demo cargado: 6 nodos, 6 aristas"));
        assert!(out.contains("sesión vaciada"));
        assert!(out.contains("grafo: 0 nodos, 0 aristas"));
    }

    #[test]
    fn repl_borra_con_cascada_visible() {
        // :del-node de Bo (id 1) arrastra sus aristas (cap. 8): KNOWS
        // Ana→Bo, Bo→Carla y LIVES_IN Bo→Lisboa — tres aristas menos.
        let (codigo, out, err) = cli_con_entrada(
            &["repl"],
            ":del-node 1\n:graph\nMATCH (p:Person)-[r:KNOWS]->(f) RETURN f.name\n:quit\n",
        );
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("ok: delete nodo 1"));
        assert!(out.contains("grafo: 5 nodos, 3 aristas"));
        // Sobreviven Carla→Ana y el self-loop de Dani.
        assert!(out.contains("f.name"));
    }

    #[test]
    fn repl_meta_desconocido_se_reporta() {
        let (codigo, _, err) = cli_con_entrada(&["repl"], ":magic\n:quit\n");
        assert_eq!(codigo, EXIT_OK);
        assert!(err.contains("meta-comando desconocido ':magic'"));
    }

    #[test]
    fn repl_tolerar_el_punto_y_coma_final() {
        let (codigo, out, err) =
            cli_con_entrada(&["repl"], "MATCH (p:Person) RETURN p.name;\n:quit\n");
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty(), "stderr: {err}");
        assert!(out.contains("\"Ana\""));
    }

    // ── REPL: transacciones (el cap. 27, visible) ────────────────

    #[test]
    fn repl_transaccion_commit_persiste() {
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":begin\n",
                ":node 0:Person name=\"Vega\"\n",
                ":commit\n",
                "MATCH (p:Person) RETURN p.name\n",
                ":quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        // El prompt cambia mientras la tx vive…
        assert!(out.contains("tx>"));
        // …el staging se confirma (resumen del cap. 27)…
        assert!(out.contains("commit: 1 operaciones"));
        // …y la consulta POSTERIOR lo ve: persistió en la sesión.
        assert!(out.contains("\"Vega\""));
    }

    #[test]
    fn repl_transaccion_rollback_descarta() {
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":begin\n",
                ":node 0:Person name=\"Fantasma\"\n",
                ":rollback\n",
                "MATCH (p:Person) RETURN p.name\n",
                ":graph\n:quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("rollback: 1 operaciones descartadas"));
        // La consulta no ve al fantasma: nunca existió.
        assert!(!out.contains("\"Fantasma\""));
        assert!(out.contains("grafo: 0 nodos, 0 aristas"));
    }

    #[test]
    fn repl_transaccion_quit_es_rollback_implicito() {
        let (codigo, out, _) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(":begin\n", ":node 0:Person name=\"Efímero\"\n", ":quit\n",),
        );
        assert_eq!(codigo, EXIT_OK);
        assert!(out.contains("rollback implícito"));
        assert!(out.contains("1 operaciones descartadas"));
    }

    #[test]
    fn repl_transaccion_las_consultas_esperan() {
        // El préstamo exclusivo del cap. 27, hecho producto: con la tx
        // abierta, el store está prestado y las consultas se rechazan.
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":begin\n",
                "MATCH (p) RETURN p\n",
                ":demo\n",
                ":commit\n:quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK);
        // Los dos rechazos van a stderr con la explicación del préstamo.
        assert_eq!(err.matches("transacción abierta").count(), 2, "err: {err}");
        assert!(err.contains("prestado"), "err: {err}");
        // Y el :commit final (vacío) dejó la sesión consistente.
        assert!(out.contains("commit: 0 operaciones"));
    }

    #[test]
    fn repl_commit_sin_tx_abierta_se_rechaza() {
        let (codigo, _, err) = cli_con_entrada(&["repl"], ":commit\n:quit\n");
        assert_eq!(codigo, EXIT_OK);
        assert!(err.contains("no hay transacción abierta"));
    }

    #[test]
    fn repl_tx_staging_invalido_no_mata_la_tx() {
        // La validación eager del cap. 27: la op inválida se expulsa y
        // la tx SIGUE VIVA con su prefijo válido.
        let (codigo, out, err) = cli_con_entrada(
            &["repl", "--graph", "empty"],
            concat!(
                ":begin\n",
                ":node 0:Person\n",
                ":edge 0 0 9 KNOWS\n", // 9 no existe: staging rechazado
                ":node 1:Person\n",
                ":commit\n",
                ":graph\n:quit\n",
            ),
        );
        assert_eq!(codigo, EXIT_OK);
        assert!(err.contains("staging rechazado"), "err: {err}");
        // El prefijo válido sobrevivió: 2 nodos tras el commit.
        assert!(out.contains("commit: 2 operaciones"));
        assert!(out.contains("grafo: 2 nodos, 0 aristas"));
    }

    // ── Script mode ───────────────────────────────────────────────

    #[test]
    fn script_por_stdin_con_comentarios_y_multiples_consultas() {
        let guion = concat!(
            "# Construye un mini-grafo y consúltalo (cap. 31)\n",
            ":clear\n",
            ":node 0:Person name=\"Hugo\" age=50\n",
            ":node 1:Person name=\"Irene\" age=33\n",
            ":edge 0 0 1 KNOWS desde=2021\n",
            "\n",
            "MATCH (p:Person) RETURN p.name, p.age\n",
            "MATCH (a)-[r:KNOWS]->(b) RETURN a.name, b.name, r.desde\n",
        );
        let (codigo, out, err) = cli_con_entrada(&["script", "-"], guion);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty(), "stderr: {err}");
        // Sin prompts en modo guion…
        assert!(!out.contains("liradb> "));
        // …dos tablas, y los datos del grafo construido.
        assert_eq!(out.matches("p.name").count(), 1);
        assert!(out.contains("\"Hugo\""));
        assert!(out.contains("\"Irene\""));
        assert!(out.contains("2021"));
    }

    #[test]
    fn script_desde_fichero() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.ql");
        std::fs::write(&ruta, "MATCH (p:City) RETURN p.name\n").unwrap();
        let ruta_str = ruta.to_str().unwrap().to_string();

        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut stdin = Cursor::new(Vec::new());
        let codigo = run_con_entrada(
            &["script".to_string(), ruta_str],
            &mut stdin,
            &mut out,
            &mut err,
        );
        assert_eq!(codigo, EXIT_OK);
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("\"Madrid\""));
        assert!(out.contains("\"Lisboa\""));
    }

    #[test]
    fn script_detiene_en_el_primer_error_con_linea() {
        let guion = concat!(
            "MATCH (p:Person) RETURN p.name\n",
            "SELECCIONA TODO\n",
            "MATCH (p:Person) RETURN p.age\n", // no debe ejecutarse
        );
        let (codigo, out, err) = cli_con_entrada(&["script", "-"], guion);
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        assert!(err.contains("línea 2:"), "stderr: {err}");
        assert!(err.contains("error de sintaxis"), "stderr: {err}");
        // La primera sí respondió; la tercera, jamás.
        assert!(out.contains("p.name"));
        assert!(!out.contains("p.age"));
    }

    #[test]
    fn script_fichero_inexistente_exit_1() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut stdin = Cursor::new(Vec::new());
        let codigo = run_con_entrada(
            &["script".to_string(), "/no/existe.ql".to_string()],
            &mut stdin,
            &mut out,
            &mut err,
        );
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        let err = String::from_utf8(err).unwrap();
        assert!(err.contains("no se puede abrir"), "stderr: {err}");
    }

    #[test]
    fn script_quit_corta_el_guion_y_sale_0() {
        let guion = "MATCH (p:Person) RETURN p.name\n:quit\nMATCH (p) RETURN p.age\n";
        let (codigo, out, _) = cli_con_entrada(&["script", "-"], guion);
        assert_eq!(codigo, EXIT_OK);
        assert!(out.contains("p.name"));
        assert!(!out.contains("p.age"));
    }

    #[test]
    fn script_con_transaccion() {
        // Los guiones también pueden transaccionar (el mismo intérprete).
        let guion = concat!(
            ":clear\n",
            ":begin\n",
            ":node 0:Person name=\"Script\"\n",
            ":commit\n",
            "MATCH (p:Person) RETURN p.name\n",
        );
        let (codigo, out, err) = cli_con_entrada(&["script", "-"], guion);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(out.contains("commit: 1 operaciones"));
        assert!(out.contains("\"Script\""));
    }

    // ── Coherencia del producto ───────────────────────────────────

    #[test]
    fn repl_y_query_y_script_vuelven_el_mismo_dato() {
        // Tres frontales, un motor: la consulta canónica del brief.
        let src = "MATCH (p:Person) WHERE p.age < 40 RETURN p.name";
        let (_, out_q, _) = cli(&["query", src]);
        let (_, out_r, _) = cli_con_entrada(&["repl"], &format!("{src}\n:quit\n"));
        let (_, out_s, _) = cli_con_entrada(&["script", "-"], &format!("{src}\n"));
        for out in [&out_q, &out_r, &out_s] {
            assert!(out.contains("\"Ana\""));
            assert!(out.contains("\"Carla\""));
            assert!(out.contains("\"Dani\""));
            assert!(!out.contains("\"Bo\""));
        }
    }
}
