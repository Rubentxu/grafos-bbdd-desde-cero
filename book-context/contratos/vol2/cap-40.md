# CONTRATO DE CAPÍTULO — Vol.II Cap. 40: Distribuir una base de datos de grafos

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **Cuarto y ÚLTIMO capítulo de la
> Parte VIII** y CIERRE DEL VOL.II: tras él sólo quedan el epílogo, el Apéndice 0 y el
> preflight D1. COBRADOR de cuatro deudas explícitas: cap. 36 («¿qué partes del mapa
> sobrevivirían a repartir el motor entre máquinas (40)?»), cap. 37 (`informe_produccion()`
> deja distribución/consenso como frente no cubierto), cap. 39 (gancho saliente YA FIJADO:
> «ya sabes QUÉ calcular y CÓMO leerlo rápido EN UNA máquina; ¿y cuando el grafo no cabe en
> una?») y cap. 30 (`GrafoEspera` construido SIN usar como anzuelo: «cuando llegue la
> concurrencia REAL multi-máquina, el gestor sabe dónde enchufar agregar_espera/quitar_tx/
> detectar_ciclo»). Código ancla: `lib.rs` declara **31 módulos** (`cap07_modelo` …
> `cap39_joins`); `louvain()`/`Particion` detectan comunidades deterministas
> (cap25_comunidades.rs:1094/217); `fnv1a_64` ya existe (cap15_indices.rs:201); el WAL
> (LSN + registro Commit + replay, cap. 28) es la versión LOCAL de un log replicado;
> `dataset_referencia_mini(seed)` (400 nodos / 1.200 aristas, Barabási-Albert con hubs,
> cap34_benchmarks.rs:400) y `SEMILLA_REFERENCIA` (cap34_benchmarks.rs:64) dan el terreno
> determinista; el ×521 del cap. 39 (266.752 tuplas para 512 triángulos) es el eco del
> skew que aquí se muda de RAM a RED. Estado verificado 2026-08-26: **878 tests**
> ALL_GREEN; runtime dependency-free (dev-deps: tempfile/proptest/criterion 0.7,
> `harness=false`); toolchain pinneada 1.96.0. Código NUEVO previsto: UN módulo
> `src/cap40_distribucion.rs` (~900-1300 líneas, std puro) + **SIN bench nuevo** (decisión
> #11: lo medible aquí son CONTADORES exactos) + wiring ADITIVO en `lib.rs` (2 líneas) —
> CERO deps nuevas, CERO cambios en caps. 7-39. El brief (líneas 1604-1620) exige:
> particionado por nodos · edge cuts y vertex cuts · replicación de fronteras · consultas
> entre particiones · consistencia · Raft · rebalanceo · hotspots · por qué distribuir
> grafos es difícil — «No se implementaría por completo, pero sí se diseñaría». Citas
> VERIFICADAS hoy (2026-08-26, venue/año exactos): Raft = **USENIX ATC '14**
> (Ongaro-Ousterhout, pp. 305-319; versión extendida raft.pdf); Pregel = **SIGMOD '10**
> (Malewicz et al., Indianápolis); PowerGraph = **OSDI '12** (Gonzalez et al., pp. 17-30
> — edge/vertex cuts, factor de replicación, greedy sobre power-law graphs); Kùzu = CIDR
> 2023 CC-BY 4.0 según ADR-001 (motor EMBEBIDO: contraste honesto en N.10); GQL = ISO/IEC
> 39075:2024 (ya citado); 2PC vía Petrov, *Database Internals* (O'Reilly 2019,
> CONVENTIONS §5). Pregunta crítica del CORPUS (`vol-II-cap-40`, Parte VIII): «Sharding
> por hash vs por comunidad; cut edges.» Gancho saliente: NO hay cap. 41 — el cierre
> apunta al EPÍLOGO y deja abiertas las fronteras hacia el Vol.III.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: TODO el edificio — CSR ordenado ×16 (14), FNV-1a (15),
  LiraQL end-to-end con optimizador y `explain` (17-21), BFS/Dijkstra/A* (22-23),
  centralidad/PageRank (24), comunidades con Louvain determinista (25),
  `Presupuesto`/`BitSet`/streaming (26), ACID/WAL/recuperación/MVCC con
  `NivelAislamiento` y `GrafoEspera` (27-30), torre de pruebas (33), datasets
  deterministas y medición (34), observabilidad/CLI (35), el hexágono final (36), el
  mapa de producción 0·6·5 (37), lectura columnar (38), WCOJ/leapfrog (39).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «distribuir escala gratis» — cada salto que cruza una partición paga un MENSAJE;
  la localidad es el recurso escaso y decide el diseño (PowerGraph, OSDI '12).
  (2) «hash es la buena porque balancea» — balancea la CARGA y destroza VECINDARIOS:
  una BFS de 2 saltos sobre hash cruza la red casi en cada paso.
  (3) «replicar es tener backups» — replicar es CONSENSO: copias sin protocolo DIVERGEN;
  Raft existe para que N réplicas actúen como UNA máquina con log único (ATC '14).
  (4) «Raft da transacciones distribuidas» — ordena UN log para LAS COPIAS de un
  fragmento; la atomicidad ENTRE fragmentos necesita 2PC ENCIMA (solo diseño, Petrov).
  (5) «los cortes son evitables» — son INEVITABLES en todo grafo no trivial; la pregunta
  es QUÉ cortas (aristas → réplicas de nodos-frontera; vértices → réplicas de aristas).
- **Objetivos de dominio (teaching)**: Knowledge — sabe decir POR QUÉ un grafo resiste
  el sharding (la arista cruzada rompe la localidad), QUÉ es edge cut vs vertex cut con
  su métrica, QUÉ garantiza Raft (log idéntico y compromiso por mayoría ante minoría
  caída) y qué NO (atomicidad entre shards). Skills — ejecutar las TRES estrategias de
  particionado sobre el dataset y LEER sus métricas; conducir un clúster Raft de 3 nodos
  con tics lógicos. Wisdom — decide CUÁNDO NO distribuir (si cabe en una máquina, la
  distribución solo añade fronteras: moraleja del volumen) y CUÁNDO hash vs comunidad
  (carga uniforme vs localidad).
- **Pregunta crítica que el capítulo tiene que responder**: «Sharding por hash vs por
  comunidad; cut edges.» Respuesta medible: hash (FNV-1a módulo k), comunidad (Louvain
  reutilizado) y balanceo codicioso ejecutadas sobre el MISMO dataset con métricas
  EXACTAS — cortes, nodos-frontera, factor de replicación, tamaños máx/mín, mensajes de
  una BFS — más el vertex cut del hub sobre la estrella. Sin cuentas verificadas no hay
  capítulo: sería folclore de charla, no ingeniería.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| Módulo `cap40_distribucion.rs` (std puro, sin deps): `AsignacionParticion` (dueño `Vec<u32>` por nodo) + `particionar_hash(n, k)` (FNV-1a del cap. 15 módulo k) + `particionar_por_comunidad(store, k)` (REUTILIZA `louvain()` del cap. 25; grupos→particiones con fusión determinista de comunidades pequeñas por menor miembro) + `particionar_balanceo_codicioso(aristas, k)` (nodos por grado descendente, cada uno a la partición que menos cortes incrementales paga) | `cargo test -p vol2-liradb --lib cap40`: `hash_modulo_k_asigna_todo_y_balancea` (todos los nodos tienen dueño < k; tamaños dentro de ±1 para n divisible) y `particiones_comunidad_respetan_grupos_de_louvain` (nodos de la misma comunidad Louvain nunca acaban en particiones distintas salvo fusión explícita) |
| **Métricas de corte EXACTAS**: `MetricasCorte { cortes_arista, nodos_frontera, factor_replicacion, tam_max, tam_min }` vía `metricas_corte(asignacion, aristas)` — definiciones literales de PowerGraph (OSDI '12): edge cut = aristas con extremos en particiones distintas; frontera = nodos incidentes a cortes; factor de replicación = Σ réplicas / n | `cortes_hash_vs_comunidad_en_mini_cuentas_exactas` (TESIS del capítulo: comunidad < hash en cortes sobre el mini dataset; si la dirección falla, SE REPORTA, no se infla) y `corte_de_arista_frontera_y_factor_replicacion_contados_a_mano` (fixture de ≤8 nodos dibujado en prosa, valores calculados a mano) |
| **Vertex-cut demostrable** (la lección PowerGraph): `replicar_hub(hub, vecinos, k)` → `InformeVertexCut { cortes_antes, cortes_despues, replicas_hub }` — el hub se replica en TODAS las particiones que guardan vecinos y cada hoja conserva su arista local | `corte_de_vertice_del_hub_elimina_cortes_pagando_replicas` (estrella conocida: rueda con centro + m hojas ⇒ edge cut m→0 pagando réplicas del centro; cifras a mano) |
| **Consultas entre particiones contadas**: `bfs_entre_particiones(adj, asignacion, origen)` → `ResultadoBfsDistribuido { visitados, mensajes_red, saltos_red }` — el coste de red es CONTADOR de trabajo (como los pasos WCOJ del cap. 39), no latencia inventada | `bfs_entre_particiones_cuenta_mensajes_y_coincide_con_bfs_local` (alcanzables IDÉNTICOS al BFS local; mensajes > 0 bajo hash y exactos en fixture conocido; 0 mensajes cuando todo es local) |
| **Hotspots y rebalanceo**: `carga_por_particion(asignacion, grados)` (concentración de carga por dueño) + `rebalancear(asignacion, aristas, particion_a_mover)` → `InformeRebalanceo { nodos_movidos, aristas_retocadas, cortes_antes, cortes_despues }` | `hub_concentra_la_carga_en_su_particion` (el hub del dataset — `DatasetReferencia.hubs` — domina la carga de SU partición) y `rebalanceo_mueve_una_particion_y_recuenta_cortes` (coste contado, antes/después) |
| **Raft mínimo DETERMINISTA**: `RolRaft { Seguidor, Candidato, Lider }` + `NodoRaft { id, termino, voto_de, log: Vec<EntradaLog>, indice_compromiso, timeout_tics }` + `EntradaLog { termino, comando }` + `MensajeRaft { PideVoto, Voto, Entradas, Acuse }` + `EnjambreRaft` (bus FIFO `VecDeque` determinista, `tic()` que avanza relojes lógicos, `caer(id)`/`revivir(id)`) — elecciones con timeouts ESCALONADOS FIJOS (sin RNG, sin sleeps, sin hilos), AppendEntries y compromiso por mayoría | `raft_eleccion_por_tics_elige_lider_con_mayoria`, `raft_appendentries_replica_el_log_en_los_seguidores` (logs IDÉNTICOS byte a byte), `raft_no_compromete_sin_mayoria_de_acuses` (mayoría caída ⇒ índice de compromiso NO avanza), `raft_seguidor_reconectado_alcanza_el_log_del_lider` — todos con tics lógicos: CERO flakiness |
| **COBRO de la deuda cap. 30**: detección de deadlock DISTRIBUIDO — las esperas locales por partición alimentan `GrafoEspera`s locales (API pública existente) y una global fusionada; `detectar_ciclo()` encuentra ciclos QUE CRUZAN PARTICIONES | `grafo_espera_global_detecta_deadlock_entre_particiones` (T1 retiene recurso en A y espera en B; T2 al revés: ciclo [T1, T2] encontrado por la pieza que el cap. 30 dejó enchufada) |
| **Informe reproducible** para la prosa: `informe_distribucion_reproducible_sobre_mini(store)` — tabla estrategias × {cortes, frontera, factor replicación, tam máx/mín, mensajes BFS, carga hub} SIN tiempos (regla del cap. 34: los enteros viven en tests) | `informe_distribucion_reproducible_sobre_mini` |
| **SIN `[[bench]]` nuevo** (decisión #11, desviación DECLARADA del patrón caps. 34/38/39): ninguna afirmación del capítulo depende de cronometraje | `verify.sh` compila `--all-targets` igual; la prosa pega SALIDAS de `cargo test` (contadores exactos), no µs |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**878 + ~14 tests nuevos ≈ 892**); cero cambios en caps. 7-39, goldens intactos |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Sharding por hash vs por comunidad; cut edges.» El capítulo la responde
convirtiendo los NUEVE puntos del brief (líneas 1610-1618) en una escalera donde cada
peldaño motiva el siguiente:

1. **Por qué distribuir grafos es difícil** → el enemigo es la ARISTA CRUZADA: los
   vecinos no están donde están los nodos; una consulta k-hop convierte cada frontera en
   latencia. En tablas la fila es autocontenida; en grafos la UNIDAD de trabajo ES la
   travesía (PowerGraph, OSDI '12).
2. **Particionado por nodos** → primera solución ingenua: hash(FNV-1a) módulo k. Balance
   perfecto O(1)… y la pregunta que abre el resto: ¿a qué precio en cortes?
3. **Edge cuts y vertex cuts** → cortar ARISTAS replica nodos-frontera; cortar VÉRTICES
   replica aristas. Factor de replicación y nº de cortes como las DOS facturas posibles
   (OSDI '12 — nació porque Pregel/SIGMOD '10 sufren en power-law graphs).
4. **Replicación de fronteras** → vertex cut del HUB sobre la estrella: m cortes → 0
   pagando k réplicas del centro. El skew no desaparece al repartir: se muda de RAM a
   RED (eco del ×521 del cap. 39 y del hub-skew del cap. 34).
5. **Sharding hash vs comunidad** (respuesta central, pregunta CORPUS) → las TRES
   estrategias sobre el MISMO dataset determinista: hash balancea tamaños pero maximiza
   cortes; comunidad (Louvain del cap. 25, spacing puro) minimiza cortes pero
   desbalancea (hotspot de comunidad grande); codicioso intermedio. Tabla de enteros
   exactos como veredicto — sin ganador absoluto, con TRADE-OFF medido.
6. **Hotspots** → `carga_por_particion`: el hub concentra la carga de SU partición;
   conexión con centralidad (cap. 24) y hubs del dataset (cap. 34).
7. **Consultas entre particiones** → BFS distribuida que CONTABILIZA mensajes/saltos de
   red (misma disciplina que contar pasos WCOJ); vocabulario: supersteps de Pregel.
8. **Consistencia y Raft** → el WAL local del cap. 28 GENERALIZADO: log replicado que
   solo avanza por MAYORÍA — tics lógicos, timeouts escalonados, AppendEntries, índice
   de compromiso, caída y reconexión, TODO determinista (USENIX ATC '14). 2PC ENTRE
   particiones: solo DISEÑO en prosa (Petrov, 2019), anclado al registro Commit.
9. **Rebalanceo** → mover una partición, recalcular cortes, pagar el traslado: coste
   contado antes/después.
10. **Cierre de deudas**: deadlock distribuido con el `GrafoEspera` GLOBAL (cap. 30
    enchufado por fin) y el hexágono del cap. 36 respondido: el mapa sobrevive POR
    MÁQUINA; cambia dónde vive cada capa y quién ordena los commits.

Hilo conductor: «en una máquina el coste era CPU y memoria; en muchas, el coste es
CRUZAR LA FRONTERA — cada corte que diseñas hoy es un mensaje que pagarás mañana».

---

## 4. La arquitectura: la frontera es la factura

Modelo mental único: **localidad = combustible**. Todo lo que ya sabes hacer (caps.
7-39) sigue valiendo DENTRO de cada máquina; lo nuevo es la contabilidad de la frontera.

```text
  UNA MÁQUINA (caps. 7-39):                  K MÁQUINAS (este capítulo):
  todo local; cortes = 0                     cada arista cruzada = UN MENSAJE
  BFS: pasos de CPU                          BFS: mensajes de red + pasos de CPU
  log: WAL local + replay (28)               log: REPLICADO por mayoría (Raft)
  commit: registro Commit (28)               entre particiones: 2PC (solo diseño)
```

```text
  EDGE CUT (cortas aristas):            VERTEX CUT (cortas vértices):
  P0: {a, b}   P1: {c, d}               P0: {a, c*}   P1: {b, c*}
      a───b ─ ─ ─ c───d                     a──►c*   b──►c*   (c REPICADO)
  arista b–c CORTADA; frontera {b, c}   cero aristas cortadas; c pagado 2 veces
  factura: cortes + réplicas de NODOS   factura: réplicas de ARISTAS del corte
```

Y debajo, la REGLA DE ORO heredada del cap. 34: dataset determinista
(`SEMILLA_REFERENCIA`), contadores de TRABAJO dentro de los tests, y CERO tiempos de
pared — aquí ni hacen falta: los enteros SON la física del sistema.

```text
Lo que SÍ se hace hoy:   capa EDUCATIVA aparte (módulo propio, como caps. 38/39): tres
                         estrategias de particionado medidas, vertex-cut del hub, BFS con
                         contador de mensajes, hotspots, rebalanceo medido, Raft
                         determinista de 3 nodos (elección + AppendEntries + compromiso
                         por mayoría + caída/recuperación), deadlock distribuido,
                         informe reproducible
Lo que AÚN NO:           red TCP/RPC real ni serialización · runtime asíncrono (tokio,
                         futuro) · cambios de pertenencia (joint consensus) · log
                         compaction ni snapshots · tolerancia bizantina · 2PC
                         IMPLEMENTADO · consultas federadas · transacciones distribuidas
```

Momento ¡ajá! perseguido: «el hub que fabricaba 521 tuplas fantasma por triángulo en el
cap. 39 ahora fabrica MENSAJES: repartir no elimina el skew — lo traduce a otra divisa.
Y la cura tampoco cambia: medir primero, decidir después».

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Un módulo nuevo `cap40_distribucion.rs`, std puro, dependency-free | Regla «primero a mano» (CONVENTIONS §4): asignación, cortes, bus de mensajes y máquina de estados Raft SON enseñables con `VecDeque` y enteros | Crate `raft`/`openraft` o `tokio`: dependencias enormes, async runtime, API opaca — nada que APRENDER y flakiness garantizado | CONVENTIONS §4; misma regla que caps. 18 (lexer), 28 (WAL), 38 (diccionario), 39 (tries) |
| 2 | Capa EDUCATIVA FUERA del motor: ni `Executor`, ni `GraphStore`, ni catálogo se tocan; la simulación vive en `cap40_distribucion` con estructuras propias que LEEN del store | Contratos de caps. 7-39 intactos; el capítulo enseña el DISEÑO de la distribución (brief: «se diseñaría»), no instala un clúster en producción | Integrar routing por partición en `run()`/`Query::execute`: proyecto de otra Parte, rompería explain/goldens | Honestidad hexagonal caps. 33-34/38/39; CONVENTIONS §2 (una idea nueva por sección) |
| 3 | SIMULACIÓN offline sobre dataset determinista: particiones = dueños en un `Vec`, red = bus FIFO en el mismo proceso | Contadores EXACTOS y reproducibles en CI; la tesis es estructural, no de latencia | Micro-servicios con sockets reales: indemostrables en tests y no-deterministas | Metodología cap. 34; precedentes: `ContandoStore` (cap. 26), contadores WCOJ (cap. 39) |
| 4 | Hash = `fnv1a_64(NodeId) % k` REUTILIZADO del cap. 15 | Ya existe, ya testeado, determinista; introduce el sharding con CERO código criptográfico nuevo | CRC32 u otro hash nuevo: mismo papel, código redundante | cap15_indices.rs:201; práctica estándar de sharding por hash |
| 5 | Comunidad vía `louvain()` EXISTENTE (cap. 25) mapeada a particiones con fusión determinista (comunidades pequeñas → menor miembro) | Spacing puro: la detección de comunidades YA es maquinaria probada; el capítulo enseña a USARLA como política de colocación | Recomputar detección propia de comunidades: duplicaría 600 líneas probadas del cap. 25 | cap25_comunidades.rs:1094 (`louvain`, determinismo documentado); Blondel 2008 (ya citado en cap. 25) |
| 6 | Métricas de corte por DEFINICIÓN literal (edge cut, frontera, factor de replicación) y fixtures pequeños dibujados a mano | Son las magnitudes de PowerGraph; con fixture de ≤8 nodos se verifican A MANO — el estándar de honestidad de la casa (K₈ del cap. 39) | Aproximaciones heurísticas (normalized cut, NE): innecesarias a esta escala y no enseñables sin teoría extra | Gonzalez et al., OSDI '12 (§2: edge vs vertex cut, replication factor) |
| 7 | Vertex cut demostrado SOLO en el caso del hub/estrella (`replicar_hub`), no un particionador vertex-cut general | Es EL contraejemplo que motiva PowerGraph entero y conecta con el hub-skew de caps. 24/34/39; un greedy vertex-cut general (LDG completo) diluye el foco | Particionador vertex-cut general con greedy heurístico: queda como reto experto | PowerGraph, OSDI '12 (greedy vertex-cut sobre power-law graphs); eco ×521 del cap. 39 |
| 8 | Mensajes de red como CONTADOR exacto, jamás latencias simuladas | Contar es reproducible; inventar milisegundos sería falsificar medición (pecado capital del cap. 34) | RTTs sintéticos ponderados: humo con decimales | Pregel, SIGMOD '10 (el mensaje por superstep COMO unidad de coste del modelo) |
| 9 | Raft mínimo IMPLEMENTADO pero determinista: tics lógicos, timeouts escalonados fijos por nodo (sin RNG ni sleeps), bus FIFO, mayoría 2-de-3, caída/revivir explícitos | La casa exige DEMOSTRAR: sin elección ni compromiso testeado, Raft sería magia. Los timeouts escalonados sacrifican el anti-split-vote aleatorio del paper A CAMBIO de determinismo total — trade-off DOCUMENTADO | (a) Solo diseño en prosa: traicionaría «sin equivalencia testeada no hay capítulo»; (b) hilos reales + `mpsc`: flakiness prohibido | Ongaro-Ousterhout, USENIX ATC '14, pp. 305-319 (+ raft.pdf: randomized timeouts como refinamiento de producción) |
| 10 | 2PC ENTRE particiones: SOLO diseño en prosa, anclado al registro Commit del WAL (cap. 28) como punto de no retorno | Raft ordena UN fragmento; 2PC coordina VARIOS — concepto imprescindible, implementación duplicativa de Raft en este capítulo | Implementar 2PC sobre el EnjambreRaft: doblaría el tamaño del módulo sin nueva idea pedagógica | Petrov, *Database Internals* (O'Reilly 2019, parte III — CONVENTIONS §5) |
| 11 | **SIN bench criterion nuevo** — desviación DECLARADA del patrón caps. 34/38/39 | Ninguna afirmación del capítulo es sobre TIEMPO: cortes, réplicas, mensajes y entradas de log son enteros EXACTOS que viven en tests; cronometrar el particionado repetiría el bench de Louvain del cap. 25 sin tesis nueva | Cuarto `[[bench]]` de particionado: números de reloj sin hipótesis que sostener | Metodología cap. 34 (¿qué afirma el capítulo y con qué moneda se demuestra?); precedente inverso: cap. 39 SÍ necesitaba delta temporal |
| 12 | COBRO de la deuda cap. 30: deadlock distribuido ensamblando esperas locales en un `GrafoEspera` global y llamando `detectar_ciclo()` | El cap. 30 dejó la pieza «enchufada» explícitamente para la concurrencia multi-máquina; además es la técnica clásica: grafo-espera global en un coordinador | Declarar la deuda impagada: contradice el contrato del cap. 30 y desperdicia spacing gratuito | cap30_mvcc.rs:599 (`GrafoEspera`, `agregar_espera`/`quitar_tx`/`detectar_ciclo`, anzuelo citado en su doc) |
| 13 | Atribución según ADR-001 y contraste honesto: Kùzu/LadybugDB son motores EMBEBIDOS mononodo; los sistemas distribuidos de referencia (clustering de Neo4j, JanusGraph, TigerGraph, etcd) se nombran por su documentación pública sin URLs fabricadas | Política VINCULANTE cap. 38+: CIDR 2023 con licencia; y honestidad de alcance: LiraDB imita conceptos de sistemas distribuidos, no a Kùzu | Citar «Kùzu distribuido» (falso) o URLs de docs no verificadas | ADR-001 (RESUELTA 2026-08-25); Jin et al., CIDR 2023, CC-BY 4.0; docs públicas de Neo4j/JanusGraph/TigerGraph/etcd (sin URL concreta en el contrato) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: Google, 2008-2010 — Pregel procesa
   «billones de vértices y trillones de aristas» pensando «like a vertex» (SIGMOD '10);
   años después CMU demuestra que en grafos NATURALES —power-law, como el dataset de
   LiraDB desde el cap. 34— ese planteamiento se rompe, y la cura es invertir el cuchillo:
   no cortes aristas, corta VÉRTICES (PowerGraph, OSDI '12). Pregunta enmarcada: ¿qué
   partes del mapa de LiraDB (cap. 36) sobreviven a repartir el motor entre máquinas?
2. **N.1-N.2 Objetivo/Problema**: 878 tests verdes, motor completo EN UNA máquina; una
   sola caja tiene techo de RAM, disco y CPU. Qué NO te dice la suite verde: qué pagarías
   MAÑANA por cada arista que hoy cruza gratis la caché.
3. **N.3 Modelo mental**: panel dual una-máquina/k-máquinas (§4) + taxonomía edge cut /
   vertex cut + bloque «lo que sí / lo que aún no» + la escalera del brief.
4. **N.4 Primera solución**: hash módulo k con FNV-1a del cap. 15 — balance perfecto,
   O(1), y una sensación de victoria que el siguiente paso desmonta.
5. **N.5 Sus límites**: `metricas_corte` sobre el mini dataset revela la factura: casi
   toda BFS de 2 saltos cruza la red; el hub cae en UNA partición y sobrecarga (hotspot);
   la estrella muestra el caso patológico extremo.
6. **N.6 Solución evolucionada**: comunidad vía Louvain (cap. 25) → menos cortes, peor
   balance; codicioso intermedio; vertex-cut del hub (m cortes → 0 pagando réplicas);
   BFS distribuida contando mensajes; rebalanceo con coste medido; y la pieza de
   CONSISTENCIA: Raft determinista de 3 nodos — más el deadlock distribuido cobrando el
   `GrafoEspera` del cap. 30 y el 2PC dibujado en prosa.
7. **N.7 Código completo ejecutable**: `cap40_distribucion.rs` referenciado por
   `include::` (nunca duplicado); SIN bloque `[[bench]]` (decisión #11 explicada).
8. **N.8 Prueba de fuego**: tabla estrategias × métricas del informe reproducible;
   estrella con vertex-cut (cifras a mano); clúster Raft: elección estable, logs
   idénticos, mayoría caída NO compromete, seguidor reconectado se pone al día;
   deadlock entre particiones detectado por el grafo-espera global. Salidas REALES de
   `cargo test` pegadas.
9. **N.9 Qué hemos sacrificado**: sin red real ni RPC ni serialización; sin runtime
   asíncrono; sin cambios de pertenencia (joint consensus), log compaction ni
   snapshots; sin tolerancia bizantina; 2PC solo diseñado; sin consultas federadas ni
   transacciones distribuidas; timeouts escalonados en vez del randomizado de producción.
10. **N.10 Cómo lo hace una BBDD real + retos**: Neo4j (clustering causal + Fabric),
    JanusGraph, TigerGraph, etcd/Consul (Raft EN PRODUCCIÓN — docs oficiales) y
    **Kùzu/LadybugDB según ADR-001** — embebido MONONODO: el contraste honesto que
    cierra el volumen. Retos: esencial (particionar la estrella a mano y predecir
    cortes ANTES de medir), intermedio (PREDECIR la tabla hash-vs-comunidad del mini
    dataset y explicar desviaciones), experto (extender `replicar_hub` a un greedy
    vertex-cut general tipo LDG).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla / 30 segundos /
    Una historia pequeña / Mini-diálogo de guardia nocturna (la partición caliente que
    fundía una máquina mientras las demás dormían). Retrieval practice: reproducir DE
    MEMORIA la taxonomía de cortes, la escalera del brief y el flujo de una elección
    Raft. Interleaving: cada reto toca ≥2 capítulos (15+40, 25+40, 28+40, 24/34/39+40,
    30+40, 36+40, 37+40). Glosario nuevo: sharding, partición, edge cut, vertex cut,
    frontera, factor de replicación, hotspot, rebalanceo, consenso, término, líder,
    quórum/mayoría, log replicado, AppendEntries, índice de compromiso, 2PC.
12. **Gancho de cierre (CIERRA EL VOLUMEN)**: el hexágono del cap. 36 quedó respondido:
    cada pieza sobrevive POR MÁQUINA; lo nuevo es la contabilidad de la frontera. Al
    EPÍLOGO con las preguntas abiertas heredadas y nuevas: ¿paralelizar leapfrog
    morsel-driven (39)? ¿orden dinámico? ¿columnas en disco? ¿red real con runtime
    asíncrono? ¿compaction/snapshots del log replicado? ¿y cuando la MEMORIA de un
    agente necesita un grafo — Vol.III?

---

## 7. Estilo y tono (consistencia con caps. 27-39)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (shard, edge cut, vertex cut, replication factor, leader
  election, quorum); salidas REALES de `cargo test` pegadas, nunca reconstruidas; la
  tabla hash-vs-comunidad se presenta como TRADE-OFF medido, no como carrera con
  ganador; el Raft se vende por su motivación original — UNDERSTANDABLE (ATC '14).
- **Diagramas**: panel dual una-máquina/k-máquinas (§4); taxonomía de cortes con la
  factura de cada lado; línea temporal ASCII de una elección Raft por tics; figura de
  la estrella antes/después del vertex-cut; escalera del brief; tabla del informe.
- **Spacing** (conceptos viejos que se EJERCITAN): FNV-1a (cap. 15), Louvain/
  `Particion` (cap. 25), WAL y registro Commit (cap. 28), `GrafoEspera` (cap. 30),
  dataset determinista y hubs (cap. 34), hexágono (cap. 36), mapa de producción (cap.
  37), contadores WCOJ y ×521 (cap. 39).
- **Interleaving**: reto esencial mezcla 34+40; el intermedio mezcla 21/25+40; el
  experto mezcla 39+40 (greedy vertex-cut y fan-out del hub).
- **Dificultad asimétrica**: una idea nueva por sección (frontera → hash → cortes →
  vertex cut → comunidad → mensajes → Raft → rebalanceo → deadlock global); los
  ejercicios exigen PREDECIR contadores y recordar la escalera sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap40` (contadores exactos) y
  `./scripts/verify.sh` ALL_GREEN como puerta; sin bench que ejecutar manualmente
  (decisión #11). Nunca «confía en mí».
- **Anécdota (única, verificada)**: Pregel (SIGMOD '10) → PowerGraph (OSDI '12): la
  misma industria descubriendo que el skew manda y obliga a cambiar QUÉ se corta.
  Fuentes para la prosa: Malewicz et al. (SIGMOD '10); Gonzalez et al. (OSDI '12,
  pp. 17-30); Ongaro-Ousterhout (USENIX ATC '14, pp. 305-319 + raft.pdf;
  raft.github.io); Jin et al. (CIDR 2023, CC-BY 4.0, ADR-001); Petrov (*Database
  Internals*, O'Reilly 2019); ISO/IEC 39075:2024; Blondel 2008 y Brin-Page 1998
  (ya citados en caps. 24-25).

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap40_distribucion; pub
  use cap40_distribucion::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al
  final, con el módulo ya compilando limpio (`cargo check -p vol2-liradb`); jamás dejar
  `lib.rs` apuntando a un módulo rojo.
- **Orden de implementación recomendado** (cada paso compila y testea solo): (1)
  `AsignacionParticion` + `particionar_hash` + `metricas_corte` con fixture a mano;
  (2) `particionar_por_comunidad` (Louvain + fusión) y comparación hash vs comunidad;
  (3) `particionar_balanceo_codicioso`; (4) `replicar_hub` (vertex-cut de la estrella);
  (5) `bfs_entre_particiones` con contadores; (6) `carga_por_particion` +
  `rebalancear`; (7) Raft: tipos + `EnjambreRaft`/bus → elección → AppendEntries →
  compromiso/mayoría → caída y reconexión; (8) `grafo_espera_global` (deuda cap. 30);
  (9) informe reproducible; (10) wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap40` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en
  `cap40_distribucion.rs` y continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). PROHIBIDO
  introducir sleeps, hilos o RNG para «hacer funcionar» Raft: si un test no pasa con
  tics lógicos, el BUG está en el protocolo, no en el test.
- **Criterio de parada honesto**: si `comunidad < hash` NO se cumple sobre el mini
  dataset, se REPORTA tal cual (patrón del ×0,86 del email en cap. 38) y la prosa
  explica POR QUÉ; prohibido inflar o esconder. Igual para el balance: si Louvain
  desbalancea, ESA es la lección (hotspot de comunidad), no un fallo.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (13 filas en §5; citas verificadas 2026-08-26: ATC '14, SIGMOD '10, OSDI '12, CIDR
  2023/ADR-001, ISO 39075:2024, Petrov 2019).
- [ ] Escenario de fallo visible, no solo happy path: hash que destroza vecindarios,
  hub que calienta UNA partición, mayoría caída que BLOQUEA el compromiso, seguidor
  desactualizado que recibe log del líder, deadlock que SOLO aparece cruzando
  particiones, partición de Louvain que desbalancea.
- [ ] Código ejecutable citado por nombre (`cap40_distribucion.rs`, wiring en `lib.rs`,
  SIN `[[bench]]` — decisión declarada); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: cinco).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa de
  contadores como patrón común).
- [ ] ≥1 ejercicio de retrieval (escalera + taxonomía + elección Raft de memoria) y
  spacing planificado (caps. 15/24/25/28/30/34/36/37/39; §7).
- [ ] Responde la pregunta crítica del CORPUS («Sharding por hash vs por comunidad;
  cut edges»: TRES estrategias MEDIDAS + vertex-cut del hub, no descritas) y cobra las
  deudas heredadas (caps. 30/36/37/39; §blockquote/§3).
- [ ] Anécdota única verificada con fuentes primarias (SIGMOD '10 + OSDI '12).
- [ ] Alcance acotado y honesto (UN módulo + wiring en `lib.rs`; cero deps, cero
  benches nuevos, cero cambios caps. 7-39; Raft limitado a elección/replicación/mayoría
  con tics lógicos — sin compaction, membership ni red).
- [ ] Gancho de CIERRE DE VOLUMEN fijado (epílogo; preguntas abiertas: morsel-driven,
  orden dinámico, columnas en disco, red real, Vol.III; §6.12).
