# Capítulo 1 — Qué es realmente un grafo

> *«Todo comenzó con un puente. Y con un hombre que decidió que "cada vez que cruzo" no era una pregunta de andar, sino de geometría.»*

## 1.0 La anécdota de la esquina

En 1736, en la ciudad prusiana de **Königsberg** (hoy Kaliningrado, en Rusia), los habitantes llevaban generaciones haciendo lo mismo: dar paseos dominicales por sus puentes. El río Pregel corta la ciudad en dos orillas y dos islas, conectadas por **siete puentes**: unos unen cada orilla con cada isla, y otros cruzan de una isla a la otra. Y la gente se entretenía con un desafío que nadie sabía resolver del todo: ¿se puede dar **un paseo que cruce cada uno de los siete puentes exactamente una vez** y volver al punto de partida? Era el reto de los paseos de los domingos, del que todo el mundo hablaba y nadie demostraba.

El problema sonaba a pasatiempo. Era un problema de *pasear*, no de *geometría* — no había que medir distancias ni ángulos. Y sin embargo **Leonhard Euler**, en ese año 1736, publicó un trabajo titulado *«Solutio problematis ad geometriam situs pertinentis»* (La solución de un problema relativo a la geometría de la posición) en los *Commentarii Academiae Scientiarum Petropolitanae*. En él demostró dos cosas que hoy nos parecen obvias y que entonces eran revolucionarias:

1. Que la respuesta a la pregunta de los puentes es **no**: no se puede cruzar los siete puentes una sola vez cada uno y volver al punto de partida (porque había más de dos cruces «impares»).
2. Que para resolverlo **no importaban ni las distancias ni las formas** —solo *qué se conecta con qué*.

Los siete puentes eran un croquis de líneas y puntos. Un siglo y medio después, ese croquis tenía nombre: **grafo**. Y aquel día de 1736 quedó sembrada la semilla de todo lo que vamos a construir en este libro. Porque un grafo no es un dibujo bonito: es la manera más antigua y más directa que tiene la humanidad de decir **"esto está conectado con aquello"**.

## 1.1 Objetivo

Si ya has leído el Volumen I, sabes dibujar un grafo y hasta recorrerlo con BFS y DFS. Este capítulo no va a repetir eso. Al terminar, vas a poder responder a una pregunta que el Vol.I apenas toca:

> **¿Qué es un grafo *para una base de datos*?**

En concreto:

1. **Re-orientar la definición del Vol.I**: un grafo es una **estructura de relaciones**, sí — pero para que de verdad sirva como base de datos necesita tres cosas que el grafo matemático no tiene: **identidad estable, datos adjuntos y etiquetas**.
2. **Contrastar el grafo matemático** (el esqueleto: solo nodos y aristas) **con el grafo de datos** (el esqueleto vestido de esquema). Es el giro total del Vol.II: el mismo grafo aparece en dos trajes distintos según su destino.
3. **Entender por qué la matriz de adyacencia** del Vol.I es magnífica para *analizar* pero insuficiente para *persistir*: lo que brilla en RAM puede ser un desastre en disco.

Este es el capítulo de apertura del Vol.II y de la Parte I «Pensar en grafos». Es conceptual: no hay código todavía (ese llega en el cap. 7). Lo que haremos aquí es poner un cimiento que los próximos 39 capítulos van a construir encima. Cada decisión técnica que tomes — desde cómo codificar un valor hasta cómo decidir un plan de ejecución de una consulta — va a apoyarse sobre esta distinción entre esqueleto y vestido. Sin ella, el resto de la obra parece magia.

## 1.2 Problema

Cierra los ojos y piensa en «Ana conoce a Bo». Trata de dibujarlo como si fueras un ordenador. Si te acuerdas del Vol.I, tu primer impulso es un círculo para Ana, un círculo para Bo, y una flecha que dice «conoce a». Eso es un grafo. Hasta aquí, el Vol.I ya te ha enseñado todo lo que necesitas para *recorrerlo*.

Ahora intenta un segundo ejercicio. Guarda ese dibujo en un fichero. Y luego, **tres meses después**, vuelve a abrirlo. Los datos te dicen:

- Ana tiene nombre, tiene 36 años y vive en Madrid.
- Bo se llama Bo, y Ana lo conoce **desde 2020**.
- Ambos son personas, pero Ana además es autora, y es el tipo de nodo «Person» a la vez que «Author».

Si tu grafo solo es «un círculo y una flecha», no tienes sitios donde poner nada de eso. La flecha dice «hay enlace» pero no dice *de qué tipo* (¿KNOWS o WORKS_AT?). El círculo de Ana no tiene dónde escribir «36» ni «Madrid». Y lo peor: si mañana borras «el círculo número 3» de la lista, todo lo que apuntaba a «el círculo número 3» pasa a apuntar al círculo equivocado.

Este es el problema del capítulo, y es el problema del Vol.II entero: **el grafo de la teoría (el esqueleto de flechas) no es suficiente para que sea una base de datos.** Necesitamos vestir el esqueleto. Las preguntas del CORPUS para este capítulo lo dirigen: ¿qué relación guarda con los caps. 1-2 del Vol.I? → re-orientarlos. ¿Qué añade el Vol.II sobre la definición de grafo? → dato + etiqueta + identidad.

Y fíjate en que el problema solo aparece **al guardar y volver a abrir**. Mientras el grafo vive en RAM con su identidad efímera de proceso (un puntero es único durante la vida del proceso), todo «funciona». El día que apagas el portátil, la identidad reciclable de la matriz de adyacencia sale a la luz: el índice 0 puede haber sido ocupado por otro nodo, y todas las aristas que apuntaban a «0» apuntan a otro. Es el problema real que el cap. 3 resolverá con `slotmap`. Aquí solo lo identificamos como punto donde la teoría del Vol.I se estrella contra la práctica de una BBDD.

## 1.3 Modelo mental

Piensa en tu **almanaque de contactos** del móvil. No es «una lista de nombres»: es una red. Cada contacto es un **punto**. Cada contacto sabe, además de su nombre, un montón de datos: su número, su cumpleaños, su ciudad. Y entre dos contactos hay **relaciones** —«le debo dinero», «es mi familia», «trabajamos juntos»—, cada una con sus detalles: «le debo *desde marzo*».

Ahora vamos a dibujarlo de verdad. Aquí tienes un minigrafo, la versión más pequeña posible que captura todo lo importante:

```
                «Ana conoce a Bo»
                                         
              +-------+                  +-------+
   id: 0      |       |                  |       |      id: 1
   label:     |  Ana  |    desde 2020    |  Bo   |      label:
   Person     |       | ───────────────▶ |       |      Person
              +-------+  label: KNOWS    +-------+
   props:                                   props:
     name: "Ana"                              name: "Bo"
     age:  36                                 city: "Oporto"
```

Fíjate en lo que hay en cada caja:

- Un **id** (0 y 1): un nombre que identifica a cada nodo. No cambia aunque le cambies el nombre a Ana.
- Un **label** (`Person`): qué *tipo* de cosa es. Es la etiqueta de la carpeta, lo que te deja clasificar sin abrir el contenido.
- Unas **propiedades** (`name`, `age`, `city`): los datos adjuntos. Esto es lo que el grafo matemático no sabe guardar.
- La **arista** lleva su propia etiqueta (`KNOWS`) y su propio dato («desde 2020»). La flecha no es un `true`: es una cosa con nombre y contenido.

Ahora el **momento ¡ajá!**. Compara esta caja con la matriz de adyacencia del Vol.I:

```
Matriz de adyacencia (Vol.I):      Property Graph (lo que vierte una BBDD):
    0   1                              id  + label + props     (para nodos)
0  [  ] [T]                            id  + source + target   (para aristas)
1  [T] [  ]                               + label + props
```

La matriz te dice que **hay** un enlace. El Property Graph te dice **qué** son los dos extremos, **de qué tipo** es el enlace, y **desde cuándo** existe. La topología es la punta del iceberg; el dato, la etiqueta y la identidad son el resto, y es ahí donde vive la base de datos.

Y como este libro va de *bases de datos*, merece la pena ver un segundo ejemplo donde lo que importa no es social sino **de información pura**: un mini-mapa con distancias y una red de dependencias, para que grabes que el mismo esqueleto sirve para todos ellos.

```
Mapa (esqueleto + datos)                Dependencias de paquetes (esqueleto + datos)

  Madrid ──[66]──▶ Toledo               app  ──[versión ">=2.0"]──▶ crateA
  Madrid ──[52]──▶ Segovia              app  ──[versión ">=1.4"]──▶ crateB
  Toledo ──[71]──▶ Cáceres              crateB ──[versión ">=0.9"]──▶ crateC
  (cada punto = city: {pop})            (cada nodo = package: {ver, licencia})
```

En el mapa, los nodos son ciudades (con su población), las aristas carreteras (con su distancia). En las dependencias, los nodos son paquetes (con su versión y licencia), las aristas relaciones «depende de» (con su *rango de versiones compatibles*). Son mundos distintos, pero **el esqueleto es el mismo**: puntos y enlaces. Lo que los convierte en bases de datos útiles es lo que hemos vestido alrededor — los datos de cada punto, la etiqueta de cada enlace y un nombre estable para cada cosa. Fijarte en que el mismo esqueleto sirve para tan distintos problemas es, en sí mismo, la intuición más poderosa de este capítulo.

Antes de seguir, dos detalles que te ahorrarán futuros tropiezos:

1. **Las tres ropas están jerarquizadas.** El orden importa. La **identidad** es la más fundamental (existe aunque no haya propiedades); las **etiquetas** son estructurales y se consultan en bucle; las **propiedades** describen a un solo elemento a la vez. Quien pone el *tipo* en una propiedad está pagando una iteración por cada clasificación; quien pone el nombre en una propiedad está mezclando identidad y dato.
2. **El vestido es asimétrico.** Los nodos visten las tres ropas de forma natural; las aristas visten *etiqueta* y *propiedades* casi siempre (en la mayoría de los modelos, una arista es «un verbo entre dos sustantivos»), pero su identidad depende del modelo (en el LPG es de primera clase; en RDF, primo del Vol.III, no lo es).

## 1.4 Primera solución

El Vol.I te dio una primera solución para representar un grafo en memoria: la **matriz de adyacencia**. Es una tabla `N × N` de casillas `true/false` donde la casilla `[i][j]` vale `true` si hay una arista que va del nodo `i` al nodo `j`.

```
      A   B   C            A  B  C
  A [ F ] [ T ][ F ]      [ ] [✓][ ]
  B [ T ][ F ][ T ]   →   [✓][ ] [✓]
  C [ F ] [ T ][ F ]       [ ] [✓][ ]
```

Es una solución maravillosa para **analizar**. ¿Está conectado? ¿De quién es vecino el nodo 0? ¿Hay ciclos? Todas esas preguntas se contestan mirando casillas. Y es la solución que el Vol.I (caps. 1-2) te enseñó a leer. Es *el esqueleto*: solo te dice qué está conectado con qué, y nada más. Y hasta ahí, está perfecta — para *analizar*.

## 1.5 Sus límites

Pero ahora pídele algo más. Quieres que ese dibujo se convierta en **base de datos**. Y la matriz de adyacencia se rompe por los cuatro flancos:

1. **¿Y las propiedades?** No hay ninguna casilla para «Ana tiene 36 años» ni para «el enlace empezó en 2020». El `true` no tiene dónde guardar *datos*. Si amplias la celda a `string` pierdes el `true/false` inmediato y empiezas a preguntarte por cada acceso cuánto cuesta comparar.
2. **¿Y las etiquetas?** No hay forma de decir «Ana es *Person* y *Author*», ni «esta flecha es *KNOWS* y aquella *WORKS_AT*». La matriz no clasifica: solo existe o no existe el enlace. Y sin clasificar no puedes escribir la consulta más básica de una BBDD — «dame todos los *Author*» — sin recorrer cada celda.
3. **¿Y la identidad estable?** Los nodos de la matriz son posiciones: `0`, `1`, `2`. Si borras el nodo del medio, «el nodo 1» pasa a significar otra cosa. El nombre de Ana ya no es estable: **se recicla**. Esto es aún peor en cuanto lo metes en disco: el día que reorganizas el fichero o persistes un grafo más pequeño, los índices se mueven y todo apunta al sitio equivocado. Es el origen del problema que el cap. 3 resolverá con `slotmap`.
4. **¿Y la arista como cosa?** En la matriz, la arista es un bit. Pero «Ana conoce a Bo desde 2020» es una cosa con su propio dato; no cabe la fecha en un bit. Y un día querrás «todas las amistades desde 2015» o «el camino más corto en kilómetros», y la arista necesita guardar su propio dato — su **propiedad** — y un *tipo* — su **label** — para distinguirla de las demás.

La raíz del problema: **la matriz de adyacencia responde a *estructura* (¿hay enlace?), no a *dato* (¿qué es cada cosa y qué significa el enlace?).** El grafo *analítico* del Vol.I sirve para recorrer; el grafo de una *base de datos* necesita además guardar qué es cada nodo, qué tipo de relación une a cada par y cómo nombrar cada cosa de forma estable. En términos de «cuándo usar cada cosa»: la matriz es válida para grafos pequeños y densos en los que la única pregunta es ¿estos dos están enlazados? — pero la BBDD vive de **clases de preguntas más variadas**: por tipo, por rango de valores, por camino entre X e Y, por camino mínimo ponderado, por vecindario entrante (el PageRank). Cada una de esas preguntas exige que la arista (y el nodo) tengan sitio para guardar **datos** y un **tipo**. La matriz, amada por los libros de algoritmos, no tiene ese sitio.

## 1.6 Solución evolucionada

La solución no es abandonar la estructura: es **vestirla**. Es la idea de un **property graph** — el modelo que Neo4j popularizó en 2007 y que el estándar **ISO/IEC 39075 (GQL)** terminó formalizando en 2024 (décadas después de que la idea circulara por la comunidad, la ISO la subió a estándar; eso es la señal de que dejó de ser una moda y se convirtió en el modelo de datos por defecto para grafos).

Conceptualmente (porque los tipos concretos los verás en el cap. 7), un grafo de datos se define así:

```
Grafo de datos = G = (V, E)          ← el esqueleto (el grafo matemático)
                       + LABELS      ← etiquetas / clasificación de cada cosa
                       + PROPS       ← datos adjuntos (esquema)
                       + IDENTIDAD   ← cada cosa tiene un nombre que no se recicla
```

Léelo en orden, porque es la tesis de todo el libro:

> **Un grafo es una estructura de relaciones que, para ser útil como base de datos, exige identidad estable + datos adjuntos + etiquetas. El grafo matemático es el esqueleto; la BBDD viste el esqueleto.**

Tres ropas, en este orden, por una razón concreta:

1. **LABELS** van antes que **PROPS** en la decisión porque clasificar es la pregunta más barata y más rápida: «dame todos los *Author*» se contesta sin abrir ningún dato. Si Ana es una `Person`, una `Author` y una `Speaker`, la búsqueda «muéstrame los Author» mira primero la etiqueta (estructural, barata) y solo después abre el cajón de propiedades si hace falta.
2. **PROPS** van después porque describen *a un solo elemento* a la vez (no iteran por todos). Son la información opaca que vive dentro del nodo o la arista: edad, ciudad, distancia, fecha, peso. Mientras que las etiquetas se indexan, las props se filtran por rango.
3. **IDENTIDAD** es la primera y la más sutil: existe *aunque no haya ninguna propiedad ni etiqueta*. Es lo que permite decir «este expediente existe aunque la ficha esté vacía» o «este nodo sobrevivió a un crash sin perder su nombre». Sin identidad estable, las dos anteriores se desordenan en cuanto borras algo.

Y la arista ya no es un bit: es una **cosa de primera clase** con su propio origen, destino, etiqueta (`KNOWS`) y datos («desde 2020»). Es exactamente lo que dibujamos en el §1.3: cada círculo con `id + label + props`, cada flecha con `label + props`. Ese es el traje que ponemos sobre el esqueleto del §1.4.

Una nota sobre el lenguaje para no acumular sorpresas: cuando oigas **property graph** o **labeled property graph (LPG)** piensa en esto. Cuando leas **RDF** (Vol.III), piensa en otra cosa — una red de *triples* sin aristas con propiedades — que es primo cercano pero no idéntico. El Vol.II vive casi enteramente en LPG; el Vol.III lo contrastará con RDF y con el mundo semántico cuando llegue el momento.

## 1.7 El ángulo de BBDD: ¿qué convierte esto en un motor?

Podrías preguntarte: «vale, un grafo con datos es bonito, pero ¿por qué esto es *una base de datos* y no "una lista de nodos"?». Gran pregunta, y es justo la del **capítulo 6** de la Parte II («Qué convierte un grafo en una base de datos»). Aquí, solo el avance — cuatro requisitos que debe cumplir cualquier motor de BBDD, sean relacionales, documentales o de grafos, y que requieren justamente las tres ropas del §1.6:

1. **Persistir** — que los datos sobrevivan a que apagues el ordenador. Una lista en RAM es un programa; una base de datos es un fichero al que puedes volver mañana, o dentro de un año, y leer en su forma original. Esto obliga a decidir *cómo serializar* cada `Value` (cap. 9) y *cómo estructurar el fichero* (caps. 10-12).
2. **Indexar** — encontrar un nodo *sin mirar todos*. Si cada búsqueda implicase recorrer los millones de filas de Ana hasta dar con ella, el sistema se moriría al primer día. Necesitas índices: por id (hash, cap. 15), por rangos de propiedades (B+ tree, cap. 15), por vecindario (CSR persistente, cap. 14).
3. **Recorrer por ambos lados** — contestar tanto «a quién apunta Ana» como «quién apunta a Ana». Por eso todo motor serio guarda una lista de adyacencia inversa además de la directa (lo verás en `MemoryStore` con `adj_in` y `adj_out`, caps. 7-8; y en el CSR dual, cap. 14).
4. **Consultar** — hablar con los datos en alguna forma de lenguaje, no solo mediante API programática. Cypher, GQL, SPARQL, Gremlin. En este libro construiremos nuestro propio **LiraQL** (caps. 17-21): un subconjunto pequeño pero suficiente para los casos reales.

Para que cada uno de esos requisitos cumpla su palabra, el grafo necesita exactamente las tres ropas del §1.6:

- una **identidad estable** que siga siendo la misma tras guardar y leer (cap. 3);
- unos **datos adjuntos tipados** que se puedan guardar, ordenar y comparar (caps. 7 y 9);
- unas **etiquetas** que permitan buscar por tipo sin abrir cada ficha (cap. 7).

En este capítulo de apertura no vas a construir nada de eso todavía. Vas a **ver el esqueleto**, entender **por qué hay que vestirlo**, y saber **qué ropa** hace falta y por qué no basta cualquier ropa. Cada una de esas tres ropas tiene su propio capítulo. La promesa es que, cuando llegues al cap. 6 («qué convierte un grafo en una base de datos»), ya tendrás la mitad del trabajo hecho: sabrás qué significa persistir, indexar, recorrer y consultar — y sabrás que las cuatro promesas dependen de la identidad, las propiedades y las etiquetas que ya hemos sembrado aquí.

## 1.8 Los porqués (con fuentes)

Vamos a dejar constancia de dónde nacen las ideas de las que dependemos. Sin fuentes, una afirmación técnica es solo opinión; con fuentes, es una línea del conocimiento de la humanidad.

| Decisión | ¿Por qué? | Fuente |
|---|---|---|
| El grafo empieza en 1736 | Euler fue el primero en tratar «qué se conecta con qué» como un objeto matemático propio, sin depender de distancias ni formas. Es el acta de nacimiento de la teoría de grafos y el origen del término. | L. Euler, *Solutio problematis ad geometriam situs pertinentis*, Commentarii Academiae Scientiarum Petropolitanae 8, 1736. Recopilado en N. Biggs, E. Lloyd y R. Wilson, *Graph Theory 1736–1936*, OUP. |
| Un grafo en su forma pura es `G=(V,E)` | La definición moderna más compacta: un conjunto de **vértices** `V` y un conjunto de **aristas** `E` que juntan pares de vértices. Es el «esqueleto» sobre el que construimos todo lo demás. | Definición estándar: J. Gross y J. Yellen, *Graph Theory and Its Applications*, 2ª ed., CRC Press. |
| Un grafo de BBDD añade datos, etiquetas e identidad | Esa es la diferencia entre *estructura de computación* y *modelo de datos*. El **property graph** es el modelo: nodos etiquetados + aristas de primera clase con propiedades. El mundo lo estandarizó en 2024. | I. Robinson, J. Webber, E. Eifrem, *Graph Databases*, 2ª ed., O'Reilly/Neo4j, 2015; ISO/IEC 39075:2024 (*Graph Query Language*, GQL). |
| Lo conceptual del capítulo no necesita tipos aún | El tipado exacto (`Value`, `id` de un tipo concreto) es una decisión de modelo de datos y se define bien en el cap. 7; aquí solo conceptuamos el *qué* para no disparar la memoria de trabajo del novato. | Regla de dificultad asimétrica del manual de estilo (Apéndice 0): una idea nueva por sección. |
| La matriz de adyacencia es la primera solución y NO la final | La representación del Vol.I (caps. 1-2) brilla para «¿están conectados u y v?» en grafos pequeños y densos. Para construir una BBDD hay que pasar a **lista de adyacencia** (cap. 2) y a **CSR persistente** (cap. 14), pero *entender primero por qué la matriz se queda corta* es lo que hace al lector elegir con criterio. | Vol. I caps. 1-2; Vol. II cap. 2; el análisis de densidad y grado medio reaparece en el cap. 14. |
| El orden de las tres ropas (identidad > etiqueta > propiedad) está jerarquizado | La identidad existe aunque no haya propiedades; las etiquetas estructuran; las propiedades describen a un solo elemento. Quien pone el tipo en `props["type"]` paga un escaneo por cada clasificación. Por eso se decide en este capítulo, no en el modelo de datos. | Regla de modelado de Robinson/Webber/Eifrem cap. 3; decisión formal en el `enum Value` y en `add_node`/`add_edge` del cap. 7. |

*(El contraste «grafo matemático vs grafo de datos» también tiene un primo famoso en el modelado de bases de datos: el **modelo entidad-relación** de Peter Chen de 1976, donde las entidades y sus relaciones son la plantilla conceptual sobre la que se construyen tablas. El property graph es, en cierto sentido, ese modelo visto con ojos de punta —verás a qué sabe en el cap. 7.)*

## 1.9 Ojo, cuidado con… (las trampas)

Todo el mundo tropieza aquí de la misma manera. Si detectas estos tres síntomas a tiempo, te ahorrarás capítulos enteros de confusión:

1. **«Todo grafo es un árbol.»** No. Un **árbol** es un grafo *muy especial*: conectado (una sola pieza), acíclico (sin bucles cerrados) y con *un solo camino* entre cualquier par de nodos. La gran mayoría de los grafos no son árboles: tienen ciclos, varias rutas, más de una componente. Si piensas que «grafo» y «árbol» son sinónimos, entender cuándo hay un ciclo o una componente suelta (cap. 5) se te va a atragantar. El árbol es el caso; el grafo es la clase.
2. **Confundir el nodo con su dato.** «Borrar el nodo 0 es borrar a Ana.» No: borrar el nodo 0 borra la *posición* que llamamos Ana; el dato «Ana, 36, Madrid» vive *dentro* de ese nodo pero no *es* el nodo. Confundirlos es la causa de la identidad reciclada: si el nodo fuera el dato, cambiarte el nombre cambiaría tu identidad. La identidad es una cosa; el dato es otra (lo resolveremos a fondo en el cap. 3).
3. **«Un grafo es solo la matriz de true/false.»** La matriz responde a *estructura*: dice si hay enlace. Una base de datos de grafos responde además a *qué* es cada extremo y *desde cuándo* existe el enlace. Sigue siendo útil para analizar (Vol.I), pero no la tomes por «el grafo entero» en una BBDD.
4. **«Las propiedades y las etiquetas son lo mismo.»** No. Las **etiquetas** (`Vec<String>` por nodo) son nombres que clasifican y se consultan en un bucle sin abrir el cajón de la propiedad. Las **propiedades** (`HashMap<String, Value>` por nodo o arista) son los datos opacos que describen a un solo elemento. Quien pone el tipo en `props["type"]` paga un escaneo por cada clasificación, y tendrá problemas desde el cap. 7 en adelante.
5. **«Un grafo es solo nodos y aristas, lo demás sobra.»** Una frase escuchada al terminar el Vol.I, cierta para el capítulo 1 de aquel volumen y falsa para el resto de este. Lo que «sobra» —propiedades, etiquetas, identidad estable— es **exactamente** lo que convierte un esqueleto en una BBDD. Si tu instinto te dice «no necesito etiquetas para empezar», recuerda: las etiquetas son el único modo de responder «dame todos los usuarios mayores de 30» sin abrir cada nodo. La pereza de no usarlas se paga con cada consulta.

**Precisión de lenguaje (glosario)**: *grafo matemático* (estructura pura `V×E`) vs *grafo de datos / Property Graph* (esqueleto + dato + etiqueta + identidad); *nodo/vértice* (la posición) vs *dato* (lo que vive dentro); *identidad estable* (nombre que no se recicla) vs *índice* (posición que sí cambia al reorganizar); *label* (la etiqueta de la carpeta, qué tipo es) vs *propiedad* (el dato opaco de dentro). *Árbol* (grafo especial) vs *grafo* (la clase general). *LPG* (labeled property graph, aristas de primera clase, lo que construye este libro) vs *RDF* (triples sin aristas con propiedades; Vol.III).

## 1.10 Una historia pequeña

Los primeros días de LiraDB, antes de que existiera el modelo de datos del cap. 7, un nodo era un `HashMap<String, String>` y una arista un par de números en un conjunto. Ana se guardaba como `{"name": "Ana", "age": "36"}` —fíjate, **"36" entre comillas**, era texto. Y la arista entre Ana y Bo era apenas `(0, 1)`: un «hay enlace» sin nombre y sin fecha.

Funcionó un día. Al siguiente, Ana quiso ordenar a sus contactos por edad y el resultado fue `1, 10, 2` — porque «36» y «102» son *textos*, y los textos se ordenan así. Después quiso saber desde cuándo conocía a cada uno, y no había ningún sitio en un bitset para «desde 2020». Quiso luego «dame todos mis amigos *Author*», y no había etiqueta que mirar. Quiso borrar a un contacto, y al día siguiente otro contacto «heredó» su hueco — el id se había reciclado. Y quiso apagar el portátil, volver a abrir el fichero al día siguiente, y... el grafo cargado difería ligeramente del grafo guardado. No mucho. Lo suficiente para que dos aristas apuntasen a un nodo que ya no existía, y un nodo que sí existía tuviese dos padres.

El «esqueleto» de flechas y true/false era precioso para recorrer, y un desastre para *ser una base de datos*. Cada cosa que pedía Ana era un síntoma del mismo diagnóstico: **faltaba el modelo**. Escribir el modelo de datos (cap. 7) fue ese mismo fin de semana. La lección no fue «usa enums»; fue: **el grafo matemático es el esqueleto, y una base de datos necesita vestirlo de datos, etiquetas e identidad.** Lo que este capítulo acaba de sembrar. Si lo grabas a fuego — y por qué lo necesita cada una de las dos docenas de «Ana quiere...» que preceden — tendrás la mitad de la obra entendida antes de escribir la primera línea de Rust.

## 1.11 Lo que te llevas

- Un grafo es una **estructura de relaciones**: vértices unidos por aristas, con o sin dirección (`G=(V,E)`). Es el esqueleto.
- Para que sea **útil como base de datos**, ese esqueleto se viste con tres ropas: **datos adjuntos** (las propiedades), **etiquetas** (qué tipo de cosa es cada elemento) e **identidad estable** (un nombre que no se recicla al reorganizar).
- El **grafo matemático** es el esqueleto (estructura pura, genial para *analizar* y recorrer). El **grafo de datos / Property Graph** es el esqueleto vestido de esquema (lo que una BBDD *persiste*).
- La **matriz de adyacencia** del Vol.I responde a *estructura* (¿hay enlace?); una BBDD de grafos responde además a *qué* es cada extremo y *desde cuándo* existe el enlace.
- La arista de una BBDD es una **cosa de primera clase**: con origen, destino, etiqueta (`KNOWS`) y datos («desde 2020») — no un bit.
- **La jerarquía importa**: identidad > etiqueta > propiedad. Quien pone el tipo en `props["type"]` o el nombre en `props["name"]` paga con cada clasificación y cada renombrado; poner cada cosa en su sitio es la forma correcta de construir el modelo.
- **El orden de las tres ropas tiene consecuencias operativas**: las etiquetas estructuran y se indexan barato; las propiedades son opacas y se filtran lento; la identidad es el pegamento que permite que las dos anteriores sobrevivan a borrados, reorganizaciones y crashes. Cambia la BBDD entera si cualquiera de las tres falla.

## 1.12 Pin de batalla

> *«El grafo matemático es el esqueleto; la base de datos viste el esqueleto con datos, etiquetas e identidad. Quien confunde el esqueleto con el vestido, construye un motor que solo sabe dibujar flechas.»*

### Resumen visual del capítulo (una sola mirada)

| Aspecto | Grafo matemático (esqueleto) | Grafo de BBDD (esqueleto vestido) |
|---|---|---|
| **Unidad** | Vértice `v ∈ V`, arista `e ∈ E` | Nodo con `id + label + props`, arista con `id + source + target + label + props` |
| **¿Tiene datos?** | No | Sí: `props` tipadas |
| **¿Tiene identidad estable?** | No (es posición en una matriz) | Sí (`id` que no se recicla) |
| **¿Tiene etiquetas?** | No | Sí (`labels` en nodos; `label` en arista) |
| **Representación típica** | Matriz de adyacencia con bits (Vol.I cap. 2) | Property Graph (cap. 7): nodos + aristas + adjacencia |
| **Para qué brilla** | Analizar (`¿existe u-v?`) | Persistir, consultar y clasificar (BBDD real) |
| **Norma detrás** | `G = (V, E)` | `G = (V, E) + LABELS + PROPS + IDENTIDAD` |
| **Lo que se pierde si solo tienes el esqueleto** | Nombre, edad, fecha, tipo de relación, identidad tras borrado | — |
| **Origen histórico** | Euler, Königsberg, 1736 | Property Graph: Neo4j 2007; estándar ISO/IEC 39075 (GQL) 2024 |

Si solo te llevaras **una tabla** del Vol.II entero, debería parecerse a esta.

## 1.13 Si solo lees 30 segundos

Un grafo es un conjunto de **vértices** unidos por **aristas** (`G=(V,E)`). Eso es el esqueleto, y es suficiente para *analizar* y recorrer — lo que ya hiciste en el Vol.I. Pero para que un grafo sea una **base de datos**, hay que vestir ese esqueleto con tres cosas que el esqueleto no tiene: **datos adjuntos** (las propiedades de cada nodo y arista), **etiquetas** (el tipo de cada cosa) e **identidad estable** (un nombre que no cambie al reorganizar). Ese es el *property graph*: el esqueleto + dato + etiqueta + identidad. Ésa es la diferencia entre «una estructura de computación» y «una base de datos de grafos», y el cimiento conceptual sobre el que construimos LiraDB.

## Ejercicios resueltos

**1. ¿Por qué un árbol es un grafo pero un grafo no tiene por qué ser un árbol?**

Porque un árbol cumple tres condiciones *especiales* que la mayoría de los grafos no cumplen: está **conectado** (una sola pieza, sin piezas sueltas), es **acíclico** (no existen caminos cerrados que te devuelvan al punto de partida) y tiene **exactamente un camino** entre cualquier par de nodos. Un grafo general relaja todas esas condiciones: puede tener ciclos (redes de amistad), varias componentes (montones de islas conectadas) y más de una ruta entre dos nodos. Así que «árbol» es un subconjunto de «grafo»: todo árbol es grafo, pero no todo grafo es árbol. Confundirlos hace que no entiendas qué es un ciclo ni qué es una componente (palabras que usarás en el cap. 5).

**2. En el ejemplo del §1.6 («Ana es Person y Author»), ¿por qué `Person` es una *etiqueta* y «36» una *propiedad*?**

Porque cumplen funciones distintas. La **etiqueta** (`Person`, `Author`) *clasifica*: te dice qué tipo de cosa es el nodo, y es exactamente lo que una búsqueda por tipo ("dame todos los autores") necesita mirar, sin abrir nada más. La **propiedad** (`age: 36`, `name: "Ana"`) *describe*: son los datos opacos que viven dentro del nodo y que necesitas abrir para leer. Es la diferencia entre la *etiqueta de la carpeta* (la miras sin abrir para saber qué hay dentro) y las *notas que están dentro de la carpeta* (necesitas abrirla). Mezclarlas (guardar el tipo en `props["type"]`) obliga a escanear todas las propiedades para clasificar — un error que pagarás caro en el cap. 7.

## Ejercicios propuestos

**Esencial (recordar).** Sin mirar el §1.6, dibuja un minigrafo de **3 nodos con 2 aristas**. Para cada nodo escribe su `id`, su `label` y 2 propiedades; para cada arista escribe su `label`. Cuando termines, compara con el ejemplo del capítulo. Criterio (rúbrica): tu dibujo separa claramente `id` (identidad estable) de `label` (clasificación) de `props` (datos adjuntos), y cada arista tiene su propio label. Pistas: (1) el `id` es el nombre que no cambiaría si rebautizas la persona; (2) el `label` dice *qué tipo de cosa* es; (3) la `prop` es un dato concreto (una edad, una ciudad).

**Intermedio (aplicar / spacing Vol.I caps. 1-2 / interleaving con la vida real).** Piensa en una **red de dependencias de paquetes de software** (por ejemplo, un proyecto Rust que depende de varios *crates*). Responde: ¿qué son los *nodos* y las *aristas*? ¿Qué *propiedades* querrías adjuntar a un nodo (paquete) y qué a una arista (A depende de B)? Y explica: ¿qué se pierde si representaras eso únicamente con la matriz de adyacencia del Vol.I cap. 2? Criterio: identificas nodos y aristas correctamente, pones datos concretos en nodos y aristas, y ves que la matriz solo guarda el «true/false» de «depende», perdiendo la versión o la condición de la dependencia. Pistas: (1) un paquete es la «caja», una dependencia es la flecha; (2) en la arista «A depende de B» podrías guardar la *versión* permitida; (3) la matriz de bits no tiene sitio para «solo si es la versión 2 » de esa dependencia.

**Modelo de respuesta (rúbrica explícita)**:

- *Nodos*: cada paquete (`crate`). Propiedades adjuntas: `nombre: String`, `versión_actual: SemVer`, `licencia: String`, `autores: Vec<String>`, `hash_del_código: String`. Etiquetas: `["Package"]` (o `["Package", "Local"]` si es un crate tuyo, `["Package", "External"]` si viene de crates.io).
- *Aristas*: una relación «A depende de B» (dirigida). Propiedades: `versión_mínima: SemVer`, `versión_máxima: SemVer`, `features_activadas: Vec<String>`. Etiqueta (label): `"DEPENDS_ON"`.
- *Por qué la matriz de bits se queda corta*: en la matriz basta con `deps[A][B] = true` para decir «A usa a B». Pero «A usa a B **solo si B ≥ 2.0 y < 3.0**» no cabe en una celda booleana: la celda solo guarda el `true`, no la *condición*. Y la pregunta «dame todas las dependencias que requieren una feature dada», como `serde/derive`, es trivial con la propiedad `features_activadas` de la arista; con la matriz obligaría a recorrer cada celda `[A][B]` y comprobar nada más que presencia, no condición. La analogía humana: «Ana conoce a Bo» (true/false) vs «Ana conoce a Bo desde 2020 y con grado de intimidad 7 sobre 10» (propiedades de arista). La matriz solo registra lo primero; la BBDD necesita lo segundo.

**Experto (crear / interleaving cap. 2).** Toma el problema real del intermedio (dependencias de paquetes) y «traduce» su modelo a las tres representaciones de memoria que verás a fondo en el **cap. 2** — *edge list*, *adjacency list* y *CSR* — a nivel conceptual, y decide **cuál retendría mejor el dato + label + identidad** que acabamos de vestir. Criterio: produces las tres variantes y argumentas qué retiene cada una de las tres ropas (datos, etiquetas, identidad). Pistas: (1) la edge list es `(u, v)` — ¿dónde pondrías el label de la arista?; (2) la adjacency list agrupa por nodo — ¿dónde la `prop`?; (3) el CSR comprime los vecinos en un único array — ¿qué ropa sacrifica para ahorrar memoria? (Lo confirmarás en el cap. 2.)

## Para profundizar

- **L. Euler**, *Solutio problematis ad geometriam situs pertinentis*, 1736 — el acta de nacimiento. Está recopilado, con comentario, en N. Biggs, E. Lloyd y R. Wilson, *Graph Theory 1736–1936* (Dover), que es la mejor manera de leer cómo nació la disciplina y por qué tardó un siglo en tener nombre.
- **J. Gross y J. Yellen**, *Graph Theory and Its Applications*, 2ª ed., CRC Press — la definición formal `G=(V,E)` y la diferencia entre grafo, dígrafo y multigrafo, si quieres la versión de referencia y neutral del modelo matemático.
- **I. Robinson, J. Webber, E. Eifrem**, *Graph Databases*, 2ª ed., O'Reilly/Neo4j, 2015 — la definición canónica y didáctica del **property graph**: nodos etiquetados + aristas de primera clase con propiedades. Es lectura obligatoria para cualquiera que vaya a construir (o usar) una GDBMS.
- **ISO/IEC 39075:2024**, *Graph Query Language (GQL)* — el estándar (2024) que convierte el property graph en modelo de datos de primera clase; es la confirmación de que la idea ya estaba en la comunidad y la ISO la subió a estándar, no al revés.
- **P. Chen**, *The Entity-Relationship Model — Toward a Unified View of Data*, ACM TODS 1(1), 1976 — el primo relacional del property graph (entidades + relaciones): el puente conceptual entre grafos y tablas. Se vuelve a usar como contraste en el cap. 7 del Vol.II y en el Vol.III.
- Dentro del libro: **cap. 2** (representaciones de memoria: edge list, adjacency list, CSR), **cap. 3** (identidad estable y `slotmap`), **cap. 7** (el modelo Property Graph tipado: `Value`, `label`, `props`), **cap. 6** (qué convierte un grafo en una BBDD), y, para el Vol.III, los caps. sobre RDF/OWL/SPARQL (cap. 41+) que contrastan LPG con el modelo de triples.

## Mini-diálogo: en guardia nocturna

> — Espera. O sea, que "grafo" no es solo dibujar círculos y flechas.
>
> — Círculos y flechas es el **esqueleto**. Es lo que ya sabías del Vol.I: lo que hay conectado. Pero para que sea una *base de datos* hay que vestirlo: cada círculo necesita una identidad estable que no se recicle, una etiqueta que diga qué tipo de cosa es, y un cajón de datos. Y cada flecha, además de apuntar, necesita decir de qué tipo es y desde cuándo existe.
>
> — Entonces el grafo matemático de Euler... ¿no sirve?
>
> — Sirve, y mucho — es la estructura de *relaciones*, y nadie la construye así de sólida sin Euler. Pero es la mitad de la historia. La otra mitad —el dato, la etiqueta, la identidad— es lo que convierte un grafo en una **base de datos**, que es todo el sentido de construir LiraDB en el resto del libro.
>
> — O sea que este capítulo es... ¿por qué importa el grafo en una BBDD?
>
> — Exacto. No vas a escribir ni una línea de código todavía. Vas a aprender *qué* cuesta hacer de un grafo una base de datos. Cuando el cap. 7 te pida tipar `Value` e `id`, ya sabrás por qué existe cada pieza. Y cuando el cap. 11 te pida persistir nodos, sabrás qué estás persistiendo: no flechas, sino *esqueletos vestidos*.
>
> — Una cosa más. ¿Y si solo me interesa el grafo matemático? ¿Me sobra todo el Vol.II?
>
> — Te sobra la mitad. El Vol.I te dio el analizador. El Vol.II te da el constructor. Si lo que quieres es leer grafos y aplicarles algoritmos en memoria, el Vol.I basta — y este capítulo lo habrás leído como «una nota al margen del Vol.I». Pero si quieres guardar grafos, consultarlos, recorrerlos por tipo, protegerlos ante un crash o, sobre todo, **comprender por qué las GDBMS modernas son como son**, este capítulo es donde empieza la conversación.

---

*(Próximo capítulo: 2 — Cómo representar un grafo en memoria. Aquí el grafo existía como idea y como estructura de datos; ahora veremos las tres representaciones — edge list, adjacency list, CSR — y cuál usar en cada circunstancia de un motor de BBDD.)*
