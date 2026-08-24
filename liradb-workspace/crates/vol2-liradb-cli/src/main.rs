//! Binario `liradb` — punto de entrada FINO (cap. 31).
//!
//! Toda la lógica vive en la lib de `liradb-cli` (testable sin arrancar
//! procesos); `main` sólo recoge `argv` + stdin, delega en
//! [`liradb_cli::run_con_entrada`] y propaga el código de salida. Lo que
//! no se puede testar no debe crecer.

use std::io::Write;

fn main() {
    // El parseo de argumentos ya es de clap (dentro de la lib); aquí sólo
    // se recogen argv y stdin, que son frontera de proceso.
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let codigo = liradb_cli::run_con_entrada(&args, &mut stdin, &mut stdout, &mut stderr);

    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(codigo);
}
