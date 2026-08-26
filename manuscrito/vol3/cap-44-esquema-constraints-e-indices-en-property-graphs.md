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
