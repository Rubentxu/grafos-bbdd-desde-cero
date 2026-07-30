//! Vol.I — Capítulo 15: Grafos en strings (tries, suffix arrays, Aho-Corasick).
//!
//! Migrado desde `vol1-grafos-de-cero-a-experto-rust.md` §15.1, §15.3, §15.4.
//!
//! - [`Trie`] — Árbol de prefijos para almacenar palabras y autocompletar.
//! - [`build_suffix_array`] — Array de sufijos ordenados lexicográficamente.
//! - [`build_lcp`] — Longest Common Prefix array.
//! - [`AhoCorasick`] — Multi-pattern matching en O(n+m+k).
//!
//! NOTA: la parte visual con `image` + `imageproc` se omite por costo de
//! compilación (ver MIGRATION-PATTERN.md §13 sobre la política general
//! de omitir dependencias de presentación).

use std::collections::{HashMap, VecDeque};

// ─────────────────────── Trie ───────────────────────

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
    pub fn new() -> Self {
        Self::default()
    }

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
        if node.is_word {
            node.word.as_deref()
        } else {
            None
        }
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
            if n.is_word
                && let Some(w) = &n.word
            {
                out.push(w.clone());
            }
            for child in n.children.values() {
                stack.push(child);
            }
        }
        out
    }
}

// ─────────────────────── Suffix Array + LCP ───────────────────────

/// Construye el suffix array de `s` (con centinela `$` añadido).
pub fn build_suffix_array(s: &str) -> Vec<usize> {
    let mut chars: Vec<char> = s.chars().collect();
    chars.push('$');
    let n = chars.len();
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by_key(|&i| chars[i..].to_vec());
    indices
}

/// Construye el LCP array dado el suffix array.
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
        if r == 0 {
            continue;
        }
        let j = sa[r - 1];
        while i + h < n && j + h < n && chars[i + h] == chars[j + h] {
            h += 1;
        }
        lcp[r] = h;
        h = h.saturating_sub(1);
    }
    lcp
}

// ─────────────────────── Aho-Corasick ───────────────────────

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
                if !ac.goto[v].contains_key(&ch) {
                    let id = ac.goto.len();
                    ac.goto.push(HashMap::new());
                    ac.fail.push(0);
                    ac.out.push(Vec::new());
                    ac.goto[v].insert(ch, id);
                }
                v = *ac.goto[v].get(&ch).unwrap();
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
                // output(v) incluye los outputs transitivos via fail.
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
mod tests_trie {
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
        assert_eq!(
            r,
            vec!["ruby".to_string(), "rust".to_string(), "ruta".to_string()]
        );
    }
}

#[cfg(test)]
mod tests_suffix {
    use super::*;

    #[test]
    fn suffix_array_banana() {
        // "banana" tiene 6 sufijos: "banana", "anana", "nana", "ana", "na", "a"
        // ordenados: "a" (5), "ana" (3), "anana" (1), "banana" (0), "na" (4), "nana" (2)
        //
        // El algoritmo incluye el centinela `$` (índice 6) en el SA; el
        // resultado completo es de 7 elementos: [6, 5, 3, 1, 0, 4, 2].
        // El test del Vol.I ignora el centinela — corregido.
        let sa = build_suffix_array("banana");
        assert_eq!(sa, vec![6, 5, 3, 1, 0, 4, 2]);
        // Los primeros 6 elementos (excluyendo el centinela) coinciden con
        // la lista del Vol.I.
        assert_eq!(&sa[1..], &[5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn lcp_banana() {
        // LCP del Vol.I es [0, 1, 3, 0, 0, 2] (6 elementos, sin centinela).
        // Con centinela, el LCP array completo es [0, 0, 1, 3, 0, 0, 2].
        let sa = build_suffix_array("banana");
        let lcp = build_lcp("banana", &sa);
        assert_eq!(lcp, vec![0, 0, 1, 3, 0, 0, 2]);
        assert_eq!(&lcp[1..], &[0, 1, 3, 0, 0, 2]);
    }
}

#[cfg(test)]
mod tests_aho {
    use super::*;

    #[test]
    fn encuentra_todas_las_ocurrencias() {
        let ac = AhoCorasick::new(&["he", "she", "his", "hers"]);
        let matches = ac.search("ushers");
        // "she" en posición 3 (termina en índice 3), "he" en posición 3, "hers" en posición 4.
        let pats: Vec<usize> = matches.iter().map(|&(_, p)| p).collect();
        assert!(pats.contains(&0), "debe encontrar 'he'");
        assert!(pats.contains(&1), "debe encontrar 'she'");
        assert!(pats.contains(&3), "debe encontrar 'hers'");
    }

    #[test]
    fn sin_matches_devuelve_vacio() {
        let ac = AhoCorasick::new(&["xyz"]);
        assert!(ac.search("abc").is_empty());
    }

    #[test]
    fn detecta_overlaps() {
        // Aho-Corasick debe encontrar overlaps: "aba" aparece en "ababa"
        // en posiciones 0 y 2.
        let ac = AhoCorasick::new(&["aba"]);
        let matches = ac.search("ababa");
        assert_eq!(
            matches.len(),
            2,
            "esperaba 2 matches, obtuve {}",
            matches.len()
        );
    }
}
