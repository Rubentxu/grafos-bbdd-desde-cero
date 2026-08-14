//! Binario `liradb` — punto de entrada FINO.
//!
//! Toda la lógica vive en la lib de `liradb-cli` (testable sin arrancar
//! procesos); `main` sólo recoge `argv`, delega en [`liradb_cli::run`] y
//! propaga el código de salida. Lo que no se puede testar no debe crecer.

use std::io::Write;

fn main() {
    // Parseo MANUAL con std (std::env::args): la regla del Vol.II es
    // "primero a mano, luego con crates" — clap llega con la CLI completa
    // del cap. 31 (subcomandos ricos, flags, ayuda generada, REPL).
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let codigo = liradb_cli::run(&args, &mut stdout, &mut stderr);

    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(codigo);
}
