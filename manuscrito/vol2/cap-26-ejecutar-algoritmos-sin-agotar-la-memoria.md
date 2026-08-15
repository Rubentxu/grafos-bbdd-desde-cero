# Capítulo 26 — Ejecutar algoritmos sin agotar la memoria (proyección, streaming, frontiers)

> *«La memoria no se gestiona con swap. Se gestiona decidiendo qué existe y cuándo.»*

## 26.0 La anécdota de la esquina

Hacia 2008-2009, en Google se toparon con un muro que no era de algoritmos. El grafo de la Web, el de las redes sociales, los mapas de enlaces entre sitios: miles de millones de vértices que no cabían en la RAM de ninguna máquina. La herramienta generalista de la casa, MapReduce, encadenaba trabajos que re-leían el grafo ENTERO en cada iteración — y los algoritmos de grafos son, por naturaleza, iterativos. PageRank con 50 pasadas era re-leer el planeta 50 veces para que cada página mirase a sus vecinas.

La respuesta se presentó en SIGMOD 2010: Grzegorz Malewicz y seis coautores firmaron «Pregel: a System for Large-Scale Graph Processing» (pp. 135-146). La idea se resume en tres palabras que desde entonces son un lema: **think like a vertex** — piensa como un vértice. No escribas código que recorre el grafo: escribe el `compute()` de UN vértice, que en cada *superpaso* lee los mensajes que le llegaron, actualiza su estado y manda mensajes a sus vecinos. El sistema reparte los vértices entre máquinas y sincroniza los superpasos con una barrera — el modelo *bulk synchronous parallel* que Leslie Valiant había publicado en 1990 en Communications of the ACM. Cada vértice trabaja sólo con su trozo; el grafo nunca necesita estar entero en un sitio.

Y fíjate en la cronología de lo que vino DESPUÉS, porque es la trama de este capítulo. **Giraph** clonó Pregel en open source (y Facebook lo escaló a billones de aristas). **GraphChi** (Kyrola, OSDI 2012) invirtió la apuesta: nada de clúster — mil millones de aristas desde el DISCO de un solo PC, con *parallel sliding windows* que procesan el grafo por bloques sin cargarlo. **GraphX** (Xin et al., OSDI 2014) lo unificó todo dentro de Spark. Dos familias sobreviven de aquella era: los que **materializan** una copia compacta del grafo y la iteran muchas veces (Pregel, Giraph, GDS), y los que **procesan sin cargar** (GraphChi y su estirpe). Hoy construyes las dos, a mano, sobre LiraDB.

## 26.1 Objetivo

Al terminar este capítulo sabrás **por qué ejecutar un algoritmo no es lo mismo que leer el grafo**, y habrás construido las dos estrategias con las que una base de datos real responde a esa diferencia. Cuatro piezas en `cap26_proyeccion.rs`:

1. **`ProyeccionPonderada`** — el grafo (o su subgrafo filtrado) materializado en memoria compacta, con pesos resueltos UNA vez. La API pública que la Parte V llevaba debiendo desde los caps. 22 y 24.
2. **`FronterasBfs`** — un `Iterator` perezoso que produce frontera a frontera leyendo el store bajo demanda, bajo un `Presupuesto` con `MotivoParada` explícito.
3. **`ContandoStore`** — el voltímetro: un wrapper que cuenta las lecturas que llegan de verdad al store, para VERIFICAR las promesas de los dos anteriores.
4. **`BitSet`** — el conjunto de visitados a mano: un bit por nodo, la lección de denso contra disperso.

Y el hito que cierra la Parte V: un BFS de profundidad 2 sobre una cadena de 500 nodos que lee **2 de las 499 aristas**, y 5 Dijkstras que cuestan **11 lecturas en vez de 45**. Medidos, no prometidos.

## 26.2 Problema

La Parte V lleva cuatro capítulos ejecutando algoritmos SOBRE el store persistente, y cada uno leyendo arista a arista lo que necesita: `out_edges(u)` y `get_edge(e)` por cada relajación. Para UNA consulta, perfecto. Pero mira los dos extremos que ya tienes:

**El extremo iterativo.** El cap. 24 calculó closeness con un BFS por cada origen; el 25 iteró Louvain nivel a nivel. Llevado a Dijkstra sobre el store (cap. 22), cada llamada re-lee y re-valida E aristas. En una cadena de 12 nodos, cinco orígenes distintos leen 11+10+9+8+7 = **45 aristas**. ¿Para qué? Los pesos no cambiaron entre la primera y la quinta pasada. Estás pagando cinco veces la misma fotocopia. Y cada una de esas lecturas tiene precio doble desde el cap. 13: la que falla la caché del buffer pool cuesta una página de disco — comprarlas por adelantado, cuando ya sabes que vas a necesitar MUCHAS, es exactamente el trato que aquí cerramos.

**El extremo local.** «¿Quién está a 2 saltos de Ana?» Con lo que sabes, la respuesta honesta del motor es: materializa el grafo y recórrelo. Sobre una cadena de 500 nodos, eso es cargar 499 aristas para usar... 2. El 99,6% de lo leído se tira. Y lo de antes al revés: aquí la estrategia del cap. 22 (leer bajo demanda) era la BUENA, y materializar sería el disparate.

Dos síntomas opuestos, un mismo diagnóstico: **tratar todas las consultas igual**. Y una espina clavada desde el cap. 22: el CSR persistente del cap. 14 sólo guarda topología (offsets y targets) — sin ids de arista ni pesos no puede alimentar un Dijkstra. La deuda está escrita en el banner de aquel módulo.

La pregunta del capítulo: ¿qué existe en memoria en cada momento, y quién decide cuándo?

## 26.3 Modelo mental

Piensa en un **archivo provincial** con dos formas de trabajar:

- **La biblioteca que fotocopia UNA sección.** Vas a necesitar esa sección muchas veces (un análisis, una tesis). Pagas UNA fotocopia completa de la sección que te interesa — ni más (filtro: sólo Personas, sólo KNOWS) ni menos — y trabajas sobre tu copia. El archivo sigue recibiendo documentos nuevos mientras tanto, pero tu copia es una **foto**: inmutable, coherente, tuya. Materializar = `ProyeccionPonderada`.
- **El archivista que va hoja a hoja.** Sólo necesitas saber qué hay a dos carpetas de la tuya. Le pides la carpeta, la miras, pides las que cita, y le dices basta. Nunca pidió lo que no necesitabas. Le pones un tope — «máximo 10 carpetas» — y él te deja una nota final: si trajo TODO lo que había o se quedó sin permiso. Streaming = `FronterasBfs` + `Presupuesto` + `MotivoParada`.

Y en la puerta del archivo, **un contador de solicitudes**: el archivista puede presumir de trabajar poco; el contador no sabe presumir, sólo sumar. Ese es `ContandoStore`.

```
               ┌─ MATERIALIZAR (K iteraciones) ─────────────────────────┐
               │  store ──proyectar(filtro)──► copia CSR (pesos ya)     │
 store vivo ───┤                                    │                   │
 (OLTP sigue   │                        K algoritmos │ CERO lecturas     │
  mutando)     │                        sobre la foto │ del store        │
               └────────────────────────────────────────────────────────┘
               ┌─ STREAMEAR (consulta local) ───────────────────────────┐
               │  store ──frontera 0──► frontera 1──► frontera 2 ──║ corte
               │            lee SÓLO la adyacencia que expande      ║ presupuesto
               │            memoria ∝ visitados, nunca ∝ grafo      ║ MotivoParada
               └───────────────────────────────────────────────────────┘
```

El momento ¡ajá!: «¿qué hay a 2 saltos de Ana?» no necesita el grafo — necesita DOS adyacencias. Y «Dijkstra desde todos los nodos» no necesita releer el grafo V veces — necesita UNA fotocopia. El tipo de consulta decide la estrategia; el motor sólo tiene que ofrecerte las dos.

## 26.4 Primera solución

La solución ingenua ya la tienes y funciona: es la de los caps. 22-23. Cada algoritmo lee el store cuando lo necesita, arista a arista, sin copia previa. Cero infraestructura nueva. Para una consulta suelta, es incluso la opción correcta.

Y su gemela simétrica, igual de tentadora: «pues materializo siempre el grafo entero al abrir la base de datos, y todos los algoritmos trabajan sobre la copia».

## 26.5 Sus límites

Ambas se rompen en el extremo contrario:

1. **Leer-bajo-demanda multiplica por K.** Cada Dijkstra del cap. 22 valida los E pesos eager (una BD prefiere fallar ruidosamente a contestar casi-bien — política que NO vamos a ablandar) y luego relaja leyendo del store. El closeness ponderado que el cap. 24 dejó apuntado como deuda exigiría V Dijkstras: V·E lecturas. Las 45 de la cadena de 12 son 11 útiles + 34 repetidas.
2. **Materializar-siempre castiga lo local.** La cadena de 500: cargar 499 aristas para contestar con 2. Y de regalo, O(grafo) de memoria para una consulta O(2 saltos) — el título del capítulo, literal.
3. **La copia ingenua no sabe de filtros ni de huecos.** Sin más, copiarías nodos borrados (huecos de `delete_node`, cap. 16) y aristas que la consulta no puede pisar (hacia nodos fuera del subgrafo: un subgrafo no tiene aristas colgando hacia nodos que no contiene).

Conclusión: no hay UNA estrategia buena; hay DOS, y la consulta elige. Empecemos por la primera.

## 26.6 Solución evolucionada, parte 1: la proyección materializada

`ProyeccionPonderada::proyectar(store, &WeightSource, &FiltroProyeccion)` hace UNA pasada y devuelve una copia compacta e inmutable. Su layout es el CSR del cap. 14 **completado** — lo único que allí no cabía en disco, aquí sí en memoria:

```text
nodes:   [id0, id1, ...]     ids ordenados          (determinismo)
index:   id → posición densa compacta huecos        (herencia del cap. 24)
offsets: [0, g0, g0+g1, ...]  fronteras de fila u32  (EL CSR del cap. 14)
targets: posiciones destino   ← lo ÚNICO que persiste el cap. 14
pesos:   f64 por arista       ← lo que AÑADE este capítulo
aristas: EdgeId por arista    ← lo que AÑADE este capítulo
```

Tres decisiones que valen un porqué cada una:

**¿Por qué el filtro hace que las aristas de nodos excluidos NI SE LEAN?** Porque `proyectar` itera las ADYACENCIAS de los nodos admitidos — no todas las aristas. Una arista cuyo ORIGEN está fuera del filtro jamás llega a `get_edge`: su adyacencia no se consulta. El ahorro del subgrafo no es «leer y tirar», es no leer. Mira el bucle, que es todo el secreto:

```rust
for &u in &nodes {                    // SÓLO nodos admitidos: la adyacencia
    for eid in store.out_edges(u) {   // de un EXCLUIDO jamás se itera
        let edge = store.get_edge(eid)?;
        stats.edges_scanned += 1;
        if !filtro.admite_arista(edge) { stats.descartadas += 1; continue; }
        let destino = match index.get(edge.target).copied().flatten() {
            Some(d) => d,
            None => { stats.descartadas += 1; continue; } // destino fuera
        };
        let w = edge_weight(edge, weight)?;               // peso UNA vez
        fila.push((destino, eid, w));
    }
}
```

Y es medible: en el test de la red mínima (2 Personas + 1 Ciudad, aristas 0→1 KNOWS, 1→4 VIVE_EN, 4→0 VIVE_EN), proyectar sólo Personas da `edges_scanned = 2` (las adyacencias de los dos nodos vivos) y `descartadas = 1` — la 1→4 se leyó y se descartó (su destino no entra), pero la 4→0 **no aparece ni en descartadas: no existe para esta proyección**. Esa es la diferencia entre filtrar al leer y filtrar leyendo.

**¿Por qué los pesos se resuelven UNA vez, con la semántica ESTRICTA del cap. 22?** Porque la calidad del dato no se negocia por el lado analítico: prop ausente/NULL = `MissingWeight`, tipo no numérico = `InvalidWeight`, NaN/±∞ = `NonFiniteWeight` — el mismo `edge_weight`, el mismo contrato, ahora pagado O(E) una sola vez en la vida de la copia. Es el trato de la analítica: 11 lecturas que valen para las 5 consultas siguientes. Los pesos negativos NO son error de proyección (Bellman-Ford los admite): los rechaza `dijkstra_proyeccion` eager sobre TODA la copia — y aquí está la sutileza económica: `dijkstra_proyeccion` valida por llamada, pero `closeness_ponderado` valida UNA vez y llama V veces al núcleo sin validar. La sanidad se paga una vez; quien itera, no re-paga.

**¿Por qué es un snapshot y no una vista viva?** Porque inmutable es una FEATURE: los V Dijkstras del closeness recorren la MISMA foto (resultados consistentes entre sí, determinismo total — dos proyecciones del mismo store son `PartialEq` idénticas), y el store puede seguir recibiendo escrituras OLTP mientras la analítica corre. La separación OLTP/analítica del guion no es un diagrama: es un tipo que congela un instante. (¿Qué garantiza que el instante fue coherente? Nada todavía — eso es la Parte VI.)

Sobre la foto, las deudas se pagan solas: `dijkstra_proyeccion` reproduce punto por punto al del cap. 22 (mismas distancias, mismos caminos, misma arista elegida entre paralelas — test `dijkstra_proyeccion_coincide_con_dijkstra_store`), y `closeness_ponderado` hace lo que el cap. 24 prometió: Wasserman-Faust con distancias PONDERADAS. En la cadena 0→1→2→3 con pesos 1, 5, 1: el nodo 0 cae de 3/6 (saltos) a 3/14 (Σd = 1+6+7), el 1 a 4/33, y el 2 ni se entera (1/3 en ambas: su mundo no toca la arista cara). La fuente de pesos cambia la respuesta — la lección del cap. 22, ahora en centralidad.

## 26.7 Solución evolucionada, parte 2: streaming por fronteras con presupuesto

La otra mitad del título. `bfs_fronteras(store, origen, dir, presupuesto)` devuelve un `FronterasBfs` que **es un `Iterator` de verdad**:

```rust
let mut it = bfs_fronteras(&s, 0, Out, Presupuesto::profundidad(2))?;
it.next();  // Some([0])  — el origen
it.next();  // Some([1])  — leyó 1 arista
drop(it);   // la adyacencia del 1 JAMÁS se consultó
```

**¿Por qué un Iterator y no un `Vec<Vec<NodeId>>` de niveles?** Porque el Vec materializa TODO antes de que empieces: pagas el recorrido completo para quizás mirar dos niveles. El iterador es perezoso de verdad: la frontera k+1 no existe hasta que la pides — su adyacencia se lee al EXPANDIR, nunca antes. El que consume decide cuándo parar (un callback que encontró lo que buscaba suelta el iterador y ahí se acabó la factura). Test: `bfs_iterador_perezoso_una_frontera` — pedir 2 fronteras de una cadena de 6 deja el voltímetro en exactamente 1 lectura de arista.

**¿Por qué el `Presupuesto` se comprueba ANTES de cada lectura?** Porque un límite que se puede superar no es un límite. Mira dónde viven los chequeos — dentro del bucle, no por frontera:

```rust
for eid in eids {
    if let Some(max) = presupuesto.max_lecturas
        && stats.aristas_leidas >= max {
        terminado = Some(MotivoParada::PresupuestoLecturas);
        break 'nodos;                       // ANTES de leer
    }
    let edge = store.get_edge(eid)?;         // la lectura autorizada
    stats.aristas_leidas += 1;
    // ... y el de nodos, ANTES de marcar el descubrimiento:
    if let Some(max) = presupuesto.max_nodos
        && stats.nodos_visitados >= max { /* corte */ }
}
```

`Presupuesto{max_profundidad, max_nodos, max_lecturas}` se valida antes de cada `get_edge` y antes de cada descubrimiento: las promesas son EXACTAS — «máximo 2 lecturas» produce exactamente 2. Y `max_lecturas` es el presupuesto más importante de los tres en un store en disco: acotar lecturas es acotar el tiempo Y la memoria de trabajo de una sola vez. (Un presupuesto de 0 no tiene sentido — «no empezar» se consigue no llamando — y se rechaza con error tipado.)

**¿Por qué `MotivoParada` es parte de la RESPUESTA?** Porque cambia lo que el resultado significa. `Completo`: se agotó la componente — tu lista de nodos ES la respuesta. `PresupuestoNodos` o `PresupuestoLecturas`: había más y no te lo puedo dar — tu lista es un recorte, y tomar decisiones sobre él como si fuera completo es mentir con estadística. `ProfundidadMaxima`: cortaste tú, en el borde exacto que pediste. El test lo clava: un nodo aislado con presupuesto de 1 nodo acaba en `Completo` (no había nada más), no en `PresupuestoNodos` — mismo recorrido, significados opuestos.

Detalle fino que la dirección regala: en streaming, `GraphDirection::In` es GRATIS — se leen las `in_edges` bajo demanda, no hay que transponer nada (la proyección dirigida-out habría necesitado una segunda copia). Y `Both` deduplica por bitset, documentando que un store simetrizado a mano paga cada par dos veces: visible en las stats, no escondido.

`bfs_streaming` es la versión de una tirada: consume el iterador y te devuelve niveles + stats + `MotivoParada` en un `RecorridoBfs`. Mismo motor, dos ergonomías.

## 26.8 Solución evolucionada, parte 3: el voltímetro y el bitset

Todo lo anterior PROMETE lecturas («2 de 499», «45 contra 11»). ¿Quién verifica la promesa? No el propio algoritmo: **no confíes en que el código se auto-auditore** — el mismo bug que infla la optimización podría desinflar el contador. `ContandoStore` es un voltímetro: un wrapper de sólo lectura sobre `&dyn GraphStore` cuyos `get_edge`/`get_node`/`out_edges`/`in_edges` suman en contadores `Cell` y delegan:

```rust
fn get_edge(&self, id: EdgeId) -> Option<&Edge> {
    self.lecturas_arista.set(self.lecturas_arista.get() + 1); // Cell: los
    self.inner.get_edge(id)                                    // métodos son &self
}
fn put_edge(&mut self, _: Edge) -> Result<(), StoreError> {
    panic!("ContandoStore es un instrumento de medida de sólo lectura")
}
```

Fíjate en las dos formas: `Cell` porque el trait de lectura va con `&self` (contar es una mutación invisible para el sistema de tipos), y el `panic` en las escrituras porque un instrumento de medida no es un store — si dejases escribir a través del voltímetro, dejarías de poder confiar en lo que cuenta. Los tests enchufan el BFS o la proyección al voltímetro y exigen que la stats interna y el contador externo — dos fuentes INDEPENDIENTES — coincidan. Es el patrón de medir desde fuera, y su lección trasciende el capítulo: cuando un sistema se autoinforma, pon un instrumento que él no controle.

Y el `BitSet`: el conjunto de visitados del BFS, a mano — `Vec<u64>`, un bit por id. **¿Por qué aquí BitSet y en el filtro HashSet?** Denso contra disperso. Los ids del store nacen densos (cap. 7): un bitset gasta 1 bit por id posible — 1/8 de lo que ocupa un `usize` en un HashSet, sin hashing ni rehash, con la consulta «¿ya visité este vecino?» en dos instrucciones. Crece bajo demanda (una palabra cada 64 ids) y su espacio es O(id_máximo_visitado/8). Pero el `FiltroProyeccion` guarda STRINGS arbitrarios (etiquetas, tipos) dispersos por definición: un bitset no puede indexarlos sin un diccionario previo, y entonces ya tienes un HashSet con pasos extra. Regla: universo denso de enteros → bitset; claves dispersas o arbitrarias → tabla hash. (¿Y si los ids fueran gigantes y dispersos? El bitset pagaría palabras vacías: el doc del `BitSet` lo deja escrito, y el HashSet ganaría. Aquí no pasa: los ids nacen del contador del store.)

## 26.9 Prueba de fuego: la tesis y la economía, medidas

**El test-tésis** (`bfs_streaming_no_lee_todo_el_grafo`): cadena de 500 nodos, 499 aristas. BFS profundidad 2 desde el 0:

```text
niveles          = [[0], [1], [2]]
parada           = ProfundidadMaxima
nodos_visitados  = 3        aristas_leidas = 2   ← 2 de 499 (<0,5%)
voltímetro       = 2        ← la stats interna NO mintió
```

Dos aristas leídas — no tres: con profundidad 2 sólo se EXPANDEN los nodos 0 y 1; para DESCUBRIR el 2 basta leer la arista de 1, y el 2 no se expande porque su frontera ya excede el presupuesto. El resto del grafo — 497 aristas — no existió para esta consulta. Esa es la tesis del capítulo en un `assert`.

**El test-economía** (`economia_multiorigen_una_lectura_por_adelantado`): misma cadena de 12 (E = 11). Por la proyección: materializar = 11 lecturas; 5 Dijkstras después = SIGUEN siendo 11. Directo contra el store (cap. 22): 11+10+9+8+7 = **45**. Cuatro veces más, y cada origen extra agranda la brecha. (¿Por qué 45 y no 55? La validación eager del cap. 22 usa `iter_edges` — el voltímetro cuenta `get_edge` — y cada Dijkstra del origen i expande los nodos i..11 y lee 11−i aristas. Calibrar contadores se hace trazando el código a mano, nunca «lo que suena razonable».)

Y las dos mitades del capítulo se dan la mano en el grafo demo de la Parte IV: el BFS streaming desde Ana produce niveles `[0], [1, 4], [2, 5]` (Dani inalcanzable: sólo su self-loop), y esos visitados son EXACTAMENTE los `alcanzados()` del Dijkstra sobre la proyección — misma componente, dos estrategias, una verdad (`bfs_en_demo_graph_alcance_y_contraste_con_dijkstra`).

**Las deudas, saldadas ante notario**: `dijkstra_proyeccion_coincide_con_dijkstra_store` (consistencia proyección↔algoritmo, la deuda del cap. 22) y `closeness_ponderado_paga_la_deuda_del_cap24` (el closeness ponderado, con consenso contra la versión por saltos cuando el peso es `Constant(1.0)` y CERO lecturas del store en los V Dijkstras). La Parte V cierra sin facturas pendientes.

¿Y si te saltas el capítulo? Síntoma detectable: tus analíticas tardan proporcionalmente a iteraciones (cada pasada relee), tus consultas locales tardan proporcionalmente al GRAFO, y no puedes demostrar ninguna de las dos cosas porque no tienes voltímetro.

## 26.10 Repaso de la Parte V: la cadena 22→26

Este capítulo cierra la Parte V. Reconstruyamos el árc completo — cada capítulo ejecutó el algoritmo académico del Vol.I SOBRE el store persistente, y cada uno descubrió que los datos viven en el disco:

```
 22 CAMINOS MÍNIMOS ──► 23 A* ──► 24 CENTRALIDAD ──► 25 COMUNIDADES ──► 26 SIN AGOTAR MEMORIA
    pesos de PROPS        heurística    V iteraciones         K niveles        la estrategia:
    del EDGE, semántica   del NODE      → proyección          → grafo          MATERIALIZE o
    ESTRICTA (fail loud)  (coords km)   PRIVADA sin pesos     simétrico        STREAM según consulta
    │                     │             + índice denso        propio           └─ la API pública
    └─ deuda: CSR no pesa └─ refactors  └─ deuda: ponderado  └─ decisión:     que las salda todas
       (cap. 14)             compartidos    y proyección        CONVIVIR         + el voltímetro
                           (validate_     pública               (no unificar)
                            edge_weights)
```

Cada eslabón dejó una garantía heredada: el **22** fijó el contrato de calidad de pesos (estricto, ruidoso) y anotó que el CSR del 14 no podía pesarlo; el **23** descubrió que los algoritmos también necesitan datos del NODO, y extrajo `validate_edge_weights` compartido; el **24** tropezó con la K-iteración y se construyó la primera proyección — privada, sin pesos, con el índice denso que aquí reutilizas — dejando apuntado «cuando exista la proyección con pesos, el BFS de saltos se cambia por Dijkstra»; el **25** decidió CONVIVIR (su `GrafoPonderado` simétrico codifica el contrato de Louvain; unificar a la fuerza sería re-asegurar 597 tests por cero valor). Y el **26** reúne: hereda el layout del 14, el contrato del 22, el índice denso del 24, la sabiduría de convivencia del 25 — y añade lo que ninguno tenía: la decisión explícita de qué existe en memoria, y el instrumento para demostrarla. El método de la Parte V en una frase: **el algoritmo es el fácil; lo difícil es ejecutarlo sin mentir sobre los datos ni quebrar la memoria.**

Estas dos piernas — materializar con filtro y streamear con presupuesto — son también las que sostendrán el futuro inmediato del libro: en el Vol.III, el cap. 51 montará GraphRAG sobre exactamente esto (PageRank personalizado multi-hop sobre proyecciones de una base de conocimiento que no cabe en memoria, con fronteras acotadas por presupuesto). Antes, la Parte VI contestará la pregunta que este capítulo deja abierta a propósito: ¿quién garantiza que la FOTO fue coherente?

## 26.11 Qué hemos sacrificado

1. **Paralelismo real, documentado no implementado**: `bloques_de_nodos(tam)` reparte rangos de posiciones — cada bloque un slice CSR independiente, perfectamente divisible entre hilos. Pero `&dyn GraphStore` no es `Sync` y el workspace no usa crates. La semilla queda; cómo lo paralelizan GDS y Kùzu es prosa de §26.12.
2. **Snapshot sin aislamiento transaccional**: la proyección es inmutable, pero ¿fue coherente el instante fotográfico? Nada lo garantiza aún: eso exige transacciones (Parte VI) y MVCC (cap. 30).
3. **Proyección dirigida-out fiel**: paralelas y self-loops se conservan tal cual (la proyección FOTOGRAFÍA, no interpreta). Quien necesite simetría o unión, la construye encima — como hicieron los caps. 24/25.
4. **El presupuesto cuenta `get_edge`, no `iter_edges`**: la validación eager del cap. 22 escapa al voltímetro (por eso 45, no 55). Contar el catálogo entero de accesos difiere el instrumento; documentado.
5. **Convivencia antes que pureza**: tres estructuras de proyección coexisten (la pública de aquí, la del 24, la del 25). El refactor unificador queda como deuda declarada, no oculta.

## 26.12 Cómo lo hace una BBDD real

- **Neo4j GDS**: `gds.graph.project()` materializa una proyección EN MEMORIA (native projection por configuración, o Cypher projection por consulta) con exactamente nuestros tres ejes: qué nodos, qué tipos de relación, qué propiedades. Vive en un graph catalog con estimación de memoria previa (`estimate`) y se suelta con `gds.graph.drop`. Es nuestra `ProyeccionPonderada` con catálogo y factura de RAM.
- **Kùzu** invierte el diseño: almacenamiento COLUMNAR (CIDR 2023). Las propiedades viven en columnas, de modo que una analítica que necesita 2 propiedades de 20 lee DOS columnas del disco — nada que copiar a RAM. Kùzu hace barato lo que nosotros materializamos; nosotros materializamos lo que Kùzu hace barato.
- **Pregel/Giraph**: materializar y iterar por superpasos BSP (Valiant 1990) — la proyección repartida entre máquinas, frontera de mensajes en cada barrera. Facebook escaló Giraph a billones de aristas.
- **GraphChi** (OSDI 2012): el streaming extremo — parallel sliding windows procesan el grafo POR BLOQUES desde el disco de un PC, sin cargarlo nunca entero. La estirpe de nuestro `FronterasBfs` bajo presupuesto de lecturas.
- **GraphX** (OSDI 2014): unifica grafo-paralelo y dato-paralelo en Spark, reinterpretando los superpasos como joins distribuidos — la prueba de que las dos familias eran dos vistas del mismo problema.
- **DuckDB**, fuera del mundo grafo: su buffer manager derrama a disco temporal cuando la consulta no cabe en RAM (out-of-core). Nuestro `Presupuesto` es su primo pequeño: en vez de reaccionar al desbordamiento, lo prohíbes de antemano.
- **El survey del guion**: (a) *paralelismo* — las fronteras y los bloques CSR son divisibles; GDS y Kùzu paralelizan exactamente por bloques de nodos; (b) *snapshots* — la foto inmutable es la separación OLTP/analítica encarnada; el paso siguiente es MVCC (cap. 30); (c) *OLTP vs analítica* — el punto (`get_edge` sobre el vivo) contra el recorrido (K pasadas sobre la foto): dos cargas, dos estrategias, una base de datos que ofrece ambas.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: en el test de la Persona-only, ¿por qué `descartadas` es 1 si DOS aristas (1→4 y 4→0) quedan fuera de la proyección?
- *Intermedio*: ¿por qué el presupuesto de lecturas se comprueba DENTRO del bucle de aristas y no por frontera? Construye un caso donde la diferencia sea visible.
- *Experto*: tu `Presupuesto` no distingue lecturas de PÁGINA (cap. 13) de lecturas de arista. ¿Qué mediría un voltímetro de páginas sobre un BFS frontera a frontera, y por qué la localidad del CSR lo favorece?

## 26.13 Lo que te llevas

- **Dos estrategias, no una**: K iteraciones sobre todo el grafo → materializar UNA vez (E lecturas); consulta local a k saltos → streaming frontera a frontera (k adyacencias). El tipo de consulta decide.
- **La proyección es el CSR del cap. 14 completado**: mismo layout, más el peso y el id de arista que el disco no podía pagar; índice denso compactando huecos; determinismo total.
- **Filtrar aquí es no leer**: las aristas de nodos excluidos NI SE LEEN (ahorro medible en `edges_scanned`/`descartadas`).
- **La sanidad de pesos se paga UNA vez**: semántica estricta heredada del 22, validación eager en `closeness_ponderado` FUERA del bucle de orígenes — 11 lecturas que valen para 5 consultas (45 de la vía directa).
- **Un Iterator, no un Vec**: la frontera k+1 no existe hasta que la pides; soltarlo a mitad corta la factura.
- **`MotivoParada` es parte de la respuesta**: `Completo` y `PresupuestoNodos` son recorridos con el mismo aspecto y significados opuestos.
- **El voltímetro externo**: dos fuentes independientes (stats interna + `ContandoStore`) que deben coincidir; nunca confíes en el autoinforme.
- **BitSet donde es denso, HashSet donde es disperso**: un bit por id si los ids nacen contiguos; tabla hash si las claves son arbitrarias.

## 26.14 Ojo, cuidado con…

- **Interpretar sin mirar `parada`**: tres nodos visitados puede ser la componente entera o el corte del presupuesto. Primero `parada`, después conclusiones.
- **Calibrar contadores de oído**: profundidad k ⇒ expandir k nodos (2 aristas leídas, no 3). Los tests de contadores se calibran trazando el código a mano.
- **Presupuestos a posteriori**: comprobar el límite tras la lectura permite superarlo de a una — el chequeo va ANTES de cada `get_edge` y de cada descubrimiento.
- **Bitset con ids dispersos**: ids gigantes y huecos pagan palabras vacías; ahí el HashSet gana. El bitset es denso o no es.
- **Confundir la proyección con una vista viva**: es una FOTO. El store siguió mutando; tu analítica contesta sobre el instante de la foto, no sobre el ahora.

## 26.15 Pin de batalla

> *«Un resultado recortado por presupuesto no es una respuesta a medias: es una promesa a medias. Sin MotivoParada, ni siquiera sabes cuál de las dos te dieron.»*

## 26.16 Si solo lees 30 segundos

Materializar (`ProyeccionPonderada`) paga E lecturas UNA vez y luego itera K algoritmos con CERO lecturas del store — 11 contra 45 en la cadena de 12. Streamear (`FronterasBfs`) lee frontera a frontera bajo demanda: profundidad 2 en la cadena de 500 lee 2 aristas de 499. El `Presupuesto` (profundidad/nodos/lecturas) nunca se supera porque se comprueba antes de leer, y `MotivoParada` dice si tu respuesta está completa o recortada. El `ContandoStore` verifica todo desde fuera: dos fuentes independientes, ninguna se auto-audita. BitSet si los ids son densos; HashSet si las claves son dispersas.

## 26.17 Una historia pequeña

La migración de este capítulo casi se pierde. El agente que escribía el módulo quedó cortado por usage-limit con las 2.150 líneas COMPLETAS pero sin cablear en `lib.rs`, sin compilar, y con cuatro tests mal calibrados. El orquestador lo terminó a mano: dos errores de compilación (`sort_unstable` no ordena `f64` — se necesita `total_cmp`; y el `#[derive(Debug)]` no sabe imprimir un `&dyn GraphStore` — impl manual con los contadores), cuatro lints, y luego lo bueno: los cuatro tests de contadores que fallaban.

Uno decía `5·E = 55` lecturas para 5 Dijkstras; el voltímetro decía 45. ¿Quién mentía? Nadie: la validación eager del cap. 22 usa `iter_edges` — que el voltímetro no cuenta como `get_edge` — y cada origen i lee E−i aristas: 11+10+9+8+7. Otro decía que el BFS de profundidad 2 leía 3 aristas; la traza de `FronterasBfs::next` a mano demostró que se EXPANDEN 2 nodos, no 3 — descubrir el nodo 2 no exige expandirlo. Las expectativas se recalibraron una a una, cada una con su derivación al lado. La moraleja quedó escrita en la bitácora de migración: los tests de contadores no se calibran con lo que suena razonable; se calibran trazando el código. El voltímetro no opina: suma.

## Ejercicios resueltos

**1. En el filtro Persona-only de §26.6, ¿por qué `descartadas == 1` si DOS aristas quedan fuera de la proyección (1→4 y 4→0)?**

Porque `descartadas` cuenta lo que se LEYÓ y se descartó, no todo lo que queda fuera. La arista 1→4 se lee (la adyacencia del nodo 1 — admitido — se itera) y se descarta: su destino (la ciudad) no pasa el filtro de nodos. La arista 4→0 NI SE LEE: su origen (la ciudad) está excluido y `proyectar` jamás consulta su adyacencia — por eso `edges_scanned == 2` (las adyacencias de los dos nodos admitidos) y la 4→0 no aparece en ninguna stats. El ahorro del subgrafo: no pagar lecturas de lo que ni entra. Verificación: `subgrafo_filtrado_por_label_y_tipo_de_arista`.

**2. ¿Por qué 5 Dijkstras del cap. 22 sobre la cadena de 12 son 45 lecturas y no 55 (5×11)?**

Dos razones que hay que separar. Primero, no cada Dijkstra lee las 11: el origen i sólo expande los nodos i..11, luego lee 11−i aristas — la suma es Σ(E−i) = 11+10+9+8+7 = 45. Segundo, la validación eager de pesos del cap. 22 recorre `iter_edges` (un iterador del store), que el voltímetro NO cuenta como `get_edge` — por eso no suma ni una lectura más. Con la proyección: 11 lecturas de materialización y las 5 consultas no tocan el store. Verificación: `economia_multiorigen_una_lectura_por_adelantado` (y su comentario con la derivación exacta).

## Ejercicios propuestos

**Esencial (recordar/aplicar).** Sin ejecutar nada, sobre la estrella 0→{1,2,3,4} (cuatro aristas 0→i) con `Presupuesto::sin_limite().con_nodos(3)`: predice `niveles`, `nodos()`, las cuatro stats (`nodos_visitados`, `aristas_leidas`, `adyacencia_consultas`, `fronteras`) y el `MotivoParada`. Luego verifica con `bfs_streaming` envuelto en un `ContandoStore`: stats interna y voltímetro deben coincidir. Repite con el origen aislado y presupuesto 1. *Pistas*: (1) ¿el límite de nodos se comprueba antes o después de descubrir?; (2) ¿cuántas aristas se leen hasta descubrir la segunda hoja?; (3) ¿qué `MotivoParada` espera a un grafo que ya no tiene nada? *Criterio*: predicción exacta de niveles+parada y coincidencia stats/voltímetro (compárate con `bfs_streaming_presupuesto_nodos_exacto`).

**Intermedio (analizar — mezcla caps. 22 y 24).** La cadena 0→1→2→3 con pesos 1, 5, 1. (a) Calcula a mano el closeness ponderado de los cuatro nodos (Wasserman-Faust con Σd ponderado). (b) Explica por qué 0 y 1 se devalúan respecto a la versión por saltos y el 2 no cambia. (c) ¿Qué error tipado esperas con pesos negativos, quién lo lanza y CUÁNTAS veces valida `closeness_ponderado`? Verifica con `closeness_ponderado_paga_la_deuda_del_cap24` (allí están los valores 3/14, 4/33, 1/3, 0 y el test de economía con voltímetro). *Pistas*: (1) ¿qué Σd y qué r para cada origen?; (2) ¿qué aristas toca el mundo alcanzable del 2?; (3) ¿dónde corre `validar_pesos_no_negativos` respecto al bucle de orígenes? *Criterio*: números exactos + la diferencia semántica salto/peso + la validación UNA vez.

**Experto (crear — cierre de Parte V, retrieval puro).** Primera parte, de memoria (sin mirar los banners de los caps. 22-25): reconstruye el árc de la Parte V — qué añadió cada capítulo (22: pesos estrictos de props; 23: heurísticas del nodo; 24: familias + proyección privada e índice denso; 25: comunidades + grafo simétrico propio) y qué garantía o deuda dejó cada uno. Segunda parte: implementa `pagerank_proyeccion(&ProyeccionPonderada)` — damping validado en (0,1), masa dangling redistribuida uniformemente, convergencia L1 (todo lo aprendiste en el 24) — con CERO lecturas del store tras materializar (voltímetro en el test) y verifica que sus scores coinciden con el `pagerank` del cap. 24 sobre el mismo grafo simetrizado. *Pistas*: (1) ¿qué convención de self-loops y paralelas debes re-declarar para que la equivalencia sea justa?; (2) ¿dónde del bucle por iteraciones se escapa la masa dangling?; (3) ¿por qué aquí no hay validación de pesos DENTRO del bucle? *Criterio*: árc completo de memoria + equivalencia con el 24 + cero lecturas medidas.

## Para profundizar

- **Malewicz et al., «Pregel: a System for Large-Scale Graph Processing» (SIGMOD 2010, pp. 135-146, DOI 10.1145/1807167.1807184)** — el paper que bautizó «think like a vertex» y los superpasos. La anécdota de la esquina, en fuente primaria.
- **L. G. Valiant, «A Bridging Model for Parallel Computation» (CACM 33(8), 1990)** — el modelo BSP en el que Pregel se apoyó veinte años después.
- **A. Kyrola, «GraphChi: Large-Scale Graph Computation on Just a PC» (OSDI 2012)** — el streaming por bloques desde disco, en un solo PC: parallel sliding windows.
- **R. S. Xin et al., «GraphX: Unifying Data-Parallel and Graph-Parallel Analytics» (OSDI 2014)** — la unificación de las dos familias en Spark.
- **Neo4j Graph Data Science — docs de graph projection (native y Cypher), memory estimation y graph catalog** — la `ProyeccionPonderada` a escala industrial, con factura de RAM.
- **G. Jin et al., «Kùzu» (CIDR 2023) y Gupta et al., «Columnar Storage and List-based Processing for Graph Queries» (VLDB 2021)** — el camino columnar: leer poco en vez de copiar.
- **DuckDB, «Memory Management in DuckDB» (blog, 2024)** — out-of-core: derramar a disco cuando no cabe, el complemento reactivo de nuestro presupuesto preventivo.
- **McCune et al., «A Survey of Vertex-Centric Frameworks» (ACM Computing Surveys, 2015)** — el mapa de toda la familia TLAV/BSP.

## Mini-diálogo: a la puerta del archivo

> — Entonces, ¿materializo o streameo?
>
> — Pregúntale a la consulta, no a la moda. ¿Vas a recorrer el grafo K veces? Fotocópialo una vez. ¿Sólo quieres saber qué hay a dos saltos? Pide dos carpetas y di basta.
>
> — ¿Y si miento y me equivoco?
>
> — Para eso está el contador de la puerta. Tus stats dicen dos lecturas; el contador dice dos; bien. El día que digan cosas distintas, ya tienes el bug acorralado entre los dos.
>
> — ¿Y el presupuesto? Me da miedo cortar la respuesta a medias.
>
> — Al contrario: cortar a medias y SABERLO es lo honesto. `MotivoParada` es la diferencia entre «esto es todo» y «esto es lo que me dejaron ver». Lo peligroso no es el recorte: es el recorte sin etiqueta.

---

*(Próximo capítulo: 27 — Qué significa una transacción. Tu proyección es la foto de un instante… ¿pero quién garantiza que ese instante fue coherente, y que dos escritores no te rompan el store mientras analizas su foto? La Parte VI — ACID, WAL, recuperación — convierte la fe en contrato. Y más adelante, en el Vol.III, el cap. 51 montará GraphRAG sobre las piernas de éste: PPR multi-hop sobre proyecciones y fronteras de una base de conocimiento que no cabe en memoria.)*
