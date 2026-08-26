# Capítulo 40 — Distribuir una base de datos de grafos

> *«Último capítulo del volumen. Treinta y nueve veces construiste piezas que vivían dentro de una sola máquina; esta vez el motor se reparte entre varias y aparece la única ley nueva del edificio: todo lo que ya sabes hacer sigue valiendo DENTRO de cada caja, y cada arista que cruce la frontera se convertirá en un mensaje que pagarás mañana.»*

## 40.0 La anécdota de la esquina

Google, 2008-2010. Un equipo construye **Pregel**, el sistema que ejecuta programas sobre grafos de «billones de vértices y trillones de aristas» (Malewicz et al., SIGMOD '10, Indianápolis). Su idea tiene una elegancia desarmante: piensa *like a vertex* — tu programa es una función que, en cada ronda (*superstep*), recibe mensajes, actualiza el estado del vértice y envía mensajes a sus vecinos. No hay bucles de red escritos a mano ni locks: hay rondas síncronas y mensajería. Funciona, se publica, toda la industria lo imita. Y años después, en CMU, Gonzalez y compañía miran los grafos NATURALES —power-law, con hubs enormes, exactamente la familia que genera el dataset de referencia de LiraDB desde el cap. 34— y demuestran que ese planteamiento se rompe: cuando repartes vértices entre máquinas y una arista conecta dos máquinas, el modelo corta ARISTAS, y un hub de grado alto fabrica una tormenta de mensajes en cada ronda. Su cura es invertir el cuchillo: no cortes aristas, corta VÉRTICES (**PowerGraph**, Gonzalez et al., OSDI '12, pp. 17-30). La misma industria, cuatro años, descubriendo que el skew manda y obliga a cambiar QUÉ se corta.

Hazte la pregunta que este capítulo te hace a nuestra criatura: tienes 892 tests verdes y un motor completo — CSR ordenado, LiraQL con optimizador, ACID con WAL y MVCC, WCOJ, lectura columnar. ¿Qué partes del mapa hexagonal del cap. 36 sobreviven a repartirlo ENTRE MÁQUINAS? Spoiler honesto: casi todas — pero ninguna gratis. Empieza a contar mensajes.

## 40.1 Objetivo

Este último capítulo cobra CUATRO deudas explícitas: la pregunta del hexágono del cap. 36 («¿qué partes del mapa sobrevivirían a repartir el motor?»), el frente que `informe_produccion()` dejó declarado y no cubierto (cap. 37: distribución y consenso), el gancho saliente del cap. 39 («ya sabes QUÉ calcular y CÓMO leerlo rápido EN UNA máquina; ¿y cuando el grafo no cabe en una?») y el anzuelo del cap. 30: el `GrafoEspera` se construyó SIN usar, con la promesa de que «cuando llegue la concurrencia REAL multi-máquina, sabrás dónde enchufarlo». Hoy se enchufa. Al terminar tendrás:

1. **Tres estrategias de particionado medidas** sobre el MISMO dataset determinista: `particionar_hash` (FNV-1a del cap. 15 módulo k, reutilizada tal cual), `particionar_por_comunidad` (REUTILIZA `louvain()` del cap. 25 con fusión determinista de comunidades pequeñas) y `particionar_balanceo_codicioso` (grado descendente, cada nodo a la partición que menos cortes incrementales paga).
2. **Métricas de corte EXACTAS** (`metricas_corte` → `MetricasCorte`): **edge cut**, **frontera** y **factor de replicación** por definición literal de PowerGraph, verificables A MANO en fixtures de ≤8 nodos — el estándar de honestidad de la casa (K₈ del cap. 39).
3. **El vertex cut demostrable** (`replicar_hub` → `InformeVertexCut`): la estrella del hub pasa de m cortes a 0 pagando réplicas del centro — la lección PowerGraph en una función.
4. **Consultas entre particiones contadas** (`bfs_entre_particiones` → `ResultadoBfsDistribuido`): mensajes y saltos de red como CONTADORES exactos, la misma disciplina que los pasos WCOJ del cap. 39 — jamás latencias inventadas.
5. **Hotspots y rebalanceo** (`carga_por_particion`, `rebalancear` → `InformeRebalanceo`): mover carga tiene precio, y se cuenta ANTES/DESPUÉS.
6. **Consenso de verdad**: un **Raft** mínimo DETERMINISTA de 3 nodos (`EnjambreRaft`): elecciones por tics lógicos con timeouts escalonados, **AppendEntries**, compromiso por mayoría, caída y reconexión (Ongaro-Ousterhout, USENIX ATC '14, pp. 305-319).
7. **El cobro del cap. 30**: `fusionar_grafos_espera` ensambla los grafos de espera LOCALES en uno GLOBAL y `detectar_ciclo()` encuentra deadlocks QUE CRUZAN PARTICIONES.
8. **Un 2PC diseñado en prosa** sobre el registro Commit del WAL (cap. 28) — la pieza que falta ENCIMA de Raft para atomicidad entre fragmentos (Petrov, *Database Internals*, O'Reilly 2019).
9. **Un informe reproducible** (`informe_distribucion_reproducible_sobre_mini`) y la suite completa: 878 + 14 = **892 tests ALL_GREEN**, goldens intactos, cero dependencias nuevas, cero cambios en caps. 7-39.

## 40.2 Problema

En realidad ya tienes 892−14 = 878 tests verdes y un motor completo EN UNA máquina. Pero una sola caja tiene techo: RAM, disco, CPU. Llega el día en que el grafo no cabe, o el cómputo no da abasto, y alguien dice la palabra mágica: «distribuyamos». Tu suite verde calla sobre la única pregunta que importa aquí: qué pagarías MAÑANA por cada arista que hoy cruza gratis la caché. Antes de construir nada, desactivemos cinco ideas equivocadas que suelen venir con el tema:

1. **«Distribuir escala gratis.»** No: cada salto de una travesía que cruza una partición paga UN MENSAJE de red. La **localidad** es el recurso escaso y decide el diseño — la tesis entera de PowerGraph (OSDI '12). En tablas la fila es autocontenida; en grafos la unidad de trabajo ES la travesía.
2. **«Hash es la buena porque balancea.»** Balancea la CARGA y destroza VECINDARIOS: ignora la topología por construcción, así que una BFS de 2 saltos cruza la red casi en cada paso. Lo vas a medir: 262 mensajes donde el monolito paga 0.
3. **«Replicar es tener backups.»** Replicar es CONSENSO: copias sin protocolo DIVERGEN (una se actualiza, otra no, y cada una cree ser la verdad). Raft existe precisamente para que N réplicas actúen como UNA máquina con un log único (ATC '14).
4. **«Raft da transacciones distribuidas.»** No: Raft ordena UN log para LAS COPIAS de UN fragmento. La atomicidad ENTRE fragmentos necesita **two-phase commit (2PC)** ENCIMA (Petrov, 2019) — aquí solo diseño, y verás por qué.
5. **«Los cortes son evitables si el particionador es bueno.»** Son INEVITABLES en todo grafo no trivial. La pregunta no es SI cortar sino QUÉ cortas: aristas (y pagas sincronizar nodos-frontera) o vértices (y pagas réplicas de aristas).

Y debajo, la pregunta crítica del corpus: «Sharding por hash vs por comunidad; cut edges». Respuesta con criterio de aceptación: las tres estrategias ejecutadas sobre el MISMO dataset con métricas EXACTAS — cortes, frontera, factor de replicación, tamaños máx/mín, mensajes de una BFS — más el vertex cut del hub. Sin cuentas verificadas no hay capítulo: sería folclore de charla, no ingeniería.

## 40.3 Modelo mental: localidad = combustible

Un solo eje ordena el capítulo. Todo lo que construiste en 39 capítulos sigue valiendo DENTRO de cada máquina; lo nuevo es la contabilidad de la FRONTERA — la línea que separa dos dueños:

```text
  UNA MÁQUINA (caps. 7-39):                  K MÁQUINAS (este capítulo):
  todo local; cortes = 0                     cada arista cruzada = UN MENSAJE
  BFS: pasos de CPU                          BFS: mensajes de red + pasos de CPU
  log: WAL local + replay (28)               log: REPLICADO por mayoría (Raft)
  commit: registro Commit (28)               entre particiones: 2PC (solo diseño)
```

Y la taxonomía que gobierna TODA la factura — memorízala, los ejercicios te la van a pedir DE MEMORIA:

```text
  EDGE CUT (cortas aristas):            VERTEX CUT (cortas vértices):
  P0: {a, b}   P1: {c, d}               P0: {a, c*}   P1: {b, c*}
      a───b ─ ─ ─ c───d                     a──►c*   b──►c*   (c REPICADO)
  arista b–c CORTADA; frontera {b, c}   cero aristas cortadas; c pagado 2 veces
  factura: cortes + réplicas de NODOS   factura: réplicas de ARISTAS del corte
```

Sobre ese panel, la escalera de diez peldaños — nueve técnicos más el cobro final de deudas:

```text
peldaño 1  dificultad       → el enemigo es la ARISTA CRUZADA: vecinos lejos de nodos
peldaño 2  sharding nodos   → hash(FNV-1a) módulo k: balance O(1)… ¿a qué precio?
peldaño 3  edge/vertex cut  → dos facturas posibles: cortes+réplicas de nodos o réplicas
peldaño 4  fronteras        → el hub replicado: m cortes → 0 pagando réplicas del centro
peldaño 5  hash vs comunidad→ la pregunta crítica: TRES estrategias MEDIDAS, trade-off
peldaño 6  hotspots         → el hub concentra la carga de SU partición
peldaño 7  consultas remotas→ BFS por supersteps con CONTADOR de mensajes
peldaño 8  Raft             → log replicado que solo avanza por MAYORÍA
peldaño 9  rebalanceo       → mover una partición: coste contado antes/después
peldaño 10 cierre de deudas → deadlock ENTRE particiones (GrafoEspera global) + hexágono
```

Y la frontera, declarada antes de escribir código — la misma disciplina «lo que sí / lo que aún no» de los caps. 33-34/38/39:

```text
Lo que SÍ se hace hoy:   capa EDUCATIVA aparte (módulo propio, como caps. 38/39): tres
                         estrategias medidas, vertex-cut del hub, BFS con contador de
                         mensajes, hotspots, rebalanceo medido, Raft determinista de 3
                         nodos (elección + AppendEntries + compromiso por mayoría +
                         caída/recuperación), deadlock distribuido, informe reproducible
Lo que AÚN NO:           red TCP/RPC real ni serialización · runtime asíncrono · cambios
                         de pertenencia (joint consensus) · log compaction ni snapshots ·
                         tolerancia bizantina · 2PC IMPLEMENTADO · consultas federadas ·
                         transacciones distribuidas
```

El momento ¡ajá! perseguido: el hub que fabricaba 521 tuplas fantasma por triángulo en el cap. 39 ahora fabrica MENSAJES — repartir no elimina el skew, lo TRADUCE a otra divisa. En una máquina el coste era CPU y memoria; en muchas, el coste es cruzar la frontera. Y la cura tampoco cambia: medir primero, decidir después.

## 40.4 Primera solución

La versión que todo el mundo escribe primero: **sharding** por hash. Cada nodo v aterriza en la partición `fnv1a_64(v) % k` — el mismo FNV-1a que reparte claves en buckets del HashIndex del cap. 15, reutilizado tal cual, cero código criptográfico nuevo:

```rust
// Esqueleto de `particionar_hash` (cap40_distribucion.rs)
pub fn particionar_hash(n: usize, k: u32) -> AsignacionParticion {
    let mut dueno = Vec::with_capacity(n);
    for id in 0..n as u32 {
        let h = fnv1a_64(&id.to_le_bytes());     // el FNV-1a del cap. 15
        dueno.push((h % k as u64) as u32);
    }
    AsignacionParticion { num_particiones: k as usize, dueno }
}
```

Correcto, O(1) por nodo, determinista — y el test `hash_modulo_k_asigna_todo_y_balancea` fija el balance MEDIDO, no prometido: FNV-1a módulo 8 sobre los ids 0..399 del mini dataset reparte **exactamente 50 nodos por bucket** (`vec![50; 8]`). Cada máquina recibe el mismo número de vértices, nadie puede quejarse. Es la sensación de victoria que el siguiente paso desmonta: balanceaste los NODOS, y a nadie le importan los nodos — le importan las TRAVESÍAS.

## 40.5 Sus límites

1. **Cómo leer el medidor.** `metricas_corte(asignacion, aristas)` devuelve las tres facturas por definición literal de PowerGraph: `cortes_arista` (aristas con extremos en particiones distintas), `nodos_frontera` (incidentes a algún corte — su estado habrá que sincronizarlo) y `factor_replicacion`. Con colocación edge-cut pura cada nodo vive UNA vez, así que el factor bruto sería 1.0 siempre — trivial. Por eso aquí se cuenta el factor OPERATIVO del gather de PowerGraph: réplica(v) = 1 + nº de particiones vecinas distintas a la propia (definición documentada en el código). Fixture de ≤8 nodos, contado A MANO — el estándar de la casa:

```text
P0 = {0, 1}   P1 = {2, 3}   P2 = {4}
aristas: 0─1, 1─2, 2─3, 3─4, 0─3

cortes      3      (1─2, 3─4, 0─3: extremos en particiones distintas)
frontera    5      ({0,1,2,3,4}: todos incidentes a algún corte)
réplicas    0:1+1  1:1+1  2:1+1  3:1+2  4:1+1  → Σ = 11
factor     11/5 = 2,2      tamaños {2, 2, 1} → máx 2, mín 1
monolito:   0 cortes, 0 frontera, factor 1,0 — el punto de partida del volumen
```

2. **El mini dataset revela la factura.** Corriendo `metricas_corte` bajo hash sobre los 400 nodos y 1.200 aristas dirigidas del `dataset_referencia_mini`: **1.062 cortes — el 88,5 % de las aristas cruzan una frontera**. Y la BFS desde el hub paga **262 mensajes de red** (el monolito: 0). Balance perfecto arriba, vecindarios desparramados abajo: el hash ignoró la topología POR CONSTRUCCIÓN.
3. **El hotspot.** `carga_por_particion` suma grados por dueño: bajo hash, la partición 6 concentra **349 de 2.400 de carga (14,5 %)** — es donde cayó el hub, el nodo 27 de grado 37. Conexión directa con la centralidad del cap. 24 y los hubs del cap. 34: el skew no se fue a ninguna parte, solo cambió de dirección postal.
4. **La estrella patológica.** Hub central + 6 hojas, k=3, hub→P0, hoja j→j % 3: las hojas 3 y 6 viven con el hub (2 aristas locales) y las otras 4 dejan su arista CORTADA. Edge cut sobre un hub es el peor caso posible: todas las travesías del centro pagan red. Guárdalo: es el contraejemplo que motiva el peldaño 4.
5. **La ley general.** No es mala suerte del dataset: cualquier particionado por nodos de un grafo conexo no trivial deja cortes — la pregunta de ingeniería es cuántos y dónde. Eso es exactamente lo que compara el siguiente paso.

## 40.6 Solución evolucionada

Ocho gestos —un peldaño de la escalera cada uno—, cada uno con alternativa descartada.

**Gesto 1: comunidad vía Louvain (spacing puro del cap. 25).** `particionar_por_comunidad` llama a `louvain()` EXISTENTE —la detección ya es maquinaria probada— y usa cada comunidad como partición. Cuando Louvain encuentra más grupos que k, funde los pequeños primero (empate → menor miembro) hacia el grupo con el que comparten MÁS aristas (empate → menor miembro): fundirse con quien ya te hablas minimiza cortes nuevos. Resultado medido sobre el mini dataset: **630 cortes frente a los 1.062 del hash — un 40,7 % menos**, y la BFS desde el hub baja de 262 a **140 mensajes**. La dirección la fija el test `cortes_hash_vs_comunidad_en_mini_cuentas_exactas`: si algún día cambiara, el test FALLA — prohibido inflar (regla §8 del contrato). Descartado reescribir detección propia: duplicaría 600 líneas probadas (Blondel 2008, ya citado en el cap. 25).

**Gesto 2: el precio de la comunidad, y el codicioso con SU lección.** La comunidad respeta vecindarios pero REGALA el balance: tamaños **115 vs 23** (tam-max/tam-min) y carga por partición `[140, 208, 202, 337, 151, 337, 415, 610]` — una partición carga 610 mientras otra anda en 140. ¿Y el intermedio esperado? `particionar_balanceo_codicioso` procesa nodos por GRADO DESCENDENTE y coloca cada uno donde paga MENOS cortes incrementales. Resultado medido, reportado TAL CUAL: **corta el 100 % de las aristas (1.200 de 1.200)**, tamaños 204/0, carga `[600, 548, 457, 392, 272, 119, 12, 0]`. ¿Por qué tan patológico? Mira la mecánica: el hub va PRIMERO, todos sus vecinos están sin colocar, así que su coste es 0 en TODAS las particiones — empate, índice menor, a P0. Luego llegan SUS HOJAS: colocarse junto al hub cuesta 1 corte; colocarse lejos, 0 — huyen todas. Después llegan los vecinos-de-hojas, que ven vecinos ya desparramados… Cada decisión es LOCALMENTE óptima y JUNTAS fabrican lo peor: greedy SIN LOOKAHEAD, la lección que este capítulo reporta en lugar de esconder (patrón del ×0,86 del cap. 38). La moraleja no es «greedy malo» sino «greedy sin presupuesto ni lookahead en grafos power-law: cuidado».

**Gesto 3: invertir el cuchillo — vertex cut del hub.** `replicar_hub(hub, vecinos, k)` implementa la cura PowerGraph para el único caso donde es indiscutible: la estrella. Baseline edge-cut (hub→P0, hoja j→j % k), luego el hub se REPLICA en cada partición que guarde hojas y cada hoja conserva su arista local:

```text
ANTES (edge cut, 6 hojas, k=3):           DESPUÉS (vertex cut):
P0: {hub, hoja3, hoja6}                   P0: {hub*, hoja3, hoja6}
P1: {hoja1, hoja4}                        P1: {hoja1, hub*, hoja4}
P2: {hoja2, hoja5}                        P2: {hoja2, hub*, hoja5}
cortes: 4                                 cortes: 0
réplicas del hub: 0                       réplicas del hub: 3
```

Los 4 cortes se vuelven **0 pagando 3 réplicas** del centro. Ésa es la segunda divisa del skew: no desaparece al repartir, se MUDA de RAM a RED — el eco exacto del ×521 del cap. 39. Un vertex-cut GENERAL (tipo LDG, greedy sobre streaming) queda como reto experto; diluirlo aquí robaría foco al contraejemplo que motivó PowerGraph entero.

**Gesto 4: BFS entre particiones, con contador de mensajes.** `bfs_entre_particiones` sigue el modelo BSP de Pregel: rondas (*supersteps*) síncronas con doble buffer — los mensajes enviados en el nivel L llegan para el L+1. Cada descubrimiento cuya víctima vive en OTRA partición suma 1 a `mensajes_red`; `saltos_red` cuenta niveles con tráfico cruzado (la versión honesta de «latencia», sin milisegundos inventados). En el camino 0─1─2─3─4 con dueños alternos P0,P1,P0,P1,P0: cada paso cruza — **4 mensajes, 4 saltos**. Monolito: 0 y 0. Y el test-tesis `bfs_entre_particiones_cuenta_mensajes_y_coincide_con_bfs_local` exige lo importante: los alcanzables son IDÉNTICOS al BFS local — la distribución cambia el COSTE, jamás el RESULTADO.

**Gesto 5: hotspots y rebalanceo con precio.** `rebalancear(asignacion, aristas, particion_a_mover)` consolida la partición movida sobre la menos cargada y cuenta el traslado ANTES/DESPUÉS con la MISMA regla. Fixture dibujado en prosa: P0={0,1}, P1={2,3}, P2={4,5} con aristas 0─1, 2─3, 4─5 locales y 0─2, 1─4 cortadas (2 cortes). Movemos P0 → P1 (destino menos cargado, empate → índice menor): **2 nodos movidos, 2 aristas retocadas, cortes 2 → 1**. La aritmética no es casualidad: mover P hacia Q SÓLO deja de cortar las aristas P–Q; las P–X siguen cortándose — por eso `cortes_antes − cortes_despues == aristas entre P y Q`, y el test lo verifica contra `metricas_corte`.

**Gesto 6: consistencia — Raft determinista de 3 nodos.** El WAL del cap. 28 era la versión LOCAL de un log replicado: LSN, registro Commit, replay. Generalízalo y obtienes Raft (Ongaro-Ousterhout, USENIX ATC '14, pp. 305-319): un **log replicado** que sólo avanza por MAYORÍA. `EnjambreRaft` monta el clúster completo sin hilos, sockets ni RNG: la red es un `VecDeque` FIFO y el tiempo son **tics lógicos** — cada `tic()` avanza relojes, emite latidos del **líder**, dispara elecciones de quien perdió paciencia y drena el bus. Los timeouts de elección van ESCALONADOS FIJOS (10/15/20 tics): sacrificio DOCUMENTADO del anti-split-vote aleatorio del paper a cambio de determinismo total en CI. El protocolo lleva **término** (época monótona), votos con huella de log («no gana quien va retrasado», §5.4.1 del paper), **AppendEntries** con prefijo `(prev_indice, prev_termino)`, truncado ante conflictos, y **índice de compromiso** recalculado por mayoría de huellas confirmadas. Caída y reconexión explícitas (`caer`/`revivir`): el caído conserva término, voto y log — el disco sobrevive aunque la máquina no.

**Gesto 7: 2PC dibujado en prosa.** Raft ordena UN log para LAS COPIAS de UN fragmento; la atomicidad ENTRE fragmentos necesita otro protocolo ENCIMA: el **two-phase commit** (Petrov, *Database Internals*, O'Reilly 2019). Fase 1 (prepare): el coordinador escribe su intención en SU log y pregunta a cada participante si puede comprometer — cada uno persiste su voto en SU log replicado. Fase 2 (commit): con todos los SÍ, el coordinador escribe el equivalente al registro Commit del cap. 28 — EL PUNTO DE NO RETORNO: a partir de él, o se compromete todo o nada se atreve a abortar sin coordinación extra. Aquí SOLO diseño: implementarlo doblaría el módulo sin idea pedagógica nueva; lo que importa es ver DÓNDE encaja — el registro Commit que ya conoces es la primitiva.

**Gesto 8: el cobro del cap. 30 — deadlock ENTRE particiones.** Cada partición sólo ve SUS esperas, así que el ciclo T1↔T2 que cruza particiones es INVISIBLE para cualquier grafo local (el test lo exige: ambos `detectar_ciclo()` locales devuelven `None`). `fusionar_grafos_espera` ensambla las esperas LOCALES en un grafo GLOBAL usando SÓLO la API pública del cap. 30 (`agregar_espera`/`quitar_tx`/`detectar_ciclo`) — sin duplicar una línea de su DFS — y el ciclo salta a la vista. Requisito operativo documentado: ids de transacción ÚNICOS EN TODO EL CLÚSTER (prefijo por partición: A=101, B=202); si dos particiones numeran sus tx desde 1, la fusión mezclaría identidades. La pieza que el cap. 30 dejó «enchufada» lleva corriente por fin.

## 40.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `crates/vol2-liradb/src/cap40_distribucion.rs` (**1.835 líneas**, std puro, 14 tests). El cableado es el mínimo posible — dos líneas aditivas en `lib.rs`: `pub mod cap40_distribucion; pub use cap40_distribucion::*;`. CERO dependencias nuevas, CERO cambios en caps. 7-39, goldens intactos. Y la desviación DECLARADA del patrón caps. 34/38/39: **NO hay cuarto `[[bench]]`** — decisión #11 del contrato. Ninguna afirmación de este capítulo es sobre tiempo: cortes, réplicas, mensajes y entradas de log son enteros EXACTOS que viven en los tests; cronometrar el particionado repetiría el bench de Louvain del cap. 25 sin hipótesis nueva. Aquí la moneda son enteros, no µs: los enteros SON la física del sistema.

Las firmas que sostienen el edificio:

```rust
pub struct AsignacionParticion { pub num_particiones: usize, pub dueno: Vec<u32> }
impl AsignacionParticion {
    pub fn monolito(n: usize) -> Self;                 // k=1: cortes = 0
    pub fn dueno_de(&self, id: u32) -> Option<u32>;    pub fn nodos_de(&self, p: usize) -> Vec<u32>;
    pub fn tamanos(&self) -> Vec<usize>;
}
pub fn particionar_hash(n: usize, k: u32) -> AsignacionParticion;              // FNV-1a % k (cap. 15)
pub fn particionar_por_comunidad(store: &dyn GraphStore, k: usize)             // louvain() + fusión
    -> AsignacionParticion;
pub fn particionar_balanceo_codicioso(aristas: &[(u32, u32)], k: usize)        // grado descendente
    -> AsignacionParticion;
pub struct MetricasCorte { pub cortes_arista: u64, pub nodos_frontera: u64,
                           pub factor_replicacion: f64, pub tam_max: usize, pub tam_min: usize }
pub fn metricas_corte(a: &AsignacionParticion, aristas: &[(u32, u32)]) -> MetricasCorte;
pub struct InformeVertexCut { pub cortes_antes: u64, pub cortes_despues: u64, pub replicas_hub: usize }
pub fn replicar_hub(hub: u32, vecinos: &[u32], k: usize) -> InformeVertexCut;
pub struct ResultadoBfsDistribuido { pub visitados: Vec<u32>, pub mensajes_red: u64, pub saltos_red: u64 }
pub fn bfs_entre_particiones(adj: &[Vec<u32>], a: &AsignacionParticion, origen: u32)
    -> ResultadoBfsDistribuido;
pub fn carga_por_particion(a: &AsignacionParticion, grados: &[u32]) -> Vec<u64>;
pub struct InformeRebalanceo { pub nodos_movidos: usize, pub aristas_retocadas: usize,
                               pub cortes_antes: u64, pub cortes_despues: u64 }
pub fn rebalancear(a: &mut AsignacionParticion, aristas: &[(u32, u32)], mover: usize)
    -> InformeRebalanceo;
pub enum RolRaft { Seguidor, Candidato, Lider }
pub struct EntradaLog { pub termino: u64, pub comando: u64 }
pub enum MensajeRaft { PideVoto { termino, candidato }, Voto { termino, candidato },
                       Entradas { termino, lider, prev_indice, prev_termino, entradas,
                                  compromiso_lider },
                       Acuse { termino, seguidor, exito, coincide_hasta } }
pub struct NodoRaft { pub id: u32, pub rol: RolRaft, pub termino: u64, pub voto_de: Option<u32>,
                      pub log: Vec<EntradaLog>, pub indice_compromiso: usize,
                      pub timeout_tics: u64, pub vivo: bool, /* …índices de líder… */ }
pub struct EnjambreRaft { pub nodos: Vec<NodoRaft>, pub latido_cada_tics: u64, pub tics_totales: u64 }
impl EnjambreRaft {
    pub fn nuevo(ids: &[u32], base_tics: u64, escalon_tics: u64) -> Self;  // timeouts escalonados
    pub fn tic(&mut self);  pub fn tics(&mut self, n: u64);
    pub fn proponer(&mut self, comando: u64) -> bool;   // false si no hay líder: sin mayoría no hay servicio
    pub fn lider(&self) -> Option<u32>;
    pub fn caer(&mut self, id: u32);  pub fn revivir(&mut self, id: u32);
}
pub fn fusionar_grafos_espera(locales: &[&GrafoEspera]) -> GrafoEspera;    // deuda cap. 30
pub fn informe_distribucion_reproducible_sobre_mini(store: &MemoryStore) -> String; // enteros, NO µs
```

Cuatro decisiones visibles en esas firmas, con su porqué:

- **La red es un bus FIFO en el mismo proceso.** Contadores exactos y reproducibles en CI; sockets reales serían indemostrables en tests y no-determinísticos (metodología del cap. 34; precedentes: `ContandoStore` del cap. 26, pasos WCOJ del cap. 39). Jamás latencia simulada: contar es reproducible, cronometrar simulado es humo (decisión #8).
- **El factor de replicación es OPERATIVO, no nominal.** Documentado en el código: con edge-cut puro cada nodo vive una vez (factor bruto 1.0 trivial); se cuenta el gather de PowerGraph — cuántas copias harían falta para evaluar el nodo localmente en cada máquina vecina.
- **Timeouts escalonados FIJOS, trade-off escrito.** El randomizado del paper existe para repartir impacientos en elecciones concurrentes; aquí el escalonado garantiza que la primera elección sea PREDECIBLE (determinismo > realismo, decisión #9).
- **El informe está pineado byte a byte por test.** Dos ejecuciones consecutivas deben ser idénticas (`assert_eq!(a, b)`) — sin dataset determinista ni orden estable, la tabla que lees abajo sería folclore.

## 40.8 Prueba de fuego

Primero el bucle rápido, en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap40

running 14 tests
test cap40_distribucion::tests_distribucion::corte_de_arista_frontera_y_factor_replicacion_contados_a_mano ... ok
test cap40_distribucion::tests_distribucion::corte_de_vertice_del_hub_elimina_cortes_pagando_replicas ... ok
test cap40_distribucion::tests_distribucion::hash_modulo_k_asigna_todo_y_balancea ... ok
test cap40_distribucion::tests_distribucion::grafo_espera_global_detecta_deadlock_entre_particiones ... ok
test cap40_distribucion::tests_distribucion::raft_appendentries_replica_el_log_en_los_seguidores ... ok
test cap40_distribucion::tests_distribucion::raft_eleccion_por_tics_elige_lider_con_mayoria ... ok
test cap40_distribucion::tests_distribucion::raft_no_compromete_sin_mayoria_de_acuses ... ok
test cap40_distribucion::tests_distribucion::rebalanceo_mueve_una_particion_y_recuenta_cortes ... ok
test cap40_distribucion::tests_distribucion::raft_seguidor_reconectado_alcanza_el_log_del_lider ... ok
test cap40_distribucion::tests_distribucion::hub_concentra_la_carga_en_su_particion ... ok
test cap40_distribucion::tests_distribucion::bfs_entre_particiones_cuenta_mensajes_y_coincide_con_bfs_local ... ok
test cap40_distribucion::tests_distribucion::cortes_hash_vs_comunidad_en_mini_cuentas_exactas ... ok
test cap40_distribucion::tests_distribucion::particiones_comunidad_respectan_grupos_de_louvain ... ok
test cap40_distribucion::tests_distribucion::informe_distribucion_reproducible_sobre_mini ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 693 filtered out; finished in 0.09s
```

Catorce verdes, workspace entero en **892 ALL_GREEN** con goldens intactos. Cuatro son TESIS, no comprobaciones accesorias: `cortes_hash_vs_comunidad_en_mini_cuentas_exactas` fija la dirección de la pregunta crítica (si la comunidad dejara de cortar menos que el hash, el test ROMPE); `bfs_entre_particiones_cuenta_mensajes_y_coincide_con_bfs_local` exige alcanzables IDÉNTICOS al BFS local — la distribución no cambia QUÉ se alcanza; `raft_no_compromete_sin_mayoria_de_acuses` demuestra que sin mayoría el servicio SE BLOQUEA (consistencia sobre disponibilidad); y `grafo_espera_global_detecta_deadlock_entre_particiones` cobra la deuda del cap. 30 con un ciclo invisible para ambos grafos locales. Velocidad sin estas equivalencias sería marketing.

Ahora la TABLA del capítulo — salida REAL del informe reproducible (contadores exactos, sin ni un µs):

```text
=== Informe de distribución (cap. 40) ===
dataset: 400 nodos, 1200 aristas dirigidas | k=8 particiones | hub: nodo 27 (grado 37)
-- estrategias x metricas (contadores exactos; BFS desde el hub) --
estrategia   cortes  frontera   factor tam-max  tam-min bfs-mensajes  bfs-saltos
hash           1062       296    3.850      50       50          262           4
comunidad       630       241    2.985     115       23          140           4
codicioso      1200       306    3.445     204        0          303           4
-- carga por partición (suma de grados de sus nodos) --
hash        [299, 272, 281, 285, 281, 320, 349, 313]
comunidad   [140, 208, 202, 337, 151, 337, 415, 610]
codicioso   [600, 548, 457, 392, 272, 119, 12, 0]
hotspot (hash): el hub 27 vive en la partición 6, que concentra 349 de 2400 de carga (14.5%)
(sin tiempos: los enteros SON la física del sistema — regla del cap. 34, decisión #11)
```

Dos lecturas, ambas obligatorias. **La comunidad gana la columna de cortes: 630 contra 1.062, un 40,7 % menos, y su BFS paga 140 mensajes contra 262.** Pero mira la fila de carga: `[140, 208, 202, 337, 151, 337, 415, 610]` — una partición carga 610 mientras otra anda en 140, tamaños 115 contra 23. **El hash gana el balance (50 exactos en cada una) y pierde la localidad; la comunidad gana la localidad y regala el balance.** No hay carrera con ganador: es un TRADE-OFF medido, y elegir depende de qué escasez temas más — CPU uniforme (hash) o red saturada (comunidad). El codicioso, con su 100 % de cortes, es la tercera columna de la lección: un algoritmo «de equilibrio» sin lookahead puede ser PEOR que ambos extremos.

La prueba de fuego continúa con las piezas pequeñas verificadas a mano: la estrella del hub pasa de **4 cortes a 0 pagando 3 réplicas** del centro (`replicar_hub`; con m hojas y m < k, tantas réplicas como hojas); el rebalanceo del fixture mueve 2 nodos, retoca 2 aristas y baja los cortes de 2 a 1 — exactamente las aristas P–Q que dejaron de cruzar. Y el clúster Raft, conducido por tics:

```text
tics:   1 … 9                10                                  11 …
nodo 0: esperando            timeout(10) → Candidato, término 1, PideVoto → 1 y 2
nodo 1: esperando            ◄── Voto(término 1, Some(0))       ┐ mayoría 2-de-3
nodo 2: esperando            ◄── Voto(término 1, Some(0))       ┘
                             nodo 0 proclamado LÍDER, término 1, latidos cada 5 tics
```

La elección converge en el **tic 10** — ni antes (nadie ha perdido la paciencia) ni muy después (el escalonado 10/15/20 garantiza que el nodo 0 impaciente primero GANE antes de que nadie más se impaciente) — y se mantiene estable sesenta tics más con el mismo líder y el mismo término. Tras `proponer(100)` y `proponer(200)`: logs **idénticos byte a byte** en los tres nodos — `[{término 1, comando 100}, {término 1, comando 200}]` — con **compromiso = 2 en los tres** (el índice del líder viaja en cada AppendEntries y los seguidores lo adoptan). Luego la prueba de fuego de verdad: `caer(1)`, `caer(2)` — mayoría CAÍDA, queda el líder solo. `proponer(77)` devuelve true (la entrada VIVE en su log), pero cuarenta tics después el **índice de compromiso sigue CONGELADO en 0**: sin mayoría de acuses, el líder ni siquiera puede saber si su entrada existe en el clúster — Raft garantiza consistencia, NO disponibilidad. Y la reconexión: el seguidor que estuvo caído durante tres propuestas revive con log vacío y alcanza al líder **sólo con los latidos** — AppendEntries con sufijo pendiente, sin una sola propuesta nueva — en menos de 200 tics. Cierra el círculo el deadlock entre particiones: T1 (id 101, partición A) espera el nodo 9 que retiene T2 (id 202, partición B), y T2 espera el nodo 7 de A; cada grafo local ve UNA arista y ningún ciclo; el grafo GLOBAL fusionado encuentra `[T1, T2]` — y `quitar_tx(T1)` lo mata. La pieza del cap. 30, enchufada y funcionando.

## 40.9 Qué hemos sacrificado

1. **Sin red real ni RPC ni serialización.** La red es un bus FIFO en el mismo proceso y los comandos Raft son enteros opacos. Contadores reproducibles a cambio de que nada de esto viaja por un cable — el DISEÑO de la distribución, no su despliegue.
2. **Sin runtime asíncrono.** Ni tokio ni tareas: un tic() secuencial es todo el scheduler. La versión concurrente es el anzuelo natural hacia el Vol.III.
3. **Sin cambios de pertenencia, log compaction ni snapshots.** El clúster es fijo (3 nodos) y los logs crecen para siempre: sin joint consensus, sin truncado por snapshot. Operativamente inviable a largo plazo; pedagógicamente suficiente para entender QUÉ falta.
4. **Sin tolerancia bizantina.** Todos los nodos son honestos: Raft tolera CAÍDAS, no mentiras (eso es PBFT, otro libro).
5. **2PC solo diseñado.** Preparar/comprometer dibujado en prosa sobre el registro Commit del WAL; implementarlo habría duplicado el módulo sin idea nueva.
6. **Timeouts escalonados en vez del randomizado de producción.** Sacrificio DOCUMENTADO (decisión #9): el anti-split-vote aleatorio del paper se cambia por determinismo total; con 3 nodos y escalones 10/15/20 la primera elección nunca es concurrente.
7. **Factor de replicación operativo y capa educativa fuera del motor.** Definido en el doc del código (gather de PowerGraph), no el nominal; y ni `Executor` ni `GraphStore` tocados: el particionado vive en estructuras propias que LEEN del store — contratos de caps. 7-39 intactos.

## 40.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j** ofrece clustering causal —réplicas con Raft por debajo y sesiones causales que apuntan a la última escritura que conoces— y Fabric para federar varios grafos: exactamente las dos mitades de este capítulo (log replicado + fronteras entre shards). **JanusGraph** delega el almacenamiento en backends distribuidos (HBase, Cassandra…) y reparte el grafo por rangos de id de vértice, con la frontera pagándose en lecturas remotas. **TigerGraph** hace particionado automático con réplicas y habla abiertamente de minimizar el tráfico entre fragmentos — nuestro conteo de mensajes, con presupuesto comercial. Y para CONSENSO en producción, los campeones son **etcd** y **Consul**: Raft corriendo desde hace años sosteniendo configuraciones de clústeres (etcd es, literalmente, el almacén detrás de Kubernetes). Según ADR-001, el contraste honesto que CIERRA el volumen: **Kùzu/LadybugDB es un motor EMBEBIDO MONONODO** (Jin et al., CIDR 2023, CC-BY 4.0; relato verificado: Waterloo → CIDR 2023 → Apple oct-2025 → repo archivado → fork LadybugDB) — no distribuye, y no lo considera un defecto: para su nicho, añadir fronteras sería añadir factura sin necesidad. Que es, palabra por palabra, la wisdom de este capítulo: **si tu grafo cabe en una máquina, distribuir no resuelve tu problema — crea uno nuevo.**

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (34+40): toma la estrella del hub con m=12 hojas y k=4. ANTES de correr nada, calcula a mano los cortes del baseline edge-cut (hub→P0, hoja j→j % 4), las réplicas del vertex cut y el factor de replicación de ambos. Luego verifica con `replicar_hub` y `metricas_corte`. Criterio: predicción escrita primero, medición después — si falla, explica QUÉ término ignoraste.
- *Intermedio* (21/25+40): PREDECIR por escrito la tabla completa hash-vs-comunidad del mini dataset (cortes, tam-max/tam-min, mensajes BFS) ANTES de correr `informe_distribucion_reproducible_sobre_mini`. Ayuda: los grados del generador del cap. 34 y la intuición «Louvain agrupa vecindarios». Verifica, y explica tus desviaciones — ¿subestimaste el desbalance de las comunidades grandes? ¿Contaste aristas paralelas como una?
- *Experto* (39+40): extiende `replicar_hub` a un greedy vertex-cut GENERAL tipo LDG: vértices en streaming, cada uno a la partición con más vecinos ya colocados, capacidad 1+ε·(n/k). Implementa, mide el factor de replicación y los cortes sobre el mini dataset, y compáralos contra las TRES estrategias edge-cut de la tabla. Restricciones: std puro, cero cambios en caps. anteriores, suite ALL_GREEN con TU test dentro.

## 40.11 Lo que te llevas

- **La frontera es la factura.** Todo lo que sabes hacer sigue valiendo DENTRO de cada máquina; lo nuevo es que cada arista cruzada es un mensaje futuro. Localidad = combustible.
- **Hash balancea y destroza vecindarios; comunidad respeta vecindarios y desbalancea la carga.** Medido: 1.062 cortes/262 mensajes contra 630/140, con cargas 50-uniformes contra [140…610]. Trade-off, no carrera.
- **Greedy sin lookahead puede ser peor que ambos extremos**: el codicioso cortó el 100 % de las aristas — decisiones localmente óptimas, desastre global. Reportado tal cual.
- **Edge cut replica nodos-frontera; vertex cut replica aristas.** El hub demuestra la cura PowerGraph: 4 cortes → 0 pagando 3 réplicas. El skew no desaparece al repartir: se muda de RAM a RED (eco del ×521 del cap. 39).
- **Replicar es consenso, no backups.** Copias sin protocolo divergen; Raft ordena un log por mayoría — y sin mayoría el compromiso se congela (consistencia sobre disponibilidad).
- **Raft no da transacciones distribuidas**: ordena UN fragmento; la atomicidad ENTRE fragmentos necesita 2PC encima — y el registro Commit de tu WAL es la primitiva.
- **El deadlock que cruza particiones sólo se ve en el grafo-espera GLOBAL** — la pieza del cap. 30, enchufada por fin, con ids de tx únicos en el clúster.
- **Y la wisdom del volumen entero**: si cabe en una máquina, no distribuyas — cada corte que diseñas hoy es un mensaje que pagarás mañana.

## 40.12 Ojo, cuidado con…

- **Leer solo la columna de cortes.** La comunidad reduce cortes un 40,7 % y te planta una partición con carga 610 entre otras de 140: el hotspot fundirá UNA máquina mientras las demás dormitan. Lee cortes Y carga, siempre juntos.
- **Confundir balance con bondad.** El codicioso «equilibraba» por diseño y cortó el 100 % de las aristas. Un algoritmo sin lookahead en grafos power-law optimiza exactamente lo equivocado.
- **Copiar sin protocolo.** Dos réplicas actualizadas por caminos distintos NO convergen solas. Antes de «tener backups», ten consenso.
- **Pedirle a Raft atomicidad entre shards.** Comprometer dos entradas en dos logs Raft independientes NO es atómico: necesitas 2PC encima, y entonces entiendes por qué los sistemas serios lo evitan cuando pueden.
- **Mayoría caída ≠ pérdida de datos.** El log crece en el líder pero el compromiso NO avanza: la entrada existe y NO está comprometida — distinción que separa a quien entiende el protocolo de quien lo usa.
- **Ids de transacción locales al fusionar grafos de espera.** Si dos particiones numeran tx desde 1, la fusión mezcla identidades y detecta ciclos falsos (o peor, no detecta los reales). Prefijo por partición, siempre.
- **Creer que un buen particionador elimina los cortes.** Son inevitables en todo grafo conexo no trivial; la ingeniería está en QUÉ cortas y cuánto cobras por ello.

## 40.13 Pin de batalla

> *«En una máquina el coste era CPU y memoria; en muchas, el coste es cruzar la frontera. Hash balancea la carga y destroza vecindarios; comunidad respeta vecindarios y desbalancea la carga; y sin mayoría, el log no compromete. Cada corte que diseñas hoy es un mensaje que pagarás mañana.»*

## 40.14 Si solo lees 30 segundos

Distribuir un motor de grafos no escala gratis: convierte cada arista cruzada en un mensaje. **Edge cut** (cortas aristas) replica nodos-frontera; **vertex cut** (cortas vértices) replica aristas — el hub de la estrella pasa de 4 cortes a 0 pagando 3 réplicas (PowerGraph, OSDI '12). Sobre el mini dataset (400 nodos, 1.200 aristas, k=8): **hash** (FNV-1a del cap. 15) da 50 nodos exactos por partición pero **1.062 cortes (88,5 %)** y 262 mensajes de BFS; **comunidad** (Louvain del cap. 25) baja a **630 cortes (−40,7 %)** y 140 mensajes pero desbalancea (tamaños 115/23, carga hasta 610); **codicioso** sin lookahead corta el 100 % — lección reportada. La BFS distribuida por supersteps cuenta mensajes (nunca latencias) y exige los mismos alcanzables que la local: la distribución cambia el COSTE, jamás el RESULTADO. Consistencia: el WAL del cap. 28 generalizado es un **log replicado** — Raft (USENIX ATC '14, pp. 305-319) elige líder por mayoría en el tic 10 (tics lógicos, timeouts escalonados 10/15/20), replica logs byte a byte idénticos, y con mayoría caída el compromiso SE CONGELA: consistencia, no disponibilidad; la atomicidad ENTRE fragmentos es 2PC encima (Petrov 2019). Deadlock entre particiones: grafo-espera GLOBAL (cap. 30 enchufado). Y la moraleja que cierra el volumen: Kùzu/LadybugDB es embebido mononodo por decisión — si tu grafo cabe en una máquina, distribuir solo añade fronteras.

## 40.15 Una historia pequeña

Stanford, 2013. Diego Ongaro lleva años peleando con Paxos, el algoritmo de consenso canónico, famoso tanto por resolver el problema de las réplicas como por ser prácticamente indescifrable — el propio Leslie Lamport había escrito dos papers y una pseudonotación griega, y aun así generaciones de doctorandos se perdían. Junto a Dennis Ousterhout, Ongaro decide algo radical para un paper de sistemas: nombrar la COMPRENSIBILIDAD como criterio de diseño PRIMARIO y ponerla en el propio título — «In Search of an Understandable Consensus Algorithm» (USENIX ATC '14, pp. 305-319). Raft no fue más rápido ni más teórico que Paxos: fue DISEÑADO para poder explicarse — decomposición en subproblemas (elección de líder, replicación, seguridad), estados reducidos, y hasta estudios con estudiantes para verificar que se entendía mejor. La lección operativa: la claridad no es un lujo posterior al funcionamiento — fue EL criterio de diseño, y por eso hoy Raft corre en etcd, Consul, CockroachDB y media industria, mientras el «más elegante» se estudia y Raft se despliega. Este capítulo le debe más de lo que parece: nuestro EnjambreRaft cabe en 600 líneas y se explica en una sección PRECISAMENTE porque el original fue diseñado para eso. Elegir la versión que tu yo de dentro-de-seis-meses pueda depurar a las dos de la mañana ES una decisión de arquitectura.

## Ejercicios resueltos

**1. Cuenta a mano las métricas del fixture de tres particiones y verifica el factor 2,2.** Asignación P0={0,1}, P1={2,3}, P2={4}; aristas 0─1, 1─2, 2─3, 3─4, 0─3. Cortes: extremos en particiones distintas → 1─2 (P0-P1), 3─4 (P1-P2), 0─3 (P0-P2) ⇒ **3**. Frontera: todo nodo incidente a algún corte ⇒ {0,1,2,3,4} ⇒ **5**. Réplicas operativas (1 + nº de particiones vecinas distintas): nodo 0 ve {P0,P2} ⇒ 2; nodo 1 ve {P0,P1} ⇒ 2; nodo 2 ve {P0,P1} ⇒ 2; nodo 3 ve {P0,P1,P2} ⇒ 3; nodo 4 ve {P1} ⇒ 2. Σ = 11 ⇒ 11/5 = **2,2**. Tamaños {2,2,1} ⇒ máx 2, mín 1. Contraste: el monolito da 0 cortes, 0 frontera y factor 1,0. Verificación: `corte_de_arista_frontera_y_factor_replicacion_contados_a_mano`.

**2. ¿Por qué el codicioso corta EXACTAMENTE el 100 % de las aristas, y por qué lo reportamos en vez de ajustarlo?** Mecánica: el orden es grado descendente; el hub (grado 37) va primero con TODOS sus vecinos sin colocar, así que su coste incremental es 0 en las 8 particiones — empate total, índice menor, a P0. Sus hojas van después: junto al hub cuestan 1 corte, lejos cuestan 0 ⇒ huyen en masa. Los siguientes en la cola ven vecinos ya sembrados en particiones distintas y eligen el mínimo local, que rara vez coincide con el de sus vecinos posteriores. Cada colocación es óptima DADO lo ya colocado; la composición de óptimos locales no lo es — greedy sin lookahead ni presupuesto. Lo reportamos intacto por la regla de la casa (§8 del contrato): si la medición contradice la expectativa, se publica, se explica y se aprende — inflar sería folclore. Verificación: `cortes_hash_vs_comunidad_en_mini_cuentas_exactas` + fila codicioso del informe.

**3. Retrieval sin pistas: recita la escalera, la taxonomía de cortes y el flujo de una elección Raft.** Cierra el libro. Escalera: dificultad (arista cruzada) → sharding por nodos (hash) → edge/vertex cut (dos facturas) → fronteras (hub replicado) → hash vs comunidad (tres estrategias medidas) → hotspots → consultas remotas (supersteps contando mensajes) → Raft (mayoría) → rebalanceo (coste antes/después) → cierre de deudas (deadlock global + hexágono). Taxonomía: edge cut factura cortes + réplicas de NODOS-frontera; vertex cut factura réplicas de ARISTAS. Elección Raft: timeout vencido → Candidato (término+1, voto propio, PideVoto) → mayoría 2-de-3 → Líder (latidos AppendEntries) → acuses → compromiso por mayoría de huellas. Si pusiste el vertex cut ANTES del hash (sin primera solución no hay límites que curar) o el compromiso antes de los acuses (se compromete lo confirmado por mayoría, no lo propuesto), relee §§40.3/40.6: el orden ES el argumento.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 34+40).** Desarrolla el reto esencial del §40.10: estrella de 12 hojas con k=4, cortes/réplicas/factor del baseline Y del vertex cut calculados A MANO antes de correr. Verificación: patrón de `corte_de_vertice_del_hub_elimina_cortes_pagando_replicas`; fallará si olvidas que las hojas j ≡ 0 (mod 4) ya viven en P0 y NO cuentan como cortes.

**Intermedio (predecir; 21/25+40).** Predicción por escrito de la tabla completa (cortes, frontera, factor, tam-max/tam-min, mensajes BFS para las tres estrategias) sobre el mini dataset, con margen de error declarado. Luego `informe_distribucion_reproducible_sobre_mini` y contraste. Criterio: si acertaste direcciones pero no magnitudes, identifica el término ignorado (fusión de comunidades pequeñas, aristas paralelas, BFS desde el hub en vez de desde un nodo medio) — el wisdom está en explicar la desviación, no en acertar.

**Experto (crear y medir; 39+40).** Greedy vertex-cut general tipo LDG sobre el mini dataset: streaming por orden de grado descendente, partición elegida por máxima superposición de vecinos ya colocados con capacidad 1+ε·(n/k), ε parametrizable. Entregables: factor de replicación y cortes comparados contra la tabla del capítulo, test de equivalencia de invariantes (todo vértice asignado a ≥1 partición, capacidad respetada), y un párrafo respondiendo: ¿cuándo compensa pagar réplicas de vértices en vez de cortes de aristas? Restricciones: std puro, suite ALL_GREEN con TU test dentro.

## Para profundizar

- **Malewicz, Austern, Bik, Dehnert, Horn, Leiser y Czajkowski, «Pregel: A System for Large-Scale Graph Processing» (SIGMOD 2010)** — el modelo «think like a vertex», supersteps y mensajería; el sistema que este capítulo emula con su BFS por rondas.
- **Gonzalez, Low, Gu, Bickson y Guestrin, «PowerGraph: Distributed Graph-Parallel Computation on Natural Graphs» (OSDI 2012, pp. 17-30)** — power-law graphs, edge cut vs vertex cut, factor de replicación y el greedy sobre streaming que inspiró el reto experto. La fuente de TODAS las definiciones de métricas del capítulo.
- **Ongaro y Ousterhout, «In Search of an Understandable Consensus Algorithm» (USENIX ATC 2014, pp. 305-319)** — Raft: elección, AppendEntries, compromiso por mayoría; versión extendida y material didáctico en raft.github.io. Nuestro EnjambreRaft es una maqueta fiel con tics lógicos.
- **Alex Petrov, *Database Internals* (O'Reilly 2019), parte III** — particionado, replicación y consenso desde la perspectiva de motores: donde el 2PC dibujado en prosa aquí está desarrollado con sus modos de fallo.
- **Jin, Feng, Chen, Liu y Salihoğlu, «KÙZU Graph Database Management System» (CIDR 2023, CC-BY 4.0)** — atribución según ADR-001: el contraste embebido-mononodo que cierra el volumen (clean-room conceptual: cero código copiado).
- **ISO/IEC 39075:2024 (GQL)** — el estándar de consulta de grafos propiedad; la consulta que hoy corre en una máquina es la misma que mañana cruzará particiones federadas (anzuelo al Vol.III).
- **Blondel, Guillaume, Lambiotte y Lefebvre (2008)** — Louvain, ya citado en el cap. 25: aquí reutilizado como política de colocación, no como detector de grupos.
- Dentro del libro: cap. 15 (FNV-1a — el hash reutilizado), cap. 24 (centralidad — por qué el hub domina), cap. 25 (Louvain y `Particion`), cap. 28 (WAL y registro Commit — el log local que se vuelve replicado), cap. 30 (`GrafoEspera` — la deuda cobrada), cap. 34 (dataset determinista y hubs), cap. 36 (el hexágono, hoy respondido), cap. 37 (el mapa de producción y su frente abierto), cap. 39 (los contadores WCOJ y el ×521, hoy mudados de RAM a RED).

## Mini-diálogo: en guardia nocturna

> — Son la una de la mañana. Alerta del clúster: UNA máquina del cliente al 97 % de CPU y creciendo; las otras siete dormitando. Ayer estaba bien. ¿Ampliamos el clúster?
>
> — Antes, la factura de particiones del cap. 35: ¿qué estrategia de sharding tienen?
>
> — Comunidad. Se lo pusieron porque «minimizaba el tráfico entre máquinas».
>
> — Claro: minimiza CORTES — 630 contra 1.062 en nuestro dataset — y a cambio te desbalancea la CARGA: mira sus métricas, apuesto a que el hub del grafo vive justo en la partición caliente. Una máquina fundiéndose, siete bostezando: eso no es falta de hierro, es un HOTSPOT con nombre y apellido.
>
> — Entonces…
>
> — Tres opciones, con precio contado: mover carga con rebalanceo (barato, pero sólo consolida particiones), pasar esa partición a hash (uniformiza, pero sube los cortes de TODO el grafo), o vertex-cut del hub: replica el centro donde hacen falta sus vecinos y sus aristas dejan de cruzar — pagamos réplicas, no máquinas nuevas.
>
> — ¿Y cuál eliges?
>
> — Ninguna sin números: cortes actuales, carga por partición, mensajes de las consultas calientes. Con los contadores delante decides con datos; a ciegas, ampliar el clúster es comprar cubos más grandes para una fuga de partición. Y mañana, en el post-mortem, se añade a la factura del diseño: cada corte aceptado aquel día es el mensaje que estamos pagando esta noche.

---

*(FIN DEL VOL.II — «Construye LiraDB». El hexágono del cap. 36 quedó respondido: cada pieza del mapa sobrevive al reparto POR MÁQUINA —CSR, catálogo, WAL, MVCC, optimizador—; lo que aprendiste de nuevo fue la contabilidad de la frontera: cortes, réplicas, mensajes y mayoría. Al EPÍLOGO, con las preguntas abiertas heredadas y las nuevas: ¿paralelizar leapfrog morsel-driven (39)? ¿orden dinámico de variables? ¿columnas en disco? ¿red real con runtime asíncrono? ¿compaction y snapshots del log replicado? ¿y cuando la MEMORIA de un agente necesita un grafo? El Vol.III — «Grafos en la era de la IA: KB-Lira» — empieza exactamente ahí.)*
