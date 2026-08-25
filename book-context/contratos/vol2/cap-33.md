# CONTRATO DE CAPÍTULO — Vol.II Cap. 33: Pruebas de una base de datos

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla: el que el
> capítulo pone a PRUEBA ya existe y está verde — `cap08_graph_store.rs` (trait
> `GraphStore` + `MemoryStore`, `StoreError::InvalidEdgeEndpoints`), `cap09_encoding.rs`
> (`FORMAT_VERSION = 1`, magic `0x4C_44_42_31`, `decode_header` valida el magic pero
> DEVUELVE la versión), `cap28_wal.rs` (framing+CRC32; `decodificar_wal` estricta con
> `WalError::{CrcInvalido, RegistroTruncado, LsnInvalido}` frente a `WalIterator`,
> que corta en silencio ante registro dañado), `cap29_recuperacion.rs`
> (`guardar_wal`/`cargar_wal`/`reabrir`, ARIES), `cap14_csr.rs` (`Csr::verify()`),
> `cap16_mantenimiento.rs` (`check()` de integridad), `cap32_import_export.rs`
> (roundtrips CSV/JSONL/GraphML, 31 tests) y la CLI `crates/vol2-liradb-cli`
> (`run_con_entrada` testeable sin TTY, 40 tests). Código NUEVO previsto: módulo
> `cap33_pruebas.rs` en `vol2-liradb`, el PRIMER directorio `tests/` del workspace
> (`vol2-liradb-cli/tests/golden_cli.rs` + `tests/golden/*.txt`) y `proptest = "1"`
> como ÚNICA dev-dependency nueva (hoy solo `tempfile = "3.10"`). Estado verificado
> 2026-08-25: `cargo test --workspace --locked` → **788 tests** en verde; toolchain
> pinneada 1.96.0. Decisiones/hallazgos irán a `MIGRATION-PATTERN.md` §38. Pregunta
> crítica del CORPUS (`vol-II-cap-33`): «Unit, contract, integration, property-based
> (`proptest`), fuzz, golden, crash.» Cap. 3 de la Parte VII. Gancho: cap. 34
> (benchmarks — ¿rápido ES correcto? primero correcto, luego rápido).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el puerto hexagonal `GraphStore` del cap. 8 (y que
  `MemoryStore` es hoy su única implementación de producción); el encoding binario
  del cap. 9 (tags de `Value`, framing+CRC del cap. 10); el autocommit del cap. 27;
  el WAL con commit en dos fases del cap. 28; ARIES simplificado del cap. 29
  (`reabrir`); MVCC del cap. 30 (API propia con `ts`: NO implementa el trait);
  la CLI testeable del cap. 31; los tres formatos del cap. 32; ~788 tests verdes
  en el workspace.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «788 tests verdes = está probado» — no: casi todos son unitarios de CASOS
  CONOCIDOS elegidos por el autor; prueban presencia de errores donde miraste,
  nunca ausencia (Dijkstra, 1970); ningún test ataca bytes que nadie escribió.
  (2) «property-based y fuzzing son lo mismo» — no: property-based genera INPUTS
  VÁLIDOS estructurados (grafos, operaciones) con *shrinking*; fuzzing machaca el
  parser con BYTES CRUDOS sin semántica; se complementan, no se sustituyen.
  (3) «un golden test es copiar el output al repo y rezar» — no: exige
  DETERMINISMO primero y una vía de regeneración explícita después.
  (4) «la recuperación del cap. 29 está testeada, un crash no puede corromperme» —
  no: hay que atacar los BYTES reales del log; hallazgo real del capítulo:
  `cargar_wal`/`reabrir` TRAGAN la cola corrupta en silencio (`WalIterator` corta
  en `Err(_) => None`) y recuperan el prefijo limpio SIN avisar de lo perdido.
  (5) «más tests sueltos = mejor probado» — la batería de contrato demuestra lo
  contrario: UNA suite bien parametrizada sobre el puerto sustituye decenas de
  tests clónicos, y sin una SEGUNDA implementación no sabes si pruebas el
  puerto o tu store.
- **Pregunta crítica que el capítulo tiene que responder**: «¿qué tipos de prueba
  existen, qué riesgo ataca cada uno, y cuál de ellos necesita LiraDB HOY?».
  Respuesta: una taxonomía de 8 pisos (unitaria → contrato → integración →
  property-based → fuzz → golden → crash → compatibilidad de formato), mapeada a
  módulos concretos del motor, con código nuevo pequeño (un verificador de
  invariantes, una batería de contrato, estrategias proptest, una suite de crash
  y unos goldens a mano) y una línea roja honesta: lo que NO se cablea (fuzzing
  nightly) y por qué.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `cap33_pruebas.rs`: `verificar_invariantes(&dyn GraphStore) -> Result<(), Vec<InvarianteRota>>` | `cargo test -p vol2-liradb --lib cap33`; tesis `invariantes_grafo_sano_pasa`, `invariantes_detectan_arista_huerfana_en_adj`, `invariantes_detectan_salida_sin_entrada` |
| Batería de contrato del puerto, parametrizada por factory (`Fn() -> Box<dyn GraphStore>`) | misma función llamada por `contrato_bateria_memory_store` y `contrato_bateria_store_alternativo` (impl didáctica HashMap escrita en los tests) |
| Estrategias proptest de grafos válidos (`arb_nodo`, `arb_arista_valida`, `arb_grafo`) con shrink acotado | las consumen TODAS las `prop_*`; sin `prop_filter` masivo (encogen mal) |
| Propiedad: roundtrip encoding byte-idéntico (cap. 9) | `cargo test -p vol2-liradb --lib cap33 prop_roundtrip_encoding_byte_identico` |
| Propiedad: replay WAL reproduce el estado (caps. 27/28) | `prop_wal_replay_reproduce_estado` (commit con `WalTransaccion`, replay en store fresco, estado igual) |
| Propiedad: roundtrip JSONL y CSV preserva el grafo (cap. 32) | `prop_jsonl_roundtrip_preserva_grafo`, `prop_csv_roundtrip_preserva_grafo` (props limitadas a tipos CSV) |
| Propiedad: CSR consistente con iteración directa (caps. 14/26) | `prop_csr_consistente_con_iteracion_directa` + `Csr::verify() == Ok` |
| Suite de crash sobre bytes REALES del WAL (fichero temporal, truncados y bit-flips) | `crash_truncado_sistematico_nunca_panico`, `crash_bit_flip_bajo_crc_es_ruidoso`, `crash_carga_estricta_reporta_cola_perdida` |
| Compatibilidad de formato (magic + versión) | `compat_magic_erroneo_rechazado`, `compat_version_la_comprueba_el_llamador` |
| Golden tests CLI a mano (std: leer fichero + `assert_eq!` + env var) | `cargo test -p liradb-cli --test golden_cli` (`golden_demo_coincide`, `golden_explain_coincide`); regeneración: `ACTUALIZAR_GOLDEN=1 cargo test -p liradb-cli --test golden_cli` |
| `proptest` como dev-dependency pineada (lock) | `./scripts/verify.sh` pasa fmt/check/test/lint con `--locked` |
| Fuzzing documentado, FUERA del pipeline | prosa N.10 + MIGRATION-PATTERN §38; NINGÚN paso `cargo-fuzz` en `verify.sh` |
| ALL_GREEN workspace | `./scripts/verify.sh` → `ALL_GREEN` (788 + ~20 tests nuevos, cifra exacta en §38) |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Unit, contract, integration, property-based (`proptest`), fuzz,
golden, crash.» — la lista de tipos del brief, que el capítulo convierte en
**respuesta en ocho pasos** (cada tipo responde UNA pregunta de riesgo distinta
sobre un módulo concreto):

1. **Unitarias**: ya existen (788, caps. 7-32) — casos conocidos, rápidas,
   localizan el fallo. El capítulo NO añade más «porque sí»: explica qué NO
   pueden ver (interacciones, inputs no elegidos, bytes hostiles).
2. **Contratos**: UNA batería compartida que corre contra CUALQUIER
   `GraphStore` tras el mismo puerto (cap. 8). Hoy: `MemoryStore` + una impl
   didáctica `StoreAlternativo` (HashMap) escrita en los tests. Ciclo de vida
   completo, duplicados, endpoints inválidos, cascada de `delete_node`,
   coherencia de iteración — todo expresado UNA vez.
3. **Integración**: la cadena completa parse → lower → optimize → execute →
   export → reimport → verificar invariantes, usando piezas de los caps.
   18/20/21/32. La CLI ya era integración end-to-end (40 tests); aquí se
   encadena con los formatos y el verificador.
4. **Property-based** (`proptest`, dev-dependency aprobada en CONVENTIONS §4):
   estrategias que generan GRAFOS VÁLIDOS aleatorios y cuatro propiedades-teorema:
   encoding byte-idéntico, replay WAL reproduce el estado, roundtrip
   JSONL/CSV preserva el grafo, CSR ≡ iteración directa.
5. **Fuzzing** (`cargo-fuzz`/libFuzzer): DOCUMENTADO, no cableado — exige
   nightly y herramienta externa; targets naturales: `decode_value` (cap. 9)
   y el parser JSON a mano (cap. 32).
6. **Golden tests**: la salida de USUARIO de la CLI (`demo`, `explain`) queda
   PACTADA en ficheros versionados; comparación a mano con std, regeneración
   explícita con `ACTUALIZAR_GOLDEN=1`.
7. **Crash testing**: inyección de fallos sobre el fichero del WAL real —
   truncado sistemático en cada prefijo, bit-flip bajo el CRC, magic podrido;
   contrato fail-stop: o recuperación limpia INFORMADA o error ruidoso, jamás
   pánico ni pérdida silenciosa.
8. **Compatibilidad de formato**: magic rechazado si no coincide; `FORMAT_VERSION`
   lo comprueba QUIEN ABRE (así es el código actual, y así se documenta) con
   política de evolución: nuevo tag de `Value` ⇒ bump de versión.

Las cuatro **invariantes textuales del brief** quedan clasificadas en §4:
dos verificables desde el puerto, dos internas con guardián propio.

---

## 4. La arquitectura: una prueba por tipo de riesgo

Modelo mental único: **cada piso de prueba ataca un riesgo que el piso inferior
no ve**. No es una pirámide de «cuántos», es una torre de «qué pregunta»:

```
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

Objetivo del código nuevo (`cap33_pruebas.rs`): el verificador de invariantes
que TODO piso superior usa como oráculo común. Sobre el puerto (`&dyn GraphStore`)
son verificables las dos primeras invariantes del brief; las otras dos viven
DENTRO de sus módulos y ya tienen guardián (honestidad hexagonal):

```text
Toda relación referencia nodos existentes.        ← puerto: iter_edges + get_node(src/tgt)
Cada relación saliente tiene su entrada correspondiente. ← puerto: out_edges(u) ↔ in_edges(tgt)
Ningún slot apunta fuera de una página.           ← INTERNO: caps. 11/16 (check())
Los índices contienen solo IDs válidos.           ← INTERNO: caps. 14/15 (Csr::verify())
```

El store hexagonal NO expone slots ni páginas — y esa es una FEATURE: el
verificador opera sobre lo observable, y lo interno tiene sus tests propios.
Para demostrar que el detector DETECTA, los tests corrompen `MemoryStore`
POR SUS CAMPOS PÚBLICOS (`adj_out`/`edges`), al estilo mutation testing.
Ese mismo verificador es el ORÁCULO COMÚN de los pisos superiores: contratos
(puerto, cap. 8), propiedades (caps. 9/28/32) y crash/golden (fichero WAL y
salida CLI) acaban todos afirmándolo.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | `verificar_invariantes(&dyn GraphStore)` solo con la API del puerto | Las invariantes de grafo son observables; slots/páginas/índices son internos y YA tienen guardián (`check()` cap. 16, `Csr::verify()` cap. 14) | Un verificador que baja a los campos de `MemoryStore`: rompe la abstracción hexagonal que sostiene el Vol.II entero | Cockburn, Hexagonal Architecture (2005); cap. 8 |
| 2 | Batería de contrato como FUNCIÓN genérica sobre factory, no macro | Legible, depurable, compone: `fn bateria(fabrica: impl Fn() -> Box<dyn GraphStore>)` | `macro_rules!` que duplique la batería por store: errores de compilación crípticos y doble mantenimiento | Pact (contract testing, pact.io docs); Meszaros, xUnit Test Patterns (2007) |
| 3 | Segunda implementación didáctica (`StoreAlternativo`) para validar la batería | Una batería de contrato sin ≥2 implementaciones no distingue «prueba el puerto» de «prueba mi store»; precedentes: `ContandoStore`, `StoreQueFalla` | Correr la batería solo contra `MemoryStore` y llamarlo contrato: autoengaño | Pact docs; caps. 26/28/29 ya escribieron stores de prueba |
| 4 | MvccStore QUEDA FUERA de la batería (documentado, no falseado) | NO implementa `GraphStore` (sus lecturas llevan `ts` de snapshot); fingirlo exigiría cambiar el cap. 30 | Adaptar `GraphStore` para `MvccStore` fijando un ts: cambia API de capítulo cerrado; queda como reto experto | lib.rs cap. 30 (API con `Ts`); decisión explícita §38 |
| 5 | `proptest = "1"` única dev-dependency nueva; `insta` descartada | Golden a mano = leer fichero + `assert_eq!` + env var: std basta; `insta` añade dependencia y formato propio para algo trivial | `insta = "1"`: snapshots cómodos pero otra maquinaria de más en un capítulo SOBRE decidir qué se necesita | CONVENTIONS §4 (regla «primero a mano») + §5 (pineo por lock) |
| 6 | Estrategias de aristas SIEMPRE válidas (extremos muestreados de nodos generados) | Las propiedades deben hablar del dominio válido; el rechazo masivo (`prop_filter`) destruye el shrinking | Generar `(u,v)` cualesquiera y filtrar: casos útiles raros y counterexamples gigantes | proptest Book (strategy composition, shrinking); Claessen & Hughes, QuickCheck, ICFP 2000 |
| 7 | Crash testing por inyección sobre BYTES reales del fichero WAL | El enemigo real es el byte podrido; `truncar_a_bytes`/bit-flip sobre `guardar_wal` → `cargar_wal`/`decodificar_wal` | «Simular» el crash con flags en memoria (como el `sync` contador del cap. 28): no ejercita el framing ni la E/S | Gray, «Why Do Computers Stop…», Tandem TR 85.7 (1985): inyecta fallos de verdad |
| 8 | Hallazgo del capítulo: la cola corrupta se recupera EN SILENCIO; se añade carga estricta en cap33 | `WalIterator` corta en `Err(_) => None` y `cargar_wal`/`reabrir` heredan el silencio; recuperar el prefijo es legítimo (ARIES), PERDER bytes sin avisar no | Endurecer `cargar_wal` del cap. 29: rompería el diseño «parada limpia» documentado del cap. 28 y sus tests | Mohan et al., ARIES, ACM TODS 1992; hallazgo verificado en `cap28_wal.rs:856` |
| 9 | Compatibilidad: `decode_header` valida magic y DEVUELVE la versión; quien abre compara | Así es el código del cap. 9; el capítulo lo hace CONTRATO testeado y define la política de evolución (nuevo tag ⇒ bump) | Cambiar `decode_header` para rechazar versiones: decide por el llamador y oculta la versión al migrador | SQLite File Format (números de versión en cabecera); cap. 9 |
| 10 | Goldens solo de SALIDA DE USUARIO estable (`demo`, `explain`); regeneración `ACTUALIZAR_GOLDEN=1` | Pactar lo que el usuario ve; determinismo verificado (sin timestamps); la regeneración es un acto explícito revisable en diff | Dorar estructuras internas (bytes del encoding): frágil y redundante con las propiedades roundtrip | Snapshot/golden-master testing (Meszaros 2007; docs de snapshot testing de Jest) |
| 11 | Fuzzing documentado y FUERA de `verify.sh` | Requiere nightly + instalación de cargo-fuzz; la toolchain pinneada estable 1.96.0 es política (reproducibilidad) | Meter nightly en `rust-toolchain.toml`: rompe el pineo por capítulo (ADR-002) por un capricho de un capítulo | cargo-fuzz book (rust-fuzz.github.io); libFuzzer; política de toolchain del workspace |
| 12 | Criterion DELIBERADAMENTE ausente | Medir rendimiento es el cap. 34 (dataset de referencia, percentiles); mezclarlos difumina ambos | Un micro-benchmark «de regalo»: contamina el suite y adelanta contenido ajeno | Brief cap. 34; CORPUS `vol-II-cap-34` (dataset 100k/500k) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: Jim Gray disecciona en
   Tandem (1985) los partes de fallo de sistemas «tolerantes a fallos» en
   producción: el hardware tolerante no era el culpable principal — el SOFTWARE
   y la operación lo eran; su respuesta: inyectar fallos a propósito (matar
   procesos, cortar discos) como rutina de prueba. Pregunta del CORPUS
   enmarcada: ¿de qué tipos se compone esa rutina en una BBDD de verdad?
2. **N.1-N.2 Objetivo/Problema**: «tienes 788 tests y aun así te daría miedo
   abrir un WAL a mano y cortarlo con tijeras». Qué NO cubren las unitarias.
3. **N.3 Modelo mental**: la torre de riesgos (§4) + tabla tipo↔pregunta↔módulo
   de LiraDB; clasificación de las 4 invariantes del brief.
4. **N.4 Primera solución**: asserts sueltos y bucles de validación copiados en
   cada test (la versión ingenua que todo el mundo escribe).
5. **N.5 Sus límites**: duplicación, sin oráculo compartido, sin inputs ajenos,
   bytes nunca atacados, salida de usuario sin pactar.
6. **N.6 Solución evolucionada**: verificador de invariantes + batería de
   contrato parametrizada + estrategias proptest + suite de crash + goldens.
   Con el hallazgo estrella: el crash test destapa la cola corrupta silenciosa.
7. **N.7 Código completo ejecutable**: `cap33_pruebas.rs` (referenciado por
   `include::`, nunca duplicado) + `tests/golden_cli.rs`.
8. **N.8 Prueba de fuego**: truncar a mano un WAL real y ver fallar RUIDOSO con
   `carga_estricta`; `ACTUALIZAR_GOLDEN=1` y el diff del dorado; `cargo test
   -p vol2-liradb --lib cap33` en milisegundos.
9. **N.9 Qué hemos sacrificado**: fuzzing fuera de CI, MVCC fuera de la batería,
   sin fsync real (herencia del cap. 29), sin differential testing contra otra BD.
10. **N.10 Cómo lo hace una BBDD real + retos**: SQLite `PRAGMA integrity_check`,
    Postgres `amcheck`/`pageinspect` y su fuzzing con sqlsmith, la simulación
    determinista de FoundationDB (Zhou et al., SIGMOD 2021) y Jepsen
    (Kingsbury) como crash testing industrial; retos esencial (invariante
    nueva + test), intermedio (propiedad delete_node+replay, caps. 27/28),
    experto (adaptador `GraphStore` para `MvccStore` o golden del modo script).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla /
    30 segundos / Una historia pequeña (Gray) / Mini-diálogo de guardia
    nocturna (suena la alerta del golden a las 3 a.m.). Retrieval practice
    obligado: reproducir DE MEMORIA las 4 invariantes y clasificar cuáles son
    visibles desde el puerto. Interleaving: cada ejercicio toca ≥2 capítulos.
    Glosario nuevo: property-based testing, shrinking, estrategia, golden
    master, contract test, fault injection, fail-stop, fuzz target.

---

## 7. Estilo y tono (consistencia con caps. 27-32)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés
  entre paréntesis la primera vez; salidas REALES de `cargo test` y del binario
  pegadas, nunca reconstruidas de memoria.
- **Diagramas**: la torre de riesgos (§4) y la clasificación textual de las
  invariantes; 1 tabla tipo↔pregunta↔módulo de LiraDB.
- **Spacing** (conceptos viejos que se EJERCITAN): puerto hexagonal (cap. 8),
  encoding/magic (cap. 9), autocommit (cap. 27), framing+CRC del WAL (cap. 28),
  `reabrir` ARIES (cap. 29), la frontera MVCC (cap. 30), la CLI testeable
  (cap. 31), roundtrips (cap. 32) y CSR (cap. 14).
- **Interleaving**: la propiedad de replay mezcla 27+28; el crash test mezcla
  9+28+29; un ejercicio mezcla 32+33 (invariantes tras importar CSV); el golden
  mezcla 31+32 (`export` a stdout dorado).
- **Dificultad asimétrica**: una idea nueva por sección (torre → contrato →
  propiedades → crash → golden → límites); los ejercicios exigen PREDECIR
  (¿qué test cazará esta mutación?) y recordar sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap33` (milisegundos),
  `cargo test -p liradb-cli --test golden_cli`, y `./scripts/verify.sh` como
  puerta ALL_GREEN — nunca «confía en mí».
- **Anécdota (única, verificada)**: Jim Gray, «Why Do Computers Stop and What
  Can Be Done About It?» (Tandem TR 85.7, 1985) — análisis de fallos de sistemas
  NonStop en producción y defensa de la inyección de fallos como rutina. Apoyo:
  Dijkstra (EWD249, 1970: «testing shows the presence, not the absence of
  bugs»); Claessen & Hughes (QuickCheck, ICFP 2000); Hamlet (Random Testing,
  Encyclopedia of Software Engineering, 1994); proptest Book; cargo-fuzz book;
  Mohan et al. (ARIES, TODS 1992); Cockburn (2005); pact.io; SQLite file
  format; Meszaros (2007); Zhou et al. (FoundationDB, SIGMOD 2021); jepsen.io.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y
  fuente (12 filas en §5).
- [x] Escenario de fallo visible, no solo happy path: cola corrupta tragada en
  silencio (hallazgo §5.8), bit-flip bajo CRC, magic podrido, mutaciones
  inyectadas en campos públicos que el verificador debe cazar.
- [x] Código ejecutable en workspace citado por nombre (IMPLEMENTADO
  2026-08-25: `cap33_pruebas.rs` (1.424 líneas), `tests/golden_cli.rs` +
  dorados, proptest 1.11.0 pineada; **809 tests** = 788 + 21 nuevos,
  verify.sh ALL_GREEN). Divergencias documentadas por el implementador:
  los dorados usan `["demo"]` y `["explain", Q]` sin `--graph` (clap lo
  rechaza en esos subcomandos); la propiedad de encoding opera sobre
  `Value` (la API pública del cap. 9), no grafo entero;
  `cargar_wal_estricta` devuelve `Result<Vec<WalRecord>,
  ErrorCargaEstricta>` con variante `Io` propia (WalError no tiene E/S y
  no se toca cap28); roundtrip CSV exige tipo fijo por clave y sin Null.
- [x] Misconcepciones corregidas explícitamente (§1: cinco, de «788 tests =
  probado» a «más tests sueltos = mejor»).
- [x] Ejercicios con solución verificable diseñados (retos N.10 mapeados a
  tests del workspace; esencial e intermedio con tesis nombrada).
- [x] ≥1 ejercicio de retrieval (invariantes de memoria, sin mirar) y spacing
  planificado (caps. 8/9/14/27/28/29/30/31/32 tocados; §7).
- [x] Responde la pregunta crítica del CORPUS (los 8 tipos, §3) y delimita
  criterion hacia el cap. 34 (§5.12) con el gancho «primero correcto, luego
  rápido».
- [x] Anécdota única verificada con fuente primaria (Gray, TR 85.7, 1985).
- [x] Alcance de código nuevo acotado y honesto (un módulo + un directorio
  tests/ + una dev-dependency; cero cambios en caps. previos; §5.4/5.8/5.11).
