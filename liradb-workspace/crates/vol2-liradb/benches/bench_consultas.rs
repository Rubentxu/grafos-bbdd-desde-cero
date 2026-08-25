//! Benchmarks de CONSULTA end-to-end sobre el dataset de referencia (cap. 34).
//!
//! Cada bench responde la segunda pregunta de la torre de instrumentos:
//! «¿cuánto tarda una consulta COMPLETA?». Las cinco consultas del contrato
//! (Q1..Q5) corren sobre el MISMO grafo determinista
//! (`dataset_referencia(SEMILLA_REFERENCIA)`), así que diferencias entre
//! runs son ruido de medición, no entrada distinta.
//!
//! Qué se mide y qué NO, con honestidad hexagonal:
//! - Q1/Q2: ejecución del plan SEMI-LIGADO que produce la regla R4 del
//!   cap. 21 (`IndexSeek` con ids ya resueltos). Los ids se resuelven UNA
//!   vez fuera de la región medida porque `Catalog::collect` es hoy
//!   cuadrático en valores distintos (~224 s medidos con los 100k emails
//!   únicos — ver doc del módulo `cap34_benchmarks`): el test
//!   `optimizador_real_produce_index_seek_en_mini` demuestra que el
//!   optimizador genera EXACTAMENTE este plan donde el catálogo es barato.
//! - Q3/Q4: pipeline de TEXTO completo (parse → lower → Executor), la misma
//!   ruta de la CLI hoy: sin catálogo, sin optimizador.
//! - Q5: camino mínimo vía API `dijkstra_path` (LiraQL aún no tiene
//!   shortest-path: DELIMITADO, no fingido).
//!
//! Junto a cada tiempo se REPORTAN los contadores internos (`ExecMetrics`
//! del cap. 20) capturados por el pipeline desenrollado — filas reales por
//! operador, la mitad de la verdad que el tiempo solo no cuenta.
//! Instrumentarlos en producción es el cap. 35.
//!
//! Baselines contra sí misma (nada de Neo4j sensacionalista):
//!
//! ```text
//! cargo bench -p vol2-liradb --bench bench_consultas -- --save-baseline cap34-inicial
//! cargo bench -p vol2-liradb --bench bench_consultas -- --baseline cap34-inicial
//! ```

use std::hint::black_box;
use std::sync::OnceLock;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use vol2_liradb::{
    DatasetReferencia, ExecMetrics, ResultSet, SEMILLA_REFERENCIA, TEXTO_Q3_SCAN_FILTRO,
    TEXTO_Q4_PROYECCION_AMPLIA, camino_minimo_q5, dataset_referencia, ejecutar_plan,
    ejecutar_texto, plan_q1_point_lookup, plan_q2_expand_desde,
};

fn dataset() -> &'static DatasetReferencia {
    static DATASET: OnceLock<DatasetReferencia> = OnceLock::new();
    DATASET.get_or_init(|| dataset_referencia(SEMILLA_REFERENCIA))
}

/// Informe de contadores junto al tiempo: nombre, filas devueltas y las
/// filas que REALMENTE fluyeron por cada operador (pre-orden).
fn reportar(nombre: &str, rs: &ResultSet, m: &ExecMetrics) {
    let operadores: Vec<String> = m
        .per_operator
        .iter()
        .map(|(op, filas)| format!("{op}:{filas}"))
        .collect();
    println!(
        "{nombre}: filas_devueltas={} | operadores [{}] ",
        rs.len(),
        operadores.join(" ")
    );
}

fn bench_consultas(c: &mut Criterion) {
    let ds = dataset();
    let mut g = c.benchmark_group("consultas_dataset_referencia");
    // Ventana moderada por grupo: cinco consultas × estadística completa
    // caben holgadas bajo los ~90 s del binario.
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(5));

    // ── Q1: point-lookup por igualdad (ruta IndexSeek, cap. 21) ──
    let q1 = plan_q1_point_lookup(ds, 7);
    let (rs, m) = ejecutar_plan(&ds.store, &q1).expect("Q1");
    reportar("Q1_point_lookup_email_index_seek", &rs, &m);
    g.throughput(Throughput::Elements(rs.len().max(1) as u64));
    g.bench_function("q1_point_lookup_email_index_seek", |b| {
        b.iter(|| black_box(ejecutar_plan(&ds.store, &q1).expect("Q1")))
    });

    // ── Q2a: expand 1-hop desde un HUB (el fanout caro) ──
    let hub = ds.hubs[0];
    let q2_hub = plan_q2_expand_desde(hub);
    let (rs, m) = ejecutar_plan(&ds.store, &q2_hub).expect("Q2 hub");
    reportar("Q2a_expand_1hop_desde_hub", &rs, &m);
    g.throughput(Throughput::Elements(rs.len().max(1) as u64));
    g.bench_function("q2a_expand_un_hop_desde_hub", |b| {
        b.iter(|| black_box(ejecutar_plan(&ds.store, &q2_hub).expect("Q2a")))
    });

    // ── Q2b: expand 1-hop desde un nodo de GRADO BAJO (el contraste) ──
    let pobre = ds.nodo_grado_bajo;
    let q2_bajo = plan_q2_expand_desde(pobre);
    let (rs, m) = ejecutar_plan(&ds.store, &q2_bajo).expect("Q2 bajo");
    reportar("Q2b_expand_1hop_desde_nodo_grado_bajo", &rs, &m);
    g.throughput(Throughput::Elements(rs.len().max(1) as u64));
    g.bench_function("q2b_expand_un_hop_desde_nodo_grado_bajo", |b| {
        b.iter(|| black_box(ejecutar_plan(&ds.store, &q2_bajo).expect("Q2b")))
    });

    // Las dos consultas PESADAS piden su propia ventana (criterion avisa si
    // no cabe: 100k nodos de scan/proyección no caben en 3 s con 30 muestras).
    g.measurement_time(Duration::from_secs(12));

    // ── Q3: scan + filtro SIN índice (predicado de rango, texto completo) ──
    let (rs, m) = ejecutar_texto(&ds.store, TEXTO_Q3_SCAN_FILTRO).expect("Q3");
    reportar("Q3_scan_filtro_sin_indice", &rs, &m);
    g.throughput(Throughput::Elements(rs.len().max(1) as u64));
    g.bench_function("q3_scan_filtro_sin_indice", |b| {
        b.iter(|| black_box(ejecutar_texto(&ds.store, TEXTO_Q3_SCAN_FILTRO).expect("Q3")))
    });

    // ── Q4: proyección amplia sobre los 100k nodos ──
    let (rs, m) = ejecutar_texto(&ds.store, TEXTO_Q4_PROYECCION_AMPLIA).expect("Q4");
    reportar("Q4_proyeccion_amplia", &rs, &m);
    g.throughput(Throughput::Elements(rs.len().max(1) as u64));
    g.bench_function("q4_proyeccion_amplia", |b| {
        b.iter(|| black_box(ejecutar_texto(&ds.store, TEXTO_Q4_PROYECCION_AMPLIA).expect("Q4")))
    });

    // ── Q5: camino mínimo vía API dijkstra_path (delimitado por contrato) ──
    let camino = camino_minimo_q5(ds).expect("Q5 encuentra camino en el dataset");
    let saltos = camino.hops() as u64;
    println!(
        "Q5_camino_minimo_dijkstra_path: hops={saltos} (par garantizado por BFS en la generación)"
    );
    g.throughput(Throughput::Elements(saltos.max(1)));
    g.bench_function("q5_camino_minimo_dijkstra_path", |b| {
        b.iter(|| black_box(camino_minimo_q5(ds)))
    });

    g.finish();
}

criterion_group!(benches, bench_consultas);
criterion_main!(benches);
