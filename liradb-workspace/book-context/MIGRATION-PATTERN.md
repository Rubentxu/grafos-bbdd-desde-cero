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

## 9. Métricas de la Fase M1

| Métrica | Valor |
|---|---|
| Caps. migrados | 3 (cap. 2, 3, 4) |
| Crates creadas | 4 (cap. 2 tiene 2 crates: vec-adj + petgraph) |
| Líneas Rust migradas | ~270 |
| Tests migrados | 13 (3 + 2 + 5 + 3 propios) |
| Bugs del libro encontrados y corregidos | 5 |
| Tiempo desde bootstrap hasta ALL_GREEN | ~10 min (incluyendo debugging) |

---

*Mantenido por: code-integration-architect (skill del BOOK-WORKFLOW).*
*Próxima revisión: tras Fase M3 (caps. 5-20 del Vol.I).*