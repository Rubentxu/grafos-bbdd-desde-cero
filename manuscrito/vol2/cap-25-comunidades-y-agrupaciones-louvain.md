# Capítulo 25 — Comunidades y agrupaciones (Louvain simplificado)

> *«El ojo ve doce tribus. La modularidad, con el grafo entero en la cabeza, prefiere seis. Ninguna de las dos miente: preguntan cosas distintas.»*

## 25.0 La anécdota de la esquina

En 2008, cuatro físicos e informáticos belgas — Vincent D. Blondel, Jean-Loup Guillaume, Renaud Lambiotte y Etienne Lefebvre — publicaron en una revista de física estadística (*Journal of Statistical Mechanics: Theory and Experiment*, artículo P10008) un método para «desplegar» la estructura de comunidades de redes enormes. El título del paper, «Fast unfolding of communities in large networks», no menciona ninguna universidad. Pero el método nació en la Université catholique de Louvain (Louvain-la-Neuve, Bélgica), y la comunidad científica, que necesita nombres cortos, lo bautizó con el de la institución: **el método de Louvain**. Hoy es uno de los papers más citados de la ciencia de redes — decenas de miles de citas — y hasta la página del propio Blondel en UCLouvain lo presenta ya como «the Louvain method».

La velocidad era la afirmación audaz del paper: «supera a todos los métodos conocidos de detección de comunidades en tiempo de cómputo» (es la claim literal del abstract), y lo demostraron con la red de llamadas de una operadora belga — 2,6 millones de clientes — y con un grafo web de 118 millones de nodos y más de mil millones de enlaces. Para dimensionar lo que eso significaba: el método clásico que lo precedió, el **divisivo de Girvan-Newman**, detectaba comunidades cortando repetidamente la arista de mayor *betweenness* — la que calculaste con tanto esfuerzo en el capítulo 24 — y tenía que recalcularla entera después de cada corte: los propios autores (Newman-Girvan 2004) reportan un coste de O(m²·n), es decir O(n³) en grafos dispersos. Con 2,6 millones de nodos, ni hablemos. Curiosamente, la **modularidad** — la métrica que guía todo este capítulo — nació precisamente en esa tradición: Newman y Girvan la inventaron en 2004 como criterio para decidir *cuándo parar de cortar aristas*. Louvain invirtió el papel de las dos piezas: en vez de cortar aristas y usar Q para parar, **optimiza Q directamente** con movimientos locales baratos y una jerarquía que se contrae sola. Ese giro — y no una fórmula nueva — es lo que lo hizo rápido.

## 25.1 Objetivo

Al terminar este capítulo sabrás **particionar un grafo en comunidades de forma verificable**: no «grupos que se ven bonitos», sino grupos con un número — la modularidad Q — que cualquiera puede recalcular sobre la partición y comparar con alternativas. Construirás las cuatro piezas del módulo `cap25_comunidades.rs`:

1. `componentes_conexas` — el suelo del concepto: alcanzabilidad pura.
2. `label_propagation` — la primera heurística densa, y sus límites documentados.
3. `modularidad` — la métrica Q de Newman-Girvan, con resolución γ, verificable sobre cualquier partición dada.
4. `louvain` — el greedy jerárquico: fase local con ΔQ exacto + agregación en supernodos, nivel a nivel, dejando un dendrograma.

El hito: detectar los dos grupos de una red social pequeña (dos K4 unidos por un puente) y *demostrar con números* que es la mejor partición.

## 25.2 Problema

El capítulo 24 te dio el mapa del «quién es importante». Esta es la pregunta complementaria: **¿quiénes forman grupo?** Piensa en la red social del `demo_graph`: Ana, Bea y Carlos se conocen entre sí; cada uno vive en una ciudad; Dani se conoce a sí mismo (ese `KNOWS` de Dani hacia sí mismo que ya te encontraste en el capítulo 24, dándole cuota de PageRank). ¿Cuántas comunidades hay? ¿Quién está con quién?

La pregunta es resbaladiza por dos motivos:

- **«Comunidad» no puede ser «grupo denso» a secas.** Un triángulo dentro de un grafo completísimo no es comunidad: ahí TODO es denso. Lo que hace comunidad es ser denso **respecto a lo que cabría esperar al azar**. Necesitas un modelo de azar de referencia — un *modelo nulo* — o el concepto no significa nada.
- **Cualquier partición es una respuesta.** «Todos en una» es una partición. «Cada uno solo» es otra. Sin una función que puntúe particiones, no puedes ni compararlas ni decir que tu algoritmo hizo un buen trabajo. «Encontré 7 comunidades» no es un resultado: 7 salió de ti, no del grafo.

Y hay una restricción de negocio, heredada del capítulo 24: esto corre **dentro de una base de datos**. Así que los análisis tienen que ser **reproducibles** (dos ejecuciones, mismo resultado: nada de aleatoriedad) y **ruidosos al fallar** (un peso negativo no se «tolera»: se señala con la arista culpable, igual que hacía Dijkstra en el capítulo 22).

## 25.3 Modelo mental

Piensa en el grafo como un **mapa de tribus**. Una tribu es una región del mapa donde la gente se relaciona sobre todo entre sí, y poco con los de fuera. La pregunta que mide la calidad del mapa:

> **De todo el peso de las aristas del grafo, ¿qué fracción cae dentro de las tribus… y cuánto MÁS de lo que esperaría el azar si repartiáramos esas mismas aristas al azar (conservando cuántas relaciones tiene cada nodo)?**

Ese «azar que conserva los grados» es el **modelo nulo de configuración**: la probabilidad de que el azar ponga una arista entre i y j es proporcional a k_i·k_j. La **modularidad** (Newman-Girvan 2004) es exactamente ese exceso sobre el azar, sumado por comunidad:

```text
Q_γ = Σ_c [ Σ_in(c)/2m − γ·(Σ_tot(c)/2m)² ]
        \_______/   \____________________/
        fracción de   lo que el azar
        peso INTERNO  esperaría para c
```

- `Σ_in(c)`: peso interno de la comunidad c, contando cada arista por sus dos extremos.
- `Σ_tot(c)`: suma de los grados (ponderados) de los miembros de c — incluye las aristas que salen fuera.
- `2m`: el peso total del grafo contando cada arista en ambas direcciones (la convención de siempre: Σ_tot de todos = 2m).
- `γ` (gamma): la **resolución** de Reichardt-Bornholdt 2006 — cuánto le exigimos al azar. γ=1 es el clásico.

Lecturas inmediatas: la partición trivial (todo junto) da Q = 1 − γ = 0 exacto; Q > 0 es «mejor que el azar»; Q < 0, peor. Y como es una fracción de 2m, **escalar todos los pesos por una constante no cambia Q** — lo testea el módulo.

El diagrama que ordena el capítulo es el anillo de tríos — el grafo canónico del límite de resolución de Fortunato-Barthélemy (2007), que veremos en 25.6:

```text
      trío 0        trío 1        trío 2         ...      trío 11
    0 ─── 1  ~~  3 ─── 4  ~~   6 ─── 7   ...         ~~ 33 ─── 34
     \  /          \  /          \  /                      \  /
      2 ────────────5 ────────────8 ───── ... ───────────── 35
      (K3)          (K3)          (K3)                     (K3)
        eslabón del anillo: cada trío cuelga del siguiente por UNA arista
```

Doce tríos idénticos, un eslabón cada uno. Tu ojo ve doce tribus. Veremos que Q, con γ=1, prefiere seis pares — y por qué eso no es un bug sino una lección profunda sobre lo que significa «comunidad global».

## 25.4 Primera solución

La solución más ingenua que funciona un poco: **una comunidad es una componente conexa**. El módulo la implementa con el BFS de toda la vida sobre la vista simétrica del store (pesos constantes 1, porque la alcanzabilidad no pesa):

```rust
pub fn componentes_conexas(store: &dyn GraphStore) -> Result<ComponentesResult, ComunidadesError>
```

O(V+E), números por menor miembro (el barrido ascendente numera cada componente con el nodo más pequeño que contiene — la renumeración canónica sale gratis). Es el **suelo del concepto**: toda comunidad vive dentro de una componente (nadie se agrupa con quien no alcanza), pero ninguna noción de densidad decide todavía.

Segundo paso, ya con ambición: **label propagation** (LPA, Raghavan-Albert-Kumara 2007). Cada nodo empieza con su etiqueta; en cada pasada (nodos por id ascendente, actualización asíncrona) adopta la etiqueta más votada entre sus vecinos, con votos ponderados por el peso de la arista. Casi lineal, sin función objetivo. El módulo lo hace determinista: empates → se conserva la propia etiqueta si empata con la máxima, y si no gana la menor.

## 25.5 Sus límites

**Las componentes mueren con un solo puente.** Dos K3 unidos por una arista: una componente. La red social real suele ser UNA componente gigante — la pregunta interesante (tribus dentro de la isla) queda intacta.

**LPA no optimiza nada verificable — y encima gotea.** No hay Q que comprobar. Peor: con pesos uniformes, los empates de la primera pasada pueden GOTEAR por los puentes. El test `lpa_separa_dos_trios_y_empates_deterministas` lo documenta con un experimento espejo que vale la pena despacio:

```text
dos K3 con puente 2-3:  LPA → 1 comunidad   Louvain → 2 comunidades
dos K3 con puente 1-4:  LPA → 2 comunidades  Louvain → 2 comunidades
```

El MISMO grafo espejado (solo cambia qué nodos toca el puente), el MISMO algoritmo… y resultados opuestos. Mecánica del goteo en el primer caso: al barrer por id ascendente, el trío izquierdo (nodos 0-2) se forma primero; cuando le llega el turno al nodo 3 (el del puente derecho), sus votos son {etiqueta 1: 1, etiqueta 4: 1, etiqueta 5: 1} — tres etiquetas empatadas, y la de él propio (3) tiene cero votos de vecinos. Adopta la 1. Arrastró la etiqueta izquierda a través del puente antes de que su propio trío tuviera tiempo de formarse. En el caso espejo (puente 1-4), cuando el barrido llega al nodo 4, el nodo 3 ya barrió y le dio un voto a la etiqueta 4 — la política de «conservar la propia si empata con la máxima» lo salva. El resultado de LPA depende del orden y de la numeración; es determinista aquí (dos ejecuciones, idéntico), pero frágil. La receta práctica — test incluido — es romper los empates con pesos (puente de peso 0.5 → LPA ya separa).

Ésta es exactamente la motivación de Louvain: una heurística sin métrica no se puede ni verificar ni defender. Hace falta la métrica primero, el algoritmo después.

## 25.6 Solución evolucionada

### Paso 1: Q como juez — `modularidad(partición dada)`

La decisión de diseño más importante del capítulo: la modularidad es una **función verificable sobre CUALQUIER partición dada**, no un subproducto interno del algoritmo. `modularidad(store, particion, weight, gamma)` la calcula sobre lo que le pases (nodos ausentes → singletons; ids de grupo u64 arbitrarios; γ validado: 0, negativo, NaN e ∞ rechazados). Con eso, Q es a la vez la métrica guía de Louvain y el oráculo de sus tests.

Cuenta conmigo los dos tríos con puente (pesos 1; 7 aristas → 2m = 14; grados [2,3,2,2,3,2] — el puente sube a 3 los nodos 1 y 4):

```text
partición perfecta {0,1,2} {3,4,5}:  cada trío In=6 (3 aristas × 2 extremos), K=7
    Q = 2·[6/14 − (7/14)²] = 12/14 − 1/2 = 5/14 ≈ 0.357
partición trivial (todo junto):      In=2m → Q = 1 − 1 = 0 exacto
singletons:  Q = −Σ(k_i/2m)² = −(4+9+4+4+9+4)/196 = −17/98
```

Los tres números están testeados contra la aritmética exacta (`EPS = 1e-12`). Fíjate en el porqué de usar Q y no «número de comunidades»: 2 comunidades (Q=5/14), 6 (Q=−17/98) y 1 (Q=0) son particiones de distinto tamaño — el conteo no las ordena; Q sí. Y para demostrar que Q está bien calculada, un ejercicio de honestidad numérica: en el MISMO grafo, ¿conviene fundir los dos tríos? ΔQ = 2/14 − 2·(7/14)² = −5/14 < 0: no. Ojo a esa fórmula, porque reaparece.

### Paso 2: el greedy local con ΔQ EXACTO — y la agregación

Louvain alterna dos fases por nivel:

1. **Fase local**: desde singletons, cada nodo (por id ascendente — el Louvain «de literatura» baraja; una BD no puede) evalúa moverse a cada comunidad *vecina* con el **ΔQ exacto** — solo cambian los términos de las DOS comunidades implicadas:

```text
ΔQ = q(In_c + 2·k_{i,c} + 2·s_i, K_c + k_i)      ← destino ganado
   + q(In_d − 2·k_{i,d} − 2·s_i, K_d − k_i)      ← origen perdido
   − q(In_c, K_c) − q(In_d, K_d)                 ← como estaban
donde q(in, k) = in/2m − γ·(k/2m)²,  k_{i,c} = peso de i hacia c,  s_i = self-loop de i
```

Se mueve solo si ΔQ > 0 estricto; empates → comunidad de menor id (`total_cmp`, la misma disciplina anti-f64 de los caps. 22 y 24). Pasada tras pasada hasta que nadie se mueva. ¿Por qué ΔQ exacto y no «recalcular Q entera»? **Trazabilidad aritmética**: cada movimiento se puede re-verificar a mano con cuatro términos, la complejidad por evaluación es O(grado) en vez de O(V+E), y la monotonía de Q queda garantizada por construcción (test: Q nunca baja entre niveles).

2. **Agregación**: cada comunidad se contrae en un **supernodo**; las aristas internas se vuelven self-loops del supernodo, las externas suman pesos entre supernodos. La fase (1) se repite sobre el grafo contraído. La gracia: movimientos que a escala de nodo estaban bloqueados (dos tríos que individualmente no ganan nada moviéndose) se vuelven visibles cuando cada trío YA es un supernodo. En el anillo de 12 tríos, el nivel 0 encuentra los 12 tríos (Q = 2/3) y el nivel 1 los funde en 6 pares (Q = 17/24): dos niveles, dendrograma real.

### Paso 3: el TEST ESTRELLA — el límite de resolución

¿Por qué el nivel 1 del anillo FUNDE tríos si tu ojo ve doce tribus? La cuenta, con la fórmula del paso 1 (cada trío: In=6, K=8; 2m=96):

```text
fundir dos tríos adyacentes (el eslabón, peso 1, se vuelve interno):
  ΔQ = 2/96 − γ·2·(8/96)²  =  1/48 − γ/72
γ=1:  ΔQ = +1/144  → FUNDE (Q pares 17/24  >  Q tríos 2/3)
γ=2:  ΔQ = −1/144  → NO funde (Q tríos 7/12  >  Q pares 13/24)
```

Ahí está, en dos fracciones, el **límite de resolución de Fortunato-Barthélemy (2007)**: la modularidad global pregunta al azar de TODO el grafo, y en un grafo grande el azar espera tan poquito cruce entre dos tríos que fusionarlos «ahorra» penalización — aunque el eslabón sea una única arista floja. La MISMA estructura local que en el grafo de 2m=14 NO se funde (ΔQ = −5/14), en el anillo de 2m=96 SÍ (ΔQ = +1/144): el tamaño del mundo cambia la respuesta. La escala crítica de Fortunato-Barthélemy: comunidades más chicas que ~√(2m) aristas se vuelven invisibles para γ=1. El remedio es γ: γ>1 exige comunidades más densas y chicas (el umbral de este anillo está en γ* = 3/2, donde ΔQ = 0). El panorama completo, todo él verificable con `modularidad()`:

| Partición del anillo | Q con γ=1 | Q con γ=2 | Ganadora |
|---|---|---|---|
| 12 tríos (la del ojo) | 2/3 ≈ 0.667 | **7/12 ≈ 0.583** | γ=2 |
| 6 pares fundidos | **17/24 ≈ 0.708** | 13/24 ≈ 0.542 | γ=1 |

El test `louvain_limite_de_resolucion_gamma` demuestra ambas columnas con particiones ground-truth y Q analíticos — y por eso es el test estrella: no valida el código, valida la MÉTRICA, enseñándote cuándo tu herramienta va a mentirte.

## 25.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap25_comunidades.rs`. Las piezas con su porqué.

### `GrafoPonderado`: la proyección hermana del cap 24 — pero que SUMA

```rust
struct GrafoPonderado {
    nodes: Vec<NodeId>,            // orden ascendente → determinismo
    self_loop: Vec<f64>,           // s_i SIN el ×2 (la convención se materializa en k)
    vecinos: Vec<Vec<(usize, f64)>>, // vecinos distintos, pesos ACUMULADOS, ordenados
    k: Vec<f64>,                   // k_i = 2·s_i + Σ_j w_ij  (self-loop contado doble)
    dos_m: f64,                    // 2m = Σ_i k_i
    edges_scanned: u64,
}
```

¿Por qué un grafo propio y no reusar la `Proyeccion` del cap 24? Porque son **familias distintas con convenciones distintas**. El cap 24 contaba *vecinos distintos* — su `GraphDirection::Both` hace unión como CONJUNTO (deduplica aristas paralelas), que es correcto para grado y centralidades. La modularidad es de **multigrafo**: tres mensajes entre Ana y Bea son un lazo triple, y deben SUMAR (el término 2·k_{i,c} del ΔQ lo exige). El test `louvain_multigrafo_paralelas_equivalen_a_peso_sumado` lo clava: 3 aristas paralelas de peso 1 ≡ una de peso 3, mismo resultado exacto. Además Louvain **reconstruye** el grafo en cada agregación: `contraer()` devuelve otro `GrafoPonderado`. Lo heredado del cap 24 es el patrón (ids ordenados, índice denso que compacta los huecos de `delete_node`, materializar una vez); lo heredado del cap 22 es la semántica estricta de pesos (`WeightSource` + `edge_weight`, con `From<PathError>` para envolver `MissingWeight`/`InvalidWeight`/`NonFiniteWeight`).

### El self-loop ×2 — la convención que sostiene la jerarquía

Un self-loop de peso s entra como A_ii = 2s: **cuenta doble** en k_i y en 2m. No es capricho: es la convención estándar de la modularidad (así k_i = Σ_j A_ij lo cuenta «una vez por dirección»), y sobre todo es lo que hace que la **contracción conserve Q**: una arista interna de peso w aportaba 2w a Σ_in (sus dos extremos) y w+w a Σ_tot; contraída, es un self-loop s=w que aporta A_cc = 2w a ambas cosas — idéntico. Test de invariante nivel a nivel: la Q de cada nivel, calculada en el grafo contraído, coincide con `modularidad()` sobre el original. En una red social, un self-loop es una relación consigo mismo (el `KNOWS` de Dani): refuerza su comunidad propia sin unirlo a nadie — en `demo_graph`, Dani acaba solo, con Q = 5/18 empatada entre DOS óptimos distintos (el test documenta el empate en vez de fingir unicidad).

### `louvain`: el bucle de niveles y su cota de terminación

```rust
let tope_niveles = n + 1;   // cota DEMOSTRABLE, no esperanza
loop {
    let (com, movimientos, pasadas) = g.fase_local(gamma, max_pasadas, &mut stats);
    if movimientos == 0 { break; }             // nada que agregar
    // ... grabar NivelLouvain (asignación de nodos ORIGINALES, Q, stats)
    mapeo = mapeo.iter().map(|&t| densa[t]).collect(); // composición
    g = g.contraer(&densa);                     // 2m se conserva
    if g.len() < 2 || niveles.len() >= tope_niveles { break; }
}
```

La cota: cada nivel arranca de singletons, así que su PRIMER movimiento vacía una comunidad ⇒ el nivel siguiente tiene estrictamente menos nodos ⇒ niveles con movimientos ≤ V (en la práctica O(log V): el grafo se contrae geométricamente). `max_pasadas` limita cada fase local — un seguro contra el ruido de f64 en ΔQ ≈ 0 (un ΔQ «positivo» de 1e-18 movería nodos para siempre). Y ojo al hallazgo testeado: `max_pasadas=1` NO rebaja la calidad final — lo que una pasada deja a medias en el nivel 0, la agregación lo repara en el nivel 1.

### La jerarquía lleva nodos ORIGINALES

`NivelLouvain` guarda, por nivel, la asignación de los nodos **originales** (la composición de particiones), su Q y su número de comunidades; `particion_en(nivel)` construye el dendrograma a demanda. ¿Por qué original y no supernodos? Porque esto es un producto: el cap 51 (Vol. III, GraphRAG) consumirá los niveles para generar resúmenes del grafo a varias granularidades, y necesita responder «¿quién está con quién?» en el vocabulario del usuario. El anidamiento queda garantizado por construcción (cada comunidad del nivel ℓ+1 es unión exacta de comunidades del ℓ — testeado en la dirección correcta: fina ⇒ gruesa).

## 25.8 Prueba de fuego

El hito del capítulo: **dos K4 unidos por un puente** (13 aristas, 2m = 26). Louvain DEBE separarlos, y la Q debe cuadrar a mano: cada K4 tiene In = 12 (6 aristas × 2) y K = 13 (los nodos del puente tienen grado 4):

```rust
let r = louvain(&s, &WeightSource::Constant(1.0), 1.0, 30).unwrap();
assert_eq!(r.num_comunidades(), 2);
assert_ne!(r.comunidad(0), r.comunidad(5));
assert!((r.modularidad - 11.0 / 26.0).abs() < 1e-12);   // 2·[12/26 − (13/26)²] = 11/26
```

Los tests que cierran el círculo (todos ejecutables con `cargo test -p vol2-liradb cap25`):

- **Oráculo**: Q del resultado == `modularidad()` de la misma partición, en CADA caso.
- **Determinismo**: dos ejecuciones idénticas, jerarquía incluida, incluso con el orden de inserción de las aristas INVERTIDO (`louvain_determinismo_y_orden_de_insercion`).
- **Ground truth sintético**: 3 anillos con cuerdas y 2 eslabones (30 nodos): recupera los 3 grupos EXACTO y con Q ≥ Q_truth — mientras `componentes_conexas` dice que todo es UNO (alcanabilidad ≠ densidad, dos nociones de grupo).
- **Los pesos reestructuran**: dos tríos con puente de peso 100 → NO se funde todo (Q es invariante a escala): el puente deja de ser explicable por el azar y la mejor partición ROMPE los tríos alrededor de él — {0,2},{1,4},{3,5} con Q = 100/2809, mejor que la trivial (0). «Más peso» no es «más fusión».
- **Fallos ruidosos**: peso negativo → `NegativeWeight { edge, weight }` con la arista señalada; prop ausente → el `MissingWeight` del cap 22 envuelto.
- **Coste medible, no declamado**: `ComunidadesStats` cuenta `edges_scanned` (96 en el anillo de 12 tríos: 48 pares × 2 aristas dirigidas), `pasadas`, `movimientos` y `niveles` — la sección «coste computacional» del guion, verificada por `louvain_stats_coherentes`.

Si te saltaras este capítulo, el síntoma te delataría: agruparías por componentes («¡una comunidad gigante!») o confiarías en LPA sin pesos, y cuando alguien pregunte «¿por qué esas comunidades y no otras?» no tendrías número ni criterio — y el cap 51 se quedaría sin dendrograma.

## 25.9 Qué hemos sacrificado

1. **El barajado del Louvain original**: la aleatoriedad explora mejor (escapa de algunos óptimos locales); la pagamos con la reproducibilidad, que en una BD no se negocia. Queda documentado el precio: varios óptimos con Q igual se resuelven por orden (`demo_graph`: dos óptimos con Q = 5/18).
2. **La re-localización de Leiden**: Louvain clásico puede dejar comunidades MAL CONECTADAS internamente (dos piezas unidas «por detrás» del supernodo). Leiden (Traag-Waltman-van Eck 2019) lo repara con una fase de refinamiento; aquí lo declaramos limitación (sección 25.10).
3. **La proyección con pesos compartida**: la Parte V merece una proyección ponderada única sobre el CSR del cap 14 — es exactamente el capítulo 26. Aquí el `GrafoPonderado` la materializa en memoria y la deuda queda declarada.
4. **Exactitud global**: greedy local = óptimo local garantizado-coherente, no óptimo global. El contrato del resultado es «una partición coherente con su Q», no «la mejor partición posible».

## 25.10 Cómo lo hace una BBDD real

**Neo4j Graph Data Science** expone exactamente estas piezas: `gds.louvain` (Tier 1) y `gds.leiden` (Tier 2), con `relationshipWeightProperty` (nuestro `WeightSource::Property`), `tolerance` (nuestro corte de ΔQ), `maxLevels` (nuestro tope de niveles) y — dato que valida el capítulo — `includeSelfLoops`, un flag que decide si los self-loops cuentan en el cálculo: la convención A_ii = 2s es una decisión de semántica real, la que discutimos en 25.7. Los modos `stats/mutate/stream/write` escriben el `communityId` como propiedad de nodo — la partición como dato consultable, igual que nuestra `Particion::grupo(id)`.

¿Por qué existe Leiden si Louvain es tan bueno? Por la crítica demoledora de Traag, Waltman y van Eck («From Louvain to Leiden: guaranteeing well-connected communities», *Scientific Reports* 9:5233, 2019): el greedy de Louvain evalúa si cada NODO está bien conectado a su comunidad, nunca si la COMUNIDAD está bien conectada consigo misma — y pueden salir (ellos lo fotografían) comunidades internamente desconectadas: dos trozos pegados solo a través de otras comunidades, invisibles tras la agregación. Leiden añade una fase de refinamiento que parte comunidades mal conectadas y garantiza (con su procedimiento de agregación) conectividad interna. La lección de ingeniería: un algoritmo que optimiza una métrica no garantiza propiedades que la métrica no mide.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: calcula a mano la Q de la partición {0,1,2},{3,4,5},{4}… no: de {0,1,2,4},{3},{5} sobre los dos tríos con puente. ¿Supera al −17/98 de los singletons? ¿Y al 0 de la trivial?
- *Intermedio*: ¿por qué el umbral γ* del anillo de 12 tríos es exactamente 3/2? Derívalo de ΔQ = 1/48 − γ/72 y comprueba que γ* no depende de k (número de tríos). ¿Qué SÍ depende de k?
- *Experto*: construye el anillo de k tríos con k parámetro y encuentra empíricamente el k donde γ=1 empieza a fundir; explica por qué el k crítico crece con… ¿o no crece? (Fortunato-Barthélemy: la escala invisible es ~√(2m).)

## 25.11 Lo que te llevas

- **Comunidad = densidad contra el azar** (modelo nulo de configuración), no componente (alcanabilidad) ni «grupo denso» a secas.
- **Q_γ es la métrica guía Y el oráculo**: verificable sobre cualquier partición dada; compara particiones de distinto tamaño; un «número de comunidades» no compara nada.
- **Greedy local con ΔQ exacto + agregación**: O(grado) por evaluación, trazable a mano, monótono en Q; el corte de aristas de Girvan-Newman (O(m²·n)) es lo que Louvain reemplazó.
- **Convenciones que sostienen todo**: self-loop A_ii = 2s (hace que la contracción conserve Q), simetrización SUMANDO (multigrafo), pesos estrictos del cap 22 y negativos rechazados eager.
- **Determinismo total**: orden por id, empates por `total_cmp` → menor, renumeración por menor miembro; cota ≤ V niveles + `max_pasadas` anti-ruido-f64.
- **El límite de resolución**: Q global no ve comunidades más chicas que ~√(2m); γ lo arregla (anillo de tríos: γ=1 → 6 pares con Q=17/24; γ=2 → 12 tríos con Q=7/12).
- **La jerarquía es el producto**: nodos ORIGINALES por nivel, anidamiento garantizado — el dendrograma que el cap 51 consumirá.

## 25.12 Ojo, cuidado con…

- **Comparar Q entre grafos o entre γ distintos**: Q es «exceso sobre el azar» DENTRO de un grafo y una γ. 0.42 aquí no es «mejor» que 0.36 allá.
- **Contar el self-loop simple**: rompe k_i, 2m y la conservación de Q al agregar. Se detecta porque la Q de un nivel deja de cuadrar con `modularidad()` sobre el original.
- **Deduplicar aristas paralelas** (la convención del cap 24): aquí el multigrafo ACUMULA; deduplicar mide otro grafo.
- **Confundir «partición coherente con su Q» con «partición óptima»**: el greedy para cuando nadie mejora; el mundo puede tener dos óptimos empatados (`demo_graph`, Q = 5/18 ×2) o uno mejor que el greedy nunca vio.

## 25.13 Pin de batalla

> *«Si tu detector de comunidades no puede decirte QUÉ está optimizando con un número que puedas recalcular, no es un detector: es una opinión compilada.»*

## 25.14 Si solo lees 30 segundos

Una comunidad es una región del grafo más densa de lo que el azar esperaría. La **modularidad Q_γ** pone número a una partición completa — «fracción de peso interno menos la esperada por el azar» — y por eso guía Y juzga. **Louvain** la optimiza en dos fases alternadas: mover nodos al vecino con mayor ΔQ exacto (greedy local, determinista), y contraer comunidades en supernodos (agregación) nivel a nivel, conservando Q. Sus límites declarados: óptimos locales (greedy) y el **límite de resolución** (Q global funde comunidades chicas: el anillo de 12 tríos lo demuestra; γ lo corrige). La jerarquía resultante, con nodos originales, es el insumo del GraphRAG del cap 51.

## 25.15 Una historia pequeña

Cuando añadimos el límite de resolución al módulo, el primer impulso fue «arreglarlo»: si Q prefiere seis pares donde el ojo ve doce tribus, que la libería corrija el resultado. Nos detuvimos a medio commit. El anillo no estaba roto — estaba ENSEÑANDO: la misma estructura local funde o no funde según el tamaño del grafo entero, porque el azar de referencia es global. Ocultar eso habría convertido el test estrella en una caja negra que «funciona». Lo dejamos, con γ al aire y los valores analíticos al lado: 17/24 contra 2/3 con γ=1, 7/12 contra 13/24 con γ=2, y el umbral en 3/2 derivable a mano. El día que un usuario pregunte «¿por qué mi detección une estos dos equipos que claramente son distintos?», la respuesta estará esperando en un test, no en un foro.

## Ejercicios resueltos

**1. ¿Por qué la partición trivial da exactamente 0 y no «casi 0»?**

Con todo en una comunidad: Σ_in = 2m (todas las aristas son internas) y Σ_tot = 2m. El término único es 2m/2m − γ·(2m/2m)² = 1 − γ. Con γ=1, exactamente 0 — sin redondeos, sin suerte: la estructura de la fórmula lo garantiza para CUALQUIER grafo. Es el test más barato y más duro de romper de la función `modularidad` (está en el doctest).

**2. En `demo_graph` (KNOWS: 0→1→2→0 y self-loop de Dani; LIVES_IN: 0→4, 1→5; k = [3,3,2,2,1,1], 2m = 12), ¿por qué Dani acaba solo SIEMPRE?**

El self-loop de Dani entra como A_33 = 2. Su única «relación» es consigo mismo: no tiene comunidad vecina a la que moverse (la fase local evalúa solo comunidades VECINAS). Queda de semilla singleton en cualquier nivel. Y las dos particiones restantes empatan EXACTO en Q = 5/18: {0,2,4},{1,5},{3} y {0,1,2,4,5},{3} — el ΔQ que las separa es 0, y el greedy determinista toma una. El test afianza lo que NO depende del camino (Dani solo, Q = 5/18), no la elección.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el capítulo: escribe Q_γ de memoria y calcula a mano la Q de la partición perfecta de los dos tríos con puente. Verifica contra el doctest de `modularidad` y el test `modularidad_particion_trivial_es_cero_y_perfecta_analitica`. Pistas: (1) ¿cuánto vale Σ_in si cada arista interna se cuenta por sus DOS extremos?; (2) ¿el puente entra en Σ_tot del trío?; (3) ¿por qué la trivial es 0 exacto?

**Intermedio (spacing con el cap 24).** Explica, nodo a nodo, la primera pasada de LPA sobre dos K3 con puente 2-3 (gotea a 1 comunidad) y sobre el espejo con puente 1-4 (separa en 2), y qué haría Louvain en ambos. Verifica con `lpa_separa_dos_trios_y_empates_deterministas`. Pistas: (1) ¿en qué orden se barren los nodos?; (2) ¿de dónde salen los votos de la etiqueta PROPIA de un nodo?; (3) ¿qué ΔQ tendría el movimiento que gotea?

**Experto (interleaving, gancho al cap 51).** Generaliza el anillo a k tríos con k parámetro: deriva el ΔQ de fundir dos tríos adyacentes en función de k y γ, predice para qué k γ=1 empieza a fundir, y verifícalo empíricamente. Luego, con γ=2 y `particion_en(0)`/`particion_en(1)` sobre el anillo de 12, lista las comunidades finas y gruesas — el dendrograma que el cap 51 resumirá. Pistas: (1) ¿qué términos de Q dependen de k?; (2) ¿2m crece con k… y el término de fusión también?; (3) ¿por qué el anidamiento solo se afirma en la dirección fina ⇒ gruesa?

## Para profundizar

- **Blondel, Guillaume, Lambiotte, Lefebvre, «Fast unfolding of communities in large networks», J. Stat. Mech. (2008) P10008** — el paper de Louvain; el abstract con la claim de velocidad y los 2,6 M de clientes está en [IOPscience](https://iopscience.iop.org/article/10.1088/1742-5468/2008/10/P10008) y [arXiv:0803.0476](https://arxiv.org/abs/0803.0476).
- **Fortunato, «Community Detection in Graphs», Physics Reports 486 (2010)** — el mapa completo: divisive, LPA, modularidad, límites.
- **Fortunato-Barthélemy, «Resolution limit in community detection», PNAS 104(1):36-41 (2007)** — [el paper del anillo de cliques](https://www.pnas.org/doi/10.1073/pnas.0605965104).
- **Traag, Waltman, van Eck, «From Louvain to Leiden», Sci. Rep. 9:5233 (2019)** — [la crítica de las comunidades mal conectadas y el algoritmo que las garantiza](https://www.nature.com/articles/s41598-019-41695-z).
- **Neo4j GDS: [Louvain](https://neo4j.com/docs/graph-data-science/current/algorithms/louvain/) y [Leiden](https://neo4j.com/docs/graph-data-science/current/algorithms/leiden/)** — `relationshipWeightProperty`, `includeSelfLoops`, `tolerance`, `maxLevels`: este capítulo en producción.

## Mini-diálogo: en guardia nocturna

> — O sea que Louvain es «mueve nodos si Q sube, contrae, repite». ¿Y por qué tanto bombo?
>
> — Porque cada pieza es verificable. El ΔQ es exacto y lo recalculas a mano; la Q de cada nivel coincide en el grafo contraído y en el original; dos ejecuciones dan lo mismo hasta con las aristas insertadas al revés. Puedes enseñarle el algoritmo a un auditor línea a línea.
>
> — Pero el anillo de tríos… tu métrica prefiere seis pares donde yo veo doce tribus.
>
> — Exacto: esa es la mejor parte. La herramienta no te falla en silencio — te dice CUÁNDO va a mentirte, con la aritmética a la vista. γ es el zoom, y el umbral 3/2 del anillo lo derivas tú, no lo decreta la librería.
>
> — ¿Y si necesito que las comunidades estén bien conectadas por dentro?
>
> — Entonces ya sabes qué paper leer, qué algoritmo pedirle a tu base de datos, y por qué existe. Eso es saber más que «usar Louvain».

---

*(Próximo capítulo: 26 — Ejecutar algoritmos sin agotar la memoria. Aquí `GrafoPonderado` materializó el grafo entero en RAM; veremos la proyección con pesos sobre el CSR del capítulo 14, el streaming y los frontiers — y saldremos de la deuda que este capítulo dejó declarada.)*
