use crate::cap11_slotted_pages::{MetaPage, PAGE_SIZE, PageHeader, PageType, SlottedPage};
use crate::cap12_pager::{PageId, Pager, PagerError};
use crate::cap13_buffer_pool::{BufferPool, BufferPoolError};

// ─────────────────── Cap 16: Compactación y mantenimiento ───────────────────
//
// En los caps 11-15 construimos el motor de almacenamiento: páginas, Pager,
// BufferPool, CSR, HashIndex y B+Tree. Cada uno añade páginas al fichero y,
// con el uso, aparecen tres problemas típicos de cualquier DBMS:
//
//   1. **Espacio muerto**: páginas asignadas con records borrados lógicamente
//      (o cabeceras con `free_space` desactualizado tras updates/deletes).
//   2. **Fragmentación**: bytes libres dispersos entre páginas que no se
//      aprovechan sin una reescritura.
//   3. **Inconsistencia**: páginas corruptas por un crash a mitad de escritura
//      (magic cambiado, `page_id` que no encaja con el offset, records
//      truncados).
//
// Este capítulo implementa las tres herramientas de mantenimiento que el
// brief (§cap 16) demanda como hito CLI:
//
//   ```text
//   liradb inspect   → estadísticas de almacenamiento
//   liradb check     → verificación de integridad
//   liradb compact   → reescritura (repack) página a página
//   ```
//
// Filosofía pedagógica (coherente con caps 12-15):
//
//   - **Errores tipados** (`MaintenanceError`) con `From<BufferPoolError>` y
//     `From<PagerError>` para `?` ergonómico, como `IndexError` (cap 15).
//
//   - **Sin crates externas**: toda la lectura/decodificación se hace con
//     las primitivas ya implementadas (`SlottedPage::decode`,
//     `PageHeader::decode`, `MetaPage::decode`). Mantenimiento offline,
//     no hot-path: leemos vía el pager subyacente sin cachear en el pool.
//
//   - **PageId = offset físico**: NO movemos páginas (eso rompería CSR,
//     índices y cualquier puntero interno). La compactación es **repack
//     in-place** página a página: reescribe cada `SlottedPage` sin huecos,
//     recalcula `free_space` y limpia bytes basura tras los records. El
//     "vacuum" que reduce el tamaño del fichero (truncate de páginas
//     finales) queda como limitación declarada — requiere reescribir
//     punteros y es tema de cap 29 (recuperación) / 36 (arquitectura).
//
// Layout de páginas (recordatorio, caps 11-12):
//
//   - Página 0  = MetaPage (`PageType::Meta`, magic `0xFE`).
//   - Páginas 1..N = Data pages (`PageType::Data`, magic `0xDA`) con
//     `PageHeader` (10 B) + records con length-prefix.
//   - Páginas libres (en la free list del pager) contienen ceros; NO las
//     tocamos: no son datos válidos.
//
// API mínima:
//
//   ```text
//   inspect(pool)          → StorageStats (totals, uso, fragmentación)
//   check(pool)            → CheckReport  (issues página a página)
//   repack_page(pool, id)  → RepackResult (bytes recuperados en 1 página)
//   compact(pool)          → CompactReport (repack de todas las Data pages)
//   ```

/// Errores de mantenimiento y compactación (cap 16).
///
/// Diseño paralelo a `IndexError` (cap 15): variante `Io` que envuelve al
/// `BufferPoolError` (que a su vez envuelve `PagerError`), más variantes
/// específicas para páginas que no encajan con el formato esperado.
#[derive(Debug)]
pub enum MaintenanceError {
    /// Error de E/S del pager / buffer pool.
    Io(BufferPoolError),
    /// La página `page_id` tiene un `PageType` distinto al esperado.
    /// `expected`/`got` son los bytes crudos del magic para diagnóstico.
    BadPageType {
        page_id: PageId,
        expected: u8,
        got: u8,
    },
    /// La página `page_id` no se pudo decodificar como SlottedPage/MetaPage
    /// (magic corrupto, records truncados, etc.).
    DecodeFailed { page_id: PageId, reason: String },
    /// El `PageId` solicitado no está asignado en el pager.
    PageNotAllocated(PageId),
}

impl std::fmt::Display for MaintenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaintenanceError::Io(e) => write!(f, "maintenance io: {e}"),
            MaintenanceError::BadPageType {
                page_id,
                expected,
                got,
            } => {
                write!(
                    f,
                    "page {page_id}: bad page type (expected {expected:#x}, got {got:#x})"
                )
            }
            MaintenanceError::DecodeFailed { page_id, reason } => {
                write!(f, "page {page_id}: decode failed ({reason})")
            }
            MaintenanceError::PageNotAllocated(id) => {
                write!(f, "page {id} not allocated in pager")
            }
        }
    }
}

impl std::error::Error for MaintenanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MaintenanceError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BufferPoolError> for MaintenanceError {
    fn from(e: BufferPoolError) -> Self {
        MaintenanceError::Io(e)
    }
}

impl From<PagerError> for MaintenanceError {
    fn from(e: PagerError) -> Self {
        MaintenanceError::Io(BufferPoolError::Io(e))
    }
}

// ──────────────── inspect: estadísticas de almacenamiento ────────────────

/// Estadísticas de almacenamiento producidas por [`inspect`].
///
/// Todas las métricas se calculan **leyendo el contenido real** de cada
/// página asignada (no se fían del `free_space` declarado en la cabecera,
/// que puede estar desactualizado tras updates). Esto permite usar `inspect`
/// antes y después de un `compact` para verificar la recuperación de espacio.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StorageStats {
    /// Total de páginas que el pager puede direccionar (tamaño del fichero).
    pub total_pages: u32,
    /// Páginas asignadas (no en la free list).
    pub allocated_pages: u32,
    /// Páginas en la free list (asignadas alguna vez, ahora reutilizables).
    pub free_pages: u32,
    /// Páginas de datos (`PageType::Data`).
    pub data_pages: u32,
    /// Páginas de metadatos (`PageType::Meta`).
    pub meta_pages: u32,
    /// Tamaño total del fichero en bytes (`total_pages * PAGE_SIZE`).
    pub bytes_on_disk: u64,
    /// Bytes efectivamente usados (PageHeader + records / info de metapágina).
    pub bytes_used: u64,
    /// Bytes libres dentro de páginas asignadas (`PAGE_SIZE - bytes_used`).
    pub bytes_free: u64,
    /// Número total de records en todas las Data pages.
    pub total_records: u64,
}

impl StorageStats {
    /// Ratio de fragmentación: `bytes_free / bytes_on_disk`.
    ///
    /// 0.0 = sin espacio libre; 1.0 = todo el fichero está vacío.
    /// Un valor alto tras muchas escrituras/borrados indica que un `compact`
    /// puede recuperar espacio dentro de las páginas.
    pub fn fragmentation_ratio(&self) -> f64 {
        if self.bytes_on_disk == 0 {
            0.0
        } else {
            self.bytes_free as f64 / self.bytes_on_disk as f64
        }
    }

    /// Ratio de utilización: `bytes_used / bytes_on_disk`.
    ///
    /// Complemento de `fragmentation_ratio`: cuánto del fichero son datos
    /// reales frente a huecos.
    pub fn utilization(&self) -> f64 {
        if self.bytes_on_disk == 0 {
            0.0
        } else {
            self.bytes_used as f64 / self.bytes_on_disk as f64
        }
    }
}

/// Calcula estadísticas de almacenamiento recorriendo todas las páginas
/// asignadas del pager.
///
/// Lee cada página vía el pager subyacente (`pool.pager_mut()`) sin cachear
/// en el buffer pool: el mantenimiento es offline, no necesita calentar la
/// caché, y así evitamos expulsar páginas útiles durante el barrido.
///
/// La página 0 se cuenta como `Meta`; el resto se intenta decodificar como
/// `SlottedPage` (Data). Las páginas libres (no asignadas) se cuentan en
/// `free_pages` pero no se inspeccionan.
pub fn inspect<P: Pager>(pool: &mut BufferPool<P>) -> Result<StorageStats, MaintenanceError> {
    let total_pages = pool.pager().num_pages();
    let mut stats = StorageStats {
        total_pages,
        bytes_on_disk: total_pages as u64 * PAGE_SIZE as u64,
        ..Default::default()
    };

    let mut buf = [0u8; PAGE_SIZE];
    for id in 0..total_pages {
        if !pool.pager().is_allocated(id) {
            stats.free_pages += 1;
            continue;
        }
        stats.allocated_pages += 1;
        // Leer crudo vía el pager (sin pasar por el pool).
        pool.pager_mut().read(id, &mut buf)?;

        if id == 0 {
            // Metapágina.
            match MetaPage::decode(&buf) {
                Ok(_meta) => {
                    stats.meta_pages += 1;
                    stats.bytes_used += (PageHeader::SIZE + MetaPage::INFO_SIZE) as u64;
                    stats.bytes_free += (PAGE_SIZE - PageHeader::SIZE - MetaPage::INFO_SIZE) as u64;
                }
                Err(_) => {
                    // Metapágina corrupta: la contamos como asignada pero sin
                    // datos usables (inspect es tolerante; check la reporta).
                    stats.meta_pages += 1;
                    stats.bytes_free += PAGE_SIZE as u64;
                }
            }
            continue;
        }

        // Data page.
        match SlottedPage::decode(&buf) {
            Ok(sp) => {
                stats.data_pages += 1;
                let used_records: usize = sp.records().iter().map(|r| r.len()).sum();
                stats.total_records += sp.records().len() as u64;
                let used = PageHeader::SIZE + used_records;
                stats.bytes_used += used as u64;
                stats.bytes_free += (PAGE_SIZE - used) as u64;
            }
            Err(_) => {
                // Página Data no decodificable: la contamos como data pero
                // sin records (inspect es tolerante; check reportará el issue).
                stats.data_pages += 1;
                stats.bytes_free += PAGE_SIZE as u64;
            }
        }
    }

    Ok(stats)
}

// ──────────────── check: verificación de integridad ────────────────

/// Tipo de problema de integridad detectado por [`check`] en una página.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    /// El magic (bytes 0-1 del header) no encaja con ningún `PageType`
    /// conocido (`0xDA` Data o `0xFE` Meta).
    BadMagic { expected: u8, got: u8 },
    /// El `page_id` guardado en el header no coincide con el offset físico
    /// de la página en el fichero. Indica una página movida o reescrita en
    /// el lugar equivocado.
    PageIdMismatch { header_says: u32, actual: u32 },
    /// El `free_space` declarado en el header no coincide con el real
    /// (calculado a partir de los records). Típico tras un crash a mitad
    /// de un update/delete.
    FreeSpaceMismatch { declared: u16, actual: u16 },
    /// Un record aparece truncado o el contador `num_records` apunta fuera
    /// de la página.
    RecordTruncated,
    /// La página no se pudo decodificar en absoluto (caso genérico).
    Undecodable(String),
}

/// Un problema de integridad localizado en una página concreta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityIssue {
    pub page_id: PageId,
    pub kind: IssueKind,
}

/// Resultado de [`check`]: lista de issues página a página.
#[derive(Debug, Clone, Default)]
pub struct CheckReport {
    /// Número de páginas asignadas verificadas.
    pub pages_checked: u32,
    /// Problemas detectados (vacío = base sana).
    pub issues: Vec<IntegrityIssue>,
}

impl CheckReport {
    /// `true` si no se detectó ningún problema.
    pub fn ok(&self) -> bool {
        self.issues.is_empty()
    }

    /// Número de problemas detectados.
    pub fn issue_count(&self) -> usize {
        self.issues.len()
    }
}

/// Verifica invariantes estructurales de todas las páginas asignadas.
///
/// Invariantes comprobadas (caps 11-12):
///   1. Toda página asignada tiene un `PageHeader` con magic válido
///      (`0xDA` Data o `0xFE` Meta) y `bytes[0] == bytes[1]`.
///   2. El `page_id` del header coincide con el offset físico (`id`).
///   3. Para Data pages, los records decodifican sin truncado
///      (`SlottedPage::decode` OK).
///   4. El `free_space` declarado en el header coincide con el real
///      (`PAGE_SIZE - header - records`).
///
/// La página 0 (metapágina) se valida con `MetaPage::decode`. Las páginas
/// libres no se verifican (contienen ceros por diseño).
///
/// Es **read-only**: no modifica nada. Para reparar `free_space` desactualizado
/// usar [`repack_page`] o [`compact`].
pub fn check<P: Pager>(pool: &mut BufferPool<P>) -> Result<CheckReport, MaintenanceError> {
    let total_pages = pool.pager().num_pages();
    let mut report = CheckReport::default();
    let mut buf = [0u8; PAGE_SIZE];

    for id in 0..total_pages {
        if !pool.pager().is_allocated(id) {
            continue;
        }
        report.pages_checked += 1;
        pool.pager_mut().read(id, &mut buf)?;

        // Invariante 1 + 2: decodificar el header crudo.
        let header_bytes: [u8; PageHeader::SIZE] = match buf[..PageHeader::SIZE].try_into() {
            Ok(b) => b,
            Err(_) => {
                // Imposible (PAGE_SIZE > 10), pero defensivo.
                report.issues.push(IntegrityIssue {
                    page_id: id,
                    kind: IssueKind::Undecodable("header slice".into()),
                });
                continue;
            }
        };
        match PageHeader::decode(&header_bytes) {
            Err(reason) => {
                // Distinguir bad magic de otro error de decode.
                let magic = buf[0];
                if magic != 0xDA && magic != 0xFE {
                    report.issues.push(IntegrityIssue {
                        page_id: id,
                        kind: IssueKind::BadMagic {
                            expected: if id == 0 { 0xFE } else { 0xDA },
                            got: magic,
                        },
                    });
                } else {
                    report.issues.push(IntegrityIssue {
                        page_id: id,
                        kind: IssueKind::Undecodable(reason),
                    });
                }
                continue;
            }
            Ok(header) => {
                // Invariante 2: page_id coincide con el offset.
                if header.page_id != id {
                    report.issues.push(IntegrityIssue {
                        page_id: id,
                        kind: IssueKind::PageIdMismatch {
                            header_says: header.page_id,
                            actual: id,
                        },
                    });
                }
            }
        }

        // Invariantes 3 + 4 específicas por tipo.
        if id == 0 {
            // Metapágina.
            if let Err(reason) = MetaPage::decode(&buf) {
                report.issues.push(IntegrityIssue {
                    page_id: id,
                    kind: IssueKind::Undecodable(reason),
                });
            }
            // La metapágina no tiene records ni free_space meaningful para
            // reportar (su "info" es fija); la validación de decode basta.
        } else {
            match SlottedPage::decode(&buf) {
                Ok(sp) => {
                    // Invariante 4: free_space declarado vs real.
                    let actual_free = sp.free_space() as u16;
                    if sp.header.free_space != actual_free {
                        report.issues.push(IntegrityIssue {
                            page_id: id,
                            kind: IssueKind::FreeSpaceMismatch {
                                declared: sp.header.free_space,
                                actual: actual_free,
                            },
                        });
                    }
                }
                Err(reason) => {
                    report.issues.push(IntegrityIssue {
                        page_id: id,
                        kind: IssueKind::RecordTruncated,
                    });
                    // Razón detallada por si hace falta depurar:
                    let _ = reason; // ya cubierta por la variante
                }
            }
        }
    }

    Ok(report)
}

// ──────────────── repack_page / compact: reescritura in-place ────────────────

/// Resultado de repackear una sola página con [`repack_page`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepackResult {
    /// Página repackeada.
    pub page_id: PageId,
    /// Bytes libres según el header ANTES del repack (posiblemente erróneo).
    pub free_before: u32,
    /// Bytes libres reales DESPUÉS del repack (siempre correcto).
    pub free_after: u32,
    /// Diferencia corregida. Positiva = el header sobreestimaba el espacio
    /// usado (metadatos desactualizados); el repack los ha alineado con la
    /// realidad. Cero = página ya consistente.
    pub bytes_reclaimed: u32,
    /// `true` si la página se modificó (header corregido o bytes basura
    /// limpiados).
    pub modified: bool,
}

/// Reescribe una Data page in-place: re-codifica su `SlottedPage` de forma
/// que (a) los records queden consecutivos sin huecos, (b) `free_space` del
/// header refleje el espacio real y (c) los bytes tras el último record se
/// pongan a cero.
///
/// NO mueve la página (el `PageId` se respeta: es un offset físico) y NO
/// elimina records. Es la unidad atómica de [`compact`].
///
/// Si la página es la metapágina (id 0) o no es una Data page válida,
/// devuelve `MaintenanceError` (no se repackea metadatos).
pub fn repack_page<P: Pager>(
    pool: &mut BufferPool<P>,
    page_id: PageId,
) -> Result<RepackResult, MaintenanceError> {
    if !pool.pager().is_allocated(page_id) {
        return Err(MaintenanceError::PageNotAllocated(page_id));
    }
    if page_id == 0 {
        // La metapágina tiene layout fijo; no se repackea.
        return Err(MaintenanceError::BadPageType {
            page_id,
            expected: 0xDA,
            got: 0xFE,
        });
    }

    // Leer crudo.
    let mut buf = [0u8; PAGE_SIZE];
    pool.pager_mut().read(page_id, &mut buf)?;

    let sp = SlottedPage::decode(&buf)
        .map_err(|reason| MaintenanceError::DecodeFailed { page_id, reason })?;
    let free_before = sp.header.free_space as usize;
    let free_after_real = sp.free_space();

    // Re-codificar: SlottedPage::encode ya empaqueta records consecutivos y
    // rellena con ceros el resto. Para que el header quede consistente,
    // construimos una SlottedPage con el free_space correcto.
    let mut repacked = SlottedPage::new(page_id, PageType::Data);
    for rec in sp.records() {
        // insert devuelve Option<usize>; si no cabe (no debería, cabían
        // antes), algo está corrupto.
        repacked.insert(rec).ok_or(MaintenanceError::DecodeFailed {
            page_id,
            reason: "record no cabe tras repack (corrupción)".into(),
        })?;
    }

    let modified =
        free_before != free_after_real || repacked.header.free_space as usize != free_before;

    // Escribir de vuelta al pager.
    let encoded = repacked.encode();
    pool.pager_mut().write(page_id, &encoded)?;

    Ok(RepackResult {
        page_id,
        free_before: free_before as u32,
        free_after: free_after_real as u32,
        bytes_reclaimed: free_before.abs_diff(free_after_real) as u32,
        modified,
    })
}

/// Resultado de [`compact`]: repack masivo de todas las Data pages.
#[derive(Debug, Clone, Default)]
pub struct CompactReport {
    /// Páginas Data efectivamente repackeadas.
    pub pages_repacked: u32,
    /// Páginas Data que se saltaron (no decodificables → no se tocan).
    pub pages_skipped: u32,
    /// Suma de `bytes_reclaimed` de cada `repack_page`.
    pub bytes_reclaimed: u64,
    /// Estadísticas ANTES del compact (para comparar).
    pub stats_before: StorageStats,
    /// Estadísticas DESPUÉS del compact.
    pub stats_after: StorageStats,
}

/// Repackea todas las Data pages asignadas del pager, una a una.
///
/// Es la operación de mantenimiento completa detrás de `liradb compact`:
///
///   1. `inspect` antes → `stats_before`.
///   2. Para cada página asignada (excepto la 0): `repack_page`. Si una
///      página no decodifica, se cuenta como `pages_skipped` y se deja
///      intacta (no se corrige corrupción estructural; eso es `check` +
///      decisión humana).
///   3. `inspect` después → `stats_after`.
///
/// **No reduce el tamaño del fichero** (vacuum/truncate): los `PageId` son
/// offsets físicos, así que mover páginas rompería CSR, índices y punteros
/// internos. Lo que sí hace es **recuperar espacio dentro de las páginas**
/// alineando `free_space` con la realidad y limpiando bytes basura.
pub fn compact<P: Pager>(pool: &mut BufferPool<P>) -> Result<CompactReport, MaintenanceError> {
    let stats_before = inspect(pool)?;
    let total_pages = pool.pager().num_pages();
    let mut report = CompactReport {
        stats_before,
        ..Default::default()
    };

    for id in 1..total_pages {
        if !pool.pager().is_allocated(id) {
            continue;
        }
        match repack_page(pool, id) {
            Ok(res) => {
                report.pages_repacked += 1;
                report.bytes_reclaimed += res.bytes_reclaimed as u64;
            }
            Err(MaintenanceError::DecodeFailed { .. }) => {
                // Página corrupta: la saltamos sin tocar (check la reporta).
                report.pages_skipped += 1;
            }
            // Otros errores (PageNotAllocated, BadPageType, Io) sí escalan:
            // algo estructural va mal y compact no debe ignorarlo.
            Err(e) => return Err(e),
        }
    }

    // Sincronizar para que stats_after lea lo escrito.
    pool.pager_mut().sync()?;
    report.stats_after = inspect(pool)?;
    Ok(report)
}

// ──────────────── Tests del cap 16 ────────────────

#[cfg(test)]
mod tests_maintenance {
    use super::*;
    use crate::{BufferPool, FilePager, Pager, PagerError};
    use std::error::Error;

    /// Pager en memoria para tests (igual que en caps 13/15).
    #[derive(Debug)]
    struct TmpPager {
        pages: Vec<Option<[u8; PAGE_SIZE]>>,
        free_list: Vec<PageId>,
    }

    impl TmpPager {
        fn new_with_meta() -> Self {
            let mut p = Self {
                pages: vec![Some([0u8; PAGE_SIZE])],
                free_list: Vec::new(),
            };
            // Inicializar la metapágina (página 0) con un MetaPage válido.
            let meta = MetaPage::new();
            p.pages[0] = Some(meta.encode());
            p
        }
    }

    impl Pager for TmpPager {
        fn allocate(&mut self) -> Result<PageId, PagerError> {
            if let Some(id) = self.free_list.pop() {
                self.pages[id as usize] = Some([0u8; PAGE_SIZE]);
                return Ok(id);
            }
            let id = self.pages.len() as PageId;
            self.pages.push(Some([0u8; PAGE_SIZE]));
            Ok(id)
        }
        fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError> {
            if page.len() != PAGE_SIZE {
                return Err(PagerError::BadBufferSize {
                    expected: PAGE_SIZE,
                    got: page.len(),
                });
            }
            let p = self.pages.get(id as usize).ok_or(PagerError::OutOfRange {
                requested: id,
                num_pages: self.pages.len() as u32,
            })?;
            let p = p.as_ref().ok_or(PagerError::FreePage(id))?;
            page.copy_from_slice(p);
            Ok(())
        }
        fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError> {
            if page.len() != PAGE_SIZE {
                return Err(PagerError::BadBufferSize {
                    expected: PAGE_SIZE,
                    got: page.len(),
                });
            }
            let num_pages = self.pages.len() as u32;
            let slot = self
                .pages
                .get_mut(id as usize)
                .ok_or(PagerError::OutOfRange {
                    requested: id,
                    num_pages,
                })?;
            if slot.is_none() {
                return Err(PagerError::FreePage(id));
            }
            *slot = Some([0u8; PAGE_SIZE]);
            slot.as_mut().unwrap().copy_from_slice(page);
            Ok(())
        }
        fn sync(&mut self) -> Result<(), PagerError> {
            Ok(())
        }
        fn num_pages(&self) -> u32 {
            self.pages.len() as u32
        }
        fn free(&mut self, id: PageId) -> Result<(), PagerError> {
            if id as usize >= self.pages.len() {
                return Err(PagerError::OutOfRange {
                    requested: id,
                    num_pages: self.pages.len() as u32,
                });
            }
            if self.free_list.contains(&id) {
                return Err(PagerError::FreePage(id));
            }
            self.pages[id as usize] = None;
            self.free_list.push(id);
            Ok(())
        }
        fn is_allocated(&self, id: PageId) -> bool {
            self.pages
                .get(id as usize)
                .map(|s| s.is_some())
                .unwrap_or(false)
        }
    }

    /// Construye un pool con N Data pages, cada una con `records_per_page`
    /// records de `record_len` bytes. Devuelve el pool listo para inspeccionar.
    fn pool_with_data_pages(
        n_data: u32,
        records_per_page: usize,
        record_len: usize,
    ) -> BufferPool<TmpPager> {
        let pager = TmpPager::new_with_meta();
        let mut pool = BufferPool::new(pager, 8);
        for page_idx in 0..n_data {
            let page_id = pool.pager_mut().allocate().unwrap();
            assert_eq!(
                page_id,
                page_idx + 1,
                "la primera Data page debe ser la página 1"
            );
            let mut sp = SlottedPage::new(page_id, PageType::Data);
            for _ in 0..records_per_page {
                let rec = vec![0xA5u8; record_len];
                sp.insert(&rec).expect("record debe caber");
            }
            let encoded = sp.encode();
            pool.pager_mut().write(page_id, &encoded).unwrap();
        }
        pool
    }

    // ─── StorageStats ───

    #[test]
    fn stats_ratios_empty_disk() {
        let s = StorageStats::default();
        assert_eq!(s.fragmentation_ratio(), 0.0);
        assert_eq!(s.utilization(), 0.0);
    }

    #[test]
    fn stats_ratios_basic() {
        let s = StorageStats {
            total_pages: 2,
            bytes_on_disk: 2 * PAGE_SIZE as u64,
            bytes_used: PAGE_SIZE as u64,
            bytes_free: PAGE_SIZE as u64,
            ..Default::default()
        };
        assert!((s.fragmentation_ratio() - 0.5).abs() < 1e-9);
        assert!((s.utilization() - 0.5).abs() < 1e-9);
    }

    // ─── inspect ───

    #[test]
    fn inspect_empty_pager_only_meta() {
        let pager = TmpPager::new_with_meta();
        let mut pool = BufferPool::new(pager, 4);
        let s = inspect(&mut pool).unwrap();
        assert_eq!(s.total_pages, 1);
        assert_eq!(s.allocated_pages, 1);
        assert_eq!(s.free_pages, 0);
        assert_eq!(s.meta_pages, 1);
        assert_eq!(s.data_pages, 0);
        assert_eq!(s.total_records, 0);
        assert_eq!(s.bytes_on_disk, PAGE_SIZE as u64);
        // Metapágina usa 10 (header) + 12 (info) = 22 bytes.
        assert_eq!(
            s.bytes_used,
            (PageHeader::SIZE + MetaPage::INFO_SIZE) as u64
        );
    }

    #[test]
    fn inspect_counts_records_and_pages() {
        // 3 Data pages, 2 records de 8 bytes cada una.
        let mut pool = pool_with_data_pages(3, 2, 8);
        let s = inspect(&mut pool).unwrap();
        assert_eq!(s.total_pages, 4); // meta + 3 data
        assert_eq!(s.allocated_pages, 4);
        assert_eq!(s.meta_pages, 1);
        assert_eq!(s.data_pages, 3);
        assert_eq!(s.total_records, 6); // 3 páginas × 2 records
        // inspect mide "bytes usados" como PageHeader::SIZE + Σ record.len()
        // (sin contar los length-prefix de 4B que SlottedPage::encode añade).
        // Esto es coherente con SlottedPage::free_space() del cap 11.
        let used_per_data = PageHeader::SIZE + 2 * 8; // header + 2 records × 8 bytes
        let expected_used =
            (PageHeader::SIZE + MetaPage::INFO_SIZE) as u64 + 3 * used_per_data as u64;
        assert_eq!(s.bytes_used, expected_used);
    }

    #[test]
    fn inspect_counts_free_pages() {
        let mut pool = pool_with_data_pages(2, 1, 4);
        // Liberar la última Data page (id 2).
        pool.pager_mut().free(2).unwrap();
        let s = inspect(&mut pool).unwrap();
        assert_eq!(s.total_pages, 3); // meta + 2 data
        assert_eq!(s.allocated_pages, 2); // meta + 1 data (la 2 está libre)
        assert_eq!(s.free_pages, 1);
        assert_eq!(s.data_pages, 1);
    }

    #[test]
    fn inspect_is_tolerant_to_corrupt_data_page() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        // Corromper la página 1 machacándola con basura (no decodifica).
        let garbage = [0xFFu8; PAGE_SIZE];
        pool.pager_mut().write(1, &garbage).unwrap();
        let s = inspect(&mut pool).unwrap();
        // Se cuenta como data page pero sin records ni bytes usados.
        assert_eq!(s.data_pages, 1);
        assert_eq!(s.total_records, 0);
        // No panic: inspect no aborta ante corrupción.
    }

    // ─── check ───

    #[test]
    fn check_clean_passer_ok() {
        let mut pool = pool_with_data_pages(3, 2, 8);
        let report = check(&mut pool).unwrap();
        assert_eq!(report.pages_checked, 4); // meta + 3 data
        assert!(report.ok(), "issues: {:?}", report.issues);
        assert_eq!(report.issue_count(), 0);
    }

    #[test]
    fn check_detects_bad_magic() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        // Corromper el magic de la página 1.
        let mut buf = [0u8; PAGE_SIZE];
        pool.pager_mut().read(1, &mut buf).unwrap();
        buf[0] = 0x11; // magic inválido
        buf[1] = 0x11;
        pool.pager_mut().write(1, &buf).unwrap();
        let report = check(&mut pool).unwrap();
        assert!(!report.ok());
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i.kind, IssueKind::BadMagic { got: 0x11, .. }))
        );
    }

    #[test]
    fn check_detects_page_id_mismatch() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        // Reescribir la página 1 con un header que dice page_id = 99.
        let mut buf = [0u8; PAGE_SIZE];
        pool.pager_mut().read(1, &mut buf).unwrap();
        // header.page_id vive en bytes 2..6 (little-endian).
        buf[2..6].copy_from_slice(&99u32.to_le_bytes());
        pool.pager_mut().write(1, &buf).unwrap();
        let report = check(&mut pool).unwrap();
        assert!(
            report.issues.iter().any(|i| matches!(
                i.kind,
                IssueKind::PageIdMismatch {
                    header_says: 99,
                    actual: 1,
                }
            )),
            "issues: {:?}",
            report.issues
        );
    }

    #[test]
    fn check_detects_free_space_mismatch() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        // Falsificar el free_space del header de la página 1.
        let mut buf = [0u8; PAGE_SIZE];
        pool.pager_mut().read(1, &mut buf).unwrap();
        // header.free_space vive en bytes 8..10 (little-endian).
        buf[8..10].copy_from_slice(&1234u16.to_le_bytes()); // valor claramente falso
        pool.pager_mut().write(1, &buf).unwrap();
        let report = check(&mut pool).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|i| matches!(i.kind, IssueKind::FreeSpaceMismatch { declared: 1234, .. })),
            "issues: {:?}",
            report.issues
        );
    }

    #[test]
    fn check_skips_free_pages() {
        let mut pool = pool_with_data_pages(2, 1, 4);
        pool.pager_mut().free(2).unwrap(); // libre → contiene ceros
        let report = check(&mut pool).unwrap();
        // La página libre NO genera un BadMagic (no se verifica).
        assert!(report.ok(), "issues: {:?}", report.issues);
        // pages_checked = meta + 1 data (la libre no cuenta).
        assert_eq!(report.pages_checked, 2);
    }

    // ─── repack_page ───

    #[test]
    fn repack_page_idempotente_on_clean_page() {
        let mut pool = pool_with_data_pages(1, 2, 8);
        let res = repack_page(&mut pool, 1).unwrap();
        // Página ya consistente → no modificada.
        assert!(!res.modified);
        assert_eq!(res.bytes_reclaimed, 0);
    }

    #[test]
    fn repack_page_corrije_free_space_corrupto() {
        let mut pool = pool_with_data_pages(1, 2, 8);
        // Falsificar free_space en la página 1.
        let mut buf = [0u8; PAGE_SIZE];
        pool.pager_mut().read(1, &mut buf).unwrap();
        buf[8..10].copy_from_slice(&1111u16.to_le_bytes());
        pool.pager_mut().write(1, &buf).unwrap();

        let res = repack_page(&mut pool, 1).unwrap();
        assert!(res.modified);
        assert_eq!(res.free_before, 1111);
        // free_after es el real recalculado por SlottedPage::free_space(),
        // que cuenta PAGE_SIZE - header - Σ record.len() (sin length-prefixes).
        let expected_free = (PAGE_SIZE - PageHeader::SIZE - 2 * 8) as u32;
        assert_eq!(res.free_after, expected_free);
        assert_eq!(res.bytes_reclaimed, 1111u32.abs_diff(expected_free));

        // Tras el repack, check ya no reporta FreeSpaceMismatch.
        let report = check(&mut pool).unwrap();
        assert!(
            report.ok(),
            "tras repack la página debe ser consistente: {:?}",
            report.issues
        );
    }

    #[test]
    fn repack_page_rechaza_meta_page() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        let r = repack_page(&mut pool, 0);
        assert!(matches!(
            r,
            Err(MaintenanceError::BadPageType {
                page_id: 0,
                expected: 0xDA,
                got: 0xFE,
            })
        ));
    }

    #[test]
    fn repack_page_rechaza_no_asignada() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        let r = repack_page(&mut pool, 99);
        assert!(matches!(r, Err(MaintenanceError::PageNotAllocated(99))));
    }

    #[test]
    fn repack_page_rechaza_corrupta() {
        let mut pool = pool_with_data_pages(1, 1, 4);
        let garbage = [0xFFu8; PAGE_SIZE];
        pool.pager_mut().write(1, &garbage).unwrap();
        let r = repack_page(&mut pool, 1);
        assert!(matches!(
            r,
            Err(MaintenanceError::DecodeFailed { page_id: 1, .. })
        ));
    }

    // ─── compact ───

    #[test]
    fn compact_repackea_todas_las_data_pages() {
        let mut pool = pool_with_data_pages(3, 2, 8);
        let report = compact(&mut pool).unwrap();
        assert_eq!(report.pages_repacked, 3);
        assert_eq!(report.pages_skipped, 0);
        // Páginas limpias → bytes_reclaimed == 0.
        assert_eq!(report.bytes_reclaimed, 0);
        assert!(report.stats_before.total_records == report.stats_after.total_records);
    }

    #[test]
    fn compact_corrige_free_space_y_mejora_stats() {
        let mut pool = pool_with_data_pages(3, 2, 8);
        // Corromper free_space en las páginas 1 y 3.
        for &pid in &[1u32, 3] {
            let mut buf = [0u8; PAGE_SIZE];
            pool.pager_mut().read(pid, &mut buf).unwrap();
            buf[8..10].copy_from_slice(&5000u16.to_le_bytes());
            pool.pager_mut().write(pid, &buf).unwrap();
        }
        // Antes: check reporta 2 issues.
        let before = check(&mut pool).unwrap();
        assert_eq!(
            before
                .issues
                .iter()
                .filter(|i| matches!(i.kind, IssueKind::FreeSpaceMismatch { .. }))
                .count(),
            2
        );

        let report = compact(&mut pool).unwrap();
        assert_eq!(report.pages_repacked, 3);
        assert!(report.bytes_reclaimed > 0);

        // Después: check limpio.
        let after = check(&mut pool).unwrap();
        assert!(after.ok(), "issues tras compact: {:?}", after.issues);
    }

    #[test]
    fn compact_salta_paginas_corruptas() {
        let mut pool = pool_with_data_pages(2, 1, 4);
        // Corromper la página 2 (basura).
        let garbage = [0xEEu8; PAGE_SIZE];
        pool.pager_mut().write(2, &garbage).unwrap();
        let report = compact(&mut pool).unwrap();
        assert_eq!(report.pages_repacked, 1); // sólo la 1
        assert_eq!(report.pages_skipped, 1); // la 2 corrupta
    }

    // ─── persistencia via FilePager ───
    //
    // NOTA: FilePager::create deja la página 0 a ceros (no inicializa una
    // MetaPage válida). Como check() verifica el magic de TODAS las páginas
    // asignadas —incluida la 0—, estos tests inicializan la metapágina
    // explícitamente, igual que hace TmpPager::new_with_meta() arriba.

    /// Crea un FilePager con la página 0 inicializada como MetaPage válida.
    fn filepool_with_meta(path: &std::path::Path, capacity: usize) -> BufferPool<FilePager> {
        let pager = FilePager::create(path).unwrap();
        let mut pool = BufferPool::new(pager, capacity);
        // Escribir una MetaPage válida en la página 0.
        let meta = MetaPage::new();
        pool.pager_mut().write(0, &meta.encode()).unwrap();
        pool
    }

    #[test]
    fn inspect_y_check_sobre_filepager_tras_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("maint.liradb");

        // Escribir 2 Data pages + metapágina.
        {
            let mut pool = filepool_with_meta(&path, 8);
            for i in 0..2u32 {
                let pid = pool.pager_mut().allocate().unwrap();
                let mut sp = SlottedPage::new(pid, PageType::Data);
                sp.insert(&[0x10 + i as u8; 16]);
                pool.pager_mut().write(pid, &sp.encode()).unwrap();
            }
            // Verificar in-memory.
            let s = inspect(&mut pool).unwrap();
            assert_eq!(s.data_pages, 2);
            assert_eq!(s.total_records, 2);
            let r = check(&mut pool).unwrap();
            assert!(r.ok(), "issues: {:?}", r.issues);
        }

        // Reabrir: la free list se pierde (documentado cap 12), pero todas
        // las páginas existen y son válidas.
        let pager2 = FilePager::open(&path).unwrap();
        let mut pool2 = BufferPool::new(pager2, 8);
        let s2 = inspect(&mut pool2).unwrap();
        assert_eq!(s2.total_pages, 3); // meta + 2 data
        assert_eq!(s2.data_pages, 2);
        assert_eq!(s2.total_records, 2);
        let r2 = check(&mut pool2).unwrap();
        assert!(r2.ok(), "issues tras reopen: {:?}", r2.issues);

        // compact debe ser idempotente y dejar la base consistente.
        let rep = compact(&mut pool2).unwrap();
        assert_eq!(rep.pages_repacked, 2);
        let r3 = check(&mut pool2).unwrap();
        assert!(r3.ok(), "issues tras compact: {:?}", r3.issues);
    }

    #[test]
    fn repack_persiste_free_space_corregido_a_disco() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("repack.liradb");

        {
            let mut pool = filepool_with_meta(&path, 8);
            let pid = pool.pager_mut().allocate().unwrap();
            let mut sp = SlottedPage::new(pid, PageType::Data);
            sp.insert(&[0xBBu8; 32]);
            pool.pager_mut().write(pid, &sp.encode()).unwrap();
            // Corromper free_space en disco.
            let mut buf = [0u8; PAGE_SIZE];
            pool.pager_mut().read(pid, &mut buf).unwrap();
            buf[8..10].copy_from_slice(&9000u16.to_le_bytes());
            pool.pager_mut().write(pid, &buf).unwrap();
        }

        // Reabrir, verificar corrupción, repackear, reabrir, verificar fix.
        {
            let pager = FilePager::open(&path).unwrap();
            let mut pool = BufferPool::new(pager, 8);
            let r = check(&mut pool).unwrap();
            assert!(!r.ok(), "debería detectar free_space corrupto");
            assert!(
                r.issues
                    .iter()
                    .any(|i| matches!(i.kind, IssueKind::FreeSpaceMismatch { declared: 9000, .. }))
            );
            let res = repack_page(&mut pool, 1).unwrap();
            assert!(res.modified);
            pool.pager_mut().sync().unwrap();
        }
        let pager2 = FilePager::open(&path).unwrap();
        let mut pool2 = BufferPool::new(pager2, 8);
        let r2 = check(&mut pool2).unwrap();
        assert!(r2.ok(), "tras repack+sync la página debe estar sana");
    }

    // ─── MaintenanceError: display, source, From ───

    #[test]
    fn maintenance_error_display_y_from_pager() {
        let e = MaintenanceError::from(PagerError::NoFreePageId);
        let s = format!("{e}");
        assert!(s.contains("maintenance io"));
        // La cadena interior viene del BufferPoolError/PagerError.
        assert!(s.contains("no free PageId"));
        // source() encadena hasta el BufferPoolError.
        let src = e.source().expect("debe tener source");
        assert!(src.to_string().contains("buffer pool io"));
    }

    #[test]
    fn maintenance_error_display_variantes() {
        let e1 = MaintenanceError::BadPageType {
            page_id: 5,
            expected: 0xDA,
            got: 0xFF,
        };
        assert!(format!("{e1}").contains("page 5"));
        assert!(format!("{e1}").contains("0xff"));

        let e2 = MaintenanceError::DecodeFailed {
            page_id: 7,
            reason: "boom".into(),
        };
        assert!(format!("{e2}").contains("page 7"));
        assert!(format!("{e2}").contains("boom"));

        let e3 = MaintenanceError::PageNotAllocated(9);
        assert!(format!("{e3}").contains("page 9 not allocated"));
    }

    #[test]
    fn maintenance_error_source_none_para_no_io() {
        let e = MaintenanceError::PageNotAllocated(3);
        assert!(e.source().is_none());
    }

    // ─── límite: pager con una sola página (meta) ───

    #[test]
    fn compact_sin_data_pages_no_op() {
        let pager = TmpPager::new_with_meta();
        let mut pool = BufferPool::new(pager, 4);
        let report = compact(&mut pool).unwrap();
        assert_eq!(report.pages_repacked, 0);
        assert_eq!(report.pages_skipped, 0);
        assert_eq!(report.bytes_reclaimed, 0);
        assert_eq!(report.stats_after.data_pages, 0);
    }
}
