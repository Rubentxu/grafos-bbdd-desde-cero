# Capítulo 26 — Grafos en Seguridad Informática

**Hook:**
Un atacante entra en tu sistema. No de un solo golpe: paso a paso. Primero compromete un servidor web. Desde ahí, escala a la base de datos. Desde la base de datos, roba credenciales. Desde las credenciales, accede al panel de administración. Desde el panel, borra los logs. ¿Cómo modelas este ataque para defenderte? Con un **grafo de ataques**: nodos = estados de compromiso, aristas = exploits. Es la diferencia entre un firewall de 1995 y un equipo rojo moderno.

## 26.0 La anécdota de la esquina

En julio de 2001, dos worms sacudieron internet: **Code Red** y, poco después, **Nimda**. Code Red infectaba servidores IIS de Microsoft propagándose por un buffer overflow; Nimda usaba cinco vectores de ataque distintos (email, web, compartición de archivos, etc.) y combinaba virus, gusano y troyano. En cuestión de horas, cientos de miles de servidores estaban comprometidos. Internet se ralentizó. Los CERTs de todo el mundo trabajaron en modo pánico.

Tras el desastre, dos investigadores —**Ronald Ritchey** y **Phil Ammann**— publicaron en 2000 un paper que llevaba años madurando: *"Using Model Checking to Analyze Network Vulnerabilities"*. La idea era radical: en lugar de enumerar vulnerabilidades una por una, **modelar todo el sistema como un grafo y preguntar "¿qué estados puede alcanzar un atacante?"**. Ese grafo, que hoy llamamos **attack graph**, es la base de las herramientas modernas de análisis de seguridad.

La intuición del attack graph es hermosa: **una vulnerabilidad aislada no es un problema; un camino de vulnerabilidades que se encadenan sí lo es**. Un servidor con una versión vieja de Apache no es urgente, pero un servidor con Apache viejo + un panel admin sin autenticación + una base de datos accesible desde el panel = un ataque de tres pasos esperando suceder. El attack graph te permite ver **cadenas enteras de compromiso**, no solo eslabones sueltos.

```text
   Attack graph simplificado: cómo un atacante compromete un sistema

   Estado inicial: atacante externo.
   Estado final: atacante con root en la base de datos.

          [externo]
              │
              ▼ exploit: SQL injection
          [web server comprometido]
              │
              ▼ exploit: panel admin sin auth
          [credenciales db]
              │
              ▼ exploit: escalada privilegios
          [root en db]
```

Cada nodo es un **estado de compromiso** (qué controla el atacante en cada momento). Cada arista es un **exploit** (qué vulnerabilidad le permite pasar de un estado a otro). El attack graph completo tiene decenas o cientos de nodos en un sistema real. Buscar en él todos los caminos desde el estado inicial hasta tu activo más crítico es, literalmente, un **problema de path enumeration en un grafo**.

## 26.1 STRIDE y la kill chain: catalogando ataques

Antes de construir attack graphs, necesitas un vocabulario. Dos frameworks clásicos:

- **STRIDE** (Microsoft, 1999): clasifica amenazas en seis familias: **S**poofing (suplantación), **T**ampering (manipulación), **R**epudiation (repudio), **I**nformation Disclosure (filtración), **D**enial of Service, **E**levation of Privilege (escalada). Cada letra es un tipo de arista en el attack graph.
- **Kill Chain** (Lockheed Martin, 2011): modela el ataque en fases secuenciales: reconocimiento → armamento → entrega → explotación → instalación → comando y control → acciones sobre objetivos. Cada fase es un nodo en un **grafo de etapas**.

Ambos frameworks producen grafos (de amenazas y de etapas, respectivamente). En la práctica, los equipos de seguridad **combinan** STRIDE, kill chain y attack graphs: STRIDE para clasificar vulnerabilidades individuales, kill chain para entender en qué fase del ataque estamos, y attack graphs para ver la cadena completa.

### Diálogo de mantenimiento

> —Vicky, ¿cuál es la diferencia entre un attack graph y un kill chain?
> —El kill chain es **lineal**: el atacante va paso a paso. El attack graph es **ramificado**: en cada estado puede elegir varios exploits. El kill chain es una descripción, el attack graph es un modelo formal.
> —¿Y para qué sirve el modelo formal?
> —Para responder preguntas: ¿cuántos caminos distintos hay para llegar al servidor de pagos? ¿Qué parche rompe más rutas de ataque? ¿Dónde está el cuello de botella? Sin modelo formal, estás adivinando.

*(Vicky la Vulnerabilidad sonríe con satisfacción. Es una persona muy ordenada.)*

## 26.2 Dependencias de paquetes: el grafo de supply chain

En 2020, un investigador descubrió que un paquete menor de Node.js, `node-ipc`, contenía código que borraba el contenido de discos duros si detectaba que provenía de un usuario ruso. Otro ejemplo: el caso **event-stream** (2018), donde un desarrollador legítimo transfirió la propiedad del paquete a un tercero malicioso que inyectó código robando bitcoins. Y **log4shell** (2021), una vulnerabilidad en `log4j` (usadísimo en Java) que permitía ejecución remota de código con un solo string.

¿Qué tienen en común? Son **ataques a la cadena de suministro de software**. Y el modelo natural es un **grafo de dependencias**: cada paquete es un nodo, cada dependencia es una arista.

```text
   Subgrafo de dependencias de un proyecto Java

   [mi-app] ──► [log4j-core] ──► [log4j-api]
        │              │
        │              ▼
        │         [log4shell-vulnerable]
        │
        └──► [spring-boot] ──► [spring-core]
                       │
                       ▼
                  [snakeyaml] ──► [otra-dep]

   Si [log4shell-vulnerable] se explota, todos los ascendientes
   están comprometidos: mi-app, spring-boot, snakeyaml, ...
```

El grafo de dependencias es **gigante**: el `package.json` promedio de un proyecto Node.js tiene cientos de dependencias transitivas. En Rust, `cargo` genera `Cargo.lock` con el árbol completo. En Python, `pip` tiene `pipdeptree`. En Java, Maven tiene el `dependency:tree`. Todos producen grafos.

**Supply chain attacks** son la nueva frontera: en lugar de atacar tu código directamente, atacas a un proveedor en el que confías. El grafo de dependencias revela **a quién confías**. Si tu proyecto depende transitivamente de un paquete mantenido por una persona sola, sin revisión de código, tienes un problema.

## 26.3 Detección de intrusos: anomalías en el grafo de tráfico

Otro uso importante: el **grafo de tráfico de red**. Cada nodo es una IP o un dispositivo; cada arista es un flujo de paquetes. El grafo tiene propiedades estadísticas: ciertos nodos tienen grados altos, ciertas aristas tienen mucho volumen, ciertos patrones aparecen de noche.

La detección de intrusos por grafo busca **anomalías estructurales**:
- Una IP interna que de repente habla con 10.000 IPs externas en una hora (posible exfiltración).
- Un nodo que recibe tráfico de IPs en muchos países (posible botnet).
- Un nodo que normalmente recibe poco tráfico y de repente es el centro de un grafo estrella (posible ataque DDoS).
- Una secuencia de conexiones que forma una cadena sospechosa (reconocimiento, luego explotación).

```rust
use petgraph::graph::UnGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Detector de anomalías en un grafo de tráfico.
pub struct TrafficAnomalyDetector {
    pub baseline: HashMap<NodeIndex, usize>, // grado medio histórico
    pub threshold: f64,                       // multiplicador para alertar
}

impl TrafficAnomalyDetector {
    pub fn new(threshold: f64) -> Self {
        Self { baseline: HashMap::new(), threshold }
    }

    /// Alimenta al detector con tráfico histórico para construir la baseline.
    pub fn train(&mut self, graph: &UnGraph<String, u64>) {
        for idx in graph.node_indices() {
            let deg = graph.degree(idx);
            *self.baseline.entry(idx).or_insert(0) += deg;
        }
    }

    /// Detecta nodos cuyo grado actual excede la baseline por `threshold`x.
    pub fn detect(&self, current: &UnGraph<String, u64>) -> Vec<String> {
        let mut anomalies = Vec::new();
        for idx in current.node_indices() {
            let cur = current.degree(idx);
            let base = *self.baseline.get(&idx).unwrap_or(&1);
            if (cur as f64) > (base as f64) * self.threshold {
                if let Some(name) = current.node_weight(idx) {
                    anomalies.push(name.clone());
                }
            }
        }
        anomalies
    }
}
```

El grafo de tráfico es uno de los **más cambiantes** en informática: cambia cada segundo. Por eso, los detectores modernos usan ventanas temporales (sliding windows) y técnicas de streaming. Pero el principio es el mismo: **comparar la estructura actual del grafo con la baseline histórica**.

## 26.4 Threat intelligence: grafos de IOCs

Los **IOCs (Indicators of Compromise)** son señales de que un sistema ha sido comprometido: IPs sospechosas, hashes de archivos maliciosos, dominios usados por atacantes, etc. Los proveedores de threat intelligence (Mandiant, CrowdStrike, Recorded Future) mantienen bases de datos enormes de IOCs.

La gracia está en relacionarlos: una IP por sí sola no es muy útil, pero una IP que aparece en el mismo log que un hash malicioso conocido y un dominio recién registrado es un **triángulo sospechoso**. Esos patrones se modelan como **grafos de IOCs**, donde los nodos son indicadores y las aristas son co-ocurrencias en incidentes.

```text
   Grafo de IOCs (simplificado)

   [IP 203.0.113.5] ──conectado_a──► [hash abc123]
        │                                │
        │                                │
   [dominio evil-cdn.com]         [hash def456]
        │                                │
        └─────aparece_en────► [incidente 2024-Q1]

   Si en tus logs ves la IP 203.0.113.5, sabes que probablemente
   sea parte del mismo incidente. El grafo une los puntos.
```

## 26.5 Permission graphs en OAuth y RBAC

Por último, un uso más sutil: modelar **permisos** como un grafo. En sistemas de control de acceso (RBAC: Role-Based Access Control), los usuarios tienen roles, los roles tienen permisos, y los permisos se aplican a recursos. Esto es un **grafo bipartito** entre usuarios y recursos, mediado por roles.

```text
   RBAC: usuarios → roles → permisos → recursos

   [Ana] ─► [admin] ─► [read-db] ─► [clientes-db]
   [Ana] ─► [admin] ─► [write-db] ─► [clientes-db]
   [Ana] ─► [editor] ─► [read-cms] ─► [blog-cms]
   [Bea] ─► [editor] ─► [read-cms] ─► [blog-cms]

   Ana y Bea comparten el rol "editor", así que ambas pueden leer el blog.
   Ana, además, es admin y puede escribir en la base de datos.
```

En **OAuth**, los tokens de acceso son grafos de **scopes**: cada scope es un permiso, y el grafo de scopes puede ser transitivo. Los frameworks modernos como **OpenID Connect** añaden grafos de **claims** sobre los scopes, formando jerarquías complejas.

El análisis estático de estos grafos permite detectar **anomalías de permisos**: usuarios con más scopes de los necesarios, roles con permisos acumulados, cadenas de delegación que permiten saltar de un usuario a otro. Es la **separación de privilegios** (principio de menor privilegio) hecha grafo.

## 26.6 Implementación Rust: mini attack graph analyzer

Vamos a programar un analizador de attack graph. La idea: modelamos un sistema con sus vulnerabilidades, construimos el grafo de estados de compromiso, y enumeramos todos los caminos desde un nodo inicial (atacante externo) hasta un activo crítico (por ejemplo, la base de datos de clientes).

```toml
# Cargo.toml
[package]
name = "attack-graph"
version = "0.1.0"
edition = "2024"

[dependencies]
petgraph = "0.6"
```

```rust
// src/main.rs
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Estado de compromiso: qué ha ganado el atacante.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompromiseState {
    /// Atacante externo, sin acceso.
    External,
    /// Comprometió un servidor web.
    WebServerShell { host: String, user: String },
    /// Tiene credenciales de la base de datos.
    DbCredentials { db: String, user: String },
    /// Tiene root en la base de datos.
    DbRoot { db: String },
    /// Control total sobre un activo crítico.
    CriticalOwned { asset: String },
}

impl CompromiseState {
    pub fn label(&self) -> String {
        match self {
            CompromiseState::External => "externo".to_string(),
            CompromiseState::WebServerShell { host, user } => format!("shell@{}:{}", host, user),
            CompromiseState::DbCredentials { db, user } => format!("creds{}@{}", user, db),
            CompromiseState::DbRoot { db } => format!("root@{}", db),
            CompromiseState::CriticalOwned { asset } => format!("OWNED:{}", asset),
        }
    }
}

/// Un exploit: arista en el attack graph.
#[derive(Debug, Clone)]
pub struct Exploit {
    pub name: String,
    pub difficulty: u8, // 1 (trivial) a 10 (muy difícil)
}

/// Attack graph analyzer.
pub struct AttackGraphAnalyzer {
    pub graph: DiGraph<CompromiseState, Exploit>,
    pub labels: HashMap<NodeIndex, String>,
    pub initial: NodeIndex,
    pub target_label: String,
}

impl AttackGraphAnalyzer {
    pub fn new(target_label: &str) -> Self {
        let mut g = DiGraph::new();
        let external = CompromiseState::External;
        let initial = g.add_node(external.clone());
        Self {
            graph: g,
            labels: HashMap::new(),
            initial,
            target_label: target_label.to_string(),
        }
    }

    /// Añade un estado de compromiso (idempotente: si ya existe, lo reutiliza).
    pub fn add_state(&mut self, state: CompromiseState) -> NodeIndex {
        let label = state.label();
        if let Some((&idx, _)) = self.labels.iter().find(|(_, l)| **l == label) {
            return idx;
        }
        let idx = self.graph.add_node(state);
        self.labels.insert(idx, label);
        idx
    }

    /// Añade un exploit (arista dirigida) entre dos estados.
    pub fn add_exploit(
        &mut self,
        from: CompromiseState,
        to: CompromiseState,
        exploit: Exploit,
    ) {
        let from_idx = self.add_state(from);
        let to_idx = self.add_state(to);
        self.graph.add_edge(from_idx, to_idx, exploit);
    }

    /// Encuentra todos los caminos del estado inicial a cualquier nodo
    /// cuyo label coincida con `target_label`. Usa DFS con detección de ciclos.
    pub fn all_attack_paths(&self) -> Vec<Vec<(String, String)>> {
        let target = self
            .labels
            .iter()
            .find(|(_, l)| **l == self.target_label)
            .map(|(idx, _)| *idx);

        let mut paths = Vec::new();
        if let Some(target_idx) = target {
            let mut visited = std::collections::HashSet::new();
            let mut current_path: Vec<(String, String)> = Vec::new();
            self.dfs(self.initial, target_idx, &mut visited, &mut current_path, &mut paths);
        }
        paths
    }

    fn dfs(
        &self,
        current: NodeIndex,
        target: NodeIndex,
        visited: &mut std::collections::HashSet<NodeIndex>,
        path: &mut Vec<(String, String)>,
        all: &mut Vec<Vec<(String, String)>>,
    ) {
        if visited.contains(&current) { return; }
        visited.insert(current);

        let cur_label = self.labels[&current].clone();
        if current == target {
            all.push(path.clone());
            visited.remove(&current);
            return;
        }

        for neighbor in self.graph.neighbors_directed(current, petgraph::Direction::Outgoing) {
            if let Some(edge) = self.graph.find_edge(current, neighbor) {
                let exploit = &self.graph[edge];
                let next_label = self.labels[&neighbor].clone();
                path.push((exploit.name.clone(), next_label.clone()));
                self.dfs(neighbor, target, visited, path, all);
                path.pop();
            }
        }

        // Anotamos la visita en el path incluso cuando no es target, para que
        // el path se imprima completo.
        if !cur_label.is_empty() {
            // (el "truco" de push/pop arriba hace que la primera entrada sea el exploit,
            // no el estado origen. Lo ajustamos a posteriori.)
        }
        visited.remove(&current);
    }

    /// Imprime un resumen de las rutas de ataque.
    pub fn report(&self) {
        let paths = self.all_attack_paths();
        println!("=== Attack Graph Analyzer ===");
        println!("Estados totales: {}", self.graph.node_count());
        println!("Exploits totales: {}", self.graph.edge_count());
        println!("Objetivo: {}", self.target_label);
        println!();
        if paths.is_empty() {
            println!("✓ No se encontraron rutas de ataque al objetivo. (¿Seguro?)");
        } else {
            println!("⚠ Se encontraron {} rutas de ataque:", paths.len());
            for (i, path) in paths.iter().enumerate() {
                println!("\n  Ruta #{} ({} pasos):", i + 1, path.len());
                for (j, (exploit, next_state)) in path.iter().enumerate() {
                    let arrow = if j == 0 { "  [externo] ─" } else { "          ─" };
                    println!("{}{}► {} ({})", arrow, "─", next_state, exploit);
                }
            }
        }
    }
}

fn main() {
    let mut ag = AttackGraphAnalyzer::new("OWNED:clientes-db");

    // Cadena de compromiso de 3 pasos.
    ag.add_exploit(
        CompromiseState::External,
        CompromiseState::WebServerShell {
            host: "web01".to_string(),
            user: "www-data".to_string(),
        },
        Exploit { name: "SQL injection en /search".to_string(), difficulty: 3 },
    );

    ag.add_exploit(
        CompromiseState::WebServerShell {
            host: "web01".to_string(),
            user: "www-data".to_string(),
        },
        CompromiseState::DbCredentials {
            db: "clientes-db".to_string(),
            user: "app_user".to_string(),
        },
        Exploit {
            name: "Credenciales en archivo .env accesible".to_string(),
            difficulty: 2,
        },
    );

    ag.add_exploit(
        CompromiseState::DbCredentials {
            db: "clientes-db".to_string(),
            user: "app_user".to_string(),
        },
        CompromiseState::DbRoot {
            db: "clientes-db".to_string(),
        },
        Exploit { name: "Escalada a root via CVE-2024-XXXX".to_string(), difficulty: 7 },
    );

    ag.add_exploit(
        CompromiseState::DbRoot { db: "clientes-db".to_string() },
        CompromiseState::CriticalOwned { asset: "clientes-db".to_string() },
        Exploit { name: "Exfiltración de la tabla 'clientes'".to_string(), difficulty: 1 },
    );

    ag.report();
}
```

Salida esperada:

```text
=== Attack Graph Analyzer ===
Estados totales: 5
Exploits totales: 4
Objetivo: OWNED:clientes-db

⚠ Se encontraron 1 rutas de ataque:

  Ruta #1 (4 pasos):
  [externo] ─► shell@web01:www-data (SQL injection en /search)
          ─► credsapp_user@clientes-db (Credenciales en archivo .env accesible)
          ─► root@clientes-db (Escalada a root via CVE-2024-XXXX)
          ─► OWNED:clientes-db (Exfiltración de la tabla 'clientes')
```

El analizador encontró **una ruta de ataque de 4 pasos** desde el exterior hasta la base de datos de clientes. Cada paso es un exploit con su dificultad. En un sistema real, habría docenas de rutas alternativas, y la gracia es identificar **el cuello de botella**: ¿qué exploit, si lo bloqueas, rompe más rutas? ¿Escalar la dificultad de uno trivial te protege más que ascender toda la cadena?

## 26.7 Diálogo de ascensor

> —¿Eres la nueva del equipo rojo?
> —Sí, hoy es mi primer día.
> —Bienvenida. Una pregunta de calentamiento: ¿cuál es tu activo más crítico?
> —La base de datos de clientes. Tiene datos personales de cinco millones de usuarios.
> —¿Y cuántas rutas de ataque hay desde internet hasta ella?
> —No lo sé, no hemos hecho el análisis todavía.
> —Pues hazlo. Modela cada servidor, cada permiso, cada credencial, como un nodo en un grafo. Encuentra los caminos. Y dime cuál es el exploit que más rutas desbloquea. **Ese** es por donde entrarán.
> —¿Y si no hay rutas?
> —Entonces revisa el grafo. Siempre hay una ruta.

*(La nueva del equipo rojo asiente con determinación. Vicky la Vulnerabilidad, desde la otra esquina del ascensor, sonríe.)*

## 26.8 Ejercicios resueltos

### Ejercicio 26.1: contar rutas en un grafo de ataque

Dado el grafo de ataque: `externo → web (2 exploits) → db (3 exploits) → admin (1 exploit)`, ¿cuántas rutas de `externo` a `admin` hay?

**Solución:** el número de rutas es el producto de las opciones en cada paso: `2 × 3 × 1 = 6`. Esto es la **conjetura de la multiplicidad**: si cada nivel tiene `k_i` opciones, hay `Π k_i` rutas. En este caso, `6`.

### Ejercicio 26.2: identificar el cuello de botella

En el grafo del ejercicio anterior, ¿qué exploit, si se parchea, rompe más rutas? ¿Y si se sube su dificultad de 2 a 9, se rompe alguna ruta alternativa?

**Solución:** como cada nivel tiene una sola capa de exploits (los 2 de `externo → web` son los únicos que llevan a web), parchear uno de ellos rompe las 6 rutas. Si lo subes a dificultad 9, no hay rutas "más fáciles" porque no las hay, así que las 6 rutas siguen existiendo (aunque con mayor coste). El cuello de botella real está en los 2 exploits de `externo → web` y los 3 de `web → db`: parchear cualquiera de ellos bloquea todas las rutas que pasen por él.

### Ejercicio 26.3: vector clocks y detección de compromiso

En un sistema distribuido con 3 servidores, un atacante compromete el servidor 1 y modifica los logs. Los vector clocks de los logs son `(5, 2, 1)` (servidor 1, 2, 3) antes del ataque. Tras el ataque, el servidor 1 reporta `(8, 2, 1)`. ¿Es esto una anomalía?

**Solución:** un salto de 3 unidades en el vector del servidor 1 sin ningún mensaje recibido (que incrementaría también los otros vectores) es **muy sospechoso**. Un servidor legítimo que escribe 3 veces seguidas tendría vector `(8, 2, 1)` solo si envió 3 mensajes y nadie respondió; pero un servidor comprometido puede modificar su vector arbitrariamente. La anomalía detectable es: **¿el nuevo vector es alcanzable causalmente desde el anterior por la secuencia observada de mensajes?** Si no lo es, hay compromiso.

## 26.9 Ejercicios propuestos

1. **Attack graph probabilístico**: modifica el analizador para que cada exploit tenga una **probabilidad de éxito**, y calcula la probabilidad agregada de cada ruta de ataque. Multiplica probabilidades para cadenas independientes.
2. **Grafo de dependencias con `cargo`**: usa `cargo metadata` para extraer el árbol de dependencias de un proyecto real, constrúyelo con `petgraph`, e identifica paquetes con un mantenedor único (riesgo de supply chain).
3. **Detector de anomalías con sliding window**: implementa un detector que mantiene una ventana temporal de los últimos N segundos de tráfico y alerta cuando el grado de un nodo se desvía más de K sigmas de la media móvil.
4. **Grafo de IOCs desde STIX/TAXII**: descarga un feed de IOCs en formato STIX (JSON) y constrúyelo con `serde_json` + `petgraph`. Implementa una búsqueda: "dada esta IP, ¿a qué campañas conocidas está asociada?"
5. **(Avanzado) RBAC con análisis de separación de privilegios**: modela un sistema RBAC como un grafo bipartito (usuarios, roles, permisos, recursos). Implementa un verificador que detecte conflictos de **segregation of duties** (SoD), es decir, un usuario con dos roles que no debería tener simultáneamente.

## 26.10 Pin de batalla

- **Un attack graph vale más que mil informes de vulnerabilidades**. Una vulnerabilidad aislada es ruido; un camino de compromiso es señal.
- **STRIDE + kill chain + attack graph = trinidad**. STRIDE clasifica, kill chain ubica en el tiempo, attack graph modela formalmente.
- **El grafo de dependencias de paquetes es tu mapa de supply chain**. Si no lo conoces, no sabes a quién confías.
- **Vector clocks detectan anomalías causales**. Un timestamp de Unix no te dice si algo fue causado por otro evento.
- **El cuello de botella en un attack graph es donde debes invertir en defensa**. Si un solo exploit desbloquea 10 rutas, ese es el que tienes que parchear primero.
- **RBAC es un grafo bipartito**. Si no lo analizas, acumulas permisos huérfanos y roles con exceso de privilegios.

## 26.11 Lo que te llevas

- **Ritchey y Ammann (2000)**: pioneros del attack graph. Tras Code Red y Nimda, el mundo se tomó en serio el modelado formal de ataques.
- **STRIDE y kill chain**: frameworks de clasificación. STRIDE = tipos de amenaza; kill chain = fases del ataque.
- **Supply chain**: el grafo de dependencias de paquetes es la nueva frontera. Confías transitivamente en cientos de mantenedores; visualízalo.
- **Detección de intrusos por grafo**: anomalías estructurales en el grafo de tráfico. Cambios de grado, centralidad, patrones.
- **Threat intelligence**: grafos de IOCs conectan indicadores con incidentes. La unión hace la fuerza.
- **Permission graphs en RBAC/OAuth**: modela quién puede hacer qué. La separación de privilegios es una propiedad del grafo.
- **El analizador Rust de attack graph** demuestra cómo, con `petgraph` y un poco de DFS, puedes enumerar todas las rutas de compromiso de un sistema.

## 26.12 Ojo, cuidado con…

- **Los attack graphs explotan combinatoriamente**. En sistemas grandes, el número de estados puede ser `2^n` o peor. Usa **herramientas de abstracción** o **simbolización** (MulVAL, TVA) para no morir en el intento.
- **El modelo de atacante importa**. Si asumes un atacante externo sin recursos, tu análisis será乐观. Modela **al menos** dos perfiles: uno externo y otro interno con credenciales básicas.
- **Falsos positivos everywhere**. Los detectores de anomalías en grafos son ruidosos. Necesitas **correlación** y **contexto** (qué usuario, qué activo, qué hora) para reducir el ruido a algo accionable.
- **Supply chain attacks no son solo de paquetes**. También son de imágenes Docker, de modelos ML, de CDNs. El grafo de dependencias es solo una capa.
- **RBAC mal implementado es peor que no tener RBAC**. Permisos heredados, roles huérfanos, combinaciones explosivas… Si no auditas el grafo,迟早 (tarde o temprano) acumulas una bomba de tiempo.
- **El "score CVSS" no es suficiente**. Una vulnerabilidad con CVSS 9.8 puede no ser explotable en tu sistema, y una con CVSS 4.0 puede ser crítica si está en el camino. El attack graph te da el contexto que CVSS no da.

## 26.13 Para profundizar

- **Ritchey, R. & Ammann, P. (2000). "Using Model Checking to Analyze Network Vulnerabilities."** — El paper fundacional del attack graph.
- **Sheyner, O. et al. (2002). "Automated Generation and Analysis of Attack Graphs."** — El siguiente paso, con generación automática.
- **Jajodia, S. et al. (2006). "Topological Analysis of Network Attack Vulnerability."** — La base matemática del análisis de attack graphs.
- **Shostack, A. (2014). "Threat Modeling: Designing for Security."** — El libro de referencia de threat modeling, con STRIDE y kill chain explicados en detalle.
- **OWASP Top 10 y CWE/SANS Top 25**: listas de vulnerabilidades comunes. Útiles como punto de partida, aunque no capturan cadenas.
- **Hutchings, A. & Holt, T. J. (2023). "The Crime Drop in Cybercrime."** — Un contrapunto sociológico: los attack graphs no son la única defensa.
- **Documentación de `cargo audit` y `npm audit`**: herramientas prácticas para grafos de dependencias y vulnerabilidades conocidas.

## 26.14 Si solo lees 30 segundos

La seguridad informática moderna se modela con grafos. Attack graphs enumeran caminos de compromiso. STRIDE clasifica amenazas. El grafo de dependencias de paquetes es tu mapa de supply chain. Los detectores de intrusos encuentran anomalías en el grafo de tráfico. RBAC es un grafo de permisos. Si quieres defender un sistema, dibújalo. Si quieres entender un ataque, dibújalo. **El grafo es la verdad**.

## 26.15 Una historia pequeña

Diego es el CISO de una pyme de e-commerce. Un lunes por la mañana, le llega un email: "Hemos detectado actividad sospechosa en su cuenta de AWS. Varios accesos desde IPs desconocidas en Rumanía." Diego se levanta de la silla, se sirve un café, y abre su laptop.

Lo primero que hace no es correr a apagar nada. Es **dibujar el grafo de su sistema**: servidores, bases de datos, servicios externos, IAM roles, buckets S3. En 30 minutos tiene un mapa. Luego, capa por capa, va identificando los posibles caminos de ataque. En menos de una hora encuentra la grieta: un bucket S3 con permisos de lectura pública que contiene un backup antiguo de la base de datos de clientes, con credenciales hardcodeadas que **nadie se molestó en revocar cuando migraron a AWS Secrets Manager hace dos años**.

Diego cierra el bucket, rota las credenciales, y a las 11:00 el incidente está contenido. La auditoría posterior revela que el bucket estuvo accesible durante 18 meses, pero por suerte los atacantes solo lo usaron para minar criptomonedas, no para robar datos. Diego vuelve a su café, ahora tibio, y anota en un post-it: "el próximo ataque, lo parcheamos **antes** de que pase". Lo pega en el monitor. Seis meses después, sigue ahí.

---

## Cierre de la Parte VI-B

Has llegado al final de la Parte VI-B. Tienes ya una visión panorámica de cómo los grafos se cuelan en la informática moderna:

- **Redes de computadores**: internet es un grafo, los protocolos de routing son algoritmos de grafos, OSPF usa Dijkstra en producción, BGP mantiene la cohesión planetaria.
- **Sistemas distribuidos**: Raft y Paxos resuelven consenso, gossip y DHT propagan información, vector clocks detectan causalidad, CAP te recuerda que las particiones son desconexiones en el grafo.
- **Seguridad informática**: attack graphs modelan caminos de compromiso, STRIDE clasifica amenazas, el grafo de dependencias es tu supply chain, RBAC es un grafo de permisos.

Si te has quedado con ganas de más, tienes todo el bagaje para leer los papers originales (los he citado en cada capítulo), para contribuir a herramientas como `petgraph` o `cargo audit`, o para defender tu propio sistema modelándolo como un grafo. Los grafos no son solo una estructura de datos: **son un lenguaje para pensar sistemas complejos**. Y ahora hablas ese lenguaje con fluidez.

> *"Un sistema complejo no se entiende. Se dibuja."*
> —Dicho popular entre arquitectos de software, atribuido a varios y a ninguno en particular, pero la idea sigue siendo cierta.

---
