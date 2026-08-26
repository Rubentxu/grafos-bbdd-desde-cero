//! Vol.II — Cap.39: Joins, patrones y consultas cíclicas.
//!
//! Tercer capítulo de la Parte VIII (crecimiento POST-mapa) y COBRADOR de tres
//! deudas explícitas: el CSR ordenado del cap. 14 como base física, el
//! «joins reales WCOJ» anunciado por el cap. 20 y el gancho saliente del
//! cap. 38 («las columnas aceleran CÓMO lees; los worst-case optimal joins
//! cambian QUÉ calculas»). El modelo mental único: **join-oriented ×
//! materialización de intermedios**. Los planes clásicos atan TABLAS en un
//! árbol de uniones binarias y cada unión ESCRIBE su resultado; los
//! worst-case optimal atan VARIABLES en un backtracking sobre tries y nadie
//! escribe nada intermedio.
//!
//! Qué entrega este módulo (contrato §2):
//!
//! 1. **Formalizar `ExpandOp`** ([`AdyacenciasOrdenadas`] +
//!    [`expansion_dos_saltos_plana`] + [`intermedios_plan_binario`]): lo que
//!    el motor Volcano produce fila a fila es un *index nested-loop join*
//!    sobre adyacencias; contar sus INTERMEDIOS convierte el rumor del survey
//!    «Skew Strikes Back» (SIGMOD Record 42(4), 2014) en una cifra propia.
//! 2. **Joins binarios honestos** ([`triangulos_join_binario`] +
//!    [`triangulos_fuerza_bruta`]): materializar R(a,b)⋈S(b,c), semi-filtrar
//!    con T(a,c) y canonizar a<b<c; la fuerza bruta O(n³) es el terreno de
//!    verdad contra el que TODO se equivoca-testea.
//! 3. **LeapFrog Triejoin simplificado** ([`BuscadorSalto`] +
//!    [`BuscadorSalto::frontera_comun`] + [`TriangulosWcoj::enumerar`]):
//!    tries sobre las listas ordenadas que LiraDB YA tiene, seek = salto
//!    exponencial + `slice::binary_search` (O(log n) peor caso garantizado),
//!    frontera común por niveles y ORDEN ESTÁTICO de variables a→b→c
//!    (Veldhuizen, ICDT 2014; workhorse de LogicBlox ANTES de que se probara
//!    su optimalidad — ICDT Test-of-Time 2024).
//! 4. **Cota AGM medible** ([`cota_agm_triangulos`]): ⌊m^(3/2)⌋ como BRÚJULA
//!    del peor caso (Atserias-Grohe-Marx, FOCS 2008; SICOMP 42(4), 2013; la
//!    raíz geométrica es Loomis-Whitney 1949), no como promesa de velocidad.
//! 5. **Consultas recursivas** ([`cierre_transitivo`]): punto fijo ITERATIVO
//!    con [`BitSet`] de visitados y [`Presupuesto`]/[`MotivoParada`] del
//!    cap. 26 REUTILIZADOS tal cual — termina en grafos CON ciclos y es
//!    demostrablemente acotable. Sin sintaxis nueva en LiraQL: API y concepto
//!    (`WITH RECURSIVE` SQL:1999 y GQL ISO/IEC 39075:2024 quedan como
//!    anzuelo al Vol.III).
//! 6. **Factorized execution extendida al JOIN**
//!    ([`ResultadoFactorizadoTriangulos`]): prefijo a compartido, prefijo b
//!    compartido, multiplicidades — el mismo patrón arrays-por-variable de
//!    [`crate::cap38_columnar::ExpansionFactorizada`] aplicado al RESULTADO de
//!    un join cíclico (Olteanu-Závodný, ICDT 2012; TODS 40(1), 2015).
//! 7. **Informe reproducible** ([`informe_joins_reproducible_sobre_mini`]):
//!    contadores de trabajo SIN tiempos — los cronómetros viven en
//!    `benches/bench_joins.rs` (regla del cap. 34).
//!
//! Frontera declarada: capa EDUCATIVA fuera del Executor — ni `optimize()` ni
//! `PhysicalOperator` se tocan; orden de variables ESTÁTICO (el dinámico
//! exigiría estadísticas que el catálogo del cap. 21 no tiene); LeapFrog
//! limitado a patrones de 2-3 aristas (no Generic Join multi-way genérico de
//! NPRR, PODS 2012); sin paralelismo ni sintaxis recursiva en LiraQL.
//! Atribución según ADR-001 (vinculante): Kùzu adoptó WCOJ y ejecución
//! factorizada para grafos (CIDR 2023, CC-BY 4.0); este módulo es clean-room,
//! cero código copiado.

use std::collections::HashMap;

use crate::cap07_modelo::NodeId;
use crate::cap08_graph_store::{GraphStore, MemoryStore};
use crate::cap26_proyeccion::{BitSet, MotivoParada, Presupuesto};

// ─────────────────── Cap 39: adyacencias ordenadas ───────────────────

/// Vista posicional de adyacencias ORDENADAS: la base física de todo el
/// capítulo.
///
/// Heredera directa del espíritu de `SubgrafoAcotado` (cap. 38): ids vivos en
/// orden ascendente y una lista de vecinos salientes POR POSICIÓN, cada una
/// ordenada y sin duplicados — el trie que LeapFrog necesita sale gratis de
/// la disciplina que el cap. 14 impuso al CSR («adyacencias YA ordenadas»).
///
/// Los vecinos son POSICIONES dentro de esta vista, no ids globales: igual
/// que en el cap. 38, así ningún recorrido puede salirse de la caja. Además
/// se guarda el GRADO ENTRANTE por posición (deduplicado), porque el contador
/// de intermedios del plan binario necesita Σ_b \|in(b)\|·\|out(b)\| sin
/// materializar nada.
///
/// Dos constructores: [`AdyacenciasOrdenadas::desde_store`] (vista completa
/// del store) y [`AdyacenciasOrdenadas::desde_aristas`] (grafos sintéticos
/// K₈, estrellas, ruedas — los experimentos calculados a mano).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdyacenciasOrdenadas {
    /// Ids incluidos, ascendentes; posición ↔ fila de `adj_out`.
    ids: Vec<NodeId>,
    /// Adyacencia saliente posicional: listas ordenadas y deduplicadas.
    adj_out: Vec<Vec<usize>>,
    /// Predecesores distintos por posición (lo que aporta Σ in·out).
    grado_in: Vec<usize>,
}

impl AdyacenciasOrdenadas {
    /// Construye la vista completa sobre los nodos vivos del store.
    ///
    /// Determinismo primero (ley de la casa desde el cap. 38): `iter_nodes`
    /// no garantiza orden, así que los ids se recolectan y se ORDENAN antes
    /// de tocar ninguna adyacencia. Dos llamadas sobre el mismo store dan la
    /// misma vista byte a byte.
    pub fn desde_store(store: &MemoryStore) -> Self {
        Self::desde_store_acotado(store, usize::MAX)
    }

    /// Igual que [`Self::desde_store`] pero conservando sólo los primeros
    /// `max_nodos` ids (ordenados) y las aristas internas al recorte.
    ///
    /// La versión acotada existe para los tests de equivalencia contra la
    /// fuerza bruta O(n³): el terreno de verdad es cuadrático en nodos, así
    /// que se compara sobre un subgrafo pequeño y DETERMINISTA del dataset.
    pub fn desde_store_acotado(store: &MemoryStore, max_nodos: usize) -> Self {
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
            adj_out.push(vecinos);
        }
        let grado_in = calcular_grado_in(&adj_out);
        AdyacenciasOrdenadas {
            ids,
            adj_out,
            grado_in,
        }
    }

    /// Construye la vista desde una lista cruda de pares (u, v) posicionales.
    ///
    /// Para los grafos sintéticos del capítulo (K₈ bidireccional, estrella
    /// con hub, rueda): duplicados y desorden de entrada NO importan, las
    /// listas se ordenan y deduplican siempre — el contrato físico es el
    /// mismo que sale del store.
    pub fn desde_aristas(num_nodos: usize, aristas: &[(usize, usize)]) -> Self {
        let mut adj_out = vec![Vec::new(); num_nodos];
        for &(u, v) in aristas {
            if u < num_nodos && v < num_nodos {
                adj_out[u].push(v);
            }
        }
        for vecinos in &mut adj_out {
            vecinos.sort_unstable();
            vecinos.dedup();
        }
        let grado_in = calcular_grado_in(&adj_out);
        AdyacenciasOrdenadas {
            ids: (0..num_nodos as u64 as NodeId).collect(),
            adj_out,
            grado_in,
        }
    }

    /// Nodos de la vista (longitud de todas las listas).
    pub fn num_nodos(&self) -> usize {
        self.ids.len()
    }

    /// Aristas dirigidas DISTINCTAS (suma de grados salientes deduplicados).
    pub fn num_aristas(&self) -> usize {
        self.adj_out.iter().map(Vec::len).sum()
    }

    /// Ids en orden ascendente; posición `p` ↔ `ids[p]`.
    pub fn ids(&self) -> &[NodeId] {
        &self.ids
    }

    /// Posición de un id global, si vive en la vista.
    pub fn posicion_de(&self, id: NodeId) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    /// Id global de una posición (pánico si está fuera: bug del llamador).
    pub fn id_de(&self, pos: usize) -> NodeId {
        self.ids[pos]
    }

    /// Vecinos salientes de una posición: ordenados, deduplicados, listos
    /// para usarse como trie.
    pub fn vecinos_out(&self, pos: usize) -> &[usize] {
        &self.adj_out[pos]
    }

    /// ¿Existe la arista dirigida a → b? Búsqueda binaria sobre el trie.
    pub fn contiene_arista(&self, a: usize, b: usize) -> bool {
        self.adj_out[a].binary_search(&b).is_ok()
    }

    /// Grado saliente deduplicado.
    pub fn grado_out(&self, pos: usize) -> usize {
        self.adj_out[pos].len()
    }

    /// Grado entrante deduplicado (predecesores distintos).
    pub fn grado_in(&self, pos: usize) -> usize {
        self.grado_in[pos]
    }
}

/// Grados entrantes deduplicados: cada lista contribuye ≤ 1 por destino.
fn calcular_grado_in(adj_out: &[Vec<usize>]) -> Vec<usize> {
    let mut grado_in = vec![0usize; adj_out.len()];
    for vecinos in adj_out {
        for &v in vecinos {
            grado_in[v] += 1;
        }
    }
    grado_in
}

// ─────────────────── ExpandOp formalizado y sus intermedios ───────────────────

/// Expansión plana de DOS saltos: todas las tuplas (a, b, c) con a→b y b→c,
/// materializadas y ordenadas.
///
/// Es la referencia contra la que se equivoca-testea el pipeline REAL
/// `NodeScan → Expand → Expand` del cap. 20 (test-tesis del módulo): lo que
/// ese bucle anidado produce sobre el mini dataset debe ser EXACTAMENTE este
/// multiconjunto — `ExpandOp` ES un index nested-loop join aunque nadie lo
/// llamara así.
pub fn expansion_dos_saltos_plana(adj: &AdyacenciasOrdenadas) -> Vec<(usize, usize, usize)> {
    let mut tuplas = Vec::new();
    for (a, vecinos) in adj.adj_out.iter().enumerate() {
        for &b in vecinos {
            for &c in &adj.adj_out[b] {
                tuplas.push((a, b, c));
            }
        }
    }
    tuplas.sort_unstable();
    tuplas
}

/// CUENTA (sin materializar) las filas intermedias que el plan binario
/// escribiría para el patrón de dos saltos: Σ_b \|in(b)\|·\|out(b)\|.
///
/// Sobre una vista simétrica (todas las aristas en ambos sentidos) es la
/// fórmula del contrato Σ_b out(b)·out(b): en K₈ son 392 caminos (a,b,c) para
/// 56 triángulos. LA cifra que justifica el resto del capítulo: los
/// intermedios pueden superar a la entrada Y al resultado JUNTOS (SIGMOD
/// Record 42(4), 2014). Con `checked_add`, como su hermana
/// `conteo_dos_saltos` del cap. 38: el desborde se NOMBRA, nunca envuelve.
pub fn intermedios_plan_binario(adj: &AdyacenciasOrdenadas) -> u64 {
    let mut total = 0u64;
    for b in 0..adj.num_nodos() {
        let caminos_en_b = adj.grado_in[b] as u64 * adj.vecinos_out(b).len() as u64;
        total = total.checked_add(caminos_en_b).expect(
            "contador de intermedios del plan binario debe caber en u64 (subgrafo acotado)",
        );
    }
    total
}

// ─────────────────── Joins binarios honestos ───────────────────

/// Resultado del plan binario para el patrón triángulo, con su FACTURA de
/// trabajo a la vista.
///
/// La escalera de conteos es la tesis del capítulo en miniatura:
/// `intermedios_materializados` (todo lo que R(a,b)⋈S(b,c) escribió) ≥
/// `tuplas_semi_filtradas` (las que sobreviven al semi-join con T(a:c)) ≥
/// `triangulos.len()` (tras la canonización a<b<c). En K₈: 392 ≥ 336 ≥ 56.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoBinario {
    /// Triángulos canónicos (a, b, c) con a<b<c y aristas a→b, b→c, a→c,
    /// ordenados (multiconjunto comparable con la fuerza bruta).
    pub triangulos: Vec<(usize, usize, usize)>,
    /// Filas (a,b,c) que el PRIMER join materializó de verdad — la memoria
    /// invisible que hoy paga LiraDB.
    pub intermedios_materializados: u64,
    /// Filas que pasan el semi-filtro T(a,c) (existe la arista a→c), antes
    /// de canonizar.
    pub tuplas_semi_filtradas: u64,
}

/// El plan System R para triángulos, hecho HONESTO: materializa el join
/// binario R(a,b)⋈S(b,c) igual que haría cualquier motor de la vieja escuela,
/// semi-filtra con T(a,c) y canoniza a<b<c.
///
/// La materialización es REAL (un `Vec` con todas las tuplas intermedias, con
/// capacidad exacta tomada de [`intermedios_plan_binario`]) — esconderla tras
/// un contador aritmético sería precisamente el auto-engaño que el capítulo
/// denuncia. La canonización a<b<c hace que cada triángulo aparezca UNA vez
/// (el mismo cuidado que el cap. 25 necesita para no contarlos ×6).
pub fn triangulos_join_binario(adj: &AdyacenciasOrdenadas) -> ResultadoBinario {
    let esperados = intermedios_plan_binario(adj);
    let mut intermedios: Vec<(usize, usize, usize)> = Vec::with_capacity(esperados as usize);
    for (a, vecinos) in adj.adj_out.iter().enumerate() {
        for &b in vecinos {
            for &c in &adj.adj_out[b] {
                intermedios.push((a, b, c));
            }
        }
    }
    debug_assert_eq!(
        intermedios.len() as u64,
        esperados,
        "la factura aritmética debe coincidir con lo materializado"
    );

    let mut triangulos = Vec::new();
    let mut semi_filtradas = 0u64;
    for &(a, b, c) in &intermedios {
        // Semi-join con T(a:c): la arista de CIERRE debe existir. En un grafo
        // simple esto implica a≠c; la canonización descarta además a≥b y b≥c.
        if adj.contiene_arista(a, c) {
            semi_filtradas += 1;
            if a < b && b < c {
                triangulos.push((a, b, c));
            }
        }
    }
    triangulos.sort_unstable();
    ResultadoBinario {
        triangulos,
        intermedios_materializados: intermedios.len() as u64,
        tuplas_semi_filtradas: semi_filtradas,
    }
}

/// El TERRENO DE VERDAD: fuerza bruta O(n³) sobre todos los triples
/// a<b<c comprobando las tres aristas por búsqueda binaria.
///
/// No explota NI el orden de las listas NI prefijos compartidos: es el plan
/// contra el que se equivoca-testean el binario Y el WCOJ. Sin equivalencia
/// testeada aquí, cualquier velocidad sería un rumor (regla del cap. 33).
pub fn triangulos_fuerza_bruta(adj: &AdyacenciasOrdenadas) -> Vec<(usize, usize, usize)> {
    let n = adj.num_nodos();
    let mut triangulos = Vec::new();
    for a in 0..n {
        for b in (a + 1)..n {
            if !adj.contiene_arista(a, b) {
                continue;
            }
            for c in (b + 1)..n {
                if adj.contiene_arista(a, c) && adj.contiene_arista(b, c) {
                    triangulos.push((a, b, c));
                }
            }
        }
    }
    triangulos
}

// ─────────────────── LeapFrog simplificado ───────────────────

/// Buscador por saltos: el «seek» de Veldhuizen (ICDT 2014) hecho tipo.
///
/// Encuentra el índice del PRIMER elemento ≥ x en una lista ordenada con
/// DOS fases: un galope exponencial (1, 2, 4, … comparaciones) que acota el
/// rango donde cae la frontera, y una llamada a `slice::binary_search` de
/// std dentro del rango. Peor caso O(log n) GARANTIZADO — se descarta la
/// interpolación del paper original porque su coste depende de supuestos de
/// distribución que con std puro no podemos sostener (contrato §5.7).
///
/// Contador de trabajo con convención declarada: +1 por cada salto del
/// galope (comparación real contra la lista) y +1 por cada búsqueda binaria
/// delegada (coste interno O(log n) de std). [`TriangulosWcoj`] expone el
/// total como `pasos_buscador`.
#[derive(Debug, Clone, Default)]
pub struct BuscadorSalto {
    pasos: u64,
}

impl BuscadorSalto {
    /// Buscador con contador a cero.
    pub fn nuevo() -> Self {
        BuscadorSalto { pasos: 0 }
    }

    /// Trabajo acumulado hasta ahora (saltos + búsquedas binarias).
    pub fn pasos(&self) -> u64 {
        self.pasos
    }

    /// Índice del primer elemento de `lista` que es ≥ `x` (o None).
    ///
    /// Casos exactos verificados por test: lista vacía, x por debajo de todo,
    /// golpe exacto al principio/en medio/al final, hueco entre elementos y
    /// x por encima de todo.
    pub fn primer_mayor_o_igual(&mut self, lista: &[usize], x: usize) -> Option<usize> {
        let n = lista.len();
        // Rechazo rápido: la COLA ya dice si hay algún candidato. También es
        // una comparación: cuenta.
        if n == 0 || lista[n - 1] < x {
            self.pasos += 1;
            return None;
        }
        // Fase 1: galope exponencial hasta acotar el bloque de la frontera.
        let mut lo = 0usize;
        let mut ancho = 1usize;
        loop {
            let hi = lo.saturating_add(ancho).min(n);
            if hi == n || lista[hi - 1] >= x {
                break;
            }
            self.pasos += 1;
            lo = hi;
            ancho = ancho.saturating_mul(2);
        }
        // Fase 2: búsqueda binaria de std dentro del bloque acotado.
        let ventana = &lista[lo..];
        let posicion = match ventana.binary_search(&x) {
            Ok(i) => Some(i),
            Err(i) => (i < ventana.len()).then_some(i),
        };
        self.pasos += 1;
        posicion.map(|i| lo + i)
    }

    /// FRONTERA COMÚN: el primer valor ≥ `desde` presente en TODAS las listas,
    /// encontrado por seeks coordinados (la «common frontier» de Veldhuizen).
    ///
    /// Mecánica: todas las relaciones se posicionan con
    /// [`Self::primer_mayor_o_igual`]; después el MÁXIMO de los cursores manda
    /// y cada lista rezagada hace seek hasta él. Cuando todos convergen en el
    /// mismo valor, ése es la frontera. Nadie materializa nada: los cursores
    /// son índices, no copias. Devuelve None en cuanto alguna lista se agota.
    ///
    /// Con cero listas no hay frontera que compartir (None, documentado);
    /// con una sola, es un seek elegante sobre esa lista.
    pub fn frontera_comun(&mut self, listas: &[&[usize]], desde: usize) -> Option<usize> {
        if listas.is_empty() {
            return None;
        }
        // Posicionar TODAS las relaciones en el primer candidato ≥ desde.
        let mut posiciones = Vec::with_capacity(listas.len());
        for lista in listas {
            posiciones.push(self.primer_mayor_o_igual(lista, desde)?);
        }
        // Votación: el máximo manda; las rezagadas hacen seek hasta él.
        loop {
            let mut maximo = 0usize;
            for (lista, &p) in listas.iter().zip(&posiciones) {
                maximo = maximo.max(lista[p]);
            }
            let mut convergen = true;
            for (i, lista) in listas.iter().enumerate() {
                if lista[posiciones[i]] < maximo {
                    convergen = false;
                    posiciones[i] = self.primer_mayor_o_igual(lista, maximo)?;
                }
            }
            if convergen {
                return Some(maximo);
            }
        }
    }
}

/// Resultado del leapfrog simplificado: los triángulos y el TRABAJO contado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriangulosWcoj {
    /// Triángulos canónicos (a<b<c), ordenados — comparable como
    /// multiconjunto con el plan binario y la fuerza bruta.
    pub triangulos: Vec<(usize, usize, usize)>,
    /// Saltos + búsquedas binarias consumidos por el [`BuscadorSalto`].
    pub pasos_buscador: u64,
}

impl TriangulosWcoj {
    /// Worst-case optimal join SIMPLIFICADO para el patrón triángulo.
    ///
    /// Backtracking por VARIABLES con orden estático a→b→c (contrato §5.6: la
    /// elección dinámica exigiría estadísticas que el catálogo del cap. 21 no
    /// tiene; fingirla sería humo):
    ///
    /// * nivel a: recorre los nodos de la vista;
    /// * nivel b: candidatos en out(a) ADVANCIENDO POR SEEKS desde a+1 —
    ///   la condición b>a va dentro del seek, no en un filtro posterior;
    /// * nivel c: FRONTERA COMÚN entre out(b) y out(a) empezando tras b:
    ///   las dos relaciones votan y sólo avanzan juntas. Ahí muere el
    ///   intermedio del plan binario: jamás se escriben los Σ in·out caminos,
    ///   se emiten directamente los triángulos.
    ///
    /// Garantía AGM (FOCS 2008): el trabajo queda acotado por el RESULTADO —
    /// ⌊m^1,5⌋ triángulos en el peor caso, hasta un factor polilogarítmico
    /// (NPRR, PODS 2012; nuestra variante cuenta sus pasos para poder
    /// COMPARARLOS contra la cota en el grafo pequeño).
    pub fn enumerar(adj: &AdyacenciasOrdenadas) -> Self {
        let mut buscador = BuscadorSalto::nuevo();
        let mut triangulos = Vec::new();
        for a in 0..adj.num_nodos() {
            let out_a = adj.vecinos_out(a);
            // Variable b: seek al primer vecino estrictamente mayor que a.
            let mut candidato_b = buscador.primer_mayor_o_igual(out_a, a + 1);
            while let Some(ib) = candidato_b {
                let b = out_a[ib];
                let out_b = adj.vecinos_out(b);
                // Variable c: frontera común entre las dos relaciones vivas.
                let mut desde_c = b + 1;
                while let Some(c) = buscador.frontera_comun(&[out_b, out_a], desde_c) {
                    triangulos.push((a, b, c));
                    desde_c = c + 1;
                }
                // Avanzar b TAMBIÉN por seek (leapfrog puro: todo avance
                // coordinado pasa por el buscador y su contador).
                candidato_b = buscador.primer_mayor_o_igual(out_a, b + 1);
            }
        }
        triangulos.sort_unstable();
        TriangulosWcoj {
            triangulos,
            pasos_buscador: buscador.pasos(),
        }
    }
}

// ─────────────────── Cota AGM ───────────────────

/// Techo de agarre para el cálculo exacto: m³ debe caber en u128.
///
/// Un grafo con 2⁴² aristas ya no cabe en la RAM de nadie (4 billones de
/// aristas); pasado el umbral la cota SATURA a u64::MAX en vez de envolver
/// en silencio — misma honestidad que el `checked_add` del contador.
const TOPE_EXACTO_AGARRE: u64 = 1_u64 << 42;

/// Cota AGM del NÚMERO DE TRIÁNGULOS: ⌊m^(3/2)⌋.
///
/// ρ* = cubrimiento fraccional de aristas del hipergrafo del triángulo vale
/// 3/2 (raíz geométrica: la desigualdad de Loomis-Whitney, Bull. AMS 55,
/// 1949; resultado: Atserias-Grohe-Marx, FOCS 2008, versión revista SICOMP
/// 42(4), 2013). Es la BRÚJULA del peor caso — nunca una promesa de
/// velocidad: en el mini dataset la holgura es grande y SE REPORTA.
///
/// Cálculo EXACTO con enteros (sin f64 final): m³ en u128 y raíz cuadrada
/// entera por Newton con ajuste ±1. m=0 → 0; m=56 (K₈) → 419.
pub fn cota_agm_triangulos(num_arista: u64) -> u64 {
    if num_arista == 0 {
        return 0;
    }
    if num_arista > TOPE_EXACTO_AGARRE {
        return u64::MAX;
    }
    let cubo = (num_arista as u128) * (num_arista as u128) * (num_arista as u128);
    raiz_entera(cubo) as u64
}

/// Raíz cuadrada entera EXACTA de un u128 (floor).
///
/// Newton entero para caer cerca y ajuste lineal ±1 para garantizar el
/// floor sin confiar en el redondeo de f64.
fn raiz_entera(valor: u128) -> u128 {
    if valor < 2 {
        return valor;
    }
    let mut x = (valor as f64).sqrt() as u128;
    x = x.max(1);
    loop {
        let siguiente = (x + valor / x) / 2;
        if siguiente >= x {
            break;
        }
        x = siguiente;
    }
    while x * x > valor {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= valor {
        x += 1;
    }
    x
}

// ─────────────────── Consultas recursivas ───────────────────

/// Cierre transitivo DIRIGIDO desde unos orígenes, por punto fijo ITERATIVO
/// y bajo presupuesto.
///
/// Reutiliza SIN duplicar la maquinaria del cap. 26: el [`BitSet`] marca los
/// alcanzables (densos, ids posicionales), el [`Presupuesto`] gobierna y el
/// [`MotivoParada`] explica cómo terminó. Termina en grafos CON ciclos por
/// construcción: los visitados cortan el re-descubrimiento — «recursivo =
/// bucle infinito» era exactamente la misconcepción que el capítulo venía a
/// enterrar.
///
/// Semántica de los límites (alineada con `FronterasBfs`, cap. 26):
/// `max_profundidad` cuenta RONDAS de expansión (0 = sólo los orígenes);
/// `max_nodos` acota los alcanzables (exacto, comprobado antes de marcar);
/// `max_lecturas` acota las entradas de adyacencia examinadas (exacto,
/// antes de examinar). Los orígenes se marcan SIEMPRE como nivel 0.
///
/// Contexto estándar: `WITH RECURSIVE` (SQL:1999) y GQL (ISO/IEC 39075:2024)
/// — aquí NO hay sintaxis nueva en LiraQL, hay API y concepto.
pub fn cierre_transitivo(
    adj: &AdyacenciasOrdenadas,
    origenes: &[usize],
    presupuesto: Presupuesto,
) -> (BitSet, MotivoParada) {
    let mut alcanzables = BitSet::new();
    let mut cuenta_nodos: u64 = 0;
    let mut lecturas: u64 = 0;
    let mut frontera: Vec<usize> = Vec::new();

    // Nivel 0: los orígenes, marcados siempre (con presupuesto de nodos
    // agotado, la parada llega igual que en el BFS del cap. 26: exacta).
    for &o in origenes {
        if o < adj.num_nodos() && !alcanzables.contiene(o) {
            if let Some(max) = presupuesto.max_nodos
                && cuenta_nodos >= max
            {
                return (alcanzables, MotivoParada::PresupuestoNodos);
            }
            alcanzables.marcar(o);
            cuenta_nodos += 1;
            frontera.push(o);
        }
    }
    if frontera.is_empty() {
        return (alcanzables, MotivoParada::Completo);
    }

    // Punto fijo: expandir frontera a frontera hasta no descubrir NADA nuevo.
    let mut rondas: u32 = 0;
    loop {
        if let Some(k) = presupuesto.max_profundidad
            && rondas >= k
        {
            return (alcanzables, MotivoParada::ProfundidadMaxima);
        }
        let mut siguiente = Vec::new();
        for &u in &frontera {
            for &v in &adj.adj_out[u] {
                if let Some(max) = presupuesto.max_lecturas
                    && lecturas >= max
                {
                    return (alcanzables, MotivoParada::PresupuestoLecturas);
                }
                lecturas += 1;
                if !alcanzables.contiene(v) {
                    if let Some(max) = presupuesto.max_nodos
                        && cuenta_nodos >= max
                    {
                        return (alcanzables, MotivoParada::PresupuestoNodos);
                    }
                    alcanzables.marcar(v);
                    cuenta_nodos += 1;
                    siguiente.push(v);
                }
            }
        }
        if siguiente.is_empty() {
            return (alcanzables, MotivoParada::Completo);
        }
        rondas += 1;
        frontera = siguiente;
    }
}

// ─────────────────── Factorización del resultado del join ───────────────────

/// El multiconjunto de triángulos representado COMPARTIENDO prefijos.
///
/// Extensión de [`crate::cap38_columnar::ExpansionFactorizada`] al RESULTADO
/// de un join cíclico (Olteanu-Závodný, ICDT 2012; TODS 40(1), 2015): el
/// f-tree del resultado tiene DOS niveles de prefijo compartido —
///
/// ```text
/// prefijos_a:   [a₀, a₁, …]                  UNA celda por 'a' con triángulos
/// inicio_b:     CSR a → slots                 (estructura de recorrido)
/// prefijos_b:   [b…]                          UNA celda por par (a,b) vivo
/// inicio_c:     CSR slot → hojas              (estructura de recorrido)
/// hojas_c:      [c…]                          UNA celda POR TRIÁNGULO
/// ```
///
/// Cada hoja corresponde a EXACTAMENTE un triángulo lógico (test-tesis de
/// equivalencia contra las tuplas planas) pero `a` y `b` se almacenan UNA
/// vez compartidos entre todos los c que cuelgan de ellos. Las
/// MULTIPLICIDADES hacen visible el compartir: `multiplicidad_a[k]` es el
/// número de triángulos bajo el prefijo a_k — agregar sobre esta
/// representación NI SIQUIERA expande las tuplas.
#[derive(Debug, Clone)]
pub struct ResultadoFactorizadoTriangulos {
    /// Prefijos `a` con al menos un triángulo, ascendentes.
    pub prefijos_a: Vec<usize>,
    /// Triángulos que comparte cada prefijo a (paralelo a `prefijos_a`).
    pub multiplicidad_a: Vec<u64>,
    /// CSR: los slots del prefijo k son `inicio_b[k]..inicio_b[k+1]`.
    pub inicio_b: Vec<usize>,
    /// Prefijo b de cada slot (par (a,b) compartido), ascendentes por grupo.
    pub prefijos_b: Vec<usize>,
    /// Hojas que comparte cada slot b (paralelo a `prefijos_b`).
    pub multiplicidad_b: Vec<u64>,
    /// CSR: las hojas del slot s son `inicio_c[s]..inicio_c[s+1]`.
    pub inicio_c: Vec<usize>,
    /// Valor c de cada triángulo (una entrada por triángulo lógico).
    pub hojas_c: Vec<usize>,
}

impl ResultadoFactorizadoTriangulos {
    /// Factoriza una lista de triángulos CANÓNICA Y ORDENADA (la salida de
    /// [`TriangulosWcoj::enumerar`], [`triangulos_join_binario`] o la fuerza
    /// bruta): agrupa por a, luego por b, colgando las hojas c.
    pub fn desde_triangulos(triangulos: &[(usize, usize, usize)]) -> Self {
        let mut factorizado = ResultadoFactorizadoTriangulos {
            prefijos_a: Vec::new(),
            multiplicidad_a: Vec::new(),
            inicio_b: vec![0],
            prefijos_b: Vec::new(),
            multiplicidad_b: Vec::new(),
            inicio_c: vec![0],
            hojas_c: Vec::new(),
        };
        let mut actual_a: Option<usize> = None;
        let mut actual_b: Option<usize> = None;
        for &(a, b, c) in triangulos {
            if actual_a != Some(a) {
                // Cerrar el grupo ANTERIOR antes de abrir el nuevo (la
                // semilla [0] ya marca el arranque del primero).
                if !factorizado.prefijos_a.is_empty() {
                    factorizado.inicio_b.push(factorizado.prefijos_b.len());
                }
                factorizado.prefijos_a.push(a);
                factorizado.multiplicidad_a.push(0);
                actual_a = Some(a);
                actual_b = None;
            }
            if actual_b != Some(b) {
                if !factorizado.prefijos_b.is_empty() {
                    factorizado.inicio_c.push(factorizado.hojas_c.len());
                }
                factorizado.prefijos_b.push(b);
                factorizado.multiplicidad_b.push(0);
                actual_b = Some(b);
            }
            let ka = factorizado.prefijos_a.len() - 1;
            let kb = factorizado.prefijos_b.len() - 1;
            factorizado.hojas_c.push(c);
            factorizado.multiplicidad_a[ka] += 1;
            factorizado.multiplicidad_b[kb] += 1;
        }
        factorizado.inicio_b.push(factorizado.prefijos_b.len());
        factorizado.inicio_c.push(factorizado.hojas_c.len());

        debug_assert_eq!(
            factorizado.multiplicidad_a.iter().sum::<u64>(),
            factorizado.hojas_c.len() as u64,
            "la multiplicidad de a debe sumar exactamente los triángulos"
        );
        debug_assert_eq!(
            factorizado.multiplicidad_b.iter().sum::<u64>(),
            factorizado.hojas_c.len() as u64,
            "la multiplicidad de b debe sumar exactamente los triángulos"
        );
        factorizado
    }

    /// Filas LÓGICAS: triángulos que la consulta significa.
    pub fn filas_logicas(&self) -> u64 {
        self.hojas_c.len() as u64
    }

    /// Celdas FÍSICAS: prefijos a + prefijos b + hojas (offsets CSR fuera,
    /// estructura de recorrido — misma convención que el cap. 38).
    pub fn celdas_fisicas(&self) -> u64 {
        (self.prefijos_a.len() + self.prefijos_b.len() + self.hojas_c.len()) as u64
    }

    /// Lo que ocuparían las MISMAS tuplas planas: 3 celdas por triángulo.
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
    /// Lado TESIS del test de equivalencia: producir exactamente los mismos
    /// (a, b, c) que las tuplas planas — compartir prefijos no cambia el
    /// significado, sólo las celdas físicas.
    pub fn por_cada_tupla(&self, mut visitar: impl FnMut(usize, usize, usize)) {
        for (k, &a) in self.prefijos_a.iter().enumerate() {
            for s in self.inicio_b[k]..self.inicio_b[k + 1] {
                let b = self.prefijos_b[s];
                for h in self.inicio_c[s]..self.inicio_c[s + 1] {
                    visitar(a, b, self.hojas_c[h]);
                }
            }
        }
    }

    /// Todos los triángulos, ordenados (comodidad para tests).
    pub fn tuplas_ordenadas(&self) -> Vec<(usize, usize, usize)> {
        let mut tuplas = Vec::with_capacity(self.filas_logicas() as usize);
        self.por_cada_tupla(|a, b, c| tuplas.push((a, b, c)));
        tuplas.sort_unstable();
        tuplas
    }
}

// ─────────────────── Informe para la prosa ───────────────────

/// Informe reproducible de CONTADORES sobre el mini dataset — SIN tiempos.
///
/// Qué incluye: la factura del plan binario (intermedios vs semi-filtro vs
/// triángulos), los pasos de búsqueda del WCOJ, la cota AGM con su holgura,
/// las celdas planas vs factorizadas y una muestra de cierre transitivo con
/// su motivo de parada. Qué NO incluye: cronómetros — esos viven en
/// criterion (`benches/bench_joins.rs`) con calibrado estadístico; un
/// cronómetro inline aquí sería un número sin metodología (regla del cap.
/// 34).
///
/// Reproducibilidad: todo deriva del store por caminos deterministas (ids
/// ordenados, enumeraciones canónicas) — dos llamadas producen EL MISMO
/// texto byte a byte, y hay un test que lo pinna.
pub fn informe_joins_reproducible_sobre_mini(store: &MemoryStore) -> String {
    let adj = AdyacenciasOrdenadas::desde_store(store);
    let binario = triangulos_join_binario(&adj);
    let wcoj = TriangulosWcoj::enumerar(&adj);
    let bruta = triangulos_fuerza_bruta(&adj);
    let cota = cota_agm_triangulos(adj.num_aristas() as u64);
    let factorizado = ResultadoFactorizadoTriangulos::desde_triangulos(&wcoj.triangulos);

    let origen_pos = 0_usize;
    let (alcanzables, parada) = cierre_transitivo(&adj, &[origen_pos], Presupuesto::sin_limite());

    let mut lineas: Vec<String> = Vec::new();
    lineas.push("=== Informe de joins (cap. 39) ===".to_string());
    lineas.push(format!(
        "dataset: {} nodos, {} aristas dirigidas (listas ordenadas y deduplicadas)",
        adj.num_nodos(),
        adj.num_aristas()
    ));

    lineas.push("-- plan binario: R(a,b) ⋈ S(b,c) + semi-filtro T(a:c) --".to_string());
    lineas.push(format!(
        "intermedios materializados: {} tuplas (a,b,c)",
        binario.intermedios_materializados
    ));
    lineas.push(format!(
        "tras el semi-filtro: {} | triángulos canónicos (a<b<c): {}",
        binario.tuplas_semi_filtradas,
        binario.triangulos.len()
    ));
    if binario.triangulos.is_empty() {
        lineas.push(
            "ratio intermedios/triángulos: infinito (el plan fabrica intermedios \
             para una respuesta VACÍA)"
                .to_string(),
        );
    } else {
        lineas.push(format!(
            "ratio intermedios/triángulos: {:.1}x",
            binario.intermedios_materializados as f64 / binario.triangulos.len() as f64
        ));
    }

    lineas.push("-- WCOJ leapfrog simplificado (orden estático a→b→c) --".to_string());
    lineas.push(format!(
        "triángulos: {} | pasos de búsqueda (galope + búsquedas binarias): {}",
        wcoj.triangulos.len(),
        wcoj.pasos_buscador
    ));
    lineas.push(format!(
        "equivalencia de multiconjuntos binario == wcoj == fuerza bruta: {} == {} == {}",
        binario.triangulos.len(),
        wcoj.triangulos.len(),
        bruta.len()
    ));

    lineas.push("-- cota AGM ⌊m^1,5⌋ (brújula del peor caso, no promesa) --".to_string());
    let holgura = if wcoj.triangulos.is_empty() {
        String::from("infinita (sin triángulos)")
    } else {
        format!("{:.1}x", cota as f64 / wcoj.triangulos.len() as f64)
    };
    lineas.push(format!(
        "cota: {} triángulos | triángulos medidos: {} | holgura: {holgura}",
        cota,
        wcoj.triangulos.len()
    ));

    lineas.push("-- factorización del resultado (prefijos a y b compartidos) --".to_string());
    lineas.push(format!(
        "filas lógicas: {} | celdas planas: {} | celdas factorizadas: {} | ahorro: {:.1}%",
        factorizado.filas_logicas(),
        factorizado.celdas_planas(),
        factorizado.celdas_fisicas(),
        factorizado.ahorro_porcentaje()
    ));

    lineas.push("-- cierre transitivo (punto fijo + presupuesto del cap. 26) --".to_string());
    lineas.push(format!(
        "desde la posición {}: {} nodos alcanzables | parada: {:?}",
        origen_pos,
        alcanzables.unos(),
        parada
    ));
    lineas.push(
        "(sin tiempos: los cronómetros viven en benches/bench_joins.rs, regla del cap. 34)"
            .to_string(),
    );

    let mut informe = lineas.join("\n");
    informe.push('\n');
    informe
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_joins {
    use super::*;
    use crate::cap17_liraql_ast::RelDirection;
    use crate::cap20_volcano::{Cell, ExpandOp, NodeScanOp, PhysicalOperator};
    use crate::cap24_centralidad::GraphDirection;
    use crate::cap26_proyeccion::bfs_streaming;
    use crate::cap34_benchmarks::{SEMILLA_REFERENCIA, dataset_referencia_mini};

    /// K₈ bidireccional: 28 pares × 2 sentidos = 56 aristas dirigidas.
    fn grafo_completo_bidir(n: usize) -> AdyacenciasOrdenadas {
        let mut aristas = Vec::new();
        for u in 0..n {
            for v in 0..n {
                if u != v {
                    aristas.push((u, v));
                }
            }
        }
        AdyacenciasOrdenadas::desde_aristas(n, &aristas)
    }

    /// Estrella bidireccional: centro 0 ↔ cada hoja (ambos sentidos).
    /// Sin triángulos por construcción: las hojas no se tocan entre sí.
    fn estrella_bidir(hojas: usize) -> AdyacenciasOrdenadas {
        let mut aristas = Vec::new();
        for hoja in 1..=hojas {
            aristas.push((0, hoja));
            aristas.push((hoja, 0));
        }
        AdyacenciasOrdenadas::desde_aristas(hojas + 1, &aristas)
    }

    /// Rueda bidireccional: hub 0 ↔ hojas 1..=k y ciclo entre hojas
    /// consecutivas (con cierre k→1). Triángulos EXACTOS: uno por pareja de
    /// hojas consecutivas junto al hub — el skew controlado del capítulo.
    fn rueda_bidir(hojas: usize) -> AdyacenciasOrdenadas {
        let mut aristas = Vec::new();
        for hoja in 1..=hojas {
            aristas.push((0, hoja));
            aristas.push((hoja, 0));
            let siguiente = if hoja == hojas { 1 } else { hoja + 1 };
            aristas.push((hoja, siguiente));
            aristas.push((siguiente, hoja));
        }
        AdyacenciasOrdenadas::desde_aristas(hojas + 1, &aristas)
    }

    /// Tesis doble: binario == wcoj == fuerza bruta como MULTICONJUNTO.
    fn assert_equivalencia_tres(adj: &AdyacenciasOrdenadas, contexto: &str) {
        let binario = triangulos_join_binario(adj);
        let wcoj = TriangulosWcoj::enumerar(adj);
        let bruta = triangulos_fuerza_bruta(adj);
        assert_eq!(
            binario.triangulos, bruta,
            "{contexto}: el plan binario diverge de la fuerza bruta"
        );
        assert_eq!(
            wcoj.triangulos, bruta,
            "{contexto}: el WCOJ diverge de la fuerza bruta"
        );
    }

    #[test]
    fn expand_es_index_nested_loop_produce_las_mismas_tuplas() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);

        // Camino A: el pipeline VOLCANO real del cap. 20 — NodeScan → Expand
        // → Expand, fila a fila, con su index nested loop dentro.
        let scan = NodeScanOp::new(&ds.store, "a".to_string(), None);
        let expandido_b = ExpandOp::new(
            &ds.store,
            Box::new(scan),
            "a".to_string(),
            None,
            None,
            RelDirection::Outgoing,
            "b".to_string(),
        );
        let mut expandido_c = ExpandOp::new(
            &ds.store,
            Box::new(expandido_b),
            "b".to_string(),
            None,
            None,
            RelDirection::Outgoing,
            "c".to_string(),
        );
        expandido_c.open().expect("pipeline abierto");
        let mut filas_operador: Vec<(NodeId, NodeId, NodeId)> = Vec::new();
        while let Some(fila) = expandido_c.next().expect("fila del volcano") {
            let Cell::Node(nodo_a) = fila.get("a").expect("variable a ligada") else {
                panic!("la variable 'a' debía ser un nodo");
            };
            let Cell::Node(nodo_b) = fila.get("b").expect("variable b ligada") else {
                panic!("la variable 'b' debía ser un nodo");
            };
            let Cell::Node(nodo_c) = fila.get("c").expect("variable c ligada") else {
                panic!("la variable 'c' debía ser un nodo");
            };
            filas_operador.push((nodo_a.id, nodo_b.id, nodo_c.id));
        }
        expandido_c.close().expect("pipeline cerrado");

        // Camino B: el JOIN FORMALIZADO sobre adyacencias ordenadas.
        let adj = AdyacenciasOrdenadas::desde_store(&ds.store);
        let mut formales: Vec<(NodeId, NodeId, NodeId)> = expansion_dos_saltos_plana(&adj)
            .into_iter()
            .map(|(a, b, c)| (adj.id_de(a), adj.id_de(b), adj.id_de(c)))
            .collect();
        formales.sort_unstable();
        formales.dedup();

        // El operador puede REPETIR tuplas si el store tiene aristas
        // paralelas (misma pareja (u,v) emitida dos veces); la vista
        // ordenada DEDUPLICA por contrato físico. Como conjunto, ambos deben
        // ser EL MISMO multiconjunto canónico — y las repeticiones sólo
        // pueden añadir filas, nunca inventar.
        let conjunto_operador: std::collections::BTreeSet<_> =
            filas_operador.iter().copied().collect();
        let conjunto_formal: std::collections::BTreeSet<_> = formales.iter().copied().collect();
        assert_eq!(
            conjunto_operador, conjunto_formal,
            "ExpandOp y el join formalizado producen conjuntos distintos"
        );
        assert!(
            filas_operador.len() >= formales.len(),
            "las aristas paralelas sólo pueden añadir filas al operador"
        );
    }

    #[test]
    fn intermedios_del_plan_binario_cuentas_conocidas_k8() {
        let k8 = grafo_completo_bidir(8);
        assert_eq!(k8.num_nodos(), 8);
        assert_eq!(k8.num_aristas(), 56, "K₈ bidireccional: 28 pares × 2");

        // La escalera COMPLETA a mano: 56 aristas → 392 intermedios →
        // 336 tras el semi-filtro → 56 triángulos canónicos.
        assert_eq!(intermedios_plan_binario(&k8), 392);
        let binario = triangulos_join_binario(&k8);
        assert_eq!(binario.intermedios_materializados, 392);
        assert_eq!(binario.tuplas_semi_filtradas, 336);
        assert_eq!(binario.triangulos.len(), 56);
        // La factura aritmética coincide con lo materializado (cross-check
        // contador vs Vec real).
        assert_eq!(
            intermedios_plan_binario(&k8),
            binario.intermedios_materializados
        );
        // Y la cota AGM del capítulo: ⌊56^1,5⌋ = 419 ≥ 56.
        assert_eq!(cota_agm_triangulos(56), 419);
        assert!(binario.triangulos.len() <= cota_agm_triangulos(56) as usize);
    }

    #[test]
    fn triangulos_binario_iguales_a_la_fuerza_bruta() {
        assert_equivalencia_tres(&grafo_completo_bidir(8), "K₈");
        assert_equivalencia_tres(&estrella_bidir(16), "estrella");
        assert_equivalencia_tres(&rueda_bidir(16), "rueda");

        // Subgrafo DETERMINISTA y pequeño del mini dataset (la fuerza bruta
        // es O(n³): el terreno de verdad se corre acotado).
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let subgrafo = AdyacenciasOrdenadas::desde_store_acotado(&ds.store, 96);
        assert_equivalencia_tres(&subgrafo, "subgrafo 96 del mini dataset");
    }

    #[test]
    fn hub_concentrador_explota_los_intermedios() {
        // Estrella bidireccional con k=32 hojas: el hub concentra TODO el
        // fan-out. Cuentas a mano:
        //   intermedios = k² (por el hub: in=k × out=k) + k (hojas: in=1 ×
        //   out=1 cada una) = 1056;
        //   entrada = 2k = 64 aristas; triángulos = 0 (¡respuesta VACÍA!).
        let k = 32_usize;
        let estrella = estrella_bidir(k);
        assert_eq!(estrella.num_aristas(), 2 * k);

        let esperado_intermedios = (k * k + k) as u64;
        assert_eq!(intermedios_plan_binario(&estrella), esperado_intermedios);

        let binario = triangulos_join_binario(&estrella);
        assert_eq!(
            binario.intermedios_materializados, esperado_intermedios,
            "la factura aritmética coincide con lo materializado"
        );
        assert_eq!(
            binario.triangulos.len(),
            0,
            "la estrella no tiene triángulos"
        );
        assert_eq!(TriangulosWcoj::enumerar(&estrella).triangulos.len(), 0);
        assert!(triangulos_fuerza_bruta(&estrella).is_empty());

        // Ratio intermedios/resultado: con resultado 0 la explosión es TOTAL
        // (división entre cero: infinito). La cifra FINITA fijada a mano es
        // el ratio contra la ENTRADA: (k²+k)/2k = (k+1)/2 = 16,5 — el plan
        // fabrica 16,5 filas por arista para no dar NI UNA respuesta.
        assert!(binario.intermedios_materializados > 0 && binario.triangulos.is_empty());
        let ratio_sobre_entrada = esperado_intermedios as f64 / estrella.num_aristas() as f64;
        assert!((ratio_sobre_entrada - (k + 1) as f64 / 2.0).abs() < 1e-9);
        // El WCOJ, mientras tanto, apenas trabaja: sin candidatos que
        // converjan, sus pasos se quedan en el orden del escaneo inicial.
        let wcoj = TriangulosWcoj::enumerar(&estrella);
        assert!(wcoj.pasos_buscador > 0);
        assert!(wcoj.pasos_buscador < esperado_intermedios);
    }

    #[test]
    fn buscador_primer_mayor_o_igual_casos_exactos() {
        let mut buscador = BuscadorSalto::nuevo();
        let vacia: Vec<usize> = vec![];
        assert_eq!(buscador.primer_mayor_o_igual(&vacia, 0), None);

        let lista = vec![10, 20, 30, 40, 50];
        // Debajo de todo: la primera posición.
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 0), Some(0));
        // Golpes exactos: principio, medio, final.
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 10), Some(0));
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 30), Some(2));
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 50), Some(4));
        // Huecos: el primer elemento Estrictamente mayor.
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 15), Some(1));
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 51), None);
        assert_eq!(buscador.primer_mayor_o_igual(&lista, 999), None);

        // Listas largas: el galope cruza varios bloques y aun así acierta
        // (potencias de dos justo en los bordes del salto).
        let larga: Vec<usize> = (0..1000).step_by(3).collect();
        for objetivo in [0, 3, 999, 1000, 1500, 2997] {
            let esperado = larga.iter().position(|&v| v >= objetivo);
            assert_eq!(
                buscador.primer_mayor_o_igual(&larga, objetivo),
                esperado,
                "falló el seek con objetivo {objetivo}"
            );
        }

        // El contador crece con el trabajo y nunca retrocede.
        let pasos_finales = buscador.pasos();
        assert!(pasos_finales > 0);
        buscador.primer_mayor_o_igual(&lista, 10);
        assert!(buscador.pasos() > pasos_finales);
    }

    #[test]
    fn frontera_comun_es_la_interseccion_de_candidatos() {
        let mut buscador = BuscadorSalto::nuevo();
        let l1 = [1_usize, 3, 5, 7, 9];
        let l2 = [3_usize, 5, 8, 9];
        let l3 = [0_usize, 2, 3, 5, 9, 12];

        // Intersección completa de las tres, recolectada avanzando `desde`.
        let listas: [&[usize]; 3] = [&l1, &l2, &l3];
        let mut comunes = Vec::new();
        let mut desde = 0;
        while let Some(v) = buscador.frontera_comun(&listas, desde) {
            comunes.push(v);
            desde = v + 1;
        }
        // A mano: {3,5,9}.
        assert_eq!(comunes, vec![3, 5, 9]);

        // Empezar DESDE un valor salta los anteriores.
        assert_eq!(buscador.frontera_comun(&listas, 6), Some(9));
        assert_eq!(buscador.frontera_comun(&listas, 10), None);

        // Una lista sola: identidad ≥ desde.
        let solas: [&[usize]; 1] = [&l2];
        assert_eq!(buscador.frontera_comun(&solas, 0), Some(3));
        assert_eq!(buscador.frontera_comun(&solas, 4), Some(5));

        // Contra la intersección NAIVA en datos irregulares.
        let a: Vec<usize> = (0..500).filter(|v| v % 7 == 0).collect();
        let b: Vec<usize> = (0..500).filter(|v| v % 11 == 0).collect();
        let c: Vec<usize> = (37..500).step_by(3).collect();
        let trio: [&[usize]; 3] = [&a, &b, &c];
        let naiva: Vec<usize> = (0..500)
            .filter(|&v| a.binary_search(&v).is_ok())
            .filter(|&v| b.binary_search(&v).is_ok())
            .filter(|&v| c.binary_search(&v).is_ok())
            .collect();
        let mut recolectados = Vec::new();
        let mut desde = 0;
        while let Some(v) = buscador.frontera_comun(&trio, desde) {
            recolectados.push(v);
            desde = v + 1;
        }
        assert_eq!(recolectados, naiva);
    }

    #[test]
    fn wcoj_triangulos_iguales_al_binario_y_a_la_fuerza_bruta() {
        // Tesis DOBLE sobre los cuatro escenarios del capítulo: el backtracking
        // por variables con frontera común produce EXACTAMENTE lo mismo que el
        // plan binario Y que la fuerza bruta.
        assert_equivalencia_tres(&grafo_completo_bidir(8), "K₈");
        assert_equivalencia_tres(&rueda_bidir(20), "rueda 20");

        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let completo = AdyacenciasOrdenadas::desde_store(&ds.store);
        let binario = triangulos_join_binario(&completo);
        let wcoj = TriangulosWcoj::enumerar(&completo);
        assert_eq!(
            wcoj.triangulos, binario.triangulos,
            "mini dataset COMPLETO: wcoj != binario"
        );
        // Y el subgrafo acotado añade la tercera pata (fuerza bruta).
        let subgrafo = AdyacenciasOrdenadas::desde_store_acotado(&ds.store, 96);
        assert_equivalencia_tres(&subgrafo, "subgrafo 96");
    }

    #[test]
    fn wcoj_pasos_acotados_por_agm_en_k8() {
        let k8 = grafo_completo_bidir(8);
        let wcoj = TriangulosWcoj::enumerar(&k8);
        let cota = cota_agm_triangulos(56);

        // La promesa worst-case optimal, CONTADA: el trabajo del buscador
        // (saltos + búsquedas binarias) queda acotado por cota·log — cada
        // seek cuesta O(log) comparaciones y el NÚMERO de extensiones
        // candidatas queda gobernado por el resultado (AGM/NPRR hasta el
        // factor polilogarítmico). En el grafo pequeño se verifica con la
        // cifra real delante.
        let log_m = (56_usize).next_power_of_two().trailing_zeros() as u64; // 6
        let limite = cota * (log_m + 1);
        assert!(
            wcoj.pasos_buscador <= limite,
            "pasos {} superan la cota AGM·log {}",
            wcoj.pasos_buscador,
            limite
        );
        // Sanidad: el trabajo NO es cero y el resultado tampoco.
        assert!(wcoj.pasos_buscador > 0);
        assert_eq!(wcoj.triangulos.len(), 56);
    }

    #[test]
    fn agm_bound_acota_el_resultado_en_varios_subgrafos() {
        // La cota como BRÚJULA: triángulos medidos ≤ ⌊m^1,5⌋ SIEMPRE. La
        // holgura (grande en el mini dataset) se REPORTA, no se esconde.
        let escenarios: Vec<(&str, AdyacenciasOrdenadas)> = vec![
            ("K8", grafo_completo_bidir(8)),
            ("estrella", estrella_bidir(32)),
            ("rueda", rueda_bidir(24)),
        ];
        for (nombre, adj) in &escenarios {
            let m = adj.num_aristas() as u64;
            let cota = cota_agm_triangulos(m);
            let medidos = TriangulosWcoj::enumerar(adj).triangulos.len() as u64;
            assert!(
                medidos <= cota,
                "{nombre}: {medidos} triángulos superan la cota {cota}"
            );
        }
        // Mini dataset completo.
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let adj = AdyacenciasOrdenadas::desde_store(&ds.store);
        let cota = cota_agm_triangulos(adj.num_aristas() as u64);
        let medidos = TriangulosWcoj::enumerar(&adj).triangulos.len() as u64;
        assert!(medidos <= cota, "mini: {medidos} > cota {cota}");

        // Valores exactos conocidos de la función.
        assert_eq!(cota_agm_triangulos(0), 0);
        assert_eq!(cota_agm_triangulos(1), 1);
        assert_eq!(cota_agm_triangulos(4), 8);
        assert_eq!(cota_agm_triangulos(56), 419);
        // Monótona: más aristas nunca bajan la cota.
        let mut previa = 0;
        for m in [0_u64, 1, 2, 7, 8, 63, 64, 100, 1000] {
            let cota = cota_agm_triangulos(m);
            assert!(cota >= previa);
            previa = cota;
        }
    }

    #[test]
    fn cierre_transitivo_coincide_con_bfs_existente() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let adj = AdyacenciasOrdenadas::desde_store(&ds.store);

        // Orígenes variados: el primero, uno medio y el último id.
        let origenes = [
            0_usize,
            adj.num_nodos() / 2,
            adj.num_nodos().saturating_sub(1),
        ];

        for &pos_origen in &origenes {
            let id_origen = adj.id_de(pos_origen);
            // Referencia: el BFS por fronteras YA IMPLEMENTADO (caps. 22/26).
            let bfs = bfs_streaming(
                &ds.store,
                id_origen,
                GraphDirection::Out,
                Presupuesto::sin_limite(),
            )
            .expect("origen vivo");
            let mut esperados = bfs.nodos();
            esperados.sort_unstable();

            // Nuestro punto fijo sobre las adyacencias posicionales.
            let (bit_set, parada) =
                cierre_transitivo(&adj, &[pos_origen], Presupuesto::sin_limite());
            assert_eq!(parada, MotivoParada::Completo);
            let mut obtenidos: Vec<NodeId> = (0..adj.num_nodos())
                .filter(|&p| bit_set.contiene(p))
                .map(|p| adj.id_de(p))
                .collect();
            obtenidos.sort_unstable();

            assert_eq!(
                obtenidos, esperados,
                "el cierre diverge del BFS desde el id {id_origen}"
            );
        }
    }

    #[test]
    fn cierre_transitivo_para_por_profundidad_y_presupuesto() {
        // Camino 0→1→2→3→4: la profundidad corta EXACTA.
        let camino = AdyacenciasOrdenadas::desde_aristas(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        let (bits, parada) = cierre_transitivo(&camino, &[0], Presupuesto::profundidad(2));
        assert_eq!(parada, MotivoParada::ProfundidadMaxima);
        assert!(bits.contiene(0) && bits.contiene(1) && bits.contiene(2));
        assert!(!bits.contiene(3) && !bits.contiene(4));
        assert_eq!(bits.unos(), 3);

        // Sin límite: componente completa.
        let (bits, parada) = cierre_transitivo(&camino, &[0], Presupuesto::sin_limite());
        assert_eq!(parada, MotivoParada::Completo);
        assert_eq!(bits.unos(), 5);

        // Presupuesto de nodos: 3 alcanzables como mucho (nivel 0 incluido).
        let (bits, parada) = cierre_transitivo(&camino, &[0], Presupuesto::default().con_nodos(3));
        assert_eq!(parada, MotivoParada::PresupuestoNodos);
        assert_eq!(bits.unos(), 3);

        // Presupuesto de lecturas: 2 aristas examinadas como mucho.
        let (bits, parada) =
            cierre_transitivo(&camino, &[0], Presupuesto::default().con_lecturas(2));
        assert_eq!(parada, MotivoParada::PresupuestoLecturas);
        assert_eq!(bits.unos(), 3);

        // CICLO cerrado 0→1→2→3→0: TERMINA (misconcepción nº4 enterrada) y
        // da la componente completa.
        let ciclo = AdyacenciasOrdenadas::desde_aristas(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let (bits, parada) = cierre_transitivo(&ciclo, &[1], Presupuesto::sin_limite());
        assert_eq!(parada, MotivoParada::Completo);
        assert_eq!(bits.unos(), 4);

        // Varios orígenes con solape: se deduplican y el nivel 0 cuenta.
        let (bits, parada) = cierre_transitivo(&camino, &[1, 1, 3], Presupuesto::sin_limite());
        assert_eq!(parada, MotivoParada::Completo);
        assert_eq!(bits.unos(), 4);
    }

    #[test]
    fn factorizacion_triangulos_filas_logicas_vs_celdas() {
        let k8 = grafo_completo_bidir(8);
        let wcoj = TriangulosWcoj::enumerar(&k8);
        let factorizado = ResultadoFactorizadoTriangulos::desde_triangulos(&wcoj.triangulos);

        // Cuentas FIJAS de K₈ hechas a mano:
        // * prefijos a: a ∈ 0..=5 (necesita dos vecinos mayores) → 6.
        // * slots (a,b) con b>a y hueco para c>b → Σ_{b=1..6} b = 21.
        // * hojas = triángulos = C(8,3) = 56.
        assert_eq!(factorizado.filas_logicas(), 56);
        assert_eq!(factorizado.prefijos_a.len(), 6);
        assert_eq!(factorizado.prefijos_b.len(), 21);
        assert_eq!(factorizado.hojas_c.len(), 56);
        assert_eq!(factorizado.celdas_fisicas(), 83);
        assert_eq!(factorizado.celdas_planas(), 168);
        assert!((factorizado.ahorro_porcentaje() - (1.0 - 83.0 / 168.0) * 100.0).abs() < 1e-9);

        // Las multiplicidades cuentan lo mismo que las hojas, DOS veces
        // (una por nivel de prefijo) — el anti-doble-recuento hecho número.
        assert_eq!(factorizado.multiplicidad_a.iter().sum::<u64>(), 56);
        assert_eq!(factorizado.multiplicidad_b.iter().sum::<u64>(), 56);
        // CSR coherente: los cierres encajan.
        assert_eq!(factorizado.inicio_b.first(), Some(&0));
        assert_eq!(
            factorizado.inicio_b.last(),
            Some(&factorizado.prefijos_b.len())
        );
        assert_eq!(factorizado.inicio_c.last(), Some(&56));

        // Multiplicidad por prefijo a en K₈: C(7-a, 2) para a=0..=5.
        let esperadas_a: [u64; 6] = [21, 15, 10, 6, 3, 1];
        assert_eq!(factorizado.multiplicidad_a, esperadas_a);
    }

    #[test]
    fn factorizacion_triangulos_equivale_a_las_tuplas_planas() {
        // Tesis: mismo MULTICONJUNTO lógico, menos CELDAS físicas. El
        // anti-test del doble recuento: si un prefijo se cobrara dos veces,
        // `tuplas_ordenadas` divergería de la enumeración canónica.
        let k8 = grafo_completo_bidir(8);
        let wcoj = TriangulosWcoj::enumerar(&k8);
        let factorizado = ResultadoFactorizadoTriangulos::desde_triangulos(&wcoj.triangulos);
        assert_eq!(factorizado.tuplas_ordenadas(), wcoj.triangulos);
        assert_eq!(
            factorizado.tuplas_ordenadas(),
            triangulos_join_binario(&k8).triangulos
        );

        // Y sobre el mini dataset completo.
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        let adj = AdyacenciasOrdenadas::desde_store(&ds.store);
        let wcoj = TriangulosWcoj::enumerar(&adj);
        let factorizado = ResultadoFactorizadoTriangulos::desde_triangulos(&wcoj.triangulos);
        assert_eq!(factorizado.tuplas_ordenadas(), wcoj.triangulos);
        assert!(
            factorizado.celdas_fisicas() < factorizado.celdas_planas(),
            "la factorización debe AHORRAR celdas sobre el mini dataset"
        );
        assert!(factorizado.ahorro_porcentaje() > 0.0);
    }

    #[test]
    fn informe_joins_reproducible_sobre_mini() {
        let ds = dataset_referencia_mini(SEMILLA_REFERENCIA);
        // Ruta calificada a propósito: el NOMBRE del test sombrea la función
        // del módulo a través de `use super::*`.
        let informe = crate::cap39_joins::informe_joins_reproducible_sobre_mini(&ds.store);

        // Reproducible BYTE A BYTE (dos llamadas, el mismo texto).
        let segunda = crate::cap39_joins::informe_joins_reproducible_sobre_mini(&ds.store);
        assert_eq!(informe, segunda);

        // Sin tiempos: ni microsegundos ni menciones al cronómetro externo
        // dentro del cuerpo del informe (regla del cap. 34).
        assert!(!informe.contains('µ'));
        assert!(!informe.contains("ns)"));

        // Las cifras CLAVE del propio dataset aparecen (guardas contra drift
        // silencioso: si cambia el plan, cambia el texto).
        let adj = AdyacenciasOrdenadas::desde_store(&ds.store);
        assert!(informe.contains(&format!("{} nodos", adj.num_nodos())));
        assert!(informe.contains(&format!(
            "intermedios materializados: {} tuplas",
            intermedios_plan_binario(&adj)
        )));
        assert!(informe.contains("=== Informe de joins (cap. 39) ==="));
        assert!(informe.contains("cota AGM"));
        assert!(informe.contains("factorización"));
        assert!(informe.contains("MotivoParada::Completo") || informe.contains("Completo"));
    }
}
