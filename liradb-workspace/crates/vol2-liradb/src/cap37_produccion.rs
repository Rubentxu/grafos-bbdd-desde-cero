//! Vol.II — Cap.37: Qué necesitaría una base de datos de producción.
//!
//! Abre la Parte VIII con la pregunta que el mapa del cap. 36 no responde:
//! ¿qué falta para vivir FUERA del laboratorio? La respuesta del libro NO es
//! implementar seguridad ni backups (eso sería teatro sin modelo de
//! despliegue): es NOMBRAR las once dimensiones de producción del brief,
//! CLASIFICARLAS contra la realidad verificable del workspace y señalar en
//! qué punto del hexágono se enchufaría cada una. La tesis es la de Gawande
//! («The Checklist Manifesto», 2009): ir a producción no es acumular
//! features, es pasar una lista de comprobación ítem a ítem — y aquí esa
//! lista vive COMPILADA, heredera directa de [`crate::informe_acid`] (27):
//! documentación que no puede mentir porque los tests la pinan.
//!
//! Criterio de graduación (aplicado sin excepciones):
//! - `Existe`: cadena completa demostrada fin a fin. HOY: ninguna.
//! - `Parcial`: símbolo citable + test verde + frontera DOCUMENTADA.
//! - `Ausente`: búsqueda fallida documentable (no hay símbolo).
//!
//! Recuento honesto al cerrar este capítulo: **0 Existe · 6 Parcial ·
//! 5 Ausente**. Ni una sola dimensión llega a Existe — y eso es una BUENA
//! noticia operativa: convierte «no sé si puedo» en un inventario accionable.
//!
//! - [`EstadoProduccion`] / [`BloqueProduccion`] / [`DimensionProduccion`] /
//!   [`InformeProduccion`] / [`informe_produccion`] — la lista ejecutable.
//!
//! Este módulo NO implementa producción (decisión de diseño §5.1 del
//! contrato): clasifica, cita evidencia y enchufa-en-el-mapa. Los caps.
//! 38-40 elegirán tres frentes técnicos; las dimensiones restantes quedan
//! como agenda honesta.

use core::fmt;

// ─────────────────── El vocabulario de la lista ───────────────────

/// Los tres estados de una dimensión de producción.
///
/// Hermano graduado de [`crate::NivelGarantia`] (cap. 27): «tiene/no tiene»
/// oculta justo lo que este capítulo enseña (las fronteras). Un booleano
/// abriría además la puerta a puntuaciones-marketing; tres estados obligan a
/// justificar cada matiz con evidencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoProduccion {
    /// Cadena completa demostrada fin a fin. Ninguna dimensión la alcanza
    /// hoy: proclamar un Existe amable sería LA falsedad que el libro prohíbe.
    Existe,
    /// Hay algo real y citable (símbolo + test verde) con una frontera
    /// DOCUMENTADA que dice hasta dónde llega.
    Parcial,
    /// No hay símbolo que citar: la búsqueda fallida es la evidencia.
    /// Documentarla es lo que distingue un hueco de una suposición.
    Ausente,
}

impl EstadoProduccion {
    /// La marca del checklist pre-vuelo: `[X]` / `[~]` / `[ ]`.
    pub fn marca(self) -> &'static str {
        match self {
            EstadoProduccion::Existe => "[X]",
            EstadoProduccion::Parcial => "[~]",
            EstadoProduccion::Ausente => "[ ]",
        }
    }

    /// Nombre en minúscula para la línea del informe.
    pub fn nombre(self) -> &'static str {
        match self {
            EstadoProduccion::Existe => "existe",
            EstadoProduccion::Parcial => "parcial",
            EstadoProduccion::Ausente => "ausente",
        }
    }
}

impl fmt::Display for EstadoProduccion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.nombre())
    }
}

/// El bloque del checklist al que pertenece una dimensión.
///
/// Agrupación de LECTURA (chunking de Gawande para recordar 11 ítems), no
/// reordenación: [`InformeProduccion::entradas`] conserva el orden EXACTO
/// del brief; sólo el [`fmt::Display`] agrupa por bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BloqueProduccion {
    /// Que los DATOS sobrevivan (tiempo, accidente, miradas).
    Datos,
    /// QUE EL PROCESO opere con cabeza (adversidad, exceso, opacidad).
    Proceso,
    /// Que las PERSONAS respondan por él (amenaza, mínimo privilegio).
    Personas,
}

impl BloqueProduccion {
    /// Cabecera del bloque tal y como aparece en el informe impreso.
    pub fn titulo(self) -> &'static str {
        match self {
            BloqueProduccion::Datos => "DATOS — que los datos sobrevivan:",
            BloqueProduccion::Proceso => "PROCESO — que el proceso opere con cabeza:",
            BloqueProduccion::Personas => "PERSONAS — que las personas respondan por él:",
        }
    }
}

impl fmt::Display for BloqueProduccion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.titulo())
    }
}

/// Una fila de la lista: la dimensión, su estado HONESTO, la evidencia,
/// quién lo resuelve en la industria y dónde se enchufaría en LiraDB.
///
/// Los campos de referencia viajan DENTRO del artefacto (no en un apartado
/// de prosa aparte) para que sean compilables, grepeables y auditables: si
/// mañana cambia un módulo citado, el test que lee esta estructura grita.
#[derive(Debug, Clone)]
pub struct DimensionProduccion {
    /// Nombre de la dimensión, idéntico al del brief (orden 1:1).
    pub nombre: &'static str,
    /// Bloque de lectura del checklist (agrupa el Display, no los datos).
    pub bloque: BloqueProduccion,
    /// El estado VERIFICADO contra el código, no el deseado.
    pub estado: EstadoProduccion,
    /// Qué hay HOY, con símbolos citables y frontera explícita.
    pub como_esta_hoy: &'static str,
    /// Referencia industrial CON mecanismo (PostgreSQL/SQLite/SQLCipher/…).
    pub quien_lo_resuelve: &'static str,
    /// Módulo/puerto del hexágono (caps. 8-36) donde se enchufaría, o «—».
    pub donde_se_enchufaria: &'static str,
}

/// El informe de producción completo: las ONCE dimensiones del brief.
///
/// Artefacto EJECUTABLE, mismo espíritu que [`crate::InformeAcid`]: los
/// tests de este módulo lo verifican para que la lista no pueda prometer
/// más de lo que el código cumple (ni menos: actualizar una clasificación
/// exige tocar el test-pinzón JUNTOS).
#[derive(Debug, Clone)]
pub struct InformeProduccion {
    entradas: Vec<DimensionProduccion>,
}

impl InformeProduccion {
    /// Las once entradas EN EL ORDEN EXACTO DEL BRIEF (fidelidad 1:1;
    /// el agrupado por bloques es cosa del Display).
    pub fn entradas(&self) -> &[DimensionProduccion] {
        &self.entradas
    }

    /// Busca una dimensión por su nombre exacto (el del brief).
    pub fn por_nombre(&self, nombre: &str) -> Option<&DimensionProduccion> {
        self.entradas.iter().find(|d| d.nombre == nombre)
    }

    /// `(existe, parcial, ausente)` — el recuento honesto de la casa.
    ///
    /// Devuelve tupla y no un score numérico: sumar honestidades produce
    /// marketing, no conocimiento (misma razón por la que el cap. 27 no
    /// dio nunca un «índice ACID»).
    pub fn recuento(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for d in &self.entradas {
            match d.estado {
                EstadoProduccion::Existe => c.0 += 1,
                EstadoProduccion::Parcial => c.1 += 1,
                EstadoProduccion::Ausente => c.2 += 1,
            }
        }
        c
    }
}

impl fmt::Display for InformeProduccion {
    /// La lista previa al vuelo: tres bloques, once líneas, leyenda y
    /// recuento. Cada entrada imprime su marca `[X]/[~]/[ ]` — las mismas
    /// que [`EstadoProduccion::marca`] usa en el checklist del capítulo.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Lista de comprobación previa a «producción» — LiraDB Lite:"
        )?;
        writeln!(f)?;
        for bloque in [
            BloqueProduccion::Datos,
            BloqueProduccion::Proceso,
            BloqueProduccion::Personas,
        ] {
            writeln!(f, "{}", bloque.titulo())?;
            for d in self.entradas.iter().filter(|d| d.bloque == bloque) {
                writeln!(
                    f,
                    "  {} {} — {}: {}",
                    d.estado.marca(),
                    d.nombre,
                    d.estado.nombre(),
                    d.como_esta_hoy
                )?;
                writeln!(f, "      Quién lo resuelve: {}", d.quien_lo_resuelve)?;
                writeln!(f, "      Dónde se enchufaría: {}", d.donde_se_enchufaria)?;
            }
        }
        let (x, p, a) = self.recuento();
        writeln!(f)?;
        writeln!(f, "Recuento: {x} existen · {p} parciales · {a} ausentes")?;
        write!(f, "Leyenda: [X]=existe   [~]=parcial   [ ]=ausente")
    }
}

// ─────────────────── La lista, entrada a entrada ───────────────────

/// El estado de producción de LiraDB Lite verificado contra el workspace.
///
/// Las once dimensiones del brief EN SU ORDEN 1:1, cada una clasificada con
/// el criterio del módulo (§ criterio de graduación). Estados verificados
/// contra el código 2026-08-25; el test-pinzón
/// `produccion_es_honesto_sobre_el_estado_actual` los PINA: cambiar uno sin
/// tocar ese test rompe la build A PROPÓSITO — la lección del cap. 27
/// («informe_acid() no puede mentir») hecha cerrojo.
///
/// ```text
/// BLOQUE 1 · QUE LOS DATOS SOBREVIVAN
///   [~] 1. Compatibilidad de formatos  magic+versión (9) · quien-abre-compara
///   [~] 2. Migraciones                 FORMAT_VERSION sí · evolución versionada no
///   [ ] 3. Cifrado                     nada · SQLCipher/pgcrypto existen fuera
///   [~] 4. Copias de seguridad         WAL→fichero+replay (28-29) · sin fsync garantizado
/// BLOQUE 2 · QUE EL PROCESO OPERE CON CABEZA
///   [~] 5. Control de recursos         Presupuesto (26) para algoritmos · nada global
///   [ ] 6. Protección ante consultas   sin timeout/cancel · caso vivo: catálogo ~224 s
///   [~] 7. Telemetría                  Contadores+spans (35) en proceso · sin export
///   [~] 8. Herramientas operativas     CLI (31) germinal · sin init/status/backup
/// BLOQUE 3 · QUE LAS PERSONAS RESPONDAN POR ÉL
///   [ ] 9.  Seguridad                  sin modelo de amenaza ni hardening escrito
///   [ ] 10. Autenticación              no hay usuarios · MongoDB 2017 lo pagó caro
///   [ ] 11. Autorización               sin roles ni GRANT por operación
/// ```
pub fn informe_produccion() -> InformeProduccion {
    InformeProduccion {
        entradas: vec![
            DimensionProduccion {
                nombre: "Compatibilidad de formatos",
                bloque: BloqueProduccion::Datos,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "magic + FORMAT_VERSION en la cabecera (cap. 9) y política \
                                «quien abre compara»: decode_header rechaza un magic \
                                corrupto y devuelve la versión leída; los roundtrips \
                                CSV/JSONL/GraphML (32-33) comprueban ida y vuelta. Falta \
                                la otra mitad: no existe v2, así que comparar nunca ha \
                                tenido que EVOLUCIONAR nada todavía",
                quien_lo_resuelve: "SQLite lo promete POR DÉCADAS (file-format stability \
                                    promise, sqlite.org/fileformat2.html); PostgreSQL \
                                    compara la versión del catálogo de datos al arrancar \
                                    y se niega a abrir un data directory anterior",
                donde_se_enchufaria: "encoding (cap. 9): FORMAT_VERSION ya vive ahí; todo \
                                      cambio de formato futuro pasa por encode_header/decode_header",
            },
            DimensionProduccion {
                nombre: "Seguridad",
                bloque: BloqueProduccion::Personas,
                estado: EstadoProduccion::Ausente,
                como_esta_hoy: "no hay modelo de amenaza ESCRITO ni hardening documentado: \
                                entre el contenido y otro proceso del mismo usuario sólo \
                                están los permisos del fichero (FilePager abre sin más, \
                                cap. 12). Embedded cambia la amenaza —sin listener de \
                                red—, no la elimina",
                quien_lo_resuelve: "PostgreSQL documenta SU modelo completo (roles, \
                                    pg_hba.conf, SSL) en postgresql.org/docs; SQLite delega \
                                    en el sistema de ficheros y lo DECLARA explícitamente",
                donde_se_enchufaria: "—: ningún punto del hexágono lo representa hoy; \
                                      empezaría como documento de amenazas junto a la CLI (31)",
            },
            DimensionProduccion {
                nombre: "Autenticación",
                bloque: BloqueProduccion::Personas,
                estado: EstadoProduccion::Ausente,
                como_esta_hoy: "no existe el concepto de usuario: ninguna firma del crate \
                                menciona credenciales, sesiones o contraseñas; quien puede \
                                abrir el proceso ES la base de datos entera",
                quien_lo_resuelve: "PostgreSQL: CREATE ROLE + pg_hba.conf; MongoDB cambió el \
                                    bind por defecto a localhost y activó auth tras el \
                                    ransomware de enero de 2017 (serie 3.6); SQLite declara \
                                    que NO tiene autenticación embebida",
                donde_se_enchufaria: "CLI/API conductora (31): la identidad llega antes de \
                                      la primera consulta; el motor jamás la necesitó \
                                      porque sólo hubo un dueño",
            },
            DimensionProduccion {
                nombre: "Autorización",
                bloque: BloqueProduccion::Personas,
                estado: EstadoProduccion::Ausente,
                como_esta_hoy: "sin roles ni GRANT: toda operación vale lo mismo para \
                                cualquiera — put_node/delete_edge del puerto (8) no \
                                transportan QUIÉN pregunta, así que no hay dónde decidir",
                quien_lo_resuelve: "PostgreSQL: GRANT/REVOKE por objeto y operación \
                                    (postgresql.org/docs); Neo4j administra roles por \
                                    comando (neo4j.com/docs)",
                donde_se_enchufaria: "puerto GraphStore (8)/Executor (20): cada operación \
                                      pasaría por una decisión permitir/denegar; hoy el \
                                      puerto no lleva identidad",
            },
            DimensionProduccion {
                nombre: "Cifrado",
                bloque: BloqueProduccion::Datos,
                estado: EstadoProduccion::Ausente,
                como_esta_hoy: "las páginas van PLANAS al disco: FilePager::write_page \
                                escribe bytes tal cual (12) y guardar_wal también (29); \
                                un dump del fichero revela labels y propiedades enteras",
                quien_lo_resuelve: "SQLCipher cifra página a página sobre SQLite \
                                    (sqlcipher.net); PostgreSQL ofrece pgcrypto POR COLUMNA — \
                                    el core NO trae TDE nativo (postgresql.org/docs)",
                donde_se_enchufaria: "bajo el puerto Pager (12): cifrar página a página \
                                      ahí sería transparente para TODO lo construido encima",
            },
            DimensionProduccion {
                nombre: "Copias de seguridad",
                bloque: BloqueProduccion::Datos,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "la mitad buena: guardar_wal persiste el log a fichero y \
                                reabrir ejecuta replay COMPLETO (28-29) — ¡la mitad de un \
                                PITR! La frontera dura: guardar_wal NO hace fsync (su \
                                doc-comment lo confiesa: std::fs::write vuelca cuando el \
                                sistema quiere), aunque FilePager::sync SÍ llama sync_all \
                                (12); tampoco hay checkpoint independiente del store de \
                                datos ni .backup estilo SQLite",
                quien_lo_resuelve: "pg_basebackup + archivado del WAL = PITR en PostgreSQL \
                                    (postgresql.org/docs); SQLite trae Online Backup API y \
                                    CLI .backup (sqlite.org)",
                donde_se_enchufaria: "recuperación (28-29) + Pager (12): sync GARANTIZADO \
                                      al guardar el WAL y un checkpoint de páginas serían \
                                      el resto del PITR",
            },
            DimensionProduccion {
                nombre: "Migraciones",
                bloque: BloqueProduccion::Datos,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "sólo la POLÍTICA de versión: FORMAT_VERSION == 1 (9) y \
                                «quien abre compara». No hay camino versionado v1→v2 ni \
                                herramienta que transforme un fichero antiguo: la primera \
                                evolución real del formato tendrá que inventar el paso \
                                «leer versión vieja → transformar → escribir nueva»",
                quien_lo_resuelve: "Flyway/Liquibase versionan esquemas SQL (flywaydb.org, \
                                    liquibase.org); Diesel migrations hace lo propio en Rust \
                                    (diesel.rs)",
                donde_se_enchufaria: "encoding (9): el número ya RESERVA el sitio; falta el \
                                      paso de transformación entre versiones",
            },
            DimensionProduccion {
                nombre: "Control de recursos",
                bloque: BloqueProduccion::Proceso,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "Presupuesto acota profundidad/nodos/lecturas de los \
                                ALGORITMOS de grafo con MotivoParada explícito (26) — pero \
                                es opt-in y sólo cubre recorridos: el Executor (20) corre \
                                sin límite y la evidencia vivida manda: Catalog::collect \
                                cuadrático corrió ~224 s sin que NADA lo detuviera",
                quien_lo_resuelve: "statement_timeout y pg_cancel_backend cortan consultas \
                                    en PostgreSQL (postgresql.org/docs); PRAGMA \
                                    soft_heap_limit acota memoria en SQLite",
                donde_se_enchufaria: "Executor (20): la idea del presupuesto ya existe; \
                                      falta hacerla valer para TODA consulta, no sólo para \
                                      quien la pide",
            },
            DimensionProduccion {
                nombre: "Protección ante consultas costosas",
                bloque: BloqueProduccion::Proceso,
                estado: EstadoProduccion::Ausente,
                como_esta_hoy: "explain ESTIMA cardinalidades (estimate, 21) pero nada \
                                CORTA: ni timeout, ni cancelación, ni guardia de coste. El \
                                caso vivo: el catálogo cuadrático (~224 s frente a 281 ms) \
                                lo cazó un humano con benchmark, no un mecanismo",
                quien_lo_resuelve: "statement_timeout ES el mecanismo industrial: matar la \
                                    consulta al vencimiento (postgresql.org/docs); \
                                    max_execution_time juega el mismo papel en MySQL",
                donde_se_enchufaria: "Executor (20): un reloj por next() del operador raíz \
                                      sería el corte mínimo; estimate (21) ya calcula el \
                                      coste para decidir ANTES de ejecutar",
            },
            DimensionProduccion {
                nombre: "Telemetría",
                bloque: BloqueProduccion::Proceso,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "Contadores con formato de exposición Prometheus (# TYPE … \
                                counter), ExecMetrics desde el cap. 20, spans por operador \
                                y liradb --profile (35). Frontera: TODO muere en el proceso \
                                — no hay export continuo a backend alguno; Prometheus queda \
                                IMITADO, no conectado",
                quien_lo_resuelve: "Prometheus define el exposition format imitado aquí a \
                                    mano (prometheus.io); OpenTelemetry estandariza \
                                    traces/metrics/logs (opentelemetry.io)",
                donde_se_enchufaria: "decoradores medidores (35): el mismo punto de tejido \
                                      serviría para EXPORTAR además de imprimir",
            },
            DimensionProduccion {
                nombre: "Herramientas operativas",
                bloque: BloqueProduccion::Proceso,
                estado: EstadoProduccion::Parcial,
                como_esta_hoy: "liradb-cli con demo/query/explain/repl/script/import/export \
                                (--graph/--plan/--stats/--profile, 31) más inspect/check/\
                                compact (16). Germinal: no hay init/status/backup ni modo \
                                daemon — la base de datos vive mientras vive el proceso que \
                                la abrió",
                quien_lo_resuelve: "sqlite3 CLI (.backup/.dump/.schema) es el precedente \
                                    embebido (sqlite.org); neo4j-admin dump/load y pg_ctl \
                                    status en servidores completos",
                donde_se_enchufaria: "CLI (31): cada subcomando nuevo es una entrada del \
                                      checklist convertida en herramienta",
            },
        ],
    }
}

// ─────────────────── Los tests de honestidad ───────────────────

#[cfg(test)]
mod tests_produccion {
    use super::*;
    use crate::{
        AntesImagenes, Checkpoint, Contadores, Edge, FORMAT_VERSION, GraphStore, MemoryStore, Node,
        Presupuesto, Wal, WalTransaccion, cargar_wal, decode_header, encode_header, guardar_wal,
        reabrir,
    };

    #[test]
    fn produccion_cubre_las_once_dimensiones_del_brief() {
        let informe = informe_produccion();
        // Orden 1:1 con el brief — nada inventado, nada omitido.
        let nombres_brief: [&str; 11] = [
            "Compatibilidad de formatos",
            "Seguridad",
            "Autenticación",
            "Autorización",
            "Cifrado",
            "Copias de seguridad",
            "Migraciones",
            "Control de recursos",
            "Protección ante consultas costosas",
            "Telemetría",
            "Herramientas operativas",
        ];
        let entradas = informe.entradas();
        assert_eq!(entradas.len(), 11);
        for (i, nombre) in nombres_brief.iter().enumerate() {
            assert_eq!(entradas[i].nombre, *nombre, "posición {i} del brief");
        }
        // Bloques de lectura: 4 datos + 4 proceso + 3 personas.
        let bloques: Vec<_> = entradas.iter().map(|d| d.bloque).collect();
        assert_eq!(
            bloques,
            [
                BloqueProduccion::Datos,
                BloqueProduccion::Personas,
                BloqueProduccion::Personas,
                BloqueProduccion::Personas,
                BloqueProduccion::Datos,
                BloqueProduccion::Datos,
                BloqueProduccion::Datos,
                BloqueProduccion::Proceso,
                BloqueProduccion::Proceso,
                BloqueProduccion::Proceso,
                BloqueProduccion::Proceso,
            ]
        );
        // Toda entrada lleva evidencia, referencia industrial y enchufe.
        for d in entradas {
            assert!(d.como_esta_hoy.len() > 60, "{} sin evidencia", d.nombre);
            assert!(!d.quien_lo_resuelve.is_empty());
            assert!(!d.donde_se_enchufaria.is_empty());
        }
        assert!(informe.por_nombre("Cifrado").is_some());
        assert!(informe.por_nombre("No existe").is_none());
    }

    /// EL TEST-PINZÓN. Pina los once estados Y verifica cada Parcial contra
    /// el código REAL (sondas ejecutables). El cerrojo funciona en DOS
    /// direcciones: (a) si alguien mejora el informe (p.ej. Copias de
    /// seguridad → Existe) sin tocar este test, el assert_exacto se pone
    /// ROJO hasta que la cadena completa esté demostrada; (b) si alguien
    /// implementa algo nuevo sin actualizar el informe, las sondas dejan de
    /// reflejar la realidad y el informe mentirá — el test fuerza que
    /// informe y código evolucionen JUNTOS, como informe_acid() desde el 27.
    #[test]
    fn produccion_es_honesto_sobre_el_estado_actual() {
        let informe = informe_produccion();
        // ── Los once estados, pinados uno a uno. ──
        let esperados: [(&str, EstadoProduccion); 11] = [
            ("Compatibilidad de formatos", EstadoProduccion::Parcial),
            ("Seguridad", EstadoProduccion::Ausente),
            ("Autenticación", EstadoProduccion::Ausente),
            ("Autorización", EstadoProduccion::Ausente),
            ("Cifrado", EstadoProduccion::Ausente),
            ("Copias de seguridad", EstadoProduccion::Parcial),
            ("Migraciones", EstadoProduccion::Parcial),
            ("Control de recursos", EstadoProduccion::Parcial),
            (
                "Protección ante consultas costosas",
                EstadoProduccion::Ausente,
            ),
            ("Telemetría", EstadoProduccion::Parcial),
            ("Herramientas operativas", EstadoProduccion::Parcial),
        ];
        for (nombre, estado) in esperados {
            assert_eq!(
                informe.por_nombre(nombre).expect(nombre).estado,
                estado,
                "{nombre}: el informe cambió — actualiza este pinzón con la \
                 evidencia nueva, o revierte la clasificación"
            );
        }
        // ── Sondeables CONTRA el código real (los Parcial citables): ──
        // Formato=Parcial porque la política «quien abre compara» EXISTE:
        // encode_header sella magic+versión y decode_header devuelve la
        // versión — y rechaza un magic corrupto.
        let cabecera = encode_header();
        assert_eq!(decode_header(&cabecera).unwrap(), FORMAT_VERSION);
        let mut rota = cabecera;
        rota[0] ^= 0xFF;
        assert!(decode_header(&rota).is_err());
        // Backups=Parcial porque guardar_wal→cargar_wal funciona y los
        // contadores sobreviven al fichero (¡y NO fsync: frontera citada!).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pinza.wal");
        let mut store = MemoryStore::new();
        let mut wal = Wal::new();
        let mut tx = WalTransaccion::begin(&mut store, &mut wal);
        tx.put_node(Node::new(0, "Person")).unwrap();
        tx.commit().unwrap();
        guardar_wal(&wal, &path).unwrap();
        let recargado = cargar_wal(&path).unwrap();
        assert_eq!(Checkpoint::tomar(&recargado), Checkpoint::tomar(&wal));
        // Control de recursos=Parcial porque Presupuesto existe y acota
        // (opt-in): construible con límites concretos.
        let presupuesto = Presupuesto::default().con_profundidad(3).con_nodos(10);
        assert_eq!(presupuesto.max_profundidad, Some(3));
        assert_eq!(presupuesto.max_nodos, Some(10));
        // Telemetría=Parcial porque Contadores habla formato Prometheus.
        assert!(
            Contadores::new()
                .to_string()
                .contains("# TYPE queries_total counter")
        );
    }

    /// Nivel-compilación: los símbolos que los Parcial CITAN existen, son
    /// usables y dicen lo que el informe afirma de ellos. Si mañana se
    /// renombra o retira uno, esta prueba NO compila — la cita muerta es
    /// imposible de esconder.
    #[test]
    fn produccion_parciales_citan_simbolos_reales() {
        // Migraciones/Formato: la versión del formato vale UNO hoy.
        assert_eq!(FORMAT_VERSION, 1);
        // Control de recursos: Presupuesto construible con sus builders.
        let presupuesto = Presupuesto::default()
            .con_profundidad(1)
            .con_nodos(5)
            .con_lecturas(7);
        assert_eq!(presupuesto.max_lecturas, Some(7));
        // Telemetría: nueve métricas expuestas con `# TYPE`.
        let texto = Contadores::new().to_string();
        assert_eq!(texto.matches("# TYPE").count(), 9);
        assert!(texto.contains("# TYPE page_reads counter"));
        // Backups: guardar_wal/reabrir usables FIN A FIN (el arranque
        // completo de una sesión a otra — la mitad honesta de un PITR).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("simbolos.wal");
        {
            let mut store = MemoryStore::new();
            let mut wal = Wal::new();
            let mut tx = WalTransaccion::begin(&mut store, &mut wal);
            tx.put_node(Node::new(0, "Person")).unwrap();
            tx.put_node(Node::new(1, "City")).unwrap();
            tx.put_edge(Edge::new(0, 0, 1, "LIVES_IN")).unwrap();
            tx.commit().unwrap();
            guardar_wal(&wal, &path).unwrap();
        }
        let mut store = MemoryStore::new();
        let informe = reabrir(&mut store, &path, &AntesImagenes::new()).unwrap();
        assert_eq!(informe.transacciones_ganadoras, 1);
        assert_eq!(store.node_count(), 2);
        assert_eq!(store.edge_count(), 1);
    }

    /// Ninguna dimensión llega a Existe — y ésa es la tesis: proclamar un
    /// «Existe» amable exigiría CADENA COMPLETA demostrada fin a fin
    /// (criterio del módulo), y no la hay ni en el formato (sin evolución
    /// versionada real) ni en backups (sin fsync garantizado ni checkpoint
    /// de páginas). Si alguien marca una dimensión Existe sin construir esa
    /// cadena, ESTE test se pone rojo antes de que la mentira llegue a prosa.
    #[test]
    fn produccion_ninguna_dimension_existe_sin_cadena_completa() {
        let informe = informe_produccion();
        let (existe, parcial, ausente) = informe.recuento();
        assert_eq!((existe, parcial, ausente), (0, 6, 5));
        assert!(
            informe
                .entradas()
                .iter()
                .all(|d| d.estado != EstadoProduccion::Existe)
        );
    }

    /// El Display ES el checklist: tres bloques, once líneas marcadas,
    /// leyenda y recuento 0·6·5 — el artefacto que la prosa reproduce.
    #[test]
    fn produccion_display_tres_bloques_once_lineas() {
        let texto = informe_produccion().to_string();
        assert!(texto.contains("Lista de comprobación previa"));
        assert!(texto.contains(BloqueProduccion::Datos.titulo()));
        assert!(texto.contains(BloqueProduccion::Proceso.titulo()));
        assert!(texto.contains(BloqueProduccion::Personas.titulo()));
        // Once líneas de entrada (una por dimensión, con su marca).
        let lineas_entrada: Vec<&str> = texto.lines().filter(|l| l.starts_with("  [")).collect();
        assert_eq!(lineas_entrada.len(), 11);
        // Marcas coherentes con los estados: seis [~], cinco [ ], cero [X]
        // (contadas SÓLO en líneas de entrada: la leyenda repite las marcas).
        let marcadas = lineas_entrada.concat();
        assert_eq!(marcadas.matches("[~]").count(), 6);
        assert_eq!(marcadas.matches("[ ]").count(), 5);
        assert_eq!(marcadas.matches("[X]").count(), 0);
        assert!(texto.contains("Recuento: 0 existen · 6 parciales · 5 ausentes"));
        assert!(texto.contains("Leyenda: [X]=existe   [~]=parcial   [ ]=ausente"));
    }
}
