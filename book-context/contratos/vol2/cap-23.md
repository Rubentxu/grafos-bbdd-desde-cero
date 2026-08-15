# CONTRATO DE CAPÍTULO — Vol.II Cap. 23: A*, heurísticas y búsquedas dirigidas

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap23_a_estrella.rs` (~1.140 líneas,
> 14 tests en `tests_a_estrella` + 4 doctests; sin crates externas — `hypot` de
> std). Decisiones reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md`
> §28. ToC línea 38 de `manuscrito/vol2/tabla-de-contenidos.md`; pregunta crítica
> de CORPUS (`vol-II-cap-23`): «Heurísticas admisibles y consistentes».
> Colateral quirúrgico en `cap22_caminos_minimos.rs`: `PathStats` ganó
> `expanded` (también lo incrementa `dijkstra_impl` — hace posible la
> comparativa), y la sanidad eager pasó a `validate_edge_weights`/`ensure_node`/
> `table_len` pub(crate) compartidas (refactor puro).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: Dijkstra y Bellman-Ford del cap. 22 sobre
  `&dyn GraphStore` (`WeightSource`, `edge_weight` con semántica estricta,
  sanidad eager de pesos, `Path`, `PathStats`, `PathError`, el newtype `Cost`
  porque `f64` no puede `Ord`, finalización anticipada al destino por invariante
  codicioso); A* ALGORÍTMICO del Vol.I (cap. 9, sobre `Vec<Vec<…>>`; el cap. 29
  lo usaba en robótica y videojuegos); props de nodo y `Value` con promoción
  Int→Float (cap. 7); el store como puerto (cap. 8); «derivar, no llevar en la
  cabeza» y los errores tipados como estilo de la casa.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «una
  heurística mejor es la que da números más grandes» — no: sobre-estimar rompe
  la optimalidad EN SILENCIO (testeado: A* responde 3.0 donde el óptimo es 2.0,
  sin ningún error); (2) «A* siempre expande menos que Dijkstra» — falso: con
  h≡0 expande EXACTAMENTE lo mismo (mismo orden de pops, testeado) y con h
  admisible-inconsistente puede expander MÁS que nodos hay (5 expansiones para
  4 nodos, medido); (3) «la euclídea es siempre admisible» — sólo si pesos y
  coordenadas viven en las MISMAS unidades: km contra minutos la rompe (165
  real vs 200 devuelto, testeado); (4) «el algoritmo debería validar mi
  heurística» — no puede: verificar admisibilidad exige el coste real de cada
  nodo al destino, que ES un Dijkstra completo (verificar = resolver); se
  valida lo barato (finita, ≥ 0) y se diagnostica lo local (consistencia O(E));
  (5) «re-abrir nodos es un bug de implementación» — es el precio deliberado
  por tolerar heurísticas admisibles-inconsistentes sin perder optimalidad.
- **NO debe saber todavía**: PageRank/centralidad (cap. 24), proyecciones
  in-memory para algoritmos (cap. 26), ALT/landmarks como técnica de producción
  (se nombra en «BBDD real» y se siembra en el ejercicio experto), búsqueda
  bidireccional, heurísticas ε-óptimas ponderadas, heurísticas aprendidas. Se
  nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: el trait `Heuristic { estimate(&self, store, node) }` (la heurística
  como conocimiento del CALLER, ligada a un destino en construcción);
  `f(n) = g(n) + h(n)` como clave del heap; **admisibilidad** (h ≤ coste real;
  NO verificable sin resolver el problema) vs **consistencia**
  (h(u) ≤ w(u,v) + h(v); LOCAL, verificable en O(E) + ≤2V estimaciones);
  `ZeroHeuristic` (h≡0 ⇒ degeneración EXACTA en Dijkstra); `EuclideanHeuristic`
  (recta por props `x`/`y` de NODO con semántica estricta
  `MissingCoordinate`/`InvalidCoordinate` y `hypot`); re-apertura de nodos y
  detección de entradas obsoletas (`g_entrada > g[v]`); caché de estimaciones
  (≤1 consulta por nodo, NaN = «sin estimar»); `check_consistency` como
  diagnóstico (NO requisito); `PathStats::expanded` como métrica del ahorro.
- `practice`: `WeightSource`/`edge_weight`/`validate_edge_weights` (cap. 22, la
  misma función); `Path`/`PathError`/`Cost` (cap. 22); props de nodo y `Value`
  (cap. 7 — PRIMERA VEZ que la Parte V lee datos del NODO, no de la arista);
  `MemoryStore` y fixtures.
- `consolidate`: sanidad eager de lo barato + documentación de lo caro;
  errores tipados; `&dyn` para no contagiar genéricos; el store como puerto.
- `out_of_scope` (solo nombrar): ALT/landmarks en producción, búsqueda
  bidireccional, ε-óptima, PageRank (cap. 24), proyección (cap. 26).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) formula las dos garantías del capítulo — con h admisible
  (y ≥ 0, que sí se revisa) el primer pop vivo del destino tiene g óptimo;
  con h sobre-estimadora el camino es subóptimo SIN error; (2) demuestra por
  qué h≡0 hace degenerar la clave `(f, g, nodo)` en `(g, nodo)` y por qué eso
  lo convierte en TEST de equivalencia exacta (no esperanza); (3) explica por
  qué la admisibilidad no se verifica (verificarla = resolver Dijkstra hacia el
  destino) mientras que la consistencia sí es O(E) local; (4) calcula el
  coste de la inconsistencia en `expanded` (5 expansiones para 4 nodos, de los
  cuales 4 con h≡0); (5) dice qué compra `expanded` en las stats: hacer
  VISIBLE el ahorro de la heurística (13 vs 10; 7 vs 3).
- **Skills**: (1) implementar el trait en tres líneas para una heurística de
  tabla (`Fija`) y razonar sobre admisibilidad/consistencia del resultado;
  (2) construir `EuclideanHeuristic::new(&store, dest, "x", "y")` y predecir
  los errores tipados (Missing/InvalidCoordinate, Int promociona);
  (3) ejecutar `check_consistency` y leer `InconsistentHeuristic { edge,
  h_from, bound }` para encontrar la arista culpable.
- **Wisdom**: (1) decide cuándo NO usar A*: sin un destino o sin información
  que acote (tabla completa ⇒ Dijkstra del cap. 22; h≡0 no ahorra nada);
  (2) decide el trade-off exacto de «inflar h para acelerar»: sobre-estimar
  cambia la garantía de óptimo a «rápido y plausible» — y el fallo es
  silencioso, así que el diagnóstico (check_consistency) es parte del contrato
  de uso, no un extra.

## 4. Modelo mental

- **El radar omnidireccional vs el GPS**: Dijkstra es un radar que emite
  ondas circulares desde el origen y asienta por coste acumulado g — no sabe
  hacia dónde queda el destino, sólo cuánto ha caminado. A* es un GPS: cada
  nodo del mapa lleva una brújula h(n) («¿cuánto queda?»), y el heap ordena
  por la SUMA f = g + h — avanzar y acercarse puntúan juntos. La trampa del
  capítulo (3 nodos baratos que se ALEJAN 100 km del destino) es invisible al
  radar y evidente al GPS.
- **Diagramas ASCII**: (a) ondas concéntricas de Dijkstra vs el frente
  sesgado hacia el destino de A* sobre el grafo-trampa; (b) la tabla
  comparativa `expanded` (Dijkstra 13 / A* 10 en la trampa; 7 / 3 en la red de
  ciudades); (c) el tunnel-check de consistencia: `h(u) ≤ w(u,v) + h(v)`.
- **Momento ¡ajá!**: «h no cambia el grafo ni el algoritmo: cambia el ORDEN
  del heap — y el orden de pops era TODO lo que Dijkstra era». Cuando h≡0, la
  clave (f, g, nodo) se vuelve (g, nodo) y A* ES Dijkstra, pop a pop.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap23_a_estrella.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Heurística como **trait**, no closure | Las heurísticas son FAMILIAS CON ESTADO ligadas al destino (la euclídea recuerda destino y nombres de props; una de landmarks llevaría tablas precalculadas) — un struct con campos lo expresa natural; el contrato (finita, ≥ 0) se valida EN UN SITIO; `&dyn Heuristic` no contagia genéricos | Closure `Fn(NodeId) -> f64`: no puede leer el store (coordenadas) sin capturar el `&dyn GraphStore` prestado — y capturado, bloquea pasar otro store. Además escribe el estado del caller en la firma | Heurísticas incapaces de leer coordenadas, o APIs contaminadas con lifetimes | Trait `Heuristic` + doc-decisiones del módulo (líns. 57-68); `Fija` en tests demuestra el caso ad-hoc en 3 líneas |
| 2 | `ZeroHeuristic` y su test «es Dijkstra exactamente» (mismo orden de pops) | Con h≡0 la clave `(Cost(f), Cost(g), NodeId)` degenera en la de Dijkstra `(g, nodo)`: mismo camino, coste, pops, expanded y relax_updates. La igualdad EXACTA del orden de pops es el test de equivalencia más fuerte que existe — un A* mal cableado (f mal sumada, desempate distinto) lo delata | Test sólo de coste/camino: aceptaría una implementación que explora distinto y «acierta» | Regresiones invisibles: A* que degenera en otra cosa que no es Dijkstra | `heuristica_cero_es_dijkstra_exactamente` (popped/expanded/relax_updates iguales); doc del heap (líns. 393-398) |
| 3 | Admisibilidad DOCUMENTADA, no verificada | Verificar h(n) ≤ coste real de n al destino exige conocer ese coste real — que ES un Dijkstra completo hacia el destino: verificar = resolver el problema (más caro que la búsqueda misma). La euclídea la garantiza POR CONSTRUCCIÓN si pesos y coordenadas están en las mismas unidades (carretera ≥ línea recta) | `check_admissibility` eager: pagaría un Dijkstra por consulta de ruta | O la API es inutilizable (coste ≥ Dijkstra) o promete una garantía que no puede dar | Test `sobre_estimacion_devuelve_suboptimo_demostrando_el_riesgo` (3.0 vs 2.0, sin error); Hart-Nilsson-Raphael 1972 (la corrección que bautizó admisible/consistente) |
| 4 | `check_consistency` como utilidad de DIAGNÓSTICO, no requisito | La consistencia es LOCAL: una pasada O(E) + ≤2V estimaciones por arista `h(u) ≤ w(u,v) + h(v)`. A* NO la exige: con h admisible-inconsistente RE-ABRE nodos y sigue devolviendo el óptimo. Rechazarla sería rechazar respuestas correctas | Exigirla en `a_star`: bloquearía heurísticas válidas; o no ofrecerla: el usuario sin diagnóstico culpa al grafo | Inconsistencia sin detectar: A* «funciona» pero expande de más y nadie sabe por qué | `check_consistency` + test `admisible_no_consistente_reexpande_y_sigue_optimo` (delata la arista B→A: 1.9 > 1.0) |
| 5 | Re-apertura (`stats.expanded += 1` tras el filtro `g_u > g[u]`), sin `settled` | Con h inconsistente un nodo expandido puede MEJORAR su g después; marcarlo definitivo (como Dijkstra, cuya h≡0 es trivialmente consistente) PERDERÍA caminos óptimos. Re-expandir conserva la garantía de óptimo con h sólo admisible | Array `settled`: óptimo sólo con h consistente — y entonces la API mentiría sobre qué admite | Óptimos perdidos en silencio (el peor fallo posible) | Test re-expansión: expanded=5 para 4 nodos (A dos veces: g=4 y g=3.6), camino óptimo 4.6 intacto |
| 6 | Unidades mezcladas (km vs min) como bug didáctico ESTRELLA | h «válida en forma» (finita, ≥ 0, euclídea) pero «subóptima en hecho»: la recta en km no acota un coste en minutos — sobre-estima, y A* devuelve 200 donde el óptimo es 165, SIN error. Es la lección de que la admisibilidad es una propiedad CONJUNTA de h y de los pesos | Presentarlo como caso raro: es el error MÁS común al reutilizar la euclídea (coordenadas heredadas + pesos «mejorados» a tiempos) | Rutas «óptimas» sistemáticamente malas y ningún test rojo | Test `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad` (200 vs 165; `check_consistency` señala la arista 1→2: 139.28 > 67) |
| 7 | `PathStats::expanded` (pops VIVOS, sin obsoletos) añadido al struct del cap. 22 | Sin la métrica, el ahorro de la heurística es INVISIBLE: «A* es mejor» quedaría en fe. `expanded` (y también lo incrementa `dijkstra_impl`) hace comparables los dos algoritmos: 13 vs 10 en la trampa, 7 vs 3 en Madrid→Barcelona | Contar sólo `popped`: las entradas obsoletas del heap inflarían el número y ocultarían exactamente el efecto que importa (la re-apertura se VE: expanded > nodos) | Mejoras no medibles ⇒ no enseñables ni depurables | MIGRATION §28.5; tests con `assert_eq!(pd.stats.expanded, 13)` / `(pa.stats.expanded, 10)` / `7` / `3` |
| 8 | Euclídea con props `x`/`y` de NODO y semántica estricta (Missing/InvalidCoordinate; Int promociona; `hypot`) | Primera vez que la Parte V lee datos del NODO: `estimate` recibe el store por ESTO. Misma semántica estricta que `edge_weight` (cap. 22) — el grafo schemaless delata sus huecos cuando se pisan. `hypot` calcula la raíz sin desbordes intermedios y garantiza ≥ 0 finito | Coordenadas en una tabla aparte o `as f64` confiado: el NaN/missing explotaría dentro del heap (panic en `Cost::cmp`) | Panic en mitad de la búsqueda, o distancias basura silenciosas | `node_coord` (líns. 178-214); test `coordenadas_ausentes_invalidas_o_no_finitas` (Int 3/4 → 5.0, el triángulo 3-4-5) |
| 9 | Destino validado EAGER en `EuclideanHeuristic::new`, resto on-demand | Fallar ANTES de empezar (2 props del destino) es más barato que fallar en medio de la búsqueda; los demás nodos se delatan al pisarlos (misma filosofía que los pesos por arista del cap. 22) | Validar TODAS las coordenadas eager: O(V) lecturas para rutas que tocan 3 nodos | Coste proporcional al grafo, no a la ruta | Test: `new` hacia nodo sin `x` da `MissingCoordinate` inmediato; hacia nodo válido, el hueco del ORIGEN salta al estimar |
| 10 | Caché de h (≤ 1 estimación por nodo) + revisión finita/≥ 0 EN CADA valor usado | Un NaN rompe el orden total de `Cost` y haría PANIC dentro del heap (f64 no implementa `Ord` precisamente por esto); un h negativo casi siempre es bug del caller y rompe el criterio de parada. Sin caché, la euclídea reelería 2 props por CADA inserción en el heap | Confiar en el implementador: el panic llegaría dentro de la stdlib, el peor lugar para depurar | Panic en `BinaryHeap` o paradas anticipadas falsas | `h_of` (líns. 313-330); tests `heuristica_negativa_o_no_finita_es_rechazada` |
| 11 | Sólo punto-a-punto (sin variante single-source) | El sesgo hacia el destino ES la gracia; las distancias intermedias que A* va fijando NO están garantizadas (con h inconsistente, ni las de los nodos tocados). `dijkstra` del cap. 22 sigue siendo LA herramienta single-source | A*-tabla: entregaría g intermedios sin garantía — mentiras tipadas | Tabla de distancias incorrecta usada como verdad | Doc de `a_star` + MIGRATION §28.6 |
| 12 | f puede desbordar a ∞ (sólo es prioridad); g no (`CostOverflow`) | ±∞ no rompe el orden de `Cost`; el enemigo es NaN (ya revisado). Pero un g infinito NO puede llegar a `Path.cost`: el centinela INFINITY de «inalcanzable» no debe confundirse con un coste real | Validar f finita: rechazaría búsquedas legítimas en grafos de costes enormes | Ok(None) (inalcanzable) confundido con coste ∞ alcanzado | Doc de `a_star` (líns. 114-116); test `nodos_desconocidos_y_coste_que_desborda` (herencia cap. 22) |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: la del cap. 22 tal cual — `dijkstra_path(store, o, d, &w)`: óptima
  y ya con finalización anticipada. En el grafo-trampa expande 13 nodos (los 10
  de la cadena + los 3 de la trampa barata que se aleja); en la red de
  ciudades, las 7. Versión naïve de «dirigir la búsqueda»: ordenar el heap sólo
  por h (greedy best-first) — va directo al destino pero pierde la optimalidad
  (g no cuenta: es la sobre-estimación llevada al extremo).
- **Qué la rompe**: el radar no distingue dirección — los 0.5 de la trampa son
  irresistibles para quien sólo mira g; y el greedy no distingue coste —
  cualquier atajo caro «que acerca» le gana. Hace falta una clave que mezcle
  AMBAS informaciones.
- **Evolución visible**: `a_star(store, origin, dest, weight, &dyn Heuristic)`
  — misma maquinaria del cap. 22 (`WeightSource`, `validate_edge_weights`,
  `Path`, `PathStats`), heap por `f = g + h` con clave `Reverse<(Cost(f),
  Cost(g), NodeId)>`, re-apertura por `g_u > g[u]`, caché de h, y las dos
  heurísticas del capítulo (`ZeroHeuristic`, `EuclideanHeuristic`) + el
  diagnóstico `check_consistency`.

## 7. Prueba de fuego

- **Los números del workspace (todos testeados)**: grafo-trampa — Dijkstra
  `expanded=13` vs A* `expanded=10`, mismo coste 9.0; Madrid→Barcelona por
  Zaragoza — 7 vs 3, mismo coste 440.0; h≡0 — MISMO camino, coste, pops,
  expanded y relax_updates que `dijkstra_path`; h admisible-inconsistente —
  `expanded=5` para 4 nodos y camino óptimo 4.6 igualmente; h sobre-estimada —
  3.0 vs óptimo 2.0 SIN error (el camino devuelto es VÁLIDO, sólo subóptimo);
  unidades km/min — 200 vs 165 silencioso, `check_consistency` señala la
  arista 1→2 (`h_from` ≈ 139.28 > `bound` 67).
- **Tests citados**: `heuristica_cero_es_dijkstra_exactamente`,
  `heuristica_cero_coincide_en_destino_inalcanzable_y_trivia`,
  `euclidea_mismo_coste_con_menos_expansiones`,
  `hito_del_brief_rutas_sobre_una_red_de_ciudades`,
  `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad`,
  `admisible_no_consistente_reexpande_y_sigue_optimo`,
  `sobre_estimacion_devuelve_suboptimo_demostrando_el_riesgo`,
  `pesos_herencia_cap22_missing_invalid_negativo`,
  `coordenadas_ausentes_invalidas_o_no_finitas`,
  `heuristica_negativa_o_no_finita_es_rechazada`, `stats_coherentes_y_display_heredado`.
- **Síntoma si el lector se salta el capítulo**: sus rutas punto-a-punto
  escalan con todo el grafo (el círculo del radar), y si copia una heurística
  de StackOverflow con unidades distintas a sus pesos, obtendrá rutas malas
  SIN ningún síntoma — ni error, ni warning, ni test rojo. Sólo
  `check_consistency` (que no sabrá ejecutar) se lo diría.

## 8. Trampas y errores comunes

1. **Mezclar unidades** (coordenadas en km, pesos en min): h válida en forma,
   subóptima en hecho. Detección: `check_consistency` señala la primera
   arista donde `h(u) > w(u,v) + h(v)`.
2. **Inflar h «para acelerar»**: sobre-estimar convierte la garantía de
   óptimo en «rápido y plausible» — y el fallo es silencioso. Detección:
   comparar contra `dijkstra_path` en un test (como hacen los tests del
   capítulo).
3. **Confundir admisible con consistente** (o asumir que A* exige
   consistencia): admisible basta para el óptimo; la consistencia sólo
   ahorra re-expansiones. Detección: `expanded` mayor que el número de nodos
   tocados = hay re-apertura ⇒ correr `check_consistency`.
- **Precisión de lenguaje (glosario)**: *h* (estimación al destino) vs *g*
  (coste acumulado real) vs *f* (su suma, la prioridad); *admisible* (no
  sobre-estima: h ≤ coste real) vs *consistente* (local: h(u) ≤ w(u,v) +
  h(v) — la consistencia implica admisibilidad con h(dest)=0, no al revés);
  *pop* (salir del heap, puede ser obsoleto) vs *expandir* (pop vivo, se
  relajan aristas); *heurística* (dice el CALLER) vs *peso* (dice la arista);
  *settled* (definitivo, sólo con consistencia) vs *re-abrir* (volver a
  expandir tras mejorar g).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval puro)**: sin mirar código ni
  apuntes, responder: (a) ¿qué degenera exactamente en la clave del heap
  cuando h≡0, y qué TRES contadores del test de equivalencia deben coincidir
  con Dijkstra para que la igualdad sea exacta (no «parecida»)? (b) ¿cuál de
  las dos propiedades — admisibilidad o consistencia — puede verificar LiraDB
  en O(E), y por qué la otra no? Verificación: test
  `heuristica_cero_es_dijkstra_exactamente` + `check_consistency` + doctests.
  Pistas graduadas: (1) f = g + 0, ¿qué le pasa a la primera componente de la
  tupla?; (2) ¿qué mide `popped` que no mide `expanded`?; (3) ¿qué
  información completa exige saber el coste real hasta el destino? Criterio:
  nombrar (f,g,nodo)→(g,nodo), popped/expanded/relax_updates, y la asimetría
  global-vs-local.
- **analizar (intermedio)**: el túnel del doctest — A(0,0), B(3,4), carretera
  de 6 km y un TÚNEL de 4 km (¡más corto que la línea recta de 5!).
  Predecir: ¿es consistente la euclídea? ¿Y admisible? ¿Qué puede devolver
  A* de A a B si el túnel está en el camino óptimo — camino válido, coste
  correcto, o silencio? Construir el store y comprobarlo con
  `a_star` + `dijkstra_path` + `check_consistency`. Pistas: (1) la recta es
  5; (2) la desigualdad triangular se rompe por el túnel; (3) ¿qué garantiza
  admisible y qué garantiza consistente? Criterio: distinguir la propiedad
  global (rota por el túnel) de la local (rota en la arista del túnel) y
  explicar qué riesgo queda.
- **crear (experto — spacing + interleaving)**: implementar
  `LandmarkHeuristic`: elegir un nodo hito L, precalcular con el `dijkstra`
  del cap. 22 (single-source, `ShortestPaths::distance`) la tabla d(L,·), y
  definir h(n) = |d(L,dest) − d(L,n)| (la desigualdad triangular la hace
  admisible; ejecutar `check_consistency` y discutir el resultado). Correr
  Madrid→Barcelona y comparar `expanded` contra `ZeroHeuristic` (7) y contra
  la euclídea (3). Pistas: (1) `Fija` en los tests es la plantilla: el trait
  se implementa en tres líneas; (2) la tabla vive en el struct — por eso el
  trait y no la closure; (3) ¿qué hito da mejor cota: Sevilla o Zaragoza?
  Criterio: camino 440.0 idéntico, `expanded` intermedio (o mejor),
  `check_consistency` interpretado.

## 10. Preguntas abiertas (gancho al capítulo 24)

1. A* necesita UN destino para saber hacia dónde ir… ¿y cuando la pregunta
   es «¿quiénes son los nodos IMPORTANTES?» — quién es el «destino» del
   PageRank? (Nace la centralidad: puntuar TODOS los nodos a la vez.)
2. Nuestra heurística leyó 2 props por nodo estimado; ¿qué pasa cuando el
   grafo no cabe en memoria y las coordenadas viven en páginas del buffer
   pool? (Proyección del cap. 26.)
3. La re-apertura expandió 5 veces para 4 nodos: ¿existe una heurística que
   NUNCA reabra? (Consistencia — y cómo conseguirla sin coordenadas: ALT.)
- **Términos nuevos de glosario**: heurística, admisible, consistente,
  f/g/h, búsqueda dirigida (informed search), re-apertura (re-expansión),
  entrada obsoleta del heap, heurística euclídea, landmark (hito), A*.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial obliga a RECONSTRUIR la degeneración de
  la clave del heap y los tres contadores del test de equivalencia desde la
  memoria (el enunciado no los regala); nada de reconocer opciones.
- **Spacing**: cap. 22 (la maquinaria heredada: `WeightSource`, `PathStats`,
  finalización anticipada — el experto REUSA `dijkstra` single-source); cap. 7
  (props de nodo, `Value`, promoción Int→Float: la primera lectura de datos
  de NODO de la Parte V); Vol.I caps. 9 y 29 (A* algorítmico y Shakey).
- **Interleaving**: el experto mezcla Dijkstra-tablas (cap. 22), el trait
  nuevo, la desigualdad triangular (¿matemáticas del Vol.I?) y la métrica
  `expanded`; el intermedio cruza pesos del cap. 22 con la geometría de h.
- **Dificultad asimétrica**: una idea nueva por sección (ordenar por f →
  trait → degeneración → euclídea → re-apertura → validación honesta →
  unidades); los ejercicios exigen predecir y reconstruir, no leer y asentir.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` (14 tests
  citados por nombre) y los doctests de `a_star`/`EuclideanHeuristic`.
- **Citas**: Hart, Nilsson y Raphael, «A Formal Basis for the Heuristic
  Determination of Minimum Cost Paths», IEEE Trans. SSC-4(2), 1968 (el paper
  de A*, nacido del robot Shakey del SRI); Hart, Nilsson y Raphael, corrección
  en SIGART Newsletter 37, 1972 (nace la distinción admisible/consistente);
  Goldberg y Harrelson, «Computing the Shortest Path: A* Search Meets Graph
  Theory», SODA 2005 (ALT/landmarks); docs de Neo4j GDS (A* con latitud/
  longitud), de pgRouting (`pgr_aStar`: heuristic/factor/epsilon) y de
  GraphHopper (`landmarks.md`, ALT); Dechter y Pearl, JACM 1985 (optimal
  efficiency con h consistente).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible, no happy path: sobre-estimación y unidades mezcladas ⇒ subóptimo EN SILENCIO (tests con números 3.0-vs-2.0 y 200-vs-165).
- [x] Código ejecutable en workspace (14 tests + 4 doctests) citado por nombre; la prosa lo referencia sin duplicarlo.
- [x] ≥1 misconcepción corregida explícitamente (5 en §1, desarrolladas en el capítulo).
- [x] Ejercicios con solución verificable (tests del workspace + comparativa contra `dijkstra_path`).
- [x] ≥1 ejercicio de retrieval (degeneración de la clave del heap + contadores, desde memoria) y ≥1 de spacing (cap. 22 single-source; cap. 7 props de nodo).
- [x] Responde la pregunta crítica de CORPUS («Heurísticas admisibles y consistentes»): qué son, cuál se verifica, cuál se documenta, cuál se diagnostica y qué pasa si se rompen.
- [x] Anécdota verificada: Hart-Nilsson-Raphael 1968, SRI, Shakey (PDP-10/PDP-15 por radio; fuentes IEEE Xplore, Wikipedia, Wired 2013, MIT CSAIL).
- [x] Spacing explícito con cap. 22 (misma maquinaria heredada, `expanded` compartido) y cap. 7 (coordenadas en props de NODO — primera vez que la Parte V lee datos de nodo).
