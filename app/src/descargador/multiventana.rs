//! Conexión de hasta tres pestañas. El contexto de las bitácoras y la evidencia
//! visible permanecen en el puente principal; cada lote compatible reparte consultas enteras.
use super::{
    flujo_csmp, lote, paquete, piloto, Alcance, Catalogo, Edicion, Perfil, PlanLote, Puente,
    Seleccion, TODAS_LAS_MEDIDAS,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::time::Duration;

pub struct Conexion {
    pub principal: Arc<Puente>,
    pub adicionales: [Arc<Puente>; 2],
}

impl lote::Conexion for Conexion {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        self.principal.llamar(comando, payload, plazo)
    }

    fn habilitar_lecturas(&self, integral: bool) {
        self.principal.habilitar_lecturas(integral);
    }

    fn soporta_search_and_download(&self) -> bool {
        self.principal.soporta_search_and_download()
    }

    fn ejecutar_distribuido(
        &self,
        solicitud: &lote::Solicitud<'_>,
        emitir: &mut dyn FnMut(lote::Evento),
    ) -> Option<Result<Vec<Value>, String>> {
        let puentes = [&self.principal, &self.adicionales[0], &self.adicionales[1]];
        let puentes = &puentes[..solicitud.plan.consultas.len().min(puentes.len())];
        let planes = repartir(solicitud.plan, puentes.len());
        match preparar(puentes, &planes, solicitud) {
            Ok(canales) => Some(ejecutar_preparado(&canales, &planes, solicitud, emitir)),
            Err(e) if solicitud.cancelar.load(Ordering::SeqCst) => Some(Err(e)),
            Err(e) => {
                emitir(lote::Evento::Aviso(format!(
                    "{e} Se continúa con la pestaña principal."
                )));
                None
            }
        }
    }
}

pub(super) type Canal<'a> = (&'a (dyn lote::Conexion + Sync), Catalogo);

pub(super) fn repartir(plan: &PlanLote, n: usize) -> Vec<PlanLote> {
    let mut partes = vec![
        PlanLote {
            consultas: Vec::new(),
            fechas: plan.fechas
        };
        n
    ];
    for (i, consulta) in plan.consultas.iter().enumerate() {
        partes[i % n].consultas.push(consulta.clone());
    }
    partes
}

fn seleccion(p: &Perfil) -> Seleccion {
    let mut s = Seleccion {
        tribunal: p.tribunal.clone(),
        ..Default::default()
    };
    match &p.alcance {
        Alcance::Seguimiento {
            pestana,
            modalidad,
            reporte,
        } => {
            s.pestana.clone_from(pestana);
            s.modalidad.clone_from(modalidad);
            s.reporte = reporte.clone().unwrap_or_else(|| TODAS_LAS_MEDIDAS.into());
        }
        Alcance::Litigantes { estado } => s.estado.clone_from(estado),
        Alcance::Calendario { mes, anio } => {
            s.mes.clone_from(mes);
            s.anio.clone_from(anio);
        }
        Alcance::Carga { desde, hasta } => {
            s.desde = Some(*desde);
            s.hasta = Some(*hasta);
        }
    }
    s
}

fn preparar<'a>(
    puentes: &[&'a Arc<Puente>],
    partes: &[PlanLote],
    solicitud: &lote::Solicitud<'_>,
) -> Result<Vec<Canal<'a>>, String> {
    let mut documentos = BTreeSet::new();
    for (i, p) in puentes.iter().enumerate() {
        if solicitud.cancelar.load(Ordering::SeqCst) {
            return Err("Descarga cancelada.".into());
        }
        if !p.conectado() {
            return Err(format!("La pestaña {} está desconectada.", i + 1));
        }
        // Comprobar documentos distintos antes de navegar en una pestaña auxiliar.
        p.llamar("catalog", json!({}), Duration::from_secs(10))
            .map_err(|e| format!("Pestaña {} · {e}", i + 1))?;
        let caps = p
            .llamar(
                "capabilities",
                json!({"documento_piloto": true}),
                Duration::from_secs(10),
            )
            .map_err(|e| format!("Pestaña {} · {e}", i + 1))?;
        piloto::registrar_documento(&caps, &mut documentos)?;
    }
    let mut canales = Vec::with_capacity(puentes.len());
    for (i, (&p, parte)) in puentes.iter().zip(partes).enumerate() {
        if solicitud.cancelar.load(Ordering::SeqCst) {
            return Err("Descarga cancelada.".into());
        }
        let catalogo = p
            .catalogo(Edicion::Integral, solicitud.catalogo.pantalla)
            .map_err(|e| format!("Pestaña {} · {e}", i + 1))?;
        if parte.consultas.iter().any(|q| {
            catalogo
                .preparar(Edicion::Integral, &seleccion(q))
                .as_ref()
                .ok()
                != Some(q)
        }) {
            return Err(format!(
                "La pestaña {} no tiene el mismo alcance disponible.",
                i + 1
            ));
        }
        canales.push((p.as_ref() as &(dyn lote::Conexion + Sync), catalogo));
    }
    Ok(canales)
}

struct Fase {
    carpeta: Option<PathBuf>,
    resultado: Result<Vec<Value>, String>,
}
enum Mensaje {
    Estado(String),
    Aviso(String),
}

fn correr_fases(
    canales: &[Canal<'_>],
    planes: &[PlanLote],
    solicitud: &lote::Solicitud<'_>,
    raiz: &Path,
    emitir: &mut dyn FnMut(lote::Evento),
) -> Vec<Fase> {
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|grupo| {
        let trabajos: Vec<_> = canales
            .iter()
            .zip(planes)
            .enumerate()
            .map(|(i, (canal, parte))| {
                let tx = tx.clone();
                grupo.spawn(move || {
                    let mut carpeta = None;
                    let mut relé = |evento| {
                        let estado = match evento {
                            lote::Evento::Carpeta(c) => {
                                carpeta = Some(c);
                                return;
                            }
                            lote::Evento::Consulta {
                                indice,
                                total,
                                etiqueta,
                            } => format!("Consulta {indice} de {total} · {etiqueta}"),
                            lote::Evento::Pagina { pagina, total, .. } => {
                                format!("Página {pagina} de {total} verificada")
                            }
                            lote::Evento::Aviso(t) => {
                                let _ = tx.send(Mensaje::Aviso(format!("Pestaña {} · {t}", i + 1)));
                                return;
                            }
                            _ => return,
                        };
                        let _ = tx.send(Mensaje::Estado(format!("Pestaña {} · {estado}", i + 1)));
                    };
                    let resultado = lote::ejecutar(
                        canal.0,
                        &canal.1,
                        Edicion::Integral,
                        parte,
                        &raiz.join(format!("Pestana-{}", i + 1)),
                        solicitud.opciones,
                        solicitud.cancelar,
                        &mut relé,
                    );
                    Fase { carpeta, resultado }
                })
            })
            .collect();
        drop(tx);
        for mensaje in rx {
            emitir(match mensaje {
                Mensaje::Estado(t) => lote::Evento::Estado(t),
                Mensaje::Aviso(t) => lote::Evento::Aviso(t),
            });
        }
        trabajos
            .into_iter()
            .map(|t| {
                t.join().unwrap_or_else(|_| Fase {
                    carpeta: None,
                    resultado: Err("Se interrumpió el trabajo en una pestaña.".into()),
                })
            })
            .collect()
    })
}

fn clave(seleccion: &Value) -> String {
    paquete::json_python(&lote::sin_fecha_descarga(seleccion), true)
}

fn incorporar(
    raiz: &Path,
    origen: &paquete::Lote,
    indice: usize,
    consulta: &Value,
    control: &Value,
) -> Result<Value, String> {
    let mut control = control.clone();
    if let Some(archivos) = control["archivos"].as_array_mut() {
        for item in archivos.iter_mut().filter(|v| v["estado"] == "OK") {
            flujo_csmp::copiar_original(&origen.carpeta, raiz, indice, item)?;
        }
    }
    if control.get("evidencia").is_some_and(|v| !v.is_null()) {
        flujo_csmp::copiar_original(&origen.carpeta, raiz, indice, &mut control["evidencia"])?;
    }
    let nombre = format!("verificacion {indice:03}.json");
    lote::escribir_json(raiz, &nombre, &control)?;
    let mut consulta = consulta.clone();
    consulta["verificacion"] = json!(nombre);
    if consulta.get("evidencia").is_some() {
        consulta["evidencia"] = control["evidencia"].clone();
    }
    Ok(consulta)
}

pub(super) fn ejecutar_preparado(
    canales: &[Canal<'_>],
    planes: &[PlanLote],
    solicitud: &lote::Solicitud<'_>,
    emitir: &mut dyn FnMut(lote::Evento),
) -> Result<Vec<Value>, String> {
    if solicitud.cancelar.load(Ordering::SeqCst) {
        return Err("Descarga cancelada.".into());
    }
    let planificadas =
        lote::selecciones_plan(solicitud.plan, solicitud.catalogo, Edicion::Integral);
    let posiciones: BTreeMap<_, _> = planificadas
        .iter()
        .enumerate()
        .map(|(i, s)| (clave(s), i))
        .collect();
    if posiciones.len() != planificadas.len() {
        return Err("El plan tiene consultas repetidas.".into());
    }
    let raiz = lote::carpeta_nueva(solicitud.destino)?;
    let mut resumen = json!({
        "version_contrato": 1, "estado": "EN_CURSO", "fecha_hora": lote::ahora(),
        "plan": planificadas, "consultas": [], "consultas_esperadas": posiciones.len(),
        "consultas_completadas": 0, "otros_filtros": "todos los registros",
        "filtro_fecha": solicitud.plan.fechas.map(|(a, b)| json!({"desde": lote::fecha_sitfa(a), "hasta": lote::fecha_sitfa(b)})),
        "pestanas_conectadas": canales.len(),
    });
    lote::escribir_json(&raiz, "resumen.json", &resumen)?;
    emitir(lote::Evento::Carpeta(raiz.clone()));
    emitir(lote::Evento::Plan(posiciones.len()));
    emitir(lote::Evento::Estado(format!(
        "Repartiendo consultas en {} pestañas…",
        canales.len()
    )));
    let fases = correr_fases(canales, planes, solicitud, &raiz, emitir);
    let mut completas = BTreeMap::new();
    let mut parciales = BTreeSet::new();
    let mut diagnosticos = Vec::new();
    let mut errores = Vec::new();
    for (i, (fase, parte)) in fases.iter().zip(planes).enumerate() {
        if let Err(e) = &fase.resultado {
            errores.push(format!("Pestaña {} · {e}", i + 1));
        }
        let Some(carpeta) = &fase.carpeta else {
            continue;
        };
        let datos = match paquete::cargar_lote_con(carpeta, true) {
            Ok(lote) => lote,
            Err(e) => {
                errores.push(format!("Pestaña {} · {e}", i + 1));
                continue;
            }
        };
        let esperadas: BTreeSet<_> =
            lote::selecciones_plan(parte, &canales[i].1, Edicion::Integral)
                .iter()
                .map(clave)
                .collect();
        for (q, control) in &datos.consultas {
            let k = clave(&q["seleccion"]);
            let Some(&pos) = posiciones.get(&k).filter(|_| esperadas.contains(&k)) else {
                errores.push(format!(
                    "Pestaña {} devolvió una consulta fuera de alcance.",
                    i + 1
                ));
                continue;
            };
            if completas.contains_key(&pos) || parciales.contains(&pos) {
                errores.push("Una consulta aparece en más de una fase.".into());
                continue;
            }
            match incorporar(&raiz, &datos, pos + 1, q, control) {
                Ok(consulta) if q["estado"] != "INCOMPLETA" => {
                    resumen["plan"][pos] = consulta["seleccion"].clone();
                    completas.insert(pos, consulta);
                }
                Ok(consulta) => {
                    resumen["plan"][pos] = consulta["seleccion"].clone();
                    parciales.insert(pos);
                }
                Err(e) => errores.push(e),
            }
        }
        let diag = carpeta.join("diagnostico_columnas_html_xls.json");
        if let Ok(v) = paquete::leer_json(&diag) {
            if let Some(items) = v["consultas"].as_array() {
                diagnosticos.extend(items.iter().cloned());
            }
        }
    }
    let completas: Vec<_> = completas.into_values().collect();
    if !diagnosticos.is_empty() {
        if let Err(e) = lote::escribir_json(
            &raiz,
            "diagnostico_columnas_html_xls.json",
            &json!({"version": 1, "pantalla": "seguimiento", "consultas": diagnosticos}),
        ) {
            emitir(lote::Evento::Aviso(format!(
                "No se guardó el diagnóstico de encabezados: {e}"
            )));
        }
    }
    let validada = errores.is_empty() && completas.len() == posiciones.len();
    resumen["consultas_completadas"] = json!(completas.len());
    resumen["consultas"] = json!(completas);
    resumen["estado"] = json!(if validada { "VALIDADA" } else { "INCOMPLETA" });
    lote::escribir_json(&raiz, "resumen.json", &resumen)?;
    if !validada {
        return Err(format!(
            "Descarga incompleta ({}/{} consultas). Los archivos verificados se conservan. {}",
            completas.len(),
            posiciones.len(),
            errores.join(" · ")
        ));
    }
    for (i, q) in completas.iter().enumerate() {
        let etiqueta = q["consulta"].as_str().unwrap_or("").to_owned();
        let estado = q["estado"].as_str().unwrap_or("").to_owned();
        emitir(lote::Evento::Consulta {
            indice: i + 1,
            total: completas.len(),
            etiqueta: etiqueta.clone(),
        });
        if estado == "SIN_RESULTADOS" {
            emitir(lote::Evento::Vacia {
                consulta: etiqueta.clone(),
            });
        } else {
            emitir(lote::Evento::Total(
                q["paginas"].as_u64().unwrap_or(0) as usize
            ));
        }
        emitir(lote::Evento::ConsultaLista {
            consulta: etiqueta,
            estado,
        });
    }
    let sumar = |campo| {
        completas
            .iter()
            .filter_map(|q| q[campo].as_u64())
            .sum::<u64>() as usize
    };
    emitir(lote::Evento::Fin(lote::Fin {
        carpeta: raiz,
        consultas: completas.len(),
        archivos: sumar("paginas"),
        pdfs: 0,
        vacias: completas
            .iter()
            .filter(|q| q["estado"] == "SIN_RESULTADOS")
            .count(),
        registros: sumar("registros"),
    }));
    Ok(completas)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::descargador::{flujo_paquete, sitfa_ficticio, SeleccionLote};
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;

    struct Ficticio {
        sitio: Mutex<sitfa_ficticio::SitfaFicticio>,
        fallar: bool,
    }
    impl lote::Conexion for Ficticio {
        fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
            if self.fallar && comando == "lock" {
                return Err("Falla sintética de conexión.".into());
            }
            self.sitio.lock().unwrap().llamar(comando, payload, plazo)
        }
    }

    fn descargar(
        pantalla: &'static str,
        base: &Path,
        fallar: bool,
    ) -> (PathBuf, PlanLote, Result<Vec<Value>, String>) {
        let catalogo = sitfa_ficticio::catalogo(pantalla);
        let fechas = Some(("2026-10-01".parse().unwrap(), "2026-10-30".parse().unwrap()));
        let seleccion = SeleccionLote {
            tribunales: vec!["901".into(), "902".into()],
            modalidades: vec!["1".into(), "2".into()],
            pestanas: vec!["Espera".into()],
            estados: vec!["1".into(), "2".into()],
            anio: "2026".into(),
            meses: vec!["11".into(), "12".into()],
            fechas: if pantalla.starts_with("calendario") {
                None
            } else {
                fechas
            },
            ..Default::default()
        };
        let plan = catalogo.planificar(Edicion::Integral, &seleccion).unwrap();
        let n = plan.consultas.len().min(3);
        let sesiones: Vec<_> = (0..n)
            .map(|i| Ficticio {
                sitio: Mutex::new(sitfa_ficticio::SitfaFicticio::nuevo(pantalla, &[])),
                fallar: fallar && i == 0,
            })
            .collect();
        let canales: Vec<Canal<'_>> = sesiones
            .iter()
            .map(|s| (s as &(dyn lote::Conexion + Sync), catalogo.clone()))
            .collect();
        let cancelar = AtomicBool::new(false);
        let opciones = lote::Opciones {
            evidencia: false,
            ..Default::default()
        };
        let solicitud = lote::Solicitud {
            catalogo: &catalogo,
            edicion: Edicion::Integral,
            plan: &plan,
            destino: base,
            opciones: &opciones,
            cancelar: &cancelar,
        };
        let mut carpeta = None;
        let resultado = ejecutar_preparado(&canales, &repartir(&plan, n), &solicitud, &mut |e| {
            if let lote::Evento::Carpeta(ruta) = e {
                carpeta = Some(ruta);
            }
        });
        (carpeta.unwrap(), plan, resultado)
    }

    #[test]
    fn multiventana_genera_lotes_normales_de_las_cinco_pantallas_y_conserva_pendientes() {
        let raiz =
            lote::carpeta_nueva(&std::env::temp_dir().join("flow-sintetico-multiventana")).unwrap();
        for (pantalla, n) in [
            ("seguimiento", 4),
            ("litigantes", 4),
            ("calendario_informes", 4),
            ("calendario_medidas", 4),
            ("carga", 2),
        ] {
            let (carpeta, plan, salida) = descargar(pantalla, &raiz.join(pantalla), false);
            assert_eq!(salida.unwrap().len(), n, "{pantalla}");
            let lote = paquete::cargar_lote(&carpeta).unwrap();
            assert_eq!(lote.resumen["pestanas_conectadas"], n.min(3), "{pantalla}");
            assert_eq!(lote.consultas.len(), n);
            assert_eq!(
                flujo_paquete::rango_comprobado_lote(&carpeta, &lote.resumen).unwrap(),
                plan.fechas
            );
            assert!(lote
                .consultas
                .iter()
                .all(|(_, m)| m["archivos"].as_array().is_some_and(|v| !v.is_empty())));
        }
        let (carpeta, _, salida) = descargar("seguimiento", &raiz.join("parcial"), true);
        assert!(salida.unwrap_err().contains("2/4 consultas"));
        assert!(paquete::cargar_lote(&carpeta).is_err());
        assert_eq!(
            paquete::cargar_lote_con(&carpeta, true)
                .unwrap()
                .consultas
                .len(),
            2
        );
        assert_eq!(lote::pendientes(&carpeta).unwrap().selecciones.len(), 2);
        let _ = std::fs::remove_dir_all(raiz);
    }
}
