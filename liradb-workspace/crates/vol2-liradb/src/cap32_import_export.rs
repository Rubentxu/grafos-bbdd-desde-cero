use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufRead, Write};

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId, Value};
use crate::cap08_graph_store::{GraphStore, StoreError};
use crate::cap27_transacciones::{Operacion, autocommit};

// ─────────────────── Cap 32: importación y exportación ───────────────────
//
// La pregunta crítica del capítulo (CORPUS): «Streaming para datasets > RAM».
// Un fichero de 10 millones de nodos NO cabe en memoria: la importación
// tiene que ser LÍNEA A LÍNEA, convirtiendo cada registro en una Operacion
// y aplicándola EN EL ACTO. La exportación, igual: iterar el store y
// escribir cada línea sin materializar la salida.
//
// La decisión de streaming más importante del capítulo (el porqué central):
// **autocommit POR REGISTRO**. La alternativa obvia —una gran `Transaccion`
// del cap. 27 con todo el fichero en staging y un solo commit— BUFFERA las
// operaciones en un `Vec<Operacion>` EN RAM: rompe exactamente el objetivo
// que el capítulo persigue. El autocommit por registro es el modo por
// defecto del motor (caps. 7-26: cada put_* su propia transacción) y aquí
// resulta ser también la ÚNICA forma de coste de memoria constante. El
// precio, documentado: un fallo a mitad de importación deja lo ya aplicado
// (sin store persistente no hay WAL que lo repare — cap. 37).
//
// Tres formatos, el mismo contrato:
//   * CSV (dos ficheros: nodos y aristas) con cabecera estilo neo4j-admin:
//     `id:ID, name:STRING, age:INT, :LABEL` para nodos y
//     `id:ID, de:START_ID, a:END_ID, tipo:TYPE, since:INT` para aristas.
//     Los sufijos de tipo (STRING/INT/FLOAT/BOOL) tipan cada columna.
//   * JSONL (un fichero, nodos y aristas mezclados línea a línea):
//     {"tipo":"nodo","id":7,"labels":["Person"],"props":{...}} y
//     {"tipo":"arista","id":0,"de":0,"a":1,"rel":"KNOWS","props":{...}}.
//   * GraphML (el estándar XML de intercambio de grafos: yEd, Gephi,
//     NetworkX): <key> declarando atributos, <node id="..."> con <data>,
//     <edge source target> con <data>.
//
// Los TRES parsers son A MANO (la crate del motor sigue SIN dependencias
// externas — un principio defendido en el grill): es la lección del lexer
// del cap. 18 repetida para DATOS. Subconjuntos documentados con honradez:
//   * CSV: comillas con comas y "" escapado; SIN saltos de línea dentro de
//     comillas (RFC 4180 los permite; aquí no).
//   * JSON: objetos/anidación/arrays/string/enteros/floats/bool/null con
//     escapes estándar incl. \uXXXX (BMP; pares sustitutos → error claro).
//   * XML (GraphML): etiquetas, atributos, entidades &amp; &lt; &gt;
//     &quot; &apos;; se ignoran comentarios y <?xml ...?>; SIN namespaces
//     ni CDATA.
//
// GraphML trae una lección de IDENTIDAD (cap. 3): sus ids de nodo son
// STRINGS externos ("n0"); los nuestros son usize densos. La importación
// mantiene un MAPA string→NodeId denso asignado por orden de aparición, y
// las aristas resuelven sus extremos por ese mapa. La exportación escribe
// nuestros ids numéricos como strings — el roundtrip es exacto.
//
// Todo fail-fast con NÚMERO DE LÍNEA: en un fichero de 10 millones, «fila
// malformada» sin línea no es un diagnóstico. Los errores son tipados
// ([`ImportError`]) y deterministas.

// ─────────────────── Errores ───────────────────

/// Errores tipados de importación/exportación. Todos los de registro
/// llevan `linea` (1-based) — el requisito del debugging de datasets.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// La cabecera CSV no tiene el marcador obligatorio (`:ID` en nodos;
    /// `:START_ID`/`:END_ID` en aristas).
    CabeceraInvalida { causa: String },
    /// Una fila no parsea (CSV mal citado, JSON roto, XML roto…).
    FilaMalformada { linea: usize, causa: String },
    /// La fila es válida pero el registro choca con el store (id
    /// duplicado, extremos de arista inexistentes): el `StoreError` del
    /// cap. 8 con la línea que lo causó.
    RegistroRechazado { linea: usize, causa: StoreError },
    /// Un fichero JSONL/GraphML con estructura válida pero semántica
    /// errónea (p.ej. `"tipo"` desconocido, `<edge>` con source desconocido).
    Semantica { linea: usize, causa: String },
    /// Error de E/S del fichero (lectura/escritura).
    Io(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::CabeceraInvalida { causa } => {
                write!(f, "import: cabecera inválida: {causa}")
            }
            ImportError::FilaMalformada { linea, causa } => {
                write!(f, "import: línea {linea} malformada: {causa}")
            }
            ImportError::RegistroRechazado { linea, causa } => {
                write!(f, "import: línea {linea} rechazada por el store: {causa}")
            }
            ImportError::Semantica { linea, causa } => {
                write!(f, "import: línea {linea}: {causa}")
            }
            ImportError::Io(e) => write!(f, "import: error de E/S: {e}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e.to_string())
    }
}

/// Lo que una importación cuenta al terminar (o fallar no siendo el caso:
/// fail-fast — si devuelve, es que importó TODO).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EstadisticasImport {
    /// Nodos aplicados.
    pub nodos: usize,
    /// Aristas aplicadas.
    pub aristas: usize,
    /// Líneas leídas (incluidas cabeceras, comentarios y vacías).
    pub lineas: usize,
}

impl fmt::Display for EstadisticasImport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "import: {} nodos y {} aristas ({} líneas)",
            self.nodos, self.aristas, self.lineas
        )
    }
}

// ─────────────────── CSV ───────────────────

/// Los tipos de columna que la cabecera CSV declara (sufijo `:TIPO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoColumna {
    /// Texto (el default si la columna no lleva sufijo).
    String,
    /// Entero i64.
    Int,
    /// Float f64.
    Float,
    /// true/false.
    Bool,
}

impl TipoColumna {
    /// Del sufijo de la cabecera; desconocido → Err (fail-fast en la
    /// cabecera, antes de gastar una lectura del fichero entero).
    fn del_sufijo(sufijo: &str) -> Result<TipoColumna, String> {
        match sufijo.to_ascii_uppercase().as_str() {
            "STRING" | "TEXT" => Ok(TipoColumna::String),
            "INT" | "INTEGER" | "LONG" => Ok(TipoColumna::Int),
            "FLOAT" | "DOUBLE" => Ok(TipoColumna::Float),
            "BOOL" | "BOOLEAN" => Ok(TipoColumna::Bool),
            otro => Err(format!(
                "tipo de columna desconocido ':{otro}' \
                                 (use STRING/INT/FLOAT/BOOL)"
            )),
        }
    }

    /// El sufijo canónico que la EXPORTACIÓN escribe.
    fn sufijo(self) -> &'static str {
        match self {
            TipoColumna::String => "STRING",
            TipoColumna::Int => "INT",
            TipoColumna::Float => "FLOAT",
            TipoColumna::Bool => "BOOL",
        }
    }
}

/// Parsea UNA línea CSV en sus campos (RFC 4180-lite).
///
/// Reglas: campos separados por `,`; un campo puede ir entre comillas
/// dobles y entonces SUS comas son literales; dentro de comillas, `""` es
/// una comilla escapada. Fuera de comillas no se admiten comillas a mitad
/// de campo (documentado: sin sorpresas silenciosas).
pub fn partir_csv(linea: &str) -> Result<Vec<String>, String> {
    let mut campos = Vec::new();
    let mut actual = String::new();
    let mut en_comillas = false;
    let bytes: Vec<char> = linea.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if en_comillas {
            if c == '"' {
                // ¿"" escapado o cierre?
                if bytes.get(i + 1) == Some(&'"') {
                    actual.push('"');
                    i += 2;
                    continue;
                }
                en_comillas = false;
                i += 1;
                continue;
            }
            actual.push(c);
            i += 1;
        } else if c == '"' && actual.is_empty() {
            en_comillas = true;
            i += 1;
        } else if c == ',' {
            campos.push(std::mem::take(&mut actual));
            i += 1;
        } else if c == '"' {
            return Err(format!("comilla inesperada dentro del campo '{}'", actual));
        } else {
            actual.push(c);
            i += 1;
        }
    }
    if en_comillas {
        return Err("comilla de apertura sin cierre".into());
    }
    campos.push(actual);
    Ok(campos)
}

/// El mapa de una cabecera de nodos: qué columnas son id/labels/props.
struct CabeceraNodos {
    /// Índice de la columna `:ID`.
    col_id: usize,
    /// Índice de la columna `:LABEL` (opcional).
    col_labels: Option<usize>,
    /// Props por índice de columna: (nombre, tipo).
    props: Vec<(String, TipoColumna)>,
    /// Número de columnas de la cabecera (para validar cada fila).
    n_columnas: usize,
}

/// El mapa de una cabecera de aristas.
struct CabeceraAristas {
    col_id: Option<usize>,
    col_de: usize,
    col_a: usize,
    col_tipo: usize,
    props: Vec<(String, TipoColumna)>,
    n_columnas: usize,
}

/// IMPORTA un CSV de nodos (streaming: línea a línea, autocommit por fila).
///
/// Cabecera esperada (estilo neo4j-admin): `id:ID, nombre:STRING, …,
/// :LABEL`. La columna `:LABEL` es opcional (múltiples labels separadas
/// por `:` dentro del campo). Propiedades sin sufijo → STRING.
pub fn importar_csv_nodos(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    let mut stats = EstadisticasImport::default();

    // Cabecera (línea 1).
    let mut linea = String::new();
    let n = entrada.read_line(&mut linea)?;
    if n == 0 {
        return Err(ImportError::CabeceraInvalida {
            causa: "fichero vacío (se esperaba la cabecera)".into(),
        });
    }
    stats.lineas += 1;
    let campos = partir_csv(linea.trim_end_matches(['\r', '\n']))
        .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;

    let mut col_id = None;
    let mut col_labels = None;
    let mut props: Vec<(String, TipoColumna)> = Vec::new();
    for (i, crudo) in campos.iter().enumerate() {
        let crudo = crudo.trim(); // el export escribe ", :LABEL": espacios fuera
        if crudo == ":LABEL" {
            col_labels = Some(i);
        } else if let Some((nombre, sufijo)) = crudo.split_once(':') {
            if sufijo.eq_ignore_ascii_case("ID") && !nombre.is_empty() {
                if col_id.is_some() {
                    return Err(ImportError::CabeceraInvalida {
                        causa: "dos columnas :ID".into(),
                    });
                }
                col_id = Some(i);
            } else {
                let tipo = TipoColumna::del_sufijo(sufijo)
                    .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
                props.push((nombre.to_string(), tipo));
            }
        } else {
            // Sin sufijo y sin marcador: prop STRING con ese nombre.
            props.push((crudo.to_string(), TipoColumna::String));
        }
    }
    let cab = CabeceraNodos {
        col_id: col_id.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna id:ID".into(),
        })?,
        col_labels,
        props,
        n_columnas: campos.len(),
    };

    // Filas (streaming).
    loop {
        let mut linea = String::new();
        let n = entrada.read_line(&mut linea)?;
        if n == 0 {
            return Ok(stats);
        }
        stats.lineas += 1;
        let recortada = linea.trim_end_matches(['\r', '\n']);
        if recortada.is_empty() {
            continue; // línea en blanco: se salta, no cuenta error
        }
        let campos = partir_csv(recortada).map_err(|e| ImportError::FilaMalformada {
            linea: stats.lineas,
            causa: e,
        })?;
        if campos.len() != cab.n_columnas {
            return Err(ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!(
                    "la fila tiene {} campos y la cabecera {}",
                    campos.len(),
                    cab.n_columnas
                ),
            });
        }
        let id: NodeId = campos[cab.col_id]
            .parse()
            .map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("id no numérico: '{}'", campos[cab.col_id]),
            })?;
        let labels: Vec<String> = cab
            .col_labels
            .map(|c| {
                campos[c]
                    .split(':')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if labels.is_empty() {
            return Err(ImportError::Semantica {
                linea: stats.lineas,
                causa: "nodo sin :LABEL (al menos una)".into(),
            });
        }
        // Las columnas de props: todas menos :ID y :LABEL (mismo orden que
        // cab.props, que se construyó recorriendo la cabecera igual).
        let ocupadas: Vec<usize> = [Some(cab.col_id), cab.col_labels]
            .into_iter()
            .flatten()
            .collect();
        let mut props = std::collections::HashMap::new();
        let valores_props: Vec<&String> = campos
            .iter()
            .enumerate()
            .filter(|(i, _)| !ocupadas.contains(i))
            .map(|(_, v)| v)
            .collect();
        for ((nombre, tipo), crudo) in cab.props.iter().zip(valores_props) {
            let valor = texto_a_valor(crudo, *tipo).map_err(|e| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("prop '{nombre}': {e}"),
            })?;
            if matches!(&valor, Value::String(s) if s.is_empty()) {
                continue; // campo vacío = prop ausente (no NULL)
            }
            props.insert(nombre.clone(), valor);
        }
        let nodo = Node { id, labels, props };
        autocommit(store, Operacion::PutNode(nodo)).map_err(|_| {
            ImportError::RegistroRechazado {
                linea: stats.lineas,
                causa: StoreError::DuplicateNode(id),
            }
        })?;
        stats.nodos += 1;
    }
}

/// IMPORTA un CSV de aristas (streaming).
///
/// Cabecera: `id:ID, de:START_ID, a:END_ID, tipo:TYPE, props…`. La columna
/// `id:ID` es OPCIONAL (sin ella, los ids se asignan secuenciales a partir
/// de 0). `tipo:TYPE` da el tipo de relación.
pub fn importar_csv_aristas(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    let mut stats = EstadisticasImport::default();

    let mut linea = String::new();
    let n = entrada.read_line(&mut linea)?;
    if n == 0 {
        return Err(ImportError::CabeceraInvalida {
            causa: "fichero vacío (se esperaba la cabecera)".into(),
        });
    }
    stats.lineas += 1;
    let campos = partir_csv(linea.trim_end_matches(['\r', '\n']))
        .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;

    let mut col_id = None;
    let mut col_de = None;
    let mut col_a = None;
    let mut col_tipo = None;
    let mut props: Vec<(String, TipoColumna)> = Vec::new();
    for (i, crudo) in campos.iter().enumerate() {
        let crudo = crudo.trim();
        if let Some((nombre, sufijo)) = crudo.split_once(':') {
            match sufijo.to_ascii_uppercase().as_str() {
                "ID" if !nombre.is_empty() => col_id = Some(i),
                "START_ID" if !nombre.is_empty() => col_de = Some(i),
                "END_ID" if !nombre.is_empty() => col_a = Some(i),
                "TYPE" if !nombre.is_empty() => col_tipo = Some(i),
                _ => {
                    let tipo = TipoColumna::del_sufijo(sufijo)
                        .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
                    props.push((nombre.to_string(), tipo));
                }
            }
        } else {
            props.push((crudo.to_string(), TipoColumna::String));
        }
    }
    let cab = CabeceraAristas {
        col_id,
        col_de: col_de.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna de:START_ID".into(),
        })?,
        col_a: col_a.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna a:END_ID".into(),
        })?,
        col_tipo: col_tipo.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna tipo:TYPE".into(),
        })?,
        props,
        n_columnas: campos.len(),
    };

    let mut siguiente_id: EdgeId = 0;
    loop {
        let mut linea = String::new();
        let n = entrada.read_line(&mut linea)?;
        if n == 0 {
            return Ok(stats);
        }
        stats.lineas += 1;
        let recortada = linea.trim_end_matches(['\r', '\n']);
        if recortada.is_empty() {
            continue;
        }
        let campos = partir_csv(recortada).map_err(|e| ImportError::FilaMalformada {
            linea: stats.lineas,
            causa: e,
        })?;
        if campos.len() != cab.n_columnas {
            return Err(ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!(
                    "la fila tiene {} campos y la cabecera {}",
                    campos.len(),
                    cab.n_columnas
                ),
            });
        }
        let id = match cab.col_id {
            Some(c) => campos[c].parse().map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("id de arista no numérico: '{}'", campos[c]),
            })?,
            None => {
                let id = siguiente_id;
                siguiente_id += 1;
                id
            }
        };
        let de: NodeId = campos[cab.col_de]
            .parse()
            .map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("START_ID no numérico: '{}'", campos[cab.col_de]),
            })?;
        let a: NodeId = campos[cab.col_a]
            .parse()
            .map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("END_ID no numérico: '{}'", campos[cab.col_a]),
            })?;
        let tipo_rel = campos[cab.col_tipo].clone();
        if tipo_rel.is_empty() {
            return Err(ImportError::Semantica {
                linea: stats.lineas,
                causa: "TYPE vacío".into(),
            });
        }
        let mut props = std::collections::HashMap::new();
        let ocupadas: Vec<usize> = [
            cab.col_id,
            Some(cab.col_de),
            Some(cab.col_a),
            Some(cab.col_tipo),
        ]
        .into_iter()
        .flatten()
        .collect();
        let valores_props: Vec<&String> = campos
            .iter()
            .enumerate()
            .filter(|(i, _)| !ocupadas.contains(i))
            .map(|(_, v)| v)
            .collect();
        for ((nombre, tipo), crudo) in cab.props.iter().zip(valores_props) {
            let valor = texto_a_valor(crudo, *tipo).map_err(|e| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("prop '{nombre}': {e}"),
            })?;
            if matches!(&valor, Value::String(s) if s.is_empty()) {
                continue;
            }
            props.insert(nombre.clone(), valor);
        }
        let arista = Edge {
            id,
            source: de,
            target: a,
            label: tipo_rel,
            props,
        };
        autocommit(store, Operacion::PutEdge(arista)).map_err(|_| {
            ImportError::RegistroRechazado {
                linea: stats.lineas,
                causa: if store.get_node(de).is_none() || store.get_node(a).is_none() {
                    StoreError::InvalidEdgeEndpoints {
                        source: de,
                        target: a,
                    }
                } else {
                    StoreError::DuplicateEdge(id)
                },
            }
        })?;
        stats.aristas += 1;
    }
}

/// Un texto CSV al [`Value`] según el tipo de su columna.
fn texto_a_valor(crudo: &str, tipo: TipoColumna) -> Result<Value, String> {
    // Campo vacío: lo representamos como String("") para que el llamador lo
    // descarte (semántica "prop ausente", no NULL). NO aplicamos tipo aquí:
    // si la columna dice INT pero la fila está vacía, NO es un error de tipo.
    if crudo.is_empty() {
        return Ok(Value::String(String::new()));
    }
    match tipo {
        TipoColumna::String => Ok(Value::String(crudo.to_string())),
        TipoColumna::Int => crudo
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| format!("'{crudo}' no es INT")),
        TipoColumna::Float => crudo
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("'{crudo}' no es FLOAT")),
        TipoColumna::Bool => match crudo {
            "true" | "1" => Ok(Value::Bool(true)),
            "false" | "0" => Ok(Value::Bool(false)),
            otro => Err(format!("'{otro}' no es BOOL (true/false)")),
        },
    }
}

/// EXPORTA los nodos a CSV (streaming por filas; cabecera en dos pasadas).
///
/// La cabecera es la UNIÓN de las props de todos los nodos (ordenadas por
/// nombre — determinista), con el sufijo de tipo de la PRIMERA aparición.
/// `Value::Bytes` no tiene representación CSV: se omite (JSONL es el
/// formato sin pérdida). Campo vacío = prop ausente en ese nodo.
pub fn exportar_csv_nodos(
    store: &dyn GraphStore,
    salida: &mut dyn Write,
) -> Result<(), ImportError> {
    // Pasada 1 (sólo cabecera): unión de props — O(#props) de memoria.
    let mut columnas: BTreeMap<String, TipoColumna> = BTreeMap::new();
    for nodo in store.iter_nodes() {
        for (nombre, valor) in &nodo.props {
            columnas
                .entry(nombre.clone())
                .or_insert(tipo_de_valor(valor));
        }
    }
    // Pasada 2: cabecera + filas.
    let mut cabecera = String::from("id:ID");
    for (nombre, tipo) in &columnas {
        cabecera.push_str(&format!(", {nombre}:{}", tipo.sufijo()));
    }
    cabecera.push_str(", :LABEL");
    writeln!(salida, "{cabecera}").map_err(|e| ImportError::Io(e.to_string()))?;
    for nodo in store.iter_nodes() {
        let mut fila = formato_csv(&nodo.id.to_string());
        for nombre in columnas.keys() {
            let campo = match nodo.props.get(nombre) {
                Some(v) => formato_valor_csv(v),
                None => String::new(),
            };
            fila.push(',');
            fila.push_str(&campo);
        }
        let labels = nodo.labels.join(":");
        fila.push(',');
        fila.push_str(&formato_csv(&labels));
        writeln!(salida, "{fila}").map_err(|e| ImportError::Io(e.to_string()))?;
    }
    Ok(())
}

/// EXPORTA las aristas a CSV (mismo contrato que [`exportar_csv_nodos`]).
pub fn exportar_csv_aristas(
    store: &dyn GraphStore,
    salida: &mut dyn Write,
) -> Result<(), ImportError> {
    let mut columnas: BTreeMap<String, TipoColumna> = BTreeMap::new();
    for arista in store.iter_edges() {
        for (nombre, valor) in &arista.props {
            columnas
                .entry(nombre.clone())
                .or_insert(tipo_de_valor(valor));
        }
    }
    let mut cabecera = String::from("id:ID, de:START_ID, a:END_ID, tipo:TYPE");
    for (nombre, tipo) in &columnas {
        cabecera.push_str(&format!(", {nombre}:{}", tipo.sufijo()));
    }
    writeln!(salida, "{cabecera}").map_err(|e| ImportError::Io(e.to_string()))?;
    for arista in store.iter_edges() {
        let mut fila = format!(
            "{},{},{},{}",
            arista.id,
            arista.source,
            arista.target,
            formato_csv(&arista.label)
        );
        for nombre in columnas.keys() {
            let campo = match arista.props.get(nombre) {
                Some(v) => formato_valor_csv(v),
                None => String::new(),
            };
            fila.push(',');
            fila.push_str(&campo);
        }
        writeln!(salida, "{fila}").map_err(|e| ImportError::Io(e.to_string()))?;
    }
    Ok(())
}

/// El tipo de columna CSV que corresponde a un `Value`.
fn tipo_de_valor(v: &Value) -> TipoColumna {
    match v {
        Value::String(_) => TipoColumna::String,
        Value::Int(_) => TipoColumna::Int,
        Value::Float(_) => TipoColumna::Float,
        Value::Bool(_) => TipoColumna::Bool,
        Value::Null => TipoColumna::String, // null se exporta vacío
        Value::Bytes(_) => TipoColumna::String,
    }
}

/// Un `Value` a texto CSV (entre comillas si lleva coma o comilla).
fn formato_valor_csv(v: &Value) -> String {
    let plano = match v {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Bytes(_) => String::new(), // sin pérdida → JSONL
    };
    formato_csv(&plano)
}

/// Escapa un texto CSV: comillas dobles si contiene coma/comilla/espacio
/// en los bordes; dentro, `"` se dobla (RFC 4180).
fn formato_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.starts_with(' ') || s.ends_with(' ') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ─────────────────── JSON (mínimo, a mano) ───────────────────
//
// El subconjunto práctico: objetos {…}, arrays […], strings con escapes
// estándar (\" \\ \/ \b \f \n \r \t \uXXXX — BMP, sin pares sustitutos),
// números (i64 exacto; si no, f64), true/false/null. Espacios/tabs/\r\n
// entre tokens. Es la lección del lexer del cap. 18, ahora para datos.

/// Un valor JSON ya parseado (árbol por LÍNEA: memoria O(línea)).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValor {
    Objeto(Vec<(String, JsonValor)>),
    Array(Vec<JsonValor>),
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl JsonValor {
    /// Acceso a clave de objeto.
    pub fn get(&self, clave: &str) -> Option<&JsonValor> {
        match self {
            JsonValor::Objeto(pares) => pares.iter().find(|(k, _)| k == clave).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A texto si soy String.
    fn como_texto(&self) -> Option<&str> {
        match self {
            JsonValor::String(s) => Some(s),
            _ => None,
        }
    }

    /// A entero si soy Int.
    fn como_int(&self) -> Option<i64> {
        match self {
            JsonValor::Int(i) => Some(*i),
            _ => None,
        }
    }
}

/// Parsea un documento JSON completo (una línea JSONL, típicamente).
pub fn parsear_json(texto: &str) -> Result<JsonValor, String> {
    let mut p = ParserJson {
        chars: texto.chars().collect(),
        pos: 0,
    };
    let v = p.valor()?;
    p.saltando_espacios();
    if p.pos != p.chars.len() {
        return Err(format!("basura tras el valor en la posición {}", p.pos));
    }
    Ok(v)
}

struct ParserJson {
    chars: Vec<char>,
    pos: usize,
}

impl ParserJson {
    fn saltando_espacios(&mut self) {
        while matches!(self.chars.get(self.pos), Some(' ' | '\t' | '\r' | '\n')) {
            self.pos += 1;
        }
    }

    fn valor(&mut self) -> Result<JsonValor, String> {
        self.saltando_espacios();
        match self.chars.get(self.pos) {
            Some('{') => self.objeto(),
            Some('[') => self.array(),
            Some('"') => Ok(JsonValor::String(self.string()?)),
            Some('t') => self.literal("true", JsonValor::Bool(true)),
            Some('f') => self.literal("false", JsonValor::Bool(false)),
            Some('n') => self.literal("null", JsonValor::Null),
            Some(c) if c.is_ascii_digit() || *c == '-' => self.numero(),
            Some(c) => Err(format!("carácter inesperado '{c}' en {}", self.pos)),
            None => Err("fin de línea inesperado".into()),
        }
    }

    fn literal(&mut self, esp: &str, valor: JsonValor) -> Result<JsonValor, String> {
        for c in esp.chars() {
            if self.chars.get(self.pos) != Some(&c) {
                return Err(format!("literal inválido (se esperaba '{esp}')"));
            }
            self.pos += 1;
        }
        Ok(valor)
    }

    fn objeto(&mut self) -> Result<JsonValor, String> {
        self.pos += 1; // '{'
        let mut pares = Vec::new();
        self.saltando_espacios();
        if self.chars.get(self.pos) == Some(&'}') {
            self.pos += 1;
            return Ok(JsonValor::Objeto(pares));
        }
        loop {
            self.saltando_espacios();
            let clave = self.string()?;
            self.saltando_espacios();
            if self.chars.get(self.pos) != Some(&':') {
                return Err(format!("se esperaba ':' tras la clave \"{clave}\""));
            }
            self.pos += 1;
            let valor = self.valor()?;
            pares.push((clave, valor));
            self.saltando_espacios();
            match self.chars.get(self.pos) {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(JsonValor::Objeto(pares));
                }
                _ => return Err("se esperaba ',' o '}'".into()),
            }
        }
    }

    fn array(&mut self) -> Result<JsonValor, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.saltando_espacios();
        if self.chars.get(self.pos) == Some(&']') {
            self.pos += 1;
            return Ok(JsonValor::Array(items));
        }
        loop {
            items.push(self.valor()?);
            self.saltando_espacios();
            match self.chars.get(self.pos) {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(JsonValor::Array(items));
                }
                _ => return Err("se esperaba ',' o ']'".into()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.chars.get(self.pos) != Some(&'"') {
            return Err(format!("se esperaba '\"' en la posición {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while let Some(c) = self.chars.get(self.pos) {
            match c {
                '"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                '\\' => {
                    self.pos += 1;
                    match self.chars.get(self.pos) {
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some('b') => out.push('\u{0008}'),
                        Some('f') => out.push('\u{000C}'),
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('u') => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                self.pos += 1;
                                hex.push(
                                    self.chars
                                        .get(self.pos)
                                        .copied()
                                        .ok_or("\\uXXXX incompleto")?,
                                );
                            }
                            let cp = u32::from_str_radix(&hex, 16)
                                .map_err(|_| "\\uXXXX inválido".to_string())?;
                            // Pares sustitutos: fuera del subconjunto (documentado).
                            let cp = if (0xD800..0xDC00).contains(&cp) {
                                return Err("par sustituto \\uD800-\\uDBFF no soportado".into());
                            } else {
                                cp
                            };
                            out.push(char::from_u32(cp).ok_or("\\uXXXX no es carácter")?);
                        }
                        Some(c) => return Err(format!("escape inválido '\\{c}'")),
                        None => return Err("escape al final del string".into()),
                    }
                    self.pos += 1;
                }
                otro => {
                    out.push(*otro);
                    self.pos += 1;
                }
            }
        }
        Err("string sin cerrar".into())
    }

    fn numero(&mut self) -> Result<JsonValor, String> {
        let inicio = self.pos;
        if self.chars.get(self.pos) == Some(&'-') {
            self.pos += 1;
        }
        while matches!(self.chars.get(self.pos), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut es_float = false;
        if self.chars.get(self.pos) == Some(&'.') {
            es_float = true;
            self.pos += 1;
            while matches!(self.chars.get(self.pos), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.chars.get(self.pos), Some('e' | 'E')) {
            es_float = true;
            self.pos += 1;
            if matches!(self.chars.get(self.pos), Some('+' | '-')) {
                self.pos += 1;
            }
            while matches!(self.chars.get(self.pos), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let texto: String = self.chars[inicio..self.pos].iter().collect();
        if !es_float && let Ok(i) = texto.parse::<i64>() {
            return Ok(JsonValor::Int(i));
        }
        texto
            .parse::<f64>()
            .map(JsonValor::Float)
            .map_err(|_| format!("número inválido '{texto}'"))
    }
}

/// Un `JsonValor` de prop al `Value` del cap. 7 (sólo escalares + el array
/// de ints 0-255 que mapea a Bytes para el roundtrip sin pérdida).
fn json_a_valor(v: &JsonValor, camino: &str) -> Result<Value, String> {
    match v {
        JsonValor::String(s) => Ok(Value::String(s.clone())),
        JsonValor::Int(i) => Ok(Value::Int(*i)),
        JsonValor::Float(f) => Ok(Value::Float(*f)),
        JsonValor::Bool(b) => Ok(Value::Bool(*b)),
        JsonValor::Null => Ok(Value::Null),
        JsonValor::Array(items) => {
            // Bytes: array de ints en 0..=255 (el convenio de exportación).
            let mut bytes = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    JsonValor::Int(i) if (0..=255).contains(i) => bytes.push(*i as u8),
                    _ => {
                        return Err(format!(
                            "prop '{camino}': array no soportado como propiedad \
                             (sólo arrays de bytes 0-255)"
                        ));
                    }
                }
            }
            Ok(Value::Bytes(bytes))
        }
        JsonValor::Objeto(_) => Err(format!(
            "prop '{camino}': objeto anidado no soportado como propiedad"
        )),
    }
}

/// IMPORTA un fichero JSONL (streaming): nodos y aristas mezclados, un
/// registro por línea con discriminador `"tipo"`.
pub fn importar_jsonl(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    let mut stats = EstadisticasImport::default();
    loop {
        let mut linea = String::new();
        let n = entrada.read_line(&mut linea)?;
        if n == 0 {
            return Ok(stats);
        }
        stats.lineas += 1;
        let recortada = linea.trim_end_matches(['\r', '\n']);
        if recortada.trim().is_empty() {
            continue;
        }
        let doc = parsear_json(recortada).map_err(|e| ImportError::FilaMalformada {
            linea: stats.lineas,
            causa: e,
        })?;
        let tipo = doc
            .get("tipo")
            .and_then(JsonValor::como_texto)
            .ok_or_else(|| ImportError::Semantica {
                linea: stats.lineas,
                causa: "falta \"tipo\":\"nodo\"|\"arista\"".into(),
            })?;
        match tipo {
            "nodo" => {
                let id = doc.get("id").and_then(JsonValor::como_int).ok_or_else(|| {
                    ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "\"id\" numérico obligatorio".into(),
                    }
                })?;
                let labels: Vec<String> = match doc.get("labels") {
                    Some(JsonValor::Array(items)) => items
                        .iter()
                        .map(|l| {
                            l.como_texto().map(str::to_string).ok_or_else(|| {
                                ImportError::Semantica {
                                    linea: stats.lineas,
                                    causa: "labels debe ser array de strings".into(),
                                }
                            })
                        })
                        .collect::<Result<_, _>>()?,
                    _ => {
                        return Err(ImportError::Semantica {
                            linea: stats.lineas,
                            causa: "\"labels\" (array) obligatorio".into(),
                        });
                    }
                };
                if labels.is_empty() {
                    return Err(ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "nodo sin labels".into(),
                    });
                }
                let mut props = std::collections::HashMap::new();
                if let Some(JsonValor::Objeto(pares)) = doc.get("props") {
                    for (k, v) in pares {
                        let valor = json_a_valor(v, k).map_err(|e| ImportError::Semantica {
                            linea: stats.lineas,
                            causa: e,
                        })?;
                        props.insert(k.clone(), valor);
                    }
                }
                let nodo = Node {
                    id: id as NodeId,
                    labels,
                    props,
                };
                let id_nodo = nodo.id;
                autocommit(store, Operacion::PutNode(nodo)).map_err(|_| {
                    ImportError::RegistroRechazado {
                        linea: stats.lineas,
                        causa: StoreError::DuplicateNode(id_nodo),
                    }
                })?;
                stats.nodos += 1;
            }
            "arista" => {
                let id = doc.get("id").and_then(JsonValor::como_int).ok_or_else(|| {
                    ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "\"id\" numérico obligatorio".into(),
                    }
                })?;
                let de = doc.get("de").and_then(JsonValor::como_int).ok_or_else(|| {
                    ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "\"de\" numérico obligatorio".into(),
                    }
                })?;
                let a = doc.get("a").and_then(JsonValor::como_int).ok_or_else(|| {
                    ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "\"a\" numérico obligatorio".into(),
                    }
                })?;
                let rel = doc
                    .get("rel")
                    .and_then(JsonValor::como_texto)
                    .ok_or_else(|| ImportError::Semantica {
                        linea: stats.lineas,
                        causa: "\"rel\" (string) obligatorio".into(),
                    })?;
                let mut props = std::collections::HashMap::new();
                if let Some(JsonValor::Objeto(pares)) = doc.get("props") {
                    for (k, v) in pares {
                        let valor = json_a_valor(v, k).map_err(|e| ImportError::Semantica {
                            linea: stats.lineas,
                            causa: e,
                        })?;
                        props.insert(k.clone(), valor);
                    }
                }
                let arista = Edge {
                    id: id as EdgeId,
                    source: de as NodeId,
                    target: a as NodeId,
                    label: rel.to_string(),
                    props,
                };
                let (eid, s, t) = (arista.id, arista.source, arista.target);
                autocommit(store, Operacion::PutEdge(arista)).map_err(|_| {
                    ImportError::RegistroRechazado {
                        linea: stats.lineas,
                        causa: if store.get_node(s).is_none() || store.get_node(t).is_none() {
                            StoreError::InvalidEdgeEndpoints {
                                source: s,
                                target: t,
                            }
                        } else {
                            StoreError::DuplicateEdge(eid)
                        },
                    }
                })?;
                stats.aristas += 1;
            }
            otro => {
                return Err(ImportError::Semantica {
                    linea: stats.lineas,
                    causa: format!("\"tipo\" desconocido: '{otro}' (nodo|arista)"),
                });
            }
        }
    }
}

/// EXPORTA el grafo a JSONL (streaming: primero los nodos, luego las
/// aristas, una línea por elemento). Es el formato SIN PÉRDIDA: hasta los
/// `Value::Bytes` viajan como array de ints y vuelven como Bytes.
pub fn exportar_jsonl(store: &dyn GraphStore, salida: &mut dyn Write) -> Result<(), ImportError> {
    for nodo in store.iter_nodes() {
        let mut props = String::from("{");
        for (i, (k, v)) in nodo.props.iter().enumerate() {
            if i > 0 {
                props.push(',');
            }
            props.push_str(&format!(
                "{}:{}",
                serializar_json_texto(k),
                serializar_json_valor(v)
            ));
        }
        props.push('}');
        let labels: Vec<String> = nodo
            .labels
            .iter()
            .map(|l| serializar_json_texto(l))
            .collect();
        writeln!(
            salida,
            "{{\"tipo\":\"nodo\",\"id\":{},\"labels\":[{}],\"props\":{}}}",
            nodo.id,
            labels.join(","),
            props
        )
        .map_err(|e| ImportError::Io(e.to_string()))?;
    }
    for arista in store.iter_edges() {
        let mut props = String::from("{");
        for (i, (k, v)) in arista.props.iter().enumerate() {
            if i > 0 {
                props.push(',');
            }
            props.push_str(&format!(
                "{}:{}",
                serializar_json_texto(k),
                serializar_json_valor(v)
            ));
        }
        props.push('}');
        writeln!(
            salida,
            "{{\"tipo\":\"arista\",\"id\":{},\"de\":{},\"a\":{},\"rel\":{},\"props\":{}}}",
            arista.id,
            arista.source,
            arista.target,
            serializar_json_texto(&arista.label),
            props
        )
        .map_err(|e| ImportError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Serializa un `Value` a JSON (los props de una línea JSONL).
fn serializar_json_valor(v: &Value) -> String {
    match v {
        Value::String(s) => serializar_json_texto(s),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            if f.is_finite() {
                f.to_string()
            } else {
                "null".to_string() // JSON no tiene NaN/∞: sin pérdida es para Bytes, no floats
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Bytes(bs) => {
            let ints: Vec<String> = bs.iter().map(|b| b.to_string()).collect();
            format!("[{}]", ints.join(","))
        }
    }
}

/// Escapa un string JSON (comillas, barra, control como \n… y no-ASCII
/// como \uXXXX para mantener la línea ASCII-segura).
fn serializar_json_texto(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) > 0x7E => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ─────────────────── GraphML (XML mínimo, a mano) ───────────────────
//
// El subconjunto GraphML que yEd/Gephi/NetworkX escriben y leen:
//   <?xml …?> (se ignora)  <graphml xmlns=…> (atributos ignorados)
//   <key id="d0" for="node|edge" attr.name="nombre" attr.type="…"/>
//   <graph edgedefault="directed">
//     <node id="n0"><data key="d0">texto</data></node>
//     <edge id="e0" source="n0" target="n1" [tipo="KNOWS"]><data…/></edge>
// El TIPO de relación no es GraphML estándar: usamos el atributo `tipo`
// propio (exportado e importado) porque GraphML no lo tiene — documentado.

/// Un evento del mini-parser XML.
#[derive(Debug, Clone, PartialEq)]
enum EventoXml {
    /// Etiqueta de apertura con sus atributos (vacía=true si `<x/>`).
    Apertura {
        etiqueta: String,
        attrs: Vec<(String, String)>,
        vacia: bool,
    },
    /// Etiqueta de cierre.
    Cierre { etiqueta: String },
    /// Texto entre etiquetas (ya con entidades decodificadas).
    Texto(String),
}

/// Tokeniza XML por eventos (sin namespaces, sin CDATA; `<!-- -->` y
/// `<?…?>` se saltan). Es el lexer del cap. 18, tercera encarnación.
fn eventos_xml(texto: &str) -> Result<Vec<EventoXml>, String> {
    let chars: Vec<char> = texto.chars().collect();
    let mut eventos = Vec::new();
    let mut i = 0usize;
    let mut texto_actual = String::new();
    while i < chars.len() {
        if chars[i] == '<' {
            if !texto_actual.trim().is_empty() {
                eventos.push(EventoXml::Texto(decodificar_entidades(
                    texto_actual.trim(),
                )?));
            }
            texto_actual.clear();
            // ¿<!-- … --> o <?xml … ?>?
            if chars.get(i + 1) == Some(&'!') || chars.get(i + 1) == Some(&'?') {
                let cierra = if chars.get(i + 1) == Some(&'?') {
                    "?>"
                } else {
                    "-->"
                };
                let mut j = i + 2;
                loop {
                    if j + 1 < chars.len() && chars[j] == cierra.chars().next().unwrap() {
                        // comprobación barata del delimitador completo
                        let restante: String = chars[j..(j + 2).min(chars.len())].iter().collect();
                        if restante.starts_with(cierra.chars().next().unwrap())
                            && delimitador_en(&chars, j, cierra)
                        {
                            i = j + cierre_len(cierra);
                            break;
                        }
                    }
                    j += 1;
                    if j >= chars.len() {
                        return Err("comentario/instrucción sin cerrar".into());
                    }
                }
                continue;
            }
            // Etiqueta real.
            let cierre = chars.get(i + 1) == Some(&'/');
            let inicio_contenido = if cierre { i + 2 } else { i + 1 };
            let mut j = inicio_contenido;
            while j < chars.len() && chars[j] != '>' {
                j += 1;
            }
            if j >= chars.len() {
                return Err("etiqueta sin '>' de cierre".into());
            }
            let contenido: String = chars[inicio_contenido..j].iter().collect();
            let vacia = contenido.ends_with('/');
            let contenido = contenido.trim_end_matches('/').trim().to_string();
            let etiqueta = contenido
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if etiqueta.is_empty() {
                return Err("etiqueta sin nombre".into());
            }
            // Atributos clave="valor" (las comillas, obligatorias aquí).
            let mut attrs = Vec::new();
            let cuerpo = contenido[etiqueta.len()..].trim();
            let cs: Vec<char> = cuerpo.chars().collect();
            let mut k = 0usize;
            while k < cs.len() {
                while k < cs.len() && cs[k].is_whitespace() {
                    k += 1;
                }
                let ini = k;
                while k < cs.len() && cs[k] != '=' {
                    k += 1;
                }
                if k >= cs.len() {
                    break;
                }
                let clave: String = cs[ini..k].iter().collect();
                k += 1; // '='
                if cs.get(k) != Some(&'"') {
                    return Err(format!("atributo {clave} sin comillas"));
                }
                k += 1;
                let vinicio = k;
                while k < cs.len() && cs[k] != '"' {
                    k += 1;
                }
                let valor: String = cs[vinicio..k].iter().collect();
                k += 1;
                attrs.push((clave, decodificar_entidades(&valor)?));
            }
            if cierre {
                eventos.push(EventoXml::Cierre { etiqueta });
            } else {
                eventos.push(EventoXml::Apertura {
                    etiqueta,
                    attrs,
                    vacia,
                });
            }
            i = j + 1;
        } else {
            texto_actual.push(chars[i]);
            i += 1;
        }
    }
    // Texto tras la última etiqueta (sin '<' no se flushea arriba).
    if !texto_actual.trim().is_empty() {
        eventos.push(EventoXml::Texto(decodificar_entidades(
            texto_actual.trim(),
        )?));
    }
    Ok(eventos)
}

/// ¿Está el delimitador `delim` en `chars[pos..]`?
fn delimitador_en(chars: &[char], pos: usize, delim: &str) -> bool {
    let dc: Vec<char> = delim.chars().collect();
    if pos + dc.len() > chars.len() {
        return false;
    }
    chars[pos..pos + dc.len()] == dc[..]
}

fn cierre_len(delim: &str) -> usize {
    delim.len()
}

/// Las cinco entidades XML básicas.
fn decodificar_entidades(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '&' {
            let mut j = i + 1;
            while j < cs.len() && cs[j] != ';' {
                j += 1;
            }
            if j >= cs.len() {
                return Err("entidad '&' sin ';'".into());
            }
            let entidad: String = cs[i + 1..j].iter().collect();
            out.push(match entidad.as_str() {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                otra => return Err(format!("entidad desconocida '&{otra};'")),
            });
            i = j + 1;
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// IMPORTA un GraphML (el fichero entero en texto: GraphML exige conocer
/// las `<key>` ANTES de los datos, así que se procesa por eventos tras una
/// única lectura; la memoria es O(fichero) — la excepción documentada del
/// streaming del capítulo, y por eso JSONL/CSV son los formatos MASIVOS).
pub fn importar_graphml(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    let mut texto = String::new();
    entrada.read_to_string(&mut texto)?;
    let eventos =
        eventos_xml(&texto).map_err(|e| ImportError::FilaMalformada { linea: 1, causa: e })?;
    stats_import_graphml(eventos, store)
}

/// El nucleo de la importación GraphML: primero las `<key>`, luego nodos
/// y aristas resolviendo ids STRING → NodeId denso (la lección del cap. 3).
fn stats_import_graphml(
    eventos: Vec<EventoXml>,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    let mut stats = EstadisticasImport::default();
    // id de key → (nombre, tipo textual); por `for` (node|edge).
    let mut claves_nodo: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut claves_arista: BTreeMap<String, (String, String)> = BTreeMap::new();
    // id EXTERNO (string) → NodeId denso interno, por orden de aparición.
    let mut mapa_ids: BTreeMap<String, NodeId> = BTreeMap::new();

    let mut i = 0usize;
    // Fase 1: recolectar <key> (pueden ir mezcladas; GraphML las pone al
    // principio — exigimos eso: tras el primer <node>, error si llega key).
    while i < eventos.len() {
        match &eventos[i] {
            EventoXml::Apertura {
                etiqueta, attrs, ..
            } if etiqueta == "key" => {
                let id = attr(attrs, "id").ok_or_else(|| ImportError::Semantica {
                    linea: 1,
                    causa: "<key> sin id".into(),
                })?;
                let para = attr(attrs, "for").unwrap_or_else(|| "node".to_string());
                let nombre = attr(attrs, "attr.name").unwrap_or(id.clone());
                let tipo = attr(attrs, "attr.type").unwrap_or_else(|| "string".to_string());
                match para.as_str() {
                    "node" => {
                        claves_nodo.insert(id.clone(), (nombre, tipo));
                    }
                    "edge" => {
                        claves_arista.insert(id.clone(), (nombre, tipo));
                    }
                    otro => {
                        return Err(ImportError::Semantica {
                            linea: 1,
                            causa: format!("<key for=\"{otro}\"> (node|edge)"),
                        });
                    }
                }
                i += 1;
            }
            EventoXml::Apertura { etiqueta, .. } if etiqueta == "node" => break,
            _ => i += 1,
        }
    }
    // Fase 2: nodos y aristas.
    let mut siguiente_edge_id: EdgeId = 0;
    while i < eventos.len() {
        let (etiqueta, attrs, vacia) = match &eventos[i] {
            EventoXml::Apertura {
                etiqueta,
                attrs,
                vacia,
            } => (etiqueta.clone(), attrs.clone(), *vacia),
            _ => {
                i += 1;
                continue;
            }
        };
        match etiqueta.as_str() {
            "node" => {
                stats.lineas += 1;
                let id_ext = attr(&attrs, "id").ok_or_else(|| ImportError::Semantica {
                    linea: stats.lineas,
                    causa: "<node> sin id".into(),
                })?;
                // Ids densos por orden de aparición (cap. 3: la identidad
                // EXTERNA se mapea a la INTERNA, no se inventa).
                let interno = mapa_ids.len();
                mapa_ids.insert(id_ext.clone(), interno);
                // <data> hasta </node>.
                let mut props = std::collections::HashMap::new();
                let mut labels = Vec::new();
                if !vacia {
                    let mut j = i + 1;
                    while j < eventos.len() {
                        match &eventos[j] {
                            EventoXml::Cierre { etiqueta } if etiqueta == "node" => break,
                            EventoXml::Apertura {
                                etiqueta, attrs, ..
                            } if etiqueta == "data" => {
                                let clave = attr(attrs, "key").unwrap_or_default();
                                // El texto va en el siguiente evento.
                                let valor = match eventos.get(j + 1) {
                                    Some(EventoXml::Texto(t)) => t.clone(),
                                    _ => String::new(),
                                };
                                if clave == "dL" {
                                    // Convención propia (dL): labels ":"-separadas.
                                    labels = valor
                                        .split(':')
                                        .filter(|s| !s.is_empty())
                                        .map(str::to_string)
                                        .collect();
                                } else if let Some((nombre, tipo)) = claves_nodo.get(&clave) {
                                    props.insert(
                                        nombre.clone(),
                                        texto_graphml_a_valor(&valor, tipo).map_err(|e| {
                                            ImportError::Semantica {
                                                linea: stats.lineas,
                                                causa: e,
                                            }
                                        })?,
                                    );
                                }
                                j += 2;
                            }
                            _ => j += 1,
                        }
                    }
                    i = j;
                }
                if labels.is_empty() {
                    labels.push("Nodo".to_string()); // GraphML no tiene labels: mínima
                }
                let nodo = Node {
                    id: mapa_ids[&id_ext],
                    labels,
                    props,
                };
                let nid = nodo.id;
                autocommit(store, Operacion::PutNode(nodo)).map_err(|_| {
                    ImportError::RegistroRechazado {
                        linea: stats.lineas,
                        causa: StoreError::DuplicateNode(nid),
                    }
                })?;
                stats.nodos += 1;
                i += 1;
            }
            "edge" => {
                stats.lineas += 1;
                let src = attr(&attrs, "source").ok_or_else(|| ImportError::Semantica {
                    linea: stats.lineas,
                    causa: "<edge> sin source".into(),
                })?;
                let tgt = attr(&attrs, "target").ok_or_else(|| ImportError::Semantica {
                    linea: stats.lineas,
                    causa: "<edge> sin target".into(),
                })?;
                let de = *mapa_ids.get(&src).ok_or_else(|| ImportError::Semantica {
                    linea: stats.lineas,
                    causa: format!("source '{src}' no declarado como <node>"),
                })?;
                let a = *mapa_ids.get(&tgt).ok_or_else(|| ImportError::Semantica {
                    linea: stats.lineas,
                    causa: format!("target '{tgt}' no declarado como <node>"),
                })?;
                let id: EdgeId = match attr(&attrs, "id").and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        let id = siguiente_edge_id;
                        siguiente_edge_id += 1;
                        id
                    }
                };
                // El tipo de relación: atributo propio `tipo` (GraphML no
                // lo estandariza — documentado) o data key="TYPE".
                let mut label = attr(&attrs, "tipo").unwrap_or_else(|| "REL".to_string());
                let mut props = std::collections::HashMap::new();
                if !vacia {
                    let mut j = i + 1;
                    while j < eventos.len() {
                        match &eventos[j] {
                            EventoXml::Cierre { etiqueta } if etiqueta == "edge" => break,
                            EventoXml::Apertura {
                                etiqueta, attrs, ..
                            } if etiqueta == "data" => {
                                let clave = attr(attrs, "key").unwrap_or_default();
                                let valor = match eventos.get(j + 1) {
                                    Some(EventoXml::Texto(t)) => t.clone(),
                                    _ => String::new(),
                                };
                                if clave == "eT" {
                                    if !valor.is_empty() {
                                        label = valor;
                                    }
                                } else if let Some((nombre, tipo)) = claves_arista.get(&clave) {
                                    props.insert(
                                        nombre.clone(),
                                        texto_graphml_a_valor(&valor, tipo).map_err(|e| {
                                            ImportError::Semantica {
                                                linea: stats.lineas,
                                                causa: e,
                                            }
                                        })?,
                                    );
                                }
                                j += 2;
                            }
                            _ => j += 1,
                        }
                    }
                    i = j;
                }
                let arista = Edge {
                    id,
                    source: de,
                    target: a,
                    label,
                    props,
                };
                let (eid, s, t) = (arista.id, arista.source, arista.target);
                autocommit(store, Operacion::PutEdge(arista)).map_err(|_| {
                    ImportError::RegistroRechazado {
                        linea: stats.lineas,
                        causa: if store.get_node(s).is_none() || store.get_node(t).is_none() {
                            StoreError::InvalidEdgeEndpoints {
                                source: s,
                                target: t,
                            }
                        } else {
                            StoreError::DuplicateEdge(eid)
                        },
                    }
                })?;
                stats.aristas += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    Ok(stats)
}

fn attr(attrs: &[(String, String)], nombre: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k == nombre)
        .map(|(_, v)| v.clone())
}

/// Texto + attr.type GraphML → `Value` (boolean/int/long/float/double/string).
fn texto_graphml_a_valor(texto: &str, tipo: &str) -> Result<Value, String> {
    match tipo {
        "boolean" => match texto {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            otro => Err(format!("'{otro}' no es boolean")),
        },
        "int" | "long" => texto
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| format!("'{texto}' no es {tipo}")),
        "float" | "double" => texto
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("'{texto}' no es {tipo}")),
        _ => Ok(Value::String(texto.to_string())),
    }
}

/// El attr.type GraphML que corresponde a un `Value` (para exportar).
fn tipo_graphml_de(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "boolean",
        Value::Int(_) => "long",
        Value::Float(_) => "double",
        _ => "string",
    }
}

/// EXPORTA el grafo a GraphML (streaming por elementos; las `<key>` salen
/// de la unión de props como en CSV). Los ids internos se escriben como
/// strings y el roundtrip los re-densifica.
pub fn exportar_graphml(store: &dyn GraphStore, salida: &mut dyn Write) -> Result<(), ImportError> {
    // Uniones de props (dos pasadas de CABECERA, no de filas).
    let mut props_nodo: BTreeMap<String, &'static str> = BTreeMap::new();
    for n in store.iter_nodes() {
        for (k, v) in &n.props {
            props_nodo.entry(k.clone()).or_insert(tipo_graphml_de(v));
        }
    }
    let mut props_arista: BTreeMap<String, &'static str> = BTreeMap::new();
    for e in store.iter_edges() {
        for (k, v) in &e.props {
            props_arista.entry(k.clone()).or_insert(tipo_graphml_de(v));
        }
    }

    let esc = escapar_xml;
    writeln!(
        salida,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<graphml \
         xmlns=\"http://graphml.graphdrawing.org/xmlns\">"
    )
    .map_err(io_err)?;
    // Claves: dN… para props de nodo, eN… para arista; más las dos
    // convenciones propias (LABELS/TYPE) documentadas en el módulo.
    writeln!(
        salida,
        "  <key id=\"dL\" for=\"node\" attr.name=\"labels\" attr.type=\"string\"/>"
    )
    .map_err(io_err)?;
    let mut id_prop_nodo: BTreeMap<String, String> = BTreeMap::new();
    for (i, (nombre, tipo)) in props_nodo.iter().enumerate() {
        let id = format!("d{i}");
        writeln!(
            salida,
            "  <key id=\"{id}\" for=\"node\" attr.name=\"{}\" attr.type=\"{tipo}\"/>",
            esc(nombre)
        )
        .map_err(io_err)?;
        id_prop_nodo.insert(nombre.clone(), id);
    }
    writeln!(
        salida,
        "  <key id=\"eT\" for=\"edge\" attr.name=\"tipo\" attr.type=\"string\"/>"
    )
    .map_err(io_err)?;
    let mut id_prop_arista: BTreeMap<String, String> = BTreeMap::new();
    for (i, (nombre, tipo)) in props_arista.iter().enumerate() {
        let id = format!("e{i}");
        writeln!(
            salida,
            "  <key id=\"{id}\" for=\"edge\" attr.name=\"{}\" attr.type=\"{tipo}\"/>",
            esc(nombre)
        )
        .map_err(io_err)?;
        id_prop_arista.insert(nombre.clone(), id);
    }
    writeln!(salida, "  <graph edgedefault=\"directed\">").map_err(io_err)?;
    for n in store.iter_nodes() {
        writeln!(salida, "    <node id=\"{}\">", n.id).map_err(io_err)?;
        writeln!(
            salida,
            "      <data key=\"dL\">{}</data>",
            esc(&n.labels.join(":"))
        )
        .map_err(io_err)?;
        for (nombre, valor) in &n.props {
            if let Value::Bytes(_) = valor {
                continue; // GraphML sin bytes: JSONL es el sin-pérdida
            }
            let id = &id_prop_nodo[nombre];
            writeln!(
                salida,
                "      <data key=\"{id}\">{}</data>",
                esc(&texto_graphml(valor))
            )
            .map_err(io_err)?;
        }
        writeln!(salida, "    </node>").map_err(io_err)?;
    }
    for e in store.iter_edges() {
        writeln!(
            salida,
            "    <edge id=\"{}\" source=\"{}\" target=\"{}\">",
            e.id, e.source, e.target
        )
        .map_err(io_err)?;
        writeln!(salida, "      <data key=\"eT\">{}</data>", esc(&e.label)).map_err(io_err)?;
        for (nombre, valor) in &e.props {
            if let Value::Bytes(_) = valor {
                continue;
            }
            let id = &id_prop_arista[nombre];
            writeln!(
                salida,
                "      <data key=\"{id}\">{}</data>",
                esc(&texto_graphml(valor))
            )
            .map_err(io_err)?;
        }
        writeln!(salida, "    </edge>").map_err(io_err)?;
    }
    writeln!(salida, "  </graph>\n</graphml>").map_err(io_err)?;
    Ok(())
}

/// Un `Value` a texto de `<data>` GraphML.
fn texto_graphml(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Bytes(_) => String::new(),
    }
}

/// Escapa las cinco entidades XML.
fn escapar_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn io_err(e: std::io::Error) -> ImportError {
    ImportError::Io(e.to_string())
}

// ─────────────────────────────────────────────────────────────────
// Auto-detección: CSV con dos secciones (nodos + aristas)
// ─────────────────────────────────────────────────────────────────

/// IMPORTA un CSV con DOS secciones consecutivas separadas por una línea
/// `# aristas`. La primera sección es la cabecera de nodos; la segunda,
/// la de aristas. Devuelve la suma de estadísticas.
///
/// Esto es lo que produce [`exportar_csv`] (nodos, línea `# aristas`,
/// aristas) y permite roundtrip sin pelearse con dos llamadas separadas.
pub fn importar_csv_unico(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
) -> Result<EstadisticasImport, ImportError> {
    // ── Sección 1: nodos ─────────────────────────────────────────
    let mut cabecera = String::new();
    let n = entrada.read_line(&mut cabecera)?;
    if n == 0 {
        return Err(ImportError::CabeceraInvalida {
            causa: "fichero CSV vacío (se esperaba la cabecera)".into(),
        });
    }
    let rec = cabecera.trim_end_matches(['\r', '\n']).to_string();
    let cab_nodos = parsear_cabecera_nodos(&rec)?;
    let mut stats = EstadisticasImport::default();
    stats.lineas += 1;

    loop {
        let mut linea = String::new();
        let leidos = entrada.read_line(&mut linea)?;
        if leidos == 0 {
            break;
        }
        let recortada = linea.trim_end_matches(['\r', '\n']);
        if recortada.is_empty() {
            continue;
        }
        // Separador: terminamos la sección de nodos.
        if recortada.trim_start().starts_with('#') {
            // Consumimos el separador y leemos la cabecera de aristas.
            let mut cab2 = String::new();
            let n2 = entrada.read_line(&mut cab2)?;
            if n2 == 0 {
                return Ok(stats);
            }
            let rec2 = cab2.trim_end_matches(['\r', '\n']);
            if rec2.is_empty() {
                continue;
            }
            let cab_aristas = parsear_cabecera_aristas(rec2)?;
            stats.lineas += 1;
            let s_aristas = importar_aristas_desde(entrada, store, cab_aristas)?;
            stats.aristas = s_aristas.aristas;
            stats.lineas += s_aristas.lineas;
            return Ok(stats);
        }
        stats.lineas += 1;
        let campos = partir_csv(recortada).map_err(|e| ImportError::FilaMalformada {
            linea: stats.lineas,
            causa: e,
        })?;
        if campos.len() != cab_nodos.n_columnas {
            return Err(ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!(
                    "la fila tiene {} campos y la cabecera {}",
                    campos.len(),
                    cab_nodos.n_columnas
                ),
            });
        }
        let id: NodeId =
            campos[cab_nodos.col_id]
                .parse()
                .map_err(|_| ImportError::FilaMalformada {
                    linea: stats.lineas,
                    causa: format!("id no numérico: '{}'", campos[cab_nodos.col_id]),
                })?;
        let labels: Vec<String> = cab_nodos
            .col_labels
            .map(|c| {
                campos[c]
                    .split(':')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if labels.is_empty() {
            return Err(ImportError::Semantica {
                linea: stats.lineas,
                causa: "nodo sin :LABEL (al menos una)".into(),
            });
        }
        let ocupadas: Vec<usize> = [Some(cab_nodos.col_id), cab_nodos.col_labels]
            .into_iter()
            .flatten()
            .collect();
        let mut props = std::collections::HashMap::new();
        let valores_props: Vec<&String> = campos
            .iter()
            .enumerate()
            .filter(|(i, _)| !ocupadas.contains(i))
            .map(|(_, v)| v)
            .collect();
        for ((nombre, tipo), crudo) in cab_nodos.props.iter().zip(valores_props) {
            let valor = texto_a_valor(crudo, *tipo).map_err(|e| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("prop '{nombre}': {e}"),
            })?;
            if matches!(&valor, Value::String(s) if s.is_empty()) {
                continue;
            }
            props.insert(nombre.clone(), valor);
        }
        let nodo = Node { id, labels, props };
        autocommit(store, Operacion::PutNode(nodo)).map_err(|_| {
            ImportError::RegistroRechazado {
                linea: stats.lineas,
                causa: StoreError::DuplicateNode(id),
            }
        })?;
        stats.nodos += 1;
    }
    // EOF sin sección de aristas: sólo nodos.
    Ok(stats)
}

/// Parsea la cabecera de nodos.
fn parsear_cabecera_nodos(linea: &str) -> Result<CabeceraNodos, ImportError> {
    let campos = partir_csv(linea).map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
    let mut col_id = None;
    let mut col_labels = None;
    let mut props: Vec<(String, TipoColumna)> = Vec::new();
    for (i, crudo) in campos.iter().enumerate() {
        let crudo = crudo.trim();
        if crudo == ":LABEL" {
            col_labels = Some(i);
        } else if let Some((nombre, sufijo)) = crudo.split_once(':') {
            if sufijo.eq_ignore_ascii_case("ID") && !nombre.is_empty() {
                if col_id.is_some() {
                    return Err(ImportError::CabeceraInvalida {
                        causa: "dos columnas :ID".into(),
                    });
                }
                col_id = Some(i);
            } else {
                let tipo = TipoColumna::del_sufijo(sufijo)
                    .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
                props.push((nombre.to_string(), tipo));
            }
        } else {
            props.push((crudo.to_string(), TipoColumna::String));
        }
    }
    Ok(CabeceraNodos {
        col_id: col_id.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna id:ID".into(),
        })?,
        col_labels,
        props,
        n_columnas: campos.len(),
    })
}

/// Parsea la cabecera de aristas.
fn parsear_cabecera_aristas(linea: &str) -> Result<CabeceraAristas, ImportError> {
    let campos = partir_csv(linea).map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
    let mut col_id = None;
    let mut col_de = None;
    let mut col_a = None;
    let mut col_tipo = None;
    let mut props: Vec<(String, TipoColumna)> = Vec::new();
    for (i, crudo) in campos.iter().enumerate() {
        let crudo = crudo.trim();
        if let Some((nombre, sufijo)) = crudo.split_once(':') {
            match sufijo.to_ascii_uppercase().as_str() {
                "ID" if !nombre.is_empty() => col_id = Some(i),
                "START_ID" if !nombre.is_empty() => col_de = Some(i),
                "END_ID" if !nombre.is_empty() => col_a = Some(i),
                "TYPE" if !nombre.is_empty() => col_tipo = Some(i),
                _ => {
                    let tipo = TipoColumna::del_sufijo(sufijo)
                        .map_err(|e| ImportError::CabeceraInvalida { causa: e })?;
                    props.push((nombre.to_string(), tipo));
                }
            }
        } else {
            props.push((crudo.to_string(), TipoColumna::String));
        }
    }
    Ok(CabeceraAristas {
        col_id,
        col_de: col_de.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna de:START_ID".into(),
        })?,
        col_a: col_a.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna a:END_ID".into(),
        })?,
        col_tipo: col_tipo.ok_or(ImportError::CabeceraInvalida {
            causa: "falta la columna tipo:TYPE".into(),
        })?,
        props,
        n_columnas: campos.len(),
    })
}

/// Procesa filas de aristas a partir de una cabecera ya parseada.
fn importar_aristas_desde(
    entrada: &mut dyn BufRead,
    store: &mut dyn GraphStore,
    cab: CabeceraAristas,
) -> Result<EstadisticasImport, ImportError> {
    let mut stats = EstadisticasImport::default();
    let mut siguiente_id: EdgeId = 0;
    loop {
        let mut linea = String::new();
        let n = entrada.read_line(&mut linea)?;
        if n == 0 {
            return Ok(stats);
        }
        let recortada = linea.trim_end_matches(['\r', '\n']);
        if recortada.is_empty() {
            continue;
        }
        stats.lineas += 1;
        let campos = partir_csv(recortada).map_err(|e| ImportError::FilaMalformada {
            linea: stats.lineas,
            causa: e,
        })?;
        if campos.len() != cab.n_columnas {
            return Err(ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!(
                    "la fila tiene {} campos y la cabecera {}",
                    campos.len(),
                    cab.n_columnas
                ),
            });
        }
        let id: EdgeId = match cab.col_id {
            Some(c) => campos[c].parse().map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("id no numérico: '{}'", campos[c]),
            })?,
            None => {
                let id = siguiente_id;
                siguiente_id += 1;
                id
            }
        };
        let de: NodeId = campos[cab.col_de]
            .parse()
            .map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("de no numérico: '{}'", campos[cab.col_de]),
            })?;
        let a: NodeId = campos[cab.col_a]
            .parse()
            .map_err(|_| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("a no numérico: '{}'", campos[cab.col_a]),
            })?;
        let label = campos[cab.col_tipo].to_string();
        let ocupadas: Vec<usize> = [
            cab.col_id,
            Some(cab.col_de),
            Some(cab.col_a),
            Some(cab.col_tipo),
        ]
        .into_iter()
        .flatten()
        .collect();
        let mut props = std::collections::HashMap::new();
        let valores_props: Vec<&String> = campos
            .iter()
            .enumerate()
            .filter(|(i, _)| !ocupadas.contains(i))
            .map(|(_, v)| v)
            .collect();
        for ((nombre, tipo), crudo) in cab.props.iter().zip(valores_props) {
            let valor = texto_a_valor(crudo, *tipo).map_err(|e| ImportError::FilaMalformada {
                linea: stats.lineas,
                causa: format!("prop '{nombre}': {e}"),
            })?;
            if matches!(&valor, Value::String(s) if s.is_empty()) {
                continue;
            }
            props.insert(nombre.clone(), valor);
        }
        let arista = Edge {
            id,
            source: de,
            target: a,
            label,
            props,
        };
        autocommit(store, Operacion::PutEdge(arista)).map_err(|_| {
            ImportError::RegistroRechazado {
                linea: stats.lineas,
                causa: StoreError::UnknownNode(de),
            }
        })?;
        stats.aristas += 1;
    }
}

/// EXPORTA un CSV con dos secciones (nodos + aristas) separadas por la
/// línea `# aristas`. Es la forma canónica de [`importar_csv_unico`].
pub fn exportar_csv(store: &dyn GraphStore, salida: &mut dyn Write) -> Result<(), ImportError> {
    exportar_csv_nodos(store, salida)?;
    writeln!(salida, "# aristas").map_err(|e| ImportError::Io(e.to_string()))?;
    exportar_csv_aristas(store, salida)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_import_export {
    use super::*;
    use crate::cap08_graph_store::MemoryStore;
    use std::io::Cursor;

    /// Un BufRead sobre un texto (la vía de los tests: sin ficheros).
    fn lector(texto: &str) -> Cursor<&[u8]> {
        Cursor::new(texto.as_bytes())
    }

    /// Grafo pequeño: 2 nodos + 1 arista, para roundtrips.
    fn grafo() -> MemoryStore {
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "Person").with_prop("name", Value::String("Zoe".into())))
            .unwrap();
        s.put_node(Node::new(1, "City").with_prop("name", Value::String("Oviedo".into())))
            .unwrap();
        s.put_edge(Edge::new(0, 0, 1, "LIVES_IN").with_prop("since", Value::Int(2019)))
            .unwrap();
        s
    }

    // ── partir_csv ───────────────────────────────────────────────

    #[test]
    fn csv_partir_basico() {
        assert_eq!(
            partir_csv("a,b,c").unwrap(),
            vec!["a".to_string(), "b".into(), "c".into()]
        );
        // Campo vacío al final.
        assert_eq!(partir_csv("a,").unwrap(), vec!["a".into(), String::new()]);
    }

    #[test]
    fn csv_partir_comillas_con_coma_y_escalpe() {
        let f = partir_csv("\"Zoe, Vega\",\"dice \"\"hola\"\"\",36").unwrap();
        assert_eq!(f[0], "Zoe, Vega");
        assert_eq!(f[1], "dice \"hola\"");
        assert_eq!(f[2], "36");
    }

    #[test]
    fn csv_partir_errores() {
        assert!(partir_csv("\"sin cerrar").is_err());
        assert!(partir_csv("comilla \" a mitad").is_err());
    }

    // ── Import CSV nodos ─────────────────────────────────────────

    #[test]
    fn csv_import_nodos_con_tipos_y_labels() {
        let texto = "id:ID,name:STRING,age:INT,alto:FLOAT,activo:BOOL,:LABEL\n\
                     0,Zoe,44,1.70,true,Person:Activo\n\
                     1,\"Oviedo, Asturias\",,1.65,false,City\n";
        let mut store = MemoryStore::new();
        let stats = importar_csv_nodos(&mut lector(texto), &mut store).unwrap();
        assert_eq!(stats.nodos, 2);
        assert_eq!(stats.lineas, 3);
        let zoe = store.get_node(0).unwrap();
        assert_eq!(zoe.labels, vec!["Person".to_string(), "Activo".into()]);
        assert_eq!(zoe.props.get("name"), Some(&Value::String("Zoe".into())));
        assert_eq!(zoe.props.get("age"), Some(&Value::Int(44)));
        assert_eq!(zoe.props.get("alto"), Some(&Value::Float(1.7)));
        assert_eq!(zoe.props.get("activo"), Some(&Value::Bool(true)));
        // Campo vacío = prop ausente (no Null).
        let ovi = store.get_node(1).unwrap();
        assert!(!ovi.props.contains_key("age"));
        // ¡La coma iba dentro de las comillas!
        assert_eq!(
            ovi.props.get("name"),
            Some(&Value::String("Oviedo, Asturias".into()))
        );
    }

    #[test]
    fn csv_import_nodos_cabecera_sin_id() {
        let mut store = MemoryStore::new();
        let err = importar_csv_nodos(&mut lector("name:STRING\nZoe\n"), &mut store).unwrap_err();
        assert!(matches!(err, ImportError::CabeceraInvalida { .. }));
        assert!(err.to_string().contains("id:ID"));
    }

    #[test]
    fn csv_import_nodos_duplicado_con_linea() {
        let texto = "id:ID,:LABEL\n0,Person\n0,Person\n";
        let mut store = MemoryStore::new();
        let err = importar_csv_nodos(&mut lector(texto), &mut store).unwrap_err();
        match err {
            ImportError::RegistroRechazado { linea, .. } => assert_eq!(linea, 3),
            otra => panic!("esperaba RegistroRechazado, llegó {otra:?}"),
        }
        // Fail-fast: el primero SÍ quedó aplicado.
        assert_eq!(store.node_count(), 1);
    }

    #[test]
    fn csv_import_nodos_fila_malformada_y_tipo_invalido() {
        let mut store = MemoryStore::new();
        // Columnas de menos.
        let err = importar_csv_nodos(&mut lector("id:ID,age:INT,:LABEL\n0,Person\n"), &mut store)
            .unwrap_err();
        assert!(matches!(err, ImportError::FilaMalformada { linea: 2, .. }));

        // INT que no es INT.
        let err = importar_csv_nodos(
            &mut lector("id:ID,age:INT,:LABEL\n0,joven,Person\n"),
            &mut store,
        )
        .unwrap_err();
        assert!(err.to_string().contains("línea 2"));
        assert!(err.to_string().contains("no es INT"));
    }

    #[test]
    fn csv_import_nodos_sin_label_es_semantica() {
        let mut store = MemoryStore::new();
        // :LABEL declarado pero vacío.
        let err = importar_csv_nodos(&mut lector("id:ID,:LABEL\n0,\n"), &mut store).unwrap_err();
        assert!(matches!(err, ImportError::Semantica { linea: 2, .. }));
    }

    // ── Import CSV aristas ───────────────────────────────────────

    #[test]
    fn csv_import_aristas_con_id_opcional() {
        // Sin id:ID → ids secuenciales desde 0 (colisiona con la arista 0
        // del fixture: usamos otro fixture vacío de aristas).
        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "A")).unwrap();
        store.put_node(Node::new(1, "B")).unwrap();
        let texto = "de:START_ID,a:END_ID,tipo:TYPE,since:INT\n0,1,KNOWS,2020\n1,0,KNOWS,2021\n";
        let stats = importar_csv_aristas(&mut lector(texto), &mut store).unwrap();
        assert_eq!(stats.aristas, 2);
        assert_eq!(store.edge_count(), 2);
        assert_eq!(store.get_edge(0).unwrap().label, "KNOWS");
        assert_eq!(
            store.get_edge(1).unwrap().props.get("since"),
            Some(&Value::Int(2021))
        );

        // Con id:ID explícito.
        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "A")).unwrap();
        store.put_node(Node::new(1, "B")).unwrap();
        let texto = "id:ID,de:START_ID,a:END_ID,tipo:TYPE\n7,0,1,KNOWS\n";
        importar_csv_aristas(&mut lector(texto), &mut store).unwrap();
        assert!(store.get_edge(7).is_some());
    }

    #[test]
    fn csv_import_aristas_extremos_inexistentes_con_linea() {
        let mut store = MemoryStore::new();
        store.put_node(Node::new(0, "A")).unwrap();
        let texto = "de:START_ID,a:END_ID,tipo:TYPE\n0,9,KNOWS\n";
        let err = importar_csv_aristas(&mut lector(texto), &mut store).unwrap_err();
        match err {
            ImportError::RegistroRechazado { linea, causa } => {
                assert_eq!(linea, 2);
                assert!(matches!(causa, StoreError::InvalidEdgeEndpoints { .. }));
            }
            otra => panic!("esperaba RegistroRechazado, llegó {otra:?}"),
        }
    }

    // ── Export CSV + roundtrip ───────────────────────────────────

    #[test]
    fn csv_export_nodos_cabecera_union_ordenada_y_comillas() {
        let mut store = grafo();
        store
            .put_node(
                Node::new(2, "Person").with_prop("apellido", Value::String("Vega, Gijón".into())),
            )
            .unwrap();
        let mut salida = Vec::new();
        exportar_csv_nodos(&store, &mut salida).unwrap();
        let texto = String::from_utf8(salida).unwrap();
        let lineas: Vec<&str> = texto.lines().collect();
        // Unión de props ORDENADA (BTreeMap): apellido, name, since? no —
        // nodos: apellido, name (+ age no existe aquí). :LABEL al final.
        assert_eq!(lineas[0], "id:ID, apellido:STRING, name:STRING, :LABEL");
        // La coma del apellido se exporta entre comillas con "" si hiciera falta.
        assert!(lineas[3].contains("\"Vega, Gijón\""));
        assert!(lineas[3].ends_with("Person"));
    }

    #[test]
    fn csv_roundtrip_completo_del_grafo_demo() {
        let demo = crate::cap20_volcano::demo_graph();
        let mut nodos = Vec::new();
        let mut aristas = Vec::new();
        exportar_csv_nodos(&demo, &mut nodos).unwrap();
        exportar_csv_aristas(&demo, &mut aristas).unwrap();

        let mut renacido = MemoryStore::new();
        let s1 = importar_csv_nodos(&mut Cursor::new(nodos.clone()), &mut renacido).unwrap();
        let s2 = importar_csv_aristas(&mut Cursor::new(aristas.clone()), &mut renacido).unwrap();
        assert_eq!(s1.nodos, demo.node_count());
        assert_eq!(s2.aristas, demo.edge_count());
        assert_eq!(renacido.node_count(), demo.node_count());
        assert_eq!(renacido.edge_count(), demo.edge_count());
        // Contenido exacto en un par de muestras.
        assert_eq!(
            renacido.get_node(0).unwrap().props,
            demo.get_node(0).unwrap().props
        );
        assert_eq!(
            renacido.get_edge(0).unwrap().label,
            demo.get_edge(0).unwrap().label
        );
    }

    // ── JSON (parser a mano) ─────────────────────────────────────

    #[test]
    fn json_literales_y_anidados() {
        let v = parsear_json(
            r#"{"a":1,"b":-2.5,"c":true,"d":null,"e":[1,2,{"x":"y"}],"f":{"g":{"h":1}}}"#,
        )
        .unwrap();
        assert_eq!(v.get("a"), Some(&JsonValor::Int(1)));
        assert_eq!(v.get("b"), Some(&JsonValor::Float(-2.5)));
        assert_eq!(v.get("c"), Some(&JsonValor::Bool(true)));
        assert_eq!(v.get("d"), Some(&JsonValor::Null));
        match v.get("e") {
            Some(JsonValor::Array(items)) => assert_eq!(items.len(), 3),
            otra => panic!("e no es array: {otra:?}"),
        }
        // Anidado profundo.
        assert_eq!(
            v.get("f").and_then(|f| f.get("g")).and_then(|g| g.get("h")),
            Some(&JsonValor::Int(1))
        );
    }

    #[test]
    fn json_escapes() {
        let v = parsear_json(r#"{"s":"línea\ncomilla \" barra \\ unicode \u0041"}"#).unwrap();
        match v.get("s") {
            Some(JsonValor::String(s)) => {
                assert_eq!(s, "línea\ncomilla \" barra \\ unicode A");
            }
            otra => panic!("s no es string: {otra:?}"),
        }
    }

    #[test]
    fn json_errores_de_parseo() {
        assert!(parsear_json("{\"a\":1} basura").is_err()); // basura tras el valor
        assert!(parsear_json("{\"a\":").is_err()); // valor colgando
        assert!(parsear_json("[1,2").is_err()); // array sin cerrar
        assert!(parsear_json("\"sin cerrar").is_err()); // string abierto
        assert!(parsear_json("{\"a\": \"\\q\"}").is_err()); // escape inválido
        // El par sustituto: subconjunto documentado.
        assert!(parsear_json(r#""\uD83D""#).is_err());
    }

    // ── Import/export JSONL ──────────────────────────────────────

    #[test]
    fn jsonl_import_nodos_y_aristas_mezclados() {
        let texto = concat!(
            "{\"tipo\":\"nodo\",\"id\":0,\"labels\":[\"Person\"],\"props\":{\"name\":\"Zoe\",\"age\":44}}\n",
            "{\"tipo\":\"nodo\",\"id\":1,\"labels\":[\"City\"],\"props\":{}}\n",
            "{\"tipo\":\"arista\",\"id\":0,\"de\":0,\"a\":1,\"rel\":\"LIVES_IN\",\"props\":{\"since\":2019}}\n",
        );
        let mut store = MemoryStore::new();
        let stats = importar_jsonl(&mut lector(texto), &mut store).unwrap();
        assert_eq!(stats.nodos, 2);
        assert_eq!(stats.aristas, 1);
        assert_eq!(
            store.get_node(0).unwrap().props.get("age"),
            Some(&Value::Int(44))
        );
        assert_eq!(store.get_edge(0).unwrap().label, "LIVES_IN");
    }

    #[test]
    fn jsonl_import_errores_con_linea() {
        // JSON roto (objeto sin cerrar en la línea 2).
        let mut store = MemoryStore::new();
        let err = importar_jsonl(
            &mut lector(
                "{\"tipo\":\"nodo\",\"id\":0,\"labels\":[\"A\"],\"props\":{}}\n{\"tipo\": \"nodo\"",
            ),
            &mut store,
        )
        .unwrap_err();
        match err {
            ImportError::FilaMalformada { linea, .. } => assert_eq!(linea, 2),
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn jsonl_import_tipo_desconocido_con_linea() {
        // tipo desconocido en la línea 2.
        let mut store = MemoryStore::new();
        let err = importar_jsonl(
            &mut lector("{\"tipo\":\"nodo\",\"id\":0,\"labels\":[\"A\"],\"props\":{}}\n{\"tipo\":\"patata\"}\n"),
            &mut store,
        )
        .unwrap_err();
        match err {
            ImportError::Semantica { linea, causa } => {
                assert_eq!(linea, 2);
                assert!(causa.contains("patata"));
            }
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn jsonl_export_formato_exacto_y_bytes_roundtrip() {
        let mut store = MemoryStore::new();
        store
            .put_node(Node::new(0, "X").with_prop("crudo", Value::Bytes(vec![1, 2, 0xFF])))
            .unwrap();
        let mut salida = Vec::new();
        exportar_jsonl(&store, &mut salida).unwrap();
        let texto = String::from_utf8(salida).unwrap();
        assert_eq!(
            texto.trim(),
            "{\"tipo\":\"nodo\",\"id\":0,\"labels\":[\"X\"],\"props\":{\"crudo\":[1,2,255]}}"
        );
        // Bytes → array de ints → Bytes (JSONL es el formato sin pérdida).
        let mut renacido = MemoryStore::new();
        importar_jsonl(&mut Cursor::new(texto.as_bytes().to_vec()), &mut renacido).unwrap();
        assert_eq!(
            renacido.get_node(0).unwrap().props.get("crudo"),
            Some(&Value::Bytes(vec![1, 2, 0xFF]))
        );
    }

    #[test]
    fn jsonl_roundtrip_del_grafo_demo() {
        let demo = crate::cap20_volcano::demo_graph();
        let mut salida = Vec::new();
        exportar_jsonl(&demo, &mut salida).unwrap();

        let mut renacido = MemoryStore::new();
        let stats = importar_jsonl(&mut Cursor::new(salida), &mut renacido).unwrap();
        assert_eq!(stats.nodos, demo.node_count());
        assert_eq!(stats.aristas, demo.edge_count());
        // Contenido EXACTO: props completos nodo a nodo.
        for (a, b) in demo.iter_nodes().zip(renacido.iter_nodes()) {
            assert_eq!(a.props, b.props, "props divergen en nodo {}", a.id);
            assert_eq!(a.labels, b.labels);
        }
        for (a, b) in demo.iter_edges().zip(renacido.iter_edges()) {
            assert_eq!(a.props, b.props, "props divergen en arista {}", a.id);
            assert_eq!(a.label, b.label);
        }
    }

    // ── XML (eventos) ────────────────────────────────────────────

    #[test]
    fn xml_eventos_apertura_cierre_texto_entidades() {
        let eventos =
            eventos_xml("<a x=\"1\">t &amp; t</a><b/><!-- com --><!--? --><?xml v?>").unwrap();
        assert_eq!(
            eventos[0],
            EventoXml::Apertura {
                etiqueta: "a".into(),
                attrs: vec![("x".into(), "1".into())],
                vacia: false
            }
        );
        assert_eq!(eventos[1], EventoXml::Texto("t & t".into()));
        assert_eq!(
            eventos[2],
            EventoXml::Cierre {
                etiqueta: "a".into()
            }
        );
        assert!(
            matches!(&eventos[3], EventoXml::Apertura { etiqueta, vacia: true, .. } if etiqueta == "b")
        );
        // Comentarios e instrucciones: saltados (sólo quedan 4 eventos).
        assert_eq!(eventos.len(), 4);
    }

    #[test]
    fn xml_errores() {
        assert!(eventos_xml("<a").is_err()); // sin '>'
        assert!(eventos_xml("<a x=1>").is_err()); // atributo sin comillas
        assert!(eventos_xml("t &patata; t").is_err()); // entidad desconocida
    }

    // ── Import/export GraphML ────────────────────────────────────

    const GRAPHML_EJEMPLO: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n",
        "  <key id=\"d0\" for=\"node\" attr.name=\"name\" attr.type=\"string\"/>\n",
        "  <key id=\"d1\" for=\"node\" attr.name=\"age\" attr.type=\"int\"/>\n",
        "  <key id=\"e0\" for=\"edge\" attr.name=\"since\" attr.type=\"int\"/>\n",
        "  <graph edgedefault=\"directed\">\n",
        "    <node id=\"n0\"><data key=\"dL\">Person</data><data key=\"d0\">Zoe</data><data key=\"d1\">44</data></node>\n",
        "    <node id=\"n1\"><data key=\"dL\">City</data><data key=\"d0\">Oviedo</data></node>\n",
        "    <edge id=\"7\" source=\"n0\" target=\"n1\"><data key=\"eT\">LIVES_IN</data><data key=\"e0\">2019</data></edge>\n",
        "  </graph>\n",
        "</graphml>\n",
    );

    #[test]
    fn graphml_import_ids_externos_a_internos() {
        let mut store = MemoryStore::new();
        let stats = importar_graphml(&mut lector(GRAPHML_EJEMPLO), &mut store).unwrap();
        assert_eq!(stats.nodos, 2);
        assert_eq!(stats.aristas, 1);
        // n0 → NodeId 0, n1 → NodeId 1 (orden de aparición, cap. 3).
        assert_eq!(
            store.get_node(0).unwrap().props.get("name"),
            Some(&Value::String("Zoe".into()))
        );
        assert_eq!(
            store.get_node(0).unwrap().props.get("age"),
            Some(&Value::Int(44))
        );
        assert_eq!(
            store.get_node(0).unwrap().labels,
            vec!["Person".to_string()]
        );
        let arista = store.get_edge(7).unwrap();
        assert_eq!(arista.source, 0);
        assert_eq!(arista.target, 1);
        assert_eq!(arista.label, "LIVES_IN");
        assert_eq!(arista.props.get("since"), Some(&Value::Int(2019)));
    }

    #[test]
    fn graphml_import_edge_a_nodo_desconocido() {
        let mal = concat!(
            "<graphml><graph>",
            "<node id=\"n0\"/>",
            "<edge source=\"nX\" target=\"n0\"/>",
            "</graph></graphml>",
        );
        let mut store = MemoryStore::new();
        let err = importar_graphml(&mut lector(mal), &mut store).unwrap_err();
        match err {
            ImportError::Semantica { causa, .. } => assert!(causa.contains("nX")),
            otra => panic!("{otra:?}"),
        }
    }

    #[test]
    fn graphml_roundtrip_del_grafo_demo() {
        let demo = crate::cap20_volcano::demo_graph();
        let mut salida = Vec::new();
        exportar_graphml(&demo, &mut salida).unwrap();
        let texto = String::from_utf8_lossy(&salida).into_owned();

        let mut renacido = MemoryStore::new();
        let stats =
            importar_graphml(&mut Cursor::new(texto.as_bytes().to_vec()), &mut renacido).unwrap();
        assert_eq!(stats.nodos, demo.node_count());
        assert_eq!(stats.aristas, demo.edge_count());
        // Labels via la convención dL, tipos via eT.
        assert_eq!(
            renacido.get_node(0).unwrap().labels,
            demo.get_node(0).unwrap().labels
        );
        assert_eq!(
            renacido.get_edge(0).unwrap().label,
            demo.get_edge(0).unwrap().label
        );
        // Y las props escalares sobreviven.
        assert_eq!(
            renacido.get_node(0).unwrap().props,
            demo.get_node(0).unwrap().props
        );
    }

    #[test]
    fn graphml_export_escapa_entidades() {
        let mut store = MemoryStore::new();
        store
            .put_node(Node::new(0, "X").with_prop("nombre", Value::String("a&b<c\"d".into())))
            .unwrap();
        let mut salida = Vec::new();
        exportar_graphml(&store, &mut salida).unwrap();
        let texto = String::from_utf8(salida).unwrap();
        assert!(texto.contains("a&amp;b&lt;c&quot;d"));
        // Y re-importa a los caracteres originales.
        let mut renacido = MemoryStore::new();
        importar_graphml(&mut Cursor::new(texto.as_bytes().to_vec()), &mut renacido).unwrap();
        assert_eq!(
            renacido.get_node(0).unwrap().props.get("nombre"),
            Some(&Value::String("a&b<c\"d".into()))
        );
    }

    // ── Streaming: línea a línea ─────────────────────────────────

    #[test]
    fn streaming_mil_nodos_jsonl_linea_a_linea() {
        // El argumento de >RAM es de DISEÑO (read_line por registro); lo
        // que sí testea esto: 1.000 líneas entran y salen contadas, y el
        // importador nunca materializa el fichero (procesa y descarta).
        let mut texto = String::new();
        for i in 0..1000 {
            texto.push_str(&format!(
                "{{\"tipo\":\"nodo\",\"id\":{i},\"labels\":[\"N\"],\"props\":{{\"v\":{i}}}}}\n"
            ));
        }
        let mut store = MemoryStore::new();
        let stats = importar_jsonl(&mut lector(&texto), &mut store).unwrap();
        assert_eq!(stats.nodos, 1000);
        assert_eq!(stats.lineas, 1000);
        assert_eq!(store.node_count(), 1000);
        assert_eq!(
            store.get_node(999).unwrap().props.get("v"),
            Some(&Value::Int(999))
        );
    }

    // ── Errores y Display ────────────────────────────────────────

    #[test]
    fn errores_display() {
        let e = ImportError::CabeceraInvalida {
            causa: "falta :ID".into(),
        };
        assert!(e.to_string().contains("cabecera"));
        let e = ImportError::FilaMalformada {
            linea: 42,
            causa: "rota".into(),
        };
        assert!(e.to_string().contains("línea 42"));
        let e = ImportError::Io("disco lleno".into());
        assert!(e.to_string().contains("E/S"));
        assert!(std::error::Error::source(&e).is_none());
    }

    #[test]
    fn stats_display() {
        let s = EstadisticasImport {
            nodos: 3,
            aristas: 2,
            lineas: 6,
        };
        assert!(s.to_string().contains("3 nodos y 2 aristas"));
    }

    // ── CSV con dos secciones (nodos + aristas) ─────────────────

    #[test]
    fn exportar_csv_unico_y_reimport_roundtrip() {
        // El exporter "compuesto" (nodos, separador `# aristas`, aristas)
        // debe ser leído por `importar_csv_unico` SIN secciones cruzadas.
        let demo = crate::cap20_volcano::demo_graph();
        let mut buf = Vec::new();
        exportar_csv(&demo, &mut buf).unwrap();
        let texto = String::from_utf8(buf).unwrap();
        // Separador presente exactamente una vez.
        assert_eq!(texto.matches("# aristas").count(), 1);

        let mut renacido = MemoryStore::new();
        let stats =
            importar_csv_unico(&mut Cursor::new(texto.into_bytes()), &mut renacido).unwrap();
        assert_eq!(stats.nodos, demo.node_count());
        assert_eq!(stats.aristas, demo.edge_count());
    }

    #[test]
    fn importar_csv_unico_solo_nodos_sin_aristas() {
        // CSV con sólo nodos (sin `# aristas`): el importer devuelve los
        // nodos sin fallar (EOF antes de la segunda sección).
        let texto = "id:ID,name:STRING,:LABEL\n0,Zoe,Person\n";
        let mut store = MemoryStore::new();
        let stats =
            importar_csv_unico(&mut Cursor::new(texto.as_bytes().to_vec()), &mut store).unwrap();
        assert_eq!(stats.nodos, 1);
        assert_eq!(stats.aristas, 0);
    }
}
