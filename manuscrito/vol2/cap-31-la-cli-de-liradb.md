# Capítulo 31 — La CLI de LiraDB

> *«Treinta capítulos construyendo el motor. Ninguno preguntando cómo se entrega. La CLI no es la utilidad que sobra: es el producto — y el único sitio donde todo lo que construiste se ve desde fuera.»*

## 31.0 La anécdota de la esquina

Cuenta John McCarthy en «History of LISP» (1979) que el sistema Lisp de MIT, corrido en un IBM 704 a finales de los cincuenta, se usaba de una forma que hoy nos parece obvia y entonces era una rareza: el programador se sentaba ante un terminal, tecleaba una expresión, y la máquina respondía con su valor. Tecleaba la siguiente, respondía. Y otra. Ese ciclo — leer, evaluar, imprimir, repetir — quedó documentado en el «LISP 1.5 Programmer's Manual» (MIT Press, 1962) y acabó teniendo nombre propio: **REPL**, *read–eval–print loop*. La interacción no era un accesorio del lenguaje: era *la* forma de programarlo. Escribías una función, la probabas al instante, la corregías, seguías. El programa y la conversación eran la misma cosa.

La segunda mitad de la historia la escribió Unix. La shell de Thompson (con la primera versión de Unix, 1971) y luego la de Bourne tenían una propiedad que Kernighan y Pike celebraban en «The Unix Programming Environment» (1984): **el mismo intérprete** que te hablaba por el terminal servía para ejecutar un fichero de órdenes. No había «el shell interactivo» y «el lenguaje de scripts»: había un intérprete y dos maneras de alimentarlo. Por eso los gestos que aprendías tecleando servían, idénticos, dentro de un guion.

Las bases de datos heredaron ambas tradiciones. `psql` de PostgreSQL es un REPL con meta-comandos de contrabarra (`\d`, `\q`); `sqlite3` tiene sus *dot-commands* (`.help`, `.quit`); `redis-cli` y `cypher-shell` de Neo4j (con sus órdenes `:` ) son lo mismo con otro signo. Cuando un usuario conoce una base de datos, la conoce **desde su prompt**.

Este capítulo abre la Parte VII: convertir el proyecto en un producto técnico. LiraDB tiene motor, lenguaje, optimizador, WAL y MVCC. Ahora necesita su prompt.

## 31.1 Objetivo

Al terminar este capítulo tendrás la CLI completa de LiraDB: el binario `liradb` con cinco subcomandos (`demo`, `query`, `explain`, `repl`, `script`), un REPL con sesión persistente y transacciones reales, y un modo guion que se ejecuta desde fichero o desde un pipe. El código vive en la crate `liradb-workspace/crates/vol2-liradb-cli` — `src/lib.rs` (1.035 líneas, **34 tests**), `src/sesion.rs` (482 líneas, el intérprete compartido) y un `src/main.rs` de 23 líneas que no hace nada más que arrancar.

Y una tesis que lo vertebra: **la CLI no es una utilidad menor — es el producto**. Cada pieza de los capítulos 7-30 encuentra aquí su cara visible: el grafo demo del cap. 20 en `liradb demo`, el optimizador del 21 en `liradb explain`, la tabla del `ResultSet` en cada consulta, el `Value` del cap. 7 en el parseo de propiedades, y — lo más bonito — la `Transaccion` del cap. 27 hecha prompt: mientras vive, el prompt dice `tx>` y las consultas esperan, porque el store está prestado.

## 31.2 Problema

En realidad, ya tenías una CLI. Tras el cap. 20 plantamos un hito deliberadamente mínimo (MIGRATION-PATTERN §25): un binario `liradb` con `demo`, `query "<LiraQL>"` y `help`, el argv parseado **a mano** con un `match`, y once tests end-to-end. Servía para su propósito — demostrar el motor desde una shell a mitad del libro — y congeló a propósito todo lo demás: «REPL, import, flags: todo cap. 31».

Míralo ahora con ojos de usuario. Es demostrable pero **rígido**:

1. **Sin sesión**. Cada `liradb query` reconstruye el grafo demo, responde y lo tira. No puedes mirar un dato, pensar, y mirar otro: cada consulta nace huérfana y muere sola.
2. **Sin escrituras**. El motor sabe escribir (`put_node`, transacciones del cap. 27, WAL del 28), pero desde la CLI sólo puedes leer el mismo grafo de cuatro personas y dos ciudades. Para el usuario, LiraDB es de sólo lectura.
3. **Sin guiones**. Nada de ficheros `.ql`, nada de pipes, nada de «ejecuta esto y guárdalo para reproducir el bug». Lo interactivo y lo automatizable eran lo mismo y era poco.
4. **Sin flags ni validación de uso**. Querías el plan sin ejecutar, o las métricas sin el plan: imposible. Y un `--graph` mal escrito habría caído en un error artesanal en vez de en uno que te dice las alternativas.

El problema de fondo: **¿cómo se ENTREGA un motor?** Un motor sin forma de usarlo es una librería con ejemplos. La pregunta crítica que el CORPUS clavó para este capítulo es exactamente ésa, en dos partes: *REPL vs script mode* (¿son dos productos?) y *clap ergonomics* (¿cuándo pagar una crate para parsear argumentos?).

## 31.3 Modelo mental

Piensa en un **quiosco levantado frente al banco**. Durante treinta capítulos construimos el banco — el motor hexagonal: `Value`, `GraphStore`, pager, WAL, MVCC. La CLI es el quiosco de la calle: un **cliente** que consume los puertos del banco (`&dyn GraphStore` para consultar, `&mut` para escribir) sin conocer sus tripas. Y el quiosco tiene un mostrador único con dos ventanillas:

```
        ┌───────────────────────── liradb (la CLI) ─────────────────────────┐
        │  run_con_entrada(args, &mut dyn Read, out, err) ──► exit 0 | 1 | 2 │
        │        │                                                            │
        │   dos VENTANILLAS, UN mostrador (sesion::interpretar_linea)         │
        │        │                                                            │
        │   ┌────┴─────────────┐   ┌──────────────────────────┐               │
        │   │ REPL (cmd_repl)  │   │ script (cmd_script)      │               │
        │   │ prompts liradb>/ │   │ sin prompts              │               │
        │   │ tx>              │   │ errores: «línea N:» EXIT │               │
        │   │ errores: sigue   │   │ '-' = stdin (pipes)      │               │
        │   │ EOF = :quit      │   │ EOF = fin                │               │
        │   └────┬─────────────┘   └──────────┬───────────────┘               │
        │        └────────────┬───────────────┘                               │
        │           Sesion { store } ──► consulta LiraQL: vol2_liradb::run    │
        │                       │────► :node/:edge/:del-*: autocommit (c. 27) │
        │                       └────► :begin ─► sub-bucle tx> (Transaccion)  │
        └─────────────────────────────────────────────────────────────────────┘
                                  │ los puertos del cap. 8
                                  ▼
                        &dyn / &mut dyn GraphStore
```

Tres ideas ordenan el dibujo:

1. **Dos frontales, un intérprete.** REPL y script no son dos programas: comparten `sesion::interpretar_linea` — el mostrador — y difieren sólo en **política**: prompts, y qué hacer con un error. Es la lección de la shell Unix: un intérprete, dos alimentaciones.
2. **El prompt `tx>` es el préstamo exclusivo hecho visible.** En el cap. 27, mientras una `Transaccion` vivía, el borrow checker impedía tocar el store. Aquí el usuario lo *ve*: el prompt cambia a `tx>` y una consulta responde «el store está prestado». El candado de la puerta del despacho, traducido a cartel.
3. **La frontera es testable.** Todo lo que la CLI hace entra por `run_con_entrada(args, entrada, out, err)` y sale por un `i32`. Stdin, stdout y stderr son *parámetros*: en producción, los de verdad; en los tests, buffers.

**El momento ¡ajá!**: REPL vs script no es «dos productos» — es **una política de errores**. Y la transacción no se puede «guardar en la sesión» para retomarla en la línea siguiente, porque el cap. 27 ya decidió, en los tipos, que la tx *es* el préstamo del store. La API que diseñaste hace cuatro capítulos decide hoy la forma de tu bucle.

## 31.4 Primera solución

La primera solución es el hito, y sigue viva dentro de la CLI final. Su forma era:

```rust
// El hito (§25): parseo MANUAL de argv, tres casos, sin estado.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("help") => { imprimir_ayuda(out); EXIT_OK }
        Some("demo")  => cmd_demo(out, err),
        Some("query") => cmd_query(args.get(1).expect("consulta"), out, err),
        _ => { emitir(err, "uso: liradb demo | query \"<LiraQL>\" | help\n"); EXIT_ERROR_USO }
    }
}
```

Con tres subcomandos y cero flags, esto es honesto y suficiente. Y traía ya, sembradas por el §25, las tres decisiones que este capítulo no toca: `run(args, out, err) -> i32` con los writers **inyectados** (los tests escriben en un `Vec<u8>`), los exit codes Unix (0/1/2), y `emitir` tolerante — `let _ = write_all(...)` para que `liradb demo | head -1` no entre en pánico cuando la tubería se cierra.

## 31.5 Sus límites

El `match` manual aguanta tres subcomandos. Lo que no aguanta es el producto:

1. **Cada flag nuevo es otra rama artesanal.** `query --plan`, `--stats`, `repl --graph demo|empty`: con `match` tienes que validar la aridad a mano, el `--graph` incorrecto produce un error tuyo sin alternativas, y `liradb --version` hay que escribirlo entero. Todo eso lo hace mejor una máquina que tú.
2. **Sin sesión no hay REPL**, y sin REPL no hay producto: el usuario no puede construir *su* grafo ni verlo crecer.
3. **Sin intérprete compartido, REPL y script divergen.** Si escribes dos bucles parecidos — uno en `cmd_repl`, otro en `cmd_script` — nace el clásico bug del meta-comando que funciona en uno y no en el otro.
4. **La transacción del cap. 27 no cabe «añadida»**: no es una estructura de datos que guardas; es un préstamo con ciclo de vida en los tipos. El REPL tiene que *adaptarse a ella*, no al revés.

## 31.6 Solución evolucionada

La evolución toca las cuatro piezas a la vez, y cada una responde una mitad de la pregunta crítica del CORPUS.

**Clap para el árbol de comandos.** La CLI adopta su **primera dependencia externa**: `clap = "4.5"` (resuelta a 4.6.6, pineada por el `Cargo.lock`) — el pago de la regla del Vol. II «primero a mano, luego con crates»: ahora que el lector sabe lo que cuesta parsear argv, deja de pagarlo. Con la **Builder API**, el árbol entero de `liradb` son unas cincuenta líneas legibles en `comando()` (`lib.rs`): cinco subcomandos, `--graph` con `value_parser(["demo", "empty"])` y `default_value("demo")`, `--plan`/`--stats` como `ArgAction::SetTrue`. El contraste con `#[derive(Parser)]` — la forma declarativa que la mayoría usa en producción, donde el árbol vive en atributos de un struct — es contenido explícito del capítulo: **derive** compra ergonomía con macros; **Builder** muestra cada nodo a la vista. Para *aprender* qué es un árbol de comandos, Builder; para *mantener* una CLI grande, derive.

**Sesión e intérprete compartido.** `Sesion { store: MemoryStore }` nace de `--graph demo|empty` y vive entre líneas. Sobre ella, `sesion::interpretar_linea` ejecuta una línea: una consulta LiraQL (lo que no empieza por `:`), o un meta-comando `:help :quit :demo :clear :graph :node :edge :del-node :del-edge :begin :commit :rollback`. Devuelve `Ok(Accion::Seguir | Salir)` o `Err(String)` **sin imprimir**: el error es un valor y cada frontal lo reporta a su manera — el REPL lo imprime y sigue; el script le añade «línea N:» y muere.

**REPL vs script: dos políticas, códigos de salida honestos.** El REPL imprime `liradb> `, trata el EOF como `:quit` (salida limpia, exit 0) y **continúa** tras los errores: es una conversación, y un error es información. El guion no imprime nada, salta comentarios (`#`, `--`) y vacías, y **detiene** en el primer error con su número de línea y exit 1: es automatización, y un guion que «sigue» tras un error ejecuta la mitad del plan sin que nadie lo note. Son las dos reglas de Eric S. Raymond en «The Art of Unix Programming» (2003): la *Rule of Silence* (un programa que no tiene nada sorprendente que decir, que calle — el guion no pone prompts) y la *Rule of Repair* («cuando debas fallar, falla ruidosamente y cuanto antes»). Y la tabla de datos siempre por stdout, los diagnósticos siempre por stderr: así `liradb query … | wc -l` cuenta filas y no errores. La tabla completa de políticas:

```
                 REPL (cmd_repl)              script (cmd_script)
──────────────────────────────────────────────────────────────────────
prompts          liradb>  ·  tx> (en tx)      ninguno (Rule of Silence)
entrada          &mut dyn Read (stdin)        fichero · '-' = stdin (pipes)
comentarios      # y -- se saltan             # y -- se saltan
error            a stderr, la sesión SIGUE    «línea N: …» a stderr y EXIT 1
:quit            sale (exit 0)                corta el guion (exit 0)
EOF              = :quit (exit 0; con tx:     = fin del guión (exit 0;
                 rollback implícito)          con tx: rollback implícito)
tx abierta       consultas rechazadas         lo mismo: un intérprete
──────────────────────────────────────────────────────────────────────
```

**La transacción real, con prompt propio.** `:begin` abre una `Transaccion` de verdad (cap. 27) en un **sub-bucle** que consume líneas del mismo frente de lectura: dentro sólo se admite *staging* (`:node`, `:edge`, `:del-*` → `staged`), `:commit`, `:rollback`, `:quit` y `:help`; las consultas se rechazan con «el store está prestado», y `:quit` o EOF con la tx abierta son **rollback implícito** — el `Drop` del cap. 27 hecho producto.

## 31.7 Código completo ejecutable

El código está en `crates/vol2-liradb-cli` y compila verde: 34 tests (11 del hito conservados + 23 nuevos), ALL_GREEN con 751 en el workspace (§36). Léelo por partes — cada decisión tiene un porqué.

### El árbol de comandos: clap Builder

```rust
fn comando() -> Command {
    Command::new("liradb")
        .version(env!("CARGO_PKG_VERSION"))
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .subcommand(Command::new("demo").about("4 consultas de muestra sobre el grafo demo"))
        .subcommand(Command::new("query").about("…")
            .arg(Arg::new("consulta").required(true).value_name("LIRAQL"))
            .arg(Arg::new("plan").long("plan").action(ArgAction::SetTrue))
            .arg(Arg::new("stats").long("stats").action(ArgAction::SetTrue)))
        // … explain · repl · script · help
}
```

Cada subcomando y flag es una línea: el árbol se *lee*. Fíjate en `disable_help_flag/subcommand`: la ayuda de clap está **desconectada** a propósito, porque la ayuda de `liradb` es la curada del libro — con ejemplos copiables, el grafo demo documentado y el gancho al cap. 32. La ayuda autogenerada documenta el *árbol*; la curada documenta el *producto*. Sin args, o con `help`/`--help`/`-h` exactos, `run_con_entrada` imprime la curada y sale 0 (la semántica del hito, conservada y testeada en `sin_args_muestra_ayuda_y_sale_0` y `help_explicito_muestra_ayuda_y_sale_0`).

### `run_con_entrada`: la entrada también es un parámetro

```rust
pub fn run_con_entrada(args: &[String], entrada: &mut dyn Read,
                       out: &mut dyn Write, err: &mut dyn Write) -> i32
```

Es el `run` del hito con la **entrada inyectada**: el REPL y `script -` leen de aquí — stdin real en producción, un `Cursor` en los tests (`cli_con_entrada` en `mod tests`). Los 23 tests nuevos existirían en esa forma: probar un REPL spawneando procesos y TTYs es frágil; probarlo alimentando un buffer es gratis. Y cuando clap rechaza el uso, el error **no** sale a stderr real: `e.render()` escribe al `err` inyectado, `use_stderr()` decide a cuál de los dos writers va, y `e.exit_code()` devuelve el 2 — que es exactamente el `EXIT_ERROR_USO` del hito. Ejecución real:

```
$ liradb repl --graph nada
error: invalid value 'nada' for '--graph <ORIGEN>'
  [possible values: demo, empty]
$ echo $?
2
```

Ese «[possible values]» es el argumento definitivo a favor de `value_parser`: la errata del usuario viene con su solución. El 2 no es capricho: es la convención que el manual de bash recoge para «uso incorrecto» (0 éxito, 1 error general, 2 misuse), y la que clap aplica a sus errores de uso. La tabla completa del contrato con la shell, con salidas reales:

```
$ liradb query "MATCH (p:Person) RETURN p.name, p.age"     → EXIT 0
p.name  | p.age
"Ana"   | 36
"Bo"    | 41
"Carla" | 29
"Dani"  | 36

$ liradb query "SELECCIONA TODO"                            → EXIT 1 (stderr)
error: error de sintaxis: toda consulta LiraQL debe empezar con MATCH (en 0..10)

$ liradb drop todo                                          → EXIT 2 (stderr, clap)
error: unrecognized subcommand 'drop'
```

La distinción importa de verdad: el 1 dice «tu consulta era una intención y falló»; el 2 dice «ni siquiera entendí lo que me pediste». Los scripts de CI — y el cap. 33, que probará esta CLI — distinguen ambos casos; un `wc -l` sobre el primero cuenta filas porque la tabla fue a stdout y el diagnóstico a stderr, y POSIX garantiza que ese stderr no se tampona: el error aparece aunque el proceso muera a mitad de frase.

### `Sesion` e `interpretar_linea`: el mostrador compartido

`Sesion::nueva(origen)` (`sesion.rs`) monta el grafo inicial (`demo_graph()` del cap. 20 o vacío) y expone `store()`, `conteo()` y el recargado en caliente (`:demo`/`:clear`). El corazón es `interpretar_linea`: recorta la línea, salta vacías y comentarios, decide si es meta-comando (`:` inicial) o consulta — con una tolerancia deliberada: el `;` final de la costumbre SQL se acepta (`repl_tolerar_el_punto_y_coma_final`). Castigar con «error de sintaxis» una consulta perfecta por un punto y coma de más es pelearse con la memoria muscular del usuario. Fuera de transacción, las escrituras van por `autocommit` del cap. 27 — cada `:node` es su propia transacción, y el mensaje lo dice:

```
$ printf ':node 0:Person name="Zoe" age=44\n:node 1:City name="Oviedo"\n\
:edge 0 0 1 LIVES_IN since=2019\nMATCH (p:Person)-[:LIVES_IN]->(c:City) \
RETURN p.name, c.name\n:quit\n' | liradb repl --graph empty
LiraDB REPL — sesión 'empty' (0 nodos, 0 aristas). :help para ayuda, :quit para salir.
liradb> ok: nodo 0 (autocommit, 1 operaciones)
liradb> ok: nodo 1 (autocommit, 1 operaciones)
liradb> ok: arista 0 (0 -> 1) (autocommit, 1 operaciones)
liradb> p.name | c.name
"Zoe"  | "Oviedo"
liradb>
```

Esa sesión es el test-tesis `repl_construye_y_consulta_su_propio_grafo`: sin ficheros ni importaciones, la CLI es un cliente completo del motor.

### `bucle_transaccion`: la tx por valor, y el prompt `tx>`

Aquí está la decisión más instructiva del capítulo. `:begin` hace `Transaccion::begin(sesion.store_mut())` y le pasa la transacción al sub-bucle **por valor**:

```rust
fn bucle_transaccion(mut tx: Transaccion<'_>, entrada: &mut dyn BufRead, …)
```

La primera versión la tomaba por `&mut`, y el compilador dijo no cuatro veces: **E0507, cannot move out of `*tx`**. Claro: `commit` y `rollback` *consumen* `self` (cap. 27 — el ciclo de vida vive en los tipos), y con una referencia no se puede consumir nada. La conclusión no fue pelear: fue entender que **la API del cap. 27 ya había decidido cómo se usa una transacción** — vive dentro de un ámbito, y el ámbito muere cuando ella muere. Por valor, el bucle termina *exactamente* cuando la tx se consume: `commit` acierte o falle (con `Err` la tx queda descartada y nada se aplicó), `rollback`, `:quit` (rollback implícito), o EOF (rollback implícito). Intentar guardarla en la sesión para «retomarla en la línea siguiente» habría sido una guerra de lifetimes: la sesión necesita `&mut store` para consultar, y la tx lo tiene prestado. Sesión real:

```
$ printf ':begin\n:node 0:Person name="Vega"\n:commit\nMATCH (p:Person) RETURN p.name\n' \
  | liradb repl --graph empty
liradb> tx>      staged
tx>      commit: 1 operaciones (1 nodos y 0 aristas escritos, 0 nodos y 0 aristas
borrados) — en RAM, no durable (cap. 28)
liradb> p.name
"Vega"
```

Y con la tx abierta, el préstamo exclusivo se hace visible — la consulta y `:demo` se rechazan en stderr:

```
transacción abierta: el store está prestado (cap. 27); las consultas esperan —
:commit o :rollback primero
transacción abierta: ':demo' no aplica aquí — sólo staging (:node/:edge/:del-*)
y :commit/:rollback/:quit
```

El test `repl_transaccion_las_consultas_esperan` exige esos dos rechazos y un `:commit` final con «0 operaciones». El staging valida *eager* como en el cap. 27: un `:edge 9 9 9 KNOWS` (extremos inexistentes) imprime «staging rechazado» y la tx **sigue viva** con su prefijo válido (`repl_tx_staging_invalido_no_mata_la_tx`). Y `:quit` dentro de la tx: `:quit con transacción abierta → rollback implícito (1 operaciones descartadas)` — salida con exit 0, grafo intacto.

### `parse_valor`: el `Value` del cap. 7, otra vez

Las propiedades del usuario llegan como texto (`name="Zoe"`, `age=44`, `since=2019`) y salen como el `Value` del cap. 7 — el mismo enum que viajó por el encoding (cap. 9), las páginas (11) y el WAL (28). El orden del `match` **es** la semántica: comillas dobles → `String`; `true`/`false` → `Bool`; `null` → `Null`; si parsea como `i64` → `Int`; si como `f64` → `Float`; si no, error con el formato esperado. Por eso `42` y `2019` son `Int` y `3.5` es `Float` — y un valor comodín a `String` habría fabricado grafos «correctos» cuyos `WHERE p.age < 40` fallan en runtime con «tipos incompatibles». Los tres errores de formato del meta-comando (`se espera <id>:<Etiqueta>`, `id no numérico`, `sin '='`) están testeados en `repl_props_de_todos_los_tipos_y_errores_de_formato`. Que el parser de `:node` sea un `split_whitespace` y no una gramática es deliberado — simplicidad sobre completitud, el «worse is better» de Richard Gabriel («Lisp: Good News, Bad News, How to Win Big», 1991): la interfaz compleja (LiraQL completa) ya existe; el meta-comando es la culata, no el motor.

### `cmd_script`: el guion y su número de línea

`cmd_script` abre el fichero (o stdin si el argumento es `-`), crea la sesión, y por cada línea llama al **mismo** `interpretar_linea` con `interactivo=false`. La única diferencia con el REPL es la política: sin prompts, y el `Err(mensaje)` del intérprete se convierte en `línea {n}: {mensaje}` + exit 1 — **inmediato**:

```
$ printf '# mini guion\nMATCH (p:Person) RETURN p.name\nSELECCIONA TODO\n\
MATCH (p:Person) RETURN p.age\n' | liradb script -
p.name
"Ana"
"Bo"
"Carla"
"Dani"
línea 3: error: error de sintaxis: toda consulta LiraQL debe empezar con MATCH (en 0..10)
$ echo $?
1
```

La tercera consulta no se ejecutó jamás: eso exige el test `script_detiene_en_el_primer_error_con_linea`. Y como el intérprete es compartido, un guion puede CONSTRUIR su grafo (`:clear`, `:node…`), transaccionar (`script_con_transaccion`) y cortarse a mitad con `:quit` (exit 0, `script_quit_corta_el_guion_y_sale_0`).

## 31.8 Prueba de fuego

Cuatro tests-tesis, más los treinta que los rodean (`mod tests` de `lib.rs`):

**A — `repl_construye_y_consulta_su_propio_grafo`.** La CLI como cliente completo: tres escrituras autocommit y una consulta que lo ve todo, alimentadas por un buffer. Sin ficheros, sin importaciones.

**B — `repl_transaccion_las_consultas_esperan`.** El préstamo exclusivo hecho producto: dos rechazos con «el store está prestado» en stderr y un commit vacío que deja la sesión consistente.

**C — `script_detiene_en_el_primer_error_con_linea`.** Fail fast con contexto: la primera consulta responde, la segunda mata el guion con «línea 2:», la tercera no existe.

**D — `repl_y_query_y_script_vuelven_el_mismo_dato`.** Tres frontales, un motor: la consulta `WHERE p.age < 40` devuelve Ana, Carla y Dani (y no Bo) por `query`, por REPL y por script.

**Síntoma si te saltas el capítulo**: llegas al cap. 32 (import/export) sin intérprete al que enchufar los formatos, y a los caps. 33-34 (pruebas, benchmarks) sin forma de ejercitar el motor end-to-end. Tu «producto» sigue siendo una librería con ejemplos: útil para quien programa contra ella, invisible para quien la usa.

## 31.9 Qué hemos sacrificado

1. **La sesión vive en RAM**: cerrar el REPL pierde el grafo. No hay «abrir un fichero» — la persistencia de los caps. 10-16 existe en el motor, pero la CLI aún no la expone. Es la delimitación honesta contra el cap. 32.
2. **Sin TTY de verdad**: no hay historial, ni edición de línea, ni colores. El REPL lee líneas planas de un `dyn Read` — que es justo lo que lo hace testable. `rustyline` y compañía son la evolución natural, fuera del libro.
3. **Un comando por línea**: LiraQL no admite consultas multilínea en el REPL (el `;` final se tolera, no se exige). Una consulta larga se escribe en un guion.
4. **Ayuda curada = ayuda mantenida a mano**: cada flag nuevo obliga a tocar `imprimir_ayuda` (dos tests vigilan que ejemplos y contenido no se pudran). Es el precio de que la ayuda sea del producto y no del árbol.
5. **El `match` de `interpretar_meta` crece plano**: doce brazos; llegar a veinte meta-comandos pediría una tabla (nombre → función). Con doce, el `match` es más legible.

## 31.10 Cómo lo hace una BBDD real

Todo lo que aquí parece pequeño, allí es industria. `psql` es un REPL de miles de líneas con meta-comandos de contrabarra, salida paginada y `COPY` para importar en bloque — el cap. 32 es su pariente pobre. `cypher-shell` (Neo4j) usa `:` para sus meta-comandos, igual que nosotros, y parametriza consultas (`:param`). `sqlite3` ejecuta guiones con `.read fichero` y promete «un statement por línea» con punto y coma obligatorio — nuestra tolerancia del `;` es al revés, pero la discusión es la misma. Y en el mundo Rust, **ripgrep** (Andrew Gallant, «BurntSushi», 2016) es el ejemplo canónico de CLI pulida: su guía (`GUIDE.md`, más larga que este capítulo) documenta cada decisión de UX — y su éxito demuestra la tesis de la Parte VII: la herramienta *es* el producto.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial*: ¿por qué `run_con_entrada` recibe `&mut dyn Read` en vez de leer `std::io::stdin()` directamente dentro del REPL? ¿Qué tests serían imposibles?
- *Intermedio*: el REPL imprime los prompts a stdout, pero la tabla de datos también. ¿Por qué no rompe eso la disciplina stdout/stderr? ¿Cuándo un prompt «contamina» un pipe?
- *Experto*: diseña `liradb repl --persist fichero.lira` sin escribir código: ¿qué piezas de los caps. 10-16 entran, dónde vive la sesión y qué pasa con la tx abierta si el proceso muere?

## 31.11 Lo que te llevas

- **La CLI es el producto**: el quiosco frente al banco; un cliente del motor hexagonal que consume puertos (`&dyn` para leer, `&mut` para escribir) sin conocer las tripas.
- **Dos frontales, un intérprete**: REPL y script comparten `interpretar_linea`; la diferencia es política — prompts y errores — y vive en el driver, no en el intérprete.
- **El prompt `tx>` es el borrow checker traducido a texto**: mientras la tx vive, el store está prestado y las consultas esperan.
- **La tx va POR VALOR al sub-bucle** porque `commit`/`rollback` consumen `self` (E0507 fue el profesor): el bucle termina exactamente cuando la tx se consume. La API del cap. 27 decidió la forma del bucle.
- **`run_con_entrada`**: argv, entrada y salidas son parámetros; los 34 tests no spawnean procesos ni TTY.
- **clap Builder** enseña el árbol; los errores de uso son de clap (exit 2, con alternativas) y la ayuda es la curada del libro (exit 0).
- **Exit codes 0/1/2 y stdout/stderr**: 0 éxito · 1 consulta o fichero · 2 uso; datos por stdout, diagnósticos por stderr — o los pipes mienten.
- **EOF y `:quit` con tx abierta = rollback implícito**: el `Drop` del cap. 27, hecho producto.

## 31.12 Ojo, cuidado con…

- **«El REPL debería guardar la tx en la sesión»**. No puede: la tx tiene el `&mut` prestado. El sub-bucle por valor no es una rareza — es la única forma que no pelea con los tipos.
- **`commit` con `Err` y querer seguir usando la tx**. `commit(self)` la consume acierte o falle: con `Err`, la transacción está descartada (nada se aplicó) y la sesión sigue. «Use of moved value» es el compilador recordándote el cap. 27.
- **Probar el binario en vez de la función**. Los tests van contra `run_con_entrada`. Y si el binario «no recoge tus cambios», comprueba DÓNDE compila: un `target-dir` global en `~/.cargo/config.toml` puede dejarte un `target/` local fantasma con rmeta de 0 bytes (nos pasó — §36). Antes de culpar al código, `ls` la ruta absoluta del binario.
- **Mezclar datos y diagnóstico**. Un error a stdout envenena los pipes y los CI. La regla no tiene excepciones: tabla → stdout; error → stderr; código → el que corresponda.
- **Confundir exit 1 con exit 2**. El 1 dice «tu consulta es válida como intención pero falló»; el 2 dice «ni siquiera entendí lo que pediste». Los scripts de CI distinguen ambos; tu CLI también debe.

*Precisión de lenguaje*: *REPL* (bucle interactivo con sesión) vs *script mode* (lote no interactivo); *frontal/driver* (quién lee líneas) vs *intérprete* (quién las ejecuta); *sesión* (el grafo vivo) vs *transacción* (el préstamo con staging); *meta-comando* (`:`) vs *consulta* (LiraQL); *ayuda curada* vs *ayuda generada*; *exit code* del proceso vs error del motor (`ExecError`).

## 31.13 Pin de batalla

> *«Si tu motor sólo se sabe usar desde un test, no tienes un producto: tienes un secreto. El prompt es el contrato con tu usuario — y el exit code, el contrato con su shell.»*

## 31.14 Si solo lees 30 segundos

La CLI de LiraDB es el cliente del motor: `liradb demo | query | explain | repl | script`. REPL y script son dos frontales del mismo intérprete (`sesion::interpretar_linea`); el REPL pone prompts (`liradb>`, y `tx>` con transacción abierta) y continúa tras errores; el script calla, salta comentarios y muere en el primer error con «línea N:» y exit 1. clap (Builder) valida el uso — exit 2, con alternativas — y la ayuda curada del libro sale con exit 0; `run_con_entrada(args, entrada, out, err) -> i32` lo hace todo testeable con buffers. Las escrituras usan el cap. 27 de verdad: `autocommit` por defecto, `:begin` abre una `Transaccion` que el sub-bucle recibe POR VALOR (porque `commit`/`rollback` consumen `self`); mientras vive, el store está prestado y las consultas esperan; `:quit`/EOF con tx abierta son rollback implícito. Datos por stdout, errores por stderr, exit 0/1/2. Import/export llega en el cap. 32 y se enchufa aquí.

## 31.15 Una historia pequeña

La tarde en que «la CLI dejó de compilar nuestros cambios» fue una tarde de fantasmas. Todo iba bien: el test nuevo, verde; `cargo build`, verde; y sin embargo el binario seguía imprimiendo la salida vieja. Reinstalar, limpiar, rezar — nada. El culpable resultó invisible: `~/.cargo/config.toml` redirigía el `target-dir` a un directorio global compartido entre cuatro instancias de desarrollo, y el `target/` del proyecto era un residuo de una sesión interrumpida, con un `rmeta` de cero bytes dentro. El binario que ejecutábamos vivía en `~/cargo-targets/debug/liradb`; el «binario» que mirábamos, en `target/debug/`, era un fantasma. La lección quedó escrita en MIGRATION-PATTERN §36: *antes de culpar al código, comprueba dónde compila realmente* — `ls` con ruta absoluta del binario que vas a probar. Los bug más difíciles de la Parte VII no están en el motor: están entre el motor y el mundo.

## Ejercicios resueltos

**1. Predice la salida exacta.** Sesión sobre `--graph empty`: `:begin` · `:node 0:Person name="Vega"` · `MATCH (p) RETURN p` · `:commit` · `:quit`. ¿Qué imprime y con qué exit code termina? El banner de sesión abre stdout. La primera línea pinta `liradb> ` y `:begin` no imprime nada: el prompt pasa a `tx>`. El `:node` imprime `staged` (estamos en staging, no autocommit). La consulta va a **stderr** con «transacción abierta: el store está prestado (cap. 27); las consultas esperan — :commit o :rollback primero», y el bucle *continúa* (dentro de la tx, un rechazo no mata nada). El `:commit` confirma UNA operación — «commit: 1 operaciones (1 nodos y 0 aristas escritos…)» — y devuelve el prompt a `liradb>`. El `:quit` sale limpio: **exit 0**, aunque haya habido un rechazo por el camino. Verificación: `repl_transaccion_commit_persiste` y `repl_transaccion_las_consultas_esperan`.

**2. ¿Por qué la transacción va por valor al sub-bucle?** Porque `commit` y `rollback` consumen `self` — el ciclo de vida de la tx vive en los tipos desde el cap. 27. Con `&mut Transaccion`, el bucle no podría cerrarla: el compilador lo dijo cuatro veces (E0507, «cannot move out of `*tx`»). Por valor, el bucle termina *exactamente* cuando la tx se consume, sea por commit (con `Ok` o con `Err`: en ambos casos la tx murió), por `rollback`, por `:quit` o por EOF — los dos últimos con rollback implícito. La alternativa «guardar la tx en la sesión» no compila: la sesión necesita `&mut store` para consultar y la tx lo tiene prestado. No es una limitación molesta: es el diseño del cap. 27 obligando al REPL a tener la forma correcta. Verificación: `repl_transaccion_quit_es_rollback_implicito` (EOF/:quit), `repl_tx_staging_invalido_no_mata_la_tx` (la tx sobrevive al staging inválido).

## Ejercicios propuestos

**Esencial (recordar — retrieval practice).** Cierra el libro y `:help`. (a) Reconstruye de memoria el árbol de meta-comandos legales fuera de transacción — los once, más qué responde `:commit` sin tx y `:magic`. (b) Lista los legales DENTRO de una transacción y di qué se rechaza en cada caso. (c) Predice la salida exacta (stdout, stderr y exit code) de la sesión del ejercicio resuelto 1 con `:rollback` en lugar de `:commit`. *Pistas*: (1) los comandos de sesión (`:demo`, `:clear`, `:graph`) ¿tocan el store?; (2) ¿qué resumen imprime el rollback y con qué número?; (3) ¿el `:quit` final cambia de significado? *Verificación*: `repl_transaccion_rollback_descarta`, `repl_commit_sin_tx_abierta_se_rechaza`, `repl_meta_desconocido_se_reporta`, y pegar la sesión en `cargo run -p liradb-cli -- repl --graph empty`. *Criterio*: árbol completo y salida byte a byte, prompts incluidos.

**Intermedio (analizar — interleaving con el cap. 27).** Sobre `--graph empty`: `:begin` · `:node 0:Person name="Efímero"` · `:node 1:Person name="Fija"` · `:quit`. (a) ¿Qué línea imprime el REPL antes de salir y con cuántas operaciones descartadas? (b) ¿Qué queda en el store y con qué exit code termina el proceso? (c) El cap. 27 decía que el `Drop` de una tx activa es rollback «seguro por construcción»; aquí, ¿quién *decide* ese rollback — el tipo, o el driver? (d) ¿Por qué la consulta durante la tx se RECHAZA en vez de convertirse en el primer caso posible de `Anomalia::LecturaSucia`? *Pistas*: (1) mira quién consume la tx en `bucle_transaccion`; (2) ¿qué préstamo retiene la tx mientras vive?; (3) EOF y `:quit` comparten camino. *Verificación*: `repl_transaccion_quit_es_rollback_implicito` y `repl_transaccion_rollback_descarta`. *Criterio*: explicar el driver como traductor de las garantías del cap. 27.

**Experto (crear — spacing con el cap. 7).** Escribe el guion `tipos.ql` que (1) vacíe la sesión, (2) construya en UNA transacción el nodo `0:Prueba valor=42 otro=3.5 flag=true nada=null texto="hola"` y la arista `:edge 0 0 0 BUCLE desde=2019`, con un staging inválido en medio (`:edge 9 9 9 KNOWS`) que deba expulsarse sin matar la tx, (3) haga commit y (4) consulte las props para demostrar que `42` llegó como `Int`, `3.5` como `Float` y `2019` como `Int` — ¿por qué no Float? Ejecútalo por las dos vías: `liradb script tipos.ql` y `cat tipos.ql | liradb script -`. *Pistas*: (1) ¿qué imprime un staging aceptado, uno rechazado, y en qué stream va cada uno?; (2) el orden de `parse_valor`: comillas → bool/null → i64 → f64; (3) ¿qué dice el resumen del commit con una op expulsada? *Verificación*: `repl_props_de_todos_los_tipos_y_errores_de_formato`, `repl_tx_staging_invalido_no_mata_la_tx`, `script_con_transaccion`. *Criterio*: guion verde por las dos vías + explicar por qué el orden del `match` decide el tipo final.

## Para profundizar

- **John McCarthy, «History of LISP» (HOPL I, 1979)** y **McCarthy, Abrahams, Edwards, Hart y Levin, «LISP 1.5 Programmer's Manual» (MIT Press, 1962)** — las fuentes primarias de la anécdota: el sistema conversacional donde nació el bucle leer-evaluar-imprimir.
- **Brian W. Kernighan y Rob Pike, «The Unix Programming Environment» (Prentice Hall, 1984)** — la shell como el intérprete que sirve igual al terminal y al guion; el modelo mental de este capítulo es suyo.
- **Eric S. Raymond, «The Art of Unix Programming» (Addison-Wesley, 2003, libre en la red)** — Rule of Silence y Rule of Repair: el porqué de los guiones callados que fallan ruidosamente; y la disciplina de datos/diagnóstico.
- **Documentación oficial de clap 4 (docs.rs / el «clap book»)** — Builder vs Derive, `value_parser`, y el comportamiento de los errores de uso (exit 2).
- **GNU Bash Reference Manual («Exit Status»)** — la convención 0/1/2 que la CLI hereda; **POSIX.1 (Base Definitions)** — stdout tamponado vs stderr sin tamponar, y por qué los diagnósticos tienen su propio canal.
- **Docs de PostgreSQL (`psql`), SQLite (la shell `sqlite3` y sus dot-commands) y Neo4j (`cypher-shell`)** — los REPLs reales de bases de datos y sus meta-comandos: la familia a la que `liradb repl` se suma.
- **ripgrep (BurntSushi) y su `GUIDE.md`** — qué significa pulir una CLI cuando la CLI es el producto; el ejemplo canónico del ecosistema Rust.
- **`MIGRATION-PATTERN.md` §25 y §36** — la prosa verificable detrás del capítulo: las decisiones del hito, el E0507, y el binario fantasma del target-dir global.

## Mini-diálogo: en guardia nocturna

> — O sea, que media Parte VII es «ponerle prompts al motor». ¿Y por eso un capítulo entero?
>
> — Porque el prompt decide si lo demás existió. Tu WAL, tu optimizador, tu MVCC — todo es invisible hasta que algo lo enseña. La CLI es el único cliente que tendrá la mayoría de tus usuarios: en ella, el cap. 27 se vuelve un prompt que cambia y un mensaje que explica por qué tu consulta espera.
>
> — ¿Y lo del `tx>`? Pensé que era decoración.
>
> — Es lo contrario. El borrow checker exigía al *código* lo que el prompt le enseña al *usuario*: mientras la transacción vive, el store está prestado. La primera vez que la consulta te responde «espera», estás viendo un tipo de Rust a través de una pantalla.
>
> — Pero el REPL perdona errores y el guion muere en el primero. No parece justo.
>
> — No es justicia: son dos audiencias. El que charla quiere seguir charlando; el que automatiza quiere la verdad entera y pronto, con número de línea. Un intérprete, dos políticas — como la shell de toda la vida.
>
> — ¿Y el exit code?
>
> — El contrato con la shell. Cero, uno o dos. Si mientes ahí, todos los scripts que te usen mienten contigo.

---

*(Próximo capítulo: 32 — Importación y exportación (CSV, JSONL, GraphML). Este capítulo dejó el quiosco montado; ahora llegan los camiones de datos. El import se enchufa a ESTE intérprete — un lector de formatos que produce operaciones, transacción a transacción, con la misma `Sesion` y los mismos resúmenes de commit. Y el export empieza donde termina cada consulta: en la tabla del `ResultSet`.)*
