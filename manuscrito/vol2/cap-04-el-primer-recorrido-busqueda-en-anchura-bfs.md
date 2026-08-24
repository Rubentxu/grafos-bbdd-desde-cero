# Capítulo 4 — El primer recorrido: búsqueda en anchura (BFS)

> *«"Existe un camino" es la pregunta más barata que una base de datos de grafos puede responder bien — y la más cara que responde mal.»*

> **Convenio de volumen (Apéndice 0 §0.6)**: la teoría algorítmica de BFS ya se publicó en el **Vol. I**. Este capítulo del **Vol. II** no la re-explica: la **reorienta**. Aquí BFS no es un ejercicio de pizarra; es **la operación fundamental que el motor de una base de datos de grafos debe soportar**, y el cimiento de la cercanía (`closeness`, cap. 24) y del "menor número de saltos" que responde toda consulta de alcanzabilidad. Lo que aquí es una **idea**, `liradb-workspace` lo ejecuta por **fábrica** en los caps. 24 y 26. Al que ya "se sabe" BFS este capítulo le da el *para qué de un motor*; al novato le da el modelo mental correcto antes de tocar una sola línea del trait `GraphStore` (cap. 8).

## 4.0 La anécdota de la esquina

En 1967, el sociólogo **Stanley Milgram** envió 296 cartas por Estados Unidos con una única regla: pasar el sobre a un desconocido de Boston **solo a través de alguien conocido por su nombre de pila**. Nadie esperaba el resultado: la mayoría llegó en **menos de seis entregas**. De esa cadena nació la idea de los **seis grados de separación** — y, décadas después, la pregunta que Facebook y LinkedIn convirtieron en negocio: *"¿a cuántos saltos está Ana de Zoe?"*.

Años antes, en los años 50, la cosa era más modesta y más concreta. Ingenieros como **Edward F. Moore** se preguntaban cómo un robot podía salir de un **laberinto** sin dar vueltas inútiles. Y en 1959, **Edsger Dijkstra** publicó un algoritmo para el trayecto más corto en redes con distancias reales. La clave de todos era la misma: **no te lances a explorar hasta el fondo; déjate avanzar en olas, nivel a nivel, desde el origen.** Ese "avanzar en olas" es la **búsqueda en anchura** (Breadth-First Search, BFS). Y aunque nació en los laberintos y las redes sociales, las bases de datos de grafos modernas la han convertido en el corazón de su motor. Este capítulo te explica esa ola.

## 4.1 Objetivo

Al terminar este capítulo sabrás **por qué un motor de grafos responde "¿existe un camino entre A y B?" sin pestañear**, y entenderás el mecanismo exacto: el recorrido **por niveles** con su **cola FIFO** y su regla de **visita**. Tres ideas:

1. La **frontera** — los nodos a exactamente *k* saltos del origen, la "ola" que se expande.
2. La **cola FIFO** — la estructura que encola los nodos para que la expansión respete el orden de los niveles.
3. La **complejidad O(V+E)** — por qué cuesta tan poco y por qué una marca de visita es lo que hace que termine.

Y un rumbo: este BFS es el mismo que la **centralidad de cercanía** del cap. 24 (`closeness_centrality`) y el **BFS por fronteras / streaming** del cap. 26 (`bfs_fronteras`, `bfs_streaming`) ejecutan sobre el trait `GraphStore` del cap. 8. Aquí lo ves como idea; allí lo ejecutas como corazón del motor.

## 4.2 Problema

Imagina que LiraDB (todavía conceptual — el modelo del cap. 7 y la API del cap. 8 están por venir) recibe una consulta cotidiana:

> **"¿Está Zoe conectada con Ana, y a cuántos saltos?"**

En una red social real, Ana y Zoe no se conocen, pero comparten una amiga, y esa amiga conoce a Zoe. La respuesta no es un simple "No"/"Sí": es "sí, a **2** saltos". El "a cuántos" es tan importante como el "Sí": es la semilla de la **cercanía**, del **camino más corto**, de "amigos de amigos".

La trampa: una BBDD **relacional** (SQL) responde "amigos de amigos" con un **join** — emparejar usuarios y volver a emparejar. A 2 saltos ya es un producto cruzado costoso; a 6 es una pesadilla combinatoria (los "seis grados" de Milgram). La pregunta en esencia es **recorrer el grafo desde Ana**: visitar sus amigos, luego los amigos de sus amigos, y ver cuándo aparece Zoe. Ese recorrido es **BFS**, y es lo que un motor de grafos hace **nativamente**, sin joins, en tiempo que no explota. El reto, pues, no es "si" se puede recorrer — es **recorrer con orden y conteo**: garantizar el **camino más corto** y saber a qué nivel.

## 4.3 Modelo mental

Deja caer una **piedra en un estanque**:

- La piedra es el **origen** (Ana).
- Las **olas** que se expanden son los **niveles**: ola 1 = amigos de Ana
  (nivel 1), ola 2 = amigos de esos amigos (nivel 2), etc.
- La **frontera** es la **ola actual**: nodos a exactamente *k* saltos.
- **Cada nodo se descubre una sola vez**, cuando la ola lo toca por
  primera vez — y *esa* es, por construcción, su distancia mínima.

La **cola FIFO** es el "muelle" donde se amontonan los nodos de la ola
actual para sacarlos en el orden en que llegaron (= orden de nivel).

```
            ·            ← nivel 2 (ola lejana)
         ·   ·
       ·   F   ·         ← nivel 1 (la FRONTERA = ola media)
       ·  [·]  ·             [·] = origen (nivel 0)
       ·   ·   ·         ← · = nodos ya descubiertos
         ·   ·
            ·
  ONDA: nivel 0:[origen]  nivel 1:[v1,v2]  nivel 2:[v3,v4,v5,v6]
  Cola FIFO: origen → v1,v2 → v3,v4,v5,v6 → ...
  Regla: descubro un nodo UNA vez, y eso fija su nivel mínimo.
```

**La distancia sale sola.** Como los niveles se expanden en orden,
cuando la ola toca a Zoe en el nivel 2 ninguna ola anterior pudo
alcanzarla por menos: si hubiera un camino de 1 salto, la ola del nivel
1 la habría tocado antes. Descubrir en *k* **es** estar a *k* saltos —
la estructura de olas lo **garantiza**. El que avanza en olas no se
pierde en el laberinto — barre por distancia y nunca vuelve atrás en
círculos, porque ya marcó lo visitado.

**El momento ¡ajá!**: *descubrir un nodo en el nivel k es, por
construcción, estar a k saltos del origen.* No hace falta comparar
rutas: la ola ya trae la respuesta.

## 4.4 Primera solución

El recorrido más natural de un novato: **"recorrer por orden de aparición"** — miras los vecinos según van saliendo, sin cola explícita ni marca de nivel:

```
recorrido ingenuo(nodo origen):
    vistos = conjunto vacío
    lista  = [origen]
    mientras haya por expandir:
        v = siguiente nodo de lista
        si v no está en vistos:
            añade v a vistos
            para cada vecino w de v:
                añade w a lista          # "a ver cuándo llego"
    devuelve lista
```

Funciona para la existencia: acabas visitando *todos* los nodos alcanzables desde Ana. Los tests mentales pasan. Zoe aparece en la lista — en algún lugar. Un novato razonable diría "listo, la encontré".

## 4.5 Sus límites

Y entonces llega la pregunta incómoda:

> **"Ya la encontré. Pero... ¿a cuántos saltos está Zoe de Ana?"**

La respuesta del recorrido ingenuo es basura: te devuelve la **lista completa sin orden por distancia**. No sabes si Zoe está a 1 salto o a 5; el orden depende del accidente del layout, no de la distancia.

En un grafo **con ciclos** (casi siempre los hay), el `si v no está en vistos` salva *parcialmente* al recorrido, pero no le da **cuántos** saltos: no se guarda el nivel. Lo que esta versión rompe exactamente:

- **"¿a qué distancia?"** → no hay niveles;
- **"¿quién está en el nivel 2?"** → lista sin capas;
- **"¿amigos de amigos en ≤ k saltos?"** → imposible sin la profundidad.

Necesitamos la **evolución**: cola FIFO + marca de nivel.

## 4.6 Solución evolucionada

La solución tiene dos piezas, y cada una arregla exactamente un límite.

**Pieza 1 — La cola FIFO.** En vez de "siguiente nodo de la lista" a lo bruto, sacamos _el más antiguo_ (primero en entrar, primero en salir). Mételos por el final y sácalos por el principio. Esto fuerza que **se acaben de expandir todos los del nivel *k* antes de tocar los del *k+1***, porque los del nivel *k* entraron antes. Esa propiedad convierte "recorrer" en "recorrer **por niveles**".

**Pieza 2 — La distancia por nivel.** Cada nodo, al ser descubierto, recibe su nivel: `distancia[vecino] = distancia[padre] + 1`. Como el descubrimiento es por olas, esa distancia es la **mínima**. Alternativamente (más compacto) una "frontera" como lista por nivel — `niveles[0] = [Ana]`, `niveles[1] = amigos`, `niveles[2] = amigos de amigos`, etc. — exactamente lo que verás como `RecorridoBfs { niveles: Vec<Vec<NodeId>> }` en el cap. 26.

**Regla de visita — el secreto de que termine.** Marcamos cada nodo como visitado **en el momento de descubrirlo (al encolarlo), no al sacarlo**. Si lo marcaras al sacar, el mismo nodo del nivel *k* entraría a la cola *k* veces (una por cada vecino que lo descubra), rompiendo niveles y duplicando trabajo. Al marcar "al descubrir", cada nodo entra a la cola exactamente una vez, sus vecinos se expanden una sola vez, y el algoritmo **termina** en O(V+E) aun con ciclos.

```
BFS por niveles (frontera a frontera):
    dist[Ana] = 0 ; encola(Ana) , marca(Ana)
    frontera_actual = [Ana]
    mientras frontera_actual no vacía:
        siguiente = []
        para cada v en frontera_actual:
            para cada vecino w de v:
                si w NO está marcado:
                    dist[w] = dist[v] + 1
                    marca(w); siguiente.push(w)
        frontera_actual = siguiente
    # niveles[0]=[Ana], niveles[1]=amigos, ..., dist[k] = nivel
```

Observa la belleza de la variante "por fronteras": no necesitas pila explícita ni descolar; la lista `siguiente` **es** la siguiente ola. Esa misma separación por olas es la que el cap. 26 explota para leer **solo** las adyacencias de la frontera actual (streaming), sin tocar el grafo entero.

## 4.7 Código completo ejecutable

Este capítulo es **conceptual**: no construye aún el motor (eso empieza con el `GraphStore` del cap. 8). Pero el BFS de este capítulo **sí está escrito en el workspace**, en los caps. que lo ejecutan por fábrica. Ancla los nombres para que, al llegarlos, reconozcas el mismo algoritmo:

**En `liradb-workspace/crates/vol2-liradb/src/cap26_proyeccion.rs`** (el BFS por niveles sobre el store del cap. 26):

- `RecorridoBfs { niveles: Vec<Vec<NodeId>> }` con la doc: *"Frontera a
  frontera: `niveles[k]` = nodos descubiertos a k saltos"* — justo la
  `frontera_actual` / `siguiente` de §4.6.
- `bfs_fronteras` / `bfs_streaming` — el iterador **perezoso** que pide
  una frontera, produce la siguiente y lee del store bajo demanda. Ese
  "99,9% de lo leído no se va a usar" que evita el cap. 26 es,
  literalmente, la ola: no expandimos el estanque entero, solo la
  frontera.
- `visitado: BitSet` — la marca de visita de §4.6 hecha bitset.
- `StreamStats { nodos_visitados, aristas_leidas, ... }` — la prueba
  medible de que O(V+E) no es un eslogan: cuenta cada lectura.

**En `.../src/cap24_centralidad.rs`**: `closeness_centrality` (Freeman
con corrección **Wasserman-Faust** para componentes desconectadas) hace
*"un BFS por nivel sobre la proyección NO ponderada (distancias =
saltos)"* — por cada origen, un BFS como el tuyo, y suma las distancias.
Su `O(V·(V+E))` anotado es "V veces nuestro O(V+E)".

En ambos, el patrón es el de este capítulo: **cola / fronteras + visita
+ nivel**. No los caces línea por línea todavía; basta con que, cuando
los veas, reconozcas la ola que aprendiste aquí.

## 4.8 Prueba de fuego

La prueba de fuego es **conceptual y cruzada**:

1. **A mano**: dibuja un grafo pequeño, ejecuta BFS con papel y lápiz (el ejercicio `recordar` del final), y comprueba que tu orden de descubrimiento por niveles coincide con la cola FIFO "de libro". Es la misma verificación que hará el cap. 26 al comparar contra sus `niveles` ordenados por id (determinismo).
2. **En el workspace**: reconoce tu algoritmo en `closeness_centrality(&s, GraphDirection::Both)` del cap. 24 — para el camino `0-1-2-3` devuelve `0.75` para el centro y `0.5` para los extremos: justo lo que predice "el centro está a menos saltos de todos". Y en `bfs_fronteras(&s, 0, GraphDirection::Out, Presupuesto::profundidad(1))` → `Some(vec![0])`: la frontera 0 es el origen, nada más se lee.

**¿Qué fallaría si te saltaras este capítulo?** Frente al código del cap. 26, no sabrías qué es `niveles[k]` ni por qué la frontera se ordena por id (determinismo); frente al cap. 24, la centralidad te olería a "suma mágica" en lugar de a "un BFS por nodo". El síntoma detectable: un muro de `Vec<NodeId>` y `u32` sin el ojo que ve la ola de Ana.

## 4.9 Qué hemos sacrificado

Tres concesiones:

1. **No soporta pesos de arista.** Cada arista "vale 1 salto". Si tu red son carreteras con kilómetros o vuelos con precio, "2 saltos" no es "2 km". Ese es el territorio de **Dijkstra** (cap. 22); BFS es el caso con pesos 1.
2. **Guarda todo lo visitado.** Para no repetir, lleva un conjunto de visitados y, en la variante de cola, la distancia. En grafos enormes eso es memoria; el cap. 26 lo vuelve perezoso (solo retiene la frontera) con sus presupuestos y un `BitSet`.
3. **No te dice el camino completo.** BFS ordena descubrimientos pero no reconstruye él solito la ruta exacta (eso requiere guardar el predecesor). Para el *menor número de saltos* basta la distancia.

## 4.10 Cómo lo hace una BBDD real

En un GDBMS, BFS **es** la unidad de consulta. "¿Existe un camino entre A y B?" (y "¿a cuántos saltos?") es LA pregunta que justifica existir a una base de datos de grafos frente a un SGBD relacional: donde SQL paga un self-join producto-cruzado (O(n²) o peor en tablas grandes), el motor de grafos hace **un BFS en O(V+E)** sobre sus adyacencias compactas (CSR, cap. 14). El tipo de almacenamiento cambia el *coste de la lectura de vecinos*, no la estructura de olas — eso responde la pregunta crítica de `CORPUS.yml`: *BFS sobre CSR vs sobre HashMap de listas* es **el mismo BFS**; cambia `adyacencia` (contigüidad y caché en CSR, dispersión por llaves en un hash), **no** el orden de niveles.

Herramientas maduras:

- **Neo4j**: `MATCH (a)-[*1..2]-(z) RETURN count(*)` va por caminos de a
  lo más 2 saltos; a veces guía búsquedas bidireccionales por anchura.
- **TigerGraph / Memgraph**: BFS y "menor número de saltos" son funciones
  de primer nivel de su lenguaje de consulta.
- **Facebook/Instagram**: "¿quién está a 1/2/3 saltos?" alimenta motores
  de grafos que recorren en anchura el grafo social bajo demanda.

El BFS que aquí es una ola es, allí, una consulta de milisegundos.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: por qué `bfs_fronteras(&s, 0, GraphDirection::Out,
  Presupuesto::profundidad(1))` no lee más allá de la frontera 0 (cap. 26).
- *Intermedio*: si usaras **pila** (DFS) en vez de cola, ¿seguiría
  garantizando "2 saltos"? ¿por qué?
- *Experto*: diseña en palabras un BFS **bidireccional** (ola desde Ana
  y desde Zoe a la vez) y justifica por qué puede ser más barato en
  grafos enormes que un BFS desde un solo lado.

## 4.11 Lo que te llevas

- **La frontera es la ola actual**: nodos a exactamente *k* saltos. Se expande toda; su vecindad no visitada es la siguiente ola.
- **La cola FIFO materializa el orden por nivel**: saca primero lo más antiguo; jamás expandes el nivel *k+1* antes de acabar el *k*.
- **La visita es la que hace terminar**: marcar al descubrir (no al sacar) garantiza que cada nodo entra a la cola una vez → O(V+E) en grafos cíclicos.
- **BFS = "camino más corto" cuando cada arista vale 1.** Con pesos, es Dijkstra (cap. 22); con profundidad, es DFS (cap. 5).
- **Alcanzabilidad en un origen cuesta O(V+E)**, no comparar V² pares: la promesa del motor de grafos frente al join relacional.
- **Lo que aquí es idea, allí es el corazón**: `closeness_centrality` (cap. 24) y `bfs_fronteras` / `bfs_streaming` (cap. 26) ejecutan ESTE BFS sobre el `GraphStore` del cap. 8.

## 4.12 Ojo, cuidado con…

- **Marcar la visita al SACAR, no al DESCUBRIR.** Error nº 1: el mismo
  nodo del nivel *k* entra a la cola una vez por cada vecino que lo
  descubre; duplicas expansión y rompes los niveles. Marca al descubrir
  (encolar).
- **Usar pila o "lista sin orden" y llamarlo BFS.** Con pila es DFS (no
  garantiza distancia); con lista a lo bruto, sin niveles. Falta el orden
  FIFO y la pregunta "¿a cuántos saltos?" no tiene respuesta.
- **Confundir BFS con Dijkstra ponderado.** Si las aristas tienen peso,
  BFS miente: "3 saltos baratos" puede ser peor que "4 saltos caros".
  BFS solo sirve cuando cada arista cuenta 1 → con pesos, cap. 22.
- **Confundir "encontrar algo" con "encontrarlo con distancia mínima".**
  Un recorrido que encuentra a Zoe sin control de nivel te da existencia,
  no el menor número de saltos. La ola es lo que añade la distancia.
- **Creer que el tipo de almacenamiento cambia el algoritmo.** BFS
  sobre CSR o sobre HashMap es el mismo BFS: cambia el coste de
  `adyacencia`, no el orden de niveles (pregunta crítica de
  `CORPUS.yml` vol-II-cap-04).

| Término | Significado exacto | No confundir con |
|---|---|---|
| Frontera / nivel | Nodos a k saltos del origen | Visita — la marca que impide volver |
| Visita | Nodo ya descubierto (no se vuelve) | Descubrimiento — primera vez que lo toca la ola |
| Alcanzabilidad | Existe (al menos) un camino | Conectividad / componente — cap. 5 |
| Camino mínimo BFS | Menor número de saltos | Camino de peso mínimo — Dijkstra, cap. 22 |
| Cola FIFO | Saca lo más antiguo | Pila LIFO — DFS, cap. 5 |

## 4.13 Pin de batalla

> *«En un grafo, cada arista que avanzas barre el límite que no has visto. La frontera es tu único frente: quien la respeta llega por el camino más corto; quien la ignora, dando vueltas.»*

## 4.14 Si solo lees 30 segundos

BFS recorre un grafo **por niveles** desde un origen: la **frontera** (nodos a exactamente *k* saltos) se expande completa, y su vecindad no visitada forma la siguiente ola. Lo garantiza una **cola FIFO** y la regla de **visitar al descubrir**, que hace terminar el algoritmo en O(V+E) aun con ciclos. Por eso "¿existe un camino entre A y B y a cuántos saltos?" es la pregunta que un motor de grafos responde barata — nativo, sin joins —: un BFS desde A. Es la idea que `closeness_centrality` (cap. 24) y `bfs_fronteras` (cap. 26) ejecutan por fábrica sobre el `GraphStore` (cap. 8).

## 4.15 Una historia pequeña

Cuando empezamos LiraDB en el papel, "consultar el grafo" sonaba a abrir una tabla y mirar. Hasta que Ana pidió algo inocente: *"¿quién está a menos de tres saltos de mí?"*. Lo intentamos con la intuición del capataz SQL — un join, luego otro, y otro— y la hoja de cálculo se llenó de combinatoria antes de acabar la primera columna. Aquella noche, dibujando la red social en la pizarra del taller, uno de nosotros trazó círculos concéntricos alrededor de Ana y dijo: *"es una ola, no una tabla"*. Marcamos el anillo a 1 salto, el de 2, el de 3... y la consulta pasó de "imposible" a "obvia". Desde entonces, cada vez que LiraDB responde en milisegundos "están a 2 saltos" a un lector que esperaba la eternidad, es esa ola dibujada la que lo hace — la misma ola que verás ejecutar en el cap. 24 y por streaming en el cap. 26.

## Ejercicios resueltos

**1. ¿Por qué al marcar la visita "al descubrir" el BFS termina en
O(V+E)?**

Cada nodo se descubre una vez (al encolarlo lo marcas), así que cada
nodo se expande a lo sumo una vez: eso son V expansiones. Al expandir
un nodo examinas todas sus aristas de salida; en total, cada arista se
examina a lo sumo una vez desde cada extremo, eso son E (o 2E en no
dirigido). La suma es O(V+E). La marca de visita lo garantiza: sin
ella, en un grafo cíclico un nodo volvería a entrar infinitas veces y
el bucle no terminaría.

**2. ¿Por qué si BFS descubre a Zoe en el nivel 2, entonces 2 ES la
distancia mínima?**

Porque los niveles se expanden en orden estricto. Si existiera un
camino de 1 salto, la ola del nivel 1 habría tocado a Zoe antes de que
la del nivel 2 se expandiera. Como tocarla "por primera vez" ocurre
solo cuando la ola avanza en orden, un descubrimiento en el nivel *k*
implica que ninguna ola anterior lo logró — la distancia mínima es *k*.
(Es el argumento del §4.3 "la distancia sale sola", y el que CLRS 22.2
formaliza.)

## Ejercicios propuestos

**Esencial (retrieval).** Dibuja: `Ana` → {`Bruno`, `Clara`, `David`}; `Bruno` → {`Elena`, `Fran`}; `Clara` → `Gonzalo`. Ejecuta BFS **a mano** desde `Ana`: contenido de la cola en cada paso, visitados, y **orden de descubrimiento por niveles**. Sin mirar el capítulo. Comprueba contra `niveles = [[Ana],[Bruno,Clara,David],[Elena,Fran,Gonzalo]]`. _(Verificable: compáralo con el `niveles` de un BFS por niveles del cap. 26.)_

**Intermedio (analizar).** Traduce "¿quién está a exactamente 2 saltos de Ana?" y "¿existe un camino de Ana a Gonzalo?" a pasos de BFS. ¿En qué fallaría un SQL self-join para lo primero? Argumenta O(V+E) frente a O(V²).

**Experto (crear / interleaving con cap. 7).** Modela (con la voz del cap. 7) un mini-grafo personas → aeropuertos → vuelos **dirigidos** (`VUELA_DE(A→B)` no sirve a la inversa). Recorre en BFS dirigido desde "tú" hasta "Hong Kong" y da su distancia en saltos; anota qué aristas descartas por sentido.

## Para profundizar

- **CLRS, "Introduction to Algorithms" — cap. 22** (Elementary Graph
  Algorithms): BFS formal, la prueba de que descubre caminos mínimos no
  ponderados, y la complejidad O(V+E).
- **E. W. Dijkstra, "A Note on Two Problems in Connexion with Graphs"
  (1959)**: el algoritmo de caminos mínimos ponderados; entiéndelo
  como el hermano mayor de BFS (BFS = el caso con pesos 1).
- **E. F. Moore** y la búsqueda en laberintos de los años 50: el
  origen del "barrer por anchura".
- **S. Milgram (1967)**, "The Small-World Problem": el experimento de
  los seis grados que convirtió el BFS en una pregunta de grafos
  sociales.
- **Workspace**: `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs`
  y `cap26_proyeccion.rs` — el BFS de este capítulo, escrito como un
  motor real.
- **L. C. Freeman (1978)** sobre centralidad de cercanía, y la
  corrección de **Wasserman-Faust** para componentes desconectadas
  (base del cap. 24).

## Mini-diálogo: en guardia nocturna

> — Vale, ola, frontera, niveles... Pero ¿por qué tanta ceremonia para "encontrar si hay un camino"? ¿No basta con empezar y ver qué aparece?
>
> — Porque el motor no pregunta "¿existe alguna manera de llegar?", pregunta "¿existe, **y cuál es el menor número de saltos**?". Si solo recorres sin niveles, encuentras a Zoe, pero no sabes si está a la vuelta de la esquina o al otro lado del planeta.
>
> — Y la cola... ¿qué pinta el orden ahí?
>
> — La cola es la que obliga al orden. Sin ella, expandes a lo bestia y rompes el "nivel a nivel". Con ella, la ola avanza bien formada y una sola vez por nodo. Eso — y nada más — es lo que hace que "¿están conectados?" cueste O(V+E) y no una eternidad.
>
> — O sea que el poder del motor de grafos no está en memorizar el grafo, sino en esta ola.
>
> — Exacto. El grafo solo se guarda; la ola es la que consulta. Y ahora ves que cada "¿a cuántos saltos?" que un lector hace a una base de datos de grafos es, por debajo, esta ola de Ana. En el cap. 5 la empujaremos hacia la profundidad — y descubriremos que hay mundos, componentes, a los que ni siquiera esta ola llega.

---

*(Próximo capítulo: **5 — DFS, componentes conexos, orden topológico y SCC**. Aquí el BFS nos mostró una ola por niveles; la profundidad explora hasta el fondo y vuelve, y con ella sabremos si el grafo se parte en mundos separados — las componentes — y cómo ordenar tareas con dependencias.)*
