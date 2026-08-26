# CONTRATO DE CAPÍTULO — Vol.III Cap. 43: El tiempo en el grafo: versionado y bitemporalidad

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **TERCER CAPÍTULO DEL VOL.III**
> («Grafos en la era de la IA: modelar, razonar y recuperar»), Parte I «Modelar datos de
> grafos». Audiencia declarada en el blockquote inicial: el lector que terminó el cap. 42 —
> o el perfil datos/IA que lo haya leído en diagonal — y que ya tiene la escalera R1-R7, las
> 10 preguntas, el validador por composición, la migración y la regresión como
> HERRAMIENTAS, no como contenido. COBRA el gancho saliente del cap. 42 (§42.9 y cierre):
> «la ronda 2 contrarresta a la ronda 1 — pero ¿QUÉ valía el 3 de marzo?» — este capítulo
> responde CON DATOS: el 3 de marzo de 2025 valía la **nota 7** (la ronda 1): la ronda 2
> (nota 8) llegó después y la contrarrestó; el grafo del cap-42 decía QUÉ contrarresta a
> qué, y este capítulo añade CUÁNDO — con la frontera del grano declarada: el dataset habla
> en años (como `anio` del cap-41); distinguir el 3 del 10 de marzo exige grano fino
> (Snodgrass: intervalos continuos; aquí el grano lo pone el dato, no el modelo).
> Progresión del hilo conductor KB-Lira: paso-1 (cap. 41) = modelo sano con mini-hub;
> paso-2 (cap. 42) = lote importado, degeneración y refactors con precio; paso-3 (ESTE
> capítulo) = **afiliaciones de personas con valid-time**: caducar sin borrar, consultas
> AS OF con coste en lecturas y bitemporalidad sobre un historial de registro. Código
> ancla VERIFICADO hoy (2026-08-26): `lib.rs` declara 34 módulos (…`cap41_modelado`,
> `cap42_antipatrones`); `kb_lira_paso1()` (cap41_modelado.rs:146) construye 30 nodos/64
> aristas con ids FIJOS a mano — las 6 `MEMBER_OF` Persona→Organizacion son las aristas
> 52-57 (Ana→UniLira, Beto→Neurónica, Carla→UniLira, Dani→Neurónica, Elena→GrafosYa,
> Fabio→GrafosYa) y NO llevan ninguna fecha; las personas del paso-2 (Gaby/Hugo/Iris,
> ids 30-32) NO traen `MEMBER_OF` del lote (cap42_antipatrones.rs:1890) — el hueco que
> este capítulo llena; `la_migracion_completa` (cap42_antipatrones.rs:2397) es PUBLICA y
> los refactors A/B/C no tocaron `MEMBER_OF`/`WORKED_ON` (solo ABOUT/SUB_TEMA_DE,
> REVIEWED_BY→Resena, conferencias y temas-año): el paso-3 puede partir del modelo
> refactorizado (67 nodos, 154 aristas, ids hasta 68) sin rozar el cap-42.
> `pregunta_01…pregunta_10` (cap41_modelado.rs:726-978) reciben `&dyn GraphStore` y NINGUNA
> filtra por props de arista (P4 es LiraQL pura sin WHERE sobre el vínculo: la regresión
> es segura); el validador del cap-41 (:390) y el paso-2 NO validan props de `MEMBER_OF`
> (solo extremos). El WAL REAL del Vol.II (cap28_wal.rs: `WalRecord{Lsn,CuerpoWal}`,
> `WalTransaccion`, `replay_wal`, truncado) es CONSOLIDADO aquí (outline: `wal-vol2`), y el
> MVCC del cap-30 (`VersionNode/VersionEdge{ts_begin,ts_end}`) aporta el vocabulario de
> versionado que este capítulo REUTILIZA por contraste. Estado verificado: **948 tests
> ALL_GREEN** (cap-42); toolchain 1.96.0; runtime dependency-free. Código NUEVO previsto:
> UN módulo `src/cap43_temporalidad.rs` (~800-1200 líneas, std puro) + wiring ADITIVO en
> `lib.rs` (2 líneas) + artefacto regenerable
> `liradb-workspace/datasets/kb-lira/paso-3/{nodes.csv,edges.csv,historico.csv}` — CERO
> deps nuevas, CERO cambios en caps. 7-42, **SIN bench** (espejo de la decisión #12 del
> cap-41: la moneda son lecturas y conjuntos exactos, no µs). Citas VERIFICADAS hoy
> (2026-08-26, venue/fecha exactos): Richard T. Snodgrass, *Developing Time-Oriented
> Database Applications in SQL*, **Morgan Kaufmann, julio 1999**, ISBN 1-55860-436-7
> (página del autor: «July, 1999»; el copyright del PDF dice 2000 — se cita 1999);
> Christian S. Jensen y Richard T. Snodgrass, «Temporal Data Management», **IEEE
> Transactions on Knowledge and Data Engineering 11(1):36-44, enero/febrero 1999** (DOI
> 10.1109/69.755613); Snodgrass (ed.), *The TSQL2 Temporal Query Language*, **Kluwer
> Academic Publishers, 1995** (ISBN 0-7923-9614-6); ISO/IEC **39075:2024** GQL, publicado
> **abril 2024** — tipos de datos temporales (date/datetime/duration) y funciones
> temporales SÍ incluidas; consultas bitemporales AS OF NO incluidas en la Parte 1
> [VERIFICAR el detalle fino contra el texto de la norma antes de citarlo en prosa];
> **Neo4j 3.4 (2018)** introdujo los tipos temporales NATIVOS (DATE, LOCAL/ZONED TIME,
> LOCAL/ZONED DATETIME, DURATION), indexables con range lookups (docs 3.4 y manual
> actual; APOC: «Neo4j 3.4 introduced temporal data types»); Petter Holme y Jari
> Saramäki, «Temporal networks», **Physics Reports 519(3):97-125, octubre 2012** (DOI
> 10.1016/j.physrep.2012.03.001; cita clave: «moving the information of *when* things
> happen from the dynamical system on the network, to the network itself»); Vassilis
> Kostakos, «Temporal graphs», **Physica A 388(6):1007-1023, 2009** (DOI
> 10.1016/j.physa.2008.11.021). JanusGraph/TigerGraph: docs NO verificadas hoy → fuera
> del cuerpo (espejo del cap-42). Gancho saliente: cap. 44 (constraints e índices — el
> AS OF sin índice PAGA la historia; «¿quién garantiza que dos afiliaciones no se
> solapen?») y cap. 45 (ingesta — el transaction-time automático al importar).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la escalera R1-R7 y las 10 preguntas con sus respuestas y
  costes (cap. 41); el validador por composición y la regresión de las 10 preguntas como
  red de seguridad (caps. 41-42); los ids FIJOS del builder (0-29) y del lote (30-58); la
  migración completa `la_migracion_completa` y el informe de reescrituras (cap. 42); el
  trait `GraphStore` (put/get/delete/iter, in/out_edges) y la frontera UTF-8 de la
  gramática mini (caps. 7-8, 17-21, declarada en cap-41: los filtros por cadenas con
  acentos se hacen con API directa, P2/P3/P6); el formato CSV del cap. 32 con round-trip
  byte a byte; `ContandoStore` y la disciplina de contadores (caps. 26, 34); el WAL del
  cap. 28 (LSN, Begin/Operacion/Commit/Rollback, replay, truncar_seguro); MVCC del cap.
  30 (`VersionNode/VersionEdge{ts_begin,ts_end}`, snapshots, gc). Perfil IA/datos: entra
  por el prólogo con estas piezas como prerrequisito declarado (el blockquote lo repite
  como el cap-42).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «la temporalidad es para las fechas de nacimiento o publicación» — no: la
  temporalidad vive en el VÍNCULO: «cuándo fue cierto que X pertenecía a Y» es un atributo
  de la arista, no de los nodos (Snodgrass 1999: valid time como propiedad de los hechos
  modelados).
  (2) «guardar la historia duplica los datos y degrada las consultas del presente» — no:
  caducar sin borrar NO degrada el presente: el barrido de adyacencia es el MISMO; lo que
  se paga es cada arista vencida que sigue en la lista (medible: 1 `get_edge` por arista
  histórica; el cap. 44 cambiará ese precio).
  (3) «bitemporalidad = dos fechas en la misma fila» — no: son DOS EJES ortogonales: el
  valid-time (cuándo fue cierto en el mundo) y el transaction-time (cuándo lo supo el
  sistema) — el caso de Dani lo demuestra: dos respuestas legítimas para la misma
  pregunta según el eje consultado.
  (4) «el 3 de marzo (la nota de la ronda) es una fecha de evento» — no: la pregunta del
  cap-42 era por la VALIDEZ (cuándo fue cierto), que no es ni cuándo ocurrió la reseña
  (evento) ni cuándo se registró (transacción): los tres tiempos se confunden todo el
  tiempo (Jensen & Snodgrass 1999).
  (5) «una fecha en un STRING vale lo mismo que una fecha tipada» — no: un string no se
  compara por rango — la lección P6 del cap-41 (un string no se expande) repetida en el
  eje temporal.
  (6) «el WAL del cap-28 ya guarda la historia» — a medias: el WAL guarda las ESCRITURAS
  (el transaction-time del ESTADO), pero no el histórico de valores corregidos; por eso
  el `HistoricoAfiliaciones` existe — el capítulo demuestra lo que el WAL puede y lo que
  no puede responder.
- **No debe saber todavía**: constraints UNIQUE temporales ni índices sobre `desde_anio`
  (cap. 44 — el validador paso-3 SIEMBRA las reglas que allí serán constraints e índices;
  el AS OF SIN índice es la lección de ESTE capítulo); pipeline de ingesta con
  transaction-time automático (cap. 45 — el histórico se construye a mano en el builder);
  RDF y quads (cap. 46); agent-memory y frescura de un KG de memoria (cap. 53 — aquí
  `graph-temporality` se PRESENTA, allí se consolida como herramienta del agente). El
  paso-3 NO pisa ninguna de esas fronteras.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| **Builder temporal** `kb_lira_paso3() -> MemoryStore`: `kb_lira_paso2_degrado()` + `la_migracion_completa()` (los 3 refactors del cap-42, sin tocarlos) + `aplicar_valid_time(&mut store) -> InformeValidTime`. Valid-time como PROPS de arista: `desde_anio:Int` / `hasta_anio:Int` (intervalo medio abierto `[desde, hasta)`, `hasta_anio` ausente = abierto — convención de Snodgrass) en las 6 `MEMBER_OF` del paso-1 (52-57) + 4 nuevas: 182 Hugo→UniLira (2019-abierta), 183 Iris→GrafosYa (2022-abierta), 184 Gaby→Neurónica (2023-abierta, se afilia RECIÉN), 185 Beto→GrafoLuna (2024-abierta; nueva organización «Instituto GrafoLuna», nodo 69); la arista 53 (Beto→Neurónica) queda con `hasta_anio=2024` (VENCIDA: Beto se muda); gancho cobrado: `REALIZA` 149 (ronda 1 de Fabio) con `hasta_anio=2025` y `CONTRARRESTA` 157 con `desde_anio=2025` (la ronda 2 caduca a la ronda 1). Total: **69 nodos, 158 aristas, 10 `MEMBER_OF`** | `estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas` (nodos/aristas por label; 10 MEMBER_OF; nodo 69 GrafoLuna) y `las_member_of_llevan_validez_desde_y_hasta_en_anios` (las 10 con `desde_anio:Int`; 53 con `hasta_anio=2024`) |
| **La validez como función** `arista_vigente_en(arista: &Edge, anio: i64) -> bool` (regla `desde <= anio < hasta` con ausencia = abierto; arista sin props de validez = vigente, retrocompatibilidad del cap-41) | `arista_vigente_en_cubre_abierta_vencida_y_futura` (unitario, 4 casos: abierta vigente, con `hasta` vigente, con `hasta` vencida, sin props) |
| **Consultas AS OF** `afiliaciones_vigentes_en(store, proyecto, anio) -> (Vec<(String,String)>, CosteLecturas)` y `afiliaciones_actuales(store, proyecto)` (envoltura con `ANIO_ACTUAL: i64 = 2026`, el «ahora» FIJO del dataset — disciplina cap. 34, nada de reloj de verdad). La P4 del cap-41 CON tiempo. `CosteLecturas {in_edges, get_edge, get_node}` (moneda del Vol.III, patrón ledger cap-41/ContandoStore cap-26). Valores pineados sobre Proyecto Kira: presente (2026) = (Ana→UniLira, Beto→GrafoLuna, Dani→Neurónica); AS OF 2023 = (Ana→UniLira, Beto→Neurónica, Dani→Neurónica) — IDÉNTICA a la P4 atemporal; AS OF 2019 = (Ana→UniLira, Beto→Neurónica) — Dani aún no se afilia (desde 2021) | `afiliaciones_actuales_de_kira_responden_beto_en_grafosluna`, `afiliaciones_as_of_2023_responden_beto_en_neuronica`, `afiliaciones_as_of_2019_no_incluyen_a_dani` (conjuntos exactos, orden natural) |
| **Tesis del coste**: el presente y el AS OF cuestan el MISMO barrido (el filtro de validez se aplica sobre datos YA leídos: `in_edges` + 1 `get_edge` por arista candidata — sin índice no hay atajo, y eso ES la lección); lo que paga la historia es cada arista vencida que sigue en la adyacencia. Kira: paso-1 = 13 lecturas (3 MEMBER_OF: 52/53/55); paso-3 = 14 (4: 52/53/55/185 — la historia de Beto cuesta 1 `get_edge` extra en CADA consulta del presente) | `el_presente_y_el_as_of_cuestan_el_mismo_barrido` (14 = 14 lecturas para 2026 y 2023, pineadas) y `cada_arista_vencida_anade_una_lectura_al_barrido` (paso-1 13 vs paso-3 14) |
| **Contraste: borrar vs caducar** (el borrado físico es la «primera solución» del capítulo) | `borrar_en_vez_de_caducar_destruye_el_as_of` (`delete_edge(53)`: el presente idéntico, pero AS OF 2023 responde sin Beto — «borrar es gratis hasta que alguien pregunta por el pasado») |
| **Bitemporalidad mínima** `HistoricoAfiliaciones` + `EntradaHistoria {ts_registro: u64, persona: usize, organizacion: usize, desde_anio: i64}` + `historico_kb_lira_paso3()` (determinista). Caso CONCRETO de Dani: entrada ts=2023 «Dani→Neurónica desde 2019» (lo que se creía); corrección ts=2025 «desde 2021» (lo cierto; la arista 55 lleva `desde_anio=2021`). `afiliacion_segun_registro(historico, persona, anio_valid, ts_registro)`: «¿qué creíamos en 2024?» → desde 2019; «¿qué sabemos hoy (2026)?» → desde 2021 — el ejemplo del outline literal | `historico_afiliaciones_registra_el_caso_de_dani` y `afiliacion_segun_registro_distingue_lo_creido_de_lo_cierto` (las dos respuestas legítimas según el eje transaction-time) |
| **Conexión con el WAL (sección 5 del outline) SIN tocar cap-28**: test que usa `WalTransaccion` REAL del cap-28 (sobre un MemoryStore limpio) para insertar una afiliación, registra la MISMA afiliación en el Historico, simula corte de luz (store limpio) y reconstruye con `replay_wal`; equivalencia declarada: LSN ≡ ts_registro, Commit ≡ entrada, replay ≡ re-lectura de la historia | `el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time` (ambas reconstrucciones convergen al mismo estado; el WAL responde el transaction-time del ESTADO y NO puede responder «¿qué creíamos en 2024?» — eso exige el Historico; la limitación DEMOSTRADA en el mismo test) |
| **Validador paso-3 por composición** `validar_modelo_kb_lira_paso3`: REUTILIZA `validar_modelo_kb_lira_paso2` (cap-42) + reglas nuevas SOLO para `MEMBER_OF`: `desde_anio:Int` requerido; `hasta_anio` opcional `Int` con `hasta_anio >= desde_anio`; `desde_anio <= ANIO_ACTUAL`. Las Resena NO entran al validador (solo prosa + test del gancho) | `validador_paso3_acepta_el_modelo_temporal` y `validador_paso3_rechaza_fixture_sin_validez` (grafo roto A MANO: MEMBER_OF sin `desde_anio`, `hasta_anio < desde_anio`, `desde_anio` futuro → violaciones con ids) |
| **REGRESIÓN OBLIGATORIA (red de seguridad triple)** | `las_10_preguntas_del_paso1_no_cambian_tras_anadir_valid_time` (las 10 del cap-41 sobre el subgrafo paso-1 del store paso-3: IDÉNTICAS — ninguna filtra props de arista; P4 atemporal sigue devolviendo Beto→Neurónica: la diferencia entre «P4 atemporal» y «P4 con tiempo» ES la lección, no una regresión — documentado en el test), `las_respuestas_del_paso2_no_cambian_tras_anadir_valid_time` (valores pineados del contrato cap-42: P1 Ana=4, P3 +2 citadores, P5 tema 24 = 24 jerárquica, P6 Neurónica=5, P8=40), `validador_paso2_acepta_el_modelo_paso3` |
| CSV determinista paso-3 (formato cap. 32, con columnas `desde_anio:INT`/`hasta_anio:INT`) + `historico.csv` (formato propio mínimo `ts_registro,persona,organizacion,desde_anio`, exportador propio + round-trip) + artefacto `datasets/kb-lira/paso-3/` commiteado | `csv_roundtrip_paso3_import_export_byte_a_byte`, `csv_paso3_coincide_con_dataset_commiteado_byte_a_byte`, `csv_historico_roundtrip_byte_a_byte` y `csv_paso1_y_paso2_intactos_tras_paso3` (los ficheros de los pasos 1-2 NO se tocan) |
| Informe reproducible `informe_temporal_reproducible`: tabla año → respuestas → costes (2026/2024/2023/2020/2019), SIN tiempos | `informe_temporal_reproducible_sobre_kb_lira` (pineado byte a byte; la tabla de la prosa) |
| **SIN `[[bench]]` nuevo** (decisión espejo #12 cap-41) | `verify.sh` compila `--all-targets` igual; prosa pega salidas de `cargo test` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**948 + ~22 tests nuevos ≈ 970**); cero cambios en caps. 7-42, goldens intactos, pasos 1-2 byte a byte |

## 3. Las preguntas críticas del capítulo y la respuesta del capítulo

**Preguntas** (propias del capítulo): (1) ¿Cómo se representa en el grafo «cuándo fue
cierto» un vínculo, sin destruir las consultas del presente? (2) ¿Qué significa
«bitemporal» y cuándo una sola fecha NO basta? (3) ¿Cuánto cuesta preguntar por el
pasado, y quién paga la historia?

Respuestas medibles:

1. **Representación**: props `desde_anio`/`hasta_anio` en la arista `MEMBER_OF` — la
   validez es atributo DEL VÍNCULO (la decisión que el cap-41 hubiera tomado con R1-R7:
   la afiliación no tiene relaciones propias ni ciclo de vida más allá de la arista; la
   MISMA escalera que reificó `Resena` en el cap-42 — R2/R3 — aquí NO sube: díptico
   cerrado con dos veredictos). Las consultas del presente (ANIO_ACTUAL=2026 fijo)
   filtran por validez en la lectura de adyacencia; AS OF (año X) filtra por X. El coste
   del presente NO se degrada: el barrido es el mismo (tesis pineada: 14 = 14 lecturas
   para 2026 y 2023) — lo que paga la historia es cada arista vencida que sigue en la
   adyacencia (paso-1: 13; paso-3: 14; con 10M de vencidas, 10M de lecturas — la factura
   que el cap. 44 cobrará con el índice).
2. **Bitemporal**: el caso de Dani — el registro de 2023 decía «desde 2019», la
   corrección de 2025 dice «desde 2021» (el grafo muestra la corregida). «¿Qué creíamos
   en 2024?» y «¿qué sabemos hoy?» son DOS preguntas con DOS respuestas legítimas: la
   primera por el eje transaction-time (el Historico), la segunda por el valid-time (la
   arista). Una sola fecha no basta porque no distingue el mundo del conocimiento
   (Jensen & Snodgrass 1999: la distinción es el fundamento de la bitemporalidad;
   TSQL2 1995: los dos ejes como tipos de tiempo del estándar).
3. **Coste**: moneda = lecturas (`CosteLecturas`), nunca µs. El AS OF no cuesta más que
   el presente (mismo barrido); lo que cuesta es la HISTORIA EN SÍ: cada arista vencida
   que conservamos añade 1 `get_edge` a cada barrido que toca a su persona — y el
   borrado físico, la alternativa barata, destruye la respuesta AS OF (test de
   contraste). El lector sale sabiendo cuándo la historia merece su factura y qué la
   abarata (índice → cap. 44).

Escalera del brief (5 secciones del outline → 5 peldaños):

1. **Tres tiempos: evento, validez y registro** → la tabla de los tres tiempos con la
   reseña de Fabio (evento = cuándo se escribió; validez = cuándo fue la nota vigente;
   registro = cuándo lo supo el sistema — el WAL del cap-28). Fuentes: Snodgrass 1999,
   Jensen & Snodgrass 1999, TSQL2 1995.
2. **Valid-time en aristas: caducar sin borrar** → `desde_anio`/`hasta_anio` en
   MEMBER_OF; el grafo «presente» ignora vencidas; gancho del cap-42 cobrado: la ronda 1
   caduca cuando la ronda 2 la contrarresta (`CONTRARRESTA{desde_anio:2025}`); «¿QUÉ
   valía el 3 de marzo?» → la nota 7 — con la frontera del grano declarada.
3. **Bitemporalidad: lo que creíamos entonces vs lo cierto ahora** → el caso de Dani: el
   grafo (valid-time corregido) vs el Historico (transaction-time con las dos entradas);
   `afiliacion_segun_registro` responde las dos preguntas.
4. **Consultas AS OF y su coste** → `afiliaciones_vigentes_en` con `CosteLecturas`; la
   tesis del barrido (presente = AS OF en lecturas) y la factura de la historia (1
   `get_edge` por vencida); borrar vs caducar con el test de contraste.
5. **Conexión con el WAL y el log de cambios (Vol.II cap. 28)** → el cap-28 construyó el
   transaction-time del motor sin llamarlo así; aquí se le pone nombre, se DEMUESTRA la
   equivalencia (WalTransaccion real + replay en un test) y se declara la frontera: el
   WAL responde el estado final, no el histórico de valores corregidos — eso es el
   Historico.

Hilo conductor: **«el grafo del cap-42 era una FOTO: decía lo que sabemos hoy. Este
capítulo lo convierte en PELÍCULA: el tiempo pasa al grafo (Holme & Saramäki 2012) — y
descubres que "cuándo" tiene dos relojes: el del mundo (valid-time) y el del
conocimiento (transaction-time), y que preguntar por el pasado tiene precio en
lecturas»**.

---

## 4. La arquitectura: la foto, la película y los dos relojes

Modelo mental único: **el grafo como PELÍCULA: cada arista nace, vive y caduca — y la
pregunta «¿qué valía entonces?» tiene dos relojes: el del mundo (valid-time) y el del
conocimiento (transaction-time)**. La figura que ordena todo el capítulo:

```text
LOS TRES TIEMPOS (con la reseña de Fabio, el gancho del cap-42):
  EVENTO    cuándo ocurrió el hecho         la ronda 2 se escribió en 2025
  VALIDEZ   cuándo fue cierto en el mundo   la nota 8 es vigente DESDE 2025
            (lo que las aristas guardan)    (la nota 7 caducó: CONTRARRESTA 2025)
  REGISTRO  cuándo lo supo el sistema       el WAL del cap-28 (transaction-time)
            (lo que el log guarda)

LA LÍNEA DE BETO (valid-time en la arista, intervalo [desde, hasta)):
  2018─────────────────────2024──────────────2026
  [─ Neurónica (53) ─────) [─ GrafoLuna (185) ─)
        hasta_anio=2024         desde_anio=2024, abierta
  «ahora» (2026): Beto→GrafoLuna · AS OF 2023: Beto→Neurónica · AS OF 2024: GrafoLuna
  (convención de medio abierto [desde, hasta) — Snodgrass 1999)

EL CASO DE DANI (bitemporalidad: dos relojes):
  eje VALIDEZ (el mundo):     arista 55: desde_anio=2021 (corregido)
  eje REGISTRO (el saber):    Historico: ts=2023 «desde 2019» ──corrección──► ts=2025 «desde 2021»
  «¿qué creíamos en 2024?» → desde 2019      «¿qué sabemos hoy (2026)?» → desde 2021
  (la MISMA pregunta, DOS respuestas legítimas según el reloj consultado)
```

Y debajo, la REGLA DE ORO heredada del cap. 34 (determinismo total, contadores de
TRABAJO, cero tiempos): el «ahora» es una CONSTANTE (`ANIO_ACTUAL=2026`), cada consulta
devuelve su `CosteLecturas` pineado, y las 10 preguntas del cap-41 + el validador paso-2
del cap-42 son la red de seguridad: si añadir validez cambia una respuesta vieja sobre
los subgrafos 1-2, el cambio está MAL. La frontera, declarada antes de codificar:
`cap43_temporalidad.rs` es aditivo (como caps. 38-42); ni `GraphStore` ni el executor ni
`cap41_modelado.rs` ni `cap42_antipatrones.rs` se tocan; el validador paso-3 REUTILIZA
el paso-2 por composición (el mismo patrón del 42 con el 41).

```text
Lo que SÍ se hace hoy:   builder paso-3 (refactorizado + valid-time), arista_vigente_en,
                         afiliaciones AS OF con CosteLecturas, el contraste borrar-vs-caducar,
                         HistoricoAfiliaciones + el caso de Dani, test de conexión con el WAL
                         real del cap-28, validador paso-3, regresión triple, CSV paso-3 +
                         artefacto commiteado
Lo que AÚN NO:           constraints UNIQUE temporales e índices sobre desde_anio (cap. 44:
                         el validador paso-3 SIEMBRA las reglas) · ingesta con transaction-time
                         automático (cap. 45: el histórico se construye a mano) · RDF/quads
                         (cap. 46) · grano sub-anual (la reseña del 3 de marzo se responde con
                         grano anual: frontera declarada) · durabilidad del Historico (RAM)
```

Momento ¡ajá! perseguido: **«el cap-42 me dijo QUÉ contrarresta a qué; este capítulo me
dice CUÁNDO — y "cuándo" tiene dos respuestas: cuándo fue cierto y cuándo lo supimos. El
grafo del cap-42 no mentía: simplemente no sabía que había pasado»**.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | `kb_lira_paso3()` PARTE del paso-2 REFACTORIZADO: `kb_lira_paso2_degrado()` + `la_migracion_completa()` (pública, cap42_antipatrones.rs:2397) + `aplicar_valid_time()` | El paso-3 construye SOBRE el modelo vigente: los refactors del 42 no tocaron `MEMBER_OF`/`WORKED_ON` (verificado: el lote NO siembra MEMBER_OF, cap42:1890; los refactors solo ABOUT/Resena/conferencias/temas-año), así que la temporalidad se añade sin rozar las reglas del 42 y su regresión se conserva por herencia; el «presente» del cap-43 ES el modelo con el que el lector cerró el 42 | (a) Partir del paso-1: el «ahora» no sería el modelo vigente (sin Resena/conferencias/subtemas) y habría que explicar los refactors; (b) partir del paso-2 DEGENERADO: el presente cargaría con los antipatrones que el 42 ya curó | cap42_antipatrones.rs:2397 (firma pública); OUTLINE-VOL3.yml cap-43 (paso-3); contrato cap-42 §2 (regresión por herencia) |
| 2 | Valid-time como PROPS de arista `desde_anio:Int`/`hasta_anio:Int` en MEMBER_OF (intervalo `[desde, hasta)`, ausencia = abierto) | La validez es atributo DEL VÍNCULO: R1-R7 del cap-41 decide que la afiliación no tiene identidad propia — sin relaciones salientes ni ciclo de vida más allá de la arista (R2/R3 no suben); la arista ya se lee completa (props incluidas) en el barrido, así que la validez viaja GRATIS en la lectura que ya se hace; el díptico con el cap-42 (Resena SÍ se reificó por tener CONTRARRESTA) enseña la MISMA regla con dos veredictos | (a) Nodos `:Afiliacion` reificados: reificación EXCESIVA aquí (R2/R3 no suben; añadiría un salto a la P4 sin comprar nada — el espejo de `Autoria` del cap-41); se NOMBRA el caso límite: si la afiliación ganara sub-relaciones (contrato, ceses), reificar sería la cura; (b) fecha única sin `hasta`: la mudanza de Beto exige el fin; (c) `vigencia:String` («2023-2025»): no se filtra por rango — lección P6 repetida | cap-41 R1-R7 y P6; cap-42 (díptico Autoria/Resena); Snodgrass 1999 (intervalos de valid time); contrato cap-41 decisión #3 (`order` como atributo del vínculo — precedente directo) |
| 3 | Grano ANUAL (`i64` años), no fechas | Coherencia con el dataset: todo el Vol.III habla en años (`anio:Int` en todo Documento, P10); determinismo total (cap. 34); la respuesta al gancho incluye la frontera: «¿qué valía el 3 de marzo?» se responde «la nota 7» y se DECLARA que distinguir el 3 del 10 exige grano fino — el grano lo pone el dato, no el modelo (Snodgrass: intervalos continuos) | Fechas exactas (DÍA): exige un tipo fecha nuevo en `Value` (rompería la serialización del cap. 9 y el CSV del cap-32) o meses ad hoc: churn sin ganancia pedagógica — la lección del grano ES contenido | cap41_modelado.rs (`anio:Int`); Snodgrass 1999 (grano e intervalos); cap-9 (Value sin tipo fecha — frontera declarada) |
| 4 | Tres tiempos formalizados: evento / validez (valid-time) / registro (transaction-time), con el WAL del cap-28 como el registro YA CONSTRUIDO | El capítulo enseña a distinguir los tres ejes antes de modelar (Jensen & Snodgrass 1999: la confusión valid/transaction es el error clásico; TSQL2 1995: los dos ejes como tipos del estándar); nombrar el WAL como transaction-time conecta el Vol.II con el modelo: el lector YA construyó el tercer tiempo sin saberlo | Presentar solo «dos fechas» (desde/hasta) sin el marco: el lector no sabría por qué el caso de Dani necesita un historial aparte | Snodgrass 1999 (cap. 1: los tres tipos de tiempo); Jensen & Snodgrass 1999, TKDE 11(1):36-44; TSQL2 (Kluwer, 1995); cap28_wal.rs (CuerpoWal) |
| 5 | Caso bitemporal CONCRETO: la corrección de la afiliación de Dani (registro 2023 «desde 2019» → corrección 2025 «desde 2021»; el grafo muestra la corregida) | Es el ejemplo del outline literal — «lo que creíamos entonces vs lo cierto ahora»: dos respuestas legítimas a la misma pregunta según el eje; se responde CON DATOS (Historico + `afiliacion_segun_registro`) y demuestra que bitemporal ≠ dos fechas: son dos ejes | (a) Bitemporalidad PLENA en el grafo (dos pares de props en la arista): el grafo solo guarda lo que el motor sabe HOY; versionar por registro exigiría un MVCC de modelo (cap. 30 lo hace por CONCURRENCIA, no por historia) — el Historico separado es la separación de ejes honesta; (b) un caso inventado fuera de KB-Lira: pierde el hilo conductor | OUTLINE cap-43 sección 3; Jensen & Snodgrass 1999 (bitemporalidad como dos ejes); cap30_mvcc.rs (contraste ts_begin/ts_end: versionado de CONCURRENCIA vs de HISTORIA — spacing) |
| 6 | AS OF con coste en LECTURAS (`CosteLecturas`), API directa (patrón P2/P3/P6 del cap-41); tesis: presente y AS OF pagan el MISMO barrido; la historia cuesta 1 `get_edge` por arista vencida conservada | «¿Cuánto cuesta preguntar por el pasado?» exige respuesta reproducible en CI: contadores, no µs (espejo decisión #12 cap-41); el filtro de validez se aplica sobre datos YA leídos (la adyacencia no distingue vigentes de vencidas SIN índice); la API directa permite el ledger exacto y evita la semántica trivalente de NULL sobre `hasta_anio` ausente | (a) LiraQL pura con `WHERE` sobre props de arista: el RETURN `a.order` del cap-41 demuestra que las props de arista viajan en las bindings, pero `hasta_anio >= X` sobre prop AUSENTE cae en la semántica trivalente del cap-20 — frágil; el reto esencial puede intentarlo ([VERIFICAR] si la gramática lo admite); (b) bench µs: la moneda son conjuntos y contadores | cap41_modelado.rs:739 (precedente `a.order`); cap20_volcano.rs (FilterOp trivalente); decisión #12 contrato cap-41; ContandoStore (cap-26) |
| 7 | El «ahora» es CONSTANTE del módulo (`ANIO_ACTUAL: i64 = 2026`), nunca `SystemTime` | Disciplina del cap. 34 (determinismo total: un test con reloj de verdad sería irreproducible); el dataset «vive» en 2026 (hoy 26-ago-2026) y el contrato pineado depende de esa constante | `SystemTime::now()` en las consultas: viola el determinismo; los tests cambiarían cada día | cap34 (dataset determinista); contrato cap-42 (misma política) |
| 8 | `HistoricoAfiliaciones` = Vec de entradas append-only con `ts_registro` monótono (asignado por `registrar`, nunca por el llamador) — el «WAL del modelo» | La conexión con el cap-28 debe ser ESTRUCTURAL: ts monótono como LSN, append-only como el log, re-lectura en orden como el replay; el test de conexión usa `WalTransaccion` REAL (API pública, cero cambios en cap-28) y demuestra qué puede y qué no puede responder el WAL | (a) Re-implementar un WAL con bytes/CRC: el cap-28 YA lo hizo y `wal-vol2` es CONSOLIDAR, no re-enseñar (outline) — duplicarlo violaría la regla; (b) integrar el Historico en el WAL real: exigiría tocar cap-28 — prohibido; la durabilidad del Historico queda como frontera (cap. 37/45) | OUTLINE cap-43 sección 5 (consolida wal-vol2); cap28_wal.rs (Lsn/TxId/CuerpoWal — API pública reutilizable); contrato cap-42 (caps. 7-41 intactos, extendido a 7-42) |
| 9 | El validador paso-3 REUTILIZA el paso-2 (composición, sin tocarlo) + reglas nuevas SOLO para MEMBER_OF (desde_anio requerido, hasta ≥ desde, desde ≤ ANIO_ACTUAL); las Resena NO entran al validador | El contrato del modelo evoluciona por composición (patrón ya probado en el 42 con el 41); el alcance del outline es «afiliaciones con valid-time»: las Resena solo cobran el gancho (prosa + test propio); las reglas temporales SIEMBRAN los constraints del cap. 44 | Reescribir el validador in situ: rompe la regla dura; añadir reglas de Resena al validador: fuera del alcance del paso-3 y duplica contenido del 42 | cap42_antipatrones.rs:1505 (firma pública del paso-2); OUTLINE cap-44 (constraints); contrato cap-42 decisión #8 |
| 10 | REGRESIÓN TRIPLE: las 10 preguntas del cap-41 sobre el subgrafo paso-1, las respuestas pineadas del cap-42 sobre el paso-2, y `validador_paso2_acepta_el_modelo_paso3` | Añadir props de validez NO debe cambiar ninguna respuesta vieja: las preguntas no filtran por props de arista y los validadores no las exigen (verificado en el código); el caso P4 ES la lección: la P4 atemporal sigue devolviendo Beto→Neurónica (su contrato no cambia) mientras la P4 CON tiempo devuelve Beto→GrafoLuna — la diferencia ES el capítulo, no una regresión; se documenta en el test para que el generador no «corrija» nada | (a) Congelar las preguntas sin re-ejecutarlas: perder la red de seguridad; (b) «actualizar» la P4 para que devuelva la afiliación vigente: pisar el contrato atemporal del cap-41 — la pregunta vieja se re-ejecuta TAL CUAL | cap41_modelado.rs:726-978 (firmas `&dyn GraphStore`); contrato cap-42 §2 (valores pineados: P5=24, P8=40); contrato cap-41 §3 |
| 11 | Alcance de código: UN módulo `cap43_temporalidad.rs` (~800-1200 líneas, std puro, ~22 tests) + wiring aditivo (2 líneas) + artefacto `datasets/kb-lira/paso-3/{nodes.csv,edges.csv,historico.csv}`; **SIN bench** | La casa enseña con código testeado; el módulo accede GRATIS a los builders 41-42, a `la_migracion_completa`, al WAL del cap-28 y al CSV del cap-32; el paso-3 es un artefacto DERIVADO del builder (como los pasos 1-2); el historico.csv usa un exportador propio mínimo (no es un GraphStore) con round-trip propio | (a) Crate nueva: churn sin ganancia (precedente caps. 38-42); (b) solo prosa/diagramas: traiciona el estándar; (c) primer bench del Vol.III: cronometrar no sostiene ninguna tesis de este capítulo | CONVENTIONS §2 y §4; contrato cap-42 decisión #11; precedente caps. 38-42 |
| 12 | Citas nuevas SOLO las verificadas hoy (2026-08-26): Snodgrass 1999 (Morgan Kaufmann, julio 1999), Jensen & Snodgrass TKDE 11(1):36-44 (ene/feb 1999), TSQL2 (Kluwer, 1995), GQL ISO/IEC 39075:2024 (abril 2024; tipos temporales SÍ, AS OF en Parte 1 [VERIFICAR]), Neo4j 3.4 (2018; tipos temporales nativos), Holme & Saramäki Physics Reports 519(3):97-125 (oct 2012), Kostakos Physica A 388(6):1007-1023 (2009); JanusGraph/TigerGraph FUERA | La bibliografía temporal clásica tiene fuentes precisas con DOI/ISBN; Holme & Saramäki entra porque «mover el cuándo del sistema al grafo» es EL modelo mental del capítulo; JanusGraph/TigerGraph no se pudieron verificar hoy → fuera (espejo del cap-42); el detalle fino de la exclusión de AS OF en GQL se marca [VERIFICAR] antes de citarlo en prosa | Citar JanusGraph/TigerGraph sin verificar: riesgo de URL o versión inventada; citar SQL:2011 (periods/temporal tables) sin verificación hoy: solo como contexto industrial con marca [VERIFICAR] | Verificación puntual realizada hoy con fuentes primarias en vivo; contrato cap-42 (misma política de citas) |

## 6. Estructura del manuscrito (partes y tempos)

1. **Blockquote inicial OBLIGATORIO**: TERCER capítulo del Vol.III, audiencia (lector
   del cap-42 + perfil IA/datos), conexión con caps. 41-42 por referencias (las 10
   preguntas, los validadores por composición, la migración, la regresión — sin
   re-explicar nada), gancho cobrado literalmente: «¿QUÉ valía el 3 de marzo?» → «la
   nota 7: la ronda 1 — el 3 de marzo de 2025 la ronda 2 aún no había contrarrestado
   nada; el grafo del cap-42 no podía decírtelo porque no guardaba CUÁNDO».
2. **Apertura (N.0, anécdota + pregunta crítica)**: la revisión de Holme & Saramäki
   (2012) — los pacientes de un hospital comparten sala durante PERIODOS: las aristas
   no están siempre activas, y «mover la información de cuándo pasa algo, del sistema
   dinámico al propio grafo» cambia todo (los caminos respetan el tiempo; la
   transitividad se rompe). Pregunta enmarcada: tu KB-Lira dice que Beto pertenece a
   Neurónica — pero se fue en 2024. ¿Tu grafo te está mintiendo por omisión?
3. **N.1-N.2 Objetivo/Problema**: objetivo medible del outline («representar historia y
   validez temporal sin destruir el rendimiento de las consultas del presente»).
   Problema: el cap-42 cerró declarando la frontera («la ronda es propiedad simple, NO
   valid-time») y el grafo atemporal miente por omisión: la P4 responde afiliaciones
   que ya no existen. Desactivar las seis misconcepciones ANTES de dibujar.
4. **N.3 Modelo mental**: la foto vs la película; la tabla de los tres tiempos (evento/
   validez/registro) con la reseña de Fabio; la línea de Beto con el intervalo
   `[desde, hasta)`; el doble reloj del caso de Dani. Holme & Saramäki como epígrafe.
5. **N.4 Primera solución**: la NO-solución doble — (a) BORRAR la arista vencida
   (`delete_edge(53)`): el presente queda limpio… y el AS OF 2023 no tiene respuesta;
   (b) apuntar la fecha en un STRING «2023-2025»: un string no se filtra por rango (la
   lección P6 repetida). El capítulo muestra ambas con sus modos de fallo ANTES de la
   solución.
6. **N.5 Sus límites**: borrar destruye la historia («¿a quién pertenecía el proyecto
   en 2023?» — sin respuesta); el string no se compara; el «ahora» de verdad cambia
   cada día (determinismo); y la pregunta del cap-42 sigue sin responderse.
7. **N.6 Solución evolucionada**: `desde_anio`/`hasta_anio` en las aristas + `arista_
   vigente_en` + `afiliaciones_vigentes_en` con `CosteLecturas` + el contraste
   borrar-vs-caducar con números + el Historico con el caso de Dani + la conexión con
   el WAL del cap-28 + el validador paso-3 por composición + la REGLA DE ORO (las
   respuestas viejas no cambian).
8. **N.7 Código completo ejecutable**: `cap43_temporalidad.rs` por `include::` (nunca
   duplicado); SIN bench (decisión #11 explicada en una línea).
9. **N.8 Prueba de fuego**: salidas REALES de `cargo test`: la tabla AS OF (ahora/2023/
   2019/2024 → respuestas → costes), el caso de Dani (las dos respuestas), el test de
   conexión con el WAL, el contraste borrar-vs-caducar, la regresión triple, el CSV
   paso-3 byte a byte.
10. **N.9 Qué hemos sacrificado**: sin índices sobre `desde_anio` ni constraints UNIQUE
    temporales (cap. 44 — el validador paso-3 es su semilla; el AS OF SIN índice es la
    lección); sin ingesta con transaction-time automático (cap. 45 — el histórico se
    construyó a mano); sin grano sub-anual (la reseña del 3 de marzo se responde con
    grano anual: frontera declarada con Snodgrass); sin durabilidad del Historico (RAM);
    sin RDF/quads (cap. 46); la bitemporalidad es MÍNIMA (el grafo guarda el valid-time
    actual; el Historico guarda el registro).
11. **N.10 Cómo lo hace una BBDD real + retos**: Neo4j — tipos temporales NATIVOS desde
    la 3.4 (2018), indexables con range lookups, pero SIN bitemporalidad de consulta
    (el patrón industrial es el mismo: props de validez en las aristas); GQL (ISO/IEC
    39075:2024, abril 2024) — tipos de datos temporales y funciones temporales
    estandarizados; las consultas AS OF no están en la Parte 1 [VERIFICAR]; la tradición
    SQL temporal: TSQL2 (1995) sentó los dos ejes que el estándar SQL lleva incorporando
    desde 2011 [VERIFICAR el detalle de SQL:2011]; Snodgrass (1999) y Jensen & Snodgrass
    (1999) como las referencias canónicas. Retos: esencial (PREDECIR por escrito
    `afiliaciones_vigentes_en` para AS OF 2020 y AS OF 2024 ANTES de correr, y cuántas
    lecturas pagará cada una), intermedio (comparar el versionado del cap-30 —
    `ts_begin/ts_end` para CONCURRENCIA — con el valid-time: qué es lo mismo y qué es
    distinto; y aplicar el patrón a `WORKED_ON`: ¿el proyecto Brújula «nació» en 2022?),
    experto (usar `WalTransaccion` real del cap-28 para registrar la corrección de Dani
    y demostrar qué puede y qué NO puede responder el WAL del cap-28 sobre «¿qué
    creíamos en 2024?» — la frontera que motiva el Historico).
12. **Baterías finales + gancho**: Lo que te llevas / Ojo cuidado / Pin / 30 segundos /
    historia pequeña (la reseña del 3 de marzo como anécdota de cierre) / Mini-diálogo
    de guardia nocturna (la consulta «¿quién pertenecía a Neurónica en 2023?» contra un
    grafo que borró la historia). Retrieval practice: recitar DE MEMORIA los tres
    tiempos con un ejemplo propio y clasificar 5 afirmaciones (evento/validez/registro)
    sin mirar. Spacing: MVCC cap-30 (el contraste), WAL cap-28 (la conexión), validador
    y las 10 preguntas (caps. 41-42), CSV cap-32, ContandoStore cap-26. Interleaving:
    cada reto toca ≥2 capítulos (43+41, 43+30, 43+28). Glosario nuevo: valid-time,
    transaction-time, tiempo de evento, bitemporalidad, AS OF, grano temporal, caducar
    sin borrar, arista vencida, intervalo de validez (medio abierto `[desde, hasta)`),
    historial de registro, «ahora» fijo del dataset. Gancho al cap. 44: «AS OF paga 1
    lectura por cada arista vencida de la adyacencia — ¿quién construye el índice que
    lo abarata? ¿y quién garantiza que dos afiliaciones no se solapen en el tiempo?».
    Abiertas: ingesta con transaction-time (45), el KG temporal como memoria del agente
    (53).

---

## 7. Estilo y tono (consistencia con el proyecto)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (valid time, transaction time, event time, bitemporal,
  AS OF, granularity, expired edge); salidas REALES de `cargo test` pegadas, nunca
  reconstruidas; las decisiones temporales se presentan como TRADE-OFF con precio en
  lecturas, nunca como dogma.
- **Diagramas**: la tabla de los tres tiempos con la reseña (§4); la línea de Beto con
  el intervalo `[desde, hasta)`; el doble reloj del caso de Dani; la tabla AS OF
  (año → respuestas → costes) como figura recurrente de las baterías.
- **Spacing** (conceptos viejos que se EJERCITAN): la escalera R1-R7 y las 10 preguntas
  (cap. 41), los refactors y la regresión (cap. 42), `GraphStore` y travesías (caps.
  7-8, 20), WAL (cap. 28), MVCC (cap. 30 — el contraste versionado-concurrencia vs
  versionado-historia), CSV (cap. 32), dataset determinista (cap. 34), ContandoStore
  (cap. 26), la frontera UTF-8 de la gramática mini (cap. 41).
- **Interleaving**: el reto esencial mezcla 34+41+43 (predicción sobre dataset
  determinista con años fijos); el intermedio mezcla 30+43 (dos versionados frente a
  frente) y 41+43 (aplicar el patrón a WORKED_ON); el experto mezcla 28+43 (el WAL real
  contra la pregunta bitemporal).
- **Dificultad asimétrica**: una idea nueva por sección (los tres tiempos → valid-time
  en aristas → bitemporalidad → AS OF y coste → WAL); los ejercicios exigen PREDECIR
  resultados y costes ANTES de correr los tests.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap43` (lecturas y conjuntos
  exactos) y `./scripts/verify.sh` ALL_GREEN como puerta. Nunca «confía en mí».
- **Anécdota (única, verificada)**: Holme & Saramäki (Physics Reports, 2012) — las
  aristas que se encienden y se apagan (los pacientes que comparten sala; el correo
  instantáneo) y la frase que ordena el capítulo: mover el «cuándo» del sistema al
  grafo. Fuentes para la prosa: Snodgrass (Morgan Kaufmann, 1999); Jensen & Snodgrass
  (IEEE TKDE, 1999); TSQL2 (Kluwer, 1995); ISO/IEC 39075:2024 GQL; Neo4j 3.4 docs
  (2018); Holme & Saramäki (2012); Kostakos (Physica A, 2009); SQL:2011 [VERIFICAR].

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap43_temporalidad; pub
  use cap43_temporalidad::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al
  final, con el módulo ya compilando limpio (`cargo check -p vol2-liradb`); jamás dejar
  `lib.rs` apuntando a un módulo rojo. Los ids nuevos (nodo 69; aristas 182-185) deben
  validarse contra los ids del paso-2 refactorizado (nodos hasta 68, aristas hasta 181):
  NO reutilizar un id existente — el test de estructura lo cazaría. `kb_lira_paso3()`
  NO debe tocar `cap41_modelado.rs` ni `cap42_antipatrones.rs`: llama a sus funciones
  públicas (`kb_lira_paso2_degrado`, `la_migracion_completa`) y añade encima.
- **Orden de implementación recomendado (PATRÓN DE TROCEO — cada pieza compila y
  testea SOLA: 1 función + 1 test, el agente de código no sobrevive a tareas largas)**:
  (1) `kb_lira_paso3()` (paso2_degrado + la_migracion_completa + aplicar_valid_time con
  las 10 MEMBER_OF + reseñas) + `estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas`
  + `las_member_of_llevan_validez_desde_y_hasta_en_anios`; (2) `arista_vigente_en` +
  `arista_vigente_en_cubre_abierta_vencida_y_futura`; (3) `afiliaciones_vigentes_en` +
  `afiliaciones_actuales` + la tabla pineada (actuales/2023/2019 — tres tests); (4)
  `CosteLecturas` + `el_presente_y_el_as_of_cuestan_el_mismo_barrido` +
  `cada_arista_vencida_anade_una_lectura_al_barrido` + `borrar_en_vez_de_caducar_
  destruye_el_as_of`; (5) el gancho de la reseña: `la_ronda_1_de_fabio_caduco_cuando_la_
  ronda_2_la_contrarresto` (las props de REALIZA 149/CONTRARRESTA 157 ya las puso la
  pieza 1); (6) `HistoricoAfiliaciones` + `historico_kb_lira_paso3` + el caso de Dani
  (dos tests); (7) test de conexión con el WAL (`el_historico_es_el_wal_del_modelo_y_el_
  wal_del_cap28_es_transaction_time` — ANTES verificar las firmas públicas de
  `WalTransaccion`/`replay_wal` del cap-28 tal cual, sin tocarlas); (8)
  `validar_modelo_kb_lira_paso3` + `validador_paso3_acepta_el_modelo_temporal` +
  `validador_paso3_rechaza_fixture_sin_validez`; (9) regresión triple (las 10 del
  paso-1, las respuestas del paso-2, `validador_paso2_acepta_el_modelo_paso3` — ANTES
  verificar que las firmas `pregunta_01…10` aceptan `&dyn GraphStore` tal cual: son las
  del cap-41, sin tocar); (10) CSV paso-3 + historico.csv + round-trips + generar y
  commitear `datasets/kb-lira/paso-3/`; (11) `informe_temporal_reproducible`; (12)
  wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap43` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap43_temporalidad.rs`
  y continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila
  (falta un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas).
  PROHIBIDO tocar `cap41_modelado.rs`, `cap42_antipatrones.rs`, el parser de LiraQL o
  el trait `GraphStore`: la temporalidad se hace CON la API existente.
- **Criterio de parada honesto**: si añadir valid-time cambia la respuesta de alguna de
  las 10 preguntas sobre el subgrafo paso-1 (o los valores pineados del paso-2), el
  cambio está MAL y se rediseña DENTRO del capítulo — prohibido «ajustar» la pregunta o
  el dataset. La ÚNICA diferencia permitida es la prevista: la P4 atemporal sigue
  diciendo Beto→Neurónica y la P4 CON tiempo dice Beto→GrafoLuna — si el test de
  regresión comparara las dos y fallara, es que se escribió mal el test, no el modelo.
  Igual con los costes: 14 = 14 lecturas para 2026 y 2023 — si el ledger midiera otra
  cosa, se explica POR QUÉ, prohibido maquillar contadores. La semántica de intervalo es
  `[desde, hasta)` (medio abierto): AS OF 2024 = GrafoLuna (Neurónica caducó) — respetar
  la convención en TODA la implementación.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5; citas verificadas 2026-08-26: Snodgrass Morgan Kaufmann julio 1999,
  Jensen & Snodgrass IEEE TKDE 11(1):36-44 ene/feb 1999, TSQL2 Kluwer 1995, ISO/IEC
  39075:2024 GQL abril 2024 [AS OF Parte 1 [VERIFICAR]], Neo4j 3.4 (2018) tipos
  temporales nativos, Holme & Saramäki Physics Reports 519(3):97-125 oct 2012, Kostakos
  Physica A 388(6):1007-1023 2009; JanusGraph/TigerGraph descartados por no verificables
  hoy; SQL:2011 marcado [VERIFICAR]).
- [ ] Escenario de fallo visible, no solo happy path: la arista vencida (53) que el
  presente descarta; el borrado que destruye el AS OF; el fixture corrupto del validador
  paso-3; el caso de Dani (dos respuestas legítimas); la limitación del WAL demostrada.
- [ ] Código ejecutable citado por nombre (`cap43_temporalidad.rs`, wiring, artefacto
  `datasets/kb-lira/paso-3/`, SIN `[[bench]]`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: seis).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa como patrón;
  reto esencial: predecir AS OF 2020/2024; intermedio: cap-30 vs valid-time y WORKED_ON;
  experto: WalTransaccion real contra «¿qué creíamos en 2024?»).
- [ ] ≥1 ejercicio de retrieval (los tres tiempos DE MEMORIA + clasificar 5 afirmaciones
  evento/validez/registro) y spacing planificado (caps. 41/42/28/30/32/34/26 + Vol.II;
  §7).
- [ ] Responde las TRES preguntas críticas del capítulo (representación = props de
  arista con la escalera R1-R7 y el díptico con Resena; bitemporalidad = el caso de Dani
  con dos ejes; coste = lecturas con la tesis del barrido 14 = 14) y cobra el gancho del
  cap-42 («¿QUÉ valía el 3 de marzo?» → la nota 7, con la frontera del grano declarada).
- [ ] Red de seguridad TRIPLE con tests de nombre exacto:
  `las_10_preguntas_del_paso1_no_cambian_tras_anadir_valid_time`,
  `las_respuestas_del_paso2_no_cambian_tras_anadir_valid_time`,
  `validador_paso2_acepta_el_modelo_paso3`.
- [ ] Anécdota única verificada con fuentes primarias (Holme & Saramäki 2012).
- [ ] Alcance acotado y honesto (UN módulo + wiring + artefacto paso-3; cero deps, cero
  benches, cero cambios caps. 7-42; frontera dura con caps. 44-46 declarada; el
  Historico en RAM sin durabilidad).
- [ ] Blockquote inicial declara TERCER CAPÍTULO DEL VOL.III (audiencia + conexión con
  caps. 41-42 por referencias) y gancho cobrado literalmente (la nota 7 del 3 de marzo);
  gancho saliente fijado (cap. 44: el índice y la unicidad temporal; cap. 45: el
  transaction-time de ingesta).


