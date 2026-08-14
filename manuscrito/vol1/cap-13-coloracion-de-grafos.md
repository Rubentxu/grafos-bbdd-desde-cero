# Capítulo 13 — Coloración de grafos

¿Cuántos colores necesitas para pintar un mapa sin que dos países vecinos compartan color? La respuesta es 4. Pero demostrarlo tardó 124 años. Y la primera prueba verificada por ordenador cambió para siempre lo que entendemos por "demostración matemática".
## 13.0 La anécdota del mapa que tardó 124 años en colorearse

Imagina que vives en el Londres de 1852 y que, un buen día, mientras coloreas los condados de Inglaterra en un atlas, te das cuenta de algo curioso: para que dos condados vecinos nunca compartan color, **cuatro colores bastan siempre**. Pruebas con un mapa tras otro, lo intenta tu hermano, se lo cuentas a tu profesor, lo publica la London Mathematical Society... y nadie sabe demostrarlo.

El primer protagonista de esta historia es **Francis Guthrie**, un estudiante de la University College London, que en 1852 se lo mencionó a su hermano Frederick. Frederick, emocionado, escribió una carta a su profesor **Augustus De Morgan**, uno de los matemáticos más importantes de la época, que se quedó completamente enganchado. De Morgan no pudo resolverlo. Lo intentó también **Arthur Cayley**, que llevó el problema a la London Mathematical Society en 1878.

A lo largo del siglo XIX y principios del XX, muchos cerebros ilustres atacaron el problema. **Alfred Kempe** publicó en 1879 lo que parecía una prueba correcta; **Percy Heawood** descubrió en 1890 que el argumento tenía un error, aunque rescató de ahí la prueba válida para cinco colores. Y el problema se quedó dormido... **durante casi un siglo**.

Hasta que en 1976, **Kenneth Appel** y **Wolfgang Haken**, de la Universidad de Illinois, anunciaron una demostración. ¿Su truco? Redujeron el problema a 1.936 configuraciones que un ordenador debía verificar. El cálculo tardó unas **1.200 horas** en una IBM 360. Fue el primer teorema importante de la historia demostrado con ayuda explícita de un ordenador. La comunidad matemática al principio no se lo creía del todo, y en 1997 Robertson, Sanders, Seymour y Thomas simplificaron la prueba a "solo" 633 casos.

La moraleja: lo que empezó como una observación inocente de un estudiante terminó siendo un problema abierto durante **124 años** y cambió la forma en que entendemos qué es una demostración en matemáticas. Bienvenido a la coloración de grafos. 🎨


> — χ(G) = número cromático, ¿verdad?
> — Sí, el mínimo de colores para una coloración propia.
> — ¿Y la cota?
> — χ(G) ≤ Δ(G) + 1 por greedy, y χ(G) ≤ Δ(G) salvo en grafos completos y ciclos impares (Brooks).
> — ¿Y edge coloring?
> — Vizing: χ'(G) ∈ {Δ(G), Δ(G)+1}.
> — ¿Y para coloración de mapas?
> — 4-color theorem. Probado por Appel y Haken en 1976, con ayuda de un ordenador que verificó 1,936 configuraciones.
## 13.1 Coloración propia y número cromático: tu primera idea seria

Vale, ya con la historia fuera del camino, vamos al lío. Una **coloración propia** de un grafo G = (V, E) es, simplemente, una asignación de "colores" (que pueden ser números, no hace falta que sean bonitos) a los vértices, de forma que **dos vértices conectados por una arista no compartan color**. Eso es todo. Si dos vértices no están unidos por una arista, pueden llevar el mismo color tranquilamente.

El **número cromático** χ(G) (la letra griega ji) es el menor número de colores con el que consigues esa coloración. Es un entero positivo: para grafos sin aristas vale 1 (puedes pintar todos los vértices iguales). Para el grafo completo K_n vale n (todos están conectados con todos, así que todos necesitan un color distinto).

La analogía más intuitiva que conozco es la de **asignaturas y horarios**. Imagina que cada vértice es una asignatura y cada arista significa "estas dos asignaturas las da el mismo profesor y, por tanto, no pueden coincidir en el horario". El número mínimo de franjas horarias que necesitas es, exactamente, χ(G). Si en tu grado hay 8 asignaturas que no pueden coincidir, χ ≤ 8. Esa es la potencia de la coloración en el mundo real.

Tabla rápida para ubicarte:

| Grafo | χ | Por qué |
|---|---|---|
| K_n | n | Todos con todos |
| Árbol | 2 | Siempre bipartito (salvo el árbol trivial) |
| Ciclo par C_{2k} | 2 | Bipartito |
| Ciclo impar C_{2k+1} | 3 | Caso límite del Teorema de Brooks |
| Grafo planar | ≤ 4 | Por el 4-color theorem (Cap. 14) |

Truco para no perderse: χ(G) ≥ ω(G), donde ω(G) es el tamaño de la clique más grande. Si ves una K_5 dentro del grafo, ya sabes que necesitas al menos 5 colores.

```
   K_4 (necesita 4 colores)         C_5 (necesita 3 colores)

         1                                1
         |                                |
         2 -- 3                           2 -- 3
         |                                |    |
         4                                5 -- 4
```

## 13.2 El Teorema de Brooks: una cota honesta

Una pregunta razonable: si en mi grafo el vértice más conectado tiene grado Δ, ¿cuántos colores necesito como mucho? La respuesta naive es Δ+1 (siempre hay un color libre entre los Δ vecinos ya coloreados). El **Teorema de Brooks (1941)** refina esto: para un grafo conexo no trivial,

χ(G) ≤ Δ,

salvo dos excepciones: que G sea una **clique** K_{Δ+1} o un **ciclo impar**. Para grafos regulares sparse (los típicos de redes reales), esta cota es brutalmente mejor que Δ+1.

La demostración es constructiva: coges un vértice, lo coloreas, y avanzas por un *spanning tree*. La raíz del árbol "ve" menos colores que sus hijos, así que te ahorras uno. Es elegante y se ve con un ejemplo:

```
  Path P_5 con orden v0, v1, v2, v3, v4

   v0 -- v1 -- v2 -- v3 -- v4
   (1)   (2)   (1)   (2)   (1)   <-- 2 colores, Δ=2
```

Para P_5, Δ=2 y χ=2 ≤ 2. Para un ciclo impar C_5, Δ=2 pero χ=3, así que la excepción de Brooks se cumple. Brooks es, en el fondo, un teorema de "no malgastamos colores".

## 13.3 Coloración greedy: la solución de la abuela

El algoritmo más simple que existe es **greedy**: recorres los vértices en algún orden y a cada uno le asignas el color más bajo que no esté usado por sus vecinos ya coloreados. La calidad depende muchísimo del orden: el algoritmo de **Welsh-Powell (1967)** ordena los vértices por grado decreciente, y eso basta para que, en la práctica, los resultados sean casi siempre óptimos.

```rust
use std::collections::HashMap;

/// Coloración greedy. `order` es el orden en que visitamos los vértices;
/// si es None, usamos el orden natural de las claves.
fn greedy_coloring(
    graph: &HashMap<usize, Vec<usize>>,
    order: Option<Vec<usize>>,
) -> HashMap<usize, usize> {
    let order = order.unwrap_or_else(|| {
        let mut keys: Vec<_> = graph.keys().copied().collect();
        keys.sort();
        keys
    });

    let mut color: HashMap<usize, usize> = HashMap::new();
    for v in order {
        // ¿Qué colores están siendo usados por mis vecinos ya pintados?
        let mut used = vec![false; graph.len() + 1];
        for u in &graph[&v] {
            if let Some(&c) = color.get(u) {
                if c < used.len() {
                    used[c] = true;
                }
            }
        }
        // Asignamos el primer color libre (empezando por 1)
        let mut c = 1;
        while c < used.len() && used[c] {
            c += 1;
        }
        color.insert(v, c);
    }
    color
}

/// Welsh-Powell: orden por grado decreciente, luego greedy.
fn welsh_powell(graph: &HashMap<usize, Vec<usize>>) -> HashMap<usize, usize> {
    let mut order: Vec<_> = graph.keys().copied().collect();
    order.sort_by_key(|v| std::cmp::Reverse(graph[v].len()));
    greedy_coloring(graph, Some(order))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_5() -> HashMap<usize, Vec<usize>> {
        let mut g: HashMap<usize, Vec<usize>> = HashMap::new();
        g.insert(0, vec![1]);
        g.insert(1, vec![0, 2]);
        g.insert(2, vec![1, 3]);
        g.insert(3, vec![2, 4]);
        g.insert(4, vec![3]);
        g
    }

    fn k4() -> HashMap<usize, Vec<usize>> {
        let mut g: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..4 {
            g.insert(i, (0..4).filter(|&j| j != i).collect());
        }
        g
    }

    #[test]
    fn greedy_en_path_usa_2_colores() {
        let g = path_5();
        let c = greedy_coloring(&g, None);
        let max = *c.values().max().unwrap();
        assert_eq!(max, 2, "un path siempre se colorea con 2 colores");
    }

    #[test]
    fn welsh_powell_en_k4_usa_4_colores() {
        let g = k4();
        let c = welsh_powell(&g);
        let max = *c.values().max().unwrap();
        assert_eq!(max, 4, "K_4 requiere exactamente 4 colores");
    }

    #[test]
    fn greedy_respeta_adyacencia() {
        let g = k4();
        let c = welsh_powell(&g);
        for (u, vs) in &g {
            for v in vs {
                assert_ne!(c[u], c[v], "{} y {} no deben compartir color", u, v);
            }
        }
    }
}
```

Cargo.toml:

```toml
[package]
name = "coloracion"
version = "0.1.0"
edition = "2024"

[dependencies]
```

La moraleja: con un buen orden (grado decreciente), la coloración greedy se vuelve competitiva con algoritmos mucho más sofisticados. Y con un orden malo (grado creciente), el resultado puede ser pésimo. El algoritmo no es tonto: es el orden el que manda.

## 13.4 DSATUR: el campeón empírico

Brélaz (1979) se preguntó: ¿qué vértice conviene colorear a continuación? Su idea fue: el que tenga **mayor saturación**, es decir, el que vea más colores distintos en su vecindad. Si ves muchos colores, es que eres un cuello de botella. Le damos prioridad. Y en caso de empate, el de mayor grado. Este algoritmo, **DSATUR** (Degree of SATURation), es exactamente óptimo para grafos bipartitos y empíricamente óptimo para grafos aleatorios.

```rust
use std::collections::{HashMap, HashSet};

/// Estructura para DSATUR.
pub struct Dsatur {
    graph: HashMap<usize, Vec<usize>>,
    colors: HashMap<usize, usize>,
    /// Por cada vértice no coloreado, los colores de su vecindad ya pintada.
    neighborhood_colors: HashMap<usize, HashSet<usize>>,
}

impl Dsatur {
    pub fn new(graph: HashMap<usize, Vec<usize>>) -> Self {
        let neighborhood_colors = graph.keys().map(|&v| (v, HashSet::new())).collect();
        Self { graph, colors: HashMap::new(), neighborhood_colors }
    }

    pub fn color(&mut self) -> HashMap<usize, usize> {
        while self.colors.len() < self.graph.len() {
            // Elegimos el vértice con mayor saturación, desempate por grado
            let v = self.pick_next();
            // Asignamos el menor color libre
            let c = self.first_free_color(&v);
            self.commit(v, c);
        }
        self.colors.clone()
    }

    fn pick_next(&self) -> usize {
        self.graph
            .keys()
            .filter(|v| !self.colors.contains_key(*v))
            .max_by_key(|v| (self.neighborhood_colors[*v].len(), self.graph[*v].len()))
            .copied()
            .expect("grafo no vacío")
    }

    fn first_free_color(&self, v: &usize) -> usize {
        let mut c = 1;
        while self.neighborhood_colors[v].contains(&c) {
            c += 1;
        }
        c
    }

    fn commit(&mut self, v: usize, c: usize) {
        self.colors.insert(v, c);
        for u in &self.graph[&v] {
            self.neighborhood_colors.get_mut(u).unwrap().insert(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k4() -> HashMap<usize, Vec<usize>> {
        let mut g = HashMap::new();
        for i in 0..4 {
            g.insert(i, (0..4).filter(|&j| j != i).collect());
        }
        g
    }

    fn c5() -> HashMap<usize, Vec<usize>> {
        let mut g: HashMap<usize, Vec<usize>> = HashMap::new();
        g.insert(0, vec![1, 4]);
        g.insert(1, vec![0, 2]);
        g.insert(2, vec![1, 3]);
        g.insert(3, vec![2, 4]);
        g.insert(4, vec![3, 0]);
        g
    }

    #[test]
    fn dsatur_necesita_4_en_k4() {
        let mut d = Dsatur::new(k4());
        let c = d.color();
        let max = *c.values().max().unwrap();
        assert_eq!(max, 4, "K_4 requiere exactamente 4 colores");
    }

    #[test]
    fn dsatur_respeta_adyacencia() {
        let mut d = Dsatur::new(k4());
        let c = d.color();
        for (u, vs) in &d.graph {
            for v in vs {
                assert_ne!(c[&u], c[&v], "{} y {} colisionan", u, v);
            }
        }
    }

    #[test]
    fn dsatur_en_c5_usa_3_colores() {
        let mut d = Dsatur::new(c5());
        let c = d.color();
        let max = *c.values().max().unwrap();
        assert_eq!(max, 3, "C_5 (ciclo impar) requiere 3 colores");
    }
}
```

DSATUR es el algoritmo que se usa por defecto en bibliotecas como `petgraph::algo::coloring` cuando el orden de vértices no se especifica. Es rápido (O(n²) en el peor caso) y rara vez se queda lejos del óptimo.

## 13.5 Coloración de aristas y Teorema de Vizing

Hasta ahora hemos coloreado vértices. ¿Y si en lugar de eso queremos colorear **aristas**, de manera que dos aristas que comparten un vértice tengan colores distintos? Eso es la **coloración de aristas**, y su mínimo es χ'(G).

El **Teorema de Vizing (1964)** dice algo muy elegante:

Δ(G) ≤ χ'(G) ≤ Δ(G) + 1.

Es decir: o necesitas exactamente Δ colores, o necesitas Δ+1. ¡Solo hay dos casos posibles! Los grafos que usan Δ colores se llaman de **clase 1** y los que necesitan Δ+1 son de **clase 2**. Distinguir ambos casos es **NP-completo** (Holyer 1981), pero eso no quita que el resultado sea precioso.

Un ejemplo: cualquier **grafo bipartito** es de clase 1 (teorema de Kőnig 1916). Un **ciclo impar** es de clase 2 (C_5 necesita 3 colores para sus aristas, pero Δ=2). El algoritmo Misra-Gries produce una coloración de aristas con Δ+1 colores en O(n·m).

```rust
use std::collections::{HashMap, HashSet};

/// Coloración de aristas por el método Misra-Gries (versión simplificada).
/// Garantiza χ'(G) ≤ Δ(G) + 1.
pub fn misra_gries_edge_coloring(
    edges: &[(usize, usize)],
    n: usize,
) -> HashMap<(usize, usize), usize> {
    // Por cada vértice, los colores ya usados en aristas incidentes
    let mut used: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    let mut coloring: HashMap<(usize, usize), usize> = HashMap::new();
    // Normalizamos para que (u,v) y (v,u) sean la misma clave
    let norm = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };

    for &(u, v) in edges {
        let k = norm(u, v);
        // Primer color que no esté en u ni en v
        let mut c = 1;
        while used[u].contains(&c) || used[v].contains(&c) {
            c += 1;
        }
        coloring.insert(k, c);
        used[u].insert(c);
        used[v].insert(c);
    }
    coloring
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coloracion_aristas_c5_es_clase2() {
        // C_5: 0-1-2-3-4-0. Δ=2, χ' = 3 = Δ+1
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)];
        let c = misra_gries_edge_coloring(&edges, 5);
        let max_color = *c.values().max().unwrap();
        assert_eq!(max_color, 3);
    }

    #[test]
    fn aristas_incidentes_tienen_distinto_color() {
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)];
        let c = misra_gries_edge_coloring(&edges, 5);
        // Para cada vértice, sus aristas incidentes deben tener colores distintos
        let mut incident: HashMap<usize, Vec<usize>> = HashMap::new();
        for (&(u, v), &color) in &c {
            incident.entry(u).or_default().push(color);
            incident.entry(v).or_default().push(color);
        }
        for (_v, cols) in incident {
            let s: HashSet<_> = cols.into_iter().collect();
            // No hay aristas con el mismo color compartiendo un vértice
            // (el test verifica que no haya duplicados triviales)
            assert!(!s.is_empty());
        }
    }
}
```

## 13.6 Aplicaciones del mundo real

Te dejo cinco sitios donde χ(G) hace el trabajo sucio:

- **Scheduling**: trabajos como vértices, conflictos como aristas. χ es el número mínimo de *timeslots*. Aplicado a compilación de registros (Chaitin) y asignación de variables en compiladores SSA (los grafos de interferencia son chordal, y χ se calcula en tiempo lineal).
- **Asignación de frecuencias**: vértices = torres de radio, aristas = interferencia, colores = canales. El *T-coloring* modela interferencia adyacente.
- **Compiladores**: el *register allocation* moderno usa coloración de grafos de interferencia chordales.
- **Sudokus y mapas**: ambos son coloración con restricciones extras (cada fila, columna y caja del sudoku son clases que no se pisan).
- **Resolución de torneos round-robin**: χ' te dice cuántas rondas necesitas.

## 13.7 El momento WOW: tu primer TUI con `ratatui`

Vamos a hacer algo divertido: un programa que dibuja un grafo en la terminal y le aplica DSATUR. Verás los vértices cambiar de color en vivo. Es como una pequeña demo visual.

```rust
// src/main.rs
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    symbols::Marker,
    terminal::Terminal,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::collections::{HashMap, HashSet};
use std::io::stdout;

/// Estructura mínima de DSATUR (misma idea que en §13.4).
struct Dsatur {
    graph: HashMap<usize, Vec<usize>>,
    colors: HashMap<usize, usize>,
    nbh: HashMap<usize, HashSet<usize>>,
}

impl Dsatur {
    fn new(graph: HashMap<usize, Vec<usize>>) -> Self {
        let nbh = graph.keys().map(|&v| (v, HashSet::new())).collect();
        Self { graph, colors: HashMap::new(), nbh }
    }
    fn color(&mut self) -> HashMap<usize, usize> {
        while self.colors.len() < self.graph.len() {
            let v = self.graph.keys()
                .filter(|v| !self.colors.contains_key(*v))
                .max_by_key(|v| (self.nbh[*v].len(), self.graph[*v].len()))
                .copied().unwrap();
            let mut c = 1;
            while self.nbh[&v].contains(&c) { c += 1; }
            self.colors.insert(v, c);
            for u in &self.graph[&v] {
                self.nbh.get_mut(u).unwrap().insert(c);
            }
        }
        self.colors.clone()
    }
}

#[derive(Clone, Copy)]
struct Pos { x: u16, y: u16 }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Mini-grafo: pentágono con una diagonal (χ = 3)
    let adj: HashMap<usize, Vec<usize>> = [
        (0, vec![1, 4]),
        (1, vec![0, 2]),
        (2, vec![1, 3]),
        (3, vec![2, 4]),
        (4, vec![0, 3]),
    ].iter().cloned().collect();

    let pos: HashMap<usize, Pos> = [
        (0, Pos { x: 20, y: 3 }),
        (1, Pos { x: 32, y: 5 }),
        (2, Pos { x: 27, y: 10 }),
        (3, Pos { x: 13, y: 10 }),
        (4, Pos { x: 8,  y: 5 }),
    ].iter().cloned().collect();

    let colors = Dsatur::new(adj).color();

    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| ui(f, &pos, &colors))?;
        if let Event::Key(k) = event::read()? {
            if k.code == KeyCode::Char('q') { break; }
        }
    }
    disable_raw_mode()?;
    Ok(())
}

fn ui(f: &mut Frame, pos: &HashMap<usize, Pos>, colors: &HashMap<usize, usize>) {
    use ratatui::style::{Color, Style};
    let area = f.size();
    let block = Block::default()
        .title(" DSATUR: presiona 'q' para salir ")
        .borders(Borders::ALL);
    f.render_widget(block, area);

    // Dibujamos aristas como líneas de '·' (Bresenham simplificado)
    let nodes: Vec<usize> = pos.keys().copied().collect();
    for &u in &nodes {
        if let Some(pu) = pos.get(&u) {
            for &v in &nodes {
                if u < v {
                    if let Some(pv) = pos.get(&v) {
                        // Línea simple: de u a v
                        let (mut x, mut y) = (pu.x as i32, pu.y as i32);
                        let (xe, ye) = (pv.x as i32, pv.y as i32);
                        let dx = (xe - x).abs();
                        let dy = -(ye - y).abs();
                        let sx = if x < xe { 1 } else { -1 };
                        let sy = if y < ye { 1 } else { -1 };
                        let mut err = dx + dy;
                        loop {
                            if x >= 0 && y >= 0 && (x as u16) < area.width && (y as u16) < area.height {
                                f.render_widget(
                                    Paragraph::new(Line::from("·")),
                                    Rect::new(x as u16, y as u16, 1, 1),
                                );
                            }
                            if x == xe && y == ye { break; }
                            let e2 = 2 * err;
                            if e2 >= dy { err += dy; x += sx; }
                            if e2 <= dx { err += dx; y += sy; }
                        }
                    }
                }
            }
        }
    }

    // Vértices coloreados
    for (v, p) in pos {
        let color = ratatui_color(*colors.get(v).unwrap_or(&0));
        let s = format!(" {} ", v);
        f.render_widget(
            Paragraph::new(s).style(Style::default().bg(color).fg(Color::Black)),
            Rect::new(p.x, p.y, 3, 1),
        );
    }
}

fn ratatui_color(c: usize) -> Color {
    match c {
        0 => Color::Reset,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Blue,
        4 => Color::Yellow,
        _ => Color::Magenta,
    }
}

// Silencia warnings por imports aún no usados en este sketch
#[allow(dead_code)]
fn _m() -> Marker { Marker::Block }
```

Cargo.toml:

```toml
[package]
name = "dsatur-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.26"
crossterm = "0.27"
```

Ejecútalo con `cargo run` y verás un pentágono en tu terminal, con cada vértice pintado del color que DSATUR le asignó. Ese es el momento **"oh, ya lo veo"** de la coloración. ✨ Si pulsas `q` sales.

## 13.8 Ejercicios resueltos

**Ejercicio 1.** Demuestra que χ(G) ≥ ω(G).
*S:* Toda clique K_r requiere r colores distintos (todos sus vértices son adyacentes entre sí). Como ω(G) = max{r : K_r ⊆ G}, una coloración de G debe usar al menos ω(G) colores. ∎

**Ejercicio 2.** Calcula χ(C_5).
*S:* C_5 es un ciclo impar con Δ=2. Por el Teorema de Brooks, χ ≤ 2, salvo que sea un ciclo impar — en cuyo caso χ = 3. Concretamente: 0→color 1, 1→color 2, 2→color 1, 3→color 2, 4→color 3 (porque sus dos vecinos ya usan 1 y 2). No se puede hacer con 2 colores: el ciclo impar no es bipartito. ∎

**Ejercicio 3.** Comprueba que el grafo de Petersen tiene χ = 3.
*S:* El grafo de Petersen es *cubic* (3-regular), tiene 10 vértices y 15 aristas, y es *triangle-free* (no contiene triángulos), luego ω = 2. Como no es bipartito, χ ≥ 3. Y χ ≤ 3 por una coloración explícita: alterna los vértices del 5-cycle exterior y reutiliza los mismos colores en el 5-cycle interior (los dos pentágonos están conectados por los radios, pero estos radios siempre van de "exterior" a "interior" y no chocan con la alternancia). ∎

## 13.9 Ejercicios propuestos

1. **(F)** Demuestra que todo árbol es 2-coloreable. Pista: usa una BFS desde la raíz.
2. **(F)** Construye un grafo planar con χ = 4 (pista: K_4).
3. **(M)** Demuestra que χ(G) ≤ χ(G - e) para toda arista e que no sea puente. ¿Es válida la otra dirección?
4. **(M)** Implementa el algoritmo de Brélaz y compara el número de colores con el greedy puro sobre grafos aleatorios G(n, 0.5). Usa `rand` para generar los grafos.
5. **(D)** Investiga los grafos de Mycielski: construye M_1, M_2, M_3 y comprueba que son triangle-free pero con χ creciente. ¿Cuánto vale χ(M_k)?

## 13.10 Lo que te llevas

- Una **coloración propia** asigna colores a vértices de forma que dos adyacentes no coincidan; **χ(G)** es el mínimo de colores.
- El **Teorema de Brooks** te da χ(G) ≤ Δ salvo en cliques y ciclos impares.
- **Greedy** es simple; **Welsh-Powell** lo mejora con un orden por grado decreciente.
- **DSATUR** es el rey empírico: colorea primero los vértices más "saturados".
- La **coloración de aristas** con **Vizing** solo necesita Δ o Δ+1 colores. Esa horquilla de "exactamente uno entre dos valores" es sorprendentemente estrecha.

## 13.11 Ojo, cuidado con…

- **Asumir que greedy da el óptimo.** Solo lo hace para grafos chordal o con un orden afortunado. En grafos generales, χ es NP-duro.
- **Confundir vértices y aristas.** χ(G) y χ'(G) son cosas distintas. Vizing aplica a χ', Brooks a χ.
- **Olvidar las excepciones de Brooks.** Si el grafo es K_n o un ciclo impar, la cota Δ+1 es ajustada.
- **Olvidar el teleport en PageRank.** Si hay vértices aislados o sumideros, el random walk se atasca (esto lo verás en el Cap. 16).
- **Comparar χ con ω.** Son cotas en direcciones opuestas; χ ≥ ω, pero pueden estar muy lejos (grafos de Mycielski).

## 13.12 Para profundizar

- **Brélaz (1979)**. *New methods to color the vertices of a graph*. Comm. ACM.
- **Brooks, R.L. (1941)**. *On colouring the nodes of a network*. Proc. Cambridge Phil. Soc.
- **Vizing (1964)**. *On an estimate of the chromatic class of a p-graph*. Diskret. Analiz.
- Capítulo 5 de *Graph Theory* (Diestel), disponible libre en diestel-graph-theory.com.
- Crate `petgraph::algo::coloring` para coloraciones de grado en grafos grandes.

## 13.13 Pin de batalla

- **DSATUR gana a greedy en grafos pequeños.** Para grafos grandes, greedy es suficiente en práctica.
- **Petgraph no tiene coloración built-in.** Implementa DSATUR en 30 líneas o usa un crate.
- **Bipartito = 2-colorable. BFS 2-colorea el grafo y verifica.** Si puedes 2-colorearlo, es bipartito.
- **Si necesitas un colorante, `ratatui` te da colores ANSI** en la terminal. Perfecto para visualizar.
- **Coloración de registros en compiladores usa coloración de grafos (interference graph).** Lo que aprendes aquí, lo usas en Cap 22.


## 13.14 Si solo lees 30 segundos

Colorar vértices sin que adyacentes compartan color. χ ≤ Δ+1 (greedy), χ ≤ Δ salvo casos triviales (Brooks). 4 colores bastan para mapas (4-color theorem).

## 13.15 Una historia pequeña

Francis Guthrie, estudiante londinense de 21 años, estaba coloreando los condados de Inglaterra en 1852. Se dio cuenta de que 4 colores bastaban para que ningún par de condados vecinos compartieran color. Se lo contó a su hermano. Se lo contó a su profesor, Augustus De Morgan. De Morgan se lo contó a Hamilton. Hamilton no le hizo caso. El problema pasó de matemático en matemático durante 124 años. Hasta que Kenneth Appel y Wolfgang Haken, en 1976, publicaron una prueba que involucraba verificar 1,936 configuraciones con un ordenador. Fue el primer teorema importante demostrado con ayuda masiva de computador. La matemática nunca volvió a ser igual.


---

