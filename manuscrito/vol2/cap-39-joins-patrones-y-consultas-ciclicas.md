# Capítulo 39 — Joins, patrones y consultas cíclicas

> *«Treinta y ocho capítulos afilando cómo calcula LiraDB y ninguno preguntó cuánto pesaba lo invisible: las tuplas intermedias que un MATCH fabrica y descarta antes de darte la respuesta. Este capítulo las cuenta, las pone en la factura y presenta a la familia de joins que jamás las escribe.»*

## 39.0 La anécdota de la esquina

Palo Alto, primeros años de la década de 2010. En LogicBlox —una plataforma comercial de planificación y analítica— Todd Veldhuizen y compañía llevan en PRODUCCIÓN un join llamado leapfrog triejoin: es el caballo de batalla (*workhorse*) que ejecuta las consultas de clientes reales, con facturas reales detrás. Lo curioso no es que funcione; es que NADIE ha probado aún qué garantías tiene. El paper que lo documenta llega en 2014 —«Leapfrog Triejoin: A Simple Worst-Case Optimal Join Algorithm», ICDT 2014— y demuestra que el algoritmo ya estaba en la familia correcta todo ese tiempo: es **worst-case optimal**, salvo un factor polilogarítmico. Y el remate que a un ingeniero le debería poner la piel de gallina: una década después, el ICDT le concede el Test-of-Time Award 2024 (databasetheory.org/node/150) con una descripción que no necesita adornos — un algoritmo implementado ANTES de que se descubrieran sus garantías de optimalidad. La producción puede ir por delante de la teoría; lo peligroso es ir por delante SIN saberlo.

Hazte la pregunta que ellos no necesitaban hacerse porque el negocio funcionaba, y que este capítulo te hace a nuestra criatura: ¿cuánto paga LiraDB HOY en resultados intermedios que ni siquiera ve? Tu suite está verde; ninguna prueba mide tuplas fantasma. No lo adivines. Cuéntalo.

## 39.1 Objetivo

Este capítulo cobra tres deudas explícitas: el CSR ordenado del cap. 14 (las adyacencias YA están ordenadas — hoy eso deja de ser un detalle de layout y se convierte en estructura de datos), el «joins reales WCOJ» prometido por el cap. 20 (hoy `ExpandOp` expande fila a fila sin saber su propio nombre) y el gancho saliente del cap. 38: «las columnas aceleran CÓMO lees; los worst-case optimal joins cambian QUÉ calculas». Al terminar tendrás:

1. **La formalización de `ExpandOp`**: `AdyacenciasOrdenadas` en `crates/vol2-liradb/src/cap39_joins.rs` (1.470 líneas, std puro, cero dependencias nuevas), heredera directa del espíritu de `SubgrafoAcotado` (cap. 38): ids ordenados, listas vecinas ordenadas y deduplicadas. Con ella, `expansion_dos_saltos_plana` (la referencia) y `intermedios_plan_binario` (el contador que convierte el rumor del survey «Skew Strikes Back», SIGMOD Record 42(4), 2014, en una cifra propia).
2. **Joins binarios honestos**: `triangulos_join_binario` materializa de VERDAD el intermedio (un `Vec` con capacidad exacta) y devuelve su factura junto al resultado; `triangulos_fuerza_bruta` O(n³) es el terreno de verdad contra el que TODO se equivoca-testea.
3. **LeapFrog simplificado** — la pregunta crítica del corpus respondida con código: `BuscadorSalto` (**seek** = salto exponencial + `slice::binary_search`, O(log n) peor caso garantizado), `BuscadorSalto::frontera_comun` (intersección coordinada por seeks) y `TriangulosWcoj::enumerar` (backtracking por variables, orden de variables ESTÁTICO a→b→c, cero intermedios materializados, contador `pasos_buscador`).
4. **La cota AGM medible**: **cota AGM** `cota_agm_triangulos(m)` = ⌊m^(3/2)⌋ calculada EXACTA con enteros u128 (Newton entero, ajuste ±1) — brújula del peor caso, nunca promesa de velocidad.
5. **Consultas recursivas acotadas**: `cierre_transitivo(origenes, presupuesto)` por punto fijo iterativo, reutilizando `BitSet`, `Presupuesto` y `MotivoParada` del cap. 26 tal cual — termina en grafos CON ciclos y equivale al BFS ya existente, exigido por test.
6. **Factorized execution extendida al JOIN**: `ResultadoFactorizadoTriangulos` — prefijos a y b compartidos, multiplicidades visibles — extendiendo el patrón de `ExpansionFactorizada` (cap. 38) al resultado de un join cíclico.
7. **El cuarto `[[bench]]`** (`benches/bench_joins.rs`, aditivo, `harness = false`) y la suite completa: **864 + 14 = 878 tests ALL_GREEN**, goldens intactos, cero cambios en caps. 7-38.

## 39.2 Problema

En realidad ya tienes 878−14 = 864 tests verdes, ACID, WAL, MVCC, CLI, lectura columnar y aparato de medición. Y aun así, un `MATCH (a)-[:conoce]->(b)-[:conoce]->(c)` atraviesa `ExpandOp` (cap. 20) fila a fila: por cada candidato expande sus vecinos con `out_edges`/`get_edge`, fabricando tuplas (a,b,c) que la mayoría se descartan en el siguiente operador. Tu suite responde «¿es CORRECTO?» y calla sobre la única pregunta que importa aquí: ¿CUÁNTAS tuplas fantasma materializó tu plan antes de darte 56 respuestas? Antes de construir nada, desactivemos cinco ideas equivocadas que suelen venir con el tema:

1. **«El coste de una consulta lo decide el tamaño de la ENTRADA.»** No: puede decidirlo el RESULTADO — los triángulos de un grafo son O(n^1,5), menos que las n² parejas posibles — y sobre todo los INTERMEDIOS, que pueden superar a la entrada y al resultado JUNTOS. El survey «Skew Strikes Back» lo formula sin rodeos: los motores tradicionales evalúan joins por parejas, y eso les fuerza a Ω(N²) en alguna instancia de la propia consulta triángulo (SIGMOD Record 42(4), 2014).
2. **«Worst-case optimal significa que va más rápido siempre.»** No: es una GARANTÍA sobre el PEOR caso — nunca eres polinómicamente peor que el tamaño del resultado. Con datos selectivos y acíclicos, un plan binario clásico suele aguantar o ganar (Veldhuizen, ICDT 2014). Lo vas a MEDIR en tu propio bench: delta modesto incluido, reportado sin vergüenza.
3. **«`ExpandOp` es primitivo; el join es otra cosa.»** No: `ExpandOp` ES un **index nested-loop join** nodo×adyacencia — por cada fila externa, sondea un índice (la lista de vecinos) y emite filas. Formalizarlo no lo denigra: es lo que permite ELEGIR OTRO PLAN.
4. **«Recursivo = bucle infinito cuando el grafo tiene ciclos.»** No: un cierre por punto fijo con visitados TERMINA por construcción, y el presupuesto del cap. 26 lo hace demostrablemente acotable. Lo que es infinito es el recursivo SIN visitados.
5. **«La factorización cambia el significado del resultado.»** No: mismo multiconjunto lógico, menos celdas físicas — la lección del cap. 38 generalizada aquí a RESULTADOS DE JOIN, con su test-tesis de equivalencia.

Y debajo, la pregunta crítica del corpus: «LeapFrog Triejoin simplificado». Respuesta con criterio de aceptación: una implementación mínima pero HONESTA — tries sobre las listas ordenadas que LiraDB YA tiene, búsqueda por saltos, frontera común entre niveles, orden estático — demostrada EQUIVALENTE al plan binario y a la fuerza bruta, con su trabajo contado y comparado contra la cota AGM. Sin equivalencia testeada no hay capítulo: sería magia, no ingeniería.

## 39.3 Modelo mental: dos orientaciones de plan — y quién paga la factura

Un solo eje ordena el capítulo: **join-oriented × materialización de intermedios**. Los planes clásicos atan TABLAS en un árbol de uniones binarias, y cada unión ESCRIBE su resultado; los worst-case optimal atan VARIABLES en un backtracking sobre tries, y nadie escribe nada intermedio:

```text
  JOIN-ORIENTED (hoy, caps. 19-21):          VARIABLE-ORIENTED (objetivo, cap. 39):
  NodeScan(a) → Expand → Expand              liga a → liga b → liga c
      R(a,b) ⋈ S(b,c)                        tries ordenados sobre las MISMAS adyacencias
        ↓ MATERIALIZA N₁                     frontera común por nivel: seek coordinado
      N₁ = Σ_b out(b)·out(b)  (K₈: 392)      (nadie escribe N₁)
        ⋈ T(a,c)  →  56 triángulos           56 triángulos directamente
```

Sobre ese panel, la escalera de ocho peldaños — los ejercicios te la van a pedir DE MEMORIA, sin pistas:

```text
peldaño 1  expand como join    → ExpandOp ES un index nested-loop join; formalizarlo abre la pregunta
peldaño 2  joins binarios      → el dogma System R: descomponer el patrón en uniones de dos tablas
peldaño 3  explosión           → K₈: 392 caminos intermedios para 56 respuestas
peldaño 4  muchos-a-muchos     → el hub concentra el fan-out; el patrón triángulo es CÍCLICO
peldaño 5  cota AGM            → ⌊m^(ρ*)⌋ acota el PEOR resultado posible (ρ* = 3/2 para el triángulo)
peldaño 6  leapfrog            → tries + seeks + frontera común: nadie escribe N₁
peldaño 7  recursivas          → cierres transitivos por punto fijo con presupuesto
peldaño 8  factorización       → compartir prefijos TAMBIÉN en resultados de join
```

Y la frontera, declarada antes de escribir código — la misma disciplina «lo que sí / lo que aún no» de los caps. 33-34/38:

```text
Lo que SÍ se hace hoy:   capa EDUCATIVA aparte (módulo propio, como la lectura analítica del 38):
                         formalizar expand, contar intermedios, leapfrog 2-3 aristas,
                         cota AGM como brújula, cierre transitivo con presupuesto,
                         factorización de resultados de join, bench comparativo
Lo que AÚN NO:           operadores nuevos en el Executor Volcano · integración con explain
                         orden DINÁMICO de variables · Generic Join multi-way genérico (NPRR)
                         sintaxis recursiva en LiraQL · paralelismo · estadísticas nuevas
```

El momento ¡ajá! perseguido: los intermedios pueden ser más grandes que la entrada Y que el resultado JUNTOS — y existe una familia de planes que jamás los escribe, porque atan variables en vez de unir tablas. LiraDB ya tenía medio camino hecho: adyacencias ordenadas desde el cap. 14.

## 39.4 Primera solución

La versión que todo el mundo escribe —incluido nuestro yo de ayer, disfrazado de `ExpandOp`— es el plan System R para el patrón triángulo: materializar R(a,b), unir con S(b,c) por b, filtrar con T(a,c):

```rust
// Esqueleto de `triangulos_join_binario` (cap39_joins.rs)
let mut intermedios: Vec<(usize, usize, usize)> = Vec::with_capacity(esperados);
for (a, vecinos) in adj.adj_out.iter().enumerate() {
    for &b in vecinos {                        // R(a,b)
        for &c in &adj.adj_out[b] {            // S(b,c): aquí nace N₁
            intermedios.push((a, b, c));       // ← la memoria invisible
        }
    }
}
for &(a, b, c) in &intermedios {
    if adj.contiene_arista(a, c) && a < b && b < c {   // semi-join T(a:c) + canonizar
        triangulos.push((a, b, c));
    }
}
```

Correcto, legible, y pasa los tests. El filtro por `contiene_arista(a, c)` es un **semi-join** (conserva las filas de la izquierda que tienen pareja a la derecha, sin añadir columnas), y la canonización `a < b < c` hace que cada triángulo aparezca UNA vez — el mismo cuidado que el cap. 25 necesita para no contarlos ×6. La diferencia con el cap. 38 es que aquí la materialización es REAL y la función devuelve su FACTURA: `ResultadoBinario` trae `intermedios_materializados` (todo lo que el primer join escribió), `tuplas_semi_filtradas` (las supervivientes del filtro) y los triángulos. La escalera de conteos es la tesis del capítulo en miniatura: 392 ≥ 336 ≥ 56 en K₈. Un contador de trabajo delata a la primera solución mejor que cualquier profiling.

## 39.5 Sus límites

1. **K₈ a mano, cuentas fijas.** El grafo completo bidireccional de 8 nodos tiene 56 aristas dirigidas (28 pares × 2 sentidos). Cada nodo tiene grado entrante y saliente 7, así que el primer join materializa 8 × 7 × 7 = **392** caminos (a,b,c); el semi-filtro deja 336; la canonización, **56** triángulos (C(8,3)). Siete tuplas fantasma por respuesta — en el grafo AMABLE del capítulo:

```text
Cuentas de K₈ a mano (verificables con lápiz):

  entrada         56 aristas    (28 pares × 2 sentidos)
  intermedios    392 tuplas     Σ_b in(b)·out(b): 8 nodos × 7 × 7
  semi-filtro    336 tuplas     sobreviven las que además tienen la arista a→c
  triángulos      56            tras canonizar a<b<c   (= C(8,3))
  cota AGM       419            ⌊56^1,5⌋ ≥ 56: la brújula nunca se queda corta
```

2. **La estrella con hub: la explosión total.** Una estrella bidireccional de k=32 hojas tiene 64 aristas y CERO triángulos (las hojas no se tocan entre sí). El plan binario aún materializa k² + k = 1.056 intermedios. Ratio intermedios/resultado: división entre cero — INFINITO. El test `hub_concentrador_explota_los_intermedios` fija la cifra finita que sí se puede comparar, contra la ENTRADA: (k+1)/2 = **16,5 intermedios por arista** para no dar NI UNA respuesta.
3. **La ley general.** No es mala suerte del ejemplo: cualquier plan que evalúe el triángulo por parejas paga Ω(n²) en alguna instancia, mientras el resultado nunca supera O(n^1,5) (SIGMOD Record 42(4), 2014). La brecha intermedios-vs-resultado es estructural, no patológica.
4. **La memoria es un recurso físico.** Cada tupla intermedia son bytes escritos, releídos y descartados — la contralmirante Hopper del cap. 38 sigue cobrando por trayecto. Y a diferencia del layout de columnas, aquí el problema no es DÓNDE están los bytes sino QUE EXISTEN.

## 39.6 Solución evolucionada

Ocho gestos —un peldaño de la escalera cada uno—, cada uno con alternativa descartada.

**Gesto 1: formalizar la base física, no cambiarla.** `AdyacenciasOrdenadas` es una vista posicional sobre las adyacencias que LiraDB ya posee: ids ordenados ascendentes (dos llamadas dan la misma vista byte a byte — la lección Mykowicz heredada de caps. 34/38), listas de vecinos ordenadas y deduplicadas, y el grado entrante precalculado porque el contador Σ_b in(b)·out(b) lo necesita. Descartado tocar la CSR del store: capa educativa hexagonal, como en el cap. 38.

**Gesto 2: contar antes de curar.** `intermedios_plan_binario` computa Σ_b in(b)·out(b) SIN materializar nada, con `checked_add` como su hermana `conteo_dos_saltos` del cap. 38: el desborde se NOMBRA, nunca envuelve. Ley de la casa desde el cap. 34: sin medida propia, la explosión sería un rumor de paper.

**Gesto 3: terreno de verdad primero.** `triangulos_fuerza_bruta` O(n³) no explota ni el orden ni prefijos compartidos: es el juez contra el que binario y WCOJ se equivoca-testean como MULTICONJUNTO exacto. Descartado confiar en «se ve bien»: los off-by-one de canonización son silenciosos.

**Gesto 4: el seek.** `BuscadorSalto::primer_mayor_o_igual(lista, x)` encuentra el índice del primer elemento ≥ x con DOS fases: un galope exponencial (compara en posiciones 1, 2, 4, …) que acota el bloque, y `slice::binary_search` de std dentro del bloque. Peor caso O(log n) GARANTIZADO. Descartada la interpolación del paper original: su coste depende de supuestos de distribución que con std puro no podemos sostener. Convención declarada del contador: +1 por salto, +1 por búsqueda binaria delegada — sin convención explícita, comparar contra la cota AGM sería trampa. Y el **trie** —la estructura de datos del algoritmo— no hay que construirlo:

```text
el trie sale GRATIS de las listas ordenadas del cap. 14:

out(3) = [5 | 7 | 12 | 19]          ← ya ordenada y deduplicada
seek(≥ 8):  5 ✗ → 7 ✗ → [12] ✓     en O(log n), sin escanear nada

un trie es eso: valores ordenados por nivel; ligar la variable b
es DESCENDER por el nivel correcto, no recorrer una tabla
```


**Gesto 5: la frontera común.** `frontera_comun(listas, desde)`: todas las relaciones se posicionan con seeks y luego el MÁXIMO de los cursores manda — cada lista rezagada salta hasta él; cuando convergen, ése es el primer valor común. Es la «common frontier» de Veldhuizen (ICDT 2014) en versión mínima:

```text
nivel c: out(b) y out(a) votan su candidato ≥ desde = b+1

out(b):  … 4 [7] 9  12 …      cursor → 7
out(a):  … 3 [7]? 11 …        seek hasta 7
                ▲
el máximo manda; la rezagada salta; convergen ⇒ FRONTERA = 7
emitir (a,b,7) y seguir desde 8 — cursores, no copias: nada escrito
```

**Gesto 6: backtracking por variables, orden estático.** `TriangulosWcoj::enumerar` liga a → b → c sobre el **orden de variables** elegido a mano: nivel b avanza con seeks dentro de out(a) pidiendo b > a (la condición va DENTRO del seek, no en un filtro posterior); nivel c resuelve con la **frontera común** entre out(b) y out(a). Ahí muere N₁: los Σ in·out caminos jamás se escriben. Descartada la reordenación dinámica tipo greedy: exigiría estadísticas por prefijo que el catálogo del cap. 21 no tiene — fingirla sería humo (contrato §5.6).

**Gesto 7: la brújula AGM.** `cota_agm_triangulos(m)` = ⌊m^(3/2)⌋, exacta con enteros (m³ cabe en u128 hasta m = 2⁴² — cuatro billones de aristas, más RAM de la que existe; pasado el umbral satura a u64::MAX en vez de envolver). El fundamento: el peor tamaño del resultado de un join está acotado por m^(ρ\*) donde ρ\* es la **cubrimiento fraccional de aristas** del hipergrafo del patrón (Atserias-Grohe-Marx, FOCS 2008; versión revista SICOMP 42(4), 2013); para el triángulo ρ\* = 3/2, y la raíz geométrica es la desigualdad de Loomis-Whitney (Bull. AMS 55, 1949). NPRR (PODS 2012, pp. 37-48) dio el primer algoritmo que NUNCA supera esa cota. Nosotros la usamos como BRÚJULA del peor caso y contamos nuestros pasos para comparar — nunca como promesa de velocidad.

**Gesto 8: factorizar el RESULTADO del join.** `ResultadoFactorizadoTriangulos` extiende `ExpansionFactorizada` (cap. 38) al f-tree de dos niveles del triángulo — la **representación factorizada** de Olteanu-Závodný (ICDT 2012; versión revista ACM TODS 40(1), mar-2015):

```text
prefijos_a:   [0, 1, 2, 3, 4, 5]            multiplicidad_a: [21,15,10,6,3,1] = C(7-a,2)
              │ CSR inicio_b
prefijos_b:   [21 slots (a,b) con b>a]      multiplicidad_b por slot
              │ CSR inicio_c
hojas_c:      [56 valores c — UNA celda POR TRIÁNGULO]

celdas físicas = 6 + 21 + 56 = 83 · planas = 3×56 = 168 · ahorro 50,6 %
```

La **multiplicidad** hace visible el compartir: sumarla por nivel da SIEMPRE las filas lógicas — el anti-doble-recuento hecho número. Y agregar sobre esta representación NI SIQUIERA expande las tuplas: responder «¿cuántos triángulos por valor de a?» es copiar multiplicidades. Para los grafos cíclicos que el matching también debe atravesar, `cierre_transitivo` cierra el círculo conceptual: **punto fijo** iterativo sobre la frontera con `BitSet` de visitados y `Presupuesto`/`MotivoParada` del cap. 26 — el **cierre transitivo** termina en grafos CON ciclos y su motivo de parada es explícito (`WITH RECURSIVE` de SQL:1999 y GQL, ISO/IEC 39075:2024, quedan como contexto estándar y anzuelo al Vol.III: aquí hay API y concepto, no sintaxis nueva).

## 39.7 Código completo ejecutable

Todo vive en dos piezas nuevas — y solo dos: `crates/vol2-liradb/src/cap39_joins.rs` (1.470 líneas, std puro, 14 tests) y `benches/bench_joins.rs`. El cableado es el mínimo posible: `pub mod cap39_joins; pub use cap39_joins::*;` en `lib.rs` (módulo 30), y la CUARTA entrada `[[bench]]` en `Cargo.toml` — cero dependencias nuevas, cero cambios en caps. 7-38:

```toml
[[bench]]
name = "bench_joins"
harness = false
```

Las firmas que sostienen el edificio:

```rust
pub struct AdyacenciasOrdenadas { /* ids ordenados + adj_out posicional + grado_in */ }
impl AdyacenciasOrdenadas {
    pub fn desde_store(store: &MemoryStore) -> Self;
    pub fn desde_store_acotado(store: &MemoryStore, max_nodos: usize) -> Self;
    pub fn desde_aristas(num_nodos: usize, aristas: &[(usize, usize)]) -> Self;
    pub fn vecinos_out(&self, pos: usize) -> &[usize];      // ordenados, deduplicados: el trie
    pub fn contiene_arista(&self, a: usize, b: usize) -> bool;
}
pub fn expansion_dos_saltos_plana(adj: &AdyacenciasOrdenadas) -> Vec<(usize, usize, usize)>;
pub fn intermedios_plan_binario(adj: &AdyacenciasOrdenadas) -> u64;   // Σ_b in(b)·out(b)
pub struct ResultadoBinario { pub triangulos: Vec<(usize, usize, usize)>,
                              pub intermedios_materializados: u64,
                              pub tuplas_semi_filtradas: u64 }
pub fn triangulos_join_binario(adj: &AdyacenciasOrdenadas) -> ResultadoBinario;
pub fn triangulos_fuerza_bruta(adj: &AdyacenciasOrdenadas) -> Vec<(usize, usize, usize)>;
impl BuscadorSalto {
    pub fn primer_mayor_o_igual(&mut self, lista: &[usize], x: usize) -> Option<usize>;
    pub fn frontera_comun(&mut self, listas: &[&[usize]], desde: usize) -> Option<usize>;
    pub fn pasos(&self) -> u64;
}
pub struct TriangulosWcoj { pub triangulos: Vec<(usize, usize, usize)>, pub pasos_buscador: u64 }
impl TriangulosWcoj { pub fn enumerar(adj: &AdyacenciasOrdenadas) -> Self; }  // orden estático a→b→c
pub fn cota_agm_triangulos(num_arista: u64) -> u64;   // ⌊m^(3/2)⌋ EXACTO (u128 + Newton)
pub fn cierre_transitivo(adj: &AdyacenciasOrdenadas, origenes: &[usize],
                         presupuesto: Presupuesto) -> (BitSet, MotivoParada);
pub struct ResultadoFactorizadoTriangulos { /* prefijos_a/b + multiplicidades + CSR + hojas */ }
impl ResultadoFactorizadoTriangulos {
    pub fn desde_triangulos(triangulos: &[(usize, usize, usize)]) -> Self;
    pub fn filas_logicas(&self) -> u64;   pub fn celdas_fisicas(&self) -> u64;
    pub fn ahorro_porcentaje(&self) -> f64;
    pub fn por_cada_tupla(&self, visitar: impl FnMut(usize, usize, usize));
}
pub fn informe_joins_reproducible_sobre_mini(store: &MemoryStore) -> String; // contadores, NO tiempos
```

Cuatro decisiones visibles en esas firmas, con su porqué:

- **La materialización del binario es REAL.** `triangulos_join_binario` reserva capacidad exacta tomada del contador y escribe TODAS las tuplas intermedias en un `Vec`; un `debug_assert_eq!` cruza factura aritmética contra lo materializado. Esconder la explosión tras un contador sería precisamente el auto-engaño que el capítulo denuncia.
- **El contador de pasos tiene convención escrita.** +1 por salto del galope, +1 por búsqueda binaria delegada. Sin convención declarada, «pasos ≤ cota·log» sería una comparación sin unidades.
- **Los TIEMPOS no viven en el módulo.** `informe_joins_reproducible_sobre_mini` imprime conteos reproducibles byte a byte (pineado por test, sin ni un `µ`); el cronómetro es de criterion, regla del cap. 34.
- **Los vecinos son POSICIONES, no ids globales.** Igual que `SubgrafoAcotado` en el cap. 38: ningún recorrido puede salirse de la caja, y el trie queda contiguo y barato de sondear.

## 39.8 Prueba de fuego

Primero el bucle rápido, en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap39

running 14 tests
test cap39_joins::tests_joins::agm_bound_acota_el_resultado_en_varios_subgrafos ... ok
test cap39_joins::tests_joins::buscador_primer_mayor_o_igual_casos_exactos ... ok
test cap39_joins::tests_joins::cierre_transitivo_coincide_con_bfs_existente ... ok
test cap39_joins::tests_joins::cierre_transitivo_para_por_profundidad_y_presupuesto ... ok
test cap39_joins::tests_joins::expand_es_index_nested_loop_produce_las_mismas_tuplas ... ok
test cap39_joins::tests_joins::factorizacion_triangulos_equivale_a_las_tuplas_planas ... ok
test cap39_joins::tests_joins::factorizacion_triangulos_filas_logicas_vs_celdas ... ok
test cap39_joins::tests_joins::frontera_comun_es_la_interseccion_de_candidatos ... ok
test cap39_joins::tests_joins::hub_concentrador_explota_los_intermedios ... ok
test cap39_joins::tests_joins::informe_joins_reproducible_sobre_mini ... ok
test cap39_joins::tests_joins::intermedios_del_plan_binario_cuentas_conocidas_k8 ... ok
test cap39_joins::tests_joins::triangulos_binario_iguales_a_la_fuerza_bruta ... ok
test cap39_joins::tests_joins::wcoj_pasos_acotados_por_agm_en_k8 ... ok
test cap39_joins::tests_joins::wcoj_triangulos_iguales_al_binario_y_a_la_fuerza_bruta ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 864 filtered out; finished in 0.00s
```

Catorce verdes, workspace entero en **878 ALL_GREEN** con goldens intactos. Cuatro son TESIS, no comprobaciones accesorias: `expand_es_index_nested_loop_produce_las_mismas_tuplas` corre el pipeline VOLCANO real (`NodeScanOp → ExpandOp → ExpandOp`) y exige que produzca lo mismo que el join formalizado — `ExpandOp` era un index nested-loop join aunque nadie lo llamara así; `wcoj_triangulos_iguales_al_binario_y_a_la_fuerza_bruta` compara los TRES planes como multiconjunto sobre K₈, rueda, subgrafo y dataset completo; `factorizacion_triangulos_equivale_a_las_tuplas_planas` recorre la estructura factorizada y exige el mismo multiconjunto — compartir prefijos no cambia el significado, sólo las celdas; y `cierre_transitivo_coincide_con_bfs_existente` confronta el punto fijo contra el BFS de los caps. 22/26 desde tres orígenes distintos. Velocidad sin estas equivalencias sería marketing.

Ahora los números. Comando real, acto explícito tuyo (nunca en CI):

```text
$ cargo bench -p vol2-liradb --bench bench_joins
```

Hardware declarado — el de la casa: Intel Xeon E5-2682 v4 @ 2,50 GHz, Linux, rustc 1.96.0, perfil release, `sample_size(30)`, warm-up 500 ms, medición 4 s, `Throughput::Elements`. Los denominadores del throughput: aristas dirigidas en los dos primeros grupos (11.906 y 2.048), filas lógicas (540) en el tercero. Medianas:

| Grupo | Plan | Mediana |
|---|---|---|
| `binario_vs_wcoj_regular` (800 nodos / 11.906 aristas) | binario materializa intermedios | **2,36 ms** |
| ídem | WCOJ leapfrog orden estático | **1,15–1,19 ms** |
| **Delta** | | **~2×** |
| `binario_vs_wcoj_hub_skew` (rueda de 513 nodos / 2.048 aristas) | binario materializa intermedios | **2,14 ms** |
| ídem | WCOJ leapfrog orden estático | **72 µs** |
| **Delta** | | **~29×** |
| `enumeracion_factorizada_vs_plana` (540 filas lógicas) | plano recorre cada tupla | **570 ns** |
| ídem | factorizado suma multiplicidades | **382 ns** |
| **Delta** | | **~1,5×** |

Y los contadores de trabajo (del módulo, deterministas, pineados por tests — no cronometran, CUENTAN):

| Escenario | Aristas | Intermedios binario | Resultado | Ratio int./res. |
|---|---|---|---|---|
| K₈ bidireccional | 56 | 392 | 56 triángulos | 7× |
| Rueda con hub (512 hojas) | 2.048 | **266.752** | 512 triángulos | **521×** |
| Grafo regular del bench | 11.906 | 188.624 | 540 triángulos | 349× |
| Estrella k=32 | 64 | 1.056 | **0** | ∞ (16,5/arista) |

Dos lecturas, ambas obligatorias:

**El experimento del capítulo es el hub-skew, y ahí el WCOJ arrasa: ~29×.** El binario escribe y relee 266.752 tuplas para devolver 512 triángulos (ratio 521×); mientras tanto el leapfrog consume `pasos_buscador = 12.832` — muy por debajo de la cota AGM ⌊2048^1,5⌋ = **92.681**. El binario no pierde por instrucciones: pierde por MEMORIA movida — la misma factura de trayectos del cap. 38, cobrada en tuplas.

**Pero el grupo regular es la honestidad del capítulo: delta de solo ~2× con intermedios 349× mayores que el resultado.** Si esperabas ×349 de speedup, la misconcepción nº2 acaba de cobrarse tu predicción — nos pasó a nosotros primero. ¿Por qué tan poco? Porque en un grafo uniforme los 188.624 intermedios son escrituras secuenciales en vectores contiguos, cache-friendly y baratas por unidad; los seeks del leapfrog pagan constantes por paso; y la garantía worst-case optimal habla del PEOR caso, no del caso medio (Veldhuizen, ICDT 2014). El ratio intermedios/resultado NO predice el speedup: predice el riesgo. Cuando el skew concentra el fan-out, el riesgo cobra intereses de ×29.

La cota AGM como brújula, con su holgura a la vista: en K₈, trabajo del buscador ≤ 2.933 pasos (cota·log: 419×7) para 56 triángulos ≤ 419 de cota; en el mini dataset la holgura es GRANDE y el test `agm_bound_acota_el_resultado_en_varios_subgrafos` la reporta sin vergüenza — la cota nunca se queda corta, casi nunca se queda cerca. Y la factorización, dos medidas: K₈ baja de 168 a 83 celdas (**50,6 %**) con multiplicidades [21,15,10,6,3,1] calculadas a mano; el grafo regular del bench, de 1.620 a 1.328 (**18 %**) — y agregar sobre ella ni siquiera expande las 540 tuplas: 570 ns planos contra 382 ns factorizados.

## 39.9 Qué hemos sacrificado

1. **Capa educativa, no motor.** Ni `optimize()` ni `PhysicalOperator` tocados: el Executor Volcano (cap. 20) sigue expandiendo fila a fila y `explain` no menciona WCOJ. Enchufarlo cambiaría planes físicos y rompería goldens — proyecto de otra Parte.
2. **Orden de variables ESTÁTICO (a→b→c).** La elección dinámica exigiría estadísticas por prefijo que el catálogo del cap. 21 no posee; fingirla sería humo. Declarada en «Para profundizar».
3. **LeapFrog limitado a patrones de 2-3 aristas.** No hay Generic Join multi-way genérico de NPRR ni tries multi-nivel completos: el mensaje cabe en el triángulo; la maquinaria general, no en un capítulo.
4. **Sin paralelismo.** Un core, backtracking secuencial. La paralelización por trozos (morsel-driven) es natural aquí — y por eso espera al modelo secuencial medido (y al cap. 40).
5. **Sin sintaxis recursiva en LiraQL.** `cierre_transitivo` es API y concepto; `WITH RECURSIVE` (SQL:1999) y GQL (ISO/IEC 39075:2024) quedan citados como contexto estándar. Parser nuevo diluiría el foco (joins).
6. **Sin estadísticas nuevas.** El catálogo del cap. 21 permanece intacto; ningún histograma de grados, ninguna cardinalidad estimada. Lo que falta para predicar órdenes de variables queda APUNTADO, no fingido.

## 39.10 Cómo lo hace una BBDD real

Nada de lo que hiciste es exótico. **Neo4j** ejecuta Cypher con un operador expand que es, literalmente, nuestro index nested-loop join de siempre — por eso sus guías de rendimiento hablan de controlar el fan-out y de `shortestPath` para caminos: la familia join-oriented con décadas de rodaje. **DuckDB** representa al campeón join-oriented bien afinado: hash joins sobre almacenamiento columnar que, con datos amables y acíclicos, aplastan a casi cualquier alternativa — coherente con nuestro delta modesto en el grafo regular. **LogicBlox** es la anécdota cerrada: LFTJ —el leapfrog triejoin de Veldhuizen— lleva años en producción comercial, publicado en ICDT 2014 y premiado como Test-of-Time 2024; producción primero, garantías después, reconocimiento al final. Y para GRAFOS, la referencia conceptual es **Kùzu** (Jin, Feng, Chen, Liu y Salihoğlu, CIDR 2023, CC-BY 4.0): adoptó exactamente nuestras dos piezas —joins worst-case optimal Y ejecución factorizada— como parte central de su motor. Nota de presente según ADR-001, con el relato verificado: nacida en Waterloo, publicada en CIDR 2023, adquirida por Apple (octubre de 2025), repo archivado, comunidad continuada en el fork LadybugDB; citamos el paper CIDR 2023, jamás «renombrada a Ladybug». Nuestro módulo es clean-room: implementa el CONCEPTO publicado, cero código copiado.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (20+39): cuenta los caminos de 2 saltos desde un nodo del mini dataset DOS veces — con la **frontera común** (`BuscadorSalto::frontera_comun` sobre `[vecinos_out(pivote)]`) y con el pipeline `ExpandOp` fila a fila — y compara totales y forma. Criterio: mismos conjuntos, y anota cuántos pasos de búsqueda gastó el seek frente a cuántas filas tiró el operador. Verificación: patrón de `expand_es_index_nested_loop_produce_las_mismas_tuplas`.
- *Intermedio* (21+34+39): PREDECIR por escrito los intermedios del plan binario del dataset de referencia (grados del generador del cap. 34 en la mano, estimación estilo catálogo del cap. 21) ANTES de correr nada; luego verifica con `intermedios_plan_binario`. Si falló tu predicción, explica QUÉ término ignoraste (¿dedup de aristas paralelas? ¿colas de distribución de grado?) — eso es el wisdom, no el acierto.
- *Experto* (24/25+39 y 38+39): extiende el leapfrog al 4-ciclo `(a)-[:e]->(b)-[:e]->(c)-[:e]->(d)-[:e]->(a)`: calcula a mano su ρ\* (cuatro hiperaristas de tamaño 2 — ¿qué peso fraccional mínimo cubre cada vértice?), deduce la cota AGM resultante, impleméntala sobre `AdyacenciasOrdenadas` con equivalencia contra fuerza bruta, y compárala contra el resultado medido. Extra (38+39): diseña la representación factorizada del 4-ciclo y sus multiplicidades.

## 39.11 Lo que te llevas

- **`ExpandOp` ES un index nested-loop join.** Formalizarlo no lo denigra: es lo que permite elegir OTRO plan. El plan no es el destino.
- **Los intermedios pueden superar a la entrada y al resultado juntos**: 266.752 tuplas para 512 triángulos (521×); en la estrella, infinito — tuplas fantasma para una respuesta vacía.
- **La cota AGM ⌊m^(ρ\*)⌋ es brújula, no promesa**: ρ\* = 3/2 para el triángulo, raíz Loomis-Whitney (1949). Acota el PEOR caso; la holgura se reporta.
- **Leapfrog simplificado**: tries GRATIS sobre las adyacencias ordenadas del cap. 14, seek log-garantizado (galope + `binary_search`), **frontera común** por niveles — atan variables, jamás escriben N₁.
- **Honestidad medida**: grafo regular → delta ~2× (worst-case optimal ≠ más rápido siempre); hub-skew → ×29 (donde el binario se hunde). El ratio intermedios/resultado predice RIESGO, no speedup.
- **La factorización también vale para resultados de JOIN**: K₈ 83 vs 168 celdas (50,6 %); agregar sobre multiplicidades ni siquiera expande tuplas (382 ns vs 570 ns).
- **Recursivo ≠ bucle infinito**: punto fijo con visitados TERMINA en ciclos, y el presupuesto del cap. 26 lo hace acotable — equivalente al BFS, exigido por test.

## 39.12 Ojo, cuidado con…

- **«Worst-case optimal siempre gana.»** Tu propio bench dice ~2× en topología amable. La garantía es del peor caso; el caso medio lo decide el workload.
- **Off-by-one en seeks.** El primer elemento ≥ x tiene casos borde traicioneros (vacía, golpe exacto, hueco, cola). El test `buscador_primer_mayor_o_igual_casos_exactos` existe porque estos bugs son silenciosos.
- **Contar el triángulo ×6.** Sin canonizar `a<b<c`, cada triángulo aparece seis veces — el mismo error que acechaba a la modularidad del cap. 25.
- **Contar la multiplicidad dos veces.** Iterar hojas Y sumar multiplicidades como si fueran filas extra corrompe el conteo; el test-tesis de equivalencia te cazará — déjalo trabajar.
- **Recursivo sin visitados.** El punto fijo termina CON visitados; sin ellos, el ciclo 0→1→2→3→0 gira para siempre. Visitados + presupuesto, siempre juntos.
- **Confiar en el orden del store.** `iter_nodes` no es estable entre runs: sin ids ordenados no hay vista determinista ni informe reproducible (la lección que ya pagaste en caps. 34/38).
- **Leer el ratio 521× como speedup anunciado.** En el regular fue 349× de ratio y ~2× de delta. Los intermedios explican el RIESGO; el speedup lo decide dónde caen esos bytes (¿cache-friendly?, ¿skew?) — mide siempre.

## 39.13 Pin de batalla

> *«Los planes clásicos atan TABLAS y cada unión escribe su resultado; los worst-case optimal atan VARIABLES y no escriben nada intermedio. Y sin equivalencia testeada contra la fuerza bruta, cualquier velocidad es un rumor.»*

## 39.14 Si solo lees 30 segundos

`ExpandOp` es un index nested-loop join; su pecado no es la lentitud sino los INTERMEDIOS invisibles: contar triángulos con dos joins binarios materializa Σ_b in(b)·out(b) caminos — K₈: 392 tuplas para 56 respuestas; rueda con hub de 512 hojas: **266.752 tuplas para 512 triángulos (521×)**; estrella: 1.056 tuplas para CERO respuestas. La cota AGM ⌊m^ρ\*⌋ (ρ\* = 3/2, Loomis-Whitney 1949; Atserias-Grohe-Marx FOCS 2008/SICOMP 2013) acota el peor resultado; NPRR (PODS 2012) dio el primer algoritmo que nunca la supera; leapfrog triejoin (Veldhuizen, ICDT 2014, Test-of-Time 2024) lo hace con tries sobre listas ordenadas, seek = galope exponencial + `binary_search` (O(log n) garantizado) y **frontera común**: liga variables a→b→c y jamás escribe N₁ — en el hub-skew, 12.832 pasos contra 92.681 de cota, y **72 µs contra 2,14 ms del binario (×29)**. Pero en el grafo regular solo gana ~2×: worst-case optimal es garantía del peor caso, no promesa. La factorización (Olteanu-Závodný ICDT 2012/TODS 2015) comprime también el RESULTADO del join: K₈ 83 vs 168 celdas, y agregar por multiplicidades ni siquiera expande tuplas (382 ns vs 570 ns). Consultas cíclicas sobre GRAFOS: cierre transitivo por punto fijo con visitados y presupuesto — termina, y equivale al BFS testeado. Equivalencia triple (binario == WCOJ == fuerza bruta) exigida por test: sin ella, todo lo anterior es magia. Gancho: ya sabes QUÉ calcular y CÓMO leerlo rápido EN UNA máquina; ¿y cuando el grafo no cabe en una?

## 39.15 Una historia pequeña

Octubre de 2024. El comité de ICDT anuncia su Test-of-Time Award: premia papers cuya influencia ha resistido una década. El galardonado describe un algoritmo que, para cuando se publicó, ya llevaba años facturando en producción comercial — el propio survey de Ngo, Ré y Rudra lo subrayó con asombro en su momento: «remarkably, this algorithm was already implemented in a commercial database system before its optimality guarantees were discovered». Piensa en lo raro del arco: primero la ingeniería que FUNCIONA sin saber por qué; luego la teoría que explica por qué (hasta un factor log); y solo al final, el premio que certifica que las dos mitades eran la misma historia. No es el orden que enseñan en la universidad —teoría primero, práctica después—, pero es el orden en que a menudo ocurre en sistemas reales. La lección operativa para ti: cuando algo funciona sospechosamente bien en producción, la ausencia de garantías no es neutralidad — es deuda. Este capítulo la liquidó al revés: primero los contadores, luego la cota, y la velocidad al final, cuando ya sabíamos qué estábamos midiendo.

## Ejercicios resueltos

**1. ¿Por qué el ratio intermedios/resultado de la estrella es INFINITO, y qué cifra finita se fija el test en su lugar?** Porque el resultado es 0 triángulos: dividir 1.056 intermedios entre 0 no produce número — la explosión es TOTAL (fabricaste tuplas para una respuesta vacía). La cifra finita comparable exige otro denominador: la ENTRADA. 1.056/64 = 16,5 = (k+1)/2 intermedios por arista con k = 32 hojas. Derivación: el hub aporta in=k, out=k → k² intermedios; cada hoja aporta in=out=1 → k más; total k²+k sobre 2k aristas = (k+1)/2. Verificación: `hub_concentrador_explota_los_intermedios`.

**2. Cuenta a mano las 83 celdas físicas de la factorización de K₈ y demuestra que las multiplicidades no doble-cuentan.** Prefijos a: los valores 0..=5 (un vértice necesita dos vecinos mayores para abrir triángulo) → 6 celdas. Slots (a,b): pares con b > a y hueco para algún c > b → 21 celdas. Hojas: 56 triángulos → 56 celdas. Total 6+21+56 = 83 contra 3×56 = 168 planas: ahorro 50,6 %. Las multiplicidades por prefijo a son C(7−a,2): [21,15,10,6,3,1]; su suma es 56, la suma de `multiplicidad_b` también es 56, y las hojas son 56 — TRES contadores independientes, un mismo total: el invariante anti-doble-recuento. Verificación: `factorizacion_triangulos_filas_logicas_vs_celdas`.

**3. Retrieval sin pistas: recita la escalera y el panel dual.** Cierra el libro. Escalera: expand-como-join → joins binarios → explosión → muchos-a-muchos → cota AGM → leapfrog → recursivas → factorización. Panel dual: arriba join-oriented (atan TABLAS, cada unión materializa su N₁), abajo variable-oriented (atan VARIABLES, frontera común, nada escrito). Si olvidaste que el peldaño 5 va ANTES del 6 —la cota justifica POR QUÉ existe el algoritmo—, o pusiste la factorización antes de la explosión (sin problema no hay cura), relee §39.3: el orden ES el argumento.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 20+39).** Desarrolla el reto esencial del §39.10: caminos de 2 saltos con frontera común contra `ExpandOp` sobre el mini dataset, totales y pasos anotados ANTES de correr. Verificación: `cargo test -p vol2-liradb --lib cap39` sigue verde con TU test dentro; fallará si tu seek acepta el pivote como propio vecino (prueba a pedir `desde = pivote+1` mal y mira cómo un auto-bucle se cuela).

**Intermedio (predecir; 21+34+39).** Predicción por escrito de los intermedios del plan binario sobre el dataset de referencia: grados típicos del generador, Σ estimado, margen de error asumido. Luego `intermedios_plan_binario` sobre `AdyacenciasOrdenadas::desde_store` y contraste. Criterio: si acertaste el orden de magnitud pero no el factor, identifica el término ignorado (dedup de paralelas, varianza de grado) — el informe del cap. 34 sobre distribución de grados te ayuda.

**Experto (crear y medir).** El camino del §39.10: 4-ciclo con ρ\* calculada a mano, implementación sobre `AdyacenciasOrdenadas`, equivalencia contra fuerza bruta y comparación contra la cota deducida. Restricciones: std puro, cero cambios en caps. anteriores, suite ALL_GREEN con TU test dentro, y si cronometras: criterion, hardware declarado, fuera del pipeline de verificación. Criterio de éxito: tu informe distingue cota teórica, trabajo contado y tiempo medido — tres números, tres significados.

## Para profundizar

- **Todd L. Veldhuizen, «Leapfrog Triejoin: A Simple Worst-Case Optimal Join Algorithm» (ICDT 2014)** — la fuente primaria del leapfrog; el seek, la frontera común y el orden de variables tal como los simplificamos aquí. **ICDT Test-of-Time Award 2024** (databasetheory.org/node/150): «implemented before its optimality guarantees were discovered».
- **Atserias, Grohe y Marx, «Size Bounds and Query Plans for Relational Joins» (FOCS 2008; versión revista SICOMP 42(4), 2013)** — la cota AGM m^(ρ\*) y la cubrimiento fraccional de aristas.
- **Ngo, Porat, Ré y Rudra, «Worst-case Optimal Join Algorithms» (PODS 2012, pp. 37-48)** — el primer algoritmo que nunca supera la cota; el Generic Join multi-way que aquí declaramos fuera de alcance.
- **Ngo, Ré y Rudra, «Skew Strikes Back: New Developments in the Theory of Join Algorithms» (SIGMOD Record 42(4), 2014)** — el survey que unifica NPRR y leapfrog; la fuente de la ley Ω(N²) vs N^1,5 del triángulo.
- **Loomis y Whitney, «An Inequality Related to the Isoperimetric Inequality» (Bull. AMS 55, 1949)** — la raíz geométrica de la cota, setenta años antes de los joins.
- **Olteanu y Závodný, «Factorised Representations of Query Results» (ICDT 2012; «Size Bounds and Factorised Representations of Query Results», ACM TODS 40(1), mar-2015)** — la teoría de f-trees y multiplicidades detrás del gesto 8.
- **Jin, Feng, Chen, Liu y Salihoğlu, «KÙZU Graph Database Management System» (CIDR 2023, CC-BY 4.0)** — WCOJ + processor factorizado para grafos; atribución y relato histórico según ADR-001 (Waterloo → CIDR 2023 → Apple oct-2025 → repo archivado → fork LadybugDB).
- **ISO/IEC 39075:2024 (GQL)** y **SQL:1999 (WITH RECURSIVE)** — el contexto estándar de las consultas recursivas que aquí quedaron en API.
- Dentro del libro: cap. 14 (CSR ordenado — el trie sale gratis), cap. 20 (`ExpandOp`, el operador que hoy formalizamos), cap. 21 (catálogo y sus estadísticas que faltan), caps. 22-23/26 (BFS y presupuesto), cap. 25 (triángulos en comunidades), cap. 34 (metodología de benches y dataset), cap. 38 (`SubgrafoAcotado`, `ExpansionFactorizada` — las semillas que este capítulo germina).

## Mini-diálogo: en guardia nocturna

> — Son la una de la mañana. La detección de comunidades del cliente —triángulos por usuario— acaba de tumbar el servicio: OUT OF MEMORY. Anoche pasaba; hoy no pasa. ¿Subimos la instancia?
>
> — Antes el árbol de spans del cap. 35: ¿en qué operador muere?
>
> — En el segundo Expand. Y hay algo raro: el resultado son unos miles de filas, no millones.
>
> — Claro: el resultado no es lo que crece. Es lo que NO ves — el intermedio del join. Con hubs en el grafo, tu plan binario escribe cientos de miles de tuplas (a,b,c) para devolver unas pocas. Subir la instancia es comprar un cubo más grande para una fuga.
>
> — Entonces…
>
> — Cambia el PLAN, no el cubo. Leapfrog: liga variables con frontera común y jamás escribe el intermedio — medimos ×29 en el escenario con skew. Y si el consumidor solo agrega, enumera factorizado: multiplicidades, no tuplas — ni siquiera expande el resultado.
>
> — ¿Y si mañana el grafo crece ×100?
>
> — Entonces revisa los CONTADORES otra vez: intermedios previstos, cota AGM, pasos. Con números delante decides entre plan, particionado… o hierro. A ciegas, solo comprarías cubos.

---

*(Próximo capítulo: 40 — distribución. Ya sabes QUÉ calcular —worst-case optimal joins— y CÓMO leerlo rápido —columnas y factorización— EN UNA máquina. ¿Y cuando el grafo no cabe en una? Particionado, cortes de aristas, replicación de fronteras y consenso. Preguntas abiertas que este capítulo deja a propósito: ¿cómo se PARALELIZA un leapfrog (morsel-driven, rayon futuro)?, ¿qué estadísticas exigiría el orden DINÁMICO de variables?, ¿y si las columnas del cap. 38 vivieran en disco?)*
