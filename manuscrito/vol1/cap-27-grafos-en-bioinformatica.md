# Capítulo 27 — Grafos en Bioinformática

**[HOOK]** Tu cuerpo tiene trillones de células. Cada célula tiene dos metros de ADN enrollados en un espacio del tamaño de una semilla de amapola. Para encontrar sentido a ese librillo, los biólogos lo cortan, lo comparan, lo pegan y lo dibujan. ¿La herramienta que aparece una y otra vez, desde los años sesenta hasta AlphaFold? Un grafo. Bienvenido a la bioinformática, donde el alfabeto es A, C, G, T y el mundo se parece sospechosamente a un grafo dirigido.

## 27.0 La anécdota de la esquina

En mil novecientos sesenta y cinco, una química llamada **Margaret Dayhoff** publicó un libro modesto, el *Atlas of Protein Sequence and Structure*. Nadie esperaba que se volviera la piedra Rosetta de la biología computacional. Dayhoff se preguntó algo casi filosófico: si dos proteínas son primas evolutivas, ¿cuánto le costaría a la naturaleza transformar una en otra, aminoácido por aminoácido?

Lo que hizo fue brillante y, en retrospectiva, obvio. Tomó los árboles genealógicos de proteínas conocidas y contó, para cada par de aminoácidos, cuántas veces uno mutaba en el otro a lo largo de la evolución. Esos conteos los empaquetó en una tabla 20×20: la primera **PAM matrix** (Point Accepted Mutation). Una matriz cuadrada con números, sí. Pero cuando la usas para alinear secuencias, cada celda se convierte en una arista ponderada entre aminoácidos. Dayhoff había construido, sin saberlo, uno de los grafos implícitos más usados de la historia de la computación.

Veinticinco años después, otro equipo tomó esa idea y la aceleró con un índice precomputado. Lo llamaron **BLAST** (Basic Local Alignment Search Tool), y se convirtió en el buscador de la biología. Google indexa páginas; BLAST indexa proteínas. Ambos hacen lo mismo: caminar por un grafo enorme en milisegundos.

## 27.1 El alfabeto secreto: A, C, G, T

Una secuencia de ADN es, en el fondo, una palabra larguísima sobre el alfabeto `{A, C, G, T}`. Una proteína es una palabra sobre 20 letras (los aminoácidos). Comparar dos de esas palabras es la operación más básica de toda la bioinformática, y es, formalmente, un problema de grafos.

¿Por qué? Porque alinear dos secuencias significa encontrar un camino óptimo en una matriz donde cada movimiento (match, mismatch, gap) tiene un costo. La matriz es el grafo. Las celdas son nodos. Las flechas son aristas. Y el alineamiento es un *path*.

```
        G  A  T  T  A  C  A
     +-------------------+
     | 0 -2 -4 -6 -8 -10-12-14
   A |-2  ?  ?  ?  ?  ?  ?  ?
   T |-4  ?  ?  ?  ?  ?  ?  ?
   C |-6  ?  ?  ?  ?  ?  ?  ?
```

Cada celda `[i][j]` es un nodo. Cada flecha (→, ↓, ↘) es una arista con peso. El mejor alineamiento es el camino de máxima puntuación.

## 27.2 Needleman-Wunsch: el alineamiento global

En 1970, **Saul Needleman** y **Christian Wunsch** publicaron un algoritmo de programación dinámica para alinear dos secuencias completas. Es global: asume que las dos secuencias son parientes cercanos y deben compararse de cabo a rabo.

La idea es sencilla y elegante. Construyes una matriz `(n+1) × (m+1)`. Cada celda `[i][j]` guarda la mejor puntuación de alinear los primeros `i` caracteres de la primera secuencia con los primeros `j` de la segunda. La recurrencia es:

```
F[i][j] = max(
    F[i-1][j-1] + score(s1[i], s2[j]),  // match o mismatch
    F[i-1][j]   + gap_penalty,          // gap en s2
    F[i][j-1]   + gap_penalty           // gap en s1
)
```

Esto es Bellman-Ford, es Floyd-Warshall, es cualquier DP de la Parte IV. Solo que disfrazado de biología. Por eso cuando lo miras con ojos de grafo, ves algo así:

```
       s2[j-1] →  s2[j]
            ↘       ↓
   s1[i-1] ──→ F[i][j]
            ↓       ↘
       s1[i]   →  s1[i+1]
```

Tres aristas entrando a cada nodo, una por cada decisión. Cuando terminas la matriz, recorres las flechas hacia atrás desde `[n][m]` y reconstruyes el alineamiento: el camino de oro entre dos genomas.

## 27.3 Smith-Waterman: cuando las secuencias son casi陌生人

Diez años después, **Temple Smith** y **Michael Waterman** se preguntaron: ¿qué pasa si solo una región de las dos secuencias es similar, y el resto es ruido? El alineamiento global te obliga a comparar todo, incluyendo el ruido. Smith-Waterman introduce una cuarta opción a la recurrencia: empezar de cero. La puntuación nunca baja de cero, y el alineamiento puede "nacer" y "morir" en cualquier celda.

En términos de grafos: ahora tienes un nodo fuente ficticio conectado a cada celda con peso 0, y un nodo sumidero que recoge los alineamientos locales. Es el truco de los componentes conexos de la Parte I, pero aplicado a secuencias.

## 27.4 Ensamblado de genomas: el rompecabezas más difícil del mundo

Secuenciar un genoma humano produce millones de fragmentos cortos (los *reads*) de unos 100-300 caracteres. Tu trabajo es pegarlos en el orden correcto, como un rompecabezas de seis mil millones de piezas del que solo tienes fotocopias borrosas. Esto se llama el problema del **Shortest Common Superstring** (SCS), y es NP-hard en general.

```
   READ_001:  ...ACGTACGT...
   READ_047:  ...CGTACGTA...
   READ_112:  ...GTACGTAC...
                  ↓ ↓ ↓ ↓
   GENOMA:    ...ACGTACGTACGTACGTAC...
```

En 2001, el **Proyecto Genoma Humano** resolvió esto para nuestra especie, con un presupuesto de tres mil millones de dólares y algoritmos que hacían llorar a los clusters de Linux. Hoy lo hace tu laptop con un Nanopore y un script en Rust.

## 27.5 Grafos de De Bruijn: la genialidad compacta

En vez de pegar reads como un dominó, los bioinformáticos modernos (Illumina, por ejemplo) convierten cada read en todos sus k-mers (subsecuencias de longitud k) y los conectan cuando se solapan en k-1 caracteres. El resultado es un **grafo de De Bruijn**, donde:

- Cada nodo es un k-mer.
- Cada arista `(u, v)` existe si los últimos k-1 caracteres de `u` coinciden con los primeros k-1 de `v`.

```
   ACGT ──→ CGTA ──→ GTAC ──→ TACG ──→ ACGT
                                          (ciclo si hay repetición)
```

Magia. Ahora el genoma es un **Eulerian path** (Parte III) sobre el grafo, no un camino sobre los reads originales. Pasar de Hamiltonian (caro) a Eulerian (barato) fue el equivalente bioinformático de cambiar un martillo por una excavadora.

**Regla de tres + inesperado:**
- Los humanos tenemos ~20.000 genes.
- Un arroz tiene más genes que tú.
- Una cebolla tiene más genes que un arroz.

(Y no, eso no te da permiso para llorar cuando los pelas.)

## 27.6 Redes PPI: el vecindario de las proteínas

Una proteína no trabaja sola. Hace pareja, forma complejos, se asocia con otras. Esto se modela con una red de **Protein-Protein Interactions** (PPI): nodos son proteínas, aristas son interacciones físicas detectadas experimentalmente.

```
   TP53 ─── MDM2
     │ ╲     │
     │  ╲    │
   ATM   BRCA1
     │     │
     └──── CREBBP
```

Aquí entra toda la artillería de la Parte V: centralidad de grado para encontrar hubs, betweenness para detectar cuellos de botella metabólicos,PageRank para encontrar proteínas "influyentes". La proteína TP53, por ejemplo, es el Brad Pitt de la red PPI: aparece en casi todo, conecta con casi todos, y su mal funcionamiento está detrás de medio cáncer.

## 27.7 Phylogenetics: árboles (grafos) evolutivos

Un **árbol filogenético** es, literalmente, un grafo acíclico donde las hojas son especies actuales y los nodos internos son ancestros comunes. **UPGMA** y **neighbor-joining** son algoritmos para construir ese árbol a partir de una matriz de distancias (que es un grafo completo ponderado entre especies).

```
              ┌── Humano
         ┌────┤
         │    └── Chimpancé
    ─────┤
         │    ┌── Ratón
         └────┤
              └── Rata
```

El árbol evolutivo no es "la verdad": es la mejor hipótesis dado un modelo. Cuando ves un árbol con coeficientes de bootstrap del 100%, alguien encontró una señal evolutiva muy fuerte. Cuando ves ramas con 60%, esa parte del árbol está admitiendo, humildemente, que no está segura.

## 27.8 Redes metabólicas y regulatorias

El metabolismo de una célula es una red donde nodos son metabolitos (glucosa, ATP, piruvato) y aristas son reacciones catalizadas por enzimas. Las **redes regulatorias** añaden otra capa: genes que regulan a otros genes. Juntas forman un grafo bipartito y dirigido que los sistemas biológicos regulan homeostáticamente.

**Truco mental del día:** las redes biológicas son scale-free. Pocos nodos con muchísimas conexiones (hubs), muchos nodos con pocas. Esto es importante: significa que si atacas un hub, derribas media red. Es la base de la toxicología moderna y, también, de por qué algunos fármacos funcionan.

## 27.9 Mini-diálogo: en el laboratorio

—Oye, ¿por qué insistes en que Needleman-Wunsch es un grafo? Es claramente una matriz.

—Porque lo es, Elena. La recurrencia define aristas, las celdas son nodos. ¿Ves esta flecha hacia `[i-1][j-1]`? Es una arista con peso `score(s1[i], s2[j])`.

—Pero no la dibujas.

—No hace falta. El grafo está ahí, implícito, como el campo gravitatorio de la Tierra. Los algoritmos no distinguen entre "matriz con recurrencia" y "grafo con pesos". Por eso DP y grafos son la misma familia.

—¿Y por qué me importa?

—Porque cuando vienen secuencias de un millón de pares de bases, y necesitas alinearlas, los trucos que aprendiste en Bellman-Ford te salvan la vida. O al menos, te ahorran tres días de cómputo.

## 27.10 Implementación Rust: Needleman-Wunsch

Vamos a implementar el clásico. Usaremos `bio` para utilidades de secuencias y escribiremos la DP a mano para que se vea el grafo.

```rust
// Cargo.toml:
// [dependencies]
// bio = "1.5"

use bio::align::pairwise::Scoring;
use bio::align::pairwise::Aligner;

/// Needleman-Wunsch con scoring simple:
///  +match    si letras iguales
///  -mismatch si letras distintas
///  -gap      por cada hueco
///
/// Devuelve (score, alineamiento).
pub fn needleman_wunsch(s1: &str, s2: &str,
                        match_score: i32,
                        mismatch: i32,
                        gap: i32) -> (i32, (String, String))
{
    let a: Vec<char> = s1.chars().collect();
    let b: Vec<char> = s2.chars().collect();
    let n = a.len();
    let m = b.len();

    // Matriz (n+1) x (m+1). El "grafo implícito".
    let mut dp = vec![vec![0i32; m+1]; n+1];

    // Bordes: empezar con gaps acumulados
    for i in 0..=n { dp[i][0] = (i as i32) * gap; }
    for j in 0..=m { dp[0][j] = (j as i32) * gap; }

    // Llenado: las 3 aristas de cada nodo
    for i in 1..=n {
        for j in 1..=m {
            let diag = dp[i-1][j-1]
                + if a[i-1] == b[j-1] { match_score } else { mismatch };
            let up   = dp[i-1][j]   + gap;
            let left = dp[i][j-1]   + gap;
            dp[i][j] = diag.max(up).max(left);
        }
    }

    // Backtrack: caminamos hacia atrás por las flechas
    let mut i = n;
    let mut j = m;
    let mut aln1 = String::new();
    let mut aln2 = String::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let diag_score = dp[i-1][j-1]
                + if a[i-1] == b[j-1] { match_score } else { mismatch };
            if dp[i][j] == diag_score {
                aln1.insert(0, a[i-1]);
                aln2.insert(0, b[j-1]);
                i -= 1; j -= 1;
                continue;
            }
        }
        if i > 0 && dp[i][j] == dp[i-1][j] + gap {
            aln1.insert(0, a[i-1]);
            aln2.insert(0, '-');
            i -= 1;
        } else {
            aln1.insert(0, '-');
            aln2.insert(0, b[j-1]);
            j -= 1;
        }
    }

    (dp[n][m], (aln1, aln2))
}

fn main() {
    let s1 = "GATTACA";
    let s2 = "GCATGCU";
    let (score, (a1, a2)) = needleman_wunsch(s1, s2, 1, -1, -2);
    println!("Score: {}", score);
    println!("s1:    {}", a1);
    println!("s2:    {}", a2);

    // Con la crate 'bio', podrías hacer:
    let scoring = Scoring::new(1, -1, |a, b| if a == b { 1 } else { -1 });
    let mut aligner = Aligner::new(1, -2, scoring);
    let alignment = aligner.local(s1, s2);
    println!("{:?}", alignment);
}
```

> Nota: la crate `bio` ofrece `Aligner::global`, `local` y `semiglobal`. Aquí la reimplementamos para que se vea el grafo. En producción, usa la crate.

## 27.11 Ejercicios resueltos

**Ejercicio 1.** Alinea `ACGT` y `AGT` con match=2, mismatch=-1, gap=-2. Muestra la matriz.

```
        ε  A   G   T
    ε   0  -2  -4  -6
    A  -2   2   0  -2
    C  -4   0   1  -1
    G  -6  -2   2   0
    T  -8  -4   0   2
```

Score final: 2. Alineamiento: `A-CGT` / `A-G-T` con un gap en s2.

**Ejercicio 2.** ¿Cuántas aristas tiene el grafo implícito de Needleman-Wunsch para secuencias de longitud n y m? Cada nodo interior tiene 3 aristas entrantes: 3nm en total, más los bordes.

**Ejercicio 3.** El algoritmo de Smith-Waterman es idéntico a Needleman-Wunsch pero con la opción "score = 0". Explica en una frase por qué esto lo convierte en alineamiento local.

*Respuesta:* porque permite que un alineamiento "empiece de cero" en cualquier celda, ignorando el resto. La puntuación nunca baja, así que los caminos óptimos quedan contenidos localmente.

## 27.12 Ejercicios propuestos

1. Implementa Smith-Waterman completo en Rust sobre el código de la sección 27.10.
2. Dado el grafo de De Bruijn con k=3 de la secuencia `ACGTACGT`, dibújalo y encuentra el Eulerian path.
3. Compara los tiempos de Needleman-Wunsch con dos esquemas: matriz completa y la versión "two-rows" que solo guarda la fila anterior.
4. Construye un grafo PPI de 10 proteínas (puedes inventar las interacciones) y calcula la centralidad de grado. ¿Quién sería el hub?
5. Investiga qué algoritmo usa BLAST para acelerar la búsqueda y explica su relación con grafos.

## 27.13 Pin de batalla

- **Usa BLOSUM o PAM en vez de match/mismatch simple.** Las matrices de sustitución reflejan la bioquímica real. Score +1/-1 es didáctico; en producción, BLOSUM62.
- **Las cadenas cortas no necesitan índices.** Las largas (genomas) sí. BLAST precomputa un índice tipo hash; BWA hace lo mismo para reads cortos.
- **De Bruijn > Overlap-Layout-Consensus** para datos con alta cobertura. Si tienes reads cortos y muchos, De Bruijn gana por goleada.
- **Un grafo PPI sin ponderar es una caricatura.** Las interacciones tienen confianza, dirección, contexto. Modela con grafos con atributos (parte IV).
- **Visualiza siempre.** Cytoscape, Graphviz, o un script en Rust con `petgraph` exportando DOT. Ver el grafo es entenderlo.

## 27.14 Lo que te llevas

- Las secuencias biológicas se comparan con DP sobre un grafo implícito de matriz.
- Needleman-Wunsch es global, Smith-Waterman es local; ambos son DP, ambos son grafos.
- De Bruijn convierte ensamblado de genomas en un Eulerian path: brillante.
- Las redes PPI, metabólicas y regulatorias son grafos reales, no metáforas.
- Tu cuerpo es un grafo. De verdad.

## 27.15 Ojo, cuidado con…

- **No confundas Needleman-Wunsch con Hirschberg.** Hirschberg (1975) hace lo mismo pero en espacio lineal con divide y vencerás. Útil para secuencias muy largas.
- **Las matrices PAM y BLOSUM no son universales.** PAM1 para secuencias muy similares, BLOSUM62 para divergencia media. Elegir mal distorsiona resultados.
- **Un árbol filogenético no es la verdad**, es la mejor hipótesis bajo un modelo. Lee el bootstrap antes de creer.
- **Las redes scale-free no son "robust by design"**. Son robustes a fallos aleatorios, frágiles a ataques dirigidos. Si atacas los hubs, la red cae.

## 27.16 Para profundizar

- **libros**: *Bioinformatics Algorithms* (Compeau & Pevzner), *Biological Sequence Analysis* (Durbin et al.).
- **papers**: Needleman & Wunsch 1970, Smith & Waterman 1981, Altschul et al. 1990 (BLAST).
- **crates**: `bio`, `rust-bio`, `ndarray`, `petgraph` para visualización.
- **cursos**: Coursera "Biology Meets Programming", Rosalind (plataforma de ejercicios).

## 27.17 Si solo lees 30 segundos

Bioinformática = grafos + biología. Needleman-Wunsch y Smith-Waterman alinean secuencias con DP sobre matrices-grafo. De Bruijn hace ensamblado via Eulerian path. Las redes PPI, metabólicas y regulatorias son grafos reales que se analizan con centralidad. Tu ADN, tus proteínas, tu metabolismo: todo son grafos. La próxima vez que comas una cebolla con más genes que tú, recuerda que la biología y la computación son primos cercanos.

## 27.18 Una historia pequeña

Lucía, estudiante de biotecnología, odiaba las matemáticas. Un día, harta de alinear secuencias a mano para su TFG, escribió un script en Rust. Empezó con Needleman-Wunsch de cien líneas. Después añadió Smith-Waterman. Luego, seducida por el código, aprendió `petgraph` y visualizó una red PPI en formato DOT. La vio renderizada y se quedó quieta un momento. Esa telaraña de proteínas era su proyecto, pero también era un grafo. Esa noche, por primera vez en su carrera, abrió un libro de algoritmos en vez de uno de bioquímica. Y durmió mejor.

---

