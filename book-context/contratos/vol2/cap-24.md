# CONTRATO DE CAPÍTULO — Vol.II Cap. 24: Centralidad y PageRank

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs` (~1.800 líneas,
> 28 tests en `tests_centralidad` + 6 doctests; cero cambios en módulos previos,
> sólo `lib.rs: mod/pub use`). Decisiones reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §29. Capítulo 3 de la
> Parte V (algoritmos sobre el grafo persistente); línea 39 de
> `manuscrito/vol2/tabla-de-contenidos.md`. Costura hacia el futuro: el cap 51
> del Vol.III (GraphRAG) usará `personalized_page_rank` como operador de
> recuperación; la `Proyeccion` no ponderada de aquí es la deuda que el cap 26
> saldó con `ProyeccionPonderada`.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: `&dyn GraphStore` como puerto y sus invariantes
  (cap. 8); BFS de saltos (Vol. I cap. 3; re-ejercitado en cap. 22 para
  Dijkstra); pesos con semántica ESTRICTA `WeightSource`/`edge_weight` y errores
  `PathError` (cap. 22); el CSR con `Direction` forward/backward (cap. 14);
  `Expand` UNDIRECTED y su convención de self-loops (cap. 20); normalización
  L2 y autovalores vistos ALGORÍTMICAMENTE en el Vol. I (caps. 16 y 24 Vol.I —
  PageRank como ejemplo, sin motor); PageRank personalizado y damping como
  prerrequisito declarado del Vol. III (`external_concepts: pagerank-vol2`).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «lo
  importante es tener muchos enlaces» — no: importa QUIÉN te enlaza (el voto de
  un nodo importante pesa más; el grado es local, PageRank es global); (2) «el
  damping es un parámetro de tuning numérico» — no: es lo que REPARA los dos
  fallos demostrables del eigenvector crudo (fuga por dangling y oscilación
  periódica) haciendo la matriz positiva ⇒ primitiva; (3) «los nodos colgantes
  hay que descartarlos» — falso como obligación: se redistribuyen uniformemente
  para conservar masa=1 en CADA iteración (la variante «no-scale» existe y se
  documenta, pero cambia el límite); (4) «si convergió, el resultado es
  correcto» — no: `converged=false` es una RESPUESTA válida (una BD prefiere
  decir «no convergió» a devolver números casi-buenos en silencio); (5) «un
  enlace duplicado duplica el voto» — no: también duplica el denominador; ROBA
  masa a los otros vecinos del mismo origen (test dedicado).
- **NO debe saber todavía**: proyección ponderada/streaming/frontiers (cap. 26),
  Louvain y modularidad (cap. 25), PPR como recuperación semántica y GraphRAG
  (Vol. III cap. 51 — aquí sólo se siembra la costura `Teleport`), algoritmos
  acelerados de PageRank (GPU, push/sweep, block strategies), Betweenness
  aproximada por muestreo, K-core/centralidades alternativas. Se nombran como
  «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: centralidad como FAMILIA de preguntas (¿cuántos vecinos? ¿a qué
  distancia estoy de todos? ¿por cuántos caminos paso? ¿quién me apunta?);
  `GraphDirection{Out,In,Both}` (In = transpuesta pura; Both = unión como
  CONJUNTO, convención `Expand` UNDIRECTED); corrección Wasserman-Faust para
  closeness en componentes desconectadas `((r-1)/(n-1))·((r-1)/Σd)`; Brandes
  2001 (σ de caminos mínimos, predecesores, dependencias hacia atrás) y su
  normalización dirigida `1/((n-1)(n-2))`; eigenvector por iteración de
  potencia con L2 por paso y sus DOS fallos; random surfer, damping d∈(0,1)
  ABIERTO, teleport, redistribución uniforme de dangling (masa=1 invariante);
  convergencia L1 + `history` por iteración (razón ≈ d·λ₂); `Teleport{Uniform,
  Personalized}` con núcleo único de potencia; `CentralidadStats` (bfs_runs,
  edges_scanned, iterations) para MEDIR el coste, no declamarlo.
- `practice`: BFS y colas (Vol. I cap. 3 / cap. 22); proyección materializada
  ids-ordenados + índice denso (patrón que el cap. 25 hereda como
  `GrafoPonderado`); semántica estricta de pesos del cap. 22 (nombrada como la
  deuda del closeness ponderado → cap. 26).
- `consolidate`: «derivar, no llevar en la cabeza» (el índice denso excluye los
  huecos de `delete_node`); el store como puerto; determinismo (orden de id,
  desempate por id en rankings); fail loudly con enums de error.
- `out_of_scope` (solo nombrar): PageRank industrial (map-reduce, bloques),
  betweenness aproximada, Louvain (cap. 25), proyección con pesos (cap. 26),
  PPR como recuperación de documentos (Vol. III cap. 51).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) nombra las CINCO familias con su pregunta y su coste
  (grado O(V+E); closeness O(V·(V+E)); betweenness O(V·E) con Brandes;
  eigenvector/PageRank O(iter·E)) y dice qué mide `CentralidadStats` para
  verificarlo; (2) explica los DOS fallos del eigenvector crudo (hojas→0 por
  fuga de masa dangling; cola+3-ciclo OSCILA por periodicidad) y los DOS
  arreglos de PageRank (damping + redistribución uniforme), con el test que lo
  DEMUESTRA en el mismo grafo; (3) calcula a mano una iteración de PageRank en
  un 3-ciclo (teleport + votos 1/grado + cuota colgante) y comprueba masa=1;
  (4) distingue delta L1 (masa que se mueve, interpretable como probabilidad,
  comparable entre tamaños) de max-delta (documentado y descartado); (5) dice
  qué es λ₂ y por qué la razón de convergencia es ≈ d·λ₂ < 1 (la masa de error
  se contrae geométricamente; el `history` lo muestra).
- **Skills**: (1) ejecutar las cinco familias sobre un store real y leer
  `ranking()`, `stats` y `Display`; (2) construir `Teleport::Personalizado`
  con semillas ponderadas y predecir qué nodos quedan fuera del «mundo»;
  (3) leer un `history` de PageRank y diagnosticar (monótono geométrico vs
  arranque ya-estacionario `history=[0.0]` vs no-convergencia).
- **Wisdom**: (1) decide cuándo NO PageRank: si la pregunta es local (dos
  nodos, un camino) los caps. 22-23 bastan y son más baratos; PageRank es para
  RANKEAR globalmente sin pregunta concreta; (2) decide qué teleport: uniforme
  (¿qué es importante en general?) vs personalizado (¿qué es relevante PARA
  ESTA consulta/usuario?) — y por qué el cap 51 lo necesitará separado.

## 4. Modelo mental

- **El surfer aleatorio**: el surfer pulsa enlaces al azar (votos de 1/grado
  del emisor) y, con probabilidad 1−d, se aburre y teletransporta. PageRank =
  fracción de ETERNIDADES que pasa en cada página = distribución estacionaria.
  El «voto» de un enlace vale la importancia del votante repartida entre sus
  enlaces: ser enlazado por uno importante vale más que ser enlazado por cien
  desconocidos. Los colgantes teletransportan (nadie queda atrapado); el
  teleport personalizado cambia «el centro del mundo» (dónde aterriza el
  aburrimiento).
- **Diagramas ASCII**: (a) el grafo trampa cola+3-ciclo con la masa rotando
  sin asentarse (eigenvector) vs asentándose (PageRank); (b) la anatomía de UNA
  iteración: teleport + cuota colgante + votos (caudales de masa); (c) la
  familia de centralidades como tabla pregunta→métrica→coste.
- **Momento ¡ajá!**: «la importancia no se DECLARA, se FLUYE: cada nodo la
  recibe de sus entrantes y la devuelve repartida entre sus salientes — y el
  damping es la válvula que impide que el flujo se pierda (colgantes) o se
  quede dando vueltas (ciclos) para siempre».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap24_centralidad.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Proyección materializada PRIVADA (`Proyeccion`) una sola vez, no el store en el bucle | Los algoritmos iterativos tocan la adyacencia iteraciones×E veces; `out_edges`+`get_edge` por arista y por ronda re-leería el store O(iteraciones) veces. Materializar: ids ordenados (determinismo), índice denso NodeId→posición (los huecos de `delete_node` quedan fuera del cálculo), vecindarios una vez | Trabajar directo sobre `&dyn GraphStore`: coste O(iter·(V+E)) de llamadas virtuales por iteración; imposible enseñar el coste medido | Cada iteración paga el store completo; el «coste computacional» del guion sería declamado | Banner del módulo (líns. 63-80); `Proyeccion::proyectar`; MIGRATION §29.1 |
| 2 | Proyección NO ponderada (deuda explícita hacia el cap. 26) | El guion del brief pide las familias «para explicar», no optimización industrial; sin pesos el BFS de saltos basta para closeness/betweenness. El cambio a ponderado (Dijkstra del cap. 22) queda documentado como costura: mismo patrón, con `edge_weight` | Proyección con pesos YA: duplicaría la semántica estricta del cap. 22 antes de tiempo y violaría una-idea-por-sección | Closeness ponderado prometido y no sostenible; carga cognitiva | Doc de `closeness_centrality` («deuda hacia el cap 26»); MIGRATION §29.1 y §31 (cap. 26 la saldó) |
| 3 | `GraphDirection` (no `Direction`), con `In` = transpuesta PURA de la salida y `Both` = unión como CONJUNTO | Colisión de nombre con el `Direction` forward/backward del CSR (cap. 14) en la API plana. `Both` deduplica vecinos y cuenta el self-loop UNA vez (convención `Expand` UNDIRECTED del cap. 20): sin dedup, un store simetrizado a mano contaría cada par doble | Mezclar in-edges en la colección Y transponer (bug real corregido): vecinos duplicados y grados inflados | Grados inflados ×2 en stores simetrizados; tests de libro (0.75 del camino) imposibles | `GraphDirection` (líns. 110-130); MIGRATION §29.2 (bug documentado); test `proyeccion_in_transpone_la_adyacencia` |
| 4 | Closeness con corrección Wasserman-Faust `((r-1)/(n-1))·((r-1)/Σd)` | En componentes desconectadas, el `(n-1)/Σd` clásico inflaría (un nodo aislado con 1 alcanzado sería «central»). WF penaliza proporcionalmente a lo que NO alcanzas: dos 2-ciclos separados → todos 1/3, no 1 | Freeman puro sin corrección: componentes separadas comparables como si fueran un mundo común | Nodos aislados «centrales»; ranking sin sentido en grafos reales (siempre desconectados) | Wasserman & Faust 1994, §4.2.1 (fórmula); test `closeness_componentes_desconectadas_wasserman_faust` |
| 5 | Betweenness = Brandes 2001 con dependencias hacia atrás, no enumeración por pares | El ingenuo corre un BFS POR PAR (O(V²·E) sólo en distancias) y encima enumera caminos que pueden ser EXPONENCIALES. Brandes: V BFS con σ y predecesores + acumulación en orden inverso: O(V·E). Números: V=1M, E=10M → Brandes ~10¹³ operaciones; por pares ~10¹⁹ — seis órdenes de magnitud | Enumeración por pares: impracticable fuera del juguete; Brandes lo cita como el estado previo O(V³)+ en no ponderado | Betweenness inutilizable en grafos reales; el capítulo no podría medir `bfs_runs=V` | Brandes 2001 (J. Math. Sociology 25(2), págs. 163-177): O(nm) no ponderado; banner del módulo (líns. 33-38); test `betweenness_camino_lineal_valores_de_libro` |
| 6 | Normalización dirigida 1/((n-1)(n-2)) aunque `Both` simetrice | Una sola convención honesta: sobre el grafo simetrizado reproduce EXACTO el valor de libro no dirigido (camino 0-1-2-3 → 2/3; estrella → 1). Dos convenciones ocultas según dirección sería una trampa de lector | Normalización 2/((n-1)(n-2)) «no dirigida» aparte: duplicar código para el mismo número; no dirigido = dirigido simetrizado con la misma fórmula | Valores que no cuadran con ningún libro; tests frágiles | Doc de `betweenness_centrality` (líns. 630-640); tests del camino (2/3) y estrella (1.0) |
| 7 | Eigenvector IMPLEMENTADO con sus dos fallos como tests (el «antes» honesto) | Es la mejor demostración del porqué del damping: el MISMO grafo (cola+3-ciclo) donde eigenvector agota 100 iteraciones sin converger (oscila) es donde `page_rank` converge. Sin L2 por paso la masa que escapa por colgantes colapsaría el vector a 0 — por eso se normaliza cada paso | Omitir eigenvector «porque PageRank es mejor»: el lector nunca VERÍA qué arregla el damping; sería magia | Damping aceptado por fe; misconception «es tuning» sin corregir | Tests `eigenvector_estrella_las_hojas_a_cero`, `eigenvector_no_converge_en_periodico_y_pagerank_si`; MIGRATION §29.4 |
| 8 | Damping ∈ (0,1) ABIERTO por ambos extremos | d=0 es «sólo teleport» (una iteración, sin estructura); d=1 es eigenvector PURO — que existe como función propia, con sus problemas documentados. OJO técnico: `Range::contains` no sirve (el inicio es inclusivo) — comparación explícita | Aceptar [0,1] con casos borde especiales: dos caminos de código para dos degeneraciones que ya tienen su función | d=0/1 silenciosamente degenerados; NaN damping aceptado | `validar_parametros_pagerank` (líns. 1016-1027); test `pagerank_damping_extremos` (incluye NaN con `matches!`) |
| 9 | Dangling redistribuido UNIFORMEMENTE (masa=1 en CADA iteración) | La decisión clásica de Brin y Page (1998): el surfer que llega a una página sin enlaces teletransporta. Conserva la masa total a 1 en cada ronda → el L1 es interpretable y el invariante es testeado, no esperado | «No-scale» (descartar la masa y renormalizar al final): documentada como variante NO implementada; cambia el límite pero no el procedimiento — otro contrato semántico sin espacio aquí | Masa que se evapora: total_mass()<1, delta L1 sin lectura, invariantes rotos | Doc de `page_rank`; test `pagerank_cadena_con_dangling_solucion_a_mano` + `total_mass()` en todos los tests |
| 10 | Convergencia por **L1** (Σ\|Δscore\|), con `history` por iteración | El L1 es la MASA total que se mueve: interpretable como probabilidad («¿cuánta masa falta por asentar?» < 1e-6) y con el mismo umbral comparable entre grafos de distinto tamaño. El history hace CONTENIDO la convergencia geométrica: el lector VE cada delta y la razón ≈ d·λ₂ < 1 | Max-delta (máx. cambio por nodo): más estricto por nodo pero SIN lectura probabilística ni comparabilidad entre tamaños — documentado y descartado | Umbral cuyo significado cambia con el tamaño del grafo; convergencia sin explicación visible | Doc del módulo (líns. 82-89); tests `pagerank_suma_uno_y_convergencia_geometrica` (monótona, razón<1, contraste `history=[0.0]`); Haveliwala & Kamvar 2003 (λ₂=d); Langville & Meyer 2006 |
| 11 | PPR separado del global vía `Teleport` con NÚCLEO compartido (`iteracion_de_potencia`) | El cap. 51 (GraphRAG, Vol. III) usará PPR como operador de recuperación: la pregunta rankea el subgrafo. Si teleport y núcleo estuvieran acoplados, GraphRAG tendría que duplicar PageRank. `page_rank` = núcleo + Uniform; `personalized_page_rank` = MISMO núcleo + semillas: cero duplicación, una costura limpia | Dos funciones gemelas con el bucle copiado: divergencia silenciosa de semántica entre global y personalizado | El día que GraphRAG llegue, dos PageRank que discrepan; bugs dobles | `Teleport` (líns. 848-911) + `iteracion_de_potencia` (líns. 1042-1127); MIGRATION §29.6; OUTLINE-VOL3 `external_concepts: pagerank-vol2` |
| 12 | `CentralidadStats` (bfs_runs, edges_scanned, iterations) en TODOS los resultados | El guion exige «coste computacional» como sección; medirlo lo convierte en contenido verificable: closeness=bfs_runs V; betweenness=bfs_runs V; PageRank=iterations×E. Sin stats sería una tabla de libro declamada | Coste sólo en prosa: el lector no puede ejecutar y comprobar | Complejidad «de fe»; imposible el ejercicio de medir V·E | `CentralidadStats` (líns. 182-201); test `closeness_camino_lineal_valores_de_libro` (edges_scanned exacto: 12+4·6) |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: responder «¿quién es importante?» con GRADO: una pasada, contar
  vecinos. Versión ingenua-v2: eigenvector crudo («ser apuntado por lo
  importante») sobre la adyacencia tal cual, con normalización L2 por paso.
- **Qué la rompe**: (a) el grado es LOCAL: el nodo con más enlaces puede ser un
  spammer; lo que importa es quién vota (test `demo_graph_ranking_plausible`:
  por grado ganan 0 y 1; por PageRank arrasa Dani con su self-loop); (b) el
  eigenvector se rompe DOS veces en grafos dirigidos reales: los nodos sin
  ENTRANTES mueren a 0 por muy bien conectados que estén hacia fuera (estrella,
  hojas→0), y la masa que entra en un colgante SE ESCAPA del sistema; (c) en
  estructura periódica (cola+3-ciclo) la iteración OSCILA sin converger:
  `converged=false` tras 100 iteraciones.
- **Evolución visible**: `page_rank` = el mismo núcleo de potencia + DOS
  arreglos quirúrgicos — damping d∈(0,1) (teleport con probabilidad 1−d: la
  matriz se vuelve positiva ⇒ primitiva ⇒ convergencia garantizada y
  geométrica, razón ≈ d·λ₂) y redistribución uniforme de la masa colgante
  (masa=1 en cada iteración). Sobre esa base, `personalized_page_rank` sólo
  cambia el vector de teleport (`Teleport::Personalized`).

## 7. Prueba de fuego

- **El test que ES el capítulo**: `eigenvector_no_converge_en_periodico_y_pagerank_si`
  — el mismo grafo cola+3-ciclo (`0→1, 1→2, 2→3, 3→1`): eigenvector agota 100
  iteraciones con `delta > 1e-9` y `converged=false`; `page_rank(0.85, 200,
  1e-9)` CONVERGE con `total_mass()=1`. El porqué del damping, demostrado.
- **Soluciones a mano verificadas**: `pagerank_ciclos_compartidos_solucion_a_mano`
  (A↔B, A↔C: sistema 2 ecuaciones, y=(0.05+0.425)/1.85);
  `pagerank_cadena_con_dangling_solucion_a_mano` (0→1→2: el colgante NO absorbe
  — masa 0.474, no 1.0); `ppr_concentra_en_semillas_solucion_a_manera`
  (semilla en dos 2-ciclos: la componente lejana queda a 0 EXACTO y
  a=0.15/(1−0.85²)).
- **La sorpresa del demo**: `demo_graph_ranking_plausible` — Dani (self-loop
  3→3) ARRASA con 0.386: su self-loop le devuelve cada voto Y cobra la cuota
  colgante de las ciudades (que no votan). El score exacto 0.3855087315 fue
  recalculado a mano tras descubrir que el razonamiento inicial ignoraba esa
  cuota (lección: self-loop + masa colgante = trampa de acumulación).
- **Invariantes**: `total_mass()=1` testeado en TODOS los escenarios; damping
  fuera de (0,1) —incluido NaN— rechazado (`pagerank_damping_extremos`);
  semillas inexistentes/negativas/masa-cero rechazadas con el nodo señalado
  (`ppr_errores_de_teleport`).
- **Síntoma si el lector se salta el capítulo**: sus rankings serán de grado
  (locales, manipulables); no sabrá diagnosticar un PageRank que no converge ni
  uno que sí pero mal; y el Vol. III perderá su operador de recuperación — el
  cap. 51 enchufa `Teleport::Personalized` aquí.

## 8. Trampas y errores comunes

1. **Confundir d pequeño con «más rápido»**: d→1 converge MÁS lento (la razón
   se acerca a 1; test: d=0.99 tarda más que d=0.5); d→0 converge ya pero
   devuelve el teleport (sin estructura). 0.85 no es tradición por gusto: es el
   equilibrio del paper original.
2. **Olvidar la masa colgante**: quien calcula PageRank «a mano» con sóo
   teleport+votos obtiene masa<1 y no lo nota sin `total_mass()`. El colgante
   devuelve su masa repartida — y en el demo es lo que hace ganar a Dani.
3. **Creer que Both = out+in apilados**: `Both` es unión como CONJUNTO
   (vecinos distintos, self-loop una vez) y `In` es la transpuesta PURA con
   aristas paralelas conservadas. Mezclar colección y transposición fue un bug
   real de la implementación (MIGRATION §29.2).
- **Precisión de lenguaje (glosario)**: *centralidad* (familia de preguntas)
  vs *PageRank* (una métrica concreta); *damping* (probabilidad de seguir el
  enlace) vs *teleport* (a dónde va el 1−d) vs *dangling* (nodos sin salida —
  redistribución, no teleport); *convergencia L1* (masa que se mueve) vs
  *max-delta*; *score* (masa/probabilidad, suma 1) vs *ranking* (orden con
  desempate por id); *σ* (número de caminos mínimos) vs *dependencia*
  (fracción de pares que pasan por u); *proyección* (vista materializada
  privada) vs *CSR persistido* (cap. 14) vs `ProyeccionPonderada` (cap. 26).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval puro)**: con el libro CERRADO:
  (a) los DOS fallos del eigenvector crudo y el grafo mínimo de cada uno;
  (b) los DOS arreglos de PageRank y qué invariante garantiza cada uno;
  (c) qué mide el delta L1 y por qué la razón entre deltas consecutivos es
  ≈ d·λ₂. Luego ejecutar `cargo test -p vol2-liradb` y localizar cada
  respuesta en un test por nombre. Pistas (≤3): (1) ¿qué le pasa a un surfer
  en una página sin enlaces?, (2) ¿puede una masa quedarse rotando para
  siempre?, (3) ¿qué matriz converge siempre: la positiva o la cruda?
  Criterio: los cuatro tests citados por nombre sin mirar.
- **analizar (intermedio — spacing con cap. 14)**: explicar por qué la
  `Proyeccion` es «el CSR del cap. 14 en memoria» y qué le falta para SERLO:
  Vec<Vec<usize>> vs offsets+targets compactos, pesos, ids de arista. Luego
  MEDIR el coste de betweenness en el camino simetrizado de 6 nodos: predecir
  `bfs_runs` y `edges_scanned` ANTES de ejecutar (fórmula V·E) y verificar con
  un mini-programa contra `CentralidadStats`. Pistas: (1) ¿cuántos BFS corre
  Brandes?, (2) ¿cuántas entradas de vecindario pisa cada BFS en un camino?,
  (3) ¿qué añade la deduplicación de Both? Criterio: predicción exacta y el
  paralelo CSR en dos frases.
- **crear (experto — interleaving cap. 22/26 + costura Vol. III)**: construir
  el mini-operador de recuperación que el cap. 51 necesitará: `ppr_por_etiqueta
  (store, etiqueta, damping) -> PageRankResult` que siembra `Teleport::
  Personalized` con TODOS los nodos de esa etiqueta a peso uniforme (usando
  `iter_nodes` del trait, cap. 8) y verifica masa=1, que los nodos de otra
  etiqueta SIN camino desde la semilla quedan cerca de 0, y que el ranking
  difiere del global. Extensión conceptual (sin código): qué habría que
  cambiar para que el mundo pese por `since` de las aristas (respuesta
  esperada: nada aquí — es la `ProyeccionPonderada` del cap. 26 con la
  semántica estricta `edge_weight` del cap. 22). Pistas: (1) ¿quién valida
  las semillas: tú o `densificar`?, (2) ¿por qué las ciudades NO quedan a 0
  exacto en el demo?, (3) ¿dónde del `iteracion_de_potencia` entraría el
  peso? Criterio: test propio verde + demostración de que el núcleo NO se
  tocó.

## 10. Preguntas abiertas (gancho al capítulo 25)

1. PageRank dice QUIÉN es importante globalmente… ¿y si la pregunta es en qué
   GRUPOS se organiza el grafo? ¿Se puede medir la calidad de una partición?
   (nace la modularidad.)
2. La `Proyeccion` de aquí es no ponderada y Both deduplica paralelas — ¿qué
   semántica de pesos necesita un algoritmo que SUMA aristas entre grupos?
   (el cap. 25 la resuelve con `GrafoPonderado`; el 26 la generaliza.)
3. Con grafos que no caben en memoria, ¿se puede materializar la proyección
   de este capítulo? ¿Por bloques? ¿Con qué presupuesto? (frontiers, cap. 26.)
- **Términos nuevos de glosario**: centralidad de grado, closeness
  (Wasserman-Faust), betweenness (Brandes), caminos mínimos σ, dependencia,
  centralidad eigenvector, random surfer, damping factor, teleport,
  PageRank personalizado, dangling node, redistribución uniforme, convergencia
  L1, matriz primitiva, masa colgante, proyección materializada.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial recita fallos/arreglos/invariantes DESDE
  LA MEMORIA y luego localiza el test que los demuestra (recordar > reconocer;
  nada del enunciado revela los nombres de los tests).
- **Spacing**: intermedio → CSR del cap. 14 (offsets/targets, `Direction`
  forward/backward: la proyección es su forma en memoria) y BFS/Dijkstra del
  cap. 22 (el ponderado que sustituirá al BFS de saltos); experto → semántica
  estricta de pesos del cap. 22 y la deuda que el cap. 26 saldó; el esencial
  re-usa el store del cap. 8 y el `demo_graph` del cap. 20.
- **Interleaving**: el experto mezcla PPR (cap. 24), etiquetas del modelo
  (cap. 7), iteración sobre el store (cap. 8) y la proyección ponderada (cap.
  26); el intermedio cruza Brandes con la medición de coste (stats) y el CSR.
- **Dificultad asimétrica**: una idea nueva por sección (familia→pregunta;
  surfer→voto; eigenvector→fallo; damping→arreglo; teleport→mundo); los
  ejercicios exigen recuperación, predicción de números y construcción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb
  tests_centralidad` (28 tests por nombre) y los doctests ejecutables de cada
  firma pública.
- **Citas**: Page, Brin, Motwani y Winograd, «The PageRank Citation Ranking:
  Bringing Order to the Web» (Stanford Digital Library, 1998/1999 — SIDL-WP-
  1999-0120; presentado en WWW7, Brisbane 1998); Brin y Page, «The Anatomy of
  a Large-Scale Hypertextual Web Search Engine» (WWW7, 1998 — random surfer y
  d=0.85); patente US 6,285,999 «Method for node ranking in a linked database»
  (solicitada 9-ene-1998, inventada por Lawrence Page, asignada a Stanford,
  concedida 4-sep-2001, expirada 2018); Brandes, «A Faster Algorithm for
  Betweenness Centrality» (Journal of Mathematical Sociology 25(2), 2001);
  Wasserman y Faust, «Social Network Analysis» (Cambridge Univ. Press, 1994);
  Haveliwala y Kamvar, «The Second Eigenvalue of the Google Matrix» (Stanford,
  2003); Langville y Meyer, «Google's PageRank and Beyond» (Princeton Univ.
  Press, 2006); Neo4j GDS docs (`gds.pageRank` con `sourceNodes`,
  `gds.betweenness`); IEEE Milestone «PageRank and the Birth of Google,
  1996–1998» (ethw.org).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: eigenvector oscilando en cola+3-ciclo mientras PageRank converge (el MISMO grafo), hojas→0, masa colgante en el demo (§6-§7).
- [x] Código ejecutable en workspace (28 tests + 6 doctests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas (importa quién enlaza; damping ≠ tuning; dangling se redistribuye; converged=false es respuesta; duplicar enlace roba masa).
- [x] Ejercicios con solución verificable (tests del workspace + miniprograma propio).
- [x] ≥1 ejercicio de retrieval (fallos/arreglos/invariantes desde memoria) y ≥2 de spacing (cap. 14 CSR, cap. 22 pesos/Dijkstra).
- [x] Responde las preguntas críticas de `CORPUS.yml` (vol-II-cap-24): «PageRank personalizado; damping factor» — §5.8-§5.11 y el capítulo entero.
- [x] Anécdota verificada: Page y Brin 1998, paper + patente US 6,285,999 (asignada a Stanford) + Google nacida del proyecto BackRub de Stanford (fuentes en §11).
- [x] Spacing con cap. 25 NO anticipado (sólo preguntas abiertas); con cap. 14 y cap. 22 sí, ejercitado.
- [x] Costura Vol. III: `Teleport::Personalized` documentado como el operador del cap. 51 sin explicarlo aún.
