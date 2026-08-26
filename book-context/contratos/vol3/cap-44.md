# CONTRATO DE CAPÍTULO — Vol.III Cap. 44: Esquema, constraints e índices en property graphs

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **CUARTO CAPÍTULO DEL VOL.III**
> («Grafos en la era de la IA: modelar, razonar y recuperar»), Parte I «Modelar datos de
> grafos». Audiencia declarada en el blockquote inicial: el lector que terminó el cap. 43 —
> o el perfil datos/IA que lo haya leído en diagonal — y que ya tiene la escalera R1-R7, los
> validadores por composición (41→42→43), la regresión como red de seguridad y la moneda de
> lecturas como HERRAMIENTAS, no como contenido. COBRA los ganchos salientes del cap. 43
> (§43.9 y cierre): (1) «¿quién construye el índice que abarata el AS OF?» — este capítulo
> responde CON DATOS: el AS OF persona-céntrica de Kira baja de **28 a 16 lecturas** con un
> índice de adyacencia por etiqueta, y la consulta global «¿quién estaba afiliado en el año
> Y?» baja de **168 a 18 lecturas** con un índice sobre `desde_anio`; (2) «¿quién garantiza
> que dos afiliaciones no se solapen?» — la **regla de no solapamiento** (`ReglaSinSolape`)
> del esquema declarativo, la unicidad temporal que el cap-43 declaró deuda. Y cobra un
> gancho ATRASADO del cap-42 (§42.5): «la especificidad por label es decisión de ESQUEMA
> (cap. 44)» — el esquema decide que `MENTIONS` sigue polimórfico, con la regresión como
> evidencia (P6 no cambia). Progresión del hilo conductor KB-Lira: paso-1 (cap. 41) =
> modelo sano; paso-2 (cap. 42) = refactors; paso-3 (cap. 43) = valid-time con AS OF a 28
> lecturas; paso-4 (ESTE capítulo) = **ORCID único, tipos por etiqueta, índice sobre año**:
> el esquema como DATO sobre el modelo temporal, los índices lógicos que cambian la factura
> del cap-43, y la regla que el validador paso-3 SIEMBRA (las reglas de los validadores
> 1-3, promovidas a constraints declarativas). Código ancla VERIFICADO hoy (2026-08-26):
> `kb_lira_paso3()` (cap43_temporalidad.rs:181) construye **68 nodos / 158 aristas** (test
> `estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas` pine 68/158 — el «69 nodos» del
> contrato cap-43 era un typo, el nodo 69 es Instituto GrafoLuna), 10 `MEMBER_OF` con
> `desde_anio` (53 Beto→Neurónica con `hasta_anio:2024`, 185 Beto→GrafoLuna abierta);
> `afiliaciones_vigentes_en` (cap43:408) paga **28 = 28 lecturas** (1 `in_edges` + 21
> `get_edge` + 6 `get_node`: 5 entrantes al proyecto + 16 salientes de Ana/Beto/Dani, de
> las que solo 4 son `MEMBER_OF` — el desperdicio medible que el índice ataca); el validador
> paso-3 (cap43:1077) REUTILIZA paso-2 y siembra: `desde_anio:Int` requerido, `hasta_anio`
> opcional `Int` con `hasta >= desde`, `desde <= ANIO_ACTUAL` — ESTAS reglas son las que
> aquí se declaran; `Violacion` es PUBLICA (cap41_modelado.rs:354) y se REUTILIZA tal cual;
> el catálogo del cap-21 (`Catalog::collect` + `selectivity` + `EqIndexEntry{label,
> property, value, ids}` en cap21_optimizador.rs:108) modela el índice de igualdad QUE NO
> CONSTRUYÓ — el cap-44 lo construye; `BPlusTree`/`HashIndex` del cap-15 son FÍSICOS
> (persisten en páginas del BufferPool, claves u64→u64, requieren Pager) — se NOMBRAN como
> hermanos persistentes, no se reutilizan (justificación en §5 #4); `GraphStore` tiene
> `iter_edges`/`iter_nodes` (usados por los validadores) y `MemoryStore.edges` es campo
> `pub` (fixtures corruptos por mutación directa, precedente cap-33/43). Ninguna prop
> `orcid` existe aún en el workspace (grep: cero) — se introduce aquí. Estado: **971 tests
> ALL_GREEN** (cap-43), toolchain 1.96.0, runtime dependency-free. Código NUEVO previsto:
> UN módulo `src/cap44_esquema.rs` (~800-1200 líneas, std puro) + wiring ADITIVO en
> `lib.rs` (2 líneas) + artefacto regenerable
> `liradb-workspace/datasets/kb-lira/paso-4/{nodes.csv,edges.csv,esquema.csv}` — CERO deps
> nuevas, CERO cambios en caps. 7-43, **SIN bench** (espejo de las decisiones #12 cap-41 y
> #11 cap-43: la moneda son lecturas, conjuntos y violaciones exactos, no µs). Citas
> VERIFICADAS hoy (2026-08-26, venue/fecha exactos): ISO/IEC **39075:2024** GQL, publicado
> **abril 2024** (iso.org/standard/76120) — la Parte 1 INCLUYE DDL de esquema: `CREATE
> PROPERTY GRAPH`, graph types con node/edge types y property value types (`NOT NULL`),
> constraints (`CONSTRAINT … FOR … REQUIRE … IS KEY / IS UNIQUE / IS NOT NULL`) y `CREATE
> INDEX` (confirmado vía preview de la norma en iTeh, la tabla de conformidad de Microsoft
> Fabric y el reference de Geode; [VERIFICAR el subclausulado fino contra el texto de la
> norma antes de citarlo en prosa]); **Neo4j** Cypher Manual actual (5.x): constraints de
> unicidad (`IS UNIQUE`), existencia (`IS NOT NULL`), key constraints y **property type
> constraints (Neo4j 5** — blog oficial «Enforcing data quality in Neo4j 5: Property
> constraints», 3 nov 2023), índices range/text/point/fulltext, y los constraints de
> unicidad respaldados por índices; Baron Schwartz, **«Schemaless Databases Don't Exist»**,
> **blog de SolarWinds, 24 febrero 2015** — «there is always a schema somewhere. Usually in
> multiple places»: la cita del «schemaless mentiroso» (§1 del outline). SQL:2011/SQL:2023:
> referencia clásica de constraints UNIQUE/NOT NULL — ya conocida, citar con cuidado
> [VERIFICAR el detalle fino]. JanusGraph/TigerGraph: docs NO verificadas hoy → fuera del
> cuerpo (espejo caps. 42-43). Gancho saliente: cap. 45 (ingesta — el esquema SIRVE a la
> validación al importar, write-time), cap. 46 (RDF — sin SHACL aquí), cap. 47 (shapes
> SHACL: el hermano declarativo del mundo RDF), cap. 53 (el índice de intervalos como
> memoria temporal del agente).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la escalera R1-R7 y las 10 preguntas con sus respuestas y
  costes (cap. 41); el validador por composición (`validar_modelo_kb_lira` → paso-2 →
  paso-3) y la regresión como red de seguridad (caps. 41-43); valid-time en aristas
  (`desde_anio`/`hasta_anio`, medio abierto `[desde, hasta)`), AS OF con coste en lecturas
  (28 = 28) y el Historico bitemporal (cap. 43); los ids FIJOS (nodos hasta 69, aristas
  hasta 185) y las 10 `MEMBER_OF` con sus intervalos; el catálogo del optimizador con
  estadísticas por etiqueta y el índice de igualdad MODELADO (`EqIndexEntry`) (cap. 21);
  los índices FÍSICOS `HashIndex`/`BPlusTree` sobre BufferPool con `range_scan` (cap. 15) y
  el CSR (cap. 14); el formato CSV del cap. 32 con round-trip byte a byte; `Value` y sus
  tipos (cap. 7); la disciplina de contadores y el determinismo total (caps. 26, 34);
  `MemoryStore.edges` mutable en tests (fixtures corruptos, precedente cap-33/43). Perfil
  IA/datos: entra por el prólogo con estas piezas como prerrequisito declarado.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «los grafos son schemaless: no hacen falta esquemas» — NO: el esquema SIEMPRE
  existe; si no está en la BBDD, está en el código o en los datos rotos (Schwartz 2015:
  «there is always a schema somewhere»); los validadores de los caps. 41-43 eran EL
  esquema del proyecto disfrazado de código.
  (2) «un índice es para hacer más rápidas las consultas: no cambia nada más» — NO: el
  índice cambia el COSTE, nunca la respuesta; y un constraint cambia QUÉ datos pueden
  existir. Confundir «constraint» con «índice» es el error central del capítulo: el
  constraint UNIQUE es la garantía; el índice solo la hace barata (y en las BBDD reales,
  el UNIQUE crea su índice — Neo4j lo documenta).
  (3) «si añado un índice, el AS OF del cap-43 se abarata solo» — NO: el índice debe CASAR
  con la forma de la consulta; el índice global sobre `desde_anio` NO toca la consulta
  persona-céntrica (28 = 28 con y sin él) — solo la consulta reescrita para usarlo se
  beneficia. Un índice que no casa con ninguna consulta sigue cobrando mantenimiento.
  (4) «un índice sobre una propiedad sirve siempre» — NO: la selectividad lo decide (cap.
  21: `anio` de Documento tiene 5 valores en 12 nodos: ningún filtro se beneficia); el
  catálogo del cap-21 es la herramienta para decidir ANTES de construir.
  (5) «la unicidad es de una propiedad, punto» — NO: «una persona, una organización» NO
  es la regla de KB-Lira; lo es «una persona, una organización, EN UN MISMO INTERVALO»
  (Beto en Neurónica y GrafoLuna son legítimos en años distintos): la unicidad temporal es
  de INTERVALOS, no de pares (el gancho «¿quién garantiza que dos afiliaciones no se
  solapen?» del cap-43).
  (6) «los validadores 41-43 eran el esqueleto; ahora hay que reescribirlos como esquema» —
  NO: el esquema es la MISMA regla como DATO; los validadores se quedan intactos (regresión
  dura), y el test de equivalencia demuestra que el esquema los subsume sin tocarlos.
- **No debe saber todavía**: validación al escribir (write-time) dentro de la ingesta
  (cap. 45 — aquí la verificación es una capa aparte, función sobre el store); SHACL y
  shapes (cap. 47 — el esquema de aquí es el hermano imperativo/declarativo del mundo LPG;
  SHACL se NOMBRA como gancho, no se explica); RDF y quads (cap. 46); la UNIQUE temporal
  con índice de intervalos REAL (cap. 53 — aquí se formula la regla y se nombra el
  «interval tree» como estructura mental, sin implementarla); integración del índice con
  el optimizador del cap-21 (el cap-44 construye índices LÓGICOS sobre el MemoryStore; el
  coste en el plan de consulta sigue siendo del cap-21). El paso-4 NO pisa esas fronteras.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| **Esquema como DATO** `Esquema` = `Vec<ReglaConstraint>` con `enum ReglaConstraint {Extremos{label, origen, destino}, Existencia{label, propiedad}, Tipo{label, propiedad, tipo}, Unicidad{label, propiedad}, SinSolape{label, desde_prop, hasta_prop, por_prop}}` + `TipoEsperado` (Int/String/Bool). `esquema_kb_lira_paso4() -> Esquema`: 6 `Extremos` (AUTHORED Persona→Documento, CITES Documento→Documento, ABOUT Documento→Tema, MENTIONS Documento→[Persona\|Organizacion\|Proyecto] — el gancho del cap-42 cobrado: la especificidad por label es DECISIÓN de esquema, y se decide NO refinar el polimorfismo (P6 como evidencia, §5 #6); MEMBER_OF Persona→Organizacion, WORKED_ON Persona→Proyecto), 3 `Existencia` (Documento.titulo, Documento.anio, MEMBER_OF.desde_anio — la siembra del paso-3), 3 `Tipo` (Documento.anio:Int, MEMBER_OF.desde_anio:Int, Persona.orcid:String), 1 `Unicidad` (Persona.orcid), 1 `SinSolape` (MEMBER_OF por persona sobre [desde_anio, hasta_anio)) | `el_esquema_paso4_declara_las_reglas_exactas` (cuenta por variante: 6/3/3/1/1 con sus labels y propiedades, nombres exactos) |
| **Motor** `verificar_esquema(store: &dyn GraphStore, esquema: &Esquema) -> Result<(), Vec<Violacion>>` — REUTILIZA `Violacion` del cap-41 (cap41_modelado.rs:354) tal cual; devuelve la lista COMPLETA con ids (patrón validadores); la regla `Unicidad` se verifica con un índice de unicidad interno (BTreeMap<valor, Vec<id>>) — el capítulo enseña que UNIQUE ≡ índice; `SinSolape` compara por pares los intervalos de cada persona (regla `[desde, hasta)` medio abierto, ausencia = abierto, la convención de Snodgrass heredada del cap-43) | `verificar_esquema_acepta_el_modelo_paso4` (Ok sobre `kb_lira_paso4()`), `verificar_esquema_rechaza_fixture_corrupto` (3 violaciones con ids sobre un fixture roto A MANO) |
| **Builder paso-4** `kb_lira_paso4() -> MemoryStore`: `kb_lira_paso3()` + `aplicar_orcid(&mut store) -> InformeOrcid`: las **9 personas** (ids 0-5, 30-32) reciben `orcid:String` con formato `0000-0000-0000-0000` determinista por id (Ana=0000-0002-0000-0001, …). SIN nodos ni aristas nuevos: **68 nodos, 158 aristas** (los pines del paso-3 intactos) | `estructura_de_kb_lira_paso4_cuenta_y_etiquetas_exactas` (68/158, 10 MEMBER_OF, 9 personas con `orcid:String`) y `las_9_personas_llevan_orcid_determinista` (valores pineados) |
| **ORCID único, dos reglas distintas** — el fixture demuestra existencia ≠ unicidad: (a) insertar una persona con el orcid de Ana → violación `Unicidad` con el id del DUPLICADO; (b) insertar una persona SIN orcid → violación `Existencia` (no de unicidad) | `orcid_duplicado_es_violacion_de_unicidad_con_id`, `persona_sin_orcid_es_violacion_de_existencia` (mutación directa de `s.edges`/`put_node` sobre el store, precedente cap-33) |
| **Equivalencia esquema ↔ validadores** — el test que cierra la decisión §5 #2: sobre el MISMO fixture roto, `verificar_esquema(esquema_paso4)` y la cadena `validar_modelo_kb_lira_paso3` producen el MISMO conjunto de violaciones para las reglas compartidas (extremos/existencia/tipo), y el esquema añade solo las suyas (unicidad, sin-solape) | `el_esquema_reproduce_las_violaciones_de_la_cadena_de_validadores` (comparación de conjuntos de `(id_implicado, descripcion)`); `verificar_esquema_es_la_puerta_canonica_del_paso4` |
| **Índice sobre año (lógico, de modelo)** `IndiceDesdeAnio`: `BTreeMap<i64, Vec<usize>>` sobre `MEMBER_OF.desde_anio`; `construir(store, label, propiedad) -> IndiceDesdeAnio` (coste = nº de aristas leídas), `rango(hasta: i64) -> Vec<usize>` (candidatas `desde <= anio`), `insertar(edge_id, anio)` (mantenimiento). Es la versión «de modelo» del `BPlusTree.range_scan(lo, hi)` del cap-15 — se NOMBRA el hermano físico, no se reutiliza (§5 #4) | `indice_desde_anio_cubre_igualdad_rango_y_mantenimiento` (unitario: construcción sobre el paso-4 = 10 lecturas, `rango(2023)` = 9 candidatas con ids exactos, insertar una MEMBER_OF nueva la añade) |
| **El AS OF global y su factura** `afiliaciones_vigentes_global_en(store, anio) -> (Vec<(String,String)>, CosteLecturas)`: «¿quién estaba afiliado a algo en el año Y?» — la consulta cuya forma CASARÍA con el índice de año. SIN índice: barrido completo (158 `get_edge` + `get_node` por extremo) ≈ **168 lecturas**; CON `IndiceDesdeAnio`: candidatas `desde <= anio` + 1 `get_edge` por candidata para el filtro de `hasta` + `get_node` de organizaciones ≈ **18 lecturas** (2026: 10 candidatas; 2023: 9). La frontera del intervalo, EXPLÍCITA: el índice simple poda por un lado; el `[desde, hasta)` cobra el otro (1 lectura por candidata, vencidas incluidas) — la lección de la sección 3 | `el_indice_de_anio_abarat_a_la_consulta_global` (168 → 18, pines por año: 2026 y 2023) y `el_indice_de_anio_no_abarat_la_consulta_por_proyecto` (**28 = 28** con y sin `IndiceDesdeAnio` — el desajuste índice-consulta, pineado, ES la lección) |
| **El índice que sí abarata el cap-43** `IndicePorLabel`: adyacencia por etiqueta `{label: String, por_origen: BTreeMap<usize, Vec<usize>>}` — la versión lógica del CSR del cap-14; `construir(store, label)`, `salientes(origen) -> Vec<usize>`. Consulta reescrita: el segundo salto del AS OF de Kira usa `salientes(persona)` de MEMBER_OF en vez de barrer `out_edges` | `el_indice_por_etiqueta_abarat_el_as_of_por_proyecto` (**28 → 16** lecturas para 2026 Y 2023: 1 `in_edges` + 5 `get_edge` + 3 `get_node` + 4 `get_edge` + 3 `get_node`; la vencida 53 sigue costando 1 lectura: el índice elimina las lecturas que NO eran historia, la factura de la historia queda en sus términos justos) |
| **Cuándo estorba (sección 3 del outline)** — dos casos medibles: (a) selectividad: con `Catalog::collect` del cap-21 se calcula que `Documento.anio` tiene 5 valores distintos en 12 nodos (selectividad ≈ 0.42 global; filtro `anio > 2022` ≈ 0.83) → un `IndiceAnioDocumento` NO reduce lecturas de P10 y se DECIDE no construirlo — el catálogo del cap-21 como herramienta de decisión; (b) mantenimiento sin uso: `IndiceDesdeAnio` se construye (10 lecturas) para la consulta persona-céntrica y NO se usa (28 = 28): el capítulo mide el coste de mantener un índice que no casa con ninguna consulta (1 insert por MEMBER_OF nueva) | `la_selectividad_del_cap21_decide_cuando_un_indice_no_merece_la_pena` (0.42/0.83 pineados vía catálogo; P10 lee lo mismo con y sin índice), `el_indice_que_no_casa_con_la_consulta_cuesta_mantenimiento` (construcción 10 lecturas + insert por escritura; cero ahorro) |
| **Regresión OBLIGATORIA (red de seguridad triple + esquema)** | `las_10_preguntas_del_paso1_no_cambian_con_esquema` (las 10 del cap-41 sobre el subgrafo paso-1 del store paso-4: IDÉNTICAS — `orcid` no filtra nada), `las_respuestas_del_paso2_no_cambian_con_esquema` (valores pineados del contrato cap-42), `validador_paso3_acepta_el_modelo_paso4` (el paso-4 cumple el contrato temporal: `orcid` no rompe las reglas del paso-3), `las_member_of_del_paso4_no_se_solapan` (la regla SinSolape pasa sobre el dato limpio: Beto [2018,2024) y [2024,∞) no se solapan — el test del gancho cobrado) |
| CSV determinista paso-4: `nodes.csv` (formato cap. 32 con columna `orcid:STRING` para personas), `edges.csv` (byte a byte el del paso-3 — los índices y el esquema NO viven en el grafo, se regeneran), `esquema.csv` (formato propio mínimo `tipo_regla,label,propiedad,valor[,extra]`, exportador propio + round-trip — el esquema es DATO y se serializa) + artefacto `datasets/kb-lira/paso-4/` commiteado | `csv_paso4_roundtrip_byte_a_byte`, `csv_paso4_coincide_con_dataset_commiteado_byte_a_byte`, `csv_esquema_roundtrip_byte_a_byte`, `csv_paso1_2_3_intactos_tras_paso4` |
| Informe reproducible `informe_esquema_reproducible`: tabla consulta → sin índice → con índice → ahorro (28/16, 168/18, 28/28), violaciones del fixture (unicidad/existencia/solape con ids), selectividades del catálogo, SIN tiempos | `informe_esquema_reproducible_sobre_kb_lira` (pineado byte a byte; la tabla de la prosa) |
| **SIN `[[bench]]` nuevo** (decisión espejo #12 cap-41 y #11 cap-43) | `verify.sh` compila `--all-targets` igual; prosa pega salidas de `cargo test` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**971 + ~26 tests nuevos ≈ 997**); cero cambios en caps. 7-43, goldens intactos, pasos 1-3 byte a byte |

## 3. Las preguntas críticas del capítulo y la respuesta del capítulo

**Preguntas** (propias del capítulo): (1) ¿Esquema abierto o estricto — y por qué «schemaless» es un mito? (2) ¿Qué es un constraint, qué es un índice, y por qué confundirlos cuesta caro? (3) ¿Cuándo un índice ayuda de verdad, cuándo es decoración con factura de mantenimiento?

Respuestas medibles:

1. **Esquema abierto vs estricto**: el «schemaless» miente por omisión — el esquema siempre está en algún sitio (Schwartz 2015: «there is always a schema somewhere»): en los validadores del proyecto (caps. 41-43), en el código de la app, o en los datos rotos. El capítulo demuestra que KB-Lira YA tenía esquema (las reglas de `validar_modelo_kb_lira` → paso-2 → paso-3, sembradas capítulo a capítulo) y lo hace DATO: `Esquema` + `verificar_esquema`. La elección abierto/estricto no es binaria: es DECIDIR QUÉ GARANTIZAS (existencia/tipo/unicidad) y QUÉ PERMITES (props libres, labels múltiples, aristas polimórficas). El fixture lo demuestra: un documento sin `anio` o un orcid duplicado ENTRAN en un grafo abierto y ROMPEN preguntas viejas (P10 silenciosa; dos Anas).
2. **Constraint ≠ índice**: el constraint es una regla sobre QUÉ datos pueden existir (si falla, el grafo NO responde: violación con id); el índice es una estructura auxiliar sobre CUÁNTO cuesta encontrarlos (si existe o no, la RESPUESTA es la misma — solo cambia el ledger). La confusión se deshace con los dos ledgers: `verificar_esquema` rechaza el duplicado (constraint), y el AS OF baja de 28 a 16 lecturas (índice) SIN cambiar una sola fila de respuesta. En las BBDD reales el UNIQUE crea su índice (Neo4j lo documenta): el capítulo enseña la unión con el índice de unicidad interno de `verificar_esquema`.
3. **Cuándo ayuda y cuándo estorba**: ayuda cuando la consulta se puede REESCRIBIR para usarlo y la selectividad es baja (el AS OF global 168 → 18 con `IndiceDesdeAnio`; el AS OF por proyecto 28 → 16 con `IndicePorLabel`); estorba cuando no casa con la consulta (28 = 28 con el índice de año en la consulta persona-céntrica — se construye, se mide que no se usa, y se paga el mantenimiento) o cuando la selectividad lo condena (el catálogo del cap-21 dice 0.83 para `anio > 2022`: el optimizador del cap-21 NUNCA elegiría ese índice — la decisión se toma con el catálogo, ANTES de construir). Y la frontera del intervalo: el índice de clave simple poda por un lado; el `[desde, hasta)` cobra el otro — de ahí que la vencida 53 siga costando 1 lectura incluso con índice; el índice de intervalos («interval tree» mental) se NOMBRA, no se implementa.

Escalera del brief (5 secciones del outline → 5 peldaños):

1. **Esquema abierto ('schemaless' mentiroso) vs estricto** → Schwartz 2015; los validadores 41-43 como esquema YA EXISTENTE disfrazado de código; `Esquema` como dato; la tabla «qué garantizas / qué permites» (abierto vs estricto con KB-Lira como caso).
2. **Constraints de existencia, tipo y unicidad** → `ReglaExistencia/Tipo/Unicidad` + la siembra del paso-3 (las reglas temporales del validador paso-3 se DECLARAN); el caso ORCID (duplicado → rechazo con id); la unicidad temporal `ReglaSinSolape` (el gancho del cap-43: «dos afiliaciones no se solapan»).
3. **Índices sobre propiedades: cuándo ayudan y cuándo estorban** → `IndiceDesdeAnio` (168 → 18 en la global; 28 = 28 en la por proyecto — el desajuste ES contenido), `IndicePorLabel` (28 → 16), la selectividad del cap-21 como juez (0.83 → no construir), el coste de mantenimiento del índice que no se usa.
4. **Etiquetas múltiples y su impacto** → las reglas aplican a todo nodo que TENGA la label (`:Paper` hereda las reglas de `:Documento`); `MENTIONS` polimórfico resuelto por DECISIÓN de esquema (cap-42 cobrado); el catálogo del cap-21 agrupa por label — las estadísticas de `:Paper` vs `:Documento`.
5. **El DDL de GQL como referencia industrial** → ISO/IEC 39075:2024: `CREATE PROPERTY GRAPH`, graph types (node/edge types, property value types `NOT NULL`), `CONSTRAINT … REQUIRE … IS KEY / IS UNIQUE / IS NOT NULL`, `CREATE INDEX` — la forma industrial de lo que el capítulo construyó a mano, con la frontera [VERIFICAR subclausulado fino].

Hilo conductor: **«los capítulos 41-43 construyeron el esquema de KB-Lira sin saberlo: cada validador sembró una regla. Este capítulo la saca del código y la convierte en DATO — y de paso construye los atajos que hacen que las reglas y las consultas no cuesten caras. El grafo deja de ser un cajón donde todo cabe: tiene contrato, y el contrato se puede leer, serializar y discutir»**.

---

## 4. La arquitectura: el contrato y el atajo

Modelo mental único: **el esquema es el CONTRATO del grafo (qué datos pueden existir); el índice es el ATAJO (cuánto cuesta encontrarlos). Un constraint y un índice son dos herramientas que la gente confunde porque en las BBDD reales viajan juntas — el UNIQUE crea su índice** (Neo4j lo documenta). La figura que ordena todo el capítulo:

```text
DOS PREGUNTAS DISTINTAS (la confusión que este capítulo deshace):
  CONSTRAINT  «¿PUEDE existir este dato?»   → si NO: violación con id, el grafo NO responde
              orcid duplicado · Documento sin anio · MEMBER_OF sin desde_anio
              · dos afiliaciones de Beto solapadas en [2018, 2024)
  ÍNDICE      «¿CUÁNTO CUESTA encontrarlo?» → si existe o no, la RESPUESTA es la misma:
              SOLO cambia el ledger de lecturas
              AS OF por proyecto: 28 → 16 (IndicePorLabel) · AS OF global: 168 → 18
              (IndiceDesdeAnio) · índice que no casa: 28 = 28 y paga mantenimiento

EL CONTRATO DE KB-LIRA (lo que los validadores 41-43 sembraron, ahora declarado):
  Extremos:   AUTHORED Persona→Documento · CITES Documento→Documento
              ABOUT Documento→Tema · MENTIONS Documento→{Persona|Organizacion|Proyecto}
              MEMBER_OF Persona→Organizacion · WORKED_ON Persona→Proyecto
  Existencia: Documento.titulo · Documento.anio · MEMBER_OF.desde_anio
  Tipo:       Documento.anio:Int · MEMBER_OF.desde_anio:Int · Persona.orcid:String
  Unicidad:   Persona.orcid   (9/9 personas la cumplen; un duplicado es violación)
  SinSolape:  MEMBER_OF por persona, intervalos [desde, hasta) disjuntos
              (Beto [2018,2024) y [2024,∞): SÍ · dos intervalos que se pisen: NO)

LOS DOS ÍNDICES Y SU FACTURA (moneda = lecturas):
  IndiceDesdeAnio  BTreeMap<anio, Vec<arista>>  global AS OF: 168 → 18
                   (poda por desde; el hasta se paga por candidata — el intervalo cobra)
  IndicePorLabel   adyacencia por etiqueta (CSR lógico)  AS OF proyecto: 28 → 16
                   (el salto persona→org deja de leer las 12 aristas que no son MEMBER_OF)
  IndiceAnioDoc    NO SE CONSTRUYE: selectividad 0.83 (cap-21) — decoración con factura
```

Y debajo, la REGLA DE ORO heredada (determinismo total, cap. 34): el paso-4 es un artefacto DERIVADO del paso-3 (mismo store, 9 props `orcid` añadidas, cero nodos/aristas nuevos); cada consulta devuelve su `CosteLecturas` pineado; las 10 preguntas del cap-41 + los pines del cap-42 + el validador paso-3 son la red de seguridad: si el esquema cambia una respuesta vieja sobre los subgrafos 1-3, el cambio está MAL. La frontera, declarada antes de codificar: `cap44_esquema.rs` es aditivo; ni `GraphStore` ni el executor ni `cap41/42/43` se tocan; el validador paso-3 acepta el paso-4 (las reglas nuevas —orcid, sin-solape— NO están gobernadas por los validadores 1-3: se declaran SOLO en el esquema).

```text
Lo que SÍ se hace hoy:   esquema declarativo (5 variantes de regla) + verificar_esquema,
                         kb_lira_paso4 (orcid a 9 personas), caso ORCID (duplicado/sin-orcid),
                         IndiceDesdeAnio + IndicePorLabel con ledger antes/después,
                         el desajuste 28=28 y la selectividad 0.83 como casos de «estorba»,
                         equivalencia esquema↔validadores, regresión cuádruple, CSV paso-4 +
                         esquema.csv + artefacto commiteado
Lo que AÚN NO:           validar al escribir dentro de la ingesta (cap. 45: el esquema SIRVE
                         al pipeline) · SHACL/shapes (cap. 47: el hermano declarativo del
                         mundo RDF) · RDF/quads (cap. 46) · índice de intervalos real
                         (nombrado como «interval tree» mental; gancho a cap. 53) ·
                         integración del índice en el optimizador del cap-21 (el catálogo
                         se reutiliza como ORÁCULO de decisión, no como planificador)
```

Momento ¡ajá! perseguido: **«los validadores de los capítulos 41-43 eran el esquema disfrazado de código: la regla "MEMBER_OF exige desde_anio" escrita como un if. El esquema es la misma regla como DATO — y por ser dato se puede verificar, serializar y discutir. Y el cap-21 ya lo sabía: su EqIndexEntry era este índice sin construir»**.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | `kb_lira_paso4()` = `kb_lira_paso3()` + `aplicar_orcid` (9 personas, ids 0-5 y 30-32): el paso-4 es una CAPA de esquema, cero nodos/aristas nuevos (68 nodos/158 aristas intactos) | El paso-4 no modela entidades: modela las REGLAS del modelo que ya existe; partir del paso-3 garantiza que el esquema se verifica contra el modelo temporal vigente (10 MEMBER_OF con intervalos) y que la regresión hereda los pines 1-3; las 9 personas con orcid hacen que la regla de unicidad sea estricta y uniforme (la persona sin orcid es el CONTRASTE: violación de existencia, no de unicidad) | (a) Añadir nodos/aristas al paso-4: fuera del alcance del outline (el paso es «ORCID único, tipos por etiqueta, índice sobre año») y rompería los pines 68/158; (b) orcid solo a las 6 del paso-1: la regla UNIQUE con valores ausentes obligaría a la semántica «si está presente» — más frágil pedagógicamente; la decisión estricta (todas) es la lección | OUTLINE-VOL3.yml cap-44 (paso-4); cap43_temporalidad.rs:181 (firma pública paso-3); cap-42 (Gaby/Hugo/Iris, ids 30-32, sin MEMBER_OF en el lote pero con afiliaciones del paso-3: 182-184) |
| 2 | Esquema DECLARATIVO (`Esquema` = `Vec<ReglaConstraint>`, 5 variantes) como la forma canónica nueva; los validadores 41-43 CONVIVEN intactos (regresión) y el test de equivalencia demuestra que el esquema los subsume para las reglas compartidas | La pregunta honesta es pedagógica: ¿el esquema SUSTITUYE a los validadores? Respuesta: como puerta de verificación, SÍ (el paso-4 valida con `verificar_esquema`); como red de seguridad histórica, los validadores se quedan (regla dura: no se tocan) — el capítulo enseña que el esquema es la MISMA regla como dato, y el test de equivalencia (mismo fixture roto → mismas violaciones con ids) lo demuestra SIN reescribir nada; declarar las reglas del paso-3 como `ReglaExistencia/Tipo` es literalmente la promesa del cap-43 («el validador paso-3 SIEMBRA las reglas que allí serán constraints») | (a) Reescribir los validadores como esquema y borrar la cadena: rompe la regresión dura y la historia pedagógica (el lector vería «tirar» lo construido); (b) esquema solo para lo NUEVO (orcid/solape), sin declarar lo viejo: el esquema no sería el contrato del modelo, sino un apéndice — se pierde la lección «los validadores eran el esquema» | cap43_temporalidad.rs:1077 (siembra); contrato cap-43 decisión #9; manuscrito cap-43 §43.9.1 («convención ejecutable, allí serán garantía»); cap42_antipatrones.rs:1505 (firma pública paso-2) |
| 3 | ORCID: todas las 9 personas con `orcid:String` (formato 4-4-4-4 determinista por id); verificación APARTE (`verificar_esquema` sobre el store), SIN tocar `GraphStore`; el duplicado se detecta insertando una persona con orcid existente → violación con id | (a) La verificación aparte es el patrón del proyecto (los validadores siempre fueron capa externa; `GraphStore` es intocable); la integración en el write (put_node que valida) se NOMBRA como el patrón industrial (Neo4j rechaza en el write) y como gancho del cap-45; (b) todas con orcid porque la unicidad estricta es la lección: la persona sin orcid demuestra que existencia y unicidad son DOS reglas distintas; (c) el UNIQUE se verifica con un índice de unicidad interno (BTreeMap<valor, Vec<id>>) porque «UNIQUE ≡ índice» es contenido, no detalle | (a) Envolver el store (`StoreConEsquema` que valida en cada put): es el cap-45 (validación al importar); además añade superficie de código que el capítulo no necesita para la tesis; (b) orcid a algunas personas: la regla «si está presente» complica el UNIQUE sin aportar lección (la semántica trivalente del cap-20 como fantasma) | cap41_modelado.rs:354 (`Violacion` pública, reutilizada); cap20_volcano.rs (semántica trivalente — fantasma evitado); Neo4j Cypher Manual (constraints de unicidad respaldados por índices; validación en el write) |
| 4 | Índice sobre año: `IndiceDesdeAnio` LÓGICO propio (`BTreeMap<i64, Vec<usize>>`), la versión «de modelo»; el `BPlusTree`/`HashIndex` del cap-15 se NOMBRAN como hermanos físicos-persistentes (con el mapeo `range_scan(lo,hi)` ≡ rango de anios), SIN reutilizarlos; y la lección HONESTA de la sección 3: el índice de clave simple NO abarata el AS OF de intervalo persona-céntrica (28 = 28) porque no casa con la forma de la consulta; abarata la consulta GLOBAL reescrita (168 → 18); el que sí abarata el AS OF del cap-43 es `IndicePorLabel` (adyacencia por etiqueta, el CSR lógico): 28 → 16 | La decisión es doble: (a) LÓGICO vs FÍSICO: el cap-15 es físico (persiste en páginas del BufferPool, claves u64→u64, UN valor por clave, requiere `Pager` de cap-12/13) — reutilizarlo sobre el `MemoryStore` exigiría montar un pager para un store en RAM y las claves `i64` no caben en `u64` sin casting; el índice lógico enseña la MISMA idea con el 1% del andamiaje y es lo que los caps. 45+ necesitarán (el paso-4 es de MODELO); (b) la consulta: reescribir la persona-céntrica para usar el índice de año exigiría un plan de consulta invertido (afiliaciones globales → filtrar por proyecto) que en el cap-43 costaba MÁS (el join por proyecto paga in_edges por persona); el capítulo DECIDE la versión honesta: el índice de año gana donde su clave manda (la pregunta global) y el índice de adyacencia gana donde el salto manda (la pregunta por proyecto) — «el índice debe casar con la consulta» es la lección, con ambos ledgers pineados | (a) Reutilizar `BPlusTree` del cap-15 sobre un `Pager` de fichero: pesado, físico, claves u64 — el lector YA lo construyó (spacing fuerte) y se le NOMBRA como la versión persistente de este mismo índice; (b) índice de intervalos real («interval tree»): se enseña el CONCEPTO (poda por un lado, el intervalo cobra el otro) y se nombra como estructura mental y gancho al cap-53, pero implementarlo aquí añade un tipo nuevo sin cambio de lección — el capítulo enseña cuándo el índice simple BASTA y cuándo NO | cap15_indices.rs:970 (`range_scan(lo, hi)` público); cap15_indices.rs:363/789 (físicos, sobre `BufferPool<P>`); cap14_csr.rs (CSR — padre del `IndicePorLabel`); manuscrito cap-43 §43.9.1 («el AS OF SIN índice —28 = 28— ES la lección»); ledger real de cap-43: 1 in_edges + 21 get_edge + 6 get_node |
| 5 | «Cuándo estorban»: dos casos medibles — (a) selectividad: `Catalog::collect` del cap-21 decide que `Documento.anio` (5 valores en 12 nodos; filtro `anio > 2022` ≈ 0.83) NO merece índice para P10 — el catálogo del cap-21 como ORÁCULO de decisión (se reutiliza, no se toca); (b) mantenimiento sin uso: `IndiceDesdeAnio` construido (10 lecturas) para la persona-céntrica mide 28 = 28 — el índice que no se usa cobra en cada escritura | La sección 3 del outline («cuándo estorban») exige un caso CON números, no prosa: (a) usa la herramienta que el proyecto YA construyó para decidir (cap-21: `selectivity`, `EqIndexEntry`); (b) convierte el desajuste de la decisión #4 en contenido (el «estorbo» no es que el índice sea malo: es que no casa con la consulta) | cap21_optimizador.rs:108 (`EqIndexEntry` — el índice de igualdad MODELADO), :154 (`Catalog::collect`), :527 (`selectivity`); cap41_modelado.rs:978 (P10 filtra `p.anio > X` sobre :Paper) |
| 6 | Etiquetas múltiples: las reglas del esquema aplican a todo nodo que TENGA la label (`:Paper`, `:Nota`, `:Informe` heredan las reglas de `:Documento`); `MENTIONS` sigue POLIMÓRFICO por decisión de esquema (`ReglaExtremos` con lista de destinos [Persona\|Organizacion\|Proyecto]) — el gancho del cap-42 cobrado con la regresión como evidencia (P6 no cambia); el `IndicePorLabel` agrupa por label exacta (el subtipo es otra label: `:Paper` y `:Documento` tienen índices distintos si se piden) | El cap-42 declaró «la especificidad por label es decisión de ESQUEMA (cap. 44)»: el capítulo DEBE responder. Respuesta: no refinar (refinar MENTIONS rompería P6 y el cap-42 lo demostró); el esquema concreta el polimorfismo como lista de destinos permitidos — la MISMA regla que el validador paso-1 ya aplicaba; y el catálogo del cap-21 ya agrupa por label: las estadísticas de `:Paper` vs `:Documento` son la evidencia de que la label es la unidad de esquema | (a) Refinar MENTIONS en subtipos: rompe P6 (regresión) y el cap-42 lo descartó como «limpieza» — sería perseguir estética; (b) aplicar reglas por label EXACTA (no por pertenencia): `:Paper` sin las reglas de `:Documento` dejaría papeles sin título — la pertenencia es la semántica GQL de label sets | cap42_antipatrones.rs §5 («MENTIONS sigue polimórfico… decisión de ESQUEMA (cap. 44)»); cap41_modelado.rs:115 (`documento(id, sub, …)` — labels `["Documento", sub]`); cap21_optimizador.rs:154 (estadísticas por etiqueta); Microsoft Fabric gql-graph-types (constraint pattern «nodes with at least the Person label») |
| 7 | GQL DDL como referencia industrial (sección 5): `CREATE PROPERTY GRAPH`, graph types (node/edge types con property value types y `NOT NULL`), `CONSTRAINT … FOR … REQUIRE … IS KEY / IS UNIQUE / IS NOT NULL`, `CREATE INDEX` — presentados como la forma industrial de lo que el capítulo construyó a mano; [VERIFICAR el subclausulado fino contra el texto de la norma] | El cap-44 es el lugar natural del DDL de GQL (el outline lo pide) y el cap-48 lo retomará como lenguaje; mostrar el contraste «nuestro `ReglaUnicidad{Persona, orcid}` ↔ `CONSTRAINT … FOR (n:Person) REQUIRE n.orcid IS UNIQUE`» es el puente que el lector de datos/IA necesita; GQL es el primer ISO nuevo desde SQL (outline cap-48) | (a) Dejarlo fuera (solo prosa, sin sección): el outline lo exige; (b) Cypher de Neo4j como referencia principal: GQL es el estándar y el capítulo ya muestra Neo4j en «cómo lo hace una BBDD real»; (c) citar subclausulados no verificados: se marca [VERIFICAR] como en el cap-43 | ISO/IEC 39075:2024 (abril 2024; iso.org/standard/76120; preview iTeh del índice de la norma — GQL-catalog, schema terms; Microsoft Fabric gql-conformance y gql-graph-types — sintaxis del estándar; Geode GQL API reference — keywords CONSTRAINT/KEY/REQUIRE/INDEX del estándar), verificadas 2026-08-26 |
| 8 | Citas: GQL ISO/IEC 39075:2024 (abril 2024, Parte 1 con DDL de esquema, [VERIFICAR fino]); Neo4j Cypher Manual actual 5.x (uniqueness `IS UNIQUE`, existence `IS NOT NULL`, key constraints, property type constraints — Neo4j 5, blog oficial 3 nov 2023; índices range/text/point/fulltext; UNIQUE respaldado por índice); Baron Schwartz, «Schemaless Databases Don't Exist», SolarWinds blog, 24 feb 2015 (el «schemaless mentiroso» del outline, VERIFICADA); SQL:2011/SQL:2023 solo como referencia clásica de UNIQUE/NOT NULL [VERIFICAR fino]; JanusGraph/TigerGraph FUERA | La cita de Schwartz es el ancla de la sección 1 (el mito con nombre y fecha); Neo4j 5.x es la versión actual del manual (constraints de tipo de propiedad como la «ReglaTipo» industrial); GQL verificado contra la norma publicada y tres fuentes secundarias que citan su sintaxis — con la marca [VERIFICAR] para el subclausulado fino (espejo del cap-43 con el AS OF); JanusGraph/TigerGraph no verificables hoy → fuera (espejo caps. 42-43) | Citar Neo4j 3.4/4.4 (versiones viejas del manual): el cap-43 citó 3.4 por los TIPOS TEMPORALES; aquí la referencia de constraints es la 5.x actual; citar docs de JanusGraph/TigerGraph sin verificar: riesgo de versión o URL inventada | Verificación puntual realizada hoy con fuentes primarias en vivo; contrato cap-43 (misma política de citas) |
| 9 | Alcance de código: UN módulo `cap44_esquema.rs` (~800-1200 líneas, std puro, ~26 tests) + wiring aditivo (2 líneas) + artefacto `datasets/kb-lira/paso-4/{nodes.csv,edges.csv,esquema.csv}`; **SIN bench** | El módulo accede GRATIS a los builders 41-43, a `Violacion`, al catálogo del cap-21 y al CSV del cap-32; el esquema.csv con exportador propio mínimo (el esquema es DATO: se serializa y se recarga — la lección de que el esquema no vive en el código) ; la moneda son lecturas y violaciones exactas, nunca µs (espejo de las decisiones #12 cap-41 y #11 cap-43) | (a) Crate nueva: churn sin ganancia (precedente caps. 38-43); (b) primer bench del Vol.III: cronometrar no sostiene ninguna tesis — el capítulo es de ESTRUCTURA (ledgers, conjuntos, selectividades); si algún caso pidiera cronometraje, es reto experto con criterion YA presente | CONVENTIONS §2 y §4; contrato cap-43 decisión #11; precedente caps. 38-43 |
| 10 | Fronteras duras: cap-45 ingesta (el esquema SIRVE a la validación al importar — «write-time» se NOMBRA como el patrón industrial y el gancho; NO se construye el pipeline); cap-46 RDF (sin shapes SHACL — se NOMBRAN como gancho); cap-47 shapes (el esquema de aquí es el hermano declarativo LPG; SHACL es la forma RDF); validadores 41-43 INTACTOS (regresión cuádruple) | El esquema sin frontera se convertiría en el cap-45 (pipeline con validación) o en el 47 (shapes); la regresión dura garantiza que declarar las reglas no rompe ninguna respuesta vieja — el test de equivalencia y los 4 tests de regresión son la puerta | (a) Validar en el write del builder (envolver put_node): cap-45; (b) comparar con SHACL en profundidad: cap-47; (c) tocar validadores: regla dura | OUTLINE-VOL3.yml caps. 45-47; contrato cap-43 (fronteras); manuscrito cap-43 §43.9 |
| 11 | `validar_modelo_kb_lira_paso4` NO existe como función nueva: la puerta de verificación del paso-4 ES `verificar_esquema(store, esquema_kb_lira_paso4())`; la cadena de validadores 1-3 se re-ejecuta SOLO como regresión | La lección de la decisión #2 es que el esquema sustituye a los validadores como forma canónica; inventar un validador paso-4 imperativo duplicaría las reglas del esquema en código (el antipatrón que el capítulo enseña a dejar atrás); la regresión «validador paso-3 acepta paso-4» demuestra que la cadena histórica sigue viva como red de seguridad | (a) `validar_modelo_kb_lira_paso4` = paso-3 + orcid + solape en imperativo: duplicación exacta del esquema — el error que el capítulo corrige; (b) borrar la cadena: regla dura | contrato cap-42 decisión #8 (composición); cap43_temporalidad.rs:1077 (paso-3 público) |
| 12 | `ReglaSinSolape` (unicidad temporal sobre MEMBER_OF por persona, intervalos `[desde, hasta)` disjuntos) entra como variante del esquema — el gancho literal del cap-43 («¿quién garantiza que dos afiliaciones no se solapen?») | El cap-43 declaró deuda explícita: «sin constraints UNIQUE temporales (cap. 44)»; la regla es pequeña (pares por persona con la convención medio abierta YA pineada) y demuestra el límite de la unicidad de propiedad simple: «una persona, una organización» es FALSO en KB-Lira (Beto tiene dos); lo único prohibido es el SOLAPE; el dato limpio pasa (Beto [2018,2024) y [2024,∞): disjuntos) y el fixture (una MEMBER_OF de Beto con [2019, 2023)) viola con id — la misma estructura de la regla temporal que SQL:2011 formalizó como period constraints [VERIFICAR fino] | (a) Dejarlo como reto experto sin implementar: el gancho del cap-43 quedaría incumplido — el outline pide cobrarlo en el blockquote; (b) ReglaUnicidad sobre (persona, org): regla INCORRECTA que el propio dato desmiente (Beto-legítimo) — se enseña por qué falla en el fixture, no como regla | manuscrito cap-43 §43.9.1 y cierre («¿quién garantiza que dos afiliaciones no se solapen en el tiempo?»); cap43_temporalidad.rs:282 (`arista_vigente_en`, convención `[desde, hasta)`); SQL:2011 period constraints [VERIFICAR] |

## 6. Estructura del manuscrito (partes y tempos)

1. **Blockquote inicial OBLIGATORIO**: CUARTO capítulo del Vol.III, audiencia (lector del
   cap-43 + perfil IA/datos), conexión con caps. 41-43 por referencias (los validadores por
   composición, el AS OF a 28 lecturas, la siembra del paso-3, la regresión — sin
   re-explicar nada), gancho cobrado literalmente: «¿quién construye el índice que abarata
   el AS OF?» → dos índices con dos ledgers (28→16, 168→18); «¿quién garantiza que dos
   afiliaciones no se solapen?» → la `ReglaSinSolape` del esquema declarativo; y el gancho
   del cap-42 («la especificidad por label es decisión de esquema») → `MENTIONS` resuelto
   por declaración.
2. **Apertura (N.0, anécdota + pregunta crítica)**: la anécdota verificada — Schwartz
   (2015) desmontando el mito: «there is always a schema somewhere. Usually in multiple
   places»; el cajón del taller donde «todo cabe» hasta que no encuentras nada: un orcid
   duplicado, un documento sin año, dos afiliaciones solapadas. Pregunta enmarcada: tu
   KB-Lira acepta cualquier cosa — ¿a qué precio?
3. **N.1-N.2 Objetivo/Problema**: objetivo medible del outline («elegir entre esquema
   abierto y estricto, y usar constraints e índices sin pagar de más»). Problema: los
   validadores 41-43 SIEMBRAN reglas pero no las declaran: cada regla nueva reescribe el
   validador (crecimiento imperativo); el AS OF del cap-43 paga 28 lecturas y el capítulo
   prometió el índice; y nadie garantiza la unicidad (dos Anas caben en el grafo).
   Desactivar las seis misconcepciones ANTES de dibujar.
4. **N.3 Modelo mental**: el CONTRATO y el ATAJO (constraint ≠ índice; UNIQUE ≡ índice en
   las BBDD reales); la tabla del contrato de KB-Lira (5 variantes de regla); los dos
   índices con su factura en lecturas; la selectividad como juez (cap-21).
5. **N.4 Primera solución**: la NO-solución doble — (a) seguir SIN esquema: el grafo
   acepta el orcid duplicado y la P10 del cap-41 calla cuando un documento pierde el
   `anio`; (b) seguir con el validador imperativo: añadir la regla de unicidad es añadir
   OTRO bloque de `if`s al validador paso-3 (la deriva que los caps. 41-43 ya mostraron
   creciendo). El capítulo muestra ambas con sus modos de fallo ANTES de la solución.
6. **N.5 Sus límites**: el grafo abierto no MENTE: no sabe que dos Anas son la misma
   persona (sin orcid no hay forma de distinguir); el validador imperativo crece sin
   reutilizarse (cada capítulo re-sembró las reglas a mano); y la promesa del cap-43 —el
   índice— sigue sin responder.
7. **N.6 Solución evolucionada**: `Esquema` como dato + `verificar_esquema` (con el
   índice de unicidad interno) + `kb_lira_paso4` (orcid a 9 personas) + `IndiceDesdeAnio`
   (la global 168 → 18) + `IndicePorLabel` (la persona-céntrica 28 → 16) + el desajuste
   28 = 28 como lección + la selectividad 0.83 del cap-21 como «no construir» + la
   equivalencia esquema↔validadores + la REGLA DE ORO (respuestas viejas intactas).
8. **N.7 Código completo ejecutable**: `cap44_esquema.rs` por `include::` (nunca
   duplicado); SIN bench (decisión #9 explicada en una línea).
9. **N.8 Prueba de fuego**: salidas REALES de `cargo test`: la tabla de ledgers
   (28→16, 168→18, 28=28), las violaciones del fixture (duplicado/sin-orcid/solape con
   ids), la equivalencia con los validadores, la selectividad del catálogo, la regresión
   cuádruple, el CSV paso-4 byte a byte.
10. **N.9 Qué hemos sacrificado**: sin validación al escribir (cap. 45 — el esquema SIRVE
    a la ingesta; aquí la verificación es una capa aparte); sin SHACL ni shapes (caps.
    46-47 — el hermano declarativo del mundo RDF, nombrado como gancho); sin índice de
    intervalos real (el «interval tree» mental queda nombrado; la UNIQUE temporal con
    poda por ambos lados es deuda declarada, gancho al cap-53); sin integración del índice
    con el optimizador del cap-21 (el catálogo se usa como ORÁCULO de decisión, no como
    planificador); los índices viven en RAM (la versión persistente es el cap-15,
    nombrada).
11. **N.10 Cómo lo hace una BBDD real + retos**: **Neo4j** — constraints de unicidad
    (`IS UNIQUE`), existencia (`IS NOT NULL`), key constraints y property type constraints
    (Neo4j 5), respaldados por índices, validación en el write; **GQL (ISO/IEC
    39075:2024, abril 2024)** — `CREATE PROPERTY GRAPH`, graph types con node/edge types y
    property value types (`NOT NULL`), `CONSTRAINT … REQUIRE … IS KEY / IS UNIQUE / IS NOT
    NULL` y `CREATE INDEX` [VERIFICAR el subclausulado fino]; **SQL** — UNIQUE/NOT NULL
    como la referencia clásica [VERIFICAR fino SQL:2011 periods]. Retos: esencial
    (44+21): PREDECIR por escrito los ledgers del AS OF con `IndicePorLabel` (16 = 16) y
    de la global con `IndiceDesdeAnio` ANTES de correr, y decidir con el catálogo si un
    índice sobre `anio` de Documento merece la pena (selectividad 0.83 → NO) — y por qué
    el mismo razonamiento SÍ justifica el índice de unicidad del orcid; intermedio
    (44+15+14, 44+43): comparar `IndiceDesdeAnio` (lógico, BTreeMap) con el `BPlusTree`
    del cap-15 (físico, sobre BufferPool, `range_scan`) y con el CSR del cap-14 — qué es
    lo mismo y qué es distinto (persistencia, claves u64, un valor por clave); y aplicar
    el esquema a `WORKED_ON`: ¿qué reglas de existencia/tipo le declararías (¿`desde_anio`
    opcional Int?) y por qué?; experto (44+43+28): usar `WalTransaccion` REAL del cap-28
    para demostrar que la escritura de una `MEMBER_OF` solapada se puede RECHAZAR antes
    del commit — el esqueleto de «constraint en el motor» (validación en la transacción),
    y la frontera que separa esto de la ingesta del cap-45.
12. **Baterías finales + gancho**: Lo que te llevas / Ojo cuidado / Pin / 30 segundos /
    historia pequeña (el cajón del taller que «todo cabe» y el contrato que lo ordena) /
    Mini-diálogo de guardia nocturna (dos Anas en producción: «¿cuál es la verdadera?» —
    el orcid que nadie declaró único). Retrieval practice: recitar DE MEMORIA la
    distinción constraint vs índice con un ejemplo propio, y clasificar 5 afirmaciones
    («¿puede existir?» vs «¿cuánto cuesta encontrarlo?») sin mirar. Spacing: cap-21
    (catálogo, `selectivity`, `EqIndexEntry`), cap-15 (BPlusTree físico), cap-14 (CSR),
    validadores y 10 preguntas (41-43), CSV cap-32, `Value` cap-7, ContandoStore cap-26.
    Interleaving: cada reto toca ≥2 capítulos (44+21, 44+15+14, 44+43+28). Glosario
    nuevo: esquema abierto, esquema estricto, constraint, regla de existencia, regla de
    tipo, regla de unicidad, unicidad temporal, esquema declarativo, índice de
    propiedades, selectividad, mantenimiento de índice, validación al escribir
    (write-time), DDL, índice de intervalos. Gancho al cap. 45: «el esquema existe, pero
    nadie valida al importar: el lote de mañana puede traer un duplicado — ¿quién lo
    ataja en el pipeline?». Abiertas: shapes SHACL (47), el índice de intervalos como
    memoria del agente (53).

---

## 7. Estilo y tono (consistencia con el proyecto)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (constraint, index, schema-less, selectivity, DDL, write-time,
  range scan, key constraint); salidas REALES de `cargo test` pegadas, nunca
  reconstruidas; las decisiones de esquema se presentan como TRADE-OFF con precio en
  lecturas o en garantías, nunca como dogma.
- **Diagramas**: la figura «constraint ≠ índice» (dos preguntas: ¿puede existir? /
  ¿cuánto cuesta encontrar?); la tabla del contrato de KB-Lira (5 variantes con sus
  labels); la tabla de ledgers (consulta → sin índice → con índice → ahorro) como figura
  recurrente; la línea de Beto con los dos intervalos disjuntos (la unicidad temporal
  que SÍ se cumple).
- **Spacing** (conceptos viejos que se EJERCITAN): el catálogo del cap-21 (`Catalog::collect`,
  `selectivity`, `EqIndexEntry` — el índice modelado que aquí se construye), los índices
  físicos del cap-15 (BPlusTree/range_scan) y el CSR del cap-14, los validadores y las 10
  preguntas (41-43), la convención `[desde, hasta)` del cap-43, CSV cap-32, `Value` cap-7,
  dataset determinista cap-34.
- **Interleaving**: el reto esencial mezcla 21+44 (selectividad y predicción de ledgers);
  el intermedio mezcla 14+15+44 (tres índices frente a frente) y 43+44 (esquema sobre
  WORKED_ON); el experto mezcla 28+43+44 (rechazo antes del commit con el WAL real).
- **Dificultad asimétrica**: una idea nueva por sección (schemaless → constraints →
  índices → etiquetas múltiples → GQL); los ejercicios exigen PREDECIR ledgers y DECIDIR
  con el catálogo ANTES de correr los tests.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap44` (lecturas, conjuntos y
  violaciones exactas) y `./scripts/verify.sh` ALL_GREEN como puerta. Nunca «confía en mí».
- **Anécdota (única, verificada)**: Baron Schwartz (SolarWinds blog, 24 feb 2015) — «there
  is always a schema somewhere» y el cajón del taller que «todo cabe». Fuentes para la
  prosa: GQL ISO/IEC 39075:2024 (abril 2024); Neo4j Cypher Manual 5.x + blog property
  constraints (3 nov 2023); Schwartz 2015; SQL:2011/SQL:2023 [VERIFICAR fino].

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap44_esquema; pub use
  cap44_esquema::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al final, con el
  módulo ya compilando limpio; jamás dejar `lib.rs` apuntando a un módulo rojo. `kb_lira_paso4()`
  NO toca `cap41/42/43`: llama a `kb_lira_paso3()` (pública, cap43:181) y añade `orcid`
  encima; los ids 0-69 y las aristas 0-185 se reutilizan SIN crear ninguno nuevo (el test
  de estructura 68/158 lo cazaría). `Violacion` y `CosteLecturas` se REUTILIZAN tal cual
  (cap41_modelado.rs:354, cap43_temporalidad.rs:356). El catálogo del cap-21 se usa por su
  API pública (`Catalog::collect`, `label_stats`, `selectivity` — verificar firmas antes
  de usarlas, sin tocarlas).
- **Orden de implementación recomendado (PATRÓN DE TROCEO — cada pieza compila y testea
  SOLA: 1 función + 1 test, el agente de código no sobrevive a tareas largas)**:
  (1) tipos del esquema (`ReglaConstraint`, `TipoEsperado`, `Esquema`) + `esquema_kb_lira_paso4()`
  + `el_esquema_paso4_declara_las_reglas_exactas`; (2) `verificar_esquema` (sin la regla
  SinSolape aún: extremos/existencia/tipo/unicidad con índice interno) + `verificar_esquema_acepta_el_modelo_paso4`;
  (3) `kb_lira_paso4()` (`paso3` + `aplicar_orcid` con las 9 personas) +
  `estructura_de_kb_lira_paso4_cuenta_y_etiquetas_exactas` + `las_9_personas_llevan_orcid_determinista`;
  (4) casos ORCID: `orcid_duplicado_es_violacion_de_unicidad_con_id` +
  `persona_sin_orcid_es_violacion_de_existencia` (mutación directa de `s.edges`/`put_node`);
  (5) `ReglaSinSolape` + `las_member_of_del_paso4_no_se_solapan` + `una_member_of_solapada_es_violacion_con_id`;
  (6) equivalencia: `el_esquema_reproduce_las_violaciones_de_la_cadena_de_validadores` +
  `verificar_esquema_es_la_puerta_canonica_del_paso4`; (7) `IndiceDesdeAnio` (construir/
  rango/insertar) + `indice_desde_anio_cubre_igualdad_rango_y_mantenimiento`;
  (8) `afiliaciones_vigentes_global_en` + `el_indice_de_anio_abarat_a_la_consulta_global`
  + `el_indice_de_anio_no_abarat_la_consulta_por_proyecto` (28 = 28 pineado); (9)
  `IndicePorLabel` + `el_indice_por_etiqueta_abarat_el_as_of_por_proyecto` (28 → 16);
  (10) selectividad con el catálogo del cap-21: `la_selectividad_del_cap21_decide_cuando_un_indice_no_merece_la_pena`
  + `el_indice_que_no_casa_con_la_consulta_cuesta_mantenimiento`; (11) regresión cuádruple
  (`las_10_preguntas_del_paso1_no_cambian_con_esquema`, `las_respuestas_del_paso2_no_cambian_con_esquema`,
  `validador_paso3_acepta_el_modelo_paso4` — ANTES verificar las firmas `pregunta_01…10`
  y `validar_modelo_kb_lira_paso3` tal cual); (12) CSV paso-4 (nodes con orcid, edges
  idéntico al paso-3, esquema.csv con round-trip propio) + generar y commitear
  `datasets/kb-lira/paso-4/`; (13) `informe_esquema_reproducible` + wiring.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap44` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap44_esquema.rs` y
  continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). PROHIBIDO
  tocar `cap41_modelado.rs`, `cap42_antipatrones.rs`, `cap43_temporalidad.rs`, el parser
  de LiraQL o el trait `GraphStore`: el esquema y los índices se hacen CON la API existente.
- **Criterio de parada honesto**: si declarar el esquema cambia la respuesta de alguna de
  las 10 preguntas sobre el subgrafo paso-1 (o los pines del paso-2, o la aceptación del
  paso-3), el cambio está MAL y se rediseña DENTRO del capítulo — prohibido «ajustar» la
  pregunta, el dataset o el esquema para que cuadre. Igual con los ledgers: 28 → 16 y
  168 → 18 y 28 = 28 son los pines de la tesis — si el ledger midiera otra cosa, se
  explica POR QUÉ (precedente cap-43: el contrato predecía 13→14 y la prosa pineó el
  ledger real 21→20), prohibido maquillar contadores. La convención de intervalo es
  `[desde, hasta)` (medio abierto, heredada del cap-43): AS OF 2024 = GrafoLuna, y el
  solape `[2018,2024)` con `[2024,∞)` NO es solape (disjuntos en 2024) — respetar en TODA
  la implementación, incluida la `ReglaSinSolape`.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 filas
  en §5; citas verificadas 2026-08-26: GQL ISO/IEC 39075:2024 abril 2024 con DDL de esquema
  en Parte 1 [VERIFICAR subclausulado fino], Neo4j Cypher Manual 5.x + property type
  constraints (blog 3 nov 2023), Baron Schwartz «Schemaless Databases Don't Exist»
  (SolarWinds blog, 24 feb 2015); SQL:2011 marcado [VERIFICAR]; JanusGraph/TigerGraph
  descartados por no verificables hoy).
- [ ] Escenario de fallo visible, no solo happy path: el orcid duplicado (violación con
  id), la persona sin orcid (existencia ≠ unicidad), la MEMBER_OF solapada (fixture), el
  desajuste 28 = 28 (índice que no casa), la selectividad 0.83 (índice que no se
  construye), el fixture roto de la equivalencia.
- [ ] Código ejecutable citado por nombre (`cap44_esquema.rs`, wiring, artefacto
  `datasets/kb-lira/paso-4/`, SIN `[[bench]]`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: seis).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa como patrón;
  esencial: ledgers + selectividad; intermedio: 14+15+44 y esquema sobre WORKED_ON;
  experto: WAL real contra la MEMBER_OF solapada).
- [ ] ≥1 ejercicio de retrieval (constraint vs índice DE MEMORIA + clasificar 5
  afirmaciones) y spacing planificado (caps. 21/15/14/41/42/43/32/7/26/34; §7).
- [ ] Responde las TRES preguntas críticas del capítulo (schemaless = mito con Schwartz y
  los validadores como esquema pre-existente; constraint ≠ índice con los dos ledgers y
  el UNIQUE ≡ índice; cuándo estorba con la selectividad del cap-21 y el desajuste
  28 = 28) y cobra los ganchos del cap-43 («¿quién construye el índice?» → 28→16 y
  168→18; «¿quién garantiza que dos afiliaciones no se solapen?» → ReglaSinSolape) y del
  cap-42 (MENTIONS por decisión de esquema).
- [ ] Red de seguridad CUÁDRUPLE con tests de nombre exacto:
  `las_10_preguntas_del_paso1_no_cambian_con_esquema`,
  `las_respuestas_del_paso2_no_cambian_con_esquema`,
  `validador_paso3_acepta_el_modelo_paso4`, `verificar_esquema_acepta_el_modelo_paso4`.
- [ ] Anécdota única verificada con fuente primaria (Schwartz 2015, SolarWinds blog).
- [ ] Alcance acotado y honesto (UN módulo + wiring + artefacto paso-4; cero deps, cero
  benches, cero cambios caps. 7-43; frontera dura con caps. 45-47 y 53 declarada; índices
  en RAM, el cap-15 nombrado como versión persistente).
- [ ] Blockquote inicial declara CUARTO CAPÍTULO DEL VOL.III (audiencia + conexión con
  caps. 41-43 por referencias) y ganchos cobrados literalmente (el índice del AS OF y la
  unicidad temporal); gancho saliente fijado (cap. 45: validación al importar; caps. 46-47:
  SHACL; cap. 53: índice de intervalos).
