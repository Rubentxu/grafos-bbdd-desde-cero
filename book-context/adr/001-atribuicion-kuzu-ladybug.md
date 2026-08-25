# ADR-001: Atribución a Kùzu/LadybugDB como relato histórico verificado

**Fecha**: 2026-08-25
**Estado**: RESUELTA (aprobada por el autor — «adelante con la reescritura propuesta»)
**Contexto**: pendiente desde la creación del guion del Vol.II; era prerrequisito del cap. 38 (columnar/vectorizado).

---

## Decisión

La atribución a Kùzu en el Vol.II se mantiene como **clean-room conceptual**, pero se reescribe como **relato histórico verificado** en prólogo y colofón, corrigiendo dos errores factuales detectados:

1. ~~«renombrado a Ladybug tras la adquisición por Apple»~~ → **LadybugDB es un fork comunitario** (no un renombrado por Apple).
2. ~~«Kùzu VLDB 2023 paper»~~ → el paper del sistema es **CIDR 2023** (Jin, Feng, Chen, Liu, Salihoğlu).

Las citas académicas históricas de los caps. 13-21 (papers previos a la adquisición) **siguen siendo válidas sin cambios**.

## Línea temporal verificada (fuentes primarias e independientes)

| Fecha | Hecho | Fuente |
|---|---|---|
| 2020-2022 | Grupo de Semih Salihoğlu (U. Waterloo) desarrolla Kùzu; predecesor GraphflowDB | Paper CIDR 2023 |
| Ene 2023 | Paper del sistema: Jin*, Feng*, Chen, Liu, Salihoğlu, «KÙZU Graph Database Management System», **CIDR 2023**, Ámsterdam, licencia **CC-BY 4.0** («Kù-zu»: sumerio «brillante» + «saber» = sabiduría) | cidrdb.org/cidr2023/papers/p48-jin.pdf |
| 2023 | Kùzu Inc. fundada en Ontario para comercializarlo (cofundadores Chen, Feng, Jin, Liu; CEO Salihoğlu; ~10 empleados en 2025) | cs.uwaterloo.ca/news/…kuzu-acquired-apple; BetaKit |
| **9 oct 2025** | Acuerdo: Apple compra todas las acciones y contrata a parte del equipo vía filial no identificada | Divulgación DMA de la UE (reportada por AppleInsider, 11 feb 2026) |
| **10 oct 2025** | Repo github.com/kuzudb/kuzu **ARCHIVADO**; última release **0.11.3** el mismo día (agrupa las extensiones más usadas); docs movidas a kuzudb.github.io; web caída; nota «Kuzu is working on something new!» | GitHub repo (estado ARCHIVED); The Register 14-oct-2025 |
| Oct-nov 2025 | Forks comunitarios: **LadybugDB** (github.com/LadybugDB/ladybug, comunidad liderada por Arun Sharma; v0.12.0 equivalente funcional a kuzu 0.11.3) y **bighorn** (Kineviz) | The Register 14-oct-2025; LinkedIn (Sharma, 25-oct-2025); blog.ladybugdb.com (v0.12.0, nov 2025) |
| 11-12 feb 2026 | La adquisición sale a la luz vía registro DMA de la UE; Apple no revela precio ni planes | The Verge, BetaKit, MacRumors, 9to5Mac |

## Política de atribución resultante

1. **Colofón**: relato histórico completo (Waterloo→CIDR 2023→MIT→adquisición→archivo→forks) con agradecimiento al equipo original.
2. **Prólogo**: sección «Sobre Kùzu y sus forks» — referencia conceptual clean-room; el error «renombrado» corregido.
3. **Capítulos 13-21**: citas de papers y diseño histórico intactas. Cuando se nombre el presente del proyecto: «archivado tras la adquisición por Apple; continúa en forks comunitarios como LadybugDB».
4. **Apéndice E** (paisaje 2026): cubrirá LadybugDB/bighorn como sucesores activos (pendiente de redactar junto a la Parte VIII).
5. **Cap. 38+**: cualquier mención a factorización/WCOJ/vectorización cita el paper CIDR 2023 y los papers VLDB/SIGMOD del grupo, indicando licencia CC-BY/MIT según corresponda.
6. Nuestro libro: CC BY-NC-SA 4.0; cero código copiado de Kùzu (verificable: nuestro código es dependencia-cero y de estilo propio).

## Consecuencias

- El cap. 38 queda desbloqueado.
- El Apéndice C (bibliografía) citará el paper como: Guodong Jin, Xiyang Feng, Ziyi Chen, Chang Liu, Semih Salihoğlu. «KÙZU Graph Database Management System». CIDR 2023.
- La corrección del prólogo elimina una afirmación falsa antes de que llegue a lectores.
