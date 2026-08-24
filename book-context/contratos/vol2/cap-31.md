# CONTRATO DE CAPÍTULO — Vol.II Cap. 31: La CLI de LiraDB

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb-cli/src/lib.rs` (1.035 líneas: clap
> Builder, `run_con_entrada`, despachar, cmd_query/demo/explain/repl/script,
> `imprimir_ayuda` curada y **34 tests** en `mod tests`, verificados
> ALL_GREEN: 728 → 751 tests workspace) y `src/sesion.rs` (482 líneas:
> `Sesion`, `interpretar_linea` compartido, `bucle_transaccion` con prompt
> `tx>` y la tx POR VALOR, parsers hacia `Value`); `main.rs` fino (23 líneas).
> Decisiones/bugs reales: `MIGRATION-PATTERN.md` **§36** (E0507 tx-por-valor,
> el binario fantasma del target-dir global, delimitación contra el cap. 32)
> y **§25** (el hito CLI mínima con parseo manual que este capítulo sustituye).
> Pregunta crítica del CORPUS (`vol-II-cap-31`): «REPL vs script mode; clap
> ergonomics». Este capítulo ABRE la Parte VII (Convertir el proyecto en un
> producto técnico). Gancho: cap. 32 (import/export que se enchufa a ESTE
> intérprete).

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: el hito CLI mínima del §25 (`run(args, out,
  err) -> i32`, 3 subcomandos con parseo MANUAL de `std::env::args`, ayuda
  literal, exit codes 0/1/2, `emitir` tolerante a E/S, 11 tests end-to-end
  con `Vec<u8>`); el pipeline parse → lower → optimizador → Volcano y el
  `ResultSet` tabulado (caps. 17-21, `vol2_liradb::run` y `explain`);
  `demo_graph()` como API única (cap. 20); el `Value` del cap. 7; el trait
  `GraphStore` como puerto hexagonal (cap. 8); la `Transaccion` del cap. 27
  (begin → stage → commit|rollback, `commit`/`rollback` CONSUMIENDO `self`,
  drop implícito = rollback seguro, `autocommit`, el préstamo exclusivo
  `&mut` como cerrojo); lectores `&self` / escritor `&mut` (cap. 30).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «la CLI es un detalle: unos `println!` y un `match`» — no: es el
  PRODUCTO que abre la Parte VII, y sus decisiones (entrada inyectada,
  intérprete compartido) deciden qué se puede testear y qué cabida tendrá el
  cap. 32; (2) «REPL y script mode son dos programas» — son DOS POLÍTICAS
  (prompts, errores) sobre UN intérprete (`sesion::interpretar_linea`);
  (3) «testear una CLI = compilar, spawnear y leer el exit code» — no:
  `run_con_entrada` con `Cursor` y `Vec<u8>` cubre lo mismo sin procesos;
  (4) «la ayuda autogenerada siempre es mejor» — la de clap documenta el
  ÁRBOL, no el PRODUCTO (ejemplos copiables, grafo demo, gancho al 32);
  (5) «el REPL debería guardar la transacción en la sesión» — guerra de
  lifetimes: la API del cap. 27 ya decidió que la tx ES el préstamo del
  store, y el sub-bucle por valor es la única forma que no pelea con los
  tipos; (6) «un guion con un error debería seguir como el REPL» — al
  revés: Unix manda fail fast (el REPL charla, el guion despacha);
  (7) «EOF con la tx abierta da igual» — es rollback implícito por
  construcción (el drop del cap. 27 hecho producto).
- **NO debe saber todavía**: import/export CSV/JSONL/GraphML y su streaming
  (cap. 32); pruebas de una BD (cap. 33); benchmarks (cap. 34);
  observabilidad (cap. 35); completions (`clap_complete`), readline/TTY raw
  mode; configuración persistente; la Derive API de clap EN PROFUNDIDAD
  (se contrasta, no se desarrolla). Se nombra como «luego lo verás» y se corta.

## 2. Conceptos (del grafo curricular)

- `present`: clap con **Builder API** (`Command`, `Arg`, `ArgAction::SetTrue`,
  `default_value`, `value_parser` con lista de valores,
  `try_get_matches_from`, `disable_help_flag/subcommand`,
  `Error::render`/`use_stderr`/`exit_code`); el REPL como bucle
  leer-ejecutar-imprimir con SESIÓN persistente entre líneas; los
  meta-comandos `:help :quit :demo :clear :graph :node :edge :del-node
  :del-edge :begin :commit :rollback`; el script mode con `-` = stdin,
  comentarios `#`/`--` y parada en el primer error CON Nº DE LÍNEA; el
  patrón «dos frontales, un intérprete» (`Accion::Seguir|Salir`,
  `Err(String)` sin imprimir); el prompt `tx>` como préstamo exclusivo
  visible; `run_con_entrada` con `&mut dyn Read` inyectado; exit codes
  0/1/2 y la disciplina stdout (datos) vs stderr (diagnósticos).
- `practice`: `Transaccion`/`autocommit`/`Operacion` del cap. 27 (ahora en
  un sub-bucle POR VALOR); el `Value` del cap. 7 (destino de `parse_valor`);
  el pipeline caps. 17-21 (`pipeline_con_detalle` parametriza lo que `demo`
  desenrollaba); `explain` (cap. 21); `demo_graph` (cap. 20); `&mut dyn
  GraphStore` como cerrojo (caps. 8/27); `BufRead::read_line` y el `Ok(0)`
  que es EOF; la cascada de `delete_node` (cap. 8) que `:del-node` expone.
- `consolidate`: la testabilidad por inyección del hito (§25: «una CLI
  testable se diseña al revés»); `demo_graph` como único punto de verdad;
  «derivar, no llevar en cabeza»; la arquitectura hexagonal vista al revés:
  la CLI es el ADAPTADOR DE CLIENTE que consume el puerto.
- `out_of_scope` (solo nombrar): import/export y sus formatos (cap. 32);
  tests de integración del motor (cap. 33); rendimiento (cap. 34);
  `clap Derive` como elección de producción (contrastada, no desarrollada);
  TTY, historial, resaltado; `clap_complete`.

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) enumera el árbol de comandos (demo/query/explain/repl/
  script + help) con sus flags (`--graph demo|empty`, `--plan`, `--stats`)
  y dice qué exit code produce cada fallo (0 éxito/ayuda/versión · 1
  consulta o fichero · 2 uso incorrecto de clap); (2) explica Builder vs
  derive con el trade-off real (árbol a la vista y cero macros frente a
  ergonomía declarativa) y por qué el libro enseña Builder aquí; (3) dice
  de memoria qué meta-comandos son legales DENTRO de una transacción y por
  qué las consultas se rechazan («el store está prestado» — cap. 27);
  (4) explica por qué la tx se pasa POR VALOR al sub-bucle y qué tiene
  que ver E0507 con que `commit`/`rollback` consuman `self`; (5) explica
  la división stdout/stderr y qué se rompe si se cruza (pipes, exit codes).
- **Skills**: (1) monta una sesión end-to-end por stdin (`:clear`, `:node`,
  `:edge`, consulta, `:begin/:commit`) y predice su salida exacta (prompts,
  autocommit, resumen de commit, exit code); (2) escribe un guion `.ql`, lo
  ejecuta con `liradb script guion.ql` y con `script -` por pipe, y lee el
  error «línea N: …»; (3) testea REPL y scripts con `cli_con_entrada`
  (buffers, sin spawn) al estilo de los 34 tests.
- **Wisdom**: (1) decide cuándo un frontal debe continuar tras un error
  (REPL: sesión humana) y cuándo morir con nº de línea (script:
  automatización, fail fast) — y por qué la decisión correcta es factorizar
  el intérprete, no duplicarlo; (2) decide cuándo una ayuda autogenerada
  basta y cuándo una ayuda curada es parte del producto (ejemplos
  copiables, grafo demo) — con su coste de mantenimiento a mano.

## 4. Modelo mental

- **El quiosco frente al banco**: durante 30 capítulos construimos el banco
  (motor: Value, GraphStore, pager, WAL, MVCC…) y la única ventanilla era
  el código de los tests. La CLI es el QUIOSCO que se levanta frente al
  banco: un CLIENTE del motor hexagonal que consume sus puertos (`&dyn
  GraphStore` para consultar, `&mut` para escribir) sin conocer sus tripas.
  Y el quiosco tiene UN mostrador — `sesion::interpretar_linea` — con DOS
  ventanillas: la del REPL (charla, pone carteles `liradb>` y `tx>`,
  perdona errores) y la del guion (despacha lotes en silencio y cierra al
  primer problema). Mientras una transacción vive, el despacho del contable
  (cap. 27) está cerrado con el candado `&mut`: la ventanilla lo PONE POR
  CARTEL — el prompt cambia a `tx>` y las consultas se rechazan con la
  explicación. **El prompt `tx>` es el borrow checker traducido a texto.**
- **Diagramas ASCII**: (a) la CLI como cliente del motor hexagonal (dos
  frontales → intérprete compartido → Sesion/store → puertos vol2_liradb),
  con `run_con_entrada` como única frontera testable; (b) el sub-bucle de
  transacción (`:begin` abre, prompt `tx>`, sólo staging, `:commit`/
  `:rollback`/`:quit`/EOF la CONSUMEN y el bucle termina); (c) la tabla de
  políticas REPL vs script (prompts, errores, EOF, exit).
- **Momento ¡ajá!**: «REPL y script no son dos productos: es UN intérprete
  con dos POLÍTICAS de error. Y la transacción no se puede "guardar en la
  sesión" porque el cap. 27 ya decidió, en los tipos, que la tx ES el
  préstamo del store — el diseño de la API de hace cuatro capítulos decide
  hoy la forma de mi bucle».

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`lib.rs` / `sesion.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | **clap Builder API** (`comando()`, sin macros) | El árbol de comandos a la vista: cada subcomando/flag es una línea de Rust que el novato puede LEER antes de saber macros. La Derive API (más ergonómica, la de producción) se contrasta en prosa | `#[derive(Parser)]`: más corto, pero esconde el árbol tras atributos y exige entender macros derive antes que entender CLIs | El lector usa `#[derive]` como incienso: copia atributos sin saber qué árbol generan | `comando()` (lib.rs 87-143); doc de la fn; MIGRATION §36 decisión 1; docs de clap (Builder vs Derive) |
| 2 | `clap = "4.5"` como PRIMERA dependencia externa de la CLI | El pago de la regla del Vol.II «primero a mano, luego con crates»: el hito (§25) parseó argv a mano DELIBERADAMENTE; ahora que el lector sabe qué es parsear, clap es ganancia visible (validación, defaults, errores con alternativas) | Seguir a mano: cada flag nuevo (`--plan`, `--stats`, `--graph`) multiplicaría ramas y los mensajes de error serían artesanales | CLI rígida que crece a golpe de `match` y errores de uso inconsistentes | Cargo.toml (clap 4.5, resuelto 4.6.6 pineado por Cargo.lock); §25 decisión 2 vs §36 |
| 3 | Ayuda CURADA (`imprimir_ayuda`) con `disable_help_flag/subcommand` y atajo manual de `help/--help/-h` ANTES de clap | La ayuda es la PORTADA del producto: ejemplos copiables (`printf … \| liradb script -`), el GRAFO DEMO documentado y el gancho al cap. 32 no los genera clap. «Sin args = ayuda + exit 0» conserva la semántica del hito y sus tests | La ayuda autogenerada de clap: buena para el árbol, muda sobre el producto; con `-h` de clap habría DOS ayudas divergentes | Dos ayudas que cuentan historias distintas; `liradb` a secas dejaría de enseñar el camino | lib.rs 87-92, 168-177, 470-504; tests `sin_args_muestra_ayuda_y_sale_0`, `help_explicito_muestra_ayuda_y_sale_0` |
| 4 | Errores de uso SÍ de clap, con `Error::render()` al writer INYECTADO + `use_stderr()` + `exit_code()` | La validación de argv es trabajo de máquina (y clap la hace mejor: «invalid value 'nada' … [possible values: demo, empty]»), pero IMPRIMIRLA a stderr real la haría intestable. `render()` al writer inyectado conserva la prueba sin procesos; `use_stderr()` distingue `--version` (stdout, exit 0) de usage (stderr, exit 2) | Dejar que clap haga `exit()` él solo (su default): mensajes correctos pero NO testables — el test moriría con el proceso | Tests de uso que exigen spawn o no existen; exit codes de uso sin verificar | lib.rs 183-201; tests `subcomando_desconocido_es_error_de_uso_de_clap`, `graph_invalido_es_error_de_uso_de_clap` |
| 5 | `run_con_entrada(args, &mut dyn Read, out, err) -> i32` | La lección del hito (salida inyectada) extendida a la ENTRADA: el REPL y `script -` leen del `dyn Read` (stdin en producción, `Cursor` en los tests). REPL y guiones testeables sin TTY ni spawn; `run()` se conserva delegando con stdin real | Leer stdin global dentro de la lib: el REPL dejaría de ser testable (leería del proceso de test); spawn del binario: lento y frágil | «El REPL no se puede testear» y 20 de los 34 tests no existen | lib.rs 160-207; helper `cli_con_entrada`; MIGRATION §36 decisión 2 |
| 6 | `main.rs` fino: argv + stdin + writers y `std::process::exit(codigo)` | Lo que no se puede testar no debe crecer: main es frontera de proceso (argv, fd 0/1/2), todo lo demás vive en la lib | Lógica en main: duplicarla en los tests o testearla con spawn | CLI cuyo comportamiento real difiere del testado | main.rs completo (23 líneas); §25 decisión 4 |
| 7 | **Dos frontales, UN intérprete** (`sesion::interpretar_linea` + `enum Accion`) | REPL y script comparten parse de meta-comandos, escrituras autocommit, transacciones y consultas: cada meta-comando funciona gratis en ambos. La DIFERENCIA (prompts, política de errores) vive en el driver: `Err(String)` sale SIN imprimir y cada frontal lo reporta a su manera | Dos bucles parecidos (uno en cmd_repl, otro en cmd_script): el doble de código y el clásico bug del meta-comando que funciona en uno y no en el otro | Divergencia silenciosa entre REPL y script — el guion que «no hace lo mismo que escribí a mano» | sesion.rs 89-116; lib.rs 378-411 y 421-464; MIGRATION §36 lección 1 |
| 8 | REPL CONTINÚA tras errores; script DETIENE en el primero con `línea N:` y exit 1 | Dos audiencias: el humano explora (un error es información; la sesión sigue) y el automatismo debe confiar (un guion que «sigue» tras un error ejecuta mitad del plan sin que nadie lo note). Es el fail fast de Unix («Repair what you can — but when you must fail, fail noisily and as soon as possible», Raymond, TAOUP); el nº de línea es el contexto mínimo para reparar | Continuar también en script (o parar también en REPL): el primero ejecuta planes a medias; el segundo convierte cada errata en adiós | `liradb script despliegue.ql` que «termina OK» habiendo saltado la mitad de las operaciones | sesion.rs 77-88; lib.rs 455-463; tests `repl_los_errores_no_cortan_la_sesion`, `script_detiene_en_el_primer_error_con_linea` |
| 9 | `script -` lee stdin; comentarios `#`/`--` y vacías se saltan | Pipe-friendly (`printf '…' \| liradb script -`): el guion se compone con el ecosistema Unix (heredocs, generate, ssh). Los comentarios hacen al guion LEGIBLE y auto-documentado; son válidos también en REPL (misma línea de código) | Sólo ficheros como argumento: sin pipes ni docs embebidas; comentarios sólo en script: dos gramáticas para recordar | Guiones ilegibles de 40 líneas que nadie se atreve a tocar | lib.rs 429-441; sesion.rs 97-101; test `script_por_stdin_con_comentarios_y_multiples_consultas` |
| 10 | El `;` final se tolera (`trim_end_matches(';')`) | El reflejo SQL/psql es universal: castigar con «error de sintaxis» una consulta perfecta por un punto y coma de más es pelearse con la memoria muscular del usuario. Tolerar ES la decisión ergonómica; documentarlo ES la decisión honesta | Rechazarlo con error de sintaxis: «correcto» según la gramática LiraQL, hostil según el producto | Usuarios que abandonan el REPL en el tercer `;` | sesion.rs 106-108; test `repl_tolerar_el_punto_y_coma_final` |
| 11 | `Sesion { store: MemoryStore }` nace de `--graph demo|empty` con `value_parser(["demo","empty"])` | La sesión es el grafo VIVO entre líneas — lo que el hito no tenía (cada `query` reconstruía el demo y lo tiraba). El `value_parser` hace que el error de uso LISTE las alternativas («[possible values: demo, empty]») en vez de fallar en runtime con «origen desconocido» | Un `String` y comprobar después: el error sería de consulta (exit 1, mensajes artesanales) en vez de uso (exit 2, con soluciones) | `--graph vacio` (errata) reportado tarde y sin alternativas | lib.rs 146-153, 229-235; sesion.rs 44-66; tests `graph_invalido_es_error_de_uso_de_clap`, `repl_graph_empty_arranca_vacio` |
| 12 | Las escrituras usan el cap. 27 DE VERDAD: `autocommit()` fuera de tx; `:begin` abre una `Transaccion` real | La CLI no se inventa su semántica de escritura: cada `:node` es su propia transacción (el modo por defecto de los caps. 7-26, HECHO VISIBLE en el mensaje «autocommit»), y `:begin` es la tx real con staging y validación eager. Reutilizar el motor ES la lección hexagonal: el cliente consume puertos, no reimplementa | Atajos directos a `store.put_node` en la CLI: dos semánticas de escritura (la del motor y la de la shell), mensajes que mienten sobre lo que pasó | La CLI promete transacciones pero usa puts sueltos: el resumen «commit: N operaciones» sería teatro | sesion.rs 304-349, 363-384; tests `repl_construye_y_consulta_su_propio_grafo`, `repl_transaccion_commit_persiste` |
| 13 | La tx se pasa **POR VALOR** al sub-bucle (`bucle_transaccion(mut tx: Transaccion)`) | E0507 ×4 («cannot move out of `*tx`») enseñó que `commit`/`rollback` CONSUMEN `self` (el ciclo de vida vive en los tipos, cap. 27): con `&mut Transaccion` el bucle no podía cerrarla. Por valor, el bucle termina EXACTAMENTE cuando la tx se consume — el tipo FUERZA el diseño. Y `commit` con Err también la consume: Err = tx descartada, sesión continúa | `&mut Transaccion` + un flag `cerrada: bool`: compila peor (E0507) y reintroduce el estado «muerta» en runtime que el cap. 27 desterró; guardar la tx EN la sesión: guerra de lifetimes (la sesión pide `&mut` para consultar cuando la tx lo tiene prestado) | El REPL pelea con el borrow checker O la tx se vuelve opcional/nullable y todos los caminos comprueban «¿está viva?» | sesion.rs 154-160; MIGRATION §36 bugs (E0507) y lección 2; tests `repl_transaccion_commit_persiste`, `repl_tx_staging_invalido_no_mata_la_tx` |
| 14 | Con la tx viva: consultas y `:demo`/`:clear`/`:graph` se RECHAZAN («el store está prestado»); sólo staging + `:commit`/`:rollback`/`:quit`/`:help`; EOF y `:quit` = rollback implícito | El préstamo exclusivo del cap. 27, hecho PRODUCTO: el usuario ve en el prompt `tx>` y en el mensaje el mismo «un único escritor» que el borrow checker exigía al código. EOF/:quit = el drop implícito del cap. 27 (nada se aplicó sin commit); en guion, `:quit` corta el guión con exit 0 | Permitir consultas durante la tx «leyendo por debajo»: imposible con `&mut` prestado (y sería lectura a medias); commitear al EOF: convertiría un Ctrl+D en un commit fantasma | El usuario crea datos «dentro de una tx», cierra la ventana y los da por confirmados — o el REPL pánico | sesion.rs 37, 161-264; tests `repl_transaccion_las_consultas_esperan`, `repl_transaccion_quit_es_rollback_implicito`, `script_quit_corta_el_guion_y_sale_0` |
| 15 | `parse_valor` hacia `Value` del cap. 7 (comillas → true/false/null → i64 → f64 → error) | El tipado del modelo, otra vez: la línea de mandos del usuario entra como TEXTO y sale como el `Value` del cap. 7 — el mismo enum que viajó por encoding (9), páginas (11) y WAL (28). El orden del match es la semántica: `42` es Int, `3.5` Float, `"hola"` String; lo demás, error con formato esperado | Un `Value::String` comodín (todo lo no reconocido es texto): los `WHERE p.age < 40` fallarían en runtime con «tipos incompatibles» en vez de en el parseo del meta-comando | Grafos «correctos» cuyas consultas numéricas fallan por props que eran Strings disfrazadas | sesion.rs 456-476; test `repl_props_de_todos_los_tipos_y_errores_de_formato`; cap. 7 |
| 16 | Exit codes 0/1/2 y stdout (datos) vs stderr (diagnósticos) | El contrato Unix para composición: 0 = todo bien (incluye `--version` y la ayuda); 1 = la CONSULTA o el FICHERO fallaron (mensaje a stderr); 2 = USO incorrecto (clap). La tabla de datos SIEMPRE por stdout, los errores SIEMPRE por stderr — así `liradb query … \| wc -l` cuenta filas y no prompts, y `2>/dev/null` silencia diagnósticos sin tocar datos | Todo a stdout y exit 0/-1/println: los pipes y los CI se vuelven mentira (recuentos con errores dentro, verdes que eran rojos) | Scripts CI que creen verde una consulta fallida; pipes que mezclan tabla con errores | lib.rs 48-53, 183-201; tests `query_error_de_parse_a_stderr_y_exit_1`, `query_match_simple_imprime_la_tabla`; GNU Bash Reference Manual («Exit Status»), POSIX (stderr no tamponado) |
| 17 | `emitir` ignora fallos de E/S (`let _ = write_all`) | Una CLI didáctica no entra en pánico si el consumidor cierra la tubería antes: `liradb demo \| head -1` debe salir limpia, no con un broken pipe asustadizo | `unwrap()`: el pánico por una tubería cerrada parece bug del motor y no lo es | «liradb se cae con head» reportado como bug de la BD | lib.rs 506-511; sesion.rs 478-482; §25 decisión 6 |
| 18 | `query --plan/--stats` reutilizan `pipeline_con_detalle` (el desenrollado de `run` que `demo` ya hacía) | Una sola función parametrizada sirve a `demo` (siempre con todo) y a `query` (a la carta); sin flags el comportamiento del hito queda INTACTO y un test lo clava | Duplicar el pipeline en cada comando: el plan de `demo` y el de `query --plan` divergen sin que nadie lo note | Dos planes que «no dicen lo mismo» según el subcomando | lib.rs 340-370; tests `query_con_plan_y_stats_a_la_carta`, `demo_ejecuta_las_cuatro_consultas_con_plan_y_tabla` |

## 6. Primera solución vs solución evolucionada

- **Ingenua (el hito del §25, conservada)**: `run(args, out, err)` con un
  `match` MANUAL sobre `std::env::args`, tres subcomandos (`demo`,
  `query "<LiraQL>"`, `help`/sin args), ayuda literal, exit codes 0/1/2 ya
  Unix, `emitir` tolerante y 11 tests end-to-end con `Vec<u8>`. Sin estado:
  cada `query` reconstruye `demo_graph()` y lo tira. Correcto para
  «demostrable desde shell a mitad del libro» (ADR-005) — y congelado a
  propósito: REPL, flags e import quedaban «todo cap. 31».
- **Qué la rompe**: (a) sin SESIÓN, no hay exploración: cada consulta nace
  y muere con su grafo; (b) sin ESCRITURAS, el motor es de sólo lectura y
  los caps. 27-30 no tienen cara visible; (c) sin GUIONES, nada se
  automatiza ni se comparte (el «cómo lo reproduzco yo» de un bug); (d)
  sin flags, `--plan`/`--stats` exigen elegir SIEMPRE la vista verbosa del
  `demo`; (e) el parseo manual no escala: `--graph` con validación y
  alternativas, aridad de `query`, mensajes de uso — cada detalle es otra
  rama artesanal.
- **Evolución visible**: clap Builder define el árbol y valida el uso
  (errores con alternativas, exit 2); `run_con_entrada` añade la ENTRADA
  inyectada a la testabilidad del hito; `Sesion` mantiene el grafo vivo
  entre líneas; `interpretar_linea` es el intérprete compartido de REPL y
  script; `:begin` abre una `Transaccion` REAL en un sub-bucle con prompt
  `tx>`; `--plan`/`--stats` desenrollan el pipeline a la carta. La CLI pasa
  de utilidad de demostración a CLIENTE COMPLETO del motor (34 tests:
  11 conservados + 23 nuevos).

## 7. Prueba de fuego

- **TEST-TESIS A** `repl_construye_y_consulta_su_propio_grafo`: la CLI como
  cliente COMPLETO del motor — un usuario construye Zoe/Oviedo con `:node`/
  `:edge` (3 autocommits visibles) y su consulta LiraQL lo ve TODO. Sin
  ficheros, sin importaciones: sesión pura.
- **TEST-TESIS B** `repl_transaccion_las_consultas_esperan`: con la tx
  abierta, la consulta LiraQL y `:demo` se rechazan con «el store está
  prestado» (DOS rechazos a stderr) y el `:commit` final dice «0
  operaciones» — el préstamo exclusivo del cap. 27, hecho producto.
- **TEST-TESIS C** `script_detiene_en_el_primer_error_con_linea`: el guion
  responde a la 1ª consulta, muere en la 2ª con «línea 2:» y la 3ª JAMÁS
  se ejecuta — fail fast con contexto, exit 1.
- **TEST-TESIS D** `repl_y_query_y_script_vuelven_el_mismo_dato`: tres
  frontales (query, REPL, script), un motor: la consulta canónica del brief
  devuelve Ana/Carla/Dani (y no Bo) por los tres caminos.
- Otros 30 tests (todos en `mod tests` de `lib.rs`, citados por nombre en el
  capítulo), por grupo: 7 del hito conservado (`sin_args_muestra_ayuda_y_sale_0`,
  `help_explicito_muestra_ayuda_y_sale_0`, `version_imprime_la_version_y_sale_0`,
  `demo_ejecuta_las_cuatro_consultas_con_plan_y_tabla`,
  `query_match_simple_imprime_la_tabla`,
  `query_error_de_parse_a_stderr_y_exit_1`,
  `query_error_runtime_de_tipos_a_stderr_y_exit_1`); 3 de uso de clap
  (`subcomando_desconocido_es_error_de_uso_de_clap`,
  `query_sin_consulta_es_error_de_uso_de_clap`,
  `graph_invalido_es_error_de_uso_de_clap`); 1 de flags
  (`query_con_plan_y_stats_a_la_carta`); 9 de sesión (`repl_consulta_y_quit`,
  `repl_eof_es_salida_limpia`, `repl_graph_empty_arranca_vacio`,
  `repl_los_errores_no_cortan_la_sesion`,
  `repl_props_de_todos_los_tipos_y_errores_de_formato`,
  `repl_demo_recarga_y_clear_vacia`, `repl_borra_con_cascada_visible`,
  `repl_meta_desconocido_se_reporta`, `repl_tolerar_el_punto_y_coma_final`);
  5 de transacciones (`repl_transaccion_commit_persiste`,
  `repl_transaccion_rollback_descarta`,
  `repl_transaccion_quit_es_rollback_implicito`,
  `repl_commit_sin_tx_abierta_se_rechaza`,
  `repl_tx_staging_invalido_no_mata_la_tx`); 5 de script
  (`script_por_stdin_con_comentarios_y_multiples_consultas`,
  `script_desde_fichero`, `script_fichero_inexistente_exit_1`,
  `script_quit_corta_el_guion_y_sale_0`, `script_con_transaccion`).
- **Síntoma si el lector se salta el capítulo**: llega al cap. 32 sin
  intérprete al que enchufar import/export; a los caps. 33-34 (pruebas,
  benchmarks) sin forma de ejercitar el motor end-to-end desde fuera; y su
  «producto» sigue siendo una librería con ejemplos — útil para quien
  programa contra ella, invisible para quien la usa.

## 8. Trampas y errores comunes

1. **Guardar la tx en la sesión** («para retomarla en la siguiente línea»):
   la sesión necesita `&mut store` para consultar y la tx LO TIENE prestado
   — guerra de lifetimes. La forma que no pelea con los tipos es el
   SUB-BUCLE por valor (E0507 fue el aviso, §36). Síntoma: E0507 o un
   `Option<Transaccion>` con comprobaciones «¿vive?» por todas partes.
2. **`commit` con Err y querer `continue` usando la tx**: `commit(self)`
   la consume ACIERTE O FALLE (cap. 27): con Err la transacción queda
   DESCARTADA y nada se aplicó; la sesión continúa sin ella. Síntoma:
   «use of moved value: `tx`».
3. **Testear el binario compilado (o culpar al código cuando el binario
   «no se reconstruye»)**: los tests van contra `run_con_entrada`, sin
   spawn. Y si el smoke test del binario no cambia… comprobar DÓNDE compila
   realmente: un `target-dir` global en `~/.cargo/config.toml` (compartido
   entre instancias) deja un `target/` local con rmeta de 0 bytes — un
   BINARIO FANTASMA (§36). Síntoma: candados de ficheros y «mis cambios no
   salen».
4. **Imprimir errores de uso a stderr real** (dejar que clap haga `exit()`
   solo): mensajes correctos pero NO testables. `Error::render()` al writer
   inyectado. Síntoma: tests de uso que sólo pasan como subprocesos.
5. **Confundir prompt con salida / datos con diagnóstico**: los prompts del
   REPL van a stdout (el REPL ES conversacional), pero la tabla de datos y
   los errores NUNCA se mezclan: datos→stdout, errores→stderr, o los pipes
   mienten. Síntoma: `liradb query … | wc -l` devuelve 6 en vez de 4.
- **Precisión de lenguaje (glosario)**: *REPL* (bucle interactivo con
  sesión) vs *script mode* (lotes no interactivos); *frontal/driver* (el
  bucle que lee: REPL o script) vs *intérprete* (`interpretar_linea`, el
  código compartido); *sesión* (el grafo vivo `Sesion`) vs *transacción*
  (el préstamo con staging del cap. 27); *meta-comando* (empieza por `:`)
  vs *consulta LiraQL* (no); *exit code* (0/1/2) vs *código de error del
  motor* (`ExecError`/`StoreError`); *stdout* (datos) vs *stderr*
  (diagnósticos); *autocommit* (cada escritura, su tx) vs *tx explícita*
  (`:begin`…`:commit`); *ayuda curada* (la del libro) vs *ayuda generada*
  (la de clap); *staging* (anotar sin aplicar) vs *apply* (escribir al
  store en el commit).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval practice)**: de MEMORIA (sin
  mirar `:help` ni el código), (a) reconstruye el árbol de meta-comandos
  legales FUERA de transacción (los 11 + qué pasa con `:commit` sin tx y
  con `:magic`); (b) lista los legales DENTRO de una tx y qué se rechaza;
  (c) predice la SALIDA EXACTA (stdout, stderr y exit code) de la sesión
  `:begin` · `:node 0:Person name="Sol"` · `MATCH (p) RETURN p.name` ·
  `:commit` · `:quit` sobre `--graph empty`. Pistas: (1) ¿qué prompt
  acompaña a cada línea?; (2) ¿dónde va el rechazo de la consulta?;
  (3) ¿qué dice el resumen del commit? Verificación:
  `repl_transaccion_las_consultas_esperan`, `repl_transaccion_commit_persiste`
  y el binario real pegando la sesión. Criterio: árbol completo de memoria
  + salida predicha byte a byte (prompts incluidos).
- **analizar (intermedio — interleaving con el cap. 27)**: sobre
  `--graph empty`, la sesión `:begin` · `:node 0:Person name="Efímero"` ·
  `:node 1:Person name="Fija"` · `:quit`. (a) ¿Qué línea imprime el REPL
  antes de salir y con cuántas operaciones descartadas? (b) ¿Qué queda en
  el store y con qué exit code termina? (c) ¿En qué se diferencia este
  rollback implícito del `Drop` del cap. 27 — y por qué aquí NO es «gratis
  por construcción» sino DECISIÓN del driver? (d) ¿Por qué la consulta
  durante la tx se RECHAZA en vez de ser una `LecturaSucia` posible?
  Pistas: (1) ¿quién consume la tx en `bucle_transaccion`?; (2) EOF y
  `:quit` comparten camino; (3) ¿qué préstamo retiene la tx mientras
  vive? Verificación: `repl_transaccion_quit_es_rollback_implicito`,
  `repl_transaccion_rollback_descarta`. Criterio: razonar el driver como
  traductor de las garantías del cap. 27.
- **crear (experto — spacing con los caps. 7 y 27)**: escribe el guion
  `tipos.ql` que (1) vacíe la sesión, (2) cree en UNA transacción el nodo
  `0:Prop valor=42 otro=3.5 flag=true nada=null texto="hola"` y una arista
  con `desde=2019`, con un staging INVÁLIDO en medio (`:edge 9 9 9 KNOWS`)
  que deba expulsarse sin matar la tx, (3) commitee y (4) consulte las
  props para demostrar que `42` llegó como `Int`, `3.5` como `Float` y
  `2019` como `Int` (¿por qué no Float?). Ejecútalo con `liradb script
  tipos.ql` y con `cat tipos.ql | liradb script -`. Pistas: (1) ¿qué
  imprime un staging aceptado y cuál rechazado, y dónde va cada uno?;
  (2) el orden de `parse_valor`: comillas → bool/null → i64 → f64;
  (3) ¿la tx sigue viva tras «staging rechazado» (cap. 27)? Verificación:
  `repl_props_de_todos_los_tipos_y_errores_de_formato`,
  `repl_tx_staging_invalido_no_mata_la_tx`, `script_con_transaccion`.
  Criterio: guion verde por las DOS vías + explicar por qué el orden del
  match decide el tipo final.

## 10. Preguntas abiertas (gancho al cap. 32 — import/export)

1. La sesión de este capítulo vive en RAM y nace de `--graph demo|empty`:
   ¿de dónde sale el grafo de un usuario REAL con 10 millones de aristas?
   Meterlo a base de `:node` en un guion es posible (este intérprete lo
   permite) pero absurdo a esa escala — hacen falta LECTORES de formatos
   (CSV, JSONL, GraphML) y una política de LOTES por transacción.
2. `liradb import datos.csv` — ¿subcomando nuevo de clap o meta-comando
   `:import` del intérprete? ¿Qué reutiliza de `Sesion`/
   `interpretar_linea` y qué NO (el streaming de un fichero grande no
   cabe en la metáfora línea a línea)?
3. Los tres formatos comparten la MISMA pregunta que `parse_valor` respondió
   para una línea: ¿cómo se mapea el texto del formato al `Value` del
   cap. 7 y quién valida? ¿Y qué pasa con el `export` — la tabla del
   `ResultSet` (cap. 20) escrita a CSV/JSONL?
- **Términos nuevos de glosario**: REPL, script mode (`-` = stdin),
  sesión, meta-comando, intérprete compartido, frontal/driver, política de
  errores, prompt (`liradb>`/`tx>`), exit code (0/1/2), stdout vs stderr,
  Builder API, `value_parser`, fail fast, rollback implícito (EOF/:quit),
  ayuda curada, entrada inyectada (`run_con_entrada`), binario fantasma
  (target-dir global).

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial reconstruye DE MEMORIA el árbol de
  meta-comandos (fuera y dentro de tx) y predice la salida exacta de una
  sesión — prompts, rechazos, resumen de commit, exit code — sin pistas
  que lo regalen; recordarlo (el árbol, la política) vale más que
  reconocerlo en el `:help`.
- **Spacing**: el hito §25 (exit codes y stdout/stderr — la tabla del
  `query` sin prompts ya era la disciplina), el `Value` del cap. 7
  (destino de `parse_valor` y su orden de match), la `Transaccion`/drop
  implícito del cap. 27 (el prompt `tx>` y el rollback por EOF), el
  `ResultSet` del cap. 20 y el `explain` del 21 (`--plan`/`--stats`), la
  cascada de `delete_node` del cap. 8 (`repl_borra_con_cascada_visible`).
- **Interleaving**: el intermedio mezcla el driver del cap. 31 con las
  garantías del 27 (`:quit` dentro de `:begin`, anomalías imposibles);
  el experto mezcla la gramática de meta-comandos con el sistema de tipos
  del 7 y el staging eager del 27.
- **Dificultad asimétrica**: una idea nueva por sección (entregar el
  motor → sesión e intérprete compartido → REPL/script como políticas →
  transacción visible → parseo de valores → exit codes); los ejercicios
  exigen reconstrucción y predicción, no lectura.
- **Bucle de feedback inmediato**: `cargo test -p liradb-cli` (34 tests,
  todos contra `run_con_entrada`, en milisegundos) y el binario real
  (`cargo run -p liradb-cli -- repl`) para ver la sesión con sus prompts.
- **Citas**: McCarthy, «History of LISP» (1979) y McCarthy et al., «LISP
  1.5 Programmer's Manual» (MIT Press, 1962) — el REPL como tradición;
  Kernighan & Pike, «The Unix Programming Environment» (Prentice Hall,
  1984) — la shell como intérprete dual interactivo/guion; Eric S.
  Raymond, «The Art of Unix Programming» (Addison-Wesley, 2003) — Rule of
  Repair (fail fast) y la disciplina stdout/stderr; GNU Bash Reference
  Manual («Exit Status»: 0/1/2); POSIX (stdout tamponado / stderr sin
  tamponar); documentación oficial de clap 4 (Builder vs Derive; exit 2
  para errores de uso); docs de PostgreSQL (psql y sus backslash-commands),
  SQLite (dot-commands de sqlite3) y Neo4j (cypher-shell y sus `:`);
  MIGRATION-PATTERN §25 y §36 como prosa verificable.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada
  y fuente (18 en la tabla §5).
- [x] Escenario de fallo visible: el script muere en la 1ª línea mala con
  «línea N:» (test C), las consultas esperan durante la tx (test B) y los
  tres errores de uso de clap están testeados con exit 2.
- [x] Código ejecutable en workspace (34 tests ALL_GREEN, 728→751 en el
  workspace — MIGRATION §36) citado por nombre y módulo, no duplicado.
- [x] Misconcepciones corregidas explícitamente (§1: siete, de «la CLI es
  un detalle» a «EOF da igual»).
- [x] Ejercicios con solución verificable (tests del workspace + el binario
  real pegado por stdin).
- [x] ≥1 ejercicio de retrieval (árbol de meta-comandos + sesión predicha
  de memoria) y ≥1 de spacing (hitos §25, caps. 7/8/20/21/27 tocados).
- [x] Responde la pregunta crítica del CORPUS («REPL vs script mode; clap
  ergonomics») y abre la Parte VII.
- [x] Anécdota verificada con fuentes de alta confianza (LISP 1.5 1962,
  McCarthy 1979; la shell Unix; psql/cypher-shell; clap docs).
- [x] Gancho explícito al cap. 32 (import/export que se enchufa a ESTE
  intérprete) y delimitación (el 31 es la SHELL; el 32, los FORMATOS).
- [x] Bugs reales del §36 contados como lecciones (E0507, el binario
  fantasma del target-dir global, el commit-Err que consume).
