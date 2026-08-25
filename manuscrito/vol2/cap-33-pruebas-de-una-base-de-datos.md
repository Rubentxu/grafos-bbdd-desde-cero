# Capítulo 33 — Pruebas de una base de datos

> *«Setecientos ochenta y ocho tests verdes demuestran que el motor sobrevive a TU imaginación. La base de datos vivirá en el mundo real — y el mundo real viene con tijeras.»*

## 33.0 La anécdota de la esquina

En 1985, Jim Gray — el mismo de las transacciones (cap. 27) y de la bitácora (cap. 28) — trabajaba en Tandem Computers, la compañía cuyas máquinas se llamaban **NonStop** porque su promesa era ésa: no parar nunca. Aquel año recogió los partes de fallo que los clientes habían reportado en producción y los diseccionó uno a uno en «Why Do Computers Stop and What Can Be Done About It?» (Tandem Computers, Technical Report 85.7, 1985): 132 incidentes reales en sistemas «tolerantes a fallos». El resultado incomodó a todos. El hardware redundante — la razón de ser del producto, discos y procesadores duplicados — apenas explicaba un 18 % de los fallos. El SOFTWARE rondaba el 40 %, y la operación — los humanos alrededor del sistema — cerca de otro 27 %. Entre ambos sumaban siete de cada diez caídas.

La respuesta de Gray no fue pedir más redundancia: fue cambiar cómo se prueba. Un sistema «tolerante» cuyos operadores jamás han visto un fallo no tiene garantías; tiene fe. Su prescripción — que Tandem practicaba matando procesos sanos y desconectando discos a propósito — fue hacer de la **inyección de fallos** (*fault injection*) una rutina, no un accidente. Cuarenta años después, esa rutina tiene nombre propio en toda base de datos seria: baterías de contrato, **property-based testing**, crash testing y **golden masters**. Este capítulo construye la de LiraDB, piso a piso, respondiendo la pregunta crítica del CORPUS: ¿qué tipos de prueba existen, qué riesgo ataca cada uno, y cuál necesita tu motor HOY?

## 33.1 Objetivo

Al terminar este capítulo tendrás la torre de pruebas completa del motor: un **verificador de invariantes** que funciona contra cualquier `GraphStore`, una **batería de contrato** (*contract test*) compartida por todas las implementaciones del puerto, cinco **propiedades** generadas con `proptest`, una suite de **crash testing** que ataca los bytes REALES del WAL, unos golden tests a mano para la salida de la CLI y los contratos de compatibilidad del formato. El código nuevo vive en dos sitios: `liradb-workspace/crates/vol2-liradb/src/cap33_pruebas.rs` (1.424 líneas, 17 tests + 1 doctest) y el PRIMER directorio `tests/` del workspace — `crates/vol2-liradb-cli/tests/golden_cli.rs` (142 líneas, 3 tests) con sus dorados en `tests/golden/*.txt`. La única dev-dependency nueva es `proptest = "1"` (resuelta a 1.11.0, pineada por el lock). El workspace termina en ALL_GREEN con **809 tests**: los 788 que tenías más estos 21.

Y una tesis que lo vertebra: **no es una pirámide de «cuántos», es una torre de «qué pregunta»**. Cada piso nuevo existe porque ataca un riesgo que el piso inferior NO ve — y el verificador de invariantes es el oráculo común que todos los pisos superiores afirman.

## 33.2 Problema

En realidad, ya tienes 788 tests verdes. Y aun así te daría miedo abrir un WAL a mano y cortarlo con tijeras. Debería darlo: mira lo que tus unitarias NO cubren:

1. **Inputs que nadie escribió.** Tus tests prueban casos CONOCIDOS, elegidos por ti. Ninguno genera grafos que no se te ocurrieron ni operaciones en orden que no planeaste. Como escribió Dijkstra en «Notes on Structured Programming» (EWD249, 1970): *el testing muestra la PRESENCIA de errores, nunca su ausencia*. Verde significa «no encontré nada donde miré», no «no hay nada».
2. **Bytes hostiles.** Todo el pipeline de persistencia — encoding (cap. 9), framing+CRC (cap. 28) — come bytes. Ningún test hasta hoy ha volteado un bit del fichero del WAL ni truncado el log a mitad de registro.
3. **Implementaciones alternativas del puerto.** Los tests actuales sólo conocen `MemoryStore`. Si mañana escribes un segundo `GraphStore`, empiezas de cero.
4. **La salida que ve el usuario.** Nadie pactó cómo debe verse `liradb demo` ni `liradb explain`. Un formateo «inocente» puede cambiar el producto sin que ningún test proteste.

La raíz de todo: **cada tipo de fallo vive en un piso distinto, y tus 788 tests viven casi todos en el mismo**. Unitarias de casos conocidos: rápidas, baratas, localizan el fallo — y ciegas a interacciones, azar y malicia.

## 33.3 Modelo mental

Piensa en una **torre de riesgos**. No se sube por cantidad de tests sino por tipo de pregunta:

```text
        ¿QUÉ RIESGO ATACA CADA PISO?
 ▲  fuzzing         bytes crudos contra el parser      (nightly, FUERA de CI)
 │  crash           el mundo hostil: cortes, bits volteados, colas perdidas
 │  golden          ¿la salida pactada sigue siendo idéntica?
 │  property-based  ¿las propiedades aguantan inputs que NO elegí yo?
 │  integración     ¿funciona la CADENA completa de piezas?
 │  contrato        ¿CUALQUIER GraphStore se comporta igual tras el puerto?
 │  unitarias       ¿esta unidad hace lo esperado en casos conocidos?
 └────────────────────────────────────────────────────────────────────────▶
   frecuentes, baratas, primeras en fallar                    caras, selectivas
```

Mapeada a módulos concretos de LiraDB:

| Piso | Pregunta de riesgo | Dónde vive |
|---|---|---|
| Unitarias | ¿acierta en casos conocidos? | caps. 7-32 (los 788) |
| Contrato | ¿cualquier store pasa el MISMO guion? | `bateria_de_contrato` (cap. 33) |
| Integración | ¿funciona parse→execute→export completo? | CLI (cap. 31) + formatos (cap. 32) |
| Property-based | ¿aguantan inputs que no elegí yo? | `prop_*` con estrategias `arb_*` |
| Fuzzing | ¿sobrevive el parser a bytes crudos? | documentado, fuera del pipeline (§33.9) |
| Golden | ¿la salida de usuario sigue idéntica? | `tests/golden_cli.rs` |
| Crash | ¿resiste el fichero podrido? | `crash_*` sobre el WAL real |
| Compatibilidad | ¿el fichero de hoy abre mañana? | `compat_*` (magic + versión, cap. 9) |

Las cuatro invariantes del grafo quedan clasificadas así:

```text
Toda relación referencia nodos existentes.               ← observable DESDE EL PUERTO
Cada relación saliente tiene su entrada correspondiente. ← observable DESDE EL PUERTO
Ningún slot apunta fuera de una página.                  ← INTERNA: check() (cap. 16)
Los índices contienen sólo IDs válidos.                  ← INTERNA: Csr::verify() (cap. 14)
```

El momento ¡ajá!: las dos primeras son verificables con la API pública del puerto (`iter_edges` + `get_node`; `out_edges` ↔ `in_edges`). Las otras dos NO — y eso es una FEATURE, no una carencia: el store hexagonal no expone slots ni páginas, así que su guardián vive dentro de su módulo. Honestidad hexagonal (cap. 8): cada capa vigila lo que posee.

## 33.4 Primera solución

La versión ingenua es la que todo el mundo escribe, incluidos nosotros hasta ayer: asserts sueltos repartidos por los tests, copiados y pegados con pequeñas variantes:

```rust
// En cada test, a mano, otra vez:
assert!(store.get_node(4).is_some());
assert!(store.get_edge(10).is_some());
assert!(store.in_edges(1).contains(&10));
// …y la misma idea, ligeramente distinta, en el fichero de tests de abajo.
```

Funciona. Detecta lo que el autor de ese test sospechó. Y ahí muere su utilidad.

## 33.5 Sus límites

1. **Duplicación sin oráculo.** Diez tests con diez bucles de validación distintos: cuando descubres una invariante nueva recuerdas actualizar diez sitios — y fallas en el undécimo.
2. **¿Pruebas el puerto o tu store?** Sin una SUITE compartida, cada implementación nueva exige reimaginar los tests. Y puede pasarlos por motivos equivocados.
3. **El universo de inputs lo pones tú.** Un bucle de validación no genera nada: comprueba lo que ya sabías que podía pasar.
4. **Los bytes del disco siguen intactos bajo los tests.** El crash del mundo real no llega por una llamada a función: llega a mitad de escritura, en un byte concreto del fichero.
5. **La salida del usuario no está pactada.** Cambiar un espacio en la tabla de resultados es invisible para todos esos asserts.

## 33.6 Solución evolucionada

### El oráculo común: verificar_invariantes

Una sola función, firma honesta: `verificar_invariantes(&dyn GraphStore) -> Result<(), Vec<InvarianteRota>>`. Recorre el puerto completo — huérfanas (`iter_edges` + `get_node`), fantasmas y asimetrías (`out_edges`/`in_edges` contra `get_edge`) — y devuelve TODAS las rotas de golpe: un diagnóstico completo vale más que uno rápido, y cada variante de `InvarianteRota` imprime en cristiano qué vista está podrida.

¿Y cómo sabes que el detector DETECTA? Con mutaciones deliberadas, estilo *mutation testing*: los tests corrompen `MemoryStore` POR SUS CAMPOS PÚBLICOS, como haría un bug real de índice. `invariantes_grafo_sano_pasa` usa el grafo demo (6 nodos, 6 aristas, self-loop incluido) como contraejemplo perfecto de «no hay bugs»; `invariantes_detectan_arista_huerfana_en_adj` cuela el id 999 en `adj_out[0]` y espera `FantasmaEnSalientes`; `invariantes_detectan_salida_sin_entrada` borra la entrada de la arista 0 en `adj_in[1]` y espera EXACTAMENTE `SalidaSinEntrada` — la dirección de la asimetría también es contrato; `invariantes_detectan_nodo_borrado_por_debajo_del_puerto` hace `nodes[4] = None` sin cascada y la LIVES_IN de Ana→Madrid queda huérfana. Si algún día refactorizas `MemoryStore` y el verificador deja de ver, estos tests sangran.

### La batería de contrato: un guion, cualquier store

`bateria_de_contrato(fabrica: impl Fn() -> Box<dyn GraphStore>)` ejercita el ciclo de vida COMPLETO del puerto: nacimiento vacío (y `out_edges(7)` vacío, jamás pánico), inserción, duplicados rechazados, endpoints inválidos, orden de inserción en las vistas, iteración coherente (`Σ out_edges == edge_count`), doble `delete_edge`, cascada de `delete_node` y re-inserción de ids reciclados — con el oráculo firmando después de cada fase. Dos detalles que valen un capítulo:

- **El contrato real manda.** La documentación del trait (cap. 8) dice que `put_node` «inserta o reemplaza»; la implementación RECHAZA duplicados con `DuplicateNode`. La batería fija el comportamiento QUE SE TESTEA: duplicados rechazados, `delete_edge` dos veces es `true` luego `false`, ids reutilizables. El doc dice una aspiración; el contrato testeable, la verdad.
- **Sin segunda implementación, no es un contrato.** La consumen `contrato_bateria_memory_store` (producción) y `contrato_bateria_store_alternativo` — un `GraphStore` sobre HashMaps escrito EN LOS TESTS, misma semántica, otra estructura interna. Ambos pasan SIN tocar una línea de la batería: eso demuestra que pruebas el PUERTO, no un store concreto. Es la misconcepción «más tests sueltos = mejor» al revés: UNA suite bien parametrizada sustituye decenas de tests clónicos. (Nota honesta: `MvccStore` queda FUERA — sus lecturas llevan `ts` de snapshot y no implementa `GraphStore`; fingirlo exigiría cambiar el cap. 30. Queda como reto experto.)

### Propiedades: inputs que no eliges tú

Con `proptest` llegan las **estrategias** (*strategies*) y el **shrinking**: el generador produce casos aleatorios y, cuando una propiedad falla, encoge el contraejemplo hasta el mínimo que aún falla. Nuestras estrategias (`arb_nodo`, `arb_arista_valida`, `arb_grafo`) comparten una decisión que vale por todas: las aristas son SIEMPRE válidas porque muestrean extremos DENTRO del pool de nodos ya generados. La alternativa ingenua — generar `(u,v)` cualesquiera y filtrar con `prop_filter` — destroza el shrinking: casos útiles rarísimos y counterexamples gigantes (la lección de Claessen y Hughes, «QuickCheck», ICFP 2000). Encoger aquí significa MENOS elementos, jamás elementos inválidos.

Sobre ellas, cinco propiedades-teorema: `prop_roundtrip_encoding_byte_identico` (todo `Value` vivo sobrevive encode→decode→reencode con BYTES idénticos — la API pública del cap. 9), `prop_wal_replay_reproduce_estado` (las mismas operaciones por autocommit del cap. 27 y por WAL real del cap. 28, disco de por medio, producen EL MISMO grafo), `prop_jsonl_roundtrip_preserva_grafo` y `prop_csv_roundtrip_preserva_grafo` (props CSV-seguros: tipo fijo por clave, sin Null — el campo vacío es prop ausente, cap. 32), y `prop_csr_consistente_con_iteracion_directa` (el CSR del cap. 14 ve los mismos vecinos que el puerto, multiconjunto a multiconjunto, más `Csr::verify()` en verde). Todas terminan firmando el oráculo común.

Dos hallazgos que destapó diseñarlas, y que valen más que los tests mismos. Primero: `Float(3.0)` se serializa como `3` en JSONL/CSV y reimporta como `Int(3)` — por eso la estrategia genera SIEMPRE floats con parte fraccional representable: la propiedad habla del dominio donde el roundtrip es exacto. Segundo: el escapador JSONL emite `\uXXXX` de cuatro dígitos (sólo BMP); un carácter fuera del BMP produciría un escape malformado que reimportaría CORROMPIDO y SIN error — la frontera honesta del parser a mano del cap. 32, documentada en la propia estrategia, y el anzuelo perfecto para el piso que falta: fuzzing.

Aclaremos de una vez: **property-based y fuzzing NO son lo mismo**. Lo primero genera inputs VÁLIDOS y estructurados, con semántica del dominio; lo segundo machaca un parser con BYTES CRUDOS sin sentido alguno, guiado por cobertura. Se complementan; ninguno sustituye al otro (Hamlet, «Random Testing», 1994, ya distinguía el muestreo sistemático del azar ciego).

### Crash testing: el hallazgo estrella

Aquí viene lo que este capítulo vino a buscar. La suite ataca los BYTES reales del fichero WAL — no flags en memoria: `wal_de_prueba()` escribe tres transacciones confirmadas (13 registros, 548 bytes) con `guardar_wal` del cap. 29, y las tijeras entran de verdad. `crash_truncado_sistematico_nunca_panico` corta el fichero en CADA prefijo posible: la lectura indulgente JAMÁS entra en pánico y cada prefijo resucitado cumple las invariantes. `crash_bit_flip_bajo_crc_es_ruidoso` voltea un bit en cada byte del cuerpo del último registro: el CRC responde SIEMPRE `CrcInvalido` — y la indulgente, callando, entrega 12 de los 13 registros.

Y entonces `crash_carga_estricta_reporta_cola_perdida` encuentra lo que nadie quería encontrar: corta DENTRO del último registro y compara los dos modos de leer. Ejecutando la misma secuencia a mano contra la API pública:

```text
WAL sano: 548 bytes, 13 registros
último registro: bytes [523..548] (25 bytes)

cargar_wal_estricta(&path)
→ Err(carga estricta del WAL: WAL: registro truncado (hay 1 bytes,
   el length-prefix reclama 25) — ¿corte de luz a mitad de escritura del log?)

cargar_wal(&path) + replay_wal(...)     [modo indulgente]
→ Ok(12 registros) · transacciones_confirmadas = 2 de 3 · nodos recuperados = 3
```

Léelo despacio: había 3 transacciones confirmadas y la recuperación del cap. 29 entrega 2 — **y no avisa de nada**. Ni error, ni log, ni cifra. La causa está en el corazón del `WalIterator` (cap. 28, línea 856): `Err(_) => None`. Ante el primer registro dañado, la iteración CALLA y termina; `cargar_wal` y `reabrir` heredan ese silencio. Recuperar el prefijo limpio es legítimo — es la parada limpia de ARIES ante un log append-only (Mohan et al., ACM TODS 1992). Lo que NO es legítimo es perder bytes sin decir cuántos. La solución es quirúrgica: `cargar_wal_estricta` vive EN cap33 y delega en `decodificar_wal`, la versión ESTRICTA que el cap. 28 YA tenía — cero cambios en capítulos cerrados; se AÑADE la voz alta, no se cambia el comportamiento de nadie. Éste es el contrato **fail-stop** del capítulo: o recuperación limpia INFORMADA, o error ruidoso con cifras — jamás pánico, jamás pérdida silenciosa.

### Golden tests: pactar la salida de usuario

El último piso no prueba el motor: prueba el PRODUCTO. `tests/golden_cli.rs` guarda en `tests/golden/demo.txt` y `tests/golden/explain.txt` la salida exacta de `liradb demo` y `liradb explain` — la cara que ve el usuario — y compara byte a byte con `std` puro: leer fichero, `assert_eq!`, y una variable de entorno. Sin crates de snapshots: `insta` aportaría maquinaria que un capítulo SOBRE decidir qué se necesita no compra (regla «primero a mano»). Dos reglas separan un golden serio de la superstición:

1. **Determinismo ANTES que dorado.** `golden_las_salidas_son_deterministas` ejecuta cada comando DOS veces y exige bytes idénticos. Con timestamps o azar visible no podrías dorar — arreglarías el comando primero. Dorar sin esto es la misconcepción «copiar el output y rezar».
2. **Regeneración explícita, revisable en diff.** `ACTUALIZAR_GOLDEN=1 cargo test -p liradb-cli --test golden_cli` reescribe los dorados e imprime un AVISO en stdout: pactar salida de usuario es una decisión de producto, no un trámite automático.

Detalle de honestidad: los dorados invocan `["demo"]` y `["explain", consulta]` SIN `--graph` — clap lo rechaza en esos subcomandos (cap. 31), así que el flag simplemente no aparece. El dorado documenta la CLI tal cual es.

## 33.7 Código completo ejecutable

Todo compila verde y vive en dos piezas que puedes leer de corrido. `crates/vol2-liradb/src/cap33_pruebas.rs` (1.424 líneas) tiene arriba el código de producción: `InvarianteRota` con sus cinco variantes y su `Display`, `verificar_invariantes`, `bateria_de_contrato` y `cargar_wal_estricta` con su enum `ErrorCargaEstricta { Io, Wal }` (WalError no tiene variante de E/S, y el cap. 28 no se toca). Abajo, en `#[cfg(test)]`: la implementación didáctica `StoreAlternativo`, las estrategias `arb_*`, las cinco `prop_*`, la suite `crash_*` y los `compat_*`. Y `crates/vol2-liradb-cli/tests/golden_cli.rs` (142 líneas): el helper `comprobar_golden` con su diff mínimo — primera línea divergente, esperado vs actual — y los tres tests. Las firmas que lo sostienen todo:

```rust
pub fn verificar_invariantes(store: &dyn GraphStore) -> Result<(), Vec<InvarianteRota>>;
pub fn bateria_de_contrato(fabrica: impl Fn() -> Box<dyn GraphStore>);
pub fn cargar_wal_estricta(path: impl AsRef<Path>) -> Result<Vec<WalRecord>, ErrorCargaEstricta>;
```

Fíjate en lo que NO hay: ni un `unsafe`, ni dependencias nuevas en producción (`proptest` y `tempfile` son dev-dependencies — nada que corre en release depende del generador aleatorio), ni cambios en ningún capítulo anterior.

## 33.8 Prueba de fuego

El bucle de feedback del capítulo entero cabe en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap33

running 17 tests
test cap33_pruebas::tests_cap33::compat_magic_erroneo_rechazado ... ok
test cap33_pruebas::tests_cap33::compat_version_la_comprueba_el_llamador ... ok
test cap33_pruebas::tests_cap33::contrato_bateria_memory_store ... ok
test cap33_pruebas::tests_cap33::contrato_bateria_store_alternativo ... ok
test cap33_pruebas::tests_cap33::invariantes_detectan_arista_huerfana_en_adj ... ok
test cap33_pruebas::tests_cap33::invariantes_detectan_entrada_sin_salida ... ok
test cap33_pruebas::tests_cap33::invariantes_detectan_nodo_borrado_por_debajo_del_puerto ... ok
test cap33_pruebas::tests_cap33::invariantes_detectan_salida_sin_entrada ... ok
test cap33_pruebas::tests_cap33::invariantes_grafo_sano_pasa ... ok
test cap33_pruebas::tests_cap33::crash_bit_flip_bajo_crc_es_ruidoso ... ok
test cap33_pruebas::tests_cap33::crash_carga_estricta_reporta_cola_perdida ... ok
test cap33_pruebas::tests_cap33::crash_truncado_sistematico_nunca_panico ... ok
test cap33_pruebas::tests_cap33::prop_wal_replay_reproduce_estado ... ok
test cap33_pruebas::tests_cap33::prop_roundtrip_encoding_byte_identico ... ok
test cap33_pruebas::tests_cap33::prop_csr_consistente_con_iteracion_directa ... ok
test cap33_pruebas::tests_cap33::prop_csv_roundtrip_preserva_grafo ... ok
test cap33_pruebas::tests_cap33::prop_jsonl_roundtrip_preserva_grafo ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 622 filtered out;
finished in 0.95s

$ cargo test -p liradb-cli --test golden_cli

running 3 tests
test golden_demo_coincide ... ok
test golden_explain_coincide ... ok
test golden_las_salidas_son_deterministas ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.00s
```

(Salta a la vista que las `prop_*` tardan lo mismo que las unitarias: 256 casos por propiedad por defecto, con estrategias baratas. Property-based no tiene por qué ser lento.) Y el binario dorado coincide consigo mismo:

```text
$ cargo run -q -p liradb-cli -- demo | head -5
LiraDB — demo del motor (caps. 17-21: parse → lower → optimizador → Volcano)
Grafo demo: 6 nodos (Person/City) y 6 aristas (KNOWS/LIVES_IN)

── [1/4] MATCH simple (NodeScan + Project) ──
LiraQL: MATCH (p:Person) RETURN p.name, p.age
```

La prueba de fuego conceptual es la del §33.6: corta TU un WAL con tijeras — `guardar_wal`, luego `fs::write(path, &bytes[..corte])` — y observa al par indulgente/estricto discrepar en la frase más importante del capítulo: uno entrega 2 transacciones de 3 callando; el otro grita `RegistroTruncado` con los bytes disponibles contados. Y si algún día cambias a propósito el formato de `explain`: `ACTUALIZAR_GOLDEN=1 cargo test -p liradb-cli --test golden_cli`, y el `git diff` de los `.txt` delante de tus ojos antes de confirmar.

## 33.9 Qué hemos sacrificado

1. **Fuzzing documentado, FUERA del pipeline.** `cargo-fuzz` exige toolchain nightly y el workspace pinea la estable 1.96.0 (política de reproducibilidad). Los **fuzz targets** naturales están identificados — `decode_value` (cap. 9) y el parser JSON a mano (cap. 32), con la frontera BMP ya señalizada — pero ningún paso de fuzz corre en `verify.sh`. Es la línea roja honesta de la torre.
2. **MVCC fuera de la batería.** `MvccStore` no implementa `GraphStore`; adaptarlo fijando un `ts` cambiaría un capítulo cerrado. Documentado como reto experto, no falseado.
3. **Sin fsync real.** Herencia del cap. 29: la suite corta bytes pero no simula la pérdida de la caché del sistema operativo a mitad de escritura. Eso es otro nivel de crash testing.
4. **Sin differential testing contra otra BD.** Comparar LiraDB contra SQLite o Neo4j en grafos aleatorios cazaría bugs de semántica que ninguna propiedad local ve. Queda fuera del libro.
5. **Criterion deliberadamente ausente.** Medir rendimiento es el asunto del cap. 34 (dataset de referencia, percentiles); mezclarlo aquí difumina ambos capítulos.

## 33.10 Cómo lo hace una BBDD real

Todo lo que aquí es un módulo de 1.400 líneas, allí es industria. **SQLite** trae `PRAGMA integrity_check`, que recorre la B-tree completa comprobando sus invariantes bajo demanda — nuestro verificador, con cuarenta años de rodaje. **PostgreSQL** ofrece `amcheck` y `pageinspect` para verificar índices y páginas desde SQL, y su equipo ha usado *sqlsmith* para generar consultas aleatorias contra el motor (property-based y fuzzing conviviendo, cada uno en su piso). La joya metodológica es **FoundationDB**: según Zhou et al. («FoundationDB: A Distributed Unbundled Transactional Key Value Store», SIGMOD 2021), TODO el sistema corre dentro de una simulación determinista con inyección de fallos — cada bug encontrado en simulación se reproduce con exactitud byte a byte, y el equipo lo considera la razón de su fiabilidad. Y **Jepsen** (Kingsbury) es el crash testing hecho servicio industrial: particiona redes, mata procesos y relojes desincronizados contra bases de datos reales — y encuentra exactamente el tipo de pérdida silenciosa que nuestra carga estricta vino a desterrar.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: añade UNA invariante nueva observable desde el puerto (por ejemplo: «ningún nodo lista la misma arista dos veces en su vista saliente») con su variante en `InvarianteRota` y su test de mutación que demuestre que el detector detecta. ¿Qué campo público de `MemoryStore` corrompes?
- *Intermedio*: escribe `prop_delete_node_replay_reproduce_estado`: operaciones con `DeleteNode` aplicadas por autocommit (cap. 27) y por WAL+replay (cap. 28); exige estados iguales y oráculo verde. ¿Qué hace falta sembrar antes para que los deletes tengan víctimas?
- *Experto*: adapta `MvccStore` al trait `GraphStore` fijando un timestamp de lectura (sin tocar el cap. 30) hasta pasar `bateria_de_contrato` — o dora la salida del modo script (`liradb script guion.ql`) y discute qué cambia cuando el comando lee ficheros externos.

## 33.11 Lo que te llevas

- **La torre, no la pirámide**: cada piso de prueba ataca un riesgo que el inferior no ve. Pregunta qué riesgo ataca tu test nuevo; si no hay respuesta, es decoración.
- **Las unitarias muestran presencia, no ausencia** (Dijkstra, 1970): 788 tests verdes no significan «probado».
- **Un oráculo común**: `verificar_invariantes` opera SÓLO sobre el puerto; lo interno tiene guardianes propios (`check()` del cap. 16, `Csr::verify()` del cap. 14). Los tests sí bajan por los campos públicos — para demostrar que el detector detecta.
- **Un contract test exige ≥2 implementaciones**: la misma batería pasa por `MemoryStore` y por `StoreAlternativo`; sin la segunda, no distingues puerto de store.
- **Property-based ≠ fuzzing**: inputs válidos estructurados con shrinking frente a bytes crudos guiados por cobertura. Se complementan.
- **El hallazgo del capítulo**: la recuperación indulgente traga la cola corrupta EN SILENCIO; `cargar_wal_estricta` grita con cifras. Fail-stop: informado o ruidoso, jamás mudo.
- **Golden = determinismo primero + regeneración explícita**: `golden_las_salidas_son_deterministas` y `ACTUALIZAR_GOLDEN=1`.
- **Compatibilidad**: el magic se rechaza siempre; `FORMAT_VERSION` lo compara QUIEN ABRE — política de evolución: nuevo tag de `Value`, bump de versión.

## 33.12 Ojo, cuidado con…

- **«Property-based es fuzzing barato»**. No: uno genera dominio válido con semántica y encoge contraejemplos útiles; el otro busca bytes que hagan sangrar un parser. Si sustituyes uno por otro, dejas un piso de la torre sin vigilar.
- **«Dorar es copiar el output al repo»**. Sin el test de determinismo previo ni la regeneración explícita, tienes un fichero muerto que fallará por motivos aleatorios o pasará por motivos peores.
- **«El cap. 29 ya me protege de crashes»**. Te protege del PÁNICO, no del silencio: recupera el prefijo y calla lo perdido. La protección real es saber QUÉ perdiste.
- **Bajar el verificador a campos de `MemoryStore`**. El verificador usa sólo el puerto; si necesita mirar dentro, la invariante es INTERNA y pertenece a su módulo (caps. 14/16).
- **Filtrar estrategias con `prop_filter` masivo**. Muestrea válido desde la generación (extremos del pool); filtrar mata el shrinking y produce counterexamples ilegibles.
- *Precisión de lenguaje*: *property-based* (dominio estructurado) vs *fuzzing* (bytes crudos); *golden master* (salida pactada) vs *snapshot* (mecánica de una librería); *contract test* (batería compartida) vs *unitaria* (caso suelto); *fault injection* (romper de verdad) vs *mock* (simular el error en memoria); *fail-stop* (ruido o informe) vs *parada limpia* (silencio tolerable).

## 33.13 Pin de batalla

> *«Cada piso de pruebas existe porque el piso inferior es ciego a algo. Un test que no ataca un riesgo nuevo no es una prueba: es decoración verde.»*

## 33.14 Si solo lees 30 segundos

Ocho pisos, ocho preguntas. Unitarias: casos conocidos (los tenías). Contrato: UNA batería sobre `&dyn GraphStore`, pasada por dos implementaciones. Integración: la cadena completa ya corre en la CLI y los formatos. Property-based con proptest: estrategias que generan grafos SIEMPRE válidos (extremos del pool, nunca filtros) y cinco propiedades — encoding idéntico byte a byte, replay WAL reproduce el estado, roundtrips JSONL/CSV preservan el grafo, CSR ≡ iteración directa. Fuzzing: documentado fuera del pipeline (nightly). Golden: la salida de `demo`/`explain` pactada en `tests/golden/*.txt`, determinismo comprobado antes, regeneración con `ACTUALIZAR_GOLDEN=1`. Crash: tijeras y bit-flips sobre el WAL REAL — y el hallazgo: `cargar_wal` recupera el prefijo y TRAGA la cola en silencio; `cargar_wal_estricta` grita `RegistroTruncado` contando bytes. Compatibilidad: magic rechazado, versión decidida por quien abre. Todo en `cap33_pruebas.rs` + `tests/golden_cli.rs`: 809 tests ALL_GREEN, feedback en milisegundos.

## 33.15 Una historia pequeña

Cuando Claessen y Hughes presentaron QuickCheck («QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs», ICFP 2000), no lo estrenaron sobre código propio: lo apuntaron contra librerías Haskell YA PUBLICADAS, código en uso que sus lectores daban por correcto. Y cazó fallos reales — entre otros, en una implementación publicada de árboles AVL y en un codificador de Huffman de un libro de texto. Ningún usuario había tropezado con esos casos en años de uso; el generador los encontró en minutos porque no buscaba donde los humanos buscan. Es exactamente la diferencia entre las 788 unitarias del motor — casos que alguien imaginó — y las propiedades de este capítulo: casos que NADIE imaginó todavía. La herramienta de 2000 vive hoy en Rust con otro nombre — proptest, cuya estrategia y shrinking son descendientes directos — y la moraleja sigue intacta: los bugs más caros viven donde nadie miró, no donde todos miraron.

## Ejercicios resueltos

**1. Predice la variante.** En el grafo demo, borras de `adj_in[1]` la arista 0 (Ana→Bo) SIN tocar nada más. ¿Qué devuelve `verificar_invariantes`? La arista 0 sigue VIVA (`iter_edges` la lista, sus extremos existen) y sigue en `out_edges(0)`; lo que falta es su entrada. Diagnóstico: `InvarianteRota::SalidaSinEntrada { arista: 0 }` — y NUNCA `EntradaSinSalida`: la dirección de la asimetría también es contrato, y el test la exige negativamente. Verificación: `invariantes_detectan_salida_sin_entrada`.

**2. Retrieval: las cuatro invariantes, sin mirar.** Cierra el libro y escribe de memoria las cuatro invariantes textuales del grafo. Luego clasifícalas: ¿cuáles son observables desde el puerto hexagonal y cuáles viven internas con guardián propio? Respuesta: (1) toda relación referencia nodos existentes — PUERTO (`iter_edges` + `get_node`); (2) cada relación saliente tiene su entrada correspondiente — PUERTO (`out_edges` ↔ `in_edges`); (3) ningún slot apunta fuera de una página — INTERNA (`check()`, cap. 16); (4) los índices contienen sólo IDs válidos — INTERNA (`Csr::verify()`, cap. 14). Si escribiste «ninguna arista huérfana» como UNA sola invariante, revisa: las huérfanas y los fantasmas/asimetrías son familias DISTINTAS con variantes distintas en `InvarianteRota`. Verificación: `invariantes_grafo_sano_pasa` y las tres mutaciones de §33.6.

## Ejercicios propuestos

**Esencial (recordar + predecir).** (a) De memoria: reconstruye los ocho pisos de la torre con la pregunta de riesgo y el módulo LiraDB de cada uno. (b) Empareja mutación → test que la caza y diagnóstico esperado: colar 999 en `adj_out[0]`; `nodes[4] = None`; voltear un bit del cuerpo del último registro del WAL; escribir un magic ajeno en la cabecera del cap. 9. *Verificación*: `invariantes_detectan_arista_huerfana_en_adj`, `invariantes_detectan_nodo_borrado_por_debajo_del_puerto`, `crash_bit_flip_bajo_crc_es_ruidoso`, `compat_magic_erroneo_rechazado`. *Criterio*: emparejamiento completo SIN abrir los tests.

**Intermedio (spacing 9+28+29: predice cifras).** Toma el WAL sano de tres transacciones (§33.6), corta el fichero en `inicio_ultimo + 3` y ANTES de ejecutar nada, anota: (a) qué variante y qué valor de `disponibles` devuelve `cargar_wal_estricta`; (b) cuántas transacciones confirmadas reportará el replay indulgente; (c) cuántos nodos tendrá el store resucitado. Luego ejecuta la secuencia y compáralo todo. *Verificación*: replicar `crash_carga_estricta_reporta_cola_perdida`. *Criterio*: las tres cifras exactas por escrito antes del primer comando.

**Intermedio (interleaving 32+33).** Exporta el demo a CSV (cap. 32), reimporta en un store vacío y firma `verificar_invariantes`. Después corrompe `adj_out` a mano y clasifica la variante que salta. Por último explica: ¿por qué la estrategia CSV genera props con TIPO FIJO POR CLAVE y sin Null — y qué pasaría con `Float(3.0)` en una columna? *Verificación*: `prop_csv_roundtrip_preserva_grafo` y el roundtrip de `exportar_csv`/`importar_csv_unico`. *Criterio*: conectar campo vacío = prop ausente (cap. 32) con la normalización Float→Int.

**Experto (crear).** Dos caminos, elige uno: (a) el adaptador `GraphStoreParaMvcc` que fija un timestamp de lectura y hace pasar `MvccStore` por `bateria_de_contrato` — documenta qué fases de la batería discuten con la semántica de snapshots; (b) dora `liradb script guion.ql` siguiendo el patrón de `golden_cli.rs` — primero el test de determinismo, después el dorado — y explica qué riesgo nuevo introduce dorar un comando que lee FICHEROS externos. *Verificación*: la batería en verde con el tercer store, o `cargo test -p liradb-cli --test golden_cli` con tu dorado nuevo. *Criterio*: cero cambios en caps. anteriores.

## Para profundizar

- **Jim Gray, «Why Do Computers Stop and What Can Be Done About It?» (Tandem Computers Technical Report 85.7, 1985)** — la fuente primaria de la anécdota: los partes de fallo reales, la cuota mínima del hardware, y la inyección de fallos como rutina.
- **E. W. Dijkstra, «Notes on Structured Programming» (EWD249, 1970)** — «testing shows the presence, not the absence of bugs»: la frase que gobierna el piso unitario.
- **Koen Claessen y John Hughes, «QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs» (ICFP 2000)** y el **proptest Book** (altsysrq.github.io/proptest-book) — generación de inputs, composición de estrategias y shrinking: el porqué de `arb_arista_valida`.
- **Richard Hamlet, «Random Testing», en Encyclopedia of Software Engineering (1994)** — el muestreo aleatorio sistemático mucho antes del hype; distingue azar ciego de perfil de uso.
- **C. Mohan, Don Haderle, Bruce Lindsay, Hamid Pirahesh y Peter Schwarz, «ARIES: A Transaction Recovery Method…», ACM TODS 17(1), 1992** — la parada limpia ante log truncado: por qué recuperar el prefijo es correcto y avisar es obligatorio.
- **Alistair Cockburn, «Hexagonal Architecture» (2005)** — el puerto como frontera de todo contrato testeable; **Meszaros, «xUnit Test Patterns» (2007)** para golden master y baterías compartidas.
- **Jingyu Zhou et al., «FoundationDB: A Distributed Unbundled Transactional Key Value Store» (SIGMOD 2021)** — simulación determinista con inyección de fallos como método, no como accesorio.
- **jepsen.io (Kyle Kingsbury)** — crash testing industrial contra bases de datos reales; lee cualquier análisis y buscarás la cola perdida en silencio en tus propios sistemas.
- **SQLite: `PRAGMA integrity_check` (docs oficiales)** y **PostgreSQL: `amcheck`, `pageinspect`, y sqlsmith** — los verificadores de invariantes y el fuzzing de los motores maduros.
- **cargo-fuzz book (rust-fuzz.github.io)** — cómo se escribe un fuzz target real, para cuando derribes la línea roja del nightly.

## Mini-diálogo: en guardia nocturna

> — Suena el pager: `golden_explain_coincide` en rojo. Son las tres de la mañana.
>
> — Respira. ¿Qué dice el diff mínimo del fallo?
>
> — «Primera diferencia en la línea 7: esperado `Expand(p, r:KNOWS, OUTGOING, f)   est. 1 filas`, actual `est. 2 filas`». ¡Alguien tocó el optimizador esta tarde!
>
> — Alguien cambió la estimación de filas. Ahora decides: ¿es intencional?
>
> — ¿Y quién soy yo para decidirlo a las tres?
>
> — Exacto: NO lo eres. El dorado no decide, DOCUMENTA. Regenera con `ACTUALIZAR_GOLDEN=1`, mira el `git diff`, y mañana el cambio se revisa como lo que es: una decisión de producto pactada, no un efecto secundario aceptado.
>
> — Podría simplemente editar el `.txt` a mano…
>
> — Podrías. Y habrías convertido el pacto en copia y rezar. La regeneración imprime un aviso y deja rastro precisamente para que esto no sea un trámite.
>
> — ¿Y si la salida cambiara entre ejecuciones?
>
> — Entonces `golden_las_salidas_son_deterministas` habría gritado antes: sin determinismo no hay dorado posible. Primero arreglas el comando, después lo pactas. Gray inyectaba fallos por la mañana; tú, por lo visto, los recibes de noche.

---

*(Próximo capítulo: 34 — Benchmarks. La torre está completa: sabes que el motor es correcto. Falta la pregunta incómoda: ¿rápido ES correcto? Dataset de referencia, percentiles y Criterion — porque «primero correcto, luego rápido» termina siempre en medir, y medir mal es otra forma de mentir.)*
