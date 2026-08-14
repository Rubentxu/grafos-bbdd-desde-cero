# Capítulo 29 — Grafos en Robótica y Videojuegos

**[HOOK]** Hay un fantasma amarillo en un laberinto que persigue a un comecocos. Para decidir a dónde ir, no piensa: calcula. Y el algoritmo que usa, versiones modernas, es el mismo que guía robots aspiradores, coches autónomos y brazos de fábrica. Los fantasmas de Pac-Man, los humanoides de Boston Dynamics, los personajes de tu videojuego favorito: todos caminan por grafos. Algunos con ruedas, otros con render. Misma matemática.

## 29.0 La anécdota de la esquina

En mil novecientos ochenta, en una sala de máquinas en Tokio, un diseñador de juegos llamado **Toru Iwatani** creó **Pac-Man**. Los cuatro fantasmas del juego — Blinky, Pinky, Inky y Clyde — recorrían un laberinto predefinido. Cada celda del laberinto era, implícitamente, un nodo. Cada conexión entre celdas, una arista. Iwatani no lo llamó "grafo", pero los fantasmas resolvían, en tiempo real, variantes del problema del camino más corto.

Mientras tanto, al otro lado del charco, en California, **Peter Hart**, **Nils Nilsson** y **Bertram Raphael** publicaban un paper de 1968 que cambiaría la robótica: el algoritmo **A\***. Lo usaron para que un robot llamado *Shakey* (sí, se llamaba Shakey, le temblaban las ruedas) navegara por una habitación con cajas. La gracia de A\* era combinar el coste real del camino con una *heurística* — una corazonada matemática sobre cuánto falta. La idea era vieja (Dijkstra, 1959), pero la heurística bien elegida hacía que A\* fuera órdenes de magnitud más rápido.

Décadas después, los dos mundos colisionaron. Los juegos adoptaron A\* (y sus variantes JPS, HPA\*) para pathfinding; los robots adoptaron A\* y sus primos D\*, D\* Lite, anytime repairable A\*. Hoy, un fantasma de Pac-Man técnicamente sofisticado usa A\* sobre un grafo de grid. Un robot aspirador usa D\* Lite. Mismo algoritmo, distinto disfraz.

## 29.1 Configuration space: el truco del origami

Antes de mover un robot, los ingenieros definen su **espacio de configuraciones** (C-space): el conjunto de todas las posiciones y orientaciones posibles. Para un brazo robótico de 6 articulaciones, es un espacio 6-dimensional. Para un coche, son 3 dimensiones (x, y, ángulo). Para un punto en un plano 2D, es simplemente el plano.

El truco: cada punto del C-space es un nodo. Los puntos alcanzables están conectados por aristas. Los obstáculos del mundo real se transforman en regiones prohibidas del C-space (crecen o se encogen según la forma del robot). El path planning se convierte en búsqueda de camino en un grafo, como siempre.

```
   Mundo real:        C-space:

   . . . . .          . . . . .
   . R . O .          . R . . .
   . . . . .          . . . O .
   . O . . .          . O . . .
   . . . G .          . . . . G
```

Donde R = robot, O = obstáculo, G = meta. La forma exacta del obstáculo "crece" en C-space para absorber el cuerpo del robot. La planificación se hace en C-space, donde el robot es un punto. Magia.

## 29.2 A* y los robots: la heurística lo cambia todo

**A\*** no es más que Dijkstra con un sesgo: una función heurística `h(n)` que estima cuánto falta para llegar a la meta. La función de evaluación es `f(n) = g(n) + h(n)`, donde `g(n)` es el coste real desde el inicio.

```
   f(n) = g(n) + h(n)
          │       │
          │       └→ ¿cuánto me queda?
          └→ ¿cuánto llevo?
```

Si la heurística es admisible (nunca sobreestima el coste real), A\* garantiza optimalidad. Si además es consistente, es óptimamente eficiente en el sentido de que no expande nodos innecesarios.

En robótica, las heurísticas favoritas son:
- **Distancia euclídea**: para espacios métricos.
- **Distancia Manhattan**: para grids 4-conectados.
- **Octile distance**: para grids 8-conectados.

Para brazos robóticos, las heurísticas son más exóticas (índice de manipulabilidad, distancia de RRT, etc.). El truco común: si tu heurística es buena, A\* es rapidísimo. Si es mala, es un Dijkstra con prurito.

## 29.3 D* y D* Lite: cuando el mundo cambia

A\* asume que el mapa es fijo. Pero los robots reales descubren cosas mientras se mueven: una puerta cerrada, una silla nueva, un charco. **D\*** (Dynamic A\*) replanifica incrementalmente: aprovecha el plan anterior y solo recalcula lo que cambió.

**D\* Lite** (Koenig & Likhachev, 2002) hace lo mismo pero de manera más limpia, usando el reverso del algoritmo: en vez de planificar desde el inicio, planifica desde la meta. Cuando descubre un obstáculo, "propaga" el cambio hacia atrás en el grafo.

```
   Estado inicial:  Plan completo A* → meta
   Descubrimiento:  "Esta arista está bloqueada"
   Replanificación: solo cambia los nodos aguas abajo
```

Los robots de Marte usaron D\* Lite. Los aspiradores Roomba usan variantes similares (más simples, claro, son aspiradoras, no rovers).

## 29.4 RRT: cuando el espacio es enorme

En dimensiones altas (brazos robóticos, manos,humanoides), A\* sobre un grid es inviable: el número de nodos explota exponencialmente. **RRT (Rapidly-exploring Random Tree)** resuelve esto con una idea casi absurda: muestrea puntos al azar en el C-space y los conecta al árbol más cercano, si no chocan.

```
   ██████████       ████
   ██  *───██───────██
   ██      ██  *    ██
   ██  *   ██  *    ██
   ████████  ██  *  ██
              ██─────██ meta
              ██
              ██
            inicio
```

En pocas iteraciones, RRT cubre el espacio libre con una "hiedra" ramificada. No garantiza optimalidad, pero da una solución rápida. Para optimalidad, RRT* añade rewire: después de añadir un nodo, mira a sus vecinos y reordena las conexiones si encuentra un camino más corto.

## 29.5 PRM: muestrear primero, planificar después

**PRM (Probabilistic Roadmap)** es la otra estrategia: en vez de crecer un árbol, muestras puntos al azar y los conectas entre sí si hay línea recta sin obstáculos. El resultado es un grafo "carretera" del C-space. Después, planificas sobre ese grafo con Dijkstra o A\*.

```
   Muestreo:         Conexión:

   .   .  *          .   .  *
    *  .     .        *──.     .
   .   *  .   .      .   *  .   .
     .     *  .        .     *──.
   .  *  .    *      .  *  .    *
                  ruta óptima: *──*──*──*
```

PRM es bueno cuando vas a hacer muchas consultas en el mismo mapa. RRT es bueno cuando solo necesitas un camino y rápido. Como siempre en robótica: depende.

## 29.6 Pathfinding en videojuegos: A* y compañía

Los videojuegos son el otro hogar de estos algoritmos. Cada NPC que "piensa" adónde ir usa alguno. Las variantes favoritas:

- **A\* sobre grid**: el clásico. Cada celda del mapa es un nodo.
- **JPS (Jump Point Search)**: optimiza A\* en grids uniformes saltando sobre nodos simétricos. Hasta 10x más rápido.
- **HPA\*** (Hierarchical Path-Finding A\*): subdivide el mapa en clusters y planifica entre clusters, no entre celdas. Esencial para mapas grandes.
- **Flow Fields**: para mover cientos de unidades a la vez, no un solo agente. Cada celda del mapa tiene un vector de dirección.

La elección depende del juego. Para un RTS con mil unidades, Flow Fields. Para un RPG con un protagonista y muchos enemigos, A\* con caché. Para un MMO con dungeons procedurales, HPA\* recocinado cada vez que cambia el mapa.

## 29.7 Game trees: cuando el adversario también piensa

Hasta ahora, un agente se movía solo. Pero ¿qué pasa si hay un adversario? Ajedrez, Go, tres en raya: las decisiones se modelan como un **árbol de juego**.

```
                    ¿Mi movida?
                   /     |     \
                mov1   mov2   mov3
                /  \    |
         ¿Su movida?   ...
         /    |    \
      res1  res2  res3
       ⋮     ⋮     ⋮
```

Cada nodo es un estado del juego. Cada nivel alterna jugador Max y jugador Min. **Minimax** recorre el árbol y elige la mejor jugada asumiendo que el adversario juega óptimo.

**Alpha-beta pruning** ahorra trabajo: si ya encontraste una jugada que garantiza un resultado mejor de lo que Min puede evitar, no explores más esa rama.

```
   Sin poda:        Con alpha-beta:
   ⋮ 14 nodos        ⋮ 7 nodos
```

En la práctica, alpha-beta permite buscar 2x más profundo con el mismo tiempo. En ajedrez, eso es la diferencia entre un amateur y un maestro.

## 29.8 MCTS y el día que AlphaGo venció a Lee Sedol

Pero hay juegos donde el árbol de juego es **inabordable**. En Go, hay más posiciones que átomos en el universo observable. Minimax no sirve. Aquí entra **MCTS (Monte Carlo Tree Search)**.

La idea, elegante como pocas: en vez de explorar todo el árbol, simulas jugadas aleatorias hasta el final, contando victorias. Las jugadas que ganan más simulaciones se exploran más. El árbol crece sesgado hacia las líneas prometedoras.

```
   Selección:        Expansión:
   ┌──a──┐           ┌──a──┐
   │  3/5│   →       │  3/5│──b (nuevo)
   └──b──┘           └──b──┘
       │                  │
   Simulación:        Backpropagation:
   juega al azar       ┌──a──┐
   hasta el final      │  4/6│
   gana o pierde       └──b──┘
                          │
                         1/1
```

**AlphaGo** (DeepMind, 2016) combinó MCTS con redes neuronales profundas: una red "policy" proponía jugadas, una red "value" evaluaba posiciones, y MCTS las integraba. En marzo de 2016, AlphaGo venció a **Lee Sedol**, campeón mundial de Go, por 4-1. Fue la primera vez que una máquina venció a un humano顶级 (顶级) en Go sin handicaps.

Detalle pop culture: en el movimiento 37 del segundo juego, AlphaGo hizo una jugada que ningún humano habría hecho. Los comentaristas se rieron al principio. Cinco minutos después, se quedaron en silencio. Esa jugada se conoce hoy como "God Move" o "Move 37" y se estudia en academias de Go de todo el mundo.

## 29.9 Behavior trees: los árboles de la IA de juegos

Para personajes no jugadores (NPCs), los **behavior trees** son la opción más popular. Son árboles donde las hojas son acciones (atacar, huir, esperar) y los nodos internos son operadores de control:

- **Selector** (OR): prueba hijos en orden hasta que uno tenga éxito.
- **Sequence** (AND): ejecuta hijos en orden; si uno falla, aborta.
- **Decorator**: modifica el comportamiento de un hijo (invertir, repetir, etc.).

```
   Selector
   ├── ¿Hay enemigo? ──→ Sequence
   │                       ├── ¿Tengo balas? ──→ Disparar
   │                       └── Apuntar
   ├── ¿Estoy herido? ──→ Curarse
   └── Patrullar
```

Formalmente, un behavior tree es un grafo acíclico con un tipo particular de aristas: las de retorno (tick, success, failure, running). Los motores modernos (Unreal, Godot) los soportan de fábrica. Y, otra vez, todo es un grafo.

## 29.10 Mini-diálogo: en la cocina, después de cenar

—Papá, ¿los robots sueñan con grafos eléctricos?

—Algo así, Lucía. Cada vez que un robot decide moverse, está eligiendo un camino en un grafo. Su mapa mental es un grafo. Sus rutas, un grafo. Sus decisiones, otro grafo.

—¿Y los fantasmas de Pac-Man?

—También. Solo que su grafo es muy pequeño: las celdas del laberinto. Y usan A\* con heurísticas muy simples: a veces van directo a Pac-Man, a veces se alejan. Lo que les hace difíciles es que se turnan: Blinky persigue, Pinky embosca, Inky flanquea, Clyde parece tonto a propósito.

—¿Clyde es tonto?

—Clyde es el que más me gusta. Cuando se acerca demasiado a Pac-Man, decide alejarse. Es el único fantasma con un criterio propio.

—¿Y los humanoides?

—Esos usan RRT y comportamiento basado en optimización. Combinan varios grafos: el del mapa, el de las trayectorias, el de los obstáculos dinámicos, el de los demás robots. Es un grafo de grafos.

—Qué cansado.

—Sí. Por eso los robots no bostezan. Aún.

## 29.11 Implementación Rust: A* en un grid 2D

```rust
// Cargo.toml:
// [dependencies]
// petgraph = "0.6"

use petgraph::graph::DiGraph;
use petgraph::algo::astar;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Celda del grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Cell { x: i32, y: i32 }

/// Nodo con prioridad para A*.
#[derive(Clone, Eq, PartialEq)]
struct OpenNode {
    f: i32,           // f = g + h
    cell: Cell,
}

impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: invertimos.
        other.f.cmp(&self.f)
    }
}

/// A* sobre un grid 2D con obstáculos.
/// 0 = libre, 1 = obstáculo.
pub fn astar_grid(grid: &[Vec<u8>], start: Cell, goal: Cell) -> Option<(i32, Vec<Cell>)> {
    let rows = grid.len() as i32;
    let cols = grid[0].len() as i32;

    let heuristic = |c: Cell| {
        // Distancia Manhattan (admisible para 4-vecinos).
        (c.x - goal.x).abs() + (c.y - goal.y).abs()
    };

    let mut open: BinaryHeap<OpenNode> = BinaryHeap::new();
    let mut g_score: std::collections::HashMap<Cell, i32> = std::collections::HashMap::new();
    let mut came_from: std::collections::HashMap<Cell, Cell> = std::collections::HashMap::new();

    g_score.insert(start, 0);
    open.push(OpenNode { f: heuristic(start), cell: start });

    // Movimientos en 4-vecinos
    let dirs = [(1,0),(-1,0),(0,1),(0,-1)];

    while let Some(OpenNode { cell: current, .. }) = open.pop() {
        if current == goal {
            // Reconstruimos el camino.
            let mut path = vec![current];
            let mut c = current;
            while let Some(&prev) = came_from.get(&c) {
                path.push(prev);
                c = prev;
            }
            path.reverse();
            return Some((g_score[&current], path));
        }

        for (dx, dy) in dirs {
            let nx = current.x + dx;
            let ny = current.y + dy;
            if nx < 0 || ny < 0 || nx >= cols || ny >= rows { continue; }
            if grid[ny as usize][nx as usize] == 1 { continue; }
            let neighbor = Cell { x: nx, y: ny };
            let tentative_g = g_score[&current] + 1;
            if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative_g);
                let f = tentative_g + heuristic(neighbor);
                open.push(OpenNode { f, cell: neighbor });
            }
        }
    }
    None
}

fn visualize(grid: &[Vec<u8>], path: &[Cell]) {
    let mut display = grid.to_vec();
    for c in path {
        display[c.y as usize][c.x as usize] = 2; // marca el path
    }
    for row in display {
        for v in row {
            print!("{}", match v {
                0 => ". ",
                1 => "██",
                2 => "::",
                _ => "? ",
            });
        }
        println!();
    }
}

fn main() {
    // 0 libre, 1 muro
    let grid = vec![
        vec![0,0,0,0,0,0,0,0,0,0],
        vec![0,1,1,1,0,1,1,1,1,0],
        vec![0,0,0,1,0,0,0,0,1,0],
        vec![0,1,0,0,0,1,1,0,0,0],
        vec![0,1,1,1,1,1,0,1,1,0],
        vec![0,0,0,0,0,0,0,0,0,0],
    ];

    let start = Cell { x: 0, y: 0 };
    let goal  = Cell { x: 9, y: 5 };

    if let Some((cost, path)) = astar_grid(&grid, start, goal) {
        println!("Camino encontrado con coste {}", cost);
        println!("Path: {:?}", path);
        visualize(&grid, &path);
    } else {
        println!("Sin camino :(");
    }
}
```

Salida (ejemplo):

```
.  .  .  .  .  .  .  .  .  .
.  ██ ██ ██ .  ██ ██ ██ ██ .
.  ::.  .  ██ .  ::.  .  ██ .
.  ██ ::.  ::.  ██ ██ ::.  .
.  ██ ██ ██ ██ ██ .  ██ ██ .
.  .  .  .  .  .  .  .  .  .
```

(Los `::` marcan el camino óptimo.)

## 29.12 Bonus: MCTS en 3 en raya

```rust
// Cargo.toml:
// [dependencies]
// rand = "0.8"

use rand::Rng;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Player { X, O, None }

#[derive(Clone)]
struct State {
    board: [Player; 9],
    current: Player,
}

impl State {
    fn new() -> Self {
        State {
            board: [Player::None; 9],
            current: Player::X,
        }
    }

    fn winner(&self) -> Option<Player> {
        let lines = [
            (0,1,2),(3,4,5),(6,7,8),
            (0,3,6),(1,4,7),(2,5,8),
            (0,4,8),(2,4,6),
        ];
        for (a,b,c) in lines {
            if self.board[a] != Player::None
                && self.board[a] == self.board[b]
                && self.board[a] == self.board[c]
            {
                return Some(self.board[a]);
            }
        }
        None
    }

    fn is_terminal(&self) -> bool {
        self.winner().is_some() || self.board.iter().all(|&c| c != Player::None)
    }

    fn legal_moves(&self) -> Vec<usize> {
        self.board.iter().enumerate()
            .filter_map(|(i, &c)| if c == Player::None { Some(i) } else { None })
            .collect()
    }

    fn apply(&self, mv: usize) -> State {
        let mut s = self.clone();
        s.board[mv] = s.current;
        s.current = match s.current {
            Player::X => Player::O,
            Player::O => Player::X,
            _ => Player::None,
        };
        s
    }
}

/// MCTS: 4 fases: selección, expansión, simulación, backprop.
fn mcts(root: &State, iters: usize) -> usize {
    let mut rng = rand::thread_rng();
    // stats: (visitas, victorias) por nodo
    let mut stats: HashMap<State, (i32, f32)> = HashMap::new();

    for _ in 0..iters {
        let mut node = root.clone();
        let mut path = vec![];

        // Selección + Expansión
        while !node.is_terminal() {
            let moves = node.legal_moves();
            let unexplored: Vec<State> = moves.iter()
                .map(|&m| node.apply(m))
                .filter(|s| !stats.contains_key(s))
                .collect();
            if !unexplored.is_empty() {
                let pick = unexplored[rng.gen_range(0..unexplored.len())].clone();
                path.push(pick.clone());
                node = pick;
                break;
            } else {
                // Elegir el hijo con mejor UCB1
                let total: i32 = stats.values().map(|(v, _)| v).sum();
                let best = moves.iter().max_by_key(|&&m| {
                    let s = &stats[&node.apply(m)];
                    let v = s.0 as f32;
                    let w = s.1;
                    // UCB1
                    ((w / v) + (2.0 * (total as f32).ln() / v).sqrt()) as i32
                }).copied().unwrap();
                let next = node.apply(best);
                path.push(next.clone());
                node = next;
            }
        }

        // Simulación (random playout)
        let mut sim = node.clone();
        while !sim.is_terminal() {
            let moves = sim.legal_moves();
            if moves.is_empty() { break; }
            let mv = moves[rng.gen_range(0..moves.len())];
            sim = sim.apply(mv);
        }

        // Resultado
        let result = match sim.winner() {
            Some(p) if p == root.current => 1.0,
            Some(_) => 0.0,
            None => 0.5,
        };

        // Backpropagation
        for s in path {
            let entry = stats.entry(s).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += result;
        }
    }

    // Elegir el movimiento más visitado
    let moves = root.legal_moves();
    *moves.iter().max_by_key(|&&m| stats.get(&root.apply(m)).unwrap_or(&(0, 0.0)).0).unwrap()
}

fn render(state: &State) {
    for i in 0..3 {
        for j in 0..3 {
            let c = match state.board[i*3 + j] {
                Player::X => 'X',
                Player::O => 'O',
                Player::None => '.',
            };
            print!("{} ", c);
        }
        println!();
    }
}

fn main() {
    let mut state = State::new();
    while !state.is_terminal() {
        render(&state);
        let mv = if state.current == Player::X {
            // Computadora con MCTS
            mcts(&state, 5000)
        } else {
            // Humano (en este ejemplo, primera libre)
            state.legal_moves()[0]
        };
        state = state.apply(mv);
    }
    render(&state);
    match state.winner() {
        Some(Player::X) => println!("X gana!"),
        Some(Player::O) => println!("O gana!"),
        None => println!("Empate!"),
    }
}
```

En 5.000 iteraciones, este MCTS empieza a jugar un tres en raya bastante decente. Con 50.000, casi no pierde. Con 500.000, perfecto. La magia: cada iteración es una búsqueda parcial en el árbol de juego, sesgada por lo aprendido. Sin heurísticas explícitas, sin reglas escritas. Solo muestras, estadística y retroceso.

## 29.13 Ejercicios resueltos

**Ejercicio 1.** Sobre un grid 4×4 sin obstáculos, encuentra el camino más corto de (0,0) a (3,3) con A\* y heurística Manhattan.

*Respuesta:* coste 6, camino `[(0,0), (1,0), (2,0), (3,0), (3,1), (3,2), (3,3)]`. La heurística coincide exactamente con el coste real, así que A\* se comporta como BFS.

**Ejercicio 2.** ¿Por qué A\* con heurística admisible garantiza optimalidad?

*Respuesta:* porque nunca sobreestima. La primera vez que A\* extrae el nodo meta de la open list, el camino encontrado es óptimo: cualquier otro nodo en la open list tendría `f ≥ f(goal)`, lo que implica que ningún otro camino puede ser más corto.

**Ejercicio 3.** En MCTS, ¿por qué la fase de selección usa UCB1 en vez de elegir siempre el hijo con más victorias?

*Respuesta:* porque UCB1 equilibra exploración y explotación. Un hijo poco visitado puede tener un valor real alto que no hemos visto. UCB1 incentiva visitarlo al menos una vez, evitando que el algoritmo se quede atascado en una línea aparentemente buena pero realmente mala. Sin UCB1, MCTS converge a jugadas mediocres; con UCB1, encuentra las brillantes.

## 29.14 Ejercicios propuestos

1. Añade obstáculos dinámicos al A\* del 29.11. Cuando el robot descubre un muro, replanifica.
2. Implementa PRM en Rust: muestrea 100 puntos en un grid 10×10, conecta vecinos cercanos, planifica con A\*.
3. Modifica el MCTS del 29.12 para que use una "política" que prefiera el centro del tablero. ¿Mejora?
4. Implementa un minimax con alpha-beta para el tres en raya. ¿Cuántos nodos ahorras vs. minimax sin poda?
5. Construye un behavior tree simple en Rust con tipos: `Selector`, `Sequence`, `Action`. Haz que un NPC decida entre atacar, huir o patrullar.

## 29.15 Pin de batalla

- **La heurística es el alma de A\***. Una buena heurística admisible multiplica el rendimiento por 10 o 100. Una mala convierte A\* en Dijkstra lento.
- **Para mapas grandes, jerarquiza.** HPA\*, navmeshes, flow fields: subdivide antes de planificar.
- **En juegos, los NPCs no necesitan optimalidad.** Un camino "suficientemente bueno" se calcula más rápido. Usa "anytime" A\* si tienes un presupuesto temporal.
- **Los algoritmos de pathfinding son una pequeña parte de la IA de juegos.** Behavior trees, máquinas de estados, planners HTN: el pathfinding es solo una pieza.
- **MCTS no es "el algoritmo de Go"**. Es un meta-algoritmo aplicable a cualquier problema con estructura de árbol y simulación barata. Úsalo en optimización combinatoria, planificación, juegos.

## 29.16 Lo que te llevas

- Robótica y videojuegos comparten el mismo problema: encontrar caminos en grafos. La diferencia es el presupuesto de tiempo y la escala.
- A\* con buena heurística es el caballito de batalla. D\* Lite para mapas dinámicos. RRT/PRM para dimensiones altas.
- Los árboles de juego y alpha-beta son la base de los juegos adversariales clásicos.
- MCTS + redes neuronales = AlphaGo, AlphaZero, MuZero. La "Move 37" de 2016 fue un punto de inflexión.
- Behavior trees y máquinas de estado son la columna vertebral de la IA de NPCs.
- Implementar A\* y MCTS en Rust es didáctico, divertido y deja código útil.

## 29.17 Ojo, cuidado con…

- **A\* sin heurística admisible no garantiza optimalidad.** Y sin heurística informada, es un Dijkstra caro.
- **RRT no es óptimo.** Si necesitas optimalidad, usa RRT\* o BIT\*.
- **MCTS requiere simulación barata.** Si cada simulación cuesta 10 ms, MCTS no escala.
- **Los behavior trees pueden volverse ilegibles.** Muchos programadores de juegos los evitan por esto. Una alternativa: GOAP (Goal-Oriented Action Planning), que es planificación sobre grafos.
- **El pathfinding consume CPU.** En un juego con 1000 NPCs, no todos pueden tener A\* por frame. Asíncrono, horneado, compartido: técnicas obligatorias.

## 29.18 Para profundizar

- **libros**: *Planning Algorithms* (LaValle), *Game AI Pro* (Rabin), *Artificial Intelligence: A Modern Approach* (Russell & Norvig).
- **papers**: A\* (Hart, Nilsson, Raphael 1968), D\* Lite (Koenig 2002), RRT (LaValle 1998), AlphaGo (Silver 2016).
- **crates**: `petgraph`, `rand`, `pathfinding` (otra crate útil), `glam` (matemáticas para juegos).
- **motores**: Godot (gratis, GDScript tiene behavior trees), Unreal (Behavior Tree nativo), Bevy (Rust, en desarrollo).

## 29.19 Si solo lees 30 segundos

Robótica y videojuegos son primos que comparten algoritmos de grafos. A\* es el caballito de batalla, D\* Lite para mapas dinámicos, RRT y PRM para espacios de alta dimensión. Los árboles de juego con alpha-beta dominan los juegos clásicos; MCTS dominó Go. Los behavior trees organizan la IA de personajes. Pac-Man, AlphaGo y tu robot aspirador usan, en el fondo, las mismas ideas: nodos, aristas, búsqueda, optimalidad. Los fantasmas de arcade y los humanoides de Boston Dynamics son primos hermanos. La diferencia es que los fantasmas tienen prisa y los robots tienen ruedas. (O patas. O ambos.)

## 29.20 Una historia pequeña

Aarón soñaba con hacer videojuegos desde los once años. A los diecisiete, leyó sobre A\* y se obsesionó. Implementó A\* en C, después en Python, después en Rust. Leía papers de pathfinding como otros leen novelas. Un día,applyó a una empresa de robótica en su ciudad. "No sé nada de robots", dijo en la entrevista. "Sé grafos", respondió el entrevistador, hojeando su portfolio. Lo contrataron. Hoy Aarón planifica rutas para coches autónomos. No hace videojuegos. Pero a veces, cuando un coche toma una decisión elegante en una intersección, Aarón piensa: "eso habría sido un gran pathfinding en un RPG". El código que le hace señas desde el tablero de la sala de conferencias tiene tres pequeños robots de Lego. Uno de ellos tiene un cartelito: "Blinky". Aarón sonríe cada vez que lo ve.

---

*Fin de la Parte VI-C. Has sobrevivido a proteínas, palabras, robots y fantasmas. La Parte VII te espera.*
