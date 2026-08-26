# Apéndice 0 — Manual de estilo unificado

> *Este apéndice explica cómo está escrito este libro: por qué sus tres volúmenes suenan distinto, qué plantilla sigue cada capítulo y qué garantías de verificación hay detrás de lo que lees.*

## 0.1. Por qué un manual de estilo común

Esta obra se publica en **tres volúmenes**, cada uno con su propia voz:

- El **Volumen I** («Grafos en Computación: de Cero a Experto») tiene una voz **narrativa y divulgativa**, basada en el estilo que Aditya Bhargava popularizó como *Grokking Algorithms*: hooks, anécdotas históricas, regla de tres, humor inesperado, ASCII art y baterías recurrentes como «Pin de batalla» o «Si solo lees 30 segundos».
- El **Volumen II** («Construye LiraDB») tiene una voz **ingenieril y metódica**: cada capítulo construye una pieza del motor LiraDB siguiendo una plantilla fija de **20 secciones en orden invariable**, que combina los diez pasos pedagógicos del proyecto con las baterías narrativas del Vol.I.
- El **Volumen III** («Grafos en la era de la IA: modelar, razonar y recuperar», esqueleto aprobado según ADR-005) abordará knowledge bases, GraphRAG y memoria de agentes. Definirá su propia voz cuando arranque su redacción, pero hereda desde ya el esqueleto común de este manual: baterías, cross-references, política de citación y proceso verificado.

Las voces son complementarias: el Vol.I te enseña *qué es* un grafo y *qué algoritmos existen*; el Vol.II te enseña *cómo construir* un sistema que los persiste y consulta; el Vol.III conectará todo con cómo la IA razona sobre grafos. Un manual común asegura que cambies de volumen sin perder el mapa.

## 0.2. Las dos plantillas, lado a lado

Cada columna conserva su propio orden interno; se emparejan por concepto:

| # | Plantilla Vol.I (Grokking 2.0) | Plantilla Vol.II (híbrida — orden fijo) |
|---|---|---|
| 1 | `# Capítulo N — <Título evocador>` | `# Capítulo N — <Título evocador>` |
| 2 | `## N.0 La anécdota de la esquina` | `## N.0 La anécdota de la esquina` |
| 3 | `## N.1 … N.K` — cuerpo técnico libre (4-12 secciones) | `## N.1 Objetivo` … `## N.10 Cómo lo hace una BBDD real + retos` (los diez pasos, ver abajo) |
| 4 | `## Lo que te llevas` | `## N.11 Lo que te llevas` |
| 5 | `## Ojo, cuidado con…` | `## N.12 Ojo, cuidado con…` |
| 6 | `## Pin de batalla` | `## N.13 Pin de batalla` |
| 7 | `## Si solo lees 30 segundos` | `## N.14 Si solo lees 30 segundos` |
| 8 | `## Una historia pequeña` | `## N.15 Una historia pequeña` |
| 9 | `## Ejercicios resueltos` | `## Ejercicios resueltos` |
| 10 | `## Ejercicios propuestos` | `## Ejercicios propuestos` (con retos esencial/intermedio/experto) |
| 11 | `## Para profundizar` | `## Para profundizar` |
| 12 | Solo Parte VI: `Diálogo de ascensor / Mini-diálogo` | `## Mini-diálogo: en guardia nocturna` |

En el Vol.II, el orden de esas 20 secciones es **fijo**: primero la anécdota (N.0), luego el ciclo completo problema → primera solución → límites → solución evolucionada (N.1-N.10), después las baterías numeradas N.11-N.15 y, al cierre, ejercicios, referencias y mini-diálogo. No se omiten ni se reordenan secciones.

Los diez pasos del cuerpo técnico del Vol.II son:

1. `N.1 Objetivo` — qué construye el capítulo y deudas que cobra.
2. `N.2 Problema` — el escenario y las ideas equivocadas que desactiva antes.
3. `N.3 Modelo mental` — la figura o escalera que ordena el tema.
4. `N.4 Primera solución` — la versión ingenua, correcta pero incompleta.
5. `N.5 Sus límites` — dónde y por qué esa versión se rompe, con cifras.
6. `N.6 Solución evolucionada` — cada gesto con su alternativa descartada.
7. `N.7 Código completo ejecutable` — el módulo del workspace, no un esbozo.
8. `N.8 Prueba de fuego` — tests y benches con salidas reales.
9. `N.9 Qué hemos sacrificado` — lo que NO hace, declarado sin vergüenza.
10. `N.10 Cómo lo hace una BBDD real + retos` — PostgreSQL/Neo4j/DuckDB/Kùzu… y retos esencial/intermedio/experto.

## 0.3. Ejemplos canónicos

Para ver cada pieza en acción, estos capítulos reales son la referencia:

- **Anécdota de apertura (N.0)**: el brazo mecánico del disco con Jim Gray en el cap. 11; MonetDB destrozado por un bucle escrito a mano en el cap. 38; el leapfrog triejoin en producción años antes de probarse su optimalidad en el cap. 39.
- **Modelo mental (N.3)**: la escalera de ocho peldaños y el contraste plan binario vs. worst-case optimal del cap. 39.
- **Primera solución, sus límites y sacrificios (N.4-N.5, N.9)**: el plan binario del cap. 39 materializa 392 tuplas intermedias en K₈ para devolver solo 56 triángulos — y el capítulo confiesa qué renuncia al no integrar esos joins en el executor.
- **Prueba de fuego (N.8)** con salidas reales: los benches medidos del cap. 34 (§34.8) y la tabla del cap. 39 donde el join WCOJ gana ~29× al plan binario, con microsegundos reales.
- **Retos graduados (N.10)**: esencial/intermedio/experto en cualquier capítulo, p. ej. cap. 25 (Louvain).
- **Mini-diálogo de guardia nocturna**: presente en todos los capítulos del Vol.II, p. ej. cap. 27 (ACID).
- **Una historia pequeña**: Grace Hopper repartiendo cables de nanosegundo (cap. 38) y el Raft diseñado para ser comprensible (cap. 40).

## 0.4. Reglas de transición entre volúmenes

- Cualquier referencia a un concepto de otro volumen usa la notación `(Vol. I, cap. N)` o `(Vol. III, cap. N)`; dentro del mismo volumen basta `(cap. N)`.
- El **cap. 32 del Vol.I** cierra ese volumen invitando a continuar con el Vol.II; el **cap. 40 del Vol.II** cierra el segundo invitando al tercero.
- El **cap. 1 del Vol.II** («Qué es realmente un grafo») abre citando explícitamente los caps. 1-2 del Vol.I como prerrequisito.
- Los caps. 21-32 del Vol.I funcionan como «semilleros» del Vol.II: cada uno termina con una nota que apunta al capítulo del Vol.II que implementa lo introducido.

## 0.5. Un solo workspace Rust

Todo el código del Vol.II vive en **un workspace único**, `liradb-workspace/` (ADR-007):

- Un `Cargo.toml` raíz y **un módulo por capítulo** (`crates/vol2-liradb/src/capNN_*.rs`): el cap. 39 corresponde a `cap39_joins.rs`, y así sucesivamente.
- La toolchain está **pinea** en `rust-toolchain.toml` (canal `1.96.0`, con `rustfmt` y `clippy`). Cambiarla exige pasar la suite completa y documentarlo en el changelog.
- Las versiones de crates van **pineadas** (sin `^` ni `~`) y el `Cargo.lock` está **commiteado**: lo que compilamos es lo que tú compilas.
- Si un crate queda obsoleto durante la escritura, se documenta y se decide en una incidencia; **no se reescriben capítulos ya publicados** salvo errata técnica.
- Regla de construcción «primero a mano, luego con crates»: cada componente de LiraDB se implementa dos veces — primero con la biblioteca estándar, después con el crate maduro — seguido de un benchmark comparativo y una decisión documentada (ADR) de cuál se queda.

Cuando veas `(LiraDB §N.M)` en la prosa, es la referencia al módulo del workspace asociado a esa sección.

## 0.6. El proceso de producción verificado

Cada capítulo pasa por la misma cadena, en este orden:

1. **Contrato de capítulo**: objetivos de aprendizaje, modelo mental, trampas y ejercicios se acuerdan antes de escribir una línea.
2. **Código en el workspace**: los ejemplos se implementan como módulos con tests, no como fragmentos sueltos.
3. **Verificación**: `./liradb-workspace/scripts/verify.sh` debe terminar en **ALL_GREEN** (formato, clippy, tests y golden files). Sin ALL_GREEN no hay prosa.
4. **Prosa**: el texto explica el código ya verde mediante `include::`; nunca duplica el código.
5. **Ensamblado**: los documentos finales se generan con `scripts/build_book.sh` a partir del `SUMARIO.txt` de cada volumen.
6. **Commits**: solo se integran cambios con la suite en ALL_GREEN.

Regla editorial derivada: los ensamblados generados en la raíz del repositorio **nunca se editan a mano** — si algo cambia, cambia la fuente y se regenera.

## 0.7. Cómo se cita

- Toda afirmación técnica lleva **fuente primaria**: paper con venue y año verificados (p. ej. Veldhuizen, ICDT 2014; Ongaro-Ousterhout, USENIX ATC 2014), especificación o documentación oficial del crate.
- El relato histórico de Kùzu sigue el ADR-001: se cita el paper CIDR 2023 (licencia CC-BY 4.0) y la cronología verificada Waterloo → adquisición por Apple (octubre de 2025) → repositorio archivado → forks comunitarios LadybugDB y bighorn. Nunca diremos que Kùzu fue «renombrada»: LiraDB es una reimplementación conceptual *clean-room* del concepto publicado, con la atribución correspondiente en el colofón.
- Los términos técnicos en inglés aparecen entre paréntesis la primera vez; luego se usa el término español.

## 0.8. Glosario estructural

Los términos que verás repetirse capítulo a capítulo:

| Término | Significado |
|---|---|
| **Capítulo** | Unidad principal (~200-700 líneas), numerado dentro de cada volumen. |
| **Parte** | Agrupación de 5-8 capítulos, numerada en romanos. |
| **Batería** | Sección recurrente fija que cierra cada capítulo (p. ej. «Pin de batalla»). |
| **Reto esencial/intermedio/experto** | Niveles de dificultad de los ejercicios propuestos del Vol.II. |
| **Contrato de capítulo** | Checklist previo a la redacción: objetivos, modelo mental, trampas y ejercicios comprometidos. |
| **Claim** | Afirmación técnica identificable (`claim_id`) con nivel de confianza y fuente asociada. |
| **Evidence card** | Recorte verificable de una fuente (paper, doc, spec) que respalda una claim. |
| **Code card** | Fragmento de código Rust con su `Cargo.toml` y tests asociados en el workspace. |
| **ADR** | Architecture Decision Record: la decisión y su justificación (Apéndice D del Vol.II). |

---
