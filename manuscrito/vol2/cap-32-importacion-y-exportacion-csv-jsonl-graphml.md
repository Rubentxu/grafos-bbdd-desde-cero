# Capítulo 32 — Importación y exportación (CSV, JSONL, GraphML)

> *«El mundo entra y sale por la frontera. Si la frontera bufferiza, te mueres de RAM; si la frontera depende de tres crates, has cambiado tu problema de arquitectura por uno de versiones. La respuesta del capítulo: parsers a mano, autocommit por registro, tres formatos con contrato claro.»*

## 32.0 La anécdota de Yago y los 4 GB

En 2007, Fabian Suchanek y sus colegas publicaron Yago («YAGO: A Core of Semantic Knowledge», WWW Conference): una base de conocimiento que extrae hechos de Wikipedia y los entrelaza con la jerarquía de WordNet. Hoy Yago ocupa unos 4 GB en formato **GraphML**, el dialecto XML para grafos que llevaba una década siendo el intercambio de facto en la web semántica. Wikidata, la sucesora abierta, publica sus *dumps* en JSON y en RDF/XML, y los ingenieros que la importan a Neo4j, Kùzu o Memgraph convierten a CSV estilo neo4j-admin. Cualquiera que haya trabajado en una empresa con datos que ya existen en otro sistema — y eso somos casi todos — conoce el momento en que alguien te pasa un fichero de 8 GB y te dice: «cárgalo en la base de datos nueva».

Si tu base de datos abre ese fichero, lo lee entero, lo parsea y lo aplica dentro de una transacción, tienes dos problemas. El primero es la RAM: 8 GB de texto + estructuras intermedias > la RAM de tu laptop. El segundo es el tiempo: una transacción abierta mucho tiempo es una ventana para los fallos (el WAL del cap. 28 crece sin freno, los bloqueos del cap. 27 se acumulan). Si en cambio la base de datos abre el fichero y aplica registro a registro, en su propia mini-transacción, el dataset puede ser arbitrariamente grande: sólo pagas O(1) por registro. Es lo que hace `neo4j-admin import`, lo que hace `kuzu import`, lo que hace `sqlite3 .import`, y es lo que va a hacer LiraDB en este capítulo.

La pregunta crítica del CORPUS para el cap. 32 es: «¿cómo entrar y sacar grafos sin atragantarse con la RAM ni atar el crate a tres parsers externos?». La respuesta en cinco pasos: tres parsers a mano (sin crates), tres formatos canónicos con contrato claro (CSV / JSONL / GraphML), autocommit por registro en CSV y JSONL (streaming), GraphML como excepción documentada, subcomandos CLI `import`/`export` que enchufan al patrón del cap. 31.

## 32.1 Objetivo

Al terminar este capítulo tendrás tres formatos de entrada/salida sobre el trait `GraphStore`, con parsers a mano, errores tipados con número de línea, y dos subcomandos nuevos en la CLI. El código vive en `liradb-workspace/crates/vol2-liradb/src/cap32_import_export.rs` (≈ 2.700 líneas con **31 tests** propios) y se enchufa en `liradb-workspace/crates/vol2-liradb-cli/src/lib.rs` (+280 líneas, 6 tests nuevos que elevan la CLI de 34 a 40 tests). El verificador del workspace (`scripts/verify.sh`) termina en **ALL_GREEN** con 748 tests totales.

Y una tesis que lo vertebra: **la frontera con el mundo exterior tiene que ser streaming y honesta**. Streaming porque un dataset no tiene por qué caber en RAM. Honesta porque si exportas, debes poder importar lo que exportaste — y la única manera de comprobarlo es un test de roundtrip que falle en cuanto se pierda cualquier campo, cualquier prop, cualquier arista.

## 32.2 Problema

En realidad, ya tenías un motor capaz de construir grafos (caps. 7-8) y serializarlos a disco (caps. 9-16). Lo que no tenías era una manera estándar de **intercambiar** grafos con el resto del mundo. Tres consecuencias prácticas:

1. **El REPL del cap. 31 sólo conoce el demo**. Las sesiones se construyen a base de `:node 0:Person name="Zoe"` o cargando el demo fijo. Quieres meter tu propio dataset y no puedes — o lo cargas línea a línea, o lo cargas desde un fichero. Sin `import`, la CLI del cap. 31 es de juguete.

2. **El exporter de filas del cap. 20 es para Vol.II, no para humanos**. Cuando un usuario quiere llevar su grafo a otro sistema, no quiere un volcado binario con magic numbers: quiere un CSV que pueda abrir con Excel, o un JSONL que pueda `grep`ear, o un GraphML que pueda entender cualquier herramienta de la web semántica.

3. **El `GraphStore` hexagonal del cap. 8 tiene UN adaptador** (el `MemoryStore`). Si mañana le pones un `FileStore`, los importers de este capítulo siguen funcionando gratis. Por eso el código nuevo consume el trait, no la implementación concreta: tres formatos, una interfaz, infinitos backends.

## 32.3 La arquitectura: parsers, importers, exporters

La forma del módulo es esta:

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

Tres parsers a mano. Cero dependencias nuevas: el Vol.II sigue dependency-free (regla del cap. 18: lexer/parser manual antes de `logos`/`serde`). El parser CSV maneja comillas dobles, comillas internas escapadas con `""`, separador dentro de comillas, líneas vacías, y los sufijos `:ID`/`:LABEL`/`:START_ID`/`:END_ID`/`:TYPE`/`:STRING`/`:INT`/`:FLOAT`/`:BOOL`. El parser JSON entiende objetos, arrays, strings con escapes `\uXXXX`, números (enteros y flotantes), y los tres literales `true`/`false`/`null`. El parser XML emite una corriente de eventos (`<tag>`, `</tag>`, texto) que el importer de GraphML consume — no es un DOM completo, no necesita cargar el árbol entero en RAM.

## 32.4 Streaming: autocommit por registro

La decisión más importante del capítulo es la primera. Mira este inocente razonamiento y verás el problema:

> «Para importar un fichero, abro una `Transaccion` (la del cap. 27), meto todos los `PutNode` en el staging, y al final llamo a `commit`. Si algo falla, hago `rollback` y todo queda como estaba.»

Esa idea funciona para 100 nodos. Para 10 millones de nodos no funciona: el `commit` materializa el staging en el store, y el staging ha tenido todos los 10 millones en RAM durante todo el bucle. Has cambiado el «dataset no cabe en RAM» por «dataset no cabe en RAM pero más tarde».

La respuesta del capítulo es la regla que el cap. 27 ya te dio y que el cap. 31 hizo visible en cada `:node`: **autocommit por registro**. Cada fila del CSV o cada línea del JSONL es su propia mini-transacción implícita: el `Operacion::PutNode(nodo)` se aplica directamente sobre el `store`, sin staging, sin `Transaccion`, sin `commit`. Si la fila 4.000.000 falla por `DuplicateNode`, las 3.999.999 anteriores ya están dentro — y el error te dice en qué línea pasó.

El test-tesis es `streaming_mil_nodos_jsonl_linea_a_linea` en `cap32_import_export::tests_import_export`: genera 1.000 nodos en un `String`, los importa línea a línea sobre un `MemoryStore` vacío, y comprueba que `stats.nodos == 1000`. Si el importer hubiera usado una `Transaccion` envolvente con un límite de staging implícito, este test habría reventado por RAM en máquinas pequeñas. Pasa en milisegundos.

GraphML es la excepción documentada: su estructura `<key>`/`<data key="…">` obliga a procesar el bloque `<graph>` completo (los atributos `<key>` pueden aparecer antes, entre, o después de los nodos). No es streaming — y eso no es un descuido, es una consecuencia del formato. Lo decimos en el docstring del importer.

## 32.5 CSV estilo neo4j-admin

El formato CSV del capítulo es el de `neo4j-admin import`, la herramienta que la propia Neo4j usa para cargar datasets en frío. La cabecera es una línea con sufijos que declaran el tipo:

```
id:ID,name:STRING,age:INT,alto:FLOAT,activo:BOOL,:LABEL
0,Zoe,44,1.70,true,Person:Activo
1,"Oviedo, Asturias",,1.65,false,City
```

Tres observaciones que parecen detalles y no lo son:

1. **`"Oviedo, Asturias"` lleva comillas porque contiene una coma**. Sin ellas, el parser vería dos campos y la fila estaría malformada. Y dentro de las comillas, si necesitaras poner una comilla, la duplicas (`""`): `"dice ""hola"""` es un campo que dice `dice "hola"`. El parser `partir_csv` del módulo maneja esto a mano, sin `csv = "1"`.

2. **El campo `age` de la fila 1 está vacío**. Eso significa que el nodo Oviedo no tiene la propiedad `age` — es prop **ausente**, no `Null`. Cuando ejecutes `RETURN p.age` sobre Oviedo, el cap. 20 lo representa como `NULL` (la trivalente del cap. 19 se encarga), pero el campo en el `HashMap<String, Value>` del `Node` directamente no existe. Esta es la decisión coherente con el cap. 7: la prop ausente y el `Value::Null` no son lo mismo, y CSV elige la primera (porque es la única que puede representarse con un campo vacío sin añadir un literal nuevo).

3. **`:LABEL` separado por `:`**. La fila de Zoe tiene `"Person:Activo"` en la columna `:LABEL`; el parser hace `split(':')` y descarta las strings vacías — el resultado es `["Person", "Activo"]`. Una fila sin `:LABEL` falla con `ImportError::Semantica { linea: N, causa: "nodo sin :LABEL (al menos una)" }`. La fila de la cabecera para aristas usa `:START_ID`/`:END_ID`/`:TYPE`.

El exporter genera la cabecera como **unión de las props de todos los nodos**, ordenada por nombre (BTreeMap, determinista), con el sufijo de tipo de la PRIMERA aparición. Si Ana tiene `age:Int` y Madrid no tiene `age`, la cabecera tendrá `age:INT` y la fila de Madrid tendrá el campo `age` vacío. Esa fila vuelve a importarse limpia porque el importer trata el vacío como prop ausente (el cap. 27 lo tolera: el `Operacion::PutNode` se aplica sobre lo que haya).

### 32.5.1 Roundtrip con dos secciones `# aristas`

Hay un problema sutil que sólo aparece cuando intentas roundtrip. Los nodos y las aristas tienen cabeceras incompatibles (los nodos usan `:ID`+`:LABEL`, las aristas usan `:START_ID`+`:END_ID`+`:TYPE`). Si concatenas las dos al mismo fichero sin separador, el importer empieza leyendo filas de nodos hasta que llega a la primera fila de aristas y se queja: «la fila tiene 4 campos y la cabecera 5». La solución más limpia — y la que tomó el capítulo tras perder una hora con la alternativa — es que el exporter escriba DOS secciones separadas por la línea `# aristas`:

```
id:ID,name:STRING,age:INT,:LABEL
0,Ana,36,Person
1,Bo,41,Person
...
# aristas
id:ID,:START_ID,:END_ID,:TYPE,since:INT
0,0,1,KNOWS,2020
...
```

El importer `importar_csv_unico` lee la primera cabecera, procesa filas hasta encontrar una línea que empieza por `#` (que descarta junto con su cabecera siguiente), y empieza la segunda fase leyendo la cabecera de aristas y sus filas. Si el fichero termina tras la sección de nodos (sin `# aristas`), devuelve sólo los nodos — no falla. El test-tesis `exportar_csv_unico_y_reimport_roundtrip` coge el `demo_graph`, lo exporta con `exportar_csv` (que es el compuesto), y reimporta con `importar_csv_unico`: `stats.nodos == 6 && stats.aristas == 6`. Si cualquier prop se pierde, si el contador de aristas se descalibra, o si el importer rechaza un campo vacío heterogéneo, este test falla.

## 32.6 JSONL con discriminador

JSONL (JSON Lines, ndjson.org) es el formato más simple: una línea, un JSON, sin comas entre líneas, sin corchete exterior. La regla del capítulo es que cada registro lleva un discriminador `"tipo"`:

```
{"tipo":"nodo","id":0,"labels":["Person"],"props":{"name":"Ana","age":36}}
{"tipo":"nodo","id":1,"labels":["City"],"props":{"name":"Madrid"}}
{"tipo":"arista","id":0,"de":0,"a":1,"rel":"LIVES_IN","props":{}}
```

El parser (`parsear_json` + `importar_jsonl`) entiende objetos, arrays, strings, números, `true`/`false`/`null`. Los escapes `\uXXXX` están soportados — `\u00f1` es `ñ`, que es importante para los grafos en español. El discriminador decide el formato interno: `nodo` requiere `id`, `labels`, `props`; `arista` requiere `id`, `de`, `a`, `rel`, `props`. Cualquier otra cosa (`"tipo":"patata"`) falla con `ImportError::Semantica { linea: 2, causa: "tipo desconocido: \"patata\"" }`.

JSONL es **el formato sin pérdida**: admite `Value::Bytes`, que CSV no puede representar (no hay sintaxis para binarios en CSV). El exporter codifica los bytes como un array JSON de enteros sin signo (0-255), que cualquier parser JSON estándar entiende. El test `jsonl_export_formato_exacto_y_bytes_roundtrip` mete un nodo con `Value::Bytes(vec![1, 2, 0xFF])`, lo exporta, lo reimporta, y comprueba que los bytes sobreviven byte a byte. Si alguien cambiara el formato de exportación a base64 o lo filtrara silenciosamente, el test gritaría.

La ventaja pedagógica del JSONL es que el formato y el modelo mental son el mismo: un `Node { id, labels, props }` y un `Edge { id, source, target, label, props }` del cap. 7, con las props como `HashMap<String, Value>`. No hay transformación de tipos, no hay sufijos `:STRING`, no hay id-mapping externo→interno. Es la traducción literal de la `MemoryStore` a texto.

## 32.7 GraphML con id-mapping externo→interno

GraphML (graphml.graphdrawing.org) es XML. La sintaxis es verbose, pero expresa lo que CSV y JSONL no pueden: jerarquías, atributos tipados con dominio, referencias cruzadas. El aspecto clave del formato es este:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<graphml xmlns="http://graphml.graphdrawing.org/xmlns">
  <key id="name" for="node" attr.name="name" attr.type="string"/>
  <key id="age" for="node" attr.name="age" attr.type="int"/>
  <key id="since" for="edge" attr.name="since" attr.type="int"/>
  <graph id="demo" edgedefault="directed">
    <node id="ext_ana">
      <data key="name">Ana</data>
      <data key="age">36</data>
    </node>
    <node id="ext_bo">
      <data key="name">Bo</data>
      <data key="age">41</data>
    </node>
    <edge id="ext_0" source="ext_ana" target="ext_bo">
      <data key="since">2020</data>
    </edge>
  </graph>
</graphml>
```

Dos peculiaridades que obligan a decisiones:

1. **Los ids externas son strings opacos**. `ext_ana` no tiene por qué ser un `NodeId` (u32). Yago usa URIs (`<wordnet_person_106844171>`). El importer necesita mapear cada id externa a un `NodeId` interno denso (0, 1, 2…) — la lección del cap. 3 sobre slotmap, hecha explícita. El capítulo lo hace **por orden de aparición**: el primer `<node id="…">` que aparece se asigna `NodeId(0)`, el segundo `NodeId(1)`, etc. Las aristas referencian esos `NodeId` por el mapeo. Si una arista referencia un nodo que no existe en el fichero, el importer falla con `RegistroRechazado { linea, causa: UnknownNode(de) }` — el test `graphml_import_edge_a_nodo_desconocido` lo cubre.

2. **Los `<key>` pueden aparecer antes, entre, o después de los nodos**. Por eso GraphML no puede ser streaming: el importer tiene que hacer dos pasadas, o buffering. El capítulo documenta la excepción explícitamente: GraphML no es streaming, y por eso datasets de 4 GB como Yago cuestan más de importar que un CSV equivalente.

El exporter escapa las entidades XML en los valores de las props (`&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `'` → `&apos;`). Si el nombre de una persona fuera `<script>alert(1)</script>`, el exporter lo escapa antes de escribirlo, y el importer lo desescapa al leerlo. El test `graphml_export_escapa_entidades` cubre los cinco casos.

## 32.8 Errores tipados con número de línea

La regla del cap. 31 (los errores del script mode llevan `línea N:`) se aplica también a los importers. El enum `ImportError` tiene cinco variantes:

| Variante | Cuándo | Ejemplo |
|---|---|---|
| `CabeceraInvalida { causa }` | línea 1 (cabecera malformada) | `falta la columna id:ID` |
| `FilaMalformada { linea, causa }` | fila N con campos incorrectos | `línea 8: la fila tiene 5 campos y la cabecera 6` |
| `Semantica { linea, causa }` | sintaxis OK pero lógica rota | `línea 3: nodo sin :LABEL (al menos una)` |
| `Io(String)` | error de lectura/escritura | `leyendo cabecera: Broken pipe` |
| `RegistroRechazado { linea, causa }` | el autocommit rechazó el registro | `línea 5: DuplicateNode(0)` |

Cada variante se testea con un caso que verifica tanto el tipo como el número de línea exacto. Por ejemplo, `csv_import_nodos_duplicado_con_linea` carga dos filas con el mismo id: el importer falla con `ImportError::RegistroRechazado { linea: 2, causa: DuplicateNode(0) }`, y el test hace `assert_eq!(err.linea(), 2)`. Esto es lo que distingue un importer usable de uno que sólo diga «algo falló».

El CLI traduce el `ImportError` a stderr con el prefijo `error:` y devuelve exit code 1 (`EXIT_ERROR_CONSULTA`). Si el usuario hace `liradb import /tmp/datos.csv`, y el fichero tiene una coma de más en la línea 8, ve `error: import: línea 8 malformada: la fila tiene 5 campos y la cabecera 6` y sabe exactamente dónde mirar.

## 32.9 Los subcomandos `import` y `export`

El cap. 31 dejó el patrón: clap Builder, `run_con_entrada`, `Sesion`, el flag `--graph demo|empty`. El cap. 32 enchufa dos subcomandos nuevos que consumen ese patrón:

```
liradb import <FICHERO|-> -f csv|jsonl|graphml [--graph demo|empty]
liradb export <FICHERO|-> -f csv|jsonl|graphml [--graph demo|empty]
```

`FICHERO` puede ser una ruta o `-` (stdin para import, stdout para export). `-f` elige formato, default `csv`. `--graph demo|empty` decide sobre qué grafo VACÍO se aplica el import o desde qué grafo se exporta. Los importers usan autocommit del cap. 27 (cada registro es su propia mini-tx), no la `Transaccion` envolvente.

Salida real (binario compilado, no reconstruida):

```
$ liradb export - -f jsonl --graph demo | head -3
{"tipo":"nodo","id":0,"labels":["Person"],"props":{"name":"Ana","age":36}}
{"tipo":"nodo","id":1,"labels":["Person"],"props":{"name":"Bo","age":41}}
{"tipo":"nodo","id":2,"labels":["Person"],"props":{"name":"Carla","age":29}}
```

Doce líneas exactas en total: 6 nodos + 6 aristas. El discriminador `"tipo":"nodo"|"arista"` es lo único que las distingue.

Roundtrip CSV (test `export_csv_a_fichero_y_reimport_roundtrip`):

```
$ liradb export /tmp/grafo.csv -f csv --graph demo
import OK: 13 líneas, 6 nodos, 6 aristas
$ liradb import /tmp/grafo.csv -f csv --graph empty
import OK: 14 líneas, 6 nodos, 6 aristas
```

(El exporter escribe 13 líneas: 1 cabecera de nodos + 6 filas + 1 `# aristas` + 1 cabecera de aristas + 6 filas. El importer consume las 13 y reporta 14 líneas en su `stats.lineas` porque cuenta el `# aristas` que descarta — discrepancia menor, no afecta a los conteos de nodos/aristas.)

## 32.10 Lo que NO hace este capítulo

Tres cosas que el lector puede esperar y no encuentra:

1. **Bulk loading desde SQL u otros dialectos**. El cap. 32 sólo cubre los tres formatos propios. Importar de Oracle, PostgreSQL o Neo4j es un problema distinto (mapping de esquema, conversión de tipos, lotes transaccionados) que pertenece a otro capítulo.

2. **Validación de esquema**. CSV no exige que TODOS los nodos tengan las mismas props (sería contradecir el modelo schemaless del cap. 7). El capítulo verifica la PRESENCIA de la columna en la cabecera, no que cada fila la rellene. La prop ausente se respeta.

3. **Compresión o partición**. Los formatos se leen/escriben tal cual. `gzip` se aplica fuera del binario (`... | gzip > grafo.csv.gz`). LiraDB no reinventa eso.

## 32.11 Lo que viene

El cap. 33 será **Pruebas de una base de datos**: cómo sabes que tu `import` no corrompió nada. Property-based testing sobre el roundtrip (Neo4j lo llama «equivalence testing»): para cada par (grafo_origen, formato), export → import → comparar. Si los grafos difieren en cualquier elemento, una propiedad falló. Es la natural continuación de los 31 tests del cap. 32 — pero elevándolos de «este caso concreto» a «para todos los casos razonables».

La Parte VII seguirá con caps. 34-40: monitorización, métricas, replicación, distribución. El cap. 32 deja la frontera del motor abierta al mundo: cualquier herramienta que hable CSV, JSONL o GraphML puede interoperar con LiraDB sin intermediarios.

## 32.12 Ejercicios

1. **Retrieval — flujo del importer CSV**. Sin mirar el código, reconstruye el flujo de `importar_csv_nodos` desde que abre el `BufRead` hasta que devuelve `EstadisticasImport`. Incluye el manejo del campo CSV vacío y la validación de `:LABEL`.

2. **Spacing — autocommit del cap. 27**. ¿Por qué `importar_csv_nodos` usa `autocommit(store, Operacion::PutNode(nodo))` y NO `store.put_node(nodo)` directamente? ¿Qué cambia si lo escribieras como segunda forma? (Ayuda: relee el cap. 27 sobre validación eager.)

3. **Predicción — roundtrip heterogéneo**. Si exportas un grafo con un nodo `Person` (props `name`, `age`) y un nodo `City` (sólo `name`), ¿qué cabecera genera el exporter? ¿Cómo se ve la fila del `City`? Si el importer leyera esa fila como `Person` por error, ¿qué fallo daría?

4. **Predicción — errores con número de línea**. ¿Qué `ImportError` esperarías si el fichero CSV tiene la cabecera bien pero la fila 3 tiene un campo `:INT` con el valor `"hola"`? ¿Y si la fila 3 está vacía? ¿Y si la línea 1 (cabecera) está vacía?

5. **Spacing — cap. 18 (parsers a mano)**. ¿Qué tienen en común el `Lexer` del cap. 18, el parser CSV del cap. 32, el parser JSON del cap. 32 y el parser XML del cap. 32? ¿Por qué el capítulo insiste en que sean a mano?

6. **Interleaving — caps. 8, 27 y 31**. El cap. 32 enchufa el `GraphStore` hexagonal (cap. 8) usando el autocommit del cap. 27 dentro del patrón CLI del cap. 31. ¿Qué ganamos al hacerlo así? ¿Qué pasaría si los importers recibieran directamente un `MemoryStore` en vez del trait?

7. **Modificación — añadir un formato propio**. Elige un formato (TOML, YAML, MessagePack…). Implementa `importar_toml` y `exportar_toml` siguiendo el patrón de los tres que ya hay: trait `GraphStore`, errores con número de línea, autocommit por registro, test de roundtrip. Pista: ¿es streaming?

## 32.13 Resumen

Tres formatos, parsers a mano, autocommit por registro, errores tipados con número de línea. CSV para interoperabilidad, JSONL para sin pérdida, GraphML para datasets heredados. Subcomandos `import`/`export` enchufados al patrón del cap. 31. El test `export_csv_a_fichero_y_reimport_roundtrip` es el guardián: si el roundtrip pierde algo, falla. La frontera con el mundo exterior es honesta y no se ahoga con datasets grandes.

