# CONTRATO DE CAPÍTULO — Vol.II Cap. 22: Caminos mínimos ponderados (Dijkstra, Bellman-Ford)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap22_caminos_minimos.rs` (~1.290
> líneas, 30 tests en `tests_caminos` + 3 doctests, sin crates externas —
> `BinaryHeap` de std como el cap. 4 del Vol.I). Decisiones reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §27 (§24: métricas del
> cap. 20; §23: NULL del cap. 20). Este capítulo ABRE la Parte V (algoritmos
> sobre el grafo persistente), línea 37 de `manuscrito/vol2/tabla-de-contenidos.md`.
> El ángulo del Vol.II no es el algoritmo (Vol.I caps. 4 y 9) sino ejecutarlo
> SOBRE el store con pesos leídos de PROPIEDADES de arista: la consulta del
> brief es `SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance`.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: `Value` con sus seis variantes y `type_name()`
  (cap. 7); `Edge.props`, `EdgeId`/`NodeId` (cap. 7); el trait `GraphStore`
  (`get_node`/`get_edge`/`out_edges`/`iter_nodes`/`iter_edges`) como puerto
  hexagonal sobre `&dyn` (cap. 8); el coste físico de cada lectura — páginas,
  pager, buffer pool (caps. 11-13); el CSR del cap. 14 (offsets + targets, SÓLO
  topología, sin ids de arista); el motor Volcano y su `Expand` que encadena
  saltos, `demo_graph` (Ana/Bo/Carla/Dani/Madrid/Lisboa) y la semántica
  «propiedad ausente = NULL» (cap. 20); Dijkstra/Bellman-Ford como algoritmos
  sobre `Vec<Vec<(usize, i64)>>` a mano (Vol. I, cap. 4) y Johnson (Vol. I,
  cap. 9).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «el
  camino con menos saltos es el más corto» — falso en cuanto las aristas pesan
  (el test `dijkstra_la_ruta_con_menos_saltos_no_es_la_mas_barata`: directo
  10.0 vs dos saltos 2.0+3.0); (2) «si falta el peso, usa 1.0 (o 0) y sigue» —
  un default silencioso convierte dato sucio en respuesta limpia y mentirosa;
  (3) «Dijkstra falla con negativos, punto» — no: puede dar respuestas
  PLAUSIBLES pero malas (peor que fallar); por eso validamos eager y no al
  pisarlos; (4) «un ciclo negativo siempre rompe Bellman-Ford» — sólo el
  ALCANZABLE desde el origen contamina la respuesta; (5) «NaN es un número más»
  — NaN rompe el orden total del heap (panic documentado en `Cost::cmp`);
  (6) «el early-exit es una micro-optimización opcional» — es el invariante
  codicioso hecho código; y BF NO puede tener uno análogo.
- **NO debe saber todavía**: A*, heurísticas admisibles/consistentes y
  coordenadas (cap. 23, para lo que ya existen `MissingCoordinate`/
  `InconsistentHeuristic` y el contador `expanded`); PageRank/centralidad
  (cap. 24); la proyección con pesos del CSR (cap. 26 — deuda explícita de
  este capítulo); bidirectional Dijkstra, delta-stepping, k-shortest (Yens),
  Johnson completo sobre el store. Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: fuente de pesos (`WeightSource::Property`/`Constant` con
  `Default = 1.0`); extracción estricta (`edge_weight`: ausente o NULL →
  `MissingWeight`, no numérico → `InvalidWeight` con `type_name`, NaN/±∞ →
  `NonFiniteWeight`; promoción Int→Float con pérdida >2^53 documentada);
  `Cost` como newtype f64 con `Ord` total para el heap; Dijkstra con
  `BinaryHeap<Reverse<(Cost, NodeId)>>`, borrado perezoso (`settled`),
  predecesores como `PathStep`, finalización anticipada al destino, sanidad
  eager O(E) de no-negatividad (`NegativeWeight`); Bellman-Ford con lista de
  relajación materializada 1 vez, V-1 pasadas con parada temprana, pasada de
  verificación → `NegativeCycle` ALCANZABLE señalando la arista que aún relaja;
  `ShortestPaths`/`Path`/`PathStats` (relax_attempts/updates, popped con
  obsoletos, rounds); `CostOverflow` para no confundir infinito real con el
  centinela `INFINITY` de inalcanzable; `PathError` (7 variantes del cap. 22).
- `practice`: `&dyn GraphStore` y sus métodos (cap. 8); `Edge.props` y
  `Value` (cap. 7); el grafo demo y NULL=ausencia (cap. 20); CSR/`neighbors_out`
  (cap. 14); heap binario y relaxación (Vol. I, cap. 4).
- `consolidate`: «derivar, no llevar en la cabeza»; «la respuesta de una BD no
  debe depender de qué zona del grafo pisó la consulta»; el store como puerto.
- `out_of_scope` (solo nombrar): A*/heurísticas (cap. 23), proyección con
  pesos CSR (cap. 26), Johnson, Yens k-shortest, delta-stepping, búsqueda
  bidireccional, parallel Dijkstra (GDS).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué el camino con menos saltos NO es el más
  barato en cuanto los pesos dejan de ser todos 1.0, con el contraejemplo del
  test; (2) enumera las tres clases de peso sucio y su error tipado exacto
  (Missing/Invalid/NonFinite) y dice por qué NULL se trata como ausencia
  (cap. 20 al revés: propiedad ausente = NULL); (3) enuncia el invariante
  codicioso («cuando un nodo sale del heap su distancia es definitiva») y usa
  ese MISMO argumento para justificar el early-exit de Dijkstra y su ausencia
  en Bellman-Ford; (4) distingue ciclo negativo alcanzable (contamina, error)
  de inalcanzable (no contamina, respuesta normal); (5) dice qué miden
  `relax_attempts` vs `relax_updates` y qué revela su ratio.
- **Skills**: (1) ejecutar `dijkstra`/`dijkstra_path`/`bellman_ford`/
  `bellman_ford_path` sobre `demo_graph` y grafos propios con `WeightSource`;
  (2) predecir el error exacto (variante + arista) antes de ejecutar, con
  pesos sucios; (3) leer `PathStats` y diagnosticar el trabajo del cálculo.
- **Wisdom**: (1) decide Dijkstra vs Bellman-Ford según el CONTRACTO de datos
  (¿puede haber negativos legítimos? Dijkstra falla ruidoso; BF paga V-1
  pasadas); (2) decide semántica estricta vs default silencioso: cuándo una
  BD debe negarse a contestar (dato sucio) en vez de contestar casi-bien.

## 4. Modelo mental

- **La red de tramos con tarifa**: nodos = estaciones, aristas = tramos con
  precio pegado a la arista (`Edge.props`), coste = lo que pagas de puerta a
  puerta. Dos modos de resolver la misma red: **la ventanilla que llama por
  tarifa** (Dijkstra: atiende en orden de precio; cuando te llaman, tu tarifa
  queda LACRADA — `settled` — porque todos los precios pendientes son ≥ que el
  tuyo; si esperas a un destino concreto, cuando lo llaman te levantas —
  early-exit) y **el tablón de anuncios por rondas** (Bellman-Ford: cada
  pasada TODOS releen el tablón y pegan su mejora; sin orden de atención, sin
  lacrar — pero tolera descuentos/negativos y detecta la rueda de descuento
  infinita). El peso que falta es un tramo SIN precio pegado: la ventanilla
  se niega a inventárselo.
- **Diagramas ASCII**: (a) el diamante del Vol.I con precios (0→1:1, 1→3:2,
  0→2:4, 2→3:3; gana la ruta de DOS saltos coste 3); (b) la vida de una
  entrada del heap (push → obsoleta → pop descartado por `settled`, popped vs
  expanded); (c) BF: pasadas como flechas sobre la lista de aristas + la
  pasada de verificación señalando la arista del ciclo.
- **Momento ¡ajá!**: «el orden de atención NO es un truco de implementación:
  es la corrección. Puedo lacrar un nodo porque ningún tramo futuro puede
  rebajar lo ya pagado — y eso es exactamente lo que un peso negativo rompe,
  y lo que Bellman-Ford paga con V-1 pasadas para recuperarlo».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap22_caminos_minimos.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Pesos en PROPIEDADES de arista (`WeightSource::Property`) con semántica ESTRICTA (`edge_weight`) | Un grafo de propiedades es schemaless: el dato lo puso un usuario o un import y puede estar sucio. Ausente o NULL → `MissingWeight`; Bool/String/Bytes → `InvalidWeight` (con `Value::type_name`); NaN/±∞ → `NonFiniteWeight`. La BD debe VER su dato sucio | Default silencioso (peso 1.0 si falta): convierte un problema de calidad de dato en una respuesta limpia y falsa; NULL=0 haría que «falta» pese menos que «cero real» | Caminos «casi-bien» sin error — el peor modo de fallo de una BD; un NaN suelto además rompe el orden del heap (panic en `Cost::cmp`) | `edge_weight` (líns. 166-194); tests `peso_ausente_o_null_es_missing`, `peso_de_tipo_no_numerico_es_invalid`, `peso_nan_o_infinito_es_no_finito`; CLRS cap. 24 (pesos reales); contraste pgRouting (cost negativo = «arista no existe», borrado silencioso) |
| 2 | NULL = ausencia de peso (mismo trato que propiedad ausente) | Coherencia inversa con el cap. 20: allí la propiedad ausente SE VEÍA como NULL; aquí el NULL SE TRATA como ausencia. «Sin precio pegado» es un único concepto | NULL = 0.0 (gratis) o NULL = salto sin peso: dos semánticas para «no lo sé», una de ellas miente | Distancias absurdas que nadie sabe de dónde salen | `edge_weight` match `None \| Some(Value::Null)`; cap. 20 (`MIGRATION` §23) |
| 3 | `WeightSource::Constant` con `Default = 1.0` | Lo menos sorprendente cuando nadie dijo qué propiedad es el peso: contar saltos — exactamente lo que el `Expand` encadenado del cap. 20 hacía sin saberlo. Deja explícito que «no ponderado» ES un caso particular de ponderado (todos a 1.0) | Sin Default (obligar a elegir): cada caller inventaría su convención | Convenciones divergentes entre callers del mismo store | `WeightSource::default()`; test `la_fuente_de_pesos_cambia_la_respuesta` (mismo grafo, dos respuestas) |
| 4 | Promoción Int→Float (`i as f64`) con pérdida >2^53 DOCUMENTADA y testeada | Los pesos nacen de `Value::Int` o `Value::Float` (cap. 7); unificar en f64 evita dos pipelines. La pérdida es real y se enseña: 2^53+1 se promueve a 2^53 | Aritmética entera exacta (i64) + rama Float aparte: duplica el álgebra de costes y el `Cost` del heap | Pérdida silenciosa sin doc: el lector confía en dígitos que f64 no representa | `edge_weight` (`Some(Value::Int(i)) => *i as f64`); test `pesos_int_se_promocionan_a_float_con_perdida_documentada`; IEEE 754 (f64: enteros exactos hasta 2^53) |
| 5 | Dijkstra sobre `&dyn GraphStore`, NO sobre el CSR del cap. 14 | Los pesos viven en `Edge.props` y el acceso EdgeId→Edge es justo el puerto del cap. 8; el CSR persiste SÓLO topología (offsets+targets, sin ids de arista): no puede responder «¿cuánto pesa esta arista?». Además `&dyn` mantiene el algoritmo agnóstico al backend (MemoryStore hoy, disco mañana) | Proyectar el CSR con pesos AHORA: es exactamente la proyección del cap. 26 — deuda explícita, no omisión | Un pseudo-CSR ad hoc con pesos a medias que el cap. 26 tendría que rehacer | Cabecera del módulo (líns. 43-53); test `proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra` (CSR = oráculo de alcanzabilidad); cap. 14 |
| 6 | `BinaryHeap<Reverse<(Cost, NodeId)>>` de std + borrado perezoso (`settled`) | std no ofrece decrease-key: en vez de una cola indexada (crate externa), se INSERTA la mejora nueva y se descarta la entrada vieja al salir (si `settled[u]`, `continue`). O(log n) por push, cero dependencias — mismo criterio que el Vol.I cap. 4 | Cola indexada con decrease-key (p.ej. `priority-queue`): dependencia externa para ahorrar entradas obsoletas; o reheapificar a mano: reinventar el heap mal | Sin Reverse: un max-heap contesta al PEOR candidato primero; sin perezoso: no hay cómo «bajar» una prioridad ya insertada | `dijkstra_impl` (líns. 583-623); docs std `BinaryHeap` (max-heap, exige `Ord`); Vol. I cap. 4 |
| 7 | `Cost` = newtype f64 con `Eq`/`Ord` totales | `f64` NO implementa `Ord` (NaN rompe el orden total) y `BinaryHeap` exige `Ord`. Todos los costes del heap son finitos (validados en `edge_weight`; desbordes → `CostOverflow`), así que el `expect` de `partial_cmp` es unreachable documentado | `f64::total_cmp` suelto por el código: el centinela/NaN vuelve a colarse por cualquier grieta; o enteros escalados: pierde `Value::Float` | Panic indocumentado en mitad de una consulta, o NaN ordenado «a medias» | `Cost` (líns. 79-103); test `cost_orden_total_con_negativos_y_cero` |
| 8 | Early-exit al destino (`if target == Some(u) { break; }`) | Invariante codicioso: cuando el destino sale del heap, su distancia es DEFINITIVA (todo lo que queda pesa igual o más). Seguir trabajando sería puro derroche | Seguir hasta vaciar el heap: mismo resultado, todo el grafo recorrido por un destino que ya contestó | Pagar la tabla completa por cada consulta punto-a-punto (test: pops ≤ 2 para vecino inmediato vs 6 de la tabla) | `dijkstra_impl` (líns. 596-599); test `dijkstra_finalizacion_anticipada_extrae_menos_nodos`; CLRS §24.3 |
| 9 | Bellman-Ford SIN early-exit por destino; PERO con parada temprana de PASADAS | Con pesos negativos un camino MÁS LARGO puede ganar DESPUÉS: no hay invariante que permita lacrar el destino. La parada que sí existe — una pasada sin cambios implica convergencia — está implementada (`if !changed { break; }`) | Early-exit por destino «simétrico» al de Dijkstra: contestaría distancias aún mejorable — resultado incorrecto silencioso | Camino subóptimo devuelto como óptimo | `bellman_ford_path` doc (líns. 732-737); test `bellman_ford_para_temprano_cuando_nada_cambia` (rounds=2 en cadena de 4 saltos) |
| 10 | Validación EAGER de negativos en TODAS las aristas antes de correr Dijkstra (`validate_edge_weights` O(E)) | Una BD prefiere FALLAR RUIDOSAMENTE a contestar con números que podrían ser válidos por casualidad. El día que el usuario importa negativos querrá saberlo en TODAS sus consultas, no sólo en las que cruzan esa zona: la respuesta no debe depender de qué parte del grafo pisó la búsqueda. Para negativos legítimos existe `bellman_ford` | Validar al pisar la arista negativa: Dijkstra daría respuestas plausibles-malas en las consultas que no la pisan — inconsistencia entre consultas del mismo grafo | «Casi-bien» intermitente: el bug más caro de depurar porque a veces acierta | `validate_edge_weights` (líns. 498-512); test `dijkstra_rechaza_negativos_aun_en_zonas_no_visitadas` (negativo en componente 3⇄4 inalcanzable); CLRS §24.3 (precondición w≥0); `NegativeWeight` Display sugiere `use bellman_ford` |
| 11 | Bellman-Ford: ciclo negativo ALCANZABLE = error (`NegativeCycle` señalando la arista); inalcanzable = no error | Aguas abajo de un ciclo negativo alcanzable las distancias tienden a -∞: devolver media tabla válida sería mentir. Un ciclo en componente INALCANZABLE no contamina nada (nadie lo alcanza) — la pasada de verificación lo exige explícitamente: `dist[u] != INFINITY && dist[u]+w < dist[v]` | Fallar con cualquier ciclo negativo en el grafo (aunque nadie lo alcance): consultas sobre grafos parcialmente sucios dejarían de responder sin motivo; o devolver la tabla «como si nada» | Media tabla de -∞ disfrazada de respuesta; o grafo entero inutilizado por una isla rota | Pasada de verificación (líns. 717-721); tests `bellman_ford_detecta_ciclo_negativo_alcanzable` (señala edge 1), `bellman_ford_ciclo_negativo_inalcanzable_no_contamina`, `bellman_ford_self_loop_negativo_es_ciclo_negativo`; CLRS §24.1 |
| 12 | `CostOverflow` reportado en vez de dejar que el coste derive a ∞ | `f64::INFINITY` es el CENTINELA de inalcanzable en `dist`; un desbordamiento real colisionaría con él y un nodo carísimo pasaría por inalcanzable | Dejar que flote a ∞: el inalcanzable y el demasiado-caro se confunden | Camino existente reportado como «no hay camino» | `dijkstra_impl` (`if !new.is_finite()`); test `dijkstra_coste_que_desborda_es_error_tipado` (1e308+1e308) |
| 13 | `pred[v]` guarda el `PathStep` COMPLETO (arista+from+to+peso) | `path_to` reconstruye el camino sin VOLVER a tocar el store (cada `get_edge` puede ser una página → buffer pool, caps. 11-13). Bonus: `Path` lleva los pesos por paso y `Display` estilo Cypher sale gratis | Guardar sólo el NodeId predecesor y re-leer las aristas al reconstruir: E lecturas extra por consulta | Latencia de reconstrucción proporcional al camino, en disco = page faults | `ShortestPaths::path_to` (líns. 452-468); `Path::Display` `(n0)-[e4 w=1.5]->(n1)... cost=3.5` |
| 14 | BF materializa la lista de relajación UNA vez (`relax: Vec<(u,v,eid,w)>`) | Los pesos se leen de las props una sola pasada: releerlas en cada ronda serían V-1 búsquedas hash por arista para el MISMO valor — y en disco, V-1 lecturas de página | Recorrer `iter_edges` en cada pasada: correcto, pero multiplica el coste de E por las rondas | BF V-1 veces más caro en I/O sin ganancia alguna | `bellman_ford` (líns. 672-676); caps. 11-13 (el coste de cada lectura) |
| 15 | `PathStats` con `relax_attempts`/`relax_updates`/`popped`/`rounds` (y `expanded` para el cap. 23) | Pedagogía del «cuánto cuesta calcular»: attempts vs updates revela el grafo que NO mejora (todo intento fallido es trabajo pagado sin fruto); popped incluye las entradas obsoletas del borrado perezoso (ver el precio del truco); rounds muestra la convergencia de BF; `expanded` (pops vivos) se añade YA para comparar Dijkstra vs A* en el cap. 23 | No medir: el lector no ve que el borrado perezoso cuesta pops muertos ni que BF converge antes de V-1 | Optimizaciones cieblas: no se puede mejorar lo que no se ve | `PathStats` (líns. 323-351); tests `dijkstra_finalizacion_anticipada_extrae_menos_nodos`, `bellman_ford_para_temprano_cuando_nada_cambia` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: encadenar `Expand` (cap. 20) y contar saltos — el camino con
  menos relaciones. Versión «ponderada ingenua»: leer `props["distance"]` y,
  si falta, usar 1.0 «para no romper» (default silencioso); o reutilizar el
  código del Vol.I con el grafo reconvertido a mano a `Vec<Vec<...>>`.
- **Qué la rompe**: (a) pesos reales — el directo 10.0 pierde contra los dos
  saltos 2.0+3.0 (`dijkstra_la_ruta_con_menos_saltos_no_es_la_mas_barata`);
  (b) el dato sucio — con `WeightSource::property("since")` sobre `demo_graph`,
  el self-loop de Dani (edge 3) no tiene la propiedad y las LIVES_IN tampoco:
  el default 1.0 contestaría con «precios» inventados; (c) el NaN suelto —
  sin validación, el heap exige `Ord` y el `partial_cmp` de NaN es `None`:
  panic a mitad de consulta; (d) la reconversión a mano a Vec es exactly lo
  que una BD debe hacer POR el usuario, no lo que le puede exigir.
- **Evolución visible**: `dijkstra(&store, 0, &WeightSource::property("distance"))`
  — la semántica estricta convierte el dato sucio en `MissingWeight { edge: 3,
  prop: "since" }` ANTES de contestar; el algoritmo corre sobre `&dyn
  GraphStore` sin conversiones; `Path` llega con pasos+pesos+coste+stats y
  Display Cypher; BF abre la puerta a negativos legítimos y señala el ciclo
  contaminante por arista.

## 7. Prueba de fuego

- **Sobre `demo_graph`** (test `demo_graph_con_pesos_reales_y_calidad_de_dato`):
  contando saltos, Ana→Carla cuesta 2.0 (0 -KNOWS-> 1 -KNOWS-> 2); con pesos
  reales (`since`) la consulta RECHAZA el grafo: `MissingWeight { edge: 3 }` —
  el self-loop de Dani es la primera arista sin el dato; Dani es inalcanzable
  desde Ana (sólo tiene self-loop) y eso NO es error, es `Ok(None)`.
- **El oráculo CSR** (`proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra`):
  la alcanzabilidad BFS sobre la proyección CSR del cap. 14 coincide EXACTO
  con `sp.reached()` — la topología y el algoritmo cuentan la misma historia.
- **Tests citados**: `cost_orden_total_con_negativos_y_cero`,
  `peso_ausente_o_null_es_missing`, `peso_de_tipo_no_numerico_es_invalid`,
  `peso_nan_o_infinito_es_no_finito`, `dijkstra_camino_clasico_del_diamante`,
  `dijkstra_la_ruta_con_menos_saltos_no_es_la_mas_barata`,
  `la_fuente_de_pesos_cambia_la_respuesta`,
  `dijkstra_multigrafo_elige_la_arista_paralela_mas_barata`,
  `dijkstra_destino_inalcanzable_devuelve_none`, `dijkstra_origen_igual_destino_coste_cero`,
  `dijkstra_self_loop_positivo_no_ayuda`,
  `dijkstra_nodos_desconocidos_en_grafo_vacio_y_con_huecos`,
  `dijkstra_rechaza_negativos_aun_en_zonas_no_visitadas`,
  `dijkstra_coste_que_desborda_es_error_tipado`,
  `dijkstra_finalizacion_anticipada_extrae_menos_nodos`,
  `dijkstra_tabla_completa_con_distancias`,
  `pesos_int_se_promocionan_a_float_con_perdida_documentada`,
  `bellman_ford_coincide_con_dijkstra_sin_negativos`,
  `bellman_ford_explota_un_peso_negativo_que_dijkstra_rechaza`,
  `bellman_ford_detecta_ciclo_negativo_alcanzable`,
  `bellman_ford_ciclo_negativo_inalcanzable_no_contamina`,
  `bellman_ford_self_loop_negativo_es_ciclo_negativo`,
  `bellman_ford_para_temprano_cuando_nada_cambia`,
  `bellman_ford_tambien_valida_los_pesos_estrictamente`,
  `proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra`,
  `demo_graph_con_pesos_reales_y_calidad_de_dato`,
  `camino_display_estilo_cypher`.
- **Síntoma si el lector se salta el capítulo**: sus caminos «ponderados» son
  BFS disfrazado (cuenta saltos), los pesos ausentes se rellenan en silencio y
  nadie puede explicar por qué la misma consulta responde distinto tras un
  import sucio — la Parte V entera (A*, PageRank) heredaría números sin
  contrato de dato.

## 8. Trampas y errores comunes

1. **Confundir "menos saltos" con "más barato"**: la trampa nº 1 conceptual;
   cura: `la_fuente_de_pesos_cambia_la_respuesta` (mismo grafo, dos ganadores).
2. **Rellenar el peso ausente** (1.0/0.0/saltar la arista): las tres variantes
   del mismo pecado — contestar con dato inventado. Cura: `MissingWeight`
   eager. (pgRouting hace la tercera con costes negativos: contraste real.)
3. **Esperar early-exit de BF o desactivar el de Dijkstra**: la primera da
   respuestas subóptimas con negativos; el segundo paga la tabla completa por
   cada punto-a-punto sin necesidad.
- **Precisión de lenguaje (glosario)**: *peso* (lo que vale UNA arista) vs
  *coste* (la suma por un camino); *relajación intentada* vs *relajación que
  mejora* (attempts/updates); *asentar/settle* (distancia definitiva) vs
  *alcanzar* (distancia finita provisional); *inalcanzable* (centinela ∞) vs
  *desbordado* (`CostOverflow`); *ciclo negativo alcanzable* vs *existente*;
  *borrado perezoso* vs *decrease-key*; *finalización anticipada* (destino
  Dijkstra) vs *parada temprana* (pasadas BF).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: sobre el grafo del test
  `la_fuente_de_pesos_cambia_la_respuesta` (0→1 w2, 1→2 w3, 0→2 w10),
  predecir SIN ejecutar camino y coste con `Constant(1.0)`, con
  `property("weight")` y con `Constant(2.5)`; y la variante EXACTA del error
  si el peso viniera de `"distance"`. Verificación: los tests del workspace.
  Pistas: (1) ¿qué cuenta un `Constant` — qué hay en las aristas?; (2) ¿la
  validación eager toca TODAS las aristas o sólo las del camino?; (3) ¿qué
  tipo devuelve la suma de floats? Criterio: tres costes exactos + variante
  del error con su arista.
- **analizar (intermedio — spacing con caps. 11-14)**: (a) por qué BF
  materializa la lista de relajación UNA vez y qué coste tendría releer las
  props en cada pasada si el store viviera en disco (cada `get_edge` puede
  ser page fault → buffer pool del cap. 13); (b) qué rompe un NaN en el heap
  si no se valida (exigencia `Ord` de `BinaryHeap`, `partial_cmp` → None →
  panic documentado); (c) por qué `pred` guarda el `PathStep` completo.
  Verificación: `bellman_ford_para_temprano_cuando_nada_cambia`,
  `peso_nan_o_infinito_es_no_finito`. Pistas: (1) ¿cuántas rondas × aristas
  releería?; (2) ¿qué orden total exige el heap y qué hace NaN con él?;
  (3) ¿cuántas lecturas de store ahorra la reconstrucción? Criterio: razonar
  I/O, no sólo complejidad.
- **crear (experto)**: parte 1 (retrieval puro): de memoria, la vida de una
  entrada del heap de Dijkstra (push, obsoleta, pop descartado) y qué
  invariante autoriza el early-exit. Parte 2: escribir el test
  `ciclo_negativo_inalcanzable_se_vuelve_error_al_conectarlo` — el grafo del
  test del ciclo inalcanzable + UNA arista desde el origen hacia el ciclo →
  `NegativeCycle` señalando una arista concreta; y que `bellman_ford_path`
  también falle. Pistas: (1) ¿qué convierte «inalcanzable» en «alcanzable»
  con una sola arista?; (2) ¿qué arista sigue relajando tras V-1 pasadas?;
  (3) ¿qué condición sobre `dist[u]` pone la pasada de verificación?
  Criterio: test verde + explicar por qué ESA arista.

## 10. Preguntas abiertas (gancho al capítulo 23)

1. Dijkstra explora en círculos crecientes alrededor del origen; si el
   destino está «al otro lado» del grafo, ¿se puede guiar la búsqueda hacia
   él sin perder la optimalidad? (nace A*.)
2. ¿Qué propiedad debe cumplir esa guía (heurística) para que la respuesta
   siga siendo exactamente la de Dijkstra? (admisibilidad/consistencia.)
3. ¿Dónde viven las coordenadas de un nodo si no en `Node.props` — y qué
   semántica estricta les toca? (`MissingCoordinate` ya existe en `PathError`.)
- **Términos nuevos de glosario**: fuente de pesos, semántica estricta,
  promoción Int→Float, newtype de orden total, borrado perezoso, entrada
  obsoleta, asentar (settle), finalización anticipada, parada temprana de
  pasadas, relajación, ciclo negativo alcanzable, centinela INFINITY,
  oráculo de consistencia.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto reconstruye DESDE LA MEMORIA la vida de
  una entrada del heap y el invariante del early-exit; el esencial recuerda
  las tres clases de peso sucio y su error tipado para predecir la variante
  exacta sin pistas en el enunciado.
- **Spacing**: Value/props (cap. 7), `&dyn GraphStore` (cap. 8), el coste de
  cada lectura (caps. 11-13: materializar `relax`, `PathStep` en `pred`), CSR
  como topología pura y oráculo (cap. 14), `Expand`/NULL del cap. 20 y el
  heap del Vol.I cap. 4 — todos RE-EJERCITADOS en los ejercicios y en §22.6-§22.10.
- **Interleaving**: el intermedio mezcla calidad de dato (Value/props) con
  I/O de disco (pager/buffer pool) y orden total (Ord); el esencial cruza
  fuente de pesos con validación eager.
- **Dificultad asimétrica**: una idea nueva por sección (fuente de pesos →
  semántica estricta → heap perezoso → early-exit → negativos → ciclo
  alcanzable); los ejercicios exigen predicción y construcción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` (30 tests
  citados por nombre) y doctests ejecutables de `dijkstra`/`bellman_ford`.
- **Citas**: E. W. Dijkstra, «A Note on Two Problems in Connexion with
  Graphs», Numerische Mathematik 1 (1959) 269-271; entrevista de Philip L.
  Frana a Dijkstra (historia oral, Charles Babbage Institute, Univ. de
  Minnesota, 2001; publicada en CACM 53(8), agosto de 2010); CLRS,
  «Introduction to Algorithms», 3ª ed., cap. 24 (§24.1 Bellman-Ford, §24.3
  Dijkstra); docs de std (`BinaryHeap`, `f64`); Neo4j GDS docs («Dijkstra
  Source-Target», pesos positivos, `relationshipWeightProperty`); Kùzu (blog
  0.0.4: sintaxis `SHORTEST k`; PR #5239: chequeo de pesos negativos que
  ERRA en weighted shortest); pgRouting (docs `pgr_dijkstra`: coste negativo
  = arista inexistente; `pgr_bellmanFord`: negativos válidos).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «por qué» con alternativa descartada y fuente (15 en la tabla §5).
- [x] Escenario de fallo visible: default silencioso → respuestas plausibles-malas; NaN → panic del heap; desbordamiento → confundido con inalcanzable (§6-§8 del capítulo).
- [x] Código ejecutable en workspace (30 tests + 3 doctests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (menos saltos ≠ más barato; default amable = mentira; Dijkstra con negativos no «falla», miente; ciclo negativo sólo importa si es alcanzable; NaN no es un número más; early-exit no es opcional ni portable a BF).
- [x] Ejercicios con solución verificable (tests del workspace).
- [x] ≥1 ejercicio de retrieval (vida de una entrada del heap desde memoria) y ≥1 de spacing (caps. 7/8/11-14/20 re-ejercitados).
- [x] Anécdota verificada: Dijkstra 1956 (ARMAC, 64 ciudades, Rotterdam→Groningen, 20 min en la terraza del café de Ámsterdam sin lápiz ni papel, con su joven prometida) según contaba el propio Dijkstra en la entrevista de Frana (CBI 2001 / CACM 53(8) 2010); publicación en Numerische Mathematik 1 (1959): 269-271.
- [x] Abre la Parte V con la consulta del brief (`SHORTEST PATH ... WEIGHT ...`) como hilo conductor y la deuda del CSR con pesos (cap. 26) explícita.
