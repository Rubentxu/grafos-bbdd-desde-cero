//! Vol.II — Hito **CLI mínima** (tras el cap. 20, ADR-005): el binario
//! `liradb` que hace el motor demostrable desde shell. Desde el cap. 21
//! incluye además `liradb explain` (el hito de ese capítulo).
//!
//! La lógica vive en esta LIB (no en `main`) para ser testeada sin
//! arrancar procesos: [`run`] recibe los argumentos ya sin `argv[0]`,
//! dos writers (salida y error) y devuelve el código de salida. Así los
//! tests de [`mod tests`] ejercitan la cadena end-to-end real —de la
//! cadena de texto al exit code— con un `Vec<u8>` como stdout.
//!
//! Subcomandos:
//! - `liradb demo` — grafo de ejemplo + 4 consultas representativas,
//!   imprimiendo consulta, plan lógico OPTIMIZADO (caps. 19+21), tabla
//!   (`Display` del `ResultSet` del cap. 20) y métricas por operador.
//! - `liradb query "<LiraQL>"` — ejecuta una consulta sobre el grafo
//!   demo (pipeline completo con optimizador); errores de
//!   parse/plan/ejecución van a stderr con su `Display` y el exit code
//!   es != 0.
//! - `liradb explain "<LiraQL>"` — el hito del cap. 21: plan ANTES
//!   (lower) y DESPUÉS (optimize) con cardinalidades estimadas, el
//!   catálogo de estadísticas y el contraste con las filas reales.
//! - `liradb help` (o sin argumentos) — ayuda breve.
//!
//! Decisiones del hito (documentadas también en MIGRATION-PATTERN §25-26):
//! - **Parseo de argumentos MANUAL con `std::env::args`**, sin clap: la
//!   regla del Vol.II es "primero a mano, luego con crates", y la CLI
//!   completa del cap. 31 introducirá clap con subcomandos ricos, REPL
//!   interactivo e import/export. Aquí basta un `match` sobre el primer
//!   argumento.
//! - **Grafo demo**: se reutiliza [`vol2_liradb::demo_graph`] (el fixture
//!   de los tests del cap. 20, ahora público) en vez de duplicarlo en la
//!   CLI — un único punto de verdad para el grafo de las demos.
//! - **Códigos de salida**: 0 = OK · 1 = error de consulta
//!   (parse/plan/ejecución) · 2 = error de uso. Estilo común de las
//!   herramientas Unix.

use std::io::Write;

use vol2_liradb::{ExecError, Executor, GraphStore, demo_graph, explain, parse};

/// Salida correcta.
pub const EXIT_OK: i32 = 0;
/// La consulta falló (parse, plan o ejecución): el mensaje va a stderr.
pub const EXIT_ERROR_CONSULTA: i32 = 1;
/// Uso incorrecto de la CLI (subcomando desconocido, arity erróneo).
pub const EXIT_ERROR_USO: i32 = 2;

/// Las 4 consultas de `liradb demo`: una por capa del pipeline.
///
/// 1. MATCH simple → `NodeScan` + `Project` (la consulta mínima).
/// 2. WHERE con comparación → `Filter` con evaluación trivalente.
/// 3. Patrón relacional → `Expand` sobre la adyacencia.
/// 4. La consulta canónica del brief (cap. 19-20) + propiedad de arista.
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

/// Punto de entrada testable de la CLI.
///
/// `args` es `argv[1..]` (sin el nombre del binario). Escribe el resultado
/// en `out` y los errores en `err`; el valor devuelto es el código de
/// salida del proceso (ver constantes [`EXIT_OK`], …).
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // Sin argumentos → ayuda breve (hito: demostrable sin leer manual).
    let Some(sub) = args.first() else {
        imprimir_ayuda(out);
        return EXIT_OK;
    };

    match sub.as_str() {
        "help" | "--help" | "-h" if args.len() == 1 => {
            imprimir_ayuda(out);
            EXIT_OK
        }
        "demo" if args.len() == 1 => cmd_demo(out, err),
        "query" if args.len() == 2 => cmd_query(&args[1], out, err),
        "query" => {
            error_de_uso(
                err,
                "`query` espera exactamente una consulta entre comillas:\n  \
                 liradb query \"MATCH (p:Person) RETURN p.name\"",
            );
            EXIT_ERROR_USO
        }
        "explain" if args.len() == 2 => cmd_explain(&args[1], out, err),
        "explain" => {
            error_de_uso(
                err,
                "`explain` espera exactamente una consulta entre comillas:\n  \
                 liradb explain \"MATCH (p:Person) RETURN p.name\"",
            );
            EXIT_ERROR_USO
        }
        otro => {
            error_de_uso(
                err,
                &format!("subcomando o argumentos no válidos: '{otro}' (ver `liradb help`)"),
            );
            EXIT_ERROR_USO
        }
    }
}

// ─────────────────── Subcomando `query` ───────────────────

/// Ejecuta una consulta LiraQL sobre el grafo demo e imprime la tabla.
///
/// La cadena completa del cap. 20 (`vol2_liradb::run` = parse → lower →
/// Volcano); cualquier fallo se reporta a stderr con su `Display` y el
/// código es [`EXIT_ERROR_CONSULTA`].
fn cmd_query(consulta: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let store = demo_graph();
    match vol2_liradb::run(consulta, &store) {
        Ok(rs) => {
            emitir(out, &rs.to_string());
            EXIT_OK
        }
        Err(e) => {
            emitir(err, &format!("error: {e}\n"));
            EXIT_ERROR_CONSULTA
        }
    }
}

// ─────────────────── Subcomando `explain` ───────────────────

/// El hito del cap. 21: plan ANTES/DESPUÉS con cardinalidades estimadas.
///
/// Toda la lógica vive en `vol2_liradb::explain` (catálogo, reglas,
/// estimaciones y el contraste con las filas reales); aquí sólo se cablea
/// el subcomando y la gestión de errores/exit codes, como `query`.
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

/// Ejecuta las 4 consultas de muestra sobre el grafo demo.
///
/// Cada bloque imprime: la consulta, su plan lógico OPTIMIZADO (el ingenuo
/// del cap. 19 reescrito por las reglas del cap. 21), la tabla de
/// resultados (`Display` del `ResultSet`) y las métricas reales por
/// operador (`ExecMetrics`). Para ver el ANTES y el DESPUÉS lado a lado
/// (con estimaciones) está `liradb explain`.
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
            // Las consultas del demo están curadas: esto sería un bug.
            emitir(err, &format!("error: {e}\n"));
            return EXIT_ERROR_CONSULTA;
        }
    }

    emitir(
        out,
        "\nPrueba tus propias consultas: liradb query \"...\" (ver `liradb help`).\n",
    );
    EXIT_OK
}

/// Pipeline completo de UNA consulta, con plan y métricas visibles.
///
/// Es `vol2_liradb::run` desenrollado: en vez de llamar directamente al
/// hito del cap. 20, usa `parse` → `lower` → `Executor` para poder
/// imprimir el plan (antes de ejecutar) y las métricas (después).
fn ejecutar_con_detalle(
    src: &str,
    store: &dyn GraphStore,
    out: &mut dyn Write,
) -> Result<(), ExecError> {
    let query = parse(src)?;
    let plan = query.lower()?;
    emitir(out, "Plan lógico:\n");
    emitir(out, &format!("{plan}\n"));

    let mut executor = Executor::new(&plan, store)?;
    let rs = executor.execute()?;
    emitir(out, "Resultado:\n");
    emitir(out, &rs.to_string());
    emitir(out, &format!("Métricas: {}\n", executor.metrics()));
    Ok(())
}

// ─────────────────── Ayuda y errores de uso ───────────────────

/// Ayuda breve: subcomandos + 2 ejemplos (el hito pide exactamente eso).
fn imprimir_ayuda(out: &mut dyn Write) {
    emitir(
        out,
        r#"liradb — CLI mínima de LiraDB (Vol.II, hito tras el cap. 20)

USO:
  liradb demo               4 consultas de muestra sobre el grafo demo
  liradb query "<LiraQL>"   Ejecuta una consulta LiraQL sobre el grafo demo
  liradb help               Muestra esta ayuda

EJEMPLOS:
  liradb query "MATCH (p:Person) RETURN p.name, p.age"
  liradb query "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = \"Ana\" RETURN f.name"

GRAFO DEMO (vol2_liradb::demo_graph, el fixture del cap. 20):
  Person: Ana(36), Bo(41), Carla(29), Dani(36) · City: Madrid, Lisboa
  KNOWS: Ana→Bo, Bo→Carla, Carla→Ana, Dani→Dani · LIVES_IN: Ana→Madrid, Bo→Lisboa

La CLI completa (REPL, import/export, configuración) llega en el cap. 31.
"#,
    );
}

/// Error de uso a stderr, con recordatorio del help.
fn error_de_uso(err: &mut dyn Write, mensaje: &str) {
    emitir(err, &format!("error de uso: {mensaje}\n"));
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

    /// Ejecuta la CLI con `args` y devuelve `(código, stdout, stderr)`.
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
        let (codigo, out, err) = cli(&["help"]);
        assert_eq!(codigo, EXIT_OK);
        assert!(err.is_empty());
        assert!(out.contains("EJEMPLOS:"));
        // El hito pide 2 ejemplos concretos en la ayuda.
        assert_eq!(out.matches("liradb query \"MATCH").count(), 2);
    }

    #[test]
    fn demo_ejecuta_las_cuatro_consultas_con_plan_y_tabla() {
        let (codigo, out, err) = cli(&["demo"]);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty());
        // Cuatro bloques, uno por consulta del hito.
        assert_eq!(out.matches("── [").count(), 4);
        // Cada bloque: consulta + plan (Display del cap. 19) + tabla + métricas.
        assert_eq!(out.matches("LiraQL:").count(), 4);
        assert!(out.contains("Plan lógico:"));
        assert!(out.contains("NodeScan(Person AS p)"));
        assert!(out.contains("Expand(p, KNOWS, OUTGOING, f)"));
        assert!(out.contains("Métricas:"));
        // Datos reales del grafo demo en las tablas.
        assert!(out.contains("\"Ana\""));
        assert!(out.contains("\"Carla\""));
        assert!(out.contains("36"));
        assert!(out.contains("29"));
    }

    #[test]
    fn query_match_simple_imprime_la_tabla() {
        let (codigo, out, err) = cli(&["query", "MATCH (p:Person) RETURN p.name, p.age"]);
        assert_eq!(codigo, EXIT_OK, "stderr: {err}");
        assert!(err.is_empty());
        let lineas: Vec<&str> = out.lines().map(str::trim_end).collect();
        // Ancho col0 = 7 ("Carla" con comillas): la cabecera se rellena.
        assert_eq!(lineas[0], "p.name  | p.age");
        // 4 personas + cabecera; Carla (7 chars con comillas) fija el ancho.
        assert!(lineas.contains(&"\"Ana\"   | 36"));
        assert!(lineas.contains(&"\"Carla\" | 29"));
        assert!(lineas.contains(&"\"Dani\"  | 36"));
        assert_eq!(lineas.len(), 5);
    }

    #[test]
    fn query_con_where_filtra_por_comparacion() {
        let (codigo, out, _err) =
            cli(&["query", "MATCH (p:Person) WHERE p.age < 40 RETURN p.name"]);
        assert_eq!(codigo, EXIT_OK);
        // Menores de 40: Ana (36), Carla (29) y Dani (36); Bo (41) queda fuera.
        assert!(out.contains("\"Ana\""));
        assert!(out.contains("\"Carla\""));
        assert!(out.contains("\"Dani\""));
        assert!(!out.contains("\"Bo\""));
        // Cabecera + 3 filas.
        assert_eq!(out.lines().count(), 4);
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
    fn query_error_de_plan_a_stderr_y_exit_1() {
        // x no está ligada por el MATCH: el binder del cap. 19 lo rechaza.
        let (codigo, out, err) = cli(&["query", "MATCH (p:Person) RETURN x.name"]);
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        assert!(out.is_empty());
        assert!(err.contains("error de planificación"), "stderr: {err}");
    }

    #[test]
    fn query_error_runtime_de_tipos_a_stderr_y_exit_1() {
        // `WHERE p.age` (INT como bool) pasa el plan (schemaless → Any)
        // y falla en ejecución: la concreción runtime del cap. 20.
        let (codigo, out, err) = cli(&["query", "MATCH (p:Person) WHERE p.age RETURN p.name"]);
        assert_eq!(codigo, EXIT_ERROR_CONSULTA);
        assert!(out.is_empty());
        assert!(err.contains("tipos incompatibles"), "stderr: {err}");
    }

    #[test]
    fn subcomando_desconocido_es_error_de_uso() {
        let (codigo, out, err) = cli(&["drop", "todo"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(out.is_empty());
        assert!(err.contains("error de uso"), "stderr: {err}");
        assert!(err.contains("'drop'"), "stderr: {err}");
    }

    #[test]
    fn query_sin_consulta_o_con_sobra_es_error_de_uso() {
        let (codigo, _, err) = cli(&["query"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(err.contains("una consulta entre comillas"), "stderr: {err}");

        let (codigo, _, err) = cli(&["query", "MATCH (p) RETURN p", "extra"]);
        assert_eq!(codigo, EXIT_ERROR_USO);
        assert!(err.contains("error de uso"), "stderr: {err}");
    }

    #[test]
    fn demo_y_query_coinciden_en_el_resultado() {
        // La consulta 2 del demo, lanzada por `query`, produce la misma
        // tabla: un solo motor detrás de dos puertas.
        let src = "MATCH (p:Person) WHERE p.age < 40 RETURN p.name, p.age";
        let (_, out_query, _) = cli(&["query", src]);
        let (_, out_demo, _) = cli(&["demo"]);
        let tabla_en_demo = out_demo
            .split("── [3/4]")
            .next()
            .unwrap()
            .split("── [2/4]")
            .nth(1)
            .unwrap();
        assert!(tabla_en_demo.contains("p.name  | p.age"));
        assert!(out_query.contains("p.name  | p.age"));
        // Mismas filas (Ana y Carla, menores de 40).
        for fila in ["\"Ana\"", "\"Carla\""] {
            assert!(out_query.contains(fila));
            assert!(tabla_en_demo.contains(fila));
        }
        assert!(!out_query.contains("\"Bo\""));
    }
}
