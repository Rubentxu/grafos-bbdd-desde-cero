# CONTRATO DE CAPÍTULO — Vol.II Cap. 1: Qué es realmente un grafo

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Capítulo de APERTURA
> del Vol.II y primera pieza de la **Parte I «Pensar en grafos»** (caps. 1-5).
> Libro conceptual: NO tiene módulo propio en `vol2-liradb` (el código empieza
> en el cap. 7). Re-ORIENTA la teoría del Vol.I (caps. 1-2) hacia «qué significa
> un grafo para una base de datos», sin re-explicarla. Responde las preguntas
> críticas del CORPUS (id `vol-II-cap-01`): «¿Qué relación tiene este capítulo
> con caps. 1-2 del Vol.I?» y «¿Qué añadidos aporta el Vol.II sobre la
> definición de grafo?». Cimiento conceptual de toda la obra.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: estaba leyendo el Vol.I y conoce la definición
  básica de grafo (nodos + aristas, dirigido o no), las tres representaciones en
  memoria (edge list, adjacency list, CSR) y los algoritmos de recorrido (BFS/DFS),
  con la notación `(Vol. I, cap. N)` clara. Tiene mínima sintaxis de Rust. Sabe que
  un *grafo de computación* es una estructura para **recorrer**.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «un grafo
  es *solo* un montón de nodos y flechas, y lo importante es el algoritmo» — no:
  para una BBDD lo decisivo es que cada nodo y arista pueden llevar **datos
  adjuntos, etiquetas e identidad estable**; (2) «un grafo y un árbol son lo
  mismo» — el árbol es un caso *particular* de grafo (conectado, acíclico, un solo
  camino entre pares); confundir el caso con la clase; (3) «un nodo ES el dato que
  guarda (su nombre, su valor)» — no: el nodo es una *posición* con identidad
  propia y datos *separados* de él; (4) «el grafo matemático y el grafo de base de
  datos son idénticos en todo» — no: el matemático es un esqueleto de relaciones;
  la BBDD lo *viste* con un esquema, tipos e identidad.
- **NO debe saber todavía**: el modelo `Property Graph` a fondo (cap. 7), los
  `Value`/`id`/`labels` tipados (cap. 7), las representaciones de memoria como
  listas de adyacencia/CSR (se NOMBRAN como «luego lo verás», cap. 2), el CSR
  persistente (cap. 14), RDF (Vol.III). Todo lo que no es este capítulo se cita
  con «avance» o «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: la re-definición de grafo para BBDD (topología **+** dato **+**
  etiqueta **+** identidad estable); el **Property Graph** a nivel conceptual
  (sin tipos aún: solo la idea de nodos/aristas con datos y etiquetas); el
  contraste grafo matemático (`G=(V,E)`) vs grafo de BBDD; el problema de
  Königsberg (Euler 1736) como origen.
- `practice`: distinguir la topología (qué hay enlazado) del dato (qué significa);
  reconocer un grafo en la vida real (red social, mapas, dependencias de paquetes);
  leer un dígrafo simple en ASCII.
- `consolidate`: la definición `G=(V,E)` (nodos + aristas, dirigido/no); las
  tres representaciones del Vol.I cap. 2 (edge list, adjacency list, CSR) que se
  re-orientarán a BBDD en el cap. 2 del Vol.II.
- `out_of_scope` (solo nombrar «luego lo verás»): matriz de adyacencia con bits
  (se usa como PRIMERA SOLUCIÓN ingenua, no como representación final); CSR
  persistente (cap. 14); RDF/RDF-star y el triple (Vol.III); el tipado `Value`
  (cap. 7); el trait `GraphStore` (cap. 8).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica qué es un grafo para una BBDD y por qué exige
  identidad estable + datos adjuntos + etiquetas — lo que el grafo matemático
  NO tiene; (2) distingue el grafo matemático (estructura pura `V×E`) del grafo de
  datos (esqueleto vestido de esquema); (3) cita el origen histórico (Euler,
  Königsberg, 1736) y lo conecta con los problemas de hoy; (4) explica por qué la
  matriz de adyacencia con bits basta para *analizar* un grafo de computación pero
  no para *persistir* un grafo de BBDD.
- **Skills**: (1) dibujar un grafo simple en ASCII y etiquetar nodos/aristas con
  datos conceptuales; (2) identificar en un problema real (red social, mapas,
  dependencias) qué es un nodo, qué es una arista y qué datos querrías adjuntar a
  cada uno; (3) ver el salto «de topología a modelo de datos».
- **Wisdom**: (1) decide que una representación que conserve *dato + etiqueta +
  identidad* vale más que una que solo encode la topología, aunque el bit de la
  matriz de adyacencia sea «más simple»; (2) decide que el grafo matemático es el
  esqueleto y la BBDD viste el esqueleto — no al revés.

## 4. Modelo mental

- **La figura/taxonomía**: el **esqueleto vs el vestido**. El grafo matemático es
  `G=(V,E)`: un esqueleto de relaciones (estructura pura). Para ser una base de
  datos, ese esqueleto necesita vestirse con tres ropas: (1) **datos adjuntos**
  (qué guarda cada nodo/arista), (2) **etiquetas** (qué *tipo* de cosa es cada
  nodo/arista), (3) **identidad estable** (cómo nombrar cada cosa sin que el nombre
  cambie cuando se reorganiza). La BBDD viste el esqueleto; sin el esqueleto, los
  vestidos no se sostienen.
- **Diagrama ASCII obligatorio (§1.3)**: un minigrafo social `Ada → Bo`, `Bo → Ana`
  dibujado como nodos circulares + aristas con flecha, y **debajo el mismo grafo** en
  dos versiones: (a) el matemático `G=(V,E)` (solo círculos y flechas sin nombres),
  (b) el de BBDD (cada círculo con un `id`, un `label` y una cajita de propiedades).
  El lector debe ver con los ojos la diferencia «esqueleto vs vestido».
- **El momento ¡ajá!**: «la matriz de bits del Vol.I me dice **que** hay un enlace;
  una BBDD de grafos me dice **qué** es cada extremo del enlace y **desde cuándo**
  existe. La topología es la punta del iceberg; el dato, la etiqueta y la identidad
  son el resto del iceberg, que es donde vive la base de datos».

## 5. Los porqués (grill — cada decisión del capítulo)

| # | Decisión | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Abrir el Vol.II con un capítulo conceptual y NO con código (el código va al cap. 7) | El lector llega del Vol.I con la teoría de recorrido; el giro de valor del Vol.II es *construir un motor*, y antes de construir hay que re-significar «grafo» para la BBDD: qué significa persistirlo, nombrarlo y vestirlo. Un cimiento mal puesto hace que caps. 2-6 (también conceptuales) y 7+ floten | Meter código ya en cap. 1 — coste: confundir «estructura de computación» con «modelo de datos» y correr antes de andar | El lector construye un grafo de bits (topología) y lo llama «base de datos», arrastrando el error toda la obra | CORPUS `vol-II-cap-01`; estilo Vol.II (Apéndice 0 §0.2); la obra es «Construye LiraDB», no «algoritmos otra vez» |
| 2 | Definir el grafo de BBDD como `G=(V,E)` + labels + props + identidad estable (Property Graph conceptual) | Responde la pregunta crítica «¿qué añadidos aporta el Vol.II sobre la definición de grafo?»: la definición matemática es necesaria pero insuficiente para una BBDD; el añadido es dato + etiqueta + identidad. Es el modelo `Property Graph` que cap. 7 tipa y que Neo4j/GQL popularizaron | Requedar solo con `G=(V,E)` y «añadir props después como detalle» — coste: la identidad y las etiquetas son decisiones de fondo, no detalles; dejarlas fuera rompe el cap. 7 | El modelado cap. 7 no «depende» de nada; los nodos se confunden con sus datos; borrar un nodo recicla su identidad | Robinson, Webber & Eifrem, *Graph Databases* 2ª ed., 2015 (Property Graph); ISO/IEC 39075:2024 (GQL); Vol.II cap. 7 |
| 3 | La **matriz de adyacencia con bits** como primera solución ingenua (no como representación final) | Muestra el contraste pedagógico del libro: la representación del Vol.I sirve para *analizar* topología, pero no para *persistir* datos/etiquetas/identidad. El lector la reconoce y entonces ve su límite | Omitir la matriz y empezar directamente con el Property Graph — coste: el novato no comprende *qué* se pierde; no hay tensión de «primera vs evolucionada» | El lector piensa que un grafo es «unas cuantas filas de true/false» y no entiende el cap. 2 ni el cap. 7 | Vol.I cap. 2 (matriz de adyacencia); Vol.II cap. 7 (la matriz de bits no basta para conceder propiedades) |
| 4 | Anécdota de apertura = Euler 1736 (Königsberg) | Es el origen verificable y verificablemente fundacional de la teoría de grafos; ancla la obra en una persona, una fecha y una ciudad reales y conecta «problema del mundo real» con «estructura de relaciones». Da al capítulo de apertura su peso histórico | Abrir con una anécdota anacrónica (p.ej. una red social moderna) — coste: pierde el azoramiento del «origen» y no enraíza la definición | El capítulo de apertura suena a «lista de definiciones» sin porqué; el término «grafo» no se explica | Euler, «Solutio problematis ad geometriam situs pertinentis» (Commentarii Academiae Scientiarum Petropolitanae, 1736); Biggs, Lloyd & Wilson, *Graph Theory 1736–1936* |
| 5 | Incluir el contraste «grafo matemático`G=(V,E)` vs grafo de datos» como hilo conductor | Es la tesis IS: «el grafo matemático es el esqueleto; la BBDD viste el esqueleto». Ordena TODO el Vol.II: esquema (datos), etiquetas, identidad (cap. 3), propiedad (cap. 7), persistencia (cap. 11+) | Presentar ambas cosas como la misma — coste: el lector no ve *qué* añade la BBDD ni por qué los caps. 2-6 existen | El Vol.II entero se lee como «Vol.I repetido»; la base de datos no se justifica | CORPUS `vol-II-cap-01` Q2; Apéndice 0 §0.4 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: el novato representa el grafo como una **matriz de adyacencia de
  bits** (Vol.I cap. 2): una tabla `N×N` de `true/false` que dice «hay enlace».
  Rápida de pintar, ideal para *analizar* topología (¿conectado? ¿cuántos vecinos?).
  "El grafo es la tabla de true/false."
- **Qué la rompe**: intenta guardar a Ana como «el nodo 0» y la flecha como «true»,
  y de repente no tienes dónde poner el *nombre* de Ana, ni su *edad*, ni la flecha
  **desde cuándo** se conocen, ni el tipo `KNOWS` vs `WORKS_AT`. Y si borras el nodo
  0, «el nodo 0» pasa a significar otra cosa: la **identidad** se recicla. La matriz
  responde a *estructura*, no a *dato*.
- **Evolución visible**: el **Property Graph** conceptual: `id` (identidad estable,
  no índice), `labels` (clasificación) y `props` (datos adjuntos) en nodos y
  aristas; las aristas con su propio `source/target/label`. El lector ve el mismo
  grafo en dos trajes: el esqueleto y el vestido. (Sin código aún — se tipa en el
  cap. 7; aquí solo se conceptúa.)

## 7. Prueba de fuego

- **Sin código ejecutable** (capítulo conceptual; `vol2-liradb` empieza en cap. 7).
  La «prueba de fuego» es **conceptual y verificable con lápiz y papel**: el lector
  debe ser capaz de (a) tomar un problema real (p.ej. una red social minúscula),
  (b) dibujar el grafo en ASCII, (c) etiquetar cada nodo con un `id`, un `label` y
  2-3 `props`, y cada arista con un `label` — y (d) **explicar por qué la matriz de
  bits no bastaba** para ese ejemplo. Los ejercicios esencial/intermedio/experto
  van verificados contra esta plantilla (rúbrica en §9).
- **Síntoma si el lector se salta el capítulo**: ignora el giro del Vol.II y trata
  los caps. 2-6 como «algoritmos repetidos del Vol.I»; llega al cap. 7 pensando que
  un grafo es la matriz de bits y no entiende por qué hay que inventar `Value`,
  `labels` e `id`. El «síntoma detectable»: confundir un nodo con su dato y un
  grafo con un árbol.

## 8. Trampas y errores comunes

1. **«Todo grafo es un árbol»** (y su inversa: «un árbol es como un grafo
   cualquiera»). El árbol es un grafo especial (conectado + acíclico + un solo
   camino entre pares de nodos). Confundir el caso con la clase hace que el lector
   no entienda ciclos ni componentes (cap. 5) ni `G=(V,E)` general.
2. **Confundir el nodo con el dato.** El nodo es una *posición* (una identidad);
   el dato (nombre, edad) está *dentro* de él pero no *es* él. El síntoma:
   «borrar el nodo 0 es borrar a Ana», cuando Ana es el dato del nodo 0 y la
   identidad del nodo no debería reciclarse (cap. 3 / cap. 7).
3. **«Un grafo es solo la matriz de true/false».** La matriz responde a
   *estructura* (¿hay enlace?); una BBDD de grafos responde además a *qué* es cada
   extremo y *desde cuándo* existe el enlace. Confundir análisis con persistencia.
- **Precisión de lenguaje (glosario)**: *grafo matemático* (estructura pura
  `V×E`) vs *grafo de datos / Property Graph* (esqueleto + dato + etiqueta +
  identidad); *nodo/vértice* vs *dato* (el dato vive dentro, no es el nodo);
  *grafo* vs *árbol* (el árbol es un grafo especial); *identidad estable* (nombre
  que no cambia al reorganizar) vs *índice* (posición que sí cambia); *label*
  (etiqueta/clasificación) vs *propiedad* (dato opaco).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval)**: SIN mirar la sección, dibuja un
  minigrafo de 3 nodos con 2 aristas y completa para cada nodo un `id`, un `label`
  y 2 `props`, y para cada arista su `label`. Luego compara con el ejemplo del
  §1.6. Pistas: (1) el `id` es el nombre que no se recicla; (2) el `label` dice
  *qué tipo* de cosa es; (3) la `prop` es un dato adjunto. Criterio: el grafo
  dibujado tiene identidad, etiquetas y datos separados — no solo flechas.
- **analizar (intermedio — spacing Vol.I caps. 1-2 / interleaving vida real)**:
  «¿qué es el grafo en una red de dependencias de paquetes?» Identifica nodos,
  aristas y las propiedades que querrías adjuntar. Relaciónalo con la matriz de
  adyacencia del Vol.I cap. 2 y explica qué se perdería si solo tuvieras `true/false`.
  Pistas: (1) ¿qué es un paquete y qué una dependencia?; (2) ¿qué guardarías en la
  arista «A depende de B»?; (3) ¿la matriz de bits serviría para saber la *versión*
  en la que se conoce?
- **crear (experto — interleaving cap. 2 y cap. 7)**: plantea un problema real
  (red social, mapa, dependencias) y «traduce» su modelo a las tres representaciones
  del cap. 2 (edge list, adjacency list, CSR) a nivel conceptual, señalando cuál
  retendría mejor el dato + label + identidad. Pistas: (1) la edge list es `(u,v)`;
  ¿dónde pondrías el label?; (2) la adjacency list agrupa por nodo; ¿dónde la prop?;
  (3) CSR comprime `V` por `E`; ¿qué sacrifica en dato?

## 10. Preguntas abiertas (gancho al cap. 2)

1. Ya sabemos *qué* es un grafo y qué aporta la BBDD (dato, etiqueta, identidad)…
   ¿cómo lo guardamos **en memoria** para poder recorrerlo rápido? Las tres
   representaciones del Vol.I (edge list, adjacency list, CSR) — ¿cuándo usar cada
   una *en una BBDD*? (Representaciones de memoria: cap. 2.)
2. Dijimos que la identidad no debe reciclarse… ¿por qué borrar un nodo del medio
   rompe «el índice 2»? ¿Qué estructura garantiza identidades estables incluso tras
   crash? (Identidad y `slotmap`: cap. 3.)
3. «La BBDD viste el esqueleto»… ¿y si quieres interoperar con el mundo semántico
   (web de datos)? Ahí el modelo cambia (triples RDF). (Vol.III, modelado RDF/OWL.)
- **Términos nuevos de glosario**: grafo matemático (`G=(V,E)`), Property Graph
  (conceptual), nodo/vértice, arista/edge (fuente→destino), label, propiedad,
  identidad estable vs índice, grafo vs árbol, matriz de adyacencia (repaso Vol.I),
  problema de Königsberg (1736).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial «recordar/aplicar» exige re-dibujar de
  memoria un minigrafo de **3 nodos con 2 aristas**, con `id` + `label` +
  `props` en cada nodo y `label` en cada arista, y luego cotejar contra el
  ejemplo canónico de §1.6. **Recordar > reconocer**: el lector construye la
  respuesta en blanco y la verifica. Si la recupera bien, ancla la idea
  «Property Graph = esqueleto vestido con dato / etiqueta / identidad»; si
  la olvida, al menos habrá generado la tensión de la búsqueda que la hace
  memorable en el siguiente repaso (spacing natural).
- **Spacing**: el intermedio re-ejercita la **definición de grafo** del
  Vol.I cap. 1 (nodos + aristas, dirigido o no, `G=(V,E)`) y la **matriz de
  adyacencia del Vol.I cap. 2** (que este capítulo *re-orienta* hacia BBDD,
  no repite — esa es la diferencia con ser "otra vez la matriz"). El experto
  ancla las **tres representaciones** (*edge list*, *adjacency list*, *CSR*)
  que el cap. 2 del Vol.II desarrollará — sembrando la semilla del volumen
  sin duplicar el contenido.
- **Interleaving**: el intermedio mezcla el **grafo matemático** con un
  dominio de la **vida real** (red de dependencias de paquetes: nodos =
  *crates*, aristas = «depende de», con la *versión* como propiedad de la
  arista) y justifica por qué la matriz de bits se queda corta para guardar
  el **rango de versiones permitido**. El experto mezcla el problema real
  con las **tres representaciones del cap. 2** (*edge list* vs *adjacency
  list* vs *CSR*), forzando al lector a traducir un mismo dominio entre
  distintos lentes — *representaciones × modelado* en lugar de N ejercicios
  clónicos del Property Graph.
- **Regla de dificultad asimétrica**: en el **conocimiento** (la
  explicación) una sola idea nueva por sección — qué es un grafo, por qué
  una BBDD lo necesita, esqueleto vs vestido, matriz vs Property Graph,
  ángulo de BBDD (qué convierte esto en motor) — todas dentro de la
  memoria de trabajo del novato. En los **ejercicios** la dificultad es la
  herramienta: el esencial es dibujar de memoria; el intermedio, razonar
  contra un dominio real; el experto, traducir entre representaciones.
- **Bucle de feedback inmediato**: capítulo **conceptual, sin `cargo test`**
  (el código empieza en el cap. 7). El feedback es la **rúbrica
  explícita** de cada ejercicio en §9 (criterios verificables contra el
  ejemplo resuelto de §1.6 y contra las representaciones del cap. 2), para
  que el lector se auto-verifique al instante sin tener que «confiar en mí».
- **Citas (alta confianza, no paramétricas)**:
  - **Origen histórico**: Leonhard Euler, «*Solutio problematis ad
    geometriam situs pertinentis*», *Commentarii Academiae Scientiarum
    Petropolitanae* 8, 1736 (el acta de nacimiento de la teoría de grafos,
    recopilado en N. Biggs, E. Lloyd y R. Wilson, *Graph Theory 1736–1936*,
    OUP).
  - **Definición moderna de grafo `G=(V,E)`**: J. Gross y J. Yellen, *Graph
    Theory and Its Applications*, 2ª ed., CRC Press (referencia de manual).
  - **Property Graph como modelo de datos**: I. Robinson, J. Webber y E.
    Eifrem, *Graph Databases*, 2ª ed., O'Reilly/Neo4j, 2015 (la definición
    canónica y didáctica de nodos etiquetados + aristas de primera clase).
  - **Estandarización 2024**: ISO/IEC 39075:2024, *Graph Query Language
    (GQL)*, el estándar que formaliza el property graph como modelo de
    datos de primera clase.
  - **Contraste con el modelo ER** (opcional, para «Para profundizar»):
    P. Chen, «*The Entity-Relationship Model — Toward a Unified View of
    Data*», *ACM TODS* 1(1), 1976 (entidades = nodos, relaciones = aristas:
    el primo relacional del property graph).

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con **alternativa descartada,
      modo de fallo y fuente** (5 filas en la tabla §5).
- [x] **Escenario de fallo visible**: la matriz de adyacencia con bits no
      puede guardar el nombre de Ana (no hay sitio para *strings* en una
      celda booleana), ni su edad (un bit no codifica enteros), ni «desde
      cuándo» (la arista es un bit, no una entidad con propiedades);
      **borrar el nodo 0 recicla su identidad** — todos los arcos que
      apuntaban al índice 0 pasan a apuntar al nuevo ocupante del hueco 0
      (la raíz del problema que el cap. 3 resolverá con `slotmap`).
- [x] Capítulo **conceptual** (sin código ejecutable): la prosa *referencia*
      el código futuro (cap. 7: `Value` tipado, `Node`/`Edge`, `PropertyGraph`)
      sin duplicarlo; la «prueba de fuego» es **lápiz-y-papel con rúbrica**
      (§7), no un test de cargo. Las secciones §1.4–§1.6 muestran la
      **evolución conceptual** (matriz de bits → matriz + props añadidas →
      Property Graph con identidad estable) sin saltarse al `cargo test`.
- [x] **Misconcepciones corregidas explícitamente** (§1: cuatro — «grafo =
      solo flechas y algoritmo», «grafo = árbol», «nodo = dato», «grafo
      matemático = grafo de BBDD»; §1.9 las repite con **glosario y forma
      de detectarlas**).
- [x] Ejercicios con **solución verificable**: rúbrica por ejercicio en §9
      (recordar: 3 nodos/2 aristas contra §1.6; analizar: dependencias de
      paquetes contra matriz de bits; crear: *edge list* vs *adjacency list*
      vs *CSR*), verificables contra §1.6 y contra el cap. 2 del Vol.II.
- [x] **≥1 ejercicio de retrieval** (esencial — dibujar el Property Graph
      de memoria y cotejarlo con §1.6) **y ≥1 toque a concepto de capítulo
      anterior** (intermedio: re-usa la matriz de adyacencia del Vol.I
      cap. 2 + dependencia real — spacing + interleaving en uno; experto:
      ancla las tres representaciones del cap. 2 del Vol.II como cross-plot
      con la propiedad / etiqueta / identidad).
- [x] **Responde las preguntas críticas del CORPUS `vol-II-cap-01`**:
      (1) «¿qué relación tiene este capítulo con caps. 1-2 del Vol.I?» —
      los **re-orienta** (no los repite); (2) «¿qué añadidos aporta el
      Vol.II sobre la definición de grafo?» — **dato + etiqueta + identidad
      estable** (Property Graph conceptual = esqueleto vestido).
- [x] **Anécdota verificada con fuente**: L. Euler (1736), Königsberg
      (ciudad prusiana, hoy Kaliningrado, Rusia), paper *«Solutio
      problematis ad geometriam situs pertinentis»*, *Commentarii Academiae
      Scientiarum Petropolitanae* 8 (Biggs/Lloyd/Wilson como recopilatorio
      moderno). Se contrasta **grafo matemático `G=(V,E)`** (esqueleto)
      vs **Property Graph** (esqueleto vestido) con la cita de
      Robinson/Webber/Eifrem y la estandarización ISO/IEC 39075:2024.
- [x] **Tesis explícita y repetida**: «un grafo es una estructura de
      relaciones que, para ser útil como base de datos, exige identidad
      estable + datos adjuntos + etiquetas. El grafo matemático es el
      esqueleto; la BBDD viste el esqueleto» (aparece en §1.6 como tesis
      central y se re-condensa en el **Pin de batalla** §1.12 y en **«Si
      solo lees 30 segundos»** §1.13, de modo que el lector la recuerde
      incluso si hojea el capítulo).
- [x] **Mini-diálogo de cierre** (§1.15) y **gancho explícito al cap. 2**
      (representaciones de memoria: *edge list* / *adjacency list* / *CSR*;
      «aquí el grafo era idea y estructura, ahora veremos cómo guardarlo
      en memoria y por qué las tres elecciones se parecen tanto y se
      diferencian tanto»).
