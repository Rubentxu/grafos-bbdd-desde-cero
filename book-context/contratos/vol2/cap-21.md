# CONTRATO DE CAPÍTULO — Vol.II Cap. 21: Un optimizador pequeño pero real (`liradb explain`)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap21_optimizador.rs` (2.165 líneas,
> 30 tests en `tests_optimizer`; colateral: cap 19 ganó `IndexSeek` y cap 20 su
> `IndexSeekOp`). Decisiones reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md`
> §26 (y §24: las métricas del cap. 20 ya numeraban la ineficiencia 4-para-1).
> Este capítulo CIERRA la Parte IV: repaso-retrieval de la cadena 17→18→19→20→21
> y sección «estadísticas y estimación de cardinalidad» (refuerzo ADR-005, línea 34
> de `manuscrito/vol2/tabla-de-contenidos.md`).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: LiraQL MATCH-WHERE-RETURN y su AST (caps. 17-18);
  `LogicalPlan` con `NodeScan`/`Expand`/`Filter`/`Project`/`CartesianProduct`,
  binder con variables únicas y `bound_variables()` (cap. 19); motor Volcano
  open/next/close, `compile` 1:1, `Executor`, `ResultSet` y métricas por operador
  (cap. 20); índices hash/B+tree en disco y por qué existen (cap. 15); `&dyn
  GraphStore` como puerto de store (cap. 8); `liradb demo|query` (hito CLI).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «un
  optimizador hace la consulta más rápida cambiando la consulta» — no: reescribe
  el PLAN, la consulta del usuario es intocable; (2) «optimizar = buscar el plan
  de coste mínimo» — no con n! órdenes de joins: reglas + estimaciones bastan
  para ordenar; (3) «si la estimación no coincide con las filas reales, es un
  bug» — falso: la estimación sirve para ORDENAR planes, no para prometer
  costes (est. 1 vs 3 reales es contenido, no bug); (4) «el optimizador decide
  con constantes mágicas» — falso: el catálogo MIRA el grafo (4 Persons, grados
  medios 1.50/1.00, fracción KNOWS 4/6); (5) «un plan distinto puede dar
  resultados distintos» — sin `ORDER BY` el orden no es parte del contrato, pero
  el multiconjunto de filas SÍ (testado por equivalencia).
- **NO debe saber todavía**: optimizadores de coste exhaustivo (Cascades/Columbia),
  histogramas y muestreo reales, `EXPLAIN ANALYZE` con tiempos por operador,
  reordenación de joins con programación dinámica completa, adaptative query
  execution, ORDER BY/LIMIT desde gramática (cap. 31), estadísticas persistentes
  incrementales. Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: optimizador como REESCRITOR de planes; `Catalog`/`LabelStats`
  (nodos por etiqueta, grados medios out/in, aristas por tipo, índice de
  igualdad (etiqueta, propiedad, valor)→ids); selectividad por tipo de predicado
  (`SEL_EQ=0.1`, `SEL_RANGE=1/3`, `SEL_NOT_EQ=0.9`, `SEL_UNKNOWN=0.5`) y
  composición AND/OR/NOT; estimación de cardinalidad por operador (`estimate`);
  cinco reglas en orden fijo (R1 punto inicial selectivo, R2 predicate pushdown
  respetando bindings, R3 absorber `HasLabel` en el escaneo, R4 `NodeScan`→
  `IndexSeek` sólo si ahorra, R5 poda de proyecciones); equivalencia pre/post
  como multiconjunto ordenado; `explain` con plan ANTES/DESPUÉS + estimaciones +
  filas reales.
- `practice`: plan lógico y `Display` (cap. 19); ejecución Volcano y métricas
  (cap. 20); `IndexSeek` como plan y como operador (caps. 19-20); parser/lowerer
  (caps. 18-19); propósito de un índice (cap. 15).
- `consolidate`: «derivar, no llevar en la cabeza» (el catálogo deriva del
  store); separación parser/planner/ejecutor; el store como puerto.
- `out_of_scope` (solo nombrar): coste exhaustivo y DP de joins, Cascados,
  histogramas/muestreo, `EXPLAIN ANALYZE` con timings, optimización adaptativa,
  `ORDER BY` (que fijaría el orden de filas), catálogo persistente.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica por qué se optimiza el PLAN y no la consulta (el
  AST es del usuario; el plan es nuestra representación interna, reordenar
  ligaduras es invisible para él); (2) calcula a mano la estimación de una
  cadena (scan × grados × fracción de tipo × selectividad) con los números del
  demo (4 × 1.5 × 4/6 = 4; 4 × 1/3 → est. 1); (3) enumera las 5 reglas EN ORDEN
  y dice por qué ese orden (R1 decide por dónde empezar; el resto cuelga
  predicados del plan resultante); (4) distingue estimación (heurística, para
  ordenar) de ejecución (verdad) y lee la discrepancia est. 1 vs 3 reales; (5)
  dice qué garantiza el contrato de equivalencia y qué no (multiconjunto sí,
  orden no, sin ORDER BY).
- **Skills**: (1) ejecutar `liradb explain "..."` y leer catálogo, ANTES,
  DESPUÉS y filas reales; (2) predecir el plan DESPUÉS de una consulta antes de
  ejecutar el explain; (3) escribir un test de equivalencia ingenuo-vs-optimizado
  con `filas_ordenadas`.
- **Wisdom**: (1) decide cuándo UNA regla no debe aplicar (R4 con `ids.len() ≥
  scan_rows` no ahorra; R5 no fusiona `Project` que transforman); (2) decide
  cuándo heurísticas simples bastan y cuándo un sistema necesita muestreo
  (ordenar 2-5 planes vs prometer costes en catálogos sesgados).

## 4. Modelo mental

- **El GPS**: misma origen-destino (la consulta), pero el planificador elige el
  ORDEN de las calles según el tráfico conocido (las estadísticas del catálogo);
  llega al mismo sitio (equivalencia) por el camino que hoy cuesta menos. El
  usuario dicta el destino; el GPS decide la ruta. R2 (pushdown) es «no recorras
  la autopista para dejar un paquete en la esquina de al lado».
- **Diagramas ASCII**: (a) plan ANTES vs DESPUÉS del demo real (Filter encima
  vs pegado al escaneo y cadena invertida); (b) la cadena de 5 eslabones del
  pipeline Parte IV (texto→tokens→AST→plan→operadores→filas con optimize
  insertado); (c) el catálogo como tabla (Person 4, grados, tipos) junto al
  grafo mini que la originó.
- **Momento ¡ajá!**: «el plan lógico no dice CÓMO ejecutar: dice QUÉ. El
  optimizador es la primera capa que ELIGE entre varios QUÉ equivalentes, y
  para elegir necesita MIRAR los datos».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap21_optimizador.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Optimizar el PLAN, no reescribir la consulta (AST) | El AST es del usuario (fiel a lo que escribió); el plan es nuestra IR. Reordenar LIGADURAS es invisible para él y `Project` re-evalúa por nombre | Reescritor de consultas sobre AST: rompería la fidelidad parse→AST, duplicaría el binder y complicaría probar equivalencia | El usuario ve «su» consulta cambiada; errores de parse ajenos a su texto | Contrato de equivalencia del banner del módulo (líns. 52-57); `Project` re-evalúa por NOMBRE |
| 2 | REGLAS en orden fijo, no búsqueda por coste exhaustivo | n! órdenes de joins: con 10 joins son 3,6 millones; System R ya lo sabía (DP sólo con pocos joins, árboles left-deep). 5 reescrituras totales cubren el 80 % de la ganancia | Enumeración exhaustiva con función de coste: combinatoria inabordable y difícil de enseñar/verificar | Tiempo de planificación > tiempo de ejecución; planes irreproducibles | Selinger et al. 1979 (§join ordering, DP con left-deep trees); `optimize` (líns. 650-656) |
| 3 | El ORDEN de las reglas está documentado y es fijo | Determinismo didáctico: mismo input → mismo output. R1 decide por dónde empezar (necesita el plan tal cual lower lo dejó); R2 cuelga predicados del plan resultante; R3/R4 pulen el escaneo; R5 limpia | Reglas iteradas hasta fijación o en orden aleatorio: planes no reproducibles, tests y explain inestables | Explicaciones que no se pueden enseñar ni testear | Doc de `optimize` (líns. 639-649); test `optimizar_es_idempotente_y_conservador` |
| 4 | R2 (predicate pushdown) es LA regla reina: filtrar antes de expandir | Un `Filter` encima del `Expand` paga el filtro por CADA fila expandida; pegado al escaneo, las filas que no pasan ni entran al pipeline. Demo: 4 escaneadas para devolver 1 (cap. 20 §24.8) | Filtrar tarde (como lower): en un grafo de 10M nodos el 4-para-1 se vuelve 40M de filas muertas | Trabajo O(n) tirado a la basura fila a fila | Métricas reales del demo (NodeScan 4 · devueltas 1); explain real del capítulo; Selinger 1979 («predicates nested as deeply as possible») |
| 5 | Pushdown respeta `bound_variables()` (fronteras: NodeScan/IndexSeek/Project; átomos con `to` se quedan arriba) | Bajar un átomo que menciona una variable que aún NO está ligada es cambiar la semántica: el ejecutor no la encontraría (o la evaluaría contra otra fila) | Empujar ciegamente todo el AND: resultados distintos pre/post — la definición de bug en un optimizador | Filas incorrectas sin error visible (peor que un panic) | `sink` (líns. 887-954) usa `bound_variables` (cap. 19); test `pushdown_no_cruza_project_ni_fusiona_filtrables` |
| 6 | Estimación por HEURÍSTICAS simples (SEL_EQ 0.1, SEL_RANGE 1/3…), no muestreo | Las estimaciones sólo necesitan ORDENAR candidatos (coste de empezar por f vs por p); para eso bastan defaults clásicos + grados medios. La discrepancia est. 1 vs 3 reales es contenido pedagógico, no bug | Muestreo/histogramas: infraestructura desproporcionada para 6 nodos y para ordenar 2-5 planes | Complejidad que oculta la idea (comparar planes, no acertar filas) | Selinger 1979 (defaults 1/10 y 1/3); consts. líns. 331-337; test `explain_la_consulta_del_reordenado` (est. 1, reales 3) |
| 7 | Estadísticas del STORE (`Catalog::collect`), no constantes mágicas | Elegir punto inicial exige comparar grados reales: Person out 1.50/in 1.00 y fracción KNOWS 4/6 cambian el ganador. Constantes a ciegas elegirían siempre el mismo lado | Umbrales fijos sin mirar datos: el «optimizador» sería una regla de estilo, no una decisión informada | Reordenar hacia el lado CARO del grafo: plan «optimizado» peor que el ingenuo | `Catalog::collect` (líns. 144-185); test `reordenar_empieza_por_el_lado_selectivo`; explain real del capítulo |
| 8 | El optimizador elige `IndexSeek`; el cap. 20 lo dejaba al caller | La decisión («¿existe un índice que AHORRE?») necesita `ids.len() < scan_rows` — datos del catálogo. El operador ejecuta; el optimizador decide: separación de responsabilidades | Caller/el propio `IndexSeekOp` adivinando: cada llamador duplicaría heurísticas y ninguna tendría estadísticas | Índices usados donde perjudican (seek que devuelve más filas que el scan) | MIGRATION §24.5 (selección difiere al cap. 21); R4 guarda `if ids.len() < scan_rows` (líns. 1049-1051); test `index_seek_no_aplica_si_no_ahorra_ni_con_rangos` |
| 9 | Tests de EQUIVALENCIA pre/post (multiconjunto ordenado) | Un optimizador que cambia RESULTADOS es un bug, no una optimización. Sin ORDER BY el orden no es contrato (exactamente como SQL), así que se comparan columnas + filas ordenadas | Comparar ResultSet a secas (orden-dependiente) o no comparar: equivalencia asumida, nunca probada | Regresión silenciosa: la consulta «va más rápido» y devuelve otra cosa | Test `equivalencia_antes_y_despues_sobre_bateria_de_consultas` (12 consultas); `equivalencia_run_pasa_por_el_optimizador` |
| 10 | Catálogo reconstruido por consulta (una pasada), sin persistir | Coste: un escaneo; a cambio, un catálogo obviamente correcto del que razonar. En un sistema real viviría en disco y se mantendría incrementalmente (los índices del cap. 15 son la infraestructura natural) | Persistencia incremental ya: invalidación, crashes a mitad de actualización — tema de caps. futuros, no de hoy | Estadísticas podridas que el optimizador cree frescas | Doc de `Catalog` (líns. 113-123); MIGRATION §26.1 |
| 11 | R4 con valor ausente produce `IndexSeek` con 0 ids (cortar de raíz) | «Zoe» no existe: la reescritura más rentable de todas es NO ejecutar nada, y también la correcta (0 filas, columnas intactas) | Dejar el scan+filter «por si acaso»: escaneo completo para devolver vacío | Trabajo O(n) para nada | Test `index_seek_valor_ausente_corta_de_raiz` |
| 12 | `explain` EJECUTA al final y contrasta est. vs reales | LiraQL es de sólo lectura: ejecutar no tiene efectos. Mostrar la discrepancia enseña el límite de las heurísticas (el lector VE est. 1 vs 3) | Sólo estimaciones: el lector nunca aprende cuánto mienten las heurísticas | Fe ciega en las estimaciones | `explain` (líns. 1191-1217); doctest con «Filas reales…: 1»; salida real del capítulo |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: no hay optimizador — el plan de `lower` se ejecuta tal cual (el
  cap. 20 lo hizo así A PROPÓSITO: `compile` 1:1) y las métricas numeran la
  ineficiencia: `NodeScan: 4 filas · … · filas devueltas: 1`. Versión «naïve de
  optimizador»: reescribir el texto de la consulta o dejar que el caller elija
  `IndexSeek` a mano.
- **Qué la rompe**: (a) el `Filter` encima del `Expand` paga por cada fila
  expandida — 4 escaneadas para devolver 1, y eso escala linealmente a millones;
  (b) la elección de índice repartida en callers nadie la hace con datos; (c)
  reordenar la consulta a mano no escala cuando el patrón tiene 3 nodos y el
  filtro está en el último.
- **Evolución visible**: `optimize(plan, &Catalog::collect(store))` — cinco
  reescrituras totales en orden fijo que transforman el ANTES (Filter encima)
  en el DESPUÉS (cadena invertida empezando por el lado selectivo, filtros
  pegados, `IndexSeek`). `run()`/`Query::execute` lo integran; `explain` lo
  muestra con estimaciones y filas reales.

## 7. Prueba de fuego

- **La salida real** (ejecutada, no inventada) de `liradb explain "MATCH
  (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name"`:
  catálogo (Person 4 · out 1.50/in 1.00 · KNOWS 4, LIVES_IN 2), ANTES con
  `Expand(p, KNOWS, OUTGOING, f) est. 4` y `Filter est. 1`, DESPUÉS con
  `NodeScan(Person AS f)` + `Filter(f.age < 40)` pegado + `Expand(f, KNOWS,
  INCOMING, p)`, y `Filas reales al ejecutar el plan optimizado: 3 (raíz
  estimada: 1)`.
- **Tests citados**: `catalogo_recuentos_del_grafo_demo`,
  `catalogo_grados_medios_por_etiqueta`, `catalogo_indice_de_igualdad`,
  `selectividad_defaults_de_system_r`, `estimacion_scan_filter_expand`,
  `reordenar_empieza_por_el_lado_selectivo`, `pushdown_canonico_del_brief_con_index_seek`,
  `pushdown_divide_el_and_entre_los_lados`, `absorb_integra_etiqueta_y_elimina_filtro`,
  `index_seek_valor_ausente_corta_de_raiz`, `index_seek_no_aplica_si_no_ahorra_ni_con_rangos`,
  `equivalencia_antes_y_despues_sobre_bateria_de_consultas`,
  `optimizar_es_idempotente_y_conservador`, `explain_canonico_con_antes_despues_y_estimaciones`,
  `explain_la_consulta_del_reordenado`.
- **Síntoma si el lector se salta el capítulo**: las consultas escanean todo el
  grafo siempre (los índices del cap. 15 siguen sin que nadie los use), los
  filtros cuelgan de la raíz y nadie puede explicar POR QUÉ un plan es mejor
  que otro — la Parte V (algoritmos sobre el grafo) heredaría un motor que
  malgasta por patrón.

## 8. Trampas y errores comunes

1. **Bajar predicados sin mirar bindings**: empujar `f.age < 40` por debajo del
   `Expand` que liga `f` cambia resultados (la trampa #1 real de todo
   pushdown). Síntoma: `pushdown_no_cruza_project_ni_fusiona_filtrables` falla.
2. **Confundir estimación con promesa**: `est. 1 filas` y 3 reales NO es un
   bug; lo sería usar la estimación para RESPONDER en vez de para ORDENAR.
3. **IndexSeek siempre**: sustituir el escaneo cuando el índice devuelve TODAS
   las filas (o más trabajo que el scan) es una «optimización» negativa; R4
   sólo aplica si `ids.len() < scan_rows`.
- **Precisión de lenguaje (glosario)**: *plan lógico* vs *plan físico*;
  *selectividad* (fracción que sobrevive) vs *cardinalidad* (filas estimadas);
  *pushdown* (bajar predicados) vs *reordenación* (elegir punto inicial);
  *estimación* vs *métrica real*; *regla* (reescritura local) vs *búsqueda por
  coste* (enumeración); *equivalencia* (multiconjunto) vs *igualdad* (orden
  incluido); *catálogo* vs *índice* (el catálogo AYUDA a decidir; el índice
  resuelve la búsqueda).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: sin ejecutar nada, predecir el plan DESPUÉS
  de `MATCH (a:Person), (c:City) WHERE a.age > 35 AND c.name = "Madrid" RETURN
  a.name, c.name` (qué regla parte el AND, dónde acaba cada átomo, cuál se
  vuelve IndexSeek y por qué). Verificación: test
  `pushdown_reparte_el_cartesiano_y_busca_indices` + `liradb explain`. Pistas:
  (1) ¿qué variables liga cada lado del cartesiano?, (2) ¿qué forma debe tener
  un átomo para R4?, (3) ¿cuántos Madrid hay bajo City? Criterio: acertar la
  partición del AND y el IndexSeek.
- **analizar (intermedio)**: explicar con números por qué el explain dice
  `raíz estimada: 1` y devuelve 3 (SEL_RANGE=1/3 vs selectividad real 3/4);
  decidir si esa discrepancia podría alguna vez cambiar el ORDEN elegido y
  qué haría falta para afinar la estimación (min/max por propiedad).
  Verificación: `explain_la_consulta_del_reordenado`. Pistas: (1) ¿cuántas
  Person tienen age<40 en el fixture?, (2) ¿usa la estimación para elegir o
  para prometer?, (3) ¿qué estadística distinguiría 1/3 de 3/4? Criterio:
  separar «ordenar planes» de «acertar filas».
- **crear (experto — cierre de Parte IV, retrieval puro)**: reconstruir desde
  memoria la cadena completa `texto → parse → lower → optimize → compile →
  open/next/close → filas` diciendo qué capa añade cada capítulo (17-21) y qué
  invariante garantiza cada una; luego extender el catálogo con min/max por
  (etiqueta, propiedad) y usarlo en `compare_selectivity` para rangos, de modo
  que el explain del demo pase de est. 1 a est. 3 SIN cambiar el plan elegido
  (equivalencia intacta). Verificación: bateria de equivalencia +
  `estimacion_scan_filter_expand` adaptado. Pistas: (1) ¿dónde del `Catalog`
  se acumula un par (min,max)?, (2) ¿qué caso del `match op` toca?, (3) ¿por
  qué el plan ganador no debe cambiar? Criterio: estimación mejorada + MISMO
  plan + tests verdes.

## 10. Preguntas abiertas (gancho al capítulo 22 — abre la Parte V)

1. El optimizador nos da el plan más barato para un PATRÓN fijo… ¿y cuando la
   pregunta es «¿cuál es el camino más corto?» — quién optimiza el RECORRIDO?
   (nacen los algoritmos de grafos: Dijkstra.)
2. ¿Puede un algoritmo como Dijkstra usar el catálogo (grados medios) para
   decidir por dónde expandirse primero? (Siembra de A* del cap. 23.)
3. ¿Qué pasa con estas optimizaciones cuando el grafo NO cabe en memoria?
   (Proyección in-memory del cap. 26.)
- **Términos nuevos de glosario**: optimizador, catálogo, estadísticas,
  selectividad, cardinalidad estimada, predicate pushdown, punto inicial
  selectivo, reordenación de expansión, regla de reescritura, IndexSeek,
  equivalencia de planes, plan ANTES/DESPUÉS.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto reconstruye DESDE LA MEMORIA la cadena de
  la Parte IV (nada del enunciado revela qué capa hace qué); el esencial obliga
  a recordar el ORDEN de las reglas para predecir el plan sin ejecutar.
- **Spacing**: esencial → reglas R2/R4 y cartesiano (este cap.); intermedio →
  heurísticas SEL_* vs fixture del cap. 20; experto → parser (18), binder (19),
  Volcano (20) y los índices del cap. 15 (por fin ALGUIEN los usa: el eq_index
  del catálogo es su primo en memoria); la sección «repaso de la Parte IV»
  re-ejercita la cadena 17→21 completa.
- **Interleaving**: el experto mezcla estimación, reescritura de planes,
  fixtures y equivalencia semántica; el intermedio cruza estadística con
  decisión de orden.
- **Dificultad asimétrica**: una idea nueva por sección (mirar datos → estimar
  → reordenar → bajar predicados → elegir índice → verificar equivalencia);
  los ejercicios exigen recuperación y predicción.
- **Bucle de feedback inmediato**: `cargo run -q -p liradb-cli -- explain "..."`
  y `cargo test -p vol2-liradb` (30 tests citados por nombre).
- **Citas**: Selinger et al., «Access Path Selection in a Relational Database
  Management System», SIGMOD 1979 (defaults 1/10 y 1/3; pushdown y DP de
  joins); PostgreSQL release notes 7.2.0 (2002, EXPLAIN ANALYZE, Martijn van
  Oosterhout); PostgreSQL docs «Using EXPLAIN»; Spark Catalyst (Armbrust et
  al., SIGMOD 2015); Kùzu docs (optimitzador cost-based de grafos); Petrov
  «Database Internals»; CMU 15-445 (query optimization).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: 4-para-1 extrapolado a millones + el pushdown que cruza bindings y rompe resultados (§6-§8 del capítulo).
- [x] Código ejecutable en workspace (30 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (optimizar ≠ cambiar la consulta; estimación ≠ promesa; IndexSeek no siempre; plan distinto ≠ resultados distintos).
- [x] Ejercicios con solución verificable (tests del workspace + explain).
- [x] ≥1 ejercicio de retrieval (cadena Parte IV desde memoria) y ≥1 de spacing (índices cap. 15, métricas cap. 20, binder cap. 19).
- [x] Responde las preguntas críticas: plan vs consulta, reglas vs coste exhaustivo, orden fijo, pushdown reina, heurísticas vs muestreo, store vs constantes, quién elige IndexSeek, equivalencia.
- [x] Sección «estadísticas y estimación de cardinalidad» incluida (TOC línea 34, ADR-005) y repaso-árco de la Parte IV (17→21).
- [x] Anécdota verificada: Selinger et al. 1979 (SIGMOD, ACM DL) y EXPLAIN ANALYZE en PostgreSQL 7.2.0 (release notes oficiales).
- [x] La salida del `explain` usada en el capítulo es la REAL, ejecutada contra el workspace (est. 1 vs 3 reales).
