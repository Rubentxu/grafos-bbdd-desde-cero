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
# Epílogo — El grafo como capa de conocimiento

> *Placeholder — se redactará tras el cap. 53.*

---

# Colofón

> *Placeholder.*

---
