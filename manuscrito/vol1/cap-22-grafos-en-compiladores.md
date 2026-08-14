# Capítulo 22 — Grafos en Compiladores

Compilar es como hacer el amor: placentero, misterioso, y un solo error de sintaxis y todo se va al traste. Lo que no te cuentan es que debajo de esa magia hay un grafo. Tu código, antes de ser instrucciones de máquina, pasa por al menos cuatro grafos distintos. Y en uno de ellos — el del *interference graph* — la coloración que aprendiste en el Capítulo 13 se cobra venganza.

En este capítulo vamos a recorrer ese viaje: AST → CFG → DFG → SSA → interference graph. Y vamos a hacer un mini-compilador de expresiones aritméticas que emite LLVM IR textual. Mano a la obra.

## 22.0 La anécdota de la lista que se compila sola

Estamos en 1958. John McCarthy, un matemático del MIT, define un lenguaje llamado LISP (List Processing). Lo que hace único a LISP es algo entonces radical: **el programa y los datos son la misma cosa**, específicamente, **listas enlazadas** (que él llama *cons cells*). Una expresión como `(if (> x 0) (+ x 1) (- x 1))` es, a la vez, código y dato: una lista cuyo primer elemento es el operador `if` y cuyos siguientes son los argumentos.

Esta homogeneidad tiene una consecuencia bonita: el compilador de LISP puede tratar el programa como una estructura de datos y manipularlo antes de compilar. Macros, optimizaciones, introspección: todo se vuelve natural cuando "el código es un grafo que puedes caminar". El AST de LISP es literalmente un grafo de listas, y muchos lenguajes modernos (Haskell, Clojure, Rust en parte) heredan algo de esa idea: el AST es una estructura de primera clase.

El punto para nosotros: **todo compilador moderno, sin excepción, tiene un AST**. Algunos lo llaman parse tree. Algunos lo adornan con tipos. Pero todos, en el fondo, están caminando un grafo.

## 22.1 AST: el Abstract Syntax Tree

Cuando escribes `let x = 1 + 2 * 3;` y el compilador lo lee, lo primero que hace es construir un **AST** (Abstract Syntax Tree). Un árbol es un grafo (acíclico, conectado), así que ya estamos en casa.

```
        let
       /   \
      x     +
           / \
          1   *
             / \
            2   3
```

Nodos: `let`, `x`, `+`, `1`, `*`, `2`, `3`. Aristas: "el operando izquierdo de `+` es 1", "el operando derecho de `+` es `*`", etc. En Rust, podemos representarlo así:

```rust
#[derive(Debug, Clone)]
pub enum Expr {
    Num(i64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Let(String, Box<Expr>, Box<Expr>), // let x = e1 in e2
}
```

Parsear "1 + 2 * 3" da `Add(Num(1), Mul(Num(2), Num(3)))`. Si quieres el árbol, lo "derivas" con un visitor. Esto es un grafo, y los visitors son traversals.

## 22.2 CFG: el Control Flow Graph

Una vez tienes el AST, el compilador hace una cosa clave: lo **aplana** en bloques básicos (Basic Blocks, BB) y dibuja cómo salta la ejecución de uno a otro. Ese dibujo es el **CFG** (Control Flow Graph). Cada BB es un nodo, y cada salto (if, while, return, break) es una arista.

```
Ejemplo: if (x > 0) { y = 1; } else { y = -1; }
print(y);

CFG:

       entry
        │
        ▼
   ┌──────────┐
   │ x > 0 ?  │
   └────┬─────┘
        │ true ───► BB_true ───┐
        │ false ──► BB_false ──┤
        │                      │
        └──────────┬───────────┘
                   ▼
              BB_print
                   │
                   ▼
                 exit
```

En Rust, podemos representarlo como un `Graph<BasicBlock, ()>`. Los BB tienen una lista de instrucciones y dos "salidas" (verdadero/falso, o secuencial + salto).

## 22.3 DFG: el Data Flow Graph

Ahora la pregunta interesante: **¿cómo viajan los datos?** El DFG (Data Flow Graph) conecta las instrucciones que **producen** un valor con las que lo **consumen**. Una variable `x` definida en BB1 y usada en BB3 genera una arista BB1 → BB3 con la etiqueta "x".

```
   BB1              BB2              BB3
┌──────────┐     ┌──────────┐     ┌──────────┐
│ t1 = a+b │ ──► │ t2 = t1*c│ ──► │ print t2 │
└──────────┘     └──────────┘     └──────────┘

t1 fluye de BB1 a BB2 (producida en BB1, consumida en BB2).
t2 fluye de BB2 a BB3.
```

Esto es la base de optimizaciones como **constant propagation** (si `t1` siempre vale 5, sustitúyelo) y **dead code elimination** (si nadie consume `t2`, bórralo).

## 22.4 SSA: Static Single Assignment, el truco de los modernos

La mayoría de compiladores modernos (LLVM, Cranelift, el tuyo si escribes uno) usan una representación llamada **SSA** (Static Single Assignment). La regla es sencilla y elegante: **cada variable se asigna una sola vez**. Si en tu código original `x` se reasigna tres veces, SSA la "renombra" en cada asignación:

```c
// Código original
x = 1
if cond:
    x = 2
else:
    x = 3
print(x)

// En SSA
x1 = 1
if cond:
    x2 = 2
else:
    x3 = 3
x4 = φ(x2, x3)  // "phi function": toma x2 o x3 según la rama
print(x4)
```

La `φ` (phi) function es donde el grafo se vuelve interesante: es un nodo virtual con dos entradas (una por rama del if). Esto convierte el flujo de control en un **grafo de dependencias explícito**: cada valor tiene un único punto de definición. Las optimizaciones se vuelven casi triviales: si dos `x` tienen el mismo valor, son la misma SSA-variable.

```
  x1=1 ──┐
         ├──► φ(x2,x3) ──► x4 ──► print
  x2=2 ──┤
  x3=3 ──┘
```

## 22.5 Liveness analysis y el interference graph

Ahora la parte que esperabas: **la coloración de grafos en acción**. El problema de la asignación de registros (register allocation) es: tienes un programa con N variables, pero la CPU solo tiene K registros. ¿Qué variables pones en cada registro, y cuáles se "derraman" a memoria (spill)?

El algoritmo clásico, debido a Chaiten (1981) y refinado por Briggs, hace esto:

1. **Liveness analysis**: para cada punto del programa, determina qué variables están "vivas" (van a leerse en el futuro) y cuáles ya están "muertas".
2. **Interference graph**: dos variables `a` y `b` **interfieren** si están vivas a la vez en algún punto. Si las pones en el mismo registro, una sobreescribirá a la otra. Conectas `a` y `b` con una arista.
3. **Coloración**: colorea el interference graph con K colores. Cada color = un registro. Si no puedes, sacas (spill) una variable a memoria y vuelves a intentar.

La coloración de grafos del Capítulo 13, **exactamente la misma**, se usa aquí. Y es NP-hard en general, pero con la heurística de Chaiten (spill el nodo con más vecinos) funciona muy bien en la práctica.

```
Interference graph de ejemplo (5 vars, 3 registros):

   a ─── b
   │ \ / │
   │  c  │
   │ / \ │
   d ─── e

  3 colores posibles? Sí: a=rojo, b=azul, c=verde, d=azul, e=rojo.
  Comprueba: vecinos de a (b, c, d) son rojo, verde, azul. ✓
```

Si tuvieras solo 2 colores, el grafo anterior no se podría colorear, así que tendrías que hacer spill de una variable.

## 22.6 Mini-compilador a LLVM IR en Rust

Vamos a hacer un compilador de expresiones aritméticas a **LLVM IR textual** (el formato `.ll`). Usaremos `syn` solo para mostrar el parseo, pero el grueso será a mano para mantenerlo pedagógico.

```toml
[dependencies]
syn = { version = "2.0", features = ["full", "parsing"] }
quote = "1.0"
```

```rust
use syn::Expr;

// Recorre un AST y emite instrucciones LLVM IR.
// Usamos nombres únicos para cada valor temporal (%t1, %t2, ...).
struct Codegen {
    code: String,
    counter: u32,
}

impl Codegen {
    fn new() -> Self { Self { code: String::new(), counter: 0 } }
    fn fresh(&mut self) -> String { self.counter += 1; format!("%t{}", self.counter) }
    fn emit(&mut self, s: &str) { self.code.push_str(s); self.code.push('\n'); }

    // Compila una expresión. Devuelve el nombre del valor LLVM que contiene el resultado.
    fn compile_expr(&mut self, e: &Expr) -> String {
        match e {
            Expr::Lit(lit) => {
                if let syn::Lit::Int(i) = lit {
                    i.to_string()
                } else { panic!("solo enteros"); }
            }
            Expr::Binary(b) => {
                let l = self.compile_expr(&b.left);
                let r = self.compile_expr(&b.right);
                let dst = self.fresh();
                let op = match b.op {
                    syn::BinOp::Add(_) => "add",
                    syn::BinOp::Sub(_) => "sub",
                    syn::BinOp::Mul(_) => "mul",
                    syn::BinOp::Div(_) => "sdiv",
                    _ => panic!("op no soportada"),
                };
                self.emit(&format!("  {} = {} i64 {}, {}", dst, op, l, r));
                dst
            }
            _ => panic!("expr no soportada: {:?}", e),
        }
    }

    pub fn compile(src: &str) -> String {
        let ast: Expr = syn::parse_str(src).expect("parse");
        let mut cg = Codegen::new();
        cg.emit("define i64 @main() {");
        let result = cg.compile_expr(&ast);
        cg.emit(&format!("  ret i64 {}", result));
        cg.emit("}");
        cg.code
    }
}

fn main() {
    let ir = Codegen::compile("1 + 2 * 3");
    println!("{}", ir);
}
```

Salida:
```llvm
define i64 @main() {
  %t1 = mul i64 2, 3
  %t2 = add i64 1, %t1
  ret i64 %t2
}
```

Eso es LLVM IR real. Lo puedes guardar en `prog.ll`, y con `llc prog.ll -o prog.s && gcc prog.s -o prog` lo conviertes en ejecutable. En serio. Tu `1 + 2 * 3` acaba en un binario de verdad.

## 22.7 Diálogo de after-hours

> — Lupe, ¿por qué LLVM introduce variables `%t1`, `%t2` aunque podría reusar `%t1`?
> — Por el SSA, Elsa. Cada valor se asigna una sola vez, así que si reusaras `%t1`, perderías la trazabilidad de qué-produce-qué. Y todas las optimizaciones (constant folding, GVN, LICM) se vuelven pan comido cuando cada valor tiene una única definición.
> — Vale, pero ¿y si mi programa tiene un `for` con 1000 iteraciones? ¿Voy a tener 1000 variables SSA?
> — No, tranquila. SSA es por **función**, no por programa entero. Y los loops se manejan con phi-nodes y bloques de cabecera, no con 1000 variables. Es un truco elegante, no una explosión.
> — Elsa, ¿te has planteado que SSA es el equivalente en compiladores de hacer cada variable `let` inmutable en Rust?
> — Exacto. SSA es **inmutabilidad forzada** a nivel de IR. Por eso los optimizadores aman SSA: si algo se asigna una vez y no se reasigna, sabes que ese valor nunca cambia.

## 22.8 Aplicaciones del mundo real

- **LLVM**: el backend de Rust, Swift, Julia, muchos lenguajes nuevos. Todo su IR está en SSA.
- **Cranelift**: el backend experimental de Rust (en wasm, en algunos casos). SSA también.
- **JavaScriptCore (JIT de Safari)**: usa SSA para su optimizador.
- **GCC (GIMPLE y RTL)**: usa una representación próxima a SSA desde los 2000s.
- **HotSpot JVM**: optimiza Java bytecode a SSA antes de generar código máquina.

## 22.9 Ejercicios resueltos

**Ejercicio 22.1.** Convierte `if (x > 0) y = 1 else y = 2; z = y + 1` a SSA.

*Solución:*
```
if x > 0 goto L_then else L_else
L_then:
  y1 = 1
  goto L_merge
L_else:
  y2 = 2
  goto L_merge
L_merge:
  y3 = φ(y1, y2)
  z = y3 + 1
```

**Ejercicio 22.2.** Dado el CFG del §22.2, ¿cuántos basic blocks tiene y cuántos hay en el camino más largo desde entry hasta exit?

*Solución:* 4 BBs (entry, BB_true, BB_false, BB_print). El camino más largo pasa por 3 aristas (entry → BB_true → BB_print → exit o entry → BB_false → BB_print → exit).

**Ejercicio 22.3.** Dibuja el interference graph para este código con 3 variables (asumiendo CPU de 2 registros):

```
a = 1       // a vivo de aquí hasta print
b = 2       // b vivo de aquí hasta print
c = a + b   // c vivo de aquí hasta print
print(c)
```

*Solución:* a, b y c están vivos durante las 4 líneas (cada uno se usa en `print` o en la suma). El interference graph es un **triángulo completo** (K₃). Se necesitan 3 colores, así que con 2 registros hay que hacer spill de uno (típicamente `c`, porque es el menos usado después).

## 22.10 Ejercicios propuestos

1. Extiende el codegen de §22.6 para soportar **paréntesis explícitos** (que ya lo hace la precedencia de `syn` por ti, pero verifica el IR generado).
2. Implementa **constant folding** en el AST: si los dos operandos de una suma son literales, reemplaza la suma por el literal resultante. Recorre el AST en post-orden.
3. Escribe una función que detecte **variables no usadas** (dead code) en un AST simple y las marque.
4. Construye un CFG a partir de un AST con `if` y `while`. Pista: un nodo del CFG = un BB; las aristas son los saltos.
5. Implementa un **liveness analysis trivial** sobre un programa de 3 líneas (sin loops) y determina qué variables están vivas después de cada instrucción.

## 22.11 Pin de batalla

- **Si tu compilador emite IR incorrecto, no mires el optimizador, mira el frontend**. El 80% de los bugs están en cómo parseas y construyes el AST.
- **No escribas tu propio backend**. Salvo que sea un proyecto de aprendizaje, usa LLVM o Cranelift. Reimplementar x86_64 a mano es masoquismo puro.
- **SSA es tu amigo, no tu enemigo**. Si tu IR está en SSA, el optimizador funciona "casi solo". Sin SSA, todo es más difícil.
- **Usa `syn` y `quote` de Rust para parsear y emitir código**. Son la combinación estándar; reinventarlas a mano es perder el tiempo.
- **Si tu `cargo build` falla con un error de tipos críptico, recuerda: el compilador de Rust también pasa por todos estos grafos**. Tu error es un nodo en el AST de tu programa, y el compilador te está explicando qué arista está rota.

## 22.12 Lo que te llevas

Compilar no es magia, es un grafo. Tu código pasa por al menos cuatro: AST, CFG, DFG, y finalmente un **interference graph** que decide qué variable va en qué registro. La coloración de grafos del Capítulo 13 es exactamente la misma técnica que usa Chaiten para asignar registros. SSA es la representación intermedia que hizo todo esto práctico. Y con `syn` y LLVM, puedes construir un mini-compilador en menos de 100 líneas de Rust.

## 22.13 Ojo, cuidado con…

- **SSA no es gratis**. Las phi-nodes complican el register allocation. Los compiladores modernos insertan phi-nodes en una fase y luego las "descomponen" en copias en otra.
- **El IR textual de LLVM no es lo que LLVM usa internamente**. Es un formato de debug. La representación real es un grafo en memoria mucho más rico.
- **Los optimizadores pueden ser NO correctos**. Hay bugs famosos en GCC y LLVM que generan código incorrecto. Por eso existen los test-suites de miscompilaciones.

## 22.14 Para profundizar

- *"Engineering a Compiler"* de Keith Cooper y Linda Torczon. El libro de cabecera, cubre AST, CFG, SSA, register allocation.
- *"The LLVM Cookbook"* y la documentación oficial de LLVM: https://llvm.org/docs/
- *"Static Single Assignment Book"* en https://pfederl.github.io/ssa-book/ — excelente y gratis.
- Repositorios: el código fuente de `rustc` (que usa LLVM) y de `cranelift` están en GitHub y son sorprendentemente legibles.
- *"Crafting Interpreters"* de Robert Nystrom: si quieres hacer un intérprete antes que un compilador, este es EL libro.

## 22.15 Si solo lees 30 segundos

Un compilador moderno transforma tu código en cuatro grafos sucesivos: AST (estructura sintáctica), CFG (flujo de control), DFG (flujo de datos) y un interference graph (conflictos entre variables). En el último, la coloración de grafos decide qué variable va en qué registro. Todo eso en menos de un segundo, mientras te tomas un café.

## 22.16 Una historia pequeña

Fermín llevaba tres meses peleándose con un `compiler error: cannot infer appropriate lifetime` en su crate de Rust. Subió la pregunta a un foro. Un senior le respondió con una sola línea: "`cargo tree` y mira el AST expandido con `cargo rustc -- -Zunpretty=expanded`". Fermín lo hizo. Vio que su macro `quote!` generaba una referencia a un valor que se caía del scope. Cambió `&` por `.to_string()`. Compiló. Esa noche entendió que el compilador de Rust no era un ente superior: era un grafo, y él acababa de aprender a leerlo.

---

