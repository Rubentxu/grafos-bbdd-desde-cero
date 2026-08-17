# Capítulo 5 — Profundidad, ciclos y componentes (DFS, componentes conexos, ordenación topológica, SCC)

> *«Un mapa no responde preguntas por sí solo. Pero casi todas las preguntas que se le hacen a un mapa se pueden escribir de cuatro maneras distintas.»*

## 5.0 La anécdota de la esquina

En 1979, William Feldman llevaba meses viendo cómo un mismo dolor atacaba a los programadores de los laboratorios Bell: **compilar un programa grande significaba compilarlo todo, y saber qué exactamente había que recompilar era un lío de proporciones épicas**. De aquel dolor nació `make`, la herramienta que tomaba un fichero de dependencias —"el paquete `db.o` necesita `db.c` y `types.h`; `db.c` necesita `types.h`"— y decidía el **orden** en el que había que ejecutar los pasos.

`make` hacía algo muy concreto y muy elegante: si cada tarea solo puede arrancar cuando sus dependencias terminaron, el orden de ejecución debe cumplir que **toda dependencia aparezca antes que quien depende de ella**. Ese orden tiene nombre: **ordenación topológica** — y un algoritmo de 1962 (el de Arthur Kahn) lo calcula en tiempo lineal.

Y está la pesadilla que todo usuario de `make`, `dpkg`, `cargo` o `npm` conoce en carne propia: la **dependencia circular**. `A` necesita `B`, `B` necesita `C`, y `C` necesita `A`. El sistema no puede decidir cuál va primero: está atrapado en un **ciclo**. El instalador lo escupe con un mensaje amargo — "dependency cycle" — y se rinde.

Ese mensaje de error es una de las ideas más importantes de toda la ciencia de grafos: **detectar un ciclo, precisamente cuando y solo cuando existe**, es un problema resoluble de forma elegante y exacta. Y del otro lado del mismo concepto se levantan las **componentes fuertemente conexas** (SCC): los grupos de paquetes condenados a depender unos de otros en círculo, que un gestor tiene que resolver o romper.

Este capítulo cierra la Parte I del camino. El Volumen I ya te enseñó estos algoritmos; aquí los vas a **reencuadrar**: DFS, componentes conexas, ordenación topológica y SCC no son cuatro ejercicios sueltos del Vol.I, sino **las cuatro preguntas estructurales que un motor de base de datos de grafos debe saber responder**. Son las consultas que LiraDB calculará cuando la construyas.

## 5.1 Objetivo

Al terminar este capítulo serás capaz de distinguir —sin dudarlo— **cuatro preguntas distintas que se le hacen a un mismo grafo**, y de ejecutar a mano la respuesta a cada una:

1. **¿Cuántos trozos tiene el grafo y quién está con quién?** — *componentes conexas* (no importa la dirección).
2. **¿Hay un ciclo?** — la *detección de ciclos*, que nace del *DFS* y de la arista hacia un vértice gris.
3. **¿Qué orden de tareas respeta todas las dependencias?** — la *ordenación topológica* de un DAG.
4. **¿Qué grupos irreductibles de dependencia mutua existen?** — las *componentes fuertemente conexas* (SCC) de un grafo dirigido.

Y, más importante aún, entenderás por qué una **base de datos de grafos** tiene que responder estas preguntas como parte de su esencia — no como aplicaciones externas, sino como las **consultas estructurales** que hacen del almacenamiento un sistema de verdad. Este es el último capítulo de la Parte I; el cap. 6 abrirá la Parte II preguntándose qué convierte un grafo en una base de datos.

## 5.2 Problema

Cuando un usuario de una base de datos de grafos pregunta, no pregunta "ejecuta mi algoritmo". Pregunta cosas de negocio — y muchas de ellas son, en el fondo, **preguntas estructurales sobre el grafo**:

- ¿Este dispositivo alcanza a aquel servidor por *alguna* ruta de cables? → **conectividad**.
- ¿Mi red eléctrica tiene un lazo que provocaría sobrecarga? → **detección de ciclos**.
- ¿En qué orden tengo que cargar estas tablas para que cada dependencia exista antes de que se use? → **ordenación topológica**.
- ¿Qué módulos forman un grupo donde todos dependen de todos, y que por tanto deben desplegarse juntos o no desplegarse? → **SCC**.

El problema es que durante mucho tiempo, en muchos sistemas, estas respuestas se buscaban *después*, por fuerza bruta: ¿conectado? → prueba todas las rutas. ¿Cíclico? → prueba todos los órdenes. Cada una de esas búsquedas es exponencial o francamente inabordable.

El cambio de paradigma —que el Vol.I ya te mostró y que aquí vamos a anclar— es que **cada una de estas preguntas tiene una respuesta en tiempo lineal, O(V+E)**, con un recorrido bien pensado. No hay que adivinar: hay que *recorrer*. Y un motor de base de datos los calcula igual que una base relacional calcula un índice: porque son **derivables del grafo** y **baratos de mantener encima de un log de mutaciones**. La Parte V (caps. 22-26) los ejecutará sobre datos persistentes; aquí entendemos el *qué* antes del *cómo se persiste*.

## 5.3 Modelo mental

El modelo mental está en el propio título: **una sola figura de mapa, mirada cuatro veces, respondiendo cuatro preguntas**. Cogemos un mismo grafo —nueve nodos, con un ciclo escondido— y le ponemos encima cada pregunta:

```
GRAFO BASE (9 nodos, DIRIGIDO):             (a) DFS: árbol de profundidad
                                              + arista de retroceso en ROJO
   0 ──→ 1 ──→ 2                                 0 ──→ 1 ──→ 2
   │      │      ▲                                ──╮ │
   │      ▼      │                                  │ 3        ... del ciclo:
   │      3 ──→ 4 ──→ 5                             │ ▼       6 ──(retroceso)→ 1
   │      │      │   │                              │ 4
   │      ▼      ▼   ▼                              │ │  5
   │      6      7   8──(otra isla)                 ▼ │  7
   └── ciclo: 1→3→4→6→1                             ╰►6
```

Las cuatro miradas sobre el MISMO esqueleto de nueve nodos:

```
(b) COMPONENTES CONEXAS (vista NO dirigida):      (c) DAG de tareas → ORDEN topológico:
    {0,1,2,3,4,5,6,7}   y   {8}                     0→1→3→4→6→2→5→7→8
    → 2 "islas": dentro de la grande,                  (cada flecha va de antes a después)
      todo se alcanza sin mirar la dirección

(d) SCC (dirigida): colapsa el ciclo                  ⓐ = {1,3,4,6} (un SCC)
    {1,3,4,6} en el supernodo ⓐ:                      0→ⓐ → 2 → 5 →7→8
    0→ⓐ · ⓐ→2 · ⓐ→5 · 5→8 · 6→7 ...
    ¡El resultado es SIEMPRE un DAG!                   → este DAG se ordena con Kahn
```

El **momento ¡ajá!**: cuando ves la vista (d) todo encaja. Un grafo dirigido, por cíclico que parezca, se reduce **siempre** a un DAG colapsando cada SCC en un supernodo. De ahí que (d) se apoye en (c): **la ordenación topológica del grafo condensado te dice en qué orden deshacer o desplegar los ciclos**. Las cuatro preguntas no son islas: son capas del mismo mapa, y juntas hacen de un montón de bytes una base de datos de grafos.

## 5.4 Primera solución: el DFS y el ciclo

Empezamos por el que nos da la lupa: el **DFS** (búsqueda en profundidad). Donde BFS recorre por capas con una cola —y por eso *minimiza pasos* (Vol. I cap. 4)—, el DFS baja hasta el fondo con una pila, o con recursión, que es la misma pila: recorre cada rama hasta agotarla antes de retroceder.

La diferencia crítica respecto a BFS es que el DFS *estructura el recorrido*. Para hacer esa estructura visible, los libros clásicos (Cormen y compañía, CLRS cap. 22) usan **tres colores**:

- **blanco**: no se ha visitado;
- **gris**: está en la pila — se está explorando una rama que pasa por él, aún no cerrada;
- **negro**: su rama terminó, ya se cerró.

En la figura (a) recorremos empezando por 0: descubrimos 1 (gris), de ahí 3 (gris), de ahí 4 (gris), de ahí 6 (gris)… y 6 tiene una arista hacia **1, que ya es gris**. Eso es el detector de ciclos.

### ¿Cómo detecta DFS un ciclo?

Esta es la pregunta crítica del capítulo, y merece una regla de oro:

> **Una arista que alcanza un vértice GRIS es una arista de retroceso, y toda arista de retroceso pertenece a un ciclo.**

¿Por qué? Porque si estás explorando la rama de `1` y llegas a `6`, y `6` apunta de vuelta a `1`, entonces dentro del recorrido actual hay un camino `1→…→6` y la arista `6→1` lo cierra: es un ciclo. No importa volver al *nodo raíz* del recorrido (aquí, 0): el ciclo `1→3→4→6→1` no pasa por 0. Lo que importa es volver a un nodo **que siga en la pila** —un gris— porque eso significa "estás dentro de mi rama, y volverse a uno mismo dentro de la rama es dar una vuelta entera".

*(Nota de oficio: una arista hacia un vértice NEGRO —ya cerrado— NO es un ciclo. Es simplemente un puente entre una rama terminada y la actual; al cerrar el negro, ya no estás "volviendo a uno mismo", estás tendiendo un puente a algo ya calculado.)*

### La primera versión ingenua

Un novato, al "detectar" un ciclo, suele hacer esto:

```
marcar visitado (solo blanco/negro, SIN gris)
DFS(v): marcar v visitado
    para cada vecino w de v:
        si w no está visitado: DFS(w)
```

Sin el gris, este código **no ve ningún ciclo**. Puede dar la vuelta completa del ciclo `1→3→4→6→1`, marcar 1, 3, 4, 6 de negro uno tras otro, y al llegar a la arista `6→1` encontrarse el 1 ya negro… y no encender ninguna alarma. El grafo más simple del mundo con un ciclo que no pasa por la raíz **rompe este detector en silencio**. Por eso el gris no es un lujo: es lo que convierte "recorrer" en "detectar ciclos".

## 5.5 Sus límites

El DFS es potente, pero solo detecta ciclos — y confunde si se usa para lo que no toca:

1. **No muestra los "trozos"**: si corres UN solo DFS partiendo de 0, visitarás todo lo alcanzable desde 0; el nodo 8, si no es alcanzable, se queda intacto, pero un solo DFS no te numera las piezas.
2. **No diferencias direcciones bien**: en un grafo dirigido, un DFS desde 0 no "ve" que 8 es otra isla aunque 8 esté conectado mirando el grafo sin flechas.
3. **El orden de los vecinos cambia el árbol**: si exploras las aristas en otro orden, el árbol de profundidad cambia (la *existencia* de un ciclo no, por fortuna — pero el *árbol* sí).
4. **No da la orden**: el DFS, aunque recorras todo, no te dice qué tarea ejecutar antes en un DAG. Necesita el truco de Kahn.

Para responder "¿cuántos trozos?" basta un truco que se monta encima de cualquiera de los dos recorridos: **arranca un nuevo recorrido cada vez que quede un nodo blanco, contando un arranque como una componente**.

## 5.6 Solución evolucionada (a): componentes conexas

La pieza 1 es trivial encima de DFS *o* BFS:

```
componentes_conexas(grafo NO dirigido, o vista simétrica del dirigido):
    num = 0
    para cada nodo s en orden:
        si s ya está etiquetado: sigue
        num += 1                       # nuevo arranque = nueva componente
        recorre desde s (BFS con cola o DFS con pila)
            etiquetando cada nodo alcanzable con num
```

Cada arranque encuentra una **componente conexa**: un conjunto donde desde cualquier punto llegas a cualquier otro *sin preocuparte por la dirección de las flechas*. En el diagrama (b), la vista no dirigida de nuestros nueve nodos produce dos componentes: la grande `{0..7}` y el nodo suelto `{8}`.

Aquí está la pieza de anclaje con el código **real** del proyecto. En

```
liradb-workspace/crates/vol2-liradb/src/cap25_comunidades.rs
```

hay una función llamado justo como lo que estamos describiendo, `componentes_conexas`. Su esqueleto es este:

```rust
pub fn componentes_conexas(store: &dyn GraphStore) -> Result<ComponentesResult, ComunidadesError> {
    let g = GrafoPonderado::proyectar(store, &WeightSource::Constant(1.0))?;
    let n = g.len();
    let mut componente = vec![usize::MAX; n];
    let mut num = 0usize;
    for s in 0..n {                        // barrido por índice: numeración por menor miembro
        if componente[s] != usize::MAX { continue; }
        let mut cola: VecDeque<usize> = VecDeque::new();
        componente[s] = num;
        cola.push_back(s);
        while let Some(v) = cola.pop_front() {
            for &(w, _) in &g.vecinos[v] {
                if componente[w] == usize::MAX {
                    componente[w] = num;
                    cola.push_back(w);
                }
            }
        }
        num += 1;
    }
    // -> ComponentesResult { particion, stats }
}
```

*(He cortado la función a su esqueleto; el código completo vive en el cap. 25. Dos detalles que ya deberías reconocer del capítulo de BFS (cap. 2 o Vol. I cap. 4): el `VecDeque` como cola y el barrido por índice que numera las componentes por su menor miembro — un patrón de determinismo que un motor de BD exige.)*

Y el detalle que es el espejo exacto de lo que estás aprendiendo: **el doc-comment de esa función advierte que "las componentes FUERTEMENTE conexas (dirección respetada) son OTRO algoritmo (Tarjan, Vol. I cap. 7) y otra pregunta"**. Cuando leas ese comentario en el cap. 25, lo reconocerás al instante: nosotros aquí lo estamos cimentando.

**Componente conexa ≠ SCC.** La componente conexa te dice "hay un cable entre estos dos" — en ambos sentidos, porque miramos el grafo *sin* flechas. La SCC te dice algo más exigente y dirigido.

## 5.7 Solución evolucionada (b): ordenación topológica (Kahn)

Cambiemos de pregunta. Ahora el grafo es un **DAG** — un grafo dirigido acíclico: un conjunto de tareas donde cada arista "debe hacerse antes que". ¿En qué orden ejecutas todas las tareas para que ninguna dependencia se ejecute antes de sus antiguos?

La receta de **Arthur Kahn (1962)** es la más transparente:

```
orden = []
cola = todos los nodos con GRADO DE ENTRADA 0   (nada los necesita antes)
mientras cola no esté vacía:
    v = sácalo de la cola
    orden.push(v)
    para cada w tal que hay arista v→w:
        entrada[w] -= 1
        si entrada[w] == 0: cola.push(w)
si orden.len() < nº de nodos:  Hay un CICLO
```

Funciona así: siempre hay al menos una tarea sin pendientes → hazla y "retírala del tablero", reduciendo la deuda de sus dependientes. Si al final procesaste todos los nodos, obtienes un orden donde **cada flecha va de antes a después**: ese es el orden topológico. Si quedan nodos sin procesar, es que **nunca hubo un nodo con grado de entrada 0 al final**: hay un ciclo alimentándose a sí mismo, y la señal exacta es "quedamos con vértices sin procesar". En el diagrama (c), el orden `0→1→3→4→6→2→5→7→8` es uno válido para el DAG de tareas.

Es exactamente el mecanismo de `make`, `dpkg`, `cargo` y `npm`: el "orden de ejecución" respeta dependencias, y cuando aparece un ciclo, **el detector ES la propia cola que se queda corta** — no un error adivinado, sino el resultado exacto del algoritmo. Ese "si sobran vértices, hay un ciclo" es la misma regla de oro en su versión topológica: un DAG *siempre* tiene al menos un nodo sin dependencias; si en algún momento no lo hay, te estás moviendo sobre un ciclo.

## 5.8 Solución evolucionada (c): SCC — Kosaraju y Tarjan

Última pregunta, la más exigente. En un grafo **dirigido**, define las **componentes fuertemente conexas** (SCC): los conjuntos donde, *siguiendo las flechas*, de cada vértice llegas a cada otro. En la figura (d), el ciclo `1→3→4→6→1` es un SCC: cada nodo del grupo alcanza a cada otro con flechas a favor. El nodo `8` es su propio SCC de un solo vértice (no se alcanza a sí mismo salvo con un camino trivial).

La idea decisiva es la **condensación**: colapsa cada SCC en un **supernodo**. El resultado **es siempre un DAG** — el *grafo de condensación*. ¿Por qué no puede haber un ciclo entre supernodos? Porque si existiera, todas sus flechas se cerrarían, sus nodos se alcanzarían entre sí siguiendo las flechas, y por definición serían… el MISMO SCC. Los supernodos, unidos entre sí, forman un DAG, y ese DAG se ordena topológicamente con Kahn. Por eso SCC y topológica son dos caras de la misma moneda.

Hay dos algoritmos históricos para calcular los SCC, y la pregunta crítica del capítulo (guardada en el `CORPUS.yml` del proyecto) es **"¿cuándo Kosaraju y cuándo Tarjan?"**. Respondámosla con precisión:

- **Kosaraju (1978)**: **dos pasadas**. Primera: un DFS que anota el *orden de finalización* de cada nodo. Segunda: un DFS sobre el **grafo transpuesto** (todas las flechas invertidas), procesando los nodos en orden de finalización decreciente. Cada árbol de esa segunda pasada es un SCC. Es **sencillísimo de explicar y de demostrar**, y por eso es el favorito para escribir correctamente a la primera. Coste: dos recorridos — **lee las aristas dos veces**, una sobre el grafo y otra sobre el transpuesto.
- **Tarjan (1972)**: **una sola pasada**, con un índice extra llamado **`lowlink`** — el vértice de menor índice de descubrimiento alcanzable desde la pila actual. Cuando el `lowlink` de un vértice coincide con su propio índice de descubrimiento, ese vértice es la raíz de un SCC terminado, y todo lo que esté por encima en la pila se expulsa como un SCC. Coste: **una sola pasada**, sin transpuesta — más rápido en la práctica y sin aristas leídas de más.

**En código real, ¿cuándo cada uno?** La regla que el oficio ha consolidado:

> Usa **Kosaraju** cuando la claridad y la baja probabilidad de bug importen más que el rendimiento — grafos medianos, didácticos, o donde la doble lectura de aristas sea despreciable. Usa **Tarjan** cuando el grafo sea ENORME y leer las aristas dos veces sea el cuello de botella real, o cuando no puedas materializar el grafo transpuesto entero. Tarjan es el que elige producción exigente; Kosaraju el que elige la cabeza clara de un aprendiz.

No hay un "bueno y malo": hay un **trade-off entre simplicidad/demostración y una sola pasada/velocidad**. Tenerlo presente te permite decidir con criterio, que es justo lo que la pregunta exige. (Este es también el punto en el que la historia se vuelve curiosa: el algoritmo de Kosaraju se describió en una nota privada de diciembre de 1978 y solo se divulgó en el libro de Aho-Hopcroft-Ullman de 1983 — casi una década después. Una idea bellísima que tardó años en aparecer en un texto porque su autor no la publicó de inmediato.)

## 5.9 Los porqués (grill con el oficio)

Para cada decisión de estas cuatro respuestas, la pregunta de fondo es "¿por qué así y no de otra forma?". Las seis decisiones que este capítulo te pide interiorizar:

**1. ¿Por qué cuatro conceptos separados y no un bloque "recorridos"?**
Porque son cuatro preguntas de negocio distintas. Mezclarlos es la misconcepción número uno del tema; separarlos es lo que evita que confundas "¿está todo conectado?" con "¿puedo ir de A a B siguiendo las flechas?". El ancla del código real lo confirma: `componentes_conexas` usa la vista **simétrica**, porque la pregunta de conectividad ignora la dirección.

**2. ¿Por qué tres colores y no "visitado/sin visitar"?**
Porque sin el gris no se detectan ciclos. El gris es "estoy en la pila"; una arista a un gris es una vuelta dentro de la rama. Con solo blanco/negro, el ciclo `1→3→4→6→1` pasaría desapercibido. Alternativa descartada: recorrer y volver a mirar — no sale a cuenta ni es exacto. Modo de fallo si no lo haces: respondes "no hay ciclos" cuando sí los hay.

**3. ¿Por qué la vista simétrica para componentes conexas (y qué pasaría si usara la dirigida)?**
Porque "hay un cable" no depende de la dirección de la flecha. Si usaras SCC para esa pregunta, dos nodos unidos por UNA sola flecha no estarían en la misma componente, y una red físicamente conectada parecería rota. El doc-comment del cap. 25 distingue explícitamente las dos preguntas.

**4. ¿Por qué Kahn y no probar todos los órdenes?**
Probar todos los órdenes es O(n!), inabordable. Kahn es O(V+E) y, de regalo, su "cola que se vacía antes de tiempo" es exactamente el detector de ciclos. Alternativa descartada: topológica por DFS inverso — válida, pero menos intuitiva para la analogía de tareas y dependencias que hace `make`.

**5. ¿Por qué colapsar SCCs siempre da un DAG?**
Porque si hubiera un ciclo entre supernodos, serían el mismo SCC por definición de alcanzabilidad mutua. Esta propiedad es la que une SCC con topológica: el grafo de condensación siempre se puede ordenar.

**6. ¿Por qué esto son "consultas del motor" y no "aplicaciones"?**
Porque una BD de grafos las calcula como **índices estructurales derivados**: O(V+E), cacheables y invalidables al mutar el grafo — exactamente el patrón de un índice en una BD relacional. Lo verás aplicado en la Parte V.

## 5.10 Cómo lo hace una BBDD real (y qué hemos sacrificado)

Este capítulo fue conceptual a propósito — es de la Parte I, antes de persistencia y consultas. Lo que aprendiste aquí no se implementa todavía en LiraDB, pero ES el contrato que la Parte V cumplirá. Las cuatro respuestas vuelven —sobre **datos persistentes**— en los capítulos de esa parte:

- **Caminos, conectividad y centralidad** → caps. 22, 24 (Dijkstra, PageRank).
- **Componentes conexas y agrupaciones** → cap. 25 (con `componentes_conexas` como punto de partida de las comunidades, y su proyección simétrica ponderada).
- **Ejecutar estos recorridos sin agotar la memoria** cuando el grafo no cabe en RAM → cap. 26 (proyección, streaming, frontiers).

Y en producción, una BD de grafos comercial no recalcula a ciegas cada vez. Estas consultas se tratan como **índices estructurales derivados**: se calculan en O(V+E), se cachean mientras el grafo no muta, y se invalidan al insertar o borrar nodos y aristas. Ese mismo patrón —"derivar, calcular una vez, invalidar al mutar"— es el que verás en los índices reales (hash, B+) de la Parte III y en el modelo Volcano de la Parte IV.

**Qué hemos sacrificado** por hacerlo conceptual: no viste un solo test automático nuevo (tus comprobaciones aquí son a mano, sobre papel); no optimizaste ni un nanosegundo; y Tarjan y Kosaraju los presentamos en versión narrativa, no en su implementación íntegra. Lo que ganaste es lo más caro de recuperar luego: **no confundir los cuatro conceptos** y **saber por qué un motor los necesita**. Ese cimiento es el que hace que, cuando en el cap. 25 leas `componentes_conexas` con su doc-comment hablando de SCC "como otra pregunta", lo reconozcas al instante.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en `componentes_conexas`, ¿por qué el barrido numera las componentes por su menor miembro? ¿Qué pasaría si el orden de los nodos en el vecindario cambiara?
- *Intermedio*: explica con tus palabras por qué una arista hacia un vértice NEGRO no es un ciclo, pero una hacia un GRIS sí.
- *Experto*: decide —sin buscar— si para el grafo de 9 nodos de la figura usarías Kosaraju o Tarjan, y justifica qué perderías eligiendo el otro.

## 5.11 Lo que te llevas

- **DFS ≠ BFS**: BFS minimiza pasos con cola; DFS estructura el recorrido con una pila y tres colores.
- **Detectar un ciclo** = encontrar una **arista a un vértice gris** (una arista de retroceso). No es "volver al nodo inicial".
- **Componentes conexas** = partición del grafo en trozos donde todo es alcanzable (vista no dirigida o simétrica). En LiraDB, `componentes_conexas` del cap. 25 lo implementa con BFS.
- **Ordenación topológica** = en un DAG, un orden donde cada arista va de antes a después. Kahn (1962) lo hace con una cola de grado de entrada 0, y "si sobran vértices" es un ciclo.
- **SCC** = en un grafo dirigido, los trozos donde de cualquier vértice llegas a cualquier otro siguiendo las flechas. Colapsarlos **siempre** da un DAG. Kosaraju (1978) simple pero doble lectura; Tarjan (1972) una sola pasada con `lowlink`.
- **Son los índices estructurales del motor**: estas cuatro respuestas son las consultas de negocio que una BD de grafos debe poder responder en O(V+E).

## 5.12 Ojo, cuidado con…

- **SCC ≠ componente conexa**: la componente ignora la dirección; la SCC exige llegar *y* volver siguiendo las flechas. Detector: dos nodos unidos por SOLO `0→1` están en la misma componente conexa pero en SCC distintos.
- **Detectar ciclos "volviendo al origen"**: con el DFS correcto solo es un ciclo la arista que apunta a un vértice **gris**. Una arista a un negro ya cerrado no lo es.
- **Aplicar ordenación topológica a un grafo con ciclos**: la cola de Kahn se vacía dejando vértices sin procesar; "el orden que sobre" NO es topológico. Señal exacta: `orden.len() < nº de nodos`.
- **Decir "esto es del Vol.I, ya lo vi"**: lo viste como algoritmos de juguete; aquí son las consultas estructurales del motor. La Parte V los volverá a usar sobre datos persistidos.

**Precisión de lenguaje (glosario):** *componente conexa* (no dirigida) vs *SCC* (dirigida); *arista de árbol / retroceso / adelante / cruzada* (según el color del otro extremo); *DAG* (dirigido acíclico) vs *condensación* (el DAG de los SCC); *orden topológico* (cada flecha de antes a después) vs *orden de DFS* (orden de descubrimiento); *blanco / gris / negro* (sin visitar / en la pila / cerrado).

## 5.13 Pin de batalla

> *«No tienes que buscar el orden: tienes que recorrer el grafo. Quien recorre en profundidad ve los ciclos; quien cuenta los arranques ve los trozos; quien sigue el grado de entrada ve el orden; quien colapsa los ciclos ve el DAG que siempre estuvo ahí.»*

## 5.14 Si solo lees 30 segundos

A un mismo grafo se le hacen cuatro preguntas. Con el **DFS** (recorrido en profundidad que colorea blanco/gris/negro) detectas **ciclos**: una arista a un vértice gris es un ciclo. Contando cuántos arranques de recorrido haces sobre la vista **no dirigida** obtienes las **componentes conexas** (los trozos donde todo se alcanza). En un DAG, el algoritmo de **Kahn** (cola de grado de entrada 0) ordena las tareas respetando dependencias — y "si sobran vértices" es un ciclo. Y en un grafo dirigido, **Kosaraju** (simple) o **Tarjan** (una pasada) calculan las **SCC**, que al colapsarse siempre dan un DAG. Son las consultas estructurales de una base de datos de grafos.

## 5.15 Una historia pequeña

Cuando arrancamos a modelar la futura LiraDB como proyecto, lo primero que dibujamos sobre la pizarra no fue una página de disco ni un buffer pool: fue un grafo con nueve nodos y un ayudante preguntando, en voz alta, *qué exactamente quería saber cada futuro usuario*. A la mañana siguiente, el mismo grafo —sin cambiar un solo nodo ni una arista— ya había respondido cuatro cosas: "es un cable y una isla suelta", "hubo un lazo en el circuito de alimentación", "este es el orden de montaje", y "este trío de módulos no se puede desplegar por separado". Nadie había escrito una sola línea de almacenamiento. Pero todo el mundo en la sala entendió por fin **que esos algoritmos no eran un repaso del Volumen I: eran el vocabulario con el que le íbamos a preguntar a nuestra base de datos**. A partir de ese día dejamos de perseguir algoritmos sueltos y empezamos a perseguir preguntas.

## Ejercicios resueltos

**1. ¿Cómo detecta exactamente el DFS un ciclo?**
Cuando, al explorar una arista `v→w`, el vértice `w` está **gris** — todavía en la pila de recursión, parte de la rama actual. Decimos que `v→w` es una arista de retroceso, y esa arista cierra un ciclo formado por el camino de `w` a `v` más la propia flecha de vuelta. Una arista hacia un vértice **negro** (ya cerrado) no es un ciclo. Por eso el color gris es indispensable: sin él el detector no ve nada.

**2. ¿Por qué la vista de `componentes_conexas` del cap. 25 es SIMÉTRICA, y qué pasaría si usara la dirigida?**
Porque "componente conexa" pregunta por alcanzabilidad SIN dirección: si `0` envía a `1`, ambos están físicamente unidos. La vista simétrica lee cada arista en ambos sentidos. Si usara la dirigida (SCC), dos nodos unidos por una sola flecha no estarían en la misma componente aunque estén conectados por un cable — la red parecería rota cuando no lo está. El coste de la vista simétrica es que pierdes la información de flujo, que es justo lo que la SCC recupera.

**3. ¿Qué hace `make` cuando encuentra una dependencia circular?**
Con `make`/`dpkg`/`npm`, el orden topológico (Kahn) deja de progresar: en algún momento no hay ninguna tarea con grado de entrada 0, porque cada una espera a otra. `orden.len() < n` es la señal exacta de ciclo. El gestor no inventa un orden: **denuncia la circularidad** y se detiene, porque un orden que no respete dependencias no es un orden válido.

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Toma el grafo base de la figura 5.3 (nueve nodos, dirigido: `0→1`, `1→3`, `3→4`, `4→6`, `6→1`, `1→2`, `4→5`, `5→8`, `6→7`). Ejecuta a mano un DFS desde `0`, anota el **orden de descubrimiento** y de **finalización** de cada nodo, y marca la arista de retroceso que revela el ciclo. Después, olvida las flechas y numera las **componentes conexas** de la vista simétrica. Verifícalo contra los diagramas (a) y (b). *Pistas*: (1) un nodo es "retroceso" cuando reapareces sobre uno **gris**; (2) cada arranque de recorrido sobre un nodo sin visitar es UNA nueva componente; (3) el ciclo se cierra con una flecha hacia arriba en tu árbol. *Criterio*: la única arista de retroceso marcada es `6→1`, y hay 2 componentes conexas.

**Intermedio (analizar — interleaving topológica + mundo real).** Modela "montar una base de datos" como un DAG de tareas: `asignar páginas → escribir registros`; `reservar buffer → cargar página`; `crear catálogo → escribir índices`; `compilar → enlazar → ejecutar`; y que `crear catálogo → compilar` también sea una dependencia. (a) Aplica Kahn y escribe un orden topológico válido. (b) Añade la dependencia `ejecutar → compilar` de modo que se forme el ciclo `compilar → enlazar → ejecutar → compilar`; explica en una frase por qué ya NO hay orden y qué está diciendo la "cola que se queda corta". *Pistas*: (1) Kahn arranca de grado de entrada 0; (2) un ciclo deja ≥1 vértice sin procesar; (3) `make` lo reporta como "dependency cycle". *Criterio*: tu orden (a) respeta TODAS las flechas de antes→después, y tu diagnóstico (b) señala que la cola se vació sin procesar todo.

**Experto (crear — gancho a la Parte V).** Toma el grafo dirigido de la figura y reduce sus **SCC**: colapsa el ciclo `{1,3,4,6}` en un supernodo `ⓐ`, deja `{0}`, `{2}`, `{5}`, `{7}`, `{8}` como sus propios SCC, y dibuja el **grafo de condensación** con las flechas restantes. Demuestra que es un DAG y ordénalo topológicamente (Kahn). Luego decide —y justifica con la regla de §5.8— si para este grafo usarías Kosaraju o Tarjan. *Pistas*: (1) un SCC es un ciclo o un nodo aislado; (2) entre SCC no puede cerrarse un nuevo ciclo por definición; (3) el orden de los supernodos es la respuesta a "¿en qué orden desplegar?". *Criterio*: el grafo de condensación es acíclico, tu orden de supernodos respeta las flechas, y tu elección de algoritmo está justificada por claridad vs velocidad.

## Para profundizar

- **Cormen, Leiserson, Rivest, Stein — *Introduction to Algorithms* (CLRS), 3ª ed.** — el cap. 22 es la referencia canónica: DFS con los tres colores, la clasificación de aristas (de árbol / retroceso / adelante / cruzada), la detección de ciclos y el algoritmo de SCC de Tarjan completo.
- **Tarjan, R. E. — “Depth-first search and linear graph algorithms”, SIAM J. Computing 1(2):146-160 (1972)** — el paper original donde el DFS se usa para SCC y otras decisiones de grafos en una sola pasada.
- **Kahn, A. B. — “Topological sorting of large networks”, CACM 5(11):558-562 (1962)** — el algoritmo que da nombre a la ordenación por grado de entrada.
- **Aho, Hopcroft, Ullman — *Data Structures and Algorithms* (1983), §9.6** — donde se documentó formalmente el algoritmo de Kosaraju (descrito en una nota suya de diciembre de 1978); la historia de una idea que tardó años en publicarse.
- **Feldman, S. I. — “Make — A Program for Maintaining Computer Programs” (1979)** — el artículo de la anécdota: cómo `make` resolvió el orden de recompilación y popularizó la ordenación topológica y la denuncia de ciclos.

## Mini-diálogo: en guardia nocturna

> — O sea que, al final de todo, "componente conexa" y "SCC" son solo dos preguntas con dos respuestas distintas sobre el mismo dibujo.
>
> — Exacto. Componente conexa pregunta "¿hay cable entre A y B?" mirando el mapa sin flechas. SCC pregunta "¿puedo ir de A a B siguiendo las flechas y volver?" — mucho más exigente.
>
> — Y las otras dos —ciclo y orden—, ¿van por el mismo lado?
>
> — Son el reverso de la misma moneda y enganchan con la vida real. `make`, `dpkg`, `cargo` y `npm` viven a vueltas con la ordenación topológica: no ejecutan nada hasta que las dependencias dejan de bloquearse. Y el día en que una dependencia circular atasca la cola, ese "¡no hay orden!" es la respuesta exacta de Kahn, no un error adivinado.
>
> — Entonces, cuando construyamos LiraDB… ¿esto ya está resuelto?
>
> — Resuelto en tu cabeza. Y eso es precisamente lo que hace falta antes de tocar una página de disco: saber qué le vas a preguntar al mapa. Eso es lo que el cap. 6 va a estudiar —qué convierte un grafo en una base de datos, no en una biblioteca de algoritmos sueltos.

---

*(Próximo capítulo: 6 — Qué convierte un grafo en una base de datos. Aquí las preguntas estructurales respondían sobre un grafo en memoria; el cap. 6 abre la Parte II y se pregunta qué hace falta para que estos mismos conceptos pasen de ser los de una biblioteca de grafo a los de un sistema que los persiste y consulta.)*
