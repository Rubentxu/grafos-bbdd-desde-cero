# CONTRATO DE CAPÍTULO — Vol.II Cap. 38: Almacenamiento columnar y ejecución vectorizada

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. **Segundo capítulo de la Parte VIII**
> (crecimiento POST-mapa), recoge el gancho explícito del cap. 37: «la lista dijo QUÉ falta;
> ahora esta Parte construye DÓNDE crecer — la primera apuesta técnica sale de una medida
> propia: ×16 CSR vs puerto». Código ancla: `lib.rs` declara 29 módulos (`cap07_modelo` …
> `cap37_produccion`; el cap. 36 fue síntesis sin módulo, solo `tests/arquitectura.rs`);
> `Node.props: HashMap<String, Value>` = ROW STORE actual (cap07_modelo.rs:61); tags de
> `Value` 0-5 en `cap09_encoding.rs`; esquema de 20 claves (`CLAVES_ESQUEMA`) con dominios
> pequeños medidos (32 ciudades, ~10 países, ~6 idiomas…) y `email` ÚNICO por nodo;
> `dataset_referencia(seed)` determinista 100k/500k + `dataset_referencia_mini(400/1.200)`
> del cap. 34; `BitSet::{new, marcar, contiene, unos}` y `ProyeccionPonderada::bloques_de_nodos`
> (precedente de batching, cap. 26); metodología de benches del cap. 34 (criterion 0.7,
> `harness = false`, hardware declarado Xeon E5-2682 v4 @2.50GHz). Estado verificado
> 2026-08-25: **853 tests** ALL_GREEN; `vol2-liradb` dependency-free en runtime
> (dev-deps: tempfile/proptest/criterion). Código NUEVO previsto: módulo
> `cap38_columnar.rs` (~800-1200 líneas, std puro) + TERCER `[[bench]]`
> `benches/bench_columnar.rs` (aditivo) — CERO dependencias nuevas, CERO cambios en
> módulos cap*. Toolchain pinneada 1.96.0 ⇒ `std::simd` NO disponible (nightly):
> SIMD HONESTO = auto-vectorización LLVM MEDIDA (delimitación al estilo cargo-fuzz
> del cap. 33). Hallazgo §41 que este capítulo RESPETA: Float-entero se pierde en
> JSONL (`Float(2.0)` → `"2"` → `Int(2)`) — las columnas viven EN MEMORIA, sin
> serialización, y la prosa lo declara frontera. Pregunta crítica del CORPUS
> (`vol-II-cap-38`): «SIMD, factorización» — prerrequisito ADR-001 **CUMPLIDO**
> (RESUELTA 2026-08-25): Kùzu archivada tras la adquisición por Apple (oct-2025),
> forks LadybugDB/bighorn; paper CIDR 2023 (Jin, Feng, Chen, Liu, Salihoğlu, CC-BY 4.0)
> es LA fuente primaria junto a X100 (CIDR 2005). Gancho saliente: cap. 39 WCOJ —
> «las columnas aceleran CÓMO lees; los worst-case optimal joins cambian QUÉ calculas».

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: TODO el edificio — modelo PropertyGraph con props
  `HashMap` (7), encoding/tags (9), CSR con su ×16 medido (14), LiraQL end-to-end con
  Volcano fila-a-fila (17-21), proyección materializada + `BitSet` + bloques de nodos
  (26), ACID/WAL/MVCC (27-30), CLI/import-export (31-32), torre de pruebas (33) y el
  aparato de medición completo (34): dataset determinista, criterion, percentiles,
  baselines, hardware declarado.
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**:
  (1) «columnar siempre gana» — no: para OLTP (leer UN nodo entero, escribir mucho)
  el row store manda; Abadi et al. midieron que un column-store NAIVE PIERDE contra un
  row-store bien afinado si la ejecución no se adapta (SIGMOD 2008). La elección la
  decide EL WORKLOAD.
  (2) «SIMD exige intrinsics o nightly» — no: LLVM auto-vectoriza bucles apretados sin
  ramas internas; lo que nightly añade es CONTROL (portable-simd) e intrinsics
  estabilizados. Sin nightly lo honesto es MEDIR el efecto y saber inspeccionar el
  ensamblador — no prometer instrucciones concretas.
  (3) «comprimir siempre acelera» — no: decodificar cuesta CPU; lo que hace rentable la
  compresión ligera es OPERAR SOBRE LOS CÓDIGOS comprimidos cuando el predicado lo
  permite (SIGMOD 2006), y aun así depende de la selectividad.
  (4) «un resultado de mil millones de tuplas necesita mil millones de celdas en RAM» —
  no: la REPRESENTACIÓN FACTORIZADA guarda arrays por variable más multiplicidades y
  preserva EXACTAMENTE el mismo multiconjunto lógico (CIDR 2023; Olteanu-Závodný ICDT
  2015); menos celdas físicas, cero filas perdidas.
- **Pregunta crítica que el capítulo tiene que responder**: «¿qué significa SIMD y
  factorización aquí — qué se puede demostrar HOY sin nightly y cuánto cambia la
  representación del resultado?» Respuesta en dos mitades medibles: (a) ejecución por
  lotes sobre columnas tipadas donde el bucle caliente auto-vectoriza — efecto MEDIDO
  contra el filtro fila-a-fila sobre `HashMap`, con método de verificación del
  ensamblador documentado fuera del pipeline; (b) una estructura factorizada mínima
  para una expansión 2-hop que demuestra filas lógicas vs celdas físicas con conteo
  exacto. Sin medida propia no hay capítulo: el ×16 del cap. 34 era la semilla, hoy
  florece en las propiedades.

---

## 2. Lo que el capítulo entrega (deliverables verificables)

| Deliverable | Cómo se verifica |
|---|---|
| Módulo `cap38_columnar.rs` (std puro, sin deps) con `TablaColumnar::desde_store(&MemoryStore, claves)` : extracción row→column DESDE los props del dataset — ids recolectados y ORDENADOS ascendente (determinismo pese al `HashMap` interno), columna dinámica por tipo (`Columna::Int(Vec<i64>) / Float(Vec<f64>) / String(Vec<String>) / Bool(Vec<bool>)`) + BITSET DE PRESENCIA reutilizando `BitSet` del cap. 26 para props sparse | `cargo test -p vol2-liradb --lib cap38`: tesis `columnar_tabla_desde_dataset_tipos_y_ordenes` (tipos correctos por clave, orden estable) y `columna_preserva_ausentes_con_bitset` |
| **Dictionary encoding** `Diccionario` (tabla `Vec<String>` + códigos `Vec<u32>` vía mapa interno) con ESTADÍSTICA integrada: cardinalidad, bytes antes (suma de strings) vs después (4·n + diccionario) | `diccionario_roundtrip_y_estadisticas_ciudad` (roundtrip código↔string exacto) y `diccionario_gana_en_ciudad_y_pierde_en_email` (ratio >1 en cardinalidad baja; ratio <1 en email único — la PÉRDIDA se reporta, no se oculta) |
| **Bit packing** a mano: `empaquetar(&[u32], bits) -> Vec<u64>` + `desempaquetar`, k = bits de la cardinalidad; casos conocidos exactos (p.ej. 3 bits: [5,2,7] → patrón binario fijado en el test) | `bit_packing_casos_conocidos_exactos` y `bit_packing_roundtrip_k_de_1_a_32` (todas las k, incluida k=0 trivial de un solo valor distinto) |
| **Batch execution**: `TAMANIO_VECTOR: usize = 1024`; filtro por predicado sobre columna en DOS PASADAS (máscara `Vec<bool>` apretada sin ramas internas → compactar índices), cola final manejada aparte | `filtro_por_lotes_equivale_al_filtro_fila` (TESIS: mismo multiconjunto de ids que el filtro fila-a-fila sobre props, mini dataset) y `vector_tamano_fijo_maneja_el_resto` (tamaños no múltiplos de 1024) |
| **Medición integrada** en el módulo: informe de ratios (compresión por columna, filas lógicas/celdas físicas factorizadas) imprimible para la prosa — NUNCA cifras duras en tests de CI | test `informe_columnar_reproducible_sobre_mini` (sobre `dataset_referencia_mini`: valores estables y verificables) |
| **Factorización simplificada y HONESTA** de resultados: `ResultadoFactorizado` para expansión 2-hop (arrays por variable + multiplicidad por pivote) vs tuplas planas materializadas; conteo exacto filas lógicas vs celdas físicas | `factorizacion_dos_saltos_filas_logicas_vs_celdas` (grafo pequeño conocido: cuentas esperadas FIJAS) y `factorizacion_equivale_a_la_expansion_plana` (TESIS: iterando la estructura factorizada salen las MISMAS tuplas que expandiendo plano) |
| TERCER bench `benches/bench_columnar.rs` (`[[bench]] name="bench_columnar"`, `harness=false`, ADITIVO en Cargo.toml): grupos `filtro_row_vs_columna_edad` (HashMap lookups vs lotes), `decodificacion_diccionario_ciudad`, `desempaquetado_bits` — cada uno con `Throughput::Elements` como cap. 34 | Compilación automática en `./scripts/verify.sh` (check/clippy `--all-targets`); ejecución MANUAL `cargo bench -p vol2-liradb --bench bench_columnar`; prosa pega salidas REALES con hardware declarado |
| Delimitación SIMD escrita (como cargo-fuzz en cap. 33): QUÉ prometemos (delta escalar-vs-lote MEDIDO; inspección opcional de asm vía `cargo asm`/`--emit=asm`, FUERA del pipeline) y QUÉ NO (intrinsics, portable-simd, garantías de vectorización) | prosa N.9/N.12 + bloque «lo que sí / lo que aún no»; NINGÚN paso asm en `verify.sh` |
| ALL_GREEN workspace | `./scripts/verify.sh` → ALL_GREEN (**853 + ~10 tests nuevos ≈ 863**); cero cambios en caps. 7-37, goldens intactos |

---

## 3. La pregunta crítica del CORPUS y la respuesta del capítulo

**Pregunta**: «SIMD, factorización.» (+ prerrequisito ADR-001: atribución Ladybug —
CUMPLIDO). El capítulo la convierte en **respuesta en nueve pasos** (los nueve puntos
del brief, líneas 1572-1583):

1. **Row store frente a column store** → la medida propia abre el capítulo: leer
   `edad > umbral` sobre 100k nodos va fila a fila por `props.get()` (cap. 7) — el
   layout ACTUAL de LiraDB es row store, y eso se MIDE antes de cambiar nada.
   Fuente del trade-off: Stonebraker et al., C-Store (VLDB 2005); Abadi et al.
   (SIGMOD 2008).
2. **Columnas de propiedades** → `TablaColumnar::desde_store`: extracción pura de
   LECTURA sobre `MemoryStore` (el store no se toca; hexagonal, como DiskStore en
   caps. 33-34).
3. **Compresión** → estadística bytes antes/después por columna; la lección de que la
   compresión ligera solo paga si la ejecución coopera (SIGMOD 2006).
4. **Dictionary encoding** → `Diccionario` String→u32; GANA en `ciudad/pais/idioma`
   (cardinalidad ≤ 32), PIERDE en `email` (cardinalidad n) — ambos ratios medidos.
5. **Bit packing** → empaquetador exacto k-bits sobre los códigos del diccionario;
   ratio final ciudad: u32 → ⌈log₂32⌉=5 bits.
6. **Vectores** → lotes de `TAMANIO_VECTOR=1024` (cabe en L1/L2, precedente
   `bloques_de_nodos` del cap. 26); X100 usó vectores ~100 (CIDR 2005) — la magnitud
   se justifica, no se copia.
7. **SIMD honesto** → sin nightly NO hay portable-simd: prometemos el EFECTO medido
   (delta criterion escalar-vs-lote) + cómo VERIFICAR la auto-vectorización mirando el
   ensamblador (opcional, fuera de CI). Delimitación explícita tipo cargo-fuzz (cap. 33).
8. **Batch execution** → filtro en dos pasadas (máscara + compactado) escrito para que
   LLVM pueda pipelinar; equivalencia con el filtro de fila exigida POR TEST.
9. **Factorización de resultados** → estructura mínima 2-hop (arrays+multiplicidad)
   con conteo filas lógicas/celdas físicas; conexión directa con el processor
   factorizado de Kùzu (CIDR 2023, CC-BY 4.0) y la teoría de Olteanu-Závodný (ICDT
   2015). El motor factorizado completo y WCOJ son del CAP. 39 — frontera declarada.

El hilo conductor: «el layout manda» — la misma ley que el ×16 de CSR (cap. 14),
ahora medida en PROPIEDADES; y «la representación del resultado también es layout».

---

## 4. La arquitectura: dos ejes — dónde viven los datos y en qué trozos se procesan

Modelo mental único: **layout × granularidad**. El cap. 14 organizó ADYACENCIAS por
nodo (CSR, ×16 medido); este capítulo organiza PROPIEDADES por atributo (columnas) y
PROCESA en trozos grandes (lotes) en lugar de valor a valor. Cuatro casillas:

```
                 PROCESADO valor-a-valor        PROCESADO POR LOTES (1024)
              ┌──────────────────────────────┬────────────────────────────────┐
  DATOS       │ HOY: props HashMap fila a    │ este cap.: máscara sobre lote  │
  POR FILA    │ fila (filtro fila del cap20) │ de valores extraídos           │
              ├──────────────────────────────┼────────────────────────────────┤
  DATOS       │ columna leída valor a valor  │ OBJETIVO: columna + lote +     │
  POR COLUMNA │ (mejor localidad, igual CPU) │ bucle apretado ⇒ auto-vectoriza│
              └──────────────────────────────┴────────────────────────────────┘
   (cap.14 ya vivió esto con adyacencias: puerto×1 → CSR×16)
```

Y debajo, la REGLA DE ORO de la honestidad heredada del cap. 34: todo número con
dataset determinista (`SEMILLA_REFERENCIA`), `black_box`, warm-up, repeticiones y
hardware declarado — sin calibrado, un delta es un rumor.

```text
Lo que SÍ se hace hoy:   capa de LECTURA analítica row→columna (en RAM, sobre MemoryStore)
                         ejecución por lotes con máscara; factorización de RESULTADOS 2-hop
Lo que AÚN NO:           integración en el Executor (Volcano sigue fila-a-fila)
                         paginación columnar en disco · intrinsics/portable-simd · WCOJ (cap. 39)
```

Momento ¡ajá! perseguido: «el ×16 no era magia del CSR: era la primera MEDIDA de una
ley general — el coste está en CÓMO están puestos los bytes — y hoy se vuelve a medir
en las propiedades… y hasta el RESULTADO de una consulta es un problema de layout».

---

## 5. Decisiones de diseño con alternativa descartada y fuente

| # | Decisión | Por qué | Alternativa descartada | Fuente / lección |
|---|---|---|---|---|
| 1 | Un módulo nuevo `cap38_columnar.rs`, std puro, dependency-free | Regla «primero a mano» (CONVENTIONS §4): dictionary encoding y bit packing SON enseñables con std; crate principal sigue sin deps runtime | Crate externo de compresión/columnas (arrow, parquet): deps enormes, API opaca, nada que APRENDER bit a bit | CONVENTIONS §4; misma regla que caps. 18 (lexer), 22 (heap propio), 28 (WAL) |
| 2 | Columnar como CAPA DE LECTURA analítica sobre `MemoryStore`; cero cambios en caps. previos | El row store es ÓPTIMO para el workload transaccional que LiraDB YA sirve (point lookups, writes); mezclar ambos layouts en el store rompería 853 tests y el modelo ACID construido | Reconvertir `MemoryStore` a híbrido row+column (PAX/tupla-slotted con sub-columnas): proyecto de OTRO capítulo y de otra Parte | Abadi et al., «Column-Stores vs. Row-Stores…», SIGMOD 2008 (naive columnar pierde sin ejecución adaptada); honestidad hexagonal caps. 33-34 |
| 3 | Extracción determinista: ids ordenados ascendentes; índice de columna = posición en ese orden | `MemoryStore` iterna sobre contenedores hash ⇒ orden NO estable entre runs; sin orden fijo, ni baselines ni tests comparables | Confiar en el orden de `iter_nodos`: comparaciones frágiles entre runs | Cap. 34 (determinismo = requisito de medición); Mytkowicz et al., ASPLOS 2009 |
| 4 | Presencia sparse con `BitSet` del cap. 26 reutilizado (marcar/contiene/unos) | El esquema ES sparse (props opcionales, cap. 7); BitSet denso es exactamente su caso (índices 0..n); spacing puro — la lección «cuándo gana a un hash set» se re-ejercita | `Option<T>` por celda (rompe densidad y vectorización) o `Null` centinela dentro de la columna (mezcla tipos en el bucle caliente) | Cap. 26 (`BitSet`, §ContandoStore); esquema sparse del dataset (cap. 34) |
| 5 | Dictionary encoding con estadística integrada y veredicto BIDIRECCIONAL (gana/pierde) | Enseñar CUÁNDO aplica es el objetivo wisdom: `ciudad` (≤5 bits/nodo) vs `email` (peor que plano); la pérdida es MATERIAL DIDÁCTICO | Vender el diccionario como mejora universal: mentiría con el email delante | D. Abadi, S. Madden, M. Ferreira, «Integrating Compression and Execution in Column-Store Database Systems», SIGMOD 2006 |
| 6 | Bit packing a mano con tests EXACTOS contra patrones binarios conocidos + roundtrip k=1..32 | Los bits se aprenden viéndolos; el roundtrip exhaustivo es la red ante off-by-one clásicos del packing | Solo test estadístico de ratio (no detectaría corrupción silenciosa de códigos) | SIGMOD 2006 (bit-packed codes operables sin descomprimir); lección de tests exactos del cap. 9 (encoding) |
| 7 | `TAMANIO_VECTOR=1024`, resto final manejado FUERA del bucle caliente | Vector cache-residente (1024×8B = 8 KiB, cabe en L1/L2) con cola limpia: el bucle principal queda regular y apto para pipeline/auto-vectorización | Vector gigante único (estilo MonetDB MIL): materializa columnas enteras — el fallo EXACTO que X100 diagnosticó (memory-bandwidth bound) | Boncz, Zukowski, Nes, «MonetDB/X100», CIDR 2005; precedente `bloques_de_nodos(tam)` (cap. 26) |
| 8 | Filtro en DOS PASADAS (máscara sin ramas internas → compactado) | LLVM auto-vectoriza bucles de comparación que producen máscara; el compactado con rama queda AISLADO y medible; sin nightly no hay compress-store vectorial | Rama por elemento (`if pred { push }`): divergencia que impide vectorizar el bucle caliente; intrinsics AVX/SSE: portabilidad y nightly prohibidas | Rust: `feature(portable_simd)` nightly-only (toolchain pinneada 1.96.0, rust-toolchain.toml); CIDR 2005 (primitivas vectorizadas) |
| 9 | SIMD HONESTO: promesa = DELTA MEDIDO escalar-vs-lote + inspección de asm OPCIONAL fuera del pipeline (`cargo asm` / `rustc --emit=asm`), como flamegraphs en cap. 34; PROHIBIDO prometer instrucciones concretas | Lo demostrable sin nightly es el EFECTO y el MÉTODO de verificación; prometer «usa AVX2» sería infalsificable y frágil entre CPUs | Portable-simd/intrinsics: exigen nightly y romperían la toolchain pinneada (ADR-002) | Delimitación espejo de cargo-fuzz (cap. 33, §5.x); política toolchain ADR-002; criterion Book (`black_box`) |
| 10 | Equivalencia OBLIGATORIA por test: filtro por lotes == filtro fila (multiconjunto) y factorización == expansión plana | Sin test-tesis de equivalencia, todo el capítulo sería demo de humo; la velocidad SIN corrección no es velocidad | Benchmarks sin verificación semántica: optimiza el número, corrompe el resultado | Torre de pruebas (cap. 33): cada piso con su contrato testeado |
| 11 | Factorización MÍNIMA VIABLE: estructura arrays-por-variable + multiplicidad para 2-hop, conteo lógico/físico; motor completo y joins factorizados = cap. 39 | El concepto cabe y se DEMUESTRA con conteo exacto; el motor (operadores factorizados, WCOJ) requiere el cap. 39 entero — fingirlo aquí sería inflarlo | Implementar el processor factorizado de Kùzu: alcance de libro propio; solaparía con WCOJ del cap. 39 | Jin, Feng, Chen, Liu, Salihoğlu, «KÙZU Graph DBMS», CIDR 2023 (CC-BY 4.0; ADR-001); Olteanu & Závodný, «Factorised Representations of Query Results», ICDT 2015; brief cap. 39 (líneas 1597-1598) |
| 12 | Tercer `[[bench]] bench_columnar.rs` ADITIVO + atribución Kùzu según ADR-001 (relato histórico: archivada oct-2025 tras adquisición por Apple; forks LadybugDB/bighorn; paper CIDR 2023; clean-room, cero código copiado) | Cargo integra el target gratis (verify compila, ejecuta cuando TÚ decides — regla del cap. 34); la política ADR-001 es VINCULANTE para cap. 38+ («cita CIDR 2023 y papers VLDB/SIGMOD del grupo») | Binario propio cronometrado; citar «Kùzu renombrado a Ladybug» (FALSO — error corregido por ADR-001) | Cargo Book (targets bench); ADR-001 (política completa); CONVENTIONS §5 (colofón/licencias) |

---

## 6. Estructura del manuscrito (partes y tempos)

1. **Apertura (N.0, anécdota + pregunta crítica)**: CWI Ámsterdam, 2003 — Boncz,
   Manegold y Nes escriben a mano el kernel de TPC-H Q1 como programa independiente y
   lo comparan con los motores… y el resultado «destroza nuestra imagen de MonetDB
   como cumbre del rendimiento analítico» (palabras del propio Boncz). Su prototipo
   académico celebrado ERA LENTO por diseño de ejecución: columna-entera-materializada,
   limitado por ANCHO DE BANDA DE MEMORIA. La cura fue X100: vectores pequeños
   cache-residentes (CIDR 2005). Pregunta enmarcada: ¿cuánto pierde LiraDB por SU
   layout de propiedades actual?
2. **N.1-N.2 Objetivo/Problema**: 853 tests verdes, motor ACID completo — y leer una
   propiedad de 100k nodos pasa fila a fila por `HashMap::get`. El gancho ×16 (cap.
   34) aplicado a las propiedades. Qué NO te dice la suite: cómo se comporta una
   ANALÍTICA sobre esos mismos datos.
3. **N.3 Modelo mental**: matriz layout×granularidad (§4) + pipeline
   extraer→codificar→empaquetar→lotear→filtrar→compactar + la frontera «lo que sí /
   lo que aún no».
4. **N.4 Primera solución**: la ingenua de todo el mundo — `Vec<(NodeId, Value)>`
   extraído con clon y filtrado elemento a elemento con `match` por tag (cap. 9):
   columna DISFRAZADA de fila (enum dispatch por celda, strings clonados, presencia
   perdida).
5. **N.5 Sus límites**: dispatch por elemento mata el bucle, sin compresión no hay
   menos bytes que mover, `Option` rompe la densidad, y el resultado de una expansión
   2-hop sobre un hub EXPLOTA en tuplas planas.
6. **N.6 Solución evolucionada**: columnas tipadas + BitSet presencia (cap. 26) +
   diccionario con estadística + bit packing exacto + lotes de 1024 con máscara en
   dos pasadas + factorización 2-hop con multiplicidad.
7. **N.7 Código completo ejecutable**: `cap38_columnar.rs` y `benches/bench_columnar.rs`
   referenciados por `include::` (nunca duplicados) + el tercer bloque `[[bench]]`.
8. **N.8 Prueba de fuego**: tests de equivalencia (los DOS test-tesis) +
   `cargo bench --bench bench_columnar` con salidas REALES pegadas (Xeon E5-2682 v4
   @2.50GHz, misma máquina que cap. 34) + tabla de ratios (ciudad vs email vs saldo) +
   conteo filas lógicas/celdas físicas del ejemplo 2-hop; inspección opcional del asm
   mostrando el bucle vectorizado.
9. **N.9 Qué hemos sacrificado**: capa de lectura sin integración en el Executor;
   columnas solo en RAM (paginación columnar en disco = siguiente paso FUERA del
   libro); sin intrinsics/portable-simd; sin RLE/Frame-of-Reference/otros codecs;
   factorización sin operadores factorizados (cap. 39); sin paralelismo.
10. **N.10 Cómo lo hace una BBDD real + retos**: DuckDB (vectores + compresión),
    ClickHouse, Vertica (Lamb et al., PVLDB 2012 — C-Store 7 años después), Sybase IQ
    (1995, Expressway 103: PRIMER comercial columnar — historia reservada de N.0),
    PostgreSQL (heap row store + extensiones), y **Kùzu/LadybugDB** según ADR-001:
    columnar + vectorizado + factorización para GRAFOS (CIDR 2023). Retos esencial
    (nuevo predicado de rango sobre columna Int con test de equivalencia), intermedio
    (¿cuándo PIERDE el diccionario? predecir el ratio de `telefono`/`email` ANTES de
    medir), experto (extender la factorización a TRIÁNGULOS y calcular células
    ahorradas).
11. **Baterías finales**: Lo que te llevas / Ojo cuidado / Pin de batalla / 30 segundos
    / Una historia pequeña / Mini-diálogo de guardia nocturna (la analítica nocturna
    que agotaba RAM con tuplas planas y ahora cabe factorizada). Retrieval practice:
    reproducir DE MEMORIA el recorrido row→diccionario→packing→lote→máscara→resultado
    y la matriz 2×2. Interleaving: cada ejercicio toca ≥2 capítulos (9+38 tags,
    26+38 BitSet/lotes, 14+38 layout, 32/41+38 float-JSONL). Glosario nuevo: row
    store, column store, dictionary encoding, bit packing, lote/vector, máscara de
    selección, auto-vectorización, factorización, multiplicidad, cardinalidad.
12. **Gancho de cierre (preguntas abiertas)**: has acelerado CÓMO se lee; ¿y si
    cambiases QUÉ se calcula? Cap. 39: expand-como-join, explosión de intermedios,
    worst-case optimal joins y la ejecución factorizada COMPLETA.

---

## 7. Estilo y tono (consistencia con caps. 27-37)

- **Voz**: didáctica, sin solemnidad; tuteo; terminología técnica en inglés entre
  paréntesis la primera vez; salidas REALES pegadas (hardware, SO y toolchain
  declarados), nunca reconstruidas; metodología reproducible; cifras honestas aunque
  el delta sea modesto (reportar ×1.2 es tan válido como ×16 — lo prohibido es
  inflar).
- **Diagramas**: matriz layout×granularidad (§4); pipeline de seis etapas; bloque
  «lo que sí / lo que aún no»; patrón de bits del packing dibujado a mano.
- **Spacing** (conceptos viejos que se EJERCITAN): props `HashMap` row store (cap. 7),
  tags de `Value` (cap. 9), CSR y su ×16 (cap. 14), Volcano fila-a-fila (cap. 20),
  `BitSet`/bloques/`Presupuesto` (cap. 26), import/export JSONL y el hallazgo
  Float-entero (caps. 32/41), torre de pruebas (cap. 33), metodología de benches y
  dataset determinista (cap. 34).
- **Interleaving**: reto esencial mezcla 21+38 (predicado nuevo + equivalencia con el
  plan existente); el intermedio mezcla 9+34+38 (predecir ratio por tipos y
  cardinalidades ANTES de medir); el experto mezcla 24/39+38 (triángulos factorizados).
- **Dificultad asimétrica**: una idea nueva por sección (layout → extracción →
  diccionario → bits → lote → máscara → factorización); los ejercicios exigen PREDECIR
  ratios y recordar el recorrido sin pistas.
- **Bucle de feedback**: `cargo test -p vol2-liradb --lib cap38` (mini dataset,
  milisegundos) y `./scripts/verify.sh` ALL_GREEN como puerta; `cargo bench` como acto
  EXPLÍCITO del lector. Nunca «confía en mí».
- **Anécdota (única, verificada)**: MonetDB/X100 en CWI — el prototipo académico que
  «destrozó su propia imagen» al compararse con un bucle escrito a mano (Boncz, página
  personal; tesis doctoral «Monet: A Next-Generation DBMS Kernel For Query-Intensive
  Applications», UvA, defendida 31-may-2002; paper CIDR 2005 con Zukowski y Nes).
  Apoyo: C-Store VLDB 2005; SIGMOD 2006 (compresión operable); SIGMOD 2008
  (row-vs-column honesto); CIDR 2023 Kùzu CC-BY 4.0 (ADR-001); ICDT 2015
  (Olteanu-Závodný); Lamb et al. PVLDB 2012 (Vertica); historia Sybase IQ 1995
  (Expressway 103, SAP/Wikipedia/dbdb.io) RESERVADA para N.10 — candidata de N.0
  descartada por ser menos pertinente a la ejecución vectorizada.

---

## Checklist de profundidad (antes de marcar DONE)

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente
  (12 filas en §5).
- [x] Escenario de fallo visible, no solo happy path: diccionario que PIERDE con
  `email` (medido y reportado), bucle con rama interna que NO vectoriza (diagnosticado
  por asm), resto de lote mal manejado (test dedicado), factorización que duplicaría
  tuplas si la multiplicidad se cuenta dos veces (test de equivalencia lo caza).
- [x] Código ejecutable en workspace citado por nombre (**IMPLEMENTADO 2026-08-25**:
  `cap38_columnar.rs` (1.458 líneas, 11 tests = los nombrados en §2),
  `benches/bench_columnar.rs` (141), tercer `[[bench]]` en Cargo.toml, wiring
  lib.rs; **864 tests ALL_GREEN**; cifras reales Xeon E5-2682 v4. Divergencias
  mínimas documentadas: enum renombrado `TipoDatoColumna` (colisión con
  `TipoColumna` del cap32 en glob re-export); `Diccionario::codigo_de()` añadido
  (mitad SIGMOD-2006: operar comprimido); el informe imprime ratios/conteos, los
  TIEMPOS viven en criterion. CIFRAS CLAVE para la prosa: filtro row 34,574 ms
  VS columna-lotes 548,52 µs = **×63 — NO es SIMD, es LAYOUT** (asm verificado:
  escalar `cmpq $51` desenrollado; con -C target-cpu=x86-64-v3 el MISMO bucle
  emite `vpcmpgtq %ymm`); diccionario gana ciudad ×1,66 / pierde email ×0,86 /
  PIERDE idioma ×0,50 con cardinalidad 6 (cadena de 2 bytes < código de 4 B —
  la regla real es len(string) vs 4 B); packing ×6,40 (5 bits vs u32);
  factorización 2-hop: 65.536 filas lógicas → 69.888 celdas físicas vs 196.608
  planas = **ahorro 64,5%**; presencia sparse ~80% en las 19 claves).
- [x] Misconcepciones corregidas explícitamente (§1: cuatro, de «columnar siempre
  gana» a «mil millones de tuplas necesitan mil millones de celdas»).
- [x] Ejercicios con solución verificable diseñados (retos N.10 con nombres previstos).
- [x] ≥1 ejercicio de retrieval (recorrido row→packing de memoria + matriz 2×2) y
  spacing planificado (caps. 7/9/14/20/26/32/33/34/41 tocados; §7).
- [x] Responde la pregunta crítica del CORPUS (SIMD honesto medido + factorización
  con conteo; prerrequisito ADR-001 CUMPLIDO) y recoge el gancho del cap. 37
  (primera apuesta técnica de la Parte VIII, semilla ×16 del cap. 34; §3/§6).
- [x] Anécdota única verificada con fuentes primarias (página personal de Boncz;
  tesis UvA 31-may-2002; CIDR 2005) — candidata descartada (Sybase IQ 1995)
  reservada para N.10 con razón explícita.
- [x] Alcance de código nuevo acotado y honesto (UN módulo + UN fichero bench + UNA
  entrada `[[bench]]` + una línea de módulo en `lib.rs`; cero dependencias nuevas,
  cero cambios en caps. 7-37; §2/§5.1/§5.2).
- [x] Gancho saliente fijado (cap. 39 WCOJ: «columnas = CÓMO lees; WCOJ = QUÉ
  calculas»; §6.12).
