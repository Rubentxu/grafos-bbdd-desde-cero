---
title: "Grafos en la era de la IA: modelar, razonar y recuperar — Knowledge bases, GraphRAG y memoria de agentes"
subtitle: "Volumen III de Grafos en Computación: de Cero a Experto"
author: "Rubentxu"
date: "2026-08-14"
lang: es
volumen: III
obra: "Grafos en Computación: de Cero a Experto"
proyecto_integrador: LiraDB
edicion: "Edición unificada 2026 — primer borrador"
licencia: "CC BY-NC-SA 4.0"
---

# Grafos en la era de la IA

**Knowledge bases, GraphRAG y memoria de agentes**

*Volumen III de la obra "Grafos en Computación: de Cero a Experto" — con LiraDB como banco de pruebas.*

---

> «Los vectores recuerdan lo que se parece; el grafo recuerda por qué se relaciona.»
> — Manifiesto KB-Lira

---

**Edición**: Primer borrador, agosto de 2026
**Volumen**: III (de III)
**Idioma**: Español (con terminología técnica estándar en inglés)
**Stack**: Rust **2024** edition + LiraDB + crates seleccionadas (ver Apéndice 0 del Vol.II)
**Proyecto integrador**: **KB-Lira** — base de conocimiento sobre LiraDB, hilo conductor del volumen
**Licencia**: CC BY-NC-SA 4.0

---

# Prólogo — El grafo como capa de conocimiento

> *Borrador. Se completará cuando se hayan redactado los caps. 41-45.*

En 2024 los grafos dejaron de ser una curiosidad de nicho y se convirtieron en **infraestructura de la IA**. Los agentes necesitan memoria que no caduque; los sistemas RAG descubrieron que la similitud vectorial sola no responde preguntas multi-hop; y la extracción con LLM convirtió cualquier montón de documentos en un grafo — si sabes modelarlo.

Este volumen es la respuesta del proyecto a esa realidad. No es un libro de "prompt engineering" ni de machine learning: es un libro de **modelado y uso de grafos como base de conocimiento**, con LiraDB — el motor que construiste (o estudiaste) en el Volumen II — como banco de pruebas.

## Qué asume este volumen

Asumimos que conoces el **modelo Property Graph** (Vol.II cap. 7), sabes lo que es un `MATCH` (Vol.II caps. 17-18) y entiendes PageRank y comunidades a nivel conceptual (Vol.II caps. 24-25). El cap. 20 del Vol.I (GNN) ayuda pero no es obligatorio: aquí los embeddings se usan, se inspeccionan y se construyen a mano, no se entrenan redes profundas.

No asumimos conocimientos previos de:

- modelado de datos de grafos (antipatrones, temporalidad, esquemas);
- RDF, OWL o SHACL;
- sistemas de recuperación (RAG, ANN, índices vectoriales);
- pipelines de extracción con LLM.

Todo se construye desde cero, con la misma regla de la obra: **primero a mano, luego con herramienta, siempre entendiendo el trade-off**.

## Cómo leer este volumen

**Ruta lineal**: del cap. 41 al 53 en orden. Tiempo estimado: 60-80 horas. Primer borrador ~300 páginas.

**Ruta "arquitecto de datos"**: Partes I y II (caps. 41-48). Para quien modela knowledge bases y no va a tocar embeddings.

**Ruta "IA"**: si vienes del mundo RAG/LLM y ya sabes modelar, empieza en el cap. 49. Los caps. 41-45 se pueden consultar como referencia de modelado cuando el grafo te empiece a doler.

## El hilo conductor: KB-Lira

Cada capítulo añade una capa a **KB-Lira**, una base de conocimiento realista de un equipo de investigación: papers, notas, informes, personas, organizaciones, proyectos y temas, con relaciones de autoría, citación, mención y afiliación. Se modela en el cap. 41, se temportaliza en el 43, se valida en el 47, se vectoriza en el 49, se consulta con GraphRAG en el 51, se enriquece con un LLM en el 52 y acaba siendo la memoria de un agente en el 53. El generador determinista del dataset vive en el workspace.

## ¿Qué te llevarás?

Después de leer este libro:

- Diseñarás knowledge bases que **no se pudren** al crecer (antipatrones, temporalidad, esquema).
- Sabrás traducir entre Property Graph y RDF sin perder el alma de ninguno.
- Implementarás un índice **HNSW** desde cero — y entenderás que es, literalmente, un grafo.
- Construirás un pipeline **GraphRAG** híbrido y sabrás evaluarlo.
- Extraerás tripletas con un LLM sin envenenar tu grafo (grounding, dedup, human-in-the-loop).
- Montarás la **memoria de largo plazo** de un agente sobre un KG temporal.

Empezamos. Bienvenido a la capa de conocimiento.

---

*(El Prólogo se completará cuando se hayan redactado los caps. 41-45. Mientras tanto, este párrafo actúa de placeholder.)*

---

# Tabla de contenidos

> *Borrador — outline detallado en `book-context/OUTLINE-VOL3.yml` (secciones, conceptos y dependencias por capítulo).*

**Prólogo — El grafo como capa de conocimiento**

**Parte I — Modelar datos de grafos**
41. Modelar entidades, propiedades y relaciones
42. Antipatrones: supernodos, reificación y otras trampas
43. El tiempo en el grafo: versionado y bitemporalidad
44. Esquema, constraints e índices en property graphs
45. Workflows de ingesta: de datos crudos al grafo (CSV, JSONL, entity resolution)

**Parte II — Knowledge bases semánticas**
46. Property Graph vs RDF: dos filosofías, un mismo objetivo
47. Ontologías y validación: OWL ligero y SHACL
48. El paisaje de lenguajes: Cypher, GQL (ISO 39075), SPARQL y Gremlin

**Parte III — Grafos × IA**
49. Embeddings de grafos: de estructura a vectores
50. Índices vectoriales: HNSW, o cuando el índice también es un grafo
51. GraphRAG: recuperación híbrida vector + grafo
52. Extracción con LLM: de texto suelto a grafo fiable
53. Grafos como memoria de agentes

**Epílogo — El grafo como capa de conocimiento**

**Apéndice A — El dataset KB-Lira: guía completa y generador**
**Apéndice B — Glosario IA + grafos (bilingüe ES/EN)**
**Apéndice C — Bibliografía y papers (GraphRAG, HNSW, RDF/OWL/SHACL, GQL)**
**Apéndice D — Herramientas del ecosistema 2026 (paisaje post-Kùzu)**
**Apéndice E — ADRs del Vol.III (modelo KB-Lira, extracción, evaluación)**

---

*(El cuerpo de los 13 capítulos se redactará cuando el outline sea aprobado y pase por chapter-planner. Este archivo es un esqueleto navegable.)*

---

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
# Capítulo 43 — El tiempo en el grafo: versionado y bitemporalidad

> *«Tercer capítulo del Volumen III. Si vienes del cap. 42 —o del perfil datos/IA que lo haya leído en diagonal— la escalera R1-R7, las 10 preguntas, los validadores por composición, la migración y la regresión son tus HERRAMIENTAS, no tu contenido: aquí el modelo ya está refactorizado y ahora se le añade el TIEMPO. El cierre del cap-42 te dejó una pregunta clavada: la ronda 2 contrarresta a la ronda 1 — pero ¿QUÉ valía el 3 de marzo? Este capítulo la responde CON DATOS: el 3 de marzo de 2025 valía la **nota 7** (la ronda 1): la ronda 2 (nota 8) llegó después y la contrarrestó; el grafo del cap-42 decía QUÉ contrarresta a qué, y no podía decirte CUÁNDO porque no guardaba el tiempo. Aquí el tiempo entra al grafo — con la frontera del grano (granularity) declarada: el dataset habla en AÑOS (como `anio` del cap-41), y con grano anual 2025 responde 8, porque la frontera de caducidad es el año mismo; distinguir el 3 del 10 de marzo exigiría grano fino, y el grano lo pone el dato, no el modelo (Snodgrass 1999: intervalos continuos).»*

## 43.0 La anécdota de la esquina

Un hospital. Los pacientes comparten sala durante **periodos** (periods of time): la sala 3 acoge a A y B en enero, a B y C en febrero, y en marzo solo a C. Si dibujas el grafo estático de «quién compartió sala con quién», aparece una arista A-B, otra B-C y otra A-C — pero A y C **nunca coincidieron**: la arista A-C es un fantasma del estático, un contagio que nunca pudo ocurrir. Holme y Saramäki (Physics Reports, 2012) revisaron esta familia de redes temporales (temporal networks) —pacientes, correo instantáneo, llamadas, contactos— y destilaron la frase que ordena este capítulo: se trata de **«mover la información de cuándo pasa algo, del sistema dinámico al propio grafo»** (moving the information of *when* things happen, from the dynamical system on the network, to the network itself). Cuando el cuándo vive fuera, la arista A-C existe y el modelo miente: los caminos temporales no son transitivos — un camino que respeta el tiempo no puede saltar de enero a marzo—, y Kostakos (Physica A, 2009) ya lo contaba para sus temporal graphs: el grafo estático **sobreproyecta** (over-projects) lo que nunca fue simultáneo.

Tu KB-Lira está enferma de lo mismo. El grafo del cap-42 dice que **Beto pertenece a Instituto Neurónica** — pero se fue en 2024 y ya no está. ¿Tu grafo te está mintiendo por omisión? No te ha mentido: **no sabía que el tiempo pasaba**.

## 43.1 Objetivo

Objetivo medible del outline: **representar la historia y la validez temporal de los hechos del grafo sin destruir el rendimiento de las consultas del presente**. Al terminar tendrás:

1. **El builder temporal** `kb_lira_paso3()`: el paso-2 refactorizado (67 nodos) + `aplicar_valid_time` → **68 nodos, 158 aristas, 10 `MEMBER_OF`** con valid-time (valid time) en props de arista.
2. **La validez como función** `arista_vigente_en` (intervalo medio abierto `[desde, hasta)`, ausencia = abierto).
3. **Consultas AS OF** `afiliaciones_vigentes_en` / `afiliaciones_actuales` con `CosteLecturas`.
4. **La tesis del coste, medida**: presente y AS OF pagan el MISMO barrido; cada arista vencida (expired edge) añade 1 `get_edge`.
5. **El contraste borrar-vs-caducar**, con números.
6. **Bitemporalidad (bitemporal) mínima**: `HistoricoAfiliaciones` + el caso de Dani (dos respuestas legítimas).
7. **La conexión con el WAL real del cap-28**, demostrada en un test.
8. **El validador paso-3** por composición con el paso-2.
9. **La regresión triple** (red de seguridad de los caps. 41-42).
10. **CSV determinista paso-3** + artefacto commiteado. **971 tests ALL_GREEN** (948 + 23), sin bench.

## 43.2 Problema

Mira el resultado del cap-42: 948 tests verdes, el modelo refactorizado, las 10 preguntas respondiendo con sus costes. Y sin embargo la P4 —la travesía de 2 saltos `(:Proyecto)<-[:WORKED_ON]-(:Persona)-[:MEMBER_OF]->(:Organizacion)` que diseñaste en el cap-41— sigue respondiendo **Beto → Instituto Neurónica**, y Beto se fue en 2024. La suite sigue verde y la respuesta es falsa. Ese es el problema: **el grafo atemporal miente por omisión**: la P4 responde afiliaciones que ya no existen, y ningún test lo caza porque nadie le ha pedido el cuándo. Verde no es verdad — la frase del cap-42 («verde no es sano») se repite en el eje temporal: el modelo estaba sano y ahora está DESACTUALIZADO, sin que el validador lo note.

Antes de dibujar nada, desactivemos las **seis** ideas equivocadas que suelen venir con el tema:

1. **«La temporalidad es para las fechas de nacimiento o publicación.»** No: la temporalidad vive en el VÍNCULO — «cuándo fue cierto que X pertenecía a Y» es un atributo de la arista, no de los nodos (Snodgrass 1999: el valid time es propiedad de los hechos modelados).
2. **«Guardar la historia duplica los datos y degrada las consultas del presente.»** No: caducar sin borrar NO degrada el presente — el barrido de adyacencia es el MISMO; lo que se paga es cada arista vencida que sigue en la lista (medible: 1 `get_edge` por arista histórica; el cap. 44 cambiará ese precio).
3. **«Bitemporalidad = dos fechas en la misma fila.»** No: son DOS EJES ortogonales — el valid-time (cuándo fue cierto en el mundo) y el transaction-time (cuándo lo supo el sistema). El caso de Dani lo demuestra: dos respuestas legítimas para la misma pregunta según el eje consultado.
4. **«El 3 de marzo (la nota de la ronda) es una fecha de evento.»** No: la pregunta del cap-42 era por la VALIDEZ (cuándo fue cierto), que no es ni cuándo ocurrió la reseña (evento) ni cuándo se registró (transacción). Los tres tiempos se confunden todo el tiempo (Jensen & Snodgrass, 1999).
5. **«Una fecha en un STRING vale lo mismo que una fecha tipada.»** No: un string no se compara por rango — la lección P6 del cap-41 (un string no se expande) repetida en el eje temporal.
6. **«El WAL del cap-28 ya guarda la historia.»** A medias: el WAL guarda las ESCRITURAS (el transaction-time del ESTADO), pero no el histórico de valores corregidos; por eso el `HistoricoAfiliaciones` existe — este capítulo demuestra lo que el WAL puede y lo que no puede responder.

Y el compromiso de honestidad que rige TODO el capítulo: los números salen de `cargo test`, nunca de la pizarra; si la tesis del contrato no cuadra con el ledger real, se pine el delta real y se explica POR QUÉ, prohibido maquillar contadores.

## 43.3 Modelo mental: la foto, la película y los dos relojes

El grafo del cap-42 era una **FOTO**: decía lo que sabemos hoy. Este capítulo lo convierte en **PELÍCULA**: cada arista nace, vive y caduca — y la pregunta «¿qué valía entonces?» tiene dos relojes: el del mundo y el del conocimiento. El panel que ordena todo:

```text
LOS TRES TIEMPOS (con la reseña de Fabio, el gancho del cap-42):
  EVENTO    cuándo ocurrió el hecho (event time)   la ronda 2 se escribió en 2025
  VALIDEZ   cuándo fue cierto en el mundo          la nota 8 es vigente DESDE 2025
            (lo que las aristas guardan)           (la nota 7 caducó: CONTRARRESTA 2025)
  REGISTRO  cuándo lo supo el sistema              el WAL del cap-28 (transaction-time)
            (lo que el log guarda)
```

La validez viaja en la arista como intervalo **medio abierto `[desde, hasta)`** (convención de Snodgrass 1999): el `hasta` se excluye — en 2024 Neurónica ya no vale. La línea de Beto:

```text
LA LÍNEA DE BETO (valid-time en la arista, intervalo [desde, hasta)):
  2018─────────────────────2024──────────────2026
  [─ Neurónica (53) ─────) [─ GrafoLuna (185) ─)
        hasta_anio=2024         desde_anio=2024, abierta
  «ahora» (2026): Beto→GrafoLuna · AS OF 2023: Beto→Neurónica · AS OF 2024: GrafoLuna
```

Y el doble reloj del caso que separa los ejes para siempre:

```text
EL CASO DE DANI (bitemporalidad: dos relojes):
  eje VALIDEZ (el mundo):     arista 55: desde_anio=2021 (corregido)
  eje REGISTRO (el saber):    Historico: ts=2023 «desde 2019» ──corrección──► ts=2025 «desde 2021»
  «¿qué creíamos en 2024?» → desde 2019      «¿qué sabemos hoy (2026)?» → desde 2021
  (la MISMA pregunta, DOS respuestas legítimas según el reloj consultado)
```

Debajo, la REGLA DE ORO heredada del cap. 34 (determinismo total): el «ahora» es una CONSTANTE (`ANIO_ACTUAL = 2026`), cada consulta devuelve su `CosteLecturas` pineado, y las 10 preguntas del cap-41 + el validador paso-2 del cap-42 son la red de seguridad: si añadir validez cambia una respuesta vieja sobre los subgrafos 1-2, el cambio está MAL. El momento ¡ajá! perseguido: **«el cap-42 me dijo QUÉ contrarresta a qué; este capítulo me dice CUÁNDO — y "cuándo" tiene dos respuestas: cuándo fue cierto y cuándo lo supimos. El grafo del cap-42 no mentía: simplemente no sabía que había pasado»** (Holme & Saramäki como epígrafe de la sección).

## 43.4 Primera solución

La primera solución — y la que todo el mundo aplica — es **la NO-solución doble**:

**(a) BORRAR la arista vencida.** Beto se fue de Neurónica: `delete_edge(53)` y listo. El presente queda limpio — `afiliaciones_actuales` responde Beto → GrafoLuna, idéntico al paso-3 normal, y el test `borrar_en_vez_de_caducar_destruye_el_as_of` lo confirma: **el presente es idéntico**. Borrar parece gratis. Y el AS OF 2023, sin la 53, responde `(Ana, UniLira); (Dani, Neurónica)` — **sin Beto**: su única afiliación vigente entonces era la que acabas de borrar. «Borrar es gratis… hasta que alguien pregunta por el pasado.»

**(b) Apuntar la fecha en un STRING.** La arista 53 se queda con `vigencia: "2018-2024"`. Legible, humana, exacta… e inservible: **un string no se filtra por rango** — «¿quién pertenecía a Neurónica en 2023?» exige parsear `"2018-2024"`, partir por el guion y comparar enteros. Es la lección P6 del cap-41 repetida: una string nunca se expande, y ahora ni se compara.

El capítulo te muestra ambas con sus modos de fallo ANTES de la solución: una destruye la historia, la otra la pone en un formato que nadie puede consultar.

## 43.5 Sus límites

La no-solución doble tiene tres límites que la delatan:

1. **Borrar destruye la historia.** «¿A quién pertenecía el proyecto en 2023?» no tiene respuesta: la arista ya no existe. El presente sigue perfecto — el límite es que el pasado deja de ser preguntable, y el pasado es exactamente lo que este capítulo quiere preguntar.
2. **El string no se compara.** `"2018-2024"` no participa en ningún rango; cada consulta temporal tendría que reimplementar un parser de fechas. El tipado no es estética: es consultabilidad (la lección P6, otra vez).
3. **El «ahora» de verdad cambia cada día.** Si la consulta usara `SystemTime::now()`, los tests del capítulo cambiarían cada día y el informe pineado moriría. El «ahora» del dataset es una constante (`ANIO_ACTUAL = 2026`) — disciplina del cap. 34: determinismo total o nada.

Y el límite que cierra la lista: **la pregunta del cap-42 sigue sin responderse**. Ninguna de las dos no-soluciones sabe decir qué valía la nota el 3 de marzo.

## 43.6 Solución evolucionada

Ocho piezas, cada una un TRADE-OFF con precio en lecturas:

**1. Valid-time en las aristas: `desde_anio` / `hasta_anio`.** `aplicar_valid_time` añade validez a las 10 `MEMBER_OF` — las 6 del paso-1 y 4 nuevas (ids 182-185) + el nodo 69 `:Organizacion` «Instituto GrafoLuna»:

```text
52 Ana→UniLira desde 2018 · 53 Beto→Neurónica 2018-2024 (VENCIDA) · 54 Carla→UniLira desde 2020
55 Dani→Neurónica desde 2021 (la CORREGIDA) · 56 Elena→GrafosYa desde 2019 · 57 Fabio→GrafosYa desde 2019
182 Hugo→UniLira 2019 · 183 Iris→GrafosYa 2022 · 184 Gaby→Neurónica 2023 · 185 Beto→GrafoLuna 2024
```

`InformeValidTime`: **8 aristas modificadas (6 + la REALIZA 149 y la CONTRARRESTA 157), 4 creadas, 1 nodo creado, 8 lecturas** (una `get_edge` por arista modificada). El esqueleto de la operación, tal cual vive en el módulo:

```rust
// cap43_temporalidad.rs — esqueleto de aplicar_valid_time()
for (id, desde, hasta) in [
    (52usize, 2018i64, None),     // Ana → UniLira
    (53, 2018, Some(2024)),       // Beto → Neurónica (VENCIDA: Beto se muda)
    (55, 2021, None),             // Dani → Neurónica (el valor CORREGIDO)
    // … 54, 56, 57
] { poner_validez(store, id, desde, hasta, &mut lecturas); }
// nodo 69 :Organizacion «Instituto GrafoLuna» + 4 MEMBER_OF (182-185)
// gancho reseña: REALIZA 149 → hasta 2025 · CONTRARRESTA 157 → desde 2025
```

La validez es atributo DEL VÍNCULO: la escalera R1-R7 del cap-41 decide que la afiliación no tiene identidad propia (sin relaciones salientes ni ciclo de vida más allá de la arista — R2/R3 no suben), y la arista ya se lee completa en el barrido: la validez viaja GRATIS en la lectura que ya se hace. El díptico con el cap-42 enseña la MISMA regla con dos veredictos: `Resena` se reificó porque tiene `CONTRARRESTA`; la afiliación no se reifica porque no tiene nada que perder.

**2. La validez como función.** `arista_vigente_en(arista, anio)` implementa `[desde, hasta)` con ausencia = abierto; una arista SIN props de validez es vigente SIEMPRE (retrocompatibilidad caps. 41-42):

```rust
// cap43_temporalidad.rs — el corazón de arista_vigente_en (4 casos)
match (desde, hasta) {
    (None, None) => true,
    (Some(d), None) => d <= anio,
    (None, Some(h)) => anio < h,
    (Some(d), Some(h)) => d <= anio && anio < h,
}
```

**3. Consultas AS OF con coste.** `afiliaciones_vigentes_en(store, proyecto, anio)` es la P4 del cap-41 CON tiempo — la MISMA travesía de 2 saltos (`WORKED_ON` entrante + `MEMBER_OF` saliente) con un filtro de validez; `afiliaciones_actuales` la envuelve contra `ANIO_ACTUAL`. Moneda: `CosteLecturas {in_edges, get_edge, get_node}` — la misma disciplina de contadores de `ContandoStore` (cap-26) y el ledger del cap-41; localizar el proyecto por nombre queda FUERA del ledger (saber QUÉ preguntamos es previo a la consulta, igual que en el cap-41).

**4. La tesis del coste — y su adaptación honesta.** El filtro de validez se aplica sobre datos YA leídos: la adyacencia no distingue vigentes de vencidas SIN índice, así que el presente y el AS OF barren EXACTAMENTE lo mismo: `el_presente_y_el_as_of_cuestan_el_mismo_barrido` pine **28 = 28 lecturas** para 2026 y 2023 (1 `in_edges` + 21 `get_edge` + 6 `get_node`). Lo que paga la historia es cada arista vencida que conservamos: `cada_arista_vencida_anade_una_lectura_al_barrido` mide la variante sin la 53 (`delete_edge`) → **27 totales (get_edge 21→20) y candidatas `MEMBER_OF` de Ana/Beto/Dani 4→3**. El contrato del capítulo predecía la tesis como «13→14» contando solo las candidatas del paso-1 vs paso-3; el ledger real mide el barrido completo (21 `get_edge`) y se pine el delta real — 21→20 y 4→3 — con su comentario: la tesis (1 lectura por arista vencida en CADA barrido que toca a su persona) sobrevive en ambas monedas; con 10M de vencidas, 10M de lecturas — la factura que el cap. 44 cobrará con el índice.

**5. Bitemporalidad mínima: `HistoricoAfiliaciones`.** Un Vec append-only de `EntradaHistoria {ts_registro, persona, organizacion, desde_anio}` donde el `ts_registro` lo asigna el propio histórico (monótono: 1, 2, …) — **el «WAL del modelo»**. Caso Dani: ts 1 (2023) «Dani→Neurónica desde 2019» — lo que se creía; ts 2 (2025) «desde 2021» — la corrección, la que coincide con la arista 55. `afiliacion_segun_registro(historico, Dani, 2024, 1)` → `(Neurónica, 2019)`; con `ts = 2` → `(Neurónica, 2021)`. Dos respuestas legítimas para la misma pregunta según el reloj que consultes — bitemporal NO es dos fechas: son dos ejes (Jensen & Snodgrass 1999: la distinción es el fundamento; TSQL2 1995: los dos ejes como tipos del estándar).

**6. El gancho cobrado.** La `REALIZA` 149 (ronda 1 de Fabio, nota 7) gana `hasta_anio:2025` y la `CONTRARRESTA` 157 (ronda 2 → ronda 1) gana `desde_anio:2025`. La regla de resolución, documentada en el módulo: la vigencia de una reseña NO vive en el nodo `:Resena` ni en su `SOBRE` — vive en su `REALIZA` (Persona→Resena) y, si es la sucesora de otra, en el `desde_anio` de su `CONTRARRESTA`. Candidata vigente = REALIZA que cumple `arista_vigente_en` Y (si emite CONTRARRESTA) todas sus CONTRARRESTA vigentes. Resultado real de `nota_de_resena_vigente_en(store, "Informe de revisión por pares 2025", anio)`: 2024 → **Some(7)** (la ronda 1 aún reina); 2025 → **Some(8)** (la ronda 2 la contrarrestó); 2026 → Some(8). La frontera del grano, declarada: el contrato pregunta «el 3 de marzo»; con grano ANUAL, 2025 responde 8 porque la frontera de caducidad es el año mismo — distinguir el 3 del 10 exige grano fino (Snodgrass: intervalos continuos; aquí el grano lo pone el dato, no el modelo).

**7. El validador paso-3 por composición.** `validar_modelo_kb_lira_paso3` REUTILIZA `validar_modelo_kb_lira_paso2` (cap-42, sin tocarlo) y añade reglas nuevas SOLO para `MEMBER_OF`: `desde_anio:Int` requerido; `hasta_anio ≥ desde_anio`; `desde_anio ≤ ANIO_ACTUAL`. El detalle de composición, con honestidad: el paso-2 ya FILTRA sus tipos (los 6 que gobierna), así que las reglas nuevas se propagan SIN filtro — el subgrafo refactorizado sigue cumpliendo su contrato, y las violaciones del paso-2 pasarían tal cual, sin taparse. Fixture corrupto a mano → **3 violaciones con ids [52, 54, 56]** (52 sin desde; 54 intervalo invertido, hasta 2019 < desde 2020; 56 validez futura, desde 2030). Estas reglas SIEMBRAN los constraints del cap. 44: lo que hoy es convención ejecutable, allí será garantía del motor.

**8. La REGLA DE ORO:** las respuestas viejas no cambian. Regresión triple: las 10 preguntas del cap-41 sobre el subgrafo paso-1 IDÉNTICAS (el único desajuste real es P4: la `MEMBER_OF` 185 añade Beto→GrafoLuna, que ES la lección — se filtra en el subgrafo con el helper `solo_afiliaciones_paso1`, org id < 30); las respuestas pineadas del paso-2 sobre el paso-3 entero (P1 Ana=4, P3 idéntico, P5 jerárquica 24, P6 Neurónica=5, P8=40 con 9 personas); `validador_paso2_acepta_el_modelo_paso3`. La P4 atemporal sigue diciendo Beto→Neurónica (su contrato no cambia) mientras la P4 CON tiempo dice Beto→GrafoLuna — **la diferencia ES el capítulo, no una regresión** (documentado en el test para que nadie lo «corrija»).

## 43.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap43_temporalidad.rs` (**1.853 líneas**, std puro, **23 tests**), cableada con dos líneas aditivas en `lib.rs`; el artefacto regenerable `datasets/kb-lira/paso-3/{nodes.csv (69 líneas), edges.csv (159 líneas), historico.csv (4 líneas)}` es la salida del builder — el dataset es lo que «importó el equipo», la temporalidad es código que se re-ejecuta. CERO dependencias nuevas, CERO cambios en caps. 7-42, goldens intactos. Y **NO hay `[[bench]]`**: decisión #11 del contrato en una línea — la moneda son lecturas y conjuntos exactos, y cronometrar no sostiene ninguna tesis de este capítulo (espejo de la decisión #12 del cap-41).

Las piezas que sostienen el edificio (nombres exactos; el código completo vive en el módulo):

```rust
pub const ANIO_ACTUAL: i64 = 2026;                                  // el «ahora» FIJO del dataset
pub fn kb_lira_paso3() -> MemoryStore;                              // 68 nodos, 158 aristas, 10 MEMBER_OF
pub fn aplicar_valid_time(store: &mut MemoryStore) -> InformeValidTime; // 8 modificadas, 4 creadas, 1 nodo, 8 lecturas
pub fn arista_vigente_en(arista: &Edge, anio: i64) -> bool;         // [desde, hasta) con ausencia = abierto
pub struct CosteLecturas { pub in_edges: usize, pub get_edge: usize, pub get_node: usize }
pub fn afiliaciones_vigentes_en(store: &dyn GraphStore, proyecto: &str, anio: i64)
    -> (Vec<(String, String)>, CosteLecturas);
pub fn afiliaciones_actuales(store: &dyn GraphStore, proyecto: &str)
    -> (Vec<(String, String)>, CosteLecturas);
pub fn nota_de_resena_vigente_en(store: &dyn GraphStore, titulo: &str, anio: i64) -> Option<i64>;
pub struct EntradaHistoria { pub ts_registro: u64, pub persona: usize,
    pub organizacion: usize, pub desde_anio: i64 }
pub struct HistoricoAfiliaciones;                                    // append-only, ts monótono (el WAL del modelo)
pub fn validar_modelo_kb_lira_paso3(store: &dyn GraphStore) -> Result<(), Vec<Violacion>>;
pub fn informe_temporal_reproducible(store: &dyn GraphStore) -> String; // la tabla del §43.8
```

## 43.8 Prueba de fuego

Primero el bucle rápido — salida REAL de `cargo test`, sin tiempos:

```text
$ cargo test -p vol2-liradb --lib cap43

running 23 tests
test cap43_temporalidad::tests_temporalidad::estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas ... ok
test cap43_temporalidad::tests_temporalidad::las_member_of_llevan_validez_desde_y_hasta_en_anios ... ok
test cap43_temporalidad::tests_arista_vigente::arista_vigente_en_cubre_abierta_vencida_y_futura ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_actuales_de_kira_responden_beto_en_grafosluna ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_as_of_2023_responden_beto_en_neuronica ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_as_of_2019_no_incluyen_a_dani ... ok
test cap43_temporalidad::tests_coste_temporalidad::el_presente_y_el_as_of_cuestan_el_mismo_barrido ... ok
test cap43_temporalidad::tests_coste_temporalidad::cada_arista_vencida_anade_una_lectura_al_barrido ... ok
test cap43_temporalidad::tests_coste_temporalidad::borrar_en_vez_de_caducar_destruye_el_as_of ... ok
test cap43_temporalidad::tests_resena_vigente::la_ronda_1_de_fabio_caduco_cuando_la_ronda_2_la_contrarresto ... ok
test cap43_temporalidad::tests_historico_afiliaciones::historico_afiliaciones_registra_el_caso_de_dani ... ok
test cap43_temporalidad::tests_historico_afiliaciones::afiliacion_segun_registro_distingue_lo_creido_de_lo_cierto ... ok
test cap43_temporalidad::tests_wal_transaction_time::el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time ... ok
test cap43_temporalidad::tests_validador_paso3::validador_paso3_acepta_el_modelo_temporal ... ok
test cap43_temporalidad::tests_validador_paso3::validador_paso3_rechaza_fixture_sin_validez ... ok
test cap43_temporalidad::tests_regresion_temporalidad::las_10_preguntas_del_paso1_no_cambian_tras_anadir_valid_time ... ok
test cap43_temporalidad::tests_regresion_temporalidad::las_respuestas_del_paso2_no_cambian_tras_anadir_valid_time ... ok
test cap43_temporalidad::tests_regresion_temporalidad::validador_paso2_acepta_el_modelo_paso3 ... ok
test cap43_temporalidad::tests_csv_paso3::csv_roundtrip_paso3_import_export_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_historico_roundtrip_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_paso3_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_paso1_y_paso2_intactos_tras_paso3 ... ok
test cap43_temporalidad::tests_informe_temporal::informe_temporal_reproducible_sobre_kb_lira ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 948 filtered out
```

Veintitrés verdes; workspace entero en **971 ALL_GREEN** (948 + 23) con goldens intactos. El gancho del cap-42, respondido por el test que lleva su nombre — la ronda 1 reina hasta 2025 y la ronda 2 la contrarresta desde entonces:

```text
$ cargo test -p vol2-liradb --lib cap43 la_ronda_1_de_fabio_caduco_cuando_la_ronda_2_la_contrarresto

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2024) = Some(7)  ← la ronda 1 aún reina
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2025) = Some(8)  ← la ronda 2 la contrarrestó
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2026) = Some(8)
```

Ahora el informe del capítulo — salida REAL de `informe_temporal_reproducible`, pineada byte a byte:

```text
Barrido temporal de afiliaciones de «Proyecto Kira» (KB-Lira paso-3)
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
2026 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas
2024 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas
2023 | Ana → Universidad de Lira; Beto → Instituto Neurónica; Dani → Instituto Neurónica | 28 lecturas
2020 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas
2019 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
Caso Dani (bitemporal): lo que se creía frente a lo que se sabe
  ts 1 · «¿qué creíamos en 2024?»   → Instituto Neurónica, desde 2019 (lo que se creía)
  ts 2 · «¿qué sabemos hoy?»        → Instituto Neurónica, desde 2021 (lo que se sabe)
```

| Año | Afiliaciones de «Proyecto Kira» (AS OF) | Lecturas |
|---|---|---|
| 2026 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 |
| 2024 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 |
| 2023 | Ana → Universidad de Lira; Beto → Instituto Neurónica; Dani → Instituto Neurónica | 28 |
| 2020 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 |
| 2019 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 |

Cuatro lecturas obligatorias. **Primera: el medio abierto gobierna.** AS OF 2024 responde **GrafoLuna** — Neurónica caducó en 2024 (`[2018, 2024)`: el 2024 queda FUERA); el intervalo no es «hasta e incluyendo». **Segunda: la tesis del barrido, con su honestidad.** 2026 y 2023 pagan las MISMAS 28 lecturas (filtro sobre datos ya leídos, sin índice no hay atajo); 2020 y 2019 bajan a 27 porque Dani aún no se afilia (desde 2021) y su organización ni se lee — 1 `get_node` menos. **Tercera: el caso Dani.** La MISMA pregunta («¿cuándo empezó Dani en Neurónica?») con DOS respuestas legítimas: la del registro en 2024 (desde 2019 — lo que se creía) y la de hoy (desde 2021 — lo cierto). **Cuarta: el WAL, demostrado.** `el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time` usa la API REAL del cap-28: `WalTransaccion::begin` + `put_node`/`put_edge` + `commit` (Begin + 3 ops + Commit = 5 registros), corte de luz con `Wal::as_bytes`/`reconstruir` y `replay_wal` → el store renacido reconstruye la arista 55 con `desde_anio=2019` (1 transacción confirmada, 3 operaciones reaplicadas) mientras el Historico con la corrección responde 2021. **La frontera, declarada: el WAL sabe lo que se escribió, no lo que se corrigió.** Equivalencia: LSN ≡ ts_registro (ambos los asigna el log, el primer LSN es 1 y el primer ts es 1), Commit ≡ entrada, replay ≡ re-lectura en orden.

Y el CSV cierra el círculo: `csv_roundtrip_paso3_import_export_byte_a_byte`, `csv_historico_roundtrip_byte_a_byte`, `csv_paso3_coincide_con_dataset_commiteado_byte_a_byte` y `csv_paso1_y_paso2_intactos_tras_paso3`: los ficheros de los pasos 1-2 ni se tocan. La validez, visible en el propio dataset — el artefacto `datasets/kb-lira/paso-3/` commiteado:

```text
$ head -3 datasets/kb-lira/paso-3/edges.csv
id:ID, de:START_ID, a:END_ID, tipo:TYPE, desde_anio:INT, hasta_anio:INT, order:INT
52,0,6,MEMBER_OF,2018,,
53,1,7,MEMBER_OF,2018,2024,
185,1,69,MEMBER_OF,2024,,

$ cat datasets/kb-lira/paso-3/historico.csv
ts_registro,persona,organizacion,desde_anio
1,3,7,2019
2,3,7,2021
3,0,6,2018
```

La arista 53 con `2018,2024` (la vencida), la 185 con `2024,` (la abierta) y el histórico con las dos entradas de Dani (persona 3 → organización 7): el caso bitemporal completo, en cuatro líneas.

## 43.9 Qué hemos sacrificado

1. **Sin índices sobre `desde_anio` ni constraints UNIQUE temporales.** El validador paso-3 SIEMBRA las reglas que allí serán constraints e índices; aquí son convención ejecutable, y el AS OF SIN índice —28 = 28 lecturas— ES la lección (cap. 44).
2. **Sin ingesta con transaction-time automático.** El histórico se construyó a mano en el builder; el pipeline que anota el cuándo al importar es el cap. 45.
3. **Sin grano sub-anual.** La reseña del 3 de marzo se responde con grano anual y la frontera queda declarada con Snodgrass: el grano lo pone el dato, no el modelo.
4. **Sin durabilidad del Historico.** Vive en RAM; su WAL de verdad (bytes + CRC, durabilidad) es el del cap-28, y la frontera queda marcada (caps. 37/45).
5. **Sin RDF ni quads.** El tiempo en tripletas (`<s, p, o, t>`) se despliega en el cap. 46.
6. **La bitemporalidad es MÍNIMA.** El grafo guarda el valid-time actual (lo que el motor sabe HOY); el Historico guarda el registro. Versionar el grafo por transaction-time exigiría un MVCC de modelo — el del cap-30 versiona por CONCURRENCIA, no por historia, y esa distinción es el reto intermedio.

## 43.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j** introdujo en la **3.4 (2018)** los tipos temporales NATIVOS —DATE, LOCAL/ZONED TIME, LOCAL/ZONED DATETIME, DURATION—, indexables con range lookups (la documentación de APOC lo confirma: «Neo4j 3.4 introduced temporal data types»), pero SIN bitemporalidad de consulta: el patrón industrial es el MISMO tuyo, props de validez en las aristas. **GQL (ISO/IEC 39075:2024, abril 2024)** estandariza los tipos de datos temporales (date/datetime/duration) y funciones temporales en su Parte 1; las consultas bitemporales AS OF NO están en esa primera parte — frontera declarada, sin afirmar más de lo verificado. La tradición SQL temporal viene de lejos: **TSQL2** (Kluwer, 1995) sentó los dos ejes como tipos del estándar, y **SQL:2011** lleva décadas incorporando periods y temporal tables — sin detalle firme aquí: la referencia canónica de la casa es **Snodgrass** (*Developing Time-Oriented Database Applications in SQL*, Morgan Kaufmann, julio 1999) y **Jensen & Snodgrass** («Temporal Data Management», IEEE TKDE 11(1):36-44, enero/febrero 1999, DOI 10.1109/69.755613). Y el lado de grafos temporales: **Holme & Saramäki** (Physics Reports 519(3):97-125, octubre 2012) y **Kostakos** (Physica A 388(6):1007-1023, 2009) — la anécdota del §43.0 con su teoría detrás.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (34+41+43): PREDICE por escrito, ANTES de correr nada, qué responderá `afiliaciones_vigentes_en(store, "Proyecto Kira", 2020)` y qué responderá para 2024 — personas, organizaciones y las LECTURAS de cada una (27 o 28, y por qué). Luego ejecuta `informe_temporal_reproducible` y verifica tu predicción contra la salida real. Pista de predicción: en 2020 Dani aún no existe (desde 2021) y en 2024 Beto ya está en GrafoLuna (medio abierto: Neurónica caducó).
- *Intermedio* (30+43, 41+43): compara el versionado del cap-30 —`VersionNode/VersionEdge{ts_begin, ts_end}`— con el valid-time: el MVCC versiona por CONCURRENCIA (snapshots para lectores concurrentes, gc), el valid-time por HISTORIA (cuándo fue cierto); escribe qué es lo mismo (intervalos de tiempo por versión) y qué es distinto (quién pregunta y quién limpia). Y aplica el patrón a `WORKED_ON`: ¿el Proyecto Brújula «nació» en 2022? ¿qué `desde_anio` le pondrías y por qué a las aristas de Elena?
- *Experto* (28+43): usa `WalTransaccion` REAL del cap-28 para registrar la corrección de Dani (la entrada que el builder puso a mano) y demuestra en un test qué puede y qué NO puede responder el WAL sobre «¿qué creíamos en 2024?» — la frontera que motiva el Historico.

## 43.11 Lo que te llevas

- **El grafo atemporal miente por omisión.** No dice mentiras: no sabe que el tiempo pasó. La P4 respondía Beto→Neurónica cuando Beto se fue en 2024.
- **La temporalidad vive en el VÍNCULO.** `desde_anio`/`hasta_anio` en la arista (intervalo medio abierto `[desde, hasta)`, ausencia = abierto): la MISMA escalera R1-R7 que reificó `Resena` aquí NO sube.
- **Tres tiempos, dos relojes.** Evento (cuándo ocurrió), validez (cuándo fue cierto — el grafo), registro (cuándo lo supo el sistema — el WAL). Bitemporal = dos ejes, no dos fechas: el caso de Dani responde dos verdades legítimas.
- **Presente y AS OF pagan el MISMO barrido.** 28 = 28 lecturas: el filtro se aplica sobre datos ya leídos; sin índice no hay atajo — y eso ES la lección.
- **La historia cobra 1 `get_edge` por arista vencida** en cada barrido que toca a su persona: 21→20, 28→27, candidatas 4→3. Borrar es gratis… hasta que alguien pregunta por el pasado.
- **Caducar sin borrar.** La respuesta vieja se conserva, el presente no cambia, y la REGLA DE ORO (respuestas viejas intactas) se verifica con la regresión triple.
- **La tabla AS OF es la figura del capítulo.** Cinco filas —2026/2024/2023/2020/2019— que cuentan la película entera: quién estaba, quién se fue, quién no había llegado, y a qué precio en lecturas.

## 43.12 Ojo, cuidado con…

- **Confundir borrar con caducar.** El presente idéntico no prueba nada: el test de contraste pregunta por el pasado.
- **Tratar el `hasta` como inclusivo.** `[2018, 2024)` deja FUERA el 2024: por eso AS OF 2024 responde GrafoLuna. El medio abierto se respeta en TODA la implementación.
- **Creer que una fecha en un string es una fecha.** Un string no se filtra por rango: la lección P6 repetida.
- **Usar el reloj de verdad en el código.** `SystemTime::now()` rompe el determinismo del cap. 34; el «ahora» es `ANIO_ACTUAL = 2026`.
- **Pedirle al WAL el histórico de valores corregidos.** El WAL sabe lo que se escribió, no lo que se corrigió; para eso existe el Historico.
- **«Corregir» la P4 atemporal.** Sigue devolviendo Beto→Neurónica por contrato; la diferencia con la P4 CON tiempo (Beto→GrafoLuna) ES el capítulo — si el test de regresión fallara, el error está en el test, no en el modelo.

## 43.13 Pin de batalla

> *«Un grafo sin tiempo no miente: simplemente no sabe que el tiempo pasó. Caducar sin borrar es darle al cuándo un lugar donde vivir — y el pasado, como la historia, se paga 1 lectura por arista vencida, hasta que alguien construye el índice.»*

## 43.14 Si solo lees 30 segundos

El grafo atemporal responde afiliaciones que ya no existen (P4 decía Beto→Neurónica cuando se fue en 2024). La cura: valid-time (valid time) en la arista —`desde_anio`/`hasta_anio`, intervalo medio abierto `[desde, hasta)`, ausencia = abierto— sobre las 10 `MEMBER_OF` de KB-Lira paso-3 (68 nodos, 158 aristas): la 53 Beto→Neurónica venció en 2024 y la 185 Beto→GrafoLuna (nodo 69) la sustituye. Tres tiempos: evento (cuándo ocurrió), validez (cuándo fue cierto), registro (cuándo lo supo el sistema — el WAL del cap-28). Bitemporalidad (bitemporal) = dos ejes: el caso de Dani — registro 2023 «desde 2019» vs corrección 2025 «desde 2021» — responde «¿qué creíamos en 2024?» → 2019 y «¿qué sabemos hoy?» → 2021. Consultas AS OF con coste en lecturas: presente y 2023 pagan el MISMO barrido (28 = 28); cada arista vencida añade 1 `get_edge` (21→20, 28→27, candidatas 4→3); borrar es gratis hasta que alguien pregunta por el pasado (`delete_edge(53)` destruye el AS OF 2023). Gancho cobrado: la nota vigente del Informe por años — 2024 → Some(7), 2025 → Some(8), 2026 → Some(8) — con el grano anual declarado: distinguir el 3 del 10 de marzo exige grano fino. Regresión triple: las 10 preguntas del cap-41 y las respuestas pineadas del cap-42 intactas; el validador paso-2 acepta el paso-3. 23 tests nuevos, workspace en **971 ALL_GREEN**, sin bench: la moneda son lecturas y conjuntos exactos. Fronteras: el índice temporal y la unicidad (cap. 44), la ingesta con transaction-time (cap. 45), RDF/quads (cap. 46).

## 43.15 Una historia pequeña

Tres de marzo de 2025. Fabio ha reescrito su reseña del «Informe de revisión por pares 2025»: la ronda 2, nota 8, contrarresta a la ronda 1, nota 7. El cap-42 te enseñó a ver la contrarresta como una arista `CONTRARRESTA` entre dos nodos `:Resena` — el QUÉ de la historia. Pero aquel grafo era una foto: podía decirte que la ronda 2 contrarrestaba a la ronda 1, no qué valía la nota el 3 de marzo. Este capítulo te dio el CUÁNDO: la ronda 1 reinó hasta 2025 (`hasta_anio:2025` en su `REALIZA`), y la ronda 2 solo comenzó a reinar cuando su `CONTRARRESTA` nació (`desde_anio:2025`). El 3 de marzo de 2025 valía la **nota 7**: la ronda 2 aún no había contrarrestado nada — y el grafo del cap-42 no podía decírtelo, no porque mintiera, sino porque no guardaba cuándo. Hoy, en tu KB-Lira temporal, esa pregunta tiene respuesta, con su grano declarado: en años, 2025 responde 8; en días, el 3 de marzo responde 7. El grano lo pone el dato, no el modelo — y tú ya sabes cuál es el tuyo.

## Ejercicios resueltos

**1. Retrieval sin pistas: los tres tiempos, DE MEMORIA, y clasifica 5 afirmaciones.** Cierra el libro y recita: **evento** = cuándo ocurrió el hecho; **validez (valid time)** = cuándo fue cierto en el mundo (lo que las aristas guardan); **registro (transaction time)** = cuándo lo supo el sistema (lo que el log guarda). Ahora clasifica: (a) «la ronda 2 se escribió en 2025» → **evento**. (b) «la nota 8 es vigente desde 2025» → **validez**. (c) «el commit del WAL registró la escritura» → **registro**. (d) «Dani se afilió a Neurónica en 2021» (la arista 55) → **validez**. (e) «en 2023 el registro anotó que Dani estaba desde 2019» (el ts 1 del Historico) → **registro**. Si clasificaste (d) como evento o (e) como validez, vuelve al §43.3: el orden ES el argumento.

**2. Explica por qué presente y AS OF pagan el mismo barrido y cuánto cuesta cada vencida.** Mecánica: `afiliaciones_vigentes_en` lee la adyacencia completa (`in_edges` + `get_edge` por arista candidata) y SOLO ENTONCES aplica `arista_vigente_en` — sin índice temporal, la adyacencia no distingue vigentes de vencidas, así que 2026 y 2023 barren lo mismo: 1 `in_edges` + 21 `get_edge` + 6 `get_node` = **28 = 28**. Cada arista vencida conservada sigue en la adyacencia de su persona y se lee antes de descartarse: medido contra la variante sin la 53, 21→20 `get_edge`, 28→27 totales y candidatas `MEMBER_OF` 4→3. La tesis del contrato (13→14 contando solo candidatas) se adaptó al ledger real con su comentario: la moneda completa es el barrido, y la lección no cambia — con 10M de vencidas, 10M de lecturas.

**3. Responde por qué el caso de Dani tiene dos respuestas legítimas.** Mecánica: la arista 55 dice `desde_anio:2021` (lo cierto, el eje VALIDEZ); el `HistoricoAfiliaciones` guarda dos entradas — ts 1 (2023) «desde 2019» y ts 2 (2025) «desde 2021» (el eje REGISTRO). `afiliacion_segun_registro(historico, Dani, 2024, 1)` responde `(Neurónica, 2019)`: lo que el sistema CREÍA en 2024; con `ts = 2` responde `(Neurónica, 2021)`: lo que se sabe tras la corrección. No es contradicción: son dos preguntas distintas sobre dos ejes ortogonales — bitemporalidad (bitemporal) es eso, no «dos fechas en la misma fila».

## Ejercicios propuestos

**Esencial (recordar + aplicar; 34+41+43).** Desarrolla el reto esencial del §43.10: PREDICE por escrito las respuestas y las lecturas de `afiliaciones_vigentes_en` para AS OF 2020 y AS OF 2024 ANTES de correr `informe_temporal_reproducible`. Criterio: predicción escrita primero; si tu predicción de 2024 dijo Neurónica, revisa la semántica medio abierta `[desde, hasta)`; si tu predicción de lecturas no distinguió 27 de 28, revisa cuándo se lee (o no) el nodo de la organización de Dani.

**Intermedio (predecir y comparar; 30+43 y 41+43).** (a) Compara `ts_begin/ts_end` del cap-30 con `desde_anio/hasta_anio`: escribe qué es lo mismo y qué es distinto entre el versionado por CONCURRENCIA (snapshots, gc) y el valid-time por HISTORIA. (b) Aplica el patrón a `WORKED_ON`: ¿el Proyecto Brújula «nació» en 2022? Justifica con la escalera R1-R7 si las aristas de Elena llevan `desde_anio` o si el nacimiento del proyecto es otro hecho. Criterio: cada respuesta con su porqué y su coste en lecturas.

**Experto (crear y demostrar; 28+43).** Registra la corrección de Dani con `WalTransaccion` REAL del cap-28 (la entrada que el builder puso a mano en `historico_kb_lira_paso3`), simula el corte de luz con `as_bytes`/`reconstruir`/`replay_wal` y demuestra en un test qué puede y qué NO puede responder el WAL sobre «¿qué creíamos en 2024?». Restricciones: std puro, sin tocar cap-28, suite ALL_GREEN con tu test dentro. Criterio: el test debe declarar la frontera (LSN ≡ ts_registro, Commit ≡ entrada, replay ≡ re-lectura) y fallar si alguien intenta leer la corrección del WAL.

## Para profundizar

- **Richard T. Snodgrass, *Developing Time-Oriented Database Applications in SQL*, Morgan Kaufmann, julio 1999 (ISBN 1-55860-436-7)** — el cap. 1 con los tres tipos de tiempo, los intervalos y la convención medio abierta; la referencia canónica de la casa.
- **Christian S. Jensen y Richard T. Snodgrass, «Temporal Data Management», IEEE TKDE 11(1):36-44, enero/febrero 1999 (DOI 10.1109/69.755613)** — valid time vs transaction time y el fundamento de la bitemporalidad.
- **Richard T. Snodgrass (ed.), *The TSQL2 Temporal Query Language*, Kluwer Academic Publishers, 1995 (ISBN 0-7923-9614-6)** — los dos ejes como tipos del estándar.
- **ISO/IEC 39075:2024 (GQL), abril 2024** — tipos de datos temporales (date/datetime/duration) y funciones temporales en la Parte 1; las consultas AS OF no están en esa parte (frontera declarada del contrato).
- **Neo4j 3.4 (2018), docs de tipos temporales** — DATE, LOCAL/ZONED TIME, LOCAL/ZONED DATETIME, DURATION, indexables con range lookups.
- **Petter Holme y Jari Saramäki, «Temporal networks», Physics Reports 519(3):97-125, octubre 2012 (DOI 10.1016/j.physrep.2012.03.001)** — la anécdota del §43.0 y «moving the information of *when* things happen from the dynamical system on the network, to the network itself».
- **Vassilis Kostakos, «Temporal graphs», Physica A 388(6):1007-1023, 2009 (DOI 10.1016/j.physa.2008.11.021)** — temporal graphs y la sobreproyección del grafo estático.
- SQL:2011 (periods y temporal tables) — contexto industrial citado sin detalle firme: frontera declarada del contrato.
- Dentro del libro: cap. 41 (escalera R1-R7, las 10 preguntas, P4 atemporal), cap. 42 (los refactors, la reseña reificada, la regresión), cap. 28 (WAL, Vol. II), cap. 30 (MVCC: el contraste versionado-concurrencia vs historia), cap. 32 (CSV round-trip), cap. 26 (ContandoStore), cap. 34 (dataset determinista), cap. 9 (Value sin tipo fecha: frontera del grano).

## Mini-diálogo: en guardia nocturna

> — Son las tres de la madrugada. El equipo pide «¿quién pertenecía a Neurónica en 2023?» y la consulta responde solo Ana y Dani.
>
> — ¿Y Beto?
>
> — Eso es. Beto trabajaba en Kira desde 2018 y era de Neurónica… hasta que alguien hizo `delete_edge(53)` cuando se fue en 2024. «Limpiamos el grafo», dijeron.
>
> — (pausa) El presente sigue perfecto, ¿verdad? Beto responde GrafoLuna por la arista 185.
>
> — Sí, por eso nadie se dio cuenta. La suite está verde.
>
> — La suite está verde y la historia está muerta. Borrar es gratis… hasta que alguien pregunta por el pasado. La arista 53 no se borra: se CADUCA — `hasta_anio: 2024`, y el AS OF 2023 la sigue viendo. El borrado destruye la respuesta; la caducidad la conserva, y cuesta exactamente 1 lectura por arista vencida en cada barrido.
>
> — O sea que la historia se paga.
>
> — Se paga en lecturas, y el cap-44 construirá el índice que la abarata. Por ahora, restaura la arista. Buenas noches.

> *Siguiente parada, cap. 44 (constraints e índices temporales): el AS OF sin índice paga 1 lectura por cada arista vencida — ¿quién construye el índice que lo abarata? ¿Y quién garantiza que dos afiliaciones no se solapen en el tiempo? Preguntas que dejamos abiertas: ¿quién anota el transaction-time automáticamente al importar el lote? (cap. 45, ingesta). ¿Y cuándo el KG temporal se convierte en la memoria de un agente? (cap. 53).*
# Capítulo 44 — Esquema, constraints e índices en property graphs

> *«Cuarto capítulo del Volumen III. Si vienes del cap. 43 —o del perfil datos/IA que lo haya leído en diagonal— los validadores por composición, la siembra del paso-3, el AS OF a 28 lecturas y la regresión como red de seguridad son tus HERRAMIENTAS, no tu contenido: el modelo ya es temporal y ahora se le añade el CONTRATO. El cierre del cap-43 te dejó dos preguntas clavadas: "AS OF paga 1 lectura por arista vencida — ¿quién construye el índice que lo abarata?" y "¿quién garantiza que dos afiliaciones no se solapen?". Este capítulo responde CON DATOS: la regla **SinSolape** del esquema declarativo —la unicidad temporal que el cap-43 declaró deuda— y dos índices con sus ledgers: el de adyacencia por etiqueta baja el AS OF persona-céntrico de Kira de **28 a 22 lecturas** (la vencida 53 sigue costando 1 lectura: el intervalo `[desde, hasta)` cobra el otro lado), y la consulta global "¿quién se afilió desde el año X?", reescrita para casar con un índice sobre `desde_anio`, baja de **12 a 3 lecturas**. Y con la honestidad de siempre: el índice de año NO casa con la pregunta persona-céntrica — **28 = 28** con y sin él, y su mantenimiento se cobra igual. De paso se cobra la deuda del cap-42: la especificidad por label es decisión de ESQUEMA, y el esquema decide que `MENTIONS` sigue polimórfico.»*

## 44.0 La anécdota de la esquina

En febrero de 2015, Baron Schwartz publicó en el blog de SolarWinds un post que se convirtió en un clásico incómodo: **«Schemaless Databases Don't Exist»** (las bases de datos sin esquema no existen). Su tesis, con la que abre el post: **"there is always a schema somewhere. Usually in multiple places"** — siempre hay un esquema en algún sitio; normalmente en varios sitios a la vez. Cuando una base de datos se vende como «schemaless» (sin esquema), no quiere decir que no tenga esquema: quiere decir que el esquema no está en la base de datos, y entonces está en el código de la aplicación, en las convenciones no escritas del equipo, o —el peor caso— en los datos rotos que nadie ha denunciado.

Piensa en el cajón del taller donde «todo cabe». Todo cabe hasta que necesitas encontrar algo: entonces el cajón sin compartimentos no te dice dónde está nada, y un tornillo suelto puede ser de cualquier mueble. Tu KB-Lira ha sido ese cajón durante tres capítulos: el cap-41 construyó el modelo y sus **validadores** (funciones Rust que recorren el grafo denunciando violaciones), el cap-42 pagó los antipatrones y el cap-43 añadió el tiempo. Cada validador era, sin decirlo, una regla de esquema: «MEMBER_OF exige `desde_anio`», «una CITES va de Documento a Documento» — escritas como `if`s dentro de funciones. El esquema de KB-Lira existía, sí: disfrazado de código, y nadie lo auditaba.

La pregunta que ordena este capítulo: **tus validadores son funciones en Rust — ¿y si fueran datos?** Un `if` no se puede serializar, ni contar, ni comparar contra el modelo; un dato sí. Este capítulo saca las reglas del código y las convierte en DATOS —y de paso construye los atajos que hacen que las reglas y las consultas no cuesten caras.

## 44.1 Objetivo

Objetivo medible del outline: **elegir entre esquema abierto y esquema estricto (schema) y usar constraints e índices sin pagar de más**. Al terminar tendrás:

1. **El esquema como DATO**: `Esquema` = `Vec<ReglaConstraint>` con **6 variantes** (Extremos, Existencia, Tipo, Unicidad, SinSolape, IntervaloValido) y `esquema_kb_lira()`: **16 reglas** (6 Extremos, 5 Existencia, 2 Tipo, 1 Unicidad, 1 SinSolape, 1 IntervaloValido).
2. **El verificador genérico** `verificar_esquema(store, esquema)`, un intérprete de reglas que REUTILIZA `Violacion` del cap-41 tal cual.
3. **El builder paso-4** `kb_lira_paso4()`: el paso-3 + `orcid` único a las **9 personas** — 68 nodos, 158 aristas intactos (el paso-4 modela REGLAS, no entidades).
4. **El caso ORCID**: la unicidad (uniqueness) y la existencia (existence) son DOS reglas distintas — un duplicado se rechaza con id; una persona sin orcid NO es un duplicado.
5. **La `ReglaSinSolape`**: el gancho del cap-43 cobrado — dos afiliaciones solapadas son violación; dos contiguas, legales (`[desde, hasta)` medio abierto).
6. **La subsunción demostrada**: sobre un fixture corrupto, la cadena de validadores 41-43 y el esquema denuncian EXACTAMENTE los mismos ids `{16, 21, 52}`.
7. **`IndiceDesdeAnio`**: la consulta global reescrita pasa de **12 a 3 lecturas**; el AS OF persona-céntrico sigue en **28 = 28** y paga 10 de mantenimiento — el desajuste ES contenido.
8. **`IndicePorLabel`**: el AS OF del cap-43 baja de **28 a 22 lecturas**; amortización: `158 + 22n < 28n` desde **n = 27**.
9. **La selectividad del cap-21 como juez**: `Documento.anio` = 5/36 ≈ **0,139** (índice NO construido), `Persona.orcid` = 9/9 = **1,0**, `Tema.nombre` = 8/9 ≈ **0,889** — el catálogo delata un duplicado residual.
10. **CSV paso-4 + informe reproducible** y el workspace en **993 tests ALL_GREEN** (971 + 22), sin bench.

## 44.2 Problema

Mira el resultado del cap-43: 971 tests verdes, el modelo temporal, el AS OF respondiendo a 28 lecturas. Y sin embargo: **el AS OF del cap-43 paga 1 lectura por cada arista vencida** — la factura de la historia que el propio capítulo prometió que este cobraría con un índice. Nadie lo ha construido. Y hay más: nada en KB-Lira impide que **dos Anas** entren en el grafo — sin una regla de unicidad, el modelo no sabe que dos personas con el mismo nombre son (o no) la misma. El grafo acepta cualquier cosa, y la P10 del cap-41 puede callarse silenciosamente si un documento pierde su `anio`: el validador se ejecuta bajo demanda, y nadie lo invoca al importar.

Antes de dibujar nada, desactivemos las **seis** ideas equivocadas que suelen venir con el tema:

1. **«Los grafos son schemaless: no hacen falta esquemas.»** No: el esquema SIEMPRE existe (Schwartz 2015: "there is always a schema somewhere"). Si no está en la base de datos, está en el código o en los datos rotos. Los validadores de los caps. 41-43 eran el esquema de KB-Lira disfrazado de código — este capítulo lo saca del disfraz.
2. **«Un índice es para hacer más rápidas las consultas: no cambia nada más.»** No: el índice cambia el COSTE, nunca la respuesta; el **constraint** cambia QUÉ datos pueden existir. Confundirlos es el error central del capítulo: el constraint UNIQUE es la garantía; el índice solo la hace barata — y en las BBDD reales, el UNIQUE crea su propio índice (Neo4j lo documenta).
3. **«Si añado un índice, el AS OF del cap-43 se abarata solo.»** No: el índice debe CASAR con la forma de la consulta. El índice global sobre `desde_anio` no toca la consulta persona-céntrica: **28 = 28** con y sin él. Un índice que no casa con ninguna consulta sigue cobrando mantenimiento.
4. **«Un índice sobre una propiedad sirve siempre.»** No: la selectividad lo decide. `Documento.anio` tiene 5 valores en 36 nodos: un índice por valor devuelve ~7 documentos — no discrimina. El catálogo del cap-21 decide ANTES de construir.
5. **«La unicidad es de una propiedad, punto.»** No: «una persona, una organización» NO es la regla de KB-Lira; lo es «una persona, una organización, EN UN MISMO INTERVALO». Beto en Neurónica y en GrafoLuna es legítimo en años distintos: la unicidad temporal es de INTERVALOS, no de pares — el gancho «¿quién garantiza que dos afiliaciones no se solapen?» del cap-43.
6. **«Hay que reescribir los validadores 41-43 como esquema.»** No: el esquema es la MISMA regla como dato; los validadores se quedan intactos (regresión dura), y el test de equivalencia demuestra que el esquema los subsume sin tocarlos.

Y el compromiso de honestidad que rige TODO el capítulo — espejo del cap-43: los números salen de `cargo test`, nunca de la pizarra; el contrato anunciaba 168→18 y 28→16 con otras bases de conteo, el ledger real mide **12→3** y **28→22**, y se pine el delta real con su porqué, prohibido maquillar contadores.

## 44.3 Modelo mental: el contrato y el atajo

El esquema es el **CONTRATO** del grafo (qué datos pueden existir); el índice es el **ATAJO** (cuánto cuesta encontrarlos). Un constraint y un índice son dos herramientas que la gente confunde porque en las BBDD reales viajan juntas — el UNIQUE crea su índice. El panel que ordena todo el capítulo:

```text
DOS PREGUNTAS DISTINTAS (la confusión que este capítulo deshace):
  CONSTRAINT  «¿PUEDE existir este dato?»   → si NO: violación con id, el grafo NO responde
              orcid duplicado · Documento sin anio · MEMBER_OF sin desde_anio
              · dos afiliaciones de Beto solapadas en [2018, 2024)
  ÍNDICE      «¿CUÁNTO CUESTA encontrarlo?» → si existe o no, la RESPUESTA es la misma:
              SOLO cambia el ledger de lecturas
              AS OF por proyecto: 28 → 22 (IndicePorLabel) · global reescrita: 12 → 3
              (IndiceDesdeAnio) · índice que no casa: 28 = 28 y paga mantenimiento
```

EL CONTRATO DE KB-LIRA — lo que los validadores 41-43 sembraron, ahora declarado como datos (16 reglas):

```text
Extremos:   AUTHORED Persona→Documento · CITES Documento→Documento · ABOUT Documento→Tema
            MENTIONS Documento→{Persona|Organizacion|Proyecto} · MEMBER_OF Persona→Organizacion
            WORKED_ON Persona→Proyecto        (MENTIONS: hasta_labels, la lista del `any`)
Existencia: Documento.titulo · Documento.anio · Persona.nombre · Organizacion.nombre · Tema.nombre
Tipo:       Documento.anio:Int · Documento.titulo:String
Unicidad:   Persona.orcid      (9/9 personas la cumplen; un duplicado es violación)
SinSolape:  MEMBER_OF por persona, intervalos [desde, hasta) disjuntos
            (Beto [2018,2024) y [2024,∞): SÍ · dos intervalos que se pisen: NO)
IntervaloValido: MEMBER_OF con desde_anio:Int REQUERIDO, desde ≤ 2026, hasta ≥ desde
```

LOS DOS ÍNDICES Y SU FACTURA (moneda = lecturas):

```text
IndiceDesdeAnio  BTreeMap<anio, Vec<arista>>  global reescrita: 12 → 3
                 (poda por desde; el hasta se paga por candidata — el intervalo cobra)
                 AS OF persona-céntrica: 28 = 28 + 10 de mantenimiento = 38  ← NO casa
IndicePorLabel   adyacencia por etiqueta (el CSR lógico del cap-14)  AS OF: 28 → 22
                 (el salto persona→org deja de leer las 2 MENTIONS y las 12 aristas ajenas)
IndiceAnioDoc    NO SE CONSTRUYE: selectividad 5/36 ≈ 0,139 — decoración con factura
```

Y debajo, **la regla del índice**, la lección de la sección 3 del outline: el índice debe casar con el PATRÓN DE ACCESO — la clave del índice es el eje de la pregunta. Índice que no se usa = solo mantenimiento. La **REGLA DE ORO** heredada (determinismo total, cap-34): el paso-4 es un artefacto DERIVADO del paso-3 (mismo store, 9 props `orcid` añadidas, cero nodos/aristas nuevos); cada consulta devuelve su coste pineado; las 10 preguntas del cap-41 + los pines del cap-42 + el validador paso-3 son la red de seguridad: si el esquema cambia una respuesta vieja, el cambio está MAL. El momento ¡ajá! perseguido: **«los validadores de los capítulos 41-43 eran el esquema disfrazado de código: la regla "MEMBER_OF exige desde_anio" escrita como un `if`. El esquema es la misma regla como DATO — y por ser dato se puede verificar, serializar y discutir. Y el cap-21 ya lo sabía: su `EqIndexEntry` era este índice sin construir.»**

## 44.4 Primera solución

La primera solución — y la que aplica casi todo el mundo — es **la NO-solución doble**:

**(a) Seguir SIN esquema.** El grafo abierto acepta todo: un orcid duplicado entra sin que nadie proteste, y dos Anas conviven felices. El modo de fallo es silencioso: la P10 del cap-41 (citas recientes que tratan un tema) deja de responder un documento cuando este pierde el `anio` — nadie lo ha borrado, la arista sigue ahí, pero el filtro `p.anio > X` la descarta en silencio. Un grafo sin esquema no MENTE: simplemente no sabe que algo está mal, y el test que lo cazaría no existe porque nadie declaró la regla. El cajón del taller, otra vez.

**(b) Seguir con el validador imperativo.** Lo que ya hay funciona: `validar_modelo_kb_lira_paso1 → paso2 → paso3`, cada capa REUTILIZANDO a la anterior. Pero añadir la regla nueva del paso-4 —«el orcid de una persona es único»— significa añadir OTRO bloque de `if`s al validador paso-3: recorrer personas, acumular valores en un `HashMap`, denunciar el segundo. Y la siguiente regla (el solape temporal), otra función más. Cada regla nueva reescribe el validador — el crecimiento imperativo que los caps. 41-43 ya mostraron capítulo a capítulo, sembrando las mismas comprobaciones a mano.

El capítulo te muestra ambas con sus modos de fallo ANTES de la solución: una deja entrar la basura sin decírtelo, la otra te obliga a reescribir código por cada regla.

## 44.5 Sus límites

La no-solución doble tiene cuatro límites que la delatan:

1. **El grafo abierto no distingue.** Sin orcid, dos Anas son indistinguibles: no hay forma de saber si son la misma persona o dos homónimas, y ninguna consulta puede deshacer la ambigüedad. El esquema no es estética: es la diferencia entre un dato y un ruido.
2. **El validador imperativo crece sin reutilizarse.** Cada capítulo re-sembró las reglas a mano: el paso-1 valida extremos y existencia, el paso-2 re-filtra tipos, el paso-3 añade las temporales. La duplicación crece con cada regla, y las reglas no tienen nombre: son `if`s anónimos dentro de funciones que nadie puede listar, contar ni serializar.
3. **La subsunción incompleta, descubierta en el acto.** Al declarar el esquema se destapó un agujero: el validador paso-3 del cap-43 exigía TRES condiciones temporales (`desde_anio` requerido, `desde ≤ ANIO_ACTUAL`, `hasta ≥ desde`) y el esquema inicial —con 5 variantes— no tenía ninguna regla que las expresara. Las reglas vivían en el código y no tenían nombre: el esquema implícito que nadie ve. La sexta variante, **`IntervaloValido`**, nació de ese descubrimiento: añadir una condición temporal ya no toca el verificador, solo el catálogo.
4. **La promesa del cap-43 sigue sin responder.** El AS OF paga 28 lecturas, la factura de la historia se cobra en cada barrido, y nadie ha construido el índice que la abarata.

## 44.6 Solución evolucionada

Ocho piezas, cada una un TRADE-OFF con precio en lecturas o en garantías:

**1. El esquema como DATO.** `ReglaConstraint` con **6 variantes** y `verificar_esquema` como intérprete genérico: recorre el grafo UNA vez por regla y devuelve la lista COMPLETA de `Violacion` (la del cap-41, reutilizada tal cual). El detalle que nació del modelo real: `Extremos` lleva `hasta_labels: Vec<String>` — la lista con semántica CUALQUIERA-uno, porque `MENTIONS` es polimórfico (→Persona|Organizacion|Proyecto, decisión #7 del cap-41) y tres reglas separadas serían un AND (el destino tendría que llevar los tres labels a la vez). El gancho del cap-42, cobrado: **la especificidad por label es decisión de ESQUEMA**, y el esquema decide NO refinar el polimorfismo — la regresión (P6 no cambia) lo avala.

**2. El builder paso-4.** `kb_lira_paso4()` llama a `kb_lira_paso3()` tal cual y añade `orcid:String` a las **9 personas** (ids 0-5 y 30-32), con formato `XXXX-XXXX-XXXX-XXXX` determinista por id (Ana = `0000-0001-2345-0001`, Beto = `0000-0002-3456-0002`…). Cero nodos y aristas nuevos: **68 nodos, 158 aristas** — los pines del paso-3 intactos. El canónico pasa su propio esquema con `Ok(())`: el esquema describe el modelo real, no un ideal.

**3. El caso ORCID: unicidad ≠ existencia.** La regla `Unicidad` se verifica con un **HashMap interno** que recuerda el PRIMER id de cada valor; el segundo es la violación — LA lección estructural: **UNIQUE ≡ índice**. Sin un índice por debajo, la unicidad ES este escaneo que construye el índice sobre la marcha. El fixture lo demuestra con dos mutaciones sobre la copia: (a) el orcid de Ana se copia a Beto → UNA violación, la de Unicidad, con `id_implicado = 1` (el SEGUNDO: el índice ya lo tenía Ana): `Persona con propiedad 'orcid' repetida (mismo valor que el nodo 0; Unicidad)`; (b) a Carla se le BORRA el orcid → violación de **Existencia**, no de unicidad: `Persona sin propiedad 'orcid' (exigida por Existencia)`. Una persona sin orcid NO es un duplicado: son dos reglas distintas sobre la misma propiedad.

**4. La `ReglaSinSolape`: el gancho del cap-43 cobrado.** Agrupa `MEMBER_OF` por par (origen, destino) y compara intervalos con el criterio de solape `a.desde < b.hasta && b.desde < a.hasta`, medio abierto `[desde, hasta)` — la convención del cap-43, ahora ejecutada por una regla. La línea de Beto, que el cap-43 dibujó a mano como semántica, ahora la GARANTIZA una regla del esquema:

```text
LA LÍNEA DE BETO (la unicidad temporal que SÍ se cumple, [desde, hasta)):
  2018─────────────────────2024──────────────2026
  [─ Neurónica (53) ─────) [─ GrafoLuna (185) ─)        ← disjuntos: LEGAL
  [─ Neurónica (190) ─)  dentro de [2018,2024)          ← SOLAPE: VIOLACIÓN
```

La arista nueva 190 (Beto→Neurónica, `[2022, 2023)`) solapa con la 53 (`[2018, 2024)`): violación con `id_implicado = 190`, la segunda en el escaneo:

```text
MEMBER_OF 1→7: la arista 190 solapa en [2022, 2023) con la arista 53 en [2018, 2024) (SinSolape)
```

Y el borde del intervalo: la 190 con `desde_anio:2024` y `hasta_anio` AUSENTE (`[2024, +∞)`) NO solapa con la 53 — `2024 < 2024` es falso; dos afiliaciones contiguas —una termina exactamente donde empieza la otra— son legales. La línea de Beto sigue siendo legal: `[2018,2024)` y `[2024,∞)` disjuntos.

**5. La subsunción, demostrada.** Fixture corrupto a mano sobre el paso-4 clonado: (a) la CITES 16 con el origen sustituido por el Tema 24; (b) el Documento 21 sin `titulo`; (c) la MEMBER_OF 52 (Ana→UniLira) sin `desde_anio`. La cadena de validadores 41-43 (`validar_modelo_kb_lira_paso3`) y el esquema fallan AMBOS, y los conjuntos de ids implicados COINCIDEN: `{16, 21, 52}`. **Tres validadores por composición, un esquema lo declara.** La dirección importa: el esquema puede denunciar MÁS que el validador (subsume), nunca MENOS — si se quedara corto, la regla que falta se añade al CATÁLOGO, no al verificador.

**6. `IndiceDesdeAnio`: el índice que casa con la consulta GLOBAL.** Un `BTreeMap<i64, Vec<EdgeId>>` sobre `MEMBER_OF.desde_anio`; construirlo son **10 lecturas** (las 10 `MEMBER_OF` del barrido). La consulta global «¿qué afiliaciones empezaron desde el año X?» se REESCRIBE para usar el rango del mapa: en 2024, barrido **12 → 3 lecturas** (×4: 10 `get_edge` + 2 `get_node` contra rango gratis + 1 `get_edge` + 2 `get_node`). Honestidad declarada: el contrato anunciaba **168 → 18** contando el fetch de TODAS las 158 aristas del store; aquí el barrido solo consume las 10 `MEMBER_OF` — la lección es la misma (de O(n) a O(resultado)), la base es otra, y el delta real se pine con su comentario. Y el caso que NO casa, medido: la consulta persona-céntrica del cap-43 en 2023 sigue en **28 = 28** — el índice responde por AÑO (`desde_anio ≥ anio`), la pregunta es por PERSONA; su rango para 2023 devuelve `[184, 185]` (la 184 de Iris es ajena al proyecto y la 185 de Beto no está vigente en 2023) y NINGUNA de las vigentes del proyecto (52, 53, 55, todas con `desde < 2023`) está en él. Índice construido y no usado: la factura del mantenimiento se cobra igual — **28 + 10 = 38** contra 28 desnudo.

**7. `IndicePorLabel`: el índice que sí abarata el cap-43.** La adyacencia particionada por etiqueta — el CSR del cap-14 (Vol.II) traído al modelo. Construirlo son **158 lecturas** (todas las aristas, agrupadas por tipo). La misma consulta persona-céntrica reescrita: el salto usa `aristas_de_tipo("WORKED_ON")` y `aristas_de_tipo("MEMBER_OF")` y deja de leer lo que no es historia — las 2 MENTIONS del `in_edges` y las 12 aristas salientes ajenas del cap-43. Ledger real en 2026: 6 `get_edge` (TODAS las WORKED_ON: 3 de Kira + 3 de Oráculo/Brújula leídas y descartadas — el bucket del tipo es global) + 3 `get_node` (personas) + 10 `get_edge` (TODAS las MEMBER_OF; **la vencida 53 sigue costando su lectura**: el intervalo `[desde, hasta)` cobra el otro lado) + 3 `get_node` (organizaciones) = **22 < 28**. El contrato anunciaba 16; la medida real añade las 3 WORKED_ON ajenas y los 3 `get_node` de las organizaciones: **22**, pineado con su comentario. La amortización, con números: una consulta única es más cara (158 + 22 = 180 > 28); el índice se paga cuando la pregunta se REPITE — `158 + 22n < 28n` desde **n = 27** (158/6 ≈ 26,3).

**8. La selectividad del catálogo (cap-21) como juez.** `selectividad_de_propiedad` mide los tres ratios reales del paso-4:

```text
Documento.anio → (36, 5, ≈0,139)   BAJA: 36 docs, solo 5 años — un índice por valor
                                   devuelve ~7 documentos: NO discrimina → NO se construye
Persona.orcid  → (9, 9, 1,0)       UNIQUE ≡ índice perfecto: la unicidad YA es un índice
Tema.nombre    → (9, 8, ≈0,889)    ¡el catálogo DELATA UN DUPLICADO RESIDUAL: los temas
                                   26 y 61 comparten «memoria de agentes»!
```

El catálogo del cap-21 lo habría sabido ANTES: un `IndiceAnioDocumento` no reduce las lecturas de P10 (su otro eje es el tema) y cuesta mantenimiento — **el catálogo no solo decide índices: descubre datos**. El duplicado 26/61 es el residuo del refactor A del cap-42 (el subtema que duplicó al tema del paso-1), invisible para los validadores y delatado por una simple cuenta de valores distintos.

**9. La REGLA DE ORO, verificada.** Regresión cuádruple: las 10 preguntas del cap-41 sobre el subgrafo paso-1 IDÉNTICAS (`orcid` no filtra nada), las respuestas pineadas del paso-2 intactas, `validador_paso3_acepta_el_modelo_paso4`, y el esquema acepta el modelo canónico de TODOS los pasos. Declarar el esquema no cambia UNA respuesta vieja.

## 44.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap44_esquema.rs` (**2.551 líneas**, std puro, **22 tests** en 9 módulos), cableada con dos líneas aditivas en `lib.rs` (`mod cap44_esquema;` + `pub use cap44_esquema::*;`). El artefacto regenerable `datasets/kb-lira/paso-4/{nodes.csv (69 líneas), edges.csv (159 líneas), esquema.csv (16 reglas)}` es la salida del builder: los índices y el esquema NO viven en el grafo — se regeneran. El `esquema.csv` tiene formato propio mínimo (`tipo_regla,label,propiedad,valor[,extra]`, `hasta_labels` unidas con `;`) con exportador y round-trip byte a byte: **el esquema es DATO y se serializa**. CERO dependencias nuevas, CERO cambios en caps. 7-43, goldens intactos. Y **NO hay `[[bench]]`**: decisión #9 del contrato en una línea — la moneda son lecturas, conjuntos y violaciones exactos, nunca µs (espejo de las decisiones #12 del cap-41 y #11 del cap-43).

Las piezas que sostienen el edificio (nombres exactos; el código completo vive en el módulo):

```rust
pub enum ReglaConstraint { Extremos{rel_tipo, desde_label, hasta_labels: Vec<String>},
    Existencia{label, propiedad}, Tipo{label, propiedad, esperado}, Unicidad{label, propiedad},
    SinSolape{rel_tipo, desde_anio_prop, hasta_anio_prop},
    IntervaloValido{rel_tipo, desde_prop, hasta_prop, anio_max} }
pub struct Esquema { pub reglas: Vec<ReglaConstraint> }
pub fn esquema_kb_lira() -> Esquema;                              // 16 reglas (6/5/2/1/1/1)
pub fn verificar_esquema(store: &dyn GraphStore, esquema: &Esquema) -> Result<(), Vec<Violacion>>;
pub fn kb_lira_paso4() -> (MemoryStore, Esquema);                 // paso-3 + orcid a 9 personas
pub struct IndiceDesdeAnio;                                        // BTreeMap<i64, Vec<EdgeId>>
pub fn afiliaciones_globales_desde_anio(...) -> (Vec<(String,String,i64)>, usize);      // 12
pub fn afiliaciones_globales_desde_anio_con_indice(...) -> ...;   // 3
pub struct IndicePorLabel;                                         // HashMap<String, Vec<EdgeId>>
pub fn afiliaciones_vigentes_en_con_indice_por_label(...) -> (Vec<(String,String)>, usize); // 22
pub fn selectividad_de_propiedad(store, label, propiedad) -> (usize, usize, f64);
pub fn informe_esquema_reproducible(store: &dyn GraphStore, esquema: &Esquema) -> String;
```

## 44.8 Prueba de fuego

Primero el bucle rápido — salida REAL de `cargo test`, sin tiempos:

```text
$ cargo test -p vol2-liradb --lib cap44

running 22 tests
test cap44_esquema::tests_esquema::verificar_esquema_acepta_el_modelo_sin_reglas ... ok
test cap44_esquema::tests_esquema::verificar_esquema_detecta_extremos_y_existencia ... ok
test cap44_esquema::tests_esquema_paso4::kb_lira_paso4_pasa_su_propio_esquema ... ok
test cap44_esquema::tests_esquema_paso4::orcid_duplicado_viola_la_unicidad_con_el_id_del_segundo ... ok
test cap44_esquema::tests_esquema_paso4::persona_sin_orcid_viola_la_existencia ... ok
test cap44_esquema::tests_esquema_paso4::dos_member_of_solapadas_violan_sin_solape_con_el_id_de_la_segunda ... ok
test cap44_esquema::tests_esquema_paso4::afiliaciones_contiguas_no_violan_sin_solape ... ok
test cap44_esquema::tests_esquema_paso4::el_esquema_subsume_la_cadena_de_validadores_sobre_fixture_corrupto ... ok
test cap44_esquema::tests_indice::indice_desde_anio_acota_candidatas_y_cuesta_su_construccion ... ok
test cap44_esquema::tests_indice_consulta_global::el_indice_simple_abaratara_la_consulta_global_reescrita ... ok
test cap44_esquema::tests_indice_consulta_global::el_indice_simple_no_abaratara_el_as_of_persona_centrica ... ok
test cap44_esquema::tests_indice_por_label::el_indice_por_label_abaratara_el_as_of_del_cap43 ... ok
test cap44_esquema::tests_selectividad::la_selectividad_del_catalogo_decide_cuando_un_indice_estorba ... ok
test cap44_esquema::tests_regresion_esquema::las_10_preguntas_del_paso1_no_cambian_tras_anadir_esquema ... ok
test cap44_esquema::tests_regresion_esquema::las_respuestas_del_paso2_no_cambian_tras_anadir_esquema ... ok
test cap44_esquema::tests_regresion_esquema::las_respuestas_del_paso3_no_cambian_tras_anadir_esquema ... ok
test cap44_esquema::tests_regresion_esquema::el_esquema_acepta_el_modelo_canonico_de_todos_los_pasos ... ok
test cap44_esquema::tests_csv_paso4::csv_roundtrip_paso4_import_export_byte_a_byte ... ok
test cap44_esquema::tests_csv_paso4::csv_esquema_roundtrip_byte_a_byte ... ok
test cap44_esquema::tests_csv_paso4::csv_paso4_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap44_esquema::tests_csv_paso4::csv_pasos_anteriores_intactos_tras_paso4 ... ok
test cap44_esquema::tests_informe_esquema::informe_esquema_reproducible_sobre_kb_lira ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 971 filtered out
```

Veintidós verdes; workspace entero en **993 ALL_GREEN** (971 + 22) con goldens intactos. Ahora el informe del capítulo — salida REAL de `informe_esquema_reproducible`, pineada byte a byte:

```text
Esquema declarativo de KB-Lira (paso-4): las reglas como datos
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
Extremos                       | 6 reglas
Existencia                     | 5 reglas
Tipo                           | 2 reglas
Unicidad                       | 1 regla
SinSolape                      | 1 regla
IntervaloValido                | 1 regla
Total                          | 16 reglas
Caso ORCID                     | 9 personas con orcid único — selectividad 9/9 = 1,0 (UNIQUE ≡ índice)
IndiceDesdeAnio · global 2024  | construcción 10 lecturas — barrido 12 → 3 lecturas con el índice
IndiceDesdeAnio · AS OF 2023   | 28 = 28 lecturas: sin mejora — + 10 de mantenimiento = 38
IndicePorLabel · AS OF 2026    | construcción 158 lecturas — 28 → 22 lecturas con el índice
Selectividad Documento.anio    | 5/36 ≈ 0,139 — índice NO construido (no discrimina)
Selectividad Persona.orcid     | 9/9 = 1,0
Selectividad Tema.nombre       | 8/9 ≈ 0,889 — duplicado residual: los temas 26 y 61 comparten «memoria de agentes»
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
El índice por AÑO casa con la consulta GLOBAL reescrita (su filtro es la clave del índice), pero no con el AS OF persona-céntrico: para 2023 su rango devuelve [184, 185] y ninguna de las vigentes del proyecto (52, 53, 55) está en él — el índice responde por AÑO, no por persona, y el mantenimiento (10) se cobra igual.
El índice por LABEL sí casa: la pregunta del cap-43 es por TIPO (WORKED_ON, MEMBER_OF) y esa es su clave — 28 → 22 lecturas. Su construcción (158 lecturas) solo se amortiza si la pregunta se REPITE: 158 + 22n < 28n desde n = 27.
```

La tabla de ledgers —la figura recurrente del capítulo—, en forma de tabla:

| Consulta | Sin índice | Con índice | Ahorro |
|---|---|---|---|
| Global «¿afiliaciones desde 2024?» (`IndiceDesdeAnio`) | 12 | 3 | ×4 |
| AS OF persona-céntrica 2023 (`IndiceDesdeAnio`) | 28 | 28 (+10 de mantenimiento = 38) | NINGUNO — no casa |
| AS OF persona-céntrica 2026 (`IndicePorLabel`) | 28 | 22 | 6 (amortiza desde n = 27) |

Cuatro lecturas obligatorias. **Primera: la subsunción, con nombres.** `el_esquema_subsume_la_cadena_de_validadores_sobre_fixture_corrupto` corrompe la CITES 16 (extremo Tema), el Documento 21 (sin `titulo`) y la MEMBER_OF 52 (sin `desde_anio`): la cadena 41-43 denuncia `{16, 21, 52}` y el esquema denuncia EXACTAMENTE los mismos ids — tres validadores por composición, un esquema lo declara, y los validadores ni se tocaron. **Segunda: el caso ORCID, dos reglas.** El duplicado viola Unicidad con el id del SEGUNDO (Beto id 1: «mismo valor que el nodo 0»); la persona sin orcid viola Existencia («sin propiedad 'orcid'») y NO Unicidad: unicidad ≠ existencia, y el `HashMap` interno del verificador es la prueba de que UNIQUE ≡ índice. **Tercera: los tres ledgers, sin maquillar.** 12 → 3 (el índice de año casa con la consulta global reescrita, ×4), 28 = 28 + 10 (el mismo índice NO casa con la persona-céntrica: responde por año, la pregunta es por persona — su rango para 2023 son `[184, 185]` y ninguna vigente del proyecto está ahí), 28 → 22 (el índice por label sí casa: la pregunta del cap-43 es por TIPO). La regla: **el índice debe casar con el patrón de acceso**. **Cuarta: la selectividad como juez y como detective.** `Documento.anio` ≈ 0,139 → no se construye (no discrimina, cuesta mantenimiento); `Persona.orcid` = 1,0 → UNIQUE ≡ índice perfecto; `Tema.nombre` ≈ 0,889 → **el catálogo delata el duplicado residual 26/61** — los datos se cuelan cuando nadie mira, y una cuenta de valores distintos los saca a la luz.

Y el CSV cierra el círculo: `csv_roundtrip_paso4_import_export_byte_a_byte`, `csv_esquema_roundtrip_byte_a_byte`, `csv_paso4_coincide_con_dataset_commiteado_byte_a_byte` y `csv_pasos_anteriores_intactos_tras_paso4` — los ficheros de los pasos 1-3 ni se tocan. El orcid viaja en el propio dataset, y el esquema es un fichero más:

```text
$ head -2 datasets/kb-lira/paso-4/nodes.csv
id:ID, anio:INT, nombre:STRING, nota:INT, orcid:STRING, ronda:INT, titulo:STRING, :LABEL
0,,Ana,,0000-0001-2345-0001,,,Persona

$ cat datasets/kb-lira/paso-4/esquema.csv
Extremos,AUTHORED,Persona,Documento
Extremos,MENTIONS,Documento,Persona;Organizacion;Proyecto
...
Unicidad,Persona,orcid
SinSolape,MEMBER_OF,desde_anio,hasta_anio
IntervaloValido,MEMBER_OF,desde_anio,hasta_anio,2026
```

El contrato de KB-Lira, en 16 líneas de CSV — legible, serializable, discutible.

## 44.9 Qué hemos sacrificado

1. **Sin validación al importar (write-time).** `verificar_esquema` es una capa aparte que se ejecuta bajo demanda sobre el store; `GraphStore` sigue siendo intocable. El esquema que valida en CADA escritura —el patrón industrial, write-time— es el cap. 45 (ingesta): aquí la puerta se abre cuando alguien la llama, no cuando llega el dato.
2. **Sin SHACL ni shapes.** El esquema de este capítulo es el hermano imperativo/declarativo del mundo LPG; en el mundo RDF la misma idea se llama **SHACL** y se declara como shapes (caps. 46-47). Se nombra como gancho, no se explica.
3. **Sin índice de intervalos real.** El índice simple poda por un lado; el `[desde, hasta)` cobra el otro — la vencida 53 sigue costando 1 lectura incluso con índice por label. El «interval tree» queda como estructura mental: la poda por AMBOS lados es deuda declarada, gancho al cap. 53.
4. **Sin integración con el optimizador del cap-21.** El catálogo se reutiliza como ORÁCULO de decisión (selectividad, `EqIndexEntry`) pero el coste en el plan de consulta sigue siendo del cap-21; los índices de aquí no alimentan ningún planificador.
5. **Los índices viven en RAM.** El `IndiceDesdeAnio`/`IndicePorLabel` son lógicos, de modelo; sus hermanos persistentes —`HashIndex`/`BPlusTree` del cap-15, con claves u64, páginas del BufferPool y `range_scan`— se nombran, no se reutilizan: montar un pager para un store en memoria no enseña nada que este capítulo necesite.
6. **El write-time es del cap-45.** El esquema se verifica cuando se pide; la escritura sigue aceptando duplicados. La frontera, declarada antes de codificar.

## 44.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j 5.x** ofrece constraints de **unicidad** (`IS UNIQUE`), **existencia** (`IS NOT NULL`), **key constraints** y —desde Neo4j 5— **property type constraints** (blog oficial «Enforcing data quality in Neo4j 5: Property constraints», 3 de noviembre de 2023), con índices range/text/point/fulltext; y lo documenta explícitamente: los constraints de unicidad están **respaldados por índices** — el UNIQUE ≡ índice industrial, la lección estructural del `HashMap` interno de tu verificador, con validación en el write. **GQL (ISO/IEC 39075:2024, publicado en abril de 2024)** es el primer lenguaje de consulta de grafos estándar ISO, y su Parte 1 incluye el **DDL de esquema**: `CREATE PROPERTY GRAPH`, graph types con node/edge types y property value types (`NOT NULL`), y constraints `CONSTRAINT … FOR … REQUIRE … IS KEY / IS UNIQUE / IS NOT NULL` más `CREATE INDEX`. El puente con lo tuyo es directo: tu `Unicidad{label: Persona, propiedad: orcid}` es, en forma industrial, `CONSTRAINT … FOR (n:Person) REQUIRE n.orcid IS UNIQUE` — sin afirmar aquí el subclausulado fino de la norma, que no hemos verificado (frontera declarada, espejo del cap-43). Y **SQL:2011/SQL:2023** quedan como la referencia clásica de `UNIQUE`/`NOT NULL`, ya conocida — citada sin detalle fino.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (44+21): PREDICE por escrito, ANTES de correr nada, (a) los ledgers de `afiliaciones_vigentes_en_con_indice_por_label` (AS OF 2026) y de `afiliaciones_globales_desde_anio_con_indice` (2024) — cuántas lecturas y por qué — y (b) decide con la selectividad del cap-21 si un índice sobre `anio` de Documento merece la pena (5/36 ≈ 0,139 → NO) y por qué el MISMO razonamiento SÍ justifica el índice de unicidad del orcid (9/9 = 1,0). Pista: el que no discrimina devuelve casi todo el label; el que discrimina devuelve un nodo.
- *Intermedio* (44+15+14, 44+43): compara `IndiceDesdeAnio` (lógico, BTreeMap en RAM) con el `BPlusTree` del cap-15 (físico, sobre BufferPool, claves u64, un valor por clave, `range_scan`) y con el CSR del cap-14: escribe qué es lo mismo y qué es distinto (persistencia, claves, granularidad). Y aplica el esquema a `WORKED_ON`: ¿qué reglas de existencia/tipo le declararías —¿`desde_anio` opcional Int?— y por qué?
- *Experto* (44+43+28): usa `WalTransaccion` REAL del cap-28 para demostrar que la escritura de la `MEMBER_OF` 190 solapada se puede RECHAZAR antes del commit — el esqueleto de «constraint en el motor» (validación en la transacción, con `verificar_esquema` como puerta dentro de la transacción) — y declara la frontera que separa esto de la ingesta del cap-45.

## 44.11 Lo que te llevas

- **El «schemaless» miente por omisión.** El esquema siempre está en algún sitio (Schwartz 2015): en los validadores del proyecto, en el código de la app o en los datos rotos. KB-Lira tenía esquema desde el cap-41; este capítulo lo hizo DATO.
- **Constraint ≠ índice.** El constraint decide QUÉ puede existir (violación con id); el índice decide CUÁNTO cuesta encontrarlo (la respuesta no cambia, solo el ledger). Y **UNIQUE ≡ índice**: la unicidad sin índice ES un escaneo que construye el índice sobre la marcha — el `HashMap` interno lo demuestra.
- **Unicidad ≠ existencia.** Un orcid duplicado es un duplicado (id del segundo); una persona sin orcid es una ausencia — dos reglas distintas sobre la misma propiedad.
- **El esquema subsume a los validadores** sin tocarlos: sobre el mismo fixture corrupto, la cadena 41-43 y el esquema denuncian los mismos ids `{16, 21, 52}`. La regla que faltaba se añade al catálogo (nació `IntervaloValido`), nunca al verificador.
- **El índice debe casar con el patrón de acceso.** La clave del índice es el eje de la pregunta: por año gana donde la pregunta es global (12→3); por label gana donde la pregunta es por tipo (28→22); el que no casa (28=28) solo cobra mantenimiento.
- **El intervalo cobra el otro lado.** La vencida 53 sigue costando 1 lectura incluso con índice por label: el `[desde, hasta)` se paga por candidata — el índice de intervalos real es deuda al cap-53.
- **El catálogo es juez y detective.** Decidió no construir el índice de `anio` (0,139) y delató el duplicado Tema 26/61 (0,889): la selectividad no solo evita gastar, descubre datos.

## 44.12 Ojo, cuidado con…

- **Confundir constraint con índice.** El UNIQUE es la garantía; el índice la hace barata. Un constraint puede existir sin índice; un índice sin constraint solo acelera lo que ya está permitido.
- **Creer que añadir un índice abarata cualquier consulta.** El de año NO abarata la persona-céntrica: 28 = 28, y su construcción (10 lecturas) se cobra igual — 38. Un índice no es magia: es una apuesta por un patrón de acceso.
- **Declarar unicidad donde hay homónimos legítimos.** «Una persona, una organización» es FALSO en KB-Lira (Beto tiene dos); lo prohibido es el SOLAPE de intervalos, y `[2018,2024)` con `[2024,∞)` son legales — el medio abierto se respeta en TODA la implementación.
- **Pedirle al validador lo que ya no es suyo.** La puerta canónica del paso-4 es `verificar_esquema`, no un «validador paso-4» imperativo que duplicaría las reglas en código — el antipatrón que este capítulo corrige.
- **«Arreglar» la P4 atemporal.** Sigue devolviendo Beto→Neurónica por contrato; la diferencia con la P4 CON tiempo (Beto→GrafoLuna) es el cap-43, y el orcid no filtra nada: si la regresión fallara, el error está en el cambio, no en la pregunta.
- **Ignorar al catálogo cuando dice 0,139.** Construir un índice que no discrimina es pagar mantenimiento por decoración — el cap-21 ya lo sabía; ahora tú también.

## 44.13 Pin de batalla

> *«No existe base de datos sin esquema: existen esquemas que nadie declaró. Saca la regla del `if` y hazla dato — y recuerda que el índice es un atajo, no un milagro: si no casa con la pregunta, solo cobra mantenimiento.»*

## 44.14 Si solo lees 30 segundos

KB-Lira paso-4: el esquema como DATO (`Esquema` = `Vec<ReglaConstraint>` con 6 variantes: Extremos —con `hasta_labels` para el `MENTIONS` polimórfico—, Existencia, Tipo, Unicidad, SinSolape, IntervaloValido), 16 reglas declaradas en `esquema_kb_lira()`, verificadas por `verificar_esquema` (reutiliza `Violacion` del cap-41). `kb_lira_paso4()` = paso-3 + `orcid` único a las 9 personas (68 nodos, 158 aristas intactos). Lecciones medidas: unicidad ≠ existencia (duplicado → «repetida (mismo valor que el nodo 0; Unicidad)» con el id del SEGUNDO; sin orcid → «exigida por Existencia») y **UNIQUE ≡ índice** (HashMap interno). `SinSolape`: la arista 190 `[2022,2023)` solapa con la 53 `[2018,2024)` → violación; `[2024,+∞)` contigua es legal. Subsunción: fixture corrupto (CITES con extremo Tema, Documento sin `titulo`, MEMBER_OF sin `desde_anio`) → validadores 41-43 y esquema denuncian los mismos ids `{16, 21, 52}`. Índices: `IndiceDesdeAnio` (10 lecturas) casa con la consulta global reescrita (12 → 3) pero NO con el AS OF persona-céntrico (28 = 28 + 10 = 38); `IndicePorLabel` (el CSR lógico, 158 lecturas) sí: 28 → 22, amortizable desde n = 27 (`158 + 22n < 28n`). La regla: el índice debe casar con el patrón de acceso. Selectividad como juez (cap-21): `Documento.anio` 5/36 ≈ 0,139 → NO construir; `Persona.orcid` 9/9 = 1,0; `Tema.nombre` 8/9 ≈ 0,889 → **delata el duplicado Tema 26/61 «memoria de agentes»**. Regresión cuádruple verde; CSV paso-4 + `esquema.csv` (16 líneas) commiteados; **993 tests ALL_GREEN**, sin bench. Fronteras: write-time al importar (cap-45), SHACL (46-47), índice de intervalos real (53).

## 44.15 Una historia pequeña

El refactor A del cap-42 creó el subtema «memoria de agentes» (nodo 61) como especialización de «Grafos de conocimiento para agentes». Lo que nadie notó es que el paso-1 ya tenía un tema «memoria de agentes» (nodo 26): el subtema duplicaba el nombre del tema padre. Los validadores 41-43 no lo vieron —nunca declararon `Tema.nombre` único, y la Existencia solo exige que el nombre ESTÉ, no que sea distinto—, y el grafo siguió creciendo con dos temas que eran el mismo para cualquier humano que leyera. Un día, el equipo quiere decidir si un índice sobre `Tema.nombre` merece la pena y el catálogo del cap-21 —que solo iba a contar valores distintos— responde: 9 temas, **8 nombres**. La selectividad delata el duplicado: los datos se cuelan cuando nadie mira, y a veces quien los saca a la luz no es un validador, sino una simple cuenta. El catálogo no solo decidió no construir un índice: descubrió un dato que ningún test había cazado.

## Ejercicios resueltos

**1. Retrieval sin pistas: las 6 reglas del esquema, DE MEMORIA, y clasifica 5 afirmaciones.** Cierra el libro y recita las 6 variantes de `ReglaConstraint`: **Extremos** (una arista `rel_tipo` va de `desde_label` a CUALQUIERA de `hasta_labels`), **Existencia** (todo nodo `label` lleva `propiedad`), **Tipo** (si lleva la propiedad, es del tipo esperado), **Unicidad** (entre los nodos `label`, la propiedad es única), **SinSolape** (intervalos `[desde, hasta)` disjuntos por par de extremos) e **IntervaloValido** (la cota inferior requerida, `desde ≤ anio_max`, `hasta ≥ desde`). Ahora clasifica: (a) «el orcid de Beto se repite» → **constraint** (¿puede existir? → no). (b) «el AS OF baja de 28 a 22 lecturas» → **índice** (¿cuánto cuesta encontrarlo?). (c) «la MEMBER_OF 190 solapa con la 53» → **constraint**. (d) «el rango del BTreeMap devuelve `[184, 185]` sin leer nada» → **índice**. (e) «el Documento 21 no tiene `titulo`» → **constraint**. Si clasificaste (b) o (d) como constraint, vuelve al §44.3: la distinción ES el capítulo.

**2. Explica por qué UNIQUE ≡ índice, con el `HashMap` interno.** Mecánica: `verificar_esquema` implementa `Unicidad` con un `HashMap<clave, primer_id>`: por cada nodo con la propiedad se intenta insertar; si la clave ya está ocupada, el nodo actual es el SEGUNDO y la violación lleva SU id («mismo valor que el nodo 0»). Sin un índice físico por debajo, la verificación ES la construcción del índice sobre la marcha: cada ejecución barre y reconstruye. En las BBDD reales el UNIQUE crea y mantiene ese índice de una vez (Neo4j lo documenta); aquí el `HashMap` es el índice efímero que demuestra la equivalencia: la unicidad no es una propiedad del dato, es una estructura que lo comprueba.

**3. Responde por qué el índice de año da 28 = 28 y el de label da 28 → 22.** Mecánica: `IndiceDesdeAnio` responde por AÑO (`desde_anio ≥ anio`); el AS OF de Kira pregunta por PERSONA — su rango para 2023 devuelve `[184, 185]` y las vigentes del proyecto (52, 53, 55, todas con `desde < 2023`) ni están en él: ni una lectura se sustituye, y la construcción (10) se cobra igual: 28 + 10 = 38. `IndicePorLabel` responde por TIPO, que ES el eje de la pregunta: evita las 2 MENTIONS del `in_edges` y las 12 aristas salientes ajenas; el ledger real en 2026: 6 WORKED_ON + 3 personas + 10 MEMBER_OF (la vencida 53 incluida: el intervalo cobra el otro lado) + 3 organizaciones = **22**. Amortización: la construcción (158) se reparte entre las n repeticiones — `158 + 22n < 28n` desde n = 27.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 44+21).** Desarrolla el reto esencial del §44.10: PREDICE por escrito los ledgers de las dos consultas con índice (AS OF 2026 con `IndicePorLabel`: 22; global 2024 con `IndiceDesdeAnio`: 3) ANTES de correr `informe_esquema_reproducible`, y decide con la selectividad si el índice de `Documento.anio` merece existir (≈0,139 → NO) y por qué el del orcid sí (1,0). Criterio: predicción escrita primero; si tu predicción de la global no fue 3, revisa qué lee el barrido (10 `get_edge` + 2 `get_node`) frente al rango gratis del BTreeMap.

**Intermedio (predecir y comparar; 44+15+14, 44+43).** (a) Compara `IndiceDesdeAnio` (lógico, BTreeMap en RAM, clave i64, varias aristas por clave) con el `BPlusTree` del cap-15 (físico, sobre BufferPool, claves u64→u64, un valor por clave, `range_scan`) y con el CSR del cap-14: escribe qué es lo mismo (la partición por clave que evita el barrido) y qué es distinto (persistencia, cardinalidad de clave, granularidad). (b) Aplica el esquema a `WORKED_ON`: declara por escrito las reglas de existencia/tipo que añadirías —¿`desde_anio` opcional Int?— y justifica cada una con la escalera R1-R7. Criterio: cada decisión con su porqué y su coste.

**Experto (crear y demostrar; 44+43+28).** Usa `WalTransaccion` REAL del cap-28 para demostrar en un test que la `MEMBER_OF` 190 solapada se RECHAZA antes del commit: abre la transacción, intenta el `put_edge`, ejecuta `verificar_esquema` como puerta DENTRO de la transacción, y haz `rollback` con la violación en la mano. Restricciones: std puro, sin tocar cap-28 ni el trait `GraphStore`, suite ALL_GREEN con tu test dentro. Criterio: el test debe declarar la frontera con el cap-45 — esto valida en la transacción del motor; el cap-45 valida al importar el lote.

## Para profundizar

- **Baron Schwartz, «Schemaless Databases Don't Exist», blog de SolarWinds, 24 de febrero de 2015** — la anécdota del §44.0 y su tesis: "there is always a schema somewhere. Usually in multiple places".
- **ISO/IEC 39075:2024 (GQL), publicado en abril de 2024 (iso.org/standard/76120)** — DDL de esquema en la Parte 1: `CREATE PROPERTY GRAPH`, graph types con node/edge types y property value types (`NOT NULL`), `CONSTRAINT … FOR … REQUIRE … IS KEY / IS UNIQUE / IS NOT NULL` y `CREATE INDEX`; el subclausulado fino no está verificado aquí (frontera declarada del contrato).
- **Neo4j Cypher Manual 5.x + blog «Enforcing data quality in Neo4j 5: Property constraints», 3 de noviembre de 2023** — constraints de unicidad (`IS UNIQUE`), existencia (`IS NOT NULL`), key constraints y property type constraints, respaldados por índices, con validación en el write.
- **SQL:2011/SQL:2023** — la referencia clásica de constraints `UNIQUE`/`NOT NULL`, citada sin detalle fino.
- Dentro del libro: cap. 41 (validadores, la `Violacion`, las 10 preguntas, la decisión #7 de MENTIONS polimórfico), cap. 42 (los antipatrones, el refactor A, el subtema 61), cap. 43 (valid-time, AS OF a 28 lecturas, la siembra del paso-3, la convención `[desde, hasta)`), cap. 21 (catálogo, `selectivity`, `EqIndexEntry` — el índice modelado que aquí se construye), cap. 15 (`BPlusTree`/`HashIndex` físicos, `range_scan`), cap. 14 (CSR), cap. 32 (CSV round-trip), cap. 28 (WAL para el reto experto), caps. 45 (ingesta: write-time), 46-47 (RDF y SHACL), 53 (índice de intervalos como memoria temporal del agente).

## Mini-diálogo: en guardia nocturna

> — Son las dos de la madrugada. El equipo ha construido el índice sobre `desde_anio` y esperaba que el AS OF volara.
>
> — ¿Y?
>
> — 28 = 28. La consulta de Kira tarda exactamente lo mismo con y sin índice. Nadie entiende nada, y el informe dice que además pagamos 10 lecturas de construcción.
>
> — (pausa) ¿Y la consulta GLOBAL? «¿quién se afilió desde 2024?»
>
> — Esa bajó de 12 a 3. Pero a nadie le importa esa consulta, se quejan de la de Kira.
>
> — Claro: el índice responde por AÑO, y la pregunta de Kira es por PERSONA. Su rango para 2023 devuelve `[184, 185]` — ni una de las vigentes del proyecto está ahí. El índice no estaba roto: estaba casado con la consulta equivocada.
>
> — O sea que el índice no es un milagro.
>
> — Es un atajo, y solo sirve si va en la dirección de la pregunta. Para la de Kira hace falta el de por etiqueta: 28 → 22, la vencida 53 incluida — la historia se paga, pero ya no paga lo que no es historia. Y si el índice no se usa, solo cobra mantenimiento. Buenas noches.

> *Siguiente parada, cap. 45 (ingesta): el esquema existe y se verifica bajo demanda — pero el lote de mañana puede traer un duplicado y nadie lo ataja al importar. ¿Quién lo aplica al vuelo, en cada escritura? El write-time es el siguiente capítulo. Abiertas: SHACL y los shapes (caps. 46-47, el hermano declarativo del mundo RDF) y el índice de intervalos real como memoria temporal del agente (cap. 53).*
# Capítulo 45 — Workflows de ingesta: de datos crudos al grafo

> *«QUINTO capítulo del Volumen III y CIERRE DE LA PARTE I, "Modelar datos de grafos". Si vienes del cap. 44 —o del perfil datos/IA que lo haya leído en diagonal— la escalera R1-R7, los antipatrones pagados, el transaction-time, el esquema declarativo con sus 16 reglas y la regresión como red de seguridad son tus HERRAMIENTAS, no tu contenido: el modelo ya es sano, y ahora se le enseña a NACER desde la basura. El cierre del cap-44 te dejó clavadas tres cosas: el write-time que nadie aplica —"el esquema valida bajo demanda, ¿y si cada importación lo aplicara al vuelo?": el lote de mañana puede traer un duplicado y nadie lo ataja al importar—, la deuda del cap-43 del transaction-time que una carga debe alimentar sola, y el duplicado Tema 26/61 que el catálogo delató —`Tema.nombre` 8/9 ≈ 0,889— y que nadie curó. Este capítulo responde CON DATOS: un pipeline de cuatro etapas que valida cada lote AL VUELO con número de línea, escribe su propio `HistorialIngesta` (29 eventos, ts 1..29, la forma del cap-43 replicada) y CURA el fantasma: el paso-5 termina con 67 nodos, 158 aristas y el esquema del cap-44 en `Ok`. Delatar no es curar: la ingesta es quien cura.»*

## 45.0 La anécdota de la esquina

«Garbage in, garbage out» — basura entra, basura sale. Las siglas **GIGO** son de esas frases que todo el mundo usa y casi nadie sabe de dónde vienen. La primera aparición documentada en prensa es del **10 de noviembre de 1957**: una pieza sindicada de *The Hammond Times* sobre los matemáticos del Ejército de EE.UU. que manejaban los primeros cerebros electrónicos —BIZMAC, UNIVAC, «GARBAGE IN-GARBAGE OUT» formaban parte del vocabulario diario de los programadores militares—. El especialista William D. Mellin, «programador de computadoras electrónicas», contaba que un problema de datos de radiofrecuencia que «a la vieja usanza habría llevado a 50 chicas con calculadoras manuales toda una semana», un programador lo resolvía en horas —y explicaba el precio: «si el problema ha sido programado descuidadamente, la respuesta será igual de incorrecta… la máquina no puede corregirlo porque no puede hacer una cosa: pensar por sí misma». La paternidad de la frase se atribuye —sin fecha firme— a George Fuechsel, instructor de programación de los primeros días de IBM. La atribución es la de siempre: nadie tiene el acta de la acuñación, pero la frase sobrevivió porque describe una verdad incómoda: **la basura no se detecta sola**.

Piensa en la cinta de entrada de aquella máquina: tarjetas perforadas que nadie revisaba antes de alimentar el lector. Tu KB-Lira lleva cuatro capítulos siendo el taller del cap-44: el 41 modeló, el 42 pagó los antipatrones, el 43 añadió el tiempo y el 44 declaró el contrato. Pero todos esos grafos NACÍAN de código: los builders de cada capítulo sembraban nodos y aristas a mano, con la suciedad justa que cada capítulo quería mostrar. En el mundo real los grafos no nacen de builders: nacen de ficheros: CSVs exportados de una hoja de cálculo, JSONL de una API, la descarga de un legado.

La pregunta que ordena este capítulo: **tus datos crudos tienen 5 Anas —«Ana», «ana garcia», «Ana G.», «ana garcía» con un orcid malformado y «Ana García (Universidad de Lira)»—, un título con un typo de una letra y un tema duplicado que nadie sabe que está duplicado. ¿Qué pasa cuando el grafo nace de un archivo, no de un builder?** El cap-44 te dejó con el cajón del taller donde «todo cabe»; este capítulo te entrega la cinta de entrada: la basura ya no se detecta sola —la detecta el pipeline, o no se detecta nunca.

## 45.1 Objetivo

Objetivo medible del outline: **construir un pipeline reproducible de carga (CSV/JSONL) con validación y deduplicación**. Al terminar tendrás:

1. **El pipeline por etapas** `cargar_paso5()` → `(MemoryStore, HistorialIngesta, InformeIngesta)`: 221 filas crudas → **67 nodos / 158 aristas**, con ids NUEVOS (la identidad la da la clave natural, cap-41) y el esquema del cap-44 en `Ok`.
2. **La validación como write-time en dos niveles**: reglas de fila por lote con rechazo selectivo `(línea, motivo)` + `verificar_esquema` como puerta final.
3. **La frontera de lote (batch)**: una pasada por fichero, lotes de 25, contadores — el **streaming** del cap-32 con frontera explícita de memoria.
4. **La idempotencia (idempotency) por clave natural (natural key)**: recargar el mismo CSV dos veces produce el mismo grafo — el test más barato del capítulo, y pasa siempre.
5. **El entity resolution a mano** (std puro, «primero a mano»): normalizar → bloquear → comparar → grafo de similitud → componentes conexas del cap-25 → fusionar. Con contadores: **91 pares todo-contra-todo → 11 comparadas (80 evitadas)**.
6. **La fusión con sobrescritura cuidada**: canónico (canonical record) con más datos, conflictos DECLARADOS, aristas reapuntadas — y el `HistorialIngesta`, la deuda del cap-43 cobrada.
7. **El caso Tema 26/61 CURADO**: 68 → 67 nodos, `Tema.nombre` 8/8 — el arco del cap-44 cerrado.
8. **El workspace en 1017 tests ALL_GREEN** (993 + 24), sin bench.

## 45.2 Problema

Mira lo que tienes. El cap-32 (Vol.II) construyó `importar_csv_nodos`, un importador que lee un CSV con cabecera `:ID`/`:LABEL`, línea a línea, con autocommit por fila. El cap-44 declaró el esquema y demostró que nadie lo aplica al importar. Y el grafo del paso-4 arrastra un fantasma: los temas 26 y 61 comparten «memoria de agentes», el catálogo del cap-21 lo delató —8/9 ≈ 0,889— y nadie lo arregló. El modelo es sano, sus reglas son datos… y el lote de mañana puede traer cualquier cosa.

Antes de dibujar nada, desactivemos las **siete** ideas equivocadas que suelen venir con el tema:

1. **«La ingesta es copiar el CSV al grafo: `importar_csv_nodos` del cap-32 ya lo hace.»** No: el cap-32 importa ficheros LIMPIOS con `:ID` y autocommitea cada fila; un fichero crudo —sin ids, con duplicados y suciedad— lo hace fallar en la cabecera o en `DuplicateNode`. Ingesta es un PIPELINE de cuatro etapas con reglas y contadores, no un bucle de copia.
2. **«El esquema del cap-44 era decorativo: se verifica bajo demanda y ya está.»** No: el gancho literal del cap-44 es que nadie valida AL IMPORTAR. Este capítulo aplica las mismas reglas como write-time —por lote y como puerta final—.
3. **«Deduplicar es quitar filas repetidas.»** No: eso es la mitad; la otra mitad es entity resolution: «Ana G.», «ana garcia» y «Ana García (Universidad de Lira)» son la MISMA persona y hay que FUSIONARLA (y reapuntar sus aristas), no borrar la fila.
4. **«Para encontrar duplicados hay que comparar todo contra todo.»** No: O(n²) no escala. El blocking (blocking) agrupa por clave de bloque y solo compara dentro del bloque —con contadores que lo demuestran: 91 comparaciones naive frente a 11 bloqueadas.
5. **«Al fusionar, la última fila gana (o la primera, da igual).»** No: la fusión tiene regla explícita —canónico con más datos, empate por menor línea— y la sobrescritura es CUIDADA: todo conflicto se declara en el informe, nada se sobrescribe en silencio.
6. **«Cargar dos veces el mismo fichero es un error del operador.»** No: es el test más barato de un pipeline sano. La idempotencia por clave natural es una PROPIEDAD de diseño —la «prueba de la identidad propia» del cap-41 aplicada a la ingesta: el pipeline asigna ids NUEVOS, la identidad la da la clave natural.
7. **«El duplicado Tema 26/61 del cap-44 es un caso cerrado: el catálogo lo delató.»** No: **delatar no es curar**; nadie lo arregló y el grafo del paso-4 lo sigue llevando. La ingesta con entity resolution lo FUSIONA: el arco del cap-44 se cierra aquí.

El crudo del paso-5 no es cualquier suciedad: está DISEÑADA para que cada pieza del pipeline tenga algo que demostrar. Los 4 ficheros sin ids contienen 3 duplicados exactos (un «Beto», el título «Recuperación aumentada con grafos» y una MEMBER_OF «Ana → Universidad de Lira»), 4 variantes de Ana —«Ana», «ana garcia», «Ana G.», «ana garcía» con el orcid malformado `0000-0001-2345-1` y «Ana García (Universidad de Lira)»—, 1 casi-duplicado con un typo de una letra en el título, el fantasma «memoria de agentes» (los temas 26/61 que el paso-4 arrastra) y una MEMBER_OF solapada `[2020,2023)` contra la `[2018,2024)` de Beto. **221 filas** en total. Es la suciedad del contrato: cada fila sucia termina, o rechazada con su número de línea, o fusionada con su conflicto declarado —ninguna se cuela—.

Y el compromiso de honestidad que rige TODO el capítulo —espejo de los caps. 43 y 44—: los números salen de `cargo test`, nunca de la pizarra. El contrato predecía 78 comparaciones para 13 personas; la medida real son **91 para 14** —el crudo tiene una Ana más—, y el delta se pine con su porqué, prohibido maquillar contadores.

## 45.3 Modelo mental: la fábrica

La ingesta es una **FÁBRICA con cuatro estaciones** (stages): extraer, validar, mapear y cargar. El esquema del cap-44 es el control de calidad; el entity resolution es el control de duplicados que fusiona ANTES de que la basura llegue al grafo. Cada estación devuelve su TIPO, nunca el vacío. El panel que ordena todo el capítulo:

```text
LA FÁBRICA (el pipeline por etapas — cada etapa devuelve su TIPO, nunca el vacío):
  CRUDOS                VALIDAR               MAPEAR                CARGAR
  (sin ids, sucio)      (esquema como         (columnas crudas →   (put_node/put_edge,
                        control de calidad)   modelo; nombres →    ids NUEVOS)
                                               entidades)
  personas.csv ──►      reglas de fila ──►    MAPEO_COLUMNAS ──►   67 nodos / 158 aristas
  documentos.csv        por LOTE (línea,      nombre_autor →       + HistorialIngesta
  temas.csv             motivo)               titulo_obra →        (append-only, ts 1..29)
  relaciones.csv        │                     resolución de
                        ▼                     nombres (clave
  verificar_esquema (cap-44) como PUERTA FINAL: Ok o violaciones con ids
  │  ── por lote: la fila sucia se RECHAZA y el pipeline sigue ──
  └─ ENTITY RESOLUTION dentro de MAPEAR:
       normalizar (sin tildes) → bloquear por inicial → comparar (Jaccard bigramas)
       → grafo de similitud (aristas SIMILAR) → componentes conexas (cap-25) = CLUSTERS
       → fusión: canónico con más datos, conflictos DECLARADOS, aristas REAPUNTADAS

EL CASO QUE CIERRA EL ARCO DEL CAP-44 (con contadores):
  «memoria de agentes» ×2 (temas 26 y 61) → mismo bloque, jaccard 1.0 → cluster de 2
  → fusión → 68 → 67 nodos · 158 aristas intactas · Tema.nombre 8/8 = 1.0
  → el catálogo delató, la ingesta CURÓ — y verificar_esquema aprueba el resultado

LA LÍNEA DE BETO (la SinSolape del cap-44 ejecutada AL VUELO, [desde, hasta)):
  2018─────────────────────────────2024──────────────2026
  [─ Neurónica (la 53, ya aceptada) ────)
  [─ Neurónica [2020,2023) ─)      ← solapa con la 53: RECHAZADA (línea 57)
  [─ GrafoLuna [2024,∞) ─)         ← contigua: LEGAL — y P4 se enriquece con ella
```

La REGLA DE ORO heredada (determinismo total, cap. 34): el paso-5 es un artefacto DERIVADO del paso-4 —la basura deliberada se genera de forma determinista, los ids los decide el pipeline por orden de carga y cada contador está pineado—. Las 10 preguntas del cap-41, los pines del cap-42, el validador del paso-3 y el esquema del cap-44 son la red de seguridad: si la ingesta cambia una respuesta vieja sobre los subgrafos 1-3, el cambio está MAL.

La moneda (nunca µs): filas leídas · lotes formados · rechazos con línea · comparaciones naive vs bloqueadas · clusters · fusiones · conflictos declarados · nodos y aristas exactos. El informe del §45.8 es la factura de la fábrica: **221 filas → 11 lotes → 4 rechazos → 6 fusiones (6 conflictos) → 67 nodos y 158 aristas** —cada número sale de `cargo test`, nunca de la pizarra—. El momento ¡ajá! perseguido: **«el grafo de similitud es un grafo: las mismas componentes conexas del cap-25 que agrupaban comunidades en redes, ahora agrupan DUPLICADOS — y "cargar dos veces el mismo fichero" no es un susto: es el test más barato del pipeline, y pasa siempre.»** Esta figura ordena el capítulo entero: cada pieza del §45.6 es una estación de la fábrica o uno de sus controles.

## 45.4 Primera solución

La primera solución —y la que aplica casi todo el mundo— es **coger el importador que ya existe** y lanzarlo contra los crudos. El cap-32 construyó `importar_csv_nodos` (cap32_import_export.rs), que ya era streaming: lee línea a línea, autocommitea por fila y exige la cabecera `:ID`/`:LABEL`. Tres modos de fallo ANTES de la solución:

**(a) Sin `:ID`, la cabecera falla.** El formato del cap-32 exige ids en el propio fichero. Los crudos del paso-5 no los tienen —y no es un descuido: es el contenido del capítulo. La identidad de una persona no es un número que el fichero DECLARE; es una decisión del modelo (cap-41). `CabeceraInvalida`, primer intento muerto.

**(b) «Arreglado» añadiendo ids a mano: los duplicados entran igual.** Supón que asignas ids a las 14 personas y a los 38 documentos. El importer autocommitea cada fila sin mirar nada más: el segundo «Beto» entra como otra persona, el duplicado exacto de «Recuperación aumentada con grafos» entra como otro paper, y «memoria de agentes» aparece dos veces como tema. El grafo queda con dos Anas —bueno, con cinco, contando las variantes— y nadie se entera. El importer no pregunta.

**(c) La fila solapada entra, y el esquema la denuncia TARDE.** La MEMBER_OF de Beto `[2020,2023)` se carga sin problema. Después, alguien ejecuta `verificar_esquema` y el cap-44 hace su trabajo: la `SinSolape` denuncia la violación… cuando la basura ya está dentro. Validar al final no es no validar: es validar cuando ya no se puede hacer nada barato.

Tres modos de fallo, tres lecciones: la cabecera demuestra que la identidad no viene en el fichero (cap-41); el autocommit demuestra que sin reglas el duplicado entra callando (cap-44: «nadie valida al importar»); y la denuncia tardía demuestra que validar el grafo completo después de cargar no sustituye a validar la fila antes de insertarla. El cap-32 no estaba roto: estaba construido para otro trabajo —ficheros limpios con `:ID`—, y el trabajo de este capítulo es más grande.

## 45.5 Sus límites

El importer del cap-32 —que es bueno en lo suyo— tiene cuatro límites que lo delatan como herramienta de ingesta:

1. **No conoce el esquema.** `importar_csv_nodos` valida cabecera y tipos básicos; de las 16 reglas del cap-44 no sabe nada: la solapada entra, un orcid duplicado entraría, y la puerta se abre cuando alguien la llama —nunca cuando llega el dato—. El cap-44 lo dejó escrito: «`verificar_esquema` es una capa aparte que se ejecuta bajo demanda… el esquema que valida en CADA escritura —el patrón industrial, write-time— es el cap. 45 (ingesta)». El gancho literal, sin respuesta hasta este capítulo.
2. **No distingue entidades.** Sin clave natural, dos Anas son indistinguibles: «Ana» y «ana garcia» son DOS nodos que ningún test sabría unir. El cap-41 lo dijo: la identidad es decisión del modelo, no del fichero —pero sin la clave, ni el modelo ni el fichero tienen la última palabra—.
3. **No agrupa duplicados.** Encontrar «los que se parecen» comparando todo contra todo es O(n²): con 14 personas son 91 pares; con 14.000 son ~98 millones. El blocking no es una optimización: es la diferencia entre posible e imposible.
4. **No recuerda.** Recargar el mismo fichero duplica el grafo y nada avisa. La promesa del cap-44 —el write-time en cada escritura— sigue sin respuesta, y el fantasma 26/61 sigue en el grafo del paso-4: el capítulo que lo delató no era el que podía curarlo.

Cuatro límites, una conclusión: el importer del cap-32 no necesita arreglarse —necesita un NUEVO VECINO que sí sepa de esquema, de entidades, de duplicados y de memoria. Ese vecino es el pipeline de la siguiente sección.

## 45.6 Solución evolucionada

Nueve piezas, cada una un TRADE-OFF con precio en contadores o en garantías:

**1. El pipeline por etapas.** `cargar_paso5()` orquesta las cuatro estaciones con salidas tipadas (`DatosCrudos → registros validados → entidades/aristas mapeadas → store`). Extraer usa `DatosCrudos::desde_csv`, que REUTILIZA `partir_csv` del cap-32 tal cual (el mismo RFC 4180-lite); validar aplica las reglas de fila y la clave natural; mapear traduce columnas crudas al vocabulario del modelo con `MAPEO_COLUMNAS` como DATO (`nombre_autor → Persona.nombre`, `titulo_obra → Documento.titulo`, `anio_publicacion → Documento.anio`, `tipo → :LABEL`, `tema_nombre → Tema.nombre`); cargar escribe SOLO por la API pública de `GraphStore` (`put_node`/`put_edge`) con ids NUEVOS —el pipeline NO toca el store por dentro: patrón hexagonal del proyecto, la misma frontera limpia que los caps. 41-44—. El cap-32 importaba; este capítulo INGIERE: extrae, valida, mapea, resuelve entidades y fusiona —con contadores en cada etapa y el esquema del cap-44 como contrato de salida—.

**2. Extraer en streaming por lotes.** El cap-32 ya era streaming (verificado: `read_line` + autocommit por fila), pero sin frontera: el pipeline introduce el **lote de 25 registros** como unidad residente máxima —el espejo de `FronterasBfs`/`Presupuesto` del cap-26, aplicado a la RAM—. `datos.registros.chunks(25)`, UNA pasada por fichero, **221 filas → 11 lotes**, cada uno con su número. ¿Por qué 25 y no 1.000? Es un TRADE-OFF, no una constante sagrada: el lote grande amortiza el recorrido pero retrasa el rechazo y alarga el lote de memoria; el lote pequeño fail-fastea antes y puebla más el histórico —el número se declara en una constante (`TAMANO_LOTE: usize = 25`) y se pinea en el informe, no se dogmatiza—. La frontera habilita el fail-fast de la pieza 3: la fila 9.000 ya no rompe después de cargar 8.999. Y cada etapa escribe sus contadores en `InformeIngesta` —filas leídas, lotes, rechazos, fusiones, nodos y aristas finales—: la factura de la fábrica, sin tiempos.

**3. Validar en dos niveles (write-time).** (a) POR LOTE, reglas de FILA derivadas del esquema: label conocida, props requeridas, tipos, formato de orcid (`XXXX-XXXX-XXXX-XXXX`), clave natural repetida y `SinSolape` LOCAL entre las MEMBER_OF del lote. El registro que viola se RECHAZA con `(línea, motivo)` y el pipeline SIGUE —solo la cabecera inválida aborta (`LoteInvalido`)—. ¿Por qué no el esquema entero por lote? Porque `Extremos`, `SinSolape` e `IntervaloValido` necesitan el grafo completo: un lote aislado SIEMPRE viola extremos. (b) FINAL: `verificar_esquema` es la puerta de salida —`Ok` obligatorio; si no, error con las violaciones—. El lote rechaza, el esquema sanciona. Los 4 rechazos reales del paso-5: línea 15 (el «Beto» duplicado), línea 7 (el «Recuperación aumentada con grafos» duplicado), línea 56 (la MEMBER_OF duplicada «Ana → Universidad de Lira») y línea 57 —**la solapada `[2020,2023)` con motivo «solape con la 53»**: la `ReglaSinSolape` del cap-44 ejecutada al vuelo, ANTES de cargar, con el id que la arista legítima tendrá al insertarse—.

Y la frontera entre «rechazar» y «fusionar», que es CONTENIDO y no detalle, se declara con dos excepciones deliberadas: el duplicado EXACTO de Tema NO se rechaza por clave repetida —es el fantasma 26/61: la decisión del contrato es que el entity resolution lo CURE, no que un rechazo lo esconda—, y el orcid malformado de una Persona tampoco —debe llegar a la fusión como nodo separado para que el conflicto se DECLARE—. La validación unitaria `validar_registro` sí rechaza ambos; el pipeline decide pasarlos porque el contrato manda que el conflicto se declare, no que la fila desaparezca en silencio.

Y la división del trabajo entre los dos niveles: la regla local ataja el solape dentro del lote ANTES de cargar; la puerta final vigila el grafo COMPLETO —si un solape escapara entre lotes, `verificar_esquema` lo denunciaría como violación y la carga terminaría con las violaciones en la mano, nunca con un grafo que «no sabe»—. El lote es el radar cercano; el esquema es la aduana.

**4. Mapear: nombres → entidades.** Las aristas crudas referencian extremos por NOMBRE («Ana», «Instituto Neurónica», «Proyecto Kira»). La etapa los resuelve a ids por clave natural —y las entidades SIN fichero propio se crean desde los extremos: `label_de_extremo` decide el label según el `rel_tipo` (`MEMBER_OF → Organizacion`, `WORKED_ON → Proyecto`, `PUBLICADO_EN → Conferencia`, `REALIZA`/`SOBRE`/`CONTRARRESTA → Resena`) y `resolver_o_crear_extremo` crea el nodo con su `nombre`. El `MENTIONS` polimórfico del cap-41 (decisión #7) se resuelve aparte: Persona por clave de persona, Proyecto por el prefijo «Proyecto », Organizacion en el resto. La «prueba de la identidad propia» del cap-41 en acción: 4 Organizacion, 3 Proyecto, 3 Conferencia y 4 Resena nacen de las aristas, sin fichero propio.

**5. Entity resolution, primero a mano** (la regla de CONVENTIONS §4). La cadena completa, con las funciones reales:

```text
normalizar_nombre (minúsculas sin tildes; paréntesis CONSERVADOS)
→ bloque_por_inicial (clave de bloque: primera letra del primer token)
→ jaccard_bigramas (bigramas SIN padding; regla de arista: mismo bloque Y
  (jaccard ≥ 0,5 O mismo primer token Y jaccard ≥ 0,25))
→ construir_grafo_similitud (MemoryStore temporal, aristas SIMILAR)
→ clusters_de_similitud = componentes_conexas del cap-25 REUTILIZADA
→ fusionar_cluster (canónico + reapuntado + conflictos)
```

Los números reales: 14 personas → **91 pares todo-contra-todo → 11 comparadas por bloque inicial (80 evitadas)** —el todo-contra-todo es C(14,2) = 91; el bloque por inicial deja 10 pares de Anas en «a» y 1 par de Beto en «b»; el resto son bloques de un miembro y no comparan—. Jaccard mide lo que la distancia de Levenshtein no ve: «ana garcia»↔«ana g.» = **0,4** (entra por la regla SECUNDARIA: mismo primer token Y ≥ 0,25), «ana garcia»↔«ana garcia (universidad de lira)» = **0,3** (los paréntesis se conservan a propósito: este caso se resuelve con bigramas, no con normalización), y el contraste «carla mendez» = **0,11** —2/18, la predicción del contrato reproducida EXACTA: sin arista—. El grafo de similitud es un GRAFO: sus componentes conexas —el BFS del Vol.I caps. 3-4, el cap-25 literal— son los clusters: **5 «Ana»… + 2 «Beto»… + 7 singletons (9 clusters)**. Y un detalle de frontera: un nombre sin tokens queda FUERA de cualquier bloque —no se compara contra nadie—; la validación ya rechaza los nombres vacíos antes de que el entity resolution los vea.

Y la DECISIÓN de métrica, demostrada con datos: `distancia_levenshtein` (DP clásica sobre `char`, ~20 líneas) y `similitud_levenshtein` se implementan a mano como métrica de CONTRASTE —el clásico de 1966 que FALLA en abreviaturas: «ana garcia» vs «ana g.» mide 5 ediciones, ≈ 0,5 de similitud, fuera del alcance de un umbral razonable, mientras el bigrama con refuerzo de primer token lo resuelve con 0,4—. Y entre «ana garcia» y «carla mendez» la real mide **9** (no ≥ 10: pineado lo medido). Por eso los NOMBRES usan bigramas y los TÍTULOS usan **Levenshtein ≥ 0,9**: el typo «…aumentada con grafo»↔«…grafos» mide **0,97** (UNA edición) y el siguiente par de títulos legítimos está a **0,77** —jaccard sobrefundiría títulos largos (0,63 entre legítimos distintos)—. La métrica es una DECISIÓN, no un dogma: se elige por tipo de entidad y se calibra sobre el dataset real. Hasta los detalles del algoritmo tienen su porqué: los bigramas van SIN padding (sin `_a`/`a_` sintéticos) porque la predicción de contraste del contrato —0,11 = 2/18— se reproduce EXACTA con esa definición, y la frontera inicial que el padding sintético daría queda cubierta por el bloque por inicial y el refuerzo de primer token.

**6. Fusión con sobrescritura cuidada (merge).** `fusionar_cluster`: el canónico es el miembro con MÁS props no vacías (`props_no_vacias`; empate → menor id, la «menor línea»); las props iguales se conservan, las ausentes se rellenan —con `Null` no: un valor vacío no enriquece—, y un valor DISTINTO no se sobrescribe: se DECLARA (`ConflictoFusion` con propiedad, valor canónico y valor descartado). Si el canónico no tiene prop `nombre`, adopta el del cluster; si la tiene, la suya manda. El orcid malformado `0000-0001-2345-1` de «ana garcía» llega a la fusión como nodo separado —el pipeline NO lo rechaza, deliberadamente: rechazarlo impediría declarar el conflicto, y la fusión salva la `Unicidad` que la fila suelta habría violado—. Y el borrado es en CASCADA (cap-08): `delete_node` arrastra las aristas incidentes, así que ANTES de borrar se hace el snapshot de `out_edges` ∪ `in_edges` de todos los miembros (con dedup por id: una arista entre dos descartados aparece en ambas listas), y después se REAPUNTAN al canónico con su mismo id, rel_tipo y props —los miembros se borran todos, incluido el canónico, porque `put_node` rechaza ids repetidos y es la única forma de escribir la unión de props—. **Reapuntar no borra**: las 158 aristas del pipeline sobreviven a las 6 fusiones.

**7. El `HistorialIngesta`: el transaction-time de la carga.** La deuda del cap-43, cobrada: un histórico append-only con ts monótono —la FORMA del `HistoricoAfiliaciones` replicada, sin tocar cap-43— donde `registrar` asigna el ts y nadie más. Seis variantes de `EventoIngesta` —`CargaIniciada`, `LoteValidado`, `RegistroRechazado`, `FusionEntidad`, `ConflictoDeclarado`, `CargaCompletada`— y **29 eventos con ts 1..29**: 1 + 11 lotes + 4 rechazos + 6 fusiones + 6 conflictos + 1 cierre, con el orden del código: primero los lotes de las entidades, luego las fusiones y sus conflictos (los ts 8..19), después los lotes de las relaciones y la `CargaCompletada` final con los conteos. La ingesta escribe el WAL del modelo: nada de lo que hizo se puede negar.

**8. Idempotencia por clave natural.** La clave `(label, nombre normalizado[, orcid])` es la «prueba de la identidad propia» aplicada a la ingesta: los ids los decide el pipeline por orden de carga, la identidad la decide la clave, nunca el fichero. Recargar el MISMO dataset produce el MISMO grafo —el test más barato del capítulo, y pasa siempre—. (El manifiesto (manifest) con hash —saltarse lo ya cargado— es la variante industrial: queda como reto experto con `fnv1a_64` del cap-15.)

**9. El caso Tema 26/61, CURADO.** Las dos filas «memoria de agentes» (los temas 26 y 61 del paso-4) son idénticas tras normalizar: mismo bloque, jaccard **1,0** → arista SIMILAR → cluster de 2 → fusión. El entity resolution no sabe de esquema: sabe de NOMBRES —y por eso cura lo que los validadores 41-44 nunca vieron: la `Existencia` exige que el nombre ESTÉ, no que sea único, y el catálogo del cap-21 solo CONTÓ valores distintos (8/9) sin poder arreglar nada—. 68 → **67 nodos**; las ABOUT que apuntaban al tema por nombre se reapuntan al canónico: **158 aristas intactas**; `Tema.nombre` 8/8. El catálogo delató, la ingesta curó —y `verificar_esquema` aprueba el resultado—. La ingesta no cambia el contrato del cap-44: lo cumple. La lección vale para todo el capítulo: la selectividad es un DETECTIVE, el pipeline es el QUE LO ARREGLA —ninguno de los dos sobra—.

Y el hilo del espaciado, recapitulado: `partir_csv` (cap-32), `componentes_conexas` (cap-25), `FronterasBfs` (cap-26), la clave natural (cap-41), `verificar_esquema` (cap-44), el transaction-time (cap-43), `fnv1a_64` (cap-15), el lexer UTF-8 (cap-18) y el determinismo (cap-34) —todos reutilizados, ninguno tocado—. El paso-5 es el capítulo que más piezas viejas ensambla de todo el Vol.III, y la prueba de fuego de que la obra reutiliza de verdad.

## 45.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap45_ingesta.rs` (**3.648 líneas**, std puro, **24 tests**), cableada con dos líneas aditivas al final del wiring de `lib.rs`:

```rust
mod cap45_ingesta;
pub use cap45_ingesta::*;
```

El módulo accede GRATIS a la API pública de los caps. 7-44: `partir_csv` (cap-32), `Violacion` (cap-41), `verificar_esquema` + `esquema_kb_lira` (cap-44), `componentes_conexas` (cap-25) y, solo en los tests de regresión, a los builders 1-4 —el paso-5 es un artefacto DERIVADO y se compara por NOMBRES, nunca por ids—. El artefacto regenerable `datasets/kb-lira/paso-5/` contiene los 4 ficheros crudos (`crudos/personas.csv` 14 filas, `documentos.csv` 38, `temas.csv` 9, `relaciones.csv` 160 —221 filas SIN ids—), el resultado (`nodes.csv` 68 líneas, `edges.csv` 159) y el informe reproducible. CERO dependencias nuevas, CERO cambios en caps. 7-44, goldens intactos. Y **NO hay `[[bench]]`**: decisión del contrato en una línea —la moneda son filas, lotes, rechazos, comparaciones evitadas, fusiones y conjuntos exactos, nunca µs— (espejo de las decisiones #12 del cap-41, #11 del cap-43 y #9 del cap-44).

Las piezas que sostienen el edificio (nombres exactos; el código completo vive en el módulo):

```rust
pub struct DatosCrudos { pub registros: Vec<RegistroCrudo> }     // etapa EXTRAER
pub fn normalizar_nombre(nombre: &str) -> String;                // minúsculas sin tildes
pub fn distancia_levenshtein(a: &str, b: &str) -> usize;         // DP, char a char
pub fn similitud_levenshtein(a: &str, b: &str) -> f64;           // 1 − dist/max
pub fn bigramas(s: &str) -> Vec<String>;                         // SIN padding
pub fn jaccard_bigramas(a: &str, b: &str) -> f64;
pub fn bloque_por_inicial(nombre: &str) -> String;               // la clave de bloque
pub fn construir_grafo_similitud(personas: &[String]) -> (MemoryStore, usize);  // SIMILAR
pub fn clusters_de_similitud(store: &MemoryStore) -> Vec<Vec<usize>>;  // cap-25 REUTILIZADA
pub fn fusionar_cluster(store: &mut MemoryStore, cluster: &[usize], nombres: &[String]) -> InformeFusion;
pub fn validar_lote_pipeline(lote, claves_entidades, claves_relaciones, aristas_aceptadas) -> Vec<Option<String>>;
fn fusionar_er(store, ids, nombres, ids_reales, regla: ReglaSimilitud, clave_primer_token, historial) -> usize;
fn mapear_relacion_pipeline(registro, store, ids) -> Result<(), String>;  // crea extremos + order
pub enum EventoIngesta { CargaIniciada, LoteValidado, RegistroRechazado, FusionEntidad,
    ConflictoDeclarado, CargaCompletada }
pub struct HistorialIngesta;                                     // append-only, ts monótono
pub fn cargar_paso5() -> (MemoryStore, HistorialIngesta, InformeIngesta);  // 67/158
pub fn informe_ingesta_reproducible() -> String;                 // pineado byte a byte
```

## 45.8 Prueba de fuego

Primero el bucle rápido —salida REAL de `cargo test`, sin tiempos—:

```text
$ cargo test -p vol2-liradb --lib cap45

running 24 tests
test cap45_ingesta::tests::los_crudos_del_paso5_tienen_las_filas_y_la_suciedad_del_contrato ... ok
test cap45_ingesta::tests::desde_csv_parsea_un_fichero_crudo_con_tipo_fijo_y_reporta_lineas_invalidas ... ok
test cap45_ingesta::tests::la_normalizacion_sin_tildes_une_variantes_de_ana ... ok
test cap45_ingesta::tests::la_distancia_de_levenshtein_distingue_ana_garcia_de_carla_mendez ... ok
test cap45_ingesta::tests::similitud_levenshtein_es_uno_para_iguales_y_cero_sin_solapamiento ... ok
test cap45_ingesta::tests::jaccard_bigramas_une_ana_garcia_con_ana_g ... ok
test cap45_ingesta::tests::el_jaccard_de_bigramas_separa_ana_g_de_carla_mendez ... ok
test cap45_ingesta::tests::el_blocking_por_inicial_evita_comparaciones ... ok
test cap45_ingesta::tests::el_grafo_de_similitud_agrupa_las_cinco_anas_en_un_cluster ... ok
test cap45_ingesta::tests::las_componentes_conexas_del_cap25_dan_los_clusters_de_entidades ... ok
test cap45_ingesta::tests::el_blocking_evita_comparaciones_en_el_grafo ... ok
test cap45_ingesta::tests::la_fusion_elige_el_canonico_con_mas_datos_y_reapunta_aristas ... ok
test cap45_ingesta::tests::mapear_registros_personas_es_idempotente_por_clave_natural ... ok
test cap45_ingesta::tests_pipeline::kb_lira_paso5_cuenta_y_etiquetas_exactas ... ok
test cap45_ingesta::tests_pipeline::el_paso5_pasa_el_esquema_del_cap44 ... ok
test cap45_ingesta::tests_regresion::las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta ... ok
test cap45_ingesta::tests_regresion::la_ingesta_preserva_el_orden_de_firma ... ok
test cap45_ingesta::tests_regresion::las_respuestas_del_paso2_no_cambian_tras_la_ingesta ... ok
test cap45_ingesta::tests_regresion::el_esquema_del_cap44_acepta_el_modelo_ingestado ... ok
test cap45_ingesta::tests_regresion::el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono ... ok
test cap45_ingesta::tests_csv_paso5::csv_paso5_roundtrip_byte_a_byte ... ok
test cap45_ingesta::tests_csv_paso5::csv_paso5_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap45_ingesta::tests_csv_paso5::csv_pasos_anteriores_intactos_tras_paso5 ... ok
test cap45_ingesta::tests_informe_ingesta::informe_ingesta_reproducible_sobre_kb_lira ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 993 filtered out
```

Veinticuatro verdes; workspace entero en **1017 ALL_GREEN** (993 + 24) con goldens intactos y los CSVs de los pasos 1-4 byte a byte. Ahora el informe del capítulo —salida REAL de `informe_ingesta_reproducible()`, pineada byte a byte, y construida con las funciones REALES: `DatosCrudos::desde_csv` sobre los ficheros commiteados, `construir_grafo_similitud` sobre los 14 nombres reales del crudo, el histórico con sus 29 eventos—:

```text
Informe de ingesta — KB-Lira paso-5 (del CSV crudo al grafo)
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
Ficheros → filas              | personas.csv 14 + documentos.csv 38 + temas.csv 9 + relaciones.csv 160 = 221 filas
Lotes de 25 filas             | 11 lotes
Rechazos (4)                  | línea 15 · clave natural repetida: Beto · línea 7 · clave natural repetida: Recuperación aumentada con grafos · línea 56 · clave natural repetida: Ana → Universidad de Lira (MEMBER_OF) · línea 57 · solape con la 53
Entity resolution · personas  | 91 pares todo-contra-todo → 11 comparadas por bloque inicial (80 evitadas)
Clusters (14 personas crudas) | 5 «Ana»… + 2 «Beto»… + 7 singletons (9 clusters)
Jaccard de bigramas           | «ana garcia»↔«ana g.» 0,4 · «ana garcia»↔«ana garcia (universidad de lira)» 0,3 · contraste «carla mendez» 0,11
Levenshtein (títulos)         | typo «…aumentada con grafo»↔«…grafos» 0,97 · siguiente par legítimo 0,77 («Recuperación aumentada con grafos» ↔ «GraphRAG: recuperación aumentada con grafos»)
Fusiones (6)                  | 4 «Ana» + 1 «Recuperación aumentada con grafos» + 1 «memoria de agentes» (el fantasma)
Conflictos declarados (6)     | orcid 0000-0001-2345-0001 ≠ 0000-0001-2345-1 — gana el canónico, nunca silencioso
Caso Tema 26/61 · curado      | «memoria de agentes» duplicada exacta: 68 → 67 nodos
Grafo final                   | 67 nodos (9 Persona · 8 Tema · 36 Documento · 4 Organizacion · 3 Proyecto · 3 Conferencia · 4 Resena) · 158 aristas
Esquema cap-44                | Ok
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
La ingesta transforma el CRUDO en grafo: 221 filas → 11 lotes → 4 rechazos → 6 fusiones (con 6 conflictos declarados, el orcid malformado entre ellos) → 67 nodos y 158 aristas. Nada se sobrescribió en silencio y el esquema del cap-44 acepta el resultado: el paso-5 es el paso-4 verificado, no una copia suya.
```

La tabla que ordena la prueba de fuego, en forma de figura:

| Pieza | Contador real |
|---|---|
| Ficheros → filas | personas 14 + documentos 38 + temas 9 + relaciones 160 = **221** |
| Lotes de 25 | **11** |
| Rechazos | **4** con línea y motivo (2 duplicados exactos, 1 MEMBER_OF duplicada, 1 solapada «solape con la 53») |
| Entity resolution | **91 → 11** comparaciones (80 evitadas) · 9 clusters |
| Fusiones | **6** (4 Anas + typo + fantasma) · conflictos declarados: **6** |
| Caso 26/61 | **68 → 67** nodos, 158 aristas intactas |
| Grafo final | 67 nodos (9 Persona · 8 Tema · 36 Documento · 4 Organizacion · 3 Proyecto · 3 Conferencia · 4 Resena) · 158 aristas |
| Esquema cap-44 | **Ok** |

El histórico, leído como película (el transaction-time del cap-43 cobrado): `CargaIniciada` (ts 1) → los 11 `LoteValidado` con los 4 `RegistroRechazado` intercalados, cada uno con su línea → las 6 `FusionEntidad` (4 Anas, el typo, el fantasma) → los 6 `ConflictoDeclarado` (el orcid entre ellos) → `CargaCompletada` con 67 nodos y 158 aristas (ts 29). La ingesta no solo carga: TESTIGA —y el test `el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono` verifica la monotonía y los conteos. Nada de lo que el pipeline hizo se puede negar después.

Cinco lecturas obligatorias. **Primera: los 4 rechazos, con nombre y apellido.** La línea 15 («Beto» duplicado) y la 7 («Recuperación aumentada con grafos» duplicado) caen por clave natural repetida; la 56 («Ana → Universidad de Lira (MEMBER_OF)») por la misma regla en relaciones; la 57 —la solapada `[2020,2023)`— por la `SinSolape` LOCAL ejecutada al vuelo: **la regla del cap-44 aplicada antes de cargar, con el id de la arista que iba a tener**. Y el pipeline siguió: cada rechazo quedó registrado como evento `RegistroRechazado` con su línea y su motivo en el histórico, y la carga terminó —rechazo selectivo, no pánico—. **Segunda: las 6 fusiones y sus 6 conflictos.** 4 «Ana» + 1 «Recuperación aumentada con grafos» (el typo) + 1 «memoria de agentes» (el fantasma). Entre los conflictos declarados está el del orcid malformado `0000-0001-2345-1`, descartado DECLARÁNDOLO —gana el canónico `0000-0001-2345-0001`, nunca silencioso—: la fila suelta habría violado la `Unicidad` del cap-44, y la fusión la salva en el acto. **Tercera: el blocking, medido.** 91 pares naive → 11 comparadas por bloque inicial → 80 evitadas, y `el_blocking_por_inicial_evita_comparaciones` lo pinea con las funciones reales. **Cuarta: el fantasma, curado.** El duplicado que el catálogo delató en el cap-44 (8/9 ≈ 0,889) desaparece por la vía que el cap-44 nunca tuvo: `kb_lira_paso5_cuenta_y_etiquetas_exactas` pinea **67 nodos** —9 Persona, 8 Tema (el fantasma se fue), 36 Documento (el typo fusionado)— y **158 aristas** intactas, `Tema.nombre` 8/8. **Quinta: la red de seguridad cuádruple, verde.** `las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta`: P1-P3, P6, P7, P9 y P10 IDÉNTICAS (responden por NOMBRES: la fusión no cambia nombres, solo ids); P4 enriquecida con la MEMBER_OF legítima Beto→GrafoLuna `[2024,∞)`; P5b responde **12** (el lote); P8 responde **9 personas / 40 publicaciones** y la fusión NO infla a Ana: sigue en **4** —las variantes fusionadas no publicaron—.

Y un HALLAZGO que el capítulo no maquilla: **la P2 (orden de firma) quedó ROTA al principio**. El CSV crudo no transportaba el `order` de los autores, y sin él la pregunta «¿quién firma primero?» del cap-41 no podía responderse —la ingesta había perdido la riqueza que el builder tenía a mano—. La solución no fue tocar la pregunta: fue añadir la columna `order` a `relaciones.csv` (40 filas AUTHORED con los órdenes reales del builder) y cargarla como prop de arista en `mapear_relacion_pipeline`. P2 responde IDÉNTICA en los **12 títulos** pineados —«Supernodos: anatomía de un cuello de botella» sigue devolviendo Beto primero (order 1) aunque Ana tenga menor id—. La lección es la tesis del capítulo en una frase: **o el formato transporta la riqueza, o se pierde**.

Y el CSV cierra el círculo: `csv_paso5_roundtrip_byte_a_byte`, `csv_paso5_coincide_con_dataset_commiteado_byte_a_byte` y `csv_pasos_anteriores_intactos_tras_paso5` —los ficheros de los pasos 1-4 ni se tocan, y el paso-5 viaja en el formato del cap-32, ahora con el `order` en la propia cabecera—:

```text
$ head -2 datasets/kb-lira/paso-5/nodes.csv
id:ID, afiliacion:STRING, anio:INT, nombre:STRING, orcid:STRING, titulo:STRING, :LABEL
0,Universidad de Lira,,Ana,0000-0001-2345-0001,,Persona

$ head -2 datasets/kb-lira/paso-5/edges.csv
id:ID, de:START_ID, a:END_ID, tipo:TYPE, desde_anio:INT, hasta_anio:INT, order:INT
0,0,13,AUTHORED,,,1
```

La fusión no dejó huecos ilegibles: el canónico de las Anas es el id 0, el de «memoria de agentes» es el 52 —y ya solo existe UNO—, y «Instituto GrafoLuna» nació como Organizacion desde el extremo de la arista, sin fichero propio.

Y una última lectura, la del lector desconfiado: el informe no es decoración —cada fila tiene su test—. `el_blocking_por_inicial_evita_comparaciones` pinea el 91 → 11; `el_grafo_de_similitud_agrupa_las_cinco_anas_en_un_cluster` pinea el cluster de 5; `el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono` pinea las 6 fusiones y los 4 rechazos con sus ts; `kb_lira_paso5_cuenta_y_etiquetas_exactas` pinea los 67 nodos con sus etiquetas y las 158 aristas; y `informe_ingesta_reproducible_sobre_kb_lira` pinea el informe ENTERO, byte a byte. El texto que acabas de leer en la tabla es, literalmente, una salida de `cargo test`.

## 45.9 Qué hemos sacrificado

1. **Sin ingesta RDF.** El pipeline entiende CSVs y JSONL de un LPG; las tripletas y quads son el cap. 46 —«¿y si los datos llegan como tripletas?» es el gancho de salida, y el paso-6 del hilo será su exportación N-Triples—.
2. **Sin SHACL ni shapes.** El hermano declarativo del mundo RDF se nombra como gancho (caps. 46-47), no se explica.
3. **Sin entity resolution probabilístico.** Fellegi-Sunter (1959) se NOMBRA en el §45.10 sin explicarlo: aquí la similitud es determinista, con umbrales fijos y explicables —eso la hace testeable—. La frontera, declarada: el ER probabilístico asigna probabilidades de match y delega los bordes a un humano; el determinista pinea cada decisión en el histórico. Dos filosofías, y esta obra elige la que se puede `cargo test`.
4. **Sin manifiesto industrial.** El hash + skip (`fnv1a_64` del cap-15) queda como reto experto: la clave natural enseña más que el manifiesto, y el manifiesto añade estado externo que gestionar.
5. **Sin streaming distribuido.** Spark/Dataflow se nombran como la escala industrial; aquí el pipeline es de una máquina: determinista, local y reproducible.
6. **La fusión es determinista y local.** Sin aprendizaje, sin probabilidades, sin merges distribuidos: cada fusión tiene su porqué en el `HistorialIngesta`, y el pipeline vive en RAM (sus hermanos persistentes son el cap-15/28, nombrados).

## 45.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j** ofrece `neo4j-admin database import`, la importación masiva OFFLINE por CSV con cabeceras `:ID`/`:LABEL`/`:START_ID`/`:END_ID`/`:TYPE` —el estilo que el cap-32 copió y que tus `nodes.csv`/`edges.csv` heredan— y `LOAD CSV` desde Cypher, streaming línea a línea dentro de la consulta. Detalle no verificado aquí (frontera declarada): la importación admin NO es idempotente por diseño —el manifiesto de qué se cargó es del equipo, no de la herramienta—, exactamente el reto que tu clave natural resuelve. En el mundo **Python**, el Record Linkage Toolkit **recordlinkage 0.15** (indexing por bloques, comparadores, umbrales) y **dedupe 3.0.2** (machine learning con aprendizaje activo) son las herramientas industriales de entity resolution: tu bloque por inicial es su indexing más simple. Y la teoría: **Swoosh** —Benjelloun, Garcia-Molina, Menestrina, Su, Whang y Widom, «Swoosh: a generic approach to entity resolution», The VLDB Journal 18(1):255-276, 2009— trata match y merge como cajas negras y demuestra propiedades (las ICAR) que garantizan convergencia; tu grafo de similitud es su versión mínima, con una lección que Swoosh sí firma: **el resultado de la fusión es una hipótesis, no un veredicto**. El libro de referencia de todo esto es **Christen**, «Data Matching: Concepts and Techniques for Record Linkage, Entity Resolution, and Duplicate Detection» (Springer, agosto 2012): normalización, blocking, métricas y merge policies —la regla «canónico con más datos» es la suya—. Y el pariente mayor de tu pipeline se llama **ETL** (extract-transform-load): Kimball, «The Data Warehouse Toolkit» (Wiley, 1996), ya citado en esta obra —tu fábrica ES un ETL de grafos, con el esquema como transformación: extraer es el *extract*, mapear es el *transform*, cargar es el *load*—. El clásico que implementaste a mano, **Levenshtein**, se publicó en Soviet Physics—Doklady 10(8):707-710, febrero de 1966: casi sesenta años después, sigue siendo la frontera entre «typo» y «paper legítimo».

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (45+41+44): añade al `crudos/personas.csv` una fila nueva —un tercer «Beto» duplicado exacto— y PREDICE por escrito, ANTES de correr nada: (a) el rechazo (línea y motivo exactos), (b) el grafo final (¿cambia el conteo de nodos?), (c) por qué las 10 preguntas del cap-41 no cambian (responden por NOMBRES: la identidad la da la clave natural, los ids son nuevos). Y como cierre de Parte, recupera la mecánica DE MEMORIA: recita las 4 etapas, la regla de fusión y el porqué del blocking (91 → 11), y clasifica 5 modos de fallo nuevos —¿lo rechaza la validación, lo fusiona el ER, o lo carga tal cual?—. Pista: la fila nueva muere en la validación, no en la fusión.
- *Intermedio* (45+32+15): compara el pipeline con `importar_csv_nodos` del cap-32 en una tabla de dos columnas —qué cubre cada uno: cabecera `:ID`, streaming, validación, dedup, idempotencia, histórico—, y construye el manifiesto de carga real: hash de cada fichero con `fnv1a_64` del cap-15 y skip si ya está cargado (demuestra con un test que la segunda carga no escribe). Variante: el extraer-JSONL reutilizando `parsear_json` del cap-32 —el MISMO pipeline, otra etapa extraer—.
- *Experto* (45+43+28): el entity resolution probabilístico por TIPO de entidad —umbrales distintos (personas 0,5/0,25; títulos 0,9) y blocking multi-campo (inicial + token + longitud)— y la comparación de tus clusters contra los 9 del capítulo con su justificación; y alimenta el `HistoricoAfiliaciones` del cap-43 desde la ingesta: cada MEMBER_OF importada se registra y la fusión re-registra bajo el canónico —«la ingesta escribe el WAL del modelo»—. Restricciones: std puro, sin tocar cap-43 ni cap-28, suite ALL_GREEN.

## 45.11 Lo que te llevas

- **La ingesta es un pipeline de cuatro etapas, no una copia.** Extraer, validar, mapear, cargar —cada una con su tipo y sus contadores—. El cap-32 era la versión para ficheros limpios con `:ID`; su `partir_csv` sobrevive, su importer no.
- **El lote es la frontera de la RAM.** Una pasada por fichero, 25 registros como unidad residente máxima: el streaming del cap-32 con frontera explícita, el espejo de `FronterasBfs` — y la frontera que hace el fail-fast posible.
- **Validar es write-time.** Por lote, con rechazo selectivo `(línea, motivo)`, y el esquema como puerta final: el lote rechaza, el esquema sanciona. La fila sucia no aborta el lote: lo registra en el histórico y sigue.
- **La identidad la da la clave natural, nunca el fichero.** Ids nuevos por carga, misma clave → mismo grafo. Recargar dos veces no asusta: es el test más barato del pipeline, y pasa siempre.
- **Entity resolution: normalizar → bloquear → comparar → grafo → componentes → fusionar.** El blocking no es una optimización: 91 → 11 comparaciones, 80 evitadas, y O(n²) queda fuera.
- **El grafo de similitud es un grafo.** Las componentes conexas del cap-25 —el BFS del Vol.I— agrupan duplicados exactamente igual que agrupaban comunidades.
- **La métrica es una decisión, no un dogma.** Jaccard de bigramas para nombres (0,4 con refuerzo de primer token), Levenshtein ≥ 0,9 para títulos (typo 0,97, par legítimo 0,77) —calibrada sobre el dataset real y pineada en el informe—.
- **Nada se sobrescribe en silencio.** Conflictos declarados (el orcid malformado, descartado con motivo), `HistorialIngesta` append-only con 29 eventos ts 1..29: la fusión deja huella, el transaction-time del cap-43 alimentado.
- **Delatar no es curar.** El catálogo delató el 26/61; la ingesta lo curó —68 → 67— y el esquema aprueba el resultado. La ingesta no cambia el contrato: lo cumple.

## 45.12 Ojo, cuidado con…

- **Reutilizar `importar_csv_nodos` para crudos.** Exige `:ID` y autocommitea sin preguntar: dos Anas entran sin que nadie proteste. Es la primera solución del capítulo, no la solución.
- **Validar solo al final.** La fila 57 entra, la `SinSolape` la denuncia cuando ya no se puede hacer nada barato. El rechazo con número de línea es la lección: fail-fast en la frontera del lote.
- **Comparar todo contra todo.** O(n²) no escala, y el contador (91 → 11) lo demuestra antes de que duela. El blocking se decide con la clave de bloque, no con esperanza.
- **Elegir una sola métrica para todo.** Levenshtein pierde abreviaturas y paréntesis («ana garcia» vs «ana g.» no llega); jaccard sobrefunde títulos largos (0,63 entre legítimos). La métrica se elige por TIPO de entidad y se calibra con el dataset real.
- **Fusionar «el último gana».** Destructivo y arbitrario: el conflicto del orcid se declara y se descarta con motivo, nunca en silencio —y el `HistorialIngesta` lo demuestra—.
- **Rechazar en la validación lo que la fusión debe curar.** El duplicado exacto de Tema NO se rechaza (es el fantasma); el orcid malformado NO se rechaza (debe llegar a la fusión para que el conflicto se declare). La frontera entre «rechazar» y «fusionar» es contenido, no detalle.
- **«Arreglar» el esquema del cap-44.** Añadir `Unicidad` a `Tema.nombre` rompería el paso-4 (8/9): el pipeline no cambia el contrato, lo cumple. Y si la regresión de las 10 preguntas falla, el error está en la ingesta, no en la pregunta — prohibido ajustar nada para que cuadre.
- **Confundir el enriquecimiento legítimo con una rotura.** La P4 de la ingesta devuelve MÁS —Beto→GrafoLuna `[2024,∞)` es una MEMBER_OF real del dataset, sin solape— y la P5b responde 12 en vez de 6: son los pines del capítulo, no un fallo. La regresión manda sobre lo que NO debía cambiar; el contenido nuevo se pinea aparte y se explica.
- **Confundir el bloque con la clave natural.** El bloque (la primera letra) reduce candidatos para COMPARAR; la clave natural (`label` + nombre normalizado) identifica para CARGAR. El «Beto» duplicado muere en la clave; las variantes de Ana viven hasta la fusión. Son dos mecanismos, dos etapas y dos contadores distintos.

## 45.13 Pin de batalla

> *«Un grafo no nace limpio: nace de datos crudos, y es el pipeline quien lo limpia. Extrae, valida, mapea, fusiona —con contadores en cada estación y el esquema como control de calidad—. Y recuerda: la identidad la da la clave natural, nunca el fichero; y nada se sobrescribe en silencio.»*

## 45.14 Si solo lees 30 segundos

KB-Lira paso-5: **221 filas crudas** (14 personas, 38 documentos, 9 temas, 160 relaciones —sin ids, con suciedad deliberada) → pipeline de 4 etapas → **67 nodos / 158 aristas**, esquema cap-44 `Ok`. Extraer en lotes de 25 (11 lotes, una pasada); validar por lote con rechazo selectivo (4 rechazos: línea 15 «Beto» duplicado, línea 7 título duplicado, línea 56 MEMBER_OF duplicada, línea 57 solape con la 53) + `verificar_esquema` como puerta final; mapear con `MAPEO_COLUMNAS` y resolución de nombres (Organizacion/Proyecto/Conferencia/Resena creadas desde los extremos). Entity resolution a mano: normalizar (sin tildes, paréntesis conservados) → bloquear por inicial (91 → 11 comparaciones, 80 evitadas) → jaccard de bigramas (0,4 · 0,3 · 0,11) → grafo SIMILAR → componentes conexas del cap-25 (9 clusters) → fusión cuidada (canónico con más datos; conflictos declarados: orcid `0000-0001-2345-0001 ≠ 0000-0001-2345-1`; aristas reapuntadas). Títulos: Levenshtein ≥ 0,9 (typo 0,97; par legítimo 0,77). **6 fusiones**: 4 Anas + el typo + el fantasma Tema 26/61 curado (68 → 67, `Tema.nombre` 8/8 —delatar no es curar—). `HistorialIngesta`: 29 eventos ts 1..29 (la forma del cap-43). Idempotencia por clave natural: recargar el mismo CSV → mismo grafo. Regresión: P1-P3, P6, P7, P9, P10 idénticas; P4 enriquecida (Beto→GrafoLuna); P5b 12; P8 9 personas/40 pubs (Ana sigue en 4); **P2 reparada** con la columna `order` (12 títulos pineados —o el formato transporta la riqueza, o se pierde—). **1017 tests ALL_GREEN**, sin bench. Fronteras: RDF (46), SHACL (47), LLM como extractor (52).

## 45.15 Una historia pequeña

Un carácter. Solo uno. En algún momento alguien tecleó «Recuperación aumentada con grafo» —sin la ese— y el CSV lo recibió como un documento más. La similitud de Levenshtein entre el título con el typo y el legítimo mide **0,97**: UNA edición. Para cualquier humano eran el mismo paper, y para la métrica también. El siguiente par de títulos legítimos más cercano está a **0,77**: margen de sobra para que un umbral bien calibrado (≥ 0,9) diga «esto es un typo, fúndelo» y «esto son dos papers reales, NO los toques». El typo se fusionó con el canónico —el mismo mecanismo que curó al fantasma de los temas y unió a las cinco Anas— y el informe lo declara: «Fusiones (6) | 4 «Ana» + 1 «Recuperación aumentada con grafos» + 1 «memoria de agentes» (el fantasma)». La lección no es que un carácter engañe a las métricas: es que la frontera entre «igual» y «distinto» se decide con números MEDIDOS sobre el dataset real, no con intuiciones. Y cuando la línea se cruza, el histórico lo dice todo: `FusionEntidad`, con su ts, sin drama y sin silencio.

## Ejercicios resueltos

**1. Retrieval sin pistas: las 4 etapas del pipeline DE MEMORIA, la regla de fusión, y 5 modos de fallo.** Cierra el libro y recita las cuatro estaciones: **extraer** (leer filas, streaming por lotes), **validar** (reglas de fila por lote con número de línea + el esquema del cap-44 como puerta final), **mapear** (columnas crudas al vocabulario del modelo + nombres → entidades), **cargar** (`put_node`/`put_edge` con ids nuevos). La regla de fusión: canónico = miembro con MÁS props no vacías (empate → menor id); iguales se conservan, ausentes se rellenan, distintas se DECLARAN y gana el canónico; aristas reapuntadas. Ahora clasifica 5 modos de fallo: (a) «la MEMBER_OF de Beto `[2020,2023)` llega al lote» → la **RECHAZA** la validación (SinSolape local, «solape con la 53»); (b) «una variante nueva de Ana: "Ana G."» → la **FUSIONA** el entity resolution (mismo bloque, mismo primer token, jaccard 0,4 ≥ 0,25); (c) «un título legítimo distinto» → lo **CARGA** tal cual (Levenshtein 0,77 < 0,9: sin arista SIMILAR); (d) «el orcid malformado de una variante de Ana» → **pasa** la validación deliberadamente y la fusión lo **DECLARA** como conflicto; (e) «la cabecera del CSV rota» → **ABORTA** la extracción (`LoteInvalido`): la única excepción al rechazo selectivo.

**2. Explica por qué 91 → 11, con los bloques.** Mecánica: `bloque_por_inicial` agrupa por la primera letra del primer token normalizado y solo se comparan los nombres del MISMO bloque. Con los 14 nombres reales del crudo: las 5 Anas comparten «a» (C(5,2) = 10 pares), los 2 Beto comparten «b» (1 par) y los 7 restantes viven en bloques de un miembro (0 pares). 10 + 1 = **11 comparadas** frente a C(14,2) = **91** del todo-contra-todo: **80 evitadas**. El blocking no es una optimización de rendimiento: es la diferencia entre 91 comparaciones y ~98 millones cuando haya 14.000 personas.

**3. Responde por qué el typo se fusiona y el par legítimo no.** Mecánica: los títulos usan `similitud_levenshtein ≥ 0,9` como regla de arista `SIMILAR`. «Recuperación aumentada con grafo» vs «Recuperación aumentada con grafos» difieren en UNA inserción: similitud = 1 − 1/max ≈ **0,97** ≥ 0,9 → arista → cluster → fusión. El siguiente par legítimo («Recuperación aumentada con grafos» ↔ «GraphRAG: recuperación aumentada con grafos») mide **0,77** < 0,9 → sin arista. El margen (0,97 vs 0,77) es el aire que separa el typo del paper real: un umbral calibrado sobre el dataset real, no sobre la pizarra. Y por qué no jaccard para títulos: sobrefunde —0,63 entre títulos legítimos distintos—.

**4. Explica por qué la P2 necesitaba la columna `order`.** Mecánica: la pregunta del cap-41 lee `e.props["order"]` de cada AUTHORED —el orden de firma es una prop de ARISTA, no un derivable del id (en «Supernodos: anatomía de un cuello de botella», Beto firma con id mayor y order 1: el id no codifica el orden). El builder del paso-1 tenía los órdenes a mano; el CSV crudo no los transportaba, y la primera versión de la ingesta los perdió en silencio —el test `la_ingesta_preserva_el_orden_de_firma` no habría existido. La cura fue del FORMATO: la columna `order` en `relaciones.csv` y su lectura en `mapear_relacion_pipeline`. La lección general: si una riqueza no viaja en el formato, la ingesta no la puede inventar —GIGO, pero a la inversa: lo que no entra, no sale.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 45+41+44).** Desarrolla el reto esencial del §45.10: añade al `crudos/personas.csv` una fila nueva —un tercer «Beto» duplicado exacto— y PREDICE por escrito, ANTES de correr nada: (a) el rechazo con su línea y motivo, (b) el grafo final (nodos y aristas: ¿cambian?), (c) por qué las 10 preguntas del cap-41 responden igual. Criterio: predicción escrita primero; si no predijiste «rechazado en validación, grafo intacto», revisa la diferencia entre clave natural e id.

**Intermedio (predecir y comparar; 45+32+15).** (a) Tabla comparativa `importar_csv_nodos` (cap-32) vs `cargar_paso5`: qué cubre cada uno —cabecera `:ID`, streaming, frontera de lote, validación, dedup, idempotencia, histórico—, cada diferencia con su porqué. (b) El manifiesto de carga: hash de cada fichero con `fnv1a_64` del cap-15 y skip si ya está cargado, demostrado con un test (la segunda carga no escribe). (c) El extraer-JSONL: `parsear_json` del cap-32 como etapa extraer alternativa —el MISMO pipeline, otra fuente—. Criterio: el manifiesto con su test verde; la tabla con sus porqués.

**Experto (crear y demostrar; 45+43+28).** El entity resolution probabilístico por tipo: umbrales y claves de bloque POR TIPO de entidad (personas: inicial + primer token; títulos: inicial + longitud), y compara tus clusters con los 9 del capítulo justificando cada diferencia. Y alimenta el `HistoricoAfiliaciones` del cap-43 desde la ingesta: cada MEMBER_OF importada se registra y la fusión re-registra bajo el canónico —«la ingesta escribe el WAL del modelo»—. Restricciones: std puro, sin tocar cap-43 ni cap-28, suite ALL_GREEN. Criterio: cada cluster con su justificación; el histórico alimentado sin tocar su `registrar`.

## Para profundizar

- **Peter Christen, «Data Matching: Concepts and Techniques for Record Linkage, Entity Resolution, and Duplicate Detection», Springer (Data-Centric Systems and Applications), agosto 2012** — el libro de referencia de las tres secciones de entity resolution: normalización, blocking, métricas de similitud y merge policies (la regla «canónico con más datos»).
- **Benjelloun, Garcia-Molina, Menestrina, Su, Whang & Widom, «Swoosh: a generic approach to entity resolution», The VLDB Journal 18(1):255-276, 2009** (DOI 10.1007/s00778-008-0098-x) — match/merge como cajas negras y las propiedades ICAR: «el resultado de la fusión es una hipótesis, no un veredicto».
- **V.I. Levenshtein, «Binary codes capable of correcting deletions, insertions, and reversals», Soviet Physics—Doklady 10(8):707-710, febrero 1966** (orig. ruso 1965) — la distancia implementada a mano en `distancia_levenshtein`.
- **«Garbage in, garbage out» (GIGO), The Hammond Times, 10 de noviembre de 1957** (vía Newspapers.com) — la primera aparición en prensa documentada, en boca de matemáticos del Ejército de EE.UU.; la acuñación se atribuye a George Fuechsel (IBM, instructor de programación) sin fecha firme.
- **Neo4j Operations Manual, `neo4j-admin database import`** (neo4j.com/docs/operations-manual/current/import/) — la importación masiva offline; el cap-32 copió su estilo de cabecera `:ID`/`:LABEL`. Detalles finos de cabecera no verificados aquí (frontera declarada del contrato).
- **Record Linkage Toolkit (recordlinkage 0.15) y dedupe 3.0.2 (Python)** — las herramientas industriales de entity resolution: indexing por bloques y machine learning con aprendizaje activo.
- **Ralph Kimball, «The Data Warehouse Toolkit», Wiley, 1996** — ETL, ya citado en esta obra.
- Dentro del libro: cap. 41 (identidad propia, la escalera R1-R7, las 10 preguntas), cap. 42 (antipatrones, la P5 jerárquica), cap. 43 (transaction-time, `HistoricoAfiliaciones`, la convención `[desde, hasta)`), cap. 44 (el esquema, `SinSolape`, `verificar_esquema`, el fantasma 26/61), cap. 25 (`componentes_conexas`), cap. 26 (`FronterasBfs`/`Presupuesto`), cap. 32 (`partir_csv`, `parsear_json`, `importar_csv_nodos`), cap. 15 (`fnv1a_64`), cap. 18 (lexer UTF-8), cap. 34 (determinismo), caps. 46 (RDF: tripletas, quads), 47 (SHACL), 52 (el LLM como extractor).

## Mini-diálogo: en guardia nocturna

> — Son las dos de la madrugada. La operadora ha recargado el CSV de personas con el importer del cap-32 y la consulta del Proyecto Kira devuelve DOS Anas.
>
> — (pausa) ¿Dos Anas o dos filas?
>
> — ¿Qué diferencia hay? El informe del pipeline dice que se fusionaron 4 y que quedó una. Pero yo veo dos.
>
> — Entonces esa recarga no pasó por el pipeline. El crudo tiene cinco variantes: «Ana», «ana garcia», «Ana G.», «ana garcía» y «Ana García (Universidad de Lira)». El pipeline las normaliza, las bloquea por inicial, las une con bigramas y deja UNA —la que tiene más datos, con el orcid bien formado—. El orcid malformado no se perdió: está declarado en el histórico, con su conflicto y su ts. Nadie se tragó nada en silencio.
>
> — ¿Y el martes importan cuarenta mil personas de otro legado?
>
> — (pausa) Cuarenta mil personas sin blocking son ochocientos millones de comparaciones. Con bloques por inicial, una fracción diminuta —y el pipeline las agrupa igual, grafo de similitud mediante. Si alguien importa cuarenta mil filas con el importer del cap-32, no tendrás dos Anas: tendrás cuarenta mil Anas y nadie las verá.
>
> — La basura no se detecta sola.
>
> — Exacto. La detecta el pipeline, y el pipeline eres tú. Buenas noches.

> *Siguiente parada, cap. 46 (RDF): tu pipeline entiende CSVs y JSONL — ¿y si los datos llegan como tripletas? La Parte I se cierra aquí: cinco capítulos, cinco peldaños. El cap-41 modeló KB-Lira (la escalera R1-R7, la identidad como decisión), el 42 pagó los antipatrones (supernodos, reificación), el 43 le dio tiempo (valid-time y transaction-time), el 44 le dio contrato (el esquema declarativo, la selectividad como detective) y el 45 le enseñó a NACER desde la basura (el pipeline, la clave natural, la fusión cuidada). El modelo, sus reglas, su historia, su verificación y su nacimiento: esa es la Parte I. La Parte II cambia de filosofía —de propiedades a tripletas—: el cap-46 abre con RDF (tripletas, quads, la otra gran familia de modelos), el 47 con ontologías y SHACL, y el 48 con los lenguajes de consulta: Cypher, GQL, SPARQL. Abiertas: el LLM como extractor no determinista que sustituiría la etapa extraer (cap-52) y la memoria del agente (cap-53).*
# Epílogo — El grafo como capa de conocimiento

> *Placeholder — se redactará tras el cap. 53.*

---

# Colofón

> *Placeholder.*

---
