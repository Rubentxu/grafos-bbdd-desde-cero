# Capítulo 30 — Grafos en Verificación y Testing

Un programa compila. Pasa los tests. Se despliega en producción. A la semana explota.
¿Qué falló? No fue el código: fue una transición de estados que nadie imaginó.
Bienvenido a la verificación formal, donde los grafos son la red de seguridad invisible.
En este capítulo vas a ver cómo un sistema entero se modela como un grafo, y cómo un algoritmo recorre ese grafo buscando errores que un humano jamás vería.

## 30.0 La anécdota de la esquina

A principios de los ochenta, Edmund Clarke y Allen Emerson eran dos jóvenes investigadores en Harvard con un problema aparentemente insoluble. Querían verificar que un circuito digital se comportara correctamente. La idea era obvia: enumerar todos los estados posibles y comprobar la propiedad en cada uno. El problema también era obvio: los circuitos tenían miles de transistores, lo que significaba 2^estados posibles, y nadie tenía tanta memoria.

Tuvieron entonces la idea que les valió el Premio Turing en 2007: tratar los estados como un grafo. Cada nodo es un estado del sistema. Cada arista, una transición. Sobre ese grafo, escribir fórmulas de una lógica temporal — "siempre que pidas el recurso, eventualmente lo obtienes" — y dejar que un algoritmo recorra el grafo buscando violaciones. Lo llamaron *model checking*.

Fue, en cierto modo, el primer caso en que la comunidad de verificación se enamoró de los grafos. Hoy, el Airbus A380 usa model checking para verificar su software de vuelo. Y todo empezó con dos personas dándose cuenta de que un sistema es, literalmente, un grafo.

## 30.1 Sistemas reactivos: el grafo más natural del mundo

Un sistema reactivo es aquel que no termina nunca: un ascensor, un semáforo, un protocolo de red, un compilador esperando entrada. Vive en un estado, recibe un evento, pasa a otro estado, repite. Esto es, sin ningún truco, un grafo dirigido.

Llamemos a nuestro sistema de prueba *Semaforín*: un semáforo minimalista. Tiene tres estados: rojo, amarillo, verde. De rojo pasa a verde, de verde a amarillo, de amarillo a rojo. Un grafo con tres nodos y tres aristas. Cero misterio.

```
       (verde)
        ↑   ↓
  (rojo) ← (amarillo)
        ↑________↓
        (regresa a rojo)
```

Hasta aquí, trivial. La gracia aparece cuando añadimos variables: un peatón esperando, un temporizador, un sensor de coches. Cada combinación de valores multiplica el número de estados. Diez variables booleanas son 1024 estados. Veinte variables son más de un millón. Ahí es donde el grafo deja de ser bonito y se vuelve útil como herramienta de razonamiento.

> **Regla de tres + inesperado.** En verificación formal: **(1)** modelas el sistema como grafo, **(2)** escribes la propiedad que quieres garantizar, **(3)** dejas que un algoritmo te diga si se cumple. Y el cuarto elemento, el que nadie espera: a veces el algoritmo te devuelve un contraejemplo, una traza concreta de cómo tu sistema falla. Eso convierte al verificador en el *mejor debugger del mundo*.

## 30.2 La explosión del espacio de estados: el problema fundamental

Hay un chiste que los verificadores cuentan en voz baja: "tenemos un sistema pequeño, de 30 variables booleanas, son sólo mil millones de estados, ¿qué puede salir mal?". El problema se llama *state space explosion* y es la razón por la que la verificación formal fue durante décadas una curiosidad académica.

Si cada variable añade un factor 2 al número de estados, una CPU moderna con 64 registros tiene 2^64 estados posibles. Eso es más que el número de átomos en un gramo de materia. No se puede enumerar.

La solución parcial es el *symbolic model checking* (ver §30.7) y la solución conceptual es no enumerar, sino *razonar sobre conjuntos* de estados a la vez. Los BDDs, los SAT solvers y los SMT solvers atacan exactamente este problema.

```
Variables booleanas:     Estados:
   1                        2
   2                        4
   4                       16
   8                      256
  16                   65 536
  24             16 777 216
  32       4 294 967 296  (~4 mil millones)
  40     ~1 billón
  64     ~1.8 × 10^19  (más que estrellas en la galaxia)
```

La moraleja: modelar es fácil, explorar el modelo es el reto. Como dijo Clarke en una charla, con esa media sonrisa suya: "el primer paso siempre funciona; el segundo es la parte difícil".

## 30.3 Lógicas temporales: CTL y LTL

Para hablar sobre un grafo de estados necesitamos un lenguaje que diga cosas como "siempre", "eventualmente", "existe un camino donde". Ahí entran las lógicas temporales. Las dos reinas son:

- **CTL (Computation Tree Logic)**: cada operador temporal va cuantificado sobre caminos. Sintaxis: `AG p` (en todo camino, siempre p), `EF p` (existe un camino donde eventualmente p), `EX p` (existe un sucesor donde p), `EG p` (existe un camino donde globalmente p).
- **LTL (Linear Time Logic)**: trabaja sobre un único camino. Sintaxis: `G p` (globalmente), `F p` (finalmente), `X p` (siguiente), `p U q` (p hasta q).

Ejemplo aplicado a Semaforín: "en todo camino, si está en verde entonces eventualmente estará en rojo". En CTL: `AG (verde → EF rojo)`. Si esta fórmula se cumple, Semaforín es seguro. Si no, el verificador te dice exactamente cuándo y por qué falla.

Las dos lógicas son equivalentes en expresividad para muchas propiedades, pero CTL es la favorita de los algoritmos que veremos porque su naturaleza arbórea se presta al recorrido recursivo del grafo.

## 30.4 Model checking: el algoritmo que recorre el grafo

La idea es deliciosa. Para verificar una propiedad CTL, calculamos el conjunto de estados que la satisfacen. Los operadores se implementan como operaciones sobre conjuntos:

- `EX p` = preimagen de `p` (estados con al menos un sucesor en `p`).
- `EG p` = greatest fixed point de `λX. p ∩ EX X` (estados desde los que existe un camino que siempre se queda en `p`).
- `AG p` = estados donde *no* existe un camino que lleve a `¬p` (complemento de `EF ¬p`).

Los dos últimos, `EG` y `AG`, se calculan por punto fijo: empiezas con un candidato y refinas hasta que ya no cambia. Es, literalmente, un BFS/DFS con lógica de conjuntos por encima. Lo que ya sabes hacer desde el Capítulo 3.

Por eso este capítulo te resultará familiar: no estás aprendiendo un algoritmo nuevo, sino reconociendo un viejo amigo vestido de smoking.

## 30.5 Bisimulación: cuándo dos sistemas son "el mismo"

Dos grafos pueden parecer distintos y, sin embargo, comportarse igual. Eso se llama *bisimulación*: una relación binaria entre estados que exige que cada transición de un lado se corresponda con una transición equivalente del otro, recursivamente.

```
  Sistema A:              Sistema B:

  (p) --a--> (q)         (p') --a--> (q')
   |                       |
   b                       b
   ↓                       ↓
  (r) <--b-- (s)         (r') <--b-- (s')
```

Si hay bisimulación entre A y B, cualquier propiedad CTL que valga en uno vale en el otro. Esto es útil en la práctica: refactorizar un protocolo no debería cambiar su comportamiento observable. La bisimulación es la prueba formal de que tu refactor no rompió nada.

En Rust, la idea es implementar una partición que se refine hasta estabilizarse — el algoritmo de Paige-Tarjan, clásico del tema.

## 30.6 Testing basado en modelos

Aquí la idea cambia de dirección. En lugar de *verificar* el sistema completo, lo usamos como *generador de tests*. El grafo de estados es un mapa del territorio; recorriéndolo con cobertura sistemática, generamos casos de prueba que un humano jamás habría escrito.

Hay tres coberturas estándar:

1. **Cobertura de estados** — todo nodo es visitado al menos una vez.
2. **Cobertura de transiciones** — toda arista es recorrida al menos una vez.
3. **Cobertura de caminos** — hasta una profundidad N, todo camino se ejercita.

La tercera es exponencial y se usa con moderación, pero es brutal para encontrar bugs en sistemas de comunicación. La herramienta estrella aquí es *Spin*, de Gerard Holzmann, que lleva décadas generando modelos de protocolos en Promela y verificándolos.

## 30.7 BDDs: la magia simbólica

Los *Binary Decision Diagrams* son una representación canónica de funciones booleanas. La idea: en lugar de enumerar estados, representas el conjunto de estados que satisfacen una propiedad como una fórmula, y operas sobre fórmulas.

```
        x1
       /  \
      0    1
      |    |
      x2   x3
     / \   / \
    0   1 0   1
    |   | |   |
    F   T F   T
```

Un BDD compacto puede representar 2^1000 estados con unos pocos megabytes. Es la diferencia entre "el Airbus A380 cabe en 100TB de RAM" y "cabe en 4GB".

En Rust existen crates como `oxidd` y `boolalg` para experimentar. La librería industrial de referencia es CUDD (en C), pero el ecosistema está creciendo.

## 30.8 Implementación Rust: un mini model checker CTL

Vamos a construir un verificador CTL minimalista sobre un grafo de `petgraph`. Nada de BDDs, sólo el algoritmo de punto fijo. Es sorprendentemente corto.

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

/// Estado de un sistema reactivo.
#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    /// Propiedades atómicas verdaderas aquí (p.ej. "verde", "abierto").
    pub labels: HashSet<String>,
}

pub struct ModelChecker {
    pub graph: DiGraph<State, String>,    // aristas etiquetadas con la acción
    pub index: HashMap<String, NodeIndex>,
}

impl ModelChecker {
    pub fn new() -> Self {
        Self { graph: DiGraph::new(), index: HashMap::new() }
    }

    pub fn add_state(&mut self, name: &str, labels: &[&str]) -> NodeIndex {
        let labels = labels.iter().map(|s| s.to_string()).collect();
        let idx = self.graph.add_node(State { name: name.into(), labels });
        self.index.insert(name.into(), idx);
        idx
    }

    pub fn add_transition(&mut self, from: &str, to: &str, action: &str) {
        let f = self.index[from];
        let t = self.index[to];
        self.graph.add_edge(f, t, action.into());
    }

    fn nodes_with(&self, label: &str) -> HashSet<NodeIndex> {
        self.graph.node_indices()
            .filter(|&n| self.graph[n].labels.contains(label))
            .collect()
    }

    /// EX p: existe un sucesor donde vale p.
    pub fn ex(&self, p: &HashSet<NodeIndex>) -> HashSet<NodeIndex> {
        self.graph.node_indices()
            .filter(|&n| self.graph.neighbors(n).any(|s| p.contains(&s)))
            .collect()
    }

    /// EF p: existe un camino donde eventualmente vale p.
    /// = least fixed point: X₀ = p; X_{i+1} = X_i ∪ pre(X_i)
    pub fn ef(&self, p: &HashSet<NodeIndex>) -> HashSet<NodeIndex> {
        let mut x = p.clone();
        loop {
            let pre = self.ex(&x);
            let next: HashSet<_> = x.union(&pre).cloned().collect();
            if next == x { break; }
            x = next;
        }
        x
    }

    /// EG p: existe un camino donde globalmente vale p.
    /// = greatest fixed point: X₀ = p; X_{i+1} = X_i ∩ pre(X_i)
    pub fn eg(&self, p: &HashSet<NodeIndex>) -> HashSet<NodeIndex> {
        let mut x = p.clone();
        loop {
            let pre = self.ex(&x);
            let next: HashSet<_> = x.intersection(&pre).cloned().collect();
            if next == x { break; }
            x = next;
        }
        x
    }

    /// AG p: en todo camino siempre vale p.
    /// = ¬ EF ¬p, o equivalentemente greatest fixed point de λX. p ∩ EX X.
    pub fn ag(&self, p: &HashSet<NodeIndex>) -> HashSet<NodeIndex> {
        self.eg(p)
    }

    /// Verifica una propiedad simple: ¿en todos los estados se cumple p?
    pub fn holds_in_all(&self, label: &str) -> bool {
        let sat = self.nodes_with(label);
        let total: HashSet<_> = self.graph.node_indices().collect();
        self.ag(&sat) == total
    }

    /// Verifica: ¿existe un estado donde se cumple p?
    pub fn holds_in_some(&self, label: &str) -> bool {
        !self.ef(&self.nodes_with(label).complement(&self.all())).is_empty()
            || self.ef(&self.nodes_with(label)) == self.all()
                && self.nodes_with(label) == self.all()
    }

    fn all(&self) -> HashSet<NodeIndex> {
        self.graph.node_indices().collect()
    }
}

/// Extensión útil: complemento de un conjunto dentro de "todos".
trait Complement {
    fn complement(&self, all: &HashSet<NodeIndex>) -> HashSet<NodeIndex>;
}
impl Complement for HashSet<NodeIndex> {
    fn complement(&self, all: &HashSet<NodeIndex>) -> HashSet<NodeIndex> {
        all.difference(self).cloned().collect()
    }
}

fn main() {
    // Semaforín: rojo → verde → amarillo → rojo
    let mut mc = ModelChecker::new();
    mc.add_state("r0", &["rojo"]);
    mc.add_state("v0", &["verde"]);
    mc.add_state("a0", &["amarillo"]);
    mc.add_state("r1", &["rojo"]);

    mc.add_transition("r0", "v0", "tick");
    mc.add_transition("v0", "a0", "tick");
    mc.add_transition("a0", "r1", "tick");
    mc.add_transition("r1", "v0", "tick");

    // ¿El sistema siempre (en todo camino) garantiza que tras verde viene algo?
    // EF(verde → EF(¬verde))  — siempre existe un sucesor no-verde
    let verdes = mc.nodes_with("verde");
    let no_verdes = verdes.complement(&mc.all());
    let tras_verde = mc.ef(&no_verdes);
    let desde_verde: HashSet<_> = verdes.iter()
        .flat_map(|&n| mc.graph.neighbors(n))
        .collect();
    let ex_no_verde_desde_verde: HashSet<_> = desde_verde.intersection(&tras_verde).cloned().collect();

    println!("Tras verde hay un camino a no-verde desde: {:?}", ex_no_verde_desde_verde);
    println!("¿En todos los estados la propiedad se cumple? {}", mc.holds_in_all("rojo"));
}
```

Fíjate: el código entero es esencialmente un BFS/DFS con lógica de conjuntos. Lo que ya sabías hacer.

## 30.9 Diálogo de ascensor

> —Oye, ¿y si en vez de probar el programa con mil inputs, recorro el grafo de estados y demuestro que la propiedad se cumple?
> —Eso es el model checking. Lleva cuarenta años funcionando. ¿Por?
> —Porque suena demasiado bonito. ¿No explota la memoria?
> —Sí, literalmente. Por eso se inventaron los BDDs, los SAT solvers y la verificación composicional. El grafo no se enumera, se representa.
> —O sea, *no modelas el sistema, modelas un modelo del sistema*.
> —Exacto. Y luego, si tienes suerte, el verificador no te devuelve un contraejemplo. Que es lo que siempre devuelve.

## 30.10 Ejercicios resueltos

**Ejercicio 30.1.** Modela un ascensor minimalista de dos pisos con cabina cerrada. Estados: `puerta_cerrada_p0`, `puerta_cerrada_p1`, `puerta_abierta_p0`, `puerta_abierta_p1`, `subiendo`, `bajando`. Escribe transiciones razonables.

*Solución.*
```rust
let mut asc = ModelChecker::new();
asc.add_state("cerrado_p0", &["cerrado", "p0"]);
asc.add_state("cerrado_p1", &["cerrado", "p1"]);
asc.add_state("abierto_p0", &["abierto", "p0"]);
asc.add_state("abierto_p1", &["abierto", "p1"]);
asc.add_state("subiendo",   &["p0", "movimiento"]);
asc.add_state("bajando",    &["p1", "movimiento"]);

asc.add_transition("cerrado_p0", "abierto_p0", "abrir");
asc.add_transition("cerrado_p1", "abierto_p1", "abrir");
asc.add_transition("abierto_p0", "cerrado_p0", "cerrar");
asc.add_transition("abierto_p1", "cerrado_p1", "cerrar");
asc.add_transition("cerrado_p0", "subiendo",   "ir_p1");
asc.add_transition("subiendo",   "cerrado_p1", "llegar");
asc.add_transition("cerrado_p1", "bajando",    "ir_p0");
asc.add_transition("bajando",    "cerrado_p0", "llegar");
```

**Ejercicio 30.2.** Sobre el modelo del ascensor, escribe una propiedad CTL: "no es cierto que existan estados donde la cabina se mueve y la puerta está abierta a la vez". Verifícala con tu checker.

*Solución.* El conjunto inseguro es `movimiento ∩ abierto`. Queremos `AG ¬(movimiento ∧ abierto)`. En código:
```rust
let movimiento = asc.nodes_with("movimiento");
let abierto    = asc.nodes_with("abierto");
let inseguros: HashSet<_> = movimiento.intersection(&abierto).cloned().collect();
let ag_no_inseguros = asc.ag(&inseguros.complement(&asc.all()));
assert!(ag_no_inseguros == asc.all(), "¡Bug! El ascensor se mueve con la puerta abierta");
```

**Ejercicio 30.3.** Implementa un check de cobertura de transiciones que reciba un grafo y devuelva el conjunto de aristas no recorridas por una traza.

*Solución.*
```rust
fn covered_edges(graph: &DiGraph<State, String>, trace: &[NodeIndex]) -> HashSet<(NodeIndex, NodeIndex)> {
    trace.windows(2)
        .filter_map(|w| graph.find_edge(w[0], w[1]).map(|e| (w[0], w[1])))
        .collect()
}

fn total_edges(graph: &DiGraph<State, String>) -> HashSet<(NodeIndex, NodeIndex)> {
    graph.edge_references().map(|e| (e.source(), e.target())).collect()
}

let trace = /* secuencia de nodos visitados por el test */;
let sin_recorrer: HashSet<_> = total_edges(&asc.graph)
    .difference(&covered_edges(&asc.graph, &trace))
    .cloned()
    .collect();
println!("Aristas sin cubrir: {:?}", sin_recorrer);
```

## 30.11 Ejercicios propuestos

1. **El puente levadizo.** Modela un puente con dos barreras, dos semáforos y un sensor. Verifica que las dos barreras nunca estén abiertas a la vez. (Pista: define `peligro = barrera_a ∧ barrera_b`.)

2. **El productor-consumidor.** Modela con tres estados (`vacío`, `lleno`, `produciendo`) y verifica la propiedad `AG (consumiendo → EF vacío)`.

3. **Bisimulación a mano.** Dibuja dos grafos y demuestra que son bisimilares (o que no lo son) construyendo explícitamente la relación.

4. **Cobertura de estados.** Implementa un BFS que devuelva el orden en que visita los estados. Compáralo con la traza de un test real.

5. **El sistema trampa.** Añade un deadlock a Semaforín y comprueba cómo `AG EF avanzar` deja de cumplirse. ¿Qué contraejemplo te devuelve el checker?

## 30.12 Pin de batalla

- **Empieza por lo pequeño.** Un verificador para 10 estados es trivial. Cuando funcione, sube. No al revés.
- **Las propiedades negativas son amigas.** `AG ¬peligro` es más fácil de verificar que `algo_bien_pasa_eventualmente`.
- **Cada contraejemplo es documentación.** Si el verificador te devuelve una traza, es un test que faltaba en tu suite. No lo borres, conviértelo en test de regresión.
- **No modeles más de la cuenta.** Más variables = más estados = más dolor. Modela lo justo para la propiedad que te importa.
- **Spin y TLA+ son tus amigos.** Antes de reinventar la rueda, mira si la rueda ya está bien inventada y bien rodada.

## 30.13 Lo que te llevas

- Un sistema reactivo es un grafo. Punto.
- La verificación formal recorre ese grafo buscando violaciones de propiedades temporales.
- Las lógicas CTL y LTL te dan el lenguaje para escribir esas propiedades.
- Los algoritmos son versiones de BFS/DFS con lógica de conjuntos por encima.
- El state space explosion es el problema real; los BDDs y el análisis simbólico lo atacan.

## 30.14 Ojo, cuidado con…

…pensar que "verificar" significa "asegurar al 100%". Un verificador sólo garantiza que el *modelo* cumple la propiedad. Si tu modelo no captura el fallo real, el verificador te dará un falso positivo de seguridad. Es el clásico "garbage in, gospel out". Modelar es el 80% del trabajo; verificar es el 20% restante.

## 30.15 Para profundizar

- *Model Checking* de Clarke, Grumberg, Kroening, Peled y Veith (MIT Press, segunda edición). La biblia.
- *Principles of Model Checking* de Baier y Katoen. Más pedagógico.
- *Spin* — herramienta de verificación de Gerard Holzmann.
- *TLA+* — el lenguaje de Leslie Lamport, usado en Amazon para diseñar sistemas distribuidos.

## 30.16 Si solo lees 30 segundos

Modela tu sistema como un grafo. Escribe la propiedad que quieres. Deja que un algoritmo recorra el grafo. Si encuentra un contraejemplo, tienes un test. Si no, tienes una demostración. Eso es verificación formal, y se hace con grafos.

## 30.17 Una historia pequeña

Marina entró al equipo de firmware de una empresa de ascensores con una misión: encontrar por qué dos modelos de la misma familia daban resultados distintos en un test de seguridad. Miró el código durante tres semanas sin encontrar nada. Una tarde, aburrida, dibujó la máquina de estados del protocolo de cabina en un papel. Había un estado fantasma, al que se llegaba sólo si dos eventos ocurrían en una ventana de 50 milisegundos. En el modelo A ese estado era inalcanzable por un detalle del reloj. En el modelo B, era alcanzable. Una arista que no debería estar, cambiaba el comportamiento. La dibujó en el informe con un círculo rojo. Su jefa la miró y dijo: "esto es exactamente lo que hacían los ingenieros de Airbus". Marina no volvió a descartar un diagrama de estados en su vida.

---

