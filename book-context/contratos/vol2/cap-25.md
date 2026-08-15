# CONTRATO DE CAPÍTULO — Vol.II Cap. 25: Comunidades y agrupaciones (Louvain simplificado)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap25_comunidades.rs` (2.212 líneas,
> 22 tests en `tests_comunidades` + 4 doctests, sin crates externas). Decisiones
> reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md` §30. Pregunta
> crítica de `CORPUS.yml` (id `vol-II-cap-25`): «Modularidad; greedy Louvain».
> ToC línea 40: «Comunidades y agrupaciones (Louvain simplificado)». Capítulo 4
> de la Parte V: el cap 24 RANKEÓ nodos (¿quién es el centro?); éste PARTICIONA
> el grafo (¿quiénes forman grupo?). Deja gancho explícito al cap 26 (proyección
> con pesos) y al cap 51 del Vol.III (GraphRAG consume la jerarquía).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: recorridos BFS sobre `&dyn GraphStore` (Vol. I
  cap. 4 y cap. 24); la PROYECCIÓN materializada una vez del cap. 24 (nodos
  ordenados por id → determinismo, índice denso que compacta huecos de
  `delete_node`, vecindarios por `GraphDirection`); la semántica ESTRICTA de
  pesos del cap. 22 (`WeightSource::{Constant, Property}` + `edge_weight`:
  prop ausente/NULL = `MissingWeight`, tipo no numérico = `InvalidWeight`,
  NaN/±∞ = `NonFiniteWeight`); f64 y por qué comparar con tolerancia
  (`total_cmp` en caps. 22/24); que una BD debe REPRODUCIR sus análisis
  (determinismo del cap 24, dos ejecuciones = mismo resultado).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «una
  comunidad es un grupo denso de gente que se conoce» — incompleto: densidad
  RESPECTO A QUÉ; sin modelo nulo, un clique dentro de un grafo completísimo no
  es comunidad; (2) «el número de comunidades mide la calidad» — falso: un
  conteo no compara particiones de distinto tamaño; hace falta una FUNCIÓN
  OBJETIVO (Q); (3) «label propagation detecta comunidades» — a medias: es una
  heurística SIN métrica que verifica nada, y con pesos uniformes GOTEA por los
  puentes (test que lo documenta); (4) «Louvain encuentra LA partición óptima»
  — falso: es greedy local con óptimos locales (el test de `demo_graph` muestra
  DOS óptimos con la MISMA Q) y la modularidad misma tiene LÍMITE DE
  RESOLUCIÓN; (5) «más peso en una arista = más fusión» — falso: Q es
  INVARIANTE a la escala de pesos; un puente pesadísimo RESTRUCTURA (rompe los
  tríos alrededor de él), no funde; (6) «los self-loops no importan» — cuentan
  DOBLE (A_ii = 2s) y dan a Dani comunidad propia en `demo_graph`.
- **NO debe saber todavía**: Leiden y sus garantías de conectividad interna
  (sólo se cita como «cómo lo hace una BBDD real» con la crítica de Traag
  2019), modularidad negativa/louvain para grafos firmados, detección de
  comunidades solapadas (k-cliques), benchmark LFR, inferencia estadística
  bayesiana (SBM de Peixoto), la proyección CON PESOS sobre CSR del cap. 26
  (aquí `GrafoPonderado` la materializa en memoria y la deuda se declara).
  Se nombran y se cortan.

## 2. Conceptos (del grafo curricular)

- `present`: partición de un grafo (`Particion`: ids de grupo DENSOS ordenados
  por menor miembro); componentes conexas como caso límite (alcanabilidad pura
  vs densidad); label propagation ASÍNCRONA determinista con votos ponderados
  y sus límites; la MODULARIDAD Q_γ de Newman-Girvan (2004) con γ de
  Reichardt-Bornholdt (2006): `Q_γ = Σ_c [Σ_in(c)/2m − γ·(Σ_tot(c)/2m)²]`,
  modelo nulo de configuración, Q=0 en la trivial, Q>0 mejor que el azar;
  Louvain simplificado: fase local greedy con ΔQ exacto + AGREGACIÓN en
  supernodos por niveles; self-loops con convención A_ii = 2s; simetrización
  SUMANDO pesos (multigrafo); el LÍMITE DE RESOLUCIÓN de
  Fortunato-Barthélemy (2007) y γ como remedio; jerarquía/dendrograma de
  niveles (`NivelLouvain`, `particion_en`).
- `practice`: BFS y cola (Vol. I cap. 4, cap. 24); `WeightSource`/`edge_weight`
  estrictos y rechazo eager de inválidos (cap. 22, como Dijkstra); proyección
  materializada con ids ordenados e índice denso (cap. 24); `total_cmp` para
  empates de f64 (caps. 22/24); `*Stats` medibles (cap. 24
  `CentralidadStats`).
- `consolidate`: «derivar, no llevar en la cabeza» (k y 2m derivan de la
  adyacencia); fallar ruidosamente antes que contestar casi-bien (errores del
  cap. 22); determinismo como contrato de un motor de BD.
- `out_of_scope` (solo nombrar): Leiden (fase de refinamiento), SBM/Peixoto,
  LFR benchmarks, comunidades solapantes, grafos firmados, proyección con
  pesos persistida (cap. 26), consumo real de la jerarquía (cap. 51 Vol.III).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) escribe Q_γ de memoria y lo calcula A MANO sobre dos tríos
  con puente (Q = 5/14 la perfecta, 0 la trivial, −17/98 los singletons);
  (2) explica por qué Q y no «número de comunidades» (Q compara particiones de
  distinto número de grupos; un conteo no es función objetivo); (3) calcula el
  ΔQ EXACTO de mover un nodo con la fórmula de los dos términos que cambian;
  (4) explica el límite de resolución con el anillo de 12 tríos: ΔQ de fundir
  dos tríos = 1/48 − γ/72, que es >0 con γ=1 (funde: Q 17/24 > 2/3) y <0 con
  γ=2 (recupera: Q 7/12 > 13/24); (5) dice por qué el self-loop cuenta doble
  (A_ii = 2s mantiene k_i = Σ_j A_ij y 2m = Σ_i k_i, y es lo que hace que la
  CONTRACCIÓN conserve Q: la arista interna w se vuelve self-loop w que aporta
  2w a ambos, igual que antes).
- **Skills**: (1) ejecutar `louvain`/`modularidad`/`label_propagation`/
  `componentes_conexas` sobre un store y leer `Particion` (grupo/grupos/
  tamanos) y `LouvainResult` (niveles, Q, `particion_en`); (2) verificar con
  `modularidad()` la Q de cualquier partición (oráculo); (3) construir el
  anillo de tríos y demostrar el límite de resolución con γ.
- **Wisdom**: (1) decide cuándo LPA basta (red enorme, sin verificación, pesos
  que rompan empates) y cuándo exige Louvain + Q verificable; (2) decide γ
  mirando la ESCALA esperada de comunidades (γ>1 para chicas; el precio:
  más comunidades y Q absolutos no comparables entre γ); (3) sabe que una
  partición «coherente con su Q» (contrato del resultado) no es «la óptima
  global» (greedy, óptimos locales empatados).

## 4. Modelo mental

- **El mapa de tribus**: el grafo es un mapa de poblaciones; una comunidad es
  una TRIBU (mucha relación interna, poca con fuera). La pregunta de Q: «de
  todo el peso de las aristas, ¿qué fracción cae DENTRO de tribus… y cuánto
  más de lo que ESPERARÍA EL AZAR si repartiéramos las mismas aristas al
  azar?» (el modelo nulo de configuración: mismo grado, conexiones al azar).
  Q es el «exceso de interioridad sobre el azar». Componentes = «islas»
  (alcanabilidad); tribus = «regiones densas dentro de las islas».
- **Diagramas ASCII**: (a) el anillo de 12 tríos con sus eslabones (el test
  estrella); (b) un nivel de Louvain: nodos → comunidades → supernodos (las
  aristas internas se vuelven self-loops); (c) la tabla Q de las dos
  particiones del anillo con γ=1 y γ=2.
- **Momento ¡ajá!**: el anillo de tríos con γ=1: «el ojo VE doce tribus, pero
  Q PREFIERE seis pares — y no es un bug del algoritmo: la métrica global
  pregunta al azar de TODO el grafo, y con 2m=96 el azar espera tan poco
  cruce que fundir sale gratis. La misma estructura local (dos tríos unidos)
  que en un grafo de 14 unidades de peso NO se funde (ΔQ=−5/14), en el anillo
  SÍ (ΔQ=+1/144). El tamaño del mundo cambia la respuesta.»

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap25_comunidades.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Q de Newman-Girvan como métrica guía, no «nº de comunidades» | Un conteo no compara particiones de distinto tamaño (2 comunidades de 3 vs 6 singletons: ¿cuál es «mejor»?). Q es una función objetivo sobre CUALQUIER partición: comparable entre sí, verificable a mano, y sirve de oráculo de tests | Maximizar densidad media interna: no penaliza lo que el azar explicaría; sin modelo nulo no hay «mejor que azar» | Heurística sin juez: imposible decir si el resultado es bueno | Newman-Girvan, PRE 69, 026113 (2004); `modularidad` (líns. 930-960); test `modularidad_particion_trivial_es_cero_y_perfecta_analitica` |
| 2 | Greedy local + agregación, no corte de aristas (divisivo) | El método original de Newman-Girvan CORTA la arista de mayor betweenness repetidamente: O(V·E²) impracticable en redes grandes. Louvain invierte el problema: en vez de quitar aristas hasta que Q parezca bueno, OPTIMIZA Q directamente con movimientos locales baratos (ΔQ en O(grado)) y contrae. El paper lo validó con 2,6 M de clientes de móvil | Divisivo con betweenness (cap 24 lo calcula: O(V·E)): días de CPU donde Louvain tarda minutos | «Detectar comunidades» = análisis inalcanzable fuera de juguetes | Blondel et al., J. Stat. Mech. P10008 (2008): «outperform all other known community detection method in terms of computation time»; `fase_local` (líns. 498-583) |
| 3 | ΔQ EXACTO por movimiento (diferencia de los 2 términos de comunidad que cambian) | Trazabilidad aritmética: cada movimiento se puede re-calcular a mano con In_c, K_c, In_d, K_d. Sin recomputar Q entera (O(V+E) por evaluación → inviable) | Recalcular Q completa tras cada movimiento hipotético: O(pasadas·V·(V+E)) — correcto pero inabordable; ΔQ aproximado: pierde el oráculo | No se puede NI enseñar el algoritmo NI confiar en su monotonía | Fórmula documentada en `fase_local` (líns. 487-497); invariante Q(resultado)==`modularidad()` testeada en cada caso |
| 4 | Self-loops cuentan DOBLE (A_ii = 2s) | Es la convención estándar de modularidad: mantiene k_i = Σ_j A_ij y 2m = Σ_i k_i contando el loop «una vez por dirección». Y es la que hace que la AGREGACIÓN conserve Q: una arista interna w aportaba 2w a Σ_in y w+w a Σ_tot; contraída es un self-loop s=w que aporta A_cc=2w a ambos — idéntico | Contarlo simple (A_ii = s): la contracción NO conservaría 2m y la Q de un nivel cambiaría al agregar → jerarquía incoherente | Niveles cuya Q no cuadra con el grafo original: el oráculo se rompe | Test `modularidad_self_loops_simetria_y_paralelas` (Juntos Q=0 con k=[5,1]); invariante de contracción en `louvain_jerarquia_monotonia_anidamiento_y_contraccion` |
| 5 | `GrafoPonderado` propio (SIMETRIZANDO SUMANDO), no la `Proyeccion` del cap 24 | Dos familias, dos convenciones: el cap 24 contaba VECINOS DISTINTOS (su `Both` deduplica paralelas — correcto para grado); Louvain/la modularidad son de MULTIGRAFO: 3 mensajes = lazo triple, DEBEN sumar (2·k_{i,c} del ΔQ lo exige). Además Louvain RECONSTRUYE el grafo en cada agregación: `contraer()` devuelve otro `GrafoPonderado`; la proyección del cap 24 no sabe ni de pesos ni de reconstrucción | Reusar la `Proyeccion`: las aristas paralelas colapsarían a 1 y Q mediría OTRO grafo (dos K4 unidos por 3 pares ≠ unidos por 1) | Q y particiones de un grafo que no es el del store | MIGRATION §30.1; test `louvain_multigrafo_paralelas_equivalen_a_peso_sumado` (3 paralelas ≡ una de peso 3) |
| 6 | Pesos negativos rechazados EAGER (`NegativeWeight`) | La lectura «fracción de peso interno» y el modelo nulo (k_i ≥ 0, (Σ_tot/2m)² términos) se rompen con negativos: Q deja de ser «exceso sobre el azar». Igual que Dijkstra en el cap 22: una BD prefiere fallar ruidosamente a contestar casi-bien | Admitirlos y «ver qué pasa»: resultados sin lectura estadística, monotonía de Louvain sin garantía | Análisis silenciosamente sin sentido | `ComunidadesError::NegativeWeight` (líns. 142-143); test `modularidad_pesos_invalidos_y_negativos`; convención Dijkstra cap 22 |
| 7 | Determinismo TOTAL: nodos por id, candidatos por id de comunidad, empates por `total_cmp` → menor, renumeración por menor miembro | El Louvain de la literatura BARAJA los nodos (mejor exploración; resultados distintos por ejecución). Un motor de BD debe REPRODUCIR sus análisis: dos ejecuciones = resultado idéntico, incluso con el orden de inserción de aristas invertido | Barajar como el paper: particiones distintas por ejecución, tests y explicaciones no reproducibles | El mismo query «a veces» responde distinto: inaceptable en una BD | `fase_local` (sort por id de comunidad + `total_cmp` estricto, líns. 541-563); `densificar` (líns. 589-610); test `louvain_determinismo_y_orden_de_insercion` |
| 8 | Cota de terminación ≤ V niveles + `max_pasadas` por nivel | Demostrable: cada nivel arranca de SINGLETONES, así que el primer movimiento VACÍA una comunidad ⇒ el nivel siguiente tiene estrictamente menos nodos ⇒ niveles con movimientos ≤ V. `max_pasadas` corta las pasadas de una fase ante el ruido de f64 en ΔQ ≈ 0 (sin seguro, un ΔQ=1e-18 «positivo» podría mover eternamente) | Confianza ciega en la convergencia numérica: un lazo infinito en producción | Query que no termina | `tope_niveles = n + 1` (lín. 1119) + comentario de cota (líns. 1115-1118); test `louvain_parametros_invalidos_y_max_pasadas` |
| 9 | La jerarquía lleva nodos ORIGINALES (`NivelLouvain.asignacion` compuesta) | El cap 51 (GraphRAG, Vol. III) consumirá los NIVELES para resúmenes a varias granularidades: necesita «¿quién está con quién?» en el vocabulario del USUARIO (ids originales), no en supernodos internos. Anidamiento garantizado por construcción (cada comunidad del nivel ℓ+1 es unión exacta de las del ℓ) | Guardar sólo la partición por nivel y recomponer fuera: cada consumidor reimplementaría la composición (y podría equivocarla) | Dendrograma sin garantía de anidamiento: resúmenes incoherentes entre niveles | `NivelLouvain` (líns. 972-988), `particion_en` (líns. 1023-1028); test de anidamiento en la DIRECCIÓN correcta (fina ⇒ gruesa) |
| 10 | LPA con política de empates «conservar la propia si empata con la máxima; si no, la MENOR» + documentar que GOTEA | El LPA original desempata AL AZAR. Sin azar hay que DECIDIR: conservar la propia frena el goteo en la 2ª mitad de la pasada… pero NO lo evita: con pesos uniformes, el primer grupo formado arrastra al vecino del puente cuya etiqueta propia aún no reúne votos (dos K3+puente 2-3 → 1 comunidad; el MISMO grafo con puente 1-4 → 2, porque el keep-own salva al nodo 4). Test que documenta el fenómeno vs Louvain (2) sobre el mismo grafo: la métrica guía contra la etiqueta sin métrica | Ocultar el límite («LPA encuentra comunidades»): el lector confiaría una heurística sin verificación | Comunidades fusionadas por artefacto del orden de barrido, creídas correctas | Test `lpa_separa_dos_trios_y_empates_deterministas` (líns. 1603-1613); receta: pesos que rompan empates (puente 0.5 → LPA separa) |
| 11 | Q final SIEMPRE sobre el grafo ORIGINAL (nunca del contraído) | La contracción conserva Q en exacto (invariante testeada nivel a nivel), pero el CONTRATO del resultado es «Q de esta partición en el store»: si mañana la contracción cambia, el contrato no | Devolver la Q del último grafo contraído: hoy igual, mañana un bug latente si alguien «optimiza» `contraer` | Q reportada ≠ `modularidad()` de la partición devuelta | `louvain` (lín. 1176); test de equivalencia en `louvain_separa_dos_cliques_unidas_por_puente` |
| 12 | El anillo de 12 tríos como TEST estrella (límite de resolución) | Fortunato-Barthélemy 2007: la modularidad GLOBAL no ve comunidades más chicas que una escala (~√(2m)): Q prefiere fusionarlas porque el azar de un grafo GRANDE espera tan poco cruce. El anillo lo DEMUESTRA con Q analíticos: γ=1 → 6 pares (Q=17/24 > 2/3), γ=2 → 12 tríos (Q=7/12 > 13/24); y ΔQ(fundir dos tríos) = 1/48 − γ/72 cambia de signo en γ=3/2 | Presentar el límite como cita bibliográfica: el lector no lo VERÍA en su código | Usuario que confía ciegamente en particiones «demostradamente» (Q) subóptimas para su pregunta real | Fortunato-Barthélemy, PNAS 104(1):36-41 (2007); test `louvain_limite_de_resolucion_gamma` (líns. 1907-1966) + `louvain_jerarquia_…` (nivel 0: 12 tríos Q=2/3; nivel 1: 6 pares Q=17/24) |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: «comunidad = componente conexa» (`componentes_conexas`, el caso
  límite: alcanzabilidad pura, O(V+E)). Falla en cuanto hay UN puente: dos
  tribus unidas por una arista son una sola componente — la red social entera
  suele ser UNA componente gigante.
- **Qué la rompe**: el grafo canónico de dos K3/K4 unidos por un puente: 1
  componente, 2 tribus a la vista. La densidad (no la alcanzabilidad) es el
  concepto. Siguiente intento: LPA (votos) — mejor, pero sin función objetivo
  que verifique nada y con el goteo por puentes.
- **Evolución visible**: (1) `modularidad` convierte «buena partición» en un
  NÚMERO verificable sobre cualquier partición dada; (2) `louvain` optimiza
  ese número con ΔQ exacto por movimiento + agregación jerárquica; (3) γ
  (Reichardt-Bornholdt) pone el zoom; (4) `NivelLouvain` guarda el
  dendrograma para el cap 51.

## 7. Prueba de fuego

- **Tests citados** (todos en `tests_comunidades`):
  `modularidad_particion_trivial_es_cero_y_perfecta_analitica`,
  `modularidad_gamma_resolucion_analitica`,
  `modularidad_self_loops_simetria_y_paralelas`,
  `modularidad_escalar_pesos_no_cambia_q_y_nodos_ausentes`,
  `modularidad_pesos_invalidos_y_negativos`,
  `componentes_dos_pares_y_puente`, `componentes_vacio_aislados_y_dirigidos`,
  `lpa_separa_dos_trios_y_empates_deterministas`,
  `lpa_aislados_convergencia_y_errores`,
  `louvain_separa_dos_cliques_unidas_por_puente` (hito: 2 K4+puente → 2
  comunidades, Q=11/26 a mano),
  `louvain_determinismo_y_orden_de_insercion`, `louvain_vacio_aislados_y_self_loops`,
  `louvain_demo_graph_dani_solo_y_empate_de_optimos` (DOS óptimos Q=5/18),
  `louvain_jerarquia_monotonia_anidamiento_y_contraccion` (monotonía de Q,
  anidamiento fino⇒grueso, invariante de contracción),
  `louvain_limite_de_resolucion_gamma` (EL estrella),
  `louvain_recupera_ground_truth_sintetico` (3 anillos con cuerdas recuperados
  EXACTO), `louvain_los_pesos_cambian_la_particion` (puente w=100 rompe los
  tríos: {0,2},{1,4},{3,5} con Q=100/2809),
  `louvain_multigrafo_paralelas_equivalen_a_peso_sumado`,
  `louvain_parametros_invalidos_y_max_pasadas` (max_pasadas=1 NO rebaja
  calidad: la agregación repara), `louvain_stats_coherentes` (edges_scanned=96
  en el anillo), `errores_display_y_std_error`, `particion_accessores_y_display`.
- **Síntoma si el lector se salta el capítulo**: agrupa por componente (una
  «comunidad» gigante) o usa LPA sin pesos y cuenta tribus que nunca
  existieron; y cuando le pregunten «¿por qué esas comunidades y no otras?»
  no tendrá número ni criterio — la Parte V se queda sin la pieza que el cap
  51 necesita.

## 8. Trampas y errores comunes

1. **Comparar Q entre grafos distintos o entre γ distintos**: Q es «exceso
   sobre el azar» DENTRO de un grafo y una γ; 0.42 en una red no es «mejor»
   que 0.36 en otra (y γ≠1 cambia la escala).
2. **Contar el self-loop simple** (A_ii = s): rompe k_i, 2m y la conservación
   de Q en la contracción. Se detecta porque la Q de un nivel ≠
   `modularidad()` sobre el original.
3. **Deduplicar aristas paralelas** (la convención del cap 24): el multigrafo
   ACUMULA; deduplicar mide otro grafo. Se detecta con el test de
   equivalencia 3-paralelas ≡ peso 3.
- **Precisión de lenguaje (glosario)**: *componente* (alcanabilidad) vs
  *comunidad* (densidad vs azar); *Q* (valor de UNA partición) vs *Louvain*
  (algoritmo que la optimiza); *Σ_in* (peso interno contado por ambos
  extremos) vs *Σ_tot* (suma de grados de la comunidad, incluye lo externo);
  *γ/resolución* (cuánto exige el modelo nulo) vs *tamaño de comunidad*;
  *nivel* (una ronda fase-local+agregación) vs *partición final* (composición
  de niveles); *modularidad de una partición dada* (función verificable) vs
  *modularidad del resultado* (lo que reporta Louvain).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: SIN mirar el capítulo, escribir Q_γ de
  memoria y calcular a mano la Q de la partición perfecta de dos tríos con
  puente (pesos 1): 2m=14, In=6, K=7 por trío → Q=5/14. Verificación:
  doctest de `modularidad` + test `modularidad_particion_trivial_es_cero…`.
  Pistas: (1) ¿cuánto vale Σ_in si cada arista interna se cuenta por sus DOS
  extremos?, (2) ¿el puente entra en K del trío?, (3) ¿qué vale la trivial y
  por qué exactamente 0? Criterio: la fórmula y el 5/14 sin mirar.
- **analizar (intermedio, spacing con cap 24)**: explicar por qué LPA devuelve
  1 comunidad en dos K3 unidos por puente 2-3 pero 2 con el puente en 1-4
  (mismo algoritmo, mismo grafo espejado), siguiendo la primera pasada nodo a
  nodo; y qué cambia en Louvain (ΔQ del goteo < 0). Verificación: test
  `lpa_separa_dos_trios_y_empates_deterministas`. Pistas: (1) en qué orden se
  barren los nodos, (2) ¿de dónde salen los votos de la etiqueta PROPIA?,
  (3) ¿cuándo la salva la política de conservar la propia? Criterio: señalar
  el nodo exacto por el que gotea y por qué Louvain no gotea.
- **crear (experto, interleaving con cap 22 y gancho al 51)**: construir el
  anillo de k tríos con k como PARÁMETRO y encontrar empíricamente el k a
  partir del cual γ=1 funde pares (ΔQ de fundir = 1/48 − γ/72 con k=12;
  generalízalo), luego usar `particion_en(0)` y `particion_en(1)` del
  anillo con γ=2 para listar las comunidades «finas» y «gruesas» — el
  dendrograma que el cap 51 resumirá. Verificación:
  `louvain_limite_de_resolucion_gamma` y
  `louvain_jerarquia_monotonia_anidamiento_y_contraccion` como plantillas.
  Pistas: (1) ¿qué términos de Q dependen de k?, (2) ¿el ΔQ de fundir
  depende del tamaño del anillo?, (3) ¿por qué el anidamiento sólo se puede
  afirmar en la dirección fina ⇒ gruesa? Criterio: el k crítico previsto
  coincide con el observado; dendrograma correcto.

## 10. Preguntas abiertas (gancho al capítulo 26 — y al Vol. III)

1. `GrafoPonderado` materializa TODO el grafo ponderado en memoria: ¿qué pasa
   cuando el grafo no cabe? (cap. 26: proyección con pesos sobre el CSR del
   cap. 14, streaming, frontiers.)
2. Louvain puede devolver comunidades MAL CONECTADAS internamente (dos piezas
   unidas «por detrás» del supernodo): ¿quién lo garantiza? (Leiden/Traag
   2019, «cómo lo hace una BBDD real».)
3. ¿Para qué sirve un dendrograma de comunidades en una base de datos?
   (cap. 51 Vol. III: GraphRAG resume cada comunidad a varias granularidades.)
- **Términos nuevos de glosario**: partición, modularidad Q, modelo nulo de
  configuración, Σ_in/Σ_tot, resolución γ, label propagation, fase local,
  agregación, supernodo, self-loop (convención ×2), jerarquía/dendrograma,
  límite de resolución, multigrafo/paralelas acumuladas.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial obliga a RECONSTRUIR la fórmula de Q y
  el cálculo 5/14 desde la memoria (el enunciado no la regala); el experto
  deriva de nuevo el ΔQ de fundir tríos sin que el capítulo lo dé hecho para
  k genérico.
- **Spacing**: cap 22 (semántica estricta de pesos, rechazo eager — errores
  envueltos con `From<PathError>`); cap 24 (proyección materializada,
  determinismo por ids, `total_cmp`, BFS, `*Stats` medibles; y el CONTRASTE
  de convenciones Both-deduplica vs suma); Vol. I cap 4/5 (BFS, componentes).
- **Interleaving**: el intermedio mezcla heurística de votos (LPA) con función
  objetivo (Q) y orden de barrido; el experto cruza γ, tamaño del grafo,
  jerarquía y consumo futuro (cap 51) — no hay ejercicios clónicos.
- **Dificultad asimétrica**: una idea nueva por sección (particionar → medir
  con Q → moverse con ΔQ → agregar en niveles → el límite de γ); los
  ejercicios exigen recuperación, predicción y construcción.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb
  cap25` (22 tests + 4 doctests citados por nombre); `cargo test --doc`.
- **Citas**: Blondel, Guillaume, Lambiotte, Lefebvre, «Fast unfolding of
  communities in large networks», J. Stat. Mech. (2008) P10008 [arXiv:0803.0476];
  Newman-Girvan, Phys. Rev. E 69, 026113 (2004) (modularidad); Reichardt-
  Bornholdt, Phys. Rev. E 74, 016110 (2006) (γ); Fortunato-Barthélemy,
  PNAS 104(1):36-41 (2007) (límite de resolución); Raghavan-Albert-Kumara,
  Phys. Rev. E 76, 036106 (2007) (LPA); Traag-Waltman-van Eck, Sci. Rep.
  9:5233 (2019) (Leiden); Neo4j GDS docs (gds.louvain, gds.leiden,
  relationshipWeightProperty, includeSelfLoops).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: LPA goteando por un puente + el anillo de tríos donde Q «miente» para γ=1 (§6-§8 del capítulo).
- [x] Código ejecutable en workspace (22 tests + 4 doctests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (comunidad≠componente; nº de comunidades≠calidad; LPA sin métrica; óptimo local empatado; más peso≠más fusión; self-loop cuenta doble).
- [x] Ejercicios con solución verificable (tests del workspace).
- [x] ≥1 ejercicio de retrieval (fórmula Q y 5/14 desde memoria) y ≥1 de spacing (cap 24 proyección/total_cmp, cap 22 pesos estrictos).
- [x] Responde las preguntas críticas de CORPUS: «Modularidad; greedy Louvain» — qué es Q y por qué guía, y cómo funciona el greedy por ΔQ + agregación.
- [x] Anécdota verificada: Louvain 2008 (J. Stat. Mech. P10008; Universidad de Louvain; 2,6 M clientes de móvil; «outperform all other known community detection method in terms of computation time»).
- [x] Los valores Q usados en el capítulo son los analíticos de los tests (5/14, −17/98, 11/26, 2/3, 17/24, 7/12, 13/24, 5/18, 100/2809) — todos calculables a mano.
