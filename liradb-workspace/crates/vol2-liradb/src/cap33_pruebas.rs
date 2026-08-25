use std::fmt;
use std::path::Path;

use crate::cap07_modelo::{Edge, EdgeId, Node, NodeId};
use crate::cap08_graph_store::{GraphStore, StoreError};
use crate::cap28_wal::{WalError, WalRecord, decodificar_wal};

// ─────────────────── Cap 33: pruebas de una base de datos ───────────────────
//
// Los caps. 7-32 dejaron ~788 tests verdes y aun así quedaría miedo abrir
// un WAL a mano y cortarlo con tijeras. Este módulo añade los pisos que
// faltaban de la torre de riesgos del capítulo: cada pieza ataca UN riesgo
// que las unitarias de casos conocidos no ven.
//
//   * EL VERIFICADOR DE INVARIANTES ([`verificar_invariantes`] +
//     [`InvarianteRota`]) — el ORÁCULO COMÚN que todos los pisos superiores
//     afirman. Opera SÓLO sobre la API del puerto `GraphStore` (cap. 8):
//     las invariantes de grafo son observables; slots/páginas/índices son
//     internos y YA tienen guardián (`check()` del cap. 16, `Csr::verify()`
//     del cap. 14). Bajar a los campos de `MemoryStore` desde el verificador
//     rompería la abstracción hexagonal que sostiene el Vol.II entero —
//     los TESTS sí bajan (por sus campos `pub`) para demostrar que el
//     detector detecta, estilo mutation testing.
//
//   * LA BATERÍA DE CONTRATO ([`bateria_de_contrato`]) — UNA función
//     parametrizada por factory que ejercita el ciclo de vida completo del
//     puerto contra CUALQUIER `GraphStore`: sin ella no sabes si pruebas el
//     puerto o tu store. Hoy corre contra `MemoryStore` y contra
//     `StoreAlternativo` (HashMap, escrita en el módulo de tests).
//
//   * LA CARGA ESTRICTA DEL WAL ([`cargar_wal_estricta`]) — el hallazgo
//     estrella del capítulo hecho código: `WalIterator` (cap. 28) corta en
//     `Err(_) => None`, así que `cargar_wal`/`reabrir` (cap. 29) RECUPERAN
//     el prefijo limpio y TRAGAN la cola corrupta EN SILENCIO. Recuperar el
//     prefijo es legítimo (ARIES, parada limpia); perder bytes sin avisar
//     no. Esta función gemela grita, reutilizando la `decodificar_wal`
//     ESTRICTA del cap. 28 — sin tocar ningún capítulo anterior.
//
// Las ESTRATEGIAS proptest (`arb_nodo`, `arb_arista_valida`, `arb_grafo`),
// las PROPIEDADES (`prop_*`), la SUITE DE CRASH sobre bytes reales y la
// implementación didáctica `StoreAlternativo` viven en el módulo de pruebas
// de este mismo fichero: consumen sólo dev-dependencies (`proptest`,
// `tempfile`) porque nada de producción debe depender del generador
// aleatorio ni de los fixtures.
//
// Fronteras honestas (documentadas, no fingidas):
//   * MVCC QUEDA FUERA de la batería: `MvccStore` (cap. 30) NO implementa
//     `GraphStore` (sus lecturas llevan `ts` de snapshot); fingirlo exigiría
//     cambiar un capítulo cerrado. Queda como reto experto del libro.
//   * El verificador no puede ver la adyacencia de un nodo QUE NO EXISTE
//     (el puerto no permite preguntar por él): un fantasma colgado de un
//     slot muerto sería invisible — y esa es exactamente la FEATURE de la
//     honestidad hexagonal: lo interno tiene sus propios guardianes.
//   * La detección de fantasmas/asimetrías es O(V·E) por el `contains`
//     sobre las listas del puerto: escala sobrado para los tamaños del
//     libro y mantiene el código legible.

// ─────────────────── El oráculo común: invariantes ───────────────────

/// Una invariante del grafo que el store incumple, con contexto suficiente
/// para diagnosticarla sin abrir el store.
///
/// Las DOS primeras familias del brief son visibles desde el puerto:
/// «toda relación referencia nodos existentes» y «cada relación saliente
/// tiene su entrada correspondiente». Las otras dos (slots dentro de
/// página, índices con IDs válidos) viven DENTRO de sus módulos y ya
/// tienen guardián propio — [`crate::check`] y [`crate::Csr::verify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvarianteRota {
    /// Una arista viva referencia un nodo que no está en el store.
    AristaHuerfana {
        /// ID de la arista huérfana.
        arista: EdgeId,
        /// Su extremo origen (tal vez inexistente).
        source: NodeId,
        /// Su extremo destino (tal vez inexistente).
        target: NodeId,
    },
    /// `out_edges(nodo)` lista una arista que no existe o no sale de ese nodo.
    FantasmaEnSalientes {
        /// El nodo cuya vista saliente está podrida.
        nodo: NodeId,
        /// El ID fantasma listado.
        arista: EdgeId,
    },
    /// `in_edges(nodo)` lista una arista que no existe o no llega a ese nodo.
    FantasmaEnEntrantes {
        /// El nodo cuya vista entrante está podrida.
        nodo: NodeId,
        /// El ID fantasma listado.
        arista: EdgeId,
    },
    /// La arista aparece en la vista SALIENTE de su origen pero falta en la
    /// ENTRANTE de su destino: las dos vistas del store discrepan.
    SalidaSinEntrada {
        /// La arista asimétrica.
        arista: EdgeId,
    },
    /// La simétrica: presente en la ENTRANTE del destino, ausente en la
    /// SALIENTE del origen.
    EntradaSinSalida {
        /// La arista asimétrica.
        arista: EdgeId,
    },
}

impl fmt::Display for InvarianteRota {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvarianteRota::AristaHuerfana {
                arista,
                source,
                target,
            } => write!(
                f,
                "invariante rota: la arista {arista} ({source} → {target}) es huérfana: \
                 algún extremo no existe en el store"
            ),
            InvarianteRota::FantasmaEnSalientes { nodo, arista } => write!(
                f,
                "invariante rota: out_edges({nodo}) lista la arista {arista}, que no \
                 existe o no sale de ese nodo"
            ),
            InvarianteRota::FantasmaEnEntrantes { nodo, arista } => write!(
                f,
                "invariante rota: in_edges({nodo}) lista la arista {arista}, que no \
                 existe o no llega a ese nodo"
            ),
            InvarianteRota::SalidaSinEntrada { arista } => write!(
                f,
                "invariante rota: la arista {arista} está en la vista saliente de su \
                 origen pero falta en la entrante de su destino"
            ),
            InvarianteRota::EntradaSinSalida { arista } => write!(
                f,
                "invariante rota: la arista {arista} está en la vista entrante de su \
                 destino pero falta en la saliente de su origen"
            ),
        }
    }
}

/// Verifica las invariantes del grafo observables desde el PUERTO
/// `GraphStore` (cap. 8) y devuelve TODAS las rotas encontradas (no para
/// en la primera: un diagnóstico completo vale más que uno rápido).
///
/// Qué comprueba, y con qué lectura del puerto:
///   1. **Huerfanas** — para cada `e` de `iter_edges`, `get_node(source)`
///      y `get_node(target)` existen.
///   2. **Fantasmas** — cada ID listado por `out_edges(u)`/`in_edges(u)`
///      corresponde a una arista viva cuyo `source`/`target` es `u`.
///   3. **Asimetría out↔in** — si el store mantiene ambas vistas, toda
///      arista viva está simultáneamente en la saliente de su origen y en
///      la entrante de su destino.
///
/// Es el ORÁCULO COMÚN del capítulo: la batería de contrato lo afirma tras
/// cada fase, las propiedades proptest lo usan como tesis auxiliar y la
/// suite de crash lo exige sobre cada store resucitado de un prefijo WAL.
pub fn verificar_invariantes(store: &dyn GraphStore) -> Result<(), Vec<InvarianteRota>> {
    let mut rotas: Vec<InvarianteRota> = Vec::new();

    // 1. Toda relación referencia nodos existentes.
    for e in store.iter_edges() {
        if store.get_node(e.source).is_none() || store.get_node(e.target).is_none() {
            rotas.push(InvarianteRota::AristaHuerfana {
                arista: e.id,
                source: e.source,
                target: e.target,
            });
        }
    }

    // 2. Los índices de adyacencia sólo listan aristas coherentes.
    //    Limitación documentada: sólo podemos preguntar por nodos VIVOS
    //    (el puerto no enumera slots de nodos muertos) — lo interno que
    //    quede fuera tiene sus guardianes propios (caps. 14/16).
    let vivos: Vec<NodeId> = store.iter_nodes().map(|n| n.id).collect();
    for u in vivos {
        for eid in store.out_edges(u) {
            let coherente = store.get_edge(eid).is_some_and(|e| e.source == u);
            if !coherente {
                rotas.push(InvarianteRota::FantasmaEnSalientes {
                    nodo: u,
                    arista: eid,
                });
            }
        }
        for eid in store.in_edges(u) {
            let coherente = store.get_edge(eid).is_some_and(|e| e.target == u);
            if !coherente {
                rotas.push(InvarianteRota::FantasmaEnEntrantes {
                    nodo: u,
                    arista: eid,
                });
            }
        }
    }

    // 3. Asimetría entre las dos vistas (si el store mantiene ambas).
    for e in store.iter_edges() {
        let en_salida = store
            .get_node(e.source)
            .is_some_and(|_| store.out_edges(e.source).contains(&e.id));
        let en_entrada = store
            .get_node(e.target)
            .is_some_and(|_| store.in_edges(e.target).contains(&e.id));
        if en_salida && !en_entrada {
            rotas.push(InvarianteRota::SalidaSinEntrada { arista: e.id });
        }
        if !en_salida && en_entrada {
            rotas.push(InvarianteRota::EntradaSinSalida { arista: e.id });
        }
    }

    if rotas.is_empty() { Ok(()) } else { Err(rotas) }
}

// ─────────────────── La batería de contrato ───────────────────

/// Ejercita el CONTRATO SEMÁNTICO completo del puerto `GraphStore` (cap. 8)
/// contra cualquier implementación, creada por `fabrica`.
///
/// Es UNA suite compartida, no tests clónicos: ciclo de vida completo
/// (insertar → consultar → duplicados → endpoints inválidos → borrar
/// arista → borrar nodo con cascada → re-insertar), coherencia de
/// iteración y el oráculo de invariantes tras cada fase. La semántica
/// exigida es la de facto de `MemoryStore` (el contrato vivo del motor):
///   * `put_node`/`put_edge` RECHAZAN duplicados (`DuplicateNode`/
///     `DuplicateEdge`) — insertan, no reemplazan, pese a que la doc del
///     trait dice «reemplaza»: el contrato real es el que se TESTEA;
///   * `put_edge` exige ambos extremos vivos (`InvalidEdgeEndpoints`);
///   * `out_edges`/`in_edges` preservan el ORDEN DE INSERCIÓN;
///   * `delete_edge` dos veces es `true` luego `false`;
///   * `delete_node` arrastra en cascada las aristas incidentes y devuelve
///     `false` si el nodo no estaba;
///   * los IDs borrados vuelven a ser reutilizables.
///
/// FALLA con `panic!` y mensaje descriptivo ante la primera violación —
/// es una batería de pruebas, no un validador en runtime. Una batería sin
/// AL MENOS DOS implementaciones no distingue «pruebas el puerto» de
/// «pruebas tu store»: hoy la consumen `MemoryStore` (producción) y
/// `StoreAlternativo` (HashMap, didáctica, en los tests).
///
/// NOTA de alcance: `MvccStore` (cap. 30) QUEDA FUERA a propósito — sus
/// lecturas llevan `ts` de snapshot y NO implementa `GraphStore`; adaptarlo
/// fijando un `ts` cambiaría la API de un capítulo cerrado (queda como reto
/// experto del libro, decisión documentada §5.4).
pub fn bateria_de_contrato(fabrica: impl Fn() -> Box<dyn GraphStore>) {
    // ── Fase 0: el store recién nacido está vacío y coherente ──
    let mut s = fabrica();
    assert_eq!(s.node_count(), 0, "un store nuevo no tiene nodos");
    assert_eq!(s.edge_count(), 0, "un store nuevo no tiene aristas");
    assert_eq!(s.iter_nodes().count(), 0, "iter_nodes sobre vacío");
    assert_eq!(s.iter_edges().count(), 0, "iter_edges sobre vacío");
    assert!(s.get_node(0).is_none(), "get_node de id ausente");
    assert!(s.get_edge(0).is_none(), "get_edge de id ausente");
    assert!(
        s.out_edges(7).is_empty(),
        "out_edges de nodo ausente: vacío, jamás pánico"
    );
    assert!(
        s.in_edges(7).is_empty(),
        "in_edges de nodo ausente: vacío, jamás pánico"
    );
    assert!(!s.delete_node(0), "delete_node de id ausente: false");
    assert!(!s.delete_edge(0), "delete_edge de id ausente: false");
    assert!(
        verificar_invariantes(s.as_ref()).is_ok(),
        "el store vacío cumple las invariantes"
    );

    // ── Fase 1: nodos (insertar y duplicados) ──
    s.put_node(Node::new(0, "Persona")).expect("put_node nuevo");
    s.put_node(Node::new(1, "Persona")).expect("put_node nuevo");
    assert_eq!(s.node_count(), 2);
    assert_eq!(
        s.get_node(0).map(|n| n.labels.clone()),
        Some(vec!["Persona".to_string()]),
        "get_node devuelve el nodo insertado"
    );
    assert_eq!(
        s.put_node(Node::new(0, "Impostor")),
        Err(StoreError::DuplicateNode(0)),
        "put_node duplicado: rechazado, NO reemplaza"
    );

    // ── Fase 2: aristas con endpoints inválidos ──
    assert_eq!(
        s.put_edge(Edge::new(100, 0, 42, "X")),
        Err(StoreError::InvalidEdgeEndpoints {
            source: 0,
            target: 42
        }),
        "target inexistente"
    );
    assert_eq!(
        s.put_edge(Edge::new(101, 42, 0, "X")),
        Err(StoreError::InvalidEdgeEndpoints {
            source: 42,
            target: 0
        }),
        "source inexistente"
    );
    assert_eq!(s.edge_count(), 0, "los rechazos no dejaron basura");

    // ── Fase 3: aristas válidas, orden de inserción y self-loop ──
    s.put_edge(Edge::new(10, 0, 1, "KNOWS"))
        .expect("arista 0→1");
    s.put_edge(Edge::new(11, 1, 0, "KNOWS"))
        .expect("arista 1→0");
    s.put_edge(Edge::new(12, 0, 0, "SELF"))
        .expect("self-loop 0→0");
    assert_eq!(s.edge_count(), 3);
    assert_eq!(
        s.out_edges(0),
        vec![10, 12],
        "la adyacencia saliente preserva el orden de inserción"
    );
    assert_eq!(s.out_edges(1), vec![11]);
    assert_eq!(s.in_edges(0), vec![11, 12]);
    assert_eq!(s.in_edges(1), vec![10]);
    let e = s.get_edge(10).expect("get_edge de la arista insertada");
    assert_eq!((e.source, e.target, e.label.as_str()), (0, 1, "KNOWS"));
    assert_eq!(
        s.put_edge(Edge::new(10, 0, 1, "KNOWS")),
        Err(StoreError::DuplicateEdge(10)),
        "put_edge duplicado: rechazado"
    );
    assert!(
        verificar_invariantes(s.as_ref()).is_ok(),
        "tras poblar, el store sigue sano"
    );

    // ── Fase 4: iteración coherente con la adyacencia ──
    assert_eq!(s.iter_nodes().count(), 2);
    assert_eq!(s.iter_edges().count(), 3);
    let suma_out: usize = s.iter_nodes().map(|n| s.out_edges(n.id).len()).sum();
    let suma_in: usize = s.iter_nodes().map(|n| s.in_edges(n.id).len()).sum();
    assert_eq!(suma_out, s.edge_count(), "Σ out_edges == edge_count");
    assert_eq!(suma_in, s.edge_count(), "Σ in_edges == edge_count");

    // ── Fase 5: borrar UNA arista (y borrarla otra vez) ──
    assert!(s.delete_edge(10), "delete_edge existente: true");
    assert!(!s.delete_edge(10), "delete_edge ya borrada: false");
    assert_eq!(s.edge_count(), 2);
    assert_eq!(s.out_edges(0), vec![12], "la arista salió de adj_out");
    assert_eq!(s.in_edges(1), Vec::<usize>::new(), "y de adj_in");
    assert!(verificar_invariantes(s.as_ref()).is_ok());

    // ── Fase 6: delete_node con CASCADA de aristas incidentes ──
    assert!(s.delete_node(1), "delete_node existente: true");
    assert_eq!(s.node_count(), 1);
    assert_eq!(
        s.edge_count(),
        1,
        "la cascada arrastró la arista 1→0; queda sólo el self-loop"
    );
    assert!(s.get_node(1).is_none());
    assert!(s.get_edge(11).is_none(), "la arista arrastrada ya no está");
    assert!(s.out_edges(1).is_empty());
    assert!(s.in_edges(1).is_empty());
    assert!(!s.delete_node(1), "delete_node ya borrado: false");
    assert!(
        verificar_invariantes(s.as_ref()).is_ok(),
        "la cascada no deja huérfanas ni fantasmas"
    );

    // ── Fase 7: re-inserción tras borrado (los ids vuelven) ──
    s.put_node(Node::new(1, "Renacido"))
        .expect("el id borrado es reutilizable");
    s.put_edge(Edge::new(11, 1, 0, "KNOWS"))
        .expect("el id de arista borrado también");
    assert_eq!(s.node_count(), 2);
    assert_eq!(s.edge_count(), 2);
    assert_eq!(
        s.get_node(1).map(|n| n.labels.clone()),
        Some(vec!["Renacido".to_string()]),
        "el nodo renacido es OTRO nodo con el mismo id"
    );
    assert!(
        verificar_invariantes(s.as_ref()).is_ok(),
        "estado final: el oráculo da la batería por buena"
    );
}

// ─────────────────── La carga ESTRICTA del WAL (hallazgo cap 28) ───────────────────

/// Errores de [`cargar_wal_estricta`]: la E/S del fichero o el WAL podrido.
#[derive(Debug)]
pub enum ErrorCargaEstricta {
    /// El fichero no se pudo leer (permisos, ausencia…).
    Io(std::io::Error),
    /// El contenido viola el formato estricto: CRC inválido, registro
    /// truncado o LSN no consecutivo ([`WalError`] del cap. 28).
    Wal(WalError),
}

impl fmt::Display for ErrorCargaEstricta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCargaEstricta::Io(e) => write!(f, "carga estricta del WAL: E/S: {e}"),
            ErrorCargaEstricta::Wal(e) => write!(f, "carga estricta del WAL: {e}"),
        }
    }
}

impl std::error::Error for ErrorCargaEstricta {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorCargaEstricta::Io(e) => Some(e),
            ErrorCargaEstricta::Wal(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for ErrorCargaEstricta {
    fn from(e: std::io::Error) -> Self {
        ErrorCargaEstricta::Io(e)
    }
}

impl From<WalError> for ErrorCargaEstricta {
    fn from(e: WalError) -> Self {
        ErrorCargaEstricta::Wal(e)
    }
}

/// Lee el fichero del WAL en modo ESTRICTO: cualquier byte podrido es un
/// error NOMBRADO, jamás una cola tragada en silencio.
///
/// EL HALLAZGO que motiva esta función: `WalIterator` (cap. 28) corta en
/// `Err(_) => None` ante el primer registro dañado, y `cargar_wal`/`reabrir`
/// (cap. 29) heredan ese silencio — recuperan el prefijo limpio y PIERDEN
/// la cola sin avisar de nada. Recuperar el prefijo es legítimo (es la
/// parada limpia de ARIES ante un log append-only); LO QUE NO ES LEGÍTIMO
/// es no decir cuánto se perdió. Esta gemela vive EN CAP33 a propósito:
/// endurecer `cargar_wal` rompería el diseño «parada limpia» documentado y
/// testeado de los caps. 28/29 — aquí se AÑADE la voz alta, no se cambia el
/// comportamiento de nadie.
///
/// Implementación honesta: lee los bytes y delega en [`decodificar_wal`],
/// la versión ESTRICTA que el cap. 28 YA tenía (framing+CRC+cadena de LSN,
/// con `WalError::{CrcInvalido, RegistroTruncado, LsnInvalido}`). No hay
/// lógica nueva de parsing que fuzzear: la misma puerta, otro cerrojo.
///
/// ```
/// use vol2_liradb::{Edge, MemoryStore, Node, Wal, WalTransaccion,
///                   cargar_wal_estricta, guardar_wal};
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("estricta.wal");
/// let mut store = MemoryStore::new();
/// let mut wal = Wal::new();
/// let mut tx = WalTransaccion::begin(&mut store, &mut wal);
/// tx.put_node(Node::new(0, "Person")).unwrap();
/// tx.commit().unwrap();
/// guardar_wal(&wal, &path).unwrap();
///
/// // Un WAL sano carga limpio y entrega TODOS sus registros…
/// let registros = cargar_wal_estricta(&path).unwrap();
/// assert_eq!(registros.len(), 3); // Begin + Operacion + Commit
///
/// // …pero un byte tocado bajo el CRC ya no pasa desapercibido.
/// let mut bytes = std::fs::read(&path).unwrap();
/// let ultimo = bytes.len() - 5; // dentro del cuerpo, bajo el CRC final
/// bytes[ultimo] ^= 0x08;
/// std::fs::write(&path, &bytes).unwrap();
/// assert!(cargar_wal_estricta(&path).is_err()); // GRITA (CrcInvalido)
/// ```
pub fn cargar_wal_estricta(path: impl AsRef<Path>) -> Result<Vec<WalRecord>, ErrorCargaEstricta> {
    let bytes = std::fs::read(path)?;
    Ok(decodificar_wal(&bytes)?)
}

// ─────────────────────────────────────────────────────────────────
// Tests: mutation testing, batería, propiedades, crash y compatibilidad
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_cap33 {
    use super::*;
    use crate::cap07_modelo::Value;
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap09_encoding::{
        FORMAT_VERSION, decode_header, decode_value, encode_header, encode_u32_le, encode_value,
    };
    use crate::cap14_csr::Csr;
    use crate::cap27_transacciones::{Operacion, autocommit};
    use crate::cap28_wal::{CuerpoWal, TxId, Wal, WalTransaccion, decode_wal_record, replay_wal};
    use crate::cap29_recuperacion::{cargar_wal, guardar_wal};
    use crate::cap32_import_export::{
        exportar_csv_aristas, exportar_csv_nodos, exportar_jsonl, importar_csv_aristas,
        importar_csv_nodos, importar_jsonl,
    };
    use proptest::prelude::*;
    use std::collections::HashMap;

    // ── La segunda implementación didáctica ──────────────────────

    /// Un `GraphStore` sobre HashMaps, escrita AQUÍ a propósito: una batería
    /// de contrato sin AL MENOS DOS implementaciones no distingue «pruebas
    /// el puerto» de «pruebas mi store» (§5.3 del capítulo). Replica la
    /// semántica EXACTA de `MemoryStore` — duplicados rechazados, cascada
    /// de `delete_node`, orden de inserción en las vistas — pero con OTRA
    /// estructura de datos: si el contrato está bien escrito, ambas pasan
    /// SIN tocar una línea de la batería.
    #[derive(Default)]
    struct StoreAlternativo {
        nodos: HashMap<NodeId, Node>,
        aristas: HashMap<EdgeId, Edge>,
        salientes: HashMap<NodeId, Vec<EdgeId>>,
        entrantes: HashMap<NodeId, Vec<EdgeId>>,
    }

    impl StoreAlternativo {
        fn new() -> Self {
            Self::default()
        }
    }

    impl GraphStore for StoreAlternativo {
        fn put_node(&mut self, node: Node) -> Result<(), StoreError> {
            if self.nodos.contains_key(&node.id) {
                return Err(StoreError::DuplicateNode(node.id));
            }
            self.salientes.entry(node.id).or_default();
            self.entrantes.entry(node.id).or_default();
            self.nodos.insert(node.id, node);
            Ok(())
        }

        fn put_edge(&mut self, edge: Edge) -> Result<(), StoreError> {
            if self.aristas.contains_key(&edge.id) {
                return Err(StoreError::DuplicateEdge(edge.id));
            }
            if !self.nodos.contains_key(&edge.source) || !self.nodos.contains_key(&edge.target) {
                return Err(StoreError::InvalidEdgeEndpoints {
                    source: edge.source,
                    target: edge.target,
                });
            }
            self.salientes.entry(edge.source).or_default().push(edge.id);
            self.entrantes.entry(edge.target).or_default().push(edge.id);
            self.aristas.insert(edge.id, edge);
            Ok(())
        }

        fn get_node(&self, id: NodeId) -> Option<&Node> {
            self.nodos.get(&id)
        }

        fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
            self.aristas.get(&id)
        }

        fn out_edges(&self, u: NodeId) -> Vec<EdgeId> {
            self.salientes.get(&u).cloned().unwrap_or_default()
        }

        fn in_edges(&self, u: NodeId) -> Vec<EdgeId> {
            self.entrantes.get(&u).cloned().unwrap_or_default()
        }

        fn node_count(&self) -> usize {
            self.nodos.len()
        }

        fn edge_count(&self) -> usize {
            self.aristas.len()
        }

        fn delete_node(&mut self, id: NodeId) -> bool {
            if !self.nodos.contains_key(&id) {
                return false;
            }
            let incidentes: Vec<EdgeId> = self
                .salientes
                .get(&id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .chain(self.entrantes.get(&id).cloned().unwrap_or_default())
                .collect();
            for eid in incidentes {
                self.delete_edge(eid);
            }
            self.nodos.remove(&id);
            self.salientes.remove(&id);
            self.entrantes.remove(&id);
            true
        }

        fn delete_edge(&mut self, id: EdgeId) -> bool {
            match self.aristas.remove(&id) {
                Some(e) => {
                    if let Some(vista) = self.salientes.get_mut(&e.source) {
                        vista.retain(|&x| x != id);
                    }
                    if let Some(vista) = self.entrantes.get_mut(&e.target) {
                        vista.retain(|&x| x != id);
                    }
                    true
                }
                None => false,
            }
        }

        fn iter_nodes(&self) -> Box<dyn Iterator<Item = &Node> + '_> {
            Box::new(self.nodos.values())
        }

        fn iter_edges(&self) -> Box<dyn Iterator<Item = &Edge> + '_> {
            Box::new(self.aristas.values())
        }
    }

    // ── Oráculo auxiliar: igualdad estructural entre stores ──────

    fn nodos_ordenados(s: &dyn GraphStore) -> Vec<Node> {
        let mut v: Vec<Node> = s.iter_nodes().cloned().collect();
        v.sort_by_key(|n| n.id);
        v
    }

    fn aristas_ordenadas(s: &dyn GraphStore) -> Vec<Edge> {
        let mut v: Vec<Edge> = s.iter_edges().cloned().collect();
        v.sort_by_key(|e| e.id);
        v
    }

    /// Igualdad ESTRUCTURAL: mismos nodos, mismas aristas y mismas vistas
    /// de adyacencia (el orden de inserción es estado observable del
    /// contrato de facto — ver [`bateria_de_contrato`]).
    fn grafos_iguales(a: &dyn GraphStore, b: &dyn GraphStore) -> bool {
        if a.node_count() != b.node_count() || a.edge_count() != b.edge_count() {
            return false;
        }
        if nodos_ordenados(a) != nodos_ordenados(b) {
            return false;
        }
        if aristas_ordenadas(a) != aristas_ordenadas(b) {
            return false;
        }
        for n in nodos_ordenados(a) {
            if a.out_edges(n.id) != b.out_edges(n.id) || a.in_edges(n.id) != b.in_edges(n.id) {
                return false;
            }
        }
        true
    }

    /// Monta un `MemoryStore` a partir del par (nodos, aristas) de las
    /// estrategias: los `expect` son seguros POR CONSTRUCCIÓN (ids densos,
    /// extremos siempre vivos) — si algún día petan, el bug está en la
    /// estrategia, y este mensaje lo dice claro.
    fn store_de(nodos: &[Node], aristas: &[Edge]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for n in nodos {
            s.put_node(n.clone())
                .expect("la estrategia genera nodos válidos");
        }
        for e in aristas {
            s.put_edge(e.clone())
                .expect("la estrategia genera aristas válidas");
        }
        s
    }

    // ── Estrategias proptest ─────────────────────────────────────

    type Props = HashMap<String, Value>;

    /// Catálogo pequeño de etiquetas y tipos de relación: pocas variantes ⇒
    /// shrink acotado y counterexamples legibles.
    fn arb_etiqueta() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["Person".to_string(), "City".to_string()])
    }

    fn arb_tipo_relacion() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["KNOWS".to_string(), "LIVES_IN".to_string()])
    }

    /// Float SIEMPRE con parte fraccional y representación binaria exacta:
    /// (2k+1)/16. El porqué doble: un float INTEGRAL se serializa como «3»
    /// en JSONL/CSV y al reimportar normaliza a Int (hallazgo documentado
    /// del capítulo), y los denominadores potenciales de 2 evitan cualquier
    /// sorpresa de redondeo en el reencode.
    fn valor_float_con_fraccion() -> impl Strategy<Value = Value> {
        (0u32..4096).prop_map(|k| Value::Float(f64::from(k) / 8.0 + 0.0625))
    }

    /// Props «simples» (Int/String/Bool/Null) con claves de catálogo: la
    /// base de [`arb_grafo`] y de la operación aleatoria del replay WAL.
    fn arb_props_simples() -> impl Strategy<Value = Props> {
        prop::collection::vec(
            (
                prop::sample::select(vec!["edad", "nombre", "activo"]),
                prop_oneof![
                    Just(Value::Null),
                    any::<bool>().prop_map(Value::Bool),
                    any::<i16>().prop_map(|v| Value::Int(i64::from(v))),
                    "[a-zA-Z]{0,6}".prop_map(Value::String),
                ],
            ),
            0..=3,
        )
        .prop_map(|pares| {
            pares
                .into_iter()
                .map(|(clave, valor)| (clave.to_string(), valor))
                .collect()
        })
    }

    /// Props CSV-SEGUROS: sólo tipos que el formato preserva, y con TIPO
    /// FIJO POR CLAVE — una columna CSV tiene un único tipo; si dos nodos
    /// le dieran tipos distintos a la misma clave, la cabecera (tipo de la
    /// primera aparición) corrompería al segundo al reimportar. Sin Null
    /// ni Bytes ni cadenas vacías: en CSV, campo vacío = prop ausente.
    fn arb_props_csv() -> impl Strategy<Value = Props> {
        (
            any::<i64>(),
            valor_float_con_fraccion(),
            any::<bool>(),
            "[a-zA-Z]{1,6}",
            prop::collection::vec(any::<bool>(), 4),
        )
            .prop_map(|(edad, peso, activo, nombre, presentes)| {
                let mut props: Props = HashMap::new();
                if presentes[0] {
                    props.insert("edad".to_string(), Value::Int(edad));
                }
                if presentes[1] {
                    props.insert("peso".to_string(), peso);
                }
                if presentes[2] {
                    props.insert("activo".to_string(), Value::Bool(activo));
                }
                if presentes[3] {
                    props.insert("nombre".to_string(), Value::String(nombre));
                }
                props
            })
    }

    /// Props para JSONL: los CSV-safe MÁS `Bytes` (JSONL es el formato SIN
    /// pérdida). Cadena ASCII imprimible: el escapador JSONL emite \uXXXX
    /// de 4 dígitos (BMP) — un carácter fuera del BMP (un emoji) produciría
    /// un escape malformado que además reimportaría CORROMPIDO en silencio
    /// (frontera descubierta diseñando esta propiedad; queda documentada
    /// como límite del subconjunto JSON del cap. 32).
    fn arb_props_jsonl() -> impl Strategy<Value = Props> {
        (arb_props_csv(), prop::collection::vec(any::<u8>(), 0..=4)).prop_map(
            |(mut props, bytes)| {
                props.insert("datos".to_string(), Value::Bytes(bytes));
                props
            },
        )
    }

    /// UN nodo con id FIJO: la unicidad la garantiza el llamador asignando
    /// la posición en su colección — jamás un filtro.
    fn arb_nodo(id: NodeId, props: impl Strategy<Value = Props>) -> impl Strategy<Value = Node> {
        (arb_etiqueta(), props).prop_map(move |(etiqueta, props)| Node {
            id,
            labels: vec![etiqueta],
            props,
        })
    }

    /// UNA arista VÁLIDA: los extremos SIEMPRE salen del pool de nodos ya
    /// generados. Ésa es LA técnica que sustituye al filtro masivo: muestrear
    /// ÍNDICES dentro del rango del pool en vez de pares cualesquiera
    /// rechazables (`prop_filter` encoge fatal: casos útiles raros y
    /// counterexamples gigantes — §5.6).
    fn arb_arista_valida(
        pool: Vec<NodeId>,
        id: EdgeId,
        props: impl Strategy<Value = Props>,
    ) -> impl Strategy<Value = Edge> {
        debug_assert!(!pool.is_empty(), "pool vacío: el llamador ramifica");
        (0..pool.len(), 0..pool.len(), arb_tipo_relacion(), props).prop_map(
            move |(s, t, tipo, props)| Edge {
                id,
                source: pool[s],
                target: pool[t],
                label: tipo,
                props,
            },
        )
    }

    /// UN GRAFO VÁLIDO completo: 0..25 nodos con ids densos 0..n y 0..60
    /// aristas cuyos extremos existen SIEMPRE. Encoger = menos elementos,
    /// nunca elementos inválidos: ése es el «shrink acotado» del contrato.
    /// Las props llegan ya ENCAJONADAS (`BoxedStrategy` es `Clone`; un
    /// `impl Strategy` no lo es, y el `prop_flat_map` necesita clonar).
    fn arb_grafo(
        props_nodo: BoxedStrategy<Props>,
        props_arista: BoxedStrategy<Props>,
    ) -> impl Strategy<Value = (Vec<Node>, Vec<Edge>)> {
        prop::collection::vec((arb_etiqueta(), props_nodo), 0..25).prop_flat_map(move |pares| {
            let nodos: Vec<Node> = pares
                .into_iter()
                .enumerate()
                .map(|(id, (etiqueta, props))| Node {
                    id,
                    labels: vec![etiqueta],
                    props,
                })
                .collect();
            let pool: Vec<NodeId> = (0..nodos.len()).collect();
            let aristas: BoxedStrategy<Vec<Edge>> = if pool.is_empty() {
                Just(Vec::new()).boxed()
            } else {
                prop::collection::vec(
                    arb_arista_valida(pool.clone(), 0, props_arista.clone()),
                    0..60,
                )
                .prop_map(|generadas| {
                    generadas
                        .into_iter()
                        .enumerate()
                        .map(|(id, mut e)| {
                            e.id = id; // ids densos: unicidad sin filtros
                            e
                        })
                        .collect()
                })
                .boxed()
            };
            (Just(nodos), aristas).boxed()
        })
    }

    /// Operación aleatoria SOBRE EL UNIVERSO de ids 0..8: muchas serán
    /// inválidas según el estado (endpoints ausentes, duplicados, deletes
    /// repetidos) y eso ES PARTE DE LA PROPIEDAD — los dos caminos deben
    /// aceptarlas o rechazarlas IDÉNTICAMENTE. No se filtran en generación:
    /// el rechazo es semántica del motor, no basura del generador.
    fn arb_operacion() -> impl Strategy<Value = Operacion> {
        prop_oneof![
            4 => (0..8usize)
                .prop_flat_map(|id| arb_nodo(id, arb_props_simples()))
                .prop_map(Operacion::PutNode),
            3 => (0..8usize, 0..8usize, 0..8usize).prop_map(|(id, s, t)| Operacion::PutEdge(
                Edge::new(id, s, t, "KNOWS")
            )),
            1 => (0..8usize).prop_map(Operacion::DeleteNode),
            1 => (0..8usize).prop_map(Operacion::DeleteEdge),
        ]
    }

    // ── Mutation testing: el detector DETECTA ────────────────────

    #[test]
    fn invariantes_grafo_sano_pasa() {
        // El fixture público del motor (cap. 20) como grafo sano de
        // referencia: 6 nodos, 6 aristas, self-loop incluido.
        let sano = crate::demo_graph();
        assert_eq!(sano.node_count(), 6);
        assert_eq!(sano.edge_count(), 6);
        assert_eq!(
            verificar_invariantes(&sano),
            Ok(()),
            "el grafo demo es el contraejemplo perfecto de «no hay bugs»: debe pasar limpio"
        );
    }

    #[test]
    fn invariantes_detectan_arista_huerfana_en_adj() {
        // MUTACIÓN (estilo mutation testing): colar en adj_out un id que no
        // corresponde a ninguna arista viva. MemoryStore expone sus campos
        // pub — el test corrompe POR AHÍ, como haría un bug real de índice.
        let mut mutante = crate::demo_graph();
        mutante.adj_out[0].push(999);

        let rotas = verificar_invariantes(&mutante).expect_err("la mutación debió ser cazada");
        assert!(
            rotas.iter().any(|r| matches!(
                r,
                InvarianteRota::FantasmaEnSalientes {
                    nodo: 0,
                    arista: 999
                }
            )),
            "falta el fantasma saliente; rotas: {rotas:?}"
        );
        // Y el Display lo cuenta en cristiano.
        let fantasma = rotas
            .iter()
            .find(|r| matches!(r, InvarianteRota::FantasmaEnSalientes { .. }))
            .expect("ya verificado arriba");
        assert!(
            fantasma.to_string().contains("out_edges(0)"),
            "el diagnóstico nombra al nodo y a su vista: {fantasma}"
        );
    }

    #[test]
    fn invariantes_detectan_salida_sin_entrada() {
        // MUTACIÓN: la arista 0 (Ana→Bo) pierde su entrada en adj_in[1].
        // La vista saliente la conserva; la entrante ya no → ASIMETRÍA.
        let mut mutante = crate::demo_graph();
        mutante.adj_in[1].retain(|&e| e != 0);

        let rotas = verificar_invariantes(&mutante).expect_err("la mutación debió ser cazada");
        assert!(
            rotas
                .iter()
                .any(|r| matches!(r, InvarianteRota::SalidaSinEntrada { arista: 0 })),
            "falta la asimetría salida-sin-entrada; rotas: {rotas:?}"
        );
        assert!(
            !rotas
                .iter()
                .any(|r| matches!(r, InvarianteRota::EntradaSinSalida { .. })),
            "la dirección de la asimetría debe ser precisa"
        );
    }

    #[test]
    fn invariantes_detectan_entrada_sin_salida() {
        // MUTACIÓN simétrica: Carla→Ana (arista 2) pierde su SALIDA.
        let mut mutante = crate::demo_graph();
        mutante.adj_out[2].retain(|&e| e != 2);

        let rotas = verificar_invariantes(&mutante).expect_err("la mutación debió ser cazada");
        assert!(
            rotas
                .iter()
                .any(|r| matches!(r, InvarianteRota::EntradaSinSalida { arista: 2 })),
            "falta la asimetría entrada-sin-salida; rotas: {rotas:?}"
        );
    }

    #[test]
    fn invariantes_detectan_nodo_borrado_por_debajo_del_puerto() {
        // MUTACIÓN: Madrid desaparece SIN cascada (corrupción directa del
        // vector de nodos). LIVES_IN Ana→Madrid (arista 4) queda HUÉRFANA.
        let mut mutante = crate::demo_graph();
        mutante.nodes[4] = None;

        let rotas = verificar_invariantes(&mutante).expect_err("la mutación debió ser cazada");
        assert!(
            rotas.iter().any(|r| matches!(
                r,
                InvarianteRota::AristaHuerfana {
                    arista: 4,
                    target: 4,
                    ..
                }
            )),
            "falta la huérfana; rotas: {rotas:?}"
        );
    }

    // ── La batería de contrato, consumida por DOS stores ─────────

    #[test]
    fn contrato_bateria_memory_store() {
        // El store de producción pasa la batería SIN tocar una línea de ella.
        bateria_de_contrato(|| Box::new(MemoryStore::new()));
    }

    #[test]
    fn contrato_bateria_store_alternativo() {
        // La segunda implementación (HashMap) pasa LA MISMA batería: eso es
        // lo que demuestra que probamos el PUERTO, no un store concreto.
        bateria_de_contrato(|| Box::new(StoreAlternativo::new()));
    }

    // ── Propiedades (property-based testing) ─────────────────────

    proptest! {
        #[test]
        fn prop_roundtrip_encoding_byte_identico(
            (nodos, aristas) in arb_grafo(arb_props_simples().boxed(), arb_props_simples().boxed()),
        ) {
            // El cap. 9 codifica VALUES (los tags del formato binario); el
            // grafo completo viaja bajo el framing del cap. 10/28. La
            // propiedad recorre TODO Value vivo del grafo generado:
            // encode → decode → reencode y exige BYTES IDÉNTICOS.
            let mut valores: Vec<&Value> = Vec::new();
            for n in &nodos {
                valores.extend(n.props.values());
            }
            for e in &aristas {
                valores.extend(e.props.values());
            }
            for v in valores {
                let bytes = encode_value(v);
                let (decodificado, resto) = decode_value(&bytes)
                    .unwrap_or_else(|e| panic!("decode falló para {v:?}: {e}"));
                proptest::prop_assert!(resto.is_empty());
                proptest::prop_assert_eq!(&decodificado, v);
                // LA tesis: mismo valor ⇒ mismos bytes. Un encoding no
                // canónico haría este assert sangrar con el counterexample
                // más pequeño que encuentre.
                proptest::prop_assert_eq!(encode_value(&decodificado), bytes.clone());
            }
        }

        #[test]
        fn prop_wal_replay_reproduce_estado(
            ops in prop::collection::vec(arb_operacion(), 0..40),
        ) {
            // Semilla común: 4 nodos base para que las aristas tengan dónde
            // engancharse desde la primera operación aleatoria.
            let sembrar = |s: &mut MemoryStore| {
                for id in 0..4usize {
                    s.put_node(Node::new(id, "Base")).expect("semilla");
                }
            };

            // Camino A (la verdad): las MISMAS operaciones en autocommit,
            // aplicadas al store sin log.
            let mut directo = MemoryStore::new();
            sembrar(&mut directo);
            let mut aceptadas = 0usize;
            for op in &ops {
                if autocommit(&mut directo, op.clone()).is_ok() {
                    aceptadas += 1;
                }
            }

            // Camino B: las mismas ops vía WAL REAL (staging + commit en dos
            // fases). La validación eager decide IGUAL que el autocommit,
            // porque ambos ven el mismo estado del store paso a paso.
            let mut con_wal = MemoryStore::new();
            sembrar(&mut con_wal);
            let mut wal = Wal::new();
            for op in &ops {
                let mut tx = WalTransaccion::begin(&mut con_wal, &mut wal);
                if tx.stage(op.clone()).is_ok() {
                    tx.commit().expect("commit tras staging válido");
                } else {
                    drop(tx); // Begin huérfano: el replay la descarta
                }
            }

            // El DISCO de por medio (caps. 28+29 reales, bytes incluidos):
            // guardar → cargar → replay sobre un store fresco.
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("replay.wal");
            guardar_wal(&wal, &path).expect("guardar_wal");
            let wal_disco = cargar_wal(&path).expect("cargar_wal");
            proptest::prop_assert_eq!(wal.as_bytes(), wal_disco.as_bytes());

            let mut renacido = MemoryStore::new();
            sembrar(&mut renacido);
            let informe =
                replay_wal(&mut renacido, &wal_disco).expect("el replay nunca falla aquí");

            // TESIS: el estado reproducido es EL MISMO que el camino directo,
            // con exactamente las operaciones aceptadas…
            proptest::prop_assert_eq!(informe.operaciones_reaplicadas, aceptadas);
            proptest::prop_assert!(
                grafos_iguales(&directo, &renacido),
                "el replay reprodujo OTRO grafo"
            );
            // …y el oráculo común lo firma.
            proptest::prop_assert!(verificar_invariantes(&renacido).is_ok());
        }

        #[test]
        fn prop_jsonl_roundtrip_preserva_grafo(
            (nodos, aristas) in arb_grafo(arb_props_jsonl().boxed(), arb_props_jsonl().boxed()),
        ) {
            let original = store_de(&nodos, &aristas);

            let mut buffer = std::io::Cursor::new(Vec::new());
            exportar_jsonl(&original, &mut buffer).expect("exportar_jsonl");
            let bytes = buffer.into_inner();

            let mut lector = std::io::Cursor::new(bytes);
            let mut reimportado = MemoryStore::new();
            let stats = importar_jsonl(&mut lector, &mut reimportado)
                .expect("un JSONL que uno mismo acaba de escribir reimporta limpio");

            // TESIS: el formato SIN PÉRDIDA preserva TODO el grafo (hasta
            // los Bytes), y el oráculo firma la coherencia estructural.
            proptest::prop_assert_eq!(stats.nodos, nodos.len());
            proptest::prop_assert_eq!(stats.aristas, aristas.len());
            proptest::prop_assert!(grafos_iguales(&original, &reimportado));
            proptest::prop_assert!(verificar_invariantes(&reimportado).is_ok());
        }

        #[test]
        fn prop_csv_roundtrip_preserva_grafo(
            (nodos, aristas) in arb_grafo(arb_props_csv().boxed(), arb_props_csv().boxed()),
        ) {
            // CSV necesita DOS ficheros (nodos y aristas): el roundtrip es
            // exportar ambos → importar en orden (primero nodos, luego
            // aristas, o los endpoints no existirían).
            let original = store_de(&nodos, &aristas);

            let mut buf_nodos = std::io::Cursor::new(Vec::new());
            exportar_csv_nodos(&original, &mut buf_nodos).expect("exportar_csv_nodos");
            let mut buf_aristas = std::io::Cursor::new(Vec::new());
            exportar_csv_aristas(&original, &mut buf_aristas).expect("exportar_csv_aristas");

            let mut reimportado = MemoryStore::new();
            let stats_nodos = importar_csv_nodos(
                &mut std::io::Cursor::new(buf_nodos.into_inner()),
                &mut reimportado,
            )
            .expect("CSV de nodos recién escrito reimporta limpio");
            let stats_aristas = importar_csv_aristas(
                &mut std::io::Cursor::new(buf_aristas.into_inner()),
                &mut reimportado,
            )
            .expect("CSV de aristas recién escrito reimporta limpio");

            // TESIS: con props CSV-seguros (tipos fijos por clave, sin Null/
            // Bytes/cadenas vacías), el roundtrip es EXACTO — la pérdida de
            // CSV es la de su ESQUEMA, no la de estos datos.
            proptest::prop_assert_eq!(stats_nodos.nodos, nodos.len());
            proptest::prop_assert_eq!(stats_aristas.aristas, aristas.len());
            proptest::prop_assert!(grafos_iguales(&original, &reimportado));
            proptest::prop_assert!(verificar_invariantes(&reimportado).is_ok());
        }

        #[test]
        fn prop_csr_consistente_con_iteracion_directa(
            (nodos, aristas) in arb_grafo(arb_props_simples().boxed(), arb_props_simples().boxed()),
        ) {
            let store = store_de(&nodos, &aristas);
            let pares: Vec<(NodeId, NodeId)> = store
                .iter_edges()
                .map(|e| (e.source, e.target))
                .collect();

            // La proyección CSR (cap. 14) sobre los MISMOS pares que dicta
            // la iteración directa del store (cap. 26 materializa lo mismo).
            let csr = Csr::from_edges(pares).expect("extremo siempre válido");
            csr.verify().expect("el CSR de un grafo válido se verifica");
            proptest::prop_assert_eq!(csr.edge_count() as usize, aristas.len());

            // TESIS: para CADA nodo, los vecinos del CSR son EL MISMO
            // multiconjunto que la iteración directa vía puerto. Se comparan
            // ORDENADOS porque el CSR preserva el orden de SU entrada (la
            // lista de pares), y el contrato del puerto no promete orden.
            let limite = nodos.iter().map(|n| n.id + 1).max().unwrap_or(0);
            for u in 0..limite {
                let mut directos: Vec<NodeId> = store
                    .out_edges(u)
                    .into_iter()
                    .map(|eid| store.get_edge(eid).expect("coherente").target)
                    .collect();
                directos.sort_unstable();
                let mut csr_out = csr.neighbors_out(u).to_vec();
                csr_out.sort_unstable();
                assert_eq!(directos, csr_out, "vecinos salientes de {u}");

                let mut entrantes: Vec<NodeId> = store
                    .in_edges(u)
                    .into_iter()
                    .map(|eid| store.get_edge(eid).expect("coherente").source)
                    .collect();
                entrantes.sort_unstable();
                let mut csr_in = csr.neighbors_in(u).to_vec();
                csr_in.sort_unstable();
                assert_eq!(entrantes, csr_in, "vecinos entrantes de {u}");
            }
        }
    }

    // ── Crash testing sobre BYTES REALES del WAL ─────────────────

    /// WAL sano de prueba: 3 transacciones confirmadas con nodos y aristas.
    /// El último registro es SIEMPRE el Commit de la tercera (los tests de
    /// crash dependen de esa geometría).
    fn wal_de_prueba() -> Wal {
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "Person")).unwrap();
            tx.put_node(Node::new(1, "Person")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "KNOWS")).unwrap();
            tx.commit().unwrap();
        }
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(2, "City")).unwrap();
            tx.put_edge(Edge::new(1, 1, 2, "LIVES_IN")).unwrap();
            tx.commit().unwrap();
        }
        {
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_edge(Edge::new(2, 2, 0, "KNOWS")).unwrap();
            tx.put_node(Node::new(3, "Person")).unwrap();
            tx.commit().unwrap();
        }
        wal
    }

    /// Posición (offset) donde EMPIEZA el último registro de un log sano.
    fn posicion_ultimo_registro(bytes: &[u8]) -> usize {
        let mut pos = 0usize;
        loop {
            match decode_wal_record(&bytes[pos..]) {
                Ok((_, cola)) => {
                    if cola.is_empty() {
                        return pos;
                    }
                    pos = bytes.len() - cola.len();
                }
                Err(_) => return pos, // no ocurre en un log sano
            }
        }
    }

    /// Operaciones redo que el replay aplicaría para esos registros: las
    /// de transacciones CON commit marker (el oráculo del informe).
    fn operaciones_confirmadas(registros: &[WalRecord]) -> usize {
        let confirmadas: std::collections::HashSet<TxId> = registros
            .iter()
            .filter(|r| matches!(r.cuerpo, CuerpoWal::Commit))
            .map(|r| r.tx_id)
            .collect();
        registros
            .iter()
            .filter(|r| {
                matches!(r.cuerpo, CuerpoWal::Operacion(_)) && confirmadas.contains(&r.tx_id)
            })
            .count()
    }

    #[test]
    fn crash_truncado_sistematico_nunca_panico() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash.wal");
        guardar_wal(&wal_de_prueba(), &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 64, "un WAL realista para cortar con tijeras");

        // CADA prefijo posible (paso 1 si el fichero es pequeño; acotado a
        // ~256 cortes si fuera grande): tijeras sistemáticas, no muestras.
        let paso = (bytes.len() / 256).max(1);
        let cortes: Vec<usize> = (0..=bytes.len())
            .step_by(paso)
            .chain(std::iter::once(bytes.len()))
            .collect();

        for corte in cortes {
            let prefijo = &bytes[..corte];
            std::fs::write(&path, prefijo).unwrap();

            // INDULGENTE (caps. 28/29): JAMÁS pánico, siempre Result y un
            // prefijo recuperado que cumple las invariantes del oráculo.
            let mut recuperado = MemoryStore::new();
            let wal_corto = Wal::reconstruir(prefijo);
            let informe = replay_wal(&mut recuperado, &wal_corto)
                .expect("la lectura indulgente nunca falla: parada limpia");
            assert!(
                verificar_invariantes(&recuperado).is_ok(),
                "corte {corte}: el prefijo resucitado viola invariantes"
            );

            // ESTRICTA (cap. 33): en memoria y desde FICHERO dicen LO MISMO.
            let en_memoria = decodificar_wal(prefijo);
            let en_fichero = cargar_wal_estricta(&path);
            match (&en_memoria, &en_fichero) {
                (Ok(memoria), Ok(fichero)) => {
                    assert_eq!(memoria, fichero, "corte {corte}");
                    assert_eq!(
                        wal_corto.record_count(),
                        memoria.len(),
                        "corte {corte}: indulgente y estricta desacuerdan"
                    );
                    assert_eq!(
                        informe.operaciones_reaplicadas,
                        operaciones_confirmadas(memoria),
                        "corte {corte}: el replay aplicó otra cosa"
                    );
                }
                (Err(_), Err(_)) => {}
                otro => panic!("corte {corte}: memoria y fichero divergen: {otro:?}"),
            }
        }
    }

    #[test]
    fn crash_bit_flip_bajo_crc_es_ruidoso() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bitflip.wal");
        let wal = wal_de_prueba();
        guardar_wal(&wal, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();

        // Volteamos bits DENTRO del cuerpo del último registro (entre su
        // length-prefix y su CRC): cualquier byte ahí está cubierto por el
        // CRC, así que la carga ESTRICTA debe gritar CrcInvalido — jamás un
        // Ok silencioso.
        let inicio = posicion_ultimo_registro(&bytes);
        let cuerpo_inicio = inicio + 4;
        let cuerpo_fin = bytes.len() - 4;
        assert!(cuerpo_fin > cuerpo_inicio);
        let total_registros = decodificar_wal(&bytes).unwrap().len();

        for k in cuerpo_inicio..cuerpo_fin {
            let mut corrupto = bytes.clone();
            corrupto[k] ^= 1u8 << (k % 8);

            match decodificar_wal(&corrupto) {
                Err(WalError::CrcInvalido { .. }) => {}
                otro => panic!("byte {k}: se esperaba CrcInvalido, llegó {otro:?}"),
            }
            std::fs::write(&path, &corrupto).unwrap();
            assert!(
                cargar_wal_estricta(&path).is_err(),
                "byte {k}: la carga estricta calló ante un CRC podrido"
            );

            // Y el HALLAZGO del capítulo, hecho test: el modo indulgente
            // TRAGA la cola sin avisar — desde el registro dañado, todo
            // desaparece del escaneo y nadie protesta.
            let wal_podrido = Wal::reconstruir(&corrupto);
            assert_eq!(
                wal_podrido.record_count(),
                total_registros - 1,
                "byte {k}: el silencio indulgente perdió más/menos de la cola"
            );
        }
    }

    #[test]
    fn crash_carga_estricta_reporta_cola_perdida() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cola.wal");
        let total_txs = 3usize;
        guardar_wal(&wal_de_prueba(), &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();

        // Estado sano de referencia (lo máximo recuperable jamás).
        let mut sano = MemoryStore::new();
        replay_wal(&mut sano, &Wal::reconstruir(&bytes)).unwrap();
        let nodos_sanos = sano.node_count();

        // Cortamos EN MEDIO del último registro (su Commit): cada corte
        // destruye la durabilidad de la tx3 y NADA más.
        let inicio_ultimo = posicion_ultimo_registro(&bytes);
        for corte in (inicio_ultimo + 1)..bytes.len() {
            let prefijo = &bytes[..corte];
            std::fs::write(&path, prefijo).unwrap();

            // ESTRICTA (cap. 33): lo GRITA con RegistroTruncado y cifras.
            match cargar_wal_estricta(&path) {
                Err(ErrorCargaEstricta::Wal(WalError::RegistroTruncado {
                    disponibles, ..
                })) => {
                    assert_eq!(
                        disponibles,
                        corte - inicio_ultimo,
                        "corte {corte}: el diagnóstico mide mal lo disponible"
                    );
                }
                otro => panic!("corte {corte}: se esperaba RegistroTruncado, llegó {otro:?}"),
            }

            // INDULGENTE (caps. 28/29): hereda el SILENCIO — recupera el
            // prefijo limpio, PIERDE la última transacción confirmada y no
            // avisa de nada. Ésa es exactamente la línea roja del capítulo.
            let wal_corto = cargar_wal(&path).unwrap();
            let mut heredado = MemoryStore::new();
            let informe =
                replay_wal(&mut heredado, &wal_corto).expect("la parada limpia tampoco falla aquí");
            assert!(
                verificar_invariantes(&heredado).is_ok(),
                "corte {corte}: lo heredado viola invariantes"
            );
            assert!(heredado.node_count() <= nodos_sanos);
            assert_eq!(
                informe.transacciones_confirmadas,
                total_txs - 1,
                "corte {corte}: la cola debió perderse EN SILENCIO (hallazgo)"
            );
        }
    }

    // ── Compatibilidad de formato (cap. 9) ───────────────────────

    #[test]
    fn compat_magic_erroneo_rechazado() {
        let mut cabecera = encode_header();
        cabecera[0] ^= 0xFF; // ya no es «LDB1»
        let err = decode_header(&cabecera).expect_err("un magic ajeno NO abre nuestra puerta");
        assert!(err.contains("magic"), "el rechazo nombra al magic: {err}");
    }

    #[test]
    fn compat_version_la_comprueba_el_llamador() {
        let cabecera = encode_header();
        // CONTRATO del cap. 9 tal cual es: decode_header valida el MAGIC y
        // DEVUELVE la versión sin juzgarla. Decidir si el formato es
        // compatible lo hace QUIEN ABRE («quien abre compara») — así el
        // migrador puede leer versiones futuras y decidir, en vez de que el
        // decoder le cierre la puerta por adelantado.
        let version = decode_header(&cabecera).unwrap();
        assert_eq!(version, FORMAT_VERSION);

        // La política del capítulo, hecha una función de dos líneas:
        // nuevo tag de Value ⇒ bump de versión ⇒ quien abra compara.
        let abrir = |v: u32| {
            if v == FORMAT_VERSION {
                Ok(())
            } else {
                Err(format!(
                    "formato versión {v} no soportado (esperaba {FORMAT_VERSION})"
                ))
            }
        };
        assert!(abrir(version).is_ok());

        // Una hipotética versión futura SIGUE siendo legible como número:
        // el decoder no la rechaza; la POLÍTICA sí.
        let mut futura = cabecera;
        futura[4..].copy_from_slice(&encode_u32_le(FORMAT_VERSION + 1));
        assert_eq!(decode_header(&futura).unwrap(), FORMAT_VERSION + 1);
        assert!(abrir(FORMAT_VERSION + 1).is_err());
    }
}
