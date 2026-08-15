# Capítulo 21 — Un optimizador pequeño pero real (`liradb explain`)

> *«El usuario dice QUÉ quiere. El motor decide CÓMO conseguirlo. Esa frontera tiene nombre: optimizador.»*

## 21.0 La anécdota de la esquina

En 1979, en el laboratorio de IBM de San José, Patricia Selinger y su equipo (Astrahan, Chamberlin, Lorie y Price) publicaron en SIGMOD un paper con un título tan sobrio como su contenido era revolucionario: «Access Path Selection in a Relational Database Management System». Describe la pieza que le faltaba a System R —la base de datos que estaba definiendo cómo sería SQL— para no ser un juguete lento: **el primer optimizador basado en coste**.

El paper es famoso por la enumeración dinámica de órdenes de joins. Pero dos detalles suyos te van a sonar muchísimo dentro de unas páginas. Primero: cuando no había estadísticas, System R asumía que una igualdad (`col = valor`) deja pasar **una décima parte** de las filas y un rango (`col < valor`), **un tercio**. Exactamente los `0.1` y `1/3` que usarás hoy en LiraDB. Segundo: entre sus heurísticas explícitas estaba «anidar los predicados lo más profundamente posible en el árbol de consulta» — lo que hoy llamamos **predicate pushdown**. Cuarenta y tantos años después, esa sigue siendo la regla número 1 del optimizador de cualquier motor que puedas nombrar, del PostgreSQL de tu servidor al Catalyst de Spark.

Y la herramienta para VERLO también tiene historia: `EXPLAIN` existe desde los orígenes de PostgreSQL en Berkeley, y la variante que además ejecuta la consulta para contrastar —`EXPLAIN ANALYZE`, «que muestra tiempos y recuentos de filas», según las notas de la 7.2.0, febrero de 2002— es el antepasado directo del `liradb explain` que construiremos hoy: plan ANTES, plan DESPUÉS, y las filas reales al final para comprobar cuánto mienten las estimaciones.

## 21.1 Objetivo

Al terminar este capítulo sabrás **por qué un motor que ya parsea, planifica y ejecuta (caps. 17-20) todavía deja la mitad del trabajo en la mesa**, y habrás construido la pieza que lo reclama: un **optimizador** — un programa que reescribe el plan lógico antes de ejecutarlo para que haga el mismo trabajo con menos esfuerzo.

Tres piezas, las tres en `cap21_optimizador.rs`:

1. **El catálogo** (`Catalog`) — estadísticas recolectadas del `GraphStore`: nodos por etiqueta, grados medios out/in, aristas por tipo, y un índice de igualdad.
2. **La estimación de cardinalidad** (`estimate`) — heurísticas simples y documentadas para adivinar cuántas filas producirá cada operador.
3. **Las cinco reglas** (`optimize`) — reescrituras en orden fijo que transforman el plan ingenuo del cap. 19 en el plan que ejecuta el Volcano del cap. 20.

Y el hito: `liradb explain "..."`, que enseña el antes y el después con estimaciones y filas reales.

## 21.2 Problema

Ejecuta el demo del capítulo anterior y mira la última consulta, la canónica del brief:

```text
LiraQL: MATCH (p:Person)-[r:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name, r.since
Plan lógico:
Project(f.name, r.since)
  Filter(f:Person AND p.name = "Ana")
    Expand(p, r:KNOWS, OUTGOING, f)
      NodeScan(Person AS p)
Métricas: Project: 1 filas
Filter: 1 filas
Expand: 4 filas
NodeScan: 4 filas
filas devueltas: 1
```

Lee las métricas de abajo arriba: el escaneo produce 4 filas, la expansión produce 4, y el filtro… devuelve 1. **Escaneamos 4 para devolver 1.** Con 6 nodos en el grafo demo eso es una anécdota; con 10 millones de personas es un delito. Y lo peor: el plan ni se inmuta, porque es exactamente lo que `lower()` (cap. 19) le pidió — pusimos el `Filter` encima de todo y el cap. 20 lo ejecutó fielmente. Las métricas del cap. 20 ya numeraban esta ineficiencia a propósito: son el mejor anuncio del optimizador.

La pregunta del capítulo: ¿quién decide que, en vez de escanear todas las personas y filtrar Ana al final, empieces directamente POR Ana?

## 21.3 Modelo mental

Piensa en el **planificador de rutas de un GPS**. Tú dictas el destino: «de mi casa al aeropuerto». Eso es la consulta — intocable. El GPS elige el ORDEN de las calles: hoy la M-30 está colapsada (lo sabe porque MIRA el tráfico), así que va por la alternativa. Mismo destino, misma llegada, distinto camino. Y si mañana el tráfico cambia, el camino cambia — pero tu destino no.

Traduce: la consulta del usuario es el destino; el plan lógico es la lista de calles; las estadísticas del catálogo son el tráfico; y el optimizador es el planificador. Tres consecuencias que vertebran todo el capítulo:

1. **El GPS nunca te cambia el destino** — el optimizador nunca cambia los resultados (lo probaremos con tests de equivalencia).
2. **El GPS necesita mirar el tráfico** — sin estadísticas del grafo no hay decisión posible, sólo manías.
3. **Convenir un orden de cálculo fijo** — el GPS evalúa primero autopistas, luego avenidas, luego calles: reglas en orden conocido, mismo input → mismo output.

```
        consulta (destino, del usuario)
                   │
        ┌──────────▼──────────┐
        │   OPTIMIZADOR       │  mira el catálogo (tráfico):
        │  5 reglas fijas     │  Person: 4 · out 1.50 / in 1.00
        │  R1..R5             │  KNOWS: 4 de 6 aristas
        └──────────┬──────────┘
     plan ANTES ──►│──► plan DESPUÉS   (mismo resultado, menos trabajo)
```

El momento ¡ajá!: hasta ahora, el plan lógico decía QUÉ hacer y el motor lo obedecía al pie de la letra. Hoy descubres que un mismo QUÉ admite muchos órdenes de cálculo equivalentes, y que elegir bien entre ellos exige **mirar los datos**.

## 21.4 Primera solución

La versión ingenua ya la tienes: es no tener optimizador. El plan de `lower` baja tal cual a `compile` (cap. 20, que a propósito era 1:1) y se ejecuta como llegó. Si el usuario quiere velocidad, que escriba mejor la consulta — o que el código que llama al motor construya el plan a mano y use `IndexSeek` directamente, que el operador existe desde el cap. 20.

Y una segunda versión ingenua, más tentadora: «pues que el motor reescriba la CONSULTA» — que detecte `WHERE p.name = "Ana"` y la convierta en otra consulta mejor escrita antes de planificarla.

## 21.5 Sus límites

Ambas se rompen en cuanto te alejas del juguete:

1. **El AST es del usuario.** Reescribir su consulta rompe el contrato de fidelidad texto→AST que construimos en los caps. 17-18: los errores dejan de apuntar a SUS palabras, y cualquier reescritura del texto exige re-parsear, re-ligar y re-verificar. El plan, en cambio, es nuestra representación interna: reordenar ligaduras ahí es invisible para él. Optimizamos el plan, no la consulta.
2. **La combinatoria se come la búsqueda exhaustiva.** Elegir «el mejor plan» enumerando todos es caro: con n joins hay n! órdenes (10 joins = 3.628.800). System R ya lo sabía: por eso enumeraba con programación dinámica y además recortaba a árboles left-deep. Nosotros, con reglas totales en orden fijo, pagamos cero combinatoria.
3. **El 4-para-1 escala linealmente.** Extrapolalo: si escaneas 4 filas para devolver 1 en un grafo de 40 millones de nodos con ese ratio, mueves ~160 millones de filas para responder con 40. El plan ingenuo no empeora con los datos — empeora CONTIGO, multiplicando tu muestreo.
4. **El caller con `IndexSeek` a mano no tiene datos.** ¿Merece la pena el índice? Depende de cuántos ids devuelva frente a cuántas filas escanea el `NodeScan` — información que vive en el grafo, no en quien llama.

La conclusión: necesitamos una pieza nueva que (a) reescriba planes, no consultas; (b) mire el grafo antes de decidir; (c) garantice los mismos resultados. Ese es el optimizador.

## 21.6 Solución evolucionada, parte 1: el catálogo (o mirar el tráfico)

Antes de elegir ruta hay que saber cómo está el tráfico. `Catalog::collect(&dyn GraphStore)` hace UNA pasada por el store y recolecta: nodos totales, nodos por etiqueta, aristas por tipo, grados saliente/entrante acumulados por etiqueta de los extremos, y un índice de igualdad `(etiqueta, propiedad, valor) → ids`:

```text
Catálogo (estadísticas del store): 6 nodos · 6 aristas
  Person: 4 nodos · grado medio out 1.50 / in 1.00
  City: 2 nodos · grado medio out 0.00 / in 1.00
  aristas por tipo: KNOWS 4, LIVES_IN 2
```

Eso no es una salida inventada: es literalmente lo que imprime `liradb explain` sobre el grafo demo. Y fíjate en el porqué de cada número:

- **¿Por qué grados medios POR ETIQUETA y no globales?** Porque el coste de un `Expand` desde `f` depende de cómo son las Person, no de cómo es el grafo en general. Una etiqueta hub (grado 500) y una hoja (grado 1) no pueden compartir media.
- **¿Por qué un índice de igualdad si el cap. 15 ya construyó índices?** El `HashIndex`/`BPlusTree` del cap. 15 viven en el fichero en disco; este catálogo es su primo en memoria, reconstruido por consulta. En un sistema real el catálogo persistiría y se mantendría incrementalmente (esa es exactamente la infraestructura natural del cap. 15); aquí reconstruirlo cuesta un escaneo y nos da un catálogo obviamente correcto del que razonar. Los índices del cap. 15, por fin, tienen alguien que decide cuándo usarlos.
- **¿Por qué constantes NO mágicas?** Porque sin mirar el grafo no hay decisión posible: la regla R1 compara costes reales (1.50 de grado out de Person, fracción 4/6 de KNOWS). Con constantes a ciegas, «optimizador» sería un nombre bonito para una manía.

## 21.7 Solución evolucionada, parte 2: estimar cardinalidad (la sección de estadísticas)

El catálogo dice cómo es el grafo; la **estimación** traduce eso a «cuántas filas producirá este operador». La función `estimate` es un pequeño `match` recursivo con fórmulas que caben en una servilleta:

```text
  NodeScan         → nodos con la etiqueta (todos si ANY)
  IndexSeek        → ids resueltos (exacto)
  Filter           → entrada × selectividad del predicado
  Expand           → entrada × grado medio de la dirección × fracción del tipo
  Project          → lo que produce su entrada
  CartesianProduct → izquierda × derecha
```

Y la **selectividad** de un predicado (la fracción de filas que se espera que sobrevivan) usa los defaults de 1979 cuando no hay estadística, y la estadística cuando la hay:

```rust
pub const SEL_EQ: f64 = 0.1;        // igualdad sin estadística (System R)
pub const SEL_RANGE: f64 = 1.0 / 3.0; // rango <, <=, >, >= (System R)
pub const SEL_NOT_EQ: f64 = 0.9;
pub const SEL_UNKNOWN: f64 = 0.5;
```

Con `AND` se multiplican (independencia), `OR` usa inclusión-exclusión, `NOT` complementa. Y el caso estrella: si el predicado es `v.prop = literal` y el índice de igualdad del catálogo conoce esa clave, la selectividad es EXACTA — `ids / nodos de la etiqueta`. Si el valor no ocurre (buscas a «Zoe» y no existe), la selectividad es 0.0: no filtras, aniquilas.

Apliquémoslo al plan ANTES del problema del §21.2, con el catálogo real:

```text
NodeScan(Person AS p)        → 4                      (hay 4 Persons)
Expand(p, KNOWS, OUT, f)     → 4 × 1.5 × (4/6) = 4    (grado × fracción de tipo)
Filter(f:Person ∧ f.age<40)  → 4 × 1.0 × 1/3 = 1.33   → est. 1
```

**¿Por qué heurísticas simples y no muestreo?** Porque la estimación sólo necesita **ordenar planes**, no prometer costes: para elegir entre empezar por `p` o por `f` basta saber cuál es más barato, con error del 50 % incluido. Un histograma o un muestreo serían infraestructura desproporcionada para comparar dos candidatos — y nos robarían el momento pedagógico de ver, con números, cuánto mienten las heurísticas. Porque mienten: en el demo, 3 de 4 personas tienen `age < 40` (selectividad real 0.75, no 1/3). Ya volveremos a ello: esa discrepancia es contenido, no bug.

## 21.8 Solución evolucionada, parte 3: las cinco reglas

`optimize(plan, &catalog)` aplica cinco reescrituras **en orden fijo**:

| # | Regla | Qué hace |
|---|---|---|
| R1 | `rule_selective_start` | Elige la variable más selectiva como punto inicial y reordena la cadena de `Expand` (los tramos a su izquierda se recorren con la dirección invertida). Es el «join ordering» de los grafos. |
| R2 | `rule_predicate_pushdown` | Parte los `AND` en átomos y baja cada átomo lo más profundo posible — sin cruzar variables que aún no están ligadas. |
| R3 | `rule_absorb_label` | El `HasLabel` del nodo escaneado se integra en la etiqueta del `NodeScan` (que filtra al escanear). |
| R4 | `rule_index_seek` | `Filter(v.prop = literal) + NodeScan` → `IndexSeek` con los ids del catálogo — **sólo si ahorra**: si el índice devolviera tantas filas como el escaneo, se queda el scan. |
| R5 | `rule_prune_projections` | Elimina proyecciones de identidad redundantes. |

**¿Por qué R2 —el predicate pushdown— es LA regla reina?** Números: en el demo pagamos 4 filas escaneadas para devolver 1. El `Filter` de la edad está ENCIMA del `Expand`, así que el motor expande cada candidata y luego la tira. Bájalo al escaneo y las filas que no cumplen `age < 40` ni siquiera entran al pipeline: no se expanden, no se filtran, no existen. Filtrar antes de expandir convierte trabajo proporcional al grafo en trabajo proporcional al resultado. Es la misma regla que Selinger escribió en 1979 y que hoy ejecuta tu PostgreSQL cada vez que lanza una query.

**¿Por qué respetando bindings?** Esta es la trampa. Bajar un átomo que menciona una variable que ahí abajo aún NO está ligada no optimiza: cambia la semántica (el runtime evaluaría la propiedad contra otra fila, o no la encontraría). La implementación lo respeta con la misma herramienta del cap. 19: `sink` consulta `bound_variables()` del subárbol y sólo hunde lo que menciona variables ya ligadas. Un `Filter(p.age > 30)` sobre un `Expand(f → p)` se queda donde está: `p` la liga la expansión. Un optimizador que cambia resultados no es un optimizador: es un bug con buen marketing.

**¿Por qué orden fijo y documentado?** Determinismo didáctico: mismo input → mismo output, siempre. R1 corre primero porque decide la FORMA de la cadena (por dónde empezar a ligar); R2 cuelga los predicados del plan resultante; R3 y R4 pulen el escaneo que quedó abajo; R5 barre. Con reglas iteradas hasta fijación o en orden arbitrario, los planes serían irreproducibles — imposibles de enseñar, de testear y de explicar en un `explain`. El test `optimizar_es_idempotente_y_conservador` además verifica que las reglas convergen: optimizar dos veces da lo mismo que una.

## 21.9 El hito: `liradb explain`

Todo junto, ejecutado de verdad en el workspace:

```console
$ cargo run -q -p liradb-cli -- explain \
    "MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name"

liradb explain — optimizador (cap. 21)
Consulta: MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE f.age < 40 RETURN p.name, f.name

Catálogo (estadísticas del store): 6 nodos · 6 aristas
  Person: 4 nodos · grado medio out 1.50 / in 1.00
  City: 2 nodos · grado medio out 0.00 / in 1.00
  aristas por tipo: KNOWS 4, LIVES_IN 2

Plan ANTES (lower, cap. 19):
Project(p.name, f.name)            est. 1 filas
  Filter(f:Person AND f.age < 40)  est. 1 filas
    Expand(p, KNOWS, OUTGOING, f)  est. 4 filas
      NodeScan(Person AS p)        est. 4 filas

Plan DESPUÉS (optimize, cap. 21):
Project(p.name, f.name)          est. 1 filas
  Expand(f, KNOWS, INCOMING, p)  est. 1 filas
    Filter(f.age < 40)           est. 1 filas
      NodeScan(Person AS f)      est. 4 filas

Filas reales al ejecutar el plan optimizado: 3 (raíz estimada: 1)
```

Lee el DESPUÉS con calma, porque en cuatro líneas están las cinco reglas:

1. **R1**: la cadena ya no empieza por `p` sino por `f` — el coste estimado de empezar por f (`4 × 1/3 × 1.0 × 4/6 ≈ 0.89`) gana al de empezar por p (`4 × 1.5 × 4/6 = 4`). El tramo KNOWS se recorre en sentido INCOMING: dar la vuelta a la flecha es gratis, la semántica es la misma.
2. **R2**: el AND se partió — `f.age < 40` bajó y quedó PEGADO al escaneo de f.
3. **R3**: `f:Person` desapareció como filtro: se absorbió en `NodeScan(Person AS f)`.
4. **R4**/**R5**: aquí no aplican (no hay igualdad ni proyecciones sobrantes) — y el explain de la canónica del brief te muestra la R4 en acción: `Filter(p.name = "Ana") + NodeScan` se convierte en `IndexSeek(Person.name = "Ana")` que lee UN nodo en vez de cuatro.

Y la última línea es la lección más honesta del capítulo: **la raíz estimada era 1 y las filas reales son 3**. La heurística de rango (1/3) subestimó la selectividad real (3/4: Ana 36, Carla 29 y Dani 36 pasan; Bo 41 no). ¿Es un bug? No: la estimación cumplió SU trabajo — ordenar candidatos (f era más barato que p, y sigue siéndolo con 0.75) — y falló el que NO tenía (predecir filas). PostgreSQL vive de la misma tensión: por eso su `EXPLAIN ANALYZE` (7.2.0, 2002) muestra, como nosotros, estimadas y reales lado a lado. Cuando la discrepancia importa, se refinan las estadísticas (histogramas); cuando no, se agradece el orden correcto.

Fíjate también en qué NO cambió: las columnas y las filas. Tres antes, tres después, las mismas. Eso no es suerte: es un contrato testeado.

## 21.10 Prueba de fuego: equivalencia, o el GPS nunca te cambia el destino

La prueba de fuego de un optimizador no es «va más rápido»: es **«va más rápido Y devuelve exactamente lo mismo»**. El test `equivalencia_antes_y_despues_sobre_bateria_de_consultas` ejecuta 12 consultas (la canónica, filtros en ambos lados, dirección entrante, sin dirección, caminos de tres nodos, anónimos intermedios, cartesiano con filtros, propiedades inline y de arista, self-loops, etiqueta inexistente, OR) por los DOS caminos — plan ingenuo de `lower` y plan optimizado de `optimize` — y compara:

- **columnas**: idénticas;
- **filas**: idénticas como multiconjunto ORDENADO.

¿Por qué ordenadas? Porque sin `ORDER BY` (que LiraQL aún no tiene) el orden de las filas no es parte del contrato — exactamente como en SQL, y exactamente porque un optimizador que reordena ligaduras puede producir las filas en otro orden. Lo que se promete es el contenido, no la secuencia.

¿Y qué pasaría si ROMPIÉRAMOS la equivalencia? Imagina el pushdown mal hecho del §21.8: `f.age < 40` empujado por debajo del `Expand` que liga `f`. En el plan, el filtro se evaluaría para cada `p` contra una `f` que aún no existe. Síntoma: filas de menos (o de más) SIN error — la peor clase de bug, porque la consulta «funciona». Los tests de equivalencia son el detector: `ingenuo ≠ optimizado` en cualquier consulta es un fallo del test, no una curiosidad.

Desde este capítulo, `run()` y `Query::execute` pasan SIEMPRE por el optimizador (`lower` → `Catalog::collect` → `optimize` → `Executor`): el usuario escribe la consulta; el motor elige la ruta.

## 21.11 Repaso de la Parte IV: la cadena completa

Este capítulo cierra la Parte IV. Reconstruyamos la cadena que ya sabes construir, de izquierda a derecha:

```
 texto LiraQL ──parse──► AST ──lower──► LogicalPlan ──optimize──► LogicalPlan' ──compile──► operadores ──open/next/close──► filas
   (cap. 17:      (cap. 18:       (cap. 19: binder     (cap. 21: catálogo       (cap. 20: 1:1            (cap. 20:
   qué es          tokens +        + plan lógico;      + estimaciones +          al árbol                modelo
   LiraQL)         errores con     el QUÉ sin el       5 reglas; el             físico)                 Volcano)
                  byte y línea)    CÓMO)               CÓMO barato)
```

Cada capa dejó una garantía que las siguientes heredan: el **lenguaje** (17) fijó MATCH-WHERE-RETURN sin prometer orden; el **parser** (18) garantiza que el AST es fiel al texto o señala el byte exacto; el **lowerer** (19) garantiza variables únicas y ligadas (`bound_variables()` — la herramienta con la que R2 respeta bindings); el **Volcano** (20) garantiza el ciclo open/next/close y NUMERA lo que fluye (sus métricas destaparon el 4-para-1); y el **optimizador** (21) garantiza mismos resultados con menos trabajo. Quita cualquier eslabón y la cadena se rompe en un sitio previsible: sin parser no hay feedback de errores; sin binder no hay pushdown seguro; sin métricas no sabrías que había nada que optimizar; sin optimizador, los índices del cap. 15 siguen siendo un órgano sin función.

## 21.12 Qué hemos sacrificado

1. **Estimaciones honestas, no precisas.** 1/3 para todo rango subestima el 0.75 del demo. El precio de la simplicidad; el premio: ver la discrepancia en cada explain.
2. **Búsqueda por coste real.** Nuestras reglas reordenan CADENAS simples; los cartesianos múltiples y los grafos cíclicos con backtracking quedan como están. Un optimizador de coste enumeraría más espacio.
3. **Catálogo no persistente.** Recalcular por consulta cuesta un escaneo completo: impensable en producción, perfecto para razonar sin dudar de la frescura de los números.
4. **Orden de filas no garantizado.** Consecuencia necesaria de reordenar ligaduras; llegará `ORDER BY` y con él la obligación.
5. **Igualdad exacta en el índice del catálogo.** `p.age = 36.0` no encuentra el `36` almacenado (sin la promoción Int/Float del runtime). Las estadísticas estiman; la ejecución decide.

## 21.13 Cómo lo hace una BBDD real

- **System R / Selinger (1979)**: el origen de todo. Coste = páginas leídas + CPU, selectividades por defecto (1/10, 1/3), enumeración dinámica de joins con poda a árboles left-deep y la heurística de hundir predicados que hoy es nuestra R2.
- **PostgreSQL**: planificador de coste con estadísticas mantenidas por `ANALYZE` (histogramas, distinct values, MCV). Su `EXPLAIN` imprime el árbol con `rows` estimadas; `EXPLAIN ANALYZE` (7.2.0, 2002) además lo ejecuta y muestra `actual rows` y tiempos por nodo — el molde exacto de nuestro «est. N filas» contra «filas reales».
- **Catalyst (Spark)**: optimizador funcional por reglas + coste: parseo → plan lógico sin resolver → resolver → reescrituras (pushdown de filtros y proyecciones incluidas) → planificación física eligiendo estrategias. Nuestro pipeline ANTES/DESPUÉS es la misma película en miniatura.
- **Kùzu (grafos)**: optimizador de grafo con reordenación de joins por coste, filtro pushdown hacia los escaneos y uso del catálogo de estadísticas del grafo — punto por punto, las tres piezas de este capítulo, a escala industrial.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en la consulta del §21.9, ¿por qué el `Expand` del DESPUÉS estima 1 si el grado in de Person es 1.00 y la fracción de KNOWS 4/6?
- *Intermedio*: ¿qué pasa —plan y filas— si el WHERE fuera `f.age < 100`? ¿Cambia R1? ¿Debería?
- *Experto*: diseña `rule_pushdown_limit` para un hipotético `Limit` encima de un `Filter`: ¿cuándo es seguro bajarlo? ¿Y sobre un `Expand` (pista: top-k)?

## 21.14 Lo que te llevas

- El **optimizador** reescribe el PLAN (nuestra IR), nunca la consulta (suya): mismo destino, otra ruta.
- El **catálogo** mira el grafo: nodos por etiqueta, grados medios, tipos, índice de igualdad. Sin datos no hay decisión, hay manía.
- **Estimar** con heurísticas (System R: 0.1 / 1/3) basta para ORDENAR planes; la discrepancia con lo real (est. 1 vs 3) es contenido, no bug.
- **Predicate pushdown** (R2) es la regla reina: filtrar antes de expandir convierte 4-para-1 en 1-para-1 — de 1979 a hoy.
- **IndexSeek lo elige el optimizador** con la única información que lo hace seguro: cuántos ids devuelve frente a cuántos escanea.
- **Equivalencia testada**: un optimizador que cambia resultados es un bug; sin ORDER BY, el multiconjunto sí, el orden no.

## 21.15 Ojo, cuidado con…

- **Bajar predicados sin mirar bindings**: la trampa nº 1. `sink` usa `bound_variables()`; tú, usa `sink`.
- **Confundir selectividad con cardinalidad**: la primera es una fracción (0.25), la segunda un número de filas (4 × 0.25 = 1). El explain muestra la segunda.
- **IndexSeek siempre**: si el índice devuelve tantas filas como el escaneo, no ahorra; R4 sólo aplica si `ids.len() < scan_rows`.
- **Leer `est. 1 filas` como una promesa**: es un argumento de comparación, no una respuesta. La respuesta son las filas reales de abajo.
- **Catálogo vs índice**: el catálogo AYUDA A DECIDIR; el índice RESUELVE la búsqueda. Uno informa reglas; el otro ejecuta lecturas.

## 21.16 Pin de batalla

> *«Un optimizador que devuelve resultados distintos no está optimizando: está mintiendo más rápido.»*

## 21.17 Si solo lees 30 segundos

El plan que sale de `lower` es correcto pero ingenuo: filtra al final, escanea de más. El optimizador lo reescribe en cinco pasos fijos — elegir el punto inicial más selectivo, bajar los predicados, absorber etiquetas, convertir igualdades en `IndexSeek`, podar proyecciones — usando estadísticas del grafo (cuántos nodos por etiqueta, qué grados medios) para comparar candidatos. Los resultados NO cambian: se testea la equivalencia antes y después. `liradb explain` te enseña el ANTES, el DESPUÉS, las filas estimadas y las reales — y la diferencia entre ellas es la lección.

## 21.18 Una historia pequeña

Cuando conectamos el optimizador por primera vez, `liradb query` dejó de devolver filas en una consulta del demo. Pánico: habíamos roto el motor. Media hora después, el culpable: un pushdown demasiado entusiasta empujaba `f.age < 40` por debajo del `Expand` que liga `f` — el filtro evaluaba la edad de una variable que aún no existía. El fix fue una línea: preguntarle al subárbol sus `bound_variables()`. La moraleja se nos quedó grabada: el optimizador trabaja sobre el contrato del binder del cap. 19; quien reescribe planes sin saber qué variables están ligadas no está optimizando, está apostando.

## Ejercicios resueltos

**1. En la salida real del §21.9, ¿por qué el plan ANTES estima 4 en el `Expand` y 1 en el `Filter`?**

`NodeScan(Person AS p)` estima 4 porque el catálogo cuenta 4 Person. El `Expand` multiplica su entrada por el grado medio OUT de Person (1.50 — seis aristas salen de personas: Ana 2, Bo 2, Carla 1, Dani 1, sobre 4 nodos) y por la fracción de aristas KNOWS (4/6): 4 × 1.5 × 0.667 = 4. El `Filter` multiplica por la selectividad de `f:Person ∧ f.age < 40`: la etiqueta ya está declarada por el patrón (1.0) y el rango usa SEL_RANGE (1/3): 4 × 1/3 = 1.33, que al mostrarse se redondea a 1. Verificación: `estimacion_scan_filter_expand` calcula estas mismas cifras contra el plan real.

**2. ¿Por qué en la canónica del brief (`WHERE p.name = "Ana"`) el átomo `f:Person` NO baja hasta el escaneo, si el pushdown baja predicados?**

Porque el pushdown baja átomos, no árdenes: `f:Person` menciona a `f`, y en el plan tras R1 `f` la liga el `Expand` — no hay NINGÚN sitio por debajo donde `f` ya esté ligada. La frontera del `Expand` es exactamente eso: lo que menciona variables ligadas por debajo baja; lo que menciona `to` se queda arriba. El plan queda `Filter(f:Person)` sobre `Expand(p, KNOWS, OUTGOING, f)` sobre `IndexSeek(Person.name = "Ana")` — verificado por `pushdown_canonico_del_brief_con_index_seek`.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin ejecutar nada, predice el plan DESPUÉS de `MATCH (a:Person), (c:City) WHERE a.age > 35 AND c.name = "Madrid" RETURN a.name, c.name`: cómo se parte el AND, dónde acaba cada átomo, cuál se convierte en `IndexSeek` y por qué el otro no. Verifícate con `cargo run -q -p liradb-cli -- explain "..."` y con el test `pushdown_reparte_el_cartesiano_y_busca_indices`. *Pistas*: (1) ¿qué variables liga cada lado del cartesiano? (2) ¿qué forma exacta exige R4? (3) ¿cuántos "Madrid" hay bajo City? *Criterio*: acertar la partición del AND y el IndexSeek exacto.

**Intermedio (analizar).** La raíz del §21.9 estima 1 y devuelve 3. (a) Calcula la selectividad real de `age < 40` sobre las Person del fixture y compara con SEL_RANGE. (b) ¿Podría esa discrepancia llegar a cambiar el plan elegido por R1? Construye un WHERE donde sí (pista: filtros a ambos lados con heurísticas que mientan en direcciones opuestas). (c) ¿Qué estadística mínima —sin histogramas completos— afinaría el rango? Verifícate con `explain_la_consulta_del_reordenado` y `estimacion_scan_filter_expand`. *Criterio*: separar «ordenar planes» de «acertar filas».

**Experto (crear — cierre de Parte IV, retrieval puro).** Primera parte, de memoria (sin mirar los caps. 17-20): reconstruye la cadena `texto → parse → lower → optimize → compile → open/next/close → filas` y escribe, para cada eslabón, qué capa la añade y qué invariante garantiza. Segunda parte: extiende `Catalog` con min/max por (etiqueta, propiedad) — acumúlalos en `collect` — y úsalos en `compare_selectivity` para estimar rangos por interpolación ((max − x)/(max − min)) en lugar de 1/3. Comprueba que el explain del §21.9 pasa de `est. 1` a algo cercano a 3 en la raíz y que el PLAN elegido no cambia (R1 sigue empezando por f). *Pistas*: (1) ¿dónde del bucle de nodos tocaría acumular el par?, (2) ¿qué rama del `match op` es la de los rangos?, (3) ¿por qué el plan ganador no debe moverse? *Criterio*: estimación mejorada + mismo plan + la batería de equivalencia (`equivalencia_antes_y_despues_sobre_bateria_de_consultas`) sigue verde.

## Para profundizar

- **P. G. Selinger et al., «Access Path Selection in a Relational Database Management System» (SIGMOD 1979)** — el paper fundacional: selectividades por defecto, dinámica de joins, y el pushdown como heurística. Nuestros 0.1 y 1/3 salen de aquí.
- **PostgreSQL, «Using EXPLAIN» (docs oficiales) y release notes 7.2.0** — el formato plan + rows estimadas + actual rows que hemos imitado.
- **Alex Petrov, «Database Internals» (O'Reilly, 2019)** — capítulo de ejecución y optimización como pieza del motor completo.
- **CMU 15-445, lecciones de query optimization** — la taxonomía heurística vs coste, con Cascades como horizonte.
- **Armbrust et al., «Spark SQL: Relational Data Processing in Spark» (SIGMOD 2015)** — Catalyst: reglas + estrategias sobre árboles de planes, en producción a escala enorme.

## Mini-diálogo: en la boca del túnel

> — Entonces el optimizador es… ¿un montón de ifs que cambian mi plan?
>
> — Es un montón de ifs con TASTE. Cada uno encarna una decisión que alguien tomó mirando datos: qué lado es más barato, qué predicado baja, qué índice ahorra. Selinger los escribió en 1979 y siguen ahí.
>
> — ¿Y si se equivoca? Mi estimación decía 1 y eran 3.
>
> — Se equivocó en la cifra y acertó en la decisión: f seguía siendo el mejor punto de partida. Las estimaciones no tienen que acertar; tienen que ORDENAR bien. El día que ordenen mal, no cambias el plan: cambias las estadísticas.
>
> — ¿Y nada puede romper mis resultados?
>
> — Todo puede romper tus resultados. Por eso no confiamos en que no: lo testeamos. Doce consultas, dos caminos, mismas filas. El optimizador no pide fe; pide evidence.

---

*(Próximo capítulo: 22 — Caminos mínimos ponderados. El optimizador eligió la ruta más barata para LIGAR un patrón; ahora la pregunta cambia: ¿cuál es el camino más corto de Ana a Carla cuando las aristas pesan? Dijkstra entra en escena — y abre la Parte V.)*
