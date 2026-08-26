# CONTRATO DE CAPÍTULO — Vol.III Cap. 42: Antipatrones: supernodos, reificación y otras trampas

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **SEGUNDO CAPÍTULO DEL VOL.III**
> («Grafos en la era de la IA: modelar, razonar y recuperar»), Parte I «Modelar datos de
> grafos». Audiencia declarada en el blockquote inicial: el lector que terminó el cap. 41 —
> o el perfil datos/IA que lo haya leído en diagonal — y que ya tiene la escalera R1-R7,
> las 10 preguntas de KB-Lira, el validador y el modelo naive como HERRAMIENTAS, no como
> contenido. COBRA el gancho saliente del cap. 41 (§41.12 y cierre): «el Tema popular de tu
> flamante KB-Lira YA acumula 6 aristas ABOUT que nadie pidió — ¿cuándo un hub deja de ser
> inocente?» — este capítulo responde CON NÚMEROS: un hub es inocente mientras no cruza el
> umbral del detector; deja de serlo cuando su grado desproporcionado paga la expansión de
> quien lo cruza. Progresión del hilo conductor KB-Lira: paso-1 (cap. 41) = modelo sano con
> mini-hub sembrado; paso-2 (ESTE capítulo) = el dataset CRECE con un lote importado que
> degenera el modelo, y se refactoriza ANTES de que crezca más (objetivo medible del
> outline). Código ancla VERIFICADO hoy (2026-08-26): `lib.rs` declara 32 módulos
> (…`cap40_distribucion`, `cap41_modelado`); `kb_lira_paso1()` → `MemoryStore` (30 nodos,
> 64 aristas, ids FIJOS a mano — cap41_modelado.rs:146; hub sembrado: tema 24 con 6 ABOUT,
> :275-303; verificado por el test :1203-1209); `validar_modelo_kb_lira` (:390) con
> `Violacion {descripcion, id_implicado, tipo_elemento}` (:354); `ModeloDocsTodopropiedades`
> con ledger de lecturas (:516); las 10 preguntas `pregunta_01…pregunta_10` (API
> directa/LiraQL, frontera UTF-8 declarada en cap-41); CSV formato cap. 32 con round-trip
> byte a byte; artefacto `datasets/kb-lira/paso-1/` commiteado. Trait `GraphStore`
> (cap08_graph_store.rs:10): put/get node+edge, out_edges/in_edges, delete_node/delete_edge,
> iter_edges/iter_nodes — TODO lo que un refactor necesita. `degree_centrality`
> (cap24_centralidad.rs:521) y Louvain (cap25) reutilizables a nivel conceptual; el
> hub-concentrador del cap. 39 (×521 tuplas fantasma) y el vertex-cut del cap. 40 (replicar
> el hub) son el eco de skew que este capítulo sistematiza a nivel de MODELO. Estado
> verificado: **920 tests** ALL_GREEN; runtime dependency-free; toolchain 1.96.0. Código
> NUEVO previsto: UN módulo `src/cap42_antipatrones.rs` (~700-1100 líneas, std puro) +
> wiring ADITIVO en `lib.rs` (2 líneas) + artefacto regenerable
> `liradb-workspace/datasets/kb-lira/paso-2/{nodes.csv,edges.csv}` — CERO deps nuevas, CERO
> cambios en caps. 7-41, **SIN bench** (espejo de la decisión #12 del cap-41: la moneda son
> reescrituras, lecturas y conjuntos exactos, no µs). Citas VERIFICADAS hoy (2026-08-26,
> venue/fecha exactos): David Allen, «Graph Modeling: All About Super Nodes», **Neo4j
> Developer Blog (Medium), 19-oct-2020** (definición relativa del supernodo, causas y
> toolbox de mitigación); **Neo4j GraphAcademy**, «Graph Data Modeling Core Principles»
> (curso gdm-40: supernodo = nodo con mucho fan-in/fan-out); **Neo4j Knowledge Base**,
> «How to Avoid Costly Traversals with Join Hints» [VERIFICAR fecha del artículo antes de
> citarlo]; **Aerospike Graph Service 3.0**, docs «Supernodes» (supernodo como ciudadano de
> primera clase: flag `~supernode`, listas de aristas multi-registro, ~6.500 aristas @1MiB
> de max-record-size); Justin Boylan-Toomey, «Neo4j Super Node Performance Issues», blog,
> **26-feb-2024** (KG académico real: 100M publicaciones × 176 campos de investigación;
> refactor supernodo→propiedad); Robinson-Webber-Eifrem, *Graph Databases*, 2ª ed. O'Reilly,
> **junio 2015**, cap. 2 «Avoiding Anti-Patterns» (p. 63; caso de email forense: entidad
> con identidad propia codificada como relación); Ralph Kimball, *The Data Warehouse
> Toolkit*, **Wiley, 1996** (star schema — precisión terminológica, NO es hub-and-spoke).
> JanusGraph/TigerGraph: docs NO verificadas hoy → fuera del cuerpo (Aerospike AGS cubre el
> rol de «motor que trata supernodos explícitamente»). Gancho saliente: cap. 43
> (temporalidad) — las rondas de reseña y las fechas de validez de las afiliaciones quedan
> declaradas como frontera.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la escalera R1-R7 completa (cap. 41); query-first y las 10
  preguntas de KB-Lira con sus respuestas y costes (cap. 41); el validador de modelo como
  convención ejecutable (cap. 41); por qué el modelo naive pierde (P6: 12 lecturas vs 5);
  los ids FIJOS del builder (0-29) y el hub sembrado (tema 24, 6 ABOUT); el trait
  `GraphStore` y las travesías in/out (Vol. II, caps. 7-8, 20); `degree_centrality`
  (Vol. II, cap. 24), Louvain y modularidad (Vol. II, cap. 25), la disciplina de dataset
  determinista (Vol. II, cap. 34), el skew del hub en joins y particionado (Vol. II, caps.
  39-40); BFS/DFS sólidos (Vol. I, caps. 3-4). Perfil IA/datos: entra por el prólogo con
  estas mismas piezas como prerrequisito declarado (el blockquote lo repite como el cap-41).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «un supernodo es un nodo con MILLONES de aristas» — no: el supernodo es RELATIVO a
  la distribución de su propio grafo («way more relationships than other nodes, relative
  to what else is in the graph», Allen 2020); el título de la sección dice «10M de
  aristas», pero lo que convierte un hub en supernodo es el desequilibrio, no el absoluto.
  (2) «los hubs son malos por definición» — no: hay hubs legítimos (anchors: un nodo
  `Conferencia` o `Tema` que se RESUELVE por clave y no se atraviesa); el antipatrón es el
  nodo DEGENERADO que concentra grado y se cruza en los caminos calientes (Allen 2020:
  hub de dominio vs hub de modelado; el criterio es el patrón de acceso, no el grado).
  (3) «refactorizar es rediseñar el grafo entero» — no: es una transformación QUIRÚRGICA
  con red de seguridad (las 10 preguntas del cap-41 deben seguir respondiendo sobre el
  subgrafo paso-1); si un refactor cambia una respuesta vieja, el refactor está MAL.
  (4) «reificar es bueno o malo en abstracto» — no: la MISMA escalera R1-R7 descartó
  `Autoria` (reificación excesiva, cap-41) y exige `Resena` (reificación insuficiente sin
  ella, cap-42): la prueba decide, el veredicto no es global.
  (5) «un tema por año es una comodidad inofensiva» — es el antipatrón clásico de nodo
  categórico de baja cardinalidad (el `Gender` de Allen 2020; los 176 campos de
  investigación de Boylan-Toomey 2024): pocos nodos que concentran cada uno muchísimas
  aristas.
- **No debe saber todavía**: valid-time/bitemporalidad y el WAL como log de cambios
  (cap. 43 — la RONDA de una reseña es propiedad simple, NO fecha de validez; «cuándo
  valía la afiliación» es del cap. 43); constraints UNIQUE e índices de propiedades
  (cap. 44 — el validador paso-2 lista reglas que ALLÍ serán constraints, aquí solo
  convención ejecutable); pipeline de ingesta CSV/JSONL con dedup (cap. 45 — el lote de
  este capítulo se «importa» a mano en el builder, la automatización es del 45); RDF y
  reificación de tripletas (cap. 46). El refactor NO pisa ninguna de esas fronteras.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| **Builder degenerado** `kb_lira_paso2_degrado() -> MemoryStore`: PARTE de `kb_lira_paso1()` (lo llama y añade encima — NUNCA lo copia) y le añade el **lote importado**: 3 personas nuevas (Gaby, Hugo, Iris; ids 30-32), 24 papers nuevos (ids 33-56; cada uno con AUTHORED de su autor, 6 CITES internas), 18 ABOUT concentrados en el tema 24, 3 en el 26, 2 en el 28, 1 en el 25, **12 ABOUT a dos Temas degenerados «publicaciones 2024/2025» (ids 57-58)**, 4 aristas `REVIEWED_BY{nota}` (Persona→Documento; 2 de Fabio sobre el Informe de revisión por pares — ronda 1 y ronda 2 —, 1 de Carla, 1 de Gaby) y la propiedad `conferencia:String` en cada paper del lote. Total degenerado: **59 nodos, 134 aristas** | `estructura_de_kb_lira_paso2_degrado_cuenta_y_etiquetas_exactas` (nodos/aristas por label contados a mano; tema 24 con 24 ABOUT entrantes) |
| **Detector de supernodos** `detectar_supernodos(store, umbral) -> Vec<SupernodoCandidato>`: un solo barrido sobre `iter_edges` que calcula la distribución de grados por label (mediana, ratio max/mediana, share por tipo de arista) y devuelve candidatos si (ratio ≥ 5× mediana del label) **Y** (share ≥ 25% de las aristas de ese tipo). `SupernodoCandidato {nodo_id, label, grado_entrante, grado_saliente, grado_total, mediana_label, ratio_vs_mediana, share_del_tipo}` | `el_hub_del_paso1_es_inocente_segun_el_detector` (tema 24: 6 ABOUT, mediana 2,5 → ratio 2,4× < 5×: SIN alarma — el gancho del cap-41 cobrado con números), `el_hub_del_paso2_degrado_es_candidato_a_supernodo` (tema 24: 24, mediana 3 → ratio 8×, share 46%: alarma), `el_detector_ya_no_alarma_tras_el_refactor` (tema 24: 12 directas, mediana 4 → ratio 3×: silencio), `distribucion_de_grados_exacta_sobre_kb_lira` |
| **Refactor A — descomponer el supernodo**: 3 subtemas intermedios (ids 59-61: «knowledge graphs», «GraphRAG», «memoria de agentes» — ámbito del tema 24), aristas `SUB_TEMA_DE` Tema→Tema, 12 de las 18 ABOUT del lote se redistribuyen a los subtemas (4+4+4), 6 se quedan en el tema 24; semántica DECLARADA: «documentos del tema T» = directos ∪ descendientes (P5 v2) | `refactor_descomponer_conserva_el_conjunto_de_p5` (unión directas+subtemas = las 24 respuestas de antes del refactor), `la_semantica_de_p5_sobre_el_tema_padre_se_preserva_por_union` (`documentos_del_tema_incluyendo_subtemas` devuelve los 24; la P5 original, solo las 12 directas — coste de migración honesto: UNA consulta se reescribe) |
| **Refactor B — reificar `Resena`** (el reto intermedio del cap-41 COBRADO como ejemplo canónico): las 4 `REVIEWED_BY{nota}` se sustituyen por nodos `:Resena {nota:Int, ronda:Int}` (ids 65-68) con `REALIZA` Persona→Resena, `SOBRE` Resena→Documento y `CONTRARRESTA` Resena→Resena (la ronda 2 de Fabio contrarresta a la ronda 1) | `refactor_reificar_resena_responde_rondas_y_contrarrestas` (las consultas «¿cuántas rondas pasó el Informe de revisión?» y «¿qué reseña contrarresta a otra?» responden SOLO tras el nodo; antes, las paralelas eran indistinguibles por significado) |
| **Refactor C — propiedades↔nodos, ambas direcciones**: (i) `conferencia:String` → nodos `:Conferencia` (ids 62-64) + `PUBLICADO_EN` Documento→Conferencia; (ii) los Temas-año 57-58 se BORRAN con sus 12 ABOUT (el año ya era `anio`, R6) | `refactor_conferencias_convierte_propiedad_en_nodo_y_temas_anio_en_propiedad` (24 PUBLICADO_EN dirigidos; los 2 temas-año y sus ABOUT ausentes; «¿qué papers publicó el equipo en ICDE?» responde expandiendo desde :Conferencia) |
| **Validador paso-2** `validar_modelo_kb_lira_paso2(store)`: REUTILIZA `validar_modelo_kb_lira` del cap-41 sobre el subgrafo paso-1 (sin tocarlo) + reglas nuevas: REALIZA Persona→Resena, SOBRE Resena→Documento, CONTRARRESTA Resena→Resena, PUBLICADO_EN Documento→Conferencia, SUB_TEMA_DE Tema→Tema; toda Resena con `nota:Int`+`ronda:Int`; prohibida la arista `REVIEWED_BY` residual | `validador_paso2_acepta_el_modelo_refactorizado` y `validador_paso2_rechaza_fixture_corrupto` (grafo roto A MANO: una REVIEWED_BY residual, una Resena sin nota, PUBLICADO_EN hacia un Documento → violaciones con ids) |
| **Coste de migración medido**: cada refactor es una función pura `refactor_*(&mut MemoryStore) -> InformeRefactor {nodos_creados, nodos_borrados, aristas_creadas, aristas_borradas, lecturas}`; la migración completa = composición de los tres | `la_migracion_completa_cuesta_reeescrituras_exactas` (nodos: −2, +3, +3, +4; aristas: −16 (4 REVIEWED_BY + 12 temas-año), +36 (3 SUB_TEMA_DE + 24 PUBLICADO_EN + 4 REALIZA + 4 SOBRE + 1 CONTRARRESTA); lecturas exactas del barrido) y `informe_migracion_reproducible_sobre_kb_lira` (tabla refactor→ops→impacto en las 10 preguntas, pineada byte a byte, sin tiempos) |
| **REGRESIÓN OBLIGATORIA: las 10 preguntas del cap-41** | `las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor` (cada una de las 10, filtrada a ids del paso-1: respuestas IDÉNTICAS antes y después de los tres refactors) y `pregunta_01_documentos_de_una_persona_sobre_paso2` … `pregunta_10_citas_recientes_que_tratan_un_tema_sobre_paso2` (las 10 re-ejecutadas sobre el paso-2 refactorizado con valores pineados: P1 Ana = 4 docs idéntico; P3 entra/sale con los 2 citadores nuevos; P5 tema 24 = 24 vía jerarquía; P6 Neurónica = 5 y naive 12 vs LPG 5 se mantiene; P8 = 40 AUTHORED con 9 personas) |
| **Tesis estructural del supernodo**: cada expansión desde el supernodo paga el grado entero | `el_supernodo_cobra_el_grado_entero_en_cada_expansion_y_el_refactor_lo_reparte` (P5 inversa degenerado: 24 lecturas; tras refactor: 12 + 4 + 4 + 4 repartidas en 4 expansiones pequeñas — el argumento del «tronco encontrando la hoja» de Allen 2020 con contadores) |
| CSV determinista paso-2 (formato cap. 32) + artefacto `datasets/kb-lira/paso-2/{nodes.csv,edges.csv}` commiteado (salida del builder DEGENERADO: el dataset es lo que «importó el equipo»; el refactor es código que se re-ejecuta) | `csv_roundtrip_paso2_import_export_byte_a_byte`, `csv_paso2_coincide_con_dataset_commiteado_byte_a_byte` y `csv_paso1_intacto_tras_paso2` (los ficheros del paso-1 NO se tocan) |
| **SIN `[[bench]]` nuevo** (decisión espejo #12 cap-41): ninguna afirmación depende de cronometraje; la moneda son reescrituras, lecturas y conjuntos exactos | `verify.sh` compila `--all-targets` igual; prosa pega salidas de `cargo test` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**920 + ~20 tests nuevos ≈ 940**); cero cambios en caps. 7-41, goldens intactos, paso-1 byte a byte |

---

## 3. Las preguntas críticas del capítulo y la respuesta del capítulo

**Preguntas** (propias del capítulo — el CORPUS define las de cap-41 y este hereda su
espíritu medible): (1) ¿Cómo se DETECTA un supernodo objetivamente, sin esperar a que
«se note»? (2) ¿Cuánto cuesta UN REFACTOR de modelo, y cómo se decide que merece la pena
antes de que el dataset crezca? (3) ¿Cuándo la reificación es excesiva y cuándo
insuficiente — con la MISMA regla?

Respuestas medibles:

1. **Detección**: `detectar_supernodos` con umbral DECLARADO (ratio ≥ 5× la mediana del
   label Y share ≥ 25% del tipo de arista), sobre distribución exacta de grados. La
   respuesta al gancho del cap-41 es un triplete de tests: paso-1 inocente (2,4×), paso-2
   degenerado culpable (8×, 46%), tras refactor silencioso (3×). El absoluto («10M de
   aristas») es el titular; el criterio reproducible es RELATIVO (Allen 2020: «relative to
   what else is in the graph») y se operacionaliza aquí con dos umbrales a la vez — porque
   un solo umbral relativo alarma con cualquier grafo pequeño (mediana 1 → todo nodo con 5
   aristas). La AND de ambos es la defensa.
2. **Coste de migración**: moneda = REESCRITURAS (put/delete de nodos y aristas) +
   lecturas del barrido de validación; cada refactor devuelve `InformeRefactor`; la
   migración completa se compone y se pinean sus totales. La decisión de refactorizar se
   toma ANTES de que el dataset crezca porque el coste es proporcional al grado del
   supernodo: reescribir 12 aristas hoy es barato; reescribir 10M «cuando crezca» es la
   factura del capítulo. La red de seguridad son las 10 preguntas del cap-41: si el
   refactor cambia una respuesta vieja sobre el subgrafo paso-1, el refactor está mal — no
   se «ajusta» la pregunta.
3. **Reificación**: la MISMA escalera R1-R7 decide los dos veredictos opuestos: `Autoria`
   (cap-41) cae en R5 (sin relaciones ni ciclo propios → reificación EXCESIVA); `Resena`
   (cap-42) sube en R2/R3 (la ronda 2 contrarresta a la ronda 1: relaciones propias y
   ciclo de vida → sin el nodo, la arista simple la PIERDE — reificación INSUFICIENTE).
   El caso industrial del email forense (Robinson et al. 2015, cap. 2, p. 63 «Avoiding
   Anti-Patterns») es el espejo: una entidad con identidad propia codificada como relación
   pierde al instante su historia (CC, respuestas).

Escalera del brief (5 secciones del outline → 5 peldaños):

1. **El supernodo: cuando un nodo tiene 10M de aristas** → definición relativa, detector
   y umbral, el triplete inocente/culpable/silencioso sobre KB-Lira; eco del ×521 del
   cap. 39 (la MISMA enfermedad en joins y particionado — caps. 39-40 ya la vieron, aquí
   se cura en el modelo).
2. **Reificación excesiva vs insuficiente** → el díptico Autoria/Resena con la escalera
   como juez; el caso Enron (Robinson et al. 2015) como anécdota industrial.
3. **Propiedades que deberían ser nodos (y al revés)** → conferencia:String → :Conferencia
   (P nueva: «¿qué papers del equipo son de ICDE?» — eco de la P6 del cap-41: un string no
   se expande); Temas-año → `anio` (nodo categórico de baja cardinalidad: Allen 2020,
   Boylan-Toomey 2024).
4. **Intermedios: nodos de relación con papel propio** → `Resena` como el caso canónico de
   reificación CORRECTA (la sección donde el cap-41 cobra su reto intermedio).
5. **Refactors de modelo y su coste de migración** → los tres refactors como funciones
   puras con `InformeRefactor`, la regresión de las 10 preguntas como red de seguridad, y
   el teorema didáctico: el coste de NO migrar crece con el grado del supernodo.

Hilo conductor: **«un antipatrón no es una forma fea de modelar: es una deuda que cobra
en lecturas cada vez que alguien cruza el nodo; el refactor es pagar la deuda ANTES de
que el interés sea el dataset entero»**.

---

## 4. La arquitectura: el antipatrón, su detector y su refactor

Modelo mental único: **el mapa del antipatrón en tres casillas — síntoma (grado),
diagnóstico (umbral), cura (refactor con precio) — con las 10 preguntas como red de
seguridad**. La figura que ordena todo el capítulo:

```text
LOTE IMPORTADO (24 papers)          DETECTOR (umbral 5× mediana Y 25% share)
        │                                   │
        ▼                                   ▼
  tema 24 «grafos de conocimiento»   paso-1: 6  · mediana 2,5 → ratio 2,4× → INOCENTE
  6 + 18 = 24 ABOUT entrantes        paso-2: 24 · mediana 3   → ratio 8×, 46% → ALARMA
  (el 46% de TODAS las ABOUT)        refactorizado: 12 · mediana 4 → ratio 3× → SILENCIO
```

```text
ANTES (degenerado)                               DESPUÉS (refactorizado)
                                                        (:Conferencia) 24 PUBLICADO_EN
        (:Tema) «grafos de conocimiento» ◄─ABOUT─ 24 docs     ▲   (3 conferencias)
                  ▲  (supernodo: 24 aristas)                  │
                  │                     (:Tema) 24 ──12 ABOUT directas──► docs paso-1+6
        (:Tema) «publicaciones 2024» ◄─ABOUT─ 6   │  SUB_TEMA_DE (3)  SUB_TEMA_DE (3)
        (:Tema) «publicaciones 2025» ◄─ABOUT─ 6   ▼  ▼
     (nodos categóricos de baja cardinalidad)  (:Tema) 59/60/61 ──4 ABOUT cada uno──► lote
         (:Documento) ─REVIEWED_BY{nota}─ (:Persona)
     (la reseña sin identidad: 2 rondas de Fabio        (:Persona)─REALIZA─►(:Resena{nota,ronda})
      indistinguibles por significado)                      │
         (:Documento).conferencia = "ICDE 2024"        SOBRE ▼        CONTRARRESTA
     (un string que nadie expande)                   (:Documento)◄──(:Resena r2)
```

Y debajo, la REGLA DE ORO heredada del cap. 34 (determinismo total, contadores de
TRABAJO, cero tiempos): cada refactor ES una transformación pura y medible — `refactor_*`
devuelve cuántas cosas creó, borró y leyó; el informe se pineó byte a byte; los tests
citan ids FIJOS. La frontera, declarada antes de codificar: `cap42_antipatrones.rs` es
aditivo (como caps. 38-40); ni `GraphStore` ni el executor ni `cap41_modelado.rs` se
tocan; el validador paso-2 REUTILIZA el del cap-41 en vez de reescribirlo.

```text
Lo que SÍ se hace hoy:   builder paso-2 DEGENERADO (lote real con ids fijos), detector de
                         supernodos con umbral declarado, 3 refactors puros con InformeRefactor,
                         validador paso-2, regresión de las 10 preguntas, P5 v2 con jerarquía,
                         informe de migración, CSV paso-2 + artefacto commiteado
Lo que AÚN NO:           valid-time y WAL como historia (cap. 43: la ronda es propiedad
                         simple, no fecha de validez) · constraints UNIQUE e índices (cap. 44:
                         el validador paso-2 SIEMBRA las reglas que allí serán constraints) ·
                         ingesta automatizada CSV/JSONL con dedup (cap. 45: el lote se añade
                         a mano en el builder) · RDF/tripletas y su reificación (cap. 46) ·
                         MENTIONS sigue polimórfico (no es antipatrón; la especificidad por
                         label es esquema → cap. 44)
```

Momento ¡ajá! perseguido: **«el hub del cap-41 no era inocente por pequeño: era inocente
por no cruzar el umbral. El dataset no creció por accidente — alguien importó un lote sin
preguntarle al detector; y cada expansión por ese nodo cobra el grado entero desde ese
día»**.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | El caso de estudio: `kb_lira_paso2_degrado()` PARTE de `kb_lira_paso1()` (lo llama) y AÑADE un lote de 24 papers con 18 ABOUT concentradas en el tema 24 + temas-año + `conferencia`-string + REVIEWED_BY | El objetivo medible del outline es «antes de que el dataset CREZCA»: sin crecimiento no hay degeneración que detectar; partir del paso-1 garantiza determinismo heredado y hace la degeneración HONESTA (los datos nuevos tienen origen: un lote importado, presagio del cap. 45); el lote NO toca a los autores del paso-1 (autores nuevos Gaby/Hugo/Iris) para que la regresión sea limpia | (a) Reutilizar paso-1 sin añadir nada y demostrar con métricas: el hub es inocente a 2,4× — no hay supernodo que refactorizar, el capítulo quedaría sin su tesis; (b) inflar con aristas artificiales generadas en bucle: datos sin procedencia, viola la disciplina del cap. 34; (c) importar vía pipeline real: es el contenido del cap. 45 | OUTLINE-VOL3.yml cap-42 (objetivo y paso-2); cap-41 §41.9#6 (hub sembrado a propósito); disciplina cap. 34 |
| 2 | Métrica del supernodo: umbral RELATIVO doble — ratio ≥ 5× la mediana del label **Y** share ≥ 25% de las aristas de su tipo — calculado con un barrido de `iter_edges` | El supernodo es un desequilibrio de DISTRIBUCIÓN, no un absoluto (Allen: «relative to what else is in the graph»; Boylan-Toomey: ~100k como regla práctica de OTRO tamaño de grafo); la AND defiende de falsas alarmas en grafos pequeños (mediana 1); la mediana (no la media) es robusta al propio hub; se conecta con `degree_centrality` (cap. 24) como la formalización del Vol.II de la misma medida | (a) Umbral absoluto tipo «>10.000 aristas»: no escala al fixture didáctico de KB-Lira y lo copia de un tamaño de grafo ajeno; (b) solo ratio: alarma en grafos diminutos; (c) sin umbral, «se ve a ojo»: no reproducible, prohibido por el estándar del proyecto | Allen 2020 (definición relativa, sin número formal); Boylan-Toomey 2024 (~100k, «no formal»); Aerospike AGS docs (umbral dependiente del motor: ~6.500 @1MiB — la prueba de que el absoluto es config, no ley); cap24_centralidad.rs:521 |
| 3 | Refactor A: descomponer el supernodo en SUBTEMAS intermedios con `SUB_TEMA_DE` y semántica de unión (documentos del tema = directos ∪ descendientes; P5 v2) | Es la cura canónica para un supernodo de TEMA (el outline lo sugiere literalmente: «subtemas intermedios: rust → rust/borrow-checker»); conserva la semántica (el conjunto de 24 respuestas se preserva) y reparte el grado (4+4+4+12 en vez de 24) — medible con lecturas | (a) Clones `SAME_AS` estilo «Gaga» (Allen): pensado para identidades físicas duplicadas, exótico y caro para temas; (b) time-buckets (hubs por período): es el patrón de ingesta/temporalidad → frontera caps. 43/45; (c) borrar ABOUT y dejar el tema vacío: pierde semántica, traición a P5 | OUTLINE cap-42 sección 1; Allen 2020 (toolbox: super node refactoring, label/relationship segregation) |
| 4 | Refactor B: reificar `Resena` (el reto intermedio del cap-41 se COBRA aquí como ejemplo canónico de reificación CORRECTA): `REALIZA`/`SOBRE`/`CONTRARRESTA`, `{nota, ronda}` | La reseña gana R2/R3: la ronda 2 CONTRARRESTA a la ronda 1 (relaciones propias y ciclo de vida propio) — exactamente lo que la escalera exige para el nodo intermedio; el contraste con `Autoria` (cap-41) enseña la sección 2 con la MISMA regla y dos veredictos; sin el nodo, «¿cuántas rondas?» y «¿qué reseña responde a cuál?» no tienen respuesta en el grafo | (a) Mantener `REVIEWED_BY{nota}`: las dos rondas de Fabio son paralelas indistinguibles por significado; (b) reificar también `Autoria`: reificación excesiva que el cap-41 ya descartó con la misma prueba; (c) dejar Resena como reto del lector: el outline exige «Intermedios: nodos de relación con papel propio» y el caso es EL ejemplo canónico | Robinson et al. 2015, cap. 2 «Avoiding Anti-Patterns» (p. 63): el email forense — entidad con identidad propia (CC, respuestas) codificada como relación; reto intermedio cap-41 (R1-R7: «evolución de la nota» como comprador del nodo) |
| 5 | Refactor C: conferencia:String → nodos `:Conferencia` + `PUBLICADO_EN` (propiedad→nodo) Y Temas-año → `anio` (nodo→propiedad, se borran) | Ambas direcciones del mismo peldaño: la conferencia es consultable («papers de ICDE») → necesita identidad (R1-R3); el año ya era `anio` y el Tema por año es un nodo categórico de baja cardinalidad sin relaciones ni ciclo propios (viola R1-R3) — el espejo exacto de los 176 campos de investigación de Boylan-Toomey y del `Gender` de Allen | (a) Mantener la conferencia como string: repetir la lección P6 del cap-41 (un string no se expande) a escala industrial; (b) mantener los Temas-año «por comodidad de filtrado»: el filtrado por año ya paga `anio` (P10) y el cap. 44 añadirá el índice | Allen 2020 (categorical variable overload: `(:Gender)`); Boylan-Toomey 2024 (FoR → propiedad indexada: de «casi un día» a segundos); cap-41 R1-R7 y P10 |
| 6 | Coste de migración: moneda = REESCRITURAS (put/delete) + lecturas, en `InformeRefactor` por refactor, informe agregado pineado byte a byte; SIN cronómetros | «¿Cuánto cuesta refactorizar?» exige una respuesta reproducible en CI: operaciones contadas, no µs (espejo decisión #12 cap-41); la descomposición en funciones puras hace la migración COMPONIBLE y la tesis medible: el coste crece con el grado del supernodo (12 aristas hoy vs 10M «cuando crezca») | Cronometrar los refactors: el capítulo ya declaró que la moneda son conjuntos y contadores; estimar «a mano»: no verificable | Metodología cap. 34; decisión #12 del contrato cap-41; cap41_modelado.rs (ledger de lecturas) |
| 7 | Las 10 preguntas del cap-41 como REGRESIÓN OBLIGATORIA: `las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor` + las 10 re-ejecutadas sobre paso-2 con valores pineados | El refactor es correcto si y solo si las respuestas viejas sobre los datos viejos siguen siendo las mismas; las preguntas del cap-41 reciben `&dyn GraphStore`, así que se re-ejecutan SIN COPIAR NADA (firma ya compatible); las respuestas que crecen (P3: +2 citadores; P8: +3 personas) lo hacen por el DATASET, no por el refactor — y el test lo distingue | (a) Aceptar que «las preguntas se re-validan a mano»: no es test; (b) congelar las 10 preguntas en paso-1 y no mirarlas: pierde la red de seguridad que justifica refactorizar; (c) tocar las funciones del cap-41: prohibido (caps. 7-41 intactos) | cap41_modelado.rs (firmas `pregunta_01…10(store, …)`); contrato cap-41 §3 (lista exacta vinculante); CONVENTIONS §2 (bucle de feedback inmediato) |
| 8 | `validar_modelo_kb_lira_paso2` REUTILIZA el validador del cap-41 (sobre el subgrafo paso-1) y añade las reglas nuevas; el cap-41 no se toca | El contrato del modelo evoluciona por COMPOSICIÓN: el validador viejo sigue validando lo viejo y el nuevo añade lo nuevo; es la semilla honesta del cap. 44 (las reglas de extremos que aquí son convención, allí serán constraints) | Reescribir `validar_modelo_kb_lira` in situ: rompe la regla dura (caps. 7-41 sin tocar) y sus 19 tests; duplicar la lógica copiándola: dos verdades que derivan | cap41_modelado.rs:390 (validador), :354 (Violacion); OUTLINE caps. 44 (constraints) y 47 (shapes) |
| 9 | MENTIONS polimórfico NO se refactoriza: no es antipatrón — el cap-41 lo justificó contra P6 y el validador lo suple; su especificidad por label es decisión de ESQUEMA | El capítulo enseña a detectar deudas caras, no a perseguir «limpieza»: tocar MENTIONS rompería P6 y adelantaría el contenido de constraints (cap. 44) e ingesta (cap. 45); la frontera se DECLARA como sección «qué NO refactorizar» | Separar `MENTIONS_ORG/PERSON/PROJECT`: fragmenta P6 y triplica vocabulario (el cap-41 ya descartó esa alternativa — decisión #7); refactorizar por estética | Contrato cap-41 decisión #7; OUTLINE caps. 44-45 |
| 10 | Frontera dura con caps. 43-45: la ronda es propiedad simple (NO valid-time); sin constraints/índices; el lote se añade a mano (sin pipeline); sin RDF | La temporalidad de las reseñas (cuándo valía la nota) y de las afiliaciones es EL cap. 43; la unicidad e índices son EL cap. 44; la ingesta automatizada con dedup es EL cap. 45 — adelantarlos vaciaría esos capítulos y violaría la carga cognitiva (CONVENTIONS §2) | Modelar las rondas con fechas de validez «por completitud»: pisar el cap. 43; crear constraints UNIQUE «ya que estamos»: pisar el cap. 44 | OUTLINE caps. 42-45 (secciones y pasos 2-5); CONVENTIONS §2 (una idea nueva por sección) |
| 11 | Alcance de código: UN módulo `cap42_antipatrones.rs` (~700-1100 líneas, std puro) en `crates/vol2-liradb` + wiring aditivo (2 líneas) + artefacto `datasets/kb-lira/paso-2/`; **SIN bench** | La casa enseña con código testeado y el módulo accede GRATIS a `GraphStore`, a los builders del cap-41 y al CSV del cap. 32; el paso-2 es un artefacto DERIVADO del builder (como el paso-1); ningún refactor se demuestra con cronómetro (decisión #12 espejo) | (a) Crate nueva: churn sin ganancia pedagógica (renombrar rompería rutas/goldens); (b) solo prosa/diagramas: traiciona el estándar; (c) primer bench del Vol.III: números de reloj sin hipótesis | CONVENTIONS §2 y §4; Apéndice 0 §0.5; precedente caps. 38-41; contrato cap-41 decisiones #1 y #12 |
| 12 | Citas nuevas SOLO las verificadas hoy (2026-08-26): Allen (Neo4j blog, 19-oct-2020), GraphAcademy (gdm-40), Aerospike AGS 3.0 docs «Supernodes», Boylan-Toomey (blog, 26-feb-2024), Robinson et al. 2015 (cap. 2, p. 63), Kimball 1996 (solo precisión terminológica); Neo4j KB join hints [VERIFICAR fecha]; JanusGraph/TigerGraph FUERA | El concepto «supernodo» no tiene paper seminal único: vive en guías de modelado y relatos de producción — se cita lo que se pudo verificar con venue/fecha exactos; Kimball entra SOLO para deshacer la confusión «star schema = hub» (el star schema es fact+dimensiones, otra familia) | Citar JanusGraph/TigerGraph sin verificar hoy: riesgo de URL o versión inventada; repetir Brin-Page/Brandes del cap-24 (ya citados, sin novedad) | Verificación puntual realizada hoy con fuentes primarias en vivo; contrato cap-41 (misma política de citas) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Blockquote inicial OBLIGATORIO**: SEGUNDO capítulo del Vol.III, audiencia (lector del
   cap-41 + perfil IA/datos), conexión con el cap-41 por referencias (las 10 preguntas,
   el validador, la escalera — sin re-explicar nada), gancho cobrado literalmente:
   «¿cuándo un hub deja de ser inocente?» → «cuando cruza el umbral del detector».
2. **Apertura (N.0, anécdota + pregunta crítica)**: un equipo real construyó un KG
   académico de 100 millones de publicaciones y sus consultas pasaron de segundos a
   «casi un día»: 176 nodos de «campo de investigación» habían crecido hasta ser
   supernodos (Boylan-Toomey, 2024). Pregunta enmarcada: tu KB-Lira tiene 30 nodos y
   alguien acaba de importar 24 papers más — ¿sabes si ya hay un supernodo EN TU GRAFO,
   antes de que «crezca»?
3. **N.1-N.2 Objetivo/Problema**: objetivo medible del outline («detectar y refactors de
   los errores de modelado más caros ANTES de que el dataset crezca»). Problema: la suite
   sigue verde (920) y el grafo ya cobra — el lote concentró el 46% de las ABOUT en un
   tema que nadie eligió. Desactivar las cinco misconcepciones ANTES de dibujar.
4. **N.3 Modelo mental**: el mapa síntoma→diagnóstico→cura→precio + el umbral (figura de
   §4) + el triplete inocente/culpable/silencioso con los números del contrato.
5. **N.4 Primera solución**: la NO-solución — importar el lote sin mirar (el builder
   degenerado) y «seguir como siempre». El detector, construido ANTES del refactor, avisa.
6. **N.5 Sus límites**: cada expansión desde el supernodo paga el grado entero (24
   lecturas en P5 inversa); los Temas-año y la conferencia-string son preguntas nuevas
   que no responden; la reseña de dos rondas es indistinguible.
7. **N.6 Solución evolucionada**: los tres refactors (A descomponer, B reificar, C
   conferencias/temas-año) como transformaciones puras con `InformeRefactor`, el
   validador paso-2 por composición, y la regla de oro: NADA de lo que respondían las 10
   preguntas sobre el paso-1 puede cambiar.
8. **N.7 Código completo ejecutable**: `cap42_antipatrones.rs` por `include::` (nunca
   duplicado); SIN bench (decisión #11 explicada en una línea).
9. **N.8 Prueba de fuego**: salidas REALES de `cargo test`: el triplete del detector, la
   regresión de las 10 preguntas, el informe de migración pineado byte a byte, el CSV
   paso-2 byte a byte, `csv_paso1_intacto_tras_paso2`.
10. **N.9 Qué hemos sacrificado**: sin valid-time (la ronda es propiedad simple; cuándo
    valía algo es el cap. 43); sin constraints UNIQUE ni índices (el validador paso-2 es
    su semilla — cap. 44); sin ingesta automatizada (el lote se añadió a mano — cap. 45);
    sin RDF (cap. 46); MENTIONS sigue polimórfico (no es antipatrón).
11. **N.10 Cómo lo hace una BBDD real + retos**: Allen (2020) — toolbox industrial
    (direccionalidad, join hints, segregación de labels/aristas, refactoring del
    supernodo); Neo4j Knowledge Base (join hints contra supernodos) [VERIFICAR fecha];
    Aerospike AGS — el supernodo como ciudadano de PRIMERA CLASE (flag `~supernode`,
    listas de aristas multi-registro); el caso Enron de Robinson et al. (2015) como el
    espejo de Resena; precisión terminológica: el star schema de Kimball (1996) NO es un
    hub-and-spoke. Retos: esencial (añadir UN paper al lote y PREDECIR el nuevo ratio y
    si el detector alarma, ANTES de correr), intermedio (aplicar `detectar_supernodos` a
    otro label — p.ej. Persona — y justificar un umbral propio por escrito), experto
    (migrar el supernodo estilo «Gaga»: clones con `SAME_AS` y medir reescrituras vs
    subtemas — ¿cuándo gana cada cura?).
12. **Baterías finales + gancho**: Lo que te llevas / Ojo cuidado / Pin / 30 segundos /
    historia pequeña / Mini-diálogo de guardia nocturna (la consulta «todos los papers
    del tema» que expande el supernodo mientras el detector calla). Retrieval practice:
    reproducir DE MEMORIA el umbral del detector y clasificar 5 situaciones (hub
    legítimo, supernodo, reificación justa/excesiva/insuficiente, nodo categórico) sin
    mirar. Interleaving: cada reto toca ≥2 capítulos (34+41+42, 24/25+42, 39/40+42).
    Glosario nuevo: supernodo (antipatrón de modelado — DISTINTO del supernodo de
    condensación del Vol.II cap. 5 y del de contracción del cap. 25), hub legítimo,
    umbral relativo, refactor de modelo, coste de migración (reescrituras), nodo
    intermedio, nodo categórico de baja cardinalidad. Gancho al cap. 43: «la ronda 2
    contrarresta a la ronda 1 — pero ¿QUÉ valía el 3 de marzo?». Abiertas: unicidad e
    índices (44), ingesta (45).

---

## 7. Estilo y tono (consistencia con el proyecto)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (supernode, hub, reification, refactor, migration cost,
  star schema); salidas REALES de `cargo test` pegadas, nunca reconstruidas; los
  refactors se presentan como TRADE-OFF con precio en reescrituras, nunca como dogma.
- **Diagramas**: el mapa síntoma→diagnóstico→cura→precio con el umbral (§4); el grafo
  degenerado vs el refactorizado lado a lado; la tabla del informe de migración
  (refactor → ops → impacto); el triplete inocente/culpable/silencioso como figura
  recurrente de las baterías.
- **Spacing** (conceptos viejos que se EJERCITAN): escalera R1-R7 y las 10 preguntas
  (cap. 41 — la regresión ES el ejercicio), GraphStore/travesías (caps. 7-8, 20),
  degree_centrality (cap. 24), Louvain (cap. 25), dataset determinista (cap. 34), skew
  del hub en joins y particionado (caps. 39-40: ×521 y réplicas del hub), formato CSV
  (cap. 32), BFS mental (Vol. I, caps. 3-4).
- **Interleaving**: reto esencial mezcla 34+41+42 (predicción sobre dataset determinista
  degenerado); el intermedio mezcla 24/25+42 (la misma distribución de grados vista
  desde la centralidad y la modularidad); el experto mezcla 39/40+42 (el skew del join y
  del vertex-cut como la MISMA enfermedad en otra divisa).
- **Dificultad asimétrica**: una idea nueva por sección (supernodo/umbral → reificación
  díptico → propiedades↔nodos → intermedios → coste de migración); los ejercicios exigen
  PREDECIR ratios y alarma del detector ANTES de correr los tests.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap42` (reescrituras, lecturas
  y conjuntos exactos) y `./scripts/verify.sh` ALL_GREEN como puerta. Nunca «confía en mí».
- **Anécdota (única, verificada)**: el KG académico de 100M publicaciones × 176 campos
  (Boylan-Toomey, 2024) — el supernodo nace cuando los datos superan al modelo, y la cura
  (campo → propiedad indexada) va de «casi un día» a segundos. Fuentes para la prosa:
  Allen (Neo4j blog, 19-oct-2020); Neo4j GraphAcademy (gdm-40); Aerospike AGS 3.0 docs
  «Supernodes»; Boylan-Toomey (26-feb-2024); Robinson et al. (O'Reilly 2015, cap. 2,
  p. 63); Kimball (Wiley, 1996); Neo4j KB join hints [VERIFICAR fecha].

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap42_antipatrones; pub
  use cap42_antipatrones::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al
  final, con el módulo ya compilando limpio (`cargo check -p vol2-liradb`); jamás dejar
  `lib.rs` apuntando a un módulo rojo. Los ids del lote (30-32 personas, 33-56 papers,
  57-58 temas-año) deben validarse contra los ids del paso-1 (0-29): NO reutilizar un id
  existente — el test de estructura lo cazaría.
- **Orden de implementación recomendado** (cada paso compila y testea solo): (1)
  `kb_lira_paso2_degrado()` + test de estructura; (2) `detectar_supernodos` +
  distribución + el triplete inocente/culpable/silencioso; (3) refactor A (subtemas) +
  preservación del conjunto de P5; (4) refactor B (Resena) + rondas/contrarrestas; (5)
  refactor C (conferencias + temas-año); (6) `validar_modelo_kb_lira_paso2` + fixture
  corrupto; (7) regresión de las 10 preguntas (ANTES verificar que las firmas
  `pregunta_01…10` aceptan `&dyn GraphStore` tal cual — son las del cap-41, sin tocar);
  (8) informe de migración + conteo de reescrituras; (9) CSV paso-2 + round-trip + generar
  y commitear `datasets/kb-lira/paso-2/`; (10) wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap42` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap42_antipatrones.rs`
  y continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). PROHIBIDO
  tocar `cap41_modelado.rs`, el parser de LiraQL o el trait `GraphStore`: el refactor se
  hace CON la API existente (put/delete/iter), no cambiándola.
- **Criterio de parada honesto**: si un refactor cambia la respuesta de alguna de las 10
  preguntas sobre el subgrafo paso-1, el refactor está MAL y se rediseña DENTRO del
  capítulo (esa es la lección: la red de seguridad existe para eso), prohibido «ajustar»
  la pregunta o el dataset. Igual si el detector no alarma en paso-2 o alarma en paso-1:
  se explican los números POR QUÉ (¿share sin ratio? ¿ratio sin share?), prohibido inflar
  umbrales para «que suene mejor».

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5; citas verificadas 2026-08-26: Allen 2020, GraphAcademy gdm-40,
  Aerospike AGS 3.0, Boylan-Toomey 2024, Robinson et al. O'Reilly 2015 cap. 2 p. 63,
  Kimball Wiley 1996; Neo4j KB join hints marcado [VERIFICAR]; JanusGraph/TigerGraph
  descartados por no verificables hoy).
- [ ] Escenario de fallo visible, no solo happy path: el hub inocente del paso-1 (2,4×),
  el detector que alarma con el lote (8×, 46%), el fixture corrupto del validador
  paso-2, la expansión que paga el grado entero, la reseña de dos rondas indistinguible
  sin el nodo.
- [ ] Código ejecutable citado por nombre (`cap42_antipatrones.rs`, wiring, artefacto
  `datasets/kb-lira/paso-2/`, SIN `[[bench]]`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: cinco).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa como patrón).
- [ ] ≥1 ejercicio de retrieval (el umbral del detector DE MEMORIA + clasificación de 5
  situaciones) y spacing planificado (caps. 41/7-8/20/24/25/32/34/39-40 + Vol.I 3-4; §7).
- [ ] Responde las TRES preguntas críticas del capítulo (detección objetiva = umbral
  doble con triplete de tests; coste de migración = reescrituras + informe pineado;
  reificación excesiva vs insuficiente = díptico Autoria/Resena con la MISMA escalera) y
  cobra el gancho del cap-41 («¿cuándo un hub deja de ser inocente?» → cuando cruza el
  umbral: triplete 2,4× / 8× / 3×).
- [ ] Las 10 preguntas del cap-41 como REGRESIÓN OBLIGATORIA con test de nombre exacto
  (`las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor`).
- [ ] Anécdota única verificada con fuentes primarias (Boylan-Toomey 2024; Allen 2020).
- [ ] Alcance acotado y honesto (UN módulo + wiring + artefacto paso-2; cero deps, cero
  benches, cero cambios caps. 7-41; frontera dura con caps. 43-46 declarada).
- [ ] Blockquote inicial declara SEGUNDO CAPÍTULO DEL VOL.III (audiencia + conexión con
  cap-41 por referencias) y gancho cobrado literalmente (el hub inocente vs el umbral);
  gancho saliente fijado (cap. 43: «¿QUÉ valía el 3 de marzo?»).
