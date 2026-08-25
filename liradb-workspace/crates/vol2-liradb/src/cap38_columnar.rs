//! Vol.II — Cap.38: Almacenamiento columnar y ejecución vectorizada.
//!
//! Segundo capítulo de la Parte VIII (crecimiento POST-mapa): recoge el gancho
//! del cap. 34 («el ×16 de CSR fue la primera MEDIDA de una ley general — el
//! coste está en CÓMO están puestos los bytes») y lo aplica a las
//! PROPIEDADES. El modelo mental único del capítulo es **layout ×
//! granularidad**: el cap. 14 organizó adyacencias por nodo (CSR); aquí se
//! organizan propiedades POR ATRIBUTO (columnas tipadas) y se PROCESAN en
//! trozos grandes ([`TAMANIO_VECTOR`] = 1024) en lugar de valor a valor.
//!
//! Qué entrega este módulo (contrato §2):
//!
//! 1. **Extracción row→column** ([`TablaColumnar::desde_store`]): capa de
//!    LECTURA analítica pura sobre [`MemoryStore`] — el store no se toca
//!    (hexagonal, como DiskStore en caps. 33-34). Los ids se recolectan y se
//!    ORDENAN ASCENDENTE: `iter_nodes` itera contenedores hash/vector con
//!    orden NO estable entre runs, y sin orden fijo no hay ni tests ni
//!    baselines comparables (Mytkowicz et al., ASPLOS 2009). Presencia sparse
//!    con el [`BitSet`] del cap. 26; cada columna es MONOTIPA.
//! 2. **Dictionary encoding** ([`Diccionario`]): String→u32 con estadística
//!    integrada y VEREDICTO BIDIRECTIONAL medido: GANA en baja cardinalidad
//!    (`ciudad`, ≤ 32 dominios) y PIERDE en alta (`email`, única por nodo).
//!    La pérdida es material didáctico, no un fallo (Abadi, Madden, Ferreira,
//!    «Integrating Compression and Execution in Column-Store Database
//!    Systems», SIGMOD 2006).
//! 3. **Bit packing a mano** ([`empaquetar`]/[`desempaquetar`]): los códigos
//!    del diccionario caben en ⌈log₂(cardinalidad)⌉ bits — ciudad: u32 → 5
//!    bits. Tests EXACTOS contra patrones binarios conocidos + roundtrip
//!    exhaustivo k=1..=32.
//! 4. **Ejecución por lotes** ([`filtrar_lote_i64`]): filtro en DOS PASADAS
//!    (máscara apretada sin ramas internas → compactación), resto del lote
//!    manejado FUERA del bucle caliente. Equivalencia con el filtro
//!    fila-a-fila exigida POR TEST ([`filtrar_fila_i64`]).
//! 5. **Factorización mínima viable 2-hop**
//!    ([`ExpansionFactorizada`]): arrays por variable + multiplicidad por
//!    pivote frente a tuplas planas, con conteo exacto de FILAS LÓGICAS vs
//!    CELDAS FÍSICAS. Conexión conceptual: el processor factorizado de Kùzu
//!    (Jin, Feng, Chen, Liu, Salihoğlu, CIDR 2023, CC-BY 4.0) y la teoría de
//!    representaciones factorizadas (Olteanu-Závodný, ICDT 2015).
//! 6. **Informe imprimible** ([`informe_columnar`]): ratios de compresión y
//!    conteos de factorización sobre el dataset de referencia, reproducibles
//!    para que la prosa pegue cifras REALES. Los TIEMPOS no viven aquí: son
//!    de criterion (`benches/bench_columnar.rs`).
//!
//! Delimitación SIMD HONESTA (espejo de la de cargo-fuzz en el cap. 33):
//! SIN nightly NO hay `std::simd` ni intrinsics (toolchain pinneada 1.96.0).
//! Lo que se promete es el EFECTO MEDIDO — delta criterion escalar-vs-lote —
//! y el MÉTODO opcional de verificación por ensamblador (`cargo asm` o
//! `rustc --emit=asm`, FUERA del pipeline de verificación). Lo que NO se
//! promete: instrucciones concretas, compress-store vectorial ni garantía de
//! vectorización. Si LLVM no vectoriza, el delta lo dirá — jamás se infla.
//!
//! Frontera declarada: esto es una capa de LECTURA; el Executor (Volcano,
//! cap. 20) sigue fila-a-fila, no hay paginación columnar en disco ni RLE /
//! Frame-of-Reference, y el motor factorizado completo y WCOJ son del
//! CAP. 39. Hallazgo del §41 respetado: las columnas viven EN MEMORIA, sin
//! serialización JSONL (donde `Float(2.0)` degeneraría en `"2"`).

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::cap07_modelo::{NodeId, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap26_proyeccion::BitSet;

// ─────────────────── Cap 38: Almacenamiento columnar ───────────────────

/// Tamaño del lote de ejecución vectorial: 1024 valores.
///
/// Un lote de 1024×8 B = 8 KiB cabe holgado en L1/L2 — el criterio que X100
/// opuso a MonetDB (materializar columnas ENTERAS acababa limitado por ancho
/// de banda de memoria; Boncz, Zukowski, Nes, CIDR 2005). La magnitud es
/// propia, no copiada: X100 usó vectores de ~100. Precedente interno:
/// `ProyeccionPonderada::bloques_de_nodos(tam)` (cap. 26).
pub const TAMANIO_VECTOR: usize = 1024;

/// Tipo físico de una columna: la etiqueta que el bucle caliente ya no
/// necesita comprobar celda a celda.
///
/// Es la mitad del argumento columnar: en el row store cada lectura pasa por
/// el tag del `Value` (cap. 9) EN TIEMPO DE EJECUCIÓN; aquí el dispatch se
/// paga UNA vez al extraer y el bucle ve `i64` pelados.
///
/// Nota de nombre: `TipoColumna` ya existe en el cap. 32 (sufijos CSV del
/// import/export); este es el gemelo analítico con dominio propio, y el
/// re-export plano del crate exige que no colisionen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDatoColumna {
    /// Enteros 64 bits.
    Int,
    /// Flotantes IEEE 754 de 64 bits.
    Float,
    /// Cadenas UTF-8.
    String,
    /// Booleanos (1 byte por celda, no 1 bit: la densidad de acceso manda).
    Bool,
}

impl TipoDatoColumna {
    /// Nombre legible para informes.
    pub fn nombre(self) -> &'static str {
        match self {
            TipoDatoColumna::Int => "Int",
            TipoDatoColumna::Float => "Float",
            TipoDatoColumna::String => "String",
            TipoDatoColumna::Bool => "Bool",
        }
    }
}

impl fmt::Display for TipoDatoColumna {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nombre())
    }
}

/// Los datos de UNA propiedad, apilados por atributo en vez de por nodo.
///
/// Una variante por tipo físico: dentro de la columna no hay tags que
/// despachar — eso era exactamente el coste oculto del row store para
/// cargas analíticas (el bucle pagaba `match` por celda).
#[derive(Debug, Clone)]
pub enum Columna {
    /// Enteros.
    Int(Vec<i64>),
    /// Flotantes.
    Float(Vec<f64>),
    /// Cadenas.
    String(Vec<String>),
    /// Booleanos.
    Bool(Vec<bool>),
}

impl Columna {
    /// Tipo físico de esta columna.
    pub fn tipo(&self) -> TipoDatoColumna {
        match self {
            Columna::Int(_) => TipoDatoColumna::Int,
            Columna::Float(_) => TipoDatoColumna::Float,
            Columna::String(_) => TipoDatoColumna::String,
            Columna::Bool(_) => TipoDatoColumna::Bool,
        }
    }

    /// Número de celdas (presentes + huecos de ausentes).
    pub fn len(&self) -> usize {
        match self {
            Columna::Int(v) => v.len(),
            Columna::Float(v) => v.len(),
            Columna::String(v) => v.len(),
            Columna::Bool(v) => v.len(),
        }
    }

    /// ¿Columna vacía?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Una columna con su VERDAD de presencia separada de sus bytes.
///
/// Decisión de diseño (contrato §5.4): la celda AUSENTE ocupa su hueco con
/// un valor neutro (0, 0.0, "", false) y la VERDAD vive en el [`BitSet`] del
/// cap. 26. Alternativas descartadas: `Option<T>` por celda (rompe densidad
/// y auto-vectorización del bucle caliente) o centinela NULL dentro de la
/// columna (reintroduce el dispatch por elemento que este capítulo elimina).
/// El BitSet denso es exactamente su caso: índices 0..n contiguos.
///
/// Los índices del BitSet son POSICIONES en la columna (0..num_filas), no
/// ids de nodo: tras un `delete_node` el espacio de ids tiene huecos y la
/// columna solo guarda filas vivas.
#[derive(Debug, Clone)]
pub struct ColumnaTipada {
    valores: Columna,
    presencia: BitSet,
    /// Valores descartados por tener otro tag distinto al de la columna.
    descartes_por_tipo: usize,
}

/// El tipo físico de un valor, si tiene columna analítica.
///
/// `Null` y `Bytes` no la tienen: el primero ES ausencia semántica (la
/// cubre el BitSet) y los segundos quedan fuera del alcance analítico del
/// capítulo (frontera declarada en la doc del módulo).
fn tipo_de(valor: &Value) -> Option<TipoDatoColumna> {
    match valor {
        Value::Int(_) => Some(TipoDatoColumna::Int),
        Value::Float(_) => Some(TipoDatoColumna::Float),
        Value::String(_) => Some(TipoDatoColumna::String),
        Value::Bool(_) => Some(TipoDatoColumna::Bool),
        Value::Null | Value::Bytes(_) => None,
    }
}

impl ColumnaTipada {
    /// Datos crudos de la columna.
    pub fn valores(&self) -> &Columna {
        &self.valores
    }

    /// Presencia sparse (índices = posiciones de fila).
    pub fn presencia(&self) -> &BitSet {
        &self.presencia
    }

    /// Celdas presentes (popcount del BitSet).
    pub fn presentes(&self) -> u64 {
        self.presencia.unos()
    }

    /// Densidad = presentes / celdas totales (1.0 si no hay filas).
    ///
    /// En el dataset de referencia ronda el 80%: el esquema ES sparse y esa
    /// sparsidad es la que justifica llevar la presencia APARTE.
    pub fn densidad(&self) -> f64 {
        let total = self.valores.len();
        if total == 0 {
            return 1.0;
        }
        self.presencia.unos() as f64 / total as f64
    }

    /// Valores que se encontraron con OTRO tipo y fueron descartados.
    ///
    /// Política «una columna, un tipo»: el tipo lo fija el PRIMER valor
    /// presente en orden de ids ascendente (determinista) y todo valor de
    /// otro tag se CUENTA y se descarta — nunca se convierte en silencio.
    /// En el dataset de referencia no ocurre ninguna mezcla (cada clave del
    /// esquema es homogénea); el contador existe para que la política sea
    /// observable, no invisible.
    pub fn descartes_por_tipo(&self) -> usize {
        self.descartes_por_tipo
    }

    /// Las cadenas presentes, en orden de ids ascendente.
    ///
    /// Devuelve `None` si la columna no es de cadenas. Es el puente hacia
    /// [`Diccionario`]: el diccionario codifica SOLO lo presente, porque un
    /// hueco no es una cadena más sino AUSENCIA (y meter "" como código
    /// inflaría la cardinalidad con un fantasma).
    pub fn cadenas_presentes(&self) -> Option<Vec<String>> {
        let Columna::String(vs) = &self.valores else {
            return None;
        };
        Some(
            (0..vs.len())
                .filter(|&i| self.presencia.contiene(i))
                .map(|i| vs[i].clone())
                .collect(),
        )
    }
}

/// Errores de las operaciones analíticas sobre [`TablaColumnar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorColumnar {
    /// La clave pedida no generó columna (no existe en ninguna fila).
    ClaveAusente(String),
    /// La clave existe pero su columna no es del tipo pedido.
    ClaveNoEntera(String),
}

impl fmt::Display for ErrorColumnar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorColumnar::ClaveAusente(clave) => write!(f, "clave '{clave}' sin columna"),
            ErrorColumnar::ClaveNoEntera(clave) => {
                write!(f, "la columna '{clave}' no es de enteros")
            }
        }
    }
}

impl std::error::Error for ErrorColumnar {}

/// El bloque columnar completo: ids ordenados + una columna por clave.
///
/// Construcción: [`TablaColumnar::desde_store`]. Es un SNAPSHOT de lectura
/// (inmutable una vez construido): el store puede seguir mutando (OLTP)
/// mientras la analítica corre sobre su copia coherente — la misma
/// separación que la proyección del cap. 26 encarnó para la topología.
#[derive(Debug, Clone)]
pub struct TablaColumnar {
    /// Ids de las filas, ASCENDENTES. La posición `i` de toda columna
    /// corresponde a `ids[i]` — el contrato de alineación de TODO el módulo.
    ids: Vec<NodeId>,
    /// BTreeMap, no HashMap: el ORDEN de claves del informe y de los tests
    /// debe ser determinista (misma ley que los ids).
    columnas: BTreeMap<String, ColumnaTipada>,
}

impl TablaColumnar {
    /// Extrae una columna por cada clave pedida desde los props del store.
    ///
    /// Determinismo (riesgo anticipado §5b del contrato): `iter_nodes` NO es
    /// estable entre runs, así que primero se recolectan TODOS los ids y se
    /// ordenan ascendente; después cada clave se rellena recorriendo ESE
    /// orden. Dos ejecuciones sobre el mismo store producen byte a byte la
    /// misma tabla.
    ///
    /// Coste: O(filas · claves) lecturas de `HashMap::get` — se paga UNA vez
    /// y luego el bucle analítico nunca vuelve a tocar el row store.
    pub fn desde_store(store: &MemoryStore, claves: &[&str]) -> Self {
        // 1) ids vivos, ordenados: la base determinista de todo lo demás.
        let mut ids: Vec<NodeId> = store.iter_nodes().map(|nodo| nodo.id).collect();
        ids.sort_unstable();

        // 2) Dos pasadas por clave. PRIMERA: fijar el tipo con el primer
        //    valor tipado presente (orden ascendente ⇒ determinista).
        //    SEGUNDA: rellenar SOLO el buffer de ese tipo — un push por fila
        //    garantiza la alineación posición↔fila aunque haya descartes.
        let mut columnas: BTreeMap<String, ColumnaTipada> = BTreeMap::new();
        for clave in claves {
            let tipo = ids.iter().find_map(|&id| {
                store
                    .get_node(id)
                    .and_then(|nodo| nodo.props.get(*clave))
                    .and_then(tipo_de)
            });

            let Some(tipo) = tipo else {
                // Ningún valor tipado en toda la tabla: no hay columna que
                // construir (una columna vacía sería ruido, no dato).
                continue;
            };

            let mut valores = match tipo {
                TipoDatoColumna::Int => Columna::Int(Vec::with_capacity(ids.len())),
                TipoDatoColumna::Float => Columna::Float(Vec::with_capacity(ids.len())),
                TipoDatoColumna::String => Columna::String(Vec::with_capacity(ids.len())),
                TipoDatoColumna::Bool => Columna::Bool(Vec::with_capacity(ids.len())),
            };
            let mut presencia = BitSet::new();
            let mut descartes = 0usize;

            for (posicion, &id) in ids.iter().enumerate() {
                let valor = store.get_node(id).and_then(|nodo| nodo.props.get(*clave));
                let visto = valor.and_then(tipo_de);
                match visto {
                    // Coincide: celda real, marcada en el BitSet.
                    Some(_) if visto == Some(tipo) => presencia.marcar(posicion),
                    // Otro tag CONCRETO (String donde la columna es Int…): se
                    // CUENTA y se descarta — nunca se convierte en silencio.
                    Some(_) => descartes += 1,
                    // Ausente, Null o Bytes: AUSENCIA (hueco neutro sin marca;
                    // Null es ausencia semántica y Bytes queda fuera del
                    // alcance analítico del capítulo — frontera declarada).
                    None => {}
                }
                // SIEMPRE un push por fila: el hueco lleva valor neutro y la
                // verdad sigue viva en el BitSet.
                match &mut valores {
                    Columna::Int(vs) => vs.push(match (visto, valor) {
                        (Some(TipoDatoColumna::Int), Some(Value::Int(v))) => *v,
                        _ => 0,
                    }),
                    Columna::Float(vs) => vs.push(match (visto, valor) {
                        (Some(TipoDatoColumna::Float), Some(Value::Float(v))) => *v,
                        _ => 0.0,
                    }),
                    Columna::String(vs) => vs.push(match (visto, valor) {
                        (Some(TipoDatoColumna::String), Some(Value::String(s))) => s.clone(),
                        _ => String::new(),
                    }),
                    Columna::Bool(vs) => vs.push(match (visto, valor) {
                        (Some(TipoDatoColumna::Bool), Some(Value::Bool(b))) => *b,
                        _ => false,
                    }),
                }
            }

            columnas.insert(
                (*clave).to_string(),
                ColumnaTipada {
                    valores,
                    presencia,
                    descartes_por_tipo: descartes,
                },
            );
        }

        TablaColumnar { ids, columnas }
    }

    /// Filas vivas (longitud de todas las columnas).
    pub fn num_filas(&self) -> usize {
        self.ids.len()
    }

    /// Ids en orden ascendente; `columna.posición i ↔ ids[i]`.
    pub fn ids(&self) -> &[NodeId] {
        &self.ids
    }

    /// Claves con columna, en orden alfabético (BTreeMap).
    pub fn claves(&self) -> impl Iterator<Item = &str> {
        self.columnas.keys().map(String::as_str)
    }

    /// La columna de una clave, si se extrajo.
    pub fn columna(&self, clave: &str) -> Option<&ColumnaTipada> {
        self.columnas.get(clave)
    }

    /// Filtro analítico por lotes sobre una columna entera.
    ///
    /// Devuelve los IDS cuya prop pasa el predicado, en orden ascendente —
    /// el mismo contrato que [`filtrar_fila_i64`], para que la equivalencia
    /// sea comparación directa de vectores. Dentro llama a
    /// [`filtrar_lote_i64`] (dos pasadas por lote); la presencia densa se
    /// materializa UNA vez por consulta (el BitSet canónico no se toca).
    pub fn filtrar_int(
        &self,
        clave: &str,
        predicado: impl Fn(i64) -> bool,
    ) -> Result<Vec<NodeId>, ErrorColumnar> {
        let col = self
            .columnas
            .get(clave)
            .ok_or_else(|| ErrorColumnar::ClaveAusente(clave.to_string()))?;
        let Columna::Int(valores) = &col.valores else {
            return Err(ErrorColumnar::ClaveNoEntera(clave.to_string()));
        };
        let admite: Vec<bool> = (0..valores.len())
            .map(|i| col.presencia.contiene(i))
            .collect();
        Ok(filtrar_lote_i64(valores, &admite, predicado)
            .into_iter()
            .map(|i| self.ids[i])
            .collect())
    }
}

// ─────────────────── Dictionary encoding ───────────────────

/// Estadística integrada del [`Diccionario`] (contrato §2: bytes antes/después).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstadisticaDiccionario {
    /// Valores distintos (tamaño de la tabla de códigos).
    pub cardinalidad: usize,
    /// Códigos emitidos (una entrada por valor presente de entrada).
    pub elementos: usize,
    /// Bytes si la columna siguiera plana: suma de `len()` de cada cadena.
    pub bytes_antes: u64,
    /// Bytes comprimidos: 4 B por código + la tabla de cadenas únicas.
    pub bytes_despues: u64,
}

impl EstadisticaDiccionario {
    /// Ratio antes/después: **> 1 GANA**, **< 1 PIERDE**. El veredicto es
    /// bidireccional por diseño (contrato §5.5): en `ciudad` (≤ 32 dominios)
    /// el ratio supera 1; en `email` (único por nodo) cae por debajo — y ese
    /// fracaso SE REPORTA, porque enseñar cuándo NO aplica es el objetivo.
    pub fn ratio(self) -> f64 {
        if self.bytes_despues == 0 {
            return 1.0;
        }
        self.bytes_antes as f64 / self.bytes_despues as f64
    }
}

/// Dictionary encoding: sustituir cadenas repetidas por enteros pequeños.
///
/// Tres piezas en una: la TABLA inversa (código → cadena), el mapa directo
/// (cadena → código) y los CÓDIGOS por fila. El beneficio no es solo
/// espacio: los códigos u32 son operables COMPRIMIDOS (filtros por igualdad
/// sin decodificar — SIGMOD 2006) y alimentan directamente el bit packing.
///
/// Orden de asignación: primera aparición en el orden de ENTRADA. Como la
/// entrada viene siempre de una columna con ids ascendentes, el resultado es
/// determinista por construcción.
#[derive(Debug, Clone)]
pub struct Diccionario {
    /// Código → cadena (la tabla inversa).
    tabla: Vec<String>,
    /// Cadena → código (el mapa directo; propio para no amarrar vidas útiles).
    indices: HashMap<String, u32>,
    /// Código por elemento, paralelo a la entrada.
    codigos: Vec<u32>,
}

impl Diccionario {
    /// Codifica exactamente el slice dado (con repeticiones).
    pub fn nuevo(valores: &[String]) -> Self {
        let mut tabla: Vec<String> = Vec::new();
        let mut indices: HashMap<String, u32> = HashMap::new();
        let mut codigos = Vec::with_capacity(valores.len());
        for valor in valores {
            let siguiente = tabla.len() as u32;
            let codigo = *indices.entry(valor.clone()).or_insert_with(|| {
                tabla.push(valor.clone());
                siguiente
            });
            codigos.push(codigo);
        }
        Diccionario {
            tabla,
            indices,
            codigos,
        }
    }

    /// Cardinalidad: cuántas cadenas distintas entraron.
    pub fn cardinalidad(&self) -> usize {
        self.tabla.len()
    }

    /// Los códigos (paralelos a la entrada).
    pub fn codigos(&self) -> &[u32] {
        &self.codigos
    }

    /// La cadena de un código (roundtrip exacto garantizado por test).
    pub fn cadena_de(&self, codigo: u32) -> Option<&str> {
        self.tabla.get(codigo as usize).map(String::as_str)
    }

    /// El código de una cadena ya vista (búsqueda directa O(1)).
    ///
    /// Es la mitad que hace RENTABLE operar comprimido: un filtro por
    /// igualdad traduce el literal UNA vez y compara u32 contra u32, sin
    /// decodificar la columna (Abadi-Madden-Ferreira, SIGMOD 2006).
    pub fn codigo_de(&self, valor: &str) -> Option<u32> {
        self.indices.get(valor).copied()
    }

    /// Roundtrip: reconstruye la entrada completa.
    pub fn decodificar(&self) -> Vec<&str> {
        self.codigos
            .iter()
            .map(|&c| self.tabla[c as usize].as_str())
            .collect()
    }

    /// Estadística de compresión (ver [`EstadisticaDiccionario`]).
    ///
    /// Convención medida: solo contenido UTF-8 (`len()`), sin cabeceras de
    /// `String` ni punteros — la MISMA convención en ambos lados del ratio,
    /// que es lo que hace la comparación honesta.
    pub fn estadisticas(&self) -> EstadisticaDiccionario {
        let bytes_antes: u64 = self
            .codigos
            .iter()
            .map(|&c| self.tabla[c as usize].len() as u64)
            .sum();
        let bytes_tabla: u64 = self.tabla.iter().map(|s| s.len() as u64).sum();
        EstadisticaDiccionario {
            cardinalidad: self.tabla.len(),
            elementos: self.codigos.len(),
            bytes_antes,
            bytes_despues: 4 * self.codigos.len() as u64 + bytes_tabla,
        }
    }
}

// ─────────────────── Bit packing ───────────────────

/// Bits mínimos para codificar `cardinalidad` valores distintos.
///
/// `bits_necesarios(1) == 0` (un solo valor no necesita información),
/// `bits_necesarios(32) == 5`, `bits_necesarios(33) == 6`.
pub fn bits_necesarios(cardinalidad: usize) -> u32 {
    if cardinalidad <= 1 {
        return 0;
    }
    let maximo_codigo = (cardinalidad - 1) as u32;
    32 - maximo_codigo.leading_zeros()
}

/// Empaqueta valores u32 en un flujo denso de bits dentro de palabras u64.
///
/// Layout (LSB-first): cada valor ocupa EXACTAMENTE `bits` bits consecutivos
/// del flujo; si no caben en la palabra actual, se parte entre dos. Es el
/// layout que se dibuja a mano en la prosa y el que hacen exactos los tests
/// contra patrones binarios conocidos.
///
/// Contratos: `bits <= 32` (panics con mensaje si no), valores deben caber
/// en `bits` (depuración lo verifica), `bits == 0` devuelve vacío (con
/// cardinalidad 1 no hay nada que almacenar — ver [`desempaquetar`]).
pub fn empaquetar(valores: &[u32], bits: u32) -> Vec<u64> {
    assert!(bits <= 32, "bit packing de hasta 32 bits, pedido {bits}");
    if valores.is_empty() || bits == 0 {
        return Vec::new();
    }
    let total_bits = valores.len() as u64 * bits as u64;
    let mut palabras = vec![0u64; total_bits.div_ceil(64) as usize];
    let mut posicion: u64 = 0;
    for (i, &valor) in valores.iter().enumerate() {
        debug_assert!(
            u64::from(valor) < (1u64 << bits),
            "valor[{i}]={valor} no cabe en {bits} bits"
        );
        let palabra_idx = (posicion / 64) as usize;
        let offset = (posicion % 64) as u32;
        palabras[palabra_idx] |= u64::from(valor) << offset;
        let usados = 64 - offset;
        if usados < bits {
            // El valor se PARTE: lo que sobró continúa al principio de la
            // palabra siguiente. `usados >= 1` porque offset <= 63.
            palabras[palabra_idx + 1] |= u64::from(valor) >> usados;
        }
        posicion += u64::from(bits);
    }
    palabras
}

/// Inverso exacto de [`empaquetar`]: recupera `n` valores de `bits` bits.
///
/// Con `bits == 0` devuelve `n` ceros (todo valor vale lo mismo cuando solo
/// hay uno distinto: el flujo no lleva información).
pub fn desempaquetar(palabras: &[u64], n: usize, bits: u32) -> Vec<u32> {
    assert!(bits <= 32, "bit packing de hasta 32 bits, pedido {bits}");
    if bits == 0 || n == 0 {
        return vec![0; n];
    }
    debug_assert!(
        n as u64 * bits as u64 <= palabras.len() as u64 * 64,
        "faltan palabras para {n} valores de {bits} bits"
    );
    let mascara = (1u64 << bits) - 1;
    let mut salida = Vec::with_capacity(n);
    for i in 0..n {
        let posicion = i as u64 * bits as u64;
        let idx = (posicion / 64) as usize;
        let offset = (posicion % 64) as u32;
        let usados = 64 - offset;
        let mut acumulado = palabras[idx] >> offset;
        if usados < bits {
            acumulado |= palabras[idx + 1] << usados;
        }
        salida.push((acumulado & mascara) as u32);
    }
    salida
}

// ─────────────────── Ejecución por lotes ───────────────────

/// Filtro por lotes sobre una columna entera: el corazón vectorizado.
///
/// Dos pasadas POR LOTE de [`TAMANIO_VECTOR`] (X100, CIDR 2005: vectores
/// cache-residentes, no columnas enteras materializadas):
///
/// 1. **Máscara**: un bucle APRETADO sin ramas internas calcula
///    `mascara[i] = admite[i] && predicado(valor[i])`. Sin `push` ni `if`
///    por elemento, LLVM puede pipelinar y — si el ensamblador acompaña —
///    auto-vectorizar (prometemos MEDIRLO, no prometerlo).
/// 2. **Compactación**: segunda vuelta que empuja las posiciones admitidas.
///    La rama vive aquí, AISLADA del bucle caliente.
///
/// El RESTO (`len % 1024`) se procesa con la misma forma fuera del bucle
/// principal: el bucle caliente queda regular y el caso borde tiene test
/// dedicado (`vector_tamano_fijo_maneja_el_resto`).
///
/// `admite` (presencia) entra en la misma máscara: un hueco no puede pasar
/// ningún predicado aunque su valor neutro lo satisficiera — semántica
/// idéntica al row store donde la prop ausente simplemente no está.
pub fn filtrar_lote_i64(
    valores: &[i64],
    admite: &[bool],
    predicado: impl Fn(i64) -> bool,
) -> Vec<usize> {
    debug_assert_eq!(
        valores.len(),
        admite.len(),
        "presencia y valores deben estar alineados"
    );
    let mut seleccion = Vec::new();
    let n = valores.len();
    let lotes_completos = n / TAMANIO_VECTOR;

    let mut mascara = [false; TAMANIO_VECTOR];
    for lote in 0..lotes_completos {
        let base = lote * TAMANIO_VECTOR;
        let chunk = &valores[base..base + TAMANIO_VECTOR];
        let chunk_admite = &admite[base..base + TAMANIO_VECTOR];

        // PASADA 1: máscara sin ramas internas (el bucle caliente).
        for (m, (&ok, &v)) in mascara.iter_mut().zip(chunk_admite.iter().zip(chunk)) {
            *m = ok && predicado(v);
        }
        // PASADA 2: compactación (rama aislada del bucle caliente).
        for (i, &m) in mascara.iter().enumerate() {
            if m {
                seleccion.push(base + i);
            }
        }
    }

    // RESTO: misma forma, tamaño arbitrario, FUERA del bucle caliente.
    let base_resto = lotes_completos * TAMANIO_VECTOR;
    let resto = &valores[base_resto..];
    let resto_admite = &admite[base_resto..];
    let mut mascara_resto = vec![false; resto.len()];
    for (m, (&ok, &v)) in mascara_resto.iter_mut().zip(resto_admite.iter().zip(resto)) {
        *m = ok && predicado(v);
    }
    for (i, &m) in mascara_resto.iter().enumerate() {
        if m {
            seleccion.push(base_resto + i);
        }
    }
    seleccion
}

/// Ids vivos del store en orden ascendente (la extracción determinista
/// reutilizable). Separa la PREPARACIÓN del ESCANEO para que el bench de
/// fila mida solo el escaneo — el mismo trato que recibe el lado columnar,
/// cuya extracción también va fuera de la región medida.
pub fn ids_ordenados(store: &MemoryStore) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = store.iter_nodes().map(|nodo| nodo.id).collect();
    ids.sort_unstable();
    ids
}

/// El filtro ROW STORE original: fila a fila, `HashMap::get` + tag por celda.
///
/// Sobre ids YA ordenados (ver [`ids_ordenados`]). Este es el contrincante
/// honesto del filtro por lotes: mismo predicado, mismo dataset, misma
/// salida esperada — y cada llamada paga el dispatch de tag y la búsqueda
/// hash que el layout columnar sacó del bucle. La equivalencia EXACTA entre
/// esta función y `TablaColumnar::filtrar_int` es TEST-TESIS del capítulo.
pub fn filtrar_fila_sobre_ids(
    store: &MemoryStore,
    ids: &[NodeId],
    clave: &str,
    predicado: impl Fn(i64) -> bool,
) -> Vec<NodeId> {
    ids.iter()
        .copied()
        .filter(
            |&id| match store.get_node(id).and_then(|nodo| nodo.props.get(clave)) {
                Some(Value::Int(v)) => predicado(*v),
                _ => false,
            },
        )
        .collect()
}

/// Conveniencia completa (prepara ids + filtra): la forma de TEST.
pub fn filtrar_fila_i64(
    store: &MemoryStore,
    clave: &str,
    predicado: impl Fn(i64) -> bool,
) -> Vec<NodeId> {
    let ids = ids_ordenados(store);
    filtrar_fila_sobre_ids(store, &ids, clave, predicado)
}

// ─────────────────── Factorización de resultados (2-hop) ───────────────────

/// Subgrafo DETERMINISTA y ACOTADO para experimentar con expansión 2-hop.
///
/// Por qué existe (riesgo anticipado §5b): expandir dos saltos desde hubs del
/// dataset real multiplica grados — el CONTEO puede desbocarse y materializar
/// tuplas planas agota RAM. La cura del capítulo es acotar: primeros
/// `max_nodos` ids vivos (ordenados), aristas internas al subgrafo, cada
/// lista de adyacencia ordenada, sin duplicados y truncada a `tope_grado`.
/// Todo determinista: mismas aristas, mismo orden, mismos números SIEMPRE.
#[derive(Debug, Clone)]
pub struct SubgrafoAcotado {
    /// Ids incluidos, ascendentes; posición ↔ fila de `adj_out`.
    pub ids: Vec<NodeId>,
    /// Adyacencia saliente POSICIONAL (paralela a `ids`), listas ordenadas
    /// y acotadas. Los destinos son POSICIONES dentro del subgrafo, no ids
    /// globales: así la expansión nunca sale de la caja.
    pub adj_out: Vec<Vec<usize>>,
}

impl SubgrafoAcotado {
    /// Construye el subgrafo inducido por los primeros `max_nodos` ids,
    /// con grado saliente acotado a `tope_grado` (menores destinos primero).
    pub fn construir(store: &MemoryStore, max_nodos: usize, tope_grado: usize) -> Self {
        let mut ids: Vec<NodeId> = store.iter_nodes().map(|nodo| nodo.id).collect();
        ids.sort_unstable();
        ids.truncate(max_nodos);
        let posicion: HashMap<NodeId, usize> = ids
            .iter()
            .copied()
            .enumerate()
            .map(|(pos, id)| (id, pos))
            .collect();

        let mut adj_out = Vec::with_capacity(ids.len());
        for &id in &ids {
            let mut vecinos: Vec<usize> = store
                .out_edges(id)
                .into_iter()
                .filter_map(|eid| store.get_edge(eid).map(|arista| arista.target))
                .filter_map(|target| posicion.get(&target).copied())
                .collect();
            vecinos.sort_unstable();
            vecinos.dedup();
            vecinos.truncate(tope_grado);
            adj_out.push(vecinos);
        }
        SubgrafoAcotado { ids, adj_out }
    }
}

/// Conteo ARITMÉTICO de tuplas 2-hop SIN materializarlas.
///
/// `Σ_p Σ_{v ∈ N⁺(p)} |N⁺(v)|` con `checked_add`: sobre un subgrafo acotado
/// el total cabe de sobra en u64, pero el desborde se NOMBRA en vez de
/// envolver en silencio — la lección del riesgo §5b convertida en código.
pub fn conteo_dos_saltos(adj_out: &[Vec<usize>]) -> u64 {
    let mut total = 0u64;
    for vecinos in adj_out {
        for &v in vecinos {
            total = total
                .checked_add(adj_out[v].len() as u64)
                .expect("subgrafo acotado: el conteo 2-hop debe caber en u64 (riesgo §5b)");
        }
    }
    total
}

/// Expansión plana de referencia: TODAS las tuplas (p, v, w), materializadas.
///
/// Es la representación que el motor Volcano produciría hoy (cap. 20) y el
/// contrincante de la factorización: 3 celdas por tupla, multiplicadas por
/// el fan-out de cada nivel. Ordenada para comparar como MULTICONJUNTO.
pub fn expansion_plana(adj_out: &[Vec<usize>]) -> Vec<(usize, usize, usize)> {
    let mut tuplas = Vec::new();
    for (p, vecinos) in adj_out.iter().enumerate() {
        for &v in vecinos {
            for &w in &adj_out[v] {
                tuplas.push((p, v, w));
            }
        }
    }
    tuplas.sort_unstable();
    tuplas
}

/// Resultado 2-hop FACTORIZADO: arrays por variable + multiplicidad.
///
/// La idea (Olteanu-Závodný, ICDT 2015; processor factorizado de Kùzu — Jin
/// et al., CIDR 2023, CC-BY 4.0): el multiconjunto lógico de tuplas se
/// representa COMPARTIENDO prefijos. Aquí, dos niveles de CSR:
///
/// ```text
/// pivotes:      [p₀, p₁, …]                 (una celda por pivote CON resultados)
/// inicio_slot:  CSR pivote → slots
/// intermedios:  [v…]                        (una celda por par (p,v) vivo)
/// inicio_destino: CSR slot → destinos
/// destinos:     [w…]                        (una celda por tupla lógica)
/// ```
///
/// Cada entrada de `destinos` corresponde a EXACTAMENTE una tupla lógica
/// (p, v, w) — nada se pierde ni duplica (test-tesis de equivalencia) — pero
/// el pivote p y el intermedio v se almacenan UNA VEZ compartidos entre
/// todos los w que cuelgan de ellos. `multiplicidad_pivote[k]` hace visible
/// cuántas tuplas comparte el pivote k.
///
/// Atribución (ADR-001, vinculante): Kùzu fue archivada tras la adquisición
/// por Apple (oct-2025); existen forks LadybugDB/bighorn. Este módulo es
/// clean-room: implementa el CONCEPTO publicado en CIDR 2023, cero código.
/// El motor factorizado completo y los joins worst-case optimal son del
/// cap. 39 — frontera declarada, aquí solo la demostración mínima.
#[derive(Debug, Clone)]
pub struct ExpansionFactorizada {
    /// Pivotes con al menos una tupla, ascendentes (posiciones del subgrafo).
    pub pivotes: Vec<usize>,
    /// Tuplas lógicas que comparte cada pivote (paralelo a `pivotes`).
    pub multiplicidad_pivote: Vec<u64>,
    /// CSR: los slots del pivote k son `inicio_slot[k]..inicio_slot[k+1]`.
    pub inicio_slot: Vec<usize>,
    /// Nodo intermedio v de cada slot (par (p, v) compartido).
    pub intermedios: Vec<usize>,
    /// CSR: los destinos del slot s son `inicio_destino[s]..inicio_destino[s+1]`.
    pub inicio_destino: Vec<usize>,
    /// Destino w de cada tupla lógica (una entrada por tupla).
    pub destinos: Vec<usize>,
}

impl ExpansionFactorizada {
    /// Factoriza la expansión 2-hop de unas adyacencias posicionales.
    ///
    /// Solo entran pivotes con AL MENOS un intermedio con salida: un pivote
    /// sin tuplas no ocupa NI una celda (la factorización no paga por
    /// filas que no existen).
    pub fn desde_adjacencias(adj_out: &[Vec<usize>]) -> Self {
        let mut pivotes = Vec::new();
        let mut multiplicidad_pivote = Vec::new();
        let mut inicio_slot = vec![0usize];
        let mut intermedios = Vec::new();
        let mut inicio_destino: Vec<usize> = Vec::new();
        let mut destinos = Vec::new();

        for (p, vecinos) in adj_out.iter().enumerate() {
            let mut filas_pivote = 0u64;
            let mut slots_pivote = 0usize;
            for &v in vecinos {
                let salidas = &adj_out[v];
                if salidas.is_empty() {
                    continue;
                }
                inicio_destino.push(destinos.len());
                intermedios.push(v);
                destinos.extend_from_slice(salidas);
                filas_pivote += salidas.len() as u64;
                slots_pivote += 1;
            }
            if slots_pivote > 0 {
                pivotes.push(p);
                inicio_slot.push(intermedios.len());
                multiplicidad_pivote.push(filas_pivote);
            }
        }
        // Cierra el último slot del CSR de destinos (y da longitud
        // intermedios+1 incluso con cero slots).
        inicio_destino.push(destinos.len());

        debug_assert_eq!(
            multiplicidad_pivote.iter().sum::<u64>(),
            destinos.len() as u64,
            "la multiplicidad debe sumar exactamente las filas lógicas"
        );
        ExpansionFactorizada {
            pivotes,
            multiplicidad_pivote,
            inicio_slot,
            intermedios,
            inicio_destino,
            destinos,
        }
    }

    /// Filas LÓGICAS: tuplas (p,v,w) que la consulta significa.
    pub fn filas_logicas(&self) -> u64 {
        self.destinos.len() as u64
    }

    /// Celdas FÍSICAS: lo que esta representación guarda de verdad
    /// (pivotes + intermedios compartidos + destinos). Los offsets CSR son
    /// estructura de recorrido, no datos — quedan fuera del conteo, como se
    /// declara en la prosa.
    pub fn celdas_fisicas(&self) -> u64 {
        (self.pivotes.len() + self.intermedios.len() + self.destinos.len()) as u64
    }

    /// Lo que ocuparían las mismas tuplas PLANAS: 3 celdas por fila lógica.
    pub fn celdas_planas(&self) -> u64 {
        3 * self.filas_logicas()
    }

    /// Ahorro porcentual de celdas frente a tuplas planas (0.0–100.0).
    pub fn ahorro_porcentaje(&self) -> f64 {
        let planas = self.celdas_planas();
        if planas == 0 {
            return 0.0;
        }
        (1.0 - self.celdas_fisicas() as f64 / planas as f64) * 100.0
    }

    /// Itera el multiconjunto lógico COMPLETO expandiendo la factorización.
    ///
    /// Es el lado TESIS del test de equivalencia: recorrer esta estructura
    /// produce exactamente las mismas tuplas que [`expansion_plana`].
    pub fn por_cada_tupla(&self, mut visitar: impl FnMut(usize, usize, usize)) {
        for (k, &p) in self.pivotes.iter().enumerate() {
            for s in self.inicio_slot[k]..self.inicio_slot[k + 1] {
                let v = self.intermedios[s];
                for d in self.inicio_destino[s]..self.inicio_destino[s + 1] {
                    visitar(p, v, self.destinos[d]);
                }
            }
        }
    }

    /// Todas las tuplas lógicas, ordenadas (comodidad para tests).
    pub fn tuplas_ordenadas(&self) -> Vec<(usize, usize, usize)> {
        let mut tuplas = Vec::with_capacity(self.filas_logicas() as usize);
        self.por_cada_tupla(|p, v, w| tuplas.push((p, v, w)));
        tuplas.sort_unstable();
        tuplas
    }
}

// ─────────────────── Informe para la prosa ───────────────────

/// Informe de RATIOS y CONTEOS sobre el dataset de referencia, imprimible.
///
/// Qué incluye: densidad y tipo por columna; veredicto del diccionario
/// (GANA en baja cardinalidad / PIERDE en `email`) con los bytes medidos;
/// ratio del bit packing sobre los códigos de `ciudad`; y filas lógicas vs
/// celdas físicas de la factorización 2-hop sobre un subgrafo acotado.
/// Qué NO incluye: tiempos — esos son de criterion
/// (`benches/bench_columnar.rs`), con warm-up y estadística; un cronómetro
/// inline aquí sería un número sin calibrado (regla del cap. 34).
///
/// Reproducibilidad: todo lo impreso deriva del store por caminos
/// deterministas (ids ordenados, BTreeMap de claves, subgrafo acotado) — dos
/// llamadas producen EL MISMO texto byte a byte, y hay un test que lo pinna.
pub fn informe_columnar(store: &MemoryStore) -> String {
    use crate::cap34_benchmarks::CLAVES_ESQUEMA;

    const CLAVES_DICCIONARIO: [&str; 5] = ["ciudad", "pais", "idioma", "categoria", "email"];
    const MAX_NODOS_SUBGRAFO: usize = 256;
    const TOPE_GRADO: usize = 16;

    let tabla = TablaColumnar::desde_store(store, &CLAVES_ESQUEMA);
    let mut lineas: Vec<String> = Vec::new();

    lineas.push("=== Informe columnar (cap. 38) ===".to_string());
    lineas.push(format!(
        "filas: {} nodos | claves del esquema: {} | columnas: {}",
        tabla.num_filas(),
        CLAVES_ESQUEMA.len(),
        tabla.claves().count()
    ));

    lineas.push("-- columnas (tipo, densidad) --".to_string());
    for clave in tabla.claves() {
        let col = tabla.columna(clave).expect("clave de la propia lista");
        lineas.push(format!(
            "{clave:<12} {:<6} densidad {:>5.1}%  presentes {}/{}",
            col.valores().tipo().nombre(),
            col.densidad() * 100.0,
            col.presentes(),
            tabla.num_filas()
        ));
    }

    lineas.push("-- dictionary encoding --".to_string());
    for clave in CLAVES_DICCIONARIO {
        let Some(col) = tabla.columna(clave) else {
            continue;
        };
        let Some(cadenas) = col.cadenas_presentes() else {
            continue;
        };
        let dic = Diccionario::nuevo(&cadenas);
        let est = dic.estadisticas();
        let veredicto = if est.ratio() >= 1.0 { "GANA" } else { "PIERDE" };
        lineas.push(format!(
            "{clave:<12} cardinalidad {:>4}  bytes {} -> {}  ratio x{:.2}  {veredicto}",
            est.cardinalidad,
            est.bytes_antes,
            est.bytes_despues,
            est.ratio()
        ));
    }
    lineas.push(
        "veredicto bidireccional: el diccionario gana en dominios pequeños \
         y pierde en claves únicas (email)"
            .to_string(),
    );

    lineas.push("-- bit packing sobre los códigos de 'ciudad' --".to_string());
    if let Some(cadenas) = tabla
        .columna("ciudad")
        .and_then(ColumnaTipada::cadenas_presentes)
    {
        let dic = Diccionario::nuevo(&cadenas);
        let bits = bits_necesarios(dic.cardinalidad());
        let empaquetado = empaquetar(dic.codigos(), bits);
        let bytes_u32 = 4 * dic.codigos().len() as u64;
        let bytes_empaquetados = 8 * empaquetado.len() as u64;
        let ratio = if bytes_empaquetados == 0 {
            1.0
        } else {
            bytes_u32 as f64 / bytes_empaquetados as f64
        };
        lineas.push(format!(
            "{} códigos en {bits} bits: {} B (u32) -> {} B (packed)  ratio x{:.2}",
            dic.codigos().len(),
            bytes_u32,
            bytes_empaquetados,
            ratio
        ));
    }

    lineas.push("-- factorización 2-hop (subgrafo acotado) --".to_string());
    let subgrafo = SubgrafoAcotado::construir(store, MAX_NODOS_SUBGRAFO, TOPE_GRADO);
    let factorizado = ExpansionFactorizada::desde_adjacencias(&subgrafo.adj_out);
    lineas.push(format!(
        "subgrafo: {} nodos, tope de grado {}",
        subgrafo.ids.len(),
        TOPE_GRADO
    ));
    lineas.push(format!(
        "filas lógicas: {} tuplas (p,v,w) | celdas planas: {} | celdas físicas: {} | ahorro {:.1}%",
        factorizado.filas_logicas(),
        factorizado.celdas_planas(),
        factorizado.celdas_fisicas(),
        factorizado.ahorro_porcentaje()
    ));
    lineas.push(format!(
        "conteo aritmético sin materializar: {} tuplas (checked_add)",
        conteo_dos_saltos(&subgrafo.adj_out)
    ));

    let mut informe = lineas.join("\n");
    informe.push('\n');
    informe
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_columnar {
    use super::*;
    use crate::cap07_modelo::Node;
    use crate::cap34_benchmarks::{
        CLAVES_ESQUEMA, SEMILLA_REFERENCIA, Xorshift64Star, dataset_referencia_mini,
    };

    /// Verificación exhaustiva celda a celda: la tabla contra el row store.
    /// Para cada fila y cada clave del esquema: presencia del BitSet ==
    /// existencia de la prop, y valor de la columna == valor del prop.
    fn verificar_contra_row_store(tabla: &TablaColumnar, store: &MemoryStore) {
        for clave in CLAVES_ESQUEMA {
            let col = tabla
                .columna(clave)
                .unwrap_or_else(|| panic!("falta la columna {clave}"));
            let Columna::Int(vals_int) = col.valores() else {
                continue;
            };
            assert_eq!(vals_int.len(), tabla.num_filas());
            for (i, &id) in tabla.ids().iter().enumerate() {
                let nodo = store.get_node(id).expect("fila viva");
                let presente = nodo.props.contains_key(clave);
                assert_eq!(
                    col.presencia().contiene(i),
                    presente,
                    "presencia divergente en fila {i} ({clave})"
                );
                if let Some(Value::Int(v)) = nodo.props.get(clave) {
                    assert_eq!(vals_int[i], *v, "valor divergente en fila {i}");
                }
            }
        }
    }

    #[test]
    fn columnar_tabla_desde_dataset_tipos_y_ordenes() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let tabla = TablaColumnar::desde_store(&ds.store, &CLAVES_ESQUEMA);

        // Orden estable: ids estrictamente ascendentes e idénticos a los
        // vivos ordenados — el determinismo que `iter_nodes` no da gratis.
        assert!(tabla.ids().windows(2).all(|w| w[0] < w[1]));
        let esperados = ids_ordenados(&ds.store);
        assert_eq!(tabla.ids(), esperados.as_slice());

        // Tipos correctos por clave según el generador del cap. 34:
        // edad/nivel/antiguedad/prioridad → Int, saldo/puntuacion → Float,
        // ciudad/email/… → String, activo/… → Bool.
        for (clave, esperado) in [
            ("edad", TipoDatoColumna::Int),
            ("nivel", TipoDatoColumna::Int),
            ("antiguedad", TipoDatoColumna::Int),
            ("prioridad", TipoDatoColumna::Int),
            ("saldo", TipoDatoColumna::Float),
            ("puntuacion", TipoDatoColumna::Float),
            ("ciudad", TipoDatoColumna::String),
            ("email", TipoDatoColumna::String),
            ("activo", TipoDatoColumna::Bool),
            ("suscriptor", TipoDatoColumna::Bool),
        ] {
            let col = tabla
                .columna(clave)
                .unwrap_or_else(|| panic!("falta {clave}"));
            assert_eq!(col.valores().tipo(), esperado, "tipo de {clave}");
        }

        // Tesis fuerte: TODAS las celdas de TODAS las columnas Int coinciden
        // con el row store (presencia y valor).
        verificar_contra_row_store(&tabla, &ds.store);

        // Homogeneidad del dataset: ninguna clave mezcla tipos.
        for clave in tabla.claves() {
            let col = tabla.columna(clave).expect("clave propia");
            assert_eq!(col.descartes_por_tipo(), 0, "{clave} sería heterogénea");
        }
    }

    #[test]
    fn columna_preserva_ausentes_con_bitset() {
        // Store a mano con huecos: ids 0, 2 y 5 (tras "delete_node" el
        // espacio de ids tiene agujeros — la columna solo guarda vivos).
        let mut store = MemoryStore::new();
        store
            .put_node(Node::new(0, "Persona").with_prop("edad", Value::Int(30)))
            .expect("id 0 nuevo");
        store.put_node(Node::new(2, "Persona")).expect("id 2 nuevo"); // SIN edad
        store
            .put_node(Node::new(5, "Persona").with_prop("edad", Value::Int(45)))
            .expect("id 5 nuevo");

        let tabla = TablaColumnar::desde_store(&store, &["edad"]);
        assert_eq!(tabla.ids(), &[0, 2, 5]);

        let col = tabla.columna("edad").expect("edad extraída");
        // Presencia POSICIONAL: la fila 1 (id 2) no está; el id 5 vive en la
        // posición 2 — el BitSet marca posiciones, no ids.
        assert_eq!(col.presentes(), 2);
        assert!(col.presencia().contiene(0));
        assert!(!col.presencia().contiene(1));
        assert!(col.presencia().contiene(2));

        // El hueco PRESERVA la alineación: 3 celdas, valor neutro en el hueco.
        let Columna::Int(valores) = col.valores() else {
            panic!("edad debía ser Int");
        };
        assert_eq!(valores, &[30, 0, 45]);
        assert!((col.densidad() - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn diccionario_roundtrip_y_estadisticas_ciudad() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let tabla = TablaColumnar::desde_store(&ds.store, &["ciudad"]);
        let ciudades = tabla
            .columna("ciudad")
            .and_then(ColumnaTipada::cadenas_presentes)
            .expect("ciudad es String");

        let dic = Diccionario::nuevo(&ciudades);

        // Roundtrip EXACTO: decodificar reconstruye la entrada byte a byte.
        assert_eq!(
            dic.decodificar(),
            ciudades.iter().map(String::as_str).collect::<Vec<_>>()
        );

        // Acceso directo por código coherente con el mapa.
        for (valor, &codigo) in ciudades.iter().zip(dic.codigos()) {
            assert_eq!(dic.cadena_de(codigo), Some(valor.as_str()));
        }
        // Y la mitad que hace rentable operar comprimido: literal → código
        // una vez, comparación u32 contra u32 después.
        let primera = &ciudades[0];
        assert_eq!(
            dic.codigo_de(primera),
            Some(dic.codigos()[0]),
            "el primer valor visto debía tener el código 0"
        );
        assert_eq!(dic.codigo_de("esta-ciudad-no-existe"), None);

        // Estadística consistente con una cuenta hecha a mano.
        let est = dic.estadisticas();
        let distintos: std::collections::HashSet<&String> = ciudades.iter().collect();
        assert_eq!(est.cardinalidad, distintos.len());
        assert_eq!(est.elementos, ciudades.len());
        let bytes_a_mano: u64 = ciudades.iter().map(|s| s.len() as u64).sum();
        assert_eq!(est.bytes_antes, bytes_a_mano);
        let bytes_tabla: u64 = distintos.iter().map(|s| s.len() as u64).sum();
        assert_eq!(est.bytes_despues, 4 * ciudades.len() as u64 + bytes_tabla);
    }

    #[test]
    fn diccionario_gana_en_ciudad_y_pierde_en_email() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let tabla = TablaColumnar::desde_store(&ds.store, &["ciudad", "email"]);

        let ratio_de = |clave: &str| -> f64 {
            let col = tabla.columna(clave).expect("columna del esquema");
            let cadenas = col
                .cadenas_presentes()
                .unwrap_or_else(|| panic!("{clave} debía ser String"));
            Diccionario::nuevo(&cadenas).estadisticas().ratio()
        };

        // VEREDICTO BIDIRECCIONAL medido: ciudad (≤ 32 dominios) gana;
        // email (único por nodo) pierde — y la pérdida SE REPORTA.
        let ratio_ciudad = ratio_de("ciudad");
        let ratio_email = ratio_de("email");
        assert!(
            ratio_ciudad > 1.0,
            "ciudad debía ganar (ratio {ratio_ciudad})"
        );
        assert!(
            ratio_email < 1.0,
            "email debía perder (ratio {ratio_email}): 4 B de código por cada \
             cadena única no recuperan nada"
        );
    }

    #[test]
    fn bit_packing_casos_conocidos_exactos() {
        // Caso 1 (el de la prosa): 3 bits, LSB-first, sin partir palabra.
        // 5=101 @0, 2=010 @3, 7=111 @6  →  0b111_010_101 = 469.
        assert_eq!(empaquetar(&[5, 2, 7], 3), vec![469u64]);
        assert_eq!(desempaquetar(&[469], 3, 3), vec![5, 2, 7]);

        // Caso 2 (partido): 21 bits × 4 valores = 83 bits → 2 palabras.
        // El 4 arranca en el bit 63: 1 bit en w0 y 3 en w1 (4>>1 = 2).
        let w0 = 1u64 | (2u64 << 21) | (3u64 << 42) | (4u64 << 63);
        assert_eq!(empaquetar(&[1, 2, 3, 4], 21), vec![w0, 2]);
        assert_eq!(desempaquetar(&[w0, 2], 4, 21), vec![1, 2, 3, 4]);

        // Caso 3 (ancho máximo): 32 bits, alineación exacta de dos por palabra.
        assert_eq!(
            empaquetar(&[u32::MAX, 0, 123], 32),
            vec![0xFFFF_FFFFu64, 123]
        );

        // Cardinalidades límite de bits_necesarios.
        assert_eq!(bits_necesarios(1), 0);
        assert_eq!(bits_necesarios(2), 1);
        assert_eq!(bits_necesarios(32), 5);
        assert_eq!(bits_necesarios(33), 6);
        assert_eq!(bits_necesarios(1 << 20), 20);
    }

    #[test]
    fn bit_packing_roundtrip_k_de_1_a_32() {
        let mut rng = Xorshift64Star::new(SEMILLA_REFERENCIA ^ 0x0CA7_0380);
        for k in 1..=32u32 {
            let techo = 1u64 << k;
            let n = 512;
            let valores: Vec<u32> = (0..n).map(|_| rng.debajo_de(techo) as u32).collect();
            let palabras = empaquetar(&valores, k);
            // Denso: el número de palabras es exactamente el necesario.
            assert_eq!(
                palabras.len(),
                (n as u64 * k as u64).div_ceil(64) as usize,
                "k={k}"
            );
            assert_eq!(desempaquetar(&palabras, n, k), valores, "roundtrip k={k}");
        }

        // k = 0 trivial: un solo valor distinto → códigos todos 0, flujo vacío.
        let codigos = vec![0u32; 64];
        assert!(empaquetar(&codigos, 0).is_empty());
        assert_eq!(desempaquetar(&[], 64, 0), codigos);
    }

    #[test]
    fn filtro_por_lotes_equivale_al_filtro_fila() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let tabla = TablaColumnar::desde_store(&ds.store, &["edad"]);
        let predicado = |edad: i64| edad > 50;

        // TESIS del capítulo: el mismo predicado sobre el mismo dataset
        // produce EXACTAMENTE los mismos ids en ambos layouts.
        let por_lotes = tabla.filtrar_int("edad", predicado).expect("edad Int");
        let por_fila = filtrar_fila_i64(&ds.store, "edad", predicado);
        assert_eq!(por_lotes, por_fila);
        assert!(!por_lotes.is_empty(), "selectividad nula: test sordo");

        // Y con la preparación separada (la forma del bench) también cuadra.
        let ids = ids_ordenados(&ds.store);
        assert_eq!(
            filtrar_fila_sobre_ids(&ds.store, &ids, "edad", predicado),
            por_lotes
        );
    }

    #[test]
    fn vector_tamano_fijo_maneja_el_resto() {
        // Referencia ingenua: filter elementwise con la misma semántica.
        let naive = |valores: &[i64], admite: &[bool]| -> Vec<usize> {
            valores
                .iter()
                .zip(admite)
                .enumerate()
                .filter(|&(_, (&v, &ok))| ok && v % 3 == 0)
                .map(|(i, _)| i)
                .collect()
        };
        for &n in &[0usize, 1, 1023, 1024, 1025, 2047, 2048, 3000] {
            let mut rng = Xorshift64Star::new(SEMILLA_REFERENCIA ^ n as u64);
            let valores: Vec<i64> = (0..n).map(|_| rng.debajo_de(100) as i64).collect();
            let admite: Vec<bool> = (0..n).map(|i| i % 7 != 0).collect();
            let esperado = naive(&valores, &admite);
            let obtenido = filtrar_lote_i64(&valores, &admite, |v| v % 3 == 0);
            assert_eq!(
                obtenido,
                esperado,
                "tamaño {n} (resto {})",
                n % TAMANIO_VECTOR
            );
        }
    }

    #[test]
    fn informe_columnar_reproducible_sobre_mini() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let primera = informe_columnar(&ds.store);
        let segunda = informe_columnar(&ds.store);

        // Reproducibilidad byte a byte: sin ella, la prosa no puede pegarla.
        assert_eq!(primera, segunda);

        // Marcadores estables que la prosa citará.
        assert!(primera.contains("-- dictionary encoding --"));
        assert!(primera.contains("ciudad") && primera.contains("GANA"));
        assert!(primera.contains("email") && primera.contains("PIERDE"));
        assert!(primera.contains("filas lógicas"));
        assert!(primera.contains("celdas físicas"));
        assert!(primera.contains("ahorro"));
        // La cabecera cuenta las filas del mini: 400 nodos deterministas.
        assert!(primera.contains(&format!("filas: {} nodos", ds.store.node_count())));
    }

    #[test]
    fn factorizacion_dos_saltos_filas_logicas_vs_celdas() {
        // Grafo a mano, cuentas FIJAS (el test que la prosa puede citar):
        //   0 → [1, 2];  1 → [3, 4];  2 → [5];  6 → [1]
        let adj: Vec<Vec<usize>> = vec![
            vec![1, 2],
            vec![3, 4],
            vec![5],
            vec![],  // 3
            vec![],  // 4
            vec![],  // 5
            vec![1], // 6
            vec![],  // 7
        ];

        // Plano: (0,1,3) (0,1,4) (0,2,5) (6,1,3) (6,1,4) → 5 tuplas.
        assert_eq!(expansion_plana(&adj).len(), 5);
        assert_eq!(conteo_dos_saltos(&adj), 5);

        let fact = ExpansionFactorizada::desde_adjacencias(&adj);
        // Pivotes con resultados: 0 y 6 (el 1 tiene aristas pero cero 2-hop).
        assert_eq!(fact.pivotes, vec![0, 6]);
        assert_eq!(fact.multiplicidad_pivote, vec![3, 2]);
        assert_eq!(fact.intermedios, vec![1, 2, 1]);
        assert_eq!(fact.destinos, vec![3, 4, 5, 3, 4]);

        // Conteo exacto: filas lógicas vs celdas físicas vs planas.
        assert_eq!(fact.filas_logicas(), 5);
        assert_eq!(fact.celdas_fisicas(), 2 + 3 + 5);
        assert_eq!(fact.celdas_planas(), 15);
        assert!((fact.ahorro_porcentaje() - (1.0 - 10.0 / 15.0) * 100.0).abs() < 1e-9);
    }

    #[test]
    fn factorizacion_equivale_a_la_expansion_plana() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        // Subgrafo determinista y acotado (riesgo §5b): 64 nodos, grado ≤ 8.
        let subgrafo = SubgrafoAcotado::construir(&ds.store, 64, 8);

        let plano = expansion_plana(&subgrafo.adj_out);
        let fact = ExpansionFactorizada::desde_adjacencias(&subgrafo.adj_out);

        // TESIS: iterando la estructura factorizada salen LAS MISMAS tuplas
        // (multiconjunto exacto) que expandiendo plano.
        assert_eq!(fact.tuplas_ordenadas(), plano);
        // El conteo aritmético coincide con lo materializado…
        assert_eq!(conteo_dos_saltos(&subgrafo.adj_out), plano.len() as u64);
        assert_eq!(fact.filas_logicas(), plano.len() as u64);
        // …y la multiplicidad por pivote suma las mismas filas.
        assert_eq!(
            fact.multiplicidad_pivote.iter().sum::<u64>(),
            fact.filas_logicas()
        );

        // Con hubs compartidos la factorización NO puede crecer: celdas
        // físicas ≤ celdas planas siempre (compartir nunca añade celdas).
        assert!(fact.celdas_fisicas() <= fact.celdas_planas());
    }
}
