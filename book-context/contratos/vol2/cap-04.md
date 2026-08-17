# CONTRATO DE CAPÍTULO — Vol.II Cap. 04: El primer recorrido: búsqueda en anchura (BFS)

> Primer capítulo "de algoritmos" de la Parte I, **reorientado al motor de base
> de datos**: no re-explica la teoría de BFS (eso es el **Vol. I**), sino que la
> presenta como **la operación fundamental que un GDBMS debe soportar** y
> anuncia el trait `GraphStore` (cap. 8) como puerto futuro. Es **un capítulo
> INTENCIONADAMENTE conceptual** (sin código ejecutable propio): lo que SÍ
> existe y se ancla es el BFS por niveles de capítulos posteriores —
> `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs`
> (`closeness_centrality`, Freeman + corrección Wasserman-Faust) y
> `cap26_proyeccion.rs` (`bfs_fronteras`, `bfs_streaming`, `FronterasBfs`,
> `RecorridoBfs { niveles: Vec<Vec<NodeId>> }`, `Presupuesto`, `MotivoParada`,
> `StreamStats`). La tesis: **lo que aquí es una idea, la fábrica del cap. 26
> lo ejecuta por streaming**.

---

## 1. El novato (perfil y punto de partida)

### ¿Qué sabe YA sin ninguna duda?
- Qué es un grafo (nodos/aristas, dirigido/no dirigido, "existe un camino");
  caps. 1-2 del **Vol. I** son prerequisito explícito (el cap. 1 del Vol. II
  ya citó esa deuda).
- El modelado Property Graph + `Value` (cap. 7 del Vol. II): sabe que una
  arista tiene dirección y un tipo, y que la "adyacencia" es una relación del
  grafo.
- La **identidad estable** de nodos/aristas (cap. 3): los nodos se referencian
  por id, no por posición — sostiene que un recorrido se ancla en "de quién
  soy vecino".
- De forma vaga, la noción de *distancia* / *menor número de saltos* (lenguaje
  cotidiano de las redes sociales); aquí se vuelve rigurosa.

### ¿Qué cree saber pero es vago/erróneo (misconcepciones a corregir)?
1. **«Todo recorrido de un grafo es DFS»** — creen que basta con "irse por un
   vecino y volver". No: hay DOS formas canónicas, y la que produce el *camino
   más corto* es la de anchura; la de profundidad no lo garantiza.
2. **«Las bases de datos no recorren grafos, me devuelven filas»** — creencia
   correcta sobre SQL, falsa para un GDBMS. "¿existe un camino entre A y B?"
   ES una consulta; BFS es cómo la responde el motor.
3. **«Para saber si hay camino tengo que mirar todos los pares de nodos»** —
   la peor: un ingenuo compara todos contra todos (O(V²)). BFS prueba que
   una consulta local de alcanzabilidad cuesta O(V+E) desde UN origen.
4. **«BFS y "menor número de saltos" son lo mismo que ponderado»** — confunde
   el BFS (no pondera: cuenta saltos) con Dijkstra (pondera aristas, cap. 22).
   BFS es el *caso con pesos 1*; el camino ponderado de un viajero es Dijkstra
   (1959).
5. **«Si ya tengo el grafo en memoria, recorrer es gratis»** — no: lo que
   cuesta son las **lecturas de adyacencia** al store. Ese coste (visible en
   `StreamStats.aristas_leidas`, cap. 26) es el corazón del motor.

### ¿Qué NO debe saber todavía (conceptos futuros)?
- La implementación real sobre `GraphStore` (`out_edges`, CSR, cap. 14): se
  anuncia como "el puerto del cap. 8" sin explicar.
- **Pesos de arista** y Dijkstra/Bellman-Ford: cap. 22. Aquí Dijkstra solo se
  nombra como "BFS con pesos 1 generalizado" (historia), no se implementa.
- `closeness_centrality` (cap. 24) y el streaming por presupuesto (cap. 26):
  se citan como **destino**, sin entrar en fórmulas ni en presupuestos.
- DFS, componentes conexas, topológica, SCC: **cap. 5**, explícitamente
  diferido como gancho de cierre (solo contraste intuitivo BFS vs DFS).
- `BitSet`/estructuras de visita: cap. 26; aquí se dice "una marca que impide
  volver", no se construye.

## 2. Conceptos (del grafo curricular)
- `present`: **recorrido por niveles / frontera** (el CONCEPTO central);
  **cola FIFO** que materializa la frontera; **nivel / distancia en saltos**;
  **visita** (no volver a un nodo ya visto → BFS termina); **alcanzabilidad**;
  **orden de descubrimiento** BFS; **O(V+E)**.
- `practice`: modelado de grafo (cap. 7, Vol. II; caps. 1-2, Vol. I): el
  ejercicio de interleaving pide DIBUJAR un grafo y recorrerlo.
- `consolidate`: el trait `GraphStore` (cap. 8) se asume como *destino*
  (se anuncia, no se usa); identidad estable (cap. 3); qué es un camino (Vol. I).
- `out_of_scope`: pesos y Dijkstra (cap. 22); DFS/componentes/SCC (cap. 5);
  closeness (cap. 24); streaming por presupuesto y `BitSet` (cap. 26);
  CSR/adyacencias compactas (cap. 14).

## 3. Objetivos de dominio (taxonomía teaching)

### Knowledge — qué SABE al terminar (3-5 afirmaciones comprobables)
- **K1**. Explica que BFS recorre el grafo **por niveles** (frontera a *k*
  saltos) y que el primer hallazgo es el de **menor número de saltos**.
- **K2**. Justifica por qué **"¿existe un camino entre A y B?"** es LA consulta
  que una BBDD de grafos responde en O(V+E), no pagando O(V²) pares a pares.
- **K3**. Distingue **BFS** (ancho, niveles, más corto) de **DFS** (profundidad,
  backtracks) y de **Dijkstra** (ponderado, cap. 22).
- **K4**. Argumenta que **la visita** (marca de nodos y no volver) es la que
  hace que BFS termine en grafos cíclicos.
- **K5**. Reconoce que la complejidad O(V+E) proviene de visitar cada nodo a
  lo sumo una vez y examinar cada arista una vez al expandir su origen.

### Skills — qué HACE (2-3 tareas)
- **S1**. Ejecuta BFS a mano sobre un grafo dibujado: reproduce la cola, marca
  visitados y lista el **orden de descubrimiento** por niveles (retrieval).
- **S2**. Traduce una pregunta de negocio ("¿quién está a 2 saltos de A?") a
  "un BFS de A con distancia ≤ k" (análisis).
- **S3**. Reconoce (en palabras) cómo `closeness_centrality` (cap. 24) y
  `bfs_fronteras` (cap. 26) **reutilizan** este BFS por niveles.

### Wisdom — qué DECIDE (1-2 trade-offs)
- **W1**. Decide **CUÁNDO NO** usar BFS: si hay **pesos de arista** (precios,
  tiempos), BFS miente ("2 saltos" ≠ "2 km") → Dijkstra (cap. 22).
- **W2**. Decide que **alcanzabilidad** se responde con UN BFS desde el origen,
  no comparando V² pares — el trade-off memoria vs. coste de lecturas de
  adyacencia (que el cap. 26 cuantifica con presupuestos).

## 4. Modelo mental

**Figura ordenadora (única): un guijarro lanzado al estanque.** El origen es la
piedra en el centro; las olas son los niveles. La **frontera** es la OLA
ACTUAL: los nodos a exactamente *k* saltos. La siguiente ola son los nodos
**recién descubiertos**. Cada nodo se descubre UNA vez, cuando la ola lo toca
por primera vez — y esa es su distancia mínima. La **cola FIFO** es el
"muelle" que encola los nodos de la ola actual para sacarlos en orden de
descubrimiento (= orden de nivel).

**Diagrama ASCII** (ola expandiéndose, frontera marcada):

```
            ·            ← nivel 2 (ola lejana)
         ·   ·
       ·   F   ·         ← nivel 1 (frontera = ola media)
       ·  [·]  ·             [·] = origen (nivel 0)
       ·   ·   ·         ← · = nodos ya descubiertos
         ·   ·
            ·
  orden de descubrimiento 0 → 1 → 2 → 4 (frontera a frontera)
  cola FIFO:  [origen] → [v1, v2] → [v3, v4, v5, v6] → ...
```

**El momento ¡ajá!**: *descubrir un nodo en el nivel k es, por construcción,
estar a k saltos del origen — la estructura de olas lo garantiza, no hay que
"buscar" nada.*

## 5. Los porqués (grill)

### P1. ¿Por qué una COLA (FIFO) y no una pila o "expandir a lo loco"?
- **Qué resuelve**: ordenar los nodos por nivel. Sacar lo más antiguo primero
  garantiza que jamás expandirás un nodo del nivel *k+1* antes de terminar los
  del *k*. Una pila (LIFO) expande en profundidad (sería DFS, cap. 5).
- **Alternativa descartada**: "recorrer por orden de aparición" sin estructura
  (ver §6): encuentra, pero NO dice a qué distancia.
- **Modo de fallo si no se hace**: no hay niveles; "¿a cuántos saltos?" queda
  sin respuesta; la cercanía (cap. 24) y el menor-número-de-saltos se caen.
- **Evidencia**: CLRS cap. 22 (BFS); la pila daría DFS (CLRS cap. 22 como
  contraste).

### P2. ¿Por qué marcamos la VISITA (no volver atrás)?
- **Qué resuelve**: que BFS termine en grafos con ciclos. Sin visita, en un
  laberinto con bucles vuelves a nodos ya vistos y el bucle se eterniza.
- **Alternativa descartada**: "recordar el camino anterior" — solo evita la
  arista de vuelta, no el ciclo largo.
- **Modo de fallo**: recorrido infinito; complejidad peor que O(V+E).
- **Evidencia**: `visitado: BitSet` en `FronterasBfs` (cap. 26, función
  `expandir`: `visitado.marcar(v)`); CLRS 22.2.

### P3. ¿Por qué "existe un camino" cuesta O(V+E) y no O(V²)?
- **Qué resuelve**: la pregunta de alcanzabilidad DE UN origen. En el peor
  caso se visitan todos los V nodos y se examinan todas las E aristas: O(V+E).
  Comparar "1 contra 2", "1 contra 3"... sería O(V²) (o más con joins).
- **Alternativa descartada**: fuerza bruta o self-join SQL que paga O(n²) por
  el producto cruzado.
- **Modo de fallo**: pedir "¿están conectados?" en una red grande degenera a
  una consulta costosísima; la BBDD de grafos pierde su razón de ser.
- **Evidencia**: CLRS 22.2 (O(V+E)); rediseño de "friends-of-friends" que
  motivó los primeros GDBMS (historia).
- **Pregunta crítica de `CORPUS.yml` para este cap**: "¿qué diferencia hay
  entre BFS sobre CSR vs sobre HashMap de listas?" → **el mismo BFS**; cambia
  el *coste de la lectura de adyacencia* (contigüidad y caché en CSR, dispersión
  en hash), **no** el orden de niveles. El cap. 14 (CSR) y cap. 26 (streaming)
  lo demuestran con cifras.

### P4. ¿Por qué "por niveles" / "por fronteras"?
- **Qué resuelve**: separar el recorrido en olas para (a) saber la distancia y
  (b) leer SOLO lo necesario — semilla del streaming por presupuesto del
  cap. 26 (`RecorridoBfs.niveles[k]`; `bfs_streaming` lee bajo demanda "frontera
  a frontera").
- **Alternativa descartada**: BFS "de una pasada" sin guardar el nivel —
  barato en espacio pero pierde la estructura que closeness y streaming
  necesitan.
- **Modo de fallo**: no puedes computar distancia por nivel, ni cortar antes
  de leer todo el grafo (el 99,9% inútil que el cap. 26 evita).
- **Evidencia**: `cap26_proyeccion.rs` (`RecorridoBfs`, "Frontera a frontera:
  `niveles[k]` = nodos descubiertos a k saltos"); `cap24_centralidad.rs` (BFS
  por nodo con `dist: Vec<Option<u32>>`).

### P5. ¿Por qué BFS y no DFS para el camino más corto?
- **Qué resuelve**: el *menor número de saltos*. BFS descubre por niveles y
  asigna a cada nodo su distancia MÍNIMA al primer contacto. DFS puede tocar
  un nodo por un camino largo antes que por uno corto.
- **Alternativa descartada**: DFS — válido para existencia y para
  componentes/topología (cap. 5), malo para distancia mínima.
- **Modo de fallo**: usar DFS (o "recorrer según aparezca") para "amigos de
  amigos en k saltos" podría afirmar distancia mayor que la real.
- **Evidencia**: CLRS 22.2 (BFS y caminos mínimos no ponderados); Dijkstra
  1959 como el hermano ponderado (BFS = pesos 1), cap. 22.

## 6. Primera solución vs solución evolucionada

- **Versión ingenua (novato)**: *"recorrer por orden de aparición"* — sigues a
  los vecinos según aparecen, sin cola explícita ni marca de nivel; basta un
  `Vec<bool>` "ya visto". Límite: **recorres pero no sabes a qué distancia**.
  Lo que la rompe exactamente: la pregunta "¿a qué distancia está Y de X?" o
  "¿quién está en el nivel 2 de X?" — el recorrido sin niveles devuelve una
  lista sin orden por distancia.
- **Cómo evoluciona**: se introduce la **cola FIFO + marca de nivel por nodo**
  (o equivalente: encolar el nodo con su `distancia = dist[padre]+1`). Cuando
  sacas todos los de nivel *k* (frontera actual), la siguiente frontera son
  exactamente los del *k+1*. Diferencia visible: respuesta por capas; distancia
  derivable al instante.
- **Nota**: este capítulo es conceptual; la "evolución" se materializa en el
  workspace del cap. 26 (`niveles: Vec<Vec<NodeId>>`), no en un fichero propio.

## 7. Prueba de fuego

- El escenario que demuestra que lo aprendido FUNCIONA es **conceptual +
  cruzado**: ejecutar BFS a mano sobre un grafo dibujado y comprobar que el
  orden de descubrimiento coincide con la cola FIFO "de libro" (CLRS). Y,
  puente al workspace: que el lector **RECONOZCA** en
  `liradb-workspace/crates/vol2-liradb/src/cap24_centralidad.rs`
  (`closeness_centrality` → distancias por saltos, Wasserman-Faust) y en
  `cap26_proyeccion.rs` (`bfs_fronteras`, `niveles[k]`) el MISMO BFS de este
  capítulo.
- **Qué fallaría si se saltara este capítulo (síntoma detectable)**: frente al
  código del cap. 26, no sabría qué es `niveles[k]`, por qué se ordena la
  frontera por id (determinismo), ni qué pide la cola; frente al cap. 24,
  pensaría que `closeness` es una suma misteriosa y no "un BFS por nodo"
  (con su `O(V·(V+E))` anotado).
- **Tests consultados (no de este cap)**: `closeness_camino_lineal_valores_de_libro`
  (0.5 / 0.75 / 0.5 para el camino 0-1-2-3) y `bfs_fronteras` con
  `Presupuesto::profundidad(1)` → `vec![0]` (frontera 0).

## 8. Trampas y errores comunes

### Los 3 errores que comete TODO el mundo aquí
1. **Marcar el nodo como visitado al SACARLO en vez de al DESCUBRIRLO** —
   el mismo nodo entra a la cola más de una vez desde varios vecinos del
   mismo nivel. Síntoma: nodos duplicados en el orden de descubrimiento.
   Regla: **se marca al descubrir (encolar), no al sacar**.
2. **Usar una pila o un `Vec` sin cola** — creen que da igual. Con pila es
   DFS (no garantiza distancia); con lista sin orden, todo mezclado. Síntoma:
   el "orden de descubrimiento" no es por niveles. Regla: **cola FIFO +
   guardar la distancia al encolar**.
3. **Confundir BFS con Dijkstra ponderado** — aplicar BFS cuando hay pesos
   (precios, tiempos) da un "menor número de saltos" que NO es el menor
   coste. Síntoma: una ruta de 3 saltos caros "gana" a una de 4 baratas.
   Regla: **BFS solo si cada arista "vale 1"**; con pesos → cap. 22.

### Precisión de lenguaje (glosario)
| Término | Significado exacto | No confundir con |
|---|---|---|
| Frontera / nivel | Nodos a *k* saltos del origen | Visita — la marca que impide volver |
| Visita | Nodo ya descubierto (no se vuelve) | Descubrimiento — primera vez que lo toca la ola |
| Alcanzabilidad | Existe (al menos) un camino | Conectividad/conexidad — componente (cap. 5) |
| Camino mínimo BFS | Menor número de saltos (aristas) | Camino de peso mínimo — Dijkstra (cap. 22) |
| Cola FIFO | Saque del más antiguo al más nuevo | Pila LIFO — DFS (cap. 5) |

## 9. Ejercicios (exercise-designer)
- **`recordar/aplicar` (Retrieval)**. "Dibuja un grafo: `Ana` conectada con
  `Bruno`, `Clara` y `David`; `Bruno` con `Elena` y `Fran`; `Clara` con
  `Gonzalo`. Ejecuta BFS **a mano** desde `Ana`: en cada paso escribe el
  contenido de la cola, marca los visitados y lista el **orden de
  descubrimiento por niveles**." **Sin pistas** en el enunciado. Verificado
  contra la cola FIFO "de libro" (CLRS) — la solución se comprueba
  comparando con `niveles` de un BFS por niveles (cap. 26). Criterio:
  `niveles = [[Ana],[Bruno,Clara,David],[Elena,Fran,Gonzalo]]`.
- **`analizar`**. "El grafo representa una red social; traduce la consulta
  '¿quién está a exactamente 2 saltos de Ana?' y '¿existe un camino de Ana a
  Gonzalo?' a pasos de BFS; ¿dónde fallaría un SQL self-join?" Criterio:
  identifica BFS de Ana con cota de profundidad k=2 vs. BFS completo;
  explica O(V+E) frente a O(V²).
- **`crear` (Interleaving con cap. 7)**. "Modela (con la voz del cap. 7:
  nodos/aristas tipadas) un mini-grafo personas → aeropuertos → vuelos
  *dirigidos*, donde una arista `VUELA_DE(A→B)` no sirve a la inversa.
  Recorre en BFS dirigido desde 'tú' hasta 'Hong Kong' y da su distancia en
  saltos. Nota qué aristas NO son válidas por su dirección." Criterio: un
  grafo dibujado + un BFS dirigido correcto; escribe qué aristas descartas
  por sentido.

## 10. Preguntas abiertas (gancho al cap. 5)
1. ¿Qué pasa si en vez de expandir por niveles exploro hasta el fondo y
   vuelvo (backtrack)? Aparece el **DFS** — y con él, saber si el grafo está
   en **componentes conexas** separadas (¿"existe un camino" tiene ahora una
   única respuesta global?).
2. Necesito ordenar el grafo para construir una secuencia de tareas antes de
   sus dependencias: el cap. 5 introduce **orden topológico** sobre un DFS.
3. Tras BFS sé quién es alcanzable; el cap. 5 responde "¿quién alcanza a
   quién *mutuamente*?" con **SCC** (componentes fuertemente conexas).

**Términos nuevos para glosario** (`book-memory-keeper`): frontera, nivel,
cola FIFO, visita, alcanzabilidad, O(V+E), orden de descubrimiento,
bit/pila (contraste DFS), "k saltos".

## 11. Diseño de retención (skill `teach`)
- **Retrieval practice**: el ejercicio `recordar/aplicar` (ejecutar BFS a mano
  sin pistas) obliga a recuperar el algoritmo desde la memoria, no a
  reconocerlo. (Recordar > reconocer.)
- **Spacing**: se re-usa y EJERCITA el **modelado del cap. 7 / Vol. I caps.
  1-2** (ejercicio `crear`: dibujar y modelar un grafo tipando aristas) y la
  **arista dirigida** del cap. 7.
- **Interleaving**: los ejercicios mezclan *teoría BFS* con *modelado del
  capítulo anterior* (dibujar + recorrer) en el `crear`; el `analizar` mezcla
  BFS con *coste de consulta relacional* (por qué el self-join es caro).
- **Regla de dificultad asimétrica**: la explicación tiene UNA idea nueva por
  sección (ola → cola → visita → complejidad → ancla); la dificultad está en
  los ejercicios (recuperación del algoritmo a mano).
- **Bucle de feedback inmediato**: el capítulo no tiene tests propios (es
  conceptual), pero da feedback cruzado verificable: comparar el BFS a mano
  con `RecorridoBfs.niveles` del cap. 26 (`cargo test -p vol2-liradb --lib
  cap26_proyeccion`) y con `closeness_centrality` del cap. 24.
- **Citas**: Dijkstra (1959); CLRS 22.2 (BFS, complejidad, caminos mínimos
  no ponderados); Freeman 1978 + Wasserman-Faust (closeness cap. 24); Moore
  (laberintos, años 50); Milgram 1967 (seis grados); los BFS por niveles del
  workspace (cap. 24/26).

---

## Checklist de profundidad (antes de marcar DONE)
- [x] Cada decisión técnica tiene su «porqué» con fuente (P1-P5 con CLRS /
      Dijkstra / workspace).
- [x] Existe un escenario de fallo visible, no solo el happy path (§6 y §8).
- [x] **Código ejecutable**: capítulo CONCEPTUAL — el código ancla vive en
      cap. 24/26; la prosa lo referencia sin duplicarlo.
- [x] Hay al menos una misconception corregida explícitamente (5 en §1; 3 en §8).
- [x] Los ejercicios tienen solución verificada (contra cola FIFO y `niveles`).
- [x] Hay ≥1 ejercicio de retrieval practice (BFS a mano) y ≥1 toque a
      concepto anterior (modelado cap. 7 — spacing/interleaving).
- [x] Responde la pregunta crítica de `CORPUS.yml` vol-II-cap-04:
      "¿Qué diferencia hay entre BFS sobre CSR vs sobre HashMap de listas?"
      → P3 (cambia el coste de la `adyacencia`, no el orden de niveles).
- [x] Ancla los nombres reales `closeness_centrality` (cap. 24),
      `bfs_fronteras` / `bfs_streaming` / `niveles[k]` (cap. 26) y el trait
      `GraphStore` (cap. 8) como «lo que aquí es una idea, allí es el
      corazón del motor».