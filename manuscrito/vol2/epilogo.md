# Epílogo — Ya sabes construir una base de datos

## Lo que hay sobre tu mesa

Este volumen empezó con una pregunta incómoda:
¿qué convierte una estructura de datos en una base de datos?

No la respondimos con teoría; la respondimos construyendo.
Cuarenta capítulos después, tienes sobre la mesa a **LiraDB**:
unas 25.000 líneas de Rust que solo usan la biblioteca estándar
— cero dependencias de runtime — sostenidas por 892 tests en verde.

## Los hitos del viaje

El primero fue el instante exacto en que tu grafo dejó de caber en memoria:
descubriste que *representar* ya es diseñar (Parte I).

Luego vino la primera consulta completa,
de `MATCH` a filas, atravesando lexer, parser, binder, plan lógico y motor Volcano (cap. 20).
Antes tenías un almacenamiento; desde entonces, una base de datos.

Poco después, el primer commit durable:
el marcador escrito en el WAL *antes* de aplicar cambios,
y ARIES rejugando la historia entera tras un crash simulado (caps. 28-29).
Y el día que `liradb explain` te mostró el plan optimizado (cap. 21),
viste por primera vez al motor *pensar*.

Los dos números que recordarás siempre llegaron al final:
el ×63 del almacenamiento columnar y el ×29 de WCOJ sobre un grafo hub-skew (caps. 38-39):
el layout manda y el skew factura.

Y el clúster Raft girando con tics lógicos (cap. 40):
elecciones, AppendEntries y compromiso por mayoría,
deterministas hasta el último paso, sin una sola hebra de red real.

¿Cómo se llegó hasta ahí? Las Partes I y II pensaron el grafo y lo hicieron modelo:
Property Graph con `Value` tipado, el trait `GraphStore` como frontera hexagonal,
un formato binario con magic number y CRC, un log append-only.
La Parte III le puso músculo físico:
slotted pages, pager, buffer pool Clock/LRU con métricas,
el CSR persistente — hallazgo ×16 frente al puerto ingenuo —,
índices hash y B+ tree, compactación con `inspect | check | compact`.

La Parte IV construyó LiraQL *antes* de escribir su código,
parser descendente a mano incluido,
con el bug del binder conservado a propósito como caso de estudio.

La Parte V cargó algoritmos sobre el grafo persistente:
Dijkstra y Bellman-Ford con semántica estricta de pesos, A* con heurísticas tuyas,
centralidad y PageRank — con el eigenvector como honesto «antes» —,
Louvain determinista y presupuestos de streaming que se miden, no se prometen
(la tesis del test 2/499 sigue ahí).

La Parte VI veló porque nada se perdiera:
transacciones con vocabulario ACID sin maquillaje,
MVCC con snapshots y niveles de aislamiento,
y el `GrafoEspera` cobrando puntual la deuda del cap. 30.

La Parte VII abrió las puertas de operación:
REPL de `liradb`, import/export streaming en CSV, JSONL y GraphML,
torre de pruebas (invariantes, proptest, goldens),
benchmarks con dataset determinista donde el catálogo cuadrático
quedó *documentado*, no parcheado,
observabilidad con `--profile`,
y un smoke test que recorre el edificio entero de arriba abajo.

Y la Parte VIII levantó la vista:
particionado hash contra comunidad con la factura de cortes delante (−40,7 %),
vertex-cut para domar al hub, consenso Raft por tics.
De un `Vec<Vec<usize>>` a un sistema distribuido emulable en tu portátil.

## Lo que LiraDB NO hace

Si has llegado hasta aquí, ya conoces la honestidad de la casa:
`informe_acid()` admite qué propiedades ACID no cumplimos,
y `informe_produccion()` resume con un 0·6·5 lo que falta para producción.

No hay red real, no hay transacciones distribuidas,
el catálogo escala cuadráticamente y lo sabemos,
y así está escrito en el código y en estas páginas.
Que sea deliberado no lo hace menos serio:
es la decisión pedagógica central del libro.

Un motor que *finge* ser de producción esconde sus decisiones detrás del marketing;
uno didáctico las muestra con nombre, número y test.
Cuando abras la documentación de PostgreSQL o el código de SQLite,
vas a reconocer cada pieza: el buffer pool, el WAL, el planificador.
Ese reconocimiento es el objetivo. Enseñar vale más que fingir.

## Qué queda por hacer — y cómo tú puedes seguir

El proyecto vive en <https://github.com/Rubentxu/grafos-bbdd-desde-cero>:
issues y PRs bienvenidos.
Las deudas documentadas son ejercicios reales esperando a alguien con tiempo:

- `Catalog::collect` es cuadrático:
  hazlo incremental y mide el antes/después en los benchmarks del cap. 34.
- El `HashIndex` nunca crece: implanta rehashing por factor de carga.
- El importador JSONL decide hoy de forma conservadora qué hacer
  con floats que parecen enteros:
  define la semántica que crees correcta y defiéndela con tests.

Más allá quedan los frentes abiertos del cap. 40:
paralelizar LeapFrog morsel-driven, orden dinámico de variables,
compactación y snapshots del log replicado,
o sustituir los tics por un runtime asíncrono con red de verdad.

Y luego está la puerta siguiente.
El Vol.III — «Grafos en la era de la IA» — empieza justo donde este cierra:
KB-Lira, GraphRAG, la memoria de un agente que necesita un grafo.
Lo que aquí construiste pieza a pieza se convertirá allí
en infraestructura de sistemas que razonan.

## Carta al lector

Te escribo esto el día que el volumen se cierra, y quiero decírtelo sin rodeos:
usar una base de datos y entender una base de datos
son dos habilidades distintas,
y solo una de ellas sobrevive a las 3 de la madrugada de un incidente.

Ningún tutorial te da lo que te ha dado este viaje.
Los tutoriales te muestran el resultado;
construir desde cero te obliga a habitar cada decisión:
por qué el commit marker va antes del apply,
por qué el eigenvector era un «antes» honesto y no un fracaso,
por qué un ×63 puede salir de cambiar un layout sin tocar un algoritmo.
Ese conocimiento no se descarga; se gana sudando tests.
Si llegaste hasta aquí, ya no miras las bases de datos igual.
Abres un `EXPLAIN` ajeno y ves un plan;
ves un log y piensas en recuperación;
escuchas «distribuido» y preguntas por cortes y réplicas.

Gracias por construir conmigo en lugar de conformarte con consumir.
Nos vemos en el Vol.III. Trae tus contadores.

---

> «What I cannot create, I do not understand.»
>
> — Richard Feynman, anotación hallada en la pizarra de su despacho de Caltech tras su muerte (1988)
