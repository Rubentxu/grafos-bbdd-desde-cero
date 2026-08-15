# CONTRATO DE CAPÍTULO — Vol.II Cap. 26: Ejecutar algoritmos sin agotar la memoria (proyección, streaming, frontiers)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap26_proyeccion.rs` (2.212 líneas,
> 27 tests en `tests_proyeccion` + 5 doctests, verificados ALL_GREEN
> `cargo test -p vol2-liradb --lib cap26` → 27 passed). Decisiones reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §31 (incluye la
> historia de la migración: agente interrumpido por usage-limit con el módulo
> completo-sin-cablear y 4 tests recalibrados a mano). Este capítulo CIERRA
> la Parte V: repaso-árc 22→23→24→25→26 (como el cap. 21 hizo con la IV) y
> cubre la línea 41 del ToC (`manuscrito/vol2/tabla-de-contenidos.md`),
> "proyección, streaming, frontiers". Ganchos: cap. 27 (Parte VI, ACID) y
> Vol.III cap. 51 (GraphRAG).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: Dijkstra/Bellman-Ford leyendo el store arista
  a arista con `WeightSource`/`edge_weight` y semántica ESTRICTA de pesos
  (MissingWeight/InvalidWeight/NonFiniteWeight, política eager anti-negativos,
  cap. 22); A* con heurísticas de NODO (cap. 23); las cinco familias de
  centralidad/PageRank/PPR sobre una proyección PRIVADA no ponderada con
  índice denso y `GraphDirection` (cap. 24); Louvain sobre un
  `GrafoPonderado` simétrico propio (cap. 25); el CSR PERSISTENTE del cap. 14
  (sólo topología: offsets + targets, SIN ids de arista ni pesos) y por qué
  el cap. 22 no podía usarlo para pesar; el precio de cada lectura que falla
  la caché (BufferPool, cap. 13); el store como puerto `&dyn GraphStore`
  (cap. 8); iteradores de Rust y `BinaryHeap<Reverse<...>>` (caps. 22-24).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «para
  ejecutar un algoritmo hay que cargar el grafo entero en memoria» — no:
  depende de la CONSULTA (K iteraciones sobre todo → materializar; ¿qué hay
  a 2 saltos? → streaming); (2) «filtrar el grafo es descartar aristas al
  leerlas» — no aquí: con la proyección por ADYACENCIAS, las aristas de
  nodos excluidos NI SE LEEN (medible en `descartadas`/`edges_scanned`);
  (3) «un Vec de niveles y un iterador de fronteras son lo mismo» — no: el
  Vec materializa TODO antes de empezar; el iterador es perezoso (la frontera
  k+1 no existe hasta que la pides, y puedes soltarlo a mitad); (4) «si el
  BFS visitó N nodos, esa es la respuesta» — incompleto: sin
  `MotivoParada` no sabes si N es «todos los que había» o «todos los que
  cabían en el presupuesto»; (5) «las stats del algoritmo bastan para creer
  que no leyó de más» — no: el auto-informe se verifica con una fuente
  EXTERNA (`ContandoStore`); (6) «HashSet es siempre la estructura para
  conjuntos» — no: con ids densos un bitset es 1/8 del espacio y sin
  hashing; con claves dispersas/arbitrarias gana el HashSet.
- **NO debe saber todavía**: paralelismo real con hilos/rayon (`&dyn
  GraphStore` no es `Sync`; queda DOCUMENTADO en `bloques_de_nodos`), MVCC y
  aislamiento transaccional de snapshots (cap. 30), out-of-core con
  temporary buffers estilo DuckDB, grafos distribuidos (Giraph/GraphX en
  producción), compresión de columnas, GraphRAG (Vol.III cap. 51). Se
  nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: proyección MATERIALIZADA pública con pesos (`ProyeccionPonderada`:
  CSR en memoria completando el del cap. 14 con pesos + ids de arista, índice
  denso `Option<usize>` que compacta huecos, determinismo por orden
  (destino, id de arista)); `FiltroProyeccion` por etiqueta de nodo / tipo de
  arista (arista hacia nodo excluido se descarta; arista DESDE nodo excluido
  ni se lee); `ProyeccionStats` (nodos/aristas/edges_scanned/descartadas =
  el precio medido); streaming por FRONTERAS (`FronterasBfs: Iterator`
  perezoso, `bfs_streaming` de una tirada, `StreamStats`);
  `Presupuesto{max_profundidad,max_nodos,max_lecturas}` comprobado ANTES de
  cada lectura/descubrimiento (exacto, nunca superado); `MotivoParada`
  (Completo/ProfundidadMaxima/PresupuestoNodos/PresupuestoLecturas) como
  parte de la RESPUESTA; `ContandoStore` (voltímetro: cuenta
  get_edge/get_node/out_edges/in_edges con `Cell`, verificación EXTERNA);
  `BitSet` a mano (Vec<u64>, 1 bit/id) vs HashSet disperso; `bloques_de_nodos`
  (procesamiento por bloques, semilla del paralelismo); OLTP vs analítica
  encarnada (snapshot inmutable vs store vivo).
- `practice`: `WeightSource`/`edge_weight` estrictos (cap. 22); BFS/Dijkstra
  (Vol.I caps. 4/9; cap. 22); `GraphDirection` Out/In/Both (cap. 24);
  corrección Wasserman-Faust del closeness (cap. 24); CSR offsets/targets
  (cap. 14); iteración y borrado perezoso de heap (cap. 22).
- `consolidate`: «derivar, no llevar en cabeza» (stats derivadas del
  recorrido); política de fallo ruidoso (negativos → error tipado); el store
  como puerto al que se pueden ENVOLVER instrumentos (decorador).
- `out_of_scope` (solo nombrar): hilos/rayon, MVCC (cap. 30), out-of-core
  completo, sistemas distribuidos, GraphRAG (Vol.III cap. 51).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué DOS estrategias y no una — el TIPO de
  consulta decide (K iteraciones sobre todo el grafo → materializar una vez;
  consulta local a k saltos → streaming frontera a frontera); (2) calcula el
  precio de cada vía con números (proyección = E lecturas UNA vez; K
  Dijkstras del cap. 22 sobre el store = Σ(E−i) — en la cadena de 12 con 5
  orígenes: 45 vs 11); (3) enuncia el contrato del presupuesto (exacto:
  comprobado antes de cada lectura y descubrimiento; nunca se supera) y por
  qué `MotivoParada` cambia la INTERPRETACIÓN del resultado; (4) dice qué
  hereda la proyección del CSR del cap. 14 (layout) y qué le añade (pesos +
  ids de arista) y por qué el persistente no podía pesar; (5) distingue
  denso (BitSet) de disperso (HashSet) y cuándo gana cada uno.
- **Skills**: (1) envolver cualquier store en `ContandoStore` y VERIFICAR
  externamente las stats de una proyección o un BFS; (2) predecir
  `niveles/stats/parada` de un BFS con presupuesto ANTES de ejecutarlo y
  comprobarlo con el voltímetro; (3) construir subgrafos con
  `FiltroProyeccion` (labels/tipos, builder) y leer sus stats (qué se leyó,
  qué se descartó, qué ni se leyó).
- **Wisdom**: (1) decide entre materializar y streamear según la consulta
  (¿cuántas iteraciones? ¿qué fracción del grafo toca?) y sabe el modo de
  fallo de elegir mal (OOM/lecturas tiradas); (2) decide cuándo NO creer un
  auto-informe y exigir una segunda fuente de medida independiente.

## 4. Modelo mental

- **La biblioteca que FOTOCOPIA una sección del archivo vs el ARCHIVADOR que
  va hoja a hoja**. Proyección: pagas UNA fotocopia completa de la sección
  que te interesa (filtro = la sección; E lecturas) y luego consultas tu
  copia mil veces sin volver al archivo — que mientras tanto sigue
  recibiendo documentos (tu copia es una FOTO: snapshot, OLTP vs analítica).
  Streaming: el archivador te trae carpeta a carpeta (frontera a frontera);
  cuando ya tienes lo que buscabas, le dices basta — nunca pidió lo que no
  necesitabas; el `Presupuesto` es el número máximo de carpetas que le
  autorizas, y `MotivoParada` es su nota final («te traje todo lo que
  había» vs «me quedé sin permiso»). El `ContandoStore` es el contador de la
  puerta: no le preguntas al archivador cuánto trabajó — miras el contador.
- **Diagramas ASCII**: (a) las dos vías lado a lado (store → proyección CSR
  → K iteraciones vs store → frontera k → frontera k+1 bajo demanda); (b)
  layout CSR de `ProyeccionPonderada` (offsets/targets/pesos/aristas) frente
  al CSR persistente del cap. 14; (c) el árc de la Parte V 22→26 con la
  garantía que deja cada capítulo.
- **Momento ¡ajá!**: «"¿qué hay a 2 saltos de Ana?" no necesita el grafo:
  necesita DOS adyacencias. Y "Dijkstra desde todos los nodos" no necesita
  releer el grafo V veces: necesita UNA fotocopia. La memoria no se gestiona
  con swap: se gestiona decidiendo QUÉ existe y CUÁNDO».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap26_proyeccion.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | DOS estrategias (materializar vs streamear) y no una | El TIPO de consulta decide: K iteraciones sobre todo el grafo amortiza E lecturas; una consulta local a k saltos usa ~k adyacencias de E. Materializar siempre tira memoria; streamear siempre multiplica lecturas por K | Una sola vía universal: K Dijkstras sobre el store pagan Σ(E−i)=45 vs 11; BFS a profundidad 2 sobre materialización paga 499 aristas para leer 2 | OOM en un caso; K·E lecturas en el otro | `economia_multiorigen_una_lectura_por_adelantado` (líns. 1731-1763) vs `bfs_streaming_no_lee_todo_el_grafo` (1887-1920); banner 35-66 |
| 2 | Proyección = CSR del cap. 14 COMPLETADO (pesos + EdgeIds) | El CSR persistente del 14 sólo vive topología (sin ids/pesos): por eso Dijkstra no podía usarlo (cap. 22). El mismo layout en memoria añade lo que falta y hereda la localidad | Hash de adyacencias: más lento de recorrer (sin orden, sin caché de rangos); re-leer props por consulta: E lecturas × K | Pesos inconsistentes entre iteraciones; re-lecturas | Doc de `ProyeccionPonderada` (líns. 338-372); MIGRATION-PATTERN §27 (deuda del 22) |
| 3 | Pesos resueltos UNA vez con `edge_weight` del cap. 22 (semántica ESTRICTA heredada) | Un solo contrato de calidad de dato en toda la Parte V; la sanidad (validar tipos/NaN/negativos) se paga O(E) una vez, no por consulta. Test-economía: 45 (store, K validaciones+relajaciones) vs 11 (proyección) | Relajar la semántica «para la analítica»: la BD contestaría casi-bien (política contraria al cap. 22) | Datos podridos aceptados en el path analítico y rechazados en el OLTP — dos verdades | `proyeccion_hereda_los_errores_estrictos_de_pesos_del_cap22` (1434-1481); `From<PathError>` (170-174) |
| 4 | Validación de negativos EAGER sobre TODA la proyección (en `dijkstra_proyeccion`), pero UNA sola vez en `closeness_ponderado` | Misma política ruidosa del cap. 22 (una BD prefiere fallar a contestar casi-bien) — pero quien llama muchas veces valida UNA vez y usa `dijkstra_interna` (núcleo sin validar): el trato de la analítica | Validar por arista relajada: coste repartido pero semántica inconsistente con el cap. 22 | V validaciones O(E) redundantes: V·E trabajo tirado | `validar_pesos_no_negativos` (673-685); `dijkstra_interna` (689-741); `closeness_ponderado` (762-787, doc 757-761) |
| 5 | Filtrar por ADYACENCIAS de nodos admitidos: las aristas de nodos excluidos NI SE LEEN | El bucle de `proyectar` itera `out_edges` SÓLO de nodos que pasan el filtro: la adyacencia del excluido jamás se consulta. El ahorro del subgrafo es real y MEDIBLE (edges_scanned/descartadas) | Leer todas las aristas y descartar por extremos: pagas E lecturas para proyectar un subgrafo pequeño | El subgrafo cuesta como el grafo | `proyectar` (428-450); `subgrafo_filtrado_por_label_y_tipo_de_arista` (1484-1530, la arista 4→0 no aparece en descartadas) |
| 6 | `FronterasBfs` es un ITERATOR perezoso, no un Vec de niveles | Streaming de verdad: la frontera k+1 no existe hasta que la pides (no se lee su adyacencia ANTES de expandirla); quien consume decide parar (pedir 2 fronteras y soltar = 1 lectura). Vec de niveles = materializar TODO primero, justo lo que el capítulo evita | `Vec<Vec<NodeId>>` calculado de golpe: pagas el grafo alcanzable aunque sólo quieras 2 niveles | Consulta local que cuesta como global | `bfs_iterador_perezoso_una_frontera` (2020-2046); `next` (1017-1053) |
| 7 | `MotivoParada` es PARTE DE LA RESPUESTA (no un log) | Saber que cortó un presupuesto cambia lo que el resultado SIGNIFICA: 3 nodos visitados con `Completo` = la componente entera; con `PresupuestoNodos` = «había más, no te lo puedo decir». Interpretarlos igual es mentir con estadística | Devolver sólo niveles: el consumidor inventa la interpretación | Decisión de negocio tomada sobre un recorte creído completo | `MotivoParada` (876-886); `bfs_streaming_presupuesto_nodos_exacto` (1949-1990: aislado con presupuesto 1 → Completo, no corte) |
| 8 | Presupuesto comprobado ANTES de cada `get_edge` y de cada descubrimiento (exacto) | «max_lecturas=N» promete que NO SE SUPERA (acotar lecturas = acotar tiempo Y memoria de trabajo); comprobar después permite desbordar de 1 | Comprobación a posteriori por frontera: el límite se supera dentro de la frontera | Presupuesto mentiroso: OOM justo donde se quería evitar | `expandir` (977-1007: check antes de `get_edge` y antes de `marcar`); `bfs_streaming_presupuesto_lecturas_exacto` (1993-2017) |
| 9 | `ContandoStore` EXTERNO (wrapper con `Cell`), no stats internas ampliadas | No confíes en que el algoritmo se auto-auditore: dos fuentes INDEPENDIENTES que deben coincidir (la stats interna y el contador de la puerta). Patrón decorador sobre `&dyn GraphStore` | Creer las stats internas: un bug de contabilidad interna se auto-oculta | Tests que verifican el auto-informe consigo mismo | `ContandoStore` (1151-1263); tésis 1887-1920 y economía 1731-1763 contrastan stats vs voltímetro; MIGRATION §31 lección 3 |
| 10 | `BitSet` a mano (Vec<u64>) para visitados; `HashSet<String>` en el filtro | Denso vs disperso: ids del store nacen densos (cap. 7) → 1 bit/id = 1/8 del espacio de HashSet<usize> y sin hashing/rehash, con el patrón «¿ya visité este vecino?» O(1) puro. Las claves del filtro son STRINGS arbitrarios (dispersos): un bitset no puede indexarlos sin diccionario | HashSet para visitados: 8× espacio y hashing por arista examinada; bitset para labels: imposible sin diccionario previo | Memoria ∝ visitados × 8 y CPU de hashing en el bucle caliente | `BitSet` (176-231, doc 181-187); `FiltroProyeccion` (260-264); `bitset_marca_consulta_y_cuenta` (1309-1330) |
| 11 | Convivencia con las proyecciones privadas de 24/25 (no unificar) | Cada una codifica el contrato de SU familia (24: unión como conjunto, no ponderada; 25: simétrica sumando, reconstructible por nivel). Unificar a la fuerza = re-asegurar 597 tests por cero valor pedagógico; la pública es la API que la Parte V esperaba | Refactor unificador: riesgo de regresión masiva y borrón pedagógico | Regresiones en Louvain/PageRank ya verificados | Banner 87-99 (decisión documentada); MIGRATION §31 decisión implícita; tests caps 24/25 intactos |
| 12 | Paralelismo DOCUMENTADO (`bloques_de_nodos`), no implementado | Los slices CSR son perfectamente divisibles entre hilos (cada bloque independiente), pero `&dyn GraphStore` no es `Sync` y el workspace no usa crates (nada de rayon). La SEMILLA queda y el cómo (GDS, Kùzu) es prosa | rayon ahora: rompe la política de dependencias y `Sync` del puerto | Promesa de paralelismo sin poder cumplirla | `bloques_de_nodos` (532-547); `bloques_de_nodos_reparto_y_ultimo_parcial` (1566-1585); banner 73-78 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: la que ya funcionaba — cada algoritmo lee el store arista a
  arista (`out_edges` + `get_edge` por relajación), como los caps. 22-23.
  Perfecta para UNA consulta; el lector no ve el problema hasta que la Parte
  V la reutiliza K veces (closeness = V Dijkstras, PageRank = iteraciones,
  Louvain = niveles).
- **Qué la rompe**: (a) K iteraciones → K·E lecturas y K validaciones de
  pesos (cadena de 12, 5 orígenes: 45 lecturas vs 11 de la proyección);
  (b) la consulta local simétrica: «¿qué hay a 2 saltos?» sobre un grafo de
  499 aristas leyéndolas todas para usar 2; (c) el CSR persistente del 14 no
  puede pesar (sólo topología) — la deuda explícita del cap. 22.
- **Evolución visible**: `ProyeccionPonderada::proyectar(store, &WeightSource,
  &FiltroProyeccion)` materializa UNA vez (E lecturas, pesos resueltos,
  huecos compactados) y `dijkstra_proyeccion`/`closeness_ponderado` iteran
  sobre slices con CERO lecturas del store (voltímetro en mano); en el otro
  extremo, `bfs_fronteras(...)` devuelve un `Iterator` bajo `Presupuesto`
  con `MotivoParada`. Los tests miden AMBAS mitades del título.

## 7. Prueba de fuego

- **TEST-TESIS** `bfs_streaming_no_lee_todo_el_grafo`: cadena de 500 nodos
  (499 aristas); BFS profundidad 2 desde el 0 → 3 nodos visitados, **2
  aristas leídas de 499** (<0,5%), 2 consultas de adyacencia, y el
  VOLTÍMETRO confirma 2 (stats == contador externo). El resto del grafo no
  existió para la consulta.
- **TEST-ECONOMÍA** `economia_multiorigen_una_lectura_por_adelantado`:
  materializar (11 = E lecturas) + 5 Dijkstras = SIGUEN siendo 11; 5
  Dijkstras directos del cap. 22 = 45 (Σ(E−i) = 11+10+9+8+7). La brecha
  crece con cada origen.
- **Deudas saldadas**: `dijkstra_proyeccion_coincide_con_dijkstra_store`
  (distancias Y caminos Y arista elegida idénticos store vs proyección,
  con paralelas y self-loops) y `closeness_ponderado_paga_la_deuda_del_cap24`
  (3/14, 4/33, 1/3, 0 a mano; consenso Constant(1.0) ≡ saltos del cap. 24;
  V Dijkstras sin UNA lectura del store tras materializar).
- **Síntoma si el lector se salta el capítulo**: sus analíticas releen el
  grafo por iteración (K·E lecturas, latencia que crece con K), sus consultas
  locales pagan el grafo entero y no puede DEMOSTRAR ninguna de las dos
  cosas porque no tiene voltímetro; y el cap. 51 del Vol.III (GraphRAG,
  PPR multi-hop) heredaría un motor sin piernas.

## 8. Trampas y errores comunes

1. **Confundir `Completo` con «ya está»**: 3 nodos visitados puede ser la
   componente entera O el presupuesto agotado — mirar `parada` antes de
   interpretar. Síntoma: decisiones sobre recortes creídos completos.
2. **Calibrar contadores «a lo que suena razonable»**: profundidad k ⇒
   EXPANDIR k nodos (no visitar k+1): con profundidad 2 se leen 2 aristas,
   no 3. Los tests de contadores se calibran TRAZANDO el código a mano
   (lección de §31 de MIGRATION: 4 tests mal calibrados).
3. **Creer el auto-informe**: las stats internas las escribe el mismo código
   que presume; el voltímetro es la segunda fuente. Si discrepan, el bug es
   de la stats (o del contador), pero lo SABES.
- **Precisión de lenguaje (glosario)**: *proyección* (copia materializada
  de un subgrafo) vs *snapshot* (la foto inmutable resultante — su propiedad
  temporal); *materializar* vs *streamear*; *frontera* (nivel k de un BFS)
  vs *componente* (todo lo alcanzable); *presupuesto* (límites declarados)
  vs *parada* (quién cortó de verdad); *lectura* (get_edge en el voltímetro)
  vs *consulta de adyacencia* (out_edges/in_edges); *denso* (ids contiguos)
  vs *disperso* (ids arbitrarios); *OLTP* (puntual, sobre el store vivo) vs
  *analítica* (iterativa, sobre la foto).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: sobre la estrella 0→{1,2,3,4} con
  `Presupuesto::sin_limite().con_nodos(3)`, predecir ANTES de ejecutar
  `niveles`, `nodos()`, las CUATRO stats y el `MotivoParada`; luego verificar
  con `bfs_streaming` envuelto en `ContandoStore` (las lecturas del
  voltímetro deben cuadrar con las stats). Pistas: (1) ¿el presupuesto se
  comprueba antes o después de descubrir?; (2) ¿cuál es la PRIMERA stats en
  tocar el límite?; (3) ¿y si el origen estuviera aislado? Criterio: predicción
  exacta de niveles + parada + coincidencia stats/voltímetro.
- **analizar (intermedio — spacing caps. 22+24)**: en la cadena 0→1→2→3 con
  pesos 1, 5, 1, calcular A MANO el closeness ponderado (Wasserman-Faust con
  Σd ponderado) de los cuatro nodos y explicar por qué el 0 y el 1 se
  devalúan respecto a la versión por saltos y el 2 ni se entera; decir qué
  pasaría con pesos negativos y por qué el rechazo es eager. Verificación:
  `closeness_ponderado_paga_la_deuda_del_cap24`. Pistas: (1) ¿qué suma d
  para cada origen?; (2) ¿qué aristas toca el mundo alcanzable del 2?;
  (3) ¿quién valida y CUÁNTAS veces en `closeness_ponderado`? Criterio:
  números exactos (3/14, 4/33, 1/3, 0) + la diferencia semántica
  salto/peso.
- **crear (experto — cierre de Parte V, retrieval puro)**: reconstruir DESDE
  LA MEMORIA el árc 22→26 (qué añadió cada capítulo y qué garantía dejó) sin
  mirar los banners; luego implementar `pagerank_proyeccion(&ProyeccionPonderada)`
  — damping, masa dangling redistribuida uniformemente, convergencia L1, sin
  UNA lectura del store tras materializar (voltímetro) — y verificar que sus
  scores coinciden con el `pagerank` del cap. 24 sobre el mismo grafo
  simetrizado (equivalencia, como hizo el cap. 21). Pistas: (1) ¿qué
  convención de self-loop/paralelas exige re-declarar aquí?; (2) ¿dónde del
  bucle por niveles está la masa que se escapa?; (3) ¿por qué la validación
  de pesos corre UNA vez fuera del bucle de orígenes? Criterio: árc completo
  de memoria + pagerank equivalente al cap. 24 + 0 lecturas medidas.

## 10. Preguntas abiertas (gancho al cap. 27 — abre la Parte VI; y al Vol.III)

1. La proyección es una FOTO coherente… ¿coherente con QUÉ instante? Si el
   store muta mientras se fotografia, ¿qué garantiza que la foto no mezcló
   dos mundos? (Nace la Parte VI: transacciones, ACID — cap. 27.)
2. Dos analíticas corriendo sobre la MISMA foto podrían compartir trabajo:
   ¿quién decide qué materializar y cuándo invalidarlo? (Catálogo de
   proyecciones: lo que GDS llama graph catalog.)
3. Cuando el grafo son millones de documentos y la pregunta es «recupera
   lo relevante a esta consulta» (no un patrón exacto), ¿cómo se combina
   recuperación vectorial con recorrido multi-hop ponderado? (Vol.III
   cap. 51, GraphRAG: el PPR del cap. 24 sobre las piernas de éste.)
- **Términos nuevos de glosario**: proyección, materializar, streaming,
  frontera, presupuesto, motivo de parada, snapshot, OLTP vs analítica,
  bitset, procesamiento por bloques, voltímetro (instrumento de medida
  externo), subgrafo inducido por filtro, superpaso (BSP, nombrado en la
  anécdota).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto reconstruye el árc de la Parte V DESDE
  LA MEMORIA (nada del enunciado revela qué aportó cada cap); el esencial
  predice stats y parada sin ejecutar (recordar el orden de comprobación
  del presupuesto).
- **Spacing**: cap. 14 (CSR persistente: la proyección ES su layout
  completado), cap. 22 (WeightSource estricto y política eager — heredada
  tal cual), cap. 24 (Wasserman-Faust, GraphDirection, índice denso — la
  Proyección privada que aquí se hace pública), cap. 25 (GrafoPonderado y
  por qué CONVIVE), cap. 13 (el precio de una lectura que falla la caché:
  por qué comprarlas por adelantado); la sección de repaso re-ejercita la
  cadena 22→26 completa.
- **Interleaving**: el intermedio mezcla pesos estrictos (22) con
  centralidad (24) y presupuesto (26); el experto mezcla PageRank (24),
  proyección (26) y equivalencia de resultados (21).
- **Dificultad asimétrica**: una idea nueva por sección (materializar →
  filtrar sin leer → streamear → presupuestar → medir desde fuera →
  repasar el árc); los ejercicios exigen predicción y reconstrucción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap26` (27
  tests citados por nombre; los del lector compilan contra el mismo módulo).
- **Citas**: Malewicz et al., «Pregel: a system for large-scale graph
  processing», SIGMOD 2010 (pp. 135-146, DOI 10.1145/1807167.1807184 —
  «think like a vertex», BSP); Valiant, «A bridging model for parallel
  computation», CACM 33(8), 1990 (BSP); Kyrola, «GraphChi», OSDI 2012
  (PSW, un PC, mil millones de aristas); Xin et al., «GraphX», OSDI 2014;
  Neo4j GDS docs (native projection / gds.graph.project, memory
  estimation); Kùzu CIDR 2023 + Gupta et al. VLDB 2021 (columnar);
  DuckDB «Memory Management» (2024, out-of-core); McCune et al., ACM
  Computing Surveys 2015 (survey TLAV).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: K·E lecturas medidas (45 vs 11) y consulta local que paga el grafo entero (2 de 499) + presupuesto superado si se comprueba tarde.
- [x] Código ejecutable en workspace (27 tests ALL_GREEN, verificados) citado por nombre y línea, no duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: seis; «filtrar no es leer-y-tirar», «Vec ≠ Iterator», «Completo ≠ corté», «no creas el auto-informe», «HashSet no siempre», «cargar todo no es la única vía»).
- [x] Ejercicios con solución verificable (tests del workspace + predicciones medibles con voltímetro).
- [x] ≥1 ejercicio de retrieval (árc Parte V desde memoria) y ≥1 de spacing (caps. 14/22/24/25/13 tocados).
- [x] Responde la pregunta crítica del CORPUS («proyección, streaming, frontiers») y las 8 piezas del brief (vista proyectada, streaming, bloques, frontiers, bitsets, paralelismo documentado, snapshots, OLTP vs analítica).
- [x] Repaso-árc de la Parte V (22→26) con diagrama, como el cap. 21 hizo con la IV; ganchos al cap. 27 (Parte VI) y Vol.III cap. 51.
- [x] Anécdota verificada con fuente: Pregel (Malewicz et al., SIGMOD 2010, DOI 10.1145/1807167.1807184) + cronología posterior verificada (GraphChi OSDI 2012, GraphX OSDI 2014) y conexiones reales verificadas (Neo4j GDS graph.project, Kùzu columnar CIDR 2023, DuckDB out-of-core 2024).
- [x] Las cifras usadas en capítulo y ejercicios son las de los tests REALES ejecutados (27 passed).
