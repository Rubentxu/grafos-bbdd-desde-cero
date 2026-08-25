//! Smoke de arquitectura de LiraDB — Vol.II, cap. 36 «Arquitectura final».
//!
//! Ningún capítulo anterior construyó EL EDIFICIO entero: cada uno probó SU
//! piso (el WAL su replay, el pager sus páginas, el executor sus filas).
//! Este fichero es la primera vez que la torre completa se levanta en una
//! respiración, desde FUERA del crate (`tests/` de integración, API pública
//! plana vía `vol2_liradb::*`) — exactamente como un usuario real la vería.
//!
//! Qué demuestra cada test, en vocabulario hexagonal (cap. 8):
//! - [`la_torre_completa_se_construye_en_una_respiracion`] — dominio +
//!   puerto + pipeline + fiabilidad + recuperación + MVCC + observabilidad,
//!   todo sobre UNA instancia de `MemoryStore`.
//! - [`la_pila_fisica_encadena_pager_pool_csr_e_indices`] — el anillo físico:
//!   `FilePager` → `BufferPool` → `PersistentCsr` / `HashIndex` / `BPlusTree`.
//! - [`cada_puerto_tiene_su_adaptador_canonico`] — inventario: los cinco
//!   traits usados en POSICIÓN de puerto con sus adaptadores canónicos.
//! - [`los_algoritmos_corren_sobre_el_mismo_puerto_que_la_consulta`] — el
//!   puerto compartido entre OLTP (consulta) y OLAP (algoritmos).
//! - [`el_formato_jsonl_sobrevive_al_roundtrip_del_puerto`] — frontera E/S
//!   (cap. 32): exportar → importar a un store fresco → igualdad elemento a
//!   elemento.
//!
//! Contrato del capítulo (§2): los tres primeros nombres son vinculantes.
//! Determinista por construcción: sin tiempos, sin hilos, sin goldens.

use std::io::BufReader;

use tempfile::tempdir;
use vol2_liradb::{
    AntesImagenes, BPlusTree, BufferPool, Catalog, Cell, Contadores, Csr, Edge, EuclideanHeuristic,
    Executor, FilePager, GraphStore, HashIndex, MemoryStore, MvccStore, Node, NodeScanOp,
    Operacion, PAGE_SIZE, Pager, PersistentCsr, PhysicalOperator, PolicyKind, Value, Wal,
    WalTransaccion, WeightSource, ZeroHeuristic, a_star, derivar_contadores, dijkstra,
    exportar_jsonl, guardar_wal, importar_jsonl, lower, metricas_consulta, optimize, page_rank,
    parse, reabrir, run, verificar_invariantes,
};

/// Consulta del recorrido guiado: Ana y a quién conoce.
const SRC_CONSULTA: &str =
    r#"MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name"#;

/// Nodos del grafo mini (determinista, compartido por todos los tests).
///
/// Todos llevan `x`/`y` para que `EuclideanHeuristic` pueda consultarse
/// contra cualquier nodo alcanzable, y las personas llevan `name` para el
/// pipeline LiraQL. Los pesos son fraccionarios (`.5`) a propósito: el
/// exportador JSONL imprime `f.to_string()` y un `Float` con valor entero
/// (p. ej. `2.0` → `"2"`) volvería como `Int` al importar — hallazgo que la
/// prosa del cap. 36 cita como agujero honesto del formato (cap. 32).
fn nodos_base() -> Vec<Node> {
    vec![
        Node::new(0, "Person")
            .with_prop("name", Value::String("Ana".into()))
            .with_prop("x", Value::Int(0))
            .with_prop("y", Value::Int(0)),
        Node::new(1, "Person")
            .with_prop("name", Value::String("Bob".into()))
            .with_prop("x", Value::Int(1))
            .with_prop("y", Value::Int(0)),
        Node::new(2, "Person")
            .with_prop("name", Value::String("Carla".into()))
            .with_prop("x", Value::Int(2))
            .with_prop("y", Value::Int(0)),
        Node::new(3, "City")
            .with_prop("name", Value::String("Madrid".into()))
            .with_prop("x", Value::Int(0))
            .with_prop("y", Value::Int(1)),
        Node::new(4, "City")
            .with_prop("name", Value::String("Lisboa".into()))
            .with_prop("x", Value::Int(0))
            .with_prop("y", Value::Int(2)),
    ]
}

/// Aristas del grafo mini. TODAS llevan `peso` porque la sanidad del cap. 22
/// valida el contrato completo de pesos antes de responder (una consulta no
/// debe depender de qué zona del grafo llegó a pisar).
fn aristas_base() -> Vec<Edge> {
    vec![
        Edge::new(0, 0, 1, "KNOWS").with_prop("peso", Value::Float(2.5)),
        Edge::new(1, 1, 2, "KNOWS").with_prop("peso", Value::Float(3.5)),
        Edge::new(2, 0, 2, "KNOWS").with_prop("peso", Value::Float(10.5)),
        Edge::new(3, 1, 3, "LIVES_IN").with_prop("peso", Value::Float(1.5)),
        Edge::new(4, 2, 4, "LIVES_IN").with_prop("peso", Value::Float(1.5)),
    ]
}

/// El mismo grafo mini cargado en un `MemoryStore` fresco.
fn grafo_base_store() -> MemoryStore {
    let mut store = MemoryStore::new();
    for nodo in nodos_base() {
        store.put_node(nodo).unwrap();
    }
    for arista in aristas_base() {
        store.put_edge(arista).unwrap();
    }
    store
}

/// El grafo mini como lote de `Operacion` (cap. 27) para `MvccStore::commit`
/// (cap. 30): mismos datos, otro consumidor del vocabulario transaccional.
fn ops_grafo_base() -> Vec<Operacion> {
    nodos_base()
        .into_iter()
        .map(Operacion::PutNode)
        .chain(aristas_base().into_iter().map(Operacion::PutEdge))
        .collect()
}

/// Dani, el nodo que la transacción WAL añade al grafo mini.
fn dani() -> Node {
    Node::new(5, "Person").with_prop("name", Value::String("Dani".into()))
}

/// Extrae la columna de strings de un `ResultSet` (orden multiconjunto: sin
/// ORDER BY, el orden no es parte del contrato del cap. 21).
fn columna_de_strings(rs: &vol2_liradb::ResultSet, nombre: &str) -> Vec<String> {
    let col = rs.column(nombre).unwrap_or_else(|| {
        panic!(
            "el resultado no trae la columna '{nombre}': {:?}",
            rs.columns
        )
    });
    let mut valores = Vec::new();
    for fila in &rs.rows {
        match &fila[col] {
            Cell::Scalar(Value::String(s)) => valores.push(s.clone()),
            otra => panic!("celda inesperada en '{nombre}': {otra:?}"),
        }
    }
    valores.sort();
    valores
}

/// TESIS 1 del capítulo: la torre completa se construye en una respiración.
///
/// Recorrido por el hexágono (cap. 8) usando SOLO la API pública plana:
/// 1. DOMINIO (cap. 7) + PUERTO (cap. 8): `MemoryStore` detrás de `&dyn GraphStore`.
/// 2. PIPELINE transversal (caps. 17-21): texto → `parse` → `lower` →
///    `Catalog::collect` + `optimize` → `Executor` → `ResultSet`.
/// 3. FIABILIDAD (cap. 28): `WalTransaccion::commit` escribe WAL antes que
///    datos (delta de bytes medible) y aplica al store.
/// 4. RECUPERACIÓN (cap. 29): `guardar_wal` a tempfile + `reabrir` reconstruye
///    el estado íntegro en un store fresco.
/// 5. INVARIANTES (cap. 33): el estado recuperado pasa la torre de pruebas.
/// 6. MVCC (cap. 30): snapshot que NO ve el commit posterior (aislamiento).
/// 7. OBSERVABILIDAD (cap. 35): `Contadores` alimentado por derivación desde
///    `ExecMetrics` del executor + bytes de WAL + commits contados.
///
/// Si esto compila y pasa, el mapa del capítulo cuadra con `lib.rs`: cada
/// anillo existe, se enchufa al siguiente y ninguno conoció al otro.
#[test]
fn la_torre_completa_se_construye_en_una_respiracion() {
    // ── Anillo DOMINIO + PUERTO (caps. 7-8): el centro compila solo y el
    //    store vive DETRÁS del trait. Todo lo demás hablará con `&dyn`.
    let mut store = MemoryStore::new();
    let mut wal = Wal::new();
    let contadores = Contadores::new();

    // ── FIABILIDAD (cap. 28), parte 1: TODA escritura pasa por el WAL (la
    //    regla de oro, aplicada al grafo entero). El delta de bytes del log
    //    es observable (lo que el `--profile` del cap. 35 reportará como
    //    `wal_bytes_written`).
    let dir = tempdir().expect("tempfile disponible");
    let ruta_wal = dir.path().join("liradb.wal");
    let bytes_antes = wal.as_bytes().len();
    {
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        for nodo in nodos_base() {
            tx.put_node(nodo).expect("nodo nuevo en store vacío");
        }
        for arista in aristas_base() {
            tx.put_edge(arista).expect("extremos ya en el buffer");
        }
        tx.commit().expect("commit durable de la base");
    }
    contadores.contar_commit();

    // ── PIPELINE transversal (caps. 17-21): una máquina montada SOBRE el
    //    puerto GraphStore, no una capa del hexágono. Cada etapa explícita
    //    para narrar el flujo vertical del brief: parse → lower → optimize
    //    → execute.
    let metricas_exec = {
        let ast = parse(SRC_CONSULTA).expect("LiraQL válida");
        let plan_logico = lower(&ast).expect("el AST baja a plan lógico");
        let catalogo = Catalog::collect(&store);
        let plan_optimo = optimize(&plan_logico, &catalogo);
        let mut ejecutor = Executor::new(&plan_optimo, &store).expect("plan compilable");
        let rs = ejecutor.execute().expect("ejecución sin errores");
        assert_eq!(
            columna_de_strings(&rs, "f.name"),
            vec!["Bob".to_string(), "Carla".to_string()]
        );
        let metricas = ejecutor.metrics();
        assert_eq!(metricas.rows_returned, 2);
        assert!(!metricas.per_operator.is_empty());
        metricas
    };

    // ── OBSERVABILIDAD (cap. 35): el registro se alimenta POR DERIVACIÓN de
    //    las métricas que ya produjo el executor — una sola verdad.
    derivar_contadores(&metricas_exec, &contadores);
    contadores.incrementar_queries_total(1);

    // ── FIABILIDAD (cap. 28), parte 2: el delta transaccional.
    {
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(dani()).expect("staging válido");
        tx.put_edge(Edge::new(5, 5, 0, "KNOWS").with_prop("peso", Value::Float(4.5)))
            .expect("extremo 5 añadido antes en el buffer");
        let resumen = tx.commit().expect("commit durable del delta");
        assert!(resumen.lsn_commit > 0);
    }
    contadores.contar_commit();
    let delta_bytes = wal.as_bytes().len() - bytes_antes;
    assert!(delta_bytes > 0, "los commits deben dejar huella en el log");
    contadores.sumar_wal_bytes(delta_bytes);
    assert_eq!(store.node_count(), 6);
    assert_eq!(store.edge_count(), 6);

    // ── RECUPERACIÓN (cap. 29): corte de luz simulado. Un store FRESCO
    //    renace leyendo SOLO el log persistido en tempfile — y como TODO
    //    pasó por el WAL, renace COMPLETO.
    guardar_wal(&wal, &ruta_wal).expect("log persistido");
    let mut renacido = MemoryStore::new();
    let informe =
        reabrir(&mut renacido, &ruta_wal, &AntesImagenes::new()).expect("recuperación completa");
    assert_eq!(informe.transacciones_ganadoras, 2);
    assert_eq!(renacido.node_count(), 6);
    assert_eq!(renacido.edge_count(), 6);

    // ── CALIDAD como lente (cap. 33): el estado recuperado no solo tiene
    //    los contadores bien — cumple los invariantes estructurales.
    verificar_invariantes(&renacido).expect("estado íntegro tras replay");
    verificar_invariantes(&store).expect("estado íntegro tras commit");

    // ── MVCC (cap. 30): el mismo grafo mini como lote transaccional. Un
    //    lector ancla su snapshot ANTES del segundo commit y NO ve lo nuevo:
    //    lecturas que no bloquean al escritor, por construcción.
    let mut mvcc = MvccStore::new();
    let base = mvcc.commit(&ops_grafo_base()).expect("lote base válido");
    // El snapshot del lector es el ts ASIGNADO al commit base (el reloj es
    // pre-incremento y arranca en 1: `reloj()` a secas devolvería el ts que
    // el SIGUIENTE commit va a tomar).
    let ts_lectura = base.ts_asignado;
    let delta_mvcc = mvcc
        .commit(&[Operacion::PutNode(dani())])
        .expect("sobreescribir/appendizar es legal en MVCC");
    assert!(delta_mvcc.ts_asignado > ts_lectura);
    assert_eq!(mvcc.iter_nodos(ts_lectura).len(), 5, "snapshot estable");
    assert_eq!(
        mvcc.iter_nodos(mvcc.reloj()).len(),
        6,
        "estado actual creció"
    );

    // ── Cierre del recibo (cap. 35): la derivación fue COHERENTE con lo que
    //    el executor midió, y la capa conductora contó lo suyo.
    let (nodos, aristas, indices) = metricas_consulta(&metricas_exec);
    let recibo = contadores.snapshot();
    assert_eq!(recibo.queries_total, 1);
    assert_eq!(recibo.nodes_scanned, nodos);
    assert_eq!(recibo.relationships_expanded, aristas);
    assert_eq!(recibo.index_hits, indices);
    assert!(recibo.nodes_scanned + recibo.index_hits >= 1);
    assert_eq!(recibo.relationships_expanded, 2, "Ana conoce a Bob y Carla");
    assert_eq!(recibo.wal_bytes_written, delta_bytes as u64);
    assert_eq!(recibo.transactions_committed, 2);
    assert_eq!(recibo.transactions_aborted, 0);
}

/// TESIS 2 del capítulo: el anillo FÍSICO encadena sus adaptadores.
///
/// Capa ADAPTADORES-almacenamiento del hexágono: nada aquí sabe nada de
/// LiraQL ni del executor. La cadena es estrictamente hacia abajo:
/// - PUERTO `Pager` (cap. 12) ← adaptador `FilePager` sobre tempfile;
/// - adaptador `BufferPool` (cap. 13) ENFRENTE del puerto (dueño del pager);
/// - adaptador `PersistentCsr` (cap. 14): CSR de RAM → páginas → RAM;
/// - adaptadores `HashIndex`/`BPlusTree` (cap. 15): cada estructura dueña de
///   SU pool sobre SU fichero (la propiedad resuelve la convivencia).
///
/// La reapertura final demuestra durabilidad física de punta a punta: lo
/// escrito sobrevive al drop del pool y del pager.
#[test]
fn la_pila_fisica_encadena_pager_pool_csr_e_indices() {
    let dir = tempdir().expect("tempfile disponible");

    // ── Pager → BufferPool → PersistentCsr: replace/load redondo.
    let pager_csr = FilePager::create(dir.path().join("csr.bin")).expect("pager creado");
    let pool_csr = BufferPool::with_policy(pager_csr, 16, PolicyKind::Clock);
    let csr_ram = Csr::from_edges([(0, 1), (1, 2), (0, 2), (2, 0)]).expect("CSR válido");
    let mut persistente = PersistentCsr::create(pool_csr).expect("cabecera escrita");
    persistente.replace(&csr_ram).expect("CSR paginado");
    let recargado = persistente.load().expect("CSR leído de páginas");
    csr_ram.verify().expect("original íntegro");
    recargado.verify().expect("recargado íntegro");
    for u in 0..=2usize {
        assert_eq!(
            recargado.neighbors_out(u),
            csr_ram.neighbors_out(u),
            "adyacencia out preservada para {u}"
        );
        assert_eq!(recargado.degree_in(u), csr_ram.degree_in(u));
    }

    // ── HashIndex sobre SU propio pool (LRU aquí: la política es del pool,
    //    invisible al índice — ese es el punto del puerto BufferPool).
    //
    //    HALLAZGO del smoke (la prosa lo cita): `HashIndex::create` hace
    //    flush de TODAS sus páginas y `BufferPool::flush_page` exige
    //    residencia — si el working set (3 + num_buckets) supera la
    //    capacidad del pool, la creación falla con `UnknownPage`. El
    //    adaptador de índice depende, sin decirlo, del tamaño del pool
    //    que lo aloja: capacidad 16 para 8 cubos.
    let pool_hash = BufferPool::with_policy(
        FilePager::create(dir.path().join("hash.bin")).expect("pager hash"),
        16,
        PolicyKind::Lru,
    );
    let mut indice = HashIndex::create(pool_hash, 8).expect("catálogo hash escrito");
    assert!(indice.is_empty());
    indice.insert(42, 7).expect("insert 42");
    indice.insert(43, 9).expect("insert 43");
    indice.insert(99, 1).expect("insert 99");
    assert_eq!(indice.get(42).expect("lectura hash"), Some(7));
    assert_eq!(indice.get(7).expect("clave ausente"), None);
    indice.flush().expect("hash durable");

    // Reapertura completa (nuevo pager + nuevo pool): el catálogo manda.
    let mut indice_reabierto = HashIndex::open(BufferPool::new(
        FilePager::open(dir.path().join("hash.bin")).expect("pager hash reabierto"),
        8,
    ))
    .expect("catálogo hash leído");
    assert_eq!(indice_reabierto.get(99).expect("persistió"), Some(1));

    // ── BPlusTree: tercer pool (Clock), inserciones ordenadas y range_scan.
    //    Capacidad holgada por el mismo hallazgo de residencia que el hash.
    let pool_arbol = BufferPool::with_policy(
        FilePager::create(dir.path().join("bptree.bin")).expect("pager árbol"),
        32,
        PolicyKind::Clock,
    );
    let mut arbol = BPlusTree::create(pool_arbol).expect("raíz escrita");
    for i in 0..12i64 {
        let clave = (i * 10) as u64;
        let _insertado = arbol.insert(clave, i as u64).expect("insert ordenado");
    }
    assert_eq!(arbol.get(50), Some(5));
    assert_eq!(arbol.get(55), None);
    let rango = arbol.range_scan(20, 60);
    assert_eq!(rango.len(), 5);
    assert_eq!(rango.first().map(|e| e.key), Some(20));
    assert_eq!(rango.last().map(|e| e.key), Some(60));
    arbol.flush().expect("árbol durable");

    let arbol_reabierto = BPlusTree::open(BufferPool::new(
        FilePager::open(dir.path().join("bptree.bin")).expect("pager árbol reabierto"),
        16,
    ))
    .expect("raíz leída");
    assert_eq!(arbol_reabierto.get(110), Some(11));
}

/// Test de INVENTARIO (contrato §2): cada uno de los CINCO puertos existe y
/// su adaptador canónico se instancia TRAS él — aserciones de tipos y uso
/// real, no reflexión. Fallaría AL COMPILAR si un trait renombrara o
/// desapareciera; fallaría EN EJECUCIÓN si el adaptador dejara de cumplirlo.
///
/// Anillo PUERTOS del hexágono con sus adaptadores:
/// - `GraphStore` (cap. 8) ↔ `MemoryStore`, usado como `&dyn`;
/// - `Pager` (cap. 12) ↔ `FilePager`, usado como `Box<dyn>`;
/// - `PhysicalOperator` (cap. 20) ↔ `NodeScanOp`, operador real del `Executor`;
/// - `WeightSource` (cap. 22) ↔ `dijkstra` (Property y Constant);
/// - `Heuristic` (cap. 23) ↔ `ZeroHeuristic` y `EuclideanHeuristic` en `a_star`.
#[test]
fn cada_puerto_tiene_su_adaptador_canonico() {
    let store = grafo_base_store();

    // ── GraphStore ↔ MemoryStore en posición de trait object.
    let puerto_datos: &dyn GraphStore = &store;
    assert_eq!(puerto_datos.node_count(), 5);
    assert_eq!(puerto_datos.edge_count(), 5);

    // ── Pager ↔ FilePager en posición de trait object.
    let dir = tempdir().expect("tempfile disponible");
    let mut pager_puerto: Box<dyn Pager> =
        Box::new(FilePager::create(dir.path().join("puerto.bin")).expect("pager creado"));
    let pagina = pager_puerto.allocate().expect("página asignada");
    pager_puerto
        .write(pagina, &[0u8; PAGE_SIZE])
        .expect("escritura");
    pager_puerto.sync().expect("sync");

    // ── PhysicalOperator ↔ NodeScanOp: el mismo tipo que `compile` (cap. 20)
    //    coloca bajo el `Executor`, movido a mano por el ciclo Volcano.
    let mut escaneo = NodeScanOp::new(puerto_datos, "p".to_string(), Some("Person".into()));
    escaneo.open().expect("cursor abierto");
    let mut filas = 0;
    while escaneo.next().expect("fila").is_some() {
        filas += 1;
    }
    escaneo.close().expect("cierre idempotente");
    assert_eq!(filas, 3, "tres Persons en el grafo mini");
    assert_eq!(escaneo.name(), "NodeScan");
    assert_eq!(escaneo.rows_produced(), 3);

    // ── WeightSource ↔ dijkstra: el usuario aporta el conocimiento del
    //    dominio (propiedad de arista) o delega (constante = saltos).
    let pesos = WeightSource::property("peso");
    let sp_ponderado = dijkstra(puerto_datos, 0, &pesos).expect("pesos válidos");
    assert_eq!(sp_ponderado.distance(2), Some(6.0), "0→1→2 gana al directo");
    let sp_saltos = dijkstra(puerto_datos, 0, &WeightSource::Constant(1.0))
        .expect("pesos constantes siempre válidos");
    // Con pesos constantes gana el MENOS SALTOS: la arista directa Ana→Carla
    // (peso real 10.5) vence al camino 0→1→2 — la lección del cap. 22 sobre
    // lo que cambia cuando el USUARIO elige la fuente de pesos.
    assert_eq!(sp_saltos.distance(2), Some(1.0));

    // ── Heuristic ↔ a_star: h≡0 degenera EXACTAMENTE en Dijkstra (mismo
    //    coste) y la euclídea —admisible con estas coordenadas— encuentra
    //    el MISMO camino óptimo.
    let camino_dijkstra = sp_ponderado.path_to(2).expect("camino conocido");
    let camino_cero = a_star(puerto_datos, 0, 2, &pesos, &ZeroHeuristic)
        .expect("búsqueda válida")
        .expect("destino alcanzable");
    assert_eq!(camino_cero.cost, camino_dijkstra.cost);
    assert_eq!(camino_cero.nodes(), camino_dijkstra.nodes());
    let euclidea = EuclideanHeuristic::new(puerto_datos, 2, "x", "y")
        .expect("coordenadas del destino validadas");
    let camino_euclideo = a_star(puerto_datos, 0, 2, &pesos, &euclidea)
        .expect("búsqueda válida")
        .expect("destino alcanzable");
    assert_eq!(camino_euclideo.nodes(), vec![0, 1, 2]);
    assert_eq!(camino_euclideo.cost, 6.0);
}

/// El puerto COMPARTIDO (caps. 22-26 sobre el cap. 8): la MISMA instancia
/// detrás de `&dyn GraphStore` alimenta al pipeline OLTP (`run`, caps. 18-20)
/// y a los algoritmos OLAP (`dijkstra`, cap. 22; `page_rank`, cap. 24) sin
/// copias ni conversiones. Esa es la decisión estructural nº 1 del capítulo:
/// algoritmos enchufados al puerto, jamás al adaptador concreto.
#[test]
fn los_algoritmos_corren_sobre_el_mismo_puerto_que_la_consulta() {
    let store = grafo_base_store();
    let puerto: &dyn GraphStore = &store;

    // OLTP: el atajo público del cap. 20 (parse → lower → optimize → execute).
    let rs = run("MATCH (c:City) RETURN c.name", puerto).expect("consulta simple");
    assert_eq!(rs.len(), 2);
    assert_eq!(
        columna_de_strings(&rs, "c.name"),
        vec!["Lisboa".to_string(), "Madrid".to_string()]
    );

    // OLAP sobre EL MISMO objeto: distancias ponderadas…
    let pesos = WeightSource::property("peso");
    let sp = dijkstra(puerto, 0, &pesos).expect("pesos válidos");
    assert_eq!(sp.distance(3), Some(4.0), "Ana→Bob→Madrid: 2.5 + 1.5");
    assert_eq!(sp.distance(4), Some(7.5), "Ana→Bob→Carla→Lisboa");

    // …y ranking global: masa conservada y estructura de votos — Carla
    // recibe DOS votos in (Ana y Bob) frente al UNO de Madrid; Bob recibe
    // el voto de Ana mientras Ana no recibe ninguno.
    let pr = page_rank(puerto, 0.85, 200, 1e-9).expect("parámetros válidos");
    assert!((pr.total_mass() - 1.0).abs() < 1e-9, "masa ≈ 1");
    assert!(pr.score(2).unwrap() > pr.score(3).unwrap());
    assert!(pr.score(1).unwrap() > pr.score(0).unwrap());

    // Una sola fuente de verdad: ambos mundos cuentan lo mismo.
    assert_eq!(puerto.node_count(), 5);
}

/// FRONTERA E/S (cap. 32) sobre el puerto (cap. 8): exportar TODO el store a
/// JSONL, importarlo a un store FRESCO y comparar elemento a elemento. El
/// formato es el contrato: si la igualdad exacta falla, el formato mintió.
#[test]
fn el_formato_jsonl_sobrevive_al_roundtrip_del_puerto() {
    let original = grafo_base_store();

    let mut jsonl = Vec::new();
    exportar_jsonl(&original, &mut jsonl).expect("exportación streaming");

    let mut fresco = MemoryStore::new();
    let stats = importar_jsonl(&mut BufReader::new(jsonl.as_slice()), &mut fresco)
        .expect("importación streaming");
    assert_eq!(stats.nodos, 5);
    assert_eq!(stats.aristas, 5);

    // Igualdad EXACTA elemento a elemento (orden por id para determinismo):
    // ids, labels, props y tipos de valor incluidos.
    let mut nodos_originales: Vec<Node> = original.iter_nodes().cloned().collect();
    nodos_originales.sort_by_key(|n| n.id);
    let mut nodos_frescos: Vec<Node> = fresco.iter_nodes().cloned().collect();
    nodos_frescos.sort_by_key(|n| n.id);
    assert_eq!(
        nodos_frescos, nodos_originales,
        "nodos idénticos tras ida y vuelta"
    );

    let mut aristas_originales: Vec<Edge> = original.iter_edges().cloned().collect();
    aristas_originales.sort_by_key(|e| e.id);
    let mut aristas_frescas: Vec<Edge> = fresco.iter_edges().cloned().collect();
    aristas_frescas.sort_by_key(|e| e.id);
    assert_eq!(
        aristas_frescas, aristas_originales,
        "aristas idénticas tras ida y vuelta"
    );
}
