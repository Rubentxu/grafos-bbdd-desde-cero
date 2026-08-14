# ADR-005 — Volumen III «Grafos en la era de la IA» + refuerzos quirúrgicos en Vol.II

**Fecha**: 2026-08-14
**Estado**: Aprobada
**Contexto**: Estudio estratégico (`book-context/PROPUESTA-EVOLUCION.md`) tras petición del autor de explorar más capítulos útiles de internals de BBDD, el alcance de los grafos para knowledge bases en IA, y refuerzos en modelado de datos (entidades, propiedades, buenas prácticas, workflows de extracción).

## Decisión

1. **Crear el Vol.III** «Grafos en la era de la IA: modelar, razonar y recuperar» — 13 capítulos en 3 partes (Modelado 41-45, Semántica 46-48, Grafos×IA 49-53), con hilo conductor **KB-Lira** (base de conocimiento de un equipo de investigación, generador determinista en el workspace). Artefactos: `vol3-grafos-era-ia.md` (esqueleto), `book-context/CURRICULUM-VOL3.yml`, `book-context/OUTLINE-VOL3.yml`.
2. **Refuerzos quirúrgicos en el guion del Vol.II sin renumerar**: cap. 15 (+apéndice B+ tree multinivel), cap. 16 (+LSM-trees), cap. 21 (+estadísticas y cardinalidad), cap. 30 (ampliado a 2PL, niveles de aislamiento, OCC, deadlocks con grafo de espera), cap. 38 (+compresión), cap. 40 (+nota híbridos vector+grafo), Apéndice E (paisaje 2026 post-Kùzu).
3. **Adoptar el pipeline pedagógico completo** (skills zcode + metodología teaching de opencode) documentado en `CONVENTIONS.md` §2: curriculum-designer → book-outline-architect → chapter-planner → chapter-writer + exercise-designer → pedagogical-reviewer; capa teaching con contrato de dominio knowledge/skills/wisdom, modelo mental, reflexión y preguntas abiertas.
4. **Medidas de utilidad transversales**: dataset hilo conductor (KB-Lira), CLI mínima anticipada tras el cap. 20 del Vol.II, redacción de prosa en paralelo al código (empezando por Parte III del Vol.II, ya cerrada en código), soluciones de ejercicios como tests del workspace.

## Alternativas descartadas

- **Propuesta B — expandir Vol.II a ~48 caps**: rechazada por alargar un volumen ya grande, retrasar su cierre y mezclar dos audiencias (constructor del motor vs modelador/usuario de IA). Registro completo en PROPUESTA-EVOLUCION.md §5.

## Consecuencias

- La obra pasa de 2 a 3 volúmenes (~85 caps totales); LEDGER, CORPUS y CONVENTIONS reflejan el cambio.
- El workspace ganará módulos del Vol.III (tipo vector en `Value`, índice HNSW, extracción LLM stubeable, evaluación GraphRAG) cuando sus capítulos pasen por chapter-planner.
- La numeración del Vol.II queda congelada (ADR-003 se mantiene); los refuerzos se aplican dentro de capítulos existentes.
- ADR-001 (atribución Ladybug/Kùzu) pendiente de reescritura como relato histórico: Kùzu→adquisición Apple (oct. 2025)→archivado→forks; antes del cap. 37 del Vol.II.
