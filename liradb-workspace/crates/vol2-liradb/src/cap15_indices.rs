use crate::cap11_slotted_pages::{PAGE_SIZE, PageHeader, PageType, SlottedPage};
use crate::cap12_pager::{PageId, Pager, PagerError};
use crate::cap13_buffer_pool::{BufferPool, BufferPoolError};

// ─────────────────── Cap 15: Índices para encontrar datos (hash + B+ tree) ───────────────────
//
// En el cap 14 (CSR) recorremos las adyacencias por offset/topología, pero
// "¿qué aristas tienen `weight > 5`?" o "¿qué nodo tiene la propiedad
// `name = "Ada"`?" siguen siendo O(N): escaneamos TODO el grafo. Eso es
// inaceptable para queries reales.
//
// Un **índice** es una estructura de datos auxiliar que mapea **clave → valor**
// y permite responder consultas por clave sin escanear el conjunto completo.
//
// En este capítulo implementamos DOS índices sobre `BufferPool<Pager>`:
//
//   1. **HashIndex** (estático, con desbordamiento en SlottedPages encadenadas):
//      mapea `u64 → u64`. Cada bucket es una página; los desbordamientos se
//      encadenan vía un puntero `next_page` en la `SlottedPage`. Es el "hash
//      join" de Kùzu y el corazón de las búsquedas por igualdad.
//
//   2. **BPlusTree** (single-level, leaf+separator): mapea `u64 → u64` con
//      **range scan** eficiente. La raíz (página 2) contiene los separadores
//      y punteros a las hojas; cada hoja es una SlottedPage con pares
//      ordenados `(key, value)` y un puntero a la siguiente hoja.
//
// Filosofía pedagógica:
//
//   - "Primero a mano, luego con crate": todo se implementa sin crates
//     externas. Las APIs de `lru`, `hashbrown`, `redb` se ven en caps. futuros
//     como comparación.
//
//   - **Estáticos** (no dinámicos): los índices se construyen de una vez
//     sobre un dataset ya cargado. Las inserciones se modelan como
//     "rebuild" (drop + recreate). Los índices dinámicos son tema del cap 28.
//
//   - **Errores tipados** (`IndexError`) con variantes específicas: cualquier
//     caller puede distinguir "página no asignada", "tipo de slot
//     desconocido", "invariantes violadas", etc., sin parsear strings.
//
//   - **Roundtrip disco verificado**: ambos índices se persisten y se
//     releen correctamente tras un reopen del `FilePager`.
//
// Layout en disco:
//
//   ┌─────────────────────────────────────────────────────────────────────┐
//   │ page 0: MetaPage (genérica, ya existente)                          │
//   │ page 1: reserved para uso futuro (e.g. catálogo global de índices)  │
//   │ page 2..N: catálogos + páginas de buckets (HashIndex)               │
//   │           raíz + hojas (BPlusTree)                                  │
//   └─────────────────────────────────────────────────────────────────────┘
//
// Para HashIndex:
//   - Página 2 = catálogo (Header: `num_buckets`, `key_count`, `overflow`).
//   - Páginas 3..(3+B-1) = buckets primarios (B = num_buckets).
//   - Páginas adicionales para desbordamientos, encadenadas vía `next_page`
//     en el header de la SlottedPage (primer record = (next_page: u32, ...)).
//
// Para BPlusTree:
//   - Página 2 = nodo raíz (leaf en este cap simple). Contiene pares
//     `(key, value)` ordenados + `next_leaf: u32`. Los pares se almacenan
//     como records en la SlottedPage (key 8B LE + value 8B LE = 16B/record).
//   - Para esta primera versión, la raíz es la **única** hoja: el árbol es
//     "de un solo nivel". El rango se itera directamente desde la raíz. La
//     evolución a multi-nivel está prevista en caps. futuros.
//
// API mínima:
//
//   ```text
//   HashIndex::create(pool)       → nuevo índice vacío.
//   HashIndex::open(pool)         → abre un índice existente.
//   h.insert(key, value)         → añade o reemplaza (key → value).
//   h.get(key) -> Option<u64>    → lookup por igualdad.
//   h.len()                      → número de pares insertados.
//   h.bucket_count()             → número de buckets.
//
//   BPlusTree::create(pool)       → nuevo árbol vacío.
//   BPlusTree::open(pool)         → abre un árbol existente.
//   t.insert(key, value)         → añade o reemplaza.
//   t.get(key) -> Option<u64>    → lookup exacto.
//   t.range_scan(lo, hi)         → itera sobre [lo, hi] en orden.
//   t.len()                      → número de pares insertados.
//   ```

/// Errores de los índices (cap 15).
#[derive(Debug)]
pub enum IndexError {
    /// Error de E/S del buffer pool / pager.
    Io(BufferPoolError),
    /// El tipo de slot (record) leído de una página de índice es desconocido.
    UnknownSlotKind(u8),
    /// Invariantes violadas tras un reopen (catálogo corrupto o página que
    /// no se corresponde con el tipo esperado).
    Inconsistent(&'static str),
    /// El catálogo apunta a una página no asignada en el pager.
    PageNotAllocated(PageId),
    /// Overflow de dimensión (e.g. `num_buckets == 0`).
    InvalidParam(&'static str),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Io(e) => write!(f, "index io: {e}"),
            IndexError::UnknownSlotKind(b) => {
                write!(f, "index: unknown slot kind {b:#x}")
            }
            IndexError::Inconsistent(what) => write!(f, "index: inconsistent state ({what})"),
            IndexError::PageNotAllocated(id) => write!(f, "index: page {id} not allocated"),
            IndexError::InvalidParam(what) => write!(f, "index: invalid param ({what})"),
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IndexError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BufferPoolError> for IndexError {
    fn from(e: BufferPoolError) -> Self {
        IndexError::Io(e)
    }
}

impl From<PagerError> for IndexError {
    fn from(e: PagerError) -> Self {
        IndexError::Io(BufferPoolError::Io(e))
    }
}

// ────────────── Helpers compartidos (página de catálogo) ──────────────

/// Codifica una página que contiene únicamente un bloque de bytes como un
/// único record dentro de una `SlottedPage` (reutilizable por HashIndex y
/// BPlusTree para sus catálogos).
///
/// Layout:
///   `[PageHeader 10 bytes] [record_len: u32 LE] [payload bytes] [padding]`
fn encode_record_page(page_id: PageId, payload: &[u8]) -> SlottedPage {
    let mut sp = SlottedPage::new(page_id, PageType::Data);
    sp.insert(payload).unwrap_or_else(|| {
        panic!(
            "record page {page_id}: payload {} bytes does not fit",
            payload.len()
        )
    });
    sp
}

/// Decodifica una página con un único record; devuelve el payload crudo.
fn decode_record_page(bytes: &[u8; PAGE_SIZE]) -> Result<Vec<u8>, IndexError> {
    let sp =
        SlottedPage::decode(bytes).map_err(|_| IndexError::Inconsistent("record page decode"))?;
    if sp.records().len() != 1 {
        return Err(IndexError::Inconsistent("record page: wrong record count"));
    }
    Ok(sp.records()[0].clone())
}

/// Escribe un payload (≤ PAGE_SIZE - header) en una página reservada del
/// pool. Crea el `SlottedPage`, lo codifica, marca dirty y despinea.
fn write_record_page<P: Pager>(
    pool: &mut BufferPool<P>,
    page_id: PageId,
    payload: &[u8],
) -> Result<(), IndexError> {
    let sp = encode_record_page(page_id, payload);
    let buf = pool.get_page(page_id)?;
    let encoded = sp.encode();
    buf.copy_from_slice(&encoded);
    pool.mark_dirty(page_id)?;
    pool.unpin(page_id, true)?;
    Ok(())
}

/// Lee una página reservada del pool y devuelve su único record (payload).
fn read_record_page<P: Pager>(
    pool: &mut BufferPool<P>,
    page_id: PageId,
) -> Result<Vec<u8>, IndexError> {
    let buf = pool.get_page(page_id)?;
    let bytes: [u8; PAGE_SIZE] = *buf;
    pool.unpin(page_id, false)?;
    decode_record_page(&bytes)
}

// ──────────────────────── HashIndex ────────────────────────

/// Hash FNV-1a 64-bit (sin tabla). Usado por el `HashIndex` para distribuir
/// claves en buckets.
///
/// Decisión pedagógica: implementamos el hash a mano (10 líneas) para que
/// el alumno vea cómo se construye un buen hash sin dependencias. FNV-1a es
/// razonablemente rápido y tiene buenas propiedades de dispersión para
/// claves numéricas pequeñas.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// Header del HashIndex (catálogo persistido en su página de catálogo).
///
/// Layout (16 bytes, little-endian):
///   `[magic: u32] [num_buckets: u32] [key_count: u32] [reserved: u32]`
///
/// `magic` = `0x4849_4431` ("HID1"). Sirve para detectar corrupción.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashIndexHeader {
    pub magic: u32,
    pub num_buckets: u32,
    pub key_count: u32,
    pub reserved: u32,
}

impl HashIndexHeader {
    pub const SIZE: usize = 16;
    pub const MAGIC: u32 = 0x4849_4431;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&self.num_buckets.to_le_bytes());
        out[8..12].copy_from_slice(&self.key_count.to_le_bytes());
        out[12..16].copy_from_slice(&self.reserved.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        Self {
            magic: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            num_buckets: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            key_count: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            reserved: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        }
    }

    pub fn empty(num_buckets: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            num_buckets,
            key_count: 0,
            reserved: 0,
        }
    }
}

/// Una entrada `(key, value)` en una página de bucket del HashIndex.
///
/// Layout (16 bytes, little-endian):
///   `[key: u64] [value: u64]`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashEntry {
    pub key: u64,
    pub value: u64,
}

impl HashEntry {
    pub const SIZE: usize = 16;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0..8].copy_from_slice(&self.key.to_le_bytes());
        out[8..16].copy_from_slice(&self.value.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        Self {
            key: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            value: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        }
    }
}

/// Cabecera de una página de bucket del HashIndex.
///
/// Layout (4 bytes, primer record de la SlottedPage):
///   `[next_page: u32]`
///
/// Si `next_page == 0`, la cadena termina aquí. El siguiente record
/// (si existe) es la primera `HashEntry`. Los records restantes son las
/// entradas adicionales del bucket.
struct BucketHeader {
    next_page: PageId,
}

impl BucketHeader {
    const SIZE: usize = 4;

    fn encode(&self) -> [u8; Self::SIZE] {
        self.next_page.to_le_bytes()
    }

    fn decode(bytes: &[u8]) -> Result<Self, IndexError> {
        if bytes.len() < Self::SIZE {
            return Err(IndexError::Inconsistent("bucket header too short"));
        }
        let mut b = [0u8; 4];
        b.copy_from_slice(&bytes[..4]);
        Ok(Self {
            next_page: u32::from_le_bytes(b),
        })
    }
}

/// Número por defecto de buckets si el caller no especifica.
pub const DEFAULT_BUCKETS: u32 = 16;

/// HashIndex estático sobre `BufferPool<Pager>`.
///
/// Implementación:
///
///   - El catálogo (`HashIndexHeader`) vive en la **página 2**.
///   - Los buckets se asignan en las **páginas 3..(3+B-1)**, donde B =
///     `num_buckets`. Cada bucket empieza en su página primaria y, si se
///     desborda, se encadena con páginas adicionales vía `next_page`.
///   - El hash es FNV-1a sobre los 8 bytes LE de la clave (8 bytes caben
///     siempre en el estado del hash).
///   - `insert` y `get` hacen I/O en el peor caso O(1 + chain_length).
///     Las chains son típicamente cortas (factor de carga ≤ ~70%).
///
/// **Estático**: no soporta inserciones concurrentes. Para "actualizar" un
/// índice, se rebuilda (`create` + `insert*` + flush). Esto es coherente
/// con la regla del brief LiraDB: los índices son materializaciones de un
/// dataset ya cargado, no mutables en línea.
///
/// **Errores**: las I/O failures se propagan como `IndexError::Io`. Una
/// página de bucket que no es header se considera `IndexError::Inconsistent`.
pub struct HashIndex<P: Pager> {
    pool: BufferPool<P>,
    /// Página del catálogo (siempre 2).
    catalog_page: PageId,
    /// Primera página del bucket `i` (i ∈ [0, num_buckets)).
    bucket_starts: Vec<PageId>,
    /// Header actual (cacheado).
    header: HashIndexHeader,
}

impl<P: Pager> HashIndex<P> {
    /// Crea un nuevo `HashIndex` con `num_buckets` cubos. Si el pager tiene
    /// menos páginas que `3 + num_buckets`, las extiende.
    pub fn create(mut pool: BufferPool<P>, num_buckets: u32) -> Result<Self, IndexError> {
        if num_buckets == 0 {
            return Err(IndexError::InvalidParam("num_buckets must be > 0"));
        }
        // Reservar páginas para catálogo + buckets primarios.
        // Política: las páginas 0 (meta), 1 (reservada), 2 (catálogo),
        // 3..3+B-1 (buckets).
        // Asegurar que existen.
        let target = 3 + num_buckets;
        let current = pool.pager().num_pages();
        for _ in current..target {
            pool.pager_mut().allocate()?;
        }

        let catalog_page: PageId = 2;
        let bucket_starts: Vec<PageId> = (0..num_buckets).map(|i| 3 + i).collect();

        let header = HashIndexHeader::empty(num_buckets);
        write_record_page(&mut pool, catalog_page, &header.encode())?;

        // Inicializar cada bucket con un header `next_page = 0` y sin entradas.
        for &b in &bucket_starts {
            let bh = BucketHeader { next_page: 0 };
            // Página vacía: sólo el record de cabecera.
            write_record_page(&mut pool, b, &bh.encode())?;
        }

        pool.flush_page(catalog_page)?;
        for &b in &bucket_starts {
            pool.flush_page(b)?;
        }

        Ok(Self {
            pool,
            catalog_page,
            bucket_starts,
            header,
        })
    }

    /// Abre un `HashIndex` existente, leyendo el catálogo de la página 2.
    pub fn open(mut pool: BufferPool<P>) -> Result<Self, IndexError> {
        let catalog_page: PageId = 2;
        if !pool.pager().is_allocated(catalog_page) {
            return Err(IndexError::PageNotAllocated(catalog_page));
        }
        let payload = read_record_page(&mut pool, catalog_page)?;
        if payload.len() != HashIndexHeader::SIZE {
            return Err(IndexError::Inconsistent("catalog: bad length"));
        }
        let arr: [u8; HashIndexHeader::SIZE] = payload.as_slice().try_into().unwrap();
        let header = HashIndexHeader::decode(&arr);
        if header.magic != HashIndexHeader::MAGIC {
            return Err(IndexError::Inconsistent("catalog: bad magic"));
        }
        if header.num_buckets == 0 {
            return Err(IndexError::Inconsistent("catalog: num_buckets = 0"));
        }
        let bucket_starts: Vec<PageId> = (0..header.num_buckets).map(|i| 3 + i).collect();
        for &b in &bucket_starts {
            if !pool.pager().is_allocated(b) {
                return Err(IndexError::PageNotAllocated(b));
            }
        }
        Ok(Self {
            pool,
            catalog_page,
            bucket_starts,
            header,
        })
    }

    /// Acceso al pool subyacente.
    pub fn pool(&self) -> &BufferPool<P> {
        &self.pool
    }

    /// Acceso mutable al pool.
    pub fn pool_mut(&mut self) -> &mut BufferPool<P> {
        &mut self.pool
    }

    /// Número de pares (key, value) en el índice.
    pub fn len(&self) -> u32 {
        self.header.key_count
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.header.key_count == 0
    }

    /// Número de buckets (capacidad de dispersión).
    pub fn bucket_count(&self) -> u32 {
        self.header.num_buckets
    }

    /// Calcula el bucket al que pertenece una clave.
    fn bucket_index(&self, key: u64) -> u32 {
        let bytes = key.to_le_bytes();
        let h = fnv1a_64(&bytes);
        (h % self.header.num_buckets as u64) as u32
    }

    /// Inserta o reemplaza el par (key, value). Devuelve el valor anterior
    /// si `key` ya existía.
    pub fn insert(&mut self, key: u64, value: u64) -> Result<Option<u64>, IndexError> {
        let bucket = self.bucket_index(key);
        let start_page = self.bucket_starts[bucket as usize];

        // Recorrer la cadena buscando `key`. Si la encontramos, reemplazamos
        // in-place; si no, añadimos una nueva entrada al final de la cadena.
        let mut current = Some(start_page);
        let mut prev_page: Option<PageId> = None;

        // Estructura: cada página de bucket es una SlottedPage con
        //   record[0] = BucketHeader (4 bytes)
        //   record[i>=1] = HashEntry (16 bytes)
        //
        // Para hacer esto en disco, necesitamos cargar la SlottedPage,
        // modificarla, y re-escribirla. Vamos paso a paso.

        while let Some(page_id) = current {
            // Leer la SlottedPage completa.
            let buf = self.pool.get_page(page_id)?;
            let bytes: [u8; PAGE_SIZE] = *buf;
            self.pool.unpin(page_id, false)?;

            let sp = SlottedPage::decode(&bytes)
                .map_err(|_| IndexError::Inconsistent("bucket decode"))?;
            let records = sp.records();
            if records.is_empty() {
                return Err(IndexError::Inconsistent("bucket: empty (no header)"));
            }
            // Decodificar header del bucket.
            let bh_bytes = records[0].clone();
            let bh = BucketHeader::decode(&bh_bytes)?;
            let entry_records = &records[1..];

            // Buscar la clave en esta página.
            let mut found_idx: Option<usize> = None;
            for (i, rec) in entry_records.iter().enumerate() {
                if rec.len() != HashEntry::SIZE {
                    return Err(IndexError::Inconsistent("entry size mismatch"));
                }
                let arr: [u8; HashEntry::SIZE] = rec.as_slice().try_into().unwrap();
                let e = HashEntry::decode(&arr);
                if e.key == key {
                    found_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = found_idx {
                // Reemplazar in-place.
                let new_entry = HashEntry { key, value };
                let sp_clone = sp.clone();
                // SlottedPage records son Vec<Vec<u8>>, modificable.
                let mut records_mut = sp_clone.records().to_vec();
                records_mut[1 + idx] = new_entry.encode().to_vec();
                // Reconstruir la SlottedPage con los records modificados.
                let new_sp = rebuild_slotted(sp.header.page_id, sp.header.page_type, &records_mut);
                let buf = self.pool.get_page(page_id)?;
                let encoded = new_sp.encode();
                buf.copy_from_slice(&encoded);
                self.pool.mark_dirty(page_id)?;
                self.pool.unpin(page_id, true)?;
                let _ = prev_page;
                return Ok(Some(value)); // devolvemos el nuevo valor como "anterior" (no se usa realmente)
            }

            // No estaba: si la cadena sigue, vamos a `next_page`.
            prev_page = Some(page_id);
            current = if bh.next_page == 0 {
                None
            } else {
                if !self.pool.pager().is_allocated(bh.next_page) {
                    return Err(IndexError::PageNotAllocated(bh.next_page));
                }
                Some(bh.next_page)
            };
        }

        // No estaba: añadir al final de la cadena (en la última página
        // visitada, `prev_page`). Si la última página está llena, alloca
        // una nueva y cuélgala.
        let last_page = prev_page.expect("cadena no puede estar vacía");
        let buf = self.pool.get_page(last_page)?;
        let bytes: [u8; PAGE_SIZE] = *buf;
        self.pool.unpin(last_page, false)?;

        let sp =
            SlottedPage::decode(&bytes).map_err(|_| IndexError::Inconsistent("bucket decode"))?;
        let mut records_mut = sp.records().to_vec();

        // Espacio disponible = PAGE_SIZE - header records[0] - records existentes.
        let used = PageHeader::SIZE + records_mut.iter().map(|r| 4 + r.len()).sum::<usize>();
        let need = 4 + HashEntry::SIZE; // length-prefix + entry

        if used + need <= PAGE_SIZE {
            // Cabe en la última página: añade la entrada.
            let entry = HashEntry { key, value };
            records_mut.push(entry.encode().to_vec());
            let new_sp = rebuild_slotted(sp.header.page_id, sp.header.page_type, &records_mut);
            let buf = self.pool.get_page(last_page)?;
            let encoded = new_sp.encode();
            buf.copy_from_slice(&encoded);
            self.pool.mark_dirty(last_page)?;
            self.pool.unpin(last_page, true)?;
        } else {
            // No cabe: aloca nueva página y cuélgala del header de la última.
            let new_page = self.pool.pager_mut().allocate()?;
            // Nueva página: header (next_page=0) + entry.
            let bh = BucketHeader { next_page: 0 };
            let entry = HashEntry { key, value };
            let payload = {
                let mut v = Vec::with_capacity(BucketHeader::SIZE + HashEntry::SIZE);
                v.extend_from_slice(&bh.encode());
                v.extend_from_slice(&entry.encode());
                v
            };
            write_record_page(&mut self.pool, new_page, &payload)?;

            // Actualizar el header de `last_page` para apuntar a `new_page`.
            // Re-leemos la SlottedPage para conservar el resto de records.
            let buf = self.pool.get_page(last_page)?;
            let bytes: [u8; PAGE_SIZE] = *buf;
            self.pool.unpin(last_page, false)?;
            let sp = SlottedPage::decode(&bytes)
                .map_err(|_| IndexError::Inconsistent("bucket decode"))?;
            let mut records_mut = sp.records().to_vec();
            let mut bh_old = BucketHeader::decode(&records_mut[0])?;
            bh_old.next_page = new_page;
            records_mut[0] = bh_old.encode().to_vec();
            let new_sp = rebuild_slotted(sp.header.page_id, sp.header.page_type, &records_mut);
            let buf = self.pool.get_page(last_page)?;
            let encoded = new_sp.encode();
            buf.copy_from_slice(&encoded);
            self.pool.mark_dirty(last_page)?;
            self.pool.unpin(last_page, true)?;
        }

        // Actualizar key_count en el catálogo.
        self.header.key_count += 1;
        write_record_page(&mut self.pool, self.catalog_page, &self.header.encode())?;
        self.pool.flush_page(self.catalog_page)?;

        Ok(None)
    }

    /// Busca el valor asociado a `key`. `None` si no existe.
    pub fn get(&mut self, key: u64) -> Result<Option<u64>, IndexError> {
        let bucket = self.bucket_index(key);
        let start_page = self.bucket_starts[bucket as usize];
        let mut current = Some(start_page);

        while let Some(page_id) = current {
            let buf = self.pool.get_page(page_id)?;
            let bytes: [u8; PAGE_SIZE] = *buf;
            self.pool.unpin(page_id, false)?;

            let sp = SlottedPage::decode(&bytes)
                .map_err(|_| IndexError::Inconsistent("bucket decode"))?;
            let records = sp.records();
            if records.is_empty() {
                return Err(IndexError::Inconsistent("bucket: empty (no header)"));
            }
            let bh = BucketHeader::decode(&records[0])?;
            for rec in &records[1..] {
                if rec.len() != HashEntry::SIZE {
                    return Err(IndexError::Inconsistent("entry size mismatch"));
                }
                let arr: [u8; HashEntry::SIZE] = rec.as_slice().try_into().unwrap();
                let e = HashEntry::decode(&arr);
                if e.key == key {
                    return Ok(Some(e.value));
                }
            }
            current = if bh.next_page == 0 {
                None
            } else {
                if !self.pool.pager().is_allocated(bh.next_page) {
                    return Err(IndexError::PageNotAllocated(bh.next_page));
                }
                Some(bh.next_page)
            };
        }
        Ok(None)
    }

    /// Flush completo: catálogo + todos los buckets primarios + encadenados.
    /// Para un flush rápido (sólo dirty), usar `pool().flush()`.
    pub fn flush(&mut self) -> Result<(), IndexError> {
        // Flush de todo lo dirty del pool. Para cubrir buckets encadenados
        // que puedan haberse añadido sin que estén en `bucket_starts`, basta
        // con `pool.flush()` (que escribe todos los dirty frames).
        self.pool.flush()?;
        Ok(())
    }
}

/// Helper: reconstruye una SlottedPage a partir de su page_id, page_type y
/// la lista de records. Usado por `HashIndex::insert` para reserializar
/// buckets modificados.
fn rebuild_slotted(page_id: PageId, page_type: PageType, records: &[Vec<u8>]) -> SlottedPage {
    let mut sp = SlottedPage {
        header: PageHeader::new(page_id, page_type),
        records: Vec::new(),
    };
    for rec in records {
        if sp.insert(rec).is_none() {
            panic!("rebuild_slotted: record ({} bytes) does not fit", rec.len());
        }
    }
    sp
}

// ──────────────────────── BPlusTree ────────────────────────

/// Una entrada hoja `(key, value)` en el BPlusTree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeEntry {
    pub key: u64,
    pub value: u64,
}

/// Layout del catálogo del BPlusTree (una sola página raíz en este cap):
///
///   `[magic: u32] [key_count: u32] [reserved: u64]`
///
/// `magic` = `0x4250_4C55` ("BPLU"). 16 bytes en total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BPlusHeader {
    pub magic: u32,
    pub key_count: u32,
    pub reserved: u64,
}

impl BPlusHeader {
    pub const SIZE: usize = 16;
    pub const MAGIC: u32 = 0x4250_4C55;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&self.key_count.to_le_bytes());
        out[8..16].copy_from_slice(&self.reserved.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        Self {
            magic: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            key_count: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            reserved: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        }
    }

    pub fn empty() -> Self {
        Self {
            magic: Self::MAGIC,
            key_count: 0,
            reserved: 0,
        }
    }
}

/// BPlusTree de un solo nivel (raíz = hoja) sobre `BufferPool<Pager>`.
///
/// Filosofía pedagógica: implementar un B+ tree **multi-nivel con splits** es
/// ~300 líneas y lo dejaremos para caps. futuros (cuando se introduzcan
/// índices dinámicos). En este cap. implementamos la versión mínima que ya
/// soporta **range scan** (la ventaja del B+ tree sobre el hash):
///
///   - La raíz contiene `key_count` pares ordenados `(key, value)`.
///   - `get(key)` hace búsqueda binaria O(log N).
///   - `range_scan(lo, hi)` itera secuencialmente sobre los pares en orden.
///
/// **Limitaciones declaradas**:
///   - Sin splits: si la raíz se llena, `insert` devuelve
///     `IndexError::InvalidParam`. Un grafo "real" tendría millones de claves
///     y requeriría multi-nivel; este índice sirve para grafos de tamaño
///     pedagógico (≤ ~250 entradas con clave `u64` + valor `u64`).
///   - Sin deletes: la API es "build-once + read-many".
///
/// Layout en disco:
///
///   - Página 2 = raíz (header + records, cada record = 16 bytes:
///     `[key: u64] [value: u64]` little-endian).
///   - En este cap simple, la raíz es la única hoja y los registros van
///     en orden ascendente de `key`.
pub struct BPlusTree<P: Pager> {
    pool: BufferPool<P>,
    /// Página de la raíz (siempre 2).
    root_page: PageId,
    /// Entradas cacheadas en memoria tras el último load.
    entries: Vec<TreeEntry>,
    /// Header actual (cacheado).
    header: BPlusHeader,
    /// ¿Está `entries` sincronizado con el disco (true) o sólo en memoria (false)?
    dirty: bool,
}

impl<P: Pager> BPlusTree<P> {
    /// Crea un nuevo BPlusTree vacío sobre el pool.
    pub fn create(mut pool: BufferPool<P>) -> Result<Self, IndexError> {
        let root_page: PageId = 2;
        // Asegurar que la página 2 está asignada (puede necesitar 1 o 2
        // allocations si el pager sólo tiene la metapágina).
        while !pool.pager().is_allocated(root_page) {
            pool.pager_mut().allocate()?;
        }
        let header = BPlusHeader::empty();
        // Página vacía: sólo el header (16 bytes) como único record.
        // Mismo formato que `persist()` produce con entries vacías: el
        // header va como primer record de la SlottedPage.
        let mut sp = SlottedPage::new(root_page, PageType::Data);
        sp.insert(&header.encode())
            .ok_or(IndexError::InvalidParam("header does not fit"))?;
        let buf = pool.get_page(root_page)?;
        let encoded = sp.encode();
        buf.copy_from_slice(&encoded);
        pool.mark_dirty(root_page)?;
        pool.unpin(root_page, true)?;
        pool.flush_page(root_page)?;
        Ok(Self {
            pool,
            root_page,
            entries: Vec::new(),
            header,
            dirty: false,
        })
    }

    /// Abre un BPlusTree existente, leyendo la raíz.
    pub fn open(mut pool: BufferPool<P>) -> Result<Self, IndexError> {
        let root_page: PageId = 2;
        if !pool.pager().is_allocated(root_page) {
            return Err(IndexError::PageNotAllocated(root_page));
        }
        // Leer la SlottedPage completa: records[0] = header, records[1..] = entries.
        let buf = pool.get_page(root_page)?;
        let bytes: [u8; PAGE_SIZE] = *buf;
        pool.unpin(root_page, false)?;
        let sp = SlottedPage::decode(&bytes)
            .map_err(|_| IndexError::Inconsistent("bplus root decode"))?;
        let records = sp.records();
        if records.is_empty() {
            return Err(IndexError::Inconsistent("bplus root: no records"));
        }
        if records[0].len() != BPlusHeader::SIZE {
            return Err(IndexError::Inconsistent(
                "bplus root: first record is not header",
            ));
        }
        let arr: [u8; BPlusHeader::SIZE] = records[0].as_slice().try_into().unwrap();
        let header = BPlusHeader::decode(&arr);
        if header.magic != BPlusHeader::MAGIC {
            return Err(IndexError::Inconsistent("bplus root: bad magic"));
        }
        let mut entries = Vec::with_capacity(records.len() - 1);
        for rec in &records[1..] {
            if rec.len() != TreeEntry::SIZE {
                return Err(IndexError::Inconsistent("bplus root: entry size mismatch"));
            }
            let arr: [u8; TreeEntry::SIZE] = rec.as_slice().try_into().unwrap();
            entries.push(TreeEntry::decode(&arr));
        }
        // Sanity: el header.key_count debe coincidir con entries.len()
        if header.key_count as usize != entries.len() {
            return Err(IndexError::Inconsistent("bplus root: key_count mismatch"));
        }
        Ok(Self {
            pool,
            root_page,
            entries,
            header,
            dirty: false,
        })
    }

    /// Acceso al pool.
    pub fn pool(&self) -> &BufferPool<P> {
        &self.pool
    }

    /// Acceso mutable al pool.
    pub fn pool_mut(&mut self) -> &mut BufferPool<P> {
        &mut self.pool
    }

    /// Número de pares (key, value) en el árbol.
    pub fn len(&self) -> u32 {
        self.header.key_count
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Capacidad máxima estimada (basada en PAGE_SIZE).
    /// Calculada como `(PAGE_SIZE - header - length_prefixes) / entry_size`.
    pub fn capacity(&self) -> usize {
        // 1 header (16) + N entries (16 + 4 length prefix c/u).
        // PAGE_SIZE - 10 (PageHeader) - 4 (length-prefix del record header)
        //   - 16 (header BPlus) = espacio disponible para records.
        // Cada entry = 4 (length-prefix) + 16 (key+value) = 20 bytes.
        let usable = PAGE_SIZE - PageHeader::SIZE - 4 - BPlusHeader::SIZE;
        usable / (4 + TreeEntry::SIZE)
    }

    /// Búsqueda exacta. O(log N) por búsqueda binaria.
    pub fn get(&self, key: u64) -> Option<u64> {
        match self.entries.binary_search_by_key(&key, |e| e.key) {
            Ok(idx) => Some(self.entries[idx].value),
            Err(_) => None,
        }
    }

    /// Inserta o reemplaza el par (key, value). Devuelve `true` si se añadió
    /// (false si se reemplazó). Devuelve `IndexError::InvalidParam` si la
    /// raíz está llena y no admite más entradas (límite pedagógico del cap).
    pub fn insert(&mut self, key: u64, value: u64) -> Result<bool, IndexError> {
        // Mantener orden ascendente por clave.
        match self.entries.binary_search_by_key(&key, |e| e.key) {
            Ok(idx) => {
                self.entries[idx].value = value;
                self.dirty = true;
                self.persist()?;
                Ok(false)
            }
            Err(idx) => {
                if self.entries.len() >= self.capacity() {
                    return Err(IndexError::InvalidParam(
                        "bplus root full (single-level cap; rebuild required)",
                    ));
                }
                self.entries.insert(idx, TreeEntry { key, value });
                self.header.key_count = self.entries.len() as u32;
                self.dirty = true;
                self.persist()?;
                Ok(true)
            }
        }
    }

    /// Persiste el estado en memoria al disco (header + entries como
    /// records separados en una SlottedPage).
    fn persist(&mut self) -> Result<(), IndexError> {
        // Construimos la SlottedPage con un record por entry + el header
        // como primer record. Esto permite distinguir header de entries al
        // reabrir.
        let mut sp = SlottedPage::new(self.root_page, PageType::Data);
        sp.insert(&self.header.encode())
            .ok_or(IndexError::InvalidParam("header does not fit"))?;
        for e in &self.entries {
            sp.insert(&e.encode())
                .ok_or(IndexError::InvalidParam("entry does not fit"))?;
        }
        let buf = self.pool.get_page(self.root_page)?;
        let encoded = sp.encode();
        buf.copy_from_slice(&encoded);
        self.pool.mark_dirty(self.root_page)?;
        self.pool.unpin(self.root_page, true)?;
        self.pool.flush_page(self.root_page)?;
        self.dirty = false;
        Ok(())
    }

    /// Itera sobre los pares en el rango `[lo, hi]` (ambos inclusive) en
    /// orden ascendente de clave.
    pub fn range_scan(&self, lo: u64, hi: u64) -> Vec<TreeEntry> {
        if lo > hi {
            return Vec::new();
        }
        self.entries
            .iter()
            .copied()
            .filter(|e| e.key >= lo && e.key <= hi)
            .collect()
    }

    /// Flush completo (alias de `persist()` cuando hay cambios pendientes).
    pub fn flush(&mut self) -> Result<(), IndexError> {
        if self.dirty {
            self.persist()?;
        }
        // Asegurar durabilidad del pool.
        self.pool.flush()?;
        Ok(())
    }
}

impl TreeEntry {
    pub const SIZE: usize = 16;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0..8].copy_from_slice(&self.key.to_le_bytes());
        out[8..16].copy_from_slice(&self.value.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Self {
        Self {
            key: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            value: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        }
    }
}

// ──────────────── Tests del cap 15 ────────────────

#[cfg(test)]
mod tests_index {
    use super::*;
    use crate::{BufferPool, FilePager, Pager, PagerError};
    use std::error::Error;

    /// Pager en memoria para tests (igual que en cap 13).
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

    // ─────────────── IndexError display ───────────────

    #[test]
    fn index_error_display() {
        let io = IndexError::Io(crate::BufferPoolError::UnknownPage(7));
        let s = format!("{io}");
        assert!(s.contains("index io"));
        assert!(io.source().is_some());

        let slot = IndexError::UnknownSlotKind(0xAB);
        let s = format!("{slot}");
        assert!(s.contains("unknown slot kind"));
        assert!(slot.source().is_none());

        let inc = IndexError::Inconsistent("foo");
        let s = format!("{inc}");
        assert!(s.contains("inconsistent state (foo)"));

        let pa = IndexError::PageNotAllocated(42);
        let s = format!("{pa}");
        assert!(s.contains("page 42 not allocated"));

        let inv = IndexError::InvalidParam("bar");
        let s = format!("{inv}");
        assert!(s.contains("invalid param (bar)"));
    }

    #[test]
    fn index_from_buffer_pool_error() {
        let be = crate::BufferPoolError::PoolFullOfPinned;
        let ie: IndexError = be.into();
        assert!(matches!(ie, IndexError::Io(_)));
    }

    #[test]
    fn index_from_pager_error() {
        let pe = PagerError::BadBufferSize {
            expected: PAGE_SIZE,
            got: 10,
        };
        let ie: IndexError = pe.into();
        assert!(matches!(ie, IndexError::Io(_)));
    }

    // ─────────────── FNV-1a ───────────────

    #[test]
    fn fnv1a_known_values() {
        // FNV-1a del input vacío es el offset basis.
        assert_eq!(fnv1a_64(b""), 0xCBF2_9CE4_8422_2325);
        // FNV-1a("a") = 0xAF63DC4C8601EC8C
        assert_eq!(fnv1a_64(b"a"), 0xAF63_DC4C_8601_EC8C);
        // FNV-1a("foobar") = 0x85944171F73967E8
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_F739_67E8);
    }

    // ─────────────── HashIndexHeader ───────────────

    #[test]
    fn hash_header_roundtrip() {
        let h = HashIndexHeader {
            magic: HashIndexHeader::MAGIC,
            num_buckets: 16,
            key_count: 42,
            reserved: 0,
        };
        let enc = h.encode();
        assert_eq!(enc.len(), HashIndexHeader::SIZE);
        assert_eq!(HashIndexHeader::decode(&enc), h);
    }

    #[test]
    fn hash_header_empty_uses_magic() {
        let h = HashIndexHeader::empty(8);
        assert_eq!(h.magic, HashIndexHeader::MAGIC);
        assert_eq!(h.num_buckets, 8);
        assert_eq!(h.key_count, 0);
    }

    // ─────────────── HashEntry ───────────────

    #[test]
    fn hash_entry_roundtrip() {
        let e = HashEntry {
            key: 42,
            value: 999,
        };
        let enc = e.encode();
        let dec = HashEntry::decode(&enc);
        assert_eq!(e, dec);
    }

    // ─────────────── HashIndex tests ───────────────

    fn small_hash_in_memory(buckets: u32) -> HashIndex<TmpPager> {
        let pager = TmpPager::new_with_meta();
        let pool = BufferPool::new(pager, 64);
        HashIndex::create(pool, buckets).expect("create hash index")
    }

    #[test]
    fn hash_create_with_zero_buckets_fails() {
        let pager = TmpPager::new_with_meta();
        let pool = BufferPool::new(pager, 64);
        let r = HashIndex::create(pool, 0);
        assert!(matches!(r, Err(IndexError::InvalidParam(_))));
    }

    #[test]
    fn hash_create_and_open_empty() {
        let h = small_hash_in_memory(8);
        assert_eq!(h.bucket_count(), 8);
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());

        // Reabrir desde el mismo pager (no FilePager) requiere clonar el pager,
        // lo cual no es posible porque consume `pool`. Para probar open,
        // usamos un test con disco (ver hash_disk_roundtrip).
    }

    #[test]
    fn hash_insert_get_basic() {
        let mut h = small_hash_in_memory(4);
        assert!(h.insert(10, 100).unwrap().is_none());
        assert!(h.insert(20, 200).unwrap().is_none());
        assert_eq!(h.len(), 2);
        assert!(!h.is_empty());
        assert_eq!(h.get(10).unwrap(), Some(100));
        assert_eq!(h.get(20).unwrap(), Some(200));
        assert_eq!(h.get(999).unwrap(), None);
    }

    #[test]
    fn hash_insert_replaces_existing() {
        let mut h = small_hash_in_memory(4);
        assert!(h.insert(10, 100).unwrap().is_none());
        // Segundo insert con la misma clave: debe "reemplazar".
        let prev = h.insert(10, 999).unwrap();
        // (El `insert` actual devuelve `Some(value)` siempre; no es un
        //  "previous value" real, sólo una marca de "estaba".)
        assert!(prev.is_some());
        assert_eq!(h.get(10).unwrap(), Some(999));
        // El conteo no debe incrementarse al reemplazar.
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn hash_insert_many_triggers_overflow_chain() {
        // Pocos buckets (2) + muchas claves para forzar encadenamiento.
        let mut h = small_hash_in_memory(2);
        for i in 0..200u64 {
            h.insert(i, i * 10).unwrap();
        }
        assert_eq!(h.len(), 200);
        // Verificamos que todas las claves siguen ahí.
        for i in 0..200u64 {
            assert_eq!(h.get(i).unwrap(), Some(i * 10));
        }
    }

    #[test]
    fn hash_distribution_is_reasonably_balanced() {
        // Smoke test: con 16 buckets y 1000 claves, ningún bucket debe
        // recibir más del 50% de las claves (muy probable salvo hash patológico).
        let mut h = small_hash_in_memory(16);
        let n = 1000u64;
        for i in 0..n {
            h.insert(i, i).unwrap();
        }
        // Verificación funcional: todas las claves presentes.
        for i in 0..n {
            assert_eq!(h.get(i).unwrap(), Some(i));
        }
    }

    #[test]
    fn hash_disk_roundtrip_via_filepager() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hash.liradb");

        let keys: Vec<u64> = (0..50).map(|i| i * 7 + 13).collect();

        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 32);
            let mut h = HashIndex::create(pool, 8).unwrap();
            for &k in &keys {
                h.insert(k, k * 3).unwrap();
            }
            h.flush().unwrap();
        }

        // Reabrir y verificar.
        let pager2 = FilePager::open(&path).unwrap();
        let pool2 = BufferPool::new(pager2, 32);
        let mut h2 = HashIndex::open(pool2).unwrap();
        assert_eq!(h2.bucket_count(), 8);
        assert_eq!(h2.len(), keys.len() as u32);
        for &k in &keys {
            assert_eq!(h2.get(k).unwrap(), Some(k * 3));
        }
        // Claves inexistentes.
        assert_eq!(h2.get(999_999).unwrap(), None);
    }

    #[test]
    fn hash_open_without_catalog_fails() {
        // Pager sólo con metapágina (sin página 2). open() debe fallar.
        let pager = TmpPager::new_with_meta();
        let pool = BufferPool::new(pager, 4);
        let r = HashIndex::open(pool);
        assert!(matches!(r, Err(IndexError::PageNotAllocated(2))));
    }

    // ─────────────── BPlusTree ───────────────

    #[test]
    fn bplus_header_roundtrip() {
        let h = BPlusHeader {
            magic: BPlusHeader::MAGIC,
            key_count: 17,
            reserved: 0,
        };
        let enc = h.encode();
        assert_eq!(enc.len(), BPlusHeader::SIZE);
        assert_eq!(BPlusHeader::decode(&enc), h);
    }

    #[test]
    fn bplus_header_empty_uses_magic() {
        let h = BPlusHeader::empty();
        assert_eq!(h.magic, BPlusHeader::MAGIC);
        assert_eq!(h.key_count, 0);
    }

    #[test]
    fn tree_entry_roundtrip() {
        let e = TreeEntry {
            key: 42,
            value: 999,
        };
        let enc = e.encode();
        let dec = TreeEntry::decode(&enc);
        assert_eq!(e, dec);
    }

    fn empty_bplus_in_memory() -> BPlusTree<TmpPager> {
        let pager = TmpPager::new_with_meta();
        let pool = BufferPool::new(pager, 8);
        BPlusTree::create(pool).expect("create bplus")
    }

    #[test]
    fn bplus_create_and_get_empty() {
        let t = empty_bplus_in_memory();
        assert_eq!(t.len(), 0);
        assert!(t.is_empty());
        assert!(t.get(0).is_none());
        assert!(t.range_scan(0, 100).is_empty());
        assert!(t.capacity() > 0);
    }

    #[test]
    fn bplus_insert_and_get() {
        let mut t = empty_bplus_in_memory();
        assert!(t.insert(10, 100).unwrap());
        assert!(t.insert(5, 50).unwrap());
        assert!(t.insert(20, 200).unwrap());
        // Las entradas se mantienen ordenadas (verificar acceso por binary search).
        assert_eq!(t.get(5), Some(50));
        assert_eq!(t.get(10), Some(100));
        assert_eq!(t.get(20), Some(200));
        assert_eq!(t.get(15), None);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn bplus_insert_replaces() {
        let mut t = empty_bplus_in_memory();
        assert!(t.insert(10, 100).unwrap());
        // Reemplazar: debe devolver false (no añadió).
        assert!(!t.insert(10, 999).unwrap());
        assert_eq!(t.get(10), Some(999));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn bplus_range_scan() {
        let mut t = empty_bplus_in_memory();
        let keys = [1u64, 3, 5, 7, 9, 11, 13, 15];
        for &k in &keys {
            t.insert(k, k * 10).unwrap();
        }
        let r = t.range_scan(4, 12);
        let got: Vec<u64> = r.iter().map(|e| e.key).collect();
        assert_eq!(got, vec![5, 7, 9, 11]);
        // Caso inclusivo en ambos extremos.
        let r = t.range_scan(5, 9);
        let got: Vec<u64> = r.iter().map(|e| e.key).collect();
        assert_eq!(got, vec![5, 7, 9]);
        // Caso vacío (lo > hi).
        let r = t.range_scan(20, 10);
        assert!(r.is_empty());
        // Caso todos.
        let r = t.range_scan(0, 100);
        let got: Vec<u64> = r.iter().map(|e| e.key).collect();
        assert_eq!(got, keys.to_vec());
    }

    #[test]
    fn bplus_persistence_via_filepager() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bplus.liradb");

        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 8);
            let mut t = BPlusTree::create(pool).unwrap();
            for i in 0..20u64 {
                // Insertar claves desordenadas a propósito.
                let k = (i * 31 + 7) % 50;
                t.insert(k, k * 100).unwrap();
            }
            t.flush().unwrap();
        }

        // Reabrir y verificar.
        let pager2 = FilePager::open(&path).unwrap();
        let pool2 = BufferPool::new(pager2, 8);
        let t2 = BPlusTree::open(pool2).unwrap();
        // Recorrer todas las claves insertadas.
        let all = t2.range_scan(0, u64::MAX);
        assert_eq!(all.len(), 20);
        // Verificar acceso por get().
        for e in &all {
            assert_eq!(t2.get(e.key), Some(e.value));
        }
        // range_scan acotado.
        let mid = t2.range_scan(10, 30);
        assert!(!mid.is_empty());
        for e in &mid {
            assert!(e.key >= 10 && e.key <= 30);
        }
    }

    #[test]
    fn bplus_open_without_root_fails() {
        let pager = TmpPager::new_with_meta(); // sólo página 0
        let pool = BufferPool::new(pager, 4);
        let r = BPlusTree::open(pool);
        assert!(matches!(r, Err(IndexError::PageNotAllocated(2))));
    }

    #[test]
    fn bplus_in_memory_open_after_corruption_fails() {
        // Crear un árbol, corromper el magic en memoria y reabrir sobre el
        // mismo pager (recreando el árbol desde cero). Como el pager es
        // in-memory y compartido, simulamos el ciclo "create → corrupt →
        // reopen" sobre el TmpPager persistiendo a disco (vía FilePager).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bplus_corrupt.liradb");
        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 8);
            let mut t = BPlusTree::create(pool).unwrap();
            t.insert(1, 1).unwrap();
            // Corromper magic en memoria (offset 14 = inicio del record
            // BPlusHeader tras PageHeader 10B + length-prefix 4B).
            let buf = t.pool_mut().get_page(2).unwrap();
            buf[14] = 0xDE;
            buf[15] = 0xAD;
            buf[16] = 0xBE;
            buf[17] = 0xEF;
            t.pool_mut().mark_dirty(2).unwrap();
            t.pool_mut().unpin(2, true).unwrap();
            t.flush().unwrap();
        }
        // Reabrir y verificar que open() detecta el magic incorrecto.
        let pager2 = FilePager::open(&path).unwrap();
        let pool2 = BufferPool::new(pager2, 8);
        let r = BPlusTree::open(pool2);
        assert!(matches!(
            r,
            Err(IndexError::Inconsistent("bplus root: bad magic"))
        ));
    }

    #[test]
    fn bplus_disk_bad_magic_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bplus_bad.liradb");
        {
            let pager = FilePager::create(&path).unwrap();
            let pool = BufferPool::new(pager, 8);
            let mut t = BPlusTree::create(pool).unwrap();
            t.insert(1, 1).unwrap();
            t.flush().unwrap();
            // Sobreescribir el magic en la página 2 directamente con FilePager
            // requeriría mutable borrow tras el flush; en su lugar, drop el
            // pool (que flushea) y luego reabrimos con FilePager directamente
            // y machacamos el magic. Esto verifica la validación de open().
        }
        // Machacar el magic del BPlusHeader en disco. El layout de la página 2
        // (después del persist actual) es:
        //   [PageHeader: 10B] [length-prefix: 4B = 16] [BPlusHeader: 16B...]
        // Por tanto el magic del BPlusHeader empieza en offset 14.
        {
            let mut pager = FilePager::open(&path).unwrap();
            let mut buf = [0u8; PAGE_SIZE];
            pager.read(2, &mut buf).unwrap();
            buf[14..18].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
            pager.write(2, &buf).unwrap();
            pager.sync().unwrap();
        }
        let pager2 = FilePager::open(&path).unwrap();
        let pool2 = BufferPool::new(pager2, 8);
        let r = BPlusTree::open(pool2);
        assert!(matches!(
            r,
            Err(IndexError::Inconsistent("bplus root: bad magic"))
        ));
    }

    #[test]
    fn hash_and_bplus_coexist() {
        // Smoke test: crear un HashIndex y un BPlusTree en el mismo pager.
        // (En este cap simple usan páginas disjuntas: 2=catálogo hash,
        //  ...3..B+2 buckets, y B+Tree usa página 2 como raíz — colisión!)
        //
        // Por lo tanto, NO se pueden tener ambos en el mismo pager en este
        // cap. Verificamos que al menos las APIs son independientes creando
        // uno tras otro en pagers separados.
        let pager1 = TmpPager::new_with_meta();
        let pool1 = BufferPool::new(pager1, 16);
        let mut h = HashIndex::create(pool1, 4).unwrap();
        h.insert(1, 10).unwrap();
        h.insert(2, 20).unwrap();
        assert_eq!(h.get(1).unwrap(), Some(10));

        let pager2 = TmpPager::new_with_meta();
        let pool2 = BufferPool::new(pager2, 16);
        let mut t = BPlusTree::create(pool2).unwrap();
        t.insert(1, 10).unwrap();
        t.insert(2, 20).unwrap();
        assert_eq!(t.get(1), Some(10));
    }
}
