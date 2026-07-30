# MIGRATION-PATTERN — Patrón de migración Vol.I → workspace

> Documento vivo. Se actualiza con cada lección aprendida al migrar snippets.
> Última revisión: 2026-07-30 (Fase M1: caps. 2, 3, 4 del Vol.I).

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

## 11. Métricas de la Fase M3b

| Métrica | Valor |
|---|---|
| Caps. migrados | 6 (caps 6, 7, 8, 17, 18, 19) |
| Crates creadas | 6 |
| Dependencias externas añadidas | `petgraph 0.6` (cap-19); `rand 0.9` (cap-17) |
| Líneas Rust migradas | ~870 |
| Tests propios añadidos | ~25 |
| Bugs del libro encontrados y corregidos | 4 |

### Bugs del Vol.I corregidos durante Fase M3b

| Cap | Bug | Tipo | Fix |
|---|---|---|---|
| 6, 7, 18 | `for u in 0..n { ... adj[u] ... }` (needless_range_loop) | Clippy lint | Iterador `for (u, adj_u) in adj.iter().enumerate()` |
| 7 | `strongconnect` con 8 args (too_many_arguments) | Clippy lint | `#[allow(clippy::too_many_arguments)]` |
| 7 | Test `basico` omitía `union(0, 4)` | Test incompleto | Añadido union |
| 7 | Test esperaba `size_of(0) == 4` con 5 elementos | Bug lógico | `size_of(0) == 5` |
| 8 | `if x { if y { ... } }` (collapsible_if) | Clippy lint | Colapsado en `if x && y { ... }` |
| 17 | `rand 0.8` usa `rng.gen::<T>()` que choca con keyword `gen` Rust 2024 | Compilación | Bump a `rand 0.9` |
| 17 | Seeds del libro (e.g. `seed=42`) con StdRng 0.9 (ChaCha12 vs ChaCha20) | Monte Carlo | Brute force como oráculo determinista |
| 18 | `branch_and_bound_is` no validaba adyacencia al incluir | Bug lógico | Comprobación `adj[next].iter().all(\|nb\| !chosen.contains(nb))` |

### Lecciones aprendidas en Fase M3b

1. **Rust 2024 reserva la keyword `gen`** — cualquier código de Rust 2021 que usaba `gen` como método (típicamente `rand::Rng::gen::<T>()`) ahora falla al compilar. rand 0.9+ ya está migrado y usa `rng.random::<T>()`.

2. **Las seeds de Monte Carlo son específicas de la versión de `rand`**. Si subes rand 0.8 → 0.9, los seeds que el libro garantizaban ahora pueden no converger. Solución: brute force como oráculo, no depender de un valor exacto del algoritmo Monte Carlo.

3. **Branch & Bound para IS requiere validación de adyacencia** al ramificar. El libro omite esta comprobación — bug lógico que produce "soluciones" inválidas.

4. **DSU test del libro tiene 2 errores**: omite una union, y la aserción del tamaño no cuadra con el número de elementos. Migrar con cuidado: leer el código Y los tests, validar cada assert contra la lógica.

---

*Mantenido por: code-integration-architect (skill del BOOK-WORKFLOW).*
*Próxima revisión: tras Fase M3c (caps. 9, 12, 13, 14, 15, 16, 20 con crates pesadas).*

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

*Mantenido por: code-integration-architect (skill del BOOK-WORKFLOW).*
*Próxima revisión: tras Fase M3b (caps. 6, 7, 8, 9, 17, 18, 19, 20).*