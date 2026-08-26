# Capítulo 41 — Modelar entidades, propiedades y relaciones

> *«Primer capítulo del Volumen III. Cerraste el Volumen II con un motor completo —920 tests verdes— y la promesa de que LiraDB respondería lo que le preguntaras. Te faltó una palabra: LiraDB responde lo que le preguntes a un MODELO que dibujas ANTES de preguntar, y el cierre del cap. 40 te dejó una deuda clavada: "¿y cuando la MEMORIA de un agente necesita un grafo?" Esta es su primera piedra: la KB-Lira, la base de conocimiento del equipo de investigación que recorrerá este volumen entero hasta convertirse en memoria de agente (cap. 53). Si vienes del Volumen II (o entras por la ruta "arquitecto" del prólogo, con perfil de datos/IA), el Property Graph (Vol. II, cap. 7), el store (Vol. II, cap. 8), LiraQL (Vol. II, caps. 17-21) y el CSV (Vol. II, cap. 32) son tus herramientas, no tu contenido: aquí el motor ya existe y se USA.»*

## 41.0 La anécdota de la esquina

2013. Robinson, Webber y Eifrem publican *Graph Databases* (O'Reilly; 1ª edición 2013, 2ª edición junio 2015) y popularizan el **property graph** como la forma NATURAL de pensar en datos conectados: cosas con identidad, unidas por relaciones con nombre, cada una con sus atributos. Cypher nace atado a ese modelo dentro de Neo4j; luego la especificación openCypher lo desata del vendor y la industria lo adopta en masa —en SIGMOD 2018, Francis et al. cuentan la historia de «Cypher: An Evolving Query Language for Property Graphs» (pp. 1433-1445, DOI 10.1145/3183713.3190657)— hasta que en 2024 el modelo y su lenguaje se estandarizan de una vez: **GQL**, ISO/IEC 39075:2024, el primer estándar ISO para consultar grafos. Once años para que una idea de modelado recorriera O'Reilly → openCypher → ISO.

Tu equipo de investigación no tiene ese problema: tiene papers, notas, informes, personas, organizaciones, proyectos y temas — y quiere PREGUNTARLE cosas a ese material: quién escribió qué, quién cita a quién, qué temas conectan a dos personas. Toda la historia de la anécdota depende de una decisión que nadie te va a ahorrar: antes de preguntar nada, tienes que decidir **qué ES cada cosa**. Esta es la pregunta que abre el volumen: ¿cuándo algo es un nodo (node), cuándo una relación (relationship) y cuándo una simple propiedad (property)?

## 41.1 Objetivo

Este capítulo abre el Vol.III y la Parte I «Modelar datos de grafos» cobrando la deuda del cierre del Vol.II: el motor ya está, ahora hay que decidir QUÉ modelar y POR QUÉ. Al terminar tendrás:

1. **La escalera R1-R7**: criterios REPRODUCIBLES nodo vs arista vs propiedad — la respuesta formal a la pregunta del corpus «más allá del instinto». Verificación: la recitas DE MEMORIA y clasificas 5 frases nuevas (§41.3 y ejercicios).
2. **Query-first**: las 10 preguntas previstas de la KB-Lira escritas ANTES de dibujar un solo círculo; cada pieza del modelo cita la pregunta que la paga. Verificación: cada gesto de §41.6 termina con la pregunta que lo paga.
3. **La KB-Lira paso-1 real**: `kb_lira_paso1()` construye un `MemoryStore` determinista (30 nodos, 64 aristas, ids FIJOS a mano, disciplina del cap. 34) con labels múltiples, aristas con propiedades (`order`), destino polimórfico y un mini-hub sembrado a propósito. Verificación: `estructura_de_kb_lira_paso1_cuenta_y_etiquetas_exactas` cuenta nodos/aristas por label a mano.
4. **Un validador de modelo COMO CÓDIGO** (`validar_modelo_kb_lira` → `Violacion` con ids): tipos de extremos y campos obligatorios convertidos en test ejecutable — semilla de los constraints del cap. 44 y los shapes del cap. 47. Verificación: acepta el modelo canónico y rechaza un fixture roto a mano.
5. **Un modelo ingenuo comparable** (`ModeloDocsTodopropiedades`) con contador de LECTURAS, y la tesis medida: P6 cuesta **12 lecturas escaneando vs 5 expandiendo**. Verificación: `naive_y_lpg_responden_menciones_con_costes_distintos`.
6. **Las 10 preguntas respondidas con tests** (`pregunta_01…` … `pregunta_10…`): LiraQL donde el lenguaje llega, API directa donde no — frontera declarada, no escondida. Verificación: los 10 tests de nombre exacto, salidas reales en §41.8.
7. **CSV determinista** reutilizando el formato del cap. 32, con round-trip byte a byte y el artefacto `datasets/kb-lira/paso-1/` commiteado. Verificación: `csv_roundtrip_import_export_byte_a_byte` y `csv_coincide_con_dataset_commiteado_byte_a_byte`.
8. **920 tests ALL_GREEN** (901 + 19), cero dependencias nuevas, cero cambios en caps. 7-40, **sin bench** (decisión #12 del contrato).

## 41.2 Problema

Mira tu suite: **920 tests verdes**. El motor parsea, optimiza, ejecuta, persiste, distribuye. Y sin embargo, una pregunta trivial —«¿qué papers citó Ana?»— NO tiene respuesta. No porque el motor sea malo: porque NADIE decidió aún qué es un nodo. Si «cita» es un string dentro de la propiedad `citas`, el motor no tiene dónde expandir; si «Ana» no es una entidad con identidad, no hay desde dónde partir. El motor responde; el MODELO decide si hay pregunta que responder.

Antes de dibujar nada, desactivemos las cinco ideas equivocadas que suelen venir con el tema:

1. **«Todo sustantivo del requisito es una entidad.»** No: «la fecha de publicación» pasa todas las pruebas de propiedad. El sustantivo NO decide; la prueba de la **identidad propia** SÍ.
2. **«Las propiedades son gratis.»** El coste está en CONSULTAR: una lista `"Ana;Beto"` como string mata búsquedas inversas y travesías — nadie expande una string.
3. **«Primero el modelo completo, luego las consultas.»** Al revés: sin preguntas previstas no hay árbitro para decidir nodo vs propiedad. Esto es **query-first**, y es la religión de este capítulo.
4. **«Un grafo dirigido no representa simetrías.»** Sí las representa: con expansión undirected o aristas duales. La dirección vive en la SEMÁNTICA (citar no es ser citado; co-publicar sí es simétrico).
5. **«Reificar siempre es más correcto.»** Reificar (convertir un vínculo en nodo) sin necesidad rompe la lectura del grafo y multiplica saltos. Aquí solo aprendes la REGLA; los antipatrones completos son el cap. 42.

Y el compromiso de honestidad que rige TODO el capítulo: si alguna de las 10 preguntas NO se responde con el modelo elegido, se REPORTA tal cual y el modelo se refactoriza DENTRO del capítulo usando la escalera — esa es la lección (el modelo se justifica contra las consultas), no un fallo que esconder. Igual si naive vs LPG no mostrara diferencia de lecturas: se explica POR QUÉ, prohibido inflar.

Y la pregunta crítica del corpus, con criterio de aceptación medible: «criterios reproducibles node vs edge vs property». Respuesta: la escalera R1-R7 aplicada frase a frase sobre KB-Lira, con las 10 preguntas testeadas ANTES del modelo — más el contraejemplo medido (naive escanea 12, LPG expande 5). Sin equivalencia testeada no hay capítulo: sería opinión de pizarra, no ingeniería.

## 41.3 Modelo mental: la escalera frase → decisión

Un solo panel ordena todo el capítulo: cada frase del requisito baja la escalera, y cada peldaño tiene una salida:

```text
FRASE DEL REQUISITO          PRUEBA DE LA IDENTIDAD PROPIA              DECISIÓN
«Ana escribió el paper P»    Ana: se nombra sola, se relaciona sola  → NODO (:Persona)
                             "escribió": vive solo del par Ana-P     → ARISTA (:AUTHORED)
                             orden de firma: atributo del vínculo    → PROP. de la arista
«publicado en 2023»          2023: sin relaciones ni vida propia     → PROPIEDAD (anio)
```

Esa prueba de la identidad propia, FORMALIZADA como escalera de siete peldaños —memorízala, te la pediré DE MEMORIA—:

```text
R1  identidad          → ¿se nombra por sí misma? (¿tiene nombre propio o es un valor?)
R2  relaciones propias → ¿se relaciona fuera de su contenedor? (¿alguien la cita, la firma…?)
R3  ciclo de vida propio→ ¿nace/cambia/muere por su cuenta?   (≥2 SÍ ⇒ NODO candidato)
R4  cardinalidad       → ¿puede aparecer VARIAS veces? (si sí, no cabe como propiedad única)
R5  vínculo            → relación entre dos nodos sin vida fuera del par ⇒ ARISTA
                          (+props si tiene atributos del vínculo; nodo intermedio SOLO si
                           gana relaciones/ciclo propios — reificación MÍNIMA)
R6  valor simple       → ¿filtrable sin identidad ni relaciones? ⇒ PROPIEDAD
R7  árbitro            → ante empate, GANA la consulta prevista
```

Debajo, el bloque sí/no que resume la lógica de negocio:

```text
¿Se nombra sola?  ¿Se relaciona fuera?  ¿Tiene vida propia?   → 2+ SÍ   ⇒ NODO
¿Es atributo de un vínculo entre dos nodos?                   →         ⇒ ARISTA (+props)
¿Es un valor simple que solo se filtra?                       →         ⇒ PROPIEDAD
¿EMPATE?                                                      →         ⇒ GANA LA CONSULTA (R7)
```

Y el grafo que este capítulo construye —dibujado antes de escribir una línea de Rust— con su hub sembrado a propósito:

```text
        AUTHORED{order}            CITES                MENTIONS (destino polimórfico)
(:Persona) ────────────────► (:Documento) ────────────► (:Documento)
    │                          │      │                  también → :Persona/:Organizacion/:Proyecto
    │ MEMBER_OF                ▼      ▼ ABOUT
    ▼                      (:Tema) ◄─┘
(:Organizacion)             hub NATURAL: el tema «grafos de conocimiento» acumula
    ▲                       6 aristas ABOUT sin que nadie lo pidiera — semilla
    │ WORKED_ON             deliberada del cap. 42 (antipatrones)
(:Proyecto)
```

Y la regla de oro heredada del cap. 34: dataset determinista (ids y datos FIJOS en el builder, nada de RNG), contadores de TRABAJO (lecturas) dentro de los tests, y CERO tiempos de pared — aquí la física es ESTRUCTURAL: qué se puede preguntar y a qué coste de lecturas, no cuánto tarda. La frontera, declarada antes de codificar: la capa educativa vive en un módulo propio (como los caps. 38-40); ni `GraphStore` ni el executor se tocan, y NO se extiende el parser de LiraQL para «que pasen» las preguntas — si una no cabe, API directa y frontera declarada. El momento ¡ajá! perseguido: «llevas dos volúmenes construyendo MOTORES que responden consultas; la mitad de las respuestas malas no son del motor sino del MODELO: lo que dibujaste como propiedad jamás podrá cruzarse, y lo que dibujaste como nodo te cobra travesías para siempre».

## 41.4 Primera solución

La versión que todo el mundo escribe primero, en la pizarra y en el código: **el modelo ingenuo todo-en-propiedades**. Cada documento es una fila; autores, citas, temas y menciones son strings separados por `;`:

```rust
// Esqueleto de `DocTodopropiedades` (cap41_modelado.rs)
pub struct DocTodopropiedades {
    pub id: usize,
    pub titulo: String,
    pub anio: i64,
    pub autores: String,   // "Ana;Beto" — en ORDEN de firma (única gracia que conserva)
    pub citas: String,     // "Consultas declarativas…;Índices…"
    pub temas: String,     // "grafos de conocimiento;memoria de agentes"
    pub menciones: String, // "Instituto Neurónica;Elena"
}
```

En la pizarra FUNCIONA. P1 («¿qué documentos ha escrito Ana?») se responde con un split: recorre las filas, parte `autores` por `;`, compara. El orden de firma incluso se conserva — el string es una lista ordenada. El esqueleto de la consulta ingenua (contador de lecturas incluido) es todo lo que necesita:

```rust
// Esqueleto de `menciones_de` (cap41_modelado.rs) — P6 versión ingenua
pub fn menciones_de(&mut self, nombre: &str) -> Vec<String> {
    let mut respuesta = Vec::new();
    for doc in &self.docs {
        self.lecturas += 1; // acceso al registro del documento
        let mencionado = doc.menciones.split(';').any(|m| m.trim() == nombre);
        if mencionado { respuesta.push(doc.titulo.clone()); }
    }
    ordenar_natural(&mut respuesta);
    respuesta
}
```

`ModeloDocsTodopropiedades::desde_store` aplasta el `MemoryStore` a filas (la construcción no cuenta lecturas; el duelo es en tiempo de CONSULTA), y su `menciones_de(nombre)` devuelve los títulos que mencionan a una entidad. Parece un modelo perfectamente razonable. Esa es la trampa.

## 41.5 Sus límites

El contador de lecturas delata la mentira. `ModeloDocsTodopropiedades` lleva un ledger: cada consulta suma 1 por cada registro que examina. P6 («¿qué documentos mencionan al Instituto Neurónica?») bajo el modelo ingenuo:

```text
P6 naive:  escanea TODOS los documentos (12) → 12 lecturas
P6 LPG:    expande SOLO el grado entrante de MENTIONS del nodo destino → 5 lecturas
```

Misma respuesta exacta (`naive_y_lpg_responden_menciones_con_costes_distintos` lo exige: los resultados son idénticos) — el modelo ingenuo FUNCIONA… de momento. Pero el coste es ESTRUCTURAL: sin nodo destino no hay dónde expandir, así que toca mirar todas las filas siempre. Y el escaneo no sabe rendirse: preguntar por «Nadie» paga igual los 12 registros completos. Peor aún, las otras preguntas se vuelven IMPOSIBLES o grotescas SIN parsear strings FUERA del store:

- **P2** («¿quién firma el paper X y en qué orden?») es trivial… solo si el string conservó el orden y solo para el string que ya tienes. Pero la INVERSA —«¿en qué papers firma Dani en segundo lugar?»— exige partir la lista de TODOS los documentos. Grotesca.
- **P3** («¿quién cita AL paper X?») es la condena: la lista de citas vive en el documento CITANTE, así que la inversa obliga a escanear todas las filas buscando el título dentro de un string. Sin parsear fuera del store, no hay respuesta.
- **P9** («¿qué temas conectan a Ana con Beto vía sus papers?») es directamente un sueño: necesita cruzar dos listas de autores contra dos listas de temas, a mano, en Rust, por cada par de filas. Nadie expande una string: el split es un testamento que firmas cada noche.

La lección no es «el split es lento»: es que **el modelo sin nodos no tiene puertas** — y una consulta es, literalmente, un camino por puertas.

## 41.6 Solución evolucionada

Aplicamos la escalera R1-R7 pieza a pieza. Cada gesto cita la pregunta que lo paga:

**Gesto 1: los nodos con identidad propia (R1-R3).** «Ana escribió el paper P»: Ana se nombra sola, se relaciona con documentos, proyectos y organizaciones — 3 SÍ ⇒ nodo `:Persona`. El paper se cita, se firma, tiene año: nodo `:Documento`. El año, en cambio, no se relaciona con nadie: PROPIEDAD (`anio:Int`). La fecha de publicación pasa las tres pruebas de propiedad — ahí muere la misconcepción #1. Son los ids FIJOS 0-5 (personas), 6-8 (organizaciones), 9-11 (proyectos), 12-23 (documentos), 24-29 (temas): determinismo total, cualquier test o prosa los cita literalmente.

**Gesto 2: la arista semántica (R5).** «escribió» vive solo del par Ana-P: ARISTA `AUTHORED` Persona→Documento. La dirección es SEMÁNTICA (R9 del contrato): quien firma no es firmado. Y aquí aparece la segunda decisión grande:

**Gesto 3: la propiedad de ARISTA `order` (R5 + R6).** ¿Quién firma PRIMERO en «Supernodos: anatomía de un cuello de botella»? Beto (order 1) aunque Ana tenga id menor — el orden NO es del documento ni de la persona: es atributo DEL VÍNCULO. Cypher y el modelo property graph permiten propiedades sobre las relaciones (Francis et al., SIGMOD '18: las relaciones llevan propiedades), y eso resuelve P2 sin reificar: `AUTHORED{order:1}`. Contraste que te adelanta el cap. 46: en RDF todo atributo es un triplete independiente (W3C RDF 1.1, Recommendation, 25-feb-2014) — el orden exigiría un nodo intermedio por firma. Reificar aquí (un nodo `Autoria`) sería reificación EXCESIVA: añade un salto a P1-P2 sin comprar nada. **Reificación mínima**: nodo intermedio SOLO si el vínculo gana relaciones o ciclo de vida propios.

**Gesto 4: labels múltiples (R6/R7).** Los documentos tienen subtipos: Paper, Nota, Informe. Opción A: propiedad `type`. Opción B: labels múltiples `:Documento` + `:Paper`. El cap. 7 ya las soporta nativamente (cap07_modelo.rs:58), y la etiqueta despacha en el MATCH: `(p:Paper)` selecciona en el patrón, no filtra después. P10 lo cobra: la consulta «papers posteriores a 2023 que citan a P y tratan T» abre con `(:Tema)<-[:ABOUT]-(p:Paper)` — sin labels múltiples sería `WHERE type = "Paper"` y la discriminación bajaría al WHERE (decisión #6 del contrato).

**Gesto 5: MENTIONS polimórfico (R7 + honestidad del modelo).** «¿Qué documentos mencionan a Neurónica?» — Neurónica puede ser persona, organización O proyecto: `MENTIONS` con destino POLIMÓRFICO (→Persona|Organizacion|Proyecto). Esto es honesto con el LPG: el modelo NO garantiza tipos en los extremos (Angles-Gutierrez, 2018 — esquema abierto, *schema-optional*). Y en vez de esconderlo, lo convertimos en código:

**Gesto 6: el validador como convención ejecutable.** `validar_modelo_kb_lira(store)` recorre TODAS las aristas comprobando tipos de extremos (AUTHORED: Persona→Documento; CITES: Documento→Documento; ABOUT: Documento→Tema; MENTIONS: →Persona|Organizacion|Proyecto; MEMBER_OF; WORKED_ON), exige `order:Int` en toda AUTHORED y `titulo`+`anio` en todo Documento. Devuelve `Ok(())` o la lista COMPLETA de `Violacion` con el id implicado. El contrato del modelo deja de ser prosa: es un test que grita si alguien inserta un `CITES` hacia un Tema — la semilla directa de los constraints del cap. 44 y de los shapes del cap. 47.

**Gesto 7: la dirección como semántica (R7).** `CITES` es dirigida: quien cita no es citado. P3 se responde con DOS expansiones espejo — `out_edges` para «¿a quién cita X?», `in_edges` para «¿quién cita a X?» (las simetrías se LEEN, no se duplican; misconcepción #4 desactivada). Y el multigrafo queda demostrado con P7: Ana y Beto co-firmaron DOS documentos, y el par {Ana, Beto} soporta CUATRO `EdgeIds` distintos — aristas paralelas con identidad propia, lo que el cap. 7 llamó **multigrafo**. Lo que el motor rechaza es repetir la IDENTIDAD (`DuplicateEdge`), jamás repetir el par (cap07_modelo.rs:153, cap08_graph_store.rs:76).

**Gesto 8: las preguntas que se pagan solas.** Las piezas ya decididas responden las cuatro restantes sin un solo gesto nuevo: P4 («¿quién trabaja en Kira y dónde está afiliado?») es la travesía MIXTA de 2 saltos `(:Proyecto)<-[:WORKED_ON]-(:Persona)-[:MEMBER_OF]->(:Organizacion)` — dos aristas que ya existían, spacing puro de la expansión del cap. 20; P5 usa `ABOUT` en ambas direcciones (la inversa, la cara visible del hub sembrado); P8 solo exige contar FILAS del `ResultSet` sobre `AUTHORED` (frontera declarada: sin agregación en la gramática, conteo en Rust); y P9 recorre `Persona→Documento→Tema←Documento←Persona` porque cada salto de ese camino fue una decisión de modelado previa — el camino no se diseña, se descubre. Esa es la señal de que el modelo está bien: las consultas que NO diseñaste explícitamente siguen teniendo puertas.

Cada gesto tiene su alternativa descartada y su porqué en la prosa del código; el hilo conductor es uno solo: **una PROPIEDAD es un valor que nunca puedes expandir; una ARISTA es una puerta que siempre puedes cruzar — modelar es decidir qué puertas existen ANTES de que te pregunten por el camino.**

## 41.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap41_modelado.rs` (**1.668 líneas**, std puro, 19 tests), cableada con dos líneas aditivas en `lib.rs` (`pub mod cap41_modelado; pub use cap41_modelado::*;`). CERO dependencias nuevas, CERO cambios en caps. 7-40, goldens intactos. Y la desviación DECLARADA del patrón caps. 34/38/39: **NO hay `[[bench]]`** — decisión #12 del contrato. Ninguna afirmación de este capítulo es sobre tiempo: las respuestas son CONJUNTOS y CONTADORES exactos que viven en los tests; cronometrar el builder no sostiene ninguna tesis. Aquí la moneda son enteros, no µs.

Las piezas que sostienen el edificio (nombres exactos, la prosa solo muestra esqueletos — el código completo se incluye desde el módulo, nunca se duplica):

```rust
pub fn kb_lira_paso1() -> MemoryStore;                    // 30 nodos, 64 aristas, ids FIJOS
pub struct Violacion { pub descripcion: String, pub id_implicado: usize, pub tipo_elemento: &'static str }
pub fn validar_modelo_kb_lira(store: &dyn GraphStore) -> Result<(), Vec<Violacion>>;
pub struct ModeloDocsTodopropiedades { pub docs: Vec<DocTodopropiedades>, pub lecturas: u64 }
pub fn menciones_lpg_con_coste(store: &dyn GraphStore, nombre: &str) -> (Vec<String>, u64);
pub fn pregunta_01_documentos_de_una_persona(store: &dyn GraphStore, persona: &str) -> Vec<String>;
pub fn pregunta_02_autores_en_orden_de_firma(store: &dyn GraphStore, titulo: &str) -> Vec<(String, i64)>;
// … pregunta_03 … pregunta_10 (misma casa, mismas firmas)
pub fn csv_nodos_kb_lira(store: &dyn GraphStore) -> String;   // formato del cap. 32
pub fn csv_aristas_kb_lira(store: &dyn GraphStore) -> String;
pub fn informe_modelado_reproducible(store: &MemoryStore) -> String; // enteros, NO µs
```

Cuatro decisiones visibles en esas firmas, con su porqué:

- **El contador de lecturas es la moneda del capítulo.** `ModeloDocsTodopropiedades.lecturas` y el ledger de `menciones_lpg_con_coste` miden trabajo ESTRUCTURAL — contadores reproducibles en CI, jamás cronómetros (decisión #12; metodología del cap. 34). La construcción del modelo naive NO cuenta lecturas: el duelo es en tiempo de CONSULTA, ahí es donde el modelo cobra lo suyo.
- **Los nombres de los tests SON el contrato.** `pregunta_01_documentos_de_una_persona` … `pregunta_10_citas_recientes_que_tratan_un_tema` copian la lista exacta del contrato §3: si una pregunta no tiene test, el capítulo no la responde — la prosa no puede inventar lo que la suite no demuestra.
- **El CSV es un ARTEFACTO, no una fuente.** `datasets/kb-lira/paso-1/` ES la salida de los builders, regenerable byte a byte; el test de coincidencia con lo commiteado grita si alguien olvida regenerarlo. La fuente única de verdad es `kb_lira_paso1()`.
- **El informe está pineado byte a byte por test.** Dos ejecuciones consecutivas deben ser idénticas (`assert_eq!(a, b)`): sin dataset determinista ni orden estable, la tabla que lees abajo sería folclore.

Y el artefacto regenerable `liradb-workspace/datasets/kb-lira/paso-1/{nodes.csv, edges.csv}` (31 + 65 líneas): SU contenido ES la salida de los builders — `csv_coincide_con_dataset_commiteado_byte_a_byte` grita si regeneras el builder y olvidas commitear.

## 41.8 Prueba de fuego

Primero el bucle rápido:

```text
$ cargo test -p vol2-liradb --lib cap41

running 19 tests
test cap41_modelado::tests_modelado::estructura_de_kb_lira_paso1_cuenta_y_etiquetas_exactas ... ok
test cap41_modelado::tests_modelado::validador_acepta_el_modelo_canonico ... ok
test cap41_modelado::tests_modelado::validador_rechaza_fixture_corrupto ... ok
test cap41_modelado::tests_modelado::naive_y_lpg_responden_menciones_con_costes_distintos ... ok
test cap41_modelado::tests_modelado::pregunta_01_documentos_de_una_persona ... ok
test cap41_modelado::tests_modelado::pregunta_02_autores_en_orden_de_firma ... ok
test cap41_modelado::tests_modelado::pregunta_03_citas_en_ambas_direcciones ... ok
test cap41_modelado::tests_modelado::pregunta_04_proyecto_y_afiliaciones_en_dos_saltos ... ok
test cap41_modelado::tests_modelado::pregunta_05_temas_de_un_documento_e_inversa ... ok
test cap41_modelado::tests_modelado::pregunta_06_menciones_a_una_entidad ... ok
test cap41_modelado::tests_modelado::pregunta_07_copublicacion_entre_dos_personas ... ok
test cap41_modelado::tests_modelado::pregunta_08_publicaciones_por_persona_contadas ... ok
test cap41_modelado::tests_modelado::pregunta_09_temas_comunes_via_papers ... ok
test cap41_modelado::tests_modelado::pregunta_10_citas_recientes_que_tratan_un_tema ... ok
test cap41_modelado::tests_modelado::aristas_paralelas_coautoria_visibles_como_multigrafo ... ok
test cap41_modelado::tests_modelado::citas_solo_apuntan_en_direccion_de_lectura ... ok
test cap41_modelado::tests_modelado::csv_roundtrip_import_export_byte_a_byte ... ok
test cap41_modelado::tests_modelado::csv_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap41_modelado::tests_modelado::informe_modelado_reproducible_sobre_kb_lira ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 901 filtered out; finished in 0.09s
```

Diecinueve verdes; workspace entero en **920 ALL_GREEN** (901 + 19) con goldens intactos. Ahora la tabla del capítulo — salida REAL de `informe_modelado_reproducible`, la misma que la prosa pega (contadores exactos, sin ni un µs):

```text
=== Informe de modelado reproducible (cap. 41) ===
dataset: KB-Lira paso-1 | 30 nodos | 64 aristas | validador: OK
-- preguntas previstas -> respuestas (contadores exactos; sin tiempos) --
P01 documentos_de_una_persona(Ana): [Grafos de conocimiento para agentes | Índices adaptativos para grafos | Notas de la reunión de arranque | Supernodos: anatomía de un cuello de botella] (4 filas)
P02 autores_en_orden(Memoria episódica en LLMs): [Carla#1, Dani#2] (2 filas)
P03 citas_bidireccionales(Supernodos…): sale=[Consultas declarativas sobre property graphs | Índices adaptativos para grafos] entra=[Recuperación aumentada con grafos]
P04 proyecto_y_afiliaciones(Kira): [Ana@Universidad de Lira, Beto@Instituto Neurónica, Dani@Instituto Neurónica]
P05 temas_e_inversa(Informe Kira | grafos de conocimiento): temas=[grafos de conocimiento] docs=[Consultas declarativas sobre property graphs | Grafos de conocimiento para agentes | Informe anual del Proyecto Kira | Notas de la reunión de arranque | Recuperación aumentada con grafos | Resumen del taller de GQL] (hub: 6 documentos)
P06 menciones(Instituto Neurónica): [Bitácora del experimento K-7 | Informe anual del Proyecto Kira | Informe de revisión por pares 2025 | Informe técnico del Proyecto Oráculo | Memoria episódica en LLMs] | coste naive: 12 lecturas vs LPG: 5 lecturas
P07 copublicacion(Ana,Beto): [Grafos de conocimiento para agentes | Supernodos: anatomía de un cuello de botella] (2 conjuntos)
P08 publicaciones_por_persona: [Ana:4, Beto:3, Carla:2, Dani:2, Elena:3, Fabio:2] (total 16 AUTHORED)
P09 temas_comunes_via_papers(Ana,Beto): [grafos de conocimiento | memoria de agentes | rendimiento]
P10 citas_recientes_que_tratan_un_tema(cita Grafos…, tema grafos…, >2023): [Recuperación aumentada con grafos (2025)]
(costes en lecturas/filas EXACTAS; sin cronómetro — decisión #12 del contrato)
```

| P | Pregunta | Respuesta real | Coste |
|---|---|---|---|
| 01 | ¿Qué documentos ha escrito Ana? | 4 docs: Grafos de conocimiento, Índices adaptativos, Notas arranque, Supernodos | 1 consulta |
| 02 | ¿Quiénes firman el paper X y en qué orden? | Carla#1, Dani#2 (Memoria episódica); Beto#1, Ana#2 (Supernodos) | API in_edges |
| 03 | ¿A quién cita X y quién cita a X? | sale=[Consultas declarativas, Índices adaptativos] entra=[Recuperación aumentada] | API out/in |
| 04 | ¿Quién trabaja en Y y su afiliación? | Ana@Univ. Lira, Beto@Neurónica, Dani@Neurónica | 1 consulta |
| 05 | ¿Temas del doc Z y documentos del tema T? | temas=[grafos de conocimiento] · docs=6 (el hub sembrado) | 2 consultas |
| 06 | ¿Qué documentos mencionan a esta entidad? | 5 docs mencionan Neurónica | **naive: 12 lecturas vs LPG: 5** |
| 07 | ¿Han co-publicado Ana y Beto? | 2 co-publicaciones (multigrafo) | 1 consulta |
| 08 | ¿Cuántas publicaciones por persona? | Ana:4, Beto:3, Carla:2, Dani:2, Elena:3, Fabio:2 (16 AUTHORED) | 1 consulta + conteo Rust |
| 09 | ¿Qué temas conectan a Ana con Beto? | grafos de conocimiento, memoria de agentes, rendimiento (3 temas; el camino admite d1==d2) | 1 consulta + dedup |
| 10 | ¿Papers >2023 que citan a P y tratan T? | Recuperación aumentada con grafos (2025) | 1 consulta |

Tres lecturas obligatorias. **Primera: la frontera del lenguaje, declarada con honestidad.** De las 10 preguntas, 7 son LiraQL puro (`run()`, caps. 18-21): P1, P4, P5, P7, P8 (conteo en Rust), P9 (dedup en Rust) y P10. Las otras tres —P2, P3, P6— se responden con API directa (`in_edges`/`out_edges`), y el motivo NO es capricho: al verificar el parser contra los datos reales descubrimos que `scan_string` del cap. 18 corrompe literales UTF-8 multi-byte («é»→«Ã©»), de modo que los filtros por títulos acentuados («Memoria episódica…», «Neurónica») NUNCA matchean el texto real del grafo. Es una frontera REAL del lenguaje del cap. 18, verificada en el código del lexer — se declara, se documenta y se trabaja alrededor; ocultarla sería trampa, contarla es honestidad.

**Segunda: los costes son estructurales, no cronometrados.** P6 es la tesis: misma respuesta en ambos modelos, y el naive paga 12 lecturas (escaneo de todos los documentos) contra 5 del LPG (expansión SOLO del grado entrante de MENTIONS del nodo destino — Neurónica tiene 5 MENTIONS entrantes; sus 2 MEMBER_OF entrantes ni se tocan, el ledger cuenta solo lo que la consulta LEE). Y P9 responde TRES temas, no dos: el camino `Persona→Documento→Tema←Documento←Persona` admite d1 == d2 — el paper 12 lo firman Ana y Beto a la vez y trata dos temas, así que ambos conectan por él. El test se corrigió a la salida REAL del motor, jamás al revés.

**Tercera: el orden natural.** Las respuestas viajan ordenadas con `clave_orden` (diacríticos→ASCII): el sort bytewise de Rust pondría «Índices» DESPUÉS de «Z» (0xC3 0x8D); con la clave natural, «Índices adaptativos» queda donde un lector lo espera, entre «G» y «N». El informe está pineado byte a byte: dos ejecuciones, idénticas (`informe_modelado_reproducible_sobre_kb_lira`).

Y la prueba de fuego cierra el círculo del problema con el que abrió el capítulo: «¿qué papers citó Ana?» YA tiene respuesta — P1 devuelve sus 4 documentos, P3 las citas en ambas direcciones, P9 los temas que comparte con Beto. El motor no cambió una línea: cambió el MODELO, y con él el espacio de preguntas respondibles. El validador sigue: `validador_acepta_el_modelo_canonico` exige `Ok(())` sobre KB-Lira, y `validador_rechaza_fixture_corrupto` rompe el grafo A MANO —CITES hacia un Tema, AUTHORED desde un Tema, MENTIONS hacia un Tema, un documento sin `anio`, un AUTHORED sin `order`— y exige violaciones con el id exacto de cada elemento. Y el CSV: `csv_roundtrip_import_export_byte_a_byte` exporta → escribe a fichero temporal → importa → exporta: **bytes idénticos**, y el grafo re-importado sigue siendo un modelo válido.

## 41.9 Qué hemos sacrificado

1. **Sin agregación en LiraQL.** P8 se responde contando FILAS del `ResultSet` desde Rust: la gramática mini tiene scan/expand/filter/project/cartesian/distinct/limit (caps. 19-20), y no tiene COUNT ni GROUP BY. Extender el parser para «que pasen» las 10 habría roto goldens y diluido el foco; el COUNT de verdad es contenido de otra parte (cap. 48).
2. **Sin tipos garantizados en los extremos de las aristas.** El LPG no los tiene (esquema abierto); el validador los SUPLE como convención ejecutable — el motor no te impedirá mañana insertar un `CITES` hacia un Tema, pero el test lo cazará en CI.
3. **Sin historia temporal.** «Ana trabaja en Kira desde marzo de 2024» es una propiedad de la arista hoy; pero el día que Ana cambie de proyecto, esa propiedad miente: cuando los HECHOS CAMBIEN (afiliaciones, notas, estados) necesitarás valid-time — el cap. 43 temporaliza la KB-Lira entera.
4. **Sin unicidad ni constraints.** Dos documentos con el mismo título pueden coexistir: sin `UNIQUE` no hay garantía (cap. 44).
5. **Sin RDF ni tripletas.** El contraste del `order` de arista te lo adelantó: en RDF todo atributo es un triplete independiente; traducir sin perder el alma es el cap. 46.
6. **Sin antipatrones.** El mini-hub Tema 24 acumula 6 aristas ABOUT «sin que nadie lo pidiera» — sembrado a propósito; refactorizarlo, sistematizar la reificación excesiva y los supernodos es EXACTAMENTE el cap. 42.

## 41.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j** documenta guías de modelado en sus docs oficiales con exactamente esta lógica: decide nodos y relaciones según las preguntas que vas a hacer (sus ejemplos canónicos —Persona, película, actuación— son query-first), y su modelo es *schema-optional*: puedes poblar sin declarar nada, igual que nuestro `MemoryStore`. El extremo contrario: **Kùzu** —el motor embebido de Waterloo— pidió desde su diseño declarar tablas de NODOS y de RELACIONES ANTES de insertar (Jin, Feng, Chen, Liu y Salihoğlu, «KÙZU Graph Database Management System», CIDR 2023, CC-BY 4.0; relato histórico según ADR-001: Waterloo → CIDR 2023 → Kùzu Inc. → adquisición por Apple (oct-2025) → repo archivado → forks comunitarios como **LadybugDB**). Donde Neo4j es esquema abierto y Kùzu/LadybugDB exige esquema, tu validador es el TÉRMINO MEDIO que este libro te enseña: convención ejecutable, sin motor que la imponga — y el cap. 47 la convertirá en shapes reales. El contraste llega hasta los extremos: el `MENTIONS` polimórfico que tu validador permite como convención, Kùzu no lo admite — cada tabla de relaciones se declara con UN par de nodos (una optimización de carga y compresión), así que el polimorfismo se paga con tablas separadas por par de extremos o con el experimental *rel group*: el esquema declarado cobra en vocabulario lo que el esquema abierto cobra en riesgo.

Y el preview que justifica todo este volumen: el pipeline **GraphRAG** industrial —extracción de entidades y relaciones con un LLM, deduplicación, grafo, comunidades (tu Louvain del cap. 25), resúmenes por comunidad y recuperación multi-hop— se construye ENTERO sobre un modelo. Si el modelo no distingue la entidad «Neurónica» del atributo «2025» o de la relación «menciona», el LLM extrae basura consistente y la recuperación produce caminos rotos. Por eso modelar importa para la IA: el modelo ES la respuesta comprimida a tus consultas — y para un agente, la memoria se modela igual que un corpus (cap. 51 lo construye, cap. 53 lo convierte en memoria de agente).

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (34+41): añade una persona nueva («Gaby») y un paper nuevo al builder `kb_lira_paso1()` (ids 30 y 31, respetando la disciplina de ids fijos). ANTES de correr nada, PREDICE por escrito qué devolverán P1 y P3 para Gaby y el paper nuevo, y si el validador seguirá pasando. Luego ejecuta: verifica las predicciones, y si el validador grita por el paper sin `anio`, explica QUÉ término de la escalera olvidaste.
- *Intermedio* (7+41): modela «Reseña»: una persona reseña un documento con una nota numérica. ¿Arista `REVIEWED_BY{nota}` o nodo intermedio `Reseña`? Decide con la PRUEBA de la identidad propia y la escalera R1-R7: la consulta «documentos reseñados por Carla» y «reseñas con nota > 4» no compran un nodo; «evolución de la nota de X» sí exige ciclo de vida propio. Modela, justifica por escrito y testea tu consulta contra el builder.
- *Experto* (20+41): dale al modelo ingenuo un índice MANUAL (un `HashMap` entidad → lista de documentos). Mide P6: las lecturas bajan de 12 a 1. Y explica POR QUÉ el LPG sigue ganando: ¿qué pregunta nueva obligaría a construir otro índice a mano? ¿qué hace el LPG que ningún índice plano puede hacer (P9)? El coste, recuerda, es de DISEÑO, no de implementación.

## 41.11 Lo que te llevas

- **El sustantivo no decide: la prueba de la identidad propia SÍ.** R1-R3 (identidad, relaciones, ciclo de vida): 2+ SÍ ⇒ nodo; atributo del vínculo ⇒ arista (+props); valor filtrable ⇒ propiedad; empate ⇒ gana la consulta prevista.
- **Las propiedades no son gratis: cuestan en CONSULTAR.** Medido: P6 bajo naive escanea 12 lecturas; el LPG expande 5 desde el nodo destino. Una string nunca se expande.
- **Query-first:** las 10 preguntas se escribieron ANTES que el primer círculo; cada arista de KB-Lira cita la pregunta que la paga. Sin árbitro, modelar es gusto personal.
- **La propiedad de ARISTA existe para eso:** `order` pertenece al vínculo, no a los extremos; reificar sin relaciones ni ciclo propios es reificación EXCESIVA (Francis et al., SIGMOD '18; contraste RDF, cap. 46).
- **La dirección es semántica:** CITES dirigida; las simetrías se leen con in_edges (P3). Y el multigrafo es real: el par {Ana, Beto} soporta 4 EdgeIds; lo prohibido es repetir IDENTIDAD, no repetir hecho.
- **El validador convierte la convención en test:** el LPG no garantiza tipos (esquema abierto); `validar_modelo_kb_lira` sí — semilla de constraints (44) y shapes (47).
- **El motor estaba verde y no respondía «¿qué papers citó Ana?»:** la mitad de las respuestas malas no son del motor, son del MODELO.

## 41.12 Ojo, cuidado con…

- **Escuchar la voz del sustantivo.** «La fecha de publicación» suena a cosa y es una propiedad: corre la escalera, no el diccionario.
- **Aplastar listas en strings «por ahora».** El split responde lo que ya sabes y condena lo que preguntarás: P3 inversa obliga a escanear todas las filas. El `;` es un testamento que firmas cada noche.
- **Reificar por estética.** Un nodo `Autoria` «más correcto» añade un salto a P1-P2 sin comprar nada: reificación MÍNIMA, y la prueba decide (el caso Reseña del reto intermedio).
- **Confundir paralelas con duplicados.** El motor rechaza ids repetidos, no hechos repetidos: dos AUTHORED entre el mismo par SON el multigrafo, no un error.
- **Pasar por alto la frontera del lenguaje.** El lexer del cap. 18 corrompe UTF-8: los filtros por títulos acentuados no matchean en LiraQL. Frontera declarada — API directa y documentación, jamás silencio.
- **Dejar el modelo sin validador.** Un `CITES` hacia un Tema puede vivir años en silencio; el validador lo caza en CI desde el día uno.

## 41.13 Pin de batalla

> *«Una PROPIEDAD es un valor que nunca podrás expandir; una ARISTA es una puerta que siempre podrás cruzar. Modelar es decidir qué puertas existen ANTES de que te pregunten por el camino — y cuando dudes, gana la consulta prevista.»*

## 41.14 Si solo lees 30 segundos

Modelar es responder «qué ES cada cosa» con una escalera de 7 peldaños: identidad, relaciones y ciclo de vida propios (≥2 SÍ ⇒ **nodo**); atributo de un vínculo ⇒ **arista** con propiedades (`order` en AUTHORED); valor simple filtrable ⇒ **propiedad**; empate ⇒ gana la consulta prevista (**query-first**: las 10 preguntas se escriben ANTES del modelo). El modelo ingenuo todo-en-propiedades (autores:"Ana;Beto") funciona en la pizarra y muere al preguntar: P6 mide **12 lecturas escaneando vs 5 expandiendo**, y P3/P9 exigen parsear strings fuera del store. La solución es la KB-Lira paso-1: 30 nodos (ids fijos), 64 aristas (16 AUTHORED, 10 CITES, 16 ABOUT, 10 MENTIONS, 6 MEMBER_OF, 6 WORKED_ON), labels múltiples `:Documento`+`:Paper/Nota/Informe`, destino polimórfico en MENTIONS y un validador que suple la ausencia de tipos — 19 tests verdes, workspace en **920 ALL_GREEN**, sin bench: la moneda son conjuntos y contadores, no µs. Frontera declarada: 7 preguntas en LiraQL, 3 en API directa porque el lexer del cap. 18 corrompe los acentos (un límite REAL del lenguaje, contado con honestidad). El tema «grafos de conocimiento» acumula 6 ABOUT sin que nadie lo pidiera: el mini-supernodo sembrado que el cap. 42 refactorizará.

## 41.15 Una historia pequeña

Bélgica, 1990. Marc Gyssens, Jan Paredaens y Dirk Van Gucht publican en PODS «A Graph-Oriented Object Database Model» (pp. 417-424): el modelo **GOOD**, donde el esquema y los datos SON grafos y toda operación es una transformación de grafo. Fue influyente —de él descienden GMOD, GOAL, G-Log (el lenguaje declarativo de Paredaens, Peelman y Tanca, 1995) y GDM— y, como documentaría el propio survey de Angles y Gutierrez en ACM Computing Surveys 40(1), 2008, esa primera ola de modelos de datos de grafos «despegó en los años ochenta y principios de los noventa… y su influencia se apagó gradualmente» con los modelos orientados a objetos, semiestructurados y XML. El abstract lo dice con una frase que es la lección de este capítulo: cuando la necesidad de gestionar información de naturaleza gráfica VOLVIÓ, el área recuperó su relevancia — y Angles y Gutierrez formalizaron diez años después el property graph en «An Introduction to Graph Data Management» (Springer LNCS 10000, 2018; arXiv:1801.00036). El modelo que usas hoy no nació con Neo4j: es la SEGUNDA ola de una idea que la industria dejó morir una vez. Formalizar el modelo antes de que el hype lo bautice es lo que permite que sobreviva — y formalizar ES esto: decidir qué es cada cosa, con una escalera, no con el instinto.

## Ejercicios resueltos

**1. Clasifica 5 frases NUEVAS con la escalera, sin mirar.** (a) «La revisión por pares del paper R terminó el 4 de marzo de 2025»: el paper es nodo (R1-R3); «revisión por pares» es arista entre Paper y Persona (R5); «el 4 de marzo» es atributo DEL VÍNCULO → propiedad de la arista (R6). (b) «Nuestro equipo trabaja en el Proyecto Brújula desde 2023»: proyecto nodo; trabaja → arista WORKED_ON; «desde 2023» → propiedad de la arista — y si mañana preguntan por el HISTORIAL de proyectos, ese «desde» se reifica (cap. 43). (c) «El documento X fue mencionado 14 veces en 2025»: ojo, la trampa del contador — 14 menciones son 14 aristas MENTIONS reales, no una propiedad «14»; el contador solo sería propiedad si las menciones individuales jamás se consultan (R7). (d) «Ana y Beto se conocieron en la conferencia GQL 2024»: si ninguna pregunta prevista cruza conferencias, quizá ni merece existir en el grafo (R7, wisdom); y «conocerse» sin preguntas es una puerta sin destino. (e) «El paper P tiene 3 revisores, cada uno con una nota»: nota → propiedad de la arista REVIEWED_BY si es del vínculo; nodo Reseña SOLO si la reseña gana relaciones o ciclo propios (se apela, se revisa) — la prueba decide, no la estética.

**2. Explica por qué el reto experto termina perdiendo: el índice manual del modelo naive.** Mecánica: un `HashMap` entidad→documentos baja P6 de 12 lecturas a 1. Y sigue perdiendo por tres razones estructurales: (a) el índice hay que CONSTRUIRLO y MANTENERLO a mano por cada tipo de pregunta —P3 inversa necesita un índice de citas, P5 otro de temas—, mientras el LPG cobra la MISMA moneda (expansión) para todas; (b) las consultas multi-salto (P9) exigen cruzar varios índices con joins a mano que el motor de travesías hace solo; (c) el coste no está en la implementación sino en el DISEÑO: cada pregunta nueva es un programa nuevo. El índice optimiza una consulta; el modelo habilita un espacio de consultas.

**3. Retrieval sin pistas: recita la escalera R1-R7 y la tabla de costes.** Cierra el libro. R1 identidad → R2 relaciones propias → R3 ciclo de vida propio (≥2 SÍ ⇒ nodo) → R4 cardinalidad → R5 vínculo sin vida fuera del par ⇒ arista (+props; nodo intermedio solo con ganancias propias) → R6 valor simple filtrable ⇒ propiedad → R7 árbitro: gana la consulta prevista. Costes: P6 12 vs 5; P8 16 AUTHORED contados en Rust; P9 3 temas con d1==d2. Si pusiste la cardinalidad antes que el ciclo de vida, o R7 antes que R6, relee §41.3: el orden ES el argumento (R7 solo desempata lo que R1-R6 no decidieron).

## Ejercicios propuestos

**Esencial (recordar + aplicar; 34+41).** Desarrolla el reto esencial del §41.10: añade «Gaby» y su paper al builder, PREDICE por escrito P1, P3 y el resultado del validador, y verifica contra los tests. Criterio: predicción escrita primero; si el validador rechaza tu paper, identifica el peldaño que ignoraste (¿olvidaste `anio`? ¿el `order` de la AUTHORED?).

**Intermedio (predecir; 7+41).** El caso «Reseña»: decide arista `REVIEWED_BY{nota}` vs nodo `Reseña` aplicando R1-R7 por escrito, con las consultas previstas como árbitro, y escribe el test que demuestra tu decisión. Criterio: la justificación debe citar qué pregunta compraría el nodo intermedio (¿existe «evolución de la nota» entre tus consultas previstas?) y qué salto extra pagas hoy por reificar.

**Experto (crear y medir; 20+41).** Índice manual del modelo naive: construye `HashMap` entidad→documentos, mide P6 (1 lectura), y responde por escrito: (a) ¿qué consulta del catálogo sigue exigiendo escaneo total? (b) ¿cuántos índices más necesitarías para cubrir P3 y P5? (c) ¿qué hace `in_edges` que un índice plano no puede hacer? Restricciones: std puro, suite ALL_GREEN con TU test dentro.

## Para profundizar

- **Robinson, Webber y Eifrem, *Graph Databases*, 2ª ed. (O'Reilly, junio 2015; 1ª ed. 2013)** — capítulo 4: guías de modelado orientadas a preguntas; el libro que popularizó el property graph (anécdota del §41.0).
- **Angles y Gutierrez, «Survey of graph database models», ACM Computing Surveys 40(1), 2008** — el censo de la PRIMERA ola de modelos de datos de grafos (GOOD, G-Log, HyperNode…) y su declive; la fuente de la historia pequeña.
- **Angles y Gutierrez, «An Introduction to Graph Data Management», Springer LNCS 10000, 2018 (arXiv:1801.00036)** — la formalización del LPG: nodos Y aristas con identidad propia, labels y propiedades; base de la decisión R5 (esquema abierto, sin tipos de extremos).
- **Francis et al., «Cypher: An Evolving Query Language for Property Graphs», SIGMOD 2018, pp. 1433-1445 (DOI 10.1145/3183713.3190657)** — las relaciones llevan propiedades; la historia de Cypher de vendor a multi-vendor.
- **La especificación openCypher** — el lenguaje abierto que desató Cypher del vendor (sin fecha citada: frontera declarada del contrato).
- **W3C, «RDF 1.1 Concepts and Abstract Syntax», Recommendation, 25-feb-2014** — el contraste: en RDF todo atributo es un triplete; el `order` de arista no existe sin reificación (adelanto del cap. 46).
- **ISO/IEC 39075:2024 (GQL)** — el estándar que cerró el viaje 2013-2024 del §41.0.
- **Jin, Feng, Chen, Liu y Salihoğlu, «KÙZU Graph Database Management System», CIDR 2023 (CC-BY 4.0)** — atribución según ADR-001 (Waterloo → CIDR 2023 → Apple oct-2025 → archivo → forks LadybugDB/bighorn); el contraste schema-optional vs esquema declarado.
- **Guías de modelado de Neo4j (docs oficiales)** — la versión industrial del query-first: decisiones de modelado guiadas por patrones de consulta.
- Dentro del libro: cap. 7 (LPG, labels múltiples nativas, `Value`), cap. 8 (`GraphStore`/`MemoryStore` y `in_edges`/`out_edges`), caps. 17-21 (LiraQL end-to-end y su frontera: sin agregación, lexer con límite UTF-8), cap. 20 (direcciones de expansión), cap. 32 (formato CSV estilo neo4j-admin), cap. 34 (dataset determinista: ids fijos, cero RNG, cero tiempos), Vol. I caps. 3-4 (la travesía mental BFS que P9 recorre).

## Mini-diálogo: en guardia nocturna

> — Son las tres de la madrugada. El equipo pide «los temas de Ana» y la query va lenta. El grafo está bien, dicen.
>
> — ¿Qué quiere decir «los temas de Ana»?
>
> — Pues… los temas de los papers que ha escrito. Mira, ya la tengo: `WHERE temas CONTAINS 'Ana'` — todo está en propiedades: `autores:"Ana;Beto"`, `temas:"grafos de conocimiento;memoria de agentes"`.
>
> — (pausa) ¿Cuántos documentos tienes?
>
> — Doce.
>
> — Perfecto. Ahora dime, sin escribir código nuevo: ¿qué documentos mencionan a Neurónica Y citan un paper de 2021? El split no es una consulta, es un testamento: cada pregunta nueva es un programa nuevo. ¿Y cuando sean doce MIL?
>
> — …
>
> — Mañana modelamos: las personas y los documentos son NODOS, «escribió» es una ARISTA, el orden de firma es propiedad de la arista, y las preguntas que ya conocemos se convierten en tests. El modelo se paga la primera noche que una pregunta cruza DOS campos. Y esta KB es la primera piedra de la memoria del agente — que a las tres de la mañana también querrá respuestas.
>
> — ¿Y las doce mil filas?
>
> — No existen. Existe un grafo con puertas. Buenas noches.

> *Siguiente parada, cap. 42 (antipatrones): el Tema «grafos de conocimiento» de tu flamante KB-Lira YA acumula 6 aristas ABOUT que nadie pidió — cuando un hub deja de ser inocente, empieza a cobrar. Preguntas que dejamos abiertas: ¿qué pasa cuando los HECHOS cambian y la afiliación de Ana ya no es la de 2023? (cap. 43, temporalidad). ¿Y quién garantiza que no existan dos documentos con el mismo título? (cap. 44, constraints e índices).*
