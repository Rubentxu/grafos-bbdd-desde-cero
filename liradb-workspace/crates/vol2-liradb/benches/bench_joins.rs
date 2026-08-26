//! Benchmarks del cap. 39: joins, patrones y consultas cíclicas.
//!
//! Tres contrastes, todos sobre datasets DETERMINISTAS propios (construidos
//! a mano o con el PRNG del cap. 34 y `SEMILLA_REFERENCIA`), con la
//! metodología de la casa: `black_box`, warm-up, `Throughput::Elements` y
//! hardware declarado en la prosa.
//!
//! - `binario_vs_wcoj_regular`: grafo aleatorio uniforme (la topología
//!   «amable»). El delta esperado es HONESTO y quizá modesto: con datos
//!   selectivos y acíclicos un plan binario clásico suele aguantar
//!   (Veldhuizen, ICDT 2014) — si no sale diferencia, SE REPORTA tal cual.
//! - `binario_vs_wcoj_hub_skew`: EL EXPERIMENTO del capítulo. Rueda con hub:
//!   el concentrador dispara los intermedios del join binario (Σ_b in·out)
//!   mientras la respuesta crece lineal — ahí el leapfrog debe AGUANTAR,
//!   porque jamás materializa el intermedio.
//! - `enumeracion_factorizada_vs_plana`: AGREGAR sobre el resultado del join
//!   en su representación factorizada (sumar multiplicidades por prefijo) ni
//!   siquiera expande las tuplas; el plano recorre todas, una a una.
//!
//! Los CONTADORES de trabajo (intermedios, pasos de búsqueda, cota AGM,
//! celdas físicas) NO viven aquí: son del módulo y su test-informe
//! (`informe_joins_reproducible_sobre_mini`) — regla del cap. 34: criterion
//! cronometra, los contadores se cuentan.

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use vol2_liradb::{
    AdyacenciasOrdenadas, ResultadoFactorizadoTriangulos, SEMILLA_REFERENCIA, TriangulosWcoj,
    Xorshift64Star, intermedios_plan_binario, triangulos_join_binario,
};

// ─────────────────── Datasets deterministas del bench ───────────────────

/// Grafo UNIFORME dirigido-simétrico: `pares` parejas distintas (u,v) con
/// aristas en ambos sentidos. Topología regular sin hubs: el caso donde el
/// plan binario es MÁS competitivo — el contraste honesto.
const NODOS_REGULAR: usize = 800;
const PARES_REGULAR: usize = 6_000;

fn grafo_regular() -> AdyacenciasOrdenadas {
    let mut rng = Xorshift64Star::new(SEMILLA_REFERENCIA);
    let mut aristas = Vec::with_capacity(PARES_REGULAR * 2);
    for _ in 0..PARES_REGULAR {
        let u = rng.debajo_de(NODOS_REGULAR as u64) as usize;
        let v = rng.debajo_de(NODOS_REGULAR as u64) as usize;
        if u != v {
            aristas.push((u, v));
            aristas.push((v, u));
        }
    }
    AdyacenciasOrdenadas::desde_aristas(NODOS_REGULAR, &aristas)
}

/// Rueda bidireccional con hub: centro 0 ↔ cada hoja y ciclo entre hojas
/// consecutivas. Skew CONTROLADO: el hub aporta in=out=hojas (sus caminos
/// dominan Σ in·out cuadráticamente) mientras los triángulos crecen LINEAL.
const HOJAS_HUB: usize = 512;

fn grafo_hub_rueda(hojas: usize) -> AdyacenciasOrdenadas {
    let mut aristas = Vec::with_capacity(hojas * 4);
    for hoja in 1..=hojas {
        aristas.push((0, hoja));
        aristas.push((hoja, 0));
        let siguiente = if hoja == hojas { 1 } else { hoja + 1 };
        aristas.push((hoja, siguiente));
        aristas.push((siguiente, hoja));
    }
    AdyacenciasOrdenadas::desde_aristas(hojas + 1, &aristas)
}

/// Contexto de una sola línea junto al grupo: las cifras que la prosa citará.
fn reportar_contexto(nombre: &str, adj: &AdyacenciasOrdenadas) {
    let wcoj = TriangulosWcoj::enumerar(adj);
    println!(
        "{nombre}: nodos={} aristas={} intermedios_binarios={} triángulos={} pasos_wcoj={}",
        adj.num_nodos(),
        adj.num_aristas(),
        intermedios_plan_binario(adj),
        wcoj.triangulos.len(),
        wcoj.pasos_buscador,
    );
}

fn bench_joins(c: &mut Criterion) {
    // ── Grupo 1: regular vs regular (delta honesto, quizá modesto) ──
    let adj = grafo_regular();
    reportar_contexto("grafo_regular_uniforme", &adj);

    let mut g = c.benchmark_group("binario_vs_wcoj_regular");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));
    g.throughput(Throughput::Elements(adj.num_aristas() as u64));

    g.bench_function("binario_materializa_intermedios", |b| {
        b.iter(|| black_box(triangulos_join_binario(black_box(&adj))))
    });
    g.bench_function("wcoj_leapfrog_orden_estatico", |b| {
        b.iter(|| black_box(TriangulosWcoj::enumerar(black_box(&adj))))
    });
    g.finish();

    // ── Grupo 2: EL EXPERIMENTO — hub skew dispara los intermedios ──
    let hub = grafo_hub_rueda(HOJAS_HUB);
    reportar_contexto("rueda_hub_skew", &hub);

    let mut g = c.benchmark_group("binario_vs_wcoj_hub_skew");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));
    g.throughput(Throughput::Elements(hub.num_aristas() as u64));

    g.bench_function("binario_materializa_intermedios", |b| {
        b.iter(|| black_box(triangulos_join_binario(black_box(&hub))))
    });
    g.bench_function("wcoj_leapfrog_orden_estatico", |b| {
        b.iter(|| black_box(TriangulosWcoj::enumerar(black_box(&hub))))
    });
    g.finish();

    // ── Grupo 3: agregar factorizado vs recorrer tuplas planas ──
    // Misma pregunta lógica («¿cuántos triángulos hay por valor de a?»),
    // dos representaciones físicas. La factorizada responde sumando
    // MULTIPLICIDADES; la plana tiene que tocar cada tupla.
    let wcoj = TriangulosWcoj::enumerar(&adj);
    let factorizado = ResultadoFactorizadoTriangulos::desde_triangulos(&wcoj.triangulos);
    let planas = wcoj.triangulos.clone();
    println!(
        "factorización: filas_lógicas={} celdas_físicas={} celdas_planas={} ahorro={:.1}%",
        factorizado.filas_logicas(),
        factorizado.celdas_fisicas(),
        factorizado.celdas_planas(),
        factorizado.ahorro_porcentaje()
    );

    let mut g = c.benchmark_group("enumeracion_factorizada_vs_plana");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));
    g.throughput(Throughput::Elements(factorizado.filas_logicas()));

    g.bench_function("agregar_sumando_multiplicidades", |b| {
        b.iter(|| {
            let mut por_a = vec![0_u64; NODOS_REGULAR];
            for (k, &a) in factorizado.prefijos_a.iter().enumerate() {
                por_a[a] = factorizado.multiplicidad_a[k];
            }
            black_box(por_a)
        })
    });
    g.bench_function("plano_recorre_cada_tupla", |b| {
        b.iter(|| {
            let mut por_a = vec![0_u64; NODOS_REGULAR];
            for &(a, _, _) in planas.iter() {
                por_a[a] += 1;
            }
            black_box(por_a)
        })
    });
    g.finish();

    // Equivalencia de las DOS respuestas (fuera del cronómetro): el mismo
    // multiconjunto agregado, distinto coste físico.
    let mut desde_factorizado = vec![0_u64; NODOS_REGULAR];
    for (k, &a) in factorizado.prefijos_a.iter().enumerate() {
        desde_factorizado[a] = factorizado.multiplicidad_a[k];
    }
    let mut desde_plano = vec![0_u64; NODOS_REGULAR];
    for &(a, _, _) in &planas {
        desde_plano[a] += 1;
    }
    assert_eq!(desde_factorizado, desde_plano);
}

criterion_group!(benches, bench_joins);
criterion_main!(benches);
