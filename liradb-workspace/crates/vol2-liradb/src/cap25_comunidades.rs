use std::collections::{HashMap, VecDeque};
use std::fmt;

use crate::cap07_modelo::{EdgeId, NodeId};
use crate::cap08_graph_store::GraphStore;
use crate::cap22_caminos_minimos::{PathError, WeightSource, edge_weight};

// ─────────────────── Cap 25: Comunidades y agrupaciones ───────────────────
//
// Parte V (algoritmos sobre el grafo persistente), capítulo 4. El cap 24
// rankeó NODOS (¿quién es el centro?); éste PARTICIONA el grafo (¿quiénes
// forman grupo?). CORPUS vol-II-cap-25: "Modularidad; greedy Louvain".
// Guion (brief §cap 25): componentes, label propagation, modularidad,
// Louvain explicado e implementado en versión simplificada, casos de uso,
// limitaciones. Hito: detectar grupos en una red social pequeña.
//
// La historia en cuatro pasos (cada uno una familia del guion):
//
//   1. **Componentes conexas** ([`componentes_conexas`]): el caso límite de
//      "comunidad" — alcanzabilidad pura, sin densidad. BFS sobre la vista
//      simétrica. Es el suelo del concepto: toda componente es candidata a
//      comunidad, pero ninguna métrica decide todavía.
//
//   2. **Label propagation** ([`label_propagation`]): la primera heurística
//      DENSA — cada nodo adopta la etiqueta más votada entre sus vecinos
//      (votos ponderados por peso de arista). Rápido (casi lineal) y sin
//      función objetivo: NO optimiza nada verificable y su resultado depende
//      del orden. Ese es exactamente su papel pedagógico: la motivación de
//      Louvain.
//
//   3. **Modularidad** ([`modularidad`]): la métrica de Newman-Girvan (2004)
//      que le pone NÚMERO a una partición:
//
//      ```text
//        Q_γ = Σ_c [ Σ_in(c)/2m − γ·(Σ_tot(c)/2m)² ]
//      ```
//
//      "fracción de aristas internas menos la que ESPERARÍA el azar" (el
//      modelo nulo de configuración), con γ el parámetro de RESOLUCIÓN
//      (Reichardt-Bornholdt 2006): γ>1 exige comunidades más densas/chicas,
//      γ<1 tolera más sueltas/grandes. Q=0 para la partición trivial (todo
//      junto), Q>0 = mejor que el azar, Q<0 = peor. Es función VERIFICABLE:
//      calculable sobre CUALQUIER partición dada — es a la vez la métrica
//      guía de Louvain y el oráculo de sus tests.
//
//   4. **Louvain simplificado** ([`louvain`]): Blondel-Guillaume-Lambiotte-
//      Lefebvre (2008). Dos fases alternadas: (a) fase LOCAL greedy — cada
//      nodo se mueve a la comunidad del vecino que más aumente ΔQ (ΔQ exacto
//      por diferencia de los dos términos de comunidad que cambian) hasta
//      que una pasada completa no mueve a nadie; (b) AGREGACIÓN — cada
//      comunidad se contrae en un supernodo (pesos internos → self-loops) y
//      se repite (a) sobre el grafo contraído. Los NIVELES se apilan hasta
//      convergencia: la partición final es la composición de todas.
//
// Decisiones de diseño (los porqués):
//
// * **`GrafoPonderado` propio, no la `Proyeccion` del cap 24**: la proyección
//   del cap 24 es NO ponderada y su `GraphDirection::Both` hace unión como
//   CONJUNTO (deduplica aristas paralelas — correcto para contar vecinos
//   distintos, FALSO para Louvain, que debe SUMAR pesos). Louvain además
//   NECESITA reconstruir el grafo en cada agregación: el `GrafoPonderado` es
//   a la vez la proyección inicial Y el grafo de nivel (contraer = otro
//   `GrafoPonderado`). Se heredan del cap 24 el PATRÓN (nodos ordenados por
//   id → determinismo; índice denso que compacta huecos; materializar una
//   vez) y del cap 22 la semántica ESTRICTA de pesos (`WeightSource` +
//   `edge_weight`: propiedad ausente/NULL = `MissingWeight`, tipo no numérico
//   = `InvalidWeight`, NaN/±∞ = `NonFiniteWeight`). La proyección con pesos
//   COMPARTIDA por toda la Parte V sigue siendo deuda del cap 26.
//
// * **Simetrización sumando**: Louvain clásico es no dirigido con pesos.
//   Cada arista dirigida u→v con peso w aporta w al par no dirigido {u,v};
//   un store simetrizado a mano (u→v y v→u) SUMA 2w — documentado, y es la
//   convención que mantiene la analogía "dos mensajes = doble lazo".
//
// * **Self-loops cuentan DOBLE** (convención estándar del modelo nulo):
//   un self-loop de peso s entra en la adyacencia como A_ii = 2s, de modo
//   que k_i = 2s + Σ_j w_ij lo cuenta una vez por dirección y 2m = Σ_i k_i
//   lo refleja. En una red social, un self-loop es "relación consigo mismo"
//   (el KNOWS de Dani en `demo_graph`): refuerza la comunidad propia sin
//   unir a nadie.
//
// * **Pesos negativos rechazados eager** (como Dijkstra en el cap 22): la
//   modularidad con pesos negativos rompe la lectura "fracción de aristas"
//   y el modelo nulo; una BD prefiere fallar ruidosamente
//   ([`ComunidadesError::NegativeWeight`]) a contestar casi-bien. Los pesos
//   0 se admiten (aristas que no suman nada).
//
// * **Determinismo total, sin aleatoriedad**: el Louvain de la literatura
//   baraja el orden de los nodos (mejor exploración, resultados distintos
//   por ejecución). Aquí NO: nodos por id ascendente, comunidades candidatas
//   por id ascendente, empates de ΔQ por `total_cmp` → gana la primera (menor
//   id), y las comunidades se renumeran al final de cada nivel por su menor
//   miembro. Dos ejecuciones = resultado idéntico, testeado — un motor de BD
//   debe poder reproducir sus análisis.
//
// * **Cota de terminación demostrable**: en cada nivel la fase local arranca
//   de singletons, así que el PRIMER movimiento vacía una comunidad y el
//   número de nodos del siguiente nivel es estrictamente menor. Los niveles
//   con movimientos son ≤ V. `max_pasadas` limita además cada fase local
//   (seguro contra el ruido de f64 en ΔQ ≈ 0).
//
// * **La jerarquía es para el cap 51 (GraphRAG)**: [`NivelLouvain`] lleva la
//   asignación de los nodos ORIGINALES (composición de particiones), su Q y
//   su número de comunidades; los niveles están ANIDADOS por construcción
//   (cada comunidad del nivel ℓ+1 es unión de comunidades del nivel ℓ). El
//   cap 51 usará esa jerarquía para resúmenes globales a varias granularida-
//   des; la partición final con su Q basta para los locales.
//
// * **Aislados**: un nodo sin vecinos no tiene comunidad candidata — queda
//   como su propia comunidad singleton para siempre (semilla). Grafo vacío:
//   resultado vacío con Q=0. Grafo sin aristas: V singletons (Q = −Σ(k/2m)²
//   con k=0 → 0; la partición trivial y la de singletons coinciden cuando
//   no hay nada que repartir).
//
// * **Limitaciones declaradas** (la última sección del guion): es Louvain
//   SIMPLIFICADO — greedy puro sin la re-localización de Leiden (Traag-
//   Waltman-van Eck 2019), puede estancarse en óptimos locales (ΔQ>0
//   estricto), varios óptimos con Q igual se resuelven por orden (el test
//   de `demo_graph` documenta uno), y la modularidad misma sufre el LÍMITE
//   DE RESOLUCIÓN de Fortunato-Barthélemy (2007): comunidades más chicas
//   que √(2m)/2 aristas son invisibles para γ=1 — demostrado en tests con
//   el anillo de tríos canónico (γ=1 funde pares adyacentes; γ=2 los
//   recupera).
//
// * **Coste computacional** (sección del guion, medible en
//   [`ComunidadesStats`]): componentes O(V+E); LPA O(pasadas·E); fase local
//   O(pasadas·E) por nivel; agregación O(E) por nivel; niveles ≤ V pero en
//   la práctica O(log V) — el tamaño del grafo se contrae geométricamente.

// ─── Errores ───

/// Errores de las comunidades del cap 25.
#[derive(Debug, Clone, PartialEq)]
pub enum ComunidadesError {
    /// La resolución γ debe ser > 0 y finita: γ ≤ 0 rompe la lectura
    /// "mejor que el azar" de la modularidad (con γ=0, juntarlo todo ya
    /// maximiza Q) y el invariante de monotonía de Louvain.
    InvalidResolution { value: f64 },
    /// El máximo de pasadas por nivel debe ser ≥ 1.
    InvalidMaxPasadas { value: u64 },
    /// Una arista con peso negativo: la modularidad con pesos negativos
    /// rompe el modelo nulo y la lectura "fracción de aristas".
    NegativeWeight { edge: EdgeId, weight: f64 },
    /// Peso inválido leído con la semántica estricta del cap 22
    /// (prop ausente/NULL, tipo no numérico, NaN/±∞).
    Weight(PathError),
    /// La partición dada menciona un nodo que no existe en el store.
    UnknownNode(NodeId),
}

impl fmt::Display for ComunidadesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComunidadesError::InvalidResolution { value } => write!(
                f,
                "invalid resolution gamma {value}: must be > 0 and finite"
            ),
            ComunidadesError::InvalidMaxPasadas { value } => {
                write!(f, "invalid max passes {value}: must be >= 1")
            }
            ComunidadesError::NegativeWeight { edge, weight } => write!(
                f,
                "edge {edge} has negative weight {weight}: community detection requires non-negative weights"
            ),
            ComunidadesError::Weight(e) => write!(f, "invalid edge weight: {e}"),
            ComunidadesError::UnknownNode(id) => write!(f, "unknown node id {id}"),
        }
    }
}

impl std::error::Error for ComunidadesError {}

impl From<PathError> for ComunidadesError {
    fn from(e: PathError) -> Self {
        ComunidadesError::Weight(e)
    }
}

// ─── Stats ───

/// Estadísticas del cálculo — la sección "coste computacional" del guion,
/// medible en vez de declamada (hereda el papel de [`crate::CentralidadStats`]).
///
/// - `edges_scanned`: aristas leídas del store durante la proyección.
/// - `pasadas`: pasadas de la fase local (Louvain, sumadas entre niveles e
///   incluyendo la pasada vacía que corta) o de LPA.
/// - `movimientos`: nodos movidos de comunidad (Louvain) o cambios de
///   etiqueta (LPA).
/// - `niveles`: niveles grabados en la jerarquía de Louvain (sólo los que
///   mueven nodos; el nivel de corte no cuenta).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComunidadesStats {
    /// Aristas leídas durante la proyección del store.
    pub edges_scanned: u64,
    /// Pasadas de la fase local (Louvain) o de propagación (LPA).
    pub pasadas: u64,
    /// Movimientos de nodo (Louvain) o cambios de etiqueta (LPA).
    pub movimientos: u64,
    /// Niveles grabados en la jerarquía (Louvain).
    pub niveles: u64,
}

// ─── Partición: la interfaz común de los tres algoritmos ───

/// Asignación de nodos a grupos: nodos ordenados por id + grupo denso por
/// posición.
///
/// Es la MISMA interfaz para [`componentes_conexas`],
/// [`label_propagation`] y [`louvain`] (y la que el cap 51 —GraphRAG—
/// consumirá para agrupar documentos): `grupo(id)` responde la comunidad de
/// un nodo, `grupos()` entrega los miembros por grupo en orden determinista.
///
/// Los ids de grupo son DENSOS (0..k−1) y están ordenados por su menor
/// miembro: el grupo 0 contiene al nodo de id más pequeño (renumeración que
/// hace comparables dos ejecuciones o dos algoritmos).
#[derive(Debug, Clone, PartialEq)]
pub struct Particion {
    /// Nodos vivos en orden ascendente de id (determinismo).
    nodes: Vec<NodeId>,
    /// Grupo por posición densa (alineada con `nodes`).
    grupo: Vec<u64>,
}

impl Particion {
    /// Construye una partición alineada (nodes ↔ grupo, misma longitud).
    fn nueva(nodes: Vec<NodeId>, grupo: Vec<u64>) -> Self {
        assert_eq!(nodes.len(), grupo.len(), "partición desalineada");
        Particion { nodes, grupo }
    }

    /// Grupo del nodo `id`, o `None` si no existe en el resultado.
    pub fn grupo(&self, id: NodeId) -> Option<u64> {
        self.nodes.binary_search(&id).ok().map(|i| self.grupo[i])
    }

    /// Número de grupos distintos.
    pub fn num_grupos(&self) -> usize {
        self.grupo
            .iter()
            .copied()
            .max()
            .map_or(0, |m| m as usize + 1)
    }

    /// Pares (nodo, grupo) en orden de id.
    pub fn entries(&self) -> Vec<(NodeId, u64)> {
        self.nodes
            .iter()
            .copied()
            .zip(self.grupo.iter().copied())
            .collect()
    }

    /// Miembros por grupo: `grupos()[g]` = nodos del grupo `g` en orden
    /// ascendente de id.
    pub fn grupos(&self) -> Vec<Vec<NodeId>> {
        let mut out: Vec<Vec<NodeId>> = vec![Vec::new(); self.num_grupos()];
        for (&n, &g) in self.nodes.iter().zip(self.grupo.iter()) {
            out[g as usize].push(n);
        }
        out.retain(|g| !g.is_empty());
        out
    }

    /// Tamaño de cada grupo (alineado con [`Particion::grupos`]).
    pub fn tamanos(&self) -> Vec<usize> {
        self.grupos().iter().map(Vec::len).collect()
    }

    /// Número de nodos.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// ¿Resultado vacío (store sin nodos)?
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Los nodos ordenados (uso interno de [`LouvainResult::particion_en`]).
    fn nodes_id(&self) -> &[NodeId] {
        &self.nodes
    }
}

impl fmt::Display for Particion {
    /// Formato tipo tabla: `n0=0 n1=0 n2=1 ...` en orden de id.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (n, g)) in self.entries().iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "n{}={}", n, g)?;
        }
        Ok(())
    }
}

// ─── Proyección ponderada simétrica ───

/// Término de comunidad de la modularidad: Σ_in(c)/2m − γ·(Σ_tot(c)/2m)².
fn q_com(in_interno: f64, k_total: f64, dos_m: f64, gamma: f64) -> f64 {
    in_interno / dos_m - gamma * (k_total / dos_m) * (k_total / dos_m)
}

/// Grafo no dirigido ponderado: la proyección del store que Louvain necesita
/// Y el grafo contraído de cada nivel de agregación.
///
/// * Simétrico: cada arista dirigida u→v con peso w aporta w al par {u,v}
///   (un store simetrizado a mano SUMA: 2w por par antiparalelo).
/// * Aristas paralelas ACUMULADAS por par (multigrafo → multipeso).
/// * Self-loops SEPARADOS (`self_loop[i] = s`) con la convención estándar
///   A_ii = 2s: cuentan doble en k_i y en 2m.
/// * Vecindarios ordenados por índice → iteración determinista.
#[derive(Debug, Clone)]
struct GrafoPonderado {
    /// Nodos (ids reales en la proyección; 0..C−1 como supernodos tras
    /// contraer) en orden ascendente.
    nodes: Vec<NodeId>,
    /// Peso s_i de los self-loops acumulados (sin el ×2 de la convención).
    self_loop: Vec<f64>,
    /// Vecinos distintos (índice denso, peso acumulado), ordenados.
    vecinos: Vec<Vec<(usize, f64)>>,
    /// Grado ponderado k_i = 2·s_i + Σ_j w_ij (self-loop contado doble).
    k: Vec<f64>,
    /// 2m = Σ_i k_i (peso total contando cada arista en ambas direcciones).
    dos_m: f64,
    /// Aristas leídas del store durante la proyección.
    edges_scanned: u64,
}

impl GrafoPonderado {
    fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Posición densa de un id de nodo (None = hueco o inexistente).
    fn posicion(&self, id: NodeId) -> Option<usize> {
        self.nodes.binary_search(&id).ok()
    }

    /// Proyecta el store en un grafo simétrico ponderado.
    ///
    /// Lee los pesos con la semántica ESTRICTA del cap 22 (`edge_weight`) y
    /// rechaza los negativos eager ([`ComunidadesError::NegativeWeight`]).
    fn proyectar(store: &dyn GraphStore, weight: &WeightSource) -> Result<Self, ComunidadesError> {
        let mut nodes: Vec<NodeId> = store.iter_nodes().map(|n| n.id).collect();
        nodes.sort_unstable();

        let table_len = nodes.last().map_or(0, |&m| m + 1);
        let mut index: Vec<Option<usize>> = vec![None; table_len];
        for (i, &n) in nodes.iter().enumerate() {
            index[n] = Some(i);
        }

        let n = nodes.len();
        let mut filas: Vec<HashMap<usize, f64>> = (0..n).map(|_| HashMap::new()).collect();
        let mut self_loop = vec![0.0_f64; n];
        let mut edges_scanned = 0u64;

        for (iu, &u) in nodes.iter().enumerate() {
            for eid in store.out_edges(u) {
                let edge = store
                    .get_edge(eid)
                    .expect("invariante del store: la adjacencia sólo contiene aristas vivas");
                let w = edge_weight(edge, weight)?;
                if w < 0.0 {
                    return Err(ComunidadesError::NegativeWeight {
                        edge: edge.id,
                        weight: w,
                    });
                }
                if edge.target == u {
                    // Self-loop: A_ii = 2s se materializa en k (abajo).
                    self_loop[iu] += w;
                } else if let Some(iv) = index[edge.target] {
                    // Simetrización SUMANDO: esta arista dirigida aporta w
                    // al par no dirigido en ambas direcciones de la matriz.
                    *filas[iu].entry(iv).or_insert(0.0) += w;
                    *filas[iv].entry(iu).or_insert(0.0) += w;
                }
                edges_scanned += 1;
            }
        }

        let mut vecinos = Vec::with_capacity(n);
        let mut k = vec![0.0_f64; n];
        for (t, fila) in filas.iter().enumerate() {
            let mut lista: Vec<(usize, f64)> = fila.iter().map(|(&j, &w)| (j, w)).collect();
            lista.sort_unstable_by_key(|&(j, _)| j);
            k[t] = 2.0 * self_loop[t] + lista.iter().map(|(_, w)| *w).sum::<f64>();
            vecinos.push(lista);
        }
        let dos_m = k.iter().sum();

        Ok(GrafoPonderado {
            nodes,
            self_loop,
            vecinos,
            k,
            dos_m,
            edges_scanned,
        })
    }

    /// Contrae una partición de ESTE grafo en supernodos: las aristas
    /// internas se vuelven self-loops del supernodo, las externas suman
    /// pesos entre supernodos. El peso total (2m) se conserva — por eso la
    /// modularidad de un nivel es igual calculada en el grafo contraído o
    /// en el original (invariante testeada).
    fn contraer(&self, com: &[usize]) -> GrafoPonderado {
        let num = com.iter().copied().max().map_or(0, |m| m + 1);
        let mut filas: Vec<HashMap<usize, f64>> = (0..num).map(|_| HashMap::new()).collect();
        let mut self_loop = vec![0.0_f64; num];

        for (i, vecinos_i) in self.vecinos.iter().enumerate() {
            // El self-loop VIEJO del nodo viaja con su comunidad.
            self_loop[com[i]] += self.self_loop[i];
            for &(j, w) in vecinos_i {
                if j <= i {
                    continue; // adyacencia simétrica: cada par una vez
                }
                if com[i] == com[j] {
                    // Arista interna → self-loop del supernodo.
                    self_loop[com[i]] += w;
                } else {
                    *filas[com[i]].entry(com[j]).or_insert(0.0) += w;
                    *filas[com[j]].entry(com[i]).or_insert(0.0) += w;
                }
            }
        }

        let mut vecinos = Vec::with_capacity(num);
        let mut k = vec![0.0_f64; num];
        for t in 0..num {
            let mut lista: Vec<(usize, f64)> = filas[t].iter().map(|(&j, &w)| (j, w)).collect();
            lista.sort_unstable_by_key(|&(j, _)| j);
            k[t] = 2.0 * self_loop[t] + lista.iter().map(|(_, w)| *w).sum::<f64>();
            vecinos.push(lista);
        }
        let dos_m = k.iter().sum();

        GrafoPonderado {
            nodes: (0..num).collect(),
            self_loop,
            vecinos,
            k,
            dos_m,
            edges_scanned: 0,
        }
    }

    /// Modularidad Q_γ de una asignación densa sobre ESTE grafo.
    ///
    /// Σ_in(c) recorre la adyacencia completa: cada arista interna suma w
    /// desde ambos extremos (2× el peso interno) y cada self-loop aporta su
    /// A_ii = 2s. Con 2m = 0 (sin aristas) Q se define 0.
    fn modularidad_de(&self, com: &[usize], gamma: f64) -> f64 {
        if self.dos_m == 0.0 {
            return 0.0;
        }
        let num = com.iter().copied().max().map_or(0, |m| m + 1);
        let mut in_interno = vec![0.0_f64; num];
        let mut k_total = vec![0.0_f64; num];
        for (i, vecinos_i) in self.vecinos.iter().enumerate() {
            let ci = com[i];
            in_interno[ci] += 2.0 * self.self_loop[i];
            k_total[ci] += self.k[i];
            for &(j, w) in vecinos_i {
                if com[j] == ci {
                    in_interno[ci] += w;
                }
            }
        }
        (0..num)
            .map(|c| q_com(in_interno[c], k_total[c], self.dos_m, gamma))
            .sum()
    }

    /// Fase LOCAL de Louvain: greedy de movimientos que maximizan ΔQ.
    ///
    /// Empieza en singletons y barre los nodos por índice ascendente
    /// (determinismo); para cada nodo evalúa moverse a cada comunidad
    /// VECINA con el ΔQ exacto — sólo cambian los términos de la comunidad
    /// propia y la de destino:
    ///
    /// ```text
    ///   ΔQ = q(In_c + 2·k_{i,c} + 2·s_i , K_c + k_i)
    ///      + q(In_d − 2·k_{i,d} − 2·s_i , K_d − k_i)
    ///      − q(In_c, K_c) − q(In_d, K_d)
    /// ```
    ///
    /// (d = comunidad actual de i, s_i su self-loop, k_{i,c} el peso de i
    /// hacia c sin contar el self-loop). Se mueve sólo con ΔQ > 0 ESTRICTO;
    /// empates → comunidad de menor id. Termina cuando una pasada completa
    /// no mueve a nadie (o al agotar `max_pasadas`). Devuelve la asignación,
    /// los movimientos y las pasadas.
    fn fase_local(
        &self,
        gamma: f64,
        max_pasadas: u64,
        stats: &mut ComunidadesStats,
    ) -> (Vec<usize>, u64, u64) {
        let n = self.len();
        let mut com: Vec<usize> = (0..n).collect();
        // Σ_in y Σ_tot vivos por comunidad (singletons: 2s y k).
        let mut in_com: Vec<f64> = self.self_loop.iter().map(|s| 2.0 * s).collect();
        let mut k_com: Vec<f64> = self.k.clone();
        let mut movimientos = 0u64;
        let mut pasadas = 0u64;

        if n == 0 || self.dos_m == 0.0 {
            return (com, 0, 0);
        }

        for _ in 1..=max_pasadas {
            pasadas += 1;
            let mut cambios = 0u64;
            for i in 0..n {
                // Peso de i hacia cada comunidad vecina (las paralelas ya
                // están sumadas en la proyección; falta agrupar por comunidad).
                let mut candidatos: Vec<(usize, f64)> = Vec::new();
                for &(j, w) in &self.vecinos[i] {
                    let c = com[j];
                    match candidatos.iter_mut().find(|(cc, _)| *cc == c) {
                        Some(x) => x.1 += w,
                        None => candidatos.push((c, w)),
                    }
                }
                if candidatos.is_empty() {
                    continue; // aislado: sin comunidad candidata
                }
                let propio = com[i];
                let w_propio = candidatos
                    .iter()
                    .find(|(c, _)| *c == propio)
                    .map(|(_, w)| *w)
                    .unwrap_or(0.0);
                // Determinismo: candidatos evaluados por id de comunidad
                // ascendente; el primero con ΔQ máximo gana los empates.
                candidatos.sort_unstable_by_key(|&(c, _)| c);

                let mut mejor: Option<(usize, f64, f64)> = None; // (c, w_ic, ΔQ)
                for &(c, w_ic) in &candidatos {
                    if c == propio {
                        continue;
                    }
                    let in_c = in_com[c] + 2.0 * w_ic + 2.0 * self.self_loop[i];
                    let k_c = k_com[c] + self.k[i];
                    let in_d = in_com[propio] - 2.0 * w_propio - 2.0 * self.self_loop[i];
                    let k_d = k_com[propio] - self.k[i];
                    let dq = q_com(in_c, k_c, self.dos_m, gamma)
                        + q_com(in_d, k_d, self.dos_m, gamma)
                        - q_com(in_com[c], k_com[c], self.dos_m, gamma)
                        - q_com(in_com[propio], k_com[propio], self.dos_m, gamma);
                    if dq > 0.0
                        && mejor
                            .as_ref()
                            .is_none_or(|(_, _, m)| dq.total_cmp(m) == std::cmp::Ordering::Greater)
                    {
                        mejor = Some((c, w_ic, dq));
                    }
                }

                if let Some((c, w_ic, _)) = mejor {
                    in_com[propio] -= 2.0 * w_propio + 2.0 * self.self_loop[i];
                    k_com[propio] -= self.k[i];
                    in_com[c] += 2.0 * w_ic + 2.0 * self.self_loop[i];
                    k_com[c] += self.k[i];
                    com[i] = c;
                    movimientos += 1;
                    cambios += 1;
                }
            }
            if cambios == 0 {
                break;
            }
        }

        stats.pasadas += pasadas;
        stats.movimientos += movimientos;
        (com, movimientos, pasadas)
    }
}

/// Renumera etiquetas arbitrarias a ids densos 0..k−1 ordenados por el
/// MENOR miembro (posición densa) de cada etiqueta: el grupo 0 contiene al
/// nodo de menor id. Hace comparables dos ejecuciones o dos algoritmos.
fn densificar(com: &[usize], min_miembro: &[usize]) -> Vec<usize> {
    let mut clave: HashMap<usize, usize> = HashMap::new();
    for (t, &c) in com.iter().enumerate() {
        let m = min_miembro[t];
        clave
            .entry(c)
            .and_modify(|x| {
                if m < *x {
                    *x = m;
                }
            })
            .or_insert(m);
    }
    let mut pares: Vec<(usize, usize)> = clave.into_iter().collect();
    pares.sort_unstable_by_key(|&(_, m)| m);
    let mapa: HashMap<usize, usize> = pares
        .iter()
        .enumerate()
        .map(|(nuevo, (viejo, _))| (*viejo, nuevo))
        .collect();
    com.iter().map(|&c| mapa[&c]).collect()
}

/// Validación compartida de la resolución γ.
fn validar_resolucion(gamma: f64) -> Result<(), ComunidadesError> {
    if !gamma.is_finite() || gamma <= 0.0 {
        return Err(ComunidadesError::InvalidResolution { value: gamma });
    }
    Ok(())
}

/// Validación compartida del máximo de pasadas.
fn validar_max_pasadas(max_pasadas: u64) -> Result<(), ComunidadesError> {
    if max_pasadas < 1 {
        return Err(ComunidadesError::InvalidMaxPasadas { value: max_pasadas });
    }
    Ok(())
}

// ─── Componentes conexas ───

/// Resultado de [`componentes_conexas`].
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentesResult {
    /// Partición: una componente por grupo (renumerada por menor miembro).
    pub particion: Particion,
    /// Estadísticas del cálculo.
    pub stats: ComunidadesStats,
}

impl ComponentesResult {
    /// Componente del nodo `id`, o `None` si no existe en el resultado.
    pub fn componente(&self, id: NodeId) -> Option<u64> {
        self.particion.grupo(id)
    }

    /// Número de componentes.
    pub fn num_componentes(&self) -> usize {
        self.particion.num_grupos()
    }

    /// Miembros de cada componente (orden ascendente de id dentro).
    pub fn componentes(&self) -> Vec<Vec<NodeId>> {
        self.particion.grupos()
    }
}

impl fmt::Display for ComponentesResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Componentes(nodos={}, componentes={})",
            self.particion.len(),
            self.num_componentes()
        )
    }
}

/// Componentes conexas sobre la vista NO dirigida del store.
///
/// El caso límite del concepto de comunidad: alcanzabilidad pura, sin
/// densidad. Cada arista dirigida se lee en ambos sentidos (un 0→1 une 0 y
/// 1); los pesos son irrelevantes para la alcanzabilidad — se proyecta con
/// peso constante 1. Las componentes se numeran por su menor miembro
/// (determinismo). O(V+E): un BFS por componente.
///
/// Las componentes FUERTEMENTE conexas (dirección respetada) son otro
/// algoritmo (Tarjan, Vol.I cap 7) y otra pregunta: aquí la vista es la
/// simétrica, la misma de las comunidades.
///
/// ```
/// use vol2_liradb::{componentes_conexas, Edge, MemoryStore, Node};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..4 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Dos pares unidos, sin puente entre ellos (dirigidos, pero la vista
/// // de componentes es simétrica).
/// for (a, b) in [(0, 1), (2, 3)] {
///     s.put_edge(Edge::new(a, a, b, "E")).unwrap();
/// }
///
/// let r = componentes_conexas(&s).unwrap();
/// assert_eq!(r.num_componentes(), 2);
/// assert_eq!(r.componente(0), r.componente(1));
/// assert_ne!(r.componente(0), r.componente(2));
/// ```
pub fn componentes_conexas(store: &dyn GraphStore) -> Result<ComponentesResult, ComunidadesError> {
    let g = GrafoPonderado::proyectar(store, &WeightSource::Constant(1.0))?;
    let n = g.len();
    let stats = ComunidadesStats {
        edges_scanned: g.edges_scanned,
        ..ComunidadesStats::default()
    };

    // El barrido por índice ascendente numera las componentes por su menor
    // miembro: la renumeración canónica sale gratis.
    let mut componente = vec![usize::MAX; n];
    let mut num = 0usize;
    for s in 0..n {
        if componente[s] != usize::MAX {
            continue;
        }
        let mut cola: VecDeque<usize> = VecDeque::new();
        componente[s] = num;
        cola.push_back(s);
        while let Some(v) = cola.pop_front() {
            for &(w, _) in &g.vecinos[v] {
                if componente[w] == usize::MAX {
                    componente[w] = num;
                    cola.push_back(w);
                }
            }
        }
        num += 1;
    }

    let particion = Particion::nueva(
        g.nodes.clone(),
        componente.iter().map(|&c| c as u64).collect(),
    );
    Ok(ComponentesResult { particion, stats })
}

// ─── Label propagation ───

/// Resultado de [`label_propagation`].
#[derive(Debug, Clone, PartialEq)]
pub struct LabelPropagationResult {
    /// Partición: una comunidad por etiqueta estable (renumerada por menor
    /// miembro).
    pub particion: Particion,
    /// Estadísticas del cálculo.
    pub stats: ComunidadesStats,
}

impl LabelPropagationResult {
    /// Etiqueta/comunidad del nodo `id`, o `None` si no existe.
    pub fn etiqueta(&self, id: NodeId) -> Option<u64> {
        self.particion.grupo(id)
    }

    /// Número de comunidades.
    pub fn num_comunidades(&self) -> usize {
        self.particion.num_grupos()
    }

    /// Miembros de cada comunidad (orden ascendente de id dentro).
    pub fn comunidades(&self) -> Vec<Vec<NodeId>> {
        self.particion.grupos()
    }
}

impl fmt::Display for LabelPropagationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LabelPropagation(pasadas={}, comunidades={})",
            self.stats.pasadas,
            self.num_comunidades()
        )
    }
}

/// Label propagation (LPA, Raghavan-Albert-Kumara 2007) sobre la vista
/// simétrica ponderada del store.
///
/// Cada nodo empieza con su propia etiqueta; en cada pasada (nodos por id
/// ascendente, actualización ASÍNCRONA — el estándar) adopta la etiqueta
/// más votada entre sus vecinos, con votos PONDERADOS por el peso de la
/// arista (`Constant(1.0)` = LPA clásico contando vecinos). Empates: si la
/// etiqueta PROPIA empata con la más votada se CONSERVA (frena el goteo
/// asimétrico de la primera pasada); si no, gana la MENOR de las empatadas
/// — el LPA original desempata al azar; aquí no hay aleatoriedad (dos
/// ejecuciones = mismo resultado). Termina cuando una pasada no cambia
/// ninguna etiqueta (o al agotar `max_pasadas`, seguro contra
/// oscilaciones). Los aislados conservan su etiqueta.
///
/// Límite pedagógico (por eso Louvain): no optimiza ninguna función
/// objetivo verificable — no hay Q que comprobar — y con pesos UNIFORMES
/// los empates de la primera pasada pueden GOTTEAR por los puentes (el
/// primer grupo que se forma arrastra al vecino del puente, cuya propia
/// etiqueta aún no reúne votos: testeado frente a Louvain, que sobre el
/// MISMO grafo sin pesos sí separa). Romper los empates con pesos es la
/// receta práctica.
///
/// ```
/// use vol2_liradb::{label_propagation, Edge, MemoryStore, Node, Value, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..6 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Dos tríos con un puente FLOJO: los pesos rompen los empates que
/// // harían gotear las etiquetas por el puente.
/// let mut eid = 0;
/// for (a, b, w) in [(0,1,1.0),(0,2,1.0),(1,2,1.0),
///                   (3,4,1.0),(3,5,1.0),(4,5,1.0),(2,3,0.5)] {
///     for (x, y) in [(a, b), (b, a)] {
///         s.put_edge(Edge::new(eid, x, y, "E").with_prop("w", Value::Float(w))).unwrap();
///         eid += 1;
///     }
/// }
///
/// let r = label_propagation(&s, &WeightSource::property("w"), 20).unwrap();
/// assert_eq!(r.num_comunidades(), 2);
/// assert_eq!(r.etiqueta(0), r.etiqueta(2));
/// assert_ne!(r.etiqueta(0), r.etiqueta(4));
/// ```
pub fn label_propagation(
    store: &dyn GraphStore,
    weight: &WeightSource,
    max_pasadas: u64,
) -> Result<LabelPropagationResult, ComunidadesError> {
    validar_max_pasadas(max_pasadas)?;
    let g = GrafoPonderado::proyectar(store, weight)?;
    let n = g.len();
    let mut stats = ComunidadesStats {
        edges_scanned: g.edges_scanned,
        ..ComunidadesStats::default()
    };

    let mut etiqueta: Vec<usize> = (0..n).collect();
    let mut movimientos = 0u64;
    let mut pasadas = 0u64;

    for _ in 1..=max_pasadas {
        pasadas += 1;
        let mut cambios = 0u64;
        for i in 0..n {
            // Votos ponderados por etiqueta vecina.
            let mut votos: Vec<(usize, f64)> = Vec::new();
            for &(j, w) in &g.vecinos[i] {
                let e = etiqueta[j];
                match votos.iter_mut().find(|(ee, _)| *ee == e) {
                    Some(x) => x.1 += w,
                    None => votos.push((e, w)),
                }
            }
            if votos.is_empty() {
                continue; // aislado: conserva su etiqueta
            }
            // Orden: peso DESCENDENTE, etiqueta ASCENDENTE — votos[0] es la
            // menor etiqueta entre las más votadas (determinismo). La
            // PROPIA se conserva si empata con la máxima.
            votos.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            let max_votos = votos[0].1;
            let mejor = match votos.iter().find(|&&(e, _)| e == etiqueta[i]) {
                Some(&(_, v)) if v.total_cmp(&max_votos) == std::cmp::Ordering::Equal => {
                    etiqueta[i]
                }
                _ => votos[0].0,
            };
            if mejor != etiqueta[i] {
                etiqueta[i] = mejor;
                movimientos += 1;
                cambios += 1;
            }
        }
        if cambios == 0 {
            break;
        }
    }

    stats.pasadas = pasadas;
    stats.movimientos = movimientos;

    // Renumeración canónica por menor miembro (la etiqueta inicial de cada
    // nodo es su propio índice: min_miembro = identidad).
    let densa = densificar(&etiqueta, &(0..n).collect::<Vec<_>>());
    let particion = Particion::nueva(g.nodes.clone(), densa.iter().map(|&c| c as u64).collect());
    Ok(LabelPropagationResult { particion, stats })
}

// ─── Modularidad ───

/// Modularidad Q_γ (Newman-Girvan 2004; γ de Reichardt-Bornholdt 2006) de
/// una partición dada sobre el grafo del store.
///
/// ```text
///   Q_γ = Σ_c [ Σ_in(c)/2m − γ·(Σ_tot(c)/2m)² ]
/// ```
///
/// "fracción de peso interno menos la que esperaría el azar" bajo el modelo
/// nulo de configuración. Q=0 en la partición trivial (todo junto); Q>0 =
/// mejor que el azar; Q<0 = peor. γ>1 exige comunidades más densas y
/// pequeñas, γ<1 tolera más laxas (γ=1 es el clásico). Escalar TODOS los
/// pesos por una constante no cambia Q (es una fracción) — testeado.
///
/// Semántica del grafo: la MISMA proyección simétrica de Louvain (aristas
/// dirigidas sumadas al par, paralelas acumuladas, self-loops ×2). La
/// partición es una lista (nodo, grupo): los grupos pueden ser cualquier
/// u64 (se densifican internamente), los nodos deben existir
/// ([`ComunidadesError::UnknownNode`]); nodos del store AUSENTES de la
/// partición cuentan como singletons (cada uno su grupo); un nodo repetido
/// queda con su última asignación.
///
/// Es la métrica guía de [`louvain`] Y su oráculo de tests: Q de la
/// partición devuelta == `modularidad` de la misma partición.
///
/// ```
/// use vol2_liradb::{modularidad, Edge, MemoryStore, Node, NodeId, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..6 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Dos tríos (0-1-2 y 3-4-5) unidos por un puente 1-4.
/// for (a, b) in [(0,1),(0,2),(1,2),(3,4),(3,5),(4,5),(1,4)] {
///     s.put_edge(Edge::new(a * 8 + b, a, b, "E")).unwrap();
///     s.put_edge(Edge::new(b * 8 + a, b, a, "E")).unwrap();
/// }
///
/// let particion: Vec<(NodeId, u64)> =
///     vec![(0,0),(1,0),(2,0),(3,1),(4,1),(5,1)];
/// // Q de la partición perfecta: 5/14 (calculado a mano en los tests).
/// let q = modularidad(&s, &particion, &WeightSource::Constant(1.0), 1.0).unwrap();
/// assert!((q - 5.0 / 14.0).abs() < 1e-12);
///
/// // La partición trivial (todo junto) es exactamente 0.
/// let trivial: Vec<(NodeId, u64)> = (0..6).map(|i| (i, 0)).collect();
/// assert_eq!(modularidad(&s, &trivial, &WeightSource::Constant(1.0), 1.0).unwrap(), 0.0);
/// ```
pub fn modularidad(
    store: &dyn GraphStore,
    particion: &[(NodeId, u64)],
    weight: &WeightSource,
    gamma: f64,
) -> Result<f64, ComunidadesError> {
    validar_resolucion(gamma)?;
    let g = GrafoPonderado::proyectar(store, weight)?;
    let n = g.len();

    // Densificar los ids de grupo de la partición: el orden es irrelevante
    // para Q (sólo importa el agrupamiento).
    let mut asig = vec![usize::MAX; n];
    let mut densas: HashMap<u64, usize> = HashMap::new();
    for &(id, c) in particion {
        let i = g.posicion(id).ok_or(ComunidadesError::UnknownNode(id))?;
        let siguiente = densas.len();
        let d = *densas.entry(c).or_insert(siguiente);
        asig[i] = d;
    }
    // Nodos ausentes → singleton (documentado arriba).
    let mut siguiente = densas.len();
    for a in &mut asig {
        if *a == usize::MAX {
            *a = siguiente;
            siguiente += 1;
        }
    }

    Ok(g.modularidad_de(&asig, gamma))
}

// ─── Louvain ───

/// Un nivel de la jerarquía de Louvain: el estado de la partición de los
/// nodos ORIGINALES al cerrar ese nivel.
///
/// La asignación está COMPUESTA (nivel 0 sobre el grafo real; nivel ℓ+1
/// contrae el ℓ), así que los niveles están anidados por construcción: cada
/// comunidad del nivel ℓ+1 es unión exacta de comunidades del nivel ℓ. Esta
/// jerarquía —no sólo la partición final— es lo que el cap 51 (GraphRAG)
/// usará para resúmenes a varias granularidades.
#[derive(Debug, Clone, PartialEq)]
pub struct NivelLouvain {
    /// Nivel (0 = grafo original).
    pub nivel: usize,
    /// Número de comunidades al cerrar el nivel.
    pub num_comunidades: usize,
    /// Modularidad Q_γ de la partición del nivel (sobre el grafo original —
    /// la contracción la conserva, invariante testeada).
    pub modularidad: f64,
    /// Pasadas de la fase local en este nivel.
    pub pasadas: u64,
    /// Movimientos de nodo de la fase local en este nivel.
    pub movimientos: u64,
    /// Comunidad (renumerada por menor miembro) de cada nodo ORIGINAL, en
    /// el orden de [`LouvainResult`] (`particion.entries()`).
    pub asignacion: Vec<u64>,
}

/// Resultado de [`louvain`].
#[derive(Debug, Clone, PartialEq)]
pub struct LouvainResult {
    /// Partición FINAL (la del último nivel; singletons si el primer nivel
    /// no movió a nadie).
    pub particion: Particion,
    /// Jerarquía de niveles (sólo los que movieron nodos).
    pub niveles: Vec<NivelLouvain>,
    /// Modularidad Q_γ de la partición final sobre el grafo original.
    pub modularidad: f64,
    /// Estadísticas del cálculo.
    pub stats: ComunidadesStats,
}

impl LouvainResult {
    /// Comunidad final del nodo `id`, o `None` si no existe en el resultado.
    pub fn comunidad(&self, id: NodeId) -> Option<u64> {
        self.particion.grupo(id)
    }

    /// Número de comunidades de la partición final.
    pub fn num_comunidades(&self) -> usize {
        self.particion.num_grupos()
    }

    /// Miembros de cada comunidad final (orden ascendente de id dentro).
    pub fn comunidades(&self) -> Vec<Vec<NodeId>> {
        self.particion.grupos()
    }

    /// Partición completa de un nivel de la jerarquía (la composición hasta
    /// ese nivel), o `None` si el nivel no existe. Es la vista del
    /// dendrograma que el cap 51 (GraphRAG) consumirá.
    pub fn particion_en(&self, nivel: usize) -> Option<Particion> {
        self.niveles
            .get(nivel)
            .map(|nv| Particion::nueva(self.particion.nodes_id().to_vec(), nv.asignacion.clone()))
    }
}

impl fmt::Display for LouvainResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Louvain(niveles={}, comunidades={}, Q={:.4}, movimientos={})",
            self.stats.niveles,
            self.num_comunidades(),
            self.modularidad,
            self.stats.movimientos
        )
    }
}

/// Louvain simplificado (Blondel et al. 2008) sobre el grafo del store.
///
/// Alterna dos fases por nivel hasta que una fase local no mueva a nadie:
///
/// 1. **Fase local greedy** ([`GrafoPonderado::fase_local`]): desde
///    singletons, cada nodo (por id ascendente — determinismo, sin barajar)
///    se mueve a la comunidad vecina con mayor ΔQ exacto, sólo si ΔQ > 0
///    estricto; pasa tras pasada hasta estabilizar.
/// 2. **Agregación** ([`GrafoPonderado::contraer`]): cada comunidad se
///    contrae en un supernodo (aristas internas → self-loops); el peso
///    total 2m se conserva, así que Q no cambia al cambiar de nivel — y el
///    nivel siguiente puede "ver" movimientos que a escala de nodo eran
///    bloqueados. Ésa es la gracia jerárquica de Louvain.
///
/// * `weight`: fuente de pesos del cap 22 (`Constant(1.0)` = no ponderado).
/// * `gamma`: resolución de la modularidad (1.0 = clásico; ver
///   [`modularidad`]).
/// * `max_pasadas`: tope de pasadas de fase local POR NIVEL (seguro contra
///   el ruido de f64 en ΔQ ≈ 0).
///
/// Resultado: partición final (nodos → comunidad, renumerada por menor
/// miembro), la JERARQUÍA de niveles (cada uno con su Q y nº de comunidades
/// — anidamiento garantizado, insumo del cap 51), Q final y stats.
///
/// Semántica del grafo: proyección simétrica con pesos (dirigidas sumadas
/// al par, paralelas acumuladas, self-loops ×2 — ver [`GrafoPonderado`]);
/// pesos negativos rechazados eager. Aislados: su propia comunidad. Q es
/// monótono no decreciente entre niveles (cada movimiento lo aumenta y la
/// contracción lo conserva) — testeado.
///
/// ```
/// use vol2_liradb::{louvain, Edge, MemoryStore, Node, WeightSource};
/// use vol2_liradb::GraphStore;
///
/// let mut s = MemoryStore::new();
/// for i in 0..8 { s.put_node(Node::new(i, "P")).unwrap(); }
/// // Dos K4 (0-3 y 4-7) unidos por un único puente 3-4.
/// for &(a, b) in &[(0,1),(0,2),(0,3),(1,2),(1,3),(2,3),
///                  (4,5),(4,6),(4,7),(5,6),(5,7),(6,7),(3,4)] {
///     s.put_edge(Edge::new(a * 16 + b, a, b, "KNOWS")).unwrap();
///     s.put_edge(Edge::new(b * 16 + a, b, a, "KNOWS")).unwrap();
/// }
///
/// let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
/// // El hito del capítulo: detectar los DOS grupos de la red.
/// assert_eq!(r.num_comunidades(), 2);
/// assert_eq!(r.comunidad(0), r.comunidad(2));
/// assert_ne!(r.comunidad(0), r.comunidad(5));
/// // Q de la partición perfecta de dos K4 con puente: 11/26.
/// assert!((r.modularidad - 11.0 / 26.0).abs() < 1e-12);
/// ```
pub fn louvain(
    store: &dyn GraphStore,
    weight: &WeightSource,
    gamma: f64,
    max_pasadas: u64,
) -> Result<LouvainResult, ComunidadesError> {
    validar_resolucion(gamma)?;
    validar_max_pasadas(max_pasadas)?;
    let g0 = GrafoPonderado::proyectar(store, weight)?;
    let n = g0.len();
    let mut stats = ComunidadesStats {
        edges_scanned: g0.edges_scanned,
        ..ComunidadesStats::default()
    };

    // mapeo[i] = posición densa (en el grafo del nivel actual) del nodo
    // ORIGINAL i. Nivel 0: identidad.
    let mut mapeo: Vec<usize> = (0..n).collect();
    let mut niveles: Vec<NivelLouvain> = Vec::new();
    let mut g = g0.clone();

    // Cota de terminación: en cada nivel la fase local arranca de
    // singletons, así que su PRIMER movimiento vacía una comunidad y el
    // grafo del siguiente nivel tiene estrictamente menos nodos. Los
    // niveles grabados son ≤ V (tope defensivo documentado).
    let tope_niveles = n + 1;
    loop {
        let (com, movimientos, pasadas) = g.fase_local(gamma, max_pasadas, &mut stats);
        if movimientos == 0 {
            break; // nada que agregar: la partición actual ya es la final
        }

        // Menor miembro ORIGINAL de cada nodo del nivel (para renumerar las
        // comunidades por su menor miembro).
        let mut min_orig = vec![usize::MAX; g.len()];
        for (i, &t) in mapeo.iter().enumerate() {
            if i < min_orig[t] {
                min_orig[t] = i;
            }
        }
        let densa = densificar(&com, &min_orig);
        let num_comunidades = densa.iter().copied().max().map_or(0, |m| m + 1);

        // Asignación de los nodos ORIGINALES en este nivel (composición).
        let asignacion: Vec<u64> = mapeo.iter().map(|&t| densa[t] as u64).collect();
        let q = g.modularidad_de(&densa, gamma);
        niveles.push(NivelLouvain {
            nivel: niveles.len(),
            num_comunidades,
            modularidad: q,
            pasadas,
            movimientos,
            asignacion,
        });

        mapeo = mapeo.iter().map(|&t| densa[t]).collect();
        g = g.contraer(&densa);
        if g.len() < 2 || niveles.len() >= tope_niveles {
            break; // 0-1 supernodos: no hay movimiento posible
        }
    }
    stats.niveles = niveles.len() as u64;

    // Partición final: la del último nivel, o singletons si nunca se movió
    // nadie (grafo vacío, sin aristas, o ya óptimo en singletons).
    let (asign_final, particion) = match niveles.last() {
        Some(nv) => (
            nv.asignacion
                .iter()
                .map(|&c| c as usize)
                .collect::<Vec<_>>(),
            Particion::nueva(g0.nodes.clone(), nv.asignacion.clone()),
        ),
        None => (
            (0..n).collect::<Vec<_>>(),
            Particion::nueva(g0.nodes.clone(), (0..n).map(|i| i as u64).collect()),
        ),
    };

    // Q final SIEMPRE sobre el grafo original (nunca del contraído): la
    // contracción lo conserva en exacto, pero el contrato del resultado es
    // "Q de esta partición en el store".
    let modularidad = g0.modularidad_de(&asign_final, gamma);

    Ok(LouvainResult {
        particion,
        niveles,
        modularidad,
        stats,
    })
}

// ══════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_comunidades {
    use super::*;
    use crate::cap07_modelo::{Edge, Node, Value};
    use crate::cap08_graph_store::MemoryStore;
    use crate::cap20_volcano::demo_graph;

    /// Tolerancia para comparar f64 contra soluciones a mano (fracciones
    /// exactas calculadas con la misma aritmética: sobra 1e-12).
    const EPS: f64 = 1e-12;

    /// ¿Está `a` a menos de `eps` de `b`?
    fn cerca(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    /// Store con nodos 0..n y las aristas NO dirigidas dadas (ambas
    /// direcciones, peso 1 — la forma en que el caller simetriza).
    fn no_dirigido(n: usize, aristas: &[(usize, usize)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for i in 0..n {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        let mut k = 0;
        for &(a, b) in aristas {
            s.put_edge(Edge::new(k, a, b, "E")).unwrap();
            s.put_edge(Edge::new(k + 1, b, a, "E")).unwrap();
            k += 2;
        }
        s
    }

    /// Store con nodos 0..n y aristas no dirigidas ponderadas (prop "w",
    /// misma semántica que WEIGHT relationship.w del cap 22).
    fn no_dirigido_ponderado(n: usize, aristas: &[(usize, usize, f64)]) -> MemoryStore {
        let mut s = MemoryStore::new();
        for i in 0..n {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        let mut k = 0;
        for &(a, b, w) in aristas {
            for (id, (x, y)) in [(k, (a, b)), (k + 1, (b, a))] {
                s.put_edge(Edge::new(id, x, y, "E").with_prop("w", Value::Float(w)))
                    .unwrap();
            }
            k += 2;
        }
        s
    }

    /// Dos tríos (K3) unidos por un puente: el grafo canónico de las
    /// cuentas a mano. Nodos 0-2 y 3-5, puente 1-4.
    fn dos_trios_con_puente() -> MemoryStore {
        no_dirigido(
            6,
            &[
                (0, 1),
                (0, 2),
                (1, 2),
                (3, 4),
                (3, 5),
                (4, 5),
                (1, 4), // puente
            ],
        )
    }

    /// El grafo canónico del LÍMITE DE RESOLUCIÓN (Fortunato-Barthélemy
    /// 2007): k tríos idénticos conectados en anillo por eslabones de
    /// peso 1. Con γ=1 la modularidad PREFIERE fundir tríos adyacentes en
    /// pares (aunque el ojo vea k tríos); con γ=2 los recupera.
    fn anillo_de_trios(k: usize) -> MemoryStore {
        let mut aristas = Vec::new();
        for t in 0..k {
            let base = 3 * t;
            for (a, b) in [(base, base + 1), (base + 1, base + 2), (base, base + 2)] {
                aristas.push((a, b));
            }
            // Eslabón del anillo: cierra la rueda.
            let u = base + 2;
            let v = 3 * ((t + 1) % k) + 1;
            aristas.push((u, v));
        }
        no_dirigido(3 * k, &aristas)
    }

    /// ¿Las dos particiones agrupan IGUAL (ignorando los ids de grupo)?
    fn mismas_particiones(a: &Particion, b: &Particion) -> bool {
        let mut ga = a.grupos();
        let mut gb = b.grupos();
        ga.sort_unstable();
        gb.sort_unstable();
        ga == gb
    }

    /// Partición ground-truth como Vec de grupos.
    fn particion_truth(grupos: &[Vec<NodeId>]) -> Particion {
        let mut nodes: Vec<NodeId> = grupos.iter().flatten().copied().collect();
        nodes.sort_unstable();
        let mut grupo = vec![0u64; nodes.len()];
        for (g, miembros) in grupos.iter().enumerate() {
            for &m in miembros {
                let i = nodes.binary_search(&m).unwrap();
                grupo[i] = g as u64;
            }
        }
        Particion::nueva(nodes, grupo)
    }

    // ─── Modularidad: cuentas a mano ───

    #[test]
    fn modularidad_particion_trivial_es_cero_y_perfecta_analitica() {
        let s = dos_trios_con_puente();
        let w = WeightSource::Constant(1.0);

        // Partición trivial (todo junto): In = 2m → Q = 1 − γ·1 = 0 exacto.
        let trivial: Vec<(NodeId, u64)> = (0..6).map(|i| (i, 0)).collect();
        assert_eq!(modularidad(&s, &trivial, &w, 1.0).unwrap(), 0.0);

        // Partición perfecta {0,1,2},{3,4,5}: a mano —
        //   2m = 14 (7 aristas × 2), In = 6 por trío, K = 7 por trío
        //   (el puente sube a 3 el grado de 1 y de 4).
        //   Q = 2·[6/14 − (7/14)²] = 12/14 − 1/2 = 5/14.
        let perfecta: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0), (2, 0), (3, 1), (4, 1), (5, 1)];
        assert!(cerca(
            modularidad(&s, &perfecta, &w, 1.0).unwrap(),
            5.0 / 14.0,
            EPS
        ));

        // Partición de singletons: Q = −Σ(k_i/2m)² = −(2·9 + 4·4)/196
        // (nodos 1 y 4 con grado 3, el resto 2) = −34/196 = −17/98.
        let singletons: Vec<(NodeId, u64)> = (0..6).map(|i| (i, i as u64)).collect();
        assert!(cerca(
            modularidad(&s, &singletons, &w, 1.0).unwrap(),
            -17.0 / 98.0,
            EPS
        ));
    }

    #[test]
    fn modularidad_gamma_resolucion_analitica() {
        let s = dos_trios_con_puente();
        let w = WeightSource::Constant(1.0);
        let perfecta: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0), (2, 0), (3, 1), (4, 1), (5, 1)];

        // γ=2: Q = 2·6/14 − 2·2·(7/14)² = 6/7 − 1 = −1/7.
        assert!(cerca(
            modularidad(&s, &perfecta, &w, 2.0).unwrap(),
            -1.0 / 7.0,
            EPS
        ));

        // γ grande penaliza la partición que el azar "explicaría" peor; la
        // trivial sigue exactamente 0 para cualquier γ (1 − γ·1·(2m/2m)²…
        // con todo junto In/2m = 1 y K/2m = 1: Q = 1 − γ).
        let trivial: Vec<(NodeId, u64)> = (0..6).map(|i| (i, 0)).collect();
        assert!(cerca(
            modularidad(&s, &trivial, &w, 2.0).unwrap(),
            -1.0,
            EPS
        ));

        // γ inválido: 0, negativo e infinito, señalados. (NaN se comprueba
        // aparte con matches! porque NaN != NaN bajo PartialEq.)
        for gamma in [0.0, -0.5, f64::INFINITY] {
            assert_eq!(
                modularidad(&s, &perfecta, &w, gamma),
                Err(ComunidadesError::InvalidResolution { value: gamma })
            );
        }
        assert!(matches!(
            modularidad(&s, &perfecta, &w, f64::NAN),
            Err(ComunidadesError::InvalidResolution { .. })
        ));
    }

    #[test]
    fn modularidad_self_loops_simetria_y_paralelas() {
        let w = WeightSource::Constant(1.0);

        // Self-loop contado DOBLE (convención A_ii = 2s): nodo 0 con
        // self-loop de peso 2 y arista 0-1 de peso 1.
        //   k = [5, 1] (2·2+1 y 1), 2m = 6.
        //   Juntos: In = A_00 + A_01 + A_10 = 4+1+1 = 6 → Q = 6/6 − 1 = 0
        //   (el modelo nulo lo explica TODO: es la partición trivial).
        //   Singletons: {0}: 4/6 − (5/6)² = −1/36; {1}: −(1/6)² = −1/36
        //   → Q = −1/18.
        let mut s = MemoryStore::new();
        for i in 0..2 {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        s.put_edge(Edge::new(0, 0, 0, "E").with_prop("w", Value::Float(2.0)))
            .unwrap();
        s.put_edge(Edge::new(1, 1, 0, "E").with_prop("w", Value::Float(1.0)))
            .unwrap();
        let wp = WeightSource::property("w");
        let juntos: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0)];
        assert_eq!(modularidad(&s, &juntos, &wp, 1.0).unwrap(), 0.0);
        let separados: Vec<(NodeId, u64)> = vec![(0, 0), (1, 1)];
        assert!(cerca(
            modularidad(&s, &separados, &wp, 1.0).unwrap(),
            -1.0 / 18.0,
            EPS
        ));

        // Cadena DIRIGIDA 0→1→2: la vista simétrica es el camino 0-1-2
        // (cada arista dirigida aporta 1 al par).
        //   k = [1,2,1], 2m = 4. {0,1}: In = 2, K = 3 →
        //   Q = 2/4 − (3/4)² − (1/4)² = −1/8.
        let mut s = MemoryStore::new();
        for i in 0..3 {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        s.put_edge(Edge::new(0, 0, 1, "E")).unwrap();
        s.put_edge(Edge::new(1, 1, 2, "E")).unwrap();
        let p: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0), (2, 1)];
        assert!(cerca(
            modularidad(&s, &p, &w, 1.0).unwrap(),
            -1.0 / 8.0,
            EPS
        ));

        // Paralelas ACUMULADAS: dos aristas 0→1 equivalen a una de peso 2
        // (k = [2,2], 2m = 4): juntos Q = 2·2/4 − 1 = 0; separados −(4+4)/16
        // = −1/2.
        let mut s = MemoryStore::new();
        for i in 0..2 {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        s.put_edge(Edge::new(0, 0, 1, "E")).unwrap();
        s.put_edge(Edge::new(1, 0, 1, "E")).unwrap();
        let juntos: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0)];
        assert_eq!(modularidad(&s, &juntos, &w, 1.0).unwrap(), 0.0);
        let separados: Vec<(NodeId, u64)> = vec![(0, 0), (1, 1)];
        assert!(cerca(
            modularidad(&s, &separados, &w, 1.0).unwrap(),
            -0.5,
            EPS
        ));
    }

    #[test]
    fn modularidad_escalar_pesos_no_cambia_q_y_nodos_ausentes() {
        // Escalar TODOS los pesos por una constante no cambia Q (es una
        // fracción de peso total) — invariante del modelo nulo.
        let base = no_dirigido_ponderado(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 5.0)]);
        let escalado = no_dirigido_ponderado(4, &[(0, 1, 10.0), (1, 2, 10.0), (2, 3, 50.0)]);
        let w = WeightSource::property("w");
        let p: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0), (2, 1), (3, 1)];
        let qa = modularidad(&base, &p, &w, 1.0).unwrap();
        let qb = modularidad(&escalado, &p, &w, 1.0).unwrap();
        assert!(cerca(qa, qb, 1e-9));

        // Nodos AUSENTES de la partición → singleton (documentado).
        // Estrella 0-1, 0-2: sólo el 0 en grupo 0 → {0},{1},{2}:
        //   k = [2,1,1], 2m = 4 → Q = −(4+1+1)/16 = −3/8.
        let s = no_dirigido(3, &[(0, 1), (0, 2)]);
        let w = WeightSource::Constant(1.0);
        let parcial: Vec<(NodeId, u64)> = vec![(0, 0)];
        assert!(cerca(
            modularidad(&s, &parcial, &w, 1.0).unwrap(),
            -3.0 / 8.0,
            EPS
        ));

        // Nodo de la partición que no existe: señalado.
        assert_eq!(
            modularidad(&s, &[(99, 0)], &w, 1.0),
            Err(ComunidadesError::UnknownNode(99))
        );
    }

    #[test]
    fn modularidad_pesos_invalidos_y_negativos() {
        let mut s = MemoryStore::new();
        for i in 0..2 {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        // Sin la prop "w" en ninguna arista.
        s.put_edge(Edge::new(0, 0, 1, "E")).unwrap();
        s.put_edge(Edge::new(1, 1, 0, "E")).unwrap();
        let p: Vec<(NodeId, u64)> = vec![(0, 0), (1, 0)];

        // Prop ausente → MissingWeight del cap 22 envuelto (señalando la
        // PRIMERA arista leída en la proyección).
        assert_eq!(
            modularidad(&s, &p, &WeightSource::property("w"), 1.0),
            Err(ComunidadesError::Weight(PathError::MissingWeight {
                edge: 0,
                prop: "w".into()
            }))
        );

        // Prop no numérica en la segunda arista (la primera es válida):
        // InvalidWeight envuelto, señalando SU arista.
        let mut s2 = MemoryStore::new();
        for i in 0..2 {
            s2.put_node(Node::new(i, "N")).unwrap();
        }
        s2.put_edge(Edge::new(0, 0, 1, "E").with_prop("w", Value::Float(1.0)))
            .unwrap();
        s2.put_edge(Edge::new(1, 1, 0, "E").with_prop("w", Value::String("mucho".into())))
            .unwrap();
        assert!(matches!(
            modularidad(&s2, &p, &WeightSource::property("w"), 1.0),
            Err(ComunidadesError::Weight(PathError::InvalidWeight {
                edge: 1,
                ..
            }))
        ));

        // Constante NaN → NonFiniteWeight envuelto.
        assert!(matches!(
            modularidad(&s, &p, &WeightSource::Constant(f64::NAN), 1.0),
            Err(ComunidadesError::Weight(PathError::NonFiniteWeight { .. }))
        ));
        // Peso NEGATIVO: rechazado eager con la arista señalada.
        assert_eq!(
            modularidad(&s, &p, &WeightSource::Constant(-1.0), 1.0),
            Err(ComunidadesError::NegativeWeight {
                edge: 0,
                weight: -1.0
            })
        );
    }

    // ─── Componentes conexas ───

    #[test]
    fn componentes_dos_pares_y_puente() {
        // Sin puente: dos componentes de tamaño 3.
        let s = no_dirigido(6, &[(0, 1), (0, 2), (3, 4), (3, 5)]);
        let r = componentes_conexas(&s).unwrap();
        assert_eq!(r.num_componentes(), 2);
        assert_eq!(
            r.componentes().iter().map(Vec::len).collect::<Vec<_>>(),
            vec![3, 3]
        );
        assert_eq!(r.componente(0), Some(0)); // renumerada por menor miembro
        assert_eq!(r.componente(3), Some(1));
        assert_eq!(r.componente(99), None);
        assert!(r.stats.edges_scanned > 0);

        // Con el puente 2-3: UNA sola componente.
        let s = no_dirigido(6, &[(0, 1), (0, 2), (3, 4), (3, 5), (2, 3)]);
        let r = componentes_conexas(&s).unwrap();
        assert_eq!(r.num_componentes(), 1);
        assert_eq!(r.componentes(), vec![vec![0, 1, 2, 3, 4, 5]]);
    }

    #[test]
    fn componentes_vacio_aislados_y_dirigidos() {
        // Vacío.
        let r = componentes_conexas(&MemoryStore::new()).unwrap();
        assert_eq!(r.num_componentes(), 0);
        assert!(r.particion.is_empty());

        // Tres aislados: tres componentes singleton.
        let s = no_dirigido(3, &[]);
        let r = componentes_conexas(&s).unwrap();
        assert_eq!(r.num_componentes(), 3);
        assert_eq!(r.componentes(), vec![vec![0], vec![1], vec![2]]);

        // Dirigidos 0→1 y 2→3: la vista es SIMÉTRICA → dos componentes.
        let mut s = MemoryStore::new();
        for i in 0..4 {
            s.put_node(Node::new(i, "N")).unwrap();
        }
        s.put_edge(Edge::new(0, 0, 1, "E")).unwrap();
        s.put_edge(Edge::new(1, 2, 3, "E")).unwrap();
        let r = componentes_conexas(&s).unwrap();
        assert_eq!(r.num_componentes(), 2);
        assert_eq!(r.componente(0), r.componente(1));
        assert_eq!(r.componente(2), r.componente(3));

        // Huecos tras delete_node: el id borrado no existe, el resto se
        // compacta.
        let mut s = no_dirigido(3, &[(0, 1), (1, 2)]);
        assert!(s.delete_node(1));
        let r = componentes_conexas(&s).unwrap();
        assert_eq!(r.num_componentes(), 2);
        assert_eq!(r.componente(1), None);
        assert_eq!(r.particion.len(), 2);
    }

    // ─── Label propagation ───

    #[test]
    fn lpa_separa_dos_trios_y_empates_deterministas() {
        // Dos tríos con puente: LPA converge a los tríos (la densidad manda
        // en los votos; la política de conservar la propia en los empates
        // frena el goteo por el puente en la segunda mitad de la pasada).
        let s = dos_trios_con_puente();
        let r = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        assert_eq!(r.num_comunidades(), 2);
        assert!(mismas_particiones(
            &r.particion,
            &particion_truth(&[vec![0, 1, 2], vec![3, 4, 5]])
        ));
        assert!(r.stats.pasadas >= 1);
        assert!(r.stats.movimientos > 0);

        // Un camino 0-1-2 se funde ENTERO: los votos arrastran en cascada
        // (0 adopta la etiqueta de 1, 2 la de ambos) — en estructuras sin
        // comunidad densa LPA converge a la componente entera. Documentar
        // el comportamiento, no imaginar particiones que la heurística no
        // tiene por qué encontrar.
        let s = no_dirigido(3, &[(0, 1), (1, 2)]);
        let r = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        assert_eq!(r.num_comunidades(), 1);

        // El LÍMITE documentado: con pesos uniformes el barrido asíncrono
        // GOTEA por los puentes cuando el primer grupo ya se formó y el
        // vecino del puente aún no reúne votos propios (aquí: dos K3 con
        // puente, evaluando primero la izquierda). Louvain sobre el MISMO
        // grafo sin pesos SÍ separa — la métrica guía contra la etiqueta
        // sin métrica.
        let s = no_dirigido(6, &[(0, 1), (0, 2), (1, 2), (3, 4), (3, 5), (4, 5), (2, 3)]);
        let lpa = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        assert_eq!(lpa.num_comunidades(), 1);
        let lou = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        assert_eq!(lou.num_comunidades(), 2);

        // Y romper los empates con pesos arregla LPA: puente flojo (0.5).
        let s = no_dirigido_ponderado(
            6,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (1, 2, 1.0),
                (3, 4, 1.0),
                (3, 5, 1.0),
                (4, 5, 1.0),
                (2, 3, 0.5),
            ],
        );
        let r = label_propagation(&s, &WeightSource::property("w"), 20).unwrap();
        assert!(mismas_particiones(
            &r.particion,
            &particion_truth(&[vec![0, 1, 2], vec![3, 4, 5]])
        ));

        // Determinismo: dos ejecuciones, mismo resultado exacto.
        let s = dos_trios_con_puente();
        let a = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        let b = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn lpa_aislados_convergencia_y_errores() {
        // Aislados: conservan su etiqueta (una comunidad cada uno).
        let s = no_dirigido(3, &[(0, 1)]);
        let r = label_propagation(&s, &WeightSource::Constant(1.0), 10).unwrap();
        assert_eq!(r.num_comunidades(), 2);
        assert_eq!(r.etiqueta(2), Some(1)); // el aislado, solo en su grupo

        // Convergencia: para antes del máximo (una pasada estable cierra).
        let s = no_dirigido(2, &[(0, 1)]);
        let r = label_propagation(&s, &WeightSource::Constant(1.0), 50).unwrap();
        assert!(r.stats.pasadas < 50);

        // Grafo vacío y parámetros inválidos.
        let r = label_propagation(&MemoryStore::new(), &WeightSource::Constant(1.0), 5).unwrap();
        assert_eq!(r.num_comunidades(), 0);
        assert_eq!(
            label_propagation(&s, &WeightSource::Constant(1.0), 0),
            Err(ComunidadesError::InvalidMaxPasadas { value: 0 })
        );
        // Los pesos estrictos del cap 22 aplican también aquí.
        assert!(matches!(
            label_propagation(&s, &WeightSource::property("w"), 5),
            Err(ComunidadesError::Weight(PathError::MissingWeight { .. }))
        ));
        assert!(matches!(
            label_propagation(&s, &WeightSource::Constant(-2.0), 5),
            Err(ComunidadesError::NegativeWeight { .. })
        ));
    }

    // ─── Louvain: el hito del capítulo ───

    #[test]
    fn louvain_separa_dos_cliques_unidas_por_puente() {
        // Dos K4 con un puente: el caso del libro. Louvain DEBE encontrar
        // los dos K4 (Q = 11/26 calculado a mano en el doctest).
        let s = no_dirigido(
            8,
            &[
                (0, 1),
                (0, 2),
                (0, 3),
                (1, 2),
                (1, 3),
                (2, 3), // K4 izquierdo
                (4, 5),
                (4, 6),
                (4, 7),
                (5, 6),
                (5, 7),
                (6, 7), // K4 derecho
                (3, 4), // puente
            ],
        );
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();

        assert_eq!(r.num_comunidades(), 2);
        assert!(mismas_particiones(
            &r.particion,
            &particion_truth(&[vec![0, 1, 2, 3], vec![4, 5, 6, 7]])
        ));
        assert!(cerca(r.modularidad, 11.0 / 26.0, EPS));

        // Q del resultado == modularidad() de la MISMA partición: la métrica
        // es el oráculo del algoritmo (y del test).
        let pares: Vec<(NodeId, u64)> = r.particion.entries();
        let q = modularidad(&s, &pares, &WeightSource::Constant(1.0), 1.0).unwrap();
        assert!(cerca(q, r.modularidad, 1e-9));

        // Cualquier partición razonable hace PEOR que la encontrada: la
        // trivial es 0 < 11/26.
        let trivial: Vec<(NodeId, u64)> = (0..8).map(|i| (i, 0)).collect();
        assert!(cerca(
            modularidad(&s, &trivial, &WeightSource::Constant(1.0), 1.0).unwrap(),
            0.0,
            EPS
        ));
    }

    #[test]
    fn louvain_determinismo_y_orden_de_insercion() {
        // Dos ejecuciones: resultado EXACTO igual (jerarquía incluida).
        let s = anillo_de_trios(12);
        let a = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        let b = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        assert_eq!(a, b);

        // El mismo grafo con las aristas insertadas en orden INVERSO: los
        // ids ordenados de la proyección absorben el orden de inserción.
        let mut s2 = MemoryStore::new();
        for i in 0..36 {
            s2.put_node(Node::new(i, "N")).unwrap();
        }
        let mut aristas = Vec::new();
        for t in 0..12 {
            let base = 3 * t;
            for (x, y) in [(base, base + 1), (base + 1, base + 2), (base, base + 2)] {
                aristas.push((x, y));
            }
            aristas.push((base + 2, 3 * ((t + 1) % 12) + 1));
        }
        let mut k = 0;
        for &(x, y) in aristas.iter().rev() {
            s2.put_edge(Edge::new(k, x, y, "E")).unwrap();
            s2.put_edge(Edge::new(k + 1, y, x, "E")).unwrap();
            k += 2;
        }
        let c = louvain(&s2, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        assert!(mismas_particiones(&a.particion, &c.particion));
        assert!(cerca(a.modularidad, c.modularidad, EPS));
    }

    #[test]
    fn louvain_vacio_aislados_y_self_loops() {
        // Vacío: sin nodos, sin comunidades, Q = 0.
        let r = louvain(&MemoryStore::new(), &WeightSource::Constant(1.0), 1.0, 10).unwrap();
        assert_eq!(r.num_comunidades(), 0);
        assert!(r.particion.is_empty());
        assert_eq!(r.modularidad, 0.0);
        assert!(r.niveles.is_empty());

        // Sin aristas (aislados): cada uno su comunidad (semilla singleton).
        let s = no_dirigido(3, &[]);
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 10).unwrap();
        assert_eq!(r.num_comunidades(), 3);
        assert_eq!(r.comunidad(0), Some(0));
        assert_eq!(r.modularidad, 0.0); // sin peso que repartir

        // Un nodo con self-loop: su propia comunidad; con 2m = 2s el modelo
        // nulo explica TODO el peso → Q = 1·(2s/2s) − (2s/2m)² = 0.
        let mut s = MemoryStore::new();
        s.put_node(Node::new(0, "N")).unwrap();
        s.put_edge(Edge::new(0, 0, 0, "E")).unwrap();
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 10).unwrap();
        assert_eq!(r.num_comunidades(), 1);
        assert_eq!(r.modularidad, 0.0);

        // Dos pares unidos + dos aislados: los pares se fusionan (Q > 0
        // negativo… aquí la fusión es el óptimo local), los aislados
        // permanecen solos.
        let s = no_dirigido(6, &[(0, 1), (2, 3)]);
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 10).unwrap();
        assert_eq!(r.comunidad(0), r.comunidad(1));
        assert_eq!(r.comunidad(2), r.comunidad(3));
        assert_ne!(r.comunidad(0), r.comunidad(2));
        assert_ne!(r.comunidad(0), r.comunidad(4)); // aislado 4
        assert_ne!(r.comunidad(0), r.comunidad(5)); // aislado 5
    }

    #[test]
    fn louvain_demo_graph_dani_solo_y_empate_de_optimos() {
        // KNOWS: 0→1→2→0 y self-loop 3→3; LIVES_IN: 0→4, 1→5. Proyectado
        // simétrico: el trío {0,1,2} (peso 1 por par), 0-4 y 1-5 (peso 1),
        // y Dani aislado con su self-loop (A_33 = 2).
        //   k = [3,3,2,2,1,1], 2m = 12.
        // Hay DOS óptimos con la MISMA Q = 5/18 ≈ 0.2778:
        //   {0,2,4},{1,5},{3}  y  {0,1,2,4,5},{3}
        // (el dq de juntar {0,4} con {1,5} en el nivel 0 es EXACTAMENTE 0:
        // 24/144−16/144 por lado… la cuenta completa en el capítulo). El
        // greedy determinista toma UNO de ellos; lo que NO depende del
        // camino: Dani solo (su self-loop no une con nadie) y Q = 5/18.
        let s = demo_graph();
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();

        assert!(cerca(r.modularidad, 5.0 / 18.0, 1e-9));
        assert_ne!(r.comunidad(3), r.comunidad(0)); // Dani, solo
        // La ciudad vive en la órbita de quien la apunta.
        assert_eq!(r.comunidad(4), r.comunidad(0));
        assert_eq!(r.comunidad(5), r.comunidad(1));
        // Coherencia: Q del resultado == modularidad de la partición.
        let pares: Vec<(NodeId, u64)> = r.particion.entries();
        let q = modularidad(&s, &pares, &WeightSource::Constant(1.0), 1.0).unwrap();
        assert!(cerca(q, r.modularidad, 1e-9));

        // Lección de las aristas dirigidas: PROYECTAR sólo KNOWS simetrizado
        // a mano dejaría el trío + Dani; LIVES_IN arrastra las ciudades al
        // trío — las comunidades dependen de QUÉ aristas proyectas.
    }

    #[test]
    fn louvain_jerarquia_monotonia_anidamiento_y_contraccion() {
        // Anillo de 12 tríos, γ=1: el nivel 0 encuentra los 12 tríos y el
        // nivel 1 los FUNDE en 6 pares (Q sube de 2/3 a 17/24) — la
        // agregación desbloquea el movimiento que a escala de nodo no había.
        let s = anillo_de_trios(12);
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();

        assert!(
            r.niveles.len() >= 2,
            "jerarquía corta: {:?}",
            r.niveles.len()
        );
        assert_eq!(r.niveles[0].num_comunidades, 12);
        assert_eq!(r.niveles[1].num_comunidades, 6);

        // Q por nivel: 12 tríos → 2/3 exacto; 6 pares → 17/24 exacto.
        assert!(cerca(r.niveles[0].modularidad, 2.0 / 3.0, 1e-9));
        assert!(cerca(r.niveles[1].modularidad, 17.0 / 24.0, 1e-9));

        // MONOTONÍA: Q no decrece entre niveles (cada movimiento lo sube y
        // la contracción lo conserva)…
        for w in r.niveles.windows(2) {
            assert!(
                w[1].modularidad >= w[0].modularidad - 1e-9,
                "Q bajó entre niveles: {:?}",
                r.niveles.iter().map(|n| n.modularidad).collect::<Vec<_>>()
            );
        }
        // …el número de comunidades no crece…
        for w in r.niveles.windows(2) {
            assert!(w[1].num_comunidades <= w[0].num_comunidades);
        }
        // …y Q final == Q del último nivel == modularidad() recomputada.
        assert!(cerca(
            r.modularidad,
            r.niveles.last().unwrap().modularidad,
            1e-9
        ));
        let pares: Vec<(NodeId, u64)> = r.particion.entries();
        assert!(cerca(
            modularidad(&s, &pares, &WeightSource::Constant(1.0), 1.0).unwrap(),
            r.modularidad,
            1e-9
        ));

        // INVARIANTE DE CONTRACCIÓN: la Q de cada nivel, calculada por el
        // grafo CONTRAÍDO, coincide con modularidad() sobre el ORIGINAL con
        // la asignación compuesta de ese nivel.
        for nv in &r.niveles {
            let pares: Vec<(NodeId, u64)> = nv
                .asignacion
                .iter()
                .enumerate()
                .map(|(i, &c)| (i, c))
                .collect();
            assert!(cerca(
                modularidad(&s, &pares, &WeightSource::Constant(1.0), 1.0).unwrap(),
                nv.modularidad,
                1e-9
            ));
        }

        // ANIDAMIENTO (dirección correcta): los niveles bajos son FINOS —
        // misma comunidad en el nivel ℓ ⇒ misma en el nivel ℓ+1 (cada
        // comunidad superior es unión exacta de las inferiores; al revés
        // NO: fundir es fusionar dos comunidades del nivel bajo).
        for w in r.niveles.windows(2) {
            for i in 0..36 {
                for j in 0..36 {
                    if w[0].asignacion[i] == w[0].asignacion[j] {
                        assert_eq!(
                            w[1].asignacion[i], w[1].asignacion[j],
                            "el anidamiento se rompe en ({i},{j})"
                        );
                    }
                }
            }
        }

        // particion_en(): la vista de dendrograma para el cap 51.
        let p0 = r.particion_en(0).unwrap();
        assert_eq!(p0.num_grupos(), 12);
        assert!(r.particion_en(99).is_none());
    }

    #[test]
    fn louvain_limite_de_resolucion_gamma() {
        // EL límite de resolución (Fortunato-Barthélemy 2007), demostrado:
        // el anillo de 12 tríos es INVISIBLE para γ=1 (la modularidad
        // prefiere 6 pares: 17/24 > 2/3) pero γ=2 lo recupera.
        let s = anillo_de_trios(12);
        let wc = WeightSource::Constant(1.0);

        let rapido = louvain(&s, &wc, 1.0, 30).unwrap();
        assert_eq!(rapido.num_comunidades(), 6); // pares adyacentes fundidos

        let fino = louvain(&s, &wc, 2.0, 30).unwrap();
        assert_eq!(fino.num_comunidades(), 12);
        // Exactamente los tríos: comparación contra ground truth.
        let mut truth = Vec::new();
        for t in 0..12 {
            truth.push((3 * t..3 * t + 3).collect::<Vec<NodeId>>());
        }
        assert!(mismas_particiones(
            &fino.particion,
            &particion_truth(&truth)
        ));
        // Y con γ=2 la Q analítica de los 12 tríos es 7/12.
        assert!(cerca(fino.modularidad, 7.0 / 12.0, 1e-9));

        // La métrica guía lo explica: Q_γ(tríos) − Q_γ(pares) cambia de
        // signo entre γ=1 y γ=2 (los pares valen 17/24 y 13/24).
        let mut trios = Vec::new();
        for t in 0..12 {
            for &n in &(3 * t..3 * t + 3).collect::<Vec<_>>() {
                trios.push((n, t as u64));
            }
        }
        let mut pares = Vec::new();
        for t in 0..6 {
            for &n in &(6 * t..6 * t + 6).collect::<Vec<_>>() {
                pares.push((n, t as u64));
            }
        }
        assert!(cerca(
            modularidad(&s, &trios, &wc, 1.0).unwrap(),
            2.0 / 3.0,
            1e-9
        ));
        assert!(cerca(
            modularidad(&s, &pares, &wc, 1.0).unwrap(),
            17.0 / 24.0,
            1e-9
        ));
        assert!(cerca(
            modularidad(&s, &trios, &wc, 2.0).unwrap(),
            7.0 / 12.0,
            1e-9
        ));
        assert!(cerca(
            modularidad(&s, &pares, &wc, 2.0).unwrap(),
            13.0 / 24.0,
            1e-9
        ));
    }

    #[test]
    fn louvain_recupera_ground_truth_sintetico() {
        // Tres grupos sintéticos generables: anillos de 10 nodos con
        // cuerdas (grado interno 3) y DOS eslabones entre grupos (grado
        // externo ≤ 1 por nodo). Determinista, sin azar.
        let mut aristas = Vec::new();
        for c in 0..3 {
            let base = 10 * c;
            for i in 0..10 {
                aristas.push((base + i, base + (i + 1) % 10)); // anillo
                aristas.push((base + i, base + (i + 5) % 10)); // cuerda
            }
        }
        aristas.push((4, 15)); // dos eslabones entre grupos
        aristas.push((24, 7));
        let s = no_dirigido(30, &aristas);
        let wc = WeightSource::Constant(1.0);

        let truth = particion_truth(&[(0..10).collect(), (10..20).collect(), (20..30).collect()]);
        let r = louvain(&s, &wc, 1.0, 30).unwrap();

        // Recupera EXACTAMENTE los tres grupos (comparación como conjuntos,
        // ignorando los ids de comunidad).
        assert_eq!(r.num_comunidades(), 3);
        assert!(
            mismas_particiones(&r.particion, &truth),
            "particion: {:?}",
            r.comunidades()
        );

        // Y no le gana nadie: Q encontrada ≥ Q del ground truth (el greedy
        // no puede hacer peor que la verdad aquí — y de hecho iguala).
        let q_truth = modularidad(&s, &truth.entries(), &wc, 1.0).unwrap();
        assert!(r.modularidad >= q_truth - 1e-9);
        assert!(cerca(r.modularidad, q_truth, 1e-9));

        // Los componentes también son 1 (los eslabones conectan todo): dos
        // nociones de "grupo" distintas — alcanzabilidad vs densidad.
        assert_eq!(componentes_conexas(&s).unwrap().num_componentes(), 1);
    }

    #[test]
    fn louvain_los_pesos_cambian_la_particion() {
        // Dos tríos con un puente cuya fuerza vive en una PROPIEDAD. Flojo
        // (peso 1): los tríos ganan y el puente queda de frontera (Q =
        // 5/14). Brutal (peso 100): el puente ROMPE los tríos — la mejor
        // partición se lleva la pareja del puente ({1,4}, In = 200) y deja
        // los restos como satélites ({0,2} y {3,5}). Q es INVARIANTE a la
        // escala (es una fracción de 2m), así que no es "más peso = más
        // fusión": es que el puente deja de ser explicable por el azar y
        // la estructura se REORGANIZA alrededor de él.
        let flojo = no_dirigido_ponderado(
            6,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (1, 2, 1.0),
                (3, 4, 1.0),
                (3, 5, 1.0),
                (4, 5, 1.0),
                (1, 4, 1.0),
            ],
        );
        let fuerte = no_dirigido_ponderado(
            6,
            &[
                (0, 1, 1.0),
                (0, 2, 1.0),
                (1, 2, 1.0),
                (3, 4, 1.0),
                (3, 5, 1.0),
                (4, 5, 1.0),
                (1, 4, 100.0),
            ],
        );
        let w = WeightSource::property("w");

        let r_flojo = louvain(&flojo, &w, 1.0, 30).unwrap();
        assert_eq!(r_flojo.num_comunidades(), 2);
        assert!(mismas_particiones(
            &r_flojo.particion,
            &particion_truth(&[vec![0, 1, 2], vec![3, 4, 5]])
        ));
        assert!(cerca(r_flojo.modularidad, 5.0 / 14.0, EPS));

        let r_fuerte = louvain(&fuerte, &w, 1.0, 30).unwrap();
        assert_eq!(r_fuerte.num_comunidades(), 3);
        assert!(mismas_particiones(
            &r_fuerte.particion,
            &particion_truth(&[vec![0, 2], vec![1, 4], vec![3, 5]])
        ));
        // Q de esa partición, a mano: [2/212 − (4/212)²]·2 +
        // [200/212 − (204/212)²] = 100/2809.
        assert!(cerca(r_fuerte.modularidad, 100.0 / 2809.0, EPS));
        // …y MEJOR que fundirlo todo (la trivial vale exactamente 0):
        let trivial: Vec<(NodeId, u64)> = (0..6).map(|i| (i, 0)).collect();
        assert_eq!(modularidad(&fuerte, &trivial, &w, 1.0).unwrap(), 0.0);
        assert!(r_fuerte.modularidad > 0.0);
    }

    #[test]
    fn louvain_multigrafo_paralelas_equivalen_a_peso_sumado() {
        // Tres aristas paralelas de peso 1 == una arista de peso 3: la
        // proyección ACUMULA paralelas, mismo grafo, mismo resultado.
        let mut paralelo = MemoryStore::new();
        for i in 0..4 {
            paralelo.put_node(Node::new(i, "N")).unwrap();
        }
        let mut k = 0;
        for &(a, b) in &[(0, 1), (0, 1), (0, 1), (2, 3), (2, 3), (2, 3), (1, 2)] {
            paralelo.put_edge(Edge::new(k, a, b, "E")).unwrap();
            paralelo.put_edge(Edge::new(k + 1, b, a, "E")).unwrap();
            k += 2;
        }
        let sumado = no_dirigido_ponderado(4, &[(0, 1, 3.0), (2, 3, 3.0), (1, 2, 1.0)]);
        let w = WeightSource::property("w");

        let a = louvain(&paralelo, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        let b = louvain(&sumado, &w, 1.0, 30).unwrap();
        assert!(mismas_particiones(&a.particion, &b.particion));
        assert!(cerca(a.modularidad, b.modularidad, 1e-9));
    }

    #[test]
    fn louvain_parametros_invalidos_y_max_pasadas() {
        let s = dos_trios_con_puente();
        let wc = WeightSource::Constant(1.0);

        // γ y max_pasadas validados por todos los algoritmos.
        for gamma in [0.0, -1.0] {
            assert_eq!(
                louvain(&s, &wc, gamma, 10),
                Err(ComunidadesError::InvalidResolution { value: gamma })
            );
        }
        assert!(matches!(
            louvain(&s, &wc, f64::NAN, 10),
            Err(ComunidadesError::InvalidResolution { .. })
        ));
        assert_eq!(
            louvain(&s, &wc, 1.0, 0),
            Err(ComunidadesError::InvalidMaxPasadas { value: 0 })
        );

        // max_pasadas=1: recorte del trabajo POR NIVEL — no de la calidad
        // final: lo que una pasada deja a medias en el nivel 0, la
        // AGREGACIÓN lo repara en el nivel 1 (la jerarquía desbloquea
        // movimientos que a escala de nodo no había). El resultado sigue
        // siendo una partición coherente con su Q — la BD responde con lo
        // que alcanzó a explorar, nunca con números incoherentes.
        let completo = louvain(&s, &wc, 1.0, 30).unwrap();
        let una_pasada = louvain(&s, &wc, 1.0, 1).unwrap();
        assert!(una_pasada.stats.pasadas <= completo.stats.pasadas);
        let pares: Vec<(NodeId, u64)> = una_pasada.particion.entries();
        assert!(cerca(
            modularidad(&s, &pares, &wc, 1.0).unwrap(),
            una_pasada.modularidad,
            1e-9
        ));
        // Nunca por encima del correr completo (misma monotonía de Q).
        assert!(una_pasada.modularidad <= completo.modularidad + 1e-9);
        assert!(cerca(una_pasada.modularidad, completo.modularidad, 1e-9));
    }

    #[test]
    fn louvain_stats_coherentes() {
        let s = anillo_de_trios(12);
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        // La proyección leyó las 4 aristas dirigidas × 48 pares = 96.
        assert_eq!(r.stats.edges_scanned, 96);
        // Niveles grabados == niveles.len(); pasadas ≥ movimientos ≥ niveles.
        assert_eq!(r.stats.niveles as usize, r.niveles.len());
        assert!(r.stats.pasadas >= r.niveles.len() as u64);
        assert!(r.stats.movimientos >= r.niveles.len() as u64);
        // La suma por nivel cuadra con los totales.
        let movs: u64 = r.niveles.iter().map(|n| n.movimientos).sum();
        assert_eq!(movs, r.stats.movimientos);
    }

    // ─── Errores, accessores y Display ───

    #[test]
    fn errores_display_y_std_error() {
        let e = ComunidadesError::InvalidResolution { value: 1.5 };
        assert!(e.to_string().contains("1.5"));
        assert!(e.to_string().contains("> 0"));
        let _: &dyn std::error::Error = &e;

        let e = ComunidadesError::InvalidMaxPasadas { value: 0 };
        assert!(e.to_string().contains("0"));
        let e = ComunidadesError::NegativeWeight {
            edge: 7,
            weight: -2.0,
        };
        assert!(e.to_string().contains("7"));
        assert!(e.to_string().contains("negative"));
        let e = ComunidadesError::Weight(PathError::MissingWeight {
            edge: 3,
            prop: "w".into(),
        });
        assert!(e.to_string().contains("w"));
        // From<PathError> funciona (costura con el cap 22).
        let e: ComunidadesError = PathError::NonFiniteWeight {
            edge: 1,
            weight: f64::NAN,
        }
        .into();
        assert!(matches!(e, ComunidadesError::Weight(_)));
        let e = ComunidadesError::UnknownNode(42);
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn particion_accessores_y_display() {
        let s = dos_trios_con_puente();
        let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
        let p = &r.particion;

        assert_eq!(p.len(), 6);
        assert!(!p.is_empty());
        assert_eq!(p.grupo(99), None);
        assert_eq!(p.num_grupos(), 2);

        // entries en orden de id; grupos con miembros ascendentes.
        let e = p.entries();
        assert_eq!(e[0].0, 0);
        for g in p.grupos() {
            assert!(g.windows(2).all(|w| w[0] < w[1]));
        }
        assert_eq!(p.tamanos(), vec![3, 3]);

        // Display tipo tabla y de los resultados.
        assert!(format!("{}", p).starts_with("n0="));
        assert!(format!("{}", r).starts_with("Louvain(niveles="));
        assert!(format!("{}", r).contains("Q="));

        let c = componentes_conexas(&s).unwrap();
        assert!(format!("{}", c).contains("componentes=1"));

        let l = label_propagation(&s, &WeightSource::Constant(1.0), 20).unwrap();
        assert!(format!("{}", l).contains("pasadas="));
        assert_eq!(l.comunidades().len(), 2);
    }
}
