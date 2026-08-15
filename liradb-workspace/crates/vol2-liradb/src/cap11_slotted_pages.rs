use crate::cap09_encoding::{decode_u32_le, encode_u32_le};

// ─────────────────── Cap 11: páginas, bloques y slotted pages ───────────────────

/// Tamaño fijo de página en bytes (4 KB). Constante del formato.
pub const PAGE_SIZE: usize = 4096;

/// Header de página (presente en todas las páginas de datos).
///
/// Layout en bytes (little-endian):
///   [magic: u8] [page_type: u8] [page_id: u32] [num_records: u16] [free_space: u16]
///
/// Total: 10 bytes. El magic es 0xDA para distinguir páginas de datos de
/// la metapágina (0xFE).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageHeader {
    pub page_id: u32,
    pub page_type: PageType,
    pub num_records: u16,
    pub free_space: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageType {
    Data = 0xDA,
    Meta = 0xFE,
}

impl PageHeader {
    pub const SIZE: usize = 10;

    pub fn new(page_id: u32, page_type: PageType) -> Self {
        Self {
            page_id,
            page_type,
            num_records: 0,
            free_space: (PAGE_SIZE - Self::SIZE) as u16,
        }
    }

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        out[0] = self.page_type as u8;
        out[1] = self.page_type as u8; // magic redundante para autochequeo
        out[2..6].copy_from_slice(&encode_u32_le(self.page_id));
        out[6..8].copy_from_slice(&self.num_records.to_le_bytes());
        out[8..10].copy_from_slice(&self.free_space.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8; Self::SIZE]) -> Result<Self, String> {
        let magic = bytes[0];
        let page_type = match bytes[1] {
            0xDA => PageType::Data,
            0xFE => PageType::Meta,
            other => return Err(format!("page: unknown magic {other:#x}")),
        };
        if magic != page_type as u8 {
            return Err("page: magic mismatch".into());
        }
        let page_id = decode_u32_le(bytes[2..6].try_into().unwrap());
        let num_records = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let free_space = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        Ok(Self {
            page_id,
            page_type,
            num_records,
            free_space,
        })
    }
}

/// Slotted page: header + records con length prefix.
///
/// Layout (PAGE_SIZE bytes):
///   [PageHeader: 10 bytes]
///   [para cada record: u32 LE length | payload | ...]
///   [padding hasta llenar la página]
#[derive(Debug, Clone, PartialEq)]
pub struct SlottedPage {
    pub header: PageHeader,
    /// Records como bytes crudos.
    pub(crate) records: Vec<Vec<u8>>,
}

impl SlottedPage {
    pub fn new(page_id: u32, page_type: PageType) -> Self {
        Self {
            header: PageHeader::new(page_id, page_type),
            records: Vec::new(),
        }
    }

    /// Devuelve el espacio libre disponible (en bytes) para nuevos records.
    pub fn free_space(&self) -> usize {
        PAGE_SIZE - PageHeader::SIZE - self.records.iter().map(|r| r.len()).sum::<usize>()
    }

    /// Añade un record si hay espacio. Devuelve el offset donde se insertó,
    /// o `None` si no cabe.
    pub fn insert(&mut self, record: &[u8]) -> Option<usize> {
        if record.len() > self.free_space() {
            return None;
        }
        let offset = PageHeader::SIZE + self.records.iter().map(|r| r.len()).sum::<usize>();
        self.records.push(record.to_vec());
        self.header.num_records += 1;
        self.header.free_space = self.free_space() as u16;
        Some(offset)
    }

    /// Codifica la página completa a bytes (PAGE_SIZE) con length-prefix.
    pub fn encode(&self) -> [u8; PAGE_SIZE] {
        let mut out = [0u8; PAGE_SIZE];
        out[..PageHeader::SIZE].copy_from_slice(&self.header.encode());
        let mut pos = PageHeader::SIZE;
        for rec in &self.records {
            if pos + 4 + rec.len() > PAGE_SIZE {
                break;
            }
            let len_bytes = encode_u32_le(rec.len() as u32);
            out[pos..pos + 4].copy_from_slice(&len_bytes);
            out[pos + 4..pos + 4 + rec.len()].copy_from_slice(rec);
            pos += 4 + rec.len();
        }
        out
    }

    /// Decodifica con length-prefix.
    pub fn decode(bytes: &[u8; PAGE_SIZE]) -> Result<Self, String> {
        let header = PageHeader::decode(bytes[..PageHeader::SIZE].try_into().unwrap())?;
        let mut records = Vec::new();
        let mut pos = PageHeader::SIZE;
        for _ in 0..header.num_records {
            if pos + 4 > PAGE_SIZE {
                return Err("page: ran out of space (header corruption?)".into());
            }
            let len_bytes: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap();
            let len = decode_u32_le(&len_bytes) as usize;
            pos += 4;
            if pos + len > PAGE_SIZE {
                return Err("page: record truncated".into());
            }
            records.push(bytes[pos..pos + len].to_vec());
            pos += len;
        }
        Ok(Self { header, records })
    }

    /// Devuelve los records almacenados.
    pub fn records(&self) -> &[Vec<u8>] {
        &self.records
    }
}

/// Metapágina (página 0): contiene el catálogo del archivo.
///
/// Layout:
///   [PageHeader]
///   [num_pages: u32] [free_pages: u32] [root_page: u32]
///
/// Total header info: 12 bytes. El resto de la página está libre para
/// extensiones futuras (versiones, checksums, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct MetaPage {
    pub header: PageHeader,
    pub num_pages: u32,
    pub free_pages: u32,
    pub root_page: u32,
}

impl MetaPage {
    pub const INFO_OFFSET: usize = PageHeader::SIZE;
    pub const INFO_SIZE: usize = 12;

    pub fn new() -> Self {
        Self {
            header: PageHeader::new(0, PageType::Meta),
            num_pages: 1, // sólo la metapágina por ahora
            free_pages: 0,
            root_page: 0,
        }
    }

    pub fn encode(&self) -> [u8; PAGE_SIZE] {
        let mut out = [0u8; PAGE_SIZE];
        out[..PageHeader::SIZE].copy_from_slice(&self.header.encode());
        let info = encode_u32_le(self.num_pages);
        let free = encode_u32_le(self.free_pages);
        let root = encode_u32_le(self.root_page);
        out[Self::INFO_OFFSET..Self::INFO_OFFSET + 4].copy_from_slice(&info);
        out[Self::INFO_OFFSET + 4..Self::INFO_OFFSET + 8].copy_from_slice(&free);
        out[Self::INFO_OFFSET + 8..Self::INFO_OFFSET + 12].copy_from_slice(&root);
        out
    }

    pub fn decode(bytes: &[u8; PAGE_SIZE]) -> Result<Self, String> {
        let header = PageHeader::decode(bytes[..PageHeader::SIZE].try_into().unwrap())?;
        if header.page_type != PageType::Meta {
            return Err("meta: page type mismatch".into());
        }
        let num_pages = decode_u32_le(
            bytes[Self::INFO_OFFSET..Self::INFO_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let free_pages = decode_u32_le(
            bytes[Self::INFO_OFFSET + 4..Self::INFO_OFFSET + 8]
                .try_into()
                .unwrap(),
        );
        let root_page = decode_u32_le(
            bytes[Self::INFO_OFFSET + 8..Self::INFO_OFFSET + 12]
                .try_into()
                .unwrap(),
        );
        Ok(Self {
            header,
            num_pages,
            free_pages,
            root_page,
        })
    }
}

impl Default for MetaPage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests_page {
    use super::*;

    #[test]
    fn page_header_roundtrip() {
        let h = PageHeader::new(42, PageType::Data);
        let enc = h.encode();
        let dec = PageHeader::decode(&enc).unwrap();
        assert_eq!(h, dec);
    }

    #[test]
    fn page_header_meta() {
        let h = PageHeader::new(0, PageType::Meta);
        let enc = h.encode();
        assert_eq!(enc[1], 0xFE);
        let dec = PageHeader::decode(&enc).unwrap();
        assert_eq!(h, dec);
    }

    #[test]
    fn page_header_magic_mismatch() {
        let mut bytes = [0u8; 10];
        bytes[0] = 0xAA;
        bytes[1] = 0xDA;
        assert!(PageHeader::decode(&bytes).is_err());
    }

    #[test]
    fn slotted_page_vacio() {
        let p = SlottedPage::new(0, PageType::Data);
        let enc = p.encode();
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(p, dec);
        assert_eq!(dec.records().len(), 0);
        assert_eq!(dec.header.num_records, 0);
    }

    #[test]
    fn slotted_page_con_records() {
        let mut p = SlottedPage::new(7, PageType::Data);
        let r1: &[u8] = b"hello";
        let r2: &[u8] = b"world!";
        let r3: &[u8] = b"LiraDB";
        assert!(p.insert(r1).is_some());
        assert!(p.insert(r2).is_some());
        assert!(p.insert(r3).is_some());
        assert_eq!(p.records().len(), 3);

        let enc = p.encode();
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(p, dec);
        assert_eq!(dec.records()[0], r1);
        assert_eq!(dec.records()[1], r2);
        assert_eq!(dec.records()[2], r3);
    }

    #[test]
    fn slotted_page_record_no_cabe() {
        let mut p = SlottedPage::new(0, PageType::Data);
        let huge = vec![0u8; PAGE_SIZE - PageHeader::SIZE];
        assert!(p.insert(&huge).is_some());
        assert!(p.insert(b"extra").is_none());
    }

    #[test]
    fn slotted_page_meta() {
        let p = SlottedPage::new(0, PageType::Meta);
        let enc = p.encode();
        assert_eq!(enc[1], 0xFE);
        let dec = SlottedPage::decode(&enc).unwrap();
        assert_eq!(dec.header.page_type, PageType::Meta);
    }

    #[test]
    fn free_space_decrementa() {
        let mut p = SlottedPage::new(0, PageType::Data);
        let initial_free = p.free_space();
        p.insert(b"hello").unwrap();
        assert!(p.free_space() < initial_free);
        assert_eq!(initial_free - p.free_space(), 5);
    }

    #[test]
    fn meta_page_roundtrip() {
        let m = MetaPage {
            header: PageHeader::new(0, PageType::Meta),
            num_pages: 42,
            free_pages: 5,
            root_page: 7,
        };
        let enc = m.encode();
        let dec = MetaPage::decode(&enc).unwrap();
        assert_eq!(m, dec);
    }

    #[test]
    fn meta_page_default() {
        let m = MetaPage::new();
        assert_eq!(m.num_pages, 1);
        assert_eq!(m.free_pages, 0);
        let enc = m.encode();
        let dec = MetaPage::decode(&enc).unwrap();
        assert_eq!(m, dec);
    }
}
