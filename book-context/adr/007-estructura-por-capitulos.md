# ADR-007 — Estructura del monorepo por volúmenes y capítulos

**Fecha**: 2026-08-14
**Estado**: Aprobada
**Contexto**: Con la prosa del Vol.II a punto de comenzar (0/40 caps redactados), la estructura heredada tenía dos puntos de dolor: (1) manuscritos como ficheros gigantes (Vol.I = 14.936 líneas) incompatibles con el flujo de redacción por capítulo y sus estados PLANNED→DRAFTING→DONE del LEDGER; (2) `crates/vol2-liradb/src/lib.rs` acumulando los caps 7-20 en ~15.500 líneas, cuando el plan original ya preveía «módulos separados por capítulo». El autor pidió explicitamente la división y estructuración por capítulos y volúmenes.

## Decisión

1. **Manuscritos por capítulo**: `manuscrito/volN/` con un fichero por unidad (portada, prólogo, tabla de contenidos, secciones/parte, `cap-NN-slug.md`, apéndices, epílogo, colofón). El orden de ensamblado vive en `manuscrito/volN/SUMARIO.txt`.
2. **Ensamblados commiteados**: `scripts/build_book.sh` concatena según cada SUMARIO y regenera los ficheros completos en la raíz (`vol1-…rust.md`, `vol2-construye-liradb.md`, `vol3-grafos-era-ia.md`) con sus nombres históricos — los enlaces existentes no se rompen y el libro completo es legible en GitHub sin ejecutar nada. Modo `--check` para detectar desincronización.
3. **Vol.I también dividido** (decisión del autor): el ensamblado regenerado es **idéntico byte a byte** al original (verificado con `cmp`) — la 2ª edición publicada queda preservada y trazable.
4. **Código por capítulo**: `vol2-liradb/src/` se divide en un módulo por capítulo (`cap07_modelo.rs` … `cap20_volcano.rs`); `lib.rs` queda sólo con docs del crate + `mod` + `pub use` (API pública sin cambios, 455 tests). Las crates `vol1-cap-NN-*` ya estaban por capítulo. Los capítulos nuevos añaden su módulo.
5. **Divisor reutilizable**: `scripts/split_manuscrito.py` documenta la heurística de límites (encabezados de nivel 1 reconocidos; `# Cargo.toml` dentro de bloques NO es límite).

## Consecuencias

- La redacción de prosa trabaja sobre `manuscrito/volN/cap-NN-*.md`; tras editar, se ejecuta `scripts/build_book.sh` y se commitean fuente + ensamblado juntos.
- `code-map.yml` pasa a apuntar a ficheros/módulos exactos por capítulo (navegación directa prosa↔código).
- Diffs de git por capítulo = trazabilidad editorial fina (qué capítulo cambió en cada commit).
- Regla: **nunca se edita el ensamblado a mano**; siempre las fuentes + build.
