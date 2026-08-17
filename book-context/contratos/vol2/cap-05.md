# CONTRATO DE CAPÍTULO — Vol.II Cap. 5: Profundidad, ciclos y componentes (DFS, componentes conexos, ordenación topológica, SCC)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Capítulo **conceptual**
> de la Parte I (cierra la Parte I; el cap. 6 abre la Parte II). No implementa
> código: REORIENTA los algoritmos ya enseñados en el Vol.I (BFS cap. 4, DFS,
> componentes conexos, orden topológico y SCC) hacia su papel de **consultas
> estructurales** que una BBDD de grafos debe poder responder. Ancla al motor:
> estos conceptos regresan implementados sobre datos persistentes en la Parte V
> (caps. 22-26). Código real que anuncia: `componentes_conexas` de
> `liradb-workspace/crates/vol2-liradb/src/cap25_comunidades.rs` (líneas 696-731).
> Pregunta crítica de `CORPUS.yml` (id `vol-II-cap-05`): «¿Cuándo Kosaraju vs
> Tarjan en código real?». ToC línea 23: «Profundidad, ciclos y componentes
> (DFS, componentes conexos, ordenación topológica, SCC)». Parta I: cap. 5 es el
> último; gancho explícito al cap. 6 («qué convierte un grafo en una base de
> datos»).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: qué es un grafo dirigido y no dirigido (Vol. I,
  cap. 1); representaciones por listas de adyacencia y matrices (Vol. I, cap.
  2); BFS y por qué da caminos más cortos en grafos no ponderados (Vol. I, cap.
  4 y este Vol. II cap. 2); recorridos por visitado; que una BD persiste grafos
  (Vol. II cap. 10 apunta a persistencia; el prólogo promete construir LiraDB).
- **Ha VISTO en el Vol.I estos algoritmos pero como "problemas de juguete"**:
  DFS, componentes conexas, orden topológico y SCC se explicaron con grafos
  pequeños aislados. La misconcepción central es que son fines en sí mismos.
- **Cree saber pero es vago/erróneo (misconcepciones a corregir)**: (1) «DFS y
  BFS son intercambiables» — son familias distintas con garantías distintas: BFS
  minimiza pasos, DFS estructura el recorrido en un árbol de profundidad y
  detecta ciclos con la arista de retroceso; (2) «un ciclo se detecta
  devolviendo al nodo inicial» — para el DFS el detector es la **arista hacia
  un nodo gris** (ya en pila), no llegar al origen; (3) «componente conexa y
  SCC son lo mismo» — una es indirecta (no importa la dirección), la otra
  SIGUE las flechas; la mismísima función `componentes_conexas` del cap. 25
  usa la vista simétrica precisamente porque es OTRA pregunta; (4) «la orden
  topológica es unida hasta que aparece un ciclo» — sin DAG no hay orden, y
  detectar el ciclo ES parte de responder la consulta; (5) «estos algoritmos
  no tienen cabida en una base de datos» — son los **índices estructurales**
  que un GDBMS calcula para "¿de dónde a dónde?", "¿qué es cíclico?", "¿qué
  orden respeta las dependencias?".
- **NO debe saber todavía**: cómo se persiste un CSR en páginas (cap. 14 se lo
  dejará claro técnicamente, pero aquí se nombra la adyacencia en memoria);
  buffer pool, índices hash/B+ (cap. 13/15); el modelo Volcano (cap. 20);
  algoritmos *sobre el grafo persistente* de la Parte V (cap. 22-26). Aquí los
  algoritmos se RE-EXPLICAN conceptualmente y se deja claro que su
  implementación persistente llega en la Parte V. Se nombran y se cortan.

## 2. Conceptos (del grafo curricular)

- `present` (se re-introducen, ya vistos en Vol.I, aquí reencuadrados como
  **consultas estructurales del motor**): DFS y el backtracking de
  profundidad (colores blanco/gris/negro, árbol de profundidad, aristas de
  árbol/retroceso/adelante/cruzada); detección de ciclos por arista hacia un
  gris; **componentes conexas** (indirecta, vista NO dirigida o simétrica,
  BFS/DFS por componente — el ancla de `componentes_conexas` del cap. 25);
  **ordenación topológica** de un DAG (Kahn 1962, el grado de entrada a 0;
  el orden de tareas y dependencias); **SCC** — componentes fuertemente
  conexas (dirigido: de cada vértice llegas a cada otro siguiendo las flechas;
  colapsar un SCC en un supernodo produce SIEMPRE un DAG — el grafo de
  componentes).
- `practice`: BFS con cola (Vol. I cap. 4, cap. 24; Vol. II cap. 2); la vista
  simétrica (que el cap. 25 formaliza en `GrafoPonderado::proyectar`); ordenar
  por grado de entrada.
- `consolidate`: «recorrer sin repetir» (visitado); la representación por
  adyacencia; que una BD reproduce sus consultas (determinismo). Este capítulo
  consolida la idea de que el grafo es un MODELO y los algoritmos son
  PREGUNTAS sobre el modelo.
- `out_of_scope` (solo nombrar): implementación persistente de estos
  algoritmos sobre CSR/páginas (Parte V: Dijkstra/Pagerank/componentes/Louvain
  en caps. 22-25, proyección en memoria limitada en cap. 26); Tarjan/Kosaraju
  detallados paso a paso en código solo se MENCIONAN como historia
  (Kosaraju 1978, Tarjan 1972); se deja el "cuándo usar cuál" como pregunta
  crítica respondida en las porqués pero profundizada en la Parte V.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge** (afirmaciones comprobables): (1) explica las TRES componentes
  del capítulo como tres preguntas DISTINTAS sobre el grafo — conexidad
  indirecta / presencia de ciclo / ordenamiento de dependencias / fuerte
  conexidad dirigida — y las distingue sin confundirlas (glosario §8);
  (2) describe cómo DFS detecta un ciclo: una arista que llega a un vértice
  GRIS (ya en la pila de recursión) es una arista de retroceso y revela un
  ciclo; (3) dice que la ordenación topológica solo existe en un DAG y que un
  orden calculable es la definición operativa de "acíclico"; (4) explica que
  contraer cada SCC a un supernodo reduce CUALQUIER grafo dirigido a un DAG
  (de ahí que SCC y topológica se apoyen mutuamente); (5) cita que Kosaraju
  (1978, publicación 1981) corre dos pasadas y Tarjan (1972) una sola con un
  índice lowlink — y cuándo se prefiere cada uno en código real.
- **Skills** (qué HACE): (1) ejecuta un DFS a mano sobre un grafo pequeño,
  colorea y detecta el ciclo por la arista de retroceso; (2) numera las
  componentes conexas de un grafo no dirigido (como hace `componentes_conexas`);
  (3) aplica Kahn (cola de grado de entrada 0) para ordenar un DAG y detecta el
  ciclo como "quedaron vértices sin procesar".
- **Wisdom** (qué DECIDE): (1) decide cuándo basta CONECTIVIDAD indirecta
  (O(V+E) barata, por ejemplo "¿este dispositivo llega a aquél por cualquier
  cable?") y cuándo exige SCC dirigida ("¿puede este módulo depender de aquél
  sin que se ciclen?"); (2) decide que estas consultas son baratas por
  adelantado y por tanto computables como **índice derivado** (se calculan,
  se cachean, se invalidan al mutar) — el puente conceptual con un GDBMS.

## 4. Modelo mental

- **La misma figura del Vol.I reencuadrada**: UN grafo, CUATRO preguntas. El
  modelo mental único es **"el grafo como mapa con varias formas de
  preguntarle"**: con abrir los ojos (a igual paisaje) respondes tres/4
  consultas distintas. La figura la da un solo grafo dibujado cuatro veces:
  (a) con un árbol de profundidad (flechas negras) + la arista de retroceso
  roja que cierra un ciclo; (b) con las 3 componentes conexas coloreadas
  (trozos donde todo se alcanza); (c) el grafo acíclico de tareas con su
  orden topológico numerado; (d) el mismo grafo dirigido con los SCC
  colapsados a supernodos → queda un DAG.
- **Diagrama(s) ASCII**: los cuatro dibujos del mismo "esqueleto" de grafo —
  9 nodos, que se ven 4 veces con cada pregunta encima.
- **Momento ¡ajá!**: «DFS, componentes, topológica y SCC NO son cuatro
  algoritmos sueltos del Vol.I: son las respuestas del motor a "¿de dónde a
  dónde?", "¿qué trozos están conectados?", "¿qué orden respeta las
  dependencias?", "¿qué queda cuando colapso cada ciclo en una burbuja?". Una
  base de datos de grafos los calcula igual que una relacional calcula un
  índice: porque responden preguntas que el usuario hace, y se derivan del
  grafo sin que el usuario tenga que pedirlos a mano.»

## 5. Los porqués (grill — la pregunta más importante de cada decisión)

| # | Decisión | ¿Por qué así y no otra forma? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Separar 4 conceptos en vez de dar "recorridos" | Son 4 preguntas distintas (conexidad, ciclo, orden, fuerte conexidad). Mezclarlas es la misconcepción nº1; separarlas es lo que un novato necesita para NO confundir SCC con componente conexa | Un solo bloque "recorridos de grafos": el lector trataría SCC y componentes como sinónimos | Confunde "¿está todo conectado?" con "¿puedo ir de A a B siguiendo flechas?" en su primera consulta real | Este contrato §2; ancla `componentes_conexas` (vista SIMÉTRICA, sería un bug usar la dirigida) |
| 2 | DFS con 3 COLOREADOS (blanco/gris/negro) y la arista de retroceso | Un recorrido sin colores no distingue "visitado y cerrado" de "en la pila". El ciclo se detecta por llegar a un GRIS, no por volver al origen | Marcar solo visitado = DFS de árbol: nunca detecta ciclos (no ve la arista de retroceso) | Un ciclo pasa desapercibido y la consulta "¿hay ciclos?" responde mal | Cormen et al., CLRS cap. 22; arista de retroceso p. 605 (3ª ed.) |
| 3 | `componentes_conexas` usa la vista NO DIRIGIDA (simétrica) | La pregunta "componente conexa" es de alcanzabilidad SIN dirección: un USB 0→1 conecta 0 y 1 en ambos sentidos. El cap. 25 lo implementa leyendo cada arista en ambos sentidos (documentado en el doc-comment) | Aplicar SCC dirigida a esa pregunta: dos nodos unidos por una sola flecha NO serían "misma componente" aunque físicamente estén juntos | Redes que parecen desconectadas aunque lo estén | `componentes_conexas` (cap25_comunidades.rs, líns. 667-731); doc "cada arista dirigida se lee en ambos sentidos" |
| 4 | Ordenación topológica por Kahn (cola de grado de entrada) | Kahn expone el orden como "siempre hay algo sin dependencias pendientes; hazlo y retíralo". Es lo más transparente para un novato Y lo que hace `make`/dpkg ante un ciclo: se atasca y lo DENUNCIA | Ordenar "probando todos los órdenes" (O(n!)): inabordable; topológica por DFS inverso: igual de válida pero menos intuitiva para la analogía de tareas | Un ciclo de dependencias produce un build que nunca termina o un instalador que rompe paquetes | Kahn, "Topological sorting of large networks", CACM 5(11):558-562 (1962) |
| 5 | SCC colapsado SIEMPRE da un DAG | Si dentro de un SCC todo se alcanza y ese SCC se vuelve un punto, entre SCCs las aristas no pueden cerrar ciclos — si cerraran, serían el mismo SCC. Así, "SCC + topológica" sobre el grafo de componentes es la receta universal para ordenar dependencias con ciclos | Quedar satisfecho con detectar el ciclo y tirarlo | El usuario de un gestor de paquetes necesita saber el ORDEN de los ciclos, no solo que existen | Teorema estándar: el grafo de condensación de SCCs es un DAG (CLRS cap. 22.5) |
| 6 | Responder la pregunta crítica «Kosaraju vs Tarjan» | Kosaraju (1978, Sharp 1981): dos pasadas (DFS + DFS sobre transpuesto) — simple de entender y demostrar, el doble de aristas leídas. Tarjan (1972/1976): una pasada con `lowlink`, más rápido en práctica y sin transpuesta. En código real se elige por simplicidad (Kosaraju, errores de borrador menores) vs velocidad/una sola pasada en grafos enormes (Tarjan) | Presentar solo uno: el lector no sabrá decidir al enfrentarse a un grafo grande | Elegir Tarjan para una explicación didáctica (innecesariamente opaco) o Kosaraju para un grafo ingente (doble I/O) | Kosaraju 1978 (S. R. Kosaraju, "Dec. 8 1978", publicado por Aho-Hopcroft-Ullman); Tarjan, SIAM J. Computing 1(2):146-160 (1972) |

## 6. Primera solución vs solución evolucionada

- **Ingenua (novato)**: «DFS es un recorrido más, igual que BFS pero con pila» y
  «ciclo = volver al mismo nodo». Detectar un ciclo buscando el nodo de partida.
  Marcar solo "visitado" (blanco/negro, sin gris).
- **Qué la rompe exactamente**: un grafo pequeño con un ciclo que NO pasa por
  el nodo raíz (p.ej. `1→2→1` cuando arrancas en 0): el novato no "vuelve al
  origen" y cree que no hay ciclo; o intenta ordenar topológicamente un grafo
  con ciclo y produce un orden falso o un lazo infinito.
- **Evolución visible (conceptual)**: (1) DFS gana el TERCER color (gris) y la
  arista de retroceso → detecta ciclos con rigor; (2) componentes = repetir
  BFS/DFS sobre nodos sin visitar contando cada arranque como una componente
  (la numeración por menor miembro que formaliza el cap. 25); (3) topológica
  = Kahn con la cola de grado 0, donde los vértices que quedan al final son el
  ciclo; (4) SCC = Kosaraju o Tarjan reduciendo cualquier grafo a un DAG.

## 7. Prueba de fuego

- **Escenario del workspace (ancla real)**: la función `componentes_conexas`
  del cap. 25 responde "¿de dónde a dónde es conectado?" sobre vistas
  simétricas — el doctest que une los pares (0,1) y (2,3) y espera 2
  componentes y `componente(0)==componente(1)`, `componente(0)!=componente(2)`
  (líns. 679-731). La CORRECCIÓN de esa idea aquí es: cuando leas ese test en
  el cap. 25, reconocerás el concepto que este cap. anunció. Aquí (Parte I) la
  "prueba de fuego" es a MANO: el lector ejecuta un DFS sobre el diagrama y
  pinta las 3 componentes / el orden topológico, y lo verifica contra la
  figura. En la Parte V ese test del workspace es la prueba automatizada.
- **Si el lector se salta este capítulo (síntoma detectable)**: al llegar al
  cap. 25 no entiende por qué se proyecta la vista SIMÉTRICA para componentes
  y por qué SCC es "otro algoritmo y otra pregunta" (el propio doc-comment lo
  dice); confunde el número de componentes con el número de comunidades; y en
  la Parte IV no sabe interpretar un plan `MATCH` que explota conectividad
  estructural.

## 8. Trampas y errores comunes

1. **SCC ≠ componente conexa**: componente conexa ignora la dirección (vista
   simétrica); SCC la respeta y exige "ida y vuelta siguiendo las flechas".
   Detector: dos nodos unidos por SOLO `0→1` están en la misma componente
   conexa pero en distinto SCC.
2. **Detectar ciclos "volviendo al origen"**: con el DFS en profundidad
   correcto sólo es un ciclo la arista que apunta a un vértice GRIS (en pila).
   Una arista hacia un negro ya cerrado NO es un ciclo.
3. **Aplicar ordenación topológica a un grafo con ciclos**: Kahn se atasca y
   quedan vértices sin procesar; "el orden que sobre" no es topológico.
   Detector: `orden.len() < V` ⇒ hay ciclo.
4. **Creer que estos son "recorridos del Vol.I"**: son la implementación de
   consultas estructurales del motor — la Parte V los vuelve a usar sobre
   datos persistidos.
- **Precisión de lenguaje (glosario)**: *componente conexa* (indirecta) vs
  *SCC* (dirigida); *arista de árbol/retroceso/adelante/cruzada* (según el
  color del otro extremo); *DAG* (grafo dirigido acíclico) vs *condensación*
  (el DAG de los SCC); *orden topológico* (un orden donde cada arista va de
  antes a después) vs *orden de DFS* (recorrido de descubrimiento); *blanco /
  gris / negro* (sin visitar / en pila / cerrado).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial) — retrieval**: el lector ejecuta A MANO un DFS
  sobre el grafo de la figura (9 nodos), anota el orden de descubrimiento y
  finalización de cada nodo, y marca la arista de retroceso que revela el
  ciclo; luego dibuja las 3 componentes conexas. Verificación: comparar con el
  diagrama resuelto de §4 del capítulo. Pistas: (1) ¿qué color tiene un nodo
  cuando vuelve a aparecerle una arista?, (2) ¿cuántos arranques de
  BFS/DFS necesitas para numular las componentes?, (3) el ciclo tiene que
  cerrarse con una flecha hacia arriba en el árbol. Criterio: la arista de
  retroceso marcada es la única hacia un gris, y 3 componentes numeradas.
- **analizar (intermedio, spacing/interleaving con topológica + mundo real)**:
  dado un mini-DAG de "montar una base de datos" (asignar página → escribir
  registro; reservar buffer → cargar página; crear catálogo → escribir
  índice; compilar → enlazar → ejecutar), el lector aplica Kahn y escribe SI
  existe el orden; luego se añade una dependencia que crea el ciclo
  `A→B→C→A` y explica por qué ya no hay orden, y qué haría `make`/`dpkg`. 
  Pistas: (1) Kahn arranca de grado de entrada 0, (2) un ciclo deja ≥1
  vértice sin procesar ("quedó cola vacía antes de tiempo"), (3) el gestor lo
  denuncia como dependencia circular en vez de ordenar. Criterio: orden
  topológico válido (cada flecha antes→después) y diagnóstico del ciclo.
- **crear (experto, gancho Parte V)**: el lector reduce el grafo DIRIGIDO de 9
  nodos a sus SCC (colapsando cada fuerte conexidad en un supernodo) y
  comprueba que el resultado es un DAG; luego ordena ese DAG de componentes
  topológicamente. Pistas: (1) un SCC es un ciclo o un nodo aislado, (2) al
  colapsar, las flechas entre SCCs no pueden ciclar, (3) el orden de los
  SCCs es la respuesta a "¿en qué orden deshacer los ciclos?". Criterio: el
  grafo de condensación es acíclico y el orden respeta flechas. (Esta es la
  pregunta crítica CORPUS: aquí se resuelve a mano Kosaraju vs Tarjan.)

## 10. Preguntas abiertas (gancho al capítulo 6 — y a la Parte V)

1. Si estos son los "índices estructurales" de una BD de grafos, ¿cómo se
   PERSISTEN y se mantienen al insertar/borrar nodos y aristas? (cap. 6 corta a
   14-15 y la Parte V los recalcula sobre datos estables.)
2. ¿Cómo se expone esta conectividad en un LENGUAJE de consultas (un `MATCH`
   que pide "¿de dónde a dónde es alcanzable?")? (Parte IV, caps. 17-21.)
3. Cuando el grafo no cabe en memoria, ¿cómo se ejecuta un DFS/componentes
   sobre él? (Parte V, cap. 26: proyección, streaming, frontiers.)
- **Términos nuevos de glosario**: DFS, colores blanco/gris/negro, árbol de
  profundidad, arista de retroceso/adelante/cruzada, componente conexa (no
  dirigida), DAG, ordenación topológica (Kahn), SCC, grafo de condensación,
  supernodo, lowlink, Kosaraju, Tarjan, índice estructural derivado.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial obliga a RE-EXECUTAR DFS y RE-DETECTAR el
  ciclo desde la memoria (el enunciado no regala la arista de retroceso) y a
  re-numerar las 3 componentes sin mirar la solución.
- **Spacing**: BFS del Vol. I cap. 4 y cap. 24 / Vol. II cap. 2 (la cola, el
  visitado); vista simétrica que el cap. 25 formalizará (se ejercita al
  dibujar las componentes); determinismo de una BD (el cap. 24/25 lo exigen).
- **Interleaving**: el intermedio cruza ordenación topológica (Kahn) con el
  modelado de dependencias real (`make`/dpkg/npm) y con SCC (el experto mezcla
  DFS dirigido + colapso + topológica). No hay ejercicios clónicos.
- **Dificultad asimétrica**: una idea nueva por sección (DFS y el ciclo →
  componentes → topológica → SCC); los ejercicios exigen recuperación,
  predicción y construcción, no reconocimiento.
- **Bucle de feedback inmediato**: respuesta a mano verificable contra el
  diagrama de §4; en la Parte V ese mismo concepto tiene tests automáticos
  (`componentes_dos_pares_y_puente`, `componentes_vacio_aislados_y_dirigidos`
  del cap. 25). Como el capítulo es conceptual, el "test" aquí ES la figura
  resuelta + la tarea del intermedio (diagnóstico del ciclo).
- **Citas**: Cormen et al., *Introduction to Algorithms*, 3ª ed., cap. 22
  (DFS, aristas, SCC); Kahn, CACM 5(11):558-562 (1962); Tarjan, SIAM J.
  Computing 1(2):146-160 (1972); Kosaraju 1978 (publicado en Aho,
  Hopcroft, Ullman, *Data Structures and Algorithms*, 1983, §9.6); la
  historia del `make` de Feldman (1979) y el uso en dpkg/npm/cargo.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (6 en la tabla §5).
- [x] Escenario de fallo visible: el novato que no detecta un ciclo que no vuelve a su raíz; Kahn atascado con un ciclo; SCC confundida con componente (§6-§8).
- [x] Capítulo conceptual pero con ancla de código REAL citado por nombre (`componentes_conexas`, cap25 líns. 696-731), sin duplicarlo.
- [x] Misconcepciones corregidas explícitamente (BFS≠DFS; ciclo por arista a un gris; componente≠SCC; topológica solo en DAG; "no son recorridos del Vol.I, son consultas del motor").
- [x] ≥1 ejercicio de retrieval (DFS + ciclo + 3 componentes a mano) y ≥1 de interleaving (Kahn con dependencias de make/dpkg/npm).
- [x] Responde la pregunta crítica de CORPUS: «¿Cuándo Kosaraju vs Tarjan en código real?» — §5.6 y ejercicio experto.
- [x] Anécdota verificada: el problema real de los ciclos de dependencias en `make`/gestores de paquetes; la historia Kosaraju 1978 y Tarjan 1972.
- [x] Enfatiza los 4 conceptos por separado (a-d) para que un novato no los confunda, y los reencuadra como los "índices estructurales" que el motor debe poder responder.
