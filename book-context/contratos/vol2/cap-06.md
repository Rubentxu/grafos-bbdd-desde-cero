# Contrato del Capítulo 6 — Qué convierte un grafo en una base de datos

> Auto-interrogatorio pedagógico previo al capítulo (PLANTILLA-CONTRATO-CAPITULO.md).
> Capítulo de **SÍNTESIS y TRANSMISIÓN** (cierra la Parte I — "pensar en grafos" —
> y abre la Parte II — "construir el motor"). Recapitula los caps. 1-5, argumenta
> POR QUÉ y CÓMO un grafo se convierte en una BASE DE DATOS PERSISTENTE y
> CONSULTABLE, y presenta el **esqueleto del motor LiraDB** organizado en
> **5 pilares** que el resto del libro levanta, parte a parte. Es la carta de
> presentación del PROPÓSITO de LiraDB: guardar grafos (Parte III), consultarlos
> con LiraQL (Parte IV), pesarlos (Parte V), hacerlos fiables y concurrentes
> (Parte VI), dejarlos como producto (Partes VII-VIII).
>
> **Pregunta crítica del CORPUS** (`vol-II-cap-06`): *«¿Cuál es la diferencia entre
> biblioteca de grafos y GDBMS?»*. Prerrequisito: Vol.I completa / Vol.II caps.
> 1-5. Costura hacia atrás: caps. 1-5. Hacia delante: caps. 7-10 (Parte II),
> 11-16 (Parte III), 17-21 (Parte IV), 22-26 (Parte V), 27-30 (Parte VI),
> 31-40 (Partes VII-VIII). NO trae código nuevo: las anclas son tests de caps.
> posteriores (`cap07_modelo`, `cap08_graph_store`, `cap11_slotted_pages`,
> `cap22_caminos_minimos`, `cap24_centralidad`).

---

## 1. El novato (perfil y punto de partida)
- **Sabe YA sin ninguna duda**: grafo `G=(V,E)` y sus tres ropas (cap. 1);
  matriz/lista/CSR elegidas por patrón de acceso (cap. 2); id estable
  `(slot, generation)` y bug ABA (cap. 3); BFS por niveles con cola FIFO y
  marca al descubrir (cap. 4); DFS con tres colores, Kahn, Kosaraju y Tarjan
  (cap. 5). Lector del Vol.I: Euler 1736, Codd 1970, Dijkstra/Moore, BFS O(V+E).
  Base de Rust suficiente.
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**:
  (a) "una biblioteca de grafos ya es una base de datos" — no: le falta
  persistencia, consulta con lenguaje, transacciones, índices;
  (b) "base de datos = fichero donde se vierte el grafo" — no: `fs::write`
  no da consulta, ni índices, ni recuperación ante crash;
  (c) "persistir == recuperarse de un crash" — son cosas distintas:
  persistencia es sobrevivir al cierre (Parte III); durabilidad ante crash
  es transacción + WAL (Parte VI);
  (d) "el algoritmo del Vol.I sirve tal cual sobre datos persistentes" —
  no: falta decidir cómo leer de disco sin agotar la RAM (Parte V, cap. 26);
  (e) "el relacional y el de grafos son especies incompatibles" — no: el
  grafo es la **4.ª generación** de modelos (jerárquico → red → relacional
  → grafo); LiraDB hereda lecciones del relacional deliberadamente.
- **NO debe saber todavía**: `LiraQL` (cap. 17), encoding binario de `Value`
  (cap. 9), `Pager`/`BufferPool` (caps. 12-13), WAL (cap. 28), MVCC
  (cap. 30), PageRank exacto (cap. 24), CSR persistente (cap. 14), CLI
  (cap. 31). Se *nombran* con su capítulo pero no se explican: el capítulo
  es estrictamente conceptual y de mapa.

## 2. Conceptos (del grafo curricular)
- `present` (se introducen por primera vez):
  - **Capacidades** que separan una estructura en memoria de una BD:
    **persistencia, consulta, transacciones, índices** (los 4 pilares
    clásicos).
  - **GDBMS** vs **biblioteca de grafos**: la frontera operativa (responde
    la pregunta crítica del CORPUS).
  - El **viaje de las 4 estaciones** (RAM → disco → consultable →
    transaccional) como metáfora ordenadora de todo el libro.
  - El **esqueleto del motor LiraDB en 5 pilares**: Persistencia
    (Parte III), Consulta (Parte IV), Algoritmos sobre disco (Parte V),
    Fiabilidad (Parte VI), Motor real (Partes VII-VIII).
  - La **historia de los modelos de datos** (jerárquico → red/CODASYL →
    relacional/Codd → grafo/Neo4j-GQL) como genealogía de la que LiraDB
    hereda.
- `practice` (se ejercitan, ya vistos y se reusan): Property Graph y sus
  tres ropas (cap. 1); las tres representaciones de memoria (cap. 2) —
  citadas como la elección que el cap. 14 reorienta a disco; id estable
  `(slot, generation)` (cap. 3) — citado como el prerrequisito que la
  Parte VI hace cierto bajo crash; BFS/DFS/componentes/SCC (caps. 4-5) —
  citados como los *contratos* que la Parte V cumplirá sobre datos
  persistentes.
- `consolidate` (se asumen y reutilizan sin explicar): definición
  `G=(V,E)`, matriz de adyacencia, BFS O(V+E), DFS con tres colores, Kahn,
  Kosaraju/Tarjan.
- `out_of_scope` (se nombran, sin explicar): ACID completo (cap. 27), WAL
  (cap. 28), recuperación (cap. 29), MVCC (cap. 30), CLI/IPC (caps. 31-40),
  RDF/SPARQL (Vol. III), WCOJ (cap. 39), distribución (cap. 40), GQL ISO
  39075 en detalle (caps. 7 y 17). Solo se preanuncian con su capítulo.

## 3. Objetivos de dominio (taxonomía teaching)
- **Knowledge** (qué SABE al terminar — 5 afirmaciones comprobables):
  1. Enumera y explica los **5 pilares** del motor LiraDB y la Parte del
     libro donde se construye cada uno.
  2. Distingue con ejemplos **biblioteca de grafos** de **GDBMS**, y nombra
     al menos dos capacidades que separan a uno de otro (responde la
     pregunta crítica del CORPUS).
  3. Identifica las **4 estaciones del viaje** (RAM → disco → consultable →
     transaccional) y la dependencia secuencial entre ellas (por qué el
     orden del libro no puede alterarse).
  4. Ubica el grafo como **4.ª generación de modelos de datos** y cita al
     menos dos precedentes (Codd 1970; System R / Gray 1976; Neo4j 2007;
     GQL ISO/IEC 39075:2024).
  5. Para cada pilar, asocia **una pregunta de negocio concreta** que lo
     justifica (ej.: "dame los vecinos de Ana con `city = Oporto`" →
     persistencia + índice; "BEGIN; ... COMMIT;" → fiabilidad).
- **Skills** (qué HACE — 2-3 tareas que ejecuta con el código/concepto):
  1. **Dibuja el diagrama de dependencias** entre los 5 pilares ("qué
     necesita qué"), identificando la pieza-bisagra que desacopla
     persistencia de consulta (el trait `GraphStore` del cap. 8).
  2. **Toma un minigrafo de los caps. 1-5 y, para cada pilar, inventa
     una pregunta real** que el grafo-en-RAM no puede responder de forma
     persistente; asocia cada pregunta a una Parte del libro.
- **Wisdom** (qué DECIDE — 1-2 trade-offs):
  1. Decide **cuándo** una aplicación necesita un GDBMS y cuándo le basta
     una biblioteca de grafos (criterio: consultas declarativas,
     persistencia tras cierre, recuperación ante crash o recorrido sobre
     datos que no caben en RAM → GDBMS; solo recorrido sobre grafo en
     RAM → biblioteca).
  2. Decide **qué orden dar a persistir → consultar → algoritmos →
     fiabilidad → producto** en cualquier proyecto de motor: el orden
     no es de "belleza", es de dependencias técnicas.

## 4. Modelo mental
- **Figura ordenadora: el viaje de las 4 estaciones** (RAM → disco →
  consultable → transaccional), con dos vagones laterales: algoritmos
  sobre el grafo persistente (Parte V) y motor real (Partes VII-VIII).
  Cada estación añade lo que la anterior NO podía: la RAM no sobrevive
  al apagado; el disco no se consulta sin un lenguaje; la consulta no es
  segura sin transacciones. El libro es el ferrocarril que une las
  cuatro estaciones más los dos vagones.
- **Diagramas ASCII necesarios**:
  - (a) **Diagrama del viaje**: cuatro vagones en fila con lo que añade
    cada uno (persistencia, consulta, transacción) más los dos vagones
    laterales (algoritmos, producto).
  - (b) **Línea de tiempo de los modelos de datos**: cuatro generaciones
    (jerárquico → red → relacional → grafo) con los hitos Codd 1970,
    System R 1976, Neo4j 2007, GQL 2024.
  - (c) **Tabla "qué pregunta responde cada pilar"**: 5 filas (los cinco
    pilares) con columnas (pregunta que responde, Parte del libro,
    ejemplo de consulta, módulo del workspace que lo materializa).
- **Momento ¡ajá!**: «Toda la Parte I construyó un grafo en RAM: rápido,
  elegante, y muerto al cerrar el programa. Una base de datos es lo que
  empieza donde la RAM acaba: guarda el grafo (persistencia), sabe
  pedirlo (consulta), no lo corrompe si algo falla a medias
  (transacciones) y lo encuentra sin escanear todo (índices). Yo no he
  aprendido "otra cosa": he aprendido la PRIMERA ESTACIÓN del viaje.
  Ahora sé cuál es todo el mapa.»

## 5. Los porqués (grill — la pregunta más importante de cada decisión)
| # | Decisión / concepto | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia (claim_id + fuente) |
|---|---|---|---|---|---|
| 1 | Definir una BD por sus 4 capacidades (persistencia, consulta, transacciones, índices) | Da un criterio OPERATIVO para distinguir «estructura en memoria» de «base de datos» y justifica el PV (por qué construir un motor) | Definir BD por «guarda en disco a secas»: demasiado laxo; cualquier fichero «sería» BD e ignora la consulta | El lector no tendría porqué construir un motor; cualquiera podría creer que `fs::write` ya «es» una BD | **C-CODD-1970-1** (CACM 13(6), data independence); **C-SYSR-1976-1** (Astrahan et al., TODS 1(2), architecture, logging/recovery) |
| 2 | Situar el grafo como 4.ª generación de modelos de datos | Explica POR QUÉ importa (el relacional no podía con redes) y de dónde vienen las lecciones que LiraDB hereda | Tratar los grafos como «otra cosa sin parentesco»: pierde la continuidad histórica y pedagógica | El lector no conecta Codd con Neo4j; pierde el porqué de los pilares | **C-CODD-1970-2** (CACM 13(6), §1: crítica a los modelos de árbol y red); **C-NEO4J-2015** (Robinson, Webber, Eifrem, cap. 1); **C-GQL-2024** (ISO/IEC 39075:2024) |
| 3 | Persistencia = la frontera que «convierte» un grafo en BD | Sin persistencia un recorrido muere con el proceso: es el requisito mínimo no negociable y el que abre el mapa | Poner primero la consulta y «persistir luego»: hay que tener bytes en disco para poder consultarlos (estructura de dependencias) | Motor sin datos persistentes = biblioteca con vocabulario de BD, falsa de solemnidad | cap. 10 (append-only); cap. 11 (slotted page como unidad de disco); Petrov, *Database Internals* (2019) cap. 1 |
| 4 | Presentar 5 pilares del LIBRO (Persistencia/Consulta/Algoritmos/Fiabilidad/Motor) junto a los 4 clásicos | Los 4 clásicos son el *criterio de qué es una BD*; los 5 son el *mapa de este libro*. Afinar consulta/algoritmos como pilares propios refleja que un GDBMS se diferencia por conectar persistencia con recorrido y por llegar a producto | Reducir el libro solo a los 4 clásicos: perdería Parte V (algoritmos) y VII-VIII (producto), que son distintivos de un GDBMS | El árbol de contenidos no se correspondería con los pilares que se publicitan; coherencia rota | `tabla-de-contenidos.md` (Partes III-VIII); prólogo (los 40 caps.); Francis et al., SIGMOD 2018 |
| 5 | LiraDB embedded, monolítico (no servidor de red, no distribuido de inicio) | Enseñanza: la complejidad de red/distribución se pospone (caps. 37/40), y el núcleo (almacenar + consultar) se construye a fondo primero | Servidor desde el cap. 1: antepone la dificultad de red a la de almacenamiento y consulta — el orden pedagógico lo exige | Un motor incomunicable a la vez que inmaduro; el lector se atasca en lujo antes que en base | `brief-liradb-original.md`; prólogo (ruta lineal; LiraDB Lite embedded); Petrov, *Database Internals*, cap. 1 |
| 6 | Anclar cada pilar a UN ejemplo concreto que volverá (`SHORTEST PATH`, `PageRank`, `¿está conectado?`, `MATCH…RETURN`, `BEGIN…COMMIT`) | La retención de un mapa se ancla a casos reconocibles; cada ejemplo reaparece implementado en su capítulo | Hablar de pilares en abstracto (sin ejemplo): el lector asiente pero no recuerda porqué cada uno es necesario | El ejercicio de retrieval (listar pilares y PORQUÉ) no tendría asidero; pérdida de storage strength | cap. 22 (`SHORTEST PATH FROM ... WEIGHT`); cap. 24 (`PageRank`); caps. 7-8 / 17 (`GraphStore` / `MATCH-WHERE-RETURN`); caps. 27-30 (`ACID` / `WAL` / `MVCC`) |

## 6. Primera solución vs solución evolucionada
- **Ingenua (la que ya conoce el lector de caps. 1-5)**: un grafo como
  `Vec<Vec<NodeId>>` o `HashMap` de listas de adyacencia, recorrido con
  BFS/DFS, todo en RAM. Rápido, elegante, y **muere al cerrar el proceso**.
  No persiste, no consulta con un lenguaje, no transacciona, no indexa.
- **Qué la rompe exactamente (5 escenarios)**: (1) cierre del proceso (el
  grafo desaparece); (2) grafo más grande que RAM (no cabe ni para
  serializarlo entero); (3) "dame los vecinos de Ana con `city = Oporto`"
  (la consulta requiere un índice que la solución no tiene); (4) crash a
  mitad de una escritura (fichero corrupto); (5) dos procesos escribiendo
  a la vez (uno pisa al otro). Los 5 escenarios se corresponden uno a uno
  con las 4 estaciones + el vagón de algoritmos.
- **Evolución visible**: declarar el **esqueleto del motor LiraDB en 5
  pilares** — Persistencia (Parte III, caps. 10-16), Consulta (Parte IV,
  caps. 17-21), Algoritmos sobre disco (Parte V, caps. 22-26), Fiabilidad
  (Parte VI, caps. 27-30), Motor real (Partes VII-VIII, caps. 31-40).
  Cada pilar responde a una pregunta que la primera solución deja
  huérfana y se construye en una Parte concreta del libro.

## 7. Prueba de fuego
- **El escenario que demuestra que lo aprendido FUNCIONA** (sin código, de
  coherencia y mapa): el lector mira el `SUMARIO.txt` del Vol.II y, **sin
  abrir ningún capítulo**, dice para cada Parte del libro (III, IV, V, VI,
  VII-VIII) qué pilar levanta y con qué pregunta de negocio. Y al revés:
  dada una pregunta de negocio ("¿está Ada conectada con Zoe?" / "¿cuál
  es el camino más barato?" / "¿se corrompió el grafo tras el último
  crash?"), dice en qué Parte y en qué capítulo se responde. Esto cierra
  el bucle "mapa ↔ territorio".
- **Síntoma si el lector se saltara este capítulo**: llega al cap. 7
  creyendo que es «otro capítulo de datos» y no el PRIMER ladrillo del
  motor; cada pilar futuro (WAL, Volcano, índices) le sonará a piezas
  sueltas en vez de al plan anunciado aquí. La «carta de presentación»
  del PROPÓSITO de LiraDB quedaría sin emitir. Los tests ancla reales
  (que NO son de este capítulo) que el lector correrá pronto:
  `cargo test -p vol2-liradb cap07_modelo` y `cap08_graph_store`.

## 8. Trampas y errores comunes
1. **Creer que una biblioteca de grafos ya es una base de datos.** No:
   la biblioteca no persiste, no cataloga, no consulta con lenguaje ni
   transacciona. La pregunta crítica del CORPUS es exactamente esta
   distinción; en LiraDB la frontera son los 4-5 pilares.
2. **Creer que "base de datos" = "volcado a fichero".** Un `fs::write`
   no da consulta, ni índices, ni recuperación; es solo la primera
   semilla del pilar persistencia (caps. 10-11).
3. **Confundir los 4 pilares clásicos con los 5 del libro.** No
   contradice: el libro REORDENA "consulta" y AÑADE "algoritmos" y
   "producto" porque es un GDBMS con pedagogía propia.
4. **Confundir persistencia con durabilidad/recuperación.** Persistencia
   = "sobrevive al cierre del proceso" (Parte III). Durabilidad ante
   crash = transacción + WAL (Parte VI). El WAL no es "otra forma de
   guardar"; es "no corromper lo guardado si todo se va al carajo a
   mitad".
5. **Pensar que el orden del libro es opinable.** El orden persistencia
   → consulta → algoritmos → fiabilidad → producto NO es estético; es
   de dependencias técnicas. Saltar a la Parte V sin la III deja al
   lector ejecutando Dijkstra sobre `Vec<Vec<NodeId>>` y volviendo a
   empezar cuando llegue a disco.
6. **Creer que el modelo de datos ya está "completo" tras el cap. 7.**
   El cap. 7 define el modelo en RAM; lo que falta es *cómo se
   serializa* (cap. 9) y *cómo se pagina* (cap. 11). Cerrar el cap. 7
   creyendo que "ya tienes una BBDD" es no haber entendido la Parte III.
- **Precisión de lenguaje (glosario)**: *biblioteca de grafos* (API en
  memoria) vs *GDBMS* (persiste, cataloga, consulta, transacciona);
  *persistencia* (sobrevive al proceso) vs *durabilidad* (sobrevive
  además a un crash); *consulta* (lenguaje para pedir, Parte IV) vs
  *algoritmo* (caminos/centralidad, Parte V); *índice* (estructura para
  encontrar sin escanear, cap. 15); *modelo de datos* (qué significa
  cada dato, cap. 7). Glosario futuro: LPG, WAL, MVCC, CSR, Volcano.

## 9. Ejercicios (exercise-designer)
- **recordar/aplicar (esencial — RETRIEVAL puro)**: SIN volver a mirar el
  capítulo ni la tabla de contenidos, escribe de memoria: (a) los **5
  pilares del libro** con su Parte correspondiente y una pregunta de
  negocio por pilar; (b) la frase-unívoca que responde a «¿qué convierte
  un grafo en una base de datos?». Comparar contra §3.1 y §6.10 del
  manuscrito. Pistas (≤3, graduadas): (1) «¿sobrevive al apagado?»; (2)
  «¿se lo pido con un lenguaje?»; (3) «¿y si el proceso se cae a
  mitad?». Criterio: 5/5 pilares correctos, Parte correcta, pregunta
  coherente — escrito desde memoria, no reconocido.
- **analizar (intermedio — SPACING a Vol.I caps. 2/4-5 y Vol.II caps.
  1-5)**: toma el minigrafo social de la Parte I (Ada → Bo → Carla →
  Dani) y para CADA pilar inventa una pregunta REAL que el grafo-en-RAM
  no puede responder de forma persistente, indicando la Parte del libro
  que la resolvería. Pistas: (1) agarra cada pilar uno a uno; (2)
  añade el dato (peso, centralidad, ciudad) que el grafo de caps. 1-5
  no guarda; (3) di la Parte. Criterio: 5 preguntas bien ancladas a
  pilar + Parte.
- **crear (experto — INTERLEAVING + gancho al cap. 8)**: dibuja el
  **diagrama de dependencias** del motor recién anunciado: 5 nodos (un
  pilar cada uno) y una flecha "necesita de" por par. Sobre el grafo,
  **señala la pieza-bisagra que desacopla dos pilares** (pista: el trait
  `GraphStore` del cap. 8 es lo que permite cambiar "disco" sin tocar
  "consulta"). Pistas: (1) un lenguaje de consulta, ¿sobre qué lee?;
  (2) un algoritmo, ¿sobre qué recorre?; (3) una transacción, ¿qué
  protege? Criterio: orden de dependencias correcto y bisagra
  identificada con su nombre.

## 10. Preguntas abiertas (gancho al cap. 7 — y al cap. 8)
- **2-3 preguntas que este capítulo NO responde y el 7 sí**:
  1. ¿Qué tipos de valor puede guardar una propiedad (`Value`) y por qué
     `Null` es explícito? (cap. 7.)
  2. ¿Qué diferencia exactamente a un **LPG** (property graph) de un RDF?
     (cap. 7.)
  3. ¿Qué aspecto concreto tiene el **modelo de datos** de LiraDB: `Value`,
     `Node`, `Edge`, `Element`, `PropertyGraph`? (cap. 7.)
- **Términos nuevos de glosario** (los registra `book-memory-keeper`):
  DBMS / GDBMS, biblioteca de grafos, persistencia, durabilidad, consulta,
  transacción, índice, modelo de datos, generaciones de modelos
  (jerárquico/red/relacional/grafo), `LiraQL` (preanunciado; se define en
  cap. 17), WAL/MVCC/CSR/Volcano (preanunciados como mapa; se definen en
  caps. 14/20/28/30).

## 11. Diseño de retención (skill `teach`)
- **Retrieval practice**: el ejercicio ESENCIAL es de retrieval puro
  (recordar los 5 pilares sin pistas), no de reconocimiento. El lector
  *escribe* desde memoria; no marca opciones.
- **Spacing**: el ejercicio INTERMEDIO reusa explícitamente el minigrafo
  social de la Parte I (Ada/Bo/Carla/Dani) y lo conecta con cada uno de
  los 5 pilares. El capítulo entero es un recapitulador espaciado de la
  Parte I.
- **Interleaving**: el ejercicio EXPERTO dibuja el grafo de dependencias
  entre los 5 pilares — *mezcla* los pilares en un mismo problema, en
  vez de tratarlos uno a uno — y obliga a identificar la pieza-bisagra
  (`GraphStore`), lo que conecta directamente con el cap. 8.
- **Regla de dificultad asimétrica**: para la EXPLICACIÓN, una sola idea
  nueva por sección (qué es BD → historia de modelos → 4 pilares
  clásicos → 5 pilares del libro → viaje de 4 estaciones); para la
  DESTREZA, exigir el mapa de memoria completo y el diagrama de
  dependencias.
- **Bucle de feedback inmediato**: los tres ejercicios se verifican
  contra el §6.6 (tabla de pilares) y §6.10 del manuscrito, y contra el
  cap. 8 ya escrito. Feedback inmediato, no "confía en mí".
- **Citas**: Codd 1970 (data independence; crítica a los modelos de
  árbol y red); Astrahan et al./Gray 1976 (logging, recovery,
  shared-update, arquitectura de un DBMS); Robinson, Webber & Eifrem
  2015 (LPG, GDBMS vs biblioteca); Francis et al. SIGMOD 2018; ISO/IEC
  39075:2024 (GQL); Petrov, *Database Internals*, 2019, caps. 1-3.

---

## Checklist de profundidad (antes de marcar DONE)
- [x] Cada decisión técnica tiene su "porqué" con fuente (Codd 1970;
      Astrahan et al. 1976; Robinson/Webber/Eifrem 2015; ISO/IEC
      39075:2024; Petrov 2019) — 6 filas en §5.
- [x] Existe un escenario de fallo visible (los 5 escenarios del §6.5:
      cierre, grafo > RAM, consulta por índice, crash a mitad, dos
      escritores concurrentes), no solo el happy path.
- [x] El capítulo es conceptual (no ejecuta código nuevo); los
      ejercicios se verifican contra el workspace vía los caps. 7, 8 y
      11 ya escritos.
- [x] Hay al menos una misconception corregida explícitamente
      ("biblioteca de grafos ≠ GDBMS"; "persistir ≠ recuperar"; "volcado
      a fichero ≠ base de datos"; "relacional y grafo no son especies
      incompatibles").
- [x] Los ejercicios tienen solución verificable (caps. 7-8 del
      workspace; §6.6 y §6.10 del manuscrito).
- [x] Hay ≥1 ejercicio de retrieval puro (esencial: listar 5 pilares de
      memoria) y ≥1 toque a concepto de capítulo anterior (el intermedio
      reusa Ada/Bo/Carla/Dani y los conceptos de caps. 1-5).
- [x] El capítulo responde la pregunta crítica de `CORPUS.yml` para su
      id (`vol-II-cap-06`: *«¿Cuál es la diferencia entre biblioteca de
      grafos y GDBMS?»*) — explícitamente, con ejemplos y anclada a Codd
      y Neo4j.
- [x] Presenta los **5 pilares** enumerados y anclados a Partes
      III-VIII con ejemplos que volverán (`SHORTEST PATH`, `PageRank`,
      `¿está conectado?`, `MATCH…RETURN`, `BEGIN…COMMIT`).
- [x] Es un capítulo de SÍNTESIS que cierra la Parte I y abre la Parte
      II, con gancho explícito a caps. 7/8/9 y cierre recursivo de la
      Parte I.
- [x] Anécdota verificada con fuente (Codd 1970; System R/Gray 1976;
      Neo4j 2007; GQL 2024) y momento ¡ajá! del viaje de las 4
      estaciones.
