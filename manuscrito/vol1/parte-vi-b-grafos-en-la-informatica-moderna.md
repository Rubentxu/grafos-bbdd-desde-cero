# Parte VI-B — Grafos en la Informática Moderna

> *"El internet no es más que un grafo que aprendió a mandar paquetes de fotos de gatos."*
> —Dicho popular entre ingenieros de redes, atribuido a varios y a ninguno en particular

Bienvenido a la Parte VI-B. Has recorrido un camino largo: BFS, DFS, Dijkstra, MST, flujo máximo, coloración, GNN… Ya tienes las herramientas. Ahora viene la pregunta interesante: **¿dónde se usan estos grafos en el mundo real?** En las próximas páginas, vamos a asomarnos por la ventana de tres dominios donde los grafos son el lenguaje nativo, no una herramienta más: las **redes de computadores** (que literalmente *son* grafos), los **sistemas distribuidos** (que coordinan grafos de nodos que no se fían unos de otros) y la **seguridad informática** (donde modelamos cómo un atacante compromete un sistema paso a paso).

La particularidad de esta Parte es que cada capítulo tiene su propio "ritmo". En redes, los grafos aparecen como topología física, lógica y de routing. En distribuidos, los grafos aparecen como grafos de eventos, anillos de consenso y tablas hash distribuidas. En seguridad, como cadenas de exploits y grafos de permisos. Lo que une todo es una idea: **un grafo es la forma más natural de modelar relaciones entre entidades que se envían mensajes**.

Una nota antes de empezar: el código Rust de estos capítulos es **didáctico**, no production-grade. Los protocolos reales (OSPF, BGP, Raft, etc.) son bestias de miles de líneas con décadas de optimizaciones, edge cases y RFCs. Aquí programamos el *alma* del algoritmo, lo que el profesor de redes te dibujaría en la pizarra. Si algún día tienes que mirar una implementación de OSPF de verdad, lo harás con los ojos limpios.

Personajes que nos acompañarán en esta Parte:

- **Roberto el Router** — el protagonista del Cap. 24. Lleva una corbata de cables, habla con un marcado acento de capa 3.
- **REX el Raft** — el protagonista del Cap. 25. Un búfalo tranquilo, replicado tres veces, que nunca se contradice a sí mismo.
- **Vicky la Vulnerabilidad** — la protagonista del Cap. 26. Sonríe mucho, siempre encuentra la puerta trasera.

---

