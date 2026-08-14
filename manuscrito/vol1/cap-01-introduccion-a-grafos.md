# Capítulo 1 — Introducción a grafos

Llevas toda la vida trabajando con grafos y no lo sabías. Tu familia de WhatsApp es un grafo. Las tuberías de tu casa son un grafo. El cerebro humano es un grafo. La pregunta no es "qué es un grafo", sino "¿por qué nadie me lo explicó así antes?"
## 1.0 La anécdota de la esquina

Once de agosto de 1735, un domingo. No había fútbol, no había Netflix, ni siquiera había ocurrido la Revolución Francesa todavía. Leonhard Euler, matemático suizo instalado en San Petersburgo, se aburre. Bueno, en realidad no se aburre: le han encargado un problema de la ciudad de Königsberg (actual Kaliningrado), que tiene una peculiaridad geográfica bastante irritante. La ciudad está atravesada por el río Pregel, y en el río hay dos islas conectadas entre sí y con las dos orillas mediante siete puentes. Los vecinos llevan décadas paseando por los puentes y nadie ha encontrado una ruta que cruce cada puente exactamente una vez. ¿Es imposible? ¿O simplemente nadie ha sido lo bastante listo?

Euler, que era de los que se iluminaba con este tipo de retos, se sentó, pensó, y en 1736 publicó «Solutio problematis ad geometriam situs pertinentis», un paper que muchos historiadores consideran el **primer artículo de teoría de grafos de la historia**. Su truco fue brutal en su sencillez: ignorar la geografía. No le importaba la forma de los puentes ni la distancia entre las islas. Solo le importaba qué trozo de tierra se conecta con qué otro trozo de tierra. Dibujó cuatro puntos (las dos orillas y las dos islas) y siete líneas (los puentes). Y con esa abstracción demostró lo que todo Königsberg sospechaba: no, no se puede. No existe tal paseo.

La moraleja lleva 290 años vigente: a veces, para resolver un problema, lo mejor es tirar la geografía a la basura y quedarte con el **esqueleto** de las conexiones. Eso, amig@ lector, es un grafo.


> — Oye, ¿y si tengo 10.000 amigos en Facebook? ¿Es un grafo?
> — Sí, pero es un grafo muy triste: tiene 10.000 vértices y solo 50.000 aristas, porque la gente no se hace amiga de todos. La densidad es bajísima. Los grafos grandes y dispersos son la norma, no la excepción.
> — Ah, entonces esto no es como las matrices cuadradas que estudié en el colegio.
> — No. Aquí las matrices son una opción, no la única. Bienvenido al mundo real.
## 1.1 ¿Qué demonios es un grafo?

Vale, ya has oído la palabra "grafo" mil veces. Pero, ¿qué es? Te lo digo de tres formas:

1. **Definición formal:** un grafo es un par $G = (V, E)$ donde $V$ es un conjunto de **vértices** (o nodos) y $E$ es un conjunto de **aristas** (o enlaces), donde cada arista es un par de vértices.
2. **Definición de programador:** un grafo es una estructura de datos que guarda qué cosas están conectadas con qué cosas.
3. **Definición de cuñado:** "es como un mapa mental, pero en plan serio".

Las tres son correctas. Quédate con la 2.

Piensa en un grafo como la red de contactos de tu móvil. Cada persona es un **vértice**. Que alguien te tenga en su agenda es una **arista**. ¿Que sigues a alguien en Instagram y él no te sigue a ti? Eso ya no es una arista "normal": es una arista **dirigida**. ¿Que sois amigos mutuos en Facebook? **No dirigida**. ¿Que en tu app de citas has puesto que esa persona te "gusta mucho"? Eso es una arista con **peso** (peso = cuánto te gusta, del 1 al 10).

## 1.2 Los adjetivos del grafo

Un grafo, por sí mismo, es un objeto bastante soso. La gracia viene con los adjetivos. Estos son los que te van a acompañar siempre:

- **Dirigido vs. no dirigido.** En un grafo no dirigido, la arista {A, B} es lo mismo que {B, A}. En uno dirigido, A→B no implica B→A. Ejemplo: Twitter es dirigido (puedes seguir a alguien sin que te siga); Facebook es no dirigido (la amistad es mutua).
- **Ponderado vs. no ponderado.** Las aristas tienen un número asociado (peso, coste, distancia…). Google Maps es ponderado: cada carretera tiene su tiempo en minutos. Tu lista de amigos de la infancia es no ponderada: o son amigos, o no lo son.
- **Simple vs. multigrafo.** En un grafo simple, entre dos vértices hay a lo sumo una arista, y nadie puede tener una arista a sí mismo (eso es un **bucle** y queda feo en un grafo simple). En un multigrafo puede haber varias aristas entre el mismo par de vértices. Ejemplo: las tuberías del vecino de arriba (si tiene fugas, hay varios caminos entre el contador y el grifo de la cocina).
- **Conexo vs. disconexo.** Un grafo es conexo si puedes ir de cualquier vértice a cualquier otro siguiendo aristas. Si hay un vértice aislado por ahí en medio, el grafo es disconexo.

## 1.3 Las familias notables: quién es quién

No todos los grafos son iguales. Hay unos cuantos "apellidos" que vas a ver mucho y conviene reconocer a primera vista:

- **Camino** $P_n$: una hilera de $n$ vértices conectados uno detrás de otro. Como las paradas del autobús.
- **Ciclo** $C_n$: un camino al que le pegas el primer vértice con el último, formando un círculo. Como la línea 6 de metro.
- **Grafo completo** $K_n$: todo conectado con todo. $K_3$ es un triángulo. $K_5$ ya empieza a ser un dibujo complicado de hacer sin que se te crucen las líneas.
- **Grafo bipartito**: los vértices se dividen en dos grupos, y las aristas SOLO van de un grupo al otro. Nunca dentro del mismo grupo. Piensa en actores y películas: un actor nunca se empareja consigo mismo, solo con películas.
- **Árbol**: un grafo conexo sin ciclos. Es la estructura de carpetas de tu ordenador, o el árbol genealógico de la familia (de ahí el nombre, qué ingeniosos somos).
- **Grafo regular**: todo vértice tiene el mismo grado. Un cubo dibujado en 2D es un grafo regular de grado 3.

## 1.4 Conceptos básicos que te van a salir mil veces

- **Grado** de un vértice: cuántas aristas inciden en él. En Twitter sería tu número de seguidores. (Y no, no es lo mismo que en Facebook: ahí es la suma de tus amigos y los amigos de tus amigos y… bueno, no, no es lo mismo.)
- **Vecindad** $N(v)$: el conjunto de vértices adyacentes a $v$.
- **Adyacencia**: dos vértices son adyacentes si hay una arista que los une.
- **Paseo**: una secuencia de vértices conectados por aristas (puede repetir vértices y aristas).
- **Camino**: un paseo que NO repite vértices.
- **Ciclo**: un camino cerrado (empieza y termina en el mismo vértice, sin repetir nada más por medio).

## 1.5 Dibuja, dibuja, dibuja

Un grafo se representa siempre igual: puntitos (vértices) y rayitas (aristas). Aquí tienes uno pequeñito:

```
    A --- B
    |     |
    |     |
    C --- D --- E
```

Es un grafo no dirigido, no ponderado, con 5 vértices y 6 aristas. El vértice D tiene grado 3 (conecta con C, con B y con E). El vértice E tiene grado 1 (solo conecta con D), así que es un **vértice hoja** o **pendiente**.

Y aquí, un ejemplo de grafo dirigido y ponderado (un trocito de mapa de metro con tiempos):

```
    A --5-- B
    |       |
    3       2
    v       v
    C --4-- D
```

Las flechitas indican dirección, los números, el peso (minutos andando, o en metro, lo que prefieras).

## 1.6 ¿Y esto para qué sirve?

Te preguntarás: «vale, muy bonito, ¿pero yo qué gano con esto?». Mucho. Los grafos están en todas partes:

- **Redes sociales**: Facebook modela la amistad como un grafo no dirigido. Twitter, dirigido.
- **Mapas y navegación**: Google Maps es un grafo ponderado enorme. Cada intersección es un vértice; cada calle, una arista con peso = tiempo medio.
- **Compiladores**: el código se convierte en un **grafo de flujo de control** para detectar errores, optimizar, etc.
- **Biología**: las proteínas interactúan en grafos; las cadenas alimenticias son grafos; el cerebro, dicen, también.
- **Machine Learning**: las redes neuronales son grafos dirigidos con pesos en las aristas (¡hola, deep learning!).
- **Redes de computadores**: Internet es un grafo de routers y enlaces. Internet de las Cosas, también.
- **Planificación de proyectos**: Pert/CPM usa grafos para saber qué tarea va antes de qué otra.

Si algo tiene "conexiones" y "cosas", probablemente se puede modelar como un grafo.

## 1.7 El grafo de tu grupo de WhatsApp

Para terminar el capítulo, vamos a hacer algo friki y bonito: dibujar el grafo de tu grupo de WhatsApp de la familia. ¿Te animas? Te lo explico:

1. Cada persona del grupo es un vértice. Dibújalo como un círculo con su nombre.
2. ¿Quién habla con quién a menudo? Traza una arista.
3. ¿Hay mensajes en una sola dirección? (Por ejemplo, tu madre siempre habla con tu tía, pero tu tía casi nunca contesta). Entonces es dirigida.
4. ¿Quién es el "centro" del grupo, el que más mensajes pone? Ese es el vértice con más grado. En teoría de grafos, si quieres ser fancy, le llamas el de mayor **centralidad de grado**.

Cuando lo tengas, fíjate: ¿es conexo? ¿Hay algún "puente" (una arista que, si la quitas, parte el grupo en dos)? ¿Hay ciclos (conversaciones que vuelven sobre sí mismas)? Estás haciendo teoría de grafos sin darte cuenta.

## 1.8 Glosario rápido

1. **Arista**: conexión entre dos vértices. Sin más.
2. **Vértice** (o nodo): un punto del grafo. Una "cosa".
3. **Grafo dirigido**: las aristas tienen dirección (flecha).
4. **Grafo no dirigido**: las aristas no tienen dirección (rayita).
5. **Grafo ponderado**: las aristas llevan un número (peso).
6. **Grado**: número de aristas que tocan un vértice.
7. **Vecindad**: vértices adyacentes a uno dado.
8. **Adyacente**: conectado directamente por una arista.
9. **Camino**: secuencia de vértices sin repetir.
10. **Ciclo**: camino que vuelve al inicio sin repetir.
11. **Paseo**: camino "vago" que puede repetir vértices.
12. **Árbol**: grafo conexo sin ciclos.
13. **Grafo bipartito**: vértices en dos grupos, aristas solo entre grupos.
14. **Grafo completo** $K_n$: todo conectado con todo.
15. **Multigrafo**: puede tener varias aristas entre el mismo par.
16. **Bucle**: arista de un vértice a sí mismo.
17. **Camino simple**: igual que camino, sin repetir vértices.
18. **Vértice aislado**: vértice sin ninguna arista.
19. **Vértice hoja** (pendiente): vértice de grado 1.
20. **Subgrafo**: un grafo dentro de otro (usando parte de sus vértices y aristas).

## 1.9 Ejercicios resueltos

**Ejercicio 1.1 (F).** Dado el siguiente grafo, di su número de vértices, aristas y el grado de cada vértice.

```
    A -- B
    |    |
    C -- D
    |
    E
```

**Solución.** Vértices: {A, B, C, D, E} → 5 vértices. Aristas: {A-B, A-C, B-D, C-D, C-E} → 5 aristas. Grados: A=2, B=2, C=3, D=2, E=1. Suma de grados = 10 = 2 × aristas ✓ (esto siempre se cumple, se llama el **handshaking lemma**).

**Ejercicio 1.2 (F).** ¿Es bipartito este grafo?

```
    1 -- 2
    |    |
    3 -- 4
```

**Solución.** Sí. Ponemos {1, 4} en el grupo A y {2, 3} en el grupo B. Las cuatro aristas (1-2, 1-3, 2-4, 3-4) van de un grupo al otro. ✓

**Ejercicio 1.3 (M).** ¿Puede existir un grafo con 5 vértices, todos de grado 4? Justifica.

**Solución.** La suma de grados sería 5·4 = 20, que tiene que ser par (handshaking lemma). 20 es par, OK. Y como máximo cada vértice puede tener grado n−1 = 4. Por tanto sí: el grafo completo $K_5$ cumple exactamente eso (cada vértice conectado con los otros 4).

## 1.10 Ejercicios propuestos

1. **(F)** Dibuja $K_4$ y di cuántos vértices y aristas tiene.
2. **(F)** ¿Cuántas aristas tiene $C_6$? ¿Es bipartito?
3. **(F)** Dibuja un árbol con 6 vértices. ¿Cuántas aristas tiene? (Pista: siempre n−1.)
4. **(M)** Demuestra que todo árbol con al menos 2 vértices tiene al menos 2 hojas.
5. **(D)** El **handshaking lemma** dice que la suma de los grados es par. Demuéstralo.

## 1.11 Lo que te llevas

- Un **grafo** es un par (V, E) que modela **cosas** y **conexiones** entre cosas.
- Los grafos pueden ser **dirigidos** o **no dirigidos**, **ponderados** o **no**, **simples** o **multigrafos**.
- Las familias importantes son: camino $P_n$, ciclo $C_n$, completo $K_n$, **bipartito**, **árbol**, regular.
- Los conceptos clave son: **grado**, **vecindad**, **adyacencia**, **camino**, **ciclo**.
- Los grafos están en todas partes: redes sociales, mapas, compiladores, biología, ML.

## 1.12 Ojo, cuidado con…

- **Confundir "dirigido" con "ponderado".** Son propiedades independientes. Un grafo puede ser dirigido y no ponderado, no dirigido y ponderado, las dos cosas, o ninguna.
- **Asumir que un grafo es conexo sin comprobarlo.** Un grafo puede tener varios componentes aislados.
- **Olvidar el handshaking lemma.** La suma de los grados SIEMPRE es par. Si te sale impar, te has equivocado contando aristas.
- **Llamar "grafo" a un árbol y al revés indistintamente.** Todo árbol es un grafo, pero no todo grafo es un árbol. El árbol es el que no tiene ciclos.

## 1.13 Para profundizar

1. Euler, L. (1736). "Solutio problematis ad geometriam situs pertinentis". *Commentarii academiae scientiarum Petropolitanae*.
2. Cormen, T. H., Leiserson, C. E., Rivest, R. L., Stein, C. (2009). *Introduction to Algorithms* (3rd ed.), capítulo 20. MIT Press. (CLRS para los amigos.)
3. Sedgewick, R. (2011). *Algorithms* (4th ed.), capítulos 4 y 5. Addison-Wesley.
4. Gross, J. L., Yellen, J. (2005). *Graph Theory and Its Applications* (2nd ed.). CRC Press.
5. West, D. B. (2001). *Introduction to Graph Theory* (2nd ed.). Prentice Hall. (El libro de cabecera de cualquier teórico de grafos.)

## 1.14 Pin de batalla

- **Antes de optimizar nada, modela en papel.** Diez minutos dibujando el grafo en una servilleta ahorran horas de código malgastado.
- **Si tu grafo tiene < 1000 nodos, casi cualquier representación sirve.** No te compliques.
- **Si tiene > 1 millón, olvida la matriz de adyacencia y pasa a listas o `petgraph`.** Tu RAM te lo agradecerá.
- **Todo grafo de tu vida diaria cabe en uno de estos: redes sociales, mapas, dependencias, dinero, conocimiento.** Aprende a ver cuál.
- **Dibuja siempre.** Aunque sea cutre, con bolígrafo en un pósit. La vista humana detecta patrones que la cabeza no.


## 1.15 Si solo lees 30 segundos

Un grafo es un par de conjuntos (V, E): vértices y aristas. Con eso modelas desde tu familia hasta internet. Lo demás son algoritmos que viven encima.

## 1.16 Una historia pequeña

Elena tenía 19 años y odiaba las matemáticas. Un día, su hermano —ingeniero de teleco— le dijo: "¿sabes qué? Un grafo es como tu grupo de WhatsApp, pero en serio." Elena dibujó los nombres de sus 14 amigos más cercanos en un papel y los conectó con líneas según con quién hablaba cada día. El dibujo era un grafo. "Ah," dijo, "esto sí lo entiendo." Empezó a leer sobre teoría de grafos esa misma semana. Tres años después, estaba haciendo el doctorado en análisis de redes sociales. A veces, un ejemplo tonto cambia una vida.


---

