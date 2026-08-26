# Capítulo 42 — Antipatrones: supernodos, reificación y otras trampas

> *«Segundo capítulo del Volumen III. Si vienes del cap. 41 —o del perfil datos/IA que lo haya leído en diagonal— la escalera R1-R7, las 10 preguntas de KB-Lira, el validador y el modelo naive son tus HERRAMIENTAS, no tu contenido: aquí el modelo ya existe y se DEGRADA. El cierre del cap. 41 te dejó una deuda clavada: el Tema popular de tu flamante KB-Lira YA acumula 6 aristas ABOUT que nadie pidió. ¿Cuándo un hub deja de ser inocente? Cuando cruza el umbral del detector — y este capítulo te da el umbral (5× la mediana y 25% del share), el detector y tres refactors con su precio en reescrituras, para pagar la deuda ANTES de que el interés sea el dataset entero.»*

## 42.0 La anécdota de la esquina

Febrero de 2024. Justin Boylan-Toomey describe en su blog un knowledge graph académico real: **100 millones de publicaciones** conectadas a **176 nodos de «campo de investigación»**. El modelo era impecable —sobre el papel: un paper, su campo, una arista ABOUT. Pero las consultas habían pasado de responder en **segundos** a tardar **«casi un día»**. ¿Qué pasó? Los 176 nodos de campo crecieron hasta acumular millones de publicaciones cada uno: cada consulta que cruzaba un campo pagaba su grado entero. La cura fue el refactor inverso al tuyo: el campo pasó de nodo a **propiedad indexada**, y las consultas volvieron a segundos. El supernodo (supernode) no nació por descuido: nació porque los DATOS superaron al MODELO — y nadie lo vio venir mientras «se notaba poco».

Es la MISMA enfermedad que ya viste dos veces en el Vol.II, en otra divisa: el hub-concentrador del cap. 39 (×521 tuplas fantasma en el join) y el vertex-cut del cap. 40 (replicar el hub por el clúster) eran el skew cobrando en tiempo y particionado. Aquí el skew se cobra en el MODELO — y se cura en el modelo.

Tu KB-Lira tiene **30 nodos** y alguien acaba de importar **24 papers** más. ¿Sabes si ya hay un supernodo EN TU GRAFO — antes de que «crezca»?

## 42.1 Objetivo

Este capítulo cobra la deuda del cierre del cap-41 con el objetivo medible del outline: **detectar y refactorizar los errores de modelado más caros ANTES de que el dataset crezca**. Al terminar tendrás:

1. **El builder degenerado** `kb_lira_paso2_degrado()`: 59 nodos, 134 aristas — el paso-1 más un lote importado que siembra los antipatrones del capítulo a propósito (presagio del cap. 45).
2. **El detector de supernodos** `detectar_supernodos`: UN barrido de `iter_edges`, umbral doble relativo (ratio ≥ 5× la mediana del label Y share ≥ 25% del tipo de arista), con la mediana calculada EXCLUYENDO al propio hub.
3. **El triplete del detector**: paso-1 inocente (3,0×), paso-2 degenerado culpable (6,0× y 46%), tras el refactor silencioso (3,0×).
4. **Tres refactors como transformaciones puras** con `InformeRefactor` (nodos/aristas creados/borrados + lecturas): A descomponer el supernodo, B reificar la reseña, C conferencias↔propiedades. Migración completa: 10/2 nodos, 48/28 aristas, 64 lecturas.
5. **El validador paso-2 por composición**: reutiliza el del cap-41 (sin tocarlo) y añade las reglas nuevas.
6. **La REGLA DE ORO**: nada de lo que respondían las 10 preguntas sobre el paso-1 puede cambiar tras el refactor — verificada por test.
7. **CSV determinista paso-2** con round-trip byte a byte y el artefacto `datasets/kb-lira/paso-2/` commiteado.
8. **948 tests ALL_GREEN** (920 + 28), cero dependencias nuevas, cero cambios en caps. 7-41, **sin bench** (decisión #11 del contrato).

## 42.2 Problema

Mira tu suite: **920 tests verdes**. El motor parsea, optimiza, ejecuta, persiste. Y sin embargo, acaba de pasar algo que nadie eligió: alguien importó un lote de 24 papers, y **18 de sus 24 ABOUT cayeron en el tema 24** — el 46% de TODAS las ABOUT del grafo (52) se concentra en un tema que nadie votó. El grafo YA cobra: cada vez que alguien pregunta «¿qué documentos tratan *grafos de conocimiento*?», la expansión lee 24 aristas. La suite sigue verde — el validador del cap-41 sigue pasando — pero el modelo se ha degenerado por debajo de la nariz de todos. Ese es el problema: **verde no es sano**.

Antes de dibujar nada, desactivemos las cinco ideas equivocadas que suelen venir con el tema:

1. **«Un supernodo es un nodo con MILLONES de aristas.»** No: el supernodo es RELATIVO a la distribución de su propio grafo — «way more relationships than other nodes, relative to what else is in the graph» (Allen, 2020). El titular «10M de aristas» vende; el criterio reproducible es el desequilibrio, no el absoluto: en el paso-2, 24 aristas bastan para alarmar.
2. **«Los hubs son malos por definición.»** No: hay **hubs legítimos** (anchors): un nodo `Conferencia` o `Tema` que se RESUELVE por clave y no se atraviesa en los caminos calientes. El antipatrón es el nodo DEGENERADO que concentra grado y se cruza — Allen distingue el hub de dominio del hub de modelado: el criterio es el patrón de acceso, no el grado.
3. **«Refactorizar es rediseñar el grafo entero.»** No: es una transformación QUIRÚRGICA con red de seguridad — las 10 preguntas del cap-41 deben seguir respondiendo sobre el subgrafo paso-1. Si un refactor cambia una respuesta vieja, el refactor está MAL; no se «ajusta» la pregunta.
4. **«Reificar es bueno o malo en abstracto.»** No: la MISMA escalera R1-R7 descartó `Autoria` (reificación excesiva, cap-41) y exige `Resena` (reificación insuficiente sin ella, este capítulo). La prueba decide; el veredicto no es global.
5. **«Un tema por año es una comodidad inofensiva.»** Es el antipatrón clásico del **nodo categórico de baja cardinalidad**: pocos nodos que concentran muchísimas aristas cada uno — el `Gender` de Allen (2020), los 176 campos de investigación de Boylan-Toomey (2024).

Y el compromiso de honestidad que rige TODO el capítulo: los números salen de `cargo test`, nunca de la pizarra; el fixture pequeño no miente — si el refactor no reduce lecturas aquí (24 = 24), se dice POR QUÉ y dónde aparece el ahorro real (a escala), prohibido inflar.

## 42.3 Modelo mental: el mapa síntoma → diagnóstico → cura → precio

Un solo panel ordena todo el capítulo — el antipatrón, su detector y su cura como tres casillas encadenadas:

```text
SÍNTOMA                    DIAGNÓSTICO                          CURA (con precio)
grado desproporcionado →   detector (umbral DOBLE relativo): →  refactor de modelo
en UN label                ratio ≥ 5× mediana del label        (reescrituras contadas en
                           Y share ≥ 25% del tipo de arista     InformeRefactor) + red de
                           mediana EXCLUYENDO al propio hub     seguridad: las 10 preguntas
```

El **umbral relativo** (threshold relativo a la distribución) se operacionaliza con dos números a la vez, porque uno solo engaña: un único umbral relativo alarmaría con cualquier grafo diminuto (mediana 1 → todo nodo con 5 aristas se dispara). La AND de ambos es la defensa. Y la mediana (no la media) se calcula **excluyendo al propio hub**: si el outlier participara en su propia línea base, el detector se desinflaría a sí mismo a medida que el hub crece — el outlier no fija su propia línea base. El triplete del capítulo, con los números reales:

```text
paso-1            grados Tema [1,2,2,2,3,6]       mediana 2,0   tema 24: 6 ABOUT  → ratio 3,0×   share 0,375   → SILENCIO
paso-2 degenerado grados Tema [1,2,3,4,6,6,6,24]  mediana 4,0   tema 24: 24 ABOUT → ratio 6,0×   share 0,4615  → ALARMA
tras refactor A   grados Tema [1,2,3,4,5,5,5,6,6,6,15] mediana 5,0 tema 24: 15 → ratio 3,0×   share 0,23     → SILENCIO
```

El 24 del paso-2 es el 46% de las ABOUT (24/52); el 15 del refactorizado son 12 ABOUT directas + 3 SUB_TEMA_DE entrantes, y su share (12/52 = 0,23) cae bajo el 25%: el detector calla por AMBOS umbrales. Y la definición que ordena el glosario: **supernodo** = nodo cuyo grado concentra un desequilibrio relativo en su propio label — DISTINTO del supernodo de condensación del Vol.II cap. 5 (colapsar un SCC en un DAG) y del de contracción del cap. 25 (la agregación de Louvain): allí es estructura de cómputo, aquí es un **antipatrón de modelado** (anti-pattern).

Y el grafo del capítulo, degenerado vs refactorizado, lado a lado — el mismo fixture antes y después de la migración:

```text
ANTES (degenerado)                                 DESPUÉS (refactorizado)
(:Tema) 24 «grafos de conocimiento»                (:Conferencia 62/63/64) ◄─PUBLICADO_EN─ 24 docs
        ◄─ABOUT─ 24 docs  (supernodo)              (:Tema) 24 ──12 ABOUT directas──► docs paso-1+6
(:Tema) 57 «publicaciones 2024» ◄─ABOUT─ 6                │ SUB_TEMA_DE (134-136)
(:Tema) 58 «publicaciones 2025» ◄─ABOUT─ 6                ▼  ▼
(nodos categóricos de baja cardinalidad)          (:Tema) 59/60/61 ──4 ABOUT cada uno──► lote
(:Documento) ─REVIEWED_BY{nota}─ (:Persona)        (:Persona)─REALIZA─►(:Resena{nota,ronda})
(la reseña sin identidad: 2 rondas de Fabio              │  SOBRE ▼        CONTRARRESTA 66→65
 indistinguibles por significado)                 (:Documento)◄──────(:Resena r2)
(:Documento).conferencia = "ICDE 2024"
(un string que nadie expande)
```

El momento ¡ajá! perseguido: **el hub del cap-41 no era inocente por pequeño: era inocente por NO cruzar el umbral. El dataset no creció por accidente — alguien importó un lote sin preguntarle al detector; y cada expansión por ese nodo cobra el grado entero desde ese día.**

## 42.4 Primera solución

La primera solución — y la que todo el mundo aplica — es **la NO-solución**: importar el lote sin mirar y «seguir como siempre». `kb_lira_paso2_degrado()` hace exactamente eso: PARTE de `kb_lira_paso1()` (lo llama, NUNCA lo copia — determinismo heredado) y añade encima el lote importado, con cada antipatrón sembrado a propósito:

```rust
// cap42_antipatrones.rs — esqueleto de kb_lira_paso2_degrado()
pub fn kb_lira_paso2_degrado() -> MemoryStore {
    let mut s = kb_lira_paso1();
    // Personas del lote (30-32): Gaby · Hugo · Iris
    // Papers del lote (33-56): 24 Documento+Paper con conferencia:String
    // Temas degenerados (57-58): «publicaciones 2024» · «publicaciones 2025»
    // AUTHORED (64-87) · CITES internas (88-93) · ABOUT (94-117): 18 al tema 24
    // ABOUT a temas-año (118-129): 6 + 6
    // ── REVIEWED_BY (130-133): la reseña SIN identidad (antipatrón a pagar) ──
    for (id, persona, documento, nota) in [
        (130usize, ids::FABIO, ids::DOC_REVISION_PARES, 7i64), // ronda 1
        (131, ids::FABIO, ids::DOC_REVISION_PARES, 8),         // ronda 2
        (132, ids::CARLA, 36, 6),
        (133, GABY, 45, 9),
    ] {
        s.put_edge(arista(id, persona, documento, "REVIEWED_BY")
            .with_prop("nota", Value::Int(nota))).unwrap();
    }
    s
}
```

59 nodos, 134 aristas: 6 ABOUT del paso-1 + 18 del lote en el tema 24; dos Temas-año con 6 ABOUT cada uno; cuatro reseñas sin identidad — dos de ellas paralelas de Fabio sobre el Informe (nota 7 y nota 8, rondas que solo la nota distingue); y un `conferencia:String` en cada paper del lote, un string que nadie puede expandir. Y el detector — construido ANTES del refactor, como manda el orden de implementación — avisa:

```text
$ cargo test -p vol2-liradb --lib cap42 -- el_hub_del_paso2_degrado_es_candidato_a_supernodo

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
→ tema 24: grado_entrante 24, mediana_label 4,0, ratio 6,0× ≥ 5×, share 0,4615 ≥ 25% — ALARMA. ÚNICO candidato.
```

Importar sin mirar funciona exactamente un día: el día en que el detector pregunta por qué.

## 42.5 Sus límites

El detector alarma, pero NO cura: señala al culpable y no dice qué hacer. Y la no-solución tiene tres límites que el detector mismo delata pero no resuelve:

1. **Cada expansión desde el supernodo paga el grado entero.** P5 inversa («¿qué documentos tratan el tema 24?») lee las 24 ABOUT en UNA expansión: `ContandoStore` (cap-26) mide 1 `in_edges` + 24 `get_edge` = 25 lecturas por consulta. La tesis estructural: 24 aristas cobradas cada vez que alguien cruza el nodo — y si el tema tuviera 10M, la consulta pagaría 10M. Es el argumento del «tronco encontrando la hoja» de Allen (2020) con contadores.
2. **Los Temas-año y la conferencia-string no responden preguntas nuevas.** «¿Qué papers publicó el equipo en ICDE?» no tiene respuesta: un string no se expande — la lección P6 del cap-41 (naive 12 lecturas vs LPG 5) repetida a escala. Y «publicaciones 2024» como Tema duplica lo que `anio:Int` ya filtra (R6).
3. **La reseña de dos rondas es indistinguible.** Las dos `REVIEWED_BY` de Fabio sobre el Informe son paralelas: solo la nota las separa — el significado de «ronda» no vive en el grafo. «¿Cuántas rondas pasó el Informe?» no tiene respuesta; «¿qué reseña responde a cuál?» ni siquiera es una pregunta formulable.

La no-solución tiene, además, el límite temporal que abre este volumen: el lote de hoy tiene 24 papers; el de la semana que viene, 240 — y cada uno con su ABOUT al mismo tema. El coste de no migrar crece con el grado del supernodo: **12 aristas hoy es barato; 10M «cuando crezca» es la factura de este capítulo.**

## 42.6 Solución evolucionada

Tres refactors de modelo (refactor de modelo: transformación quirúrgica del grafo con red de seguridad), cada uno una función PURA — `refactor_*(&mut MemoryStore) -> InformeRefactor` — donde `InformeRefactor` cuenta `nodos_creados`, `nodos_borrados`, `aristas_creadas`, `aristas_borradas` y `lecturas`: la moneda son reescrituras (rewrites) + lecturas, jamás µs. Los tres como TRADE-OFF con precio, nunca como dogma:

**Refactor A — descomponer el supernodo.** Crea 3 subtemas intermedios (ids 59-61: «knowledge graphs», «GraphRAG», «memoria de agentes»), redistribuye 12 de las 18 ABOUT del lote (4+4+4, ids 137-148) y enlaza con 3 `SUB_TEMA_DE` (134-136, Tema→Tema). Semántica DECLARADA: «documentos del tema T» = directos ∪ descendientes (`documentos_del_tema_incluyendo_subtemas`, una profundidad) — la unión conserva el conjunto: 24 = 12 directas (6 paso-1 + 6 del lote que se quedan) + 12 vía subtemas. Precio: **3/0 nodos, 15/12 aristas, 25 lecturas** (1 `in_edges` + 24 `get_edge`).

**Refactor B — reificar la reseña.** El reto intermedio del cap-41 COBRADO como ejemplo canónico de reificación CORRECTA: 4 nodos `:Resena {nota:Int, ronda:Int}` (ids 65-68), `REALIZA` Persona→Resena (149-152), `SOBRE` Resena→Documento (153-156) y **`CONTRARRESTA`** Resena→Resena (157: la ronda 2 de Fabio, nota 8, contrarresta a la ronda 1, nota 7). El díptico con la MISMA regla: `Autoria` cayó en R5 (sin relaciones ni ciclo propios → reificación EXCESIVA); `Resena` sube en R2/R3 (la ronda 2 contrarresta a la ronda 1: relaciones propias y ciclo de vida → sin el nodo, la arista simple la PIERDE — reificación INSUFICIENTE). Precio: **4/0 nodos, 9/4 aristas, 1 lectura** (un único barrido). `REVIEWED_BY` residual = 0.

**Refactor C — conferencias/temas-año, las DOS direcciones.** `conferencia:String` → 3 nodos `:Conferencia` (62-64: ICDE 2024, SIGMOD 2024, VLDB 2025) + 24 `PUBLICADO_EN` (158-181); el borrado de la propiedad usa la mutación directa del campo público del `MemoryStore` (el trait no ofrece `remove_prop` y `put_node` rechaza duplicados — precedente cap-33, documentado en el código). Y los Temas-año 57-58 se BORRAN con sus 12 ABOUT (118-129): el año ya era `anio` (R6), el refactor no añade nada porque la propiedad YA era la buena. Precio: **3/2 nodos, 24/12 aristas, 38 lecturas** (24 `get_node` + 2 `in_edges` + 12 `get_edge`).

**El validador paso-2 por composición.** `validar_modelo_kb_lira_paso2` REUTILIZA `validar_modelo_kb_lira` del cap-41 sobre el MISMO store (el subgrafo paso-1 que sigue dentro sigue cumpliendo SU contrato) y añade las reglas nuevas: extremos de `REALIZA`/`SOBRE`/`CONTRARRESTA`/`PUBLICADO_EN`/`SUB_TEMA_DE`, toda `Resena` con `nota:Int`+`ronda:Int`, y `REVIEWED_BY` PROHIBIDA (cualquier residual es deuda del refactor B sin pagar). El hallazgo validador, contado con honestidad: el validador base trata los tipos nuevos como «tipo de arista desconocido» — **36 violaciones en el grafo refactorizado** (24 PUBLICADO_EN + 4 REALIZA + 4 SOBRE + 1 CONTRARRESTA + 3 SUB_TEMA_DE) — y el wrapper filtra exactamente los 6 tipos que gobierna; el subgrafo paso-1 queda sano: cero violaciones base. La composición es la semilla honesta del cap. 44: estas reglas, allí, serán constraints.

**Y la REGLA DE ORO:** nada de lo que respondían las 10 preguntas sobre el paso-1 puede cambiar. Se verifica con `las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor` (cada pregunta filtrada a ids < 30, respuestas IDÉNTICAS antes y después de A+B+C). El detalle de honestidad P8: el filtro por persona < 30 NO basta — Elena y Fabio SÍ son del paso-1 y publican en el lote (54-56 y 51-53) — así que el test usa el helper `publicaciones_por_persona_en_paso1`, que cuenta SOLO las `AUTHORED` con target < 30 en ambos stores. Las respuestas que crecen (P3: +2 citadores; P8: +3 personas) crecen por el DATASET, no por el refactor — y el test lo distingue.

Cada refactor, además, es una decisión con alternativa descartada — trade-off, no dogma: A descarta los clones `SAME_AS` estilo «Gaga» de Allen (pensados para identidades físicas duplicadas: exóticos y caros para temas) y los time-buckets (hubs por período: frontera de ingesta/temporalidad, caps. 43/45); B descarta mantener `REVIEWED_BY{nota}` (las dos rondas de Fabio seguirían paralelas e indistinguibles por significado) y reificar también `Autoria` (el cap-41 ya lo descartó con la misma prueba); C descarta mantener la conferencia-string (repetir la lección P6 del cap-41 a escala industrial) y conservar los Temas-año «por comodidad de filtrado» (el año ya paga `anio`, P10).

## 42.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap42_antipatrones.rs` (**2.710 líneas**, std puro, **28 tests**), cableada con dos líneas aditivas en `lib.rs`; el artefacto regenerable `datasets/kb-lira/paso-2/{nodes.csv (60 líneas), edges.csv (135 líneas)}` es la salida del builder degenerado — el dataset es lo que «importó el equipo»; el refactor es código que se re-ejecuta. CERO dependencias nuevas, CERO cambios en caps. 7-41, goldens intactos. Y **NO hay `[[bench]]`**: decisión #11 del contrato en una línea — la moneda son reescrituras, lecturas y conjuntos exactos, y cronometrar no sostiene ninguna tesis de este capítulo (espejo de la decisión #12 del cap-41).

Las piezas que sostienen el edificio (nombres exactos; la prosa solo muestra esqueletos — el código completo vive en el módulo):

```rust
pub fn kb_lira_paso2_degrado() -> MemoryStore;                     // 59 nodos, 134 aristas
pub const RATIO_MINIMO_VS_MEDIANA: f64 = 5.0;
pub const SHARE_MINIMO_POR_TIPO: f64 = 0.25;
pub struct SupernodoCandidato { pub nodo_id: NodeId, pub label: String,
    pub grado_entrante: usize, pub grado_saliente: usize, pub grado_total: usize,
    pub mediana_label: f64, pub ratio_vs_mediana: f64, pub share_del_tipo: f64 }
pub fn detectar_supernodos(store: &dyn GraphStore) -> Vec<SupernodoCandidato>;
pub struct InformeRefactor { pub nodos_creados: usize, pub nodos_borrados: usize,
    pub aristas_creadas: usize, pub aristas_borradas: usize, pub lecturas: usize }
pub fn refactor_a_descomponer_supernodo(store: &mut MemoryStore) -> InformeRefactor;
pub fn refactor_b_reificar_resena(store: &mut MemoryStore) -> InformeRefactor;
pub fn refactor_c_conferencias_y_temas_anio(store: &mut MemoryStore) -> InformeRefactor;
pub fn documentos_del_tema_incluyendo_subtemas(store: &dyn GraphStore, tema_id: usize) -> Vec<usize>;
pub fn validar_modelo_kb_lira_paso2(store: &dyn GraphStore) -> Result<(), Vec<Violacion>>;
pub fn la_migracion_completa(store: &mut MemoryStore) -> InformeMigracion;
```

El corazón del detector, en siete líneas reales — la mediana sin el outlier:

```rust
// cap42_antipatrones.rs — el umbral relativo (núcleo de detectar_supernodos)
let otros: Vec<usize> = grados.iter().filter(|(n, _)| *n != hub).map(|(_, g)| *g).collect();
let mediana = mediana_de(&otros);
let ratio = grado_max as f64 / mediana;
let share = share_del_tipo_dominante(hub, &incidentes_por_nodo_tipo, &totales_por_tipo);
if ratio >= RATIO_MINIMO_VS_MEDIANA && share >= SHARE_MINIMO_POR_TIPO { /* ALARMA */ }
```

## 42.8 Prueba de fuego

Primero el bucle rápido — salida REAL de `cargo test`:

```text
$ cargo test -p vol2-liradb --lib cap42

running 28 tests
test cap42_antipatrones::tests_antipatrones::el_hub_del_paso1_es_inocente_segun_el_detector ... ok
test cap42_antipatrones::tests_antipatrones::distribucion_de_grados_exacta_sobre_kb_lira ... ok
test cap42_antipatrones::tests_antipatrones::estructura_de_kb_lira_paso2_degrado_cuenta_y_etiquetas_exactas ... ok
test cap42_antipatrones::tests_antipatrones::el_hub_del_paso2_degrado_es_candidato_a_supernodo ... ok
test cap42_antipatrones::tests_refactor_a::refactor_descomponer_conserva_el_conjunto_de_p5 ... ok
test cap42_antipatrones::tests_refactor_a::la_semantica_de_p5_sobre_el_tema_padre_se_preserva_por_union ... ok
test cap42_antipatrones::tests_refactor_a::el_detector_ya_no_alarma_tras_el_refactor ... ok
test cap42_antipatrones::tests_refactor_b::refactor_reificar_resena_responde_rondas_y_contrarrestas ... ok
test cap42_antipatrones::tests_refactor_c::refactor_conferencias_convierte_propiedad_en_nodo_y_temas_anio_en_propiedad ... ok
test cap42_antipatrones::tests_validador_paso2::validador_paso2_acepta_el_modelo_refactorizado ... ok
test cap42_antipatrones::tests_validador_paso2::validador_paso2_rechaza_fixture_corrupto ... ok
test cap42_antipatrones::tests_regresion_preguntas_paso1::las_10_preguntas_del_paso1_no_cambian_sobre_el_subgrafo_paso1_tras_refactor ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_01_documentos_de_una_persona_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_02_autores_en_orden_de_firma_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_03_citas_en_ambas_direcciones_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_04_proyecto_y_afiliaciones_en_dos_saltos_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_05_temas_de_un_documento_e_inversa_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_06_menciones_a_una_entidad_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_07_copublicacion_entre_dos_personas_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_08_publicaciones_por_persona_contadas_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_09_temas_comunes_via_papers_sobre_paso2 ... ok
test cap42_antipatrones::tests_preguntas_paso2::pregunta_10_citas_recientes_que_tratan_un_tema_sobre_paso2 ... ok
test cap42_antipatrones::tests_coste_expansion::el_supernodo_cobra_el_grado_entero_en_cada_expansion_y_el_refactor_lo_reparte ... ok
test cap42_antipatrones::tests_migracion::la_migracion_completa_cuesta_reeescrituras_exactas ... ok
test cap42_antipatrones::tests_migracion::informe_migracion_reproducible_sobre_kb_lira ... ok
test cap42_antipatrones::tests_csv_paso2::csv_roundtrip_paso2_import_export_byte_a_byte ... ok
test cap42_antipatrones::tests_csv_paso2::csv_paso2_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap42_antipatrones::tests_csv_paso2::csv_paso1_intacto_tras_paso2 ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 736 filtered out
```

Veintiocho verdes; workspace entero en **948 ALL_GREEN** (920 + 28) con goldens intactos. Ahora la tabla del capítulo — salida REAL de `InformeMigracion` (contadores exactos, sin ni un µs):

```text
Migración completa de KB-Lira: paso-2 degenerado → modelo refactorizado
────────────────────────────────────────────────────────────────────────
Refactor A · descomponer supernodo     | 3 nodos (+3/-0) | 15 aristas (+15/-12) | 25 lecturas
Refactor B · reificar reseña           | 4 nodos (+4/-0) | 9 aristas (+9/-4) | 1 lecturas
Refactor C · conferencias / temas-año  | 3 nodos (+3/-2) | 24 aristas (+24/-12) | 38 lecturas
TOTAL                                  | 10 nodos (+10/-2) | 48 aristas (+48/-28) | 64 lecturas
────────────────────────────────────────────────────────────────────────
Impacto sobre las 10 preguntas del cap-41 (referencia: KB-Lira paso-1):
P1: idéntica
P2: idéntica
P3: idéntica
P4: idéntica
P5: jerárquica 24
P6: idéntica
P7: idéntica
P8: enriquecida 40
P9: idéntica
P10: idéntica
```

| Refactor | nodos +/− | aristas +/− | lecturas |
|---|---|---|---|
| A · descomponer supernodo | 3/0 | 15/12 | 25 |
| B · reificar reseña | 4/0 | 9/4 | 1 |
| C · conferencias/temas-año | 3/2 | 24/12 | 38 |
| **TOTAL** | **10/2** | **48/28** | **64** |

Tres lecturas obligatorias. **Primera: el triplete responde el gancho del cap-41 con números.** El hub inocente del paso-1 (6 ABOUT, mediana 2,0, ratio 3,0×, share 0,375) → SILENCIO; el mismo nodo tras el lote (24 ABOUT, mediana 4,0, ratio 6,0×, share 0,4615) → ALARMA, único candidato; tras el refactor A (15 = 12 ABOUT + 3 SUB_TEMA_DE, mediana 5,0, ratio 3,0×, share 0,23) → SILENCIO. Un hub es inocente mientras no cruza el umbral — ni antes ni después.

**Segunda: la tesis estructural, con su honestidad.** P5 inversa sobre el degenerado: UNA expansión paga el grado entero — `ContandoStore` mide 1 `in_edges` + 24 `get_edge` = 25 lecturas, reparto `[24]`. Tras el refactor A: las MISMAS 24 aristas en 4 expansiones — reparto `[12, 4, 4, 4]`, 4 `in_edges` + 27 `get_edge` = 31 lecturas (las 3 SUB_TEMA_DE extra que la unión cruza). El total es idéntico: 24 = 24 — el fixture no miente, el refactor NO reduce lecturas aquí, las REPARTE. El ahorro real aparece a escala: cuando el supernodo tenga 10M de ABOUT y una consulta cruce SOLO un subtema, pagará 4 lecturas en vez de 10M (y 4 × 6 = 24: la consulta de un solo subtema paga 1/6 del hub).

**Tercera: el impacto pineado y la frontera declarada.** P5 es «jerárquica 24»: la consulta directa de la pregunta ve 12 docs (6 paso-1 + 6 del lote que se quedan: 37-40, 45, 50) y la unión jerárquica recupera los 24 completos — UNA consulta se reescribe (coste de migración honesto). P8 «enriquecida 40»: 9 personas — Ana 4, Beto 3, Carla 2, Dani 2, Elena 6, Fabio 5, Gaby 6, Hugo 6, Iris 6 (16 AUTHORED del paso-1 + 24 del lote). Y la frontera UTF-8, contada con honestidad: P10 sobre el paper del lote 41 («GraphRAG: recuperación aumentada con grafos») devuelve **vacío** — el título del citado lleva «ó» y el lexer mini del cap-18 corrompe literales UTF-8 multi-byte (frontera ya documentada en el cap-41); el vacío está pineado con su comentario en el test, jamás escondido. El CSV cierra el círculo: `csv_roundtrip_paso2_import_export_byte_a_byte`, `csv_paso2_coincide_con_dataset_commiteado_byte_a_byte` y `csv_paso1_intacto_tras_paso2` — los ficheros del paso-1 ni se tocan.

## 42.9 Qué hemos sacrificado

1. **Sin valid-time.** La ronda de una reseña es propiedad simple `{nota, ronda}` — NO fecha de validez. «¿Qué valía la nota el 3 de marzo?» y el historial de afiliaciones son el cap. 43.
2. **Sin constraints UNIQUE ni índices.** El validador paso-2 SIEMBRA las reglas que allí serán constraints; aquí son convención ejecutable, no garantía del motor (cap. 44).
3. **Sin ingesta automatizada.** El lote se añadió a mano en el builder; el pipeline CSV/JSONL con dedup es el cap. 45.
4. **Sin RDF ni reificación de tripletas.** El contraste del cap-41 (todo atributo es un triplete) se despliega en el cap. 46.
5. **MENTIONS sigue polimórfico.** NO es antipatrón: el cap-41 lo justificó contra P6 y el validador lo suple; su especificidad por label es decisión de ESQUEMA (cap. 44). Refactorizar por estética sería la trampa que este capítulo enseña a no perseguir.
6. **Sin bench.** Ninguna afirmación depende de cronometraje (decisión #11): la moneda son reescrituras, lecturas y conjuntos exactos.

## 42.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **David Allen (Neo4j Developer Blog, 19-oct-2020)** escribió la guía industrial «Graph Modeling: All About Super Nodes»: la definición relativa del supernodo, sus causas (hub de dominio vs hub de modelado) y el toolbox de mitigación — direccionalidad y segregación de labels/aristas, join hints, y el refactoring del supernodo. La **Neo4j Knowledge Base** documenta los join hints contra travesías costosas (citado sin fecha: frontera declarada del contrato), y la **GraphAcademy (gdm-40)** define el supernodo como el nodo con mucho fan-in/fan-out. El extremo opuesto: **Aerospike Graph Service 3.0** trata el supernodo como ciudadano de PRIMERA CLASE — flag `~supernode`, listas de aristas multi-registro, ~6.500 aristas @1MiB de max-record-size — la prueba de que el umbral absoluto es configuración del motor, no ley de la naturaleza: tu detector relativo no necesita saber qué tamaño tiene el grafo ajeno.

Y la precisión terminológica que evita un error de examen: el **star schema** de Kimball (*The Data Warehouse Toolkit*, Wiley, 1996) NO es un hub-and-spoke. El star schema es tabla de hechos + dimensiones — otra familia, nacida para agregar, no para navegar; su diagrama se PARECE a un hub, y por eso la confusión. No lo llames «supernodo»: la moneda del modelado dimensional es distinta.

El espejo industrial de tu `Resena`: Robinson, Webber y Eifrem (*Graph Databases*, 2ª ed., O'Reilly, 2015, cap. 3 «Data Modeling with Graphs», sección «Avoiding Anti-Patterns», p. 63) narran el caso de **email forense** — modelado sobre el corpus de Enron para investigar intercambios de información tipo insider trading: la primera iteración `(Alice)-[:EMAILED]->(Bob)` leía bien y era lossy — el email concreto no existía como entidad; la cura fue el nodo intermedio `(Alice)-[:SENT]->(email)-[:TO]->(Bob)` con su CC/BCC. Tu `REVIEWED_BY{nota}` es su `EMAILED`: una entidad con identidad propia codificada como relación, que pierde al instante su historia.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (34+41+42): añade UN paper más al lote con su ABOUT al tema 24 en `kb_lira_paso2_degrado`. ANTES de correr nada, PREDICE por escrito el nuevo ratio y el nuevo share del detector (recuerda: la mediana excluye al hub y las ABOUT totales cambian) y si el detector alarma. Luego ejecuta `detectar_supernodos` y verifica tu predicción contra la salida real.
- *Intermedio* (24/25+42): aplica `detectar_supernodos` a otro label — p.ej. Persona (9 personas; Gaby, Hugo, Iris y Elena firman 6 papers del lote cada una) — y justifica POR ESCRITO un umbral propio: ¿por qué el 5× de los Temas no sirve para Personas? Conecta con `degree_centrality` del cap. 24 y la modularidad del cap. 25: ¿qué diría la MISMA distribución de grados desde esas dos divisas?
- *Experto* (39/40+42): migra el supernodo estilo «Gaga» — los clones con `SAME_AS` del toolbox de Allen — sobre el MISMO fixture: mide reescrituras de esa cura contra los subtemas del refactor A (recuerda el skew del hub en joins del cap. 39 y el vertex-cut del cap. 40: la MISMA enfermedad en otra divisa). Responde: ¿cuándo gana cada cura?

## 42.11 Lo que te llevas

- **El supernodo es RELATIVO, no un absoluto.** «10M de aristas» es el titular; el criterio es el desequilibrio contra la distribución del propio grafo (Allen 2020: «relative to what else is in the graph»).
- **El detector: umbral DOBLE.** ratio ≥ 5× la mediana del label Y share ≥ 25% del tipo, mediana EXCLUYENDO al propio hub, UN barrido de `iter_edges`. La AND defiende de falsas alarmas en grafos pequeños.
- **El triplete es el gancho cobrado.** 3,0× silencio → 6,0×·46% alarma → 3,0× silencio tras el refactor: un hub es inocente hasta que cruza el umbral — no por pequeño, por no cruzar.
- **Un antipatrón es una deuda que cobra en lecturas.** Cada expansión paga el grado entero (24 en UNA expansión); el refactor reparte (12+4+4+4) y el ahorro real aparece a escala: 4 lecturas vs 10M cuando una consulta cruza un solo subtema.
- **El refactor de modelo tiene precio y se mide.** Reescrituras + lecturas en `InformeRefactor`: migración completa 10/2 nodos, 48/28 aristas, 64 lecturas — la decisión se toma ANTES de que crezca porque el coste crece con el grado del supernodo.
- **La REGLA DE ORO.** Nada de lo que respondían las 10 preguntas sobre el paso-1 puede cambiar; si cambia, el refactor está mal — y las respuestas que crecen crecen por el DATASET, no por el refactor.
- **Reificar no es bueno ni malo: la escalera decide.** `Autoria` (excesiva) y `Resena` (insuficiente) con la MISMA regla: la prueba decide, no la estética.

## 42.12 Ojo, cuidado con…

- **Confundir el titular con el criterio.** Un nodo con 1.000 aristas puede ser inocente y uno con 30 culpable: lo decide la distribución de SU label, no el absoluto.
- **Perseguir limpieza.** MENTIONS polimórfico no es antipatrón; tocar lo que el cap-41 justificó contra P6 rompería la pregunta — la especificidad por label es esquema (cap. 44).
- **Refactorizar sin red de seguridad.** Si una respuesta vieja cambia, el refactor está MAL — prohibido «ajustar» la pregunta o el dataset para que cuadre.
- **Modelar el año como Tema «por comodidad de filtrado».** El año ya es `anio` (R6); el nodo categórico de baja cardinalidad es el `Gender` de Allen y los 176 campos de Boylan-Toomey: poca cardinalidad, muchísimo grado.
- **Dejar la reseña en la arista.** Dos rondas paralelas indistinguibles por significado son reificación INSUFICIENTE — tan antipatrón como la excesiva.
- **Creer que el refactor reduce lecturas en el fixture pequeño.** Aquí 24 = 24: el refactor REPARTE y paga 3 lecturas extra por las SUB_TEMA_DE; el ahorro aparece a escala. La honestidad también es didáctica.

## 42.13 Pin de batalla

> *«Un antipatrón no es una forma fea de modelar: es una deuda que cobra en lecturas cada vez que alguien cruza el nodo. El refactor es pagar la deuda ANTES de que el interés sea el dataset entero — y el detector es el portero que te dice cuándo: 5× la mediana y 25% del share, y la mediana nunca la fija el propio outlier.»*

## 42.14 Si solo lees 30 segundos

El supernodo (supernode) no es «un nodo con 10M de aristas»: es un desequilibrio RELATIVO en su propio grafo. Se detecta con UN barrido y dos umbrales: **ratio ≥ 5× la mediana del label Y share ≥ 25% del tipo de arista** (mediana excluyendo al hub). El triplete sobre KB-Lira: paso-1 silencio (6 ABOUT, ratio 3,0×), paso-2 alarma (24 ABOUT, ratio 6,0×, share 46% — un lote importado degeneró el modelo sin tocar la suite, que seguía en 920 verdes), tras el refactor silencio (15, ratio 3,0×). La cura son tres refactors puros con `InformeRefactor`: A descomponer el supernodo en subtemas (3/0 nodos, 15/12 aristas, 25 lecturas), B reificar la reseña (4/0, 9/4, 1 — la ronda 2 contrarresta a la ronda 1), C conferencias en nodo y temas-año en propiedad (3/2, 24/12, 38). Migración total: **10/2 nodos, 48/28 aristas, 64 lecturas**. REGLA DE ORO: las 10 preguntas del cap-41 responden idéntico sobre el paso-1 tras el refactor (test de nombre exacto). La tesis: el supernodo cobra el grado entero por expansión (24); el refactor reparte (12+4+4+4) y el ahorro real aparece a escala (4 lecturas vs 10M). 28 tests nuevos, workspace en **948 ALL_GREEN**, sin bench: la moneda son reescrituras y lecturas, no µs. Fronteras: la ronda es propiedad simple (cap. 43), el validador siembra constraints (cap. 44), el lote se importó a mano (cap. 45), MENTIONS sigue polimórfico (no es antipatrón).

## 42.15 Una historia pequeña

Houston, 2001. El corpus de correos de Enron —publicado por la FERC en 2003 como parte de la investigación— se convirtió en el estándar de facto del email forense. Y ahí, en la sección «Avoiding Anti-Patterns» de *Graph Databases* (Robinson, Webber y Eifrem, O'Reilly, 2ª ed., cap. 3, p. 63), los autores cuentan la historia que es el espejo exacto de tu reseña: un equipo modela correos investigando posibles intercambios de información para insider trading (piensa: Enron). Primera iteración: `(Alice)-[:EMAILED]->(Bob)`. Lee bien, suena perfecto — y era **lossy**: al pedir el email concreto que violaba la política, el email NO EXISTÍA; donde esperaban varios correos confirmando la actividad corrupta, solo veían que Alice y Bob se habían escrito varias veces. Habían codificado una entidad con identidad propia —el email, con su asunto, sus CC, sus respuestas— como una relación. La cura fue el nodo intermedio: `(Alice)-[:SENT]->(email)-[:TO]->(Bob)`, y de ahí los `CC`/`BCC` que solo un nodo puede tener. Los autores lo resumen en una frase que es la moraleja de este capítulo: «Don't (accidentally) encode entities as relationships». Tu `REVIEWED_BY{nota}` era su `EMAILED`: la reseña no existía, y por eso la ronda 2 no podía contrarrestar a la ronda 1 — el CC de tu investigación no existe sin el nodo.

## Ejercicios resueltos

**1. Retrieval sin pistas: recita el umbral del detector DE MEMORIA.** Cierra el libro. Ratio ≥ 5× la mediana del grado del label Y share ≥ 25% de las aristas del tipo dominante; la mediana se calcula EXCLUYENDO al propio hub (el outlier no fija su propia línea base); la AND de ambos umbrales es la defensa contra las falsas alarmas en grafos pequeños. El triplete: paso-1 3,0×/0,375 silencio; paso-2 6,0×/0,4615 alarma; refactorizado 3,0×/0,23 silencio. Si recitas solo el ratio o solo el share, vuelve a §42.3: el orden ES el argumento (sin el share, cualquier grafo diminuto alarma; sin el ratio, un hub legítimo con muchos vecinos parecería culpable).

**2. Clasifica 5 situaciones sin mirar.** (a) El nodo `:Conferencia` «ICDE 2024» con 8 PUBLICADO_EN entrantes tras el refactor C: **hub legítimo** — se resuelve por clave y no se atraviesa en caminos calientes (anchor). (b) El tema 24 del paso-2 (24 ABOUT, ratio 6,0×, share 46%): **supernodo**. (c) El nodo `Autoria{order}` del cap-41: **reificación excesiva** — R5: sin relaciones ni ciclo propios, añade un salto a P1-P2 sin comprar nada. (d) Las dos `REVIEWED_BY` paralelas de Fabio: **reificación insuficiente** — R2/R3: la ronda 2 contrarresta a la ronda 1, y sin el nodo la arista la pierde. (e) El Tema «publicaciones 2024»: **nodo categórico de baja cardinalidad** — R6: el año ya era `anio`.

**3. Explica por qué el refactor A no reduce lecturas en el fixture y dónde aparece el ahorro.** Mecánica: degenerado reparto `[24]` (25 lecturas con ContandoStore); refactorizado reparto `[12,4,4,4]` (31 lecturas: las 3 SUB_TEMA_DE extra que la unión cruza). El total es idéntico porque el fixture tiene las MISMAS 24 aristas — el refactor REPARTE, no elimina. El ahorro es estructural y aparece a escala: si una consulta cruza SOLO un subtema, lee 4 aristas en vez del grado entero del hub; con 10M de ABOUT, 4 lecturas vs 10M (y 4×6=24: 1/6 del hub). La decisión de refactorizar se toma ANTES de que crezca porque el coste de la deuda crece con el grado — reescribir 12 aristas hoy es barato; 10M «cuando crezca» es la factura.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 34+41+42).** Desarrolla el reto esencial del §42.10: añade un paper al lote con su ABOUT al tema 24, PREDICE por escrito el nuevo ratio, el nuevo share y el veredicto del detector ANTES de correr, y verifica contra la salida real. Criterio: predicción escrita primero; si tu predicción de share olvidó que las ABOUT totales cambian (52 → 53), revisa el cálculo de `share_del_tipo`.

**Intermedio (predecir; 24/25+42).** Aplica `detectar_supernodos` al label Persona y justifica por escrito un umbral propio: ¿qué distribución real ves (Ana 4, Elena 6, Gaby/Hugo/Iris 6 AUTHORED) y por qué el 5× de los Temas no le sirve? Relaciona con `degree_centrality` (cap. 24) y la modularidad (cap. 25): ¿dirían lo mismo las tres divisas sobre la misma distribución? Criterio: umbral justificado con números del grafo, no copiado.

**Experto (crear y medir; 39/40+42).** Implementa la cura «Gaga» (clones con `SAME_AS`) sobre el fixture del tema 24 y mide reescrituras con `InformeRefactor` contra el refactor A. Responde por escrito: (a) ¿cuándo gana cada cura? (b) ¿qué hace `SAME_AS` que los subtemas no pueden hacer (y al revés)? (c) ¿cómo se relaciona con el skew del join (cap. 39) y el vertex-cut (cap. 40)? Restricciones: std puro, suite ALL_GREEN con tu test dentro.

## Para profundizar

- **David Allen, «Graph Modeling: All About Super Nodes», Neo4j Developer Blog (Medium), 19-oct-2020** — la definición relativa del supernodo, causas (hub de dominio vs hub de modelado) y el toolbox de mitigación (direccionalidad, segregación, refactoring).
- **Neo4j GraphAcademy, «Graph Data Modeling Core Principles» (curso gdm-40)** — supernodo = nodo con mucho fan-in/fan-out.
- **Neo4j Knowledge Base, «How to Avoid Costly Traversals with Join Hints»** — join hints contra supernodos (sin fecha citada: frontera declarada del contrato).
- **Aerospike Graph Service 3.0, docs «Supernodes»** — el supernodo como ciudadano de primera clase: flag `~supernode`, listas de aristas multi-registro, ~6.500 aristas @1MiB de max-record-size.
- **Justin Boylan-Toomey, «Neo4j Super Node Performance Issues», blog, 26-feb-2024** — el KG académico de 100M publicaciones × 176 campos: de «casi un día» a segundos con campo → propiedad indexada (anécdota del §42.0).
- **Robinson, Webber y Eifrem, *Graph Databases*, 2ª ed. (O'Reilly, junio 2015), cap. 3 «Data Modeling with Graphs», sección «Avoiding Anti-Patterns» (p. 63)** — el caso de email forense sobre el corpus de Enron: entidad con identidad propia codificada como relación (historia pequeña y espejo de Resena); los autores lo relatan en la entrevista de InfoQ (may-2014).
- **Ralph Kimball, *The Data Warehouse Toolkit* (Wiley, 1996)** — el star schema como precisión terminológica: fact + dimensiones, NO hub-and-spoke.
- Dentro del libro: cap. 41 (escalera R1-R7, las 10 preguntas, validador, modelo naive), caps. 7-8 y 20 (GraphStore, travesías), cap. 24 (degree_centrality), cap. 25 (Louvain y sus supernodos de contracción), cap. 26 (ContandoStore), cap. 32 (CSV), cap. 34 (dataset determinista), caps. 39-40 (skew del hub en joins y vertex-cut), Vol. I caps. 3-4 (BFS).

## Mini-diálogo: en guardia nocturna

> — Son las tres de la madrugada. El equipo pide «todos los papers del tema *grafos de conocimiento*» y la expansión va lenta.
>
> — ¿Cuántas ABOUT tiene el tema 24?
>
> — Veinticuatro. Pero el detector no alarmó esta mañana… la mediana era 2,0 y el ratio 3,0×.
>
> — (pausa) ¿Y las ABOUT totales?
>
> — Cincuenta y dos. Veinticuatro de cincuenta y dos… un 46%. Eso es ≥ 25%.
>
> — Pero el ratio no llegaba a 5×, ¿verdad? La AND exige los dos. El hub del paso-1 era inocente por NO cruzar el umbral — con 6 ABOUT y mediana 2,0, ratio 3,0×. La pregunta es qué pasó ESTA noche: alguien importó 24 papers y 18 de sus ABOUT cayeron en el mismo tema. Ahora son 24 ABOUT, la mediana de los demás Temas es 4,0, y el ratio es 24/4 = 6,0×.
>
> — O sea que el detector…
>
> — El detector avisó hace horas: «ALARMA, tema 24, ratio 6,0×, share 46%, único candidato». La consulta lenta no es un problema del motor: es el cobro de la deuda — cada expansión paga las 24 aristas del grado entero. Descompón el tema en subtemas ANTES de que el lote de mañana traiga 240 papers y el interés sea el dataset entero. La suite está verde; el modelo, no.
>
> — ¿Y la reseña que Fabio rehizo ayer?
>
> — Esa la pagamos hoy con un nodo. Buenas noches.

> *Siguiente parada, cap. 43 (temporalidad): la ronda 2 contrarresta a la ronda 1 — pero ¿QUÉ valía el 3 de marzo? Preguntas que dejamos abiertas: ¿quién garantiza que no existan dos papers con el mismo título ni dos reseñas idénticas? (cap. 44, unicidad e índices — el validador paso-2 es su semilla). ¿Y quién importa el lote de mañana sin degenerar el modelo? (cap. 45, ingesta).*
