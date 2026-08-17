# CONTRATO DE CAPÍTULO — Vol.II Cap. 3: Identidad, referencias y datos estables

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **Capítulo
> conceptual** de la Parte I del Vol.II (sin módulo de código propio
> en `vol2-liradb`; el código empieza en el cap. 7). Es **uno de los
> más importantes** del libro: define **qué hace que una identidad sea
> estable** y cumple la promesa literal que
> `liradb-workspace/crates/vol2-liradb/src/cap07_modelo.rs` (líneas
> 3-6) dejó escrita — *«NOTA: en el cap 3 (Vol.II) se sustituirán por
> IDs generacionales (slotmap). Aquí usamos `usize` por simplicidad
> pedagógica.»* — y que `cap08_graph_store.rs` (tombstones +
> `Vec<Option<T>>`) ya usa como cimiento y anuncia reemplazar.

**Preguntas críticas del CORPUS (`vol-II-cap-03`)** que este contrato
responde: *«¿Por qué no usar índices posicionales? Caso de uso
real.»* y *«Estabilidad de IDs tras crash + recovery.»*

**Ganchos**: cap. 4 (BFS que recorrerá el grafo guardando ids en
cola + visitados *asumiendo* estabilidad); cap. 7 (el código ancla
ya escrito); cap. 8 (`MemoryStore` con tombstones); caps. 28-30 (la
promesa de estabilidad tras crash que aquí se ancla y ellos hacen
cierta).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: qué es un grafo (V + E, dirigido o
  no) (Vol.I caps. 1-2; Vol.II cap. 1); las cuatro representaciones
  (matriz, lista, edge list, CSR) y sus O(...) (Vol.I cap. 2 / Vol.II
  cap. 2); qué es un `Vec<T>`, `HashMap`, `Option<T>` y
  `Vec<Option<T>>`; que `NodeId = EdgeId = usize` en el
  `cap07_modelo.rs` con la nota «se sustituirá por IDs generacionales
  (slotmap)»; que `MemoryStore` usa tombstones (`None`) y NO
  re-numera al borrar (cap. 8).
- **Cree saber pero es vago/erróneo (misconcepciones)**:
  1. **«El id ES la posición en el array»** — un id que es índice es
     una *dirección* que se corrompe al mover o borrar.
  2. **«Un hueco es un error que conviene compactar»** — el
     tombstone es la forma correcta: hueco *enterrado*, no
     re-numerado.
  3. **«Reutilizar un índice liberado es inocuo»** — toda referencia
     viva señalará al recién llegado sin avisar. Es el **bug ABA**.
  4. **«Una clave natural (email, DNI) sirve como id»** — la clave
     natural es *dato*, mutable por definición; al cambiar rompe
     todas las referencias.
- **NO debe saber todavía**: la API completa de `slotmap` (se esboza
  la *forma* `(slot, generation)`, no la API); `Pager`/páginas
  (caps. 11-12); persistencia real (cap. 10); índices (cap. 15);
  WAL/recovery (caps. 28-30); concurrencia / `versioned CAS`. Se
  nombran como «luego lo verás».

## 2. Conceptos (del grafo curricular)

- `present`: **IDENTIDAD vs DATO**; **identidad vs índice**; **era /
  generación** (`(slot, generation)`); **id-reuse / problema ABA**;
  **slot vs elemento**; **surrogate key vs natural key**;
  **tombstone** (puente al cap. 8); **dangling pointer** como
  analogía desde C.
- `practice`: distinguir `None` (hueco) de `Some` (habitado) en
  `Vec<Option<T>>`; razonar sobre invariantes DERIVADAS, no
  memorizadas.
- `consolidate`: grafo y representación (Vol.I cap. 2 / Vol.II cap.
  2); separación identidad/datos declarada por cap. 7 con
  `NodeId = EdgeId = usize` y promesa de migrar; tombstones
  `Vec<Option<T>>` del cap. 8.
- `out_of_scope` (nombrados): `slotmap` completo (esbozo);
  persistencia post-crash (caps. 28-30 — *promesa anclada aquí*);
  CSR persistente (cap. 14); páginas (cap. 11); concurrencia
  lock-free.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge** (al terminar sabe afirmar):
  1. Distingue **índice** (posición volátil) de **id** (nombre
     estable).
  2. Enuncia la solución: id = `(slot, generation)`; cada borrado
     sube la generación del slot; un id viejo «apunta a nada»
     (generación no casa) en vez de al recién llegado.
  3. Explica el **problema ABA** y por qué es la forma canónica del
     id-reuse.
  4. Distingue **clave natural** (dato mutable) de **clave
     surrogate** (id inventado, inmutable) y por qué la BBDD elige
     surrogate.
  5. Enuncia la **estabilidad de IDs tras crash + recovery** que los
     caps. 28-30 harán cierta, y por qué un id debe sobrevivir a un
     reinicio sin re-numerarse.
- **Skills** (ejecuta):
  1. Simula en pizarra / mini-snippet un `Vec<(T, generation)>` y
     comprueba que un id viejo devuelve `None` mientras el recién
     llegado sí se encuentra.
  2. Identifica en `MemoryStore` (cap. 8) los puntos donde el
     `usize` actual es el *puntal pedagógico* que este capítulo
     convierte en *tornillo generacional*.
  3. Decide si un identificador del sistema que construye es «el
     *quién*» (id) o «el *qué*» (dato).
- **Wisdom** (cuándo NO / qué pesa más):
  1. La **identidad es una PROMESA COMPARTIDA**: el BFS del cap. 4,
     el `GraphStore` del cap. 8 y el WAL de los caps. 28-30 la
     «compran». Sin ella todo se rompe por contagio.
  2. Prefiere **clave surrogate estable** aunque la natural «se lea
     mejor», porque el dato del dominio SIEMPRE cambia y la
     identidad NUNCA debe.

## 4. Modelo mental

- **Figura rectora**: el **número de expediente del hospital**
  (cap. 7) llevado al extremo. La estantería tiene *huecos físicos*
  que NO se rellenan renumerando; el 2 queda *enterrado* (tombstone).
  Pero en un `Vec<usize>` puro, reinsertar «el 2» significa otra
  persona. Un *número de legajo* (generación) lo resuelve: cada vez
  que una casilla se libera, sube su legajo; un id viejo lleva el
  legajo viejo y «muere» limpio. **El id real es `(slot, legajo)`,
  no el slot desnudo.**
- **Diagramas ASCII obligatorios**:
  - (a) `Vec<Option<T>>` con tombstones (cómo se ven los huecos).
  - (b) `Vec<(T, generation)>` con insertar / borrar / re-insertar y
    demostración de que un id viejo deja de coincidir.
  - (c) **ABA** como secuencia A→B→A visual.
- **Momento ¡ajá!**: *«Un índice de array es una DIRECCIÓN que se
  rompe al mover las cosas. Un id estable es un NOMBRE que aguanta.
  La diferencia se compra con un contador por slot — una u32 por
  casilla. Caro de entender, barato de implementar, y es LO QUE
  permite que millones de referencias sigan siendo válidas aunque
  añadas y borres nodos para siempre.»*

## 5. Los porqués (grill — la pregunta más importante de cada decisión)

### 5.1 ¿Por qué el id NO es la posición del array?
- **Resuelve**: que el id sobreviva a borrar / reordenar /
  reinsertar. Identidad = nombre estable.
- **Se descartó**: `id = índice` (cap. 7, pedagógico) — el índice
  es una *dirección* que el borrado corrompe; la clave natural
  (email) — es *dato*, mutable.
- **Modo de fallo si no**: arista `(source, target)` apuntando al
  nodo equivocado tras un borrado — corrupción sin error.
- **Evidencia**: `cap07_modelo.rs` líneas 3-6 (nota de slotmap);
  `cap08_graph_store.rs` (`Vec<Option<Node>>` + comentario
  «Reciclar claves sin un número de generación es lo que un
  esquema serio evita»); CORPUS `vol-II-cap-03`.

### 5.2 ¿Por qué `(slot, generation)` y no otra cosa?
- **Resuelve**: O(1) de acceso (el `slot` indexa directo), +1 u32
  de overhead (la generación) y garantía de que un id viejo nunca
  coincide con uno nuevo aunque el slot se reutilice.
- **Se descartó**:
  - **UUID por nodo**: 16-28 bytes + hashing; estable pero no
    adyacente. Vale para identidad global entre máquinas (no es
    nuestro caso).
  - **Una era global**: basta que *un* slot recicle para que la era
    suba e invalide referencias válidas a OTROS slots. La
    generación debe ser POR SLOT.
- **Modo de fallo si no**: bug ABA (id-reuse silencioso) o
  invalidación masiva por culpa de una sola casilla.
- **Evidencia**: crate `slotmap` (Rust); generational arenas en
  motores ECS (`bevy_ecs`/`legion`); Herlihy et al. sobre
  lock-free data structures y versioned CAS.

### 5.3 ¿Por qué el tombstone del cap. 8 y no compactar / re-numerar?
- **Resuelve**: borrar deja HUECO en el `Vec<Option<T>>` SIN
  mover a los siguientes — el id del resto no salta. Barato y la
  única forma de no romper referencias por corrimiento.
- **Se descartó**: **compactar a la izquierda tras cada borrado**.
  O(n) por borrado y re-numera TODAS las referencias; el id deja
  de ser estable incluso para vecinos que no se tocaron.
- **Insuficiente por sí solo**: evita el corrimiento pero NO el
  reciclaje si reinsertas en el hueco. De ahí la generación de §5.2.
- **Evidencia**: `MemoryStore::delete_node` → `self.nodes[id]
  = None`; cap. 8 §tombstones; cap. 16 (compactación real:
  reordena físico, NO re-numera identidades).

### 5.4 ¿Por qué surrogate key en vez de natural key?
- **Resuelve**: que el id NO signifique nada del mundo real para
  que NADA del mundo real pueda cambiarlo. Cambias el email y el
  id sigue siendo el mismo; las aristas no se rompen.
- **Se descartó**: usar la clave natural como id. Es MUTABLE por
  definición y obliga a cascadas o deja aristas huérfanas.
- **Modo de fallo si no**: renombrar un nodo y TODAS sus
  amistades/relaciones apuntan al vacío o al equivocado.
- **Evidencia**: literatura clásica de diseño relacional;
  Kleppmann, *Designing Data-Intensive Applications*, cap. 3;
  práctica común en Postgres/MySQL.

### 5.5 ¿Por qué la «estabilidad de IDs tras crash + recovery»?
- **Resuelve**: que el id de un elemento vivo sobreviva a un
  reinicio sin re-numerarse. Si el motor re-numera tras un crash,
  invalida todas las referencias externas guardadas antes del
  fallo (WAL, snapshots, caches de aplicación).
- **Se descartó**: «renumerar desde cero» tras cada reload.
  Apariencia limpia, realidad catastrófica.
- **Cómo se cumple**: el par `(slot, generation)` se PERSISTE
  junto al dato (caps. 9-11) y la generación se SUBE al liberar
  (este capítulo); el WAL del cap. 28 loguea la operación; el
  recovery del cap. 29 reconstruye SIN re-numerar.
- **Evidencia**: CORPUS `vol-II-cap-03` («Estabilidad de IDs tras
  crash + recovery»); ADR del cap. 16 sobre compactación «no
  re-numera»; caps. 28-30.

## 6. Primera solución vs solución evolucionada

- **Ingenua**: `id = índice en el array`. `cap07_modelo.rs` la usa
  aposta como puntal pedagógico porque `nodes[i]` es legible y
  O(1). Funciona mientras NUNCA se borre ni se reutilice.
- **Qué la rompe**: el **escenario borrar + re-insertar**. Creas
  `A, B, C` (0, 1, 2), borras `B` (1), reinsertas a otro en el
  hueco: cualquier arista que «recordaba» el 1 señala al recién
  llegado. **ABA en su forma pura.**
- **Evolución visible**: id `usize` → `(slot, generation)`. Al
  borrar `B` la generación del slot 1 sube de `g` a `g+1`; el
  nuevo nodo rellena el slot 1 con `(1, g+2)`. La referencia vieja
  `(1, g)` deja de casar la generación → `None`. El `MemoryStore`
  del cap. 8 ya depende de esto: `Vec<Option<T>>` distingue
  huecos; este capítulo AÑADE la **generación** para que
  REUTILIZAR el hueco no suponga REUTILIZAR el id.

## 7. Prueba de fuego

- **Capítulo conceptual**: la «prueba de fuego» es una
  **demostración razonada** que el lector puede reproducir a mano
  contra el modelo ya instalado: (a) escribe el estado del
  `MemoryStore` del cap. 8 (`Vec<Option<Node>>` con tombstones);
  (b) simula borrar el nodo 1 y re-insertar uno nuevo en el slot
  1 SIN generación. Observa cómo `let x = 1;` señala al recién
  llegado; (c) repite con `(slot, generation)`: comprueba que
  `buscar((1, g_viejo)) == None` mientras `buscar((1, g_actual)) ==
  Some(nuevo)`. Verificable con un mini-snippet en el workspace.
- **Síntoma si el lector se salta este capítulo**: el código del
  cap. 4 (BFS) y del cap. 8 (`GraphStore`) le parecerán puntales
  arbitrarios; ante un borrado producirá, silenciosamente, el mismo
  nodo visitado dos veces o un vecino fantasma — el bug ABA en la
  cara, sin error que lo delate.

## 8. Trampas y errores comunes

1. **Tratar el índice como nombre estable**: `let j = i; grafo[j]`.
   Cualquier borrado en medio invalida `j` en silencio.
2. **Compactar / re-numerar para «limpiar huecos»**: reescribe
   TODAS las referencias (aristas, adjacencias, WAL). El tombstone
   es la alternativa barata; la compactación real del cap. 16
   reordena almacenamiento físico pero **NO** re-numera.
3. **Usar una clave natural como id**: cambias el email y todas las
   amistades apuntan al vacío. La natural va a `props`; el id es
   surrogate.
4. **Reutilizar el slot sin generar**: rellenar el hueco ahorra
   memoria (bien), hacerlo SIN subir la generación = bug ABA (mal).
   La generación es el *precio* de la reutilización segura.
- **Glosario**: *identidad* (el «quién») vs *valor / dato* (el
  «qué»); *índice* (posición) vs *id* (nombre); *slot* (casilla)
  vs *elemento* (lo que vive en ella); *tombstone* (hueco
  enterrado, `None`); *era / generación*; *ABA problem*; *surrogate
  key / natural key*; *dangling pointer*; *id-reuse*.

## 9. Ejercicios (exercise-designer)

- **`recordar` (esencial — retrieval)**: SIN mirar el capítulo ni
  el código, escribe de memoria: (1) definición de *id estable* vs
  *índice*; (2) mecanismo generacional completo (qué es
  `(slot, generation)`, qué hace un borrado, qué devuelve un id
  viejo al releer). *Criterio*: tres afirmaciones mínimas — el id
  es nombre inmutable; un borrado sube la generación del slot; un
  id con generación vieja «apunta a nada».
- **`analizar / interleaving` (intermedio — punteros en C)**: en C,
  `int *p = &arr[i];` y luego compactar / redimensionar `arr` deja
  `p` colgando (dangling pointer). Explica cómo la generación de
  LiraDB es el análogo seguro: ¿qué sustituye a la dirección de
  memoria? ¿qué comprueba la generación que el puntero de C no
  comprueba? *Pistas*: (1) el puntero de C guarda solo la dirección;
  el id guarda DOS números; (2) `&arr[i]` no avisa al reasignar
  el array; LiraDB SÍ; (3) «dirección reciclada» en C ≈ «slot
  reciclado sin generación» en Rust. *Criterio*: relaciona
  *dangling pointer* con *generación quemada*.
- **`crear` (experto — test local)**: implementa un mini-snippet en
  `liradb-workspace/crates/vol2-liradb/examples/` que (a) construya
  un `Arena` con `Vec<(Option<T>, u32)>`; (b) reproduzca el doble
  escenario (sin / con generación) y devuelva con `cargo run
  --example` la consola clara de «apunta al recién llegado» vs
  «apunta a nada». *Criterio*: tests visibles y diferenciales.

## 10. Preguntas abiertas (gancho al cap. 4)

1. Tenemos la identidad estable… ¿cómo la **recorren** los
   algoritmos? El BFS del cap. 4 guardará esos ids en una cola y en
   un `HashSet<NodeId>` de visitados, **asumiendo** que no cambiará
   mientras el BFS vive. ¿Qué ve el `Some / None` y cómo lo maneja?
2. Un id estable vivirá en el `trait GraphStore` del cap. 8 y en
   el `Pager`/páginas de disco (caps. 11-16). ¿Cómo se lleva un
   `(slot, generation)` a bytes en un fichero que debe sobrevivir a
   un reinicio? ¿Qué campos se persisten juntos?
3. La promesa «el id no cambia mientras el elemento exista»…
   ¿cuánto cuesta mantenerla con WAL y recovery (caps. 28-30)?
   ¿Cómo sabemos, tras un crash, que un id NO fue reutilizado?
- **Términos nuevos de glosario**: identidad estable,
  era / generación, `(slot, generation)`, `slotmap` (nombrado),
  tombstones, problema ABA, surrogate key / natural key, dangling
  pointer, id-reuse, `Vec<(T, generation)>`.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: ejercicio esencial obliga a RE-ESCRIBIR
  de memoria la definición y el mecanismo generacional — recordar,
  no reconocer.
- **Spacing**: re-ejercita la promesa de `NodeId = EdgeId = usize`
  del cap. 7 y los tombstones del cap. 8.
- **Interleaving**: ejercicio intermedio mezcla la generación con
  *referencias estables* de lenguajes con punteros (dangling pointer
  en C). El BFS del cap. 4 es el anzuelo que cierra.
- **Regla de dificultad asimétrica**: una idea nueva por sección
  (identidad → índice no sirve → tombstones → generación → ABA →
  surrogate keys); los ejercicios exigen recuperación.
- **Bucle de feedback inmediato**: el mini-snippet es ejecutable
  (`cargo run --example ...`); feedback «apunta a nada vs apunta
  al recién llegado» verificable al instante.
- **Citas (alta confianza, no paramétricas)**:
  - `cap07_modelo.rs` líneas 3-6 (la nota de slotmap que este
    capítulo cumple).
  - `cap08_graph_store.rs` (tombstones + comentario «Reciclar
    claves sin un número de generación es lo que un esquema serio
    evita»).
  - **Problema ABA**: Herlihy et al., *The Art of Multiprocessor
    Programming*, cap. 10 (non-blocking sync y versioned CAS).
  - **`slotmap`** / generational arenas: docs.rs del crate
    `slotmap`; pattern usado en `bevy_ecs` y `legion`.
  - **Surrogate vs natural keys**: Kleppmann, *Designing Data-
    Intensive Applications*, cap. 3; convención clásica del
    diseño relacional.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa
      descartada, modo de fallo y fuente (5 filas en §5).
- [x] Escenario de fallo visible: borrar + re-insertar (ABA) y
      *dangling pointer* en C.
- [x] Capítulo conceptual: no genera código nuevo; **ancla** la
      promesa del `cap07_modelo.rs` (líneas 3-6) y conecta con el
      `MemoryStore` del cap. 8.
- [x] ≥4 misconcepciones corregidas explícitamente (§1).
- [x] Ejercicios con solución verificable: retrieval + interleaving
      + mini-snippet.
- [x] ≥1 retrieval (recordar) y ≥1 spacing (cap. 7 promesa + cap. 8
      tombstones).
- [x] Responde AMBAS preguntas críticas del CORPUS `vol-II-cap-03`.
- [x] Anécdota del cap. 11.0 aplicada a IDENTIDAD (no a páginas):
      «el fichero se mueve» → «la posición del array se mueve».
- [x] Cita cada afirmación técnica a `cap07_modelo.rs`,
      `cap08_graph_store.rs`, CORPUS y papers / docs de
      `slotmap`/ABA.
- [x] Estructura del cap. 11 obligatoria: 3.0 anécdota → objetivo →
      problema → modelo mental ASCII → primera solución → límites →
      evolucionada → porqués → trampas → historia → ejercicios →
      profundizar → mini-diálogo → gancho al cap. 4 (BFS).
- [x] Mini-diálogo de cierre con «en guardia nocturna».
