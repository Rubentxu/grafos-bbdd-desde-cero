//! Vol.III — Cap.45: Workflows de ingesta: de datos crudos al grafo.
//!
//! QUINTO capítulo del Vol.III («Grafos en la era de la IA»), Parte I
//! «Modelar datos de grafos», y CIERRE DE LA PARTE I. El cap. 32 importaba
//! ficheros LIMPIOS con `:ID` y autocommit por fila; el cap. 44 dejó el
//! esquema declarativo con un gancho literal: «nadie valida al importar: el
//! lote de mañana puede traer un duplicado». ESTE capítulo responde CON DATOS:
//! la ingesta es un PIPELINE de cuatro etapas (extraer → validar → mapear →
//! cargar) que aplica las reglas del esquema AL VUELO por lote, resuelve
//! duplicados por entity resolution y fusiona con sobrescritura cuidada.
//!
//! Modelo mental único: **la ingesta es una FÁBRICA con cuatro estaciones;
//! el esquema del cap-44 es el control de calidad; el entity resolution es el
//! control de duplicados que fusiona ANTES de que la basura llegue al grafo**.
//! La moneda son filas, lotes, rechazos, fusiones y conjuntos exactos — nunca
//! µs (espejo de las decisiones #12 cap-41, #11 cap-43 y #9 cap-44).
//!
//! Qué entrega ESTA pieza (implementación incremental, contrato §2, pasos
//! 6-7 del patrón de troceo):
//!
//! 1. **Tipos del pipeline**: [`RegistroCrudo`] (una fila de CSV sin
//!    interpretar: `tipo` = qué entidad/relación describe, `campos` = pares
//!    columna→valor), [`DatosCrudos`] (el lote completo de un fichero),
//!    [`ErrorIngesta`] (fila inválida con número de línea, o lote inválido)
//!    e [`InformeIngesta`] (contadores con Display en tabla).
//! 2. **Extraer**: [`DatosCrudos::desde_csv`], el constructor que parsea UN
//!    fichero CSV crudo (cabecera = nombres de columnas, filas = registros
//!    con `tipo` fijo) REUTILIZANDO [`partir_csv`] del cap-32 (RFC 4180-lite:
//!    comillas, `""` escapado) — la etapa extraer del pipeline empieza aquí.
//! 3. **Los datasets crudos del paso-5** (`datasets/kb-lira/paso-5/crudos/`):
//!    personas, documentos, temas y relaciones SIN ids y con la suciedad
//!    deliberada del contrato (3 duplicados exactos, 4 variantes de Ana, 1
//!    typo de título, el fantasma «memoria de agentes» 26/61 y 1 MEMBER_OF
//!    solapada `[2020,2023)`) — el contenido que la validación y el entity
//!    resolution de las piezas siguientes van a rechazar y fusionar.
//! 4. **Entity resolution: el grafo de similitud y sus clusters** —
//!    [`construir_grafo_similitud`] (un nodo `Persona` por nombre en un
//!    `MemoryStore` temporal, arista `SIMILAR` si mismo bloque Y
//!    `jaccard ≥ 0.5` o mismo primer token Y `jaccard ≥ 0.25`, con el
//!    contador de pares comparados bajo blocking) y [`clusters_de_similitud`]
//!    (las componentes conexas del cap-25 REUTILIZADA: los clusters de
//!    duplicados son las componentes del grafo `SIMILAR`).
//! 5. **Fusión con sobrescritura cuidada** — [`fusionar_cluster`] reduce un
//!    cluster de duplicados a SU canónico (el miembro con MÁS props no
//!    vacías; empate → menor id), rellena las props ausentes, DECLARA los
//!    conflictos (propiedad, valor canónico, valor descartado — gana el
//!    canónico, nunca silencioso) y REAPUNTA las aristas de los descartados
//!    al canónico (delete_edge + put_edge con el mismo id, rel_tipo y props,
//!    solo cambian los extremos); [`InformeFusion`] y [`ConflictoFusion`]
//!    llevan los conteos reales.
//! 6. **`HistorialIngesta`** — el transaction-time de la carga, la deuda del
//!    cap-43 cobrada: un tipo NUEVO append-only con ts monótono (la forma
//!    del `HistoricoAfiliaciones` del cap-43 REPLICADA, sin tocar cap-43)
//!    cuyos seis eventos (`CargaIniciada/LoteValidado/RegistroRechazado/
//!    FusionEntidad/ConflictoDeclarado/CargaCompletada`) registran la vida
//!    de la carga: la fusión y el rechazo DEJAN HUELLA.
//! 7. **El pipeline completo** — [`cargar_paso5`]: los 4 ficheros crudos de
//!    `datasets/kb-lira/paso-5/crudos/` → extraer por lotes de 25 → validar
//!    por lote (rechazo selectivo con línea y motivo) → mapear entidades →
//!    entity resolution y fusión ANTES de las relaciones → mapear las 158
//!    aristas (creando Organizacion/Proyecto/Conferencia/Resena desde los
//!    extremos) → `verificar_esquema` del cap-44 como puerta final → **67
//!    nodos / 158 aristas**, esquema `Ok`, y un `InformeIngesta` con los
//!    contadores reales (221 filas, 11 lotes, 4 rechazos, 6 fusiones).
//!
//! Fronteras duras: nada de `cap32/41/42/43/44` se toca — el pipeline se hace
//! CON la API existente (`partir_csv` es pública y se reutiliza aquí); sin
//! wiring hasta que el módulo compile limpio.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use super::cap07_modelo::{Edge, Node, Value};
use super::cap08_graph_store::{GraphStore, MemoryStore};
use super::cap25_comunidades::componentes_conexas;
use super::cap32_import_export::{exportar_csv_aristas, exportar_csv_nodos, partir_csv};
use super::cap44_esquema::{esquema_kb_lira, verificar_esquema};

/// UNA fila de CSV crudo sin interpretar.
///
/// `tipo` es qué entidad o relación describe la fila (p. ej. `Persona`,
/// `Documento`, `Tema` o `Relacion`); `campos` son los pares columna→valor
/// tal cual vinieron del fichero. La interpretación (mapeo a props, tipos,
/// validación) es trabajo de etapas posteriores del pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistroCrudo {
    /// Qué entidad/relación describe la fila (fijo por fichero).
    pub tipo: String,
    /// Pares columna→valor en el orden de la cabecera.
    pub campos: Vec<(String, String)>,
}

/// Error de la etapa extraer: una fila inválida (con su número de línea) o
/// un lote/fichero inválido entero (cabecera ausente o malformada).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorIngesta {
    /// Una fila concreta es inválida: el pipeline la RECHAZA y sigue.
    LineaInvalida {
        /// Número de línea en el fichero (1 = cabecera).
        linea: usize,
        /// Motivo legible del rechazo.
        motivo: String,
    },
    /// El lote entero es inválido (cabecera ausente o rota): aborta.
    LoteInvalido {
        /// Motivo legible del aborto.
        motivo: String,
    },
}

impl fmt::Display for ErrorIngesta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorIngesta::LineaInvalida { linea, motivo } => {
                write!(f, "línea {linea}: {motivo}")
            }
            ErrorIngesta::LoteInvalido { motivo } => write!(f, "lote inválido: {motivo}"),
        }
    }
}

impl Error for ErrorIngesta {}

/// El contenido crudo de UN fichero: la lista de registros sin interpretar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatosCrudos {
    /// Los registros del fichero, en orden de lectura.
    pub registros: Vec<RegistroCrudo>,
}

impl DatosCrudos {
    /// Construye los datos crudos desde el contenido de UN fichero CSV.
    ///
    /// La primera línea es la cabecera (nombres de columnas); cada línea
    /// siguiente es un registro con `tipo` FIJO (el parámetro). El separador
    /// y las comillas los gestiona [`partir_csv`] del cap-32, REUTILIZADO
    /// tal cual (RFC 4180-lite): la etapa extraer del pipeline es la misma
    /// para los cuatro ficheros crudos del paso-5.
    ///
    /// Una fila con número de columnas distinto de la cabecera, vacía o con
    /// comillas rotas devuelve [`ErrorIngesta::LineaInvalida`] con su número
    /// de línea; un contenido sin cabecera devuelve
    /// [`ErrorIngesta::LoteInvalido`] (aborta, como la cabecera del cap-32).
    pub fn desde_csv(contenido: &str, tipo: &str) -> Result<DatosCrudos, ErrorIngesta> {
        let mut lineas = contenido.lines();
        let cabecera = lineas.next().unwrap_or("");
        if cabecera.trim().is_empty() {
            return Err(ErrorIngesta::LoteInvalido {
                motivo: "fichero vacío (se esperaba una cabecera)".into(),
            });
        }
        let columnas =
            partir_csv(cabecera).map_err(|motivo| ErrorIngesta::LoteInvalido { motivo })?;
        if columnas.is_empty() {
            return Err(ErrorIngesta::LoteInvalido {
                motivo: "cabecera vacía".into(),
            });
        }
        let mut registros = Vec::new();
        for (i, linea) in lineas.enumerate() {
            let n_linea = i + 2; // 1 = cabecera
            if linea.trim().is_empty() {
                return Err(ErrorIngesta::LineaInvalida {
                    linea: n_linea,
                    motivo: "línea vacía".into(),
                });
            }
            let campos = partir_csv(linea).map_err(|motivo| ErrorIngesta::LineaInvalida {
                linea: n_linea,
                motivo,
            })?;
            if campos.len() != columnas.len() {
                return Err(ErrorIngesta::LineaInvalida {
                    linea: n_linea,
                    motivo: format!(
                        "{} campos, la cabecera declara {}",
                        campos.len(),
                        columnas.len()
                    ),
                });
            }
            let pares = columnas.iter().cloned().zip(campos).collect();
            registros.push(RegistroCrudo {
                tipo: tipo.to_string(),
                campos: pares,
            });
        }
        Ok(DatosCrudos { registros })
    }
}

/// Contadores del pipeline: la moneda del capítulo (filas, lotes, rechazos,
/// fusiones y conjuntos exactos — nunca tiempos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InformeIngesta {
    /// Filas de datos leídas en total (sin cabeceras).
    pub filas_leidas: usize,
    /// Lotes formados por la etapa extraer.
    pub lotes: usize,
    /// Registros rechazados por la validación, con su línea.
    pub rechazos: usize,
    /// Entidades fusionadas por el entity resolution.
    pub fusiones: usize,
    /// Nodos del grafo final.
    pub nodos_finales: usize,
    /// Aristas del grafo final.
    pub aristas_finales: usize,
}

impl fmt::Display for InformeIngesta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Informe de ingesta — KB-Lira paso-5")?;
        writeln!(f, "{}", "─".repeat(48))?;
        writeln!(f, "  filas leídas:     {}", self.filas_leidas)?;
        writeln!(f, "  lotes:            {}", self.lotes)?;
        writeln!(f, "  rechazos:         {}", self.rechazos)?;
        writeln!(f, "  fusiones:         {}", self.fusiones)?;
        writeln!(f, "  nodos finales:    {}", self.nodos_finales)?;
        writeln!(f, "  aristas finales:  {}", self.aristas_finales)?;
        Ok(())
    }
}

/// Normaliza un nombre de entidad para el entity resolution.
///
/// Qué hace, siguiendo el contrato §2 («minúsculas sin tildes, tabla
/// estática ~10 líneas»): pasa a minúsculas, elimina las tildes con una
/// tabla estática (`á→a`, `é→e`, `í→i`, `ó→o`, `ú→u`, `ü→u`, `ñ→n`),
/// recorta los espacios de los extremos y colapsa las secuencias de
/// espacios a una sola.
///
/// Qué NO hace, a propósito: NO elimina los paréntesis ni su contenido.
/// «Ana García (Universidad de Lira)» normaliza a «ana garcia (universidad
/// de lira)» — el contrato §5 (decisión #5) resuelve el caso del paréntesis
/// con la métrica de bigramas (mismo primer token Y `jaccard ≥ 0.25`), NO
/// con la normalización: si esta borrara el paréntesis, el par se volvería
/// idéntico a «ana garcia» y el caso de contraste del contrato (jaccard
/// 0.47 con refuerzo de primer token) no existiría. La normalización UNE
/// las variantes ortográficas (mayúsculas, tildes, espacios) y DISTINGUE
/// lo que la comparación debe decidir (abreviaturas, paréntesis).
pub fn normalizar_nombre(nombre: &str) -> String {
    const SUSTITUCIONES: &[(&str, &str)] = &[
        ("á", "a"),
        ("é", "e"),
        ("í", "i"),
        ("ó", "o"),
        ("ú", "u"),
        ("ü", "u"),
        ("ñ", "n"),
    ];
    let mut normalizado = nombre.trim().to_lowercase();
    for (con_tilde, sin_tilde) in SUSTITUCIONES {
        normalizado = normalizado.replace(con_tilde, sin_tilde);
    }
    normalizado.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Distancia de Levenshtein (1966) entre dos cadenas: el número mínimo de
/// inserciones, borrados y sustituciones para convertir `a` en `b`.
///
/// DP clásica O(|a| × |b|) en std puro sobre `char`, no sobre bytes: un
/// carácter multi-byte cuenta como UNA edición, no como dos o más. En el
/// pipeline los nombres llegan YA normalizados por [`normalizar_nombre`]
/// (sin tildes), así que la distancia mide diferencia de contenido real, y
/// la pareja «ana garcia» / «ana garcía» normalizada a 0 queda en manos de
/// la normalización (que es quien debe unir variantes).
pub fn distancia_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut anterior: Vec<usize> = (0..=b.len()).collect();
    let mut actual = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        actual[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            actual[j + 1] = (anterior[j + 1] + 1)
                .min(actual[j] + 1)
                .min(anterior[j] + usize::from(ca != cb));
        }
        std::mem::swap(&mut anterior, &mut actual);
    }
    anterior[b.len()]
}

/// Similitud de Levenshtein normalizada: `1 − dist / max(|a|, |b|)`.
///
/// 1.0 si las cadenas son idénticas, 0.0 si no comparten nada (la distancia
/// alcanza el máximo) y —por definición— 1.0 cuando ambas son vacías, para
/// evitar el 0/0: «vacío ≡ vacío» es la identidad, no el desacuerdo total.
/// Longitudes en `char`, coherente con [`distancia_levenshtein`].
pub fn similitud_levenshtein(a: &str, b: &str) -> f64 {
    let max = a.chars().count().max(b.chars().count());
    if max == 0 {
        return 1.0;
    }
    1.0 - distancia_levenshtein(a, b) as f64 / max as f64
}

/// Bigramas (pares de caracteres consecutivos) de una cadena, SIN
/// duplicados y en orden (BTreeSet/HashSet → `Vec` ordenado).
///
/// Definición exacta, siguiendo el contrato §2/§5: los pares de `char`
/// consecutivos de la cadena TAL COMO SE RECIBE — el pipeline llama antes a
/// [`normalizar_nombre`] («normalizar → bloquear → comparar»), así que esta
/// función no normaliza: recibe la forma ya canónica.
///
/// DECISIÓN documentada: SIN padding inicial ni final («_a»/«a_»). El
/// contrato define los bigramas como «pares de chars consecutivos» y no
/// menciona padding; y la predicción del contrato §5 para el caso de
/// contraste («ana garcia» vs «carla mendez»: 0.11) se reproduce EXACTA con
/// la definición sin padding (2/18 ≈ 0.111) — señal de que era la
/// intencionada. Consecuencia: el primer y el último carácter no tienen
/// vecino sintético (los q-grams industriales suelen rellenar los extremos
/// para dar peso a las fronteras), así que pesan menos; en los nombres de
/// persona la frontera inicial queda cubierta por el bloque por inicial y
/// por `mismo_primer_token`, de modo que no hay información del extremo
/// que el pipeline no recupere por otra vía.
pub fn bigramas(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut unicos = HashSet::new();
    for par in chars.windows(2) {
        unicos.insert(par.iter().collect::<String>());
    }
    let mut ordenados: Vec<String> = unicos.into_iter().collect();
    ordenados.sort();
    ordenados
}

/// Similitud de Jaccard entre los conjuntos de bigramas de `a` y `b`:
/// `|intersección de bigramas| / |unión de bigramas|` (0.0 si no comparten
/// ningún bigrama, 1.0 si los conjuntos son idénticos).
///
/// Convención para las cadenas vacías, espejo de [`similitud_levenshtein`]:
/// dos vacías → 1.0 (el 0/0 se define como la identidad: «vacío ≡ vacío»);
/// una vacía y la otra no → 0.0 (`|∅ ∩ A| = 0`). Sin padding (véase
/// [`bigramas`]).
///
/// La regla de arista del contrato §2/§5 es: mismo bloque Y (`jaccard ≥
/// 0.5` O mismo primer token Y `jaccard ≥ 0.25`). Valores REALES medidos
/// con esta definición (pines del criterio de parada honesto §8):
/// «ana garcia» vs «ana g.» = **0.4** (el contrato §5 predijo 0.56: no
/// alcanza la regla primaria del 0.5, pero el refuerzo del primer token la
/// une con 0.4 ≥ 0.25); «ana garcia» vs «ana garcia (universidad de lira)»
/// = 9/33 ≈ **0.27** (predijo 0.47, misma regla secundaria); «ana garcia»
/// vs «carla mendez» = 2/18 ≈ **0.111** (predijo 0.11 — COINCIDE).
pub fn jaccard_bigramas(a: &str, b: &str) -> f64 {
    let lista_a = bigramas(a);
    let lista_b = bigramas(b);
    if lista_a.is_empty() && lista_b.is_empty() {
        return 1.0;
    }
    let conjunto_a: HashSet<&str> = lista_a.iter().map(String::as_str).collect();
    let conjunto_b: HashSet<&str> = lista_b.iter().map(String::as_str).collect();
    let interseccion = conjunto_a.intersection(&conjunto_b).count();
    let union = conjunto_a.union(&conjunto_b).count();
    interseccion as f64 / union as f64
}

/// La clave de bloque del blocking por inicial: la PRIMERA LETRA del primer
/// token del nombre normalizado (p. ej. «Ana García (Universidad de Lira)»
/// → «a», «Beto» → «b»).
///
/// Definición exacta: [`normalizar_nombre`] → primer token → primer `char`.
/// El contrato §2/§5 la usa para agrupar candidatos: solo se comparan entre
/// sí los nombres del MISMO bloque. Con los 14 nombres REALES del CSV crudo
/// del paso-5, las 5 Anas comparten «a» (10 pares) y los dos Beto comparten
/// «b» (1 par) — los demás bloques tienen un solo miembro y no generan
/// comparaciones. Un nombre sin tokens devuelve la cadena vacía (fuera de
/// cualquier bloque: no se compara contra nadie; la validación ya rechaza
/// los nombres vacíos antes de llegar aquí).
pub fn bloque_por_inicial(nombre: &str) -> String {
    normalizar_nombre(nombre)
        .split_whitespace()
        .next()
        .and_then(|token| token.chars().next())
        .map(|c| c.to_string())
        .unwrap_or_default()
}

/// ¿`a` y `b` comparten el primer token tras normalizar? (p. ej. «ana
/// garcia» y «ana g.» → sí; «ana garcia» y «carla mendez» → no). Es la
/// segunda mitad de la regla compuesta del contrato §2/§5: mismo bloque Y
/// (`jaccard ≥ 0.5` O mismo primer token Y `jaccard ≥ 0.25`).
///
/// Dos nombres sin tokens (ambos vacíos tras normalizar) devuelven `true`
/// vacuamente (`None == None`); la validación rechaza los nombres vacíos
/// antes de que el entity resolution los compare.
pub fn mismo_primer_token(a: &str, b: &str) -> bool {
    normalizar_nombre(a).split_whitespace().next() == normalizar_nombre(b).split_whitespace().next()
}

/// Construye el grafo de similitud de los nombres dados: un `MemoryStore`
/// temporal con un nodo `Persona` por nombre (ids 0..n) y una arista
/// `SIMILAR` entre dos nombres cuando comparten bloque por inicial Y la
/// regla compuesta del contrato §2/§5: `jaccard_bigramas ≥ 0.5`, o mismo
/// primer token Y `jaccard_bigramas ≥ 0.25`.
///
/// Devuelve también el número de pares REALMENTE comparados, el contador
/// del blocking: solo se comparan los nombres del MISMO bloque por inicial
/// (las 5 Anas → 10 pares, los 2 Beto → 1 par; con los 14 nombres del CSV
/// crudo son 11 comparaciones frente a las 91 del todo-contra-todo). El
/// grafo es un clúster de trabajo: [`clusters_de_similitud`] lo convierte
/// en clusters de duplicados con las componentes conexas del cap-25.
pub fn construir_grafo_similitud(personas: &[String]) -> (MemoryStore, usize) {
    let mut store = MemoryStore::new();
    for i in 0..personas.len() {
        store
            .put_node(Node::new(i, "Persona"))
            .expect("ids 0..n densos: ningún nodo se repite");
    }
    let mut comparaciones = 0usize;
    let mut n_aristas = 0usize;
    for i in 0..personas.len() {
        for j in (i + 1)..personas.len() {
            if bloque_por_inicial(&personas[i]) != bloque_por_inicial(&personas[j]) {
                continue;
            }
            comparaciones += 1;
            let a = normalizar_nombre(&personas[i]);
            let b = normalizar_nombre(&personas[j]);
            let jaccard = jaccard_bigramas(&a, &b);
            if jaccard >= 0.5 || (mismo_primer_token(&a, &b) && jaccard >= 0.25) {
                store
                    .put_edge(Edge::new(n_aristas, i, j, "SIMILAR"))
                    .expect("ambos extremos ya existen: arista válida");
                n_aristas += 1;
            }
        }
    }
    (store, comparaciones)
}

/// Los clusters de duplicados del grafo de similitud: las componentes
/// conexas del cap-25 REUTILIZADA sobre el `MemoryStore` temporal.
///
/// `componentes_conexas` (cap25) trabaja sobre `&dyn GraphStore` y
/// devuelve `Result<ComponentesResult, ComunidadesError>`; `MemoryStore`
/// implementa `GraphStore`, así que el adaptador es la coerción estándar
/// `&MemoryStore → &dyn GraphStore` en la llamada (sin tocar cap-25) y el
/// `expect` es seguro: sobre un grafo sin pesos negativos la proyección
/// con peso constante del cap-25 nunca falla. Devuelve los clusters como
/// lista de ids de personas, en el orden canónico del cap-25: numerados
/// por menor miembro, miembros ascendentes.
pub fn clusters_de_similitud(store: &MemoryStore) -> Vec<Vec<usize>> {
    let resultado = componentes_conexas(store)
        .expect("grafo de similitud sin pesos negativos: no puede fallar");
    resultado.componentes()
}

/// Un conflicto de fusión DECLARADO (contrato §5, decisión de fusión):
/// dos nodos del cluster tenían la MISMA propiedad con valores DISTINTOS
/// (p. ej. el `orcid` de una variante de Ana). La regla explícita: gana el
/// canónico y el valor descartado se registra AQUÍ — nada se sobrescribe en
/// silencio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictoFusion {
    /// El nombre de la propiedad en conflicto (p. ej. `orcid`).
    pub propiedad: String,
    /// El valor que conserva el canónico.
    pub canónico: String,
    /// El valor descartado del otro miembro del cluster.
    pub descartado: String,
}

/// Los conteos REALES de una fusión de cluster: la moneda del capítulo
/// (fusiones, nodos, conflictos y aristas — nunca tiempos).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InformeFusion {
    /// Miembros del cluster que dejaron de ser nodos (cluster − canónico).
    pub fusiones: usize,
    /// Nodos totales del store tras la fusión.
    pub nodos_finales: usize,
    /// Los conflictos declarados, con sus valores (canónico y descartado).
    pub conflictos: Vec<ConflictoFusion>,
    /// Aristas de los nodos descartados REAPUNTADAS al canónico.
    pub aristas_reapuntadas: usize,
}

/// ¿Cuántas props NO vacías tiene un nodo? — la métrica del canónico.
///
/// El contrato §5 dice «más props no vacías»: en el modelo del cap-07 la
/// única prop vacía es [`Value::Null`] (un valor que no enriquece a la
/// entidad). Todo lo demás —incluida la cadena vacía— cuenta como contenido.
fn props_no_vacias(nodo: &Node) -> usize {
    nodo.props.values().filter(|v| !v.is_null()).count()
}

/// Convierte un [`Value`] a texto para el informe de conflictos: las cadenas
/// tal cual (el `orcid`, la afiliación), `Null` como `null` y el resto en su
/// forma Debug (los conflictos se DECLARAN legibles, no con representaciones
/// internas).
fn valor_a_texto(valor: &Value) -> String {
    match valor {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        otro => format!("{otro:?}"),
    }
}

/// Fusiona UN cluster de duplicados con «sobrescritura cuidada» (contrato
/// §5, decisión de fusión; §2: «reapuntar no borra»).
///
/// Qué hace, paso a paso:
///
/// 1. **Canónico**: el miembro con MÁS props no vacías ([`props_no_vacias`]);
///    si hay empate, gana el de MENOR id (la «menor línea» del contrato —
///    los ids del cluster son las líneas del fichero). Es la entidad más
///    rica: «el más datos» es la regla que un humano elegiría (Christen
///    2012, merge policies).
/// 2. **Snapshot de aristas ANTES de borrar**: `delete_node` del cap-08
///    borra EN CASCADA las aristas incidentes del nodo (cap08_graph_store.rs:
///    recorre `adj_out` y `adj_in` llamando a `delete_edge`). Por eso se
///    recolectan `out_edges` ∪ `in_edges` de todos los miembros (con dedup
///    por id: una arista entre dos descartados aparece en ambas listas)
///    antes de tocar nada, y se hace la distinción que el informe exige:
///    las aristas incidentes a los DESCARTADOS son las que se REAPUNTAN.
/// 3. **Unión de props sin sobrescribir en silencio**: las props del
///    canónico son la base; las IGUALES se conservan; las AUSENTES se
///    rellenan (con `Null` no — un valor vacío no enriquece); las DISTINTAS
///    registran un [`ConflictoFusion`] y gana el canónico (nunca silencioso:
///    el orcid malformado de una variante se descarta DECLARÁNDOLO).
/// 4. **Identidad**: si el canónico no tiene prop `nombre`, adopta
///    `nombres[canónico]` (el nombre canónico de la entidad). Si ya la
///    tiene, la suya manda — no se sobrescribe nada.
/// 5. **Borrado y re-creación**: se borran TODOS los miembros (incluido el
///    canónico, porque `put_node` rechaza ids repetidos y es la única forma
///    de escribir la unión de props) y se re-inserta el canónico con sus
///    labels intactos y la unión.
/// 6. **Re-apuntado**: cada arista del snapshot vuelve con el MISMO id,
///    rel_tipo y props (delete_edge + put_edge); los extremos que eran
///    miembros del cluster pasan a ser el canónico. Las aristas del
///    canónico vuelven iguales (reapuntar no borra: las 158 aristas del
///    pipeline sobreviven a las fusiones).
///
/// Devuelve el [`InformeFusion`] con los conteos reales: fusiones,
/// nodos finales del store, conflictos declarados y aristas reapuntadas.
pub fn fusionar_cluster(
    store: &mut MemoryStore,
    cluster: &[usize],
    nombres: &[String],
) -> InformeFusion {
    let miembros: Vec<usize> = cluster
        .iter()
        .copied()
        .filter(|&id| store.get_node(id).is_some())
        .collect();
    if miembros.is_empty() {
        return InformeFusion::default();
    }

    let canónico = *miembros
        .iter()
        .max_by(|&&a, &&b| {
            props_no_vacias(store.get_node(a).expect("miembro filtrado"))
                .cmp(&props_no_vacias(
                    store.get_node(b).expect("miembro filtrado"),
                ))
                .then(b.cmp(&a))
        })
        .expect("miembros no vacío");

    let mut aristas_tocadas = HashSet::new();
    let mut aristas_descartadas = HashSet::new();
    for &id in &miembros {
        for eid in store.out_edges(id).into_iter().chain(store.in_edges(id)) {
            aristas_tocadas.insert(eid);
            if id != canónico {
                aristas_descartadas.insert(eid);
            }
        }
    }
    let mut snapshot: Vec<Edge> = aristas_tocadas
        .iter()
        .map(|&eid| {
            store
                .get_edge(eid)
                .expect("arista viva en el snapshot")
                .clone()
        })
        .collect();
    snapshot.sort_by_key(|e| e.id);

    let mut props = store
        .get_node(canónico)
        .expect("el canónico vive")
        .props
        .clone();
    let mut conflictos = Vec::new();
    for &id in &miembros {
        if id == canónico {
            continue;
        }
        let nodo = store.get_node(id).expect("miembro filtrado");
        for (clave, valor) in &nodo.props {
            match props.get(clave) {
                Some(actual) if actual == valor => {}
                Some(actual) => conflictos.push(ConflictoFusion {
                    propiedad: clave.clone(),
                    canónico: valor_a_texto(actual),
                    descartado: valor_a_texto(valor),
                }),
                None if valor.is_null() => {}
                None => {
                    props.insert(clave.clone(), valor.clone());
                }
            }
        }
    }
    if !props.contains_key("nombre")
        && let Some(nombre) = nombres.get(canónico)
    {
        props.insert("nombre".into(), Value::String(nombre.clone()));
    }

    let labels = store
        .get_node(canónico)
        .expect("el canónico vive")
        .labels
        .clone();
    for &id in &miembros {
        store.delete_node(id);
    }
    store
        .put_node(Node {
            id: canónico,
            labels,
            props,
        })
        .expect("el id canónico quedó libre tras el borrado");

    let mut aristas_reapuntadas = 0usize;
    for arista in snapshot {
        let source = if miembros.contains(&arista.source) {
            canónico
        } else {
            arista.source
        };
        let target = if miembros.contains(&arista.target) {
            canónico
        } else {
            arista.target
        };
        store.delete_edge(arista.id);
        store
            .put_edge(Edge {
                id: arista.id,
                source,
                target,
                label: arista.label,
                props: arista.props,
            })
            .expect("extremos vivos (canónico o ajenos al cluster): arista válida");
        if aristas_descartadas.contains(&arista.id) {
            aristas_reapuntadas += 1;
        }
    }

    InformeFusion {
        fusiones: miembros.len() - 1,
        nodos_finales: store.node_count(),
        conflictos,
        aristas_reapuntadas,
    }
}

// ───────────────────────────────────────────────────────────────────────
// Pieza VALIDAR (reglas de fila) y MAPEAR (clave natural → id).
// Contrato §2, pasos 10-11 del patrón de troceo.
// ───────────────────────────────────────────────────────────────────────

/// ¿Tiene `orcid` el formato del contrato §2, `XXXX-XXXX-XXXX-XXXX`?
///
/// Definición exacta: cuatro grupos de CUATRO dígitos ASCII separados por
/// guiones (19 caracteres). El orcid malformado del CSV crudo del paso-5
/// (`0000-0001-2345-1`) falla: el último grupo tiene un solo dígito.
fn orcid_valido(orcid: &str) -> bool {
    let partes: Vec<&str> = orcid.split('-').collect();
    partes.len() == 4
        && partes
            .iter()
            .all(|parte| parte.len() == 4 && parte.chars().all(|c| c.is_ascii_digit()))
}

/// El valor de la columna `nombre` de un registro crudo, si la columna
/// existe. Una columna presente con valor vacío devuelve `Some("")`: la
/// distinción la decide la regla de presencia, no la búsqueda.
fn campo<'a>(registro: &'a RegistroCrudo, nombre: &str) -> Option<&'a str> {
    registro
        .campos
        .iter()
        .find(|(columna, _)| columna == nombre)
        .map(|(_, valor)| valor.as_str())
}

/// Valida UNA fila cruda: las reglas de FILA del pipeline (contrato §2,
/// «reglas de FILA derivadas del esquema: label conocida, props requeridas,
/// tipos, formato de orcid»). Es la validación de fila del write-time por
/// lote: el registro que viola se RECHAZA con su motivo y el lote sigue
/// (solo la cabecera inválida aborta, en la etapa extraer).
///
/// Reglas exactas y motivos de rechazo, por tipo:
///
/// * `Persona` — `nombre` no vacío (motivo `nombre vacío`); si la columna
///   `orcid` está presente y no vacía, debe cumplir `XXXX-XXXX-XXXX-XXXX`
///   (motivo `orcid malformado: {orcid}`). Es la única regla que rechaza
///   una fila del CSV crudo del paso-5: «ana garcía» con el orcid
///   `0000-0001-2345-1`.
/// * `Documento` — `titulo` no vacío (motivo `titulo vacío`); `anio`
///   presente y numérico (motivos `anio vacío` y `anio no numérico: {anio}`).
/// * `Tema` — `tema_nombre` no vacío (motivo `tema_nombre vacío`).
/// * `Relacion` — `desde`, `hasta` y `tipo` no vacíos (motivos `desde
///   vacío`, `hasta vacío` y `tipo de relación vacío`).
/// * Cualquier otro `tipo` — motivo `tipo de registro desconocido: {tipo}`
///   (la «label conocida» del esquema del cap-44).
///
/// Las reglas de LOTE —clave natural repetida y `SinSolape` local entre
/// las MEMBER_OF— son de la pieza de validación por lotes del pipeline
/// completo, no de la regla de fila.
pub fn validar_registro(registro: &RegistroCrudo) -> Result<(), String> {
    match registro.tipo.as_str() {
        "Persona" => {
            let nombre = campo(registro, "nombre").unwrap_or("");
            if nombre.trim().is_empty() {
                return Err("nombre vacío".into());
            }
            if let Some(orcid) = campo(registro, "orcid")
                && !orcid.is_empty()
                && !orcid_valido(orcid)
            {
                return Err(format!("orcid malformado: {orcid}"));
            }
            Ok(())
        }
        "Documento" => {
            let titulo = campo(registro, "titulo").unwrap_or("");
            if titulo.trim().is_empty() {
                return Err("titulo vacío".into());
            }
            let anio = campo(registro, "anio").unwrap_or("");
            if anio.trim().is_empty() {
                return Err("anio vacío".into());
            }
            if anio.trim().parse::<i64>().is_err() {
                return Err(format!("anio no numérico: {anio}"));
            }
            Ok(())
        }
        "Tema" => {
            let nombre = campo(registro, "tema_nombre").unwrap_or("");
            if nombre.trim().is_empty() {
                return Err("tema_nombre vacío".into());
            }
            Ok(())
        }
        "Relacion" => {
            let desde = campo(registro, "desde").unwrap_or("");
            if desde.trim().is_empty() {
                return Err("desde vacío".into());
            }
            let hasta = campo(registro, "hasta").unwrap_or("");
            if hasta.trim().is_empty() {
                return Err("hasta vacío".into());
            }
            let tipo = campo(registro, "tipo").unwrap_or("");
            if tipo.trim().is_empty() {
                return Err("tipo de relación vacío".into());
            }
            Ok(())
        }
        otro => Err(format!("tipo de registro desconocido: {otro}")),
    }
}

/// Aplica [`validar_registro`] a todo el lote: `Ok(())` si TODAS las filas
/// pasan, o `Err` con las `(línea_local, motivo)` de las rechazadas, en
/// orden. Fail-fast por lote: la fila sucia se rechaza y el lote sigue —
/// el pipeline nunca aborta por una fila.
///
/// `línea_local` es el índice 0-based DENTRO del lote (el primer registro
/// es la 0); el llamador le suma la línea inicial del lote en el fichero
/// para obtener la línea absoluta que pide el contrato (la etapa cargar lo
/// hace por lote).
pub fn validar_lote(lote: &[RegistroCrudo]) -> Result<(), Vec<(usize, String)>> {
    let rechazos: Vec<(usize, String)> = lote
        .iter()
        .enumerate()
        .filter_map(|(i, registro)| validar_registro(registro).err().map(|motivo| (i, motivo)))
        .collect();
    if rechazos.is_empty() {
        Ok(())
    } else {
        Err(rechazos)
    }
}

/// La clave natural de una entidad nombrada por su nombre COMPLETO:
/// [`normalizar_nombre`] (la normalización del entity resolution, contrato
/// §2) sobre el nombre crudo. Temas usan su `tema_nombre` y Documentos su
/// `titulo` — «Recuperación aumentada con grafo(s)» normalizan distinto y
/// el ER las une después, no la clave. Es la clave del `HashMap` `ids` de
/// la etapa mapear: mismo nombre normalizado → mismo id — recargar no
/// duplica.
fn clave_natural(nombre: &str) -> String {
    normalizar_nombre(nombre)
}

/// La clave natural de una PERSONA: el PRIMER TOKEN del nombre normalizado.
///
/// DECISIÓN documentada — el contrato §2 pinea el mapeo «las 4 variantes
/// de Ana a UNA clave natural» y el resultado final de «9 personas»; la
/// normalización completa distingue «ana» de «ana garcia», así que la
/// clave que las UNE es el primer token (la misma señal que el
/// `mismo_primer_token` del ER, contrato §5): «Ana», «ana garcia», «Ana
/// G.», «ana garcía» y «Ana García (Universidad de Lira)» caen en la clave
/// «ana», y los dos Beto en «beto». La diferenciación fina (abreviaturas,
/// paréntesis) sigue siendo del entity resolution de la fusión.
fn clave_persona(nombre: &str) -> String {
    normalizar_nombre(nombre)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Las columnas dadas de un registro presentes y NO vacías, como
/// `(columna, Value::String)`: la lista de props que el mapear rellena al
/// crear o actualizar. Una columna vacía no aporta prop (no se rellena
/// basura).
fn props_de_campos(
    registro: &RegistroCrudo,
    nombres: &[&'static str],
) -> Vec<(&'static str, Value)> {
    nombres
        .iter()
        .filter_map(|nombre| {
            campo(registro, nombre)
                .filter(|valor| !valor.is_empty())
                .map(|valor| (*nombre, Value::String(valor.to_string())))
        })
        .collect()
}

/// Rellena las props AUSENTES de un nodo existente sin sobrescribir nada
/// (la «sobrescritura cuidada» del contrato §2/§5 aplicada al upsert del
/// mapear: las props iguales se conservan, las ausentes se rellenan, las
/// presentes con otro valor NO se tocan — el conflicto se declara en la
/// fusión, no aquí).
///
/// Como `put_node` rechaza ids repetidos y `delete_node` borra en cascada
/// las aristas incidentes, el patrón es el de [`fusionar_cluster`]:
/// snapshot de aristas → delete → re-put con la unión de props → re-put de
/// las aristas con el MISMO id (reapuntar no borra).
fn rellenar_props_ausentes(
    store: &mut MemoryStore,
    id: usize,
    nuevas: &[(&str, Value)],
) -> Result<(), String> {
    let nodo = store
        .get_node(id)
        .ok_or_else(|| format!("nodo {id} inexistente"))?;
    let mut props = nodo.props.clone();
    let mut cambiadas = false;
    for (clave, valor) in nuevas {
        if !props.contains_key(*clave) {
            props.insert((*clave).to_string(), valor.clone());
            cambiadas = true;
        }
    }
    if !cambiadas {
        return Ok(());
    }
    let aristas: Vec<Edge> = store
        .out_edges(id)
        .into_iter()
        .chain(store.in_edges(id))
        .map(|eid| {
            store
                .get_edge(eid)
                .expect("arista viva en el snapshot")
                .clone()
        })
        .collect();
    let labels = nodo.labels.clone();
    store.delete_node(id);
    store
        .put_node(Node { id, labels, props })
        .expect("el id quedó libre tras el borrado");
    for arista in aristas {
        store
            .put_edge(arista)
            .expect("extremos vivos: la arista vuelve igual");
    }
    Ok(())
}

/// Mapea UN registro crudo al modelo (la etapa MAPEAR del pipeline,
/// contrato §2): interpreta la fila según su `tipo` y la carga en el
/// store con un id NUEVO asignado por clave natural (nombre normalizado →
/// id), reutilizando el id si la clave ya existe — la IDEMPOTENCIA por
/// clave natural del contrato (decisión #4).
///
/// Clave natural exacta por tipo:
///
/// * `Persona` — el PRIMER TOKEN de `normalizar_nombre(nombre)` (véase
///   [`clave_persona`]: «Ana» y «ana garcia» son la MISMA persona); crea
///   una Persona con props `nombre`, `orcid` (si viene) y `afiliacion`
///   (si viene); si la clave ya existe, REUTILIZA el id y rellena las
///   props ausentes sin sobrescribir.
/// * `Documento` — `normalizar_nombre(titulo)` (completo); crea un
///   Documento con labels `["Documento", subtipo]` (`Paper`/`Nota`/
///   `Informe` según la columna `tipo`, o solo `Documento` si no viene) y
///   props `titulo` + `anio` (Int). Si la clave ya existe, no-op: el
///   Documento nace completo.
/// * `Tema` — `normalizar_nombre(tema_nombre)` (completo); crea un Tema
///   con prop `nombre`. Idempotente como los anteriores.
/// * `Relacion` — resuelve `desde` y `hasta` POR CLAVE NATURAL contra el
///   mapa `ids` (la clave del extremo depende del `rel_tipo`: los
///   extremos persona usan [`clave_persona`], documento/tema
///   [`clave_natural`]) y crea la arista dirigida con `tipo` como label y
///   las props `desde_anio`/`hasta_anio` (Int) si vienen. Las AUTHORED
///   leen además la columna `order` (el orden de firma del cap-41) y la
///   cargan como prop de arista `order` (Int) cuando viene. Los extremos
///   deben existir YA en el mapa (personas/documentos/temas de sus
///   ficheros); la creación de entidades referenciadas SOLO por aristas
///   (Organizacion, Proyecto, Conferencia, Reseña) es de la pieza del
///   pipeline completo.
///
/// DECISIÓN documentada — la columna `afiliacion` de personas.csv se
/// mapea a la PROPIEDAD `afiliacion` de la Persona, NO a una arista
/// `MEMBER_OF`. El contrato §2 pinea las MEMBER_OF en relaciones.csv (el
/// duplicado exacto y la solapada `[2020,2023)` viven en ese fichero y el
/// conteo de aristas del contrato, 158, no deja hueco para aristas
/// derivadas de columnas); la prop, en cambio, alimenta la métrica del
/// canónico («más props no vacías», fusión §5) y el test de fusión del
/// módulo ya la usa como prop.
///
/// Fail-fast: la fila pasa por [`validar_registro`] ANTES de tocar el
/// store — mapear nunca interpreta basura.
pub fn mapear_registro(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    validar_registro(registro)?;
    match registro.tipo.as_str() {
        "Persona" => mapear_persona(registro, store, ids),
        "Documento" => mapear_documento(registro, store, ids),
        "Tema" => mapear_tema(registro, store, ids),
        "Relacion" => mapear_relacion(registro, store, ids),
        otro => Err(format!("tipo de registro desconocido: {otro}")),
    }
}

fn mapear_persona(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let nombre = campo(registro, "nombre").unwrap_or("").to_string();
    let clave = clave_persona(&nombre);
    if let Some(&id) = ids.get(&clave) {
        let nuevas = props_de_campos(registro, &["orcid", "afiliacion"]);
        return rellenar_props_ausentes(store, id, &nuevas);
    }
    let id = siguiente_id_libre(store);
    let mut nodo = Node::new(id, "Persona").with_prop("nombre", Value::String(nombre));
    for (prop, valor) in props_de_campos(registro, &["orcid", "afiliacion"]) {
        nodo = nodo.with_prop(prop, valor);
    }
    store.put_node(nodo).map_err(|e| e.to_string())?;
    ids.insert(clave, id);
    Ok(())
}

fn mapear_documento(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let titulo = campo(registro, "titulo").unwrap_or("").to_string();
    let clave = clave_natural(&titulo);
    if ids.contains_key(&clave) {
        return Ok(());
    }
    let id = siguiente_id_libre(store);
    let mut nodo = Node::new(id, "Documento").with_prop("titulo", Value::String(titulo));
    if let Some(anio) = campo(registro, "anio") {
        let anio: i64 = anio
            .trim()
            .parse()
            .map_err(|_| format!("anio no numérico: {anio}"))?;
        nodo = nodo.with_prop("anio", Value::Int(anio));
    }
    if let Some(subtipo) = campo(registro, "tipo").filter(|t| !t.is_empty()) {
        nodo.labels.push(subtipo.to_string());
    }
    store.put_node(nodo).map_err(|e| e.to_string())?;
    ids.insert(clave, id);
    Ok(())
}

fn mapear_tema(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let nombre = campo(registro, "tema_nombre").unwrap_or("").to_string();
    let clave = clave_natural(&nombre);
    if ids.contains_key(&clave) {
        return Ok(());
    }
    let id = siguiente_id_libre(store);
    let nodo = Node::new(id, "Tema").with_prop("nombre", Value::String(nombre));
    store.put_node(nodo).map_err(|e| e.to_string())?;
    ids.insert(clave, id);
    Ok(())
}

/// La estrategia de clave natural de un extremo de arista según el
/// `rel_tipo` (contrato §2, `MAPEO_COLUMNAS`: «tipo_relacion/de/a →
/// label/extremos»): los extremos persona se resuelven con
/// [`clave_persona`] (primer token), los de documento/tema con
/// [`clave_natural`] (nombre completo). Los tipos cuyos extremos son
/// entidades sin fichero propio (MEMBER_OF → Organizacion, WORKED_ON →
/// Proyecto, PUBLICADO_EN → Conferencia, REALIZA/SOBRE/CONTRARRESTA →
/// Reseña) se declaran aquí para documentar el extremo esperado y la
/// resolución fallará con «sin mapear» hasta la pieza del pipeline
/// completo, que CREA esas entidades.
fn clave_extremo(rel_tipo: &str, es_desde: bool, nombre: &str) -> Option<String> {
    let (tipo_desde, tipo_hasta) = match rel_tipo {
        "AUTHORED" => ("Persona", "Documento"),
        "CITES" => ("Documento", "Documento"),
        "ABOUT" => ("Documento", "Tema"),
        "SUB_TEMA_DE" => ("Tema", "Tema"),
        "MENTIONS" => ("Documento", "Persona"),
        "MEMBER_OF" => ("Persona", "Organizacion"),
        "WORKED_ON" => ("Persona", "Proyecto"),
        "PUBLICADO_EN" => ("Documento", "Conferencia"),
        "REALIZA" => ("Persona", "Resena"),
        "SOBRE" => ("Resena", "Documento"),
        "CONTRARRESTA" => ("Resena", "Resena"),
        _ => return None,
    };
    let tipo = if es_desde { tipo_desde } else { tipo_hasta };
    Some(match tipo {
        "Persona" => clave_persona(nombre),
        _ => clave_natural(nombre),
    })
}

/// Resuelve UN extremo de arista por clave natural: la clave que dicta
/// el `rel_tipo` ([`clave_extremo`]) contra el mapa `ids`. Estricto: si
/// la entidad no está mapeada, `Err` con el lado y el nombre — la
/// creación de extremos referenciados solo por aristas es de la pieza del
/// pipeline completo.
fn resolver_extremo(
    rel_tipo: &str,
    es_desde: bool,
    nombre: &str,
    ids: &HashMap<String, usize>,
) -> Result<usize, String> {
    let lado = if es_desde { "desde" } else { "hasta" };
    let clave = clave_extremo(rel_tipo, es_desde, nombre)
        .ok_or_else(|| format!("tipo de relación desconocido: {rel_tipo}"))?;
    ids.get(&clave)
        .copied()
        .ok_or_else(|| format!("extremo '{lado}' sin mapear: {nombre}"))
}

fn mapear_relacion(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let desde = campo(registro, "desde").unwrap_or("");
    let hasta = campo(registro, "hasta").unwrap_or("");
    let rel_tipo = campo(registro, "tipo").unwrap_or("").to_string();
    let desde_id = resolver_extremo(&rel_tipo, true, desde, ids)?;
    let hasta_id = resolver_extremo(&rel_tipo, false, hasta, ids)?;
    let es_authered = rel_tipo == "AUTHORED";
    let mut arista = Edge::new(store.edge_count(), desde_id, hasta_id, rel_tipo);
    for (columna, valor) in [
        ("desde_anio", campo(registro, "desde_anio")),
        ("hasta_anio", campo(registro, "hasta_anio")),
    ] {
        if let Some(texto) = valor.filter(|v| !v.is_empty()) {
            let anio: i64 = texto
                .trim()
                .parse()
                .map_err(|_| format!("{columna} no numérico: {texto}"))?;
            arista = arista.with_prop(columna, Value::Int(anio));
        }
    }
    if es_authered && let Some(texto) = campo(registro, "order").filter(|v| !v.is_empty()) {
        let orden: i64 = texto
            .trim()
            .parse()
            .map_err(|_| format!("order no numérico: {texto}"))?;
        arista = arista.with_prop("order", Value::Int(orden));
    }
    store.put_edge(arista).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const CRUDOS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../datasets/kb-lira/paso-5/crudos"
    );

    fn leer(nombre: &str) -> String {
        fs::read_to_string(format!("{CRUDOS}/{nombre}")).unwrap_or_else(|e| {
            panic!("no se pudo leer datasets/kb-lira/paso-5/crudos/{nombre}: {e}")
        })
    }

    /// Los ficheros crudos existen, con las filas y la suciedad deliberada
    /// que el contrato §2 pinea: 3 duplicados exactos (Beto, DOC_RAG y una
    /// MEMBER_OF), 4 variantes de Ana (una con orcid malformado), 1 typo de
    /// título, el fantasma «memoria de agentes» (26/61) y 1 MEMBER_OF
    /// solapada `[2020,2023)`.
    #[test]
    fn los_crudos_del_paso5_tienen_las_filas_y_la_suciedad_del_contrato() {
        let personas = leer("personas.csv");
        let filas_personas: Vec<&str> = personas.lines().collect();
        assert_eq!(filas_personas.len(), 15, "cabecera + 14 filas");
        assert_eq!(filas_personas[0], "nombre,orcid,afiliacion");

        // Las 9 personas canónicas del builder (nombres exactos del paso-4).
        for nombre in [
            "Ana", "Beto", "Carla", "Dani", "Elena", "Fabio", "Gaby", "Hugo", "Iris",
        ] {
            assert!(
                personas
                    .lines()
                    .any(|l| l.starts_with(&format!("{nombre},"))),
                "falta la persona canónica {nombre}"
            );
        }
        // 4 variantes de Ana: minúsculas, abreviatura, tilde (con orcid
        // malformado) y con paréntesis de afiliación.
        for variante in [
            "ana garcia",
            "Ana G.",
            "ana garcía,0000-0001-2345-1,",
            "Ana García (Universidad de Lira)",
        ] {
            assert!(
                personas.contains(variante),
                "falta la variante de Ana: {variante}"
            );
        }
        // Duplicado EXACTO de Beto (rechazado por clave natural repetida).
        assert_eq!(
            personas
                .matches("Beto,0000-0002-3456-0002,Instituto Neurónica")
                .count(),
            2,
            "el duplicado exacto de Beto"
        );

        let documentos = leer("documentos.csv");
        let filas_documentos: Vec<&str> = documentos.lines().collect();
        assert_eq!(filas_documentos.len(), 39, "cabecera + 38 filas");
        assert_eq!(filas_documentos[0], "titulo,anio,tipo");
        // DOC_RAG: duplicado exacto de «Recuperación aumentada con grafos».
        assert_eq!(
            documentos
                .matches("Recuperación aumentada con grafos,2025,Paper")
                .count(),
            2,
            "el duplicado exacto del documento RAG"
        );
        // El casi-duplicado con typo en el título (se fusionará por ER).
        assert!(
            documentos.contains("Recuperación aumentada con grafo,2025,Paper"),
            "el typo de título"
        );

        let temas = leer("temas.csv");
        let filas_temas: Vec<&str> = temas.lines().collect();
        assert_eq!(filas_temas.len(), 10, "cabecera + 9 filas");
        assert_eq!(filas_temas[0], "tema_nombre");
        // El fantasma del cap-44: «memoria de agentes» en dos filas (26 y 61).
        assert_eq!(
            temas.matches("memoria de agentes").count(),
            2,
            "el fantasma tema 26/61"
        );

        let relaciones = leer("relaciones.csv");
        let filas_relaciones: Vec<&str> = relaciones.lines().collect();
        assert_eq!(filas_relaciones.len(), 161, "cabecera + 160 filas");
        assert_eq!(
            filas_relaciones[0], "desde,hasta,tipo,desde_anio,hasta_anio,order",
            "la columna `order` transporta el orden de firma de las AUTHORED"
        );
        // Duplicado EXACTO de la MEMBER_OF 52 (Ana → Universidad de Lira).
        assert_eq!(
            relaciones
                .matches("Ana,Universidad de Lira,MEMBER_OF,2018,")
                .count(),
            2,
            "el duplicado exacto de MEMBER_OF"
        );
        // La MEMBER_OF solapada [2020,2023) contra la 53 [2018,2024).
        assert!(
            relaciones.contains("Beto,Instituto Neurónica,MEMBER_OF,2020,2023"),
            "la MEMBER_OF solapada [2020,2023)"
        );
    }

    /// `desde_csv` parsea un fichero crudo completo (cabecera → columnas,
    /// filas → registros con `tipo` fijo) reutilizando `partir_csv` del
    /// cap-32, y reporta filas inválidas con su número de línea.
    #[test]
    fn desde_csv_parsea_un_fichero_crudo_con_tipo_fijo_y_reporta_lineas_invalidas() {
        let contenido = leer("personas.csv");
        let datos = DatosCrudos::desde_csv(&contenido, "Persona").expect("personas.csv parsea");
        assert_eq!(datos.registros.len(), 14, "cabecera + 14 filas");
        let ana = &datos.registros[0];
        assert_eq!(ana.tipo, "Persona");
        assert_eq!(
            ana.campos,
            vec![
                ("nombre".to_string(), "Ana".to_string()),
                ("orcid".to_string(), "0000-0001-2345-0001".to_string()),
                ("afiliacion".to_string(), "Universidad de Lira".to_string()),
            ]
        );
        // La variante con orcid malformado conserva el valor tal cual.
        let variante_tilde = &datos.registros[11];
        assert_eq!(
            variante_tilde.campos[0],
            ("nombre".to_string(), "ana garcía".to_string())
        );
        assert_eq!(
            variante_tilde.campos[1],
            ("orcid".to_string(), "0000-0001-2345-1".to_string())
        );

        // Sin cabecera → el lote es inválido (aborta, como la cabecera cap-32).
        let err = DatosCrudos::desde_csv("", "Persona").unwrap_err();
        assert!(
            matches!(err, ErrorIngesta::LoteInvalido { ref motivo } if motivo.contains("cabecera")),
            "esperaba LoteInvalido por fichero vacío, fue {err}"
        );

        // Fila con número de columnas distinto → LineaInvalida con su línea.
        let err = DatosCrudos::desde_csv("a,b\n1,2\n3,4,5\n", "X").unwrap_err();
        match err {
            ErrorIngesta::LineaInvalida { linea, motivo } => {
                assert_eq!(linea, 3, "la línea 3 tiene 3 campos");
                assert!(motivo.contains("3 campos"), "motivo: {motivo}");
            }
            otro => panic!("esperaba LineaInvalida en la línea 3, fue {otro}"),
        }
    }

    /// `normalizar_nombre` (contrato §2) une las variantes ortográficas de
    /// Ana: minúsculas, tildes y espacios no impiden que «Ana García»,
    /// «ana garcia» y «ANA GARCÍA» normalicen a la MISMA cadena (la base
    /// común «ana garcia»), mientras que la abreviatura «Ana G.» conserva
    /// su forma corta («ana g.», mismo primer token) y el paréntesis de
    /// afiliación se CONSERVA — el contrato §5 (decisión #5) lo decide con
    /// bigramas, no con la normalización.
    #[test]
    fn la_normalizacion_sin_tildes_une_variantes_de_ana() {
        let base = normalizar_nombre("Ana García");
        assert_eq!(base, "ana garcia");
        assert_eq!(normalizar_nombre("ana garcia"), base);
        assert_eq!(normalizar_nombre("ANA GARCÍA"), base);
        assert_eq!(
            normalizar_nombre("  Ana   García "),
            base,
            "trim + espacios colapsados"
        );
        assert_eq!(normalizar_nombre("Ana G."), "ana g.");
        assert_eq!(
            normalizar_nombre("Ana García (Universidad de Lira)"),
            "ana garcia (universidad de lira)",
            "el paréntesis se conserva: lo decide la métrica, no la normalización"
        );
    }

    /// La distancia de Levenshtein distingue «ana garcia» de «carla
    /// mendez» (solo comparten «ara»: 9 ediciones — pin del valor MEDIDO,
    /// criterio de parada honesto del contrato §8) y mide 1 entre «ana
    /// garcia» y «ana garcía» SIN normalizar (la tilde es una edición) y 0
    /// TRAS normalizar (la normalización ya unió la variante). De paso se
    /// pine el caso de contraste del contrato §5: «ana garcia» vs «ana g.»
    /// = 5 ediciones ≈ 0.5 de similitud — donde los bigramas (pieza 4)
    /// aciertan y Levenshtein falla.
    #[test]
    fn la_distancia_de_levenshtein_distingue_ana_garcia_de_carla_mendez() {
        assert_eq!(distancia_levenshtein("ana garcia", "carla mendez"), 9);
        assert_eq!(distancia_levenshtein("ana garcia", "ana garcía"), 1);
        assert_eq!(
            distancia_levenshtein(
                &normalizar_nombre("ana garcia"),
                &normalizar_nombre("ana garcía")
            ),
            0
        );
        assert_eq!(distancia_levenshtein("ana garcia", "ana g."), 5);
    }

    /// La similitud de Levenshtein es 1.0 para cadenas idénticas (y para
    /// dos vacías, donde el 0/0 se define como la identidad: «vacío ≡
    /// vacío»), y 0.0 cuando no comparten nada (la distancia alcanza el
    /// máximo de la fórmula).
    #[test]
    fn similitud_levenshtein_es_uno_para_iguales_y_cero_sin_solapamiento() {
        assert_eq!(similitud_levenshtein("ana garcia", "ana garcia"), 1.0);
        assert_eq!(similitud_levenshtein("", ""), 1.0);
        assert_eq!(similitud_levenshtein("aaaaa", "bbbbb"), 0.0);
        // El contraste del contrato §5: 1 − 5/10 = 0.5 exacto.
        assert_eq!(similitud_levenshtein("ana garcia", "ana g."), 0.5);
    }

    /// La similitud de bigramas (contrato §2/§5) UNE «ana garcia» y
    /// «Ana G.»: jaccard REAL = 0.4 (4 bigramas compartidos de 10 en la
    /// unión: «an», «na», «a » y « g»). No alcanza la regla primaria del
    /// contrato (0.5) — el contrato §5 predijo 0.56 y el valor medido es
    /// 0.4, pin del criterio honesto §8 — pero la regla compuesta la une:
    /// mismo bloque «a» Y mismo primer token Y 0.4 ≥ 0.25 → arista
    /// `SIMILAR` (donde Levenshtein fallaba, ≈ 0.5).
    #[test]
    fn jaccard_bigramas_une_ana_garcia_con_ana_g() {
        let a = normalizar_nombre("ana garcia");
        let b = normalizar_nombre("Ana G.");
        let j = jaccard_bigramas(&a, &b);
        assert_eq!(j, 0.4, "4/10 — pin del valor real");
        assert!(j < 0.5, "no llega a la regla primaria del contrato");
        assert_eq!(
            bloque_por_inicial(&a),
            bloque_por_inicial(&b),
            "mismo bloque"
        );
        assert!(mismo_primer_token(&a, &b), "mismo primer token");
        assert!(j >= 0.25, "la regla compuesta del contrato la une");
    }

    /// La similitud de bigramas SEPARA «ana garcia» de «carla mendez»:
    /// jaccard REAL = 2/18 ≈ 0.111 (solo comparten «a » y «ar»). El
    /// contrato §5 predijo 0.11 y el valor medido COINCIDE. Bajo el umbral
    /// 0.5 del contrato y sin primer token común → SIN arista.
    #[test]
    fn el_jaccard_de_bigramas_separa_ana_g_de_carla_mendez() {
        let a = normalizar_nombre("ana garcia");
        let b = normalizar_nombre("Carla Méndez");
        let j = jaccard_bigramas(&a, &b);
        assert_eq!(j, 2.0 / 18.0, "pin del valor real");
        assert!(j < 0.5, "umbral del contrato: sin arista");
        assert!(!mismo_primer_token(&a, &b), "primer token distinto");
    }

    /// El blocking por inicial (contrato §2) evita comparaciones. Con las
    /// 14 personas REALES del CSV crudo (las 9 canónicas + 4 variantes de
    /// Ana + el duplicado de Beto), el todo-contra-todo haría 14×13/2 = 91
    /// comparaciones; bloqueando por la primera letra del primer token
    /// normalizado solo se comparan las 5 Anas (bloque «a»: 10 pares) y los
    /// 2 Beto (bloque «b»: 1 par) → **11 comparaciones, 80 evitadas**. El
    /// contrato §5 predijo «78 para 13 personas → ~11»: el ~11 se cumple
    /// EXACTO y el 78 es 91 porque el dataset REAL tiene 14 personas (los
    /// 14 nombres del CSV son la fuente de verdad — criterio de parada
    /// honesto §8).
    #[test]
    fn el_blocking_por_inicial_evita_comparaciones() {
        let personas = leer("personas.csv");
        let nombres: Vec<&str> = personas
            .lines()
            .skip(1)
            .map(|linea| linea.split(',').next().unwrap())
            .collect();
        assert_eq!(nombres.len(), 14, "las 14 filas del CSV crudo");

        let naive = nombres.len() * (nombres.len() - 1) / 2;
        assert_eq!(naive, 91, "14×13/2 = todo-contra-todo");

        let mut por_bloque = std::collections::BTreeMap::<String, usize>::new();
        for nombre in &nombres {
            *por_bloque.entry(bloque_por_inicial(nombre)).or_insert(0) += 1;
        }
        assert_eq!(por_bloque.get("a"), Some(&5), "las 5 Anas comparten bloque");
        assert_eq!(
            por_bloque.get("b"),
            Some(&2),
            "los dos Beto comparten bloque"
        );

        let bloqueadas: usize = por_bloque
            .values()
            .map(|miembros| miembros * (miembros - 1) / 2)
            .sum();
        assert_eq!(bloqueadas, 11, "10 pares de Anas + 1 par de Betos");

        let evitadas = naive - bloqueadas;
        assert_eq!(evitadas, 80, "comparaciones que el blocking no hace");
    }

    /// El grafo de similitud (contrato §2) agrupa las 5 Anas del CSV crudo
    /// en UN cluster: un nodo `Persona` por nombre (ids 0..14) y aristas
    /// `SIMILAR` que conectan «Ana», «ana garcia», «Ana G.», «ana garcía» y
    /// «Ana García (Universidad de Lira)». Las aristas REALES son 7 (6 entre
    /// Anas + 1 entre los Beto duplicados) y el cluster que contiene a la
    /// Ana canónica tiene EXACTAMENTE los 5 ids de las Anas y solo ellos.
    /// De paso se pine el contador del grafo: 11 comparaciones (10 pares de
    /// Anas + 1 par de Betos), no las 91 del todo-contra-todo.
    #[test]
    fn el_grafo_de_similitud_agrupa_las_cinco_anas_en_un_cluster() {
        let nombres: Vec<String> = leer("personas.csv")
            .lines()
            .skip(1)
            .map(|linea| linea.split(',').next().unwrap().to_string())
            .collect();
        assert_eq!(nombres.len(), 14, "las 14 filas del CSV crudo");

        let (store, comparaciones) = construir_grafo_similitud(&nombres);
        assert_eq!(comparaciones, 11, "10 pares de Anas + 1 par de Betos");
        assert_eq!(
            store.edge_count(),
            7,
            "6 aristas entre Anas + 1 entre Betos"
        );

        let clusters = clusters_de_similitud(&store);
        let cluster_anas = clusters
            .iter()
            .find(|c| c.contains(&0))
            .expect("la Ana canónica pertenece a algún cluster");
        assert_eq!(
            cluster_anas.as_slice(),
            &[0, 9, 10, 11, 12],
            "las 5 Anas y solo ellas"
        );
    }

    /// Las componentes conexas del cap-25 REUTILIZADAS dan los clusters de
    /// entidades: sobre el grafo de similitud completo de los 14 nombres del
    /// CSV crudo, los clusters REALES son las 5 Anas juntas (ids 0, 9, 10,
    /// 11 y 12), los 2 Beto juntos (ids 1 y 13) y 7 singletons (Carla, Dani,
    /// Elena, Fabio, Gaby, Hugo e Iris): 9 clusters en total, numerados por
    /// menor miembro y con miembros ascendentes, el orden canónico del
    /// cap-25.
    #[test]
    fn las_componentes_conexas_del_cap25_dan_los_clusters_de_entidades() {
        let nombres: Vec<String> = leer("personas.csv")
            .lines()
            .skip(1)
            .map(|linea| linea.split(',').next().unwrap().to_string())
            .collect();
        assert_eq!(nombres.len(), 14, "las 14 filas del CSV crudo");

        let (store, _) = construir_grafo_similitud(&nombres);
        let clusters = clusters_de_similitud(&store);
        assert_eq!(
            clusters,
            vec![
                vec![0, 9, 10, 11, 12],
                vec![1, 13],
                vec![2],
                vec![3],
                vec![4],
                vec![5],
                vec![6],
                vec![7],
                vec![8],
            ],
            "las 5 Anas juntas, los 2 Beto juntos, el resto singletons"
        );
    }

    /// El blocking dentro del grafo de similitud evita comparaciones: con
    /// los 14 nombres del CSV crudo el todo-contra-todo haría 91 pares; el
    /// grafo solo compara dentro del bloque por inicial → 11 REALES (10
    /// pares de Anas + 1 par de Betos), 80 evitadas. Es el contador del
    /// deliverable «78 → ~11» del contrato §2 medido con el dataset REAL de
    /// 14 personas (91 → 11, criterio de parada honesto §8 — el 78 del
    /// contrato asumía 13 personas).
    #[test]
    fn el_blocking_evita_comparaciones_en_el_grafo() {
        let nombres: Vec<String> = leer("personas.csv")
            .lines()
            .skip(1)
            .map(|linea| linea.split(',').next().unwrap().to_string())
            .collect();
        assert_eq!(nombres.len(), 14, "las 14 filas del CSV crudo");

        let naive = nombres.len() * (nombres.len() - 1) / 2;
        assert_eq!(naive, 91, "14×13/2 = todo-contra-todo");

        let (_, comparaciones) = construir_grafo_similitud(&nombres);
        assert_eq!(
            comparaciones, 11,
            "pin real: 10 pares de Anas + 1 par de Betos"
        );
        assert!(
            comparaciones < naive,
            "el blocking compara menos que el naive"
        );
        assert_eq!(
            naive - comparaciones,
            80,
            "comparaciones que el bloqueo por inicial evita"
        );
    }

    /// La fusión con «sobrescritura cuidada» (contrato §2, decisión de
    /// fusión §5): el cluster de las 5 Anas [0,9,10,11,12] se reduce a UNA
    /// sola Persona — el canónico (id 0: MÁS props no vacías; empate con la
    /// 9 → MENOR id) conserva SU `orcid`, rellena la `afiliacion` ausente,
    /// DECLARA el conflicto del `orcid` distinto de la Ana 10 (gana el
    /// canónico, nunca silencioso) y sus 2 `AUTHORED` al Documento 13 se
    /// REAPUNTAN al canónico (reapuntar no borra: el Documento conserva sus
    /// 2 aristas; los ids 9..12 desaparecen del store).
    #[test]
    fn la_fusion_elige_el_canonico_con_mas_datos_y_reapunta_aristas() {
        let mut store = MemoryStore::new();
        // La 0 es la más rica (2 props no vacías); la 9 empata a 2 props →
        // gana la de MENOR id; la 10 trae el orcid DISTINTO (el conflicto a
        // declarar); la 11 trae un Value::Null que NO cuenta como no vacía;
        // la 12 repite el orcid del canónico (se conserva igual, sin
        // conflicto). El Documento 13 es ajeno al cluster.
        store
            .put_node(
                Node::new(0, "Persona")
                    .with_prop("nombre", Value::String("Ana".into()))
                    .with_prop("orcid", Value::String("0000-0001-2345-0001".into())),
            )
            .unwrap();
        store
            .put_node(
                Node::new(9, "Persona")
                    .with_prop("orcid", Value::String("0000-0001-2345-0001".into()))
                    .with_prop("afiliacion", Value::String("Universidad de Lira".into())),
            )
            .unwrap();
        store
            .put_node(
                Node::new(10, "Persona")
                    .with_prop("orcid", Value::String("0000-0001-2345-9999".into())),
            )
            .unwrap();
        store
            .put_node(
                Node::new(11, "Persona")
                    .with_prop("afiliacion", Value::String("Universidad de Lira".into()))
                    .with_prop("telefono", Value::Null),
            )
            .unwrap();
        store
            .put_node(
                Node::new(12, "Persona")
                    .with_prop("orcid", Value::String("0000-0001-2345-0001".into())),
            )
            .unwrap();
        store
            .put_node(Node::new(13, "Documento").with_prop(
                "titulo",
                Value::String("Recuperación aumentada con grafos".into()),
            ))
            .unwrap();
        store.put_edge(Edge::new(0, 9, 13, "AUTHORED")).unwrap();
        store.put_edge(Edge::new(1, 12, 13, "AUTHORED")).unwrap();
        assert_eq!(store.node_count(), 6);
        assert_eq!(store.edge_count(), 2);

        let nombres = vec![
            "Ana".to_string(),
            "ana garcia".to_string(),
            "Ana G.".to_string(),
            "ana garcía".to_string(),
            "Ana García (Universidad de Lira)".to_string(),
        ];
        let informe = fusionar_cluster(&mut store, &[0, 9, 10, 11, 12], &nombres);

        // El informe con los conteos REALES.
        assert_eq!(informe.fusiones, 4, "5 miembros − 1 canónico");
        assert_eq!(
            informe.nodos_finales, 2,
            "la Persona canónica + el Documento"
        );
        assert_eq!(
            informe.aristas_reapuntadas, 2,
            "las 2 AUTHORED de las Anas 9 y 12"
        );
        assert_eq!(informe.conflictos.len(), 1, "solo el orcid distinto");
        assert_eq!(
            informe.conflictos[0],
            ConflictoFusion {
                propiedad: "orcid".into(),
                canónico: "0000-0001-2345-0001".into(),
                descartado: "0000-0001-2345-9999".into(),
            },
            "gana el canónico y el descarte se DECLARA (nunca silencioso)"
        );

        // UNA sola Persona final: el canónico 0 con la unión de props sin
        // conflicto (afiliacion rellenada) y SU orcid conservado.
        assert_eq!(store.node_count(), 2);
        let canonico = store.get_node(0).expect("el canónico vive");
        assert_eq!(canonico.labels, vec!["Persona".to_string()]);
        assert_eq!(
            canonico.props.get("nombre"),
            Some(&Value::String("Ana".into())),
            "conserva su nombre"
        );
        assert_eq!(
            canonico.props.get("orcid"),
            Some(&Value::String("0000-0001-2345-0001".into())),
            "conserva SU valor: gana el canónico"
        );
        assert_eq!(
            canonico.props.get("afiliacion"),
            Some(&Value::String("Universidad de Lira".into())),
            "la prop ausente se rellena (sin conflicto)"
        );
        assert_eq!(canonico.props.len(), 3);
        for id in [9, 10, 11, 12] {
            assert!(store.get_node(id).is_none(), "el nodo {id} se borró");
        }

        // El Documento sigue con sus AUTHORED, reapuntadas al canónico.
        let documento = store.get_node(13).expect("el Documento no se toca");
        assert_eq!(
            documento.props.get("titulo"),
            Some(&Value::String("Recuperación aumentada con grafos".into()))
        );
        assert_eq!(store.edge_count(), 2, "reapuntar no borra");
        for arista in store.iter_edges() {
            assert_eq!(arista.label, "AUTHORED");
            assert_eq!(arista.source, 0, "reapuntada al canónico");
            assert_eq!(arista.target, 13, "el Documento conserva su arista");
        }
    }

    /// La etapa MAPEAR es idempotente por clave natural (contrato §2,
    /// decisión #4): «Ana» y «ana garcia» normalizan a la MISMA clave
    /// («ana garcia») y mapean al MISMO nodo — una sola Persona que
    /// conserva el `orcid` de la primera fila; un Documento mapeado dos
    /// veces sigue siendo UN nodo con su label `Paper`; y la fila de
    /// «ana garcía» con orcid malformado (`0000-0001-2345-1`) se rechaza
    /// con motivo ANTES de tocar el store.
    #[test]
    fn mapear_registros_personas_es_idempotente_por_clave_natural() {
        let mut store = MemoryStore::new();
        let mut ids = std::collections::HashMap::new();

        let ana = RegistroCrudo {
            tipo: "Persona".into(),
            campos: vec![
                ("nombre".into(), "Ana".into()),
                ("orcid".into(), "0000-0001-2345-0001".into()),
                ("afiliacion".into(), "Universidad de Lira".into()),
            ],
        };
        mapear_registro(&ana, &mut store, &mut ids).expect("Ana es válida");
        assert_eq!(store.node_count(), 1);

        let ana_garcia = RegistroCrudo {
            tipo: "Persona".into(),
            campos: vec![
                ("nombre".into(), "ana garcia".into()),
                ("orcid".into(), String::new()),
                ("afiliacion".into(), String::new()),
            ],
        };
        mapear_registro(&ana_garcia, &mut store, &mut ids).expect("misma clave: reutiliza");
        assert_eq!(
            store.node_count(),
            1,
            "idempotencia: «Ana» y «ana garcia» son UNA persona"
        );
        assert_eq!(ids.len(), 1, "una sola clave natural");
        let persona = store.get_node(0).expect("la persona vive");
        assert_eq!(persona.labels, vec!["Persona".to_string()]);
        assert_eq!(
            persona.props.get("nombre"),
            Some(&Value::String("Ana".into())),
            "conserva el nombre de la primera fila"
        );
        assert_eq!(
            persona.props.get("orcid"),
            Some(&Value::String("0000-0001-2345-0001".into())),
            "la prop ausente se rellena sin sobrescribir"
        );

        let documento = RegistroCrudo {
            tipo: "Documento".into(),
            campos: vec![
                (
                    "titulo".into(),
                    "Grafos de conocimiento para agentes".into(),
                ),
                ("anio".into(), "2021".into()),
                ("tipo".into(), "Paper".into()),
            ],
        };
        mapear_registro(&documento, &mut store, &mut ids).expect("documento válido");
        mapear_registro(&documento, &mut store, &mut ids).expect("idempotente: mismo id");
        assert_eq!(
            store.node_count(),
            2,
            "1 persona + 1 documento: mapear dos veces no duplica"
        );
        let doc = store.get_node(1).expect("el documento vive");
        assert_eq!(
            doc.labels,
            vec!["Documento".to_string(), "Paper".to_string()],
            "el subtipo Paper va como label (contrato §2: tipo → :LABEL)"
        );
        assert_eq!(
            doc.props.get("titulo"),
            Some(&Value::String("Grafos de conocimiento para agentes".into()))
        );
        assert_eq!(doc.props.get("anio"), Some(&Value::Int(2021)));

        let malformada = RegistroCrudo {
            tipo: "Persona".into(),
            campos: vec![
                ("nombre".into(), "ana garcía".into()),
                ("orcid".into(), "0000-0001-2345-1".into()),
                ("afiliacion".into(), String::new()),
            ],
        };
        let motivo = mapear_registro(&malformada, &mut store, &mut ids).unwrap_err();
        assert!(
            motivo.contains("orcid"),
            "el motivo nombra el orcid malformado: {motivo}"
        );
        assert_eq!(
            store.node_count(),
            2,
            "la fila inválida se rechaza sin tocar el store"
        );
    }
}

// ───────────────────────────────────────────────────────────────────────
// Pieza PIPELINE COMPLETO: HistorialIngesta + cargar_paso5().
// Contrato §2, pasos 13-15 del patrón de troceo.
// ───────────────────────────────────────────────────────────────────────

/// Un evento del historial de ingesta: el transaction-time de la carga
/// (la deuda del cap-43 cobrada). La ingesta escribe SU WAL del modelo:
/// append-only, el ts lo decide el histórico, nunca el llamador.
///
/// Los seis eventos del contrato §2, con sus datos:
/// * `CargaIniciada` — la carga empieza.
/// * `LoteValidado` — un lote pasó la validación (con su número).
/// * `RegistroRechazado` — una fila violó una regla de lote: el pipeline
///   sigue (rechazo selectivo) y la fila lleva su línea y su motivo.
/// * `FusionEntidad` — el entity resolution fusionó un cluster: la
///   canónica y las descartadas (por nombre).
/// * `ConflictoDeclarado` — la fusión descartó un valor distinto del
///   canónico y lo DECLARA (nada se sobrescribe en silencio).
/// * `CargaCompletada` — la carga termina con los conteos finales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoIngesta {
    /// La carga empieza: primer evento del histórico.
    CargaIniciada,
    /// Un lote se validó (con o sin rechazos): el número es 1-based,
    /// global a la carga (los ficheros se procesan en serie).
    LoteValidado { lote: usize },
    /// Una fila se rechazó: su número de línea en el fichero (1 =
    /// cabecera) y el motivo del rechazo. El lote sigue.
    RegistroRechazado { linea: usize, motivo: String },
    /// Un cluster de duplicados se fusionó: la entidad canónica (que
    /// conserva su id y su nombre) y las descartadas (por nombre crudo).
    FusionEntidad {
        canónica: String,
        descartadas: Vec<String>,
    },
    /// La fusión descartó un valor distinto del canónico y lo declara:
    /// propiedad, valor conservado y valor descartado.
    ConflictoDeclarado {
        propiedad: String,
        canónico: String,
        descartado: String,
    },
    /// La carga termina: los conteos finales del grafo.
    CargaCompletada { nodos: usize, aristas: usize },
}

impl fmt::Display for EventoIngesta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventoIngesta::CargaIniciada => write!(f, "CargaIniciada"),
            EventoIngesta::LoteValidado { lote } => write!(f, "LoteValidado (lote {lote})"),
            EventoIngesta::RegistroRechazado { linea, motivo } => {
                write!(f, "RegistroRechazado (línea {linea}: {motivo})")
            }
            EventoIngesta::FusionEntidad {
                canónica,
                descartadas,
            } => {
                write!(
                    f,
                    "FusionEntidad (canónica: {canónica}, descartadas: {})",
                    descartadas.join(", ")
                )
            }
            EventoIngesta::ConflictoDeclarado {
                propiedad,
                canónico,
                descartado,
            } => write!(
                f,
                "ConflictoDeclarado ({propiedad}: {canónico} ≠ {descartado})"
            ),
            EventoIngesta::CargaCompletada { nodos, aristas } => {
                write!(f, "CargaCompletada ({nodos} nodos, {aristas} aristas)")
            }
        }
    }
}

/// El historial append-only de la carga (transaction-time): el «WAL del
/// modelo» que la ingesta escribe automáticamente. La FORMA del
/// `HistoricoAfiliaciones` del cap-43 REPLICADA (sin tocar cap-43): solo se
/// pueden AÑADIR eventos y el `ts` lo asigna el propio histórico, nunca el
/// llamador — el `registrar` devuelve el ts asignado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorialIngesta {
    /// Los eventos en orden de registro, con su ts monótono (1, 2, …).
    eventos: Vec<(u64, EventoIngesta)>,
    /// El siguiente ts a asignar: la fuente de la monotonía.
    siguiente_ts: u64,
}

impl HistorialIngesta {
    /// Historial vacío, listo para registrar desde el ts 1.
    pub fn nueva() -> Self {
        HistorialIngesta {
            eventos: Vec::new(),
            siguiente_ts: 1,
        }
    }

    /// Registra un evento: le asigna el siguiente ts monótono (append-only,
    /// el ts lo decide el histórico, NUNCA el llamador) y lo devuelve.
    pub fn registrar(&mut self, evento: EventoIngesta) -> u64 {
        let ts = self.siguiente_ts;
        self.siguiente_ts += 1;
        self.eventos.push((ts, evento));
        ts
    }

    /// Los eventos en orden de registro, con su ts: para tests, el informe
    /// y la exportación a CSV de las piezas siguientes.
    pub fn eventos(&self) -> &[(u64, EventoIngesta)] {
        &self.eventos
    }
}

impl fmt::Display for HistorialIngesta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Historial de ingesta — {} eventos", self.eventos.len())?;
        writeln!(f, "{}", "─".repeat(48))?;
        for (ts, evento) in &self.eventos {
            writeln!(f, "ts {:>2}: {evento}", ts)?;
        }
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────────
// El pipeline completo, pieza a pieza. Las piezas 8-10 (validar_registro y
// mapear_registro) siguen siendo los ladrillos; el pipeline las ensambla y
// decide los tres casos que el contrato §2 pinea y que los ladrillos a
// solas no resuelven:
//
// 1. La fila con orcid malformado NO se rechaza en el pipeline (los 4
//    rechazos pineados son Beto/DOC_RAG/MEMBER_OF/solapada): llega a la
//    FUSIÓN como nodo separado y el conflicto se DECLARA — «el orcid
//    malformado se descarta con motivo (la fusión salva la Unicidad)».
//    `validar_registro` mantiene su contrato unitario (la rechaza: el test
//    de mapear_registro lo pinea); el pipeline la deja pasar
//    DELIBERADAMENTE.
// 2. El duplicado EXACTO de Tema (el fantasma 26/61) NO se rechaza como
//    «clave natural repetida»: es EL caso que el entity resolution cura
//    (decisión #11 — «el ER no sabe de esquema, sabe de NOMBRES»);
//    rechazarlo impediría la cura y rompería el pin de 6 fusiones. La regla
//    de clave repetida se aplica a Personas, Documentos y Relaciones.
// 3. La métrica de similitud es una DECISIÓN (decisión #5): jaccard de
//    bigramas para nombres cortos (personas, temas — abreviaturas y
//    paréntesis); similitud de Levenshtein ≥ 0.9 para TÍTULOS de documento
//    (un typo es UNA edición ≈ 0.97; el siguiente título legítimo más
//    cercano está a 0.77 — jaccard sobrefunde títulos largos: 0.63 entre
//    títulos legítimos distintos).
// ───────────────────────────────────────────────────────────────────────

/// ¿Qué regla de similitud usa el entity resolution para UN tipo de
/// entidad? (véase la decisión 3 del bloque anterior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReglaSimilitud {
    /// Jaccard de bigramas con refuerzo de primer token (la regla del
    /// contrato §2/§5): nombres cortos — personas y temas.
    Bigramas,
    /// Similitud de Levenshtein ≥ 0.9: títulos largos — documentos.
    Levenshtein,
}

/// La validación de FILA del pipeline: [`validar_registro`] (las reglas de
/// fila de la pieza 10) con UNA excepción deliberada — el motivo
/// `orcid malformado` de una Persona NO se rechaza.
///
/// Por qué: el contrato §2 pinea 4 rechazos (los duplicados exactos de
/// Beto/DOC_RAG/MEMBER_OF y la solapada) y la fila «ana garcía» con el
/// orcid `0000-0001-2345-1` NO está entre ellos; la fila de fusión del
/// contrato dice textualmente que «el orcid malformado de la variante de
/// Ana se descarta con motivo (el canónico tiene el bien formado — la
/// fusión salva la `Unicidad` que la fila suelta habría violado)»: para que
/// la fusión lo descarte DECLARÁNDOLO, la fila tiene que llegar a la fusión
/// como nodo separado. `validar_registro` mantiene su contrato unitario
/// (la rechaza: el test `mapear_registros_personas_es_idempotente_por_clave_natural`
/// lo pinea); el pipeline decide pasarla porque el contrato manda que el
/// conflicto se DECLARE, no que la fila desaparezca en silencio.
fn validar_fila_pipeline(registro: &RegistroCrudo) -> Result<(), String> {
    match validar_registro(registro) {
        Err(motivo) if registro.tipo == "Persona" && motivo.starts_with("orcid malformado") => {
            Ok(())
        }
        otro => otro,
    }
}

/// La validación de LOTE del pipeline: las reglas de fila
/// ([`validar_fila_pipeline`]) + clave natural repetida + `SinSolape`
/// LOCAL entre las MEMBER_OF del lote (contrato §2, «write-time en dos
/// niveles»). Devuelve, por fila del lote, `None` si se acepta o el motivo
/// si se rechaza; el llamador le añade la línea absoluta del fichero.
///
/// Reglas exactas:
/// * **Clave natural repetida** — la MISMA fila ya aceptada (el «duplicado
///   EXACTO» del contrato): se aplica a Personas (mismo nombre crudo),
///   Documentos (mismo título crudo) y Relaciones (misma fila cruda
///   completa). Los TEMAS se excluyen DELIBERADAMENTE: el duplicado exacto
///   de Tema es el fantasma 26/61 y la decisión #11 manda que la ingesta lo
///   CURE con la fusión del entity resolution, no con un rechazo.
/// * **SinSolape local** — entre las MEMBER_OF del lote con la MISMA
///   persona y organización, con la convención de intervalo del cap-43
///   `[desde, hasta)` (hasta ausente = abierto): la solapada se rechaza con
///   motivo `solape con la {id}` donde `{id}` es la arista YA aceptada que
///   solapa (el id que la arista tendrá al cargarse: los ids del pipeline
///   son el orden de inserción, 0-based).
///   Una MEMBER_OF aceptada de un lote, para el `SinSolape` local: el id que
///   la arista tendrá al cargarse y la tupla persona/organización con su
///   intervalo `[desde, hasta)` (ausencia de año = abierto).
type MemberOfLote = (usize, String, String, Option<i64>, Option<i64>);

fn validar_lote_pipeline(
    lote: &[RegistroCrudo],
    claves_entidades: &mut HashSet<String>,
    claves_relaciones: &mut HashSet<String>,
    aristas_aceptadas: &mut usize,
) -> Vec<Option<String>> {
    // SinSolape LOCAL: las MEMBER_OF aceptadas de ESTE lote, con el id de
    // la arista que tendrán (el contador de aceptadas ANTES de cargarla).
    let mut member_of_lote: Vec<MemberOfLote> = Vec::new();
    let mut validaciones = Vec::with_capacity(lote.len());
    for registro in lote {
        if let Err(motivo) = validar_fila_pipeline(registro) {
            validaciones.push(Some(motivo));
            continue;
        }
        match registro.tipo.as_str() {
            "Persona" | "Documento" => {
                let nombre = campo(
                    registro,
                    if registro.tipo == "Persona" {
                        "nombre"
                    } else {
                        "titulo"
                    },
                )
                .unwrap_or("");
                let clave = format!("{}|{nombre}", registro.tipo);
                if !claves_entidades.insert(clave) {
                    validaciones.push(Some(format!("clave natural repetida: {nombre}")));
                } else {
                    validaciones.push(None);
                }
            }
            // El duplicado EXACTO de Tema NO se rechaza: es el caso 26/61
            // (decisión #11) — la fusión del entity resolution lo cura.
            "Tema" => validaciones.push(None),
            "Relacion" => {
                let fila_cruda: String = registro
                    .campos
                    .iter()
                    .map(|(_, valor)| valor.as_str())
                    .collect::<Vec<_>>()
                    .join("|");
                let desde = campo(registro, "desde").unwrap_or("");
                let hasta = campo(registro, "hasta").unwrap_or("");
                let rel_tipo = campo(registro, "tipo").unwrap_or("");
                if !claves_relaciones.insert(fila_cruda) {
                    validaciones.push(Some(format!(
                        "clave natural repetida: {desde} → {hasta} ({rel_tipo})"
                    )));
                    continue;
                }
                if rel_tipo == "MEMBER_OF" {
                    let d = anio_opcional(campo(registro, "desde_anio"));
                    let h = anio_opcional(campo(registro, "hasta_anio"));
                    let solapada = member_of_lote.iter().find(|(_, p, o, pd, ph)| {
                        p == desde && o == hasta && solapan(d, h, *pd, *ph)
                    });
                    if let Some((id_existente, _, _, _, _)) = solapada {
                        validaciones.push(Some(format!("solape con la {id_existente}")));
                        continue;
                    }
                    member_of_lote.push((*aristas_aceptadas, desde.into(), hasta.into(), d, h));
                }
                *aristas_aceptadas += 1;
                validaciones.push(None);
            }
            otro => validaciones.push(Some(format!("tipo de registro desconocido: {otro}"))),
        }
    }
    validaciones
}

/// Un año de columna CSV como `Option<i64>`: `None` para vacío o no
/// numérico (la validación de fila ya descartó los anios no numéricos de
/// los Documentos; en las Relaciones el año vacío es «abierto»).
fn anio_opcional(texto: Option<&str>) -> Option<i64> {
    texto.and_then(|t| t.trim().parse().ok())
}

/// El siguiente id de nodo LIBRE: `max(id existente) + 1`.
///
/// Antes de la fusión los ids son densos (0..n) y `node_count()` coincide
/// con el máximo; tras las fusiones, los ids de los descartados quedan
/// HUECOS y `node_count()` (la longitud del mapa) puede apuntar a un id
/// VIVO (en el pipeline, el 53 existe como Tema y `node_count()` vale 53):
/// los nodos creados sobre la marcha (entidades sin fichero propio) usan
/// el siguiente id libre, nunca `node_count()`.
fn siguiente_id_libre(store: &MemoryStore) -> usize {
    store
        .iter_nodes()
        .map(|n| n.id)
        .max()
        .map_or(0, |max| max + 1)
}

/// ¿Solapan dos intervalos `[desde, hasta)` con la convención del cap-43
/// (hasta ausente = abierto)? `a` y `b` solapan si `a_desde < b_hasta` Y
/// `b_desde < a_hasta` — el caso de la MEMBER_OF solapada `[2020,2023)`
/// contra la 53 `[2018,2024)` sí solapa; la 185 `[2024,∞)` contra la 53 no.
fn solapan(
    a_desde: Option<i64>,
    a_hasta: Option<i64>,
    b_desde: Option<i64>,
    b_hasta: Option<i64>,
) -> bool {
    let a_d = a_desde.unwrap_or(i64::MIN);
    let a_h = a_hasta.unwrap_or(i64::MAX);
    let b_d = b_desde.unwrap_or(i64::MIN);
    let b_h = b_hasta.unwrap_or(i64::MAX);
    a_d < b_h && b_d < a_h
}

/// El grafo de similitud de TÍTULOS de documento: el mismo `MemoryStore`
/// temporal con aristas `SIMILAR` que [`construir_grafo_similitud`], pero
/// con la regla de Levenshtein (la decisión 3 del bloque del pipeline).
///
/// La regla: mismo bloque por inicial Y `similitud_levenshtein ≥ 0.9`. El
/// umbral tiene margen real sobre el dataset: el casi-duplicado con typo
/// («Recuperación aumentada con grafo(s)») está a 0.97 (UNA edición) y el
/// siguiente par de títulos legítimos más cercano a 0.77 — jaccard de
/// bigramas no sirve aquí: sobrefunde títulos largos (0.63 entre títulos
/// legítimos distintos).
fn construir_grafo_similitud_titulos(titulos: &[String]) -> (MemoryStore, usize) {
    let mut store = MemoryStore::new();
    for i in 0..titulos.len() {
        store
            .put_node(Node::new(i, "Documento"))
            .expect("ids 0..n densos: ningún nodo se repite");
    }
    let mut comparaciones = 0usize;
    let mut n_aristas = 0usize;
    for i in 0..titulos.len() {
        for j in (i + 1)..titulos.len() {
            if bloque_por_inicial(&titulos[i]) != bloque_por_inicial(&titulos[j]) {
                continue;
            }
            comparaciones += 1;
            let a = normalizar_nombre(&titulos[i]);
            let b = normalizar_nombre(&titulos[j]);
            if similitud_levenshtein(&a, &b) >= 0.9 {
                store
                    .put_edge(Edge::new(n_aristas, i, j, "SIMILAR"))
                    .expect("ambos extremos ya existen: arista válida");
                n_aristas += 1;
            }
        }
    }
    (store, comparaciones)
}

/// Aplica el entity resolution a UN tipo de entidad ya mapeado: grafo de
/// similitud (la regla que dicta [`ReglaSimilitud`]) → clusters con
/// [`clusters_de_similitud`] → [`fusionar_cluster`] por cluster ≥ 2, y
/// registra los eventos en el [`HistorialIngesta`] (`FusionEntidad` por
/// cluster y `ConflictoDeclarado` por conflicto — nunca silencioso).
///
/// `nombres` son los nombres crudos en orden de mapeo e `ids_reales` sus
/// ids en el store (mismo índice): los clusters del grafo temporal (ids
/// 0..n del tipo) se traducen a los ids reales. Tras cada fusión se
/// REPUNTA la clave natural de TODOS los miembros al id canónico (las
/// aristas de las piezas siguientes resuelven los nombres por clave).
fn fusionar_er(
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
    nombres: &[String],
    ids_reales: &[usize],
    regla: ReglaSimilitud,
    clave_primer_token: bool,
    historial: &mut HistorialIngesta,
) -> usize {
    let grafo = match regla {
        ReglaSimilitud::Bigramas => construir_grafo_similitud(nombres).0,
        ReglaSimilitud::Levenshtein => construir_grafo_similitud_titulos(nombres).0,
    };
    let mut fusiones = 0usize;
    for cluster in clusters_de_similitud(&grafo) {
        if cluster.len() < 2 {
            continue;
        }
        let reales: Vec<usize> = cluster.iter().map(|&i| ids_reales[i]).collect();
        let informe = fusionar_cluster(store, &reales, &[]);
        // La fusión deja vivo EXACTAMENTE al canónico: su id es el único
        // miembro que sigue en el store.
        let canonico_id = reales
            .iter()
            .copied()
            .find(|&id| store.get_node(id).is_some())
            .expect("la fusión deja vivo al canónico");
        // CUIDADO: `idx_canonico` es el MIEMBRO del cluster (un índice en
        // la lista de nombres), no su posición dentro del cluster — el
        // canónico no tiene por qué ser el primer miembro.
        let idx_canonico = *cluster
            .iter()
            .find(|&&i| ids_reales[i] == canonico_id)
            .expect("el canónico es miembro del cluster");
        let descartadas: Vec<String> = cluster
            .iter()
            .filter(|&&i| i != idx_canonico)
            .map(|&i| nombres[i].clone())
            .collect();
        // UN evento `FusionEntidad` POR ENTIDAD DESCARTADA (no por cluster):
        // el contrato §2 pinea «6 fusiones y 4 rechazos» — tantos eventos
        // de fusión como entidades fusionadas (4 Anas + el typo + el
        // fantasma), el espejo de los 4 `RegistroRechazado`.
        for descartada in descartadas {
            historial.registrar(EventoIngesta::FusionEntidad {
                canónica: nombres[idx_canonico].clone(),
                descartadas: vec![descartada],
            });
        }
        for conflicto in &informe.conflictos {
            historial.registrar(EventoIngesta::ConflictoDeclarado {
                propiedad: conflicto.propiedad.clone(),
                canónico: conflicto.canónico.clone(),
                descartado: conflicto.descartado.clone(),
            });
        }
        for &i in &cluster {
            let clave = if clave_primer_token {
                clave_persona(&nombres[i])
            } else {
                clave_natural(&nombres[i])
            };
            ids.insert(clave, canonico_id);
        }
        fusiones += informe.fusiones;
    }
    fusiones
}

/// Mapea UNA fila de entidad al modelo creando SIEMPRE un nodo nuevo (id =
/// `node_count()`): la variante «mapear una fila por nodo» del pipeline.
///
/// Por qué NO reutiliza `mapear_registro`: su dedup por clave natural
/// (idempotencia a nivel unidad, test pineado) fundiría en UN nodo las
/// variantes de Ana y las dos filas del fantasma ANTES de la fusión — y el
/// contrato §2/§5 exige que esas filas existan como nodos SEPARADOS para
/// que el entity resolution las fusione con sus conteos REALES (4 Anas +
/// typo + fantasma = 6 fusiones, conflicto del orcid DECLARADO). La
/// idempotencia del pipeline la garantiza la FUSIÓN, no el mapeo.
fn mapear_fila_entidad(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<usize, String> {
    validar_fila_pipeline(registro)?;
    match registro.tipo.as_str() {
        "Persona" => {
            let nombre = campo(registro, "nombre").unwrap_or("").to_string();
            let id = siguiente_id_libre(store);
            let mut nodo =
                Node::new(id, "Persona").with_prop("nombre", Value::String(nombre.clone()));
            for (prop, valor) in props_de_campos(registro, &["orcid", "afiliacion"]) {
                nodo = nodo.with_prop(prop, valor);
            }
            store.put_node(nodo).map_err(|e| e.to_string())?;
            ids.insert(clave_persona(&nombre), id);
            Ok(id)
        }
        "Documento" => {
            let titulo = campo(registro, "titulo").unwrap_or("").to_string();
            let id = siguiente_id_libre(store);
            let mut nodo =
                Node::new(id, "Documento").with_prop("titulo", Value::String(titulo.clone()));
            if let Some(anio) = campo(registro, "anio") {
                let anio: i64 = anio
                    .trim()
                    .parse()
                    .map_err(|_| format!("anio no numérico: {anio}"))?;
                nodo = nodo.with_prop("anio", Value::Int(anio));
            }
            if let Some(subtipo) = campo(registro, "tipo").filter(|t| !t.is_empty()) {
                nodo.labels.push(subtipo.to_string());
            }
            store.put_node(nodo).map_err(|e| e.to_string())?;
            ids.insert(clave_natural(&titulo), id);
            Ok(id)
        }
        "Tema" => {
            let nombre = campo(registro, "tema_nombre").unwrap_or("").to_string();
            let id = siguiente_id_libre(store);
            let nodo = Node::new(id, "Tema").with_prop("nombre", Value::String(nombre.clone()));
            store.put_node(nodo).map_err(|e| e.to_string())?;
            ids.insert(clave_natural(&nombre), id);
            Ok(id)
        }
        otro => Err(format!("tipo de registro desconocido: {otro}")),
    }
}

/// El label de la entidad SIN fichero propio que se crea al resolver un
/// extremo desconocido, según el `rel_tipo` (la decisión del contrato §2:
/// los extremos de las aristas referencian entidades por nombre; el
/// pipeline las crea cuando no existen). `None` = el extremo debe existir
/// ya (personas, documentos, temas: sus ficheros se mapearon antes).
fn label_de_extremo(rel_tipo: &str, es_desde: bool) -> Option<&'static str> {
    match (rel_tipo, es_desde) {
        ("MEMBER_OF", false) => Some("Organizacion"),
        ("WORKED_ON", false) => Some("Proyecto"),
        ("PUBLICADO_EN", false) => Some("Conferencia"),
        ("REALIZA", false) => Some("Resena"),
        ("SOBRE", true) => Some("Resena"),
        ("CONTRARRESTA", _) => Some("Resena"),
        // MENTIONS es polimórfico (decisión #7 del cap-41) y se resuelve
        // aparte: Persona por clave de persona, Proyecto por el prefijo
        // «Proyecto », Organizacion en el resto.
        _ => None,
    }
}

/// Resuelve UN extremo de arista por clave natural, CREANDO el nodo cuando
/// la entidad no existe: la pieza que completa el mapeo de
/// Organizacion/Proyecto/Conferencia/Resena (la pieza 8-10 las dejó «sin
/// mapear»). El label del nodo nuevo lo dicta el `rel_tipo`
/// ([`label_de_extremo`]); la prop `nombre` satisface la `Existencia` del
/// esquema del cap-44 (Organizacion.nombre).
fn resolver_o_crear_extremo(
    rel_tipo: &str,
    es_desde: bool,
    nombre: &str,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<usize, String> {
    let lado = if es_desde { "desde" } else { "hasta" };
    if rel_tipo == "MENTIONS" && !es_desde {
        if let Some(&id) = ids.get(&clave_persona(nombre)) {
            return Ok(id);
        }
        let clave = clave_natural(nombre);
        if let Some(&id) = ids.get(&clave) {
            return Ok(id);
        }
        let label = if nombre.trim_start().starts_with("Proyecto ") {
            "Proyecto"
        } else {
            "Organizacion"
        };
        let id = siguiente_id_libre(store);
        store
            .put_node(Node::new(id, label).with_prop("nombre", Value::String(nombre.to_string())))
            .map_err(|e| e.to_string())?;
        ids.insert(clave, id);
        return Ok(id);
    }
    let clave = clave_extremo(rel_tipo, es_desde, nombre)
        .ok_or_else(|| format!("tipo de relación desconocido: {rel_tipo}"))?;
    if let Some(&id) = ids.get(&clave) {
        return Ok(id);
    }
    match label_de_extremo(rel_tipo, es_desde) {
        Some(label) => {
            let id = siguiente_id_libre(store);
            store
                .put_node(
                    Node::new(id, label).with_prop("nombre", Value::String(nombre.to_string())),
                )
                .map_err(|e| e.to_string())?;
            ids.insert(clave, id);
            Ok(id)
        }
        None => Err(format!("extremo '{lado}' sin mapear: {nombre}")),
    }
}

/// Mapea UNA relación cruda: [`mapear_relacion`] de la pieza 8-10 con el
/// resolver que CREA los extremos sin fichero propio
/// ([`resolver_o_crear_extremo`]). Los ids de arista son el orden de
/// inserción (`edge_count`): el pipeline asigna ids NUEVOS (la identidad
/// la da la clave natural, cap-41). Las AUTHORED cargan además la prop
/// `order` (Int) desde la columna `order` del CSV, el orden de firma del
/// cap-41 que la P2 lee.
fn mapear_relacion_pipeline(
    registro: &RegistroCrudo,
    store: &mut MemoryStore,
    ids: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let desde = campo(registro, "desde").unwrap_or("");
    let hasta = campo(registro, "hasta").unwrap_or("");
    let rel_tipo = campo(registro, "tipo").unwrap_or("").to_string();
    let desde_id = resolver_o_crear_extremo(&rel_tipo, true, desde, store, ids)?;
    let hasta_id = resolver_o_crear_extremo(&rel_tipo, false, hasta, store, ids)?;
    let es_authered = rel_tipo == "AUTHORED";
    let mut arista = Edge::new(store.edge_count(), desde_id, hasta_id, rel_tipo);
    for (columna, valor) in [
        ("desde_anio", campo(registro, "desde_anio")),
        ("hasta_anio", campo(registro, "hasta_anio")),
    ] {
        if let Some(texto) = valor.filter(|v| !v.is_empty()) {
            let anio: i64 = texto
                .trim()
                .parse()
                .map_err(|_| format!("{columna} no numérico: {texto}"))?;
            arista = arista.with_prop(columna, Value::Int(anio));
        }
    }
    if es_authered && let Some(texto) = campo(registro, "order").filter(|v| !v.is_empty()) {
        let orden: i64 = texto
            .trim()
            .parse()
            .map_err(|_| format!("order no numérico: {texto}"))?;
        arista = arista.with_prop("order", Value::Int(orden));
    }
    store.put_edge(arista).map_err(|e| e.to_string())
}

/// El PIPELINE COMPLETO del paso-5 (contrato §2, `kb_lira_paso5`): los 4
/// ficheros crudos de `datasets/kb-lira/paso-5/crudos/` → **67 nodos /
/// 158 aristas** con el esquema del cap-44 `Ok`.
///
/// Las cuatro etapas encadenadas, con sus decisiones:
///
/// 1. **EXTRAER** — los 4 ficheros con [`DatosCrudos::desde_csv`] (pieza
///    1, `partir_csv` del cap-32 reutilizado), en lotes de 25 registros
///    por fichero (la frontera de streaming: la unidad residente máxima es
///    el lote). Cuenta filas (221) y lotes (11).
/// 2. **VALIDAR** — por lote con [`validar_lote_pipeline`]: las reglas de
///    fila, la clave natural repetida y el `SinSolape` local; los rechazos
///    se registran en el [`HistorialIngesta`] con línea y motivo y el lote
///    sigue (rechazo selectivo, nunca aborta por una fila). Al final,
///    `verificar_esquema` del cap-44 sobre el store completo como PUERTA
///    final (el esquema valida grafos enteros; el lote aislado no puede).
/// 3. **MAPEAR + FUSIONAR** — las entidades se mapean UNA FILA POR NODO
///    ([`mapear_fila_entidad`]: la idempotencia la da la FUSIÓN, no el
///    mapeo — sin nodos separados no hay fusiones que contar); el entity
///    resolution ([`fusionar_er`]) fusiona los clusters ANTES de mapear
///    las relaciones y registra `FusionEntidad`/`ConflictoDeclarado`. Las
///    relaciones se mapean después, resolviendo los extremos por clave
///    natural y CREANDO las entidades sin fichero propio (Organizacion,
///    Proyecto, Conferencia, Resena) desde los extremos desconocidos.
/// 4. **CARGAR** — `put_node`/`put_edge` con ids nuevos (la identidad la
///    da la clave natural, cap-41); las 158 aristas aceptadas sobreviven a
///    las fusiones porque el re-apuntado no borra.
///
/// Devuelve `(store, historial, informe)` con los contadores reales. El
/// dataset commiteado es determinista: si la puerta final del esquema
/// fallara, el pipeline hace pánico con las violaciones (nunca entrega un
/// grafo que «no sabe» — contrato §2).
pub fn cargar_paso5() -> (MemoryStore, HistorialIngesta, InformeIngesta) {
    const TAMANO_LOTE: usize = 25;
    let crudos = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../datasets/kb-lira/paso-5/crudos"
    );

    let mut store = MemoryStore::new();
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut historial = HistorialIngesta::nueva();
    let mut informe = InformeIngesta::default();
    let mut claves_entidades: HashSet<String> = HashSet::new();
    let mut claves_relaciones: HashSet<String> = HashSet::new();
    let mut aristas_aceptadas = 0usize;
    let mut lote_global = 0usize;

    historial.registrar(EventoIngesta::CargaIniciada);

    // ── EXTRAER + VALIDAR + MAPEAR: las entidades (personas, documentos,
    //    temas), una fila por nodo, en el orden de los ficheros. ──
    let mut nombres_personas: Vec<String> = Vec::new();
    let mut ids_personas: Vec<usize> = Vec::new();
    let mut nombres_documentos: Vec<String> = Vec::new();
    let mut ids_documentos: Vec<usize> = Vec::new();
    let mut nombres_temas: Vec<String> = Vec::new();
    let mut ids_temas: Vec<usize> = Vec::new();

    for (fichero, tipo) in [
        ("personas.csv", "Persona"),
        ("documentos.csv", "Documento"),
        ("temas.csv", "Tema"),
    ] {
        let contenido =
            std::fs::read_to_string(format!("{crudos}/{fichero}")).unwrap_or_else(|e| {
                panic!("no se pudo leer datasets/kb-lira/paso-5/crudos/{fichero}: {e}")
            });
        let datos = DatosCrudos::desde_csv(&contenido, tipo)
            .unwrap_or_else(|e| panic!("{fichero} inválido: {e}"));
        informe.filas_leidas += datos.registros.len();
        for (k, lote) in datos.registros.chunks(TAMANO_LOTE).enumerate() {
            lote_global += 1;
            let linea_inicial = k * TAMANO_LOTE + 2; // 1 = cabecera
            let validaciones = validar_lote_pipeline(
                lote,
                &mut claves_entidades,
                &mut claves_relaciones,
                &mut aristas_aceptadas,
            );
            historial.registrar(EventoIngesta::LoteValidado { lote: lote_global });
            for (i, motivo) in validaciones.iter().enumerate() {
                if let Some(motivo) = motivo {
                    historial.registrar(EventoIngesta::RegistroRechazado {
                        linea: linea_inicial + i,
                        motivo: motivo.clone(),
                    });
                    informe.rechazos += 1;
                }
            }
            for (i, registro) in lote.iter().enumerate() {
                if validaciones[i].is_none() {
                    let id = mapear_fila_entidad(registro, &mut store, &mut ids)
                        .expect("fila validada: el mapeo no puede fallar");
                    let nombre = campo(registro, "nombre")
                        .or_else(|| campo(registro, "titulo"))
                        .or_else(|| campo(registro, "tema_nombre"))
                        .unwrap_or_default()
                        .to_string();
                    match tipo {
                        "Persona" => {
                            nombres_personas.push(nombre);
                            ids_personas.push(id);
                        }
                        "Documento" => {
                            nombres_documentos.push(nombre);
                            ids_documentos.push(id);
                        }
                        _ => {
                            nombres_temas.push(nombre);
                            ids_temas.push(id);
                        }
                    }
                }
            }
        }
    }

    // ── FUSIONAR: entity resolution ANTES de mapear las relaciones. ──
    informe.fusiones += fusionar_er(
        &mut store,
        &mut ids,
        &nombres_personas,
        &ids_personas,
        ReglaSimilitud::Bigramas,
        true,
        &mut historial,
    );
    informe.fusiones += fusionar_er(
        &mut store,
        &mut ids,
        &nombres_documentos,
        &ids_documentos,
        ReglaSimilitud::Levenshtein,
        false,
        &mut historial,
    );
    informe.fusiones += fusionar_er(
        &mut store,
        &mut ids,
        &nombres_temas,
        &ids_temas,
        ReglaSimilitud::Bigramas,
        false,
        &mut historial,
    );

    // ── EXTRAER + VALIDAR + CARGAR: las relaciones, con los extremos sin
    //    fichero propio creados sobre la marcha. ──
    let contenido =
        std::fs::read_to_string(format!("{crudos}/relaciones.csv")).unwrap_or_else(|e| {
            panic!("no se pudo leer datasets/kb-lira/paso-5/crudos/relaciones.csv: {e}")
        });
    let datos = DatosCrudos::desde_csv(&contenido, "Relacion")
        .unwrap_or_else(|e| panic!("relaciones.csv inválido: {e}"));
    informe.filas_leidas += datos.registros.len();
    for (k, lote) in datos.registros.chunks(TAMANO_LOTE).enumerate() {
        lote_global += 1;
        let linea_inicial = k * TAMANO_LOTE + 2;
        let validaciones = validar_lote_pipeline(
            lote,
            &mut claves_entidades,
            &mut claves_relaciones,
            &mut aristas_aceptadas,
        );
        historial.registrar(EventoIngesta::LoteValidado { lote: lote_global });
        for (i, motivo) in validaciones.iter().enumerate() {
            if let Some(motivo) = motivo {
                historial.registrar(EventoIngesta::RegistroRechazado {
                    linea: linea_inicial + i,
                    motivo: motivo.clone(),
                });
                informe.rechazos += 1;
            }
        }
        for (i, registro) in lote.iter().enumerate() {
            if validaciones[i].is_none() {
                mapear_relacion_pipeline(registro, &mut store, &mut ids)
                    .expect("fila validada: la arista no puede fallar");
            }
        }
    }

    // ── PUERTA FINAL: el esquema del cap-44 sobre el grafo completo. ──
    if let Err(violaciones) = verificar_esquema(&store, &esquema_kb_lira()) {
        panic!("el paso-5 no cumple el esquema del cap-44: {violaciones:?}");
    }

    informe.nodos_finales = store.node_count();
    informe.aristas_finales = store.edge_count();
    informe.lotes = lote_global;
    historial.registrar(EventoIngesta::CargaCompletada {
        nodos: informe.nodos_finales,
        aristas: informe.aristas_finales,
    });

    (store, historial, informe)
}

// ─────────────────── CSV determinista del paso-5 (formato cap. 32) ───────────────────

/// Exporta los NODOS del paso-5 al formato del cap. 32 (el MISMO que
/// `csv_nodos_kb_lira` del cap-41 y los pasos 2-4): la cabecera nace de la
/// unión de props — en el paso-5 la columna `orcid:STRING` solo existe en
/// las Personas y `anio:INT` solo en los Documentos, como en el paso-4.
pub fn csv_nodos_kb_lira_paso5(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_nodos(store, &mut buf).expect("export nodos paso-5");
    String::from_utf8(buf).expect("CSV UTF-8")
}

/// Exporta las ARISTAS del paso-5 (mismo contrato que
/// [`csv_nodos_kb_lira_paso5`]): `MEMBER_OF` con `desde_anio:INT` y, donde
/// el intervalo está cerrado, `hasta_anio:INT`; `AUTHORED` con `order:INT`
/// (el orden de firma del cap-41 que viaja en la columna `order` del crudo).
pub fn csv_aristas_kb_lira_paso5(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_aristas(store, &mut buf).expect("export aristas paso-5");
    String::from_utf8(buf).expect("CSV UTF-8")
}

// ─────────────────── Informe reproducible del capítulo (para la prosa) ───────────────────

/// El informe de ingesta REPRODUCIBLE del capítulo: la tesis completa en
/// texto plano, SIN tiempos de ejecución — la moneda son filas, lotes,
/// rechazos, comparaciones evitadas, clusters, fusiones, conflictos y
/// conjuntos exactos —, para que la prosa del cap-45 lo cite tal cual.
///
/// Todo se calcula de las funciones REALES del pipeline ([`cargar_paso5`],
/// [`construir_grafo_similitud`], [`clusters_de_similitud`],
/// [`jaccard_bigramas`], [`similitud_levenshtein`] y
/// [`verificar_esquema`]): los únicos literales son los NOMBRES de los
/// pares clave del contrato §2/§5 (las variantes de Ana, el typo de título
/// y el fantasma «memoria de agentes»), nunca los contadores.
///
/// Partes, en el orden del contrato §2:
///
/// 1. **Fichero → filas → lotes**: los conteos REALES por fichero crudo
///    (14 + 38 + 9 + 160 = 221) y los 11 lotes de 25 del pipeline.
/// 2. **Rechazos**: los 4 motivos con su línea, leídos del
///    [`HistorialIngesta`] real (nada hardcodeado).
/// 3. **Entity resolution**: las 91 comparaciones del todo-contra-todo
///    frente a las 11 del bloque por inicial (80 evitadas) y los 9
///    clusters REALES de las 14 personas crudas (5 Anas + 2 Betos + 7
///    singletons), ambos con [`construir_grafo_similitud`]/
///    [`clusters_de_similitud`] sobre los nombres leídos del CSV.
/// 4. **Similitud**: jaccard de bigramas y Levenshtein en los pares clave
///    del capítulo, con las funciones REALES (0,4 / 0,3 / 0,11 y 0,97 /
///    0,77 — el siguiente par legítimo de títulos se busca en los datos).
/// 5. **Fusiones y conflictos**: los 6 `FusionEntidad` y los 6
///    `ConflictoDeclarado` del historial real, con el orcid malformado a
///    la vista (la fusión salva la `Unicidad` que la fila suelta habría
///    violado).
/// 6. **Delta de nodos**: el caso Tema 26/61 curado — los `nodos_finales`
///    + el fantasma exacto (la fusión de dos filas IDÉNTICAS) = 68 → 67.
/// 7. **Veredicto**: el esquema del cap-44 acepta el modelo (`Ok`).
pub fn informe_ingesta_reproducible() -> String {
    let (store, historial, informe) = cargar_paso5();

    // Parte 1: filas reales por fichero (el mismo lector del pipeline).
    let crudos = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../datasets/kb-lira/paso-5/crudos"
    );
    let mut por_fichero: Vec<(String, usize)> = Vec::new();
    for (fichero, tipo) in [
        ("personas.csv", "Persona"),
        ("documentos.csv", "Documento"),
        ("temas.csv", "Tema"),
        ("relaciones.csv", "Relacion"),
    ] {
        let contenido = std::fs::read_to_string(format!("{crudos}/{fichero}"))
            .unwrap_or_else(|e| panic!("no se pudo leer {fichero}: {e}"));
        let datos = DatosCrudos::desde_csv(&contenido, tipo)
            .unwrap_or_else(|e| panic!("{fichero} inválido: {e}"));
        por_fichero.push((fichero.to_string(), datos.registros.len()));
    }

    // Parte 2: los 4 rechazos reales (línea y motivo) del historial.
    let rechazos: Vec<String> = historial
        .eventos()
        .iter()
        .filter_map(|(_, evento)| match evento {
            EventoIngesta::RegistroRechazado { linea, motivo } => {
                Some(format!("línea {linea} · {motivo}"))
            }
            _ => None,
        })
        .collect();

    // Parte 3: el entity resolution sobre los 14 nombres REALES del CSV
    // crudo (los mismos que el pipeline lee), con los contadores REALES.
    let nombres_personas: Vec<String> = std::fs::read_to_string(format!("{crudos}/personas.csv"))
        .expect("personas.csv leíble")
        .lines()
        .skip(1)
        .map(|linea| linea.split(',').next().unwrap_or_default().to_string())
        .collect();
    let naive = nombres_personas.len() * (nombres_personas.len() - 1) / 2;
    let (grafo, comparadas) = construir_grafo_similitud(&nombres_personas);
    let clusters = clusters_de_similitud(&grafo);
    let multiples: Vec<&Vec<usize>> = clusters.iter().filter(|c| c.len() > 1).collect();
    let singletons = clusters.len() - multiples.len();
    let clusters_txt = {
        let multiples_txt: Vec<String> = multiples
            .iter()
            .map(|c| format!("{} «{}»…", c.len(), nombres_personas[c[0]]))
            .collect();
        multiples_txt.join(" + ") + &format!(" + {singletons} singletons")
    };

    // Parte 4: las similitudes de los pares clave, con las funciones REALES.
    let jaccard = |a: &str, b: &str| fmt_similitud(jaccard_bigramas(a, b));
    let jaccard_ana_g = jaccard("ana garcia", "ana g.");
    let jaccard_ana_parentesis = jaccard("ana garcia", "ana garcia (universidad de lira)");
    let jaccard_contraste = jaccard("ana garcia", "carla mendez");
    let lev_typo = fmt_similitud(similitud_levenshtein(
        &normalizar_nombre("Recuperación aumentada con grafo"),
        &normalizar_nombre("Recuperación aumentada con grafos"),
    ));
    let (lev_legitimo, legit_a, legit_b) = siguiente_par_legitimo_titulos();
    let lev_legitimo = fmt_similitud(lev_legitimo);

    // Parte 5: fusiones y conflictos reales del historial.
    let fusion_anas = historial
        .eventos()
        .iter()
        .filter(|(_, e)| matches!(e, EventoIngesta::FusionEntidad { canónica, .. } if canónica == "Ana"))
        .count();
    let fusion_typo = historial
        .eventos()
        .iter()
        .filter(|(_, e)| matches!(e, EventoIngesta::FusionEntidad { canónica, .. } if canónica == "Recuperación aumentada con grafos"))
        .count();
    let conflictos: Vec<&EventoIngesta> = historial
        .eventos()
        .iter()
        .filter_map(|(_, e)| match e {
            EventoIngesta::ConflictoDeclarado { .. } => Some(e),
            _ => None,
        })
        .collect();
    let orcid = conflictos
        .iter()
        .find_map(|e| match e {
            EventoIngesta::ConflictoDeclarado {
                propiedad,
                canónico,
                descartado,
            } if propiedad == "orcid" => Some((canónico.clone(), descartado.clone())),
            _ => None,
        })
        .expect("el orcid malformado de Ana se declara en la fusión");
    let (orcid_canonico, orcid_descartado) = orcid;

    // Parte 6: el fantasma EXACTO del Tema 26/61 — canónica y descartada
    // IDÉNTICAS (el duplicado invisible que el ER cura): el grafo final
    // sería `nodos_finales + 1` sin la fusión.
    let fantasmas = historial
        .eventos()
        .iter()
        .filter(|(_, e)| {
            matches!(e, EventoIngesta::FusionEntidad { canónica, descartadas }
                if canónica == descartadas.first().map(String::as_str).unwrap_or_default())
        })
        .count();
    let sin_cura = informe.nodos_finales + fantasmas;

    // Parte 7: las etiquetas del grafo final y el veredicto del esquema.
    let mut por_label: HashMap<String, usize> = HashMap::new();
    for nodo in store.iter_nodes() {
        *por_label.entry(nodo.labels[0].clone()).or_insert(0) += 1;
    }
    let conteo = |label: &str| por_label.get(label).copied().unwrap_or(0);
    let etiquetas = [
        "Persona",
        "Tema",
        "Documento",
        "Organizacion",
        "Proyecto",
        "Conferencia",
        "Resena",
    ]
    .iter()
    .map(|l| format!("{} {l}", conteo(l)))
    .collect::<Vec<_>>()
    .join(" · ");
    let veredicto = match verificar_esquema(&store, &esquema_kb_lira()) {
        Ok(()) => "Ok".to_string(),
        Err(violaciones) => format!("NO — {violaciones:?}"),
    };

    // La tabla del capítulo (formato de los informes cap-43/44: columnas
    // `|` y separador `─` del ancho de la fila más larga).
    let filas: Vec<(String, String)> = vec![
        (
            "Ficheros → filas".into(),
            format!(
                "{} = {} filas",
                por_fichero
                    .iter()
                    .map(|(f, n)| format!("{f} {n}"))
                    .collect::<Vec<_>>()
                    .join(" + "),
                informe.filas_leidas
            ),
        ),
        (
            "Lotes de 25 filas".into(),
            format!("{} lotes", informe.lotes),
        ),
        (
            format!("Rechazos ({})", informe.rechazos),
            rechazos.join(" · "),
        ),
        (
            "Entity resolution · personas".into(),
            format!(
                "{naive} pares todo-contra-todo → {comparadas} comparadas por bloque inicial ({} evitadas)",
                naive - comparadas
            ),
        ),
        (
            format!("Clusters ({} personas crudas)", nombres_personas.len()),
            format!("{clusters_txt} ({} clusters)", clusters.len()),
        ),
        (
            "Jaccard de bigramas".into(),
            format!(
                "«ana garcia»↔«ana g.» {jaccard_ana_g} · «ana garcia»↔«ana garcia (universidad de lira)» {jaccard_ana_parentesis} · contraste «carla mendez» {jaccard_contraste}"
            ),
        ),
        (
            "Levenshtein (títulos)".into(),
            format!(
                "typo «…aumentada con grafo»↔«…grafos» {lev_typo} · siguiente par legítimo {lev_legitimo} («{legit_a}» ↔ «{legit_b}»)"
            ),
        ),
        (
            format!("Fusiones ({})", informe.fusiones),
            format!(
                "{fusion_anas} «Ana» + {fusion_typo} «Recuperación aumentada con grafos» + {fantasmas} «memoria de agentes» (el fantasma)"
            ),
        ),
        (
            format!("Conflictos declarados ({})", conflictos.len()),
            format!(
                "orcid {orcid_canonico} ≠ {orcid_descartado} — gana el canónico, nunca silencioso"
            ),
        ),
        (
            "Caso Tema 26/61 · curado".into(),
            format!(
                "«memoria de agentes» duplicada exacta: {sin_cura} → {} nodos",
                informe.nodos_finales
            ),
        ),
        (
            "Grafo final".into(),
            format!(
                "{} nodos ({etiquetas}) · {} aristas",
                informe.nodos_finales, informe.aristas_finales
            ),
        ),
        ("Esquema cap-44".into(), veredicto),
    ];

    let ancho_etiqueta = filas.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let filas_texto: Vec<String> = filas
        .iter()
        .map(|(l, d)| format!("{l:<ancho_etiqueta$} | {d}"))
        .collect();
    let ancho = filas_texto.iter().map(String::len).max().unwrap_or(0);

    let mut buf = String::from("Informe de ingesta — KB-Lira paso-5 (del CSV crudo al grafo)\n");
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');
    for fila in &filas_texto {
        buf.push_str(fila);
        buf.push('\n');
    }
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');
    buf.push_str(&format!(
        "La ingesta transforma el CRUDO en grafo: {} filas → {} lotes → {} rechazos → {} \
         fusiones (con {} conflictos declarados, el orcid malformado entre ellos) → {} nodos \
         y {} aristas. Nada se sobrescribió en silencio y el esquema del cap-44 acepta el \
         resultado: el paso-5 es el paso-4 verificado, no una copia suya.\n",
        informe.filas_leidas,
        informe.lotes,
        informe.rechazos,
        informe.fusiones,
        conflictos.len(),
        informe.nodos_finales,
        informe.aristas_finales,
    ));
    buf
}

/// Similitud formateada para la prosa: dos decimales con coma decimal
/// española y los ceros finales recortados (0,4 · 0,3 · 0,11 · 0,97 · 0,77).
fn fmt_similitud(valor: f64) -> String {
    let mut texto = format!("{valor:.2}").replace('.', ",");
    while texto.ends_with('0') {
        texto.pop();
    }
    texto
}

/// El par de TÍTULOS REALES del dataset (documentos.csv) con la MAYOR
/// similitud de Levenshtein SIN contar el typo del capítulo (el par
/// «Recuperación aumentada con grafo(s)», ~0.97) ni los títulos idénticos
/// (los duplicados exactos miden 1.0): la frontera que separa el caso del
/// typo de los pares legítimos (0,77 — la evidencia del umbral ≥ 0.9).
fn siguiente_par_legitimo_titulos() -> (f64, String, String) {
    let crudos = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../datasets/kb-lira/paso-5/crudos"
    );
    let titulos: Vec<String> = std::fs::read_to_string(format!("{crudos}/documentos.csv"))
        .expect("documentos.csv leíble")
        .lines()
        .skip(1)
        .map(|linea| linea.split(',').next().unwrap_or_default().to_string())
        .collect();
    let mut mejor: (f64, String, String) = (0.0, String::new(), String::new());
    for i in 0..titulos.len() {
        for j in (i + 1)..titulos.len() {
            let a = normalizar_nombre(&titulos[i]);
            let b = normalizar_nombre(&titulos[j]);
            if a == b {
                continue;
            }
            let sim = similitud_levenshtein(&a, &b);
            if sim < 0.9 && sim > mejor.0 {
                mejor = (sim, titulos[i].clone(), titulos[j].clone());
            }
        }
    }
    mejor
}

#[cfg(test)]
mod tests_pipeline {
    use super::*;

    /// El pipeline completo produce EXACTAMENTE el modelo del contrato §2:
    /// 67 nodos (9 personas, 8 temas — el fantasma 26/61 curado, 36
    /// documentos — el typo fusionado, y las 14 entidades sin fichero
    /// propio creadas desde los extremos) y 158 aristas (las 160 crudas
    /// menos los 2 rechazos de relación). Los contadores reales del
    /// informe (221 filas, 11 lotes, 4 rechazos, 6 fusiones) y el
    /// historial con sus eventos.
    #[test]
    fn kb_lira_paso5_cuenta_y_etiquetas_exactas() {
        let (store, historial, informe) = cargar_paso5();

        assert_eq!(store.node_count(), 67, "67 nodos — el pin del contrato");
        assert_eq!(store.edge_count(), 158, "158 aristas — el pin del contrato");

        let mut por_label: HashMap<String, usize> = HashMap::new();
        for nodo in store.iter_nodes() {
            *por_label.entry(nodo.labels[0].clone()).or_insert(0) += 1;
        }
        assert_eq!(por_label.get("Persona"), Some(&9), "9 personas");
        assert_eq!(
            por_label.get("Tema"),
            Some(&8),
            "8 temas — el fantasma 26/61 se fue"
        );
        assert_eq!(
            por_label.get("Documento"),
            Some(&36),
            "36 documentos — el typo se fusionó"
        );
        assert_eq!(
            por_label.get("Organizacion"),
            Some(&4),
            "4 organizaciones creadas desde los extremos"
        );
        assert_eq!(
            por_label.get("Proyecto"),
            Some(&3),
            "3 proyectos creados desde los extremos"
        );
        assert_eq!(
            por_label.get("Conferencia"),
            Some(&3),
            "3 conferencias creadas desde los extremos"
        );
        assert_eq!(
            por_label.get("Resena"),
            Some(&4),
            "4 reseñas creadas desde los extremos"
        );
        assert_eq!(
            store
                .iter_edges()
                .filter(|e| e.label == "MEMBER_OF")
                .count(),
            10,
            "las 10 MEMBER_OF del paso-3"
        );

        // Los contadores REALES del pipeline.
        assert_eq!(informe.filas_leidas, 221, "14 + 38 + 9 + 160 filas");
        assert_eq!(informe.lotes, 11, "1 + 2 + 1 + 7 lotes de 25");
        assert_eq!(
            informe.rechazos, 4,
            "Beto, DOC_RAG, MEMBER_OF y la solapada"
        );
        assert_eq!(informe.fusiones, 6, "4 Anas + el typo + el fantasma");
        assert_eq!(informe.nodos_finales, 67);
        assert_eq!(informe.aristas_finales, 158);

        // El historial: los 6 eventos de fusión y los 4 de rechazo, con
        // ts monótono 1..n (la forma del HistoricoAfiliaciones del cap-43).
        let fusiones = historial
            .eventos()
            .iter()
            .filter(|(_, e)| matches!(e, EventoIngesta::FusionEntidad { .. }))
            .count();
        let rechazos = historial
            .eventos()
            .iter()
            .filter(|(_, e)| matches!(e, EventoIngesta::RegistroRechazado { .. }))
            .count();
        let ts: Vec<u64> = historial.eventos().iter().map(|(ts, _)| *ts).collect();
        let esperados: Vec<u64> = (1..=ts.len() as u64).collect();
        assert_eq!(fusiones, 6, "las 6 fusiones registradas en el historial");
        assert_eq!(rechazos, 4, "los 4 rechazos registrados en el historial");
        assert_eq!(ts, esperados, "ts 1..n monótono, append-only");
    }

    /// La puerta final del pipeline: el modelo del paso-5 cumple el
    /// esquema declarativo del cap-44 (`verificar_esquema` → `Ok(())`).
    #[test]
    fn el_paso5_pasa_el_esquema_del_cap44() {
        let (store, _, _) = cargar_paso5();
        verificar_esquema(&store, &esquema_kb_lira())
            .expect("el paso-5 cumple el esquema del cap-44");
    }
}

// ─────────────────────────────────────────────────────────────────────
// Red de seguridad del capítulo: regresión de las preguntas del cap-41
// (paso-1) y de los pines del cap-42 contra el modelo INGESTADO del
// paso-5. Las preguntas buscan POR NOMBRE (títulos, personas, temas) —
// la ingesta asigna ids nuevos, así que la identidad solo puede
// verificarse por clave natural. El objetivo de estos tests NO es forzar
// igualdad: es DETECTAR qué cambia y documentar el motivo REAL (la
// ingesta normaliza lo que el builder tenía a mano).
// ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests_regresion {
    use super::*;
    use crate::cap41_modelado::kb_lira_paso1;
    use crate::cap41_modelado::nodo_por_nombre;
    use crate::cap41_modelado::pregunta_01_documentos_de_una_persona as q01;
    use crate::cap41_modelado::pregunta_02_autores_en_orden_de_firma as q02;
    use crate::cap41_modelado::pregunta_03_citas_en_ambas_direcciones as q03;
    use crate::cap41_modelado::pregunta_04_proyecto_y_afiliaciones_en_dos_saltos as q04;
    use crate::cap41_modelado::pregunta_05_temas_de_un_documento_e_inversa as q05;
    use crate::cap41_modelado::pregunta_06_menciones_a_una_entidad as q06;
    use crate::cap41_modelado::pregunta_07_copublicacion_entre_dos_personas as q07;
    use crate::cap41_modelado::pregunta_08_publicaciones_por_persona_contadas as q08;
    use crate::cap41_modelado::pregunta_09_temas_comunes_via_papers as q09;
    use crate::cap41_modelado::pregunta_10_citas_recientes_que_tratan_un_tema as q10;
    use crate::cap42_antipatrones::documentos_del_tema_incluyendo_subtemas;

    /// Las 10 preguntas del cap-41 sobre `kb_lira_paso1()` (referencia) y
    /// sobre `cargar_paso5().0` (ingesta). Las preguntas buscan por NOMBRE
    /// (títulos, personas, temas): deben responder IGUAL pese a los ids
    /// nuevos. Las diferencias REALES detectadas se pinean con su motivo:
    ///
    /// * **P2** idéntica — `relaciones.csv` lleva la columna `order` (el
    ///   orden de firma del builder del cap-41) y el pipeline la carga
    ///   como prop de arista, así que la ingesta responde el MISMO orden
    ///   de firma que el paso-1 (el test
    ///   [`la_ingesta_preserva_el_orden_de_firma`] lo pinea título a
    ///   título).
    /// * **P4** difiere SOLO en una fila nueva legítima — la ingesta
    ///   responde las mismas 3 afiliaciones del paso-1 (Ana→Universidad
    ///   de Lira, Beto→Instituto Neurónica, Dani→Instituto Neurónica,
    ///   vía MEMBER_OF, el mismo patrón LiraQL del cap-41) MÁS
    ///   Beto→Instituto GrafoLuna: la MEMBER_OF `[2024,∞)` existe en
    ///   `relaciones.csv` como hecho del mundo, no se solapa (la
    ///   solapada `[2020,2023)` sí se rechazó) y el pipeline la carga
    ///   bien. Enriquecimiento del dataset, no regresión del shape.
    /// * **P5 inversa** responde 12 docs en vez de 6 — el paso-5 ingiere
    ///   el dataset COMPLETO (paso-1 + lote del cap-42): 6 papers del
    ///   lote tienen ABOUT directa al tema «grafos de conocimiento»
    ///   (mismo fenómeno «enriquecida» que el cap-42 documentó).
    /// * **P8** enriquecida — 9 personas / 40 AUTHORED (el lote), pero
    ///   los conteos por persona canónica del paso-1 se conservan
    ///   EXACTOS (Ana 4, Beto 3, Carla 2, Dani 2): las 4 Anas
    ///   fusionadas no inflan a Ana.
    ///
    /// El resto (P1, P3, P5a, P6, P7, P9, P10) es idéntico por nombre:
    /// el fantasma 26/61 fusionado sobrevive por nombre y ninguna
    /// pregunta dependiente de ids se rompe.
    #[test]
    fn las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta() {
        let referencia = kb_lira_paso1();
        let (ingesta, _, _) = cargar_paso5();
        let r = &referencia;
        let i = &ingesta;

        // P1: idéntica — los 4 AUTHORED de Ana (las variantes fusionadas
        // no publicaron; el canónico conserva el nombre «Ana»).
        assert_eq!(
            q01(i, "Ana"),
            q01(r, "Ana"),
            "P1: idéntica por nombre — las 4 Anas fusionadas no añaden AUTHORED"
        );

        // P2: idéntica — el crudo transporta el orden de firma en la
        // columna `order` y el pipeline la carga como prop de arista.
        assert_eq!(
            q02(i, "Memoria episódica en LLMs"),
            q02(r, "Memoria episódica en LLMs"),
            "P2: idéntica — el order de firma sobrevive a la ingesta"
        );
        assert_eq!(
            q02(i, "Supernodos: anatomía de un cuello de botella"),
            q02(r, "Supernodos: anatomía de un cuello de botella"),
            "P2b: idéntica — Beto firma primero aunque Ana tenga menor id"
        );

        // P3: idéntica — el CITES del paso-1 se carga igual por título.
        assert_eq!(
            q03(i, "Supernodos: anatomía de un cuello de botella"),
            q03(r, "Supernodos: anatomía de un cuello de botella"),
            "P3: idéntica — las CITES del lote son internas y no tocan el paso-1"
        );

        // P4: DIFIERE solo por la MEMBER_OF legítima nueva (GrafoLuna).
        let p4_ref = q04(r, "Proyecto Kira");
        let p4_ing = q04(i, "Proyecto Kira");
        assert_eq!(p4_ref.len(), 3, "referencia: Ana/Beto/Dani");
        assert_eq!(
            p4_ing.len(),
            4,
            "ingesta: las 3 del paso-1 + Beto→Instituto GrafoLuna"
        );
        for fila in &p4_ref {
            assert!(
                p4_ing.contains(fila),
                "P4: la fila del paso-1 {fila:?} sobrevive en la ingesta"
            );
        }
        assert_eq!(
            p4_ing,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Beto".to_string(), "Instituto GrafoLuna".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ],
            "P4: MEMBER_OF se reconstruye con los MISMOS extremos por clave natural \
             (la P4 real del cap-41 responde POR ARISTAS, no por la prop afiliacion); \
             la fila extra es la MEMBER_OF [2024,∞) del dataset, sin solape — la solapada \
             [2020,2023) sí se rechazó en la ingesta"
        );

        // P5a: idéntica — el tema del Informe Kira sobrevive por nombre.
        // P5b: DIFIERE — 12 docs en vez de 6: el lote añade 6 ABOUT directas.
        let (temas_ref, docs_ref) = q05(
            r,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        let (temas_ing, docs_ing) = q05(
            i,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        assert_eq!(
            temas_ing, temas_ref,
            "P5a: idéntica — el fantasma 26/61 fusionado conserva el nombre «grafos de conocimiento»"
        );
        assert_eq!(
            docs_ref.len(),
            6,
            "referencia: los 6 docs del paso-1 con ABOUT directa"
        );
        assert_eq!(
            docs_ing,
            vec![
                "Anclaje semántico para grafos de conocimiento",
                "Consultas declarativas sobre property graphs",
                "Control de calidad en grafos de conocimiento",
                "Evaluación de sistemas GraphRAG",
                "Grafos de conocimiento para agentes",
                "Informe anual del Proyecto Kira",
                "Materialización incremental de reglas en grafos",
                "Notas de la reunión de arranque",
                "Recuperación aumentada con grafos",
                "Resumen del taller de GQL",
                "Streaming de aristas sobre grafos de conocimiento",
                "Versionado de ontologías en grafos",
            ],
            "P5b: enriquecida — los 6 del paso-1 + 6 papers del lote con ABOUT directa \
             al tema (la ingesta ingiere el dataset COMPLETO del paso-5; el cap-42 ya \
             documentó este fenómeno como «P5: jerárquica 24»)"
        );

        // P6: idéntica (org, persona y proyecto) — las MENTIONS se cargan igual.
        assert_eq!(
            q06(i, "Instituto Neurónica"),
            q06(r, "Instituto Neurónica"),
            "P6: idéntica — el lote no añade MENTIONS"
        );
        assert_eq!(
            q06(i, "Elena"),
            q06(r, "Elena"),
            "P6b: idéntica — Elena sigue mencionada solo por el Informe Kira"
        );
        assert_eq!(
            q06(i, "Proyecto Kira"),
            q06(r, "Proyecto Kira"),
            "P6c: idéntica — el nodo Proyecto creado desde el extremo WORKED_ON responde igual"
        );

        // P7: idéntica — co-publicación por nombre.
        assert_eq!(
            q07(i, "Ana", "Beto"),
            q07(r, "Ana", "Beto"),
            "P7: idéntica — las 2 co-publicaciones del paso-1"
        );
        assert!(
            q07(i, "Elena", "Dani").is_empty(),
            "P7b: idéntica — Elena y Dani nunca co-publicaron"
        );

        // P8: enriquecida — pero los conteos de las personas que el lote
        // NO tocó se conservan EXACTOS (las Anas fusionadas no inflan a
        // Ana). Elena y Fabio SÍ publicaron en el lote (3→6 y 2→5, el
        // mismo enriquecimiento que pinea el cap-42); su pino está en la
        // lista completa de abajo.
        let p8_ing = q08(i);
        for (persona, conteo) in [("Ana", 4), ("Beto", 3), ("Carla", 2), ("Dani", 2)] {
            assert_eq!(
                p8_ing.iter().find(|(p, _)| p == persona).map(|(_, n)| n),
                Some(&conteo),
                "P8: {persona} conserva su conteo del paso-1 tras la ingesta"
            );
        }
        assert_eq!(
            p8_ing,
            vec![
                ("Ana".to_string(), 4),
                ("Beto".to_string(), 3),
                ("Carla".to_string(), 2),
                ("Dani".to_string(), 2),
                ("Elena".to_string(), 6),
                ("Fabio".to_string(), 5),
                ("Gaby".to_string(), 6),
                ("Hugo".to_string(), 6),
                ("Iris".to_string(), 6),
            ],
            "P8: enriquecida 9 personas / 40 AUTHORED (el lote); los conteos canónicos \
             del paso-1 (Ana 4, Beto 3, Carla 2, Dani 2) idénticos — la fusión de las \
             4 Anas no duplica publicaciones"
        );

        // P9: idéntica — los 3 temas comunes por nombre.
        assert_eq!(
            q09(i, "Ana", "Beto"),
            q09(r, "Ana", "Beto"),
            "P9: idéntica — el lote no AUTHORED para Ana/Beto"
        );

        // P10: idéntica en ambos casos — las CITES del lote son internas.
        assert_eq!(
            q10(
                i,
                "Grafos de conocimiento para agentes",
                "grafos de conocimiento",
                2023
            ),
            q10(
                r,
                "Grafos de conocimiento para agentes",
                "grafos de conocimiento",
                2023
            ),
            "P10: idéntica — ningún paper del lote cita al paso-1"
        );
        assert_eq!(
            q10(
                i,
                "Grafos de conocimiento para agentes",
                "memoria de agentes",
                2023
            ),
            q10(
                r,
                "Grafos de conocimiento para agentes",
                "memoria de agentes",
                2023
            ),
            "P10b: idéntica — el tema «memoria de agentes» sobrevive por nombre al fantasma"
        );
    }

    /// La P2 del cap-41 (orden de firma) responde IGUAL sobre la ingesta
    /// que sobre el builder: la columna `order` de relaciones.csv viaja
    /// hasta la prop de arista y `pregunta_02_autores_en_orden_de_firma`
    /// lee `e.props["order"]` en ambos mundos. Recorre los 12 títulos del
    /// paso-1 con AUTHORED (los 16 órdenes del builder) y pinea los dos
    /// casos del contrato y dos papers del lote (order:1, cap-42).
    #[test]
    fn la_ingesta_preserva_el_orden_de_firma() {
        let referencia = kb_lira_paso1();
        let (ingesta, _, _) = cargar_paso5();
        let r = &referencia;
        let i = &ingesta;

        for titulo in [
            "Grafos de conocimiento para agentes",
            "Consultas declarativas sobre property graphs",
            "Memoria episódica en LLMs",
            "Índices adaptativos para grafos",
            "Recuperación aumentada con grafos",
            "Supernodos: anatomía de un cuello de botella",
            "Notas de la reunión de arranque",
            "Bitácora del experimento K-7",
            "Informe anual del Proyecto Kira",
            "Informe de revisión por pares 2025",
            "Informe técnico del Proyecto Oráculo",
            "Resumen del taller de GQL",
        ] {
            assert_eq!(
                q02(i, titulo),
                q02(r, titulo),
                "P2 ({titulo}): el orden de firma del paso-1 sobrevive a la ingesta"
            );
        }

        // Los dos casos del enunciado, con los valores del builder cap-41.
        assert_eq!(
            q02(i, "Supernodos: anatomía de un cuello de botella"),
            vec![("Beto".to_string(), 1), ("Ana".to_string(), 2)],
            "Supernodos: Beto firma primero aunque Ana tenga menor id — el orden \
             viene de la PROP de arista, no del id"
        );
        assert_eq!(
            q02(i, "Memoria episódica en LLMs"),
            vec![("Carla".to_string(), 1), ("Dani".to_string(), 2)],
            "Memoria episódica: Carla firma primero, Dani segunda"
        );

        // El lote (cap-42): los papers del lote llevan order:1.
        assert_eq!(
            q02(i, "Grafos de conocimiento en producción"),
            vec![("Gaby".to_string(), 1)],
            "paper del lote: único autor con order 1 (el pin del cap-42)"
        );
        assert_eq!(
            q02(i, "Memoria de trabajo en agentes con grafos"),
            vec![("Fabio".to_string(), 1)],
            "paper del lote: Fabio firma solo, order 1"
        );
    }

    /// Los pines del cap-42 sobre el modelo ingestado: P1 Ana = 4; la P5
    /// JERÁRQUICA (`documentos_del_tema_incluyendo_subtemas`) responde el
    /// conjunto completo del universo paso-5; P6 Neurónica = 5; P8 = 40
    /// con 9 personas.
    ///
    /// ⚠️ P5 jerárquica: los subtemas SÍ sobreviven a la ingesta — el
    /// pipeline carga las 3 `SUB_TEMA_DE` de `relaciones.csv` (memoria de
    /// agentes, knowledge graphs, GraphRAG → grafos de conocimiento) y la
    /// unión responde **29**, NO 24. El pin 24 del cap-42 media sobre SU
    /// mundo (builder paso-2: los 18 papers del lote con ABOUT DIRECTA al
    /// tema 24 + refactor A moviendo 12); el dataset crudo del paso-5
    /// reparte los ABOUT del lote entre el padre y los subtemas, así que
    /// la unión por jerarquía del mundo ingestado captura 29 documentos
    /// (los 12 directos + los del lote vía subtemas + Bitácora y Memoria
    /// episódica del paso-1, que en el cap-42 no entraban por esta rama).
    /// Ningún documento se pierde: la ingesta ingiere el mundo tal cual,
    /// sin el paso intermedio «supernodo» del cap-42.
    #[test]
    fn las_respuestas_del_paso2_no_cambian_tras_la_ingesta() {
        let (ingesta, _, _) = cargar_paso5();
        let i = &ingesta;

        // P1 (cap-42): Ana firma 4 documentos, EXACTAMENTE los del paso-1.
        assert_eq!(
            q01(i, "Ana"),
            vec![
                "Grafos de conocimiento para agentes",
                "Índices adaptativos para grafos",
                "Notas de la reunión de arranque",
                "Supernodos: anatomía de un cuello de botella",
            ],
            "P1(Ana): los 4 documentos del paso-1 — la fusión de variantes no los altera"
        );

        // P5 jerárquica (cap-42): por NOMBRE del tema (la ingesta asigna
        // ids nuevos), con los subtemas que el pipeline SÍ carga.
        let id_tema = nodo_por_nombre(i, "grafos de conocimiento")
            .expect("el tema «grafos de conocimiento» existe en la ingesta");
        let subtemas: Vec<usize> = i
            .in_edges(id_tema)
            .iter()
            .filter_map(|&eid| i.get_edge(eid))
            .filter(|e| e.label == "SUB_TEMA_DE")
            .map(|e| e.source)
            .collect();
        assert_eq!(
            subtemas.len(),
            3,
            "el pipeline carga las 3 SUB_TEMA_DE (memoria de agentes, knowledge graphs, GraphRAG)"
        );
        let jerarquia = documentos_del_tema_incluyendo_subtemas(i, id_tema);
        assert_eq!(
            jerarquia.len(),
            29,
            "P5 jerárquica: los 29 docs del universo paso-5 (12 directos ∪ subtemas) — \
             el pin 24 del cap-42 media sobre su builder; la ingesta no pasa por el \
             supernodo y captura el conjunto completo del crudo"
        );

        // P6 (cap-42): Instituto Neurónica = 5 menciones, idénticas.
        assert_eq!(
            q06(i, "Instituto Neurónica"),
            vec![
                "Bitácora del experimento K-7",
                "Informe anual del Proyecto Kira",
                "Informe de revisión por pares 2025",
                "Informe técnico del Proyecto Oráculo",
                "Memoria episódica en LLMs",
            ],
            "P6(Instituto Neurónica): las 5 menciones del paso-1 — el lote no añade MENTIONS"
        );

        // P8 (cap-42): 40 AUTHORED con 9 personas; Ana sigue en 4.
        let p8 = q08(i);
        assert_eq!(p8.len(), 9, "P8: las 9 personas del universo paso-5");
        let total: usize = p8.iter().map(|(_, n)| n).sum();
        assert_eq!(total, 40, "P8: 40 AUTHORED — 16 del paso-1 + 24 del lote");
        assert_eq!(
            p8,
            vec![
                ("Ana".to_string(), 4),
                ("Beto".to_string(), 3),
                ("Carla".to_string(), 2),
                ("Dani".to_string(), 2),
                ("Elena".to_string(), 6),
                ("Fabio".to_string(), 5),
                ("Gaby".to_string(), 6),
                ("Hugo".to_string(), 6),
                ("Iris".to_string(), 6),
            ],
            "P8: el pin del cap-42 (9 personas / 40 AUTHORED) se reproduce sobre la ingesta"
        );
    }

    /// Regresión explícita de la puerta final: el modelo ingestado cumple
    /// el esquema declarativo del cap-44 (`verificar_esquema` → `Ok(())`).
    #[test]
    fn el_esquema_del_cap44_acepta_el_modelo_ingestado() {
        let (store, _, _) = cargar_paso5();
        verificar_esquema(&store, &esquema_kb_lira())
            .expect("el modelo ingestado cumple el esquema del cap-44");
    }

    /// El historial de la ingesta es append-only con ts monótono 1..n y
    /// registra las fusiones y los rechazos del contrato. Conteos reales
    /// pineados: 6 `FusionEntidad` (4 Anas + el typo DOC_RAG + el fantasma
    /// 26/61), 4 `RegistroRechazado` (Beto, DOC_RAG, MEMBER_OF duplicada y
    /// la solapada), 6 `ConflictoDeclarado` (el orcid malformado de Ana
    /// entre ellos) y 29 eventos en total.
    #[test]
    fn el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono() {
        let (_, historial, _) = cargar_paso5();

        let fusiones = historial
            .eventos()
            .iter()
            .filter(|(_, e)| matches!(e, EventoIngesta::FusionEntidad { .. }))
            .count();
        let rechazos = historial
            .eventos()
            .iter()
            .filter(|(_, e)| matches!(e, EventoIngesta::RegistroRechazado { .. }))
            .count();
        let conflictos = historial
            .eventos()
            .iter()
            .filter(|(_, e)| matches!(e, EventoIngesta::ConflictoDeclarado { .. }))
            .count();
        let ts: Vec<u64> = historial.eventos().iter().map(|(ts, _)| *ts).collect();

        assert_eq!(
            fusiones, 6,
            "6 FusionEntidad: 4 Anas + typo DOC_RAG + fantasma 26/61"
        );
        assert_eq!(
            rechazos, 4,
            "4 RegistroRechazado: Beto, DOC_RAG, MEMBER_OF y la solapada"
        );
        assert_eq!(
            conflictos, 6,
            "6 ConflictoDeclarado (el orcid malformado entre ellos)"
        );
        assert_eq!(
            ts,
            (1..=ts.len() as u64).collect::<Vec<u64>>(),
            "ts 1..n monótono, append-only"
        );
        assert_eq!(
            ts.len(),
            29,
            "1 CargaIniciada + 11 LoteValidado + 6 + 4 + 6 + 1 CargaCompletada"
        );
    }
}
// ─────────────────── CSV determinista del paso-5 (tests de honestidad) ───────────────────

#[cfg(test)]
mod tests_csv_paso5 {
    use super::*;
    use crate::cap32_import_export::{importar_csv_aristas, importar_csv_nodos};
    use std::io::BufReader;

    // Los tests llevan el nombre EXACTO del contrato; para llamar a las
    // funciones homónimas de cap-41/42/43/44 sin sombra, se re-importan con
    // alias (mismo patrón que los capítulos anteriores).
    use crate::cap41_modelado::{
        csv_aristas_kb_lira as csv_aristas_paso1, csv_nodos_kb_lira as csv_nodos_paso1,
        kb_lira_paso1,
    };
    use crate::cap42_antipatrones::{
        csv_aristas_kb_lira_paso2, csv_nodos_kb_lira_paso2, kb_lira_paso2_degrado,
    };
    use crate::cap43_temporalidad::{
        csv_aristas_kb_lira_paso3, csv_historico, csv_nodos_kb_lira_paso3, historico_kb_lira_paso3,
        kb_lira_paso3,
    };
    use crate::cap44_esquema::{
        csv_aristas_kb_lira_paso4, csv_esquema, csv_nodos_kb_lira_paso4, kb_lira_paso4,
    };

    /// Exporta nodos+aristas del paso-5 → importa (cap. 32) → exporta de
    /// nuevo: bytes IDÉNTICOS (mismo patrón que el roundtrip del cap-41/42/
    /// 43/44). El grafo ingestado es 100% exportable/importable.
    #[test]
    fn csv_paso5_roundtrip_byte_a_byte() {
        let (store, _, _) = cargar_paso5();
        let nodos_v1 = csv_nodos_kb_lira_paso5(&store);
        let aristas_v1 = csv_aristas_kb_lira_paso5(&store);

        // export → fichero temporal → import → export: bytes IDÉNTICOS.
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let ruta_nodos = tmp_dir.path().join("nodes.csv");
        let ruta_aristas = tmp_dir.path().join("edges.csv");
        std::fs::write(&ruta_nodos, &nodos_v1).expect("write nodes");
        std::fs::write(&ruta_aristas, &aristas_v1).expect("write edges");

        let mut s2 = MemoryStore::new();
        let f_nodos = std::fs::File::open(&ruta_nodos).expect("open nodes");
        importar_csv_nodos(&mut BufReader::new(f_nodos), &mut s2).expect("import nodes");
        let f_aristas = std::fs::File::open(&ruta_aristas).expect("open edges");
        importar_csv_aristas(&mut BufReader::new(f_aristas), &mut s2).expect("import edges");

        assert_eq!(s2.node_count(), store.node_count());
        assert_eq!(s2.edge_count(), store.edge_count());
        assert_eq!(csv_nodos_kb_lira_paso5(&s2), nodos_v1);
        assert_eq!(csv_aristas_kb_lira_paso5(&s2), aristas_v1);
    }

    /// datasets/kb-lira/paso-5/ ES la salida de la ingesta: nodes.csv +
    /// edges.csv (formato cap. 32) generados de [`cargar_paso5`] real. Si
    /// alguien regenera el pipeline y olvida commitear, este test grita
    /// (mismo mecanismo que cap-41/42/43/44).
    #[test]
    fn csv_paso5_coincide_con_dataset_commiteado_byte_a_byte() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-5");
        let (store, _, _) = cargar_paso5();

        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso5(&store), esperado_nodos);
        assert_eq!(csv_aristas_kb_lira_paso5(&store), esperado_aristas);
    }

    /// El paso-5 NO toca los datasets de los pasos anteriores: los ficheros
    /// commiteados de datasets/kb-lira/paso-1/ … paso-4/ siguen siendo la
    /// salida EXACTA de los builders del cap-41 al cap-44 (mismo patrón que
    /// `csv_pasos_anteriores_intactos_tras_paso4` del cap-44).
    #[test]
    fn csv_pasos_anteriores_intactos_tras_paso5() {
        let base1 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-1");
        let base2 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-2");
        let base3 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-3");
        let base4 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-4");

        // Paso-1: los builders del cap-41 siguen produciendo los ficheros.
        let s1 = kb_lira_paso1();
        let n1 = std::fs::read_to_string(format!("{base1}/nodes.csv"))
            .expect("dataset paso-1 nodes.csv");
        let e1 = std::fs::read_to_string(format!("{base1}/edges.csv"))
            .expect("dataset paso-1 edges.csv");
        assert_eq!(csv_nodos_paso1(&s1), n1);
        assert_eq!(csv_aristas_paso1(&s1), e1);

        // Paso-2: los builders del cap-42 siguen produciendo los ficheros.
        let s2 = kb_lira_paso2_degrado();
        let n2 = std::fs::read_to_string(format!("{base2}/nodes.csv"))
            .expect("dataset paso-2 nodes.csv");
        let e2 = std::fs::read_to_string(format!("{base2}/edges.csv"))
            .expect("dataset paso-2 edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso2(&s2), n2);
        assert_eq!(csv_aristas_kb_lira_paso2(&s2), e2);

        // Paso-3: los builders del cap-43 siguen produciendo los ficheros.
        let s3 = kb_lira_paso3();
        let n3 = std::fs::read_to_string(format!("{base3}/nodes.csv"))
            .expect("dataset paso-3 nodes.csv");
        let e3 = std::fs::read_to_string(format!("{base3}/edges.csv"))
            .expect("dataset paso-3 edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso3(&s3), n3);
        assert_eq!(csv_aristas_kb_lira_paso3(&s3), e3);
        let esperado_historico = std::fs::read_to_string(format!("{base3}/historico.csv"))
            .expect("dataset paso-3 historico.csv");
        assert_eq!(
            csv_historico(&historico_kb_lira_paso3()),
            esperado_historico
        );

        // Paso-4: el builder del cap-44 produce los ficheros (y el esquema).
        let (s4, esquema4) = kb_lira_paso4();
        let n4 = std::fs::read_to_string(format!("{base4}/nodes.csv"))
            .expect("dataset paso-4 nodes.csv");
        let e4 = std::fs::read_to_string(format!("{base4}/edges.csv"))
            .expect("dataset paso-4 edges.csv");
        let es4 = std::fs::read_to_string(format!("{base4}/esquema.csv"))
            .expect("dataset paso-4 esquema.csv");
        assert_eq!(csv_nodos_kb_lira_paso4(&s4), n4);
        assert_eq!(csv_aristas_kb_lira_paso4(&s4), e4);
        assert_eq!(csv_esquema(&esquema4), es4);
    }
}

// ─────────────────── Informe reproducible (test pineado byte a byte) ───────────────────

#[cfg(test)]
mod tests_informe_ingesta {
    use super::*;

    /// El informe es REPRODUCIBLE: `informe_ingesta_reproducible` produce una
    /// salida estable byte a byte (ficheros → filas → lotes → rechazos,
    /// naive vs bloqueadas, clusters, jaccard/Levenshtein, fusiones,
    /// conflictos, delta de nodos y veredicto del esquema, todo con valores
    /// REALES del pipeline) — el literal de abajo es la salida REAL fijada a
    /// mano.
    #[test]
    fn informe_ingesta_reproducible_sobre_kb_lira() {
        let reporte = informe_ingesta_reproducible();
        let esperado = r#"Informe de ingesta — KB-Lira paso-5 (del CSV crudo al grafo)
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
Ficheros → filas              | personas.csv 14 + documentos.csv 38 + temas.csv 9 + relaciones.csv 160 = 221 filas
Lotes de 25 filas             | 11 lotes
Rechazos (4)                  | línea 15 · clave natural repetida: Beto · línea 7 · clave natural repetida: Recuperación aumentada con grafos · línea 56 · clave natural repetida: Ana → Universidad de Lira (MEMBER_OF) · línea 57 · solape con la 53
Entity resolution · personas  | 91 pares todo-contra-todo → 11 comparadas por bloque inicial (80 evitadas)
Clusters (14 personas crudas) | 5 «Ana»… + 2 «Beto»… + 7 singletons (9 clusters)
Jaccard de bigramas           | «ana garcia»↔«ana g.» 0,4 · «ana garcia»↔«ana garcia (universidad de lira)» 0,3 · contraste «carla mendez» 0,11
Levenshtein (títulos)         | typo «…aumentada con grafo»↔«…grafos» 0,97 · siguiente par legítimo 0,77 («Recuperación aumentada con grafos» ↔ «GraphRAG: recuperación aumentada con grafos»)
Fusiones (6)                  | 4 «Ana» + 1 «Recuperación aumentada con grafos» + 1 «memoria de agentes» (el fantasma)
Conflictos declarados (6)     | orcid 0000-0001-2345-0001 ≠ 0000-0001-2345-1 — gana el canónico, nunca silencioso
Caso Tema 26/61 · curado      | «memoria de agentes» duplicada exacta: 68 → 67 nodos
Grafo final                   | 67 nodos (9 Persona · 8 Tema · 36 Documento · 4 Organizacion · 3 Proyecto · 3 Conferencia · 4 Resena) · 158 aristas
Esquema cap-44                | Ok
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
La ingesta transforma el CRUDO en grafo: 221 filas → 11 lotes → 4 rechazos → 6 fusiones (con 6 conflictos declarados, el orcid malformado entre ellos) → 67 nodos y 158 aristas. Nada se sobrescribió en silencio y el esquema del cap-44 acepta el resultado: el paso-5 es el paso-4 verificado, no una copia suya.
"#;
        assert_eq!(
            reporte, esperado,
            "la salida del informe debe estar pineada byte a byte"
        );
    }
}
