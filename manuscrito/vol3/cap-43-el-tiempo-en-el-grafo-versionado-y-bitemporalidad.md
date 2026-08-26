# Capítulo 43 — El tiempo en el grafo: versionado y bitemporalidad

> *«Tercer capítulo del Volumen III. Si vienes del cap. 42 —o del perfil datos/IA que lo haya leído en diagonal— la escalera R1-R7, las 10 preguntas, los validadores por composición, la migración y la regresión son tus HERRAMIENTAS, no tu contenido: aquí el modelo ya está refactorizado y ahora se le añade el TIEMPO. El cierre del cap-42 te dejó una pregunta clavada: la ronda 2 contrarresta a la ronda 1 — pero ¿QUÉ valía el 3 de marzo? Este capítulo la responde CON DATOS: el 3 de marzo de 2025 valía la **nota 7** (la ronda 1): la ronda 2 (nota 8) llegó después y la contrarrestó; el grafo del cap-42 decía QUÉ contrarresta a qué, y no podía decirte CUÁNDO porque no guardaba el tiempo. Aquí el tiempo entra al grafo — con la frontera del grano (granularity) declarada: el dataset habla en AÑOS (como `anio` del cap-41), y con grano anual 2025 responde 8, porque la frontera de caducidad es el año mismo; distinguir el 3 del 10 de marzo exigiría grano fino, y el grano lo pone el dato, no el modelo (Snodgrass 1999: intervalos continuos).»*

## 43.0 La anécdota de la esquina

Un hospital. Los pacientes comparten sala durante **periodos** (periods of time): la sala 3 acoge a A y B en enero, a B y C en febrero, y en marzo solo a C. Si dibujas el grafo estático de «quién compartió sala con quién», aparece una arista A-B, otra B-C y otra A-C — pero A y C **nunca coincidieron**: la arista A-C es un fantasma del estático, un contagio que nunca pudo ocurrir. Holme y Saramäki (Physics Reports, 2012) revisaron esta familia de redes temporales (temporal networks) —pacientes, correo instantáneo, llamadas, contactos— y destilaron la frase que ordena este capítulo: se trata de **«mover la información de cuándo pasa algo, del sistema dinámico al propio grafo»** (moving the information of *when* things happen, from the dynamical system on the network, to the network itself). Cuando el cuándo vive fuera, la arista A-C existe y el modelo miente: los caminos temporales no son transitivos — un camino que respeta el tiempo no puede saltar de enero a marzo—, y Kostakos (Physica A, 2009) ya lo contaba para sus temporal graphs: el grafo estático **sobreproyecta** (over-projects) lo que nunca fue simultáneo.

Tu KB-Lira está enferma de lo mismo. El grafo del cap-42 dice que **Beto pertenece a Instituto Neurónica** — pero se fue en 2024 y ya no está. ¿Tu grafo te está mintiendo por omisión? No te ha mentido: **no sabía que el tiempo pasaba**.

## 43.1 Objetivo

Objetivo medible del outline: **representar la historia y la validez temporal de los hechos del grafo sin destruir el rendimiento de las consultas del presente**. Al terminar tendrás:

1. **El builder temporal** `kb_lira_paso3()`: el paso-2 refactorizado (67 nodos) + `aplicar_valid_time` → **68 nodos, 158 aristas, 10 `MEMBER_OF`** con valid-time (valid time) en props de arista.
2. **La validez como función** `arista_vigente_en` (intervalo medio abierto `[desde, hasta)`, ausencia = abierto).
3. **Consultas AS OF** `afiliaciones_vigentes_en` / `afiliaciones_actuales` con `CosteLecturas`.
4. **La tesis del coste, medida**: presente y AS OF pagan el MISMO barrido; cada arista vencida (expired edge) añade 1 `get_edge`.
5. **El contraste borrar-vs-caducar**, con números.
6. **Bitemporalidad (bitemporal) mínima**: `HistoricoAfiliaciones` + el caso de Dani (dos respuestas legítimas).
7. **La conexión con el WAL real del cap-28**, demostrada en un test.
8. **El validador paso-3** por composición con el paso-2.
9. **La regresión triple** (red de seguridad de los caps. 41-42).
10. **CSV determinista paso-3** + artefacto commiteado. **971 tests ALL_GREEN** (948 + 23), sin bench.

## 43.2 Problema

Mira el resultado del cap-42: 948 tests verdes, el modelo refactorizado, las 10 preguntas respondiendo con sus costes. Y sin embargo la P4 —la travesía de 2 saltos `(:Proyecto)<-[:WORKED_ON]-(:Persona)-[:MEMBER_OF]->(:Organizacion)` que diseñaste en el cap-41— sigue respondiendo **Beto → Instituto Neurónica**, y Beto se fue en 2024. La suite sigue verde y la respuesta es falsa. Ese es el problema: **el grafo atemporal miente por omisión**: la P4 responde afiliaciones que ya no existen, y ningún test lo caza porque nadie le ha pedido el cuándo. Verde no es verdad — la frase del cap-42 («verde no es sano») se repite en el eje temporal: el modelo estaba sano y ahora está DESACTUALIZADO, sin que el validador lo note.

Antes de dibujar nada, desactivemos las **seis** ideas equivocadas que suelen venir con el tema:

1. **«La temporalidad es para las fechas de nacimiento o publicación.»** No: la temporalidad vive en el VÍNCULO — «cuándo fue cierto que X pertenecía a Y» es un atributo de la arista, no de los nodos (Snodgrass 1999: el valid time es propiedad de los hechos modelados).
2. **«Guardar la historia duplica los datos y degrada las consultas del presente.»** No: caducar sin borrar NO degrada el presente — el barrido de adyacencia es el MISMO; lo que se paga es cada arista vencida que sigue en la lista (medible: 1 `get_edge` por arista histórica; el cap. 44 cambiará ese precio).
3. **«Bitemporalidad = dos fechas en la misma fila.»** No: son DOS EJES ortogonales — el valid-time (cuándo fue cierto en el mundo) y el transaction-time (cuándo lo supo el sistema). El caso de Dani lo demuestra: dos respuestas legítimas para la misma pregunta según el eje consultado.
4. **«El 3 de marzo (la nota de la ronda) es una fecha de evento.»** No: la pregunta del cap-42 era por la VALIDEZ (cuándo fue cierto), que no es ni cuándo ocurrió la reseña (evento) ni cuándo se registró (transacción). Los tres tiempos se confunden todo el tiempo (Jensen & Snodgrass, 1999).
5. **«Una fecha en un STRING vale lo mismo que una fecha tipada.»** No: un string no se compara por rango — la lección P6 del cap-41 (un string no se expande) repetida en el eje temporal.
6. **«El WAL del cap-28 ya guarda la historia.»** A medias: el WAL guarda las ESCRITURAS (el transaction-time del ESTADO), pero no el histórico de valores corregidos; por eso el `HistoricoAfiliaciones` existe — este capítulo demuestra lo que el WAL puede y lo que no puede responder.

Y el compromiso de honestidad que rige TODO el capítulo: los números salen de `cargo test`, nunca de la pizarra; si la tesis del contrato no cuadra con el ledger real, se pine el delta real y se explica POR QUÉ, prohibido maquillar contadores.

## 43.3 Modelo mental: la foto, la película y los dos relojes

El grafo del cap-42 era una **FOTO**: decía lo que sabemos hoy. Este capítulo lo convierte en **PELÍCULA**: cada arista nace, vive y caduca — y la pregunta «¿qué valía entonces?» tiene dos relojes: el del mundo y el del conocimiento. El panel que ordena todo:

```text
LOS TRES TIEMPOS (con la reseña de Fabio, el gancho del cap-42):
  EVENTO    cuándo ocurrió el hecho (event time)   la ronda 2 se escribió en 2025
  VALIDEZ   cuándo fue cierto en el mundo          la nota 8 es vigente DESDE 2025
            (lo que las aristas guardan)           (la nota 7 caducó: CONTRARRESTA 2025)
  REGISTRO  cuándo lo supo el sistema              el WAL del cap-28 (transaction-time)
            (lo que el log guarda)
```

La validez viaja en la arista como intervalo **medio abierto `[desde, hasta)`** (convención de Snodgrass 1999): el `hasta` se excluye — en 2024 Neurónica ya no vale. La línea de Beto:

```text
LA LÍNEA DE BETO (valid-time en la arista, intervalo [desde, hasta)):
  2018─────────────────────2024──────────────2026
  [─ Neurónica (53) ─────) [─ GrafoLuna (185) ─)
        hasta_anio=2024         desde_anio=2024, abierta
  «ahora» (2026): Beto→GrafoLuna · AS OF 2023: Beto→Neurónica · AS OF 2024: GrafoLuna
```

Y el doble reloj del caso que separa los ejes para siempre:

```text
EL CASO DE DANI (bitemporalidad: dos relojes):
  eje VALIDEZ (el mundo):     arista 55: desde_anio=2021 (corregido)
  eje REGISTRO (el saber):    Historico: ts=2023 «desde 2019» ──corrección──► ts=2025 «desde 2021»
  «¿qué creíamos en 2024?» → desde 2019      «¿qué sabemos hoy (2026)?» → desde 2021
  (la MISMA pregunta, DOS respuestas legítimas según el reloj consultado)
```

Debajo, la REGLA DE ORO heredada del cap. 34 (determinismo total): el «ahora» es una CONSTANTE (`ANIO_ACTUAL = 2026`), cada consulta devuelve su `CosteLecturas` pineado, y las 10 preguntas del cap-41 + el validador paso-2 del cap-42 son la red de seguridad: si añadir validez cambia una respuesta vieja sobre los subgrafos 1-2, el cambio está MAL. El momento ¡ajá! perseguido: **«el cap-42 me dijo QUÉ contrarresta a qué; este capítulo me dice CUÁNDO — y "cuándo" tiene dos respuestas: cuándo fue cierto y cuándo lo supimos. El grafo del cap-42 no mentía: simplemente no sabía que había pasado»** (Holme & Saramäki como epígrafe de la sección).

## 43.4 Primera solución

La primera solución — y la que todo el mundo aplica — es **la NO-solución doble**:

**(a) BORRAR la arista vencida.** Beto se fue de Neurónica: `delete_edge(53)` y listo. El presente queda limpio — `afiliaciones_actuales` responde Beto → GrafoLuna, idéntico al paso-3 normal, y el test `borrar_en_vez_de_caducar_destruye_el_as_of` lo confirma: **el presente es idéntico**. Borrar parece gratis. Y el AS OF 2023, sin la 53, responde `(Ana, UniLira); (Dani, Neurónica)` — **sin Beto**: su única afiliación vigente entonces era la que acabas de borrar. «Borrar es gratis… hasta que alguien pregunta por el pasado.»

**(b) Apuntar la fecha en un STRING.** La arista 53 se queda con `vigencia: "2018-2024"`. Legible, humana, exacta… e inservible: **un string no se filtra por rango** — «¿quién pertenecía a Neurónica en 2023?» exige parsear `"2018-2024"`, partir por el guion y comparar enteros. Es la lección P6 del cap-41 repetida: una string nunca se expande, y ahora ni se compara.

El capítulo te muestra ambas con sus modos de fallo ANTES de la solución: una destruye la historia, la otra la pone en un formato que nadie puede consultar.

## 43.5 Sus límites

La no-solución doble tiene tres límites que la delatan:

1. **Borrar destruye la historia.** «¿A quién pertenecía el proyecto en 2023?» no tiene respuesta: la arista ya no existe. El presente sigue perfecto — el límite es que el pasado deja de ser preguntable, y el pasado es exactamente lo que este capítulo quiere preguntar.
2. **El string no se compara.** `"2018-2024"` no participa en ningún rango; cada consulta temporal tendría que reimplementar un parser de fechas. El tipado no es estética: es consultabilidad (la lección P6, otra vez).
3. **El «ahora» de verdad cambia cada día.** Si la consulta usara `SystemTime::now()`, los tests del capítulo cambiarían cada día y el informe pineado moriría. El «ahora» del dataset es una constante (`ANIO_ACTUAL = 2026`) — disciplina del cap. 34: determinismo total o nada.

Y el límite que cierra la lista: **la pregunta del cap-42 sigue sin responderse**. Ninguna de las dos no-soluciones sabe decir qué valía la nota el 3 de marzo.

## 43.6 Solución evolucionada

Ocho piezas, cada una un TRADE-OFF con precio en lecturas:

**1. Valid-time en las aristas: `desde_anio` / `hasta_anio`.** `aplicar_valid_time` añade validez a las 10 `MEMBER_OF` — las 6 del paso-1 y 4 nuevas (ids 182-185) + el nodo 69 `:Organizacion` «Instituto GrafoLuna»:

```text
52 Ana→UniLira desde 2018 · 53 Beto→Neurónica 2018-2024 (VENCIDA) · 54 Carla→UniLira desde 2020
55 Dani→Neurónica desde 2021 (la CORREGIDA) · 56 Elena→GrafosYa desde 2019 · 57 Fabio→GrafosYa desde 2019
182 Hugo→UniLira 2019 · 183 Iris→GrafosYa 2022 · 184 Gaby→Neurónica 2023 · 185 Beto→GrafoLuna 2024
```

`InformeValidTime`: **8 aristas modificadas (6 + la REALIZA 149 y la CONTRARRESTA 157), 4 creadas, 1 nodo creado, 8 lecturas** (una `get_edge` por arista modificada). El esqueleto de la operación, tal cual vive en el módulo:

```rust
// cap43_temporalidad.rs — esqueleto de aplicar_valid_time()
for (id, desde, hasta) in [
    (52usize, 2018i64, None),     // Ana → UniLira
    (53, 2018, Some(2024)),       // Beto → Neurónica (VENCIDA: Beto se muda)
    (55, 2021, None),             // Dani → Neurónica (el valor CORREGIDO)
    // … 54, 56, 57
] { poner_validez(store, id, desde, hasta, &mut lecturas); }
// nodo 69 :Organizacion «Instituto GrafoLuna» + 4 MEMBER_OF (182-185)
// gancho reseña: REALIZA 149 → hasta 2025 · CONTRARRESTA 157 → desde 2025
```

La validez es atributo DEL VÍNCULO: la escalera R1-R7 del cap-41 decide que la afiliación no tiene identidad propia (sin relaciones salientes ni ciclo de vida más allá de la arista — R2/R3 no suben), y la arista ya se lee completa en el barrido: la validez viaja GRATIS en la lectura que ya se hace. El díptico con el cap-42 enseña la MISMA regla con dos veredictos: `Resena` se reificó porque tiene `CONTRARRESTA`; la afiliación no se reifica porque no tiene nada que perder.

**2. La validez como función.** `arista_vigente_en(arista, anio)` implementa `[desde, hasta)` con ausencia = abierto; una arista SIN props de validez es vigente SIEMPRE (retrocompatibilidad caps. 41-42):

```rust
// cap43_temporalidad.rs — el corazón de arista_vigente_en (4 casos)
match (desde, hasta) {
    (None, None) => true,
    (Some(d), None) => d <= anio,
    (None, Some(h)) => anio < h,
    (Some(d), Some(h)) => d <= anio && anio < h,
}
```

**3. Consultas AS OF con coste.** `afiliaciones_vigentes_en(store, proyecto, anio)` es la P4 del cap-41 CON tiempo — la MISMA travesía de 2 saltos (`WORKED_ON` entrante + `MEMBER_OF` saliente) con un filtro de validez; `afiliaciones_actuales` la envuelve contra `ANIO_ACTUAL`. Moneda: `CosteLecturas {in_edges, get_edge, get_node}` — la misma disciplina de contadores de `ContandoStore` (cap-26) y el ledger del cap-41; localizar el proyecto por nombre queda FUERA del ledger (saber QUÉ preguntamos es previo a la consulta, igual que en el cap-41).

**4. La tesis del coste — y su adaptación honesta.** El filtro de validez se aplica sobre datos YA leídos: la adyacencia no distingue vigentes de vencidas SIN índice, así que el presente y el AS OF barren EXACTAMENTE lo mismo: `el_presente_y_el_as_of_cuestan_el_mismo_barrido` pine **28 = 28 lecturas** para 2026 y 2023 (1 `in_edges` + 21 `get_edge` + 6 `get_node`). Lo que paga la historia es cada arista vencida que conservamos: `cada_arista_vencida_anade_una_lectura_al_barrido` mide la variante sin la 53 (`delete_edge`) → **27 totales (get_edge 21→20) y candidatas `MEMBER_OF` de Ana/Beto/Dani 4→3**. El contrato del capítulo predecía la tesis como «13→14» contando solo las candidatas del paso-1 vs paso-3; el ledger real mide el barrido completo (21 `get_edge`) y se pine el delta real — 21→20 y 4→3 — con su comentario: la tesis (1 lectura por arista vencida en CADA barrido que toca a su persona) sobrevive en ambas monedas; con 10M de vencidas, 10M de lecturas — la factura que el cap. 44 cobrará con el índice.

**5. Bitemporalidad mínima: `HistoricoAfiliaciones`.** Un Vec append-only de `EntradaHistoria {ts_registro, persona, organizacion, desde_anio}` donde el `ts_registro` lo asigna el propio histórico (monótono: 1, 2, …) — **el «WAL del modelo»**. Caso Dani: ts 1 (2023) «Dani→Neurónica desde 2019» — lo que se creía; ts 2 (2025) «desde 2021» — la corrección, la que coincide con la arista 55. `afiliacion_segun_registro(historico, Dani, 2024, 1)` → `(Neurónica, 2019)`; con `ts = 2` → `(Neurónica, 2021)`. Dos respuestas legítimas para la misma pregunta según el reloj que consultes — bitemporal NO es dos fechas: son dos ejes (Jensen & Snodgrass 1999: la distinción es el fundamento; TSQL2 1995: los dos ejes como tipos del estándar).

**6. El gancho cobrado.** La `REALIZA` 149 (ronda 1 de Fabio, nota 7) gana `hasta_anio:2025` y la `CONTRARRESTA` 157 (ronda 2 → ronda 1) gana `desde_anio:2025`. La regla de resolución, documentada en el módulo: la vigencia de una reseña NO vive en el nodo `:Resena` ni en su `SOBRE` — vive en su `REALIZA` (Persona→Resena) y, si es la sucesora de otra, en el `desde_anio` de su `CONTRARRESTA`. Candidata vigente = REALIZA que cumple `arista_vigente_en` Y (si emite CONTRARRESTA) todas sus CONTRARRESTA vigentes. Resultado real de `nota_de_resena_vigente_en(store, "Informe de revisión por pares 2025", anio)`: 2024 → **Some(7)** (la ronda 1 aún reina); 2025 → **Some(8)** (la ronda 2 la contrarrestó); 2026 → Some(8). La frontera del grano, declarada: el contrato pregunta «el 3 de marzo»; con grano ANUAL, 2025 responde 8 porque la frontera de caducidad es el año mismo — distinguir el 3 del 10 exige grano fino (Snodgrass: intervalos continuos; aquí el grano lo pone el dato, no el modelo).

**7. El validador paso-3 por composición.** `validar_modelo_kb_lira_paso3` REUTILIZA `validar_modelo_kb_lira_paso2` (cap-42, sin tocarlo) y añade reglas nuevas SOLO para `MEMBER_OF`: `desde_anio:Int` requerido; `hasta_anio ≥ desde_anio`; `desde_anio ≤ ANIO_ACTUAL`. El detalle de composición, con honestidad: el paso-2 ya FILTRA sus tipos (los 6 que gobierna), así que las reglas nuevas se propagan SIN filtro — el subgrafo refactorizado sigue cumpliendo su contrato, y las violaciones del paso-2 pasarían tal cual, sin taparse. Fixture corrupto a mano → **3 violaciones con ids [52, 54, 56]** (52 sin desde; 54 intervalo invertido, hasta 2019 < desde 2020; 56 validez futura, desde 2030). Estas reglas SIEMBRAN los constraints del cap. 44: lo que hoy es convención ejecutable, allí será garantía del motor.

**8. La REGLA DE ORO:** las respuestas viejas no cambian. Regresión triple: las 10 preguntas del cap-41 sobre el subgrafo paso-1 IDÉNTICAS (el único desajuste real es P4: la `MEMBER_OF` 185 añade Beto→GrafoLuna, que ES la lección — se filtra en el subgrafo con el helper `solo_afiliaciones_paso1`, org id < 30); las respuestas pineadas del paso-2 sobre el paso-3 entero (P1 Ana=4, P3 idéntico, P5 jerárquica 24, P6 Neurónica=5, P8=40 con 9 personas); `validador_paso2_acepta_el_modelo_paso3`. La P4 atemporal sigue diciendo Beto→Neurónica (su contrato no cambia) mientras la P4 CON tiempo dice Beto→GrafoLuna — **la diferencia ES el capítulo, no una regresión** (documentado en el test para que nadie lo «corrija»).

## 43.7 Código completo ejecutable

Todo vive en UNA pieza nueva: `liradb-workspace/crates/vol2-liradb/src/cap43_temporalidad.rs` (**1.853 líneas**, std puro, **23 tests**), cableada con dos líneas aditivas en `lib.rs`; el artefacto regenerable `datasets/kb-lira/paso-3/{nodes.csv (69 líneas), edges.csv (159 líneas), historico.csv (4 líneas)}` es la salida del builder — el dataset es lo que «importó el equipo», la temporalidad es código que se re-ejecuta. CERO dependencias nuevas, CERO cambios en caps. 7-42, goldens intactos. Y **NO hay `[[bench]]`**: decisión #11 del contrato en una línea — la moneda son lecturas y conjuntos exactos, y cronometrar no sostiene ninguna tesis de este capítulo (espejo de la decisión #12 del cap-41).

Las piezas que sostienen el edificio (nombres exactos; el código completo vive en el módulo):

```rust
pub const ANIO_ACTUAL: i64 = 2026;                                  // el «ahora» FIJO del dataset
pub fn kb_lira_paso3() -> MemoryStore;                              // 68 nodos, 158 aristas, 10 MEMBER_OF
pub fn aplicar_valid_time(store: &mut MemoryStore) -> InformeValidTime; // 8 modificadas, 4 creadas, 1 nodo, 8 lecturas
pub fn arista_vigente_en(arista: &Edge, anio: i64) -> bool;         // [desde, hasta) con ausencia = abierto
pub struct CosteLecturas { pub in_edges: usize, pub get_edge: usize, pub get_node: usize }
pub fn afiliaciones_vigentes_en(store: &dyn GraphStore, proyecto: &str, anio: i64)
    -> (Vec<(String, String)>, CosteLecturas);
pub fn afiliaciones_actuales(store: &dyn GraphStore, proyecto: &str)
    -> (Vec<(String, String)>, CosteLecturas);
pub fn nota_de_resena_vigente_en(store: &dyn GraphStore, titulo: &str, anio: i64) -> Option<i64>;
pub struct EntradaHistoria { pub ts_registro: u64, pub persona: usize,
    pub organizacion: usize, pub desde_anio: i64 }
pub struct HistoricoAfiliaciones;                                    // append-only, ts monótono (el WAL del modelo)
pub fn validar_modelo_kb_lira_paso3(store: &dyn GraphStore) -> Result<(), Vec<Violacion>>;
pub fn informe_temporal_reproducible(store: &dyn GraphStore) -> String; // la tabla del §43.8
```

## 43.8 Prueba de fuego

Primero el bucle rápido — salida REAL de `cargo test`, sin tiempos:

```text
$ cargo test -p vol2-liradb --lib cap43

running 23 tests
test cap43_temporalidad::tests_temporalidad::estructura_de_kb_lira_paso3_cuenta_y_etiquetas_exactas ... ok
test cap43_temporalidad::tests_temporalidad::las_member_of_llevan_validez_desde_y_hasta_en_anios ... ok
test cap43_temporalidad::tests_arista_vigente::arista_vigente_en_cubre_abierta_vencida_y_futura ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_actuales_de_kira_responden_beto_en_grafosluna ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_as_of_2023_responden_beto_en_neuronica ... ok
test cap43_temporalidad::tests_as_of::afiliaciones_as_of_2019_no_incluyen_a_dani ... ok
test cap43_temporalidad::tests_coste_temporalidad::el_presente_y_el_as_of_cuestan_el_mismo_barrido ... ok
test cap43_temporalidad::tests_coste_temporalidad::cada_arista_vencida_anade_una_lectura_al_barrido ... ok
test cap43_temporalidad::tests_coste_temporalidad::borrar_en_vez_de_caducar_destruye_el_as_of ... ok
test cap43_temporalidad::tests_resena_vigente::la_ronda_1_de_fabio_caduco_cuando_la_ronda_2_la_contrarresto ... ok
test cap43_temporalidad::tests_historico_afiliaciones::historico_afiliaciones_registra_el_caso_de_dani ... ok
test cap43_temporalidad::tests_historico_afiliaciones::afiliacion_segun_registro_distingue_lo_creido_de_lo_cierto ... ok
test cap43_temporalidad::tests_wal_transaction_time::el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time ... ok
test cap43_temporalidad::tests_validador_paso3::validador_paso3_acepta_el_modelo_temporal ... ok
test cap43_temporalidad::tests_validador_paso3::validador_paso3_rechaza_fixture_sin_validez ... ok
test cap43_temporalidad::tests_regresion_temporalidad::las_10_preguntas_del_paso1_no_cambian_tras_anadir_valid_time ... ok
test cap43_temporalidad::tests_regresion_temporalidad::las_respuestas_del_paso2_no_cambian_tras_anadir_valid_time ... ok
test cap43_temporalidad::tests_regresion_temporalidad::validador_paso2_acepta_el_modelo_paso3 ... ok
test cap43_temporalidad::tests_csv_paso3::csv_roundtrip_paso3_import_export_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_historico_roundtrip_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_paso3_coincide_con_dataset_commiteado_byte_a_byte ... ok
test cap43_temporalidad::tests_csv_paso3::csv_paso1_y_paso2_intactos_tras_paso3 ... ok
test cap43_temporalidad::tests_informe_temporal::informe_temporal_reproducible_sobre_kb_lira ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 948 filtered out
```

Veintitrés verdes; workspace entero en **971 ALL_GREEN** (948 + 23) con goldens intactos. El gancho del cap-42, respondido por el test que lleva su nombre — la ronda 1 reina hasta 2025 y la ronda 2 la contrarresta desde entonces:

```text
$ cargo test -p vol2-liradb --lib cap43 la_ronda_1_de_fabio_caduco_cuando_la_ronda_2_la_contrarresto

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2024) = Some(7)  ← la ronda 1 aún reina
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2025) = Some(8)  ← la ronda 2 la contrarrestó
→ nota_de_resena_vigente_en(«Informe de revisión por pares 2025», 2026) = Some(8)
```

Ahora el informe del capítulo — salida REAL de `informe_temporal_reproducible`, pineada byte a byte:

```text
Barrido temporal de afiliaciones de «Proyecto Kira» (KB-Lira paso-3)
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
2026 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas
2024 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 lecturas
2023 | Ana → Universidad de Lira; Beto → Instituto Neurónica; Dani → Instituto Neurónica | 28 lecturas
2020 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas
2019 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 lecturas
──────────────────────────────────────────────────────────────────────────────────────────────────────────────
Caso Dani (bitemporal): lo que se creía frente a lo que se sabe
  ts 1 · «¿qué creíamos en 2024?»   → Instituto Neurónica, desde 2019 (lo que se creía)
  ts 2 · «¿qué sabemos hoy?»        → Instituto Neurónica, desde 2021 (lo que se sabe)
```

| Año | Afiliaciones de «Proyecto Kira» (AS OF) | Lecturas |
|---|---|---|
| 2026 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 |
| 2024 | Ana → Universidad de Lira; Beto → Instituto GrafoLuna; Dani → Instituto Neurónica | 28 |
| 2023 | Ana → Universidad de Lira; Beto → Instituto Neurónica; Dani → Instituto Neurónica | 28 |
| 2020 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 |
| 2019 | Ana → Universidad de Lira; Beto → Instituto Neurónica | 27 |

Cuatro lecturas obligatorias. **Primera: el medio abierto gobierna.** AS OF 2024 responde **GrafoLuna** — Neurónica caducó en 2024 (`[2018, 2024)`: el 2024 queda FUERA); el intervalo no es «hasta e incluyendo». **Segunda: la tesis del barrido, con su honestidad.** 2026 y 2023 pagan las MISMAS 28 lecturas (filtro sobre datos ya leídos, sin índice no hay atajo); 2020 y 2019 bajan a 27 porque Dani aún no se afilia (desde 2021) y su organización ni se lee — 1 `get_node` menos. **Tercera: el caso Dani.** La MISMA pregunta («¿cuándo empezó Dani en Neurónica?») con DOS respuestas legítimas: la del registro en 2024 (desde 2019 — lo que se creía) y la de hoy (desde 2021 — lo cierto). **Cuarta: el WAL, demostrado.** `el_historico_es_el_wal_del_modelo_y_el_wal_del_cap28_es_transaction_time` usa la API REAL del cap-28: `WalTransaccion::begin` + `put_node`/`put_edge` + `commit` (Begin + 3 ops + Commit = 5 registros), corte de luz con `Wal::as_bytes`/`reconstruir` y `replay_wal` → el store renacido reconstruye la arista 55 con `desde_anio=2019` (1 transacción confirmada, 3 operaciones reaplicadas) mientras el Historico con la corrección responde 2021. **La frontera, declarada: el WAL sabe lo que se escribió, no lo que se corrigió.** Equivalencia: LSN ≡ ts_registro (ambos los asigna el log, el primer LSN es 1 y el primer ts es 1), Commit ≡ entrada, replay ≡ re-lectura en orden.

Y el CSV cierra el círculo: `csv_roundtrip_paso3_import_export_byte_a_byte`, `csv_historico_roundtrip_byte_a_byte`, `csv_paso3_coincide_con_dataset_commiteado_byte_a_byte` y `csv_paso1_y_paso2_intactos_tras_paso3`: los ficheros de los pasos 1-2 ni se tocan. La validez, visible en el propio dataset — el artefacto `datasets/kb-lira/paso-3/` commiteado:

```text
$ head -3 datasets/kb-lira/paso-3/edges.csv
id:ID, de:START_ID, a:END_ID, tipo:TYPE, desde_anio:INT, hasta_anio:INT, order:INT
52,0,6,MEMBER_OF,2018,,
53,1,7,MEMBER_OF,2018,2024,
185,1,69,MEMBER_OF,2024,,

$ cat datasets/kb-lira/paso-3/historico.csv
ts_registro,persona,organizacion,desde_anio
1,3,7,2019
2,3,7,2021
3,0,6,2018
```

La arista 53 con `2018,2024` (la vencida), la 185 con `2024,` (la abierta) y el histórico con las dos entradas de Dani (persona 3 → organización 7): el caso bitemporal completo, en cuatro líneas.

## 43.9 Qué hemos sacrificado

1. **Sin índices sobre `desde_anio` ni constraints UNIQUE temporales.** El validador paso-3 SIEMBRA las reglas que allí serán constraints e índices; aquí son convención ejecutable, y el AS OF SIN índice —28 = 28 lecturas— ES la lección (cap. 44).
2. **Sin ingesta con transaction-time automático.** El histórico se construyó a mano en el builder; el pipeline que anota el cuándo al importar es el cap. 45.
3. **Sin grano sub-anual.** La reseña del 3 de marzo se responde con grano anual y la frontera queda declarada con Snodgrass: el grano lo pone el dato, no el modelo.
4. **Sin durabilidad del Historico.** Vive en RAM; su WAL de verdad (bytes + CRC, durabilidad) es el del cap-28, y la frontera queda marcada (caps. 37/45).
5. **Sin RDF ni quads.** El tiempo en tripletas (`<s, p, o, t>`) se despliega en el cap. 46.
6. **La bitemporalidad es MÍNIMA.** El grafo guarda el valid-time actual (lo que el motor sabe HOY); el Historico guarda el registro. Versionar el grafo por transaction-time exigiría un MVCC de modelo — el del cap-30 versiona por CONCURRENCIA, no por historia, y esa distinción es el reto intermedio.

## 43.10 Cómo lo hace una BBDD real + retos

Nada de lo que hiciste es exótico. **Neo4j** introdujo en la **3.4 (2018)** los tipos temporales NATIVOS —DATE, LOCAL/ZONED TIME, LOCAL/ZONED DATETIME, DURATION—, indexables con range lookups (la documentación de APOC lo confirma: «Neo4j 3.4 introduced temporal data types»), pero SIN bitemporalidad de consulta: el patrón industrial es el MISMO tuyo, props de validez en las aristas. **GQL (ISO/IEC 39075:2024, abril 2024)** estandariza los tipos de datos temporales (date/datetime/duration) y funciones temporales en su Parte 1; las consultas bitemporales AS OF NO están en esa primera parte — frontera declarada, sin afirmar más de lo verificado. La tradición SQL temporal viene de lejos: **TSQL2** (Kluwer, 1995) sentó los dos ejes como tipos del estándar, y **SQL:2011** lleva décadas incorporando periods y temporal tables — sin detalle firme aquí: la referencia canónica de la casa es **Snodgrass** (*Developing Time-Oriented Database Applications in SQL*, Morgan Kaufmann, julio 1999) y **Jensen & Snodgrass** («Temporal Data Management», IEEE TKDE 11(1):36-44, enero/febrero 1999, DOI 10.1109/69.755613). Y el lado de grafos temporales: **Holme & Saramäki** (Physics Reports 519(3):97-125, octubre 2012) y **Kostakos** (Physica A 388(6):1007-1023, 2009) — la anécdota del §43.0 con su teoría detrás.

**Retos para el lector (esencial / intermedio / experto):**

- *Esencial* (34+41+43): PREDICE por escrito, ANTES de correr nada, qué responderá `afiliaciones_vigentes_en(store, "Proyecto Kira", 2020)` y qué responderá para 2024 — personas, organizaciones y las LECTURAS de cada una (27 o 28, y por qué). Luego ejecuta `informe_temporal_reproducible` y verifica tu predicción contra la salida real. Pista de predicción: en 2020 Dani aún no existe (desde 2021) y en 2024 Beto ya está en GrafoLuna (medio abierto: Neurónica caducó).
- *Intermedio* (30+43, 41+43): compara el versionado del cap-30 —`VersionNode/VersionEdge{ts_begin, ts_end}`— con el valid-time: el MVCC versiona por CONCURRENCIA (snapshots para lectores concurrentes, gc), el valid-time por HISTORIA (cuándo fue cierto); escribe qué es lo mismo (intervalos de tiempo por versión) y qué es distinto (quién pregunta y quién limpia). Y aplica el patrón a `WORKED_ON`: ¿el Proyecto Brújula «nació» en 2022? ¿qué `desde_anio` le pondrías y por qué a las aristas de Elena?
- *Experto* (28+43): usa `WalTransaccion` REAL del cap-28 para registrar la corrección de Dani (la entrada que el builder puso a mano) y demuestra en un test qué puede y qué NO puede responder el WAL sobre «¿qué creíamos en 2024?» — la frontera que motiva el Historico.

## 43.11 Lo que te llevas

- **El grafo atemporal miente por omisión.** No dice mentiras: no sabe que el tiempo pasó. La P4 respondía Beto→Neurónica cuando Beto se fue en 2024.
- **La temporalidad vive en el VÍNCULO.** `desde_anio`/`hasta_anio` en la arista (intervalo medio abierto `[desde, hasta)`, ausencia = abierto): la MISMA escalera R1-R7 que reificó `Resena` aquí NO sube.
- **Tres tiempos, dos relojes.** Evento (cuándo ocurrió), validez (cuándo fue cierto — el grafo), registro (cuándo lo supo el sistema — el WAL). Bitemporal = dos ejes, no dos fechas: el caso de Dani responde dos verdades legítimas.
- **Presente y AS OF pagan el MISMO barrido.** 28 = 28 lecturas: el filtro se aplica sobre datos ya leídos; sin índice no hay atajo — y eso ES la lección.
- **La historia cobra 1 `get_edge` por arista vencida** en cada barrido que toca a su persona: 21→20, 28→27, candidatas 4→3. Borrar es gratis… hasta que alguien pregunta por el pasado.
- **Caducar sin borrar.** La respuesta vieja se conserva, el presente no cambia, y la REGLA DE ORO (respuestas viejas intactas) se verifica con la regresión triple.
- **La tabla AS OF es la figura del capítulo.** Cinco filas —2026/2024/2023/2020/2019— que cuentan la película entera: quién estaba, quién se fue, quién no había llegado, y a qué precio en lecturas.

## 43.12 Ojo, cuidado con…

- **Confundir borrar con caducar.** El presente idéntico no prueba nada: el test de contraste pregunta por el pasado.
- **Tratar el `hasta` como inclusivo.** `[2018, 2024)` deja FUERA el 2024: por eso AS OF 2024 responde GrafoLuna. El medio abierto se respeta en TODA la implementación.
- **Creer que una fecha en un string es una fecha.** Un string no se filtra por rango: la lección P6 repetida.
- **Usar el reloj de verdad en el código.** `SystemTime::now()` rompe el determinismo del cap. 34; el «ahora» es `ANIO_ACTUAL = 2026`.
- **Pedirle al WAL el histórico de valores corregidos.** El WAL sabe lo que se escribió, no lo que se corrigió; para eso existe el Historico.
- **«Corregir» la P4 atemporal.** Sigue devolviendo Beto→Neurónica por contrato; la diferencia con la P4 CON tiempo (Beto→GrafoLuna) ES el capítulo — si el test de regresión fallara, el error está en el test, no en el modelo.

## 43.13 Pin de batalla

> *«Un grafo sin tiempo no miente: simplemente no sabe que el tiempo pasó. Caducar sin borrar es darle al cuándo un lugar donde vivir — y el pasado, como la historia, se paga 1 lectura por arista vencida, hasta que alguien construye el índice.»*

## 43.14 Si solo lees 30 segundos

El grafo atemporal responde afiliaciones que ya no existen (P4 decía Beto→Neurónica cuando se fue en 2024). La cura: valid-time (valid time) en la arista —`desde_anio`/`hasta_anio`, intervalo medio abierto `[desde, hasta)`, ausencia = abierto— sobre las 10 `MEMBER_OF` de KB-Lira paso-3 (68 nodos, 158 aristas): la 53 Beto→Neurónica venció en 2024 y la 185 Beto→GrafoLuna (nodo 69) la sustituye. Tres tiempos: evento (cuándo ocurrió), validez (cuándo fue cierto), registro (cuándo lo supo el sistema — el WAL del cap-28). Bitemporalidad (bitemporal) = dos ejes: el caso de Dani — registro 2023 «desde 2019» vs corrección 2025 «desde 2021» — responde «¿qué creíamos en 2024?» → 2019 y «¿qué sabemos hoy?» → 2021. Consultas AS OF con coste en lecturas: presente y 2023 pagan el MISMO barrido (28 = 28); cada arista vencida añade 1 `get_edge` (21→20, 28→27, candidatas 4→3); borrar es gratis hasta que alguien pregunta por el pasado (`delete_edge(53)` destruye el AS OF 2023). Gancho cobrado: la nota vigente del Informe por años — 2024 → Some(7), 2025 → Some(8), 2026 → Some(8) — con el grano anual declarado: distinguir el 3 del 10 de marzo exige grano fino. Regresión triple: las 10 preguntas del cap-41 y las respuestas pineadas del cap-42 intactas; el validador paso-2 acepta el paso-3. 23 tests nuevos, workspace en **971 ALL_GREEN**, sin bench: la moneda son lecturas y conjuntos exactos. Fronteras: el índice temporal y la unicidad (cap. 44), la ingesta con transaction-time (cap. 45), RDF/quads (cap. 46).

## 43.15 Una historia pequeña

Tres de marzo de 2025. Fabio ha reescrito su reseña del «Informe de revisión por pares 2025»: la ronda 2, nota 8, contrarresta a la ronda 1, nota 7. El cap-42 te enseñó a ver la contrarresta como una arista `CONTRARRESTA` entre dos nodos `:Resena` — el QUÉ de la historia. Pero aquel grafo era una foto: podía decirte que la ronda 2 contrarrestaba a la ronda 1, no qué valía la nota el 3 de marzo. Este capítulo te dio el CUÁNDO: la ronda 1 reinó hasta 2025 (`hasta_anio:2025` en su `REALIZA`), y la ronda 2 solo comenzó a reinar cuando su `CONTRARRESTA` nació (`desde_anio:2025`). El 3 de marzo de 2025 valía la **nota 7**: la ronda 2 aún no había contrarrestado nada — y el grafo del cap-42 no podía decírtelo, no porque mintiera, sino porque no guardaba cuándo. Hoy, en tu KB-Lira temporal, esa pregunta tiene respuesta, con su grano declarado: en años, 2025 responde 8; en días, el 3 de marzo responde 7. El grano lo pone el dato, no el modelo — y tú ya sabes cuál es el tuyo.

## Ejercicios resueltos

**1. Retrieval sin pistas: los tres tiempos, DE MEMORIA, y clasifica 5 afirmaciones.** Cierra el libro y recita: **evento** = cuándo ocurrió el hecho; **validez (valid time)** = cuándo fue cierto en el mundo (lo que las aristas guardan); **registro (transaction time)** = cuándo lo supo el sistema (lo que el log guarda). Ahora clasifica: (a) «la ronda 2 se escribió en 2025» → **evento**. (b) «la nota 8 es vigente desde 2025» → **validez**. (c) «el commit del WAL registró la escritura» → **registro**. (d) «Dani se afilió a Neurónica en 2021» (la arista 55) → **validez**. (e) «en 2023 el registro anotó que Dani estaba desde 2019» (el ts 1 del Historico) → **registro**. Si clasificaste (d) como evento o (e) como validez, vuelve al §43.3: el orden ES el argumento.

**2. Explica por qué presente y AS OF pagan el mismo barrido y cuánto cuesta cada vencida.** Mecánica: `afiliaciones_vigentes_en` lee la adyacencia completa (`in_edges` + `get_edge` por arista candidata) y SOLO ENTONCES aplica `arista_vigente_en` — sin índice temporal, la adyacencia no distingue vigentes de vencidas, así que 2026 y 2023 barren lo mismo: 1 `in_edges` + 21 `get_edge` + 6 `get_node` = **28 = 28**. Cada arista vencida conservada sigue en la adyacencia de su persona y se lee antes de descartarse: medido contra la variante sin la 53, 21→20 `get_edge`, 28→27 totales y candidatas `MEMBER_OF` 4→3. La tesis del contrato (13→14 contando solo candidatas) se adaptó al ledger real con su comentario: la moneda completa es el barrido, y la lección no cambia — con 10M de vencidas, 10M de lecturas.

**3. Responde por qué el caso de Dani tiene dos respuestas legítimas.** Mecánica: la arista 55 dice `desde_anio:2021` (lo cierto, el eje VALIDEZ); el `HistoricoAfiliaciones` guarda dos entradas — ts 1 (2023) «desde 2019» y ts 2 (2025) «desde 2021» (el eje REGISTRO). `afiliacion_segun_registro(historico, Dani, 2024, 1)` responde `(Neurónica, 2019)`: lo que el sistema CREÍA en 2024; con `ts = 2` responde `(Neurónica, 2021)`: lo que se sabe tras la corrección. No es contradicción: son dos preguntas distintas sobre dos ejes ortogonales — bitemporalidad (bitemporal) es eso, no «dos fechas en la misma fila».

## Ejercicios propuestos

**Esencial (recordar + aplicar; 34+41+43).** Desarrolla el reto esencial del §43.10: PREDICE por escrito las respuestas y las lecturas de `afiliaciones_vigentes_en` para AS OF 2020 y AS OF 2024 ANTES de correr `informe_temporal_reproducible`. Criterio: predicción escrita primero; si tu predicción de 2024 dijo Neurónica, revisa la semántica medio abierta `[desde, hasta)`; si tu predicción de lecturas no distinguió 27 de 28, revisa cuándo se lee (o no) el nodo de la organización de Dani.

**Intermedio (predecir y comparar; 30+43 y 41+43).** (a) Compara `ts_begin/ts_end` del cap-30 con `desde_anio/hasta_anio`: escribe qué es lo mismo y qué es distinto entre el versionado por CONCURRENCIA (snapshots, gc) y el valid-time por HISTORIA. (b) Aplica el patrón a `WORKED_ON`: ¿el Proyecto Brújula «nació» en 2022? Justifica con la escalera R1-R7 si las aristas de Elena llevan `desde_anio` o si el nacimiento del proyecto es otro hecho. Criterio: cada respuesta con su porqué y su coste en lecturas.

**Experto (crear y demostrar; 28+43).** Registra la corrección de Dani con `WalTransaccion` REAL del cap-28 (la entrada que el builder puso a mano en `historico_kb_lira_paso3`), simula el corte de luz con `as_bytes`/`reconstruir`/`replay_wal` y demuestra en un test qué puede y qué NO puede responder el WAL sobre «¿qué creíamos en 2024?». Restricciones: std puro, sin tocar cap-28, suite ALL_GREEN con tu test dentro. Criterio: el test debe declarar la frontera (LSN ≡ ts_registro, Commit ≡ entrada, replay ≡ re-lectura) y fallar si alguien intenta leer la corrección del WAL.

## Para profundizar

- **Richard T. Snodgrass, *Developing Time-Oriented Database Applications in SQL*, Morgan Kaufmann, julio 1999 (ISBN 1-55860-436-7)** — el cap. 1 con los tres tipos de tiempo, los intervalos y la convención medio abierta; la referencia canónica de la casa.
- **Christian S. Jensen y Richard T. Snodgrass, «Temporal Data Management», IEEE TKDE 11(1):36-44, enero/febrero 1999 (DOI 10.1109/69.755613)** — valid time vs transaction time y el fundamento de la bitemporalidad.
- **Richard T. Snodgrass (ed.), *The TSQL2 Temporal Query Language*, Kluwer Academic Publishers, 1995 (ISBN 0-7923-9614-6)** — los dos ejes como tipos del estándar.
- **ISO/IEC 39075:2024 (GQL), abril 2024** — tipos de datos temporales (date/datetime/duration) y funciones temporales en la Parte 1; las consultas AS OF no están en esa parte (frontera declarada del contrato).
- **Neo4j 3.4 (2018), docs de tipos temporales** — DATE, LOCAL/ZONED TIME, LOCAL/ZONED DATETIME, DURATION, indexables con range lookups.
- **Petter Holme y Jari Saramäki, «Temporal networks», Physics Reports 519(3):97-125, octubre 2012 (DOI 10.1016/j.physrep.2012.03.001)** — la anécdota del §43.0 y «moving the information of *when* things happen from the dynamical system on the network, to the network itself».
- **Vassilis Kostakos, «Temporal graphs», Physica A 388(6):1007-1023, 2009 (DOI 10.1016/j.physa.2008.11.021)** — temporal graphs y la sobreproyección del grafo estático.
- SQL:2011 (periods y temporal tables) — contexto industrial citado sin detalle firme: frontera declarada del contrato.
- Dentro del libro: cap. 41 (escalera R1-R7, las 10 preguntas, P4 atemporal), cap. 42 (los refactors, la reseña reificada, la regresión), cap. 28 (WAL, Vol. II), cap. 30 (MVCC: el contraste versionado-concurrencia vs historia), cap. 32 (CSV round-trip), cap. 26 (ContandoStore), cap. 34 (dataset determinista), cap. 9 (Value sin tipo fecha: frontera del grano).

## Mini-diálogo: en guardia nocturna

> — Son las tres de la madrugada. El equipo pide «¿quién pertenecía a Neurónica en 2023?» y la consulta responde solo Ana y Dani.
>
> — ¿Y Beto?
>
> — Eso es. Beto trabajaba en Kira desde 2018 y era de Neurónica… hasta que alguien hizo `delete_edge(53)` cuando se fue en 2024. «Limpiamos el grafo», dijeron.
>
> — (pausa) El presente sigue perfecto, ¿verdad? Beto responde GrafoLuna por la arista 185.
>
> — Sí, por eso nadie se dio cuenta. La suite está verde.
>
> — La suite está verde y la historia está muerta. Borrar es gratis… hasta que alguien pregunta por el pasado. La arista 53 no se borra: se CADUCA — `hasta_anio: 2024`, y el AS OF 2023 la sigue viendo. El borrado destruye la respuesta; la caducidad la conserva, y cuesta exactamente 1 lectura por arista vencida en cada barrido.
>
> — O sea que la historia se paga.
>
> — Se paga en lecturas, y el cap-44 construirá el índice que la abarata. Por ahora, restaura la arista. Buenas noches.

> *Siguiente parada, cap. 44 (constraints e índices temporales): el AS OF sin índice paga 1 lectura por cada arista vencida — ¿quién construye el índice que lo abarata? ¿Y quién garantiza que dos afiliaciones no se solapen en el tiempo? Preguntas que dejamos abiertas: ¿quién anota el transaction-time automáticamente al importar el lote? (cap. 45, ingesta). ¿Y cuándo el KG temporal se convierte en la memoria de un agente? (cap. 53).*
