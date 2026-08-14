# Capítulo 31 — Grafos en Sistemas de Recomendación

Has visto mil veces esa fila de "porque también te podría gustar". Aparece en la pantalla con una seguridad que parece magia.
Detrás de esa fila hay un grafo, una factorización de matrices, y un pequeño acto de fe estadística.
En este capítulo vas a desmontar el truco y, ya que estamos, construir tu propio recomendador.

## 31.0 La anécdota de la esquina

En 1992, un grupo de Xerox PARC en California tuvo un problema muy terrenal: demasiados emails. Los investigadores se suscribían a listas, reenviaban, filtraban, y al final nadie encontraba nada. El grupo de Goldberg, Nichols y Oki decidió entonces construir un sistema que aprendiera de las anotaciones manuales de los usuarios. Lo llamaron *Tapestry*, y era esencialmente esto: "si Ana y tú etiquetasteis los mismos mensajes como 'interesante', entonces te podría gustar lo que Ana etiquetó como 'interesante' y tú todavía no has visto".

El método era un poco cutre: cada usuario tenía que marcar sus emails a mano, y luego *escribir consultas* sobre qué le interesaba. No había nada automático. Pero el principio — *la gente que coincide contigo en el pasado probablemente coincida contigo en el futuro* — prendió, y se convirtió en el corazón del filtrado colaborativo moderno.

Moral de la anécdota: el primer sistema de recomendación del mundo no usaba matrices ni embeddings. Usaba consultas, dedos, y un grafo social de "usuarios como yo". A veces, lo más simple es lo que se queda.

## 31.1 La matriz usuarios × items: el grafo hecho tabla

El truco mental es ver los datos de rating como un *grafo bipartito* entre dos conjuntos: usuarios e items. Cada arista lleva un peso: el rating.

```
  Ana --5-->  Peli_A
  Ana --3-->  Peli_B
  Ana --?-->  Peli_C       <-- aquí queremos predecir
  Beto --4-->  Peli_A
  Beto --5-->  Peli_C
  Beto --2-->  Peli_D
  Ceci --5-->  Peli_B
  Ceci --4-->  Peli_C
```

Visualmente, parece una tabla:

```
            Peli_A   Peli_B   Peli_C   Peli_D
   Ana         5        3        ?        -
   Beto        4        -        5        2
   Ceci        -        5        4        -
```

El signo `?` es exactamente lo que el sistema de recomendación intenta rellenar. Y esa tabla, si la miras con los ojos de un grafo, es una matriz de adyacencia de un grafo bipartito ponderado. Ya sabes sumar, multiplicar y factorizar matrices — te lo recordaré con cariño en §31.3.

> **Regla de tres + inesperado.** Para recomendar: **(1)** modelas usuarios e items como nodos, **(2)** los ratings como aristas ponderadas, **(3)** predices los huecos. Lo inesperado: el grafo es *enorme* y la matriz es *casi toda ceros* (sparse). Una tabla de 10 millones de usuarios y 1 millón de productos tiene 10^13 celdas, pero menos del 0.01% están llenas. La vida de un sistema de recomendación es vivir entre los huecos.

## 31.2 Filtrado colaborativo: vecindad e ideas

Hay dos grandes familias de filtrado colaborativo:

1. **Basado en vecindad (memory-based).** Para un usuario, encuentra los K usuarios más parecidos. Predice su rating sobre un item como la media ponderada de los ratings de esos vecinos. La "parecido" se mide típicamente con correlación de Pearson o coseno.
2. **Basado en modelo (model-based).** Aprende parámetros globales que expliquen los ratings observados. Aquí entran matrix factorization, factorization machines, redes neuronales.

Ambas son grafos en el fondo. En la primera, recorres el grafo social "usuarios similares" (un K-NN sobre usuarios). En la segunda, navegas un grafo implícito en un espacio latente: usuarios e items son puntos, y las recomendaciones son los items más cercanos a un usuario.

La primera es la más intuitiva: si te gustó lo mismo que a Beto, mira qué más le gustó a Beto. La segunda es la más escalable: en Netflix, donde hay cientos de millones de ratings, entrenar un modelo global es la única opción realista.

## 31.3 Matrix factorization: el truco del embedding

La idea, popularizada por Simon Funk durante el Netflix Prize, es la siguiente: cada usuario `u` y cada item `i` se representan por vectores `p_u` y `q_i` en un espacio de dimensión `k` (típicamente, entre 50 y 200). El rating predicho es el producto punto:

```
  r̂(u, i) = p_u · q_i
```

Visualmente, los vectores son puntos en un espacio latente:

```
  Espacio latente (k=2):

            q_Peli_C
              *
             /
            /  ← ¿qué tan cerca está p_Ana?
     p_Ana *------* q_Peli_A
            \
             \
              * q_Peli_B
```

El aprendizaje consiste en encontrar `p_u` y `q_i` que minimicen el error cuadrático sobre los ratings observados, con regularización L2 para evitar el overfitting. Esto se entrena con descenso de gradiente o, mejor, con ALS (Alternating Least Squares) si la matriz es densa.

El grafo implícito es éste: si dos usuarios tienen vectores cercanos, son similares. Si dos items tienen vectores cercanos, son parecidos. Las recomendaciones son aristas en este grafo latente.

## 31.4 PageRank con restart: random walks para recomendar

Otra forma de mirar el problema: haz un random walk por el grafo bipartito usuarios-items, pero con *probabilidad de reinicio*. Cada paso, con probabilidad `α` vuelves al usuario original. La distribución estacionaria te dice qué items son los más relevantes para ese usuario.

```
  Ana → Peli_A ← Beto → Peli_C
        ↑   ↓
        Peli_B ← Ceci
```

El PageRank personalizado (PPR, *Personalized PageRank*) es exactamente esto, y se usa en producción en sitios como Pinterest o Twitter. La gracia es que no necesitas ratings explícitos: las aristas pueden ser "el usuario tocó este item", "lo guardó", "lo vio durante 30 segundos". Cualquier señal de interacción basta.

En Rust, calcular el PPR aproximado se hace con un muestreo de walks:

```rust
use rand::Rng;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

pub fn ppr<R: Rng>(
    graph: &DiGraph<&str, f32>,
    start: petgraph::graph::NodeIndex,
    alpha: f32,
    steps: usize,
    rng: &mut R,
) -> HashMap<petgraph::graph::NodeIndex, f32> {
    let mut visits: HashMap<_, f32> = HashMap::new();
    let mut current = start;
    for _ in 0..steps {
        *visits.entry(current).or_insert(0.0) += 1.0;
        if rng.gen::<f32>() < alpha {
            current = start;
        } else {
            let succs: Vec<_> = graph.neighbors(current).collect();
            if succs.is_empty() {
                current = start;
            } else {
                current = succs[rng.gen_range(0..succs.len())];
            }
        }
    }
    let total = visits.values().sum::<f32>();
    visits.values_mut().for_each(|v| *v /= total);
    visits
}
```

## 31.5 Cold start: el invitado inesperado

Llega un usuario nuevo, sin un solo rating, sin un solo click. ¿Qué recomiendas? El sistema no sabe nada de él. Esto se llama *cold start* y es, junto con la escalabilidad, el problema más citado de los sistemas de recomendación.

Tres estrategias estándar:

1. **Recomendación por contenido.** Usa los metadatos: el usuario acaba de registrarse y dice que le gusta la ciencia ficción. Recomienda los items de ciencia ficción mejor valorados por la población general.
2. **Exploración forzada.** Muestra un set diverso y aleatorio al principio. Aprende de los clicks.
3. **Preguntar directamente.** Pide al usuario que valore 10 items al registrarse. Es lo que hace Netflix en su onboarding.

El cold start es, en el fondo, un problema de grafo incompleto. Tienes un nodo nuevo y ninguna arista. Tu trabajo es decidir qué aristas *crees* que debería tener.

## 31.6 Sesgo de popularidad: recomendar lo popular no es lo mejor

El sistema más tonto del mundo recomendaría siempre el item más popular. "¿Qué es lo que más le gusta a la gente? Lo más popular. Recomiendo eso". En términos de accuracy, este sistema es un baseline muy difícil de superar. En términos de utilidad para el usuario, es un desastre.

El sesgo de popularidad significa que tus recomendaciones terminan siendo todas iguales, todos los usuarios ven el mismo top 10, y los items de cola larga nunca son descubiertos. Para romper el sesgo, hay técnicas de *inverse propensity scoring*, *diversificación* y *coverage-aware ranking*.

Moraleja: la métrica que elijas define el sistema. Si mides accuracy, te quedas con lo popular. Si mides *catalogue coverage*, te ves forzado a explorar.

## 31.7 A/B testing de recomendadores: el juez final

Un sistema de recomendación no se evalúa en un notebook. Se evalúa *en producción*, comparando dos versiones: el modelo A y el modelo B, cada uno con 50% del tráfico. Mides clicks, conversiones, tiempo en página. El que gana, se queda.

Las métricas típicas son:

- **CTR** (click-through rate): fracción de recomendaciones que reciben click.
- **MAP@K** (Mean Average Precision): calidad del ranking top-K.
- **NDCG** (Normalized Discounted Cumulative Gain): premia los aciertos en las primeras posiciones.
- **Diversity**: variedad de los items recomendados.
- **Coverage**: fracción del catálogo que aparece en alguna recomendación.

El A/B testing es, en cierto sentido, un *experimento controlado sobre un grafo*: dos tratamientos aplicados al mismo conjunto de nodos, midiendo el efecto en las aristas que se crean (clicks, conversiones).

## 31.8 Implementación Rust: mini recomendador con matrix factorization

Vamos a construir un recomendador minimalista. Representamos usuarios e items en un espacio latente de dimensión `k`. Entrenamos con descenso de gradiente. Evaluamos con MAP@K.

```rust
use ndarray::{Array1, Array2};
use rand::Rng;
use std::collections::HashMap;

/// Observación: un usuario calificó un item con un número.
#[derive(Debug, Clone)]
pub struct Rating {
    pub user: usize,
    pub item: usize,
    pub value: f32,
}

pub struct MatrixFactorization {
    pub n_users: usize,
    pub n_items: usize,
    pub k: usize,
    pub p: Array2<f32>,       // embeddings de usuarios  (n_users × k)
    pub q: Array2<f32>,       // embeddings de items    (n_items × k)
    pub bu: Array1<f32>,      // bias de usuario
    pub bi: Array1<f32>,      // bias de item
    pub global_mean: f32,
}

impl MatrixFactorization {
    pub fn new(n_users: usize, n_items: usize, k: usize) -> Self {
        Self {
            n_users, n_items, k,
            p: Array2::zeros((n_users, k)),
            q: Array2::zeros((n_items, k)),
            bu: Array1::zeros(n_users),
            bi: Array1::zeros(n_items),
            global_mean: 0.0,
        }
    }

    pub fn predict(&self, u: usize, i: usize) -> f32 {
        let pu = self.p.row(u);
        let qi = self.q.row(i);
        pu.dot(&qi) + self.bu[u] + self.bi[i] + self.global_mean
    }

    /// Entrenamiento con SGD y regularización L2.
    pub fn fit(&mut self, ratings: &[Rating], epochs: usize, lr: f32, reg: f32) {
        self.global_mean = ratings.iter().map(|r| r.value).sum::<f32>() / ratings.len() as f32;

        let mut rng = rand::thread_rng();
        // Inicialización aleatoria pequeña.
        for elem in self.p.iter_mut() { *elem = rng.gen_range(-0.05..0.05); }
        for elem in self.q.iter_mut() { *elem = rng.gen_range(-0.05..0.05); }

        for _ in 0..epochs {
            for r in ratings {
                let u = r.user; let i = r.item; let v = r.value;
                let pred = self.predict(u, i);
                let err = v - pred;

                // Actualización de biases.
                self.bu[u] += lr * (err - reg * self.bu[u]);
                self.bi[i] += lr * (err - reg * self.bi[i]);

                // Actualización de factores latentes.
                let pu = self.p.row(u).to_owned();
                let qi = self.q.row(i).to_owned();
                for f in 0..self.k {
                    self.p[[u, f]] += lr * (err * qi[f] - reg * pu[f]);
                    self.q[[i, f]] += lr * (err * pu[f] - reg * qi[f]);
                }
            }
        }
    }

    /// Top-K items para un usuario, excluyendo los ya valorados.
    pub fn recommend(&self, u: usize, already: &[usize], k: usize) -> Vec<(usize, f32)> {
        let mut scores: Vec<_> = (0..self.n_items)
            .filter(|i| !already.contains(i))
            .map(|i| (i, self.predict(u, i)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(k).collect()
    }
}

/// MAP@K: Mean Average Precision at K.
pub fn map_at_k(
    model: &MatrixFactorization,
    test: &[Rating],
    k: usize,
) -> f32 {
    // Agrupamos ratings por usuario.
    let mut by_user: HashMap<usize, Vec<&Rating>> = HashMap::new();
    for r in test { by_user.entry(r.user).or_default().push(r); }

    let mut aps = Vec::new();
    for (u, rat) in by_user.iter() {
        let positives: std::collections::HashSet<_> = rat.iter().map(|r| r.item).collect();
        let recs: Vec<_> = model.recommend(*u, &[], k);
        let mut hits = 0;
        let mut ap = 0.0;
        for (rank, (i, _)) in recs.iter().enumerate() {
            if positives.contains(i) {
                hits += 1;
                ap += hits as f32 / (rank as f32 + 1.0);
            }
        }
        aps.push(ap / hits.max(1) as f32);
    }
    aps.iter().sum::<f32>() / aps.len() as f32
}

fn main() {
    // Mini-dataset: 5 usuarios, 6 películas, ratings dispersos.
    let ratings = vec![
        Rating { user: 0, item: 0, value: 5.0 },
        Rating { user: 0, item: 1, value: 3.0 },
        Rating { user: 0, item: 2, value: 4.0 },
        Rating { user: 1, item: 0, value: 4.0 },
        Rating { user: 1, item: 2, value: 5.0 },
        Rating { user: 1, item: 3, value: 2.0 },
        Rating { user: 2, item: 1, value: 5.0 },
        Rating { user: 2, item: 2, value: 4.0 },
        Rating { user: 2, item: 4, value: 3.0 },
        Rating { user: 3, item: 0, value: 1.0 },
        Rating { user: 3, item: 3, value: 4.0 },
        Rating { user: 3, item: 5, value: 5.0 },
        Rating { user: 4, item: 2, value: 2.0 },
        Rating { user: 4, item: 4, value: 5.0 },
        Rating { user: 4, item: 5, value: 4.0 },
    ];

    let train: Vec<Rating> = ratings.iter().take(12).cloned().collect();
    let test:  Vec<Rating> = ratings.iter().skip(12).cloned().collect();

    let mut model = MatrixFactorization::new(5, 6, 4);
    model.fit(&train, 200, 0.02, 0.05);

    println!("Recomendaciones para el usuario 0: {:?}", model.recommend(0, &[0, 1, 2], 3));
    println!("MAP@3 = {:.3}", map_at_k(&model, &test, 3));
}
```

El código es un boceto, no un sistema de producción. Pero contiene los tres ingredientes mágicos: la factorización, el entrenamiento, y la evaluación. Lo demás es escala.

## 31.9 Diálogo de ascensor

> —¿Y si en vez de ratings usamos los clicks como señal?
> —Funciona, pero los clicks tienen mucho ruido. La gente clickea por accidente, por aburrimiento, por salir del paso. Los ratings son más limpios, pero más escasos.
> —¿Y los dos a la vez?
> —Eso es lo que hacen los modelos modernos. Pesan señales según la confianza que tengas en cada una. Es un grafo con aristas de colores: rojo para ratings, azul para clicks, verde para tiempo-en-página.
> —Me gusta. ¿Y el cold start?
> —Ahí no hay grafo. Hay humo. Y tienes que decidir cuánto humo te fías.

## 31.10 Ejercicios resueltos

**Ejercicio 31.1.** Calcula, sobre la mini-tabla del §31.1, la predicción de rating de Ana sobre Peli_C usando el filtro colaborativo user-based con K=2 (vecinos más cercanos por coseno). ¿Qué predice Beto (su vecino más cercano)? ¿Y Ceci?

*Solución.* Calculamos la similitud coseno entre los vectores de ratings. Ana=[5,3,?,−], Beto=[4,−,5,2], Ceci=[−,5,4,−]. Coseno Ana·Beto ≈ 0.85, Ana·Ceci ≈ 0.91. Los dos vecinos más cercanos son Ceci y Beto. La predicción de Ana sobre Peli_C es la media ponderada: (0.91·4 + 0.85·5) / (0.91+0.85) ≈ 4.46.

**Ejercicio 31.2.** En el mismo dataset, entrena una matrix factorization con k=2 durante 50 epochs. Predice los huecos.

*Solución.* Se ejecuta el código de §31.8 con los datos de ejemplo. Los embeddings convergen y la predicción para Ana sobre Peli_C suele caer entre 3.8 y 4.4 dependiendo de la inicialización aleatoria.

**Ejercicio 31.3.** Calcula MAP@2 sobre el conjunto de test. ¿Cuántos hits consigues?

*Solución.* La métrica MAP@K exige que los items relevantes aparezcan en las K primeras posiciones. Con un modelo bien entrenado, el MAP@2 sobre este dataset minúsculo debería estar entre 0.4 y 0.7. Con un modelo mal entrenado, cercano a 0.

## 31.11 Ejercicios propuestos

1. **Bias de popularidad.** Implementa un recomendador "tonto" que siempre devuelve los K items más populares. Compara su MAP@K con el de matrix factorization. ¿Cuánto pierde?

2. **Cold start con contenido.** Añade un vector de metadatos por item (género, año, director). Para usuarios nuevos, recomienda los items más cercanos a sus preferencias declaradas.

3. **Personalized PageRank.** Implementa PPR sobre el grafo bipartito usuarios-items y compara con matrix factorization en términos de MAP@K.

4. **Diversificación.** Modifica el recomendador para que la lista de K recomendaciones tenga items distintos entre sí (MMR — Maximal Marginal Relevance).

5. **Cobertura de catálogo.** Mide la fracción de items distintos que aparecen en las recomendaciones de 1000 usuarios. Compara MF vs. popularidad pura.

## 31.12 Pin de batalla

- **La métrica es el sistema.** Si mides accuracy, te quedas con lo popular. Mide lo que de verdad te importa: cobertura, diversidad, conversión.
- **Los embeddings no son la verdad.** Son una compresión con pérdida. Dos usuarios similares pueden tener embeddings cercanos por motivos distintos. Inspecciona siempre.
- **El grafo es sparse.** Usa estructuras sparse-aware. En Rust, `ndarray` con vistas sparse o `sprs` para matrices dispersas.
- **El cold start es un problema de producto, no sólo de algoritmo.** A veces la mejor solución es pedirle al usuario que valore 10 cosas al registrarse.
- **Nunca evalúes offline sin A/B testing en producción.** Hay una diferencia abismal entre "el modelo predice bien en el dataset" y "el modelo da más ingresos en producción".

## 31.13 Lo que te llevas

- Los sistemas de recomendación viven en grafos bipartitos ponderados.
- La matrix factorization convierte el problema en geometría: recomendar es encontrar los puntos más cercanos en un espacio latente.
- El PageRank personalizado es otra forma de mirar el mismo problema: navegación aleatoria con reinicio.
- El cold start y el sesgo de popularidad son los dos demonios del dominio.
- Medir bien es la mitad del trabajo.

## 31.14 Ojo, cuidado con…

…el *feedback loop*. Un recomendador decide qué ve el usuario. El usuario consume o ignora. Eso entrena la siguiente versión del recomendador. El sistema aprende a recomendarse a sí mismo, y las burbujas de filtro se refuerzan. No es un bug; es una propiedad emergente del bucle. Romperlo requiere intervención deliberada: exploración forzada, diversificación, y diversidad de fuentes.

## 31.15 Para profundizar

- *Recommender Systems: The Textbook* de Charu Aggarwal. Completo y riguroso.
- *Mining of Massive Datasets* (capítulo 9) de Leskovec, Rajaraman y Ullman. Gratis online.
- LightFM de Maciej Kula — implementa hybrid matrix factorization.
- Implicit — librería de Ben Frederickson para feedback implícito (clicks, vistas).

## 31.16 Si solo lees 30 segundos

Los ratings son aristas. Los usuarios e items son nodos. Predecir un rating es predecir el peso de una arista que falta. Matrix factorization es geometría: convierte usuarios e items en puntos y predice por distancia. PageRank con restart es navegación aleatoria. El resto es escala y métricas.

## 31.17 Una historia pequeña

Bruno llevaba seis meses construyendo un recomendador para una tienda de música online. Los modelos iban bien en el offline. El director le pidió lanzarlo. Semana uno, todo normal. Semana dos, las ventas cayeron 8%. Bruno se volvió loco mirando logs, dashboards, métricas. Hasta que una noche, revisando los emails de soporte, leyó: "ya no encuentro los discos de jazz que compro siempre, sólo me sale reggaeton". El recomendador, entrenado con los clicks de la mayoría (jóvenes), había empujado el jazz a la cola. Para los fans de jazz, la home se había vuelto inútil. Bruno tardó dos semanas en añadir un factor de "diversidad de género" y un A/B test con cohortes explícitas. Cuando volvió a lanzar, las ventas subieron 14%. Moraleja: el offline miente si no mides la cola.

---

