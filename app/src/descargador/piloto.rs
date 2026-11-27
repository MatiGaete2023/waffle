//! Piloto de listados de Seguimiento. Cada conexión ejecuta consultas completas.
use super::{
    archivo_local, comprobar_extension, lote, paquete, resultados, Alcance, Catalogo, Edicion,
    Pantalla, PlanLote, Puente, Seleccion, TODAS_LAS_MEDIDAS,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const TIPO: &str = "PILOTO_LISTADOS";
pub const PESTANAS: usize = 3;

#[derive(Debug)]
pub enum Evento {
    Carpeta(PathBuf),
    Avance { pestana: usize, texto: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Medidas {
    pub pestanas: usize,
    pub tiempo_ms: u64,
    pub consultas: usize,
    pub registros: usize,
    pub xls: usize,
}

#[derive(Debug)]
pub struct Comparacion {
    pub una: Medidas,
    pub tres: Medidas,
    pub filas_nuevas: usize,
    pub filas_ausentes: usize,
    pub columnas_coinciden: bool,
}

type Canal<'a> = (&'a (dyn lote::Conexion + Sync), Catalogo);

struct Fase {
    carpeta: Option<PathBuf>,
    consultas: usize,
    fin: Result<lote::Fin, String>,
}

fn repartir(plan: &PlanLote, n: usize) -> Result<Vec<PlanLote>, String> {
    if !matches!(n, 1 | PESTANAS) || plan.consultas.len() < n {
        return Err(format!(
            "Se necesitan al menos {n} consultas completas para esta medición."
        ));
    }
    if plan
        .consultas
        .iter()
        .any(|p| p.pantalla != Pantalla::Seguimiento)
    {
        return Err("El piloto admite solo listados de Seguimiento.".into());
    }
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
    Ok(partes)
}

pub(super) fn registrar_documento(
    capacidades: &Value,
    documentos: &mut BTreeSet<String>,
) -> Result<(), String> {
    comprobar_extension(Edicion::Integral, capacidades)?;
    let doc = capacidades["documento_piloto"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .ok_or("Actualiza y recarga la extensión Integral 2.11.1 en Chrome para usar varias pestañas.")?;
    if documentos.insert(doc.to_owned()) {
        Ok(())
    } else {
        Err("Dos códigos apuntan a la misma pestaña SITFA. Conecta cada código desde una pestaña distinta.".into())
    }
}

pub fn ejecutar(
    puentes: &[Arc<Puente>],
    plan: &PlanLote,
    destino: &Path,
    pausa: Duration,
    cancelar: &AtomicBool,
    emitir: &(dyn Fn(Evento) + Sync),
) -> Result<Medidas, String> {
    let planes = repartir(plan, puentes.len())?;
    let mut documentos = BTreeSet::new();
    let mut canales = Vec::new();
    for (i, (p, parte)) in puentes.iter().zip(&planes).enumerate() {
        let mut preparar = || -> Result<Catalogo, String> {
            if cancelar.load(Ordering::SeqCst) {
                return Err("Piloto cancelado.".into());
            }
            // Leer el catálogo visible. El piloto no navega ni cambia de pantalla en Chrome.
            let datos = p.llamar("catalog", json!({}), Duration::from_secs(15))?;
            let catalogo: Catalogo =
                serde_json::from_value(datos).map_err(|_| "Catálogo inválido.")?;
            if catalogo.pantalla != Pantalla::Seguimiento
                || parte.consultas.iter().any(|consulta| {
                    let Alcance::Seguimiento {
                        pestana,
                        modalidad,
                        reporte,
                    } = &consulta.alcance
                    else {
                        return true;
                    };
                    let seleccion = Seleccion {
                        tribunal: consulta.tribunal.clone(),
                        pestana: pestana.clone(),
                        modalidad: modalidad.clone(),
                        reporte: reporte.clone().unwrap_or_else(|| TODAS_LAS_MEDIDAS.into()),
                        ..Default::default()
                    };
                    !catalogo
                        .preparar(Edicion::Integral, &seleccion)
                        .is_ok_and(|actual| actual == *consulta)
                })
            {
                return Err("Abre Seguimiento con acceso a las opciones del lote.".into());
            }
            let caps = p.llamar(
                "capabilities",
                json!({"documento_piloto": true}),
                Duration::from_secs(15),
            )?;
            registrar_documento(&caps, &mut documentos)?;
            p.registrar_search_and_download(caps["protocolos"]["search_and_download"] == 1);
            Ok(catalogo)
        };
        emitir(Evento::Avance {
            pestana: i,
            texto: "Comprobando Seguimiento…".into(),
        });
        let catalogo = preparar().map_err(|e| format!("Pestaña {} · {e}", i + 1))?;
        canales.push((p.as_ref() as &(dyn lote::Conexion + Sync), catalogo));
    }
    ejecutar_preparado(&canales, &planes, destino, pausa, cancelar, emitir)
}

fn descargar_fase(
    canal: &Canal<'_>,
    plan: &PlanLote,
    destino: &Path,
    opciones: &lote::Opciones,
    cancelar: &AtomicBool,
    avance: impl Fn(String),
) -> Fase {
    let mut carpeta = None;
    let mut fin = None;
    let mut consultas = 0;
    let mut actual = 0;
    let resultado = lote::ejecutar(
        canal.0,
        &canal.1,
        Edicion::Integral,
        plan,
        destino,
        opciones,
        cancelar,
        &mut |evento| match evento {
            lote::Evento::Carpeta(ruta) => carpeta = Some(ruta),
            lote::Evento::Consulta { indice, total, .. } => {
                actual = indice;
                avance(format!("Consulta {indice} de {total}"));
            }
            lote::Evento::Pagina { pagina, total, .. } => {
                avance(format!("Consulta {actual} · página {pagina} de {total}"))
            }
            lote::Evento::ConsultaLista { .. } => consultas += 1,
            lote::Evento::Aviso(texto) => avance(texto),
            lote::Evento::Fin(resumen) => fin = Some(resumen),
            _ => {}
        },
    );
    let fin = resultado.and_then(|hechas| {
        fin.filter(|f| f.consultas == plan.consultas.len() && hechas.len() == f.consultas)
            .ok_or_else(|| "La pestaña no completó todas sus consultas.".into())
    });
    if fin.is_err() {
        cancelar.store(true, Ordering::SeqCst);
    }
    avance(match &fin {
        Ok(fin) => format!(
            "{} de {} consultas verificadas",
            fin.consultas,
            plan.consultas.len()
        ),
        Err(e) => format!("{consultas} de {} consultas · {e}", plan.consultas.len()),
    });
    Fase {
        carpeta,
        consultas,
        fin,
    }
}

fn ejecutar_preparado(
    canales: &[Canal<'_>],
    planes: &[PlanLote],
    destino: &Path,
    pausa: Duration,
    cancelar: &AtomicBool,
    emitir: &(dyn Fn(Evento) + Sync),
) -> Result<Medidas, String> {
    if cancelar.load(Ordering::SeqCst) {
        return Err("Piloto cancelado.".into());
    }
    let carpeta = lote::carpeta_nueva(destino)?;
    let esperadas: usize = planes.iter().map(|parte| parte.consultas.len()).sum();
    let mut flujo = json!({
        "tipo": TIPO, "estado": "EN_CURSO", "consultas_esperadas": esperadas, "fases": {},
    });
    lote::escribir_json(&carpeta, "flujo.json", &flujo)?;
    emitir(Evento::Carpeta(carpeta.clone()));
    let opciones = lote::Opciones {
        evidencia: false,
        conservar_filtros: false,
        pausa,
    };
    let inicio = Instant::now();
    let fases = std::thread::scope(|grupo| {
        let trabajos: Vec<_> = canales
            .iter()
            .zip(planes)
            .enumerate()
            .map(|(i, (canal, plan))| {
                let destino = carpeta.join(format!("Pestaña {}", i + 1));
                let opciones = &opciones;
                grupo.spawn(move || {
                    descargar_fase(canal, plan, &destino, opciones, cancelar, |texto| {
                        emitir(Evento::Avance { pestana: i, texto });
                    })
                })
            })
            .collect();
        trabajos
            .into_iter()
            .map(|t| {
                t.join().unwrap_or_else(|_| {
                    cancelar.store(true, Ordering::SeqCst);
                    Fase {
                        carpeta: None,
                        consultas: 0,
                        fin: Err("Se interrumpió la ejecución.".into()),
                    }
                })
            })
            .collect::<Vec<_>>()
    });
    let mut medidas = Medidas {
        pestanas: fases.len(),
        tiempo_ms: inicio.elapsed().as_millis() as u64,
        consultas: 0,
        registros: 0,
        xls: 0,
    };
    let mut fallas = Vec::new();
    for (i, (fase, parte)) in fases.into_iter().zip(planes).enumerate() {
        let relativa = fase
            .carpeta
            .as_deref()
            .and_then(|r| r.strip_prefix(&carpeta).ok());
        let estado = if fase.fin.is_ok() {
            "VALIDADA"
        } else {
            "INCOMPLETA"
        };
        flujo["fases"][format!("Pestaña {}", i + 1)] = json!({
            "carpeta": relativa, "estado": estado, "consultas": fase.consultas,
            "consultas_esperadas": parte.consultas.len(),
        });
        medidas.consultas += fase.consultas;
        match fase.fin {
            Ok(fin) => {
                medidas.registros += fin.registros;
                medidas.xls += fin.archivos;
            }
            Err(e) => {
                fallas.push(format!("Pestaña {} · {e}", i + 1));
            }
        }
    }
    let completo = fallas.is_empty() && medidas.consultas == esperadas;
    flujo["estado"] = json!(if completo { "VALIDADA" } else { "INCOMPLETA" });
    flujo["medidas"] = json!(medidas);
    lote::escribir_json(&carpeta, "flujo.json", &flujo)?;
    if completo {
        Ok(medidas)
    } else {
        Err(format!("Piloto incompleto ({}/{esperadas} consultas). Los archivos verificados se conservan. {}", medidas.consultas, fallas.join(" · ")))
    }
}

fn leer_medidas(carpeta: &Path) -> Result<Medidas, String> {
    let archivo = std::fs::File::open(carpeta.join("flujo.json"))
        .map_err(|_| "Selecciona la carpeta de una medición del piloto.")?;
    let mut datos = Vec::new();
    const LIMITE: usize = 64 * 1024;
    archivo
        .take((LIMITE + 1) as u64)
        .read_to_end(&mut datos)
        .map_err(|e| e.to_string())?;
    if datos.len() > LIMITE {
        return Err("El resumen del piloto excede el límite de lectura.".into());
    }
    let resumen: Value =
        serde_json::from_slice(&datos).map_err(|_| "Resumen del piloto inválido.")?;
    if resumen["tipo"] != TIPO || resumen["estado"] != "VALIDADA" {
        return Err("La medición elegida debe ser un piloto completo.".into());
    }
    serde_json::from_value(resumen["medidas"].clone())
        .map_err(|_| "La medición elegida no contiene sus indicadores.".into())
}

type Huellas = BTreeMap<[u8; 32], usize>;

#[derive(Default)]
struct Inventario {
    alcance: Vec<String>,
    columnas: BTreeSet<[u8; 32]>,
    filas: Huellas,
}

fn huella<T: Serialize>(valor: &T) -> Result<[u8; 32], String> {
    serde_json::to_vec(valor)
        .map(|bytes| Sha256::digest(bytes).into())
        .map_err(|e| e.to_string())
}

fn inventario(carpeta: &Path) -> Result<Inventario, String> {
    let lotes = archivo_local::cargar_fuentes(&[carpeta.to_owned()], false)?;
    let mut salida = Inventario::default();
    for lote in &lotes {
        let plan = lote.resumen["plan"]
            .as_array()
            .ok_or("Falta el alcance de una fase del piloto.")?;
        let rango = &lote.resumen["filtro_fecha"];
        for seleccion in plan {
            salida.alcance.push(paquete::json_python(
                &json!([resultados::limpio(seleccion), rango]),
                true,
            ));
        }
        for tabla in resultados::tablas(lote)? {
            let seleccion = paquete::json_python(&resultados::limpio(&tabla.seleccion), true);
            let columnas = huella(&(&seleccion, &tabla.cabecera))?;
            salida.columnas.insert(columnas);
            for (_, fila) in tabla.filas {
                let fila: Vec<_> = fila.iter().map(|c| c.texto_python()).collect();
                *salida.filas.entry(huella(&(columnas, fila))?).or_default() += 1;
            }
        }
    }
    salida.alcance.sort_unstable();
    Ok(salida)
}

pub fn comparar(una: &Path, tres: &Path) -> Result<Comparacion, String> {
    let (una_med, tres_med) = (leer_medidas(una)?, leer_medidas(tres)?);
    if una_med.pestanas != 1 || tres_med.pestanas != PESTANAS {
        return Err("Selecciona la medición completa de una pestaña como referencia.".into());
    }
    let (a, b) = (inventario(una)?, inventario(tres)?);
    if a.alcance != b.alcance
        || a.alcance.len() != una_med.consultas
        || b.alcance.len() != tres_med.consultas
    {
        return Err(
            "Las mediciones tienen alcances distintos. Usa las mismas consultas y fechas.".into(),
        );
    }
    let faltan = |a: &Huellas, b: &Huellas| -> usize {
        a.iter()
            .map(|(k, n)| n.saturating_sub(b.get(k).copied().unwrap_or(0)))
            .sum()
    };
    Ok(Comparacion {
        una: una_med,
        tres: tres_med,
        filas_nuevas: faltan(&b.filas, &a.filas),
        filas_ausentes: faltan(&a.filas, &b.filas),
        columnas_coinciden: a.columnas == b.columnas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descargador::{sitfa_ficticio, SeleccionLote};
    use std::sync::{Barrier, Mutex};

    struct Carpeta(PathBuf);

    impl Carpeta {
        fn nueva() -> Self {
            let mut id = [0u8; 8];
            getrandom::fill(&mut id).unwrap();
            let ruta = std::env::temp_dir().join(format!("flow-piloto-{}", u64::from_ne_bytes(id)));
            std::fs::create_dir(&ruta).unwrap();
            Self(ruta)
        }
    }

    impl Drop for Carpeta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct Ficticio {
        sesion: Mutex<sitfa_ficticio::SitfaFicticio>,
        barrera: Option<Arc<Barrier>>,
    }

    impl Ficticio {
        fn nuevo(barrera: Option<Arc<Barrier>>, cambia: bool) -> Self {
            let paginas = if cambia { 1 } else { 2 };
            let casos = [("901|Espera|1|None", paginas), ("902|Espera|2|None", 0)];
            Self {
                sesion: Mutex::new(sitfa_ficticio::SitfaFicticio::nuevo("seguimiento", &casos)),
                barrera,
            }
        }
    }

    impl lote::Conexion for Ficticio {
        fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
            if comando == "lock" {
                if let Some(b) = &self.barrera {
                    b.wait();
                }
            }
            self.sesion.lock().unwrap().llamar(comando, payload, plazo)
        }
    }

    fn medir(destino: &Path, plan: &PlanLote, n: usize, cambia: bool) -> (PathBuf, Medidas) {
        let barrera = (n == 3).then(|| Arc::new(Barrier::new(n)));
        let tab: Vec<_> = (0..n)
            .map(|i| Ficticio::nuevo(barrera.clone(), cambia && i == 0))
            .collect();
        let canales: Vec<Canal<'_>> = tab
            .iter()
            .map(|t| {
                (
                    t as &(dyn lote::Conexion + Sync),
                    sitfa_ficticio::catalogo("seguimiento"),
                )
            })
            .collect();
        let carpeta = Mutex::new(None);
        let medidas = ejecutar_preparado(
            &canales,
            &repartir(plan, n).unwrap(),
            destino,
            Duration::ZERO,
            &AtomicBool::new(false),
            &|evento| {
                if let Evento::Carpeta(ruta) = evento {
                    *carpeta.lock().unwrap() = Some(ruta);
                }
            },
        )
        .unwrap();
        (carpeta.into_inner().unwrap().unwrap(), medidas)
    }

    #[test]
    fn piloto_tres_pestanas_conserva_filas_y_detecta_una_pagina_ausente() {
        let destino = Carpeta::nueva();
        let catalogo = sitfa_ficticio::catalogo("seguimiento");
        let plan = catalogo
            .planificar(
                Edicion::Integral,
                &SeleccionLote {
                    tribunales: vec!["901".into(), "902".into()],
                    modalidades: vec!["1".into(), "2".into()],
                    pestanas: vec!["Espera".into()],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(plan.consultas.len(), 4);
        let (una, a) = medir(&destino.0, &plan, 1, false);
        let (tres, b) = medir(&destino.0, &plan, 3, false);
        assert_eq!((a.consultas, a.xls, a.registros), (4, 4, 12));
        assert_eq!((b.consultas, b.xls, b.registros), (4, 4, 12));
        assert_eq!(
            archivo_local::fases_conjunto(&tres, false).unwrap().len(),
            3
        );
        let c = comparar(&una, &tres).unwrap();
        assert_eq!(
            (c.filas_nuevas, c.filas_ausentes, c.columnas_coinciden),
            (0, 0, true)
        );

        let (distinta, d) = medir(&destino.0, &plan, 3, true);
        assert_eq!((d.consultas, d.xls, d.registros), (4, 3, 9));
        let cambio = comparar(&una, &distinta).unwrap();
        assert_eq!((cambio.filas_nuevas, cambio.filas_ausentes), (0, 3));
    }

    #[test]
    fn piloto_rechaza_documentos_repetidos_y_extension_sin_identificador() {
        let mut documentos = BTreeSet::new();
        let mut caps = json!({
            "tipo": "CSMP_RUS_CAPACIDADES", "version": "2.9.0", "escritura_rus": false,
            "protocolos": { "bitacoras_lectura": 1, "descarga_conjunta": 1, "pdf_seleccion": 1, "pdf_copia_fija": 1,
                "calendario_lectura": 1, "historia_lectura": 1, "historico_lectura": 2, "search_and_download": 1 },
            "flow_rust_version": "2.11.1", "documento_piloto": "ficticio-1"
        });
        registrar_documento(&caps, &mut documentos).unwrap();
        assert!(registrar_documento(&caps, &mut documentos)
            .unwrap_err()
            .contains("misma pestaña"));
        caps["documento_piloto"] = Value::Null;
        assert!(registrar_documento(&caps, &mut documentos)
            .unwrap_err()
            .contains("recarga"));
        assert_eq!(documentos.len(), 1);
    }
}
