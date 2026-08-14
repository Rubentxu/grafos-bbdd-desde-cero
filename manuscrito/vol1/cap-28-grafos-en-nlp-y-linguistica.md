# Capítulo 28 — Grafos en NLP y Lingüística

**[HOOK]** El lenguaje humano es la estructura más complicada que la evolución ha producido. Tiene gramática recursiva, ambigüedad infinita y reglas que se rompen a propósito. ¿Cómo lo modela una máquina? Con grafos, por supuesto. El lenguaje no es una lista de palabras. Es una telaraña, un árbol, un laberinto. Y cada vez que abres un buscador, un chatbot o un traductor, estás caminando por uno.

## 28.0 La anécdota de la esquina

En mil novecientos ochenta y cinco, un psicólogo de Princeton llamado **George Miller** publicó algo raro: una base de datos de palabras inglesas conectadas por sinónimos. La llamó **WordNet**. Miller llevaba años intentando que los lexicógrafos — esos académicos que escriben diccionarios — adoptaran algo más sistemático. Los lexicógrafos, curtidos en el papel y la tinta, no querían saber nada de bases de datos, ni de "redes semánticas", ni de "grafos". Demasiado formal, demasiado computacional.

Miller, testarudo, insistió. La idea era simple: en vez de definir cada palabra en un párrafo, conectarla con sus sinónimos (*synsets*), hiperónimos (es un), hipónimos (tipo de), y merónimos (parte de). El resultado fue un grafo de más de cien mil nodos. Hoy, WordNet es el Lego con el que se construyen medio NLP clásico y una buena parte del moderno.

La ironía: Miller no se consideraba un informático. Era psicólogo. Pero el grafo que inventó cambió la lingüística computacional para siempre. A veces, las mejores contribuciones a un campo vienen de alguien que apenas lo pisa.

## 28.1 El lenguaje como red

Antes de meternos en árboles y grafos sintácticos, una verdad incómoda: el lenguaje natural se modela naturalmente como una red. Las **co-ocurrencias** (qué palabras aparecen juntas) forman un grafo enorme. Los **sinónimos** forman otro. Las **traducciones** entre idiomas forman un grafo bipartito. Casi cualquier fenómeno lingüístico serio se reduce, al final, a: "hay un grafo por aquí".

**Regla de tres + inesperado:**
- Los verbos irregulares del español son unos 250.
- Los del inglés, unos 180.
- Los del alemán, casi infinitos. (Y sí, los alemanes están orgullosos.)

## 28.2 Dependency parsing: la frase como árbol

Una frase en lenguaje natural no es una lista plana de palabras. Es una estructura. "El gato negro come pescado" no es `[El, gato, negro, come, pescado]`. Es una jerarquía donde "come" es el núcleo, "gato" su sujeto, "negro" un modificador de "gato", "pescado" el objeto.

El **dependency parsing** modela esto como un grafo dirigido: cada palabra es un nodo, y cada flecha es una relación gramatical (`nsubj`, `dobj`, `amod`, `det`, etc.). El resultado es un árbol (más o menos).

```
        come  (VERB, raíz)
       /    \
     gato  pescado
    /   \
   El   negro

  Relaciones:
    El  → gato       (det)
    negro → gato     (amod)
    gato → come      (nsubj)
    pescado → come   (dobj)
```

En la práctica, las dependencias no siempre forman un árbol limpio: hay ciclos, hay aristas múltiples, hay nodos sueltos. Por eso el dependency parser devuelve un grafo dirigido, no un árbol formal. Los algoritmos para construirlo son el "transition-based parsing" y el "graph-based parsing". Ambos son búsquedas en grafos, disfrazadas de lingüística.

## 28.3 Constituency parsing: la gramática como árbol (CFGs)

Una **gramática libre de contexto** (CFG) es un conjunto de reglas de producción. Las reglas, técnicamente, forman un grafo: nodos son símbolos (terminales y no terminales), aristas son producciones. Cuando parseas una frase, recorres ese grafo desde `S` (símbolo inicial) hasta las palabras.

```
   S
   |
  NP ──── VP
  |        |
 Det      V  ──── NP
  |       |        |
  El    come    Det ── N
                    |    |
                    un  gato
```

El **Cocke-Kasami-Younger (CKY)** algorithm hace esto con DP sobre la gramática, en tiempo O(n³ · |G|). Es el mismo Bellman-Ford de la Parte IV, con un traje de lingüista.

La diferencia con dependency parsing: constituency es jerárquico (qué contiene qué), dependency es relacional (quién depende de quién). Los humanos leen los dos a la vez sin pensarlo. Las máquinas, todavía, con dificultad.

## 28.4 WordNet, ConceptNet, FrameNet: las tres redes seminales

**WordNet** (Princeton, 1985) es el abuelo. Grafos de sinónimos en inglés, ahora en muchos idiomas. Usa `petgraph` con `DiGraph<Nodo, Relacion>` y unas 50 mil aristas en su núcleo.

**ConceptNet** (MIT, 1999) es el primo multicultural. UneWordNet con conocimiento de sentido común. "Gato es un mamífero", "llover hace que la gente lleve paraguas". Aristas con pesos.

**FrameNet** (Berkeley, 1997) es el pariente teórico. Modela "frames" semánticos: situaciones prototípicas con participantes. El frame "compra" tiene comprador, vendedor, objeto, dinero.

Las tres son grafos. Las tres siguen activas. Las tres son, en parte, código abierto.

## 28.5 Knowledge graphs: cuando Google entra a la fiesta

En 2012, Google anunció su **Knowledge Graph** con bombos y platillos. La idea: en vez de devolver páginas que contienen tus palabras, devolver *entidades* conectadas. "Barack Obama" es un nodo, "es presidente de" es una arista, "Estados Unidos" es otro nodo.

```
   Barack Obama ──es presidente de──→ Estados Unidos
         │                                  │
         │ esposa de                        │ capital
         ↓                                  ↓
   Michelle Obama                        Washington D.C.
```

Otros grandes: **Wikidata** (colaborativo, abierto), **DBpedia** (extraído de Wikipedia), **YAGO** (mezcla de Wikipedia y WordNet). Todos comparten una idea: modelar el mundo como un grafo de entidades y relaciones, consultable con un lenguaje tipo SPARQL.

**Detalle geek**: en RDF (el formato estándar), un Knowledge Graph es un multigrafo dirigido con aristas etiquetadas, donde cada arista puede tener su propio grafo de propiedades. Un grafo de grafos, esencialmente.

## 28.6 Embeddings y node2vec: cuando el grafo se vuelve vector

Los modelos clásicos de NLP (TF-IDF, LSA) producen vectores dispersos. Los modernos (Word2Vec, GloVe) producen vectores densos de ~300 dimensiones donde palabras similares están cerca. ¿Cómo se hace?

**GloVe** (Pennington et al., 2014) factoriza la matriz de co-ocurrencia global. Esa matriz es, técnicamente, un grafo de co-ocurrencia pesado.

**TransE** (Bordes et al., 2013) es para Knowledge Graphs: cada entidad y cada relación son vectores, y se entrena de modo que `head + relation ≈ tail`. Si "París - Francia ≈ Roma - Italia", el modelo aprendió la noción de "capital de".

**node2vec** (Grover & Leskovec, 2016) extiende Word2Vec a cualquier grafo: paseos aleatorios sesgados producen "frases" que el modelo convierte en vectores. Después, nodos similares tienen vectores similares.

```
   word2vec    "El gato come"
                ↓ paseos
   node2vec    "Gato → come → pescado → come → atún"
                ↓
   vectores    [0.12, -0.45, 0.78, ...]
```

Misma idea, dominios distintos. El truco está en que el espacio vectorial preserva la estructura del grafo. Las distancias en el embedding se corresponden con distancias en el grafo original.

## 28.7 Mini-diálogo: en una cafetería de la facultad

—Camila, ¿me puedes ayudar con un dependency parser?

—Depende. ¿Tu grafo tiene ciclos?

—A veces.

—Entonces no es un árbol, es un grafo. ¿Estás usando un transition-based o un graph-based?

—Transition-based. Arc-Standard.

—Bien. Cada estado es un nodo en un grafo implícito enorme. Las acciones (SHIFT, LEFT-ARC, RIGHT-ARC) son aristas. Y tu algoritmo de parsing es esencialmente un BFS/Dijkstra sobre ese grafo.

—¿Y por qué nadie me lo dijo así?

—Porque a los lingüistas les da urticaria hablar de grafos. Y a los informáticos les da urticaria hablar de lingüística. Es un problema cultural. Te recomiendo leer el capítulo sobre parsing de Jurafsky con la cabeza puesta en grafos. Todo encaja.

## 28.8 Implementación Rust: un mini dependency parser

Vamos a implementar un parser de dependencias *transition-based* simplificado. Usamos `petgraph` para el grafo y operaciones simples de shift-reduce.

```rust
// Cargo.toml:
// [dependencies]
// petgraph = "0.6"

use petgraph::graph::DiGraph;
use petgraph::dot::Dot;
use std::collections::HashMap;

/// Representa una palabra y sus features básicos.
#[derive(Clone, Debug)]
struct Token {
    form: String,
    pos: String,        // categoría gramatical
}

#[derive(Clone, Debug)]
struct DepEdge {
    relation: String,
}

/// Estado del parser: una pila y un buffer de tokens pendientes.
struct ParserState {
    stack: Vec<usize>,      // índices en el grafo
    buffer: Vec<usize>,     // índices en el grafo
}

/// Parser arc-standard minimal.
/// Construye un grafo dirigido de dependencias.
pub struct DependencyParser {
    graph: DiGraph<Token, DepEdge>,
}

impl DependencyParser {
    pub fn new() -> Self {
        Self { graph: DiGraph::new() }
    }

    /// Añade tokens al grafo. Devuelve sus índices.
    pub fn add_tokens(&mut self, tokens: Vec<Token>) -> Vec<usize> {
        tokens.into_iter().map(|t| self.graph.add_node(t)).collect()
    }

    /// Crea una arista de dependencia entre head y dependent.
    pub fn add_dep(&mut self, head: usize, dep: usize, rel: &str) {
        self.graph.add_edge(
            self.graph.node_indices().nth(head).unwrap(),
            self.graph.node_indices().nth(dep).unwrap(),
            DepEdge { relation: rel.to_string() },
        );
    }

    /// Arc-standard: aplica SHIFT y arcs hasta vaciar el buffer.
    pub fn parse_arc_standard(&mut self, sentence: Vec<Token>) {
        let indices = self.add_tokens(sentence);
        let n = indices.len();
        let mut state = ParserState {
            stack: vec![indices[0]],         // raíz inicial
            buffer: indices[1..].to_vec(),
        };

        while !state.buffer.is_empty() {
            // SHIFT
            let next = state.buffer.remove(0);
            state.stack.push(next);
        }

        // Tras vaciar el buffer, creamos aristas simples:
        //   cada token depende del anterior (cadena lineal).
        // En un parser real, las acciones se deciden con un clasificador
        // entrenado (MLP, red neuronal, etc.).
        for i in 1..n {
            self.add_dep(i - 1, i, "next");
        }
    }

    /// Exporta el grafo en formato DOT para Graphviz.
    pub fn to_dot(&self) -> String {
        format!("{:?}", Dot::new(&self.graph))
    }
}

fn main() {
    let mut parser = DependencyParser::new();
    let sentence = vec![
        Token { form: "El".into(),   pos: "DET".into()  },
        Token { form: "gato".into(), pos: "NOUN".into() },
        Token { form: "negro".into(),pos: "ADJ".into()  },
        Token { form: "come".into(), pos: "VERB".into() },
        Token { form: "pescado".into(), pos:"NOUN".into()},
    ];

    parser.parse_arc_standard(sentence);
    println!("{}", parser.to_dot());
    // Imprimirá un grafo DOT con 5 nodos y 4 aristas.
}
```

En producción, el clasificador de acciones (¿SHIFT, LEFT-ARC, RIGHT-ARC?) se entrena con un modelo neuronal. Los *transition systems* más modernos (Chu-Liu/Edmonds, por ejemplo) usan MST sobre grafos completamente conectados con pesos aprendidos. Sí, otra vez grafos.

## 28.9 Ejercicios resueltos

**Ejercicio 1.** Construye manualmente el árbol de constituyentes de "el gato negro come pescado". Muestra los nodos y las aristas.

```
   S
   ├── NP
   │   ├── Det ("el")
   │   ├── N ("gato")
   │   └── Adj ("negro")
   └── VP
       ├── V ("come")
       └── NP
           ├── Det (implícito)
           └── N ("pescado")
```

**Ejercicio 2.** Dado el grafo de co-ocurrencia `gato ↔ come, come ↔ pescado, gato ↔ negro, come ↔ rápido`, ¿cuál es la centralidad de grado de "come"?

*Respuesta:* 3 (come aparece en 3 aristas). Es el hub local.

**Ejercicio 3.** ¿Por qué TransE funciona mejor en Knowledge Graphs que en grafos con relaciones 1-a-N?

*Respuesta:* TransE asume que la relación es una traslación en el espacio vectorial. Para relaciones 1-a-N (como "es padre de"), un único vector para "es padre de" no puede conectar muchas cabezas diferentes con sus respectivas colas. Tiene problemas con relaciones simétricas, 1-a-N y N-a-1. Variantes como TransR y RotatE lo arreglan.

## 28.10 Ejercicios propuestos

1. Implementa un parser de constituyentes CKY para la gramática `S → NP VP, NP → Det N, VP → V NP` con vocabulario "el, gato, come, pescado".
2. Construye un mini-WordNet en Rust: 20 nodos (palabras), 30 aristas (sinonimia, hiperonimia). Visualízalo con `petgraph`.
3. Dado un grafo de co-ocurrencia de 5 documentos, calcula embeddings usando un método tipo node2vec (sin librería, solo SVD sobre la matriz de transiciones).
4. ¿Cuántas aristas tiene un dependency graph "casi" arbóreo de N nodos? ¿Cuántas tiene uno completamente conectado? Explica la diferencia.
5. Implementa un grafo de Knowledge Graph simple: 10 entidades, 15 relaciones. Implementa una función `query(entity, relation)` que devuelve los nodos conectados.

## 28.11 Pin de batalla

- **El preprocesamiento importa más que el modelo.** Tokenización, lematización, etiquetado POS. Si el input es basura, el output es basura elegante.
- **WordNet es un grafo, no una ontología completa.** No modela一切 (todo). Combínalo con ConceptNet si necesitas sentido común.
- **Para NLP moderno, transformer > grafo.** Pero los grafos siguen siendo insustituibles para relaciones estructuradas, KG, y razonamiento explícito.
- **Usa spaCy o Stanza como baseline.** Reimplementar parsers desde cero es didáctico, pero en producción usa herramientas que ya funcionan.
- **Las métricas de evaluación son parte del modelo.** UAS, LAS, BLEU, ROUGE: entiéndelas antes de confiar en un número.

## 28.12 Lo que te llevas

- El lenguaje es un grafo. Las palabras se conectan por co-ocurrencia, sinónimos, dependencias.
- Dependency parsing y constituency parsing son dos formas de modelar la sintaxis: como relaciones y como jerarquías.
- WordNet, ConceptNet y FrameNet son redes semánticas seminales; los Knowledge Graphs modernos son sus herederos.
- Los embeddings (Word2Vec, GloVe, TransE) comprimen grafos en vectores. Geometría del espacio vectorial ↔ estructura del grafo.
- Un buen parser es, en el fondo, un BFS/Dijkstra con traje de lingüista.

## 28.13 Ojo, cuidado con…

- **"Knowledge Graph" no es un término técnico estricto.** Cada empresa lo redefine. Asegúrate de qué quiere decir cada uno cuando lo uses.
- **Los embeddings de grafos no son perfectos.** Capturan estructura local, pero pierden información global. Combina con otras señales si necesitas precisión.
- **Un dependency parser al 95% LAS suena bien, pero a escala produce millones de errores.** La calidad importa.
- **Cuidado con los sesgos en WordNet y KG.** Fueron construidos por humanos con sus prejuicios. "Enfermera" puede estar sesgado hacia femenino en ciertos embeddings.

## 28.14 Para profundizar

- **libros**: *Speech and Language Processing* (Jurafsky & Martin), *Foundations of Statistical Natural Language Processing* (Manning & Schütze).
- **papers**: WordNet (Miller 1995), TransE (Bordes 2013), node2vec (Grover 2016).
- **crates**: `petgraph` (siempre), `tch-rs` para embeddings neuronales, `rand` para muestreo.
- **datasets**: WordNet, ConceptNet, Universal Dependencies, WikiData.

## 28.15 Si solo lees 30 segundos

El lenguaje natural se modela como grafo en todos los niveles: palabras (co-ocurrencia, sinónimos), sintaxis (árboles de constituyentes, dependencias), semántica (WordNet, KG), y aprendizaje (embeddings). Dependency parsing es búsqueda en grafos. CKY es DP. TransE y node2vec comprimen grafos en vectores. Cada vez que tu teléfono "entiende" un comando de voz, hay grafosimplícitos haciendo su trabajo silencioso.

## 28.16 Una historia pequeña

Pablo, ingeniero junior, fue contratado para "mejorar el buscador" de una intranet. El buscador era un `grep` glorificado. Leyó sobre WordNet, leyó sobre node2vec, leyó sobre knowledge graphs. Pasó tres meses indexando la documentación interna en un grafo con `petgraph` y, encima, un índice de embeddings con `tch-rs`. Lanzó la versión nueva. El CTO buscó "oficina del rector" y la herramienta le devolvió: "Secretaría General, tercer piso, edificio histórico". Pablo sonrió. No le dieron un aumento, pero se llevó algo mejor: la certeza de que un grafo bien construido puede devolver respuestas que ningún `grep` encuentra. A veces, el código correcto no se nota hasta que se extraña.

---

