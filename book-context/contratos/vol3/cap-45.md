# CONTRATO DE CAPÍTULO — Vol.III Cap. 45: Workflows de ingesta: de datos crudos al grafo

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **QUINTO CAPÍTULO DEL VOL.III**
> («Grafos en la era de la IA: modelar, razonar y recuperar») y **CIERRE DE LA PARTE I**
> «Modelar datos de grafos». Audiencia: el lector que terminó el cap. 44 — o el perfil
> datos/IA que lo haya leído en diagonal — y que ya tiene la escalera R1-R7, el esquema
> declarativo, el transaction-time del cap-43 y la regresión como red de seguridad:
> HERRAMIENTAS, no contenido. COBRA los ganchos salientes de los caps. 43-44: (1) el
> gancho LITERAL del cierre del cap-44 («el esquema existe, pero nadie valida al importar:
> el lote de mañana puede traer un duplicado — ¿quién lo ataja en el pipeline?») — este
> capítulo responde CON DATOS: el pipeline aplica las reglas del esquema AL VUELO por lote
> (la `ReglaSinSolape` incluida: la MEMBER_OF solapada `[2020,2023)` se RECHAZA con número
> de línea antes de cargar) y `verificar_esquema` es la puerta FINAL que el paso-5 cruza
> con `Ok`; (2) la deuda del cap-43 («el transaction-time que la ingesta debe alimentar
> automáticamente») — la fusión REGISTRA su evento en un `HistorialIngesta` append-only
> con ts monótono, la forma del `HistoricoAfiliaciones` del cap-43 replicada para eventos
> de carga; (3) el duplicado residual que el catálogo del cap-44 DELATÓ (`Tema.nombre`
> 8/9 ≈ 0.889: los temas 26 y 61 comparten «memoria de agentes») — la ingesta lo CURA: el
> entity resolution fusiona las dos filas y el paso-5 termina con **67 nodos** (68 − 1
> fantasma) y las **158 aristas** intactas, esquema `Ok`. Progresión del hilo conductor:
> paso-1 (41) = modelo sano; paso-2 (42) = refactors; paso-3 (43) = valid-time; paso-4
> (44) = esquema/índices; paso-5 (ESTE capítulo) = **CSVs crudos con duplicados → KB-Lira
> deduplicada**: el mismo grafo que los validadores 41-44 describían, PRODUCIDO desde
> basura deliberada por un pipeline reproducible. Código ancla VERIFICADO hoy
> (2026-08-26): `importar_csv_nodos` del cap-32 (cap32_import_export.rs:249) YA es
> streaming (línea a línea, autocommit por fila, cabecera `:ID`/`:LABEL` obligatoria)
> pero SIN frontera de lote, SIN validación y SIN dedup — el cap-32 es la «primera
> solución» que este capítulo supera; `partir_csv` (cap32:180) es PÚBLICA y se REUTILIZA;
> `parsear_json` (cap32:739) es PÚBLICA (extraer-JSONL = reto intermedio); `texto_a_valor`
> es privada → se replica local (10 líneas); `Violacion` (cap41:354) se reutiliza;
> `verificar_esquema` (cap44:163) + `esquema_kb_lira()` (16 reglas) son la etapa VALIDAR
> y la puerta final; `HistoricoAfiliaciones` (cap43:813) se REPLICA en forma, NO se toca;
> `componentes_conexas` (cap25:696, pública) se REUTILIZA sobre el grafo de similitud
> temporal — los clusters de duplicados son las componentes conexas del cap-25;
> `fnv1a_64` (cap15:226) es la semilla del manifiesto (reto experto); la API pública de
> `GraphStore` (put_node/put_edge/iter_*) es la ÚNICA vía de escritura — el pipeline NO
> toca el store. Estado: **993 tests ALL_GREEN** (cap-44), toolchain 1.96.0, runtime
> dependency-free. Código NUEVO previsto: UN módulo `src/cap45_ingesta.rs` (~900-1300
> líneas, std puro, ~20 tests) + wiring ADITIVO en `lib.rs` (2 líneas) + artefacto
> regenerable `datasets/kb-lira/paso-5/` (crudos/ + resultado + historial.csv + informe)
> — CERO deps nuevas, CERO cambios en caps. 7-44, **SIN bench** (espejo de las decisiones
> #12 cap-41, #11 cap-43 y #9 cap-44: la moneda son filas, lotes, rechazos, comparaciones
> evitadas, fusiones y conjuntos exactos, no µs). Citas VERIFICADAS hoy (2026-08-26,
> venue/fecha exactos): Peter Christen, **«Data Matching: Concepts and Techniques for
> Record Linkage, Entity Resolution, and Duplicate Detection»**, Springer (Data-Centric
> Systems and Applications), **agosto 2012** (274 págs.); Benjelloun, Garcia-Molina,
> Menestrina, Su, Whang & Widom, **«Swoosh: a generic approach to entity resolution»**,
> **The VLDB Journal 18(1):255-276, 2009** (DOI 10.1007/s00778-008-0098-x); V.I.
> Levenshtein, **«Binary codes capable of correcting deletions, insertions, and
> reversals»**, **Soviet Physics—Doklady 10(8):707-710, febrero 1966** (orig. ruso 1965);
> «garbage in, garbage out» (GIGO): atribuido a **George Fuechsel, IBM, ~1957** (primera
> impresión 10 nov 1957, The Hammond Times; [VERIFICAR el detalle fino]); Kimball, «The
> Data Warehouse Toolkit», Wiley 1996 — YA citado en la obra; Neo4j **`neo4j-admin
> database import`** (Operations Manual actual: neo4j.com/docs/operations-manual/current/
> import/ — página y comando verificados; [VERIFICAR la cabecera `:ID`/`:LABEL`/
> `:START_ID`/`:END_ID`/`:TYPE` contra el manual]); Python Record Linkage Toolkit
> **recordlinkage 0.15** y **dedupe 3.0.2** — verificados, se NOMBRAN en N.10. RDF/quads
> NO (cap. 46). Gancho saliente: cap. 46 (RDF: «¿y si llegan tripletas?»), cap. 47
> (SHACL), cap. 52 (el LLM como extractor no determinista sustituye la etapa extraer).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: la escalera R1-R7, los validadores por composición y las
  10 preguntas (cap. 41); los antipatrones y refactors con coste (cap. 42); valid-time
  `[desde, hasta)` y el `HistoricoAfiliaciones` append-only con ts monótono (cap. 43); el
  esquema DECLARATIVO (16 reglas, 6 variantes) + `verificar_esquema`, el UNIQUE ≡ índice,
  `ReglaSinSolape`, `IntervaloValido` y el duplicado residual Tema 26/61 delatado por la
  selectividad (cap. 44); el CSV del cap-32 (cabecera `:ID`/`:LABEL`/sufijos `:TIPO`,
  round-trip byte a byte) y su importer STREAMING (línea a línea, autocommit por fila);
  `componentes_conexas` (cap-25), `FronterasBfs`/`Presupuesto` (cap-26), `fnv1a_64`
  (cap-15); la disciplina de contadores y el determinismo total (caps. 26, 34); la API
  pública de `GraphStore` (put_node/put_edge/iter_*). Perfil IA/datos: entra por el
  prólogo con estas piezas como prerrequisito declarado.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «La ingesta es copiar el CSV al grafo: importar_csv_nodos del cap-32 ya lo hace» —
  NO: el cap-32 importa ficheros LIMPIOS con `:ID` y autocommits cada fila; un fichero
  crudo (sin ids, con duplicados y suciedad) lo hace fallar en la cabecera o en
  `DuplicateNode`. Ingesta es un PIPELINE de cuatro etapas con reglas y contadores, no un
  bucle de copia.
  (2) «El esquema del cap-44 era decorativo: se verifica bajo demanda y ya está» — NO: el
  gancho literal del cap-44 es que nadie valida AL IMPORTAR; este capítulo aplica las
  mismas reglas como write-time (por lote) y como puerta final.
  (3) «Deduplicar es quitar filas repetidas» — NO: eso es la mitad; la otra mitad es
  entity resolution: «Ana G.», «ana garcia» y «Ana García (Universidad de Lira)» son la
  MISMA persona y hay que FUSIONARLA (y reapuntar sus aristas), no borrar la fila.
  (4) «Para encontrar duplicados hay que comparar todo contra todo» — NO: O(n²) no
  escala; el blocking agrupa por clave de bloque y solo compara dentro del bloque — con
  contadores que lo demuestran (78 comparaciones naive frente a ~11 bloqueadas).
  (5) «Al fusionar, la última fila gana (o la primera, da igual)» — NO: la fusión tiene
  regla explícita (canónico con más datos, empate por menor línea) y la SOBRESCRITURA ES
  CUIDADA: todo conflicto se declara en el informe, nada se sobrescribe en silencio.
  (6) «Cargar dos veces el mismo fichero es un error del operador» — NO: es el test más
  barato de un pipeline sano; la idempotencia por clave natural es una PROPIEDAD de
  diseño (la «prueba de la identidad propia» del cap-41 aplicada a la ingesta: el
  pipeline asigna ids NUEVOS, la identidad la da la clave natural).
  (7) «El duplicado Tema 26/61 del cap-44 es un caso cerrado: el catálogo lo delató» —
  NO: delatar no es curar; nadie lo arregló y el grafo del paso-4 lo sigue llevando. La
  ingesta con entity resolution lo FUSIONA: el arco del cap-44 se cierra aquí.
- **No debe saber todavía**: RDF, tripletas, quads, IRIs y la ingesta RDF (cap. 46 — el
  pipeline de aquí es de CSVs/JSONL de un LPG); SHACL y shapes (cap. 47 — nombrados como
  gancho); extracción con LLM (cap. 52 — el LLM como extractor NO determinista se NOMBRA
  como la frontera: el pipeline de aquí es determinista y eso lo hace testeable); entity
  resolution PROBABILÍSTICO (Fellegi-Sunter se NOMBRA en N.10, no se explica); pipelines
  distribuidos (Spark/Dataflow nombrados como la escala industrial); el manifiesto con
  hash como mecanismo PRINCIPAL (la clave natural es la lección; el manifiesto es reto
  experto). El paso-5 NO pisa esas fronteras.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| **Dataset crudo deliberadamente sucio** `datasets/kb-lira/paso-5/crudos/` — 4 ficheros SIN ids y SIN cabecera estilo cap-32 (columnas renombradas: `nombre_autor,orcid`; `titulo_obra,anio_publicacion,tipo`; `tema_nombre`; `tipo_relacion,de,a[,desde_anio]`), re-expresan TODO el paso-4 + suciedad deliberada: 3 duplicados EXACTOS (Beto, DOC_RAG, una MEMBER_OF), 4 variantes de Ana («Ana G.», «ana garcia», «Ana García (Universidad de Lira)», una con orcid malformado `0000-0001-2345-1`), 1 casi-duplicado con typo en el título, el fantasma «memoria de agentes» (26/61) y 1 MEMBER_OF solapada `[2020,2023)`. ~198 filas. Generado por `dataset_crudo_kb_lira_paso5() -> String` (transformación determinista del paso-4 + suciedad) y commiteado | `csv_crudo_contiene_suciedad_deliberada` (pines: filas por fichero, las 4 Anas, el typo, el fantasma, la solapada) y `csv_crudo_coincide_con_dataset_commiteado_byte_a_byte` |
| **Pipeline por etapas** `PipelineIngesta { extraer, validar, mapear, cargar }` — etapas puras con salida tipada (`DatosCrudos → DatosValidados → Mapeo → MapeoResuelto → ResultadoIngesta`); `kb_lira_paso5() -> (MemoryStore, HistorialIngesta, InformeIngesta)`: **67 nodos / 158 aristas**, ids NUEVOS asignados por el pipeline (la identidad la da la clave natural, cap-41), schema `Ok` | `kb_lira_paso5_cuenta_y_etiquetas_exactas` (67/158, 9 personas, 8 temas — el fantasma se fue), `el_paso5_pasa_el_esquema_del_cap44` (`verificar_esquema(store, esquema_kb_lira())` → `Ok(())`) |
| **Extraer en streaming por lotes** — `lotes_de_registros(entrada: &mut dyn BufRead, tamano_lote) -> impl Iterator<Item = Result<Vec<RegistroCrudo>, ErrorIngesta>>`: UNA pasada por fichero (`n_pasadas = 1`), lote = 25 registros (~8 lotes), contadores (filas, lotes, rechazos); el lote es la frontera explícita de memoria (espejo `FronterasBfs` cap-26); `partir_csv` del cap-32 REUTILIZADO | `el_pipeline_paso5_extrae_registros_crudos_en_una_sola_pasada` (n_pasadas = 1, lotes y filas pineados), `el_pipeline_paso5_carga_por_lotes_con_contadores` |
| **Validar: write-time en dos niveles** — (a) POR LOTE: reglas de FILA derivadas del esquema (label conocida, props requeridas, tipos, formato de orcid) + clave natural repetida + `SinSolape` LOCAL entre las MEMBER_OF del lote; el registro que viola se RECHAZA con `(linea, motivo)` y el pipeline sigue (solo la cabecera inválida aborta); (b) FINAL: `verificar_esquema` como puerta de salida (`Ok` obligatorio, si no → `ErrorIngesta::EsquemaNoCumplido`) | `el_pipeline_paso5_rechaza_la_fila_sucia_con_linea_y_motivo` (4 rechazos pineados: duplicados exactos de Beto/DOC_RAG/MEMBER_OF y la solapada con motivo «solape con la 53»), `el_pipeline_paso5_nunca_aborta_por_una_fila_sucia` |
| **Mapear: columnas crudas → modelo** — `MAPEO_COLUMNAS` como DATO (`nombre_autor → Persona.nombre`, `titulo_obra → Documento.titulo`, `anio_publicacion → Documento.anio`, `tipo → :LABEL`, `tema_nombre → Tema.nombre`, `tipo_relacion/de/a → label/extremos`); la etapa resuelve NOMBRES → entidades (las aristas crudas referencian entidades por nombre; el pipeline las resuelve a ids — la «prueba de la identidad propia» del cap-41 en acción) | `el_pipeline_paso5_mapea_columnas_crudas_al_modelo` (las 4 variantes de Ana a UNA clave natural; aristas por nombre resueltas) |
| **Entity resolution** — std puro: `normalizar_nombre` (minúsculas sin tildes, tabla estática ~10 líneas; el lexer UTF-8 del cap-18 como aliado), `distancia_levenshtein` (~20 líneas, DP) + `similitud_levenshtein`, `bigramas` + `jaccard_bigramas`, `bloque_por_inicial`. Regla de arista: mismo bloque Y (`jaccard ≥ 0.5` O mismo primer token Y `jaccard ≥ 0.25`). Grafo de similitud = `MemoryStore` temporal con aristas `SIMILAR`; clusters = `componentes_conexas` del cap-25 REUTILIZADA. Contadores: naive (78 para 13 personas) vs con blocking (~11) → **evitadas ~67** | `la_normalizacion_sin_tildes_une_variantes_de_ana`, `la_distancia_de_levenshtein_distingue_ana_garcia_de_carla_mendez`, `el_blocking_por_inicial_evita_comparaciones`, `el_grafo_de_similitud_agrupa_las_cinco_anas_en_un_cluster`, `las_componentes_conexas_del_cap25_dan_los_clusters_de_entidades` |
| **Fusión con sobrescritura cuidada** — canónico = miembro con MÁS props no vacías (empate → menor línea); props iguales se conservan, ausentes se rellenan, CONFLICTO real → gana el canónico y se DECLARA en el informe `(propiedad, valor_canonico, valor_descartado, linea)` — NUNCA silencioso (el orcid malformado se descarta declarándolo); aristas reapuntadas al canónico | `la_fusion_elige_el_canonico_con_mas_datos_y_reapunta_aristas`, `la_fusion_declara_los_conflictos_sin_sobrescribir_en_silencio` |
| **HistorialIngesta (transaction-time de la carga)** — tipo NUEVO append-only, ts monótono (la forma del `HistoricoAfiliaciones` del cap-43 REPLICADA, sin tocar cap-43): `CargaIniciada/LoteValidado/RegistroRechazado/FusionEntidad/ConflictoDeclarado/CargaCompletada`; CSV con round-trip | `el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono` (6 fusiones y 4 rechazos, ts 1..n), `csv_historial_ingesta_roundtrip_byte_a_byte` |
| **Idempotencia por clave natural** — clave = `(label, nombre normalizado[, orcid])`; recargar el MISMO dataset dos veces → mismo grafo (sin duplicados, ids idénticos) | `recargar_el_mismo_csv_no_duplica_el_grafo` (dos cargas: 67/158 ambas, ids iguales, ts nuevos), `recargar_tras_anadir_un_duplicado_nuevo_lo_fusiona` |
| **El caso Tema 26/61 CURADO** — las dos filas «memoria de agentes» (mismo nombre normalizado, mismo bloque) forman un cluster; la fusión las reduce a UNA: 68 → **67 nodos**, 158 aristas (reapuntar no borra); el informe declara el delta | `la_ingesta_cura_el_duplicado_tema_26_61`, `el_informe_ingesta_declara_el_delta_de_nodos` |
| **Regresión OBLIGATORIA** — las 10 preguntas del cap-41 sobre el paso-5 IDÉNTICAS (devuelven NOMBRES: la fusión no cambia nombres, solo ids), pines 42-44 intactos, validador paso-3 acepta el paso-5, esquema `Ok`, CSVs 1-4 byte a byte | `las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta`, `las_respuestas_del_paso2_no_cambian_tras_la_ingesta`, `validador_paso3_acepta_el_modelo_paso5`, `el_paso5_pasa_el_esquema_del_cap44`, `csv_pasos_1_4_intactos_tras_la_ingesta` |
| CSV paso-5: `nodes.csv` + `edges.csv` (formato cap-32) + `historial.csv` + `informe.txt` + artefacto commiteado | `csv_paso5_roundtrip_byte_a_byte`, `csv_paso5_coincide_con_dataset_commiteado_byte_a_byte` |
| Informe reproducible `informe_ingesta_reproducible`: fichero → filas → lotes → rechazos; comparaciones naive vs bloqueadas; clusters y fusiones; conflictos; delta de nodos; veredicto del esquema; SIN tiempos | `informe_ingesta_reproducible_sobre_kb_lira` (pineado byte a byte) |
| **SIN `[[bench]]` nuevo** (espejo #12 cap-41, #11 cap-43, #9 cap-44) | `verify.sh` compila `--all-targets` igual; prosa pega salidas de `cargo test` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**993 + ~20 tests ≈ 1013**); cero cambios en caps. 7-44, goldens intactos, pasos 1-4 byte a byte |

## 3. Las preguntas críticas del capítulo y la respuesta del capítulo

**Preguntas** (propias): (1) ¿Cómo convierto CSVs crudos —sin ids, con duplicados y
suciedad— en un grafo limpio y reproducible? (2) ¿Cómo deduplico ENTIDADES sin comparar
todo contra todo y sin equivocarme al fusionar? (3) ¿Cómo garantizo que recargar el mismo
dataset no duplique el grafo y que el esquema del cap-44 se cumpla al importar?

Respuestas medibles:

1. **La ingesta es un pipeline de cuatro etapas, no una copia.** Extraer (leer filas,
   streaming por lotes) → Validar (reglas de fila por lote con número de línea; el
   esquema del cap-44 como puerta final) → Mapear (columnas crudas al vocabulario del
   modelo + nombres → entidades) → Cargar (put_node/put_edge con ids nuevos). Cada etapa
   tiene salida TIPADA y contadores; el cap-32 era la versión para ficheros limpios con
   `:ID` — su `partir_csv` sobrevive, su importer no sirve para crudos (cabecera
   obligatoria, autocommit, sin validación ni dedup). La fila que viola se RECHAZA con
   línea y motivo; el pipeline sigue; la puerta final es `verificar_esquema` → `Ok` (o
   error con violaciones, nunca un grafo que «no sabe»).
2. **Entity resolution = normalizar + bloquear + comparar + fusionar.** La normalización
   (minúsculas sin tildes) unifica «Ana García»/«ana garcia»; el blocking por inicial
   evita el O(n²): 78 comparaciones naive → ~11 con bloques (contadores); la similitud de
   bigramas (Jaccard, con refuerzo de primer token) agrupa «Ana G.» y «Ana García
   (Universidad de Lira)» donde Levenshtein falla («ana garcia» vs «ana g.» = 5 ediciones
   ≈ 0.5 de similitud: la métrica importa); el grafo de similitud es un GRAFO — sus
   componentes conexas (cap-25, reutilizado) son los clusters de entidades. La fusión no
   es «el último gana»: canónico = más datos, conflictos DECLARADOS, aristas reapuntadas.
3. **Idempotencia por clave natural + esquema como contrato.** La clave
   `(label, nombre normalizado[, orcid])` es la «prueba de la identidad propia» del
   cap-41 aplicada a la ingesta: el pipeline asigna ids nuevos y la identidad la decide
   la clave, no el fichero — recargar el mismo CSV produce el mismo grafo (test). Y el
   arco del cap-44 se cierra: el duplicado Tema 26/61 que el catálogo delató, la ingesta
   lo CURA (fusión por nombre idéntico): 68 → 67 nodos, esquema `Ok`. La regla
   `SinSolape` del cap-44, aplicada AL VUELO: la MEMBER_OF solapada `[2020,2023)` se
   rechaza con línea.

Escalera del brief (5 secciones del outline → 5 peldaños):

1. **El pipeline por etapas** → `PipelineIngesta` con salidas tipadas; el esquema del
   cap-44 como etapa VALIDAR; `MAPEO_COLUMNAS` como dato; el cap-32 como primera
   solución superada.
2. **Streaming para datasets mayores que la RAM** → `lotes_de_registros`: una pasada,
   lote = frontera explícita (25 registros); hermano de `FronterasBfs` (cap-26) y del
   streaming del cap-32.
3. **Idempotencia: recargar sin duplicar** → clave natural (upsert) como mecanismo;
   manifiesto con hash como la variante industrial (reto experto, `fnv1a_64` del cap-15).
4. **Entity resolution: blocking y grafo de similitud** → normalización, Levenshtein (el
   clásico con fecha: 1966), Jaccard de bigramas, bloque por inicial, grafo `SIMILAR`,
   componentes conexas del cap-25.
5. **Fusión de entidades y sobrescritura cuidada** → regla de fusión explícita,
   conflictos declarados, re-apuntado de aristas, `HistorialIngesta` alimentado (el
   transaction-time del cap-43 cobrado), el caso Tema 26/61 curado.

Hilo conductor: **«el cap-32 importaba; este capítulo INGIERE: extrae, valida, mapea,
resuelve entidades y fusiona — con contadores en cada etapa y el esquema del cap-44 como
contrato de salida. El lote de mañana puede venir sucio: el pipeline es quien lo limpia, y
recargarlo dos veces ya no asusta a nadie»**.

---

## 4. La arquitectura: el modelo mental de la fábrica

Modelo mental único: **la ingesta es una FÁBRICA con cuatro estaciones; el esquema del
cap-44 es el control de calidad; el entity resolution es el control de duplicados que
fusiona ANTES de que la basura llegue al grafo**. La figura que ordena todo el capítulo:

```text
LA FÁBRICA (el pipeline por etapas — cada etapa devuelve su TIPO, nunca el vacío):
  CRUDOS                VALIDAR               MAPEAR                CARGAR
  (sin ids, sucio)      (esquema como         (columnas crudas →   (put_node/put_edge,
                        control de calidad)   modelo; nombres →    ids NUEVOS)
                                               entidades)
  personas_crudas.csv ──► reglas de fila ──► MAPEO_COLUMNAS ──► 67 nodos / 158 aristas
  documentos_crudos.csv  por LOTE (línea,    nombre_autor →      + HistorialIngesta
  temas_crudos.csv       motivo)             titulo_obra →       (append-only, ts 1..n)
  relaciones_crudas.csv  │                   resolución de
                         ▼                   nombres (clave
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

LA MONEDA (nunca µs): filas leídas · lotes formados · rechazos con línea ·
  comparaciones naive 78 vs bloqueadas ~11 (evitadas ~67) · clusters · fusiones (6) ·
  conflictos declarados · nodos/aristas exactos
```

Y debajo, la REGLA DE ORO heredada (determinismo total, cap. 34): el paso-5 es un
artefacto DERIVADO del paso-4 (la basura deliberada se genera de forma determinista, los
ids los decide el pipeline por orden de carga, cada contador pineado); las 10 preguntas
del cap-41 + los pines del cap-42 + el validador paso-3 + el esquema del cap-44 son la
red de seguridad: si la ingesta cambia una respuesta vieja sobre los subgrafos 1-3, el
cambio está MAL. La frontera, declarada antes de codificar: `cap45_ingesta.rs` es
aditivo; ni `GraphStore` ni `cap32/41/42/43/44` se tocan; el pipeline escribe SOLO por la
API pública; el esquema del cap-44 NO se modifica (la Unicidad sobre `Tema.nombre` se
NOMBRA como decisión posible y se descarta: el paso-4 no la cumple y la ingesta no cambia
el contrato, lo cumple).

Momento ¡ajá! perseguido: **«el grafo de similitud es un grafo: las mismas componentes
conexas del cap-25 que agrupaban comunidades en redes, ahora agrupan DUPLICADOS — y
"cargar dos veces el mismo fichero" no es un susto: es el test más barato del pipeline,
y pasa siempre»**.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Dataset crudo PROPIO en `datasets/kb-lira/paso-5/crudos/` (4 ficheros SIN ids, columnas renombradas, suciedad deliberada: 3 duplicados exactos, 4 variantes de Ana, 1 typo, el fantasma 26/61, 1 MEMBER_OF solapada) — generado por `dataset_crudo_kb_lira_paso5()` y commiteado | El paso-5 parte de datos CRUDOS: la suciedad es CONTENIDO (sin ella no hay validación ni dedup que mostrar); sin `:ID` el cap-32 falla en la cabecera y se demuestra que el pipeline asigna ids NUEVOS — la conexión con el cap-41 («la identidad es decisión del modelo, no del fichero»); los nombres de columna distintos del modelo hacen el MAPEO real | (a) Reutilizar los CSVs limpios del paso-4: sin duplicados, sin mapeo, sin rechazos — un round-trip vacío; (b) dataset con ids: contradice «la ingesta asigna identidad» | OUTLINE-VOL3.yml cap-45 (paso-5: «CSVs crudos con duplicados»); contrato cap-41 (identidad); cap44 (selectividad delata 26/61) |
| 2 | Pipeline por etapas `PipelineIngesta { extraer, validar, mapear, cargar }` con salidas tipadas; el esquema del cap-44 como etapa VALIDAR: reglas de FILA por lote (fail-fast con línea) + `verificar_esquema` como puerta FINAL | El esquema valida GRAFOS completos (`Extremos`, `SinSolape`, `IntervaloValido` necesitan el grafo entero); un lote aislado SIEMPRE viola extremos — por eso las reglas de fila (derivadas del esquema) van por lote y el esquema entero cierra la puerta. El registro que viola se RECHAZA con `(linea, motivo)` y el pipeline sigue | (a) `verificar_esquema` por lote: falsos positivos de grafo parcial; (b) validar solo al final: la fila 9000 rompe después de cargar 8999; (c) abortar el lote ante una fila sucia: el rechazo selectivo con línea es la lección (y el histórico lo registra) | cap44_esquema.rs:163; manuscrito cap-44 §44.9.1 («sin validación al importar… es el cap. 45» — el gancho literal); cap43:1077 (siembra, ahora reglas de fila) |
| 3 | Streaming por LOTES: `lotes_de_registros` (iterator, UNA pasada por fichero, lote = 25, contadores); `partir_csv` del cap-32 REUTILIZADO; `texto_a_valor` (privada) replicada local en 10 líneas | El cap-32 YA es streaming (verificado: `read_line` + autocommit por fila, cap32:249) pero sin frontera de lote: el pipeline introduce la frontera explícita «lote = N registros» (la unidad residente máxima es el lote, declarable con contadores sin medir memoria); espejo de `FronterasBfs` del cap-26 (spacing directo); la frontera permite el fail-fast por lote de la decisión #2 | (a) Reutilizar `importar_csv_nodos`: exige `:ID` y autocommits (en crudos: `CabeceraInvalida` o `DuplicateNode`) — sirve de PRIMERA SOLUCIÓN (N.4), no de pipeline; (b) carga completa en RAM: contradice «datasets mayores que la RAM» | cap32:180 (`partir_csv` pública), :249 (streaming), :363 (`texto_a_valor` privada); cap26:800-925 (`FronterasBfs`) |
| 4 | Idempotencia por CLAVE NATURAL (upsert): clave = `(label, nombre normalizado[, orcid])`; recargar el mismo CSV → mismo grafo. El manifiesto (hash + skip) queda como reto experto | La clave natural es la «prueba de la identidad propia» del cap-41 aplicada a la ingesta: la identidad NO viene del fichero (ids nuevos) sino de la clave — la lección es de MODELADO; el manifiesto (hash + skip) es la variante industrial pero añade estado externo (un fichero de manifiesto que gestionar) sin lección nueva | (a) Manifiesto como mecanismo principal: el lector no ve la fusión (el skip evita procesar) y la superficie crece; (b) sin idempotencia: la recarga duplica el grafo — el test más barato no existiría | contrato cap-41 (identidad propia); cap15:226 (`fnv1a_64` para el reto); Neo4j admin import (no idempotente por diseño — el manifiesto es del equipo, gancho N.10) |
| 5 | Entity resolution: normalización (minúsculas sin tildes) → bloque por INICIAL → comparación en bloque con `jaccard_bigramas` (umbral 0.5, o mismo primer token con jaccard ≥ 0.25) → grafo de similitud (MemoryStore temporal, aristas `SIMILAR`) → clusters con `componentes_conexas` del cap-25 REUTILIZADA; Levenshtein a mano (~20 líneas) como métrica de CONTRASTE; contadores (naive 78 vs ~11 → evitadas ~67) | La métrica compuesta resuelve los TRES casos con UNA regla: idéntico tras normalizar (jaccard 1.0: el fantasma 26/61), abreviatura/paréntesis («ana garcia» vs «ana g.»: jaccard 0.56; vs «Ana García (Universidad de Lira)»: primer token igual + 0.47 ≥ 0.25) y no-duplicados («ana garcia» vs «carla mendez»: 0.11 — sin arista). Levenshtein es el clásico (1966) y FALLA en abreviaturas (≈0.5): la tabla de contraste enseña que la métrica es una DECISIÓN. El grafo de similitud como grafo + componentes conexas = clusters: el cap-25 reutilizado literalmente (BFS del Vol.I caps. 3-4 como espaciado) | (a) Todo-contra-todo: las 78 comparaciones medidas y evitadas — el blocking se demuestra, no se afirma; (b) Levenshtein como única métrica: pierde abreviaturas y paréntesis (se enseña POR QUÉ); (c) crates de similitud: regla «primero a mano» de CONVENTIONS §4 | Christen 2012 (Springer, agosto 2012 — blocking y similitud, VERIFICADA); Levenshtein 1966 (Soviet Physics—Doklady 10(8):707-710, VERIFICADA); cap25:696 (`componentes_conexas`); CONVENTIONS §4 |
| 6 | Fusión con sobrescritura CUIDADA: canónico = miembro con MÁS props no vacías (empate → menor línea); iguales se conservan, ausentes se rellenan, conflicto real → gana el canónico y se DECLARA (`propiedad, valor_canonico, valor_descartado, linea`); aristas reapuntadas | «Sobrescritura cuidada» = nada se sobrescribe en silencio: el pipeline NUNCA borra un valor del canónico sin declararlo; el orcid malformado de la variante de Ana se descarta con motivo (el canónico tiene el bien formado — la fusión salva la `Unicidad` que la fila suelta habría violado); el re-apuntado preserva las 158 aristas (reapuntar no borra) | (a) «El último gana»: destructivo y arbitrario — conflicto oculto; (b) «El primero gana» sin informe: la sobrescritura silenciosa es el antipatrón que el capítulo denuncia; (c) canónico por orden alfabético: «el más datos» es la regla que un humano elegiría | Christen 2012 (merge policies); decisión #5 (reglas, no magia) |
| 7 | `HistorialIngesta`: tipo NUEVO append-only, ts monótono (la forma del `HistoricoAfiliaciones` del cap-43 REPLICADA); eventos `CargaIniciada/LoteValidado/RegistroRechazado/FusionEntidad/ConflictoDeclarado/CargaCompletada`; la fusión y el rechazo REGISTRAN su evento; CSV con round-trip | La deuda del cap-43 cobrada: «el transaction-time que la ingesta debe alimentar automáticamente» — la ingesta escribe SU WAL del modelo (append-only, el ts lo decide el histórico, nunca el llamador); NO se extiende `EntradaHistoria` porque sus campos (persona, organizacion, desde_anio) no encajan con eventos de carga — regla dura: cap-43 intacto | (a) Extender `EntradaHistoria`/`HistoricoAfiliaciones`: tocar cap-43 (prohibido) y forzar campos de afiliación en eventos de carga; (b) sin historial: la fusión sería un borrado de huellas — el transaction-time exige el registro | cap43:798-868 (la forma replicada: `registrar` asigna ts), cap43:1565 (`csv_historico` — la forma del CSV) |
| 8 | Citas (verificadas hoy 2026-08-26): Christen 2012 (Springer, agosto 2012); Swoosh — Benjelloun et al., VLDB Journal 18(1):255-276, 2009; Levenshtein 1966 (Soviet Physics—Doklady 10(8):707-710); GIGO — George Fuechsel, IBM, ~1957 [VERIFICAR fino]; Kimball 1996 (ya citado); Neo4j `neo4j-admin database import` (Operations Manual actual, [VERIFICAR cabecera fina]); recordlinkage 0.15 y dedupe 3.0.2 (nombrados en N.10) | Christen es el libro de referencia de ER (blocking, normalización, métricas, fusión) — la fuente de las 3 secciones de entity resolution; Swoosh da la teoría del grafo de similitud (match/merge como cajas negras, propiedades ICAR: «el resultado es una hipótesis, no un veredicto»); Levenshtein con fecha exacta (el clásico implementado a mano); GIGO es la anécdota del N.0; Neo4j admin import es el «cómo lo hace una BBDD real» (importación masiva offline — el cap-32 ya copió su estilo de cabecera) | Citar libros/versiones sin verificar: riesgo de venue/año inventado — todas las de arriba verificadas con fuentes en vivo; Fellegi-Sunter (1959) solo NOMBRADO, sin citar detalle | Verificación puntual de hoy (fuentes primarias en vivo: Springer, springer.com/article/10.1007/s00778-008-0098-x, ADS/SemanticScholar, neo4j.com/docs/operations-manual/current/import/, recordlinkage.readthedocs.io, docs.dedupe.io) |
| 9 | Alcance de código: UN módulo `cap45_ingesta.rs` (~900-1300 líneas, std puro, ~20 tests) + wiring aditivo (2 líneas en `lib.rs`) + artefactos `datasets/kb-lira/paso-5/`; **SIN bench** | El módulo accede GRATIS a `partir_csv` (cap-32), `Violacion` (cap-41), `verificar_esquema`+`esquema_kb_lira` (cap-44), `componentes_conexas` (cap-25) y a los builders 1-4 (regresión); la moneda son filas, lotes, rechazos, comparaciones evitadas, fusiones y conjuntos exactos — nunca µs (espejo #12 cap-41, #11 cap-43, #9 cap-44) | (a) Crate nueva: churn sin ganancia (precedente caps. 38-44); (b) primer bench del Vol.III: cronometrar no sostiene ninguna tesis — el streaming se demuestra con la frontera de lote y los contadores | CONVENTIONS §2 y §4; contrato cap-44 decisión #9 (espejo) |
| 10 | Fronteras duras: cap-46 RDF (la ingesta de tripletas es del 46); cap-47 SHACL (nombrado); cap-52 (el LLM como extractor no determinista sustituiría la etapa extraer — nombrado); validadores 41-44 INTACTOS (regresión cuádruple); el pipeline NO toca `GraphStore` (solo API pública); el esquema del cap-44 NO se modifica | El pipeline sin frontera se convertiría en el cap-46 o en un validador que duplique el cap-44; la regresión dura garantiza que limpiar datos no cambia NINGUNA respuesta vieja; escribir por la API pública mantiene la capa externa limpia (patrón hexagonal del proyecto) | (a) Ingesta RDF aquí: fuera del outline (el paso-6 es la exportación N-Triples del 46); (b) añadir `Unicidad` a `Tema.nombre`: el paso-4 NO la cumple (8/9) — el pipeline no cambia el contrato, lo CUMPLE | OUTLINE-VOL3.yml caps. 45-47; manuscrito cap-44 §44.9 (frontera write-time); contrato cap-44 (StoreConEsquema descartado «es el cap-45») |
| 11 | El caso Tema 26/61: la ingesta lo CURA (fusión por nombre normalizado idéntico, jaccard 1.0) — 68 → 67 nodos, 158 aristas (reapuntar no borra), `Tema.nombre` 8/8 = 1.0; el informe declara el delta con su motivo; si al implementar el fantasma 26 tuviera aristas, la fusión las reapunta y el conteo se ajusta con su porqué — prohibido maquillar | Es EL caso que cierra el arco del cap-44: el catálogo delató (8/9 ≈ 0.889), la ingesta cura — el pipeline hace visible la deuda y la cobra; los validadores 1-4 nunca lo vieron (Existencia exige que el nombre ESTÉ, no que sea único) y la ingesta sí: el entity resolution no sabe de esquema, sabe de NOMBRES | (a) Dejarlo sin tocar: el arco quedaría abierto y el paso-5 arrastraría el fantasma; (b) arreglarlo en el esquema (Unicidad en Tema.nombre): tocaría el cap-44 y el paso-4 no pasaría su propio esquema — la decisión correcta es la fusión en la ingesta | cap44:1461 (`selectividad_de_propiedad` — el 8/9); manuscrito cap-44 §44.15 (la historia del fantasma) |
| 12 | Regresión OBLIGATORIA: `las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta` (devuelven NOMBRES: la fusión no cambia nombres, solo ids — el fantasma no tenía aristas, por eso era invisible), `las_respuestas_del_paso2_no_cambian_tras_la_ingesta`, `validador_paso3_acepta_el_modelo_paso5`, `el_paso5_pasa_el_esquema_del_cap44`, `csv_pasos_1_4_intactos_tras_la_ingesta` | La red de seguridad cuádruple heredada: si la ingesta cambia una respuesta vieja, el cambio está MAL; el paso-5 es un artefacto DERIVADO del paso-4 y se verifica contra TODOS los pines históricos (espejo del cap-44, que verificó contra 41-43) | (a) Menos regresión: rompería la cadena de confianza del volumen; (b) ajustar una pregunta para que cuadre: prohibido por la regla de oro | contratos cap-41 a cap-44 (la cadena); cap41:726-1000 (`pregunta_01…10`, devuelven `Vec<String>`) |

## 6. Estructura del manuscrito (partes y tempos)

1. **Blockquote inicial OBLIGATORIO**: QUINTO capítulo del Vol.III y CIERRE DE LA PARTE
   I, audiencia (lector del cap-44 + perfil IA/datos), conexión con caps. 41-44 por
   referencias, ganchos cobrados literalmente: «nadie valida al importar» → write-time
   por lote + puerta final; la deuda del cap-43 → `HistorialIngesta`; el fantasma 26/61
   → curado (67 nodos).
2. **Apertura (N.0, anécdota + pregunta crítica)**: «garbage in, garbage out» (GIGO) —
   George Fuechsel, programador y profesor de IBM, ~1957, la era de las tarjetas
   perforadas: el ordenador devuelve lo que le das y la basura no se detecta sola.
   Pregunta enmarcada: tu KB-Lira acepta el lote de mañana tal cual — ¿a qué precio?
   (eco del cap-44: el cajón del taller, ahora la cinta de entrada).
3. **N.1-N.2 Objetivo/Problema**: objetivo medible del outline («construir un pipeline
   reproducible de carga (CSV/JSONL) con validación y deduplicación»). Problema: el
   cap-32 importa ficheros limpios con `:ID`; los datos reales llegan sin ids, con
   duplicados y suciedad; el esquema del cap-44 existe y NADIE lo aplica al importar (el
   gancho); el grafo del paso-4 arrastra el fantasma 26/61. Desactivar las SIETE
   misconcepciones ANTES de dibujar.
4. **N.3 Modelo mental**: la FÁBRICA (cuatro estaciones; el esquema como control de
   calidad; el entity resolution como control de duplicados que fusiona antes de cargar);
   el diagrama ASCII del §4; la regla de fusión y la moneda (contadores, nunca µs).
5. **N.4 Primera solución**: `importar_csv_nodos` del cap-32 DIRECTAMENTE sobre los
   crudos — (a) sin `:ID`: `CabeceraInvalida`; (b) «arreglado» añadiendo ids a mano: los
   duplicados entran (autocommit por fila) y el grafo queda con dos Anas y dos «memoria
   de agentes»; (c) la fila solapada entra y el esquema, verificado después, denuncia la
   violación — tarde. Tres modos de fallo ANTES de la solución.
6. **N.5 Sus límites**: el importer del cap-32 no conoce el esquema (no valida al
   importar), no distingue entidades (dos Anas indistinguibles sin clave natural), no
   agrupa duplicados (todo-contra-todo no escala: 78 comparaciones solo para 13
   personas) y no recuerda (recargar duplica). La promesa del cap-44 sigue sin responder.
7. **N.6 Solución evolucionada**: `PipelineIngesta` (4 etapas tipadas) +
   `lotes_de_registros` (una pasada, frontera de lote) + validación por lote con rechazo
   `(linea, motivo)` + `verificar_esquema` final + `MAPEO_COLUMNAS` + entity resolution
   (normalización → blocking → jaccard → grafo de similitud → componentes conexas del
   cap-25) + fusión con conflictos declarados + `HistorialIngesta` + idempotencia por
   clave natural + el caso Tema 26/61 curado + la REGLA DE ORO.
8. **N.7 Código completo ejecutable**: `cap45_ingesta.rs` por `include::`; SIN bench
   (decisión #9 en una línea).
9. **N.8 Prueba de fuego**: salidas REALES de `cargo test`: los 4 rechazos con línea y
   motivo, las 6 fusiones con ts del histórico, las comparaciones 78 → 11 (evitadas 67),
   el delta 68 → 67, el esquema `Ok`, la regresión cuádruple, los CSVs byte a byte.
10. **N.9 Qué hemos sacrificado**: sin ingesta RDF (cap. 46); sin SHACL (caps. 46-47,
    nombrado como gancho); sin ER probabilístico (Fellegi-Sunter nombrado; aquí la
    similitud es determinista y explicable); sin manifiesto industrial (reto experto con
    `fnv1a_64` del cap-15); sin streaming distribuido (Spark/Dataflow nombrados); el
    pipeline vive en RAM (la versión persistente del store es el cap-15/28, nombrada).
11. **N.10 Cómo lo hace una BBDD real + retos**: **Neo4j** — `neo4j-admin database
    import` (importación masiva offline, cabeceras `:ID`/`:LABEL`/`:START_ID`/`:END_ID`/
    `:TYPE`, [VERIFICAR fino]) y `LOAD CSV` (streaming línea a línea desde Cypher);
    **Python** — recordlinkage 0.15 (indexing por bloques, comparadores, umbral) y
    dedupe 3.0.2 (ML con aprendizaje activo); **Swoosh (VLDB J. 18:255-276, 2009)** —
    match/merge como cajas negras y las propiedades ICAR: el grafo de similitud de aquí
    es su versión mínima; **Christen 2012** como el libro de referencia. Retos: esencial
    (45+41+44): PREDECIR por escrito los contadores del pipeline ANTES de correr
    (rechazos, fusiones, comparaciones evitadas, nodos/aristas finales) y justificar por
    qué las 10 preguntas no cambian; intermedio (45+32, 45+15): comparar el pipeline con
    `importar_csv_nodos` del cap-32 (tabla de qué cubre y qué no) y con el
    `BPlusTree`/`fnv1a_64` del cap-15; y el extraer-JSONL reutilizando `parsear_json`
    del cap-32 (el MISMO pipeline, otra etapa extraer); experto (45+43+28): el manifiesto
    de carga real (hash con `fnv1a_64` + skip si ya cargado) Y alimentar el
    `HistoricoAfiliaciones` del cap-43 desde la ingesta (cada MEMBER_OF importada se
    registra; la fusión re-registra bajo el canónico) — «la ingesta escribe el WAL del
    modelo».
12. **Baterías finales + gancho**: Lo que te llevas / Ojo cuidado / Pin / 30 segundos /
    historia pequeña (la continuación del cap-44: el fantasma 26/61 que el catálogo
    delató, ahora curado — «delatar no es curar») / Mini-diálogo de guardia nocturna (la
    operadora recarga el CSV y aparecen DOS Anas: «¿cuál es la verdadera?» — el orcid que
    nadie declaró único, fusionado por el pipeline: eco del cap-44). Retrieval practice:
    recitar DE MEMORIA las 4 etapas, la regla de fusión y el porqué del blocking (78 →
    11), y clasificar 5 modos de fallo («¿lo rechaza la validación, lo fusiona el ER, o
    lo carga tal cual?»). Spacing: cap-32 (`partir_csv`), cap-25 (`componentes_conexas`),
    cap-26 (`FronterasBfs`), cap-41 (identidad propia, 10 preguntas), cap-44 (esquema,
    `SinSolape`, el fantasma), cap-43 (transaction-time → `HistorialIngesta`), cap-15
    (`fnv1a_64`), cap-18 (lexer UTF-8), cap-34 (determinismo). Interleaving: 45+41+44
    (esencial), 45+32+15 (intermedio), 45+43+28 (experto). Glosario nuevo: pipeline,
    etapa, extracción, validación al importar (write-time), mapeo, carga, lote, streaming,
    frontera de lote, idempotencia, clave natural, upsert, manifiesto de carga, hash,
    entity resolution, deduplicación, normalización, blocking, clave de bloque, distancia
    de Levenshtein, bigramas, similitud de Jaccard, grafo de similitud, umbral, cluster,
    canónico, fusión, re-apuntado, sobrescritura cuidada, conflicto declarado, GIGO.
    Gancho al cap. 46: «tu pipeline entiende CSVs y JSONL — ¿y si los datos llegan como
    tripletas RDF? La Parte I se cierra aquí: la Parte II cambia de filosofía, de
    propiedades a tripletas». Abiertas: SHACL (47), extracción con LLM (52), memoria del
    agente (53).

---

## 7. Estilo y tono (consistencia con el proyecto)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez (pipeline, stage, streaming, batch, idempotency, natural key,
  upsert, manifest, blocking, similarity, threshold, cluster, canonical, merge, write-time,
  GIGO); salidas REALES de `cargo test` pegadas, nunca reconstruidas; las decisiones se
  presentan como TRADE-OFF con contadores (comparaciones evitadas, rechazos, fusiones),
  nunca como dogma.
- **Diagramas**: la fábrica del §4 (4 estaciones con sus tipos); el grafo de similitud de
  las Anas (5 nodos, aristas `SIMILAR`, 1 cluster); la línea de Beto con la MEMBER_OF
  solapada `[2020,2023)` rechazada (la `SinSolape` del cap-44 al vuelo); la tabla de
  contadores del informe como figura recurrente.
- **Spacing** (conceptos viejos que se EJERCITAN): `partir_csv` del cap-32 (reutilizado),
  `componentes_conexas` del cap-25 (reutilizada — el BFS del Vol.I caps. 3-4 de nuevo),
  `FronterasBfs`/`Presupuesto` del cap-26 (la frontera de lote), la clave natural del
  cap-41, `verificar_esquema` + `SinSolape` del cap-44 (write-time), el transaction-time
  del cap-43 (`HistorialIngesta`), `fnv1a_64` del cap-15 (reto experto), el lexer UTF-8
  del cap-18 (normalización), el determinismo del cap-34.
- **Interleaving**: el reto esencial mezcla 41+44+45 (clave natural + esquema +
  contadores); el intermedio mezcla 32+45 y 15+45; el experto mezcla 28+43+45.
- **Dificultad asimétrica**: una idea nueva por sección (pipeline → streaming →
  idempotencia → entity resolution → fusión); los ejercicios exigen PREDECIR contadores
  y DECIDIR métricas/umbrales ANTES de correr los tests.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap45` (contadores, conjuntos y
  rechazos exactos) y `./scripts/verify.sh` ALL_GREEN como puerta. Nunca «confía en mí».
- **Anécdota (única, verificada)**: GIGO — George Fuechsel (IBM, ~1957) [VERIFICAR fino].
  Fuentes para la prosa: Christen 2012; Swoosh (VLDB J. 18:255-276, 2009); Levenshtein
  1966; Kimball 1996 (ya citado); Neo4j admin import [VERIFICAR cabecera fina];
  recordlinkage 0.15 y dedupe 3.0.2 (nombrados).

---

## 8. Riesgos e interrupciones del generador

- **El módulo es ADITIVO**: hasta que `lib.rs` no declare `mod cap45_ingesta; pub use
  cap45_ingesta::*;`, NADA del workspace puede romperse. Wiring SIEMPRE al final, con el
  módulo ya compilando limpio; jamás dejar `lib.rs` apuntando a un módulo rojo.
  `kb_lira_paso5()` NO toca `cap32/41/42/43/44`: llama a la API pública de cada uno
  (`partir_csv`, `Violacion`, `verificar_esquema`+`esquema_kb_lira`, `componentes_conexas`,
  `pregunta_01…10`, `validar_modelo_kb_lira_paso3`) y a los builders 1-4 solo en tests de
  regresión; los ids del paso-5 los asigna el pipeline (son OTRO grafo con el mismo
  contenido — la comparación es por NOMBRES, nunca por ids).
- **Orden de implementación recomendado (PATRÓN DE TROCEO — cada pieza compila y testea
  SOLA: 1 función + 1 test, el agente de código no sobrevive a tareas largas)**:
  (1) tipos crudos (`RegistroPersonaCrudo`/`RegistroDocumentoCrudo`/`RegistroTemaCrudo`/
  `RegistroRelacionCruda` + `DatasetCrudo` + `ErrorIngesta`) + `leer_fila_cruda` +
  `el_pipeline_paso5_extrae_registros_crudos_en_una_sola_pasada`; (2) `normalizar_nombre`
  + `la_normalizacion_sin_tildes_une_variantes_de_ana`; (3) `distancia_levenshtein` +
  `similitud_levenshtein` + `la_distancia_de_levenshtein_distingue_ana_garcia_de_carla_mendez`;
  (4) `bigramas` + `jaccard_bigramas` +
  `el_jaccard_de_bigramas_separa_ana_g_de_carla_mendez`; (5) `bloque_por_inicial` +
  `el_blocking_por_inicial_evita_comparaciones` (78 → ~11); (6) `grafo_de_similitud` +
  `el_grafo_de_similitud_agrupa_las_cinco_anas_en_un_cluster`; (7) `resolver_clusters`
  (REUTILIZA `componentes_conexas` del cap-25) +
  `las_componentes_conexas_del_cap25_dan_los_clusters_de_entidades`; (8) `regla_de_fusion`
  + `la_fusion_declara_los_conflictos_sin_sobrescribir_en_silencio`; (9) `fusionar_clusteres`
  (re-apuntado) + `la_fusion_elige_el_canonico_con_mas_datos_y_reapunta_aristas`;
  (10) `validar_lote` (reglas de fila + clave repetida + `SinSolape` local + rechazo con
  línea) + `el_pipeline_paso5_rechaza_la_fila_sucia_con_linea_y_motivo`; (11) `mapear`
  (`MAPEO_COLUMNAS` + resolución nombre→entidad) +
  `el_pipeline_paso5_mapea_columnas_crudas_al_modelo`; (12) `lotes_de_registros` (streaming,
  una pasada, lote = 25) + `el_pipeline_paso5_carga_por_lotes_con_contadores`;
  (13) `HistorialIngesta` + `registrar` +
  `el_historial_ingesta_registra_fusion_y_rechazo_con_ts_monotono`; (14) `cargar`
  (put_node/put_edge, ids nuevos) + `recargar_el_mismo_csv_no_duplica_el_grafo`;
  (15) `kb_lira_paso5()` completo + `la_ingesta_cura_el_duplicado_tema_26_61` +
  `el_paso5_pasa_el_esquema_del_cap44`; (16) regresión cuádruple
  (`las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta`,
  `las_respuestas_del_paso2_no_cambian_tras_la_ingesta`,
  `validador_paso3_acepta_el_modelo_paso5`, `csv_pasos_1_4_intactos_tras_la_ingesta` —
  ANTES verificar las firmas `pregunta_01…10`, `validar_modelo_kb_lira_paso3` y los
  exportadores CSV tal cual); (17) CSV paso-5 + historial.csv + `informe_ingesta_reproducible`
  + generar y commitear `datasets/kb-lira/paso-5/` + wiring en `lib.rs`.
- **Estado parcial tolerable**: si el generador se interrumpe, el daño queda AISLADO —
  `cargo test -p vol2-liradb --lib cap45` señala qué piezas faltan; el resto sigue
  ALL_GREEN. Retomar: releer §2, greppear qué tests ya existen en `cap45_ingesta.rs` y
  continuar por el primer nombre ausente en la tabla.
- **Señal de corte clara**: `./scripts/verify.sh` en ROJO ⇒ o el módulo no compila (falta
  un paso) o el wiring se adelantó (deshacer wiring, no parchear a ciegas). PROHIBIDO
  tocar `cap32_import_export.rs`, `cap41_modelado.rs`, `cap42_antipatrones.rs`,
  `cap43_temporalidad.rs`, `cap44_esquema.rs`, el parser de LiraQL o el trait
  `GraphStore`: el pipeline se hace CON la API existente.
- **Criterio de parada honesto**: si la ingesta cambia la respuesta de alguna de las 10
  preguntas sobre el subgrafo paso-1 (o los pines del paso-2, o la aceptación del paso-3,
  o el `Ok` del esquema del cap-44), el cambio está MAL y se rediseña DENTRO del
  capítulo — prohibido «ajustar» la pregunta, el dataset o el esquema para que cuadre.
  Igual con los contadores: 78 → 11 comparaciones, 4 rechazos, 6 fusiones, 68 → 67 nodos
  y 158 aristas son los pines de la tesis — si el conteo real midiera otra cosa, se
  explica POR QUÉ (precedente cap-43: el contrato predecía 13→14 y la prosa pineó el
  ledger real 21→20; cap-44: 168→18 se pineó 12→3), prohibido maquillar contadores. La
  convención de intervalo es `[desde, hasta)` (heredada del cap-43): la MEMBER_OF
  solapada `[2020,2023)` solapa con la 53 `[2018,2024)`; la 185 `[2024,∞)` no — respetar
  en la validación local. La identidad de la regresión es por NOMBRES (las preguntas
  devuelven `Vec<String>`), NUNCA por ids del paso-5.

---

## Checklist de profundidad (antes de marcar DONE)

- [ ] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12
  filas en §5; citas verificadas 2026-08-26: Christen 2012 (Springer, agosto 2012),
  Swoosh (VLDB Journal 18(1):255-276, 2009), Levenshtein (Soviet Physics—Doklady
  10(8):707-710, 1966), GIGO/Fuechsel ~1957 [VERIFICAR fino], Kimball 1996 (ya citado),
  Neo4j admin import [VERIFICAR cabecera fina], recordlinkage 0.15 y dedupe 3.0.2
  (nombrados); RDF/quads fuera).
- [ ] Escenario de fallo visible, no solo happy path: los 4 rechazos con línea y motivo
  (duplicados exactos y la MEMBER_OF solapada `[2020,2023)`), el cap-32 fallando en la
  cabecera cruda, la recarga duplicando SIN idempotencia, la fusión con conflicto
  declarado (orcid malformado), el fantasma 26/61 antes de la cura.
- [ ] Código ejecutable citado por nombre (`cap45_ingesta.rs`, wiring aditivo, artefacto
  `datasets/kb-lira/paso-5/`, SIN `[[bench]]`); prosa vía `include::`.
- [ ] Misconcepciones corregidas explícitamente (§1: siete).
- [ ] Ejercicios con solución verificable (retos N.10 con predicción previa como patrón;
  esencial: contadores + clave natural; intermedio: 32+45, 15+45 y extraer-JSONL;
  experto: manifiesto con `fnv1a_64` + alimentar el `HistoricoAfiliaciones` del cap-43).
- [ ] ≥1 ejercicio de retrieval (las 4 etapas y la regla de fusión DE MEMORIA + clasificar
  5 modos de fallo) y spacing planificado (caps. 32/25/26/41/43/44/15/18/34; §7).
- [ ] Responde las TRES preguntas críticas del capítulo (pipeline por etapas con el
  esquema como validar; entity resolution con blocking + similitud + grafo + fusión
  cuidada; idempotencia por clave natural) y cobra los ganchos del cap-44 («nadie valida
  al importar» → write-time + puerta final), del cap-43 (transaction-time →
  `HistorialIngesta`) y del cap-44 otra vez (el fantasma 26/61 → curado).
- [ ] Red de seguridad CUÁDRUPLE con tests de nombre exacto:
  `las_10_preguntas_del_paso1_no_cambian_tras_la_ingesta`,
  `las_respuestas_del_paso2_no_cambian_tras_la_ingesta`,
  `validador_paso3_acepta_el_modelo_paso5`, `el_paso5_pasa_el_esquema_del_cap44`.
- [ ] Anécdota única verificada (GIGO — Fuechsel, IBM, ~1957; [VERIFICAR fino]).
- [ ] Alcance acotado y honesto (UN módulo + wiring + artefacto paso-5; cero deps, cero
  benches, cero cambios caps. 7-44; frontera dura con caps. 46-47 y 52 declarada; el
  pipeline escribe SOLO por la API pública de `GraphStore`; el esquema del cap-44 no se
  modifica).
- [ ] Blockquote inicial declara QUINTO CAPÍTULO DEL VOL.III y CIERRE DE LA PARTE I
  (audiencia + conexión con caps. 41-44 por referencias) y ganchos cobrados literalmente
  (el write-time del cap-44, el transaction-time del cap-43, el duplicado Tema 26/61);
  gancho saliente fijado (cap. 46: RDF; cap. 47: SHACL; cap. 52: el LLM como extractor).
