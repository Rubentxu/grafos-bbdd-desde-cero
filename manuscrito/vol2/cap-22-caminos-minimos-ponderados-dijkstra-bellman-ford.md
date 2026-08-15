# Capítulo 22 — Caminos mínimos ponderados (Dijkstra y Bellman-Ford)

> *«Un camino puede tener más saltos y costar menos. El día que tus aristas pesan, "cerca" deja de significar "pocos saltos" y empieza a significar "barato".»*

## 22.0 La anécdota de la esquina

Según contaba el propio Dijkstra, en 1956 hizo «dos cosas importantes»: terminó su carrera y asistió a la inauguración oficial del ARMAC, el computador del Mathematisch Centrum de Ámsterdam. Para la inauguración necesitaba una demostración que los no-informáticos entendieran —y su respuesta también—, así que preparó un programa que hallara la ruta más corta entre dos ciudades de Holanda sobre un mapa reducido de 64 ciudades (seis bits bastaban para identificar una). La pregunta que se hizo era de una sencillez insultante: ¿cuál es el camino más corto de Rotterdam a Groningen?

Y aquí la parte que ha contado mil veces quien firma el algoritmo: «Una mañana estaba de compras en Ámsterdam con mi joven prometida, y cansados, nos sentamos en la terraza de un café a tomar un café, y yo estaba pensando si sería capaz de hacerlo, y entonces diseñé el algoritmo del camino mínimo. Como he dicho, fue una invención de veinte minutos». Después remataba: «de hecho, se publicó en el 59, tres años tarde». Y un detalle que es puro Dijkstra: «una de las razones de que [el paper] sea tan agradable es que lo diseñé sin lápiz ni papel. Sin lápiz ni papel estás casi obligado a evitar toda complejidad evitable».

Ese algoritmo nacido en una servilleta que nunca existió se publicó como «A Note on Two Problems in Connexion with Graphs» (Numerische Mathematik 1, 1959, págs. 269-271), dos páginas y media; en los años sesenta ya aparecía en un libro alemán de investigación operativa como «Das Dijkstra'sche Verfahren», y hoy vive en cada GPS: como le gustaba decir al entrevistador que acababa de consultar una ruta, «esta mañana ha usado usted mi algoritmo». Nosotros vamos a instalarlo donde nunca ha estado tan a gusto: sobre un grafo persistente cuyo peso es una PROPIEDAD escrita por un usuario que puede, tranquilamente, haberse equivocado. (Fuente: entrevista de Philip L. Frana a E. W. Dijkstra, historia oral del Charles Babbage Institute, 2001, publicada en Communications of the ACM 53(8), agosto de 2010.)

## 22.1 Objetivo

Al terminar este capítulo sabrás **por qué "el camino con menos saltos" deja de ser la respuesta correcta en cuanto las aristas pesan**, y habrás ejecutado los dos algoritmos clásicos de caminos mínimos SOBRE el grafo persistente de LiraDB — con los pesos leídos de las propiedades de las aristas, no de una matriz preparada a mano como en el Vol.I (cap. 4).

Cuatro piezas, todas en `cap22_caminos_minimos.rs`:

1. **La fuente de pesos** (`WeightSource`) — de dónde sale el peso de una arista: una propiedad (`WEIGHT relationship.distance`, la consulta del brief) o una constante.
2. **La extracción estricta** (`edge_weight`) — semántica tipada para el dato sucio: ausente, no numérico, no finito.
3. **Dijkstra** (`dijkstra` / `dijkstra_path`) — min-heap de std, borrado perezoso, finalización anticipada.
4. **Bellman-Ford** (`bellman_ford` / `bellman_ford_path`) — pesos negativos legítimos y detección del ciclo negativo que contamina.

Este capítulo abre la Parte V: algoritmos que ya no recorren el motor (Parte IV), sino el grafo.

## 22.2 Problema

La consulta que abre esta parte existe en el brief desde el principio:

```text
SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance
```

Fíjate en lo que exige: el peso no lo pone quien consulta, lo pone QUIÉN GUARDÓ el dato. `relationship.distance` es una propiedad de arista (cap. 7): un `Value` dentro de `Edge.props`. Y un grafo de propiedades es schemaless — nadie garantiza que la propiedad exista en todas las aristas, ni que sea numérica donde exista.

Compruébalo con nuestro propio `demo_graph` (cap. 20): las tres `KNOWS` «de verdad» llevan `since` (2020, 2021, 2022), pero el self-loop de Dani (edge 3) no lleva ninguna, y las dos `LIVES_IN` tampoco. Pregunta por `SHORTEST ... WEIGHT relationship.since` y el grafo te responde: no puedo, hay tramos sin precio. El problema del capítulo no es el algoritmo — ya lo conoces del Vol.I — sino el CONTRACTO entre el algoritmo y un dato que nadie validó al escribirlo.

Y hay un segundo problema, más silencioso. Hasta hoy, «camino» en LiraDB ha querido decir saltos: el `Expand` del cap. 20 encadena relaciones y cuenta. Pero con pesos, el directo caro pierde contra el rodeo barato:

```text
        2.0        3.0
    0 ────► 1 ────► 2        coste por arriba: 5.0
    └──────────────────►      coste por abajo: 10.0 (una arista sola)
             10.0
```

El camino de dos saltos cuesta 5; el de un salto, 10. Menos aristas no es más barato. Todo lo que sigue existe para responder a esa obviedad con rigor.

## 22.3 Modelo mental

Piensa en una **red de tramos con tarifa**: nodos = estaciones; aristas = tramos; pegado a cada tramo, su precio (la propiedad). El coste de un viaje es la suma de los precios de los tramos que cruzas. Sobre esa misma red, dos formas de organizarse:

- **Dijkstra es la ventanilla que llama por tarifa.** Cada estación espera su turno con un número provisional; la ventanilla llama SIEMPRE al más barato pendiente. Cuando te llama, tu tarifa queda lacrada: es definitiva, porque todo el que sigue en la cola pagó ya igual o más para llegar hasta ahí. Eso es `settled`. Y si tú sólo esperabas a UNA estación (tu destino), cuando la llaman te levantas y te vas: **finalización anticipada**.
- **Bellman-Ford es el tablón de anuncios por rondas.** Cada tarde (cada pasada), todos los viajeros releen el tablón completo de tarifas y pegan su mejora. No hay orden de llamada, nada se lacra — pero tolera descuentos (pesos negativos) y, si en el pueblo hay una rueda de descuento infinita que alguien puede alcanzar, la última pasada la delata.

```
 DIJKSTRA (ventanilla)                    BELLMAN-FORD (tablón)
 heap: [(0,A)]                            ronda 1: releer TODAS las aristas
 llama a A (0.0)  ── lacrado              ronda 2: releer TODAS las aristas
 heap: [(1,B),(4,C)]                      ...hasta que una ronda no pega nada
 llama a B (1.0)  ── lacrado              pasada extra de verificación:
 llama a C (3.0)  ── lacrado                ¿algo aún mejora? → ciclo negativo
 (todo lo que quede pesa ≥ que lo
  ya llamado: por eso lacre = verdad)
```

El momento ¡ajá!: **el orden de llamada no es un truco de implementación, es la corrección misma**. Puedo lacrar a B porque ningún tramo futuro puede rebajar lo ya pagado — y eso es exactamente lo que un precio negativo rompe. Dijkstra rechaza negativos porque su prueba de corrección los usa; Bellman-Ford paga V-1 pasadas para recuperar el derecho a tenerlos.

## 22.4 Primera solución

La versión ingenua ya la tienes funcionando: encadenar `Expand` (cap. 20) y contar saltos. De hecho es TAN legítima que la hicimos oficial: `WeightSource::Constant(1.0)` es el `Default` — con todos los tramos a precio 1, el camino mínimo ponderado degenera en el camino con menos saltos, que es lo que el motor llevaba haciendo sin saberlo.

Y la versión ingenua «ponderada» que escribiría cualquiera: leer `edge.props["distance"]` y, si no está, usar 1.0 «para no romper la consulta».

## 22.5 Sus límites

1. **El default amable miente.** Con `1.0` donde falta el dato, un import a medias produce caminos con precios inventados — y nadie lo sabe. Un grafo schemaless debe VER su dato sucio; una base de datos que rellena el hueco no está siendo amable, está mintiendo con buena letra.
2. **El NULL no es cero ni es uno.** En el cap. 20 decidimos que la propiedad ausente SE VE como NULL; hoy decidimos la simetría: el NULL SE TRATA como ausencia. «Sin precio pegado» es un único concepto con un único error (`MissingWeight`).
3. **Un NaN suelto revienta el heap.** `BinaryHeap` exige `Ord`; `f64` no lo implementa precisamente porque el NaN no tiene orden total. Sin validación previa, el día que alguien importe un `distance: NaN`, tu consulta muere de panic a mitad de cálculo — o peor, ordena «a medias».
4. **El infinito ya está ocupado.** Usamos `f64::INFINITY` como centinela de «inalcanzable». Si un coste real desborda a infinito (1e308 + 1e308), se colaría en el disfraz de inalcanzable: un camino que existe reportado como «no hay camino».

## 22.6 Solución evolucionada, parte 1: la fuente de pesos (o el precio pegado al tramo)

Primera pieza: decir DE DÓNDE sale el peso. Dos historias, un enum:

```rust
pub enum WeightSource {
    Property(String),   // WEIGHT relationship.distance — el caso del brief
    Constant(f64),      // todas igual; Default = 1.0 = contar saltos
}
```

Y la extracción, con semántica estricta y errores tipados (`edge_weight`):

| El dato que hay en `props[name]` | Resultado |
|---|---|
| nada, o `Value::Null` | `MissingWeight { edge, prop }` |
| `Bool`, `String`, `Bytes` | `InvalidWeight { edge, prop, found }` (con `Value::type_name`) |
| `Int(i)` | `i as f64` — promoción Int→Float |
| `Float` NaN o ±∞ | `NonFiniteWeight { edge, weight }` |

Los tres errores hablan en cristiano, con arista y propiedad — esto es su `Display` REAL:

```text
edge 3 has no weight property 'since' (missing or NULL)    ← MissingWeight
edge 3: weight property 'cost' is String, not a number     ← InvalidWeight
edge 3: non-finite weight NaN                              ← NonFiniteWeight
```

**¿Por qué tan estrictos, si un default sería tan cómodo?** Porque las tres filas de esa tabla son preguntas distintas que el usuario necesita oír por separado: «no guardaste el peso» (corríjase el import), «guardaste texto donde iba un número» (corríjase el ETL), «guardaste algo que no es representable» (corríjase el origen). Un `1.0` silencioso fusiona las tres en una respuesta limpia sobre un dato roto. El test `demo_graph_con_pesos_reales_y_calidad_de_dato` lo documenta con nuestro propio grafo: con `WeightSource::property("since")`, la primera arista sin el dato —el self-loop de Dani, edge 3— es la que salta, ANTES de calcular nada.

**¿Por qué promoción Int→Float y no aritmética entera?** Porque los pesos nacen del `Value` del cap. 7, que ya mezcla `Int` y `Float`, y porque el coste de un camino es una suma que se quiere uniforme. Tiene un precio documentado y testeado: f64 representa enteros exactos sólo hasta 2^53, así que `9.007.199.254.740.993` (2^53+1) se promueve a `9.007.199.254.740.992` — el test `pesos_int_se_promocionan_a_float_con_perdida_documentada` lo clava con números, y con valores razonables la suma es exacta.

**¿Por qué `Constant` por defecto?** Lo menos sorprendente cuando nadie ha dicho qué propiedad es el peso: contar saltos, la semántica que el motor ya tenía. Y deja explícita una idea que vale el capítulo entero: «no ponderado» no es otro problema — es el caso particular en que todos los tramos valen 1. El test `la_fuente_de_pesos_cambia_la_respuesta` ejecuta el MISMO grafo con las dos fuentes y gana un camino distinto: ponderado, el rodeo (5.0); a saltos, el directo (1.0).

## 22.7 Solución evolucionada, parte 2: Dijkstra sobre el store (la ventanilla)

El corazón es literalmente el del Vol.I, cap. 4 — pero leyendo aristas del puerto del cap. 8:

```rust
let mut heap: BinaryHeap<Reverse<(Cost, NodeId)>> = BinaryHeap::new();
while let Some(Reverse((Cost(d), u))) = heap.pop() {
    if settled[u] { continue; }          // entrada obsoleta: borrado perezoso
    settled[u] = true;                   // lacre: dist[u] es definitiva
    if target == Some(u) { break; }      // finalización anticipada
    for eid in store.out_edges(u) {
        // ... relajar: new = d + w; si new < dist[v] { push }
    }
}
```

Cada línea tiene su porqué:

**¿Por qué sobre `&dyn GraphStore` y no sobre el CSR del cap. 14?** Porque los pesos viven en `Edge.props`, y EdgeId→Edge es exactamente el acceso que da el trait; el CSR persiste SÓLO topología (offsets + targets, sin ids de arista), así que no puede responder «¿cuánto pesa esta arista?». Podríamos proyectar un CSR con pesos… y esa es exactamente la proyección que el cap. 26 generalizará. Deuda explícita, no omisión. De regalo, trabajar contra el trait mantiene el algoritmo agnóstico al backend: `MemoryStore` hoy, disco mañana.

**¿Por qué `Reverse`?** El `BinaryHeap` de std es un max-heap: sin `Reverse`, llama primero al PEOR candidato. La clave `(Cost, NodeId)` ordena por coste y desempata por id — mismas entradas, mismo orden de salida, siempre.

**¿Por qué borrado perezoso y no decrease-key?** Porque std no tiene decrease-key (no hay forma de «bajar» la prioridad de un elemento ya insertado). El patrón: cuando `dist[v]` mejora, se INSERTA una entrada nueva y la vieja se descarta al salir, detectada por `settled[u]`. Coste: O(log n) por push y algunos pops muertos (que `PathStats.popped` cuenta honestamente). La alternativa —una cola indexada con decrease-key— existe en crates externas; la descartamos por el mismo criterio del Vol.I: std y nada más que std.

**¿Por qué `Cost` como newtype?** `f64` no implementa `Ord` (culpa del NaN) y el heap lo exige. `Cost` envuelve el f64 y le da orden total; su `expect` es un unreachable documentado —todo lo que entra en el heap es finito porque `edge_weight` ya lo garantizó y los desbordes se reportan como `CostOverflow` antes de tocar el centinela.

**¿Por qué finalización anticipada?** No es una micro-optimización: es el invariante codicioso hecho código. Cuando tu destino sale del heap, su distancia es definitiva (todo lo pendiente pesa igual o más). El test `dijkstra_finalizacion_anticipada_extrae_menos_nodos` lo mide: en una cadena 0→1→…→5, preguntar por 1 asienta {0, 1} y se va; la tabla completa asienta los seis.

**¿Y los negativos?** Aquí está la decisión más discutida del capítulo: `validate_edge_weights` recorre TODAS las aristas ANTES de correr y rechaza cualquier negativo con `PathError::NegativeWeight` — aunque la consulta no fuera a pisar esa zona. ¿No es exagerado? Piensa qué significa la alternativa: consultas que a veces aciertan y a veces devuelven números plausibles pero malos, según qué zona del grafo tocaron. Una base de datos prefiere FALLAR RUIDOSAMENTE a contestar casi-bien; el día que importes pesos negativos, querrás enterarte en todas tus consultas, y el propio mensaje te dice la salida: `use bellman_ford`.

### Dijkstra en marcha: la vida de una entrada del heap

Merece la pena seguir una ejecución completa. El diamante del Vol.I — `0→1 (e0, w1)`, `1→3 (e1, w2)`, `0→2 (e2, w4)`, `2→3 (e3, w3)` — con destino 3:

| Paso | Pop | Acción | Heap tras el paso |
|---|---|---|---|
| 0 | — | `dist[0]=0`, push (0, 0) | `[(0,0)]` |
| 1 | (0, 0) | asienta 0; relaja e0: `dist[1]=1` (push); relaja e2: `dist[2]=4` (push) | `[(1,1), (4,2)]` |
| 2 | (1, 1) | asienta 1; relaja e1: `dist[3]=3` (push, con su `PathStep`) | `[(3,3), (4,2)]` |
| 3 | (3, 3) | asienta 3 = **destino → break**. La (4, 2) ni se mira | (queda (4,2), ignorada) |

Tres pops, tres expandidos, cuatro intentos de relajación, tres que mejoraron. La entrada `(4, 2)` sobrevive en el heap sin importar nada: ahí está el borrado perezoso en acción — si el bucle continuara (variante tabla completa), saldría, asentaría el 2 con `dist[2]=4` definitivo, y relajaría e3 (`4+3=7 > 3`: intento fallido, no mejora). El momento en que un pop descubre `settled[u] == true` y hace `continue` es la firma del patrón: esa entrada nació de una promesa que otra mejora dejó obsoleta. Y la relajación que sí mejora es literalmente:

```rust
if new < dist[v] {
    dist[v] = new;
    pred[v] = Some(PathStep { edge: eid, from: u, to: v, weight: w });
    heap.push(Reverse((Cost(new), v)));   // la entrada vieja quedará obsoleta
}
```

Nota lo que NO hay: ningún «buscar en el heap y bajar la prioridad». Se inserta y se abandona. El coste de esa comodidad —pops muertos— queda anotado en `stats.popped`, no escondido.

## 22.8 Solución evolucionada, parte 3: Bellman-Ford (el tablón que admite descuentos)

Bellman-Ford compra con sus V-1 pasadas el derecho que Dijkstra no puede pagar: pesos negativos LEGÍTIMOS (una arista de -4 es una respuesta, no un error — test `bellman_ford_explota_un_peso_negativo_que_dijkstra_rechaza`: la ruta 3 + (−4) = −1 gana al directo 1). Míralo en rondas sobre ese mismo grafo — aristas en orden de id: `e0: 0→2 (w3)`, `e1: 2→3 (w−4)`, `e2: 0→3 (w1)`:

| Ronda | Lo que pasa al releer el tablón | dist tras la ronda |
|---|---|---|
| 1 | e0: `dist[2]=3` (mejora); e1: `3−4=−1` → `dist[3]=−1` (¡mejora!); e2: `0+1=1 < −1`? No | `[0, ∞, 3, −1]` |
| 2 | nada mejora | `[0, ∞, 3, −1]` → `changed=false`, BREAK |

`rounds == 2`, no las V−1 = 3 posibles: la parada temprana trabajó. Fíjate en la ronda 1: BF procesó `e1` ANTES que `e2` por pura casualidad de orden de lista — con otro orden, `dist[3]` habría bajado primero a 1 y a −1 en la ronda 2. El RESULTADO converge igual (−1); el camino que recorre cada ronda, no. Dijkstra no tiene esa tolerancia al azar: su heap impone un orden que ES su prueba de corrección.

Tres decisiones más de la implementación:

**La lista de relajación se materializa UNA vez.** Antes de las pasadas, `bellman_ford` recolecta `(source, target, edge_id, weight)` de todas las aristas en un `Vec`. ¿Por qué? Porque los pesos se leen de las props, y releerlas en cada ronda serían V-1 búsquedas hash por arista para el MISMO valor. Y si el store viviera en disco (caps. 11-13): V-1 lecturas de página por arista. El capítulo del buffer pool se cobra aquí su moraleja: cada lectura que puedas no repetir, no la repitas.

**Parada temprana de pasadas, sí; del destino, no.** Si una pasada no mejora nada, el tablón ha convergido: otra vuelta no cambiaría nada, `if !changed { break; }` — una cadena de 4 saltos converge en 2 rondas, no en las V-1=4 posibles (test `bellman_ford_para_temprano_cuando_nada_cambia`, `rounds == 2`). Pero NO hay early-exit «cuando el destino ya tiene distancia», y la ausencia es deliberada: con negativos, un camino más LARGO puede ganar DESPUÉS. Lacrar el destino exigiría el invariante codicioso, y BF existe precisamente porque renunció a él.

**La pasada de verificación distingue el ciclo que contamina.** Tras las V-1 pasadas, una vuelta extra: si alguna arista con `dist[u]` finito TODAVÍA relaja, hay un ciclo negativo ALCANZABLE desde el origen → `PathError::NegativeCycle` señalando ESA arista. ¿Por qué reachable y no «exista»? Aguas abajo de un ciclo alcanzable las distancias tienden a −∞: devolver media tabla válida sería mentir. Pero un ciclo en una componente que NADIE alcanza desde el origen no contamina la respuesta — el test `bellman_ford_ciclo_negativo_inalcanzable_no_contamina` tiene los dos casos en el mismo archivo: la isla rota responde con normalidad, y el mismo ciclo conectado al origen sí es error. Fíjate en la condición exacta: `dist[u] != f64::INFINITY && dist[u] + w < dist[v]` — la primera mitad ES la distinción alcanzable/inalcanzable.

Y una garantía silenciosa que agradece el lector: sin negativos, BF y Dijkstra dan LA MISMA tabla, distancia a distancia (test `bellman_ford_coincide_con_dijkstra_sin_negativos`). Dos algoritmos, un contrato de resultado.

## 22.9 Prueba de fuego

Los tests no verifican «devuelve un número»: verifican que el camino es VÁLIDO contra el store — continuidad, aristas existentes, coste = suma de pesos (`assert_camino_valido`). Tres pruebas destacadas:

**El oráculo CSR.** El cap. 14 nos dio una proyección topológica del grafo; hoy le pedimos que actúe de testigo (`proyeccion_csr_consistente_con_lo_alcanzado_por_dijkstra`): la alcanzabilidad por BFS sobre el CSR debe coincidir EXACTO con `sp.reached()` de Dijkstra. Topología y algoritmo cuentan la misma historia — y cuando el cap. 26 añada pesos al CSR, este test es el molde del oráculo definitivo.

**El grafo demo con calidad de dato.** `dijkstra_path(&demo, 0, 2, &Default::default())` → coste 2.0 por 0 -KNOWS-> 1 -KNOWS-> 2; con `property("since")` → `MissingWeight { edge: 3 }` (el self-loop de Dani); y Dani inalcanzable → `Ok(None)`, que no es error: es una respuesta. Es el doctest del módulo, ejecutado por `cargo test --doc` en cada build:

```rust
let store = demo_graph();
let sp = dijkstra(&store, 0, &Default::default()).unwrap(); // pesos = 1.0 (saltos)
assert_eq!(sp.distance(2), Some(2.0)); // 0 -KNOWS-> 1 -KNOWS-> 2
assert_eq!(sp.distance(5), Some(2.0)); // 0 -KNOWS-> 1 -LIVES_IN-> 5 (Lisboa)
assert_eq!(sp.distance(3), None);      // Dani (sólo self-loop) es inalcanzable
```

Tres líneas, tres lecciones: los saltos son `Constant(1.0)`; un camino puede cruzar tipos de arista (KNOWS + LIVES_IN) si la fuente de pesos no discrimina; y `None` es parte del contrato, no un fallo.

**La respuesta con formato.** `Path` implementa `Display` estilo Cypher — `(n0)-[e4 w=1.5]->(n1)-[e7 w=2]->(n2) cost=3.5` — porque el `PathStep` guarda arista, extremos y peso de cada salto. ¿Por qué el predecesor es el paso COMPLETO y no un `NodeId`? Para que `path_to` reconstruya sin volver a tocar el store: cada `get_edge` que te ahorras es una página que no pides al pager (cap. 12). En memoria no se nota; en disco, es la diferencia entre una reconstrucción gratis y E lecturas.

¿Y si te saltas este capítulo? Tus caminos «ponderados» siguen siendo BFS con otro nombre, los pesos ausentes se rellenan en silencio y nadie puede explicar por qué la misma consulta responde distinto tras un import sucio. Síntoma exacto: números plausibles, cero contrato.

## 22.10 Qué miden las PathStats

`PathStats` es la métrica del cap. 20 aplicada al algoritmo — el «cuánto cuesta calcular»:

- **`relax_attempts` vs `relax_updates`**: aristas consideradas vs relajaciones que MEJORARON una distancia. El ratio es un diagnóstico del grafo: si intentas 1.000 veces y mejoras 12, el grafo «no mejora» — casi todo tu trabajo fue mirar aristas que ya no podían ganar. Un camino óptimo en un grafo hostil es, sobre todo, una colección de intentos fallidos.
- **`popped`** (Dijkstra): extracciones del heap, entradas obsoletas INCLUIDAS. Es el precio visible del borrado perezoso: pops muertos que existen porque std no tiene decrease-key.
- **`rounds`** (Bellman-Ford): pasadas ejecutadas — la convergencia real frente a las V-1 teóricas.
- **`expanded`**: pops VIVOS (sin obsoletos). Aparece numerado YA, pero es del cap. 23: será la vara para medir cuánto ahorra A* frente a Dijkstra. Hoy coincide con los pops útiles; mañana, la comparación.

## 22.11 Qué hemos sacrificado

1. **Velocidad frente a un CSR con pesos**: cada relajación pasa por `out_edges` + `get_edge` del trait. El cap. 26 pagará esa deuda con la proyección.
2. **Decrease-key real**: el perezoso gasta memoria en entradas duplicadas y pops muertos. Medido en `popped`; aceptado por no depender de crates.
3. **Aritmética exacta**: la promoción Int→Float pierde más allá de 2^53. Documentada y testeada; para pesos físicos, irrelevante; para bitcoins, ya sabes dónde mirar.
4. **Dijkstra «híbrido» que validara sólo lo que pisa**: sería más rápido en grafos parcialmente sucios — y una inconsistencia entre consultas. Preferimos el fail ruidoso.
5. **Recuperar distancias parciales tras un ciclo negativo**: cuando BF detecta el ciclo, tira TODA la respuesta. Devolver «la parte buena» invitaría a leerla como completa.

## 22.12 Cómo lo hace una BBDD real

- **Neo4j (Cypher clásico)**: la función `shortestPath()` cuenta RELACIONES, no pesos — exactamente nuestro `Constant(1.0)` como ciudadano de primera clase.
- **Neo4j GDS**: `gds.shortestPath.dijkstra.stream(..., { relationshipWeightProperty: 'distance' })` — el mismo parámetro conceptual que nuestro `WeightSource::Property`, y el mismo contrato: la documentación exige pesos POSITIVOS y, si no especificas propiedad, corre «unweighted» (nuestro `Constant`).
- **Kùzu**: la sintaxis `SHORTEST k` sobre relaciones de longitud variable (`MATCH (a)-[:Follows* SHORTEST 1..10]->(b)`) se resuelve con un recursive join tipo BFS — saltos, otra vez — y la variante ponderada llegó con función de coste explícita. Detalle delicioso: un PR de 2025 añadió al weighted shortest path un chequeo de pesos negativos que ERRA en vez de ignorarlos — la industria llegando, por su cuenta, a nuestra misma decisión del §22.7.
- **pgRouting**: `pgr_dijkstra(edges_sql, start, end)` — y una política opuesta a la nuestra que vale oro como contraste: «un valor negativo en la columna cost se interpreta como que la arista NO EXISTE». Ellos BORRAN silenciosamente; nosotros gritamos `NegativeWeight`. Para negativos legítimos tienen `pgr_bellmanFord` (experimental), con Bellman (1958) y Ford (1956) en el nombre.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en el grafo del §22.2, ¿qué camino gana con `Constant(2.5)` y cuál es su coste? ¿Cambia el ganador respecto a `Constant(1.0)`?
- *Intermedio*: pgRouting trata el coste negativo como arista inexistente; nosotros devolvemos `NegativeWeight`. Construye un caso donde esa diferencia cambie la RESPUESTA (no sólo el error).
- *Experto*: ¿qué le pasaría a `Cost::cmp` si relajáramos un NaN hasta el heap a pesar de la validación? Describe el modo de fallo exacto y por qué lo llamamos «unreachable documentado».

## 22.13 Lo que te llevas

- **El peso es un dato, no un parámetro**: vive en `Edge.props`, llega por `WeightSource::Property` y su ausencia es `MissingWeight`, nunca un default.
- **Semántica estricta en tres errores tipados**: ausente/NULL, tipo no numérico, no finito — un grafo schemaless debe VER su dato sucio.
- **`settled` + heap con `Reverse`**: el lacre del invariante codicioso; el borrado perezoso es el precio de no tener decrease-key en std.
- **Early-exit al destino en Dijkstra, NUNCA en Bellman-Ford**: uno lo autoriza el invariante; el otro renunció a él para admitir negativos.
- **Ciclo negativo: sólo el ALCANZABLE contamina** — y la arista señalada es la que aún relaja.
- **`CostOverflow` existe para que el infinito real no se disfraze de inalcanzable.**
- **`PathStats` numera el trabajo**: attempts vs updates, popped con muertos, rounds de convergencia.

## 22.14 Ojo, cuidado con…

- **Confundir menos saltos con más barato**: la trampa nº 1. Cura: ejecutar `la_fuente_de_pesos_cambia_la_respuesta` y mirar cómo gana un camino distinto por fuente.
- **Peso vs coste**: el peso es de UNA arista; el coste, del camino entero. `PathStep.weight` y `Path.cost` están a un lado y otro de esa frontera.
- **Inalcanzable vs desbordado**: ambos dan «infinito» en f64, pero uno es `Ok(None)` y el otro es `CostOverflow`. El centinela sólo significa lo primero.
- **Esperar symetría de early-exits**: BF tiene parada temprana (pasadas), no finalización anticipada (destino). No son lo mismo ni sirven para lo mismo.
- **Tratar NaN como «un número raro»**: es un ladrón de orden total. Todo lo que entra en un heap debe poder compararse SIEMPRE.

## 22.15 Pin de batalla

> *«Una base de datos que rellena el peso ausente con un 1.0 no está siendo amable: está mintiendo con buena letra.»*

## 22.16 Si solo lees 30 segundos

Los caminos mínimos ponderados leen el peso de las PROPIEDADES de las aristas (`WeightSource`), y como el grafo es schemaless, la extracción es estricta: peso ausente o NULL, tipo no numérico, o NaN/±∞ son errores tipados — nunca defaults. Dijkstra corre sobre `&dyn GraphStore` con un heap binario (`Reverse` + borrado perezoso, porque std no tiene decrease-key), lacra nodos al asentarlos y corta cuando el destino sale del heap; valida TODOS los pesos ANTES de empezar y rechaza negativos aunque no los pise, porque contestar casi-bien es lo peor que puede hacer una BD. Bellman-Ford admite negativos (V-1 pasadas sobre una lista materializada una vez, con parada temprana), y su pasada de verificación convierte el ciclo negativo ALCANZABLE en un error que señala la arista — el inalcanzable no contamina. Ambos devuelven lo mismo: tabla `ShortestPaths` y `Path` con pasos, coste y estadísticas.

## 22.17 Una historia pequeña

La primera vez que ejecutamos `dijkstra` con `WeightSource::property("since")` sobre el grafo demo, el resultado fue un error: `MissingWeight { edge: 3 }`. Reunión exprés: «ponle un default de 1.0 y listo, así no rompemos las consultas de nadie». Lo estuvimos mirando un rato hasta que alguien hizo la cuenta en voz alta: con el default, el self-loop de Dani costaría 1 — un precio que nadie escribió jamás — y las `LIVES_IN` también, y la consulta habría respondido un camino «barato» por tramos sin etiquetar. Decidimos esa tarde que el error era la respuesta correcta. Meses después, al leer la documentación de pgRouting, encontramos que ellos eligieron lo contrario (el coste sospechoso borra la arista, en silencio) — y nos alegramos dos veces: de nuestra decisión, y de tener un contraejemplo industrial con el que explicarla.

## Ejercicios resueltos

**1. Sobre `demo_graph`, ¿por qué `dijkstra(&s, 0, &WeightSource::property("since"))` falla con `MissingWeight { edge: 3 }` y no con una de las `LIVES_IN`, si ambas carecen de `since`?**

Porque la validación es EAGER y recorre `iter_edges()` en orden de id: las aristas 0, 1 y 2 (las KNOWS reales) llevan `since`; la primera que no lo lleva es la 3 —el self-loop de Dani— y ahí se detiene el mundo. Las `LIVES_IN` (ids 4 y 5) también están sucias, pero el error informa de la PRIMERA ofensora. Y fallar es correcto aunque la consulta fuera 0→5 y jamás pisara el self-loop: la respuesta de una BD no debe depender de qué zona del grafo llegó a tocar la búsqueda. Verificación: `demo_graph_con_pesos_reales_y_calidad_de_dato`.

**2. En el diamante del Vol.I (0→1 peso 1, 1→3 peso 2, 0→2 peso 4, 2→3 peso 3), ¿qué guarda `pred[3]` y cómo reconstruye `path_to` sin tocar el store?**

Gana 0→1→3 (coste 3.0 contra 7.0 por arriba): la última relajación que fijó `dist[3]` fue la arista 1 con `PathStep { edge: 1, from: 1, to: 3, weight: 2.0 }` — y eso es exactamente `pred[3]`. `path_to(3)` sigue la cadena hacia atrás (3 → 1 → 0 se corta en `pred[0] = None`), da la vuelta, y devuelve `nodes() = [0, 1, 3]` con `cost = 3.0` SIN una sola lectura del store: cada paso ya lleva su arista y su peso. Los predecesores no pueden ciclar: un ciclo en la cadena exigiría distancias estrictamente decrecientes al rodearlo — un ciclo negativo, que BF ya habría rechazado. Verificación: `dijkstra_camino_clasico_del_diamante` con `assert_camino_valido`.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sobre el grafo del §22.2 (0→1 peso 2, 1→2 peso 3, 0→2 peso 10), predice SIN ejecutar: camino y coste con `Constant(1.0)`, con `property("weight")`, y con `Constant(2.5)`; y la variante EXACTA del error (con su arista) si el peso viniera de `"distance"`. Verifícate con `la_fuente_de_pesos_cambia_la_respuesta` y `peso_ausente_o_null_es_missing`. *Pistas*: (1) ¿qué lee un `Constant` de las aristas — y qué cuenta entonces?; (2) ¿la validación eager mira el camino o el grafo?; (3) ¿en qué orden entrega `iter_edges` las ofensoras? *Criterio*: los tres costes exactos y la variante del error con arista y propiedad.

**Intermedio (analizar — con los caps. 11-14 en la mano).** (a) Explica por qué `bellman_ford` materializa la lista de relajación una sola vez, y qué coste tendría releer `Edge.props` en cada pasada si el store viviera en disco (cada `get_edge` puede costar una página: ¿cuántas rondas × aristas releería una cadena de 4 saltos?). (b) ¿Qué rompería exactamente un NaN que llegara al heap a pesar de todo — qué exige `BinaryHeap` de sus elementos y qué devuelve `partial_cmp` con NaN? (c) ¿Por qué `pred` guarda el `PathStep` completo en vez de sólo el `NodeId`? Verifícate con `bellman_ford_para_temprano_cuando_nada_cambia` y `peso_nan_o_infinito_es_no_finito`. *Pistas*: (1) cuenta las rondas del test; (2) mira el `expect` de `Cost::cmp`; (3) cuenta las lecturas de `get_edge` en una reconstrucción. *Criterio*: razonar en lecturas/páginas, no sólo en O-grandes.

**Experto (crear).** Primera parte, de memoria (sin mirar el capítulo): escribe la vida de una entrada del heap de Dijkstra — cuándo nace (push), cuándo queda obsoleta, cuándo la descartan y qué contador la cuenta — y enuncia el invariante que autoriza el `break` del early-exit. Segunda parte: escribe el test `ciclo_negativo_inalcanzable_se_vuelve_error_al_conectarlo` — parte del grafo de `bellman_ford_ciclo_negativo_inalcanzable_no_contamina` y añade UNA arista desde el origen hacia el ciclo; exige `NegativeCycle` señalando una arista concreta, y que `bellman_ford_path` también falle. *Pistas*: (1) ¿qué convierte «inalcanzable» en «alcanzable» con una sola arista?; (2) ¿qué arista tiene que seguir relajando tras V-1 pasadas?; (3) ¿qué condición sobre `dist[u]` pone la pasada de verificación? *Criterio*: test verde con `cargo test -p vol2-liradb` y saber explicar por qué ESA arista y no otra.

## Para profundizar

- **E. W. Dijkstra, «A Note on Two Problems in Connexion with Graphs» (Numerische Mathematik 1, 1959, 269-271)** — las dos páginas y media originales: camino mínimo y árbol generador mínimo, tal como salieron de la terraza del café.
- **Philip L. Frana, «An Interview with Edsger W. Dijkstra» (CACM 53(8), agosto 2010; historia oral del Charles Babbage Institute, 2001)** — el propio Dijkstra contando los veinte minutos, la falta de lápiz y papel, y «evitar toda complejidad evitable».
- **Cormen, Leiserson, Rivest y Stein, «Introduction to Algorithms», 3ª ed., cap. 24** — la prueba del invariante codicioso (§24.3) y el teorema de detección de ciclos negativos alcanzables (§24.1): el rigor detrás de cada decisión de este capítulo.
- **Docs de Neo4j GDS («Dijkstra Source-Target Shortest Path») y de pgRouting (`pgr_dijkstra`, `pgr_bellmanFord`)** — los dos contractos de pesos de producción, con el contraste silencio-vs-error comentado en §22.12.
- **Blog de Kùzu (release 0.0.4, «Shortest path queries»)** — cómo sintaxis Cypher (`SHORTEST k`), BFS y recursive joins conviven en un motor embebido real.

## Mini-diálogo: en la ventanilla

> — Entonces Dijkstra es un BFS con precios.
>
> — Es un BFS que aprendió a callarse hasta que sabe. BFS contesta por niveles de saltos; Dijkstra espera y llama por tarifas, y sólo cuando te llama tu número es definitivo. Esa paciencia ES la corrección.
>
> — ¿Y por qué tanto drama con un peso negativo? Total, un descuento.
>
> — Porque el lacre entero depende de que nadie pueda rebajar lo ya pagado. Un descuento rompe esa promesa: el que ya fue llamado podría mejorar. Bellman-Ford es el que acepta vivir sin lacre — y por eso paga V-1 pasadas y vigila la rueda de descuento infinita.
>
> — ¿Y si falta el precio de un tramo?
>
> — La ventanilla se cierra. Un `1.0` inventado te daría un viaje barato por tramos que nadie tasó. Preferimos que el grafo pase vergüenza una vez a que mienta para siempre.

---

*(Próximo capítulo: 23 — A*, heurísticas y búsquedas dirigidas. Dijkstra explora en círculos crecientes alrededor del origen; el destino podría estar al otro lado del grafo, esperando a que el círculo llegue. ¿Se puede tirar del hilo sin romper la optimalidad? La respuesta exige una propiedad nueva — admisibilidad — y `PathStats.expanded`, el contador que dejamos encendido hoy, medirá cuánto ahorra.)*
