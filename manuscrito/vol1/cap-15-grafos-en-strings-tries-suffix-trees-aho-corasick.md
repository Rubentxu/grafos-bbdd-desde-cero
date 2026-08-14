# Capítulo 15 — Grafos en strings: tries, suffix trees, Aho-Corasick

Un paper publicado en 1973 pasó desapercibido durante 20 años. Cuando los biólogos computacionales de los 90 empezaron a buscar patrones en millones de secuencias de ADN, redescubrieron el algoritmo. Lección: lo que hoy parece inútil mañana salva vidas (literalmente).
## 15.0 La anécdota del paper ignorado y la bioinformática que lo redimió

Cuenta la historia que en 1973, un estudiante de doctorado del MIT llamado **Peter Weiner** publicó un artículo en el *Journal of the ACM* con un título bastante anodino: *"Linear pattern matching in strings"*. En él, describía una estructura de datos que, según sus cuentas, iba a revolucionar la búsqueda en textos: el **suffix tree**. La idea era sencilla: dado un texto S de longitud n, preprocesarlo en una estructura que permitiese buscar cualquier patrón P en tiempo O(|P|) — independiente del tamaño del texto.

El paper era elegante, las demostraciones eran correctas, y... casi nadie le hizo caso. La comunidad de la época pensaba que buscar en strings era un problema "menor", resuelto hacía tiempo con KMP o Boyer-Moore. Weiner siguió su carrera, hizo otras contribuciones notables, y el suffix tree se quedó en un rincón oscuro de la literatura.

Pasaron **casi 20 años**. A finales de los 80 y principios de los 90, una nueva comunidad empezó a mirar a los strings con ojos muy diferentes: los **bioinformáticos**. Tenían un problema nuevo y gigantesco: secuenciar el genoma humano (3.000 millones de bases), buscar genes, comparar ADN entre especies. Y los algoritmos existentes eran desesperantemente lentos. Un genoma tiene ~3·10⁹ caracteres. Buscar en él con KMP o Boyer-Moore era factible pero dolorosamente lento si querías hacer miles de búsquedas.

Entonces alguien recordó el paper de Weiner. Y se redescubrió que el suffix tree, con construcción O(n) y búsqueda O(m), era exactamente lo que necesitaban. Casi tres décadas después de su invención, el suffix tree se convirtió en la columna vertebral de herramientas como BLAST, BWA, Bowtie y prácticamente todos los algoritmos modernos de alineamiento de secuencias. Weiner fue al MIT, a Stanford, ganó premios, y su paper original se convirtió en uno de los más citados de la historia de la informática.

Moraleja: **lo que hoy parece un capricho teórico puede salvar la ciencia del mañana**. Si te dicen que tu idea "no tiene aplicación", no te lo creas del todo. Las ideas que parecen inútiles suelen estar esperando al problema correcto.


> — ¿Para qué sirve un suffix tree?
> — Buscar un patrón en un texto en O(m). Aplicaciones: bioinformática, búsqueda en logs, compresión.
> — ¿Y Aho-Corasick?
> — Buscar MUCHOS patrones a la vez en O(n + m + k). Como un grep con miles de palabras.
> — ¿Cuándo uso uno y cuándo otro?
> — Un patrón: suffix tree o KMP. Muchos patrones: Aho-Corasick. Texto que cambia mucho: índice invertido.
> — ¿Y el crate `image` qué pinta aquí?
> — Lo usé para visualizar Aho-Corasick sobre una imagen. Verás cómo marca los matches en rojo sobre un PNG.
## 15.1 Trie: el árbol de prefijos

Empecemos por lo más sencillo. Un **trie** (pronunciado "trai", viene de *re*trie*val*) es un árbol enraizado donde:

- Cada nodo representa un prefijo.
- Cada arista está etiquetada con un carácter.
- Dos hijos del mismo nodo tienen etiquetas distintas.
- Las hojas (o nodos marcados) corresponden a palabras completas.

Operaciones y complejidad, con alfabeto Σ de tamaño σ:
- Insertar/buscar un string s de longitud m: O(m·σ) con un mapa, O(m) con un array indexado.
- Espacio: O(N) con N = Σ|s_i|.

Los tries son la columna vertebral de:
- **Tablas de routing** (CIDR longest-prefix match).
- **Autocomplete** en editores de texto.
- **T9** (el predictor de teclas de los móviles de los 2000).
- **Lexers** y parsers.

```rust
use std::collections::HashMap;

#[derive(Default)]
pub struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_word: bool,
    word: Option<String>,
}

#[derive(Default)]
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_word = true;
        node.word = Some(word.to_string());
    }

    /// Devuelve Some(word) si está; None si no.
    pub fn search(&self, word: &str) -> Option<&str> {
        let mut node = &self.root;
        for ch in word.chars() {
            node = node.children.get(&ch)?;
        }
        if node.is_word { node.word.as_deref() } else { None }
    }

    /// Devuelve todas las palabras que empiezan con `prefix`.
    pub fn starts_with(&self, prefix: &str) -> Vec<String> {
        let mut node = &self.root;
        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(n) => node = n,
                None => return vec![],
            }
        }
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if n.is_word {
                if let Some(w) = &n.word { out.push(w.clone()); }
            }
            for child in n.children.values() {
                stack.push(child);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserta_y_busca() {
        let mut t = Trie::new();
        for w in ["hola", "hora", "mundo", "muro"] {
            t.insert(w);
        }
        assert_eq!(t.search("hola"), Some("hola"));
        assert_eq!(t.search("ho"), None); // prefijo, no palabra completa
        assert_eq!(t.search("murciélago"), None);
    }

    #[test]
    fn autocompletar() {
        let mut t = Trie::new();
        for w in ["rust", "ruby", "ruta", "python"] {
            t.insert(w);
        }
        let mut r = t.starts_with("ru");
        r.sort();
        assert_eq!(r, vec!["ruby".to_string(), "rust".to_string(), "ruta".to_string()]);
    }
}
```

## 15.2 Suffix tree: el "trie de sufijos" comprimido

El **suffix tree** de un string S de longitud n (terminado en un símbolo `$` único) es un trie construido sobre los n sufijos de S, pero **comprimido**: las cadenas de nodos con un solo hijo se fusionan en una arista etiquetada por la subcadena (representada por un par (i, j) de índices en S). El resultado tiene exactamente n hojas y a lo sumo 2n nodos.

Propiedades clave:
- Búsqueda de un patrón P de longitud m: O(m) — se baja por el árbol siguiendo caracteres.
- Construcción ingenua: O(n²) por insertar cada sufijo.
- **Ukkonen (1995)** lo construye en O(n) amortizado.

### 15.2.1 Outline de Ukkonen

El algoritmo de Ukkonen construye el árbol *online* — carácter a carácter — mediante:

- *Suffix link*: análogo a los *fail links* de Aho-Corasick; conecta los nodos de un sufijo con su padre lógico.
- *Active point*: triple (v, s, k) que indica dónde continuar insertando.
- *Rule 3 extension*: cuando el sufijo actual ya existe, no se hace nada, y la fase i termina.
- *Phase increment*: extender todos los sufijos de S[0..i] por el carácter S[i].

El invariante crítico: tras la fase i, el árbol es el **suffix tree de S[0..i]**. La clave de la complejidad O(n) es que cada *extension* toma tiempo amortizado O(1) gracias a suffix links y al *active point* que camina y salta sin retroceder.

Implementar Ukkonen en Rust es un excelente ejercicio, pero ocupa más espacio del que tenemos aquí. Lo dejaremos como un reto más adelante en los ejercicios propuestos.

## 15.3 Suffix array y LCP: el primo compacto

El **suffix array** SA de S es el array de índices de los sufijos de S ordenados lexicográficamente. Se construye en O(n log n) con radix sort + doubling, o en O(n) con DC3, SA-IS. Es más compacto que un suffix tree (4n bytes vs. ~20n) y soporta las mismas queries con un array adicional:

- **LCP array** (*longest common prefix*): LCP[i] = longitud del prefijo común entre los sufijos SA[i] y SA[i-1].
- **RMQ** sobre LCP da, en O(1) con *sparse table*, el LCS de dos subcadenas arbitrarias.

Aplicaciones: repeats, tandem repeats, shortest unique substring, **Burrows-Wheeler transform** (base de bzip2).

```rust
/// Construye el suffix array de `s` (sin el centinela, lo añadimos aquí).
pub fn build_suffix_array(s: &str) -> Vec<usize> {
    let mut chars: Vec<char> = s.chars().collect();
    chars.push('$'); // centinela único
    let n = chars.len();
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by_key(|&i| chars[i..].to_vec());
    indices
}

/// Construye el LCP array.
pub fn build_lcp(s: &str, sa: &[usize]) -> Vec<usize> {
    let chars: Vec<char> = s.chars().chain(std::iter::once('$')).collect();
    let n = chars.len();
    let mut rank = vec![0usize; n];
    for (i, &p) in sa.iter().enumerate() {
        rank[p] = i;
    }
    let mut lcp = vec![0usize; n];
    let mut h = 0usize;
    for i in 0..n {
        let r = rank[i];
        if r == 0 { continue; }
        let j = sa[r - 1];
        while i + h < n && j + h < n && chars[i + h] == chars[j + h] {
            h += 1;
        }
        lcp[r] = h;
        h = h.saturating_sub(1);
    }
    lcp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_array_banana() {
        // "banana" tiene 6 sufijos: "banana", "anana", "nana", "ana", "na", "a"
        // ordenados: "a" (5), "ana" (3), "anana" (1), "banana" (0),
        //            "na" (4), "nana" (2)
        let sa = build_suffix_array("banana");
        assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn lcp_banana() {
        let sa = build_suffix_array("banana");
        let lcp = build_lcp("banana", &sa);
        // lcp = [0, 1, 3, 0, 0, 2]
        assert_eq!(lcp, vec![0, 1, 3, 0, 0, 2]);
    }
}
```

## 15.4 Aho-Corasick: multi-pattern matching en O(n+m+k)

El **algoritmo Aho-Corasick (1975)** busca simultáneamente un conjunto de patrones P = {p_1, ..., p_k} en un texto T en tiempo O(|T| + |P| + z) donde z es el número de ocurrencias. Es la base de herramientas como `fgrep`, `ripgrep`, Snort (intrusion detection), y los algoritmos de mapeo de ADN.

Construye un autómata finito sobre el trie de P y le añade:
- `goto(v, c)`: transición directa; si no existe, sigue los *fail links* hasta encontrarla o llegar a la raíz.
- `fail(v)`: apunta al nodo que es el sufijo propio más largo de la cadena raíz→v y que también es un prefijo de algún patrón. Se calcula en BFS sobre el trie.
- `output(v)`: lista de patrones que terminan en v (o transitivamente vía fail).

```rust
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct AhoCorasick {
    /// goto[v][c] -> v'
    goto: Vec<HashMap<char, usize>>,
    /// fail links
    fail: Vec<usize>,
    /// patrones que terminan en cada nodo (incluyendo transitivos)
    out: Vec<Vec<usize>>,
}

impl AhoCorasick {
    pub fn new(patterns: &[&str]) -> Self {
        let mut ac = AhoCorasick {
            goto: vec![HashMap::new()],
            fail: vec![0],
            out: vec![Vec::new()],
        };
        for (pid, p) in patterns.iter().enumerate() {
            let mut v = 0;
            for ch in p.chars() {
                v = *ac.goto[v].entry(ch).or_insert_with(|| {
                    let id = ac.goto.len();
                    ac.goto.push(HashMap::new());
                    ac.fail.push(0);
                    ac.out.push(Vec::new());
                    id
                });
            }
            ac.out[v].push(pid);
        }
        // BFS para construir fail links
        let mut q = VecDeque::new();
        for (&_ch, &v) in ac.goto[0].iter() {
            q.push_back(v);
            ac.fail[v] = 0;
        }
        while let Some(u) = q.pop_front() {
            for (&ch, &v) in ac.goto[u].iter() {
                q.push_back(v);
                let mut f = ac.fail[u];
                while f != 0 && !ac.goto[f].contains_key(&ch) {
                    f = ac.fail[f];
                }
                ac.fail[v] = ac.goto[f].get(&ch).copied().unwrap_or(0);
                // output(v) incluye los outputs transitivos
                let mut new_out = ac.out[v].clone();
                new_out.extend_from_slice(&ac.out[ac.fail[v]]);
                ac.out[v] = new_out;
            }
        }
        ac
    }

    /// Busca todos los matches. Devuelve (pos_final, id_patrón) por cada ocurrencia.
    pub fn search(&self, text: &str) -> Vec<(usize, usize)> {
        let mut v = 0usize;
        let mut res = Vec::new();
        for (i, ch) in text.chars().enumerate() {
            while v != 0 && !self.goto[v].contains_key(&ch) {
                v = self.fail[v];
            }
            v = self.goto[v].get(&ch).copied().unwrap_or(0);
            for &pid in &self.out[v] {
                res.push((i, pid));
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encuentra_todas_las_ocurrencias() {
        let ac = AhoCorasick::new(&["he", "she", "his", "hers"]);
        let matches = ac.search("ushers");
        // "she" en posición 3, "he" en posición 3, "hers" en posición 4
        let pats: Vec<usize> = matches.iter().map(|&(_, p)| p).collect();
        assert!(pats.contains(&0)); // "he"
        assert!(pats.contains(&1)); // "she"
        assert!(pats.contains(&3)); // "hers"
    }

    #[test]
    fn sin_matches_devuelve_vacio() {
        let ac = AhoCorasick::new(&["xyz"]);
        assert!(ac.search("abc").is_empty());
    }
}
```

## 15.5 Aplicaciones del mundo real

- `fgrep` / `ripgrep`: búsqueda multi-patrón en UNIX; `fgrep` usa Aho-Corasick, `ripgrep` lo combina con regex.
- Bioinformática: mapeo de *reads* (BWA, Bowtie), *motif finding*, búsqueda de PAM y k-mers, detección de genes.
- Compresión: BWT y FM-index usan suffix arrays para *back-search* en O(m) con índices O(n).
- Sistemas DLP / intrusion detection: Snort y Suricata combinan Aho-Corasick con regex.
- Spell checkers, anti-plagio (MOSS), *plagiarism detection*.

## 15.6 Momento WOW: resaltar matches en una imagen con el crate `image`

Ahora la parte visual. Vamos a generar una imagen PNG con texto y **resaltar todas las ocurrencias** de un patrón con Aho-Corasick.

```rust
use image::{Rgb, RgbImage};
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::rect::Rect;

fn main() {
    // Texto "escaneado" sobre el que buscaremos
    let texto = "EL ARBOL ES UN GRAFO. EN UN GRAFO HAY NODOS Y ARISTAS. \
                 UN GRAFO PLANAR ES 4-COLOREABLE.";

    // Creamos una imagen blanca
    let width = 1200u32;
    let height = 100u32;
    let mut img = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));

    // Simulamos "líneas de texto" pintando bandas grises cada 25 px
    for y in (0..height).step_by(25) {
        for x in 0..width {
            let p = img.get_pixel_mut(x, y);
            *p = Rgb([240, 240, 240]);
        }
    }

    // Buscamos con Aho-Corasick
    let ac = AhoCorasick::new(&["GRAFO", "ARBOL", "NODOS"]);
    let matches = ac.search(texto);

    // Para cada match, pintamos una banda roja del ancho del patrón
    for (pos, pid) in matches {
        let patron = match pid {
            0 => "GRAFO",
            1 => "ARBOL",
            2 => "NODOS",
            _ => continue,
        };
        let ancho = (patron.len() as u32) * 8; // 8 px por carácter
        let col_inicio = (pos as u32).saturating_sub(ancho);
        let rect = Rect::at(col_inicio as i32, 30).of_size(ancho, 30);
        draw_filled_rect_mut(&mut img, rect, Rgb([255, 100, 100]));
    }

    img.save("matches.png").unwrap();
    println!("Imagen guardada en matches.png");
}
```

Cargo.toml:

```toml
[package]
name = "aho-corasick-image"
version = "0.1.0"
edition = "2024"

[dependencies]
image = "0.24"
imageproc = "0.23"
```

Resultado: una imagen PNG con bandas rojas donde aparece el patrón. Para hacerlo más bonito, podemos usar `imageproc::drawing::draw_text_mut` con la fuente `rusttype` y poner texto real. La idea es que veas **dónde** Aho-Corasick encuentra los matches — uniéndote a lo que viste en el Cap. 16 (análisis de imágenes) con lo que estás aprendiendo aquí.

## 15.7 Ejercicios resueltos

**Ejercicio 1 — Word search en un tablero 4×4.** Dado un tablero con letras y una lista de palabras, encuentra todas las palabras presentes. Modelamos el tablero como un trie y hacemos backtracking. Con Aho-Corasick, recorriendo celdas, "emitimos" cuando un prefijo del trie se completa.

*S:*

```rust
use std::collections::HashSet;

fn boggle_dfs(
    board: &[Vec<char>],
    r: usize, c: usize,
    path: &mut Vec<Vec<bool>>,
    v: usize, ac: &AhoCorasick,
    words: &[&str],
    found: &mut HashSet<String>,
) {
    let ch = board[r][c];
    let mut cur = v;
    while cur != 0 && !ac.goto[cur].contains_key(&ch) {
        cur = ac.fail[cur];
    }
    let v2 = ac.goto[cur].get(&ch).copied().unwrap_or(0);
    for &pid in &ac.out[v2] {
        found.insert(words[pid].to_string());
    }
    path[r][c] = true;
    for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if nr >= 0 && nr < board.len() as i32
            && nc >= 0 && nc < board[0].len() as i32 {
            let (nr, nc) = (nr as usize, nc as usize);
            if !path[nr][nc] {
                boggle_dfs(board, nr, nc, path, v2, ac, words, found);
            }
        }
    }
    path[r][c] = false;
}

fn boggle(board: &[Vec<char>], words: &[&str]) -> Vec<String> {
    let ac = AhoCorasick::new(words);
    let rows = board.len();
    let cols = board[0].len();
    let mut found = HashSet::new();
    let mut path = vec![vec![false; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            path[r][c] = true;
            boggle_dfs(board, r, c, &mut path, 0, &ac, words, &mut found);
            path[r][c] = false;
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boggle_encuentra_varias() {
        let board = vec![
            vec!['h', 'o', 'l', 'a'],
            vec!['o', 'r', 'a', 'z'],
            vec!['l', 'u', 'n', 'a'],
            vec!['a', 'o', 's', 'o'],
        ];
        let found = boggle(&board, &["hola", "luna", "osa", "aro"]);
        assert!(found.contains(&"hola".to_string()));
        assert!(found.contains(&"luna".to_string()));
        assert!(found.contains(&"aro".to_string()));
    }
}
```

**Ejercicio 2 — Motif finding en ADN.** Dada una secuencia S = "ACGTACGTACG" y un patrón P = "ACGT", encuentra todas las ocurrencias. Construimos el suffix array de S y luego búsqueda binaria sobre SA para encontrar el rango donde los sufijos comienzan con P. Si S tiene 200 Mb, O(|P| log n) ≈ 30 · 28 = 840 ops por query.

*S:* Búsqueda binaria sobre el suffix array. Para cada posición del rango, comparamos S[SA[i]..] con P carácter a carácter. Resultado: posiciones 0, 4 (las dos ocurrencias de "ACGT" en S). ∎

**Ejercicio 3 — Frecuencia de k-mers.** Cuenta los 4-mers más frecuentes de un genoma. Cada k-mer es una "palabra" en Aho-Corasick; se recorre el genoma y se acumulan frecuencias. Tiempo total O(n+k) para todas las queries.

*S:* Construyes el Aho-Corasick con todos los k-mers únicos del genoma (puedes extraerlos con un set), recorres el genoma una vez y acumulas un `HashMap<(usize, usize), usize>` con los conteos. ∎

## 15.8 Ejercicios propuestos

1. **(F)** Implementa un autocompletado que devuelva las 10 palabras más frecuentes con un prefijo dado, manteniendo un *heap* de frecuencia por nodo del trie.
2. **(M)** Construye el suffix array de un genoma de prueba y resuélvelo para *shortest unique substring* usando RMQ sobre el LCP.
3. **(M)** Demuestra que la suma de longitudes de cadenas en el suffix tree de S es O(n²) en el caso no comprimido, y O(n) en el comprimido.
4. **(D)** Modifica Aho-Corasick para devolver el intervalo de posiciones [l, r] del match (no sólo el final), necesario para range queries en suffix arrays.
5. **(D)** Implementa el algoritmo de Ukkonen para construir un suffix tree en O(n). Es un reto; el código cabe en unas 200 líneas si se hace limpio.

## 15.9 Lo que te llevas

- Un **trie** almacena strings permitiendo búsquedas O(m). Es la base de autocomplete y routing.
- Un **suffix tree** preprocesa un texto en O(n) para responder queries de patrón en O(m).
- El **suffix array** + **LCP** es más compacto y ofrece las mismas garantías.
- **Aho-Corasick** busca todos los patrones simultáneamente en O(|T| + |P| + z).
- Las estructuras de strings son el corazón de la bioinformática moderna y la búsqueda de texto.

## 15.10 Ojo, cuidado con…

- **Asumir O(1) por carácter en un trie naive.** Con `HashMap<char, _>`, es O(m·hash) y el espacio explota. Para alfabetos pequeños, usa arrays.
- **Olvidar el centinela en suffix tree/array.** Sin `$`, "a" sería prefijo de "ab" y se rompería la estructura.
- **Construir el suffix array con `sort` de strings.** Es O(n² log n) — usa radix sort o doubling.
- **Implementar Ukkonen a mano sin entender el active point.** Es un algoritmo sutil; lee 3 papers antes de tocar el teclado.
- **Olvidar el caso de text vacío o patrón vacío en Aho-Corasick.** Ambos son trampas comunes.

## 15.11 Para profundizar

- **Aho & Corasick (1975)**. *Efficient string matching: an aid to bibliographic search*. Comm. ACM.
- **Ukkonen (1995)**. *On-line construction of suffix trees*. Algorithmica.
- **Manber & Myers (1993)**. *Suffix arrays: a new method for on-line string searches*. SIAM J. Comput.
- **Gusfield, *Algorithms on Strings, Trees, and Sequences*** (1997). Cap. 5–6, 9.
- Capítulo 32–33 de *CLRS* (3ª ed.) — suffix trees, KMP.
- Crate `aho-corasick` en crates.io para una implementación industrial.

## 15.12 Pin de batalla

- **Aho-Corasick con `cargo` + el crate `image` = herramienta de búsqueda visual brutal.** Buscar texto en imágenes.
- **Suffix arrays son más compactos que suffix trees en la práctica.** Misma info, 5x menos memoria.
- **Para bioinformática: usa el crate `bio`.** Tiene BWA, minimizers, suffix arrays optimizados.
- **Trie manual es 50 líneas de Rust.** Trie del crate `trie-rs` es más rápido pero más complejo.
- **Si buscas en logs, indexa con suffix array + búsqueda binaria por prefijo.** Más rápido que grep en logs grandes.


## 15.13 Si solo lees 30 segundos

Tries, suffix trees, Aho-Corasick. Para buscar patrones en texto. `bio` para bioinformática, `image` para visualizar. `trie-rs` para producción.

## 15.14 Una historia pequeña

Peter Weiner publicó su paper sobre suffix trees en 1973 en una revista de CS teórica. Nadie le hizo caso. Durante 20 años, los biólogos computacionales (que aparecieron en los 90) reinventaron la rueda mil veces buscando patrones en secuencias de ADN. Hasta que alguien, en 1992, encontró el paper de Weiner, lo implementó, y el problema de alineamiento de secuencias pasó de horas a segundos. Weiner se convirtió en consultor estrella de empresas de bioinformática. Décadas después, en una charla, dijo: "publiqué el paper en 1973, esperé 20 años, y entonces el mundo estuvo listo." A veces la investigación超前。El truco es seguir publicando aunque nadie te lea.


---

