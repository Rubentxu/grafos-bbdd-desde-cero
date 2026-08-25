# CONTRATO DE CAPÍTULO — Vol.II Cap. 36: Arquitectura final de LiraDB

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla: TODO existe y está
> verde — `lib.rs` de `vol2-liradb` declara 28 módulos (`cap07_modelo` … `cap30_mvcc`,
> `cap32_import_export`, `pub mod cap33_pruebas`, `mod cap34_benchmarks`,
> `pub mod cap35_observabilidad`) re-exportados con API plana; el frontal es el paquete
> `liradb-cli` (binario `liradb`: subcomandos `demo/query/explain/repl/script/import/export`
> + flags `--graph/--plan/--stats/--profile`, verificado 2026-08-25); los puertos son
> EXACTAMENTE cinco traits: `GraphStore` (cap 8), `Pager` (cap 12), `PhysicalOperator`
> (cap 20), `WeightSource` (cap 22), `Heuristic` (cap 23); los adaptadores canónicos:
> `MemoryStore`, `FilePager`, `BufferPool`, `Csr`/`PersistentCsr`,
> `HashIndex`/`BPlusTree` (sobre `BufferPool<Pager>`), `Wal`/`WalTransaccion`,
> `MvccStore` (API propia, NO implementa `GraphStore`) y los decoradores de observabilidad
> (cap 35). Estado verificado 2026-08-25: **843 tests** ALL_GREEN; `vol2-liradb`
> dependency-free en runtime (dev-deps: `tempfile`, `proptest`, `criterion`);
> `tracing 0.1.44` SOLO en la CLI; goldens demo/explain byte-exactos intactos.
> Deuda documentada que el capítulo NOMBRARÁ sin reparar: `Catalog::collect` cuadrático
> (~224 s vs 281 ms, MIGRATION-PATTERN §39), sin `GraphStore` respaldado por disco
> end-to-end, LiraQL sin LIMIT/agregación (§39 hallazgo 6), write skew atraviesa el MVCC
> snapshot (cap 30). Este capítulo es SÍNTESIS: cierra la Parte VII recogiendo el gancho
> explícito del cap. 35 («el mapa completo del motor; los puertos medidos cierran el
> círculo hexagonal») y del epílogo del brief (Vec<Vec<Edge>> → motor completo).
> Pregunta crítica del CORPUS (`vol-II-cap-36`): **«Diagrama hexagonal final.»**
> Gancho saliente: Parte VIII (caps. 37-40) como crecimiento POST-mapa.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: TODOS los componentes pieza a pieza — modelo
  PropertyGraph/Value (7), puerto `GraphStore` (8), encoding/framing (9-10), slotted
  pages (11), `Pager`/`FilePager` (12), `BufferPool` (13), CSR (14), índices (15),
  mantenimiento (16), LiraQL (17), lexer/parser (18), plan lógico (19), Volcano (20),
  optimizador/`explain` (21), algoritmos sobre el puerto (22-26),
  ACID/WAL/recuperación/MVCC (27-30), CLI (31), import/export (32), torre de pruebas
  (33), benchmarks (34), contadores/trazas `--profile` (35).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «la arquitectura es el diagrama que se dibuja ANTES de codificar» — no: aquí es un
  mapa que se valida DESPUÉS contra el código existente; si el mapa y `lib.rs`
  discrepan, gana `lib.rs`.
  (2) «el diagrama vertical del brief (CLI→Parser→Planner→Optimizer→Executor→Storage) ES
  la arquitectura» — no: es UNA vista (el FLUJO de una consulta); falta la vista de
  DEPENDENCIAS; confundirlas lleva a preguntas mal planteadas y refactorings equivocados.
  (3) «un capítulo de arquitectura reorganiza código» — no: su entrega es comprensión
  VERIFICABLE; retocar módulos para «dejarlos bonitos» estropearía un motor sostenido
  por 843 tests.
  (4) «lo que aparece en el mapa funciona de punta a punta» — no: leer el mapa incluye
  leer sus agujeros (DiskStore pendiente, catálogo cuadrático, sin LIMIT, write skew);
  un mapa sin fronteras es marketing, no arquitectura.
- **Pregunta crítica que el capítulo tiene que responder**: «Diagrama hexagonal final.»
  Respuesta: el hexágono de Cockburn (ya citado en cap 8) dibujado COMPLETO — dominio
  puro en el centro, cinco puertos, adaptadores alrededor, frontal CLI fuera — CUADRADO
  uno a uno con los 28 módulos de `lib.rs` + la CLI, reconciliado con la vista de flujo
  del brief y DEMOSTRADO con un test que construye la torre completa de una vez.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `crates/vol2-liradb/tests/arquitectura.rs` (integración, std puro, dev-dep `tempfile` existente; CERO cambios en módulos `cap*`): el «smoke de arquitectura» — UN test que construye la torre completa en una respiración: `MemoryStore` como `dyn GraphStore` → grafo mini → `Wal` + `WalTransaccion::commit` (delta de bytes) → `MvccStore` snapshot → pipeline texto→`parse`→`lower`→`optimize`→`Executor`→`ResultSet` → `Contadores`/derivación de `ExecMetrics` (35); y un segundo test que baja por la pila física: `FilePager(tempfile)` → `BufferPool<FilePager>` → `Csr::replace/load` + `HashIndex`/`BPlusTree` | `cargo test -p vol2-liradb --test arquitectura`: tesis `la_torre_completa_se_construye_en_una_respiracion`, `la_pila_fisica_encadena_pager_pool_csr_e_indices` |
| Test de INVENTARIO: cada puerto existe y su adaptador canónico se instancia TRAS él (aserciones de tipos + construcción real, no reflexión): `GraphStore`↔`MemoryStore`, `Pager`↔`FilePager`, `PhysicalOperator`↔operadores del `Executor`, `WeightSource`↔`dijkstra`, `Heuristic`↔`a_star` | `cada_puerto_tiene_su_adaptador_canonico`; fallaría al compilar si un trait renombrara o desapareciera |
| TABLA módulo↔rol-arquitectónico (deliverable prosa): 28 módulos de `lib.rs` + paquete `liradb-cli`, cada fila con rol (dominio/puerto/adaptador/pipeline/algoritmos/fiabilidad/frontera-E-S/calidad/frontal), verificada contra `code-map.yml` | Revisión cruzada fila a fila contra `lib.rs` real; ningún módulo sin fila; ninguna fila sin módulo |
| Diagrama hexagonal FINAL (ASCII, convención del libro) + vista de flujo vertical del brief RECONCILIADAS como dos vistas legítimas | Todo nombre del diagrama es grepeable en el workspace; el flujo coincide con `pipeline_con_detalle` de la CLI |
| Relato de las TRES decisiones estructurales con evidencia MEDIDA — puerto (cap 8), encoding/framing estables (caps 9-10, reutilizados por WAL cap 28), pipeline (caps 17-21) — MÁS la sección de deuda técnica y «qué haríamos diferente» del brief (límites, dependencias, decisiones, deuda) | Números citados del cap. 34/MIGRATION §39 (CSR vs puerto ×16, hub ×794, catálogo ~224 s); cada ítem de deuda referenciado a MIGRATION §39/§40 o a código concreto (`out_edges()` clona Vec) |
| ALL_GREEN intacto | `./scripts/verify.sh` → 843 + N tests nuevos; fmt/clippy limpios; goldens sin regenerar |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Diagrama hexagonal final.» — el capítulo la convierte en **respuesta en
nueve pasos**:

1. **Por qué un mapa AHORA**: treinta y cinco capítulos mirando LA PIEZA; ninguno mirando
   EL EDIFICIO. Epitafio de Wren: *si buscas su monumento, mira alrededor* — el monumento
   de este libro es `lib.rs`, y este capítulo enseña a LEERLO.
2. **El nivel de zoom correcto** (modelo C4 de Simon Brown): ni el pájaro (contexto) ni
   el gusano (clases/líneas): el nivel COMPONENTE — piezas y dependencias.
3. **El centro**: dominio puro SIN dependencias — `Value`/`Node`/`Edge`/`PropertyGraph`
   (cap 7) y el AST de LiraQL (cap 17). Compila solo; es lo que hace al motor TESTEABLE.
4. **El primer anillo — los CINCO puertos**: `GraphStore` (todo acceso a datos),
   `Pager` (todo acceso a páginas), `PhysicalOperator` (toda producción de filas),
   `WeightSource` y `Heuristic` (los dos puntos donde el USUARIO aporta conocimiento del
   dominio). Cinco traits sostienen veintinueve capítulos.
5. **El segundo anillo — adaptadores**: almacenamiento (`MemoryStore`, `FilePager`,
   `BufferPool`, `Csr`, `HashIndex`/`BPlusTree`), fiabilidad (`Wal`/`WalTransaccion`,
   recuperación, `MvccStore`), formato/E-S (encoding, `AppendOnlyLog`, slotted pages,
   mantenimiento, CSV/JSONL/GraphML), observabilidad (decoradores del cap 35). Regla
   heredada: cada adaptador vive DETRÁS de un puerto; ninguno se conoce entre sí.
6. **El transversal — el pipeline**: parse(18) → lower(19) → optimize(21) → execute(20)
   es una MÁQUINA montada sobre el puerto `GraphStore`, no una capa del hexágono: cruza
   todos los anillos en cada consulta. Aquí se reconcilia con el diagrama vertical del
   brief: esa vertical es el FLUJO; el hexágono son las DEPENDENCIAS.
7. **El frontal y las costuras de calidad**: la CLI es el único habitante externo; torre
   de pruebas (33), benchmarks (34) y observabilidad (35) no son capas: son LENTES sobre
   el mismo motor — por eso pudieron añadirse sin tocar nada.
8. **Inventario uno a uno**: la tabla módulo↔rol cierra la brecha entre dibujo y código;
   si un módulo real no tiene fila, el mapa está MAL (misma honestidad que el recibo del
   cap 35: una sola verdad).
9. **El relato y la frontera**: tres decisiones (puerto, formato, pipeline) explican POR
   QUÉ aguantó 29 capítulos de crecimiento; la deuda visible explica QUÉ falta para que
   sea otra cosa; y el mapa queda como BASE del crecimiento futuro (Parte VIII).

---

## 4. La arquitectura: un EDIFICIO terminado se lee en dos VISTAS

Modelo mental único: **el monumento y sus dos vistas**. La vista RADIAL (hexágono de
Cockburn) responde «¿quién depende de quién?» — la vista de la CONSTRUCCIÓN y de la
evolución. La vista VERTICAL (brief) responde «¿por dónde pasa una consulta?» — la vista
del USO. Con una sola, no sabes ni defender el edificio ni ampliarlo.

```
                ┌─────────────────────────────────────────────┐
                │ FRONTAL: paquete liradb-cli (caps 31-35)    │
                │ demo query explain repl script import       │
                │ export · --graph --plan --stats --profile   │
                └─────────────────────┬───────────────────────┘
                                      │
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ ADAPTADORES — viven DETRÁS de un puerto                                  │
 │ almacenamiento: MemoryStore(8) · FilePager(12) · BufferPool(13)          │
 │                 Csr(14) · HashIndex/BPlusTree(15)                        │
 │ fiabilidad:     Wal/WalTransaccion(28) · recuperación(29) · MvccStore(30)│
 │ formato/E/S:    encoding(9) · AppendOnlyLog(10) · SlottedPage(11)        │
 │                 inspect/check/compact(16) · CSV/JSONL/GraphML(32)        │
 │ observación:    Contadores · MedidorOperador · MedidorPaginas(35)        │
 └────────────────────────────────────┬─────────────────────────────────────┘
                                      │ implementan / envuelven
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ PUERTOS — traits                                                         │
 │ GraphStore(8) · Pager(12) · PhysicalOperator(20) ·                       │
 │ WeightSource(22) · Heuristic(23)                                         │
 └────────────────────────────────────┬─────────────────────────────────────┘
                                      │
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ DOMINIO — compila sin depender de nadie                                  │
 │ Value · Node · Edge · PropertyGraph(7) · LiraQL AST(17)                  │
 └──────────────────────────────────────────────────────────────────────────┘

 TRANSVERSAL (flujo de una consulta — la vista vertical del brief):
 texto → Parser/Lexer(18) → lower·LogicalPlan(19) → optimize·Catalog(21)
       → Executor·Volcano(20) → ResultSet    [sobre el puerto GraphStore]
 ALGORITMOS sobre el puerto (22-26) · CALIDAD como lentes: pruebas(33),
 benchmarks(34), observabilidad(35)
```

Momento ¡ajá! perseguido: «la arquitectura NO fue un diagrama previo que cumplimos: fue
TRES decisiones mantenidas con disciplina durante 29 capítulos — el mapa no dibuja lo
que se prometió, dibuja lo que AGUANTÓ». Y el ¡ajá! honesto: el anillo físico está
construido y probado, pero la ruta de consultas hoy vive en RAM — decirlo es parte de
leer el mapa.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Capítulo de SÍNTESIS: cero cambios en módulos `cap*`, cero features nuevas | El valor del capítulo es COMPRENSIÓN verificable; tocar el motor «para dejarlo bonito» invalidaría 843 tests como argumento de estabilidad — justo lo que el mapa celebra | Refactoring cosmético de cierre (renombrar, reagrupar módulos): riesgo puro sin aprendizaje nuevo | Brief cap. 36 («se revisan», no «se modifica»); contratos cap-33/34/35: honestidad > cosmética |
| 2 | SÍ hay UN artefacto nuevo: `tests/arquitectura.rs` (smoke de arquitectura) | El mapa mapea CÓDIGO: el método del libro exige que compile contra lo mapeado; NINGÚN test previo construye toda la pila en una respiración — cada capítulo probó SU piso, nunca el edificio entero; da bucle de feedback al reto de creación | (a) Cero código (precedente caps 1-6): el mapa quedaría «confía en mí»; (b) parser estático de dependencias: Rust ya lo garantiza por visibilidad/módulos privados — duplicar al compilador es frágil e inútil | Prime directive (CONVENTIONS §6: el código es el centro); plantilla §11: feedback inmediato vía `cargo test` |
| 3 | El test es INTEGRACIÓN (`tests/arquitectura.rs`), no módulo nuevo de lib | No ensucia la API pública ni `lib.rs`; es aditivo por construcción y refleja el rol del capítulo: mirar el edificio desde FUERA | Nuevo módulo `cap36_*` en lib.rs: un capítulo de síntesis no añade superficie de librería | Estructura real de `vol2-liradb/tests/`; decisión espejo de la CLI (tests de integración propios) |
| 4 | DOS vistas reconciliadas: radial-hexagonal (dependencias) + vertical-brief (flujo) | Son preguntas distintas («¿quién necesita a quién?» vs «¿por dónde pasa una consulta?»); el diagrama único del brief responde solo la segunda — y llevó al lector a pensar que Storage «está debajo» del Executor cuando en realidad el Executor habla con un TRAIT | Un único diagrama canónico: cualquier vista sola deja preguntas sin respuesta y produce refactorings equivocados | Modelo C4 de Simon Brown (c4model.com): niveles/vistas separadas para audiencias separadas; diagrama del brief, brief-liradb-original.md:1503 |
| 5 | Nivel de zoom COMPONENTE (ni clases ni contexto) | 29 módulos y 5 traits caben en una página; clases de Rust (impl blocks, genéricos) hundirían al lector en ruido sin añadir dependencias visibles | Diagrama de clases UML completo: nivel CODE de C4 — útil para quien MODIFICA una clase, no para quien aprende el edificio | Brown, «C4 model for software architecture» (c4model.com); CONVENTIONS §3 (diagramas ASCII) |
| 6 | Vocabulario hexagonal de Cockburn como idioma oficial del mapa | Ya es EL marco del libro desde el cap. 8 (puerto/adaptador); cambiar de marco en el capítulo final rompería la continuidad pedagógica | Redibujar en capas horizontales «clean architecture»: sin ganancia, colisión con 27 capítulos de vocabulario ganado | Cockburn, «Hexagonal Architecture (Ports and Adapters)» (2005, citado en cap. 8) |
| 7 | Tabla módulo↔rol como DELIVERABLE verificable (no adorno) | Convierte el mapa en contrato comprobable: 28 filas + CLI, una por módulo REAL de lib.rs; incluye la honestidad de que cap 31 vive en la CLI, no en la lib | Dibujo sin inventario: el lector no puede distinguir mapa fiel de mapa aspiracional | lib.rs verificado 2026-08-25; code-map.yml como segunda fuente del inventario |
| 8 | Las TRES decisiones estructurales como columna narrativa (puerto cap 8; encoding/framing caps 9-10; pipeline caps 17-21) | Explican los TRES tipos de crecimiento sin rupturas: algoritmos enchufados al puerto (22-26), WAL reutilizando framing+CRC32 (28), observabilidad envolviendo puertos (35) — cada una con evidencia MEDIDA del cap 34 | Narrar «una decisión por parte»: diluye el mensaje; el lector debe salir con TRES ideas, no veintitrés | Doc-comment de cap28 en lib.rs (framing cap 10 + encoding cap 9 reutilizados); MIGRATION §39 (CSR vs puerto ×16, hub ×794) |
| 9 | Deuda NOMBRADA, jamás reparada en este capítulo | Continuidad con caps. 33-35: el mapa marca fronteras (catálogo cuadrático ~224 s, sin DiskStore end-to-end, LiraQL sin LIMIT/agregación, write skew); fingir que no están sería la falsedad que el libro prohíbe | «Arreglar rápido el catálogo antes de cerrar»: scope creep clásico — el capítulo mapea, no repara (MIGRATION §39 la deja como candidata) | MIGRATION-PATTERN.md §39/§40; contratos cap-33 §5 y cap-34 §5.7 (frontera frío/caliente) |
| 10 | Sección «qué haríamos diferente» con casos CONCRETOS medidos | La lista del brief lo pide; lo abstracto no enseña: `out_edges()` clona un Vec por llamada (×16 medido — justifica las proyecciones del cap 26), autocommit implícito hasta el cap 27, `eq_push` lineal | Lista genérica de «lecciones aprendidas» sin código: no verificable, no memorable | MIGRATION §39 hallazgos 1/3; código cap08 (`out_edges`), cap21 (`eq_push`) |
| 11 | Anécdota ÚNICA: Christopher Wren y las 52 iglesias + epitafio | Tras el Gran Incendio (1666) reconstruyó 52 iglesias con UN lenguaje común (iglesia-auditorio: nave clara, tribunas, torre-aguja) — muchas obras, UNA arquitectura; y su epitafio en San Pablo («lector, si buscas su monumento, mira alrededor») es EXACTAMENTE el gesto del capítulo: el mapa invita a mirar alrededor de `lib.rs`. DESCARTADA Brunelleschi: ocultó deliberadamente cómo se sostiene su cúpula — el mensaje contrario a un mapa público (y su lección «diseñar anticipando la construcción» ya la enseñó el cap. 8); Torre de Babel: no técnica | — | Epitafio en St Paul's Cathedral, Londres (1723, fuente primaria visitable); Adrian Tinniswood, «His Invention So Fertile: A Life of Christopher Wren» (2001); National Churches Trust: «Wren rebuilt 52 churches in the City of London» |
| 12 | Sin golden nuevo; puerta de calidad = `--test arquitectura` + ALL_GREEN | Lo determinista ya está cubierto (goldens caps. 31-35); el test nuevo es determinista por naturaleza (sin tiempos, sin hilos) y el resto del workspace NO cambia | Golden del árbol/arquitectura impresa: duplicaría lo que ya protegen los goldens existentes | Política de determinismo del workspace; cap-35 §5.11 |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: Londres, 1666: un incendio borra la
   ciudad; Wren levanta 52 iglesias en décadas con un lenguaje común — y pide como
   epitafio que no se busque su monumento en piedra sino ALREDEDOR. Pregunta: ¿sabrías
   mirar alrededor de LiraDB y señalar qué la mantiene en pie?
2. **N.1-N.2 Objetivo/Problema**: sabes cada pieza; nadie ha visto todavía el edificio.
   Síntoma detectable: explicas el WAL pero dudas si `Catalog` depende de `Pager`; ante
   «¿dónde meterías ORDER BY?» no sabes por dónde empezar a responder.
3. **N.3 Modelo mental**: el monumento y sus dos vistas (§4): hexágono completo +
   vertical de flujo + reglas de lectura de cada vista.
4. **N.4 Primera solución**: intentar describir LiraDB con UNA lista de ficheros o con
   el diagrama vertical del brief — versiones ingenuas que responden «qué hay» pero no
   «quién depende de quién» ni «por qué aguanta».
5. **N.5 Sus límites**: la lista no distingue puerto de adaptador; la vertical sugiere
   dependencias físicas inexistentes (Executor→Storage) y oculta las reales (Executor→
   trait GraphStore).
6. **N.6 Solución evolucionada**: hexágono de Cockburn completado con los cinco puertos
   REALES; adaptadores agrupados por función; pipeline transversal; frontal y lentes de
   calidad. Cada anillo validado contra módulos concretos.
7. **N.7 Código completo ejecutable**: la tabla módulo↔rol (28 + CLI) por `include::`/
   referencia a `lib.rs` y `code-map.yml` (nunca duplicada); `tests/arquitectura.rs`
   incluido y comentado por bloques — el único código nuevo del capítulo.
8. **N.8 Prueba de fuego**: `cargo test -p vol2-liradb --test arquitectura` en verde =
   la torre completa construida de una respiración; después el recorrido guiado: una
   consulta `--profile` del cap. 35 RELEÍDA como viaje por el hexágono (cada span es un
   anillo).
9. **N.9 Qué hemos sacrificado**: el mapa no sustituye a ningún capítulo (referencia,
   nunca repite); no cubre despliegue/procesos (cap. 37); nombra deuda sin repararla;
   el nivel página sigue fuera de la ruta de consulta (frontera de 33-35).
10. **N.10 Cómo lo hace una BBDD real + retos**: PostgreSQL (arquitectura documentada en
    niveles: backend/parser/planner/executor/storage), SQLite (b-tree + pager + vdbe:
    el mapa canónico de un proceso único), Neo4j/Kùzu (VLDB paper, atribución
    clean-room); retos: **esencial** (recordar/aplicar: dibujar DE MEMORIA el hexágono y
    colocar 10 componentes — `MemoryStore`, `BPlusTree`, `WalTransaccion`, `IndexSeekOp`,
    `EuclideanHeuristic`, `SuscriptorArbol`, …— sin mirar §4; autocorrección contra el
    diagrama), **intermedio** (analizar: ¿qué ROMPERÍA si `GraphStore` expusiera páginas?
    — acoplaría los 12+ consumidores del puerto al formato físico: CSR, índices y
    algoritmos dejarían de compilar contra un store en RAM; conecta con cap 14, que baja
    a páginas detrás de SU propia costura), **experto** (crear: añade un componente
    IMAGINARIO al mapa — p. ej. `DiskStore` u `OrderByOp` — lista qué puertos toca, qué
    módulos vecinos usa, qué NO puede tocar y cómo extenderías `tests/arquitectura.rs`
    para afirmarlo).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla / 30 segundos /
    Una historia pequeña / Mini-diálogo de guardia nocturna (3 a.m.: «¿puedes dibujarme
    el sistema?» — y esta vez el pager de guardia DIBUJA el hexágono de memoria antes de
    abrir un solo log). Retrieval practice obligatorio (reto esencial). Interleaving
    masivo DECLARADO: el capítulo entero ES spacing — ejercita caps. 7-35; el reto
    intermedio clasifica elementos de caps. aleatorios en la capa correcta
    (CRC32→formato, `ExpandOp`→pipeline, Louvain→algoritmos-sobre-puerto,
    `MedidorPaginas`→observación). Glosario nuevo: arquitectura (software architecture),
    vista (view), diagrama de componentes (component diagram), frontera (boundary),
    deuda técnica (technical debt), nivel C4 (context/container/component/code).
12. **Gancho de cierre (preguntas abiertas)**: el mapa está completo — y un mapa sirve
    para CRECER con criterio. Parte VIII: ¿qué necesitaría LiraDB para producción (37)?
    ¿dónde se enchufaría el columnar/vectorizado (38)? ¿joins peor-caso-óptimos y
    recursivos (39)? ¿qué partes del mapa sobrevivirían a repartir el motor entre
    máquinas (40)? Cada capítulo empieza señalando un punto del hexágono.

---

## 7. Estilo y tono (consistencia con caps. 27-35)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (architecture, view, boundary, component diagram, technical
  debt); NINGUNA salida falsificada: todo nombre de módulo/test citable del workspace;
  los números medidos (×16, ×794, ~224 s) citados con su origen (cap. 34, MIGRATION §39).
- **Diagramas**: el hexágono completo (§4), la vertical del brief como segunda vista,
  la tabla módulo↔rol como inventario; bloque «lo que SÍ / lo que aún no» idéntico en
  espíritu al de caps. 33-35.
- **Spacing**: el capítulo ES spacing institucionalizado — cada sección referencia el
  capítulo que introdujo la pieza; el reto esencial obliga a RECUPERAR de memoria
  componentes de media docena de capítulos distintos.
- **Interleaving**: reto intermedio mezcla caps. aleatorios (formato/pipeline/
  algoritmos/observabilidad); el experto combina diseño (mapa), código (test nuevo) y
  juicio (fronteras).
- **Dificultad asimétrica**: UNA idea nueva por sección (vista radial → vista vertical →
  puertos → adaptadores → transversal → frontal → inventario → tres decisiones → deuda);
  la dificultad concentrada en los ejercicios (recordar sin pistas, decidir fronteras).
- **Bucle de feedback**: `cargo test -p vol2-liradb --test arquitectura` (milisegundos) +
  `./scripts/verify.sh` ALL_GREEN como puerta final del volumen de código. Nunca
  «confía en mí».
- **Anécdota (única, verificada)**: Christopher Wren (1632-1723): 52 iglesias tras el
  Gran Incendio de Londres (1666) con un lenguaje común, y el epitafio de San Pablo
  («lector, si buscas su monumento, mira alrededor», 1723). DESCARTADAS: Brunelleschi
  (secretismo constructivo: anti-mapa; su lección ya pertenece al cap. 8), Torre de
  Babel (no técnica), Apollo 1202 (reservada, monitorización ≠ mapa). Apoyo técnico:
  Cockburn (hexagonal, ya citado cap. 8), Simon Brown C4 model (c4model.com), código
  real de `lib.rs`/CLI verificado 2026-08-25.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5: C4/c4model.com, Cockburn, epitafio de Wren/Tinniswood, lib.rs y
  MIGRATION §39/§40 verificados).
- [x] Escenario de fallo visible: mapa que no cuadra con lib.rs (misconcepción 1 y paso
  8), vista única que induce refactorings equivocados (§5.4), costo de violar el puerto
  (reto intermedio N.10), deuda nombrada-no-fingida (§5.9).
- [x] Código ejecutable citado por nombre: `tests/arquitectura.rs` (ÚNICO artefacto
  nuevo, integración, aditivo; tesis `la_torre_completa_se_construye_en_una_respiracion`,
  `la_pila_fisica_encadena_pager_pool_csr_e_indices`,
  `cada_puerto_tiene_su_adaptador_canonico`) + inventario contra `lib.rs` real (28
  módulos + CLI, verificado 2026-08-25). IMPLEMENTADO 2026-08-25 (509 líneas, 5
  tests = los 3 tesis + 2 de apoyo; **848 tests** ALL_GREEN, el test corre en
  0.00s). HALLAZGOS de componer el edificio entero (material «leer los agujeros
  del mapa»): HashIndex::create exige pool.capacity() ≥ 3+num_buckets
  (UnknownPage — acoplamiento oculto entre adaptadores vecinos, invisible con
  pools holgados como el TmpPager del cap. 15); Float(2.0) → "2" → Int(2) en
  roundtrip JSONL (pérdida silenciosa de floats enteros); reloj MVCC
  pre-incremento arrancando en 1 (el snapshot correcto es
  ResumenCommitMvcc.ts_asignado); la recuperación SOLO renace lo confirmado por
  WAL (autocommits previos al WAL no sobreviven a reabrir).
- [x] Misconcepciones corregidas explícitamente (§1: cuatro). Ejercicios con solución
  verificable diseñados (retos N.10).
- [x] ≥1 ejercicio de retrieval (hexágono de memoria, 10 componentes) y spacing
  planificado (TODO el capítulo ejercita caps. 7-35; §6.11).
- [x] Responde la pregunta crítica del CORPUS (diagrama hexagonal final: dibujado,
  inventariado uno a uno, demostrado con test) y recoge el gancho del cap. 35
  («puertos medidos cierran el círculo hexagonal», §3 y §6.12).
- [x] Anécdota única verificada con fuente primaria (epitafio en St Paul's, 1723;
  Tinniswood 2001; National Churches Trust) — Brunelleschi y Babel descartadas con
  razón explícita.
- [x] Alcance de código nuevo acotado y honesto (UN archivo de tests de integración;
  cero cambios en módulos cap*, cero dependencias nuevas, goldens intactos; §5.1-§5.3).
- [x] Gancho saliente fijado (Parte VIII caps. 37-40 como crecimiento post-mapa; §6.12).
