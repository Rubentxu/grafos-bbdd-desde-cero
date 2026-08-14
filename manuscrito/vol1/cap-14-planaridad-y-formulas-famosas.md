# Capítulo 14 — Planaridad y fórmulas famosas

Un matemático polaco en un campo de concentración austríaco demostró la planaridad con la cabeza, sin papel. Lo publicó en 1930. La fórmula de Euler, el teorema de Kuratowski, y el 4-color theorem. Planaridad es de los temas más bellos de grafos.
## 14.0 La anécdota del matemático que demostró la planaridad en un campo de concentración

Cuenta la historia que en 1930, un joven matemático polaco llamado **Kazimierz Kuratowski** publicó uno de los teoremas más bellos de la teoría de grafos: la caracterización de los grafos planares. Hasta ahí, todo normal. Pero el contexto en el que lo pensó es lo que hace que la historia merezca la pena ser contada.

Kuratowski, nacido en Varsovia en 1896, fue uno de los matemáticos más importantes de la primera mitad del siglo XX, miembro de la famosa **Escuela de Topología Polaca** junto a nombres como Sierpinski, Mazurkiewicz y el mismísimo Stefan Banach. En 1930, estando en Lwów (entonces Polonia, hoy Ucrania), publicó su teorema: un grafo es planar si y solo si no contiene una subdivisión de K_5 ni de K_{3,3}. Pero esa es la parte "tranquila" de su vida.

Lo que poca gente sabe es que en la Segunda Guerra Mundial, Kuratowski fue arrestado por los nazis en 1939 (su esposa, también matemática, lo fue también). Estuvo preso en varios campos, incluido un campo de concentración austríaco. Y según cuentan sus allegados, parte del trabajo mental que hizo para mantener la cordura fue **pensar en grafos planares y subdivisiones**. Lo pensó en la cabeza, sin papel, sin lápiz, sin ordenador. Cuando la guerra terminó, salió vivo (no todos los matemáticos polacos tuvieron esa suerte — su colega Stefan Banach sobrevivió al gueto de Lwów pero murió de cáncer de pulmón en 1945) y siguió publicando hasta su muerte en 1980.

Lección: las ideas matemáticas, a veces, sobreviven donde la vida ordinaria no lo hace. Y un teorema que se demostró en 1930 sigue siendo la base de los algoritmos modernos de planaridad, **casi un siglo después**. Hoy día, si abres un navegador y entras en una página web, los algoritmos de layout que deciden dónde van los nodos y las aristas usan, en su esencia, ideas que vienen de Kuratowski.


> — ¿Qué es un grafo planar?
> — Uno que se puede dibujar en el plano sin que las aristas se crucen. Por ejemplo, K_4 es planar. K_5 no.
> — ¿Cómo lo caractérizo?
> — K_5 y K_{3,3} son los menores prohibidos (Kuratowski). Si tu grafo no contiene ninguno, es planar.
> — ¿Y la fórmula de Euler?
> — V - E + F = 2 en grafos conexos planares. Donde F son caras.
> — ¿Y el 4-color theorem?
> — 4 colores bastan para colorar un mapa planar sin que dos regiones adyacentes compartan color. Probado con ordenador en 1976.
## 14.1 ¿Qué significa que un grafo sea "planar"?

Un grafo es **planar** si puedes dibujarlo en un plano (un papel, una pizarra, lo que sea) de manera que las aristas **no se crucen**. No se trata de que "casi" no se crucen, ni de que "sólo un poquito". Cero cruces, salvo en los extremos de las aristas. Es una propiedad topológica: si un grafo es planar, lo es en cualquier *embedding* razonable; si no, no lo es nunca.

La pregunta "¿es G planar?" es algorítmicamente decidible y se puede responder en tiempo lineal (Hopcroft-Tarjan 1974, Boyer-Myrvold 2004). Pero la definición, siendo sencilla, esconde una maquinaria combinatoria brutal.

Tres ejemplos que te aclararán:
- Un **árbol** es planar (cualquier árbol se puede dibujar sin cruces).
- K_4 es planar (la "pirámide" clásica).
- K_5 **no** es planar. Da igual cómo lo dibujes, alguna arista se cruzará.
- K_{3,3} **no** es planar (es el "diagrama de los tres servicios y tres interruptores" que te contaban en clase de lógica).

## 14.2 Fórmula de Euler: V - E + F = 2

Aquí viene la primera herramienta seria. Si G es un grafo **conexo** y **planar**, y lo dibujas en el plano, obtienes un mapa con V vértices, E aristas y F **caras** (regiones, incluida la cara infinita, la "externa"). La **Fórmula de Euler** (1758) dice:

V - E + F = 2

Es uno de los teoremas más bellos de la matemática. La demostración es por inducción: tomas un *spanning tree* (que tiene V-1 aristas, una sola cara externa, luego V - (V-1) + 1 = 2 ✓), y luego cada arista extra que añades parte exactamente una cara en dos, manteniendo el invariante.

Comprobación con K_4: V=4, E=6. Como K_4 es *maximal planar* (cada cara es un triángulo), tiene 4 caras. Luego 4 - 6 + 4 = 2. ✓

## 14.3 Consecuencias inmediatas de Euler

Una de las gracias de la fórmula de Euler es que de ella se sacan **cotas** que los grafos planares deben respetar:

1. **Sin multiaristas, E ≤ 3V - 6.** Cada cara tiene ≥ 3 aristas y cada arista bordea 2 caras, así que 2E ≥ 3F. Sustituyendo F = 2 - V + E: 2E ≥ 3(2 - V + E), de donde E ≤ 3V - 6.
2. **Sin triángulos (girth ≥ 4), E ≤ 2V - 4.** Si cada cara tiene ≥ 4 aristas: 2E ≥ 4F, y entonces E ≤ 2V - 4.
3. **Todo grafo planar tiene un vértice de grado ≤ 5.** Por (1), suma de grados = 2E ≤ 6V - 12, así que el promedio de grado es < 6.
4. **K_5 no es planar.** Si lo fuera, tendría E ≤ 3·5 - 6 = 9, pero K_5 tiene 10 aristas. Contradicción.
5. **K_{3,3} no es planar (sin subdivisiones que creen triángulos).** K_{3,3} tiene 6 vértices, 9 aristas, girth 4. Si fuera planar, E ≤ 2·6 - 4 = 8, contradicción con E = 9.

Ejemplo clásico: el **grafo de Petersen** tiene V=10, E=15. La cota E ≤ 3V-6 da E ≤ 24, que no excluye planaridad. Pero Petersen contiene K_{3,3} como menor, así que tampoco es planar. Las cotas no son siempre suficientes; necesitamos teoremas más finos.

## 14.4 Teorema de Kuratowski: K_5 y K_{3,3} son los villanos

El **Teorema de Kuratowski (1930)** lleva la discusión a su forma definitiva:

> Un grafo G es planar **si y solo si** no contiene una **subdivisión** de K_5 ni de K_{3,3} como subgrafo.

Una subdivisión (también llamada *topological minor*) es lo que obtienes cuando reemplazas aristas por *paths* internamente disjuntos. Es decir: si puedes "estirar" las aristas de K_5 o K_{3,3} para que sigan siendo paths en G, entonces G no es planar.

Los dos grafos prohibidos K_5 y K_{3,3} se llaman **grafos de Kuratowski**, y son las dos únicas "razones" por las que un grafo puede no ser planar. Es un resultado notable: por muy enrevesado que sea tu grafo, si no es planar, la culpa la tienen estos dos.

```
  K_5 (no planar)                K_{3,3} (no planar)

       1 ---- 2                      1       4
       |\  /|                       / \     / \
       | \/ |                      /   \   /   \
       | /\ |                     2 --- 3 --- 5
       |/  \|                      \   /   \   /
       5 ---- 4                     \ /     \ /
                                     6       
   (no hay forma de dibujarlo
    sin cruzar aristas)            (conexión 3x3 bipartita)
```

## 14.5 Teorema de Wagner: menores en vez de subdivisiones

Una versión equivalente y a veces más manejable es el **Teorema de Wagner (1937)**:

> G es planar **si y solo si** G no contiene K_5 ni K_{3,3} como **minor**.

Un *minor* se obtiene contrayendo aristas (a diferencia de la subdivisión, que las "estira"). Las dos caracterizaciones son equivalentes: planaridad es cerrada bajo contracción de aristas, y por eso las dos formulaciones dan el mismo resultado.

Wagner también conjeturó (y Robertson-Seymour demostraron en su *Graph Minor Theorem*) que para cada *k*, los grafos sin *crossing number* mayor que *k* están caracterizados por un número finito de menores excluidos. Esa conjetura es la base de toda la teoría moderna de *graph minors*.

## 14.6 Boyer-Myrvold: cómo decidir planaridad en O(n)

El algoritmo práctico para planaridad en tiempo lineal es el de **Boyer y Myrvold (2004)**. El esquema simplificado:

1. Construir un *spanning tree* T e identificar las *back edges* (aristas no del árbol).
2. Para cada *back edge* e, calcular su *lower span* — el rango de aristas de T que la embedding puede "flippear".
3. Si alguna back edge viola restricciones, **G no es planar**.
4. Si todo pasa, construir el *planar embedding* mediante un *walk-up* sobre las back edges.

Implementaciones de referencia: `planarity.c` de John Boyer, o `boost::boyer_myrvold_planarity_test` de la Boost Graph Library en C++. En Rust, `petgraph` ofrece integración con crates externos de planaridad, y también podemos hacer una detección simplificada a mano con fines pedagógicos.

## 14.7 4-color theorem: la prueba con ordenador

> **Todo grafo planar es 4-coloreable.**

Este es el teorema del que hablamos en el capítulo anterior. Probado por **Appel y Haken (1976)**, simplificado por **Robertson, Sanders, Seymour y Thomas (1997)**. La prueba es por *discharging*: se asume una *minimal counterexample* G y se analizan sus posibles configuraciones locales; se reduce a 633 casos explícitos que se verifican computacionalmente.

El argumento histórico más instructivo es el de **Kempe (1879)**, que intentó probarlo por inducción. La idea: si G es planar minimal con χ ≥ 5, contiene un vértice v de grado ≤ 5. Si deg(v) ≤ 4, se colorea G-v con 4 colores y se reusa uno para v. Si deg(v) = 5, los vecinos a, b, c, d, e de v usan los 4 colores y Kempe intenta "recolorear" la *Kempe chain* de dos colores para liberar uno. Heawood encontró un error en 1890, y la corrección total tuvo que esperar casi un siglo.

Lección: en matemáticas, los argumentos "evidentes" pueden esconder bugs. Y a veces la única manera de cerrar el argumento es con un ordenador verificando 633 casos. Es la prueba de que el siglo XX trajo una nueva manera de hacer matemáticas.

## 14.8 Dualidad planar: del mapa al grafo de caras

Si G es planar y conexo, su **dual** G* tiene un vértice por cada cara de G, y una arista entre dos vértices de G* por cada arista compartida por las dos caras correspondientes. Si tienes una arista que es puente (sólo bordea una cara), su dual tiene un *loop* (una arista de un vértice a sí mismo).

Propiedades bonitas:
- V(G*) = F(G), E(G*) = E(G), F(G*) = V(G).
- Si G es planar, (G*)* es isomorfo a G (salvo embedding).
- G es bipartito **si y solo si** G* es euleriano (todos los grados pares).
- Si G es 3-regular planar, entonces G* es triangulado.

Aplicaciones: mapas coropletas, redes de flujo, análisis de circuitos eléctricos planos (Kirchhoff usa dualidad planar para resolver mallas).

```
  Grafo G                  Dual G*

   v1 -e1- v2            f1 -e1*- f2
    |  \   |               |       |
   e2  e3  e4             e2*     e4*
    |    \ |               |       |
   v3 -e5- v4            f3 -e5*- f4
```

## 14.9 Detección práctica de K_{3,3} en Rust

Vamos a implementar una heurística simple (no completa, pero ilustrativa) que detecta si un grafo contiene K_{3,3} como subgrafo. Es una simplificación del verdadero test de planaridad.

```rust
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Tipo de grafo: lista de adyacencias con conjuntos.
type Graph = HashMap<usize, HashSet<usize>>;

/// Construye un grafo de prueba (no planar): K_{3,3}.
fn k33() -> Graph {
    let mut g: Graph = HashMap::new();
    for i in 0..6 { g.insert(i, HashSet::new()); }
    // Parte A: {0, 1, 2}, parte B: {3, 4, 5}
    for &a in &[0, 1, 2] {
        for &b in &[3, 4, 5] {
            g.get_mut(&a).unwrap().insert(b);
            g.get_mut(&b).unwrap().insert(a);
        }
    }
    g
}

/// Heurística: ¿contiene K_{3,3} como subgrafo?
/// No detecta subdivisiones completas; sólo el caso "puro".
fn has_k33_subgraph(g: &Graph) -> bool {
    let nodes: Vec<usize> = g.keys().copied().collect();
    // Buscamos 6 vértices (a1, a2, a3) en parte A y (b1, b2, b3) en parte B
    for combo in nodes.iter().combinations(6) {
        let vs: Vec<usize> = combo.iter().map(|&&v| v).collect();
        for split in 1..vs.len() {
            let (a, b) = vs.split_at(split);
            if a.len() != 3 || b.len() != 3 { continue; }
            // Comprobamos que cada a[i] está conectado con cada b[j]
            let mut ok = true;
            'outer: for &ai in a {
                for &bj in b {
                    if !g[&ai].contains(&bj) || !g[&bj].contains(&ai) {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok { return true; }
        }
    }
    false
}

/// Heurística: ¿contiene K_5 como subgrafo?
fn has_k5_subgraph(g: &Graph) -> bool {
    let nodes: Vec<usize> = g.keys().copied().collect();
    for combo in nodes.iter().combinations(5) {
        let vs: Vec<usize> = combo.iter().map(|&&v| v).collect();
        let mut ok = true;
        'outer: for (i, &u) in vs.iter().enumerate() {
            for (j, &v) in vs.iter().enumerate() {
                if i != j && !g[&u].contains(&v) {
                    ok = false;
                    break 'outer;
                }
            }
        }
        if ok { return true; }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k33_es_detectado() {
        let g = k33();
        assert!(has_k33_subgraph(&g));
    }

    #[test]
    fn k4_no_contiene_k33() {
        // K_4: 4 vértices, no contiene K_{3,3}
        let mut g: Graph = (0..4).map(|i| (i, HashSet::new())).collect();
        for i in 0..4 {
            for j in 0..4 {
                if i != j {
                    g.get_mut(&i).unwrap().insert(j);
                }
            }
        }
        assert!(!has_k33_subgraph(&g));
    }

    #[test]
    fn k5_es_detectado() {
        // K_5: 5 vértices, todos conectados con todos
        let mut g: Graph = (0..5).map(|i| (i, HashSet::new())).collect();
        for i in 0..5 {
            for j in 0..5 {
                if i != j {
                    g.get_mut(&i).unwrap().insert(j);
                }
            }
        }
        assert!(has_k5_subgraph(&g));
    }
}
```

Cargo.toml:

```toml
[package]
name = "planaridad"
version = "0.1.0"
edition = "2024"

[dependencies]
itertools = "0.12"
```

Esta heurística es O(n⁶) en el peor caso (combinaciones de 6), pero es muy clara pedagógicamente. Para grafos grandes, `petgraph` tiene `petgraph::algo::is_planar` (o el crate `planar`) y la implementación de Boyer-Myrvold está disponible en C++/Java.

## 14.10 TUI con ratatui: visualizar planaridad

Reutilicemos la técnica del capítulo anterior para mostrar un grafo y su planaridad. Vamos a dibujar dos grafos lado a lado: K_4 (planar) y K_5 (no planar), y debajo de cada uno indicamos si pasa el test.

```rust
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{collections::HashSet, io::stdout};

type Graph = std::collections::HashMap<usize, HashSet<usize>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let k4 = k_graph(4);
    let k5 = k_graph(5);
    // Posiciones absolutas dentro de cada mitad
    let pos4 = vec![(15, 4), (35, 1), (45, 7), (25, 8)];
    let pos5 = vec![(15, 4), (38, 1), (50, 7), (40, 10), (20, 8)];

    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.size());

            draw_graph(f, chunks[0], &k4, &pos4, "K_4 (planar)", true);
            draw_graph(f, chunks[1], &k5, &pos5, "K_5 (NO planar)", false);
        })?;
        if let Event::Key(k) = event::read()? {
            if k.code == KeyCode::Char('q') { break; }
        }
    }
    disable_raw_mode()?;
    Ok(())
}

fn k_graph(n: usize) -> Graph {
    let mut g: Graph = (0..n).map(|i| (i, HashSet::new())).collect();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                g.get_mut(&i).unwrap().insert(j);
            }
        }
    }
    g
}

fn draw_graph(f: &mut Frame, area: Rect, g: &Graph, pos: &[(u16, u16)],
               title: &str, planar: bool) {
    let status = if planar { "[PLANAR ✓]" } else { "[NO PLANAR ✗]" };
    let block = Block::default()
        .title(format!(" {} {}", title, status))
        .borders(Borders::ALL);
    f.render_widget(block, area);

    // Aristas
    for (u, vs) in g {
        for v in vs {
            if u < v {
                let (a, b) = (pos[*u], pos[*v]);
                draw_line(f, a, b, area);
            }
        }
    }
    // Vértices
    for (i, p) in pos.iter().enumerate() {
        let c = if planar { Color::Green } else { Color::Red };
        let s = format!(" {} ", i);
        f.render_widget(
            Paragraph::new(s).style(Style::default().bg(c).fg(Color::Black)),
            Rect::new(area.x + p.0, area.y + p.1, 3, 1),
        );
    }
}

fn draw_line(f: &mut Frame, a: (u16, u16), b: (u16, u16), area: Rect) {
    let (mut x, mut y) = (a.0 as i32, a.1 as i32);
    let (xe, ye) = (b.0 as i32, b.1 as i32);
    let dx = (xe - x).abs();
    let dy = -(ye - y).abs();
    let sx = if x < xe { 1 } else { -1 };
    let sy = if y < ye { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= 0 && y >= 0 {
            let (ux, uy) = (x as u16, y as u16);
            if ux < area.width && uy < area.height {
                f.render_widget(
                    Paragraph::new("·"),
                    Rect::new(area.x + ux, area.y + uy, 1, 1),
                );
            }
        }
        if x == xe && y == ye { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}
```

Cargo.toml:

```toml
[package]
name = "planaridad-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.26"
crossterm = "0.27"
```

## 14.11 Ejercicios resueltos

**Ejercicio 1.** Verifica la fórmula de Euler para K_4.
*S:* K_4 tiene V=4, E=6. Como es maximal planar (cada cara es un triángulo), tiene 4 caras (3 triangulares + 1 externa). 4 - 6 + 4 = 2. ✓ ∎

**Ejercicio 2.** Demuestra que K_{3,3} no es planar.
*S:* K_{3,3} tiene V=6, E=9, girth 4 (es bipartito). Si fuera planar, por la cota sin triángulos, E ≤ 2V - 4 = 8, contradicción con E = 9. ∎

**Ejercicio 3.** Si G es planar conexo con 10 vértices y 25 aristas, ¿es posible?
*S:* E ≤ 3V - 6 = 24, pero E = 25 > 24, así que NO es planar. ∎

## 14.12 Ejercicios propuestos

1. **(F)** Encuentra un grafo no planar con sólo 9 vértices y muestra que contiene K_{3,3} como menor.
2. **(F)** Demuestra que el cubo Q_3 es planar y calcula su dual (¿qué grafo obtienes? Pista: piensa en el octaedro).
3. **(M)** ¿Cuántas caras tiene el dodecaedro (20 vértices, 30 aristas, todas las caras pentagonales)? Aplica Euler.
4. **(M)** Implementa un test naive de planaridad: prueba todas las embeddings de aristas en una cuadrícula y verifica que no se cruzan. Discute su complejidad.
5. **(D)** Investiga el teorema de Fáry: todo grafo planar se puede dibujar con aristas rectas. Busca una demostración y discútela.

## 14.13 Lo que te llevas

- Un grafo es **planar** si admite un dibujo sin cruces. La decisión algorítmica se hace en tiempo lineal.
- La **fórmula de Euler** V - E + F = 2 es la base de toda la teoría de planaridad.
- **Kuratowski**: K_5 y K_{3,3} son los dos únicos "culpables" de la no-planaridad.
- **Boyer-Myrvold** da un test lineal práctico.
- El **4-color theorem** se demostró con ayuda del ordenador (633 casos).

## 14.14 Ojo, cuidado con…

- **Confundir "poco denso" con "planar".** Hay grafos con E ≤ 3V-6 que no son planares (Petersen).
- **Asumir planaridad = simplicidad.** Los multigrafos y grafos con loops tienen sus propias reglas.
- **Olvidar las componentes conexas.** Si G no es conexo, la fórmula de Euler se convierte en V - E + F = C + 1, donde C es el número de componentes.
- **Pensar que Boyer-Myrvold es trivial.** La implementación es delicada; usa la de `petgraph` o `boost`.
- **Olvidar la conexión con el 4-color theorem.** Sin planaridad, χ puede ser arbitrariamente grande. Con planaridad, χ ≤ 4.

## 14.15 Para profundizar

- **Kuratowski (1930)**. *Sur le problème des courbes gauches en topologie*. Fund. Math.
- **Wagner (1937)**. *Über eine Eigenschaft der ebenen Komplexe*. Math. Ann.
- **Appel & Haken (1976)**. *Every planar map is four colorable*. Illinois J. Math.
- **Boyer & Myrvold (2004)**. *On the cutting edge*. J. Graph Algorithms Appl.
- Capítulo 4 de *Graph Theory* (Diestel, 5ª ed., libre en línea).
- Crate `petgraph` + `planar` para planaridad en Rust.

## 14.16 Pin de batalla

- **K_5 y K_{3,3} son los menores prohibidos de planaridad.** Si el grafo los contiene, no es planar.
- **Euler: V - E + F = 2 en conexo planar.** Consecuencia: E ≤ 3V - 6.
- **Boyer-Myrvold para testar planaridad en O(V+E).** Es el algoritmo canónico moderno.
- **Si el grafo es planar, dualidad con flujo.** Planar + bipartito = max-flow = min-cut dual.
- **El 4-color theorem es planar + coloración = 4.** Los mapas siempre se pueden pintar con 4 colores.


## 14.17 Si solo lees 30 segundos

Planar = dibujable sin cruces. K_5 y K_{3,3} son los menores prohibidos. Euler: V - E + F = 2. 4 colores bastan para mapas.

## 14.18 Una historia pequeña

Kazimierz Kuratowski era un matemático polaco en los años 30. Cuando los nazis invadieron Polonia, lo detuvieron y lo mandaron a un campo de concentración en Austria. No tenía papel, ni lápiz, ni libros. Pero su cabeza seguía funcionando. Y demostró el teorema de caracterización de grafos planares mentalmente. Cuando lo liberaron en 1945, publicó su demostración. Años después, en una entrevista, le preguntaron cómo lo había hecho sin papel. "Las matemáticas no necesitan papel. Solo necesitan tiempo y silencio. Yo tenía ambos en abundancia, aunque por las razones equivocadas." El teorema de Kuratowski-Wagner es uno de los más bellos de teoría de grafos, y fue concebido en el peor lugar imaginable.


---

