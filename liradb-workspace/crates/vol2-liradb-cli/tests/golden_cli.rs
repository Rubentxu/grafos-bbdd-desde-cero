//! Cap. 33 — **Golden tests de la CLI, a mano** (std puro: leer fichero +
//! `assert_eq!` + env var de regeneración).
//!
//! La salida de USUARIO (`liradb demo`, `liradb explain`) queda PACTADA en
//! ficheros versionados bajo `tests/golden/`. Nada de crates de snapshots:
//! `insta` y compañía aportan maquinaria que este capítulo — precisamente
//! sobre decidir qué se necesita — no compra.
//!
//! * DETERMINISMO primero (misconcepción nº3 del capítulo): hay un test
//!   que ejecuta cada comando DOS veces y exige bytes idénticos; sin eso,
//!   dorar es copiar el output al repo y rezar.
//! * COMPARACIÓN a mano con diff mínimo en el mensaje del fallo (línea
//!   concreta + contenido esperado vs actual).
//! * REGENERACIÓN explícita: `ACTUALIZAR_GOLDEN=1 cargo test -p liradb-cli
//!   --test golden_cli` reescribe los dorados e imprime un AVISO en stdout;
//!   el cambio queda visible en el `git diff` para revisarlo como lo que
//!   es: una decisión de producto, no un artefacto automático.
//!
//! Todo corre por [`liradb_cli::run_con_entrada`] (cap. 31): sin spawn ni
//! TTY; la captura es un `Vec<u8>`.

use std::path::PathBuf;

use liradb_cli::run_con_entrada;

/// La consulta dorada de `explain`: la canónica del brief (MATCH relacional
/// con variable de arista + WHERE por propiedad + RETURN mixto). Estable por
/// diseño: toca parseo, binding, optimizador (expand + push-down) y plan.
const CONSULTA_EXPLAIN: &str = "MATCH (p:Person)-[r:KNOWS]->(f:Person) \
         WHERE p.name = \"Ana\" RETURN f.name, r.since";

/// Ejecuta la CLI in-process y devuelve su salida estándar COMO BYTES.
///
/// Un comando dorado debe salir 0 y no escribir nada a stderr: si lo hace,
/// el fallo es del comando, no del dorado.
fn ejecutar(args: &[&str]) -> Vec<u8> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut stdin = std::io::empty();
    let codigo = run_con_entrada(&args, &mut stdin, &mut out, &mut err);
    assert_eq!(
        codigo,
        0,
        "{args:?} terminó con código {codigo} (stderr: {})",
        String::from_utf8_lossy(&err)
    );
    assert!(
        err.is_empty(),
        "{args:?} escribió a stderr: {}",
        String::from_utf8_lossy(&err)
    );
    out
}

fn ruta_dorado(nombre: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(nombre)
}

/// El corazón del golden a mano: compara contra el fichero versionado o,
/// con `ACTUALIZAR_GOLDEN=1`, lo regenera AVISANDO por stdout.
fn comprobar_golden(nombre: &str, actual: &[u8]) {
    let ruta = ruta_dorado(nombre);
    if std::env::var("ACTUALIZAR_GOLDEN").as_deref() == Ok("1") {
        std::fs::write(&ruta, actual).expect("escribir el dorado");
        println!(
            "AVISO: dorado '{nombre}' REGENERADO ({} bytes). Revisa el git diff antes \
             de confirmar: pactar salida de usuario es una decisión, no un trámite.",
            actual.len()
        );
        return;
    }
    let esperado = std::fs::read_to_string(&ruta).unwrap_or_else(|e| {
        panic!(
            "no se pudo leer el dorado '{nombre}' ({e}). Regenera con: \
             ACTUALIZAR_GOLDEN=1 cargo test -p liradb-cli --test golden_cli"
        )
    });
    let actual_texto =
        String::from_utf8(actual.to_vec()).expect("la salida de la CLI es UTF-8 legítimo");
    if esperado != actual_texto {
        panic!(
            "el dorado '{nombre}' DIVERGIÓ.\n{}\nSi el cambio es intencional, \
             regenera con ACTUALIZAR_GOLDEN=1 y revisa el diff.",
            primera_divergencia(nombre, &esperado, &actual_texto)
        );
    }
}

/// Diff mínimo: la PRIMERA línea distinta, con su número y ambos contenidos.
/// Suficiente para diagnosticar un cambio de formato sin herramientas.
fn primera_divergencia(_nombre: &str, esperado: &str, actual: &str) -> String {
    let lineas_esperadas: Vec<&str> = esperado.lines().collect();
    let lineas_actuales: Vec<&str> = actual.lines().collect();
    for (i, (e, a)) in lineas_esperadas
        .iter()
        .zip(lineas_actuales.iter())
        .enumerate()
    {
        if e != a {
            return format!(
                "primera diferencia en la línea {}:\n  esperado: {e:?}\n  actual:   {a:?}",
                i + 1
            );
        }
    }
    format!(
        "coinciden hasta la línea {}; una versión tiene {} líneas y la otra {}",
        lineas_esperadas.len().min(lineas_actuales.len()),
        lineas_esperadas.len(),
        lineas_actuales.len()
    )
}

#[test]
fn golden_demo_coincide() {
    comprobar_golden("demo.txt", &ejecutar(&["demo"]));
}

#[test]
fn golden_explain_coincide() {
    comprobar_golden("explain.txt", &ejecutar(&["explain", CONSULTA_EXPLAIN]));
}

#[test]
fn golden_las_salidas_son_deterministas() {
    // PRECONDICIÓN del dorado (la misconcepción «copiar y rezar»): dos
    // ejecuciones consecutivas deben producir EXACTAMENTE los mismos bytes.
    // Sin timestamps, sin iteración de HashMap visible al usuario, sin azar:
    // si esto falla, NO se puede dorar — se arregla el comando primero.
    for args in [&["demo"][..], &["explain", CONSULTA_EXPLAIN][..]] {
        let primera = ejecutar(args);
        let segunda = ejecutar(args);
        assert_eq!(
            primera, segunda,
            "{args:?} no es determinista: corregir antes de dorar"
        );
    }
}
