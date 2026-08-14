# Apéndice 0 — Manual de estilo unificado

> *Borrador inicial — se completará en la Fase 2.*

## 0.1. Por qué un manual de estilo común

Esta obra se publica en **dos volúmenes** con voces distintas:

- El **Volumen I** ("Grafos en Computación: de Cero a Experto") tiene una voz **narrativa y divulgativa**, basada en el estilo que Aditya Bhargava popularizó como "Grokking Algorithms": hooks, anécdotas históricas, regla de tres, humor inesperado, ASCII art, "Pin de batalla", "Si solo lees 30 segundos", "Una historia pequeña" y Diálogos de ascensor.

- El **Volumen II** ("Construye LiraDB") tiene una voz **ingenieril y metódica**, basada en la plantilla pedagógica de 10 pasos del brief original de LiraDB: objetivo → problema → modelo mental → primera solución → sus límites → solución evolucionada → código completo ejecutable → prueba de fuego → qué hemos sacrificado → cómo lo hace una BBDD real + retos.

Ambas voces son válidas y complementarias. El Vol.I te enseña *qué es* un grafo y *qué algoritmos existen*. El Vol.II te enseña *cómo construir* un sistema que los persiste y los consulta. La fusión en una sola obra exige un manual que documente ambas plantillas y diga **cuándo y cómo se aplican**.

## 0.2. Las dos plantillas lado a lado

| # | Plantilla Vol.I (Grokking 2.0) | Plantilla Vol.II (híbrida) |
|---|---|---|
| 1 | `# Capítulo N — Título evocador` | `# Capítulo N — Título evocador` |
| 2 | `## N.0 La anécdota de la esquina` | `## N.0 La anécdota de la esquina` |
| 3 | `## N.1 ...` (cuerpo técnico libre, 4-12 secciones) | `## N.1 Objetivo` … `## N.10 Cómo lo hace una BBDD real + retos` (10 pasos fijos) |
| 4 | `## Ejercicios resueltos` | `## Ejercicios resueltos` (con niveles) |
| 5 | `## Ejercicios propuestos` | `## Ejercicios propuestos` (con niveles) |
| 6 | `## Lo que te llevas` | `## N.11 Lo que te llevas` |
| 7 | `## Ojo, cuidado con…` | `## N.12 Ojo, cuidado con…` |
| 8 | `## Para profundizar` | `## Para profundizar` |
| 9 | `## Pin de batalla` | `## N.13 Pin de batalla` |
| 10 | `## Si solo lees 30 segundos` | `## N.14 Si solo lees 30 segundos` |
| 11 | `## Una historia pequeña` | `## N.15 Una historia pequeña` |
| 12 | (sólo en Parte VI) `## Diálogo de ascensor / Mini-diálogo` | `## Mini-diálogo: en guardia nocturna` |

**Regla**: en el Vol.II, el orden es **fijo** y la sección técnica va numerada `N.1`–`N.10` con los títulos del brief LiraDB. No se eligen baterías sueltas.

## 0.3. Tabla "qué batería aplica en qué volumen"

| Batería | Vol.I | Vol.II |
|---|:-:|:-:|
| Anécdota de apertura | ✅ siempre | ✅ siempre (N.0) |
| 10 pasos LiraDB | ❌ no aplica | ✅ siempre (N.1–N.10) |
| Lo que te llevas | ✅ siempre | ✅ siempre (N.11) |
| Ojo, cuidado con… | ✅ siempre | ✅ siempre (N.12) |
| Pin de batalla | ✅ siempre | ✅ siempre (N.13) |
| Si solo lees 30 segundos | ✅ siempre | ✅ siempre (N.14) |
| Una historia pequeña | ✅ siempre | ✅ siempre (N.15) |
| Ejercicios resueltos | ✅ siempre | ✅ siempre |
| Ejercicios propuestos | ✅ siempre | ✅ siempre (esencial/intermedio/experto) |
| Para profundizar | ✅ siempre | ✅ siempre |
| Diálogo de ascensor | ⚠️ sólo Parte VI Vol.I | ✅ siempre (mini-diálogo) |

## 0.4. Reglas de transición entre volúmenes

- Cualquier referencia a un concepto del Vol.I desde el Vol.II debe incluir la notación `(Vol. I, cap. N)`.
- El **cap. 32 del Vol.I** (Quantum Computing) cierra el Vol.I invitando al lector a continuar con el Vol.II.
- El **cap. 1 del Vol.II** ("Qué es realmente un grafo") abre citando explícitamente los caps. 1-2 del Vol.I como prerequisito.
- Los caps. 21-32 del Vol.I (Grafos en la Informática Moderna) funcionan como "semilleros" del Vol.II: cada uno termina con una nota al pie apuntando al capítulo del Vol.II que implementa lo que ese cap. introdujo.

## 0.5. Política de versiones Rust y `Cargo.lock`

- Cada capítulo del Vol.II incluye su propio `Cargo.toml` con versiones **pineadas** (sin `^` ni `~`).
- Cada workspace de capítulo incluye `rust-toolchain.toml` con la versión exacta de Rust stable usada para escribirlo.
- El `Cargo.lock` se commitea al repositorio, no se regenera por CI.
- Si una versión de crate queda obsoleta durante la escritura, se documenta en `book-context/CHANGELOG.md` y se abre una incidencia; **no se reescriben caps ya publicados**.

## 0.6. Convención de cross-references

- `(Vol. I, cap. N)` — referencia al Volumen I.
- `(Vol. II, cap. N)` — referencia al Volumen II.
- `(cap. N)` sin prefijo — referencia dentro del mismo Volumen.
- `(LiraDB §N.M)` — referencia a una sección del workspace `liradb-workspace/`.

## 0.7. Glosario de términos estructurales

| Término | Significado |
|---|---|
| **Capítulo** | Unidad principal (~200-700 líneas). Numerado dentro de cada Vol. |
| **Parte** | Agrupación de 5-8 capítulos. Numerada en romanos. |
| **Batería** | Sección recurrente fija. |
| **Reto esencial/intermedio/experto** | Niveles de ejercicios en Vol.II. |
| **Claim** | Afirmación técnica con `claim_id` y `confidence_score`. |
| **Evidence card** | Recorte verificable de fuente, extraído por `source-researcher`. |
| **Code card** | Snippet de código Rust con `Cargo.toml` asociado. |
| **ADR** | Architecture Decision Record (Apéndice D Vol.II). |

*(El Manual de estilo se completará con ejemplos canónicos cuando se hayan publicado los primeros caps. del Vol.II.)*

---

