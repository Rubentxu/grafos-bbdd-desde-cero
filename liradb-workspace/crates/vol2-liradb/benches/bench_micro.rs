//! Microbenchmarks de COMPONENTES de LiraDB (cap. 34).
//!
//! Cada grupo responde la pregunta de la torre de instrumentos: «¿cuánto
//! tarda UNA operación de pieza?» — encode/decode del cap. 9, seek de los
//! índices del cap. 15 sobre páginas reales en tempfile, iteración CSR vs.
//! puerto (caps. 14/8), proyección/Dijkstra/PageRank (caps. 26/22/24),
//! frío/caliente HONESTO del BufferPool (caps. 12/13) y UNA TRAMPA que mide
//! mal a propósito para la sección de trampas de la prosa.
//!
//! Reglas del fichero:
//! - `Throughput::Elements(n)` SIEMPRE: sin denominador declarado,
//!   «segundos» no dicen nada (criterion convierte a elem/s comparables).
//! - APIs REALES, nada reescrito: lo que se mide es lo que hay en el crate.
//! - El frío/caliente es de COMPONENTE (BufferPool sobre FilePager real):
//!   el cold-cache de CONSULTAS completas no existe hoy — no hay DiskStore
//!   detrás del puerto — y fingirlo está prohibido.
//!
//! Ejecución manual (fuera de verify.sh, como todo bench):
//!
//! ```text
//! cargo bench -p vol2-liradb --bench bench_micro
//! ```

use std::hint::black_box;
use std::sync::OnceLock;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;
use vol2_liradb::{
    BPlusTree, BufferPool, Csr, DatasetReferencia, FilePager, FiltroProyeccion, GraphStore,
    HashIndex, NODOS_DATASET, NodeId, PAGE_SIZE, Pager, ProyeccionPonderada, SEMILLA_REFERENCIA,
    Value, WeightSource, dataset_referencia, decode_value, dijkstra, encode_value, page_rank,
};

/// Dataset compartido del proceso (una construcción de ~0,3 s en release).
fn dataset() -> &'static DatasetReferencia {
    static DATASET: OnceLock<DatasetReferencia> = OnceLock::new();
    DATASET.get_or_init(|| dataset_referencia(SEMILLA_REFERENCIA))
}

// ─────────────────── Cap 9: encode/decode de Value ───────────────────

const VALORES_ENCODE: u64 = 256;

fn bench_encoding(c: &mut Criterion) {
    let mut g = c.benchmark_group("09_encoding_value");
    // Mezcla de tipos representativa: strings dominan en property graphs.
    let muestras: Vec<Value> = (0..VALORES_ENCODE)
        .map(|i| match i % 5 {
            0 => Value::Int(i as i64),
            1 => Value::Float(f64::from(i as u32) / 3.0),
            2 => Value::String(format!("valor-{i}-de-longitud-media")),
            3 => Value::Bool(i.is_multiple_of(2)),
            _ => Value::Bytes(vec![i as u8; 16]),
        })
        .collect();

    g.throughput(Throughput::Elements(VALORES_ENCODE));
    g.bench_function("encode_decode_roundtrip", |b| {
        b.iter(|| {
            for v in &muestras {
                let bytes = encode_value(v);
                let (recuperado, resto) = decode_value(&bytes).expect("roundtrip limpio");
                assert!(resto.is_empty());
                black_box(recuperado);
            }
        })
    });
    g.finish();
}

// ─────────────────── Cap 15: seek de índices sobre páginas ─────────────

/// Índice hash CALIENTE: creado, poblado y flushed sobre FilePager real.
///
/// La capacidad del pool (4096 frames) cubre directorio + cubetas: el bench
/// mide el coste LÓGICO del seek, no el thrash del pool (eso se mide solo en
/// su grupo frío/caliente, donde pertenece).
struct IndiceHashBench {
    _dir: TempDir,
    indice: HashIndex<FilePager>,
}

fn indice_hash_caliente(claves: usize, buckets: u32) -> IndiceHashBench {
    let dir = tempfile::tempdir().expect("tempdir");
    let pager = FilePager::create(dir.path().join("hash.idx")).expect("pager");
    let pool = BufferPool::new(pager, 4096);
    let mut indice = HashIndex::create(pool, buckets).expect("crea hash");
    for k in 0..claves as u64 {
        indice.insert(k, k ^ 0xDEAD_BEEF).expect("inserta");
    }
    indice.flush().expect("flush");
    IndiceHashBench { _dir: dir, indice }
}

/// Árbol B+ CALIENTE de un solo nivel (cap 15): capacidad física ~200
/// entradas por página — el límite pedagógico del capítulo marca la escala.
struct ArbolBench {
    _dir: TempDir,
    arbol: BPlusTree<FilePager>,
}

fn arbol_b_caliente(claves: u64) -> ArbolBench {
    let dir = tempfile::tempdir().expect("tempdir");
    let pager = FilePager::create(dir.path().join("arbol.idx")).expect("pager");
    let pool = BufferPool::new(pager, 64);
    let mut arbol = BPlusTree::create(pool).expect("crea árbol");
    for k in 0..claves {
        arbol.insert(k, k.wrapping_mul(7)).expect("inserta");
    }
    arbol.flush().expect("flush");
    ArbolBench { _dir: dir, arbol }
}

fn bench_seek_indices(c: &mut Criterion) {
    let mut g = c.benchmark_group("15_seeks_indices");

    // HashIndex: 10.000 claves / 2.048 cubetas (carga media 5 + overflow).
    const BUSQUEDAS_HASH: u64 = 10_000;
    let mut hash = indice_hash_caliente(BUSQUEDAS_HASH as usize, 2_048);
    g.throughput(Throughput::Elements(BUSQUEDAS_HASH));
    g.bench_function("hash_index_get", |b| {
        b.iter(|| {
            for i in 0..BUSQUEDAS_HASH {
                black_box(hash.indice.get(i).expect("get ok"));
            }
        })
    });

    // B+ tree: 192 entradas (capacidad de página ~200) — exactas y rango.
    const CLAVES_ARBOL: u64 = 192;
    const BUSQUEDAS_ARBOL: u64 = CLAVES_ARBOL * 8;
    let arbol = arbol_b_caliente(CLAVES_ARBOL);
    g.throughput(Throughput::Elements(BUSQUEDAS_ARBOL));
    g.bench_function("bplustree_get", |b| {
        b.iter(|| {
            for i in 0..BUSQUEDAS_ARBOL {
                black_box(arbol.arbol.get(i % CLAVES_ARBOL));
            }
        })
    });

    const ANCHO_RANGO: u64 = 32;
    const RANGOS: u64 = CLAVES_ARBOL / ANCHO_RANGO; // cobertura completa
    g.throughput(Throughput::Elements(RANGOS * ANCHO_RANGO));
    g.bench_function("bplustree_range_scan_ancho_32", |b| {
        b.iter(|| {
            for paso in 0..RANGOS {
                let lo = paso * ANCHO_RANGO;
                black_box(arbol.arbol.range_scan(lo, lo + ANCHO_RANGO - 1));
            }
        })
    });

    g.finish();
}

// ─────────────────── Caps 14/8: iteración CSR vs. puerto ──────────────

fn bench_iteracion_csr_vs_store(c: &mut Criterion) {
    let ds = dataset();
    let pares: Vec<(NodeId, NodeId)> = ds
        .store
        .iter_edges()
        .map(|e| (e.source, e.target))
        .collect();
    let csr = Csr::from_edges(pares).expect("csr del dataset");
    let nodos = ds.store.node_count();
    let aristas = ds.store.edge_count() as u64;
    let store = &ds.store;

    let mut g = c.benchmark_group("14_csr_vs_puerto");
    g.sample_size(50);
    g.measurement_time(Duration::from_secs(8));
    g.throughput(Throughput::Elements(aristas));

    // CSR: slices planos, sin clones ni indirección por arista.
    g.bench_function("csr_neighbors_out_todos", |b| {
        b.iter(|| {
            let mut suma = 0u64;
            for u in 0..nodos {
                for v in csr.neighbors_out(u) {
                    suma += *v as u64;
                }
            }
            black_box(suma)
        })
    });

    // Puerto (cap 8): `out_edges` CLONA el Vec de aristas por llamada y cada
    // eid exige un `get_edge`. Es el coste honesto de la abstracción actual —
    // el contraste que justifica materializar proyecciones (cap 26).
    g.bench_function("puerto_out_edges_y_get_edge_todos", |b| {
        b.iter(|| {
            let mut suma = 0u64;
            for u in 0..nodos {
                for eid in store.out_edges(u) {
                    if let Some(e) = store.get_edge(eid) {
                        suma += e.target as u64;
                    }
                }
            }
            black_box(suma)
        })
    });

    g.finish();
}

// ─────────────────── Caps 26/22/24: proyección y algoritmos ───────────

fn bench_algoritmos(c: &mut Criterion) {
    let ds = dataset();
    let mut g = c.benchmark_group("algoritmos_dataset");
    // Grupos caros: menos samples y ventana corta pero suficiente (criterion
    // reparte las iteraciones dentro de la ventana declarada).
    g.sample_size(20);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_secs(4));

    // Proyección materializada completa (cap 26): UNA pasada que paga la
    // semántica estricta de pesos y deja el store quieto después.
    g.throughput(Throughput::Elements(ds.store.edge_count() as u64));
    g.bench_function("26_proyeccion_ponderada_proyectar", |b| {
        b.iter(|| {
            black_box(
                ProyeccionPonderada::proyectar(
                    &ds.store,
                    &WeightSource::default(),
                    &FiltroProyeccion::todo(),
                )
                .expect("proyecta"),
            )
        })
    });

    // Dijkstra completo desde un hub (peso constante = contar saltos).
    g.throughput(Throughput::Elements(NODOS_DATASET as u64));
    g.bench_function("22_dijkstra_completo_desde_hub", |b| {
        b.iter(|| {
            black_box(dijkstra(&ds.store, ds.hubs[0], &WeightSource::default()).expect("dijkstra"))
        })
    });

    // PageRank con tope de 10 iteraciones: cada llamada paga SU proyección
    // interna (así vive hoy la API del cap 24) más las iteraciones de
    // potencia; denominador = aristas × iteraciones máximas.
    const ITERACIONES_PR: u64 = 10;
    g.throughput(Throughput::Elements(
        ds.store.edge_count() as u64 * ITERACIONES_PR,
    ));
    g.bench_function("24_page_rank_hasta_10_iteraciones", |b| {
        b.iter(|| black_box(page_rank(&ds.store, 0.85, ITERACIONES_PR, 1e-9).expect("pagerank")))
    });

    g.finish();
}

// ─────────────────── Caps 12/13: frío vs. caliente HONESTO ────────────

const PAGINAS_POOL: u32 = 256;

fn prepara_fichero_paginas(path: &std::path::Path) {
    let mut pager = FilePager::create(path).expect("pager nuevo");
    for _ in 0..PAGINAS_POOL {
        pager.allocate().expect("asigna");
    }
    for id in 0..PAGINAS_POOL {
        let mut buf = vec![0u8; PAGE_SIZE];
        buf[0] = id as u8;
        pager.write(id, &buf).expect("escribe");
    }
    pager.sync().expect("sync");
}

fn abre_pool(path: &std::path::Path, capacidad: usize) -> BufferPool<FilePager> {
    let pager = FilePager::open(path).expect("reabre");
    BufferPool::new(pager, capacidad)
}

fn una_pasada(pool: &mut BufferPool<FilePager>) -> u64 {
    let mut checksum = 0u64;
    for id in 0..PAGINAS_POOL {
        let pagina = pool.get_page(id).expect("lee");
        checksum += u64::from(pagina[0]);
        pool.unpin(id, false).expect("unpin");
    }
    checksum
}

fn bench_buffer_pool_frio_caliente(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pool_bench.bin");
    prepara_fichero_paginas(&path);

    let mut g = c.benchmark_group("13_buffer_pool");
    g.throughput(Throughput::Elements(u64::from(PAGINAS_POOL)));

    // FRÍO: 16 frames « 256 páginas ⇒ CADA barrido medido es todo-miss
    // (Clock/LRU expulsa lo visitado antes de volver a necesitarlo).
    let mut frio = abre_pool(&path, 16);
    g.bench_function("pasada_fria_todo_misses", |b| {
        b.iter(|| black_box(una_pasada(&mut frio)))
    });

    // CALIENTE: capacidad ≥ páginas ⇒ tras el calentamiento, todo-hit.
    let mut caliente = abre_pool(&path, PAGINAS_POOL as usize + 16);
    una_pasada(&mut caliente); // precarga fuera de la región medida
    g.bench_function("pasada_caliente_todo_hits", |b| {
        b.iter(|| black_box(una_pasada(&mut caliente)))
    });
    g.finish();

    // INFORME de contadores por VENTANA (deltas, porque los contadores del
    // cap. 13 son monotónicos y no se reinician): el hit_ratio es lo único
    // HONESTO que este capítulo puede decir de frío/caliente — a nivel
    // componente. El cold-cache de consultas completas espera a un DiskStore
    // detrás del puerto (no existe hoy).
    let mut informe_frio = abre_pool(&path, 16);
    let base_fria = informe_frio.metrics();
    una_pasada(&mut informe_frio);
    let mf = informe_frio.metrics();
    let (hits, misses) = (
        mf.buffer_hits - base_fria.buffer_hits,
        mf.buffer_misses - base_fria.buffer_misses,
    );
    println!(
        "buffer_pool FRÍO   (cap=16, {} páginas): ventana hits={hits} misses={misses} \
         evictions={} hit_ratio={:.3}",
        PAGINAS_POOL,
        mf.evictions - base_fria.evictions,
        if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    );
    let mut informe_caliente = abre_pool(&path, PAGINAS_POOL as usize + 16);
    una_pasada(&mut informe_caliente); // precarga: ventana fuera del informe
    let base_caliente = informe_caliente.metrics();
    una_pasada(&mut informe_caliente);
    una_pasada(&mut informe_caliente);
    let mc = informe_caliente.metrics();
    let (hits, misses) = (
        mc.buffer_hits - base_caliente.buffer_hits,
        mc.buffer_misses - base_caliente.buffer_misses,
    );
    println!(
        "buffer_pool CALIENTE(cap={}, {} páginas): ventana hits={hits} misses={misses} \
         evictions={} hit_ratio={:.3}",
        PAGINAS_POOL + 16,
        PAGINAS_POOL,
        mc.evictions - base_caliente.evictions,
        if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    );
}

// ─────────────────── LA TRAMPA didáctica (miente a propósito) ─────────

/// ⚠️ ESTE BENCH MIENTE A PROPÓSITO — es el espécimen de la sección de
/// trampas de la prosa (N.12). NO tomar su cifra como válida jamás.
///
/// Comete los dos pecados clásicos a la vez:
///
/// 1. **Setup contabilizado**: construir el valor (`format!`, la
///    asignación) ocurre DENTRO del bucle medido. Lo que crece no es la
///    operación bajo estudio (encode) sino el trabajo de preparación — la
///    cifra infla el coste real del componente.
///
/// 2. **Resultado sin `black_box`**: el `Vec<u8>` devuelto se descarta sin
///    opacidad alguna, invitando al compilador a eliminar trabajo muerto
///    cuando el benchmark se simplifica. La defensa estándar es
///    `std::hint::black_box` — exactamente lo que TODOS los otros benches
///    de este fichero sí hacen.
///
/// Ejercicio de la prosa: PREDECIR qué cifra infla cada pecado antes de
/// correrlo, y comparar contra `09_encoding_value/encode_decode_roundtrip`.
fn bench_trampa_didactica(c: &mut Criterion) {
    let mut g = c.benchmark_group("trampas");
    g.throughput(Throughput::Elements(VALORES_ENCODE));
    g.bench_function("trampa_setup_en_el_loop_sin_black_box", |b| {
        b.iter(|| {
            // Mismo bucle que `09_encoding_value/encode_decode_roundtrip`,
            // salvo UN detalle venenoso: el SETUP (format! + String) se
            // factura dentro del tiempo medido. Y el resultado final nunca
            // pasa por black_box...
            for i in 0..VALORES_ENCODE {
                let v = Value::String(format!("valor-{i}-de-longitud-media"));
                let bytes = encode_value(&v);
                let _ = decode_value(&bytes);
            }
        })
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_encoding,
    bench_seek_indices,
    bench_iteracion_csr_vs_store,
    bench_algoritmos,
    bench_buffer_pool_frio_caliente,
    bench_trampa_didactica,
);
criterion_main!(benches);
