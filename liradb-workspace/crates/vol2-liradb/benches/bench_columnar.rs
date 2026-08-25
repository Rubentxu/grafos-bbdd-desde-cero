//! Benchmarks del cap. 38: almacenamiento columnar y ejecución vectorizada.
//!
//! Tres contrastes, todos sobre el MISMO dataset determinista
//! (`dataset_referencia(SEMILLA_REFERENCIA)`, 100k nodos / 500k aristas del
//! cap. 34), con la metodología de la casa: `black_box`, warm-up,
//! `Throughput::Elements` y hardware declarado en la prosa.
//!
//! - `filtro_row_vs_columna_edad`: el MISMO predicado (`edad > UMBRAL`)
//!   ejecutado fila a fila (HashMap lookups + tag por celda, el layout actual
//!   de LiraDB) frente al filtro por lotes de 1024 sobre columna tipada.
//!   La extracción columnar va FUERA de la región medida en ambos lados —
//!   se mide el ESCANEO analítico repetible, no la preparación única: es la
//!   separación OLTP (escribir/leer filas) vs. analítica (leer columnas
//!   muchas veces) hecha cronometraje.
//! - `decodificacion_diccionario_ciudad`: cuánto cuesta recuperar las
//!   cadenas desde los códigos u32 — la mitad del precio que SIGMOD 2006 nos
//!   advirtió (comprimir acelera SOLO si puedes operar sin decodificar).
//! - `desempaquetado_bits`: deshacer el bit packing de los mismos códigos
//!   (u32 → k bits → u32).
//!
//! Delimitación SIMD honesta (contrato §2): lo que este fichero promete es
//! el DELTA medido row-vs-columna; la verificación por ensamblador
//! (`cargo asm` / `rustc --emit=asm`) es OPCIONAL y vive FUERA de este
//! pipeline. Nada aquí garantiza instrucciones vectoriales concretas.

use std::hint::black_box;
use std::sync::OnceLock;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use vol2_liradb::{
    CLAVES_ESQUEMA, ColumnaTipada, DatasetReferencia, Diccionario, GraphStore, SEMILLA_REFERENCIA,
    TAMANIO_VECTOR, TablaColumnar, bits_necesarios, dataset_referencia, desempaquetar, empaquetar,
    filtrar_fila_sobre_ids, ids_ordenados,
};

/// Umbral del predicado compartido por ambas rutas del filtro.
const UMBRAL_EDAD: i64 = 50;

fn dataset() -> &'static DatasetReferencia {
    static DATASET: OnceLock<DatasetReferencia> = OnceLock::new();
    DATASET.get_or_init(|| dataset_referencia(SEMILLA_REFERENCIA))
}

/// Informe de una sola línea junto al grupo: selectividad real del filtro.
fn reportar(nombre: &str, seleccionados: usize, total: usize) {
    println!("{nombre}: {seleccionados}/{total} filas pasan el predicado");
}

fn bench_columnar(c: &mut Criterion) {
    let ds = dataset();

    // ── Grupo 1: filtro escalar row vs filtro por lotes sobre columna ──
    // Preparación UNICA fuera de las regiones medidas (ambos lados reciben
    // el mismo trato: ids ordenados para la fila, tabla extraída para la
    // columna). Dentro solo queda el ESCANEO.
    let ids = ids_ordenados(&ds.store);
    let tabla = TablaColumnar::desde_store(&ds.store, &CLAVES_ESQUEMA);
    // La selectividad que la prosa citará: la MISMA condición que se
    // cronometra (contar presencia con `|_| true` sería otra historia).
    let pasan_edad = filtrar_fila_sobre_ids(&ds.store, &ids, "edad", |v| v > UMBRAL_EDAD).len();
    reportar(
        &format!("filtro edad > {UMBRAL_EDAD}"),
        pasan_edad,
        ds.store.node_count(),
    );

    let mut g = c.benchmark_group("filtro_row_vs_columna_edad");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(6));
    g.throughput(Throughput::Elements(ds.store.node_count() as u64));

    g.bench_function("row_escalar_hashmap", |b| {
        b.iter(|| {
            black_box(filtrar_fila_sobre_ids(
                &ds.store,
                black_box(&ids),
                "edad",
                |v| v > UMBRAL_EDAD,
            ))
        })
    });
    g.bench_function("columna_lotes_1024", |b| {
        b.iter(|| black_box(tabla.filtrar_int("edad", |v| v > UMBRAL_EDAD).expect("Int")))
    });
    g.finish();

    // ── Grupo 2: decode del diccionario de ciudad ──
    let ciudades = tabla
        .columna("ciudad")
        .and_then(ColumnaTipada::cadenas_presentes)
        .expect("ciudad es String");
    let diccionario = Diccionario::nuevo(&ciudades);
    let estadistica = diccionario.estadisticas();
    println!(
        "diccionario ciudad: cardinalidad={} bytes {} -> {} ratio x{:.2}",
        estadistica.cardinalidad,
        estadistica.bytes_antes,
        estadistica.bytes_despues,
        estadistica.ratio()
    );

    let mut g = c.benchmark_group("decodificacion_diccionario_ciudad");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));
    g.throughput(Throughput::Elements(diccionario.codigos().len() as u64));
    g.bench_function("decodificar_completa", |b| {
        b.iter(|| black_box(diccionario.decodificar()))
    });
    g.finish();

    // ── Grupo 3: unpack del bit packing de esos mismos códigos ──
    let bits = bits_necesarios(estadistica.cardinalidad);
    let empaquetado = empaquetar(diccionario.codigos(), bits);
    println!(
        "packing ciudad: {} códigos a {bits} bits -> {} palabras u64 ({} B vs {} B en u32)",
        diccionario.codigos().len(),
        empaquetado.len(),
        8 * empaquetado.len(),
        4 * diccionario.codigos().len(),
    );

    let n_codigos = diccionario.codigos().len();
    let mut g = c.benchmark_group("desempaquetado_bits");
    g.sample_size(30);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));
    g.throughput(Throughput::Elements(n_codigos as u64));
    g.bench_function("desempaquetar_codigos_ciudad", |b| {
        b.iter(|| black_box(desempaquetar(black_box(&empaquetado), n_codigos, bits)))
    });
    g.finish();

    // El tamaño de lote queda visible en la salida para citarlo en prosa.
    println!("TAMANIO_VECTOR = {TAMANIO_VECTOR}");
}

criterion_group!(benches, bench_columnar);
criterion_main!(benches);
