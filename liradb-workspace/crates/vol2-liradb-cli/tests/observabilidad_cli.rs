//! Cap. 35 — Tests de integración de la observabilidad de la CLI.
//!
//! Tres frentes, los nombres EXACTOS del contrato del capítulo:
//!
//! * `suscriptor_arbol_jerarquia_query_plan_optimise_execute` — la
//!   jerarquía del brief capturada con [`SuscriptorArbol`] vía
//!   `tracing::dispatcher::with_default` (thread-local, sin stderr ni
//!   globales): asserts sobre NODOS y PADRES tipados, jamás sobre texto.
//!
//! * `perfil_cli_*` — el hito `liradb query --profile '...'`: salida SIN
//!   golden byte-exacto (los tiempos varían); se asertan ESTRUCTURA
//!   (nombres/nesting exactos) y CONTADORES exactos, nunca duraciones.
//!
//! * `jerarquia_cuatro_niveles_componente_indice_sobre_pool` — el 4º nivel
//!   (page fetch) demostrado a nivel COMPONENTE: un índice real (cap. 15)
//!   sobre `BufferPool` (cap. 13) sobre pager REAL en fichero temporal,
//!   envuelto medidor+span. La ruta de consultas aún corre sobre
//!   `MemoryStore`: esa frontera se DOCUMENTA, no se finge (herencia de
//!   caps. 33/34).
//!
//! * `perfil_aditivo_goldens_demo_explain_intactos` — vigilancia del flag
//!   ADITIVO: los dorados del cap. 31 siguen byte-exactos sin regenerar.

use std::sync::Arc;

use liradb_cli::observabilidad::{PagerTrazado, SuscriptorArbol, arbol_indentado};
use liradb_cli::{EXIT_OK, run_con_entrada};
use tracing::info_span;
use vol2_liradb::{BufferPool, Contadores, FilePager, HashIndex, MedidorPaginas};

/// La consulta canónica del brief (la misma que dora `explain.txt`).
const CONSULTA_EXPLAIN: &str = "MATCH (p:Person)-[r:KNOWS]->(f:Person) \
         WHERE p.name = \"Ana\" RETURN f.name, r.since";

/// Q2 del demo: Project(Filter(NodeScan)) — contadores conocidos de antemano.
const CONSULTA_Q2: &str = "MATCH (p:Person) WHERE p.age < 40 RETURN p.name";

/// Ejecuta la CLI in-process; devuelve (exit, stdout, stderr).
fn cli(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut stdin = std::io::empty();
    let codigo = run_con_entrada(&args, &mut stdin, &mut out, &mut err);
    (
        codigo,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// Instala un SuscriptorArbol fresco para el cierre de `f` y lo devuelve:
/// el andamiaje de captura de TODOS los tests de jerarquía.
fn capturar<T>(f: impl FnOnce() -> T) -> (Arc<SuscriptorArbol>, T) {
    let sub = Arc::new(SuscriptorArbol::nuevo());
    let dispatch = tracing::Dispatch::from(Arc::clone(&sub));
    let r = tracing::dispatcher::with_default(&dispatch, f);
    (sub, r)
}

/// Id del primer span con ese nombre (pánico descriptivo si falta).
fn id_de(sub: &SuscriptorArbol, nombre: &str) -> u64 {
    sub.arbol()
        .into_iter()
        .find(|(_, n)| n.nombre == nombre)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no hay span '{nombre}' en el árbol"))
}

// ─── La jerarquía completa del brief ────────────────────────────────────

/// TESIS §2: el subscriber reconstruye query → parse/plan/optimise/
/// execute → operador → storage_read con padres contextuales correctos y
/// duraciones fijadas por el `try_close` sobrescrito.
#[test]
fn suscriptor_arbol_jerarquia_query_plan_optimise_execute() {
    let (sub, ()) = capturar(|| {
        let _gq = info_span!("query").entered();
        {
            let _g = info_span!("parse").entered();
        }
        {
            let _g = info_span!("plan").entered();
        }
        {
            // Fase «opcional» del brief: existe como span cuando hay paso
            // de optimización (el pipeline del hito hoy no la tiene).
            let _g = info_span!("optimise").entered();
        }
        {
            let _ge = info_span!("execute").entered();
            let _gi = info_span!("IndexSeek").entered();
            let _gr = info_span!("storage_read").entered();
        }
    });

    assert_eq!(sub.total(), 7, "los siete nombres del brief");

    // Raíz única + hijos directos de query EN ORDEN.
    let raices = sub.hijos(None);
    assert_eq!(raices.len(), 1);
    let id_query = raices[0];
    assert_eq!(
        sub.hijos(Some(id_query)),
        vec![
            id_de(&sub, "parse"),
            id_de(&sub, "plan"),
            id_de(&sub, "optimise"),
            id_de(&sub, "execute"),
        ]
    );

    // Cadena profunda: execute → IndexSeek → storage_read.
    let id_execute = id_de(&sub, "execute");
    let id_seek = id_de(&sub, "IndexSeek");
    assert_eq!(sub.padre_de(id_seek), Some(id_execute));
    assert_eq!(sub.padre_de(id_de(&sub, "storage_read")), Some(id_seek));

    // Duraciones: todo cerró (try_close sobrescrito ES quien las fija).
    for (id, nodo) in sub.arbol() {
        assert!(
            nodo.duracion.is_some(),
            "span {id} ({}) quedó abierto",
            nodo.nombre
        );
    }

    // Y el render indentado respeta los CUATRO niveles en orden creciente.
    let texto = arbol_indentado(&sub);
    let i_q = texto.find("query").unwrap();
    let i_e = texto.find("└─ execute").unwrap();
    let i_s = texto.find("└─ IndexSeek").unwrap();
    let i_p = texto.find("└─ storage_read").unwrap();
    assert!(i_q < i_e && i_e < i_s && i_s < i_p, "\n{texto}");
}

// ─── El hito --profile: estructura pactada, tiempos libres ──────────────

/// TESIS §2: árbol indentado con los operadores REALES anidados bajo
/// execute (Project → Filter → NodeScan para Q2), fases y contadores.
#[test]
fn perfil_cli_arbol_indentado_spans_operador() {
    let (codigo, out, err) = cli(&["query", CONSULTA_Q2, "--profile"]);
    assert_eq!(codigo, EXIT_OK, "stderr: {err}");
    assert!(err.is_empty(), "stderr: {err}");

    // Secciones en ORDEN (índices crecientes en la salida).
    let posicion = |aguja: &str| {
        out.find(aguja)
            .unwrap_or_else(|| panic!("falta '{aguja}' en:\n{out}"))
    };
    let i_res = posicion("Resultado:");
    let i_per = posicion("Perfil (cap. 35):");
    let i_fas = posicion("Fases:");
    let i_arb = posicion("Árbol de spans:");
    let i_con = posicion("Contadores:");
    assert!(i_res < i_per && i_per < i_fas && i_fas < i_arb && i_arb < i_con);

    // Jerarquía EXACTA del pipeline del hito (sin optimise: no hay paso
    // de optimización en esta ruta — frontera documentada, no fingida).
    for linea in [
        "├─ parse",
        "├─ plan",
        "└─ execute",
        "   └─ Project",
        "      └─ Filter",
        "         └─ NodeScan",
    ] {
        assert!(
            out.contains(linea),
            "el árbol debe traer '{linea}':\n{}",
            posicion_arbol(&out)
        );
    }
    // La raíz sin conector, y NADA quedó abierto.
    assert!(out.contains("\nquery"), "raíz query presente");
    assert!(!out.contains("abierto"), "ningún span abierto al imprimir");
}

/// Extracto del bloque del árbol (para mensajes de fallo legibles).
fn posicion_arbol(out: &str) -> String {
    let ini = out.find("Árbol de spans:").unwrap_or(0);
    let fin = out.find("Contadores:").unwrap_or(out.len());
    out[ini..fin].to_string()
}

/// TESIS §2: contadores EXACTOS (derivados de ExecMetrics: una sola
/// verdad) y las tres fases cronometradas con unidad de tiempo presente.
#[test]
fn perfil_contadores_exactos_y_fases_cronometradas() {
    let (codigo, out, err) = cli(&["query", CONSULTA_Q2, "--profile"]);
    assert_eq!(codigo, EXIT_OK, "stderr: {err}");

    // Recibo EXACTO de Q2: 4 Person escaneadas, cero expansión/seek;
    // sin pager ni WAL ni transacciones en la ruta de consulta (0s
    // honestos: la frontera del capítulo, impresa tal cual).
    for esperado in [
        "# TYPE queries_total counter\nqueries_total 1\n",
        "# TYPE nodes_scanned counter\nnodes_scanned 4\n",
        "# TYPE relationships_expanded counter\nrelationships_expanded 0\n",
        "# TYPE index_hits counter\nindex_hits 0\n",
        "# TYPE page_reads counter\npage_reads 0\n",
        "# TYPE page_writes counter\npage_writes 0\n",
        "# TYPE wal_bytes_written counter\nwal_bytes_written 0\n",
        "# TYPE transactions_committed counter\ntransactions_committed 0\n",
        "# TYPE transactions_aborted counter\ntransactions_aborted 0\n",
    ] {
        assert!(
            out.contains(esperado),
            "falta recibo '{esperado}' en:\n{out}"
        );
    }

    // Fases: exactamente parse, plan, execute — cada una CON duración
    // (unidad presente); los valores NO se asertan (varían por máquina).
    let ini = out.find("Fases:\n").unwrap() + "Fases:\n".len();
    let fin = out.find("Árbol de spans:").unwrap();
    let fases: Vec<&str> = out[ini..fin].lines().collect();
    assert_eq!(fases.len(), 3, "tres fases, ni más ni menos:\n{out}");
    assert!(fases[0].starts_with("  parse"));
    assert!(fases[1].starts_with("  plan"));
    assert!(fases[2].starts_with("  execute"));
    for fase in &fases {
        assert!(
            fase.contains("ns") || fase.contains("µs") || fase.contains("ms"),
            "fase sin unidad de tiempo: '{fase}'"
        );
    }

    // Composición aditiva: --profile respeta --plan/--stats.
    let (_, out2, _) = cli(&["query", CONSULTA_Q2, "--profile", "--plan", "--stats"]);
    assert!(out2.contains("Plan lógico:"), "--plan compone");
    assert!(out2.contains("Métricas: Project:"), "--stats compone");
    assert!(!out.contains("Plan lógico:"), "sin --plan no aparece");
}

/// Vigilancia del flag ADITIVO: los goldens del cap. 31 siguen BYTE-EXACTOS
/// sin regenerar (misma comparación manual que `golden_cli.rs`, repetida
/// aquí a propósito: si `--profile` ensuciara la salida por defecto, ESTE
/// test rompe primero y señala la causa).
#[test]
fn perfil_aditivo_goldens_demo_explain_intactos() {
    let dir_demo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/demo.txt");
    let dir_exp = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/explain.txt");

    let (codigo, out, _) = cli(&["demo"]);
    assert_eq!(codigo, EXIT_OK);
    assert_eq!(
        out,
        std::fs::read_to_string(&dir_demo).unwrap(),
        "golden demo.txt DIVERGIÓ: --profile dejó de ser aditivo"
    );

    let (codigo, out, _) = cli(&["explain", CONSULTA_EXPLAIN]);
    assert_eq!(codigo, EXIT_OK);
    assert_eq!(
        out,
        std::fs::read_to_string(&dir_exp).unwrap(),
        "golden explain.txt DIVERGIÓ: --profile dejó de ser aditivo"
    );

    // Y la consulta con --profile sale limpia PERO no toca los dorados:
    // son subcomandos/flags distintos por construcción.
    let (codigo, _, err) = cli(&["query", CONSULTA_Q2, "--profile"]);
    assert_eq!(codigo, EXIT_OK);
    assert!(err.is_empty());
}

// ─── El cuarto nivel a nivel COMPONENTE ─────────────────────────────────

/// TESIS §2 (frontera honesta): query → execute → index_seek →
/// storage_read sobre un HashIndex REAL (cap. 15) montado sobre
/// BufferPool (cap. 13) sobre FilePager REAL en fichero temporal, con
/// medidor+span APILADOS en el pager. Los page_reads del medidor cuadran
/// con los spans emitidos dentro de la región trazada.
///
/// Truco de presión documentado: UN solo bucket (todas las claves caen en
/// la misma cadena) con 2000 entradas ≈ 8 páginas de desborde contra un
/// pool de 4 frames — los `get` NO caben en caché y atraviesan el pager
/// de verdad. (`create` exige capacidad ≥ 3+num_buckets: su flush final
/// toca todas las páginas primarias; con 1 bucket, 4 frames sobran.)
#[test]
fn jerarquia_cuatro_niveles_componente_indice_sobre_pool() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("indice.db");
    let pager = FilePager::create(&ruta).unwrap();

    let contadores = Contadores::new();
    let pool = BufferPool::new(
        PagerTrazado::nuevo(MedidorPaginas::nuevo(pager, &contadores)),
        4,
    );
    let mut indice = HashIndex::create(pool, 1).unwrap();
    for k in 0..2000u64 {
        indice.insert(k, k).unwrap(); // un bucket: cadena de desborde larga
    }
    indice.flush().unwrap();

    // Línea base ANTES de la región trazada (create/insert ya tocaron
    // páginas sin subscriber): sólo el DELTA pertenece a este test.
    let antes = contadores.snapshot();
    let lecturas_pool_antes = indice.pool().metrics().page_reads;

    let (sub, encontrados) = capturar(|| {
        let _gq = info_span!("query").entered();
        {
            let _ge = info_span!("execute").entered();
            let _gi = info_span!("index_seek").entered();
            let mut vistos = Vec::new();
            for k in [0u64, 50, 199, 250, 350] {
                if let Some(v) = indice.get(k).unwrap() {
                    vistos.push(v);
                }
            }
            vistos
        }
    });

    assert_eq!(encontrados.len(), 5, "las cinco claves están indexadas");

    // Jerarquía de CUATRO niveles, padres exactos:
    let id_query = id_de(&sub, "query");
    let id_execute = id_de(&sub, "execute");
    let id_seek = id_de(&sub, "index_seek");
    assert_eq!(sub.padre_de(id_execute), Some(id_query));
    assert_eq!(sub.padre_de(id_seek), Some(id_execute));

    let storage_reads: Vec<u64> = sub
        .arbol()
        .into_iter()
        .filter(|(_, n)| n.nombre == "storage_read")
        .map(|(id, _)| id)
        .collect();
    assert!(
        !storage_reads.is_empty(),
        "cadena de ~8 páginas vs pool de 4: DEBE haber misses → storage_read"
    );
    for &id in &storage_reads {
        assert_eq!(
            sub.padre_de(id),
            Some(id_seek),
            "cada page fetch cuelga del index_seek"
        );
    }

    // Coherencia medidor↔spans: cada read OK del pager = 1 span Y 1 page_read.
    let despues = contadores.snapshot();
    let delta_reads = despues.page_reads - antes.page_reads;
    assert!(
        delta_reads > 0,
        "sin lecturas de pager no hay cuarto nivel que demostrar"
    );
    assert_eq!(
        delta_reads as usize,
        storage_reads.len(),
        "medidor y traza cuentan LA MISMA realidad"
    );
    // Y el POOL (cap. 13, composición) vio exactamente las mismas:
    assert_eq!(
        indice.pool().metrics().page_reads - lecturas_pool_antes,
        delta_reads
    );
}
