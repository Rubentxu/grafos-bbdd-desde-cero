# CONTRATO DE CAPÍTULO — Vol.II Cap. 32: Importación y exportación (CSV, JSONL, GraphML)

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap32_import_export.rs` (~2.700
> líneas con `ImportError`, `EstadisticasImport`, `TipoColumna`, `CabeceraNodos`,
> `CabeceraAristas`, parsers CSV/JSON/XML a mano, `importar_csv_nodos`,
> `importar_csv_aristas`, `importar_csv_unico` con separador `# aristas`,
> `importar_jsonl`, `importar_graphml`, los exporters equivalentes y
> **31 tests** propios en `mod tests_import_export`) y la CLI
> `crates/vol2-liradb-cli/src/lib.rs` con los subcomandos `import FICHERO
> -f FMT --graph ORIG` y `export FICHERO -f FMT --graph ORIG` (+280 líneas,
> 6 tests nuevos que elevan la CLI de 34 a 40 tests). Verificado ALL_GREEN:
> 728 → 748 tests workspace. Decisiones/bugs reales:
> `MIGRATION-PATTERN.md` **§37** (separador `# aristas` para roundtrip,
> lints nuevos clippy 1.96 `single_char_add_str`/`for_kv_map`/
> `collapsible_if` con let chains, `use std::io::Read` para `.chain()`,
> el campo `errores` que no existe en `EstadisticasImport`).
> Pregunta crítica del CORPUS (`vol-II-cap-32`): «¿cómo entrar y sacar
> grafos sin atragantarse con la RAM?». Este capítulo es el cap. 2 de la
> Parte VII (convertir el proyecto en un producto técnico). Gancho:
> cap. 31 (la CLI con REPL/script donde se enchufan estos subcomandos).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el `Value` schemaless del cap. 7 (con
  `Value::Bytes` que no tiene representación CSV); el trait `GraphStore`
  hexagonal del cap. 8 (MemoryStore + operaciones `put_node`/`put_edge`/
  `delete_*` y `iter_nodes`/`iter_edges`); el autocommit del cap. 27
  (cada `Operacion` se aplica directamente, sin `Transaccion` envolvente);
  la CLI del cap. 31 (clap Builder, `run_con_entrada`, REPL/script con
  `sesion::interpretar_linea`, `-` lee stdin/escribe a stdout,
  `Emitir write_all` tolerante a E/S); el `id-mapping` externo→interno
  que abrió el cap. 3 sobre slotmap.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «para importar, leo el fichero entero y lo proceso» — no: datasets
  > RAM revientan; CSV y JSONL se procesan línea a línea con autocommit
  por registro; GraphML es la excepción documentada; (2) «un único formato
  es suficiente» — no: CSV es interoperabilidad (neo4j-admin), JSONL es
  sin pérdida (admite `Value::Bytes`), GraphML es heredado (Yago,
  Wikidata); (3) «importar un CSV es trivial: split por comas» — no:
  comillas, comillas escapadas con `""`, separador dentro de comillas,
  sufijos `:ID`/`:LABEL`/`:START_ID`/`:END_ID`/`:TYPE`/`:STRING`/`:INT`/
  `:FLOAT`/`:BOOL`, líneas vacías, cabeceras heterogéneas; (4) «el
  parser de CSV es separable del de JSON y del de XML» — comparten
  filosofía (a mano, sin crate) pero NO código: cada uno tiene su propia
  gramática y sus errores tipados; (5) «el exporter puede simplemente
  escribir lo que el importer espera» — no: el exporter CSV produce DOS
  secciones separadas por `# aristas` (la única manera de hacer roundtrip
  con grafos heterogéneos, donde las cabeceras de nodos y aristas son
  incompatibles).
- **Pregunta crítica que el capítulo tiene que responder**: «¿cómo
  conectar LiraDB con el mundo exterior sin atragantarse con la RAM ni
  atar el crate a un parser externo?». Respuesta: tres parsers a mano
  (cero dependencias nuevas), tres formatos canónicos con contrato claro,
  autocommit por registro en CSV/JSONL (streaming), GraphML con su
  estructura particular, subcomandos CLI `import`/`export` con `-f` y
  `--graph` y tests roundtrip que detectan cualquier regresión.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `cap32_import_export.rs` con parsers CSV/JSON/XML a mano | `cargo test -p vol2-liradb --lib cap32` (31/31) |
| `ImportError` tipado con número de línea | `tests_import_export::errores_display` y los `…_con_linea` |
| Streaming CSV: nodos y aristas con `importar_csv_nodos` / `importar_csv_aristas` | `streaming_mil_nodos_jsonl_linea_a_linea` (autocommit por registro) |
| Streaming JSONL con discriminador `"tipo"` | `jsonl_import_nodos_y_aristas_mezclados` y `jsonl_export_formato_exacto_y_bytes_roundtrip` |
| GraphML con id-mapping externo→interno | `graphml_import_ids_externos_a_internos`, `graphml_roundtrip_del_grafo_demo`, `graphml_export_escapa_entidades`, `graphml_import_edge_a_nodo_desconocido` |
| `importar_csv_unico` con separador `# aristas` (roundtrip limpio) | `exportar_csv_unico_y_reimport_roundtrip`, `importar_csv_unico_solo_nodos_sin_aristas` |
| Subcomandos `import`/`export` en la CLI | `cargo test -p liradb-cli` (40/40), `export_csv_a_fichero_y_reimport_roundtrip`, `export_jsonl_a_stdout`, `export_graphml_a_fichero_y_reimport`, `import_csv_fichero_inexistente_exit_1` |
| ALL_GREEN workspace | `./scripts/verify.sh` → `ALL_GREEN` |
| Documentación cruzada | code-map.yml (cap-32 en modules), MIGRATION-PATTERN §37, LEDGER sesión 33 |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «¿cómo entrar y sacar grafos sin atragantarse con la RAM?».

**Respuesta en cinco pasos** (cada uno es una sección del manuscrito):

1. **Streaming obligatorio** (CSV y JSONL): un `Transaccion` envolvente
   bufferizaría TODOS los registros en RAM hasta el commit; datasets de
   10 GB romperían. La solución del cap. 27 ya estaba: cada `Operacion`
   se aplica directamente. El importer hace `autocommit(store,
   Operacion::PutNode(...))` por registro.
2. **Tres formatos, tres compromisos** (sin código duplicado):
   - **CSV** estilo neo4j-admin: `id:ID,name:STRING,age:INT,activo:BOOL,:LABEL`
     — el más interoperable, el más frágil (comillas, escapes).
   - **JSONL** discriminador: `{"tipo":"nodo","id":0,"labels":["Person"],"props":{"name":"Ana"}}`
     — el más simple, el más «nuestro» (admite `Value::Bytes`).
   - **GraphML** XML: `<graph id="..."><node id="ext"><data key="k">v</data></node><edge id="..." source="..." target="..."><data key="t">LIKES</data></edge></graph>`
     — el más expresivo, el más verboso; mapeo id externo→interno.
3. **GraphML es la excepción documentada**: su estructura `<key>`/`<data>`
   obliga a procesar el bloque `<graph>` completo (los atributos `<key>`
   pueden aparecer antes o entre los nodos). El capítulo dice por qué
   el importer de GraphML no es streaming.
4. **Errores tipados con número de línea**: el cap. 31 ya tiene la
   lección de que los errores deben llegar al usuario CON contexto
   («línea N:»). `ImportError { CabeceraInvalida | FilaMalformada {
   linea, causa } | Semantica { linea, causa } | Io | RegistroRechazado }`.
5. **Subcomandos CLI que reutilizan el patrón del cap. 31**:
   `liradb import FICHERO -f FMT --graph demo|empty` carga sobre un grafo
   que arranca del demo o vacío; `liradb export FICHERO -f FMT --graph
   demo|empty` serializa al fichero o a stdout (`-`).

---

## 4. La arquitectura: importers + exporters + tipos comunes

```
                    ┌──────────────────────────────────┐
                    │   vol2_liradb::GraphStore        │  ← trait cap 8
                    │   (hexagonal, in-memory hoy)     │
                    └──────────────────────────────────┘
                                  ▲
        ┌──────────────────┬──────┴──────┬─────────────────┐
        │                  │             │                  │
   importar_csv_…    importar_jsonl   importar_graphml
   exportar_csv_…    exportar_jsonl   exportar_graphml
        │                  │             │                  │
   ┌────┴────┐        ┌────┴────┐        ┌────┴────┐
   │ partir_  │        │parsear_ │        │eventos_ │
   │ csv      │        │ json    │        │ xml      │
   │(a mano)  │        │(a mano) │        │(a mano)  │
   └──────────┘        └─────────┘        └──────────┘
```

Tres parsers a mano (filosofía del cap. 18, regla del crate:
dependency-free). Cada parser es un módulo lógico dentro de
`cap32_import_export.rs`. Los importers comparten `ImportError`,
`EstadisticasImport` y `autocommit` del cap. 27.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Parsers a mano, sin crates | Cero dependencias nuevas (regla del Vol.II: aprender primero, depender después) | `csv = "1"`, `serde_json = "1"`, `quick-xml = "0.31"`: ahorra líneas pero ata el crate a 3 dependencias más | Cap. 18 (lexer/parser manual), decisión MIGRATION-PATTERN §37 |
| 2 | CSV estilo neo4j-admin con `:ID`/`:LABEL`/`:START_ID`/`:END_ID`/`:TYPE`/`:STRING`/`:INT`/`:FLOAT`/`:BOOL` | Compatibilidad con herramientas existentes (neo4j-admin import, Kùzu, Memgraph) | CSV sin tipos (todo STRING): el importer tendría que inferir, frágil y ambiguo | neo4j-admin CSV format, manual Kùzu `COPY FROM` |
| 3 | JSONL con discriminador `"tipo":"nodo"|"arista"` | Cada registro autocontenido, sin estado cross-línea | NDJSON con dos archivos separados: obliga al usuario a mantener la pareja sincronizada | NDJSON spec (ndjson.org), el patrón de Wikidata JSON dumps |
| 4 | GraphML con id-mapping externo→interno por orden de aparición | Las ids externas pueden ser strings opacos (Yago usa URIs); las internas son NodeId densos | Asumir que la id externa ES NodeId (e.g. «q42» no parsea como u32) | Cap. 3 (slotmap: asignación densa predecible), GraphML spec 1.0 |
| 5 | Exporter CSV con dos secciones separadas por `# aristas` | Cabeceras incompatibles entre nodos y aristas; sin separador el importer rompe | Concatenar al mismo stream: la línea 8 (cabecera de aristas con 4 campos) se parseaba como fila de nodo y daba `FilaMalformada` | Lección §37: «la única manera limpia» (test-tesis `exportar_csv_unico_y_reimport_roundtrip`) |
| 6 | Campo CSV vacío = prop AUSENTE (no `Null`) | Coherente con la semántica schemaless del cap. 7 (prop ausente = NULL en queries) | CSV vacío = `Null` explícito: añadiría un tipo nuevo que el motor no tiene | Cap. 7 `Value::Null` (existe pero NO en props de Node — props son `HashMap<String, Value>` y ausentes son equivalentes) |
| 7 | Autocommit por registro (CSV/JSONL) | Streaming: datasets > RAM no revientan; cada fila es su propia tx implícita | Una `Transaccion` envolvente que se commitea al final: bufferiza todo en RAM | Cap. 27 (autocommit), el costo de las transacciones explícitas es proporcional al dataset |
| 8 | `ImportError` con número de línea | El usuario debe saber DÓNDE se rompió (no basta con «error parseando CSV») | Errores stringly-typed tipo `Result<(), String>`: ambiguos, no testables | Cap. 31 (la lección del script mode: «línea N:»); ergonomía de clang/rustc |
| 9 | Subcomandos CLI `import`/`export` enchufados al cap. 31 | La CLI ya tiene el patrón `clap Builder` + `run_con_entrada` + `--graph` | Hacer un binario separado `liradb-import`: duplica dispatch, errores, formato de salida | Cap. 31 (la regla «un intérprete, dos políticas» — aquí son dos subcomandos, un mismo motor) |
| 10 | `-` para stdin/stdout en `import`/`export` | Pipe-friendly (cap. 31 ya lo usa en `script -` y `repl`) | Forzar ruta de fichero: rompe los flujos `... | liradb import - -f csv` | Unix philosophy (Raymond TAOUP, Rule of Composition) |
| 11 | JSONL admite `Value::Bytes` | Sin pérdida: el roundtrip preserva datos binarios | Filtrar `Value::Bytes` en el exporter: pérdida silenciosa de información | Cap. 7 (Bytes es un tipo de primera); el cap. 32 distingue CSV (sin Bytes, prop ausente) vs JSONL (con Bytes, hex-encoded) |
| 12 | GraphML importer NO es streaming | La estructura `<key>` puede aparecer entre nodos; los `<data key="x">` referencian ids declaradas después | Streamizar a fuerza de re-lecturas: complica el código sin ganancia clara | Documentación del capítulo (decisión explícita, no un descuido) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (anécdota + pregunta crítica)**: el dataset de Yago
   (Wikipedia, 4 GB de hechos en GraphML) o el export de Wikidata
   (JSON dumps de 100+ GB) — «¿cómo los metes en una base de datos sin
   que tu laptop se ahogue?». Pregunta del CORPUS enmarcada.
2. **Por qué streaming (filosofía)**: el `Transaccion` envolvente
   bufferiza; el autocommit del cap. 27 es la respuesta. Analogía: el
   read-eval-print del cap. 31 era carácter-a-carácter; el import es
   línea-a-línea.
3. **Los tres formatos** (sección central, la más larga):
   - CSV estilo neo4j-admin: la sintaxis exacta, los sufijos `:TIPO`,
     el roundtrip con dos secciones `# aristas`. Ejemplos pegados del
     demo_graph.
   - JSONL discriminador: la sintaxis, la tabla del Value completo
     (incluido Bytes), la diferencia con CSV.
   - GraphML con id-mapping: la sintaxis, la excepción documentada del
     streaming, el escape de entidades XML.
4. **Errores tipados con número de línea**: tabla de los 5 variantes de
   `ImportError` con ejemplos reales del output del test.
5. **Subcomandos CLI**: el patrón del cap. 31 reutilizado (sin TTY ni
   spawn). Ejemplos pegados de la salida real:
   - `liradb export - -f jsonl --graph demo | head -3`
   - `liradb export /tmp/grafo.csv -f csv --graph demo`
   - `liradb import /tmp/grafo.csv -f csv --graph empty`
6. **Cierre (qué viene)**: cap. 33 (pruebas de una base de datos —
   el siguiente paso de la Parte VII: ¿cómo sabes que tu import no
   corrompió nada?). Conexión con caps. 28/29/30 (la tx implícita por
   registro se beneficia del WAL).

---

## 7. Estilo y tono (consistencia con caps. 27-31)

- **Voz**: igual que el cap. 31 — didáctica, sin solemnidad, con
  anécdotas verificadas y salidas REALES del binario pegadas (no
  reconstruidas de memoria).
- **Diagramas**: 2 ASCII trees (arquitectura parsers + importers; la
  dualidad CSV con/sección), 1 tabla del `Value` ↔ representación.
- **Ejemplos**: 4-5 pegados del binario real (export a stdout, export
  a fichero + reimport, error con línea N, CSV con campos vacíos).
- **Spacing**: el autocommit del cap. 27 (la base de la respuesta
  streaming), el trait `GraphStore` del cap. 8 (puerto hexagonal), la
  CLI del cap. 31 (el patrón clap+entrada inyectada), el `Value`
  schemaless del cap. 7 (la razón del campo CSV vacío).
- **Interleaving**: el intermedio conecta streaming con autocommit
  (cap. 27), parsers a mano con el lexer del cap. 18, errores con
  número de línea con el script mode del cap. 31; el experto conecta
  el roundtrip con el contrato de `:LABEL` (cap. 7) y el ciclo de
  vida de la `Transaccion` (cap. 27) — un import es un «begin» que
  NUNCA se commitea entero, sino registro a registro.
- **Dificultad asimétrica**: una idea nueva por sección (streaming →
  formatos → errores → CLI → cierre); los ejercicios exigen
  reconstrucción mental del flujo de un registro y predicción del
  output de un CSV con campos vacíos heterogéneos.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap32` (31
  tests, milisegundos), `cargo test -p liradb-cli` (40 tests, CLI
  end-to-end), el binario real para probar roundtrips a mano.
- **Citas**: neo4j-admin CSV header format docs; GraphML 1.0 spec
  (graphml.graphdrawing.org); NDJSON spec (ndjson.org); Wikipedia
  Yago (Suchanek et al. 2007, WWW Conf) — dataset real en GraphML;
  Raymond TAOUP (Rule of Composition para `-`); MIGRATION-PATTERN §37
  como prosa verificable.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada
  y fuente (12 en la tabla §5).
- [x] Escenario de fallo visible: línea N del fichero en el error
  (test-tesis `csv_import_aristas_extremos_inexistentes_con_linea`),
  duplicado detectado (test-tesis `csv_import_nodos_duplicado_con_linea`),
  fichero inexistente (`import_csv_fichero_inexistente_exit_1`).
- [x] Código ejecutable en workspace (31 tests cap32 + 6 CLI nuevos =
  ALL_GREEN 748 tests — MIGRATION §37) citado por nombre y función, no
  duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: cinco, de
  «leo entero» a «un único formato basta»).
- [x] Ejercicios con solución verificable (los 31 tests del módulo +
  los 6 de la CLI + el binario real).
- [x] ≥1 ejercicio de retrieval (predecir el output del export a
  stdout de los 6 nodos + 6 aristas; reconstruir el flujo de un
  registro CSV con `:LABEL` separado por `:`) y ≥1 de spacing
  (caps. 7/8/18/27/31 tocados).
- [x] Responde la pregunta crítica del CORPUS («cómo entrar y sacar
  grafos sin atragantarse con la RAM») y abre la puerta al cap. 33.
- [x] Anécdota verificada con fuentes de alta confianza (Yago 2007,
  neo4j-admin docs, GraphML spec, NDJSON spec, Raymond TAOUP).
- [x] Gancho explícito al cap. 31 (la CLI donde se enchufan los
  subcomandos) y delimitación (el 31 es la SHELL; el 32, los
  FORMATOS que entran/salen).
- [x] Bugs reales del §37 contados como lecciones (separador
  `# aristas` para roundtrip, lints nuevos clippy 1.96, `use Read`
  para chain, campo `errores` inexistente).
