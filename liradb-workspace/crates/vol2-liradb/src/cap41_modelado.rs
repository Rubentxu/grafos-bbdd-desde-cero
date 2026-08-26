//! Vol.III — Cap.41: Modelar entidades, propiedades y relaciones.
//!
//! PRIMER capítulo del Vol.III («Grafos en la era de la IA») y apertura de la
//! Parte I «Modelar datos de grafos». Aquí el motor YA existe y se USA: el
//! modelo Property Graph (cap. 7), el `MemoryStore` (cap. 8), LiraQL
//! end-to-end (caps. 17-21) y el formato CSV (cap. 32) son herramientas, no
//! contenido. El capítulo cobra la deuda del cierre del Vol.II: «¿y cuando la
//! MEMORIA de un agente necesita un grafo?» — esta es la primera piedra: la
//! KB-Lira, base de conocimiento del equipo de investigación que evolucionará
//! hasta ser memoria de agente en el cap. 53.
//!
//! Modelo mental único: **la escalera frase → decisión**, con las consultas
//! previstas como árbitro final. Una PROPIEDAD es un valor que nunca puedes
//! expandir; una ARISTA es una puerta que siempre puedes cruzar — modelar es
//! decidir qué puertas existen ANTES de que te pregunten por el camino.
//!
//! Qué entrega este módulo (contrato §2):
//!
//! 1. **Constructor determinista** [`kb_lira_paso1`] → `MemoryStore` con ids
//!    FIJOS escritos a mano (disciplina del cap. 34: nada de RNG): 6 Personas,
//!    3 Organizaciones, 3 Proyectos, 12 Documentos con labels múltiples
//!    (`Documento`+`Paper`/`Nota`/`Informe`), 6 Temas y 64 aristas
//!    AUTHORED/CITES/ABOUT/MENTIONS/MEMBER_OF/WORKED_ON. AUTHORED lleva la
//!    propiedad de arista `order` (orden de firma). Un Tema («grafos de
//!    conocimiento») acumula 6 aristas ABOUT a propósito: el mini-hub que el
//!    cap. 42 refactorizará.
//! 2. **Validador del modelo como código** ([`validar_modelo_kb_lira`] →
//!    [`Violacion`] con ids): tipos de extremos por tipo de arista y campos
//!    obligatorios de Documento. Semilla de constraints (cap. 44) y shapes
//!    (cap. 47) — hoy, convención ejecutable.
//! 3. **Modelo ingenuo comparable** ([`ModeloDocsTodopropiedades`]): los mismos
//!    documentos con autores/citas/temas/menciones como strings concatenadas,
//!    con contador de LECTURAS. La tesis P6 se mide contra él: naive escanea
//!    TODOS los documentos; el LPG expande SOLO desde el nodo destino.
//! 4. **Las 10 preguntas previstas** (§3), cada una con test de nombre
//!    exacto. Frontera de lenguaje DECLARADA y verificada contra el parser
//!    (caps. 18-19): la gramática mini no tiene `ORDER BY`/`DISTINCT`/
//!    agregación (→ post-proceso en Rust, P2/P8/P9) y —hallazgo de este
//!    capítulo— el lexer del cap. 18 corrompe literales UTF-8 multi-byte
//!    (`scan_string`: `text.push(b as char)`), de modo que los filtros por
//!    títulos/nombres con ACENTOS («Memoria episódica…», «Neurónica») no
//!    pueden expresarse en LiraQL: P2/P3/P6 se responden con API directa
//!    (`in_edges`/`out_edges`), P1/P4/P5/P7/P8/P9/P10 con `run()`.
//! 5. **CSV determinista** reutilizando el formato del cap. 32:
//!    [`csv_nodos_kb_lira`]/[`csv_aristas_kb_lira`] + round-trip byte a byte +
//!    comparación contra el artefacto commiteado `datasets/kb-lira/paso-1/`.
//! 6. **Informe reproducible** ([`informe_modelado_reproducible`]): tabla
//!    pregunta → respuesta(s) → coste, SIN tiempos (decisión #12: sin bench;
//!    la moneda son conjuntos y contadores exactos).
//!
//! Frontera declarada: capa EDUCATIVA fuera del motor — ni `GraphStore` ni el
//! executor se tocan; SIN extensión del parser (si una pregunta no cabe:
//! API/post-proceso + comentario); sin agregación COUNT en LiraQL; sin tipos
//! garantizados en extremos (el validador SUPLE esa ausencia); antipatrones y
//! refactors NO son contenido aquí (cap. 42).

use crate::cap07_modelo::{Edge, Node, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap20_volcano::{Cell, run};
use crate::cap32_import_export::{exportar_csv_aristas, exportar_csv_nodos};

// ─────────────────── KB-Lira paso-1: builder determinista ───────────────────

/// Ids FIJOS del dataset (parte del contrato determinista: cualquier test o
/// prosa puede citarlos literalmente).
///
/// ```text
/// 0-5   Personas        Ana Beto Carla Dani Elena Fabio
/// 6-8   Organizaciones  Universidad de Lira · Instituto Neurónica · Laboratorio GrafosYA
/// 9-11  Proyectos       Proyecto Kira · Proyecto Oráculo · Proyecto Brújula
/// 12-23 Documentos      12-17 Paper · 18-19 Nota · 20-22 Informe · 23 Nota
/// 24-29 Temas           grafos de conocimiento · lenguajes de consulta ·
///                       memoria de agentes · índices y almacenamiento · RAG · rendimiento
/// ```
pub mod ids {
    pub const ANA: usize = 0;
    pub const BETO: usize = 1;
    pub const CARLA: usize = 2;
    pub const DANI: usize = 3;
    pub const ELENA: usize = 4;
    pub const FABIO: usize = 5;
    pub const UNI_LIRA: usize = 6;
    pub const NEURONICA: usize = 7;
    pub const GRAFOS_YA: usize = 8;
    pub const KIRA: usize = 9;
    pub const ORACULO: usize = 10;
    pub const BRUJULA: usize = 11;
    pub const DOC_GRAFOS_AGENTES: usize = 12;
    pub const DOC_CONSULTAS_DECLARATIVAS: usize = 13;
    pub const DOC_MEMORIA_EPISODICA: usize = 14;
    pub const DOC_INDICES_ADAPTATIVOS: usize = 15;
    pub const DOC_RAG: usize = 16;
    pub const DOC_SUPERNODOS: usize = 17;
    pub const DOC_NOTA_ARRANQUE: usize = 18;
    pub const DOC_BITACORA_K7: usize = 19;
    pub const DOC_INFORME_KIRA: usize = 20;
    pub const DOC_REVISION_PARES: usize = 21;
    pub const DOC_INFORME_ORACULO: usize = 22;
    pub const DOC_TALLER_GQL: usize = 23;
    pub const TEMA_GRAFOS_CONOCIMIENTO: usize = 24;
}

/// Nodo con labels arbitrarias (labels múltiples son nativas desde el cap. 7).
fn nodo(id: usize, labels: &[&str], nombre: &str) -> Node {
    let mut n = Node {
        id,
        labels: labels.iter().map(|l| l.to_string()).collect(),
        props: std::collections::HashMap::new(),
    };
    n.props
        .insert("nombre".into(), Value::String(nombre.into()));
    n
}

fn documento(id: usize, sublabel: &str, titulo: &str, anio: i64) -> Node {
    let mut n = nodo(id, &["Documento", sublabel], titulo);
    // Los documentos usan `titulo` como propiedad principal; `nombre` queda
    // alineada con el mismo valor para que las travesías polimórficas P6
    // puedan filtrar por {nombre: …} sin conocer el tipo del destino.
    n.props
        .insert("titulo".into(), Value::String(titulo.into()));
    n.props.insert("anio".into(), Value::Int(anio));
    n
}

fn arista(id: usize, source: usize, target: usize, label: &str, order: Option<i64>) -> Edge {
    let mut e = Edge::new(id, source, target, label);
    if let Some(o) = order {
        e.props.insert("order".into(), Value::Int(o));
    }
    e
}

/// Construye KB-Lira **paso-1**: la base de conocimiento del equipo de
/// investigación, con ids y datos FIJOS (determinismo total, regla del
/// cap. 34). Cada pieza cita la pregunta prevista (§3) que la paga:
///
/// - `AUTHORED{order}` Persona→Documento: P1 (docs de una persona), P2 (orden
///   de firma — atributo DEL VÍNCULO, no reificación), P7/P8/P9.
/// - `CITES` Documento→Documento dirigida: P3 (ambas direcciones), P10.
/// - `ABOUT` Documento→Tema: P5, P9, P10. El tema «grafos de conocimiento»
///   acumula 6 aristas entrantes A PROPOSITO: mini-hub sembrado (gancho cap. 42).
/// - `MENTIONS` Documento→Persona|Organizacion|Proyecto: P6 (destino
///   polimórfico — el validador suple la ausencia de tipos en extremos).
/// - `MEMBER_OF`/`WORKED_ON`: P4 (travesía mixta de 2 saltos).
pub fn kb_lira_paso1() -> MemoryStore {
    let mut s = MemoryStore::new();

    // ── Personas (0-5) ──
    for (id, nombre) in [
        (ids::ANA, "Ana"),
        (ids::BETO, "Beto"),
        (ids::CARLA, "Carla"),
        (ids::DANI, "Dani"),
        (ids::ELENA, "Elena"),
        (ids::FABIO, "Fabio"),
    ] {
        s.put_node(nodo(id, &["Persona"], nombre)).unwrap();
    }

    // ── Organizaciones (6-8) ──
    for (id, nombre) in [
        (ids::UNI_LIRA, "Universidad de Lira"),
        (ids::NEURONICA, "Instituto Neurónica"),
        (ids::GRAFOS_YA, "Laboratorio GrafosYA"),
    ] {
        s.put_node(nodo(id, &["Organizacion"], nombre)).unwrap();
    }

    // ── Proyectos (9-11) ──
    for (id, nombre) in [
        (ids::KIRA, "Proyecto Kira"),
        (ids::ORACULO, "Proyecto Oráculo"),
        (ids::BRUJULA, "Proyecto Brújula"),
    ] {
        s.put_node(nodo(id, &["Proyecto"], nombre)).unwrap();
    }

    // ── Documentos (12-23): labels múltiples `Documento` + subtipo ──
    for (id, sub, titulo, anio) in [
        (
            12usize,
            "Paper",
            "Grafos de conocimiento para agentes",
            2021i64,
        ),
        (
            13,
            "Paper",
            "Consultas declarativas sobre property graphs",
            2022,
        ),
        (14, "Paper", "Memoria episódica en LLMs", 2024),
        (15, "Paper", "Índices adaptativos para grafos", 2023),
        (16, "Paper", "Recuperación aumentada con grafos", 2025),
        (
            17,
            "Paper",
            "Supernodos: anatomía de un cuello de botella",
            2024,
        ),
        (18, "Nota", "Notas de la reunión de arranque", 2023),
        (19, "Nota", "Bitácora del experimento K-7", 2024),
        (20, "Informe", "Informe anual del Proyecto Kira", 2024),
        (21, "Informe", "Informe de revisión por pares 2025", 2025),
        (22, "Informe", "Informe técnico del Proyecto Oráculo", 2023),
        (23, "Nota", "Resumen del taller de GQL", 2025),
    ] {
        s.put_node(documento(id, sub, titulo, anio)).unwrap();
    }

    // ── Temas (24-29) ──
    for (id, nombre) in [
        (ids::TEMA_GRAFOS_CONOCIMIENTO, "grafos de conocimiento"),
        (25, "lenguajes de consulta"),
        (26, "memoria de agentes"),
        (27, "índices y almacenamiento"),
        (28, "RAG"),
        (29, "rendimiento"),
    ] {
        s.put_node(nodo(id, &["Tema"], nombre)).unwrap();
    }

    // ── AUTHORED (16): Persona→Documento con orden de firma ──
    for (id, p, d, order) in [
        (0usize, ids::ANA, ids::DOC_GRAFOS_AGENTES, 1i64),
        (1, ids::BETO, ids::DOC_GRAFOS_AGENTES, 2),
        (2, ids::BETO, ids::DOC_CONSULTAS_DECLARATIVAS, 1),
        (3, ids::CARLA, ids::DOC_MEMORIA_EPISODICA, 1),
        (4, ids::DANI, ids::DOC_MEMORIA_EPISODICA, 2),
        (5, ids::ANA, ids::DOC_INDICES_ADAPTATIVOS, 1),
        (6, ids::ELENA, ids::DOC_RAG, 1),
        (7, ids::FABIO, ids::DOC_RAG, 2),
        (8, ids::BETO, ids::DOC_SUPERNODOS, 1),
        (9, ids::ANA, ids::DOC_SUPERNODOS, 2),
        (10, ids::ANA, ids::DOC_NOTA_ARRANQUE, 1),
        (11, ids::DANI, ids::DOC_BITACORA_K7, 1),
        (12, ids::ELENA, ids::DOC_INFORME_KIRA, 1),
        (13, ids::FABIO, ids::DOC_REVISION_PARES, 1),
        (14, ids::CARLA, ids::DOC_INFORME_ORACULO, 1),
        (15, ids::ELENA, ids::DOC_TALLER_GQL, 1),
    ] {
        s.put_edge(arista(id, p, d, "AUTHORED", Some(order)))
            .unwrap();
    }

    // ── CITES (10): Documento→Documento, dirigida (semántica documental) ──
    for (id, d, cita_a) in [
        (
            16usize,
            ids::DOC_CONSULTAS_DECLARATIVAS,
            ids::DOC_GRAFOS_AGENTES,
        ),
        (17, ids::DOC_MEMORIA_EPISODICA, ids::DOC_GRAFOS_AGENTES),
        (
            18,
            ids::DOC_MEMORIA_EPISODICA,
            ids::DOC_CONSULTAS_DECLARATIVAS,
        ),
        (
            19,
            ids::DOC_INDICES_ADAPTATIVOS,
            ids::DOC_CONSULTAS_DECLARATIVAS,
        ),
        (20, ids::DOC_RAG, ids::DOC_MEMORIA_EPISODICA),
        (21, ids::DOC_RAG, ids::DOC_SUPERNODOS),
        (22, ids::DOC_RAG, ids::DOC_GRAFOS_AGENTES),
        (23, ids::DOC_SUPERNODOS, ids::DOC_CONSULTAS_DECLARATIVAS),
        (24, ids::DOC_SUPERNODOS, ids::DOC_INDICES_ADAPTATIVOS),
        (25, ids::DOC_REVISION_PARES, ids::DOC_RAG),
    ] {
        s.put_edge(arista(id, d, cita_a, "CITES", None)).unwrap();
    }

    // ── ABOUT (16): Documento→Tema. Hub sembrado: tema 24 recibe 6. ──
    for (id, d, t) in [
        (
            26usize,
            ids::DOC_GRAFOS_AGENTES,
            ids::TEMA_GRAFOS_CONOCIMIENTO,
        ),
        (27, ids::DOC_GRAFOS_AGENTES, 26),
        (
            28,
            ids::DOC_CONSULTAS_DECLARATIVAS,
            ids::TEMA_GRAFOS_CONOCIMIENTO,
        ),
        (29, ids::DOC_CONSULTAS_DECLARATIVAS, 25),
        (30, ids::DOC_MEMORIA_EPISODICA, 26),
        (31, ids::DOC_INDICES_ADAPTATIVOS, 27),
        (32, ids::DOC_RAG, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (33, ids::DOC_RAG, 28),
        (34, ids::DOC_SUPERNODOS, 29),
        (35, ids::DOC_NOTA_ARRANQUE, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (36, ids::DOC_BITACORA_K7, 26),
        (37, ids::DOC_INFORME_KIRA, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (38, ids::DOC_REVISION_PARES, 28),
        (39, ids::DOC_REVISION_PARES, 29),
        (40, ids::DOC_INFORME_ORACULO, 25),
        (41, ids::DOC_TALLER_GQL, ids::TEMA_GRAFOS_CONOCIMIENTO),
    ] {
        s.put_edge(arista(id, d, t, "ABOUT", None)).unwrap();
    }

    // ── MENTIONS (10): destino POLIMÓRFICO Persona|Organizacion|Proyecto ──
    for (id, d, e) in [
        (42usize, ids::DOC_NOTA_ARRANQUE, ids::KIRA),
        (43, ids::DOC_NOTA_ARRANQUE, ids::UNI_LIRA),
        (44, ids::DOC_BITACORA_K7, ids::KIRA),
        (45, ids::DOC_BITACORA_K7, ids::NEURONICA),
        (46, ids::DOC_INFORME_KIRA, ids::NEURONICA),
        (47, ids::DOC_INFORME_KIRA, ids::ELENA),
        (48, ids::DOC_REVISION_PARES, ids::NEURONICA),
        (49, ids::DOC_INFORME_ORACULO, ids::ORACULO),
        (50, ids::DOC_INFORME_ORACULO, ids::NEURONICA),
        (51, ids::DOC_MEMORIA_EPISODICA, ids::NEURONICA),
    ] {
        s.put_edge(arista(id, d, e, "MENTIONS", None)).unwrap();
    }

    // ── MEMBER_OF (6): Persona→Organizacion ──
    for (id, p, o) in [
        (52usize, ids::ANA, ids::UNI_LIRA),
        (53, ids::BETO, ids::NEURONICA),
        (54, ids::CARLA, ids::UNI_LIRA),
        (55, ids::DANI, ids::NEURONICA),
        (56, ids::ELENA, ids::GRAFOS_YA),
        (57, ids::FABIO, ids::GRAFOS_YA),
    ] {
        s.put_edge(arista(id, p, o, "MEMBER_OF", None)).unwrap();
    }

    // ── WORKED_ON (6): Persona→Proyecto ──
    for (id, p, pr) in [
        (58usize, ids::ANA, ids::KIRA),
        (59, ids::BETO, ids::KIRA),
        (60, ids::DANI, ids::KIRA),
        (61, ids::CARLA, ids::ORACULO),
        (62, ids::FABIO, ids::ORACULO),
        (63, ids::ELENA, ids::BRUJULA),
    ] {
        s.put_edge(arista(id, p, pr, "WORKED_ON", None)).unwrap();
    }

    s
}

// ─────────────────── Validador del modelo (convención ejecutable) ───────────────────

/// Una violación del contrato del modelo KB-Lira, con el id del elemento
/// implicado (`"nodo"` o `"arista"`). Es la semilla de los constraints del
/// cap. 44 y de los shapes del cap. 47: hoy, convención TESTEABLE.
#[derive(Debug, Clone, PartialEq)]
pub struct Violacion {
    /// Descripción legible del incumplimiento.
    pub descripcion: String,
    /// Id del nodo o arista implicada.
    pub id_implicado: usize,
    /// `"nodo"` o `"arista"`.
    pub tipo_elemento: &'static str,
}

impl std::fmt::Display for Violacion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.tipo_elemento, self.id_implicado, self.descripcion
        )
    }
}

fn tiene_label(n: &Node, label: &str) -> bool {
    n.has_label(label)
}

/// Valida el contrato del modelo sobre `store`:
///
/// - Tipos de extremos por tipo de arista: `AUTHORED` Persona→Documento;
///   `CITES` Documento→Documento; `ABOUT` Documento→Tema; `MENTIONS`
///   →Persona|Organizacion|Proyecto; `MEMBER_OF` Persona→Organizacion;
///   `WORKED_ON` Persona→Proyecto.
/// - Campos obligatorios: todo `Documento` con `titulo` (String no vacío) y
///   `anio` (Int).
/// - `AUTHORED` con propiedad `order` (Int).
///
/// LPG NO garantiza tipos en extremos (Angles-Gutierrez 2018): este validador
/// convierte la convención en test ejecutable. Devuelve `Ok(())` si no hay
/// violaciones o la lista completa (para reportarlas todas de una pasada).
pub fn validar_modelo_kb_lira(store: &dyn GraphStore) -> Result<(), Vec<Violacion>> {
    let mut malas: Vec<Violacion> = Vec::new();

    for ar in store.iter_edges() {
        let src = store.get_node(ar.source);
        let dst = store.get_node(ar.target);
        let (src_ok, dst_ok): (bool, bool) = match ar.label.as_str() {
            "AUTHORED" => (
                src.map(|n| tiene_label(n, "Persona")).unwrap_or(false),
                dst.map(|n| tiene_label(n, "Documento")).unwrap_or(false),
            ),
            "CITES" => (
                src.map(|n| tiene_label(n, "Documento")).unwrap_or(false),
                dst.map(|n| tiene_label(n, "Documento")).unwrap_or(false),
            ),
            "ABOUT" => (
                src.map(|n| tiene_label(n, "Documento")).unwrap_or(false),
                dst.map(|n| tiene_label(n, "Tema")).unwrap_or(false),
            ),
            "MENTIONS" => (
                src.map(|n| tiene_label(n, "Documento")).unwrap_or(false),
                dst.map(|n| {
                    ["Persona", "Organizacion", "Proyecto"]
                        .iter()
                        .any(|l| tiene_label(n, l))
                })
                .unwrap_or(false),
            ),
            "MEMBER_OF" => (
                src.map(|n| tiene_label(n, "Persona")).unwrap_or(false),
                dst.map(|n| tiene_label(n, "Organizacion")).unwrap_or(false),
            ),
            "WORKED_ON" => (
                src.map(|n| tiene_label(n, "Persona")).unwrap_or(false),
                dst.map(|n| tiene_label(n, "Proyecto")).unwrap_or(false),
            ),
            otro => {
                malas.push(Violacion {
                    descripcion: format!("tipo de arista desconocido '{otro}'"),
                    id_implicado: ar.id,
                    tipo_elemento: "arista",
                });
                continue;
            }
        };
        if !src_ok || !dst_ok {
            malas.push(Violacion {
                descripcion: format!(
                    "{}→{} viola los extremos de {}",
                    ar.source, ar.target, ar.label
                ),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            });
        }
        if ar.label == "AUTHORED" && !matches!(ar.props.get("order"), Some(Value::Int(_))) {
            malas.push(Violacion {
                descripcion: "AUTHORED sin propiedad order:Int".into(),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            });
        }
    }

    for n in store.iter_nodes() {
        if tiene_label(n, "Documento") {
            let titulo_ok =
                matches!(n.props.get("titulo"), Some(Value::String(t)) if !t.is_empty());
            let anio_ok = matches!(n.props.get("anio"), Some(Value::Int(_)));
            if !titulo_ok {
                malas.push(Violacion {
                    descripcion: "Documento sin titulo:String".into(),
                    id_implicado: n.id,
                    tipo_elemento: "nodo",
                });
            }
            if !anio_ok {
                malas.push(Violacion {
                    descripcion: "Documento sin anio:Int".into(),
                    id_implicado: n.id,
                    tipo_elemento: "nodo",
                });
            }
        }
    }

    if malas.is_empty() { Ok(()) } else { Err(malas) }
}

// ─────────────────── Localizar nodos por identidad ───────────────────

/// Localiza un nodo por su propiedad `nombre` (escaneo ÚNICO, fuera del
/// ledger de lecturas: localizar QUÉ entidad preguntamos es previo a ambas
/// arquitecturas — naive y LPG parten de la misma pregunta).
pub fn nodo_por_nombre(store: &dyn GraphStore, nombre: &str) -> Option<usize> {
    store
        .iter_nodes()
        .find(|n| n.props.get("nombre") == Some(&Value::String(nombre.into())))
        .map(|n| n.id)
}

// ─────────────────── Modelo ingenuo comparable (todo-propiedades) ───────────────────

/// Un documento del modelo INGENUO: autores/citas/temas/menciones aplastados
/// en strings separadas por `";"`. Funciona en la pizarra… hasta que hay que
/// preguntarle algo.
#[derive(Debug, Clone, PartialEq)]
pub struct DocTodopropiedades {
    pub id: usize,
    pub titulo: String,
    pub anio: i64,
    /// `"Ana;Beto"` — en ORDEN de firma (única gracia que conserva).
    pub autores: String,
    /// títulos citados concatenados: `"Consultas declarativas…;Índices…"`.
    pub citas: String,
    pub temas: String,
    /// nombres mencionados: `"Instituto Neurónica;Elena"`.
    pub menciones: String,
}

/// El modelo ingenuo COMPLETO con contador global de LECTURAS.
///
/// Se construye DESDE el mismo `MemoryStore` (fuente única de verdad: ambos
/// modelos describen los mismos hechos). La construcción NO cuenta lecturas;
/// el duelo es en tiempo de CONSULTA — ahí es donde el modelo cobra lo suyo.
#[derive(Debug, Clone, Default)]
pub struct ModeloDocsTodopropiedades {
    pub docs: Vec<DocTodopropiedades>,
    /// Accesos a registro realizados por las consultas (P6 y similares).
    pub lecturas: u64,
}

impl ModeloDocsTodopropiedades {
    /// Aplasta el grafo a filas todo-propiedades (en orden de id de documento).
    pub fn desde_store(store: &dyn GraphStore) -> Self {
        let mut docs = Vec::new();
        for n in store.iter_nodes().filter(|n| n.has_label("Documento")) {
            let titulo = match n.props.get("titulo") {
                Some(Value::String(t)) => t.clone(),
                _ => String::new(),
            };
            let anio = match n.props.get("anio") {
                Some(Value::Int(a)) => *a,
                _ => 0,
            };
            // autores en orden de firma (lo único que un string SÍ conserva).
            let mut firmas: Vec<(i64, String)> = Vec::new();
            let mut citas: Vec<String> = Vec::new();
            for eid in store.out_edges(n.id) {
                let e = store.get_edge(eid).expect("edge id válido");
                match e.label.as_str() {
                    "AUTHORED" => {
                        let order = match e.props.get("order") {
                            Some(Value::Int(o)) => *o,
                            _ => i64::MAX,
                        };
                        if let Some(autor) = store.get_node(e.source)
                            && let Some(Value::String(nombre)) = autor.props.get("nombre")
                        {
                            firmas.push((order, nombre.clone()));
                        }
                    }
                    "CITES" => {
                        if let Some(citado) = store.get_node(e.target)
                            && let Some(Value::String(t)) = citado.props.get("titulo")
                        {
                            citas.push(t.clone());
                        }
                    }
                    _ => {}
                }
            }
            firmas.sort_by_key(|(o, _)| *o);
            let temas: Vec<String> = store
                .out_edges(n.id)
                .iter()
                .filter_map(|&eid| store.get_edge(eid))
                .filter(|e| e.label == "ABOUT")
                .filter_map(|e| store.get_node(e.target))
                .filter_map(|t| t.props.get("nombre").cloned())
                .filter_map(|v| match v {
                    Value::String(s) => Some(s),
                    _ => None,
                })
                .collect();
            let menciones: Vec<String> = store
                .out_edges(n.id)
                .iter()
                .filter_map(|&eid| store.get_edge(eid))
                .filter(|e| e.label == "MENTIONS")
                .filter_map(|e| store.get_node(e.target))
                .filter_map(|m| m.props.get("nombre").cloned())
                .filter_map(|v| match v {
                    Value::String(s) => Some(s),
                    _ => None,
                })
                .collect();
            docs.push(DocTodopropiedades {
                id: n.id,
                titulo,
                anio,
                autores: firmas
                    .into_iter()
                    .map(|(_, f)| f)
                    .collect::<Vec<_>>()
                    .join(";"),
                citas: citas.join(";"),
                temas: temas.join(";"),
                menciones: menciones.join(";"),
            });
        }
        docs.sort_by_key(|d| d.id);
        Self { docs, lecturas: 0 }
    }

    /// P6 versión ingenua: ¿qué documentos mencionan a `nombre`?
    ///
    /// Sin identidad propia NO hay dónde expandir: toca ESCANEAR todos los
    /// documentos y partir el string campo a campo. Coste exacto: 1 lectura
    /// por documento examinado (todas).
    pub fn menciones_de(&mut self, nombre: &str) -> Vec<String> {
        let mut respuesta = Vec::new();
        for doc in &self.docs {
            self.lecturas += 1; // acceso al registro del documento
            let mencionado = doc.menciones.split(';').any(|m| m.trim() == nombre);
            if mencionado {
                respuesta.push(doc.titulo.clone());
            }
        }
        ordenar_natural(&mut respuesta);
        respuesta
    }
}

/// P6 versión LPG: expande SOLO desde el nodo destino.
///
/// Coste exacto (ledger declarado): 1 lectura por arista **MENTIONS**
/// entrante examinada (= grado entrante de ese tipo: 5 para «Instituto
/// Neurónica», que además recibe 2 `MEMBER_OF` entrantes que la consulta
/// nunca toca — el ledger cuenta solo lo que la consulta LEE). Localizar el
/// id por identidad es O(1) con ids fijos / IndexSeek (cap. 21) y queda
/// FUERA del ledger — el modelo naive ni siquiera puede plantearse ese
/// paso: no tiene nodos.
pub fn menciones_lpg_con_coste(store: &dyn GraphStore, nombre_entidad: &str) -> (Vec<String>, u64) {
    let id = nodo_por_nombre(store, nombre_entidad).expect("entidad conocida");
    let mut lecturas: u64 = 0;
    let mut titulos: Vec<String> = store
        .in_edges(id)
        .iter()
        .filter_map(|&eid| store.get_edge(eid))
        .filter(|e| {
            if e.label == "MENTIONS" {
                lecturas += 1;
                true
            } else {
                false
            }
        })
        .filter_map(|e| store.get_node(e.source))
        .filter_map(|d| d.props.get("titulo").cloned())
        .filter_map(|v| match v {
            Value::String(s) => Some(s),
            _ => None,
        })
        .collect();
    ordenar_natural(&mut titulos);
    (titulos, lecturas)
}

// ─────────────────── Las 10 preguntas previstas (§3) ─────────────────── //
//
// TODAS se responden con `run()` LiraQL (caps. 18-21). Donde la gramática
// mini no llega, el post-proceso en Rust está DECLARADO en la función:
//   - P2: sin ORDER BY en LiraQL → ordenar por `order` en Rust.
//   - P8: sin agregación COUNT → contar filas del ResultSet en Rust.
//   - P9: sin DISTINCT → deduplicar temas en Rust.
// Restricción estructural descubierta al verificar el parser (cap19):
// dos patrones separados por coma NO pueden compartir variables
// (`SharedPatternVariables`) — toda consulta multi-salto se expresa como
// UNA sola cadena con direcciones mixtas.

/// Extrae la columna `col` como Vec<String> (celdas escalares String).
fn columna_texto(rs: &crate::cap20_volcano::ResultSet, col: usize) -> Vec<String> {
    rs.rows
        .iter()
        .filter_map(|fila| fila.get(col))
        .filter_map(|c| match c {
            Cell::Scalar(Value::String(s)) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Extrae la columna `col` como Vec<i64> (celdas escalares Int).
fn columna_entero(rs: &crate::cap20_volcano::ResultSet, col: usize) -> Vec<i64> {
    rs.rows
        .iter()
        .filter_map(|fila| fila.get(col))
        .filter_map(|c| match c {
            Cell::Scalar(Value::Int(i)) => Some(*i),
            _ => None,
        })
        .collect()
}

/// Clave de orden «natural» para la prosa: minúsculas y sin diacríticos.
///
/// El `sort` de bytes de Rust pondría «Índices» (0xC3 0x8D) DESPUÉS de «Z»,
/// lo que volvería las respuestas difíciles de leer en el libro. Con esta
/// clave, «Índices» ordena donde un lector espera: entre «G» y «N».
fn clave_orden(texto: &str) -> String {
    texto
        .chars()
        .map(|c| match c {
            'á' | 'Á' => 'a',
            'é' | 'É' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'Ó' => 'o',
            'ú' | 'Ú' => 'u',
            'ü' | 'Ü' => 'u',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Ordena `v` con [`clave_orden`] (estable: empates conservan el orden del
/// ResultSet, que es el orden de exploración del motor).
fn ordenar_natural(v: &mut [String]) {
    v.sort_by_cached_key(|a| clave_orden(a));
}

/// P1: ¿Qué documentos ha escrito Ana?
///
/// `AUTHORED` saliente desde Persona. PURA LiraQL (el nombre «Ana» es ASCII,
/// cabe en la gramática mini; el post-proceso solo ordena para la prosa).
pub fn pregunta_01_documentos_de_una_persona(store: &dyn GraphStore, persona: &str) -> Vec<String> {
    let q = format!(
        "MATCH (p:Persona {{nombre:\"{persona}\"}})-[:AUTHORED]->(d:Documento) \
         RETURN d.titulo AS titulo"
    );
    let rs = run(&q, store).expect("P1 ejecuta");
    let mut v = columna_texto(&rs, 0);
    ordenar_natural(&mut v);
    v
}

/// P2: ¿Quiénes firman el paper X y EN QUÉ ORDEN?
///
/// La propiedad DE ARISTA `order` viaja en el RETURN (`a.order`).
///
/// FRONTERA DECLARADA (verificada en el parser, caps. 18-19): la gramática
/// mini no tiene ORDER BY → el orden final lo fija el sort por `order` en
/// Rust. Y el filtro se escribe con API directa: el lexer del cap. 18
/// (cap18_lexer_parser.rs, `scan_string`: `text.push(b as char)`) corrompe
/// literales UTF-8 multi-byte — «Memoria episódica…» se lexea como
/// «…episÃ³dica…» y NUNCA matchea el título real del grafo. Los títulos del
/// dataset llevan acentos, así que los filtros por título se resuelven en
/// Rust con el índice de nodos del store.
pub fn pregunta_02_autores_en_orden_de_firma(
    store: &dyn GraphStore,
    titulo: &str,
) -> Vec<(String, i64)> {
    let doc = store
        .iter_nodes()
        .find(|n| {
            n.has_label("Documento")
                && n.props.get("titulo") == Some(&Value::String(titulo.to_string()))
        })
        .expect("documento conocido en KB-Lira");
    let mut parejas: Vec<(String, i64)> = store
        .in_edges(doc.id)
        .iter()
        .filter_map(|&eid| store.get_edge(eid))
        .filter(|e| e.label == "AUTHORED")
        .filter_map(|e| {
            e.props.get("order").and_then(|v| match v {
                Value::Int(o) => Some((e.source, *o)),
                _ => None,
            })
        })
        .filter_map(|(src, o)| store.get_node(src).map(|p| (p, o)))
        .filter_map(|(p, o)| match p.props.get("nombre") {
            Some(Value::String(n)) => Some((n.clone(), o)),
            _ => None,
        })
        .collect();
    parejas.sort_by_key(|(_, o)| *o);
    parejas
}

/// P3: ¿A quién cita el paper X y quién cita AL paper X?
///
/// Dos consultas DIRIGIDAS espejo (la dirección ES la semántica: quien cita
/// no es citado). FRONTERA DECLARADA: mismo motivo que P2 — el título de
/// consulta lleva acentos («…anatomía…») que el lexer mini corrompe; las
/// dos direcciones se leen con `out_edges`/`in_edges` (cap. 8) y se ordenan
/// de forma natural.
pub fn pregunta_03_citas_en_ambas_direcciones(
    store: &dyn GraphStore,
    titulo: &str,
) -> (Vec<String>, Vec<String>) {
    let doc = store
        .iter_nodes()
        .find(|n| {
            n.has_label("Documento")
                && n.props.get("titulo") == Some(&Value::String(titulo.to_string()))
        })
        .expect("documento conocido en KB-Lira");
    let titulo_de = |nid: usize| -> Option<String> {
        store
            .get_node(nid)
            .and_then(|n| match n.props.get("titulo") {
                Some(Value::String(t)) => Some(t.clone()),
                _ => None,
            })
    };
    let mut salientes: Vec<String> = store
        .out_edges(doc.id)
        .iter()
        .filter_map(|&eid| store.get_edge(eid))
        .filter(|e| e.label == "CITES")
        .filter_map(|e| titulo_de(e.target))
        .collect();
    let mut entrantes: Vec<String> = store
        .in_edges(doc.id)
        .iter()
        .filter_map(|&eid| store.get_edge(eid))
        .filter(|e| e.label == "CITES")
        .filter_map(|e| titulo_de(e.source))
        .collect();
    ordenar_natural(&mut salientes);
    ordenar_natural(&mut entrantes);
    (salientes, entrantes)
}

/// P4: ¿Quién trabaja en el proyecto Y y en qué organización está afiliado?
///
/// Travesía MIXTA de 2 saltos en UNA cadena (entrante WORKED_ON + saliente
/// MEMBER_OF). PURA LiraQL.
pub fn pregunta_04_proyecto_y_afiliaciones_en_dos_saltos(
    store: &dyn GraphStore,
    proyecto: &str,
) -> Vec<(String, String)> {
    let q = format!(
        "MATCH (pr:Proyecto {{nombre:\"{proyecto}\"}})<-[:WORKED_ON]-(p:Persona)\
         -[:MEMBER_OF]->(o:Organizacion) RETURN p.nombre AS persona, o.nombre AS org"
    );
    let rs = run(&q, store).expect("P4 ejecuta");
    let mut filas: Vec<(String, String)> = columna_texto(&rs, 0)
        .into_iter()
        .zip(columna_texto(&rs, 1))
        .collect();
    filas.sort_by_key(|a| clave_orden(&a.0));
    filas
}

/// P5: ¿De qué temas trata el documento Z (y qué documentos tratan el tema T)?
///
/// ABOUT en ambas direcciones. PURA LiraQL (dos consultas espejo).
pub fn pregunta_05_temas_de_un_documento_e_inversa(
    store: &dyn GraphStore,
    titulo_doc: &str,
    nombre_tema: &str,
) -> (Vec<String>, Vec<String>) {
    let temas = run(
        &format!(
            "MATCH (d:Documento {{titulo:\"{titulo_doc}\"}})-[:ABOUT]->(t:Tema) \
             RETURN t.nombre AS tema"
        ),
        store,
    )
    .expect("P5 temas");
    let docs = run(
        &format!(
            "MATCH (t:Tema {{nombre:\"{nombre_tema}\"}})<-[:ABOUT]-(d:Documento) \
             RETURN d.titulo AS documento"
        ),
        store,
    )
    .expect("P5 inversa");
    let mut a = columna_texto(&temas, 0);
    ordenar_natural(&mut a);
    let mut b = columna_texto(&docs, 0);
    ordenar_natural(&mut b);
    (a, b)
}

/// P6: ¿Qué documentos mencionan a esta entidad (persona/org/proyecto)?
///
/// FRONTERA DECLARADA: el destino es POLIMÓRFICO (→Persona|Organizacion|
/// Proyecto, decisión #7) y además casi todos los nombres de entidad llevan
/// acentos que el lexer mini corrompe («Neurónica» → «NeurÃ³nica»). El
/// patrón `(x {{nombre:"…"}})` sin etiqueta SÍ existe en la gramática, pero
/// no resiste UTF-8: esta pregunta se responde con `in_edges` directo — la
/// expansión entrante es, de hecho, la lectura barata que la tesis P6 mide.
pub fn pregunta_06_menciones_a_una_entidad(
    store: &dyn GraphStore,
    nombre_entidad: &str,
) -> Vec<String> {
    let id = nodo_por_nombre(store, nombre_entidad).expect("entidad conocida en KB-Lira");
    let mut titulos: Vec<String> = store
        .in_edges(id)
        .iter()
        .filter_map(|&eid| store.get_edge(eid))
        .filter(|e| e.label == "MENTIONS")
        .filter_map(|e| store.get_node(e.source))
        .filter_map(|d| match d.props.get("titulo") {
            Some(Value::String(t)) => Some(t.clone()),
            _ => None,
        })
        .collect();
    ordenar_natural(&mut titulos);
    titulos
}

/// P7: ¿Han co-publicado Ana y Beto alguna vez?
///
/// Cadena con arista entrante: `(a)-[:AUTHORED]->(d)<-[:AUTHORED]-(b)`.
/// PURA LiraQL.
pub fn pregunta_07_copublicacion_entre_dos_personas(
    store: &dyn GraphStore,
    a: &str,
    b: &str,
) -> Vec<String> {
    let q = format!(
        "MATCH (x:Persona {{nombre:\"{a}\"}})-[:AUTHORED]->(d:Documento)\
         <-[:AUTHORED]-(y:Persona {{nombre:\"{b}\"}}) RETURN d.titulo AS conjunto"
    );
    let rs = run(&q, store).expect("P7 ejecuta");
    let mut v = columna_texto(&rs, 0);
    ordenar_natural(&mut v);
    v
}

/// P8: ¿Cuántas publicaciones tiene cada miembro del equipo?
///
/// FRONTERA DECLARADA: LiraQL mini no tiene agregación COUNT (caps. 17-21:
/// scan/expand/filter/project/cartesian — sin GROUP BY). El conteo de filas
/// por autor se hace DESDE Rust sobre el ResultSet.
pub fn pregunta_08_publicaciones_por_persona_contadas(
    store: &dyn GraphStore,
) -> Vec<(String, usize)> {
    let rs = run(
        "MATCH (p:Persona)-[:AUTHORED]->(d:Documento) \
         RETURN p.nombre AS autor, d.titulo AS titulo",
        store,
    )
    .expect("P8 ejecuta");
    let autores = columna_texto(&rs, 0);
    let mut conteo: Vec<(String, usize)> = Vec::new();
    for autor in autores {
        match conteo.iter_mut().find(|(a, _)| *a == autor) {
            Some((_, n)) => *n += 1,
            None => conteo.push((autor, 1)),
        }
    }
    conteo.sort();
    conteo
}

/// P9: ¿Qué temas conectan a Ana con Beto vía sus papers?
///
/// Camino de 4 saltos con direcciones mixtas:
/// `Persona→Documento→Tema←Documento←Persona`. FRONTERA: sin DISTINCT en la
/// gramática → deduplicar temas en Rust.
pub fn pregunta_09_temas_comunes_via_papers(
    store: &dyn GraphStore,
    a: &str,
    b: &str,
) -> Vec<String> {
    let q = format!(
        "MATCH (x:Persona {{nombre:\"{a}\"}})-[:AUTHORED]->(d1:Documento)\
         -[:ABOUT]->(t:Tema)<-[:ABOUT]-(d2:Documento)<-[:AUTHORED]-(y:Persona {{nombre:\"{b}\"}}) \
         RETURN t.nombre AS tema"
    );
    let rs = run(&q, store).expect("P9 ejecuta");
    let mut temas = columna_texto(&rs, 0);
    ordenar_natural(&mut temas);
    temas.dedup();
    temas
}

/// P10: ¿Qué papers posteriores a 2023 citan al paper P y tratan del tema T?
///
/// Cadena `Tema←ABOUT—paper—CITES→paper` + filtro por propiedad `anio`.
/// El label `:Paper` (subtipo) despacha directamente en MATCH — esa es la
/// paga de las labels múltiples (decisión #6). PURA LiraQL.
pub fn pregunta_10_citas_recientes_que_tratan_un_tema(
    store: &dyn GraphStore,
    titulo_citado: &str,
    tema: &str,
    anio_minimo: i64,
) -> Vec<(String, i64)> {
    let q = format!(
        "MATCH (t:Tema {{nombre:\"{tema}\"}})<-[:ABOUT]-(p:Paper)\
         -[:CITES]->(c:Paper {{titulo:\"{titulo_citado}\"}}) \
         WHERE p.anio > {anio_minimo} RETURN p.titulo AS titulo, p.anio AS anio"
    );
    let rs = run(&q, store).expect("P10 ejecuta");
    let mut filas: Vec<(String, i64)> = columna_texto(&rs, 0)
        .into_iter()
        .zip(columna_entero(&rs, 1))
        .collect();
    filas.sort_by_key(|a| clave_orden(&a.0));
    filas
}

// ─────────────────── CSV determinista (formato del cap. 32) ───────────────────

/// Exporta los NODOS de KB-Lira al formato CSV estilo neo4j-admin del cap. 32
/// (cabecera = unión BTreeMap de props; `:LABEL` con labels unidas por `:`).
pub fn csv_nodos_kb_lira(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_nodos(store, &mut buf).expect("export nodos");
    String::from_utf8(buf).expect("CSV UTF-8")
}

/// Exporta las ARISTAS de KB-Lira (mismo contrato que [`csv_nodos_kb_lira`]).
pub fn csv_aristas_kb_lira(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_aristas(store, &mut buf).expect("export aristas");
    String::from_utf8(buf).expect("CSV UTF-8")
}

// ─────────────────── Informe reproducible (para la prosa) ───────────────────

/// Tabla pregunta → respuesta(s) → coste, calculada VIVA sobre el store.
/// Sin tiempos (decisión #12): los enteros SON la física del modelo. Esta
/// salida es la que la prosa pega LITERALMENTE.
pub fn informe_modelado_reproducible(store: &MemoryStore) -> String {
    let mut lineas: Vec<String> = Vec::new();
    lineas.push("=== Informe de modelado reproducible (cap. 41) ===".to_string());
    lineas.push(format!(
        "dataset: KB-Lira paso-1 | {} nodos | {} aristas | validador: {}",
        store.node_count(),
        store.edge_count(),
        match validar_modelo_kb_lira(store) {
            Ok(()) => "OK".to_string(),
            Err(v) => format!("{} violaciones", v.len()),
        }
    ));
    lineas.push(
        "-- preguntas previstas -> respuestas (contadores exactos; sin tiempos) --".to_string(),
    );

    let fmt_lista = |v: &[String]| {
        if v.is_empty() {
            "(ninguno)".to_string()
        } else {
            v.join(" | ")
        }
    };

    let r1 = pregunta_01_documentos_de_una_persona(store, "Ana");
    lineas.push(format!(
        "P01 documentos_de_una_persona(Ana): [{}] ({} filas)",
        fmt_lista(&r1),
        r1.len()
    ));

    let r2 = pregunta_02_autores_en_orden_de_firma(store, "Memoria episódica en LLMs");
    let r2s: Vec<String> = r2.iter().map(|(a, o)| format!("{a}#{o}")).collect();
    lineas.push(format!(
        "P02 autores_en_orden(Memoria episódica en LLMs): [{}] ({} filas)",
        r2s.join(", "),
        r2.len()
    ));

    let (r3a, r3b) = pregunta_03_citas_en_ambas_direcciones(
        store,
        "Supernodos: anatomía de un cuello de botella",
    );
    lineas.push(format!(
        "P03 citas_bidireccionales(Supernodos…): sale=[{}] entra=[{}]",
        fmt_lista(&r3a),
        fmt_lista(&r3b)
    ));

    let r4 = pregunta_04_proyecto_y_afiliaciones_en_dos_saltos(store, "Proyecto Kira");
    let r4s: Vec<String> = r4.iter().map(|(p, o)| format!("{p}@{o}")).collect();
    lineas.push(format!(
        "P04 proyecto_y_afiliaciones(Kira): [{}]",
        r4s.join(", ")
    ));

    let (r5a, r5b) = pregunta_05_temas_de_un_documento_e_inversa(
        store,
        "Informe anual del Proyecto Kira",
        "grafos de conocimiento",
    );
    lineas.push(format!(
        "P05 temas_e_inversa(Informe Kira | grafos de conocimiento): temas=[{}] docs=[{}] (hub: {} documentos)",
        fmt_lista(&r5a),
        fmt_lista(&r5b),
        r5b.len()
    ));

    let r6 = pregunta_06_menciones_a_una_entidad(store, "Instituto Neurónica");
    let (_, lecturas_lpg) = menciones_lpg_con_coste(store, "Instituto Neurónica");
    let mut naive = ModeloDocsTodopropiedades::desde_store(store);
    let _ = naive.menciones_de("Instituto Neurónica");
    lineas.push(format!(
        "P06 menciones(Instituto Neurónica): [{}] | coste naive: {} lecturas vs LPG: {lecturas_lpg} lecturas",
        fmt_lista(&r6),
        naive.lecturas
    ));

    let r7 = pregunta_07_copublicacion_entre_dos_personas(store, "Ana", "Beto");
    lineas.push(format!(
        "P07 copublicacion(Ana,Beto): [{}] ({} conjuntos)",
        fmt_lista(&r7),
        r7.len()
    ));

    let r8 = pregunta_08_publicaciones_por_persona_contadas(store);
    let total: usize = r8.iter().map(|(_, n)| n).sum();
    let r8s: Vec<String> = r8.iter().map(|(a, n)| format!("{a}:{n}")).collect();
    lineas.push(format!(
        "P08 publicaciones_por_persona: [{}] (total {} AUTHORED)",
        r8s.join(", "),
        total
    ));

    let r9 = pregunta_09_temas_comunes_via_papers(store, "Ana", "Beto");
    lineas.push(format!(
        "P09 temas_comunes_via_papers(Ana,Beto): [{}]",
        fmt_lista(&r9)
    ));

    let r10 = pregunta_10_citas_recientes_que_tratan_un_tema(
        store,
        "Grafos de conocimiento para agentes",
        "grafos de conocimiento",
        2023,
    );
    let r10s: Vec<String> = r10.iter().map(|(t, a)| format!("{t} ({a})")).collect();
    lineas.push(format!(
        "P10 citas_recientes_que_tratan_un_tema(cita Grafos…, tema grafos…, >2023): [{}]",
        r10s.join(", ")
    ));

    lineas.push(
        "(costes en lecturas/filas EXACTAS; sin cronómetro — decisión #12 del contrato)"
            .to_string(),
    );

    let mut informe = lineas.join("\n");
    informe.push('\n');
    informe
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_modelado {
    use super::*;
    use crate::cap08_graph_store::StoreError;
    use crate::cap32_import_export::{importar_csv_aristas, importar_csv_nodos};
    use std::io::BufReader;

    // Los tests llevan el nombre EXACTO del contrato; para llamar a las
    // funciones homónimas sin sombra, se re-importan con alias.
    use super::{
        pregunta_01_documentos_de_una_persona as q01, pregunta_02_autores_en_orden_de_firma as q02,
        pregunta_03_citas_en_ambas_direcciones as q03,
        pregunta_04_proyecto_y_afiliaciones_en_dos_saltos as q04,
        pregunta_05_temas_de_un_documento_e_inversa as q05,
        pregunta_06_menciones_a_una_entidad as q06,
        pregunta_07_copublicacion_entre_dos_personas as q07,
        pregunta_08_publicaciones_por_persona_contadas as q08,
        pregunta_09_temas_comunes_via_papers as q09,
        pregunta_10_citas_recientes_que_tratan_un_tema as q10,
    };

    #[test]
    fn estructura_de_kb_lira_paso1_cuenta_y_etiquetas_exactas() {
        let s = kb_lira_paso1();

        assert_eq!(s.node_count(), 30);
        assert_eq!(s.edge_count(), 64);

        let cuenta_label = |label: &str| s.iter_nodes().filter(|n| n.has_label(label)).count();
        assert_eq!(cuenta_label("Persona"), 6);
        assert_eq!(cuenta_label("Organizacion"), 3);
        assert_eq!(cuenta_label("Proyecto"), 3);
        assert_eq!(cuenta_label("Documento"), 12);
        assert_eq!(cuenta_label("Paper"), 6);
        assert_eq!(cuenta_label("Nota"), 3);
        assert_eq!(cuenta_label("Informe"), 3);
        assert_eq!(cuenta_label("Tema"), 6);

        let cuenta_tipo = |tipo: &str| s.iter_edges().filter(|e| e.label == tipo).count();
        assert_eq!(cuenta_tipo("AUTHORED"), 16);
        assert_eq!(cuenta_tipo("CITES"), 10);
        assert_eq!(cuenta_tipo("ABOUT"), 16);
        assert_eq!(cuenta_tipo("MENTIONS"), 10);
        assert_eq!(cuenta_tipo("MEMBER_OF"), 6);
        assert_eq!(cuenta_tipo("WORKED_ON"), 6);

        // Labels múltiples nativas: el paper 12 ES Documento Y Paper.
        let d12 = s.get_node(ids::DOC_GRAFOS_AGENTES).unwrap();
        assert!(d12.has_label("Documento") && d12.has_label("Paper"));
        assert!(!d12.has_label("Nota"));

        // Todo AUTHORED lleva order:Int en 1..=2.
        for e in s.iter_edges().filter(|e| e.label == "AUTHORED") {
            match e.props.get("order") {
                Some(Value::Int(o)) => assert!((1..=2).contains(o), "order {o} fuera de rango"),
                otro => panic!("AUTHORED {:?} sin order:Int", otro),
            }
        }

        // Hub sembrado: el tema «grafos de conocimiento» acumula 6 ABOUT.
        let hub = s
            .in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO)
            .iter()
            .filter(|&&eid| s.get_edge(eid).unwrap().label == "ABOUT")
            .count();
        assert_eq!(hub, 6, "mini-supernodo sembrado para el cap. 42");

        // Determinismo total: dos llamadas, mismo grafo.
        let s2 = kb_lira_paso1();
        assert_eq!(csv_nodos_kb_lira(&s), csv_nodos_kb_lira(&s2));
        assert_eq!(csv_aristas_kb_lira(&s), csv_aristas_kb_lira(&s2));
    }

    #[test]
    fn validador_acepta_el_modelo_canonico() {
        let s = kb_lira_paso1();
        assert_eq!(validar_modelo_kb_lira(&s), Ok(()));
    }

    #[test]
    fn validador_rechaza_fixture_corrupto() {
        // Grafo roto A MANO: CITES hacia Tema, AUTHORED Tema→Documento,
        // MENTIONS hacia Tema, documento sin anio, AUTHORED sin order.
        let mut s = MemoryStore::new();
        s.put_node(nodo(0, &["Documento"], "Doc roto")).unwrap();
        let mut doc_sin_anio = nodo(1, &["Documento", "Paper"], "Sin año");
        doc_sin_anio
            .props
            .insert("titulo".into(), Value::String("Sin año".into()));
        // sin `anio`: debe saltar la otra violación
        s.put_node(doc_sin_anio).unwrap();
        s.put_node(nodo(2, &["Tema"], "tema intruso")).unwrap();
        s.put_edge(Edge::new(0, 0, 2, "CITES")).unwrap(); // Documento→Tema ✗
        s.put_edge(Edge::new(1, 2, 0, "AUTHORED")).unwrap(); // Tema→Documento ✗ (y sin order)
        s.put_edge(Edge::new(2, 1, 2, "MENTIONS")).unwrap(); // →Tema ✗

        let err = validar_modelo_kb_lira(&s).expect_err("fixture corrupto");
        // Arista 0: CITES con destino Tema.
        assert!(
            err.iter()
                .any(|v| v.id_implicado == 0 && v.tipo_elemento == "arista")
        );
        // Arista 1: AUTHORED con source Tema (además de falta de order).
        assert!(
            err.iter()
                .any(|v| v.id_implicado == 1 && v.tipo_elemento == "arista")
        );
        // Arista 2: MENTIONS con destino Tema (fuera de Persona|Org|Proyecto).
        assert!(
            err.iter()
                .any(|v| v.id_implicado == 2 && v.tipo_elemento == "arista")
        );
        // Nodo 1: Documento sin anio:Int.
        assert!(err.iter().any(|v| v.id_implicado == 1
            && v.tipo_elemento == "nodo"
            && v.descripcion.contains("anio")));
    }

    #[test]
    fn naive_y_lpg_responden_menciones_con_costes_distintos() {
        let s = kb_lira_paso1();

        let mut naive = ModeloDocsTodopropiedades::desde_store(&s);
        let r_naive = naive.menciones_de("Instituto Neurónica");
        let lecturas_naive = naive.lecturas;

        let (r_lpg, lecturas_lpg) = menciones_lpg_con_coste(&s, "Instituto Neurónica");

        // Misma RESPUESTA (el modelo ingenuo funciona… de momento).
        assert_eq!(r_naive, r_lpg);

        // …pero coste ESTRUCTURAL distinto: naive escaneó TODOS los
        // documentos (12); LPG expandió SOLO el grado entrante (5).
        assert_eq!(lecturas_naive, 12);
        assert_eq!(lecturas_lpg, 5);
        assert_eq!(naive.docs.len(), 12);

        // El escaneo no sabe rendirse antes: aunque la respuesta sea vacía,
        // paga los 12 registros completos.
        let mut grande = ModeloDocsTodopropiedades::desde_store(&s);
        let _ = grande.menciones_de("Nadie");
        assert_eq!(grande.lecturas, 12);
    }

    // ── Las 10 preguntas previstas (salidas reales para la prosa) ──

    #[test]
    fn pregunta_01_documentos_de_una_persona() {
        let s = kb_lira_paso1();
        assert_eq!(
            q01(&s, "Ana"),
            vec![
                "Grafos de conocimiento para agentes",
                "Índices adaptativos para grafos",
                "Notas de la reunión de arranque",
                "Supernodos: anatomía de un cuello de botella",
            ]
        );
        // Nadie que no escribió: lista vacía, no error.
        assert!(q01(&s, "Zoe").is_empty());
    }
    #[test]
    fn pregunta_02_autores_en_orden_de_firma() {
        let s = kb_lira_paso1();
        assert_eq!(
            q02(&s, "Memoria episódica en LLMs"),
            vec![("Carla".to_string(), 1), ("Dani".to_string(), 2)]
        );
        // En el paper de Supernodos firma PRIMERO Beto (order 1) aunque Ana
        // tenga menor id: el orden viene de la PROPIEDAD DE ARISTA.
        assert_eq!(
            q02(&s, "Supernodos: anatomía de un cuello de botella"),
            vec![("Beto".to_string(), 1), ("Ana".to_string(), 2)]
        );
    }

    #[test]
    fn pregunta_03_citas_en_ambas_direcciones() {
        let s = kb_lira_paso1();
        let (cita_a, citado_por) = q03(&s, "Supernodos: anatomía de un cuello de botella");
        assert_eq!(
            cita_a,
            vec![
                "Consultas declarativas sobre property graphs",
                "Índices adaptativos para grafos",
            ]
        );
        assert_eq!(citado_por, vec!["Recuperación aumentada con grafos"]);
    }

    #[test]
    fn pregunta_04_proyecto_y_afiliaciones_en_dos_saltos() {
        let s = kb_lira_paso1();
        assert_eq!(
            q04(&s, "Proyecto Kira"),
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
    }

    #[test]
    fn pregunta_05_temas_de_un_documento_e_inversa() {
        let s = kb_lira_paso1();
        let (temas, docs) = q05(
            &s,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        assert_eq!(temas, vec!["grafos de conocimiento"]);
        // La INVERSA es la cara visible del hub sembrado: 6 documentos,
        // ordenados de forma natural (la «Í» de un hipotético título no
        // descarrila el orden — la prosa quiere respuestas estables).
        assert_eq!(
            docs,
            vec![
                "Consultas declarativas sobre property graphs",
                "Grafos de conocimiento para agentes",
                "Informe anual del Proyecto Kira",
                "Notas de la reunión de arranque",
                "Recuperación aumentada con grafos",
                "Resumen del taller de GQL",
            ]
        );
    }

    #[test]
    fn pregunta_06_menciones_a_una_entidad() {
        let s = kb_lira_paso1();
        // Polimorfismo REAL: misma forma de consulta para org…
        assert_eq!(
            q06(&s, "Instituto Neurónica"),
            vec![
                "Bitácora del experimento K-7",
                "Informe anual del Proyecto Kira",
                "Informe de revisión por pares 2025",
                "Informe técnico del Proyecto Oráculo",
                "Memoria episódica en LLMs",
            ]
        );
        // …persona…
        assert_eq!(q06(&s, "Elena"), vec!["Informe anual del Proyecto Kira"]);
        // …y proyecto, SIN cambiar la consulta.
        assert_eq!(
            q06(&s, "Proyecto Kira"),
            vec![
                "Bitácora del experimento K-7",
                "Notas de la reunión de arranque",
            ]
        );
    }

    #[test]
    fn pregunta_07_copublicacion_entre_dos_personas() {
        let s = kb_lira_paso1();
        // Ana y Beto co-firmaron DOS documentos (multigrafo en acción).
        assert_eq!(
            q07(&s, "Ana", "Beto"),
            vec![
                "Grafos de conocimiento para agentes",
                "Supernodos: anatomía de un cuello de botella",
            ]
        );
        // Elena y Dani nunca co-publicaron.
        assert!(q07(&s, "Elena", "Dani").is_empty());
    }

    #[test]
    fn pregunta_08_publicaciones_por_persona_contadas() {
        let s = kb_lira_paso1();
        assert_eq!(
            q08(&s),
            vec![
                ("Ana".to_string(), 4),
                ("Beto".to_string(), 3),
                ("Carla".to_string(), 2),
                ("Dani".to_string(), 2),
                ("Elena".to_string(), 3),
                ("Fabio".to_string(), 2),
            ]
        );
        // Total de filas del ResultSet = nº de aristas AUTHORED (16).
        let rs = crate::cap20_volcano::run(
            "MATCH (p:Persona)-[:AUTHORED]->(d:Documento) RETURN p.nombre AS autor",
            &s,
        )
        .unwrap();
        assert_eq!(rs.len(), 16);
    }

    #[test]
    fn pregunta_09_temas_comunes_via_papers() {
        let s = kb_lira_paso1();
        // Camino de 4 saltos: Ana→doc→tema←doc←Beto. Comparten «grafos de
        // conocimiento» (varios papeles), «rendimiento» (Supernodos) y
        // «memoria de agentes»: el camino permite d1 == d2 — el paper 12
        // lo firman AMBOS y trata dos temas, así que ambos conectan.
        // La salida real del motor son 3 temas (nada de inventar 2).
        assert_eq!(
            q09(&s, "Ana", "Beto"),
            vec![
                "grafos de conocimiento",
                "memoria de agentes",
                "rendimiento"
            ]
        );
    }

    #[test]
    fn pregunta_10_citas_recientes_que_tratan_un_tema() {
        let s = kb_lira_paso1();
        // Papers >2023 que citan «Grafos de conocimiento para agentes»
        // Y tratan «grafos de conocimiento»: sólo el paper de RAG (2025);
        // Memoria episódica (2024) lo cita pero trata «memoria de agentes».
        assert_eq!(
            q10(
                &s,
                "Grafos de conocimiento para agentes",
                "grafos de conocimiento",
                2023
            ),
            vec![("Recuperación aumentada con grafos".to_string(), 2025)]
        );
        // Cambiando SÓLO el tema, responde «Memoria episódica» (cita al
        // paper fundacional y trata memoria de agentes).
        assert_eq!(
            q10(
                &s,
                "Grafos de conocimiento para agentes",
                "memoria de agentes",
                2023
            ),
            vec![("Memoria episódica en LLMs".to_string(), 2024)]
        );
    }

    // ── Direccionalidad y multigrafos ──

    #[test]
    fn aristas_paralelas_coautoria_visibles_como_multigrafo() {
        let s = kb_lira_paso1();

        // El par {Ana, Beto} aparece unido por AUTHORED a través de DOS
        // documentos distintos, con CUATRO EdgeIds distintos: el par de
        // personas soporta tantas relaciones como hechos existan entre él.
        let edges_de = |p: usize| -> Vec<usize> {
            s.out_edges(p)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == "AUTHORED")
                .copied()
                .collect()
        };
        let ana: Vec<usize> = edges_de(ids::ANA);
        let beto: Vec<usize> = edges_de(ids::BETO);
        let docs_ana: Vec<usize> = ana
            .iter()
            .map(|&eid| s.get_edge(eid).unwrap().target)
            .collect();
        let docs_beto: Vec<usize> = beto
            .iter()
            .map(|&eid| s.get_edge(eid).unwrap().target)
            .collect();
        let comunes: Vec<usize> = docs_ana
            .iter()
            .filter(|d| docs_beto.contains(d))
            .copied()
            .collect();
        assert_eq!(comunes, vec![ids::DOC_GRAFOS_AGENTES, ids::DOC_SUPERNODOS]);

        let ids_involucrados: Vec<usize> = ana
            .iter()
            .chain(beto.iter())
            .filter(|&&eid| comunes.contains(&s.get_edge(eid).unwrap().target))
            .copied()
            .collect();
        assert_eq!(ids_involucrados.len(), 4);
        let sin_repetidos: std::collections::HashSet<usize> =
            ids_involucrados.iter().copied().collect();
        assert_eq!(
            sin_repetidos.len(),
            4,
            "EdgeIds distintos: paralelas legibles"
        );

        // Y a nivel de STORE: dos aristas con el MISMO (source,target,label)
        // conviven si tienen EdgeIds distintos — eso ES un multigrafo. Lo
        // rechazado es el ID DUPLICADO (cap. 7/8), no el par repetido.
        let mut m = MemoryStore::new();
        m.put_node(Node::new(0, "Persona")).unwrap();
        m.put_node(Node::new(1, "Documento")).unwrap();
        m.put_edge(Edge::new(100, 0, 1, "AUTHORED")).unwrap();
        m.put_edge(Edge::new(200, 0, 1, "AUTHORED")).unwrap();
        assert_eq!(m.out_edges(0).len(), 2, "paralelas aceptadas");
        assert_eq!(
            m.put_edge(Edge::new(100, 0, 1, "AUTHORED")),
            Err(StoreError::DuplicateEdge(100)),
            "lo único prohibido: repetir IDENTIDAD"
        );
    }

    #[test]
    fn citas_solo_apuntan_en_direccion_de_lectura() {
        let s = kb_lira_paso1();
        let cita_out = |id: usize| -> Vec<usize> {
            s.out_edges(id)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == "CITES")
                .map(|&eid| s.get_edge(eid).unwrap().target)
                .collect()
        };
        let cita_in = |id: usize| -> Vec<usize> {
            s.in_edges(id)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == "CITES")
                .map(|&eid| s.get_edge(eid).unwrap().source)
                .collect()
        };

        // «Supernodos» (17) cita a 13 y 15; le cita 16. Asimetría total.
        let mut out17 = cita_out(ids::DOC_SUPERNODOS);
        out17.sort();
        assert_eq!(
            out17,
            vec![
                ids::DOC_CONSULTAS_DECLARATIVAS,
                ids::DOC_INDICES_ADAPTATIVOS
            ]
        );
        assert_eq!(cita_in(ids::DOC_SUPERNODOS), vec![ids::DOC_RAG]);

        // «Grafos de conocimiento para agentes» (12): citado por 13, 14 y
        // 16 (los tres papers que lo citan en el builder), y NO cita a
        // nadie — la flecha jamás se invierte sola.
        assert!(cita_out(ids::DOC_GRAFOS_AGENTES).is_empty());
        let mut in12 = cita_in(ids::DOC_GRAFOS_AGENTES);
        in12.sort();
        assert_eq!(
            in12,
            vec![
                ids::DOC_CONSULTAS_DECLARATIVAS,
                ids::DOC_MEMORIA_EPISODICA,
                ids::DOC_RAG,
            ]
        );
    }

    // ── CSV determinista y artefacto commiteado ──

    #[test]
    fn csv_roundtrip_import_export_byte_a_byte() {
        let s = kb_lira_paso1();
        let nodos_v1 = csv_nodos_kb_lira(&s);
        let aristas_v1 = csv_aristas_kb_lira(&s);

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

        assert_eq!(s2.node_count(), s.node_count());
        assert_eq!(s2.edge_count(), s.edge_count());

        let nodos_v2 = csv_nodos_kb_lira(&s2);
        let aristas_v2 = csv_aristas_kb_lira(&s2);
        assert_eq!(nodos_v1, nodos_v2, "roundtrip nodos byte a byte");
        assert_eq!(aristas_v1, aristas_v2, "roundtrip aristas byte a byte");

        // Y el grafo re-importado sigue siendo un modelo válido.
        assert_eq!(validar_modelo_kb_lira(&s2), Ok(()));
    }

    #[test]
    fn csv_coincide_con_dataset_commiteado_byte_a_byte() {
        // datasets/kb-lira/paso-1/ ES la salida de los builders: si alguien
        // regenera el builder y olvida commitear, este test grita.
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-1");
        let s = kb_lira_paso1();
        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");
        assert_eq!(csv_nodos_kb_lira(&s), esperado_nodos);
        assert_eq!(csv_aristas_kb_lira(&s), esperado_aristas);
    }

    // ── Informe reproducible ──

    #[test]
    fn informe_modelado_reproducible_sobre_kb_lira() {
        let s = kb_lira_paso1();
        let a = informe_modelado_reproducible(&s);
        let b = informe_modelado_reproducible(&s);
        assert_eq!(a, b, "dos ejecuciones, mismas bytes (sin tiempos)");

        for fragmento in [
            "KB-Lira paso-1 | 30 nodos | 64 aristas",
            "validador: OK",
            "P01 documentos_de_una_persona(Ana)",
            "P02 autores_en_orden",
            "P03 citas_bidireccionales",
            "P04 proyecto_y_afiliaciones(Kira)",
            "P05 temas_e_inversa",
            "P06 menciones(Instituto Neurónica)",
            "coste naive: 12 lecturas vs LPG: 5 lecturas",
            "P07 copublicacion(Ana,Beto)",
            "P08 publicaciones_por_persona",
            "total 16 AUTHORED",
            "P09 temas_comunes_via_papers(Ana,Beto)",
            "P10 citas_recientes",
            "sin cronómetro",
        ] {
            assert!(a.contains(fragmento), "informe sin «{fragmento}»:\n{a}");
        }
    }
}
