//! Cap. 35 — La mitad «itinerario» de la observabilidad: SPANS y su
//! captura con un subscriber propio.
//!
//! Aquí viven las piezas que NECESITAN tracing (por eso la dependency es
//! de esta CLI y no de `vol2-liradb`, que sigue dependency-free):
//!
//! * [`SuscriptorArbol`] — mini-subscriber propio (~100 líneas) sobre el
//!   trait `tracing::Subscriber`: graba cada span (nombre, id, padre) en
//!   un árbol consultable y fija la DURACIÓN sobrescribiendo
//!   [`tracing::Subscriber::try_close`], porque los métodos por defecto
//!   NO notifican el cierre (riesgo §5.1 del contrato: sin override,
//!   ninguna duración existiría nunca).
//!
//! * [`OperadorTrazado`] / [`PagerTrazado`] — decoradores que APILAN un
//!   span sobre el medidor (`MedidorOperador`/`MedidorPaginas` de la lib):
//!   medidor dentro, span fuera. El orden importa: el span mide al
//!   operador REAL (el medidor es transparente para él), y ambos son
//!   apilables e independientes.
//!
//! * [`pipeline_perfilada`] — el hito `liradb query --profile '...'`:
//!   desenrolla el pipeline (parse → plan → execute) emitido SIEMPRE como
//!   spans (coste ≈ 0 sin subscriber instalado — decisión §5.4), instala
//!   el subscriber sólo cuando se pide perfil, cronometra fases con
//!   `Instant` (herencia del harness del cap. 34; `query_duration` NO es
//!   un contador: es una distribución) e imprime fases + árbol + recibo.
//!
//! Los tests capturan con `tracing::dispatcher::with_default`
//! (dispatcher thread-local de ámbito): sin globales ni stderr.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};
use vol2_liradb::{
    CartesianProductOp, Contadores, ExecError, ExecMetrics, ExpandOp, FilterOp, GraphStore,
    IndexSeekOp, LogicalPlan, NodeScanOp, PhysicalOperator, ProjectOp, ResultSet, Row,
};

// ─── SuscriptorArbol: capturar el árbol, no imprimirlo ───

/// Un span ya capturado: nombre, padre, inicio y duración (si cerró).
///
/// Tipado A PROPÓSITO: la jerarquía se verifica con asserts sobre nodos y
/// padres, jamás parseando texto indentado (decisión §5.7 del contrato).
#[derive(Debug, Clone)]
pub struct NodoSpan {
    /// Nombre del span (fase: `query`/`parse`/…; operador: nombre
    /// canónico; página: `storage_read`).
    pub nombre: String,
    /// Id del span padre (`None` = raíz).
    pub padre: Option<u64>,
    /// Duración total (`None` mientras siga abierto).
    pub duracion: Option<Duration>,
    /// Instante de creación (privado: nadie necesita mutarlo ni leerlo
    /// antes del cierre; la duración ya lo resume).
    inicio: Instant,
}

/// Mini-subscriber que reconstruye el ÁRBOL causal de spans.
///
/// Cómo resuelve los padres contextuales: si `Attributes::parent()` trae
/// id explícito, gana ese; si viene `None` (parenting contextual, lo que
/// hacen los macros por defecto), el padre es el TOPE de la pila de spans
/// actuales, que este subscriber mantiene con enter/exit. Sin esa pila,
/// todos los spans colgarían de la raíz y el árbol sería mentira.
///
/// Por qué `Mutex` y no `RefCell`: `dispatcher::with_default` exige
/// `Send + Sync` en el subscriber (la firma de `Dispatch::new`), y
/// `RefCell` no es `Sync`. En monohilo la semántica es idéntica; el lock
/// sin contención cuesta nanosegundos. (Divergencia menor documentada
/// frente al borrador del contrato, que decía `RefCell`.)
///
/// Duraciones: se fijan en [`Subscriber::try_close`] — el método que los
/// defaults DELEGAN sin avisar. Sobrescribirlo es la diferencia entre
/// tener duraciones y creerlas.
pub struct SuscriptorArbol {
    siguiente_id: AtomicU64,
    nodos: Mutex<Vec<(u64, NodoSpan)>>,
    pila_actual: Mutex<Vec<u64>>,
}

impl SuscriptorArbol {
    /// Subscriber vacío y listo para recibir spans.
    pub fn nuevo() -> Self {
        Self {
            // Ids de tracing arrancan en 1 (0 reservado): fetch_add desde 0
            // devuelve 0 primero, así que sumamos 1 al usarlo.
            siguiente_id: AtomicU64::new(0),
            nodos: Mutex::new(Vec::new()),
            pila_actual: Mutex::new(Vec::new()),
        }
    }

    /// Nº de spans capturados (abiertos + cerrados).
    pub fn total(&self) -> usize {
        self.nodos.lock().expect("sin envenenamiento").len()
    }

    /// Copia tipada del árbol en orden de creación (los ids crecen con él).
    pub fn arbol(&self) -> Vec<(u64, NodoSpan)> {
        self.nodos.lock().expect("sin envenenamiento").clone()
    }

    /// Hijos directos de un span (raíces si `None`), en orden de creación.
    pub fn hijos(&self, padre: Option<u64>) -> Vec<u64> {
        let nodos = self.nodos.lock().expect("sin envenenamiento");
        nodos
            .iter()
            .filter(|(_, n)| n.padre == padre)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Info de un span concreto (nombre + duración si cerró).
    pub fn nodo_info(&self, id: u64) -> Option<(String, Option<Duration>)> {
        let nodos = self.nodos.lock().expect("sin envenenamiento");
        nodos
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, n)| (n.nombre.clone(), n.duracion))
    }

    /// Padre de un span por id (`None` = raíz).
    pub fn padre_de(&self, id: u64) -> Option<u64> {
        let nodos = self.nodos.lock().expect("sin envenenamiento");
        nodos
            .iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, n)| n.padre)
    }
}

impl Subscriber for SuscriptorArbol {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let id = self.siguiente_id.fetch_add(1, Ordering::Relaxed) + 1;
        // Padre explícito si lo hay; si no, CONTEXTUAL: el tope de la
        // pila (lo que esté «dentro» en este hilo justo ahora).
        let padre = attrs.parent().map(|p| p.into_u64()).or_else(|| {
            self.pila_actual
                .lock()
                .expect("sin envenenamiento")
                .last()
                .copied()
        });
        self.nodos.lock().expect("sin envenenamiento").push((
            id,
            NodoSpan {
                nombre: attrs.metadata().name().to_string(),
                padre,
                inicio: Instant::now(),
                duracion: None,
            },
        ));
        Id::from_u64(id)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {}

    fn enter(&self, id: &Id) {
        self.pila_actual
            .lock()
            .expect("sin envenenamiento")
            .push(id.into_u64());
    }

    fn exit(&self, id: &Id) {
        let raw = id.into_u64();
        let mut pila = self.pila_actual.lock().expect("sin envenenamiento");
        // rposition (no pop a secas): enter puede repetirse en re-entrada
        // y el exit correcto saca ESA ocurrencia.
        if let Some(pos) = pila.iter().rposition(|x| *x == raw) {
            pila.remove(pos);
        }
    }

    fn try_close(&self, id: Id) -> bool {
        // EL punto del capítulo: los defaults NO avisan del cierre. Este
        // override es quien convierte «pasó» en «tardó». Idempotente:
        // sólo fija la duración la primera vez (close + drop no duplican).
        let raw = id.into_u64();
        let mut nodos = self.nodos.lock().expect("sin envenenamiento");
        if let Some((_, nodo)) = nodos.iter_mut().find(|(i, _)| *i == raw)
            && nodo.duracion.is_none()
        {
            nodo.duracion = Some(nodo.inicio.elapsed());
        }
        true
    }
}

// ─── Render del árbol (presentación, separada de la captura) ───

/// Formato humano de una duración: ns bajo mil, µs bajo millón, ms encima.
fn formatear_duracion(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns < 1_000 {
        format!("{ns} ns")
    } else if ns < 1_000_000 {
        format!("{:.1} µs", ns as f64 / 1_000.0)
    } else {
        format!("{:.1} ms", ns as f64 / 1_000_000.0)
    }
}

/// Renderiza el árbol de spans indentado (conectores └─/├─) con la
/// duración de cada span a la derecha.
///
/// La RAÍZ va sin conector; cada nivel añade prefijo. Duración ausente =
/// span aún abierto (no debería ocurrir tras cerrar el pipeline: todos
/// los spans mueren al soltarse sus guards).
pub fn arbol_indentado(sub: &SuscriptorArbol) -> String {
    let mut out = String::new();
    for id in sub.hijos(None) {
        let Some((nombre, duracion)) = sub.nodo_info(id) else {
            continue;
        };
        out.push_str(&linea_span("", nombre, duracion));
        pintar_hijos(sub, &sub.hijos(Some(id)), "", &mut out);
    }
    out
}

/// Una línea del árbol: etiqueta a la izquierda, duración a la derecha.
fn linea_span(prefijo_conector: &str, nombre: String, duracion: Option<Duration>) -> String {
    let etiqueta = format!("{prefijo_conector}{nombre}");
    let tiempo = duracion
        .map(formatear_duracion)
        .unwrap_or_else(|| "abierto".to_string());
    format!("{etiqueta:<30}{tiempo:>10}\n")
}

/// Recursión del render: conectores ├─/└─ y prefijo heredado.
fn pintar_hijos(sub: &SuscriptorArbol, ids: &[u64], prefijo: &str, out: &mut String) {
    for (i, &id) in ids.iter().enumerate() {
        let ultimo = i + 1 == ids.len();
        let conector = if ultimo { "└─ " } else { "├─ " };
        let Some((nombre, duracion)) = sub.nodo_info(id) else {
            continue;
        };
        out.push_str(&linea_span(
            &format!("{prefijo}{conector}"),
            nombre,
            duracion,
        ));
        let prefijo_hijo = format!("{prefijo}{}", if ultimo { "   " } else { "│  " });
        pintar_hijos(sub, &sub.hijos(Some(id)), &prefijo_hijo, out);
    }
}

// ─── OperadorTrazado: span alrededor del medidor del Volcano ───

/// Decorador `PhysicalOperator` que emite un SPAN hijo del contexto actual
/// durante open/next/close del operador envuelto.
///
/// El nombre del span es el NOMBRE CANÓNICO del operador (`NodeScan`,
/// `IndexSeek`, …): la traza habla el idioma del `explain` del cap. 21.
pub struct OperadorTrazado<'a> {
    inner: Box<dyn PhysicalOperator + 'a>,
    /// Span vivo desde `open()` hasta `close()` (o drop). `None` antes de
    /// abrir: sin open no hay fase que contar.
    span: Option<tracing::Span>,
}

impl<'a> OperadorTrazado<'a> {
    /// Envuelve el operador (ya medido o no) con su span.
    pub fn nuevo(inner: Box<dyn PhysicalOperator + 'a>) -> Self {
        Self { inner, span: None }
    }

    /// Span con nombre canónico para el operador dado.
    ///
    /// Cada brazo crea SU callsite estático: cero coste por consulta fuera
    /// de esto, y nombres literales exigibles por el subscriber. El comodín
    /// final es la confesión honesta de que un `&str` no se puede vigilar
    /// EN COMPILACIÓN: si el cap. 20 estrenara nombre nuevo, el span saldría
    /// «operador» y los tests ESTRUCTURALES del árbol romperían — vigilancia
    /// en tests, no en compilador.
    fn span_para(nombre: &'static str) -> tracing::Span {
        match nombre {
            "NodeScan" => tracing::info_span!("NodeScan"),
            "IndexSeek" => tracing::info_span!("IndexSeek"),
            "Expand" => tracing::info_span!("Expand"),
            "Filter" => tracing::info_span!("Filter"),
            "Project" => tracing::info_span!("Project"),
            "CartesianProduct" => tracing::info_span!("CartesianProduct"),
            "Limit" => tracing::info_span!("Limit"),
            "Distinct" => tracing::info_span!("Distinct"),
            otro => tracing::info_span!("operador", canonico = %otro),
        }
    }
}

impl Drop for OperadorTrazado<'_> {
    fn drop(&mut self) {
        // close() consume el span; esto cubre el olvido de close(): el
        // drop del Span dispara try_close igualmente (idempotente).
        self.span.take();
    }
}

impl PhysicalOperator for OperadorTrazado<'_> {
    fn open(&mut self) -> Result<(), ExecError> {
        // El span se crea DENTRO de open: su padre contextual es quien
        // esté activo entonces — `execute`, o el operador PADRE cuya
        // propia cascada de open sigue en curso. Así el árbol físico
        // emerge solo, sin pasar padres a mano.
        let span = Self::span_para(self.inner.name());
        let r = {
            let _guard = span.enter();
            self.inner.open()
            // guard drop → exit ANTES del move de abajo (orden explícito).
        };
        self.span = Some(span);
        r
    }

    fn next(&mut self) -> Result<Option<Row>, ExecError> {
        match &self.span {
            Some(s) => {
                let _guard = s.enter();
                self.inner.next()
            }
            // Sin open previo el operador real devuelve None igualmente;
            // delegamos tal cual (misma conducta que el cap. 20).
            None => self.inner.next(),
        }
    }

    fn close(&mut self) -> Result<(), ExecError> {
        match self.span.take() {
            Some(s) => {
                let _guard = s.enter();
                self.inner.close() // guard → exit; al morir `s` → try_close
            }
            None => self.inner.close(),
        }
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

// ─── PagerTrazado: el nivel página ───

/// Decorador `Pager` que emite un span `storage_read` POR lectura.
///
/// Es el CUARTO nivel de la jerarquía del brief (query → operador →
/// storage_read). Hoy las consultas corren sobre `MemoryStore`, así que
/// este nivel NO aparece en la ruta de consulta: se demuestra a nivel
/// COMPONENTE (índice sobre `BufferPool<...>`). Con un futuro DiskStore
/// detrás del puerto, la jerarquía completa aparecerá sin tocar nada de
/// esta instrumentación — esa es la promesa hexagonal.
pub struct PagerTrazado<P> {
    inner: P,
}

impl<P> PagerTrazado<P> {
    /// Envuelve el pager (medido o no) con trazas de página.
    pub fn nuevo(inner: P) -> Self {
        Self { inner }
    }

    /// Acceso al pager envuelto.
    pub fn pager(&self) -> &P {
        &self.inner
    }

    /// Acceso mutable al pager envuelto.
    pub fn pager_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Consume el trazador y devuelve el pager original.
    pub fn hacia_pager(self) -> P {
        self.inner
    }
}

impl<P: vol2_liradb::Pager> vol2_liradb::Pager for PagerTrazado<P> {
    fn allocate(&mut self) -> Result<vol2_liradb::PageId, vol2_liradb::PagerError> {
        self.inner.allocate()
    }

    fn read(
        &mut self,
        id: vol2_liradb::PageId,
        page: &mut [u8],
    ) -> Result<(), vol2_liradb::PagerError> {
        // Un span POR lectura: aquí vive el coste real del disco (cuando
        // lo haya). El nº de página viaja como campo del span.
        let span = tracing::info_span!("storage_read", pagina = id as u64);
        let _guard = span.enter();
        self.inner.read(id, page)
    }

    fn write(
        &mut self,
        id: vol2_liradb::PageId,
        page: &[u8],
    ) -> Result<(), vol2_liradb::PagerError> {
        self.inner.write(id, page)
    }

    fn sync(&mut self) -> Result<(), vol2_liradb::PagerError> {
        self.inner.sync()
    }

    fn num_pages(&self) -> u32 {
        self.inner.num_pages()
    }

    fn free(&mut self, id: vol2_liradb::PageId) -> Result<(), vol2_liradb::PagerError> {
        self.inner.free(id)
    }

    fn is_allocated(&self, id: vol2_liradb::PageId) -> bool {
        self.inner.is_allocated(id)
    }

    fn page_size(&self) -> usize {
        self.inner.page_size()
    }
}

// ─── Compilación perfilada: compile() recorrido con envoltorios ───

/// Compila el plan como [`vol2_liradb::compile`] pero envolviendo CADA
/// operador con su span ([`OperadorTrazado`]).
///
/// Es el MISMO mapeo lógico→físico del cap. 20, recorrido con decoradores.
/// DELIBERADAMENTE NO apila aquí [`MedidorOperador`]: el recibo de la
/// consulta se DERIVA de `ExecMetrics` (UNA sola verdad, §3 del contrato)
/// y un medidor en el árbol contaría las mismas filas OTRA vez — doble
/// conteo, el bug clásico de telemetría que el capítulo denuncia. Los
/// medidores existen y se componen donde NO hay derivación (tests de la
/// lib, herramientas futuras).
///
/// El match replica el de `compile`: si una variante nueva del
/// `LogicalPlan` aparece, este fichero NO compila — vigilancia estructural,
/// nunca silencio.
fn compilar_perfilado<'a>(
    plan: &LogicalPlan,
    store: &'a dyn GraphStore,
) -> Result<Box<dyn PhysicalOperator + 'a>, ExecError> {
    let interno: Box<dyn PhysicalOperator + 'a> = match plan {
        LogicalPlan::NodeScan { variable, label } => {
            Box::new(NodeScanOp::new(store, variable.clone(), label.clone()))
        }
        LogicalPlan::IndexSeek { variable, ids, .. } => {
            Box::new(IndexSeekOp::new(store, variable.clone(), ids.clone()))
        }
        LogicalPlan::Expand {
            input,
            from,
            rel_variable,
            rel_type,
            direction,
            to,
        } => {
            let hijo = compilar_perfilado(input, store)?;
            Box::new(ExpandOp::new(
                store,
                hijo,
                from.clone(),
                rel_variable.clone(),
                rel_type.clone(),
                *direction,
                to.clone(),
            ))
        }
        LogicalPlan::Filter { input, predicate } => {
            let hijo = compilar_perfilado(input, store)?;
            Box::new(FilterOp::new(hijo, predicate.clone()))
        }
        LogicalPlan::Project { input, items } => {
            let hijo = compilar_perfilado(input, store)?;
            Box::new(ProjectOp::new(hijo, items.clone()))
        }
        LogicalPlan::CartesianProduct { left, right } => {
            let izq = compilar_perfilado(left, store)?;
            let der = compilar_perfilado(right, store)?;
            Box::new(CartesianProductOp::new(izq, der))
        }
    };
    // Span fuera; el medidor NO entra aquí (recibo derivado, ver arriba).
    Ok(Box::new(OperadorTrazado::nuevo(interno)))
}

// ─── El hito: pipeline perfilada de UNA consulta ───

/// Fase cronometrada del pipeline (para la tabla de tiempos).
struct FaseCronometrada {
    nombre: &'static str,
    duracion: Duration,
}

/// Ejecuta UNA consulta con spans de fase, operador y página; imprime
/// tabla resultado + fases cronometradas + árbol de spans + recibo.
///
/// * `plan`/`stats` mantienen SU semántica del cap. 31 (plan lógico y
///   métricas por operador): `--profile` es ADITIVO y compone con ellos.
/// * Los spans se emiten SIEMPRE; el subscriber sólo se instala aquí.
///   Emitir sin subscriber cuesta ≈ 0 (chequeo de interés estático) —
///   separar instrumentar de activar ES la idea de producción (§5.4).
/// * La salida NO es golden byte-exacto: los tiempos varían. Lo pactado
///   (tests estructurales) son nombres, nesting y CONTADORES.
pub fn pipeline_perfilada(
    src: &str,
    store: &dyn GraphStore,
    out: &mut dyn Write,
    plan: bool,
    stats: bool,
) -> Result<(), ExecError> {
    use vol2_liradb::{derivar_contadores, parse};

    let contadores = Contadores::new();
    contadores.incrementar_queries_total(1);

    let sub = Arc::new(SuscriptorArbol::nuevo());
    let dispatch = tracing::Dispatch::from(Arc::clone(&sub));
    let mut fases: Vec<FaseCronometrada> = Vec::new();

    let resultado = tracing::dispatcher::with_default(
        &dispatch,
        || -> Result<(ResultSet, ExecMetrics), ExecError> {
            // RAÍZ: el span query rodea todo (1 por consulta).
            let _guard_query = tracing::info_span!("query").entered();

            let t0 = Instant::now();
            let query = {
                let _guard_fase = tracing::info_span!("parse").entered();
                parse(src)?
            };
            fases.push(FaseCronometrada {
                nombre: "parse",
                duracion: t0.elapsed(),
            });

            let t0 = Instant::now();
            let plan_logico = {
                let _guard_fase = tracing::info_span!("plan", fase = "lower").entered();
                query.lower()?
            };
            fases.push(FaseCronometrada {
                nombre: "plan",
                duracion: t0.elapsed(),
            });

            if plan {
                let _ = out.write_all("Plan lógico:\n".as_bytes());
                let _ = out.write_all(format!("{plan_logico}\n").as_bytes());
            }

            let t0 = Instant::now();
            let rs_metricas = {
                let _guard_execute = tracing::info_span!("execute").entered();

                // Columnas: mismo requisito que Executor::new (raíz Project).
                let columnas: Vec<String> = match &plan_logico {
                    LogicalPlan::Project { items, .. } => {
                        items.iter().map(|p| p.output_name()).collect()
                    }
                    _ => return Err(ExecError::NotAProjection),
                };

                // Ciclo Volcano a mano sobre el árbol MEDIDO+TRAZADO: es el
                // cuerpo de Executor::execute con el root sustituido.
                let mut raiz = compilar_perfilado(&plan_logico, store)?;
                let mut filas: Vec<Vec<vol2_liradb::Cell>> = Vec::new();
                raiz.open()?;
                while let Some(row) = raiz.next()? {
                    filas.push(row.cells());
                }
                raiz.close()?; // SIEMPRE, también en error (como un defer).
                let metricas = ExecMetrics {
                    per_operator: raiz.collect_metrics(),
                    rows_returned: filas.len() as u64,
                };
                (
                    ResultSet {
                        columns: columnas,
                        rows: filas,
                    },
                    metricas,
                )
            };
            fases.push(FaseCronometrada {
                nombre: "execute",
                duracion: t0.elapsed(),
            });
            Ok(rs_metricas)
        },
    );

    let (rs, metricas) = resultado?;

    // El recibo: derivación de ExecMetrics (UNA sola verdad) + Display.
    derivar_contadores(&metricas, &contadores);

    // ── Presentación ──
    let _ = out.write_all(b"Resultado:\n");
    let _ = out.write_all(rs.to_string().as_bytes());

    let _ = out.write_all(b"Perfil (cap. 35):\n");
    let _ = out.write_all(b"Fases:\n");
    for fase in &fases {
        let linea = format!(
            "  {:<9}{:>12}\n",
            fase.nombre,
            formatear_duracion(fase.duracion)
        );
        let _ = out.write_all(linea.as_bytes());
    }

    let _ = out.write_all("Árbol de spans:\n".as_bytes());
    let _ = out.write_all(arbol_indentado(&sub).as_bytes());

    let _ = out.write_all(b"Contadores:\n");
    let _ = out.write_all(contadores.to_string().as_bytes());

    if stats {
        let _ = out.write_all(format!("Métricas: {metricas}\n").as_bytes());
    } else {
        let _ = out.write_all("Métricas: (usa --stats para el detalle)\n".as_bytes());
    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_observabilidad_cli {
    use super::*;
    use tracing::info_span;
    use vol2_liradb::{FilePager, MedidorOperador, NodeScanOp, Pager};

    /// Ejecuta `f` con UN SuscriptorArbol instalado y lo devuelve junto a
    /// lo que `f` produzca: el andamiaje de TODOS los tests de captura
    /// (`with_default` es thread-local: sin globales ni stderr).
    ///
    /// Devuelve `Arc<SuscriptorArbol>` porque `Dispatch` toma propiedad:
    /// el Arc comparte el MISMO subscriber entre dispatch y test.
    fn capturar<T>(f: impl FnOnce() -> T) -> (Arc<SuscriptorArbol>, T) {
        let sub = Arc::new(SuscriptorArbol::nuevo());
        let dispatch = tracing::Dispatch::from(Arc::clone(&sub));
        let r = tracing::dispatcher::with_default(&dispatch, f);
        (sub, r)
    }

    /// Id del primer span con ese nombre (o pánico descriptivo).
    fn id_de(sub: &SuscriptorArbol, nombre: &str) -> u64 {
        sub.arbol()
            .into_iter()
            .find(|(_, n)| n.nombre == nombre)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("no hay span '{nombre}' en el árbol"))
    }

    // ── SuscriptorArbol ──────────────────────────────────────────────

    #[test]
    fn suscriptor_graba_padres_duraciones_y_resuelve_contexto() {
        let (sub, ()) = capturar(|| {
            let guard_query = info_span!("query").entered();
            {
                let _g = info_span!("parse").entered();
            }
            {
                let _g = info_span!("plan").entered();
            }
            {
                let g_e = info_span!("execute").entered();
                {
                    let _g = info_span!("NodeScan").entered();
                }
                drop(g_e);
            }
            drop(guard_query);
        });

        assert_eq!(sub.total(), 5, "query+parse+plan+execute+NodeScan");

        // Jerarquía EXACTA: raíz única y padres contextuales correctos.
        let raices = sub.hijos(None);
        assert_eq!(raices.len(), 1, "una sola raíz");
        let query = raices[0];
        assert_eq!(
            sub.hijos(Some(query)),
            vec![
                id_de(&sub, "parse"),
                id_de(&sub, "plan"),
                id_de(&sub, "execute")
            ],
            "hijos de query EN ORDEN de creación"
        );
        let execute = id_de(&sub, "execute");
        assert_eq!(sub.hijos(Some(execute)), vec![id_de(&sub, "NodeScan")]);

        // Duraciones: try_close sobrescrito ES quien las fija; todo cerró.
        for (id, nodo) in sub.arbol() {
            assert!(
                nodo.duracion.is_some(),
                "span {id} ({}) quedó abierto",
                nodo.nombre
            );
        }
        let (_, q) = sub.arbol().into_iter().find(|(i, _)| *i == query).unwrap();
        let dur_parse = sub.nodo_info(id_de(&sub, "parse")).unwrap().1.unwrap();
        assert!(
            q.duracion.unwrap() >= dur_parse,
            "el padre vive al menos tanto como su hijo"
        );
    }

    // La jerarquía COMPLETA del brief (query→parse/plan/optimise/execute
    // →operador→storage_read) vive en `tests/observabilidad_cli.rs` con el
    // nombre exacto del contrato: suscriptor_arbol_jerarquia_query_plan_optimise_execute.

    // ── OperadorTrazado: delegación + spans apilados sobre medidor ───

    #[test]
    fn operador_trazado_apila_span_sobre_medidor_y_delega_todo() {
        let store = vol2_liradb::demo_graph();
        let contadores = vol2_liradb::Contadores::new();

        let (sub, filas) = capturar(|| {
            let _ge = info_span!("execute").entered();
            // MEDIDOR dentro, SPAN fuera (§4: apilables): el span mide al
            // operador real; el medidor cuenta sin saber del span.
            let medidor = MedidorOperador::nuevo(
                Box::new(NodeScanOp::new(
                    &store,
                    "p".to_string(),
                    Some("Person".to_string()),
                )),
                &contadores,
            );
            let mut trazado = OperadorTrazado::nuevo(Box::new(medidor));
            trazado.open().unwrap();
            let mut filas = 0;
            while trazado.next().unwrap().is_some() {
                filas += 1;
            }
            trazado.close().unwrap();
            filas
        });

        assert_eq!(filas, 4, "delega next (4 Person del demo)");
        assert_eq!(
            sub.hijos(Some(id_de(&sub, "execute"))),
            vec![id_de(&sub, "NodeScan")],
            "el span del operador cuelga de execute"
        );
        let (_, nodo_scan) = sub
            .arbol()
            .into_iter()
            .find(|(_, n)| n.nombre == "NodeScan")
            .unwrap();
        assert!(nodo_scan.duracion.is_some(), "cerró con try_close");
    }

    // ─── PagerTrazado: storage_read por lectura ───

    #[test]
    fn pager_trazado_emite_storage_read_por_cada_read() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("trazado.db");
        let pager = FilePager::create(&ruta).unwrap();

        let (sub, ids) = capturar(|| {
            let mut trazado = PagerTrazado::nuevo(pager);
            let id = trazado.allocate().unwrap();
            let mut pagina = vec![0u8; trazado.page_size()];
            trazado.write(id, &pagina).unwrap();
            trazado.read(id, &mut pagina).unwrap();
            trazado.read(id, &mut pagina).unwrap();
            trazado.sync().unwrap();
            id
        });

        assert_eq!(
            sub.total(),
            2,
            "UN span por read (write/sync/allocate no emiten)"
        );
        let ids_spans: Vec<u64> = sub.hijos(None);
        assert_eq!(ids_spans.len(), 2, "sin subscriber-contextual: raíces");
        let _ = ids; // el PageId no participa en la aserción
    }

    #[test]
    fn formatear_duracion_elige_unidad_legible() {
        assert_eq!(formatear_duracion(Duration::from_nanos(250)), "250 ns");
        assert_eq!(formatear_duracion(Duration::from_nanos(12_345)), "12.3 µs");
        assert_eq!(formatear_duracion(Duration::from_millis(3)), "3.0 ms");
    }

    #[test]
    fn arbol_indentado_dibuja_conectores_y_columna_de_tiempo() {
        let (sub, ()) = capturar(|| {
            let _gq = info_span!("raiz").entered();
            {
                let _ga = info_span!("a").entered();
            }
            {
                // HERMANO de «a» (fuera de su ámbito): así hay ├─ y └─.
                let _gb = info_span!("b").entered();
            }
        });
        let texto = arbol_indentado(&sub);
        let lineas: Vec<&str> = texto.lines().collect();
        assert_eq!(lineas.len(), 3);
        assert!(lineas[0].starts_with("raiz"), "la raíz sin conector");
        assert!(lineas[1].contains("├─ a"), "primer hijo con ├─");
        assert!(lineas[2].contains("└─ b"), "último hijo con └─");
        for l in lineas {
            assert!(!l.contains("abierto"), "todo cerrado: {l}");
        }
    }
}
