# Colofón

**Agradecimientos** — A los investigadores cuyo trabajo este volumen explica y reimplementa conceptualmente: Semih Salihoğlu y el equipo de Kùzu (atribuidos abajo), Peter Boncz, Marcin Zukowski y Niels Nes por MonetDB/X100, Diego Ongaro y John Ousterhout por Raft, Grzegorz Malewicz y colegas por Pregel, Joseph Gonzalez y colegas por PowerGraph, y C. Mohan y colegas por ARIES. A PostgreSQL y SQLite como referentes docentes eternos: generaciones enteras aprendieron internals de bases de datos leyendo su código y sus documentos. A la comunidad de Rust, por una biblioteca estándar tan completa que permitió construir LiraDB entero sin una sola dependencia. Y a ti, lector, por construir en lugar de consumir.

**Sobre esta edición** — Edición unificada 2026 de una obra en tres volúmenes. Este Volumen II, «Construye LiraDB», queda cerrado con 40 capítulos y 5 apéndices (A-E). Todo el código está escrito en Rust (edición 2024) usando exclusivamente la biblioteca estándar, sin dependencias de runtime, y se verifica con 892 tests en verde: cada ejemplo del libro es el mismo código que se compila y se prueba.

**Versión Python** — El Vol.II tendrá una versión paralela en Python (LiraDB-py) en un repositorio hermano, compartiendo estructura y decisiones arquitectónicas.

**Licencia** — CC BY-NC-SA 4.0.

**Atribuciones** — A Semih Salihoğlu y al equipo de Kùzu —Guodong Jin, Xiyang Feng, Ziyi Chen, Chang Liu y el resto del grupo de la Universidad de Waterloo— por los papers seminales sobre GDBMS modernos, en particular «KÙZU Graph Database Management System» (CIDR 2023). La arquitectura conceptual de los caps. 37-40 del Vol.II se inspira en ese paper y en las publicaciones del grupo; la reimplementación es clean-room: ningún código de Kùzu ha sido copiado. Kùzu Inc., la empresa que comercializó la base de datos con licencia MIT, fue adquirida por Apple en octubre de 2025 y su repositorio quedó archivado; el proyecto continúa hoy en forks comunitarios como LadybugDB y bighorn. Los papers permanecen públicamente accesibles bajo sus licencias originales (CC-BY 4.0 / MIT según el caso). Texto y código de este libro están bajo CC BY-NC-SA 4.0.

**Contacto** — Errores, sugerencias y contribuciones: <https://github.com/Rubentxu/grafos-bbdd-desde-cero> (issues y PRs bienvenidos).
