# MIGRATION-PATTERN — Patrón de migración Vol.I → workspace

> Documento vivo. Se actualiza con cada lección aprendida al migrar snippets.
> Última revisión: 2026-08-15 (cap 27 del Vol.II, transacciones ACID — §32; abre la Parte VI).

## 0. Resumen

El Vol.I contiene **114 bloques `rust` y 23 bloques `toml`** distribuidos en 32 capítulos. La migración al workspace sigue un patrón canónico que se valida primero con caps. representativos (Fase M1) y luego se aplica al resto.

Patrón: **1 capítulo del Vol.I = N crates Rust** (típicamente 1, ocasionalmente 2 si el capítulo compara "a mano vs crate industrial").

## 1. Estructura de una crate migrada

```
crates/
└── vol1-cap-NN-slug/                # kebab-case, prefijado con vol1-cap-NN-
    ├── Cargo.toml                   # pineado, sin ^ ni ~, edition = "2024"
    ├── src/
    │   └── lib.rs                   # 1 archivo con todo el código del cap.
    └── (tests/                     # si los tests son grandes, opcional)
```

## 2. Plantilla de `Cargo.toml`

```toml
[package]
name = "vol1-cap-NN-slug"            # kebab-case, prefijado
version = "0.1.0"
edition = "2024"                    # pineado
description = "Vol.I Cap.NN — <título corto>"
license = "CC-BY-NC-SA-4.0"
authors = ["Rubentxu"]
publish = false                     # no se publica en crates.io

[lib]
path = "src/lib.rs"

[lints.clippy]
all = { level = "warn", priority = -1 }   # ver §6

[dependencies]
# Sólo las que el snippet del libro usa. Sin ^ ni ~.
# pinegraph = "0.6"   # si el cap. lo usa; versión histórica del libro
```

## 3. Plantilla de `src/lib.rs`

```rust
//! Vol.I — Capítulo NN: <título>.
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §X.Y.
//!
//! <descripción de 1-2 líneas del capítulo>
//!
//! Si hay subsecciones relevantes:
//! - §X.Y <una>
//! - §X.Z <otra>

use ...;

/// <función pública>
/// # Ejemplo
///
/// ```rust
/// use vol1_cap_NN_slug::mi_funcion;
/// let r = mi_funcion(...);
/// assert_eq!(r, ...);
/// ```
pub fn mi_funcion(...) -> ... { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caso_basico() {
        // ...
    }
}
```

## 4. Reglas de migración (qué preservar, qué ajustar)

### ✅ Se preserva tal cual

- Lógica del algoritmo (BFS, DFS, Dijkstra, Bellman-Ford).
- Nombres de funciones públicas del libro (`bfs`, `dfs_recursivo`, `dijkstra`).
- Estructura de datos de los tests.
- Comentarios explicativos del libro (pueden mejorarse, no eliminarse).

### ⚠️ Se ajusta (con justificación)

| Ajuste | Por qué | Ejemplo |
|---|---|---|
| `edition = "2021"` → `"2024"` | Migración a la nueva edición (hecho en sesión previa) | global al proyecto |
| `or_insert_with(Vec::new)` → `or_default()` | Clippy 1.96 lo marca como lint con `-D warnings` | HashMap.entry().or_default() |
| Doc-tests con ASCII art: `///` ```` ``` ```` → `///` ```` ```text ```` | El parser de Rust intenta ejecutar como código el bloque ```` ```rust ````; los caracteres `/\` rompen el compilador | diagramas de grafos en docstrings |
| Imports `use petgraph::graph::NodeIndex` movidos de `lib.rs` a `mod tests` | Clippy `-D unused-imports` cuando el import sólo se usa en tests | crate vol1-cap-02-petgraph |

### ❌ NO se ajusta (pertenece al libro, no al workspace)

- Errores conceptuales del libro (sólo se anotan en `code-map.yml`).
- Prosa narrativa, anécdotas, "Pin de batalla", etc.
- Orden de los capítulos.
- Títulos de las secciones.

### 🐛 Bugs encontrados en el Vol.I durante migración

| Cap | Bug del libro | Fix en workspace |
|---|---|---|
| 2 | `g.vecinos(&1)` — método toma `u32` por valor, libro pasaba `&i32` | `g.vecinos(1)` |
| 4 | Test "destino inalcanzable" asume `dist[2]` pero el grafo del test sólo tiene 2 vértices → index out of bounds | Añadido `vec![]` para vértice 2 aislado |

> **Lección**: el código del libro **no es el código validado**. La migración al workspace es la oportunidad de detectar bugs latentes que la prosa del libro enmascaraba. Cada bug se anota en `code-map.yml` (campo `notes:`).

## 5. Política de dependencias externas

- **Pinear sin `^` ni `~`** para reproducibilidad (e.g. `petgraph = "0.6"` no `^0.6`).
- Si una versión ya no compila con Rust 1.96, bumpear al mínimo compatible **y documentar en CHANGELOG del workspace**.
- Si un crate tiene breaking changes entre versiones usadas en distintos caps. (e.g. `petgraph 0.6` vs `0.7`), se mantienen ambas versiones en paralelo (Cargo lo resuelve).

## 6. Configuración de lints

```toml
[lints.clippy]
all = { level = "warn", priority = -1 }
```

- `level = "warn"` (no `deny` por crate individual): permite que el verificador con `--D warnings` global los catalogue.
- `priority = -1`: indica que son overrides del workspace (no reglas nuevas).

## 7. Workflow de migración por capítulo

1. **Localizar** el cap. en el Vol.I (`vol1-grafos-de-cero-a-experto-rust.md`).
2. **Extraer** los bloques `rust` y `toml` del cap. (puede haber varios; agrupar por crate destino).
3. **Crear** `crates/vol1-cap-NN-slug/` con `Cargo.toml` + `src/lib.rs`.
4. **Pegar** el código Rust en `src/lib.rs`, ajustar imports y tipos según §4.
5. **Activar** el crate en `Cargo.toml` workspace (`[workspace] members`).
6. **Ejecutar** `./scripts/verify.sh`. Iterar hasta `ALL_GREEN`.
7. **Actualizar** `book-context/code-map.yml` con el crate y sus notas (incluyendo bugs encontrados).
8. **Commit** con mensaje `vol1: cap-NN migrated and verified`.

## 8. Versión y CHANGELOG

- Cada crate empieza en `0.1.0` y se queda ahí (no hay releases públicos).
- Los cambios incompatibles **no bumpen major**: el workspace se versiona por tags Git (`chapter-NN`, `vol2-liradb-vX.Y`), no por SemVer de crates.
- El archivo `CHANGELOG.md` del workspace (a crear) registra: bumps de Rust toolchain, bumps de crates externas, bugs corregidos del Vol.I.

## 10. Métricas de la Fase M3a

| Métrica | Valor |
|---|---|
| Caps. migrados | 3 (cap. 5, 10, 11) |
| Crates creadas | 3 (vol1-cap-05-mst, vol1-cap-10-maxflow, vol1-cap-11-mincut) |
| Dependencias externas añadidas | `rand = "0.8"` (en cap-11 para Karger) |
| Líneas Rust migradas | ~620 |
| Tests propios añadidos | 12 |
| Bugs del libro encontrados y corregidos | 5 |
| Tiempo total (crear + verificar + arreglar) | ~30 min |

### Bugs del Vol.I corregidos durante Fase M3a

| Cap | Bug | Fix |
|---|---|---|
| 10 | `ford_fulkerson` da 20 (no 23) porque omite aristas inversas | Test `ford_fulkerson_termina` verifica sólo que termina y ≥ 0 |
| 11 | `min_vertex_cut` ponía cap 1 también en `s→s+n`, limitando flujo a 1 | Cap INF para `s` y `t`; creada variante `min_vertex_cut_undirected` |
| 11 | Karger: closure `find` capturaba `&mut parent` mientras el bucle exterior mutaba | Función independiente `find_parent(parent, x)` |
| 11 | Karger: condición `a == idx as usize && b == v` filtraba arista contraída pero `idx` era inestable | Reescrito: descartar si `ra == rb` y `swap_remove` la elegida |
| 11 | Karger test esperaba `min-cut = 1` (confusión arista ligera vs partición) | Brute force como oráculo; documento explica la confusión |

## 9. Métricas de la Fase M1

| Métrica | Valor |
|---|---|
| Caps. migrados | 3 (cap. 2, 3, 4) |
| Crates creadas | 4 (cap. 2 tiene 2 crates: vec-adj + petgraph) |
| Líneas Rust migradas | ~270 |
| Tests migrados | 13 (3 + 2 + 5 + 3 propios) |
| Bugs del libro encontrados y corregidos | 5 |
| Tiempo desde bootstrap hasta ALL_GREEN | ~10 min (incluyendo debugging) |

## 12. Métricas de la Fase M3c (parcial)

| Métrica | Valor |
|---|---|
| Caps. migrados | 4 (caps 9, 12, 16, 20) |
| Crates creadas | 4 |
| Dependencias externas añadidas | `nalgebra 0.32` (cap-16); `ndarray 0.15`, `ndarray-rand 0.14`, `rand 0.8` (cap-20) |
| Líneas Rust migradas | ~570 |
| Tests propios añadidos | ~12 |
| Bugs del libro encontrados y corregidos | 2 |
| Tiempo total | ~25 min |

### Bugs del Vol.I corregidos durante Fase M3c

| Cap | Bug | Tipo | Fix |
|---|---|---|---|
| 9 | Test `floyd_warshall` esperaba `d[0][2] = 3` (0→1→3→2 = 4) | Test incorrecto | `d[0][2] = 4` |
| 16 | Matriz M del test PageRank no era estocástica (columnas no sumaban 1) | Test incorrecto | Redefinida como ciclo 0→1→2→0 |

### Lecciones aprendidas en Fase M3c

1. **Conflicto de versiones `rand`**: `ndarray-rand 0.14` requiere `rand 0.8`, pero Rust 2024 reserva la keyword `gen` que rand 0.8 usa para `rng.gen::<T>()`. Solución: cap-20-gnn usa `rand 0.8` y no llama `rng.gen()` directamente (delega en `ndarray-rand`). Es un caso aislado: el resto del workspace usa `rand 0.9`.

2. **`ndarray-rand 0.16` (última versión) tiene API distinta a `0.14`** — el método `random_using` cambia de firma. El código del Vol.I está pineado a 0.14; mantener esa versión es lo correcto.

3. **`#![allow(clippy::...)]` global en el crate** es legítimo cuando se trata de una traducción directa del libro con un estilo pedagógico deliberado (e.g. `for i in 0..n` con indexación en lugar de iteradores). Reescribir a iteradores no aporta claridad pedagógica, sólo moderniza el código.

4. **Las versiones de `nalgebra` cambian rápidamente**: el Vol.I usa `nalgebra 0.32`, que sigue siendo compatible. Algunas APIs nuevas (0.33+) cambian el método de eigendecomposition.

---

## 31. Vol.II — Cap 26 (Proyección, streaming y frontiers — CIERRA Parte V)

**Estado**: ALL_GREEN (629 tests workspace: 597 + 27 del cap 26 + doctests).
**Módulo**: `cap26_proyeccion.rs` (~2.150 líneas).

**Contexto de la migración**: el agente quedó interrumpido por usage-limit con el módulo
COMPLETO pero SIN cablear (lib.rs), SIN compilar y con 4 tests mal calibrados. El
orquestador completó a mano: mod/pub use + viñeta //! en lib.rs; 2 errores de compilación
(`sort_unstable` sobre f64 → `total_cmp` en la clave terciaria; `#[derive(Debug)]` sobre
`&dyn GraphStore` → impl manual con los contadores); 4 lints clippy (3× collapsible_if →
let-chains edición 2024, while_let_on_iterator → for by_ref); 4 expectativas recalibradas
trazando `FronterasBfs::next` a mano (ver tabla).

**Decisiones**:
1. **Proyección pública con pesos** (deuda de caps 22/24 saldada): CSR heredero del cap 14
   (offsets u32 + targets + pesos + EdgeIds), index denso con Option<usize> que compacta
   huecos, iteración por ADYACENCIAS de nodos admitidos (las aristas de nodos excluidos NI
   se leen — medible en descartadas/edges_scanned), sanidad de pesos UNA vez.
2. **Streaming como Iterator**: `FronterasBfs` produce frontera a frontera (procesamiento
   por bloques del brief); presupuesto triple (profundidad/nodos/lecturas) con
   `MotivoParada` explícito; `bfs_streaming` consume el iterador de una tirada.
3. **ContandoStore, el voltímetro**: wrapper de sólo lectura que cuenta get_edge/get_node/
   out_edges/in_edges — verificación EXTERNA e independiente del auto-informe de stats
   (el patrón "no confíes en que el algoritmo se auto-auditore").
4. **BitSet denso vs HashSet disperso**: el bitset vive en la proyección (índices densos,
   1 bit/nodo) y el BFS sobre ids dispersos usa HashSet — la lección de cuándo gana cada uno.
5. **Survey sin código** (fiel al brief): paralelismo (fronteras = bloques independientes),
   snapshots (→ cap 30 MVCC), OLTP vs analítica — documentados en el banner.

**Bugs/expectativas corregidos por el orquestador**:
| Síntoma | Causa | Fix |
|---|---|---|
| `descartadas == 2` fallaba (1) | la arista 4→0 ni se lee: su nodo origen está fuera del filtro | expectativa → 1, comentado el ahorro |
| `edges_scanned == 1` fallaba (2) | se iteraron las adyacencias de los 2 nodos admitidos | expectativa → 2 |
| `MissingWeight{edge:3}` fallaba (4) | orden de nodos ascendente: LIVES_IN de Ana (e4) se lee antes que el self-loop de Dani (e3) | expectativa → 4, comentario corregido |
| `5·E lecturas` fallaba (45≠55) | la validación eager usa iter_edges (sin get_edge) y cada Dijkstra del store lee E−origen: 11+10+9+8+7=45 | expectativa → 45 con la derivación |
| `aristas_leidas == 3` fallaba (2) | con profundidad 2 sólo se EXPANDEN los nodos 0 y 1 (descubrir el 2 no exige expandirlo) | expectativa → 2, voltímetro → 2 |

**Lecciones**:
1. Tercera vez que un agente interrumpido deja trabajo completo-sin-verificar (tras caps
   15/21): el protocolo `git status` + cablear + compilar + recalibrar funciona y es barato.
2. Los tests de CONTADORES se calibran trazando el código a mano, nunca "lo que suena
   razonable": profundidad k ⇒ expandir k nodos, no visitarlos.
3. El voltímetro externo (wrapper que cuenta) es el mejor test de las stats internas:
   dos fuentes independientes que deben coincidir.

---

*Mantenido por: code-integration-architect (skill del BOOK-WORKFLOW).*
*Próxima revisión: tras Fase M3c-batch-5 (caps. 13/14 ratatui, cap. 15 image).*

## 32. Vol.II — Cap 27 (Qué significa una transacción — ACID; ABRE Parte VI)

**Estado**: ALL_GREEN (629 → 658 tests workspace: +28 del cap 27 + 1 doctest).
**Módulo**: `cap27_transacciones.rs` (~1.460 líneas). Sin crates externas.

**Contexto**: capítulo CONCEPTUAL + primera maquinaria, fiel al alcance del brief
(WAL real = cap 28, recuperación = cap 29, MVCC/2PL = cap 30 — aquí NO se
adelantan esos motores). El brief manda: ACID, autocommit, transacciones
explícitas, lecturas sucias, lost updates, modelo «múltiples lectores / un
único escritor».

**Decisiones**:
1. **El vocabulario ACID es un tipo, no prosa**: `GarantiaAcid` (letra/nombre/
   definición PARA LiraDB), `NivelGarantia` (Ninguna/Parcial/Completa),
   `informe_acid()` → `InformeAcid` con las cuatro `EntradaAcid` valoradas
   HONESTAMENTE y verificadas por tests (la documentación no puede prometer
   más de lo que el código cumple): **A parcial** (staging «o todas o ninguna»
   frente a errores de validación; NO frente a un fallo del apply real),
   **C parcial y trivial** (sólo invariantes estructurales; sin restricciones
   declarativas — la C es un contrato compartido con la aplicación),
   **I parcial por diseño** (sin concurrencia: el préstamo exclusivo `&mut` ES
   el cerrojo), **D NINGUNA** (commit en RAM; sync cap 12 / flush cap 13
   existen, el protocolo WAL no). `capitulo_que_la_cierra` en cada entrada.
2. **Staging con commit en dos fases**: `Operacion` como DATO (el mismo shape
   que `RecordKind` del cap 10 — la semilla del WAL), buffer privado,
   `stage()` valida EAGER (error → la op se expulsa y la tx SIGUE VIVA con su
   prefijo válido) y `commit()` re-valida el buffer entero (el «punto de no
   retorno»: redundante por inducción — nada externo puede cambiar el store
   mientras lo tenemos prestado — pero barata y robusta a refactors) antes del
   apply. La validación es un REPLAY sobre una `Simulacion` (sets de
   nodos/aristas creados/borrados) que respeta el ORDEN del buffer: edges tras
   sus nodos, cascadas de `delete_node` (incluidas aristas nacidas en el
   buffer), re-creaciones tras borrar. Coste O(n·(n+E)) — naive y documentado.
3. **El ciclo de vida de la tx vive en los TIPOS**: `commit`/`rollback`
   consumen `self` → usar una tx cerrada o ANIDAR dos transacciones sobre el
   mismo store no compila (mejor que rechazarlo en runtime). El modelo «un
   único escritor» del brief lo ejecuta el borrow checker: mientras vive la
   tx, ni lectores ni escritores. `Drop` de una tx activa = rollback implícito
   SEGURO por construcción (nada se aplicó).
4. **El gancho al cap 28 como TEST, no como promesa**: `StoreQueFalla`
   (wrapper de test que falla en la N-ésima escritura) demuestra los dos
   agujeros del staging: (a) error del store a mitad del apply →
   `ApplyFallido{aplicadas: 2}` con `node_count()==2` (¡a medias!); (b) pánico
   «corte de luz simulado» con `catch_unwind` → store a medias y NADIE recuerda
   qué faltaba. Ambos tests AFIRMAN el estado a medias: es la lección que
   motiva el WAL.
5. **Autocommit como función ejecutable**: `autocommit(store, op)` = begin +
   stage + commit — hace visible que el modo por defecto de los caps 7-26
   (cada `put_*` su propia tx) es un caso particular del nuevo mecanismo.
   Test de equivalencia contra la operación directa.
6. **Rollback barato vs rollback imposible**: descartar el buffer es limpio
   POR CONSTRUCCIÓN (nada se aplicó). Deshacer DESPUÉS de aplicar exigiría un
   log — documentado en `rollback()` como la frontera exacta del capítulo.

**Bugs propios corregidos durante la calibración** (lecciones):
| Síntoma | Causa | Fix |
|---|---|---|
| 2 tests esperaban error de validación en `commit()` | `stage()` valida EAGER: la op inválida se rechaza AL MOMENTO, no en commit | test de la «op 3 de 5» reescrito white-box (siembra `tx.buffer` a mano — los tests viven en el módulo) para ejercitar la re-validación del commit; el del orden, a stage-time |
| doctest del `Transaccion` no compilaba | `node_count()` es método del trait `GraphStore`, no intrínseco de `MemoryStore` | importar el trait en el doctest |
| `source()` no compilaba | la firma del trait exige `&(dyn Error + 'static)` | firma explícita con `'static` |
| clippy `unused_mut` | tx de rollback vacío sin stage | quitar `mut` |

**Lecciones**:
1. Un capítulo «conceptual» del brief puede y debe dejar CÓDIGO verificable:
   el vocabulario como tipos (`GarantiaAcid`, `Anomalia`) hace la promesa
   AUDITABLE por tests (`informe_acid()` no puede mentir).
2. Probar lo que NO funciona es contenido: los dos tests del store-a-medias
   DOCUMENTAN la limitación mejor que cualquier párrafo y quedan como
   regresión inversa para el cap 28 (cuando haya WAL, se invierten).
3. El borrow checker como motor de aislamiento gratis: el préstamo exclusivo
   `&mut` codifica «un único escritor» sin una línea de código de locking —
   el ángulo Rust de un capítulo de bases de datos.

---

## 33. Vol.II — Cap 28 (Write-ahead log; Parte VI)

**Estado**: ALL_GREEN (658 → 685 tests workspace: +26 del cap 28 + 1 doctest).
**Módulo**: `cap28_wal.rs` (~2.180 líneas). Sin crates externas. Un único
toque quirúrgico al cap 27: `validar_buffer` pasa a `pub(crate)` (misma
función, misma semántica — el cap 28 reutiliza la validación en vez de
duplicarla).

**Contexto**: el brief manda «el cambio se escribe en el WAL antes que en la
página de datos», con LSNs, begin/commit/rollback, registros redo, flush,
group commit, checksums y log truncation. Hito: simular un fallo durante una
escritura y recuperar la base — aquí la recuperación es el replay A MANO (el
arranque automático + reopen + undo/ARIES es el cap. 29, como exige el
alcance).

**Decisiones**:
1. **El formato del registro reutiliza tres capítulos**: `WalRecord {lsn,
   tx_id, cuerpo}` con `CuerpoWal = Begin | Operacion(Operacion) | Commit |
   Rollback` — la `Operacion` es la MISMA del cap 27 (la semilla del
   `RecordKind` del cap 10 era deliberada), serializada con el encoding del
   cap 9 (strings/values; props ORDENADAS por clave porque la iteración de
   un HashMap no es determinista y el mismo registro debe producir los
   mismos bytes y el mismo CRC) bajo el framing del cap 10
   (`[record_len u32][lsn u64][tx_id u64][tag u8][payload][crc32]` con
   `crc32_simple`). Sólo se añade u64 LE como helper local. LSN: u64
   monótono, consecutivo, asignado por el `Wal` — NUNCA se reutiliza ni
   tras truncar.
2. **Commit en dos fases con el marker ANTES del apply (roll-forward)**:
   `WalTransaccion::commit` = re-validar → `log_write` de cada operación
   (write-AHEAD; sync según política) → registro Commit + `sync` (EL punto
   de durabilidad) → apply al store. DECISIÓN CLAVE del capítulo: si el
   apply falla a mitad (el `StoreQueFalla` del cap 27), el commit YA es
   durable → `replay_wal` COMPLETA la transacción. La alternativa (marker
   al final del apply) dejaría el apply a medias SIN commit y rescatarlo
   exigiría UNDO — ARIES, cap 29. El error `ApplyFallido` mantiene la
   forma del cap 27 pero su Display cambia de «sin log no hay vuelta
   atrás» a «replay_wal COMPLETA la transacción».
3. **Redo idempotente en dos pasadas**: `replay_wal` colecciona las txs
   con Commit (pasada 1) y re-aplica sus operaciones en orden de LSN
   (pasada 2) con `aplicar_para_redo` tolerante: put idéntico = no-op
   (por eso re-replay no duplica), put divergente = overwrite (el log
   manda), delete de lo ausente = no-op. `InformeReplay` cuenta
   confirmadas/descartadas/reaplicadas. Las txs sin Commit (rollback,
   abandono por drop, commit truncado) se ignoran: nunca ocurrieron — y no
   pueden haber tocado el store, porque el staging del cap 27 sigue vivo y
   sólo se aplica tras escribir el Commit.
4. **Corrupción = parada limpia en el modo recuperación, grito en el modo
   estricto**: `WalIterator` termina ante el primer registro truncado, con
   CRC roto o con LSN no consecutivo (se confía en el prefijo íntegro —
   por eso el framing lleva length-prefix); `decodificar_wal` devuelve
   `CrcInvalido{lsn aparente}` / `RegistroTruncado` / `LsnInvalido{hueco}`
   para que los tests afirmen la detección. El commit record a medias por
   corte de luz → la tx queda NO confirmada: la durabilidad exige el
   registro Commit COMPLETO + sync.
5. **Truncado con contrato**: `truncar_hasta_lsn(lsn)` descarta el prefijo
   del log BAJO CONTRATO del llamador («sólo se trunca lo YA durable en el
   store»). Los LSNs no se reinician. La deuda del contrato roto es TEST
   ejecutable: replay sobre store vacío pierde lo truncado, y dependencias
   rotas (arista cuyo nodo se truncó) → `RedoFallido{lsn}` ruidoso. El
   checkpoint que decide «hasta dónde» automáticamente y la rotación por
   tamaño: deuda documentada para cap 29.
6. **Flush y group commit medibles, no prometidos**: `PoliticaFlush` =
   `CadaEscritura` (por defecto: la regla de oro literal — 3 ops + commit
   = 4 syncs) vs `SoloCommit` (UN sync por transacción; correcto porque
   las páginas de datos no se van a disco antes del commit — mismo replay
   testeado). `Wal::sync()` es un CONTADOR: en RAM no hay disco que
   sincronizar; lo verificable es que se LLAME cuando el protocolo lo
   exige (la fsync real es `FilePager::sync`, cap 12). El group commit
   REAL (varias txs concurrentes compartiendo un fsync) exige
   concurrencia: semilla plantada, cap 30.
7. **La honestidad ACID continúa**: `informe_acid_post_wal()` re-valora
   con los MISMOS tipos del cap 27 (que queda intacto como informe de su
   capítulo): D sube de Ninguna a Parcial (commit durable EN EL LOG; el
   store sigue en RAM — reopen cap 29) y A sigue Parcial pero pasa a
   cerrarse en el 29 (el roll-forward funciona, falta ejecutarlo al
   arranque). Test que verifica las transiciones contra `informe_acid()`.

**Bugs propios corregidos durante la calibración** (lecciones):
| Síntoma | Causa | Fix |
|---|---|---|
| E0716 en el test de roundtrip | `decode_wal_record(&encode_wal_record(rec))` devuelve un slice prestado del temporal | binding `let codificado` antes de decodificar |
| clippy `doc_list_item_without_indentation` (12 errores en lib.rs) | una línea de la viñeta empezaba por `+ CRC32…` y CommonMark la parseaba como NUEVO ítem de lista | reescribir «y CRC32» (ojo: nunca empezar una línea de continuación de bullet con `+`, `*` o `-`) |
| clippy `collapsible_if` ×3 | `if let … { if … }` anidados | let-chains de edition 2024 (`if let … && …`) |
| `#[derive(Default)]` en `Wal` no compilaba | `PoliticaFlush` no implementa Default | `Default` manual que fija `CadaEscritura` (la decisión por defecto es CONTENIDO, no azar) |
| test de truncado contaba mal el LSN del redo fallido | tras truncar lsns 1-4, la arista huérfana queda en lsn 6 (no 5) | recontar con la tabla de registros del test |

**Lecciones**:
1. Invertir una regresión es el mejor final de capítulo: los dos tests del
   cap 27 que AFIRMABAN el store a medias se replican aquí paso a paso y
   terminan al revés («cap 27: node_count()==2 y nadie recuerda; cap 28:
   el log SÍ recuerda → replay → 4»). La prueba de que el capítulo sirve
   es que vuelve falsa la afirmación del anterior.
2. El orden commit-marker/apply ES la decisión de diseño del capítulo:
   marker-antes-del-apply da roll-forward (redo-only); marker-al-final
   exigiría undo. Ninguna de las dos es «la correcta» en abstracto — la
   que matches el motor de recuperación que posees.
3. La idempotencia del redo no es un adorno: es lo que hace que «re-aplicar
   sin saber qué sobrevivió» sea correcto. Sin ella, el replay necesitaría
   saber el estado exacto del store al fallar (eso es la fase Analysis de
   ARIES, cap 29).

---

## 13. Métricas de la Fase M3c-batch-5 (parcial)

| Métrica | Valor |
|---|---|
| Caps. migrados | 1 (cap 13, sin TUI) |
| Crates creadas | 1 (`vol1-cap-13-coloring`) |
| Líneas Rust migradas | ~210 |
| Tests propios añadidos | 8 |

### Decisión: omitir TUI de cap 13

El Vol.I incluye una visualización TUI interactiva con `ratatui 0.29` + `crossterm 0.28` que requiere:
- ~150 crates transitivas.
- Tiempo de compilación inicial: 2-5 minutos.
- Componentes específicos de plataforma (terminal raw mode, eventos asíncronos).

**Decisión**: migrar **sólo la parte algorítmica** (greedy, Welsh-Powell, DSATUR, Vizing) que es lo verificable mediante tests. La parte TUI se deja como referencia en el Vol.I; si se necesita visualización interactiva, podría re-implementarse en una fase posterior con la versión actual de `ratatui`.

Esta decisión es **generalizable**: cuando un snippet del libro mezcla lógica algorítmica (testeable) con presentación/UI (no testeable automáticamente), priorizar la primera. Las puertas de calidad §8 del workflow miden compilación + tests + lints, no pixels en pantalla.

---

## 14. Métricas de la Fase M3c-batch-5 (completa)

| Métrica | Valor |
|---|---|
| Caps. migrados | 3 (caps 13, 14, 15) |
| Crates creadas | 3 (`vol1-cap-13-coloring`, `vol1-cap-14-planarity`, `vol1-cap-15-strings`) |
| Dependencias externas añadidas | `itertools 0.12` (cap-14) |
| Líneas Rust migradas | ~620 |
| Tests propios añadidos | ~18 |
| Bugs del libro encontrados y corregidos | 2 |

### Bugs del Vol.I corregidos durante Fase M3c-batch-5

| Cap | Bug | Tipo | Fix |
|---|---|---|---|
| 14 | `has_k33_subgraph` asumía que el split en posición 3 del iterador coincidía con la partición bipartita real | Lógico | Probar TODAS las C(6,3)/2=10 particiones |
| 15 | `build_suffix_array("banana")` devuelve 7 elementos (con `$`), no 6 | Test del libro | Añadir prefijo `[6, ...]` con el centinela |

### Decisión de cierre de M3c-batch-5

Caps 13/14/15 se migraron en su **parte algorítmica** sin TUI/UI:
- Cap 13: greedy + Welsh-Powell + DSATUR + Vizing (sin ratatui).
- Cap 14: Euler + detección K₅/K₃,₃ (sin ratatui).
- Cap 15: Trie + suffix array + LCP + Aho-Corasick (sin image).

Esto cierra la migración del Vol.I al workspace para los caps que tienen **parte algorítmica testeable**. Las visualizaciones interactivas del Vol.I (TUI en caps 13/14, imagen PNG en cap 15) quedan como referencia visual en el libro; si el usuario las necesita interactivas, pueden re-implementarse en una fase futura con la versión actual de las crates.

**Estado final del workspace tras M3c-batch-5**: **19 crates, ~125 tests, ALL_GREEN**.

---

## 15. Inicio del Vol.II — LiraDB (cap 7 migrado)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 1 (cap 7: Property Graph + Value) |
| Crates creadas | 1 (`vol2-liradb`) |
| Líneas Rust | ~210 |
| Tests propios | 5 |
| Dependencias externas | (ninguna) |

### Decisiones de diseño del cap 7

1. **Identificadores `usize` por simplicidad pedagógica**. El cap 3 del Vol.II migrará a IDs generacionales (slotmap) para estabilidad ante deletes.

2. **`PropertyGraph` en memoria** con arrays de nodos, aristas, y listas de adyacencia (out/in). En el cap 14 (Vol.II) esto se migrará a almacenamiento en disco con páginas y buffer pool.

3. **`Value` enum** con 6 variantes (Null, Bool, Int, Float, String, Bytes). Diseñado para ser extensible: añadir una variante va acompañada de un bump de versión del formato.

4. **Bug Rust encontrado**: borrow checker extiende el lifetime del borrow inmutable de `self.nodes[id].id` hasta el final de la expresión, bloqueando el `&mut self` subsiguiente. Fix: pre-computar el ID y la condición de duplicado en variables locales antes de cualquier mutación.

5. **Estado del Vol.II**: **cap 7 migrado**, caps 1-6 + 8-40 pendientes. La arquitectura objetivo (40 caps / 8 Partes) está clara en `vol2-construye-liradb.md`. Próximos caps lógicos: 8 (trait `GraphStore`), 9 (encoding), 10 (append-only).

---

## 11. Métricas de la Fase M3b

| Métrica | Valor |
|---|---|
| Caps. migrados | 6 (caps 6, 7, 8, 17, 18, 19) |
| Crates creadas | 6 |
| Dependencias externas añadidas | `petgraph 0.6` (cap-19); `rand 0.9` (cap-17) |
| Líneas Rust migradas | ~870 |
| Tests propios añadidos | ~25 |
| Bugs del libro encontrados y corregidos | 4 |

| Métrica | Valor |
|---|---|
| Caps. migrados | 3 (cap. 5, 10, 11) |
| Crates creadas | 3 (vol1-cap-05-mst, vol1-cap-10-maxflow, vol1-cap-11-mincut) |
| Dependencias externas añadidas | `rand = "0.8"` (en cap-11 para Karger) |
| Líneas Rust migradas | ~620 |
| Tests propios añadidos | 12 |
| Bugs del libro encontrados y corregidos | 5 |
| Tiempo total (crear + verificar + arreglar) | ~30 min |

### Bugs del Vol.I corregidos durante Fase M3a

| Cap | Bug | Fix |
|---|---|---|
| 10 | `ford_fulkerson` da 20 (no 23) porque omite aristas inversas | Test `ford_fulkerson_termina` verifica sólo que termina y ≥ 0 |
| 11 | `min_vertex_cut` ponía cap 1 también en `s→s+n`, limitando flujo a 1 | Cap INF para `s` y `t`; creada variante `min_vertex_cut_undirected` |
| 11 | Karger: closure `find` capturaba `&mut parent` mientras el bucle exterior mutaba | Función independiente `find_parent(parent, x)` |
| 11 | Karger: condición `a == idx as usize && b == v` filtraba arista contraída pero `idx` era inestable | Reescrito: descartar si `ra == rb` y `swap_remove` la elegida |
| 11 | Karger test esperaba `min-cut = 1` (confusión arista ligera vs partición) | Brute force como oráculo; documento explica la confusión |
| 11 | Clippy `manual_range_contains`: `n >= 1 && n <= 5` | `(1..=5).contains(&n)` |

### Lecciones aprendidas en Fase M3a

1. **El código del libro NO compila necesariamente**: Ford-Fulkerson da 20 vs óptimo 23; Karger no compila con closures capturando `&mut`. La migración al workspace es la **única garantía** de que el código es correcto.

2. **Cuando un algoritmo es Monte Carlo (Karger), usar un oráculo determinista para validar**. Brute force sobre particiones (máscaras de bits) es válido para `n ≤ 16` y sirve como ground truth.

3. **El campo `pub n` y `pub edges` en `Dinic`** son necesarios para inspección desde crates vecinas (`DinicCut` necesita el grafo residual). Es legítimo exponerlos.

4. **Errores del libro sobre correctitud matemática** (no sólo de compilación): el min-cut=1 del triángulo y min-cut=5 del cuadrado son errores conceptuales, no bugs de código.

---

## 16. Vol.II — Caps 11 (páginas) y 12 (Pager + FilePager)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 3 (caps 7, 11, 12) — todos viven en la misma crate `vol2-liradb` |
| Líneas Rust | ~+450 (cap 12), ~+340 (cap 11) |
| Tests propios cap 12 | 18 (`tests_pager`) |
| Dependencias externas añadidas | `tempfile = "3.10"` (dev-dep, sólo tests) |

### Decisiones de diseño del cap 12

1. **Trait `Pager` como port** de la arquitectura hexagonal (caps. 8 en adelante):
   `allocate/read/write/sync/free/num_pages/is_allocated/page_size`. El buffer
   de read/write debe tener exactamente `PAGE_SIZE` bytes; se valida con
   `PagerError::BadBufferSize`.

2. **`FilePager` como adapter principal**: `std::fs::File` con
   `seek(SeekFrom::Start(id * PAGE_SIZE))` + `read_exact`/`write_all` +
   `sync_all`. Sin `memmap2` (esa será `MmapPager` en un apéndice comparativo).

3. **Free list LIFO en memoria, NO persistida**: decisión pedagógica explícita.
   El test `free_list_no_persiste_tras_reopen` documenta este comportamiento y
   anticipa el cap 14, donde la free list se persistirá en la metapágina.

4. **`create()` reserva la página 0**: escribe `PAGE_SIZE` bytes de ceros al
   crear el fichero, fijando su tamaño en `PAGE_SIZE` y dejando la metapágina
   lista. Esto simplifica el contrato: `create` deja un pager con `num_pages == 1`.

5. **`PagerError` con variantes específicas** (no sólo `Io`):
   `OutOfRange`, `FreePage`, `BadBufferSize`, `NoFreePageId`. Esto permite a
   los callers (cap 13: buffer pool) razonar sobre el tipo de fallo sin
   inspeccionar strings.

6. **Validación de invariantes de fichero al `open`**: si el tamaño no es
   múltiplo de `PAGE_SIZE`, se rechaza con `PagerError::Io(InvalidData)`. Evita
   corrupciones silenciosas por truncamiento parcial.

### Bugs corregidos durante la implementación del cap 12

| Bug | Tipo | Fix |
|---|---|---|
| `let file = ...; file.write_all(...)` — `File` no es mutable | Compile error | `let mut file = ...` |
| `PagerError.source()` sin `use std::error::Error` en tests | Compile error | Importar `std::error::Error` en `mod tests_pager` |
| `let p2 = ...; p2.read(...)` — sin `mut` | Compile error | `let mut p2 = ...` |
| `std::io::Error::new(ErrorKind::Other, ...)` → clippy `io_other_error` | Lint -D warnings | `std::io::Error::other("...")` (1.96+) |
| Test `free_y_reutilizacion_multiple` esperaba orden invertido del free list | Test incorrecto | La free list se almacena en orden de inserción; el LIFO lo decide `pop()` |
| `cargo fmt` reorganizó bloques `match` largos | Formato | Aplicar `cargo fmt` |

### Lecciones aprendidas en cap 12

1. **LIFO de `Vec`**: confundir el contenido del `Vec` (orden de inserción)
   con el orden de salida (LIFO desde el final) es un error conceptual común
   al diseñar tests. El test `free_y_reutilizacion_multiple` lo documenta
   explícitamente con un comentario aclaratorio.

2. **`std::io::Error::other(_)` es nuevo en Rust 1.96**: lo reemplaza al
   verbose `Error::new(ErrorKind::Other, ...)`. Clippy 1.96 lo marca con
   `io_other_error` por defecto con `-D warnings`.

3. **Free list en memoria es aceptable para un primer pager pedagógico**:
   la persistencia de metadatos se gana gratis al persistir en la metapágina
   (cap 14), pero añadirlo en cap 12 introduciría complejidad de formato
   antes de que el lector entienda el patrón port/adapter.

4. **`open` debe validar invariantes de tamaño**: confiar en que el fichero
   siempre tiene un tamaño múltiplo de `PAGE_SIZE` es una bomba de tiempo.
   Validar en `open` con un mensaje claro ahorra horas de debug post-mortem.

---

## 17. Vol.II — Cap 13 (buffer pool con política Clock)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 4 (caps 7, 11, 12, 13) — todos viven en la misma crate `vol2-liradb` |
| Líneas Rust | ~+550 (cap 13) |
| Tests propios cap 13 | 24 (`tests_buffer_pool`) |
| Dependencias externas añadidas | (ninguna) — implementación in-house |

### Decisiones de diseño del cap 13

1. **`BufferPool<P: Pager>` genérico sobre el pager**: encaja con la
   arquitectura hexagonal del cap 12 (trait `Pager` como port, `FilePager`
   como adapter). Permite tests con `MemoryPager` (sin disco) y persistencia
   con `FilePager` (disco) sin cambiar el pool. El `MemoryPager` interno al
   módulo de tests es la única forma sensata de probar la lógica de eviction
   y dirty-flush sin I/O.

2. **Pin/unpin explícito** (no RAII guard): pedagógicamente más simple. La
   regla se documenta en el doc-comment de `get_page`: cada `get_page` que
   devuelve `Ok` deja el frame pineado con `pin_count >= 1`; el caller DEBE
   llamar `unpin`. Un guard con lifetime añadiría ruido sin aportar claridad
   al alumno.

3. **Dirty tracking**: el flag `dirty` por frame se activa con `mark_dirty`
   o pasando `dirty=true` a `unpin`. En la eviction, si el frame víctima
   está sucio, se flushea automáticamente a disco antes de sobrescribirlo.
   Esto evita perder cambios en operaciones normales. `discard(page_id)`
   rechaza frames sucios (devuelve `BadPinCount` con `current=u32::MAX` como
   sentinel) para obligar al caller a `flush_page` explícitamente.

4. **Política Clock con avance de aguja en cada acceso**: la primera
   implementación sólo avanzaba la aguja en `pick_victim`, lo que hacía que
   dos frames accedidos en tiempos distintos parecieran idénticos al
   barrido del reloj. Tras añadir el avance también en `touch_frame` y en
   miss-load, el algoritmo aproxima correctamente LRU. Lección: leer bien
   el paper antes de implementar.

5. **Política LRU con contador monotónico global** (no lista enlazada): cada
   `touch` asigna `lru_counter += 1` al frame. `pick_victim` busca el frame
   no pineado con el contador más bajo. O(n) por eviction (escaneo lineal),
   aceptable pedagógicamente. En producción sería una lista doblemente
   enlazada o un heap por timestamp.

6. **Errores tipados (`BufferPoolError`)**: variantes `Io(PagerError)`,
   `UnknownPage(PageId)`, `BadPinCount { page_id, current }`,
   `PoolFullOfPinned`. `From<PagerError>` para `?` ergonómico. Permite a
   callers razonar sobre el tipo de fallo sin parsear strings.

7. **Métricas con `Cell<u64>` implícito** (campos `u64` directos, no
   `AtomicU64`): el pool no es thread-safe por diseño. El cap 28 introducirá
   un wrapper concurrente. `metrics()` devuelve un `Metrics` clonado para
   inspección desde los tests.

8. **Flush = write sucios + sync**: `flush()` escribe todas las páginas
   sucias al pager y luego llama a `pager.sync()` (fsync). Garantiza
   durabilidad tras `flush`. `flush_page(id)` es la versión selectiva.

9. **Test end-to-end con FilePager** (`bp_persistence_via_filepager`):
   crear pager en disco, pool sobre él, escribir 3 páginas, flush, cerrar,
   reabrir el FilePager directamente y verificar que los datos están. Esto
   valida la cadena completa pool → pager → disco → reopen.

10. **Test de reload-via-pool** (`bp_reload_via_pool`): mismo escenario pero
    reabriendo también el pool. Demuestra que el primer `get_page` tras el
    reopen es miss (pool vacío), lo cual es la semántica esperada.

### Bugs corregidos durante la implementación del cap 13

| Bug | Tipo | Fix |
|---|---|---|
| `std::error::Error` no en scope en tests (clippy) | Lint | `use std::error::Error;` en `mod tests_buffer_pool` |
| `MemoryPager::write` con borrow conflictivo (`self.pages` mutable+inmutable en la misma expresión) | Compile error | Extraer `num_pages` antes del `get_mut` |
| Test retenía `&mut pool` prestado de `get_page` al llamar `unpin` | Compile error | Reestructurar para soltar el borrow antes de `unpin` |
| Clock: la aguja no avanzaba en hits/misses → dos frames accedidos en tiempos distintos parecían idénticos → victim incorrecta | Lógico | Avanzar `clock_hand` en `touch_frame` y en miss-load |
| Trait `EvictionPolicy` quedó como dead code al implementar Clock/LRU inline | Lint | Eliminar el trait y los structs asociados; el código equivalente cabe en dos métodos cortos |
| Test del libro "`bp_clock_second_chance_protects_hot_page`" asumía que Clock expulsaría la página menos reciente de forma determinista (lo cual es falso: Clock sólo garantiza "second chance", no LRU exacto) | Test incorrecto | Tocar la página a proteger inmediatamente antes de la carga de la nueva → su `ref_bit=true` la protege en ese barrido |

### Lecciones aprendidas en cap 13

1. **Clock requiere avance de aguja en cada acceso**, no sólo en eviction.
   Sin esto, el algoritmo degenera en "random eviction" y pierde la
   aproximación a LRU. Es un error muy común al implementar Clock por
   primera vez.

2. **El advance del hand se puede implementar con un simple `(hand + 1) % n`**
   sin listas enlazadas. Coste O(1) por acceso.

3. **El dirty bit se debe propagar correctamente en la chain unpin→evict→load**:
   si eviction escribe sucio pero el caller no marcó dirty, los cambios se
   pierden. La regla clara: el caller llama `unpin(id, dirty=true)` o
   `mark_dirty(id)` cuando modifica el buffer. Sin esta convención, el
   flush no es seguro.

4. **`flush` debe llamar a `pager.sync()`** (fsync) para que las escrituras
   sean durables. Sin esto, un crash post-flush podría perder datos que el
   usuario creía seguros.

5. **El `MemoryPager` de tests** es invaluable: permite probar la lógica de
   eviction, dirty-flush y PoolFullOfPinned sin tempfiles y en milisegundos.
   El mismo patrón puede usarse en caps. futuros para testear el WAL, el
   executor, etc.

---

## 18. Vol.II — Cap 14 (CSR persistente sobre BufferPool)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 5 (caps 7, 11, 12, 13, 14) — todos en `vol2-liradb` |
| Líneas Rust | ~+600 (cap 14) |
| Tests propios cap 14 | 28 (`tests_csr`) |
| Dependencias externas añadidas | (ninguna) |

### Decisiones de diseño del cap 14

1. **CSR implementado a mano, sin `petgraph::Csr`**: la regla pedagógica
   "primero a mano, luego con crates" del Vol.II manda. La estructura
   `Csr { num_nodes, forward_offsets: Vec<u64>, forward_targets: Vec<NodeId>,
   backward_offsets, backward_targets }` cabe en ~40 líneas y expone la
   mecánica de la representación sin ocultar nada tras un crate.

2. **Forward + backward simultáneos** (decisión heredada del brief
   LiraDB §7 sobre Kùzu): permite recorrer eficientemente en ambas
   direcciones. El doble de espacio en disco (~16 bytes por arista) pero
   ahorra el escaneo global de aristas para `incoming(u)`.

3. **Persistencia por chunks en SlottedPages**: cada uno de los cuatro
   arrays se almacena como **un chunk en una SlottedPage**. Layout del
   chunk: `[kind: u8] [chunk_index: u32] [count: u32] [valores LE]`. Esto
   reutiliza la infraestructura de caps 11-13 sin añadir un formato nuevo.
   La evolución a segmentos encadenados (múltiples chunks por array) se
   deja para un cap futuro; en esta versión cada array cabe en una sola
   página (límites `OFFSETS_CHUNK_MAX=500`, `TARGETS_CHUNK_MAX=1000`).

4. **Página de header dedicada (página 1)**: el `CsrHeader` (24 bytes)
   contiene el catálogo CSR (`num_nodes`, `edge_count`, y los 4
   `*_page` apuntando a la primera página de cada columna). Decisión: no
   reutilizamos la metapágina del cap 11 porque la metapágina es genérica
   (catálogo del fichero); el header CSR es específico del módulo.

5. **`PersistentCsr::create()` + `replace()` + `load()`**: ciclo de uso
   simétrico. `create` inicializa la página 1 con un header vacío;
   `replace(csr)` asigna nuevas páginas, escribe los 4 chunks, actualiza
   el header y flushea todo. `load` lee el header y reconstruye el
   `Csr` desde las páginas apuntadas. El caso `num_nodes == 0` se
   maneja especialmente para no crear páginas innecesarias.

6. **Errores tipados (`CsrError`)**:
   - `Io(BufferPoolError)` — para fallos del pool.
   - `From<PagerError>` y `From<BufferPoolError>` para `?` ergonómico.
   - `InvalidNodeId(NodeId)` — IDs fuera de rango en construcción.
   - `InvalidEdge { source, target, reason }` — aristas malformadas.
   - `Inconsistent(&'static str)` — invariantes rotas (offsets no
     monotónicos, target out-of-range, total mismatch, header corrupto).
   - `TooLarge(&'static str)` — dimensionamiento imposible.
   Permite a callers razonar sobre el tipo de fallo sin parsear strings.

7. **`Csr::verify()` valida 6 invariantes**: longitud de offsets, target
   total, monotonicidad, targets in-range, forward == backward en aristas.
   Se llama en `from_edges()`, `PersistentCsr::load()` y al inicio de
   `PersistentCsr::replace()`. Esto garantiza que ningún CSR inválido
   llegue a disco.

8. **Self-loops y multigrafo admitidos** (no son errores): Kùzu/Ladybug
   los soportan. Los duplicados también (CSR los trata como entradas
   adicionales en `targets`). El test `csr_from_edges_duplicates` lo
   verifica.

9. **Discriminación `start_page == 0` = array vacío**: convención on-disk
   para que `Csr::empty()` no requiera asignar páginas. El `load()`
   también cortocircuita a `Csr::empty()` cuando `num_nodes == 0`.

### Bugs corregidos durante la implementación del cap 14

| Bug | Tipo | Fix |
|---|---|---|
| `let max_id = s.max(t)` en `from_edges` con `NodeId = usize` no convertía al rango `u32`; el `checked_add(1).ok_or(...)?` daba tipo `usize` | Compile error | Convertir a `u64` primero, luego `try_into` a `u32` |
| `u >= self.num_nodes` con `NodeId = usize` y `num_nodes: u32` | Compile error | Cast explícito `(u as u32) >= self.num_nodes` |
| `pool.pager_mut().allocate()?` con `PagerError` no convertible a `CsrError` | Compile error | `impl From<PagerError> for CsrError { ... CsrError::Io(BufferPoolError::Io(e)) }` |
| `_other` warning clippy (`unused_variables` en match exhaustivo) | Lint | Renombrar a `_other` |
| `loop { ... break; }` que nunca iteraba (clippy `never_loop`) | Lint -D warnings | Refactorizar `read_array_u64` a una versión single-page sin `loop` |
| `forward_offsets.len() >= self.num_nodes as usize + 1` → clippy `int_plus_one` | Lint -D warnings | Cambiar a `> self.num_nodes as usize` |
| `self.forward_offsets[u as usize]` con `NodeId = usize` → clippy `unnecessary_cast` | Lint -D warnings | Quitar el `as usize` redundante |
| Test `csr_from_edges_with_self_loops`: tracé mal `adj_in` y puse valores esperados incorrectos | Test incorrecto | Recalcular manualmente y corregir aserciones |
| Test `persistent_csr_replace_keeps_invariants`: asumía que `backward_targets` != `forward_targets` para un grafo simétrico | Test incorrecto | Cambiar el grafo de test a uno asimétrico (DAG con distintas distribuciones de out/in) |
| Test `persistent_csr_create_load_empty_roundtrip`: el load devolvía `vec![]` para offsets (longitud 0) en vez de `vec![0]` (longitud 1) | Lógico | En `load()`, cortocircuitar a `Csr::empty()` cuando `num_nodes == 0` |

### Lecciones aprendidas en cap 14

1. **El "array vacío" en disco necesita una convención explícita**: cuando
   `num_nodes == 0` no hay nada que persistir, pero el formato debe
   distinguir entre "no hay página asignada" (vacío intencional) y "página
   corrupta". La convención `start_page == 0` ⇒ array vacío funciona
   mientras el pager reserve la página 0 para la metapágina y nunca la use
   para datos.

2. **Single-chunk arrays son suficientes para grafos pedagógicos**: 500
   nodos × 8 bytes/offset ≈ 4 KB cabe en una página. 1000 aristas × 4
   bytes/target ≈ 4 KB también. CSR de miles de nodos (grafos
   académicos/medianos) entran en una sola página. Grafos grandes
   (>100k nodos) requieren segmentación — eso es cap 15+.

3. **La suma total de degrees es una invariante cruzada fuerte**:
   `Σdegree_out(u) == Σdegree_in(u) == edge_count`. El test con
   generador LCG determinista (50 aristas aleatorias sobre 20 nodos)
   atrapa corrupción silenciosa que las invariantes individuales no
   detectarían.

4. **`Csr::verify()` se llama en TODOS los puntos críticos**: construcción,
   replace, load. Sí, es redundante en el caso normal, pero la
   invariante "nada corrupto llega a disco" vale la pena.

5. **El `unused_variables` de match exhaustivos** (`other => ...`) es un
   clippy reciente en 1.96 que rompe `cargo clippy -D warnings` aunque
   el match sea correcto. Renombrar a `_other` (o `_`) es la forma
   idiomática.

6. **`never_loop` detecta bucles que nunca iteran** (tienen `break` antes
   de cualquier `continue` o segunda iteración). Clippy 1.96 lo marca
   con `-D warnings`. Cuando el patrón "loop + break" se usa para
   proteger contra loops infinitos, hay que refactorizar a una versión
   lineal — como hicimos con `read_array_u64`.

---

## 19. Vol.II — Cap 15 (HashIndex + BPlusTree sobre BufferPool)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 6 (caps 7, 11, 12, 13, 14, 15) |
| Líneas Rust | ~+650 (cap 15) |
| Tests propios cap 15 | 27 (`tests_index`) |
| Dependencias externas añadidas | (ninguna) |

### Decisiones de diseño del cap 15

1. **Dos índices sobre `BufferPool<Pager>`, sin crates externas**. La regla
   "primero a mano, luego con crate" del Vol.II se respeta: ni `hashbrown`,
   ni `lru`, ni `redb`. Hash FNV-1a y binary search implementados in-house.

2. **`HashIndex` (estático, con desbordamiento encadenado)**:
   - Hash FNV-1a 64-bit sobre los 8 bytes LE de la clave `u64`.
   - `HashIndexHeader` (16 bytes) en la **página 2**: magic `0x4849_4431`
     ("HID1") + `num_buckets` + `key_count` + reserved. Sirve para
     detectar corrupción al reabrir.
   - **Páginas 3..(3+B-1)** = buckets primarios (B = num_buckets).
   - Cada bucket es una `SlottedPage` cuyo **primer record** es el
     `BucketHeader` (4 bytes: `next_page: u32`). Si `next_page != 0`,
     la cadena continúa en esa página. Los records siguientes son
     `HashEntry` (16 bytes: `key: u64` + `value: u64`).
   - **Overflow chain**: cuando una página de bucket está llena, se
     aloca una nueva y se cuelga del header de la anterior. La cadena
     se recorre linealmente en `get`.
   - `insert` busca primero en la cadena. Si encuentra la clave,
     reemplaza in-place. Si no, añade al final (allocating nueva página
     si la última está llena). `key_count` se actualiza en el catálogo.
   - `bucket_count` por defecto: 16 (`DEFAULT_BUCKETS`).

3. **`BPlusTree` (single-level, raíz = hoja)**:
   - `BPlusHeader` (16 bytes) en la **página 2**: magic `0x4250_4C55`
     ("BPLU") + `key_count` + reserved.
   - La raíz es la **única hoja**: contiene todas las entradas en orden
     ascendente de `key`, cada una como un `TreeEntry` (16 bytes) que
     es un record separado de la SlottedPage (header + N entries).
   - `get(key)` = `binary_search_by_key` O(log N).
   - `range_scan(lo, hi)` = iteración lineal filtrada.
   - **Limitación declarada**: sin splits ni deletes. Si la raíz se llena,
     `insert` devuelve `IndexError::InvalidParam("bplus root full…")`.
     Pedagógicamente correcto para grafos pequeños; la extensión a
     multi-nivel está prevista en caps. futuros.

4. **Errores tipados `IndexError`** con variantes específicas:
   - `Io(BufferPoolError)` — fallos de I/O.
   - `UnknownSlotKind(u8)` — slot desconocido al decodificar.
   - `Inconsistent(&'static str)` — invariantes rotas tras reopen.
   - `PageNotAllocated(PageId)` — catálogo apunta a página libre.
   - `InvalidParam(&'static str)` — overflow de dimensión (e.g.
     `num_buckets == 0` o raíz B+ llena).
   `From<PagerError>` y `From<BufferPoolError>` para `?` ergonómico.

5. **Magic + verificación al `open`**: ambos índices usan magics únicos
   (`HID1`, `BPLU`) en sus catálogos. Si un byte del magic está corrupto
   en disco, `open()` rechaza con `IndexError::Inconsistent("...bad magic")`.
   Tests `bplus_disk_bad_magic_fails` y
   `bplus_in_memory_open_after_corruption_fails` validan este contrato.

6. **Layout de página del catálogo BPlusTree corregido**: el `persist()`
   escribe el header como **primer record** de la `SlottedPage` y cada
   entry como un record independiente. Esto permite a `open()`
   distinguir records por tamaño (`BPlusHeader::SIZE` vs `TreeEntry::SIZE`)
   en vez de concatenar todo en un único blob. Decisión aprendida del bug
   detectado en implementación (ver tabla abajo).

7. **Páginas reservadas disjuntas entre HashIndex y BPlusTree**: ambos
   usan la página 2 como catálogo. Por tanto, **no pueden coexistir en
   el mismo `BufferPool`** en este cap. (Un grafo "real" usaría un
   catálogo global en la página 1; eso es tema de cap 8 / Apéndice D del
   brief.) El test `hash_and_bplus_coexist` documenta este comportamiento
   creando cada índice en pagers separados.

### Bugs corregidos durante la implementación del cap 15

| Bug | Tipo | Fix |
|---|---|---|
| `BPlusTree::create` sólo llamaba `allocate()` una vez; con pager recién creado (1 página) la página 2 no quedaba asignada | Lógico | `while !pool.pager().is_allocated(root_page) { pool.pager_mut().allocate()?; }` |
| `BPlusTree::persist()` concatenaba header+entries en un único record → `open()` leía sólo los 16 primeros bytes como header, ignorando las entries | Lógico | Reescribir `persist` para escribir header como primer record y cada entry como record independiente |
| Test `bplus_disk_bad_magic_fails` corrompía bytes 0..4 de la página pensando que era el magic del BPlusHeader, pero bytes 0..9 son el `PageHeader` y 10..14 son el length-prefix | Test incorrecto | Corromper bytes 14..18 (inicio del record BPlusHeader tras `PageHeader 10B + length-prefix 4B`) |
| `let mut sp_mut = sp.clone();` con `mut` innecesario | Lint -D warnings (`unused-mut`) | Quitar `mut` |
| `let mut bh_old = BucketHeader { next_page: 0 };` con valor inicial reasignado sin leer | Lint -D warnings (`unused-assignments`) | Inicializar directamente desde `BucketHeader::decode(&records_mut[0])?` |
| `let mut sp = SlottedPage::new(page_id, page_type);` reasignado en la siguiente línea antes de usarse | Lint -D warnings (`unused-assignments`) | Construir `sp` directamente como struct literal sin pre-`new` |

### Lecciones aprendidas en cap 15

1. **El "primer record" como "header lógico" es un patrón limpio** para
   catálogos persistentes en SlottedPages: el formato distingue header
   de entries por tamaño (16B vs 16B, en este caso concreto, pero con
   magic discrimante) en vez de offsets mágicos o padding.

2. **FNV-1a 64-bit es un buen hash "didáctico"**: 10 líneas de Rust,
   propiedades de dispersión razonables, y forma parte del canon
   (implementado por `fnv` crate, `hashbrown`, etc.). El test
   `fnv1a_known_values` verifica el offset basis y dos hashes canónicos
   ("" → 0xCBF29CE484222325, "a" → 0xAF63DC4C8601EC8C, "foobar" →
   0x85944171F73967E8). Cualquier desviación señala un bug en el código.

3. **El overflow chain del HashIndex no re-packing**: insert añade al
   final de la cadena, no re-organiza entries dentro de una página.
   Pedagógicamente correcto, aunque sub-óptimo. Una versión "con packing"
   recorrería la página llena y desplazaría entries para hacer hueco
   (más complejo; queda para cap futuro).

4. **`BPlusTree` raíz = hoja es la decisión correcta para un cap
   introductorio**: el alumno entiende "orden, binary search, range scan"
   sin la complejidad de splits, propagaciones y rebalancing. Los caps.
   28 (MVCC) y 36 (arquitectura final) son los lugares naturales para
   introducir B+ tree multi-nivel.

5. **`read_record_page` y `write_record_page` siguen siendo útiles** como
   helpers de "1 record por SlottedPage" para catálogos que NO contienen
   listas (HashIndexHeader). Para catálogos con listas (BPlusHeader +
   TreeEntry[]), escribimos la SlottedPage directamente. Esta separación
   refleja la jerarquía de complejidad: índice simple → índice ordenado.

6. **Detectar el "magic no encaja" requiere saber dónde está**:
   el magic del BPlusHeader vive en el offset 14 de la página (10 de
   `PageHeader` + 4 de length-prefix). Corromper el offset equivocado
   testea una ruta de error distinta (e.g. PageHeader magic mismatch).
   Documentar el offset en el comentario del test previene horas de
   "por qué falla mi test de bad-magic".

---

## 20. Vol.II — Cap 16 (Compactación y mantenimiento: inspect|check|compact)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 7 (caps 7, 11, 12, 13, 14, 15, 16) |
| Líneas Rust | ~+560 (cap 16) |
| Tests propios cap 16 | 25 (`tests_maintenance`) |
| Dependencias externas añadidas | (ninguna) |

### Decisiones de diseño del cap 16

1. **Tres operaciones que cierran la Parte III** (motor de almacenamiento),
   correspondientes a los hitos CLI del brief (§cap 16):
   - `inspect()` → [`StorageStats`]: totals (total/allocated/free pages),
     data/meta pages, `bytes_on_disk`/`bytes_used`/`bytes_free`,
     `total_records`, `fragmentation_ratio()` y `utilization()`.
   - `check()` → [`CheckReport`] con `Vec<IntegrityIssue>` (cada issue =
     `{ page_id, kind }`). Read-only: NO repara.
   - `compact()` → [`CompactReport`]: repack masivo + `inspect` antes/después.

2. **Invariantes verificadas por `check`** (definidas en caps 11-12):
   - Magic válido (`0xDA` Data / `0xFE` Meta) con `bytes[0] == bytes[1]`.
   - `header.page_id == offset físico` (detecta páginas movidas o
     reescritas en el lugar equivocado).
   - Records decodifican sin truncado (`SlottedPage::decode` OK).
   - `free_space` declarado == real (`PAGE_SIZE - header - Σ records`).
   [`IssueKind`] cubre cada caso: `BadMagic`, `PageIdMismatch`,
   `FreeSpaceMismatch`, `RecordTruncated`, `Undecodable`.

3. **`compact` es repack in-place, NO vacuum/truncate**. El `PageId` es un
   offset físico; mover páginas rompería CSR (cap 14), HashIndex/B+Tree
   (cap 15) y cualquier puntero interno. La compactación recupera espacio
   **dentro** de las páginas (alinea `free_space`, limpia bytes basura tras
   el último record) sin cambiar el tamaño del fichero. El vacuum/truncate
   queda declarado como limitación explícita — tema de caps 29 (recuperación)
   y 36 (arquitectura final).

4. **`repack_page` como unidad atómica**. Re-codifica una `SlottedPage`
   (records consecutivos + padding a cero + `free_space` recalculado) y la
   escribe al mismo `PageId`. No elimina records ni toca la metapágina
   (id 0 → `MaintenanceError::BadPageType`). Devuelve
   [`RepackResult`] con `bytes_reclaimed` (diferencia entre el
   `free_space` erróneo y el real).

5. **`inspect` y `compact` son tolerantes; `check` es exhaustivo**.
   `inspect` no aborta ante una página corrupta (la cuenta como data sin
   records). `compact` salta páginas corruptas (`pages_skipped`) sin
   tocarlas — la corrección de corrupción estructural es decisión humana
   tras leer `check`. `check` reporta TODOS los issues encontrados.

6. **Lectura vía `pager_mut()`, sin cachear en el pool**. El mantenimiento
   es offline: no queremos calentar la caché ni expulsar páginas útiles.
   Por eso se lee crudo (`[0u8; PAGE_SIZE]` + `pager.read`) en vez de
   `pool.get_page()`. Coherente con el patrón "administración ≠ hot path".

7. **Errores tipados [`MaintenanceError`]** paralelos a `IndexError` (cap 15):
   `Io(BufferPoolError)`, `BadPageType { expected, got }`,
   `DecodeFailed { reason }`, `PageNotAllocated`. Con `From<BufferPoolError>`
   y `From<PagerError>` para `?` ergonómico, como en todos los caps
   anteriores.

### Bugs corregidos durante la implementación del cap 16

| Bug | Tipo | Fix |
|---|---|---|
| `repack_page` mezclaba tipos `u16`/`usize`/`u32` al comparar `header.free_space` (u16) con `sp.free_space()` (usize) y con `free_before` | Compilación | Unificar a `usize` en las variables locales; castear a `u32` sólo al construir `RepackResult` |
| Tests `inspect_counts_records_and_pages` y `repack_page_corrije_free_space_corrupto` contaban length-prefixes de 4B que `SlottedPage::free_space()` excluye | Test incorrecto | Ajustar el cálculo esperado a `PAGE_SIZE - header - Σ record.len()` (sin prefix), coherente con la primitiva existente del cap 11 |
| Tests de persistencia via `FilePager` asumían que `FilePager::create` inicializa una `MetaPage` válida en página 0, pero la deja a ceros → `check` reportaba `BadMagic` en página 0 | Test incorrecto | Helper `filepool_with_meta()` que escribe una `MetaPage` válida tras `create`, igual que `TmpPager::new_with_meta()` |

### Lecciones aprendidas en cap 16

1. **`PageId` como offset físico es un contrato que limita el repack**.
   Una compactación "de verdad" (que reduzca el fichero) exige reescribir
   TODOS los punteros: CSR (offsets/columnas en páginas 1..N), catálogos de
   índices (página 2 + buckets/hojas), y metapágina. En un sistema real
   eso se hace con un snapshot consistente (cap 30) o offline. Declarar la
   limitación en el propio cap es más honesto que implementar un vacuum
   a medias que corrompería punteros.

2. **`SlottedPage::free_space()` es la fuente de verdad, no el header**.
   El `free_space` del header es metadato que PUEDE desincronizarse tras
   un crash a mitad de un update. `check` compara ambos; `repack` los
   reconcilia. Esta dualidad (metadato vs realidad) es exactamente lo que
   distingue a `inspect` (cuenta lo real) de una consulta ingenua al header.

3. **Mantenimiento offline = leer sin pool**. Pasar por `BufferPool` para
   un barrido completo sería contraproducente: llena los frames con páginas
   que no se volverán a usar y expulsa las calientes. El patrón
   `pager_mut().read(id, &mut buf)` con un buffer reutilizable es más
   simple y no contamina la caché. Lección transferible a backups,
   estadísticas y recovery.

4. **`check` read-only + `compact` reparador es la separación correcta**.
   Un `check` que repara implícitamente escondería corrupción real (magic
   corrupto = fallo de disco, no `free_space` desactualizado). Mantener
   `check` puramente diagnóstico permite al operador decidir qué reparar.

---

---

## 21. Vol.II — Cap 17 (Diseñar un lenguaje pequeño: LiraQL)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 8 (caps 7, 11, 12, 13, 14, 15, 16, 17) |
| Líneas Rust | ~+580 (cap 17) |
| Tests propios cap 17 | 41 (`tests_query`) |
| Dependencias externas añadidas | (ninguna) — diseño puro, sin crates |

### Decisiones de diseño del cap 17

1. **Diseño, no implementación**. El cap.17 fija el *qué* (tokens, gramática,
   AST, errores) pero no el *cómo* (lexer/parser). El lexer y el parser
   descendente manual llegan en el cap.18, el plan lógico en el cap.19 y el
   motor Volcano en el cap.20. Esta separación es coherente con el hito del
   brief (§cap 17): `pub enum AstNode { Match(MatchClause), Where(Expression),
   Return(ReturnClause) }` — un contrato de tipos, no un parser.

2. **Lenguaje "LiraQL" = mini-Cypher**. Sólo consulta (`MATCH-WHERE-RETURN`),
   recortado intencionadamente: no hay CREATE/MERGE/DELETE (DML del cap.31 de
   la CLI), ni WITH, ni OPTIONAL MATCH, ni recursión (SET en cap.22+). Las
   tres cláusulas son obligatorias (RETURN siempre presente). La gramática
   EBNF completa se documenta en el propio `lib.rs` (comentario del cap.17).

3. **`Span` en TODO el AST**. Cada nodo lleva `Span { start, end }` (rango
   semiabierto en bytes UTF-8), igual que `rustc`/`miette`/`codespan-reporting`.
   El patrón `QueryError { kind, span }` apunta al carácter exacto del error.
   El lexer del cap.18 rellenará los spans gratuitamente.

4. **Reutilización de tipos existentes**: `Expression::Literal` envuelve al
   `Value` del cap.7 (Int/Float/String/Bool/Null/Bytes) en vez de crear un
   enum `Literal` duplicado. Misma filosofía que el resto del Vol.II: los
   tipos fundamentales se definen una vez y se reutilizan.

5. **`TokenKind` deriva `PartialEq` pero NO `Eq`** porque contiene
   `TokenKind::Float(f64)` y `f64` no implementa `Eq` (NaN ≠ NaN). Esto es
   correcto: los lexemas flotantes no tienen igualdad total. Decision
   documentada en el campo `notes` de code-map.yml.

6. **Validación semántica (`Query::validate()`)** devuelve `Vec<QueryError>`
   (no `Result`) para reportar TODOS los errores de una vez (mejor UX que
   fallar en el primero). Reglas: MATCH no vacío, node pattern no trivial
   (no `()` puro), sin variables duplicadas (nodos, ni nodo↔arista), RETURN
   no vacío, alias no vacío, variables de WHERE/RETURN declaradas en MATCH.
   El alcance de variables incluye las de arista (`-[r:KNOWS]->` liga `r`).

7. **Pretty-printer (`Display`) canónico** para TODO el AST
   (`Expression`, `NodePattern`, `RelationshipPattern`, `PathPattern`,
   `MatchClause`, `WhereClause`, `ReturnClause`, `Query`). Útil para:
   (a) tests de round-trip en el cap.18, (b) `liradb explain` (cap.21),
   (c) normalización de consultas. `hex_bytes()` propio para `Value::Bytes`
   (sin crates).

8. **`AstNode` como enum** (no como struct `Query`) para coincidir
   literalmente con el hito del brief y permitir construir sub-árboles en
   tests y que el planner del cap.19 opere cláusula a cláusula.

### Bugs corregidos durante la implementación del cap 17

| Bug | Tipo | Fix |
|---|---|---|
| `TokenKind` derivaba `Eq` pero `Float(f64)` no implementa `Eq` | Compile error | Cambiar `#[derive(Debug, Clone, PartialEq, Eq)]` → `#[derive(Debug, Clone, PartialEq)]` |
| Clippy `approx_constant`: usé `3.14` como float de ejemplo (clippy lo confunde con `f64::consts::PI`) | Lint -D warnings | Cambiar `Value::Float(3.14)` → `Value::Float(2.5)` en los tests |
| Clippy `collapsible_if`: `if let Some(alias) { if alias.trim().is_empty() { ... } }` | Lint -D warnings | Colapsar con let-chaining: `if let Some(alias) = ... && alias.trim().is_empty() { ... }` (estable en Rust 2024) |
| `references_var` declarado privado pero sólo usado en tests | dead_code warning | Hacerlo `pub` (es parte conceptual de la API del AST, útil para el planner cap.19) |

### Lecciones aprendidas en cap 17

1. **`f64` no implementa `Eq`** (NaN ≠ NaN). Cualquier enum que contenga un
   `f64` no puede derivar `Eq`; sólo `PartialEq`. Es un detalle fácil de
   olvidar cuando se diseña un token kind que incluye literales flotantes.

2. **Let-chaining es la forma idiomática en Rust 2024** para anidar
   `if let` + condición. Clippy 1.96 lo sugiere con `-D warnings` cuando
   encuentra un `if` dentro de un `if let`. Requiere `edition = "2024"`.

3. **`validate() → Vec<Error>` (no `Result`)** es mejor UX para un lenguaje
   de consulta: el usuario quiere ver TODOS los errores de su query de una
   vez, no iterar fix-compile ciclos por cada fallo individual. Patrón opuesto
   al de los índices/buffer pool (caps 12-16), donde `Result` basta porque
   son operaciones que abortan en el primer fallo.

4. **Diseñar el AST antes del lexer/parser separa preocupaciones** y permite
   que el cap.18 (lexer/parser) se concentre en la mecánica del escaneo/parsing
   sin rediseñar los tipos. Es el mismo principio que "trait `Pager` como
   port antes de `FilePager`" (cap 12): definir el contrato primero.

5. **Reutilizar `Value` del cap.7** para los literales evita duplicar tipos
   y mantiene coherencia entre el modelo de datos y el lenguaje de consulta.
   Una decisión pequeña pero que paga dividendos en los caps.19-20
   (plan/executor operan sobre los mismos tipos).

---

## 22. Vol.II — Cap 18 (Lexer + parser descendente manual)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 9 (caps 7, 11, 12, 13, 14, 15, 16, 17, 18) |
| Líneas Rust | ~+770 (cap 18: lexer + parser + tests) |
| Tests propios cap 18 | 73 (`tests_lexer_parser`) |
| Tests totales workspace | 366 (293 previos + 73 nuevos) |
| Dependencias externas añadidas | (ninguna) — lexer y parser 100% manuales |

### Decisiones de diseño del cap 18

1. **Lexer y parser separados, no combinados**. El lexer produce `Vec<Token>`
   (con `Eof` final); el parser consume ese stream. Es la arquitectura clásica
   de dragon book: separar escaneo (regular, sin contexto) de parsing
   (context-free). Un parser combinado (scannerless) es posible pero oscurece
   la enseñanza del escaneo, que es objetivo explícito del brief (§11).

2. **Lexer 100% manual** (sin `logos`, sin `nom`). `Lexer` es un struct sobre
   `&[u8]` con un cursor `pos: u32`. El bucle `lex()` despacha por el primer
   byte de cada token (`scan_token`). La regla es **maximal-munch**: el token
   más largo posible gana, por lo que `->` se reconoce antes que `-` y `<>`
   antes que `<`. Esto es lo que todo lexer real hace; enseñarlo a mano vale
   más que delegar a `logos` en esta fase. La versión con `logos` llega en
   el apéndice comparativo (propuesta del brief §11).

3. **`Dash` se introduce formalmente aquí**. El cap.17 definió `ArrowRight`
   (`->`), `ArrowLeft` (`<-`) y `DashDash` (`--`) pero olvidó el guión simple
   `-`, necesario para los extremos de las relaciones entrantes y sin
   dirección (`-[ ... ]-` y `<-[ ... ]-`). El lexer del cap.18 lo produce;
   el parser lo consume. Es un fix retroactivo menor al vocabulario del cap.17.

4. **`Expression::Variable` para el hito `RETURN p`**. La gramática EBNF del
   cap.17 decía `primary ::= literal | property_access | '(' expression ')'`,
   que no contempla una variable sola. Pero el hito del brief (`RETURN p`)
   exige aceptar `p` como referencia al nodo completo. Se añade
   `Expression::Variable { name, span }`, distinta de `PropertyAccess` (que
   requiere `.propiedad`). Es coherente con Cypher real, donde `RETURN p`
   retorna el nodo y `RETURN p.name` retorna una propiedad.

5. **Parser descendente recursivo predictivo**. Una función por regla EBNF:
   `parse_match_clause`, `parse_path_pattern`, `parse_node_pattern`,
   `parse_relationship_pattern`, `parse_where_clause`, `parse_return_clause`,
   `parse_return_item`. La precedencia de operadores (`OR < AND < NOT <
   comparación`) se resuelve encadenando funciones (`parse_or → parse_and →
   parse_not → parse_comparison → parse_primary`): cada nivel consume
   operadores de menor precedencia y delega al siguiente para los más fuertes.
   Es la técnica clásica de "precedence climbing por funciones" (Wirth,
   Dragon Book). Sin tabla de precedencia, sin Pratt parser: el lector ve
   la jerarquía directamente en la pila de llamadas.

6. **Errores léxicos y sintácticos separados pero compatibles**. `LexError`
   (UnexpectedChar/UnterminatedString/InvalidEscape/IntegerOverflow/
   MalformedNumber) y `ParseError` (UnexpectedToken/MissingMatch/
   MissingReturn/MalformedRelationship/TrailingTokens/...). `ParseError`
   envuelve `LexError` vía `ParseErrorKind::Lex(LexError)` con `impl From` y
   `impl std::error::Error::source()`. El parser propaga el primer error
   léxico automáticamente sin código extra.

7. **Recovery minimalista: reportar y abortar**. El parser devuelve
   `Result<Query, ParseError>` y aborta en el primer error sintáctico. No hay
   recovery multi-error (estilo `pest`/`tree-sitter`). Es una decisión
   pedagógica: un único mensaje claro y bien localizado (con `Span` exacto)
   es más útil para aprender que una cascada de errores derivados. El
   recovery completo se deja como ejercicio y se menciona en el comentario
   del cap.18.

8. **`parse()` y `parse_query()` como API pública**. `parse(src)` es la
   entrada canónica; `parse_query` es un alias explícito. Ambas lexan y
   parsean en un paso. Para tests que inyectan tokens sintéticos existe
   `Parser::from_tokens(tokens)`.

9. **Spans precisos en TODO**. Cada token lleva su `Span` exacto (bytes
   `[start, end)`); el parser los propaga a los nodos del AST (unión de
   extremos). Los mensajes de error incluyen `(en start..end)` o
   `(en offset N)` según el span sea vacío o no, estilo rustc/miette.

### Bugs corregidos durante la implementación del cap 18

| Bug | Tipo | Fix |
|---|---|---|
| `TokenKind` del cap.17 no tenía `Dash` (guión simple) — imposible parsear `-[` y `]-` | Gap de diseño cap.17 | Añadir `TokenKind::Dash`; el lexer lo produce; actualizar test de cobertura de variantes |
| `Expression` del cap.17 no tenía variante para variable sola (`RETURN p`) — hito del brief no parseaba | Gap de diseño cap.17 | Añadir `Expression::Variable { name, span }`; actualizar `span()`/`references_var()`/`variables()`/`Display` |
| Lexer no reconocía `<>` como `NotEq` (sólo `<`, `<=`, `<-`) | Bug de escaneo | Añadir rama `self.match_byte(b'>')` → `TokenKind::NotEq` en la rama `b'<'` |
| Clippy `match_overlapping_arm`: caso `b' '` solapaba con `0x20..=0x7e` en `escape_byte` | Lint -D warnings | Eliminar el caso `b' '` redundante (cae en el rango ASCII imprimible) |
| Clippy `unnecessary_map_or`: `peek_next().map_or(false, ...)` | Lint -D warnings | Cambiar a `is_some_and(...)` |
| Clippy `approx_constant`: `3.14` en test de lexer | Lint -D warnings | Cambiar a `2.5` (no es constante famosa) |
| Round-trip estricto `q1 == q2` fallaba por paréntesis redundantes del fuente original que el `Display` canonicaliza | Test incorrecto | Verificar idempotencia de la forma canónica (`display(parse(display(parse(src)))) == display(parse(src))`) en vez de igualdad de spans |

### Lecciones aprendidas en cap 18

1. **Maximal-munch es la regla de oro del lexer**. Reconocer `->` antes que
   `-`, `<>` antes que `<`, y `--` antes que `-` evita ambigüedades. Olvidar
   una combinación (como pasó con `<>`) rompe el parser de formas sutiles
   que se manifiestan como "token inesperado" lejos del error real.

2. **El diseño del AST precede al lexer/parser** (lección del cap.17
   confirmada). El cap.18 no rediseñó ningún tipo fundamental: sólo añadió
   `Dash` (omisión del cap.17) y `Expression::Variable` (exigido por el
   hito del brief). Toda la mecánica del parser es código nuevo, pero los
   tipos son estables. Esto valida la decisión del cap.17 de fijar el
   vocabulario antes del escáner.

3. **Precedence climbing por funciones es legible**. No necesita tabla de
   precedencia ni Pratt parser: la pila de llamadas (`parse_or → parse_and →
   parse_not → parse_comparison → parse_primary`) espeja la jerarquía
   gramatical. Para un lenguaje pequeño como LiraQL es la opción más
   mantenible.

4. **`From<LexError> for ParseError` + `source()`** es el patrón idiomático
   para encadenar errores en Rust. El lexer produce `LexError`; el parser los
   propaga como `ParseErrorKind::Lex(...)` sin perder la cadena causal
   (`std::error::Error::source()` sigue apuntando al `LexError` original).

5. **Round-trip de ASTs con spans exactos es frágil**. El `Display`
   canonicaliza (normaliza paréntesis y whitespace), lo que cambia los spans.
   Para tests de round-trip hay que comparar o bien la forma canónica
   idempotente (`display(parse(display(parse(src))))`) o bien ignorar spans
   en la igualdad estructural. Comparar `Query` con `==` (incluyendo spans)
   sólo funciona si el fuente original ya está en forma canónica.

6. **`RETURN p` (variable sola) es un caso que Cypher resuelve con un tipo
   de expresión distinto**. Confundirlo con `PropertyAccess` sin propiedad
   habría roto la semántica del executor del cap.20 (un nodo no es una
   propiedad). La lección: modelar cada constructor sintáctico con su
   variante de AST, aunque parezcan casos de un mismo concepto.

---

## 23. Vol.II — Cap 19 (Del AST al plan lógico)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 10 (caps 7, 11, 12, 13, 14, 15, 16, 17, 18, 19) |
| Líneas Rust | ~+780 (cap 19: LogicalType, Bindings, ScalarExpr, LogicalPlan, PlanError, Planner, Display, tests) |
| Tests propios cap 19 | 40 (`tests_logical_plan`) |
| Tests totales workspace | 406 (366 previos + 40 nuevos) |
| Dependencias externas añadidas | (ninguna) — binder y planner 100% in-house |

### Decisiones de diseño del cap 19

1. **El plan lógico es un árbol de operadores, no código**. `LogicalPlan`
   (enum con `Box` hijos) declara *qué* calcular: `NodeScan` (hoja: liga una
   variable a los nodos con un label), `Expand` (un tramo de relación por
   nivel), `Filter` (predicados), `Project` (raíz: RETURN) y
   `CartesianProduct` (patrones disjuntos separados por coma). El cap. 20
   (Volcano) sólo tiene que recorrer el árbol operador a operador; el cap. 21
   sólo tiene que reescribirlo. Sin traits, sin magia: la estructura ES el plan.

2. **`Bindings` responde la pregunta crítica del CORPUS** ("cómo representar
   variables ligadas"): `Vec<(String, BindingKind)>` — nombre → `Node`/`Edge`
   — en orden de declaración. `Vec` en vez de `HashMap` para mantener el
   orden de ligadura determinista (tests, explain, executor) con coste O(n)
   aceptable para un lenguaje didáctico. `declare()` rechaza duplicados.

3. **`ScalarExpr` = `Expression` resuelto (el papel del binder)**. Diferencias
   con el AST: sin `Span` (los errores de plan citan el span de la cláusula
   originaria); `Variable` → `Var { name, kind }` con el `BindingKind`
   **incrustado** para que el executor nunca re-resuelva nombres; nueva
   variante `HasLabel { variable, label }` que NO existe en la sintaxis —la
   construye el planner (ver decisión 5).

4. **Predicados sin push-down, deliberadamente**. WHERE + propiedades inline
   (`{edad: 30}`) + labels de nodos no-iniciales se conjuntan (AND
   left-asociativo) en UN `Filter` sobre el plan del MATCH. Es correcto pero
   ingenuo: exactamente el "antes" que el optimizador del cap. 21 mejorará
   (su primera regla es push-down de predicados, y el ejemplo del brief
   transforma `Filter(name = "Ana") + NodeScan` en `IndexSeek`).

5. **Bug del brief corregido: el label del nodo destino se imponía de
   verdad**. El plan de ejemplo del brief para
   `MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.name = "Ana" RETURN f.name`
   omite cualquier operador/predicado que exija `f:Person` (el `NodeScan`
   sólo filtra `p`): tal cual, devolvería conocidos de CUALQUIER etiqueta.
   Fix: el label del nodo inicial alimenta el `NodeScan` (como el brief
   muestra), pero el de los nodos de la cadena baja como predicado
   `HasLabel` al `Filter` → `Filter(f:Person AND p.name = "Ana")`. Es el
   mismo patrón que Neo4j (label predicate) hasta que el optimizador
   reordena.

6. **Inferencia de tipos conservadora (schemaless)**. `LogicalType`
   (Any/Null/Bool/Int/Float/String/Bytes/Node/Edge): las propiedades tipan
   `Any` y `Any`/`Null` son comodines compatibles con todo (la comparación
   concreta se resuelve en ejecución, cap. 20). Sólo se rechaza lo que
   *seguro* está mal: WHERE no-booleano (`WHERE 3`), igualdades imposibles
   (`p = TRUE` con `p` nodo), órdenes imposibles (`TRUE < FALSE`,
   `1 < "x"`), operandos no-bool de AND/OR/NOT. Numéricos cruzados
   (Int vs Float) y comparaciones con Any pasan.

7. **Nodos/relaciones anónimos → variables internas** `(_n1, _e2, …)` con
   contador que salta nombres ocupados (evita colisiones con variables de
   usuario que empiecen por `_`). Nota: `()` desnudo lo rechaza el
   `validate()` del cap 17 (pedagógicamente "inútil"), pero el binder del
   cap 19 lo acepta y liga con interna — divergencia documentada en tests.

8. **Límites declarados como errores tipados** (material caps. 20-21):
   re-ligar una variable en el mismo patrón (`(a)-[:X]->(a)`) →
   `VariableRebind` (el re-binding es del executor); patrones separados por
   coma que comparten variables → `SharedPatternVariables` (exige join);
   ambos con mensajes que apuntan al cap que los resolverá.

9. **Pretty-printer del plan como base de `liradb explain`**. `Display`
   dibuja el árbol con indentación de 2 espacios por nivel en el formato
   exacto del brief (`Project(f.name)` / `Expand(p, KNOWS, OUTGOING, f)` /
   `NodeScan(Person AS p)`); `ScalarExpr` usa paréntesis mínimos por
   precedencia NOT > AND > OR (el `Filter` del ejemplo imprime
   `b:Person AND (a.age > 30 OR b.age > 40)` sin ruido).
   `LogicalPlan::bound_variables()` expone las variables ligadas en orden
   — lo que el push-down del cap. 21 necesita para saber qué predicados
   puede mover.

10. **API mínima**: `lower(&Query) -> Result<LogicalPlan, PlanError>` +
    `Query::lower()` (atajo). El pipeline completo queda
    `parse(src)?.lower()?` — el cap. 20 añadirá el paso a filas.

### Bugs corregidos durante la implementación del cap 19

| Bug | Tipo | Fix |
|---|---|---|
| Plan del brief omitía imponer `f:Person` (habría devuelto conocidos de cualquier etiqueta) | Bug de diseño del brief | Labels de nodos de la cadena bajan como `HasLabel` al Filter (decisión 5); divergencia documentada en banner + test canónico |
| Paréntesis mínimos: `Or` dentro de `And` no se envolvía (`a OR b AND c` por `(a OR b) AND c`) | Bug lógico en Display | Regla de wrap por contexto: `Or` se envuelve en ctx `And`/`Not`; `And` en ctx `Or`/`Not` |
| Sintaxis `box` en patrones sigue experimental (E0658 en 1.96) | Compile error | Destructurar con `let … else` en dos pasos (`Project` → `input.as_ref()` → `NodeScan`) |
| Test esperaba `Project(p)\nNodeScan(...)` sin indentar el hijo | Test incorrecto | El render siempre indenta hijos 2 espacios por nivel; corregir expectativas |
| Clippy `len_zero`: `err.span.len() > 0` | Lint -D warnings | `!err.span.is_empty()` |
| Test de `type_of` no declaraba `p` en los Bindings → `UnknownVariable` | Test incorrecto | Declarar `p` como Node antes de usar `ScalarExpr::prop("p", …)` |

### Lecciones aprendidas en cap 19

1. **El plan de un brief/tutorial puede ser "correcto de pinta" pero
   semánticamente incompleto**: nadie extraña el predicado de label hasta
   que la consulta devuelve filas de más. Al traducir un plan de ejemplo a
   código ejecutable, cada restricción del patrón (label, props, dirección,
   tipo) debe quedar representada en algún operador o predicado — el test
   canónico del display es la red de seguridad.

2. **Separar "expresión sintáctica" (`Expression`) de "expresión resuelta"
   (`ScalarExpr`) es el patrón binder de las bases de datos reales**
   (Ladybug/Kùzu: Parser → Binder → Planner). Incrustar la resolución en el
   propio nodo (`Var { kind }`) elimina toda re-resolución downstream: el
   executor del cap. 20 evaluará sin tablas de símbolos.

3. **El plan ingenuo es pedagógicamente necesario**: generar un `Filter`
   arriba de todo (sin push-down) y un `CartesianProduct` para patrones
   disjuntos da al cap. 21 un "antes" concreto que optimizar. Ocultar la
   ingenuidad ahora le robaría al optimizador su razón de existir.

4. **Paréntesis mínimos en pretty-printers de expresiones exige reglas por
   contexto**, no un flag global: la misma expresión se envuelve distinto
   según cuelgue de `AND`, `OR` o `NOT`. Y los asociativos (`a AND b AND c`)
   no se envuelven entre sí. Testear cada combinación de anidamiento evita
   displays ambiguos que romperían un futuro re-parseo del explain.

## 24. Vol.II — Cap 20 (El motor de ejecución: modelo Volcano)

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 11 (caps 7, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20) |
| Líneas Rust | ~+1.080 (cap 20: Cell/Row, eval_scalar con lógica trivalente, trait PhysicalOperator, 8 operadores, compile, Executor, ResultSet, métricas, tests) |
| Tests propios cap 20 | 37 (`tests_executor`) |
| Tests totales workspace | 443 (406 previos + 37 nuevos) |
| Dependencias externas añadidas | (ninguna) — motor 100% in-house sobre `GraphStore` del cap 8 |

### Decisiones de diseño del cap 20

1. **La fila es un par `(nombre, Cell)`, no un array posicional**. `Row`
   (`Vec<(String, Cell)>`) materializa en ejecución los `Bindings` del cap
   19: `NodeScanOp` crea la fila, `ExpandOp` la extiende (clonando:
   materialización explícita), `CartesianProductOp` concatena con
   `merge()` y `ProjectOp` produce la fila de SALIDA re-ligando cada
   `Projection` a su `output_name()`. Con UN solo tipo de fila, el trait
   `PhysicalOperator` queda uniforme (open/next/close devuelven siempre
   `Row`) — no hacen falta dos jerarquías (binding rows vs output rows).

2. **`Cell = Scalar(Value) | Node | Edge` resuelve `RETURN p` sin magia**.
   Una variable entera evalúa al ELEMENTO (así funciona Cypher); una
   propiedad o comparación evalúa a escalar. La igualdad de nodos/aristas
   es POR IDENTIDAD de id (`WHERE a = b` encuentra self-loops), coherente
   con `eq_compatible(Node, Node)` que el cap 19 ya aceptaba.

3. **Semántica de evaluación = SQL/Cypher, no de Rust**: NULL domina toda
   comparación (`p.missing > 30` → NULL, la fila se descarta); Int/Float
   promocionan; igualdad entre tipos distintos es `false` (Cypher) pero su
   ORDEN es NULL; sólo números y cadenas se ordenan (espejo exacto de
   `order_compatible` del cap 19). AND/OR/NOT en lógica trivalente
   (FALSE∧NULL=FALSE, NULL∨TRUE=TRUE…) con CORTOCIRCUITO REAL: `FALSE AND
   x` no evalúa `x`. El cortocircuito es observable y testeado: si `x`
   sería un `TypeMismatch` runtime (propiedad Int usada como bool), la
   rama corta lo elide.

4. **El trait `PhysicalOperator` es la interfaz del brief ampliada con
   observabilidad**: `open`/`next`/`close` (la tríada Volcano exacta del
   brief §cap 20) + `name()`/`rows_produced()`/`collect_metrics()`. Las
   métricas por operador (`ExecMetrics`, pre-orden raíz→hojas) cuentan lo
   que DE VERDAD fluyó — la semilla del `explain` con cardinalidades
   reales del cap 21 (que añadirá las estimadas).

5. **Ocho operadores, un solo patrón de estado**. `NodeScanOp` (cursor
   perezoso sobre `iter_nodes`; orden del store = determinista),
   `IndexSeekOp` (liga los NodeIds que recibe: la SELECCIÓN de índice se
   difiere al optimizador del cap 21, que es donde el brief lo usa; id
   inexistente → `UnknownNode`, índice desactualizado), `ExpandOp` (bucle
   anidado clásico: fila externa del input + candidatos de adyacencia por
   dirección; UNDIRECTED = out+in con el self-loop UNA vez),
   `FilterOp`, `ProjectOp`, `CartesianProductOp`, `LimitOp`, `DistinctOp`.
   `Limit`/`Distinct` son operadores completos aunque la gramática LiraQL
   (caps 17-18) aún no exponga las keywords: quedan listos para la CLI
   (cap 31) y el optimizador, documentado en el operador.

6. **`CartesianProductOp` MATERIALIZA su lado derecho en `open()`** — la
   lección Volcano que motiva el cap 21: los iteradores son monotónicos
   (nadie rebobina), así que re-leer un lado exige copiarlo. Ese coste
   (memoria + filas de más antes del filtro) es exactamente el "antes"
   que el optimizador eliminará reordenando el punto de partida.

7. **El executor consume el PUERTO, no la implementación**: todo el
   motor va contra `&dyn GraphStore` (cap 8). `MemoryStore` hoy; el store
   en disco de la Parte III mañana, sin tocar el motor. `PropertyGraph`
   (cap 7) queda como estructura didáctica del modelo, no como store de
   consultas — decisión documentada en el banner del capítulo.

8. **`compile()` es 1:1 POR AHORA**: `LogicalPlan` → árbol físico sin
   reescrituras. Es deliberado: el cap 21 insertará ahí el push-down de
   filtros, `NodeScan`→`IndexSeek` y la reordenación de expansiones. El
   `Filter` arriba del árbol que produce `lower()` se ejecuta tal cual y
   las métricas dejan ver su ineficiencia (4 filas escaneadas para
   devolver 1) — el mejor material didáctico para el optimizador.

9. **Ciclo de vida explícito**: `Executor::execute()` hace
   `open → next* → close` con `close` SIEMPRE (también tras error, como
   un defer); `open` resetea estado para re-ejecutar (testeado:
   open-close-reopen del scan). `Executor::new` exige `Project` raíz
   (`NotAProjection` si no) porque las columnas salen de sus items —
   invariante que `lower()` ya garantiza.

10. **API de tres niveles**: `compile(plan, store)` (operadores
    programáticos: los tests de Limit/Distinct envuelven el árbol a
    mano), `Executor` (ciclo + métricas + ResultSet) y `run(src, store)` /
    `Query::execute(&store)` (el hito del brief: consultas completas
    desde texto). `ExecError` envuelve `ParseError`/`PlanError` con
    `From` + `source()` para que el pipeline entero reporte con un solo
    tipo; los errores propios son de EJECUCIÓN (`TypeMismatch` runtime es
    la concreción del `Any` schemaless que el cap 19 no podía rechazar).

### Bugs corregidos durante la implementación del cap 20

| Bug | Tipo | Fix |
|---|---|---|
| Helper placeholder `input_store()` dejado en `ExpandOp::new` (compilaba como `-> !`) | Residuo de redacción | `ExpandOp` recibe `&'a dyn GraphStore` explícito como el resto de hojas; helper eliminado |
| `while let Some(id) = cursor.next()` con `self.*` dentro | Clippy `while_let_on_iterator` | `for id in cursor.by_ref()` (préstamos de campos disjuntos siguen funcionando) |
| Bucles `while pos < len` que retornaban en la primera iteración (IndexSeek, Cartesian) | Clippy `never_loop` | Reestructurar a `if pos >= len { return None }` / condición compuesta `if let` (let-chains, edición 2024) |
| Test esperaba 2 filas en `age < 40` olvidando a Carla (29) | Test incorrecto | Recalcular fixture completo: 3 filas (Ana→Bo, Carla→Ana, Dani→Dani) |
| Anchos de la tabla `ResultSet` contados a ojo (4 espacios) | Test incorrecto | Expectativas exactas tras `trim_end`: el pad es `max(cabecera, celdas)` por columna |
| Asignar `dyn GraphStore` desde `&MemoryStore` en helpers de tests necesitaba lifetime nombrado | E0106 | `fn compilar_interno<'a>(src: &str, store: &'a MemoryStore) -> Box<dyn PhysicalOperator + 'a>` |
| `matches!(&err, …{ context, expected, got })` liga referencias dobles | E0277 (`&str` vs `str`) | Desreferenciar en el guard: `*context == "Filter (WHERE)"` |

### Lecciones aprendidas en cap 20

1. **Un solo tipo de fila simplifica el trait Volcano**: dejar que
   `Project` produzca filas "de salida" (entradas nombradas por columna)
   en vez de un tipo distinto elimina genéricos y dobles jerarquías. La
   convención "debajo de Project las entradas son variables; encima,
   columnas" se cumple sola porque el planner sólo pone Project en la
   raíz.

2. **El cortocircuito sólo es pedagógico si es OBSERVABLE**: prometerlo
   en caps 17/19 y cumplirlo en el 20 exigía un test donde la rama elidida
   habría ERRADO (propiedad Int como bool). Ese test convierte una nota al
   pie en un comportamiento verificable.

3. **Volcano se explica mejor con sus límites**: el cartesiano que
   materializa, el scan que produce 4 filas para que el filtro devuelva
   1, el limit que corta el pull de raíz — las métricas por operador
   NUMERAN esas ineficiencias y preparan el discurso del optimizador.

4. **Guardar un iterador que pide prestado al store en `&mut self` se
   resuelve copiando la referencia ANTES**: `let store = self.store;` en
   `open()` desacopla el lifetime del cursor ('a) del de `&mut self`, y
   los préstamos de campos disjuntos (cursor/store/label/produced) dentro
   de `next()` compilan sin gymnastics.

## 25. Vol.II — Hito CLI mínima (binario `liradb`, tras el cap 20)

> NO es el cap. 31: es la "CLI mínima anticipada tras el cap. 20" que el
> ADR-005 (medidas de utilidad transversales) encarga para que LiraDB sea
> demostrable desde shell a mitad del libro. El cap. 31 la EXPANDIRÁ
> (REPL interactivo, import/export CSV/GraphML, clap con subcomandos
> ricos, configuración); nada de eso se adelanta aquí.

| Métrica | Valor |
|---|---|
| Caps. Vol.II migrados | 11 (sin cambio: el hito no es un capítulo) |
| Líneas Rust | ~+437 en crate nueva `liradb-cli` (Cargo.toml + lib.rs 390 con tests + main.rs 22) y ~+65 en `vol2-liradb` (`demo_graph()` público + fixture delegando) |
| Tests propios | 11 (CLI, end-to-end sin spawn) + 1 doctest (`demo_graph`) |
| Tests totales workspace | 455 (443 previos + 12 nuevos), ALL_GREEN |
| Dependencias externas añadidas | (ninguna) — parseo de args manual con `std`; única dependencia: `vol2-liradb` por path |

### Decisiones de diseño del hito CLI mínima

1. **Nombre: package `liradb-cli`, binario `liradb`**. El package sigue
   el prefijo del directorio (`crates/vol2-liradb-cli/`, coherente con
   `vol2-liradb`), pero un `[[bin]] name = "liradb"` hace que la orden
   de usuario sea `liradb demo` — no `liradb-cli demo`. Se construye con
   `cargo run -p liradb-cli -- demo` y, tras `cargo install --path`,
   directamente `liradb demo`.

2. **Parseo de argumentos MANUAL con `std::env::args`** — sin clap, por
   la regla del Vol.II "primero a mano, luego con crates". Un `match`
   sobre el primer argumento basta para tres subcomandos (`demo`,
   `query "<LiraQL>"`, `help`/sin args). clap y su ayuda generada
   llegan con la CLI completa del cap. 31; la regla está documentada en
   el banner del crate y en `main.rs`.

3. **`demo_graph()` público en `vol2-liradb`** (no en la CLI): el grafo
   demo ya existía como fixture privado de los tests del cap. 20
   (`tests_executor::grafo`, el de Ana/Bo/Carla/Dani + Madrid/Lisboa).
   En vez de duplicarlo en la CLI, se promueve a API pública con doctest
   y el fixture de tests pasa a delegar en él — un único punto de verdad
   para el grafo de las demos (si el Vol.III trae KB-Lira, crece ahí y
   todas las demos lo ven). La CLI no conoce la estructura del grafo:
   sólo llama al helper.

4. **Testabilidad: lib con `run(args, out, err) -> i32`, main fino**.
   Toda la lógica vive en `src/lib.rs`: `run` recibe `argv[1..]`, dos
   `&mut dyn Write` (stdout/stderr) y devuelve el código de salida.
   `src/main.rs` sólo recoge argv, delega y propaga el exit code. Así
   los 11 tests ejercitan la CLI real end-to-end (argumento → tabla/plan
   → exit code) escribiendo en un `Vec<u8>`, sin spawn de procesos ni
   assertions sobre el binario compilado.

5. **Exit codes Unix**: 0 = OK · 1 = error de consulta (parse/plan/
   ejecución, con su `Display` a stderr) · 2 = error de uso
   (subcomando desconocido, arity incorrecto). Los tres tipos de error
   de consulta están testeados por separado (`error de sintaxis`,
   `error de planificación`, `tipos incompatibles en Filter (WHERE)`),
   cubriendo las tres capas del pipeline caps. 18/19/20.

6. **Escrituras tolerantes a E/S**: un helper `emitir` hace
   `let _ = write_all(...)` en vez de `unwrap()` — una CLI didáctica no
   debe entrar en pánico si la salida se cierra antes de tiempo
   (`liradb demo | head -1`). Pedagógico y práctico.

7. **`demo` enseña el pipeline entero**: cada bloque imprime consulta →
   plan lógico (`Display` del cap. 19, la base de `liradb explain`) →
   tabla (`Display` del `ResultSet` del cap. 20) → métricas reales por
   operador (`ExecMetrics`). Para ello `demo` desenrolla `run()` en
   `parse → lower → Executor` (todo API pública del cap. 20) en vez de
   llamar al atajo: necesita el plan antes de ejecutar y las métricas
   después. Colateral didáctico: las métricas NUMERAN la ineficiencia
   del plan ingenuo (4 filas escaneadas para devolver 1) — el mejor
   anuncio del optimizador del cap. 21.

8. **Alcance congelado**: nada de REPL, import/export, flags, ficheros
   de configuración ni `--explain` (que llegará con el cap. 21 como
   subcomando o flag). La ayuda es un literal `r#"..."#` con
   subcomandos y 2 ejemplos, exactamente lo que el hito pide.

### Bugs corregidos durante el hito CLI mínima

| Bug | Tipo | Fix |
|---|---|---|
| Test y doctest esperaban 2 filas en `age < 40` olvidando a Dani (36) | Test incorrecto (mismo resbalón que en cap 20) | 3 filas: Ana, Carla y Dani; Bo (41) queda fuera |
| Anchos de tabla supuestos a ojo en los asserts (`"p.name | p.age"`) | Test incorrecto | `"Carla"` (7 chars con comillas) fija el ancho de columna en 7: `"p.name  | p.age"` con doble espacio |
| `verify.sh` BLOCKED por import y línea larga | rustfmt (`cargo fmt --check`) | `cargo fmt --all` (orden de imports: tipos antes que funciones) |

### Lecciones aprendidas en el hito CLI mínima

1. **Una CLI testable se diseña al revés**: primero `run(args, out,
   err) -> i32`, después el `main`. Testear el binario compilado
   (spawn + pipe + exit code) es posible pero frágil; testear la
   función que main llama es gratis y cubre lo mismo.

2. **El grafo de demo es API, no fixture**: cuando tests y herramienta
   comparten datos, el dato sube a la API pública con doctest — el
   fixture privado se queda como alias de una línea y deja de poder
   divergir de lo que ve el usuario.

3. **El mismo resbalón dos veces es una lección**: el fixture de cap 20
   ya pilló a alguien con "age < 40 son 2 filas" (Dani, 36, existe).
   Repetirlo en los tests de la CLI confirma que el fixture debe ser
   único y compartido: donde hay dos copias hay dos bugs iguales.

---

## 26. Vol.II — Cap 21 (Un optimizador pequeño pero real: `liradb explain`)

**Estado**: ALL_GREEN (375 unit + 2 doctests en vol2-liradb; 455→485 tests workspace).
**Módulo**: `crates/vol2-liradb/src/cap21_optimizador.rs` (primer capítulo que nace directo
como módulo propio, ADR-007) + subcomando `explain` en `liradb-cli`.

**Métricas**: ~1.900 líneas el módulo, 30 tests (`tests_optimizer`). Colateral: cap 19 ganó el
operador `IndexSeek` (el plan lógico debe poder EXPRESAR el uso de índice para que el
optimizador lo elija) y cap 20 su compilación a `IndexSeekOp`.

**Decisiones**:
1. **Catálogo de estadísticas** recolectado del `&dyn GraphStore` al abrir: nodos por etiqueta,
   grado medio out/in por etiqueta, aristas por tipo. Sin persistir: recalcular al abrir es lo
   pedagógicamente simple.
2. **Reglas en orden fijo y documentado** (fn `optimize(plan, &stats)`): predicate pushdown
   (bajar Filter hacia NodeScan/Expand respetando bindings), combinación de Filters adyacentes,
   reordenación por selectividad estimada. El orden es parte del contrato didáctico.
3. **Estimación por heurísticas simples** (selectividad por tipo de predicado + grados medios),
   no por muestreo: suficiente para ORDENAR planes, no para prometer costes. `explain` muestra
   estimación vs filas reales — la discrepancia es contenido pedagógico, no bug.
4. **Equivalencia testeada**: los resultados pre/post optimización son idénticos en todas las
   consultas del brief y del demo.
5. El agente quedó interrumpido por usage-limit ANTES de actualizar docs y formatear; el
   orquestador completó fmt + fix de imports sin usar + verificación + docs. Lección: un agente
   cancelado puede dejar trabajo válido sin verificar en el árbol — `git status` antes de
   lanzar el siguiente.

## 27. Vol.II — Cap 22 (Caminos mínimos ponderados: Dijkstra + Bellman-Ford)

**Estado**: ALL_GREEN (405 unit + 5 doctests en vol2-liradb; 486→519 tests workspace).
**Módulo**: `crates/vol2-liradb/src/cap22_caminos_minimos.rs`. Abre la Parte V (algoritmos
sobre el grafo persistente).

**Métricas**: ~1.290 líneas el módulo, 30 tests (`tests_caminos`) + 3 doctests. Sin crates
externas (`BinaryHeap` de std, como el cap 4 del Vol.I).

**Decisiones**:
1. **Fuente de pesos** (`WeightSource`): propiedad de arista configurable (`Property(name)`,
   como `WEIGHT relationship.distance` del brief) o `Constant` (Default `1.0` = contar
   saltos). Semántica ESTRICTA y tipada en `edge_weight`: ausente o NULL → `MissingWeight`,
   tipo no numérico → `InvalidWeight` (con `Value::type_name`), NaN/±∞ → `NonFiniteWeight`,
   Int→Float con pérdida >2^53 documentada y testeada. Un grafo schemaless debe VER sus
   problemas de dato, no silenciarlos con un default.
2. **Sobre `&dyn GraphStore`, no sobre el CSR**: los pesos viven en `Edge.props` y el CSR del
   cap 14 sólo persiste topología (sin ids de arista) — la proyección con pesos llega en el
   cap 26. El CSR queda como oráculo de consistencia en un test (alcanzabilidad BFS sobre la
   proyección == `reached()` de Dijkstra).
3. **Negativos**: Dijkstra valida TODAS las aristas eagermente y rechaza
   (`NegativeWeight`) aunque la consulta no vaya a tocar esa zona — una BD prefiere fallar
   ruidosamente a contestar casi-bien. Bellman-Ford los acepta y sólo se rinde con un ciclo
   negativo ALCANZABLE desde el origen (`NegativeCycle`, señalando una arista que aún
   relaja); el inalcanzable no contamina.
4. **Misma interfaz para ambos**: tabla `ShortestPaths` (dist+pred+`PathStats`) + camino
   `Path` (pasos con arista/peso, `nodes()`, `hops()`, Display estilo Cypher) y variantes
   `_path` punto-a-punto. Dijkstra con finalización anticipata al destino (invariante
   codicioso) y Bellman-Ford SIN early-exit por destino (nada lo justifica sin el invariante)
   pero con parada temprana de pasadas. `CostOverflow` evita confundir un infinito real con
   el centinela `INFINITY` de inalcanzable.
5. **`Cost`**: newtype f64 con `Ord` total para el heap (f64 no puede: los NaN); el
   `expect` de `partial_cmp` es unreachable documentado porque todos los costes se validan
   finitos antes de entrar.
6. Corregido al vuelo: el bloque integrador `vol2-liradb` de code-map.yml seguía sin listar
   `cap21_optimizador` y las stats/next_action apuntaban al cap 21 ya hecho (deuda de la
   interrupción de la Sesión 15) — actualizados junto al cap 22.

---

## 28. Vol.II — Cap 23 (A*, heurísticas y búsquedas dirigidas)

**Estado**: ALL_GREEN (419 unit + 9 doctests en vol2-liradb; 519→537 tests workspace).
**Módulo**: `crates/vol2-liradb/src/cap23_a_estrella.rs`. Parte V (algoritmos sobre el
grafo persistente), continuación del cap 22.

**Métricas**: ~1.000 líneas el módulo, 14 tests (`tests_a_estrella`) + 4 doctests. Sin crates
externas (`hypot` de std para la euclídea). Cambios quirúrgicos en `cap22_caminos_minimos.rs`
(Ver abajo).

**Decisiones**:
1. **La heurística es un TRAIT, no una closure**: `Heuristic { estimate(&self, store, node)
   -> Result<f64, PathError> }`. Porqué: las heurísticas son familias CON ESTADO ligadas a un
   destino (la euclídea recuerda destino y nombres de props; una de landmarks precalcularía
   tablas); el contrato (finita, ≥ 0, h(dest)=0 si admisible) se documenta y valida EN UN
   SITIO (a_star revisa cada estimación); y `&dyn Heuristic` evita genéricos contagiosos. Una
   closure no podría leer el store (coordenadas) sin capturarlo prestado. Implementaciones:
   `ZeroHeuristic` (h≡0) y `EuclideanHeuristic` (recta por props x/y de NODO, con la MISMA
   semántica estricta que `edge_weight`: MissingCoordinate/InvalidCoordinate, Int promociona;
   destino validado eager en `new`, resto on-demand).
2. **Reutilización máxima del cap 22**: `a_star` usa tal cual WeightSource/`edge_weight`,
   `Path`, `PathStats`, `PathError` y la sanidad eager de pesos — EXTRAÍDA del cuerpo de
   `dijkstra_impl` a `validate_edge_weights` pub(crate) compartida (refactor puro, mismo
   comportamiento; también `ensure_node`/`table_len` pasaron a pub(crate)). Heap por
   `f = g + h` con clave `Reverse<(Cost(f), Cost(g), NodeId)>` (Cost, no f64: Ord total); con
   h≡0 la clave degenera en la de Dijkstra y el orden de pops es IDÉNTICO (testeado).
3. **Validación HONESTA, por coste**: eager lo barato (pesos O(E), h finita y ≥0 por
   estimación cacheada ≤1 vez/nodo — un NaN haría PANIC en `Cost::cmp`); ADMISIBILIDAD no
   verificable (exigiría el coste real = resolver Dijkstra) → documentada y el riesgo
   DEMOSTRADO con tests (h sobre-estimada y unidades km/min mezcladas ⇒ subóptimo en
   silencio); CONSISTENCIA sí es local O(E) → utilidad `check_consistency` como diagnóstico
   (`InconsistentHeuristic{edge, h_from, bound}`), pero `a_star` NO la exige.
4. **Re-apertura, no `settled`**: con h admisible pero inconsistente un nodo expandido puede
   mejorar su g y debe re-expandirse; las entradas obsoletas del heap se detectan por
   `g_entrada > g[v]`. Resultado: A* sigue devolviendo el ÓPTIMO con heurísticas sólo
   admisibles, al precio de expandir más (medido en tests: 5 expansiones para 4 nodos).
   Rechazar la inconsistencia habría sido rechazar respuestas correctas.
5. **`PathStats` extendido con `expanded`** (pops vivos, sin obsoletos): añadido al struct
   del cap 22 e incrementado también por `dijkstra_impl`, que es lo que hace posible la
   comparativa Dijkstra vs A* (13 vs 10 en el grafo-trampa; 7 vs 3 en la red de ciudades del
   hito). `PathError` extendido a 12 variantes (5 nuevas: MissingCoordinate,
   InvalidCoordinate, NonFiniteHeuristic, NegativeHeuristic, InconsistentHeuristic).
6. **Sólo punto-a-punto**: sin variante single-source — el sesgo hacia el destino es la
   gracia y las distancias intermedias no quedan garantizadas (con h inconsistente ni las
   de los nodos tocados). f puede desbordar a ∞ (sólo prioridad; `Cost` tolera ±∞), g no
   (`CostOverflow` heredado).
7. Corregido durante la implementación: MemoryStore NO reemplaza ids de arista existentes
   (`DuplicateEdge`) — un test que mutaba una arista in situ se reescribió con store fresco;
   y los doctests nuevos necesitaban `use vol2_liradb::GraphStore` para los métodos del trait
   (misma lección que los doctests del cap 22).

---

## 29. Vol.II — Cap 24 (Centralidad y PageRank)

**Estado**: ALL_GREEN (447 unit + 15 doctests en vol2-liradb; 537→571 tests workspace).
**Módulo**: `crates/vol2-liradb/src/cap24_centralidad.rs`. Parte V (algoritmos sobre el
grafo persistente), capítulo 3.

**Métricas**: ~1.800 líneas el módulo, 28 tests (`tests_centralidad`) + 6 doctests. Sin
crates externas. Cero cambios en módulos previos (sólo lib.rs: mod/pub use/cabecera).

**Decisiones**:
1. **Proyección materializada, no store en el bucle**: `Proyeccion { nodes, index,
   vecinos }` compacta el store UNA vez (ids ordenados → determinismo; índice denso
   NodeId→posición que deja los huecos de `delete_node` fuera del cálculo; vecindarios
   por dirección). Los algoritmos iterativos tocan la adyacencia iteraciones×E veces:
   re-leer `out_edges`+`get_edge` en cada ronda sería pagar el store O(n) veces. Es la
   forma en memoria del CSR del cap 14; la proyección CON PESOS es el cap 26 (deuda
   explícita del closeness ponderado).
2. **`GraphDirection` (no `Direction`)**: colisión con el `Direction` forward/backward
   del CSR (cap 14) en la API plana. `Both` = unión como CONJUNTO (vecinos distintos,
   self-loop una vez — convención Expand UNDIRECTED del cap 20): sin dedup, un store
   simetrizado a mano contaría cada par doble. `In` = transpuesta de la salida pura
   (BUG corregido durante la implementación: la primera versión mezclaba in-edges en la
   colección y ADEMÁS transponía).
3. **Alcance según guion** (brief: grado, closeness, betweenness, eigenvector, PageRank
   "para explicar familias, sin optimización industrial"): TODAS implementadas. Grado
   O(V+E) normalizado por n-1; closeness por BFS de saltos con WASSERMAN-FAUST
   (((r-1)/(n-1))·((r-1)/Σd)) para componentes desconectadas — el ponderado queda como
   deuda hacia cap 26; betweenness = BRANDES 2001 (σ, predecesores, dependencias hacia
   atrás; O(V·E)) con normalización dirigida 1/((n-1)(n-2)) que sobre grafo simetrizado
   reproduce el libro no dirigido (camino 0-1-2-3 → 2/3, estrella → 1).
4. **Eigenvector = el "antes" honesto**: iteración de potencia sobre adyacencia CRUDA
   (x_u ← Σ_{v→u} x_v) con L2 por paso (la masa colgante ESCAPA; sin renormalizar el
   vector colapsaría). Sus DOS fallos son tests: estrella hojas→centro con hojas a 0
   (quien no recibe enlaces muere) y cola+3-ciclo que OSCILA (converged=false tras
   agotar iteraciones) — el MISMO grafo donde PageRank converge: el damping no es un
   truco numérico, hace la matriz positiva (primitiva) y garantiza convergencia.
5. **PageRank**: damping ∈ (0,1) ABIERTO por ambos extremos (0 = puro teleport, 1 =
   eigenvector — que existe como función propia por eso). OJO: `Range::contains` no
   sirve para excluir el 0 (el inicio es INCLUSIVO) — comparación explícita. Dangling
   redistribuido UNIFORMEMENTE (Brin-Page 1998; variante no-scale documentada y no
   implementada): masa total = 1 en CADA iteración, invariante testeado. Convergencia
   por **L1** (la masa que se mueve: interpretable como probabilidad y comparable entre
   grafos de distinto tamaño; max-delta documentado y descartado por sin lectura de
   masa). `PageRankResult.history` guarda el delta de CADA iteración: la razón
   geométrica ≈ d·λ₂ es contenido del capítulo (testeada monótona y < 1; y el contraste
   del grafo que arranca en el estacionario: history = [0.0], una iteración).
6. **PPR separado del global para GraphRAG (cap 51)**: `enum Teleport { Uniform,
   Personalized(Vec<(NodeId, f64)>) }` — el MISMO núcleo `iteracion_de_potencia` para
   `page_rank` y `personalized_page_rank` (cero duplicación). Validación del teleport:
   pesos ≥ 0 (negativo señalado por nodo), masa > 0, nodos existentes. El cap 51
   enchufará su operador de recuperación como `Teleport::Personalized` sin tocar el
   núcleo.
7. **Multigrafo con sutileza documentada**: cada arista paralela vale 1/grado — el
   duplicado NO duplica el voto (también duplica el denominador) pero SÍ roba masa a
   los otros vecinos (test). Brandes cuenta paralelas como caminos distintos (σ
   consistente); grado las cuenta una por arista.
8. Corregido durante la implementación: además del bug de In/Both, el razonamiento a
   mano del test de demo_graph ignoraba que el self-loop de Dani TAMBIÉN cobra la
   cuota colgante uniforme → su score real es 0.386 (TOP), no 1/6 — recalculado el
   valor exacto y reescrito el test como lección (self-loop + masa colgante = trampa
   de acumulación). Y NaN en `assert_eq!` de errores con f64 no compara (NaN != NaN
   bajo PartialEq) → `matches!`.


---

## 30. Vol.II — Cap 25 (Comunidades y agrupaciones)

**Estado**: ALL_GREEN (469 unit + 19 doctests en vol2-liradb; 571→597 tests workspace).
**Módulo**: `crates/vol2-liradb/src/cap25_comunidades.rs`. Parte V (algoritmos sobre el
grafo persistente), capítulo 4.

**Métricas**: ~2.200 líneas el módulo, 22 tests (`tests_comunidades`) + 4 doctests. Sin
crates externas. Cero cambios en módulos previos (sólo lib.rs: mod/pub use/cabecera).

**Decisiones**:
1. **`GrafoPonderado` propio, no la `Proyeccion` del cap 24**: la del cap 24 es NO
   ponderada y su `GraphDirection::Both` hace unión como CONJUNTO (deduplica
   paralelas — correcto para contar vecinos distintos, FALSO para Louvain, que debe
   SUMAR pesos). Louvain además RECONSTRUYE el grafo en cada agregación: el
   `GrafoPonderado` es a la vez la proyección inicial Y el grafo de nivel
   (`contraer()` devuelve otro `GrafoPonderado`). Se heredan el PATRÓN del cap 24
   (ids ordenados → determinismo, índice denso, materializar una vez) y la semántica
   ESTRICTA de pesos del cap 22 (`WeightSource`/`edge_weight` con
   `From<PathError>` → `ComunidadesError::Weight`).
2. **Semántica de pesos y dirección, documentada en el contrato**: cada arista
   dirigida u→v aporta w al par {u,v} (un store simetrizado a mano SUMA 2w); las
   paralelas ACUMULAN (multigrafo → multipeso, testeado: 3 paralelas de peso 1 ≡ una
   de peso 3 con idéntico resultado); los self-loops se separan con la convención
   estándar **A_ii = 2s** (cuentan doble en k_i y 2m — el self-loop de Dani en
   `demo_graph` le da comunidad propia sin unirlo a nadie); los pesos NEGATIVOS se
   rechazan eager (como Dijkstra en cap 22: la modularidad con negativos rompe el
   modelo nulo).
3. **Modularidad como función VERIFICABLE** (`modularidad(store, particion, weight,
   gamma)`): calculable sobre CUALQUIER partición dada — es la métrica guía del
   algoritmo y el oráculo de los tests (Q del resultado == `modularidad()` de la
   misma partición, testeado en cada caso). Nodos ausentes de la partición →
   singletons; ids de grupo u64 arbitrarios (densificados internamente — cuidado
   con la asignación O(max_id) si no se densificara); γ de Reichardt-Bornholdt
   validado (0, negativo, NaN e ∞ rechazados — con el mismo `matches!` para NaN
   que el cap 24).
4. **Louvain simplificado, determinista**: fase local greedy con ΔQ EXACTO (diferencia
   de los dos términos de comunidad que cambian — misma fórmula que las
   implementaciones de referencia, autocontenida aquí) y sólo ΔQ>0 ESTRICTO;
   agregación conservando 2m (⇒ la Q de cada nivel es igual en el grafo contraído y
   en el original, INVARIANTE TESTEADA nivel a nivel); niveles hasta que una fase no
   mueva nada. **Cota de terminación demostrable**: cada nivel arranca de singletons,
   su primer movimiento vacía una comunidad ⇒ el siguiente nivel tiene estrictamente
   menos nodos ⇒ niveles ≤ V. `max_pasadas` por nivel = seguro anti-ruido de f64
   (ΔQ ≈ 0). Determinismo TOTAL sin barajar (el Louvain de la literatura baraja):
   nodos por id, candidatos por id de comunidad, empates por `total_cmp` → el
   primero gana, renumeración por menor miembro — dos ejecuciones idénticas,
   testeado incluso con orden de inserción de aristas invertido.
5. **La jerarquía es el producto para el cap 51 (GraphRAG)**: `NivelLouvain` lleva la
   asignación de los nodos ORIGINALES (composición de particiones) + Q + nº de
   comunidades + stats por nivel; anidamiento garantizado por construcción
   (OJO al assertirlo: la dirección correcta es "misma comunidad en el nivel ℓ ⇒
   misma en el ℓ+1" — los niveles bajos son FINOS, fundir es fusionar);
   `particion_en(nivel)` construye el dendrograma a demanda.
6. **El límite de resolución como TEST estrella** (Fortunato-Barthélemy 2007): anillo
   de 12 tríos — γ=1 funde pares adyacentes (6 comunidades, Q=17/24 > 2/3 de los
   tríos sueltos; DOS niveles de jerarquía 12→6), γ=2 los recupera (12 tríos
   exactos, Q=7/12). La métrica guía explica el fenómeno: los valores Q_γ de ambas
   particiones se verifican analíticamente con `modularidad()`.
7. **Hallazgos corregidos durante la implementación** (lecciones, no bugs del código):
   (a) el LPA determinista GOTEA por los puentes con pesos uniformes — el primer
   grupo que se forma arrastra al vecino del puente cuya etiqueta propia aún no
   reúne votos; política de empates "conservar la propia si empata con la máxima,
   si no la menor" + pesos que rompan empates; y el camino 0-1-2 se funde ENTERO
   (cascada de votos) — documentado en test en vez de imaginar particiones que la
   heurística no tiene por qué encontrar. (b) Un puente PESADISIMO no "fusiona todo":
   Q es invariante a escala y la mejor partición ROMPE los tríos alrededor del
   puente ({0,2},{1,4},{3,5} con Q=100/2809, mejor que la trivial 0) — el test
   enseña que los pesos RESTRUCTURAN, no que "más peso = más fusión". (c) En
   `demo_graph` hay DOS óptimos con la MISMA Q=5/18 ({0,2,4},{1,5},{3} y
   {0,1,2,4,5},{3}) separados por un dq EXACTAMENTE 0 — el test afianza Q y las
   pertenencias que no dependen del camino, no la partición completa. (d)
   `max_pasadas=1` NO rebaja la calidad final: lo que una pasada deja a medias en
   el nivel 0, la agregación lo repara en el nivel 1 (la jerarquía desbloquea
   movimientos) — testeado.

---

*Mantenido por: code-integration-architect (skill del BOOK-WORKFLOW).*
*Próxima revisión: tras Vol.II cap 26 (Ejecutar algoritmos sin agotar la memoria):
la proyección CON PESOS sobre el CSR del cap 14 que la Parte V espera — saldará la
deuda del closeness ponderado (cap 24) y podrá unificar la proyección simétrica
ponderada del cap 25 con la vista por bloques/frontiers del guion.*