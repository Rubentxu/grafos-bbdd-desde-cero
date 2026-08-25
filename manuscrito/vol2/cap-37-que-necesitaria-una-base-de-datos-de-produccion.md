# Capítulo 37 — Qué necesitaría una base de datos de producción

> *«El enemigo no era la ignorancia — siempre queda documentación que leer — sino la ineptitud: saber qué se debe hacer y no verificarlo. La cura histórica tiene forma de lista. Y la única lista útil es la que no puede mentir.»*

## 37.0 La anécdota de la esquina

Primeros días de enero de 2017. Un atacante anónimo automatiza una idea cruelmente simple: barrer Internet buscando servidores MongoDB expuestos — sin firewall, sin credenciales, respondiendo a cualquiera — borrar sus datos y dejar una nota de rescate pidiendo bitcoins por devolverlos. No hubo exploit, ni zero-day, ni fuerza bruta: el software de base de datos más popular de su categoría aceptaba conexiones de cualquier origen con la autenticación desactivada por configuración por defecto, y quien lo instalaba así en una máquina con IP pública se lo encontraba abierto.

La ola fue medible porque había gente midiendo. El **4 de enero**, Bitdefender reportaba unas **2.000 instancias borradas**, citando el seguimiento de John Matherly con Shodan, su buscador de dispositivos expuestos. El **6 de enero**, Ars Technica ya contaba **más de 10.000**. Dark Reading llegó a contabilizar **22.900**, y The Hacker News cifraba **más de 27.000** hacia el día 9: decenas de miles en la primera semana, cada una con su nota de rescate. Nadie hackeó MongoDB. Lo que se explotó fue la distancia entre lo que el producto permitía configurar y lo que cada operador había **verificado** sobre su propia instalación. El post-scriptum es tan instructivo como la historia: MongoDB cambió el bind por defecto a localhost precisamente por esto, en la serie 3.6 (noviembre de 2017).

Detente en la causa raíz, porque es una fila de este capítulo: la dimensión «Autenticación» estaba, para cada una de esas bases de datos, en estado **ausente** — y nadie lo sabía, o nadie lo había mirado con la obligación de comprobarlo. Ahora la pregunta incómoda, la misma que abre la Parte VIII: ¿podrías hacer esa misma auditoría sobre LiraDB — **sin mentir**?

## 37.1 Objetivo

El capítulo anterior cerró con un mapa completo: el hexágono dibujado, 28 módulos cuadrados uno a uno, 853 tests verdes (848 + los 5 que este capítulo añade). Un mapa sirve para crecer con criterio, y crecer empieza por preguntar qué falta para vivir fuera del laboratorio. Al terminar tendrás:

1. **`informe_produccion()`**: una lista ejecutable en `crates/vol2-liradb/src/cap37_produccion.rs` (623 líneas, std puro, cero dependencias nuevas) que clasifica las **once dimensiones de producción** del brief contra la realidad verificable del workspace — cada entrada con evidencia citable, referencia industrial y punto del hexágono donde se enchufaría.
2. **El criterio de graduación**: `Existe` (cadena completa demostrada fin a fin), `Parcial` (símbolo citable + test verde + frontera documentada), `Ausente` (búsqueda fallida documentable). Hermano graduado del `NivelGarantia` del cap. 27.
3. **Cinco tests de honestidad** que pinean el informe: la lista vive compilada, y mentir en ella rompe la build.
4. **La tabla de 11 filas** — el corazón del capítulo — cuadrada fila a fila con el módulo, y el recuento honesto: **0 existen · 6 parciales · 5 ausentes**.

La tesis es la de Atul Gawande («The Checklist Manifesto», 2009): **ir a producción no es acumular features — es pasar una lista de comprobación (checklist) verificable, ítem a ítem, contra la realidad**.

## 37.2 Problema

Te preguntan: «¿esto puede ir a producción?». Y descubres que solo sabes responder con gestos. Empiezas a enumerar features al azar — «pues… replicación, y usuarios, y… backups» — sin saber cuándo parar ni cómo probar nada de lo dicho. Antes de construir el instrumento, desactivemos cuatro ideas equivocadas que suelen venir con el tema:

1. **«Producción significa más rendimiento y más features.»** No: es **otro eje**. Producción (*production readiness*) no es el camino feliz más rápido: es operar bajo condiciones adversas u hostiles — fallos de disco, procesos muertos a mitad, miradas indiscretas, consultas descontroladas. Un motor veloz sin copias de seguridad ni límites no es de producción; es un laboratorio con buena prensa.
2. **«Una base de datos embebida no necesita seguridad.»** El modelo de amenaza **cambia** — no hay listener de red que escanear — pero no desaparece: siguen ahí los permisos del fichero, otros procesos del mismo usuario, el portátil perdido. SQLite no trae autenticación embebida y lo declara alto y claro en su documentación.
3. **«Las migraciones son cosa de esquemas; schemaless se libra.»** El **formato** también evoluciona. Por eso `FORMAT_VERSION` existe desde el cap. 9: el número está reservado para el día en que el formato cambie. Que todavía valga 1 no significa que siempre vaya a valer.
4. **«Lo del catálogo cuadrático fue un fallo puntual.»** Fue un **caso vivo**: `Catalog::collect` tardó ~224 s frente a los 281 ms de construir todo el grafo (MIGRATION-PATTERN §39), y nada lo detectó — lo cazó un humano con un benchmark. Ningún mecanismo cortó la consulta. Eso convierte la «protección ante consultas costosas» en una dimensión con evidencia forense propia.

## 37.3 Modelo mental: la lista de comprobación previa al vuelo

Gawande distingue dos clases de fallo en dominios maduros: los de **ignorancia** (no sabemos suficiente) y los de **ineptitud** (sabemos lo que hay que hacer y no lo aplicamos con disciplina). Su cura histórica no es más formación: es la **lista de comprobación** — verificada ítem a ítem, en aviación desde 1935 y en cirugía desde el estudio de Haynes en NEJM (360:491-499, 2009), donde un checklist quirúrgico de la OMS redujo la mortalidad en ocho hospitales del planeta. Ir a producción es exactamente ese acto: no «tener muchas cosas», sino **recorrer una lista y poder demostrar cada marca**.

Nuestra lista tiene once ítems recordados en tres bloques — la agrupación es un truco de lectura (chunking), no un segundo modelo — y tres marcas posibles. Cada marca tiene semántica operativa precisa: `[X]` significa «te lo demuestro fin a fin ahora mismo»; `[~]` significa «hay algo real que puedes tocar y grepear, y aquí está escrita su frontera»; `[ ]` significa «busqué el símbolo y no existe — la búsqueda fallida documentada ES la evidencia». Ninguna marca se concede por optimismo ni se quita por pesimismo: se justifica con evidencia o no se concede.

```text
     LISTA DE COMPROBACIÓN PREVIA A «PRODUCCIÓN» — LiraDB Lite
     ══════════════════════════════════════════════════════════════════
     BLOQUE 1 · QUE LOS DATOS SOBREVIVAN (tiempo, accidente, miradas)
       [~] 1. Compatibilidad de formatos  magic+versión (9) · quien-abre-compara (33)
       [~] 2. Migraciones                 FORMAT_VERSION sí · evolución versionada no
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

**El momento ¡ajá!**: *production-ready no es tener más cosas — es poder demostrar, ítem a ítem contra la realidad, qué aguantará el golpe.* Y el ¡ajá! honesto adicional: **ni una sola dimensión llega a Existe** — y eso es una buena noticia operativa, no una confesión derrotista. La lista convierte «no sé si puedo» en un inventario accionable: seis frentes a medias con frontera conocida, cinco huecos con nombre.

## 37.4 Primera solución

¿Qué harías sin método? Buscar «production database checklist», copiar veinte bullets aspiracionales — replicación, sharding, alta disponibilidad, cifrado, auditoría, clustering — y pegarlos en un documento. Responde «qué dice Internet». Y de hecho es media lista de este capítulo: varias filas de la tabla vienen de referencias industriales de ese género.

## 37.5 Sus límites

1. **Miente por exceso.** Exige replicación y sharding a una base de datos **embebida sin servidor**: no hay proceso que replique, ni puerto que balancear. Una lista que no conoce tu arquitectura exige lo que quizá nunca necesites.
2. **Miente por omisión.** No distingue «lo tienes a medias» de «no existe». Nuestro WAL persistido con replay completo es «la mitad de un PITR» — una lista binaria lo marcaría o verde o rojo, y las dos marcas son falsas.
3. **Nadie puede auditarla.** Vive en un wiki o en un PDF que se pudre: nada conecta cada bullet con el código que supuestamente lo cumple. Al mes siguiente puede estar mintiendo sin que nadie lo note.
4. **Es el género literario exacto del que se alimentó el ransomware.** Miles de operadores habían leído «pon autenticación a tus MongoDB» en listas de buenas prácticas. La lectura no salvó a nadie: la **verificación** habría bastado.

## 37.6 Solución evolucionada: las once dimensiones, clasificadas

La evolución no añade bullets: añade **disciplina**. Cada dimensión se clasifica contra el workspace con un criterio que no admite amabilidades: `Parcial` exige símbolo citable más test verde con frontera documentada; `Ausente` exige búsqueda fallida documentable; `Existe` exigiría cadena completa demostrada fin a fin — y ninguna la alcanza hoy.

El orden de lectura recomendado recorre los tres bloques como quien revisa un checklist en tierra. **DATOS** pregunta si lo que escribiste sobrevive al tiempo, al accidente y a las miradas: formato que evoluciona sin romperse, copias restaurables, secretos ilegibles. **PROCESO** pregunta si el motor opera con cabeza bajo adversidad: recursos acotados, consultas descontroladas cortadas, comportamiento observable desde fuera, herramientas para operarlo sin abrir el código. **PERSONAS** pregunta quién responde por el sistema: identidad, permisos, mínimo privilegio. Fíjate en que los tres bloques son también una progresión de gravedad: un dato perdido se restaura; un proceso desbocado se mata; una base de datos abierta al mundo se lee entera — y el ransomware MongoDB vivió exactamente ese tercer bloque.

Esta es la tabla corazón del capítulo, cuadrada fila a fila con `informe_produccion()` (las filas siguen el orden del brief; los bloques son agrupación de lectura):

| Dimensión | Estado | Qué hay HOY (evidencia citable) | Quién lo resuelve (industria) | Se enchufa en |
|---|---|---|---|---|
| Compatibilidad de formatos | `[~]` parcial | magic + `FORMAT_VERSION` en la cabecera (cap. 9); `decode_header` rechaza magic corrupto; roundtrips CSV/JSONL/GraphML ida y vuelta (32-33) | SQLite promete estabilidad del formato **por décadas** (*file-format stability promise*, sqlite.org/fileformat2.html); PostgreSQL compara la versión del catálogo al arrancar | encoding (9): todo cambio futuro pasa por `encode_header`/`decode_header` |
| Seguridad | `[ ]` ausente | Sin modelo de amenaza escrito ni **hardening** documentado; entre el contenido y otro proceso solo los permisos del fichero (`FilePager`, 12) | PostgreSQL documenta su modelo completo (roles, `pg_hba.conf`, SSL); SQLite delega en el sistema de ficheros y lo declara | — : empezaría como documento de amenazas junto a la CLI (31) |
| Autenticación | `[ ]` ausente | No existe el concepto de usuario: ninguna firma del crate menciona credenciales | PostgreSQL: `CREATE ROLE` + `pg_hba.conf`; MongoDB activó auth/bind-localhost tras el ransomware (serie 3.6); SQLite declara que NO trae auth embebida | CLI/API conductora (31): la identidad llega antes de la primera consulta |
| Autorización | `[ ]` ausente | `put_node`/`delete_edge` del puerto (8) no transportan quién pregunta: no hay dónde decidir | PostgreSQL: `GRANT`/`REVOKE` por objeto y operación; Neo4j administra roles por comando | Puerto GraphStore (8)/Executor (20): permitir/denegar por operación |
| Cifrado | `[ ]` ausente | Páginas planas: `FilePager::write_page` escribe bytes tal cual (12); `guardar_wal` igual (29); un dump revela labels y propiedades enteras | SQLCipher cifra página a página sobre SQLite; PostgreSQL ofrece `pgcrypto` por columna — el core NO trae TDE nativo | Bajo el puerto Pager (12): transparente para todo lo construido encima |
| Copias de seguridad | `[~]` parcial | `guardar_wal` persiste el log y `reabrir` ejecuta replay completo (28-29): **la mitad de un PITR**. Frontera dura: `guardar_wal` NO hace fsync (doc-comment `cap29_recuperacion.rs:611`) aunque `FilePager::sync` SÍ llama `sync_all` (12) | `pg_basebackup` + archivado del WAL = PITR (PostgreSQL); Online Backup API y `.backup` de la CLI sqlite3 | Recuperación (28-29) + Pager (12): sync garantizado y checkpoint de páginas |
| Migraciones | `[~]` parcial | Solo la política: `FORMAT_VERSION == 1` (9) y «quien abre compara». No hay camino versionado v1→v2 que transforme un fichero antiguo | Flyway/Liquibase versionan esquemas SQL; Diesel migrations en Rust (diesel.rs) | encoding (9): el número reserva el sitio; falta el paso de transformación |
| Control de recursos | `[~]` parcial | `Presupuesto` acota profundidad/nodos/lecturas de los **algoritmos** con `MotivoParada` (26) — opt-in y solo recorridos; el Executor corre sin límite: el catálogo cuadrático corrió ~224 s sin que nada lo detuviera | `statement_timeout` y `pg_cancel_backend` cortan consultas (PostgreSQL); `PRAGMA soft_heap_limit` acota memoria en SQLite | Executor (20): la idea existe; falta hacerla valer para toda consulta |
| Protección ante consultas costosas | `[ ]` ausente | `explain` estima cardinalidades (21) pero nada corta — **estimate estima pero nada corta**: ni timeout, ni cancelación, ni guardia de coste; el caso vivo lo cazó un humano | `statement_timeout` ES el mecanismo industrial: matar la consulta al vencimiento; `max_execution_time` juega el mismo papel en MySQL | Executor (20): un reloj por `next()` del operador raíz sería el corte mínimo |
| Telemetría | `[~]` parcial | Contadores con formato de exposición Prometheus (`# TYPE … counter`), `ExecMetrics` (20), spans por operador y `--profile` (35). Todo muere en el proceso: **Prometheus queda imitado, no conectado** | Prometheus define ese exposition format (prometheus.io); OpenTelemetry estandariza traces/metrics/logs | Decoradores medidores (35): el mismo punto serviría para exportar |
| Herramientas operativas | `[~]` parcial | `liradb-cli` con demo/query/explain/repl/script/import/export + flags (31), más inspect/check/compact (16). Germinal: sin init/status/backup ni modo daemon | CLI sqlite3 (`.backup`/`.dump`/`.schema`) como precedente embebido; `neo4j-admin dump/load` y `pg_ctl status` en servidores | CLI (31): cada subcomando nuevo es un ítem hecho herramienta |

Lee la columna de estados con lupa: **Compatibilidad de formatos es Parcial y no Existe** — el formato se lee y se valida, pero comparar versiones nunca ha tenido que *evolucionar* nada todavía, así que la cadena completa que exigiría un `[X]` no existe. Y **Control de recursos es Parcial y no Ausente**: el `Presupuesto` del cap. 26 existe, es construible y acota de verdad — su frontera es ser opt-in para algoritmos, no un límite global del motor. El matiz es la lección; por eso el tipo tiene tres estados y no un booleano.

## 37.7 Código completo ejecutable

El código vive en `crates/vol2-liradb/src/cap37_produccion.rs` (623 líneas, std puro) y `lib.rs` lo declara con la convención de siempre: `pub mod cap37_produccion; pub use cap37_produccion::*;`. Es el único código nuevo del capítulo — y es el patrón-tesis del libro: heredero directo de `informe_acid()` (cap. 27), documentación que no puede mentir porque los tests la pinan. Los tipos, en su esqueleto:

```rust
pub enum EstadoProduccion { Existe, Parcial, Ausente }

impl EstadoProduccion {
    /// La marca del checklist pre-vuelo: [X] / [~] / [ ].
    pub fn marca(self) -> &'static str {
        match self {
            EstadoProduccion::Existe  => "[X]",
            EstadoProduccion::Parcial => "[~]",
            EstadoProduccion::Ausente => "[ ]",
        }
    }
}

pub enum BloqueProduccion { Datos, Proceso, Personas }

pub struct DimensionProduccion {
    pub nombre: &'static str,
    pub bloque: BloqueProduccion,
    pub estado: EstadoProduccion,
    pub como_esta_hoy: &'static str,
    pub quien_lo_resuelve: &'static str,
    pub donde_se_enchufaria: &'static str,
}

pub fn informe_produccion() -> InformeProduccion { /* las once entradas */ }
```

Tres decisiones con su porqué:

- **Tres estados, no booleanos ni scores.** «Tiene/no tiene» oculta justo lo que este capítulo enseña: las fronteras. Y `recuento()` devuelve una tupla `(existe, parcial, ausente)` — sumar honestidades en un número produce marketing, no conocimiento (misma razón por la que el cap. 27 nunca dio un «índice ACID»).
- **Las referencias viajan DENTRO del artefacto** (`quien_lo_resuelve` es un campo, no un apartado de prosa): son compilables, grepeables y auditables — la referencia no se desincroniza del informe en la próxima edición.
- **El orden 1:1 del brief se conserva en `entradas()`**; solo el `Display` agrupa por bloques. Fidelidad literal al brief — nada inventado, nada omitido — y el ejercicio de memoria del final depende de ese orden.

Para que veas cómo respira una entrada real, así imprime el Display la fila de copias de seguridad (strings literales del módulo, abreviados):

```text
DATOS — que los datos sobrevivan:
  [~] Copias de seguridad — parcial: la mitad buena: guardar_wal persiste el log
      a fichero y reabrir ejecuta replay COMPLETO (28-29) — ¡la mitad de un PITR! …
      Quién lo resuelve: pg_basebackup + archivado del WAL = PITR en PostgreSQL …
      Dónde se enchufaría: recuperación (28-29) + Pager (12): sync GARANTIZADO …
```

Cada línea de entrada lleva su marca `[~]`, su estado en minúscula, y las dos líneas de contexto: quién lo resuelve y dónde se enchufaría. El informe completo termina con `Recuento: 0 existen · 6 parciales · 5 ausentes` y `Leyenda: [X]=existe   [~]=parcial   [ ]=ausente`. Es exactamente la figura del §37.3 hecha programa.

## 37.8 Prueba de fuego

```text
$ cargo test -p vol2-liradb --lib cap37

running 5 tests
test cap37_produccion::tests_produccion::produccion_cubre_las_once_dimensiones_del_brief ... ok
test cap37_produccion::tests_produccion::produccion_ninguna_dimension_existe_sin_cadena_completa ... ok
test cap37_produccion::tests_produccion::produccion_display_tres_bloques_once_lineas ... ok
test cap37_produccion::tests_produccion::produccion_es_honesto_sobre_el_estado_actual ... ok
test cap37_produccion::tests_produccion::produccion_parciales_citan_simbolos_reales ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 663 filtered out; finished in 0.00s
```

Cinco verdes en milisegundos, y el workspace completo sigue en **853 tests ALL_GREEN** con goldens intactos. Pero la prueba de verdad es la **demostración inversa**, herencia directa del cap. 27 («informe_acid() no puede mentir»): cambia UNA entrada a `EstadoProduccion::Existe` sin implementar nada — `produccion_ninguna_dimension_existe_sin_cadena_completa` se pone rojo al instante. Y si tocas un estado que ya tenía evidencia — digamos, marcar Herramientas operativas como Ausente por error — el pinzón te dice exactamente qué hacer:

```text
thread 'cap37_produccion::tests_produccion::produccion_es_honesto_sobre_el_estado_actual'
panicked at src/cap37_produccion.rs:
Herramientas operativas: el informe cambió — actualiza este pinzón con la
evidencia nueva, o revierte la clasificación
```

El cerrojo funciona en las DOS direcciones:

- Si **mejoras el código** (implementas fsync garantizado y checkpoint, digamos) y olvidas actualizar el informe, `produccion_es_honesto_sobre_el_estado_actual` falla con sus sondas ejecutables — el test verifica cada Parcial contra código real: `encode_header`/`decode_header` rechazando un magic corrupto, un roundtrip `guardar_wal`→`cargar_wal` con checkpoint idéntico, un `Presupuesto` construible, `Contadores` hablando formato Prometheus.
- Si **inflas el informe** sin tocar el código, el mismo pinzón te muerde: los estados están pineados uno a uno.

Actualizar una clasificación exige tocar informe y test **juntos** — como debe ser: es la única clase de lista que no se queda obsoleta ni infla.

## 37.9 Qué hemos sacrificado

1. **Nada de producción quedó implementado.** Clasificamos, no reparamos: meter un password «de paso» sería teatro sin modelo de despliegue que lo justifique — y rompería la frontera de honestidad de los caps. 33-36. La Parte VIII elegirá frentes; este capítulo mapea.
2. **Granularidad gruesa.** Tres estados, no porcentajes. Un «73% production-ready» sería precisión falsa: o puedes demostrar la frontera, o no.
3. **La lista omite dimensiones que un sistema grande también quiere.** La **replicación** (*replication*) y la alta disponibilidad brillan por su ausencia del informe base — van a los ejercicios, no a las once del brief. Nombrar el hueco de la lista también es honestidad.
4. **La amenaza de red queda fuera de alcance** mientras no exista servidor. Es la frontera embedded declarada, no fingida: cuando haya listener, habrá que reescribir el modelo de amenazas entero.
5. **Sin golden nuevo.** El Display ya está pineado por `produccion_display_tres_bloques_once_lineas`; dorarlo otra vez duplicaría la garantía.

## 37.10 Cómo lo hace una BBDD real

Recorrido por bloque, con PostgreSQL como ejemplo completo, SQLite como contraejemplo embebido y Neo4j donde aplique.

**Bloque DATOS.** La compatibilidad de formatos tiene dos escuelas: SQLite publica su *file format stability promise* — el fichero que escribes hoy se leerá dentro de décadas — y PostgreSQL, al revés, compara la versión del catálogo de datos al arrancar y se niega a abrir un directorio anterior. Nosotros tenemos la segunda mitad de la política («quien abre compara») sin haber necesitado aún la primera. Las **migraciones** (*schema migration*) son industria madura: Flyway y Liquibase versionan esquemas SQL con scripts numerados; Diesel migrations hace lo propio en Rust. El reto experto del final te pide diseñar el paso v1→v2 de `FORMAT_VERSION` — exactamente lo que esas herramientas llevan décadas refinando. En **backups**, `pg_basebackup` + archivado continuo del WAL dan el **PITR** (*point-in-time recovery*: restaurar a un instante concreto) — y date cuenta: nuestro `guardar_wal`→`reabrir` ya ES la mitad de un PITR; le falta el fsync garantizado y el checkpoint de páginas. SQLite trae Online Backup API y `.backup` en su CLI. En **cifrado**, SQLCipher cifra página a página sobre SQLite; PostgreSQL ofrece `pgcrypto` por columna — su core no trae TDE nativo. El punto de enchufe natural es bajo el puerto `Pager`: cifrar ahí sería transparente para todo lo construido encima.

**Bloque PROCESO.** El mecanismo industrial contra consultas descontroladas es el **`statement_timeout`** de PostgreSQL: matar la consulta al vencimiento, complementado por `pg_cancel_backend` para cancelarla a mano. Nuestro `explain` calcula estimaciones — estimate estima — pero nada corta, y el catálogo de ~224 s es el recibo. En **recursos**, SQLite expone `PRAGMA soft_heap_limit` para acotar memoria; nuestro `Presupuesto` es el embrión, limitado a algoritmos opt-in. La **telemetría**: Prometheus consume un formato de exposición que ya imitamos a mano en el cap. 35 (`# TYPE … counter`) — el paso que falta es exportarlo a un backend, y OpenTelemetry estandariza traces/metrics/logs para no inventar el protocolo. Las **herramientas operativas**: `sqlite3` demuestra que hasta un motor embebido merece CLI con `.dump`/`.schema`; en servidores completos, `pg_ctl status` y `neo4j-admin dump/load`. Nuestros inspect/check/compact del cap. 16 eran el germen; faltan init/status/backup. Y una nota de paisaje: Neo4j sigue vivo y administrado por roles; Kùzu fue archivada tras su adquisición por Apple (octubre de 2025) y la comunidad continúa en los forks LadybugDB y bighorn — su paper de sistema es CIDR 2023 (atribución según ADR-001).

**Bloque PERSONAS.** PostgreSQL es la referencia completa: roles con `CREATE ROLE`, permisos por objeto con `GRANT`/`REVOKE`, control de conexión con `pg_hba.conf`, TLS para el tráfico. El **cifrado en reposo/en tránsito** (*at rest/in transit*) separa lo que protege el disco de lo que protege la red — nosotros no tenemos ninguno porque ni siquiera hay red. Y el contraste embebido lo da SQLite: declara en su documentación que no incluye autenticación — el acceso lo decide el sistema de ficheros —, que es exactamente nuestro modelo actual, dicho sin rodeos. MongoDB aprendió la diferencia entre configurar y verificar en enero de 2017: la lección no es «pon password», es **poder demostrar qué tienes puesto**.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (recordar — retrieval practice): cierra el libro y escribe DE MEMORIA las once dimensiones **en su orden del brief**, con su marca `[X]/[~]/[ ]` junto a cada una. Sin pistas. Autocorrígete contra la salida impresa de `informe_produccion()` (o contra la figura del §37.3): el orden importa porque el test `produccion_cubre_las_once_dimensiones_del_brief` lo exige posición a posición.
- *Intermedio* (analizar — interleaving caps. 10/12-13/16/31): dos procesos abren EL MISMO fichero LiraDB a la vez. ¿Qué dimensión rompe primero? Solución esperada: **ninguna de las once lo cubre** — falta el locking inter-proceso, hueco nº12. `FilePager`/`BufferPool` asumen dueño único (caps. 12-13); dos escritores mezclan páginas, el CRC salta tarde y antes hay corrupción silenciosa. Contrasta con SQLite, que usa POSIX advisory locks y los documenta en lockingv3.html.
- *Experto* (crear): añade UNA dimensión nueva — locking inter-proceso o replicación — como entrada nº12 completa de `informe_produccion()`: nombre, bloque, estado justificado, evidencia, referencia industrial, enchufe. Actualiza el test-pinzón y el recuento del Display. Criterio de éxito: suite verde, y el informe SIGUE sin poder mentir.

## 37.11 Lo que te llevas

- **Producción es otro eje**: no rendimiento ni features — garantías operativas bajo adversidad y hostilidad, verificables ítem a ítem.
- **La lista de comprobación es el acto**: Gawande — el enemigo es la ineptitud de no verificar lo sabido, no la ignorancia.
- **Tres estados honestos**: `Existe` exige cadena completa; `Parcial`, símbolo + test + frontera documentada; `Ausente`, búsqueda fallida documentada. Sumarlos en un score produce marketing.
- **El recuento de la casa: 0·6·5.** Ni una dimensión llega a Existe — y la lista convierte esa confesión en inventario accionable.
- **El informe es ejecutable**: `informe_produccion()` pineado por cinco tests, cerrojo bidireccional — código e informe evolucionan juntos o la build grita.
- **Cada dimensión lleva referencia y enchufe**: quién lo resuelve en la industria y en qué punto del hexágono se conectaría. El mapa del 36 deja de ser retrato: es base de crecimiento.
- **Casos vivos como argumentos**: el catálogo de ~224 s prueba por qué la protección ante consultas costosas es una dimensión; el ransomware MongoDB prueba por qué la autenticación lo es.
- **«La mitad de un PITR» es un cumplido preciso**: nuestro WAL+replay vale eso — y saberlo con exactitud vale más que un bullet verde de una lista ajena.

## 37.12 Ojo, cuidado con…

- **Confundir Parcial con «casi Existe».** La frontera ES la lección: Backups=Parcial porque `guardar_wal` no hace fsync — el doc-comment lo confiesa — y proclamarlo completo sería exactamente la falsedad que los tests existen para impedir.
- **Sumar honestidades.** «6 de 11 = 55% listo» mezcla cosas incomparables: un Parcial con fsync pendiente no pesa lo mismo que uno sin export de métricas. Tupla, no score.
- **Copiar checklists genéricas sin contrastar.** Es el género que el ransomware se comió: cada ítem ajeno necesita TU evidencia local antes de marcarlo.
- **Creer que embedded exime.** Sin listener de red, el fichero y los procesos del mismo usuario siguen siendo la superficie: SQLite lo declara, nosotros deberíamos también.
- **Actualizar el informe sin los tests (o al revés).** El pinzón exige que evolucionen juntos; si ves verde tras tocar solo uno, sospecha del pinzón antes que celebrar.

## 37.13 Pin de batalla

> *«Una lista sin evidencia verificable es marketing; pinada por tests, es un contrato. La producción no se promete: se recorre ítem a ítem, con la build mirando.»*

## 37.14 Si solo lees 30 segundos

Capítulo-inventario: cero producción implementada, un artefacto nuevo — `cap37_produccion.rs` (623 líneas, 5 tests) — con `informe_produccion()`: las once dimensiones del brief clasificadas contra el código real. Recuento honesto **0 existen · 6 parciales · 5 ausentes**. Parciales: compatibilidad de formatos, copias de seguridad (la mitad de un PITR: WAL+replay, sin fsync garantizado), migraciones (política sí, camino v1→v2 no), control de recursos (`Presupuesto` opt-in para algoritmos), telemetría (Prometheus imitado, no conectado), herramientas operativas (CLI germinal). Ausentes: seguridad, autenticación, autorización, cifrado, protección ante consultas costosas (caso vivo: catálogo ~224 s que nadie cortó). Modelo mental: el checklist pre-vuelo de Gawande — el enemigo es la ineptitud de no verificar, y la cura es una lista que no pueda mentir. Referencias industriales dentro del artefacto; enchufe en el hexágono por dimensión. Gancho: la Parte VIII construye DÓNDE crecer, empezando por lo columnar.

## 37.15 Una historia pequeña

Wright Field, Ohio, 30 de octubre de 1935. El Boeing Model 299 — el bombardero de cuatro motores más avanzado de su época, prototipo de la futura B-17 — despega ante los mandos del Cuerpo Aéreo del Ejército, se encabrita, entra en pérdida y se estrella en llamas. Mueren el mayor Ployer P. Hill — jefe de la rama de vuelo de Wright Field, piloto de pruebas veterano que había evaluado cerca de sesenta aparatos — y Leslie Tower, jefe de pilotos de pruebas de Boeing. La junta investigadora descartó lo impensable: no hubo fallo estructural, ni de motores, ni de diseño. Los mandos de vuelo seguían bloqueados por el *gust lock* — un seguro que inmoviliza superficies en tierra contra las ráfagas — que la tripulación olvidó soltar antes de despegar. No ignoraban el procedimiento: lo sabían, y no lo aplicaron. La respuesta del Cuerpo Aéreo no fue entrenar mejor a nadie: fue inventar un género — la lista de comprobación por fases (despegue, vuelo, aterrizaje), hoy obligatoria en toda la aviación civil y militar. Gawande abre su libro con esta historia porque contiene toda la tesis: cuanto más competente es la gente, más cree que no necesita la lista — y más la necesita. LiraDB acaba de escribir la suya. Que esté pineada por tests es nuestra manera de que ningún mayor Hill la recite de memoria en vez de recorrerla.

## Ejercicios resueltos

**1. ¿Por qué Compatibilidad de formatos es `[~]` y no `[X]`, si el formato «funciona»?** Porque `Existe` exige cadena completa demostrada fin a fin, y falta el eslabón que este capítulo bautiza: la evolución versionada. Tenemos magic + `FORMAT_VERSION` en la cabecera (cap. 9), `decode_header` que rechaza un magic corrupto y devuelve la versión, y roundtrips de import/export (32-33) — pero no existe v2, así que «quien abre compara» nunca ha tenido que negociar un cambio real. El criterio no permite el `[X]` amable; dárselo al formato sería la falsedad que el libro prohíbe. Verificación: `produccion_ninguna_dimension_existe_sin_cadena_completa` pinea `(0, 6, 5)` y afirma que ninguna entrada alcanza `Existe`.

**2. ¿Por qué Control de recursos es `[~]` y Protección ante consultas costosas es `[ ]`, si ambas hablan de límites?** Porque Parcial exige símbolo citable con frontera documentada, y solo uno lo tiene. `Presupuesto` (cap. 26) existe, es construible con `con_profundidad/con_nodos/con_lecturas`, acota de verdad con `MotivoParada` — y su frontera está dicha: opt-in y solo para algoritmos de grafo. Para timeouts y cancelación no hay símbolo que citar: la búsqueda fallida ES la evidencia, y el catálogo cuadrático (~224 s frente a 281 ms, §39) demostró en producción doméstica que nada corta una consulta cara. Verificación: `produccion_parciales_citan_simbolos_reales` construye el `Presupuesto`; `produccion_es_honesto_sobre_el_estado_actual` pinea ambos estados.

**3. ¿Por qué Migraciones es `[~]` Parcial y no `[ ]` Ausente, si «no hay herramienta que migre nada»?** Porque lo que se clasifica es la dimensión, no la herramienta completa. La POLÍTICA de versionado existe y es citable: `FORMAT_VERSION == 1` vive en el cap. 9, está pineado por test (`produccion_parciales_citan_simbolos_reales` afirma `assert_eq!(FORMAT_VERSION, 1)`), y la regla «quien abre compara» ya opera desde los roundtrips del cap. 33. Lo que falta — el paso transformador v1→v2 — está documentado como frontera en la propia entrada. Con tres estados, «solo la mitad política» es exactamente un Parcial; llamarlo Ausente borraría el trabajo real del cap. 9, y llamarlo Existe prometería una evolución que nunca se ha ejecutado. Verificación: la fila pineada en `produccion_es_honesto_sobre_el_estado_actual` como `("Migraciones", EstadoProduccion::Parcial)`.

## Ejercicios propuestos

**Esencial (recordar — retrieval practice).** Del reto esencial del §37.10, ahora con verificación dura: escribe las once dimensiones de memoria con su marca, y comprueba contra `informe_produccion().to_string()` — imprímelo desde un test temporal o léelo de `produccion_display_tres_bloques_once_lineas`. Criterio: once nombres exactos (el test compara strings), once marcas correctas, y el orden del brief respetado. Si fallaste dos o menos: bien; si fallaste el orden: repite mañana — el orden es parte del contrato.

**Intermedio (analizar — interleaving 12-13 + 16 + 31).** Desarrolla el escenario del reto intermedio: enumera qué invariantes viola la doble apertura (¿quién es dueño del offset de escritura? ¿qué ve `check` del cap. 16?), predice si el CRC del framing detectará la corrupción pronto o tarde, y explica por qué NINGUNA de las once dimensiones cubre el hueco. Cierra comparando con los POSIX advisory locks de SQLite (lockingv3.html): ¿qué cambiaría en nuestro modelo de amenazas si existiera locking inter-proceso? Criterio: el hueco nº12 nombrado, con la cadena de daño explicada sin ejecutar nada.

**Experto (crear — extensión honesta).** Ejecuta el reto experto: entrada nº12 completa en `informe_produccion()` (locking inter-proceso o replicación), test-pinzón actualizado con los DOCE estados, Display con recuento nuevo. Restricciones: std puro, cero dependencias, y cada clasificación con evidencia citable o búsqueda fallida documentada. Verificación: `cargo test -p vol2-liradb --lib` verde. Criterio: la suite entera sigue siendo la prueba de que el informe no miente — si tu entrada nueva rompe esa propiedad, no está terminada.

## Para profundizar

- **Atul Gawande, «The Checklist Manifesto: How to Get Things Right» (Metropolitan Books, 2009)** — la tesis del modelo mental; abre con el accidente del Model 299.
- **Alex B. Haynes et al., «A Surgical Safety Checklist to Reduce Morbidity and Mortality in a Global Population» (NEJM 360:491-499, 2009)** — el estudio de la OMS: la mortalidad quirúrgica cayó cuando alguien verificó ítem a ítem lo sabido.
- **National Museum of the U.S. Air Force, «Model 299 Crash» (nationalmuseum.af.mil)** — el acta de la junta investigadora de 1935: gust lock, no diseño. Fuente primaria de la historia pequeña.
- **Bitdefender (4-ene-2017), Ars Technica (6-ene-2017), Dark Reading y The Hacker News (9-ene-2017)** — la cronología independiente del ransomware MongoDB; y las notas de versión de MongoDB 3.6 (bind por defecto a localhost).
- **postgresql.org/docs** — `CREATE ROLE`/`GRANT`/`REVOKE`, `pg_hba.conf`, `pgcrypto` (y por qué el core no trae TDE nativo), *Continuous Archiving and Point-in-Time Recovery*, `statement_timeout`, `pg_cancel_backend`.
- **sqlite.org** — *fileformat2.html* (file-format stability promise), *lockingv3.html* (advisory locks), la CLI y su `.backup`/`.dump`.
- **sqlcipher.net** — cifrado página a página sobre SQLite; **flywaydb.org / liquibase.org / diesel.rs** — migraciones versionadas.
- **prometheus.io** (exposition format) y **opentelemetry.io** — la telemetría que imitamos y la que estandariza el mundo real.
- Dentro del libro: cap. 9 (FORMAT_VERSION), caps. 12-13 (Pager/pool y su dueño único), cap. 26 (Presupuesto), cap. 27 (informe_acid, el hermano mayor), caps. 28-29 (WAL/recuperación: la mitad del PITR), cap. 31 (CLI), cap. 34 (la medida ×16), caps. 35-36 (contadores y hexágono).

## Mini-diálogo: en guardia nocturna

> — Antes de apagar: mañana te presentan a los de infraestructura. Quieren «pasar LiraDB a producción» el mes que viene. ¿Qué les dices?
>
> — Les enseño una lista.
>
> — ¿Una lista? Eso es todo tu arsenal?
>
> — Es exactamente el arsenal correcto. Los aviones no despegan porque el piloto sea bueno: despegan porque alguien recorrió el checklist ANTES del despegue. El orden importa: la lista se recorre en tierra, no durante el incendio.
>
> — Vale, pero… ¿qué pasa cuando lean que hay ceros?
>
> — Que leerán la verdad con dirección. Cero existen, seis a medias con frontera escrita, cinco huecos con nombre. Cada fila dice qué hay, quién lo resuelve en la industria y dónde se enchufaría en el hexágono. ¿Prefieres que lleve veinte bullets de Internet y fe ciega?
>
> — Y si me preguntan «¿cuándo estará listo?»…
>
> — Les enseñas el recuento y el criterio: una fila pasa a `[X]` cuando exista cadena completa demostrada — y el test-pinzón será quien firme la fecha. Ni yo ni el director del proyecto podemos mentir ahí: la build nos delata.
>
> — Suena raro dormir tranquilo con cinco casillas vacías.
>
> — Al contrario: es la primera noche que sé EXACTAMENTE qué queda. Apaga. La lista aguanta.

---

*(Próximo capítulo: 38 — almacenamiento columnar y ejecución vectorizada. La lista dijo QUÉ falta; ahora la Parte VIII construye DÓNDE crecer, señalando un punto del hexágono. El primero sale de una medida propia: el cap. 34 midió el puerto clonando un Vec por llamada — ×16 frente a la CSR cruda iterando 500k aristas. Columnar/vectorizado es la primera de las tres apuestas técnicas.)*
