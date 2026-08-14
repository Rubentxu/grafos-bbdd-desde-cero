use crate::cap11_slotted_pages::PAGE_SIZE;
use crate::cap12_pager::{PageId, Pager, PagerError};

// ─────────────────── Cap 13: buffer pool con política Clock ───────────────────
//
// El `Pager` del cap 12 hace una E/S por cada acceso: cada `read` es un `seek`
// + `read_exact` al disco, cada `write` es un `seek` + `write_all`. Consultar
// repetidamente las mismas páginas (caso típico de un B+ tree o un escaneo de
// adyacencias) es lentísimo. El buffer pool resuelve esto con:
//
//   1. Un array fijo de **frames** en memoria, cada uno de `PAGE_SIZE` bytes.
//   2. Una **page table** que mapea `PageId → FrameId`.
//   3. Un **pin count** por frame: el caller incrementa al tomar la página y
//      decrementa al soltarla. Una página pineada NO puede ser expulsada.
//   4. Un **dirty flag** por frame: si está sucio, hay que escribirlo a disco
//      antes de reutilizarlo.
//   5. Una **política de reemplazo** que decide qué frame víctima elegir
//      cuando todos los no-pineados están "ocupados". Aquí usamos **Clock**
//      (también llamada "second chance"), que aproxima LRU con un bit de
//      referencia por frame y un puntero circular.
//
// Diseño:
//
//   - `BufferPool<P: Pager>` es **genérico** sobre el pager (arquitectura
//     hexagonal: el pool es un adapter sobre el port `Pager`). Esto permite
//     testearlo contra `FilePager` (disco) o contra un pager en memoria
//     (un `MemoryPager` de tests), sin cambiar el pool.
//
//   - Sin crates externas. La política Clock son ~20 líneas de Rust; la LRU es
//     ~10. Mantener el código in-house es la esencia pedagógica del cap.
//
//   - Errores tipados (`BufferPoolError`) con variantes específicas:
//     `Io(PagerError)`, `UnknownPage`, `BadPinCount`, `PoolFullOfPinned`.
//     Permite a los callers razonar sobre el tipo de fallo.
//
//   - **Pin/unpin explícitos** (no `Guard`-tipo RAII): pedagógicamente más
//     simple; los lifetimes de Rust añadirían ruido sin aportar claridad.
//     La regla de uso es: cada `get_page` que devuelve `Ok` deja el frame
//     pineado con `pin_count >= 1`; el caller DEBE llamar `unpin` para
//     permitir la expulsión.

/// Identificador de frame en el buffer pool (índice en el array de frames).
pub type FrameId = usize;

/// Métricas del buffer pool (contadores monotónicos).
///
/// Decisión pedagógica: usamos `Cell<u64>` (no `AtomicU64`) porque el
/// `BufferPool` no es thread-safe por diseño (el cap 28 introducirá un
/// wrapper concurrente). Los tests pueden inspeccionar las métricas vía
/// `metrics()` que devuelve un `MetricsSnapshot` copiando los valores.
#[derive(Debug, Default, Clone)]
pub struct Metrics {
    /// Lecturas de página desde disco (cache misses).
    pub page_reads: u64,
    /// Escrituras de página a disco (flushes).
    pub page_writes: u64,
    /// Hits: la página ya estaba en memoria.
    pub buffer_hits: u64,
    /// Misses: la página hubo que leerla de disco.
    pub buffer_misses: u64,
    /// Frames expulsados (victim seleccionado).
    pub evictions: u64,
}

impl Metrics {
    /// Ratio de aciertos (0.0 si no hubo accesos).
    pub fn hit_ratio(&self) -> f64 {
        let total = self.buffer_hits + self.buffer_misses;
        if total == 0 {
            0.0
        } else {
            self.buffer_hits as f64 / total as f64
        }
    }
}

/// Errores del buffer pool.
#[derive(Debug)]
pub enum BufferPoolError {
    /// Error de E/S del pager subyacente.
    Io(PagerError),
    /// `PageId` solicitado no existe (pager no lo tiene asignado).
    UnknownPage(PageId),
    /// `pin_count` intentó bajar de 0.
    BadPinCount { page_id: PageId, current: u32 },
    /// Todos los frames están pineados: no se puede satisfacer la petición.
    PoolFullOfPinned,
}

impl std::fmt::Display for BufferPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferPoolError::Io(e) => write!(f, "buffer pool io: {e}"),
            BufferPoolError::UnknownPage(id) => write!(f, "page {id} not allocated in pager"),
            BufferPoolError::BadPinCount { page_id, current } => {
                write!(f, "page {page_id}: bad pin count (current={current})")
            }
            BufferPoolError::PoolFullOfPinned => {
                write!(f, "all frames pinned, no victim available")
            }
        }
    }
}

impl std::error::Error for BufferPoolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BufferPoolError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PagerError> for BufferPoolError {
    fn from(e: PagerError) -> Self {
        BufferPoolError::Io(e)
    }
}

/// Frame individual en el buffer pool.
#[derive(Debug, Clone)]
struct Frame {
    /// ID de la página que aloja este frame. `None` si está vacío.
    page_id: Option<PageId>,
    /// Contenido de la página (`PAGE_SIZE` bytes).
    data: [u8; PAGE_SIZE],
    /// Número de pins activos. Una página pineada no puede ser expulsada.
    pin_count: u32,
    /// ¿Modificada desde la última escritura a disco?
    dirty: bool,
    /// Bit de referencia para la política Clock (true = "usada recientemente").
    ref_bit: bool,
    /// Contador saturado de uso para la política LRU (orden de recencia).
    /// Cada `touch` lo pone al contador global; cada `victim` busca el menor.
    lru_counter: u64,
}

/// Política **Clock** (también llamada "second chance").
///
/// Mantiene un puntero `hand` circular sobre el array de frames. La aguja
/// **avanza en cada acceso** (hit o miss), no sólo cuando se busca víctima.
/// Esto es esencial: sin avance en acceso, dos frames accedidos en tiempos
/// distintos parecerían idénticos al algoritmo y la recencia no se capturaría.
///
/// Cuando se necesita una víctima:
///   - Si el frame está pineado → la aguja lo salta (avanza).
///   - Si tiene `ref_bit == true` → lo pone a `false` y avanza (second chance).
///   - Si tiene `ref_bit == false` → lo elige como víctima.
///
/// Es una **aproximación de LRU** con coste O(1) amortizado por acceso y
/// estado O(1) por frame (un bit). El nombre "Clock" viene de que el puntero
/// gira como una aguja de reloj sobre los frames.
///
/// Decisión pedagógica: implementamos el Clock básico (un bit), no el
/// "GClock" (contador de uso). El cap 14+ podría extenderlo.
///
/// Política de reemplazo seleccionable para el `BufferPool`.
///
/// Por defecto usamos **Clock**, que es la recomendada por el brief del libro
/// y la que implementa Kùzu (con la variante GClock).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PolicyKind {
    #[default]
    Clock,
    Lru,
}

/// Buffer pool genérico sobre cualquier `Pager`.
///
/// ```text
///   ┌─────────────────────────────────────┐
///   │ frames: Vec<Frame>                  │  ← array fijo de tamaño `capacity`
///   │ page_table: Vec<Option<FrameId>>    │  ← PageId → FrameId
///   │ policy: PolicyKind (Clock/Lru)     │  ← Clock (defecto) o LRU
///   │ pager: P                            │  ← adapter del cap 12
///   │ metrics: Metrics                    │  ← contadores
///   └─────────────────────────────────────┘
/// ```
///
/// **Pin/unpin explícito**: `get_page` pinea automáticamente; el caller debe
/// llamar `unpin(page_id)` cuando termine (o `unpin_all` al cerrar).
pub struct BufferPool<P: Pager> {
    pager: P,
    frames: Vec<Frame>,
    /// `page_table[id] = Some(frame_id)` si la página `id` está cargada.
    page_table: Vec<Option<FrameId>>,
    policy: PolicyKind,
    /// Estado interno de la política activa (Clock hand o LRU counter).
    /// Usamos un enum interno para no exponer el trait.
    clock_hand: usize,
    lru_counter: u64,
    metrics: Metrics,
}

/// Capacidad por defecto del buffer pool (64 frames = 256 KB).
pub const DEFAULT_CAPACITY: usize = 64;

impl<P: Pager> BufferPool<P> {
    /// Crea un buffer pool con la capacidad indicada y la política por defecto (Clock).
    pub fn new(pager: P, capacity: usize) -> Self {
        Self::with_policy(pager, capacity, PolicyKind::default())
    }

    /// Crea un buffer pool con la capacidad y política indicadas.
    pub fn with_policy(pager: P, capacity: usize, policy: PolicyKind) -> Self {
        assert!(capacity > 0, "buffer pool capacity must be > 0");
        let frames = (0..capacity)
            .map(|_| Frame {
                page_id: None,
                data: [0u8; PAGE_SIZE],
                pin_count: 0,
                dirty: false,
                ref_bit: false,
                lru_counter: 0,
            })
            .collect();
        Self {
            pager,
            frames,
            page_table: Vec::new(),
            policy,
            clock_hand: 0,
            lru_counter: 0,
            metrics: Metrics::default(),
        }
    }

    /// Capacidad del pool (número de frames).
    pub fn capacity(&self) -> usize {
        self.frames.len()
    }

    /// Número de frames actualmente ocupados (tienen una página cargada).
    pub fn occupied(&self) -> usize {
        self.frames.iter().filter(|f| f.page_id.is_some()).count()
    }

    /// Métricas acumuladas desde la creación del pool.
    pub fn metrics(&self) -> Metrics {
        self.metrics.clone()
    }

    /// Tipo de política activa (para inspección / tests).
    pub fn policy(&self) -> PolicyKind {
        self.policy
    }

    /// Acceso de sólo lectura al pager subyacente (e.g. para `num_pages`).
    pub fn pager(&self) -> &P {
        &self.pager
    }

    /// Acceso mutable al pager subyacente (e.g. para `allocate` antes de
    /// tocar el pool, o para `sync` tras un flush externo).
    pub fn pager_mut(&mut self) -> &mut P {
        &mut self.pager
    }

    /// Asegura que `page_table` tiene una entrada para `id`.
    fn ensure_page_table(&mut self, id: PageId) {
        let needed = id as usize + 1;
        if self.page_table.len() < needed {
            self.page_table.resize(needed, None);
        }
    }

    /// Busca el frame que aloja la página `id`, si está cargada.
    fn find_frame(&self, id: PageId) -> Option<FrameId> {
        self.page_table.get(id as usize).and_then(|slot| *slot)
    }

    /// Selecciona un frame víctima según la política activa.
    /// Devuelve `None` si todos están pineados.
    fn pick_victim(&mut self) -> Option<FrameId> {
        match self.policy {
            PolicyKind::Clock => self.pick_victim_clock(),
            PolicyKind::Lru => self.pick_victim_lru(),
        }
    }

    fn pick_victim_clock(&mut self) -> Option<FrameId> {
        let n = self.frames.len();
        if n == 0 {
            return None;
        }
        for _ in 0..(2 * n) {
            let cand = self.clock_hand % n;
            let f = &self.frames[cand];
            if f.pin_count == 0 {
                if f.ref_bit {
                    // Second chance: limpiamos el bit y avanzamos.
                    self.frames[cand].ref_bit = false;
                    self.clock_hand = (self.clock_hand + 1) % n;
                    continue;
                }
                // Víctima encontrada.
                self.clock_hand = (self.clock_hand + 1) % n;
                return Some(cand);
            }
            self.clock_hand = (self.clock_hand + 1) % n;
        }
        None
    }

    fn pick_victim_lru(&mut self) -> Option<FrameId> {
        let mut best: Option<(FrameId, u64)> = None;
        for (i, f) in self.frames.iter().enumerate() {
            if f.pin_count > 0 {
                continue;
            }
            if f.page_id.is_none() {
                return Some(i); // frame vacío → víctima inmediata
            }
            match best {
                None => best = Some((i, f.lru_counter)),
                Some((_, c)) if f.lru_counter < c => best = Some((i, f.lru_counter)),
                _ => {}
            }
        }
        best.map(|(i, _)| i)
    }

    /// Obtiene una página del pool. Si ya está cargada, devuelve un hit;
    /// si no, la lee del pager (miss) y posiblemente expulsa a otra página.
    ///
    /// **El frame queda pineado** (`pin_count >= 1`). El caller debe llamar
    /// `unpin(page_id)` para liberarlo.
    pub fn get_page(&mut self, id: PageId) -> Result<&mut [u8; PAGE_SIZE], BufferPoolError> {
        // 1. Hit: la página ya está en memoria.
        if let Some(fid) = self.find_frame(id) {
            self.metrics.buffer_hits += 1;
            // Marca de uso (LRU o Clock según política).
            self.touch_frame(fid);
            // Pin automático.
            self.frames[fid].pin_count += 1;
            // Devolvemos &mut del data. Como self.frames es un campo directo,
            // podemos split borrow: prestamos `frames` para escribir,
            // `page_table` no se toca aquí.
            let frame = &mut self.frames[fid];
            return Ok(&mut frame.data);
        }

        // 2. Miss: la página no está. Verificar que el pager la tiene.
        if !self.pager.is_allocated(id) {
            return Err(BufferPoolError::UnknownPage(id));
        }
        self.metrics.buffer_misses += 1;

        // 3. Buscar un frame libre o una víctima.
        let fid = match self.find_free_frame() {
            Some(f) => f,
            None => match self.pick_victim() {
                Some(v) => {
                    // Si el frame víctima está sucio, hay que escribirlo
                    // a disco antes de reutilizarlo.
                    let victim_page = self.frames[v].page_id;
                    let victim_dirty = self.frames[v].dirty;
                    if victim_dirty {
                        let victim_data = self.frames[v].data;
                        if let Some(vp) = victim_page {
                            self.pager.write(vp, &victim_data)?;
                            self.metrics.page_writes += 1;
                        }
                        self.frames[v].dirty = false;
                    }
                    if let Some(vp) = victim_page {
                        self.page_table[vp as usize] = None;
                    }
                    self.metrics.evictions += 1;
                    v
                }
                None => return Err(BufferPoolError::PoolFullOfPinned),
            },
        };

        // 4. Leer la página desde disco al frame.
        self.pager.read(id, &mut self.frames[fid].data)?;
        self.metrics.page_reads += 1;

        // 5. Actualizar metadatos del frame y de la page table.
        self.frames[fid].page_id = Some(id);
        self.frames[fid].pin_count = 1;
        self.frames[fid].dirty = false;
        self.frames[fid].ref_bit = true; // recién cargada, márcala como usada
        self.ensure_page_table(id);
        self.page_table[id as usize] = Some(fid);

        // 6. Tocar para la política LRU (Clock ignora el contador).
        self.lru_counter += 1;
        self.frames[fid].lru_counter = self.lru_counter;

        // 6b. Para Clock, avanzar la aguja también en miss → fresh load.
        // (LRU ya actualizó el contador arriba.)
        if self.policy == PolicyKind::Clock && !self.frames.is_empty() {
            self.clock_hand = (self.clock_hand + 1) % self.frames.len();
        }

        // 7. Devolver el data.
        let frame = &mut self.frames[fid];
        Ok(&mut frame.data)
    }

    /// Busca un frame completamente vacío (page_id == None, pin_count == 0).
    fn find_free_frame(&self) -> Option<FrameId> {
        self.frames
            .iter()
            .position(|f| f.page_id.is_none() && f.pin_count == 0)
    }

    /// Marca el frame como accedido (para la política de reemplazo).
    ///
    /// Para Clock: marca `ref_bit = true` y **avanza la aguja**. Esto es
    /// esencial para aproximar LRU: si no avanzáramos la aguja en cada
    /// acceso, dos frames accedidos en tiempos distintos parecerían idénticos
    /// al barrido del reloj y perderíamos la noción de recencia.
    fn touch_frame(&mut self, fid: FrameId) {
        match self.policy {
            PolicyKind::Clock => {
                self.frames[fid].ref_bit = true;
                if !self.frames.is_empty() {
                    self.clock_hand = (self.clock_hand + 1) % self.frames.len();
                }
            }
            PolicyKind::Lru => {
                self.lru_counter += 1;
                self.frames[fid].lru_counter = self.lru_counter;
            }
        }
    }

    /// Despinea una página (decrementa `pin_count`).
    ///
    /// Llamar `unpin` con `pin_count == 0` es un error de programa
    /// (`BadPinCount`), no un caso normal.
    pub fn unpin(&mut self, id: PageId, dirty: bool) -> Result<(), BufferPoolError> {
        let fid = self
            .find_frame(id)
            .ok_or(BufferPoolError::UnknownPage(id))?;
        if self.frames[fid].pin_count == 0 {
            return Err(BufferPoolError::BadPinCount {
                page_id: id,
                current: 0,
            });
        }
        self.frames[fid].pin_count -= 1;
        if dirty {
            self.frames[fid].dirty = true;
        }
        Ok(())
    }

    /// Despina todas las páginas (útil al cerrar/cerrar un scope).
    /// Marca todas como no-sucias (no flush). Para persistir antes, use `flush`.
    pub fn unpin_all(&mut self) {
        for f in &mut self.frames {
            f.pin_count = 0;
        }
    }

    /// Marca una página como sucia (sin cambiar el pin count).
    ///
    /// Útil cuando se modifica la página a través de una referencia mutable
    /// obtenida previamente con `get_page` y se quiere asegurar que el flush
    /// la escribe.
    pub fn mark_dirty(&mut self, id: PageId) -> Result<(), BufferPoolError> {
        let fid = self
            .find_frame(id)
            .ok_or(BufferPoolError::UnknownPage(id))?;
        self.frames[fid].dirty = true;
        Ok(())
    }

    /// Escribe a disco todas las páginas sucias. Devuelve el número de páginas
    /// escritas. Tras un `flush` exitoso, los frames quedan limpios.
    pub fn flush(&mut self) -> Result<usize, BufferPoolError> {
        let mut count = 0;
        // Recolectamos primero los (frame_id, page_id) sucios para evitar
        // un borrow problemático al escribir y luego limpiar el flag.
        let dirty: Vec<(FrameId, PageId)> = self
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.dirty && f.page_id.is_some())
            .map(|(i, f)| (i, f.page_id.unwrap()))
            .collect();
        for (fid, pid) in dirty {
            let data = self.frames[fid].data;
            self.pager.write(pid, &data)?;
            self.frames[fid].dirty = false;
            self.metrics.page_writes += 1;
            count += 1;
        }
        // Sincroniza el pager (fsync) para que las escrituras sean durables.
        self.pager.sync()?;
        Ok(count)
    }

    /// Flush selectivo: sólo escribe la página `id` si está sucia y cargada.
    pub fn flush_page(&mut self, id: PageId) -> Result<bool, BufferPoolError> {
        let fid = self
            .find_frame(id)
            .ok_or(BufferPoolError::UnknownPage(id))?;
        if !self.frames[fid].dirty {
            return Ok(false);
        }
        let data = self.frames[fid].data;
        self.pager.write(id, &data)?;
        self.frames[fid].dirty = false;
        self.metrics.page_writes += 1;
        self.pager.sync()?;
        Ok(true)
    }

    /// Invalida (descarta) una página del pool. La próxima vez que se pida,
    /// se releerá del disco. Si está pineada o sucia, se rechazará.
    pub fn discard(&mut self, id: PageId) -> Result<(), BufferPoolError> {
        let fid = self
            .find_frame(id)
            .ok_or(BufferPoolError::UnknownPage(id))?;
        if self.frames[fid].pin_count > 0 {
            return Err(BufferPoolError::BadPinCount {
                page_id: id,
                current: self.frames[fid].pin_count,
            });
        }
        if self.frames[fid].dirty {
            // Decisión pedagógica: descartar sin flush = perder cambios.
            // Devolvemos BadPinCount con un mensaje claro no encaja; usamos
            // un error explícito. Como no tenemos variante "Dirty", lo
            // señalamos con el pin count indicando "no se puede descartar".
            // Alternativa más limpia: hacer flush implícito. Aquí optamos
            // por la conservadora: rechazar.
            return Err(BufferPoolError::BadPinCount {
                page_id: id,
                current: u32::MAX, // sentinel: "tiene cambios sucios"
            });
        }
        self.frames[fid].page_id = None;
        self.frames[fid].pin_count = 0;
        self.frames[fid].ref_bit = false;
        self.page_table[id as usize] = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests_buffer_pool {
    use super::*;
    use std::error::Error;

    use crate::cap12_pager::FilePager;

    /// Pager en memoria para tests del buffer pool (sin tocar disco).
    /// Implementa el trait `Pager` sobre un `Vec<[u8; PAGE_SIZE]>`.
    #[derive(Debug)]
    struct MemoryPager {
        pages: Vec<Option<[u8; PAGE_SIZE]>>,
        free_list: Vec<PageId>,
    }

    impl MemoryPager {
        fn new() -> Self {
            Self {
                pages: vec![Some([0u8; PAGE_SIZE])], // metapágina
                free_list: Vec::new(),
            }
        }
    }

    impl Pager for MemoryPager {
        fn allocate(&mut self) -> Result<PageId, PagerError> {
            if let Some(id) = self.free_list.pop() {
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
            self.free_list.push(id);
            Ok(())
        }

        fn is_allocated(&self, id: PageId) -> bool {
            (id as usize) < self.pages.len()
                && self.pages[id as usize].is_some()
                && !self.free_list.contains(&id)
        }
    }

    /// Crea un pool con 3 frames (capacidad pequeña para forzar evictions).
    fn small_pool() -> (BufferPool<MemoryPager>, Vec<PageId>) {
        let mut pager = MemoryPager::new();
        // Asignamos 5 páginas para forzar evictions.
        let mut ids = Vec::new();
        for _ in 0..5 {
            ids.push(pager.allocate().unwrap());
        }
        let pool = BufferPool::new(pager, 3);
        (pool, ids)
    }

    #[test]
    fn bp_error_display() {
        let io_err = BufferPoolError::Io(PagerError::OutOfRange {
            requested: 5,
            num_pages: 1,
        });
        let s = format!("{io_err}");
        assert!(s.contains("buffer pool io"));
        assert!(io_err.source().is_some());

        let up_err = BufferPoolError::UnknownPage(42);
        let s = format!("{up_err}");
        assert!(s.contains("page 42 not allocated"));
        assert!(up_err.source().is_none());

        let pin_err = BufferPoolError::BadPinCount {
            page_id: 7,
            current: 0,
        };
        let s = format!("{pin_err}");
        assert!(s.contains("bad pin count"));
        assert!(pin_err.source().is_none());

        let full_err = BufferPoolError::PoolFullOfPinned;
        let s = format!("{full_err}");
        assert!(s.contains("all frames pinned"));
    }

    #[test]
    fn bp_from_pager_error() {
        let pe = PagerError::BadBufferSize {
            expected: PAGE_SIZE,
            got: 10,
        };
        let be: BufferPoolError = pe.into();
        assert!(matches!(be, BufferPoolError::Io(_)));
    }

    #[test]
    fn metrics_hit_ratio() {
        let m = Metrics::default();
        assert_eq!(m.hit_ratio(), 0.0);
        let m = Metrics {
            buffer_hits: 3,
            buffer_misses: 1,
            ..Default::default()
        };
        assert!((m.hit_ratio() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn bp_basic_get_unpin() {
        let (mut pool, ids) = small_pool();
        let id = ids[0];

        let buf = pool.get_page(id).unwrap();
        assert_eq!(buf.len(), PAGE_SIZE);

        // Inicialmente todo a cero.
        assert!(buf.iter().all(|&b| b == 0));

        pool.unpin(id, false).unwrap();

        // Segundo get → hit.
        let _ = pool.get_page(id).unwrap();
        pool.unpin(id, false).unwrap();

        let m = pool.metrics();
        assert_eq!(m.page_reads, 1);
        assert_eq!(m.buffer_misses, 1);
        assert_eq!(m.buffer_hits, 1);
        assert_eq!(m.page_writes, 0);
    }

    #[test]
    fn bp_modify_mark_dirty_flush() {
        let (mut pool, ids) = small_pool();
        let id = ids[0];

        {
            let buf = pool.get_page(id).unwrap();
            buf[..4].copy_from_slice(&42u32.to_le_bytes());
        }
        pool.mark_dirty(id).unwrap();
        pool.unpin(id, true).unwrap();

        let m = pool.metrics();
        assert_eq!(m.page_reads, 1);
        assert_eq!(m.page_writes, 0);

        let written = pool.flush().unwrap();
        assert_eq!(written, 1);

        let m = pool.metrics();
        assert_eq!(m.page_writes, 1);
    }

    #[test]
    fn bp_unknown_page() {
        let (mut pool, _ids) = small_pool();
        let r = pool.get_page(9999);
        assert!(matches!(r, Err(BufferPoolError::UnknownPage(9999))));
    }

    #[test]
    fn bp_unpin_unknown_page() {
        let (mut pool, ids) = small_pool();
        // La página 9999 nunca se cargó.
        let r = pool.unpin(9999, false);
        assert!(matches!(r, Err(BufferPoolError::UnknownPage(9999))));
        // Aseguramos que ids[0] se usa para evitar warning de unused.
        let _ = ids[0];
    }

    #[test]
    fn bp_double_unpin_error() {
        let (mut pool, ids) = small_pool();
        let id = ids[0];
        let _ = pool.get_page(id).unwrap();
        pool.unpin(id, false).unwrap();
        let r = pool.unpin(id, false);
        assert!(matches!(
            r,
            Err(BufferPoolError::BadPinCount {
                page_id,
                current: 0
            }) if page_id == id
        ));
    }

    #[test]
    fn bp_eviction_when_pool_full() {
        // Pool con capacidad 2.
        let mut pager = MemoryPager::new();
        let mut ids = Vec::new();
        for _ in 0..4 {
            ids.push(pager.allocate().unwrap());
        }
        let mut pool = BufferPool::new(pager, 2);

        // Cargamos páginas 0 y 1.
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        let _ = pool.get_page(ids[1]).unwrap();
        pool.unpin(ids[1], false).unwrap();
        assert_eq!(pool.occupied(), 2);

        // Cargamos páginas 2 y 3 → debe expulsar.
        let _ = pool.get_page(ids[2]).unwrap();
        pool.unpin(ids[2], false).unwrap();
        let _ = pool.get_page(ids[3]).unwrap();
        pool.unpin(ids[3], false).unwrap();

        let m = pool.metrics();
        assert_eq!(m.evictions, 2);
        assert_eq!(m.page_reads, 4);
        // Volver a pedir ids[0] debe ser miss (fue expulsado).
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        let m = pool.metrics();
        assert!(m.buffer_misses >= 5);
    }

    #[test]
    fn bp_dirty_page_is_flushed_on_eviction() {
        let mut pager = MemoryPager::new();
        let p1 = pager.allocate().unwrap();
        let p2 = pager.allocate().unwrap();
        let mut pool = BufferPool::new(pager, 1);

        // Cargamos p1, la modificamos y la marcamos sucia.
        {
            let buf = pool.get_page(p1).unwrap();
            buf[0] = 0xAA;
        }
        pool.mark_dirty(p1).unwrap();
        pool.unpin(p1, true).unwrap();

        // Cargamos p2 → expulsa p1 (y debe flushear antes).
        let _ = pool.get_page(p2).unwrap();
        pool.unpin(p2, false).unwrap();

        // El pager debe haber escrito p1. Lo verificamos reabriendo.
        let pager_ref = pool.pager_mut();
        let mut buf = [0u8; PAGE_SIZE];
        pager_ref.read(p1, &mut buf).unwrap();
        assert_eq!(buf[0], 0xAA);

        let m = pool.metrics();
        assert_eq!(m.page_writes, 1);
    }

    #[test]
    fn bp_pool_full_of_pinned() {
        let mut pager = MemoryPager::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(pager.allocate().unwrap());
        }
        let mut pool = BufferPool::new(pager, 2);

        // Pineamos 2 páginas (capacidad completa).
        let _ = pool.get_page(ids[0]).unwrap();
        let _ = pool.get_page(ids[1]).unwrap();
        // No las despineamos.

        // Tercer get → PoolFullOfPinned.
        let r = pool.get_page(ids[2]);
        assert!(matches!(r, Err(BufferPoolError::PoolFullOfPinned)));
    }

    #[test]
    fn bp_flush_no_dirty_is_noop() {
        let (mut pool, ids) = small_pool();
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();

        let written = pool.flush().unwrap();
        assert_eq!(written, 0);

        let m = pool.metrics();
        assert_eq!(m.page_writes, 0);
    }

    #[test]
    fn bp_flush_page_only_dirty() {
        let (mut pool, ids) = small_pool();
        let a = ids[0];
        let b = ids[1];

        // Modificamos a pero no b.
        {
            let buf = pool.get_page(a).unwrap();
            buf[0] = 0x11;
        }
        pool.mark_dirty(a).unwrap();
        pool.unpin(a, true).unwrap();

        let _ = pool.get_page(b).unwrap();
        pool.unpin(b, false).unwrap();

        let written = pool.flush_page(a).unwrap();
        assert!(written);
        let written = pool.flush_page(b).unwrap();
        assert!(!written);
    }

    #[test]
    fn bp_persistence_via_filepager() {
        // Test end-to-end: crear pager en disco, pool sobre él, escribir
        // páginas a través del pool, flush, cerrar, reabrir y verificar.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bp.liradb");

        let ids: Vec<PageId>;
        {
            let pager = FilePager::create(&path).unwrap();
            let mut pool = BufferPool::new(pager, 4);
            let mut pids = Vec::new();
            for _ in 0..3 {
                let pid = pool.pager_mut().allocate().unwrap();
                {
                    let buf = pool.get_page(pid).unwrap();
                    // Patrón: 4 bytes LE con el page_id en el inicio.
                    buf[..4].copy_from_slice(&pid.to_le_bytes());
                    buf[4] = 0xCC;
                }
                pool.mark_dirty(pid).unwrap();
                pool.unpin(pid, true).unwrap();
                pids.push(pid);
            }
            assert_eq!(pool.flush().unwrap(), 3);
            ids = pids;
        }

        // Reabrir y leer SIN buffer pool: verificar que los datos están.
        let mut pager2 = FilePager::open(&path).unwrap();
        let mut buf = [0u8; PAGE_SIZE];
        for pid in &ids {
            pager2.read(*pid, &mut buf).unwrap();
            let stored = u32::from_le_bytes(buf[..4].try_into().unwrap());
            assert_eq!(stored, *pid);
            assert_eq!(buf[4], 0xCC);
        }
    }

    #[test]
    fn bp_reload_via_pool() {
        // Mismo escenario que bp_persistence_via_filepager pero reabriendo
        // también el pool, y verificando que el primer get_page es miss
        // (pool vacío tras reopen).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bp2.liradb");

        let pid: PageId;
        {
            let pager = FilePager::create(&path).unwrap();
            let mut pool = BufferPool::new(pager, 2);
            pid = pool.pager_mut().allocate().unwrap();
            {
                let buf = pool.get_page(pid).unwrap();
                buf[..4].copy_from_slice(&pid.to_le_bytes());
                buf[4] = 0xDD;
            }
            pool.mark_dirty(pid).unwrap();
            pool.unpin(pid, true).unwrap();
            pool.flush().unwrap();
        }

        let pager2 = FilePager::open(&path).unwrap();
        let mut pool2 = BufferPool::new(pager2, 2);
        assert_eq!(pool2.occupied(), 0);
        {
            let buf = pool2.get_page(pid).unwrap();
            let stored = u32::from_le_bytes(buf[..4].try_into().unwrap());
            assert_eq!(stored, pid);
            assert_eq!(buf[4], 0xDD);
        }
        pool2.unpin(pid, false).unwrap();

        let m = pool2.metrics();
        assert_eq!(m.buffer_misses, 1);
        assert_eq!(m.page_reads, 1);
    }

    #[test]
    fn bp_clock_second_chance_protects_hot_page() {
        // Con la política Clock, una página con `ref_bit = true` al momento
        // de la búsqueda de víctima recibe "second chance": se le baja el bit
        // y se avanza la aguja. Verificamos que una página tocada
        // inmediatamente antes de la carga de una nueva página sobrevive.
        let mut pager = MemoryPager::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(pager.allocate().unwrap());
        }
        let mut pool = BufferPool::new(pager, 2);

        // Cargar ids[0] (miss). ref_bit = true.
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        // Cargar ids[1] (miss). ref_bit = true. Pool lleno: [ids[0], ids[1]].
        let _ = pool.get_page(ids[1]).unwrap();
        pool.unpin(ids[1], false).unwrap();

        // Tocar ids[0] inmediatamente antes de cargar ids[2]: su ref_bit
        // vuelve a ser true (hit).
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();

        // Cargar ids[2] → la aguja empieza a buscar. Como ids[0] acaba de ser
        // tocado, su ref_bit es true → second chance. ids[1] no fue tocado
        // desde su carga, su ref_bit podría haber sido puesto a false en una
        // vuelta previa del reloj → es la víctima esperada.
        let _ = pool.get_page(ids[2]).unwrap();
        pool.unpin(ids[2], false).unwrap();

        // ids[0] debe seguir en memoria: hit.
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();

        let m = pool.metrics();
        // Si Clock protegió ids[0], entonces ids[0] NO fue releyendo → no hay
        // page_read extra por el último get(ids[0]).
        // Las cargas nuevas son: ids[0], ids[1], ids[2] → 3 misses.
        assert_eq!(
            m.buffer_misses, 3,
            "ids[0] debe sobrevivir como hit al cargar ids[2]"
        );
        assert!(m.evictions >= 1, "debe haber al menos una expulsión");
    }

    #[test]
    fn bp_lru_policy_evicts_least_recent() {
        // Con LRU, la página menos recientemente usada debe ser expulsada.
        let mut pager = MemoryPager::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(pager.allocate().unwrap());
        }
        let mut pool = BufferPool::with_policy(pager, 2, PolicyKind::Lru);
        assert_eq!(pool.policy(), PolicyKind::Lru);

        // Cargar ids[0] y ids[1] en orden.
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        let _ = pool.get_page(ids[1]).unwrap();
        pool.unpin(ids[1], false).unwrap();
        // Tocar ids[0] para que sea la más reciente.
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();

        // Cargar ids[2] → expulsa ids[1] (más antigua).
        let _ = pool.get_page(ids[2]).unwrap();
        pool.unpin(ids[2], false).unwrap();

        // ids[0] sigue en memoria (hit), ids[1] fue expulsado (miss).
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        let _ = pool.get_page(ids[1]).unwrap();
        pool.unpin(ids[1], false).unwrap();

        let m = pool.metrics();
        assert_eq!(m.buffer_misses, 4, "ids[2] + ids[1] deben ser misses");
        assert!(m.evictions >= 1);
    }

    #[test]
    fn bp_capacity_is_correct() {
        let pager = MemoryPager::new();
        let pool = BufferPool::new(pager, 8);
        assert_eq!(pool.capacity(), 8);
        assert_eq!(pool.occupied(), 0);
    }

    #[test]
    fn bp_occupied_tracks_loaded_pages() {
        let (mut pool, ids) = small_pool();
        assert_eq!(pool.occupied(), 0);
        let _ = pool.get_page(ids[0]).unwrap();
        pool.unpin(ids[0], false).unwrap();
        assert_eq!(pool.occupied(), 1);
        let _ = pool.get_page(ids[1]).unwrap();
        pool.unpin(ids[1], false).unwrap();
        assert_eq!(pool.occupied(), 2);
    }

    #[test]
    fn bp_unpin_all_resets_pins() {
        let mut pager = MemoryPager::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(pager.allocate().unwrap());
        }
        let mut pool = BufferPool::new(pager, 3);
        let _ = pool.get_page(ids[0]).unwrap();
        let _ = pool.get_page(ids[1]).unwrap();
        // Ambos pineados.
        pool.unpin_all();
        // Ahora ids[2] puede cargar.
        let r = pool.get_page(ids[2]);
        assert!(r.is_ok());
    }

    #[test]
    fn bp_discard_cleans_dirty_ok() {
        let (mut pool, ids) = small_pool();
        let id = ids[0];
        let _ = pool.get_page(id).unwrap();
        pool.unpin(id, false).unwrap(); // no dirty
        pool.discard(id).unwrap();
        // Tras discard, el frame está libre: próximo get debe re-leer.
        let m0 = pool.metrics();
        let _ = pool.get_page(id).unwrap();
        pool.unpin(id, false).unwrap();
        let m1 = pool.metrics();
        assert_eq!(m1.page_reads, m0.page_reads + 1);
        assert_eq!(pool.occupied(), 1); // ids[0] recarga
    }

    #[test]
    fn bp_discard_dirty_rechazado() {
        let (mut pool, ids) = small_pool();
        let id = ids[0];
        {
            let buf = pool.get_page(id).unwrap();
            buf[0] = 0xFF;
        }
        pool.mark_dirty(id).unwrap();
        pool.unpin(id, true).unwrap();
        // Discard con dirty = error.
        let r = pool.discard(id);
        assert!(matches!(r, Err(BufferPoolError::BadPinCount { .. })));
    }

    #[test]
    fn bp_discard_unknown_page() {
        let (mut pool, _ids) = small_pool();
        let r = pool.discard(9999);
        assert!(matches!(r, Err(BufferPoolError::UnknownPage(9999))));
    }

    #[test]
    fn bp_mark_dirty_unknown_page() {
        let (mut pool, _ids) = small_pool();
        let r = pool.mark_dirty(9999);
        assert!(matches!(r, Err(BufferPoolError::UnknownPage(9999))));
    }
}
