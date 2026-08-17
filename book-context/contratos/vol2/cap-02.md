# CONTRATO DE CAPÍTULO — Vol.II Cap. 2: Cómo representar un grafo en memoria

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Capítulo **CONCEPTUAL**
> de la Parte I del Vol.II (sin código propio en `vol2-liradb`; el código
> empieza en el cap. 7). El ángulo es el **MOTOR DE BASE DE DATOS**, NO el
> algoritmo en sí: el Vol.I (caps. 2-3) ya cubrió las representaciones como
> artefactos algorítmicos; aquí las re-orientamos hacia «¿qué guarda un motor?»
> y preparamos el terreno del cap. 8 (`trait GraphStore` + `MemoryStore`) y del
> cap. 14 (CSR persistente sobre páginas).
>
> **Código ancla (se lee, no se escribe)**: `liradb-workspace/crates/vol2-liradb/src/cap08_graph_store.rs`
> — el `MemoryStore` implementa **listas de adyacencia** con
> `adj_out: Vec<Vec<EdgeId>>` y `adj_in: Vec<Vec<EdgeId>>`. Esa elección
> concreta (lista de adyacencia + dirección inversa) es el corazón conceptual
> del capítulo.
>
> **Ganchos salientes**: cap. 3 (identidad estable, `slotmap`); cap. 8 (el
> `trait GraphStore` que cristaliza las decisiones de este capítulo en API);
> cap. 14 (el CSR que aquí se nombra y se dibuja, pero se construye y se
> persiste allí).
>
> **Responde la pregunta crítica de CORPUS** `vol-II-cap-02`:
> «¿Cuándo usar edge list, adjacency list y CSR en una BBDD?»
> («¿por qué `slotmap` y no índices reciclados?» se pospone a cap. 3 y solo
> se nombra como out_of_scope aquí).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda al llegar aquí**: qué es un grafo
  (`V` vértices + `E` aristas, dirigido/no dirigido, grado, adyacencia) y la
  definición de un property graph «esqueleto vestido» del cap. 1 (datos
  adjuntos + etiquetas + identidad estable). Conoce, del Vol.I (caps. 2-3),
  que hay tres formas de **mirar** un grafo en memoria — *edge list*,
  *adjacency list*, CSR —, y reconoce una matriz de adyacencia cuando la
  ve. Entiende BFS/DFS a nivel de idea (Vol.I, caps. 4-5) y maneja la
  notación O(...) básica, sabiendo que V² es mucho mayor que V para grafos
  grandes. Ha leído que `MemoryStore` (cap. 8) guarda
  `adj_out: Vec<Vec<EdgeId>>` y `adj_in`.
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**:
  1. «La matriz de adyacencia es *la* forma de guardar un grafo.» No: es la
     que ocupa O(V²) y desperdicia el 99,99 % en grafos reales (dispersos);
     su acceso O(1) a la arista (u,v) NO es la operación que una BBDD ejecuta
     un millón de veces al día.
  2. «Una representación es mejor que otra y ya está.» No: NO existe la
     representación ganadora. La pregunta que desbloquea es siempre «¿qué
     operación ejecuta la BBDD?», y cada representación es la mejor para
     UNA familia de operaciones y la peor para OTRAS.
  3. «En una lista de adyacencia, "¿existe la arista u→v?" es O(1).» No: es
     O(grado de u). Lo que es O(1) es *localizar* la lista; *recorrerla*
     cuesta lo que tarda. En un BFS lo que importa es el recorrido.
  4. «La representación se decide una vez y se olvida.» No: persigue al
     sistema entero — del `trait GraphStore` (cap. 8) al formato persistente
     (cap. 14). Cambia cuando cambia el patrón de acceso dominante.
  5. «Guardar en memoria = guardar en disco.» No: en memoria importa la
     localidad de caché; en disco, la página y la contigüidad. La
     representación que brilla en RAM puede ser un desastre persistida.
  6. «Edge list, adjacency list y CSR son tres nombres para lo mismo.» No:
     edge list = `Vec<(u,v)>` plano; adjacency list = `Vec<Vec<...>>` por
     nodo; CSR = arrays planos `offsets` + `targets`. Tres estructuras, tres
     perfiles de coste.
- **NO debe saber todavía**: encoding en bytes (cap. 9), slotted pages
  (cap. 11), CSR **persistente** sobre páginas (cap. 14, se dibuja y se
  nombra como anticipo, pero NO se explica su layout en disco), IDs
  generacionales / `slotmap` (cap. 3, solo se nombra como «luego lo verás»),
  buffer pool y localidad de disco (caps. 12-13), el `trait GraphStore`
  completo (cap. 8), índices (cap. 15). Se cortan nombrando el capítulo
  futuro como referencia.

## 2. Conceptos (del grafo curricular)

- `present` (se introducen por primera vez):
  - **Matriz de adyacencia** como representación para una BBDD: O(V²) de
    espacio, acceso a una arista concreta en O(1), desastre estructural en
    grafos dispersos.
  - **Lista de adyacencia** como la elección natural del motor en memoria
    (lo que el `MemoryStore` del cap. 8 implementa): O(V+E), iterar vecinos
    en O(grado de u).
  - **Lista de aristas / edge list**: O(E), compacta, mala para
    «vecinos de u», buena para carga/importación/respaldo.
  - **CSR** como «lista de adyacencia comprimida en arrays planos»:
    `offsets` acumulados + `targets` aplanados. O(V+E), contigua en
    memoria. La BBDD la usará al persistir (cap. 14).
  - **Densidad** (E vs V²) y **grado medio** como brújula para elegir.
  - **Dirección inversa** (`adj_in`): por qué un motor que ejecuta PageRank
    o «¿quién apunta a X?» mantiene una segunda lista (forward + backward).
  - **Localidad de caché**: por qué arrays planos contiguos (CSR) ganan a
    listas dispersas (`Vec<Vec>`) cuando lo que importa es *leer en
    ráfaga*.
  - Lema rector del capítulo y de toda la Parte I: **la representación se
    elige por el patrón de acceso, no al revés**.
- `practice`: usar O(...) conscientemente para comparar espacio y tiempo;
  derivar «costo de guardar» vs «costo de responder» de una misma
  estructura; conectar cada representación con la operación de BBDD que la
  hace brillar (BFS → lista; PageRank / entrantes → lista inversa;
  «¿existe esta arista?» → matriz en grafo pequeño y denso; persistir
  barato → CSR).
- `consolidate`: definición de grafo, vértice, arista, grado, adyacencia,
  recorrido BFS (Vol.I, caps. 2-5); el «esqueleto vestido» del cap. 1; el
  `MemoryStore` del cap. 8 como concreto de lo que hasta ahora era
  abstracto.
- `out_of_scope` (se nombran como «luego lo verás», sin explicar): CSR
  **persistente** sobre páginas (cap. 14), IDs generacionales / `slotmap`
  (cap. 3), buffer pool y localidad de disco (caps. 12-13), índices
  (cap. 15), el `trait GraphStore` completo (cap. 8), encoding (cap. 9).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge** (afirmaciones comprobables):
  1. Dado un grafo, sabe decir cuánta memoria ocupa cada representación
     en O(...): matriz O(V²), lista de adyacencia O(V+E), edge list O(E),
     CSR O(V+E) en arrays planos.
  2. Sabe decir qué operación hace **barata** cada representación
     («existe(u,v)» → matriz; «vecinos(u)» → lista / CSR; «exportar
     todo» → edge list) y cuál la hace **cara**.
  3. Sabe que `MemoryStore` del cap. 8 implementa listas de adyacencia
     (`adj_out`/`adj_in`) y **por qué** esa elección es razonable para
     un primer motor en memoria que muta con frecuencia.
  4. Puede explicar la **densidad** de un grafo (E / V²) y por qué los
     grafos reales son dispersos, lo que vuelve a la matriz casi siempre
     un desperdicio.
  5. Puede **definir el CSR** como «lista de adyacencia comprimida en
     arrays planos», citar su origen (Yale Sparse Matrix Package, 1977)
     y anunciar que volverá en el cap. 14 para persistir adyacencias.
- **Skills** (tareas que ejecuta):
  1. Dibujar el **mismo** grafo de 4 nodos en matriz, lista y CSR, y
     comprobar que las tres representan el mismo grafo.
  2. Calcular a mano la **memoria O(...)** de cada representación para un
     grafo dado (V, E, grado medio) y descartar las que no caben.
  3. Dada una operación de BBDD (BFS, PageRank, «¿existe esta arista?»,
     «exportar todo»), elegir la representación que la hace barata y
     justificar el trade-off contra al menos una alternativa.
  4. Reconocer en código real (`MemoryStore`, cap. 8) que la elección
     «lista de adyacencia» no es decorativa: es la respuesta a una
     pregunta de acceso concreta, y viene acompañada de su espejo
     `adj_in`.
- **Wisdom** (cuándo NO / qué trade-off pesa más):
  1. **NO** elige representación «porque sí»: pregunta primero
     «¿qué operación ejecuta la BBDD un millón de veces al día?», porque
     la misma estructura que vuelve trivial un BFS es un despropósito
     para «¿existe la arista (u,v)?» en un grafo denso.
  2. Pesa que en un **motor** la elección no es solo velocidad en RAM:
     cuando llegue el cap. 14 la representación dictará cuánto disco, con
     qué localidad y con qué formato se escribe. La decisión se hace hoy
     pensando en el después.

## 4. Modelo mental

- **La figura que ordena todo**: el **mismo grafo de 4 personas contado de
  tres maneras** — celda de una rejilla (matriz), entrada de un catálogo
  por persona (lista), tramo comprimido de una carretera de una sola vía
  (CSR). Es la ilustración rectora del capítulo: tres espejos, no tres
  grafos.
- **Diagrama(s) ASCII obligatorio(s)**:
  - §2.3 — el mismo grafo en tres columnas (matriz / lista / CSR), con
    Ana, Bruno, Carlos y Dana y aristas que reaparecen como `1` en
    matriz, como `EdgeId` en lista y como `target` en `targets`.
  - §2.6 — variantes sintéticas de la lista de adyacencia (forward) y de
    la edge list, para anclar la diferencia.
- **El momento ¡ajá!**: **cada representación hace trivial una pregunta
  distinta y miserable otra.** La matriz es trivial para
  «¿existe(u,v)?»; la lista y el CSR son triviales para
  «¿vecinos de u?»; ninguna es trivial para todo a la vez. Guardar un
  grafo no es elegir una figura bonita: es decidir **a qué operación le
  vas a regalar la velocidad** y a cuáles vas a cobrar más caro. La
  pregunta que desbloquea todo es una sola: **¿qué operación ejecuta la
  BBDD?**.

## 5. Los porqués (grill — la pregunta más importante de cada decisión)

### 5.1 ¿Por qué la matriz de adyacencia para «existe(u,v)» y no para BFS?
- **Resuelve**: comprobar presencia de una arista en O(1) por doble
  índice. Es el primer dibujo del libro de algoritmos (CLRS cap. 22) y la
  intuición más rápida de enseñar.
- **Se descartó para BFS** porque recorrer todos los vecinos de u
  obliga a barrer una fila completa: O(V), aunque el grado de u sea 4.
  Precio: **O(V²) de memoria**.
- **Si no lo supiéramos**, guardarías un grafo de 1 M de nodos disperso
  como matriz: ~4 TB (u32) o ~125 GB (1 bit). Es el mismo argumento de
  densidad que el cap. 14 convierte en cifra con tinta.
- **Evidencia**: análisis de densidad del cap. 14; CLRS, *Introduction to
  Algorithms*, cap. 22 (presenta las dos opciones canónicas y los costes
  O(V²) y O(V+E)).

### 5.2 ¿Por qué la lista de adyacencia para «vecinos de u»?
- **Resuelve**: iterar vecinos en O(grado(u)), con espacio total
  **O(V+E)**.
- **Se descartó** para «¿existe u→v?» porque exige barrer la lista
  (O(grado)).
- **Modo de fallo**: para PageRank, que recorre *entrantes*, una sola
  lista no basta. El `MemoryStore` del cap. 8 mantiene la segunda
  (`adj_in`), y esa duplicación es la semilla del par forward / backward
  del cap. 14. Precio oculto: sincronizar las dos listas al borrar —
  `delete_edge` del cap. 8 hace `retain` en `adj_out[source]` *y* en
  `adj_in[target]`.
- **Evidencia**: `liradb-workspace/crates/vol2-liradb/src/cap08_graph_store.rs`
  (líneas 79-80: `adj_out: Vec<Vec<EdgeId>>`, `adj_in: Vec<Vec<EdgeId>>`).

### 5.3 ¿Por qué CSR?
- **Resuelve**: las mismas ventajas de la lista de adyacencia pero en
  arrays planos contiguos: mejor **localidad de caché** y un formato
  trivialmente persistible. La lista de Ana deja de ser un `Vec` con su
  propia alocación y pasa a ser `targets[offsets[Ana]..offsets[Ana+1]]`.
- **Se descartó** (hasta el cap. 14) por ser incómodo de *mutar* en
  caliente: insertar una arista puede empujar todo el array `targets` y
  re-encajar los `offsets`. En memoria el `Vec<Vec>` es más cómodo para
  insertar / borrar; el CSR brilla cuando lees y persistes, no cuando
  editas al vuelo.
- **Evidencia**: Yale Sparse Matrix Package, 1977 (informes
  YALEU/DCS/RR-112 y RR-114, Eisenstat, Gursky, Schultz y Sherman);
  cap. 14 del Vol.II.
- **Si no lo conociéramos**, no sabríamos por qué el cap. 14 elige
  arrays planos para bajar la adyacencia a páginas.

### 5.4 ¿Por qué existe la edge list?
- **Resuelve**: el formato más compacto para guardar **todas** las
  aristas (O(E)), y el formato natural de una carga por lotes y de un
  respaldo.
- **Se descarta** para BFS porque no hay forma de saltar a «los vecinos
  de u» sin escanear la lista completa (O(E)). Una consulta
  «vecinos(u)» se vuelve O(E); un BFS sobre edge list degrada a O(V·E).
- **Es la diferencia** entre el inventario de un almacén (edge list) y
  el plano de los pasillos por los que caminas cada día (lista / CSR).

### 5.5 ¿Por qué mantener dirección inversa (`adj_in`)?
- **Resuelve**: PageRank, «¿quién apunta a X?», shortest-path con pesos
  invertidos, recálculo de centralidad — todas las operaciones que
  recorren el grafo «al revés».
- **Se descartó** la opción de calcularla al vuelo sobre `adj_out`
  porque costaría O(E) por consulta. Mantenerla duplicada cuesta el
  doble de espacio en adyacencia pero convierte cada consulta entrante
  en O(grado_in(u)).
- **Evidencia**: `MemoryStore::in_edges(u)` en
  `cap08_graph_store.rs`; referencia anticipada al par CSR forward /
  backward del cap. 14.

### 5.6 ¿Resuelve el cap. 2 los IDs generacionales?
- **No lo resuelve; lo pospone**. La pregunta del CORPUS
  «¿por qué `slotmap` y no índices reciclados?» se responde en el
  cap. 3. Aquí solo se fija el QUIJADA: el `usize` que identifica un
  nodo NO es una posición re-numerable, es una **clave estable** (así lo
  usa `MemoryStore` con `Vec<Option<Node>>` y huecos en vez de
  re-numerar al borrar).

## 6. Primera solución vs solución evolucionada

- **Versión ingenua que escribiría un novato**: la **matriz de
  adyacencia**. `bool[u][v]`, o `for i in 0..n { for j in 0..n }`.
  Es lo primero que imprime el libro de algoritmos. Bonita, simétrica,
  evidente.
- **Qué la rompe exactamente**: un grafo real. 1 M de nodos → 10¹²
  celdas. Aunque solo haya 4 M de aristas (densidad 4·10⁻⁶), la matriz
  pide TB. Un BFS sobre la fila 7 tarda O(V) en encontrar 4 vecinos.
  La matriz responde a la pregunta que un motor casi nunca hace
  («¿existe esta arista?» un par de veces) y cobra a precio de fila
  entera la pregunta que sí ejecuta un millón de veces
  («vecinos de u»).
- **Cómo evoluciona en el capítulo**: no es que se «mejore la matriz»;
  es que se **IRRADIA** a otras representaciones y se aprende a
  elegir. La matriz sigue siendo la mejor para «existe(u,v)» en grafos
  pequeños y densos; la lista de adyacencia es la que `MemoryStore`
  implementa (cap. 8); la edge list es la de cargas y backups; y el CSR
  — la «lista comprimida» — es la que la BBDD acabará persistiendo
  (cap. 14). La evolución no es lineal: es cambiar de lente según la
  operación.

## 7. Prueba de fuego

Capítulo **conceptual**: la prueba de fuego es **decisional y de
cálculo**, no un test de `cargo`. El lector demuestra que lo aprendido
funciona cuando:

1. **Calcula la memoria** (ejercicio «analizar»): para un grafo dado
   (V, E, grado medio, `u32` para ids), escribe el coste O(...) de
   matriz, lista de adyacencia, edge list y CSR, y decide cuál compite
   y cuál no en una máquina típica de ~16 GB de RAM.
2. **Elige la representación** (ejercicio «crear»): dado un perfil de
   carga (BFS dominante, PageRank periódico, «¿existe esta arista?»,
   «exportar todo»), selecciona la(s) estructura(s) y justifica el
   trade-off contra al menos una alternativa descartada, conectando
   con `adj_out`/`adj_in` del cap. 8 y con el par forward/backward del
   cap. 14.
3. **Ancla al código**: reconoce que `MemoryStore`
   (`cap08_graph_store.rs`) guarda `adj_out: Vec<Vec<EdgeId>>` y
   `adj_in: Vec<Vec<EdgeId>>`, y puede explicar por qué
   `out_edges(u)` y `in_edges(u)` son baratos en esa representación y
   por qué `delete_edge` debe mantener las dos sincronizadas.

**Síntoma si el lector se salta el capítulo**: llega al cap. 8 sin
entender por qué el trait `GraphStore` expone `out_edges`/`in_edges`
separados ni por qué usar `Vec<Vec<EdgeId>>`; y al cap. 14 sin saber
por qué el CSR es «justo la lista de adyacencia comprimida» ni por qué
dos arrays valen más que una alocación por nodo. **Detectable**:
elegiría matriz de adyacencia para una BBDD (catástrofe de memoria) o
no sabría que tiene que mantener una lista inversa para PageRank.

## 8. Trampas y errores comunes

1. **Creer que la matriz es la representación «correcta»** solo porque
   es la primera que les enseñaron. Detectarlo: el lector NO calcula
   V² antes de descartarla para un grafo grande.
2. **Creer que «vecinos de u: O(1) en lista de adyacencia»**:
   confunden *localizar* la lista (O(1)) con *recorrerla* (O(grado)).
   En BFS lo que importa es el recorrido.
3. **Usar edge list, lista de adyacencia y CSR como sinónimos**: edge
   list = pares planos `(u,v)`; lista = un `Vec` por nodo (posiblemente
   disperso en el heap); CSR = arrays planos `offsets` + `targets`.
   Tres estructuras, tres perfiles de coste, tres usos.
4. **Suponer que la elección en memoria vale para el disco**: lo que
   brilla en RAM (un `Vec<Vec>` disperso) puede ser un desastre
   persistido en páginas. El cap. 14 volverá sobre esta herida.
5. **Pensar que basta una sola representación**: un motor responde
   operaciones distintas; la madurez es saber **cuándo** usar cada
   una (o combinarlas) según la consulta que llega.
6. **Confundir GDBMS con biblioteca de algoritmos**: aquí no eliges
   representación para que un algoritmo «se vea bonito» en una pizarra;
   eliges para que un *motor* responda millones de consultas. Eso
   cambia qué pesa más — el patrón de acceso, no la elegancia.

**Precisión de lenguaje (glosario)**: `vecinos(u)` (toda la lista) ≠
`existe(u,v)` (una celda); **grado** ≠ **grado medio**; **denso**
(E ≈ V²) ≠ **disperso** (E ≤ cV); **CSR** ≠ «matriz» — es una forma
*comprimida* de la lista de adyacencia; **GDBMS / motor de BBDD** ≠
biblioteca de algoritmos; **lista de adyacencia** ≠ **edge list**;
**forward** (`adj_out`) ≠ **backward** (`adj_in`).

## 9. Ejercicios (exercise-designer)

- **`recordar/aplicar` (esencial — retrieval practice)**: SIN mirar el
  texto, el lector dibuja de memoria el mismo grafo («Ana→Bruno,
  Ana→Carlos, Bruno→Carlos, Carlos→Dana») en (a) matriz de adyacencia,
  (b) lista de adyacencia (forward), (c) edge list, e identifica cuál
  estructura le devuelve los vecinos de Bruno sin barrer todo.
  *Pistas*: (1) la arista es `1` en la celda, un elemento en la lista,
  un par `(u,v)` en el edge list; (2) localizar la lista es O(1);
  recorrerla es O(grado); (3) en el edge list no hay «por origen» —
  toda pregunta sobre un nodo exige barrer. *Criterio*: las tres
  figuras representan el mismo grafo.
- **`analizar` (intermedio — calcular memoria)**: dado un grafo de
  **V = 1.000.000** nodos y **E = 4.000.000** aristas (grado medio 4,
  ids de 32 bits = 4 bytes), calcular el coste aproximado de memoria
  de: matriz de adyacencia con `u32`/celda; matriz de adyacencia con
  1 bit/celda; lista de adyacencia (datos + cabecera por `Vec`);
  edge list (un par `(u,v)` de 8 bytes por arista); CSR (`offsets` +
  `targets`). Decir cuáles caben en una máquina de ~16 GB de RAM.
  *Pistas (graduadas)*: (1) la matriz es V² × bytes_por_celda;
  (2) el CSR es la suma de sus dos arrays; (3) la lista de adyacencia
  nunca puede ser más barata que el CSR — solo se le añaden cabeceras
  por `Vec`. *Solución de referencia*: matriz u32 ≈ 4 TB; matriz a
  1 bit ≈ 125 GB; CSR ≈ (1 M + 1)·4 + 4 M·4 ≈ 20 MB; edge list ≈
  8·4 M ≈ 32 MB; lista de adyacencia ≈ CSR + overhead por nodo ≈
  40-50 MB.
- **`crear` (experto — perfil de carga + anclaje al cap. 8 y cap. 14)**:
  una BBDD de grafos declara su perfil de carga: consultas
  dominantes = **BFS desde muchos orígenes**, más un **PageRank**
  periódico que necesita, para cada nodo, sumar la influencia de
  *quienes le apuntan* (vecinos entrantes). Diseña la representación
  en memoria: ¿qué estructura(s) eliges, necesitas dirección inversa,
  contra qué alternativa lo justificas? *Pistas*: conviene una lista
  o CSR por cada dirección para que ambos recorridos sean baratos;
  descarta la matriz por O(V²) y la edge list por «vecinos de u»
  O(E); conecta con `adj_out` + `adj_in` del `MemoryStore` (cap. 8) y
  con el par CSR forward/backward del cap. 14. *Criterio*: argumenta
  en contra de al menos una alternativa y conecta tu decisión con el
  código del cap. 8.

## 10. Preguntas abiertas (gancho al siguiente capítulo)

- Si un `usize` es la clave que nombra a un nodo y NO una posición
  re-numerable, ¿cómo hacemos que la identidad **sobreviva** a un
  crash y a los reciclajes de espacio? ¿Qué estructura garantiza IDs
  estables incluso tras reorganizar el fichero? (cap. 3, Identidad,
  referencias y datos estables — `slotmap`.)
- Cuando el grafo deja de caber en RAM, ¿cómo bajamos estas
  estructuras a páginas de disco sin perder la baratura de acceso?
  (cap. 14, CSR persistente.)
- ¿Cómo sabe la BBDD si una arista de la lista es `KNOWS` o
  `WORKS_AT`, o si lleva la fecha «desde 2020»? Es decir, ¿cómo se
  *tipan* esos datos y propiedades? (cap. 7, el modelo Property
  Graph + `Value`; cap. 9, encoding.)

**Términos nuevos de glosario** (los registra `book-memory-keeper`):
**matriz de adyacencia**, **lista de adyacencia**, **edge list / lista
de aristas**, **CSR (Compressed Sparse Row)**, **densidad**, **grado
medio**, **lista inversa / backward**, **forward / backward**, **patrón
de acceso**, **localidad de caché**, **representación de un grafo**.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el ejercicio `recordar/aplicar` obliga a
  DIBUJAR las tres representaciones (matriz, lista, edge list) desde
  la memoria (recordar > reconocer), sin pistas que las regalen.
- **Spacing**: el capítulo re-ejercita el grafo y el BFS del Vol.I
  (caps. 2-5 — el esencial obliga a reconstruir un grafo y su BFS) y
  adelanta el CSR del cap. 14 (el diagrama del §2.3 planta la semilla
  que el cap. 14 vendrá a cosechar, con las mismas cifras de memoria
  que el ejercicio `analizar` verifica).
- **Interleaving**: el `analizar` mezcla el cálculo O(...) (matemática)
  con la noción de RAM/disco (hardware), y el `crear` mezcla
  estructura × operación de BBDD × patrón de acceso — temas vecinos,
  no clones.
- **Regla de dificultad asimétrica**: en el conocimiento una sola
  idea nueva por sección; en los ejercicios, esfuerzo de recuperación
  y de cálculo.
- **Bucle de feedback inmediato**: el `analizar` tiene dígitos
  verificables contra el análisis del cap. 14; el `recordar/aplicar`
  se auto-verifica al cotejar las tres figuras contra el modelo
  mental del §2.3; el `crear` se verifica al confrontar la decisión
  con `MemoryStore` (`cap08_graph_store.rs`).
- **Citas (alta confianza, no paramétricas)**:
  - **CSR — origen**: Yale Sparse Matrix Package, 1977, informes
    YALEU/DCS/RR-112 y RR-114 (Eisenstat, Gursky, Schultz y Sherman).
  - **Lista de adyacencia — convención canónica**: Cormen, Leiserson,
    Rivest y Stein, *Introduction to Algorithms* (CLRS), cap. 22.
  - **Densidad y dígitos de memoria**: se verifican contra el
    análisis del cap. 14 del Vol.II.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con **alternativa
      descartada, modo de fallo y fuente** (6 filas en §5: matriz,
      lista, CSR, edge list, dirección inversa, IDs generacionales).
- [x] **Escenario de fallo visible**: matriz sobre grafo disperso =
      TB; edge list sobre «vecinos de u» = O(E); CSR para mutar
      mucho = incómodo; lista de adyacencia sin inversa = PageRank
      carísimo.
- [x] Capítulo **conceptual**: no genera código, pero ancla la
      elección al `MemoryStore` del cap. 8 (`adj_out`/`adj_in`) y
      prepara el terreno ejecutable del cap. 14.
- [x] Hay **≥4 misconcepciones corregidas explícitamente** (matriz
      no es «la correcta»; localizar ≠ recorrer; edge list ≠ CSR;
      RAM ≠ disco; biblioteca ≠ GDBMS).
- [x] Ejercicios con **solución verificable**: el `analizar` tiene
      dígitos contra el cap. 14; el `recordar/aplicar` se auto-coteja
      contra §2.3; el `crear` se ancla contra `cap08_graph_store.rs`.
- [x] Hay **≥1 ejercicio de retrieval** (`recordar/aplicar`) y
      **≥1 toque a concepto de capítulo anterior** (Vol.I caps. 2-5,
      BFS y notación O(...)) → spacing.
- [x] Responde la pregunta crítica del CORPUS `vol-II-cap-02`:
      «¿cuándo edge list vs lista de adyacencia vs CSR en una BBDD?»
      —se responde de lleno; «¿por qué `slotmap` y no índices
      reciclados?» se pospone al cap. 3 (nombrado como out_of_scope).
- [x] **Anécdota verificada con fuente**: Yale Sparse Matrix Package
      (1977), informes YALEU/DCS/RR-112 y RR-114.
- [x] **Tesis explícita y repetida**: «la representación se elige
      por el patrón de acceso, no al revés» — aparece en §2.1, §2.6,
      §2.7, §2.9 y se condensa en el **Pin de batalla** §2.13.
- [x] **Mini-diálogo de cierre** (§2.15) y **gancho explícito al
      cap. 3** (identidad estable / `slotmap`).