# Capítulo 36 — Arquitectura final de LiraDB

> *«Lector, si buscas su monumento, mira alrededor.» — epitafio de Christopher Wren, catedral de San Pablo, Londres, 1723. Este capítulo hace exactamente ese gesto con LiraDB: el monumento no está escrito en estas páginas — está en `lib.rs`, alrededor tuyo.*

## 36.0 La anécdota de la esquina

Septiembre de 1666. El Gran Incendio ha borrado del mapa el Londres medieval: unas 13.000 casas y decenas de iglesias parroquiales reducidas a ceniza en cuatro días. De esa ruina sale encargado un hombre que no se hacía llamar arquitecto sino *surveyor*: Christopher Wren, profesor de astronomía, nombrado responsable de la reconstrucción. Durante las décadas siguientes levantó **52 iglesias** en la City — así lo cuenta hoy el National Churches Trust — y terminó su obra maestra, San Pablo, treinta y cinco años después de poner la primera piedra.

¿Cómo hace una sola persona para firmar medio centenar de edificios distintos? Con un **lenguaje común**: la iglesia-auditorio protestante — nave despejada para predicar y oír, tribunas laterales, torre con aguja coronando la perspectiva. Cada templo es diferente; todos hablan el mismo idioma. Mete un ciego de confianza en cualquiera de ellas y sabrá dónde está el altar, dónde la puerta, dónde el campanario. Esa legibilidad ES la arquitectura: no las piedras, sino el orden reconocible de las piedras.

Y cuando Wren murió, en 1723, no pidió un mausoleo. Su epitafio, grabado junto a su tumba bajo la cúpula de San Pablo, dice: *«Subtus conditur… lector, si monumentum requiris, circumspice»* — lector, si buscas su monumento, mira alrededor. El monumento es el edificio entero que tienes encima. Es exactamente el gesto de este capítulo, el último de código del volumen: después de veintinueve capítulos mirando piezas, vamos a mirar ALREDEDOR — los 28 módulos de `lib.rs` y la CLI — y comprobar si sabes señalar qué mantiene a LiraDB en pie.

## 36.1 Objetivo

El capítulo anterior cerró con una promesa: *los puertos medidos cierran el círculo hexagonal*. Aquí lo cerramos. Al terminar tendrás:

1. **El diagrama hexagonal final**: el hexágono de Cockburn dibujado COMPLETO — dominio puro en el centro, cinco puertos, adaptadores agrupados alrededor, frontal CLI fuera — cuadrado uno a uno con los módulos reales, y reconciliado con la vista vertical del brief.
2. **La tabla módulo↔rol**: los 28 módulos de `vol2-liradb` más el paquete `liradb-cli`, una fila por cada uno, verificada contra `code-map.yml`. Ningún módulo sin fila; ninguna fila sin módulo.
3. **El único artefacto nuevo del capítulo**: `crates/vol2-liradb/tests/arquitectura.rs` (509 líneas, 5 tests, integración desde fuera del crate) — la primera vez que la torre completa se construye en una respiración. Cero cambios en módulos `cap*`, cero dependencias nuevas, goldens intocados.
4. **La lectura de los agujeros del mapa**: componer el edificio entero destapó acoplamientos y pérdidas que ningún capítulo individual podía ver. Un mapa sin fronteras es marketing; este tiene las suyas a la vista.

La tesis que lo vertebra: **la arquitectura no fue un diagrama previo que cumplimos — fue tres decisiones sostenidas con disciplina durante veintinueve capítulos, y el mapa dibuja lo que AGUANTÓ**, no lo que prometimos.

## 36.2 Problema

Sabes cada pieza: el modelo del cap. 7, el puerto del 8, el pager del 12, el pool del 13, el WAL del 28… Pero pregúntate: ¿depende `Catalog` de `Pager`? Ante «¿dónde meterías ORDER BY?», ¿por dónde empezarías a responder? Si dudas, es normal — y es el síntoma exacto: treinta y cinco capítulos mirando LA PIEZA; ninguno mirando EL EDIFICIO. Desactivemos primero las cuatro ideas equivocadas que suelen venir con el tema:

1. **«La arquitectura es el diagrama que se dibuja ANTES de codificar.»** No. Aquí es un mapa que se valida DESPUÉS contra el código existente. Si el mapa y `lib.rs` discrepan, gana `lib.rs` — siempre.
2. **«El diagrama vertical del brief (CLI→Parser→Planner→Optimizer→Executor→Storage) ES la arquitectura.»** No: es UNA vista — el flujo de una consulta. Falta la vista de dependencias, y confundirlas lleva a preguntas mal planteadas y refactorings equivocados.
3. **«Un capítulo de arquitectura reorganiza el código.»** No: su entrega es comprensión VERIFICABLE. Renombrar o reagrupar módulos «para dejarlos bonitos» estropearía un motor sostenido por 848 tests — invalidaría justo lo que el mapa celebra.
4. **«Todo lo que aparece en el mapa funciona de punta a punta.»** No: leer el mapa incluye leer sus agujeros. Los cuatro que aparecieron al componer el edificio van en §36.8, con nombre y error.

## 36.3 Modelo mental: el monumento y sus dos vistas

Una construcción terminada se lee en DOS vistas complementarias. La vista RADIAL responde «¿quién depende de quién?» — es la vista de la construcción y de la evolución. La vista VERTICAL responde «¿por dónde pasa una consulta?» — es la vista del uso. Empieza por la segunda, porque ya la conoces: es la que el brief dibujó el día uno y que la CLI recorre en cada ejecución.

```text
VISTA VERTICAL — por dónde pasa UNA consulta (la vista del brief)

  'MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name'
     |
     v
  parse()       caps. 17-18    texto -> tokens -> AST
     |
     v
  lower()       cap. 19        AST -> LogicalPlan (binder ligando variables)
     |
     v
  optimize()    cap. 21        Catalog::collect(&store) + reglas -> mejor plan
     |
     v
  Executor      cap. 20        Volcano: open/next/close por operador
     |                             |
     v                             +--> habla con &dyn GraphStore (un PUERTO;
  ResultSet     filas -> la tabla que imprime la CLI     no con paginas ni disco)
```

Fíjate en la bifurcación final: el `Executor` NO habla con disco ni con páginas. Habla con un trait. Ese detalle, invisible en la vertical, es la columna vertebral del edificio — y es lo que solo muestra la otra vista. Para elegir el nivel de zoom usamos el modelo C4 de Simon Brown («Software Architecture for Developers»): ni el pájaro (contexto: LiraDB frente al mundo) ni el gusano (clases y líneas): el nivel **COMPONENTE** — piezas y dependencias. Veintinueve módulos y cinco traits caben en una página; los `impl` de Rust hundirían al lector en ruido sin añadir ninguna dependencia visible.

Regla de lectura: la vertical te sirve para EXPLICAR el sistema; la radial para DEFENDERLO y AMPLIARLO. Con una sola, o no sabes contar cómo funciona, o no sabes por dónde crece sin derribarlo.

## 36.4 Primera solución

¿Cómo describirías LiraDB a un compañero nuevo? Las dos versiones ingenuas — las que todos escribimos, incluido nuestro yo del capítulo 6:

**Versión lista**: abrir `src/lib.rs` y enumerar los 28 módulos con sus ficheros. «Aquí está el modelo, aquí el pager, aquí el WAL…» Responde QUÉ HAY. Y de hecho es media tabla de este capítulo — pero sola no distingue puerto de adaptador, ni dice quién puede hablar con quién, ni explica POR QUÉ puedes añadir PageRank sin tocar el parser.

**Versión diagrama único**: colgar el diagrama vertical del brief y darlo por completo. «CLI arriba, Storage abajo.» Responde CÓMO SE USA. Y de hecho es media vista de este capítulo — pero sugiere dependencias físicas inexistentes (que el Executor esté ENCIMA del almacenamiento, apilado) y oculta las reales (que todo pase por traits).

Las dos son honestas y las dos se quedan cortas por el mismo motivo: responden una pregunta cada una, y pretenden responderlas todas.

## 36.5 Sus límites

1. **La lista nivela todo.** En `lib.rs`, `cap07_modelo` y `cap28_wal` son dos líneas `mod` consecutivas; en realidad uno es el dominio puro y el otro un protocolo de durabilidad que reutiliza el formato del cap. 10. Sin capas, la lista no puede explicar por qué el WAL no tocó al parser cuando nació.
2. **La vertical miente sobre las dependencias.** Sugiere que `Executor` descansa físicamente sobre «Storage»: no hay tal cosa. El Executor recibe un `&dyn GraphStore`. La dependencia física real es la INVERSA de lo que el dibujo insinúa: el adaptador concreto vive detrás del puerto, no debajo del cliente.
3. **Produce preguntas mal planteadas.** «¿Mi consulta lee del disco?» — hoy no: vive en RAM detrás del puerto. «Si cambio el pager, ¿toca el optimizador?» — no: están separados por DOS anillos. Quien solo tiene la vertical adivina; quien tiene las dos vistas LEE.
4. **No escala al crecer.** Cuando llegue la Parte VIII (columnar, distribución), la lista tendrá 40 líneas y el diagrama vertical seguirá siendo UN camino entre muchos. Lo que aguanta el crecimiento no es la enumeración: es el mapa de fronteras.

## 36.6 Solución evolucionada: el hexágono completo

Aquí está. Es el hexágono de Cockburn que el cap. 8 abrió en miniatura — un mostrador y una cocina — ahora con TODAS las cocinas que veintinueve capítulos añadieron alrededor:

```text
                ┌─────────────────────────────────────────────┐
                │ FRONTAL: paquete liradb-cli (caps 31-35)    │
                │ demo query explain repl script import       │
                │ export · --graph --plan --stats --profile   │
                └─────────────────────┬───────────────────────┘
                                      │ única habitante externa
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ ADAPTADORES — viven DETRÁS de un puerto                                  │
 │ almacenamiento: MemoryStore(8) · FilePager(12) · BufferPool(13)          │
 │                 Csr(14) · HashIndex/BPlusTree(15)                        │
 │ fiabilidad:     Wal/WalTransaccion(28) · recuperación(29) · MvccStore(30)│
 │ formato/E/S:    encoding(9) · AppendOnlyLog(10) · SlottedPage(11)        │
 │                 inspect/check/compact(16) · CSV/JSONL/GraphML(32)        │
 │ observabilidad: Contadores · MedidorOperador · MedidorPaginas(35)        │
 └────────────────────────────────────┬─────────────────────────────────────┘
                                      │ implementan / envuelven
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ PUERTOS — los CINCO traits                                               │
 │ GraphStore(8) · Pager(12) · PhysicalOperator(20) ·                       │
 │ WeightSource(22) · Heuristic(23)                                         │
 └────────────────────────────────────┬─────────────────────────────────────┘
                                      │ hablan SOLO de dominio
 ┌────────────────────────────────────▼─────────────────────────────────────┐
 │ DOMINIO — compila sin depender de nadie                                  │
 │ Value · Node · Edge · PropertyGraph(7) · LiraQL AST(17)                  │
 └──────────────────────────────────────────────────────────────────────────┘

 TRANSVERSAL — el pipeline (caps. 17-21): parse -> lower -> optimize ->
 execute, montado SOBRE el puerto GraphStore; cruza todos los anillos en
 cada consulta.  ALGORITMOS (22-26): enchufados al MISMO puerto que la
 consulta.  CALIDAD (33-35): pruebas, benchmarks y observadores — lentes,
 no capas.
```

Recorrido por los anillos. **El centro** es dominio puro: `Value`/`Node`/`Edge`/`PropertyGraph` y el AST de LiraQL. Compila sin depender de nadie, y eso es lo que hace al motor TESTEABLE — la torre de pruebas del cap. 33 existe porque el centro no arrastra a nadie. **El primer anillo** son los cinco puertos: `GraphStore` (todo acceso a datos), `Pager` (todo acceso a páginas), `PhysicalOperator` (toda producción de filas), y `WeightSource`/`Heuristic` — los dos puntos donde el USUARIO aporta conocimiento del dominio: el peso de una arista, la estimación hacia el destino. Cinco traits sostienen veintinueve capítulos. **El segundo anillo** son los adaptadores, agrupados por función: almacenamiento, fiabilidad, formato/E-S y observabilidad. Regla heredada del cap. 8: cada adaptador vive DETRÁS de un puerto, y ninguno se conoce entre sí — `HashIndex` no sabe que `Wal` existe. **Arriba**, el frontal: la CLI es el único habitante externo. Y **atravesándolo todo**, el pipeline: parse→lower→optimize→execute no es una capa del hexágono, es una MÁQUINA montada sobre el puerto `GraphStore` — cruza todos los anillos en cada consulta. Por eso las dos vistas se necesitan: la vertical describe el recorrido de esa máquina; la radial describe quién depende de quién.

Y aquí está el momento ¡ajá!: **la arquitectura NO fue un diagrama previo que cumplimos. Fue TRES decisiones mantenidas con disciplina durante veintinueve capítulos** — y el mapa no dibuja lo que se prometió: dibuja lo que AGUANTÓ.

1. **EL PUERTO (cap. 8)**: todo acceso a datos pasa por `GraphStore`. Gracias a eso, la Parte V entera (algoritmos, caps. 22-26) creció ENCHUFADA al mismo trait que la consulta — sin tocar un cliente. El precio se midió en el cap. 34: correr sobre el puerto cuesta ×16 frente a la CSR cruda. Se pagó a sabiendas: es el coste de la frontera que permite el resto.
2. **FORMATOS ESTABLES (caps. 9-10)**: el encoding de valores y el framing con CRC32 se construyeron una vez para el log append-only… y el cap. 28 los REUTILIZÓ tal cual para el WAL. Fiabilidad nació sin inventar un solo byte de formato.
3. **EL PIPELINE (caps. 17-21)**: parse→lower→optimize→execute quedó definido como máquina sobre el puerto. Por eso el cap. 35 pudo envolverlo con medidores y spans SIN TOCAR NADA: instrumentar fue envolver puertos, jamás editar el motor.

Y el ¡ajá! honesto: el anillo físico está construido y probado (CSR persistente, índices sobre pools), pero la ruta de consultas HOY vive en RAM. Decirlo es parte de leer el mapa.

## 36.7 Código completo ejecutable: el inventario y el humo

Dos piezas. Primero, la tabla que convierte el dibujo en contrato comprobable — cada fila grepeable en `lib.rs` (segunda fuente: `code-map.yml`). Segundo, el único código nuevo del capítulo: `tests/arquitectura.rs`.

| Módulo (`lib.rs`) | Caps. | Anillo | Rol en una línea |
|---|---|---|---|
| `cap07_modelo` | 7 | Dominio | `Value`/`Node`/`Edge`/`PropertyGraph`: los datos puros, sin dependencias |
| `cap08_graph_store` | 8 | Puerto (+adaptador) | trait `GraphStore`: TODO acceso a datos; `MemoryStore` detrás |
| `cap09_encoding` | 9 | Formato/E-S | objetos↔bytes para strings y `Value`; lo heredará el WAL |
| `cap10_append_only` | 10 | Formato/E-S | `AppendOnlyLog`: length-prefix + CRC32, germen del framing |
| `cap11_slotted_pages` | 11 | Formato/E-S | `SlottedPage`/`PageHeader`: registros variables dentro de una página |
| `cap12_pager` | 12 | Puerto (+adaptador) | trait `Pager`: TODA página; `FilePager` sobre `std::fs` |
| `cap13_buffer_pool` | 13 | Almacenamiento | `BufferPool<P>`: frames, pinning, dirty tracking, Clock/LRU, métricas |
| `cap14_csr` | 14 | Almacenamiento | `Csr`/`PersistentCsr`: adyacencias comprimidas sobre `BufferPool<Pager>` |
| `cap15_indices` | 15 | Almacenamiento | `HashIndex`/`BPlusTree`: encontrar sin escanear, cada uno sobre SU pool |
| `cap16_mantenimiento` | 16 | Calidad de almacén | `inspect`/`check`/`compact`: stats, invariantes, compactación in-place |
| `cap17_liraql_ast` | 17 | Dominio | tokens, gramática EBNF, AST y validación semántica de LiraQL |
| `cap18_lexer_parser` | 18 | Pipeline | `parse()`: texto → tokens → AST (descendente recursivo a mano) |
| `cap19_plan_logico` | 19 | Pipeline | `lower()`: AST → `LogicalPlan` con binder y tipos lógicos |
| `cap20_volcano` | 20 | Puerto (+pipeline) | trait `PhysicalOperator`, los ocho operadores, `Executor`, `run()` |
| `cap21_optimizador` | 21 | Pipeline | `Catalog::collect` + `optimize` + `explain`: reglas y cardinalidades |
| `cap22_caminos_minimos` | 22 | Algoritmos (+puerto) | Dijkstra/Bellman-Ford sobre el puerto; `WeightSource` aporta pesos |
| `cap23_a_estrella` | 23 | Algoritmos (+puerto) | A* reutilizando el cap. 22; `Heuristic` aporta h(n) del usuario |
| `cap24_centralidad` | 24 | Algoritmos | centralidades + PageRank (global y personalizado) vía proyección |
| `cap25_comunidades` | 25 | Algoritmos | componentes, label propagation, modularidad Q, Louvain |
| `cap26_proyeccion` | 26 | Algoritmos | proyección ponderada, streaming por fronteras, presupuestos |
| `cap27_transacciones` | 27 | Fiabilidad | vocabulario ACID tipado; `Transaccion` con staging y commit |
| `cap28_wal` | 28 | Fiabilidad | `Wal`/`WalTransaccion`: write-ahead con el framing de los caps. 9-10 |
| `cap29_recuperacion` | 29 | Fiabilidad | ARIES simplificado: analizar/redo/deshacer, `reabrir`, checkpoint |
| `cap30_mvcc` | 30 | Fiabilidad | `MvccStore`: versiones por elemento y snapshots (API propia, NO `GraphStore`) |
| `cap32_import_export` | 32 | Frontera E/S | CSV/JSONL/GraphML en streaming sobre el puerto |
| `cap33_pruebas` | 33 | Calidad (lente) | torre de pruebas: invariantes y regresiones invertidas |
| `cap34_benchmarks` | 34 | Calidad (lente) | harness de percentiles; midió ×16 y ×794 |
| `cap35_observabilidad` | 35 | Calidad (lente) | `Contadores`, medidores, trazas: envuelve puertos sin tocar el motor |
| paquete `liradb-cli` | 31+ | Frontal | binario `liradb`: demo/query/explain/repl/script/import/export + flags |

Honestidad de inventario: el cap. 31 no vive en `lib.rs` — vive en el PAQUETE `liradb-cli`. Un capítulo de síntesis no retoca nada para que la tabla «cuadre»: la tabla cuadra porque describe lo real, incluida esta asimetría.

Ahora el humo. La cabecera del test demuestra la API pública plana — el usuario ve UN crate, no 28:

```rust
// crates/vol2-liradb/tests/arquitectura.rs (integración: FUERA del crate)
use vol2_liradb::{
    AntesImagenes, BPlusTree, BufferPool, Catalog, Cell, Contadores, Csr, Edge,
    EuclideanHeuristic, Executor, FilePager, GraphStore, HashIndex, MemoryStore,
    MvccStore, Node, NodeScanOp, Operacion, PAGE_SIZE, Pager, PersistentCsr,
    PhysicalOperator, PolicyKind, Value, Wal, WalTransaccion, WeightSource,
    ZeroHeuristic, a_star, derivar_contadores, dijkstra, exportar_jsonl,
    guardar_wal, importar_jsonl, lower, metricas_consulta, optimize, page_rank,
    parse, reabrir, run, verificar_invariantes,
};
```

La tesis principal construye la torre completa en una respiración — cada bloque de comentarios del test cita SUS capítulos:

```rust
#[test]
fn la_torre_completa_se_construye_en_una_respiracion() {
    // Anillo DOMINIO + PUERTO (caps. 7-8): MemoryStore detrás de &dyn GraphStore…
    // FIABILIDAD (cap. 28): toda escritura pasa por WalTransaccion::commit…
    // PIPELINE (caps. 17-21): parse -> lower -> Catalog::collect -> optimize
    //   -> Executor -> ResultSet == ["Bob", "Carla"]
    // OBSERVABILIDAD (cap. 35): derivar_contadores desde ExecMetrics…
    // RECUPERACIÓN (cap. 29): guardar_wal + reabrir renacen 6 nodos/6 aristas
    //   en un store FRESCO; verificar_invariantes (cap. 33) da el visto bueno.
    // MVCC (cap. 30): snapshot que NO ve el commit posterior…
}
```

El segundo test baja por el anillo físico — cadena estricta hacia abajo, sin saber que LiraQL existe:

```rust
#[test]
fn la_pila_fisica_encadena_pager_pool_csr_e_indices() {
    let pool_csr = BufferPool::with_policy(
        FilePager::create(dir.path().join("csr.bin"))?, 16, PolicyKind::Clock);
    let mut persistente = PersistentCsr::create(pool_csr)?;
    persistente.replace(&csr_ram)?;            // CSR de RAM -> paginas
    let recargado = persistente.load()?;       // paginas -> RAM, adyacencia igual
    // HashIndex y BPlusTree, cada uno sobre SU pool (LRU aqui, Clock alla):
    // la propiedad del pool resuelve la convivencia de vecinos.
}
```

Y el test de inventario afirma los cinco puertos EN POSICIÓN de trait object — `&dyn GraphStore`, `Box<dyn Pager>`, un `NodeScanOp` movido a mano por su ciclo Volcano, `dijkstra` con `WeightSource`, `a_star` con dos `Heuristic` — fallaría AL COMPILAR si un trait renombrara, y EN EJECUCIÓN si un adaptador dejara de cumplirlo.

## 36.8 Prueba de fuego

Milisegundos, no minutos — determinista por construcción: sin tiempos, sin hilos, sin goldens:

```text
$ cargo test -p vol2-liradb --test arquitectura

running 5 tests
test cada_puerto_tiene_su_adaptador_canonico ... ok
test el_formato_jsonl_sobrevive_al_roundtrip_del_puerto ... ok
test los_algoritmos_corren_sobre_el_mismo_puerto_que_la_consulta ... ok
test la_pila_fisica_encadena_pager_pool_csr_e_indices ... ok
test la_torre_completa_se_construye_en_una_respiracion ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Cinco verdes, 0.00 s. Junto a ellos, el marcador del volumen: **848 tests** (843 previos + 5 nuevos) ALL_GREEN, goldens byte-exactos intactos. La torre completa se construye de una respiración — algo que NINGÚN capítulo anterior había hecho: cada uno probó SU piso; este probó el edificio.

Después, el recorrido guiado: releer la salida de `--profile` del cap. 35 como viaje por el hexágono (duraciones ilustrativas — varían entre máquinas; lo pactado son nombres y anidamiento):

```text
query                           <- FRONTAL: la CLI instala subscriber y cronometra
├─ parse                         <- TRANSVERSAL: caps. 17-18
├─ plan                          <- TRANSVERSAL: caps. 19/21
└─ execute                       <- TRANSVERSAL: cap. 20
   └─ Project                    <- operador tras el puerto PhysicalOperator
      └─ Filter                     (cada uno habla con &dyn GraphStore)
         └─ Expand
            └─ NodeScan
Contadores: nodes_scanned 4 · relationships_expanded 4   <- LENTE (cap. 35),
                                                            derivada de ExecMetrics
```

Cada span ES un anillo. Y el nivel que NO aparece — `storage_read` bajo el `NodeScan` — también es lectura del mapa: sin `DiskStore` tras el puerto, la ruta de consulta no baja a páginas. El mapa marca la frontera en lugar de fingirla.

### Los agujeros del mapa

Componer el edificio entero — algo que ningún capítulo hizo antes — destapó lo que cada pieza por separado escondía. Cuatro hallazgos, todos citables en `tests/arquitectura.rs`:

1. **Acoplamiento oculto entre adaptadores vecinos.** `HashIndex::create` fallaba con `UnknownPage(2)` si el pool tenía menos frames de los que el índice necesitaba: hace flush de TODAS sus páginas al crear, y `BufferPool::flush_page` exige RESIDENCIA. Invisible durante todo el cap. 15 porque sus tests usaban un pool holgado. Moraleja: los anillos son limpios, pero DOS adaptadores del mismo anillo tenían un contrato sin escribir — y solo emerge al componer con configuraciones adversas. *(Estado posterior, 2026-08-26: reparada tras el cierre — hoy `create` valida EAGER con la variante nueva `IndexError::CapacidadInsuficiente { requerida, disponible }` en vez del críptico `UnknownPage(2)`; y el mínimo REAL resultó ser `1 + num_buckets`, no la estimación conservadora aquí escrita: las páginas 0 y 1 van directo al pager y nunca pasan por el pool durante el create.)*
2. **Pérdida silenciosa en el roundtrip JSONL.** `Float(2.0)` se serializa como `"2"` y reimporta como `Int(2)`: el formato del cap. 32 pierde floats enteros sin avisar. Por eso el grafo mini del smoke usa pesos fraccionarios (2.5, 3.5…) — y por eso el test de roundtrip compara elemento a elemento, incluyendo TIPOS. *(Reparada tras el cierre, 2026-08-26: serialización con formato Debug (`{:?}`) — Float(2.0) sale `"2.0"` y reimporta Float(2.0); compatibilidad atrás intacta («2» sigue importando Int) y cero cambios de goldens.)*
3. **Un lector descuidado ve su propio futuro.** El reloj MVCC es pre-incremento arrancando en 1: tras un commit, `reloj()` devuelve el ts que el SIGUIENTE commit va a TOMAR. El snapshot correcto del lector es `ResumenCommitMvcc.ts_asignado` del commit que quiere ver. El smoke lo codifica: `iter_nodos(ts_asignado)` ve 5 nodos; `iter_nodos(reloj())` ve 6.
4. **La recuperación SOLO renace lo confirmado.** Autocommits previos al WAL no sobreviven a `reabrir` (el redo falla limpiamente con `InvalidEdgeEndpoints` sobre un store vacío). Es la regla de oro del cap. 28 con evidencia nueva: el store NO es tu durabilidad — el log SÍ.

## 36.9 Qué hemos sacrificado

```text
Lo que SÍ está en el mapa:  28 modulos + CLI, cuadrados uno a uno; 5 puertos con
                            adaptador canonico instanciado en test; pipeline,
                            algoritmos y lentes verdes en 0.00 s; 848 ALL_GREEN;
                            catálogo O(n²) REPARADO post-cierre (2026-08-26)
Lo que AUN NO:              DiskStore detras de GraphStore (la ruta vive en RAM);
                            LIMIT/agregación en LiraQL; write skew cerrado;
                            cola corrupta eliminada
```

1. **El mapa referencia, nunca repite.** Cada sección remite a SU capítulo; quien quiera el CÓMO de cada pieza, vuelve a él. Este capítulo enseña a LEER el edificio, no a reexplicarlo.
2. **Deuda técnica nombrada, jamás reparada dentro del volumen — y una saldada justo después.** El catálogo cuadrático de `Catalog::collect` (~224 s frente a 281 ms según MIGRATION-PATTERN §39) quedó señalizado aquí y PAGADO tras el cierre (2026-08-26): O(V²)→O(V), ~224 s → ~3-4 s (**×68**) sobre el dataset de referencia — sin la medición de este capítulo nadie habría sabido qué reparar ni cómo verificar el arreglo. Siguen vivas: no existe `GraphStore` respaldado por disco end-to-end; LiraQL no expone LIMIT ni agregación (aunque `LimitOp`/`DistinctOp` esperan en el cap. 20); el write skew atraviesa el snapshot MVCC (frontera del cap. 30); la cola corrupta del WAL está mitigada, no eliminada (`cargar_wal_estricta`, cap. 33). Reparar cualquier cosa «de paso» sería scope creep — y rompería el argumento de estabilidad.
3. **Qué haríamos diferente, con cifras**: `out_edges()` clona un `Vec` por llamada — los ×16 del cap. 34 son su factura, y justifican las proyecciones del cap. 26; el autocommit implícito gobernó siete partes antes de que el cap. 27 diera vocabulario para nombrarlo; `eq_push` del optimizador es lineal donde podría indexar. Nada de esto es secreto: está medido y fechado.
4. **Sin despliegue ni procesos.** El mapa cubre el MOTOR; qué significa «producción» es el siguiente capítulo, no este.
5. **Sin golden nuevo.** Lo determinista ya está dorado (caps. 31-35); el smoke es determinista por naturaleza. La puerta de calidad es `--test arquitectura` + ALL_GREEN, nunca «confía en mí».

## 36.10 Cómo lo hace una BBDD real

Documentar la propia arquitectura es un género con clásicos — y LiraDB acaba de escribir el suyo en miniatura. **SQLite** mantiene el documento canónico del género: «Architecture of SQLite» (sqlite.org/arch.html), con su diagrama de stack — tokenizer, parser, generador de código, VDBE, b-tree, pager, VFS — donde cada caja corresponde a ficheros concretos. Es exactamente nuestro gesto: un mapa que grepea. **PostgreSQL** documenta sus internos en la documentación oficial (Part VII, «Internals», con el capítulo «Overview of PostgreSQL Internals»): backend, parser, planner/optimizer, executor y acceso a storage — niveles separados para audiencias separadas, la idea C4 aplicada por un proyecto con treinta años. **Kùzu** (el GDBMS embebido del paper publicado en VLDB) describe sus componentes — ejecución vectorizada, buffer manager, catálogo, WAL — en el propio artículo académico: el mapa como parte del contrato científico. Y **DuckDB** publica en sus docs una página de arquitectura que explica POR QUÉ su motor vectorizado difiere del tuple-at-a-time: el mapa como argumento, no solo como inventario. La diferencia entre estos documentos y el nuestro no es conceptual: es escala y décadas de rodaje. El gesto — dibujar el edificio y cuadrarlo contra el código — es el mismo.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (recordar — retrieval practice): cierra el libro y dibuja DE MEMORIA el hexágono con sus cuatro bandas, colocando diez componentes sin pistas — por ejemplo: `MemoryStore`, `BPlusTree`, `WalTransaccion`, `IndexSeekOp`, `EuclideanHeuristic`, `SuscriptorArbol`, `SlottedPage`, `Catalog`, `PageRank`, `MedidorPaginas`. Autocorrígete contra §36.6: cada componente tiene exactamente UNA banda correcta. Si dudaste en alguno, ése es tu hueco de estudio.
- *Intermedio* (analizar): ¿qué ROMPERÍA si `GraphStore` expusiera páginas? Escríbelo con ejemplo concreto del código: los doce o más consumidores del puerto — CSR, índices, Dijkstra, PageRank, Louvain, el Executor… — pasarían a depender del formato físico; un `Csr` en RAM o un store de tests dejarían de compilar sin un pager debajo. Contrasta con el cap. 14: bajó a páginas detrás de SU PROPIA costura (`BufferPool<Pager>`), sin contagiar a nadie.
- *Experto* (crear): añade al mapa un componente IMAGINARIO — `DiskStore` u `OrderByOp` — y enumera: qué puertos implementa o consume, qué módulos vecinos usa, qué NO puede tocar, y cómo extenderías `tests/arquitectura.rs` para afirmarlo (¿un sexto test? ¿una aserción nueva en `cada_puerto_tiene_su_adaptador_canonico`?). Después revisa §36.9: acabas de descubrir por qué esas piezas están en la lista de deuda.

## 36.11 Lo que te llevas

- **Dos vistas, un edificio**: la radial (¿quién depende de quién?) para defender y ampliar; la vertical (¿por dónde pasa una consulta?) para explicar. Ninguna basta sola.
- **La arquitectura es lo que aguantó**: tres decisiones — el puerto (cap. 8), los formatos estables (caps. 9-10), el pipeline (caps. 17-21) — sostenidas veintinueve capítulos. El mapa dibuja lo sobrevivido, no lo prometido.
- **Cinco puertos sostienen veintinueve capítulos**: `GraphStore`, `Pager`, `PhysicalOperator`, `WeightSource`, `Heuristic`. Todo lo demás vive DETRÁS de alguno.
- **El pipeline es transversal, no una capa**: cruza todos los anillos en cada consulta, montado sobre el puerto de datos.
- **Inventario o marketing**: 28 módulos + CLI, una fila por módulo real. Si un módulo no tiene fila, el mapa está MAL — misma honestidad que el recibo del cap. 35.
- **Leer el mapa incluye leer sus agujeros**: acoplamiento hash↔pool, float-entero perdido en JSONL, reloj MVCC pre-incremento, recuperación solo-de-lo-confirmado.
- **El nivel de zoom importa**: componente (C4), ni pájaro ni gusano.
- **Síntesis sin cirugía**: cero cambios en módulos previos, un solo artefacto nuevo de integración, 848 verdes. Comprender es la entrega; tocar, la tentación evitada.

## 36.12 Ojo, cuidado con…

- **Confundir las dos vistas.** «El Executor está ENCIMA del Storage» — no: habla con un TRAIT. Refactoring basado en la vertical sola mueve muebles equivocados.
- **Dibujar el mapa aspiracional.** Si un nombre del diagrama no grepea en `lib.rs`, sobra; si falta un módulo, falta fila. El mapa que discrepa del código pierde — y debe perder.
- **Retocar módulos «para dejarlos bonitos» en un capítulo de síntesis.** Cada renombrado cosmético invalida 843 tests como argumento de estabilidad. La síntesis mira; no opera.
- **Creer que lo dibujado funciona de punta a punta.** El anillo físico existe y está probado; la ruta de consultas sigue en RAM. Ambas cosas son verdad a la vez — el mapa las dice ambas.
- **Olvidar que el cap. 31 vive en la CLI.** El frontal no es un módulo de la lib; es OTRO paquete. La tabla lo declara; tu mapa mental debería también.
- **Leer el hexágono como capas rígidas.** El pipeline atraviesa los anillos; los algoritmos y las lentes cuelgan del MISMO puerto que la consulta. No hay «pisos» por donde subir: hay fronteras que cruzar conscientemente.

## 36.13 Pin de batalla

> *«La arquitectura no es el diagrama que se dibuja antes de codificar: es lo que aguantó. Cinco puertos sostienen veintinueve capítulos — y un mapa solo es arquitectura si cuadra, fila a fila, con el código que dice describir.»*

## 36.14 Si solo lees 30 segundos

Capítulo de síntesis: cero cambios de código, un artefacto nuevo — `tests/arquitectura.rs` (5 tests, 0.00 s) que construye la torre completa: dominio+puerto → pipeline → WAL → recuperación → invariantes → MVCC → contadores, más la pila física FilePager→BufferPool→CSR/índices. Dos vistas: hexágono radial (dominio puro; cinco puertos: `GraphStore`/`Pager`/`PhysicalOperator`/`WeightSource`/`Heuristic`; adaptadores agrupados; frontal CLI) + vertical de flujo (parse→lower→optimize→execute, transversal sobre el puerto). Tabla módulo↔rol: 28 módulos + CLI, uno a uno. Tres decisiones lo explican: el puerto (×16 de coste asumido), los formatos estables (el WAL reutilizó framing+CRC32), el pipeline. Agujeros nombrados: hash↔pool (`UnknownPage`), float entero perdido en JSONL, reloj MVCC pre-incremento, recuperación solo de lo confirmado. Deuda visible, no fingida: catálogo O(n²) (reparada ×68 post-cierre), sin DiskStore, sin LIMIT, write skew. 848 tests ALL_GREEN.

## 36.15 Una historia pequeña

Mayo de 1851, Hyde Park, Londres. Mientras un comité rechazaba los 245 proyectos clásicos para la Gran Exposición, un jardinero — Joseph Paxton, jefe de jardines del duque de Devonshire — garabateó su idea sobre un papel secante y la envió con los días contados. Su Palacio de Cristal era una caja de hierro y vidrio de 1.851 pies (564 metros) de largo — la cifra coincide con el año — erigida en unas cinco semanas por los cristaleros y en unos cinco meses de obra total. El truco no era el vidrio: eran las JUNTAS. Paxton diseñó todo sobre un módulo dictado por el mayor panel estándar que la fábrica Chance Brothers podía fundir — unos 293.000 paneles idénticos sobre celosías prefabricadas — y los constructores Fox y Henderson atornillaron piezas fabricadas en serie usando, por primera vez a esta escala, tuercas y tornillos de rosca estandarizada (la futura Whitworth): hasta entonces cada tornillo era único y NO intercambiable. Piezas estándar + juntas estándar = un edificio de medio kilómetro montado en meses. Y hay remate: en 1852 el palacio se DESMONTO pieza a pieza y se volvió a erigir, más grande, en Sydenham (ardería en 1936). Solo un edificio cuyo inventario y juntas estaban documentados podía permitírselo. Moraleja para LiraDB: los puertos son tus roscas Whitworth — por eso la Parte VIII podrá desmontar y reerigir piezas enteras sin demoler nada.

## Ejercicios resueltos

**1. Coloca cada pieza en su anillo (interleaving caps. 10, 20, 25, 35).** CRC32 → formato/E-S: vive en el framing del cap. 10 y reaparece en el WAL (cap. 28), ambos adaptadores de formato. `ExpandOp` → pipeline transversal: es un operador del cap. 20 que produce filas tras el puerto `PhysicalOperator`, hablando con `&dyn GraphStore`. Louvain → algoritmos: cap. 25, enchufado al MISMO puerto que la consulta, sin conocer al parser. `MedidorPaginas` → lente de calidad: decorador del cap. 35 que ENVUELVE el puerto `Pager` — no es capa, es voltímetro. Criterio: si dudaste entre dos anillos, pregúntate «¿implemento un puerto, consumo el puerto de datos, o envuelvo un puerto?».

**2. ¿Depende `Catalog` de `Pager`?** No. `Catalog::collect` toma `&dyn GraphStore` (cap. 21): recolecta estadísticas A TRAVÉS del puerto de datos. El `Pager` está DOS anillos más abajo, detrás de la cadena de adaptadores, y el catálogo no sabe que existe. Prueba ejecutable: la tesis del smoke construye `Catalog::collect(&store)` sobre un `MemoryStore` sin ningún pager en escena. Ésta era la pregunta-síntoma del §36.2 — y fíjate en el método: no la respondimos discutiendo, la respondimos GREPEANDO y compilando.

**3. ¿Dónde meterías ORDER BY?** Usa el mapa como herramienta de diseño, no como adorno. Respuesta guiada por las dos vistas: (a) un operador nuevo `SortOp` que implemente `PhysicalOperator` junto a sus hermanos `LimitOp`/`DistinctOp` del cap. 20 — que YA existen esperando sin gramática que los invoque; (b) la palabra clave nueva en el vocabulario del cap. 17 y el parser del 18; (c) su bajada a plan en el cap. 19. Puertos: ninguno nuevo. Adaptadores: ninguno tocado. Una feature bien colocada se reconoce porque el diff cae ENTERO en el anillo transversal. (Nota: LIMIT/agregación siguen siendo deuda declarada en §36.9 — diseñarla así es exactamente el ejercicio que la Parte VIII hará con piezas mayores.)

## Ejercicios propuestos

**Esencial (recordar — retrieval).** Sin mirar §36.6: escribe de memoria los CINCO puertos y, junto a cada uno, su adaptador canónico. Verifica contra el test de inventario (`cargo test -p vol2-liradb --test arquitectura` — el test que fallaría al compilar si un trait renombrara). Criterio: cinco parejas correctas SIN releer; el nombre del adaptador importa tanto como el del trait.

**Intermedio (interleaving 13+15+36: predecir).** Del hallazgo 1 de §36.8 —reparado tras el cierre—: hoy `HashIndex::create` valida EAGER con `IndexError::CapacidadInsuficiente { requerida, disponible }`, y el mínimo REAL es `pool.capacity() >= 1 + num_buckets` (página 2 de catálogo + un frame por bucket; las páginas 0 y 1 van directo al pager y NUNCA pasan por el pool durante el create). ANTES de tocar código: con 8 buckets, predice qué pasa con capacidad 9 (¿funciona? ¿por qué basta exactamente esa?) y con capacidad 8 (¿qué variante de error, con qué valores en sus campos?). Explica en una frase por qué el error antiguo era críptico (`UnknownPage(2)` no dice nada de capacidad) y qué información nueva aporta la variante nueva. Verifica con un test propio sobre tempfile modelado en `la_pila_fisica_encadena_...`, y remata explicando por qué el cap. 15 nunca lo vio (pista: ¿qué pool usaban SUS tests?).

**Intermedio (interleaving 30+36: derivar).** Del hallazgo 3: dos commits MVCC asignan ts 1 y ts 2. Sin ejecutar nada, responde: ¿qué devuelve `reloj()` justo después? ¿Qué snapshot debe anclar un lector que quiere ver EXACTAMENTE el estado tras el primer commit? ¿Qué vería un lector descuidado que anclara `reloj()`? Verifica con un test modelado en el bloque MVCC de la tesis principal. Criterio: la frase «un lector descuidado ve su propio futuro» explicada con ts concretos.

**Experto (crear).** Convierte un agujero en documentación ejecutable: escribe `tests/arquitectura_recuperacion.rs` (o un sexto test) demostrando el hallazgo 4 — un store poblado por autocommits SIN WAL no renace tras `reabrir`, mientras que el mismo contenido pasado por `WalTransaccion` renace íntegro. Restricciones del capítulo: cero cambios en módulos `cap*`, integración desde `tests/`, errores tipados afirmados (no `.unwrap()` a ciegas). *Verificación*: `cargo test` verde y el comentario del test citando el cap. 28. *Criterio*: el test fallaría si alguien «arreglara» la recuperación para revivir lo no confirmado — documentación que muerde.

## Para profundizar

- **Alistair Cockburn, «Hexagonal Architecture (Ports and Adapters)» (2005)** — el marco que el cap. 8 adoptó y este capítulo dibuja completo; la misma voz, treinta capítulos después.
- **Simon Brown, «Software Architecture for Developers» y el C4 model (c4model.com)** — context/container/component/code: la justificación del nivel de zoom COMPONENTE y de mantener vistas separadas para audiencias separadas.
- **«Architecture of SQLite» (sqlite.org/arch.html)** — el documento canónico del género: stack dibujado, cajas grepables. Tu próxima lectura si este capítulo te gustó.
- **PostgreSQL, documentación oficial, Part VII «Internals»** (incluido «Overview of PostgreSQL Internals») — la arquitectura documentada por niveles de un motor veterano.
- **Xiyang Feng et al., «Kùzu: a vectorized property graph database management system» (VLDB 2024)** — un GDBMS moderno publicando su propio mapa de componentes; atribución clean-room según el Colofón.
- **DuckDB, «Why DuckDB» y docs de arquitectura (duckdb.org)** — el mapa como argumento: por qué vectorizado y no tuple-at-a-time.
- **Adrian Tinniswood, «His Invention So Fertile: A Life of Christopher Wren» (2001); National Churches Trust («Wren rebuilt 52 churches in the City of London»)** — fuentes de la anécdota; el epitafio se visita in situ en San Pablo.
- **Hermione Hobhouse, «The Crystal Palace and the Great Exhibition» (Athlone, 2002); Kate Colquhoun, «A Thing in Disguise: The Visionary Life of Joseph Paxton» (2003)** — fuentes de la historia pequeña: módulo del panel, prefabricación, dos vidas del palacio.
- Dentro del libro: cap. 6 (este capítulo es su hermano mayor: los 5 pilares PROMETIDOS frente a lo AGUANTADO), cap. 8 (el primer puerto), caps. 34-35 (las cifras y los ojos del mapa).

## Mini-diálogo: en guardia nocturna

> — Tres de la madrugada. Turno de guardia. Y… nada. Ni un pager, ni un dashboard en rojo, ni una cola creciendo. ¿Esto no te pone nervioso?
>
> — Ayer habría dicho que sí. Hoy no. ¿Sabes qué tienes que anoche no tenías?
>
> — ¿El… mapa? Acabo de imprimir el hexágono.
>
> — Entonces úsalo. ¿Por qué CREES que no suena nada?
>
> — Porque… la consulta de ayer quedó arreglada: el índice entra en el plan, el recibo baja a `nodes_scanned 4`…
>
> — Eso es lo de ayer. Baja un piso. ¿Por qué PUDO arreglarse en quince minutos?
>
> — Porque el perfil señaló el `NodeScan`… y porque el índice ya estaba ahí, enchufado al mismo puerto que usa la consulta, sin tocar nada más…
>
> — Exacto. Esta noche tranquila no es suerte: es estructura. Cada capa se puede MEDIR (los contadores del 35), cada fallo tiene TEST (la torre del 33), y cada pieza vive detrás de un puerto que se cambia sin demoler (el 8). Los sistemas ruidosos son los que no puedes mirar. Este se deja mirar entero — por eso calla.
>
> — ¿Y si mañana suena?
>
> — Sonará, como siempre. Pero abrirás el mapa, pondrás el dedo en un anillo, y sabrás por dónde empezar ANTES de abrir un solo log. Eso es lo que Wren les dejó a los londinenses: no piedras — un lenguaje. Duerme: el edificio aguanta.

---

*(Próximo capítulo: 37 — el mapa está completo, y un mapa sirve para CRECER con criterio. Parte VIII: ¿qué necesitaría LiraDB para producción?, ¿dónde se enchufaría el almacenamiento columnar y la ejecución vectorizada?, ¿joins peor-caso-óptimos y consultas recursivas?, ¿qué partes del hexágono sobrevivirían a repartir el motor entre máquinas? Cada capítulo empezará señalando un punto del mapa que acabas de aprender a leer.)*
