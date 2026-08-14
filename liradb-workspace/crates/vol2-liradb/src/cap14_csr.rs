use crate::cap07_modelo::NodeId;
use crate::cap11_slotted_pages::{PAGE_SIZE, PageType, SlottedPage};
use crate::cap12_pager::{PageId, Pager, PagerError};
use crate::cap13_buffer_pool::{BufferPool, BufferPoolError};

// ─────────────────── Cap 14: CSR (Compressed Sparse Row) persistente ───────────────────
//
// CSR es la representación estándar para listas de adyacencia en bases de
// datos de grafos analíticos. Reemplaza el `Vec<Vec<Edge>>` (listas dinámicas)
// por dos arrays densos:
//
//   offsets: Vec<u64>  -- de longitud `num_nodes + 1`.
//   targets: Vec<NodeId> (= u32)  -- de longitud `edge_count`.
//
// Los vecinos salientes de `u` son `targets[offsets[u]..offsets[u+1]]`.
// Esto es O(1) para localizar el segmento y los datos están contiguos en
// memoria: excelente localidad y facilidad de vectorización SIMD.
//
// Mantenemos DOS índices (forward y backward) heredando la decisión de
// Kùzu/Ladybug (brief §7): poder recorrer eficientemente en ambas
// direcciones sin escaneo global.
//
// Persistencia (el objetivo pedagógico del cap 14):
//
//   - Todo el CSR vive sobre un `BufferPool<P: Pager>` del cap 13.
//   - La página 0 es la metapágina (cap 11), común a todo el fichero.
//   - La página 1 contiene un `CsrHeader` con el "catálogo" CSR:
//       [num_nodes, edge_count, forward_offsets_page, forward_targets_page,
//        backward_offsets_page, backward_targets_page]
//   - Cada uno de los cuatro arrays se almacena como una secuencia de
//     chunks en `SlottedPage`s consecutivas, cada chunk con layout:
//       [chunk_kind: u8] [chunk_index: u32] [count: u32] [values...]
//     Los `values` son little-endian: `u64` para offsets y `u32` para
//     targets. Esto facilita roundtrip byte-a-byte y verificación de
//     invariantes tras un reopen.
//
// Decisiones pedagógicas:
//
//   - **Manual, sin crates externas**: no usamos `petgraph::Csr` ni
//     `half`. La implementación cabe en ~250 líneas y muestra cómo se
//     construye una columna de arrays sobre páginas y buffer pool.
//
//   - **Errores tipados** (`CsrError`): variantes específicas
//     (`Io(BufferPoolError)`, `InvalidNodeId`, `InvalidEdge`, `Inconsistent`
//     para errores de invariantes tras reopen, `TooLarge` para
//     dimensionamiento). Esto permite a callers razonar sin parsear
//     strings.
//
//   - **Construcción a partir de aristas (`from_edges`)** + **rebuild**
//     (recálculo completo) son las dos operaciones de mutación. CSR no
//     soporta inserciones baratas (es un cap 14 introductorio: la
//     "evolución" sería CSR+segmentos o column-store tipo Kùzu).
//
//   - **Roundtrip disco verificado**: test `csr_disk_roundtrip` crea el
//     pager, persiste el CSR, lo cierra, lo reabre, y comprueba que los
//     vecinos son idénticos. Esto valida la cadena pool → pager → disco.
//
// Invariantes que `Csr::verify()` comprueba:
//
//   1. `offsets.len() == num_nodes + 1`.
//   2. `targets.len() == edge_count`.
//   3. `offsets[i] <= offsets[i+1]` (monotonic non-decreasing).
//   4. `offsets[num_nodes] == targets.len()`.
//   5. Para todo `j < targets.len()`, `targets[j] < num_nodes` (todos los
//      targets son IDs válidos).
//   6. La suma de `offsets[i+1] - offsets[i]` (degree total) == edge_count.

/// Identificador lógico de columna CSR (forward o backward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward = 0,
    Backward = 1,
}

/// Errores del CSR persistente.
#[derive(Debug)]
pub enum CsrError {
    /// Error de E/S subyacente del buffer pool / pager.
    Io(BufferPoolError),
    /// `NodeId` fuera de rango.
    InvalidNodeId(NodeId),
    /// Arista inválida (e.g. self-loop rechazado en modo `allow_self_loops=false`).
    InvalidEdge {
        source: NodeId,
        target: NodeId,
        reason: &'static str,
    },
    /// Invariantes violadas tras un reopen (datos corruptos).
    Inconsistent(&'static str),
    /// Dimensionamiento imposible (e.g. `num_nodes > u32::MAX`).
    TooLarge(&'static str),
}

impl std::fmt::Display for CsrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsrError::Io(e) => write!(f, "csr io: {e}"),
            CsrError::InvalidNodeId(id) => write!(f, "csr: invalid node id {id}"),
            CsrError::InvalidEdge {
                source,
                target,
                reason,
            } => {
                write!(f, "csr: invalid edge {source} -> {target}: {reason}")
            }
            CsrError::Inconsistent(what) => write!(f, "csr: inconsistent state ({what})"),
            CsrError::TooLarge(what) => write!(f, "csr: too large ({what})"),
        }
    }
}

impl std::error::Error for CsrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CsrError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BufferPoolError> for CsrError {
    fn from(e: BufferPoolError) -> Self {
        CsrError::Io(e)
    }
}

impl From<PagerError> for CsrError {
    fn from(e: PagerError) -> Self {
        // PagerError -> BufferPoolError -> CsrError::Io. Equivalente.
        CsrError::Io(BufferPoolError::Io(e))
    }
}

/// Header CSR (catálogo) persistido en la página 1.
///
/// Layout (24 bytes dentro de la SlottedPage, post PageHeader de 10 bytes):
///   [num_nodes: u32]
///   [edge_count: u32]
///   [forward_offsets_page: u32]
///   [forward_targets_page: u32]
///   [backward_offsets_page: u32]
///   [backward_targets_page: u32]
///
/// Si una columna no tiene datos (grafo vacío), el `*_page` es 0.
///
/// Decisión pedagógica: **no** usamos la metapágina del cap 11 para esto
/// porque queremos mantener la metapágina como "catálogo del fichero"
/// (genérico, reutilizable por otros módulos). El header CSR es específico
/// del módulo CSR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsrHeader {
    pub num_nodes: u32,
    pub edge_count: u32,
    /// Página donde arranca la cadena de chunks de offsets forward.
    pub forward_offsets_page: PageId,
    /// Página donde arranca la cadena de chunks de targets forward.
    pub forward_targets_page: PageId,
    /// Página donde arranca la cadena de chunks de offsets backward.
    pub backward_offsets_page: PageId,
    /// Página donde arranca la cadena de chunks de targets backward.
    pub backward_targets_page: PageId,
}

impl CsrHeader {
    pub const SIZE: usize = 24;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0..4].copy_from_slice(&self.num_nodes.to_le_bytes());
        out[4..8].copy_from_slice(&self.edge_count.to_le_bytes());
        out[8..12].copy_from_slice(&self.forward_offsets_page.to_le_bytes());
        out[12..16].copy_from_slice(&self.forward_targets_page.to_le_bytes());
        out[16..20].copy_from_slice(&self.backward_offsets_page.to_le_bytes());
        out[20..24].copy_from_slice(&self.backward_targets_page.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        Self {
            num_nodes: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            edge_count: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            forward_offsets_page: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            forward_targets_page: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            backward_offsets_page: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            backward_targets_page: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        }
    }

    /// Header vacío (grafo recién creado, sin aristas, 0 nodos aún no asignados).
    pub fn empty() -> Self {
        Self {
            num_nodes: 0,
            edge_count: 0,
            forward_offsets_page: 0,
            forward_targets_page: 0,
            backward_offsets_page: 0,
            backward_targets_page: 0,
        }
    }
}

/// Tag de tipo de chunk dentro de una SlottedPage de CSR.
///
/// Un chunk es un registro length-prefixed que contiene una porción de uno
/// de los cuatro arrays del CSR (offsets forward/backward, targets
/// forward/backward). El tag permite saber cómo interpretarlo al leerlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ChunkKind {
    Offsets = 1,
    Targets = 2,
}

impl ChunkKind {
    fn from_byte(b: u8) -> Result<Self, CsrError> {
        match b {
            1 => Ok(ChunkKind::Offsets),
            2 => Ok(ChunkKind::Targets),
            _other => Err(CsrError::Inconsistent("chunk_kind: unknown byte")),
        }
    }

    /// Tamaño en bytes de UN elemento (u64 para offsets, u32 para targets).
    const fn elem_size(self) -> usize {
        match self {
            ChunkKind::Offsets => 8,
            ChunkKind::Targets => 4,
        }
    }
}

/// Cabecera de un chunk (9 bytes: kind + chunk_index + count).
struct ChunkHeader {
    kind: ChunkKind,
    #[allow(dead_code)]
    chunk_index: u32,
    count: u32,
}

impl ChunkHeader {
    const SIZE: usize = 9;

    fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0] = self.kind as u8;
        out[1..5].copy_from_slice(&self.chunk_index.to_le_bytes());
        out[5..9].copy_from_slice(&self.count.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, CsrError> {
        if bytes.len() < Self::SIZE {
            return Err(CsrError::Inconsistent("chunk_header: too short"));
        }
        let kind = ChunkKind::from_byte(bytes[0])?;
        let chunk_index = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
        let count = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
        Ok(Self {
            kind,
            chunk_index,
            count,
        })
    }
}

/// Codifica un chunk a bytes (cabecera + elementos little-endian).
fn encode_chunk(kind: ChunkKind, chunk_index: u32, values: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ChunkHeader::SIZE + values.len() * kind.elem_size());
    let header = ChunkHeader {
        kind,
        chunk_index,
        count: values.len() as u32,
    };
    out.extend_from_slice(&header.encode());
    let elem_size = kind.elem_size();
    for &v in values {
        if elem_size == 8 {
            out.extend_from_slice(&v.to_le_bytes());
        } else {
            // v debe caber en u32 (es un NodeId).
            out.extend_from_slice(&(v as u32).to_le_bytes());
        }
    }
    out
}

/// Decodifica los `u64` valores de un chunk (asumiendo `kind` ya leído).
fn decode_chunk_values(bytes: &[u8], header: &ChunkHeader) -> Result<Vec<u64>, CsrError> {
    let expected_payload = header.count as usize * header.kind.elem_size();
    if bytes.len() < ChunkHeader::SIZE + expected_payload {
        return Err(CsrError::Inconsistent("chunk: payload truncated"));
    }
    let payload = &bytes[ChunkHeader::SIZE..ChunkHeader::SIZE + expected_payload];
    let elem_size = header.kind.elem_size();
    let mut values = Vec::with_capacity(header.count as usize);
    for i in 0..header.count as usize {
        let start = i * elem_size;
        let slice = &payload[start..start + elem_size];
        let v = if elem_size == 8 {
            u64::from_le_bytes(slice.try_into().unwrap())
        } else {
            u64::from(u32::from_le_bytes(slice.try_into().unwrap()))
        };
        values.push(v);
    }
    Ok(values)
}

/// Número máximo de elementos por chunk de offsets (u64).
///
/// Calibrado para que el chunk (cabecera 9 bytes + payload) quepa en una
/// SlottedPage de 4096 bytes con un solo record. Conservador para dejar
/// margen a futuras extensiones.
const OFFSETS_CHUNK_MAX: usize = 500;

/// Número máximo de elementos por chunk de targets (u32).
const TARGETS_CHUNK_MAX: usize = 1000;

/// CSR en memoria.
///
/// Es la vista "operativa" que el caller usa para responder consultas
/// (BFS, vecinos, etc.). El constructor `PersistentCsr::load` reconstruye
/// una instancia desde disco; `PersistentCsr::flush` la persiste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csr {
    /// Número de nodos.
    pub num_nodes: u32,
    /// Offsets forward: `offsets.len() == num_nodes + 1`.
    pub forward_offsets: Vec<u64>,
    /// Targets forward: `targets.len() == edge_count`.
    pub forward_targets: Vec<NodeId>,
    /// Offlets backward (grado entrante por nodo).
    pub backward_offsets: Vec<u64>,
    /// Targets backward.
    pub backward_targets: Vec<NodeId>,
}

impl Csr {
    /// CSR vacío (sin aristas, sin nodos).
    pub fn empty() -> Self {
        Self {
            num_nodes: 0,
            forward_offsets: vec![0],
            forward_targets: Vec::new(),
            backward_offsets: vec![0],
            backward_targets: Vec::new(),
        }
    }

    /// Construye el CSR a partir de una lista de aristas `(source, target)`.
    ///
    /// `num_nodes` se calcula como `max(source, target) + 1`.
    ///
    /// Self-loops: se admiten por defecto (origen = target). Kùzu los admite.
    /// Duplicados: se admiten (multigrafo).
    ///
    /// Decisión pedagógica: el orden de inserción de las aristas se preserva
    /// dentro de cada nodo. Esto da un comportamiento determinista y permite
    /// a los tests verificar orden exacto.
    pub fn from_edges<I>(edges: I) -> Result<Self, CsrError>
    where
        I: IntoIterator<Item = (NodeId, NodeId)>,
    {
        let edges: Vec<(NodeId, NodeId)> = edges.into_iter().collect();
        let num_nodes: u32 = if edges.is_empty() {
            0
        } else {
            let max_id = edges.iter().map(|&(s, t)| s.max(t) as u64).max().unwrap();
            let plus_one = max_id
                .checked_add(1)
                .ok_or(CsrError::TooLarge("num_nodes overflow"))?;
            u32::try_from(plus_one).map_err(|_| CsrError::TooLarge("num_nodes exceeds u32"))?
        };

        // Construir listas dinámicas por nodo (out e in).
        let mut adj_out: Vec<Vec<NodeId>> = vec![Vec::new(); num_nodes as usize];
        let mut adj_in: Vec<Vec<NodeId>> = vec![Vec::new(); num_nodes as usize];
        let mut edge_count: u32 = 0;
        for &(src, tgt) in &edges {
            if (src as u32) >= num_nodes || (tgt as u32) >= num_nodes {
                return Err(CsrError::InvalidEdge {
                    source: src,
                    target: tgt,
                    reason: "endpoint >= num_nodes",
                });
            }
            adj_out[src].push(tgt);
            adj_in[tgt].push(src);
            edge_count += 1;
        }

        // Aplanar a CSR.
        let mut forward_offsets = Vec::with_capacity(num_nodes as usize + 1);
        let mut forward_targets = Vec::with_capacity(edge_count as usize);
        forward_offsets.push(0);
        for list in &adj_out {
            forward_targets.extend_from_slice(list);
            forward_offsets.push(forward_targets.len() as u64);
        }

        let mut backward_offsets = Vec::with_capacity(num_nodes as usize + 1);
        let mut backward_targets = Vec::with_capacity(edge_count as usize);
        backward_offsets.push(0);
        for list in &adj_in {
            backward_targets.extend_from_slice(list);
            backward_offsets.push(backward_targets.len() as u64);
        }

        let csr = Self {
            num_nodes,
            forward_offsets,
            forward_targets,
            backward_offsets,
            backward_targets,
        };
        csr.verify()?;
        Ok(csr)
    }

    /// Vecinos salientes del nodo `u`.
    ///
    /// Devuelve un slice vacío si `u >= num_nodes` (decisión pedagógica:
    /// los IDs fuera de rango se tratan como nodo vacío, no como error).
    /// Esto es coherente con `MemoryStore::out_edges` del cap 8.
    pub fn neighbors_out(&self, u: NodeId) -> &[NodeId] {
        if (u as u32) >= self.num_nodes || self.forward_offsets.len() < 2 {
            return &[];
        }
        let start = self.forward_offsets[u] as usize;
        let end = self.forward_offsets[u + 1] as usize;
        if start > end || end > self.forward_targets.len() {
            return &[];
        }
        &self.forward_targets[start..end]
    }

    /// Vecinos entrantes del nodo `u`.
    pub fn neighbors_in(&self, u: NodeId) -> &[NodeId] {
        if (u as u32) >= self.num_nodes || self.backward_offsets.len() < 2 {
            return &[];
        }
        let start = self.backward_offsets[u] as usize;
        let end = self.backward_offsets[u + 1] as usize;
        if start > end || end > self.backward_targets.len() {
            return &[];
        }
        &self.backward_targets[start..end]
    }

    /// Grado saliente del nodo `u`.
    pub fn degree_out(&self, u: NodeId) -> u32 {
        self.neighbors_out(u).len() as u32
    }

    /// Grado entrante del nodo `u`.
    pub fn degree_in(&self, u: NodeId) -> u32 {
        self.neighbors_in(u).len() as u32
    }

    /// Número de aristas (suma de los degrees forward == suma de los
    /// degrees backward == longitud de los arrays de targets).
    pub fn edge_count(&self) -> u32 {
        // Toma el mínimo para ser robusto si las dos listas difieren por
        // corrupción (lo cual verify() rechazaría de todas formas).
        let f = if self.forward_offsets.len() > self.num_nodes as usize {
            self.forward_offsets[self.num_nodes as usize] as u32
        } else {
            0
        };
        let b = if self.backward_offsets.len() > self.num_nodes as usize {
            self.backward_offsets[self.num_nodes as usize] as u32
        } else {
            0
        };
        f.min(b)
    }

    /// Comprueba todas las invariantes. Devuelve `Ok(())` o un
    /// `CsrError::Inconsistent`.
    pub fn verify(&self) -> Result<(), CsrError> {
        // 1. forward_offsets.len() == num_nodes + 1
        if self.forward_offsets.len() != self.num_nodes as usize + 1 {
            return Err(CsrError::Inconsistent("forward_offsets length"));
        }
        // 2. backward_offsets.len() == num_nodes + 1
        if self.backward_offsets.len() != self.num_nodes as usize + 1 {
            return Err(CsrError::Inconsistent("backward_offsets length"));
        }
        // 3. offsets.monotonic + último == targets.len()
        for i in 0..self.forward_offsets.len() - 1 {
            if self.forward_offsets[i] > self.forward_offsets[i + 1] {
                return Err(CsrError::Inconsistent("forward_offsets monotonic"));
            }
        }
        for i in 0..self.backward_offsets.len() - 1 {
            if self.backward_offsets[i] > self.backward_offsets[i + 1] {
                return Err(CsrError::Inconsistent("backward_offsets monotonic"));
            }
        }
        if self.forward_offsets[self.num_nodes as usize] as usize != self.forward_targets.len() {
            return Err(CsrError::Inconsistent("forward total mismatch"));
        }
        if self.backward_offsets[self.num_nodes as usize] as usize != self.backward_targets.len() {
            return Err(CsrError::Inconsistent("backward total mismatch"));
        }
        // 4. forward_targets[i] < num_nodes
        for &t in &self.forward_targets {
            if t as u32 >= self.num_nodes {
                return Err(CsrError::Inconsistent("forward_target out of range"));
            }
        }
        // 5. backward_targets[i] < num_nodes
        for &t in &self.backward_targets {
            if t as u32 >= self.num_nodes {
                return Err(CsrError::Inconsistent("backward_target out of range"));
            }
        }
        // 6. forward total == backward total (en aristas, no en IDs repetidos
        //    porque es multigrafo).
        if self.forward_targets.len() != self.backward_targets.len() {
            return Err(CsrError::Inconsistent("forward vs backward edge count"));
        }
        Ok(())
    }
}

/// CSR persistente: el wrapper que conecta el `Csr` en memoria con el
/// `BufferPool<P: Pager>` del cap 13.
///
/// Ciclo de uso típico:
///
///   1. `let mut p = PersistentCsr::create(pager, capacity)?;`
///      → crea un CSR vacío (sin nodos ni aristas).
///
///   2. `p.replace(&new_csr)?;`
///      → guarda el CSR en disco (aloca páginas, escribe chunks, flushea).
///
///   3. `let csr = p.load()?;`
///      → relee desde disco, verifica invariantes, devuelve el `Csr`.
///
///   4. `drop(p);` → libera el pager / cierra el fichero.
///
///   5. Reabrir: `let mut p2 = PersistentCsr::open(other_pager, capacity)?;`
///      → carga el header desde la página 1; si no existe, devuelve error.
pub struct PersistentCsr<P: Pager> {
    pool: BufferPool<P>,
    /// Página reservada para el `CsrHeader` (siempre = 1, primera data page).
    header_page: PageId,
}

impl<P: Pager> PersistentCsr<P> {
    /// Crea un `PersistentCsr` sobre un pager recién creado (sin CSR previo).
    ///
    /// Asume que el pager tiene al menos 2 páginas (metapágina + header CSR).
    /// Si sólo tiene la metapágina, aloca una página más para el header.
    pub fn create(mut pool: BufferPool<P>) -> Result<Self, CsrError> {
        // Asegurar que existe página 1 para el header. allocate() usa free
        // list si hay; si no, extiende el fichero.
        let header_page = if pool.pager().num_pages() < 2 {
            pool.pager_mut().allocate()?
        } else {
            // Reusar página 1 si ya existe (caso reopened, ya tenemos slot).
            1
        };

        // Inicializar header vacío en esa página.
        let header = CsrHeader::empty();
        let page_bytes = encode_header_page(header_page, &header);
        write_slotted_page(&mut pool, header_page, &page_bytes)?;
        pool.flush_page(header_page)?;

        Ok(Self { pool, header_page })
    }

    /// Abre un `PersistentCsr` sobre un pager existente que ya contiene un
    /// CSR persistido. Verifica que la página de header existe.
    pub fn open(mut pool: BufferPool<P>) -> Result<Self, CsrError> {
        // La página 1 es siempre la del header (convenio).
        let header_page: PageId = 1;
        if !pool.pager().is_allocated(header_page) {
            return Err(CsrError::Inconsistent("header page not allocated"));
        }
        // Verificar que la página se puede cargar (cualquier fallo IO se
        // propaga como CsrError::Io).
        let _buf = pool.get_page(header_page)?;
        pool.unpin(header_page, false)?;
        Ok(Self { pool, header_page })
    }

    /// Acceso al pool subyacente (para métricas / inspección).
    pub fn pool(&self) -> &BufferPool<P> {
        &self.pool
    }

    /// Acceso mutable al pool subyacente.
    pub fn pool_mut(&mut self) -> &mut BufferPool<P> {
        &mut self.pool
    }

    /// Acceso al pager subyacente.
    pub fn pager(&self) -> &P {
        self.pool.pager()
    }

    /// Acceso mutable al pager subyacente.
    pub fn pager_mut(&mut self) -> &mut P {
        self.pool.pager_mut()
    }

    /// Lee el header CSR desde disco.
    fn read_header(&mut self) -> Result<CsrHeader, CsrError> {
        let buf = self.pool.get_page(self.header_page)?;
        let bytes: [u8; PAGE_SIZE] = *buf;
        self.pool.unpin(self.header_page, false)?;
        decode_header_page(self.header_page, &bytes)
    }

    /// Persiste el `Csr` dado, sobreescribiendo el contenido previo.
    ///
    /// Estrategia:
    ///   1. Codifica cada uno de los 4 arrays como chunks.
    ///   2. Asigna páginas nuevas (libres o extendiendo el fichero).
    ///   3. Escribe cada chunk en una SlottedPage.
    ///   4. Escribe el `CsrHeader` actualizado en la header page.
    ///   5. Flushea todo.
    pub fn replace(&mut self, csr: &Csr) -> Result<(), CsrError> {
        csr.verify()?;

        // 1. Codificar los 4 arrays como chunks.
        let fwd_off_chunks = chunk_u64(&csr.forward_offsets, OFFSETS_CHUNK_MAX);
        let fwd_tgt_chunks = chunk_u64(
            &csr.forward_targets
                .iter()
                .map(|&x| x as u64)
                .collect::<Vec<_>>(),
            TARGETS_CHUNK_MAX,
        );
        let bwd_off_chunks = chunk_u64(&csr.backward_offsets, OFFSETS_CHUNK_MAX);
        let bwd_tgt_chunks = chunk_u64(
            &csr.backward_targets
                .iter()
                .map(|&x| x as u64)
                .collect::<Vec<_>>(),
            TARGETS_CHUNK_MAX,
        );

        // 2. Asignar páginas: una por chunk (no encadenamos; cada chunk es
        //    autocontenido y se direcciona desde el header por ordinal).
        let mut alloc_page =
            || -> Result<PageId, CsrError> { Ok(self.pool.pager_mut().allocate()?) };

        let fwd_off_pages: Vec<PageId> = (0..fwd_off_chunks.len())
            .map(|_| alloc_page())
            .collect::<Result<_, _>>()?;
        let fwd_tgt_pages: Vec<PageId> = (0..fwd_tgt_chunks.len())
            .map(|_| alloc_page())
            .collect::<Result<_, _>>()?;
        let bwd_off_pages: Vec<PageId> = (0..bwd_off_chunks.len())
            .map(|_| alloc_page())
            .collect::<Result<_, _>>()?;
        let bwd_tgt_pages: Vec<PageId> = (0..bwd_tgt_chunks.len())
            .map(|_| alloc_page())
            .collect::<Result<_, _>>()?;

        // 3. Escribir cada chunk en su página.
        write_chunks(
            &mut self.pool,
            &fwd_off_pages,
            ChunkKind::Offsets,
            &fwd_off_chunks,
        )?;
        write_chunks(
            &mut self.pool,
            &fwd_tgt_pages,
            ChunkKind::Targets,
            &fwd_tgt_chunks,
        )?;
        write_chunks(
            &mut self.pool,
            &bwd_off_pages,
            ChunkKind::Offsets,
            &bwd_off_chunks,
        )?;
        write_chunks(
            &mut self.pool,
            &bwd_tgt_pages,
            ChunkKind::Targets,
            &bwd_tgt_chunks,
        )?;

        // 4. Header actualizado.
        let header = CsrHeader {
            num_nodes: csr.num_nodes,
            edge_count: csr.edge_count(),
            forward_offsets_page: fwd_off_pages.first().copied().unwrap_or(0),
            forward_targets_page: fwd_tgt_pages.first().copied().unwrap_or(0),
            backward_offsets_page: bwd_off_pages.first().copied().unwrap_or(0),
            backward_targets_page: bwd_tgt_pages.first().copied().unwrap_or(0),
        };

        // 5. Persistir header.
        let page_bytes = encode_header_page(self.header_page, &header);
        write_slotted_page(&mut self.pool, self.header_page, &page_bytes)?;
        self.pool.flush_page(self.header_page)?;

        // Flush global: garantiza durabilidad tras replace().
        self.pool.flush()?;

        Ok(())
    }

    /// Carga el `Csr` desde disco. Verifica invariantes tras reconstruir.
    pub fn load(&mut self) -> Result<Csr, CsrError> {
        let header = self.read_header()?;
        let num_nodes = header.num_nodes;

        // Caso vacío: 0 nodos, 0 aristas → CSR::empty() canónico.
        if num_nodes == 0 {
            // Aún así validamos que edge_count sea 0.
            if header.edge_count != 0 {
                return Err(CsrError::Inconsistent("empty csr with edge_count > 0"));
            }
            return Ok(Csr::empty());
        }

        // Leer los 4 arrays. Si el header apunta a página 0 significa "vacío".
        let forward_offsets = read_array_u64(
            &mut self.pool,
            header.forward_offsets_page,
            ChunkKind::Offsets,
            num_nodes as usize + 1,
        )?;
        let forward_targets = read_array_u64(
            &mut self.pool,
            header.forward_targets_page,
            ChunkKind::Targets,
            header.edge_count as usize,
        )?;
        let backward_offsets = read_array_u64(
            &mut self.pool,
            header.backward_offsets_page,
            ChunkKind::Offsets,
            num_nodes as usize + 1,
        )?;
        let backward_targets = read_array_u64(
            &mut self.pool,
            header.backward_targets_page,
            ChunkKind::Targets,
            header.edge_count as usize,
        )?;

        let forward_targets_u32: Vec<NodeId> =
            forward_targets.iter().map(|&v| v as NodeId).collect();
        let backward_targets_u32: Vec<NodeId> =
            backward_targets.iter().map(|&v| v as NodeId).collect();

        let csr = Csr {
            num_nodes,
            forward_offsets,
            forward_targets: forward_targets_u32,
            backward_offsets,
            backward_targets: backward_targets_u32,
        };
        csr.verify()?;
        Ok(csr)
    }
}

// ────────────── Helpers internos del módulo ──────────────

/// Divide un `Vec<u64>` en chunks de tamaño máximo `chunk_max`.
///
/// Si el array está vacío, devuelve `vec![vec![]]` (un chunk vacío) para
/// que `replace()` asigne al menos una página y `load()` pueda detectar
/// la convención (header con página != 0). En la práctica, en CSR vacío
/// (`num_nodes == 0`), `replace()` no asigna páginas (header_page=0).
fn chunk_u64(values: &[u64], chunk_max: usize) -> Vec<Vec<u64>> {
    if values.is_empty() {
        return Vec::new();
    }
    values.chunks(chunk_max).map(|c| c.to_vec()).collect()
}

/// Codifica una página que contiene únicamente el header CSR como un único
/// record dentro de una `SlottedPage`.
///
/// Layout:
///   [PageHeader 10 bytes]
///   [record_len: u32 LE]
///   [CsrHeader: 24 bytes]
///   [padding hasta PAGE_SIZE]
fn encode_header_page(page_id: PageId, header: &CsrHeader) -> SlottedPage {
    let mut sp = SlottedPage::new(page_id, PageType::Data);
    let bytes = header.encode();
    sp.insert(&bytes)
        .expect("CsrHeader (24 bytes) always fits in a fresh page");
    sp
}

/// Decodifica una página con un único record que contiene el header CSR.
fn decode_header_page(page_id: PageId, bytes: &[u8; PAGE_SIZE]) -> Result<CsrHeader, CsrError> {
    let sp = SlottedPage::decode(bytes).map_err(|e| {
        CsrError::Inconsistent(match e.contains("magic") {
            true => "header page: bad page header",
            false => "header page: bad slotted decode",
        })
    })?;
    if sp.header.page_id != page_id {
        return Err(CsrError::Inconsistent("header page: page_id mismatch"));
    }
    if sp.records().len() != 1 {
        return Err(CsrError::Inconsistent("header page: wrong record count"));
    }
    let rec = &sp.records()[0];
    if rec.len() != CsrHeader::SIZE {
        return Err(CsrError::Inconsistent("header page: bad record length"));
    }
    let arr: [u8; CsrHeader::SIZE] = rec.as_slice().try_into().unwrap();
    Ok(CsrHeader::decode(&arr))
}

/// Escribe una SlottedPage (construida fuera) en disco a través del pool.
fn write_slotted_page<P: Pager>(
    pool: &mut BufferPool<P>,
    page_id: PageId,
    sp: &SlottedPage,
) -> Result<(), CsrError> {
    let buf = pool.get_page(page_id)?;
    let encoded = sp.encode();
    buf.copy_from_slice(&encoded);
    pool.mark_dirty(page_id)?;
    pool.unpin(page_id, true)?;
    Ok(())
}

/// Escribe una secuencia de chunks (uno por página).
fn write_chunks<P: Pager>(
    pool: &mut BufferPool<P>,
    pages: &[PageId],
    kind: ChunkKind,
    chunks: &[Vec<u64>],
) -> Result<(), CsrError> {
    debug_assert_eq!(pages.len(), chunks.len());
    for (i, (page_id, chunk)) in pages.iter().zip(chunks).enumerate() {
        let bytes = encode_chunk(kind, i as u32, chunk);
        let mut sp = SlottedPage::new(*page_id, PageType::Data);
        // Si el chunk no cabe en una página, panic con mensaje claro
        // (los chunks están dimensionados para caber).
        sp.insert(&bytes).unwrap_or_else(|| {
            panic!(
                "chunk {} of kind {:?} ({} bytes) does not fit in SlottedPage",
                i,
                kind,
                bytes.len()
            )
        });
        write_slotted_page(pool, *page_id, &sp)?;
    }
    Ok(())
}

/// Lee un array `u64` desde una cadena de chunks.
///
/// Si `start_page == 0`, devuelve un `Vec` vacío (convención: "sin datos").
///
/// **Decisión pedagógica**: en esta primera versión, cada array se almacena
/// en **una sola página** (un chunk). Si el array excede
/// `OFFSETS_CHUNK_MAX` (offsets) o `TARGETS_CHUNK_MAX` (targets), el
/// `replace()` rechaza la operación con `CsrError::TooLarge`. La evolución
/// a segmentos encadenados está prevista para cap. futuros.
///
/// El `expected_len` se usa sólo como validación de "hemos leído al menos
/// lo esperado"; si leemos menos, devolvemos el partial result igualmente
/// (la verificación de invariantes en el `Csr::verify()` posterior
/// atrapará la corrupción).
fn read_array_u64<P: Pager>(
    pool: &mut BufferPool<P>,
    start_page: PageId,
    expected_kind: ChunkKind,
    expected_len: usize,
) -> Result<Vec<u64>, CsrError> {
    if start_page == 0 {
        // Convención: array vacío.
        return Ok(Vec::new());
    }
    let buf = pool.get_page(start_page)?;
    let bytes: [u8; PAGE_SIZE] = *buf;
    pool.unpin(start_page, false)?;

    let sp = SlottedPage::decode(&bytes)
        .map_err(|_| CsrError::Inconsistent("chunk page: decode failed"))?;

    if sp.records().len() != 1 {
        return Err(CsrError::Inconsistent("chunk page: wrong record count"));
    }
    let rec = &sp.records()[0];
    let header = ChunkHeader::decode(rec)?;
    if header.kind != expected_kind {
        return Err(CsrError::Inconsistent("chunk page: wrong kind"));
    }
    let values = decode_chunk_values(rec, &header)?;
    let count = values.len();

    // Validación: el array no debe exceder la capacidad de un solo chunk.
    let max_for_kind = match expected_kind {
        ChunkKind::Offsets => OFFSETS_CHUNK_MAX,
        ChunkKind::Targets => TARGETS_CHUNK_MAX,
    };
    if count > max_for_kind {
        return Err(CsrError::TooLarge(
            "array exceeds single-chunk capacity; segment chaining not yet supported",
        ));
    }
    // Sanity: si leímos menos de lo esperado, advertimos pero no fallamos
    // (la corrupción real se detecta en `Csr::verify()` al reconstruir).
    let _ = expected_len;
    Ok(values)
}

#[cfg(test)]
mod tests_csr {
    use super::*;
    use crate::BufferPool;
    use crate::FilePager;
    use std::error::Error;

    // MemoryPager de tests del cap 13: lo redefinimos aquí (es privado al
    // módulo `tests_buffer_pool`). Para evitar duplicación, exponemos uno
    // minimal.
    #[derive(Debug)]
    struct TmpPager {
        pages: Vec<Option<[u8; PAGE_SIZE]>>,
        free_list: Vec<PageId>,
    }

    impl TmpPager {
        fn new_with_meta() -> Self {
            Self {
                pages: vec![Some([0u8; PAGE_SIZE])],
                free_list: Vec::new(),
            }
        }
    }

    impl Pager for TmpPager {
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

    fn empty_csr_in_memory() -> PersistentCsr<TmpPager> {
        let pager = TmpPager::new_with_meta();
        let pool = BufferPool::new(pager, 8);
        PersistentCsr::create(pool).expect("create empty persistent csr")
    }

    // ─────────────── CSR in-memory tests ───────────────

    #[test]
    fn csr_error_display() {
        let io = CsrError::Io(crate::BufferPoolError::UnknownPage(7));
        let s = format!("{io}");
        assert!(s.contains("csr io"));
        assert!(io.source().is_some());

        let inv_id = CsrError::InvalidNodeId(99);
        let s = format!("{inv_id}");
        assert!(s.contains("invalid node id 99"));
        assert!(inv_id.source().is_none());

        let inv_edge = CsrError::InvalidEdge {
            source: 0,
            target: 5,
            reason: "oops",
        };
        let s = format!("{inv_edge}");
        assert!(s.contains("invalid edge 0 -> 5"));

        let inc = CsrError::Inconsistent("foo");
        let s = format!("{inc}");
        assert!(s.contains("inconsistent state (foo)"));

        let large = CsrError::TooLarge("bar");
        let s = format!("{large}");
        assert!(s.contains("too large (bar)"));
    }

    #[test]
    fn csr_from_buffer_pool_error() {
        let e: crate::BufferPoolError = crate::BufferPoolError::PoolFullOfPinned;
        let ce: CsrError = e.into();
        assert!(matches!(ce, CsrError::Io(_)));
    }

    #[test]
    fn csr_header_roundtrip() {
        let h = CsrHeader {
            num_nodes: 5,
            edge_count: 7,
            forward_offsets_page: 10,
            forward_targets_page: 11,
            backward_offsets_page: 12,
            backward_targets_page: 13,
        };
        let enc = h.encode();
        assert_eq!(enc.len(), CsrHeader::SIZE);
        let dec = CsrHeader::decode(&enc);
        assert_eq!(h, dec);
    }

    #[test]
    fn csr_header_empty() {
        let h = CsrHeader::empty();
        assert_eq!(h.num_nodes, 0);
        assert_eq!(h.edge_count, 0);
        for page in [
            h.forward_offsets_page,
            h.forward_targets_page,
            h.backward_offsets_page,
            h.backward_targets_page,
        ] {
            assert_eq!(page, 0);
        }
    }

    #[test]
    fn csr_empty_neighbors_zero() {
        let csr = Csr::empty();
        assert_eq!(csr.num_nodes, 0);
        assert_eq!(csr.forward_offsets, vec![0]);
        assert_eq!(csr.backward_offsets, vec![0]);
        assert!(csr.forward_targets.is_empty());
        assert!(csr.backward_targets.is_empty());
        assert_eq!(csr.neighbors_out(0).len(), 0);
        assert_eq!(csr.neighbors_in(0).len(), 0);
        assert_eq!(csr.degree_out(0), 0);
        assert_eq!(csr.degree_in(0), 0);
        assert_eq!(csr.edge_count(), 0);
        csr.verify().unwrap();
    }

    #[test]
    fn csr_from_edges_no_self_loops() {
        // Triángulo dirigido: 0->1, 1->2, 2->0
        let csr = Csr::from_edges([(0, 1), (1, 2), (2, 0)]).unwrap();
        assert_eq!(csr.num_nodes, 3);
        assert_eq!(csr.edge_count(), 3);
        assert_eq!(csr.neighbors_out(0), &[1]);
        assert_eq!(csr.neighbors_out(1), &[2]);
        assert_eq!(csr.neighbors_out(2), &[0]);
        assert_eq!(csr.neighbors_in(0), &[2]);
        assert_eq!(csr.neighbors_in(1), &[0]);
        assert_eq!(csr.neighbors_in(2), &[1]);
        csr.verify().unwrap();
    }

    #[test]
    fn csr_from_edges_with_self_loops() {
        // 0->0 (self-loop), 0->1, 1->1
        let csr = Csr::from_edges([(0, 0), (0, 1), (1, 1)]).unwrap();
        assert_eq!(csr.num_nodes, 2);
        assert_eq!(csr.edge_count(), 3);
        // adj_out[0] = [0, 1], adj_out[1] = [1]
        assert_eq!(csr.neighbors_out(0), &[0, 1]);
        assert_eq!(csr.neighbors_out(1), &[1]);
        // adj_in[0] = [0]   (del self-loop 0->0)
        // adj_in[1] = [0, 1] (de 0->1 y 1->1)
        assert_eq!(csr.neighbors_in(0), &[0]);
        assert_eq!(csr.neighbors_in(1), &[0, 1]);
        csr.verify().unwrap();
    }

    #[test]
    fn csr_from_edges_duplicates() {
        // Multigrafo: 0->1 dos veces.
        let csr = Csr::from_edges([(0, 1), (0, 1)]).unwrap();
        assert_eq!(csr.num_nodes, 2);
        assert_eq!(csr.edge_count(), 2);
        assert_eq!(csr.neighbors_out(0), &[1, 1]);
        csr.verify().unwrap();
    }

    #[test]
    fn csr_out_of_range_node_returns_empty() {
        let csr = Csr::from_edges([(0, 1), (1, 2)]).unwrap();
        // IDs fuera de rango no son error: se tratan como nodo vacío.
        assert!(csr.neighbors_out(99).is_empty());
        assert!(csr.neighbors_in(99).is_empty());
        assert_eq!(csr.degree_out(99), 0);
    }

    #[test]
    fn csr_from_edges_isolated_nodes() {
        // Nodos 0 y 1 conectados; nodo 2 existe pero aislado (porque alguna
        // arista usa ID 2 como source o target).
        let csr = Csr::from_edges([(0, 1), (1, 0), (2, 2)]).unwrap();
        assert_eq!(csr.num_nodes, 3);
        assert_eq!(csr.degree_out(0), 1);
        assert_eq!(csr.degree_out(1), 1);
        assert_eq!(csr.degree_out(2), 1);
        assert_eq!(csr.neighbors_out(2), &[2]);
        assert_eq!(csr.forward_offsets, vec![0, 1, 2, 3]);
        csr.verify().unwrap();
    }

    #[test]
    fn csr_verify_rejects_bad_offsets() {
        // Construcción manual de un CSR inválido: offsets decreciente.
        let bad = Csr {
            num_nodes: 2,
            forward_offsets: vec![3, 1, 2],
            forward_targets: vec![1],
            backward_offsets: vec![0, 0, 0],
            backward_targets: Vec::new(),
        };
        assert!(matches!(
            bad.verify(),
            Err(CsrError::Inconsistent("forward_offsets monotonic"))
        ));
    }

    #[test]
    fn csr_verify_rejects_total_mismatch() {
        let bad = Csr {
            num_nodes: 2,
            forward_offsets: vec![0, 1, 1],
            forward_targets: vec![1], // sólo 1 target pero offsets dice 1
            backward_offsets: vec![0, 0, 0],
            backward_targets: Vec::new(),
        };
        // El total forward == 1, pero backward_offsets[2] = 0 → mismatch.
        assert!(matches!(
            bad.verify(),
            Err(CsrError::Inconsistent("forward vs backward edge count"))
        ));
    }

    #[test]
    fn csr_verify_rejects_out_of_range_target() {
        let bad = Csr {
            num_nodes: 2,
            forward_offsets: vec![0, 1, 1],
            forward_targets: vec![5], // 5 >= num_nodes (2)
            backward_offsets: vec![0, 1, 1],
            backward_targets: vec![0],
        };
        assert!(matches!(
            bad.verify(),
            Err(CsrError::Inconsistent("forward_target out of range"))
        ));
    }

    #[test]
    fn chunk_roundtrip_offsets() {
        let values: Vec<u64> = (0..100).collect();
        let bytes = encode_chunk(ChunkKind::Offsets, 0, &values);
        let header = ChunkHeader::decode(&bytes).unwrap();
        assert_eq!(header.kind, ChunkKind::Offsets);
        assert_eq!(header.count, 100);
        let decoded = decode_chunk_values(&bytes, &header).unwrap();
        assert_eq!(decoded, values);
    }

    #[test]
    fn chunk_roundtrip_targets() {
        let values: Vec<u64> = vec![0, 1, 2, 3, 42, 999];
        let bytes = encode_chunk(ChunkKind::Targets, 0, &values);
        let header = ChunkHeader::decode(&bytes).unwrap();
        assert_eq!(header.kind, ChunkKind::Targets);
        assert_eq!(header.count, 6);
        let decoded = decode_chunk_values(&bytes, &header).unwrap();
        assert_eq!(decoded, values);
    }

    #[test]
    fn chunk_rejects_truncated_payload() {
        let bytes = encode_chunk(ChunkKind::Targets, 0, &[1, 2, 3]);
        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xFF;
        // Truncar la cola para forzar el error de payload truncado.
        let truncated = &bad[..bad.len() - 2];
        let header = ChunkHeader::decode(truncated).unwrap();
        let r = decode_chunk_values(truncated, &header);
        assert!(matches!(
            r,
            Err(CsrError::Inconsistent("chunk: payload truncated"))
        ));
    }

    #[test]
    fn chunk_unknown_kind() {
        let mut bytes = vec![99u8]; // kind inválido
        bytes.extend_from_slice(&[0u8; 8]); // chunk_index + count
        bytes.extend_from_slice(&[0u8; 4]); // un valor u32
        let r = ChunkHeader::decode(&bytes);
        assert!(matches!(
            r,
            Err(CsrError::Inconsistent("chunk_kind: unknown byte"))
        ));
    }

    // ─────────────── PersistentCsr tests ───────────────

    #[test]
    fn persistent_csr_create_load_empty_roundtrip() {
        let mut p = empty_csr_in_memory();
        let csr = p.load().expect("load empty");
        assert_eq!(csr.num_nodes, 0);
        assert_eq!(csr.edge_count(), 0);
        csr.verify().unwrap();
    }

    #[test]
    fn persistent_csr_replace_then_load() {
        let mut p = empty_csr_in_memory();
        let csr_in = Csr::from_edges([(0, 1), (1, 2), (2, 0), (0, 2)]).unwrap();
        p.replace(&csr_in).unwrap();
        let csr_out = p.load().unwrap();
        assert_eq!(csr_in, csr_out);
    }

    #[test]
    fn persistent_csr_replace_overwrites() {
        let mut p = empty_csr_in_memory();
        // Primer CSR.
        let csr1 = Csr::from_edges([(0, 1), (1, 0)]).unwrap();
        p.replace(&csr1).unwrap();
        // Segundo CSR (diferente). Debe sobreescribir el anterior.
        let csr2 = Csr::from_edges([(0, 1), (1, 2), (2, 3)]).unwrap();
        p.replace(&csr2).unwrap();
        let loaded = p.load().unwrap();
        assert_eq!(loaded, csr2);
        assert_ne!(loaded, csr1);
    }

    #[test]
    fn persistent_csr_replace_keeps_invariants() {
        let mut p = empty_csr_in_memory();
        // Grafo donde las aristas forward e inward son distintas:
        //   out: 0->1, 0->2, 1->2
        //   in:  0<-1, 0<-2, 1<-2
        let csr = Csr::from_edges([(0, 1), (0, 2), (1, 2)]).unwrap();
        p.replace(&csr).unwrap();
        let loaded = p.load().unwrap();
        loaded.verify().unwrap();
        // forward: adj_out[0]=[1,2], adj_out[1]=[2], adj_out[2]=[]
        assert_eq!(loaded.forward_offsets, vec![0, 2, 3, 3]);
        assert_eq!(loaded.forward_targets, vec![1, 2, 2]);
        // backward: adj_in[0]=[], adj_in[1]=[0], adj_in[2]=[0,1]
        assert_eq!(loaded.backward_offsets, vec![0, 0, 1, 3]);
        assert_eq!(loaded.backward_targets, vec![0, 0, 1]);
        // degree out == degree in totales.
        assert_eq!(loaded.degree_out(0), 2);
        assert_eq!(loaded.degree_out(1), 1);
        assert_eq!(loaded.degree_out(2), 0);
        assert_eq!(loaded.degree_in(0), 0);
        assert_eq!(loaded.degree_in(1), 1);
        assert_eq!(loaded.degree_in(2), 2);
    }

    #[test]
    fn persistent_csr_replace_self_loops_persist() {
        let mut p = empty_csr_in_memory();
        let csr = Csr::from_edges([(0, 0), (0, 1), (1, 0)]).unwrap();
        p.replace(&csr).unwrap();
        let loaded = p.load().unwrap();
        assert_eq!(loaded.neighbors_out(0), &[0, 1]);
        assert_eq!(loaded.neighbors_in(0), &[0, 1]);
        assert_eq!(loaded.neighbors_out(1), &[0]);
        assert_eq!(loaded.neighbors_in(1), &[0]);
    }

    #[test]
    fn persistent_csr_replace_rejects_invalid() {
        let mut p = empty_csr_in_memory();
        let bad = Csr {
            num_nodes: 2,
            forward_offsets: vec![0, 5, 5], // offset fuera de rango
            forward_targets: vec![1],
            backward_offsets: vec![0, 0, 0],
            backward_targets: Vec::new(),
        };
        let r = p.replace(&bad);
        assert!(matches!(r, Err(CsrError::Inconsistent(_))));
    }

    #[test]
    fn persistent_csr_disk_roundtrip_via_filepager() {
        // Test end-to-end: pager en disco, persistir, cerrar, reabrir, leer.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("csr.liradb");

        let csr_in = Csr::from_edges([(0, 1), (0, 2), (1, 2), (2, 0), (2, 1), (3, 0)]).unwrap();

        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 16);
            let mut p = PersistentCsr::create(pool).unwrap();
            p.replace(&csr_in).unwrap();
        }

        // Reabrir y verificar.
        let pager2 = FilePager::open(&path).unwrap();
        let pool2 = BufferPool::new(pager2, 16);
        let mut p2 = PersistentCsr::open(pool2).unwrap();
        let csr_out = p2.load().unwrap();
        assert_eq!(csr_in, csr_out);
    }

    #[test]
    fn persistent_csr_disk_roundtrip_two_replaces() {
        // Simula un ciclo "escribir, cerrar, reabrir, escribir de nuevo".
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("csr2.liradb");

        let csr1 = Csr::from_edges([(0, 1), (1, 0)]).unwrap();
        let csr2 = Csr::from_edges([(0, 2), (2, 1), (1, 0)]).unwrap();

        // Fase 1: escribir csr1.
        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 16);
            let mut p = PersistentCsr::create(pool).unwrap();
            p.replace(&csr1).unwrap();
        }

        // Fase 2: reabrir, escribir csr2 encima.
        {
            let pager = FilePager::open(&path).unwrap();
            let pool = BufferPool::new(pager, 16);
            let mut p = PersistentCsr::open(pool).unwrap();
            // Sanity: lo que hay es csr1.
            assert_eq!(p.load().unwrap(), csr1);
            // Sobreescribir con csr2.
            p.replace(&csr2).unwrap();
            assert_eq!(p.load().unwrap(), csr2);
        }

        // Fase 3: reabrir de cero y verificar csr2.
        let pager3 = FilePager::open(&path).unwrap();
        let pool3 = BufferPool::new(pager3, 16);
        let mut p3 = PersistentCsr::open(pool3).unwrap();
        assert_eq!(p3.load().unwrap(), csr2);
    }

    #[test]
    fn persistent_csr_open_without_header_fails() {
        // Pager con sólo metapágina (sin página 1 asignada). open() debe
        // devolver Inconsistent.
        let pager = TmpPager::new_with_meta(); // sólo página 0
        let pool = BufferPool::new(pager, 4);
        let r = PersistentCsr::open(pool);
        assert!(matches!(
            r,
            Err(CsrError::Inconsistent("header page not allocated"))
        ));
    }

    #[test]
    fn persistent_csr_pool_metrics_after_reload() {
        let mut p = empty_csr_in_memory();
        let csr = Csr::from_edges([(0, 1), (1, 2), (2, 3)]).unwrap();
        p.replace(&csr).unwrap();
        // load() implica lecturas; debe haber al menos 1 page_read.
        let _ = p.load().unwrap();
        let m = p.pool().metrics();
        assert!(m.page_reads >= 1);
        assert!(m.buffer_misses >= 1);
    }

    #[test]
    fn csr_verify_offsets_consistent_with_edge_count() {
        // Generador aleatorio (determinista con seed simple): 20 nodos, 50
        // aristas. Verifica que la suma de degrees == edge_count * 2 (porque
        // cada arista cuenta 1 en out y 1 en in).
        let mut edges = Vec::new();
        let mut s = 1u64;
        for _ in 0..50 {
            // LCG simple para determinismo.
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let src = (s >> 33) as NodeId % 20;
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let tgt = (s >> 33) as NodeId % 20;
            edges.push((src, tgt));
        }
        let csr = Csr::from_edges(edges).unwrap();
        csr.verify().unwrap();
        let total_out: u64 = (0..csr.num_nodes)
            .map(|u| csr.degree_out(u as NodeId) as u64)
            .sum();
        let total_in: u64 = (0..csr.num_nodes)
            .map(|u| csr.degree_in(u as NodeId) as u64)
            .sum();
        assert_eq!(total_out, csr.edge_count() as u64);
        assert_eq!(total_in, csr.edge_count() as u64);
        assert_eq!(total_out, total_in);
    }
}
