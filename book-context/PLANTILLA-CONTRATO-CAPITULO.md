# PLANTILLA CONTRATO DE CAPÍTULO — ¿Qué necesita este capítulo para enseñar a un novato?

> Obligatorio antes de `chapter-writer` (ver CONVENTIONS.md §2). Se rellena por capítulo y se
> guarda en `book-context/contratos/volN/cap-NN.md`. Combina: metodología **teaching** (opencode),
> **grill-with-docs** (interrogatorio contra la documentación/fuentes), **exercise-designer** y
> la plantilla híbrida del Apéndice 0.

El contrato es el **auto-interrogatorio**: cada campo es una pregunta que, si no se responde,
dejará un hueco que un novato notará. Un capítulo DONE es la respuesta a TODOS estos campos.

---

## 1. El novato (perfil y punto de partida)
- ¿Qué sabe YA sin ninguna duda al llegar aquí (prerrequisitos del CURRICULUM)?
- ¿Qué cree saber pero en realidad es vago/erróneo (misconcepción a corregir)?
- ¿Qué NO debe saber todavía (conceptos futuros que NO se anticipan — y dónde se corta)?

## 2. Conceptos (del grafo curricular)
- `present` (se introducen por primera vez):
- `practice` (se ejercitan, ya vistos):
- `consolidate` (se asumen y reutilizan):
- `out_of_scope` (se nombran como "luego lo verás", sin explicar):

## 3. Objetivos de dominio (taxonomía teaching)
- **Knowledge** (qué SABE al terminar — 3-5 afirmaciones comprobables):
- **Skills** (qué HACE — 2-3 tareas que ejecuta con el código):
- **Wisdom** (qué DECIDE — 1-2 "cuándo NO hacerlo / qué trade-off pesa más"):

## 4. Modelo mental
- La figura/taxonomía/analogía que ordena todo el tema (una sola, explícita).
- Diagrama(s) ASCII necesarios.
- El "momento ¡ajá!" que el lector debe tener.

## 5. Los porqués (grill — la pregunta más importante de cada decisión)
Para CADA decisión técnica del capítulo, responder «¿por qué así y no de otra forma?»:
- ¿Qué problema concreto resuelve?
- ¿Qué alternativa se descartó y por qué (con coste medible si lo hay)?
- ¿Qué pasaría si no lo hiciéramos (modo de fallo)?
- Evidencia: claim_id + fuente (docs, paper, spec) — `reference-validator` lo verifica.

## 6. Primera solución vs solución evolucionada
- ¿Cuál es la versión ingenua que escribiría un novato (y sus límites)?
- ¿Qué la rompe exactamente (input/escenario que falla)?
- ¿Cómo evoluciona el código del capítulo desde esa versión (diferencia visible)?

## 7. Prueba de fuego
- El escenario/consulta/tests que demuestran que lo aprendido FUNCIONA (del workspace).
- ¿Qué fallaría si el lector se saltara este capítulo (síntoma detectable)?

## 8. Trampas y errores comunes
- Los 3 errores que comete TODO el mundo aquí (y cómo detectarlos).
- Precisión de lenguaje: términos que se confunden (glosario).

## 9. Ejercicios (exercise-designer)
- `recordar/aplicar` (1): 
- `analizar` (1): 
- `crear` (1): 
- Cada uno con: pistas (≤3, graduadas), solución verificable en el workspace (test), criterios de evaluación.

## 10. Preguntas abiertas (gancho al siguiente capítulo)
- 2-3 preguntas que este capítulo NO responde y el siguiente SÍ (explicitadas al final).
- Términos nuevos de glosario (los registra `book-memory-keeper`).

## 11. Diseño de retención (skill `teach` — ~/.agents/skills/teach/)
La fluidez (leer y asentir) produce **dominio ilusorio**; la meta es **storage strength** (retención a largo plazo). Para lograrla, el capítulo incorpora dificultad deseable:

- **Retrieval practice**: al menos UN ejercicio obliga a *recordar* contenido del capítulo (o de capítulos previos) desde la memoria — sin pistas en el enunciado que lo regalen. Recordar > reconocer.
- **Spacing**: cada capítulo referencia y EJERCITA al menos un concepto de capítulos anteriores (¿qué concepto viejo se re-usa aquí y qué ejercicio lo toca?).
- **Interleaving**: los ejercicios mezclan temas vecinos (p.ej. un ejercicio de slotted pages que obliga a razonar sobre endianness del cap. 9), en vez de N ejercicios clónicos del mismo tema.
- **Regla de dificultad asimétrica**: para el CONOCIMIENTO (la explicación) la dificultad es el enemigo — una sola idea nueva por sección, dentro de la memoria de trabajo. Para la DESTREZA (los ejercicios) la dificultad es la herramienta — esfuerzo de recuperación.
- **Bucle de feedback inmediato**: el lector puede verificar cada ejercicio al instante (`cargo test` del workspace) — feedback automático y Tight, no "confía en mí".
- **Citas**: toda afirmación técnica con fuente de alta confianza (paper, spec, libro de referencia) — nunca solo el conocimiento paramétrico del autor.

---

## Checklist de profundidad (antes de marcar DONE)
- [ ] Cada decisión técnica tiene su «porqué» con fuente.
- [ ] Existe un escenario de fallo visible, no solo el happy path.
- [ ] El código del capítulo es ejecutable (test en workspace) y la prosa lo referencia sin duplicarlo.
- [ ] Hay al menos una misconcepción corregida explícitamente.
- [ ] Los ejercicios tienen solución verificada.
- [ ] Hay ≥1 ejercicio de retrieval practice (recordar, no reconocer) y ≥1 toque a concepto de capítulo anterior (spacing).
- [ ] El capítulo responde las preguntas críticas de `CORPUS.yml` para su id.
