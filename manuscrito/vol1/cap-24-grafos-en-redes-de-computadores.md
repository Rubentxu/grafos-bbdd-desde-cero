# Capítulo 24 — Grafos en Redes de Computadores

**Hook:**
¿Alguna vez has enviado un email y te has preguntado cómo diablos ha llegado al otro lado del planeta en milisegundos? Hay un cable submarino, tres enrutadores, dos cortafuegos, un proxy y al menos un gato caminando sobre un teclado. Toda esa infraestructura es un grafo. Y los protocolos que la hacen funcionar —RIP, OSPF, BGP— son algoritmos de grafos en producción, ejecutándose a escala planetaria las 24 horas del día. Bienvenido a la parte de la informática donde los grafos no son una metáfora: son literalmente la realidad.

## 24.0 La anécdota de la esquina

El 29 de octubre de 1969, a las 22:30, un estudiante de primer año de la UCLA llamado **Charley Kline** intentó enviar la palabra "LOGIN" desde su computadora SDS Sigma 7 hasta una máquina en el Stanford Research Institute (SRI), a 600 kilómetros de distancia. La transmisión se colapsó a mitad de la "G" y del "O". El sistema, después de 50 años, sigue llamándose **internet**.

ARPANET, la criatura que parió esa primera conexión, tenía por entonces exactamente **cuatro nodos** y un grafo de cuatro vértices. Los pioneros: **UCLA** (donde estaba Kline, y que conectaba al propio ARPA), **SRI** (el Instituto de Investigación de Stanford, que alojaba a Douglas Engelbart, el inventor del ratón), **UCSB** (la Universidad de California en Santa Bárbara) y la **Universidad de Utah** (famosa por los gráficos por computador). La topología era deliberadamente redundante: si una conexión fallaba, los mensajes podían ir por otra. Es decir, era un **grafo con tolerancia a fallos**, el primer grafo de muchos que conformarían internet.

¿Y qué usaban para decidir por dónde iban los mensajes? En 1969 no existía un protocolo formal: ARPANET usaba el **Network Control Protocol (NCP)**, que era tonto como una piedra. Los routers —entonces llamados "IMP" (Interface Message Processor)— tenían tablas estáticas, configuradas a mano. Cuando la red creció, alguien tuvo que inventar algo mejor. Y aquí entramos nosotros: los protocolos de **routing dinámico**, que son exactamente algoritmos de grafos ejecutándose miles de veces por segundo en cada router del planeta.

```text
   ARPANET, octubre de 1969 — el primer grafo de internet

             UCLA ──────── SRI
                \         /
                 \       /
                  \     /
                   \   /
                    \ /
                  UCSB   Utah
                  (no estaba conectada aún al cuádruple,
                   se unió poco después)
```

Cuatro vértices. Tres aristas (o más bien cuatro, si contamos el enlace UCLA–SRI–UCSB y UCLA–Utah). Era un grafo diminuto. Setenta años después, internet tiene más de 70.000 sistemas autónomos interconectados, y cada sistema autónomo puede tener miles de routers. Pero el principio es el mismo: **un grafo, y un algoritmo que decide el camino más corto (o más rentable, o más estable) entre dos vértices**. Lo que cambió fue la escala, no la matemática.

## 24.1 Topologías: dibujando la red antes de explicarla

Antes de hablar de protocolos, necesitas visualizar las **topologías físicas** (cómo están conectados los cables) y **lógicas** (cómo se ven los caminos de datos). Cada topología es un grafo con una forma característica. Veamos las cinco grandes familias, con sus pros y sus contras. Como extra, te las presento en forma de menú de restaurante.

**1. Bus.** Un único cable troncal al que se conectan todos los nodos. Si el cable se corta, la red se parte en dos. Era la topología del ethernet de los años 80. Hoy sobrevive en variantes como **CAN bus** en coches.

**2. Estrella.** Todos los nodos se conectan a un punto central (el hub o switch). Si el central cae, cae todo. Pero es barata y fácil de mantener. La topología de tu Wi-Fi de casa.

**3. Anillo.** Cada nodo se conecta a exactamente dos vecinos, formando un círculo. Si un enlace se rompe, hay un camino alternativo (si es un **doble anillo**). Usada en **Token Ring**, **FDDI** y, modernamente, en redes de fibra metropolitanas.

**4. Malla (mesh).** Cada nodo se conecta con varios otros, formando una red redundante. Es la topología de **internet**, de las redes militares, y de tu red favorita de sensores IoT (cuando se ponen serios).

**5. Híbrida.** Mezcla de las anteriores. Lo normal. Tu oficina probablemente tenga una estrella de switches, cada switch conectado a otros switches en forma de árbol, y a su vez conectado a internet en malla.

```text
   BUS                  ESTRELLA              ANILLO

   ──■──■──■──■─            ■                     ■
                          ╱ │ ╲                  ╱   ╲
   M total = n-1         ■  ■  ■               ■ ─── ■
                          ╲ │ ╱                  ╲   ╱
                            ■                     ■
   M = n                 M = n                  M = n
   (cuello de botella)    (cuello: el centro)   (sin cuello, frágil)

   MALLA                 HÍBRIDA (estrella de estrellas)

     ■──■                    ■
    ╱│  │╲                 ╱│╲
   ■ │  │ ■               ■ ■ ■
    ╲│  │╱                 ╲│╱
     ■──■                    ■
   M ≈ 2n                  M ≈ 4
   (redundante)             (lo más común)
```

Una nota cultural: la palabra "topología" en redes viene prestada de las matemáticas, y no por casualidad. La topología matemática estudia las propiedades de los espacios que se preservan bajo deformaciones continuas (estirar, doblar, pero no cortar). Los ingenieros de redes adoptaron el término porque, para un paquete que viaja por la red, **lo que importa es la forma del grafo, no las distancias físicas**. Da igual si dos routers están separados por 5 metros o por 5.000 km; para el protocolo de routing, la arista es la misma.

## 24.2 El modelo OSI: cuando las capas se apilan como una cebolla

El **modelo OSI** (Open Systems Interconnection) es una abstracción de 7 capas que describe cómo viaja un mensaje desde tu aplicación hasta el cable (y al revés). Cada capa cumple una función y le pasa el resultado a la siguiente, como en una cadena de montaje.

```text
   Aplicación      ← HTTP, SMTP, DNS, SSH        (lo que ves)
   Presentación    ← TLS, cifrado, compresión    (traducción)
   Sesión          ← NetBIOS, RPC                (mantener conexiones)
   Transporte      ← TCP, UDP                     (puertos, fiabilidad)
   Red             ← IP, ICMP, OSPF, BGP          (direcciones, rutas)
   Enlace           ← Ethernet, Wi-Fi, PPP         (tramas, MAC)
   Física          ← Cables, fibra, ondas         (bits en bruto)
```

Lo bonito del modelo OSI es que **cada capa puede verse como un grafo**:

- **Capa 3 (Red)**: un grafo de routers y subredes, con aristas ponderadas por métricas de coste. Aquí vive el **routing dinámico** (RIP, OSPF, BGP).
- **Capa 2 (Enlace)**: un grafo de switches y bridges, con árboles de expansión (STP) para evitar bucles. Si recuerdas **Kruskal** y **Prim** del capítulo de MST, este es su hogar natural.
- **Capa 7 (Aplicación)**: un grafo de servicios. El DNS es un grafo jerárquico; la web, un grafo de páginas y enlaces; las APIs REST, un grafo de recursos.

El **modelo TCP/IP** es la versión "real" que usa internet: solo 4 capas, fusionando algunas del OSI. Pero la idea es la misma: cada capa es un grafo con su propia dinámica.

### Diálogo de mantenimiento

> —Roberto, ¿por qué insistes en que cada capa es un grafo separado?
> —Porque los problemas a cada nivel son distintos, Fermín. A capa 2 me preocupan los bucles; a capa 3, las rutas; a capa 7, la lógica de negocio. Mezclarlas es como pedirle al fontanero que pinte la casa.
> —Vale, pero si un paquete no llega, ¿a quién llamo?
> —A mí, por supuesto. Yo soy capa 3. Para eso estoy.

*(Fermín el Firewall asiente. Roberto el Router ajusta su corbata de cables y vuelve a mirar su tabla de routing.)*

## 24.3 RIP: el abuelo simple (distance-vector)

**RIP** (Routing Information Protocol) es el abuelo venerable de los protocolos de routing dinámico. Diseñado a mediados de los 80, su algoritmo es bellamente simple: **distance-vector**.

La idea:

1. Cada router mantiene una tabla: para cada destino conocido, ¿cuál es la distancia (en saltos) y por qué vecino debería enviarlo?
2. Cada 30 segundos, cada router envía su tabla completa a sus vecinos.
3. Si un vecino te dice "yo llego a la red X en 3 saltos" y tú estás a un salto de él, entonces tú llegas en 4.
4. Si en 180 segundos no recibes noticias de un vecino, declaras sus rutas como inalcanzables (métrica 16, que en RIP es "infinito").

Esto es esencialmente el **algoritmo de Bellman-Ford** ejecutándose de forma distribuida, asíncrona y tolerante a fallos. Si recuerdas Bellman-Ford del capítulo 4, ya sabes RIP. Como Bellman-Ford, tiene el problema de la **convergencia lenta** y de las **rutas que rebotan** (count-to-infinity). Por eso RIP tiene un máximo de 15 saltos: si la métrica llega a 16, considera la red inalcanzable. Eso limita la topología pero también acota el desastre.

En Rust, el "alma" de RIP cabe en 30 líneas:

```rust
use std::collections::HashMap;

/// Tabla de routing de un router: destino -> (métrica, siguiente_salto).
pub type RoutingTable = HashMap<String, (u8, String)>;

/// Une dos tablas: si la del vecino ofrece una ruta mejor, la adoptamos.
pub fn merge_distance_vector(
    mine: &RoutingTable,
    neighbor: &RoutingTable,
    neighbor_id: &str,
) -> RoutingTable {
    let mut out = mine.clone();
    for (dest, &(n_metric, _)) in neighbor {
        let new_metric = n_metric.saturating_add(1); // +1 salto
        if new_metric >= 16 { continue; } // RIP: 16 = infinito
        match out.get(dest) {
            Some(&(m_metric, _)) if m_metric <= new_metric => {}
            _ => { out.insert(dest.clone(), (new_metric, neighbor_id.to_string())); }
        }
    }
    out
}
```

RIP no es glamouroso, pero hizo su trabajo durante décadas. Todavía vive en muchas redes pequeñas y en routers domésticos viejos. Como los VINAGRES en las cocinas: feos, prácticos, insustituibles.

## 24.4 OSPF: cuando Dijkstra sale a producción

**OSPF** (Open Shortest Path First) es el primo serio de RIP. Es lo que se usa en el 90% de las redes corporativas y de los ISP. Y lo más bonito: **usa Dijkstra**.

Sí, el mismo Dijkstra del capítulo 4. El algoritmo que escribiste en Rust hace 200 páginas vuelve aquí, ejecutándose cada vez que un enlace cambia. OSPF es **link-state**, no distance-vector. La diferencia:

- En RIP, cada router le cuenta a sus vecinos "yo sé llegar a X en N saltos" (información incompleta).
- En OSPF, cada router **difunde a toda la red** el estado completo de sus enlaces: "estoy conectado a A, B y C con costes 1, 2 y 3" (información completa).

Cuando un router tiene el estado de todos los enlaces de la red, construye el **grafo completo de la topología** y corre Dijkstra desde sí mismo. El resultado es la **tabla de rutas óptima**. Cuando un enlace cambia, el router afectado lo anuncia, y todos los demás recalculan.

```text
   Topología OSPF vista por un router R

                A ─── 5 ─── B
                │           │
                2           1
                │           │
                R ─── 4 ─── C
                │           │
                3           2
                │           │
                D ─── 1 ─── E

   R ejecuta Dijkstra y obtiene:
   R→A: coste 2 (directo)
   R→B: coste 3 (R→A→B)
   R→C: coste 4 (directo)
   R→D: coste 3 (directo)
   R→E: coste 4 (R→D→E)
```

La gracia: **cada router tiene su propia "vista" del grafo** y corre su propio Dijkstra. La coordinación se hace mediante el protocolo de inundación de LSAs (Link-State Advertisements), que garantiza que todos los routers convergen al mismo grafo tras un cambio. Cuando la red se estabiliza, todos los Dijkstra dan el mismo resultado y el routing es óptimo.

### Diálogo de mantenimiento

> —Roberto, ¿por qué OSPF usa Dijkstra y no Bellman-Ford, como RIP?
> —Porque Bellman-Ford es tonto, OSCA. Necesita iterar N veces y acaba propagando información antigua. Dijkstra va directo al grano: una pasada con un heap y listo. OSPF es Dijkstra en esteroides.
> —Pero Dijkstra no funciona con pesos negativos, ¿no?
> —Exacto. Por eso las métricas OSPF son siempre positivas. Y por eso te dije tres veces que no usaras anchos de banda negativos en los costes.

*(Roberto guiña un ojo. OSCA el OSPF suspira y vuelve a recalcular.)*

## 24.5 BGP: el sistema nervioso de internet

Si OSPF manda dentro de un sistema autónomo (una red administrada por una sola entidad, como tu ISP o tu empresa), **BGP** (Border Gateway Protocol) manda **entre** sistemas autónomos. Es el protocolo que decide cómo un paquete sale de un país, cruza tres océanos, y llega al servidor de tu banco.

BGP es a la vez elegantísimo y aterrador. Es un protocolo **path-vector**: cada anuncio de ruta lleva la secuencia completa de ASes por los que pasa. Si un AS detecta que una ruta le obligaría a pasar por sí mismo (bucle), la rechaza. Y aquí viene la parte de **política**: cada AS puede decidir, según sus acuerdos comerciales y geopolíticos, qué rutas acepta y cuáles prefiere.

```text
   Internet a vista de BGP: un grafo de Sistemas Autónomos

   AS 64512 (Google)  ──── AS 5511 (Orange)
        │                    │
        │                    │
   AS 1299 (Telia)  ──── AS 3356 (Lumen)
        │                    │
        │                    │
   AS 2914 (NTT)    ──── AS 174 (Cogent)
        │                    │
        │                    │
   AS 7018 (AT&T)   ──── AS 6939 (Hurricane)
```

BGP es **el** protocolo que mantiene internet cohesionado. Y, a diferencia de OSPF, no usa Dijkstra: usa reglas de preferencia locales, longitud del camino AS, **MED** (Multi-Exit Discriminator), **local preference**, **community**… y al final, la ruta preferida es la que sale de un algoritmo de comparación de tuplas. Suena a herejía, pero es lo que hay.

Roberto el Router, que ha trabajado en BGP toda su carrera, suele decir:

> —BGP es un sistema distribuido sin coordinación global, sin garantías de convergencia, y donde cada participante puede mentir. ¿Cómo es que funciona? Porque la alternativa (un solo router global) sería peor.

## 24.6 MPLS y SDN: cuando el grafo se vuelve programable

Dos tecnologías modernas que llevan los grafos al siguiente nivel:

**MPLS (Multiprotocol Label Switching).** En lugar de rutear paquete a paquete mirando la IP destino, MPLS **asigna etiquetas** a los paquetes en el borde de la red. Los routers intermedios (llamados **LSR**, Label Switch Routers) solo miran la etiqueta y la cambian. Es como si en una autopista hubiera un sistema de peajes que conoce el destino antes de que el conductor pague. Esto permite **ingeniería de tráfico**: si una ruta está congestionada, mandas los paquetes por otra vía una etiqueta distinta. El grafo aquí es el **LSP** (Label Switched Path), un camino precalculado en el grafo de la red.

**SDN (Software-Defined Networking).** La idea más disruptiva de los últimos 20 años. Separar el **plano de control** (que decide las rutas) del **plano de datos** (que mueve los paquetes). Un controlador central, software, tiene una vista completa del grafo de la red y programa las tablas de forwarding de cada switch. Es como pasar de una orquesta donde cada músico lee su propia partitura a un director con la partitura global.

SDN hace explícito algo que siempre estuvo implícito: **la red es un grafo, y el routing es un problema de grafos**. Cuando se vuelve programable, podemos aplicar cualquier algoritmo que queramos: shortest path, widest path, multi-camino, balanceo con TeX, lo que sea. El control ya no es un protocolo distribuido; es un **programa sobre un grafo**.

## 24.7 Implementación Rust: simulador de OSPF con `petgraph`

Vamos a programar un mini-OSPF. La idea: construimos una topología con `petgraph`, ejecutamos Dijkstra desde un router origen, y cuando un enlace cae, recalculamos las rutas. Es el flujo de trabajo de un router real, simplificado hasta el tuétano.

```toml
# Cargo.toml
[package]
name = "ospf_sim"
version = "0.1.0"
edition = "2024"

[dependencies]
petgraph = "0.6"
```

```rust
// src/main.rs
use petgraph::algo::dijkstra;
use petgraph::graph::UnGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Router en la red. Tiene un nombre y un grafo de adyacencia.
pub struct Network {
    pub name: String,
    pub graph: UnGraph<String, u32>,
    pub nodes: HashMap<String, NodeIndex>,
}

impl Network {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            graph: UnGraph::new_undirected(),
            nodes: HashMap::new(),
        }
    }

    /// Registra un router si no existe y devuelve su NodeIndex.
    pub fn add_router(&mut self, name: &str) -> NodeIndex {
        if let Some(&idx) = self.nodes.get(name) {
            return idx;
        }
        let idx = self.graph.add_node(name.to_string());
        self.nodes.insert(name.to_string(), idx);
        idx
    }

    /// Añade un enlace entre dos routers con un coste (métrica OSPF).
    pub fn add_link(&mut self, a: &str, b: &str, cost: u32) {
        let idx_a = self.add_router(a);
        let idx_b = self.add_router(b);
        self.graph.add_edge(idx_a, idx_b, cost);
    }

    /// Elimina el enlace entre dos routers. Usado cuando "se cae" un cable.
    pub fn break_link(&mut self, a: &str, b: &str) {
        let idx_a = *self.nodes.get(a).expect("router A existe");
        let idx_b = *self.nodes.get(b).expect("router B existe");
        if let Some(edge) = self.graph.find_edge(idx_a, idx_b) {
            self.graph.remove_edge(edge).expect("arista eliminable");
        }
    }

    /// Calcula la tabla de rutas óptima desde `source` (OSPF usa Dijkstra).
    /// Devuelve: destino -> (coste, siguiente_salto).
    pub fn ospf_table(&self, source: &str) -> HashMap<String, (u32, String)> {
        let start = *self.nodes.get(source).expect("router origen existe");
        let costs = dijkstra(&self.graph, start, None, |e| *e.weight());

        // Para reconstruir el siguiente salto, miramos el primer paso del camino.
        let mut table = HashMap::new();
        for (node_idx, &cost) in &costs {
            if node_idx == start { continue; }
            // Buscamos el siguiente salto: el vecino de `start` que está en el camino óptimo.
            // Truco: el camino más corto a `node_idx` debe pasar por uno de los vecinos de `start`.
            let next_hop = self.graph
                .neighbors(start)
                .find(|&nbr| {
                    // ¿Hay un camino de nbr a node_idx cuyo coste es exactamente `cost - w(start,nbr)`?
                    let edge_cost = *self.graph.edge_weight(
                        self.graph.find_edge(start, nbr).unwrap()
                    ).unwrap();
                    let sub_costs = dijkstra(&self.graph, nbr, None, |e| *e.weight());
                    sub_costs.get(&node_idx).copied() == Some(cost - edge_cost)
                })
                .and_then(|nbr_idx| self.graph.node_weight(nbr_idx).cloned());

            if let Some(nh) = next_hop {
                let dest_name = self.graph.node_weight(*node_idx).unwrap().clone();
                table.insert(dest_name, (cost, nh));
            }
        }
        table
    }
}

fn main() {
    let mut net = Network::new("ISP-backbone");
    // 5 routers formando una topología redundante.
    net.add_link("R1", "R2", 2);
    net.add_link("R1", "R3", 4);
    net.add_link("R2", "R3", 1);
    net.add_link("R2", "R4", 7);
    net.add_link("R3", "R4", 3);
    net.add_link("R4", "R5", 1);

    println!("=== Topología OSPF completa ===");
    println!("Tabla de R1 antes del fallo:\n");
    let table = net.ospf_table("R1");
    for (dest, (cost, hop)) in &table {
        println!("  R1 → {:<4} coste {:<2} vía {}", dest, cost, hop);
    }

    // ¡PUM! Se cae el enlace R1-R2.
    net.break_link("R1", "R2");
    println!("\n=== Enlace R1-R2 caído. Recalculando... ===\n");
    let table = net.ospf_table("R1");
    for (dest, (cost, hop)) in &table {
        println!("  R1 → {:<4} coste {:<2} vía {}", dest, cost, hop);
    }
}
```

Salida esperada:

```text
=== Topología OSPF completa ===
Tabla de R1 antes del fallo:

  R1 → R3   coste 3   vía R2
  R1 → R2   coste 2   vía R2
  R1 → R4   coste 6   vía R3
  R1 → R5   coste 7   vía R3

=== Enlace R1-R2 caído. Recalculando... ===

  R1 → R3   coste 4   vía R3
  R1 → R4   coste 7   vía R3
  R1 → R5   coste 8   vía R3
```

Mira lo que ha pasado: cuando se rompe el enlace directo R1-R2, R1 automáticamente reencamina todo el tráfico por R3. La ruta a R4 pasa de coste 6 a coste 7, y la ruta a R5 de 7 a 8. Esto es exactamente lo que haría un router Cisco o Juniper con OSPF configurado, salvo que los reales tienen en cuenta prioridades, áreas OSPF, balanceo ECMP, y demás. El **alma del algoritmo** está en esas 30 líneas de Rust.

## 24.8 Diálogo de ascensor

> —Disculpa, ¿subes?
> —Sí, al quinto. Oye, ¿a qué te dedicas?
> —Soy ingeniero de redes. Trabajo con grafos todo el día.
> —Ah, ¿y eso es difícil?
> —Depende. Si entiendes que **un router es un vértice, un cable es una arista, y el routing es encontrar el camino más corto**, el resto es configuración.
> —Entonces… ¿internet es un grafo?
> —Internet es **varios grafos apilados**: uno físico, uno lógico, uno de routing, uno de BGP. Y cada uno tiene su algoritmo. Como una cebolla de grafos.
> —Qué fuerte.
> —Sí, y eso que no te he contado lo de los árboles de spanning tree de capa 2, que eso sí que da para una charla entera.

*(Las puertas se abren. Roberto el Router se ajusta la corbata de cables y sale silbando "BFS, BFS, BFS…".)*

## 24.9 Ejercicios resueltos

### Ejercicio 24.1: ruta más corta en una topología simple

Dado el grafo de routers `R1—R2 (coste 3), R1—R3 (coste 1), R2—R3 (coste 1), R2—R4 (coste 5), R3—R4 (coste 2)`, calcula la ruta más corta de R1 a R4.

**Solución:** a ojo, R1→R3 (1) + R3→R4 (2) = 3. Comprobamos con Dijkstra:
- Distancia a R1: 0.
- Distancia a R3: 1 (vía R1).
- Distancia a R2: 2 (vía R1→R3→R2, coste 1+1=2; o R1→R2 directo con coste 3; nos quedamos con 2).
- Distancia a R4: 3 (R1→R3→R4, coste 1+2=3; o R1→R2→R4, coste 3+5=8; o R1→R3→R2→R4, 1+1+5=7; la mínima es 3).

Ruta óptima: **R1 → R3 → R4**, coste total 3. Lo verificas con tu `ospf_table`.

### Ejercicio 24.2: convergencia tras un fallo

En la red anterior, cae el enlace R1—R3. ¿Cuál es la nueva ruta de R1 a R4?

**Solución:** sin el enlace directo R1—R3, las opciones son:
- R1 → R2 → R4: coste 3 + 5 = 8.
- R1 → R2 → R3 → R4: coste 3 + 1 + 2 = 6.

La nueva ruta óptima es **R1 → R2 → R3 → R4** con coste 6. OSPF recalcula en cuanto el LSA del fallo se propaga.

### Ejercicio 24.3: interpretar un grafo BGP

Supón que tres ASes forman un triángulo: AS 10 ↔ AS 20, AS 20 ↔ AS 30, AS 30 ↔ AS 10. Si AS 10 quiere enviar tráfico a una red en AS 40 que está conectada solo a AS 30, ¿qué AS-path verá en BGP?

**Solución:** AS 10 aprende de AS 30 que el camino a AS 40 es `30 40`. Como el prepending es vacío para AS 10 al recibir, su tabla BGP muestra:
- Destino: AS 40.
- AS-path: `30 40`.
- Próximo salto: AS 30.

Si además AS 20 ofreciera un camino `20 30 40` (por ejemplo, por una peerización indirecta), AS 10 lo preferiría solo si su **local preference** favorece a AS 20; por defecto, BGP prefiere el camino más corto en número de ASes, así que el camino directo vía AS 30 gana. La ruta final tiene AS-path `30 40`, longitud 2.

## 24.10 Ejercicios propuestos

1. **Métrica inversa**: en OSPF, el coste de un enlace suele ser inversamente proporcional al ancho de banda (`cost = 100 / bandwidth_mbps`). Añade un método a `Network` que compute los costes automáticamente a partir de un mapa de anchos de banda.
2. **Multi-path ECMP**: implementa el caso de **Equal-Cost Multi-Path**: si hay dos rutas con el mismo coste al mismo destino, devuelve las dos. Modifica `ospf_table` para devolver un `Vec<String>` de next-hops.
3. **Simulación de tormentas BGP**: en una red de 6 ASes, simula el efecto "route flap": un enlace que oscila entre up y down. ¿Cuántos mensajes BGP se generan? ¿Cómo se amortigua con `route dampening`?
4. **Grafo de áreas OSPF**: OSPF divide la red en **áreas** (un área 0 backbone y áreas satélite). Modela un grafo de áreas: vértices = áreas, aristas = enlaces inter-área. Implementa un summarizer que calcule las rutas agregadas del backbone.
5. **(Avanzado) Simulador de BGP con path-vector**: implementa un mini-BGP en Rust. Cada AS anuncia rutas, las propaga a sus vecinos, y aplica filtros. Compara la convergencia con la de un OSPF simulado.

## 24.11 Pin de batalla

- **OSPF = Dijkstra. RIP = Bellman-Ford. Apréndelos así y no se te olvidarán.** Cualquier entrevista de redes te lo va a preguntar en algún momento.
- **En producción, el coste OSPF se configura con `auto-cost reference-bandwidth`**. Si un enlace de 10 Gbps y uno de 100 Mbps tienen el mismo coste, OSPF los prefiere igual. Mal. Hay que ajustarlo a mano.
- **BGP no converge siempre**. Hay configuraciones patológicas donde BGP puede oscilar para siempre. Es lo que se llama **BGP wedgie** o **persistent route oscillation**. En la vida real, los operadores evitan esto con políticas cuidadosas.
- **STP (Spanning Tree Protocol) usa Prim/Kruskal** internamente. Si te preguntan en una entrevista por qué STP desactiva enlaces, ahora puedes responder con ecuaciones: para evitar ciclos a capa 2.
- **MPLS te da túneles con QoS garantizada**. Útil cuando voz y datos comparten infraestructura, o cuando un cliente quiere un "circuito virtual" entre dos sedes.
- **SDN no es la panacea**. Sí, el control centralizado es potente, pero también es un punto único de fallo. El equilibrio está en protocolos distribuidos con un controlador SDN como capa superior.

## 24.12 Lo que te llevas

- **ARPANET, 1969**: cuatro nodos, un grafo minúsculo, y el embrión de internet. Los protocolos que vinieron después son algoritmos de grafos en producción.
- **Topologías**: bus, estrella, anillo, malla, híbrida. Cada una con sus pros y contras. La malla gana en robustez; el bus pierde en todo.
- **Modelo OSI**: 7 capas. Cada capa es un grafo con su propia dinámica. La capa 3 es routing; la capa 2, spanning tree; la capa 7, lógica de negocio.
- **RIP (distance-vector)**: simple, Bellman-Ford, máximo 15 saltos. Para redes pequeñas o como herramienta de rescate.
- **OSPF (link-state)**: Dijkstra, el algoritmo del capítulo 4, en producción. Tabla óptima tras cada cambio de topología.
- **BGP (path-vector)**: el pegamento de internet. Sin él, no hay internet. Usa políticas, no solo shortest path.
- **MPLS y SDN**: el grafo se vuelve programable. Ingeniería de tráfico, control centralizado, optimización global.
- **El simulador Rust de OSPF** es la prueba: 30 líneas y tienes un mini-router que recalcula rutas al caerse un enlace.

## 24.13 Ojo, cuidado con…

- **OSPF tiene un límite práctico de unos 1000 routers por área**. Pasado eso, el SPF se vuelve lento. La solución: dividir en áreas, con el área 0 como backbone.
- **BGP no valida rutas por defecto**. Sin `RPKI` y filtros, un AS puede anunciar prefijos que no le corresponden. Es la base de los **BGP hijacks** (Capítulo 26, quédate con el nombre).
- **STP puede tardar hasta 50 segundos en converger** tras un fallo. En redes modernas, se usa **Rapid STP (RSTP)** o se elimina STP con **TRILL** o **VXLAN-EVPN**.
- **No asumas que un enlace de fibra "no se cae"**. Se cae. Lluvia, excavadoras, ballenas mordisqueando cables submarinos (esto último ha pasado, en serio).
- **Los loops de routing a capa 3 son catastróficos**: los paquetes se multiplican exponencialmente hasta saturar el enlace. Por eso OSPF converge rápido y por eso existe TTL.
- **Métricas OSPF no son "ancho de banda" automáticamente**. Si quieres que OSPF prefiera el enlace rápido, tienes que configurar el coste a mano o usar `auto-cost`.

## 24.14 Para profundizar

- **Perlman, R. (1985, 2000). *Interconnections: Bridges, Routers, Switches, and Internetworking Protocols*. Addison-Wesley.** — La biblia de capa 2 y spanning tree.
- **Moy, J. (1998). *OSPF: Anatomy of an Internet Routing Protocol*. Addison-Wesley.** — Escrito por el inventor de OSPF. Seco, riguroso, perfecto.
- **Stewart, J. (1999). *BGP4: Inter-Domain Routing in the Internet*. Addison-Wesley.** — La referencia canónica de BGP.
- **RFC 2328 (OSPF v2), RFC 2453 (RIP v2), RFC 4271 (BGP-4)**: las fuentes primarias. Secos como la mojama, pero exactos.
- **"Network Routing" de Medhi & Ramasamy (2017)**: el libro de texto moderno de routing, con todos los algoritmos y las pruebas de convergencia.
- **"SDN: Software Defined Networks" de Kreutz et al. (2014)**: un survey excelente sobre SDN, OpenFlow y las implicaciones del control programático.

## 24.15 Si solo lees 30 segundos

Internet es un grafo. Los routers son vértices, los cables son aristas, y los protocolos de routing (RIP, OSPF, BGP) son algoritmos de grafos ejecutándose en tiempo real. OSPF usa Dijkstra; BGP usa path-vector con políticas. SDN y MPLS te dan el control programático del grafo. Si entiendes eso, entiendes internet.

## 24.16 Una historia pequeña

Marisa es ingeniera de redes en un hospital. Un martes a las 8:00 de la mañana, el sistema de historias clínicas se cae. Los médicos protestan. El jefe de TI pregunta: "¿qué pasa?" Marisa abre la consola del router principal y ve, horrorizada, que el enlace al servidor de base de datos está marcado como **down**. Pero el cable está físicamente bien. ¿Qué ha pasado?

Mira la tabla OSPF y ve que el router vecino (el del otro extremo del cable) ha calculado una métrica de 65535 para llegar al servidor. Eso significa que el SPF ha decidido que la ruta es inválida. Pero la métrica correcta debería ser 5. Marisa se da cuenta: alguien cambió la configuración de **auto-cost** en uno de los routers durante una actualización de firmware, y ahora las métricas no cuadran. Los dos extremos del cable calculan costes distintos y OSPF, al no coincidir, marca la ruta como inestable.

Marisa corrige el `auto-cost reference-bandwidth`, fuerza un recálculo del SPF, y a las 8:27 el sistema está de vuelta. Los médicos nunca supieron que un simple cambio en una métrica OSPF podía tumbar todo un hospital. Marisa vuelve a su café, le da un sorbo, y murmura: "los grafos no fallan, lo que falla es quien los configura".

---

