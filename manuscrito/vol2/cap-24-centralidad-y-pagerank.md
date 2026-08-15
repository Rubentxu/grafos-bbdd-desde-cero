# Capítulo 24 — Centralidad y PageRank

> *«Un enlace no es un voto igualitario: es la reputación del que enlaza, repartida entre sus salidas.»*

## 24.0 La anécdota de la esquina

En 1996, Larry Page llegó al doctorado de Stanford con una idea que sonaba académica: estudiar el **grafo de enlaces** de la web. Su director, Terry Winograd, le sugirió el ángulo que lo cambiaría todo: en ciencia, un artículo vale por sus citas — y un enlace ES una cita. La web entera, vista así, es el mayor grafo de citas jamás construido. Con Sergey Brin —y con Rajeev Motwani y Winograd como coautores— el proyecto, primero llamado BackRub, se convirtió en un buscador llamado Google, y en enero de 1998 la idea quedó por escrito: «The PageRank Citation Ranking: Bringing Order to the Web», un informe técnico de Stanford presentado también en la conferencia WWW7 (Brisbane, abril de 1998).

El corazón del paper es una imagen que vale un buscador: el **surfer aleatorio**. Alguien que navega pulsando enlaces al azar para siempre. ¿En qué páginas pasará más tiempo? La respuesta a esa pregunta de paseo infinito es el PageRank. Y dos detalles que hoy siguen vigentes: el factor de amortiguación `d = 0.85` que usaremos tal cual, y el nombre — un juego de palabras con el apellido Page.

La historia tiene un tercer acta verificable: la **patente US 6,285,999**, «Method for node ranking in a linked database». Solicitada el 9 de enero de 1998 por el propio Page… y asignada a Stanford, no a Google: la universidad la licenció en exclusiva a la empresa que sus dos estudiantes de doctorado fundaron el 4 de septiembre de 1998. La patente se concedió el 4 de septiembre de 2001 — tres años después, al día. En 2018 expiró. El IEEE la conmemora como hito («PageRank and the Birth of Google, 1996–1998»).

Este capítulo construye PageRank — y las cuatro familias de centralidad que lo rodean — sobre el grafo persistente de LiraDB. Y lo hace empezando por el final: primero veremos el algoritmo que PageRank vino a arreglar, y veremos fallar.

## 24.1 Objetivo

Al terminar este capítulo sabrás responder, sobre el grafo REAL de tu store, la pregunta «¿quién es importante aquí?» — y sabrás por qué esa pregunta tiene cinco respuestas distintas, cada una con su coste. Habrás construido, en `cap24_centralidad.rs`:

1. **Las familias clásicas**: grado, closeness (con corrección de Wasserman-Faust) y betweenness (algoritmo de Brandes 2001).
2. **El «antes» honesto**: la centralidad eigenvector — implementada con sus DOS fallos como tests, porque son la mejor demostración de por qué existe el siguiente.
3. **PageRank**: damping en (0,1) ABIERTO, masa colgante redistribuida uniformemente, convergencia L1 con historial por iteración — y su variante personalizada (`Teleport::Personalized`), la costura que el Vol. III usará para GraphRAG.

Una regla del capítulo, heredada del brief: estas familias no están implementadas con optimización industrial — están para explicar cómo PIENSA cada una, y el coste se mide (`CentralidadStats`), no se declama.

## 24.2 Problema

Mira el grafo demo que arrastras desde el capítulo 20:

```
   KNOWS:      0 → 1 → 2 → 0        (Ana, Bo, Carla: triángulo)
               3 → 3                (Dani: self-loop)
   LIVES_IN:   0 → 4,  1 → 5        (Madrid, Sevilla: sumideros)
```

Pregunta: ¿quién es la persona «más importante»? La respuesta ingenua ya la tienes: contar aristas. Pero el grado es una medida LOCAL: cuenta vecinos y no pregunta quién los aporta. En la web de 1998 esto era literalmente un campo de batalla: los buscadores contaban palabras y enlaces, así que los spammers rellenaban páginas de ambas cosas. La genialidad de Page y Brin fue darse cuenta de que **un enlace lo publica OTRO**: tú puedes repetir mil palabras, pero no puedes obligar a mil webs a enlazarte. El enlace ajeno es una señal difícil de falsificar.

Falta, aún así, la segunda mitad del insight: no todos los enlaces valen igual. Ser citado por un desconocido es algo; ser citado por una fuente importante es otra cosa. Quiero una métrica donde **la importancia se propague**: un nodo es importante si nodos importantes lo apuntan. Y una condición de ingeniería: el cálculo debe correr sobre `&dyn GraphStore`, el grafo persistente que llevas desde el capítulo 8, no sobre una matriz ad-hoc.

Y aquí un problema práctico que condiciona todo el código: los algoritmos iterativos van a tocar la adyacencia muchísimas veces — decenas de iteraciones, todas las aristas cada una. Si cada acceso pasa por `out_edges` + `get_edge`, pagamos el store completo en CADA ronda.

## 24.3 Modelo mental

El **surfer aleatorio**, con tres reglas:

1. **Pulsa enlaces**: desde la página actual, elige un enlace saliente al azar. Si la página tiene 3 enlaces, cada uno recibe 1/3 de su «voto».
2. **Se aburre**: con probabilidad 1−d (el `damping` d es la probabilidad de seguir pulsando), no pulsa nada y **teletransporta** a una página elegida según el vector de teleport (uniforme: cualquier página, 1/n).
3. **Los callejones teletransportan**: si cae en una página sin enlaces salientes (nodo colgante, dangling), se teletransporta igualmente.

```
   UNA iteración de PageRank (d = 0.85):

   x[u] = (1-d)·t[u]              ← teleport: la renta básica (1/n)
        + d·D/n                   ← cuota de la masa colgante
        + d·Σ_{v→u} x[v]/grado(v) ← votos: cada vecino reparte su
                                     importancia entre sus salidas

   Σ_u x[u] = 1  en CADA iteración  (la masa no se crea ni se destruye)
```

PageRank(u) = fracción de la eternidad que el surfer pasa en u. La «importancia» es una **masa** que fluye por los enlaces: entra por los votos, sale repartida entre las salidas, y el teleport la devuelve al sistema. Este modelo de masa es el que hace legibles todas las decisiones del capítulo: el delta de convergencia es «cuánta masa se movió», el dangling es «masa a punto de fugarse», el damping es «con qué fuerza vuelve la masa al sistema».

Comprueba el modelo en miniatura: una estrella de tres hojas apuntando a un centro. Las hojas no reciben votos — sin teleport, su importancia se desangra a 0 y el centro lo acumula TODO (es el fallo 1 del §24.5, y el test lo clava: centro > 0.999999, hojas < 1e-6). Con teleport, cada hoja cobra su renta básica de 1−d repartida en 1/n y el cuadro deja de ser tan despótico. Y si una hoja tuviera un enlace saliente hacia otra hoja, su voto valdría poco (pobre votante)… pero el del centro, repartido entre SUS salidas, vale por su prestigio. Voto = prestigio del emisor dividido por sus salidas: ni más ni menos.

El momento ¡ajá!: la importancia no se declara — se FLUYE. Y toda la aritmética del capítulo es contabilidad de ese flujo.

## 24.4 Primera solución

Primero la infraestructura común. Toda la Parte V corre sobre una **proyección** del store: `Proyeccion::proyectar(store, dir)` materializa una sola vez los nodos (ordenados por id: determinismo), un índice denso NodeId→posición (los huecos de `delete_node` quedan fuera del cálculo) y los vecindarios. Es el CSR del capítulo 14 en forma de memoria volátil: misma idea (adyacencia compactada, acceso O(1) por nodo), sin persistencia. ¿Por qué materializar? Porque el bucle de PageRank toca cada arista `iteraciones` veces: re-leer el store por ronda sería pagarlo O(iteraciones) veces. ¿Y por qué es PRIVADA y no ponderada? El guion pide las familias «para explicar»; el BFS de saltos basta. La versión con pesos — con la semántica estricta `edge_weight` del capítulo 22 — es una deuda explícita hacia el capítulo 26, que la saldó con `ProyeccionPonderada`.

Con dirección configurable: `GraphDirection::Out` (salientes), `In` (la transpuesta PURA de la salida, con aristas paralelas conservadas) y `Both` (la vista «no dirigida» como CONJUNTO: vecinos distintos y self-loop una sola vez — la convención del `Expand` UNDIRECTED del capítulo 20; sin dedup, un store simetrizado a mano contaría cada par doble).

Sobre esa base, las cinco familias del capítulo — cuatro «de consulta puntual» y una global. Las preguntas, para tenerlas juntas:

- **Grado** — «¿cuántos vecinos?». Una pasada, O(V+E), normalizado por n−1 (el máximo posible: estar conectado a todos). Con dirección: en el triángulo `0→1, 0→2, 2→0`, el nodo 0 tiene grado out 1.0 (2 de 2 posibles), pero grado in 0.5. Mismo grafo, dos preguntas: el que HABLA y el que es ESCUCHADO.
- **Closeness** — «¿a qué distancia estoy de todos?». Un BFS por nodo: O(V·(V+E)). En el camino simetrizado 0-1-2-3 sale el valor de libro: extremos 0.5 y centro 0.75 — `C = 3/Σd` con Σd = 6 en los extremos y 4 en el centro. Con la corrección de **Wasserman-Faust** para componentes desconectadas: `C(u) = ((r−1)/(n−1))·((r−1)/Σd)`, donde r es cuántos nodos alcanzas. Sin ella, en un grafo de dos 2-ciclos separados cada nodo daría 1.0 (¡perfecto dentro de su burbuja!) — con ella dan 1/3: penalizados por el mundo que NO alcanzan.
- **Betweenness** — «¿qué fracción de los caminos mínimos AJENOS pasan por mí?». Es la métrica del control: el nodo que está en TODOS los caminos entre otros dos es un cuello de botella — un aeropuerto hub, un router troncal, un corredor de información. En el camino 0-1-2-3, los intermedios 1 y 2 acumulan TODOS los pares: crudo 4, normalizado 2/3. En la estrella, el centro llega al máximo absoluto, 1.0: sin él, nadie habla con nadie. Aquí está la joya algorítmica del capítulo: **Brandes 2001**, con números en el §24.7.

Y la versión ingenua de la pregunta global: la **centralidad eigenvector**. «Soy lo que mis entrantes valen», sin ningún correctivo: `x_u ← Σ_{v→u} x_v`, iteración de potencia sobre la adyacencia cruda, normalizada en L2 cada paso. ¿Por qué la normalización, si «ensucia» el autovector? Porque la masa que escapa por los colgantes no vuelve: sin renormalizar, el vector entero se desangra hacia 0 y la iteración no tendría punto fijo que alcanzar. Con L2 por paso, al menos el vector conserva norma — y sus dos fallos quedan a la vista, que es justo lo que queremos de él. Caso borde honesto: un grafo sin aristas (autovalor 0, cualquier vector es autovector) devuelve el uniforme con `converged = true` sin iterar — no todo es drama.

## 24.5 Sus límites

El eigenvector crudo se rompe de DOS maneras en grafos dirigidos reales. No lo afirmamos: lo ejecutamos.

**Fallo 1: la masa se fuga por los colgantes.** Estrella con las hojas apuntando al centro: converge — pero las hojas mueren a 0 (`ev.score(hoja) < 1e-6`) y el centro se lo lleva todo (`> 0.999999`). ¿Y si el centro tuviera a su vez un sumidero favorito? La masa que entra en un nodo sin salidas SE ESCAPA del sistema: el flujo de importancia se desangra. En la cadena 0→1→2 lo ves en pequeño: la masa fluye río abajo, llega a 2… y desaparece del mundo. Iteración a iteración, el vector entero mengua — por eso el código renormaliza en L2 cada paso: para que al menos el vector conserve la norma mientras la ESTRUCTURA interna colapsa. En la web esto no es un caso borde: casi TODA página tiene enlaces salientes a alguna parte… y muchas (PDFs, imágenes, páginas abandonadas) no. Es la regla, no la excepción.

**Fallo 2: la oscilación periódica.** El grafo trampa: una cola que desemboca en un 3-ciclo (`0→1, 1→2, 2→3, 3→1`). La masa muere en la cola y lo que queda ROTA en el ciclo: en cada iteración el vector se desplaza un terzo de ciclo y al siguiente vuelve — oscila para siempre. El test lo dice sin rodeos: `eigenvector_centrality(&s, 100, 1e-9)` devuelve `converged = false` tras agotar las cien iteraciones, con `delta > 1e-9`.

```
   grafo: 0 → 1 → 2 → 3, con 3 → 1   (cola + 3-ciclo: 1→2→3→1)

   eigenvector (sin damping)          PageRank (d = 0.85)
   ─────────────────────────────      ─────────────────────────────
   la cola muere; la masa             la cola muere; la masa rota
   ROTA en el ciclo sin decaer:       AMORTIGUADA: en cada paso un
   it k:   pico en 1                  15 % se va al teleport y
   it k+1: pico en 2                  termina asentándose:
   it k+2: pico en 3, como en k       δ_k → 0 geométricamente
   δ_k ≈ constante > tol, nunca baja  (razón ≈ d·λ₂ < 1)
   converged = false                  converged = true
```

Si tu motor de ranking se rompe en un ciclo de tres nodos, no tienes un motor de ranking. Hace falta algo que repare AMBOS fallos: que devuelva la masa fugada y que amortigüe la rotación. Ese algo no es un parche numérico — es el damping.

El test que lo clava es tan corto que cabe entero — y es, en cuatro líneas, la tesis del capítulo:

```rust
// Cola + 3-ciclo: T→A, A→B, B→C, C→A.
let s = dirigido(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]);

let ev = eigenvector_centrality(&s, 100, 1e-9).unwrap();
assert!(!ev.converged);                  // agotó las 100 y sigue oscilando
assert!(ev.delta > 1e-9);

let pr = page_rank(&s, 0.85, 200, 1e-9).unwrap();
assert!(pr.converged);                   // MISMO grafo: converge
assert!((pr.total_mass() - 1.0).abs() < 1e-6);  // y la masa sigue ahí
```

## 24.6 Solución evolucionada

PageRank es el eigenvector con exactamente DOS arreglos quirúrgicos.

**Arreglo 1: el teleport (damping).** Con probabilidad 1−d, el surfer reaparece en una página del vector t. ¿Por qué repara la oscilación? Algebra lineal honesta, con nombre: **Perron-Frobenius**. La iteración de potencia converge al autovector dominante si todo lo demás es, en módulo, menor que 1. Un ciclo puro tiene autovalores complejos de módulo 1 (las raíces de la unidad: por eso rota sin decaer). La matriz de Google `G = d·M + (1−d)·t·1ᵀ` contrae TODOS los autovalores salvo el 1 por el factor d: con teleport uniforme, el segundo autovalor es exactamente d (Haveliwala y Kamvar, 2003, lo demostraron). Con d = 0.85, la masa de error se contrae un 15 % por iteración… pero se contrae SIEMPRE: la matriz es positiva (todas las entradas > 0, gracias al teleport que toca todo el mundo), luego primitiva, luego con un ÚNICO estacionario y convergencia garantizada — sin importar ciclos, colas ni componentes. Y por eso `damping ∈ (0,1)` ABIERTO por ambos extremos: d = 0 es teleport puro (una iteración, sin estructura — no hay grafo que rankear), y d = 1 es eigenvector PURO… que ya vimos fallar, y que por eso existe como función propia con sus tests de fallo. Los bordes no son valores válidos: son otros dos algoritmos. (Detalle técnico que ya nos costó un test: para excluir el 0 no sirve `Range::contains` — el inicio del rango es inclusivo. Comparación explícita.)

**Arreglo 2: redistribución uniforme de la masa colgante.** Un surfer en un nodo sin salidas teletransporta. ¿Por qué uniforme y no «se descarta y se renormaliza al final» (la variante «no-scale»)? Porque con redistribución la masa total es 1 EN CADA ITERACIÓN — un invariante que testeamos, no una esperanza — y el delta L1 conserva su lectura de probabilidad. La variante no-scale está documentada en el código como alternativa legítima: cambia el límite, no el procedimiento. Aquí no la implementamos: dos contratos semánticos para el mismo algoritmo es una trampa de lector.

El damping también paga otra deuda silenciosa: las **componentes desconectadas**. El teleport es lo único que conecta mundos que no se tocan — hace la cadena irreducible. Con teleport uniforme, cada componente recibe masa proporcional a su tamaño: dos 2-ciclos aislados se reparten 0.5 y 0.5; un 3-ciclo frente a un 2-ciclo, 3/5 y 2/5. Sin teleport no habría respuesta global posible: cada componente tendría su propio PageRank y ninguno sabría nada del otro.

Los EXTREMOS, medidos y no filosofados: con d = 0.01 el resultado es prácticamente el teleport uniforme (todos a 1/3 en la cadena con colgante, con margen 0.01); y a d fijo, subir d retrasa la convergencia — d = 0.99 necesita MÁS iteraciones que d = 0.5 para la misma tolerancia, porque la razón de contracción se acerca a 1 (`pagerank_damping_extremos` lo testea con las dos ejecuciones).

Sobre el núcleo compartido (`iteracion_de_potencia`), dos funciones públicas con una costura:

```rust
pub enum Teleport {
    Uniform,                          // PageRank global: 1/n
    Personalized(Vec<(NodeId, f64)>), // PPR: semillas ponderadas
}

pub fn page_rank(store, damping, max_iterations, tol)
    -> Result<PageRankResult, _>
{ /* mismo núcleo, t = Teleport::Uniform */ }

pub fn personalized_page_rank(store, seeds, damping, max_iterations, tol)
    -> Result<PageRankResult, _>
{ /* MISMO núcleo, t = Teleport::Personalized(seeds) */ }
```

¿Por qué el teleport es un parámetro y no está pegado al cálculo? Porque el PageRank personalizado no es una floritura: es un operador de RECUPERACIÓN. Su lectura cambia por completo: el global pregunta «¿qué es importante en general?»; el personalizado pregunta «¿qué es relevante PARA ESTE punto de partida?». Las semillas definen «el centro del mundo»: la masa que escapa por el damping vuelve a ELLAS. Con dos 2-ciclos desconectados y semilla en el nodo 0, la componente lejana queda a masa EXACTAMENTE 0 — fuera del mundo — y en la componente sembrada la solución a mano es `a = 0.15/(1−0.85²) ≈ 0.54`. El Vol. III (capítulo 51, GraphRAG) enchufará aquí su pregunta del usuario como `Teleport::Personalized` sobre el subgrafo de documentos. Si teleport y núcleo estuvieran acoplados, tendría que duplicar PageRank; tal como está, no toca el núcleo.

## 24.7 Código completo ejecutable

El código vive en `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs`. Los tests corren con `cargo test -p vol2-liradb tests_centralidad` (28, todos verdes). Recorremos las decisiones.

**Brandes, con números.** Betweenness(u) = Σ σ_st(u)/σ_st: la fracción de caminos mínimos entre pares ajenos que pasan por u. La versión ingenua corre un BFS POR PAR: V² pares × O(E) por BFS = O(V²·E)… y encima eso sólo da distancias — enumerar los caminos de cada par puede explotar exponencialmente (Brandes cita el estado previo como O(V³) y peor). Brandes 2001 lo reduce a V BFS con acumulación: en el BFS se cuentan los caminos (σ) y se anotan predecesores; después se recorren los nodos EN ORDEN INVERSO acumulando dependencias `delta[v] += (σ_v/σ_w)·(1 + delta[w])` — la recursión que evita enumerar caminos. Coste: O(V·E). Números: V = 1.000.000, E = 10.000.000 → por pares ~10¹⁹ operaciones (a 10⁹ op/s: siglos); Brandes ~10¹³ (del orden de horas). La normalización es la dirigida, 1/((n−1)(n−2)) — con `Both` sobre un grafo simetrizado reproduce EXACTO el libro no dirigido: camino 0-1-2-3 → intermedios a 2/3; estrella → centro a 1.0. Las aristas paralelas son caminos distintos (σ las cuenta).

**El bucle de PageRank.** Cae entero en `iteracion_de_potencia`: (1) sumar la masa de los colgantes; (2) sembrar el vector nuevo con teleport + cuota colgante; (3) repartir votos de 1/grado por cada arista saliente; (4) delta L1, history, y parar bajo tolerancia. Multigrafo, sutileza testeada: duplicar la arista 0→2 NO duplica el voto de 2 (también duplica el denominador)… pero le ROBA la mitad de la cuota al otro vecino: 2 sube, 1 baja (`pagerank_multigrafo_el_duplicado_roba_masa`).

Y cuando el grafo es simétrico, el sistema se resuelve A MANO — el mejor test es el que puedes verificar con lápiz. A↔B y A↔C (A reparte entre B y C; ambos le devuelven todo): por simetría x_B = x_C = y, con dos ecuaciones `x = (1−d)/3 + d·2y` e `y = (1−d)/3 + d·x/2`. Con d = 0.85: `y = (0.05 + 0.425)/1.85 ≈ 0.257` y `x = 1 − 2y ≈ 0.487`. El test `pagerank_ciclos_compartidos_solucion_a_mano` ejecuta y compara — EPS 1e-6.

**Por qué L1 y no max-delta.** El delta L1 (Σ|Δscore|) es la MASA TOTAL que se movió en la iteración: «¿cuánta masa falta por asentar?» — un número con lectura de probabilidad, y comparable entre grafos de distinto tamaño (1e-6 de masa es 1e-6 de masa con 6 nodos o con 6 millones). El max-delta (el cambio del nodo que más se movió) es más estricto por nodo, pero no significa nada como masa y su umbral cambia de sentido con el tamaño del grafo. Documentado y descartado.

**Por qué el history es contenido.** `PageRankResult.history` guarda el delta de CADA iteración. Ejecuta la cadena 0→1→2 (colgante al final) y míralo — esto es la salida real, no un esquema:

```text
$ page_rank(0→1→2, d=0.85, tol=1e-10)  →  converged=true, 33 iteraciones
history: 3.778e-1  2.676e-1  1.668e-1  4.726e-2  2.800e-2  1.931e-2
         7.134e-3  2.630e-3  2.063e-3  1.022e-3  2.897e-4  2.229e-4
         ... (monótono a la baja) ...
         3.670e-9  2.146e-9  1.492e-9  5.629e-10 1.998e-10 1.584e-10 8.006e-11
```

Tres lecturas en esos números. Primera: decrece MONÓTONO desde la segunda iteración (la primera mezcla el arranque) — el test lo exige término a término. Segunda: la razón entre consecutivos no es una constante limpia — oscila (el colgante devuelve la masa a saltos) — pero se mantiene acotada por debajo de 1: la contracción geométrica de Perron-Frobenius trabajando, con la huella del colgante encima. Tercera: el contraste es todavía más elocuente. En dos 2-ciclos simetrizados el teleport uniforme YA es el estacionario: converge en UNA iteración y `history = [0.0]`. El historial cuenta ambas historias — y en ningún caso tienes que creerte nada: ejecutas y ves.

**El coste, medido.** `CentralidadStats { bfs_runs, edges_scanned, iterations }`: closeness y betweenness reportan un BFS por nodo; PageRank acumula `iteraciones × E`. El test del camino lineal comprueba `edges_scanned` EXACTO (12 de proyección + 24 de BFS). La tabla del guion queda verificable:

| Familia | Pregunta | Coste | Stats |
|---|---|---|---|
| Grado | ¿cuántos vecinos? | O(V+E) | — |
| Closeness | ¿lo lejos que estoy de todos? | O(V·(V+E)) | bfs_runs = V |
| Betweenness | ¿por cuántos caminos paso? | O(V·E) | bfs_runs = V |
| Eigenvector | ¿quién me apunta? | O(iter·E) | iterations |
| PageRank | ¿dónde pasa la eternidad el surfer? | O(iter·E) | iterations |

**Cómo leer un resultado.** Los tres tipos devuelven más que números: `score(id)` (que responde `None` para ids inexistentes o borrados), `entries()` en orden de id, `ranking()` por score descendente con desempate por id (determinismo: dos ejecuciones dan el mismo ranking, siempre), y en PageRank la transparencia del cálculo: `converged`, `delta`, `history`, `damping` y `Display` — `PageRank(d=0.85, iteraciones=33, delta=8.01e-11, convergido=sí)`. La validación de entrada falla ruidosamente ANTES de iterar: `InvalidDamping` (con los bordes explicados en el propio mensaje de error), `InvalidTolerance`, `InvalidMaxIterations`, y para el teleport personalizado, `NegativeTeleportWeight` señalando el nodo, `ZeroTeleportMass` y `UnknownNode`. Una función que va a correr 500 iteraciones no puede descubrir en la 499 que la semilla no existía.

## 24.8 Prueba de fuego

El test que ES el capítulo, `eigenvector_no_converge_en_periodico_y_pagerank_si`: el MISMO grafo cola+3-ciclo donde eigenvector agota 100 iteraciones sin converger, `page_rank(&s, 0.85, 200, 1e-9)` converge con masa 1. Los DOS fallos del §24.5, con su cura al lado.

Después, el grafo demo — y su sorpresa pedagógica. KNOWS forma el triángulo 0→1→2→0 más el self-loop de Dani (3→3); LIVES_IN lleva de 0 y 1 a las ciudades 4 y 5. ¿Quién gana? **Dani, con 0.386** — por delante de todo el triángulo. Nosotros también pensamos que era un bug. No lo era: el self-loop le devuelve CADA voto (es su único vecino saliente), y encima cobra la cuota uniforme de la masa colgante de las ciudades, que no votan. Self-loop + masa colgante = trampa de acumulación. Las salidas reales, lado a lado:

```text
PageRank (d=0.85, 92 iteraciones):   Grado OUT (una pasada):
  n3 (Dani)   = 0.386                  n0 = 0.40   ← Ana
  n0 (Ana)    = 0.151                  n1 = 0.40   ← Bo
  n1 (Bo)     = 0.122                  n2 = 0.20
  n4 (Madrid) = 0.122                  n3 = 0.20   ← Dani, "casi marginal"
  n2 (Carla)  = 0.110                  n4 = 0.00
  n5 (Sevilla)= 0.110                  n5 = 0.00
```

Por GRADO, Dani es casi el más marginal (1/5 — sólo supera a las ciudades, que no votan); por PageRank, arrasa. Dos métricas, dos historias — y ninguna de las dos «la verdad». Fíjate también en los empates estructurales: n1 con n4, n2 con n5 — la simetría rotacional del triángulo arrastra a cada persona a empatar con la ciudad que alimenta. El ranking desempata por id (determinismo), pero el empate es información del grafo, no ruido.

Los invariantes, en todos los tests: `total_mass() = 1` (desviaciones de ~1e-15: redondeo de f64); damping fuera de (0,1) — incluido NaN, que no compara bajo `PartialEq` y por eso se testea con `matches!` — rechazado ruidosamente; semillas negativas, de masa cero o inexistentes rechazadas señalando el nodo culpable. Y `converged = false` es una RESPUESTA: una base de datos prefiere decir «no convergió» que devolver números casi-buenos en silencio.

Dos cierres de análisis sobre el mismo demo. Primero: personaliza en Madrid (semilla = nodo 4) y el grafo se RE-CENTRA — `ppr_vs_global_en_demo_graph`, salida real:

```text
   global           PPR semilla=Madrid(4)
   n3 = 0.386       n3 = 0.328   ← Dani se desinfla: pierde el teleport uniforme
   n4 = 0.122       n4 = 0.254   ← Madrid se duplica: el teleport vuelve a casa
   n0 = 0.151       n0 = 0.128   ← hasta Ana BAJA: fuera de órbita, fuera de masa
```

La ciudad sube, la trampa de Dani se desinfla al dejar de cobrar teleport uniforme, y hasta el nodo 0 (que la apunta) baja — quien no está en la órbita de la semilla pierde teleport sin ganar nada equivalente. Mismo grafo, mismo algoritmo, otro mundo — esa es la potencia del vector de teleport. Segundo: borra el nodo central de una cadena (`delete_node`) y el índice denso de la proyección excluye el hueco: los ids restantes siguen puntuando, el borrado responde `None` — no existe, no puntúa (`pagerank_huecos_tras_delete_node`). La proyección deriva del store; no lleva el grafo «en la cabeza».

## 24.9 Qué hemos sacrificado

1. **La proyección no ponderada.** El closeness ponderado (Dijkstra del capítulo 22 por cada origen) queda como deuda hacia el capítulo 26: cuando exista la proyección con pesos, el BFS de este capítulo se sustituye sin tocar nada más. Deuda declarada, no olvidada.
2. **Nada de optimización industrial.** Ni PageRank por bloques, ni betweenness aproximada por muestreo, ni GPU. El guion manda: familias para explicar, coste para medir.
3. **La variante no-scale de dangling.** Documentada, no implementada: dos contratos semánticos para un algoritmo es una trampa.
4. **Arranque asistido, acoplado.** El arranque es el propio teleport (uniforme o semillas): ahorra iteraciones de mezclado y es una línea — los arranques «inteligentes» (scores del grado) ganarían poco y oscurecerían el análisis.
5. **Empates no desempatados por significado.** El ranking desempata por id ascendente (determinismo); en el demo, 1 empata con 4 y 2 con 5 por simetría estructural — el orden entre iguales es convención.

## 24.10 Cómo lo hace una BBDD real

- **Neo4j GDS** expone `gds.pageRank` con la MISMA parametrización de siempre y los mismos números de hace 27 años: `dampingFactor` 0.85, `maxIterations` 20, tolerancia 1e-7. Y el PageRank personalizado NO es otra función: es el parámetro `sourceNodes`, que acepta pares (nodo, sesgo) — traducción literal de nuestro `Personalized(Vec<(NodeId, f64)>)`. Mismo núcleo, otro mundo: exactamente la costura de nuestro `Teleport`.
- **`gds.betweenness`** implementa Brandes… y ofrece una variante por MUESTREO para grafos grandes, porque hasta O(V·E) se queda corto en producción. Nuestro `CentralidadStats` mide lo que su muestreo recorta.
- **PPR como base de recomendación** es patrón clásico de la industria: el «Who to Follow» de Twitter se construyó sobre random walks con reinicio en el grafo de follows, y la personalización de Google News (2007) rankeó noticias con PageRank sobre el grafo usuario-noticia. La receta es siempre la misma: semilla = usuario/consulta, ranking = dónde pasa el surfer su tiempo.
- **Google**, por supuesto, ya no usa «el» PageRank de 1998: cientos de señales encima. Pero la decisión de diseño — medir la importancia como proceso global sobre el grafo de enlaces, no como recuento local — es la que ordenó la web.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en la cadena 0→1→2, ¿por qué el colgante 2 NO absorbe toda la masa si recibe el voto de 1 y no emite nada?
- *Intermedio*: ¿cómo cambiaría el history de un PageRank con d = 0.5 frente a d = 0.95 en el MISMO grafo? Predícelo y mídelo.
- *Experto*: ¿qué teleport reproduce la centralidad de grado como caso degenerado de PPR, y con qué d? (Pista: piensa qué pasa cuando d→0 y las semillas son TODOS los nodos con peso 1/n… y por qué aún así no es el grado.)

## 24.11 Lo que te llevas

- «Importante» tiene cinco respuestas con cinco costes: grado O(V+E), closeness O(V·(V+E)), betweenness O(V·E) (Brandes, no por pares), eigenvector/PageRank O(iter·E). Mídelo con `CentralidadStats`.
- El eigenvector crudo falla DOS veces: masa que se fuga por colgantes y oscilación periódica. Ambos fallos son tests.
- PageRank = eigenvector + damping (teleport) + redistribución colgante. La matriz se vuelve positiva ⇒ primitiva ⇒ converge SIEMPRE, geométricamente, razón ≈ d·λ₂ — y el `history` te lo enseña.
- Masa = 1 en cada iteración es un invariante testeado; el delta L1 es «masa que se mueve» — interpretable y comparable entre tamaños.
- `Teleport{Uniform, Personalized}` separa el MUNDO del NÚCLEO: el mismo código rankea la web entera o la órbita de una consulta. El capítulo 51 del Vol. III vive en esa costura.

## 24.12 Ojo, cuidado con…

- **Confundir damping con tuning**: no es velocidad, es reparación. d→1 converge MÁS lento (la razón se acerca a 1); d→0 devuelve el teleport. 0.85 es el equilibrio del paper, no una manía.
- **Olvidar la masa colgante**: quien calcula «a mano» con solo teleport+votos obtiene masa < 1 y no lo nota. `total_mass()` existe; úsalo.
- **Both no es out+in apilados**: es unión como CONJUNTO (vecinos distintos, self-loop UNA vez); `In` es la transpuesta pura. Mezclar ambas cosas fue un bug real de esta implementación.
- **Duplicar aristas esperando duplicar votos**: el duplicado roba masa a los OTROS vecinos del mismo origen. Semántica multigrafo: sutil, testeada.
- **Eigenvector en grafos dirigidos**: si necesitas el autovector de verdad (espectral), el Vol. I cap. 16 lo trata; aquí su papel es ser el «antes» honesto del PageRank.

## 24.13 Pin de batalla

> *«Sin damping, la importancia se pierde en los callejones o se queda dando vueltas para siempre. El teleport no es un parámetro: es decidir a dónde vuelve toda masa que se extravía.»*

## 24.14 Si solo lees 30 segundos

La centralidad pregunta «¿quién es importante?». El grado cuenta vecinos; el closeness mide distancias; el betweenness mide atascos (Brandes: V BFS en vez de todos los pares); el eigenvector propaga importancia… y se rompe en grafos dirigidos: la masa fuga por los colgantes y oscila en los ciclos. PageRank lo repara con dos arreglos — teleport con probabilidad 1−d (la matriz se vuelve primitiva: converge siempre, a razón ≈ d·λ₂) y redistribución uniforme de la masa colgante (masa total 1 en cada iteración). El teleport personalizado cambia el centro del mundo: semillas = consulta, ranking = recuperación. Esa es la pieza que GraphRAG heredará.

## 24.15 Una historia pequeña

La primera vez que corrimos PageRank sobre el grafo demo, Dani ganó con 0.386. Presentamos el resultado como «seguramente un bug»: nadie enlaza a Dani salvo él mismo. Pasamos la tarde buscando el error — reparticiones mal sumadas, teleport mal normalizado, el self-loop contado dos veces — hasta que hicimos lo que había que hacer desde el principio: resolver el sistema a mano. El número era 0.386 exacto: el self-loop devuelve cada voto y las ciudades le pagan la cuota colgante. El algoritmo estaba bien; MALA era nuestra intuición sobre «quién es importante». Esa tarde aprendimos la regla que atraviesa el capítulo: cuando un ranking sorprende, primero se hace la aritmética — el surfer nunca se equivoca sobre dónde pasa su tiempo; nosotros sí sobre dónde debería.

## Ejercicios resueltos

**1. En la cadena 0→1→2 (2 es colgante), calcula a mano la PRIMERA iteración desde el arranque uniforme y comprueba la masa.**

Arranque: x = [1/3, 1/3, 1/3]. Teleport uniforme t = 1/3, d = 0.85. Masa colgante D = x[2] = 1/3. Cuota colgante: d·D/n = 0.85·(1/3)/3 ≈ 0.0944 por nodo. Base por nodo: (1−d)·t + cuota = 0.05 + 0.0944 = 0.1444. Votos: 0→1 aporta 0.85·(1/3) ≈ 0.2833 a 1; 1→2 aporta 0.2833 a 2. Resultado: y = [0.1444, 0.4278, 0.4278], suma = 1.0000 exacto. La masa se conserva DESDE la primera iteración — el invariante no espera al límite. (El límite, para curiosos: 0.184, 0.341, 0.474 — el colgante acumula, pero la redistribución le impide absorberlo todo.)

**2. ¿Por qué la corrección de Wasserman-Faust da 1/3 a TODOS los nodos de dos 2-ciclos desconectados, y Freeman puro daría 1?**

Cada nodo alcanza a 1 de los 3 posibles, con Σd = 1. Freeman puro aplicado a lo alcanzable daría (r−1)/Σd = 1/1 = 1: la forma ingenua compara «mi mundo alcanzado» consigo mismo y premia la burbuja. Wasserman-Faust multiplica por la fracción alcanzada: C = ((r−1)/(n−1))·((r−1)/Σd) = (1/3)·(1/1) = 1/3: penalizado por los 2 nodos que NO alcanza. Verificación: `closeness_componentes_desconectadas_wasserman_faust`.

## Ejercicios propuestos

**Esencial (recordar — retrieval puro).** Con el libro CERRADO: (a) los DOS fallos del eigenvector crudo, con el grafo mínimo de cada uno; (b) los DOS arreglos de PageRank y qué garantiza cada uno; (c) qué mide el delta L1 y por qué la razón entre deltas consecutivos tiende a d·λ₂. Después ejecuta `cargo test -p vol2-liradb tests_centralidad` y localiza cada afirmación en un test POR NOMBRE. *Pistas*: (1) ¿qué le pasa al surfer en una página sin enlaces? (2) ¿puede una masa rotar en un ciclo para siempre? (3) ¿qué matriz converge siempre: la positiva o la cruda? *Criterio*: cuatro tests citados correctamente sin mirar.

**Intermedio (analizar — spacing con el cap. 14).** Explica en dos frases por qué la `Proyeccion` es «el CSR del capítulo 14 en memoria» y qué le falta para SERLO (offsets+targets compactos vs `Vec<Vec<usize>>`, pesos, ids de arista). Luego PREDICE, antes de ejecutar, `bfs_runs` y `edges_scanned` de `betweenness_centrality` sobre el camino simetrizado de 6 nodos (fórmula V·E menos la deduplicación de Both) y verifícalo con un miniprograma que lea `CentralidadStats`. *Pistas*: (1) ¿cuántos BFS corre Brandes? (2) ¿cuántas entradas de vecindario pisa cada BFS en un camino? (3) ¿qué cuenta `edges_scanned` de la proyección además del BFS? *Criterio*: predicción exacta y el paralelo CSR bien dicho.

**Experto (crear — interleaving caps. 7-8-22-26 + costura Vol. III).** Construye el mini-operador de recuperación que el capítulo 51 necesitará: `ppr_por_etiqueta(store, etiqueta, damping) -> Result<PageRankResult>` que siembre `personalized_page_rank` con TODOS los nodos de esa etiqueta a peso uniforme (recorriendo `iter_nodes` del trait, cap. 8). Verifica: masa = 1; los nodos de otra etiqueta SIN camino desde la semilla quedan a ~0 (¿por qué en el demo las ciudades NO quedan a 0 exacto?); el ranking difiere del global. Extensión conceptual sin código: ¿qué habría que cambiar para que los enlaces PESen por la propiedad `since`? (Respuesta esperada: nada aquí — es la `ProyeccionPonderada` del cap. 26 leyendo pesos con la semántica estricta `edge_weight` del cap. 22; el núcleo de potencia no se toca.) *Pistas*: (1) ¿quién valida las semillas: tu función o `densificar`? (2) ¿por dónde entra el peso en `iteracion_de_potencia`? (3) ¿por qué tu función no debe duplicar el bucle? *Criterio*: test propio verde, núcleo intacto.

## Para profundizar

- **L. Page, S. Brin, R. Motwani y T. Winograd, «The PageRank Citation Ranking: Bringing Order to the Web» (Stanford Digital Library, 1998/1999; presentado en WWW7, Brisbane, 1998)** — el paper del capítulo: el surfer aleatorio, el damping 0.85, la web como grafo de citas.
- **S. Brin y L. Page, «The Anatomy of a Large-Scale Hypertextual Web Search Engine» (WWW7, 1998)** — el paper de Google como sistema, con PageRank como pieza.
- **U. Brandes, «A Faster Algorithm for Betweenness Centrality» (Journal of Mathematical Sociology 25(2), 2001)** — la reducción de todos-los-pares a V BFS con dependencias hacia atrás.
- **S. Wasserman y K. Faust, «Social Network Analysis: Methods and Applications» (Cambridge Univ. Press, 1994)** — la corrección de cercanía en componentes desconectadas.
- **A. Langville y C. Meyer, «Google's PageRank and Beyond» (Princeton Univ. Press, 2006)** y **T. Haveliwala y S. Kamvar, «The Second Eigenvalue of the Google Matrix» (Stanford, 2003)** — por qué converge y a qué velocidad: λ₂ = d.
- **Neo4j Graph Data Science — `gds.pageRank` (con `sourceNodes`) y `gds.betweenness`** — las mismas decisiones de diseño, en producción.
- **Patente US 6,285,999** («Method for node ranking in a linked database», 1998-2018) e **IEEE Milestone «PageRank and the Birth of Google, 1996–1998»** (ethw.org) — la historia, documentada.

## Mini-diálogo: en la cima del ranking

> — Entonces el damping es… ¿el parámetro que hace que el bucle acabe?
>
> — Es el parámetro que hace que el bucle PUEDA acabar: contrae todos los autovalores salvo el 1. Sin él, un miserable ciclo de tres nodos te deja oscilando para siempre — lo hemos ejecutado, no es retórica.
>
> — ¿Y el 0.85 es sagrado?
>
> — Es el del paper y el de Neo4j. Bájalo y rankeas el teleport; súbelo y tardas más en asentar. El número importa menos que el intervalo: (0,1) ABIERTO, porque los bordes son otros dos algoritmos degenerados.
>
> — Y lo personalizado… ¿eso no es hacer trampa con el resultado?
>
> — Es cambiar la pregunta. El global pregunta qué importa en general; el personalizado pregunta qué importa DESDE ti. La web entera o tu órbita: mismo surfer, distinto mapa. Recuérdalo — en el Vol. III, esa diferencia será un buscador.

---

*(Próximo capítulo: 25 — Comunidades y agrupaciones. PageRank dice quién es importante; aún no sabe decir en qué GRUPOS se organiza el grafo. Llega Louvain — y con él una sorpresa: no podrá reutilizar la proyección de este capítulo tal cual. Te avisamos por algo.)
