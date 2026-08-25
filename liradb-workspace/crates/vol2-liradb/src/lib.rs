//! Vol.II — Cap.7: Modelo de datos de LiraDB.
//!
//! Migrado desde el brief original LiraDB (§"Property Graph + Value").
//!
//! Define los tipos fundamentales de un Property Graph:
//! - [`Value`] — Tipos de datos primitivos (null, bool, int, float, string, bytes).
//! - [`NodeId`] / [`EdgeId`] — Identificadores únicos estables (ver cap 3 para slotmap).
//! - [`Node`] — Vértice con labels y propiedades.
//! - [`Edge`] — Arista dirigida con label, source/target y propiedades.
//! - [`Element`] — Enum para tratar nodos y aristas uniformemente.
//! - [`PropertyGraph`] — Grafo en memoria con listas de adyacencia.
//! - [`GraphStore`] / [`MemoryStore`] — Trait hexagonal (cap 8) + impl en memoria.
//! - [`Pager`] / [`FilePager`] — Gestor de páginas en disco (cap 12): asignación,
//!   lectura, escritura y sincronización de páginas de tamaño fijo.
//! - [`BufferPool`] + [`PolicyKind`] — Caché de páginas en memoria (cap 13):
//!   pinning, dirty tracking, política de reemplazo Clock (por defecto) o LRU,
//!   y métricas (hits/misses/reads/writes/evictions).
//! - [`Csr`] / [`PersistentCsr`] — Representación CSR (Compressed Sparse Row)
//!   persistente (cap 14): forward + backward indexes, `replace()` y `load()`
//!   sobre `BufferPool<Pager>`, con `CsrHeader` (24 bytes) en la página 1.
//! - [`HashIndex`] / [`BPlusTree`] — Índices secundarios (cap 15): hash estático
//!   con FNV-1a + overflow encadenado, y B+ tree de un solo nivel con
//!   búsqueda binaria y `range_scan`. Ambos viven sobre `BufferPool<Pager>`,
//!   con catálogos persistidos (magic + counts) y errores tipados
//!   (`IndexError`). Sin crates externas — implementados a mano conforme a
//!   la regla "primero a mano, luego con crates" del Vol.II.
//! - [`inspect`] / [`check`] / [`compact`] / [`repack_page`] — Mantenimiento
//!   (cap 16): estadísticas de almacenamiento ([`StorageStats`]), verificación
//!   de integridad ([`CheckReport`] con [`IssueKind`]: bad magic, page_id
//!   mismatch, free_space desactualizado, records truncados) y compactación
//!   in-place ([`CompactReport`]). Errores tipados [`MaintenanceError`].
//!   Cierra la Parte III (motor de almacenamiento) del Vol.II.
//! - [`Span`] / [`TokenKind`] / [`Expression`] / [`PathPattern`] /
//!   [`MatchClause`] / [`WhereClause`] / [`ReturnClause`] / [`Query`] —
//!   Diseño del lenguaje de consulta **LiraQL** (cap 17, abre la Parte IV):
//!   tokens, gramática (EBNF), AST, expresiones, patrones de camino
//!   (MATCH-WHERE-RETURN mini), validación semántica y pretty-printer.
//!   Errores tipados [`QueryError`] con posición ([`Span`]). El lexer y el
//!   parser llegan en el cap.18.
//! - [`Lexer`] / [`Parser`] / [`parse`] — Texto → tokens → AST (cap 18):
//!   escáner manual con maximal-munch y parser descendente recursivo con
//!   precedencia por cadena de funciones. Errores tipados [`LexError`] y
//!   [`ParseError`] con `Span` exacto.
//! - [`LogicalPlan`] / [`ScalarExpr`] / [`Bindings`] / [`LogicalType`] /
//!   [`lower`] — Del AST al **plan lógico** (cap 19): el binder liga las
//!   variables del MATCH ([`Bindings`]), resuelve las expresiones de
//!   WHERE/RETURN a [`ScalarExpr`] (sin spans, ya ligadas) y construye el
//!   árbol de operadores `NodeScan` / `Expand` / `Filter` / `Project` /
//!   `CartesianProduct` con pretty-printer (base de `liradb explain`, cap 21)
//!   e inferencia de tipos básica ([`LogicalType`]). Errores tipados
//!   [`PlanError`].
//! - [`PhysicalOperator`] / [`Row`] / [`Cell`] / [`Executor`] / [`run`] — El
//!   **motor de ejecución Volcano** (cap 20, cierra el hito "ejecutar
//!   consultas completas desde texto"): cada operador del plan es un iterador
//!   pull-based (`open`/`next`/`close`) que produce filas ([`Row`]) de
//!   variables ligadas a celdas ([`Cell`]: escalar, nodo o arista).
//!   Operadores `NodeScanOp` / `IndexSeekOp` / `ExpandOp` (direcciones
//!   out/in/undirected) / `FilterOp` (evaluación trivalente con NULL y
//!   cortocircuito) / `ProjectOp` / `CartesianProductOp` (materializa su lado
//!   derecho: Volcano no rebobina) / `LimitOp` / `DistinctOp`, sobre el trait
//!   `GraphStore` del cap 8. [`Executor`] compila el plan y expone métricas
//!   por operador ([`ExecMetrics`], semilla del explain del cap 21);
//!   [`ResultSet`] devuelve columnas + filas; [`run`] y [`Query::execute`]
//!   completan el pipeline parse → lower → optimize → execute. Errores tipados
//!   [`ExecError`].
//! - [`Catalog`] / [`LabelStats`] / [`optimize`] / [`estimate`] / [`explain`] —
//!   El **optimizador pequeño pero real** (cap 21, cierra la Parte IV):
//!   [`Catalog::collect`] recolecta estadísticas del `GraphStore` (nodos por
//!   etiqueta, grados medios out/in, aristas por tipo e índice de igualdad
//!   (etiqueta, propiedad, valor) → ids); [`optimize`] aplica cinco reglas en
//!   orden fijo sobre el `LogicalPlan` (punto inicial más selectivo con
//!   reordenación de expansiones, push-down de predicados, absorción de
//!   `HasLabel` en el escaneo, `NodeScan` → `IndexSeek`, poda de
//!   proyecciones); [`estimate`] estima cardinalidades con heurísticas
//!   documentadas (System R + estadísticas); [`explain`] produce el plan
//!   ANTES/DESPUÉS con estimaciones (el hito `liradb explain`). `run` y
//!   `Query::execute` pasan por el optimizador con resultados equivalentes
//!   (multiconjunto de filas; sin ORDER BY el orden no es parte del contrato).
//! - [`WeightSource`] / [`edge_weight`] / [`dijkstra`] / [`dijkstra_path`] /
//!   [`bellman_ford`] / [`bellman_ford_path`] / [`ShortestPaths`] / [`Path`] —
//!   **Caminos mínimos ponderados** (cap 22, abre la Parte V: algoritmos
//!   sobre el grafo persistente): los pesos se leen de las PROPIEDADES de las
//!   aristas (fuente configurable [`WeightSource::Property`] tipo
//!   `WEIGHT relationship.distance`, o constante para contar saltos) con
//!   semántica estricta y errores tipados; Dijkstra con `BinaryHeap` de std,
//!   borrado perezoso, predecesores por arista y finalización anticipada al
//!   destino; Bellman-Ford que admite pesos negativos y detecta ciclos
//!   negativos alcanzables. Ambos devuelven la misma interfaz: tabla
//!   [`ShortestPaths`] y camino [`Path`] con stats ([`PathStats`]).
//! - [`Heuristic`] / [`ZeroHeuristic`] / [`EuclideanHeuristic`] / [`a_star`] /
//!   [`check_consistency`] — **A*, heurísticas y búsquedas dirigidas** (cap
//!   23, Parte V): A* reutiliza toda la maquinaria del cap 22 (misma fuente
//!   de pesos, mismos errores, mismo tipo de camino [`Path`]/[`PathStats`])
//!   y añade la dimensión que Dijkstra no tiene: una heurística h(n) que el
//!   USUARIO de la API aporta mediante el trait [`Heuristic`] (la euclídea
//!   [`EuclideanHeuristic`] lee coordenadas x/y de las PROPIEDADES DE NODO;
//!   [`ZeroHeuristic`] degenera en Dijkstra). El heap se ordena por
//!   f(n) = g(n) + h(n) y sesga la búsqueda hacia el destino; admisibilidad
//!   documentada (no verificable sin resolver el problema), consistencia
//!   diagnosticable con [`check_consistency`], y re-apertura de nodos para
//!   seguir siendo óptimo con heurísticas admisibles pero inconsistentes.
//! - [`GraphDirection`] / [`degree_centrality`] / [`closeness_centrality`] /
//!   [`betweenness_centrality`] / [`eigenvector_centrality`] /
//!   [`page_rank`] / [`personalized_page_rank`] / [`Teleport`] —
//!   **Centralidad y PageRank** (cap 24, Parte V): las familias de
//!   centralidad del guion sobre `&dyn GraphStore` vía una proyección
//!   materializada una vez (grado O(V+E); closeness por BFS con corrección
//!   Wasserman-Faust para componentes desconectadas; betweenness con el
//!   algoritmo de Brandes O(V·E); eigenvector por iteración de potencia
//!   sobre la adyacencia cruda, con sus DOS fallos documentados — masa que
//!   escapa por nodos colgantes y oscilación en grafos periódicos) y
//!   PageRank como eigenvector REPARADO: damping factor configurable con
//!   validación estricta (0,1) abierto, convergencia por L1 (la masa que
//!   se mueve) con historial por iteración (la razón geométrica ≈ d es
//!   contenido del capítulo), y dangling nodes redistribuidos
//!   uniformemente (Brin-Page 1998). El **PageRank personalizado**
//!   ([`Teleport::Personalized`]) concentra el teleport en semillas
//!   ponderadas — la costura que el cap 51 (GraphRAG) usará como operador
//!   de recuperación, separada aquí del global para no acoplarse nunca.
//!   Errores tipados [`CentralidadError`]; stats [`CentralidadStats`]
//!   (BFS, aristas, iteraciones) para MEDIR el coste computacional.
//! - [`componentes_conexas`] / [`label_propagation`] / [`modularidad`] /
//!   [`louvain`] / [`Particion`] / [`NivelLouvain`] — **Comunidades y
//!   agrupaciones** (cap 25, Parte V): particionar el grafo en vez de
//!   rankear nodos. Componentes conexas (el caso límite: alcanzabilidad),
//!   label propagation determinista (la heurística sin métrica que MOTIVA
//!   lo que sigue), la modularidad Q_γ de Newman-Girvan como función
//!   verificable sobre cualquier partición dada (γ = resolución de
//!   Reichardt-Bornholdt), y Louvain simplificado (Blondel 2008): fase
//!   local greedy con ΔQ exacto + agregación en supernodos por nivel,
//!   sobre una proyección simétrica ponderada (dirigidas sumadas al par,
//!   paralelas acumuladas, self-loops ×2 de la convención estándar) con
//!   los pesos estrictos del cap 22 (`WeightSource`). Determinismo total
//!   (orden por id, empates por `total_cmp` → menor id, renumeración por
//!   menor miembro), Q monótono entre niveles y anidamiento garantizado:
//!   la jerarquía [`NivelLouvain`] es el dendrograma que el cap 51
//!   (GraphRAG) consumirá para resúmenes locales/globales. Errores tipados
//!   [`ComunidadesError`]; stats [`ComunidadesStats`].
//! - [`ProyeccionPonderada`] / [`FiltroProyeccion`] / [`dijkstra_proyeccion`] /
//!   [`bfs_fronteras`] / [`bfs_streaming`] / [`Presupuesto`] / [`BitSet`] /
//!   [`ContandoStore`] — **Ejecutar algoritmos sin agotar la memoria** (cap 26,
//!   CIERRA la Parte V): las dos estrategias de la analítica real, medibles.
//!   La PROYECCIÓN MATERIALIZADA pública con pesos (heredera del CSR del cap
//!   14; semántica estricta del cap 22 pagada UNA vez; filtra por etiqueta de
//!   nodo / tipo de arista; `dijkstra_proyeccion` y `closeness_ponderado`
//!   sobre ella — la deuda que los caps 22 y 24 dejaron anunciada) y el
//!   STREAMING por fronteras que NO materializa: `bfs_fronteras` produce
//!   frontera a frontera (procesamiento por bloques) bajo un [`Presupuesto`]
//!   de profundidad/nodos/lecturas con `MotivoParada` explícito, y
//!   [`ContandoStore`] es el "voltímetro" que demuestra en tests que no se
//!   leyó el grafo entero. [`BitSet`] para índices densos (la lección de cuándo
//!   gana a un hash set).
//! - [`GarantiaAcid`] / [`NivelGarantia`] / [`InformeAcid`] / [`Anomalia`] /
//!   [`Operacion`] / [`Transaccion`] / [`autocommit`] — **Qué significa una
//!   transacción (ACID)** (cap 27, ABRE la Parte VI: fiabilidad): el
//!   vocabulario ACID tipado con valoración honesta del estado actual
//!   ([`informe_acid()`]: A parcial por staging, C trivial, I por préstamo
//!   exclusivo del borrow checker, D ninguna sin WAL) y la transacción como
//!   OBJETO con begin → staging → commit/rollback: las operaciones se
//!   acumulan en un buffer y sólo se aplican tras validar TODO el buffer
//!   (atomicidad naive «o todas o ninguna» frente a errores de validación,
//!   simulando el buffer contra el store y su propio orden). Límites
//!   documentados en tests como gancho al cap. 28: un fallo o pánico a
//!   mitad del APPLY real deja el store a medias (sin log no hay vuelta
//!   atrás) y el commit en RAM no es durable. [`autocommit`] hace visible
//!   el modo por defecto de los caps. 7-26 (cada put_*/delete_* era su
//!   propia transacción). El WAL real es cap. 28, la recuperación cap. 29
//!   y el aislamiento MVCC/2PL cap. 30 — aquí se sientan las palabras y
//!   el esqueleto.
//! - [`WalRecord`] / [`CuerpoWal`] / [`Lsn`] / [`TxId`] / [`Wal`] /
//!   [`WalTransaccion`] / [`replay_wal`] / [`PoliticaFlush`] /
//!   [`InformeReplay`] / [`WalError`] / [`informe_acid_post_wal`] —
//!   **Write-ahead log** (cap 28, Parte VI): la regla «el cambio se
//!   escribe en el WAL antes que en la página de datos» hecha protocolo.
//!   El [`WalRecord`] (LSN u64 monótono + TxId + Begin/Operacion/
//!   Commit/Rollback) reutiliza la `Operacion` del cap. 27 y el framing
//!   y CRC32 del cap. 10 (encoding del cap. 9 para strings/values). El
//!   commit de [`WalTransaccion`] escribe TODAS las operaciones al log,
//!   el registro Commit y hace `sync` (la durabilidad, CONTADA en tests,
//!   con [`PoliticaFlush`] CadaEscritura/SoloCommit — semilla del group
//!   commit) ANTES de aplicar al store: un apply a medias se completa
//!   con [`replay_wal`] (redo idempotente de lo confirmado en orden de
//!   LSN) — el test-tesis que INVIERTE la regresión del cap. 27:
//!   StoreQueFalla + replay = transacción COMPLETA. Parada limpia ante
//!   CRC/cola truncada, truncado con contrato firmado por el llamador
//!   (`truncar_hasta_lsn`), errores tipados y la re-valoración honesta
//!   de ACID tras el WAL ([`informe_acid_post_wal`]: D de Ninguna a
//!   Parcial). La recuperación al arranque (reopen + replay automático)
//!   es cap. 29.
//! - [`EstadoTx`] / [`ElementoId`] / [`Analisis`] / [`analizar`] /
//!   [`redo`] / [`deshacer`] / [`recuperar`] / [`reabrir`] /
//!   [`AntesImagenes`] / [`capturar_antes`] / [`Checkpoint`] /
//!   [`truncar_seguro`] / [`rotar_si_excede`] / [`guardar_wal`] /
//!   [`cargar_wal`] / [`informe_acid_post_recovery`] —
//!   **Recuperación después de un fallo** (cap 29, Parte VI): el arranque
//!   automático que el cap. 28 dejó pendiente, con el esqueleto de ARIES
//!   (Analysis-Redo-Undo). [`analizar`] recorre el log hacia delante para
//!   reconstruir la tabla de transacciones (ganadoras/perdedoras), los
//!   contadores `next_lsn`/`next_tx_id` (reabrir = escanear el log) y la
//!   dirty element table (primer LSN que tocó cada nodo/arista);
//!   [`redo`] re-aplica TODAS las operaciones en orden de LSN de forma
//!   idempotente (el store queda en el estado del instante del fallo);
//!   [`deshacer`] deshace, en orden inverso, las operaciones de las
//!   transacciones perdedoras — la pieza NUEVA: en el cap. 28 el undo era
//!   trivialmente vacío (no-steal), aquí se demuestra con un test-tesis de
//!   un store al que una perdedora «robó» escrituras, y se documenta la
//!   única frontera que un log de solo after-image no cruza (deshacer un
//!   borrado robado exige la imagen anterior → [`AntesImagenes`]).
//!   [`guardar_wal`]/[`cargar_wal`] ponen el log en un fichero real (el
//!   `sync` del cap. 28 era un contador) y [`reabrir`] ejecuta el flujo
//!   completo de arranque: leer fichero + reconstruir + analizar + redo +
//!   undo. [`Checkpoint`] + [`truncar_seguro`] automatizan el truncado que
//!   el cap. 28 dejaba firmado a mano, y [`rotar_si_excede`] cierra la
//!   rotación por tamaño. [`informe_acid_post_recovery`] re-valora ACID:
//!   A y D siguen Parcial pero avanzan (A: falta el before-image; D: el
//!   store de datos aún no tiene checkpoint independiente — cap. 37).
//! - [`Ts`] / [`VersionNode`] / [`VersionEdge`] / [`MvccStore`] /
//!   [`NivelAislamiento`] / [`Recurso`] / [`GrafoEspera`] /
//!   [`informe_acid_post_mvcc`] —
//!   **Snapshots y concurrencia (MVCC limitado)** (cap 30, CIERRA la Parte VI):
//!   la capa de versionado que resuelve la anomalía histórica del Vol.II
//!   — el modelo «un único escritor por el borrow checker» del cap. 27
//!   impedía que DOS lectores leyeran a la vez sin bloquear al escritor.
//!   `MvccStore` mantiene, POR ELEMENTO, una cadena de versiones
//!   `ts_begin` / `ts_end?` / valor; las lecturas con snapshot
//!   (`leer_nodo`, `leer_arista`, `iter_nodos`, `iter_aristas`) clonan
//!   la versión visible al `ts` del lector y NO bloquean al escritor
//!   (`&self` + clonar, sin locks). El commit asigna un `ts` nuevo,
//!   RETIRA la versión actual si la había, APPENDIZA la nueva y aplica
//!   al `inner` (delete-then-put para soportar sobreescritura sobre el
//!   `MemoryStore` de inserción estricta del cap. 8). `gc(hasta)` purga
//!   versiones retiradas cuyo `ts_end < hasta`. `NivelAislamiento`
//!   cierra el vocabulario abierto por el `Anomalia` del cap. 27:
//!   `Instantanea` (el de este capítulo) PROHÍBE lectura sucia y
//!   actualización perdida — y DEJA PASAR write skew (la frontera que
//!   Serializable SI con predicate locks cerraría). `GrafoEspera`
//!   construye la estructura del wait-for graph para deadlocks
//!   (detección O(V+E) por DFS con colores): aunque hoy no pueden
//!   ocurrir (un único escritor), la pieza existe como anzuelo para
//!   caps. futuros de concurrencia real. [`informe_acid_post_mvcc`]
//!   re-valora ACID: I avanza significativamente (lectura sucia y
//!   actualización perdida pasan a estar prohibidas) aunque write skew
//!   sigue pasando.
//!
//! `cap32_import_export` — Importación y exportación (CSV, JSONL, GraphML):
//!   Tres formatos se exponen sobre el trait `GraphStore`: CSV estilo
//!   neo4j-admin (`:ID`, `:LABEL`, `:START_ID`, `:END_ID`, `:TYPE`, sufijos
//!   de tipo `:STRING`/`:INT`/`:FLOAT`/`:BOOL`), JSONL discriminador
//!   (`"tipo":"nodo"|"arista"`, sin pérdida, soporta `Value::Bytes`),
//!   y GraphML XML (con mapeo id externo→interno por orden de aparición).
//!   CSV y JSONL son **streaming**: cada registro se procesa en autocommit
//!   (`Operacion::PutNode`/`PutEdge`/`DeleteNode`/`DeleteEdge` fuera de
//!   transacción), así datasets mayores que la RAM se importan sin buffering
//!   completo. GraphML es la excepción documentada: procesa el bloque
//!   `<key>`/`<graph>` en memoria por la estructura del formato (atributos
//!   `<key id="…">` referenciables desde varios sitios). El exporter de
//!   CSV une las props de TODOS los nodos/aristas en la cabecera (orden
//!   BTreeMap, determinista) y deja el campo vacío en filas donde la prop
//!   no exista (semántica "prop ausente", NO NULL). El parser CSV se hace a
//!   mano (sin crate `csv`) y maneja comillas dobles, comillas internas
//!   escapadas con `""`, y separador `,`. El parser JSON a mano soporta
//!   objetos, arrays, strings, números, `true`/`false`/`null`. El parser
//!   XML a mano emite una corriente de `EventoXml` (apertura/cierre/texto)
//!   consumida por el importer de GraphML. [`ImportError`] distingue
//!   `CabeceraInvalida` (línea 1), `FilaMalformada` (línea N), `Semantica`
//!   (semántica OK sintácticamente), `Io` (lectura/escritura) y
//!   `RegistroRechazado` (la autocommit rechazó el registro — p.ej.
//!   `DuplicateNode`).
//!
//! Organización del crate: cada capítulo vive en su propio módulo de origen —
//! `cap07_modelo`, `cap08_graph_store`, `cap09_encoding`, `cap10_append_only`,
//! `cap11_slotted_pages`, `cap12_pager`, `cap13_buffer_pool`, `cap14_csr`,
//! `cap15_indices`, `cap16_mantenimiento`, `cap17_liraql_ast`,
//! `cap18_lexer_parser`, `cap19_plan_logico`, `cap20_volcano`,
//! `cap21_optimizador`, `cap22_caminos_minimos`, `cap23_a_estrella`,
//! `cap24_centralidad`, `cap25_comunidades`, `cap26_proyeccion`,
//! `cap27_transacciones`, `cap28_wal`, `cap29_recuperacion`, `cap30_mvcc`
//! y `cap32_import_export` — y este
//! `lib.rs` es sólo el punto de entrada: declara los módulos y los re-exporta
//! con `pub use capNN::*` para mantener una API pública plana
//! (`vol2_liradb::Node`, `vol2_liradb::run`, ...). Cada módulo viaja con sus
//! tests (`mod tests_*`).

mod cap07_modelo;
mod cap08_graph_store;
mod cap09_encoding;
mod cap10_append_only;
mod cap11_slotted_pages;
mod cap12_pager;
mod cap13_buffer_pool;
mod cap14_csr;
mod cap15_indices;
mod cap16_mantenimiento;
mod cap17_liraql_ast;
mod cap18_lexer_parser;
mod cap19_plan_logico;
mod cap20_volcano;
mod cap21_optimizador;
mod cap22_caminos_minimos;
mod cap23_a_estrella;
mod cap24_centralidad;
mod cap25_comunidades;
mod cap26_proyeccion;
mod cap27_transacciones;
mod cap28_wal;
mod cap29_recuperacion;
mod cap30_mvcc;
pub mod cap32_import_export;
pub mod cap33_pruebas;

pub use cap07_modelo::*;
pub use cap08_graph_store::*;
pub use cap09_encoding::*;
pub use cap10_append_only::*;
pub use cap11_slotted_pages::*;
pub use cap12_pager::*;
pub use cap13_buffer_pool::*;
pub use cap14_csr::*;
pub use cap15_indices::*;
pub use cap16_mantenimiento::*;
pub use cap17_liraql_ast::*;
pub use cap18_lexer_parser::*;
pub use cap19_plan_logico::*;
pub use cap20_volcano::*;
pub use cap21_optimizador::*;
pub use cap22_caminos_minimos::*;
pub use cap23_a_estrella::*;
pub use cap24_centralidad::*;
pub use cap25_comunidades::*;
pub use cap26_proyeccion::*;
pub use cap27_transacciones::*;
pub use cap28_wal::*;
pub use cap29_recuperacion::*;
pub use cap30_mvcc::*;
pub use cap32_import_export::*;
pub use cap33_pruebas::*;
