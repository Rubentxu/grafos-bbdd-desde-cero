//! Vol.III — Cap.44: Esquema declarativo: las reglas como datos.
//!
//! CUARTO capítulo del Vol.III («Grafos en la era de la IA»), Parte I
//! «Modelar datos de grafos». El cap. 41 validó KB-Lira con un validador
//! escrito a mano por arista/nodo (convención ejecutable); el cap. 42 pagó
//! los antipatrones y el cap. 43 añadió la dimensión tiempo. ESTE capítulo
//! hace el paso siguiente del contrato: separar las REGLAS del CÓDIGO que
//! las comprueba. El esquema pasa a ser DATOS ([`Esquema`] + [`ReglaConstraint`])
//! y el verificador ([`verificar_esquema`]) se vuelve un intérprete genérico:
//! añadir una regla nueva no toca el motor, solo el catálogo.
//!
//! Modelo mental único: **un constraint declarativo es una PRUEBA que el
//! grafo debe pasar, no una función que lo recorre**. La regla dice QUÉ se
//! exige (datos); el verificador decide CÓMO comprobarlo (código). Esta
//! separación es la semilla de los shapes del cap. 47 y del capítulo de
//! calidad de datos.
//!
//! Qué entrega ESTA pieza (implementación incremental, contrato §2):
//!
//! 1. **Seis reglas** ([`ReglaConstraint`]): `Extremos` (tipos de extremos
//!    por tipo de arista — el hueco del LPG denunciado en el cap. 41),
//!    `Existencia` (campo obligatorio por label), `Tipo` (tipo esperado de
//!    una propiedad), `Unicidad` (propiedad con valor único por label),
//!    `SinSolape` (intervalos temporales disjuntos por par de extremos) e
//!    `IntervaloValido` (el contrato temporal del cap. 43 como DATOS: la
//!    cota inferior REQUERIDA, `hasta >= desde`, sin validez futura).
//! 2. **Verificador genérico** ([`verificar_esquema`]) que recorre el grafo
//!    UNA vez por regla y devuelve `Err(Vec<Violacion>)` con el id y tipo de
//!    cada elemento implicado (la [`Violacion`] del cap. 41, REUTILIZADA).
//! 3. **Lección estructural**: `Unicidad` se implementa con un `HashMap`
//!    interno clave→primer id. UNIQUE ≡ índice: sin un índice por debajo,
//!    la unicidad ES un escaneo que construye el índice en cada verificación.
//! 4. **KB-Lira paso-4** ([`esquema_kb_lira`] + [`kb_lira_paso4`]): el modelo
//!    del paso-3 (cap. 43) más `orcid` en las 9 personas, validado contra el
//!    esquema canónico que declara COMO DATOS lo que los validadores de los
//!    caps. 41-43 exigían a mano. Primera mutación de `Extremos` nacida del
//!    modelo real: MENTIONS es polimórfico (→Persona|Organizacion|Proyecto,
//!    decisión #7 del cap. 41) y `hasta_labels` (lista, semántica
//!    CUALQUIERA-uno como el `any` del validador) lo expresa con UNA regla.
//! 5. **La tesis cobrada** (`el_esquema_subsume_la_cadena_de_validadores_
//!    sobre_fixture_corrupto`): sobre un fixture corrupto — una CITES con
//!    un extremo sustituido por un Tema, un Documento sin `titulo`, una
//!    MEMBER_OF sin `desde_anio` — el esquema detecta TODO lo que la cadena
//!    de validadores de los caps. 41-43 detecta (los MISMOS ids implicados,
//!    por subsunción). Lo que tres validadores exigían por composición, un
//!    esquema lo declara.
//!
//! Frontera declarada: solo API pública del cap. 7 (modelo), cap. 41
//! (la violación y [`crate::cap41_modelado::nodo_por_nombre`]) y cap. 43
//! (`kb_lira_paso3`, el validador paso-3, [`ANIO_ACTUAL`] y
//! [`crate::cap43_temporalidad::arista_vigente_en`]); el verificador NO
//! escribe en el store; `TipoEsperado` queda mínimo (`Int`/`String`) — se
//! extiende sin tocar el verificador.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::cap07_modelo::{Edge, EdgeId, Value};
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap32_import_export::{exportar_csv_aristas, exportar_csv_nodos};
use crate::cap41_modelado::{Violacion, ids, nodo_por_nombre};
use crate::cap43_temporalidad::{
    ANIO_ACTUAL, afiliaciones_vigentes_en, arista_vigente_en, kb_lira_paso3,
};

// ─────────────────── Esquema declarativo (las reglas como datos) ───────────────────

/// El tipo esperado de una propiedad en la regla [`ReglaConstraint::Tipo`].
///
/// Mínimo a propósito: añadir una variante (`Float`, `Bool`, …) es un cambio
/// local que el verificador despacha sin reescribir.
///
/// `Int`/`String` son las dos variantes que el verificador despacha; los
/// tests de ESTA pieza solo construyen las reglas `Extremos`/`Existencia`
/// (patrón de troceo incremental), así que las variantes restantes quedan
/// como semilla del catálogo para las piezas siguientes (cap. 47).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum TipoEsperado {
    Int,
    String,
}

/// Una regla del esquema, expresada como DATOS (nada de código por regla).
///
/// - `Extremos`: toda arista `rel_tipo` debe ir de un nodo `desde_label` a un
///   nodo con CUALQUIERA de los labels `hasta_labels` (el contrato que el
///   cap. 41 suplía a mano). La lista nace del modelo real: MENTIONS tiene
///   destino POLIMÓRFICO (→Persona|Organizacion|Proyecto) y un solo label no
///   lo expresa; tres reglas separadas serían un AND (el destino tendría que
///   llevar los tres labels), la lista es el OR del `any` del validador.
/// - `Existencia`: todo nodo `label` debe tener la propiedad `propiedad`.
/// - `Tipo`: si un nodo `label` tiene la propiedad, debe ser del tipo esperado.
/// - `Unicidad`: entre los nodos `label`, la propiedad debe ser única.
/// - `SinSolape`: entre aristas `rel_tipo`, intervalos `[desde, hasta)`
///   disjuntos para cada par (origen, destino); `hasta` ausente = +infinito.
/// - `IntervaloValido`: toda arista `rel_tipo` declara `desde_prop:Int` con
///   `desde <= anio_max` (sin validez futura) y, si `hasta_prop` está, debe
///   ser `Int` con `hasta >= desde` (un intervalo cuyo fin precede a su
///   inicio no es un intervalo). Las reglas temporales que el cap. 43
///   escribía a mano en el validador paso-3, ahora como datos.
///
/// Igual que [`TipoEsperado`]: el catálogo es completo pero los tests de esta
/// pieza solo construyen `Extremos`/`Existencia` (troceo incremental); las
/// demás variantes son semilla para las piezas siguientes.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ReglaConstraint {
    Extremos {
        rel_tipo: String,
        desde_label: String,
        hasta_labels: Vec<String>,
    },
    Existencia {
        label: String,
        propiedad: String,
    },
    Tipo {
        label: String,
        propiedad: String,
        esperado: TipoEsperado,
    },
    Unicidad {
        label: String,
        propiedad: String,
    },
    SinSolape {
        rel_tipo: String,
        desde_anio_prop: String,
        hasta_anio_prop: String,
    },
    IntervaloValido {
        rel_tipo: String,
        desde_prop: String,
        hasta_prop: String,
        anio_max: i64,
    },
}

/// Un esquema declarativo: la lista de reglas que todo grafo debe cumplir.
#[derive(Debug, Clone, Default)]
pub struct Esquema {
    pub reglas: Vec<ReglaConstraint>,
}

impl Esquema {
    /// Esquema vacío: todo grafo lo cumple (el contrato mínimo).
    pub fn nueva() -> Self {
        Self { reglas: Vec::new() }
    }

    /// Builder encadenable: devuelve el esquema con `regla` añadida.
    pub fn con(mut self, regla: ReglaConstraint) -> Self {
        self.reglas.push(regla);
        self
    }
}

// ─────────────────── Verificador genérico (el intérprete de reglas) ───────────────────

/// Verifica `esquema` sobre `store`: `Ok(())` si se cumplen todas las reglas
/// o `Err(violaciones)` con la lista COMPLETA (una pasada por regla, se
/// reportan todos los incumplimientos, no solo el primero).
pub fn verificar_esquema(store: &dyn GraphStore, esquema: &Esquema) -> Result<(), Vec<Violacion>> {
    let mut malas: Vec<Violacion> = Vec::new();

    for regla in &esquema.reglas {
        match regla {
            ReglaConstraint::Extremos {
                rel_tipo,
                desde_label,
                hasta_labels,
            } => {
                for ar in store.iter_edges().filter(|e| e.label == *rel_tipo) {
                    let src_ok = store
                        .get_node(ar.source)
                        .map(|n| n.has_label(desde_label))
                        .unwrap_or(false);
                    // OR de labels: el destino vale si lleva CUALQUIERA de los
                    // `hasta_labels` (MENTIONS polimórfico del cap. 41).
                    let dst_ok = store
                        .get_node(ar.target)
                        .map(|n| hasta_labels.iter().any(|l| n.has_label(l)))
                        .unwrap_or(false);
                    if !src_ok || !dst_ok {
                        malas.push(Violacion {
                            descripcion: format!(
                                "{rel_tipo} {}→{} viola Extremos: origen sin label \
                                 '{desde_label}' o destino sin ninguno de {:?}",
                                ar.source, ar.target, hasta_labels
                            ),
                            id_implicado: ar.id,
                            tipo_elemento: "arista",
                        });
                    }
                }
            }
            ReglaConstraint::Existencia { label, propiedad } => {
                for n in store.iter_nodes().filter(|n| n.has_label(label)) {
                    if !n.props.contains_key(propiedad) {
                        malas.push(Violacion {
                            descripcion: format!(
                                "{label} sin propiedad '{propiedad}' (exigida por Existencia)"
                            ),
                            id_implicado: n.id,
                            tipo_elemento: "nodo",
                        });
                    }
                }
            }
            ReglaConstraint::Tipo {
                label,
                propiedad,
                esperado,
            } => {
                for n in store.iter_nodes().filter(|n| n.has_label(label)) {
                    if let Some(v) = n.props.get(propiedad) {
                        let tipo_ok = match esperado {
                            TipoEsperado::Int => matches!(v, Value::Int(_)),
                            TipoEsperado::String => matches!(v, Value::String(_)),
                        };
                        if !tipo_ok {
                            malas.push(Violacion {
                                descripcion: format!(
                                    "{label} con propiedad '{propiedad}' de tipo {:?} \
                                     (esperado {:?} por Tipo)",
                                    v.type_name(),
                                    tipo_esperado_nombre(esperado)
                                ),
                                id_implicado: n.id,
                                tipo_elemento: "nodo",
                            });
                        }
                    }
                }
            }
            ReglaConstraint::Unicidad { label, propiedad } => {
                // LECCIÓN: UNIQUE ≡ ÍNDICE. Sin un índice real por debajo, la
                // unicidad ES este HashMap interno: por cada valor se recuerda
                // el PRIMER id que lo reclamó; el SEGUNDO es la violación.
                // El grafo no tiene índices propios hasta el cap. 15/21: aquí
                // la verificación construye el índice sobre la marcha.
                let mut primer_id: HashMap<String, usize> = HashMap::new();
                for n in store.iter_nodes().filter(|n| n.has_label(label)) {
                    let Some(v) = n.props.get(propiedad) else {
                        continue; // sin la propiedad no hay valor que deba ser único
                    };
                    let clave = clave_valor(v);
                    match primer_id.entry(clave) {
                        std::collections::hash_map::Entry::Vacant(vacio) => {
                            vacio.insert(n.id);
                        }
                        std::collections::hash_map::Entry::Occupied(ocupado) => {
                            malas.push(Violacion {
                                descripcion: format!(
                                    "{label} con propiedad '{propiedad}' repetida \
                                     (mismo valor que el nodo {}; Unicidad)",
                                    ocupado.get()
                                ),
                                id_implicado: n.id, // el SEGUNDO: el índice ya lo tenía
                                tipo_elemento: "nodo",
                            });
                        }
                    }
                }
            }
            ReglaConstraint::SinSolape {
                rel_tipo,
                desde_anio_prop,
                hasta_anio_prop,
            } => {
                // Agrupa por par (origen, destino): el solape solo importa
                // entre aristas que unen los MISMOS extremos.
                let mut por_par: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
                for ar in store.iter_edges().filter(|e| e.label == *rel_tipo) {
                    por_par
                        .entry((ar.source, ar.target))
                        .or_default()
                        .push(ar.id);
                }
                for ((from, to), ids) in por_par {
                    // Orden de ids = orden de iter_edges (determinista).
                    for i in 0..ids.len() {
                        for j in (i + 1)..ids.len() {
                            let a = store.get_edge(ids[i]).expect("id agrupado válido");
                            let b = store.get_edge(ids[j]).expect("id agrupado válido");
                            let (a_desde, a_hasta) =
                                intervalo_de(a, desde_anio_prop, hasta_anio_prop);
                            let (b_desde, b_hasta) =
                                intervalo_de(b, desde_anio_prop, hasta_anio_prop);
                            // [desde, hasta): solape si a.desde < b.hasta && b.desde < a.hasta
                            let solapan = a_desde < b_hasta.unwrap_or(i64::MAX)
                                && b_desde < a_hasta.unwrap_or(i64::MAX);
                            if solapan {
                                malas.push(Violacion {
                                    descripcion: format!(
                                        "{rel_tipo} {}→{}: la arista {} solapa en \
                                         [{}, {}) con la arista {} en [{}, {}) (SinSolape)",
                                        from,
                                        to,
                                        b.id, // la SEGUNDA en el orden de escaneo
                                        b_desde,
                                        fmt_hasta(b_hasta),
                                        a.id,
                                        a_desde,
                                        fmt_hasta(a_hasta)
                                    ),
                                    id_implicado: b.id,
                                    tipo_elemento: "arista",
                                });
                            }
                        }
                    }
                }
            }
            ReglaConstraint::IntervaloValido {
                rel_tipo,
                desde_prop,
                hasta_prop,
                anio_max,
            } => {
                // El contrato temporal del cap. 43 como regla declarativa:
                // (1) la cota inferior es REQUERIDA y `Int`; (2) sin validez
                // futura: `desde <= anio_max`; (3) si hay cota superior, debe
                // ser `Int` con `hasta >= desde`. El validador paso-3 del
                // cap. 43 escribía estas tres comprobaciones a mano sobre
                // MEMBER_OF; el catálogo las declara y el verificador —que
                // no cambia— las ejecuta.
                for ar in store.iter_edges().filter(|e| e.label == *rel_tipo) {
                    let desde = match ar.props.get(desde_prop) {
                        Some(Value::Int(d)) => *d,
                        _ => {
                            malas.push(Violacion {
                                descripcion: format!(
                                    "{rel_tipo} {} sin '{desde_prop}:Int' \
                                     (exigido por IntervaloValido)",
                                    ar.id
                                ),
                                id_implicado: ar.id,
                                tipo_elemento: "arista",
                            });
                            continue;
                        }
                    };
                    if desde > *anio_max {
                        malas.push(Violacion {
                            descripcion: format!(
                                "{rel_tipo} {} con '{desde_prop}' {desde} posterior a \
                                 {anio_max} (sin validez futura; IntervaloValido)",
                                ar.id
                            ),
                            id_implicado: ar.id,
                            tipo_elemento: "arista",
                        });
                    }
                    match ar.props.get(hasta_prop) {
                        Some(Value::Int(h)) if *h < desde => malas.push(Violacion {
                            descripcion: format!(
                                "{rel_tipo} {} con '{hasta_prop}' {h} anterior a \
                                 '{desde_prop}' {desde} (IntervaloValido)",
                                ar.id
                            ),
                            id_implicado: ar.id,
                            tipo_elemento: "arista",
                        }),
                        Some(Value::Int(_)) => {}
                        Some(otro) => malas.push(Violacion {
                            descripcion: format!(
                                "{rel_tipo} {} con '{hasta_prop}' no-Int: {otro:?} \
                                 (IntervaloValido)",
                                ar.id
                            ),
                            id_implicado: ar.id,
                            tipo_elemento: "arista",
                        }),
                        None => {}
                    }
                }
            }
        }
    }

    if malas.is_empty() { Ok(()) } else { Err(malas) }
}

/// Clave canónica de un valor para el HashMap de [`ReglaConstraint::Unicidad`].
///
/// `Value` (cap. 7) no deriva `Hash` y, al ser de otro módulo, la regla del
/// orfanato impide implementárselo aquí: la clave es una representación
/// textual que distingue tipo y contenido (dos valores iguales comparten
/// clave; un `Int` y un `String` con el mismo texto, no).
fn clave_valor(v: &Value) -> String {
    match v {
        Value::Int(i) => format!("int:{i}"),
        Value::String(s) => format!("str:{s}"),
        otro => format!("{:?}", otro),
    }
}

fn tipo_esperado_nombre(e: &TipoEsperado) -> &'static str {
    match e {
        TipoEsperado::Int => "Int",
        TipoEsperado::String => "String",
    }
}

/// Lee el intervalo `[desde, hasta)` de una arista: `desde` ausente se trata
/// como sin cota inferior; `hasta` ausente como +infinito (intervalo abierto,
/// la convención del cap. 43).
fn intervalo_de(ar: &Edge, desde_prop: &str, hasta_prop: &str) -> (i64, Option<i64>) {
    let desde = match ar.props.get(desde_prop) {
        Some(Value::Int(d)) => *d,
        _ => i64::MIN,
    };
    let hasta = match ar.props.get(hasta_prop) {
        Some(Value::Int(h)) => Some(*h),
        _ => None,
    };
    (desde, hasta)
}

fn fmt_hasta(hasta: Option<i64>) -> String {
    hasta.map_or_else(|| "+inf".to_string(), |h| h.to_string())
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_esquema {
    use super::*;
    use crate::cap07_modelo::Node;
    use crate::cap08_graph_store::MemoryStore;

    #[test]
    fn verificar_esquema_acepta_el_modelo_sin_reglas() {
        // Un esquema vacío es el contrato mínimo: ningún grafo puede fallarlo.
        let s = MemoryStore::new();
        let esquema = Esquema::nueva();
        assert_eq!(verificar_esquema(&s, &esquema), Ok(()));
    }

    #[test]
    fn verificar_esquema_detecta_extremos_y_existencia() {
        // Grafo mínimo a mano: 0 Persona, 1 Tema, 2 Documento SIN titulo.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "Persona").with_prop("nombre", Value::String("Ana".into())))
            .unwrap();
        s.put_node(Node::new(1, "Tema").with_prop("nombre", Value::String("grafos".into())))
            .unwrap();
        s.put_node(Node::new(2, "Documento")).unwrap();
        // AUTHORED desde un Tema (debería ser Persona→Documento): extremos ✗.
        s.put_edge(Edge::new(0, 1, 2, "AUTHORED")).unwrap();

        let esquema = Esquema::nueva()
            .con(ReglaConstraint::Extremos {
                rel_tipo: "AUTHORED".into(),
                desde_label: "Persona".into(),
                hasta_labels: vec!["Documento".into()],
            })
            .con(ReglaConstraint::Existencia {
                label: "Documento".into(),
                propiedad: "titulo".into(),
            });

        let err = verificar_esquema(&s, &esquema).expect_err("esquema violado");
        // La arista 0 con extremos equivocados y el nodo 2 sin titulo.
        assert_eq!(err.len(), 2);
        assert!(
            err.iter()
                .any(|v| v.id_implicado == 0 && v.tipo_elemento == "arista")
        );
        assert!(
            err.iter()
                .any(|v| v.id_implicado == 2 && v.tipo_elemento == "nodo")
        );
    }
}

// ─────────────────── KB-Lira paso-4: orcid y el esquema canónico ───────────────────

/// ORCID FIJOS de las 9 personas de KB-Lira paso-4 (disciplina determinista
/// del cap. 34: nada de RNG): strings estáticos en el formato ORCID real
/// (`XXXX-XXXX-XXXX-XXXX`), TODOS distintos → el modelo canónico cumple
/// `Unicidad(Persona.orcid)`. No son ORCID válidos de verdad (la suma de
/// verificación no cierra) y no necesitan serlo: el contrato solo exige que
/// sean SIEMPRE los mismos. Patrón estable por persona: `0000-000{n}-…-00{n}`
/// con n = 1..9 en orden de id (Ana=1 … Iris=9).
const ORCID_PERSONAS: [(usize, &str); 9] = [
    (ids::ANA, "0000-0001-2345-0001"),
    (ids::BETO, "0000-0002-3456-0002"),
    (ids::CARLA, "0000-0003-4567-0003"),
    (ids::DANI, "0000-0004-5678-0004"),
    (ids::ELENA, "0000-0005-6789-0005"),
    (ids::FABIO, "0000-0006-7890-0006"),
    (30, "0000-0007-8901-0007"), // Gaby (lote del cap-42)
    (31, "0000-0008-9012-0008"), // Hugo
    (32, "0000-0009-0123-0009"), // Iris
];

/// El esquema CANÓNICO de KB-Lira paso-4: las reglas que los validadores de
/// los caps. 41-43 exigían A MANO (extremos, existencia, tipos) ahora como
/// DATOS, más las dos reglas nuevas del paso-4. Dieciséis reglas:
///
/// - **Extremos** (6): AUTHORED Persona→Documento, CITES Documento→Documento,
///   ABOUT Documento→Tema, MEMBER_OF Persona→Organizacion, WORKED_ON
///   Persona→Proyecto y MENTIONS Documento→Persona|Organizacion|Proyecto —
///   la lista de destinos reproduce el `any` del validador del cap. 41
///   (destino polimórfico, decisión #7).
/// - **Existencia** (5): Documento.titulo, Documento.anio, Persona.nombre,
///   Organizacion.nombre, Tema.nombre.
/// - **Tipo** (2): Documento.anio Int, Documento.titulo String.
/// - **Unicidad** (1): Persona.orcid — LA NUEVA del paso-4.
/// - **SinSolape** (1): MEMBER_OF con intervalos `[desde_anio, hasta_anio)`
///   disjuntos por par (persona, organización) — la regla temporal del
///   cap. 43 promovida a constraint: el gancho «¿quién garantiza que dos
///   afiliaciones de una persona a la misma organización no se solapen?».
/// - **IntervaloValido** (1): MEMBER_OF con `desde_anio:Int` REQUERIDO y
///   `desde_anio <= ANIO_ACTUAL` (sin validez futura) y, si `hasta_anio`
///   está presente, `hasta_anio >= desde_anio` — las tres comprobaciones
///   del validador paso-3 (cap. 43) promovidas a constraint declarativo.
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn esquema_kb_lira() -> Esquema {
    Esquema::nueva()
        // ── Extremos: los seis tipos de arista del paso-1 (cap. 41) ──
        .con(ReglaConstraint::Extremos {
            rel_tipo: "AUTHORED".into(),
            desde_label: "Persona".into(),
            hasta_labels: vec!["Documento".into()],
        })
        .con(ReglaConstraint::Extremos {
            rel_tipo: "CITES".into(),
            desde_label: "Documento".into(),
            hasta_labels: vec!["Documento".into()],
        })
        .con(ReglaConstraint::Extremos {
            rel_tipo: "ABOUT".into(),
            desde_label: "Documento".into(),
            hasta_labels: vec!["Tema".into()],
        })
        .con(ReglaConstraint::Extremos {
            rel_tipo: "MEMBER_OF".into(),
            desde_label: "Persona".into(),
            hasta_labels: vec!["Organizacion".into()],
        })
        .con(ReglaConstraint::Extremos {
            rel_tipo: "WORKED_ON".into(),
            desde_label: "Persona".into(),
            hasta_labels: vec!["Proyecto".into()],
        })
        // MENTIONS es POLIMÓRFICO (decisión #7 del cap. 41): el destino vale
        // con CUALQUIERA de los tres labels, igual que el `any` del validador.
        // Tres reglas separadas NO valen: serían un AND (el destino tendría
        // que llevar los tres labels a la vez).
        .con(ReglaConstraint::Extremos {
            rel_tipo: "MENTIONS".into(),
            desde_label: "Documento".into(),
            hasta_labels: vec!["Persona".into(), "Organizacion".into(), "Proyecto".into()],
        })
        // ── Existencia: los campos obligatorios que el cap. 41 exigía ──
        .con(ReglaConstraint::Existencia {
            label: "Documento".into(),
            propiedad: "titulo".into(),
        })
        .con(ReglaConstraint::Existencia {
            label: "Documento".into(),
            propiedad: "anio".into(),
        })
        .con(ReglaConstraint::Existencia {
            label: "Persona".into(),
            propiedad: "nombre".into(),
        })
        .con(ReglaConstraint::Existencia {
            label: "Organizacion".into(),
            propiedad: "nombre".into(),
        })
        .con(ReglaConstraint::Existencia {
            label: "Tema".into(),
            propiedad: "nombre".into(),
        })
        // ── Tipo: el contrato documental del cap. 41 ──
        .con(ReglaConstraint::Tipo {
            label: "Documento".into(),
            propiedad: "anio".into(),
            esperado: TipoEsperado::Int,
        })
        .con(ReglaConstraint::Tipo {
            label: "Documento".into(),
            propiedad: "titulo".into(),
            esperado: TipoEsperado::String,
        })
        // ── Unicidad: LA NUEVA del paso-4 — cada persona con su orcid ──
        .con(ReglaConstraint::Unicidad {
            label: "Persona".into(),
            propiedad: "orcid".into(),
        })
        // ── SinSolape: la regla temporal del cap. 43 como constraint ──
        .con(ReglaConstraint::SinSolape {
            rel_tipo: "MEMBER_OF".into(),
            desde_anio_prop: "desde_anio".into(),
            hasta_anio_prop: "hasta_anio".into(),
        })
        // ── IntervaloValido: el contrato temporal del paso-3 como dato ──
        // El validador paso-3 (cap. 43) exigía a mano las tres condiciones
        // (desde REQUERIDO, desde <= ANIO_ACTUAL, hasta >= desde); aquí son
        // UNA regla del catálogo: añadir una condición no toca el verificador.
        .con(ReglaConstraint::IntervaloValido {
            rel_tipo: "MEMBER_OF".into(),
            desde_prop: "desde_anio".into(),
            hasta_prop: "hasta_anio".into(),
            anio_max: ANIO_ACTUAL,
        })
}

/// Construye KB-Lira **paso-4**: PARTE de [`kb_lira_paso3`] (cap-43, lo llama
/// tal cual: refactorizado del cap-42 + valid-time) y le añade `orcid:String`
/// a las 9 personas (ids 0-5 —las 6 del paso-1— y 30-32 —Gaby/Hugo/Iris, el
/// lote del cap-42—). El modelo canónico NO tiene duplicados: todas con
/// orcid ÚNICO, porque el canónico debe pasar su propio esquema con `Ok(())`;
/// el fixture corrupto que demuestra la `Unicidad` se construye a mano
/// mutando el store en los tests de las piezas siguientes.
///
/// Devuelve el par `(store, esquema)` listo para [`verificar_esquema`].
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn kb_lira_paso4() -> (MemoryStore, Esquema) {
    let mut store = kb_lira_paso3();
    for (id, orcid) in ORCID_PERSONAS {
        let persona = store.nodes[id]
            .as_mut()
            .expect("persona del paso-3 presente");
        persona
            .props
            .insert("orcid".into(), Value::String(orcid.into()));
    }
    (store, esquema_kb_lira())
}

#[cfg(test)]
mod tests_esquema_paso4 {
    use super::*;
    use crate::cap07_modelo::Node;
    use crate::cap43_temporalidad::validar_modelo_kb_lira_paso3;
    use std::collections::BTreeSet;

    /// El modelo canónico del paso-4 pasa SU PROPIO esquema con `Ok(())`
    /// (las quince reglas). Es la prueba de que el esquema describe el modelo
    /// real y no un ideal: si una regla no cuadrara con lo que los validadores
    /// 41-43 ya exigen, este test lo denunciaría.
    #[test]
    fn kb_lira_paso4_pasa_su_propio_esquema() {
        let (store, esquema) = kb_lira_paso4();
        assert_eq!(verificar_esquema(&store, &esquema), Ok(()));

        // Honestidad de la Unicidad: si NINGUNA persona tuviera orcid, la
        // regla pasaría por no tener valores que comprobar (el verificador
        // salta los nodos sin la propiedad). El paso-4 debe dejar las 9
        // personas con orcid no vacío.
        let personas: Vec<&Node> = store
            .iter_nodes()
            .filter(|n| n.has_label("Persona"))
            .collect();
        assert_eq!(personas.len(), 9);
        for p in &personas {
            assert!(
                matches!(p.props.get("orcid"), Some(Value::String(s)) if !s.is_empty()),
                "persona {} sin orcid:String",
                p.id
            );
        }
    }

    /// FIXTURE CORRUPTO de la `Unicidad`, construido A MANO sobre el canónico
    /// (precedente caps. 42-43: mutación directa del campo público `nodes`):
    /// el orcid de Beto (id 1) pasa a ser el de Ana (id 0). El verificador
    /// reporta la violación con el id del SEGUNDO — el HashMap interno ya
    /// tenía el valor en manos de Ana, y el índice recuerda al PRIMERO:
    /// `id_implicado == 1`, nunca 0.
    #[test]
    fn orcid_duplicado_viola_la_unicidad_con_el_id_del_segundo() {
        // MemoryStore deriva `Clone` (cap. 8): clonar el paso-4 y corromper la
        // COPIA deja el canónico intacto para el resto de tests de la pieza.
        let (store, esquema) = kb_lira_paso4();
        let mut store = store.clone();
        let beto = store.nodes[ids::BETO]
            .as_mut()
            .expect("Beto presente en el paso-4");
        beto.props
            .insert("orcid".into(), Value::String("0000-0001-2345-0001".into()));

        let err = verificar_esquema(&store, &esquema).expect_err("orcid duplicado");
        // Exactamente UNA violación, y es la de Unicidad: ninguna otra regla
        // del esquema canónico se resiente porque dos personas compartan orcid.
        assert_eq!(err.len(), 1);
        let v = &err[0];
        assert_eq!(v.id_implicado, ids::BETO); // el SEGUNDO: Ana (0) lo reclamó antes
        assert!(v.descripcion.contains("orcid"));
        assert!(v.descripcion.contains("Unicidad"));
    }

    /// LECCIÓN DEL CAPÍTULO: unicidad y existencia son DOS reglas distintas.
    /// Carla (id 2) SIN orcid no es un duplicado — el verificador de Unicidad
    /// salta los nodos sin la propiedad (sin valor no hay nada que deba ser
    /// único) y solo la regla de Existencia la denuncia. Misma mutación, dos
    /// lecturas del esquema.
    #[test]
    fn persona_sin_orcid_viola_la_existencia() {
        let (mut store, esquema) = kb_lira_paso4();
        // Borrado por mutación directa del campo público `nodes` (precedente
        // cap. 42: `put_node` rechaza duplicados y el trait no tiene
        // remove_prop — esta ES la eliminación real).
        let carla = store.nodes[ids::CARLA]
            .as_mut()
            .expect("Carla presente en el paso-4");
        carla.props.remove("orcid");

        // La Existencia de Persona.orcid NO está en el esquema canónico (el
        // paso-4 la declara solo como Unicidad): se añade aquí para que la
        // lección quede visible — las dos reglas conviven y denuncian cosas
        // distintas sobre la misma propiedad.
        let esquema_con_existencia = esquema.con(ReglaConstraint::Existencia {
            label: "Persona".into(),
            propiedad: "orcid".into(),
        });

        let err = verificar_esquema(&store, &esquema_con_existencia)
            .expect_err("Carla sin orcid viola Existencia");
        // UNA violación, la de Existencia, con el id de Carla. La Unicidad NO
        // la reporta: una persona sin orcid no comparte ningún valor.
        assert_eq!(err.len(), 1);
        let v = &err[0];
        assert_eq!(v.id_implicado, ids::CARLA);
        assert!(v.descripcion.contains("Existencia"));
        assert!(
            !v.descripcion.contains("Unicidad"),
            "sin orcid NO es un duplicado: Existencia y Unicidad son reglas distintas"
        );
    }

    /// EL GANCHO DEL CAP. 43 COBRADO: «¿quién garantiza que dos afiliaciones
    /// de una persona a la misma organización no se solapen?» La `SinSolape`
    /// del esquema canónico responde: una `MEMBER_OF` NUEVA (id 190,
    /// Beto→Neurónica con `[2022, 2023)`) SOLAPA con la 53 (Beto→Neurónica,
    /// `[2018, 2024)`): el criterio `a.desde < b.hasta && b.desde < a.hasta`
    /// da 2022 < 2024 && 2018 < 2023 → TRUE. El verificador denuncia la arista
    /// NUEVA — la SEGUNDA en el orden de escaneo (el par se agrupa por ids
    /// recorridos, 53 primero, 190 después): `id_implicado == 190`, nunca 53.
    #[test]
    fn dos_member_of_solapadas_violan_sin_solape_con_el_id_de_la_segunda() {
        // Copia del paso-4 (precedente de la Unicidad): el canónico queda
        // intacto para el resto de tests de la pieza.
        let (store, esquema) = kb_lira_paso4();
        let mut store = store.clone();
        store
            .put_edge(
                Edge::new(190, ids::BETO, ids::NEURONICA, "MEMBER_OF")
                    .with_prop("desde_anio", Value::Int(2022))
                    .with_prop("hasta_anio", Value::Int(2023)),
            )
            .unwrap();

        let err = verificar_esquema(&store, &esquema).expect_err("afiliaciones solapadas");
        // EXACTAMENTE una violación: ninguna otra regla del esquema canónico
        // se resiente por añadir una MEMBER_OF legal en extremos.
        assert_eq!(err.len(), 1);
        let v = &err[0];
        assert_eq!(v.id_implicado, 190); // la NUEVA: la segunda en el escaneo
        assert_eq!(v.tipo_elemento, "arista");
        // El par denunciado es (Beto→Neurónica), no otro: la descripción
        // lleva el "origen→destino" de la arista infractora.
        assert!(
            v.descripcion
                .contains(&format!("{}→{}", ids::BETO, ids::NEURONICA))
        );
        assert!(v.descripcion.contains("SinSolape"));
    }

    /// EL BORDE DEL INTERVALO `[desde, hasta)`: la afiliación 190 con
    /// `desde_anio:2024` y `hasta_anio` AUSENTE (intervalo `[2024, +∞)`) NO
    /// solapa con la 53 (`[2018, 2024)`) porque el intervalo es MEDIO ABIERTO:
    /// `a.desde < b.hasta` da 2024 < 2024 → FALSE. Dos afiliaciones CONTIGUAS
    /// — una termina exactamente donde empieza la otra — son legales: la
    /// convención del cap. 43, ahora ejecutada por la regla.
    #[test]
    fn afiliaciones_contiguas_no_violan_sin_solape() {
        let (store, esquema) = kb_lira_paso4();
        let mut store = store.clone();
        store
            .put_edge(
                Edge::new(190, ids::BETO, ids::NEURONICA, "MEMBER_OF")
                    .with_prop("desde_anio", Value::Int(2024)),
            )
            .unwrap();

        // Sin violación de SinSolape y sin tocar ninguna otra regla (la 190
        // es una MEMBER_OF legal en extremos): el esquema COMPLETO pasa.
        assert_eq!(verificar_esquema(&store, &esquema), Ok(()));
    }

    /// LA TESIS DEL CAPÍTULO, COBRADA: el esquema declarativo SUBSUME a la
    /// cadena de validadores imperativos de los caps. 41-43. Tres corrupciones
    /// a mano sobre el paso-4 clonado (precedente pieza 3: mutación directa de
    /// los campos públicos `nodes`/`edges` del [`MemoryStore`]):
    ///
    /// (a) la CITES 16 (Consultas→GrafosAgentes) con el `source` sustituido
    ///     por el Tema 24 — el hueco de extremos que el cap. 41 denunciaba
    ///     con su validador a mano;
    /// (b) el Documento 21 (revisión por pares) SIN `titulo` — el campo
    ///     obligatorio que el cap. 41 exigía;
    /// (c) la MEMBER_OF 52 (Ana→UniLira) SIN `desde_anio` — la cota REQUERIDA
    ///     que el validador paso-3 del cap. 43 exigía.
    ///
    /// El validador de la cadena ([`validar_modelo_kb_lira_paso3`], cap. 43)
    /// y el esquema (`verificar_esquema` + `esquema_kb_lira`) fallan AMBOS, y
    /// los conjuntos de ids implicados COINCIDEN: cada id que el validador
    /// denuncia tiene su equivalente en el esquema (16, 21 y 52). La dirección
    /// importa — el esquema puede denunciar MÁS que el validador (subsume:
    /// declara que lo cubre), pero NUNCA menos: si el esquema se quedara
    /// corto, la regla que falta se añade al CATÁLOGO, no al verificador.
    #[test]
    fn el_esquema_subsume_la_cadena_de_validadores_sobre_fixture_corrupto() {
        // Copia del paso-4 (precedente de la Unicidad): el canónico queda
        // intacto para el resto de tests de la pieza.
        let (store, esquema) = kb_lira_paso4();
        let mut store = store.clone();

        // (a) CITES 16 con extremo inválido: el from pasa a ser el Tema 24
        //     (mutación directa del campo público `edges`: `put_edge`
        //     rechaza ids ya presentes y el trait no permite re-apuntar).
        store.edges[16]
            .as_mut()
            .expect("CITES 16 presente en el paso-4")
            .source = ids::TEMA_GRAFOS_CONOCIMIENTO;

        // (b) Documento 21 sin titulo: se BORRA la propiedad (igual que el
        //     precedente cap. 42 para el orcid, aquí sobre el nodo).
        store.nodes[ids::DOC_REVISION_PARES]
            .as_mut()
            .expect("Documento 21 presente en el paso-4")
            .props
            .remove("titulo");

        // (c) MEMBER_OF 52 (Ana→UniLira) sin desde_anio: lo que el validador
        //     paso-3 del cap. 43 exigía como REQUERIDO, ahora ausente.
        store.edges[52]
            .as_mut()
            .expect("MEMBER_OF 52 presente en el paso-4")
            .props
            .remove("desde_anio");

        // Ambos detectan la corrupción. Los textos de las violaciones NO se
        // comparan (el validador y el esquema redactan distinto): se comparan
        // los ids implicados — el contrato es quién está denunciado.
        let err_validador = validar_modelo_kb_lira_paso3(&store)
            .expect_err("el fixture corrupto viola el validador de la cadena 41-43");
        let err_esquema = verificar_esquema(&store, &esquema)
            .expect_err("el fixture corrupto viola el esquema declarativo");

        let ids_validador: BTreeSet<usize> = err_validador.iter().map(|v| v.id_implicado).collect();
        let ids_esquema: BTreeSet<usize> = err_esquema.iter().map(|v| v.id_implicado).collect();

        // Las corrupciones son EXACTAMENTE las que la cadena denuncia: la
        // CITES 16 (extremos), el Documento 21 (titulo) y la MEMBER_OF 52
        // (desde_anio) — ninguna otra regla del paso-3 se resiente.
        assert_eq!(ids_validador, BTreeSet::from([16, 21, 52]));

        // Subsumisión en la dirección correcta: el esquema detecta TODO lo
        // que el validador detecta (nunca menos). Aquí coinciden exactos;
        // si el esquema denunciara MÁS, la tesis seguiría en pie.
        assert!(
            ids_esquema.is_superset(&ids_validador),
            "el esquema debe subsanar al validador: esquema {ids_esquema:?} \
             contra validador {ids_validador:?}"
        );
        assert_eq!(
            ids_esquema, ids_validador,
            "los MISMOS ids implicados: {err_esquema:?}"
        );
    }
}

// ─────────────────── Índice desde-año: el rango que la búsqueda lineal no tiene ───────────────────

/// Índice secundario de aristas por `prop_anio:Int`: las lecturas `>= anio`
/// se resuelven por rango del `BTreeMap` (logarítmico + el tamaño del
/// resultado) en lugar de barrer todas las aristas del tipo — la moneda de
/// la pieza son las LECTURAS: un índice se PAGA al construir y se COBRA en
/// cada consulta.
///
/// `construido` cuenta el coste de construcción en LECTURAS: las aristas de
/// `rel_tipo` consumidas de [`GraphStore::iter_edges`] (el filtro por
/// label). Las props se leen del propio `&Edge` iterado — NO se llama a
/// [`GraphStore::get_edge`] —, así que cada arista del tipo cuesta 1 y el
/// total es el nº de aristas de `rel_tipo` barridas (lleven `prop_anio` o
/// no: la lectura ya se pagó).
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub struct IndiceDesdeAnio {
    mapa: BTreeMap<i64, Vec<EdgeId>>,
    construido: usize,
}

impl IndiceDesdeAnio {
    /// Barre `iter_edges` UNA vez: por cada arista `rel_tipo` (1 lectura)
    /// con `prop_anio:Int`, agrupa su id por año. Las aristas del tipo sin
    /// la propiedad o con valor no-Int se leen igual (cuentan en
    /// `construido`) y se descartan del mapa.
    pub fn construir(store: &dyn GraphStore, rel_tipo: &str, prop_anio: &str) -> Self {
        let mut mapa: BTreeMap<i64, Vec<EdgeId>> = BTreeMap::new();
        let mut construido: usize = 0;
        for ar in store.iter_edges() {
            if ar.label != rel_tipo {
                continue;
            }
            construido += 1;
            if let Some(Value::Int(anio)) = ar.props.get(prop_anio) {
                mapa.entry(*anio).or_default().push(ar.id);
            }
        }
        Self { mapa, construido }
    }

    /// Cuántas aristas tienen `prop_anio >= anio`: la resta del rango
    /// (tamaño del resultado), el acceso por índice que la búsqueda lineal
    /// no tiene.
    pub fn candidatas_desde(&self, anio: i64) -> usize {
        self.mapa.range(anio..).map(|(_, ids)| ids.len()).sum()
    }

    /// Las ids de las aristas con `prop_anio >= anio` (para la consulta),
    /// en orden de año y, dentro del año, en orden de iteración.
    pub fn aristas_desde(&self, anio: i64) -> Vec<EdgeId> {
        self.mapa
            .range(anio..)
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect()
    }

    /// El coste de construcción en lecturas: las aristas de `rel_tipo`
    /// consumidas del barrido (una por arista del tipo, hayan entrado o no
    /// en el mapa).
    pub fn coste_construccion(&self) -> usize {
        self.construido
    }
}

#[cfg(test)]
mod tests_indice {
    use super::*;

    /// EL GANCHO DE LA PIEZA: sobre KB-Lira paso-4, el índice MEMBER_OF/
    /// desde_anio se construye en 10 lecturas (las 10 `MEMBER_OF` del
    /// barrido) y `candidatas_desde` acota el rango sin barrer nada.
    ///
    /// Cuenta a mano de los 10 `desde_anio` (cap. 43): 52:2018, 53:2018,
    /// 54:2020, 55:2021, 56:2019, 57:2019, 182:2019, 183:2022, 184:2023,
    /// 185:2024 → SOLO la 185 (Beto→GrafoLuna) cumple `>= 2024`: las 184
    /// (Gaby, 2023) NO, las 182 (Hugo, 2019) y 183 (Iris, 2022) NO, la 53
    /// (Beto→Neurónica, 2018) NO, las 52-57 del paso-1 (2018-2021) NO.
    #[test]
    fn indice_desde_anio_acota_candidatas_y_cuesta_su_construccion() {
        let (store, _esquema) = kb_lira_paso4();
        let indice = IndiceDesdeAnio::construir(&store, "MEMBER_OF", "desde_anio");

        // Construcción: 10 lecturas (las 10 MEMBER_OF consumidas del barrido).
        assert_eq!(indice.coste_construccion(), 10);
        // Rango 2024..: solo la 185, y su id es la candidata única.
        assert_eq!(indice.candidatas_desde(2024), 1);
        assert_eq!(indice.aristas_desde(2024), vec![185]);
        // Bordes: antes de todas (10) y después de todas (0).
        assert_eq!(indice.candidatas_desde(1800), 10);
        assert_eq!(indice.candidatas_desde(2030), 0);
        assert!(indice.aristas_desde(2030).is_empty());
    }
}

// ─────────────────── La consulta global y el índice: cuándo SÍ se paga (sección 3) ───────────────────

/// La consulta GLOBAL «¿qué afiliaciones EMPEZARON desde el año `anio`?»: TODAS
/// las personas del grafo, no una. Es la consulta cuya FORMA casa con
/// [`IndiceDesdeAnio`] — su filtro es el de la clave del índice (`desde_anio`),
/// así que se puede REESCRIBIR para que el rango del `BTreeMap` acote las
/// candidatas de O(n) a O(resultado).
///
/// Devuelve `(filas, lecturas)` con `filas = (nombre_persona, nombre_org, desde)`
/// y `lecturas` el coste TOTAL (la moneda del capítulo). Convención de
/// contabilidad COHERENTE con [`IndiceDesdeAnio::construir`] (pieza 6): el
/// filtro por label vive en la ITERACIÓN y cada `MEMBER_OF` consumida del
/// barrido paga 1 `get_edge` (las 10); los `get_node` solo de las que
/// cualifican (persona + organización, 2 por fila).
///
/// Discrepancia DECLARADA con el contrato (168 → 18): el contrato contaba el
/// barrido como 158 `get_edge` — el fetch de TODAS las aristas del store — más
/// 1 `get_node` por organización; aquí, con la convención de la pieza 6, el
/// barrido paga 1 `get_edge` por `MEMBER_OF` (10, el label se filtra al iterar)
/// y 2 `get_node` por fila. Ambos conteos cuentan la MISMA lección — el índice
/// baja las candidatas de O(n) a O(resultado) — pero los números absolutos
/// difieren (168→18 ≈ ×9,3 frente a 12→3 = ×4 en 2024) porque las bases
/// cuentan lecturas distintas.
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn afiliaciones_globales_desde_anio(
    store: &dyn GraphStore,
    anio: i64,
) -> (Vec<(String, String, i64)>, usize) {
    let mut lecturas: usize = 0;
    let mut filas: Vec<(String, String, i64)> = Vec::new();

    // Barrido lineal: el label se filtra en la iteración (convención pieza 6)
    // y cada MEMBER_OF consumida paga su get_edge — la arista completa.
    for ar in store.iter_edges() {
        if ar.label != "MEMBER_OF" {
            continue;
        }
        let e = store.get_edge(ar.id).expect("arista del barrido presente");
        lecturas += 1;
        let Some(Value::Int(desde)) = e.props.get("desde_anio") else {
            continue;
        };
        if *desde < anio {
            continue;
        }
        let persona = store.get_node(e.source).expect("la persona existe");
        lecturas += 1;
        let org = store.get_node(e.target).expect("la organizacion existe");
        lecturas += 1;
        let Some(Value::String(nombre_persona)) = persona.props.get("nombre") else {
            continue;
        };
        let Some(Value::String(nombre_org)) = org.props.get("nombre") else {
            continue;
        };
        filas.push((nombre_persona.clone(), nombre_org.clone(), *desde));
    }

    // Orden estable entre implementaciones (el orden de `aristas_desde` difiere
    // del orden del barrido: agrupa por año).
    filas.sort_by(|a, b| (&a.0, &a.1, a.2).cmp(&(&b.0, &b.1, b.2)));
    (filas, lecturas)
}

/// La MISMA consulta global reescrita para usar [`IndiceDesdeAnio`]: el rango
/// `aristas_desde(anio)` acota las candidatas a las que el índice ya conoce
/// (GRATIS: no lee nada del store) y solo esas pagan su `get_edge` + 2
/// `get_node`. La respuesta es IDÉNTICA a [`afiliaciones_globales_desde_anio`]:
/// el índice cambia el coste, nunca las filas.
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn afiliaciones_globales_desde_anio_con_indice(
    indice: &IndiceDesdeAnio,
    store: &dyn GraphStore,
    anio: i64,
) -> (Vec<(String, String, i64)>, usize) {
    let mut lecturas: usize = 0;
    let mut filas: Vec<(String, String, i64)> = Vec::new();

    // El rango es gratis: las candidatas vienen del BTreeMap, no del store.
    for eid in indice.aristas_desde(anio) {
        let e = store.get_edge(eid).expect("candidata del índice presente");
        lecturas += 1;
        // El índice garantiza `desde_anio >= anio` por construcción (la clave
        // del BTreeMap): la prop se lee para la fila, no para filtrar.
        let Some(Value::Int(desde)) = e.props.get("desde_anio") else {
            continue;
        };
        let persona = store.get_node(e.source).expect("la persona existe");
        lecturas += 1;
        let org = store.get_node(e.target).expect("la organizacion existe");
        lecturas += 1;
        let Some(Value::String(nombre_persona)) = persona.props.get("nombre") else {
            continue;
        };
        let Some(Value::String(nombre_org)) = org.props.get("nombre") else {
            continue;
        };
        filas.push((nombre_persona.clone(), nombre_org.clone(), *desde));
    }

    filas.sort_by(|a, b| (&a.0, &a.1, a.2).cmp(&(&b.0, &b.1, b.2)));
    (filas, lecturas)
}

#[cfg(test)]
mod tests_indice_consulta_global {
    use super::*;

    /// LA LECCIÓN POSITIVA DE LA SECCIÓN 3: el índice simple SÍ abarata la
    /// consulta GLOBAL — porque la consulta se REESCRIBE para usar su clave.
    /// Sobre KB-Lira paso-4 en 2024 (solo la `MEMBER_OF` 185, Beto→GrafoLuna,
    /// tiene `desde_anio >= 2024`):
    ///
    /// - Barrido: 10 `get_edge` (las 10 MEMBER_OF consumidas, el label se
    ///   filtra al iterar — convención pieza 6) + 2 `get_node` (persona +
    ///   organización de la única que cualifica) = **12**.
    /// - Con índice: rango GRATIS (1 candidata en el BTreeMap) + 1 `get_edge`
    ///   + 2 `get_node` = **3**.
    ///
    /// Los números REALES de ESTA consulta (el contrato anunciaba 168 → 18 con
    /// otra forma de contar — ver [`afiliaciones_globales_desde_anio`]): el
    /// argumento es el mismo, el índice reduce las candidatas de O(n) a
    /// O(resultado); el ratio cambia porque el contrato contaba el fetch de
    /// TODAS las 158 aristas del store y aquí solo las 10 MEMBER_OF.
    #[test]
    fn el_indice_simple_abaratara_la_consulta_global_reescrita() {
        let (store, _esquema) = kb_lira_paso4();
        let indice = IndiceDesdeAnio::construir(&store, "MEMBER_OF", "desde_anio");

        let (sin, coste_sin) = afiliaciones_globales_desde_anio(&store, 2024);
        let (con, coste_con) = afiliaciones_globales_desde_anio_con_indice(&indice, &store, 2024);

        // La respuesta NO cambia: el índice cambia el coste, nunca las filas.
        assert_eq!(
            sin,
            vec![("Beto".to_string(), "Instituto GrafoLuna".to_string(), 2024)]
        );
        assert_eq!(con, sin);

        // Los pines reales de la consulta: barrido 12 (10 get_edge + 2
        // get_node) contra índice 3 (rango gratis + 1 get_edge + 2 get_node).
        assert_eq!(coste_sin, 12);
        assert_eq!(coste_con, 3);
        assert!(coste_con < coste_sin);

        // El ahorro escala con la selectividad del rango: 2023 (2 candidatas:
        // la 184 Iris y la 185 Beto) → 14 contra 6; 1800 (el rango lo cubre
        // TODO) → 30 = 30: un índice cuyo rango no poda nada no paga nada.
        let (sin_23, c_sin_23) = afiliaciones_globales_desde_anio(&store, 2023);
        let (con_23, c_con_23) = afiliaciones_globales_desde_anio_con_indice(&indice, &store, 2023);
        assert_eq!(sin_23, con_23);
        assert_eq!((c_sin_23, c_con_23), (14, 6));

        let (sin_1800, c_sin_1800) = afiliaciones_globales_desde_anio(&store, 1800);
        let (con_1800, c_con_1800) =
            afiliaciones_globales_desde_anio_con_indice(&indice, &store, 1800);
        assert_eq!(sin_1800, con_1800);
        assert_eq!(sin_1800.len(), 10);
        assert_eq!((c_sin_1800, c_con_1800), (30, 30));
    }

    /// LA LECCIÓN HONESTA DEL CAPÍTULO: el índice simple NO abarata la consulta
    /// AS OF persona-céntrica del cap-43 — el índice no casa con la consulta.
    /// `afiliaciones_vigentes_en` arranca por `in_edges` del proyecto y sigue
    /// las `MEMBER_OF` de 3 personas; el índice clave por `desde_anio` no
    /// filtra por PERSONA, que es el eje de la pregunta. El intento de usarlo
    /// se mide: para 2023 sus candidatas son [184, 185] — la 184 es de Iris
    /// (ajena al proyecto) y la 185 (Beto) aún NO está vigente en 2023; las
    /// tres vigentes del proyecto (52 Ana, 53 Beto, 55 Dani) tienen
    /// `desde < 2023`: NI SIQUIERA están en el rango del índice. El segundo
    /// salto lee las mismas 4 `MEMBER_OF` candidatas de las 3 personas con o
    /// sin índice: **28 = 28**.
    ///
    /// Y el coste de MANTENIMIENTO se cobra igual: construir el índice son 10
    /// lecturas ([`IndiceDesdeAnio::coste_construccion`]) que NADIE recupera —
    /// la factura total con índice es 28 + 10 = 38 contra 28 sin él.
    #[test]
    fn el_indice_simple_no_abaratara_el_as_of_persona_centrica() {
        let (store, _esquema) = kb_lira_paso4();

        // La consulta del cap-43, SIN índice: 1 in_edges + 21 get_edge + 6
        // get_node = 28 (los pines del ledger de la pieza 3, intactos en el
        // paso-4: orcid no añade aristas).
        let (filas_sin, coste_sin) = afiliaciones_vigentes_en(&store, "Proyecto Kira", 2023);
        assert_eq!(coste_sin.in_edges, 1);
        assert_eq!(coste_sin.get_edge, 21);
        assert_eq!(coste_sin.get_node, 6);
        assert_eq!(coste_sin.total(), 28);

        // El índice se construye (10 lecturas: las 10 MEMBER_OF del barrido).
        let indice = IndiceDesdeAnio::construir(&store, "MEMBER_OF", "desde_anio");
        assert_eq!(indice.coste_construccion(), 10);

        // El intento honesto de usarlo, medido: el índice responde por AÑO
        // (desde_anio >= anio), no por PERSONA. Su rango para 2023 son las
        // candidatas [184, 185]: NINGUNA de las vigentes del proyecto en 2023
        // (52, 53, 55 — todas con desde < 2023) aparece; la única del proyecto
        // (185) no está vigente en 2023. El índice no sustituye NI UNA lectura
        // del segundo salto: las 4 MEMBER_OF candidatas de Ana/Beto/Dani se
        // leen igual.
        assert_eq!(indice.aristas_desde(2023), vec![184, 185]);

        // La consulta CON el índice construido cuesta EXACTAMENTE lo mismo:
        // 28 = 28 — el índice simple no filtra por la persona, que es el eje
        // de la pregunta. Un índice que no se usa solo cuesta su mantenimiento.
        let (filas_con, coste_con) = afiliaciones_vigentes_en(&store, "Proyecto Kira", 2023);
        assert_eq!(filas_con, filas_sin);
        assert_eq!(coste_con, coste_sin);
        assert_eq!((coste_sin.total(), coste_con.total()), (28, 28));

        // La factura del mantenimiento sin uso: 10 lecturas de construcción +
        // 1 por cada MEMBER_OF nueva, con cero ahorro a cambio — 28 + 10 = 38
        // contra 28 de la consulta desnuda.
        assert_eq!(indice.coste_construccion() + coste_con.total(), 38);
    }
}

// ─────────────────── Índice por label: la adyacencia por TIPO (el CSR del cap. 14 al modelo) ───────────────────

/// Índice de ADYACENCIA por etiqueta: para cada `rel_tipo` de arista, la lista
/// de sus ids. Es el CSR lógico del Vol.II cap-14 — la adyacencia particionada
/// por tipo — traído al modelo: una consulta que pide las aristas de UN tipo
/// la resuelve en O(1) ([`IndicePorLabel::aristas_de_tipo`]) en vez de barrer
/// las 158 aristas del store filtrando por label.
///
/// `construido` cuenta el coste de construcción en LECTURAS (la moneda de la
/// pieza): TODAS las aristas consumidas de [`GraphStore::iter_edges`] — 158 en
/// KB-Lira paso-4, sean del tipo que sean (el filtro por tipo vive en el mapa,
/// no en la iteración).
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub struct IndicePorLabel {
    mapa: HashMap<String, Vec<EdgeId>>,
    construido: usize,
}

impl IndicePorLabel {
    /// Barre `iter_edges` UNA vez (158 lecturas en KB-Lira paso-4): agrupa
    /// cada arista por su `rel_tipo`. Dentro de un tipo, las ids quedan en
    /// orden de iteración (el orden por id de `MemoryStore`, determinista).
    pub fn construir(store: &dyn GraphStore) -> Self {
        let mut mapa: HashMap<String, Vec<EdgeId>> = HashMap::new();
        let mut construido: usize = 0;
        for ar in store.iter_edges() {
            construido += 1;
            mapa.entry(ar.label.clone()).or_default().push(ar.id);
        }
        Self { mapa, construido }
    }

    /// Las ids de las aristas de `rel_tipo`, sin leer NADA del store: `None`
    /// si el tipo no tiene aristas. El acceso O(1) por tipo que la consulta
    /// del cap-43 no tenía — barría `in_edges`/`out_edges` (la adyacencia
    /// COMPLETA, con las aristas de todos los tipos) y descartaba después.
    pub fn aristas_de_tipo(&self, rel_tipo: &str) -> Option<&[EdgeId]> {
        self.mapa.get(rel_tipo).map(Vec::as_slice)
    }

    /// El coste de construcción en lecturas: TODAS las aristas del barrido
    /// (158 en KB-Lira paso-4), hayan entrado en el mapa o no.
    pub fn coste_construccion(&self) -> usize {
        self.construido
    }
}

/// La MISMA consulta persona-céntrica del cap-43 ([`afiliaciones_vigentes_en`])
/// reescrita sobre [`IndicePorLabel`]: cada salto usa
/// [`IndicePorLabel::aristas_de_tipo`] y solo lee las aristas del tipo que el
/// salto necesita — nunca la adyacencia completa del proyecto ni de las
/// personas.
///
/// Devuelve `(filas, lecturas)` con `filas = (nombre_persona, nombre_org)` —
/// las MISMAS filas que el cap-43, en el mismo orden (clave natural por
/// persona) — y `lecturas` el coste TOTAL en get_edge + get_node (no hay
/// expansiones que contar: el índice ya no barre adyacencias).
///
/// El precio REAL sobre KB-Lira paso-4 con «Proyecto Kira» en 2026
/// ([`ANIO_ACTUAL`]): **22** lecturas — 6 `get_edge` de TODAS las WORKED_ON
/// (3 de Kira + 3 de Oráculo/Brújula leídas y descartadas: el bucket del tipo
/// es GLOBAL, no por proyecto) + 3 `get_node` de las personas + 10 `get_edge`
/// de TODAS las MEMBER_OF (la vencida 53 sigue costando su lectura: el
/// intervalo `[desde, hasta)` cobra el otro lado) + 3 `get_node` de las
/// organizaciones — frente a las 28 del cap-43 (1 `in_edges` + 5 `get_edge`
/// entrantes con 2 MENTIONS + 3 `get_node` + 16 `get_edge` salientes con 12
/// no-MEMBER_OF + 3 `get_node`). El contrato anunciaba 16 (3 WORKED_ON + 3
/// personas + 10 MEMBER_OF); la medida REAL añade las 3 WORKED_ON de los
/// otros proyectos y los 3 `get_node` de las organizaciones: **22**.
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn afiliaciones_vigentes_en_con_indice_por_label(
    store: &dyn GraphStore,
    indice: &IndicePorLabel,
    proyecto: &str,
    anio: i64,
) -> (Vec<(String, String)>, usize) {
    let mut lecturas: usize = 0;
    let mut filas: Vec<(String, String)> = Vec::new();

    // 1) Localizar el proyecto por nombre: barrido previo, FUERA del ledger
    //    (misma convención que el cap-43: saber QUÉ entidad preguntamos es
    //    previo a la consulta).
    let proyecto_id = nodo_por_nombre(store, proyecto).expect("proyecto conocido en KB-Lira");

    // 2) `WORKED_ON` por el índice: TODAS las del tipo (6 en KB-Lira) pagan
    //    su get_edge; se quedan las del proyecto (3). La consulta del cap-43
    //    leía las 5 entrantes del proyecto (3 WORKED_ON + 2 MENTIONS del hub
    //    leídas y descartadas); el índice por label evita las MENTIONS y la
    //    expansión `in_edges`, pero cobra las WORKED_ON de los otros
    //    proyectos (Oráculo/Brújula), que comparten bucket.
    let mut personas: Vec<(usize, String)> = Vec::new();
    for eid in indice.aristas_de_tipo("WORKED_ON").into_iter().flatten() {
        let e = store.get_edge(*eid).expect("WORKED_ON del índice presente");
        lecturas += 1;
        if e.target != proyecto_id {
            continue;
        }
        let persona = store.get_node(e.source).expect("la persona existe");
        lecturas += 1;
        let Some(Value::String(nombre_persona)) = persona.props.get("nombre") else {
            continue;
        };
        personas.push((e.source, nombre_persona.clone()));
    }

    // 3) `MEMBER_OF` por el índice, en UN solo pase (el bucket NO se re-barre
    //    una vez por persona): solo las de las personas del proyecto y
    //    VIGENTES en `anio` responden. La consulta del cap-43 barría
    //    `out_edges` de cada persona — 16 aristas (4 MEMBER_OF + 12
    //    AUTHORED/CITES/… leídas y descartadas); el índice por label solo lee
    //    MEMBER_OF, y aun las ajenas al proyecto (6) se leen antes de
    //    descartarse: leer la etiqueta cuesta igual.
    for eid in indice.aristas_de_tipo("MEMBER_OF").into_iter().flatten() {
        let af = store.get_edge(*eid).expect("MEMBER_OF del índice presente");
        lecturas += 1;
        let Some((_, nombre_persona)) = personas.iter().find(|(id, _)| *id == af.source) else {
            continue;
        };
        if !arista_vigente_en(af, anio) {
            continue;
        }
        let org = store.get_node(af.target).expect("la organizacion existe");
        lecturas += 1;
        let Some(Value::String(nombre_org)) = org.props.get("nombre") else {
            continue;
        };
        filas.push((nombre_persona.clone(), nombre_org.clone()));
    }

    // Orden natural por nombre de persona (el mismo criterio que el cap-43).
    filas.sort_by_key(|a| clave_orden(&a.0));
    (filas, lecturas)
}

/// Clave de orden «natural» para la prosa: minúsculas y sin diacríticos.
///
/// Réplica local de la `clave_orden` PRIVADA del cap-43 (que a su vez replica
/// la del cap-41): las piezas incrementalistas no comparten helpers privados
/// hasta la integración. Sin ella, el `sort` de bytes de Rust pondría
/// «Índices» DESPUÉS de «Z».
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

#[cfg(test)]
mod tests_indice_por_label {
    use super::*;

    /// LA DEUDA DEL CAP-43, COBRADA: la pieza 6 demostró que el índice por
    /// AÑO no abarata la consulta persona-céntrica (28 = 28) porque su clave
    /// (`desde_anio`) no es el eje de la pregunta. El índice por LABEL SÍ:
    /// la consulta pregunta por TIPO (`WORKED_ON`/`MEMBER_OF`) y esa ES su
    /// clave. «El cap. 44 cambiará ese precio»: **28 → 22** lecturas sobre
    /// la misma pregunta, con el MISMO resultado.
    ///
    /// Desglose REAL del AS OF de «Proyecto Kira» en 2026 (`ANIO_ACTUAL`, el
    /// «hoy» de KB-Lira — donde la MEMBER_OF 53 de Beto ya está VENCIDA):
    ///
    /// - Cap-43 (28): 1 `in_edges` (expansión entrante de Kira) + 5
    ///   `get_edge` (las 5 entrantes: 3 WORKED_ON + 2 MENTIONS del hub leídas
    ///   y descartadas) + 3 `get_node` (Ana, Beto, Dani) + 16 `get_edge` (las
    ///   16 salientes de las 3 personas: 4 MEMBER_OF + 12 AUTHORED/CITES/…
    ///   leídas y descartadas) + 3 `get_node` (las organizaciones).
    /// - Con índice por label (22): 6 `get_edge` (TODAS las WORKED_ON del
    ///   grafo: 3 de Kira + 3 de Oráculo/Brújula leídas y descartadas) + 3
    ///   `get_node` (personas) + 10 `get_edge` (TODAS las MEMBER_OF — la
    ///   vencida 53 sigue costando su lectura: el intervalo `[desde, hasta)`
    ///   cobra el otro lado) + 3 `get_node` (organizaciones).
    ///
    /// El contrato anunciaba 16 (3 WORKED_ON + 3 personas + 10 MEMBER_OF); la
    /// medida REAL añade las 3 WORKED_ON de los otros proyectos (el bucket
    /// del tipo es GLOBAL, no por proyecto) y los 3 `get_node` de las
    /// organizaciones (las filas necesitan el nombre): **22** < 28.
    #[test]
    fn el_indice_por_label_abaratara_el_as_of_del_cap43() {
        let (store, _esquema) = kb_lira_paso4();

        // La consulta del cap-43, SIN índice: los pines del ledger de la
        // pieza 3, intactos en el paso-4 (orcid no añade aristas).
        let (filas_sin, coste_sin) = afiliaciones_vigentes_en(&store, "Proyecto Kira", ANIO_ACTUAL);
        assert_eq!(coste_sin.in_edges, 1);
        assert_eq!(coste_sin.get_edge, 21);
        assert_eq!(coste_sin.get_node, 6);
        assert_eq!(coste_sin.total(), 28);

        // El índice se construye en 158 lecturas: TODAS las aristas del
        // paso-4 agrupadas por label (el filtro por tipo vive en el mapa, no
        // en la iteración). Los buckets de los dos tipos del salto, en orden
        // de iteración (determinista en MemoryStore).
        let indice = IndicePorLabel::construir(&store);
        assert_eq!(indice.coste_construccion(), 158);
        assert_eq!(
            indice.aristas_de_tipo("WORKED_ON").map(|ids| ids.to_vec()),
            Some(vec![58, 59, 60, 61, 62, 63])
        );
        assert_eq!(
            indice.aristas_de_tipo("MEMBER_OF").map(|ids| ids.to_vec()),
            Some(vec![52, 53, 54, 55, 56, 57, 182, 183, 184, 185])
        );

        // La consulta CON el índice por label: el resultado es IDÉNTICO al
        // del cap-43 (en 2026, Beto ya no responde Neurónica — la 53 venció
        // en 2024 — sino GrafoLuna por la 185)…
        let (filas_con, coste_con) = afiliaciones_vigentes_en_con_indice_por_label(
            &store,
            &indice,
            "Proyecto Kira",
            ANIO_ACTUAL,
        );
        assert_eq!(
            filas_sin,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto GrafoLuna".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ]
        );
        assert_eq!(filas_con, filas_sin);

        // …y el coste REAL baja: 22 < 28. El índice por label cambia el
        // precio que la pieza 6 declaraba imbatible para el índice por año.
        assert_eq!(coste_con, 22);
        assert!(coste_con < coste_sin.total());

        // La factura HONESTA del mantenimiento (como en la pieza 6): el
        // índice se paga al construir (158 lecturas) y se cobra en cada
        // consulta. Una consulta única es más cara (158 + 22 = 180 > 28); el
        // índice se amortiza cuando la pregunta se REPITE: 158 + 22n < 28n a
        // partir de n = 27 (158/6 ≈ 26,3) — frente al caso opuesto de la
        // pieza 6, donde el índice por año no se amortiza nunca (28 = 28).
        assert_eq!(indice.coste_construccion() + coste_con, 180);
        let n = 27;
        assert!(indice.coste_construccion() + coste_con * n < coste_sin.total() * n);
    }
}

// ─────────────────── Selectividad: el catálogo decide cuándo un índice estorba ───────────────────

/// La selectividad de una propiedad por label:
/// `(nodos_con_label, valores_distintos, valores_distintos / nodos_con_label)`.
///
/// Es la métrica del catálogo del cap-21 (Vol.II) aplicada a KB-Lira: el
/// optimizador ya estimaba cardinalidades por etiqueta para elegir el plan, y
/// la selectividad ES lo que ese catálogo mide cuando decide si un índice
/// merece existir. Dos lecturas del ratio, ambas útiles:
///
/// - `1,0` (o cerca): la propiedad es (casi) única — un índice por valor
///   devuelve UN nodo. UNIQUE ≡ índice, la lección estructural de la pieza 3:
///   la unicidad YA es un índice, no necesita otro.
/// - Baja: pocos valores distintos compartidos por muchos nodos — un índice
///   por valor devuelve casi TODO el label: no discrimina y solo cobra
///   mantenimiento. Es el caso del `anio` de los documentos.
///
/// Recorre `iter_nodes` en UN barrido (como [`verificar_esquema`]): cuenta los
/// nodos del label y, entre los que llevan la propiedad, los valores distintos
/// con un `HashSet` de la clave TEXTUAL — `Value` no deriva `Hash` (precedente
/// pieza 1: la regla `Unicidad` usa un `HashMap<String, NodeId>` por la misma
/// razón).
#[allow(dead_code)] // semilla de contrato: la consumen los tests de ESTA pieza
pub fn selectividad_de_propiedad(
    store: &dyn GraphStore,
    label: &str,
    propiedad: &str,
) -> (usize, usize, f64) {
    let mut nodos_con_label: usize = 0;
    let mut valores_distintos: HashSet<String> = HashSet::new();
    for nodo in store.iter_nodes() {
        if !nodo.has_label(label) {
            continue;
        }
        nodos_con_label += 1;
        if let Some(valor) = nodo.props.get(propiedad) {
            valores_distintos.insert(clave_textual(valor));
        }
    }
    let selectividad = if nodos_con_label == 0 {
        0.0
    } else {
        valores_distintos.len() as f64 / nodos_con_label as f64
    };
    (nodos_con_label, valores_distintos.len(), selectividad)
}

/// Clave textual de un [`Value`] para el `HashSet` de la selectividad (el
/// `Value` del cap-7 no deriva `Hash`; el prefijo conserva el tipo: un `Int(1)`
/// no colisiona con un `String("1")`).
fn clave_textual(valor: &Value) -> String {
    match valor {
        Value::Null => "<null>".into(),
        Value::Bool(b) => format!("b:{b}"),
        Value::Int(i) => format!("i:{i}"),
        Value::Float(f) => format!("f:{f}"),
        Value::String(s) => format!("s:{s}"),
        Value::Bytes(b) => format!("bytes:{}", b.len()),
    }
}

#[cfg(test)]
mod tests_selectividad {
    use super::*;

    /// La sección 3 del outline — «cuándo ayudan y cuándo estorban»: el
    /// catálogo decide ANTES de construir. Los tres ratios REALES del paso-4:
    ///
    /// - `Documento.anio` → (36, 5, ≈0,14): 36 documentos (los 12 del paso-1
    ///   con 2021-2025 y los 24 del lote del cap-42 con 2024/2025) comparten
    ///   SOLO 5 años. Selectividad BAJA = un índice SOLO por `anio` devuelve
    ///   ~7 documentos por valor de media: no discrimina. Es la explicación
    ///   de la pieza 7 ([`IndiceDesdeAnio`] costaba 10 lecturas de
    ///   construcción y NO abarataba el AS OF del cap-43): los `desde_anio`
    ///   están repartidos por 5 años, pero la consulta persona-céntrica no
    ///   filtra por ese eje — y el caso P10 (citas por año+tema) tampoco se
    ///   beneficia de un índice SOLO por `anio`: su otro eje es `tema`. La
    ///   decisión del capítulo es NO construirlo: cuesta mantenimiento y no
    ///   discrimina — el catálogo del cap-21 lo habría sabido ANTES.
    /// - `Persona.orcid` → (9, 9, 1,0): nueve personas, nueve orcid distintos.
    ///   UNIQUE ≡ índice perfecto (conexión con la pieza 3: la unicidad YA es
    ///   un índice; declararla como regla no necesita índice físico).
    /// - `Tema.nombre` → (9, 8, ≈0,89): NUEVE temas pero OCHO nombres: el
    ///   subtema 61 «memoria de agentes» del refactor A (cap-42) DUPLICA el
    ///   tema 26 del paso-1 — el residuo del antipatrón que el catálogo
    ///   delata como duplicado, no como índice perdido.
    #[test]
    fn la_selectividad_del_catalogo_decide_cuando_un_indice_estorba() {
        let (store, _esquema) = kb_lira_paso4();

        // (a) anio: selectividad BAJA (0,14) — el índice estorba: casi todos
        // los documentos comparten pocos años, un índice por valor no acota.
        let (docs, anios, sel_anio) = selectividad_de_propiedad(&store, "Documento", "anio");
        assert_eq!((docs, anios), (36, 5));
        assert!((sel_anio - 5.0 / 36.0).abs() < 1e-9);

        // (b) orcid: selectividad 1,0 — UNIQUE ≡ índice perfecto.
        let (personas, orcids, sel_orcid) = selectividad_de_propiedad(&store, "Persona", "orcid");
        assert_eq!((personas, orcids), (9, 9));
        assert_eq!(sel_orcid, 1.0);

        // (c) nombre: 1,0 salvo el duplicado «memoria de agentes» — 0,89.
        let (temas, nombres, sel_nombre) = selectividad_de_propiedad(&store, "Tema", "nombre");
        assert_eq!((temas, nombres), (9, 8));
        assert!((sel_nombre - 8.0 / 9.0).abs() < 1e-9);
    }
}

// ─────────────── Red de seguridad cuádruple: el esquema no cambia NINGUNA respuesta vieja ───────────────

/// Regresión de `kb_lira_paso4` (el orcid + el esquema) sobre las respuestas
/// de los caps. 41 (paso-1), 42 (paso-2 refactorizado) y 43 (paso-3
/// temporal): añadir `orcid` a las personas y DECLARAR el modelo como
/// esquema NO puede cambiar ninguna respuesta vieja — el orcid es una
/// propiedad nueva que ninguna pregunta lee y el esquema es datos, no
/// mutación.
///
/// Mecanismo de comparación IDÉNTICO al de los caps. 42-43
/// (`tests_regresion_preguntas_paso1` y `tests_regresion_temporalidad`): cada
/// pregunta se ejecuta sobre el store nuevo (paso-4) y su respuesta se FILTRA
/// al subgrafo paso-1 resolviendo el id del nodo (`titulo`/`nombre`) en el
/// store — las filas del lote/valid-time no participan en el contrato de esta
/// pieza. En P4 la fila Beto→GrafoLuna (la mudanza temporal del cap-43) se
/// filtra resolviendo el id de la organización; en P8 (conteos SIN ids) el
/// subgrafo se recalcula contando solo las `AUTHORED` a documentos con id < 30.
#[cfg(test)]
mod tests_regresion_esquema {
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
    use crate::cap42_antipatrones::validar_modelo_kb_lira_paso2;
    use crate::cap43_temporalidad::afiliaciones_actuales;
    use crate::cap43_temporalidad::informe_temporal_reproducible;
    use crate::cap43_temporalidad::nota_de_resena_vigente_en;
    use crate::cap43_temporalidad::validar_modelo_kb_lira_paso3;

    /// Primer id del lote: el subgrafo paso-1 ocupa los ids 0..30.
    const LOTE_INICIO: usize = 30;

    /// ¿El valor `campo` (`titulo`/`nombre`) pertenece a un nodo del subgrafo
    /// paso-1 (id < 30) en `store`? Copia del helper del cap-42 (replicado
    /// también en el cap-43).
    fn es_paso1(store: &dyn GraphStore, campo: &str, valor: &str) -> bool {
        store.iter_nodes().any(|n| {
            n.id < LOTE_INICIO && n.props.get(campo) == Some(&Value::String(valor.to_string()))
        })
    }

    /// P8 restringida al subgrafo paso-1: `pregunta_08` devuelve (nombre,
    /// conteo) SIN ids, y el lote añade AUTHORED desde personas del paso-1
    /// (Fabio→51-53, Elena→54-56) — recontar solo las AUTHORED a documentos
    /// con id < 30 es la única forma de comparar el subgrafo paso-1 sin
    /// contaminación del lote. Copia del helper del cap-42 (replicado en el
    /// cap-43).
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
    /// contaminación temporal. Copia del helper del cap-43.
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

    /// Las 10 preguntas del cap-41 responden IDÉNTICO sobre el subgrafo
    /// paso-1 (ids 0-29) DESPUÉS de añadir el orcid y declarar el esquema.
    /// El paso-4 solo ENRIQUECE el modelo con la prop `orcid` (9 personas) y
    /// con el esquema como DATOS: ninguna pregunta del cap-41 la lee, y las
    /// filas del lote/valid-time se filtran resolviendo el id del nodo
    /// (titulo/nombre) en el store paso-4.
    #[test]
    fn las_10_preguntas_del_paso1_no_cambian_tras_anadir_esquema() {
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

        // ── Store paso-4: el paso-3 del cap-43 + el orcid de este capítulo ──
        let paso4 = kb_lira_paso4().0;

        // ── Las mismas 10 preguntas sobre el store con esquema ──
        let a1 = q01(&paso4, "Ana");
        let a2 = q02(&paso4, "Memoria episódica en LLMs");
        let (a3a, a3b) = q03(&paso4, "Supernodos: anatomía de un cuello de botella");
        let a4 = q04(&paso4, "Proyecto Kira");
        let (a5a, a5b) = q05(
            &paso4,
            "Informe anual del Proyecto Kira",
            "grafos de conocimiento",
        );
        let a6 = q06(&paso4, "Instituto Neurónica");
        let a7 = q07(&paso4, "Ana", "Beto");
        let a9 = q09(&paso4, "Ana", "Beto");
        let a10 = q10(
            &paso4,
            "Grafos de conocimiento para agentes",
            "grafos de conocimiento",
            2023,
        );

        // ── Comparación SOLO sobre el subgrafo paso-1 (ids < 30) ──
        let solo_titulos = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&paso4, "titulo", t))
                .cloned()
                .collect()
        };
        let solo_nombres = |v: &[String]| -> Vec<String> {
            v.iter()
                .filter(|t| es_paso1(&paso4, "nombre", t))
                .cloned()
                .collect()
        };
        let solo_paso1_citas = |v: &[(String, i64)]| -> Vec<(String, i64)> {
            v.iter()
                .filter(|(t, _)| es_paso1(&paso4, "titulo", t))
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
            solo_afiliaciones_paso1(&paso4, &a4),
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
            publicaciones_por_persona_en_paso1(&paso4),
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
    /// ejecutados sobre el paso-4 ENTERO (sin filtros): el orcid y el esquema
    /// no tocan AUTHORED/CITES/ABOUT/MENTIONS/WORKED_ON, así que las
    /// respuestas del cap-42 se mantienen EXACTAS sobre el modelo con
    /// esquema. Réplica del test homónimo del cap-43, un nivel más arriba.
    #[test]
    fn las_respuestas_del_paso2_no_cambian_tras_anadir_esquema() {
        let store = kb_lira_paso4().0;

        // P1: los 4 documentos de Ana — Ana no publicó en el lote.
        assert_eq!(
            q01(&store, "Ana"),
            vec![
                "Grafos de conocimiento para agentes",
                "Índices adaptativos para grafos",
                "Notas de la reunión de arranque",
                "Supernodos: anatomía de un cuello de botella",
            ],
            "P1(Ana): idéntico al paso-2 — el orcid no toca AUTHORED"
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
        // (6 paso-1 + 18 del lote) — el orcid no toca ABOUT/SUB_TEMA_DE.
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

    /// Los valores pineados del contrato cap-43 (la dimensión tiempo)
    /// ejecutados sobre el paso-4 ENTERO: el orcid y el esquema no tocan las
    /// `MEMBER_OF` (ni su valid-time) ni la navegación SOBRE/REALIZA/
    /// CONTRARRESTA de las reseñas, así que las respuestas del cap-43 se
    /// mantienen EXACTAS. Réplica de los tests `tests_as_of`,
    /// `tests_resena_vigente` y `tests_informe_temporal` del cap-43, sobre el
    /// modelo con esquema.
    #[test]
    fn las_respuestas_del_paso3_no_cambian_tras_anadir_esquema() {
        let store = kb_lira_paso4().0;

        // Afiliaciones ACTUALES de «Proyecto Kira» (2026, el «hoy» de
        // KB-Lira): Beto ya NO está en Neurónica — la 185 (GrafoLuna) manda.
        let (actuales, _) = afiliaciones_actuales(&store, "Proyecto Kira");
        assert_eq!(
            actuales,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto GrafoLuna".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ],
            "afiliaciones actuales: idénticas al paso-3 — el orcid no toca MEMBER_OF"
        );

        // AS OF 2023: la 185 aún no existe y la 53 (Beto→Neurónica,
        // [2018, 2024)) está vigente → Beto responde Neurónica.
        let (as_of_2023, _) = afiliaciones_vigentes_en(&store, "Proyecto Kira", 2023);
        assert_eq!(
            as_of_2023,
            vec![
                ("Ana".to_string(), "Universidad de Lira".to_string()),
                ("Beto".to_string(), "Instituto Neurónica".to_string()),
                ("Dani".to_string(), "Instituto Neurónica".to_string()),
            ],
            "AS OF 2023: idéntico al paso-3 — la validez no cambia con el esquema"
        );

        // La reseña vigente del Informe de revisión por pares: ronda 1 (nota
        // 7) hasta que su REALIZA 149 caduca en 2025; la ronda 2 (nota 8) la
        // contrarresta desde entonces.
        let titulo_informe = "Informe de revisión por pares 2025";
        assert_eq!(
            nota_de_resena_vigente_en(&store, titulo_informe, 2024),
            Some(7),
            "la reseña vigente en 2024 es la ronda 1 (nota 7)"
        );
        assert_eq!(
            nota_de_resena_vigente_en(&store, titulo_informe, 2025),
            Some(8),
            "la reseña vigente en 2025 es la ronda 2 (nota 8, contrarresta)"
        );

        // El informe temporal es byte a byte el del paso-3: el orcid no llega
        // a ninguna de sus lecturas (tabla AS OF + caso Dani bitemporal, que
        // se alimenta del histórico del cap-43, no de las props de persona).
        // Comparar las DOS salidas reales evita duplicar el literal pineado
        // del cap-43 y define el contrato exacto de ESTA pieza: sin cambios.
        assert_eq!(
            informe_temporal_reproducible(&store),
            informe_temporal_reproducible(&kb_lira_paso3()),
            "el informe temporal del paso-4 es byte a byte el del paso-3"
        );
    }

    /// El modelo canónico del paso-4 cumple el contrato de TODOS los pasos a
    /// la vez: su propio esquema ([`verificar_esquema`] + [`esquema_kb_lira`],
    /// el canónico que la pieza 2 ya comprueba) y la cadena de validadores
    /// que el paso-4 hereda — el validador del paso-2 (cap-42) y el del
    /// paso-3 (cap-43). El validador del cap-41 NO se aplica directamente:
    /// su contrato es solo el paso-1 y denunciaría los tipos de arista nuevos
    /// (SUB_TEMA_DE/SOBRE/REALIZA/CONTRARRESTA) que las reglas del paso-2 ya
    /// gobiernan — el paso-2 lo encadena filtrado a su alcance.
    #[test]
    fn el_esquema_acepta_el_modelo_canonico_de_todos_los_pasos() {
        let (store, esquema) = kb_lira_paso4();

        assert_eq!(
            verificar_esquema(&store, &esquema),
            Ok(()),
            "el esquema canónico acepta el modelo canónico del paso-4"
        );
        assert_eq!(
            validar_modelo_kb_lira_paso2(&store),
            Ok(()),
            "el validador del paso-2 (cap-42) acepta el modelo paso-4"
        );
        assert_eq!(
            validar_modelo_kb_lira_paso3(&store),
            Ok(()),
            "el validador del paso-3 (cap-43) acepta el modelo paso-4"
        );
    }
}

// ─────────────────── CSV determinista (mismo contrato que caps. 41-43) ───────────────────

/// Exporta los NODOS del paso-4 (con `orcid:String` en las 9 personas) al
/// formato CSV del cap. 32 (cabecera = unión BTreeMap de props; `:LABEL` con
/// labels unidas por `:`). Es el MISMO formato que
/// [`crate::cap41_modelado::csv_nodos_kb_lira`],
/// [`crate::cap42_antipatrones::csv_nodos_kb_lira_paso2`] y
/// [`crate::cap43_temporalidad::csv_nodos_kb_lira_paso3`]: el orcid entra en
/// la cabecera SOLO porque existe en los nodos Persona — el exportador
/// genérico del cap. 32 no conoce el esquema, la columna nace de los datos.
pub fn csv_nodos_kb_lira_paso4(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_nodos(store, &mut buf).expect("export nodos paso-4");
    String::from_utf8(buf).expect("CSV UTF-8")
}

/// Exporta las ARISTAS del paso-4 (mismo contrato que
/// [`csv_nodos_kb_lira_paso4`]). Las `MEMBER_OF` llevan `desde_anio:INT` y,
/// donde el intervalo está cerrado, `hasta_anio:INT` — idéntico al paso-3:
/// el orcid no toca ninguna arista.
pub fn csv_aristas_kb_lira_paso4(store: &dyn GraphStore) -> String {
    let mut buf: Vec<u8> = Vec::new();
    exportar_csv_aristas(store, &mut buf).expect("export aristas paso-4");
    String::from_utf8(buf).expect("CSV UTF-8")
}

// ─────────────────── CSV del esquema (formato propio, round-trip) ───────────────────

/// Exporta el [`Esquema`] a CSV: UNA regla por línea, primera columna el
/// nombre de la variante y después sus parámetros en el MISMO orden que los
/// campos de la variante (determinista, sin cabecera):
///
/// ```text
/// Extremos,AUTHORED,Persona,Documento
/// Extremos,MENIONS,Documento,Persona;Organizacion;Proyecto
/// Existencia,Documento,titulo
/// Tipo,Documento,anio,Int
/// Unicidad,Persona,orcid
/// SinSolape,MEMBER_OF,desde_anio,hasta_anio
/// IntervaloValido,MEMBER_OF,desde_anio,hasta_anio,2026
/// ```
///
/// La lista `hasta_labels` de `Extremos` se serializa con `;` como separador
/// interno (los labels de KB-Lira no llevan `;`); una lista de UN elemento va
/// sin separador. [`esquema_desde_csv`] es el inverso exacto (round-trip).
pub fn csv_esquema(esquema: &Esquema) -> String {
    let mut buf = String::new();
    for regla in &esquema.reglas {
        buf.push_str(&csv_regla(regla));
        buf.push('\n');
    }
    buf
}

/// Importa un CSV producido por [`csv_esquema`] (round-trip): una regla por
/// línea en el orden original (el orden del catálogo es parte del formato).
/// Líneas en blanco se saltan; cualquier otra desviación del formato
/// documentado en [`csv_esquema`] es un pánico con la línea implicada.
pub fn esquema_desde_csv(contenido: &str) -> Esquema {
    let mut esquema = Esquema::nueva();
    for (i, linea) in contenido.lines().enumerate() {
        let recortada = linea.trim_end_matches('\r');
        if recortada.is_empty() {
            continue; // línea en blanco: se salta
        }
        let campos: Vec<&str> = recortada.split(',').collect();
        esquema.reglas.push(regla_desde_csv(&campos, i + 1));
    }
    esquema
}

/// Una regla a su línea CSV (inverso de [`regla_desde_csv`]).
fn csv_regla(regla: &ReglaConstraint) -> String {
    match regla {
        ReglaConstraint::Extremos {
            rel_tipo,
            desde_label,
            hasta_labels,
        } => format!(
            "Extremos,{rel_tipo},{desde_label},{}",
            hasta_labels.join(";")
        ),
        ReglaConstraint::Existencia { label, propiedad } => {
            format!("Existencia,{label},{propiedad}")
        }
        ReglaConstraint::Tipo {
            label,
            propiedad,
            esperado,
        } => format!(
            "Tipo,{label},{propiedad},{}",
            tipo_esperado_nombre(esperado)
        ),
        ReglaConstraint::Unicidad { label, propiedad } => {
            format!("Unicidad,{label},{propiedad}")
        }
        ReglaConstraint::SinSolape {
            rel_tipo,
            desde_anio_prop,
            hasta_anio_prop,
        } => format!("SinSolape,{rel_tipo},{desde_anio_prop},{hasta_anio_prop}"),
        ReglaConstraint::IntervaloValido {
            rel_tipo,
            desde_prop,
            hasta_prop,
            anio_max,
        } => format!("IntervaloValido,{rel_tipo},{desde_prop},{hasta_prop},{anio_max}"),
    }
}

/// Una línea CSV a su regla (inverso de [`csv_regla`]). `n_linea` es la línea
/// del fichero (base 1) para el mensaje de error, igual que el importador del
/// cap. 43.
fn regla_desde_csv(campos: &[&str], n_linea: usize) -> ReglaConstraint {
    match campos[0] {
        "Extremos" => {
            assert_eq!(
                campos.len(),
                4,
                "fila {n_linea} (Extremos): se esperaban 4 campos: {campos:?}"
            );
            let hasta_labels: Vec<String> = campos[3]
                .split(';')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            ReglaConstraint::Extremos {
                rel_tipo: campos[1].into(),
                desde_label: campos[2].into(),
                hasta_labels,
            }
        }
        "Existencia" => {
            assert_eq!(
                campos.len(),
                3,
                "fila {n_linea} (Existencia): se esperaban 3 campos: {campos:?}"
            );
            ReglaConstraint::Existencia {
                label: campos[1].into(),
                propiedad: campos[2].into(),
            }
        }
        "Tipo" => {
            assert_eq!(
                campos.len(),
                4,
                "fila {n_linea} (Tipo): se esperaban 4 campos: {campos:?}"
            );
            let esperado = match campos[3] {
                "Int" => TipoEsperado::Int,
                "String" => TipoEsperado::String,
                otro => panic!("fila {n_linea} (Tipo): tipo esperado desconocido: {otro:?}"),
            };
            ReglaConstraint::Tipo {
                label: campos[1].into(),
                propiedad: campos[2].into(),
                esperado,
            }
        }
        "Unicidad" => {
            assert_eq!(
                campos.len(),
                3,
                "fila {n_linea} (Unicidad): se esperaban 3 campos: {campos:?}"
            );
            ReglaConstraint::Unicidad {
                label: campos[1].into(),
                propiedad: campos[2].into(),
            }
        }
        "SinSolape" => {
            assert_eq!(
                campos.len(),
                4,
                "fila {n_linea} (SinSolape): se esperaban 4 campos: {campos:?}"
            );
            ReglaConstraint::SinSolape {
                rel_tipo: campos[1].into(),
                desde_anio_prop: campos[2].into(),
                hasta_anio_prop: campos[3].into(),
            }
        }
        "IntervaloValido" => {
            assert_eq!(
                campos.len(),
                5,
                "fila {n_linea} (IntervaloValido): se esperaban 5 campos: {campos:?}"
            );
            let anio_max: i64 = campos[4].parse().expect("IntervaloValido anio_max entero");
            ReglaConstraint::IntervaloValido {
                rel_tipo: campos[1].into(),
                desde_prop: campos[2].into(),
                hasta_prop: campos[3].into(),
                anio_max,
            }
        }
        otro => panic!("fila {n_linea}: regla desconocida: {otro:?}"),
    }
}

// ─────────────────── Los tests de honestidad (pieza incremental) ───────────────────

#[cfg(test)]
mod tests_csv_paso4 {
    use super::*;
    use crate::cap32_import_export::{importar_csv_aristas, importar_csv_nodos};
    use std::io::BufReader;

    // Los tests llevan el nombre EXACTO del contrato; para llamar a las
    // funciones homónimas de cap-41/cap-42/cap-43 sin sombra, se re-importan
    // con alias (mismo patrón que los capítulos anteriores).
    use crate::cap41_modelado::{
        csv_aristas_kb_lira as csv_aristas_paso1, csv_nodos_kb_lira as csv_nodos_paso1,
        kb_lira_paso1,
    };
    use crate::cap42_antipatrones::{
        csv_aristas_kb_lira_paso2, csv_nodos_kb_lira_paso2, kb_lira_paso2_degrado,
    };
    use crate::cap43_temporalidad::{
        csv_aristas_kb_lira_paso3, csv_historico, csv_nodos_kb_lira_paso3, historico_kb_lira_paso3,
    };

    /// Exporta nodos+aristas del paso-4 → importa (cap. 32) → exporta de
    /// nuevo: bytes IDÉNTICOS (mismo patrón que el roundtrip del cap-41/42/43).
    /// El orcid sobrevive al viaje: si el importador lo perdiera, la cabecera
    /// de la segunda exportación carecería de la columna `orcid` y el
    /// byte-a-byte gritaría.
    #[test]
    fn csv_roundtrip_paso4_import_export_byte_a_byte() {
        let s = kb_lira_paso4().0;
        let nodos_v1 = csv_nodos_kb_lira_paso4(&s);
        let aristas_v1 = csv_aristas_kb_lira_paso4(&s);

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

        let nodos_v2 = csv_nodos_kb_lira_paso4(&s2);
        let aristas_v2 = csv_aristas_kb_lira_paso4(&s2);
        assert_eq!(nodos_v1, nodos_v2, "roundtrip nodos byte a byte");
        assert_eq!(aristas_v1, aristas_v2, "roundtrip aristas byte a byte");
    }

    /// esquema_kb_lira → CSV → importación → CSV: bytes IDÉNTICOS (el orden
    /// del catálogo y el separador `;` de `hasta_labels` son parte del
    /// formato, no se pierden en el viaje).
    #[test]
    fn csv_esquema_roundtrip_byte_a_byte() {
        let esquema = esquema_kb_lira();
        let v1 = csv_esquema(&esquema);

        let esquema2 = esquema_desde_csv(&v1);
        let v2 = csv_esquema(&esquema2);
        assert_eq!(v1, v2, "roundtrip esquema byte a byte");

        // El esquema reconstruido es IDÉNTICO también como datos: mismas 16
        // reglas en el mismo orden (la lista de labels de MENTIONS incluida).
        assert_eq!(esquema2.reglas, esquema.reglas);
    }

    /// datasets/kb-lira/paso-4/ ES la salida de los builders del paso-4:
    /// nodes.csv + edges.csv (formato cap. 32) y esquema.csv (formato de
    /// [`csv_esquema`]). Si alguien regenera el builder y olvida commitear,
    /// este test grita (mismo mecanismo que cap-41/42/43).
    #[test]
    fn csv_paso4_coincide_con_dataset_commiteado_byte_a_byte() {
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-4");
        let (store, esquema) = kb_lira_paso4();

        let esperado_nodos =
            std::fs::read_to_string(format!("{base}/nodes.csv")).expect("dataset nodes.csv");
        let esperado_aristas =
            std::fs::read_to_string(format!("{base}/edges.csv")).expect("dataset edges.csv");
        assert_eq!(csv_nodos_kb_lira_paso4(&store), esperado_nodos);
        assert_eq!(csv_aristas_kb_lira_paso4(&store), esperado_aristas);

        let esperado_esquema =
            std::fs::read_to_string(format!("{base}/esquema.csv")).expect("dataset esquema.csv");
        assert_eq!(csv_esquema(&esquema), esperado_esquema);
    }

    /// El paso-4 NO toca los datasets de los pasos anteriores: los ficheros
    /// commiteados de datasets/kb-lira/paso-1/, paso-2/ y paso-3/ siguen
    /// siendo la salida EXACTA de los builders del cap-41, cap-42 y cap-43
    /// (mismo patrón que `csv_paso1_y_paso2_intactos_tras_paso3` del cap-43).
    #[test]
    fn csv_pasos_anteriores_intactos_tras_paso4() {
        let base1 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-1");
        let base2 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-2");
        let base3 = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datasets/kb-lira/paso-3");

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
    }
}

// ─────────────────── Informe reproducible del capítulo (para la prosa) ───────────────────

/// El informe esquemático REPRODUCIBLE del capítulo: la tesis completa en
/// texto plano, SIN tiempos de ejecución — la moneda son las LECTURAS al
/// store —, para que la prosa del cap-44 lo cite tal cual.
///
/// Parte 1 — el catálogo: las reglas del esquema por tipo, contadas REALES
/// de [`esquema_kb_lira`] (`esquema.reglas`, nada hardcodeado).
///
/// Parte 2 — el caso ORCID: las 9 personas del paso-4 con orcid único
/// (selectividad 1,0 de [`selectividad_de_propiedad`] — UNIQUE ≡ índice, la
/// lección estructural de la pieza 3).
///
/// Parte 3 — el caso índice: [`IndiceDesdeAnio`] (10 lecturas de
/// construcción, [`IndiceDesdeAnio::coste_construccion`]) abarata la
/// consulta GLOBAL reescrita (12 → 3 en 2024,
/// [`afiliaciones_globales_desde_anio`] contra
/// [`afiliaciones_globales_desde_anio_con_indice`]) pero NO el AS OF
/// persona-céntrico del cap-43 (28 = 28 en 2023, [`afiliaciones_vigentes_en`],
/// y la factura del mantenimiento se cobra igual: + 10 = 38); [`IndicePorLabel`]
/// (158 lecturas) SÍ lo abarata: 28 → 22 en 2026
/// ([`afiliaciones_vigentes_en_con_indice_por_label`]).
///
/// Parte 4 — la selectividad del catálogo: los tres ratios REALES de
/// [`selectividad_de_propiedad`] (Documento.anio 5/36 ≈ 0,139 con el índice
/// NO construido, Persona.orcid 9/9 = 1,0 y Tema.nombre 8/9 ≈ 0,889, el
/// duplicado residual del subtema 61 frente al tema 26 que el propio store
/// delata).
pub fn informe_esquema_reproducible(store: &dyn GraphStore, esquema: &Esquema) -> String {
    let mut buf = String::new();
    buf.push_str("Esquema declarativo de KB-Lira (paso-4): las reglas como datos\n");

    // Parte 1: el catálogo — conteo REAL de reglas por tipo del `esquema`.
    let mut extremos = 0usize;
    let mut existencia = 0usize;
    let mut tipo = 0usize;
    let mut unicidad = 0usize;
    let mut sin_solape = 0usize;
    let mut intervalo_valido = 0usize;
    for regla in &esquema.reglas {
        match regla {
            ReglaConstraint::Extremos { .. } => extremos += 1,
            ReglaConstraint::Existencia { .. } => existencia += 1,
            ReglaConstraint::Tipo { .. } => tipo += 1,
            ReglaConstraint::Unicidad { .. } => unicidad += 1,
            ReglaConstraint::SinSolape { .. } => sin_solape += 1,
            ReglaConstraint::IntervaloValido { .. } => intervalo_valido += 1,
        }
    }
    let reglas = |n: usize| -> String { format!("{n} regla{}", if n == 1 { "" } else { "s" }) };

    // Parte 2: el caso ORCID — la selectividad REAL de Persona.orcid.
    let (personas, orcids, _) = selectividad_de_propiedad(store, "Persona", "orcid");

    // Parte 3: el caso índice — costes y efectos REALES sobre el store.
    let indice_anio = IndiceDesdeAnio::construir(store, "MEMBER_OF", "desde_anio");
    let (_, global_sin) = afiliaciones_globales_desde_anio(store, 2024);
    let (_, global_con) = afiliaciones_globales_desde_anio_con_indice(&indice_anio, store, 2024);
    let asof_2023 = afiliaciones_vigentes_en(store, "Proyecto Kira", 2023)
        .1
        .total();
    let candidatas_2023 = indice_anio.aristas_desde(2023);
    let indice_label = IndicePorLabel::construir(store);
    let asof_2026 = afiliaciones_vigentes_en(store, "Proyecto Kira", ANIO_ACTUAL)
        .1
        .total();
    let asof_2026_con = afiliaciones_vigentes_en_con_indice_por_label(
        store,
        &indice_label,
        "Proyecto Kira",
        ANIO_ACTUAL,
    )
    .1;

    // Parte 4: la selectividad — los tres ratios REALES del catálogo y el
    // duplicado residual que el propio store delata (los temas que comparten
    // `nombre`).
    let (docs, anios, _) = selectividad_de_propiedad(store, "Documento", "anio");
    let (temas, nombres, _) = selectividad_de_propiedad(store, "Tema", "nombre");
    let mut por_nombre: HashMap<String, Vec<usize>> = HashMap::new();
    for nodo in store.iter_nodes() {
        if nodo.has_label("Tema")
            && let Some(Value::String(nombre)) = nodo.props.get("nombre")
        {
            por_nombre.entry(nombre.clone()).or_default().push(nodo.id);
        }
    }
    let (nombre_duplicado, ids_duplicados) = por_nombre
        .iter()
        .find(|(_, ids)| ids.len() > 1)
        .map(|(n, ids)| (n.clone(), ids.clone()))
        .expect("el duplicado residual del cap-42 está en KB-Lira");

    // La tabla del capítulo (formato del informe del cap-43: columnas `|` y
    // separador `─` del ancho de la fila más larga).
    let mut filas: Vec<(String, String)> = Vec::new();
    filas.push(("Extremos".into(), reglas(extremos)));
    filas.push(("Existencia".into(), reglas(existencia)));
    filas.push(("Tipo".into(), reglas(tipo)));
    filas.push(("Unicidad".into(), reglas(unicidad)));
    filas.push(("SinSolape".into(), reglas(sin_solape)));
    filas.push(("IntervaloValido".into(), reglas(intervalo_valido)));
    filas.push(("Total".into(), reglas(esquema.reglas.len())));
    filas.push((
        "Caso ORCID".into(),
        format!(
            "{personas} personas con orcid único — selectividad {} (UNIQUE ≡ índice)",
            fmt_selectividad(personas, orcids)
        ),
    ));
    filas.push((
        "IndiceDesdeAnio · global 2024".into(),
        format!(
            "construcción {} lecturas — barrido {} → {} lecturas con el índice",
            indice_anio.coste_construccion(),
            global_sin,
            global_con
        ),
    ));
    filas.push((
        "IndiceDesdeAnio · AS OF 2023".into(),
        format!(
            "{asof_2023} = {asof_2023} lecturas: sin mejora — + {} de mantenimiento = {}",
            indice_anio.coste_construccion(),
            asof_2023 + indice_anio.coste_construccion()
        ),
    ));
    filas.push((
        "IndicePorLabel · AS OF 2026".into(),
        format!(
            "construcción {} lecturas — {asof_2026} → {asof_2026_con} lecturas con el índice",
            indice_label.coste_construccion()
        ),
    ));
    filas.push((
        "Selectividad Documento.anio".into(),
        format!(
            "{} — índice NO construido (no discrimina)",
            fmt_selectividad(docs, anios)
        ),
    ));
    filas.push((
        "Selectividad Persona.orcid".into(),
        fmt_selectividad(personas, orcids),
    ));
    filas.push((
        "Selectividad Tema.nombre".into(),
        format!(
            "{} — duplicado residual: los temas {} comparten «{nombre_duplicado}»",
            fmt_selectividad(temas, nombres),
            ids_duplicados
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(" y ")
        ),
    ));

    let ancho_etiqueta = filas.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let filas_texto: Vec<String> = filas
        .iter()
        .map(|(l, d)| format!("{l:<ancho_etiqueta$} | {d}"))
        .collect();
    let ancho = filas_texto.iter().map(String::len).max().unwrap_or(0);
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');
    for fila in &filas_texto {
        buf.push_str(fila);
        buf.push('\n');
    }
    buf.push_str(&"─".repeat(ancho));
    buf.push('\n');

    // Las dos lecciones que cierran el capítulo, con los números de arriba.
    buf.push_str(&format!(
        "El índice por AÑO casa con la consulta GLOBAL reescrita (su filtro es la clave del índice), \
         pero no con el AS OF persona-céntrico: para 2023 su rango devuelve {candidatas_2023:?} y \
         ninguna de las vigentes del proyecto (52, 53, 55) está en él — el índice responde por AÑO, \
         no por persona, y el mantenimiento ({}) se cobra igual.\n",
        indice_anio.coste_construccion()
    ));
    buf.push_str(&format!(
        "El índice por LABEL sí casa: la pregunta del cap-43 es por TIPO (WORKED_ON, MEMBER_OF) y esa \
         es su clave — {asof_2026} → {asof_2026_con} lecturas. Su construcción ({} lecturas) solo se \
         amortiza si la pregunta se REPITE: {} + {}n < {}n desde n = {}.\n",
        indice_label.coste_construccion(),
        indice_label.coste_construccion(),
        asof_2026_con,
        asof_2026,
        indice_label.coste_construccion().div_ceil(asof_2026 - asof_2026_con)
    ));
    buf
}

/// Selectividad formateada para la prosa: cociente con coma decimal española
/// y los ceros finales recortados (`1,0`, no `1,000`); `=` cuando el cociente
/// es exacto y `≈` cuando redondea (`5/36 ≈ 0,139`).
fn fmt_selectividad(nodos: usize, valores: usize) -> String {
    let ratio = valores as f64 / nodos as f64;
    let tres_decimales = format!("{ratio:.3}");
    let mut texto = tres_decimales.clone();
    while texto.ends_with('0') {
        texto.pop();
    }
    if texto.ends_with('.') {
        texto.push('0');
    }
    // `=` solo si el redondeo a tres decimales es EXACTO (9/9 = 1,0); si no,
    // `≈` (5/36 y 8/9 no son exactos en tres decimales).
    let es_exacto = tres_decimales
        .parse::<f64>()
        .expect("tres decimales de un f64 siempre parsean")
        == ratio;
    format!(
        "{valores}/{nodos} {} {}",
        if es_exacto { "=" } else { "≈" },
        texto.replace('.', ",")
    )
}

#[cfg(test)]
mod tests_informe_esquema {
    use super::*;

    /// El informe es REPRODUCIBLE: `informe_esquema_reproducible` produce una
    /// salida estable byte a byte (el catálogo de reglas, el caso ORCID, el
    /// caso índice y la selectividad del cap-44, todo con valores REALES) —
    /// el literal de abajo es la salida REAL fijada a mano.
    #[test]
    fn informe_esquema_reproducible_sobre_kb_lira() {
        let (store, esquema) = kb_lira_paso4();
        let reporte = informe_esquema_reproducible(&store, &esquema);
        // La salida REAL fijada a mano. `r#"..."#` con saltos de línea
        // literales: las filas de la tabla llevan sangría de ALINEACIÓN propia
        // (el estilo `\n\` del cap-42 la recortaría).
        let esperado = r#"Esquema declarativo de KB-Lira (paso-4): las reglas como datos
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
Extremos                       | 6 reglas
Existencia                     | 5 reglas
Tipo                           | 2 reglas
Unicidad                       | 1 regla
SinSolape                      | 1 regla
IntervaloValido                | 1 regla
Total                          | 16 reglas
Caso ORCID                     | 9 personas con orcid único — selectividad 9/9 = 1,0 (UNIQUE ≡ índice)
IndiceDesdeAnio · global 2024  | construcción 10 lecturas — barrido 12 → 3 lecturas con el índice
IndiceDesdeAnio · AS OF 2023   | 28 = 28 lecturas: sin mejora — + 10 de mantenimiento = 38
IndicePorLabel · AS OF 2026    | construcción 158 lecturas — 28 → 22 lecturas con el índice
Selectividad Documento.anio    | 5/36 ≈ 0,139 — índice NO construido (no discrimina)
Selectividad Persona.orcid     | 9/9 = 1,0
Selectividad Tema.nombre       | 8/9 ≈ 0,889 — duplicado residual: los temas 26 y 61 comparten «memoria de agentes»
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
El índice por AÑO casa con la consulta GLOBAL reescrita (su filtro es la clave del índice), pero no con el AS OF persona-céntrico: para 2023 su rango devuelve [184, 185] y ninguna de las vigentes del proyecto (52, 53, 55) está en él — el índice responde por AÑO, no por persona, y el mantenimiento (10) se cobra igual.
El índice por LABEL sí casa: la pregunta del cap-43 es por TIPO (WORKED_ON, MEMBER_OF) y esa es su clave — 28 → 22 lecturas. Su construcción (158 lecturas) solo se amortiza si la pregunta se REPITE: 158 + 22n < 28n desde n = 27.
"#;
        assert_eq!(
            reporte, esperado,
            "la salida del informe debe estar pineada byte a byte"
        );
    }
}
