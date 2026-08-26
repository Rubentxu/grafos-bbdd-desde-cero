# CONTRATO DE CAPÍTULO — Vol.II Cap. 39: Joins, patrones y consultas cíclicas

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **Tercer capítulo de la Parte VIII**
> (crecimiento POST-mapa) y COBRADOR de tres deudas explícitas: cap. 14 («WCOJ sobre
> adyacencias ordenadas (cap. 39)» — el CSR ordenado es la base física), cap. 20 («joins
> reales WCOJ (cap. 39)» — hoy `ExpandOp` expande naive fila a fila) y cap. 38 («el motor
> factorizado completo y joins factorizados = cap. 39», gancho saliente fijado: «las columnas
> aceleran CÓMO lees; los worst-case optimal joins cambian QUÉ calculas»). Código ancla:
> `lib.rs` declara 30 módulos (`cap07_modelo` … `cap38_columnar`); `ExpandOp`
> (cap20_volcano.rs:658) recorre `out_edges`/`in_edges` + `get_edge` POR CANDIDATO, fila a
> fila — es un *index nested-loop join* sin saberlo; el plan lógico solo tiene
> `Expand`/`CartesianProduct` (cap19_plan_logico.rs:573/595) y el optimizador del cap. 21
> (`optimize`, cap21_optimizador.rs:650) reordena CADENAS salto a salto; adyacencias YA
> ordenadas: CSR (cap. 14) y `SubgrafoAcotado::construir` (cap38_columnar.rs:773); semilla
> factorizada a EXTENDER: `ExpansionFactorizada` 2-hop (cap38_columnar.rs:861);
> `BitSet`/`Presupuesto`/`MotivoParada` (cap. 26); BFS/Dijkstra (caps. 22-23); triángulos
> ya presentes en comunidades (cap. 25); dataset determinista y `SEMILLA_REFERENCIA`
> (cap. 34). Estado verificado 2026-08-25: **864 tests** ALL_GREEN; runtime dependency-free
> (dev-deps: tempfile/proptest/criterion 0.7, `harness=false`); toolchain pinneada 1.96.0.
> Código NUEVO previsto: `src/cap39_joins.rs` (~900-1300 líneas, std puro) + CUARTO
> `[[bench]]` `benches/bench_joins.rs` (aditivo) — CERO deps nuevas, CERO cambios en caps.
> 7-38. Citas VERIFICADAS hoy (2026-08-26): AGM = **FOCS 2008** (Atserias-Grohe-Marx;
> versión revista SICOMP 42(4), 2013); NPRR = **PODS 2012** (pp. 37-48); el survey «Skew
> Strikes Back» = **SIGMOD Record 42(4), 2014** (Ngo-Ré-Rudra; NI «PODS 2012» NI «SIGMOD
> 2013»); LeapFrog = **ICDT 2014** (Veldhuizen, LogicBlox; ICDT Test-of-Time 2024);
> factorización = **ICDT 2012** + versión revista **ACM TODS 40(1), 2015** (el contrato del
> cap. 38 citaba «ICDT 2015» — discrepancia detectada aquí y CORREGIDA en cap-38 el
> 2026-08-26); Kùzu = CIDR 2023
> CC-BY 4.0 según ADR-001. Pregunta crítica del CORPUS (`vol-II-cap-39`, Parte VIII):
> «LeapFrog Triejoin simplificado». Gancho saliente: cap. 40 distribución — «ya sabes QUÉ
> calcular y CÓMO leerlo rápido EN UNA máquina; ¿y cuando el grafo no cabe en una?».

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: TODO el edificio — modelo PropertyGraph (7), CSR ordenado
  con su ×16 (14), LiraQL end-to-end: AST/parser/plan lógico (17-19), Volcano fila-a-fila
  con `ExpandOp` (20), optimizador con catálogo y `explain` (21), BFS/Dijkstra/A* (22-23),
  comunidades con triángulos (25), `Presupuesto` y `BitSet` (26), ACID/WAL/MVCC (27-30),
  torre de pruebas (33), medición (34), lectura columnar + factorización 2-hop (38).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «el coste de una consulta lo decide el tamaño de la ENTRADA» — no: puede decidirlo
  el RESULTADO (triángulos = n^1,5) y sobre todo los INTERMEDIOS, que pueden superar a
  entrada y resultado a la vez (SIGMOD Record 42(4), 2014).
  (2) «worst-case optimal significa que va más rápido siempre» — no: es una GARANTÍA
  sobre el PEOR caso (nunca eres polinómicamente peor que el tamaño del resultado); con
  datos selectivos y acíclicos un plan binario clásico suele ganar (Veldhuizen, ICDT 2014).
  (3) «ExpandOp es primitivo, el join es otra cosa» — no: `ExpandOp` ES un index
  nested-loop join nodo×adyacencia; formalizarlo es lo que permite ELEGIR OTRO plan.
  (4) «recursivo = bucle infinito con ciclos en el grafo» — no: el cierre transitivo por
  punto fijo con visitados TERMINA, y el presupuesto del cap. 26 lo hace acotable.
  (5) «la factorización cambia el resultado» — no: mismo multiconjunto lógico, menos
  celdas físicas (lección del cap. 38 generalizada aquí a resultados de JOIN).
- **Objetivos de dominio (teaching)**: Knowledge — sabe decir QUÉ es un join binario,
  POR QUÉ explotan los intermedios, QUÉ acota la cota AGM y CÓMO la frontera común evita
  materializarla. Skills — ejecutar `triangulos_join_binario` vs `TriangulosWcoj` sobre
  el dataset y LEER sus contadores de trabajo. Wisdom — decide CUÁNDO NO usar WCOJ
  (consulta acíclica selectiva) y CUÁNDO la factorización no paga (consumidor de filas
  planas).
- **Pregunta crítica que el capítulo tiene que responder**: «LeapFrog Triejoin
  simplificado». Respuesta medible: una implementación mínima pero HONESTA — tries sobre
  las listas ordenadas que LiraDB YA tiene, búsqueda por saltos, frontera común entre
  niveles y orden de variables estático — demostrada equivalente al plan binario y a la
  fuerza bruta, con su trabajo contado y comparado contra la cota AGM. Sin equivalencia
  testeada no hay capítulo: sería magia, no ingeniería.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| Módulo `cap39_joins.rs` (std puro, sin deps): `AdyacenciasOrdenadas` posicional (heredera directa de `SubgrafoAcotado` del cap. 38: ids ordenados, listas vecinas ordenadas/dedup) + `expansion_dos_saltos_plana` (tuplas de referencia) + `intermedios_plan_binario` (CUENTA las filas intermedias que el esquema binario actual materializaría para un patrón dado) | `cargo test -p vol2-liradb --lib cap39`: tesis `expand_es_index_nested_loop_produce_las_mismas_tuplas` (lo que produce `ExpandOp` REAL sobre el mini dataset == el join formalizado sobre adyacencias) y `intermedios_del_plan_binario_cuentas_conocidas_k8` (K₈ a mano: 56 aristas → 392 caminos (a,b,c) intermedios → 56 triángulos) |
| **Joins binarios honestos**: `triangulos_join_binario` (materializa R(a,b)⋈S(b,c), semi-filtra con T(a,c)) + `triangulos_fuerza_bruta` (terreno de verdad O(n³)) | `triangulos_binario_iguales_a_la_fuerza_bruta` (multiconjunto exacto) y `hub_concentrador_explota_los_intermedios` (estrella bidireccional conocida: ratio intermedios/resultado FIJO esperado, calculado a mano) |
| **LeapFrog simplificado** (pregunta crítica del CORPUS): `BuscadorSalto::primer_mayor_o_igual(slice, x)` (salto exponencial + `binary_search` de std, O(log n) peor caso garantizado) + `frontera_comun(listas)` (intersección coordinada por seeks — la «common frontier» de Veldhuizen) + `TriangulosWcoj::enumerar(adj)` (backtracking por variables a→b→c, ORDEN ESTÁTICO, cero intermedios materializados) + contador `pasos_buscador` | `buscador_primer_mayor_o_igual_casos_exactos`, `frontera_comun_es_la_interseccion_de_candidatos`, tesis DOBLE `wcoj_triangulos_iguales_al_binario_y_a_la_fuerza_bruta`, y `wcoj_pasos_acotados_por_agm_en_k8` (trabajo contado ≤ cota·log, verificado en el grafo pequeño) |
| **Cota AGM medible**: `cota_agm_triangulos(num_aristas)` = ⌊m^(3/2)⌋ (cubrimiento fraccional ρ* = 3/2 del patrón triángulo) usada como BRÚJULA, no como promesa | `agm_bound_acota_el_resultado_en_varios_subgrafos` (mini dataset + K₈ + estrella: triángulos medidos ≤ cota SIEMPRE; el caso apretado se documenta en prosa, no se fabrica) |
| **Consultas recursivas**: `cierre_transitivo(origenes, presupuesto)` → (`BitSet` alcanzables, `MotivoParada`) por punto fijo ITERATIVO reutilizando `BitSet`/`Presupuesto` del cap. 26; termina en grafos CON ciclos; límite de profundidad explícito | tesis de spacing `cierre_transitivo_coincide_con_bfs_existente` (contra el BFS ya implementado, caps. 22/26) y `cierre_transitivo_para_por_profundidad_y_presupuesto` |
| **Factorized execution extendida al JOIN**: `ResultadoFactorizadoTriangulos` (prefijo a compartido, prefijo b compartido, multiplicidades — el f-tree del resultado) siguiendo el patrón de `ExpansionFactorizada` (cap. 38) | `factorizacion_triangulos_filas_logicas_vs_celdas` (cuentas fijas K₈) y tesis `factorizacion_triangulos_equivale_a_las_tuplas_planas` (mismo multiconjunto) |
| **Informe reproducible** para la prosa: intermedios binarios vs pasos WCOJ vs cota AGM vs celdas planas/factorizadas sobre el mini dataset — SIN tiempos (esos viven en criterion, regla del cap. 34) | `informe_joins_reproducible_sobre_mini` |
| CUARTO bench `benches/bench_joins.rs` (`[[bench]] name="bench_joins"`, `harness=false`, ADITIVO en Cargo.toml): grupos `binario_vs_wcoj_regular`, `binario_vs_wcoj_hub_skew` (EL experimento del capítulo: skew que dispara los intermedios), `enumeracion_factorizada_vs_plana`; `Throughput::Elements` como cap. 34 | Compilación automática en `./scripts/verify.sh` (check/clippy `--all-targets`); ejecución MANUAL `cargo bench -p vol2-liradb --bench bench_joins`; prosa pega salidas REALES (Xeon E5-2682 v4 @2.50GHz, misma máquina que caps. 34/38) |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**864 + ~14 tests nuevos ≈ 878**); cero cambios en caps. 7-38, goldens intactos |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «LeapFrog Triejoin simplificado.» El capítulo la responde convirtiendo los
OCHO puntos del brief (líneas 1591-1598) en una escalera donde cada peldaño motiva el siguiente:

1. **Expand como join** → primero se MIRA lo que hay: `ExpandOp` es un index nested-loop
   join (fila ligada × lista de adyacencia) y `optimize` del cap. 21 elige DÓNDE empezar
   una cadena pero siempre salto a salto. Formalizarlo abre la pregunta: ¿hay OTROS planes?
2. **Joins binarios** → el dogma System R: descomponer el patrón en uniones de dos tablas
   (hash join mental + nested loop). Es el plan que LiraDB tiene de facto.
3. **Explosión de resultados intermedios** → contar triángulos con dos joins binarios
   materializa Σ_b out(b)·out(b) caminos de dos saltos: en K₈ son 392 tuplas para 56
   triángulos; con un hub, Ω(n²) intermedios frente a n^1,5 de resultado (SIGMOD Record
   42(4), 2014). LA cifra que justifica el resto del capítulo.
4. **Muchos-a-muchos** → el fan-out del hub ES el muchos-a-muchos. Los triángulos son EL
   patrón canónico (comunidades del cap. 25, fraude) y su hipergrafo es CÍCLICO — el caso
   donde los planes binarios sufren.
5. **Worst-case optimal joins** → AGM (FOCS 2008): el PEOR tamaño del resultado está
   acotado por m^(ρ*) con ρ* = cubrimiento fraccional de aristas (3/2 para el triángulo;
   raíz geométrica: Loomis-Whitney 1949). NPRR (PODS 2012) dio el primer algoritmo que
   NUNCA supera esa cota: el coste se acota por el RESULTADO, no por los intermedios que
   un plan torpe fabrique.
6. **LeapFrog Triejoin simplificado** (respuesta central): tries sobre las listas
   ordenadas que LiraDB YA posee (caps. 14/38), búsqueda por saltos que encuentra el
   primer candidato ≥ x, y FRONTERA COMÚN: en cada nivel todas las relaciones votan sus
   candidatos y solo avanzan juntas. Variable-oriented: se ligan VARIABLES, no tablas;
   jamás se materializa el intermedio del paso 3 (Veldhuizen, ICDT 2014; workhorse de
   LogicBlox; Test-of-Time 2024). Mínimo viable: patrones de 2-3 aristas, orden estático.
7. **Consultas recursivas** → el otro significado de «cíclico»: GRAFOS con ciclos.
   Cierre transitivo por punto fijo iterativo con visitados (`BitSet`) y presupuesto
   (`Presupuesto`/`MotivoParada`, cap. 26); conexión directa con BFS/Dijkstra (22-23);
   contexto estándar: `WITH RECURSIVE` (SQL:1999) y GQL (ISO/IEC 39075:2024) como anzuelo
   al Vol.III — aquí NO hay sintaxis nueva en LiraQL, hay API y concepto.
8. **Factorized execution** → extender la semilla del cap. 38 a resultados de JOIN:
   el triángulo también admite compartir prefijos (a, b) con multiplicidades
   (Olteanu-Závodný, ICDT 2012 / TODS 40(1), 2015; processor factorizado de Kùzu, CIDR
   2023 CC-BY 4.0, ADR-001). Contar ≠ enumerar: agregar sobre la representación
   factorizada ni siquiera expande las tuplas.

Hilo conductor: «deja de preguntarte qué TABLAS unes y empieza a preguntarte qué VARIABLES
ligas — y quién paga mientras tanto: los intermedios».

---

## 4. La arquitectura: dos orientaciones de plan — y quién paga la factura

Modelo mental único: **join-oriented × materialización de intermedios**. Los planes
clásicos atan TABLAS en un árbol de uniones binarias (cada unión escribe su resultado);
los worst-case optimal atan VARIABLES en un backtracking con tries (nadie escribe nada
intermedio).

```text
  JOIN-ORIENTED (hoy, caps. 19-21):        VARIABLE-ORIENTED (objetivo, cap. 39):
  NodeScan(a) → Expand → Expand            liga a → liga b → liga c
      R(a,b) ⋈ S(b,c)                      tries ordenados sobre las MISMAS adyacencias
        ↓ MATERIALIZA N₁                   frontera común por nivel: seek coordinado
      N₁ = Σ_b out(b)·out(b)  (K₈: 392)    (nadie escribe N₁)
        ⋈ T(a,c)  →  56 triángulos          56 triángulos directamente
```

Y debajo, la REGLA DE ORO heredada del cap. 34: cada número con dataset determinista
(`SEMILLA_REFERENCIA`), contadores de TRABAJO (pasos de búsqueda, tuplas intermedias,
celdas físicas) dentro de los tests, y TIEMPOS solo en criterion. Sin equivalencia
testeada contra la fuerza bruta, cualquier velocidad es un rumor.

```text
Lo que SÍ se hace hoy:   capa EDUCATIVA aparte (módulo propio, como la lectura analítica del cap. 38):
                         formalizar expand, contar intermedios, leapfrog 2-3 aristas,
                         cota AGM como brújula, cierre transitivo con presupuesto,
                         factorización de resultados de join, bench comparativo
Lo que AÚN NO:           operadores nuevos dentro del Executor Volcano · integración con
                         liradb explain · orden DINÁMICO de variables · Generic Join multi-way
                         genérico de NPRR · sintaxis recursiva en LiraQL · paralelismo ·
                         estadísticas nuevas del catálogo (cap. 21 intacto)
```

Momento ¡ajá! perseguido: «los intermedios pueden ser más grandes que la entrada Y que el
resultado JUNTOS — y existe una familia de planes que jamás los escribe, porque atan
variables en vez de unir tablas. LiraDB ya tenía medio camino hecho: adyacencias ordenadas
desde el cap. 14».

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Un módulo nuevo `cap39_joins.rs`, std puro, dependency-free | Regla «primero a mano» (CONVENTIONS §4): tries, seeks y fronteras SON enseñables con slices ordenados y `binary_search`; el crate sigue sin deps runtime | Crate externo de joins (datafusion, etc.): deps enormes, API opaca, nada que APRENDER | CONVENTIONS §4; misma regla que caps. 18 (lexer), 28 (WAL), 38 (diccionario/packing) |
| 2 | Capa EDUCATIVA hexagonal FUERA del Executor: ni `optimize()` ni `PhysicalOperator` se tocan; los operadores nuevos viven en `cap39_joins` con sus propias estructuras | 864 tests verdes y contratos de caps. 19-21 intactos; el capítulo enseña PLANES, no instala uno en producción; precedente exacto: la lectura columnar del cap. 38 | Integrar `WcojJoinOp` en el Executor y una regla nueva en `optimize`: proyecto de otra Parte, mezclaría pedagogía con producción y rompería explain/goldens | Honestidad hexagonal caps. 33-34/38; CONVENTIONS §2 (una idea nueva por sección) |
| 3 | MEDIR ANTES: formalizar `ExpandOp` como join y CONTAR intermedios antes de proponer curas | Ley de la casa desde el cap. 34: sin medida propia, la explosión de intermedios es un rumor de paper; el Ω(n²) del triángulo se demuestra EN el dataset de LiraDB | Empezar por la teoría (AGM) y «confiar» en que aplica: dominio ilusorio | Metodología cap. 34; SIGMOD Record 42(4), 2014 (survey: «traditional databases evaluate joins pairwise… forces Ω(N²)») |
| 4 | Triángulos como caso de estudio canónico (detección de comunidades — cap. 25 ya los usa — y fraude) | Patrón mínimo CÍCLICO: suficiente para disparar la explosión y para mostrar la cura; conecta con material previo (spacing) | 4-ciclos o quads: misma lección, más índices y cuentas; queda como reto experto | Brief líneas 1594; cap. 25 (modularidad cuenta triángulos); SIGMOD Record 42(4), 2014 (ejemplo O(N^1,5)) |
| 5 | LeapFrog SIMPLIFICADO sobre listas ordenadas existentes (no NPRR genérico) | Las adyacencias de LiraDB YA están ordenadas (cap. 14/38): el trie sale gratis; Veldhuizen vende exactamente esto: «easy to understand, simple to implement», y fue workhorse comercial ANTES de probarse | Implementar Generic Join/NPRR completo: maquinaria profunda, análisis no enseñable en un capítulo, mismo mensaje | Veldhuizen, ICDT 2014 (ICDT Test-of-Time 2024, databasetheory.org); CIDR 2023 (Kùzu adopta WCOJ — precedencia industrial, ADR-001) |
| 6 | Orden de variables ESTÁTICO (a→b→c), elegido a mano y justificado en prosa | La elección dinámica exige estadísticas por prefijo que el catálogo del cap. 21 no tiene; fingirla sería humo; el estático basta para la tesis del capítulo | Reordenación dinámica tipo greedy por cardinalidad estimada: fuera de alcance, declarada en «Para profundizar» | Veldhuizen, ICDT 2014 (§6, extensiones); frontera honesta del cap. 21 (solo cadenas) |
| 7 | Seek = salto exponencial + `binary_search` de std (O(log n) peor caso, cero supuestos) | La interpolación del paper original puede ser más rápida en la práctica pero su coste depende de supuestos de distribución; con std puro lo honesto es la garantía logarítmica | Interpolation search pura: peor caso lineal, injustificable sin datos uniformes | Rust std (`slice::binary_search`, docs oficiales); Veldhuizen, ICDT 2014 (define el seek; nuestra variante es la suya con búsqueda binaria) |
| 8 | Cierre transitivo ITERATIVO con `BitSet` de visitados + `Presupuesto`/`MotivoParada` del cap. 26; sin sintaxis nueva en LiraQL | Spacing puro (22/26); termina en grafos cíclicos por construcción; el presupuesto lo hace DEMOSTRABLEMENTE acotable; `WITH RECURSIVE`/GQL quedan como contexto estándar y anzuelo al Vol.III | Parser de `WITH RECURSIVE` en LiraQL: alcance de lenguaje entero que diluiría el foco (joins) | ISO/IEC 39075:2024 (GQL); SQL:1999 (CTEs recursivos); caps. 22/26 (BFS, presupuesto) |
| 9 | Factorización de TRIÁNGULOS extendiendo el patrón arrays-por-variable+multiplicidad del cap. 38; cita EXACTA ICDT 2012 / TODS 40(1) 2015 | El concepto ya demostrado en 2-hop se generaliza al resultado de un JOIN cíclico; conteo lógico/físico idéntico al del cap. 38 (misma métrica, más patrones) | d-representaciones con referencias simbólicas compartidas: potencia extra, complejidad de aliasing innecesaria aquí | Olteanu-Závodný, ICDT 2012 («Factorised Representations…», openproceedings.org) y TODS 40(1), 2015 («Size Bounds…»); Jin et al., CIDR 2023 (CC-BY 4.0, ADR-001) |
| 10 | CUARTO `[[bench]] bench_joins.rs` ADITIVO — SÍ al bench | El capítulo ES una tesis de clases de coste: sin delta medido (regular vs hub-skew) sería teoría de humo; criterion ya está en dev-deps y Cargo integra el target gratis (regla cap. 34) | Solo contadores de trabajo sin cronometraje: pierde la mitad empírica del argumento | Metodología de benches cap. 34 (criterion 0.7, `harness=false`, hardware declarado); precedente cap. 38 (tercer bench) |
| 11 | Equivalencia OBLIGATORIA por test: WCOJ == binario == fuerza bruta (multiconjunto) y factorizada == plana | Sin test-tesis, el capítulo vendería velocidad sin corrección; los off-by-one de seeks y las dobles cuentas de multiplicidad SE CAZAN aquí | Tests estadísticos o «visual»: no detectan corrupción silenciosa | Torre de pruebas (cap. 33); lección de equivalencia del cap. 38 (filtro por lotes == filtro fila) |
| 12 | Atribución según ADR-001: relato histórico verificado (Waterloo → CIDR 2023 CC-BY 4.0 → compra por Apple oct-2025 → repo archivado → forks LadybugDB/bighorn); clean-room, cero código copiado | Política VINCULANTE para cap. 38+: toda mención de factorización/WCOJ cita CIDR 2023 con licencia | Citar «Kùzu renombrado a Ladybug» (FALSO) o papers inexistentes (VLDB 2023) | ADR-001 (RESUELTA 2026-08-25, política completa); The Register 14-oct-2025; repo GitHub archivado |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: LogicBlox, años 2010 — Todd Veldhuizen
   y compañía llevan en PRODUCCIÓN comercial un join llamado leapfrog triejoin sin que
   nadie haya probado aún su optimalidad; el paper (ICDT 2014) demuestra que lo ES
   «hasta un factor log», y una década después gana el ICDT Test-of-Time Award 2024:
   «implemented before its optimality guarantees were discovered» (palabras del propio
   paper). Pregunta enmarcada: ¿cuánto paga LiraDB HOY en intermedios que ni siquiera ve?
2. **N.1-N.2 Objetivo/Problema**: 864 tests verdes, motor ACID completo — y un MATCH de
   tres aristas que atraviesa `ExpandOp` fila a fila fabricando intermedios invisibles.
   Qué NO te dice la suite verde: cuántas tuplas fantasma materializa tu plan antes de
   darte 56 respuestas.
3. **N.3 Modelo mental**: panel dual join-oriented vs variable-oriented (§4) + el bloque
   «lo que sí / lo que aún no» + la escalera de ocho peldaños del brief.
4. **N.4 Primera solución**: la ingenua de todo el mundo — contar triángulos con dos
   joins binarios: materializar R(a,b), unir con S(b,c) por b, filtrar con T(a,c).
   Correcta, legible, y con un contador de trabajo que la delata.
5. **N.5 Sus límites**: K₈ a mano (392 intermedios para 56 triángulos); la estrella con
   hub donde el ratio explota; la ley Ω(n²) vs n^1,5; memoria como recurso físico.
6. **N.6 Solución evolucionada**: listas ordenadas → tries → `BuscadorSalto` → frontera
   común → `TriangulosWcoj` con orden estático → cota AGM como brújula (no promesa) →
   factorización del RESULTADO → cierre transitivo con presupuesto para los grafos
   cíclicos que el matching también debe atravesar.
7. **N.7 Código completo ejecutable**: `cap39_joins.rs` y `benches/bench_joins.rs`
   referenciados por `include::` (nunca duplicados) + el cuarto bloque `[[bench]]`.
8. **N.8 Prueba de fuego**: los DOS test-tesis de equivalencia + `agm_bound_acota_*` +
   `cargo bench --bench bench_joins` con salidas REALES pegadas (Xeon E5-2682 v4
   @2.50GHz): grupo regular (delta honesto, quizá modesto) vs grupo hub-skew (donde el
   binario se hunde y el WCOJ aguanta) + tabla intermedios/pasos/cota/celdas del informe.
9. **N.9 Qué hemos sacrificado**: capa educativa sin integración en Executor ni `explain`;
   orden de variables estático; LeapFrog limitado a patrones de 2-3 aristas (no ∃₁
   completa ni d-representaciones); sin Generic Join multi-way genérico («Para
   profundizar»); sin paralelismo; sin sintaxis recursiva en LiraQL; sin estadísticas nuevas.
10. **N.10 Cómo lo hace una BBDD real + retos**: Neo4j (Cypher expand + shortest path),
    DuckDB (hash joins columnares), LogicBlox (LFTJ en producción — la anécdota de N.0
    cerrada), y **Kùzu/LadybugDB** según ADR-001: joins worst-case optimal + processor
    factorizado para GRAFOS (CIDR 2023, CC-BY 4.0). Retos: esencial (contar caminos de
    2 saltos con frontera común y comparar contra `ExpandOp`), intermedio (PREDECIR los
    intermedios del plan binario ANTES de medir y explicar la desviación), experto
    (extender el leapfrog al 4-ciclo, calcular su ρ\* a mano y comparar contra cota).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla / 30 segundos /
    Una historia pequeña / Mini-diálogo de guardia nocturna (la consulta de comunidad que
    agotaba RAM con intermedios y ahora se enumera factorizada). Retrieval practice:
    reproducir DE MEMORIA la escalera expand→binario→explosión→AGM→leapfrog→factorización
    y el panel dual. Interleaving: cada reto toca ≥2 capítulos (14+39, 20+39, 21+39,
    25+39, 26+39, 38+39). Glosario nuevo: index nested-loop join, semi-join, trie, orden
    de variables, frontera común, seek/salto exponencial, worst-case optimal, cota AGM,
    cubrimiento fraccional de aristas, cierre transitivo, punto fijo, representación
    factorizada, multiplicidad.
12. **Gancho de cierre (preguntas abiertas)**: ya sabes QUÉ calcular (WCOJ) y CÓMO
    leerlo rápido (cap. 38) EN UNA máquina; ¿qué pasa cuando ni los datos ni el cálculo
    caben en una? Cap. 40: particionado, cortes de aristas, replicación de fronteras y
    consenso. Abiertas: ¿cómo se PARALELIZA un leapfrog (morsel-driven, rayon futuro)?,
    ¿qué estadísticas exigiría el orden DINÁMICO de variables?, ¿y si las columnas del
    cap. 38 vivieran en disco?

---

## 7. Estilo y tono (consistencia con caps. 27-38)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez; salidas REALES pegadas (hardware, SO y toolchain
  declarados), nunca reconstruidas; deltas honestos aunque sean modestos; la cota AGM
  se presenta como BRÚJULA (acota el peor caso) y se muestra su holgura sin vergüenza.
- **Diagramas**: panel dual de orientación de planes (§4); dibujo del trie de prefijos
  (a→b) y de la frontera común avanzando junta; la escalera de ocho peldaños del brief;
  bloque «lo que sí / lo que aún no»; tabla de cuentas K₈ hecha a mano.
- **Spacing** (conceptos viejos que se EJERCITAN): CSR ordenado (cap. 14),
  `ExpandOp`/Volcano (cap. 20), optimizador y catálogo (cap. 21), BFS/Dijkstra (22-23),
  triángulos/modularidad (cap. 25), `BitSet`/`Presupuesto`/`MotivoParada` (cap. 26),
  dataset y benches (cap. 34), `SubgrafoAcotado`/`ExpansionFactorizada` (cap. 38).
- **Interleaving**: reto esencial mezcla 20+39 (expand vs frontera común); el intermedio
  mezcla 21+34+39 (predecir intermedios con el catálogo ANTES de medir); el experto
  mezcla 24/25+39 (4-ciclos y su ρ\*) y 38+39 (factorizada vs plana).
- **Dificultad asimétrica**: una idea nueva por sección (formalizar expand → joins
  binarios → explosión → cota AGM → seek → frontera común → cierre → factorización);
  los ejercicios exigen PREDECIR conteos y recordar la escalera sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap39` (mini dataset,
  milisegundos, contadores exactos) y `./scripts/verify.sh` ALL_GREEN como puerta;
  `cargo bench --bench bench_joins` como acto EXPLÍCITO del lector. Nunca «confía en mí».
- **Anécdota (única, verificada)**: LogicBlox/leapfrog — producción antes que la prueba
  (Veldhuizen, ICDT 2014; Test-of-Time 2024, databasetheory.org/node/150). Fuentes para
  la prosa: Atserias-Grohe-Marx (FOCS 2008; SICOMP 42(4), 2013); Ngo-Porat-Ré-Rudra
  (PODS 2012, pp. 37-48); «Skew Strikes Back» (SIGMOD Record 42(4), 2014 — NO «PODS
   2012», NO «SIGMOD 2013»); Veldhuizen (ICDT 2014); Olteanu-Závodný (ICDT 2012; TODS
   40(1), mar-2015 — la cita errónea «ICDT 2015» del cap. 38 fue corregida allí el
   2026-08-26); Loomis-Whitney (Bull. AMS 55, 1949); Jin et al. (CIDR 2023, CC-BY 4.0,
  ADR-001); ISO/IEC 39075:2024 (GQL); SQL:1999 (WITH RECURSIVE).

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap39_joins; pub use
  cap39_joins::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al final, con el
  módulo ya compilando limpio (`cargo check -p vol2-liradb`); jamás dejar `lib.rs`
  apuntando a un módulo roto.
- **Orden de implementación recomendado** (cada paso compila y testea solo):
  (1) `AdyacenciasOrdenadas` + expansión plana + contador de intermedios; (2) fuerza
  bruta + join binario + equivalencia; (3) `BuscadorSalto` + `frontera_comun`; (4)
  `TriangulosWcoj` + equivalencias dobles; (5) cota AGM; (6) `cierre_transitivo`;
  (7) factorización de triángulos; (8) informe; (9) bench + `[[bench]]`; (10) wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap39` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap39_joins.rs`,
  y continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). Los benches:
  verify.sh solo los COMPILA (regla cap. 34); interrumpir `cargo bench` no corrompe nada.
- **Criterio de parada honesto**: si `binario_vs_wcoj_hub_skew` NO muestra diferencia
  medible, se REPORTA tal cual (como el ×0,86 del email en cap. 38) y la prosa explica
  POR QUÉ — prohibido inflar o esconder el resultado.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5; citas verificadas 2026-08-26, venues/años exactos).
- [ ] Escenario de fallo visible, no solo happy path: hub que explota intermedios
  (medido), seek con off-by-one (casos exactos), triángulo contado 6 veces si la
  expansión undirected no canoniza, multiplicidad doble-contada (tesis factorizada==plana),
  ciclo infinito sin visitados (test de terminación).
- [ ] Código ejecutable en workspace citado por nombre (`cap39_joins.rs`,
  `benches/bench_joins.rs`, cuarto `[[bench]]`, wiring en `lib.rs`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: cinco).
- [ ] Ejercicios con solución verificable diseñados (retos N.10 con nombres previstos).
- [ ] ≥1 ejercicio de retrieval (escalera de memoria + panel dual) y spacing planificado
  (caps. 14/20/21/22/25/26/34/38; §7).
- [ ] Responde la pregunta crítica del CORPUS («LeapFrog Triejoin simplificado»:
  implementación EQUIVALENTE y CONTADA, no descrita) y cobra las deudas heredadas
  (caps. 14/20/38; §blockquote/§3).
- [ ] Anécdota única verificada con fuentes primarias (ICDT 2014 + Test-of-Time 2024).
- [ ] Alcance de código nuevo acotado y honesto (UN módulo + UN fichero bench + UNA
  entrada `[[bench]]` + wiring en `lib.rs`; cero dependencias y cero cambios caps. 7-38).
- [ ] Gancho saliente fijado (cap. 40 distribución: «¿y cuando el grafo no cabe en una?»;
  preguntas abiertas de paralelismo/orden dinámico/columnas en disco; §6.12).
