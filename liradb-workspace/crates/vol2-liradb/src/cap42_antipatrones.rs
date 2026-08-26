//! Vol.III — Cap.42: Antipatrones: supernodos, reificación y otras trampas.
//!
//! SEGUNDO capítulo del Vol.III («Grafos en la era de la IA»), Parte I «Modelar
//! datos de grafos». El cap. 41 sembró un mini-hub a propósito (tema 24 con 6
//! ABOUT «sin que nadie lo pidiera») y dejó la pregunta abierta: «¿cuándo un
//! hub deja de ser inocente?». Este módulo la cobra: el dataset CRECE con un
//! **lote importado** que degenera el modelo, y el detector de supernodos
//! (siguiente pieza) demostrará con umbrales que el hub ya no es inocente —
//! antes de que «crezca» (objetivo medible del outline).
//!
//! Modelo mental único: **un antipatrón no es una forma fea de modelar: es una
//! deuda que cobra en lecturas cada vez que alguien cruza el nodo**; el refactor
//! es pagar la deuda ANTES de que el interés sea el dataset entero.
//!
//! Qué entrega ESTA pieza (implementación incremental, contrato §2 y §8):
//!
//! 1. **Builder degenerado** [`kb_lira_paso2_degrado`] → `MemoryStore`: PARTE de
//!    [`kb_lira_paso1`] (lo llama y añade encima — NUNCA lo copia) y le añade el
//!    lote importado (presagio del cap. 45: el lote se añade A MANO): 3 personas
//!    nuevas (Gaby/Hugo/Iris, ids 30-32), 24 papers (ids 33-56) con
//!    `conferencia:String` en cada uno, `AUTHORED{order:1}` por paper, 6 CITES
//!    internas, 18 ABOUT concentradas en el tema 24 (el hub pasa de 6 a 24
//!    aristas entrantes — el 46% de TODAS las ABOUT), 3+2+1 ABOUT repartidas en
//!    los temas 26/28/25, 2 Temas degenerados «publicaciones 2024/2025» (ids
//!    57-58) con 6 ABOUT cada uno, y 4 `REVIEWED_BY{nota}` — incluida la pareja
//!    paralela de Fabio sobre el Informe de revisión por pares: dos rondas
//!    INDISTINGUIBLES por significado (el antipatrón que la reificación de
//!    `Resena` pagará en una pieza posterior).
//!
//! 2. **Detector de supernodos** ([`detectar_supernodos`] →
//!    `Vec<SupernodoCandidato>`): UN barrido de `iter_edges` calcula, POR LABEL
//!    de nodo, el grado entrante/saliente/total, la mediana del grado del label
//!    (sobre los demás nodos, sin el outlier) y el share por tipo de arista;
//!    alarma si `ratio_vs_mediana ≥ 5×` Y `share_del_tipo ≥ 25%` (umbral doble
//!    relativo, decisión #2 del contrato). Sobre KB-Lira cobra el gancho del
//!    cap-41 con números: paso-1 SILENCIO (tema 24: 6 ABOUT, mediana 2,0 →
//!    ratio 3,0× < 5×), paso-2 ALARMA (tema 24: 24 ABOUT, mediana 4,0 → ratio
//!    6,0× y share 46%).
//!
//! Total degenerado: **59 nodos, 134 aristas**. Los tres refactors, el
//! validador paso-2 y la regresión de las 10 preguntas llegan en piezas
//! posteriores (contrato §8: orden de implementación recomendado).
//!
//! Frontera declarada (idéntica al cap-41): el lote se añade sin pipeline de
//! ingesta (cap. 45), sin constraints ni índices (cap. 44), sin valid-time para
//! las rondas (cap. 43) — aquí SOLO se siembra la deuda que el capítulo cobrará.

use crate::cap07_modelo::{Edge, Node, NodeId, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap32_import_export::{exportar_csv_aristas, exportar_csv_nodos};
use crate::cap41_modelado::Violacion;
use crate::cap41_modelado::ids;
use crate::cap41_modelado::kb_lira_paso1;
use crate::cap41_modelado::validar_modelo_kb_lira;
use std::collections::HashMap;
use std::fmt;

// ─────────────────── KB-Lira paso-2 (degenerado): el lote importado ───────────────────

/// Ids FIJOS del lote (parte del contrato determinista del cap-42: cualquier
/// test o prosa puede citarlos literalmente). NO reutilizan ids del paso-1
/// (0-29).
///
/// ```text
/// 30-32 Personas       Gaby · Hugo · Iris
/// 33-56 Papers         lote importado (24)
/// 57-58 Temas          publicaciones 2024 · publicaciones 2025 (degenerados)
/// ```
const GABY: usize = 30;
const HUGO: usize = 31;
const IRIS: usize = 32;
const LOTE_INICIO: usize = 33;
const LOTE_FIN: usize = 56;
const TEMA_PUBLICACIONES_2024: usize = 57;
const TEMA_PUBLICACIONES_2025: usize = 58;

/// Nodo con labels arbitrarias (mismo estilo que el cap. 41).
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

/// Paper del lote: labels `Documento`+`Paper`, con `titulo`/`anio` como el
/// paso-1 y la propiedad `conferencia:String` — un string que NADIE puede
/// expandir (la lección P6 del cap-41 repetida a escala: refactor C lo
/// convertirá en nodos `:Conferencia`).
fn paper_lote(id: usize, titulo: &str, anio: i64, conferencia: &str) -> Node {
    let mut n = nodo(id, &["Documento", "Paper"], titulo);
    n.props
        .insert("titulo".into(), Value::String(titulo.into()));
    n.props.insert("anio".into(), Value::Int(anio));
    n.props
        .insert("conferencia".into(), Value::String(conferencia.into()));
    n
}

fn arista(id: usize, source: usize, target: usize, label: &str) -> Edge {
    Edge::new(id, source, target, label)
}

/// Construye KB-Lira **paso-2 degenerado**: PARTE de [`kb_lira_paso1`] (lo
/// llama) y AÑADE el lote importado (ids 30-58), sembrando los antipatrones
/// del capítulo (contrato §2 y decisión #1):
///
/// - 18 `ABOUT` concentradas en el tema 24 («grafos de conocimiento»): el hub
///   pasa de 6 a 24 aristas entrantes. Cada expansión desde ese nodo pagará el
///   grado entero (tesis estructural del capítulo, con contadores en tests
///   posteriores).
/// - `conferencia:String` en cada paper del lote: el string que no se expande.
/// - 2 Temas degenerados «publicaciones 2024/2025» (ids 57-58): nodos
///   categóricos de baja cardinalidad — el año ya es `anio` (R6 del cap-41);
///   refactor C los borrará con sus 12 ABOUT.
/// - 4 `REVIEWED_BY{nota:Int}`: las dos de Fabio sobre el Informe de revisión
///   por pares (ronda 1 = nota 7, ronda 2 = nota 8) son PARALELAS
///   indistinguibles por significado — la reseña sin identidad que exige el
///   nodo `Resena` (refactor B).
/// - `AUTHORED{order:1}` por paper del lote, con autores Gaby/Hugo/Iris
///   (nuevos) y Fabio/Elena (paso-1): la regresión de las 10 preguntas queda
///   limpia porque Ana/Beto/Carla/Dani NO tocan el lote.
/// - 6 `CITES` internas entre papers del lote (pares fijos a mano).
///
/// Aristas nuevas con ids 64-133: 64-87 AUTHORED, 88-93 CITES, 94-117 ABOUT,
/// 118-129 ABOUT a los temas-año, 130-133 REVIEWED_BY.
pub fn kb_lira_paso2_degrado() -> MemoryStore {
    let mut s = kb_lira_paso1();

    // ── Personas del lote (30-32) ──
    for (id, nombre) in [(GABY, "Gaby"), (HUGO, "Hugo"), (IRIS, "Iris")] {
        s.put_node(nodo(id, &["Persona"], nombre)).unwrap();
    }

    // ── Papers del lote (33-56): Documento+Paper con conferencia:String ──
    for (id, titulo, anio, conferencia) in [
        (
            33usize,
            "Grafos de conocimiento en producción",
            2024i64,
            "ICDE 2024",
        ),
        (
            34,
            "Unificación de entidades en knowledge graphs",
            2024,
            "ICDE 2024",
        ),
        (
            35,
            "Razonamiento multihop sobre grafos de conocimiento",
            2024,
            "ICDE 2024",
        ),
        (
            36,
            "Consulta declarativa de knowledge graphs a escala",
            2024,
            "ICDE 2024",
        ),
        (
            37,
            "Control de calidad en grafos de conocimiento",
            2024,
            "ICDE 2024",
        ),
        (
            38,
            "Materialización incremental de reglas en grafos",
            2024,
            "ICDE 2024",
        ),
        (
            39,
            "Anclaje semántico para grafos de conocimiento",
            2024,
            "ICDE 2024",
        ),
        (40, "Versionado de ontologías en grafos", 2024, "ICDE 2024"),
        (
            41,
            "GraphRAG: recuperación aumentada con grafos",
            2024,
            "SIGMOD 2024",
        ),
        (
            42,
            "Reranking de párrafos con caminos de grafos",
            2024,
            "SIGMOD 2024",
        ),
        (
            43,
            "Indexación de vecindarios para recuperación",
            2024,
            "SIGMOD 2024",
        ),
        (
            44,
            "Fusión de grafos y texto para respuestas",
            2024,
            "SIGMOD 2024",
        ),
        (45, "Evaluación de sistemas GraphRAG", 2024, "SIGMOD 2024"),
        (
            46,
            "Memoria gráfica para asistentes conversacionales",
            2024,
            "SIGMOD 2024",
        ),
        (
            47,
            "Particionado de knowledge graphs para recuperación",
            2024,
            "SIGMOD 2024",
        ),
        (
            48,
            "Agregación de evidencias en grafos",
            2024,
            "SIGMOD 2024",
        ),
        (49, "Grafos de conocimiento temporales", 2025, "VLDB 2025"),
        (
            50,
            "Streaming de aristas sobre grafos de conocimiento",
            2025,
            "VLDB 2025",
        ),
        (
            51,
            "Memoria de trabajo en agentes con grafos",
            2025,
            "VLDB 2025",
        ),
        (
            52,
            "Planificación de tareas con memoria gráfica",
            2025,
            "VLDB 2025",
        ),
        (
            53,
            "Consolidación de recuerdos en agentes",
            2025,
            "VLDB 2025",
        ),
        (54, "RAG híbrido: vectorial más grafos", 2025, "VLDB 2025"),
        (55, "Recuperación guiada por ontologías", 2025, "VLDB 2025"),
        (
            56,
            "Lenguajes de consulta para grafos de conocimiento",
            2025,
            "VLDB 2025",
        ),
    ] {
        s.put_node(paper_lote(id, titulo, anio, conferencia))
            .unwrap();
    }

    // ── Temas degenerados (57-58): nodo categórico de baja cardinalidad ──
    for (id, nombre) in [
        (TEMA_PUBLICACIONES_2024, "publicaciones 2024"),
        (TEMA_PUBLICACIONES_2025, "publicaciones 2025"),
    ] {
        s.put_node(nodo(id, &["Tema"], nombre)).unwrap();
    }

    // ── AUTHORED (64-87): 24, uno por paper del lote, order:1 ──
    for (id, autor, paper) in [
        (64usize, GABY, 33),
        (65, GABY, 34),
        (66, GABY, 35),
        (67, GABY, 36),
        (68, GABY, 37),
        (69, GABY, 38),
        (70, HUGO, 39),
        (71, HUGO, 40),
        (72, HUGO, 41),
        (73, HUGO, 42),
        (74, HUGO, 43),
        (75, HUGO, 44),
        (76, IRIS, 45),
        (77, IRIS, 46),
        (78, IRIS, 47),
        (79, IRIS, 48),
        (80, IRIS, 49),
        (81, IRIS, 50),
        (82, ids::FABIO, 51),
        (83, ids::FABIO, 52),
        (84, ids::FABIO, 53),
        (85, ids::ELENA, 54),
        (86, ids::ELENA, 55),
        (87, ids::ELENA, 56),
    ] {
        s.put_edge(arista(id, autor, paper, "AUTHORED").with_prop("order", Value::Int(1)))
            .unwrap();
    }

    // ── CITES (88-93): 6 internas entre papers del lote, pares fijos ──
    for (id, paper, cita_a) in [
        (88usize, 34, 33),
        (89, 36, 33),
        (90, 41, 35),
        (91, 45, 41),
        (92, 51, 46),
        (93, 54, 41),
    ] {
        s.put_edge(arista(id, paper, cita_a, "CITES")).unwrap();
    }

    // ── ABOUT (94-117): 18 al hub (tema 24) + 3 al 26 + 2 al 28 + 1 al 25 ──
    for (id, paper, tema) in [
        (94usize, 33, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (95, 34, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (96, 35, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (97, 36, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (98, 37, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (99, 38, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (100, 39, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (101, 40, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (102, 41, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (103, 42, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (104, 43, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (105, 44, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (106, 45, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (107, 46, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (108, 47, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (109, 48, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (110, 49, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (111, 50, ids::TEMA_GRAFOS_CONOCIMIENTO),
        (112, 51, 26),
        (113, 52, 26),
        (114, 53, 26),
        (115, 54, 28),
        (116, 55, 28),
        (117, 56, 25),
    ] {
        s.put_edge(arista(id, paper, tema, "ABOUT")).unwrap();
    }

    // ── ABOUT a los Temas degenerados (118-129): 6 a cada uno ──
    for (id, paper, tema) in [
        (118usize, 33, TEMA_PUBLICACIONES_2024),
        (119, 34, TEMA_PUBLICACIONES_2024),
        (120, 35, TEMA_PUBLICACIONES_2024),
        (121, 36, TEMA_PUBLICACIONES_2024),
        (122, 41, TEMA_PUBLICACIONES_2024),
        (123, 42, TEMA_PUBLICACIONES_2024),
        (124, 49, TEMA_PUBLICACIONES_2025),
        (125, 50, TEMA_PUBLICACIONES_2025),
        (126, 51, TEMA_PUBLICACIONES_2025),
        (127, 52, TEMA_PUBLICACIONES_2025),
        (128, 53, TEMA_PUBLICACIONES_2025),
        (129, 54, TEMA_PUBLICACIONES_2025),
    ] {
        s.put_edge(arista(id, paper, tema, "ABOUT")).unwrap();
    }

    // ── REVIEWED_BY (130-133): la reseña SIN identidad (antipatrón a pagar) ──
    // Las dos de Fabio sobre el Informe de revisión por pares (ronda 1 = nota
    // 7, ronda 2 = nota 8) son paralelas: solo la nota las distingue — el
    // significado de «ronda» no vive en el grafo. Carla y Gaby reseñan papers
    // del lote.
    for (id, persona, documento, nota) in [
        (130usize, ids::FABIO, ids::DOC_REVISION_PARES, 7i64),
        (131, ids::FABIO, ids::DOC_REVISION_PARES, 8),
        (132, ids::CARLA, 36, 6),
        (133, GABY, 45, 9),
    ] {
        s.put_edge(
            arista(id, persona, documento, "REVIEWED_BY").with_prop("nota", Value::Int(nota)),
        )
        .unwrap();
    }

    s
}

// ─────────────────── CSV determinista (mismo contrato que el cap. 41) ───────────────────

/// Exporta los NODOS del paso-2 degenerado al formato CSV del cap. 32
/// (cabecera = unión BTreeMap de props; `:LABEL` con labels unidas por `:`).
/// Es el MISMO formato que [`crate::cap41_modelado::csv_nodos_kb_lira`]: el
/// dataset es lo que «importó el equipo», el refactor es código.
pub fn csv_nodos_kb_lira_paso2(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_nodos(store, &mut buf).expect("export nodos paso-2");
    String::from_utf8(buf).expect("CSV UTF-8")
}

/// Exporta las ARISTAS del paso-2 degenerado (mismo contrato que
/// [`csv_nodos_kb_lira_paso2`]).
pub fn csv_aristas_kb_lira_paso2(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_aristas(store, &mut buf).expect("export aristas paso-2");
    String::from_utf8(buf).expect("CSV UTF-8")
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_antipatrones {
    use super::*;
    use crate::cap08_graph_store::GraphStore;

    #[test]
    fn estructura_de_kb_lira_paso2_degrado_cuenta_y_etiquetas_exactas() {
        let s = kb_lira_paso2_degrado();

        // 59 nodos = 30 del paso-1 + 29 del lote (3 personas + 24 papers + 2 temas).
        assert_eq!(s.node_count(), 59);
        // 134 aristas = 64 del paso-1 + 70 del lote (24 AUTHORED + 6 CITES +
        // 24 ABOUT + 12 ABOUT temas-año + 4 REVIEWED_BY).
        assert_eq!(s.edge_count(), 134);

        let cuenta_label = |label: &str| s.iter_nodes().filter(|n| n.has_label(label)).count();
        assert_eq!(cuenta_label("Persona"), 9); // 6 paso-1 + 3 lote
        assert_eq!(cuenta_label("Organizacion"), 3);
        assert_eq!(cuenta_label("Proyecto"), 3);
        assert_eq!(cuenta_label("Documento"), 36); // 12 paso-1 + 24 lote
        assert_eq!(cuenta_label("Paper"), 30); // 6 paso-1 + 24 lote
        assert_eq!(cuenta_label("Nota"), 3);
        assert_eq!(cuenta_label("Informe"), 3);
        assert_eq!(cuenta_label("Tema"), 8); // 6 paso-1 + 2 degenerados

        let cuenta_tipo = |tipo: &str| s.iter_edges().filter(|e| e.label == tipo).count();
        assert_eq!(cuenta_tipo("AUTHORED"), 40); // 16 paso-1 + 24 lote
        assert_eq!(cuenta_tipo("CITES"), 16); // 10 paso-1 + 6 lote
        assert_eq!(cuenta_tipo("ABOUT"), 52); // 16 paso-1 + 24 lote + 12 temas-año
        assert_eq!(cuenta_tipo("MENTIONS"), 10);
        assert_eq!(cuenta_tipo("MEMBER_OF"), 6);
        assert_eq!(cuenta_tipo("WORKED_ON"), 6);
        assert_eq!(cuenta_tipo("REVIEWED_BY"), 4);

        // El hub dejó de ser inocente: el tema 24 acumula EXACTAMENTE 24 ABOUT
        // entrantes (6 del paso-1 + 18 del lote).
        let hub = s
            .in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO)
            .iter()
            .filter(|&&eid| s.get_edge(eid).unwrap().label == "ABOUT")
            .count();
        assert_eq!(hub, 24, "tema 24: 6 ABOUT del paso-1 + 18 del lote");

        // Los Temas degenerados acumulan 6 ABOUT cada uno (12 en total).
        let about_de = |tema: usize| {
            s.in_edges(tema)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == "ABOUT")
                .count()
        };
        assert_eq!(about_de(TEMA_PUBLICACIONES_2024), 6);
        assert_eq!(about_de(TEMA_PUBLICACIONES_2025), 6);

        // Todo paper del lote es Documento+Paper y lleva conferencia:String
        // (el string que nadie expande — semilla del refactor C).
        for n in s
            .iter_nodes()
            .filter(|n| (LOTE_INICIO..=LOTE_FIN).contains(&n.id))
        {
            assert!(
                n.has_label("Documento") && n.has_label("Paper"),
                "paper {} sin labels Documento+Paper",
                n.id
            );
            assert!(
                matches!(n.props.get("conferencia"), Some(Value::String(c)) if !c.is_empty()),
                "paper {} sin conferencia:String",
                n.id
            );
        }

        // Las 4 REVIEWED_BY llevan nota:Int; las 24 AUTHORED del lote, order:1.
        for e in s.iter_edges().filter(|e| e.label == "REVIEWED_BY") {
            assert!(
                matches!(e.props.get("nota"), Some(Value::Int(_))),
                "REVIEWED_BY {} sin nota:Int",
                e.id
            );
        }
        for e in s.iter_edges().filter(|e| (64..=87).contains(&e.id)) {
            assert_eq!(
                e.props.get("order"),
                Some(&Value::Int(1)),
                "AUTHORED {} sin order:1",
                e.id
            );
        }
    }

    #[test]
    fn distribucion_de_grados_exacta_sobre_kb_lira() {
        // Conteo a mano sobre el API del cap. 8: grados entrante/saliente POR
        // TIPO de arista, verificados contra los builders deterministas del
        // paso-1 y del paso-2 (los mismos conteos que alimenta el detector).
        let grados = |s: &MemoryStore, nodo: usize, tipo: &str| -> (usize, usize) {
            let entrante = s
                .in_edges(nodo)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == tipo)
                .count();
            let saliente = s
                .out_edges(nodo)
                .iter()
                .filter(|&&eid| s.get_edge(eid).unwrap().label == tipo)
                .count();
            (entrante, saliente)
        };

        // ── paso-1 ──
        let p1 = kb_lira_paso1();
        // Tema 24: 6 ABOUT entrantes (el hub sembrado del cap-41), 0 salientes.
        assert_eq!(grados(&p1, ids::TEMA_GRAFOS_CONOCIMIENTO, "ABOUT"), (6, 0));
        // Grado entrante TOTAL del tema 24 (en un Tema solo inciden ABOUT) = 6.
        assert_eq!(p1.in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO).len(), 6);
        // Tema 26: 3 ABOUT entrantes.
        assert_eq!(grados(&p1, 26, "ABOUT"), (3, 0));
        // Ana (0) firma 4 AUTHORED salientes en el paso-1.
        assert_eq!(grados(&p1, ids::ANA, "AUTHORED"), (0, 4));

        // ── paso-2 ──
        let p2 = kb_lira_paso2_degrado();
        // El lote concentra 18 ABOUT más en el tema 24: 6 + 18 = 24 entrantes.
        assert_eq!(grados(&p2, ids::TEMA_GRAFOS_CONOCIMIENTO, "ABOUT"), (24, 0));
        assert_eq!(p2.in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO).len(), 24);
        // Tema 26: 3 del paso-1 + 3 del lote = 6.
        assert_eq!(grados(&p2, 26, "ABOUT"), (6, 0));
        // Gaby (30) firma los 6 AUTHORED del lote que le tocan.
        assert_eq!(grados(&p2, GABY, "AUTHORED"), (0, 6));
        // Temas degenerados (57/58): 6 ABOUT entrantes cada uno.
        assert_eq!(grados(&p2, TEMA_PUBLICACIONES_2024, "ABOUT"), (6, 0));
        assert_eq!(grados(&p2, TEMA_PUBLICACIONES_2025, "ABOUT"), (6, 0));
    }

    #[test]
    fn el_hub_del_paso1_es_inocente_segun_el_detector() {
        let s = kb_lira_paso1();
        let candidatos = detectar_supernodos(&s);
        assert!(
            candidatos.is_empty(),
            "paso-1: tema 24 con 6 ABOUT → mediana 2,0 → ratio 3,0× < 5×: el \
             hub es inocente por NO cruzar el umbral (obtenido: {candidatos:?})"
        );
    }

    #[test]
    fn el_hub_del_paso2_degrado_es_candidato_a_supernodo() {
        let s = kb_lira_paso2_degrado();
        let candidatos = detectar_supernodos(&s);
        let hub = candidatos
            .iter()
            .find(|c| c.nodo_id == ids::TEMA_GRAFOS_CONOCIMIENTO && c.label == "Tema")
            .expect("el tema 24 debe ser candidato a supernodo");

        // Conteos exactos: 24 ABOUT entrantes (6 del paso-1 + 18 del lote).
        assert_eq!(hub.grado_entrante, 24);
        assert_eq!(hub.grado_saliente, 0);
        assert_eq!(hub.grado_total, 24);
        // Mediana del label Tema SIN el hub: [1,2,3,4,6,6,6] → 4,0.
        assert!(
            (hub.mediana_label - 4.0).abs() < 1e-9,
            "mediana real: {}",
            hub.mediana_label
        );
        // Ratio 24 / 4 = 6,0× ≥ 5× (cruza el umbral de alarma).
        assert!(
            (hub.ratio_vs_mediana - 6.0).abs() < 1e-9,
            "ratio real: {}",
            hub.ratio_vs_mediana
        );
        // Share: 24 de las 52 ABOUT totales = 24/52 = 0,4615… ≥ 25%.
        let share_esperado = 24.0 / 52.0;
        assert!(
            (hub.share_del_tipo - share_esperado).abs() < 1e-9,
            "share real: {} (esperado {share_esperado})",
            hub.share_del_tipo
        );
        // El lote no creó otros candidatos: el hub es el ÚNICO.
        assert_eq!(
            candidatos.len(),
            1,
            "único candidato: el tema 24 (obtenido: {candidatos:?})"
        );
    }
}

// ─────────────────── Detector de supernodos (contrato §2, decisión #2) ───────────────────

/// Umbral del ratio: un nodo solo alarma si su grado total es ≥ 5× la mediana
/// del grado de SU label. Es RELATIVO a la distribución, no un absoluto
/// («way more relationships than other nodes, relative to what else is in the
/// graph», Allen 2020): el absoluto tipo «10M de aristas» es el titular; el
/// criterio reproducible es el desequilibrio de la distribución.
pub const RATIO_MINIMO_VS_MEDIANA: f64 = 5.0;

/// Umbral del share: el tipo de arista que concentra el candidato debe acumular
/// ≥ 25% de TODAS las aristas de ese tipo. La AND de ambos umbrales es la
/// defensa contra las falsas alarmas en grafos pequeños (mediana 1 → con el
/// ratio solo, cualquier nodo de grado 5 alarmaría).
pub const SHARE_MINIMO_POR_TIPO: f64 = 0.25;

/// Un candidato a supernodo: nodo cuyo grado total supera [`RATIO_MINIMO_VS_MEDIANA`]
/// veces la mediana del grado de su label Y cuyo tipo de arista dominante
/// concentra ≥ [`SHARE_MINIMO_POR_TIPO`] de las aristas de ese tipo.
#[derive(Debug, Clone, PartialEq)]
pub struct SupernodoCandidato {
    /// Id del nodo (`NodeId = usize`, cap. 7 — mismo estilo que el cap-41).
    pub nodo_id: NodeId,
    /// Label por la que el nodo es candidato (un nodo con labels múltiples
    /// participa en la distribución de CADA una de sus labels).
    pub label: String,
    /// Grado entrante TOTAL del nodo (todos los tipos de arista).
    pub grado_entrante: usize,
    /// Grado saliente TOTAL del nodo (todos los tipos de arista).
    pub grado_saliente: usize,
    /// `grado_entrante + grado_saliente`.
    pub grado_total: usize,
    /// Mediana del grado total de los DEMÁS nodos del label: la línea base no
    /// puede fijarla el propio outlier (si el hub participara en su mediana, el
    /// detector se desinflaría a sí mismo a medida que el hub crece).
    pub mediana_label: f64,
    /// `grado_total / mediana_label` (p.ej. 6,0× en el paso-2).
    pub ratio_vs_mediana: f64,
    /// Fracción de las aristas del tipo dominante del nodo que son incidentes a
    /// él (p.ej. tema 24 en el paso-2: 24 de las 52 ABOUT = 0,46).
    pub share_del_tipo: f64,
}

/// Mediana de `valores` (lista par: media de los dos centrales).
fn mediana_de(valores: &[usize]) -> f64 {
    let mut v = valores.to_vec();
    v.sort_unstable();
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2] as f64,
        n => (v[n / 2 - 1] + v[n / 2]) as f64 / 2.0,
    }
}

/// Fracción de las aristas del tipo DOMINANTE de `hub` que son incidentes a él:
/// `incidentes_del_tipo / totales_del_tipo` (p.ej. tema 24 en el paso-2: 24
/// ABOUT incidentes de 52 ABOUT totales → 0,46).
fn share_del_tipo_dominante(
    hub: NodeId,
    incidentes: &HashMap<(NodeId, String), usize>,
    totales_por_tipo: &HashMap<String, usize>,
) -> f64 {
    let mut dominante: Option<(String, usize, usize)> = None;
    for ((nodo, tipo), &cuantas) in incidentes {
        if *nodo != hub {
            continue;
        }
        let Some(&total) = totales_por_tipo.get(tipo) else {
            continue;
        };
        if total == 0 {
            continue;
        }
        match &dominante {
            Some((_, c, _)) if *c >= cuantas => {}
            _ => dominante = Some((tipo.clone(), cuantas, total)),
        }
    }
    match dominante {
        Some((_, cuantas, total)) => cuantas as f64 / total as f64,
        None => 0.0,
    }
}

/// Detecta supernodos con UN barrido de `iter_edges` (contrato §2, decisión #2):
/// distribución de grados POR LABEL de nodo y dos umbrales FIJOS y declarados.
///
/// - `ratio_vs_mediana ≥ RATIO_MINIMO_VS_MEDIANA` (5×): grado total del nodo
///   contra la mediana del grado de los DEMÁS nodos de su label — la línea base
///   no puede fijarla el propio outlier.
/// - `share_del_tipo ≥ SHARE_MINIMO_POR_TIPO` (25%): el tipo de arista
///   dominante del nodo acumula al menos 1 de cada 4 aristas de ese tipo.
///
/// La AND de ambos es la defensa contra las falsas alarmas (decisión #2).
///
/// Sobre KB-Lira el veredicto es el triplete del capítulo:
///
/// - paso-1: tema 24 con 6 ABOUT → mediana 2,0 → ratio 3,0× < 5× → SILENCIO
///   (el gancho del cap-41: el hub era inocente por NO cruzar el umbral).
/// - paso-2: tema 24 con 24 ABOUT → mediana 4,0 → ratio 6,0× y share 46% →
///   ALARMA (el lote importado cruzó el umbral antes de que «crezca»).
pub fn detectar_supernodos(store: &dyn GraphStore) -> Vec<SupernodoCandidato> {
    // Labels por nodo: un nodo con labels múltiples participa en TODAS las
    // distribuciones (el `label` del candidato es la que dispara la alarma).
    let mut labels_por_nodo: HashMap<NodeId, Vec<String>> = HashMap::new();
    for n in store.iter_nodes() {
        labels_por_nodo.insert(n.id, n.labels.clone());
    }

    // UN solo barrido de aristas: grados in/out por nodo, total por tipo de
    // arista y aristas incidentes por (nodo, tipo).
    let mut entrantes: HashMap<NodeId, usize> = HashMap::new();
    let mut salientes: HashMap<NodeId, usize> = HashMap::new();
    let mut totales_por_tipo: HashMap<String, usize> = HashMap::new();
    let mut incidentes_por_nodo_tipo: HashMap<(NodeId, String), usize> = HashMap::new();

    for e in store.iter_edges() {
        *salientes.entry(e.source).or_insert(0) += 1;
        *entrantes.entry(e.target).or_insert(0) += 1;
        *totales_por_tipo.entry(e.label.clone()).or_insert(0) += 1;
        *incidentes_por_nodo_tipo
            .entry((e.source, e.label.clone()))
            .or_insert(0) += 1;
        *incidentes_por_nodo_tipo
            .entry((e.target, e.label.clone()))
            .or_insert(0) += 1;
    }

    // Distribución de grado_total POR LABEL.
    let mut grados_por_label: HashMap<String, Vec<(NodeId, usize)>> = HashMap::new();
    for (&nodo, labels) in &labels_por_nodo {
        let grado =
            entrantes.get(&nodo).copied().unwrap_or(0) + salientes.get(&nodo).copied().unwrap_or(0);
        for label in labels {
            grados_por_label
                .entry(label.clone())
                .or_default()
                .push((nodo, grado));
        }
    }

    let mut candidatos: Vec<SupernodoCandidato> = Vec::new();
    for (label, grados) in grados_por_label {
        // El candidato de cada label: el nodo con MAYOR grado total (empates →
        // menor id: determinismo total, disciplina del cap. 34).
        let mut por_grado: Vec<(NodeId, usize)> = grados.to_vec();
        por_grado.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let (hub, grado_max) = por_grado[0];
        if grado_max == 0 {
            continue;
        }
        // Mediana sobre los DEMÁS nodos del label: la línea base sin el outlier.
        let otros: Vec<usize> = grados
            .iter()
            .filter(|(n, _)| *n != hub)
            .map(|(_, g)| *g)
            .collect();
        if otros.is_empty() {
            continue;
        }
        let mediana = mediana_de(&otros);
        if mediana <= 0.0 {
            continue;
        }
        let ratio = grado_max as f64 / mediana;
        let share = share_del_tipo_dominante(hub, &incidentes_por_nodo_tipo, &totales_por_tipo);
        if ratio >= RATIO_MINIMO_VS_MEDIANA && share >= SHARE_MINIMO_POR_TIPO {
            candidatos.push(SupernodoCandidato {
                nodo_id: hub,
                label,
                grado_entrante: entrantes.get(&hub).copied().unwrap_or(0),
                grado_saliente: salientes.get(&hub).copied().unwrap_or(0),
                grado_total: grado_max,
                mediana_label: mediana,
                ratio_vs_mediana: ratio,
                share_del_tipo: share,
            });
        }
    }
    candidatos.sort_by(|a, b| a.label.cmp(&b.label).then(a.nodo_id.cmp(&b.nodo_id)));
    candidatos
}

// ─────────────────── Refactor A: descomponer el supernodo (contrato §8) ───────────────────

/// Informe de lo que un refactor hizo sobre el grafo: contadores de nodos y
/// aristas creados/borrados, y las LECTURAS que costó — cada llamada de
/// lectura al store (`in_edges`, `get_edge`, …) incrementa el contador; las
/// escrituras (`put_*`, `delete_*`) NO.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformeRefactor {
    pub nodos_creados: usize,
    pub nodos_borrados: usize,
    pub aristas_creadas: usize,
    pub aristas_borradas: usize,
    pub lecturas: usize,
}

impl InformeRefactor {
    /// Informe vacío: todo a cero.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Ids de los subtemas del refactor A: continúan la numeración del lote (el
/// paso-2 termina en el nodo 58). Mismo shape que los Temas del cap-41:
/// label `Tema` + `nombre:String`.
const SUBTEMA_KNOWLEDGE_GRAPHS: usize = 59;
const SUBTEMA_GRAPHRAG: usize = 60;
const SUBTEMA_MEMORIA_AGENTES: usize = 61;

/// Primer id de arista libre tras el paso-2 (que termina en 133). El refactor
/// asigna bloques contiguos como el builder: 134-136 `SUB_TEMA_DE`, 137-148
/// las 12 ABOUT redistribuidas.
const PRIMER_ID_ARISTA_REFACTOR_A: usize = 134;

/// Destino del paper del lote en el refactor A: los 12 papers que BAJAN del
/// tema 24 al subtema que mejor les describe (4 por subtema). Los OTROS 6 del
/// lote (37-40, 45, 50) devuelven `None`: SE QUEDAN en el tema 24 — el padre
/// conserva identidad propia, no se vacía.
fn subtema_destino_del_lote(paper: usize) -> Option<usize> {
    match paper {
        33..=36 => Some(SUBTEMA_KNOWLEDGE_GRAPHS),
        41..=44 => Some(SUBTEMA_GRAPHRAG),
        46..=49 => Some(SUBTEMA_MEMORIA_AGENTES),
        _ => None,
    }
}

/// Refactor A: **descomponer el supernodo** (contrato §8) — la deuda del
/// paso-2 se paga ANTES de que el interés sea el dataset entero. Sobre el
/// grafo degenerado:
///
/// 1. Crea 3 Temas subtemas (ids 59-61): «knowledge graphs», «GraphRAG» y
///    «memoria de agentes».
/// 2. Redistribuye 12 de las 18 ABOUT del lote que llegaban al tema 24
///    (4 a cada subtema): borra las 12 ABOUT originales y crea las nuevas
///    DESDE LOS MISMOS documentos (ids 137-148). Las otras 6 del lote se
///    quedan en el tema 24.
/// 3. Crea 3 `SUB_TEMA_DE` (Tema→Tema, ids 134-136): 59→24, 60→24, 61→24 —
///    la arista que convierte la lectura P5 del padre en una UNIÓN
///    (directas ∪ subtemas) sin perder ni duplicar documentos.
///
/// El informe devuelto: 3 nodos y 15 aristas creadas, 12 aristas borradas, 0
/// nodos borrados, y 25 lecturas — UN barrido: 1 `in_edges` sobre el tema 24
/// + 1 `get_edge` por cada una de sus 24 aristas entrantes.
pub fn refactor_a_descomponer_supernodo(store: &mut MemoryStore) -> InformeRefactor {
    let mut informe = InformeRefactor::new();
    let mut lecturas: usize = 0;

    // ── 1) Nodos Tema subtemas (59-61) ──
    for (id, nombre) in [
        (SUBTEMA_KNOWLEDGE_GRAPHS, "knowledge graphs"),
        (SUBTEMA_GRAPHRAG, "GraphRAG"),
        (SUBTEMA_MEMORIA_AGENTES, "memoria de agentes"),
    ] {
        store.put_node(nodo(id, &["Tema"], nombre)).unwrap();
        informe.nodos_creados += 1;
    }

    // ── 2) Redistribución: UN barrido sobre las entrantes al tema 24 ──
    // En el paso-2 son EXACTAMENTE 24 (6 del paso-1 + 18 del lote). Las 12 del
    // lote con destino subtema se borran y se re-crean desde el MISMO
    // documento; las 6 del lote y las 6 del paso-1 pasan de largo.
    let entrantes = store.in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO);
    lecturas += 1;
    let mut proximo_id = PRIMER_ID_ARISTA_REFACTOR_A + 3; // 137
    for eid in entrantes {
        let e = store.get_edge(eid).expect("arista entrante presente");
        lecturas += 1;
        let (source, es_about) = (e.source, e.label == "ABOUT");
        if !es_about {
            continue;
        }
        let Some(sub) = subtema_destino_del_lote(source) else {
            continue;
        };
        assert!(store.delete_edge(eid), "la ABOUT {eid} debe existir");
        informe.aristas_borradas += 1;
        store
            .put_edge(arista(proximo_id, source, sub, "ABOUT"))
            .unwrap();
        proximo_id += 1;
        informe.aristas_creadas += 1;
    }

    // ── 3) SUB_TEMA_DE (134-136): el padre se vuelve la unión de subtemas ──
    for (id, sub) in [
        (PRIMER_ID_ARISTA_REFACTOR_A, SUBTEMA_KNOWLEDGE_GRAPHS),
        (PRIMER_ID_ARISTA_REFACTOR_A + 1, SUBTEMA_GRAPHRAG),
        (PRIMER_ID_ARISTA_REFACTOR_A + 2, SUBTEMA_MEMORIA_AGENTES),
    ] {
        store
            .put_edge(arista(
                id,
                sub,
                ids::TEMA_GRAFOS_CONOCIMIENTO,
                "SUB_TEMA_DE",
            ))
            .unwrap();
        informe.aristas_creadas += 1;
    }

    informe.lecturas = lecturas;
    informe
}

/// Los documentos con `ABOUT` DIRECTA al tema MÁS los documentos con `ABOUT`
/// a CUALQUIER subtema (una profundidad): la lectura P5 del tema padre tras
/// el refactor A — la unión que conserva el conjunto exacto de documentos,
/// aunque el nodo se haya descompuesto.
pub fn documentos_del_tema_incluyendo_subtemas(
    store: &dyn GraphStore,
    tema_id: usize,
) -> Vec<usize> {
    let mut documentos: Vec<usize> = Vec::new();
    let mut subtemas: Vec<usize> = Vec::new();

    // Un barrido sobre las entrantes al tema: las ABOUT son documentos
    // directos; las SUB_TEMA_DE revelan los subtemas (una profundidad).
    for eid in store.in_edges(tema_id) {
        let Some(e) = store.get_edge(eid) else {
            continue;
        };
        match e.label.as_str() {
            "ABOUT" => documentos.push(e.source),
            "SUB_TEMA_DE" => subtemas.push(e.source),
            _ => {}
        }
    }
    for sub in subtemas {
        for eid in store.in_edges(sub) {
            let Some(e) = store.get_edge(eid) else {
                continue;
            };
            if e.label == "ABOUT" {
                documentos.push(e.source);
            }
        }
    }

    // Unión: ordenado y sin duplicados (un documento puede tener ABOUT directa
    // y a un subtema a la vez; la unión lo cuenta una sola vez).
    documentos.sort_unstable();
    documentos.dedup();
    documentos
}

#[cfg(test)]
mod tests_refactor_a {
    use super::*;

    /// P5 ANTES del refactor: 24 docs con ABOUT directa al tema 24 (6 del
    /// paso-1 + 18 del lote). P5 DESPUÉS: `documentos_del_tema_incluyendo_subtemas`
    /// devuelve el MISMO conjunto ordenado — los 12 movidos se encuentran vía
    /// subtemas; nada se pierde ni se duplica.
    #[test]
    fn refactor_descomponer_conserva_el_conjunto_de_p5() {
        let mut s = kb_lira_paso2_degrado();
        let antes = documentos_del_tema_incluyendo_subtemas(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(antes.len(), 24, "hub paso-2: 6 del paso-1 + 18 del lote");

        let _informe = refactor_a_descomponer_supernodo(&mut s);
        let despues = documentos_del_tema_incluyendo_subtemas(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);

        assert_eq!(
            despues, antes,
            "la unión tras descomponer conserva el conjunto de P5 (obtenido: {despues:?})"
        );
    }

    /// Tras el refactor, la semántica de P5 sobre el PADRE se conserva POR
    /// UNIÓN: 12 ABOUT directas al 24 (6 del paso-1 + 6 del lote que se
    /// quedan) y la unión completa sigue siendo 24.
    #[test]
    fn la_semantica_de_p5_sobre_el_tema_padre_se_preserva_por_union() {
        let mut s = kb_lira_paso2_degrado();
        let informe = refactor_a_descomponer_supernodo(&mut s);

        let directas: Vec<usize> = s
            .in_edges(ids::TEMA_GRAFOS_CONOCIMIENTO)
            .iter()
            .filter(|&&eid| s.get_edge(eid).unwrap().label == "ABOUT")
            .map(|&eid| s.get_edge(eid).unwrap().source)
            .collect();
        assert_eq!(
            directas.len(),
            12,
            "6 del paso-1 + 6 del lote que se quedan"
        );

        let union = documentos_del_tema_incluyendo_subtemas(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(union.len(), 24, "directas ∪ subtemas = el mismo P5");

        // El informe refleja exactamente la redistribución.
        assert_eq!(informe.nodos_creados, 3);
        assert_eq!(informe.nodos_borrados, 0);
        assert_eq!(informe.aristas_creadas, 15); // 3 SUB_TEMA_DE + 12 ABOUT
        assert_eq!(informe.aristas_borradas, 12);
        assert_eq!(informe.lecturas, 25); // 1 in_edges + 1 get_edge por arista entrante
    }

    /// El refactor paga la deuda: el tema 24 baja de 24 a 12 ABOUT directas y
    /// el detector deja de alarmar. Distribución REAL del label Tema tras el
    /// refactor (11 temas, incluyendo los 3 subtemas; grado TOTAL, como lo ve
    /// el detector, contando también las 3 SUB_TEMA_DE entrantes al 24):
    /// grados [1,2,3,4,5,5,5,6,6,6,15] → hub 24 (15) → mediana de los demás
    /// 5,0 → ratio 15/5 = 3,0× < 5× (el contrato espera 3×) y share 12/52 =
    /// 0,23 < 25%: el detector calla por AMBOS umbrales.
    #[test]
    fn el_detector_ya_no_alarma_tras_el_refactor() {
        let mut s = kb_lira_paso2_degrado();
        let _informe = refactor_a_descomponer_supernodo(&mut s);

        // Distribución de grados real del label Tema, verificada a mano.
        let mut grados: Vec<usize> = s
            .iter_nodes()
            .filter(|n| n.has_label("Tema"))
            .map(|n| s.in_edges(n.id).len() + s.out_edges(n.id).len())
            .collect();
        grados.sort_unstable();
        let hub = *grados.last().expect("hay Temas");
        assert_eq!(
            hub, 15,
            "tema 24: 12 ABOUT directas + 3 SUB_TEMA_DE entrantes"
        );
        let otros: Vec<usize> = grados.into_iter().filter(|&g| g != hub).collect();
        let mediana = mediana_de(&otros);
        assert!(
            (mediana - 5.0).abs() < 1e-9,
            "mediana real del label Tema: {mediana} (esperada 5,0)"
        );
        let ratio = hub as f64 / mediana;
        assert!(
            (ratio - 3.0).abs() < 1e-9,
            "ratio real: {ratio} (esperado 3,0×)"
        );

        let candidatos = detectar_supernodos(&s);
        assert!(
            candidatos.is_empty(),
            "tema 24 con ratio 3,0× < 5× y share 0,23 < 25% → el detector calla \
             (obtenido: {candidatos:?})"
        );
    }
}

// ─────────────────── Refactor B: reificar la reseña (contrato §8) ───────────────────

/// Ids de los nodos `:Resena` del refactor B (65-68): continúan la numeración
/// de nodos (el paso-2 termina en 58 y el refactor A en 61). El contrato los
/// fija para que la prosa pueda citarlos literalmente.
const RESENA_FABIO_RONDA1: usize = 65;
const RESENA_FABIO_RONDA2: usize = 66;
const RESENA_CARLA: usize = 67;
const RESENA_GABY: usize = 68;

/// Primer id de arista libre tras el refactor A (que termina en 148). El
/// refactor B asigna bloques contiguos como el builder: 149-152 `REALIZA`,
/// 153-156 `SOBRE` y 157 `CONTRARRESTA`.
const PRIMER_ID_ARISTA_REFACTOR_B: usize = 149;

/// Nodo `:Resena` del refactor B: label `Resena` + `nota:Int` + `ronda:Int` —
/// las dos propiedades que las 4 `REVIEWED_BY` paralelas no podían distinguir
/// por significado.
fn resena(id: usize, nota: i64, ronda: i64) -> Node {
    let mut n = Node {
        id,
        labels: vec!["Resena".to_string()],
        props: std::collections::HashMap::new(),
    };
    n.props.insert("nota".into(), Value::Int(nota));
    n.props.insert("ronda".into(), Value::Int(ronda));
    n
}

/// Refactor B: **reificar la reseña** (contrato §8) — la deuda de las 4
/// `REVIEWED_BY` paralelas se paga dando identidad a la reseña. Sobre el grafo
/// degenerado:
///
/// 1. Identifica las 4 `REVIEWED_BY` (ids 130-133) en UN barrido de
///    `iter_edges`: Fabio→Informe (rondas 1 y 2, notas 7 y 8), Carla→paper del
///    lote (nota 6) y Gaby→paper del lote (nota 9).
/// 2. BORRA las 4 aristas: el significado deja de vivir en una propiedad que
///    nadie puede expandir.
/// 3. CREA 4 nodos `:Resena` (ids 65-68) con `nota:Int` y `ronda:Int` — las
///    dos propiedades que distinguen las rondas de Fabio: la reseña deja de
///    ser un dato colgado de una arista para ser un ciudadano del grafo.
/// 4. CREA las aristas que re-sitúan el significado:
///    - 4 `REALIZA` (Persona→Resena, ids 149-152): quién reseñó.
///    - 4 `SOBRE` (Resena→Documento, ids 153-156): qué documento se reseñó.
///    - 1 `CONTRARRESTA` (Resena→Resena, id 157): la ronda 2 de Fabio
///      contrarresta a la ronda 1 — una relación ENTRE reseñas que la arista
///      plana ni siquiera podía nombrar.
///
/// El informe devuelto: 4 nodos y 9 aristas creadas, 4 aristas borradas, 0
/// nodos borrados, y 1 lectura — UN barrido de `iter_edges`.
pub fn refactor_b_reificar_resena(store: &mut MemoryStore) -> InformeRefactor {
    let mut informe = InformeRefactor::new();
    let mut lecturas: usize = 0;

    // ── 1) Identificar las 4 REVIEWED_BY (130-133) en UN barrido ──
    // El par de Fabio es INDISTINGUIBLE por significado en la arista: solo la
    // nota las separa. La ronda se deriva de (persona, nota) — el mismo
    // emparejamiento que sembró el builder (ronda 1 = nota 7, ronda 2 = nota 8).
    let revisadas: Vec<(usize, usize, usize, i64)> = store
        .iter_edges()
        .filter(|e| e.label == "REVIEWED_BY")
        .map(|e| {
            let nota = match e.props.get("nota") {
                Some(Value::Int(n)) => *n,
                _ => panic!("REVIEWED_BY {} sin nota:Int", e.id),
            };
            (e.id, e.source, e.target, nota)
        })
        .collect();
    lecturas += 1;

    // ── 2) BORRAR las 4 aristas: la reseña deja de ser una propiedad ──
    for (eid, _, _, _) in &revisadas {
        assert!(store.delete_edge(*eid), "la REVIEWED_BY {eid} debe existir");
        informe.aristas_borradas += 1;
    }

    // ── 3) CREAR los 4 nodos :Resena (65-68) con nota:Int y ronda:Int ──
    let resenas: Vec<(usize, usize, usize, i64)> = revisadas
        .iter()
        .map(|&(eid, persona, documento, nota)| {
            let (id_resena, ronda) = match (persona, nota) {
                (ids::FABIO, 7) => (RESENA_FABIO_RONDA1, 1),
                (ids::FABIO, 8) => (RESENA_FABIO_RONDA2, 2),
                (ids::CARLA, 6) => (RESENA_CARLA, 1),
                (GABY, 9) => (RESENA_GABY, 1),
                (p, n) => panic!("REVIEWED_BY {eid} inesperada: persona {p}, nota {n}"),
            };
            store.put_node(resena(id_resena, nota, ronda)).unwrap();
            informe.nodos_creados += 1;
            (id_resena, persona, documento, ronda)
        })
        .collect();

    // ── 4) REALIZA (149-152) y SOBRE (153-156): quién y qué se reseñó ──
    for (i, &(id_resena, persona, documento, _)) in resenas.iter().enumerate() {
        store
            .put_edge(arista(
                PRIMER_ID_ARISTA_REFACTOR_B + i,
                persona,
                id_resena,
                "REALIZA",
            ))
            .unwrap();
        store
            .put_edge(arista(
                PRIMER_ID_ARISTA_REFACTOR_B + 4 + i,
                id_resena,
                documento,
                "SOBRE",
            ))
            .unwrap();
        informe.aristas_creadas += 2;
    }

    // ── 5) CONTRARRESTA (157): la ronda 2 de Fabio contrarresta a la ronda 1 ──
    let hay_ronda1 = resenas
        .iter()
        .any(|&(_, persona, _, ronda)| persona == ids::FABIO && ronda == 1);
    let hay_ronda2 = resenas
        .iter()
        .any(|&(_, persona, _, ronda)| persona == ids::FABIO && ronda == 2);
    if hay_ronda1 && hay_ronda2 {
        store
            .put_edge(arista(
                PRIMER_ID_ARISTA_REFACTOR_B + 8,
                RESENA_FABIO_RONDA2,
                RESENA_FABIO_RONDA1,
                "CONTRARRESTA",
            ))
            .unwrap();
        informe.aristas_creadas += 1;
    }

    informe.lecturas = lecturas;
    informe
}

#[cfg(test)]
mod tests_refactor_b {
    use super::*;

    /// La reificación responde las DOS preguntas que la arista plana no podía:
    /// «¿cuántas rondas pasó el Informe?» → 2 (dos Resena con SOBRE al
    /// Informe, con ronda 1 y 2 distintas); «¿qué reseña contrarresta a
    /// otra?» → la CONTRARRESTA 66→65, con {nota:8, ronda:2} contrarrestando
    /// {nota:7, ronda:1}. Y la deuda queda pagada: NINGUNA REVIEWED_BY
    /// residual en el grafo.
    #[test]
    fn refactor_reificar_resena_responde_rondas_y_contrarrestas() {
        let mut s = kb_lira_paso2_degrado();
        let informe = refactor_b_reificar_resena(&mut s);

        // Informe: 4 nodos y 9 aristas creadas, 4 aristas borradas, 0 nodos
        // borrados, 1 lectura (un único barrido de iter_edges).
        assert_eq!(informe.nodos_creados, 4);
        assert_eq!(informe.nodos_borrados, 0);
        assert_eq!(informe.aristas_creadas, 9); // 4 REALIZA + 4 SOBRE + 1 CONTRARRESTA
        assert_eq!(informe.aristas_borradas, 4);
        assert_eq!(informe.lecturas, 1);

        // ¿Cuántas rondas pasó el Informe de revisión por pares? → 2: dos
        // Resena con SOBRE al Informe, con ronda 1 y 2 distintas.
        let mut rondas: Vec<i64> = s
            .iter_edges()
            .filter(|e| e.label == "SOBRE" && e.target == ids::DOC_REVISION_PARES)
            .map(|e| {
                let n = s.get_node(e.source).expect("Resena de la SOBRE presente");
                match n.props.get("ronda") {
                    Some(Value::Int(r)) => *r,
                    _ => panic!("Resena {} sin ronda:Int", n.id),
                }
            })
            .collect();
        rondas.sort_unstable();
        assert_eq!(rondas, vec![1, 2], "el Informe pasó 2 rondas distintas");

        // ¿Qué reseña contrarresta a otra? → 66 (ronda 2) contrarresta a 65
        // (ronda 1), con sus notas 8 y 7.
        let contr = s
            .iter_edges()
            .find(|e| e.label == "CONTRARRESTA")
            .expect("la CONTRARRESTA 66→65 debe existir");
        assert_eq!(
            (contr.source, contr.target),
            (RESENA_FABIO_RONDA2, RESENA_FABIO_RONDA1)
        );
        let ronda2 = s.get_node(RESENA_FABIO_RONDA2).expect("Resena 66 presente");
        let ronda1 = s.get_node(RESENA_FABIO_RONDA1).expect("Resena 65 presente");
        assert_eq!(ronda2.props.get("nota"), Some(&Value::Int(8)));
        assert_eq!(ronda2.props.get("ronda"), Some(&Value::Int(2)));
        assert_eq!(ronda1.props.get("nota"), Some(&Value::Int(7)));
        assert_eq!(ronda1.props.get("ronda"), Some(&Value::Int(1)));

        // YA NO existe ninguna arista REVIEWED_BY: la deuda está pagada.
        assert_eq!(
            s.iter_edges().filter(|e| e.label == "REVIEWED_BY").count(),
            0,
            "ninguna REVIEWED_BY residual tras la reificación"
        );

        // Las 4 REALIZA parten de la persona correcta: Fabio→65 y 66,
        // Carla→67, Gaby→68 (en el orden de ids del barrido).
        let realiza: Vec<(usize, usize)> = s
            .iter_edges()
            .filter(|e| e.label == "REALIZA")
            .map(|e| (e.source, e.target))
            .collect();
        assert_eq!(
            realiza,
            vec![
                (ids::FABIO, RESENA_FABIO_RONDA1),
                (ids::FABIO, RESENA_FABIO_RONDA2),
                (ids::CARLA, RESENA_CARLA),
                (GABY, RESENA_GABY),
            ]
        );
    }
}

// ─────────────────── Refactor C: conferencia en nodo, temas-año en propiedad ───────────────────

/// Ids de los nodos `:Conferencia` del refactor C (62-64): continúan la
/// numeración (el paso-2 termina en 58, el refactor A en 61 y el refactor B
/// arranca en 65 — 62-64 quedan libres para las conferencias).
const CONFERENCIA_ICDE: usize = 62;
const CONFERENCIA_SIGMOD: usize = 63;
const CONFERENCIA_VLDB: usize = 64;

/// Primer id de arista libre tras el refactor B (que termina en 157). El
/// refactor C asigna las 24 `PUBLICADO_EN` como bloque contiguo (158-181).
const PRIMER_ID_ARISTA_REFACTOR_C: usize = 158;

/// Refactor C: **la propiedad que nadie puede expandir pasa a nodo y el nodo
/// categórico pasa a propiedad** (contrato §8) — los DOS antipatrones
/// simétricos del paso-2 se cobran en una pieza. Sobre el grafo degenerado:
///
/// 1. `conferencia:String` → nodo: crea 3 nodos `:Conferencia` (ids 62-64,
///    «ICDE 2024», «SIGMOD 2024», «VLDB 2025») y 24 aristas `PUBLICADO_EN`
///    (Documento→Conferencia, ids 158-181): una por cada paper del lote, al
///    nodo de SU conferencia. BORRA la propiedad `conferencia` de cada paper:
///    el API del cap. 8 NO ofrece `remove_prop` y `put_node` rechaza
///    duplicados (la única vía por trait sería `delete_node`, que se llevaría
///    las aristas por cascada) — se muta el campo `pub nodes` del
///    `MemoryStore` directamente, el MISMO estilo que cap33 (que asigna
///    `mutante.nodes[4] = None`): la eliminación real de la clave, no un
///    `Value::Null` que dejaría un rastro del antipatrón.
/// 2. Temas-año → propiedad: BORRA los 2 Temas degenerados 57-58
///    («publicaciones 2024/2025») con sus 12 ABOUT (6+6, ids 118-129). El año
///    ya vive como `anio:Int` en cada paper del lote — el refactor no añade
///    nada porque la propiedad YA era la buena (R6 del cap-41).
///
/// El informe devuelto: 3 nodos y 24 aristas creadas, 2 nodos y 12 aristas
/// borradas, y 38 lecturas — 24 `get_node` (una por paper) + 2 `in_edges`
/// (una por tema-año) + 12 `get_edge` (una por ABOUT borrada).
pub fn refactor_c_conferencias_y_temas_anio(store: &mut MemoryStore) -> InformeRefactor {
    let mut informe = InformeRefactor::new();
    let mut lecturas: usize = 0;

    // ── 1) Nodos :Conferencia (62-64): el string que nadie expandía ──
    for (id, nombre) in [
        (CONFERENCIA_ICDE, "ICDE 2024"),
        (CONFERENCIA_SIGMOD, "SIGMOD 2024"),
        (CONFERENCIA_VLDB, "VLDB 2025"),
    ] {
        store.put_node(nodo(id, &["Conferencia"], nombre)).unwrap();
        informe.nodos_creados += 1;
    }

    // ── 2) PUBLICADO_EN + borrar conferencia:String de cada paper del lote ──
    for (proximo_id, paper) in (PRIMER_ID_ARISTA_REFACTOR_C..).zip(LOTE_INICIO..=LOTE_FIN) {
        let n = store.get_node(paper).expect("paper del lote presente");
        lecturas += 1;
        let conferencia = match n.props.get("conferencia") {
            Some(Value::String(c)) => c.clone(),
            _ => panic!("paper {paper} sin conferencia:String (semilla del refactor C)"),
        };
        let conf_id = match conferencia.as_str() {
            "ICDE 2024" => CONFERENCIA_ICDE,
            "SIGMOD 2024" => CONFERENCIA_SIGMOD,
            "VLDB 2025" => CONFERENCIA_VLDB,
            c => panic!("paper {paper}: conferencia no mapeada «{c}»"),
        };
        // Sin API de remove_prop en el trait (put_node rechaza duplicados):
        // mutación del campo público del MemoryStore — la eliminación real.
        let nodo = store
            .nodes
            .get_mut(paper)
            .and_then(|n| n.as_mut())
            .expect("paper del lote presente");
        assert!(
            nodo.props.remove("conferencia").is_some(),
            "paper {paper} debía llevar conferencia"
        );
        store
            .put_edge(arista(proximo_id, paper, conf_id, "PUBLICADO_EN"))
            .unwrap();
        informe.aristas_creadas += 1;
    }

    // ── 3) Temas-año (57-58) y sus 12 ABOUT: el año ya es anio:Int ──
    for tema in [TEMA_PUBLICACIONES_2024, TEMA_PUBLICACIONES_2025] {
        let entrantes = store.in_edges(tema);
        lecturas += 1;
        for eid in entrantes {
            let e = store.get_edge(eid).expect("arista al tema-año presente");
            lecturas += 1;
            assert_eq!(
                e.label, "ABOUT",
                "la única arista hacia el tema-año {tema} es ABOUT (era {})",
                e.label
            );
            assert!(store.delete_edge(eid), "la ABOUT {eid} debe existir");
            informe.aristas_borradas += 1;
        }
        assert!(store.delete_node(tema), "el Tema {tema} debe existir");
        informe.nodos_borrados += 1;
    }

    informe.lecturas = lecturas;
    informe
}

#[cfg(test)]
mod tests_refactor_c {
    use super::*;

    /// El refactor C paga la deuda SIMÉTRICA del paso-2: la propiedad
    /// `conferencia` (el string que nadie expande) se convierte en nodo
    /// `:Conferencia` con PUBLICADO_EN, y los Temas-año 57-58 (el nodo
    /// categórico de baja cardinalidad) se borran porque `anio` ya existía.
    /// La pregunta «¿qué papers publicó el equipo en ICDE?» se responde
    /// expandiendo desde el nodo :Conferencia 62.
    #[test]
    fn refactor_conferencias_convierte_propiedad_en_nodo_y_temas_anio_en_propiedad() {
        let mut s = kb_lira_paso2_degrado();

        // ANTES: los papers con conferencia="ICDE 2024" — la pieza 1 sembró 8
        // papers por conferencia (33-40 ICDE, 41-48 SIGMOD, 49-56 VLDB).
        let antes_icde: Vec<usize> = s
            .iter_nodes()
            .filter(|n| (LOTE_INICIO..=LOTE_FIN).contains(&n.id))
            .filter(|n| {
                matches!(
                    n.props.get("conferencia"),
                    Some(Value::String(c)) if c == "ICDE 2024"
                )
            })
            .map(|n| n.id)
            .collect();
        assert_eq!(
            antes_icde.len(),
            8,
            "8 papers por conferencia en la pieza 1"
        );
        assert_eq!(antes_icde, (33..=40).collect::<Vec<usize>>());

        let informe = refactor_c_conferencias_y_temas_anio(&mut s);

        // Informe: 3 nodos y 24 aristas creadas; 2 nodos y 12 aristas borradas;
        // 38 lecturas (24 get_node + 2 in_edges + 12 get_edge).
        assert_eq!(informe.nodos_creados, 3);
        assert_eq!(informe.nodos_borrados, 2);
        assert_eq!(informe.aristas_creadas, 24);
        assert_eq!(informe.aristas_borradas, 12);
        assert_eq!(informe.lecturas, 38);

        // EXACTAMENTE 24 PUBLICADO_EN, una por paper del lote (33-56).
        let publicados: Vec<(usize, usize)> = s
            .iter_edges()
            .filter(|e| e.label == "PUBLICADO_EN")
            .map(|e| (e.source, e.target))
            .collect();
        assert_eq!(publicados.len(), 24, "una PUBLICADO_EN por paper del lote");
        let mut papers: Vec<usize> = publicados.iter().map(|&(p, _)| p).collect();
        papers.sort_unstable();
        assert_eq!(papers, (LOTE_INICIO..=LOTE_FIN).collect::<Vec<usize>>());

        // Los nodos 57-58 YA NO existen y no queda ninguna ABOUT hacia ellos.
        assert!(
            s.get_node(TEMA_PUBLICACIONES_2024).is_none(),
            "el Tema publicaciones-2024 desaparece"
        );
        assert!(
            s.get_node(TEMA_PUBLICACIONES_2025).is_none(),
            "el Tema publicaciones-2025 desaparece"
        );
        let about_residual: Vec<usize> = s
            .iter_edges()
            .filter(|e| {
                e.label == "ABOUT"
                    && (e.target == TEMA_PUBLICACIONES_2024 || e.target == TEMA_PUBLICACIONES_2025)
            })
            .map(|e| e.id)
            .collect();
        assert!(
            about_residual.is_empty(),
            "ninguna ABOUT hacia 57/58 (obtenido: {about_residual:?})"
        );

        // No queda ninguna prop `conferencia` en ningún paper del lote.
        let con_conferencia: Vec<usize> = s
            .iter_nodes()
            .filter(|n| (LOTE_INICIO..=LOTE_FIN).contains(&n.id))
            .filter(|n| n.props.contains_key("conferencia"))
            .map(|n| n.id)
            .collect();
        assert!(
            con_conferencia.is_empty(),
            "ningún paper conserva conferencia (obtenido: {con_conferencia:?})"
        );

        // «¿qué papers publicó el equipo en ICDE?» expandiendo desde el nodo
        // :Conferencia 62: PUBLICADO_EN hacia 62 == los que tenían
        // conferencia="ICDE 2024" antes del refactor.
        let mut icde: Vec<usize> = s
            .iter_edges()
            .filter(|e| e.label == "PUBLICADO_EN" && e.target == CONFERENCIA_ICDE)
            .map(|e| e.source)
            .collect();
        icde.sort_unstable();
        assert_eq!(
            icde, antes_icde,
            "la expansión desde :Conferencia 62 responde la pregunta"
        );
    }
}

// ─────────────────── Validador del contrato paso-2 ───────────────────

/// Tipos de arista que el contrato del paso-2 GOBIERNA con sus propias
/// reglas. El validador base del cap-41 no los conoce y los denuncia como
/// «tipo de arista desconocido»: al reutilizar un validador paso-1 sobre un
/// grafo paso-2 esas denuncias son esperables (el paso-1 no podía nombrar lo
/// que el lote aún no había sembrado) y se filtran — las reglas nuevas de
/// [`validar_modelo_kb_lira_paso2`] las sustituyen por las del contrato del
/// paso-2. `REVIEWED_BY` está en la lista porque su denuncia base es la misma
/// (tipo desconocido), pero la regla nueva NO la admite: si aparece, es deuda
/// residual del refactor B sin pagar.
const TIPOS_GOBERNADOS_POR_PASO2: &[&str] = &[
    "REVIEWED_BY",
    "REALIZA",
    "SOBRE",
    "CONTRARRESTA",
    "PUBLICADO_EN",
    "SUB_TEMA_DE",
];

/// Valida el contrato del modelo KB-Lira **paso-2** sobre `store`:
///
/// - REUTILIZA [`validar_modelo_kb_lira`] (cap-41) tal cual sobre el mismo
///   store: el subgrafo paso-1 que sigue dentro debe seguir cumpliendo su
///   contrato (extremos de `AUTHORED`/`CITES`/`ABOUT`/`MENTIONS`/
///   `MEMBER_OF`/`WORKED_ON`, `titulo`/`anio` en todo Documento, `order` en
///   AUTHORED). Solo se filtran las denuncias «tipo de arista desconocido»
///   sobre los tipos que el paso-2 añade (la lista de arriba): cualquier
///   OTRA violación del cap-41 se conserva.
/// - Reglas nuevas (los refactors A/B/C las pagan):
///   - `REALIZA` Persona→Resena; `SOBRE` Resena→Documento; `CONTRARRESTA`
///     Resena→Resena; `PUBLICADO_EN` Documento→Conferencia; `SUB_TEMA_DE`
///     Tema→Tema.
///   - Toda `Resena` con `nota:Int` y `ronda:Int` (las dos propiedades que
///     las 4 `REVIEWED_BY` paralelas no podían distinguir por significado).
///   - PROHIBIDO `REVIEWED_BY`: cualquier residual es la deuda del refactor B
///     sin pagar.
///
/// Devuelve `Ok(())` si no hay violaciones o la lista completa (las base
/// conservadas + las nuevas), para reportarlas todas de una pasada.
pub fn validar_modelo_kb_lira_paso2(store: &dyn GraphStore) -> Result<(), Vec<Violacion>> {
    let mut malas: Vec<Violacion> = Vec::new();

    // ── 1) Contrato base del cap-41, filtrado a lo que ES del paso-1 ──
    if let Err(base) = validar_modelo_kb_lira(store) {
        for v in base {
            let gobernada_por_paso2 = v.tipo_elemento == "arista"
                && store
                    .get_edge(v.id_implicado)
                    .map(|e| TIPOS_GOBERNADOS_POR_PASO2.contains(&e.label.as_str()))
                    .unwrap_or(false);
            if !gobernada_por_paso2 {
                malas.push(v);
            }
        }
    }

    // ── 2) Reglas nuevas: extremos de los tipos del paso-2 ──
    for ar in store.iter_edges() {
        let esperados: Option<(&str, &str)> = match ar.label.as_str() {
            "REALIZA" => Some(("Persona", "Resena")),
            "SOBRE" => Some(("Resena", "Documento")),
            "CONTRARRESTA" => Some(("Resena", "Resena")),
            "PUBLICADO_EN" => Some(("Documento", "Conferencia")),
            "SUB_TEMA_DE" => Some(("Tema", "Tema")),
            "REVIEWED_BY" => {
                malas.push(Violacion {
                    descripcion: "arista residual REVIEWED_BY: la reseña debe reificarse (refactor B)"
                        .into(),
                    id_implicado: ar.id,
                    tipo_elemento: "arista",
                });
                continue;
            }
            _ => None,
        };
        let Some((src_esperado, dst_esperado)) = esperados else {
            continue;
        };
        let src_ok = store
            .get_node(ar.source)
            .map(|n| n.has_label(src_esperado))
            .unwrap_or(false);
        let dst_ok = store
            .get_node(ar.target)
            .map(|n| n.has_label(dst_esperado))
            .unwrap_or(false);
        if !src_ok || !dst_ok {
            malas.push(Violacion {
                descripcion: format!(
                    "{}→{} viola los extremos de {} (esperado {}→{})",
                    ar.source, ar.target, ar.label, src_esperado, dst_esperado
                ),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            });
        }
    }

    // ── 3) Reglas nuevas: toda Resena con nota:Int y ronda:Int ──
    for n in store.iter_nodes() {
        if !n.has_label("Resena") {
            continue;
        }
        if !matches!(n.props.get("nota"), Some(Value::Int(_))) {
            malas.push(Violacion {
                descripcion: "Resena sin nota:Int".into(),
                id_implicado: n.id,
                tipo_elemento: "nodo",
            });
        }
        if !matches!(n.props.get("ronda"), Some(Value::Int(_))) {
            malas.push(Violacion {
                descripcion: "Resena sin ronda:Int".into(),
                id_implicado: n.id,
                tipo_elemento: "nodo",
            });
        }
    }

    if malas.is_empty() { Ok(()) } else { Err(malas) }
}

#[cfg(test)]
mod tests_validador_paso2 {
    use super::*;

    /// La cadena completa (refactor A → B → C) deja el grafo en un estado que
    /// CUMPLE el contrato del paso-2: el validador base del cap-41 pasa sobre
    /// el subgrafo paso-1 que sigue dentro (una vez filtradas sus denuncias de
    /// tipos desconocidos, que las reglas nuevas gobiernan) y las reglas
    /// nuevas pasan sobre el lote refactorizado.
    #[test]
    fn validador_paso2_acepta_el_modelo_refactorizado() {
        let mut s = kb_lira_paso2_degrado();
        refactor_a_descomponer_supernodo(&mut s);
        refactor_b_reificar_resena(&mut s);
        refactor_c_conferencias_y_temas_anio(&mut s);

        assert_eq!(
            validar_modelo_kb_lira_paso2(&s),
            Ok(()),
            "el modelo refactorizado cumple el contrato del paso-2"
        );
    }

    /// Tres corrupciones a mano sobre el MemoryStore (precedente cap-33 y
    /// refactor C: `store.nodes[..]`/`store.edges[..]`): la deuda sin pagar
    /// (REVIEWED_BY residual), una Resena sin `nota`, y una PUBLICADO_EN
    /// hacia un Documento. En los tres casos el validador falla con la
    /// violación CONCRETA (id y tipo esperados).
    #[test]
    fn validador_paso2_rechaza_fixture_corrupto() {
        // (a) Grafo degenerado SIN refactor: el validador base del cap-41
        // SOLO denuncia los 4 REVIEWED_BY como tipos desconocidos (el subgrafo
        // paso-1 está sano) — mensaje exacto del cap-41:
        let deg = kb_lira_paso2_degrado();
        let base = validar_modelo_kb_lira(&deg).unwrap_err();
        assert_eq!(base.len(), 4, "solo los 4 REVIEWED_BY son desconocidos");
        assert!(
            base.iter()
                .all(|v| v.descripcion == "tipo de arista desconocido 'REVIEWED_BY'"),
            "mensajes exactos del cap-41: {base:?}"
        );
        // El paso-2 lo rechaza: REVIEWED_BY residual, con los ids concretos
        // 130-133 (las 4 aristas del lote que la reificación debía pagar).
        let err = validar_modelo_kb_lira_paso2(&deg).unwrap_err();
        let mut ids: Vec<usize> = err.iter().map(|v| v.id_implicado).collect();
        ids.sort_unstable();
        assert_eq!(ids, (130..=133).collect::<Vec<usize>>());
        assert!(
            err.iter()
                .all(|v| v.tipo_elemento == "arista" && v.descripcion.contains("REVIEWED_BY")),
            "la única deuda pendiente es la REVIEWED_BY: {err:?}"
        );

        // (b) Refactor completo + una Resena sin `nota:Int`: falla con el id
        // concreto del nodo 65 (RESENA_FABIO_RONDA1).
        let mut s = kb_lira_paso2_degrado();
        refactor_a_descomponer_supernodo(&mut s);
        refactor_b_reificar_resena(&mut s);
        refactor_c_conferencias_y_temas_anio(&mut s);
        s.nodes[RESENA_FABIO_RONDA1]
            .as_mut()
            .expect("Resena 65 presente")
            .props
            .remove("nota");
        let err = validar_modelo_kb_lira_paso2(&s).unwrap_err();
        assert_eq!(err.len(), 1, "solo falta la nota: {err:?}");
        assert_eq!(err[0].id_implicado, RESENA_FABIO_RONDA1);
        assert_eq!(err[0].tipo_elemento, "nodo");
        assert!(
            err[0].descripcion.contains("nota"),
            "{}",
            err[0].descripcion
        );

        // (c) Refactor completo + la PUBLICADO_EN 158 (paper 33) redirigida a
        // un Documento (21): falla con el id concreto de la arista.
        let mut s = kb_lira_paso2_degrado();
        refactor_a_descomponer_supernodo(&mut s);
        refactor_b_reificar_resena(&mut s);
        refactor_c_conferencias_y_temas_anio(&mut s);
        s.edges[PRIMER_ID_ARISTA_REFACTOR_C]
            .as_mut()
            .expect("PUBLICADO_EN 158 presente")
            .target = ids::DOC_REVISION_PARES;
        let err = validar_modelo_kb_lira_paso2(&s).unwrap_err();
        assert_eq!(err.len(), 1, "solo la PUBLICADO_EN está corrupta: {err:?}");
        assert_eq!(err[0].id_implicado, PRIMER_ID_ARISTA_REFACTOR_C);
        assert_eq!(err[0].tipo_elemento, "arista");
        assert!(
            err[0].descripcion.contains("PUBLICADO_EN"),
            "{}",
            err[0].descripcion
        );
    }
}

// ─────────────────── Regresión: las 10 preguntas del paso-1 tras el refactor ───────────────────

/// Regresión de los 3 refactors (A descomponer-supernodo, B reificar-reseña,
/// C conferencias/temas-año) sobre las 10 preguntas del cap-41: cada pregunta
/// responde IDÉNTICO sobre el subgrafo paso-1 (ids 0-29) después del refactor
/// completo. El lote (ids >= 30) puede enriquecer algunas respuestas — P5b
/// devuelve 12 docs para el tema 24 (6 del paso-1 + 6 del lote que se quedan)
/// y P8 suma authorships del lote (Gaby/Hugo/Iris y papers 51-56 de
/// Fabio/Elena) —: esas filas NO participan en el contrato de esta pieza y se
/// filtran resolviendo el id del nodo (titulo/nombre) en el store
/// refactorizado; en P8 la respuesta no expone ids, así que el conteo del
/// subgrafo paso-1 se recalcula contando solo las AUTHORED a documentos
/// paso-1 (ver [`publicaciones_por_persona_en_paso1`]).
#[cfg(test)]
mod tests_regresion_preguntas_paso1 {
    use super::*;
    use crate::cap41_modelado::pregunta_01_documentos_de_una_persona as q01;
    use crate::cap41_modelado::pregunta_02_autores_en_orden_de_firma as q02;
    use crate::cap41_modelado::pregunta_03_citas_en_ambas_direcciones as q03;
    use crate::cap41_modelado::pregunta_04_proyecto_y_afiliaciones_en_dos_saltos as q04;
    use crate::cap41_modelado::pregunta_05_temas_de_un_documento_e_inversa as q05;
    use crate::cap41_modelado::pregunta_06_menciones_a_una_entidad as q06;
    use crate::cap41_modelado::pregunta_07_copublicacion_entre_dos_personas as q07;
    use crate::cap41_modelado::pregunta_09_temas_comunes_via_papers as q09;
    use crate::cap41_modelado::pregunta_10_citas_recientes_que_tratan_un_tema as q10;

    /// Primer id del lote: el subgrafo paso-1 ocupa los ids 0..30.
    const LOTE_INICIO: usize = 30;

    /// ¿El valor `campo` (`titulo`/`nombre`) pertenece a un nodo del subgrafo
    /// paso-1 (id < 30) en `store`?
    fn es_paso1(store: &dyn GraphStore, campo: &str, valor: &str) -> bool {
        store.iter_nodes().any(|n| {
            n.id < LOTE_INICIO && n.props.get(campo) == Some(&Value::String(valor.to_string()))
        })
    }

    /// P8 restringida al subgrafo paso-1: `pregunta_08` devuelve (nombre,
    /// conteo) SIN ids, y el lote añade AUTHORED desde personas del paso-1
    /// (Fabio→51-53, Elena→54-56) — recontar solo las AUTHORED a documentos
    /// con id < 30 es la única forma de comparar el subgrafo paso-1 sin
    /// contaminación del lote.
    fn publicaciones_por_persona_en_paso1(store: &dyn GraphStore) -> Vec<(String, usize)> {
        let mut conteo: HashMap<String, usize> = HashMap::new();
        for e in store
            .iter_edges()
            .filter(|e| e.label == "AUTHORED" && e.target < LOTE_INICIO)
        {
            let Some(n) = store.get_node(e.source) else {
                continue;
            };
            if let Some(Value::String(nombre)) = n.props.get("nombre") {
                *conteo.entry(nombre.clone()).or_insert(0) += 1;
            }
        }
        let mut v: Vec<(String, usize)> = conteo.into_iter().collect();
        v.sort();
        v
    }

    #[test]
    fn las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor() {
        // ── Referencia: las 10 preguntas sobre KB-Lira paso-1 (ids 0-29) ──
        let paso1 = kb_lira_paso1();
        let r1 = q01(&paso1, "Ana");
        let r2 = q02(&paso1, "Memoria episódica en LLMs");
        let (r3a, r3b) = q03(&paso1, "Supernodos: anatomía de un cuello de botella");
        let r4 = q04(&paso1, "Proyecto Kira");
        let (r5a, r5b) = q05(
            &paso1,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        let r6 = q06(&paso1, "Instituto Neurónica");
        let r7 = q07(&paso1, "Ana", "Beto");
        let r9 = q09(&paso1, "Ana", "Beto");
        let r10 = q10(
            &paso1,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        );

        // ── Store paso-2 DEGRADADO + los 3 refactors del cap-42 ──
        let mut store = kb_lira_paso2_degrado();
        refactor_a_descomponer_supernodo(&mut store);
        refactor_b_reificar_resena(&mut store);
        refactor_c_conferencias_y_temas_anio(&mut store);

        // ── Las mismas 10 preguntas sobre el store refactorizado ──
        let a1 = q01(&store, "Ana");
        let a2 = q02(&store, "Memoria episódica en LLMs");
        let (a3a, a3b) = q03(&store, "Supernodos: anatomía de un cuello de botella");
        let a4 = q04(&store, "Proyecto Kira");
        let (a5a, a5b) = q05(
            &store,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        let a6 = q06(&store, "Instituto Neurónica");
        let a7 = q07(&store, "Ana", "Beto");
        let a9 = q09(&store, "Ana", "Beto");
        let a10 = q10(
            &store,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        );

        // ── Comparación SOLO sobre el subgrafo paso-1 (ids < 30) ──
        let solo_titulos = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&store, "titulo", t))
                .cloned()
                .collect()
        };
        let solo_nombres = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&store, "nombre", t))
                .cloned()
                .collect()
        };

        assert_eq!(
            solo_titulos(&a1),
            solo_titulos(&r1),
            "P01 documentos_de_una_persona(Ana): el subgrafo paso-1 no cambia (obtenido: {a1:?})"
        );

        assert_eq!(
            a2, r2,
            "P02 autores_en_orden_de_firma(Memoria episódica en LLMs): no cambia (obtenido: {a2:?})"
        );

        assert_eq!(
            (solo_titulos(&a3a), solo_titulos(&a3b)),
            (solo_titulos(&r3a), solo_titulos(&r3b)),
            "P03 citas_en_ambas_direcciones(Supernodos…): el subgrafo paso-1 no cambia \
             (sale={a3a:?} entra={a3b:?})"
        );

        assert_eq!(
            a4, r4,
            "P04 proyecto_y_afiliaciones_en_dos_saltos(Proyecto Kira): no cambia (obtenido: {a4:?})"
        );

        assert_eq!(
            (solo_nombres(&a5a), solo_titulos(&a5b)),
            (solo_nombres(&r5a), solo_titulos(&r5b)),
            "P05 temas_e_inversa(Informe Kira | grafos de conocimiento): el subgrafo paso-1 no \
             cambia (temas={a5a:?} docs={a5b:?})"
        );

        assert_eq!(
            solo_titulos(&a6),
            solo_titulos(&r6),
            "P06 menciones_a_una_entidad(Instituto Neurónica): el subgrafo paso-1 no cambia \
             (obtenido: {a6:?})"
        );

        assert_eq!(
            solo_titulos(&a7),
            solo_titulos(&r7),
            "P07 copublicacion_entre_dos_personas(Ana,Beto): el subgrafo paso-1 no cambia \
             (obtenido: {a7:?})"
        );

        assert_eq!(
            publicaciones_por_persona_en_paso1(&store),
            publicaciones_por_persona_en_paso1(&paso1),
            "P08 publicaciones_por_persona_contadas: los conteos del subgrafo paso-1 no cambian"
        );

        assert_eq!(
            solo_nombres(&a9),
            solo_nombres(&r9),
            "P09 temas_comunes_via_papers(Ana,Beto): el subgrafo paso-1 no cambia (obtenido: {a9:?})"
        );

        let solo_paso1_citas = |v: &[(String, i64)]| -> Vec<(String, i64)> {
            v.iter()
                .filter(|(t, _)| es_paso1(&store, "titulo", t))
                .cloned()
                .collect()
        };
        assert_eq!(
            solo_paso1_citas(&a10),
            solo_paso1_citas(&r10),
            "P10 citas_recientes_que_tratan_un_tema(Grafos…, grafos de conocimiento, >2023): el \
             subgrafo paso-1 no cambia (obtenido: {a10:?})"
        );
    }
}

// ─────────────────── Las 5 preguntas del cap-41 sobre el grafo paso-2 REFACTORIZADO ───────────────────

/// Las 5 preguntas del cap-41 respondidas sobre el grafo paso-2 COMPLETO tras
/// la cadena de refactors (A descomponer-supernodo → B reificar-reseña → C
/// conferencias/temas-año), con valores REALES pineados (realidad primero:
/// cada expectativa se fijó ejecutando la pregunta contra el store).
///
/// El lote (ids >= 30) enriquece las respuestas donde el refactor lo permite:
/// - P5b con la consulta DIRECTA de la pregunta devuelve 12 docs del tema 24
///   (6 paso-1 + 6 del lote que se quedan en el padre); la unión jerárquica
///   [`documentos_del_tema_incluyendo_subtemas`] devuelve 24 (6 paso-1 + 18
///   del lote: los 12 movidos se encuentran vía los subtemas 59-61).
/// - Gaby/Hugo/Iris (30-32) NO traen `WORKED_ON`/`MEMBER_OF` del lote: P4
///   responde idéntico al paso-1.
#[cfg(test)]
mod tests_preguntas_paso2 {
    use super::*;
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

    /// El grafo sobre el que se ejecutan las 5 preguntas: KB-Lira paso-2
    /// degenerado + los 3 refactors del capítulo (A → B → C).
    fn grafo_paso2_refactorizado() -> MemoryStore {
        let mut s = kb_lira_paso2_degrado();
        refactor_a_descomponer_supernodo(&mut s);
        refactor_b_reificar_resena(&mut s);
        refactor_c_conferencias_y_temas_anio(&mut s);
        s
    }

    /// P1: los 4 documentos de Ana son EXACTAMENTE los del paso-1 — Ana no
    /// publicó en el lote (los AUTHORED del lote son Gaby/Hugo/Iris y
    /// Fabio/Elena).
    #[test]
    fn pregunta_01_documentos_de_una_persona_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q01(&store, "Ana"),
            vec![
                "Grafos de conocimiento para agentes",
                "Índices adaptativos para grafos",
                "Notas de la reunión de arranque",
                "Supernodos: anatomía de un cuello de botella",
            ],
            "P1(Ana): idéntico al paso-1 (Ana no publicó en el lote)"
        );
    }

    /// P2: el paper del LOTE id 33 («Grafos de conocimiento en producción»)
    /// firma SOLO Gaby con order 1 (los 24 papers del lote llevan un único
    /// AUTHORED{order:1}); el paper del paso-1 «Supernodos…» conserva Beto#1
    /// y Ana#2 — idéntico al paso-1.
    #[test]
    fn pregunta_02_autores_en_orden_de_firma_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q02(&store, "Grafos de conocimiento en producción"),
            vec![("Gaby".to_string(), 1)],
            "P2(paper del lote 33): único autor del lote con order 1"
        );
        assert_eq!(
            q02(&store, "Supernodos: anatomía de un cuello de botella"),
            vec![("Beto".to_string(), 1), ("Ana".to_string(), 2)],
            "P2(paper del paso-1): idéntico al paso-1 (Beto#1, Ana#2)"
        );
    }

    /// P3: las CITES del lote son INTERNAS (88-93, todas paper→paper del
    /// lote) y ninguna alcanza el paso-1: para «Supernodos…» la respuesta es
    /// idéntica al paso-1 (sale hacia Consultas declarativas e Índices
    /// adaptativos; entra solo desde Recuperación aumentada). Para el paper
    /// del lote 33: NO cita a nadie y es citado por los papers 34 y 36.
    #[test]
    fn pregunta_03_citas_en_ambas_direcciones_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        let (sale, entra) = q03(&store, "Supernodos: anatomía de un cuello de botella");
        assert_eq!(
            sale,
            vec![
                "Consultas declarativas sobre property graphs",
                "Índices adaptativos para grafos",
            ],
            "P3a(Supernodos…): salientes idénticas al paso-1"
        );
        assert_eq!(
            entra,
            vec!["Recuperación aumentada con grafos"],
            "P3a(Supernodos…): entrantes idénticas al paso-1 (el lote no lo cita)"
        );
        let (sale, entra) = q03(&store, "Grafos de conocimiento en producción");
        assert_eq!(
            sale,
            Vec::<String>::new(),
            "P3b(paper del lote 33): no cita a nadie"
        );
        assert_eq!(
            entra,
            vec![
                "Consulta declarativa de knowledge graphs a escala",
                "Unificación de entidades en knowledge graphs",
            ],
            "P3b(paper del lote 33): citado por los papers del lote 34 y 36"
        );
    }

    /// P5: los temas del «Informe anual del Proyecto Kira» siguen siendo solo
    /// «grafos de conocimiento». La parte «documentos del tema» de la
    /// pregunta usa la consulta DIRECTA (`<-[:ABOUT]-`): tras el refactor A
    /// devuelve 12 docs (6 paso-1 + 6 del lote que se quedan en el padre:
    /// 37-40, 45, 50) — valor REAL. La unión jerárquica
    /// [`documentos_del_tema_incluyendo_subtemas`] recupera los 24 (suma los
    /// 12 movidos a los subtemas 59-61), el conjunto completo que la pregunta
    /// NO ve con la consulta directa.
    #[test]
    fn pregunta_05_temas_de_un_documento_e_inversa_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        let (temas, docs) = q05(
            &store,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        assert_eq!(
            temas,
            vec!["grafos de conocimiento".to_string()],
            "P5a: el Informe Kira conserva su único tema (ABOUT 37 intacta)"
        );
        assert_eq!(
            docs,
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
            "P5b: consulta DIRECTA → 12 docs (6 paso-1 + 6 del lote que se quedan en el tema 24)"
        );
        // La jerarquía (directas ∪ subtemas) recupera el conjunto COMPLETO:
        // 24 docs — 6 paso-1 + 18 del lote.
        let ids_jerarquia =
            documentos_del_tema_incluyendo_subtemas(&store, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(
            ids_jerarquia.len(),
            24,
            "la unión jerárquica devuelve los 24 docs (obtenido: {ids_jerarquia:?})"
        );
        let mut titulos: Vec<String> = ids_jerarquia
            .iter()
            .filter_map(
                |&id| match store.get_node(id).and_then(|n| n.props.get("titulo")) {
                    Some(Value::String(t)) => Some(t.clone()),
                    _ => None,
                },
            )
            .collect();
        titulos.sort_by_cached_key(|t| {
            t.chars()
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
                .collect::<String>()
        });
        assert_eq!(
            titulos,
            vec![
                "Agregación de evidencias en grafos",
                "Anclaje semántico para grafos de conocimiento",
                "Consulta declarativa de knowledge graphs a escala",
                "Consultas declarativas sobre property graphs",
                "Control de calidad en grafos de conocimiento",
                "Evaluación de sistemas GraphRAG",
                "Fusión de grafos y texto para respuestas",
                "Grafos de conocimiento en producción",
                "Grafos de conocimiento para agentes",
                "Grafos de conocimiento temporales",
                "GraphRAG: recuperación aumentada con grafos",
                "Indexación de vecindarios para recuperación",
                "Informe anual del Proyecto Kira",
                "Materialización incremental de reglas en grafos",
                "Memoria gráfica para asistentes conversacionales",
                "Notas de la reunión de arranque",
                "Particionado de knowledge graphs para recuperación",
                "Razonamiento multihop sobre grafos de conocimiento",
                "Recuperación aumentada con grafos",
                "Reranking de párrafos con caminos de grafos",
                "Resumen del taller de GQL",
                "Streaming de aristas sobre grafos de conocimiento",
                "Unificación de entidades en knowledge graphs",
                "Versionado de ontologías en grafos",
            ],
            "P5b jerárquico: los 24 docs del tema 24 vía directas ∪ subtemas (obtenido: {titulos:?})"
        );
    }

    /// P4: las afiliaciones del Proyecto Kira siguen intactas (Ana→
    /// Universidad de Lira, Beto→Instituto Neurónica, Dani→Instituto
    /// Neurónica) — el lote NO siembra `WORKED_ON`/`MEMBER_OF` para
    /// Gaby/Hugo/Iris, así que ninguna persona nueva entra en la respuesta.
    #[test]
    fn pregunta_04_proyecto_y_afiliaciones_en_dos_saltos_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q04(&store, "Proyecto Kira"),
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ],
            "P4(Proyecto Kira): idéntico al paso-1"
        );
        let personas_lote_con_vinculo: Vec<(usize, String)> = store
            .iter_edges()
            .filter(|e| matches!(e.label.as_str(), "WORKED_ON" | "MEMBER_OF"))
            .filter(|e| (GABY..=IRIS).contains(&e.source))
            .map(|e| (e.source, e.label.clone()))
            .collect();
        assert!(
            personas_lote_con_vinculo.is_empty(),
            "Gaby/Hugo/Iris (30-32) no traen WORKED_ON/MEMBER_OF del lote \
             (obtenido: {personas_lote_con_vinculo:?})"
        );
    }

    #[test]
    fn pregunta_06_menciones_a_una_entidad_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q06(&store, "Instituto Neurónica"),
            vec![
                "Bitácora del experimento K-7",
                "Informe anual del Proyecto Kira",
                "Informe de revisión por pares 2025",
                "Informe técnico del Proyecto Oráculo",
                "Memoria episódica en LLMs",
            ],
            "P6(Instituto Neurónica): idéntico al paso-1 — el lote NO añade MENTIONS \
             (los únicos tipos del lote son AUTHORED/CITES/ABOUT/REVIEWED_BY)"
        );
    }

    #[test]
    fn pregunta_07_copublicacion_entre_dos_personas_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q07(&store, "Ana", "Beto"),
            vec![
                "Grafos de conocimiento para agentes",
                "Supernodos: anatomía de un cuello de botella",
            ],
            "P7(Ana,Beto): las 2 co-publicaciones del paso-1 — el lote no toca a Ana/Beto"
        );
        assert_eq!(
            q07(&store, "Fabio", "Elena"),
            vec!["Recuperación aumentada con grafos"],
            "P7(Fabio,Elena): co-publican SOLO en el paso-1 (DOC_RAG); los AUTHORED del lote \
             son disjuntos (Fabio→51-53, Elena→54-56)"
        );
        assert!(
            q07(&store, "Fabio", "Gaby").is_empty(),
            "P7(Fabio,Gaby): sin papers compartidos (Gaby→33-38, Fabio→51-53)"
        );
    }

    #[test]
    fn pregunta_08_publicaciones_por_persona_contadas_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q08(&store),
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
            "P8: 9 personas — los 6 del paso-1 (16 AUTHORED) + Gaby/Hugo/Iris (6 c/u) y \
             Fabio/Elena sumando sus 3 authorships del lote (5 y 6)"
        );
        let total: usize = q08(&store).iter().map(|(_, n)| n).sum();
        assert_eq!(total, 40, "16 AUTHORED del paso-1 + 24 del lote");
        let rs = crate::cap20_volcano::run(
            "MATCH (p:Persona)-[:AUTHORED]->(d:Documento) RETURN p.nombre AS autor",
            &store,
        )
        .unwrap();
        assert_eq!(rs.len(), 40, "el ResultSet expone las 40 aristas AUTHORED");
    }

    #[test]
    fn pregunta_09_temas_comunes_via_papers_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q09(&store, "Ana", "Beto"),
            vec![
                "grafos de conocimiento",
                "memoria de agentes",
                "rendimiento",
            ],
            "P9(Ana,Beto): idéntico al paso-1 — el lote no AUTHORED para Ana/Beto, así que \
             los subtemas 59-61 (papers 33-36/41-44/46-49) no entran en el camino"
        );
    }

    #[test]
    fn pregunta_10_citas_recientes_que_tratan_un_tema_sobre_paso2() {
        let store = grafo_paso2_refactorizado();
        assert_eq!(
            q10(
                &store,
                "Grafos de conocimiento para agentes",
                "grafos de conocimiento",
                2023
            ),
            vec![("Recuperación aumentada con grafos".to_string(), 2025)],
            "P10(paper del paso-1): idéntico al paso-1 — las CITES del lote (88-93) son \
             internas y ninguna alcanza el paso-1"
        );
        // Paper del LOTE 41 como citado: lo citan 45 y 54 (CITES 91 y 93), y SOLO 45
        // («Evaluación de sistemas GraphRAG», 2024) se queda con ABOUT directa al tema
        // 24 tras el refactor A — pero el resultado REAL es vacío: el título del citado
        // lleva «ó» y el lexer mini del cap. 18 corrompe literales UTF-8 multi-byte
        // (frontera ya documentada en el cap-41 para P2/P3/P6). Realidad primero.
        assert!(
            q10(
                &store,
                "GraphRAG: recuperación aumentada con grafos",
                "grafos de conocimiento",
                2023
            )
            .is_empty(),
            "P10(paper del lote 41): el filtro por título con acento nunca matchea \
             (frontera UTF-8 del lexer) — el 45 sería el único citante con ABOUT al 24"
        );
    }
}

// ─────────────────── El supernodo cobra el grado entero; el refactor lo reparte ───────────────────

/// Medida ESTRUCTURAL del coste de la consulta P5 «documentos del tema»:
/// cuántas aristas `ABOUT` recorre la expansión del tema y de sus subtemas —
/// espejo exacto de [`documentos_del_tema_incluyendo_subtemas`]. Devuelve
/// `(total, reparto)`: el total de aristas ABOUT leídas y cuántas leyó CADA
/// expansión, en orden de ejecución (tema → subtemas).
///
/// El instrumento de contraste es [`ContandoStore`](crate::cap26_proyeccion::ContandoStore)
/// (cap-26), que cuenta las llamadas REALES (`in_edges`/`get_edge`); el test
/// cruza ambas medidas.
pub fn contar_lecturas_de_expansion_about(
    store: &dyn GraphStore,
    tema_id: usize,
) -> (usize, Vec<usize>) {
    let entrantes = store.in_edges(tema_id);
    let mut reparto: Vec<usize> = Vec::new();
    let mut total: usize = 0;

    let directas = entrantes
        .iter()
        .filter(|&&eid| store.get_edge(eid).is_some_and(|e| e.label == "ABOUT"))
        .count();
    total += directas;
    reparto.push(directas);

    let subtemas: Vec<usize> = entrantes
        .iter()
        .filter_map(|&eid| {
            let e = store.get_edge(eid)?;
            (e.label == "SUB_TEMA_DE").then_some(e.source)
        })
        .collect();
    for sub in subtemas {
        let n = store
            .in_edges(sub)
            .iter()
            .filter(|&&eid| store.get_edge(eid).is_some_and(|e| e.label == "ABOUT"))
            .count();
        total += n;
        reparto.push(n);
    }

    (total, reparto)
}

#[cfg(test)]
mod tests_coste_expansion {
    use super::*;
    use crate::cap26_proyeccion::ContandoStore;

    /// Tesis estructural del capítulo, con números: expandir «¿qué documentos
    /// tratan el tema 24?» desde el supernodo DEGENERADO paga el grado entero
    /// (24 aristas ABOUT en UNA expansión); tras el refactor A el coste se
    /// REPARTE (12 + 4 + 4 + 4 en 4 expansiones) sin cambiar el resultado.
    #[test]
    fn el_supernodo_cobra_el_grado_entero_en_cada_expansion_y_el_refactor_lo_reparte() {
        // ── Grafo DEGENERADO: el hub cobra el grado entero en UNA expansión ──
        let s = kb_lira_paso2_degrado();
        let docs = documentos_del_tema_incluyendo_subtemas(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(docs.len(), 24, "6 del paso-1 + 18 del lote");

        let (total, reparto) =
            contar_lecturas_de_expansion_about(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(total, 24, "el supernodo cobra sus 24 aristas ABOUT");
        assert_eq!(
            reparto,
            vec![24],
            "el DEGENERADO lee el grado entero en UNA sola expansión"
        );

        let medidor = ContandoStore::new(&s);
        let _ = documentos_del_tema_incluyendo_subtemas(&medidor, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(medidor.consultas_in(), 1, "una única in_edges al tema 24");
        assert_eq!(medidor.lecturas_arista(), 24, "24 get_edge, una por ABOUT");
        assert_eq!(medidor.total_lecturas(), 25);

        // ── Grafo REFACTORIZADO: las MISMAS 24 aristas, REPARTIDAS ──
        let mut s = kb_lira_paso2_degrado();
        let _informe = refactor_a_descomponer_supernodo(&mut s);

        let docs_refactor =
            documentos_del_tema_incluyendo_subtemas(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(
            docs_refactor, docs,
            "el refactor conserva el conjunto (24 docs)"
        );

        let (total_refactor, reparto_refactor) =
            contar_lecturas_de_expansion_about(&s, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(total_refactor, 24, "las mismas 24 aristas ABOUT en total");
        assert_eq!(
            reparto_refactor,
            vec![12, 4, 4, 4],
            "12 ABOUT directas al 24 + 4 por subtema (59/60/61): el coste se REPARTE en 4 expansiones"
        );

        let medidor = ContandoStore::new(&s);
        let _ = documentos_del_tema_incluyendo_subtemas(&medidor, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(
            medidor.consultas_in(),
            4,
            "1 in_edges al tema 24 + 1 por subtema"
        );
        assert_eq!(
            medidor.lecturas_arista(),
            27,
            "15 get_edge del tema (12 ABOUT + 3 SUB_TEMA_DE) + 12 de los subtemas"
        );
        assert_eq!(medidor.total_lecturas(), 31);

        // ── Honestidad del fixture pequeño: el TOTAL es el mismo (24 = 24).
        //    El refactor NO reduce el coste aquí: lo REPARTE (y paga 3
        //    lecturas extra por las SUB_TEMA_DE que la unión debe cruzar). El
        //    ahorro REAL aparece a escala: si el supernodo tiene 10M de ABOUT
        //    y una consulta cruza SOLO un subtema, paga 4 lecturas en vez de
        //    las 10M del hub completo.
        assert_eq!(total, total_refactor, "mismo coste total en este fixture");
        let expansion_minima = reparto_refactor.iter().copied().min().unwrap();
        assert_eq!(
            expansion_minima, 4,
            "una expansión de UN subtema lee 4 aristas"
        );
        assert_eq!(
            expansion_minima * 6,
            total,
            "4 × 6 = 24: la consulta de un solo subtema paga 1/6 del hub — y a escala \
             (10M de aristas) paga 4 en vez de 10M"
        );
    }
}

// ─────────────────── La migración completa: los tres refactors en cadena ───────────────────

/// Informe agregado de la migración completa (contrato §2): los tres
/// [`InformeRefactor`] en el orden del refactor (A → B → C), el total sumado
/// y el impacto sobre las 10 preguntas del cap-41 (una línea por pregunta).
/// [`Display`](std::fmt::Display) lo pinta como tabla para la prosa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformeMigracion {
    /// Informe del refactor A (descomponer el supernodo).
    pub refactor_a: InformeRefactor,
    /// Informe del refactor B (reificar la reseña).
    pub refactor_b: InformeRefactor,
    /// Informe del refactor C (conferencias en nodo, temas-año en propiedad).
    pub refactor_c: InformeRefactor,
    /// Suma de los tres informes: el coste COMPLETO de la migración.
    pub total: InformeRefactor,
    /// Una línea por pregunta del cap-41, clasificada ejecutando la pregunta
    /// REAL contra KB-Lira paso-1 (referencia) y contra el grafo refactorizado.
    pub impacto_preguntas: Vec<String>,
}

/// Aplica la migración completa sobre el grafo degenerado: los TRES refactors
/// en el orden del contrato (A descomponer supernodo → B reificar reseña → C
/// conferencias/temas-año), agregando sus [`InformeRefactor`] en un
/// [`InformeMigracion`] con el total y el impacto sobre las 10 preguntas.
pub fn la_migracion_completa(store: &mut MemoryStore) -> InformeMigracion {
    let refactor_a = refactor_a_descomponer_supernodo(store);
    let refactor_b = refactor_b_reificar_resena(store);
    let refactor_c = refactor_c_conferencias_y_temas_anio(store);
    let total = InformeRefactor {
        nodos_creados: refactor_a.nodos_creados
            + refactor_b.nodos_creados
            + refactor_c.nodos_creados,
        nodos_borrados: refactor_a.nodos_borrados
            + refactor_b.nodos_borrados
            + refactor_c.nodos_borrados,
        aristas_creadas: refactor_a.aristas_creadas
            + refactor_b.aristas_creadas
            + refactor_c.aristas_creadas,
        aristas_borradas: refactor_a.aristas_borradas
            + refactor_b.aristas_borradas
            + refactor_c.aristas_borradas,
        lecturas: refactor_a.lecturas + refactor_b.lecturas + refactor_c.lecturas,
    };
    let impacto_preguntas = impacto_en_las_10_preguntas(store);
    InformeMigracion {
        refactor_a,
        refactor_b,
        refactor_c,
        total,
        impacto_preguntas,
    }
}

/// Clasifica el impacto de la migración sobre las 10 preguntas del cap-41:
/// ejecuta cada pregunta REAL contra KB-Lira paso-1 (la referencia del
/// contrato) y contra el grafo refactorizado, y emite una línea por pregunta.
///
/// - «idéntica»: la respuesta no cambió (P1-P4, P6-P7, P9-P10).
/// - «jerárquica N»: P5 — la consulta DIRECTA de la pregunta ve 12 docs tras
///   el refactor A (6 paso-1 + 6 del lote que se quedan en el padre), pero la
///   unión jerárquica [`documentos_del_tema_incluyendo_subtemas`] recupera los
///   24 completos (N).
/// - «enriquecida N»: P8 — el lote añade 24 AUTHORED (Gaby/Hugo/Iris y
///   Fabio/Elena): 40 aristas totales (N), 9 personas.
fn impacto_en_las_10_preguntas(migrado: &MemoryStore) -> Vec<String> {
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

    let paso1 = kb_lira_paso1();
    let iguales: [bool; 10] = [
        q01(&paso1, "Ana") == q01(migrado, "Ana"),
        q02(&paso1, "Memoria episódica en LLMs") == q02(migrado, "Memoria episódica en LLMs"),
        q03(&paso1, "Supernodos: anatomía de un cuello de botella")
            == q03(migrado, "Supernodos: anatomía de un cuello de botella"),
        q04(&paso1, "Proyecto Kira") == q04(migrado, "Proyecto Kira"),
        q05(
            &paso1,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        ) == q05(
            migrado,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        ),
        q06(&paso1, "Instituto Neurónica") == q06(migrado, "Instituto Neurónica"),
        q07(&paso1, "Ana", "Beto") == q07(migrado, "Ana", "Beto"),
        q08(&paso1) == q08(migrado),
        q09(&paso1, "Ana", "Beto") == q09(migrado, "Ana", "Beto"),
        q10(
            &paso1,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        ) == q10(
            migrado,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        ),
    ];

    let mut lineas: Vec<String> = Vec::with_capacity(10);
    for (i, &igual) in iguales.iter().enumerate() {
        let p = i + 1;
        let etiqueta = if igual {
            "idéntica".to_string()
        } else {
            match p {
                5 => format!(
                    "jerárquica {}",
                    documentos_del_tema_incluyendo_subtemas(migrado, ids::TEMA_GRAFOS_CONOCIMIENTO)
                        .len()
                ),
                8 => format!(
                    "enriquecida {}",
                    q08(migrado).iter().map(|(_, n)| n).sum::<usize>()
                ),
                _ => "distinta".to_string(),
            }
        };
        lineas.push(format!("P{p}: {etiqueta}"));
    }
    lineas
}

/// Una fila de la tabla: nombre alineado a 38 columnas y los tres contadores
/// del informe (nodos y aristas como +creadas/-borradas, lecturas aparte).
fn fila_informe(f: &mut fmt::Formatter<'_>, nombre: &str, r: &InformeRefactor) -> fmt::Result {
    writeln!(
        f,
        "{nombre:<38} | {} nodos (+{}/-{}) | {} aristas (+{}/-{}) | {} lecturas",
        r.nodos_creados,
        r.nodos_creados,
        r.nodos_borrados,
        r.aristas_creadas,
        r.aristas_creadas,
        r.aristas_borradas,
        r.lecturas
    )
}

impl fmt::Display for InformeMigracion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Migración completa de KB-Lira: paso-2 degenerado → modelo refactorizado"
        )?;
        writeln!(f, "{}", "─".repeat(88))?;
        fila_informe(f, "Refactor A · descomponer supernodo", &self.refactor_a)?;
        fila_informe(f, "Refactor B · reificar reseña", &self.refactor_b)?;
        fila_informe(f, "Refactor C · conferencias / temas-año", &self.refactor_c)?;
        fila_informe(f, "TOTAL", &self.total)?;
        writeln!(f, "{}", "─".repeat(88))?;
        writeln!(
            f,
            "Impacto sobre las 10 preguntas del cap-41 (referencia: KB-Lira paso-1):"
        )?;
        for linea in &self.impacto_preguntas {
            writeln!(f, "{linea}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_migracion {
    use super::*;

    /// La migración completa paga las deudas del paso-2 con los contadores
    /// EXACTOS del contrato §2: 10 nodos creados / 2 borrados, 48 aristas
    /// creadas / 28 borradas y 64 lecturas (25 + 1 + 38), repartidas entre los
    /// tres refactors (A 3/0 y 15/12, B 4/0 y 9/4, C 3/2 y 24/12).
    #[test]
    fn la_migracion_completa_cuesta_reeescrituras_exactas() {
        let mut s = kb_lira_paso2_degrado();
        let m = la_migracion_completa(&mut s);

        // Refactor A: 3 subtemas (59-61), 0 borrados; 15 aristas (3
        // SUB_TEMA_DE + 12 ABOUT redistribuidas), 12 borradas; 25 lecturas.
        assert_eq!(m.refactor_a.nodos_creados, 3);
        assert_eq!(m.refactor_a.nodos_borrados, 0);
        assert_eq!(m.refactor_a.aristas_creadas, 15);
        assert_eq!(m.refactor_a.aristas_borradas, 12);
        assert_eq!(m.refactor_a.lecturas, 25);

        // Refactor B: 4 nodos :Resena (65-68); 9 aristas (4 REALIZA + 4 SOBRE
        // + 1 CONTRARRESTA), 4 REVIEWED_BY borradas; 1 lectura.
        assert_eq!(m.refactor_b.nodos_creados, 4);
        assert_eq!(m.refactor_b.nodos_borrados, 0);
        assert_eq!(m.refactor_b.aristas_creadas, 9);
        assert_eq!(m.refactor_b.aristas_borradas, 4);
        assert_eq!(m.refactor_b.lecturas, 1);

        // Refactor C: 3 nodos :Conferencia (62-64), 2 Temas-año borrados; 24
        // PUBLICADO_EN, 12 ABOUT borradas; 38 lecturas.
        assert_eq!(m.refactor_c.nodos_creados, 3);
        assert_eq!(m.refactor_c.nodos_borrados, 2);
        assert_eq!(m.refactor_c.aristas_creadas, 24);
        assert_eq!(m.refactor_c.aristas_borradas, 12);
        assert_eq!(m.refactor_c.lecturas, 38);

        // Totales: 10 nodos creados / 2 borrados; 48 aristas creadas / 28
        // borradas; 64 lecturas (25 + 1 + 38).
        assert_eq!(m.total.nodos_creados, 10);
        assert_eq!(m.total.nodos_borrados, 2);
        assert_eq!(m.total.aristas_creadas, 48);
        assert_eq!(m.total.aristas_borradas, 28);
        assert_eq!(m.total.lecturas, 64);
    }

    /// El informe de la migración es REPRODUCIBLE: `format!("{}", …)` produce
    /// una salida estable byte a byte (tabla de nodos/aristas/lecturas por
    /// refactor + totales + el impacto sobre las 10 preguntas, una línea por
    /// pregunta) — el literal de abajo es la salida REAL fijada a mano.
    #[test]
    fn informe_migracion_reproducible_sobre_kb_lira() {
        let mut s = kb_lira_paso2_degrado();
        let m = la_migracion_completa(&mut s);
        let reporte = format!("{m}");
        assert_eq!(
            reporte,
            "Migración completa de KB-Lira: paso-2 degenerado → modelo refactorizado\n\
             ────────────────────────────────────────────────────────────────────────────────────────\n\
             Refactor A · descomponer supernodo     | 3 nodos (+3/-0) | 15 aristas (+15/-12) | 25 lecturas\n\
             Refactor B · reificar reseña           | 4 nodos (+4/-0) | 9 aristas (+9/-4) | 1 lecturas\n\
             Refactor C · conferencias / temas-año  | 3 nodos (+3/-2) | 24 aristas (+24/-12) | 38 lecturas\n\
             TOTAL                                  | 10 nodos (+10/-2) | 48 aristas (+48/-28) | 64 lecturas\n\
             ────────────────────────────────────────────────────────────────────────────────────────\n\
             Impacto sobre las 10 preguntas del cap-41 (referencia: KB-Lira paso-1):\n\
             P1: idéntica\n\
             P2: idéntica\n\
             P3: idéntica\n\
             P4: idéntica\n\
             P5: jerárquica 24\n\
             P6: idéntica\n\
             P7: idéntica\n\
             P8: enriquecida 40\n\
             P9: idéntica\n\
             P10: idéntica\n",
            "la salida del informe debe estar pineada byte a byte"
        );
    }
}

#[cfg(test)]
mod tests_csv_paso2 {
    use super::*;
    use crate::cap32_import_export::{importar_csv_aristas, importar_csv_nodos};
    use std::io::BufReader;

    // Los tests llevan el nombre EXACTO del contrato; para llamar a las
    // funciones homónimas del cap-41 sin sombra, se re-importan con alias.
    use crate::cap41_modelado::csv_aristas_kb_lira as csv_aristas_paso1;
    use crate::cap41_modelado::csv_nodos_kb_lira as csv_nodos_paso1;

    /// Exporta nodos+aristas del paso-2 → importa (cap. 32) → exporta de
    /// nuevo: bytes IDÉNTICOS (mismo patrón que el roundtrip del cap-41).
    #[test]
    fn csv_roundtrip_paso2_import_export_byte_a_byte() {
        let s = kb_lira_paso2_degrado();
        let nodos_v1 = csv_nodos_kb_lira_paso2(&s);
        let aristas_v1 = csv_aristas_kb_lira_paso2(&s);

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

        let nodos_v2 = csv_nodos_kb_lira_paso2(&s2);
        let aristas_v2 = csv_aristas_kb_lira_paso2(&s2);
        assert_eq!(nodos_v1, nodos_v2, "roundtrip nodos byte a byte");
        assert_eq!(aristas_v1, aristas_v2, "roundtrip aristas byte a byte");
    }

    /// datasets/kb-lira/paso-2/ ES la salida de los builders del paso-2: si
    /// alguien regenera el builder y olvida commitear, este test grita
    /// (mismo mecanismo que el cap-41 contra datasets/kb-lira/paso-1/).
    #[test]
    fn csv_paso2_coincide_con_dataset_commiteado_byte_a_byte() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-2");
        let s = kb_lira_paso2_degrado();
        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso2(&s), esperado_nodos);
        assert_eq!(csv_aristas_kb_lira_paso2(&s), esperado_aristas);
    }

    /// El paso-2 NO toca el dataset del paso-1: los ficheros commiteados de
    /// datasets/kb-lira/paso-1/ siguen siendo la salida exacta de los
    /// builders del cap-41 (y el builder del paso-2 sigue construyendo sobre
    /// kb_lira_paso1(), sin copiarlo).
    #[test]
    fn csv_paso1_intacto_tras_paso2() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-1");
        let s1 = kb_lira_paso1();
        let s2 = kb_lira_paso2_degrado();

        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");

        // Los ficheros del paso-1 coinciden con lo que exportan los builders
        // del cap-41 sobre kb_lira_paso1().
        assert_eq!(csv_nodos_paso1(&s1), esperado_nodos);
        assert_eq!(csv_aristas_paso1(&s1), esperado_aristas);

        // Y el grafo del paso-2 ES el del paso-1 (más el lote): sus primeras
        // 30 nodos y 64 aristas son exactamente el paso-1.
        let nodos2 = csv_nodos_kb_lira_paso2(&s2);
        let aristas2 = csv_aristas_kb_lira_paso2(&s2);
        let filas_nodos: Vec<&str> = nodos2.lines().collect();
        let filas_aristas: Vec<&str> = aristas2.lines().collect();
        assert!(filas_nodos.len() > 31, "paso-2 añade nodos al paso-1");
        assert!(filas_aristas.len() > 65, "paso-2 añade aristas al paso-1");
    }
}
