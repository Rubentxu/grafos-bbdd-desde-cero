# CONTRATO DE CAPÍTULO — Vol.II Cap. 37: Qué necesitaría una base de datos de producción

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **ABRE la Parte VIII** (crecimiento
> POST-mapa) recogiendo el gancho explícito del cap. 36. Código ancla: `lib.rs` declara 28
> módulos (`cap07_modelo` … `cap35_observabilidad`) con API plana; EXACTAMENTE cinco puertos
> (`GraphStore` 8, `Pager` 12, `PhysicalOperator` 20, `WeightSource` 22, `Heuristic` 23);
> frontal `liradb-cli` (`demo/query/explain/repl/script/import/export` + `--graph/--plan/
> --stats/--profile`). Estado verificado 2026-08-25: **848 tests** ALL_GREEN;
> `vol2-liradb` dependency-free en runtime. Deuda NOMBRADA que este capítulo USA COMO
> EVIDENCIA y no repara: `Catalog::collect` cuadrático (~224 s vs 281 ms, MIGRATION §39);
> HashIndex capacity ≥ 3+num_buckets, Float-entero perdido en roundtrip JSONL, reloj MVCC
> arrancando en ts=1 y recuperación solo renace lo confirmado (todo §41); write skew en el
> snapshot MVCC (cap. 30); cola corrupta mitigada-no-eliminada (cap. 33); sin `GraphStore`
> respaldado por disco end-to-end (frontera declarada 33-36); `guardar_wal` SIN fsync
> garantizado (doc-comment `cap29_recuperacion.rs:611`; `FilePager::sync` SÍ llama
> `sync_all`, cap. 12). Precedente del patrón «informe ejecutable honesto»: `informe_acid()`
> (27) y sus re-valoraciones `_post_wal/_post_recovery/_post_mvcc` (28-30) — documentación
> AUDITABLE por tests («informe_acid() no puede mentir»). Pregunta crítica del CORPUS
> (`vol-II-cap-37`): **«Lista de cosas que LiraDB Lite NO hace (con referencias).»**
> Política ADR-001 VIGENTE: toda mención a Kùzu usa el relato histórico correcto (archivada
> tras la adquisición por Apple, oct-2025; continúa en los forks LadybugDB/bighorn; paper
> CIDR 2023 — NUNCA «renombrado a Ladybug»). Gancho saliente: cap. 38 columnar/vectorizado
> — primera de las TRES apuestas técnicas, con la semilla CSR×16 medida en el cap. 34.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: TODO el motor y el edificio entero — modelo PropertyGraph
  (7), puerto `GraphStore` (8), encoding/framing con MAGIC + FORMAT_VERSION (9-10), slotted
  pages (11), `Pager`/`FilePager` con `sync_all` (12), `BufferPool` (13), CSR (14),
  índices (15), mantenimiento (16), LiraQL end-to-end (17-21), algoritmos sobre el puerto
  con `Presupuesto` (22-26), ACID/WAL/recuperación ARIES/MVCC (27-30), CLI (31),
  import/export (32), torre de pruebas (33), benchmarks (34), contadores/trazas (35) y el
  hexágono final (36).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «producción = más rendimiento y más features» — no: es OTRO EJE, garantías operativas
  frente a condiciones adversas y hostiles; un motor rápido sin backups ni límites no es
  de producción.
  (2) «una BBDD embebida no necesita seguridad» — el modelo de amenaza CAMBIA (no hay
  listener de red), no desaparece (permisos del fichero, procesos hostiles del mismo usuario).
  (3) «las migraciones son cosa de esquemas; schemaless se libra» — no: el FORMATO también
  evoluciona; `FORMAT_VERSION` existe desde el cap. 9 para eso.
  (4) «el catálogo cuadrático fue un fallo puntual» — no: es el CASO VIVO de por qué la
  protección ante consultas costosas es una dimensión — nada lo detectó; lo cazó un humano
  con un benchmark (§39).
- **No debe saber todavía**: columnar/vectorizado (38), WCOJ (39), distribución/Raft (40).
  Corte: este capítulo ENUMERA y CLASIFICA; los tres siguientes ELIGEN tres frentes técnicos.
- **Pregunta crítica que el capítulo tiene que responder**: «Lista de cosas que LiraDB Lite
  NO hace (con referencias).» Respuesta: `informe_produccion()` — las ONCE dimensiones del
  brief clasificadas una a una contra la realidad verificable del workspace, cada entrada
  con evidencia citable, referencia industrial y punto del hexágono donde se enchufaría.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| `crates/vol2-liradb/src/cap37_produccion.rs` (ÚNICO archivo nuevo, ~300-400 líneas, std puro): `EstadoProduccion { Existe, Parcial, Ausente }`, `BloqueProduccion { Datos, Proceso, Personas }`, `DimensionProduccion { nombre, bloque, estado, como_esta_hoy, donde_se_enchufaria, quien_lo_resuelve }`, `InformeProduccion` (entradas/por_nombre/Display) y `informe_produccion()` con LAS ONCE entradas del brief en orden del brief, cada una con bloque asignado | `cargo test -p vol2-liradb --lib`: los tests nombrados en la fila siguiente; Display imprime 11 entradas agrupadas en 3 bloques |
| Tests de honestidad dentro del módulo (espejo exacto del cap. 27): `produccion_cubre_las_once_dimensiones_del_brief`, `produccion_es_honesto_sobre_el_estado_actual` (PINNA los 11 estados: implementar auth/cifrado/timeout ROMPE el test A PROPÓSITO hasta actualizar el informe — la lección del 27 hecha cerrojo), `produccion_parciales_citan_simbolos_reales` (nivel-compilación: `FORMAT_VERSION == 1`, `Presupuesto` construible, Contadores con `# TYPE`, `guardar_wal`/`reabrir` usables), `produccion_ninguna_dimensión_existe_sin_cadena_completa`, `produccion_display_tres_bloques_once_lineas` | `cargo test -p vol2-liradb --lib`; mentir en el informe = test en rojo (nunca «confía en mí») |
| `lib.rs`: añadir `pub mod cap37_produccion; pub use cap37_produccion::*;` — convención `capNN_*` de la Parte VII; ÚNICO cambio en lib.rs | `cargo build` + `./scripts/verify.sh` |
| TABLA prosa de 11 filas × (dimensión · estado · evidencia workspace citable · cómo lo resuelve una BBDD real CON FUENTE · dónde se enchufaría) — el corazón del capítulo | Revisión cruzada fila a fila contra `lib.rs`/`code-map.yml` y docs oficiales (postgresql.org, sqlite.org, sqlcipher.net, flywaydb.org, prometheus.io, opentelemetry.io) |
| Diagrama checklist ASCII (§4) con leyenda `[X]/[~]/[ ]` y recuento 0·6·5 | Todo nombre del diagrama grepeable en el workspace; estados IDÉNTICOS a `informe_produccion()` |
| ALL_GREEN intacto | `./scripts/verify.sh` → 848 + N tests nuevos; fmt/clippy limpios; goldens demo/explain byte-exactos sin regenerar |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «Lista de cosas que LiraDB Lite NO hace (con referencias).» — convertida en
**respuesta en ocho pasos**:

1. **Por qué una lista AHORA**: el cap. 36 dibujó lo que SOMOS (mapa); abrir la Parte VIII
   exige preguntar qué faltaría para vivir FUERA del laboratorio — honestidad antes que
   features: sin lista honesta, crecer sería improvisar.
2. **La tesis de Gawande**: en dominios maduros el fallo rara vez es ignorancia; es
   ineptitud (saber y no aplicar con disciplina). La cura histórica es la LISTA DE
   VERIFICACIÓN verificada ítem a ítem (aviación, cirugía — estudio WHO/NEJM 2009).
3. **Qué es una dimensión de producción**: capacidad que el sistema necesita para operar
   bajo condiciones adversas u hostiles — NO una feature del camino feliz. Criterio:
   **Parcial** exige símbolo citable + test verde con frontera DOCUMENTADA; **Ausente**
   exige búsqueda fallida documentable; **Existe** exige cadena completa demostrada fin a fin.
4. **Las once del brief, mapeadas UNA A UNA contra el código real** (compatibilidad de
   formatos, seguridad, autenticación, autorización, cifrado, backups, migraciones, control
   de recursos, protección ante consultas costosas, telemetría, herramientas operativas).
5. **El informe EJECUTABLE**: `informe_produccion()` hereda la tesis del cap. 27
   (`informe_acid()`): documentación que no puede mentir porque los tests la pinan —
   la lista vive compilada, no en un wiki que se pudre.
6. **CON REFERENCIAS** (exigencia literal del CORPUS): cada entrada lleva
   `quien_lo_resuelve` — PostgreSQL (roles/GRANT, pg_basebackup/PITR, statement_timeout,
   pgcrypto), SQLite (file-format stability promise, .backup, modelo sin auth), SQLCipher,
   Flyway/Liquibase/Diesel migrations, Prometheus/OpenTelemetry — verificables en docs
   oficiales.
7. **Dónde se enchufaría**: cada dimensión señala SU punto del hexágono del cap. 36
   (formato→encoding 9; cifrado→bajo Pager 12; timeout→Executor 20; telemetría→decoradores
   35; ops→CLI 31…): el mapa deja de ser retrato y se vuelve BASE DE CRECIMIENTO.
8. **La frontera**: el capítulo NO implementa producción — nombra, clasifica y enchufa-en-
   el-mapa. Los caps. 38-40 construirán TRES frentes técnicos (columnar, WCOJ,
   distribución); las dimensiones restantes quedan como agenda honesta.

---

## 4. Modelo mental: la LISTA DE COMPROBACIÓN previa al vuelo

Modelo mental ÚNICO: **el checklist pre-vuelo** (cultura del checklist; Atul Gawande, «The
Checklist Manifesto», 2009). Ir a producción no es acumular features: es PASAR una lista
verificable ítem a ítem. Las once dimensiones se recuerdan agrupadas en tres bloques:
DATOS (sobreviven al tiempo y al accidente) → PROCESO (opera con cabeza bajo adversidad) →
PERSONAS (responden por el sistema). El informe ejecutable ES la lista — y por estar pinada
por tests, es la única clase de lista que no se queda obsoleta ni infla.

```
     LISTA DE COMPROBACIÓN PREVIA A «PRODUCCIÓN» — LiraDB Lite
     ══════════════════════════════════════════════════════════════════
     BLOQUE 1 · QUE LOS DATOS SOBREVIVAN (tiempo, accidente, miradas)
       [~] 1. Compatibilidad de formatos  magic+versión (9) · quien-abre-compara (33)
       [ ] 2. Migraciones                 FORMAT_VERSION sí · evolución versionada no
       [ ] 3. Cifrado                     nada · SQLCipher/pgcrypto/TDE existen fuera
       [~] 4. Copias de seguridad         WAL→fichero+replay (28-29) · sin fsync/checkpoint
     BLOQUE 2 · QUE EL PROCESO OPERE CON CABEZA (adversidad, exceso, opacidad)
       [~] 5. Control de recursos         Presupuesto (26) para algoritmos · nada global
       [ ] 6. Protección ante consultas   sin timeout/cancel · caso vivo: catálogo ~224 s
       [~] 7. Telemetría                  Contadores+spans (35) en proceso · sin export
       [~] 8. Herramientas operativas     CLI (31) germinal · sin init/status/backup
     BLOQUE 3 · QUE LAS PERSONAS RESPONDAN POR ÉL (amenaza, mínimo privilegio)
       [ ] 9.  Seguridad                  sin modelo de amenaza ni hardening documentado
       [ ] 10. Autenticación              no hay usuarios · MongoDB 2017: decenas de miles
                                          de instancias sin auth, borradas
       [ ] 11. Autorización               sin roles ni GRANT por operación
     LEYENDA: [X]=Existe   [~]=Parcial   [ ]=Ausente     HOY: 0 X · 6 ~ · 5 vacíos
```

Momento ¡ajá! perseguido: «production-ready no es tener más cosas: es PODER DEMOSTRAR,
ítem a ítem contra la realidad, qué aguantará el golpe — y el enemigo no es la ignorancia
(siempre quedan docs que leer) sino la ineptitud de no verificar lo sabido». ¡Ajá! honesto
adicional: ni una sola dimensión llega a Existe — y eso es una BUENA noticia operativa:
la lista convierte «no sé si puedo» en un inventario accionable.

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Capítulo de INVENTARIO HONESTO: clasifica (Existe/Parcial/Ausente), NO implementa producción | El valor es el mapa de huecos con evidencia; empezar a implementar seguridad/backups sin servidor ni despliegue sería teatro — y rompería la frontera declarada 33-36 | Implementar «lo fácil» (un timeout, un password): feature huérfana sin modelo de despliegue que la justifique | brief 1551-1566 (lista exacta de contenidos); contratos 33-36 (honestidad > cosmética) |
| 2 | SÍ hay UN artefacto nuevo: `cap37_produccion.rs` con `informe_produccion()` | Es el patrón-tesis del libro: informe_acid (27) demostró que la documentación AUDITABLE por tests no puede mentir; el CORPUS pide una lista y el método del libro exige feedback ejecutable | (a) Capítulo 100% conceptual: lista «confía en mí» que nadie puede auditar; (b) solo tests/integración estilo cap-36: el informe debe ser API pública reutilizable (futura CLI/admin), convención `capNN_*` ya establecida en 33-35 | cap27 `informe_acid()` + MIGRATION («AUDITABLE por tests»); PLANTILLA §11 (feedback inmediato vía cargo test) |
| 3 | Enum de TRES estados (no booleanos), hermano de `NivelGarantia` | La honestidad del libro es GRADUADA desde el cap. 27 (Ninguna/Parcial/Completa): «tiene/no tiene» oculta justo lo que enseña este capítulo (las fronteras) | Booleano tiene/no-tiene o score numérico: pierde el matiz y abre la puerta a puntuaciones-marketing | `NivelGarantia` cap27_transacciones.rs; criterio de graduación en §3.3 |
| 4 | Campo `quien_lo_resuelve` con referencias industriales DENTRO del artefacto | El CORPUS exige «con referencias»; meterlas en el tipo las hace compilables, grepeables y auditables — la referencia viaja con la entrada, no en un apartado aparte | Referencias solo en prosa: se desincronizan del informe en la próxima edición | CORPUS.yml `vol-II-cap-37`; docs oficiales listadas por dimensión en §6.10 |
| 5 | Las 11 del brief UNA A UNA en orden del brief; los tres bloques son AGRUPACIÓN de lectura, no reordenación | Fidelidad al brief (nada inventado, nada omitido); chunking Gawande para recordar 11 ítems sin traicionar el orden original | Reordenar/agrupar en el propio informe: dificultaría el diff contra el brief y el reto de retrieval | brief-liradb-original.md:1553-1565; Gawande 2009 (listas agrupadas por fase) |
| 6 | Ninguna dimensión llega a Existe (recuento 0·6·5) | Criterio §3.3 aplicado sin excepciones: hasta el formato (magic+versión+roundtrips) carece de política de evolución versionada; proclamar un Existe exigiría demostrar cadena completa — no la hay | Dar un «Existe» amable (p.ej. formato) para no sonar pesimistas: sería la falsedad que el libro prohíbe | Estados verificados contra código 2026-08-25 (ver evidencias en §4) |
| 7 | Deuda NOMBRADA usada COMO EVIDENCIA, jamás reparada | Continuidad 33-36; el catálogo cuadrático (~224 s, §39) es el mejor argumento POSIBLE para la dimensión «protección ante consultas costosas»: ocurrió de verdad, aquí, y nada lo detectó | «Ya que estamos, arreglamos el catálogo»: scope creep clásico — este cap mapea, la Parte VIII elige frentes | MIGRATION-PATTERN.md §39 (hallazgo 1), §40, §41 |
| 8 | Anécdota ÚNICA: ransomware MongoDB de enero 2017 (detalle y fuentes en §7) | Es la única candidata cuya causa raíz ES una dimensión del capítulo (autenticación ausente + seguridad): conecta el estado más dramático del informe con consecuencias reales verificables por prensa técnica independiente | (a) Knight Capital 2012: lección de DESPLIEGUE (servidor con código viejo), no de una dimensión; cifras discrepantes ($440M/$460M); (b) apagón S3 feb-2017: incidente de proveedor cloud, change-management ya implícito en checkpoints; (c) Apolo 1202: ya reservada/usada (monitorización ≠ lista) | §7 con fuentes; grep vol1/vol2 confirma no-uso previo |
| 9 | Modelo mental ÚNICO: checklist pre-vuelo (Gawande); los tres bloques SON las secciones de la lista, no un segundo modelo | Gawande aporta el ACTO (verificar ítem a ítem contra la realidad) y la cita fuerte (2009 + estudio WHO/NEJM); los anillos «datos→procesos→personas» aportaban estructura pero no acto — se conservan como ordenación interna | Anillos concéntricos como modelo principal: describen DÓNDE vive cada dimensión, no QUÉ se hace con ellas (y competirían con el hexágono del 36, otro círculo más) | Gawande, «The Checklist Manifesto» (Metropolitan Books, 2009); Haynes et al., NEJM 360:491-499 (2009) |
| 10 | Sin golden nuevo; puerta de calidad = tests del módulo + ALL_GREEN (848+N) | Lo determinista ya está cubierto (goldens 31-35); el informe es determinista por naturaleza (constantes literales) y sus tests LO pinan | Golden de la salida Display: duplicaría lo que ya garantiza `produccion_display_tres_bloques_once_lineas` | Política de determinismo del workspace; precedente cap-35 §5.11 |
| 11 | ADR-001 aplicado en todo el capítulo (paisaje N.10) | Cualquier mención a Kùzu: «archivada tras la adquisición por Apple (oct-2025); continúa en forks comunitarios LadybugDB/bighorn; paper CIDR 2023» — NUNCA «renombrado» | Repetir el error histórico corregido: falsedad factual ya desterrada del proyecto | book-context/adr/001-atribuicion-kuzu-ladybug.md (RESUELTA 2026-08-25) |
| 12 | Gancho saliente FIJADO a cap-38 (columnar/vectorizado) con semilla CSR×16 del cap. 34 | La Parte VIII es crecimiento CON CRITERIO: el primer frente sale de una MEDIDA propia (el puerto clona un Vec por llamada, ×16 vs CSR) — la lista dice QUÉ falta, la medida dice POR DÓNDE EMPEZAR | Gancho genérico «siguiente: almacenamiento avanzado»: desperdiciaría la continuidad narrativa del 36 | cap34 (CSR 1,33 ms vs puerto 21,11 ms iterando 500k aristas); cap-36 §6.12 |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: enero de 2017 — una ola automatizada
   encuentra en Internet miles de servidores MongoDB sin autenticación y borra su contenido
   dejando una nota de rescate; en días el recuento pasa de ~2.000 a decenas de miles. Nadie
   hackeó MongoDB: EXPLOTÓ la distancia entre lo que el producto permitía configurar y lo
   que cada operador VERIFICÓ. Pregunta: ¿podrías hacer la misma auditoría sobre LiraDB —
   sin mentir?
2. **N.1-N.2 Objetivo/Problema**: tienes el mapa (36) y 848 tests verdes; aún así, ante
   «¿esto puede ir a producción?» solo sabes responder con gestos. Síntoma detectable:
   empiezas a enumerar features al azar y no sabes cuándo PARAR ni cómo PROBAR lo dicho.
3. **N.3 Modelo mental**: el checklist pre-vuelo (§4): tres bloques, once ítems, tres
   marcas; la lista que corre en `cargo test`.
4. **N.4 Primera solución**: la lista ingenua — buscar «production database checklist» y
   copiar veinte bullets aspiracionales (replicación, sharding, HA…) sin contrastar NINGUNO
   contra nuestro código.
5. **N.5 Sus límites**: esa lista miente por EXCESO (exige lo que quizá nunca necesites:
   no hay servidor) y por OMISIÓN (no distingue lo que tenemos a medias de lo que no existe);
   nadie puede AUDITARLA; es el género literario exacto del que el ransomware se alimentó.
6. **N.6 Solución evolucionada**: las once dimensiones del brief, cada una CLASIFICADA
   contra el workspace con evidencia citable, referencia industrial y punto de enchufe en
   el hexágono — la tabla de 11 filas como corazón del capítulo.
7. **N.7 Código completo ejecutable**: `cap37_produccion.rs` por `include::`/referencia
   (nunca duplicado): tipos, `informe_produccion()` con sus once entradas y los cinco tests
   de honestidad — el ÚNICO código nuevo.
8. **N.8 Prueba de fuego**: `cargo test -p vol2-liradb --lib` en verde = la lista no miente.
   Demostración inversa: cambiar UNA entrada a `Existe` sin implementar nada → test rojo
   al instante (la mentira es detectable por construcción, herencia directa del cap. 27).
9. **N.9 Qué hemos sacrificado**: nada de producción quedó implementado (clasificamos);
   granularidad gruesa (3 estados, no %); la lista del breve omite dimensiones que un
   sistema grande también quiere — REPLICACIÓN/ALTA DISPONIBILIDAD brilla por su ausencia
   (se nombra, va a ejercicios, no al informe base); la amenaza de RED queda fuera de
   alcance mientras no exista servidor (frontera embedded declarada, no fingida).
10. **N.10 Cómo lo hace una BBDD real + retos**: PostgreSQL (roles/GRANT y pg_hba.conf;
    pgcrypto para cifrado de columna — el core NO trae TDE nativo; pg_basebackup + archivado
    WAL = PITR — ¡nuestro WAL del 28-29 ya es la mitad de un PITR!; statement_timeout y
    pg_cancel_backend), SQLite (file-format stability PROMISE documentada durante décadas;
    sin autenticación embebida — el acceso lo decide el sistema de ficheros; CLI
    `.backup`/`.dump` y Online Backup API; cifrado vía SEE/SQLCipher), Neo4j/LadybugDB
    (neo4j-admin dump/load; atribución Kùzu SEGÚN ADR-001). Retos: **esencial** (retrieval:
    escribir DE MEMORIA las once dimensiones con su marca [X]/[~]/[ ], sin pistas, y
    autocorregirse contra la salida impresa de `informe_produccion()`), **intermedio**
    (analizar: DOS procesos abren el MISMO fichero LiraDB — ¿qué dimensión rompe primero?
    Solución esperada: NINGUNA de las once lo cubre — falta locking inter-proceso, hueco
    nº12; FilePager/BufferPool asumen dueño único (12-13); dos escritores mezclan páginas,
    el CRC salta TARDE y antes hay corrupción silenciosa; conectar con los POSIX advisory
    locks de SQLite), **experto** (crear: añadir UNA dimensión nueva — locking inter-proceso
    O replicación — como entrada nº12 completa + actualizar test-pinzón y recuento del
    Display; criterio: suite verde y el informe SIGUE sin poder mentir).
11. **Baterías finales**: Lo que te llevas / Ojo, cuidado / Pin de batalla («una lista sin
    evidencia verificable es marketing; pinada por tests, es un contrato») / Si solo lees
    30 segundos (las once + 0·6·5) / Una historia pequeña (el checklist quirúrgico de la
    OMS: la mortalidad cayó cuando alguien VERIFICÓ ítem a ítem lo sabido — Haynes,
    NEJM 2009) / Mini-diálogo de guardia nocturna (3 a.m.: «¿esto puede ir a producción?» —
    el pager de guardia no improvisa: SACA LA LISTA). Retrieval practice obligatorio (reto
    esencial). Spacing DECLARADO: el capítulo ejercita caps. 9, 12-13, 20, 26, 28-31,
    33-36; interleaving: el reto intermedio mezcla E-S física (12-13), integridad (10/16)
    y operaciones (31). Glosario nuevo: producción (production readiness), autenticación
    (authentication), autorización (authorization), cifrado en reposo/en tránsito
    (at rest/in transit), copia de seguridad (backup), PITR (point-in-time recovery),
    migración (schema migration), statement_timeout, telemetría (telemetry), lista de
    comprobación (checklist), replicación (replication — ausente de la lista del brief).
12. **Gancho de cierre (preguntas abiertas)**: la lista dice QUÉ falta; la Parte VIII
    construye DÓNDE crecer: ¿dónde se enchufa lo COLUMNAR en nuestro hexágono — bajo el
    puerto `Pager`, un adaptador nuevo junto al CSR? (38, semilla ×16 del 34); ¿qué gana
    el optimizador con WCOJ y consultas cíclicas? (39); ¿qué partes del mapa sobrevivirían
    a repartir el motor entre máquinas — y qué dimensiones de ESTA lista explotan de golpe?
    (40). Cada capítulo empieza señalando un punto del hexágono del 36.

---

## 7. Estilo y tono (consistencia con caps. 27-36)

- **Voz**: didáctica, sin solemnidad; tuteo; término técnico en inglés entre paréntesis la
  primera vez (authentication, authorization, encryption at rest, PITR, checklist);
  NINGUNA salida falsificada: todo nombre de módulo/test/tipo citable del workspace; los
  números medidos (~224 s §39, ×16 cap. 34, 848 tests) citados CON SU ORIGEN.
- **Anécdota ÚNICA (verificada)**: ransomware MongoDB, enero de 2017. Cronología con
  prensa técnica independiente: ~2.000 instancias borradas el 4-ene (Bitdefender, citando
  el seguimiento de John Matherly/Shodan); >10.000 el 6-ene (Ars Technica); 22.900
  contabilizadas (Dark Reading); >27.000 en cuestión de jornadas (The Hacker News,
  9-ene-2017). PRECISIÓN OBLIGATORIA: NO fijar «28.000 el primer día» como dato exacto —
  las cifras publicadas oscilan 22.900–>27.000 según fuente y día; el capítulo dirá
  «decenas de miles en la primera semana» con los rangos. Contexto verificable: mongod
  escuchaba en todas las interfaces con autenticación DESACTIVADA por defecto; tras los
  incidentes, MongoDB cambió el bind por defecto a localhost (serie 3.6, nov-2017).
  Descartadas con razón explícita: Knight Capital 2012 (despliegue parcial; cifras
  discrepantes $440M/$460M), apagón S3 feb-2017 (proveedor cloud), Apolo 1202 (reservada).
- **Referencias por dimensión (docs oficiales, verificables)**: postgresql.org/docs
  (CREATE ROLE/GRANT, pgcrypto — el core NO trae TDE nativo—, continuous archiving/PITR,
  statement_timeout, pg_cancel_backend); sqlite.org (fileformat2.html, lockingv3.html,
  CLI .backup); sqlcipher.net; flywaydb.org/liquibase.org/diesel.rs (migraciones);
  prometheus.io (exposition format — YA IMITADO en el cap. 35) y opentelemetry.io;
  Gawande 2009 + Haynes et al. NEJM 2009 (checklists).
- **Diagramas**: el checklist ASCII de §4 como figura central; la tabla 11 filas como
  inventario paralelo al módulo-rúbrica del 36; bloque «lo que SÍ / lo que aún no» en la
  línea de 33-36.
- **Dificultad asimétrica**: UNA idea nueva por sección (dimensión → clasificación →
  referencia → enchufe → informe ejecutable); la dificultad concentrada en los retos
  (recordar once ítems sin pistas; descubrir el hueco nº12; extender la lista sin romper
  la honestidad).
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib` (milisegundos) +
  `./scripts/verify.sh` ALL_GREEN. Nunca «confía en mí».
- **Español neutro profesional** en prosa y en nombres de tipos/tests (castellano serpenteante
  como el resto del workspace: `EstadoProduccion`, `produccion_es_honesto_...`).

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (12 filas
  en §5: Gawande 2009/Haynes NEJM 2009, docs PostgreSQL/SQLite/SQLCipher/Flyway/Prometheus,
  MIGRATION §39-§41, ADR-001, código verificado 2026-08-25).
- [x] Escenario de fallo visible: la lista que miente por exceso/omisión (§6.5), el informe
  que promete de más ROMPE su test (§6.8), el caso vivo de consulta costosa indetectada
  (~224 s, §5.7), el hueco nº12 del locking inter-proceso (reto intermedio).
- [x] Misconcepciones corregidas explícitamente (§1: cuatro). Ejercicios con solución
  verificable diseñados (retos N.10).
- [x] ≥1 ejercicio de retrieval (once dimensiones de memoria) y spacing planificado
  (caps. 9, 12-13, 20, 26, 28-31, 33-36; §6.11). Responde la pregunta crítica del CORPUS
  («lista de lo que NO hace, con referencias»: informe_produccion(), once entradas,
  referencias dentro del artefacto) y recoge el gancho del cap. 36 (§3 y §6.12).
- [x] Anécdota única verificada con fuentes múltiples e independientes (The Hacker News,
  Ars Technica, Dark Reading, Bitdefender/Shodan) — Knight Capital, S3 y Apolo descartadas
  con razón explícita; precisión anti-cifra-urbana documentada (§7).
- [x] Alcance de código acotado y honesto (UN módulo `cap37_produccion.rs` + una
  declaración en `lib.rs`; cero dependencias nuevas, cero cambios en módulos cap*, goldens
  intactos; §5.2/§5.10). Gancho saliente fijado (cap-38 columnar, semilla CSR×16; §6.12).
- [x] PENDIENTE RESUELTO (2026-08-25): `cap37_produccion.rs` implementado (**623
  líneas**, std puro, cero dependencias) con los CINCO tests nombrados
  (identificador del 4.º sin tilde: `produccion_ninguna_dimension_existe_sin_cadena_completa`,
  regla sin acentos en identificadores); **853 tests** = 848+5 ALL_GREEN.
  Recuento verificado 0·6·5. Ajuste respecto al brief inline: Control de
  recursos = **Parcial** (no Ausente): `Presupuesto` (cap26_proyeccion.rs)
  existe y es construible — coincide con el diagrama §4 `[~]` y el recuento
  fijado en tres sitios del contrato. El pinzón verifica los Parcial contra
  código real mediante sondas ejecutables (`decode_header`, roundtrip WAL +
  `Checkpoint::tomar`, `Presupuesto`, `Contadores` con `# TYPE`) y pinea los
  Ausentes como evidencia negativa documentada. Frases citables del módulo:
  «la mitad de un PITR» (Backups), «Prometheus queda IMITADO, no conectado»
  (Telemetría), «estimate estima pero nada corta» (Protección). NOTA para la
  prosa: en el diagrama §4 de este contrato la fila Migraciones aparece como
  `[ ]` (correcto, Ausente) — no confundir con `[~]`.
