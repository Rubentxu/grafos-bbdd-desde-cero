# Apéndice D — Cómo está escrito este libro: técnicas y por qué

Este libro es, además de un libro sobre grafos, un experimento de escritura técnica accesible. Antes de empezar a escribir, investigué qué hacen los libros técnicos que enganchan a todo el mundo — desde el clásico *Grokking Algorithms* de Aditya Bhargava hasta *The Pragmatic Programmer* de Hunt & Thomas. La conclusión: la mayoría de los libros técnicos aburridos lo son por cinco razones, todas evitables. Las detallo aquí, y dónde las aplico en este libro.

## 1. "Just in time, not just in case"

**Problema**: muchos libros explican 30 conceptos en el cap. 1 "por si acaso", y el lector se ahoga antes de empezar.

**Nuestra solución**: cada capítulo introduce solo lo que se necesita **en ese momento**. Si el cap. 3 usa stacks, los explica dentro del cap. 3, no en un anexo de "estructuras de datos básicas".

**Dónde verlo**: el cap. 1 (grafos básicos) no menciona Dijkstra. El cap. 3 (BFS/DFS) introduce colas, no heaps. Los heaps llegan en el cap. 4 (Dijkstra) cuando hacen falta.

## 2. Hook + Anécdota + Visual + Humor

**Problema**: muchos libros empiezan con "En este capítulo estudiaremos los grafos, que son una estructura matemática compuesta por...". Spoiler: nadie pasa del segundo párrafo.

**Nuestra solución**: cada capítulo abre con tres elementos:

1. **Hook** (3-5 líneas): una pregunta, un escenario, una afirmación provocadora.
2. **Anécdota de la esquina** (~100-180 palabras): la historia del inventor o del problema. Humaniza.
3. **Visual ASCII**: un diagrama que explica la intuición antes de cualquier fórmula.

**Dónde verlo**: el cap. 5 (MST) abre con Borůvka electrificando Moravia en 1926, no con la definición de MST. El cap. 20 (GNN) abre con Kipf en Amsterdam en 2016, no con la fórmula `H' = σ(Ã·H·W)`.

## 3. Regla de tres + humor inesperado

**Problema**: el humor aleatorio aburre. El humor ausente aburre más. El humor sarcasmo echa para atrás.

**Nuestra solución**: el humor aparece con la **regla de tres** (dos cosas normales + tercera absurda) y con **auto-ironía** (reírse del autor, del campo, del código). Nunca del lector.

**Dónde verlo**: "Hay tres tipos de gente: los que hacen backup, los que todavía no han perdido datos importantes, y los que viven al límite." (cap. 31)

## 4. Personajes + historias pequeñas + diálogos

**Problema**: los conceptos abstractos no se quedan. Si el lector no asocia el algoritmo a un nombre humano, lo olvidará en una semana.

**Nuestra solución**: cada capítulo termina con un **mini-relato** donde un personaje ficticio aplica el concepto. Y dentro del capítulo, **diálogos cortos** entre personajes inventados que muestran el concepto en acción.

**Dónde verlo**: "Roberto el Router" en el cap. 24. "Fermín el Firewall" en el cap. 26. La historia de Marta, la junior que heredó un servidor con 200 procesos (cap. 23).

## 5. Voz activa + frases cortas + prosa escaneable

**Problema**: la prosa académica promedio tiene subordinadas de 40 palabras, voz pasiva y adverbios. Es agotador de leer.

**Nuestra solución**: voz activa, frases cortas, bullets/listas/bold para escaneo rápido. El libro está pensado para leerse en una pantalla, en metro, en 15 minutos por capítulo.

**Dónde verlo**: en todas partes. Nunca "será realizado por el algoritmo", siempre "el algoritmo lo hace".

## 6. Ilustraciones ASCII más que fórmulas

**Problema**: una fórmula $O(V \log V + E)$ impresa en un párrafo se pierde. El lector pasa de largo.

**Nuestra solución**: cuando un concepto tiene estructura visual (un grafo, una pila, un árbol), lo **dibujamos en ASCII** antes de la fórmula. El dibujo queda en la memoria; la fórmula se olvida.

**Dónde verlo**: el cap. 15 (suffix trees) tiene un dibujo ASCII paso a paso de la construcción de Ukkonen. El cap. 23 (deadlock) tiene un RAG visualizado con caracteres Unicode.

## 7. Pin de batalla + Si solo lees 30 segundos

**Problema**: la teoría se olvida; los tips prácticos del mundo real se quedan.

**Nuestra solución**: cada capítulo cierra con dos secciones rápidas:
- **"Pin de batalla"**: 3-5 tips prácticos aprendidos con sangre. Cosas que solo sabes después de pegarte con el código.
- **"Si solo lees 30 segundos"**: 1-2 frases finales que destilan la idea principal. Como si le explicaras a tu madre en 30 segundos.

**Dónde verlo**: todos los capítulos de la Parte VI tienen ambas secciones. Ejemplo del cap. 23: "Si ves un deadlock solo en producción los viernes a las 3am, mira el cron, no la aplicación."

## 8. Honestidad histórica

**Problema**: muchos libros atribuyen inventos a quien los popularizó, no a quien los creó. (El "algoritmo húngaro" no es húngaro; es soviético-alemán-japonés-estadounidense con varias paternidades.)

**Nuestra solución**: cuando la historia es controvertida, lo decimos. Atribuimos a quien lo inventó, citando también a quien lo popularizó.

**Dónde verlo**: cap. 8 (matching bipartito), la anécdota del "algoritmo húngaro" explica la injusticia de la nomenclatura. Cap. 12 (flujo de costo mínimo) menciona a Kantorovich, Koopmans, y la paternidad compartida del Nobel de Economía 1975.

## 9. Recursos para profundizar (no para abrumar)

**Problema**: las bibliografías tradicionales son largas y poco útiles. El lector no sabe por dónde empezar.

**Nuestra solución**: cada capítulo cierra con 3-5 referencias seleccionadas, priorizando:
1. **Libros/libre acceso** (pueden descargarse).
2. **Papers seminales** con autor y año correctos.
3. **Vídeos / cursos online** cuando existen.
4. **Crates / herramientas** que el lector puede usar de inmediato.

**Dónde verlo**: el "Para profundizar" de cada capítulo.

## Lo que NO hacemos

Para ser honestos sobre el estilo, también vale la pena decir lo que **no** hacemos:

- **No usamos humor de humillación** ("¿de verdad no lo entiendes? Eres tonto"). Jamás.
- **No usamos sarcasmo**. La mayoría de los lectores lo lee mal en texto.
- **No usamos referencias temporales absolutas** ("en 2017 se publicó..."). El libro envejece mal con eso. Preferimos "un investigador holandés publicó...".
- **No comprimimos la ironía histórica**. Si la historia del algoritmo es interesante, la contamos.
- **No asumimos que el lector sabe Rust**. Si no lo sabe, los snippets se pueden saltar sin perder la idea.
- **No usamos jerga innecesaria**. Si podemos decir "camino más corto" sin decir "shortest path", lo decimos. Pero si el término inglés es estándar (BFS, NP-hard), lo mantenemos porque es el que vas a encontrar en papers.

## Si quieres contribuir

¿Encontraste un error? ¿Tienes una anécdota jugosa? ¿Quieres proponer un capítulo nuevo?

- **Errores y erratas**: envía un patch. Indica capítulo, sección, frase exacta.
- **Anécdotas**: si tienes una buena historia sobre un grafo o un algoritmo que no esté aquí, mándala. Las mejores se añaden a la siguiente edición.
- **Nuevos capítulos**: si eres experto en un foco CS no cubierto (criptografía, sistemas de archivos, hardware, etc.), escríbelo en el mismo estilo y mándalo.

---

