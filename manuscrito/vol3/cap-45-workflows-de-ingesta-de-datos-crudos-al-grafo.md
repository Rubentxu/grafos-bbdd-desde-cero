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
