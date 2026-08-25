// ─────────────────── Cap 35: Observabilidad interna ───────────────────
//
// El cap. 34 enseñó a MEDIR («¿cuánto tarda?»); este módulo es la pieza
// std que hace visible el TRABAJO del motor: contadores agregados (el
// RECIBO) y los dos decoradores medidores que generalizan `ContandoStore`
// (cap. 26) al resto de puertos. La otra mitad del capítulo — los SPANS
// (el ITINERARIO) y el subscriber que reconstruye el árbol — vive en la
// CLI (`liradb-cli/src/observabilidad.rs`): ahí es donde los spans se
// EMITEN en producción, y por eso `tracing` es dependency de la CLI y no
// de aquí. Este crate permanece dependency-free, como 28 capítulos antes.
//
//   * [`Contadores`] — registro A MANO con campos fijos nombrados EXACTO
//     igual que las métricas del brief (`queries_total`, `nodes_scanned`,
//     …), mutabilidad interior `Cell<u64>` (el patrón de `ContandoStore`:
//     compartir `&Contadores` sin `mut`), `snapshot()` inmutable y
//     `Display` que imita el TEXT FORMAT de Prometheus
//     (prometheus.io/docs/instrumenting/exposition_formats). Campos fijos =
//     typo imposible en compilación y orden determinista; el crate
//     `metrics` (facade con recorder GLOBAL) se documenta como equivalente
//     industrial y NO entra — el lector debe ver dónde vive el estado.
//
//   * [`MedidorOperador`] / [`MedidorPaginas`] — DECORADORES sobre los
//     traits ya existentes (`PhysicalOperator` cap. 20, `Pager` cap. 12).
//     El motor no sabe que lo observan; apilar decoradores compone vistas
//     sin acoplarse (GoF Decorator con precedente interno: cap. 26).
//
//   * DERIVACIÓN, no duplicación: `nodes_scanned`/`relationships_expanded`/
//     `index_hits` se SUMAN desde `ExecMetrics.per_operator` POR NOMBRE
//     canónico — UNA sola verdad: los números del `--profile`, del
//     `--stats` (cap. 31) y del `explain` (cap. 21) coinciden por
//     construcción. Duplicar contadores paralelos sería el bug clásico de
//     telemetría: dos fuentes que divergen en silencio.
//
// HONESTIDAD de fiabilidad (verificado en caps. 27/28, NO se tocan):
//   - `Wal` no expone contador de bytes, pero sí el total vía
//     `Wal::as_bytes()`: `wal_bytes_written` se calcula POR DELTA
//     antes/después del commit.
//   - `Transaccion::commit(self)`/`rollback(self)` CONSUMEN self sin
//     contar nada: envolverlos transparentemente exigiría tocar caps.
//     publicados o hacks de `Drop` que confunden ownership. Los contadores
//     `transactions_committed`/`aborted` viven en la CAPA CONDUCTORA (el
//     patrón del REPL: quien llama a commit/rollback cuenta el resultado).
//
// FRONTERA declarada (heredada de caps. 33/34): las consultas corren hoy
// sobre `MemoryStore`, así que `page_reads`/`page_writes` NO aparecen en la
// ruta de consulta — se demuestran a nivel COMPONENTE (índice sobre
// `BufferPool<FilePager>`, test en la CLI). Fingir page fetches sería la
// falsedad exacta que este libro prohíbe.

use std::cell::Cell;
use std::fmt;
use std::time::Instant;

use crate::cap12_pager::{PageId, Pager, PagerError};
use crate::cap20_volcano::{ExecError, ExecMetrics, PhysicalOperator, Row};

// ─── SnapshotContadores: la foto inmutable ───

/// Foto INMUTABLE y copiable del registro de contadores.
///
/// Es lo que se imprime, guarda o compara: un struct plano de `u64` sin
/// mutabilidad interior, para que nadie pueda modificar el registro a
/// través de su snapshot ni arrastrar préstamos. Campos públicos a
/// propósito — ningún consumidor necesita métodos para leer un número.
///
/// El test-tesis del campo fijo: añadir una métrica nueva obliga a tocar
/// este struct, el `Display` y los incrementos — tres puntos visibles en
/// el diff, frente al `HashMap` donde una clave mal escrita compila y
/// miente en runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SnapshotContadores {
    /// Consultas ejecutadas (1 por pasada del pipeline).
    pub queries_total: u64,
    /// Filas producidas por operadores `NodeScan` (Σ de `per_operator`).
    pub nodes_scanned: u64,
    /// Filas producidas por operadores `Expand` (Σ de `per_operator`).
    pub relationships_expanded: u64,
    /// Filas producidas por operadores `IndexSeek` (Σ de `per_operator`).
    pub index_hits: u64,
    /// Lecturas de página del pager envuelto (`read()` OK).
    pub page_reads: u64,
    /// Escrituras de página del pager envuelto (`write()` OK).
    pub page_writes: u64,
    /// Bytes escritos al WAL, POR DELTA de `Wal::as_bytes().len()`.
    pub wal_bytes_written: u64,
    /// Transacciones confirmadas, contadas en la capa conductora.
    pub transactions_committed: u64,
    /// Transacciones abortadas (rollback), contadas en la capa conductora.
    pub transactions_aborted: u64,
}

// ─── Contadores: el registro (la mitad «recibo» del capítulo) ───

/// Registro de contadores del motor: campos fijos + `Cell<u64>` + Display
/// estilo Prometheus.
///
/// Por qué campos FIJOS y no `HashMap<&'static str, u64>`: (1) el typo en
/// una clave de mapa compila y falla en runtime — aquí el compilador es el
/// guardián; (2) el orden del `Display` es el de declaración, requisito
/// del text format determinista; (3) cero hashing en el camino caliente.
/// Por qué `Cell` y no `AtomicU64`: el motor es monohilo (lo garantiza el
/// borrow checker desde el cap. 27) y la honestidad del capítulo incluye
/// NO fingir concurrencia — migrar a atómicos es mecánico si algún día se
/// necesitan. Se pasa por referencia compartida (`&Contadores`) a todos
/// los medidores decorador: el mismo préstamo que hizo famoso a
/// `ContandoStore` (cap. 26).
#[derive(Debug, Default)]
pub struct Contadores {
    queries_total: Cell<u64>,
    nodes_scanned: Cell<u64>,
    relationships_expanded: Cell<u64>,
    index_hits: Cell<u64>,
    page_reads: Cell<u64>,
    page_writes: Cell<u64>,
    wal_bytes_written: Cell<u64>,
    transactions_committed: Cell<u64>,
    transactions_aborted: Cell<u64>,
}

impl Contadores {
    /// Registro a cero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cuenta una consulta ejecutada (o `n`, si se agrupan).
    pub fn incrementar_queries_total(&self, n: u64) {
        self.queries_total.set(self.queries_total.get() + n);
    }

    /// Acumula filas escaneadas (`NodeScan`).
    pub fn incrementar_nodes_scanned(&self, n: u64) {
        self.nodes_scanned.set(self.nodes_scanned.get() + n);
    }

    /// Acumula filas expandidas (`Expand`).
    pub fn incrementar_relationships_expanded(&self, n: u64) {
        self.relationships_expanded
            .set(self.relationships_expanded.get() + n);
    }

    /// Acumula filas traídas por índice (`IndexSeek`).
    pub fn incrementar_index_hits(&self, n: u64) {
        self.index_hits.set(self.index_hits.get() + n);
    }

    /// Acumula lecturas de página OK del pager medido.
    pub fn incrementar_page_reads(&self, n: u64) {
        self.page_reads.set(self.page_reads.get() + n);
    }

    /// Acumula escrituras de página OK del pager medido.
    pub fn incrementar_page_writes(&self, n: u64) {
        self.page_writes.set(self.page_writes.get() + n);
    }

    /// Suma el DELTA de bytes del WAL (`Wal::as_bytes().len()` después
    /// menos antes). `usize` porque así devuelve `.len()`; internamente
    /// todo es `u64`.
    pub fn sumar_wal_bytes(&self, delta: usize) {
        self.wal_bytes_written
            .set(self.wal_bytes_written.get() + delta as u64);
    }

    /// Cuenta un commit — LO LLAMA la capa conductora tras consumir la
    /// transacción con éxito (caps. 27/28 no cuentan nada por sí mismos).
    pub fn contar_commit(&self) {
        self.transactions_committed
            .set(self.transactions_committed.get() + 1);
    }

    /// Cuenta un rollback/abort — mismo contrato de capa conductora.
    pub fn contar_rollback(&self) {
        self.transactions_aborted
            .set(self.transactions_aborted.get() + 1);
    }

    /// Foto inmutable del estado actual (lo que se imprime/compara).
    pub fn snapshot(&self) -> SnapshotContadores {
        SnapshotContadores {
            queries_total: self.queries_total.get(),
            nodes_scanned: self.nodes_scanned.get(),
            relationships_expanded: self.relationships_expanded.get(),
            index_hits: self.index_hits.get(),
            page_reads: self.page_reads.get(),
            page_writes: self.page_writes.get(),
            wal_bytes_written: self.wal_bytes_written.get(),
            transactions_committed: self.transactions_committed.get(),
            transactions_aborted: self.transactions_aborted.get(),
        }
    }
}

impl fmt::Display for Contadores {
    /// Imita el TEXT FORMAT de Prometheus: una línea `# TYPE <nombre>
    /// counter` seguida de `<nombre> <valor>` por métrica, en ORDEN FIJO
    /// de declaración. Se imita (no se depende): aprender el formato de
    /// exposición escribiéndolo a mano enseña qué es — pares nombre/valor
    /// planos, legibles sin más — sin pagar exporter ni dependency. Un
    /// backend real (N.10) parsearía esto tal cual.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.snapshot();
        for (nombre, valor) in [
            ("queries_total", s.queries_total),
            ("nodes_scanned", s.nodes_scanned),
            ("relationships_expanded", s.relationships_expanded),
            ("index_hits", s.index_hits),
            ("page_reads", s.page_reads),
            ("page_writes", s.page_writes),
            ("wal_bytes_written", s.wal_bytes_written),
            ("transactions_committed", s.transactions_committed),
            ("transactions_aborted", s.transactions_aborted),
        ] {
            writeln!(f, "# TYPE {nombre} counter")?;
            writeln!(f, "{nombre} {valor}")?;
        }
        Ok(())
    }
}

// ─── Derivación: UNA sola verdad desde ExecMetrics ───

/// A qué contador alimenta cada nombre canónico de operador (cap. 20).
///
/// Función PRIVADA compartida por la derivación ([`metricas_consulta`] y
/// [`derivar_contadores`]) y por [`MedidorOperador`]: si mañana un nombre
/// cambia hay UN punto de verdad que actualizar — y el test de
/// consistencia contra `ExecMetrics` directo grita si algo se rompe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampoOperador {
    NodesScanned,
    RelationshipsExpanded,
    IndexHits,
    /// Operadores sin métrica propia en el recibo (`Filter`, `Project`,
    /// `CartesianProduct`, `Limit`, `Distinct`): sus filas quedan en los
    /// contadores locales del medidor, no en el registro global.
    Ninguno,
}

fn campo_de_operador(nombre: &str) -> CampoOperador {
    // Literales EXACTOS de los `name()` del cap. 20 (verificados):
    // cambiar uno allí rompe ESTE match en tests, nunca en silencio.
    match nombre {
        "NodeScan" => CampoOperador::NodesScanned,
        "Expand" => CampoOperador::RelationshipsExpanded,
        "IndexSeek" => CampoOperador::IndexHits,
        _ => CampoOperador::Ninguno,
    }
}

/// Deriva `(nodes_scanned, relationships_expanded, index_hits)` sumando
/// `ExecMetrics.per_operator` POR NOMBRE canónico.
///
/// Función PURA sobre datos que ya existen: los números del `--profile`,
/// del `--stats` y del `explain` coinciden POR CONSTRUCCIÓN porque todos
/// salen del mismo `collect_metrics()` del cap. 20. Devuelve tupla (y no
/// struct) para calcar la firma fijada en el contrato del capítulo.
pub fn metricas_consulta(metricas: &ExecMetrics) -> (u64, u64, u64) {
    let mut nodos = 0;
    let mut aristas = 0;
    let mut indices = 0;
    for (nombre, filas) in &metricas.per_operator {
        match campo_de_operador(nombre) {
            CampoOperador::NodesScanned => nodos += filas,
            CampoOperador::RelationshipsExpanded => aristas += filas,
            CampoOperador::IndexHits => indices += filas,
            CampoOperador::Ninguno => {}
        }
    }
    (nodos, aristas, indices)
}

/// Vuelca la derivación de `ExecMetrics` DIRECTAMENTE en el registro.
///
/// Atajo del hito `--profile`: tras ejecutar, el pipeline llama esto y el
/// `Display` del registro ya muestra el recibo completo. Rellena los TRES
/// contadores derivados; `queries_total` lo suma aparte quien conduzca
/// (es 1 por consulta, algo que las métricas del executor no saben).
pub fn derivar_contadores(metricas: &ExecMetrics, contadores: &Contadores) {
    let (nodos, aristas, indices) = metricas_consulta(metricas);
    contadores.incrementar_nodes_scanned(nodos);
    contadores.incrementar_relationships_expanded(aristas);
    contadores.incrementar_index_hits(indices);
}

// ─── MedidorOperador: el voltímetro del Volcano ───

/// Decorador que envuelve cualquier `PhysicalOperator` y acumula trabajo
/// en un [`Contadores`] compartido — la GENERALIZACIÓN de `ContandoStore`
/// (cap. 26) al puerto de ejecución del cap. 20.
///
/// Qué cuenta y dónde:
/// * llamadas a `next()`, filas vistas y TIEMPO dentro de `next()` →
///   contadores LOCALES propios (getters), porque son del operador
///   concreto, no del motor entero;
/// * filas producidas según el NOMBRE canónico del operador envuelto →
///   el registro COMPARTIDO (`NodeScan`→`nodes_scanned`, etc.), para que
///   un árbol de medidores produzca el mismo recibo que la derivación de
///   [`ExecMetrics`].
///
/// Delega open/next/close/name/rows_produced/collect_metrics sin tocar
/// nada: el operador real no sabe que lo miden (transparencia decorator).
pub struct MedidorOperador<'a> {
    inner: Box<dyn PhysicalOperator + 'a>,
    contadores: &'a Contadores,
    llamadas_next: Cell<u64>,
    filas_vistas: Cell<u64>,
    tiempo_ns: Cell<u64>,
}

impl fmt::Debug for MedidorOperador<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // El inner no exige Debug: informamos de lo que SÍ sabemos.
        f.debug_struct("MedidorOperador")
            .field("operador", &self.inner.name())
            .field("llamadas_next", &self.llamadas_next.get())
            .field("filas_vistas", &self.filas_vistas.get())
            .field("tiempo_ns", &self.tiempo_ns.get())
            .finish()
    }
}

impl<'a> MedidorOperador<'a> {
    /// Envuelve el operador a medir, acumulando en `contadores`.
    pub fn nuevo(inner: Box<dyn PhysicalOperator + 'a>, contadores: &'a Contadores) -> Self {
        Self {
            inner,
            contadores,
            llamadas_next: Cell::new(0),
            filas_vistas: Cell::new(0),
            tiempo_ns: Cell::new(0),
        }
    }

    /// Llamadas a `next()` acumuladas (no se resetean: es un contador).
    pub fn llamadas_next(&self) -> u64 {
        self.llamadas_next.get()
    }

    /// Filas que el operador envuelto entregó por `next()`.
    pub fn filas_vistas(&self) -> u64 {
        self.filas_vistas.get()
    }

    /// Tiempo acumulado DENTRO de `next()` del operador real, en ns.
    ///
    /// Incluye el coste de TODOS los hijos (el pull anida): el tiempo del
    /// `Project` contiene el del `Filter` debajo. Para atribuir costes por
    /// nivel está el árbol de spans de la CLI — cada vista, su pregunta.
    pub fn tiempo_total_ns(&self) -> u64 {
        self.tiempo_ns.get()
    }
}

impl PhysicalOperator for MedidorOperador<'_> {
    fn open(&mut self) -> Result<(), ExecError> {
        self.inner.open()
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        self.llamadas_next.set(self.llamadas_next.get() + 1);
        let t0 = Instant::now();
        let resultado = self.inner.next();
        self.tiempo_ns
            .set(self.tiempo_ns.get() + t0.elapsed().as_nanos() as u64);
        if let Ok(Some(_fila)) = &resultado {
            self.filas_vistas.set(self.filas_vistas.get() + 1);
            // Al registro compartido SOLO por nombre canónico: la misma
            // verdad que la derivación de ExecMetrics reportará después.
            match campo_de_operador(self.inner.name()) {
                CampoOperador::NodesScanned => self.contadores.incrementar_nodes_scanned(1),
                CampoOperador::RelationshipsExpanded => {
                    self.contadores.incrementar_relationships_expanded(1)
                }
                CampoOperador::IndexHits => self.contadores.incrementar_index_hits(1),
                CampoOperador::Ninguno => {}
            }
        }
        resultado
    }

    fn close(&mut self) -> Result<(), ExecError> {
        self.inner.close()
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn rows_produced(&self) -> u64 {
        self.inner.rows_produced()
    }

    fn collect_metrics(&self) -> Vec<(&'static str, u64)> {
        self.inner.collect_metrics()
    }
}

// ─── MedidorPaginas: el voltímetro del pager ───

/// Decorador `Pager` que cuenta reads/writes/syncs y BYTES movidos en un
/// [`Contadores`] compartido.
///
/// Bytes movidos = nº de operaciones × tamaño de página (el buffer de
/// `read`/`write` SIEMPRE mide exactamente una página — contrato del
/// trait, cap. 12): no hace falta mirar contenido, sólo multiplicar.
/// Syncs y bytes van a contadores locales (getters); reads/writes también
/// alimentan el registro (`page_reads`/`page_writes`), las métricas del
/// brief que el pool del cap. 13 reportará por COMPOSICIÓN, no por copia.
///
/// Componible: envuelve un `FilePager` directo o OTRO envoltorio (el
/// `PagerTrazado` de la CLI apila el span FUERA de medidores como éste).
pub struct MedidorPaginas<'a, P> {
    inner: P,
    contadores: &'a Contadores,
    syncs: Cell<u64>,
    bytes_leidos: Cell<u64>,
    bytes_escritos: Cell<u64>,
}

impl<P: Pager> MedidorPaginas<'_, P> {
    /// Envuelve el pager a medir, acumulando en `contadores`.
    ///
    /// La lifetime del préstamo va explícita aquí para LIGARLA a la vida
    /// del medidor devuelto (la elisión no puede inferirla sola cuando el
    /// struct que se devuelve la contiene).
    pub fn nuevo<'a>(inner: P, contadores: &'a Contadores) -> MedidorPaginas<'a, P> {
        MedidorPaginas {
            inner,
            contadores,
            syncs: Cell::new(0),
            bytes_leidos: Cell::new(0),
            bytes_escritos: Cell::new(0),
        }
    }

    /// Llamadas `sync()` OK acumuladas.
    pub fn syncs(&self) -> u64 {
        self.syncs.get()
    }

    /// Bytes LEÍDOS acumulados (reads × tamaño de página).
    pub fn bytes_leidos(&self) -> u64 {
        self.bytes_leidos.get()
    }

    /// Bytes ESCRITOS acumulados (writes × tamaño de página).
    pub fn bytes_escritos(&self) -> u64 {
        self.bytes_escritos.get()
    }

    /// Acceso al pager envuelto (para flush/cierre del dueño).
    pub fn pager(&self) -> &P {
        &self.inner
    }

    /// Acceso mutable al pager envuelto.
    pub fn pager_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Consume el medidor y devuelve el pager original.
    pub fn hacia_pager(self) -> P {
        self.inner
    }
}

impl<P: Pager> Pager for MedidorPaginas<'_, P> {
    fn allocate(&mut self) -> Result<PageId, PagerError> {
        self.inner.allocate()
    }

    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError> {
        let r = self.inner.read(id, page);
        if r.is_ok() {
            self.contadores.incrementar_page_reads(1);
            self.bytes_leidos
                .set(self.bytes_leidos.get() + page.len() as u64);
        }
        r
    }

    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError> {
        let r = self.inner.write(id, page);
        if r.is_ok() {
            self.contadores.incrementar_page_writes(1);
            self.bytes_escritos
                .set(self.bytes_escritos.get() + page.len() as u64);
        }
        r
    }

    fn sync(&mut self) -> Result<(), PagerError> {
        let r = self.inner.sync();
        if r.is_ok() {
            self.syncs.set(self.syncs.get() + 1);
        }
        r
    }

    fn num_pages(&self) -> u32 {
        self.inner.num_pages()
    }

    fn free(&mut self, id: PageId) -> Result<(), PagerError> {
        self.inner.free(id)
    }

    fn is_allocated(&self, id: PageId) -> bool {
        self.inner.is_allocated(id)
    }

    fn page_size(&self) -> usize {
        self.inner.page_size()
    }
}

// ══════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_cap35 {
    use super::*;
    use crate::MemoryStore;
    use crate::cap07_modelo::{Node, Value};
    use crate::cap08_graph_store::GraphStore;
    use crate::cap11_slotted_pages::PAGE_SIZE;
    use crate::cap12_pager::FilePager;
    use crate::cap13_buffer_pool::BufferPool;
    use crate::cap17_liraql_ast::CompareOp;
    use crate::cap19_plan_logico::{LogicalPlan, Projection, ScalarExpr};
    use crate::cap20_volcano::{Executor, FilterOp, NodeScanOp, ProjectOp};
    use crate::cap27_transacciones::Transaccion;
    use crate::cap28_wal::{PoliticaFlush, Wal, WalTransaccion};

    /// Predicado `p.age < 40` (el del Q2 del demo), construido una vez.
    fn predicado_q2() -> ScalarExpr {
        ScalarExpr::Compare {
            op: CompareOp::Lt,
            left: Box::new(ScalarExpr::Property {
                variable: "p".to_string(),
                property: "age".to_string(),
            }),
            right: Box::new(ScalarExpr::Literal(Value::Int(40))),
        }
    }

    /// Proyección `RETURN p.name` (la del Q2), construida una vez.
    fn proyeccion_nombre() -> Vec<Projection> {
        vec![Projection {
            expr: ScalarExpr::Property {
                variable: "p".to_string(),
                property: "name".to_string(),
            },
            alias: None,
        }]
    }

    /// Plan Q2 del demo a mano: Project(Filter(NodeScan)) — 4 Person
    /// escaneadas, 3 pasan el filtro.
    fn plan_q2_filtro() -> LogicalPlan {
        LogicalPlan::Project {
            input: Box::new(LogicalPlan::Filter {
                input: Box::new(LogicalPlan::NodeScan {
                    variable: "p".to_string(),
                    label: Some("Person".to_string()),
                }),
                predicate: predicado_q2(),
            }),
            items: proyeccion_nombre(),
        }
    }

    // ── El registro: snapshot inmutable y Display exacto ─────────────

    /// TESIS §2: el Display imita el text format de Prometheus con ORDEN
    /// fijo, y `snapshot()` es una FOTO: más incrementos después no
    /// cambian lo ya capturado.
    #[test]
    fn contadores_display_formato_texto_y_snapshot_exacto() {
        let c = Contadores::new();
        c.incrementar_queries_total(1);
        c.incrementar_nodes_scanned(4);
        c.incrementar_relationships_expanded(2);
        c.incrementar_index_hits(1);

        let esperado = "\
# TYPE queries_total counter\n\
queries_total 1\n\
# TYPE nodes_scanned counter\n\
nodes_scanned 4\n\
# TYPE relationships_expanded counter\n\
relationships_expanded 2\n\
# TYPE index_hits counter\n\
index_hits 1\n\
# TYPE page_reads counter\n\
page_reads 0\n\
# TYPE page_writes counter\n\
page_writes 0\n\
# TYPE wal_bytes_written counter\n\
wal_bytes_written 0\n\
# TYPE transactions_committed counter\n\
transactions_committed 0\n\
# TYPE transactions_aborted counter\n\
transactions_aborted 0\n";
        assert_eq!(c.to_string(), esperado);

        // La foto congela el estado…
        let foto = c.snapshot();
        assert_eq!(foto.queries_total, 1);
        assert_eq!(foto.nodes_scanned, 4);
        assert_eq!(foto.relationships_expanded, 2);
        assert_eq!(foto.index_hits, 1);

        // …y el registro sigue vivo DESPUÉS: la foto NO cambia.
        c.incrementar_nodes_scanned(10);
        c.sumar_wal_bytes(512);
        c.contar_commit();
        assert_eq!(foto.nodes_scanned, 4, "el snapshot debe ser inmutable");
        assert_eq!(
            c.snapshot().nodes_scanned,
            14,
            "el registro acumula sobre lo previo"
        );
        assert_eq!(c.snapshot().wal_bytes_written, 512);
        assert_eq!(c.snapshot().transactions_committed, 1);
    }

    /// TESIS §2: campos FIJOS, sin mapa — `SnapshotContadores` se
    /// construye campo a campo (compilar ES el test de nombres) y el
    /// orden del Display es el de declaración, no el de un hash.
    #[test]
    fn contadores_campos_fijos_sin_mapa_sin_typos() {
        // Si un campo cambia de nombre o tipo, esta construcción falla
        // EN COMPILACIÓN: el typo imposible que promete la decisión §5.1.
        let esperado = SnapshotContadores {
            queries_total: 9,
            nodes_scanned: 8,
            relationships_expanded: 7,
            index_hits: 6,
            page_reads: 5,
            page_writes: 4,
            wal_bytes_written: 3,
            transactions_committed: 2,
            transactions_aborted: 1,
        };
        assert_eq!(esperado.queries_total, 9);
        assert_eq!(esperado.transactions_aborted, 1);

        // Y Default da TODO a cero (un registro nuevo no mintió nada).
        assert_eq!(
            SnapshotContadores::default(),
            SnapshotContadores {
                queries_total: 0,
                nodes_scanned: 0,
                relationships_expanded: 0,
                index_hits: 0,
                page_reads: 0,
                page_writes: 0,
                wal_bytes_written: 0,
                transactions_committed: 0,
                transactions_aborted: 0,
            }
        );
    }

    // ── Derivación: UNA verdad contra ExecMetrics ────────────────────

    /// TESIS §2: la derivación por nombre canónico COINCIDE con
    /// `ExecMetrics` directo. Si el cap. 20 renombrara `NodeScan` (o
    /// cualquiera), la suma derivada bajaría a 0 y ESTE test rompe —
    /// vigilancia explícita contra la rotura silenciosa de telemetría.
    #[test]
    fn metricas_consulta_deriva_de_exec_metrics_por_nombre_canonico() {
        let store = crate::demo_graph();
        let plan = plan_q2_filtro();

        // (1) Vía Executor (el camino de producción del cap. 20).
        let mut executor = Executor::new(&plan, &store).unwrap();
        let rs = executor.execute().unwrap();
        let metricas = executor.metrics();
        assert_eq!(rs.len(), 3);

        let (nodos, aristas, indices) = metricas_consulta(&metricas);

        // (2) La MISMA suma hecha A MANO sobre per_operator: la verdad
        // contra la que se vigila la función pura.
        let nodos_manual: u64 = metricas
            .per_operator
            .iter()
            .filter(|(n, _)| *n == "NodeScan")
            .map(|(_, f)| f)
            .sum();
        assert_eq!(nodos, nodos_manual, "Σ NodeScan derivada ≠ manual");
        assert_eq!(nodos, 4, "el demo tiene 4 Person");
        assert_eq!((aristas, indices), (0, 0), "Q2 no expande ni seeka");

        // (3) Y `derivar_contadores` vuelca lo mismo al registro.
        let c = Contadores::new();
        c.incrementar_queries_total(1);
        derivar_contadores(&metricas, &c);
        let s = c.snapshot();
        assert_eq!(
            (s.nodes_scanned, s.relationships_expanded, s.index_hits),
            (4, 0, 0)
        );
        assert_eq!(s.queries_total, 1);
    }

    /// La derivación TAMBIÉN cuadra en la ruta relacional (Expand) y en
    /// la de índice (IndexSeek): las tres patas del recibo, desde la
    /// única fuente que existe.
    #[test]
    fn derivacion_cubre_expand_e_indexseek_del_pipeline_real() {
        let store = crate::demo_graph();

        // Expand: Q3 del demo (KNOWS produce 4 filas bajo el scan).
        let query = crate::parse("MATCH (p:Person)-[:KNOWS]->(f:Person) RETURN f.name").unwrap();
        let plan = query.lower().unwrap();
        let mut executor = Executor::new(&plan, &store).unwrap();
        executor.execute().unwrap();
        let (nodos, aristas, indices) = metricas_consulta(&executor.metrics());
        assert_eq!((nodos, aristas, indices), (4, 4, 0));

        // IndexSeek: el plan que la regla R4 del cap. 21 produce, montado
        // a mano (los ids llegan resueltos — decisión del cap. 20).
        let plan_seek = LogicalPlan::Project {
            input: Box::new(LogicalPlan::IndexSeek {
                variable: "p".to_string(),
                label: Some("Person".to_string()),
                property: "name".to_string(),
                value: Value::String("Ana".to_string()),
                ids: vec![0],
            }),
            items: vec![Projection {
                expr: ScalarExpr::Property {
                    variable: "p".to_string(),
                    property: "age".to_string(),
                },
                alias: None,
            }],
        };
        let mut executor = Executor::new(&plan_seek, &store).unwrap();
        let rs = executor.execute().unwrap();
        assert_eq!(rs.len(), 1, "sólo Ana se llama Ana");
        let (nodos, aristas, indices) = metricas_consulta(&executor.metrics());
        assert_eq!((nodos, aristas, indices), (0, 0, 1));
    }

    // ── MedidorOperador: el voltímetro del Volcano ───────────────────

    /// TESIS §2: delega TODO y además cuenta llamadas a next(), filas y
    /// tiempo; las filas del operador envuelto alimentan el registro
    /// compartido según su nombre canónico.
    #[test]
    fn medidor_operador_cuenta_llamadas_filas_y_tiempo() {
        let store = crate::demo_graph();
        let contadores = Contadores::new();

        let scan = Box::new(NodeScanOp::new(
            &store,
            "p".to_string(),
            Some("Person".to_string()),
        ));
        let mut medido = MedidorOperador::nuevo(scan, &contadores);
        assert_eq!(medido.name(), "NodeScan", "delega name");

        medido.open().unwrap();
        let mut filas = 0;
        while medido.next().unwrap().is_some() {
            filas += 1;
        }
        medido.close().unwrap();

        assert_eq!(filas, 4, "el demo tiene 4 Person");
        assert_eq!(medido.llamadas_next(), 5, "4 filas + 1 None final");
        assert_eq!(medido.filas_vistas(), 4);
        assert_eq!(medido.rows_produced(), 4, "delega rows_produced");
        assert!(
            medido.tiempo_total_ns() > 0,
            "el tiempo dentro de next() debe ser positivo"
        );

        // El registro compartido recibió las filas por nombre canónico —
        // la MISMA cifra que la derivación de ExecMetrics dará después.
        let s = contadores.snapshot();
        assert_eq!(s.nodes_scanned, 4);
        assert_eq!((s.relationships_expanded, s.index_hits), (0, 0));

        // collect_metrics delegado: pre-orden intacto (cap. 20).
        assert_eq!(
            medido.collect_metrics(),
            vec![("NodeScan", 4)],
            "delega collect_metrics"
        );
    }

    /// COMPOSICIÓN con el pipeline real: un ÁRBOL de medidores (Project
    /// sobre Filter sobre NodeScan) produce el recibo que coincide con
    /// `derivar_contadores` sobre las métricas del Executor — dos caminos,
    /// un solo número (Q2 del grafo demo).
    #[test]
    fn medidor_operador_en_arbol_coincide_con_exec_metrics() {
        let store = crate::demo_graph();
        let contadores = Contadores::new();
        let plan = plan_q2_filtro();

        // Camino A: árbol de MEDIDORES alrededor de los operadores reales
        // (medidor dentro; arriba del todo el Project SIN medir porque no
        // alimenta el recibo — igual que hará el --profile).
        let mut raiz: Box<dyn PhysicalOperator> = Box::new(ProjectOp::new(
            Box::new(MedidorOperador::nuevo(
                Box::new(FilterOp::new(
                    Box::new(MedidorOperador::nuevo(
                        Box::new(NodeScanOp::new(
                            &store,
                            "p".to_string(),
                            Some("Person".to_string()),
                        )),
                        &contadores,
                    )),
                    predicado_q2(),
                )),
                &contadores,
            )),
            proyeccion_nombre(),
        ));

        // Ejecutar a mano el ciclo Volcano (lo que hace Executor).
        raiz.open().unwrap();
        while raiz.next().unwrap().is_some() {}
        raiz.close().unwrap();

        // Camino B: el Executor normal + derivación.
        let mut executor = Executor::new(&plan, &store).unwrap();
        executor.execute().unwrap();
        let c2 = Contadores::new();
        derivar_contadores(&executor.metrics(), &c2);

        assert_eq!(
            contadores.snapshot(),
            c2.snapshot(),
            "árbol de medidores y derivación de ExecMetrics deben coincidir"
        );
    }

    /// Un `Expand` envuelto alimenta `relationships_expanded`: el mapa
    /// canónico usa el nombre REAL del operador envuelto, no el de la
    /// raíz del árbol.
    #[test]
    fn medidor_operador_alimenta_relationships_expanded_desde_texto() {
        use crate::compile;

        let store = crate::demo_graph();
        let contadores = Contadores::new();

        // La RAÍZ de Q3 es un Project: envolverla no cambia el recibo
        // (CampoOperador::Ninguno) — comprobación negativa del mapa.
        let query = crate::parse("MATCH (p:Person)-[:KNOWS]->(f) RETURN f.name").unwrap();
        let plan = query.lower().unwrap();
        let raiz = compile(&plan, &store).unwrap();
        let mut raiz_medida = MedidorOperador::nuevo(raiz, &contadores);
        raiz_medida.open().unwrap();
        while raiz_medida.next().unwrap().is_some() {}
        raiz_medida.close().unwrap();
        assert_eq!(
            contadores.snapshot().relationships_expanded,
            0,
            "la raíz Project no alimenta el recibo (mapa por nombre)"
        );
        assert_eq!(
            raiz_medida.filas_vistas(),
            4,
            "las filas SÍ se cuentan en el medidor local"
        );

        // Ahora el Expand directamente (sub-plan extraído del árbol).
        let expand_plan = extraer_expand(&plan);
        let expand = compile(&expand_plan, &store).unwrap();
        let mut medido = MedidorOperador::nuevo(expand, &contadores);
        medido.open().unwrap();
        while medido.next().unwrap().is_some() {}
        medido.close().unwrap();
        assert_eq!(
            contadores.snapshot().relationships_expanded,
            4,
            "4 aristas KNOWS en el demo"
        );
    }

    /// Baja al primer `Expand` del plan (para medirlo aislado).
    fn extraer_expand(plan: &LogicalPlan) -> LogicalPlan {
        match plan {
            LogicalPlan::Expand { .. } => plan.clone(),
            LogicalPlan::Project { input, .. } | LogicalPlan::Filter { input, .. } => {
                extraer_expand(input)
            }
            otro => otro.clone(),
        }
    }

    // ── MedidorPaginas: el voltímetro del pager ──────────────────────

    /// TESIS §2: reads/writes/syncs y bytes movidos (× PAGE_SIZE) sobre un
    /// FilePager REAL en fichero temporal.
    #[test]
    fn medidor_paginas_cuenta_reads_writes_y_bytes_movidos() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("paginas.db");
        let pager = FilePager::create(&ruta).unwrap();

        let contadores = Contadores::new();
        let mut medido = MedidorPaginas::nuevo(pager, &contadores);

        let id_a = medido.allocate().unwrap();
        let id_b = medido.allocate().unwrap();
        assert_eq!((id_a, id_b), (1, 2), "la 0 es metapágina");

        let buf = vec![0u8; PAGE_SIZE];
        medido.write(id_a, &buf).unwrap();
        let mut buf2 = vec![0u8; PAGE_SIZE];
        medido.read(id_a, &mut buf2).unwrap();
        medido.read(id_b, &mut buf2).unwrap();
        medido.sync().unwrap();

        let s = contadores.snapshot();
        assert_eq!((s.page_reads, s.page_writes), (2, 1));
        assert_eq!(medido.syncs(), 1);
        assert_eq!(medido.bytes_leidos(), 2 * PAGE_SIZE as u64);
        assert_eq!(medido.bytes_escritos(), PAGE_SIZE as u64);

        // Delegación transparente del resto del trait.
        assert_eq!(medido.num_pages(), 3);
        assert!(medido.is_allocated(id_a));
        assert!(!medido.is_allocated(42));

        // Un read FALLIDO no cuenta (contadores de éxito, como los del
        // pool del cap. 13).
        let mut malo = [0u8; 7]; // tamaño incorrecto: error garantizado
        assert!(medido.read(id_a, &mut malo).is_err());
        assert_eq!(contadores.snapshot().page_reads, 2);

        drop(medido.hacia_pager()); // devolver el pager al dueño
    }

    /// Composición con el BufferPool (cap. 13): pool y medidor cuentan la
    /// misma realidad por caminos distintos — composición, no copia.
    #[test]
    fn medidor_paginas_compone_con_buffer_pool() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("pool.db");
        let pager = FilePager::create(&ruta).unwrap();

        let contadores = Contadores::new();
        let mut pool = BufferPool::new(MedidorPaginas::nuevo(pager, &contadores), 2);

        // allocate vía el medidor (dentro del pool): ninguna métrica aún.
        let id = pool.pager_mut().allocate().unwrap();

        // get_page en MISS → pager.read → cuenta en ambos lados.
        {
            let pagina = pool.get_page(id).unwrap();
            pagina[0] = 9;
        }
        pool.unpin(id, true).unwrap(); // dirty: el flush la escribirá
        pool.flush().unwrap(); // → pager.write

        let s = contadores.snapshot();
        let m = pool.metrics();
        assert_eq!(m.page_reads, 1, "un miss = una lectura de pager");
        assert_eq!(m.page_writes, 1, "flush de una dirty = una escritura");
        assert_eq!(s.page_reads, m.page_reads, "mismo read contado dos veces");
        assert_eq!(
            s.page_writes, m.page_writes,
            "mismo write contado dos veces"
        );
    }

    // ── Fiabilidad HONESTA: WAL por delta, tx en conductora ──────────

    /// TESIS §2: `wal_bytes_written` = DELTA de `Wal::as_bytes().len()`
    /// alrededor de `WalTransaccion::commit`. El WAL no expone contador de
    /// bytes (sólo el TOTAL) — el delta ES la medida honesta disponible.
    #[test]
    fn wal_bytes_escritos_delta_tras_commit_waltransaccion() {
        let mut store = MemoryStore::new();
        let mut wal = Wal::con_politica(PoliticaFlush::SoloCommit);
        let contadores = Contadores::new();

        let antes = wal.as_bytes().len();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        let mut nodo = Node::new(100, "Persona");
        nodo.props
            .insert("name".to_string(), Value::String("Delta".to_string()));
        tx.put_node(nodo).unwrap();
        let resumen = tx.commit().unwrap();

        let despues = wal.as_bytes().len();
        let delta = despues.saturating_sub(antes);
        assert!(delta > 0, "un commit debe escribir bytes al log");

        contadores.sumar_wal_bytes(delta);
        assert_eq!(contadores.snapshot().wal_bytes_written, delta as u64);
        assert_eq!(
            contadores.snapshot().wal_bytes_written as usize,
            despues - antes,
            "delta = total final − total inicial"
        );
        assert!(resumen.nodos_escritos >= 1);
    }

    /// TESIS §2: `transactions_committed`/`aborted` se cuentan en la CAPA
    /// CONDUCTORA — el patrón del REPL (`:commit`/`:rollback` consumen la
    /// tx y el conductor cuenta el resultado), demostrado aquí alrededor
    /// de `Transaccion` (cap. 27). Los caps. 27/28 NO exponen contadores
    /// y NO se tocan.
    #[test]
    fn transacciones_committed_aborted_contadas_en_conductora() {
        let mut store = MemoryStore::new();
        let contadores = Contadores::new();

        // Conductora: commit exitoso.
        let mut tx = Transaccion::begin(&mut store);
        let mut nodo = Node::new(200, "Persona");
        nodo.props
            .insert("name".to_string(), Value::String("Confirmada".to_string()));
        tx.put_node(nodo).unwrap();
        tx.commit().unwrap();
        contadores.contar_commit();

        // Conductora: rollback (el patrón :quit-dentro-de-tx del REPL).
        let tx = Transaccion::begin(&mut store);
        tx.rollback();
        contadores.contar_rollback();

        let s = contadores.snapshot();
        assert_eq!((s.transactions_committed, s.transactions_aborted), (1, 1));
        assert_eq!(
            store.node_count(),
            1,
            "sólo el nodo del commit persiste (el rollback no aplicó nada)"
        );
    }
}
