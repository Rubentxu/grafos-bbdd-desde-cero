# CONVENTIONS — Convenciones de la obra unificada

> Convenciones vigentes para la obra de 3 volúmenes. Este archivo es la **fuente de verdad** para el estilo editorial y técnico; cualquier conflicto entre una skill y este archivo, gana este archivo.

## 0. Sistema de volúmenes

| ID | Título | Foco | Estado |
|---|---|---|---|
| Vol.I | "Grafos en Computación: de Cero a Experto" | Algoritmos, estructuras, aplicaciones (Rust) | Publicado (2ª ed.) |
| Vol.II | "Construye LiraDB — De los algoritmos fundamentales a un motor persistente de consultas en Rust" | Motor de BBDD de grafos desde cero (proyecto integrador LiraDB) | En escritura |
| Vol.III | "Grafos en la era de la IA: modelar, razonar y recuperar" | Knowledge bases, GraphRAG y memoria de agentes (hilo conductor KB-Lira) | Esqueleto (outline aprobado 2026-08-14) |

- Numeración de capítulos: cada Volumen reinicia en 1 (autoportante).
- Cross-references: notación `(Vol. I, cap. 21)` o `(Vol. II, cap. 11)`.
- Lenguaje común: Español (terminología técnica estándar en inglés entre paréntesis la primera vez).
- Stack común: Rust **2024** edition. Toolchain pinneada por capítulo vía `rust-toolchain.toml`.

## 1. Plantilla pedagógica

### Vol.I — "Grokking 2.0" (plantilla narrativa)

Cada capítulo del Vol.I sigue el orden:

```
# Capítulo N — <Título evocador>
## N.0 La anécdota de la esquina
## N.1 ... N.K  (cuerpo técnico)
## Ejercicios resueltos
## Ejercicios propuestos
## Lo que te llevas
## Ojo, cuidado con…
## Para profundizar
## Pin de batalla
## Si solo lees 30 segundos
## Una historia pequeña
## [Solo en Parte VI] Diálogo de ascensor / Mini-diálogo
```

Referencia completa: Apéndice D del Vol.I (`Cómo está escrito este libro`).

### Vol.II — Plantilla híbrida (10 pasos LiraDB + baterías Vol.I)

Cada capítulo del Vol.II sigue el orden FIJO (definido en el Apéndice 0):

```
# Capítulo N — <Título evocador>
## N.0 La anécdota de la esquina                          (batería Vol.I)
## N.1 Objetivo                                           (paso 1 LiraDB)
## N.2 Problema                                           (paso 2)
## N.3 Modelo mental                                      (paso 3)
## N.4 Primera solución                                   (paso 4)
## N.5 Sus límites                                        (paso 5)
## N.6 Solución evolucionada                              (paso 6)
## N.7 Código completo ejecutable                         (paso 7)
## N.8 Prueba de fuego                                    (paso 8)
## N.9 Qué hemos sacrificado                              (paso 9)
## N.10 Cómo lo hace una BBDD real + retos (esencial/intermedio/experto)  (paso 10)
## N.11 Lo que te llevas                                  (batería Vol.I)
## N.12 Ojo, cuidado con…                                 (batería Vol.I)
## N.13 Pin de batalla                                    (batería Vol.I)
## N.14 Si solo lees 30 segundos                          (batería Vol.I)
## N.15 Una historia pequeña                              (batería Vol.I)
## Ejercicios resueltos                                   (batería Vol.I)
## Ejercicios propuestos                                  (batería Vol.I)
## Para profundizar                                       (batería Vol.I, refs DBMS)
## Mini-diálogo: en guardia nocturna                      (batería Vol.I Parte VI)
```

## 2. Pipeline pedagógico (skills + metodología teaching)

El diseño pedagógico de cada volumen sigue una cadena de skills con artefactos verificables:

| Fase | Skill (zcode) | Artefacto | Estado en Vol.III |
|---|---|---|---|
| 1. Currículo | `curriculum-designer` | `book-context/CURRICULUM-VOL3.yml` (grafo de conceptos, prerrequisitos, out-of-scope, orden topológica) | ✅ |
| 2. Índice | `book-outline-architect` | `book-context/OUTLINE-VOL3.yml` (caps → secciones → conceptos, check de dependencias) | ✅ |
| 3. Contrato por capítulo | `chapter-planner` | contrato con objetivos de aprendizaje + ejercicios listados | pendiente |
| 4. Redacción | `chapter-writer` + `code-example-generator` | capítulo según plantilla del Apéndice 0 | pendiente |
| 5. Ejercicios | `exercise-designer` | 3-5 ejercicios graduados (recordar/aplicar/analizar/crear), pistas ≤3 niveles, **soluciones verificables en el workspace** (tests en `verify.sh`) | pendiente |
| 6. Revisión pedagógica | `pedagogical-reviewer` | informe por capítulo | pendiente |

**Metodología teaching (opencode)** como capa complementaria — origen: `~/.config/opencode/teaching/`:

- **Contrato de dominio por capítulo**: objetivos divididos en *knowledge* (qué sabe), *skills* (qué hace) y *wisdom* (qué decide y cuándo NO hacerlo). Sustituye a "objetivo de aprendizaje" plano en los contratos de `chapter-planner`.
- **Modelo mental explícito**: cada capítulo abre su cuerpo con una taxonomía/figura que ordena el tema (como la taxonomía de propósito de los plugins en las lecciones de opencode).
- **Reflexión**: batería nueva opcional "La primera vez que no lo entendí" — qué costó entender y por qué (estilo learning-records).
- **Preguntas abiertas**: cada capítulo cierra con lo que NO resuelve, como gancho al siguiente (estilo "Questions Still Open").
- **Design before code** (learning-record 0004): cuando un tema tiene decisiones de política abiertas, primero el diseño como contrato (AST), luego la implementación (parser). Ya es patrón del proyecto: Vol.II caps 17→18.

**Regla de verificación**: toda solución de ejercicio y todo ejemplo del Vol.II/III compila y pasa `./scripts/verify.sh` del workspace (ALL_GREEN). Las soluciones de ejercicios del Vol.III viven como tests en el workspace, no en la prosa.

## 3. Tipografía y maquetación

- **Tú** — tuteo siempre. Esto es una conversación, no un paper.
- **Términos en negrita** la primera vez que se introducen (van al glosario).
- **Bloques de código** — Rust idiomático, Rust **2024** edition.
- **`Cargo.toml`** mostrado la primera vez que aparece cada crate.
- **Diagramas ASCII** cuando un grafo/estructura se entiende mejor dibujado.
- **Blockquotes `>`** para epígrafes y diálogos.
- **Sin emojis decorativos** salvo en diagramas donde aporten claridad.

## 4. Política de crates (Vol.II — LiraDB)

| Categoría | Crates | Política |
|---|---|---|
| Estructuras de grafo | `petgraph`, `slotmap` | OK |
| Persistencia | `memmap2`, `crc32fast`, `zerocopy`, `redb` (opcional) | OK |
| Buffers / caching | `lru` | OK |
| Serialización | `serde` (NO `bincode` — declarado unmaintained desde 2025-12) | OK |
| Errores | `thiserror` | OK |
| CLI | `clap` | OK |
| Observabilidad | `tracing` | OK |
| Testing | `proptest`, `criterion`, `cargo-fuzz` | OK |
| Parsing | `logos` (preferido), `pest` (comparativo) | OK |
| Concurrencia | `tokio` (mínima), `rayon` (futuro morsel-driven) | OK |

**Regla "primero a mano, luego con crates"**: cada componente se implementa 2 veces — manual con `std` y luego con el crate maduro — seguido de benchmark comparativo y ADR de keep/discard.

**Regla de versiones**: `Cargo.lock` pinneado por capítulo; `rust-toolchain.toml` fija la toolchain. Esto se formaliza en el ADR-002.

## 5. Política de citación

- Afirmaciones técnicas con `claim_id` y `confidence_score`.
- Fuentes: papers seminales, RFCs, docs oficiales de crates, libros de referencia (Sedgewick, Petrov, Needham/Hodler).
- Atribución a Ladybug/Kùzu: clean-room conceptual reimplementation; cuando se cite el Kùzu VLDB paper, indicar la licencia original (MIT/CC-BY 4.0) y mantener la atribución en el Colofón del Vol.II.

## 6. Reglas del workflow (resumen)

- **Prime directive**: el código del workspace de ejemplos es el centro; la prosa lo explica vía `include::`, nunca lo duplica.
- **Code-map bidireccional**: `code-integration-architect` lo mantiene.
- **Puerta de calidad §8**: 7 condiciones obligatorias antes de `DONE` (ver `BOOK-WORKFLOW.md`).
- **Límite de remediación**: 3 ciclos por capítulo por la misma causa; al 4º, escalar al autor.

## 7. Glosario estructural

| Término | Significado |
|---|---|
| **Capítulo** | Unidad principal del libro (~ 200-700 líneas). Numerado dentro de cada Volumen. |
| **Parte** | Agrupación de capítulos (5-8 caps). Numerada en romanos (I-VIII). |
| **Batería** | Sección recurrente fija al final de un capítulo (ej. "Pin de batalla"). |
| **Ejercicio resuelto** | Problema con solución completa en el capítulo. |
| **Ejercicio propuesto** | Problema sin solución, para el lector. Niveles: esencial, intermedio, experto. |
| **Reto** | Variante del ejercicio propuesto en Vol.II con esos 3 niveles. |
| **Claim** | Afirmación técnica con `claim_id`, `confidence_score` y referencia. |
| **Evidence card** | Recorte verificable de fuente, extraído por `source-researcher` en B2. |
| **Code card** | Snippet de código Rust con contrato y `Cargo.toml` asociado. |
| **Code-map** | Tabla bidireccional prosa↔código mantenida por `code-integration-architect`. |