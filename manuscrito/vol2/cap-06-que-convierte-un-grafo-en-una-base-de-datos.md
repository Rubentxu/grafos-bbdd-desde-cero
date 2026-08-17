# Capítulo 6 — Qué convierte un grafo en una base de datos

> *«En RAM un grafo es lo que calculas; en una base de datos es lo que persiste, lo que consultas y lo que guardas aunque todo se apague. La diferencia no es el algoritmo: es la persistencia, y todo lo que cuelga de ella.»*

## 6.0 La anécdota de la esquina

En 1970, Edgar Codd publicó en *Communications of the ACM* once páginas que, medio siglo después, siguen siendo la frontera entre «datos que se usan» y «datos que se guardan». El artículo se llamaba *«A Relational Model of Data for Large Shared Data Banks»* (CACM, vol. 13, núm. 6, pp. 377-387), y su tesis de apertura era una declaración de independencia: los usuarios de una gran base de datos «deben estar protegidos de tener que saber cómo se organizan los datos dentro de la máquina». En otras palabras: el *cómo* guardas los datos no debería apretar el *qué* puedes preguntarles.

Pero Codd escribió teoría. La demostración vino seis años después con **System R**, el prototipo de IBM — firmado por Astrahan, Blasgen, Chamberlin, Eswaran, **Jim Gray** y nueve colegas más en *ACM Transactions on Database Systems* (1(2), pp. 97-137, 1976). System R no era solo un lenguaje elegante: era un motor que **guardaba**, **recuperaba** y **mantenía consistente** los datos frente a fallos y a varios usuarios a la vez. Tenía logging, recovery y control de acceso en un entorno de actualización compartida. System R fue la primera vez que el mundo vio a un DBMS funcionando de verdad con las cuatro promesas que Codd solo había teorizado.

¿Por qué abre así un capítulo sobre grafos? Porque LiraDB — el motor que este libro entero construye — va a rehacer ese camino, pero para grafos. Y el punto es que hay un instante concreto en que un «grafo que calcula» se convierte en una «base de datos que guarda». Ese instante es lo que este capítulo quiere que veas venir, cincuenta años de historia en la mano.

## 6.1 Objetivo

Hasta aquí, en la Parte I, aprendiste a *pensar en grafos*: a definirlos (cap. 1), a representarlos en memoria — edge list, lista de adyacencia, CSR (cap. 2), a darles identidad estable (cap. 3), a recorrerlos con BFS (cap. 4) y con DFS/componentes/topológica/SCC (cap. 5). Todo eso es extraordinario, y todo eso tiene un límite silencioso: **vive en RAM, y la RAM se vacía al cerrar el programa.**

Este capítulo es el puente. Su objetivo es responder a la pregunta que da título al libro y que se convierte en tu propósito durante los próximos treinta y cuatro capítulos:

> **¿Qué convierte un grafo en una base de datos?**

Y para responderla, vamos a hacer dos cosas que quizá no esperabas:

1. **Cerrar la Parte I**, recogiendo en un mapa lo que ya sabes — no para repetirlo, sino para mostrarte que eras el pasajero de la primera estación de un viaje más largo.
2. **Abrir la Parte II**, presentando el esqueleto del motor que el resto del libro construye, organizado en **cinco pilares** que irás levantando, capítulo a capítulo, parte a parte.

Al terminar, sabrás exactamente por qué se construye LiraDB, qué piezas tiene, y qué capítulo del libro levanta cada pieza. El resto de la obra será ir llenando la casilla donde este capítulo puso la etiqueta.

## 6.2 Problema

Tomemos literalmente todo lo que has construido en los cinco capítulos anteriores. Tienes un `PropertyGraph` incipiente — o, más modestamente, un `Vec` de listas de adyacencia. Lo pueblas con un millón de nodos y tres millones de aristas. Ejecutas un BFS y encuentras en qué componente está cada nodo — como en el cap. 5. Después haces `exit` y cierras el terminal.

Y ahí está el problema, con toda su crudeza:

> **El grafo que tardaste en construir no está en ninguna parte.**

Se fue con el proceso. De «el mundo conocido» del programa ya no queda ni un bit. Si querías mantener el resultado de tu BFS, o los nodos que añadiste, o las aristas que ponderaste, tuviste que guardarlo TÚ, a mano, a un formato que inventaste, sin que nada te ayudara a volver a leerlo.

Detente un segundo y mira lo que implica. Has invertido cinco capítulos en construir algo extraordinariamente elegante: el grafo vestido del cap. 1, con la representación adecuada del cap. 2, con ids estables del cap. 3, con algoritmos eficientes de los caps. 4-5. Pero todo eso es un programa: un programa que, cuando termina, deja el mundo exactamente igual que como lo encontró, salvo por el calor residual del CPU. Si te pidiera un cliente que repitiese la consulta mañana con los mismos datos, tendrías que volver a empezar desde cero. Y si te pidiera añadir un nodo nuevo, no podrías porque el nodo anterior ya no existe.

Ahora agrava el problema por los cinco costados:

- **El tiempo.** Ese grafo no desapareció por descuido: desapareció porque la RAM es volátil por diseño. No hay ninguna estructura de datos en memoria que sobreviva al apagado; para eso existe el disco, y nadie te ha enseñado aún a hablarlo.
- **El tamaño.** Tu grafo cabe en RAM hoy. Uno real — el grafo social del planeta, el de los vuelos del mundo, el de los papers citados entre sí — no cabe. ¿Recorres un grafo que desborda la memoria leyendo de un fichero? Ese problema nuevo no existe en toda la Parte I.
- **La pregunta.** En RAM puedes *calcular* «¿está conectado A con B?» recorriendo. Pero sobre un grafo persistente quieres *preguntarlo en un lenguaje*, que el motor averigüe por ti, y que la respuesta sea coherente aunque otros estén escribiendo a la vez.
- **La confianza.** Si un proceso se cae por la mitad de una escritura, ¿el grafo queda a medias? ¿corrupto? ¿recuperable? Una «estructura en memoria» ni se plantea eso — y por eso no es una base de datos.
- **El recorrido.** Tus algoritmos del Vol.I eran bellos sobre `Vec<Vec<NodeId>>`, pero ahora los necesitas sobre datos que viven en disco. ¿Cómo recorrer un grafo que no cabe en RAM sin leerlo todo?

La raíz del problema es la misma en todos los casos: **las estructuras de datos de la Parte I responden a «¿cómo represento y recorro esto en RAM?», pero ninguna responde a «¿cómo lo guardo, lo pregunto, lo protejo y lo recorro cuando deja de caber en RAM?».** Ese segundo grupo de preguntas es lo que convierte un grafo en una base de datos.

Y aquí está la ironía que importa: **las preguntas del segundo grupo no son «más difíciles» que las del primero — son de otra naturaleza**. Una lista de adyacencia en RAM y un fichero en disco no son «lo mismo con más memoria»: el disco lee bloques, no bytes; el fichero no se puede «recorrer en o(n)» sin saber qué hay al principio; y un fallo eléctrico puede dejar bytes a medias. Esto obliga a re-diseñar el modelo para un medio con reglas distintas. Esa es la razón por la que el libro tiene Partes enteras dedicadas a «persistir» y a «recorrer sobre persistido» — no son adornos, son respuestas a problemas que la Parte I ni siquiera sabía que existían.

## 6.3 Modelo mental: el viaje de cuatro estaciones

Imagina un ferrocarril con cuatro estaciones. En cada una se sube algo que la anterior no podía llevar, y el destino final es el motor que construiremos. Es el mapa completo de lo que este libro hace.

```
�──────────────────────────────────────────────────────────────────────────┐
│                            EL VIAJE DE LiraDB                             │
│          (lo que ya hicimos  →  lo que este libro construye)             │
└──────────────────────────────────────────────────────────────────────────┘

 ESTACIÓN 1            ESTACIÓN 2            ESTACIÓN 3            ESTACIÓN 4
 GRAFO EN RAM          GRAFO EN DISCO        GRAFO CONSULTABLE     GRAFO TRANSACCIONAL
 -----------           ---------------       -----------------     -------------------
 Vec / HashMap / CSR   páginas + pager        un LENGUAJE que       nadie se pierde a
 lista de adyacencia   + buffer pool +        pide datos (LiraQL)   medio escribir:
 BFS / DFS / SCC       índices (Parte III)    (Parte IV)            WAL, recovery,
 (PARTE I)             (caps. 11-16)          (caps. 17-21)         MVCC (Parte VI)
        |                     |                      |               (caps. 27-30)
        └─── muere al ────────┴───¿le pregunto?─────┴──¿y si fallo?──┘
             cerrar el        se recorre sin         se responde        sobrevive al crash
             proceso          cargar todo en RAM     con un motor

  + VAGÓN LATERAL:   ALGORITMOS SOBRE EL GRAFO PERSISTENTE  (Parte V, caps. 22-26)
  + VAGÓN DE CIERRE: EL MOTOR COMO PRODUCTO REAL            (Partes VII-VIII, caps. 31-40)
```

El viaje es exactamente la historia de la informática de datos: primero aprendiste a representar (estación 1), ahora toca *persistir* (estación 2), luego *consultar* (estación 3) y finalmente *ser fiable bajo concurrencia* (estación 4). Cada estación añade lo que la anterior no podía:

| Estación | Añade | Responde | Coste nuevo que introduce |
|---|---|---|---|
| 1 · RAM | velocidad, recorrido | «¿cómo represento/recorro?» | volatilidad: muere al salir |
| 2 · Disco | durabilidad | «¿cómo guardo y encuentro?» | lentitud del disco, formato |
| 3 · Consulta | un lenguaje | «¿cómo le pregunto al motor?» | parser, plan, ejecución |
| 4 · Transaccional | consistencia | «¿cómo no corromperlo?» | logging, bloqueo, aislamiento |

Fíjate en lo importante para el diseño pedagógico: **cada estación depende de la anterior.** No puedes consultar un grafo que no está persistido; no puedes transaccionar un grafo que no se consulta. Esto fija el orden del libro. La Parte I te dejó en la estación 1. A partir del cap. 7, pasamos a la 2.

El viaje tiene, además, dos vagones laterales que cuelgan de él:

- **El vagón de algoritmos** (Parte V, caps. 22-26) engancha en la estación 2: una vez que el grafo vive en disco, los recorridos del Vol.I — BFS, DFS, Dijkstra, PageRank — se vuelven preguntas caras (no cabe todo en RAM) y necesitan sus propias técnicas (proyección, streaming, frontiers).
- **El vagón de producto** (Partes VII-VIII, caps. 31-40) engancha al final: cuando el motor ya guarda, consulta, calcula y se recupera, queda empaquetarlo como herramienta (CLI, importadores, benchmarks, observabilidad) y mirar el horizonte (producción, columnar, distribución).

Estos dos vagones no son estaciones porque no añaden una *capacidad* nueva al motor — son aplicaciones o materializaciones del viaje. Un GDBMS sin ellos existe; sin las cuatro estaciones, no.

### La idea que ordena todo el capítulo

> **Las cuatro capabilidades que toda base de datos necesita — persistencia, consulta, transacciones e índices — son, exactamente, las respuestas a las cuatro preguntas que una estructura en memoria NO sabe responder.** Levantar un motor es subir las cuatro estaciones, una por una.

### Un segundo diagrama: la historia de los modelos de datos

El grafo no es una rareza: es la **cuarta generación** de un árbol de modelos. Codd lo criticó todo desde ahí dentro:

```
MODELOS DE DATOS (una historia en 4 generaciones)
─────────────────────────────────────────────────
 1. JERÁRQUICO ............ árboles de ficheros predeterminados
 2. RED (CODASYL) ......... grafos de punteros entre registros
 3. RELACIONAL ............ tablas + claves exteriores  (Codd 1970; System R 1976)
 4. GRAFO (GDBMS) ......... nodos + aristas de 1ª clase con props  (Neo4j 2007/2010; GQL 2024)
       ▲
       └── LiraDB vive AQUÍ, heredando las tres generaciones anteriores
```

Codd, en 1970, dedicaba su sección 1 a explicar por qué los modelos de árbol y red eran insuficientes. El relacional quitó los punteros. Y el grafo, cuarenta años después, los trajo de vuelta — pero **tipados, con identidad y con propiedades**, no como flechas mudas. Ese es exactamente el hilo que el cap. 7 continuará con el property graph y el `Value`.

Hay un detalle histórico que merece no perderse: la segunda generación, el **modelo de red CODASYL** (Conference on Data Systems Languages, 1969-71), ya era literalmente un *grafo*: registros conectados por punteros (`SET` y `OWNER`). Lo que le faltó fue justamente lo que el relacional ganó: **independencia de los datos** (Codd 1970, §1) — poder preguntar sin tener que saber cómo navegan los punteros. El modelo de grafos de 2007 (Neo4j) recogió esa idea y la vistió de propiedades: la arista dejó de ser un puntero mudo y pasó a ser un dato de primera clase con tipo, origen, destino y propiedades. La lección es importante para LiraDB: el grafo *no tira a la basura* las ideas del relacional, las hereda. Por eso el cap. 7 (modelo de datos) abrirá citando a Codd junto a Robinson/Webber/Eifrem — y por eso el cap. 28 (WAL) se parecerá, en su forma, al logging de System R.

## 6.4 Primera solución (la que ya conoces y su espejo)

La «primera solución» al problema de *guardar* es la que cualquier novato (nosotros incluidos) aplicaría al día uno, y es la solución con la que ya llevas cinco capítulos conviviendo: **mantener el grafo en memoria y, cuando toque terminar, volcarlo a un fichero a mano.**

```
// Primera solución (ingenua, y la tentación natural):
//     1. construye el grafo en RAM (Parte I)
//     2. al terminar: fs::write("graph.bin", serializa_grafo(&g))
//     3. al abrir:     let g = deserializa_grafo(&fs::read("graph.bin"))
```

Suena razonable. ¿No basta? No. Porque ese volcado es a la vez la imagen deformada de lo que viene: un fichero plano al que nadie le pregunta nada. Para «guardar un grafo» en canalización, hemos de reconocer que la Parte I nos dio el *material* de la estación 1, pero aún no tenemos billete para la 2.

Date cuenta de algo: si esa «primera solución» bastara, no habría existido System R. En 1976, con todo el conocimiento de Codd ya en la calle (el artículo de 1970 tenía seis años), el equipo de IBM se embarcó en un proyecto de años, no para «volcar tablas a fichero», sino para construir un sistema que **guardara, consultara y se recuperara**. La diferencia entre «serializar al final» y «persistir durante toda la vida del programa» es exactamente la diferencia entre un script y una base de datos. Si quieres convencerte, mira cuánto tardó System R en aparecer: seis años desde Codd hasta el primer DBMS relacional que funcionaba. Eso no se invierte en `fs::write`.

## 6.5 Sus límites

La solución del volcado plano se rompe en cuanto dejas de guardar y empiezas a *querer algo*:

1. **Consulta imposible.** Para responder «dame los amigos de Ana y sus edades», tienes que leer TODO el fichero, deserializar todas las entradas, y filtrar en tu programa. No hay lenguaje, no hay optimizador, no hay índice: cada pregunta es un escaneo completo a mano.
2. **Índices inexistentes.** Sin índices, encontrar el nodo con `city == "Oporto"` es recorrer el millón de nodos. El índice (cap. 15, hash + B+ tree) es lo que permite saltar a los datos; hoy estás condenado a la búsqueda lineal.
3. **Sin transacciones, sin recuperación.** ¿Escribiste el fichero y el proceso se cayó al 60 %? El fichero queda a medias, y nadie sabe cómo volver a un estado coherente. Es el territorio de la Parte VI (caps. 27-30), que hoy no sabes ni nombrar.
4. **Borrosidad del formato.** Cómo guardaste qué: los strings con longitud-prefijo, si es little-endian, cómo distinguir `Int` de `String`… Es el cap. 9 (encoding) y el modelo del cap. 7. Si lo improvisas, te perseguirá durante todo el libro.
5. **El viaje en disco es distinto.** El disco no lee nodos; lee **páginas** (cap. 11). La estructura óptima en RAM (CSR) no es la óptima en disco. Estás a punto de descubrir (caps. 11-16) que persistir no es `serialize` del modelo en memoria, sino re-diseñar el modelo para un medio distinto.

El límite de fondo: **el volcado guarda «cómo pensabas en RAM», no «una base de datos».** Una base de datos es aquello a lo que se le pueden hacer consultas, encontrar sin escanear, y confiar aunque algo falle. Nada de eso aparece en `fs::write`.

## 6.6 Solución evolucionada: el esqueleto en cinco pilares

La solución evolucionada es **LiraDB** — y este es el momento del capítulo donde la declaramos. No es una sola técnica; es un **esqueleto de motor** con cinco pilares, cada uno respondiendo a una pregunta que la solución ingenua dejó sin respuesta, y cada uno construyéndose en una parte concreta del libro. Apréndetelos bien: son el mapa de todo lo que viene.

### Pilar 1 — Persistencia (guardar en disco) — Parte III, caps. 10-16

> *Pregunta que responde:* ¿cómo sobrevive el grafo al cierre del programa y cómo lo encuentro sin escanearlo todo?

Sin persistencia no hay base de datos, solo un grafo que se despide al cerrar. Ese pilar es la estación 2 del viaje y de él dependen todas las demás. Se construye en la Parte III, y una pieza ya la viste de lejos en el capítulo 11-piloto: la **página**. El capítulo 11 te enseñó *por qué el disco no lee bytes sino bloques*, y cómo una **slotted page** guarda registros de longitud variable sin corromperse. Alrededor de esa idea se levantan la persistencia append-only (cap. 10), el gestor de páginas (`Pager`, cap. 12), el buffer pool (cap. 13, LRU/Clock — clave en grafos power-law), el almacenamiento de adyacencias en disco (cap. 14, CSR persistente) y los índices (cap. 15). *Ejemplo que volverá:* la consulta «muestra los nodos con `city = Oporto`», imposible sin un índice que encuentre sin recorrer el millón de nodos (cap. 15).

Una sutileza importante: la persistencia NO entra al programa como una «capa» aparte que se le añade al grafo de la Parte I. Obliga a **re-diseñar el modelo para un medio distinto**. La lista de adyacencia `Vec<Vec<NodeId>>` que era óptima en RAM (cap. 2) se aplastará en CSR para ser contigua en disco (cap. 14); las propiedades `HashMap<String, Value>` se serializarán con prefijo de longitud (cap. 9); los nodos y aristas dejarán de ser objetos Rust y se convertirán en bytes con una cabecera que detecta corrupción (cap. 11). Esa es la sensación de «el viaje en disco es distinto» del §6.5: persistir no es `serialize` del modelo en memoria, es un modelo nuevo que vive en páginas.

### Pilar 2 — Consulta (un lenguaje para pedir datos) — Parte IV, LiraQL, caps. 17-21

> *Pregunta que responde:* ¿cómo le pido datos al motor con un lenguaje, en vez de hacer yo el recorrido a mano?

Guardar no basta si no puedes preguntar. La Parte IV construye **LiraQL**, tu mini-lenguaje de consulta de grafos, en la estirpe de MATCH-WHERE-RETURN (el subconjunto útil de Cypher que todos los GDBMS comparten — Francis et al. lo documentaron en SIGMOD 2018, y el estándar GQL lo formalizó en ISO/IEC 39075:2024). Levantar un lenguaje es un tour-de-force en miniatura: diseñas la gramática (cap. 17), construyes el lexer y el parser (cap. 18), pasas del árbol sintáctico al plan lógico (cap. 19), lo ejecutas con el modelo Volcano (cap. 20) y añades un optimizador pequeño pero real (cap. 21). *Ejemplo que volverá:* `MATCH (a:Person)-[r:KNOWS]->(b) WHERE a.name = "Ada" RETURN b.name` — la primera consulta que pide el grafo a LiraDB.

Una sutileza: este pilar se construye sobre el anterior (no puedes consultar lo no persistido), pero también introduce una **bisagra de diseño** importantísima que conviene no perder: el trait `GraphStore` del cap. 8. La consulta no llama a disco; llama al trait, que decide si la respuesta sale de RAM, de páginas cacheadas o del disco. Esa indirección es lo que permitirá que el motor madure (cambiar el `Pager`, añadir un índice, mover una adyacencia a CSR) sin tocar ni una línea de LiraQL. Si el lector se salta el cap. 8, no entiende por qué la Parte IV no se derrumba cuando la Parte III cambia.

### Pilar 3 — Algoritmos sobre datos persistentes — Parte V, caps. 22-26

> *Pregunta que responde:* ¿qué pasa cuando el algoritmo de la Parte I se ejecuta sobre un grafo que ya no cabe en RAM?

Este pilar es lo que distingue a un GDBMS de una biblioteca: no solo guarda y pregunta, **calcula**. Recorriste Dijkstra, Bellman-Ford, A*, PageRank y Louvain en el Vol.I sobre grafos de juguete en RAM. Ahora los ejecutas sobre el grafo persistente, leyendo pesos de las **propiedades de las aristas** (que el cap. 7 te dará el modelo para guardar). *Ejemplos que volverán:* `SHORTEST PATH FROM node:1 TO node:42 WEIGHT relationship.distance` (cap. 22, con la lección de que el camino con menos saltos no es el más barato); `PageRank` para encontrar «quién es central» en la red (cap. 24, con su lección de que importa *quién* te enlaza, no cuántos); `¿están conectados?` vía componentes (revisitando el cap. 5). Y, para rematar, **no agotar la RAM** al ejecutarlos sobre grafos enormes — proyección, streaming y frontiers (cap. 26).

La sutileza algorítmica del pilar es la que ya anunció el cap. 4 al hablar de BFS por fronteras: si el grafo no cabe en RAM, un BFS «a lo bruto» que cargue todos los vecinos de cada nodo es inviable. La Parte V reusa la idea de ola/frontera (cap. 4) pero, además, añade técnicas de **proyección** (un `GrafoPonderado` derivado del `GraphStore`, cap. 26) y de **presupuestos** (parar el recorrido tras cierto número de saltos o aristas leídas) para que un algoritmo no se ahogue en disco. Es exactamente el patrón «calcular sin cargar todo», y se convierte en el contrato del pilar.

### Pilar 4 — Fiabilidad (transacciones, WAL, recuperación, concurrencia) — Parte VI, caps. 27-30

> *Pregunta que responde:* ¿qué pasa si el proceso se cae a mitad de escritura, o si dos hilos escriben a la vez?

Este es el pilar que System R le mostró al mundo en 1976 (logging y recovery en un entorno de actualización compartida) y que separa a un motor «que funciona» de un motor «en el que confías». La Parte VI te pedirá que digas qué significa una transacción ACID de verdad (cap. 27), que construyas el write-ahead log (WAL, cap. 28), que recuperes el estado tras un fallo (cap. 29) y que manejes snaps, concurrencia y aislamiento con MVCC (cap. 30). Añade un guiño que ya tuviste: el borrow checker del trait `GraphStore` (cap. 8) es el germen del «único escritor» que aquí se vuelve transacción. *Ejemplo que volverá:* `BEGIN; ...; COMMIT;` (o la recuperación de un grafo que quedó a medias tras un `kill -9` a mitad del WAL).

Y aquí se cierra el bucle con un detalle precioso: la Parte VI también **confirma la promesa del cap. 3**. Recordarás que el cap. 3 introdujo `(slot, generation)` para que un id no se reciclara nunca. La Parte VI (caps. 28-30) demuestra que esa promesa sigue siendo cierta después de un crash y bajo concurrencia: la generación no se reinicia, el WAL la registra junto al dato, y el recovery la respeta. Sin la Parte VI, la promesa del cap. 3 era un pacto sobre el papel; con ella, es un contrato verificable.

### Pilar 5 — El motor real (CLI, importación, producto técnico) — Partes VII-VIII, caps. 31-40

> *Pregunta que responde:* ¿cómo se convierte este código en algo que alguien usa de verdad?

Un motor no vive en un `lib.rs` académico: vive en una **CLI** (`liradb`, cap. 31), en la importación/exportación de datos reales (CSV, JSONL, GraphML, cap. 32), en pruebas, benchmarks, perfilado y observabilidad (caps. 33-35), y en una arquitectura final documentada (cap. 36). Y una vez que LiraDB es sólido y manejable, el libro te empuja al horizonte: qué necesitaría una base de datos de producción (cap. 37), el almacenamiento columnar y la ejecución vectorizada (cap. 38), joins y consultas cíclicas (WCOJ, cap. 39) y la distribución de una base de datos de grafos (cap. 40). Es la estación final: del juguete al instrumento.

Este pilar es el que la Parte I no podía ni soñar: por muy bueno que sea tu BFS en RAM, sin CLI nadie puede invocarlo desde su terminal, y sin importadores nadie carga datos del mundo real en él. Las Partes VII-VIII son las que convierten un programa que pasa tests en una herramienta que alguien usa. Son, además, la *prueba* de que los cuatro pilares anteriores funcionan integrados: si la CLI responde a una consulta en milisegundos, sabes que persistencia + consulta + algoritmo + fiabilidad están haciendo su trabajo al unísono.

---

Fíjate en cómo cierran los cinco pilares sobre los cuatro clásicos: **persistencia (1)** y **transacciones (4)** son puros en la lista clásica; **consulta (2)** une la «consulta» clásica con el lenguaje; **índices** quedan dentro de la persistencia (cap. 15); y **algoritmos (3)** y **motor real (5)** son los dos aditamentos que hacen de LiraDB un *graph* DBMS y un *producto*, no una relacional disfrazada. Por eso este capítulo los presenta como el esqueleto del motor: porque son exactamente las piezas que los próximos treinta y cuatro capítulos levantan.

### Tabla resumen: pilar ↔ pregunta ↔ Parte ↔ ejemplo que volverá

| Pilar | Pregunta que responde | Parte del libro | Caps. | Ejemplo que volverá |
|---|---|---|---|---|
| 1 · Persistencia | ¿sobrevive al cierre? ¿cómo encuentro sin escanear? | III | 10-16 | `MATCH (n) WHERE n.city = 'Oporto'` (índice, cap. 15) |
| 2 · Consulta | ¿cómo le pregunto con un lenguaje? | IV | 17-21 | `MATCH (a:Person)-[:KNOWS]->(b) RETURN b.name` (cap. 17) |
| 3 · Algoritmos | ¿qué pasa sobre datos persistentes? | V | 22-26 | `SHORTEST PATH FROM 1 TO 42 WEIGHT r.distance` (cap. 22) |
| 4 · Fiabilidad | ¿qué pasa si fallo a mitad o escriben dos a la vez? | VI | 27-30 | `BEGIN; ...; COMMIT;` y recovery tras `kill -9` (cap. 28) |
| 5 · Motor real | ¿cómo lo usa alguien? | VII-VIII | 31-40 | `liradb load data.csv; liradb query "MATCH ..."` (cap. 31) |

## 6.7 Por qué así (los porqués de la arquitectura)

Este capítulo no tiene código, pero sí un porqué de arquitectura detrás de cada elección. Grilla rápida:

- **¿Por qué definir «base de datos» por cuatro capabilidades y no por «guardar en un fichero»?** Porque el criterio define el trabajo. Si «guardar a fichero» fuera suficiente, ya habrías terminado con `fs::write`. La consulta, la transacción y el índice son lo que separa un volcado de un motor. Es el mismo criterio con el que Codd separó «saber cómo se guardan los datos» de «poder preguntarles» (CACM 13(6), 1970), y con el que System R (Astrahan et al., TODS 1976) lo demostró en la práctica. Modo de fallo si no lo hacemos: el lector cierra este capítulo creyendo que «guardar en un fichero» ya es una base de datos, y cada capítulo futuro de las Partes III-VI le parecerá un sobreesfuerzo.
- **¿Por qué presentar el grafo como 4.ª generación?** Porque la continuidad explica por qué LiraDB hereda del relacional: las lecciones de Codd (tipar, identidad, independencia de datos) y de System R (logging, recovery, concurrencia) no se reinventan; se re-aplican a un modelo nuevo. El cap. 7 lo confirmará al citar a Codd y a Neo4j en el mismo párrafo. Sin esta genealogía, el lector trata a LiraDB como «otro grafo más» y pierde la conexión con cincuenta años de ingeniería de datos.
- **¿Por qué el orden del libro (persistir → consultar → algoritmos → fiabilidad → producto)?** Porque las dependencias lo exigen: no consultas lo que no está persistido, no transaccinas lo que no se consulta, no haces producto de algo inmaduro. Cada pilar es prerequisito del siguiente. Es el viaje de las cuatro estaciones, y no se pueden subir en otro orden. La forma de comprobarlo: si intentas escribir la consulta del cap. 17 sin la persistencia del cap. 11, no tienes dónde leer.
- **¿Por qué LiraDB es *embedded* y monolítica, no un servidor de red?** Por enseñanza. La complejidad de la red y la distribución se pospone deliberadamente a los caps. 37 y 40; primero se construye a fondo el núcleo (almacenar + consultar). Es la ruta que el prólogo llamaba «embedded, didáctico», y es lo que hace factible el proyecto. Construir un servidor de red desde el día uno antepone la dificultad de red a la de almacenamiento — y eso es exactamente lo que un libro paso a paso NO puede permitirse.
- **¿Por qué añadir «algoritmos» como pilar propio y no como aplicación externa?** Porque un GDBMS se justifica por las preguntas que una relacional no responde barato («¿a cuántos saltos?», «¿quién es central?», «¿qué comunidades hay?»). Si esos algoritmos viven en una herramienta externa, no tienes un motor; tienes una biblioteca de grafos con SQL pegado. Esa es la línea divisoria entre un GDBMS y Neo4j-solo-como-biblioteca: los algoritmos son parte del contrato del motor.

## 6.7.1 Cómo lo hace una BBDD real (lo que este capítulo aprende de la industria)

Es útil anclar cada pilar a un motor que ya lo resolvió, para que el lector vea que no estamos inventando sino re-aplicando lecciones probadas:

- **Neo4j** (el GDBMS de referencia desde 2007): persiste con su formato propio (basado en nodos + relaciones + propiedades en archivos), consulta con Cypher, ejecuta algoritmos vía APOC y sus librerías internas, transacciona con su propio control de concurrencia, y expone una API de producto madura (Neo4j Browser, Neo4j Bloom, drivers para 8+ lenguajes). Es el caso canónico de los **5 pilares** que este capítulo describe: cada uno tiene su equivalente en Neo4j. (Fuente: Robinson, Webber & Eifrem, *Graph Databases*, 2.ª ed., cap. 8.)
- **TigerGraph** (el GDBMS de producción con mejor rendimiento en grafos densos): usa **CSR en memoria** como formato canónico (lo que el cap. 14 de LiraDB replicará), un lenguaje propio (GSQL), y una arquitectura «single-node embebible o distribuida». Su elección de CSR confirma una de las decisiones del cap. 2 de LiraDB: para recorrer, CSR gana a `Vec<Vec>` en producción. (Fuente: *TigerGraph Documentation*, architecture overview.)
- **Memgraph** (la apuesta por in-memory + streaming): mantiene el grafo en RAM pero con **persistencia transaccional** vía WAL (lo que la Parte VI de LiraDB enseñará a construir), expone Cypher, y se publicita como «base de datos en streaming» — exactamente el rol del vagón lateral de la Parte V en este libro.
- **Amazon Neptune** y **JanusGraph** (los GDBMS serverless/distribuidos): añaden el vagón «distribución» del cap. 40 sobre un núcleo de GDBMS clásico. Neptune usa un backend de triples (más cercano a RDF) pero expone tanto GQL como SPARQL; JanusGraph es la opción open-source compatible con el ecosistema de Apache TinkerPop/Gremlin.
- **PostgreSQL + Apache AGE** (la prueba de que un GDBMS puede nacer como extensión de un RDBMS): añade el modelo de grafos como una capa sobre tablas relacionales, aprovechando toda la fiabilidad transaccional que PostgreSQL ya tiene. Es la prueba industrial de que «la Parte VI (fiabilidad) ya estaba resuelta» — solo faltaba ponerle encima el property graph. (Fuente: documentación de Apache AGE.)
- **SQLite** (la prueba inversa: una BBDD sin servidor que sí implementa los 4 pilares): un único fichero, transaccional (ACID), con índices hash y B+ tree, y con un lenguaje SQL. No es de grafos, pero demuestra que los pilares son ortogonales al modelo de datos: la misma ingeniería de páginas + WAL + buffer pool sirve para grafos que para tablas.

Lo que LiraDB aprende de toda esa familia: **los cinco pilares son el patrón, no el detalle**. La elección de CSR vs Vec<Vec>, de Cypher vs LiraQL, de MVCC vs 2PL, de Java vs Rust — son decisiones de implementación que cambian, pero las cuatro capacidades clásicas son invariantes. Quien las entienda podrá leer el código de cualquier GDBMS sin sentirse perdido.

## 6.8 Las trampas (ojo, cuidado con…)

- **Creer que una biblioteca de grafos ya es una base de datos.** No. La biblioteca no persiste (tu grafo muere al cerrar), no cataloga, no consulta con lenguaje ni transacciona. La frontera son los pilares. Si no la ves, cada capítulo futuro te parecerá «por qué tanto lío». El test decisivo: si tu «biblioteca» sobrevive a un `kill -9` y responde consultas en milisegundos, ya no es una biblioteca — es un GDBMS con vocabulario humilde.
- **Creer que «base de datos» = «volcado a fichero».** Un `fs::write` no da consulta, ni índices, ni recuperación. Es solo la primera semilla del pilar persistencia, y del cap. 10 (append-only) y el 11 (páginas) depende que sea algo más. El volcado es a la base de datos lo que el boceto a la novela: ambos tienen «lo mismo» por encima, pero el primero no aguanta una segunda lectura.
- **Confundir los 4 pilares clásicos con los 5 del libro.** No contradice; el libro REORDENA «consulta» y añade «algoritmos» y «producto» porque es un GDBMS con una pedagogía. Entiende los 4 como el *qué es una BD* y los 5 como el *mapa de este libro*. Quien cuenta 4 pilares y ve «5 capítulos», está mezclando niveles.
- **Confundir *persistencia* con *durabilidad/recuperación*.** Persistencia es «sobrevive al cierre del proceso» (Parte III); durabilidad y solidez frente a crash es otra cosa, más profunda (Parte VI). El WAL no es «otra forma de guardar»: es «no corromper lo guardado si todo se va al carajo a mitad». Esta distinción es la que pagarás si la confundes cuando en el cap. 28 hablemos de *write-ahead*: persistencia es lo que hiciste al escribir el fichero; durabilidad es lo que pasa cuando el sistema se cae antes de que el fichero se cierre.
- **Pensar que el orden del libro es opinable.** El orden persistir → consultar → algoritmos → fiabilidad → producto NO es estético; es de dependencias técnicas. Saltar a algoritmos (Parte V) sin tener persistencia (Parte III) deja al lector ejecutando Dijkstra sobre `Vec<Vec<NodeId>>` y volviendo a empezar cuando llegue a disco. Es la misma razón por la que no aprendes a cocinar el soufflé antes de aprender a hervir agua.
- **Creer que el modelo del cap. 7 ya es la «base de datos entera».** El cap. 7 define el modelo en RAM; lo que falta es *cómo se serializa* (cap. 9), *cómo se pagina* (cap. 11), *cómo se cataloga* (caps. 12-13). Quien cierre el cap. 7 pensando «ya tengo una BBDD» no ha entendido las Partes III-VI. El modelo sin persistencia es como un programa sin compilador: describe lo que quieres, pero no ejecuta nada.

## 6.9 Una historia pequeña

En la primera versión real de LiraDB, antes de que existiera este capítulo — es decir, antes de que existiera el *plan* — guardar un grafo significaba una función que recorría los nodos y escribía texto plano. Funcionó dos semanas. Al día quince, alguien quiso «la edad de Ana», y tuvimos que leer el fichero entero para encontrarla, porque no había manera de preguntarle al fichero, solo de escarbarlo. A la semana siguiente, un proceso se cayó a mitad de una escritura y el fichero quedó a medio nodo, y nadie pudo decirnos si esa entrada era válida o basura con forma de entrada válida.

No reescribimos el código ese día. Reescribimos la PEDAGOGÍA: dibujamos el viaje de las cuatro estaciones, enumeramos los pilares y decidimos que el libro entero sería levantarlos en orden. Cuando, unos capítulos después, construyamos la página (cap. 11) y el índice (cap. 15) y el WAL (cap. 28), no estaremos improvisando: estaremos subiendo las estaciones 2, 3 y 4 del mapa que este capítulo te muestra. Ese es el valor de tener un plan.

La moraleja, en una frase, es la que también le sacó el equipo de System R en 1976: **construir un motor no se improvisa; se levanta piedra a piedra, con un mapa que dice qué piedra va primero y por qué.** Y el mapa de este libro es este capítulo. Quien lo lea como «una introducción bonita» se perderá cada decisión de los caps. 9-30. Quien lo lea como «el plano de un motor» entenderá por qué cada página de disco, cada plan de consulta, cada línea de WAL, existe.

## 6.10 Lo que te llevas

- Una **base de datos** no es «un grafo que se guarda»: es una estructura que persiste, se consulta, transacciona y se indexa. Las cuatro capabilidades son la respuesta a las cuatro preguntas que la RAM no sabe responder.
- Una **biblioteca de grafos** (lo que hiciste en la Parte I) y un **GDBMS** (lo que construimos del cap. 7 en adelante) se distinguen por esas capabilidades, no por el vocabulario. La pregunta crítica del CORPUS tiene aquí su respuesta: no es lo mismo, y la frontera son los pilares.
- El grafo es la **4.ª generación** de modelos de datos (jerárquico → red → relacional → grafo), y LiraDB hereda las lecciones de Codd y de System R. El grafo no tira por la borda el relacional: lo re-orienta a redes, conservando la independencia de datos y añadiendo aristas con propiedades.
- El esqueleto del motor son **5 pilares**: Persistencia (Parte III), Consulta/LiraQL (Parte IV), Algoritmos sobre disco (Parte V), Fiabilidad (Parte VI), Motor real/producto (Partes VII-VIII).
- El **viaje de las 4 estaciones** (RAM → disco → consulta → transacción) fija el orden del libro: no se puede subir ninguna estación antes de la anterior. Los dos vagones laterales (algoritmos sobre persistido, motor como producto) cuelgan del viaje, pero no lo redefinen.
- Cada pilar se ancla a un ejemplo que volverá: `SHORTEST PATH` (cap. 22), `PageRank` (cap. 24), `¿está conectado?` (cap. 5 → cap. 26), `MATCH…RETURN` (cap. 17), `BEGIN…COMMIT` (caps. 27-30). Cuando veas esos ejemplos implementados, reconocerás el pilar al que pertenecen.
- El capítulo **cierra la Parte I y abre la Parte II**. Los caps. 1-5 construyeron un grafo en RAM; el cap. 6 muestra por qué eso no basta; del cap. 7 en adelante empezamos a levantar el motor de verdad.

## 6.11 Qué hemos sacrificado

Sería injusto no decirlo: este capítulo paga un precio por ser mapa y no construcción.

- **No tiene código.** Toda la Parte I tiene Rust ejecutable; este capítulo se queda en prosa y diagramas. El lector que prefiera ver bytes no encuentra consuelo aquí — y debe esperar al cap. 7.
- **No tiene tests propios.** Los pilares no se verifican todavía: solo se nombran. La verificación empieza con el primer test del cap. 7 (`cargo test -p vol2-liradb cap07_modelo`) y se consolida cuando la CLI del cap. 31 ejecuta una consulta real.
- **Su «prueba de fuego» es de coherencia, no de ejecución.** El lector demuestra que entendió el mapa cuando puede decir, ante cualquier capítulo futuro, a qué pilar pertenece. No hay una corrida `cargo` que confirme esto — solo el siguiente capítulo del libro.
- **Anuncia herramientas que aún no existen.** `LiraQL`, WAL, MVCC, PageRank, CSR persistente, CLI — todos se *mencionan* aquí con su capítulo, pero no se construyen. Si el lector abre este capítulo y espera ver `MATCH`, verá solo la promesa.

Lo que ganamos a cambio: **coherencia**. El resto del libro se lee con un plano en la cabeza. Cada decisión técnica aparece con su porqué, su alternativa, su modo de fallo y su fuente — pero sobre todo, con su pilar. Eso convierte cada capítulo posterior en un paso del plano, no en una sorpresa.

## 6.12 Pin de batalla

> *«En RAM un grafo es lo que calculas; en una base de datos es lo que persiste, lo que consultas y lo que guardas aunque todo se apague. La diferencia no es el algoritmo: es la persistencia, y todo lo que cuelga de ella.»*

Y el mapa de una sola mirada del capítulo:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                      LiraDB en una sola imagen                            │
├──────────────────────────────────────────────────────────────────────────┤
│ Parte I (caps. 1-5)   →  Grafo en RAM .............................. ✓  │
│ Parte III (caps. 10-16) → Persistencia (pilar 1) .................... → │
│ Parte IV (caps. 17-21) → Consulta / LiraQL (pilar 2) ................ → │
│ Parte V (caps. 22-26) → Algoritmos sobre disco (pilar 3) ............ → │
│ Parte VI (caps. 27-30) → Fiabilidad / WAL / MVCC (pilar 4) .......... → │
│ Partes VII-VIII (caps. 31-40) → Motor real / producto (pilar 5) ..... ✓  │
└──────────────────────────────────────────────────────────────────────────┘
```

Si solo te llevaras una tabla de este capítulo, debería parecerse a esta.

## 6.13 Si solo lees 30 segundos

La Parte I te dio el grafo-en-RAM (estación 1 del viaje). Una base de datos es lo que sube las otras tres estaciones: **persistir** el grafo en disco (Parte III), **consultarlo** con un lenguaje, LiraQL (Parte IV), **ejecutar algoritmos** sobre él sin cargarlo entero (Parte V) y **hacerlo fiable** bajo fallos y concurrencia (Parte VI), para convertirlo al final en un **producto real** (Partes VII-VIII). Esos son los 5 pilares del motor LiraDB — y el resto del libro es levantarlos, en orden, capítulo a capítulo. Empieza ahora: cap. 7, el modelo de datos.

Si solo recuerdas una cosa de este capítulo, que sea esto: **una biblioteca de grafos te da un grafo; un GDBMS te da una base de datos que además es un grafo.** La diferencia es persistencia + consulta + transacciones + índices + (en nuestro caso) algoritmos y producto. Quien ve la frontera, ve el libro. Quien no la ve, lo lee como cinco capítulos de algoritmia con un motor de regalo al final.

## 6.14 Ejercicios

### Ejercicios resueltos

**1. Lista los cuatro pilares clásicos de una base de datos y el problema que resuelve cada uno.**

Persistencia (los datos sobreviven al cierre del proceso: resuelve «¿dónde queda el grafo cuando apago?»); consulta (un lenguaje para pedir datos: resuelve «¿cómo le pregunto al motor en vez de escarbar yo el fichero?»); transacciones (consistencia frente a fallos y concurrencia: resuelve «¿qué pasa si se cae a mitad o escriben dos a la vez?»); índices (encontrar sin escanear: resuelve «¿cómo encuentro el nodo de Oporto sin recorrer el millón?»). Ninguno lo puede responder la estructura en RAM de la Parte I.

**2. ¿Por qué el viaje tiene que subir las estaciones en ese orden?**

Porque cada estación depende de la anterior: no hay grafo consultable (estación 3) si no hay grafo persistido (estación 2); no hay grafo transaccional (estación 4) si no hay consultas a transaccionar (estación 3). Por eso el libro persiste (Parte III) antes de consultar (Parte IV), consulta antes de fiabilizar (Parte VI) y fiabiliza antes de empaquetar como producto (Partes VII-VIII).

**3. ¿Qué distingue exactamente a una biblioteca de grafos de un GDBMS?**

Una biblioteca de grafos te da un grafo en memoria con operaciones (añadir nodo, añadir arista, recorrer vecinos, ejecutar BFS/DFS); pero **muere al cerrar el proceso**, no responde a un lenguaje, no se recupera tras un crash, no escala más allá de la RAM, y no garantiza consistencia bajo concurrencia. Un GDBMS (LiraDB, Neo4j, TigerGraph) añade persistencia (los datos sobreviven), un lenguaje de consulta (Cypher/GQL/LiraQL), transacciones con recuperación (ACID + WAL), e índices para encontrar sin escanear. La frontera operativa son los pilares del §6.6: si tu sistema no cumple al menos las cuatro capabilidades clásicas, no es un GDBMS — es una biblioteca con vocabulario de base de datos.

### Ejercicios propuestos

**Esencial (recordar — RETRIEVAL).** Sin mirar el capítulo ni la tabla de contenidos, escribe de memoria: (1) los **5 pilares de LiraDB** con su Parte correspondiente y una pregunta de negocio por pilar; (2) la frase que responde «¿qué convierte un grafo en una base de datos?». Verifica contra §6.6 y §6.10. Pistas graduadas: (1) empieza por «¿sobrevive al apagado?»; (2) «¿se lo pido con un lenguaje?»; (3) «¿y si el proceso se cae a mitad?» y «¿lo encuentro sin recorrerlo todo?». Criterio: los cinco pilares y su porqué, escritos de memoria, sin reconocimiento. Es la primera vez que el libro exige retrieval de un mapa completo — y es el germen del hábito que cada capítulo posterior te pedirá cultivar.

**Intermedio (analizar — SPACING a Vol.I caps. 2/4-5 y Vol.II caps. 1-5).** Toma el minigrafo social de la Parte I (Ada→Bo→Carla→Dani). Para CADA pilar, inventa una pregunta real que el grafo-en-RAM no puede responder de forma persistente, e indica la Parte del libro que la resolvería. P.ej. «¿está conectado Ada con Dani?» → persistencia+algoritmo (Parte V); «¿cuál es el tren más barato?» → algoritmos con pesos de arista (cap. 22); «¿quién es el contacto central?» → PageRank (cap. 24); «¿cómo le pregunto sin recorrerlo todo?» → consulta (Parte IV); «¿qué pasa si se cae el proceso a mitad de una inserción?» → fiabilidad (Parte VI). Pistas: (1) agarra los 5 pilares uno a uno; (2) añade el dato (peso, centralidad, ciudad) que el grafo de caps. 1-5 no guarda; (3) di la Parte. Criterio: 5 preguntas bien ancladas a pilar + Parte.

**Experto (crear — INTERLEAVING a caps. 7-9).** Dibuja el diagrama de dependencias del motor recién anunciado: 5 nodos (un pilar cada uno) y una flecha «necesita de» por par. ¿Qué debe existir antes que qué? Sobre el grafo, señala el capítulo-bisagra que desacopla dos pilares (pista: el trait `GraphStore` del cap. 8 es la pieza que permite cambiar «disco» sin tocar «consulta»). Pistas: (1) un lenguaje de consulta, ¿sobre qué lee?; (2) un algoritmo, ¿sobre qué recorre?; (3) una transacción, ¿qué protege? Criterio: orden de dependencias correcto y bisagra identificada (prepara cap. 8).

**Modelo de respuesta (rúbrica explícita del experto)**: el grafo de dependencias tiene 5 nodos (P1 Persistencia, P2 Consulta, P3 Algoritmos, P4 Fiabilidad, P5 Motor real) y las flechas mínimas: P1 → P2 (consulta lee de persistencia), P1 → P3 (algoritmos recorre persistencia), P2 → P3 (algoritmos usan resultados de consulta), P2 → P4 (transacciones protegen consultas), P1 → P4 (transacciones registran en persistencia), P1+P2+P3+P4 → P5 (el motor real integra todo). La **bisagra** que desacopla P1 de P2 es el trait `GraphStore` del cap. 8 — un contrato que dice «dame nodos y aristas», sin que el cliente sepa si la respuesta sale de RAM, del buffer pool, o del disco. Esa indirección es lo que permite que P1 madure (cambiar el `Pager`, añadir un índice, mover adyacencia a CSR) sin tocar ni una línea de P2.

## 6.15 Para profundizar

- **E. F. Codd**, *A Relational Model of Data for Large Shared Data Banks*, CACM 13(6), pp. 377-387, 1970 — la sección 1 critica los modelos de árbol y red: la semilla de «por qué hace falta un modelo mejor» que hoy re-cuenta el grafo. Re-leer solo la §1 es uno de los mejores ejercicios de un ingeniero de datos: once páginas que cambiaron cómo se piensa el almacenamiento.
- **M. M. Astrahan, et al.** (incl. **J. Gray**), *System R: Relational Approach to Database Management*, ACM TODS 1(2), pp. 97-137, 1976 — el primer DBMS relacional de verdad: logging, recovery, actualización compartida. La demostración práctica de los pilares 2 y 4. Gray firmaba como autor principal del logging; esa es la genealogía directa del WAL del cap. 28.
- **I. Robinson, J. Webber, E. Eifrem**, *Graph Databases*, 2.ª ed., O'Reilly/Neo4j, 2015 — GDBMS vs biblioteca de grafos (caps. 1 y 8), y la definición del property graph que el cap. 7 detallará. Lectura obligada para cualquiera que vaya a construir (o usar) una GDBMS.
- **N. Francis et al.**, *Cypher: An Evolving Query Language for Property Graphs*, SIGMOD 2018 — el lenguaje de consulta de grafos del pilar 2; y **ISO/IEC 39075:2024 (GQL)**, el estándar que lo formaliza. Cypher es la fuente directa de LiraQL (cap. 17).
- **A. Petrov**, *Database Internals*, O'Reilly, 2019, caps. 1-3 — la mirada actual al layout de páginas y al log estructurado; el puente hacia la Parte III de este libro. Si este capítulo despierta tu curiosidad sobre cómo se «baja» un modelo a disco, ese libro es el siguiente paso.
- **CODASYL Data Base Task Group**, *April 1971 Report*, ACM SIGMOD Record (varias reimpresiones) — el documento fundacional del modelo de red: la 2.ª generación de la que habla este capítulo, ya literalmente un grafo, sin propiedades ni identidad estable. Leerlo es ver qué le faltó al grafo de 1971 para ser una base de datos moderna.
- Dentro del libro: cap. 7 (el modelo de datos, siguiente), cap. 8 (el trait `GraphStore`), caps. 11-16 (la estación 2, con el cap. 11-piloto como primera página), caps. 22 y 24 (SHORTEST PATH y PageRank anclados aquí), `tabla-de-contenidos.md` (el mapa de Partes).

## 6.16 Mini-diálogo: en guardia nocturna

> — O sea, que llevo cinco capítulos construyendo… ¿un grafo en un `Vec`?
>
> — Sí. Y bien construido. Pero un `Vec` es una estructura de datos, y va a morir al cerrar el programa. Eso es la estación 1 del viaje.
>
> — ¿Y una base de datos es… la estación 2?
>
> — La 2, la 3 y la 4 juntas: guardarlo en disco sin corromperlo, poder preguntárselo con un lenguaje, y que aguante si todo se apaga a mitad. Por eso el resto del libro existe: son años de lecciones — Codd, System R, Neo4j — aplicadas a un solo edificio que tú vas a levantar piedra a piedra.
>
> — Entonces el cap. 7 es…
>
> — La primera pegada. Decide qué significa exactamente "tener un grafo" dentro de una base de datos: qué tipos guarda una propiedad, qué es un id, qué diferencia un label de una propiedad. Es el primer ladrillo del motor que acabamos de dibujar. Manos a la obra.
>
> — Un momento: si los pilares son cinco, ¿por qué el libro no empieza directamente con la persistencia, en vez de pasar por la Parte I de grafos-en-RAM?
>
> — Porque sin un grafo que valga la pena persistir, persistir no tiene sentido. La Parte I te enseñó a *pensar* en grafos: a vestirlos con identidad, etiquetas y propiedades, a recorrerlos bien. Sin ese modelo, persistirías bytes sin significado. El orden del libro es «primero piensa bien, luego guarda bien»: lo que Codd hizo en 1970 (pensar el modelo) antes de que System R lo guardara en 1976.
>
> — Y la consulta, ¿no podría ir antes de la persistencia? ¿Una API en RAM?
>
> — Esa es exactamente la diferencia entre una *biblioteca de grafos* y un *GDBMS*: la consulta sin persistencia es una API bonita que muere al cerrar. La consulta CON persistencia es lo que el cap. 17 empezará a construir. Y por eso la Parte III va antes que la IV: no lees del disco si todavía no tienes disco.
>
> — ¿Y si solo me interesa el grafo matemático? ¿Me sobra todo el Vol.II?
>
> — Te sobra la mitad, pero no la totalidad. El Vol.I te dio el analizador; el Vol.II te da el constructor. Si solo quieres leer grafos y aplicarles algoritmos en RAM, el Vol.I basta — y este capítulo lo habrás leído como «una nota al margen del Vol.I». Pero si quieres guardar grafos, consultarlos, recorrerlos por tipo, protegerlos ante un crash, o — sobre todo — **comprender por qué los GDBMS modernos son como son**, este capítulo es donde empieza la conversación. Y termina cuando el último `cargo test` del cap. 40 pase en verde.

---

*(Próximo capítulo: 7 — El modelo de datos de LiraDB (Property Graph + Value). Aquí dibujamos el esqueleto del motor; ahora daremos cuerpo a su primera pieza: el modelo de datos que todo lo demás persiste, indexa, consulta y recorre.)*
