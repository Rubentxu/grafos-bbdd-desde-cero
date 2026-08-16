# CONTRATO DE CAPÍTULO — Vol.II Cap. 30: Snapshots y concurrencia — MVCC limitado

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap30_mvcc.rs` (~1.158 líneas,
> 21 tests en `tests_mvcc`, ALL_GREEN — el contador del workspace pasa de
> 707 a 728 al añadir este módulo, sin doctests, sin crates externas).
> Decisiones reales: `liradb-workspace/book-context/MIGRATION-PATTERN.md`
> §35. Prerrequisitos: caps. 8 (`GraphStore`/`MemoryStore` hexagonal), 26
> (proyección como foto inmutable: el «snapshot» ya era vocabulario), 27
> (`Operacion`, `Anomalia::LecturaSucia/ActualizacionPerdida`, «único
> escritor por el borrow checker»), 28-29 (durabilidad y ARIES, el `Ts`
> lógico es el `LSN` con cambio de nombre). Este capítulo CIERRA la
> Parte VI (ACID): reta al lector a distinguir lectura sucia y lost
> update (las anomalías del cap. 27) del write skew (la anomalía que
> SOBREVIVE al cap. 30). Ganchos: cap. 31 (CLI: el `MvccStore` será el
> corazón del REPL) y cap. 40 (distribución: la concurrencia REAL de
> varios procesos abre write skew — SSI entra en juego).
> Preguntas críticas cubiertas (CORPUS línea 363-368): «MVCC con timestamp
> por tupla; garbage collection de versiones».

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el modelo «múltiples lectores, un único
  escritor» del cap. 27 (el `&mut dyn GraphStore` es el cerrojo, y por
  eso ninguna anomalía de aislamiento PODÍA ocurrir); el vocabulario
  ACID tipado del cap. 27 (`GarantiaAcid`, `NivelGarantia`, `Anomalia::{
  LecturaSucia, ActualizacionPerdida}`, `Operacion::{PutNode, PutEdge,
  DeleteNode, DeleteEdge}`); la recuperación ARIES del cap. 29
  (análisis-redo-undo, dirty page table, CLR documentadas); el
  `MemoryStore` del cap. 8 (inserción estricta: `put_node` sobre id
  existente devuelve `DuplicateNode`); el store como PUERTO `&dyn
  GraphStore` (cambia el backend sin tocar el resto); el WAL del cap.
  28 (el `LSN` es el «hermano mayor» del `Ts` de este capítulo); la
  proyección del cap. 26 como foto inmutable (el «snapshot» ya era
  vocabulario, pero sin maquinaria).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1)
  «un snapshot es una copia física de los datos» — no: es un
  TIMESTAMP lógico (un `u64` que crece); cada elemento lleva su cadena
  y el snapshot es la versión visible ESE timestamp; (2) «MVCC exige
  varios escritores» — no aquí: el `&mut self` del commit sigue
  garantizando un único escritor (los `ts` no se pisan); lo que MVCC
  MULTIPLICA son los lectores — `&self` clona y los lectores NUNCA
  bloquean al escritor; (3) «los snapshots ocupan el doble de memoria»
  — parcialmente: mientras una versión sea visible para ALGÚN snapshot
  vivo, la cadena la retiene; `gc` la devuelve; (4) «con MVCC se acabó
  el problema de la concurrencia» — no: en Snapshot Isolation el
  WRITE SKEW sigue pasando (dos tx disjuntas que leen y reescriben a
  partir del mismo snapshot producen un resultado no serializable);
  cerrar eso exige Serializable SI con predicate locks, fuera del
  alcance; (5) «los deadlocks pueden aparecer en cualquier momento» —
  no aquí: con un único escritor (`&mut self` lo prohíbe) no HAY
  ciclos en el grafo de espera; la estructura existe como vocabulario
  y anzuelo, no como código en uso; (6) «un `Ts` mayor significa
  versión más reciente» — sí, pero la consistencia de un snapshot NO
  es cronológica: es por CONSTRUCCIÓN (cadena append-only, lectura por
  valor).
- **NO debe saber todavía**: timestamp físico (`SystemTime::now` por
  nodo — relojes desviados entre máquinas abren problemas nuevos),
  concurrencia REAL de varios escritores con cerrojos (la Parte VIII),
  Serializable SI con predicate locks (Cahill et al. 2008, fuera del
  alcance), GC en background como tarea programada, vector clocks /
  true time (Parte VIII). Se nombran como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: MVCC como CAPA hexagonal sobre `GraphStore`; `VersionNode`/
  `VersionEdge` `{ts_begin, ts_end?, valor}` y CADENA por elemento
  (ordenada por `ts_begin` ASC; la última entrada es la actual); `Ts =
  u64` como CONTADOR lógico monótono (NO mide tiempo real); lecturas
  por snapshot SIN bloqueos (`&self`, clonan — la cadena es el
  cerrojo); `version_visible_` por recorrido inverso (máximo
  `ts_begin ≤ ts` con `ts_end > ts` o `None`); commit con UN SOLO
  timestamp por lote (asigna, retira, appendiza, aplica al `inner`);
  `validar_mvcc` PROPIA con semántica de SOBREESCRITURA; `gc(hasta)`
  que purga versiones retiradas con `ts_end < hasta`; `NivelAislamiento`
  como vocabulario (Instantanea PROHÍBE lectura sucia y lost update,
  DEJA PASAR write skew); `GrafoEspera` con aristas `(TxIdLocal,
  TxIdLocal, Recurso)` y `detectar_ciclo` por DFS 3 colores en O(V+E),
  aunque HOY no pueda haber ciclos; `informe_acid_post_mvcc()` que
  re-valora el aislamiento.
- `practice`: `Operacion` del cap. 27 (los cuatro casos del commit);
  `ResumenCommitMvcc` (análogo a `ResumenCommitWal`, con `ts_asignado`
  y `versiones_retiradas`); la hexagonal del cap. 8 (el `MvccStore::
  inner: MemoryStore` es CONCRETO hoy); la distinción cap. 26 «OLTP
  vs analítica» — el cap. 30 ES la maquinaria que da MATERIALIDAD a
  la «foto»; el `LSN` del cap. 28 (el `Ts` es el «LSN sin disco»); el
  patrón cap. 27 «validación antes de aplicar».
- `consolidate`: el borrow checker como expresión del aislamiento
  (cap. 27); el log append-only como «memoria de versiones» mínima
  (cap. 10/28); «derivar, no llevar en cabeza» (`ts_end` se deriva);
  validación ruidosa; `Display` como contrato para debugging; «UNA
  idea por error tipado» (`MvccError::{Validacion, Store}`).
- `out_of_scope` (sólo nombrar): timestamp físico, vector clocks, true
  time (Parte VIII); concurrencia real con cerrojos; Serializable SI
  con predicate locks (Cahill et al. 2008); MVCC sobre backend
  paginado (caps. 36-37); GC automática en background.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica las TRES reglas de MVCC (cadena por
  elemento, `Ts` lógico monótono, `&mut` para el único escritor +
  `&self` para N lectores); (2) calcula a mano la versión visible
  para un `ts` (recorrido inverso, `ts_begin ≤ ts ∧ (ts_end > ts ∨
  ts_end = None)`); (3) enuncia por qué la SOBREESCRITURA distingue
  `validar_mvcc` de `validar_buffer` (la MVCC crea nueva versión); (4)
  dice qué anomalías PROHÍBE Instantanea (lectura sucia, lost update)
  y CUÁL DEJA PASAR (write skew); (5) explica por qué `GrafoEspera`
  existe HOY sin uso (anzuelo — `&mut self` impide ciclos).
- **Skills**: (1) ejecutar `MvccStore::commit([...])` y predecir
  `ts_asignado`, `nodos_escritos`, `aristas_escritas`, `versiones_retiradas`
  antes del resumen; (2) demostrar el test-tesis de DOS lectores
  concurrentes (snap-a, commit, snap-b — ambas observaciones
  coinciden) y leer `iter_nodos(ts)` antes y después de un commit;
  (3) aplicar `gc(hasta)` y predecir cuántas versiones se retiran y si
  alguna cadena queda vacía.
- **Wisdom**: (1) decide cuándo INSTANTANEA es suficiente y cuándo
  NO (sistemas con lecturas y reescrituras DISJUNTAS — p.ej. «mover
  fondos de A a B y de C a D» — pueden perder write skew aunque cada
  tx «vea» un snapshot correcto; ahí la honestidad exige SSI, no más
  versionado); (2) sabe que el `&mut self` actual es una BENIGN
  LIMITATION, no un bug.

## 4. Modelo mental

- **El fotógrafo y el editor de versiones** (dos analogías que juntas
  lo ordenan todo):
  1. **El fotógrafo**: el lector es un fotógrafo con un contador de
     exposición. Cuando dispara, anota un número (`ts`) y durante su
     revelado (transacción) sólo ve lo que estaba en el visor ESE
     instante. Puede disparar N fotos simultáneas — cada una con su
     número — y cada revelado ve SU momento. El escritor hace clic en
     su obturador (commit) y CAMBIA el visor; pero el revelado
     anterior ya está en su cubeta, terminado, y el nuevo visor no lo
     toca. Las fotos viejas que nadie quiere revelar se tiran (`gc`).
  2. **El editor de versiones** (tipo Git): un `PutNode` NO pisa el
     `Node` anterior — RETIRA su versión actual (le pone `ts_end`) y
     APPENDIZA una nueva. La historia está disponible para quien sepa
     qué commit le interesa (`leer_nodo(id, ts)`). Un `DeleteNode`
     RETIRA la versión actual sin appendizar (la AUSENCIA es el nuevo
     estado).
- **Diagrama ASCII** (cadena de versiones por elemento):
  ```
  Nodo 0 «Ana»:
  +-----------+-----------+--------+
  | ts_begin=1| ts_begin=4| ts_begin=7 |
  | ts_end=4  | ts_end=7  | ts_end=None|
  | «Ana»     | «Ana S.»  | «Ana S.» |
  +-----------+-----------+--------+
    ▲ ts=2 ve «Ana»             ▲ ts=8 ve «Ana S.»; ts=5 ve «Ana S.»
  ```
- **Momento ¡ajá!**: «MVCC no necesita varios escritores para servir
  a varios lectores — la CONSISTENCIA del snapshot viene del
  versionado, no de los locks. El `&mut self` del commit y el `&self`
  de la lectura conviven: el escritor no molesta a los lectores porque
  modifica una COPIA (la versión nueva), no la que están leyendo. Y
  los `ts` no son medida de tiempo: son el ORDEN del programa, lo que
  hace los snapshots comparables y la GC segura».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | MVCC como CAPA sobre `GraphStore` (hexagonal) | Invertir la dependencia: el `MvccStore` CONTIENE un `MemoryStore` (`inner`), no hereda. Permite cambiar el backend (FilePager+CSR del cap. 14) sin tocar el versionado | Herencia: re-implementar para cada backend (3× trabajo) | Backend nuevo = reescribir MVCC | Doc `MvccStore` 178-208; §35 MIGRATION lección 1; cap. 8 hexagonal |
| 2 | Cadena por ELEMENTO (`HashMap<NodeId, Vec<VersionNode>>`) | La consulta típica es «dame el nodo X en el instante T» — el recorrido inverso de la cadena del elemento es O(1) en la práctica (append-only, actual al final) | Índice global `(Ts → ElementId)`: ocupa espacio por cada versión de cada elemento y obliga a filtrar | `leer_nodo` O(versiones totales) en vez de O(del elemento) | `MvccStore` 184-189; `version_visible_node` 505-510 |
| 3 | `Ts = u64` (contador lógico), NO `SystemTime` | El `Ts` es el ORDEN del programa: dos commits no pueden coincidir. Un reloj de tiempo real mezcla «orden» con «cuándo pasó» y abre deriva entre máquinas (Parte VIII). Contador: barato, monótono, portable | Timestamp físico: deriva de relojes; NO aporta info que el orden no dé ya | «¿este snapshot es anterior al mío?» pierde respuesta determinista | `siguiente_ts` 238-242; doc 78-80 |
| 4 | Lecturas por VALOR (`clone` de la versión) sobre `&self` | El lector toma su foto y se la lleva — no le importa lo que el escritor haga después. Sin cerrojos de lectura: el borrow checker permite `&self` (N lectores) y `&mut self` (1 escritor) sobre el MISMO `MvccStore` simultáneamente | `RwLock`: la lectura bloquea al escritor, contradice el objetivo | Lectores bloquean al escritor | `leer_nodo` 244-253; test-tesis `varios_snapshots_coexisten_sin_bloquearse` 928-950 |
| 5 | Commit con UN SOLO `ts` por lote | Asignar UN `ts` da una vista atómica: para cualquier `ts`, todos los elementos tocados son visibles en su nueva versión o todos en la vieja. UNO POR OPERACIÓN abre una ventana en la que un lector ve la mitad del commit (lectura sucia) | Un `ts` por operación: snapshot «mezclado» dentro del mismo commit | Lectura sucia dentro de un commit | `commit` 299-379: `let ts = self.siguiente_ts()` antes del bucle |
| 6 | `validar_mvcc` PROPIA (no `validar_buffer` del cap. 27) | `validar_buffer` del 27 asume INSERCIÓN ESTRICTA: rechaza `PutNode` de id existente. En MVCC SOBREESCRIBIR es legal. Reutilizar el validador exigiría cambiar el contrato de `MemoryStore` o mentir. La validación propia separa las dos políticas | Reutilizar `validar_buffer`: 6 tests fallaban durante calibración | PutNode sobre id existente devuelve `DuplicateNode` y no se distingue de «id NO está» | `validar_mvcc` 381-455; MIGRATION §35 «6 tests fallaban con `Validacion(DuplicateNode)`» |
| 7 | `inner` refleja la versión ACTUAL (espejo material) | Las queries no-MVCC necesitan una versión «del momento presente»: las que NO piden snapshot usan el `inner`. MVCC vive ENCIMA; el `inner` es la verdad material para el código que no sabe de versiones | Sin espejo: las queries no-MVCC leen la cadena y eligen la actual — más complejo, doble fuente | `inner` desincronizado | `MvccStore` 192-197; `commit` `delete-then-put` 331-333 |
| 8 | `delete-then-put` en el `inner` al hacer commit | `MemoryStore` (cap. 8) es de INSERCIÓN ESTRICTA. MVCC SOBREESCRIBE legalmente: `delete_node` (silencioso si no existe) + `put_node`. La CADENA ya hizo su trabajo | Cambiar `MemoryStore` a sobreescritura: rompe el contrato del cap. 8 | `put_node` falla con `DuplicateNode` en commit legítimo | `commit` 331-333; doc 328-330 |
| 9 | `gc(hasta)` purga versiones retiradas con `ts_end < hasta` | Invariante: ningún snapshot con `ts ≥ hasta` puede ver versión retirada con `ts_end < hasta` (ts monótonos). Si la actual (`ts_end = None`) tiene `ts_begin < hasta`, NO se quita. Vaciar cadenas cuando TODAS son retiradas libera también la entrada del mapa | GC agresiva: un snapshot antiguo podría quedarse sin su versión visible | `gc` quita la versión que un snapshot vivo necesita | `gc` 465-500, doc 460-464; test `gc_descarta_versiones_retiradas_antiguas` 955-977 |
| 10 | `NivelAislamiento` como vocabulario (no parámetro del commit) | El commit es SIEMPRE en Instantanea — lo que MVCC da por construcción. Los otros niveles son vocabulario (LecturaSucia: lo que MVCC rechaza; Serializable: lo que Instantanea NO cierra — write skew). Codificar como flag abriría una API que el motor no implementa | Nivel como parámetro: API tentadora que la implementación no soporta; mentiría | «commit en lectura sucia» devuelve datos que otra tx podría revertir | `NivelAislamiento` 103-128, doc 99-115; `niveles_prohiben_las_anomalias_esperadas` 994-1011 |
| 11 | `GrafoEspera` construido aunque NO se use | Aunque HOY no hay concurrencia de escritores (`&mut self` lo prohíbe), la estructura es la pieza estándar que un gestor de cerrojos usaría. Construirla aquí es vocabulario y anzuelo: cuando llegue la Parte VIII, el gestor sabe dónde enchufar `agregar_espera`/`quitar_tx`/`detectar_ciclo` sin re-pensar el modelo | Postergar a la Parte VIII: el lector llega a la distribución sin haber visto la pieza; la analogía «deadlock = ciclo» se aprende DOS veces | El cap. 40 abre sin vocabulario | `GrafoEspera` 597-708; tests 1015-1070; §35 MIGRATION decisión 7 |
| 12 | `write skew` ABIERTO y DOCUMENTADO | En Snapshot Isolation, dos tx que leen y modifican elementos DISJUNTOS a partir del mismo snapshot pueden producir un resultado no serializable (p.ej. «Ana y Bea deben estar disponibles las 24h» — cada una actualiza SU guardia a una hora distinta que libera la suya; ambas confirmadas, resultado: nadie cubre las 24h). Cerrarlo exige Serializable SI con predicate locks (Cahill et al. 2008) — fuera del alcance | «Arreglar» el write skew: predicate locks sobre `MemoryStore` rompen la simplicidad de la capa hexagonal | El motor contesta «instantánea» a una pregunta que exige serializable — modo de fallo silencioso y peligroso | `NivelAislamiento::Instantanea` 108-114; `informe_acid_post_mvcc` 761-768; §35 MIGRATION lección 4 |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: la del cap. 27 — un único escritor por construcción
  (`&mut dyn GraphStore`); los «lectores concurrentes» del cap. 27 NO
  existían en realidad, era vocabulario: el borrow checker era el
  cerrojo. Cualquier intento de tener varios lectores bloqueaba al
  escritor (lectura sucia) o viceversa (lost update). Las anomalías
  del cap. 27 (`Anomalia::LecturaSucia`, `Anomalia::ActualizacionPerdida`)
  se definían pero no PODÍAN ocurrir — el capítulo era sobre el
  vocabulario ACID, no sobre la maquinaria de aislamiento.
- **Qué la rompe**: la promesa de la Parte VI era AISLAMIENTO REAL —
  varios lectores leyendo MIENTRAS un escritor escribe, sin lecturas
  sucias, sin actualizaciones perdidas. La del cap. 27 era una promesa
  vacía: el motor era de un solo hilo lógico. Si el lector abre DOS
  hilos Rust y uno llama `put_node` y el otro `get_node`, el segundo
  ESPERA al primero (el borrow checker no admite `&self` y `&mut self`
  simultáneos). Sin MVCC, no hay forma de dar a un lector un estado
  coherente sin bloquear al escritor.
- **Evolución visible**: el `MvccStore` introduce TRES mecanismos que
  el cap. 27 no tenía — (a) cadenas de versiones por elemento, (b) `Ts`
  lógico monótono asignado por el store, (c) lecturas por valor que
  devuelven la versión visible al `ts` pedido. La API de commit es la
  misma (`commit(&[Operacion])`); lo que cambia es que las lecturas
  son `&self` y CLONAN. El test-tesis
  `varios_snapshots_coexisten_sin_bloquearse` (928-950) demuestra la
  diferencia: dos lectores toman la misma foto, un commit ocurre entre
  medias, AMBOS ven lo suyo sin interferencia — lo que el cap. 27 no
  podía hacer.

## 7. Prueba de fuego

- **TEST-TESIS 1** `leer_en_snapshot_anterior_devuelve_la_version_visible`
  (807-826): un nodo se reescribe en un commit posterior; el snapshot
  anterior SIGUE viendo la versión vieja. La diferencia entre MVCC y
  un store que sobrescribe (el cap. 27 sin MVCC pierde la versión).
- **TEST-TESIS 2** `varios_snapshots_coexisten_sin_bloquearse` (928-950):
  lector A lee, commit ocurre, lector B (foto antes del commit) lee —
  ambos ven lo mismo. La promesa del capítulo: N lectores + 1
  escritor sin bloqueos de lectura.
- **TEST-TESIS 3** `niveles_prohiben_las_anomalias_esperadas` (994-1011):
  `NivelAislamiento::prohibe()` dice la verdad — Instantanea prohíbe
  lectura sucia y lost update, DEJA PASAR write skew; Serializable
  prohíbe las tres (SSI con predicate locks cerraría write skew).
- **TEST-TESIS 4** `grafo_espera_detecta_ciclo_de_dos` (1031-1041): el
  `GrafoEspera` detecta el ciclo T1→T2→T1; `quitar_tx` (1056-1062) lo
  rompe. Aunque no se usa en producción HOY, la pieza está testeada.
- **TEST-TESIS 5** `informe_post_mvcc_avanza_el_aislamiento` (1115-1146):
  el `informe_acid_post_mvcc()` documenta que el aislamiento AVANZA
  (lectura sucia y lost update pasan a prohibidas) — pero write skew
  sigue pasando y el closer del aislamiento salta al cap. 40.
- **Síntoma si el lector se salta este capítulo**: su `MvccStore`
  sobrescribirá el `Node` anterior (no habrá cadena); un commit con
  `ts=2` no podrá distinguir «versión reescrita en `ts=2`» de
  «versión inicial que sigue vigente»; «snapshot» será un eufemismo
  para «el estado en RAM ahora mismo». Y — lo más importante — NO
  entenderá por qué write skew es un problema HONESTO: creerá que
  MVCC «lo arregla todo» y diseñará transacciones disjuntas confiando
  en la garantía equivocada.

## 8. Trampas y errores comunes

1. **Confundir `mv.reloj()` con un `ts` válido**: `reloj()` es el
   SIGUIENTE timestamp a asignar — NO una snapshot válida (todavía no
   hay versión visible para ese `ts`). Lección §35 MIGRATION: usar
   `resumen.ts_asignado` del commit inicial o de un commit anterior.
   Síntoma: `leer_nodo(2, mv.reloj())` devuelve `None` cuando debería
   devolver la versión actual.
2. **Asumir que `validar_buffer` del cap. 27 sirve para MVCC**: la del
   27 rechaza `PutNode` de id existente; la MVCC SOBREESCRIBE.
   Síntoma: 6 tests fallaban con `Validacion(DuplicateNode)` durante la
   calibración. El fix fue `validar_mvcc` PROPIA.
3. **Creer que la versión «más reciente» es siempre la actual**: la
   versión visible para un `ts` se calcula por `ts_begin ≤ ts ∧
   (ts_end > ts ∨ ts_end = None)`, NO por «el último siempre». Olvidar
   el caso de la versión retirada devuelve `None` cuando había versión.
4. **Olvidar que `gc(hasta)` puede vaciar cadenas**: si TODAS las
   versiones quedan retiradas y `ts_end < hasta`, la entrada del mapa
   se ELIMINA. Síntoma: el test `gc_elimina_cadenas_vacias_cuando_el_
   elemento_se_borra` falla si se asume que la cadena sobrevive.
5. **Pensar que `write skew` es un bug**: NO — es la frontera HONESTA
   de Snapshot Isolation. Cerrarlo exige Serializable SI con predicate
   locks (Cahill et al. 2008). El cap. 30 lo DEJA ABIERTO a propósito.
   Síntoma: diseñar transacciones disjuntas creyendo que MVCC las
   serializa — produce un resultado no serializable.
- **Precisión de lenguaje**: *snapshot* (foto lógica con `Ts`) vs *foto
  física* (copia de los datos); *versión* (entrada `{ts_begin,
  ts_end?, valor}`) vs *elemento* (la entidad lógica); *versión actual*
  (la última con `ts_end = None`) vs *versión visible* (la que cumple
  la condición para un `ts`); *retirar* (poner `ts_end`) vs *borrar*
  (eliminar del mapa); *`Ts`* (contador lógico) vs *`LSN`* (posición
  WAL — ambos «órdenes», planos distintos); *write skew* (anomalía de
  SI: dos tx disjuntas leen y reescriben a partir del mismo snapshot)
  vs *lost update* (anomalía clásica: dos tx escriben el mismo
  elemento y una pisa a la otra — Instantanea la prohíbe).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial)**: predice, ANTES de ejecutar, cuántas
  versiones retira el siguiente commit y cuál es el `ts_asignado`.
  Parte del `store_basico()` (3 nodos, 2 aristas, `ts=1`); ejecuta un
  segundo commit con `[PutNode(Nodo::new(0, "Renacido"))]` y comprueba
  el `ResumenCommitMvcc` (nodos_escritos=1, versiones_retiradas=1,
  ts_asignado=2); luego un tercero con `[DeleteNode(0)]` y comprueba
  (nodos_escritos=0, versiones_retiradas=1, ts_asignado=3 — un delete
  RETIRA sin appendizar). Pistas: (1) ¿qué hace `DeleteNode` con la
  cadena?; (2) ¿`reloj()` es lo mismo que `ts_asignado`?; (3) tras el
  `DeleteNode`, ¿cuántas entradas tiene la cadena del nodo 0?
  Criterio: predicción exacta de los tres `ResumenCommitMvcc` y de la
  longitud de la cadena (1, 2, 1 respectivamente).
- **analizar (intermedio — spacing caps. 27 + 29)**: tomando la cadena
  del nodo 0 tras los TRES commits anteriores (`{ts_begin=1, ts_end=3,
  "Ana"}` y vacío tras el delete), explica por qué `leer_nodo(0, 1)`
  devuelve la versión inicial, por qué `leer_nodo(0, 2)` también, y
  por qué `leer_nodo(0, 3)` devuelve `None`; conecta con
  `Anomalia::ActualizacionPerdida` del cap. 27 (¿qué habría pasado
  SIN MVCC si dos tx leen `leer_nodo(0, 2)` y reescriben?); conecta
  con la `RecoveryError` del cap. 29 (¿qué ganaría la MVCC sobre el
  WAL si el sistema cae a mitad del commit?). Pistas: (1) ¿cuál es la
  versión con `ts_begin ≤ ts ∧ ts_end > ts` para cada `ts`?; (2) ¿qué
  condición del cap. 27 PROHIBIRÍA la actualización perdida?; (3)
  ¿`delete-then-put` es atómico ante un crash? Criterio: tres
  predicciones de `leer_nodo` correctas + una frase que conecte
  `Anomalia::ActualizacionPerdida` con la condición de visibilidad +
  una frase que conecte `delete-then-put` con la fragilidad ante
  crash (cap. 29 lo cubre; cap. 30 NO).
- **crear (experto — `gc` y razonamiento inverso)**: implementa
  `gc(hasta)` sobre un `MvccStore` con esta historia: `ts=1` escribe
  nodo 0 («Ana»), `ts=2` reescribe («Ana S.»), `ts=3` reescribe
  («Ana Sofía»), `ts=4` borra; predice cuántas versiones se quitan
  con `gc(4)` y `gc(5)` y demuestra con `cargo test` que coincide con
  `gc_descarta_versiones_retiradas_antiguas`. LUEGO razona al revés:
  dado un `MvccStore` con N reescrituras del mismo nodo y SIN deletes,
  ¿cuál es el MÍNIMO `gc(hasta)` que vacía todas las versiones
  retiradas? Pistas: (1) tras `ts=4` (delete), ¿cuál es el `ts_end`
  de la versión inicial?; (2) ¿la cadena queda con longitud 1 o 0
  tras el delete?; (3) ¿`gc(hasta)` quita la versión con `ts_end =
  None`? Criterio: predicciones exactas + identificación de que la
  versión con `ts_end = None` (la «actual») NO se quita NUNCA por
  `gc` (los snapshots futuros la necesitan) + reconocimiento de que
  el `gc` mínimo para vaciar todo es `mv.reloj()` (el siguiente `ts`
  a asignar — más allá, ningún snapshot puede estar vivo).

## 10. Preguntas abiertas (gancho al cap. 31 — Parte VII; y al cap. 40 — Parte VIII)

1. ¿Cómo expone el `MvccStore` sus lecturas a un REPL? Si el usuario
   teclea `MATCH (n:Person) RETURN n` en el CLI del cap. 31, ¿qué `ts`
   toma para el snapshot? (Nace la Parte VII: el REPL del cap. 31
   necesita una política — ¿`mv.reloj() - 1` por comando? ¿un `ts`
   fijo por sesión? ¿aislamiento por transacción?).
2. ¿Cómo se integra el `Ts` con el WAL del cap. 28? Hoy conviven (el
   commit MVCC NO escribe WAL; la durabilidad es independiente). Si el
   sistema cae a mitad de un `delete-then-put` del `inner`, ¿el
   recovery del cap. 29 sabe qué hacer? (Frontera entre MVCC y WAL
   en caps. futuros).
3. ¿Y si DOS escritores confirman a la vez? Hoy `&mut self` lo prohíbe
   — un único escritor lógico. Cuando lleguen los cerrojos de la
   Parte VIII (cap. 40 — distribución), el `GrafoEspera` aquí
   construido se enchufa, pero write skew se vuelve REAL: dos nodos
   de la red podrían ver el MISMO snapshot y reescribir DISJUNTOS —
   el cierre exige Serializable SI con predicate locks (Cahill et al.
   2008, «Serializable Snapshot Isolation», SIGMOD 2008).
- **Términos nuevos de glosario**: MVCC (Multi-Version Concurrency
  Control), versión, cadena de versiones, timestamp lógico (`Ts`),
  snapshot, espejo material, sobreescritura vs inserción estricta,
  validación previa, garbage collection de versiones, niveles de
  aislamiento, write skew, predicado de visibilidad, grafo de espera
  (wait-for graph), deadlock detection (DFS 3 colores), SSI
  (Serializable Snapshot Isolation), predicate lock.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el experto razona al revés sobre `gc`
  (cuál es el `hasta` mínimo que vacía todo — exige recuperar la
  invariante de la versión con `ts_end = None` y la monotonicidad);
  el esencial predice el `ResumenCommitMvcc` de tres commits
  consecutivos sin ejecutar.
- **Spacing**: cap. 8 (hexagonal — `inner: MemoryStore` lo CONSUME);
  cap. 22 (política ruidosa); cap. 26 (la «foto inmutable» del 26 ES
  el `leer_nodos(ts)` del 30, ahora con maquinaria); cap. 27
  (`Operacion`, `Anomalia::LecturaSucia/ActualizacionPerdida` — el
  vocabulario que el 30 PROHÍBE); cap. 28 (el `Ts` es el «LSN
  lógico»); cap. 29 (el `delete-then-put` del `inner` es frágil ante
  crash — el ARIES lo cubre si MVCC se enchufa al WAL).
- **Interleaving**: el esencial mezcla tres commits consecutivos
  (recordar la diferencia entre PutNode y DeleteNode); el intermedio
  mezcla cap. 27 (anomalías) con cap. 29 (recuperación) sobre la
  misma cadena de versiones; el experto mezcla `gc` con razonamiento
  inverso.
- **Dificultad asimétrica**: una idea nueva por sección (cadena →
  `Ts` lógico → lecturas por valor → commit con un solo `ts` →
  validación propia → `gc` → niveles de aislamiento → `GrafoEspera` →
  `informe_acid_post_mvcc`); los ejercicios exigen predicción,
  razonamiento inverso y conexión entre caps.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb cap30`
  (21 tests citados por nombre — los del lector compilan contra el
  mismo módulo). El test-tesis `varios_snapshots_coexisten_sin_bloquearse`
  es la demostración central.
- **Citas**: Reed, «Naming and Synchronization in a Decentralized
  Computer System», MIT 1978 (la primera propuesta formal de
  versiones múltiples — tesis doctoral, cap. 4); Cahill, Fekete,
  Liarokapis, Bernstein, «Serializable Snapshot Isolation in
  PostgreSQL», VLDB 2008 (el algoritmo que cierra el write skew que
  el cap. 30 DEJA ABIERTO — DOI 10.14778/1454159.1454166);
  PostgreSQL Global Development Group, «13.2. Transaction Isolation»,
  PostgreSQL docs §13.2 (las definiciones operativas de los niveles
  — el vocabulario del que `NivelAislamiento` es una traducción);
  Wu et al., «An Empirical Evaluation of In-Memory Multi-Version
  Concurrency Control», VLDB 2017 (MVCC es la elección de la mayoría
  de motores analíticos en RAM); Ports & Grittner, «Serializable
  Snapshot Isolation in FoundationDB», 2016 (implementación
  industrial de SSI — puente al cap. 40).

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa
      descartada y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: lector A lee, commit ocurre, lector
      B lee — ambos ven lo mismo (test-tesis); `mv.reloj()` malinterpretado
      como `ts` válido; `validar_buffer` rechazando un PutNode legítimo.
- [x] Código ejecutable en workspace (21 tests ALL_GREEN) citado por
      nombre y línea.
- [x] Misconcepciones corregidas explícitamente (§1: seis; «snapshot
      ≠ copia física», «MVCC no exige varios escritores», «write skew
      sigue pasando», «grafo de espera existe sin uso», «Ts ≠ tiempo»,
      «la monotonicidad es global, no por cadena»).
- [x] Ejercicios con solución verificable (los tres commits predichos
      coinciden con tests del workspace + el `ResumenCommitMvcc` y la
      longitud de la cadena son medibles).
- [x] ≥1 ejercicio de retrieval (predicción del `ResumenCommitMvcc` y
      razonamiento inverso sobre `gc`) y ≥1 de spacing (caps. 27, 28,
      29 tocados).
- [x] Responde la pregunta crítica del CORPUS («MVCC con timestamp por
      tupla; garbage collection de versiones») y entrega las piezas
      del brief.
- [x] Re-valoración ACID honesta (write skew queda ABIERTO, closer =
      40).
- [x] Ganchos al cap. 31 (CLI: cómo expone el `MvccStore` sus
      lecturas) y al cap. 40 (distribución: el `GrafoEspera` se
      enchufa, write skew se vuelve REAL — SSI con predicate locks).
- [x] Citas verificadas: Reed 1978, Cahill et al. 2008, PostgreSQL
      docs §13.2, Wu et al. 2017, Ports & Grittner 2016.