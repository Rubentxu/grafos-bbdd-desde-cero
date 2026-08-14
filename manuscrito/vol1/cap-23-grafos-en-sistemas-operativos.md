# Capítulo 23 — Grafos en Sistemas Operativos

Si alguna vez tu programa se ha quedado colgado y has rezado para que no sea un deadlock, este capítulo es para ti. Hay tres clases de programadores: los que creen que los deadlocks son teoría, los que ya han luchado contra uno en producción, y los que ya ni se acuerdan porque aprendieron a prevenirlos con grafos. Bienvenido al club de los que duermen tranquilos.

En este capítulo dibujamos un tipo de grafo muy especial: el **RAG** (Resource Allocation Graph). Verás que un deadlock es, literalmente, un **ciclo en un grafo**. Y **prevenirlo** es romper una de las cuatro condiciones que forman ese ciclo. Vamos a ello.

## 23.0 La anécdota del banquero holandés

Estamos en 1965. Edsger Dijkstra, un holandés con un talento sobrenatural para los algoritmos (¿recuerdas Dijkstra del shortest path?), publica un paper titulado *«Cooperating Sequential Processes»*. En él, entre otras joyas, presenta el **problema del banquero**.

Imagina un banquero con un capital limitado que recibe peticiones de préstamos de varios clientes (procesos). Cada cliente declara de antemano su **máximo** de dinero que podría llegar a necesitar. El banquero solo concede un préstamo si, después de hacerlo, existe una **secuencia segura** de concesiones que permita a TODOS los clientes terminar sin quedarse sin dinero. Si no existe esa secuencia, el banquero dice "no" y el cliente espera.

La pregunta clave es: ¿existe un estado seguro? Para responderla, Dijkstra diseñó un algoritmo que esencialmente **explora un grafo implícito** de estados (nodo = estado de concesión de recursos, arista = concesión válida). El algoritmo es una especie de BFS con poda: si desde el estado actual no puedes llegar a un estado en el que todos terminan, rechazas. Y el test "¿hay un ciclo en el RAG?" es la versión *on-the-fly* de la misma idea. La elegancia del paper es que Dijkstra no solo resolvió un problema técnico: formuló el deadlock en términos de teoría de grafos, y eso nos dio una herramienta reusable.

## 23.1 RAG: el grafo que ve los deadlocks

El **RAG** (Resource Allocation Graph) es un grafo bipartito con dos tipos de nodos:

- **Procesos** (P₁, P₂, …): circulitos.
- **Recursos** (R₁, R₂, …): cuadraditos. Cada uno con un contador (¿cuántas instancias hay?).

Y dos tipos de aristas:

- **Asignación** (recurso → proceso): "el proceso P₁ tiene la instancia de R₂". Flecha del recurso al proceso.
- **Petición** (proceso → recurso): "el proceso P₂ está esperando una instancia de R₃". Flecha del proceso al recurso.

```
Ejemplo:

   R1 (2 instancias)        R2 (1 instancia)
    │  ╲                      │
    │   ╲                     │
    ▼    ▼                    ▼
   P1    P2 ◄──────────────► P3

  R1 → P1  (R1 asignada a P1)
  R1 → P2  (R1 asignada a P2)
  P2 → R2  (P2 pide R2)
  P2 → P3  (P2 pide P3) ??? raro
```

(Perdona, la última arista no es estándar; en RAG solo hay aristas entre procesos y recursos.) Versión limpia:

```
   R1 (2)        R2 (1)
   │  │            ▲
   │  │            │
   ▼  ▼            │
   P1  P2──────────┘
       │
       │ pide
       ▼
      R3 (2) ────► P3
```

**Regla de oro**: si en el RAG hay un **ciclo**, hay un deadlock potencial. Si el ciclo involucra solo recursos con una instancia, hay un deadlock seguro. Si hay recursos multi-instancia, hay que hacer un análisis más fino (el del banquero).

## 23.2 Las 4 condiciones de Coffman

Para que haya un deadlock, deben cumplirse **simultáneamente** las 4 condiciones de Coffman (1971):

1. **Exclusión mutua**: cada recurso está en uso por, a lo sumo, un proceso.
2. **Hold and wait**: un proceso que tiene un recurso puede pedir más.
3. **No preemption**: un recurso no se le puede quitar a un proceso a la fuerza; solo lo libera él voluntariamente.
4. **Circular wait**: existe una cadena circular de procesos cada uno esperando un recurso que tiene el siguiente.

**Prevenir un deadlock** = romper una de las cuatro. Por ejemplo:

- Romper **#1**: usar recursos compartidos (lectores/escritores) cuando sea posible.
- Romper **#2**: pedir TODOS los recursos al inicio, en una sola petición atómica.
- Romper **#3**: permitir preemption. Caro, a veces imposible (una impresora a mitad de impresión).
- Romper **#4**: imponer un **orden total** sobre los recursos. Si todos los procesos piden los recursos en el mismo orden (p. ej., siempre R₁ antes que R₂), no puede haber espera circular. Es la más usada en la práctica.

## 23.3 Algoritmo del banquero en forma de grafo

Implementemos el algoritmo del banquero. La entrada:

- `n`: número de procesos.
- `m`: número de tipos de recursos.
- `max[i][j]`: máximo que el proceso `i` puede pedir del recurso `j`.
- `alloc[i][j]`: lo que `i` ya tiene.
- `avail[j]`: disponible del recurso `j`.

El test de seguridad:

```
1. Work = avail. Finish[i] = false para todo i.
2. Buscar i tal que:
      Finish[i] == false
      Y need[i] = max[i] - alloc[i] <= work
   Si no existe, ir a 4.
3. Work = Work + alloc[i]  (simula que i termina y libera sus recursos)
   Finish[i] = true. Volver a 2.
4. Si Finish[i] == true para todo i, estado SEGURO.
   Si no, INSEGURO.
```

Si el estado es seguro, se concede la petición. Si no, se rechaza y el proceso espera.

Esto es esencialmente una búsqueda en un grafo implícito: cada nodo es un vector `Work + Finish`, y las aristas son "elige un proceso `i` cuya necesidad cabe en `Work` y avánzalo". Si la búsqueda exhaustiva encuentra una permutación donde todos terminan, hay un **camino seguro** en el grafo.

## 23.4 Process scheduling: dependencias y topological sort

Otra aplicación clásica: el **grafos de dependencias** entre procesos (o tareas). Si la tarea B necesita el resultado de A, hay una arista A → B. Para ejecutar todas las tareas respetando dependencias, necesitas un **topological sort**. Si el grafo tiene un ciclo, hay una dependencia circular y alguien va a esperar para siempre.

```
   compilar    test_unit
       │           │
       ▼           ▼
    linkear    test_integracion
              ╲     ╱
               ▼   ▼
              deploy
```

Esto es un DAG (Directed Acyclic Graph). El topological sort te da un orden lineal válido: `compilar → test_unit → linkear → test_integracion → deploy`.

## 23.5 Sistemas de archivos: inodos y B-trees

Un **inode** (Unix) o **MFT entry** (NTFS) es una estructura de datos que apunta a los bloques de un archivo. Un directorio es un nodo que apunta a inodes de archivos y otros directorios. La estructura es un **grafo dirigido** (con ciclos: los hard links) o un árbol (sin ciclos: las symlinks bien hechas). El comando `find` recorre ese grafo; `du` lo recorre y suma tamaños; `ln` añade una arista.

```
       / (inode 1)
        │
        ├─► home (inode 100)
        │      │
        │      └─► ana (inode 200)
        │              │
        │              ├─► carta.txt (inode 500)
        │              └─► proyectos (inode 300)
        │                       │
        │                       └─► borrador (inode 400)
        │
        └─► etc (inode 50)
               │
               └─► passwd (inode 51)
```

Y dentro de cada directorio, los nombres se almacenan en **B-trees** (o variantes como B+trees), que son árboles balanceados optimizados para acceso a disco. Otro grafo, otra estructura, otra jornada.

## 23.6 Memoria: páginas y allocation graphs

La memoria virtual divide la RAM en **páginas** (típicamente 4 KB) y mantiene un mapeo de páginas virtuales a páginas físicas. Ese mapeo es, otra vez, un grafo (de hecho, un **grafo bipartito** entre páginas virtuales y marcos físicos).

Y cuando un programa pide memoria con `malloc`, el allocator mantiene un **allocation graph** interno: cada bloque libre apunta al siguiente bloque libre (lista libre). Cuando liberas un bloque, se reinserta en la lista. Es un grafo enlazado clásico.

## 23.7 Simulador de RAG en Rust puro

Vamos a hacer un mini-simulador que detecte deadlocks visualizando el RAG en ASCII. Solo `std`:

```rust
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Un nodo del RAG: o un proceso o un recurso.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Nodo {
    Proceso(String),
    Recurso(String, usize), // nombre + instancias totales
}

/// Arista del RAG: Asignacion (R -> P) o Peticion (P -> R).
#[derive(Debug, Clone)]
pub enum Arista {
    Asignacion { desde: Nodo, hacia: Nodo },
    Peticion { desde: Nodo, hacia: Nodo },
}

pub struct RAG {
    pub nodos: HashSet<Nodo>,
    pub aristas: Vec<Arista>,
}

impl RAG {
    pub fn new() -> Self { Self { nodos: HashSet::new(), aristas: Vec::new() } }

    pub fn asignar(&mut self, r: &str, p: &str) {
        let rn = Nodo::Recurso(r.to_string(), 1);
        let pn = Nodo::Proceso(p.to_string());
        self.nodos.insert(rn.clone());
        self.nodos.insert(pn.clone());
        self.aristas.push(Arista::Asignacion { desde: rn, hacia: pn });
    }

    pub fn pedir(&mut self, p: &str, r: &str) {
        let rn = Nodo::Recurso(r.to_string(), 1);
        let pn = Nodo::Proceso(p.to_string());
        self.nodos.insert(rn.clone());
        self.nodos.insert(pn.clone());
        self.aristas.push(Arista::Peticion { desde: pn, hacia: rn });
    }

    /// Detecta si hay un ciclo en el RAG. Si lo hay, hay un deadlock.
    pub fn tiene_deadlock(&self) -> bool {
        // Construimos el grafo dirigido subyacente (sin distinguir tipo de arista).
        let mut adj: HashMap<&Nodo, Vec<&Nodo>> = HashMap::new();
        for n in &self.nodos { adj.insert(n, vec![]); }
        for a in &self.aristas {
            let (d, h) = match a {
                Arista::Asignacion { desde, hacia } => (desde, hacia),
                Arista::Peticion { desde, hacia } => (desde, hacia),
            };
            adj.get_mut(d).unwrap().push(h);
        }
        // DFS con marca de "en la pila".
        let mut visitado: HashSet<&Nodo> = HashSet::new();
        let mut en_pila: HashSet<&Nodo> = HashSet::new();
        for n in &self.nodos {
            if Self::dfs_ciclo(n, &adj, &mut visitado, &mut en_pila) {
                return true;
            }
        }
        false
    }

    fn dfs_ciclo<'a>(
        n: &'a Nodo,
        adj: &HashMap<&'a Nodo, Vec<&'a Nodo>>,
        visitado: &mut HashSet<&'a Nodo>,
        en_pila: &mut HashSet<&'a Nodo>,
    ) -> bool {
        if en_pila.contains(n) { return true; }
        if visitado.contains(n) { return false; }
        visitado.insert(n);
        en_pila.insert(n);
        for w in &adj[n] {
            if Self::dfs_ciclo(w, adj, visitado, en_pila) { return true; }
        }
        en_pila.remove(n);
        false
    }

    /// Dibuja el RAG en ASCII.
    pub fn ascii(&self) -> String {
        let mut s = String::new();
        s.push_str("  Recursos (■) y Procesos (●):\n");
        let procs: Vec<&Nodo> = self.nodos.iter().filter(|n| matches!(n, Nodo::Proceso(_))).collect();
        let recs: Vec<&Nodo> = self.nodos.iter().filter(|n| matches!(n, Nodo::Recurso(_, _))).collect();

        for r in &recs {
            if let Nodo::Recurso(nombre, inst) = r {
                s.push_str(&format!("  ■ {} ({} instancias)\n", nombre, inst));
            }
        }
        for p in &procs {
            if let Nodo::Proceso(nombre) = p {
                s.push_str(&format!("  ● {}\n", nombre));
            }
        }
        s.push_str("\n  Aristas:\n");
        for a in &self.aristas {
            let (d, h, flecha) = match a {
                Arista::Asignacion { desde, hacia } => (desde, hacia, "──asignado a──►"),
                Arista::Peticion { desde, hacia } => (desde, hacia, "──espera──►"),
            };
            s.push_str(&format!("    {} {} {}\n", d, flecha, h));
        }
        s
    }
}

impl fmt::Display for Nodo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nodo::Proceso(n) => write!(f, "P({})", n),
            Nodo::Recurso(n, _) => write!(f, "R[{}]", n),
        }
    }
}

fn main() {
    // Escenario 1: sin deadlock
    let mut rag1 = RAG::new();
    rag1.asignar("R1", "P1");
    rag1.asignar("R2", "P2");
    rag1.pedir("P1", "R3");
    println!("=== Escenario 1 ===");
    println!("{}", rag1.ascii());
    println!("¿Deadlock? {}\n", rag1.tiene_deadlock());

    // Escenario 2: con deadlock
    // P1 tiene R1, espera R2.
    // P2 tiene R2, espera R1.
    let mut rag2 = RAG::new();
    rag2.asignar("R1", "P1");
    rag2.asignar("R2", "P2");
    rag2.pedir("P1", "R2");
    rag2.pedir("P2", "R1");
    println!("=== Escenario 2 ===");
    println!("{}", rag2.ascii());
    println!("¿Deadlock? {}\n", rag2.tiene_deadlock());
}
```

Salida esperada:
```
=== Escenario 1 ===
  Recursos (■) y Procesos (●):
  ■ R1 (1 instancias)
  ■ R2 (1 instancias)
  ■ R3 (1 instancias)
  ● P1
  ● P2

  Aristas:
    R[R1] ──asignado a──► P(P1)
    R[R2] ──asignado a──► P(P2)
    P(P1) ──espera──► R[R3]
¿Deadlock? false

=== Escenario 2 ===
  Recursos (■) y Procesos (●):
  ■ R1 (1 instancias)
  ■ R2 (1 instancias)
  ● P1
  ● P2

  Aristas:
    R[R1] ──asignado a──► P(P1)
    R[R2] ──asignado a──► P(P2)
    P(P1) ──espera──► R[R2]
    P(P2) ──espera──► R[R1]
¿Deadlock? true
```

¿Ves la elegancia? Un deadlock es un **ciclo en el grafo**. La detección es un DFS con marca de pila (el mismo algoritmo que viste en el capítulo de Kosaraju). El RAG es el lenguaje común entre el sistema operativo y tú.

## 23.8 Diálogo de guardia nocturna

> — Bárbara, son las 3am. El servidor de pagos se ha colgado. ¿Cómo sé si es un deadlock?
> — Aplica el test de los cuatro: si ves un RAG con un ciclo, es deadlock. Si no, puede ser un livelock, un simple bloqueo de I/O, o que alguien olvidó un `await`.
> — Vale, ¿y cómo lo soluciono en caliente?
> — La opción "guarra" es matar el proceso de menor prioridad y liberar sus recursos. La opción "elegante" es esperar a que el timeout del lock expire. La opción "para que no vuelva a pasar" es imponer un orden de adquisición de locks y rezar.
> — ¿Orden de adquisición? ¿Como en Java con `synchronized` en un orden fijo?
> — Sí. Y en Rust, lo mismo: un struct `LockOrder` que documenta el orden de adquisición. Si alguien lo viola, el test de integración lo cazará.
> — Me apunto "imponer orden total" como bullet point en el postmortem.

## 23.9 Aplicaciones del mundo real

- **Linux kernel**: usa wait graphs, lock dependency graphs, y desde 2006 un lockdep validator que detecta potenciales deadlocks en tiempo de compilación/ejecución.
- **MySQL InnoDB**: mantiene un wait-for graph de transacciones. Si detecta un ciclo, mata la transacción de menor peso (la que ha hecho menos cambios) y emite un error `ER_LOCK_DEADLOCK`.
- **Java**: el JFR (Java Flight Recorder) captura grafos de threads y locks para analizar deadlocks post-mortem.
- **PostgreSQL**: usa un grafo de esperas similar a InnoDB.
- **Sistemas distribuidos (Hadoop, Kafka)**: deadlocks distribuidos. La detección requiere consenso entre nodos (algoritmos como Chandy-Misra-Haas).

## 23.10 Ejercicios resueltos

**Ejercicio 23.1.** Dado el RAG del escenario 2 del §23.7, identifica el ciclo y los procesos involucrados.

*Solución:* El ciclo es `P1 → R2 → P2 → R1 → P1`. Los procesos involucrados son P1 y P2. El recurso R1 está en poder de P1, R2 en poder de P2, y cada uno quiere el del otro.

**Ejercicio 23.2.** ¿Por qué "imponer un orden total sobre los recursos" rompe la condición de circular wait?

*Solución:* Si todos los procesos piden los recursos en el mismo orden (p. ej., R₁ antes que R₂ antes que R₃), entonces es imposible que un proceso A tenga R₁ y espere R₂ mientras otro B tiene R₂ y espera R₁: el que pide R₂ ya no puede tener R₁ antes (porque R₁ viene antes en el orden, lo pediría y liberaría antes). Por tanto, no puede haber ciclos de espera.

**Ejercicio 23.3.** En el algoritmo del banquero, demuestra que si un estado es inseguro, no todas las peticiones pueden ser concedidas.

*Solución:* Si el estado es inseguro, no existe ninguna secuencia de concesiones que permita a todos los procesos terminar. Esto significa que, en cualquier rama del "grafo de estados", al menos un proceso quedará sin recursos. Por tanto, conceder CUALQUIER petición adicional (que reduce `Work`) solo empeora la situación, no la mejora. Conclusión: en estado inseguro, la siguiente petición debe ser rechazada o diferida.

## 23.11 Ejercicios propuestos

1. Extiende el RAG del §23.7 para soportar **recursos multi-instancia** (un recurso R con N instancias). Ajusta la detección: solo hay deadlock si el ciclo pasa por al menos una instancia saturada.
2. Implementa el **algoritmo del banquero completo** (con `max`, `alloc`, `avail`) en Rust. Incluye una función `es_seguro()`.
3. Simula un **sistema de archivos** minimal como grafo: directorios como nodos, archivos como nodos, "contiene" como arista. Implementa `find` (path traversal) y `du` (suma de tamaños recursiva).
4. Escribe un detector de **topological order** para un grafo de tareas (compilar → test → deploy). Si hay ciclo, devuelve error.
5. Añade al RAG un **tercer tipo de arista**: `Preempcion { desde, hacia }` que simula que el SO le quita el recurso al proceso. ¿Cómo cambia la detección de ciclos?

## 23.12 Pin de batalla

- **Si un deadlock aparece solo en producción los viernes a las 3am, mira el cron, no la aplicación**. Jobs programados compiten por recursos a horas raras; ahí nacen los deadlocks.
- **Lockdep en Linux es tu amigo**. Compila tu kernel o driver con `CONFIG_PROVE_LOCKING=y` y te avisará de posibles ciclos de locks en tiempo de compilación.
- **El orden de adquisición de locks es ley**. Documéntalo en un comentario, en un test, en un README. Si un junior lo viola, no será culpa suya, será tuya por no haberlo dejado claro.
- **Timeouts son obligatorios en producción**. Un `lock.acquire(timeout=5s)` evita que un deadlock se cuelgue para siempre. Un `try_lock` con backoff es aún mejor.
- **En Rust, los tipos ayudan**. `Mutex` y `RwLock` previenen data races, pero NO deadlocks. El borrow checker no te salva de pedir dos locks en orden inverso en dos funciones distintas. Sé explícito.

## 23.13 Lo que te llevas

Un deadlock es un **ciclo en el Resource Allocation Graph**. Para que exista, deben cumplirse las 4 condiciones de Coffman. Prevenirlo = romper una de las cuatro (lo más común: orden total de adquisición). El algoritmo del banquero decide dinámicamente si una concesión es segura buscando una secuencia de finalizaciones en un grafo implícito de estados. Todo el modelo de un sistema operativo — procesos, recursos, memoria, archivos — se puede dibujar como grafos, y todos los problemas interesantes (deadlock, scheduling, fragmentación) se pueden reducir a traversals.

## 23.14 Ojo, cuidado con…

- **Lockdep solo detecta interbloqueos potenciales en tiempo de compilación**, no los garantiza. Un test en runtime sigue siendo necesario.
- **En sistemas distribuidos, la detección de deadlocks no es trivial**. Requiere consenso (Chandy-Misra-Haas) o un coordinador central. Latencia y particiones de red complican todo.
- **El algoritmo del banquero es conservador**. A veces rechaza peticiones que en la práctica no causarían deadlock (asume que todos los procesos piden su máximo de inmediato). En sistemas reales se usa con cuidado.

## 23.15 Para profundizar

- *"Operating Systems: Three Easy Pieces"* de Remzi Arpaci-Dusseau. Capítulo sobre deadlocks: explicación brillante y con sentido del humor.
- *"The Art of Multiprocessor Programming"* de Maurice Herlihy y Nir Shavit. Cubre wait-free algorithms y estructuras de datos lock-free.
- *"Cooperating Sequential Processes"* (1965), el paper original de Dijkstra. Difícil de leer pero histórico.
- Documentación de `lockdep` en el kernel de Linux: https://www.kernel.org/doc/Documentation/locking/lockdep-design.txt
- *"Database Transaction Models for Advanced Applications"* de Ahmed Elmagarmid (capítulo sobre deadlocks distribuidos).

## 23.16 Si solo lees 30 segundos

Un deadlock es un ciclo en un grafo (el RAG). El sistema operativo lo detecta con un DFS; tú lo previenes rompiendo una de las 4 condiciones de Coffman (lo más fácil: imponer un orden de adquisición de locks). El algoritmo del banquero es la versión "automática" de la misma idea, pero más conservadora. La próxima vez que veas un cuelgue misterioso, dibuja el RAG.

## 23.17 Una historia pequeña

Dijkstra el junior llevaba dos meses en su primer trabajo cuando un pipeline de datos se cayó. Cada noche, a las 3am, un cron ejecutaba dos jobs que acababan pillando los mismos archivos. El equipo llevaba semanas echando la culpa al scheduler. Una noche, con café y paciencia, Dijkstra el junior dibujó el RAG en una pizarra: Job A tenía `dataset_x.lock` y pedía `dataset_y.lock`; Job B tenía `dataset_y.lock` y pedía `dataset_x.lock`. Ciclo. Muerto. La solución fue tonta: cambiar el orden de adquisición de un lock. Tres líneas de código. A la mañana siguiente, el pipeline no se cayó. La senior le dijo: "bienvenido al club de los que duermen tranquilos". Y desde esa noche, Dijkstra el junior dibuja grafos antes de ir a dormir.

---

*Y con esto cerramos la Parte 6-A. Tres capítulos, tres dominios, un solo grafo.*
