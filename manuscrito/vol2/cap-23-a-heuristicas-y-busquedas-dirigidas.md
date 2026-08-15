# Capítulo 23 — A*, heurísticas y búsquedas dirigidas

> *«Dijkstra sabe perfectamente cuánto ha caminado. No tiene ni idea de hacia dónde va. Esa diferencia es este capítulo.»*

## 23.0 La anécdota de la esquina

En 1966, en el Stanford Research Institute de Menlo Park, un robot llamado **Shakey** («temblón», por cómo le vibraba el chasis al arrancar) intentaba hacer algo que hoy suena trivial y entonces no lo era: moverse por una habitación llena de cajas sin chocar. Shakey no llevaba cerebro dentro: una cámara de TV y un telémetro láser enviaban las imágenes por radio a un DEC PDP-10 y un PDP-15 que ocupaban una habitación entera al lado, y el ordenador le devolvía por radio las órdenes. Todo ese viaje de ida y vuelta costaba tanto que Shakey se movía a paso de tortuga meditabunda (puedes verlo en las películas del SRI que conserva el Computer History Museum; el propio Peter Hart lo cuenta en el artículo de Wired de 2013).

Para que un robot que apenas se movía no perdiera además la vida planeando rutas, tres investigadores del proyecto — **Peter Hart, Nils Nilsson y Bertram Raphael** — escribieron en 1968 el paper «A Formal Basis for the Heuristic Determination of Minimum Cost Paths» (*IEEE Transactions on Systems Science and Cybernetics*, SSC-4(2)): el algoritmo **A\***. La idea no era explorar mejor que Dijkstra, sino explorar *menos*: si el robot sabe HACIA DÓNDE queda el destino, no necesita mirar en todas direcciones. Cuenta Hart que en el laboratorio iban nombrando versiones del algoritmo con letras —A, A1, A2— y al ganador le pusieron un asterisco, la estrella, para señalar que ése era el bueno. En 1972 los mismos tres autores publicaron una corrección en el SIGART Newsletter que bautizaba dos palabras que usaremos todo el capítulo: **admisible** y **consistente**. Casi sesenta años después, A\* sigue siendo el algoritmo que lleva tu GPS, los NPC de los videojuegos y el planificador de rutas de las bases de datos de grafos.

En el Vol.I ya programaste A\* sobre un grafo en memoria (cap. 9) y lo viste en robótica (cap. 29). Este capítulo lo hace correr **sobre el grafo del store** de LiraDB, y ahí aparece una pregunta que el Vol.I no necesitaba responder: ¿de dónde sale la heurística cuando los datos viven en una base de datos? Respuesta: de las **props de los nodos** — y de un trait.

## 23.1 Objetivo

Al terminar este capítulo sabrás **por qué Dijkstra explora «en círculo» y cómo dirigir la búsqueda hacia el destino sin perder la garantía de optimalidad**, y habrás usado (no reescrito: *usado*) cuatro piezas sobre la maquinaria del capítulo 22:

1. El trait `Heuristic` — el contrato «¿cuánto queda para llegar?».
2. `ZeroHeuristic` — h≡0, que convierte A\* en Dijkstra *exactamente* (y por eso es un test).
3. `EuclideanHeuristic` — la distancia recta leyendo las props `x`/`y` de los nodos.
4. `check_consistency` — el diagnóstico O(E) que delata la arista que rompe tu heurística.

Y una métrica nueva, `PathStats::expanded`, para **medir** el ahorro en vez de creerlo.

## 23.2 Problema

Piensa en el grafo-trampa del test `euclidea_mismo_coste_con_menos_expansiones`: diez ciudades en línea sobre el eje X (tramos de 1 km) y una **trampa**: tres nodos colgados al norte, a 100, 200 y 300 km del destino, unidos por aristas baratísimas de 0.5. Quieres ir de la ciudad 0 a la ciudad 9.

Dijkstra —tu `dijkstra_path` del cap. 22, con su finalización anticipada— expande **los 13 nodos**: la cadena y la trampa entera. No es un bug: Dijkstra ordena el heap por el coste acumulado `g`, y esos 0.5 son *objetivamente* más baratos que cualquier tramo de la cadena. Para quien sólo mira `g`, la trampa es irresistible. El radar no tiene brújula.

El problema no es de eficacia sino de *información*: el algoritmo sabe dónde has estado (`g`) y no sabe hacia dónde vas. La red de ciudades del hito cuenta la misma historia en grande: para ir de Madrid a Barcelona, Dijkstra asienta **las 7 ciudades** del mapa —incluidas Sevilla y Bilbao, que están en dirección *contraria*— antes de que Barcelona salga del heap. En un grafo de millones de nodos, ese círculo que crece desde el origen es la diferencia entre milisegundos y minutos.

## 23.3 Modelo mental

Dos imágenes, un contraste:

```
      DIJKSTRA (radar)                     A* (GPS)
      ordena el heap por g                 ordena el heap por f = g + h
      ondas circulares                     frente sesgado hacia el destino

  destino ●                                destino ●
          |  ____                                 /__
          | /    \                               /    ↗ ¡voy hacia allí!
      ____|/      \                          ___/↗
     /    |        \                        /  ↗ ← h(n) dice "queda poco"
    | ____ |         \                      | ↗
    |/    \|          ● origen              ● origen
      ondas: "lo más barato primero"        g (andado) + h (que queda)
```

- **Dijkstra es un radar omnidireccional**: emite desde el origen y asienta lo más cercano en coste, sin importar la dirección. Óptimo, sí; y ciego al destino.
- **A\* es un GPS**: cada nodo lleva una brújula `h(n)` que responde «¿cuánto queda hasta el destino?», y el heap ordena por la suma `f(n) = g(n) + h(n)`. Avanzar (`g`) y acercarse (`h`) puntúan juntos. La trampa del norte tiene `h` gigantesca —300 km rectos— y el GPS ni la mira.

El momento ¡ajá! es éste: **h no cambia el grafo, no cambia el algoritmo, no cambia ni una línea del bucle de relajación. Cambia el ORDEN del heap — y el orden de pops era todo lo que Dijkstra era.** Si metes `h ≡ 0`, la clave de ordenación degenera en la de Dijkstra y A\* *es* Dijkstra, pop a pop. Todo el capítulo es una consecuencia de esa frase.

## 23.4 Primera solución

La solución ingenua ya la tienes escrita: `dijkstra_path(store, origen, destino, &peso)`. Es óptima, está testeada y termina en cuanto el destino sale del heap. En la trampa devuelve el camino correcto (coste 9.0) tras expandir 13 nodos.

El intento naïve de «dirigir» la búsqueda también lo escribiría cualquiera: si la brújula es buena, ordena el heap **sólo por h** y ve directo al destino (greedy best-first). Y funciona… hasta que no funciona: una arista cara «que acerca» le gana a un desvío barato, porque `g` no cuenta para nada en la ordenación. El greedy es la sobre-estimación llevada al extremo: confía *tanto* en la corazonada que ignora lo andado.

## 23.5 Sus límites

Las dos soluciones ingenuas fallan por extremos opuestos y complementarios:

1. **Dijkstra**: toda la información sobre lo andado (`g`), ninguna sobre lo que queda. Paga el grafo entero en el peor caso: 13 expansiones donde hacían falta 10; 7 donde hacían falta 3.
2. **Greedy por h**: toda la información sobre lo que queda, ninguna sobre lo andado. Rápido y **sin garantía de optimalidad** — el mismo defecto que tendrá cualquier heurística que sobre-estima, como veremos en la sección 23.8.

La ley del todo-o-nada: o exploras en círculo, o pierdes el óptimo. Falta la idea que mezcle ambas informaciones *sumándolas* — y que sepa qué garantía conserva esa suma. Esa idea es A\*, y en una base de datos necesita resolver antes una pregunta de diseño: **¿de dónde sale h?**

## 23.6 Solución evolucionada

### El trait `Heuristic`: la heurística la aporta el CALLER

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap23_a_estrella.rs`. El corazón del diseño es un contrato de tres líneas:

```rust
pub trait Heuristic {
    fn estimate(&self, store: &dyn GraphStore, node: NodeId) -> Result<f64, PathError>;
}
```

¿Por qué un trait y no una closure `Fn(NodeId) -> f64`, que sería menos ceremonia? Tres razones, y ninguna es estética:

1. **Las heurísticas interesantes tienen estado y están ligadas a un destino.** La euclídea recuerda las coordenadas del destino y los nombres de las props (`x`, `y`); una de landmarks (sección 23.11) llevaría tablas de distancias precalculadas. Un struct con campos lo expresa sin esfuerzo; una closure tendría que fabricar ese estado con `Rc<RefCell<…>>`.
2. **Una closure no podría leer el store.** `estimate` recibe `&dyn GraphStore` porque la euclídea necesita las props *del nodo que se está estimando* — datos que no están en la closure sino en el grafo. La alternativa sería capturar el store prestado, y entonces esa closure queda casada con ese store: no podrías pasarle otro. Es la PRIMERA VEZ que un algoritmo de la Parte V lee datos del **NODO** (todo el cap. 22 leyó aristas): por eso la firma es lo que es.
3. **El contrato se valida en un sitio.** `a_star` revisa cada estimación que usa (finita, no negativa); si cada caller se fabricara su closure, ese contrato se repartiría por el código.

Y ojo: el trait no cierra la puerta al caso ad-hoc. Los tests definen `Fija(Vec<f64>)` —una tabla de valores por nodo— en tres líneas, y con ella construyen los casos patológicos del capítulo (inconsistente, sobre-estimada, negativa). El patrón es el mismo que `WeightSource` en el cap. 22: el algoritmo fija la maquinaria; el caller aporta la política.

### `ZeroHeuristic`: h≡0, o por qué un test no es una esperanza

```rust
pub struct ZeroHeuristic;   // h(n) = 0.0 para todo n
```

La heurística nula es admisible (0 ≤ coste real, siempre), consistente (0 ≤ w + 0, siempre)… y completamente inútil para dirigir nada. Su valor es doble, y los dos son de ingeniería, no de teoría:

- **Es la línea base**: contra ella se mide cuánto ahorra una heurística real (`expanded` con h≡0 = trabajo de Dijkstra).
- **Es un test de equivalencia exacta.** Con h≡0, la clave del heap `Reverse<(Cost(f), Cost(g), NodeId)>` degenera en `(Cost(g), NodeId)` — *exactamente* la de Dijkstra. No «parecida»: la misma. Por eso el test `heuristica_cero_es_dijkstra_exactamente` exige MISMO camino, MISMO coste y **MISMO orden de pops** (`popped`, `expanded` y `relax_updates` idénticos). Si tu A\* suma mal la f, desempata distinto o relaja en otro orden, el test se pone rojo. Un test que compara sólo el coste aceptaría una implementación que «acierta» explorando distinto; un test de orden de pops no perdona nada.

Ese es el sentido de la frase del modelo mental: A\* contiene a Dijkstra como caso degenerado, y ese caso degenerado es un **freno de regresión**, no una curiosidad.

### `EuclideanHeuristic`: coordenadas como props de nodo

La heurística canónica del capítulo: la distancia en línea recta de cada nodo al destino, leyendo las coordenadas de las props del nodo:

```rust
let h = EuclideanHeuristic::new(&store, destino, "x", "y")?;
// estimate(n) = hypot(x_n − x_dest, y_n − y_dest)
```

Tres decisiones escondidas aquí:

1. **El destino se liga al construir** (`new` lee y valida SUS coordenadas eager — son 2 props, fallar antes de empezar es más barato que fallar en medio de la búsqueda). Por eso `estimate` no recibe el destino: una heurística apunta a UN destino.
2. **Semántica estricta, la misma que `edge_weight` en el cap. 22**: prop ausente o `Null` → `PathError::MissingCoordinate { node, prop }`; tipo no numérico o Float no finito → `InvalidCoordinate`; un `Int` se promociona (coordenadas `Int(3)`/`Int(4)` dan la hipotenusa 5.0, el triángulo 3-4-5 del test). Un grafo schemaless delata sus huecos *cuando se pisan*, no antes.
3. **`hypot` y no `((dx*dx)+(dy*dy)).sqrt()`**: `f64::hypot` calcula la raíz sin desbordes intermedios y garantiza resultado ≥ 0 y finito con entradas finitas — que ya validamos. h nunca llega al heap con un NaN (ver la validación de `h_of` abajo).

¿Por qué es admisible? Porque **una carretera nunca es más corta que la línea recta**: si los pesos son distancias en las mismas unidades que las coordenadas, cada arista satisface `w(u,v) ≥ dist_recta(u,v)`, y por la desigualdad triangular ninguna ruta puede ser más corta que la recta al destino. Admisible *por construcción*, no por fe. Guarda esa condición de unidades: es la bomba de la sección 23.8.

### El algoritmo: la misma maquinaria, otra clave

`a_star(store, origen, destino, weight, heuristic)` reutiliza del cap. 22 *tal cual* la sanidad eager (`validate_edge_weights`, extraída a `pub(crate)` para compartirla — refactor puro), `WeightSource`/`edge_weight`, `Path`, `PathError`, `PathStats` y la reconstrucción por predecesores. Lo nuevo son cuatro detalles:

```rust
let mut heap: BinaryHeap<Reverse<(Cost, Cost, NodeId)>> = BinaryHeap::new();
// (f, g, nodo): f = g + h prioriza; g y nodo desempatan (determinismo)

while let Some(Reverse((_, Cost(g_u), u))) = heap.pop() {
    if g_u > g[u] { continue; }        // entrada obsoleta: su g ya fue superado
    stats.expanded += 1;                // pop VIVO: esto es lo que se mide
    if u == dest { break; }            // válido con h admisible
    for eid in store.out_edges(u) { /* relajar como en cap. 22 */ }
}
```

1. **La clave es `(f, g, nodo)`, no un `f64`**: el heap exige `Ord`, y `f64` no lo implementa precisamente por los NaN. El newtype `Cost` del cap. 22 devuelve; los desempates por `g` y luego por id dan determinismo.
2. **Entradas obsoletas**: cuando un nodo mejora su `g`, su entrada vieja *sigue* en el heap (los `BinaryHeap` no borran). Al salir, su `g` queda comparado contra el vigente (`g_u > g[u]`) y se descarta sin expandir. Comparación exacta: ambos valores salen de las mismas sumas.
3. **Re-apertura, no `settled`**: aquí está la diferencia estructural con Dijkstra. Dijkstra puede marcar nodos como definitivos porque h≡0 es *trivialmente consistente* y un nodo expandido jamás mejora. Con una h inconsistente, un nodo ya expandido PUEDE mejorar su `g` después… y hay que re-expandirlo o pierdes caminos. Por eso `stats.expanded` puede superar el número de nodos: es el precio medido, no escondido, de tolerar heurísticas imperfectas.
4. **Caché de estimaciones** (`h_cache`, NaN = «sin estimar»): la heurística se consulta a lo sumo UNA vez por nodo — la euclídea si no reelería dos props por cada inserción en el heap.

Y el criterio de parada, enunciado con precisión: cuando el destino sale del heap como entrada *viva*, su `g` es óptimo **si h es admisible** — cualquier camino aún por descubrir pasa por un nodo del heap cuyo `f` es cota inferior de su coste, y todos eran ≥ `f(destino)`. Con h≡0 esto es literalmente el argumento de Dijkstra del cap. 22.

### La validación honesta: qué se revisa, qué se documenta, qué se diagnostica

Ésta es la decisión de diseño más importante del capítulo, y es una decisión sobre *costes*. El cap. 22 validó eager los pesos negativos porque era O(E) y saltársela producía respuestas mentira. Aquí la escala manda:

| Propiedad | Coste de verificarla | Decisión |
|---|---|---|
| Pesos no negativos | O(E), herencia cap. 22 | Eager, misma función |
| h finita y ≥ 0 | 1 comparación por estimación (cacheada) | Se revisa SIEMPRE (`NonFiniteHeuristic`, `NegativeHeuristic`) |
| **Admisibilidad** (h ≤ coste real) | ¡Resolver Dijkstra hacia el destino! | **No verificable** — se documenta, el riesgo se DEMUESTRA en tests |
| **Consistencia** (h(u) ≤ w(u,v)+h(v)) | O(E) + ≤2V estimaciones, LOCAL | Utilidad `check_consistency`, diagnóstico opcional |

- **h finita y ≥ 0 sí es negociable que se revise**: un NaN rompería el orden total de `Cost` y haría *panic dentro del heap* — el peor lugar del mundo para depurar. Un h negativo casi siempre es un bug del caller y rompe el criterio de parada. La función `h_of` es la aduana (y de paso, la caché):

```rust
fn h_of(heuristic: &dyn Heuristic, store: &dyn GraphStore,
        cache: &mut [f64], node: NodeId) -> Result<f64, PathError> {
    if cache[node].is_nan() {                    // NaN = "todavía no estimada"
        let v = heuristic.estimate(store, node)?;
        if !v.is_finite() { return Err(PathError::NonFiniteHeuristic { node, value: v }); }
        if v < 0.0      { return Err(PathError::NegativeHeuristic  { node, value: v }); }
        cache[node] = v;                          // ≤ 1 consulta por nodo
    }
    Ok(cache[node])
}
```

- **La admisibilidad NO se puede verificar**: saber si `h(n) ≤ coste real de n al destino` exige conocer ese coste real… que es exactamente el problema que estás resolviendo. Verificarla costaría más que la propia búsqueda. Así que el contrato se **documenta** y el riesgo se **demuestra** (tests: h sobre-estimada y unidades mezcladas ⇒ camino subóptimo *sin ningún error*).
- **La consistencia sí es local**, y por eso existe `check_consistency`: una pasada por las aristas comprobando `h(u) ≤ w(u,v) + h(v)`. Devuelve `InconsistentHeuristic { edge, h_from, bound }` señalando la PRIMERA arista culpable. Y fíjate en la asimetría deliberada: es una utilidad de diagnóstico, **no un requisito** — A\* no la exige, porque rechazar una heurística inconsistente sería rechazar respuestas correctas (con h admisible-inconsistente, la re-apertura conserva el óptimo; el test lo mide: **5 expansiones para 4 nodos**, porque A se expande dos veces —con g=4 y luego con g=3.6—, y el coste 4.6 sigue siendo el óptimo que da Dijkstra).

## 23.7 El hito: rutas sobre una red de ciudades

El test `hito_del_brief_rutas_sobre_una_red_de_ciudades` monta la red del capítulo con coordenadas estilizadas en km (Madrid en el origen, Zaragoza a (190,130), Barcelona a (380,180)…) y carreteras siempre algo más largas que la recta. La pregunta: Madrid→Barcelona.

- El directo cuesta 460. La ruta por Zaragoza, 240 + 200 = **440**: ésa es la óptima, y la encuentra cualquiera de los dos algoritmos.
- Dijkstra expande **7 nodos**: asienta todas las ciudades — Valladolid, Bilbao, Valencia, Sevilla incluidas — porque ordena por `g` y el círculo no distingue.
- A\* con la euclídea expande **3**: Madrid, Zaragoza, Barcelona. La recta desde Madrid a Barcelona (≈ 420 km) ya adelanta que Valladolid y Sevilla están *lejos* del destino: sus `f` nacen altísimos y no salen del heap. El resto del mapa ni se mira.

```
              Bilbao                    Barcelona ● ← destino (3ª y última)
               |                          ▲
           Valladolid          Zaragoza ●──┘ ← 2ª: la recta la señala
               |                          ▲
             Sevilla          Valencia    │
                                │         │
                             Madrid ●─────┘ ← 1ª
        (Dijkstra: las 7 · A*: 3 · mismo camino, mismo coste 440)
```

Ese «7 vs 3» es `PathStats::expanded`, la métrica que este capítulo añadió al struct del cap. 22 (y que `dijkstra_impl` también incrementa). ¿Qué compra? Hace **visible** el ahorro de la heurística: sin ella, «A\* es mejor» es una fe; con ella, es un número que puedes `assert_eq!`. Y además delata la re-apertura (`expanded` > nodos tocados = hay inconsistencia). En el grafo-trampa de la sección 23.2, la misma métrica dice 13 vs 10: Dijkstra cae en la trampa barata que se aleja; A\* ve que esos nodos están lejísimos del destino y no los toca.

## 23.8 El bug didáctico estrella: kilómetros contra minutos

Éste es el error que quiero que te lleves tatuado. Test `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad`: coordenadas en km, pesos en **minutos**. Autopista directa de 200 km con tráfico (200 min) contra un desvío rápido por el norte (98 + 67 = 165 min, el óptimo real).

La euclídea sigue siendo «válida en forma»: finita, ≥ 0, geometría impecable. Pero la recta de 139 km ya no acota *minutos* — sobre-estima. Y entonces, sin error, sin warning, sin pánico:

- Dijkstra (que no usa h) devuelve el óptimo: **165** por el desvío.
- A\* devuelve **200** por la directa, camino `[0, 2]` perfectamente válido — encadena, las aristas existen, el coste suma — sólo que subóptimo.

`h` hundió al desvío con «139 km que no son minutos» y premió la directa. Es el mismo síntoma que la sobre-estimación deliberada del otro test (h(1)=10 ⇒ A\* responde 3.0 donde el óptimo es 2.0): **A\* no miente sobre el camino que devuelve — miente sobre su optimalidad, y en silencio.** Por eso la admisibilidad no se «verifica y santifica»: se entiende o se sufre.

¿Y cómo se caza? Con el diagnóstico local: `check_consistency` señala la arista culpable, la del desvío final — `h(1) ≈ 139.28 > w + h(2) = 67 + 0`. La primera arista cuya desigualdad triangular se rompe en unidades. Un O(E) que contesta lo que la búsqueda jamás te dirá.

## 23.9 Prueba de fuego

Los tests de este capítulo no afirman «funciona»: afirman números. Ejecútalos (`cargo test -p vol2-liradb`) y léelos como una tabla de garantías:

| Test | Lo que demuestra |
|---|---|
| `heuristica_cero_es_dijkstra_exactamente` | h≡0 ⇒ MISMO camino, coste, pops, expanded, relax_updates que Dijkstra |
| `euclidea_mismo_coste_con_menos_expansiones` | Trampa: 13 vs 10 expansiones, mismo coste 9.0, y `check_consistency` ok |
| `hito_del_brief_rutas_sobre_una_red_de_ciudades` | Madrid→Barcelona: 7 vs 3, coste 440.0 por Zaragoza |
| `admisible_no_consistente_reexpande_y_sigue_optimo` | 5 expansiones para 4 nodos, óptimo 4.6 intacto, `check_consistency` señala B→A |
| `sobre_estimacion_devuelve_suboptimo_demostrando_el_riesgo` | 3.0 vs 2.0 **sin error**; el camino devuelto es válido |
| `unidades_mezcladas_km_vs_minutos_rompen_la_admisibilidad` | 200 vs 165 silencioso; `check_consistency` delata la arista |
| `coordenadas_ausentes_invalidas_o_no_finitas` | Missing/InvalidCoordinate; Int promociona (3-4-5 ⇒ 5.0) |
| `heuristica_negativa_o_no_finita_es_rechazada` | NaN ⇒ error tipado (sin él: panic en `Cost::cmp`) |

¿Qué pasaría si te saltas este capítulo? Dos síntomas detectables: tus rutas punto-a-punto escalan con todo el grafo (el círculo del radar), y el día que copies una heurística con unidades distintas a tus pesos obtendrás rutas malas **sin ningún síntoma** — ni error, ni test rojo. Sólo `check_consistency`, que no sabrás ejecutar, te lo habría dicho.

## 23.10 Qué hemos sacrificado

1. **La tabla single-source**: A\* sólo responde punto-a-punto. El sesgo hacia el destino es toda la gracia, y las distancias intermedias que va fijando NO están garantizadas (con h inconsistente, ni las de los nodos tocados). ¿Distancias desde un origen a todos? `dijkstra` del cap. 22, sin discusión.
2. **Cero re-expansiones**: tolerar h inconsistente cuesta re-abrir (5 expansiones para 4 nodos). Una implementación con `settled` sería más rápida… y mentiría sobre qué heurísticas admite.
3. **No hay `check_admissibility`**: no es una omisión, es una imposibilidad con presupuesto. La hermana local existe; la global no puede.
4. **La euclídea exige coordenadas y unidades coherentes**: grafos sin geometría (redes sociales, grafos de dependencias) no tienen recta que medir. Para ellos quedan las heurísticas de tabla — y los landmarks de la sección siguiente.

## 23.11 Cómo lo hace una BBDD real

- **Neo4j GDS** ofrece `gds.shortestPath.astar`: la heurística es la distancia entre las coordenadas **`latitude`/`longitude` que deben vivir como propiedades del nodo** — exactamente nuestra `EuclideanHeuristic` con otros nombres de props, y con la misma condición implícita de que el coste sea una distancia comparable.
- **pgRouting** expone `pgr_aStar` con parámetros `heuristic`, `factor` y `epsilon` y columnas x1/y1/x2/y2: la euclídea multiplicada por un factor que el usuario puede *subir* — es decir, sobre-estimación deliberada y configurable, cambiando óptimo garantizado por velocidad. Y su documentación/comunidad advierten del revés real: `pgr_astar` puede salir *más lento* que `pgr_dijkstra` si la heurística es mala (¡nuestro h≡0 de nuevo, pero en producción!). La moraleja de este capítulo vive en sus issues.
- **GraphHopper** (el motor de rutas detrás de muchos servidores de mapas) usa **ALT**: *A\*, Landmarks, Triangle inequality* — se eligen unos pocos nodos «hito» (landmarks), se precalculan sus distancias a todo el grafo, y `h(n) = máx |d(L,t) − d(L,n)|` sobre los hitos acota por la desigualdad triangular. Es la heurística de nuestro ejercicio experto, con pedigree: Goldberg y Harrelson la publicaron en SODA 2005 («A\* Search Meets Graph Theory»). Cuando NO hay coordenadas — o las rectas mienten, como con ferris y túneles — los landmarks son la euclídea del pobre y del rico a la vez.
- **Kùzu** hoy resuelve `SHORTEST` con recursive joins semánticos (lo veremos en el cap. 26) sin heurística geométrica: un recordatorio de que A\* es una herramienta para preguntas *métricas* punto-a-punto, no para todo patrón.

## 23.12 Lo que te llevas

- **f = g + h**: la clave del heap mezcla lo andado y lo que queda; h sólo cambia el ORDEN de los pops.
- **h≡0 ⇒ A\* ES Dijkstra**, pop a pop — y por eso es un test de regresión, no una curiosidad.
- **Admisible** (h ≤ coste real: global, NO verificable sin resolver el problema) ≠ **consistente** (local, O(E) con `check_consistency`). Admisible basta para el óptimo; consistente evita re-abrir.
- **Re-apertura** en vez de `settled`: con h inconsistente se re-expande y el óptimo se conserva — medido en `expanded`.
- **La euclídea es admisible por construcción** sólo si pesos y coordenadas están en las mismas unidades; mezclar km y minutos produce subóptimos **en silencio** (200 vs 165).
- **`expanded`** hace visible el ahorro: 13 vs 10 en la trampa, 7 vs 3 en Madrid→Barcelona.
- La heurística es un **trait ligado al destino** que lee props de NODO del store — la primera vez que la Parte V mira datos del nodo.

## 23.13 Ojo, cuidado con…

- **Confundir admisible con consistente**: la consistencia implica admisibilidad (con h(dest)=0); al revés no. Admisibilidad = óptimo; consistencia = además, sin re-expansiones.
- **Sustituir `h` por el coste real «ya calculado» de una ejecución anterior**: si el grafo cambió (¡grafo mutable!), tu h sobre-estima y el silencio vuelve.
- **Contar `popped` como trabajo**: los pops obsoletos no expanden; lo que cuesta es `expanded`. Son métricas distintas con nombres parecidos.
- **Validar TODAS las coordenadas al empezar**: el destino se valida eager (2 props); el resto se delata al pisarlo. Validar O(V) para una ruta que toca 3 nodos es pagar el grafo entero otra vez — el pecado del radar.

## 23.14 Pin de batalla

> *«Una heurística que miente hacia arriba no acelera tu búsqueda: te vende un camino peor sin decirte nada. La admisibilidad no se verifica — se entiende.»*

## 23.15 Si solo lees 30 segundos

Dijkstra explora en círculo porque su heap sólo conoce `g`, lo andado. A\* ordena por `f = g + h`, donde `h(n)` es lo que el CALLER sabe («¿cuánto queda?») — en LiraDB, un trait `Heuristic` ligado al destino que lee props de nodo (`EuclideanHeuristic` con `x`/`y`). Con h **admisible** (nunca sobre-estima) el primer pop vivo del destino es óptimo; con h **consistente** ni siquiera re-abre nodos; con h sobre-estimadora —o con unidades mezcladas— el resultado es subóptimo **sin error**, y sólo `check_consistency` (O(E), local) te señala la arista culpable. El ahorro se mide en `PathStats::expanded`: 13→10, 7→3.

## 23.16 Una historia pequeña

Cuando Ana montó la red de ciudades para el hito, reutilizó las coordenadas de un mapa viejo — en kilómetros — pero los pesos los sacó de un servicio de tráfico que devolvía **minutos**. «A\* con euclídea, esto vuela», dijo, y voló: Madrid→Barcelona en 3 expansiones. Sólo que por la autopista directa. Dijkstra, con el mismo grafo, encontraba la ruta por el desvío rápido, 35 minutos antes. Ningún test fallaba — el camino de Ana encadenaba, sumaba, existía. Tardó una tarde en caer en la cuenta de que la heurística hablaba kilómetros y el grafo escuchaba minutos, y cinco minutos en confirmarlo: `check_consistency` señaló la arista del desvío y su `139.28 > 67` fue la confesión. Desde entonces, en LiraDB, toda heurística nueva pasa por el túnel de consistencia antes de estrenarse. No porque sea obligatorio. Porque es silencioso.

## Ejercicios resueltos

**1. ¿Por qué el test de h≡0 compara también el ORDEN de pops y no sólo el coste?**

Porque la clave del heap con h≡0, `Reverse<(Cost(g+0), Cost(g), NodeId)>`, se ordena exactamente como la de Dijkstra, `(Cost(g), NodeId)`: mismas prioridades, mismos desempates. Comparar sólo camino y coste dejaría pasar una implementación que suma mal `f` o desempata por otra cosa pero «acierta» el resultado en ese grafo. Exigir `popped`, `expanded` y `relax_updates` idénticos convierte la equivalencia en exacta: cualquier desviación del orden es un rojo. Es el test de equivalencia más fuerte que se puede pedir sin leer el heap paso a paso.

**2. Con h admisible pero inconsistente, ¿por qué re-abrir nodos conserva el óptimo?**

Porque la garantía de parada no depende de que un nodo expandido sea definitivo: depende de que, al salir el destino como entrada viva, todo camino por descubrir tenga una cota inferior `f ≥ f(destino)`. Esa cota sólo exige admisibilidad (h nunca promete de menos). La inconsistencia hace que un nodo expandido pueda mejorar su `g` después; si lo marcáramos `settled`, perderíamos esa mejora y con ella el camino óptimo que la atraviesa. Re-expandir paga el precio (5 expansiones para 4 nodos en el test) pero no toca la garantía. Marcar `settled` es una optimización válida sólo bajo consistencia — y por eso es una decisión, no un default.

## Ejercicios propuestos

**Esencial (retrieval).** Sin mirar el código ni el capítulo: (a) di qué degenera exactamente en la clave del heap cuando h≡0 y qué TRES contadores debe igualar A\* para que la equivalencia con Dijkstra sea exacta; (b) di cuál de las dos propiedades —admisible, consistente— puede verificar LiraDB en O(E), y qué tendría que resolver para verificar la otra. Verifica tus respuestas contra `heuristica_cero_es_dijkstra_exactamente` y la doc de `check_consistency`.

**Intermedio (analizar).** El túnel del doctest: A en (0,0), B en (3,4) — recta de 5 —, una carretera de 6 y un túnel de 4. Antes de ejecutar nada, razona: ¿es consistente la euclídea hacia B en ese grafo? ¿Es admisible? Si el camino óptimo de A a B pasa por el túnel, ¿qué puede devolver A\* — camino válido, coste correcto, o silencio? Monta el store, compruébalo con `a_star` + `dijkstra_path` + `check_consistency`, y explica la diferencia entre la propiedad global rota y la local rota.

**Experto (crear, con spacing del cap. 22).** Implementa `LandmarkHeuristic`: elige un hito L (¿Sevilla o Zaragoza? discute cuál acota mejor hacia Barcelona), precalcula con el `dijkstra` single-source del cap. 22 la tabla `d(L,·)`, y define `h(n) = |d(L,dest) − d(L,n)|` (la desigualdad triangular la hace admisible; el destino fíjalo en construcción, como la euclídea). Corre Madrid→Barcelona y compara `expanded` contra `ZeroHeuristic` (7) y la euclídea (3). Ejecuta `check_consistency` e interpreta lo que diga. Criterio de éxito: camino 440.0 idéntico, `expanded` medido, y una frase defendiendo tu elección de hito.

## Para profundizar

- **Hart, Nilsson y Raphael, «A Formal Basis for the Heuristic Determination of Minimum Cost Paths» (IEEE Trans. SSC, 1968)** — el paper de A\*, nacido del robot Shakey. Corto, legible, y las definiciones son las que usamos aquí.
- **Hart, Nilsson y Raphael, corrección en SIGART Newsletter 37 (1972)** — donde se aclara la distinción admisible/consistente que este capítulo explota.
- **Dechter y Pearl, «Generalized Best-First Search Strategies and the Optimality of A\*» (JACM, 1985)** — por qué A\* con h consistente es óptimamente eficiente entre quienes usan la misma h.
- **Goldberg y Harrelson, «Computing the Shortest Path: A\* Search Meets Graph Theory» (SODA, 2005)** — ALT: landmarks + desigualdad triangular, la evolución natural de este capítulo.
- **Docs de Neo4j GDS (A\* con latitud/longitud), de pgRouting (`pgr_aStar` y sus parámetros `heuristic`/`factor`/`epsilon`) y de GraphHopper (`landmarks.md`)** — las tres decisiones de producción sobre la misma teoría.

## Mini-diálogo: frente al mapa

> — Entonces A\* es Dijkstra más una corazonada.
>
> — Menos una mitad del mapa. La corazonada —h— sólo reordena el heap; el bucle, la relajación, el camino, todo es del capítulo 22. Por eso h≡0 ES Dijkstra, pop a pop: no es una casualidad, es el caso degenerado del diseño.
>
> — ¿Y si mi corazonada exagera?
>
> — Entonces llega antes y por el camino equivocado, y no te enteras: el camino que te devuelve existe, encadena, suma. Se llama subóptimo silencioso, y es el motivo de que exista `check_consistency`. Dijkstra no se equivoca así porque no promete nada que no haya andado. La heurística es la primera pieza de LiraDB que habla del futuro — y hablar del futuro tiene reglas.
>
> — ¿Y la regla?
>
> — Nunca prometas menos de lo que queda. En km, si el grafo mide en km.

---

*(Próximo capítulo: 24 — Centralidad y PageRank. A\* necesitaba UN destino para saber hacia dónde ir; ahora la pregunta se invierte: cuando no hay destino sino importancia repartida, ¿quién manda a quién? La Parte V deja de buscar caminos y empieza a puntuar nodos.)*
