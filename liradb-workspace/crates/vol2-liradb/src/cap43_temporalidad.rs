//! Vol.III — Cap.43: Temporalidad (valid-time) sobre KB-Lira.
//!
//! TERCER capítulo del Vol.III («Grafos en la era de la IA»), Parte I «Modelar
//! datos de grafos». El cap. 42 pagó las deudas estructurales del lote
//! importado (supernodo, reseñas sin identidad, conferencias-string); este
//! capítulo añade la DIMENSIÓN TIEMPO a las afiliaciones: las 6 `MEMBER_OF`
//! del paso-1 (ids 52-57) dejan de ser hechos atemporales y pasan a llevar
//! `desde_anio:Int` / `hasta_anio:Int` (ausencia = intervalo abierto).
//!
//! Modelo mental único: **en un grafo de conocimiento la temporalidad no es
//! una propiedad decorativa: es una pregunta que el modelo callado responde
//! MAL** («¿de quién es Beto HOY?» sobre KB-Lira responde Neurónica cuando se
//! fue en 2024). El valid-time convierte la pregunta en un predicado evaluable
//! sobre `ANIO_ACTUAL`.
//!
//! Qué entrega ESTA pieza (implementación incremental, contrato §2):
//!
//! 1. **Valid-time de las afiliaciones**: añade `desde_anio:Int` a las 6
//!    `MEMBER_OF` del paso-1 (52 Ana→UniLira 2018; 53 Beto→Neurónica 2018 con
//!    `hasta_anio:2024` — VENCIDA: Beto se muda; 54 Carla→UniLira 2020; 55
//!    Dani→Neurónica 2021 — el valor CORREGIDO del caso bitemporal; 56
//!    Elena→GrafosYa 2019; 57 Fabio→GrafosYa 2019).
//! 2. **Nuevas afiliaciones** (ids 182-185) + el nodo `:Organizacion`
//!    «Instituto GrafoLuna» (id 69): 182 Hugo→UniLira (2019), 183
//!    Iris→GrafosYa (2022), 184 Gaby→Neurónica (2023) y 185 Beto→GrafoLuna
//!    (2024) — la que hace coherente la salida de Beto de Neurónica.
//! 3. **Gancho reseña**: el ciclo de la reseña de Fabio también es temporal —
//!    la `REALIZA` 149 (ronda 1) gana `hasta_anio:2025` y la `CONTRARRESTA`
//!    157 (la ronda 2 contrarrestando a la ronda 1) gana `desde_anio:2025`.
//!
//! Resultado ([`kb_lira_paso3`]): 68 nodos, 158 aristas, 10 `MEMBER_OF`.
//! Informe de la operación ([`InformeValidTime`]): 8 aristas modificadas,
//! 4 aristas creadas, 1 nodo creado y 8 lecturas (una `get_edge` por arista
//! modificada).

use crate::cap07_modelo::{Edge, Node, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap32_import_export::{exportar_csv_aristas, exportar_csv_nodos};
use crate::cap41_modelado::{Violacion, ids, nodo_por_nombre};
use crate::cap42_antipatrones::{
    kb_lira_paso2_degrado, la_migracion_completa, validar_modelo_kb_lira_paso2,
};

/// El año de referencia del «hoy» de KB-Lira: toda consulta temporal evalúa
/// contra esta constante (el cap. 45 la convertirá en parámetro de consulta).
/// Semilla de contrato: aún sin consumidores dentro del crate (las piezas
/// posteriores la usan), por eso se desactiva la advertencia de código muerto.
#[allow(dead_code)]
pub const ANIO_ACTUAL: i64 = 2026;

/// Personas del lote del cap-42 (ids públicos en el builder de paso-2).
const GABY: usize = 30;
const HUGO: usize = 31;
const IRIS: usize = 32;

/// Id del nodo `:Organizacion` «Instituto GrafoLuna»: continúa la numeración
/// de nodos (el cap-42 termina en 68). Mismo shape que las organizaciones del
/// cap-41: label `Organizacion` + `nombre:String`.
const GRAPO_LUNA: usize = 69;

/// Ids fijos de las `MEMBER_OF` nuevas (182-185): continúan la numeración de
/// aristas (el cap-42 termina en 157).
const PRIMER_ID_MEMBER_OF_NUEVA: usize = 182;

/// Informe de lo que el valid-time hizo sobre el grafo: cuántas aristas
/// MODIFICADAS (props añadidas), creadas, nodos creados, y las LECTURAS que
/// costó — cada llamada de lectura al store (`get_edge`, …) incrementa el
/// contador; las escrituras (`put_*`) NO (misma convención que el cap-42).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformeValidTime {
    /// Aristas existentes a las que se añadieron props de validez.
    pub aristas_modificadas: usize,
    /// Aristas `MEMBER_OF` nuevas con su validez desde el alta.
    pub aristas_creadas: usize,
    /// Nodos creados (el `:Organizacion` «Instituto GrafoLuna»).
    pub nodos_creados: usize,
    /// Lecturas al store (una `get_edge` por arista modificada).
    pub lecturas: usize,
}

/// Añade `desde_anio:Int` (y `hasta_anio:Int` si el intervalo está cerrado) a
/// una arista existente, mutando sus props en sitio. Cuesta 1 lectura.
fn poner_validez(
    store: &mut MemoryStore,
    id: usize,
    desde: i64,
    hasta: Option<i64>,
    lecturas: &mut usize,
) {
    assert!(
        store.get_edge(id).is_some(),
        "la arista {id} debe existir para recibir validez"
    );
    *lecturas += 1;
    let e = store.edges[id].as_mut().expect("arista presente");
    e.props.insert("desde_anio".into(), Value::Int(desde));
    if let Some(h) = hasta {
        e.props.insert("hasta_anio".into(), Value::Int(h));
    }
}

/// `MEMBER_OF` nueva con su `desde_anio:Int` (intervalo abierto: sin
/// `hasta_anio`).
fn member_of_desde(id: usize, source: usize, target: usize, desde: i64) -> Edge {
    Edge::new(id, source, target, "MEMBER_OF").with_prop("desde_anio", Value::Int(desde))
}

/// Aplica el valid-time sobre el grafo refactorizado del cap-42:
///
/// 1. **6 `MEMBER_OF` del paso-1 (52-57)**: añade `desde_anio`; la 53 (Beto→
///    Neurónica) además `hasta_anio:2024` — la afiliación VENCIDA que explica
///    la mudanza de Beto.
/// 2. **Nodo 69 `:Organizacion` «Instituto GrafoLuna»** y **4 `MEMBER_OF`
///    nuevas (182-185)**: 182 Hugo→UniLira (2019), 183 Iris→GrafosYa (2022),
///    184 Gaby→Neurónica (2023), 185 Beto→GrafoLuna (2024) — todas abiertas.
/// 3. **Gancho reseña**: la `REALIZA` 149 (ronda 1 de Fabio) gana
///    `hasta_anio:2025` (el ciclo de reseñas se cerró) y la `CONTRARRESTA`
///    157 (la ronda 2) gana `desde_anio:2025`.
///
/// Coste real: 8 aristas modificadas (6 + 2), 4 aristas creadas, 1 nodo
/// creado y 8 lecturas (una `get_edge` por arista modificada).
pub fn aplicar_valid_time(store: &mut MemoryStore) -> InformeValidTime {
    let mut informe = InformeValidTime::default();
    let mut lecturas: usize = 0;

    // ── 1) Valid-time de las 6 MEMBER_OF del paso-1 (52-57) ──
    for (id, desde, hasta) in [
        (52usize, 2018i64, None), // Ana → UniLira
        (53, 2018, Some(2024)),   // Beto → Neurónica (VENCIDA: Beto se muda)
        (54, 2020, None),         // Carla → UniLira
        (55, 2021, None),         // Dani → Neurónica (el valor CORREGIDO)
        (56, 2019, None),         // Elena → GrafosYa
        (57, 2019, None),         // Fabio → GrafosYa
    ] {
        poner_validez(store, id, desde, hasta, &mut lecturas);
        informe.aristas_modificadas += 1;
    }

    // ── 2) Nodo 69 :Organizacion «Instituto GrafoLuna» y 4 MEMBER_OF (182-185) ──
    let org = Node::new(GRAPO_LUNA, "Organizacion")
        .with_prop("nombre", Value::String("Instituto GrafoLuna".into()));
    store.put_node(org).unwrap();
    informe.nodos_creados += 1;

    for (i, (persona, organizacion, desde)) in [
        (HUGO, ids::UNI_LIRA, 2019i64),
        (IRIS, ids::GRAFOS_YA, 2022),
        (GABY, ids::NEURONICA, 2023),
        (ids::BETO, GRAPO_LUNA, 2024),
    ]
    .iter()
    .enumerate()
    {
        store
            .put_edge(member_of_desde(
                PRIMER_ID_MEMBER_OF_NUEVA + i,
                *persona,
                *organizacion,
                *desde,
            ))
            .unwrap();
        informe.aristas_creadas += 1;
    }

    // ── 3) Gancho reseña: el ciclo de Fabio también es temporal ──
    // La REALIZA 149 (ronda 1) se cerró en 2025; la CONTRARRESTA 157 (la
    // ronda 2 contrarrestando a la ronda 1) nace en 2025.
    poner_validez(store, 149, 0, Some(2025), &mut lecturas);
    informe.aristas_modificadas += 1;
    poner_validez(store, 157, 2025, None, &mut lecturas);
    informe.aristas_modificadas += 1;

    informe.lecturas = lecturas;
    informe
}

/// Construye KB-Lira **paso-3 (con temporalidad)**: PARTE de
/// [`kb_lira_paso2_degrado`] (lo llama), le aplica [`la_migracion_completa`]
/// (el refactorizado del cap-42) y encima [`aplicar_valid_time`] (este
/// capítulo). Total: 68 nodos, 158 aristas, 10 `MEMBER_OF` con valid-time.
pub fn kb_lira_paso3() -> MemoryStore {
    let mut store = kb_lira_paso2_degrado();
    la_migracion_completa(&mut store);
    aplicar_valid_time(&mut store);
    store
}

#[cfg(test)]
mod tests_temporalidad {
    use super::*;

    /// La estructura de KB-Lira paso-3 es la del refactorizado del cap-42
    /// (67 nodos: 59 + 10 creados − 2 borrados; 154 aristas: 134 + 48 − 28)
    /// MÁS la semilla temporal: el nodo 69 y 4 `MEMBER_OF` nuevas → 68 nodos
    /// y 158 aristas, con 10 `MEMBER_OF` (6 del paso-1 + 4 nuevas).
    #[test]
    fn estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas() {
        let s = kb_lira_paso3();

        assert_eq!(s.node_count(), 68);
        assert_eq!(s.edge_count(), 158);

        assert_eq!(
            s.iter_edges().filter(|e| e.label == "MEMBER_OF").count(),
            10
        );

        let grapo_luna = s.get_node(69).expect("el nodo 69 debe existir");
        assert!(grapo_luna.has_label("Organizacion"));
        assert_eq!(
            grapo_luna.props.get("nombre"),
            Some(&Value::String("Instituto GrafoLuna".into()))
        );
    }

    /// Las 10 `MEMBER_OF` llevan `desde_anio:Int`; el intervalo cerrado solo
    /// donde toca: la 53 (Beto→Neurónica) tiene `hasta_anio:2024` y la 185
    /// (Beto→GrafoLuna) NO tiene `hasta_anio` (abierta). El gancho reseña: la
    /// REALIZA 149 (ronda 1) tiene `hasta_anio:2025` y la CONTRARRESTA 157
    /// `desde_anio:2025`.
    #[test]
    fn las_member_of_llevan_validez_desde_y_hasta_en_anios() {
        let s = kb_lira_paso3();

        let member_of: Vec<&Edge> = s.iter_edges().filter(|e| e.label == "MEMBER_OF").collect();
        assert_eq!(member_of.len(), 10);
        for e in &member_of {
            assert!(
                matches!(e.props.get("desde_anio"), Some(Value::Int(_))),
                "MEMBER_OF {} sin desde_anio:Int",
                e.id
            );
        }

        // Valores fijos por arista (el contrato determinista del capítulo).
        let desde = |id: usize| match s.get_edge(id).unwrap().props.get("desde_anio") {
            Some(Value::Int(a)) => *a,
            otro => panic!("arista {id} sin desde_anio:Int: {otro:?}"),
        };
        assert_eq!(desde(52), 2018);
        assert_eq!(desde(53), 2018);
        assert_eq!(desde(54), 2020);
        assert_eq!(desde(55), 2021); // el valor CORREGIDO del caso bitemporal
        assert_eq!(desde(56), 2019);
        assert_eq!(desde(57), 2019);
        assert_eq!(desde(182), 2019);
        assert_eq!(desde(183), 2022);
        assert_eq!(desde(184), 2023);
        assert_eq!(desde(185), 2024);

        // Intervalo cerrado solo donde toca: la 53 venció en 2024; la 185
        // (la nueva afiliación de Beto) está abierta.
        assert_eq!(
            s.get_edge(53).unwrap().props.get("hasta_anio"),
            Some(&Value::Int(2024))
        );
        assert!(!s.get_edge(185).unwrap().props.contains_key("hasta_anio"));

        // Gancho reseña: la ronda 1 de Fabio se cerró en 2025 y la
        // CONTRARRESTA (ronda 2 → ronda 1) nace en 2025.
        let r1 = s.get_edge(149).unwrap();
        assert_eq!(r1.label, "REALIZA");
        assert_eq!(r1.props.get("hasta_anio"), Some(&Value::Int(2025)));

        let cr = s.get_edge(157).unwrap();
        assert_eq!(cr.label, "CONTRARRESTA");
        assert_eq!(cr.props.get("desde_anio"), Some(&Value::Int(2025)));
    }
}

/// ¿Está vigente una arista en el año `anio` según su valid-time?
///
/// Regla (intervalo medio abierto `[desde, hasta)`):
///
/// - Sin props de validez → vigente SIEMPRE (retrocompatibilidad caps 41-42).
/// - Solo `desde_anio:Int` → vigente si `desde <= anio` (intervalo abierto).
/// - Solo `hasta_anio:Int` → vigente si `anio < hasta`.
/// - Ambas → vigente si `desde <= anio < hasta` (medio abierto).
///
/// Una prop presente que NO sea `Value::Int` se ignora (como si no existiera);
/// un intervalo `[x, x)` es vacío → nunca vigente.
pub fn arista_vigente_en(arista: &Edge, anio: i64) -> bool {
    let desde = match arista.props.get("desde_anio") {
        Some(Value::Int(d)) => Some(*d),
        _ => None,
    };
    let hasta = match arista.props.get("hasta_anio") {
        Some(Value::Int(h)) => Some(*h),
        _ => None,
    };

    match (desde, hasta) {
        (None, None) => true,
        (Some(d), None) => d <= anio,
        (None, Some(h)) => anio < h,
        (Some(d), Some(h)) => d <= anio && anio < h,
    }
}

#[cfg(test)]
mod tests_arista_vigente {
    use super::*;

    /// Helper: arista `MEMBER_OF` a mano con las props de validez indicadas.
    fn member_of(desde: Option<i64>, hasta: Option<i64>) -> Edge {
        let mut e = Edge::new(900, 1, 2, "MEMBER_OF");
        if let Some(d) = desde {
            e.props.insert("desde_anio".into(), Value::Int(d));
        }
        if let Some(h) = hasta {
            e.props.insert("hasta_anio".into(), Value::Int(h));
        }
        e
    }

    /// Los cuatro estados del intervalo temporal: abierta (solo `desde`),
    /// medio abierto (ambas), vencida con `hasta` sin `desde`, y atemporal
    /// (sin props). Incluye el borde del intervalo vacío `[2023, 2023)`.
    #[test]
    fn arista_vigente_en_cubre_abierta_vencida_y_futura() {
        let abierta = member_of(Some(2018), None);
        assert!(arista_vigente_en(&abierta, 2018));
        assert!(arista_vigente_en(&abierta, 2026));
        assert!(arista_vigente_en(&abierta, 2030));

        let cerrada = member_of(Some(2018), Some(2024));
        assert!(arista_vigente_en(&cerrada, 2018));
        assert!(arista_vigente_en(&cerrada, 2023));
        assert!(!arista_vigente_en(&cerrada, 2024));
        assert!(!arista_vigente_en(&cerrada, 2025));

        let vencida = member_of(None, Some(2024));
        assert!(arista_vigente_en(&vencida, 2023));
        assert!(!arista_vigente_en(&vencida, 2024));

        let atemporal = member_of(None, None);
        assert!(arista_vigente_en(&atemporal, 1900));
        assert!(arista_vigente_en(&atemporal, 2026));
        assert!(arista_vigente_en(&atemporal, 2100));

        let vacia = member_of(Some(2023), Some(2023));
        assert!(!arista_vigente_en(&vacia, 2023));
    }
}

// ─────────────────── Consultas AS OF con coste (contrato §2) ───────────────────

/// Coste de una consulta temporal AS OF en LECTURAS al store, por tipo de
/// acceso. Convención de contabilidad heredera del cap-42: cada llamada de
/// lectura incrementa UN contador — `in_edges` por expansión entrante,
/// `get_edge` por cada arista leída y `get_node` por cada nodo. La
/// localización previa del proyecto ([`nodo_por_nombre`], barrido `iter_nodes`)
/// queda FUERA del ledger, igual que en el cap-41: saber QUÉ entidad
/// preguntamos es previo a la consulta.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CosteLecturas {
    /// Llamadas a `in_edges` (la expansión entrante del proyecto).
    pub in_edges: usize,
    /// Llamadas a `get_edge` (una por arista leída, `WORKED_ON` o no).
    pub get_edge: usize,
    /// Llamadas a `get_node` (personas y organizaciones).
    pub get_node: usize,
}

impl CosteLecturas {
    /// Lecturas totales: la suma de los tres contadores.
    pub fn total(&self) -> usize {
        self.in_edges + self.get_edge + self.get_node
    }
}

/// Clave de orden «natural» para la prosa: minúsculas y sin diacríticos.
///
/// Réplica local de la `clave_orden` PRIVADA del cap-41 (las piezas
/// incrementalistas no comparten helpers privados hasta la integración): el
/// `sort` de bytes de Rust pondría «Índices» DESPUÉS de «Z», y la prosa
/// quiere respuestas estables en el orden que un lector espera.
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

/// P4-T: ¿Quién trabaja en el proyecto Y y en qué organización está afiliado
/// EN EL AÑO A (AS OF)?
///
/// La MISMA travesía de 2 saltos que P4 del cap-41 — entrante `WORKED_ON`
/// desde el proyecto + saliente `MEMBER_OF` desde la persona — con UN filtro
/// temporal: solo las `MEMBER_OF` VIGENTES en `anio` ([`arista_vigente_en`])
/// responden. Sobre un grafo sin valid-time el resultado es idéntico a P4
/// (retrocompatibilidad de los caps 41-42, que no ponían validez); con él, la
/// pregunta que el grafo atemporal responde MAL («¿de quién es Beto HOY?» →
/// Neurónica, cuando se fue en 2024) responde la verdad.
///
/// Coste real sobre KB-Lira paso-3 con «Proyecto Kira»: 1 `in_edges` + 21
/// `get_edge` + 6 `get_node` = 28 lecturas (5 aristas entrantes al proyecto,
/// 16 salientes de Ana/Beto/Dani, 3 nodos de persona + 3 de organización).
pub fn afiliaciones_vigentes_en(
    store: &dyn GraphStore,
    proyecto: &str,
    anio: i64,
) -> (Vec<(String, String)>, CosteLecturas) {
    let mut coste = CosteLecturas::default();
    let mut filas: Vec<(String, String)> = Vec::new();

    // 1) Localizar el proyecto por nombre: barrido previo, fuera del ledger.
    let proyecto_id = nodo_por_nombre(store, proyecto).expect("proyecto conocido en KB-Lira");

    // 2) `WORKED_ON` entrantes al proyecto: 1 in_edges + 1 get_edge por
    //    arista candidata (las MENTIONS del hub se leen y se descartan).
    coste.in_edges += 1;
    for eid in store.in_edges(proyecto_id) {
        let e = store.get_edge(eid).expect("arista entrante presente");
        coste.get_edge += 1;
        if e.label != "WORKED_ON" {
            continue;
        }
        let persona = store.get_node(e.source).expect("la persona existe");
        coste.get_node += 1;
        let Some(Value::String(nombre_persona)) = persona.props.get("nombre") else {
            continue;
        };

        // 3) `MEMBER_OF` salientes de la persona, VIGENTES en `anio`. El
        //    segundo salto paga su coste en los get_edge de los candidatos
        //    (la expansión `out_edges` no tiene contador propio).
        for eid_af in store.out_edges(e.source) {
            let af = store.get_edge(eid_af).expect("arista saliente presente");
            coste.get_edge += 1;
            if af.label != "MEMBER_OF" || !arista_vigente_en(af, anio) {
                continue;
            }
            let org = store.get_node(af.target).expect("la organizacion existe");
            coste.get_node += 1;
            let Some(Value::String(nombre_org)) = org.props.get("nombre") else {
                continue;
            };
            filas.push((nombre_persona.clone(), nombre_org.clone()));
        }
    }

    // Orden natural por nombre de persona (mismo criterio que P4 en el cap-41).
    filas.sort_by_key(|a| clave_orden(&a.0));
    (filas, coste)
}

/// Envoltura de [`afiliaciones_vigentes_en`] contra [`ANIO_ACTUAL`]: la
/// versión «¿de quién es cada persona HOY?» — la pregunta que el grafo
/// atemporal responde mal por defecto.
pub fn afiliaciones_actuales(
    store: &dyn GraphStore,
    proyecto: &str,
) -> (Vec<(String, String)>, CosteLecturas) {
    afiliaciones_vigentes_en(store, proyecto, ANIO_ACTUAL)
}

/// Coste del barrido AS OF de las afiliaciones de «Proyecto Kira» en `anio`,
/// sin el ruido de las filas: la consulta real ([`afiliaciones_vigentes_en`])
/// descartando el resultado. Es la unidad de medida para comparar barridos
/// entre años (mismo grafo) y entre grafos (misma pregunta): el proyecto es
/// fijo porque KB-Lira es la KB de ejemplo del capítulo.
pub fn coste_afiliaciones_kb(store: &dyn GraphStore, anio: i64) -> CosteLecturas {
    afiliaciones_vigentes_en(store, "Proyecto Kira", anio).1
}

#[cfg(test)]
mod tests_as_of {
    use super::*;

    /// 2026 (el «hoy» de KB-Lira): Beto ya NO está en Neurónica — su
    /// `MEMBER_OF` 53 venció en 2024 (`hasta_anio:2024`) y la 185 (Beto→
    /// Instituto GrafoLuna, desde 2024) es la vigente. El grafo atemporal del
    /// cap-41 (P4) diría «Instituto Neurónica»; el valid-time dice la verdad.
    #[test]
    fn afiliaciones_actuales_de_kira_responden_beto_en_grafosluna() {
        let s = kb_lira_paso3();
        let (filas, coste) = afiliaciones_actuales(&s, "Proyecto Kira");

        assert_eq!(
            filas,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto GrafoLuna".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
        assert_eq!(coste.in_edges, 1);
        assert_eq!(coste.get_edge, 21);
        assert_eq!(coste.get_node, 6);
        assert_eq!(coste.total(), 28);
    }

    /// 2023: la `MEMBER_OF` 185 (GrafoLuna) aún no existe y la 53 (Beto→
    /// Neurónica, [2018, 2024)) está vigente → Beto responde Neurónica, el
    /// mismo resultado que P4 atemporal (la validez no cambia el pasado).
    #[test]
    fn afiliaciones_as_of_2023_responden_beto_en_neuronica() {
        let s = kb_lira_paso3();
        let (filas, coste) = afiliaciones_vigentes_en(&s, "Proyecto Kira", 2023);

        assert_eq!(
            filas,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
        assert_eq!(coste.total(), 28);
    }

    /// 2019: la afiliación de Dani (55, desde 2021) aún NO ha empezado → Dani
    /// no responde, aunque YA trabaja en Kira (WORKED_ON 60, atemporal). El
    /// costo baja una `get_node`: la organización de Dani no se lee.
    #[test]
    fn afiliaciones_as_of_2019_no_incluyen_a_dani() {
        let s = kb_lira_paso3();
        let (filas, coste) = afiliaciones_vigentes_en(&s, "Proyecto Kira", 2019);

        assert_eq!(
            filas,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
        assert_eq!(coste.in_edges, 1);
        assert_eq!(coste.get_edge, 21);
        assert_eq!(coste.get_node, 5);
        assert_eq!(coste.total(), 27);
    }
}

// ─────────────────────── El coste de la temporalidad (tesis) ───────────────────────

#[cfg(test)]
mod tests_coste_temporalidad {
    use super::*;

    /// Cuántas `MEMBER_OF` candidatas tiene el barrido del proyecto: las
    /// salientes de sus personas que el paso 3 de [`afiliaciones_vigentes_en`]
    /// LEE con `get_edge` (etiquetadas `MEMBER_OF`, vigentes o no — el filtro
    /// temporal llega DESPUÉS de la lectura).
    fn member_of_candidatas(store: &dyn GraphStore) -> usize {
        let pid = nodo_por_nombre(store, "Proyecto Kira").expect("proyecto conocido");
        let personas: Vec<usize> = store
            .in_edges(pid)
            .iter()
            .filter_map(|&eid| {
                let e = store.get_edge(eid)?;
                (e.label == "WORKED_ON").then_some(e.source)
            })
            .collect();
        personas
            .iter()
            .flat_map(|&n| store.out_edges(n))
            .filter(|&eid| store.get_edge(eid).unwrap().label == "MEMBER_OF")
            .count()
    }

    /// La tesis del coste temporal: el filtro de validez se aplica sobre datos
    /// YA leídos, y sin un índice temporal no hay atajo — preguntar por el
    /// «hoy» (2026) o por 2023 barre EXACTAMENTE la misma adyacencia y paga
    /// el mismo precio: 1 `in_edges` + 21 `get_edge` + 6 `get_node` = **28**
    /// lecturas en ambos casos (los valores reales del ledger de la pieza 3).
    #[test]
    fn el_presente_y_el_as_of_cuestan_el_mismo_barrido() {
        let s = kb_lira_paso3();

        let presente = coste_afiliaciones_kb(&s, 2026);
        let as_of = coste_afiliaciones_kb(&s, 2023);

        assert_eq!(presente, as_of);
        assert_eq!(presente.in_edges, 1);
        assert_eq!(presente.get_edge, 21);
        assert_eq!(presente.get_node, 6);
        assert_eq!(presente.total(), 28);
        assert_eq!(as_of.total(), 28);
    }

    /// Una arista VENCIDA no se filtra gratis: sigue en la adyacencia de Beto
    /// y el barrido la lee con `get_edge` ANTES de descartarla. La tesis del
    /// contrato (13→14, o 3→4 contando solo las `MEMBER_OF` candidatas de Ana/
    /// Beto/Dani) medida de la forma coherente con el ledger de la pieza 3:
    /// contra una variante del MISMO paso-3 sin la arista 53 (`delete_edge`),
    /// el PRESENTE es idéntico pero el barrido encoge 1 `get_edge`: **21→20**
    /// (y **28→27** en total), porque la vencida ya no está en la adyacencia.
    /// En candidatas: **4** (Ana 1, Beto 2: la 53 vencida + la 185 vigente,
    /// Dani 1) → **3** (Beto queda con 1 sola) — la historia de Beto cuesta
    /// exactamente 1 lectura extra por arista vencida.
    #[test]
    fn cada_arista_vencida_anade_una_lectura_al_barrido() {
        let s = kb_lira_paso3();
        let mut sin_vencida = kb_lira_paso3();
        assert!(sin_vencida.delete_edge(53));

        let con = coste_afiliaciones_kb(&s, 2026);
        let sin = coste_afiliaciones_kb(&sin_vencida, 2026);

        // El presente es idéntico (Beto sigue en GrafoLuna por la 185)…
        let (filas_con, _) = afiliaciones_vigentes_en(&s, "Proyecto Kira", 2026);
        let (filas_sin, _) = afiliaciones_vigentes_en(&sin_vencida, "Proyecto Kira", 2026);
        assert_eq!(filas_con, filas_sin);

        // …pero el barrido MEMBER_OF encoge en 1 get_edge: la 53 vencida ya
        // no está en la adyacencia de Beto (4 candidatas → 3).
        assert_eq!(member_of_candidatas(&s), 4);
        assert_eq!(member_of_candidatas(&sin_vencida), 3);

        assert_eq!(con.get_edge, 21);
        assert_eq!(sin.get_edge, 20);
        assert_eq!(con.get_node, 6);
        assert_eq!(sin.get_node, 6);
        assert_eq!(con.total(), 28);
        assert_eq!(sin.total(), 27);
    }

    /// BORRAR en vez de CADUCAR: sobre una variante con `delete_edge(53)` el
    /// presente es idéntico (Beto responde GrafoLuna por la 185, la 53 borrada
    /// no se echa de menos) — borrar parece gratis. Pero el AS OF 2023 miente:
    /// Beto DESAPARECE de la respuesta cuando en 2023 sí era de Neurónica.
    /// «Borrar es gratis… hasta que alguien pregunta por el pasado»: el
    /// borrado destruye la historia; el caducado la conserva.
    #[test]
    fn borrar_en_vez_de_caducar_destruye_el_as_of() {
        let s = kb_lira_paso3();
        let mut sin_historia = kb_lira_paso3();
        assert!(sin_historia.delete_edge(53));

        // Presente: idéntico al paso-3 normal — la 185 (GrafoLuna) responde.
        let (actuales, _) = afiliaciones_actuales(&sin_historia, "Proyecto Kira");
        assert_eq!(
            actuales,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto GrafoLuna".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );

        // AS OF 2023: Beto desaparece — su única MEMBER_OF vigente entonces
        // era la 53, y está borrada.
        let (as_of_2023, _) = afiliaciones_vigentes_en(&sin_historia, "Proyecto Kira", 2023);
        assert_eq!(
            as_of_2023,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );

        // El contraste: el paso-3 CON la 53 (vencida, no borrada) responde
        // Beto en 2023; la variante borrada lo ha perdido para siempre.
        let (as_of_con_historia, _) = afiliaciones_vigentes_en(&s, "Proyecto Kira", 2023);
        assert_eq!(
            as_of_con_historia,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
    }
}

// ─────────────────── Gancho cap-42: la reseña vigente (valid-time) ───────────────────

/// La nota de la RESEÑA vigente sobre un documento en el año `anio` — el
/// gancho del cap-42: «¿QUÉ valía el 3 de marzo?», respondido con grano
/// ANUAL y frontera DECLARADA (el contrato dice «cuándo» en años, no en
/// días; la prosa explica la diferencia de grano).
///
/// Travesía (la MISMA navegación que el refactor B del cap-42 sembró):
///
/// 1. Localiza el documento por título ([`nodo_por_nombre`], barrido previo).
/// 2. `SOBRE` ENTRANTES al documento (Resena→Documento, ids 153-156): cada
///    origen es un nodo `:Resena` con `nota:Int` y `ronda:Int`.
/// 3. La VIGENCIA de una reseña NO está en la reseña ni en su `SOBRE`: está
///    en su arista `REALIZA` (Persona→Resena, ids 149-152) — la ronda 1 de
///    Fabio (Resena 65) la lee en la REALIZA 149 (`hasta_anio:2025`, puesta
///    por [`aplicar_valid_time`]), la ronda 2 (Resena 66) en la REALIZA 150
///    (sin validez → abierta). Y si la reseña es la SUCESORA de otra — emite
///    una `CONTRARRESTA` (Resena→Resena, la 157 de la ronda 2) — su reinado
///    como reseña vigente solo EMPIEZA cuando contrarresta: el `desde_anio`
///    de esa CONTRARRESTA (2025) es el límite inferior de su vigencia.
///    Candidata vigente = REALIZA que cumple [`arista_vigente_en`] y, si
///    emite CONTRARRESTA, todas sus CONTRARRESTA también vigentes.
///
/// Regla de resolución (documentada para la prosa):
///
/// - CERO reseñas vigentes → `None` (el documento aún no se ha reseñado o la
///   única reseña caducó y nadie la contrarrestó).
/// - UNA reseña vigente → su `nota:Int`.
/// - VARIAS vigentes → la de MAYOR `ronda:Int` (la última ronda manda: es la
///   valoración vigente del documento; la CONTRARRESTA del cap-42 es la
///   relación que materializa ese reemplazo).
pub fn nota_de_resena_vigente_en(
    store: &dyn GraphStore,
    titulo_documento: &str,
    anio: i64,
) -> Option<i64> {
    let doc_id = nodo_por_nombre(store, titulo_documento)?;

    // 2) Reseñas que SOBRE el documento: las SOBRE entrantes al documento.
    let mut candidatas: Vec<(i64, i64)> = Vec::new(); // (ronda, nota)
    for eid in store.in_edges(doc_id) {
        let sobre = store.get_edge(eid).expect("arista entrante presente");
        if sobre.label != "SOBRE" {
            continue;
        }
        let resena = store.get_node(sobre.source).expect("la resena existe");
        if !resena.has_label("Resena") {
            continue;
        }
        let (Some(Value::Int(nota)), Some(Value::Int(ronda))) =
            (resena.props.get("nota"), resena.props.get("ronda"))
        else {
            continue;
        };

        // 3) La vigencia de la reseña vive en su REALIZA (Persona→Resena):
        //    entrante al nodo Resena, como la sembró el refactor B del cap-42.
        //    Y si la reseña emite CONTRARRESTA (es la sucesora), solo reina
        //    desde el desde_anio de esa CONTRARRESTA (ronda 2 → desde 2025).
        let vigente = store
            .in_edges(sobre.source)
            .iter()
            .filter_map(|&rid| store.get_edge(rid))
            .any(|realiza| realiza.label == "REALIZA" && arista_vigente_en(realiza, anio))
            && store
                .out_edges(sobre.source)
                .iter()
                .filter_map(|&cid| store.get_edge(cid))
                .filter(|e| e.label == "CONTRARRESTA")
                .all(|cr| arista_vigente_en(cr, anio));
        if vigente {
            candidatas.push((*ronda, *nota));
        }
    }

    match candidatas.len() {
        0 => None,
        // Una sola vigente: su nota. Varias: la de mayor ronda (la última
        // ronda reemplaza a las anteriores — la CONTRARRESTA lo materializa).
        _ => candidatas
            .into_iter()
            .max_by_key(|&(ronda, _)| ronda)
            .map(|(_, nota)| nota),
    }
}

#[cfg(test)]
mod tests_resena_vigente {
    use super::*;

    /// El gancho del cap-42 respondido con grano anual: la ronda 1 de Fabio
    /// (nota 7) es la vigente hasta que su REALIZA 149 caduca en 2025; la
    /// ronda 2 (nota 8, REALIZA 150 abierta) la contrarresta desde entonces.
    ///
    /// NOTA de grano: el contrato pregunta «el 3 de marzo de 2025» — con
    /// grano FINO la respuesta sería la nota 7 (la ronda 1 aún no había
    /// caducado ese día). Con el grano ANUAL declarado de esta pieza, 2025
    /// responde 8: la frontera de caducidad es el AÑO 2025 y la frontera se
    /// evalúa de forma medio abierta `[desde, hasta)`.
    #[test]
    fn la_ronda_1_de_fabio_caduco_cuando_la_ronda_2_la_contrarresto() {
        let s = kb_lira_paso3();
        let titulo_informe = "Informe de revisión por pares 2025";

        assert_eq!(nota_de_resena_vigente_en(&s, titulo_informe, 2024), Some(7));
        assert_eq!(nota_de_resena_vigente_en(&s, titulo_informe, 2025), Some(8));
        assert_eq!(nota_de_resena_vigente_en(&s, titulo_informe, 2026), Some(8));
    }
}

// ─────────────── Pieza: histórico bitemporal de afiliaciones ───────────────

/// Una fila del histórico de afiliaciones: el REGISTRO de «quién se afilió a
/// qué y desde cuándo» en el momento en que se anotó (transaction-time).
///
/// La analogía del contrato: este es el WAL del modelo — append-only, cada
/// `registrar` gana un `ts_registro` monótono y NUNCA se reescribe. La arista
/// del grafo (valid-time, el «qué es cierto») y esta entrada (transaction-time,
/// el «qué se creía») son dos hechos distintos que la corrección del caso Dani
/// hace explícitos: ts 1 dice «desde 2019» (lo que se creía), ts 2 dice «desde
/// 2021» (lo cierto — la arista 55 del cap-43).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaHistoria {
    /// `ts_registro` asignado por el propio histórico (monótono: 1, 2, …).
    pub ts_registro: u64,
    /// Persona de la afiliación (id del nodo `:Person` del cap-41).
    pub persona: usize,
    /// Organización a la que se afilió (id del nodo `:Organizacion` del cap-41).
    pub organizacion: usize,
    /// `desde_anio` que se REGISTRÓ en ese momento (puede no ser lo cierto).
    pub desde_anio: i64,
}

/// El histórico append-only de afiliaciones (transaction-time): el «WAL del
/// modelo». Solo se pueden AÑADIR entradas; el `ts_registro` lo asigna el
/// propio histórico, nunca el llamador.
#[derive(Debug, Clone)]
pub struct HistoricoAfiliaciones {
    entradas: Vec<EntradaHistoria>,
    siguiente_ts: u64,
}

impl HistoricoAfiliaciones {
    /// Histórico vacío, listo para registrar desde el ts 1.
    pub fn nueva() -> Self {
        HistoricoAfiliaciones {
            entradas: Vec::new(),
            siguiente_ts: 1,
        }
    }

    /// Registra una afiliación: asigna el siguiente ts monótono (append-only,
    /// el ts lo decide el histórico, NUNCA el llamador) y lo devuelve.
    pub fn registrar(&mut self, persona: usize, organizacion: usize, desde_anio: i64) -> u64 {
        let ts = self.siguiente_ts;
        self.siguiente_ts += 1;
        self.entradas.push(EntradaHistoria {
            ts_registro: ts,
            persona,
            organizacion,
            desde_anio,
        });
        ts
    }

    /// «¿Qué creíamos en el año `anio_valid` según el registro hasta el
    /// `ts_registro`?»: la ÚLTIMA entrada de esa persona con
    /// `ts_registro <= ts_registro` — la corrección manda.
    ///
    /// NOTA: `anio_valid` NO filtra aquí — el histórico responde por registro
    /// (transaction-time); el valid-time real vive en la arista del grafo
    /// (`desde_anio`/`hasta_anio` del cap-43). Se mantiene en la firma porque
    /// la pregunta del contrato llega con año y las piezas posteriores
    /// (cap-44/45) la convierten en un predicado real.
    pub fn afiliacion_segun_registro(
        &self,
        persona: usize,
        anio_valid: i64,
        ts_registro: u64,
    ) -> Option<(usize, i64)> {
        let _ = anio_valid;
        self.entradas
            .iter()
            .filter(|e| e.persona == persona && e.ts_registro <= ts_registro)
            .max_by_key(|e| e.ts_registro)
            .map(|e| (e.organizacion, e.desde_anio))
    }

    /// Las entradas en orden de registro: para tests y exportación a CSV.
    pub fn entradas(&self) -> &[EntradaHistoria] {
        &self.entradas
    }
}

/// El histórico del caso CONCRETO de KB-Lira (paso-3): determinista, los ts
/// son 1, 2, … en orden de registro.
///
/// Caso Dani (ids del cap-41: `ids::DANI` = 3, `ids::NEURONICA` = 7):
/// - ts 1: Dani → Neurónica desde 2019 — LO QUE SE CREÍA en 2023.
/// - ts 2: Dani → Neurónica desde 2021 — LA CORRECCIÓN (lo cierto; coincide
///   con la arista 55 del cap-43, `desde_anio:2021`).
///
/// Además una entrada de otra persona para que el histórico no sea trivial
/// (Ana → UniLira desde 2018, la arista 52 del cap-43).
pub fn historico_kb_lira_paso3() -> HistoricoAfiliaciones {
    let mut h = HistoricoAfiliaciones::nueva();
    h.registrar(ids::DANI, ids::NEURONICA, 2019);
    h.registrar(ids::DANI, ids::NEURONICA, 2021);
    h.registrar(ids::ANA, ids::UNI_LIRA, 2018);
    h
}

#[cfg(test)]
mod tests_historico_afiliaciones {
    use super::*;

    /// El caso de Dani registrado: 2+ entradas, la última de Dani dice 2021
    /// (la corrección manda) y los ts son 1, 2, … monótonos.
    #[test]
    fn historico_afiliaciones_registra_el_caso_de_dani() {
        let h = historico_kb_lira_paso3();
        let entradas = h.entradas();

        assert!(
            entradas.len() >= 2,
            "el histórico debe registrar el caso de Dani"
        );

        let ultima_de_dani = entradas
            .iter()
            .rev()
            .find(|e| e.persona == ids::DANI)
            .expect("Dani registrado");
        assert_eq!(ultima_de_dani.desde_anio, 2021);

        for (i, e) in entradas.iter().enumerate() {
            assert_eq!(e.ts_registro, (i as u64) + 1, "ts monótonos 1, 2, …");
        }
    }

    /// «¿Qué creíamos?» vs «¿qué es cierto?»: el registro responde por ts —
    /// con ts 1 (el registro de 2023) cree que Dani está desde 2019; tras la
    /// corrección (ts 2) responde 2021. `anio_valid` (2024) no cambia nada:
    /// el histórico responde por REGISTRO, no por el año de la pregunta — el
    /// valid-time real está en la arista 55 del grafo.
    #[test]
    fn afiliacion_segun_registro_distingue_lo_creido_de_lo_cierto() {
        let h = historico_kb_lira_paso3();

        assert_eq!(
            h.afiliacion_segun_registro(ids::DANI, 2024, 1),
            Some((ids::NEURONICA, 2019)),
            "ts 1: lo que se creía en 2023"
        );
        assert_eq!(
            h.afiliacion_segun_registro(ids::DANI, 2024, 2),
            Some((ids::NEURONICA, 2021)),
            "ts 2: tras la corrección — lo cierto"
        );
    }
}

// ─────────────────── El WAL del cap-28 como transaction-time ───────────────────

#[cfg(test)]
mod tests_wal_transaction_time {
    use super::*;
    use crate::cap28_wal::{CuerpoWal, Wal, WalTransaccion, replay_wal};

    /// LA CONEXIÓN CONCEPTUAL: el WAL del cap-28 ES el transaction-time del
    /// ESTADO, y el [`HistoricoAfiliaciones`] es el MISMO patrón aplicado al
    /// CONOCIMIENTO.
    ///
    /// - El `WalTransaccion` del cap-28 (write-AHEAD real) hace durable la
    ///   arista de Dani ANTES de aplicarla; el histórico registra el MISMO
    ///   hecho con un `ts_registro` que asigna él, como el WAL asigna el LSN.
    /// - El corte de luz se simula con la API real del cap-28: `as_bytes`
    ///   (lo único que sobrevive), `Wal::reconstruir` (reabrir el log al
    ///   despertar) y `replay_wal` (el redo en orden de LSN).
    #[test]
    fn el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time() {
        // ── 1) El hecho se escribe DURANTE un commit real del cap-28 ──
        // Una única transacción WAL con TODO el cambio de estado: los dos
        // nodos y la MEMBER_OF Dani→Neurónica desde 2019 (la arista 55 del
        // libro, aquí con el valor SIN corregir: 2019).
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(
                Node::new(ids::DANI, "Person").with_prop("nombre", Value::String("Dani".into())),
            )
            .unwrap();
            tx.put_node(
                Node::new(ids::NEURONICA, "Organizacion")
                    .with_prop("nombre", Value::String("Neurónica".into())),
            )
            .unwrap();
            tx.put_edge(
                Edge::new(55, ids::DANI, ids::NEURONICA, "MEMBER_OF")
                    .with_prop("desde_anio", Value::Int(2019)),
            )
            .unwrap();
            tx.commit().unwrap(); // Begin + 3 ops + Commit = 5 registros
        }

        // El MISMO hecho en el «WAL del modelo»: el histórico append-only,
        // con su ts_registro asignado por él (ts 1).
        let mut historico = HistoricoAfiliaciones::nueva();
        historico.registrar(ids::DANI, ids::NEURONICA, 2019);

        // ── 2) Corte de luz: el store muere; el WAL (bytes) y el histórico viven ──
        let bytes = wal.as_bytes(); // ← lo único que sobrevive al corte
        let wal_cargado = Wal::reconstruir(bytes); // reabrir el log al despertar
        let mut renacido = MemoryStore::new();
        let informe = replay_wal(&mut renacido, &wal_cargado).unwrap();

        // El ESTADO reconstruido responde el transaction-time del hecho: la
        // afiliación sobrevive al corte.
        assert_eq!(informe.transacciones_confirmadas, 1);
        assert_eq!(informe.operaciones_reaplicadas, 3);
        let arista = renacido
            .get_edge(55)
            .expect("la MEMBER_OF de Dani sobrevive al corte");
        assert_eq!(arista.source, ids::DANI);
        assert_eq!(arista.target, ids::NEURONICA);
        assert_eq!(arista.props.get("desde_anio"), Some(&Value::Int(2019)));

        // El CONOCIMIENTO reconstruido responde lo mismo: ts 1 = el registro
        // de 2019 (con el año de referencia del «hoy» del capítulo).
        assert_eq!(
            historico.afiliacion_segun_registro(ids::DANI, ANIO_ACTUAL, 1),
            Some((ids::NEURONICA, 2019))
        );

        // ── 3) LA FRONTERA ──
        // La corrección (2021) solo entra en el HISTÓRICO: el modelo aprende
        // que lo cierto es 2021. El WAL del cap-28 NO vuelve a escribir nada:
        // solo rehace el estado final de lo ESCRITO, no la historia de los
        // valores corregidos.
        let ts_correccion = historico.registrar(ids::DANI, ids::NEURONICA, 2021);
        assert_eq!(ts_correccion, 2);

        // El histórico (que SÍ registró la corrección) responde 2021…
        assert_eq!(
            historico.afiliacion_segun_registro(ids::DANI, ANIO_ACTUAL, 2),
            Some((ids::NEURONICA, 2021))
        );

        // …pero el WAL, con los MISMOS bytes de antes del corte, sigue
        // reconstruyendo la versión 2019: «el WAL sabe lo que se escribió,
        // no lo que se corrigió».
        let mut otro_renacido = MemoryStore::new();
        replay_wal(&mut otro_renacido, &wal_cargado).unwrap();
        assert_eq!(
            otro_renacido
                .get_edge(55)
                .expect("la arista 55 reconstruida")
                .props
                .get("desde_anio"),
            Some(&Value::Int(2019)),
            "la corrección 2021 jamás pasó por el WAL: el redo solo rehace lo ESCRITO"
        );

        // ── 4) La equivalencia declarada (assert de concepto) ──
        // LSN ≡ ts_registro: ambos los asigna el log/histórico (monótonos),
        // nunca el llamador — el primer LSN es 1 y el primer ts es 1.
        assert_eq!(wal.iter().next().expect("primer registro").lsn, 1);
        assert_eq!(historico.entradas()[0].ts_registro, 1);
        // Commit ≡ entrada del histórico: ambos son el punto de durabilidad —
        // 1 commit en el WAL y 1 entrada del histórico antes de la corrección.
        let commits = wal
            .iter()
            .filter(|r| matches!(r.cuerpo, CuerpoWal::Commit))
            .count();
        assert_eq!(commits, 1);
        assert_eq!(wal.record_count(), 5); // Begin + 3 ops + Commit
        // replay ≡ re-lectura en orden: rehacer es volver a LEER el log en
        // orden de LSN (el histórico se re-lee igual: max_by_key por ts).
        assert_eq!(renacido.edge_count(), 1);
    }
}

// ─────────────────── Validador paso-3: reglas temporales (contrato) ───────────────────

/// Valida el contrato del modelo KB-Lira **paso-3** sobre `store`:
///
/// - REUTILIZA [`validar_modelo_kb_lira_paso2`] (cap-42) tal cual: el subgrafo
///   refactorizado debe seguir cumpliendo su contrato. Cualquier violación del
///   paso-2 se PROPAGA sin filtrar — el paso-3 parte del modelo refactorizado
///   que el paso-2 ya aceptaba, y añadir props de validez no cambia extremos,
///   tipos ni campos obligatorios.
/// - Reglas nuevas (SOLO sobre `MEMBER_OF` — el valid-time del capítulo):
///   - `desde_anio:Int` REQUERIDO: toda afiliación declara cuándo empezó.
///   - `hasta_anio` si presente: `Int` y `hasta_anio >= desde_anio` (un
///     intervalo cuyo fin es anterior a su inicio no es un intervalo válido).
///   - `desde_anio <= ANIO_ACTUAL`: una afiliación que empieza en el futuro
///     no puede existir hoy (no validez futura).
///
/// Devuelve `Ok(())` si no hay violaciones o la lista completa (las del
/// paso-2 propagadas + las temporales), para reportarlas todas de una pasada.
pub fn validar_modelo_kb_lira_paso3(store: &dyn GraphStore) -> Result<(), Vec<Violacion>> {
    let mut malas: Vec<Violacion> = Vec::new();

    // ── 1) Contrato del paso-2, propagado tal cual ──
    if let Err(paso2) = validar_modelo_kb_lira_paso2(store) {
        malas.extend(paso2);
    }

    // ── 2) Reglas temporales nuevas: SOLO sobre MEMBER_OF ──
    for ar in store.iter_edges() {
        if ar.label != "MEMBER_OF" {
            continue;
        }
        let desde = match ar.props.get("desde_anio") {
            Some(Value::Int(d)) => *d,
            _ => {
                malas.push(Violacion {
                    descripcion: "MEMBER_OF sin desde_anio:Int".into(),
                    id_implicado: ar.id,
                    tipo_elemento: "arista",
                });
                continue;
            }
        };
        if desde > ANIO_ACTUAL {
            malas.push(Violacion {
                descripcion: format!(
                    "MEMBER_OF con desde_anio {desde} posterior a ANIO_ACTUAL ({ANIO_ACTUAL})"
                ),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            });
        }
        match ar.props.get("hasta_anio") {
            Some(Value::Int(h)) if *h < desde => malas.push(Violacion {
                descripcion: format!("MEMBER_OF con hasta_anio {h} anterior a desde_anio {desde}"),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            }),
            Some(Value::Int(_)) => {}
            Some(otro) => malas.push(Violacion {
                descripcion: format!("MEMBER_OF con hasta_anio no-Int: {otro:?}"),
                id_implicado: ar.id,
                tipo_elemento: "arista",
            }),
            None => {}
        }
    }

    if malas.is_empty() { Ok(()) } else { Err(malas) }
}

#[cfg(test)]
mod tests_validador_paso3 {
    use super::*;

    /// La cadena completa del paso-3 (refactors del cap-42 + valid-time) deja
    /// el grafo en un estado que CUMPLE el contrato del paso-3: el paso-2
    /// acepta el modelo refactorizado (las props de validez no cambian
    /// extremos ni campos obligatorios) y las 10 `MEMBER_OF` declaran
    /// `desde_anio:Int` en el pasado.
    #[test]
    fn validador_paso3_acepta_el_modelo_temporal() {
        let s = kb_lira_paso3();

        assert_eq!(
            validar_modelo_kb_lira_paso3(&s),
            Ok(()),
            "el modelo temporal cumple el contrato del paso-3"
        );
    }

    /// Tres corrupciones a mano sobre el MemoryStore (precedente cap-33/cap-42:
    /// mutación directa del campo `pub edges`): (a) la MEMBER_OF 52 sin
    /// `desde_anio`, (b) la 54 con `hasta_anio < desde_anio` (2020 → 2019) y
    /// (c) la 56 con `desde_anio` en el futuro (2030 > ANIO_ACTUAL). El
    /// validador falla con las violaciones CONCRETAS: ids 52, 54 y 56.
    #[test]
    fn validador_paso3_rechaza_fixture_sin_validez() {
        let mut s = kb_lira_paso3();

        // (a) MEMBER_OF 52 (Ana→UniLira) sin desde_anio:Int.
        s.edges[52]
            .as_mut()
            .expect("MEMBER_OF 52 presente")
            .props
            .remove("desde_anio");

        // (b) MEMBER_OF 54 (Carla→UniLira) con intervalo invertido: hasta 2019
        //     anterior a desde 2020.
        {
            let e54 = s.edges[54].as_mut().expect("MEMBER_OF 54 presente");
            e54.props.insert("desde_anio".into(), Value::Int(2020));
            e54.props.insert("hasta_anio".into(), Value::Int(2019));
        }

        // (c) MEMBER_OF 56 (Elena→GrafosYa) con validez futura: desde 2030.
        s.edges[56]
            .as_mut()
            .expect("MEMBER_OF 56 presente")
            .props
            .insert("desde_anio".into(), Value::Int(2030));

        let err = validar_modelo_kb_lira_paso3(&s).unwrap_err();
        assert_eq!(err.len(), 3, "solo las tres MEMBER_OF corruptas: {err:?}");
        let mut ids: Vec<usize> = err.iter().map(|v| v.id_implicado).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![52, 54, 56]);
        assert!(
            err.iter().all(|v| v.tipo_elemento == "arista"),
            "todas son violaciones de aristas: {err:?}"
        );
        let por_id = |id: usize| {
            err.iter()
                .find(|v| v.id_implicado == id)
                .expect("violación presente")
        };
        assert!(
            por_id(52).descripcion.contains("desde_anio"),
            "{}",
            por_id(52).descripcion
        );
        assert!(
            por_id(54).descripcion.contains("hasta_anio"),
            "{}",
            por_id(54).descripcion
        );
        assert!(
            por_id(56).descripcion.contains("ANIO_ACTUAL"),
            "{}",
            por_id(56).descripcion
        );
    }
}

// ─────────────────── Red de seguridad triple: el valid-time no cambia NINGUNA respuesta vieja ───────────────────

/// Regresión de [`aplicar_valid_time`] (cap-43) sobre las respuestas del
/// cap-41 (paso-1) y del cap-42 (paso-2 refactorizado): añadir la DIMENSIÓN
/// TIEMPO a las afiliaciones NO puede cambiar ninguna respuesta vieja.
///
/// Mecanismo de comparación IDÉNTICO al del cap-42
/// (`tests_regresion_preguntas_paso1::las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor`):
/// cada pregunta se ejecuta sobre el store nuevo (paso-3) y su respuesta se
/// FILTRA al subgrafo paso-1 resolviendo el id del nodo (`titulo`/`nombre`)
/// en el store — las filas del lote/valid-time no participan en el contrato
/// de esta pieza. En P8 (conteos SIN ids) el subgrafo se recalcula contando
/// solo las `AUTHORED` a documentos con id < 30.
#[cfg(test)]
mod tests_regresion_temporalidad {
    use super::*;
    use crate::cap41_modelado::kb_lira_paso1;
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
    use std::collections::HashMap;

    /// Primer id del lote: el subgrafo paso-1 ocupa los ids 0..30.
    const LOTE_INICIO: usize = 30;

    /// ¿El valor `campo` (`titulo`/`nombre`) pertenece a un nodo del subgrafo
    /// paso-1 (id < 30) en `store`? Copia del helper del cap-42.
    fn es_paso1(store: &dyn GraphStore, campo: &str, valor: &str) -> bool {
        store.iter_nodes().any(|n| {
            n.id < LOTE_INICIO && n.props.get(campo) == Some(&Value::String(valor.to_string()))
        })
    }

    /// P8 restringida al subgrafo paso-1: `pregunta_08` devuelve (nombre,
    /// conteo) SIN ids, y el lote añade AUTHORED desde personas del paso-1
    /// (Fabio→51-53, Elena→54-56) — recontar solo las AUTHORED a documentos
    /// con id < 30 es la única forma de comparar el subgrafo paso-1 sin
    /// contaminación del lote. Copia del helper del cap-42.
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

    /// P4 restringida al subgrafo paso-1: la pregunta devuelve (persona,
    /// organización) SIN ids, y el valid-time añade la `MEMBER_OF` 185
    /// Beto→Instituto GrafoLuna (nodo 69) — resolver el id de la
    /// organización en el store y quedarse con las del subgrafo paso-1
    /// (id < 30) es la única forma de comparar la afiliación del paso-1 sin
    /// contaminación temporal.
    fn solo_afiliaciones_paso1(
        store: &dyn GraphStore,
        afiliaciones: &[(String, String)],
    ) -> Vec<(String, String)> {
        afiliaciones
            .iter()
            .filter(|(_, org)| es_paso1(store, "nombre", org))
            .cloned()
            .collect()
    }

    /// LA red de seguridad del capítulo: las 10 preguntas del cap-41
    /// responden IDÉNTICO sobre el subgrafo paso-1 (ids 0-29) DESPUÉS de
    /// añadir el valid-time. El tiempo solo ENRIQUECE el grafo con las
    /// afiliaciones nuevas (ids 182-185): esas filas no participan en el
    /// contrato de esta pieza y se filtran resolviendo el id del nodo
    /// (titulo/nombre) en el store paso-3.
    ///
    /// La excepción que ES la lección (P4): la afiliación atemporal sigue
    /// devolviendo Beto→Neurónica sobre el subgrafo paso-1 — su contrato no
    /// cambia, la MEMBER_OF 53 sigue ahí — mientras la lectura CON tiempo del
    /// paso-3 completo devuelve Beto→GrafoLuna (la 185, su mudanza en 2024).
    /// Esa fila se FILTRA en esta regresión: la diferencia entre lo que el
    /// modelo atemporal callado responde y lo que el tiempo revela es la
    /// lección del capítulo, no una regresión.
    #[test]
    fn las_10_preguntas_del_paso1_no_cambian_tras_anadir_valid_time() {
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

        // ── Store paso-3: el refactor del cap-42 + el valid-time de este capítulo ──
        let paso3 = kb_lira_paso3();

        // ── Las mismas 10 preguntas sobre el store temporal ──
        let a1 = q01(&paso3, "Ana");
        let a2 = q02(&paso3, "Memoria episódica en LLMs");
        let (a3a, a3b) = q03(&paso3, "Supernodos: anatomía de un cuello de botella");
        let a4 = q04(&paso3, "Proyecto Kira");
        let (a5a, a5b) = q05(
            &paso3,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        let a6 = q06(&paso3, "Instituto Neurónica");
        let a7 = q07(&paso3, "Ana", "Beto");
        let a9 = q09(&paso3, "Ana", "Beto");
        let a10 = q10(
            &paso3,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        );

        // ── Comparación SOLO sobre el subgrafo paso-1 (ids < 30) ──
        let solo_titulos = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&paso3, "titulo", t))
                .cloned()
                .collect()
        };
        let solo_nombres = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&paso3, "nombre", t))
                .cloned()
                .collect()
        };
        let solo_paso1_citas = |v: &[(String, i64)]| -> Vec<(String, i64)> {
            v.iter()
                .filter(|(t, _)| es_paso1(&paso3, "titulo", t))
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

        // P4: la afiliación atemporal del subgrafo paso-1 sigue siendo
        // Beto→Neurónica (la MEMBER_OF 53 no cambió; la 185 Beto→GrafoLuna
        // es la lectura CON tiempo — la lección, no una regresión).
        assert_eq!(
            solo_afiliaciones_paso1(&paso3, &a4),
            r4,
            "P04 proyecto_y_afiliaciones_en_dos_saltos(Proyecto Kira): el subgrafo paso-1 no \
             cambia — la fila Beto→GrafoLuna (185) se filtra por id 69 (obtenido: {a4:?})"
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
            publicaciones_por_persona_en_paso1(&paso3),
            publicaciones_por_persona_en_paso1(&paso1),
            "P08 publicaciones_por_persona_contadas: los conteos del subgrafo paso-1 no cambian"
        );

        assert_eq!(
            solo_nombres(&a9),
            solo_nombres(&r9),
            "P09 temas_comunes_via_papers(Ana,Beto): el subgrafo paso-1 no cambia (obtenido: {a9:?})"
        );

        assert_eq!(
            solo_paso1_citas(&a10),
            solo_paso1_citas(&r10),
            "P10 citas_recientes_que_tratan_un_tema(Grafos…, grafos de conocimiento, >2023): el \
             subgrafo paso-1 no cambia (obtenido: {a10:?})"
        );
    }

    /// Los valores pineados del contrato cap-42 (`tests_preguntas_paso2`)
    /// ejecutados sobre el paso-3 ENTERO (sin filtros): el valid-time no
    /// toca AUTHORED/CITES/ABOUT/MENTIONS, así que las respuestas del
    /// cap-42 se mantienen EXACTAS sobre el modelo temporal.
    #[test]
    fn las_respuestas_del_paso2_no_cambian_tras_anadir_valid_time() {
        let store = kb_lira_paso3();

        // P1: los 4 documentos de Ana — Ana no publicó en el lote.
        assert_eq!(
            q01(&store, "Ana"),
            vec![
                "Grafos de conocimiento para agentes",
                "Índices adaptativos para grafos",
                "Notas de la reunión de arranque",
                "Supernodos: anatomía de un cuello de botella",
            ],
            "P1(Ana): idéntico al paso-2 — el valid-time no toca AUTHORED"
        );

        // P3: las citas del paper del paso-1 «Supernodos…».
        let (sale, entra) = q03(&store, "Supernodos: anatomía de un cuello de botella");
        assert_eq!(
            sale,
            vec![
                "Consultas declarativas sobre property graphs",
                "Índices adaptativos para grafos",
            ],
            "P3a(Supernodos…): salientes idénticas al paso-2"
        );
        assert_eq!(
            entra,
            vec!["Recuperación aumentada con grafos"],
            "P3a(Supernodos…): entrantes idénticas al paso-2"
        );

        // P5: la unión jerárquica del tema 24 sigue recuperando los 24 docs
        // (6 paso-1 + 18 del lote) — el valid-time no toca ABOUT/SUB_TEMA_DE.
        let ids_jerarquia =
            documentos_del_tema_incluyendo_subtemas(&store, ids::TEMA_GRAFOS_CONOCIMIENTO);
        assert_eq!(
            ids_jerarquia.len(),
            24,
            "la unión jerárquica devuelve los 24 docs (obtenido: {ids_jerarquia:?})"
        );

        // P6: las 5 menciones a Neurónica — idénticas al paso-2.
        assert_eq!(
            q06(&store, "Instituto Neurónica"),
            vec![
                "Bitácora del experimento K-7",
                "Informe anual del Proyecto Kira",
                "Informe de revisión por pares 2025",
                "Informe técnico del Proyecto Oráculo",
                "Memoria episódica en LLMs",
            ],
            "P6(Instituto Neurónica): idéntico al paso-2 — el lote NO añade MENTIONS"
        );

        // P8: los 40 AUTHORED repartidos entre las 9 personas — idéntico al
        // paso-2 (16 del paso-1 + 24 del lote).
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
    }

    /// El validador del paso-2 acepta el modelo paso-3: añadir props de
    /// validez a las `MEMBER_OF` (y las 4 afiliaciones nuevas) no rompe
    /// NINGUNA regla del contrato cap-42 — el paso-3 parte del modelo
    /// refactorizado que el paso-2 ya aceptaba.
    #[test]
    fn validador_paso2_acepta_el_modelo_paso3() {
        assert_eq!(
            validar_modelo_kb_lira_paso2(&kb_lira_paso3()),
            Ok(()),
            "el modelo temporal cumple el contrato del paso-2"
        );
    }
}

// ─────────────────── CSV determinista (mismo contrato que cap. 41/42) ───────────────────

/// Exporta los NODOS del paso-3 (con temporalidad) al formato CSV del cap. 32
/// (cabecera = unión BTreeMap de props; `:LABEL` con labels unidas por `:`).
/// Es el MISMO formato que [`crate::cap41_modelado::csv_nodos_kb_lira`] y
/// [`crate::cap42_antipatrones::csv_nodos_kb_lira_paso2`]: el dataset es lo
/// que «importó el equipo», la temporalidad es código.
pub fn csv_nodos_kb_lira_paso3(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_nodos(store, &mut buf).expect("export nodos paso-3");
    String::from_utf8(buf).expect("CSV UTF-8")
}

/// Exporta las ARISTAS del paso-3 (mismo contrato que
/// [`csv_nodos_kb_lira_paso3`]). Las `MEMBER_OF` llevan `desde_anio:INT` y,
/// donde el intervalo está cerrado, `hasta_anio:INT` — ausencia = abierto.
pub fn csv_aristas_kb_lira_paso3(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_aristas(store, &mut buf).expect("export aristas paso-3");
    String::from_utf8(buf).expect("CSV UTF-8")
}

// ─────────────────── CSV del histórico de afiliaciones (round-trip) ───────────────────

/// Exporta el [`HistoricoAfiliaciones`] a CSV: cabecera
/// `ts_registro,persona,organizacion,desde_anio` + una línea por entrada en
/// orden de registro. Formato propio mínimo (todos los campos son enteros:
/// sin comillas, determinista).
pub fn csv_historico(historico: &HistoricoAfiliaciones) -> String {
    let mut buf = String::from("ts_registro,persona,organizacion,desde_anio\n");
    for e in historico.entradas() {
        buf.push_str(&format!(
            "{},{},{},{}\n",
            e.ts_registro, e.persona, e.organizacion, e.desde_anio
        ));
    }
    buf
}

/// Importa un CSV producido por [`csv_historico`] (round-trip): reconstruye
/// el histórico con los `ts_registro` ORIGINALES (append-only, nunca se
/// reasignan) y `siguiente_ts = máximo + 1`, de modo que el siguiente
/// `registrar` continúa la numeración sin colisionar.
pub fn historico_desde_csv(contenido: &str) -> HistoricoAfiliaciones {
    let mut lineas = contenido.lines();
    let cabecera = lineas.next().expect("el CSV debe empezar por la cabecera");
    assert_eq!(
        cabecera, "ts_registro,persona,organizacion,desde_anio",
        "cabecera inesperada: {cabecera:?}"
    );

    let mut entradas: Vec<EntradaHistoria> = Vec::new();
    let mut siguiente_ts: u64 = 1;
    for (i, linea) in lineas.enumerate() {
        let recortada = linea.trim_end_matches('\r');
        if recortada.is_empty() {
            continue; // línea en blanco: se salta
        }
        let campos: Vec<&str> = recortada.split(',').collect();
        assert_eq!(
            campos.len(),
            4,
            "fila {} malformada (se esperaban 4 campos): {recortada:?}",
            i + 2
        );
        let ts: u64 = campos[0].parse().expect("ts_registro entero");
        let persona: usize = campos[1].parse().expect("persona entera");
        let organizacion: usize = campos[2].parse().expect("organizacion entera");
        let desde_anio: i64 = campos[3].parse().expect("desde_anio entero");
        entradas.push(EntradaHistoria {
            ts_registro: ts,
            persona,
            organizacion,
            desde_anio,
        });
        siguiente_ts = siguiente_ts.max(ts + 1);
    }

    HistoricoAfiliaciones {
        entradas,
        siguiente_ts,
    }
}

// ─────────────────── Los tests de honestidad (pieza incremental) ───────────────────

#[cfg(test)]
mod tests_csv_paso3 {
    use super::*;
    use crate::cap32_import_export::{importar_csv_aristas, importar_csv_nodos};
    use std::io::BufReader;

    // Los tests llevan el nombre EXACTO del contrato; para llamar a las
    // funciones homónimas de cap-41/cap-42 sin sombra, se re-importan con
    // alias (mismo patrón que los capítulos anteriores).
    use crate::cap41_modelado::{
        csv_aristas_kb_lira as csv_aristas_paso1, csv_nodos_kb_lira as csv_nodos_paso1,
        kb_lira_paso1,
    };
    use crate::cap42_antipatrones::{
        csv_aristas_kb_lira_paso2, csv_nodos_kb_lira_paso2, kb_lira_paso2_degrado,
    };

    /// Exporta nodos+aristas del paso-3 → importa (cap. 32) → exporta de
    /// nuevo: bytes IDÉNTICOS (mismo patrón que el roundtrip del cap-41/42).
    #[test]
    fn csv_roundtrip_paso3_import_export_byte_a_byte() {
        let s = kb_lira_paso3();
        let nodos_v1 = csv_nodos_kb_lira_paso3(&s);
        let aristas_v1 = csv_aristas_kb_lira_paso3(&s);

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

        let nodos_v2 = csv_nodos_kb_lira_paso3(&s2);
        let aristas_v2 = csv_aristas_kb_lira_paso3(&s2);
        assert_eq!(nodos_v1, nodos_v2, "roundtrip nodos byte a byte");
        assert_eq!(aristas_v1, aristas_v2, "roundtrip aristas byte a byte");
    }

    /// histórico → CSV → importación → CSV: bytes IDÉNTICOS. Y el histórico
    /// reconstruido responde igual (los ts originales se conservan) y su
    /// `siguiente_ts` continúa en máximo+1.
    #[test]
    fn csv_historico_roundtrip_byte_a_byte() {
        let h = historico_kb_lira_paso3();
        let v1 = csv_historico(&h);

        let h2 = historico_desde_csv(&v1);
        let v2 = csv_historico(&h2);
        assert_eq!(v1, v2, "roundtrip histórico byte a byte");

        // Las entradas se reconstruyen idénticas (ts originales, sin
        // reasignación) y registrar continúa tras el máximo+1.
        assert_eq!(h2.entradas(), h.entradas());
        let mut h3 = h2.clone();
        let ts = h3.registrar(ids::ANA, ids::UNI_LIRA, 2018);
        assert_eq!(ts, h.entradas().len() as u64 + 1, "siguiente_ts = máximo+1");
    }

    /// datasets/kb-lira/paso-3/ ES la salida de los builders del paso-3:
    /// nodes.csv + edges.csv (formato cap. 32) y historico.csv (formato de
    /// [`csv_historico`]). Si alguien regenera el builder y olvida commitear,
    /// este test grita (mismo mecanismo que cap-41/cap-42).
    #[test]
    fn csv_paso3_coincide_con_dataset_commiteado_byte_a_byte() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-3");
        let s = kb_lira_paso3();

        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso3(&s), esperado_nodos);
        assert_eq!(csv_aristas_kb_lira_paso3(&s), esperado_aristas);

        let esperado_historico = std::fs::read_to_string(format!("{base}/historico.csv"))
            .expect("dataset historico.csv");
        assert_eq!(
            csv_historico(&historico_kb_lira_paso3()),
            esperado_historico
        );
    }

    /// El paso-3 NO toca los datasets de los pasos anteriores: los ficheros
    /// commiteados de datasets/kb-lira/paso-1/ y paso-2/ siguen siendo la
    /// salida EXACTA de los builders del cap-41 y cap-42 (mismo patrón que
    /// `csv_paso1_intacto_tras_paso2` del cap-42).
    #[test]
    fn csv_paso1_y_paso2_intactos_tras_paso3() {
        let base1 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-1");
        let base2 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-2");

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
    }
}

// ─────────────────── Informe reproducible del capítulo (para la prosa) ───────────────────

/// El informe temporal REPRODUCIBLE del capítulo: la tesis completa en texto
/// plano, SIN tiempos de ejecución — la prosa del cap-43 lo cita tal cual.
///
/// Parte 1 — tabla AS OF de las afiliaciones de «Proyecto Kira» por año
/// (`[2026, 2024, 2023, 2020, 2019]`, de hoy hacia atrás): las respuestas
/// REALES de [`afiliaciones_vigentes_en`] (persona → organización, en el
/// orden natural del cap-41) y su coste REAL en lecturas al store
/// ([`CosteLecturas::total`], el ledger de la pieza 3).
///
/// Parte 2 — el caso Dani bitemporal: «¿qué creíamos en 2024?» (ts 1 → desde
/// 2019, lo que el registro anotó en su día) frente a «¿qué sabemos hoy?»
/// (ts 2 → desde 2021, tras la corrección), ambas respuestas de
/// [`afiliacion_segun_registro`] sobre [`historico_kb_lira_paso3`]. El nombre
/// de la organización se resuelve en `store` — nada hardcodeado.
pub fn informe_temporal_reproducible(store: &dyn GraphStore) -> String {
    let mut buf = String::new();
    buf.push_str("Barrido temporal de afiliaciones de «Proyecto Kira» (KB-Lira paso-3)\n");

    // Parte 1: la tabla AS OF por año — respuestas y coste REALES de la
    // consulta del capítulo ([`afiliaciones_vigentes_en`]).
    let mut filas: Vec<String> = Vec::with_capacity(5);
    for anio in [2026i64, 2024, 2023, 2020, 2019] {
        let (afiliaciones, coste) = afiliaciones_vigentes_en(store, "Proyecto Kira", anio);
        let parejas: Vec<String> = afiliaciones
            .iter()
            .map(|(persona, org)| format!("{persona} → {org}"))
            .collect();
        filas.push(format!(
            "{anio} | {} | {} lecturas",
            parejas.join("; "),
            coste.total()
        ));
    }
    let ancho = filas.iter().map(String::len).max().unwrap_or(0);
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');
    for fila in &filas {
        buf.push_str(fila);
        buf.push('\n');
    }
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');

    // Parte 2: el caso Dani bitemporal — el registro (transaction-time)
    // frente a la arista (valid-time): lo que se creía vs lo que se sabe.
    let nombre_org = |id: usize| -> String {
        match store.get_node(id) {
            Some(nodo) => match nodo.props.get("nombre") {
                Some(Value::String(nombre)) => nombre.clone(),
                _ => format!("organización #{id}"),
            },
            None => format!("organización #{id}"),
        }
    };
    let historico = historico_kb_lira_paso3();
    let (org_ts1, desde_ts1) = historico
        .afiliacion_segun_registro(ids::DANI, 2024, 1)
        .expect("el histórico registró el ts 1 de Dani");
    let (org_ts2, desde_ts2) = historico
        .afiliacion_segun_registro(ids::DANI, ANIO_ACTUAL, 2)
        .expect("el histórico registró el ts 2 de Dani");
    buf.push_str("Caso Dani (bitemporal): lo que se creía frente a lo que se sabe\n");
    buf.push_str(&format!(
        "  ts 1 · {:<26} → {}, desde {desde_ts1} (lo que se creía)\n",
        "«¿qué creíamos en 2024?»",
        nombre_org(org_ts1)
    ));
    buf.push_str(&format!(
        "  ts 2 · {:<26} → {}, desde {desde_ts2} (lo que se sabe)\n",
        "«¿qué sabemos hoy?»",
        nombre_org(org_ts2)
    ));
    buf
}

#[cfg(test)]
mod tests_informe_temporal {
    use super::*;

    /// El informe es REPRODUCIBLE: `informe_temporal_reproducible` produce una
    /// salida estable byte a byte (tabla AS OF de afiliaciones de «Proyecto
    /// Kira» por año con respuestas y coste reales + el caso Dani bitemporal)
    /// — el literal de abajo es la salida REAL fijada a mano.
    #[test]
    fn informe_temporal_reproducible_sobre_kb_lira() {
        let s = kb_lira_paso3();
        let reporte = informe_temporal_reproducible(&s);
        // La salida REAL fijada a mano: `concat!` une los fragmentos (las dos
        // líneas del caso Dani llevan sangría propia, que el estilo `\n\` del
        // cap-42 no puede representar porque recorta el espacio inicial).
        let esperado = concat!(
            "Barrido temporal de afiliaciones de «Proyecto Kira» (KB-Lira paso-3)\n\
             ──────────────────────────────────────────────────────────────────────────────────────────────────────────────\n\
             2026 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas\n\
             2024 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas\n\
             2023 | Ana → Universidad de Lira; Beto → Instituto Neurónica; Dani → Instituto Neurónica | 28 lecturas\n\
             2020 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas\n\
             2019 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas\n\
             ──────────────────────────────────────────────────────────────────────────────────────────────────────────────\n\
             Caso Dani (bitemporal): lo que se creía frente a lo que se sabe\n",
            "  ts 1 · «¿qué creíamos en 2024?»   → Instituto Neurónica, desde 2019 (lo que se creía)\n",
            "  ts 2 · «¿qué sabemos hoy?»        → Instituto Neurónica, desde 2021 (lo que se sabe)\n",
        );
        assert_eq!(
            reporte, esperado,
            "la salida del informe debe estar pineada byte a byte"
        );
    }
}
