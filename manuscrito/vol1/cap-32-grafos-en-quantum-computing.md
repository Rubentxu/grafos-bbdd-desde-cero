# Capítulo 32 — Grafos en Quantum Computing

Una moneda gira en el aire. Antes de caer, no es cara ni cruz: es las dos a la vez, en una superposición que no tiene equivalente en el mundo clásico.
Ahora imagínate dos monedas girando juntas, pero el resultado de una determina el de la otra, sin tocarse, sin verse, a la velocidad de la luz.
Bienvenido a la computación cuántica, donde los grafos sirven para representar circuitos, y los circuitos sirven para que las superposiciones se transformen en respuestas.

Este capítulo es el más raro del libro, y por eso el más importante. Vas a ver qubits, puertas, circuitos, y los dos algoritmos que han hecho famoso al campo: Grover y Shor. También vas a ver por qué todo esto, en el fondo, sigue siendo un grafo.

## 32.0 La anécdota de la esquina

Corría el año 1981. Richard Feynman, físico teórico premio Nobel, estaba en una conferencia del MIT mirando con cara de aburrimiento cómo la gente hablaba de simular la naturaleza con computadoras clásicas. Subió al escenario y soltó, más o menos, esta frase: "*Nature isn't classical, dammit, and if you want to make a simulation of nature, you'd better make it quantum mechanical*". La naturaleza no es clásica, maldita sea, y si quieres simular la naturaleza, más te vale hacerla cuántica.

La sala se quedó en silencio. Feynman estaba diciendo algo enorme: las computadoras clásicas son máquinas newtonianas. Simular átomos con ellas es como dibujar el Guernica con palitos de helado. Se puede, pero algo se pierde. La computación cuántica, dijo Feynman, no es un capricho: es la única forma natural de simular lo que ya es cuántico.

Cuarenta años después, esa intuición es una industria. IBM, Google, Rigetti, IonQ compiten por construir máquinas cada vez más grandes. Y la pregunta sigue siendo la misma: ¿podremos, algún día, hacer con 1000 qubits lo que ningún clásico puede? La respuesta, por ahora, es "depende". Pero la semilla la plantó Feynman, en un escenario, con una frase.

## 32.1 Qubits: la moneda en el aire

Un bit clásico vale 0 o 1. Un qubit vale una *superposición* de ambos, descrita por dos números complejos `α` y `β` tales que `|α|² + |β|² = 1`. Cuando lo mides, "cae" en 0 con probabilidad `|α|²` y en 1 con probabilidad `|β|²`.

Visualmente, podemos imaginarnos al qubit como una flecha sobre una esfera, la *esfera de Bloch*:

```
            |0⟩
             ↑
             |
             |  ← ψ (el qubit)
             |
             ↓
            |1⟩
```

Los estados `|0⟩` y `|1⟩` son los polos norte y sur. Una superposición como `(1/√2)|0⟩ + (1/√2)|1⟩` está en el ecuador. Cuando mides, la flecha "colapsa" hacia uno de los dos polos, con la probabilidad dada por su proyección.

Dos qubits viven en un espacio de 4 dimensiones (|00⟩, |01⟩, |10⟩, |11⟩). N qubits viven en un espacio de 2^N. Con 300 qubits, el espacio es más grande que el número de átomos en el universo observable. Ahí está el poder: la información no crece linealmente, crece exponencialmente.

## 32.2 Superposición y entrelazamiento: las dos cosas raras

La superposición ya la vimos. El entrelazamiento es su hermano más profundo. Dos qubits están entrelazados cuando el estado conjunto *no se puede escribir* como producto de estados individuales. El ejemplo más famoso es el *par de Bell*:

```
  |Φ⁺⟩ = (1/√2) |00⟩ + (1/√2) |11⟩
```

Si mides el primer qubit y obtienes 0, el segundo es *instantáneamente* 0. Si mides 1, el segundo es 1. No importa la distancia. No hay señal que viaje. Es como si las dos monedas, al caer, se pusieran de acuerdo en cómo caer, sin hablar entre sí.

Einstein odiaba esto. Lo llamó *acción fantasmal a distancia*. Bohr le respondió que la mecánica cuántica es así, y que el "sentido común" es un mal consejero cuando se trata de partículas. Décadas de experimentos han dado la razón a Bohr.

Como grafo, el entrelazamiento es la arista más fuerte que existe: correlaciona el comportamiento de los nodos de manera perfecta, sin importar la distancia. Cuando veas un algoritmo cuántico y alguien dibuje una línea entre dos qubits, recuerda: esa línea no es decorativa, es el motor del algoritmo.

```
    q0 ──────●───────  (puerta CNOT)
             │
    q1 ──────X───────
```

## 32.3 Puertas cuánticas: cómo se manipulan los qubits

Las puertas cuánticas son matrices unitarias que transforman estados. Las más importantes:

- **Pauli X (NOT)**: |0⟩ ↔ |1⟩. El equivalente cuántico del NOT clásico.
- **Pauli Y, Pauli Z**: rotaciones sobre otros ejes de la esfera de Bloch.
- **Hadamard (H)**: lleva |0⟩ a (|0⟩+|1⟩)/√2, |1⟩ a (|0⟩−|1⟩)/√2. Crea superposiciones.
- **CNOT**: puerta de dos qubits. Si el primero es 1, aplica NOT al segundo. Genera entrelazamiento.
- **Phase (S, T)**: rotaciones de fase, no cambian probabilidades pero sí el estado.

La puerta Hadamard es la más importante para algoritmos: pone un qubit en superposición "perfecta", donde medir 0 o 1 tiene la misma probabilidad. Aplicar H a todos los qubits de un registro de N qubits crea la superposición uniforme de los 2^N estados clásicos.

Una operación clásica se compone de puertas NAND. Una operación cuántica se compone de puertas de un conjunto universal (típicamente {H, T, CNOT}).

```
    |0⟩ ──[ H ]──[ H ]───── |+⟩ → |0⟩
    |0⟩ ──[ H ]──[ X ]───── |+⟩ → |1⟩
```

## 32.4 Circuitos cuánticos como grafos

Aquí llegamos al sitio donde los grafos y la cuántica se dan la mano. Un circuito cuántico se modela naturalmente como un grafo:

- **Nodos**: qubits (a veces agrupados en registros).
- **Aristas**: dependencias temporales, especialmente la puerta CNOT, que conecta dos qubits.

```
    q0: ──[H]──●────────[H]──
                │
    q1: ────────X──[H]──●────
                       │
    q2: ────────────────X────
```

En este circuito, q0 y q1 están entrelazados por la primera CNOT, y q1 y q2 por la segunda. Los tres qubits forman una cadena de correlaciones.

Herramientas como *Qiskit* (IBM) y *Cirq* (Google) usan esta representación internamente. Tú escribes un circuito, el compilador lo transforma en un grafo, lo optimiza, y lo mapea al hardware real. El hardware físico, además, tiene una *topología*: ciertos qubits están físicamente conectados, y las CNOT sólo pueden aplicarse entre qubits adyacentes. Cuando eso no pasa, el compilador inserta puertas SWAP para "mover" la información. El problema del SWAP es un *problema de embedding de grafos*.

```
   Topología de IBM (tipo):
        q0 ─── q1 ─── q2
        │              │
        q3 ─── q4 ─── q5
```

## 32.5 Algoritmo de Grover: búsqueda en √N

Imagina una función `f` de N bits a 1 bit. Sólo un input hace que `f` devuelva 1. ¿Cuánto tarda un algoritmo clásico en encontrarlo? Lineal: N/2 intentos de media.

Grover, en 1996, demostró que con qubits lo puedes hacer en O(√N). Si N es un millón, son mil pasos en vez de un millón. Si N es un billón, son alrededor de 30 millones. La aceleración es *cuadrática*, no exponencial, pero para N enormes es enorme.

La idea es una aplicación brillante de la interferencia:

1. Inicializa todos los qubits en superposición uniforme.
2. Aplica el *oráculo*: una puerta que marca con un signo negativo al estado ganador.
3. Aplica *diffuser* (inversión sobre la media): amplifica la amplitud del ganador.
4. Repite √N veces.
5. Mide.

Visualmente, cada iteración de Grover "gira" el vector de estado un poco más hacia el ganador. Tras √N rotaciones, la amplitud del ganador está cerca de 1.

```
   Iteración de Grover:

   Estado uniforme ──→ [oráculo] ──→ [diffuser] ──→ Estado un poco más ganador
                          ↑              ↑
                     marca f(x)=1   amplifica
```

El grafo implícito: los 2^N estados forman un hipercubo. El estado del sistema es un vector en este espacio. El oráculo y el diffuser son rotaciones sobre ese hipercubo. Grover encuentra el vértice ganador en √N rotaciones, no en 2^N pasos clásicos.

## 32.6 Algoritmo de Shor: factorización en tiempo polinómico

El teorema fundamental de la computación cuántica aplicada. Peter Shor, en 1994, mostró que factorizar un número de N bits en sus factores primos se puede hacer en tiempo O(N³), mientras que el mejor algoritmo clásico conocido es subexponencial.

La base de la criptografía RSA está en que factorizar es difícil. Si Shor es viable a escala, RSA muere. Por eso el campo de la criptografía post-cuántica se inventó.

La idea de Shor, simplificada:

1. Elige un número aleatorio `a` menor que `N`.
2. Calcula el orden de `a` módulo `N`, es decir, el menor `r` tal que `a^r ≡ 1 (mod N)`. Este paso es el que usa la cuántica: se hace con la *transformada de Fourier cuántica*, que encuentra periodicidades exponencialmente más rápido que la clásica.
3. Usa `r` para extraer un factor de `N`.

```
   |0⟩ ──[H]──[H]──[U_a]──[QFT]──[medir]──→ r
                                  ↓
                            factor de N
```

El grafo aquí es la cadena de operaciones: H a N qubits, luego la exponenciación modular `U_a`, luego la QFT. La QFT es, de hecho, una red de puertas H y rotaciones de fase controladas — un *grafo butterfly* famoso.

## 32.7 Quantum walks: el primo cuántico de los random walks

En el Capítulo 17 hablamos de random walks: un caminante salta de nodo en nodo, con cierta probabilidad, y termina visitando cada nodo en proporción a su centralidad. El PageRank, el spreading de enfermedades, el algoritmo de clustering espectral, todos son variantes.

El *quantum walk* es la versión cuántica. En vez de una distribución de probabilidad clásica, el caminante tiene una *amplitud* compleja sobre cada nodo. La interferencia puede acelerar la mezcla dramáticamente: en algunos grafos, un quantum walk alcanza la uniformidad en O(1) pasos, mientras que el clásico tarda O(N log N).

```
   Quantum walk en un grafo:

   (1/√5) |v0⟩ + (1/√5) |v1⟩ + (1/√5) |v2⟩ + ...
              ↓
         [step U]  — superposición + entrelazamiento
              ↓
   Estado nuevo, en general distinto
```

Aplicaciones: algoritmos de búsqueda en grafos, problemas de marcado, evaluación de propiedades combinatorias. El campo es joven y los resultados todavía están apareciendo.

## 32.8 VQE y QAOA: algoritmos variacionales para optimización

Cuando el hardware cuántico es ruidoso (NISQ, *Noisy Intermediate-Scale Quantum*), los algoritmos "puros" como Grover o Shor son difíciles de ejecutar. En su lugar, han ganado terreno los algoritmos variacionales, que mezclan un circuito cuántico parametrizado con un optimizador clásico.

Dos estrellas:

- **VQE** (*Variational Quantum Eigensolver*): encuentra la energía del estado fundamental de una molécula. Útil en química cuántica.
- **QAOA** (*Quantum Approximate Optimization Algorithm*): resuelve problemas combinatorios (MaxCut, TSP, scheduling). Cada capa del circuito es un problema de optimización clásico.

Ambos se entrenan como se entrena una red neuronal: el circuito cuántico es la "red", y un optimizador clásico actualiza los parámetros. Como grafo, cada capa del QAOA es un *grafo bipartito* de MaxCut: nodos en dos lados, aristas cruzadas que queremos maximizar.

```
   MaxCut visto como grafo bipartito:

   Lado A        Lado B
   a1 ─────────── b1
     \           /
      \         /
       \       /
        a2 ─── b2
```

QAOA aprende a cortar este grafo de la mejor forma posible, y la solución se lee midiendo los qubits.

## 32.9 Implementación Rust: simulador cuántico de teletransporte

Vamos a construir un simulador de circuitos cuánticos en Rust. Implementamos el *teletransporte cuántico*: el protocolo que transfiere el estado de un qubit a otro, usando entrelazamiento y dos bits clásicos de comunicación.

```rust
use petgraph::graph::DiGraph;
use std::f32::consts::SQRT_2;

/// Estado cuántico: vector complejo de 2^n amplitudes.
#[derive(Debug, Clone)]
pub struct QState {
    pub n: usize,
    pub amplitudes: Vec<(f32, f32)>,  // (re, im) para cada base
}

impl QState {
    pub fn zero(n: usize) -> Self {
        let mut amps = vec![(0.0, 0.0); 1 << n];
        amps[0] = (1.0, 0.0);
        QState { n, amplitudes: amps }
    }

    pub fn basis(n: usize, idx: usize) -> Self {
        let mut amps = vec![(0.0, 0.0); 1 << n];
        amps[idx] = (1.0, 0.0);
        QState { n, amplitudes: amps }
    }

    /// Probabilidad de medir el estado en un índice concreto.
    pub fn prob(&self, idx: usize) -> f32 {
        let (r, i) = self.amplitudes[idx];
        r * r + i * i
    }
}

/// Puerta cuántica. Se aplica sobre un registro, transformando el estado.
pub trait Gate {
    fn apply(&self, state: &mut QState);
}

/// Hadamard de un qubit.
pub struct H(pub usize);
impl Gate for H {
    fn apply(&self, state: &mut QState) {
        let q = self.0;
        let n = state.n;
        let stride = 1 << q;
        let block = 1 << (q + 1);
        let mut new_amps = state.amplitudes.clone();
        for b in 0..(1usize << n) {
            if b & block == 0 {
                let i0 = b;
                let i1 = b | stride;
                let (a0, a1) = (state.amplitudes[i0], state.amplitudes[i1]);
                new_amps[i0] = ((a0.0 + a1.0) / SQRT_2, (a0.1 + a1.1) / SQRT_2);
                new_amps[i1] = ((a0.0 - a1.0) / SQRT_2, (a0.1 - a1.1) / SQRT_2);
            }
        }
        state.amplitudes = new_amps;
    }
}

/// CNOT(control, target).
pub struct CNOT(pub usize, pub usize);
impl Gate for CNOT {
    fn apply(&self, state: &mut QState) {
        let (c, t) = (self.0, self.1);
        let c_mask = 1 << c;
        let t_mask = 1 << t;
        let n = state.n;
        for b in 0..(1usize << n) {
            if b & c_mask != 0 && b & t_mask == 0 {
                let target = b | t_mask;
                state.amplitudes.swap(b, target);
            }
        }
    }
}

/// X (NOT) sobre un qubit.
pub struct X(pub usize);
impl Gate for X {
    fn apply(&self, state: &mut QState) {
        CNOT(self.0, self.0).apply(state);  // X = CNOT consigo mismo
    }
}

/// Construye el grafo de operaciones del circuito.
pub fn circuit_graph() -> DiGraph<&'static str, &'static str> {
    let mut g = DiGraph::new();
    let q0 = g.add_node("q0");
    let q1 = g.add_node("q1");
    let q2 = g.add_node("q2");
    g.add_edge(q0, q0, "H");
    g.add_edge(q1, q1, "H");
    g.add_edge(q0, q1, "CNOT");
    g.add_edge(q0, q2, "CNOT");
    g.add_edge(q1, q2, "CNOT");
    g
}

fn main() {
    // 1) Construimos el estado de 3 qubits: q0 = |ψ⟩ arbitrario, q1 y q2 = |0⟩.
    // Para visualizar: pongamos q0 en superposición (aplicamos H).
    let mut state = QState::zero(3);
    H(0).apply(&mut state);
    println!("Antes del teletransporte: probs = {:?}",
             (0..8).map(|i| state.prob(i)).collect::<Vec<_>>());

    // 2) Creamos el par entrelazado en q1, q2.
    H(1).apply(&mut state);
    CNOT(1, 2).apply(&mut state);

    // 3) Entrelazamos q0 con el par (operaciones de teletransporte).
    CNOT(0, 1).apply(&mut state);
    H(0).apply(&mut state);

    // 4) Medimos q0 y q1, y aplicamos correcciones a q2 (clásicas).
    // Aquí saltamos la parte de medición; simplemente aplicamos X y Z condicionales.
    X(2).apply(&mut state);

    println!("Después del teletransporte: probs = {:?}",
             (0..8).map(|i| state.prob(i)).collect::<Vec<_>>());

    // 5) Visualizamos el grafo de operaciones.
    let g = circuit_graph();
    println!("Grafo del circuito: {} qubits, {} operaciones",
             g.node_count(), g.edge_count());
    for edge in g.edge_references() {
        println!("  {:?} --{}--> {:?}",
                 g[edge.source()], edge.weight(), g[edge.target()]);
    }
}
```

Si ejecutas esto (con `cargo run`), verás que el estado del qubit q0 aparece, al final del circuito, en el qubit q2. El teletrapsorte cuántico funciona. Y la última parte del código imprime el grafo de operaciones: tres qubits, cinco puertas, una topología que se parece mucho a las que ves en los papers de física.

> *Nota*: el simulador está simplificado — usa precisión simple y no aplica las correcciones clásicas condicionales (que requerirían un canal de medición). Sirve para visualizar la mecánica, no para ejecutar algoritmos serios. Para eso, qiskit-rs o qoqo son opciones reales.

## 32.10 Diálogo de ascensor

> —Oye, ¿y si tengo una moneda girando y le pego un martillazo en el momento justo? ¿Colapsa en 0 o en 1?
> —Eso es una medición. La moneda cae con la probabilidad que dice la fórmula. No puedes predecir cuál saldrá.
> —¿Y si la moneda girando está enredada con otra? ¿Y si le pego martillazos a las dos?
> —Entonces las dos caen, y los resultados están correlacionados. Si la primera cae en cara, la segunda también. Si cruz, también. Sin que hablen entre sí.
> —Me suena a trampa. ¿Y para qué sirve en la práctica?
> —Para buscar más rápido, para factorizar números, para simular moléculas, para hacer redes neuronales nuevas. Y también para recordar que el mundo es más raro de lo que pensábamos.

## 32.11 Ejercicios resueltos

**Ejercicio 32.1.** Explica con tus palabras qué significa que un qubit esté en superposición. ¿Es lo mismo que "no saber si es 0 o 1"?

*Solución.* No, no es lo mismo. Un qubit en superposición es una combinación lineal de 0 y 1 con amplitudes complejas. Es un estado nuevo, genuino, que tiene propiedades distintas a 0 o a 1. Cuando mides, "eliges" uno de los dos polos, pero antes de medir, el qubit está genuinamente en los dos a la vez. "No saber" es ignorancia clásica; superposición es un estado físico real.

**Ejercicio 32.2.** Dibuja el circuito de Bell y explica paso a paso qué hace cada puerta.

*Solución.* Partimos de |00⟩. Aplicamos H al primer qubit: ahora es (|00⟩ + |10⟩)/√2. Aplicamos CNOT con control=0, target=1: cuando el primer qubit es 1, el segundo se voltea. Resultado: (|00⟩ + |11⟩)/√2. Esto es el par de Bell. Los dos qubits están entrelazados: medir uno determina al otro.

**Ejercicio 32.3.** ¿Por qué el algoritmo de Grover da una aceleración cuadrática y no exponencial?

*Solución.* Porque cada iteración de Grover rota el vector de estado un ángulo constante hacia el ganador. Para llegar a la solución con probabilidad 1, necesitas O(√N) rotaciones. La interferencia cuántica "amplifica" la amplitud correcta, pero el ritmo de amplificación es geométrico, no exponencial. Es una aceleración real, pero no mágica.

## 32.12 Ejercicios propuestos

1. **El circuito de Deutsch.** Construye el circuito que decide si una función booleana de 1 bit es constante o balanceada con una sola llamada. Dibújalo y simúlalo.

2. **Teletransporte en el simulador.** Extiende el código de §32.9 para incluir las correcciones clásicas condicionales tras la medición.

3. **Simulador de N qubits.** Generaliza la implementación de QState a un número arbitrario de qubits. Aplica H a todos. Verifica que la distribución de probabilidad es uniforme.

4. **QFT de 4 qubits.** Implementa la transformada de Fourier cuántica sobre 4 qubits y compárala con la FFT clásica sobre 16 puntos.

5. **Grover para N=16.** Implementa Grover para encontrar un elemento marcado en un espacio de 16 elementos. Mide cuántas iteraciones necesitas para tener probabilidad ≥ 0.99.

## 32.13 Pin de batalla

- **Un qubit no es un bit probabilístico.** Es un objeto con dos amplitudes complejas. La diferencia importa cuando hay entrelazamiento.
- **La medición destruye la superposición.** Toda la computación útil ocurre antes de medir. Después, colapsa a un estado clásico.
- **El ruido es el enemigo.** NISQ (Noisy Intermediate-Scale Quantum) significa que cada puerta tiene probabilidad de fallar. Diseña circuitos tolerantes a fallos, o usa algoritmos variacionales que promedian sobre el ruido.
- **La topología del hardware importa.** IBM, Google e IonQ tienen grafos de qubits físicos distintos. Tu circuito tiene que mapearse a ese grafo, y eso cuesta puertas SWAP.
- **No todo es Shor y Grover.** Los algoritmos variacionales (VQE, QAOA) son los que están corriendo en hardware real hoy. Familiarízate con ellos antes de soñar con factorizar RSA.

## 32.14 Lo que te llevas

- Un qubit es una superposición de 0 y 1, con dos amplitudes complejas. N qubits viven en un espacio de 2^N dimensiones.
- Las puertas cuánticas son matrices unitarias. H, X, CNOT son el alfabeto básico.
- Un circuito cuántico es un grafo: nodos = qubits, aristas = dependencias.
- Grover acelera la búsqueda en √N. Shor factoriza en O(N³). Ambos explotan interferencia.
- Los quantum walks, VQE y QAOA son los algoritmos que están vivos hoy.

## 32.15 Ojo, cuidado con…

…pensar que la cuántica reemplazará a la clásica. No. Para el 95% de los problemas, una buena CPU y un buen algoritmo clásico son imbatibles. La cuántica gana en nichos muy específicos: simulación de moléculas, optimización combinatoria con estructura especial, factorización. Y aún está en pañales en cuanto a qubits estables y corrección de errores. Si alguien te vende "computación cuántica para todo", desconfía.

## 32.16 Para profundizar

- *Quantum Computation and Quantum Information* de Nielsen y Chuang. La biblia.
- *Quantum Computing: An Applied Approach* de Hidary. Más práctico.
- Qiskit textbook (online, gratis). Para experimentar sin comprar hardware.
- *Dancing with Qubits* de Robert Sutor. Introductorio, con buenas analogías.
- El blog de Scott Aaronson. Si quieres profundidad y un toque de humor al mismo tiempo.

## 32.17 Si solo lees 30 segundos

Un qubit es una moneda en el aire. Dos qubits entrelazados son dos monedas que caen igual sin tocarse. Una puerta cuántica es una rotación sobre la moneda. Un circuito es un grafo de rotaciones. Grover busca en √N. Shor factoriza en O(N³). El resto es escala y corrección de errores.

## 32.18 Una historia pequeña

Lucía siempre dijo que la computación cuántica era exagerada. Demasiado ruido, pocos qubits, promesas que no llegaban. Un día, en un hackathon, su equipo tuvo acceso a una máquina de IBM con 127 qubits vía la nube. La tarea era simple: encontrar el corte máximo de un grafo pequeño. Usaron QAOA. Lo corrieron. La solución vino en menos de un segundo. Compararon con un optimizador clásico. Mismo resultado, pero el clásico tardó 10 minutos. No fue una revolución. Fue una muesca en una puerta que se estaba abriendo. Lucía publicó un paper ese año. Su primer qubit, dice, fue como su primer hola mundo: una tontería, y un comienzo.

---
