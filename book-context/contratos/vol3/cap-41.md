# CONTRATO DE CAPÍTULO — Vol.III Cap. 41: Modelar entidades, propiedades y relaciones

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **PRIMER CAPÍTULO DEL VOL.III**
> («Grafos en la era de la IA: modelar, razonar y recuperar») y ABRE LA PARTE I «Modelar
> datos de grafos». Audiencia declarada en el blockquote inicial del capítulo: el lector
> que terminó el Vol.II (o perfil de datos/IA que entra por la ruta «arquitecto» del
> prólogo, con LPG e internals básicos asumidos). Conexión con LiraDB SIN repetirla:
> aquí el motor ya existe y se USA — el modelo Property Graph (Vol. II, cap. 7), el
> store en memoria (Vol. II, cap. 8), LiraQL end-to-end (Vol. II, caps. 17-21) y el
> formato CSV (Vol. II, cap. 32) son herramientas, no contenido. COBRA la deuda del
> cierre del Vol.II (cap. 40): «¿y cuando la MEMORIA de un agente necesita un grafo?» —
> esta es la primera piedra: la KB-Lira, base de conocimiento del equipo de investigación
> que evolucionará hasta ser memoria de agente en el cap. 53. Código ancla VERIFICADO hoy
> (2026-08-26): `lib.rs` declara **31 módulos** (`cap07_modelo` … `cap40_distribucion`);
> labels múltiples nativas (cap07_modelo.rs:58), `Edge` con props (:86), rechazo de
> duplicados (cap07_modelo.rs:153, cap08_graph_store.rs:76); trait `GraphStore`
> hexagonal (cap08_graph_store.rs:10); `run(src, &dyn GraphStore)` ejecuta LiraQL
> completa (cap20_volcano.rs:1411) — SIN agregación (operadores caps. 19-20: scan/
> expand/filter/project/cartesian/distinct/limit); formato CSV estilo neo4j-admin
> (cap32, lib.rs:245); disciplina de dataset determinista (`SEMILLA_REFERENCIA`
> cap34_benchmarks.rs:64; `dataset_referencia_mini` :400). Estado verificado 2026-08-26:
> **901 tests** ALL_GREEN; runtime dependency-free (dev-deps: tempfile/proptest/
> criterion 0.7); toolchain pinneada 1.96.0. Código NUEVO previsto: UN módulo `src/cap41_modelado.rs`
> (~700-1000 líneas, std puro) + wiring ADITIVO en `lib.rs` (2 líneas) + el artefacto
> regenerable `liradb-workspace/datasets/kb-lira/paso-1/{nodes.csv,edges.csv}` — CERO
> deps nuevas, CERO cambios en caps. 7-40, **SIN bench** (decisión #12). Citas
> VERIFICADAS hoy (2026-08-26, venue/año exactos): Robinson-Webber-Eifrem, *Graph
> Databases*, 2ª ed., O'Reilly, **junio 2015** (1ª ed. 2013); Angles-Gutierrez,
> «Survey of graph database models», **ACM Computing Surveys 40(1), 2008**;
> Angles-Gutierrez, «An Introduction to Graph Data Management», Springer LNCS 10000,
> **2018** (arXiv:1801.00036); Francis et al., «Cypher: An Evolving Query Language for
> Property Graphs», **SIGMOD '18**, pp. 1433-1445, DOI 10.1145/3183713.3190657;
> especificación openCypher [VERIFICAR año CIF antes de citarlo]; RDF 1.1, W3C
> Recommendation **25-feb-2014**; GQL = ISO/IEC 39075:2024 (ya citado en Vol.II);
> Kùzu = CIDR 2023 CC-BY 4.0 según ADR-001. Preguntas críticas del CORPUS
> (`vol-III-cap-41`, Parte I): «Criterios reproducibles node vs edge vs property (más
> allá del instinto)» y «¿Cómo se vincula el modelo con las consultas previstas?».
> Gancho saliente: cap. 42 (antipatrones) — este capítulo SIEMBRA a propósito un mini-hub.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el modelo Property Graph con identidad propia de nodos Y
  aristas, labels múltiples y props tipadas (`Value`) (Vol. II, cap. 7); el trait
  `GraphStore` y `MemoryStore` (Vol. II, cap. 8); LiraQL de texto a ResultSet con
  optimizador (Vol. II, caps. 17-21); travesías out/in/undirected (Vol. II, cap. 20);
  BFS/DFS sólidos (Vol. I, caps. 3-4). Perfil IA/datos: llega por el prólogo con estas
  mismas piezas como prerrequisito declarado.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «todo sustantivo del requisito es una entidad» — no: «la fecha de publicación»
  pasa todas las pruebas de propiedad; el sustantivo NO decide, la prueba de la
  identidad propia SÍ.
  (2) «las propiedades son gratis» — el coste está en CONSULTAR: una lista `"Ana;Beto"`
  como string mata búsquedas inversas y travesías (nadie expande una string).
  (3) «primero el modelo completo, luego las consultas» — al revés: sin preguntas
  previstas no hay árbitro para decidir nodo vs propiedad (query-first).
  (4) «un grafo dirigido no representa simetrías» — se representan con expansión
  undirected o aristas duales; la dirección vive en la SEMÁNTICA (citar sí,
  co-publicar no).
  (5) «reificar siempre es más correcto» — reificar sin necesidad rompe la lectura del
  grafo y multiplica saltos; aquí solo la REGLA (los antipatrones son el cap. 42).
- **Objetivos de dominio (teaching)**: Knowledge — sabe decir QUÉ hace que algo sea
  nodo (identidad propia, relaciones y ciclo de vida independientes), QUÉ lo hace
  arista (vínculo entre dos nodos sin vida fuera del par) y QUÉ propiedad (valor
  filtrable sin identidad); sabe POR QUÉ las aristas tienen identidad propia en LPG y
  qué gana con props de arista (Angles-Gutierrez 2008/2018; Francis et al. SIGMOD '18).
  Skills — construir `kb_lira_paso1()` sobre `MemoryStore`; responder las 10 preguntas
  del contrato con LiraQL/API y LEER su coste en lecturas; validar el modelo con el
  validador de tipos. Wisdom — decide CUÁNDO reificar (solo con identidad/relaciones/
  ciclo propio) y CUÁNDO NO modelar algo («si ninguna pregunta prevista lo cruza,
  quizá ni merece existir en el grafo»).
- **Pregunta crítica que el capítulo tiene que responder**: «Criterios reproducibles
  node vs edge vs property» + «vinculación modelo↔consultas». Respuesta medible: la
  escalera R1-R7 aplicada frase a frase sobre KB-Lira, y las 10 preguntas previstas
  ANTES del modelo, cada una con test que demuestra que el modelo la responde — más el
  contraejemplo medible: el modelo ingenuo todo-propiedades responde P6 escaneando TODOS
  los documentos mientras el LPG expande desde el nodo destino. Sin equivalencia
  testeada no hay capítulo: sería opinión de pizarra, no ingeniería.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| Módulo `cap41_modelado.rs` (std puro, sin deps): constructor determinista `kb_lira_paso1() -> MemoryStore` — ids FIJOS y datos fijos (~6 Personas, 3 Organizaciones, 3 Proyectos, ~12 Documentos con labels múltiples `Documento`+`Paper`/`Nota`/`Informe`, 6 Temas, ~40 aristas AUTHORED/CITES/MEMBER_OF/WORKED_ON/ABOUT/MENTIONS), siguiendo la disciplina del dataset determinista (cap. 34) | `cargo test -p vol2-liradb --lib cap41`: `estructura_de_kb_lira_paso1_cuenta_y_etiquetas_exactas` (nodos/aristas por label contados a mano sobre el builder) |
| **Validador del modelo como código** (semilla de constraints/shapes): `validar_modelo_kb_lira(store) -> Result<(), Vec<Violacion>>` — tipos de extremos por tipo de arista (AUTHORED: Persona→Documento; CITES: Documento→Documento; ABOUT→Tema; MENTIONS→Persona|Organizacion|Proyecto), campos obligatorios (todo Documento con titulo+anio) | `validador_acepta_el_modelo_canonico` y `validador_rechaza_fixture_corrupto` (grafo roto A MANO: CITES hacia Tema → violaciones con ids) |
| **Modelo ingenuo comparable**: `ModeloDocsTodopropiedades` — documentos con autores/citas/temas como strings concatenadas + contador de LECTURAS | tesis `naive_y_lpg_responden_menciones_con_costes_distintos`: P6 bajo naive exige escanear TODOS los documentos (lecturas = nº docs), bajo LPG expande SOLO desde el nodo destino (lecturas = grado entrante); cifras EXACTAS en fixture conocido |
| **Las 10 preguntas del contrato** (§3), cada una con test: mezcla `run()` LiraQL donde el lenguaje mini llega y API directa donde no (frontera DECLARADA, decisión #8) | `pregunta_01_documentos_de_una_persona` … `pregunta_10_citas_recientes_que_tratan_un_tema` (lista EXACTA de nombres en §3; salidas reales pegadas en prosa) |
| Direccionalidad y multigrafos demostrados: paralelas permitidas (co-autoría repetida), inversas legibles | `aristas_paralelas_coautoria_visibles_como_multigrafo` (dos AUTHORED entre el mismo par vía documentos distintos + EdgeIds distintos) y `citas_solo_apuntan_en_direccion_de_lectura` (in_edges/out_edges) |
| CSV determinista reutilizando el formato del cap. 32: `csv_nodos_kb_lira(store)` / `csv_aristas_kb_lira(store)` -> String + round-trip import→export idempotente (tempfile) | `csv_roundtrip_import_export_byte_a_byte` (export → import → export: bytes IDÉNTICOS) |
| Artefacto regenerable `liradb-workspace/datasets/kb-lira/paso-1/{nodes.csv,edges.csv}` commiteado; su contenido ES la salida de los builders (regenerable byte a byte) | test de bytes + verificación manual única al generar |
| Informe reproducible para la prosa: `informe_modelado_reproducible(store)` — tabla pregunta → respuesta(s) → coste (lecturas), SIN tiempos | `informe_modelado_reproducible_sobre_kb_lira` |
| **SIN `[[bench]]` nuevo** (decisión #12, espejo de la #11 del cap. 40): ninguna afirmación del capítulo depende de cronometraje; la moneda son CONJUNTOS y CONTADORES exactos | `verify.sh` compila `--all-targets` igual; prosa pega salidas de `cargo test` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**901 + ~20 tests nuevos ≈ 921**); cero cambios en caps. 7-40, goldens intactos |

---

## 3. Las preguntas críticas del CORPUS y la respuesta del capítulo

**Preguntas**: «Criterios reproducibles node vs edge vs property» y «¿Cómo se vincula
el modelo con las consultas previstas?». El capítulo las responde invirtiendo el orden
tradicional: PRIMERO se escriben las preguntas, DESPUÉS se deriva el modelo. Las 10
preguntas previstas para KB-Lira paso-1 (CONTRATO VINCULANTE — cada una con test):

1. ¿Qué documentos ha escrito Ana? → AUTHORED saliente desde Persona.
   Test: `pregunta_01_documentos_de_una_persona`.
2. ¿Quiénes firman el paper X y EN QUÉ ORDEN? → AUTHORED entrante + propiedad de
   arista `order`. Test: `pregunta_02_autores_en_orden_de_firma`.
3. ¿A quién cita el paper X y quién cita AL paper X? → CITES en ambas direcciones.
   Test: `pregunta_03_citas_en_ambas_direcciones`.
4. ¿Quién trabaja en el proyecto Y y en qué organización está afiliado? → WORKED_ON +
   MEMBER_OF (travesía mixta de 2 saltos). Test: `pregunta_04_proyecto_y_afiliaciones_en_dos_saltos`.
5. ¿De qué temas trata el documento Z (y qué documentos tratan el tema T)?
   → ABOUT ambas direcciones. Test: `pregunta_05_temas_de_un_documento_e_inversa`.
6. ¿Qué documentos mencionan a esta organización/persona/proyecto?
   → MENTIONS polimórfico. Test: `pregunta_06_menciones_a_una_entidad`.
7. ¿Han co-publicado Ana y Beto alguna vez? → par hacia documento común.
   Test: `pregunta_07_copublicacion_entre_dos_personas`.
8. ¿Cuántas publicaciones tiene cada miembro del equipo? → conteo de filas por autor
   sobre el ResultSet (LiraQL mini SIN agregación — frontera honesta).
   Test: `pregunta_08_publicaciones_por_persona_contadas`.
9. ¿Qué temas conectan a Ana con Beto vía sus papers? → camino 3 saltos
   Persona→Documento→Tema←Documento←Persona. Test: `pregunta_09_temas_comunes_via_papers`.
10. ¿Qué papers posteriores a 2023 citan al paper P y tratan del tema T? → CITES
    entrante + ABOUT + filtro por propiedad `year`. Test: `pregunta_10_citas_recientes_que_tratan_un_tema`.

Escalera del brief (5 secciones → 5 peldaños):

1. **De los requisitos al modelo** → entrevistar las preguntas: las 10 anteriores se
   redactan ANTES de dibujar un solo círculo; cada decisión del modelo cita la pregunta
   que la paga.
2. **Nodo vs propiedad: la prueba de la identidad propia** → tres preguntas (¿identidad
   independiente?, ¿relaciones propias?, ¿ciclo de vida propio?) ordenadas en la
   escalera R1-R7 (§5, decisión #4): fecha y título caen como propiedades; Persona,
   Tema y Organización sobreviven como nodos.
3. **Arista vs nodo: reificación y relaciones con atributos** → `order` en AUTHORED es
   atributo DEL VÍNCULO (propiedad de arista basta — Francis et al., SIGMOD '18);
   contraste RDF: allí TODO atributo es triplete independiente (W3C RDF 1.1, 2014),
   adelanto del cap. 46; el caso «Reseña» queda COMO RETO intermedio: decide el lector
   con la prueba, no el libro por decreto.
4. **Direccionalidad y multigrafos** → CITES dirigida por semántica documental;
   co-autoría como multigrafo con paralelas legibles; inversas vía expansión
   undirected/in_edges (Vol. II, cap. 20).
5. **Prueba de fuego** → las 10 preguntas respondidas y su coste en lecturas; el tema
   popular acumula aristas ABOUT sin que nadie lo pidiera — el mini-supernodo sembrado
   que el cap. 42 refactorizará.

Hilo conductor: «una PROPIEDAD es un valor que nunca puedes expandir; una ARISTA es una
puerta que siempre puedes cruzar — modelar es decidir qué puertas existen ANTES de que
te pregunten por el camino».

---

## 4. La arquitectura: el modelo es la respuesta comprimida a tus consultas

Modelo mental único: **la escalera frase → decisión**, con las consultas previstas como
árbitro final. Cada frase del requisito baja la escalera; cada peldaño tiene una salida.

```text
FRASE DEL REQUISITO          PRUEBA DE LA IDENTIDAD PROPIA              DECISIÓN
«Ana escribió el paper P»    Ana: se nombra sola, se relaciona sola  → NODO (:Person)
                             "escribió": vive solo del par Ana-P     → ARISTA (:AUTHORED)
                             orden de firma: atributo del vínculo    → PROP. de la arista
«publicado en 2023»          2023: sin relaciones ni vida propia     → PROPIEDAD (year)
```

```text
        AUTHORED{order}            CITES                MENTIONS (destino polimórfico)
(:Person) ────────────────► (:Document) ────────────► (:Document)
    │                          │      │                también → :Person/:Org/:Project
    │ MEMBER_OF                ▼      ▼ ABOUT
    ▼                      (:Topic) ◄─┘
(:Organization)             hub NATURAL: el tema popular acumula aristas
    ▲                       (semilla deliberada del cap. 42)
    │ WORKED_ON
(:Project)
```

Y debajo, la REGLA DE ORO heredada del cap. 34: dataset determinista (ids y datos
FIJOS en el builder, nada de RNG), contadores de TRABAJO (lecturas) dentro de los
tests, y CERO tiempos de pared — aquí la física es ESTRUCTURAL: qué se puede preguntar
y a qué coste de lecturas, no cuánto tarda.

```text
Lo que SÍ se hace hoy:   capa EDUCATIVA aparte (módulo propio, como caps. 38-40):
                         builder determinista de KB-Lira paso-1, validador de modelo,
                         modelo ingenuo comparable con contador de lecturas, las 10
                         preguntas testeadas (LiraQL cuando llega, API cuando no,
                         frontera declarada), CSV determinista + round-trip, informe
Lo que AÚN NO:           antipatrones y refactors (cap. 42) · valid-time/bitemporal
                         (cap. 43) · constraints e índices (cap. 44) · ingesta CSV cruda
                         con duplicados (cap. 45) · RDF/tripletas (cap. 46) · SHACL
                         (cap. 47) · agregación COUNT en LiraQL (otra parte; frontera
                         declarada) · tipos garantizados en extremos de aristas
```

Momento ¡ajá! perseguido: «llevas dos volúmenes construyendo MOTORES que responden
consultas; resulta que la mitad de las respuestas malas no son del motor sino del
MODELO: lo que dibujaste como propiedad jamás podrá cruzarse, y lo que dibujaste como
nodo te cobra travesías para siempre».

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Alcance de código: UN módulo `cap41_modelado.rs` DENTRO de `crates/vol2-liradb` (continúa la numeración cap07…cap40), std puro | La casa enseña con código testeado («sin equivalencia testeada no hay capítulo»); el módulo accede GRATIS a MemoryStore/run/CSV; Apéndice 0 §0.5: un workspace único, un módulo por capítulo; renombrar crate rompería rutas/goldens de 901 tests por cosmética | (a) Crate nueva `vol3-*`: churn de Cargo.toml/workspace sin ganancia pedagógica; (b) SOLO prosa/diagramas: traicionaría el estándar del proyecto | CONVENTIONS §2 y §4; Apéndice 0 §0.5; precedente caps. 38-40 (módulo educativo aditivo) |
| 2 | El generador KB-Lira vive como FUNCIÓN constructora en el módulo; `datasets/kb-lira/paso-1/` es el ARTEFACTO (CSV commiteado, regenerable byte a byte desde el builder) | Fuente única de verdad ejecutable; el fichero es derivado, no fuente; la CLI/binario generador no aporta hasta la ingesta real del cap. 45 | Binario generador dedicado o script suelto: duplicaría la verdad y exigiría mantener otro target | OUTLINE-VOL3.yml línea 27 («generador… pendiente» — se interpreta como artefacto); disciplina cap. 34 |
| 3 | Query-first: las 10 preguntas se REDACTAN antes del modelo y cada pieza del modelo cita la pregunta que la justifica | Es la respuesta directa a la pregunta crítica #2 del CORPUS; sin árbitro, nodo-vs-propiedad es gusto personal | Modelar «lo que aparece en la frase»: misconcepción #1/#3 desmontada en N.2 | Robinson-Webber-Eifrem 2015 (cap. 4: guidelines orientadas a preguntas); CORPUS vol-III-cap-41 |
| 4 | Prueba de la identidad propia FORMALIZADA como escalera R1-R7: R1 identidad (¿se nombra por sí?), R2 relaciones propias (¿se relaciona fuera de su contenedor?), R3 ciclo de vida propio (≥2 SÍ ⇒ nodo candidato); R4 cardinalidad múltiple ⇒ no cabe como propiedad única; R5 vínculo entre nodos sin vida fuera del par ⇒ arista (+props si tiene atributos del vínculo; nodo intermedio SOLO si gana relaciones/ciclo propios); R6 valor simple filtrable ⇒ propiedad; R7 árbitro: ante empate GANA la consulta prevista | Convierte el «instinto» del experto en checklist reproducible — exactamente lo que pide el CORPUS (#1) | Dejar los criterios como consejos sueltos en prosa: no reproducibles, no enseñables a novato | Angles-Gutierrez CSUR 40(1) 2008 y capítulo 2018 (definición formal LPG: nodos Y aristas con identidad propia); Robinson et al. 2015; Francis et al. SIGMOD '18 |
| 5 | Propiedad de ARISTA `order` en AUTHORED (no nodo intermedio `Autoria`) | El orden de firma pertenece AL VÍNCULO, no a endpoints; reificar aquí añadiría un salto por consulta P1-P2 sin comprar nada — antipatrón temprano que el cap. 42 sistematizará | Nodo intermedio Autoria{orden}: reificación excesiva en el primer modelo del volumen | Francis et al. SIGMOD '18 (relationships carry properties); Robinson et al. 2015; frontera dura OUTLINE-VOL3.yml cap-42 |
| 6 | Subtipos de documento como LABELS múltiples (`:Document`+`:Paper`/`:Nota`/`:Informe`), no propiedad `type` | Cap. 7 ya soporta Vec<String> de labels (línea 58); la etiqueta despacha patrones (MATCH (d:Paper)) y agrupa estadísticas; la propiedad sería un filtro más débil | Propiedad `type:string`: obliga a filtrar en WHERE en vez de seleccionar en MATCH | cap07_modelo.rs:58; Francis et al. SIGMOD '18; ISO/IEC 39075:2024 |
| 7 | MENTIONS con destino POLIMÓRFICO (→Persona|Organizacion|Proyecto) y validador que suple la ausencia de tipos | Es honesto con LPG: el modelo NO garantiza tipos en extremos; el validador convierte esa convención en test ejecutable (y siembra constraints del cap. 44 y shapes del cap. 47) | Aristas separadas MENTIONS_PERSON/ORG/PROJECT: triplica vocabulario y fragmenta P6 | Angles-Gutierrez 2018 (LPG sin schema de extremos); ISO/IEC 39075:2024 |
| 8 | P8 (conteo) se responde contando filas del ResultSet desde Rust; frontera DECLARADA: LiraQL mini no tiene agregación | Honesta con caps. 17-21 (scan/expand/filter/project/distinct/limit — sin COUNT); añadir agregación rompería goldens y diluye el foco (es contenido de otra parte) | Extender el parser con COUNT para «que pasen» las 10: alcance prohibido (riesgo §8) | cap20_volcano.rs (conjunto de operadores); honestidad hexagonal caps. 33-34 |
| 9 | Direccionalidad: CITES dirigida; simetrías vía in_edges/expansión undirected; paralelas visibles como multigrafo | La dirección es SEMÁNTICA (quien cita no es citado); LPG admite paralelas con ids propios — el caso co-autoría lo demuestra con fixture | Forzar simetrías con nodos intermedios: reificación injustificada | cap20 ExpandOp direcciones; Robinson et al. 2015; Angles-Gutierrez 2008 (multigraph en LPG) |
| 10 | Validador de modelo COMO CÓDIGO (`Violacion` con ids) | El contrato del modelo deja de ser prosa: es test ejecutable, semilla directa de constraints (cap. 44) y shapes (cap. 47); detecta el drift del dataset en caps. futuros | Validar a mano en cada capítulo: no escala a 13 pasos de KB-Lira | Diseño propio anclado en la necesidad del hilo conductor (caps. 44/47) |
| 11 | CSV con el FORMATO del cap. 32 (estilo neo4j-admin); JSONL queda para el cap. 45 | Spacing puro: reutiliza exporter/importer probados; el round-trip idempotente es test barato con tempfile | Formato nuevo ad-hoc: duplicación gratuita y riesgo de drift con cap32 | cap32 (lib.rs:245-269); OUTLINE-VOL3.yml cap-45 |
| 12 | **SIN bench criterion** (desviación declarada, espejo de la #11 del cap. 40) | La tesis es estructural: conjuntos respondibles y lecturas exactas; cronometrar el builder no sostiene ninguna afirmación del capítulo | Primer bench del Vol.III: números de reloj sin hipótesis | Metodología cap. 34 (¿qué afirma el capítulo y con qué moneda lo demuestra?) |
| 13 | Frontera dura con cap. 42: aquí SOLO la regla de decisión + un caso claro por sección; el mini-hub Tema se SIEMBRA a propósito (gancho medible) | Una idea nueva por sección (CONVENTIONS §2); el supernodo/refactor/reificación excesiva son EL contenido del cap. 42 — adelantarlos vaciaría ese capítulo | Enseñar antipatrones aquí «para completar»: viola outline y carga cognitiva | OUTLINE-VOL3.yml caps. 42-44; CONVENTIONS §2 |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Blockquote inicial OBLIGATORIO**: PRIMER capítulo del Vol.III, audiencia
   (lector del Vol.II + perfiles IA/datos por la ruta del prólogo), conexión mínima
   con LiraDB por referencias `(Vol. II, cap. N)` — sin re-explicar internals.
2. **Apertura (N.0, anécdota + pregunta crítica)**: 2013-2024 — *Graph Databases*
   (O'Reilly, 2013/2015) populariza el property graph como forma natural de pensar;
   Cypher nace atado a ese modelo y acaba adoptado por media industria (Francis et
   al., SIGMOD '18) hasta estandarizarse en GQL (ISO/IEC 39075:2024). Pregunta
   enmarcada: tu equipo quiere preguntarle cosas a SUS documentos — ¿qué ES cada cosa
   antes de preguntar?
3. **N.1-N.2 Objetivo/Problema**: motor verde (901 tests) — y sin embargo «¿qué papers
   citó Ana?» no tiene respuesta: nadie decidió aún qué es nodo. Desactivar las cinco
   misconceptions ANTES de dibujar.
4. **N.3 Modelo mental**: panel frase→decisión + escalera R1-R7 + bloque sí/no +
   KB-Lira dibujado (§4).
5. **N.4 Primera solución**: el modelo ingenuo todo-en-propiedades (autores:"Ana;Beto",
   citas:"D2;D5") — funciona en la pizarra y responde P1 a base de split.
6. **N.5 Sus límites**: el contador de lecturas delata a P6 (escaneo total vs expansión
   local); P3/P9 IMPOSIBLES sin parsear strings FUERA del store.
7. **N.6 Solución evolucionada**: aplicar R1-R7 pieza a pieza — nodos con identidad,
   aristas semánticas, props de arista (`order`), labels múltiples, MENTIONS
   polimórfico con validador; cada gesto cita la pregunta que lo paga.
8. **N.7 Código completo ejecutable**: `cap41_modelado.rs` por `include::` (nunca
   duplicado); SIN bench (decisión #12 explicada en una línea).
9. **N.8 Prueba de fuego**: las 10 preguntas verdes con salidas REALES de `cargo test`;
   validador acepta/rechaza; CSV round-trip byte a byte; tabla pregunta→coste(lecturas).
10. **N.9 Qué hemos sacrificado**: sin agregación en LiraQL; sin tipos garantizados en
    extremos (el validador es convención ejecutable); afiliaciones sin historia
    temporal (cap. 43); sin unicidad/constraints (cap. 44); sin RDF (cap. 46).
11. **N.10 Cómo lo hace una BBDD real + retos**: guías de modelado de Neo4j (docs
    oficiales), Kùzu/LadybugDB según ADR-001 (CIDR 2023, CC-BY 4.0) y el pipeline
    GraphRAG industrial como preview de por qué el modelo importa para LLMs (sin URLs
    fabricadas). Retos: esencial (añadir persona+paper al builder y PREDECIR P1/P3
    antes de correr), intermedio (modelar «Reseña»: arista con props vs nodo
    intermedio, JUSTIFICAR con la prueba y testear la consulta), experto (darle índice
    manual al modelo naive y MEDIR lecturas vs LPG — explicar por qué sigue perdiendo).
12. **Baterías finales + gancho**: Lo que te llevas / Ojo cuidado / Pin / 30 segundos /
    historia pequeña / Mini-diálogo de guardia nocturna (la consulta que pedía «los
    temas de Ana» contra una lista separada por comas). Retrieval practice: reproducir
    DE MEMORIA la escalera R1-R7 y clasificar 5 frases nuevas sin mirar. Interleaving:
    cada reto toca ≥2 capítulos (7/8+41, 17-18/21+41, 20+41, 32+41, 34+41). Glosario
    nuevo: entidad, identidad propia, propiedad de arista, reificación (mínima),
    multigrafo/aristas paralelas, direccionalidad, esquema abierto, query-first.
    Gancho al cap. 42: «el Tema popular de tu flamante KB-Lira YA acumula aristas que
    nadie pidió: ¿cuándo un hub deja de ser inocente?». Abiertas: hechos que cambian
    (43), garantías de unicidad (44).

---

## 7. Estilo y tono (consistencia con el proyecto)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (node, relationship, property, reification, multigraph,
  schema-optional); salidas REALES de `cargo test` pegadas, nunca reconstruidas; las
  decisiones de modelo se presentan como TRADE-OFF justificado contra consultas, nunca
  como dogma.
- **Diagramas**: panel frase→decisión (§4); grafo KB-Lira con su hub sembrado; tabla
  pregunta→coste; escalera R1-R7 como figura recurrente de las baterías.
- **Spacing** (conceptos viejos que se EJERCITAN): modelo/labels/Value (cap. 7),
  GraphStore/MemoryStore (cap. 8), LiraQL parse→run (caps. 17-20), optimizador y
  estadísticas por etiqueta (cap. 21, mención), formato CSV (cap. 32), disciplina de
  datasets deterministas (cap. 34), travesía mental BFS (Vol. I, caps. 3-4).
- **Interleaving**: reto esencial mezcla 34+41 (predicción sobre dataset determinista);
  el intermedio mezcla 7+41 (labels/props vs nodo); el experto mezcla 20+41 (coste de
  expansión vs escaneo con contadores).
- **Dificultad asimétrica**: una idea nueva por sección (entrevistar preguntas →
  identidad propia → reificación mínima → dirección/multigrafo → prueba de fuego);
  los ejercicios exigen PREDECIR respuestas y costes ANTES de correr los tests.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap41` (contadores y
  conjuntos exactos) y `./scripts/verify.sh` ALL_GREEN como puerta. Nunca «confía en mí».
- **Anécdota (única, verificada)**: del modelo al estándar — *Graph Databases*
  (O'Reilly 2013/2ª ed. 2015) → openCypher/Cypher adoptado multi-vendor (SIGMOD '18)
  → GQL ISO/IEC 39075:2024. Fuentes para la prosa: Robinson-Webber-Eifrem (O'Reilly
  2015); Angles-Gutierrez (CSUR 40(1), 2008; Springer 2018); Francis et al. (SIGMOD
  '18, pp. 1433-1445); W3C RDF 1.1 (Rec., 25-feb-2014) como contraste en §3-peldaño 3;
  ISO/IEC 39075:2024; Jin et al. (CIDR 2023, CC-BY 4.0, ADR-001).

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap41_modelado; pub use
  cap41_modelado::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al final, con
  el módulo ya compilando limpio (`cargo check -p vol2-liradb`); jamás dejar `lib.rs`
  apuntando a un módulo rojo.
- **Orden de implementación recomendado** (cada paso compila y testea solo): (1)
  `kb_lira_paso1()` + test de estructura; (2) validador + fixture corrupto; (3) modelo
  naive + contadores + tesis de lecturas; (4) preguntas 1-5 vía `run()`; (5) preguntas
  6-10 (API/undirected — ANTES verificar en cap18/cap20 qué patrones direccionales
  admite el parser y ajustar la prosa prometida); (6) CSV determinista + round-trip
  tempfile; (7) informe reproducible; (8) generar y commitear `datasets/kb-lira/paso-1/`;
  (9) wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap41` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap41_modelado.rs`
  y continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). PROHIBIDO
  extender el parser de LiraQL para «que pasen» las 10 preguntas: si una no cabe en el
  lenguaje, API directa + frontera declarada (o deuda explícita hacia el cap. 48).
- **Criterio de parada honesto**: si alguna de las 10 preguntas NO se responde con el
  modelo elegido, se REPORTA tal cual y el modelo SE REFACTORIZA dentro del capítulo
  usando la escalera R1-R7 — ESA es la lección del capítulo (el modelo se justifica
  contra las consultas), no un fallo que esconder. Igual si naive vs LPG no muestra
  diferencia de lecturas en algún caso: se explica POR QUÉ, prohibido inflar.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (13 filas en §5; citas verificadas 2026-08-26: Robinson et al. O'Reilly 2015,
  Angles-Gutierrez CSUR 2008 + Springer 2018, Francis et al. SIGMOD '18, W3C RDF 1.1
  2014, ISO/IEC 39075:2024, CIDR 2023/ADR-001; openCypher CIF marcado [VERIFICAR]).
- [ ] Escenario de fallo visible, no solo happy path: lista-como-propiedad que obliga
  a escanear (medido), CITES hacia Tema cazada por el validador, documento sin año,
  conteo imposible en LiraQL (frontera declarada), paralelas visibles como multigrafo.
- [ ] Código ejecutable citado por nombre (`cap41_modelado.rs`, wiring, `datasets/kb-lira/paso-1/`,
  SIN `[[bench]]`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: cinco).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa como patrón).
- [ ] ≥1 ejercicio de retrieval (escalera R1-R7 + clasificación de memoria) y spacing
  planificado (caps. 7/8/17-18/20/32/34 + Vol.I 3-4; §7).
- [ ] Responde las DOS preguntas críticas del CORPUS (criterios reproducibles =
  escalera R1-R7; vinculación modelo↔consultas = query-first con 10 preguntas
  testeadas) y cobra la deuda heredada del cierre del Vol.II (cap. 40 → memoria de
  agente, §blockquote).
- [ ] Anécdota única verificada con fuentes primarias (SIGMOD '18 + ISO 2024 + O'Reilly).
- [ ] Alcance acotado y honesto (UN módulo + wiring + artefacto datasets/; cero deps,
  cero benches, cero cambios caps. 7-40; frontera dura con caps. 42-48 declarada).
- [ ] Blockquote inicial declara APERTURA DEL VOL.III (audiencia + conexión LiraDB por
  referencias) y gancho saliente fijado (cap. 42: el hub sembrado; §6.12).
