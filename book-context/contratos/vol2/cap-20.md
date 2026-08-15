# CONTRATO DE CAPÍTULO — Vol.II Cap. 20: El motor de ejecución (modelo Volcano)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap20_volcano.rs` (2.446 líneas, 37 tests
> en `tests_executor`, ALL_GREEN). Decisiones y bugs reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §24; hito CLI que lo hace
> demostrable: §25 (binario `liradb`, `demo`/`query`). Este capítulo CIERRA el hito
> del brief «ejecutar consultas completas desde texto» (línea 28 de
> `manuscrito/vol2/tabla-de-contenidos.md`) y SIEMBRA el discurso del optimizador
> (cap. 21).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: `Value`/`Node`/`Edge` y `has_label` (cap. 7); el
  puerto `GraphStore` con `iter_nodes`/`get_node`/`out_edges`/`in_edges` y
  `MemoryStore` (cap. 8); métricas que OBSERVAN mientras el sistema trabaja
  (`hit_ratio` del buffer pool, cap. 13); índices hash/B+tree (cap. 15); AST de
  LiraQL y la promesa de cortocircuito en WHERE (cap. 17); `parse` con `Span`
  (cap. 18); `LogicalPlan` (NodeScan/IndexSeek/Expand/Filter/Project/
  CartesianProduct), `ScalarExpr`, `Bindings`, `eq_compatible`/`order_compatible`
  y `lower()` que garantiza un `Project` raíz (cap. 19).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «ejecutar
  es recorrer el plan recursivamente y devolver un Vec enorme» — el modelo de
  materialización total: ignora streaming, latencia de primera fila y LIMIT; (2)
  «NULL es false» — en WHERE se descarta, sí, pero `NOT (NULL)` sigue siendo
  NULL: no es falso, es DESCONOCIDO (lógica trivalente); (3) «un iterador se
  puede volver a leer» — Volcano es monotónico: nadie rebobina (por eso el
  cartesiano materializa); (4) «close es opcional si next se agota» — el estado
  de una ejecución abortada queda vivo (fuga); (5) «comparar nodos es comparar
  sus propiedades» — la igualdad de nodos es IDENTIDAD de id (si no,
  `WHERE a = b` mentiría sobre self-loops).
- **NO debe saber todavía**: reescritura de planes, push-down de filtros,
  `NodeScan→IndexSeek`, estimación de cardinalidades (cap. 21, que ya existe en el
  workspace como `cap21_optimizador` y se cita como «luego»); operadores
  vectorizados/columnares (cap. 38); joins reales WCOJ (cap. 39); paralelismo
  (exchange/morsels, nombrado sin explicar); ORDER BY/agregación (fuera de
  LiraQL mini). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: motor de ejecución; modelo Volcano/iterator (pull, demand-driven);
  tríada `open`/`next`/`close`; operador físico (`PhysicalOperator`: open, next,
  close, name, rows_produced, collect_metrics); `Row` como secuencia de ligaduras
  `(variable, Cell)`; `Cell = Scalar(Value) | Node | Edge`; `eval_scalar` con
  semántica NULL SQL/Cypher y lógica trivalente CON cortocircuito observable;
  igualdad de nodos/aristas por identidad de id; `Executor` (ciclo
  open→next*→close con close SIEMPRE); `compile()` LogicalPlan→árbol físico 1:1;
  `ResultSet` con `Display` tabular; `ExecMetrics` en pre-orden (semilla del
  `explain`); `ExecError` sin `Span` envolviendo `Parse`/`Plan`; `run(src,store)`
  y `Query::execute` (el hito); `demo_graph()` como API pública.
- `practice`: `ScalarExpr` y `Projection::output_name()` (cap. 19); `Value` y
  propiedades schemaless ausentes (cap. 7); adyacencia por dirección
  (`out_edges`/`in_edges`, cap. 8); iteradores perezosos de Rust; errores tipados
  con `Display`/`Error`/`From`.
- `consolidate`: «derivar, no llevar en la cabeza» (métricas reales, no
  supuestas); puertos hexagonales (`&dyn GraphStore`, no `MemoryStore` concreto);
  la cadena texto→tokens→AST→plan→filas como UN pipeline.
- `out_of_scope` (solo nombrar): `optimize`/`Catalog` del cap. 21 (ya invocado
  por `Query::execute` en el código final, con equivalencia verificada);
  vectorización (cap. 38); LIMIT/DISTINCT como keywords de la gramática (caps.
  17-18 no las exponen: los operadores se usan programáticamente hasta la CLI
  del cap. 31); paralelismo (exchange de Volcano'89, morsels).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) enuncia la tríada `open/next/close` y dice qué hace cada
  parte y QUIÉN limpia si el consumidor aborta (el `close` SIEMPRE del
  `Executor`, como un defer); (2) explica por qué pull-based y no materializar,
  con el escenario `LIMIT 1` sobre 1M de filas (Volcano: el scan produce 1;
  materializador: produce 1M y tira 999.999); (3) explica por qué
  `CartesianProductOp` materializa el lado derecho en `open()` (Volcano no
  rebobina) y qué coste paga (memoria + filas de más antes del filtro); (4)
  reproduce las tablas trivalentes de AND/OR y predice `WHERE p.missing = 1` (0
  filas) y `WHERE NOT p.missing = 1` (también 0); (5) lee `ExecMetrics` en
  pre-orden y señala la ineficiencia que el optimizador del cap. 21 atacará.
- **Skills**: (1) ejecutar `liradb query "MATCH (p:Person) WHERE p.age < 40
  RETURN p.name, p.age"` y leer la tabla, y `liradb demo` para ver
  plan+tabla+métricas; (2) componer operadores a mano (`LimitOp::new(compile(
  plan,store), 2)`) y drenarlos con el ciclo Volcano completo; (3) escribir tests
  de ejecución sobre `demo_graph()`.
- **Wisdom**: (1) decide cuándo el modelo iterador es suficiente y cuándo duele
  (tuple-at-a-time vs vectorizado: el puente al cap. 38); (2) reconoce que las
  métricas REALES por operador son la única base honesta para optimizar (no
  estimar a ojo).

## 4. Modelo mental

- **La cadena de montaje**: cada estación pide la pieza a la anterior SOLO
  cuando la necesita (pull/demand-driven); nadie almacena el almacén entero. El
  `Project` es la última estación (la que entrega al cliente), el `NodeScan` es
  la primera (la que va al almacén = `GraphStore`); `Filter` y `Expand` son
  estaciones intermedias que transforman o descartan. Si el cliente (la raíz)
  deja de pedir, la cadena entera se detiene — eso ES un `Limit`.
- **Diagramas ASCII**: (a) el pipeline de la consulta canónica con las flechas
  de petición `next()` subiendo y las `Row` bajando; (b) el árbol
  `LogicalPlan→operadores` (traducción 1:1 de `compile`); (c) el cartesiano
  materializando el lado derecho (un solo `Vec<Row>` re-leído por puntero);
  (d) `LIMIT 1` sobre 1M: pull vs materialización.
- **Momento ¡ajá!**: «la consulta no se CALCULA; se VA PIDIENDO». Nadie produce
  una fila que el consumidor no haya pedido — y por eso un `Limit(1)` apaga un
  escaneo de un millón de filas tras la primera.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap20_volcano.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Iteradores pull (`next` devuelve `Option<Row>`) | Streaming: la primera fila sale sin esperar a la última; `Limit` corto-circuita el pipeline DE RAÍZ (con `LIMIT 1` sobre 1M de filas, el scan produce 1) | Materializar cada operador en `Vec`: latencia de primera fila = trabajo total; memoria O(n) por operador; LIMIT tira filas ya calculadas | Consulta `LIMIT 1` que tarda lo mismo que devolver todo | Graefe, Volcano, TKDE 1994; CMU 15-445 («iterator model… almost every row-based DBMS»); test `limit_corta_el_pipeline` (NodeScan: 2 con Limit 2) |
| 2 | Tríada `open`/`next`/`close` (y no sólo `next`) | `open` reserva/posiciona (cursor del scan, materialización del cartesiano) y RESETEA para re-ejecutar; `close` libera y se propaga a los hijos (idempotente). El `Executor` cierra SIEMPRE, también tras error — como un defer | Sólo `next`: ¿quién construye cursores? ¿quién limpia si el consumidor aborta a mitad? Estado vivo entre consultas (fuga) | El close que nunca llega: cursores/`Vec` de una ejecución abortada quedan retenidos; re-ejecución duplica filas | Trait `PhysicalOperator` (doc-comment); `Executor::execute` (`close` tras el loop, `drained?`); tests `nodescan_ciclo_open_close_reopen`, `distinct_deduplica_filas` (re-open) |
| 3 | Cada operador es un STRUCT que implementa el MISMO trait | Componibilidad: el plan del cap. 19 se traduce a un árbol que se enchufa (`input: Box<dyn PhysicalOperator + 'a>`); `compile()` es 1:1 y el cap. 21 reescribirá el plan ANTES, no el motor | Un `match` gigante en un executor monolítico: cada operador nuevo toca el motor entero; imposible componer Limit/Distinct a mano | El motor deja de crecer; el lector no puede experimentar con operadores propios | `compile()`; `collect_metrics()` recursivo por defecto del trait; MIGRATION-PATTERN §24.5 |
| 4 | `CartesianProductOp` MATERIALIZA el lado derecho en `open()` | Volcano es monotónico: un operador no puede rebobinar su input, y el producto necesita re-leer el lado derecho por cada fila izquierda. Contraste honesto: memoria + filas de más antes del filtro — el «antes» que el cap. 21 elimina reordenando | Re-abrir el lado derecho por fila: `open` tras `close` reinicia estado, pero acoplaría el ciclo de vida al interno y rompería la idempotencia documentada | Producto que devuelve la primera combinación y luego `None` (o filas duplicadas) | `CartesianProductOp::open` (bucle next→Vec, close del derecho); MIGRATION-PATTERN §24.6 y lección 3 |
| 5 | `eval_scalar` con semántica NULL SQL/Cypher | El estándar de facto (SQL ISO y openCypher): cualquier comparación con NULL da NULL; en schemaless, propiedad ausente = NULL (cap. 7). Sin ella, `p.missing > 30` sería false o un error — ambos mienten | Semántica de Rust (`Option` o pánico): incompatibilidad con 40 años de SQL y con Cypher; consultas «que funcionan» hasta que falta un dato | Filas descartadas o errores donde el estándar dice «desconocido» | Tests `eval_comparaciones_null_promocion_y_tipos`, `hito_where_con_null_no_pasa_nada`; openCypher/SQL three-valued logic |
| 6 | Igualdad de nodos/aristas por IDENTIDAD de id | `WHERE a = b` es el predicado «mismo nodo»: encuentra self-loops (Dani→Dani). Igualdad de VALOR (comparar props) diría que Ana y Dani (ambos 36… y hasta dos nodos idénticos) son «el mismo» — mentir | Igualdad estructural de propiedades: ambigua (¿qué props?), O(props) por comparación y semánticamente falsa | Self-loops perdidos; nodos distintos «iguales» | `eq_cells` (`x.id == y.id`); test `eval_igualdad_de_nodos_por_identidad` + `hito_self_loop_con_igualdad_de_nodos` |
| 7 | Cortocircuito REAL en AND/OR (`FALSE AND x` no evalúa `x`) | Prometido en caps. 17 y 19; la rama elidida no se evalúa — OBSERVABLE cuando habría errado (`TRUE AND p.age` con `age` INT → `TypeMismatch`; `FALSE AND p.age` → FALSE) | Evaluar ambos lados siempre: errores fantasma en datos NULL/sin tipar y trabajo innecesario | Predicados que explotan sobre filas que ni les incumben | `eval_scalar` (ramas And/Or); test `eval_cortocircuito_real` (MIGRATION-PATTERN §24-lección 2) |
| 8 | `Row = Vec<(String, Cell)>` con `bind`/`get`/`merge` | Es la materialización EN EJECUCIÓN de los `Bindings` del cap. 19: el scan crea, el expand extiende (clona: materialización explícita), el cartesiano concatena, el project re-liga a columnas. UN tipo de fila ⇒ trait uniforme (no dos jerarquías binding/output) | Array posicional sin nombres: barato pero el binder debería garantizar offsets frágiles; `HashMap`: orden de ligadura perdido (RETURN p, p y Display lo necesitan) | Columnas mal alineadas; `RETURN p, p` imposible de conservar | MIGRATION-PATTERN §24.1; tests `row_bind_get_merge_y_display`, `project_nombres_columnas_y_celdas` |
| 9 | El motor va contra `&dyn GraphStore` (no contra `MemoryStore`) | Hexagonal (cap. 8): el motor es código de DOMINIO; hoy `MemoryStore`, mañana el store en disco de la Parte III SIN tocar el motor | Acoplarse a `MemoryStore`: reescribir el motor al persistir; imposible testear con stores falsos | Motor atado a memoria que hay que reescribir | Firma de `compile`/`run`/`Query::execute`; MIGRATION-PATTERN §24.7 |
| 10 | Métricas por operador (`name`, `rows_produced`, `collect_metrics` pre-orden) | `ExecMetrics` cuenta lo que DE VERDAD fluyó (`NodeScan: 4 | Filter: 1`): la semilla del `explain` con cardinalidades reales del cap. 21 — el eco del `hit_ratio` del cap. 13 (observar mientras se trabaja) | Sin métricas: optimizar a ojo; «creo que el scan es el problema» | Optimizador sin evidencia; ineficiencias invisibles | `ExecMetrics` + `Executor::metrics`; tests `executor_metricas_por_operador`, `executor_metricas_del_cartesiano`, `limit_corta_el_pipeline` |
| 11 | `IndexSeekOp` recibe los NodeIds «desde fuera» | La SELECCIÓN del índice es decisión del OPTIMIZADOR (cap. 21: `Filter(name="Ana")+NodeScan → IndexSeek`); el operador sólo ejecuta. ID inexistente = `UnknownNode` (índice desactualizado) | Que el operador consulte índices: mezcla ejecución con planificación y esconde la decisión de coste | Motor que decide por sí mismo qué índice usar | Doc-comment de `IndexSeekOp`; test `indexseek_ids_exactos_y_error_stale`; MIGRATION-PATTERN §24.5 |
| 12 | `LimitOp`/`DistinctOp` sin keyword en la gramática | LiraQL mini (caps. 17-18) no expone LIMIT/DISTINCT: los operadores quedan listos (programáticos, como los tests) para la CLI del cap. 31 y el optimizador | Ampliar la gramática aquí: cambia el alcance de DOS capítulos anteriores | Keywords a medias o operadores que nadie puede ejercitar | Doc-comments; tests `limit_corta_el_pipeline`, `distinct_deduplica_filas` |
| 13 | `compile()` 1:1 sin reescrituras | Deliberado: el cap. 21 inserta push-down/IndexSeek/reordenación ANTES de compilar. El `Filter` alto que produce `lower()` se ejecuta tal cual y las métricas NUMERAN su ineficiencia (4 escaneadas para 1 devuelta) — el mejor material didáctico | Optimizar en `compile`: enterró la frontera planificación/ejecución y complica el cap. 21 | El lector nunca VERÍA el problema que motiva el optimizador | MIGRATION-PATTERN §24.8; test `executor_metricas_por_operador` |
| 14 | `Executor::new` exige `Project` raíz (`NotAProjection`) | Las columnas del `ResultSet` salen de sus `Projection::output_name()`; es la invariante que `lower()` ya garantiza (cap. 19). Mejor fallar aquí con un error claro que devolver columnas vacías | Aceptar cualquier raíz: ¿de dónde salen las columnas? Silencio o «columns: []» | Resultados sin cabecera; errores lejanos al origen | `Executor::new`; test `executor_rechaza_plan_sin_project_raiz` |
| 15 | `ExecError` SIN `Span`, envolviendo `Parse`/`Plan` con `From`+`source()` | La ejecución opera sobre el plan ya resuelto: un span de TEXTO no significaría nada aquí. El pipeline entero reporta con UN tipo (`run`) sin perder la causa raíz | Reutilizar `QueryError`/`PlanError` con spans: mentir sobre el origen; o `String`: imposible `match` | Errores de ejecución con posiciones absurdas o sin tipar | `ExecError` (doc-comment); test `hito_errores_parse_plan_y_runtime` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: una función `eval_plan(plan, store) -> Vec<Vec<Value>>` recursiva
  que materializa CADA operador en un `Vec` completo (escanear todos los nodos →
  filtrar la lista → proyectar la lista). Sobre `demo_graph()` (6 nodos)
  funciona y los tests incluso pasarían.
- **Qué la rompe**: (a) `LIMIT 1` sobre 1M de filas: calcula el millón entero
  y descarta 999.999 — latencia de primera fila = trabajo total; (b) memoria:
  cada operador intermedio duplica el resultado en un `Vec`; (c) abortar a
  mitad (error en la fila 3 de un millón) deja listas gigantes ya construidas;
  (d) `RETURN p` no cabe en `Vec<Value>`: falta el nodo ENTERO (necesitamos
  `Cell`); (e) sin ciclo de vida no hay re-ejecución ni liberación ordenada.
- **Evolución visible en el capítulo**: la tríada `open/next/close` como trait
  (`PhysicalOperator`), 8 structs-operadores componibles, `Row`/`Cell` como
  moneda común, `Executor` con el ciclo garantizado (close SIEMPRE), `compile`
  1:1 desde el `LogicalPlan` y `run(src, store)` como hito de texto-a-filas.

## 7. Prueba de fuego

- **El HITO del libro**: `cargo run -p liradb-cli -- query "MATCH (p:Person)
  WHERE p.age < 40 RETURN p.name, p.age"` imprime la tabla (Ana/Carla/Dani) —
  texto → tokens → AST → plan → operadores → filas, sin escribir una línea de
  Rust. `liradb demo` añade plan + métricas por operador (MIGRATION-PATTERN
  §25). En librería: `run(src, &store)` y `Query::execute(&store)`.
- **Tests reales del módulo** (se citan, no se duplican): `row_bind_get_merge_y_display`,
  `cell_display_y_type_name`, `eval_literales_variables_y_propiedades`,
  `eval_comparaciones_null_promocion_y_tipos`, `eval_igualdad_de_nodos_por_identidad`,
  `eval_logica_trivalente_completa`, `eval_cortocircuito_real`,
  `eval_errores_defensivos`, `nodescan_todos_con_label_y_orden`,
  `nodescan_ciclo_open_close_reopen`, `indexseek_ids_exactos_y_error_stale`,
  `expand_outgoing_filtra_por_tipo`, `expand_direcciones_incoming_y_undirected`,
  `expand_liga_variable_de_relacion`, `expand_error_si_from_no_es_nodo`,
  `filter_pasa_true_y_descarta_false_y_null`, `filter_error_no_booleano_en_ejecucion`,
  `project_nombres_columnas_y_celdas`, `cartesian_product_materializa_y_cruza`,
  `limit_corta_el_pipeline`, `distinct_deduplica_filas`,
  `hito_consulta_canonica_del_brief`, `hito_match_solo_y_variedad_de_return`,
  `hito_where_and_or_not`, `hito_where_con_null_no_pasa_nada`,
  `hito_props_inline_y_props_de_arista`, `hito_patrones_direccionales`,
  `hito_camino_de_dos_tramos_con_anonimo_intermedio`,
  `hito_self_loop_con_igualdad_de_nodos`, `hito_label_inexistente_vacio`,
  `hito_query_execute_coincide_con_run`, `hito_errores_parse_plan_y_runtime`,
  `exec_error_display_y_from`, `executor_rechaza_plan_sin_project_raiz`,
  `executor_metricas_por_operador`, `executor_metricas_del_cartesiano`,
  `result_set_display_tabla_y_column` (37, ALL_GREEN).
- **Síntoma si el lector se salta el capítulo**: tiene un `LogicalPlan` bonito
  que no produce NINGUNA fila; consulta la API de `GraphStore` a mano para cada
  pregunta; y cualquier `WHERE` con propiedades ausentes le da resultados
  falsos (false en vez de NULL).

## 8. Trampas y errores comunes

1. **Llamar `next` antes de `open`** (o tras `close`): el contrato dice que se
   agota en silencio (`Ok(None)`), no que reabra. Re-ejecutar = `open` de nuevo.
2. **Tratar NULL como false**: en `Filter` ambos se descartan, pero `NOT NULL`
   es NULL (sigue fuera): «no conocido» ≠ «conocido falso». Test
   `hito_where_con_null_no_pasa_nada`.
3. **Esperar que el cartesiano re-lea el lado derecho gratis**: nadie rebobina;
   el `Vec<Row>` de `open()` ES el coste. (Y comparar por valor dos nodos en
   lugar de por id.)
- **Precisión de lenguaje (glosario)**: *operador lógico* (cap. 19) vs
  *operador físico* (este cap.); *pull/demand-driven* vs *push*; *pipeline* vs
  *materializar*; *ligar* (bind) vs *proyectar*; *fila interna* (variables) vs
  *fila de salida* (columnas); *trivalente* (TRUE/FALSE/NULL); *pre-orden*
  (raíz→hojas); *raíz* del plan; *lado derecho/izquierdo*; *iterador monotónico*.

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval puro)**: SIN mirar el capítulo ni el
  código, escribir de MEMORIA el trait `PhysicalOperator` (las tres firmas de la
  tríada + `name` + `rows_produced`) y responder: si el consumidor aborta tras
  un error en `next`, ¿quién limpia y por qué? Compilarlo contra
  `cap20_volcano.rs` (el nombre real corrige la memoria). Verificación: test
  propio que drene un `NodeScanOp` con el ciclo completo +
  `nodescan_ciclo_open_close_reopen`. Pistas: (1) ¿qué devuelve `next` al
  agotarse?, (2) ¿qué método RESETEA para re-ejecutar?, (3) ¿dónde está el
  `close` del `Executor::execute` respecto al `?` del error? Criterio: firmas
  exactas (incluye `Result` y `Option`) y el orden close-antes-que-drained.
- **analizar (intermedio — spacing + interleaving)**: leer las métricas reales
  de la consulta canónica (`Project: 1 | Filter: 1 | Expand: 4 | NodeScan: 4 |
  filas devueltas: 1`) y: (a) decir qué operador produce filas que nadie
  consume y cuántas; (b) implementar `OffsetOp { skip }` (salta n, luego emite)
  componiéndolo a mano sobre `compile()` como los tests de `LimitOp` —
  incluyendo `collect_metrics` — y predecir sus métricas ANTES de ejecutar.
  Interleaving: obliga a razonar con las métricas del cap. 13 (observar sin
  intervenir) y el `LogicalPlan` del cap. 19 (¿por qué `OffsetOp` NO está en
  `compile()`?). Verificación: patrón de `limit_corta_el_pipeline` +
  `executor_metricas_por_operador`. Pistas: (1) el contrato de `next` tras
  saltar, (2) ¿qué hereda `collect_metrics` del trait?, (3) ¿dónde empezaría a
  contar `rows_produced`? Criterio: predicción de métricas ACERTADA antes de
  ejecutar + operador re-ejecutable tras close.
- **crear (experto — cierre del hito)**: escribir `run_materializando(plan,
  store) -> ResultSet` (el intérprete de la §6: recursivo, cada operador un
  `Vec<Row>` completo, SIN el trait) y un test de EQUIVALENCIA contra el
  `Executor` en ≥4 consultas del grafo demo (mismas columnas, mismas filas
  ordenadas). Luego: envolver ambas rutas en `Limit(1)`-sobre-entrada-grande
  (generar 1.000 personas) y mostrar con métricas que el Volcano escanea 1
  fila y el materializador escanea 1.000. Pistas: (1) el cartesiano del
  materializador NO necesita materializar el lado derecho (¿por qué?), (2)
  ¿dónde contarías filas en el recursivo?, (3) ¿qué dice el §20.5 del
  capítulo sobre la latencia de la primera fila? Criterio: equivalencia
  demostrada + la cifra 1-vs-1000 explicada con el pull de raíz.

## 10. Preguntas abiertas (gancho al capítulo 21)

1. Las métricas muestran 4 filas escaneadas para devolver 1: ¿quién decide
   EMPEZAR por el nodo con filtro (o por su índice del cap. 15) en lugar de
   escanear? (Nace `optimize`.)
2. `IndexSeekOp` existe pero nadie lo elige: ¿qué información necesita un
   optimizador para saber que `name = "Ana"` selecciona 1 fila de 4?
   (Estadísticas/catálogo.)
3. ¿Cómo se hace esa decisión VISIBLE para el humano que depura? (Nace
   `liradb explain`: plan estimado vs métricas reales de este capítulo.)
- **Términos nuevos de glosario**: motor de ejecución, modelo Volcano/iterator,
  pull (demand-driven), pipeline, materializar, operador físico, tríada
  open/next/close, iterador monotónico, lógica trivalente, cortocircuito,
  identidad (de nodo), fila ligada, pre-orden, ResultSet, close garantizado.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial reconstruye el trait DESDE LA MEMORIA (el
  enunciado no muestra ni una firma); el intermedio predice métricas antes de
  ejecutar; el experto reconstruye la solución ingenua para contrastarla.
- **Spacing**: esencial → ciclo de vida y errores (caps. 15/19: errores tipados
  con `From`); intermedio → métricas observacionales (cap. 13 `hit_ratio`) y
  `LogicalPlan`/`compile` (cap. 19); experto → `GraphStore` (cap. 8),
  `ScalarExpr` trivalente (caps. 7/17/19) y el fixture `demo_graph`.
- **Interleaving**: el intermedio mezcla diseño de operador + lectura de
  métricas + plan lógico; el experto mezcla evaluación, equivalencia semántica
  y coste de ejecución.
- **Dificultad asimétrica**: una idea nueva por sección (fila → tríada →
  operadores → cartesiano que materializa → métricas → hito); los ejercicios
  exigen recuperación y predicción, no reconocimiento.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap20` (37 tests
  citados por nombre) y `cargo run -p liradb-cli -- demo|query "..."`.
- **Citas**: Graefe, «Volcano—An Extensible and Parallel Query Evaluation
  System», IEEE TKDE 6(1):120-135, 1994 (DBLP/SIGMOD Anthology); Graefe,
  «Encapsulation of Parallelism in the Volcano Query Processing System», SIGMOD
  1989 (exchange); Graefe, «The Cascades Framework», IEEE DE Bull. 1995 →
  optimizador de SQL Server (Microsoft Research); CMU 15-445 (iterator model);
  Boncz/Zukowski/Nes, «MonetDB/X100», CIDR 2005 (vectorización, puente cap.
  38); Raasveldt/Mierle, DuckDB, SIGMOD 2020 (push + morsels); Leis et al.,
  «Morsel-Driven Parallelism», CIDR 2014.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (15 en la tabla §5).
- [x] Escenario de fallo visible: materializar todo con LIMIT 1; el close que nunca llega (fuga); NULL que se propaga (NOT NULL sigue fuera).
- [x] Código ejecutable en workspace (37 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (materializar todo ≠ ejecutar; NULL ≠ false; rebobinar ≠ re-open; comparar nodos ≠ comparar props).
- [x] Ejercicios con solución verificable (`cargo test` + CLI).
- [x] ≥1 ejercicio de retrieval (trait desde memoria) y ≥1 de spacing (métricas cap. 13, plan cap. 19, store cap. 8).
- [x] Responde las preguntas críticas: por qué pull y no materializar, por qué open/next/close, por qué structs+trait, por qué el cartesiano materializa, por qué NULL SQL/Cypher, por qué identidad de id, por qué cortocircuito observable, por qué Row=(String,Cell), por qué &dyn GraphStore.
- [x] Anécdota verificada: Graefe/Volcano 1994 (TKDE) con herencia en Cascades/SQL Server y en los motores modernos; fuentes DBLP/SIGMOD/CMU/MSR.
- [x] El hito se celebra: `run(src,store)`, `Query::execute` y `liradb query|demo` con salida real capturada.
