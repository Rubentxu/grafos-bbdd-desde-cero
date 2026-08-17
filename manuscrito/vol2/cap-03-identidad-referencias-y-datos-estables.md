# Capítulo 3 — Identidad, referencias y datos estables

> *«Un objeto no se cita por el lugar donde está. Se cita por quién es. El lugar cambia; el quién no.»*

## 3.0 La anécdota de la esquina

En los años sesenta, los fichadores de un autor de humor tecnológico — hacía décadas que guardaban sus chistes en **tarjetas perforadas** apiladas en una caja — cometieron un error que se hizo legendario en las oficinas de computación: reordenaron la pila para "quedar más ordenados" y, al hacerlo, **renumeraron las tarjetas desde 1**. De un día para otro, todas las fichas que apuntaban "la tarjeta 47 habla de Dijkstra" pasaron a estar en blanco o, peor, a apuntar a otra cosa — sin que nadie avisara. El contenido de cada tarjeta seguía siendo el mismo; lo que se rompieron fueron las *referencias*, porque estaban ancladas a la *posición física* en la pila, y la posición se movió. Lo más cruel del chiste es que las tarjetas no tenían nada de malo después de la reordenación — estaban exactamente igual de perforadas, con los mismos agujeros. Lo único que cambió fue el *número escrito a mano* en la esquina, y eso bastó para tirar abajo medio archivo. Cada referencia cruzada del libro — "la tarjeta 47 habla de...", "ver también la 12", "citada por la 88" — quedó apuntando al vacío o, peor, a una ficha sin parentesco ninguno con la original.

Ese chiste encierra la lección más profunda de este capítulo — y una a la que toda base de datos debe sobrevivir día sí y día también: **confundir la identidad con la posición es apostar a que nada cambie nunca de sitio.** En un sistema de ficheros, un descriptor de archivo cerrado y reasignado; en C, un puntero a un array redimensionado; en un grafo de base de datos, el `id` de un nodo. Todos son lo mismo cuando se quedan colgando: una promesa de "esto sigue siendo esto", rota por el simple acto de mover, borrar o reinsertar.

Este capítulo define qué hace que una identidad sea **estable**, por qué un índice de array no puede serlo, y esboza la solución que LiraDB ya tiene prometida desde las líneas 3-6 de su código ancla (`cap07_modelo.rs`): las **eras generacionales** (`(slot, generation)`), donde cada borrado incrementa la generación y un id viejo «apunta a nada» en lugar de apuntar al dato equivocado.

## 3.1 Objetivo

Al terminar este capítulo sabrás distinguir dos cosas que la mayoría de la gente confunde toda su vida: **IDENTIDAD** (el *quién* — un nombre que no cambia mientras el elemento exista) y **DATO** (el *qué* — el contenido, la posición, el valor, que sí puede cambiar). Y, con eso, entenderás por qué LiraDB no puede quedarse en `pub type NodeId = usize` como identificador definitivo y por qué el comentario honesto del `cap07_modelo.rs` («en el cap 3 se sustituirán por IDs generacionales (slotmap)») es la pieza que ahora vamos a colocar.

En concreto, terminarás sabiendo:

1. **Por qué un índice de array NO es un id**: borrar o reinsertar lo corrompe, o por corrimiento, o por reciclaje silencioso.
2. **Qué es un id estable**: un nombre que sobrevive a añadir, borrar y reordenar elementos.
3. **Qué lo hace posible**: la **era / generación** por slot — el id pasa a ser `(slot, generation)`.
4. **Qué es el problema ABA** y por qué es el enemigo jurado del reciclaje de ids.
5. **Por qué una BBDD elige claves surrogate** (inventadas, estables) sobre claves naturales (reales, mutables).

Este es un capítulo conceptual de la Parte I: no implementaremos un módulo de código nuevo, pero colocaremos la pieza que el `cap07_modelo.rs` dejó anunciada desde la primera línea de sus identificadores.

## 3.2 Problema

Retoma el minigrafo de Ana y Bo (cap. 1). Imagina que lo tienes en un simple array:

```
índices:    0       1       2
nodos:  [Ana]    [Bo]    [Carlos]
```

La arista "Ana conoce a Bo" se representa (en versión simple) como el par de índices `(0, 1)`. El `1` dice «esto, al final, apunta a Bo». Fácil, rápido, barato. Funciona... hasta que deja de funcionar. Ahora borra a `Bo`:

```
índices:    0    1(hueco)    2
nodos:  [Ana]    [   ]    [Carlos]
```

¿Qué haces con el hueco? Tienes dos malas opciones, y ambas rompen algo:

- **Opción A — corre el array a la izquierda**: `[Ana][Carlos]`, y ahora `Carlos` pasó del índice 2 al 1. La arista `(0, 1)` que decía «conozco a Bo» ahora dice «conozco a Carlos». Corrompida en silencio. Todas las referencias al índice ≥ 1 también.
- **Opción B — deja el hueco y rellena cuando puedas**: mañana insertas a `Dani`, y el «hueco 1» que quedó de `Bo` lo ocupa `Dani`. La arista `(0, 1)` que decía «conozco a Bo» ahora dice «conozco a Dani». Corrompida en silencio *otra vez*.

El problema de fondo es uno solo: **usaste una posición física como nombre, y la posición es volátil.** El índice `1` no es *Bo*; es *el lugar donde en este momento reside algo*. La identidad quedó atada al sitio, y el sitio se mueve.

Date cuenta de que esto es independiente del lenguaje, del sistema operativo y del tipo de grafo. Es la misma trampa que cae un array de C (`malloc` + `realloc` reubica el bloque y el viejo `&arr[i]` queda colgando), un `ArrayList` de Java (doblado por encima del umbral y los `Integer` cacheados de antes ahora apuntan al elemento equivocado), un descriptor de archivo de POSIX (cerrado y reasignado por otro `open()`), un `WeakReference` de Java/Python/Rust sin contraparte GC robusta. En todos los casos, lo que se corrompe no es el dato en sí — es la *promesa* de que «esta cosa sigue siendo la misma». Cuando esa promesa se ata a una *dirección*, la dirección se reasigna sin avisar; cuando se ata a un *nombre*, el nombre puede quedarse quieto incluso mientras el contenido se mueve. La elección entre atar la promesa a una dirección o a un nombre es la elección que divides este capítulo en dos.

Este es exactamente el problema del que LiraDB no puede escapar, porque una base de datos de grafos es *referencias por todas partes*: cada arista refiere a su `source` y su `target`, cada lista de adyacencia guarda los `EdgeId` de sus vecinos, el `trait GraphStore` del cap. 8 los expone, y el BFS del cap. 4 los recorrerá. Si esos ids fueran posiciones, un solo borrado envenenaría el grafo entero sin lanzar un solo error.

## 3.3 Modelo mental

Piensa en un **hospital con un archivo de expedientes** (retomamos la imagen del cap. 7, ahora llevada al extremo):

- En el archivo hay **estanterías con huecos**. Cada expediente ocupa una casilla (un **slot**). Si un expediente se retira, su casilla queda vacía — no se «corren» todos los demás para tapar el hueco, porque **renumerar expedientes rompería todos los enlaces** que dicen «el expediente 2 era el cardiólogo».
- El **número de expediente (el id)** es un *nombre*: lo puso la secretaría cuando el paciente entró y **no cambia nunca**, por mucho que el expediente se mueva de estantería, se vacíe una casilla vecina, o el paciente cambie de teléfono.
- El **teléfono es un dato (una clave natural)**: el paciente puede cambiarlo, y de hecho lo hará — pero cambiarlo NO debe hacer que «el expediente de Ana» apunte a otra persona.

Ahora el giro que resuelve el reciclaje. Una casilla que se vacía podría *volver a usarse* mañana para otro expediente. ¿Cómo distinguimos «este hueco 2 es el viejo expediente de Bo» de «este hueco 2 ahora es el de Dani»? Añadimos un **número de legajo (generación)** que sube cada vez que la casilla se vacía:

```
slot 1, generación 7   →  era el expediente de Bo (liberado)
slot 1, generación 8   →  ahora es el expediente de Dani
```

Visualmente, la estantería del archivo pasaría de algo perfectamente plano (un casillero por paciente) a un casillero donde cada hueco lleva, pintado al lado, un sello de cera con la fecha de su última vida:

```
              ┌──────── estantería generacional ─────────┐
              │                                           │
  slot 0      │  [ Ana,      legajo 3 ]    «ocupado»     │
  slot 1      │  [ ——,       legajo 8 ]    «hueco quemado»│ ← fue Bo
  slot 2      │  [ Carlos,   legajo 2 ]    «ocupado»     │
  slot 1'     │  [ Dani,     legajo 9 ]    «recién llegado»│ ← mismo slot físico, otro lógico
              │                                           │
              │  referencia vieja (slot 1, legajo 7): ya NO casa
              │  referencia nueva (slot 1, legajo 9): apunta a Dani
              └───────────────────────────────────────────┘
```

Una referencia que «recordaba» `(slot 1, generación 7)` sigue existiendo, pero ya NO coincide con `(1, 8)`: **apunta a nada** (aborto rápido) en vez de apuntar silenciosamente a Dani. El id real, en LiraDB, es la pareja **`(slot, generation)`** — no el slot desnudo.

```
                ┌────────────────────────  slotmap model ─────────────────────────┐
  Ingénuo:      Vec<Option<T>>  (tombstones del cap. 8)
    0: Some(Ana)   1: None      2: Some(Carlos)
                   ▲   hueco enterrado (NO re-numerado)
   arista 0→1 dice «conozco el slot 1»: HOY señala al vacío,
                                          MAÑANA a Dani. AMBIGUO.

  Con generación:  slots: Vec<(T, generation)>
    slot 0: (Ana,    3)
    slot 1: (None,   8)     ← fue liberado: la generación subió a 8
    slot 2: (Carlos, 2)
    slot 1: (None,   8)     → ahora inserto Dani: slot 1 pasa a (Dani, 9)
  Un id viejo (1, 7) compara 7 != 9 → «nada, abortar».
  Un id (1, 9) → «Dani».  INEQUÍVOCO.
```

Dos ideas son el corazón del capítulo:

1. **IDENTIDAD ≠ POSICIÓN.** El id es un nombre lógico, inmutable; el slot es un hueco físico, volátil. Separarlos es lo que permite que las referencias sobrevivan.
2. **GENERACIÓN ≠ DATO.** La generación no es un atributo del elemento; es una *marca de nacimiento del slot* que mide cuántas vidas ha tenido la casilla. Subirla al liberar impide que un id viejo «caiga» en un dato nuevo.

### El momento ¡ajá!

> *«Un índice de array es una DIRECCIÓN que se rompe al mover las cosas. Un id con generación es un NOMBRE que aguanta. La promesa de LiraDB — "un id no cambia mientras el elemento exista, y liberarlo no lo reasigna a otro" — se compra exactamente aquí: un contador por slot.»*

## 3.4 Primera solución

La solución ingenua que probablemente ya estás pensando es la del código ancla del cap. 7: **`pub type NodeId = usize`** — un id que ES el índice.

```rust
// Solución ingenua (la del cap. 7, correcta solo mientras no se borre).
pub type NodeId = usize;

let mut nodos: Vec<Node> = vec![ana, bo, carlos];
let quien_conozco = nodos[1]; // «Bo» — mientras el índice 1 sea Bo.
```

Es la más barata y legible que existe: **`nodos[i]` es O(1)**, no gasta memoria extra, y el `i` se lee solo. Es *perfecta* como puntal pedagógico — por eso el cap. 7 la usa. Su única condición es un supuesto que nadie dijo en voz alta: *nada se borra nunca, ni se reordena, ni se reinserta en un hueco*.

Si tu programa es de los que *leen y dibujan* (como los algoritmos del Vol.I), esta solución basta. Si es de los que *leen, escriben, borran y vuelven a escribir* (como cualquier base de datos de la historia), no basta — y por eso este capítulo existe. La trampa está en que el «supuesto no dicho» parece razonable al principio: «¿quién va a borrar un nodo, si los nodos son datos?» Mucha gente, resulta. Y basta un solo borrado para que todas las referencias implícitas a ese índice queden mintiendo en silencio. Cuando el cap. 7 escribe `pub type NodeId = usize;`, nos está diciendo exactamente esto: *esto es un andamio, ahora te toca subir a por la pieza de arriba*.

## 3.5 Sus límites

La solución se rompe en cuanto el grafo es *vivo* — es decir, en cuanto lo tratas como una base de datos y no como una estructura de juguete. Tres escenarios, tres formas de morir:

1. **El corrimiento.** Borrar `Bo` (índice 1) y «compactar» hace que `Carlos` pase a ocupar el índice 1. La arista que «recordaba» el 1 ahora saluda a Carlos. **Referencia que miente** — el peor modo de fallo, porque *parece* correcta.
2. **El reciclaje (bug ABA).** Dejar el hueco y rellenarlo después con `Dani` reutiliza el índice 1. La referencia vieja al 1 sigue siendo 1, pero ya es Dani. **Referencia que miente otra vez**, ahora sin que haya corrido nada: el hueco se rellenó.
3. **La dependencia del orden físico.** Cualquier reordenamiento — importar un fichero en otro orden, compactar en disco (cap. 16) — invalida todas las referencias si el id fuera la posición.

La raíz es una sola y ya la conoces: **el índice es una dirección, no un nombre.** Y una base de datos vive de nombres estables: un `Edge` refiere `source` y `target`; una lista de adyacencia refiere `EdgeId`; el BFS del cap. 4 guardará ids en su cola y en su conjunto de visitados. Si el id es un índice, cada borrado corrompe el conjunto entero de referencias *sin lanzar ningún error*.

Fíjate en lo aterrador del matiz: **no es que se caiga el sistema — es que no se entera nadie.** Una arista que apunta al nodo equivocado es un error de datos tolerable («¿por qué Carlos es amigo de Dani si nunca se vieron?»), no una señal de fallo. Esa «corrupción silenciosa» es exactamente lo que LiraDB no puede permitirse.

## 3.6 Solución evolucionada: las eras generacionales

La idea tiene nombre propio en el mundo Rust: **generational arenas**, y existe implementada, además de en LiraDB, en crates como `slotmap`. El modelo es frío y elegante — un `Vec` por arena con, al menos, dos datos por slot:

```rust
// La forma conceptual (no la API final del cap. 7):
struct Slot<T> {
    value: Option<T>,      // None = tombstone, Some = habitado
    generation: u32,       // cuántas veces se ha liberado este slot
}
struct Arena<T> {
    slots: Vec<Slot<T>>,   // NUNCA re-númeramos los slots
    free: Vec<usize>,      // cola de slots libres que se pueden reutilizar
}
// El id NO es el slot: es el par (slot, generation).
type Id = (usize, u32);
```

Las reglas que hacen todo el trabajo:

1. **Insertar**: toma un slot de `free` (o amplía el array), pon `value = Some(x)` y devuelve el id **`(slot, generación_actual)`**.
2. **Leer**: `buscar(id)` devuelve `Some` solo si `slots[slot].generation == id.generation` y el slot está `Some`. Si no, `None`.
3. **Borrar**: pon `value = None`, añade el slot a `free`, e **incrementa `generation`**. El id viejo queda «quemado»: ya nunca coincidirá.

El acoplamiento con el `MemoryStore` del cap. 8 es exacto: el `Vec<Option<Node>>` de la solución ingenua se convierte en un `Vec<(Option<Node>, u32)>`. Cada slot del array del cap. 8 *ya* lleva un `None` como tombstone; la generación es el campo adicional que lo hace seguro reciclarlo. Esa es la asimetría que distingue el cap. 3 del cap. 8: el cap. 8 entiende que borrar es `slots[id] = None`; el cap. 3 añade que, además, hay que *subir la generación* del slot. Es la única operación nueva — una asignación de un `u32` y un `push` a `free` — que compra toda la promesa de estabilidad.

El `slotmap` real de Rust lo expresa casi igual (módulo `slotmap::new_key_type` + `SlotMap`), y es lo que LiraDB importará cuando crezca: la misma forma `(slot, generation)` y, encima, optimizaciones que nosotros ni necesitamos — slots secundarios en páginas, freelists con hints de generación, etc. Para nuestro capítulo basta con la forma conceptual; la eficiencia es materia del Vol.II cuando la arena pase de Miles a Decenas de Millones de elementos.

Vuelve al ejemplo. Borras a `Bo` del slot 1: la generación de `1` pasa de 7 a 8. Insertas a `Dani»; ocupa el slot 1 con generación 9 (porque borrar lo subió a 8, e insertar sobre libre otra vez lo sube a 9 — el esquema exacto varía, lo constante es que *sube al reutilizar*). Ahora la referencia vieja que «recordaba» `(1, 7)`:

```
buscar((1, 7))  →  slots[1].generation == 9  ≠  7  →  None   («ya no existe, aborta»)
buscar((1, 9))  →  slots[1].generation == 9, value == Dani → Some(Dani)
```

El id viejo **no encuentra nada**; no cae sobre Dani. El error dejó de ser silencioso: se convierte en un `None` que el código puede manejar (el cap. 8 añade la costumbre de `get_* -> Option`, y el cap. 4 el BFS descubre que un visitante ya no existe). Y la arista `(0,1)` del ejemplo ingenua, tras el borrado, da `None` en vez de un falso «Carlos» o un falso «Dani».

Ese es el salto de calidad que este capítulo anuncia y que el código ancla promete: **`NodeId = usize` se convierte en `(slot, generation)`** — y como el cap. 7 guardó los alias `NodeId`/`EdgeId` en un único `type alias`, migrar tocará un punto, no cien.

## 3.7 Por qué así y no de otra forma

1. **¿Por qué `(slot, generation)` y no un UUID por nodo?** Porque LiraDB quiere ids *contiguos y O(1)*: el slot da el acceso directo al array; la generación añade una comparación de u32. Un UUID (16-28 bytes) sería estable pero *no adyacente*, y tendrías que buscarlo en un `HashMap` por cada acceso. Coste del UUID: memoria y hashing en cada lectura. Elegimos el par (slot, gen): casi gratis y estable.

   — Entonces, ¿cuándo usaría alguien un UUID? Cuando la identidad debe ser **global** (otra máquina puede nombrarla sin coordinación), lo que LiraDB no necesita en su núcleo. Más adelante (import/export, cap. 32) sí aparecerán *claves externas* — pero esas son *datos*, no identidad interna.

2. **¿Por qué no validar con un solo contador global de generaciones?** Porque acoplarías todos los slots a la misma era: bastaría que *un* slot se reciclara para que la «era mundial» suba y **invalide referencias perfectamente válidas a otros slots**. La generación tiene que ser POR slot, para que liberar una casilla no dañe a las demás. Es la diferencia entre aislar y tiranizar.

3. **¿Por qué el id nunca se re-reutiliza a sí mismo, aunque el slot sí?** Porque reutilizar el *slot físico* es deseable (ahorro de memoria: no hacerla crecer sin límite); lo que se prohíbe es reutilizar el *id lógico*. La generación es el contrato que nos permite quedarnos con la ventaja (compactación de slots) y descartar el peligro (id-reuse). Por eso el cap. 8 dirá: *«los tombstones (`None`) dejan huecos que nunca se reutilizan; en producción, los ids generacionales y los slots reciclables con número de generación lo resolverían»*.

4. **¿Por qué `u32` y no `u64` o `u16` para la generación?** Por la misma regla que ya conocemos: el contador tiene que ser lo bastante grande para no volver a cero mientras el slot viva, y lo bastante pequeño para no malgastar bytes. Un `u32` da 4 mil millones de reciclados por slot — a un ritmo de 1000 borrados por segundo, son 50 días de vida útil; a un ritmo humano (unos pocos borrados por minuto cuando el grafo está en reposo), son siglos. Un `u16` (65 535) se queda corto: una noche de pruebas intensivas lo desbordaría y haría fallar la comparación de generación sin motivo aparente. Un `u64` no aporta nada útil a cambio de 4 bytes extra por slot, que se vuelve molesto cuando el slot guarda enteros pequeños. **`u32` es el equilibrio clásico**, y es el que `slotmap` adoptó por defecto; LiraDB heredará esa elección.

5. **¿Por qué no almacenamos la generación como sello global?** Porque entonces todos los ids nacerían en un mismo instante y morirían en el mismo: cualquier borrado de cualquier elemento sube el reloj y obliga a invalidar todas las referencias pendientes. Una sola escritura remota mal sincronizada (p.ej. una arista de un WAL a medio loguear) cascadearía en un «apunta a nada» para todos los nodos del grafo. La generación **por slot** es la partícula mínima de contabilidad necesaria: cada hueco lleva su propio reloj, independiente, y aislar es la única forma de tener un sistema estable por construcción.

## 3.8 La trampa maestra: el problema ABA

Si algo debes llevarte de este capítulo, es el nombre y la cara del enemigo: el **problema ABA**, la forma canónica del id-reuse, conocido de sobra en programación concurrente (Herlihy et al., *The Art of Multiprocessor Programming*, cap. 10).

- **A** es un slot con valor.
- **B**: el valor se borra; el slot queda libre.
- **A** (otra vez): un valor nuevo ocupa el slot.

Si una referencia «vio» el slot durante B y «vuelve» después del segundo A, cree que está hablando con el mismo de antes — porque solo miraba el lugar, y el lugar devuelve un valor. Es el mismo nombre, otra cara. En un *lock-free stack*, el ABA hace que una operación *compare-and-swap* sobre «el mismo puntero» cambie un nodo reinsertado y corrompa la estructura; en un grafo, hace que una arista apunte al recién llegado.

La generación es el antídoto clásico: no comparas *solo* el slot (A→B→A parece el mismo), comparas **`(slot, generation)`**, y la generación cambió entre el primer A y el segundo. El «mismo» deja de ser el mismo. Esa es, palabra por palabra, la razón por la que se habla de «A-B-A» cuando se explica por qué los índices desnudos son traicioneros.

## 3.9 Claves surrogate vs naturales (o «por qué el email no es el id»)

Una vez separas **identidad de dato**, una decisión de diseño aparece sola: ¿qué uso como id, una clave natural (que es un dato) o una clave surrogate (que invento)? LiraDB elige **surrogate**: un id sintético, inmutable, sin significado intrínseco, generado por la arena.

La clave natural — el `email`, el `nombre`, el `DNI`, el valor que «se lee solo» — es un **dato**: mutable, cambiante, y no tuyo para congelar. El día que un usuario cambia su email, todas las referencias que usaban el email como clave de enlace (las aristas «amistad», las notas «quién lo conoce») se rompen, porque el dato dejó de ser lo que era. En una BBDD relacional, cambiar una clave natural foránea obliga a cascadas de actualización o deja huérfanos; en un grafo, huérfanos significa aristas que apuntan al vacío (Kleppmann, *Designing Data-Intensive Applications*, cap. 3, y la convención clásica del diseño relacional).

La clave surrogate — el id de la arena — vale por lo que NO dice: no significa nada del mundo real, y por eso **nada del mundo real puede cambiarla.** Un id no es «el email de Ana»; es «el expediente 0». Si Ana cambia de email, el `0` no se inmuta; la arista `(0, ...)` sigue refiriendo al mismo elemento, y solo actualizas el dato `props["email"]`. Esa es la sabiduría del capítulo: **cuando el id no puede cambiar, las referencias no pueden romperse por un dato mutable.**

Y no es un detalle teórico: si guardas `email` como clave única en una tabla relacional y tu usuario cambia de dirección, tu `UPDATE` debe recorrer TODAS las tablas donde `email` aparece como foreign key. Un día que se te olvide una, esa foreign key queda colgando de la dirección vieja — el equivalente exacto de nuestra arista que apunta al nodo equivocado. Las BBDD serias evitan esto desde hace décadas; LiraDB lo aplica al grafo desde la primera línea. Un buen test para saber si una columna es un id o un dato: **pregúntate «¿qué pasa si el usuario lo cambia?».** Si la respuesta es «algo se rompe», no era un id; era un dato. Si la respuesta es «solo se actualiza un valor», sí era un id.

## 3.10 Estabilidad después de un crash (la promesa que dejamos anclada)

El CORPUS pregunta por la **estabilidad de los IDs tras crash + recovery** — y esta es la promesa que este capítulo coloca y que los caps. 28-30 (WAL y recuperación) harán cierta. El ingrediente que aporta la generación es doble:

1. **El id no cambia mientras el elemento existe.** Por mucho que un `MemoryStore` se reabra, el slot y la generación de un elemento vivo se conservan; el id lógico sigue siendo el mismo. (En el disco, la cámara de bytes — caps. 9-11 — persistirá el par `(slot, generation)` junto al dato.)
2. **Un id liberado jamás se reasigna.** Tras un crash, podría darse la tentación de «renumerar desde cero» (más limpio, más compacto). LiraDB no lo hará: re-numerar invalidaría las referencias externas que alguien guardó antes del crash. Con generaciones, un id viejo sencillamente «ya no existe» — y eso es un estado *detectable*, no una corrupción.

No lo implementamos aquí — es material de la Parte VI. Pero conviene quedarse con la ecuación mental: **generación = la vacuna contra que un crash «reutilice» una identidad por accidente.** Cuando en el cap. 28 escribas el WAL y en el 29 recuperes un fichero, recordarás que cada id era un `(slot, generation)` y que re-numerar habría sido perder todo lo que este capítulo protege.

Y para que la promesa no quede en intenciones, vamos a firmarla en términos que cualquier capítulo posterior pueda verificar:

- El id se **escribe** en el WAL junto al `before` y `after` de la operación (cap. 28).
- El id se **recupera** del disco leyendo exactamente los mismos bytes (`slot`, `generation`, `value`) que estaban antes del fallo (cap. 29).
- El id se **compara** con la regla `slots[id.slot].generation == id.generation` cada vez que se intenta resolverlo — incluso después de un recovery, no solo durante la ejecución normal.
- Si una herramienta externa (un índice secundario, un fichero de exportación) guarda un id, ese id sigue siendo válido después de un crash + recovery, mientras el elemento siga vivo. La condición es: la generación nunca se reinicia globalmente; solo se incrementa, y solo por slot. Cualquier esquema que reinicie generaciones a 0 al arrancar está rompiendo la promesa que este capítulo firma.

## 3.11 Ojo, cuidado con…

1. **Tratar el índice como nombre estable.** El síntoma clásico es una variable `let j = i;` que «recuerda» a `i` y luego usa `grafo[j]` esperando el mismo de antes. Cuando algo se borra o se reordena, `j` ya no es lo que era. Pregúntate siempre: ¿este número es *a quién* (id) o *dónde* (posición)?
2. **Compactar y re-numerar para «quitar huecos».** Compactar el almacenamiento *físico* está bien (cap. 16) SI conservas la identidad lógica. Lo que no se puede hacer es «reordenar y renumerar los ids», porque reescribes todas las referencias a la vez. La compactación de LiraDB reordena; no re-numera.
3. **Usar una clave natural como id.** Cambias el email y rompes las amistades. La clave natural vive en `props`; el id es la clave surrogate.
4. **Reutilizar el slot sin generación.** Rellenar un hueco físico ahorra memoria; hacerlo *sin* subir la generación es exactamente el bug ABA. Quieres las dos cosas: slot reciclado, id quemado.
5. **Confundir «reutilizar el slot» con «reutilizar el id».** Reusar el slot sin quemar el id es la definición exacta de ABA. La generación es lo que separa el ahorro legítimo del bug silencioso.
6. **Pensar que el id puede «nacer antes de tiempo».** Un error sutil: asignar ids cuando un objeto aún no existe (por ejemplo, en una caché especulativa). El id debe entregarse en el mismo momento que el elemento entra en la arena; si lo entregas antes, te has comprometido con un slot que luego puede reciclarse y le has mentido a quien lo recibió. La regla de oro: **el id y el primer valor del slot nacen juntos** — `insert()` devuelve `(slot, generation)` *después* de poblar el slot; nunca antes.

- **Glosario**: **identidad** (el «quién» estable) vs **valor/dato** (el «qué» mutable); **índice** (posición física) vs **id** (nombre lógico); **slot** (casilla) vs **elemento** (lo que vive en ella); **tombstone** (`None`, hueco enterrado); **generación / era** (cuántas veces se liberó la casilla); **ABA problem**; **surrogate key / natural key**; **dangling pointer** (puntero que quedó colgando); **id-reuse**.

## 3.12 Lo que te llevas

- **Un id no es una posición.** El índice depende del orden físico y se corrompe al borrar, reordenar o reinsertar (corrimiento o reciclaje).
- **Un id ESTABLE es un nombre lógico** que no cambia mientras el elemento existe, y cuya liberación no lo reasigna a otro.
- **La era generacional** (`(slot, generation)`) compra esa estabilidad por menos de un contador por slot: cada borrado sube la generación, y un id viejo «apunta a nada» en vez de a un dato equivocado (antídoto del problema ABA).
- **Claves surrogate, no naturales:** el id no significa nada del mundo real para que el mundo real no pueda cambiarlo; el email va a `props`, no a la identidad.
- **Estabilidad tras crash = no renumerar nunca:** la generación es la garantía de que un crash no reutilice una identidad por accidente (confirmado en caps. 28-30).

## 3.13 Una historia pequeña

Cuando escribimos el `cap07_modelo.rs`, hicimos trampa con buena conciencia: `pub type NodeId = usize`, y un comentario honrado — *«se sustituirán por IDs generacionales (slotmap)»*. Al principio parecía suficiente. Ana escribió el primer test del `MemoryStore` (cap. 8): tres nodos, dos aristas, todo el mundo a su sitio. El test pasó a la primera. Luego escribió un segundo test, más realista: borrar un nodo y reinsertar otro en el hueco. Y al ejecutar el invariante — «la arista `0 → 1` debe seguir apuntando al nodo 1 original» — el test pasó también, porque casualmente el nodo reinsertado tenía los mismos datos que el borrado. La lección que nos dio ese «pasa-verde» no es que el código estuviera bien: es que **los tests con datos felices no descubren bugs de identidad**. Tuvimos que añadir `assert_ne!(vecinos_de_cero, [nuevo])` para forzar al código a fracasar. Fracasó. Y entonces entendimos que la identidad no es un campo que se rellena: es la **promesa** de que las referencias sigan siendo válidas para siempre — y esa promesa se compra con una generación por slot, no con datos coincidentes.

Es la misma razón por la que ningún test que solo verifique «las props del nodo son las que escribí» atrapará el bug ABA. La identidad se prueba con **referencias cruzadas**: «esta arista debe seguir apuntando al elemento que apuntaba antes; aunque ese elemento sea exactamente igual a otro, no es el mismo». Es un test incómodo de escribir, y por eso casi nadie lo hace — y por eso esta clase de bugs sobreviven hasta producción.

## 3.14 Ejercicios resueltos

**1. ¿Por qué una arista de un grafo es tan sensible al id-reuse?**

Porque una arista *es* una pareja de referencias: `(source, target)`. Si `target = 1` era «Bo» y, tras borrar y reinsertar, el `1` ahora es «Dani», la arista dice «Ana conoce a Dani» sin que ninguna línea de código se entere. No es un crash, no es un `None`: es la **corrupción silenciosa** más peligrosa, porque pasa las validaciones (el nodo existe) y solo falla el *semántico* (es el equivocado). Por eso el `MemoryStore` del cap. 8 usa tombstones (`None`) — sin re-numerar — y por eso LiraDB añadirá la generación para permitir reutilizar el slot físico sin reutilizar la identidad.

**2. ¿Qué cambia exactamente cuando borro `(slot, generation)` y por qué se dice que «las generaciones no reutilizan ids»?**

Al borrar, pongo `value = None`, añado el slot a `free` y **incremento `generation`**. Mañana, al reinsertar un elemento en ese slot, la generación ya **no es la de antes** (subió al liberar, y quizá de nuevo al ocupar). Cualquier id `(slot, gen_antigua)` que alguien conservara falla la comprobación `generation` y devuelve `None`. Así, el *slot* se reutiliza (ahorro de memoria) pero el *id lógico* jamás se recicla: el sistema es capaz de distinguir «la v4 de este slot» de «la v7». Esto es justo lo que el test de `buscar((1, 7))` del §3.6 demuestra con números.

## 3.15 Ejercicios propuestos

**Esencial (recordar — retrieval).** Sin mirar el capítulo, escribe de memoria la definición de *id estable* frente a *índice*, y el mecanismo generacional completo: qué es `(slot, generation)`, qué hace un borrado (borrar + incrementar generación) y qué devuelve un id viejo al buscar (nada). Criterio: tus tres afirmaciones cubren que (1) el id es un nombre inmutable, (2) un borrado sube la generación del slot, y (3) un id con generación vieja «apunta a nada» en vez de al dato recién insertado. Verifica contra §3.6 y §3.8.

**Intermedio (interleaving — referencias/punteros).** En C, `int *p = &arr[i];` y luego «compactar» o redimensionar `arr` deja a `p` **colgando** (dangling pointer): apunta a una dirección que ya no es `arr[i]`, o que ahora es otro elemento. Explica cómo la generación de LiraDB es el análogo seguro de ese puntero colgante: ¿qué sustituye a la «dirección de memoria» en el id `(slot, generation)`? ¿qué hace la comprobación de generación que el puntero de C no tiene? Pistas: (1) el puntero de C guarda solo la dirección; el id guarda *dos* números (slot y generación); (2) al compactar en C, `&arr[i]` ya no es válido pero C no lo sabe; en LiraDB, ¿qué «lo sabe»?; (3) ¿en qué se parecen «dirección reciclada» (C) a «slot reciclado sin generación» (el bug ABA)? Criterio: relacionas *dangling pointer* con *generación quemada* y explicas que ambos son el mismo problema de identidad estable.

**Experto (crear / analizar).** Simula en un snippet el doble escenario: (a) un `Arena` con `Vec<Option<T>>` SIN generación donde insertas, borras y reinsertas en el mismo slot — observa cómo una variable `id = slot` señala ahora al recién llegado; (b) el mismo `Arena` con `(T, generation)` — observa cómo `buscar((slot, gen_vieja))` devuelve `None` mientras `buscar((slot, gen_actual))` devuelve el dato. Criterio: tu test comprueba que sin generación el id antiguo «apunta al recién llegado» (ABA) y con generación «apunta a nada».

## Para profundizar

- **El crate `slotmap`** (Rust) — la implementación de referencia de *generational arenas*: lee su doc para ver la forma real `(slot, generation)` y cómo expone `get`, `insert`, `remove` y el contador de versiones. Es la semilla del futuro tipo de id de LiraDB.
- **Generational arenas en motores ECS** (p.ej. `bevy_ecs` / `legion`) — el mismo patrón usado para referencias estables a entidades en juegos; la literatura de *generational indexing* como forma de evitar el id-reuse en sistemas donde los objetos nacen y mueren por frame. La motivación es idéntica: cada sistema quiere que el id de una entidad siga siendo válido mientras esa entidad exista.
- **El problema ABA en concurrencia** (Herlihy, Shavit, *The Art of Multiprocessor Programming*, cap. 10) — la forma canónica del id-reuse y por qué los algoritmos sin bloqueos lo evitan con versiones (`versioned CAS`). Aunque nosotros no usamos concurrencia lock-free todavía, la misma idea de comparar *dirección + versión* es lo que aquí se traduce a *slot + generación*.
- **Surrogate vs natural keys en diseño de BBDD** (Kleppmann, *Designing Data-Intensive Applications*, cap. 3) — la regla clásica «el id no debe depender del dominio» y por qué las claves sintéticas estables (UUIDs, enteros autoincrement, hashes) son la costumbre en sistemas reales.
- **Database Internals** (Alex Petrov), caps. 2-3 — layouts de página y *generational storage*; explica por qué la idea de generación reaparece en *log-structured merge trees* y *copy-on-write B-trees*: a otro nivel (páginas en vez de slots), el problema es el mismo.
- **Weak references en sistemas GC** (patrones de Java `WeakReference`, CPython, Rust `Weak<T>`) — la otra cara de la moneda: referencias que *deliberadamente* quieren caducar. Un `WeakRef` en una arena generacional detecta exactamente la misma «caducidad» que el `None` del cap. 8 — pero para objetos compartidos, no para ids. Buen puente mental entre el mundo de las identidades estables y el de la memoria gestionada.
- Dentro del libro: **cap. 7** (por qué `usize` hoy y la nota de `slotmap` que este cap. cumple), **cap. 8** (los tombstones `Vec<Option<T>>` a los que coloca la generación), **cap. 14** (CSR/segmentos, donde la posición física y el id lógico vuelven a separarse), **cap. 16** (compactación real que reordena el almacenamiento físico sin re-numerar identidades), **caps. 28-30** (la «estabilidad tras crash» que este cap. promete), y en el Vol.I **cap. 2** (la representación como «dónde físico»).

## Mini-diálogo: en guardia nocturna

> — Venga, ¿de verdad es para tanto un número? Es un `usize`, son cuatro líneas.
>
> — Ese `usize` es la diferencia entre una referencia y un accidente. Borras un nodo del medio y, si el id es la posición, la arista que «recordaba» el 1 ahora saluda a otro. Sin error, sin aviso: corrupción silenciosa.
>
> — Pero el `Vec<Option<T>>` del cap. 8 ya deja los huecos...
>
> — Sí, el tombstone evita el *corrimiento*: borrar el 2 no mueve a los demás. Pero si mañana reinsertas y el hueco se vuelve a llenar, el índice viejo apunta al recién llegado. Eso es el bug de ABA en persona. La generación sube al liberar: el id viejo sigue siendo el mismo número, pero *quemado* — ya no encuentra nada.
>
> — O sea, que para que el BFS del cap. 4 pueda guardar ids sin miedo...
>
> — Exacto. La identidad se vuelve un nombre que no cambia mientras el elemento vive, y que al morir no lo hereda nadie. Con eso, una cola puede guardar un id, un `GraphStore` puede referirlo, un WAL puede loguearlo sin miedo a que recicle. Todo el resto del motor cuelga de esta promesa.

---

*(Próximo capítulo: 4 — El primer recorrido: búsqueda en anchura (BFS). Aquí la identidad era estable como idea; ahora verás un algoritmo que LA RECORRE — que guarda ids en una cola y en un conjunto de visitados, asumiendo que un id estable no cambiará a mitad del cruce del grafo. BFS sobre el grafo con ids estables es la primera prueba de que la identidad que definimos aguanta ser utilizada.)*
