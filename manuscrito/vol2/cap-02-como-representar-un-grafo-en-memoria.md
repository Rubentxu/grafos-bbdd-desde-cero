# Capítulo 2 — Cómo representar un grafo en memoria

> *«No hay una forma de guardar un grafo. Hay preguntas, y para cada pregunta una forma que la hace barata y otra que la hace miserable.»*

## 2.0 La anécdota de la esquina

En 1977, en el departamento de informática de Yale, un equipo de matemáticos aplicados —Stanley Eisenstat, Andrew Sherman y sus colegas— publicó un paquete que no era un algoritmo ni una base de datos. Era una *forma de mirar los datos*. Llevaban años resolviendo sistemas de ecuaciones lineales con millones de variables: la matriz de un yacimiento de petróleo, de un puente, de una malla de elementos finitos. Esas matrices eran casi todo ceros. Guardar cada cero era un absurdo que no cabía en ninguna máquina de la época; pero si tirabas los ceros, ¿cómo seguías multiplicando la matriz por un vector sin perderte?

Su respuesta —el **Yale Sparse Matrix Package**, en los informes YALEU/DCS/RR-112 y RR-114— fue comprimir la matriz en tres arrays planos: los valores distintos de cero, los índices de columna de cada uno, y los comienzos acumulados de fila. Hoy a ese formato lo llamamos **Compressed Sparse Row (CSR)**, y lleva medio siglo dando de comer a la computación científica.

¿Y qué tiene que ver eso con tu base de datos de grafos? Todo. Porque la **matriz de adyacencia de un grafo es una matriz dispersa**: cada arista es un «distinto de cero», y en un grafo real los ceros son el 99,99 % de la superficie. La decisión que tomó el equipo de Yale en un laboratorio de matemáticas —*no guardes lo que no importa, y recuerda dónde empieza cada fila*— es exactamente la decisión que tomará el motor de LiraDB en el capítulo 14. Este capítulo es donde empiezas a pensar como ellos: no «¿cómo guardo un grafo?», sino **«¿qué operación voy a ejecutar, y qué guardado hace que sea barata?»**.

Ese cambio de pregunta es sutil y, por eso mismo, el más difícil de adoptar. Un físico te diría que chocas contra un muro cuadrático; un ingeniero de bases de datos te diría que chocas contra un *patrón de acceso*. Los dos tienen razón, y este capítulo es la cámara lenta de esa colisión: primero la ves venir en una pizarra (matriz de adyacencia), después la mides (los terabytes de un grafo social), y al final huyes hacia las representaciones que de verdad habitan un motor real. Ninguna de las cuatro que verás es «la buena». Cada una es la buena *para una operación*. Por eso, antes de verlas, hay que dejar grabada a fuego la pregunta que las elige.

## 2.1 Objetivo

Al terminar este capítulo sabrás **que una representación de grafo no se elige en el vacío**: se elige por el patrón de acceso de la base de datos que la va a usar. Y sabrás dibujar, medir y comparar las cuatro candidatas que verás una y otra vez en este libro:

1. La **matriz de adyacencia** — la que imprime el libro de algoritmos, O(V²), y que solo compite en grafos pequeños y densos.
2. La **lista de adyacencia** — la que de verdad usa `MemoryStore` en el cap. 8 (`adj_out`, `adj_in`), O(V+E).
3. La **edge list / lista de aristas** — el formato compacto de carga y de respaldo, O(E).
4. El **CSR** — la «lista de adyacencia comprimida» en arrays planos, que volverá en serio en el cap. 14 y que nació en Yale en 1977.

No vas a escribir código en este capítulo: la Parte I del Vol.II es conceptual, y el primer fichero real de `vol2-liradb` llega en el cap. 7. Pero vas a hacer algo más difícil y más duradero: **fijar en tu cabeza la cuestión que determina todo lo demás — *¿qué operación ejecuta la BBDD?*** Si este capítulo cumpliera su misión y solo recordaras una frase dentro de un año, sería exactamente esa.

En concreto, al terminar serás capaz de:

- Calcular a mano el coste de memoria O(...) de las cuatro representaciones para un grafo concreto y descartar las que no caben.
- Decir, para cada operación de un motor (existe arista, vecinos, recorrer, exportar), cuál representación la hace barata y por qué.
- Reconocer en código real (`MemoryStore`, cap. 8) que la elección «lista de adyacencia» no es decorativa: es la respuesta a una pregunta de acceso concreta.

## 2.2 Problema

Tienes un grafo. Quieres construir un motor de base de datos encima. La primera pregunta del constructor no es «¿qué algoritmo uso?», sino una que suena demasiado simple: **¿dónde viven los vecinos?** Suena a detalle trivial. No lo es. Y la mejor manera de verlo es dejando que la respuesta ingenua se estrelle sola.

Imagina que guardas el grafo como una **matriz de adyacencia**: una tabla de `V × V` donde la celda `[u][v]` vale 1 si existe la arista u→v. Ahora coge un grafo realista —digamos el de una red social con 1.000.000 de nodos (personas) y 4.000.000 de aristas (amistades). Ese grafo tiene un **grado medio de 4**: cada persona conoce, en promedio, a otras 4.

La matriz de ese grafo tiene `1.000.000 × 1.000.000 = 10¹²` celdas. A un bit por celda, **125 GB**. A un `u32` por celda —lo que de verdad usarías si quisieras distinguir tipos de relación—, **4 TB**. Y tu grafo entero, con todas sus aristas, ocupa nada. La matriz es un desierto con un lago: la densidad (aristas sobre celdas posibles) es `4 M / 10¹² = 4·10⁻⁶`. Cuatro partes por millón de la tabla tienen información.

El problema de fondo es que **estás pagando por la superficie cuadrada del mundo** (V²) cuando tu grafo solo puebla un hilo de carretera (V+E). Y no es un problema abstracto: son literalmente 4 terabytes que no tienes. No estás discutiendo sobre cuál de dos algoritmos es medio nanosegundo más rápido; estás discutiendo si tu base de datos *funciona* o simplemente no arranca.

Y hay un segundo matiz que se le escapa al recién llegado. Cuando un libro de algoritmos te enseña la matriz, te la enseña para *pensar*, no para *almacenar*. La matriz hace triviales las preguntas que se hacen las matemáticas («¿estos dos vértices están conectados?», un `1` o un `0`). Las matemáticas no se preocupan por barrer un millón de celdas para sacar a cuatro vecinos: tú, que vas a construir un motor, sí. El motor no pregunta «¿existe la arista?» un par de veces; lo pregunta *y* recorre vecindarios un millón de veces al día. La representación que elige un motor no puede ignorar ese hecho.

Y un tercero, que es la causa de la mayor parte de las peleas con el compilador. En la matriz, cada arista es un bit. Pero «Ana conoce a Bo desde 2020» es una cosa con su propio dato; no cabe la fecha en un bit. Y un día querrás «todas las amistades desde 2015» o «el camino más corto en kilómetros», y la arista necesita guardar su propio dato — su **propiedad** — y un *tipo* — su **label** — para distinguirla de las demás. La matriz del Vol.I es el esqueleto; aquí ya sabes, por el cap. 1, que el motor viste el esqueleto, y la matriz desnuda no tiene dónde colgar el vestido.

## 2.3 Modelo mental

Piensa en el **mismo grafo pequeño dibujado de tres maneras**, y compruébalo: son tres espejos, no tres grafos. Un grafo de 4 personas —Ana conoce a Bruno y a Carlos; Bruno conoce a Carlos; Carlos conoce a Dana—, cruzado en las tres direcciones que vamos a usar durante todo el libro:

```
EL MISMO GRAFO:  Ana→Bruno, Ana→Carlos, Bruno→Carlos, Carlos→Dana

MATRIZ DE ADYACENCIA          LISTA DE ADYACENCIA         CSR (comprimido)
    A   B   C   D                 Ana: [B, C]            offsets=[0,2,3,4,4]
  A  .   1   1   .                 Bruno:[C]              targets=[B,C,C,D]
  B  .   .   1   .                 Carlos:[D]                 └nodo Ana┘
  C  .   .   .   1                 Dana: []                     └Bruno┘ ...
  D  .   .   .   .
```

- En la **matriz**, cada arista es una celda `[fila][col]`. Localizar «¿existe Ana→Bruno?» es `matriz[0][1]`: un doble índice, **O(1) instantáneo**. Pero el espacio que pagas es el cuadrado completo, con sus 12 celdas vacías. Cada `1` en la tabla cuesta, en realidad, todo lo que no se ve: la fila entera detrás de él.
- En la **lista de adyacencia**, cada persona tiene su propia lista de a quién conoce. «¿Quiénes son los vecinos de Ana?» es `lista[0]`: una resta de dirección, y luego recorres `[B, C]`. Espacio solo para lo que existe. No hay superficie cuadrada que mantener: solo datos reales.
- En el **CSR**, comprimes esas listas en un solo array plano de `targets` y otro de `offsets` que dice dónde empieza cada persona. La lista de Ana no es un objeto: es el intervalo `targets[0..2]`. Misma información que la lista de adyacencia, misma memoria, pero **contigua** — y por eso le gusta al planificador de la CPU y más tarde al disco.

El **momento ¡ajá!** de este capítulo: **cada representación convierte en trivial una pregunta distinta.** La matriz es trivial para «¿existe esta arista?». La lista y el CSR son triviales para «¿quién es vecino de X y con quiénes?». Y ninguna de las tres es trivial para *todo* a la vez. Guardar un grafo no es elegir una figura bonita: es decidir **a qué operación le vas a regalar la velocidad** y a cuáles vas a cobrar más caro.

Y un detalle del diseño que te perseguirá hasta el cap. 14: **la dirección.** El grafo del dibujo es dirigido: Ana conoce a Carlos, pero Carlos no conoce a Ana. «¿A quién conoce Ana?» (salientes) y «¿quién conoce a Ana?» (entrantes) son dos preguntas con dos respuestas. Cualquier representación que elijamos debería, si el motor lo pide, poder responder ambas con baratura parecida — motivo por el que `MemoryStore` (cap. 8) mantiene *dos* listas (`adj_out`, `adj_in`) y por el que el cap. 14 guardará *dos* CSR espejo.

## 2.4 Primera solución

La versión que escribe cualquiera al empezar es la **matriz de adyacencia**. Es el primer dibujo del libro de algoritmos, es fácil de razonar, y durante una clase entera parece impecable. CLRS, *Introduction to Algorithms*, cap. 22 —el manual enorme de este terreno— presenta las dos opciones canónicas (lista y matriz) y, de las dos, la matriz es la que visualiza mejor en una pizarra. Nadie lo puede negar: es *bonita*.

```rust
// Lo que tu instinto dibuja primero.
// nota: n = V. La tabla tiene V*V celdas.
struct MatrizAdy {
    celdas: Vec<Vec<bool>>,   // celdas[u][v] = ¿existe u → v?
}
```

Para comprobar si Ana conoce a Bruno: `celdas[0][1]`, un doble índice, sin buscar nada. Para los que conocen a Ana: barres la fila 0. Cuenta lo que acaba de pasar en la pizarra con nuestros 4 nodos:

- Existe arista: un acceso `celdas[i][j]`. Ninguna otra representación lo hace más rápido.
- Enlistar vecinos de Bruno: barres la fila de Bruno, `celdas[1][*]`, y guardas las columnas con `1`.
- Espacio usado: `4 × 4 = 16` celdas para un grafo de 4 aristas.

Los tests con un grafo de 4 nodos pasan. Tres personas, un puñado de aristas, una matriz de 4×4. Nada se queja. Ni siquiera te das cuenta de que ya has hecho una elección con consecuencias — porque para 4 nodos, *todas* las opciones son indiferenciables, y eso es lo insidioso de los juguetes.

## 2.5 Sus límites

Hasta que alguien trae un grafo con más de un puñado de nodos, y se acaba la fiesta. Reúne los tres síntomas con números de verdad; los re-verás, ampliados, en la tabla del cap. 14:

1. **El espacio es O(V²) puro.** No importa cuán poquitas aristas tengas: si engordas el número de nodos, el cuadrado te devora. El grafo social del §2.2 necesita terabytes. No es un retoque: es un muro estructural escrito en la notación. A la pregunta «¿cabe?», la matriz responde con una constante que es V²; todo lo demás —cuántas aristas, qué grado— es ruido que no la modifica.
2. **Recorrer vecinos es O(V), no O(grado).** Para saber a quién conoce Ana tienes que barrer *toda* la fila 0, aunque Ana conozca solo a 2 de un millón. Y disculpa por adelantado la ironía: la operación que más ejecuta una base de datos de grafos —«quién es vecino de quién», el corazón de un BFS y de un PageRank— es justo la que la matriz hace mal. El acceso O(1) de la matriz contesta una pregunta que el motor casi nunca hace; la pregunta que el motor hace de verdad (enlistar) la matriz la cobra a precio de fila entera.
3. **El grafo real es disperso.** Los grafos de redes, de rutas, de la web, de conocimiento: todos tienen grado medio pequeño frente a V. La densidad cae como `E/V² ≈ (grado_medio)/V`, que tiende a 0. La matriz solo compite cuando un grafo es *denso* (E ≈ V²), y eso casi no existe fuera de los juguetes y de unos pocos casos específicos (grafos de co-ocurrencia muy conectados, el grafo completo de un campeonato).

El diagnóstico no es «la matriz es mala». Es **«la matriz es la respuesta correcta a la pregunta equivocada»**: es brillante si tu operación central es «¿existe esta arista?» en un grafo pequeño y tupido, y es un despropósito para un motor que vive de recorrer.

## 2.6 Solución evolucionada

Cuando cambias la pregunta —de «¿existe esta arista?» a «¿quiénes son sus vecinos?»—, la elección se mueve sola hacia otra familia de representaciones. Aquí está el arsenal que manejarás en todo el libro, y la idea rectora de cada una.

### 2.6.1 La lista de adyacencia — la que usa `MemoryStore` (cap. 8)

Guardas un `Vec` por nodo con sus vecinos. Si tu grafo es dirigido y quieres responder tanto «¿a quién conoce X?» como «¿quién conoce a X?», guardas dos. En memoria, LiraDB lo hará literalmente así (cap. 8):

```rust
// Fragmento real de cap08_graph_store.rs (MemoryStore):
pub adj_out: Vec<Vec<EdgeId>>,   // adj_out[u] = aristas que salen de u
pub adj_in: Vec<Vec<EdgeId>>,    // adj_in[v]  = aristas que entran a v
```

Fíjate en un detalle pequeño que dice mucho: no guardamos `Vec<Vec<NodeId>>` de vecinos, sino `Vec<Vec<EdgeId>>` de **identificadores de arista**. En un Property Graph cada arista puede tener su propio tipo y sus propiedades, así que lo que la lista guarda es el ID de la arista, y desde ahí coges origen y destino cuando haga falta (una indirección extra que, en la práctica, cuesta poco). Lo esencial para este capítulo: **dos `Vec` por cada nodo, uno saliente y otro entrante.** «¿Quiénes siguen a Ana?» y «¿a quién sigue Ana?» son consultas distintas, y ambas merecen ser baratas — por eso hay dos listas, y por eso en el cap. 14 verás que el motor guarda **un par espejo** en CSR (forward y backward).

El coste: espacio **O(V+E)**, «vecinos de u» en **O(grado de u)**, y nada que escanear para el recorrido de un BFS `u → out_edges(u) → targets`. El precio oculto: **mantener las dos listas sincronizadas** cuando borras un nodo o una arista — mira `delete_edge` del cap. 8: hay que hacer `retain` en `adj_out[source]` *y* en `adj_in[target]`; y `delete_node` detona una cascada que recoge todas sus aristas para borrarlas una a una. Una segunda moneda la paga la memoria a gran escala: cada `Vec` interior vive en su propio trozo del heap, así que un grafo de un millón de nodos es un millón de islotes dispersos. La representación funciona, pero no es *contigua*.

### 2.6.2 La edge list — el formato de carga y de respaldo

Es el más simple de todos: un `Vec` de pares `(u, v)`, uno por arista, en orden plano. Espacio **O(E)**, el más compacto de los tres. Su virtud es la compacidad y un orden natural: es justo lo que sacas de un fichero cuando *cargas* el grafo, o lo que vuelcas cuando *respaldas* todo. Es, de hecho, el formato de intercambio por defecto: una base de datos exporta su contenido como una larga lista de aristas, y otra lo importa leyendo esa misma lista.

```
// El mismo grafo en edge list:
Ana→Bruno
Ana→Carlos
Bruno→Carlos
Carlos→Dana
```

Su tragedia es la simetría con su virtud: la información está organizada *por conjunto*, no *por origen*. Para preguntar «¿quién es vecino de Ana?» tienes que escanear la lista completa **O(E)**, porque no hay manera de saltar a «las aristas de Ana» sin buscarlas. Edge list para *mover* el grafo; lista o CSR para *habitarlo*. Es la diferencia entre el inventario de un almacén (edge list) y el plano de los pasillos por los que caminas cada día (lista / CSR).

### 2.6.3 El CSR — la lista de adyacencia comprimida (volverá en el cap. 14)

CSR toma la idea de la lista de adyacencia y la aplasta en **dos arrays planos**: `offsets` (dónde empieza la lista de cada nodo) y `targets` (todos los vecinos, uno tras otro, sin huecos), más un `offsets[n]` final igual al número de aristas. Como viste en el §2.3, la lista de Ana no es un objeto: es `targets[offsets[Ana]..offsets[Ana+1]]`. Misma información y casi la misma memoria que la lista de adyacencia, pero **contigua**: la CPU la precarga de carrerilla (localidad de caché), el disco la persiste en menos saltos, y no hay una alocación por nodo.

```
// El mismo grafo en CSR (forward):
offsets = [0, 2, 3, 4, 4]      // Ana empieza en 0, Bruno en 2, Carlos en 3, Dana en 4
targets = [B, C, C, D]          // (sin aristas salientes de Dana, offsets[4]=4)
```

Su debilidad es la seña de identidad de «comprimido»: es incómodo de *mutar* en caliente. Insertar una arista puede «empujar» a todo el array `targets` si no queda hueco, y re-encajar los `offsets`. Por eso, en un motor en memoria que muta mucho (inserta, borra), el `Vec<Vec>` del cap. 8 es más cómodo; y por eso cuando LiraDB quiera *leer en ráfagas y persistir* el recorrido de manera barata sobre páginas, elegirá el CSR (cap. 14). Es el heredero directo del paquete de Yale de 1977, y su historia vuelve en el cap. 14 con el detalle completo de por qué dos arrays valen más que un millón de `Vec`.

Una sutileza que pagarás si no la ves: para responder «¿cuántas aristas salen del nodo i?», en una lista de adyacencia calculas `lista[i].len()` (una lectura de longitud, O(1)). En el CSR, calculas `offsets[i+1] - offsets[i]` (una resta, también O(1), pero con dos accesos). En un grafo enorme esa resta se mide en milisegundos acumulados; lo verás en el cap. 14 cuando se cuente el coste real con `cargo bench`. No es un drama, pero merece estar en tu cabeza: el CSR cambia la *forma* de las preguntas, no solo la *velocidad*.

### 2.6.4 Una cuarta olvidada que reaparecerá: el diccionario de aristas

Hay una quinta forma que merece un párrafo porque la verás en bases de datos reales: el **mapa / diccionario de aristas** indexado por el par `(u, v)` — un `HashMap<(NodeId, NodeId), EdgeId>` o un árbol B+ sobre ese par. Existe por una razón muy concreta: la consulta «dame los datos de la arista entre Ana y Bo» es **O(1)** en hash y **O(log E)** en B+, sin importar el grado de Ana. Ninguna de las otras cuatro te la da así de barata: en la matriz es O(1) pero solo tienes el bit; en la lista de Ana o Bo es O(grado); en el CSR igual; en la edge list es O(E).

El precio: el espacio se va a `O(E)` extra solo para el índice, y **no te resuelve los vecindarios** — para «vecinos de Ana» vuelves a la lista. Por eso los motores serios mantienen dos estructuras: una *lista de adyacencia* (o CSR) para recorrer, y un *índice de aristas* (hash o B+) para «dame esta arista». LiraDB combinará exactamente así cuando llegue el cap. 15. La regla es la misma: **una estructura por operación dominante**, y la combinación que elijas define el motor.

Para cerrar: las cuatro (cinco, con el diccionario) representaciones no compiten en el vacío; compiten por **baratura de operación**. La matriz gana en «existe(u,v)» de un grafo denso; la lista y el CSR ganan en «vecinos(u)»; la edge list gana en «exportar / importar todo»; el diccionario gana en «dame esta arista concreta». Un motor serio no suele elegir una: **combina** y sabe cuándo está ejecutando cada operación. La pregunta «¿qué operación ejecuta la BBDD?» se contesta, casi siempre, con una combinación.

## 2.7 Los porqués (grill)

Reunamos la comparación en una sola tabla — la que te acompañará hasta el cap. 14 — y hagamos luego la ronda de «¿por qué así y no de otra forma?» que exige que cada elección rinda cuentas:

| Representación | Espacio | «existe(u,v)» | «vecinos(u)» | Talento que la define |
|---|---|---|---|---|
| Matriz de adyacencia | O(V²) | O(1) | O(V) — barre la fila | responder una arista puntual |
| Lista de adyacencia | O(V+E) | O(grado u) | O(grado u) — localiza en O(1) | recorrer vecindario (vecinos) |
| Edge list | O(E) | O(E) — escanea | O(E) — escanea | mover / importar / exportar |
| CSR | O(V+E) contiguo | O(grado u) | O(grado u) — localiza en O(1) | leer y persistir vecindario |

**¿Por qué la matriz de adyacencia para «existe(u,v)» y no para BFS?** — Resuelve la *presencia* de una arista en O(1) con un doble índice. Se descartó para el recorrido de vecinos porque barres una fila entera (O(V)) aunque el grado sea 2. Precio: O(V²) de memoria. Si no lo supiéramos, guardarías un grafo de 1 M de nodos disperso como matriz: 4 TB. Es el mismo argumento de densidad que el cap. 14 convierte en cifra con tinta.

**¿Por qué la lista de adyacencia para «vecinos de u»?** — Resuelve *enlistar* vecinos en O(grado), con O(V+E) de memoria. Se descartó para «¿existe u→v?» porque no hay más remedio que barrer la lista (O(grado)). Modo de fallo: un PageRank que necesita entrantes no va con una sola lista; hay que mantener una segunda (backward). El `MemoryStore` del cap. 8 duplicará la dirección (`adj_in`), y esa duplicación es la semilla del par forward / backward del cap. 14.

**¿Por qué CSR?** — Resuelve lo que la lista resuelve, pero en arrays planos contiguos: mejor **localidad de caché** y un formato trivialmente persistible. Se descartó —hasta el cap. 14— por ser tedioso de *mutar*: en memoria el `Vec<Vec>` es más cómodo para insertar / borrar; el CSR brilla cuando lees y persistes, no cuando editas al vuelo. Fuente: **Yale Sparse Matrix Package, 1977** (informes YALEU/DCS/RR-112 y RR-114). Si no lo conociéramos, no sabríamos por qué el cap. 14 elige arrays planos para bajar la adyacencia a páginas.

**¿Por qué existe la edge list?** — Resuelve la compacidad total (O(E)) y el formato natural de carga / respaldo. Se descartó para BFS porque no hay forma de saltar a los vecinos de u sin escanearla entera. Modo de fallo: una consulta «vecinos de u» se convierte en O(E); un BFS sobre edge list degrada a O(V·E).

**¿Por qué mantener dirección inversa (`adj_in`)?** — Resuelve PageRank y «¿quién apunta a X?» sin pagar O(E) por consulta. El precio es duplicar el espacio de adyacencia y sincronizar ambas listas al borrar — tarea que ya hace `delete_edge` del cap. 8 con dos `retain`. Es la misma razón por la que el cap. 14 guardará un par CSR espejo (forward + backward).

**¿Resuelve este cap. los IDs generacionales?** — No. Los pospone. La pregunta de CORPUS «¿por qué `slotmap` y no índices reciclados?» se responde en el cap. 3. Aquí solo se fija una pieza: el `usize` que nombra a un nodo es una **clave estable**, no una posición re-numerable — y por eso `MemoryStore` del cap. 8 guarda `Vec<Option<T>>` con huecos en vez de re-numerar al borrar. Piénsalo: cuando borras a Bruno, ¿renumeras a Carlos para «compactar» el array? Si lo haces, cambias el nombre de Carlos y rompes todas las aristas que lo mencionan. Eso es material del cap. 3.

## 2.8 Trampas y errores comunes

Todo el mundo tropieza aquí de la misma manera. Si detectas estos seis síntomas a tiempo, te ahorrarás capítulos enteros de confusión:

1. **Creer que la matriz es la representación «correcta»** solo porque es la primera que te enseñaron. Antes de defenderla, calcula V². Si tu grafo tiende a pocas aristas por nodo, estás defendiendo terabytes.
2. **Confundir «localizar la lista» con «recorrerla»**. En una lista de adyacencia, *localizar* a los vecinos de Ana es O(1); *recorrerlos* es O(grado de Ana). En un BFS lo que cuenta es el recorrido, y ese es barato en lista / CSR, no en matriz. Confundirlo te hace creer que matriz y lista empatan en recorrido cuando no empatan en absoluto.
3. **Usar edge list, lista de adyacencia y CSR como sinónimos.** Edge list = pares planos; lista = un `Vec` por nodo; CSR = `offsets` + `targets` planos. Tres estructuras, tres perfiles de coste, tres usos.
4. **Suponer que la elección en memoria vale para el disco.** Lo que brilla en RAM (un `Vec<Vec>` disperso por el heap) puede ser un desastre persistido en páginas: el cap. 14 volverá sobre esta herida y te mostrará por qué el CSR gana en disco incluso donde en RAM empataba.
5. **Pensar que basta una sola representación.** Un motor responde operaciones distintas; la madurez es saber cuándo usar cada una (o combinarlas) *según la consulta que llega*.
6. **Confundir GDBMS con biblioteca de algoritmos.** Aquí no eliges representación para que un algoritmo «se vea bonito» en una pizarra: eliges para que un *motor* responda millones de consultas. Eso cambia qué pesa más — el patrón de acceso, no la elegancia.

**Precisión de lenguaje (glosario)**: `vecinos(u)` (toda la lista) ≠ `existe(u,v)` (una celda); **grado** ≠ **grado medio**; **denso** (E ≈ V²) ≠ **disperso** (E ≤ cV); **CSR** ≠ «matriz» — es una forma *comprimida* de la lista de adyacencia; **GDBMS / motor de BBDD** ≠ biblioteca de algoritmos; **lista de adyacencia** ≠ **edge list**; **forward** (`adj_out`) ≠ **backward** (`adj_in`).

## 2.9 Una historia pequeña

La primera versión de nuestro prototipo guardaba el grafo social de prueba —unas 30 personas y 90 amistades— como una matriz de adyacencia. Para 30 personas es una tabla de 30×30: se imprime en una hoja. Funcionó gloriosamente una mañana entera, mientras nuestros BFS eran del tamaño de un recreo.

A la tarde, Ana importó el grafo real de un pequeño foro: 40.000 nodos y 300.000 aristas. La hoja se convirtió en un muro: `40.000² = 1.600 millones` de celdas, aunque el grafo solo tuviera 300.000 aristas — una densidad del 0,02 %. El CPU hacía gala: para enlistar el vecindario de cada nodo barría una fila de 40.000 celdas y encontraba 7. Íbamos a tardar una semana en hacer un BFS.

No era un fallo de optimización. Era que habíamos elegido **la respuesta correcta a la pregunta equivocada**. Cambiamos a listas de adyacencia —una por nodo, `Vec` por `Vec`, una para salientes y otra para entrantes— y el mismo BFS que iba a tardar una semana tardó un parpadeo. La moraleja no fue «cambia de estructura». Fue que, desde entonces, en LiraDB nadie toca una representación sin preguntarse antes qué operación va a ejecutar el motor una y otra vez. Esa pregunta —no la tabla— es la que siempre estuvo mandando desde el principio, y no habíamos querido mirarla.

## 2.10 Lo que te llevas

- **La representación se elige por el patrón de acceso, no al revés.** La misma estructura que vuelve trivial un BFS es un despropósito para otra consulta; la pregunta que desbloquea todo es «¿qué operación ejecuta la BBDD?».
- Las **cuatro candidatas** y su perfil: matriz O(V²) / O(1) en «existe» — solo grafos pequeños y densos; lista de adyacencia O(V+E) / O(grado) en «vecinos» — la elección de `MemoryStore` (cap. 8); edge list O(E) — el formato de carga / respaldo; CSR O(V+E) contiguo — la lista comprimida que el cap. 14 persistirá.
- La **dirección inversa** (`adj_in`) no es un lujo: PageRank, «¿quién apunta a X?», caminos inversos y centralidad la necesitan. Sin ella, esas consultas degradan a O(E).
- **Localizar ≠ recorrer**: en lista / CSR, *localizar* la lista de vecinos es O(1); *recorrerla* es O(grado). El BFS vive del recorrido.
- **CSR como lista de adyacencia comprimida**: `targets` plano + `offsets` acumulado. Misma información que la lista, pero contigua. Por eso la CPU la ama y el disco la ama. Nació en Yale en 1977 (YSMP) y vuelve en el cap. 14.
- **La representación en RAM ≠ la representación en disco**: lo que brilla en memoria puede ser un desastre persistido. La elección de hoy condiciona el cap. 14.

## 2.11 Ojo, cuidado con…

- **Defender la matriz sin haber calculado V².** Si tu grafo tiende a pocas aristas por nodo, V² te cuesta terabytes. La matriz no es «la forma» de un grafo: es la forma de una pregunta muy concreta (existe arista en grafo denso).
- **Decir «vecinos de u: O(1) en lista»** sin distinguir localizar de recorrer. Lo que es O(1) es la indirección al `Vec`; lo que cuesta O(grado) es la iteración que el BFS ejecuta.
- **Tratar edge list, lista y CSR como sinónimos.** Tres estructuras distintas con tres perfiles de coste y tres usos. Si las mezclas, mezclas las cifras de memoria del cap. 14.
- **Olvidar la dirección inversa.** Un motor que solo guarda `adj_out` puede hacer BFS, pero no PageRank. El cap. 8 lo sabe, y por eso `adj_in` existe.
- **Olvidar que la elección en RAM no se traslada tal cual a disco.** El cap. 14 te demostrará que el CSR gana en persistencia incluso donde en RAM empataba con `Vec<Vec>`.

## 2.12 Lo que hemos sacrificado

Toda elección tiene un precio. Lo que cada representación del capítulo te cobra por darte lo que te da:

- **Matriz**: accesibilidad O(1) a cambio de O(V²) de memoria. En un grafo disperso es un coste insoportable; en un grafo denso (co-ocurrencia, n = 50, casi todos enlazados) es la elección obvia.
- **Lista de adyacencia**: baratura de recorrido a cambio de un millón de `Vec` dispersos en el heap. La localidad de caché sufre; la mutación (insertar / borrar) es cómoda. La sincronización de `adj_out` y `adj_in` es trabajo que paga el `delete_edge`.
- **Edge list**: compacidad total y simplicidad a cambio de O(E) en «vecinos de u». Sirve para mover el grafo, no para habitarlo.
- **CSR**: localidad y persistencia barata a cambio de mutaciones incómodas. Insertar una arista puede empujar `targets` y re-encajar `offsets`; por eso en un motor que muta mucho se prefiere `Vec<Vec>` y se reserva el CSR para cuando se lee / persiste en ráfagas (cap. 14).

## 2.13 Cómo lo hace una BBDD real

Las cuatro representaciones viven, con variantes, en motores serios:

- **Neo4j** usa **listas de adyacencia** para cada nodo, con sus relaciones almacenadas como registros de primera clase con punteros al nodo origen y destino. Mantiene ambas direcciones (`adj_out` / `adj_in` análogos) para que las relaciones puedan recorrerse en ambos sentidos con la misma baratura. Verás el mismo patrón en `MemoryStore` (cap. 8). Además, Neo4j persiste cada relación como un registro físico en disco con un identificador estable (el «relationship id» que aquí será `EdgeId`).
- **TigerGraph** usa **listas de adyacencia en formato CSR** dentro de su motor en memoria, precisamente por la localidad de caché y por la facilidad de bajarlo a disco en una sola pasada. El CSR vive en producción, no solo en los libros: el cap. 14 lo materializa con cifras reales. TigerGraph lo llama «storage format» y lo trata como el canon; cualquier consulta Cypher que hagas se compila contra ese CSR.
- **Amazon Neptune** y **JanusGraph** mantienen una **edge list implícita** (tabla de aristas) como almacén canónico y construyen **índices de adyacencia** sobre ella para acelerar vecindarios — una arquitectura que combina dos representaciones: la edge list para responder «dame todas las aristas» y la lista (o el CSR) para responder «vecinos de u». Sobre esa base añaden caché en memoria que se parece mucho a un `MemoryStore` como el del cap. 8.
- **PostgreSQL con la extensión AGE** (graph sobre Postgres) usa tablas: nodos en una tabla, aristas en otra con `source_id` y `target_id`. Cuando haces «vecinos de u», Postgres hace un index scan sobre `source_id` — el equivalente industrial de nuestra edge list con índice. La moraleja para LiraDB: la lista de adyacencia «no hace falta inventarla desde cero» si tienes un buen índice; pero el CSR es más rápido que un index scan cuando el acceso es por *vecindario completo*.
- **Memgraph** usa listas de adyacencia en memoria y serializa a disco en formato CSR comprimido, optimizando para que las páginas que contienen los vecinos de un nodo estén contiguas. El mismo recorrido que aquí dibujamos a mano, industrializado.
- **ArangoDB** y **OrientDB** almacenan grafos como documentos JSON enlazados por identificadores; cada nodo lleva literalmente en su documento las claves de sus vecinos. Es la versión «lista de adyacencia implícita en el documento» — equivalente conceptual al `adj_out` del `MemoryStore`, con la ventaja de que el documento también lleva las propiedades (el vestido del cap. 1).
- **Virtuoso** (usado por la web semántica) trabaja con triples, pero a nivel de almacenamiento usa una combinación de mapas hash por sujeto + predicado y de CSR-like layouts para los predicados más consultados. Su mapa hash es exactamente el «diccionario de aristas» del §2.6.4.

**Retos para el lector (esencial / intermedio / experto)**:

- *Esencial*: para el grafo del §2.3, dibuja la matriz, la lista de adyacencia y la edge list. Identifica la operación «vecinos de Ana» y di cuántas celdas / entradas / posiciones se barren en cada representación. ¿Cuál es la única en la que esa operación cuesta O(grado)?
- *Intermedio*: dado un grafo con 50 nodos y 200 aristas (densidad ≈ 0,08), ¿qué representación es la más barata en memoria? ¿Y con 50 nodos y 1.200 aristas (densidad ≈ 0,48)? ¿En qué punto el CSR y la matriz de `bool` empatan en memoria?
- *Experto*: tu BBDD tiene que responder consultas Cypher como `MATCH (a)-[:KNOWS]->(b)` desde cualquier nodo origen, y debe permitir recorrido inverso `MATCH (b)<-[:KNOWS]-(a)`. ¿Una sola estructura basta? Justifica frente a las alternativas.

## 2.14 Pin de batalla

> *«No hay una forma de guardar un grafo. Hay preguntas, y para cada pregunta una forma que la hace barata y otra que la hace miserable. Pregunta antes de elegir.»*

### Resumen visual del capítulo (una sola mirada)

| Representación | Espacio | «existe(u,v)» | «vecinos(u)» | Mejor para… | La usa… | Origen |
|---|---|---|---|---|---|---|
| **Matriz de adyacencia** | O(V²) | **O(1)** | O(V) | grafos pequeños y densos; responder aristas puntuales | los libros de algoritmos como visualización | tradición matemática, formalizada en CLRS cap. 22 |
| **Lista de adyacencia** | O(V+E) | O(grado u) | **O(grado u)** (localizar O(1)) | mutar en memoria; recorrido BFS/DFS | `MemoryStore` del cap. 8 (`adj_out`, `adj_in`) | convención canónica de CLRS cap. 22 |
| **Edge list** | O(E) | O(E) | O(E) | cargar / respaldar / intercambiar el grafo | formatos de import / export; tabla de aristas de AGE | la forma más antigua de escribir un grafo |
| **CSR** | O(V+E) contiguo | O(grado u) | **O(grado u)** (localizar O(1)) | persistir y leer en ráfaga | LiraDB persistente (cap. 14); TigerGraph; Memgraph | **Yale Sparse Matrix Package, 1977** |

Si solo te llevaras **una tabla** del capítulo, debería parecerse a esta.

## 2.15 Si solo lees 30 segundos

Una matriz de adyacencia vale para «¿existe esta arista?» en un grafo pequeño y denso, y es un desastre en cualquier grafo realista por culpa de O(V²). Una lista de adyacencia vale para «vecinos de u» en O(grado), con espacio O(V+E) — es la elección de `MemoryStore` en el cap. 8 (`adj_out`, `adj_in`). Una edge list vale para mover / exportar el grafo, O(E). El CSR es la lista comprimida en arrays planos, contigua y persistible — la recoge el cap. 14. La pregunta que las elige es siempre la misma: **¿qué operación ejecuta la BBDD?**

## Ejercicios resueltos

**1. Localizar vs recorrer en una lista de adyacencia.** Para el grafo del §2.3, ¿cuánto cuesta localizar a los vecinos de Carlos y cuánto recorrerlos? — *Localizar* es `lista[índice_de_Carlos]`: un acceso de array, **O(1)**. *Recorrer* es iterar su lista, que tiene 1 elemento (solo Dana): **O(grado de Carlos) = O(1)**. El matiz: localizar es O(1) *siempre*; recorrer es O(grado), y el grado medio es lo que manda. En la matriz, en cambio, localizar *y* recorrer coinciden en ser caros: para obtener a los vecinos de Carlos barres su fila completa, **O(V)** aunque tenga 1 vecino.

**2. ¿Por qué un BFS ama la lista de adyacencia y aborrece la edge list?** — Un BFS, en cada paso, toma un nodo y enlista a sus vecinos para visitarlos. En la lista de adyacencia, «vecinos de u» es O(grado u), así que el BFS completo es O(V+E). En una edge list, «vecinos de u» te obliga a escanear *todas* las aristas para quedarte con las que tocan a u: O(E) por nodo, y el BFS degrada a O(V·E). La edge list guarda el grafo de forma compacta, pero no organiza la información *por origen*, que es justo el eje de acceso de un recorrido. (Esta es la semilla del cap. 4 del Vol.II, donde el BFS se aterriza sobre `MemoryStore`.)

## Ejercicios propuestos

**Esencial (recordar/aplicar — retrieval).** De memoria, sin mirar el capítulo: dibuja el grafo «Ana→Bruno, Ana→Carlos, Bruno→Carlos, Carlos→Dana» de tres maneras — (a) matriz de adyacencia, (b) lista de adyacencia con dirección saliente, (c) edge list. Luego coteja tus tres figuras contra el modelo mental del §2.3 y verifica que las tres representan exactamente el mismo grafo. Criterio de evaluación: la arista Bruno→Carlos debe aparecer como `1` en la celda de matriz, como un elemento en la lista de Bruno, y como un par `(Bruno, Carlos)` en el edge list.

**Intermedio (analizar — calcula la memoria).** Dado un grafo de **V = 1.000.000** nodos y **E = 4.000.000** aristas (grado medio 4, ids de 32 bits = 4 bytes), calcula el coste aproximado de memoria de cada representación y di cuáles caben en una máquina de ~16 GB de RAM:

- Matriz de adyacencia con `u32` por celda.
- Matriz de adyacencia con 1 bit por celda.
- Lista de adyacencia (datos: un `EdgeId` de 4 bytes por arista saliente = 4 M targets, más la cabecera `Vec` por nodo).
- Edge list (un par `(u,v)` de 8 bytes por arista).
- CSR (`offsets`: (V+1)·4 B; `targets`: E·4 B).

*Pistas (graduadas)*: (1) la matriz es V² × bytes_por_celda; (2) el CSR es la suma de sus dos arrays; (3) la lista de adyacencia, al ser la misma información que el CSR pero con una cabecera de `Vec` y una alocación por nodo, nunca puede ser más barata que el CSR — en la práctica lo supera en decenas de MB solo por las cabeceras. *Solución de referencia*: matriz u32 ≈ **4 TB**; matriz a 1 bit ≈ **125 GB**; CSR ≈ (1 M + 1)·4 + 4 M·4 ≈ **20 MB**; edge list ≈ 8·4 M ≈ **32 MB**; lista de adyacencia ≈ CSR + overhead por nodo (≈ 24-32 B) ≈ **40-50 MB**. Veredicto: las dos matrices no caben; de las que caben, el CSR es la más ceñida y contigua.

**Experto (crear — anclaje al cap. 8 y cap. 14).** Una base de datos de grafos te dice su perfil de carga: sus consultas dominantes son **recorridos BFS desde muchos orígenes**, y además ejecuta un **PageRank periódico** que necesita, para cada nodo, sumar la influencia de *quienes le apuntan a él* (vecinos entrantes). Diseña la representación en memoria de su adyacencia: ¿qué estructura(s) eliges, necesitas dirección inversa, y contra qué alternativa lo justificas? *Pistas*: conviene una lista o CSR por cada dirección para que ambos recorridos sean baratos; descarta la matriz por O(V²) y la edge list por «vecinos de u» O(E). *Criterio*: argumentas en contra de al menos una alternativa y conectas tu decisión con la estructura `adj_out` + `adj_in` del `MemoryStore` (cap. 8, `cap08_graph_store.rs` líneas 79-80) y con el par CSR forward / backward del cap. 14.

## Para profundizar

- **Yale Sparse Matrix Package (YSMP), 1977** — informes YALEU/DCS/RR-112 y RR-114 (Eisenstat, Gursky, Schultz, Sherman). El origen del CSR; alto valor histórico, difícil de encontrar digitalizado.
- **«Introduction to Algorithms» (CLRS), cap. 22** — la *convención* canónica de las listas de adyacencia en los libros de algoritmos, con su análisis O(V+E) frente a la matriz O(V²). De aquí saca el Vol.I la notación.
- **«The Art of Computer Programming» (Knuth), vol. 4A, §7.4.1** — las formas de representar grafos en memoria y el «traverse» como operación definitoria que decide qué estructura sirve.
- **«Database Internals» (Alex Petrov), caps. 2-3** — cómo un motor real combina el layout de datos en memoria y en páginas; el puente directo hacia el cap. 14.
- **Cap. 8 del Vol.II** (`cap08_graph_store.rs`) — el código que *ancora* este capítulo: lee `MemoryStore` y ve `adj_out` / `adj_in` en su hábitat, junto a las dos direcciones y a la duplicación síncrona al borrar.
- **Cap. 14 del Vol.II** — donde el CSR se construye, se verifican sus invariantes y se persiste sobre páginas; esta lectura es el «después» que este capítulo prepara, con las mismas cifras de memoria que usas en el ejercicio intermedio.
- **Cap. 3 del Vol.II** — donde el ID deja de ser «un `usize`» y pasa a ser una clave estable que sobrevive al crash y al reciclaje de espacio; la pregunta de CORPUS sobre `slotmap`.

## Mini-diálogo: en la cafetería de la universidad

> — O sea, que la matriz de adyacencia que me enseñaron en algoritmos... ¿no sirve?
>
> — Sirve perfectamente. Sirve para responder «¿existe la arista de Ana a Bruno?» en O(1). Lo que no sirve es para el trabajo que hace un motor de grafos: «¿quiénes son todos los vecinos de Ana?» en un grafo de un millón de nodos. Eso es lo que un motor ejecuta millones de veces al día. Y para eso, una matriz de un millón de millones de celdas no solo es lenta: no cabe.
>
> — Entonces la lista de adyacencia es la buena, y ya está.
>
> — Es la buena para recorrer, que es lo que domina un BFS y un PageRank. Pero no para «¿existe esta arista?» puntual, ni para exportar el grafo entero — eso es edge list. Ese es el punto: no hay «la buena». Hay la buena *para lo que tu base de datos ejecuta de verdad*. La pregunta es la que elige, no la tabla.
>
> — ¿Y cuándo acabamos decidiendo?
>
> — Cuando dejes de preguntarte «¿cómo guardo esto?» y empieces a preguntarte «¿qué pregunta le voy a hacer a esta base de datos un millón de veces?». Ahí la representación se elige sola — y te darás cuenta de que estabas guardando una arista, pero pensando en un vecindario.

---

*(Próximo capítulo: 3 — Identidad, referencias y datos estables. Aquí vimos que la forma de guardar depende de la operación; ahora veremos la moneda que toda representación usa para nombrar a los nodos — el ID — y por qué un `usize` estable es una cosa y un índice que se re-numera es otra bien distinta.)*