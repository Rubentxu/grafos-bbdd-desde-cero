use crate::cap11_slotted_pages::PAGE_SIZE;

// ─────────────────── Cap 12: trait Pager + FilePager ───────────────────
//
// Responsabilidades del gestor de páginas:
//   - Crear páginas (allocate).
//   - Leer páginas a un buffer (read).
//   - Escribir páginas desde un buffer (write).
//   - Liberar páginas (free, vía free list interna).
//   - Sincronizar el estado con disco (sync).
//
// Diseño: trait `Pager` (port) + varias implementaciones (adapter).
// En este capítulo implementamos `FilePager` (basado en `std::fs::File`).
// En un capítulo posterior (apéndice comparativo) se añadiría `MmapPager`.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Identificador de página (índice 0-based; la página 0 es la metapágina).
pub type PageId = u32;

/// Errores del gestor de páginas.
#[derive(Debug)]
pub enum PagerError {
    /// Error de E/S subyacente (lectura, escritura, seek, flush).
    Io(std::io::Error),
    /// El `PageId` solicitado está fuera del rango de páginas del fichero.
    OutOfRange { requested: PageId, num_pages: u32 },
    /// La página solicitada está en la free list (no fue asignada todavía).
    FreePage(PageId),
    /// El buffer pasado a read/write no tiene `PAGE_SIZE` bytes.
    BadBufferSize { expected: usize, got: usize },
    /// Overflow: no quedan IDs de página disponibles (4 GiB agotados).
    NoFreePageId,
}

impl std::fmt::Display for PagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PagerError::Io(e) => write!(f, "pager io error: {e}"),
            PagerError::OutOfRange {
                requested,
                num_pages,
            } => {
                write!(f, "page {requested} out of range (num_pages={num_pages})")
            }
            PagerError::FreePage(id) => write!(f, "page {id} is in free list"),
            PagerError::BadBufferSize { expected, got } => {
                write!(f, "buffer size {got} != expected {expected}")
            }
            PagerError::NoFreePageId => write!(f, "no free PageId available"),
        }
    }
}

impl std::error::Error for PagerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PagerError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PagerError {
    fn from(e: std::io::Error) -> Self {
        PagerError::Io(e)
    }
}

/// Trait del gestor de páginas (port en arquitectura hexagonal).
///
/// El buffer `page` en `read`/`write` debe tener exactamente `PAGE_SIZE` bytes
/// (ver constante [`PAGE_SIZE`]). El caller es responsable de codificar el
/// contenido específico (e.g. `SlottedPage`, `MetaPage`).
pub trait Pager {
    /// Asigna una nueva página (nunca reutiliza un ID en la free list).
    /// Devuelve el `PageId` asignado.
    fn allocate(&mut self) -> Result<PageId, PagerError>;

    /// Lee la página `id` en el buffer `page` (debe tener `PAGE_SIZE` bytes).
    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError>;

    /// Escribe el buffer `page` (debe tener `PAGE_SIZE` bytes) en la página `id`.
    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError>;

    /// Sincroniza el estado del pager con disco (`fsync`/`fdatasync`).
    fn sync(&mut self) -> Result<(), PagerError>;

    /// Número total de páginas que el pager puede direccionar (incluyendo
    /// páginas en la free list, que existen en disco pero no están asignadas).
    fn num_pages(&self) -> u32;

    /// Libera una página: la marca como libre y la añade a la free list.
    /// La página sigue ocupando espacio en disco hasta un futuro `vacuum`.
    fn free(&mut self, id: PageId) -> Result<(), PagerError>;

    /// ¿Está la página `id` asignada (no en la free list)?
    fn is_allocated(&self, id: PageId) -> bool;

    /// Tamaño de página (en bytes) usado por este pager.
    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

/// `FilePager`: implementación basada en `std::fs::File`.
///
/// Estrategia:
///   - 1 fichero = N páginas de `PAGE_SIZE` bytes.
///   - Página `i` ocupa los bytes `[i*PAGE_SIZE .. (i+1)*PAGE_SIZE)`.
///   - `allocate` extiende el fichero con ceros (página vacía) y devuelve
///     el nuevo `PageId`; o reutiliza uno de la free list si existe.
///   - `free` añade el ID a la free list (no reduce el fichero).
///   - `sync` llama a `sync_all()` del `File` subyacente.
///
/// Decisiones pedagógicas (no óptimas, sí legibles):
///   - Sin `memmap2`. Toda I/O es `read`/`write`/`seek` de `std`.
///   - Free list en memoria (no persistida). Para producción habría que
///     guardarla en la metapágina; eso es tema del cap 14.
///   - Sin pre-allocación: el fichero crece página a página.
#[derive(Debug)]
pub struct FilePager {
    file: File,
    path: PathBuf,
    /// Número total de páginas direccionables (fichero_len / PAGE_SIZE).
    num_pages: u32,
    /// IDs de páginas liberadas, reutilizables por futuras `allocate`.
    free_list: Vec<PageId>,
}

impl FilePager {
    /// Abre un fichero existente como pager. Si el fichero no existe, devuelve
    /// `Io` con `NotFound`.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PagerError> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let len = file.metadata()?.len();
        if len % PAGE_SIZE as u64 != 0 {
            return Err(PagerError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("file size {len} not a multiple of PAGE_SIZE={PAGE_SIZE}"),
            )));
        }
        let num_pages = (len / PAGE_SIZE as u64) as u32;
        Ok(Self {
            file,
            path,
            num_pages,
            free_list: Vec::new(),
        })
    }

    /// Crea (o trunca) un fichero nuevo con sólo la metapágina (página 0).
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, PagerError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        // Reservar la página 0 escribiéndola vacía. Esto fija el tamaño del
        // fichero en PAGE_SIZE y deja la metapágina lista para `MetaPage`.
        let zeros = vec![0u8; PAGE_SIZE];
        file.write_all(&zeros)?;
        file.sync_all()?;
        Ok(Self {
            file,
            path,
            num_pages: 1,
            free_list: Vec::new(),
        })
    }

    /// Ruta del fichero subyacente (para diagnóstico / CLI).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// IDs de páginas actualmente en la free list (para inspección / tests).
    pub fn free_list(&self) -> &[PageId] {
        &self.free_list
    }

    /// Extiende el fichero en exactamente `extra` páginas (rellenas con ceros).
    fn extend_by(&mut self, extra: u32) -> Result<(), PagerError> {
        if extra == 0 {
            return Ok(());
        }
        let zeros = vec![0u8; PAGE_SIZE * extra as usize];
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&zeros)?;
        self.num_pages = self
            .num_pages
            .checked_add(extra)
            .ok_or(PagerError::NoFreePageId)?;
        Ok(())
    }

    /// Calcula el offset en bytes de la página `id`.
    fn offset_of(id: PageId) -> u64 {
        id as u64 * PAGE_SIZE as u64
    }
}

impl Pager for FilePager {
    fn allocate(&mut self) -> Result<PageId, PagerError> {
        // Reutilizar free list primero (LIFO).
        if let Some(id) = self.free_list.pop() {
            return Ok(id);
        }
        // Si no hay IDs libres, extender el fichero.
        let id = self.num_pages;
        self.extend_by(1)?;
        Ok(id)
    }

    fn read(&mut self, id: PageId, page: &mut [u8]) -> Result<(), PagerError> {
        if page.len() != PAGE_SIZE {
            return Err(PagerError::BadBufferSize {
                expected: PAGE_SIZE,
                got: page.len(),
            });
        }
        if id >= self.num_pages {
            return Err(PagerError::OutOfRange {
                requested: id,
                num_pages: self.num_pages,
            });
        }
        if self.free_list.contains(&id) {
            return Err(PagerError::FreePage(id));
        }
        self.file.seek(SeekFrom::Start(Self::offset_of(id)))?;
        self.file.read_exact(page)?;
        Ok(())
    }

    fn write(&mut self, id: PageId, page: &[u8]) -> Result<(), PagerError> {
        if page.len() != PAGE_SIZE {
            return Err(PagerError::BadBufferSize {
                expected: PAGE_SIZE,
                got: page.len(),
            });
        }
        if id >= self.num_pages {
            return Err(PagerError::OutOfRange {
                requested: id,
                num_pages: self.num_pages,
            });
        }
        if self.free_list.contains(&id) {
            return Err(PagerError::FreePage(id));
        }
        self.file.seek(SeekFrom::Start(Self::offset_of(id)))?;
        self.file.write_all(page)?;
        Ok(())
    }

    fn sync(&mut self) -> Result<(), PagerError> {
        self.file.sync_all()?;
        Ok(())
    }

    fn num_pages(&self) -> u32 {
        self.num_pages
    }

    fn free(&mut self, id: PageId) -> Result<(), PagerError> {
        if id >= self.num_pages {
            return Err(PagerError::OutOfRange {
                requested: id,
                num_pages: self.num_pages,
            });
        }
        if self.free_list.contains(&id) {
            return Err(PagerError::FreePage(id));
        }
        self.free_list.push(id);
        Ok(())
    }

    fn is_allocated(&self, id: PageId) -> bool {
        id < self.num_pages && !self.free_list.contains(&id)
    }
}

#[cfg(test)]
mod tests_pager {
    use super::*;
    use crate::cap11_slotted_pages::{MetaPage, PageHeader, PageType};
    use std::error::Error;

    /// Crea un pager en un directorio temporal, devuelve (Pager, TempDir).
    fn temp_pager() -> (FilePager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.liradb");
        let pager = FilePager::create(&path).unwrap();
        (pager, dir)
    }

    /// Buffer de PAGE_SIZE lleno de un patrón determinista (basado en page_id).
    fn pattern_page(id: PageId) -> [u8; PAGE_SIZE] {
        let mut buf = [0u8; PAGE_SIZE];
        // Header: page_id little-endian en los primeros 4 bytes.
        buf[..4].copy_from_slice(&id.to_le_bytes());
        // Cuerpo: (i + id) % 256
        for (i, b) in buf.iter_mut().enumerate().skip(PageHeader::SIZE) {
            *b = ((i as u32 + id) % 256) as u8;
        }
        buf
    }

    #[test]
    fn pager_error_display_y_source() {
        let io_err = PagerError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "eof",
        ));
        let s = format!("{io_err}");
        assert!(s.contains("pager io error"));
        // .source() sólo para variantes que envuelven errores.
        assert!(io_err.source().is_some());

        let range_err = PagerError::OutOfRange {
            requested: 5,
            num_pages: 3,
        };
        let s = format!("{range_err}");
        assert!(s.contains("page 5 out of range"));
        assert!(s.contains("num_pages=3"));
        assert!(range_err.source().is_none());

        let free_err = PagerError::FreePage(7);
        let s = format!("{free_err}");
        assert!(s.contains("page 7 is in free list"));

        let size_err = PagerError::BadBufferSize {
            expected: PAGE_SIZE,
            got: 10,
        };
        let s = format!("{size_err}");
        assert!(s.contains("buffer size 10"));
        assert!(s.contains("expected 4096"));

        let no_id_err = PagerError::NoFreePageId;
        let s = format!("{no_id_err}");
        assert!(s.contains("no free PageId"));
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::other("boom");
        let pe: PagerError = io.into();
        assert!(matches!(pe, PagerError::Io(_)));
    }

    #[test]
    fn create_y_open_roundtrip() {
        let (_pager, dir) = temp_pager();
        let path = dir.path().join("test.liradb");
        // El pager ya se creó; ahora reabrimos y verificamos num_pages == 1.
        drop(_pager);
        let p2 = FilePager::open(&path).unwrap();
        assert_eq!(p2.num_pages(), 1);
        assert_eq!(p2.page_size(), PAGE_SIZE);
    }

    #[test]
    fn open_archivo_no_existente_falla() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.liradb");
        let r = FilePager::open(&path);
        assert!(matches!(r, Err(PagerError::Io(_))));
    }

    #[test]
    fn open_archivo_con_tamanho_invalido_falla() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.liradb");
        // Escribir PAGE_SIZE-1 bytes: no es múltiplo.
        std::fs::write(&path, vec![0u8; PAGE_SIZE - 1]).unwrap();
        let r = FilePager::open(&path);
        assert!(matches!(r, Err(PagerError::Io(_))));
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("not a multiple of PAGE_SIZE"));
    }

    #[test]
    fn allocate_extiende_fichero() {
        let (mut pager, _dir) = temp_pager();
        assert_eq!(pager.num_pages(), 1); // sólo metapágina
        let p1 = pager.allocate().unwrap();
        let p2 = pager.allocate().unwrap();
        let p3 = pager.allocate().unwrap();
        assert_eq!((p1, p2, p3), (1, 2, 3));
        assert_eq!(pager.num_pages(), 4);
        assert!(pager.is_allocated(0));
        assert!(pager.is_allocated(1));
        assert!(pager.is_allocated(3));
    }

    #[test]
    fn read_write_roundtrip() {
        let (mut pager, _dir) = temp_pager();
        let id = pager.allocate().unwrap();
        let buf = pattern_page(id);
        pager.write(id, &buf).unwrap();
        let mut out = [0u8; PAGE_SIZE];
        pager.read(id, &mut out).unwrap();
        assert_eq!(buf, out);
    }

    #[test]
    fn read_buffer_mal_tamano_falla() {
        let (mut pager, _dir) = temp_pager();
        let id = pager.allocate().unwrap();
        let mut small = [0u8; 10];
        let r = pager.read(id, &mut small);
        assert!(matches!(
            r,
            Err(PagerError::BadBufferSize {
                expected: PAGE_SIZE,
                got: 10
            })
        ));
    }

    #[test]
    fn write_buffer_mal_tamano_falla() {
        let (mut pager, _dir) = temp_pager();
        let id = pager.allocate().unwrap();
        let r = pager.write(id, b"too short");
        assert!(matches!(
            r,
            Err(PagerError::BadBufferSize {
                expected: PAGE_SIZE,
                got: 9
            })
        ));
    }

    #[test]
    fn read_pagina_inexistente_falla() {
        let (mut pager, _dir) = temp_pager();
        let mut buf = [0u8; PAGE_SIZE];
        let r = pager.read(99, &mut buf);
        assert!(matches!(
            r,
            Err(PagerError::OutOfRange {
                requested: 99,
                num_pages: 1
            })
        ));
    }

    #[test]
    fn free_y_reutilizacion() {
        let (mut pager, _dir) = temp_pager();
        let p1 = pager.allocate().unwrap(); // 1
        let p2 = pager.allocate().unwrap(); // 2
        let p3 = pager.allocate().unwrap(); // 3
        assert_eq!((p1, p2, p3), (1, 2, 3));

        // Liberar p2; el siguiente allocate debe devolver p2 (LIFO).
        pager.free(p2).unwrap();
        assert!(!pager.is_allocated(p2));
        assert_eq!(pager.free_list(), &[p2]);

        let p4 = pager.allocate().unwrap();
        assert_eq!(p4, p2, "LIFO: free list debe reutilizar p2");
        assert!(pager.free_list().is_empty());
        assert_eq!(pager.num_pages(), 4); // no creció el fichero
    }

    #[test]
    fn free_y_reutilizacion_multiple() {
        let (mut pager, _dir) = temp_pager();
        let ids: Vec<PageId> = (0..5).map(|_| pager.allocate().unwrap()).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);

        for &id in &ids[..3] {
            pager.free(id).unwrap();
        }
        // free_list almacena en orden de inserción (1, 2, 3); el orden
        // LIFO lo decide el `pop` (sale 3 primero).
        assert_eq!(pager.free_list(), &[1, 2, 3]);

        // Siguientes allocates consumen LIFO: último en entrar, primero en salir.
        assert_eq!(pager.allocate().unwrap(), 3);
        assert_eq!(pager.allocate().unwrap(), 2);
        assert_eq!(pager.allocate().unwrap(), 1);
        assert!(pager.free_list().is_empty());
    }

    #[test]
    fn free_sobre_id_no_asignado_o_fuera_de_rango() {
        let (mut pager, _dir) = temp_pager();
        // num_pages == 1 → id=5 fuera de rango.
        let r = pager.free(5);
        assert!(matches!(
            r,
            Err(PagerError::OutOfRange {
                requested: 5,
                num_pages: 1
            })
        ));

        // Liberar un id que ya está en la free list → error.
        pager.allocate().unwrap(); // id=1
        pager.free(1).unwrap();
        let r = pager.free(1);
        assert!(matches!(r, Err(PagerError::FreePage(1))));
    }

    #[test]
    fn read_en_pagina_libre_falla() {
        let (mut pager, _dir) = temp_pager();
        let id = pager.allocate().unwrap();
        pager.write(id, &pattern_page(id)).unwrap();
        pager.free(id).unwrap();
        let mut buf = [0u8; PAGE_SIZE];
        let r = pager.read(id, &mut buf);
        assert!(matches!(r, Err(PagerError::FreePage(_))));
    }

    #[test]
    fn write_en_pagina_libre_falla() {
        let (mut pager, _dir) = temp_pager();
        let id = pager.allocate().unwrap();
        pager.free(id).unwrap();
        let r = pager.write(id, &pattern_page(id));
        assert!(matches!(r, Err(PagerError::FreePage(_))));
    }

    #[test]
    fn sync_no_falla() {
        let (mut pager, _dir) = temp_pager();
        pager.sync().unwrap();
    }

    #[test]
    fn persistencia_reabrir_tras_sync() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("persist.liradb");
        // Escribir varias páginas, sync, cerrar, reabrir, releer.
        {
            let mut p = FilePager::create(&path).unwrap();
            for id_alloc in 0..4 {
                let id = p.allocate().unwrap();
                let buf = pattern_page(id);
                p.write(id, &buf).unwrap();
                assert_eq!(id, id_alloc + 1);
            }
            p.sync().unwrap();
        }
        let mut p2 = FilePager::open(&path).unwrap();
        assert_eq!(p2.num_pages(), 5);
        let mut buf = [0u8; PAGE_SIZE];
        for id in 1..=4u32 {
            p2.read(id, &mut buf).unwrap();
            assert_eq!(buf, pattern_page(id));
        }
    }

    #[test]
    fn datos_persistidos_entre_allocations() {
        let (mut pager, _dir) = temp_pager();
        let id_a = pager.allocate().unwrap();
        pager.write(id_a, &pattern_page(id_a)).unwrap();
        pager.sync().unwrap();

        // Más allocations no corrompen datos previos.
        for _ in 0..10 {
            let _ = pager.allocate().unwrap();
        }
        let mut buf = [0u8; PAGE_SIZE];
        pager.read(id_a, &mut buf).unwrap();
        assert_eq!(buf, pattern_page(id_a));
    }

    #[test]
    fn metapagina_inicial_vacia() {
        let (mut pager, _dir) = temp_pager();
        // La página 0 existe tras create() y está "asignada" pero vacía.
        assert!(pager.is_allocated(0));
        let mut buf = [0u8; PAGE_SIZE];
        pager.read(0, &mut buf).unwrap();
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn escribir_metapagina_y_reabrir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meta.liradb");

        // Construir una MetaPage, codificarla, escribirla como página 0.
        let m = MetaPage {
            header: PageHeader::new(0, PageType::Meta),
            num_pages: 42,
            free_pages: 5,
            root_page: 7,
        };
        {
            let mut p = FilePager::create(&path).unwrap();
            p.write(0, &m.encode()).unwrap();
            p.sync().unwrap();
        }

        // Reabrir y decodificar la página 0 como MetaPage.
        let mut p2 = FilePager::open(&path).unwrap();
        let mut buf = [0u8; PAGE_SIZE];
        p2.read(0, &mut buf).unwrap();
        let m2 = MetaPage::decode(&buf).unwrap();
        assert_eq!(m2.num_pages, 42);
        assert_eq!(m2.free_pages, 5);
        assert_eq!(m2.root_page, 7);
    }

    #[test]
    fn free_list_no_persiste_tras_reopen() {
        // Decisión pedagógica: la free list es en memoria. Un reopen la
        // pierde (lo cual se corregirá en cap 14 cuando se persista en la
        // metapágina). Aquí documentamos el comportamiento actual.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fl.liradb");
        {
            let mut p = FilePager::create(&path).unwrap();
            let a = p.allocate().unwrap();
            let b = p.allocate().unwrap();
            assert_eq!((a, b), (1, 2));
            p.free(a).unwrap();
            assert_eq!(p.free_list(), &[1]);
            p.sync().unwrap();
        }
        let p2 = FilePager::open(&path).unwrap();
        assert!(p2.free_list().is_empty(), "free list es en memoria");
        assert_eq!(p2.num_pages(), 3);
        // La página 1 sigue en disco (no se truncó), pero ahora aparece
        // como "asignada" hasta que se implemente persistencia de free list.
        assert!(p2.is_allocated(1));
    }

    #[test]
    fn is_allocated_resume_estado() {
        let (mut pager, _dir) = temp_pager();
        assert!(pager.is_allocated(0));
        let a = pager.allocate().unwrap();
        assert!(pager.is_allocated(a));
        pager.free(a).unwrap();
        assert!(!pager.is_allocated(a));
        assert!(!pager.is_allocated(999)); // fuera de rango → false
    }
}
