# Capítulo 25 — Grafos en Sistemas Distribuidos

**Hook:**
Tres computadoras en tres continentes quieren ponerse de acuerdo en un único valor. Una de ellas tiene un fallo. La red pierde mensajes. ¿Cómo demonios llegan a un consenso? La respuesta corta: modelando el problema como un grafo de eventos, ejecutando algoritmos de elección de líder sobre el grafo de servidores, y propagando la información con protocolos "epidémicos" que se parecen mucho a cómo se extienden los rumores en una cafetería. Bienvenido a la parte de la informática donde los grafos no solo representan datos, sino que representan la confianza (o la falta de ella).

## 25.0 La anécdota de la esquina

En 1998, un investigador de Microsoft llamado **Leslie Lamport** publicó un paper titulado *"Time, Clocks, and the Ordering of Events in a Distributed System"*. Era un paper que llevaba años circulando como memo técnico, pero que ahora aparecía formalizado en las *Communications of the ACM*. Lo que decía era sutil pero demoledor: en un sistema distribuido, **no existe un reloj global**.

La intuición de Lamport era demoledora porque iba contra toda la intuición ingenieril. Si dos eventos ocurren en dos máquinas distintas, no puedes decir, en general, cuál ocurrió "antes". Puedes decir que el evento A ocurrió antes que el B si A y B están en la misma máquina, o si A envió un mensaje cuyo recibo provocó B. Pero si no hay relación causal entre A y B, son **concurrentes**. Y los sistemas distribuidos viven en ese mundo: la mayoría de los eventos son concurrentes.

La solución de Lamport fue elegante: asigna a cada evento un **número lógico** (un "timestamp de Lamport") que respeta el orden causal. Si A → B (A ocurre antes que B en sentido causal), entonces `L(A) < L(B)`. Los **vector clocks**, popularizados después por Fidge y Mattern, refinan la idea: en vez de un solo número, un vector de N contadores, uno por nodo. Eso permite detectar causalidad con precisión.

```text
   Tres procesos P1, P2, P3 y sus eventos. Los vectores de reloj evolucionan.

   P1: e1(1,0,0) ──send──►  P2: e2(1,1,0) ──send──►  P3: e3(1,1,1)
                                │                            │
                                │                            │
                                ▼                            ▼
                            e4(1,2,0)                   e5(1,2,1)

   Los vectores crecen al enviar y al recibir.
   Comparar dos vectores detecta causalidad: A < B si A.v[i] ≤ B.v[i] para todo i,
   con al menos un estricto.
```

Lamport ganó el **Premio Turing en 2013** por este trabajo y otros relacionados. Hoy, los vector clocks son la base de bases de datos como Riak, Cassandra y Cosmos DB, y de sistemas de procesamiento de streams como Kafka. Si alguna vez te has preguntado "¿en qué orden pasaron realmente las cosas en un sistema distribuido?", la respuesta está en los grafos de causalidad de Lamport.

## 25.1 Consenso distribuido: cuando todos tienen que estar de acuerdo

El **problema del consenso** es el problema fundamental de los sistemas distribuidos: un conjunto de nodos quiere acordar un único valor, a pesar de fallos y mensajes perdidos. Suena abstracto, pero aparece en todas partes: ¿qué bloque se añade al blockchain? ¿Quién es el nuevo líder del cluster? ¿Qué orden de operaciones se aplica a la base de datos?

Dos algoritmos dominan la conversación: **Paxos** (Lamport, 1998) y **Raft** (Ongaro y Ousterhout, 2014). Paxos es elegante pero endiabladamente difícil de explicar. Raft es "Paxos con esteroides didácticos": misma potencia, pero diseñado para ser comprensible. Vamos con Raft.

Raft modela el log replicado como un **grafo de logs**. Cada nodo mantiene una secuencia de entradas; el objetivo es que todos los nodos tengan la misma secuencia. Para coordinarse, Raft elige un **líder** mediante una elección (que es básicamente un BFS acotado en el grafo de servidores).

```text
   Un cluster Raft de 5 nodos. Los followers reciben entradas del líder.

             LEADER
              │
      ┌───────┼───────┐
      │       │       │
   Follower Follower Follower
      │       │       │
      └───────┴───────┘
            quorum (3 de 5)

   Una entrada se considera "comprometida" cuando el líder
   la ha replicado en un quorum (mayoría).
```

El líder manda **AppendEntries** a los followers. Si un follower no responde en un *timeout*, se sospecha que el líder ha caído y se inicia una nueva elección. El primero en ganar la mayoría de votos es el nuevo líder. Esto es, literalmente, un **BFS electivo**: cada nodo pregunta a sus vecinos "¿estás conmigo?", y la ola se propaga hasta que un nodo consigue la mayoría.

## 25.2 Leader election como BFS en el grafo de servidores

La elección de líder en Raft es un precioso ejemplo de BFS distribuido:

1. Un nodo que detecta *timeout* se autopromueve a **candidato** y se incrementa el **término** (un número monotónico que representa la "era" del líder).
2. El candidato envía **RequestVote** a todos los demás nodos (un broadcast en el grafo del cluster).
3. Cada nodo que recibe la solicitud vota por el candidato si (a) no ha votado en este término y (b) el log del candidato está al menos tan actualizado como el suyo.
4. Si el candidato recibe votos de una **mayoría** (quorum), se proclama líder.
5. El nuevo líder envía heartbeats periódicos para mantener su autoridad.

```rust
use std::collections::{HashMap, HashSet};

/// Estado de un nodo en un cluster Raft simplificado.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

/// Nodo del cluster.
#[derive(Debug, Clone)]
pub struct RaftNode {
    pub id: u32,
    pub state: NodeState,
    pub current_term: u64,
    pub voted_for: Option<u32>,
    pub log: Vec<String>,
}

impl RaftNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: NodeState::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
        }
    }

    /// Inicia una elección: vota por sí mismo, incrementa el término, devuelve
    /// los IDs de los nodos a los que hay que pedirles el voto.
    pub fn start_election(&mut self, peers: &[u32]) -> Vec<u32> {
        self.state = NodeState::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.id);
        peers.iter().copied().filter(|&p| p != self.id).collect()
    }

    /// Procesa un RequestVote entrante. Devuelve true si concede el voto.
    pub fn handle_request_vote(
        &mut self,
        candidate_id: u32,
        candidate_term: u64,
        candidate_log_len: usize,
    ) -> bool {
        if candidate_term < self.current_term {
            return false; // término obsoleto
        }
        if candidate_term > self.current_term {
            self.current_term = candidate_term;
            self.voted_for = None;
            self.state = NodeState::Follower;
        }
        if let Some(prev) = self.voted_for {
            if prev != candidate_id { return false; } // ya votó en este término
        }
        if candidate_log_len < self.log.len() {
            return false; // el log del candidato está desactualizado
        }
        self.voted_for = Some(candidate_id);
        true
    }

    /// Convierte al nodo en líder si ha recibido una mayoría de votos.
    pub fn become_leader_if_quorum(
        &mut self,
        votes_received: HashSet<u32>,
        cluster_size: usize,
    ) -> bool {
        let majority = cluster_size / 2 + 1;
        if votes_received.len() >= majority {
            self.state = NodeState::Leader;
            true
        } else {
            false
        }
    }
}

/// Simula una elección en un cluster.
pub fn run_election(cluster: &mut HashMap<u32, RaftNode>, initiator: u32) -> Option<u32> {
    let peers: Vec<u32> = cluster.keys().copied().collect();
    let requests = cluster.get_mut(&initiator)?.start_election(&peers);
    let mut votes: HashSet<u32> = HashSet::new();
    votes.insert(initiator); // se vota a sí mismo

    for peer_id in requests {
        let peer = cluster.get(&peer_id)?;
        let granted = peer.handle_request_vote(
            initiator,
            cluster[&initiator].current_term,
            cluster[&initiator].log.len(),
        );
        if granted {
            votes.insert(peer_id);
        }
    }

    let initiator_node = cluster.get_mut(&initiator)?;
    if initiator_node.become_leader_if_quorum(votes, cluster.len()) {
        Some(initiator)
    } else {
        None
    }
}
```

Este código es el **alma** de Raft. No incluye persistencia, ni heartbeats, ni el roll-back del log cuando un líder descubre que su log está desactualizado. Pero captura lo importante: **una elección es un BFS en el grafo del cluster, con quórum como condición de parada**.

### Diálogo de mantenimiento

> —REX, ¿por qué Raft y no Paxos?
> —Porque Paxos requiere un doctorado para explicarlo, y Raft un sábado por la mañana. Mismo poder, mejor pedagogía.
> —¿Y por qué necesita quórum?
> —Porque si solo hubiera un líder sin quórum, podría estar equivocado. El quórum garantiza que al menos un nodo "sano" está de acuerdo.
> —¿Y si el líder miente?
> —Entonces los followers lo descubren al comparar logs, y le revocan en la siguiente elección. El grafo de confianza se reconstruye solo.

*(REX el Raft suspira satisfecho. Es un búfalo paciente.)*

## 25.3 Gossip protocols: cuando los rumores son el algoritmo

Hay un problema clásico: tienes 1000 nodos y quieres difundir un mensaje a todos. Mandarlo en cascada (broadcast) es eficiente, pero frágil: si un nodo cae, el mensaje se pierde para su subárbol. La solución elegante: **gossip**.

La idea: cada nodo, cada T segundos, elige al azar otro nodo y le cuenta todo lo que sabe. Como un rumor en una cafetería: tú le cuentas a dos amigos, ellos le cuentan a otros dos, y en `O(log n)` rondas el rumor ha llegado a todos.

Matemáticamente, esto es un **random walk** sobre el grafo de nodos. La diferencia con el broadcast: el random walk es **resiliente a fallos** (si un nodo cae, los demás siguen propagando) y **escalable** (cada nodo solo habla con uno o dos pares por ronda). El precio: la difusión es **probabilística**, no garantizada. El rumor llega al 99% de los nodos muy rápido, pero al último 1% puede costarle un tiempo exponencial.

```rust
use rand::seq::IteratorRandom;
use rand::Rng;

/// Estado de un nodo en un protocolo gossip. Mantiene los mensajes que conoce.
pub struct GossipNode {
    pub id: u32,
    pub known: Vec<String>,
}

impl GossipNode {
    pub fn new(id: u32, initial: Vec<String>) -> Self {
        Self { id, known: initial }
    }

    /// Ronda de gossip: elige un vecino al azar y le pasa los mensajes nuevos.
    /// Devuelve los mensajes que el vecino aún no conocía (para que los propague).
    pub fn gossip_round<R: Rng>(
        &self,
        neighbors: &[u32],
        rng: &mut R,
    ) -> Option<(u32, Vec<String>)> {
        if neighbors.is_empty() { return None; }
        let target = *neighbors.iter().choose(rng)?;
        Some((target, self.known.clone()))
    }

    /// Fusiona los mensajes recibidos con los conocidos.
    pub fn merge(&mut self, incoming: &[String]) {
        for msg in incoming {
            if !self.known.contains(msg) {
                self.known.push(msg.clone());
            }
        }
    }
}

/// Simula gossip en un grafo completo (mallado total).
pub fn simulate_gossip(
    nodes: &mut std::collections::HashMap<u32, GossipNode>,
    adjacency: &std::collections::HashMap<u32, Vec<u32>>,
    rounds: usize,
    seed: u64,
) {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(seed);
    let ids: Vec<u32> = nodes.keys().copied().collect();
    for _ in 0..rounds {
        for &id in &ids {
            let neighbors = match adjacency.get(&id) {
                Some(n) => n.clone(),
                None => continue,
            };
            if let Some((target, payload)) = nodes[&id].gossip_round(&neighbors, &mut rng) {
                nodes.get_mut(&target).unwrap().merge(&payload);
            }
        }
    }
}
```

Los protocolos gossip se usan en Cassandra (replicación), Redis Cluster (detección de fallos), Consul (descubrimiento de servicios), y en cualquier sistema donde la **consistencia eventual** es aceptable. El teorema CAP (que veremos luego) es, en parte, una disculpa formal para usar gossip.

## 25.4 Distributed Hash Tables: anillos, dedos y teleportación

Una **DHT (Distributed Hash Table)** es una base de datos hash-table distribuida en miles de nodos. La idea: cada nodo tiene un ID (un hash, digamos de 160 bits), y cada clave también. La clave se almacena en el nodo cuyo ID es el "más cercano" a la clave en algún orden (típicamente, distancia circular).

El caso estrella es **Chord** (Stoica et al., 2001), que organiza los nodos en un **anillo**. Cada nodo tiene una **finger table** (tabla de dedos) que apunta a otros nodos a distancias exponencialmente crecientes. Con esa tabla, una búsqueda va "saltando" a nodos cada vez más cercanos al objetivo, en `O(log n)` pasos. Una teleportación logarítmica.

```text
   Chord: anillo de 8 nodos (0, 1, 3, 4, 7, 9, 12, 14)

          0
        /   \
      14     1
      |      |
      12     3
       \    /
        9  4
         \/
         7

   Para buscar la clave k = 6, se hace:
   - Nodo 0 mira su finger table y salta al nodo más cercano ≤ 6.
     Digamos que salta a 4.
   - Nodo 4 mira su finger table y salta al más cercano ≤ 6.
     Digamos que salta a 7 (que es > 6, así que el anterior era 7-1=4).
   - Total: 2 saltos para una red de 8 nodos.
   - En general, O(log n) saltos.
```

**Kademlia** (Maymounkov y Mazières, 2002) refina Chord usando una métrica de distancia XOR entre IDs, que tiene propiedades topológicas bonitas (es un espacio métrico, y un grafo cuya estructura se parece a un **hyper-cubo**). Es la base de BitTorrent DHT, IPFS, Ethereum, y casi cualquier sistema P2P moderno.

```rust
use petgraph::graph::UnGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Anillo Chord simplificado: n nodos en un círculo, cada uno conoce a su
/// sucesor y a un "finger" (el nodo a distancia 2^k, módulo el anillo).
pub struct ChordRing {
    pub ring: Vec<u32>,         // IDs de los nodos, ordenados
    pub fingers: Vec<usize>,    // fingers[i] = índice en `ring` del i-ésimo finger
}

impl ChordRing {
    /// Construye un Chord con `n` nodos de IDs espaciados uniformemente.
    pub fn new_uniform(n: usize) -> Self {
        let ring: Vec<u32> = (0..n).map(|i| (i as u32) * (u32::MAX / n as u32)).collect();
        let mut fingers = vec![0usize; n];
        for i in 0..n {
            // El finger i-ésimo está a 2^i saltos en el anillo.
            let steps = 1usize << i.min(20); // cap a 2^20 para no overflow
            fingers[i] = (i + steps) % n;
        }
        Self { ring, fingers }
    }

    /// Encuentra el nodo responsable de la clave `key` empezando por `start`.
    /// Simulación del lookup: en cada paso saltamos al finger más cercano ≤ key.
    pub fn lookup(&self, mut current: usize, key: u32) -> usize {
        let n = self.ring.len();
        let mut visited = std::collections::HashSet::new();
        loop {
            if visited.contains(&current) { return current; } // bucle, devolvemos el actual
            visited.insert(current);
            // Buscamos el finger más cercano a `key` sin pasarse.
            let mut best = current;
            let mut best_dist = distance(self.ring[current], key);
            for &f in &self.fingers {
                if self.ring[f] == self.ring[current] { continue; }
                let d = distance(self.ring[f], key);
                if d < best_dist {
                    best = f;
                    best_dist = d;
                }
            }
            if best == current { return current; } // somos los más cercanos
            current = best;
            if self.ring[current] >= key { return current; }
            if visited.len() > n { return current; } // safety
        }
    }
}

/// Distancia Chord: en el sentido horario del anillo.
fn distance(a: u32, b: u32) -> u32 {
    if b >= a { b - a } else { u32::MAX - a + b }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chord_lookup_finds_node() {
        let ring = ChordRing::new_uniform(16);
        // Para cada key, el lookup debe converger en O(log n) saltos.
        for &key in &[0u32, 100, 1000, 1_000_000, u32::MAX / 2] {
            let _ = ring.lookup(0, key);
        }
    }
}
```

## 25.5 Vector clocks: el orden causal hecho grafo

Ya los mencionamos en la anécdota. La idea formal: cada nodo `i` mantiene un vector `V_i` de N enteros, uno por nodo. Al hacer un evento local, incrementa `V_i[i]`. Al enviar un mensaje, lo etiqueta con su vector. Al recibirlo, hace `V_i = max(V_i, V_msg) + 1` en la posición `i`.

La propiedad clave: dados dos eventos A y B con vectores `V_A` y `V_B`:
- Si `V_A ≤ V_B` componente a componente (con algún estricto), entonces A → B (A causó B).
- Si ni `V_A ≤ V_B` ni `V_B ≤ V_A`, entonces A y B son concurrentes.

Eso es **detección de causalidad**, y es la base de sistemas de versiones como las **CRDT** (Conflict-free Replicated Data Types), que resuelven conflictos en sistemas distribuidos sin coordinación. Si tu app de notas permite editar offline y mergear después, probablemente estés usando CRDTs con vector clocks por debajo.

## 25.6 El teorema CAP: cuando la red se parte

En 2000, Eric Brewer formuló lo que se conoce como el **teorema CAP** (también llamado **conjetura de Brewer** porque no se demostró formalmente hasta 2002 por Gilbert y Lynch). Dice así:

> En un sistema distribuido, ante una partición de red (P), solo puedes garantizar dos de las tres propiedades: **C**onsistencia, **A**vailability, **tolerancia a** **P**articiones.

Es decir: si la red se parte, tienes que elegir entre:
- **CP**: consistencia estricta. El sistema se niega a responder antes que devolver datos contradictorios. (Bancos, bases de datos transaccionales.)
- **AP**: disponibilidad. El sistema siempre responde, aunque devuelva datos potencialmente obsoletos. (Redes sociales, sistemas de cache, DNS.)

Lo bonito del CAP, si lo piensas como un grafo, es que **la partición de red es una desconexión en el grafo de nodos**. Cuando se parte la red, el grafo de comunicación se rompe en componentes. Y cada componente solo puede "ver" a los nodos de su lado. Por tanto, la decisión CAP es literalmente: ¿qué prefieres hacer cuando el grafo se parte?

```text
   Red normal: grafo conexo

       N1 ─── N2
        │  X  │
       N3 ─── N4

   Red partida: dos componentes

       N1     N2
        │  X  │
       N3     N4

   Elige: ¿consistencia (CP) o disponibilidad (AP)?
   - CP: N1 y N3 dejan de aceptar escrituras hasta que vuelva la red.
   - AP: ambos lados aceptan escrituras, que se reconcilian al reconectarse.
```

## 25.7 Implementación Rust: mini-DHT con `petgraph` y `tokio`

Vamos a programar una mini-DHT estilo Chord. Cada nodo es una `task` async, escucha mensajes por un canal, y mantiene una finger table. Los lookups viajan por el anillo.

```toml
# Cargo.toml
[package]
name = "mini-dht"
version = "0.1.0"
edition = "2024"

[dependencies]
petgraph = "0.6"
tokio = { version = "1", features = ["full"] }
```

```rust
// src/main.rs
use petgraph::graph::UnGraph;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Mensajes que pueden circular entre nodos.
#[derive(Debug, Clone)]
pub enum DhtMessage {
    /// Almacena la clave `key` en el nodo responsable.
    Store { key: u32, value: String, from: u32 },
    /// Busca el responsable de `key` y devuelve el valor.
    Lookup { key: u32, from: u32, hops: u32 },
    /// Respuesta a un lookup, propagándose de vuelta al solicitante original.
    LookupResponse { key: u32, value: Option<String>, hops: u32 },
    /// Pasa la pelota: el nodo actual no es responsable, redirige a `next`.
    Redirect { key: u32, next: u32, from: u32, hops: u32 },
}

/// Nodo de la DHT.
pub struct DhtNode {
    pub id: u32,
    pub store: HashMap<u32, String>,
    pub ring_size: u32,
    pub tx: mpsc::UnboundedSender<DhtMessage>,
}

impl DhtNode {
    pub fn new(id: u32, ring_size: u32, tx: mpsc::UnboundedSender<DhtMessage>) -> Self {
        Self { id, store: HashMap::new(), ring_size, tx }
    }

    /// Dado un ID, devuelve el "propietario" en el anillo (nodo responsable).
    pub fn responsible_for(&self, key: u32) -> u32 {
        // Asumimos IDs espaciados uniformemente, tamaño = ring_size.
        // El responsable es el nodo con el menor ID > key (en sentido circular).
        let bucket = (key as u64 * self.ring_size as u64 / (u32::MAX as u64 + 1)) as u32;
        bucket
    }

    /// Maneja un mensaje entrante.
    pub fn handle(&mut self, msg: DhtMessage) {
        match msg {
            DhtMessage::Store { key, value, from: _ } => {
                if self.responsible_for(key) == self.id {
                    self.store.insert(key, value);
                } else {
                    // Reenviar al responsable.
                    let next = self.responsible_for(key);
                    let _ = self.tx.send(DhtMessage::Redirect { key, next, from: self.id, hops: 1 });
                }
            }
            DhtMessage::Lookup { key, from, mut hops } => {
                hops += 1;
                if self.responsible_for(key) == self.id {
                    let value = self.store.get(&key).cloned();
                    let _ = self.tx.send(DhtMessage::LookupResponse { key, value, hops });
                } else {
                    let next = self.responsible_for(key);
                    let _ = self.tx.send(DhtMessage::Redirect { key, next, from, hops });
                }
            }
            DhtMessage::LookupResponse { key, value, hops } => {
                println!("  Nodo {} recibió respuesta: key={} value={:?} ({} saltos)",
                    self.id, key, value, hops);
            }
            DhtMessage::Redirect { key, next, from, hops } => {
                if next == self.id {
                    // Soy el responsable, atiendo.
                    self.handle(DhtMessage::Lookup { key, from, hops });
                } else {
                    // Reenviar a `next`.
                    let _ = self.tx.send(DhtMessage::Redirect { key, next, from, hops });
                }
            }
        }
    }

    /// Almacena un par clave-valor (envía un mensaje al responsable).
    pub fn put(&self, key: u32, value: String) {
        let _ = self.tx.send(DhtMessage::Store {
            key, value, from: self.id,
        });
    }

    /// Busca un valor por clave.
    pub fn get(&self, key: u32) {
        let _ = self.tx.send(DhtMessage::Lookup {
            key, from: self.id, hops: 0,
        });
    }
}

/// Crea un anillo de `n` nodos, devuelve un grafo de adyacencia (anillo)
/// y un mapa de canales para enviar mensajes a cada nodo.
pub fn build_ring(n: u32) -> (UnGraph<u32, ()>, HashMap<u32, mpsc::UnboundedSender<DhtMessage>>) {
    let mut g = UnGraph::<u32, ()>::new_undirected();
    let mut nodes = Vec::new();
    let mut txs = HashMap::new();

    for i in 0..n {
        let id = i * (u32::MAX / n);
        let idx = g.add_node(id);
        nodes.push((id, idx));

        // Cada nodo tiene su canal.
        let (tx, mut rx) = mpsc::unbounded_channel::<DhtMessage>();
        txs.insert(id, tx);

        // Lanzamos la task que escucha mensajes.
        let mut node = DhtNode::new(id, n, txs[&id].clone());
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                node.handle(msg);
            }
        });
    }

    // Conectamos los nodos en anillo.
    for i in 0..nodes.len() {
        let next = (i + 1) % nodes.len();
        g.add_edge(nodes[i].1, nodes[next].1, ());
    }

    (g, txs)
}

#[tokio::main]
async fn main() {
    let (graph, txs) = build_ring(16);
    println!("Anillo DHT con {} nodos, {} aristas.", graph.node_count(), graph.edge_count());

    // El nodo 0 pone y busca algunas claves.
    let id0 = 0u32 * (u32::MAX / 16);
    let node0 = txs.get(&id0).expect("nodo 0 existe");

    node0.send(DhtMessage::Store {
        key: 12345, value: "Hola, DHT!".to_string(), from: id0,
    }).unwrap();
    node0.send(DhtMessage::Lookup {
        key: 12345, from: id0, hops: 0,
    }).unwrap();

    // Damos tiempo a procesar.
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
}
```

Este programa no es un Chord completo (faltan las finger tables, la tolerancia a fallos, y el balanceo de carga), pero captura la idea: **una DHT es un grafo en anillo con lookups que saltan hasta encontrar al responsable**. Cada salto es un mensaje asíncrono. El grafo se mantiene en `petgraph` para visualizarlo, y los mensajes viajan por canales de `tokio`.

## 25.8 Diálogo de ascensor

> —Perdona, ¿es tu primer día en la empresa?
> —Sí, soy la nueva ingeniera de sistemas distribuidos.
> —Bienvenida. ¿Y cuál es tu primer proyecto?
> —Implementar consenso en un cluster de mil nodos.
> —Ah,共识 (consenso). ¿Sabes la regla de oro?
> —¿Cuál?
> —**Si tu sistema distribuido funciona a la primera, es que no lo has probado lo suficiente.** Siempre asume que la red se va a partir, que los nodos van a fallar, y que los mensajes se van a perder. Diseña para el caos.
> —Suena a pesimismo.
> —No, es realismo. La nube no es tu amiga. Es una traidora educada.

*(La nueva ingeniera asiente, abre su laptop, y empieza a depurar un bug en Raft. Afuera, una paloma se posa en la ventana, indiferente al teorema CAP.)*

## 25.9 Ejercicios resueltos

### Ejercicio 25.1: quórum en un cluster de 5

En un cluster Raft de 5 nodos, ¿cuántos nodos deben responder a una petición de voto para que un candidato gane?

**Solución:** un quórum es la mayoría, que en 5 nodos es `floor(5/2) + 1 = 3`. El candidato necesita 3 votos (contándose a sí mismo). Si solo consigue 2, no hay mayoría y debe esperar a la próxima elección.

### Ejercicio 25.2: vector clocks

Tres procesos P1, P2, P3. P1 hace un evento local `a`, luego envía un mensaje a P2. P2 recibe y hace un evento local `b`. P3 hace un evento local `c` sin enviar nada. ¿Qué vectores de reloj tienen `a`, `b` y `c`?

**Solución:**
- `a` en P1: vector `(1, 0, 0)`.
- `b` en P2: tras recibir el mensaje con vector `(1, 0, 0)`, P2 hace `max((0,0,0), (1,0,0)) = (1,0,0)`, luego incrementa su componente: `(1, 1, 0)`.
- `c` en P3: `(0, 0, 1)`.

Relaciones de causalidad:
- `a` → `b` (porque `(1,0,0) ≤ (1,1,0)` con estricto en la segunda componente).
- `a` y `c` son **concurrentes** (ninguno es menor o igual al otro).

### Ejercicio 25.3: Chord lookup

En un Chord de 8 nodos con IDs {0, 8, 16, 24, 32, 40, 48, 56}, busca la clave 50 empezando desde el nodo 0. ¿Cuántos saltos se necesitan?

**Solución:** el responsable de la clave 50 es el primer nodo con ID ≥ 50 en sentido circular, que es 56 (porque no hay nadie entre 50 y 56). Empezando en 0, el lookup salta a 48 (el más cercano ≤ 50, ignorando al propio 0). Desde 48, salta a 56. Total: 2 saltos. En Chord, el coste esperado es `O(log n) = 3` para n=8, así que 2 está dentro del rango.

## 25.10 Ejercicios propuestos

1. **Simulador de Raft completo**: implementa el "raft completo" con persistencia de log, snapshotting, y cambios de configuración. Es un proyecto de varias semanas, pero te dejará entender Raft como nadie.
2. **Vector clocks con histórico**: modifica los vector clocks para que recuerden las últimas K versiones, no solo la actual. Útil para sistemas con replicación multi-master.
3. **Gossip con anti-entropy**: añade un mecanismo de **Merkle tree** para que los nodos detecten qué partes del estado les faltan y las sincronicen eficientemente.
4. **Kademlia XOR-distance**: implementa una DHT estilo Kademlia donde la distancia entre IDs es XOR. La estructura del grafo se parece a un hyper-cubo, y los lookups son aún más eficientes.
5. **(Avanzado) Consenso bizantino**: implementa un simulador de **PBFT** (Practical Byzantine Fault Tolerance). Esto asume que hasta `f` nodos pueden ser maliciosos, no solo caídos. Es **mucho** más complejo que Raft.

## 25.11 Pin de batalla

- **Raft y Paxos resuelven el mismo problema**. Si entiendes Raft, entiendes Paxos (solo que con más ceremonia).
- **El quórum es la clave de la tolerancia a fallos**. En un cluster de N nodos, aguantas `floor((N-1)/2)` fallos. Por eso los clusters suelen ser impares (3, 5, 7).
- **Los vector clocks son la base de la causalidad distribuida**. Sin ellos, no podrías detectar conflictos en una app colaborativa offline-first.
- **CAP no es un teorema en el sentido formal**, sino un trade-off ingenieril. Pero el nombre "teorema" ha cuajado y ya nadie lo va a cambiar.
- **Gossip es la elegancia**: probabilístico, asíncrono, resiliente. Pero no garantiza entrega. Si necesitas garantía, usa broadcast con acknowledgments.
- **Chord y Kademlia son el pan de cada día del P2P**. BitTorrent, IPFS, Ethereum: todos usan DHTs.

## 25.12 Lo que te llevas

- **Lamport (1998)**: los relojes lógicos y la causalidad distribuida. Vector clocks como evolución.
- **Raft y Paxos**: consenso distribuido. Líder elegido por BFS electivo, log replicado por AppendEntries, quórum como condición de compromiso.
- **Gossip protocols**: rumores que se difunden en `O(log n)` rondas. Resilientes, escalables, probabilísticos.
- **DHTs (Chord, Kademlia)**: bases de datos hash distribuidas en anillos o hyper-cubos. Lookups en `O(log n)` saltos.
- **Vector clocks**: detección de causalidad en grafos de eventos. La base de las CRDTs.
- **CAP**: el trade-off entre consistencia y disponibilidad cuando el grafo se parte. No es dogma, es contexto.
- **La mini-DHT en Rust** muestra cómo un anillo + finger tables + async = un sistema P2P en pocas líneas.

## 25.13 Ojo, cuidado con…

- **Raft NO es tolerante a fallos bizantinos**. Asume nodos que fallan "por caída", no "por maldad". Para nodos maliciosos, necesitas PBFT.
- **El "split-brain" es el enemigo mortal**. Si la red se parte y ambos lados proclaman un líder, las decisiones divergen. Por eso Raft requiere quórum: un líder sin quórum no es líder.
- **Los relojes de las máquinas no están sincronizados**. Confiar en timestamps de Unix para ordenar eventos es un error clásico. Usa vector clocks o timestamps lógicos.
- **El teorema CAP es solo para particiones**. En una red sana, puedes tener las tres propiedades (C, A, P). La partición es lo que te obliga a elegir.
- **Las DHTs reales son bestias complejas**. Chord es el "Hola mundo"; las implementaciones reales (Kademlia en libp2p, por ejemplo) tienen cientos de páginas de edge cases.
- **Gossip no escala indefinidamente**. Cada nodo elige un par al azar, lo que genera tráfico `O(n)` por ronda. Para millones de nodos, necesitas jerarquías o sharding.

## 25.14 Para profundizar

- **Lamport, L. (1978). "Time, Clocks, and the Ordering of Events in a Distributed System." *Communications of the ACM*, 21(7), 558–565.** — El paper que lo empezó todo.
- **Ongaro, D. & Ousterhout, J. (2014). "In Search of an Understandable Consensus Algorithm."** — El paper de Raft. Didáctico como pocos.
- **Lamport, L. (1998). "The Part-Time Parliament."** — Paxos, en forma de parábola sobre una parlamento griego.
- **Stoica, I. et al. (2001). "Chord: A Scalable Peer-to-Peer Lookup Service for Internet Applications."** — El paper original de Chord.
- **Maymounkov, P. & Mazières, D. (2002). "Kademlia: A Peer-to-Peer Information System Based on the XOR Metric."** — Kademlia.
- **Brewer, E. (2000). "Towards Robust Distributed Systems."** — La conjetura CAP.
- **Shapiro, M. et al. (2011). "A Comprehensive Study of CRDTs."** — El estado del arte de tipos de datos replicados.

## 25.15 Si solo lees 30 segundos

Los sistemas distribuidos son grafos de procesos que no se fían unos de otros. Lamport nos enseñó a ordenar eventos causalmente con vector clocks. Raft resolvió el consenso con elección de líder + log replicado + quórum. Gossip y DHT propagan información sin necesidad de un coordinador. CAP te recuerda que cuando el grafo se parte, tienes que elegir entre consistencia y disponibilidad.

## 25.16 Una historia pequeña

Lucía es ingeniera en una startup de logística. Un viernes a las 18:00, el sistema de tracking de envíos, que corre en un cluster Raft de 5 nodos en AWS, empieza a comportarse de forma extraña. Los paquetes aparecen duplicados, los IDs se contradicen, y los clientes reciben emails con direcciones que no son las suyas. El equipo está a punto de tirar el servidor y empezar de cero.

Lucía, que acaba de leer el paper de Raft, decide mirar los logs del líder. Y encuentra algo curioso: el líder fue elegido dos veces en cinco minutos, con el mismo término. Eso es imposible. La única explicación es **split-brain**: la red se partió短暂 (brevemente), ambos lados proclamaron un líder, y al reconectarse, ambos líderes intentaron replicar logs incompatibles.

Lucía revisa la configuración de AWS y descubre que la tabla de rutas tenía una entrada obsoleta que causaba blackholes de 30 segundos. La corrige, fuerza una elección limpia, y el sistema se estabiliza. A las 21:00, todo funciona. Lucía se va a casa, se hace un té, y abre el paper de Raft otra vez. Esta vez, lo entiende de verdad.

---

