# CONTRATO DE CAPÍTULO — Vol.II Cap. 19: Del AST al plan lógico

> Rellenado a partir de `PLANTILLA-CONTRATO-CAPITULO.md`. Código ancla:
> `liradb-workspace/crates/vol2-liradb/src/cap19_plan_logico.rs` (1.985 líneas,
> 40 tests en `tests_logical_plan`). Decisiones y bugs reales:
> `liradb-workspace/book-context/MIGRATION-PATTERN.md` §23 (incluido el bug del brief:
> su plan de ejemplo omitía imponer `f:Person`). Línea 32 de
> `manuscrito/vol2/tabla-de-contenidos.md`. Es el tercer capítulo de la Parte IV:
> consume el AST de los caps. 17-18 y produce el árbol que el Volcano del cap. 20
> ejecutará y el optimizador del cap. 21 reescribirá.

---

## 1. El novato (perfil y punto de partida)

- **Sabe YA sin ninguna duda**: LiraQL = MATCH-WHERE-RETURN (cap. 17); el AST `Query`
  con `Span { start, end }` en TODO nodo y `Expression` (Literal/Variable/PropertyAccess/
  Compare/And/Or/Not) envolviendo al `Value` del cap. 7; `parse()` texto → tokens → AST
  con errores localizados `(en start..end)` y `write_span_suffix` (cap. 18); `Query::validate()`
  → `Vec<QueryError>` (UX: reporta todo a la vez); `Value` (Null/Bool/Int/Float/String/Bytes)
  y labels como cadenas sin esquema (cap. 7); que existen `HashIndex`/`BPlusTree` (cap. 15);
  el patrón de error tipado `{ kind, span }` (caps. 17-18).
- **Cree saber pero es vago/erróneo (misconcepciones centrales)**: (1) «el AST ya es la
  consulta, solo hay que ejecutarlo» — el AST es SINTAXIS: dice lo que pediste, no cómo se
  calcula; (2) «un plan que se ve bien es correcto» — el bug del brief: su plan omitía
  `f:Person` y habría devuelto conocidos de CUALQUIER etiqueta sin que nadie lo notara;
  (3) «`( )` anónimo no liga nada» — falso: también liga (`_n1`), porque el executor
  necesita nombrarlo todo; (4) «`Any` = sin tipo» — no: es «no prometido» por schemaless;
  la comparación concreta se decide en ejecución; (5) «el `Filter` arriba del árbol es un
  descuido del binder» — es el «antes» deliberado que el cap. 21 optimiza; ocultarlo ahora
  le robaría al optimizador su razón de existir; (6) «optimizar y planificar son lo mismo»
  — construir el plan (hoy) ≠ elegir entre planes equivalentes (cap. 21, Selinger).
- **NO debe saber todavía**: el modelo Volcano open/next/close y la evaluación trivalente
  (cap. 20); el push-down de predicados y la regla `index_seek` como REESCRITURA (cap. 21);
  re-binding de variables y ciclos en patrones (executor); estimación de cardinalidades y
  costes; joins; DML (`CREATE/MERGE`, cap. 31). `IndexSeek` se DECLARA aquí como operador,
  pero `lower()` jamás lo construye — se corta explicando quién lo hará.

## 2. Conceptos (del grafo curricular)

- `present`: binder / binding (tabla `Bindings`: nombre → `BindingKind` Node/Edge, en orden
  de declaración); lowering (`lower()`: AST → `LogicalPlan`); operadores lógicos (`NodeScan`,
  `Expand`, `Filter`, `Project`, `CartesianProduct`, `IndexSeek` declarado-por-el-21);
  expresión resuelta (`ScalarExpr`: sin `Span`, `Var { name, kind }` incrustado, `HasLabel`
  fabricado por el planner); variables internas `_n1`/`_e2` para anónimos; inferencia de
  tipos conservadora (`LogicalType`, `Any`/`Null` comodines, `eq_compatible`/
  `order_compatible`); `PlanErrorKind` (UnknownVariable, DuplicateVariable, VariableRebind,
  SharedPatternVariables, TypeMismatch, EmptyMatch/EmptyReturn) con `Span`; `Display`
  canónico del plan (indentación 2 espacios, paréntesis mínimos por precedencia
  NOT > AND > OR); `bound_variables()`.
- `practice`: `Span` y localización de errores (caps. 17-18); literales `Value` reutilizados
  (cap. 7); `parse()` como productor del AST que `lower()` consume (cap. 18);
  pretty-printers canónicos y errores tipados `{ kind, span }` (caps. 17-18).
- `consolidate`: «derivar, no re-resolver» (el `kind` incrustado en `Var`); una sola fuente
  de verdad por tipo (`Value`, `CompareOp`); límites declarados como errores tipados que
  apuntan al capítulo que los resolverá.
- `out_of_scope` (solo nombrar): ejecución y cortocircuito observable (cap. 20), push-down /
  `IndexSeek` como regla / estadísticas (cap. 21), re-binding y ciclos (cap. 20), hash join,
  DML (cap. 31).

## 3. Objetivos de dominio (taxonomía teaching)

- **Knowledge**: (1) explica la diferencia AST / plan lógico / plan físico con la analogía
  del restaurante y dice qué capa responde qué pregunta; (2) reproduce el lowering de la
  consulta estrella paso a paso, incluyendo por qué `p:Person` alimenta el `NodeScan` pero
  `f:Person` baja como `ScalarExpr::HasLabel` al `Filter` (el bug del brief); (3) enuncia
  las reglas de `eq_compatible`/`order_compatible` y por qué `Any`/`Null` son comodines en
  un motor schemaless; (4) distingue los 7 `PlanErrorKind` y razona qué se detecta en plan
  y qué DEBE esperar a la ejecución; (5) dice por qué el `Display` del plan es canónico
  (base de `liradb explain` y de los tests: idempotente, sin ruido, paréntesis mínimos).
- **Skills**: (1) predecir el texto EXACTO del `Display` de un plan a partir de la consulta
  LiraQL (y viceversa); (2) escribir tests de lowering con `assert_eq!` sobre
  `plan.to_string()` siguiendo el patrón de los 40 tests del módulo; (3) construir a mano
  `Bindings`/`ScalarExpr` para casos límite (anónimos, props inline, OR anidado).
- **Wisdom**: (1) decide qué errores se rechazan en plan y cuáles se aplazan a ejecución —
  rechazar sólo lo PROBABLE (`WHERE 3`, `p = TRUE`), nunca lo meramente posible
  (`p.edad = TRUE` con schemaless); (2) resiste la tentación de «optimizar dentro del
  binder»: el plan correcto-ingenuo de hoy es el material del optimizador de mañana
  (separación de capas, la lección de Selinger 1979).

## 4. Modelo mental

- **El restaurante**: AST = el pedido manuscrito del cliente («lo de siempre, sin cebolla»)
  — tachones, ambigüedad, y el papel se puede señalar (el `Span`); plan lógico = la comanda
  interna con estaciones numeradas (estación 1: sacar los platos `Person`; estación 2:
  seguir los enlaces `KNOWS` salientes; estación 3: quedarse con los que cumplan; estación
  4: emplatar `f.name`) — dice QUÉ produce cada estación y en qué orden, nada de quién lo
  hace; plan físico (caps. 20-21) = quién cocina y con qué utensilio (¿índice del cap. 15 o
  vaciar los armarios?). El **binder** es el camarero jefe: traduce nombres a números de
  estación y rechaza lo imposible señalando la línea del papel (`PlanError { kind, span }`).
- **Diagramas ASCII**: (a) pipeline texto → tokens → AST → plan lógico → [cap. 20] físico →
  filas; (b) árbol del plan de la consulta estrella con su `Display` exacto; (c) la tabla
  `Bindings` en construcción mientras el binder baja el patrón (`{p:NODE}` → `{p:NODE,
  f:NODE}`).
- **Momento ¡ajá!**: «la misma petición admite muchos planes equivalentes — y uno
  EQUIVOCADO puede verse perfecto» (el bug del brief). El plan no es un dibujo bonito: es
  un programa sobre el que se razona y se hacen pruebas; el test canónico del `Display` es
  la red que atrapó al brief.

## 5. Los porqués (grill — cada decisión del código)

| # | Decisión (`cap19_plan_logico.rs`) | ¿Por qué así? | Alternativa descartada + coste | Modo de fallo si no | Evidencia |
|---|---|---|---|---|---|
| 1 | Plan INTERMEDIO: `lower()` entre `parse()` y el executor | El AST es sintaxis (lo que pidiste); el plan es álgebra (cómo se calcula). Da un punto único de validación semántica y un árbol que el 20 ejecuta y el 21 reescribe SIN tocar la sintaxis | Intérprete que ejecuta el AST directo: errores de nombre descubiertos en ejecución, índices invisibles (no hay punto de reescritura), cada optimización futura tocaría sintaxis | Consultas que devuelven filas de más o fallan a mitad de recorrido, sin explicación posible | MIGRATION-PATTERN §23 lección 2 (patrón binder: Kùzu Parser→Binder→Planner); Selinger et al. 1979: la representación interna nació PARA que el optimizador elija |
| 2 | `Bindings` = `Vec<(String, BindingKind)>` en orden de declaración | Determinismo: mismo AST ⇒ mismo orden de ligadura ⇒ `Display`, columnas y tests estables entre ejecuciones; O(n) aceptable en lenguaje didáctico | `HashMap`: O(1) pero orden de iteración no especificado ⇒ `explain` y orden de variables inestables | Tests intermitentes y `explain` no reproducible | Código `Bindings` (doc-comment); test `bindings_declara_consulta_y_itera` |
| 3 | Anónimos ligan variables internas `_n1`/`_e2` con contador que salta ocupados | `( )` TAMBIÉN liga: el executor necesita nombrarlo todo; saltar nombres ocupados evita colisión con variables de usuario que empiecen por `_` | `Option<String>` en cada operador: duplica los `match` por todo el motor y complica `Expand { to }` | Dos anónimos colisionando ⇒ bindings corruptos silenciosos | `fresh_internal_var`; tests `lower_nodo_anonimo_genera_variable_interna`, `lower_dos_patrones_con_anonimos_no_colisionan` |
| 4 | `ScalarExpr` sin `Span`, con `kind` incrustado en `Var` | El span muere en parse: el plan vive en ejecución, no en el fuente; los errores de plan citan el span de la cláusula que originó la expresión. El `kind` incrustado elimina toda re-resolución downstream | Arrastrar el `Span` por todo el plan: memoria y ruido en cada igualdad estructural; `Var { name }` a secas: el executor re-resolvería nombres con tabla de símbolos en cada fila | Executor con tabla de símbolos en runtime = el binder re-hecho mil veces | MIGRATION-PATTERN §23 lección 2; `build_scalar` (usa el span del AST para el error, no lo guarda) |
| 5 | Labels de nodos de la CADENA bajan como `HasLabel` al `Filter`; sólo el inicial alimenta el `NodeScan` | Corrige el bug del brief: su plan omitía `f:Person` y habría devuelto conocidos de cualquier etiqueta. `NodeScan` sólo puede absorber UN label (el de su hoja); el resto son predicados | Un operador `NodeLabelFilter` separado: un operador nuevo para lo que es un predicado; la conjunción AND en el `Filter` basta y se presta al push-down del cap. 21 | Filas de más devueltas en silencio — el peor modo de fallo porque nadie se queja | MIGRATION-PATTERN §23 decisión 5 y lección 1; tests `lower_display_ejemplo_canonico_del_brief`, `lower_estructura_del_ejemplo_canonico`, `lower_path_de_tres_nodos_encadena_expands` |
| 6 | Un único `Filter` arriba (WHERE + inline), SIN push-down | El lector debe VER la ineficiencia primero: `NodeScan` produce 1M filas para que el filtro deje 1. Es el «antes» exacto que la primera regla del cap. 21 mejora | Push-down ya en el binder: mezcla capas, adelanta el cap. 21 y le roba su razón de existir | Optimizador sin «antes» contra el que medirse = magia indemostrable | MIGRATION-PATTERN §23 decisión 4 y lección 3; test `lower_estructura_del_ejemplo_canonico` (Filter ENCIMA del Expand, comentado) |
| 7 | `CartesianProduct` para patrones disjuntos; `SharedPatternVariables` si comparten | La coma en Cypher ES el producto de los matches; los disjuntos son correctos-pero-ingenuos (el 21 los reordenará). Compartir variables exige un JOIN, maquinaria que no existe: mejor error claro que join a medias | Rechazar la coma: recorta el lenguaje; inventar un join ingenuo ahora: adelanta capítulos y duplica superficie de error | Duplicados de variable interpretados como «lo mismo» sin join ⇒ resultados incorrectos | `lower()` paso 1; tests `lower_patrones_disjuntos_cartesian_product`, `lower_patrones_que_comparten_variables_exigen_join` |
| 8 | `LogicalType` conservador: propiedades → `Any`; `Any`/`Null` comodines | Schemaless (cap. 7): el store no garantiza tipos; prometer lo que nadie garantiza rechaza consultas VÁLIDAS (falsos positivos). `TypeMismatch` sólo cuando es PROBABLE, no posible | Tipado estricto con esquema inferido: `p.edad = TRUE` sería rechazado hoy y quizá sea un Bool mañana | Base de datos que se niega a ejecutar consultas legales | Doc-comments de `LogicalType`/`type_of`; test `lower_where_property_schemaless_pasa` |
| 9 | `eq_compatible` (iguales, numéricos cruzados, comodines) ≠ `order_compatible` (numéricos y strings sólo) | La igualdad es más tolerante que el orden: `Bool`/`Node`/`Edge` NO son ordenables (¿`TRUE < FALSE`?), pero sí comparables por igualdad (`a = b` encuentra self-loops, cap. 20) | Una sola regla «comparable»: o rechaza igualdades legales o acepta órdenes sin sentido | `ORDER BY` imposible o `WHERE a = b` rechazado | `LogicalType::eq_compatible`/`order_compatible`; tests `eq_compatible_reglas`, `order_compatible_reglas` |
| 10 | `Display` canónico: indentación 2 espacios, paréntesis mínimos por precedencia NOT > AND > OR con reglas por contexto | Es la base de `liradb explain` (cap. 21) y de los tests: idempotente, sin ruido, re-parseable. La misma expresión se envuelve distinto según cuelgue de AND/OR/NOT | Paréntesis siempre (`(a AND (b))`): ruido que esconde la forma; nunca: ambigüedad (`a OR b AND c`) | Explains ambiguos que romperían un futuro re-parseo | MIGRATION-PATTERN §23 decisión 9, lección 4 y bug «Or dentro de And»; tests `scalar_display_*`, `plan_display_es_estable_e_idempotente` |
| 11 | `bound_variables()` en orden de ligadura, sin duplicados | Es exactamente lo que el push-down del cap. 21 necesita: saber QUÉ predicados puede mover bajo QUÉ operador (un predicado sólo baja si sus variables ya están ligadas allí) | Sin esa API: el optimizador tendría que re-derivar el binder | Push-down de predicados con variables aún no ligadas ⇒ planes inválidos | `bound_variables`/`collect_bound`; test `integracion_parse_lower_plan_pipeline_completo` |
| 12 | `IndexSeek` declarado aquí, construido por el cap. 21 | El plan lógico debe poder EXPRESAR el uso de índice para que alguien pueda ELEGIRLO; elegir es optimizar, no planificar. Los `ids` llegan ya resueltos (plan «semi-ligado») y el `Display` los oculta (pinta el predicado) | Construirlo en `lower()`: el binder no conoce catálogo/estadísticas; omitirlo: el cap. 21 no tendría dónde aterrizar la regla | Optimizador sin objetivo de reescritura | `LogicalPlan::IndexSeek` (doc-comment); MIGRATION-PATTERN §26 (colateral del cap. 21) |
| 13 | `PlanError { kind, span }` con `write_span_suffix`, `std::error::Error` | Mismo patrón que `QueryError`/`ParseError`: el mensaje localiza la cláusula ofensiva; `Error` lo hace usable con `?`/`Box<dyn Error>` (la CLI del hito ya lo imprime) | `String` de error: imposible de matchear; sin span: «variable no ligada» sin decir cuál ni dónde | Errores de plan no diagnosticables | `PlanError` + tests `plan_error_display_localiza_y_es_std_error`, `plan_error_shared_variables_display` |
| 14 | Tres errores de variables distintos: `DuplicateVariable` / `VariableRebind` / `SharedPatternVariables` | Son tres confusiones distintas: declarar dos veces en todo el MATCH; re-ligar DENTRO del mismo patrón (`(a)-[:X]->(a)`, que es un CICLO: trabajo del executor, cap. 20); compartir ENTRE patrones por coma (que exige join, cap. 21). Cada mensaje nombra el capítulo que lo resolverá | Un `UnknownOrBadVariable` genérico: el usuario no sabe QUÉ hizo mal ni el libro sabe a dónde remitirlo | Ciclos rechazados en el sitio equivocado; joins pedidos al módulo equivocado | `PlanErrorKind`; tests `lower_variable_relidada_en_el_mismo_patron`, `bindings_rechaza_duplicados`, `lower_patrones_que_comparten_variables_exigen_join` |

## 6. Primera solución vs solución evolucionada

- **Ingenua**: un intérprete monolítico que recorre el AST y consulta el store a la vez
  (o, peor, el «plan del brief»: bonito, y semánticamente incompleto).
- **Qué la rompe**: (a) `WHERE x.name = "A"` con `x` sin ligar — se descubre con el grafo a
  medio recorrer, sin un span útil que señalar; (b) el índice del cap. 15 es INVISIBLE para
  un intérprete: no hay árbol que reescribir; (c) el plan del brief omite `f:Person` ⇒
  devuelve conocidos de cualquier etiqueta y NADIE lo nota (el síntoma es «demasiadas
  filas», no un error); (d) cada reordenación futura tocaría sintaxis.
- **Evolución visible en el capítulo**: binder separado (`Bindings` + `build_scalar` con
  spans del AST), lowering en 3 pasos cláusula a cláusula (MATCH → WHERE → RETURN), un solo
  `Filter` conjuntado con `and_all`, `type_of` conservador, y `Display` canónico. Todos los
  límites declarados como `PlanErrorKind` tipados que apuntan al capítulo que los resuelve.

## 7. Prueba de fuego

- **Tests reales del módulo** (se citan, no se duplican): `lower_display_ejemplo_canonico_
  del_brief` y `lower_estructura_del_ejemplo_canonico` (la red que caza al brief: texto
  EXACTO del plan + estructura `And(HasLabel(f,Person), p.name = "Ana")`),
  `lower_match_nodo_solo`, `lower_sin_label_any` (`NodeScan(ANY AS p)`),
  `lower_nodo_anonimo_genera_variable_interna` (`_n1`),
  `lower_dos_patrones_con_anonimos_no_colisionan`,
  `lower_propiedades_inline_bajan_al_filter`,
  `lower_where_y_props_inline_se_conjuntan_en_un_filter`,
  `lower_path_de_tres_nodos_encadena_expands`,
  `lower_direccion_entrante_y_sin_definir` (INCOMING/UNDIRECTED),
  `lower_relacion_con_variable_y_sin_tipo` (`r:KNOWS`, `ANY`),
  `lower_patrones_disjuntos_cartesian_product`,
  `lower_return_alias_y_nombres_derivados`,
  `lower_where_variable_no_ligada` (span del ACCESO, no de la cláusula),
  `lower_return_variable_no_ligada`, `lower_propiedad_inline_variable_no_ligada`,
  `lower_variable_relidada_en_el_mismo_patron`,
  `lower_patrones_que_comparten_variables_exigen_join`, `lower_match_y_return_vacios`
  (ASTs a mano), `lower_where_no_booleano` (`WHERE 3`), `lower_where_igualdad_imposible`
  (`p = TRUE`), `lower_where_orden_imposible` (`TRUE < FALSE`),
  `lower_where_property_schemaless_pasa` (`p.edad = TRUE` OK),
  `lower_where_bool_literal_y_and_sobre_no_bool`, `lower_not_sobre_no_booleano`,
  `lower_return_item_type_checkeado`, `integracion_parse_lower_plan_pipeline_completo`,
  `plan_display_es_estable_e_idempotente`, `plan_error_display_localiza_y_es_std_error`,
  `plan_error_shared_variables_display`.
- **Síntoma si el lector se salta el capítulo**: errores de nombres descubiertos en
  ejecución (o nunca), consultas imposibles de `explain`/optimizar, y — el modo de fallo
  estrella — resultados con filas de más y ningún diagnóstico (exactamente lo que habría
  pasado con el plan del brief).

## 8. Trampas y errores comunes

1. **Confundir AST con plan**: el AST dice lo que PEDISTE; el plan dice CÓMO se calcula.
   Casi todos los errores de diseño posteriores nacen de mezclarlas.
2. **Creer que el `Filter` arriba es un bug del binder**: es el «antes» del cap. 21;
   optimizar aquí mezcla capas.
3. **Creer que `( )` no liga**: liga como `_n1` — el executor necesita nombrarlo todo.
4. **Creer que `Any` = «sin tipo»**: es «no prometido» (schemaless del cap. 7); la
   comparación concreta se resuelve en ejecución.
5. **Confundir los tres errores de variables** (duplicado / re-ligar en el mismo patrón /
   compartir entre patrones): tres correcciones distintas en tres capítulos distintos.
6. **Buscar `HasLabel` en la sintaxis**: no existe — lo fabrica el planner para bajar los
   labels de la cadena al `Filter`.
- **Precisión de lenguaje (glosario)**: *binder* / *binding* / *lowering*; *expresión
  resuelta* (`ScalarExpr`) vs *expresión sintáctica* (`Expression`); *predicado inline*
  (props `{edad: 30}` y labels de cadena) vs *WHERE*; *plan lógico* vs *plan físico*;
  *push-down*; *comodín* (`Any`/`Null`); *variable interna*; *re-binding*; *producto
  cartesiano*; *selectividad* (se nombra para el cap. 21).

## 9. Ejercicios (exercise-designer)

- **recordar/aplicar (esencial — retrieval puro)**: dada
  `MATCH (a:Person)-[:KNOWS]->(b:Person) WHERE a.age > 30 OR b.age > 40 RETURN a, b`,
  redibujar DE MEMORIA el árbol del plan y escribir su `Display` exacto (incluidos los
  paréntesis del `OR` dentro del `AND` y la indentación de 2 espacios). Verificación:
  patrón del test `plan_display_es_estable_e_idempotente`
  (`Filter(b:Person AND (a.age > 30 OR b.age > 40))`). Pistas (≤3): (1) ¿de qué nodo de la
  cadena alimenta el label al `NodeScan` y de cuáles baja al `Filter`?, (2) ¿qué liga más,
  un `AND` o un `OR`, y cuándo necesita paréntesis?, (3) ¿qué pinta `Expand` entre `a` y
  `b`? Criterio: árbol y texto exactos, con `b:Person` en el `Filter` (no en un scan).
- **analizar (intermedio)**: para la consulta estrella, comparar dos planes candidatos —
  el del brief original (sin `f:Person`) y el que produce `lower()` — y explicar cuál
  devuelve filas de más y por qué NADIE lo notaría sin test; después escribir el test
  estructural que lo cazaría (qué `matches!`/`assert_eq!` sobre el predicado). Pistas:
  (1) ¿qué nodo del patrón NO es el inicial?, (2) ¿dónde acaba un label que el `NodeScan`
  no puede absorber?, (3) mira `lower_estructura_del_ejemplo_canonico`. Criterio: identificar
  el predicado ausente Y el modo de fallo silencioso (filas de más, no error).
- **crear (experto)**: implementar `RETURN *` (proyectar TODAS las variables ligadas, en
  orden de declaración): exige usar `Bindings::iter()` en el paso 3 de `lower()` y decidir
  el `output_name()` de cada proyección; escribir el test de display
  (`Project(p, f)` para la consulta estrella). Pistas: (1) ¿qué estructura conserva el
  orden de ligadura?, (2) ¿qué pasa con las variables internas `_n1` — se proyectan?,
  (3) ¿en qué paso de `lower()` viven aún los bindings? Criterio: display exacto, orden de
  declaración, decisión explícita sobre internas, test en el workspace.

## 10. Preguntas abiertas (gancho al capítulo 20)

1. Tenemos un árbol que declara QUÉ calcular: ¿quién lo ejecuta, en qué orden, y qué
   significa «pedir la siguiente fila» a un árbol? (Volcano: open/next/close.)
2. ¿Qué vale `p.edad` cuando `p` no tiene edad, y qué vale `NULL > 30`? (Lógica
   trivalente; el cortocircuito prometido se vuelve observable.)
3. ¿Cuánto cuesta de verdad el `Filter` ingenuo que dejamos arriba? (Las métricas del
   executor numeran la ineficiencia; el cap. 21 la elimina.)
- **Términos nuevos de glosario**: plan lógico, operador, binder, binding, lowering,
  expresión resuelta, predicado inline, push-down, comodín, variable interna, re-binding,
  producto cartesiano, access path, selectividad.

## 11. Diseño de retención (skill `teach`)

- **Retrieval practice**: el esencial obliga a REDIBUJAR de memoria el árbol y el texto
  canónico del plan (nada del enunciado regala la estructura ni los paréntesis); el
  intermedio exige recordar el bug del brief y su cura sin releer la sección.
- **Spacing**: esencial → precedencia y `Display` de expresiones (caps. 17-18) y reglas de
  paréntesis; intermedio → `Span` y mensajes localizados (cap. 18), labels schemaless
  (cap. 7); experto → `Bindings` (este cap.) + `Value`/`CompareOp` (caps. 7/17); la sección
  «¿y el índice?» re-ejercita `HashIndex`/`BPlusTree` (cap. 15).
- **Interleaving**: el esencial mezcla sintaxis (precedencia del parser), semántica de
  patrones (labels) y pretty-printing; el experto cruza resolución de nombres con diseño
  de API pública.
- **Dificultad asimétrica**: cada sección introduce UNA idea nueva (ligar / resolver /
  bajar / tipar / imprimir); los ejercicios exigen recuperación sin pistas en el enunciado.
- **Bucle de feedback inmediato**: `cargo test -p vol2-liradb` (40 tests del módulo,
  citados por nombre).
- **Citas**: Selinger et al., «Access Path Selection in a RDBMS», SIGMOD 1979 (ACM DL
  10.1145/582095.582099); Graefe, «Volcano—An Extensible and Parallel Query Evaluation
  System», IEEE TKDE 1994; Graefe & McKenna, «The Volcano Optimizer Generator», VLDB 1993;
  Armbrust et al., «Spark SQL: Relational Data Processing in Spark», SIGMOD 2015
  (Catalyst); PostgreSQL docs (EXPLAIN VERBOSE); Kùzu docs (arquitectura Parser→Binder→
  Planner); Neo4j manual (execution plans); Ramakrishnan & Gehrke caps. de query
  processing; CMU 15-445.

---

## Checklist de profundidad

- [x] Cada decisión técnica tiene su «porqué» con alternativa descartada y fuente (14 en la tabla §5).
- [x] Escenario de fallo visible: variable no ligada detectada AQUÍ con span (no en ejecución) + el plan que escanea todo cuando existiría índice + el bug del brief (filas de más en silencio).
- [x] Código ejecutable en workspace (40 tests) citado por nombre, no duplicado.
- [x] Misconcepciones corregidas explícitamente (AST ≠ plan; plan bonito ≠ correcto; `()` sí liga; `Any` ≠ sin tipo; Filter arriba ≠ descuido; planificar ≠ optimizar).
- [x] Ejercicios con solución verificable (`cargo test -p vol2-liradb`).
- [x] ≥1 ejercicio de retrieval (redibujar el plan de memoria) y ≥1 de spacing (precedencia caps. 17-18, Value cap. 7, índices cap. 15).
- [x] Responde la pregunta crítica de CORPUS.yml (`vol-II-cap-19`): «Cómo representar variables ligadas, patrones y predicados» — `Bindings` + `LogicalPlan` + `ScalarExpr::HasLabel`/`Compare`.
- [x] Anécdota verificada: Selinger et al. 1979 (System R, IBM San José; SIGMOD '79; ACM DL), con la precisión de que el vocabulario lógico/físico maduró con Volcano/Cascadas.
