# ADR-006 — Monorepo único público (manuscritos + workspace)

**Fecha**: 2026-08-14
**Estado**: Aprobada
**Contexto**: El plan original (2026-07-30) aprobó un repo Git **separado** para el workspace de código. Tras el hito CLI mínima, el autor decidió: *«todo debe ser un único repositorio git, trazable y público»*. Existían además dos riesgos: 9 capítulos del Vol.II sin commitear y los manuscritos sin ningún control de versiones.

## Decisión

1. **Monorepo único** `grafos-bbdd-desde-cero`: manuscritos (Vol.I-III) + `book-context/` en la raíz, y **todo** el workspace de código bajo `liradb-workspace/`. Sin repos anidados ni submódulos.
2. **Publicación en GitHub (público)**: https://github.com/Rubentxu/grafos-bbdd-desde-cero — rama `main`.
3. **Historial preservado y reencauzado**: los 12 commits del workspace antiguo se reescribieron con prefijo `liradb-workspace/` (plumbing: `read-tree --prefix` + `commit-tree`, preservando autores, fechas y mensajes; `git filter-branch` no disponible en el entorno). El commit `c63fb26` importa los manuscritos encima.
4. **Commit previo del trabajo pendiente**: caps 12-20 + hito CLI mínima quedaron commiteados (`6df4ee1`) ANTES de la reescritura — nada de trabajo verificado se perdió.

## Consecuencias

- La decisión de «repo separado» del plan original queda **superseded**; el resto del plan (validación, code-map, disciplina) sigue vigente sin cambios.
- Disciplina reforzada: **sólo se commitea/pushea con ALL_GREEN** (`liradb-workspace/scripts/verify.sh`, 455 tests).
- `.gitignore` raíz excluye: `.zcode/`, `.atl/` (estado local de agentes) y `vol1-v3-backup-*.md` (backup redundante del Vol.I; el historial git es ahora la trazabilidad).
- `LICENSE` raíz: CC BY-NC-SA 4.0 para toda la obra; código de terceros en el workspace conserva su licencia de origen.
- README del monorepo reescrito: 3 volúmenes, estructura del repo, cómo ejecutar `verify.sh` y `liradb demo`.
- Los paths absolutos `~/Documentos/Libros-AI/liradb-workspace` citados en sesiones/memorias previas pasan a ser `~/Documentos/Libros-AI/grafos-bbdd-desde-cero/liradb-workspace`.
