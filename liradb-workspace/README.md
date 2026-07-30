# liradb-workspace

**Workspace de ejemplos ejecutables** para la obra *"Grafos en Computación: de Cero a Experto"*, en dos volúmenes:

- **Volumen I** — *"Grafos en Computación: de Cero a Experto"* (Rust edition 2024, `petgraph` + crates seleccionadas).
- **Volumen II** — *"Construye LiraDB"* — proyecto integrador: una base de datos de grafos embebida, construida desde cero en Rust.

Este repositorio es la **única fuente de verdad del código** mostrado en los libros. Los manuscritos (en [`grafos-bbdd-desde-cero/`](../grafos-bbdd-desde-cero/)) referencian este workspace; **no duplican** el código.

---

## Quickstart

```bash
# 1. Clonar
git clone https://github.com/rubentxu/liradb-workspace.git
cd liradb-workspace

# 2. Verificar (compila + tests + lints + fmt)
./scripts/verify.sh

# 3. (opcional) Verbose — ver todo el output de cargo
./scripts/verify.sh --verbose

# 4. (opcional) Saltar un paso (debugging)
./scripts/verify.sh --skip=lint
```

**Requisitos**: Rust stable ≥ 1.96, `cargo`, `python3` (para parsear el `stack-profile.yml`). Sin otras dependencias externas.

---

## Estructura

```
liradb-workspace/
├── README.md
├── Cargo.toml                  # workspace virtual
├── Cargo.lock                  # pinneado, committeado
├── rust-toolchain.toml         # canal estable pinneado
├── LICENSE                     # CC BY-NC-SA 4.0
├── .gitignore
├── planning/
│   └── stack-profile.yml       # comandos del verificador
├── scripts/
│   └── verify.sh               # ejecutor de la cadena
├── crates/                     # crates Rust (una por capítulo o módulo)
│   ├── vol1-cap-NN-...         # Volumen I
│   ├── vol2-cap-NN-...         # Volumen II (caps. ilustrativos)
│   ├── vol2-liradb/            # Proyecto integrador LiraDB
│   └── vol2-liradb-cli/        # CLI binario (cap. 31)
├── snippets/                   # extractos sueltos sin crate
├── tests/                      # tests de integración cross-crate
├── build/                      # output del verificador (gitignored)
│   └── verify-report.jsonl
└── book-context/
    ├── code-map.yml            # tabla bidireccional capítulo ↔ crate/snippet
    └── snippets.yml            # inventario de snippets sueltos
```

---

## Política de calidad

**Ningún capítulo del libro se publica sin `ALL_GREEN`**. Esta es la puerta §8 del workflow `BOOK-WORKFLOW.md` y se aplica tanto a los snippets del Vol.I (migrados) como al código nuevo del Vol.II.

La cadena de verificación es, en orden:

1. `cargo fmt --all --check` — formato canónico
2. `cargo check --workspace --all-targets --locked` — compilación rápida (sin binarios)
3. `cargo test --workspace --locked` — tests (deben pasar)
4. `cargo clippy --workspace --all-targets --locked -- -D warnings` — lints, **warnings como errores**

Si algún paso falla, el verificador devuelve `BLOCKED` con detalle en `build/verify-report.jsonl`. La remediación se delega a la skill `code-example-generator` (errores de compilación/test) o `chapter-writer` (lógica incorrecta).

---

## Política de versiones

- **`Cargo.lock` se commitea** al repo (no se regenera por CI).
- **`rust-toolchain.toml`** pinea el canal estable exacto (actualmente `1.96.0`).
- **Las dependencias externas se pinean** en los `Cargo.toml` individuales (sin `^` ni `~`) cuando la versión es crítica para la reproducibilidad.
- **`main` siempre verde**: sólo se mergea con `ALL_GREEN`.

---

## Política de tags

- `chapter-NN` — un tag por cada cap. que alcanza `DONE` (Vol.I y Vol.II).
- `vol2-liradb-vX.Y` — tag por cada hito del motor LiraDB (e.g. `v0.1-lite`, `v1.0-final`).
- `main` — siempre verde (verificado por el último `verify.sh` antes del push).

---

## Cómo se relaciona con los libros

| Libro | Ubicación | Estado |
|---|---|---|
| Vol.I (manuscrito) | `../grafos-bbdd-desde-cero/vol1-grafos-de-cero-a-experto-rust.md` | Publicado |
| Vol.II (manuscrito) | `../grafos-bbdd-desde-cero/vol2-construye-liradb.md` | En escritura |
| **Workspace de código** (este repo) | `/` | Bootstrap (Fase M0) |

El code-map bidireccional está en `book-context/code-map.yml`. Cuando un cap. se mueve a `DONE` en el workflow del libro, su entrada correspondiente del code-map debe estar sincronizada con las crates/snippets reales de este workspace.

---

## Licencia

CC BY-NC-SA 4.0. Ver `LICENSE` para el texto completo.

Atribuciones a terceros:

- **Kùzu / Ladybug** (papers seminales sobre arquitecturas GDBMS modernas) — el Vol.II se inspira en el Kùzu VLDB 2023 paper y las publicaciones de Semih Salihoğlu. La reimplementación es **clean-room conceptual**: ningún código de Kùzu/Ladybug ha sido copiado. Ver `book-context/ATTRIBUTIONS.md` (cuando se cree).

---

## Contribución

PRs sólo contra capítulos en `PLANNED` o `IN_REVIEW` (estado reflejado en `../grafos-bbdd-desde-cero/book-context/LEDGER.md`). Toda PR debe adjuntar `build/verify-report.jsonl` con `ALL_GREEN`.

---

*Mantenedor: Rubentxu — bootstrap creado el 2026-07-30.*