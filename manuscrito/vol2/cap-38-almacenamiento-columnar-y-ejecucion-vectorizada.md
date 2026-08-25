# Capítulo 38 — Almacenamiento columnar y ejecución vectorizada

> *«Llevábamos treinta y siete capítulos afilando cómo calcula LiraDB. Este capítulo midió algo incómodo: casi todo ese tiempo no se iba en calcular, sino en encontrar dónde estaban los bytes. Cambiar dónde están puestos dividió el coste por sesenta — sin tocar ni una línea del motor transaccional.»*

## 38.0 La anécdota de la esquina

Ámsterdam, primeros años de la década de 2000. En el Centrum Wiskunde & Informatica (CWI), Peter Boncz, Marcin Manegold y Niels Nes tenían entre manos MonetDB: un motor analítico main-memory celebrado, orgullo académico del grupo, almacenado por columnas mucho antes de que fuera moda. Y entonces hicieron lo más peligroso que un equipo puede hacer con su propio producto: escribir a mano, como programa independiente, el núcleo de una consulta de TPC-H — la Q1, agregaciones puras sobre una tabla grande — y ponerlo al lado del motor. El programa de veinte líneas destrozó la competencia… y también destrozó, en palabras del propio Boncz, «nuestra imagen de MonetDB como cumbre del rendimiento analítico». Su prototipo académico era LENTO por decisión de diseño: materializaba columnas enteras en cada operador, y el sistema completo vivía limitado por el ANCHO DE BANDA DE MEMORIA (*memory-bandwidth bound*) — el CPU pasaba hambre mientras los buses se ahogaban moviendo gigas que nadie miraba.

La cura fue MonetDB/X100 (Boncz, Zukowski y Nes, CIDR 2005): dejar de materializar columnas enteras y procesar en **vectores pequeños, cache-residentes** — unos ~100 valores en X100 — con primitivas apretadas que el CPU puede pipelinar. La lección no fue «usa SIMD»: fue que el enemigo no era falta de instrucciones, sino CÓMO estaban puestos los bytes y EN QUÉ TROZOS se recorrían.

Hazte la misma pregunta que ellos se hicieron con su criatura, porque este capítulo se la hace a la nuestra: ¿cuánto pierde LiraDB por SU layout de propiedades actual? No lo adivines. Mídelo.

## 38.1 Objetivo

La lista del capítulo anterior dijo QUÉ falta; esta Parte VIII construye DÓNDE crecer, señalando puntos concretos del hexágono. El primero sale de una medida propia: el cap. 34 midió el puerto clonando un `Vec` por llamada frente a la CSR cruda iterando 500k aristas — **×16** — y bautizó la ley: el coste está en CÓMO están puestos los bytes. Ese capítulo midió adyacencias. Aquí la ley florece en las PROPIEDADES. Al terminar tendrás:

1. **`TablaColumnar`** en `crates/vol2-liradb/src/cap38_columnar.rs` (1.458 líneas, std puro, cero dependencias nuevas): extracción row→column desde los props del dataset, ids ordenados ascendentes (determinismo), columnas MONOTIPA (`Int/Float/String/Bool`) y bitset de presencia reutilizando el `BitSet` del cap. 26 para props dispersos (*sparse*).
2. **Diccionario y bits**: `Diccionario` (cadena→`u32`) con estadística integrada y veredicto BIDIRECCIONAL — gana donde la cardinalidad es baja, PIERDE donde no — más bit packing (*bit-packing*) escrito a mano con tests exactos contra patrones binarios conocidos.
3. **Ejecución por lotes** (*batch execution*): `TAMANIO_VECTOR = 1024`, máscara en dos pasadas, resto manejado fuera del bucle caliente, y la equivalencia con el filtro fila-a-fila exigida POR TEST. Más la **factorización mínima** de resultados 2-hop con conteo exacto de filas lógicas contra celdas físicas.
4. **El tercer `[[bench]]`** del workspace (`benches/bench_columnar.rs`, 141 líneas, aditivo en `Cargo.toml`, `harness = false`) y la suite completa: **853 + 11 = 864 tests ALL_GREEN**, goldens intactos.

La tesis que lo vertebra es una matriz de dos ejes — DÓNDE viven los datos × EN QUÉ TROZOS se procesan — y una promesa de honestidad: sin medida propia no hay capítulo. El ×16 era la semilla; hoy se vuelve a medir en propiedades… y hasta el RESULTADO de una consulta va a resultar ser un problema de layout.

## 38.2 Problema

En realidad ya tienes 864 tests verdes, ACID, WAL, MVCC, CLI y aparato de medición. Y aun así, leer UNA propiedad — «dame los ids con `edad > 50`» — sobre los 100.000 nodos del dataset pasa fila a fila por `Node.props: HashMap<String, Value>` (cap. 7): búsqueda hash por nodo, tag del `Value` (cap. 9) por celda, puntero tras puntero. Tu suite responde «¿es CORRECTO?» y calla sobre «¿cuánto cuesta una ANALÍTICA sobre estos mismos datos?». Antes de construir nada, desactivemos cuatro ideas equivocadas que suelen venir con el tema:

1. **«Columnar siempre gana.»** No: para OLTP — leer UN nodo entero, escribir mucho — el row store manda, y tu `MemoryStore` no va a ninguna parte. Abadi, Madden y Hachem midieron (SIGMOD 2008) que un column-store NAIVE PIERDE contra un row-store bien afinado si la ejecución no se adapta. La elección la decide EL WORKLOAD, no el titular.
2. **«SIMD exige intrinsics o nightly.»** No: LLVM auto-vectoriza (*auto-vectorización*) bucles apretados sin ramas internas; lo que nightly añade es CONTROL (`portable_simd`). Con la toolchain pinneada 1.96.0 (ADR-002), lo honesto es MEDIR el efecto y saber inspeccionar ensamblador — no prometer instrucciones concretas.
3. **«Comprimir siempre acelera.»** No: decodificar cuesta CPU. Lo que hace rentable la compresión ligera es OPERAR SOBRE LOS CÓDIGOS cuando el predicado lo permite (Abadi, Madden y Ferreira, SIGMOD 2006) — y aun así depende de la selectividad. Ya verás al diccionario PERDER medido en dos claves.
4. **«Mil millones de tuplas necesitan mil millones de celdas en RAM.»** No: la REPRESENTACIÓN FACTORIZADA guarda arrays por variable más multiplicidades y preserva EXACTAMENTE el mismo multiconjunto lógico (Olteanu y Závodný, ICDT 2015; processor factorizado de Kùzu, CIDR 2023). Menos celdas físicas, cero filas perdidas — y lo vas a contar a mano.

Y debajo, la pregunta crítica del corpus: «¿qué significa SIMD y factorización AQUÍ — qué se puede demostrar HOY sin nightly y cuánto cambia la representación del resultado?» Respuesta en dos mitades medibles.

## 38.3 Modelo mental: layout × granularidad

Dos ejes independientes. El PRIMERO es dónde viven los bytes: agrupados por FILA (todas las props de un nodo juntas, como tu `HashMap`) o por COLUMNA (un atributo de todos los nodos junto, contiguo). El SEGUNDO es en qué trozos se procesan: valor a valor, o por lotes grandes. Cuatro casillas, y LiraDB vive HOY en la superior izquierda:

```text
                 PROCESADO valor-a-valor          PROCESADO POR LOTES (1024)
              ┌───────────────────────────────┬─────────────────────────────────┐
  DATOS       │ HOY: props HashMap fila a     │ este cap.: máscara sobre lote   │
  POR FILA   │ fila (el filtro del cap. 20)  │ de valores extraídos            │
              ├───────────────────────────────┼─────────────────────────────────┤
  DATOS       │ columna leída valor a valor   │ OBJETIVO: columna + lote +      │
  POR COLUMNA │ (mejor localidad, igual CPU)  │ bucle apretado ⇒ auto-vectoriza │
              └───────────────────────────────┴─────────────────────────────────┘
   (el cap. 14 ya vivió esto con adyacencias: puerto ×1 → CSR ×16)
```

Sobre esa matriz, el pipeline completo del capítulo — seis etapas que conviene recitar de memoria, porque los ejercicios te lo van a pedir SIN pistas:

```text
row store (props HashMap) ──extraer──▶ columnas tipadas (+ BitSet de presencia)
                                            │
                                 diccionario (String → u32)
                                            │
                                 bit packing (u32 → ⌈log₂(cardinalidad)⌉ bits)
                                            ▼
  filtro fila-a-fila  ◀══ EQUIVALENCIA POR TEST ══▶ lotes de 1024 → máscara → compactación
                                                                          │
  expansión 2-hop PLANA ◀══ EQUIVALENCIA POR TEST ══▶ resultado FACTORIZADO (arrays+multiplicidad)
```

Y la frontera, declarada antes de escribir código — la misma disciplina «lo que sí / lo que aún no» de los caps. 33-34:

```text
Lo que SÍ se hace hoy:   capa de LECTURA analítica row→columna (en RAM, sobre MemoryStore)
                         ejecución por lotes con máscara; factorización de RESULTADOS 2-hop
Lo que AÚN NO:           integración en el Executor (Volcano, cap. 20, sigue fila-a-fila)
                         paginación columnar en disco · intrinsics/portable-simd · WCOJ (cap. 39)
```

El momento ¡ajá! perseguido: el ×16 del cap. 34 no era magia del CSR — era la primera MEDIDA de una ley general. El coste está en CÓMO están puestos los bytes. Hoy se vuelve a medir en las propiedades, con una vuelta de tuerca: hasta el RESULTADO de una consulta es un problema de layout.

## 38.4 Primera solución

La versión que todo el mundo escribe — incluido nuestro yo de ayer — es extraer la propiedad a un vector de pares y filtrarla elemento a elemento:

```rust
let mut extraido: Vec<(NodeId, Value)> = Vec::new();
for id in ids_ordenados(&store) {
    if let Some(v) = store.get_node(id).and_then(|n| n.props.get("edad")) {
        extraido.push((id, v.clone()));          // clona el String si lo hay
    }
}
let seleccion: Vec<NodeId> = extraido
    .into_iter()
    .filter(|(_, v)| matches!(v, Value::Int(edad) if *edad > 50))
    .map(|(id, _)| id)
    .collect();
```

Correcto, legible, y pasa los tests. Es una «columna DISFRAZADA de fila»: cada celda arrastra el tag del `Value`, cada lectura despacha un `match`, cada string vive en su propia alocación del heap, y la presencia (que la prop exista o no) se pierde al extraer. Parece columnar porque los datos van juntos; se comporta como fila porque el PROCESADO sigue siendo valor a valor.

## 38.5 Sus límites

1. **El dispatch por celda mata el bucle.** Un `matches!` por elemento es una rama impredecible para el predictor; LLVM no puede pipelinar ni vectorizar lo que depende de un tag que cambia por celda. El bucle caliente paga impuesto de tipo 100.000 veces.
2. **Sin compresión, no hay menos bytes que mover.** Aunque los valores estén juntos, cada uno pesa lo mismo (o más, con cabeceras de `String`). El ancho de banda — el muro que hundió a MonetDB — sigue ahí.
3. **`Option<T>` rompería la densidad.** Envolver cada celda en un `Option` mete un discriminante por elemento: adiós bucle regular, adiós vectorización. Y un centinela NULL dentro de la columna reintroduce el dispatch que querías eliminar.
4. **Y el resultado EXPLOTA.** Una expansión 2-hop desde los hubs del dataset multiplica grados: materializar tuplas `(p, v, w)` planas dispara celdas por el fanout de DOS niveles. El problema del layout se repite UN nivel más arriba: en la forma del resultado.

## 38.6 Solución evolucionada

Seis gestos, cada uno con alternativa descartada.

**Gesto 1: columnas tipadas, extraídas del store, sin tocarlo.** `TablaColumnar::desde_store(&MemoryStore, claves)` es una capa de LECTURA analítica pura — hexagonal, como DiskStore en los caps. 33-34: el store transaccional no cambia ni un byte, porque para OLTP (point lookups, writes) el row store ES óptimo (SIGMOD 2008). Cada columna es monotipa (`Columna::Int(Vec<i64>) / Float / String / Bool`): el tipo se fija UNA vez al extraer y el bucle caliente ve `i64` pelados, nunca tags. Los ids se recolectan y se ORDENAN ascendente: `iter_nodes` itera contenedores hash con orden NO estable entre runs, y sin orden fijo no hay baselines ni tests comparables (la lección Mytkowicz, ASPLOS 2009, aplicada a datos). Descartada la reconversión del store a híbrido row+column: rompería 853 tests para resolver un workload que hoy no sirve.

**Gesto 2: presencia aparte, con el `BitSet` del cap. 26.** El esquema es disperso (~80 % de presencia por clave; `email` 100 %). La celda ausente ocupa su hueco con un valor neutro (0, 0.0, "", false) y LA VERDAD vive en el `BitSet`: marcar/contiene/unos, índices posicionales 0..n contiguos — exactamente su caso de uso, re-ejercitado del cap. 26. Descartados `Option<T>` por celda (rompe densidad) y centinela NULL (reintroduce dispatch).

**Gesto 3: diccionario (*dictionary encoding*) con estadística integrada y veredicto bidireccional.** Sustituir cadenas repetidas por enteros pequeños: tabla `Vec<String>` + códigos `Vec<u32>`. La estadística cuenta bytes antes (suma de cadenas) y después (4 B por código + la tabla), y el ratio SE REPORTA EN AMBAS DIRECCIONES: gana en `ciudad/pais/categoria`, PIERDE en `email` — y esa pérdida es material didáctico, no un fallo. Además `codigo_de()` traduce un literal UNA vez para filtrar por igualdad COMPRIMIDO, sin decodificar: la mitad que hace rentable todo esto (SIGMOD 2006).

**Gesto 4: bit packing a mano.** Los códigos caben en ⌈log₂(cardinalidad)⌉ bits: 32 ciudades → 5 bits. `empaquetar(&[u32], bits) -> Vec<u64>` apila valores LSB-first partiéndolos entre palabras cuando no caben; `desempaquetar` es el inverso exacto. Tests contra patrones binarios CONOCIDOS ([5,2,7] a 3 bits → 469, dibujado a mano abajo) más roundtrip exhaustivo k = 1..=32: la red ante los off-by-one clásicos. Descartado un crate externo (arrow/parquet): deps enormes, API opaca, nada que APRENDER bit a bit — la regla «primero a mano» de siempre (CONVENTIONS §4).

**Gesto 5: lotes de 1024 con máscara en dos pasadas.** `TAMANIO_VECTOR = 1024` — 1024 × 8 B = 8 KiB, cache-residente en L1/L2; la magnitud es propia (X100 usó ~100), justificada por el mismo criterio que opuso X100 a MonetDB: materializar columnas ENTERAS acaba limitado por ancho de banda. PASADA 1: máscara `mascara[i] = admite[i] && predicado(valor[i])` — bucle APRETADO sin ramas internas, apto para pipelinar y auto-vectorizar. PASADA 2: compactación con su rama, AISLADA del bucle caliente. El resto (`len % 1024`) se procesa con la misma forma FUERA del bucle principal, con test dedicado. Descartado el `if pred { push }` por elemento: divergencia que impide vectorizar.

**Gesto 6: factorización mínima del RESULTADO.** Para la expansión 2-hop, `ExpansionFactorizada` guarda arrays por variable (pivotes, intermedios, destinos) más multiplicidad por pivote — compartiendo prefijos en vez de repetirlos — y expone conteos exactos: filas LÓGICAS (tuplas que la consulta significa) contra celdas FÍSICAS (lo que de verdad se almacena). El multiconjunto es IDÉNTICO al plano, exigido por test-tesis. El motor factorizado completo y los worst-case optimal joins son del CAP. 39: fingirlos aquí sería inflarlo (Kùzu dedica a ello buena parte de su paper, CIDR 2023, CC-BY 4.0 — atribución según ADR-001).

## 38.7 Código completo ejecutable

Todo vive en dos piezas nuevas — y solo dos: `crates/vol2-liradb/src/cap38_columnar.rs` (1.458 líneas, std puro) y `benches/bench_columnar.rs`. El cableado es el mínimo posible: `pub mod cap38_columnar; pub use cap38_columnar::*;` en `lib.rs` (módulo 29), y la tercera entrada `[[bench]]` en `Cargo.toml` — cero dependencias nuevas, cero cambios en caps. 7-37. Las firmas que sostienen el edificio:

```rust
pub const TAMANIO_VECTOR: usize = 1024;
pub enum TipoDatoColumna { Int, Float, String, Bool }
pub enum Columna { Int(Vec<i64>), Float(Vec<f64>), String(Vec<String>), Bool(Vec<bool>) }
pub struct ColumnaTipada { /* valores + presencia BitSet + descartes_por_tipo */ }
pub struct TablaColumnar { /* ids ordenados + BTreeMap<clave, ColumnaTipada> */ }
impl TablaColumnar {
    pub fn desde_store(store: &MemoryStore, claves: &[&str]) -> Self;
    pub fn filtrar_int(&self, clave: &str, predicado: impl Fn(i64) -> bool)
        -> Result<Vec<NodeId>, ErrorColumnar>;   // ids, orden ascendente
}
pub struct Diccionario { /* tabla + índices + códigos */ }
impl Diccionario {
    pub fn nuevo(valores: &[String]) -> Self;
    pub fn codigo_de(&self, valor: &str) -> Option<u32>;  // operar comprimido
    pub fn estadisticas(&self) -> EstadisticaDiccionario; // ratio() >1 GANA / <1 PIERDE
}
pub fn bits_necesarios(cardinalidad: usize) -> u32;
pub fn empaquetar(valores: &[u32], bits: u32) -> Vec<u64>;
pub fn desempaquetar(palabras: &[u64], n: usize, bits: u32) -> Vec<u32>;
pub fn filtrar_lote_i64(valores: &[i64], admite: &[bool], predicado: impl Fn(i64) -> bool) -> Vec<usize>;
pub fn filtrar_fila_i64(store: &MemoryStore, clave: &str, predicado: impl Fn(i64) -> bool) -> Vec<NodeId>;
pub struct ExpansionFactorizada { /* pivotes, multiplicidad, inicio_slot,
                                     intermedios, inicio_destino, destinos */ }
impl ExpansionFactorizada {
    pub fn desde_adjacencias(adj_out: &[Vec<usize>]) -> Self;
    pub fn filas_logicas(&self) -> u64;
    pub fn celdas_fisicas(&self) -> u64;
    pub fn por_cada_tupla(&self, visitar: impl FnMut(usize, usize, usize));
}
pub fn informe_columnar(store: &MemoryStore) -> String;  // ratios/conteos; tiempos NO
```

Tres decisiones visibles en esas firmas, con su porqué:

- **`TipoDatoColumna`, no `TipoColumna`.** El nombre del cap. 32 (sufijos CSV del import/export) ya existe y el re-export plano del crate hace colisionar globales: el gemelo analítico lleva nombre propio. Pequeño, documentado, y mejor que un alias confuso.
- **`codigo_de()` existe y su test lo afirma.** Es la mitad SIGMOD-2006 del trato: traducir el literal UNA vez y comparar `u32` contra `u32` comprimido. Sin ella, el diccionario sería solo compresión — y ya sabes que comprimir sin cooperación de ejecución pierde.
- **Los TIEMPOS no viven en el módulo.** `informe_columnar` imprime ratios y conteos (reproducibles byte a byte, pineado por test); el cronómetro es de criterion, con warm-up y estadística. Un `Instant::now()` inline sería exactamente el número sin calibrado que el cap. 34 prohibió.

El patrón de bits del packing, dibujado a mano (es el caso 1 del test exacto):

```text
empaquetar([5, 2, 7], 3)   LSB-first, sin partir palabra

  valor   bits     posición
  5     = 101   →  [0..3)
  2     = 010   →  [3..6)
  7     = 111   →  [6..9)

  palabra u64:  0b111_010_101 = 469
                ↑↑↑ ↑↑↑ ↑↑↑
                7   2   5      (se lee de derecha a izquierda: LSB-first)
```

Y el esquema de la factorización, que es CSR aplicado dos veces — el cap. 14 reaparece donde menos lo esperabas:

```text
pivotes:        [p₀, p₁, …]                  una celda por pivote CON resultados
inicio_slot:    CSR pivote → slots
intermedios:    [v…]                         una celda por par (p, v) vivo
inicio_destino: CSR slot → destinos
destinos:       [w…]                         una celda por tupla lógica (p, v, w)

cada entrada de `destinos` = EXACTAMENTE una tupla lógica (nada se pierde);
p y v se almacenan UNA VEZ, compartidos por todos los w que cuelgan de ellos.
```

## 38.8 Prueba de fuego

Primero el bucle rápido, en milisegundos:

```text
$ cargo test -p vol2-liradb --lib cap38

running 11 tests
test cap38_columnar::tests_columnar::bit_packing_casos_conocidos_exactos ... ok
test cap38_columnar::tests_columnar::bit_packing_roundtrip_k_de_1_a_32 ... ok
test cap38_columnar::tests_columnar::columnar_tabla_desde_dataset_tipos_y_ordenes ... ok
test cap38_columnar::tests_columnar::columna_preserva_ausentes_con_bitset ... ok
test cap38_columnar::tests_columnar::diccionario_gana_en_ciudad_y_pierde_en_email ... ok
test cap38_columnar::tests_columnar::diccionario_roundtrip_y_estadisticas_ciudad ... ok
test cap38_columnar::tests_columnar::factorizacion_dos_saltos_filas_logicas_vs_celdas ... ok
test cap38_columnar::tests_columnar::factorizacion_equivale_a_la_expansion_plana ... ok
test cap38_columnar::tests_columnar::filtro_por_lotes_equivale_al_filtro_fila ... ok
test cap38_columnar::tests_columnar::informe_columnar_reproducible_sobre_mini ... ok
test cap38_columnar::tests_columnar::vector_tamano_fijo_maneja_el_resto ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 853 filtered out; finished in 0.00s
```

Once verdes, y el workspace entero en **864 ALL_GREEN** con goldens intactos. Dos de ellos son TESIS, no comprobaciones accesorias: `filtro_por_lotes_equivale_al_filtro_fila` exige que el MISMO predicado sobre el MISMO dataset produzca EXACTAMENTE los mismos ids en ambos layouts — velocidad sin corrección no es velocidad — y `factorizacion_equivale_a_la_expansion_plana` recorre la estructura factorizada y compara el multiconjunto completo contra la expansión plana. Si mañana alguien cuenta la multiplicidad dos veces, ese test se pone rojo antes de que el bug llegue a producción.

Ahora los números. Comando real, acto explícito tuyo (nunca en CI):

```text
$ cargo bench -p vol2-liradb --bench bench_columnar
filtro edad > 50: 30.220/100.000 filas pasan el predicado
...
TAMANIO_VECTOR = 1024
```

Hardware declarado — el de la casa desde el cap. 34: Intel Xeon E5-2682 v4 @ 2,50 GHz, Linux, rustc 1.96.0, perfil release, dataset de referencia (100k nodos / 500k aristas, `SEMILLA_REFERENCIA`). Selectividad real del filtro: 30.220 de 100.000.

**El hallazgo estrella del capítulo** — el mismo predicado `edad > 50`, dos layouts:

| Grupo `filtro_row_vs_columna_edad` | Mediana | Lectura |
|---|---|---|
| `row_escalar_hashmap` | **34,574 ms** | búsqueda hash + tag por celda, puntero tras puntero |
| `columna_lotes_1024` | **548,52 µs** | columna tipada contigua, máscara en dos pasadas |
| **Delta** | **×63** | y NO es SIMD: es LAYOUT |

Antes de que nadie grite «¡SIMD!», el ensamblador dice otra cosa. Inspección opcional — FUERA del pipeline, igual que los flamegraphs del cap. 34 — copiando el bucle caliente a un fichero propio:

```text
$ rustc --emit=asm -C opt-level=3 bucle_caliente.rs
# toolchain pinneada (x86-64 base): ESCALAR, desenrollado — un entero por instrucción
    cmpq    $51, (%rax)          ; LLVM reescribe v > 50 como v >= 51
    ...
$ rustc --emit=asm -C opt-level=3 -C target-cpu=x86-64-v3 bucle_caliente.rs
# el MISMO bucle, con AVX disponible:
    vpcmpgtq %ymm1, %ymm2, %ymm0     ; 4 enteros i64 comparados a la vez
```

Traducción: el binario que TODOS construimos con la toolchain pinneada compara escalar — y aún así gana ×63. El ×63 viene de eliminar búsquedas hash, dispatch de tags y saltos de puntero; de que los 100k enteros vivan CONTIGUOS y los lotes en caché. La auto-vectorización está DEMOSTRADA como alcanzable (el mismo bucle emite `vpcmpgtq` con AVX), pero es la guinda, no el pastel. «El coste está en CÓMO están puestos los bytes» — demostrado DOS veces: ×16 en adyacencias (cap. 34), ×63 en propiedades.

**Compresión: los ratios, en ambas direcciones.** Diccionario sobre el dataset completo (convención medida en ambos lados: solo contenido UTF-8):

| Clave | Cardinalidad | Ratio | Veredicto | Lectura |
|---|---|---|---|---|
| `categoria` | dominio pequeño | **×2,14** | GANA | pocas cadenas, relativamente largas |
| `pais` | ~10 | **×1,69** | GANA | dominio diminuto, nombres medianos |
| `ciudad` | 32 | **×1,66** | GANA | ⌈log₂32⌉ = 5 bits por nodo |
| `email` | única por nodo | **×0,86** | PIERDE | 4 B de código no recuperan nada |
| `idioma` | **6** | **×0,50** | PIERDE | ¡cardinalidad mínima y PIERDE! |

Fíjate en `idioma`, porque es la mejor lección del capítulo: cardinalidad 6 — tan baja como `ciudad` — y aun así el diccionario DOBLA el espacio. La regla real no es «cardinalidad baja gana»: es **len(cadena) contra tamaño de código**. Cadenas de ~2 bytes codificadas a 4 B por celda son un negocio ruinoso. El test `diccionario_gana_en_ciudad_y_pierde_en_email` pinea el veredicto bidireccional: la pérdida se REPORTA, no se oculta.

Y encima del diccionario, el packing de los códigos de `ciudad` (presencia ~80 % ⇒ 79.996 códigos):

| Representación | Bytes |
|---|---|
| códigos `u32` planos (79.996 × 4 B) | 319.984 B |
| empaquetados a 5 bits (⌈log₂32⌉) | **50.000 B** |

Ratio **×6,40**: los mismos códigos, un 84 % menos de bytes. Pero el precio de deshacerlo se MIDE — grupos `decodificacion_diccionario_ciudad` y `desempaquetado_bits`:

| Bench | Mediana | Denominador |
|---|---|---|
| `decodificar_completa` | 81,86 µs | 79.996 códigos (**977 Melem/s**) |
| `desempaquetar_codigos_ciudad` | 191,14 µs | 79.996 códigos (**418,52 Melem/s**) |

Decodificar toda la columna de códigos cuesta 82 microsegundos; desempaquetarlos, 191. Ninguno gratis — y por eso `codigo_de()` importa: un filtro por igualdad traduce el literal UNA vez y compara u32 comprimidos, sin pagar ninguno de esos dos precios (SIGMOD 2006). Comprimir acelera SOLO cuando la ejecución coopera.

**Factorización: contar sin materializar.** El ejemplo a mano del test `factorizacion_dos_saltos_filas_logicas_vs_celdas` — cuentas FIJAS que puedes verificar con lápiz:

```text
adj: 0→[1,2]  1→[3,4]  2→[5]  3→[] 4→[] 5→[]  6→[1]  7→[]

PLANO   : (0,1,3) (0,1,4) (0,2,5) (6,1,3) (6,1,4)   → 5 tuplas × 3 celdas = 15
FACTORIZ: pivotes=[0,6] · intermedios=[1,2,1] · destinos=[3,4,5,3,4]
          → 2 + 3 + 5 = 10 celdas físicas (ahorro 33,3 %)

el pivote 1 TIENE aristas pero CERO tuplas 2-hop: ocupa NI UNA celda.
```

Y sobre el subgrafo acotado del informe — 256 nodos con tope de grado 16, que son exactamente la ventana densa semilla del dataset del cap. 34, así que el conteo tocó el techo teórico 256·16·16:

```text
filas lógicas: 65.536 tuplas (p,v,w) | celdas planas: 196.608 | celdas físicas: 69.888
ahorro 64,5% | conteo aritmético sin materializar: 65.536 (checked_add)
```

Sesenta y cinco mil tuplas lógicas en menos de setenta mil celdas físicas, cuando planas serían casi doscientas mil — y el multiconjunto IDÉNTICO, exigido por el test-tesis. Ahí tienes la respuesta a la misconcepción nº4: mil tuplas no necesitan mil celdas; un millón de resultados no necesita un millón de nada. La REPRESENTACIÓN también es layout.

## 38.9 Qué hemos sacrificado

1. **Capa de lectura, no motor.** El Executor Volcano (cap. 20) sigue fila-a-fila: estas columnas no sirven consultas LiraQL todavía. Enchufarlo es proyecto propio — y cambiaría el plan físico, no solo el rendimiento.
2. **Columnas solo en RAM.** Sin paginación columnar en disco. Y respeto explícito del hallazgo del §41: aquí NO hay serialización — las columnas viven en memoria, donde `Float(2.0)` es `Float(2.0)`; exportarlo a JSONL degeneraría otra vez en `"2"`→`Int(2)`. La frontera queda declarada, no escondida.
3. **Sin intrinsics ni portable-simd.** Toolchain pinneada 1.96.0 (ADR-002): lo prometido es el DELTA MEDIDO y el MÉTODO de inspección del asm, opcional y fuera de CI. Sin compress-store vectorial: la compactación lleva su rama, aislada y medible.
4. **Sin otros codecs.** Ni RLE, ni Frame-of-Reference, ni dictarios por bloques: un codec enseñado bien vale más que cinco mencionados. (El experto del final te deja añadir RLE tú.)
5. **Factorización sin operadores factorizados.** La estructura y sus conteos sí; el motor completo (joins sobre representaciones factorizadas, WCOJ) es literalmente el siguiente capítulo. Kùzu necesita media docena de secciones para eso; nosotros una frontera honesta.
6. **Sin paralelismo.** Un core, lotes secuenciales. El paralelismo por trozos es natural aquí — y por eso mismo espera a tener el modelo secuencial medido.

## 38.10 Cómo lo hace una BBDD real

Nada de lo que hiciste es exótico: es la versión artesanal de industria madura. **DuckDB** es el emblema del género: embebido, columnar, ejecución vectorizada sobre vectores de ~2048 valores — nuestro `TAMANIO_VECTOR` con otro dígito — y compresión ligera operable sin descomprimir, la genealogía directa de X100. **PostgreSQL** es el recordatorio de que el row store no es el malo del cine: heap de filas con TOAST para valores grandes, y lo columnar llega por EXTENSIONES (Citus columnar, pg_analytics) cuando el workload analítico lo justifica — exactamente nuestra capa de lectura sin tocar el store, solo que con décadas de rodaje. **ClickHouse** lleva el modelo al extremo distribuido, y **Parquet** es el estándar de facto de columnas comprimidas en reposo: dictionary encoding y bit packing (RLE/Bit-Packing Hybrid) son LITERALESMENTE las dos técnicas que acabas de escribir a mano, en un formato que mueve medio ecosistema de datos. Y la historia reservada: **Sybase IQ** (1995, nacida del proyecto Expressway 103) fue el PRIMER comercial columnar — años antes de C-Store (Stonebraker et al., VLDB 2005) y de que Vertica lo industrializara (Lamb et al., PVLDB 2012). Para GRAFOS, la referencia conceptual es **Kùzu** (Jin, Feng, Chen, Liu y Salihoğlu, CIDR 2023, CC-BY 4.0): columnar para grafos, ejecución vectorizada y — su aportación distintiva — un processor FACTORIZADO que combina ambas ideas de este capítulo en un solo motor. Nota de presente según ADR-001: Kùzu fue archivada tras su adquisición por Apple (octubre de 2025) y la comunidad continúa en los forks LadybugDB y bighorn; citamos el paper CIDR 2023, jamás «renombrado a Ladybug». Nuestro módulo es clean-room: implementa el CONCEPTO publicado, cero código copiado.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (21+38): añade un predicado NUEVO al filtro por lotes — un rango, `edad >= 30 && edad < 45` — y escribe su test de equivalencia contra el filtro fila-a-fila sobre el dataset mini, siguiendo el patrón de `filtro_por_lotes_equivale_al_filtro_fila`. Antes de correrlo, anota la selectividad que esperas. Criterio: mismo contrato de salida (ids ascendentes), y el test falla si la máscara trata un hueco ausente como cero válido.
- *Intermedio* (9+34+38): el esquema tiene una clave que el informe no mide: `telefono`. Decide ANTES de medir si el diccionario GANA o PIERDE sobre ella, con la regla del capítulo (len de cadena típica contra 4 B de código) y estimando la cardinalidad del generador del cap. 34. Luego verifica contra `informe_columnar` o el bench. Si tu predicción falló, di QUÉ suposición era la equivocada — eso es el wisdom, no el acierto.
- *Experto* (crea y mide): dos caminos. (a) Extiende el packing a k > 32 (dos palabras garantizadas por valor) o añade RLE simple sobre los códigos ordenados, y compara ratios contra ×6,40 sobre los mismos datos. (b) Reproduce el efecto `target-cpu` del §38.8 con el snippet asm: compila el bucle caliente en base x86-64 y con x86-64-v3, y documenta las líneas `cmpq`/`vpcmpgtq` que encuentras. En ambos casos: fuera del pipeline de verificación, y las cifras con hardware declarado.

## 38.11 Lo que te llevas

- **Layout × granularidad**: dónde viven los bytes y en qué trozos se procesan son DOS decisiones independientes. LiraDB vivía en fila-a-fila/valor-a-valor; el objetivo era columna/lote.
- **El ×63 no es SIMD**: es LAYOUT. Contigüidad tipada sin búsquedas hash ni dispatch de tags. La auto-vectorización existe (asm con `vpcmpgtq` bajo AVX), pero el pastel lo hornea el orden de los bytes.
- **La ley general, ahora con dos medidas**: ×16 en adyacencias (CSR, cap. 34), ×63 en propiedades. El coste está en CÓMO están puestos los bytes.
- **Diccionario con veredicto bidireccional y operar comprimido**: gana en dominios pequeños (×1,66–×2,14), pierde en únicos (×0,86) — e incluso con cardinalidad 6 si las cadenas son cortísimas (×0,50; regla: len(cadena) contra tamaño de código). `codigo_de()` traduce el literal una vez y compara códigos: decodificar cuesta (81,86 µs), desempaquetar más (191,14 µs).
- **Lotes cache-residentes**: 1024 × 8 B = 8 KiB; máscara sin ramas + compactación aislada; el resto fuera del bucle caliente.
- **La representación del resultado también es layout**: 65.536 tuplas lógicas en 69.888 celdas físicas (ahorro 64,5 %), multiconjunto idéntico exigido por test.
- **Equivalencia o nada**: cada aceleración lleva su test-tesis contra la versión simple. Velocidad sin corrección es humo.

## 38.12 Ojo, cuidado con…

- **«Columnar siempre gana.»** Para point lookups y writes, tu `HashMap` es imbatible y SE QUEDA. Abadi et al. (SIGMOD 2008) midieron un columnar naive perdiendo contra un row store afinado. Workload primero.
- **Prometer instrucciones concretas.** «Esto usa AVX2» es infalsificable sin mirar el asm y frágil entre CPUs. Promete el DELTA medido y enseña el MÉTODO — la delimitación espejo de cargo-fuzz (cap. 33).
- **«Comprimir siempre acelera.»** `idioma` perdió ×0,50 con cardinalidad 6. Y decodificar/desempaquetar tienen precio medido. Compresión sin ejecución cooperante es decoración.
- **Mil tuplas ≠ mil celdas.** Si sigues materializando resultados planos por reflejo, la factorización del cap. 39 te va a parecer magia. No lo es: prefijos compartidos + multiplicidad.
- **Contar la multiplicidad dos veces.** El error clásico de factorización: iterar destinos Y sumar multiplicidades como si fueran filas adicionales. El test-tesis de equivalencia existe para cazarte — déjalo trabajar.
- **Confiar en el orden del store.** `iter_nodes` no es estable entre runs: sin ids ordenados, ni baselines ni informes reproducibles (Mytkowicz, ASPLOS 2009, otra vez).
- **Meter la presencia dentro de la columna.** `Option<T>` o centinelas reintroducen el dispatch por celda que este capítulo elimina. Verdad en el `BitSet`; bytes limpios en la columna.

## 38.13 Pin de batalla

> *«El ×63 no lo regaló el compilador: lo regaló el ORDEN DE LOS BYTES. Y cada cifra del capítulo lleva su test de equivalencia detrás — la velocidad sin corrección no es velocidad, es marketing.»*

## 38.14 Si solo lees 30 segundos

Matriz layout×granularidad: fila/columna × valor-a-valor/lote. LiraDB extraía props fila a fila vía `HashMap::get` — medido: filtrar `edad > 50` sobre 100k nodos tardaba **34,574 ms**; sobre columna tipada con lotes de 1024 y máscara en dos pasadas, **548,52 µs = ×63**, y el asm lo prueba: bucle ESCALAR (`cmpq $51`) — el layout manda, no el SIMD (con `-C target-cpu=x86-64-v3` el mismo bucle emite `vpcmpgtq %ymm`). Pipeline: extraer→codificar→empaquetar→lotear→máscara→compactar, todo con equivalencia exigida por test contra el camino fila. Diccionario bidireccional: ciudad ×1,66, país ×1,69, categoría ×2,14 GANAN; email ×0,86 e idioma ×0,50 PIERDEN (regla real: len(cadena) vs 4 B de código). Packing: 79.996 códigos a 5 bits → 50.000 B vs 319.984 B (×6,40); decodificar 81,86 µs, desempaquetar 191,14 µs — operar comprimido o no compres. Factorización 2-hop: 65.536 tuplas lógicas en 69.888 celdas físicas vs 196.608 planas (ahorro 64,5 %), pivote sin tuplas = cero celdas. Sin nightly no hay portable-simd: promesa = efecto medido + método de asm, fuera de CI. Gancho: has acelerado CÓMO se lee; el cap. 39 cambia QUÉ se calcula — worst-case optimal joins y el motor factorizado completo.

## 38.15 Una historia pequeña

Finales de los años ochenta. Grace Hopper — contralmirante de la Marina de EE. UU., pionera de los compiladores — recorría auditorios repartiendo trozos de cable de teléfono de unos TREINTA CENTÍMETROS. No eran recuerdos: eran nanosegundos. La luz en el vacío avanza ~29,98 cm en una milmillonésima de segundo, así que aquel pedazo de cobre ERA la distancia máxima que una señal podía recorrer por segundo y por nano — y Hopper lo usaba para aterrizar la abstracción: cuando los ingenieros le pedían «hacer las cosas más rápido con más cálculo», ella respondía sosteniendo el cable: en una nanosegunda tus bits no pueden estar más lejos de esto. Y remataba con la moraleja operativa: por qué los satélites — 300.000 km de ida — hacían las conversaciones lentas, y por qué en una máquina el dato LEJOS del que lo necesita es tiempo puro, facturable. Treinta y siete capítulos después de que este libro empezara con algoritmos, la contralmirante sigue teniendo razón y este capítulo es su factura: el ×63 no salió de calcular más rápido, salió de que los bytes dejaron de viajar — misma máquina, misma señal, menos distancia. Cuando alguien te proponga «optimizar» empezando por el procesador, saca tú el cable: pregunta primero cuántos nanosegundos está pagando en trayectos.

## Ejercicios resueltos

**1. ¿Por qué `idioma` pierde ×0,50 si su cardinalidad (6) es tan baja como la de `ciudad` (32), que gana ×1,66?** Porque el criterio de rentabilidad NO es la cardinalidad sola: es len(cadena) contra tamaño de código. Los idiomas del generador son cadenas de ~2 bytes; el diccionario paga 4 B por código, así que cada celda DOBLA su tamaño (bytes_después ≈ 4·n + tabla ≈ el doble de bytes_antes ⇒ ratio ≈ 0,50). En `ciudad`, cadenas de ~6-8 caracteres contra 4 B de código: gana aunque tenga cinco veces más cardinalidad. Verificación: la tabla del §38.8 (medidas del dataset completo) y `EstadisticaDiccionario::ratio()` con su convención UTF-8 en ambos lados.

**2. Cuenta a mano el ejemplo del test: ¿por qué el pivote 1 no ocupa NI UNA celda, y de dónde salen las 10 celdas físicas?** El pivote 1 tiene aristas (1→3, 1→4) pero sus vecinos 3 y 4 no tienen SALIDA, así que produce CERO tuplas 2-hop — y la estructura solo entra pivotes CON resultados: pagar celdas por filas inexistentes sería la mentira que la factorización elimina. Las 10 celdas son: 2 pivotes ([0,6]) + 3 intermedios ([1,2,1]) + 5 destinos ([3,4,5,3,4]), contra 15 planas (5 tuplas × 3). La multiplicidad lo declina: pivote 0 comparte 3 tuplas, pivote 6 comparte 2 — y 3+2 = 5 = filas lógicas, el invariante que el `debug_assert` del constructor y el test-tesis exigen. Verificación: `factorizacion_dos_saltos_filas_logicas_vs_celdas`.

**3. Retrieval sin pistas: recita el pipeline y la matriz.** Cierra el libro. Pipeline: extraer (ids ordenados, tipos fijados una vez) → diccionario (String→u32, estadística bidireccional) → packing (⌈log₂(cardinalidad)⌉ bits, LSB-first) → lotes de 1024 → máscara sin ramas → compactación (rama aislada) — con presencia en BitSet aparte y equivalencia testeada contra el filtro fila. Matriz 2×2: datos por fila o por columna × procesado valor-a-valor o por lotes; LiraDB vivía arriba-izquierda, el objetivo era abajo-derecha, y el cap. 14 ya había cruzado el mismo mapa con adyacencias. Si olvidaste la etapa de presencia o pusiste la compactación ANTES de la máscara, relee §38.3 y §38.6 — el orden no es cosmética: la máscara genera, la compactación selecciona.

## Ejercicios propuestos

**Esencial (recordar + aplicar; 21+38).** Ejecuta el reto esencial del §38.10: predicado de rango nuevo sobre columna Int, test de equivalencia contra fila, selectividad predicha por escrito ANTES del primer comando. Verificación: `cargo test -p vol2-liradb --lib cap38` sigue verde con TU test dentro, y el test falla deliberadamente si cambias la semántica de los huecos (prueba a quitar el `&& ok` de la máscara y mira cómo un ausente con valor neutro 0 se cuela en el resultado).

**Intermedio (predecir; 9+34+38).** Desarrolla el reto intermedio completo: cardinalidad esperada de `telefono` según el generador del cap. 34, longitud típica de cadena, ratio predicho con la regla len-vs-código, veredicto GANA/PIERDE — todo por escrito. Luego corre `informe_columnar` sobre el dataset (añadiendo la clave a `CLAVES_DICCIONARIO` temporalmente) y contrasta. Criterio: si acertaste el veredicto pero no el orden de magnitud, explica qué término del ratio ignoraste (¿la tabla? ¿los huecos?).

**Experto (crear y medir).** Elige camino del §38.10: (a) packing k > 32 o RLE sobre códigos ordenados con ratio comparado contra ×6,40; (b) reproducción del efecto `target-cpu` con el asm de ambos builds documentado. Restricciones: std puro, fuera del pipeline de verificación, hardware declarado, y — si tocas el módulo — cero cambios en caps. anteriores y suite ALL_GREEN. Criterio de éxito: tu informe cita cifras propias con metodología, no titulares.

## Para profundizar

- **Peter Boncz, Marcin Zukowski, Niels Nes, «MonetDB/X100: Hyper-Pipelining Query Execution» (CIDR 2005)** — la fuente primaria de la anécdota y del modelo de lotes: vectores cache-residentes, primitivas apretadas, compresión operable.
- **Peter Boncz, «Monet: A Next-Generation DBMS Kernel For Query-Intensive Applications» (tesis doctoral, UvA, 31-may-2002)** — el punto donde la imagen de MonetDB choca con el bucle escrito a mano.
- **Daniel Abadi, Sam Madden, Miguel Ferreira, «Integrating Compression and Execution in Column-Store Database Systems» (SIGMOD 2006)** — operar sobre códigos sin descomprimir: la mitad rentable del diccionario.
- **Daniel Abadi, Sam Madden, Nabil Hachem, «Column-Stores vs. Row-Stores: How Different Are They Really?» (SIGMOD 2008)** — el antídoto contra «columnar siempre gana»: el naive column-store que pierde.
- **Michael Stonebraker et al., «C-Store: A Column-oriented DBMS» (VLDB 2005)** — el columnar analítico fundacional; **Andrew Lamb et al., «The Vertica Analytic Database: C-Store 7 Years Later» (PVLDB 5(12), 2012)** — de paper a producto.
- **Guodong Jin, Xiyang Feng, Ziyi Chen, Chang Liu, Semih Salihoğlu, «KÙZU Graph Database Management System» (CIDR 2023, CC-BY 4.0)** — columnar + vectorizado + factorización para grafos; atribución según ADR-001 (archivada oct-2025; forks LadybugDB/bighorn).
- **Dan Olteanu, Jakub Závodný, «Factorised Representations of Query Results» (ICDT 2015)** — la teoría de prefijos compartidos y multiplicidades detrás del gesto 6.
- **duckdb.org (docs de almacenamiento y ejecución vectorizada)** y **parquet.apache.org (encodings: Dictionary, RLE/Bit-Packing Hybrid)** — las técnicas del capítulo en producción y en formato abierto.
- **postgresql.org/docs (TOAST; extensión Citus columnar)** — el row store de producción y cómo llega lo columnar sin tocar el core.
- **Grace Hopper en Late Night with David Letterman (NBC, 1986) y sus grabaciones de conferencias (Computer History Museum)** — el cable de la nanosegundo, repartido en directo.
- Dentro del libro: caps. 7 y 9 (row store y tags), cap. 14 (CSR y su ley de localidad), cap. 20 (Volcano fila-a-fila), cap. 26 (BitSet y bloques), caps. 32/41 (import/export y la frontera Float-entero), cap. 33 (delimitaciones honestas), cap. 34 (metodología de medición y el ×16).

## Mini-diálogo: en guardia nocturna

> — Son las dos de la mañana. La analítica nocturna del cliente — «dame los usuarios mayores de 50 por ciudad» — tarda más que anoche, y anoche ya tardaba. El árbol de spans del cap. 35 señala un bloque enorme en NodeScan. ¿Compramos más CPU?
>
> — El span no separa comparar de BUSCAR, pero tenemos el bench: el mismo predicado era 34,574 ms fila a fila y 548 µs por columnas. ¿Eso es treinta milisegundos de… búsquedas?
>
> — De TRAYECTOS. Hash lookup por nodo, tag por celda, puntero tras puntero. El CPU apenas calcula: espera. Comprar más CPU es comprar más camareros para un restaurante donde el problema es la distancia entre cocina y mesa.
>
> — Entonces la cura es…
>
> — Mover la cocina. Extraer la columna una vez, lotes de 1024, máscara y compactación — ×63 sin tocar el motor transaccional. Y si el resultado se pone gordo, factorízalo: 65.536 tuplas cabían en 69.888 celdas.
>
> — ¿Y si aun así es lento?
>
> — Entonces sí hablamos de hierro, o de paralelismo por lotes. Pero con el layout arreglado sabrás QUE lo compras y PARA QUÉ. Ahora al revés, estarías pagando CPU para esperar más rápido.

---

*(Próximo capítulo: 39 — joins worst-case optimal y el motor factorizado. Las columnas aceleran CÓMO lees; los worst-case optimal joins cambian QUÉ calculas: expand-como-join, explosión de intermedios y la ejecución factorizada completa que este capítulo dejó en la frontera.)*
